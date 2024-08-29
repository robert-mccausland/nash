use std::io::{stderr, stdin, stdout, BufRead, BufReader, Write};

use crate::{
    components::root::Root, errors::ExecutionError, CommandExecutor, SystemCommandExecutor,
};

pub use stack::ExecutorStack;
pub use values::{FileMode, Type, Value};

pub mod builtins;
pub mod commands;
mod stack;
mod values;

pub struct ExecutorOptions {
    max_call_stack_depth: usize,
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
    context: ExecutorContext<'a>,
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

    pub(crate) fn execute(&mut self, root: &Root) -> Result<ExecutionOutput, ExecutionError> {
        let mut stack = ExecutorStack::new();

        let exit_code = root
            .execute(&mut stack, &mut self.context)
            .map_err(|mut err| {
                err.set_call_stack(stack.get_call_stack().clone());
                return err;
            })?;

        return Ok(ExecutionOutput::new(exit_code));
    }
}

pub struct ExecutionOutput {
    exit_code: u8,
}

impl ExecutionOutput {
    fn new(exit_code: u8) -> Self {
        Self { exit_code }
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}
