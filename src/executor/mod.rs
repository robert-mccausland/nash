use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    io::{stderr, stdin, stdout, BufRead, BufReader, Write},
    rc::Rc,
};

use commands::CommandExecutor;

use crate::{
    components::{function::Function, root::Root, Identifier},
    constants::UNDERSCORE,
    errors::ExecutionError,
    utils::formatting::fmt_collection,
    SystemCommandExecutor,
};

pub mod builtins;
pub mod commands;

pub struct ExecutorOptions {
    max_call_stack_depth: usize,
}

impl ExecutorOptions {
    fn default() -> Self {
        Self {
            max_call_stack_depth: 64,
        }
    }
}

pub struct ExecutorContext {
    pub command_executor: Box<dyn CommandExecutor>,
    pub stdin: Box<dyn BufRead>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
    pub options: ExecutorOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Value {
    #[default]
    Void,
    String(String),
    Integer(i32),
    Boolean(bool),
    Command(commands::Command),
    Array(Rc<RefCell<Vec<Value>>>),
    Tuple(Vec<Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Void => f.write_str("void")?,
            Value::String(data) => {
                f.write_str("\"")?;
                f.write_str(&data.replace("\"", "\\\""))?;
                f.write_str("\"")?;
            }
            Value::Integer(data) => data.fmt(f)?,
            Value::Boolean(data) => data.fmt(f)?,
            Value::Command(data) => data.fmt(f)?,
            Value::Array(data) => fmt_collection("[", ",", "]", data.borrow().iter(), f)?,
            Value::Tuple(data) => fmt_collection("(", ",", ")", data.iter(), f)?,
        };

        return Ok(());
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Integer(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<commands::Command> for Value {
    fn from(value: commands::Command) -> Self {
        Value::Command(value)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Value::Array(Rc::new(RefCell::new(
            value.into_iter().map(Into::into).collect::<Vec<_>>(),
        )))
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

    pub(crate) fn execute(&mut self, root: &Root) -> Result<(), ExecutionError> {
        let mut stack = ExecutorStack::new();

        root.execute(&mut stack, &mut self.context)
            .map_err(|mut err| {
                err.set_call_stack(stack.call_stack);
                return err;
            })?;

        return Ok(());
    }
}

pub struct ExecutorStack {
    functions: HashMap<String, Function>,
    scopes: Vec<ExecutorScope>,
    call_stack: Vec<String>,
}

impl ExecutorStack {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            scopes: Vec::new(),
            call_stack: Vec::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(ExecutorScope::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn assign_variable(
        &mut self,
        value: Value,
        variable_name: &str,
    ) -> Result<(), ExecutionError> {
        if variable_name == UNDERSCORE {
            return Ok(());
        }

        if let Some(variable) = self.get_variable_mut(variable_name) {
            *variable = value;
        } else {
            return Err(format!("Couldn't find variable with name: {variable_name}.").into());
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

        let last_scope = self.scopes.last_mut().unwrap();
        last_scope.declare_variable(value, variable_name);

        Ok(())
    }

    pub fn declare_function(
        &mut self,
        function: Function,
        function_name: &str,
    ) -> Result<(), ExecutionError> {
        if function_name == UNDERSCORE {
            return Err(format!("Function name must not be _").into());
        }

        if let Some(_) = self.functions.get(function_name) {
            return Err(format!("Function with name {function_name} already exists").into());
        }

        self.functions.insert(function_name.to_owned(), function);

        Ok(())
    }

    pub fn resolve_variable(&self, variable_name: &str) -> Result<Value, ExecutionError> {
        Ok(self
            .get_variable(variable_name)
            .ok_or::<ExecutionError>(
                format!("Couldn't find variable with name: {variable_name}.").into(),
            )?
            .clone())
    }

    pub fn execute_function(
        &mut self,
        function_name: &str,
        arguments: Vec<Value>,
        context: &mut ExecutorContext,
    ) -> Result<Value, ExecutionError> {
        if self.call_stack.len() >= context.options.max_call_stack_depth {
            return Err(format!(
                "Call stack depth limit of {} exceeded",
                context.options.max_call_stack_depth
            )
            .into());
        }

        self.call_stack.push(function_name.to_owned());
        let result = if let Some(function) = self.functions.get(function_name) {
            // Would be nice to avoid cloning here - but would have to solve some mutability problems
            function.clone().code.execute(self, context)?;
            Value::Void
        } else {
            builtins::call_builtin(function_name, &arguments, context)?
        };
        self.call_stack.pop();
        return Ok(result);
    }

    fn get_variable(&self, variable_name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get_variable(variable_name) {
                return Some(value);
            }
        }

        None
    }

    fn get_variable_mut(&mut self, variable_name: &str) -> Option<&mut Value> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(value) = scope.get_variable_mut(variable_name) {
                return Some(value);
            }
        }

        None
    }
}
struct ExecutorScope {
    variables: HashMap<String, Value>,
    hidden_variables: Vec<Value>,
}

impl ExecutorScope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            hidden_variables: Vec::new(),
        }
    }

    pub fn declare_variable(&mut self, value: Value, variable_name: &str) {
        if variable_name == UNDERSCORE {
            return;
        }

        if let Some(old) = self.variables.insert(variable_name.to_owned(), value) {
            // Keep any hidden variables in scope until this scope ends, this means
            // the deconstruction of them will still happen at the same time if they
            // are hidden.
            self.hidden_variables.push(old);
        }
    }

    pub fn get_variable(&self, variable_name: &str) -> Option<&Value> {
        self.variables.get(variable_name)
    }

    pub fn get_variable_mut(&mut self, variable_name: &str) -> Option<&mut Value> {
        self.variables.get_mut(variable_name)
    }
}
