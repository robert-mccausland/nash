use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    io::{stderr, stdin, stdout, BufRead, BufReader, Write},
    rc::Rc,
};

use commands::CommandExecutor;
use serde::Serialize;

use crate::{
    components::{function::Function, root::Root},
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Value {
    #[default]
    Void,
    String(String),
    Integer(i32),
    Boolean(bool),
    Command(commands::Command),
    Array(Rc<RefCell<Vec<Value>>>, Type),
    Tuple(Vec<Value>),
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Value::Void => Type::Void,
            Value::String(_) => Type::String,
            Value::Integer(_) => Type::Integer,
            Value::Boolean(_) => Type::Boolean,
            Value::Command(_) => Type::Command,
            Value::Array(_, value_type) => Type::Array(value_type.clone().into()),
            Value::Tuple(values) => {
                Type::Tuple(values.iter().map(|x| x.get_type()).collect::<Vec<_>>())
            }
        }
    }

    pub fn new_array<I: IntoIterator<Item = T>, T: Into<Value>>(
        values: I,
        array_type: Type,
    ) -> Result<Value, ExecutionError> {
        let values = values
            .into_iter()
            .map(|value| {
                let value = value.into();
                if value.get_type() != array_type {
                    Err("Array item did not match array type")
                } else {
                    Ok(value)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self::Array(Rc::new(RefCell::new(values)), array_type))
    }
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
            Value::Array(data, _) => fmt_collection("[", ",", "]", data.borrow().iter(), f)?,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Type {
    Void,
    String,
    Integer,
    Boolean,
    Command,
    Array(Box<Type>),
    Tuple(Vec<Type>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => f.write_str("void"),
            Type::String => f.write_str("string"),
            Type::Integer => f.write_str("integer"),
            Type::Boolean => f.write_str("boolean"),
            Type::Command => f.write_str("command"),
            Type::Array(array_type) => {
                f.write_str("[")?;
                array_type.fmt(f)?;
                f.write_str("]")?;

                Ok(())
            }
            Type::Tuple(item_types) => fmt_collection("(", ",", ")", item_types.iter(), f),
        }
    }
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

    pub fn declare_and_assign_variable(
        &mut self,
        variable_name: &str,
        value: Value,
    ) -> Result<(), ExecutionError> {
        self.declare_variable(variable_name, value.get_type())?;
        self.assign_variable(variable_name, value)?;

        Ok(())
    }

    pub fn assign_variable(
        &mut self,
        variable_name: &str,
        value: Value,
    ) -> Result<(), ExecutionError> {
        if variable_name == UNDERSCORE {
            return Ok(());
        }

        if let Some(variable) = self.get_variable_mut(variable_name) {
            let variable_type = &variable.value_type;
            let value_type = value.get_type();
            if *variable_type != value_type {
                return Err(
                    format!("Can not assign a value of type {variable_type} to a variable of type {value_type}").into(),
                );
            }
            variable.value = Some(value);
        } else {
            return Err(format!("Couldn't find variable with name: {variable_name}.").into());
        }

        Ok(())
    }

    pub fn declare_variable(
        &mut self,
        variable_name: &str,
        value_type: Type,
    ) -> Result<(), ExecutionError> {
        if variable_name == UNDERSCORE {
            return Ok(());
        }

        let last_scope = self.scopes.last_mut().unwrap();
        last_scope.declare_variable(variable_name, value_type);

        Ok(())
    }

    pub fn declare_function(
        &mut self,
        function_name: &str,
        function: Function,
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
            .value
            .clone()
            .ok_or::<ExecutionError>(
                format!("Variable {variable_name} has not been initialized.").into(),
            )?)
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
            if function.arguments.len() != arguments.len() {
                return Err(format!(
                    "Function {} requires {} arguments, but {} were provided",
                    function.name.value,
                    function.arguments.len(),
                    arguments.len()
                )
                .into());
            }

            // Would be nice to avoid cloning here - but would have to solve some mutability problems
            let function = function.clone();
            function.code.execute_with_initializer(
                |stack| {
                    for (value, name) in arguments.into_iter().zip(function.arguments) {
                        stack.declare_and_assign_variable(&name.value, value)?;
                    }

                    Ok(())
                },
                self,
                context,
            )?;

            Value::Void
        } else {
            builtins::call_builtin(function_name, &arguments, context)?
        };
        self.call_stack.pop();
        return Ok(result);
    }

    fn get_variable(&self, variable_name: &str) -> Option<&Variable> {
        for scope in self.scopes.iter().rev() {
            if let Some(variable) = scope.get_variable(variable_name) {
                return Some(variable);
            }
        }

        None
    }

    fn get_variable_mut(&mut self, variable_name: &str) -> Option<&mut Variable> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(variable) = scope.get_variable_mut(variable_name) {
                return Some(variable);
            }
        }

        None
    }
}
struct ExecutorScope {
    variables: HashMap<String, Variable>,
    hidden_variables: Vec<Variable>,
}

impl ExecutorScope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            hidden_variables: Vec::new(),
        }
    }

    pub fn declare_variable(&mut self, variable_name: &str, value_type: Type) {
        if variable_name == UNDERSCORE {
            return;
        }

        if let Some(old) = self
            .variables
            .insert(variable_name.to_owned(), Variable::new(value_type))
        {
            // Keep any hidden variables in scope until this scope ends, this means
            // the deconstruction of them will still happen at the same time if they
            // are hidden.
            self.hidden_variables.push(old);
        }
    }

    pub fn get_variable(&self, variable_name: &str) -> Option<&Variable> {
        self.variables.get(variable_name)
    }

    pub fn get_variable_mut(&mut self, variable_name: &str) -> Option<&mut Variable> {
        self.variables.get_mut(variable_name)
    }
}

struct Variable {
    pub value: Option<Value>,
    pub value_type: Type,
}

impl Variable {
    fn new(value_type: Type) -> Self {
        Self {
            value: None,
            value_type,
        }
    }
}
