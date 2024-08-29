use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    process::{self, ChildStdout, Stdio},
};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDefinition {
    pub program: String,
    pub arguments: Vec<String>,
    pub capture_stderr: bool,
}

impl CommandDefinition {
    pub fn new(program: String, arguments: Vec<String>, capture_stderr: bool) -> Self {
        Self {
            program,
            arguments,
            capture_stderr,
        }
    }
}

impl From<&str> for CommandDefinition {
    fn from(value: &str) -> Self {
        Self::new(value.to_owned(), Vec::new(), false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub commands: Vec<CommandDefinition>,
    pub source: Option<PipelineSource>,
    pub destination: Option<PipelineDestination>,
}

impl Pipeline {
    pub fn new(
        commands: Vec<CommandDefinition>,
        source: Option<PipelineSource>,
        destination: Option<PipelineDestination>,
    ) -> Self {
        Self {
            commands,
            source,
            destination,
        }
    }
}

impl<'a, I: IntoIterator<Item = &'a str>> From<I> for Pipeline {
    fn from(value: I) -> Self {
        Pipeline::new(
            value
                .into_iter()
                .map(|command| (*command).into())
                .collect::<Vec<_>>(),
            None,
            None,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PipelineSource {
    File(String),
    Literal(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PipelineDestination {
    FileWrite(String),
    FileAppend(String),
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub stdout: String,
    pub command_outputs: Vec<(u8, String)>,
}

impl PipelineResult {
    pub fn new(stdout: String, command_outputs: Vec<(u8, String)>) -> Self {
        Self {
            stdout,
            command_outputs,
        }
    }
}

pub trait CommandExecutor {
    fn run(&self, command: &Pipeline) -> io::Result<PipelineResult>;
}

pub struct SystemCommandExecutor;

impl SystemCommandExecutor {
    pub fn new() -> Self {
        SystemCommandExecutor {}
    }
}

enum DataSource {
    Null(),
    Literal(String),
    File(File),
    ChildStdout(ChildStdout),
}

enum StdioOrString {
    Stdio(Stdio),
    String(String),
}

impl DataSource {
    fn to_stdio(self) -> StdioOrString {
        match self {
            DataSource::Null() => StdioOrString::Stdio(Stdio::null()),
            DataSource::Literal(value) => StdioOrString::String(value),
            DataSource::File(file) => StdioOrString::Stdio(Stdio::from(file)),
            DataSource::ChildStdout(stdout) => StdioOrString::Stdio(Stdio::from(stdout)),
        }
    }

    fn read_to_file(self, mut file: File) -> io::Result<()> {
        match self {
            DataSource::Null() => {}
            DataSource::Literal(value) => {
                io::copy(&mut value.as_bytes(), &mut file)?;
            }
            DataSource::File(mut source) => {
                io::copy(&mut source, &mut file)?;
            }
            DataSource::ChildStdout(mut stdout) => {
                io::copy(&mut stdout, &mut file)?;
            }
        }

        Ok(())
    }

    fn read_to_string(self) -> io::Result<String> {
        Ok(match self {
            DataSource::Null() => String::new(),
            DataSource::Literal(value) => value,
            DataSource::File(mut file) => {
                let mut buf = String::new();
                file.read_to_string(&mut buf)?;
                buf
            }
            DataSource::ChildStdout(mut stdout) => {
                let mut buf = String::new();
                stdout.read_to_string(&mut buf)?;
                buf
            }
        })
    }
}

pub struct PipelineExecutionOptions {
    pub command_options: Vec<CommandExecutionOptions>,
}

#[derive(Clone)]
pub struct CommandExecutionOptions {
    pub capture_stderr: bool,
}

impl CommandExecutor for SystemCommandExecutor {
    fn run(&self, pipeline: &Pipeline) -> io::Result<PipelineResult> {
        let mut processes = Vec::new();
        let mut input = if let Some(source) = &pipeline.source {
            match source {
                PipelineSource::File(file) => DataSource::File(File::open(file)?),
                PipelineSource::Literal(literal) => DataSource::Literal(literal.to_owned()),
            }
        } else {
            DataSource::Null()
        };

        for command in &pipeline.commands {
            let mut process = process::Command::new(command.program.to_owned());
            process.args(command.arguments.to_owned());
            let input_string = match input.to_stdio() {
                StdioOrString::Stdio(input) => {
                    process.stdin(input);
                    None
                }
                StdioOrString::String(value) => {
                    process.stdin(Stdio::piped());
                    Some(value)
                }
            };
            process.stdout(Stdio::piped());

            if command.capture_stderr {
                process.stderr(Stdio::piped());
            }

            let mut process = process.spawn()?;
            let stdout = process.stdout.take().unwrap();

            if let Some(input_string) = input_string {
                io::copy(
                    &mut input_string.as_bytes(),
                    &mut process.stdin.take().unwrap(),
                )?;
            }

            let stderr = process.stderr.take();
            processes.push((process, stderr));

            input = DataSource::ChildStdout(stdout);
        }

        let mut outputs = Vec::new();
        for (process, stderr) in &mut processes {
            let status = process.wait()?;
            let mut stderr_data = String::new();
            if let Some(stderr) = stderr {
                stderr.read_to_string(&mut stderr_data)?;
            }

            outputs.push((
                status
                    .code()
                    .ok_or(io::Error::other("Unable to get exit code for command"))?
                    .try_into()
                    .map_err(|_| io::Error::other("Exit code was not between 0 and 255"))?,
                stderr_data,
            ));
        }

        let stdout = if let Some(destination) = &pipeline.destination {
            let file = match destination {
                PipelineDestination::FileWrite(path) => File::create(path)?,
                PipelineDestination::FileAppend(path) => {
                    OpenOptions::new().append(true).open(path)?
                }
            };
            input.read_to_file(file)?;
            String::new()
        } else {
            input.read_to_string()?
        };

        return Ok(PipelineResult::new(stdout, outputs));
    }
}
