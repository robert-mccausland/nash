use std::collections::HashMap;

use crate::{
    components::{ControlFlowOptions, EvaluationException, EvaluationResult},
    constants::UNDERSCORE,
    ExecutionError, Executor,
};

use super::{
    builtins,
    root::Function,
    values::{Type, Value},
};

pub struct Stack {
    functions: HashMap<String, Function>,
    scopes: Vec<Scope>,
    call_stack: Vec<String>,
}

impl Stack {
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
        self.scopes.push(Scope::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn assign_variable(
        &mut self,
        variable_name: &str,
        value: Value,
    ) -> Result<(), ExecutionError> {
        // TODO - maybe we can make this unsupported if we drop tuple destructing?
        if variable_name == UNDERSCORE {
            return Ok(());
        }

        if let Some(variable) = self.get_variable_mut(variable_name) {
            if !variable.mutable {
                return Err(format!("Can't assign to a variable that is not mutable").into());
            }
            Self::set_variable(variable, value)?;
        } else {
            return Err(format!("Couldn't find variable with name: {variable_name}.").into());
        }

        Ok(())
    }

    pub fn declare_variable_init(
        &mut self,
        variable_name: &str,
        value: Value,
        mutable: bool,
    ) -> Result<(), ExecutionError> {
        self.declare_variable(variable_name, value.get_type(), mutable, Some(value))
    }

    pub fn declare_variable_uninit(
        &mut self,
        variable_name: &str,
        value_type: Type,
    ) -> Result<(), ExecutionError> {
        self.declare_variable(variable_name, value_type, true, None)
    }

    fn declare_variable(
        &mut self,
        variable_name: &str,
        value_type: Type,
        mutable: bool,
        initial_value: Option<Value>,
    ) -> Result<(), ExecutionError> {
        if value_type == Type::Void {
            return Err("Variables must not be declared with a type of void".into());
        }

        // TODO - ensure these still get assigned to the scope?
        if variable_name == UNDERSCORE {
            return Ok(());
        }

        let last_scope = self.scopes.last_mut().unwrap();
        last_scope.declare_variable(variable_name, value_type, mutable);

        if let Some(initial_value) = initial_value {
            Self::set_variable(
                last_scope.get_variable_mut(variable_name).unwrap(),
                initial_value,
            )?;
        }

        Ok(())
    }

    fn set_variable(variable: &mut Variable, value: Value) -> Result<(), ExecutionError> {
        let variable_type = &variable.value_type;
        let value_type = value.get_type();
        if !variable_type.is_assignable_to(&value_type) {
            return Err(format!(
                "Can not assign a value of type {value_type} to a variable of type {variable_type}"
            )
            .into());
        }
        variable.value = Some(value);

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

    pub fn execute_function<E: Executor>(
        &mut self,
        function_name: &str,
        instance: Option<Value>,
        arguments: Vec<Value>,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        if self.call_stack.len() >= executor.options().max_call_stack_depth {
            return Err(format!(
                "Call stack depth limit of {} exceeded",
                executor.options().max_call_stack_depth
            )
            .into());
        }

        self.call_stack.push(function_name.to_owned());
        let result = if let Some(instance) = instance {
            builtins::call_builtin_instance(function_name, &instance, &arguments, executor)?
        } else if let Some(function) = self.functions.get(function_name) {
            self.call_function(function.clone(), arguments, executor)?
        } else {
            builtins::call_builtin(function_name, &arguments, executor)?
        };

        self.call_stack.pop();
        return Ok(result);
    }

    fn call_function<E: Executor>(
        &mut self,
        function: Function,
        arguments: Vec<Value>,
        executor: &mut E,
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
                    stack.declare_variable_init(&name.value, value, false)?;
                }

                Ok(())
            },
            self,
            executor,
        );

        self.scopes = outer_scope;
        let result = if let Err(exception) = result {
            match exception {
                EvaluationException::ControlFlow(ControlFlowOptions::Return(value)) => value,
                EvaluationException::ControlFlow(ControlFlowOptions::Break()) => {
                    return Err("Break must be used in a loop block".into())
                }
                EvaluationException::ControlFlow(ControlFlowOptions::Continue()) => {
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
struct Scope {
    variables: HashMap<String, Variable>,
    hidden_variables: Vec<Variable>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            hidden_variables: Vec::new(),
        }
    }

    pub fn declare_variable(&mut self, variable_name: &str, value_type: Type, mutable: bool) {
        if variable_name == UNDERSCORE {
            return;
        }

        if let Some(old) = self
            .variables
            .insert(variable_name.to_owned(), Variable::new(value_type, mutable))
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
    pub mutable: bool,
}

impl Variable {
    fn new(value_type: Type, mutable: bool) -> Self {
        Self {
            value: None,
            value_type,
            mutable,
        }
    }
}
