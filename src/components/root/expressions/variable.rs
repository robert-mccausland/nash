use std::fmt::Debug;

use serde::Serialize;

use crate::{
    components::{
        builtins::{get_builtin_instance_type, get_builtin_type},
        root::identifier::Identifier,
        stack::Stack,
        values::{Type, Value},
        EvaluationResult, PostProcessContext, Tokens,
    },
    errors::PostProcessError,
    lexer::TokenValue,
    Executor,
};

use super::{Expression, ExpressionComponent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VariableExpression {
    name: Identifier,
    arguments: Option<Vec<Expression>>,
}

impl VariableExpression {
    pub fn get_type_on_instance(
        &self,
        instance_type: Type,
        context: &mut PostProcessContext,
    ) -> Result<Type, PostProcessError> {
        self.get_type_impl(Some(instance_type), context)
    }

    fn get_type_impl(
        &self,
        instance_type: Option<Type>,
        context: &mut PostProcessContext,
    ) -> Result<Type, PostProcessError> {
        if let Some(arguments) = &self.arguments {
            let argument_types = arguments
                .iter()
                .map(|arg| arg.get_type(context))
                .collect::<Result<Vec<_>, PostProcessError>>()?;

            if let Some(instance_type) = instance_type {
                if let Some(return_type) = get_builtin_instance_type(
                    &self.name.value,
                    instance_type,
                    argument_types.as_slice(),
                ) {
                    return Ok(return_type);
                }

                return Err(format!(
                    "Instance function with name '{}' and type signature ({}) not found",
                    &self.name.value,
                    argument_types
                        .iter()
                        .map(|t| format!("{t}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .into());
            }

            let Some(function) = context.functions.get(&self.name.value) else {
                if let Some(return_type) =
                    get_builtin_type(&self.name.value, argument_types.as_slice())
                {
                    return Ok(return_type);
                }

                return Err(format!(
                    "Function with name '{}' and type signature ({}) not found",
                    &self.name.value,
                    argument_types
                        .iter()
                        .map(|t| format!("{t}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .into());
            };

            if function.0 != argument_types {
                return Err("Arguments are not correct".into());
            }

            let return_type = function.1.clone();

            return Ok(return_type);
        } else if let Some(instance_type) = instance_type {
            let Type::Object(value) = instance_type else {
                return Err(format!("Unable to get field on non-object type").into());
            };

            let field_type = value.get(&self.name.value).ok_or::<PostProcessError>(
                format!("No field with name '{}' found on object", &self.name.value).into(),
            )?;

            return Ok(field_type.value.clone());
        } else {
            let variable_name = self.name.value.as_str();
            let Some(value_type) = context.find_variable(variable_name) else {
                return Err(format!("Variable '{variable_name}' has not been declared").into());
            };

            return Ok(value_type.variable_type);
        }
    }
}

impl ExpressionComponent for VariableExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        let token = tokens.peek_value();
        if let Some(TokenValue::Identifier(identifier)) = token {
            tokens.next();
            let name = (*identifier).into();
            let mut arguments = None;

            if matches!(tokens.peek_value(), Some(TokenValue::LeftBracket())) {
                tokens.next();
                let mut args = Vec::new();
                if let Some(TokenValue::RightBracket()) = tokens.peek_value() {
                    tokens.next();
                } else {
                    loop {
                        args.push(Expression::parse(tokens)?);
                        let next = tokens.next_value();
                        if let Some(TokenValue::RightBracket()) = next {
                            break;
                        } else if let Some(TokenValue::Comma()) = next {
                        } else {
                            return Err(
                                "Expected function argument to be followed by `,` or `)`".into()
                            );
                        }
                    }
                }

                arguments = Some(args)
            }

            return Ok(Some(VariableExpression { name, arguments }));
        }

        return Ok(None);
    }

    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        self.evaluate_on_instance(None, stack, executor)
    }

    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        self.get_type_impl(None, context)
    }
}

impl VariableExpression {
    pub fn evaluate_on_instance<E: Executor>(
        &self,
        instance: Option<Value>,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        Ok(if let Some(arguments) = &self.arguments {
            let arguments = arguments
                .iter()
                .map(|x| x.evaluate(stack, executor))
                .collect::<Result<Vec<_>, _>>()?;

            stack
                .execute_function(&self.name.value, instance, arguments, executor)?
                .into()
        } else if instance.is_none() {
            stack.resolve_variable(&self.name.value)?.into()
        } else {
            let Some(Value::Object(object)) = instance else {
                panic!("Instance variable must only be on object types");
            };

            let object = object.borrow();
            let value = object
                .get(&self.name.value)
                .expect("Attempted to access instance variable that does not exit");

            value.value.clone()
        })
    }
}
