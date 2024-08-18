use std::{
    collections::HashMap,
    fmt::Display,
    io::{stderr, stdin, stdout, BufRead, BufReader, Write},
};

use commands::CommandExecutor;

use crate::{
    components::{function::Function, root::Root},
    constants::UNDERSCORE,
    errors::ExecutionError,
    SystemCommandExecutor,
};

pub mod builtins;
pub mod commands;

pub struct ExecutorContext {
    pub command_executor: Box<dyn CommandExecutor>,
    pub stdin: Box<dyn BufRead>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Value {
    #[default]
    Void,
    String(String),
    Integer(i32),
    Boolean(bool),
    Command(commands::Command),
    Tuple(Vec<Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Void => f.write_str("void")?,
            Value::String(data) => {
                f.write_str("\"")?;
                f.write_str(data)?;
                f.write_str("\"")?;
            }
            Value::Integer(data) => data.fmt(f)?,
            Value::Boolean(data) => data.fmt(f)?,
            Value::Command(data) => data.fmt(f)?,
            Value::Tuple(data) => {
                f.write_str("(")?;
                let mut first = true;
                for element in data {
                    if !first {
                        f.write_str(",")?;
                    } else {
                        first = false;
                    }
                    element.fmt(f)?;
                }
                f.write_str(")")?;
            }
        };

        return Ok(());
    }
}
pub struct Executor {
    context: ExecutorContext,
}

impl Executor {
    pub fn new<
        T: CommandExecutor + 'static,
        Stdin: BufRead + 'static,
        Stdout: Write + 'static,
        Stderr: Write + 'static,
    >(
        command_executor: T,
        stdin: Stdin,
        stdout: Stdout,
        stderr: Stderr,
    ) -> Self {
        Executor {
            context: ExecutorContext {
                command_executor: Box::new(command_executor),
                stdin: Box::new(stdin),
                stdout: Box::new(stdout),
                stderr: Box::new(stderr),
            },
        }
    }

    pub fn default() -> Self {
        Self::new(
            SystemCommandExecutor::new(),
            BufReader::new(stdin()),
            stdout(),
            stderr(),
        )
    }

    pub(crate) fn execute(&mut self, root: &Root) -> Result<(), ExecutionError> {
        let mut stack = ExecutorStack {
            functions: HashMap::new(),
            variables: HashMap::new(),
        };

        root.execute(&mut stack, &mut self.context)?;

        return Ok(());
    }
}

pub struct ExecutorStack {
    pub variables: HashMap<String, Value>,
    pub functions: HashMap<String, Function>,
}

impl ExecutorStack {
    pub fn assign_variable(
        &mut self,
        value: Value,
        variable_name: &str,
    ) -> Result<(), ExecutionError> {
        if variable_name == UNDERSCORE {
            return Ok(());
        }

        if let Some(variable) = self.variables.get_mut(variable_name) {
            *variable = value;
        } else {
            return Err("Couldn't find variable".into());
        }

        Ok(())
    }

    pub fn declare_variable(
        &mut self,
        value: Value,
        variable_name: &str,
    ) -> Result<(), ExecutionError> {
        if variable_name == UNDERSCORE {
            return Ok(());
        }
        if let Some(_) = self.variables.get(variable_name) {
            return Err("Variable already exists".into());
        } else {
            self.variables.insert(variable_name.to_owned(), value);
        }

        Ok(())
    }

    pub fn resolve_variable(&self, variable_name: &str) -> Result<Value, ExecutionError> {
        Ok(self
            .variables
            .get(variable_name)
            .ok_or::<ExecutionError>("Variable not found".into())?
            .clone())
    }
}
