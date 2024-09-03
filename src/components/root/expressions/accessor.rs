use serde::Serialize;

use crate::{
    components::{stack::Stack, values::Value, EvaluationResult, Parsable, Tokens},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    Executor, ParserError,
};

use super::{variable::VariableExpression, BaseExpression};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessorExpression {
    inner: Box<BaseExpression>,
    accessor: Accessor,
}

impl AccessorExpression {
    pub fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        inner: BaseExpression,
        tokens: &mut Backtrackable<I>,
    ) -> Result<Result<Self, BaseExpression>, ParserError> {
        let Some(accessor) = Accessor::try_parse(tokens)? else {
            return Ok(Err(inner));
        };

        return Ok(Ok(AccessorExpression {
            inner: Box::new(inner),
            accessor,
        }));
    }

    pub fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        let inner = self.inner.evaluate(stack, executor)?;
        Ok(match &self.accessor {
            Accessor::Integer(integer) => {
                let Value::Tuple(mut values) = inner else {
                    return Err("Cannot use get expression on non-tuple value".into());
                };

                let len = values.len();
                let result = values.get_mut(*integer as usize).ok_or(format!(
                    "Cannot get element at index {:} because tuple only has {:} elements",
                    integer, len
                ))?;

                core::mem::take(result).into()
            }
            Accessor::Variable(variable) => {
                variable.evaluate_on_instance(Some(inner), stack, executor)?
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Accessor {
    Integer(u32),
    Variable(VariableExpression),
}

impl Accessor {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        let Some(TokenValue::Dot()) = tokens.peek_value() else {
            return Ok(None);
        };
        tokens.next();

        if let Some(TokenValue::IntegerLiteral(integer)) = tokens.peek_value() {
            tokens.next();
            let value = u32::from_str_radix(integer, 10).map_err::<ParserError, _>(|_| {
                "Numeric accessors must be positive integers {err}".into()
            })?;
            return Ok(Some(Accessor::Integer(value)));
        }

        if let Some(value) = VariableExpression::try_parse(tokens)? {
            return Ok(Some(Accessor::Variable(value)));
        }

        return Err("Unable to parse accessor".into());
    }
}
