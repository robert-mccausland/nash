use std::io;

use serde::Serialize;

pub trait CommandExecutor {
    fn run(&self, command: &Pipeline) -> io::Result<PipelineOutput>;
}

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
pub struct PipelineOutput {
    pub stdout: Option<String>,
    pub command_outputs: Vec<CommandOutput>,
}

impl PipelineOutput {
    pub fn new<I: IntoIterator<Item = CommandOutput>>(
        stdout: Option<String>,
        command_outputs: I,
    ) -> Self {
        Self {
            stdout,
            command_outputs: command_outputs.into_iter().collect::<Vec<_>>(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub exit_code: u8,
    pub stderr: Option<String>,
}

impl CommandOutput {
    pub fn new(exit_code: u8, stderr: Option<String>) -> Self {
        Self { exit_code, stderr }
    }
}

impl From<u8> for CommandOutput {
    fn from(value: u8) -> Self {
        Self::new(value, None)
    }
}
