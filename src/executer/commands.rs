use std::{
    fs::File,
    io::{self, copy, Read},
    process::{self, Stdio},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    command: String,
    arguments: Vec<String>,
    output: Option<CommandOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandOutput {
    destination: OutputDestination,
    source: OutputSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OutputDestination {
    File(String),
    Command(Box<Command>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputSource {
    Stdout,
    Stderr,
    All,
}

impl Command {
    pub fn new(command: String, arguments: Vec<String>) -> Self {
        Self {
            command,
            arguments,
            output: None,
        }
    }

    fn try_pipe(&self, destination: OutputDestination, source: OutputSource) -> Option<Self> {
        let mut result = self.clone();
        let mut inner = &mut result;
        while let Some(ref mut output) = inner.output {
            if let OutputDestination::Command(ref mut command) = output.destination {
                inner = command.as_mut()
            } else {
                return None;
            }
        }

        inner.output = Some(CommandOutput {
            destination,
            source,
        });

        Some(result)
    }

    pub fn try_pipe_command(&self, command: Command, source: OutputSource) -> Option<Self> {
        self.try_pipe(OutputDestination::Command(Box::new(command)), source)
    }

    pub fn try_pipe_file(&self, file: String, source: OutputSource) -> Option<Self> {
        self.try_pipe(OutputDestination::File(file), source)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StatusCode {
    Terminated,
    Value(u8),
}

impl From<i32> for StatusCode {
    fn from(value: i32) -> Self {
        Self::Value(value.to_le_bytes()[0])
    }
}

impl From<Option<i32>> for StatusCode {
    fn from(value: Option<i32>) -> Self {
        match value {
            None => Self::Terminated,
            Some(x) => x.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub status_code: StatusCode,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandExecutor {
    fn run(&self, command: &Command) -> io::Result<CommandResult>;
}

pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn run(&self, command: &Command) -> io::Result<CommandResult> {
        let mut stdout = None;
        let mut stderr = None;
        let mut input_source = None;
        let mut command = command;
        let mut processes = Vec::new();
        loop {
            let mut process = process::Command::new(command.command.to_owned());
            process.args(command.arguments.to_owned());
            process.stdin(Stdio::piped());
            process.stdout(Stdio::piped());
            process.stderr(Stdio::piped());

            if let Some(input_source) = input_source {
                match input_source {
                    OutputSource::Stdout => {
                        process.stdin(Stdio::from(stdout.unwrap()));
                    }
                    OutputSource::Stderr => todo!(),
                    OutputSource::All => todo!(),
                }
            }

            let mut process = process.spawn()?;
            stdout = process.stdout.take();
            stderr = process.stderr.take();
            processes.push(process);

            let Some(ref output) = command.output else {
                break;
            };

            input_source = Some(output.source.clone());

            match &output.destination {
                OutputDestination::File(file) => {
                    copy(&mut stdout.take().unwrap(), &mut File::create(file)?)?;
                    break;
                }
                OutputDestination::Command(nested_command) => command = nested_command.as_ref(),
            }
        }

        let mut status_code = StatusCode::Value(0);
        let mut stdout_data = String::new();
        if let Some(mut stdout) = stdout {
            stdout.read_to_string(&mut stdout_data)?;
            stdout_data.truncate(stdout_data.trim_end().len());
        }

        let mut stderr_data = String::new();
        if let Some(mut stderr) = stderr {
            stderr.read_to_string(&mut stderr_data)?;
            stderr_data.truncate(stderr_data.trim_end().len());
        }

        for process in &mut processes {
            let result = process.wait()?;

            // Only update status code if all previous commands were successful
            if matches!(status_code, StatusCode::Value(0)) {
                status_code = result.code().into();
            }
        }

        return Ok(CommandResult {
            status_code,
            stdout: stdout_data,
            stderr: stderr_data,
        });
    }
}
