use std::{
    fmt::Display,
    io::{self, Read},
    process::{self, Stdio},
};

use crate::ExecutionError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    command: String,
    arguments: Vec<String>,
    next: Option<Box<Command>>,
}

impl Command {
    pub fn new(command: String, arguments: Vec<String>) -> Self {
        Self {
            command,
            arguments,
            next: None,
        }
    }

    pub fn pipe(&self, next: Command) -> Result<Self, ExecutionError> {
        let mut result = self.clone();
        result.next = Some(Box::new(next));
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
        f.write_str(&self.command)?;
        for arg in &self.arguments {
            f.write_str(" ")?;
            f.write_str("\"")?;
            f.write_str(&arg)?;
            f.write_str("\"")?;
        }

        if let Some(next) = &self.next {
            f.write_str(" => ")?;
            next.fmt(f)?;
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

impl CommandExecutor for SystemCommandExecutor {
    fn run(&self, command: &Command) -> io::Result<CommandResult> {
        let mut processes = Vec::new();
        let mut current = (command, Stdio::null());

        let mut stdout = loop {
            let mut process = process::Command::new(current.0.command.to_owned());
            process.args(current.0.arguments.to_owned());
            process.stdin(current.1);
            process.stdout(Stdio::piped());

            let mut process = process.spawn()?;
            let stdout = process.stdout.take().unwrap();
            processes.push(process);

            if let Some(next) = &current.0.next {
                current = (next, Stdio::from(stdout));
            } else {
                break stdout;
            }
        };

        let mut status_code = StatusCode::Value(0);
        for process in &mut processes {
            let result = process.wait()?;

            // Only update status code if all previous commands were successful
            if matches!(status_code, StatusCode::Value(0)) {
                status_code = result.code().into();
            }
        }

        let mut stdout_data = String::new();
        stdout.read_to_string(&mut stdout_data)?;

        return Ok(CommandResult {
            stdout: stdout_data,
            status_code,
        });
    }
}
