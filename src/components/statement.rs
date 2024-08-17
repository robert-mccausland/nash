use crate::{
    constants::VAR,
    executor::{ExecutorContext, ExecutorStack, Value},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

use super::{
    errors::{ExecutionError, ParserError},
    expression::Expression,
    Identifier, Tokens,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Declaration(Assignment, Expression),
    Assignment(Assignment, Expression),
    Expression(Expression),
}

impl Statement {
    pub(super) fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Statement, ParserError> {
        let statement = Self::parse_content(tokens)?;
        let Some(TokenValue::Semicolon()) = tokens.next_value() else {
            return Err("statement must end with ;".into());
        };
        return Ok(statement);
    }

    pub fn execute(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> Result<(), ExecutionError> {
        match self {
            Statement::Assignment(assignment, expression) => {
                let result = expression.execute(stack, context)?;
                match assignment {
                    Assignment::Simple(identifier) => {
                        stack.assign_variable(result, &identifier.value)?;
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
                            stack.assign_variable(result, &identifier.value)?;
                        }
                    }
                }
            }
            Statement::Declaration(assignment, expression) => {
                let result = expression.execute(stack, context)?;
                match assignment {
                    Assignment::Simple(identifier) => {
                        stack.declare_variable(result, &identifier.value)?;
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
                            stack.declare_variable(result, &identifier.value)?;
                        }
                    }
                }
            }
            Statement::Expression(expression) => {
                let value = expression.execute(stack, context)?;

                if let Value::Void = value {
                } else {
                    if let Err(err) = writeln!(&mut context.stdout, "{:}", value) {
                        return Err(format!("Error writing to stdout: {err}").into());
                    }
                }
            }
        };

        return Ok(());
    }

    fn parse_content<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Statement, ParserError> {
        let next = tokens.peek_value();
        // Any statement starting with var must be a declaration
        if let Some(TokenValue::Keyword(VAR)) = next {
            tokens.next();
            let assignment = Assignment::try_parse(tokens)
                .ok_or::<ParserError>("var must be followed by an assignment".into())?;
            return Ok(Statement::Declaration(
                assignment,
                Expression::parse(tokens)?,
            ));
        }

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

#[derive(Debug, Clone, PartialEq, Eq)]
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
