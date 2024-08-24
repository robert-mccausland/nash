use serde::Serialize;

use crate::{
    constants::{BREAK, CONTINUE, EXIT, RETURN, VAR},
    executor::{ExecutorContext, ExecutorStack, Value},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    ExecutionError,
};

use super::{
    errors::ParserError, expressions::Expression, type_definition::TypeDefinition,
    EvaluationResult, Identifier, Tokens,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Statement {
    Declaration(Identifier, TypeDefinition),
    DeclarationAssignment(Assignment, Expression),
    Assignment(Assignment, Expression),
    Expression(Expression),
    Exit(Expression),
    Return(Expression),
    Break(),
    Continue(),
}

pub enum ControlFlowOptions {
    Exit(u8),
    Return(Value),
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

    pub fn execute(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        match self {
            Statement::Declaration(variable_name, type_definition) => {
                stack.declare_variable(&variable_name.value, type_definition.value.clone())?;
            }
            Statement::Assignment(assignment, expression) => {
                let result = expression.evaluate(stack, context)?;
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
            Statement::DeclarationAssignment(assignment, expression) => {
                let result = expression.evaluate(stack, context)?;
                match assignment {
                    Assignment::Simple(identifier) => {
                        stack.declare_and_assign_variable(&identifier.value, result)?;
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
                            stack.declare_and_assign_variable(&identifier.value, result)?;
                        }
                    }
                }
            }
            Statement::Expression(expression) => {
                expression.evaluate(stack, context)?;
            }
            Statement::Return(expression) => {
                let result = expression.evaluate(stack, context)?;
                return Err(ControlFlowOptions::Return(result).into());
            }
            Statement::Exit(expression) => {
                let Value::Integer(value) = expression.evaluate(stack, context)? else {
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
            return if let Some(assignment) = Assignment::try_parse(tokens) {
                Ok(Statement::DeclarationAssignment(
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
