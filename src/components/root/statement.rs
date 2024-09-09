use serde::Serialize;

use crate::{
    components::{
        stack::Stack,
        values::{Type, Value},
        ControlFlowOptions, EvaluationResult, PostProcessContext, ScopeType,
    },
    constants::{BREAK, CONTINUE, EXIT, MUT, RETURN, VAR},
    errors::PostProcessError,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    ExecutionError, Executor, ParserError,
};

use super::{
    expressions::Expression, identifier::Identifier, type_definition::TypeDefinition, Tokens,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Statement {
    Declaration(Identifier, TypeDefinition),
    DeclarationAssignment(bool, Assignment, Expression),
    Assignment(Assignment, Expression),
    Expression(Expression),
    Exit(Expression),
    Return(Expression),
    Break(),
    Continue(),
}

impl Statement {
    pub(super) fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Statement, ParserError> {
        let statement = Self::parse_content(tokens)?;
        let Some(TokenValue::Semicolon()) = tokens.peek_value() else {
            return Err("statement must end with ;".into());
        };
        tokens.next();
        return Ok(statement);
    }

    pub fn post_process(&self, context: &mut PostProcessContext) -> Result<(), PostProcessError> {
        match self {
            Statement::Declaration(name, variable_type) => {
                context.declare_variable(name.value.clone(), variable_type.value.clone());
            }
            Statement::DeclarationAssignment(_, assignment, value) => {
                let variable_type = value.get_type(context)?;
                match assignment {
                    Assignment::Simple(name) => {
                        context.declare_variable(name.value.clone(), variable_type)
                    }
                    Assignment::Tuple(_) => todo!(),
                }
            }
            Statement::Assignment(assignment, value) => match assignment {
                Assignment::Simple(name) => {
                    let name = name.value.clone();
                    let variable_type = context.find_variable(name.as_str()).ok_or::<PostProcessError>(
                        format!("Unable to assign to variable '{name}' has it has not been declared yet").into()
                    )?;
                    let value_type = value.get_type(context)?;
                    if !value_type.is_assignable_to(&variable_type) {
                        return Err(format!(
                            "Unable to assign a value of type '{value_type}' to a variable of type '{variable_type}'",
                        ).into());
                    }
                }
                Assignment::Tuple(_) => todo!(),
            },
            Statement::Expression(value) => {
                value.get_type(context)?;
            }
            Statement::Exit(value) => {
                let Type::Integer = value.get_type(context)? else {
                    return Err("Value provided to an exit statement must be an integer".into());
                };

                // Currently everything is inside a root scope, but idk maybe at some point it wont be.
                dbg!(&context);
                if !context.has_parent_scope(&ScopeType::Root) {
                    return Err("Exit statement can only be used from inside the root scope".into());
                }
            }
            Statement::Return(value) => {
                let Some(scope) = context.get_matching_parent_scope(|scope_type| {
                    matches!(scope_type, ScopeType::Function(_))
                }) else {
                    return Err("Return statement can only be used from inside a function".into());
                };

                let ScopeType::Function(declared_return_type) = &scope.scope_type else {
                    panic!()
                };

                let declared_return_type = declared_return_type.clone();
                let actual_return_type = value.get_type(context)?;
                if actual_return_type != declared_return_type {
                    return Err(format!("Function has a declared return type of {declared_return_type}, but return statement got a type of {actual_return_type}").into());
                }
            }
            Statement::Break() => {
                if !context.has_parent_scope(&ScopeType::Looped) {
                    return Err(
                        "Break statement can only be used from inside a looped block".into(),
                    );
                }
            }
            Statement::Continue() => {
                if !context.has_parent_scope(&ScopeType::Looped) {
                    return Err(
                        "Continue statement can only be used from inside a looped block".into(),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn execute<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        match self {
            Statement::Declaration(variable_name, type_definition) => {
                stack
                    .declare_variable_uninit(&variable_name.value, type_definition.value.clone())?;
            }
            Statement::Assignment(assignment, expression) => {
                let result = expression.evaluate(stack, executor)?;
                match assignment {
                    Assignment::Simple(identifier) => {
                        stack.assign_variable(&identifier.value, result)?;
                    }
                    Assignment::Tuple(identifiers) => {
                        let Value::Tuple(result) = result else {
                            return Err(
                                "Can't use a tuple assignment with a non-tuple value".into()
                            );
                        };

                        if identifiers.len() > result.len() {
                            return Err("Not enough values in tuple to fill assignment".into());
                        }

                        for (identifier, result) in identifiers.iter().zip(result) {
                            stack.assign_variable(&identifier.value, result)?;
                        }
                    }
                }
            }
            Statement::DeclarationAssignment(mutable, assignment, expression) => {
                let result = expression.evaluate(stack, executor)?;
                match assignment {
                    Assignment::Simple(identifier) => {
                        stack.declare_variable_init(&identifier.value, result, *mutable)?;
                    }
                    Assignment::Tuple(identifiers) => {
                        let Value::Tuple(result) = result else {
                            return Err(
                                "Can't use a tuple assignment with a non-tuple value".into()
                            );
                        };

                        if identifiers.len() > result.len() {
                            return Err("Not enough values in tuple to fill assignment".into());
                        }

                        for (identifier, result) in identifiers.iter().zip(result) {
                            stack.declare_variable_init(&identifier.value, result, true)?;
                        }
                    }
                }
            }
            Statement::Expression(expression) => {
                expression.evaluate(stack, executor)?;
            }
            Statement::Return(expression) => {
                let result = expression.evaluate(stack, executor)?;
                return Err(ControlFlowOptions::Return(result).into());
            }
            Statement::Exit(expression) => {
                let Value::Integer(value) = expression.evaluate(stack, executor)? else {
                    return Err("exit statement must be provided with an integer value".into());
                };

                let exit_code = value.try_into().map_err::<ExecutionError, _>(|_| {
                    format!("exit code must be between 0 and 255, but got {value}").into()
                })?;

                return Err(ControlFlowOptions::Exit(exit_code).into());
            }
            Statement::Break() => return Err(ControlFlowOptions::Break().into()),
            Statement::Continue() => return Err(ControlFlowOptions::Continue().into()),
        };

        return Ok(Value::Void);
    }

    fn parse_content<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Statement, ParserError> {
        let next = tokens.peek_value();
        // Any statement starting with var must be a declaration
        if let Some(TokenValue::Keyword(VAR)) = next {
            tokens.next();

            let mutable = if let Some(TokenValue::Keyword(MUT)) = tokens.peek_value() {
                tokens.next();
                true
            } else {
                false
            };

            return if let Some(assignment) = Assignment::try_parse(tokens) {
                Ok(Statement::DeclarationAssignment(
                    mutable,
                    assignment,
                    Expression::parse(tokens)?,
                ))
            } else {
                let Some(TokenValue::Identifier(value)) = tokens.peek_value() else {
                    return Err("var must be followed by assignment or identifier".into());
                };
                tokens.next();
                let Some(TokenValue::Colon()) = tokens.peek_value() else {
                    return Err("variable declaration must be followed by a :".into());
                };
                tokens.next();

                let type_definition = TypeDefinition::parse(tokens)?;

                if !mutable {
                    return Err("Uninitialized variable must be mutable".into());
                }

                Ok(Statement::Declaration((*value).into(), type_definition))
            };
        }

        if let Some(TokenValue::Keyword(EXIT)) = next {
            tokens.next();
            return Ok(Statement::Exit(Expression::parse(tokens)?));
        };

        if let Some(TokenValue::Keyword(RETURN)) = next {
            tokens.next();
            return Ok(Statement::Return(Expression::parse(tokens)?));
        };

        if let Some(TokenValue::Keyword(BREAK)) = next {
            tokens.next();
            return Ok(Statement::Break());
        };

        if let Some(TokenValue::Keyword(CONTINUE)) = next {
            tokens.next();
            return Ok(Statement::Continue());
        };

        if let Some(assignment) = Assignment::try_parse(tokens) {
            return Ok(Statement::Assignment(
                assignment,
                Expression::parse(tokens)?,
            ));
        }

        // Otherwise it might be a bare expression
        let expression = Expression::parse(tokens)?;
        return Ok(Statement::Expression(expression));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Assignment {
    Simple(Identifier),
    Tuple(Vec<Identifier>),
}

impl Assignment {
    pub fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Option<Self> {
        tokens.backtrack_if_none(Self::parse_impl)
    }

    fn parse_impl<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Option<Self> {
        let next = tokens.next_value();
        if let Some(TokenValue::Identifier(identifier)) = next {
            return if let Some(TokenValue::Equals()) = tokens.next_value() {
                Some(Assignment::Simple((*identifier).into()))
            } else {
                None
            };
        };

        if let Some(TokenValue::LeftBracket()) = next {
            let mut identifiers = Vec::new();
            let mut next = tokens.next_value();
            if let Some(TokenValue::RightBracket()) = next {
            } else {
                loop {
                    let Some(TokenValue::Identifier(identifier)) = next else {
                        return None;
                    };
                    identifiers.push((*identifier).into());
                    next = tokens.next_value();
                    let Some(TokenValue::Comma()) = next else {
                        if let Some(TokenValue::RightBracket()) = next {
                            break;
                        }
                        return None;
                    };
                    next = tokens.next_value();
                }
            }
            return if let Some(TokenValue::Equals()) = tokens.next_value() {
                Some(Assignment::Tuple(identifiers))
            } else {
                None
            };
        };

        return None;
    }
}
