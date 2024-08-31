use std::io::{stderr, stdin, stdout, BufRead, BufReader, Write};

use crate::CommandExecutor;
use system_command_executor::SystemCommandExecutor;
pub mod commands;
mod system_command_executor;

pub struct ExecutorOptions {
    pub max_call_stack_depth: usize,
}

impl ExecutorOptions {
    pub fn default() -> Self {
        Self {
            max_call_stack_depth: 64,
        }
    }
}

pub struct ExecutorContext<'a> {
    pub command_executor: Box<dyn CommandExecutor + 'a>,
    pub stdin: Box<dyn BufRead + 'a>,
    pub stdout: Box<dyn Write + 'a>,
    pub stderr: Box<dyn Write + 'a>,
    pub options: ExecutorOptions,
}

pub struct Executor<'a> {
    pub context: ExecutorContext<'a>,
}

impl<'a> Executor<'a> {
    pub fn new<
        T: CommandExecutor + 'a,
        Stdin: BufRead + 'a,
        Stdout: Write + 'a,
        Stderr: Write + 'a,
    >(
        command_executor: T,
        stdin: Stdin,
        stdout: Stdout,
        stderr: Stderr,
        options: ExecutorOptions,
    ) -> Self {
        Executor {
            context: ExecutorContext {
                command_executor: Box::new(command_executor),
                stdin: Box::new(stdin),
                stdout: Box::new(stdout),
                stderr: Box::new(stderr),
                options,
            },
        }
    }

    pub fn default() -> Self {
        Self::new(
            SystemCommandExecutor::new(),
            BufReader::new(stdin()),
            stdout(),
            stderr(),
            ExecutorOptions::default(),
        )
    }
}
