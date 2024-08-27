use std::{
    fmt::Display,
    fs::File,
    io::{self, Read, Stdin},
    process::{self, ChildStdout, Stdio},
};

use crate::ExecutionError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    commands: Vec<(String, Vec<String>)>,
    source: Option<CommandSource>,
    destination_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandSource {
    File(String),
    Literal(String),
}

impl Command {
    pub fn new(command: String, arguments: Vec<String>) -> Self {
        Self {
            commands: vec![(command, arguments)],
            source: None,
            destination_file: None,
        }
    }

    pub fn open(path: String) -> Self {
        Self {
            commands: vec![],
            source: Some(CommandSource::File(path)),
            destination_file: None,
        }
    }

    pub fn literal(value: String) -> Self {
        Self {
            commands: vec![],
            source: Some(CommandSource::Literal(value)),
            destination_file: None,
        }
    }

    pub fn write(path: String) -> Self {
        Self {
            commands: vec![],
            source: None,
            destination_file: Some(path),
        }
    }

    pub fn pipe(&self, mut next: Command) -> Result<Self, ExecutionError> {
        if next.source.is_some() {
            return Err("Cannot pipe to command with a source".into());
        }
        if self.destination_file.is_some() {
            return Err("Cannot pipe from command with a destination".into());
        }

        let mut result = self.clone();
        result.commands.append(&mut next.commands);
        result.destination_file = result.destination_file;
        Ok(result)
    }
}

impl From<&str> for Command {
    fn from(value: &str) -> Self {
        Command::new(value.to_owned(), Vec::new())
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

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(source) = &self.source {
            match source {
                CommandSource::File(path) => {
                    f.write_str("open(")?;
                    path.fmt(f)?;
                    f.write_str(")")?;
                }
                CommandSource::Literal(literal) => literal.fmt(f)?,
            }
        }

        let mut first = true;
        for (command, args) in &self.commands {
            if first && self.source.is_none() {
                first = false;
            } else {
                f.write_str(" => ")?;
            }

            f.write_str(command)?;
            for arg in args {
                f.write_str(" ")?;
                f.write_str("\"")?;
                f.write_str(arg)?;
                f.write_str("\"")?;
            }
        }
        if let Some(destination) = &self.destination_file {
            f.write_str(" => write(")?;
            destination.fmt(f)?;
            f.write_str(")")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub stdout: String,
    pub status_code: StatusCode,
}

impl CommandResult {
    pub fn new(status_code: u8, stdout: &str) -> Self {
        Self {
            stdout: stdout.to_owned(),
            status_code: (status_code as i32).into(),
        }
    }
}

pub trait CommandExecutor {
    fn run(&self, command: &Command) -> io::Result<CommandResult>;
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

impl CommandExecutor for SystemCommandExecutor {
    fn run(&self, command: &Command) -> io::Result<CommandResult> {
        let mut processes = Vec::new();
        let mut input = if let Some(source) = &command.source {
            match source {
                CommandSource::File(file) => DataSource::File(File::open(file)?),
                CommandSource::Literal(literal) => DataSource::Literal(literal.to_owned()),
            }
        } else {
            DataSource::Null()
        };

        for (command, arguments) in &command.commands {
            let mut process = process::Command::new(command.to_owned());
            process.args(arguments.to_owned());
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

            let mut process = process.spawn()?;
            let stdout = process.stdout.take().unwrap();

            if let Some(input_string) = input_string {
                io::copy(
                    &mut input_string.as_bytes(),
                    &mut process.stdin.take().unwrap(),
                )?;
            }

            processes.push(process);

            input = DataSource::ChildStdout(stdout);
        }

        let mut status_code = StatusCode::Value(0);
        for process in &mut processes {
            let result = process.wait()?;

            // Only update status code if all previous commands were successful
            if matches!(status_code, StatusCode::Value(0)) {
                status_code = result.code().into();
            }
        }

        let stdout = if let Some(destination) = &command.destination_file {
            input.read_to_file(File::create(destination)?)?;
            String::new()
        } else {
            input.read_to_string()?
        };

        return Ok(CommandResult {
            stdout,
            status_code,
        });
    }
}
