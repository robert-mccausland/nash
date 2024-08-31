use std::collections::HashMap;

use crate::{
    components::{ControlFlowOptions, EvaluationException, EvaluationResult},
    constants::UNDERSCORE,
    ExecutionError,
};

use super::{
    builtins,
    root::Function,
    values::{Type, Value},
    ExecutorContext,
};

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

    pub fn get_call_stack(&self) -> &Vec<String> {
        &self.call_stack
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
        if value_type == Type::Void {
            return Err("Variables must not be declared with a type of void".into());
        }

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
        instance: Option<Value>,
        arguments: Vec<Value>,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        if self.call_stack.len() >= context.options.max_call_stack_depth {
            return Err(format!(
                "Call stack depth limit of {} exceeded",
                context.options.max_call_stack_depth
            )
            .into());
        }

        self.call_stack.push(function_name.to_owned());
        let result = if let Some(instance) = instance {
            builtins::call_builtin_instance(function_name, &instance, &arguments, context)?
        } else if let Some(function) = self.functions.get(function_name) {
            self.call_function(function.clone(), arguments, context)?
        } else {
            builtins::call_builtin(function_name, &arguments, context)?
        };

        self.call_stack.pop();
        return Ok(result);
    }

    fn call_function(
        &mut self,
        function: Function,
        arguments: Vec<Value>,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        if function.arguments.len() != arguments.len() {
            return Err(format!(
                "Function {} requires {} arguments, but {} were provided",
                function.name.value,
                function.arguments.len(),
                arguments.len()
            )
            .into());
        }

        // Replace current scope with an empty scope to prevent function from accessing outer scope
        let outer_scope = core::mem::replace(&mut self.scopes, Vec::new());

        // Would be nice to avoid cloning here - but would have to solve some mutability problems
        let result = function.code.execute_with_initializer(
            |stack| {
                for (value, (name, argument_type)) in arguments.into_iter().zip(function.arguments)
                {
                    let value_type = value.get_type();
                    if argument_type.value != value_type {
                        return Err(format!(
                            "Argument {} has type {} but got value with type {}",
                            name.value, argument_type.value, value_type
                        )
                        .into());
                    }
                    stack.declare_and_assign_variable(&name.value, value)?;
                }

                Ok(())
            },
            self,
            context,
        );

        self.scopes = outer_scope;
        let result = if let Err(exception) = result {
            match exception {
                EvaluationException::AlterControlFlow(ControlFlowOptions::Return(value)) => value,
                EvaluationException::AlterControlFlow(ControlFlowOptions::Break()) => {
                    return Err("Break must be used in a loop block".into())
                }
                EvaluationException::AlterControlFlow(ControlFlowOptions::Continue()) => {
                    return Err("Continue must be used in a loop block".into())
                }
                err => return Err(err),
            }
        } else {
            Value::Void
        };

        let value_type = result.get_type();
        if value_type != function.return_type.value {
            return Err(format!(
                "Function {} should return type {} but got value with type {}",
                function.name.value, function.return_type.value, value_type
            )
            .into());
        };
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
