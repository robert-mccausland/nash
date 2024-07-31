use std::{
    fs::File,
    io::{self, copy, Read},
    process::{self, Stdio},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    inner: Vec<(String, Vec<String>)>,
    output_file: Option<String>,
}

impl Command {
    pub fn new(command: String, arguments: Vec<String>) -> Self {
        Self {
            inner: vec![(command, arguments)],
            output_file: None,
        }
    }

    pub fn pipe(&self, next: &Command) -> Self {
        let mut inner = self.inner.clone();
        inner.append(&mut next.inner.clone());
        Self {
            inner,
            output_file: None,
        }
    }

    pub fn pipe_file(&self, file: String) -> Self {
        Self {
            inner: self.inner.clone(),
            output_file: file.into(),
        }
    }

    pub fn has_file(&self) -> bool {
        self.output_file.is_some()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StatusCode {
    Ok,
    Terminated,
    Error(u8),
}

impl From<i32> for StatusCode {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Ok,
            x => Self::Error(x.to_le_bytes()[0]),
        }
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
pub struct CommandOutput {
    pub status_code: StatusCode,
    pub stdout: String,
}

pub trait CommandExecutor {
    fn run(&self, command: &Command) -> io::Result<CommandOutput>;
}

pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn run(&self, command: &Command) -> io::Result<CommandOutput> {
        let mut stdout = None;
        let mut commands = command
            .inner
            .iter()
            .map(|(command, arguments)| {
                let mut command = process::Command::new(command);
                command.args(arguments);
                command.stdin(Stdio::piped());
                command.stdout(Stdio::piped());
                if let Some(stdout) = stdout.take() {
                    command.stdin(Stdio::from(stdout));
                }

                let mut command = command.spawn()?;
                stdout = command.stdout.take();
                return Ok(command);
            })
            .collect::<io::Result<Vec<_>>>()?;

        let mut status_code = StatusCode::Ok;
        for command in &mut commands {
            let output = command.wait()?;

            // Only update status code if all previous commands were successful
            if matches!(status_code, StatusCode::Ok) {
                status_code = output.code().into();
            }
        }

        let mut output = String::new();
        if let Some(mut stdout) = stdout {
            if let Some(output_file) = command.output_file.as_ref() {
                copy(&mut stdout, &mut File::create(output_file)?)?;
            } else {
                stdout.read_to_string(&mut output)?;
                output.truncate(output.trim_end().len());
            }
        }

        return Ok(CommandOutput {
            status_code,
            stdout: output,
        });
    }
}
