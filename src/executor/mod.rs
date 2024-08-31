use std::io::{self, stderr, stdin, stdout, BufRead, BufReader, Stderr, Stdin, Stdout, Write};

use commands::{Pipeline, PipelineOutput};

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

pub trait Executor
where
    Self::Stdin: BufRead,
    Self::Stdout: Write,
    Self::Stderr: Write,
{
    type Stdin;
    type Stdout;
    type Stderr;

    fn stdin(&mut self) -> &mut Self::Stdin;
    fn stdout(&mut self) -> &mut Self::Stdout;
    fn stderr(&mut self) -> &mut Self::Stderr;
    fn run_pipeline(&self, pipeline: &Pipeline) -> io::Result<PipelineOutput>;
    fn options(&self) -> &ExecutorOptions;
}

pub struct SystemExecutor {
    options: ExecutorOptions,
    stdin: <SystemExecutor as Executor>::Stdin,
    stdout: <SystemExecutor as Executor>::Stdout,
    stderr: <SystemExecutor as Executor>::Stderr,
}

impl SystemExecutor {
    pub fn new(options: ExecutorOptions) -> Self {
        Self {
            options,
            stdin: BufReader::new(stdin()),
            stdout: stdout(),
            stderr: stderr(),
        }
    }
}

impl Executor for SystemExecutor {
    type Stdin = BufReader<Stdin>;

    type Stdout = Stdout;

    type Stderr = Stderr;

    fn stdin(&mut self) -> &mut Self::Stdin {
        &mut self.stdin
    }

    fn stdout(&mut self) -> &mut Self::Stdout {
        &mut self.stdout
    }

    fn stderr(&mut self) -> &mut Self::Stderr {
        &mut self.stderr
    }

    fn run_pipeline(&self, pipeline: &Pipeline) -> io::Result<PipelineOutput> {
        system_command_executor::run_pipeline(pipeline)
    }

    fn options(&self) -> &ExecutorOptions {
        &self.options
    }
}
