use crate::{
    constants::{FALSE, TRUE},
    errors::ParserError,
    executor::{ExecutorContext, ExecutorStack, Value},
    lexer::{Token, TokenValue},
};

mod command;
mod string;

pub use command::CommandLiteral;
use serde::Serialize;
pub use string::StringLiteral;

use super::{Backtrackable, Evaluatable, EvaluationResult, Parsable, Tokens};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegerLiteral {
    pub value: i32,
}

impl IntegerLiteral {
    fn parse_impl(value: &str) -> Result<IntegerLiteral, ParserError> {
        let Ok(value) = i32::from_str_radix(value, 10) else {
            return Err(format!("Unable to parse {value} as a number").into());
        };
        return Ok(value.into());
    }
}

impl From<i32> for IntegerLiteral {
    fn from(value: i32) -> Self {
        IntegerLiteral { value }
    }
}

impl Parsable for IntegerLiteral {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        Ok(
            if let Some(TokenValue::IntegerLiteral(value)) = tokens.peek_value() {
                tokens.next();
                Some(Self::parse_impl(value)?)
            } else {
                None
            },
        )
    }
}

impl Evaluatable for IntegerLiteral {
    fn evaluate(
        &self,
        _stack: &mut ExecutorStack,
        _context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        Ok(Value::Integer(self.value).into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BooleanLiteral {
    pub value: bool,
}

impl From<bool> for BooleanLiteral {
    fn from(value: bool) -> Self {
        BooleanLiteral { value }
    }
}
impl Parsable for BooleanLiteral {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        Ok(
            if let Some(TokenValue::Keyword(TRUE)) = tokens.peek_value() {
                tokens.next();
                Some(true.into())
            } else if let Some(TokenValue::Keyword(FALSE)) = tokens.peek_value() {
                tokens.next();
                Some(false.into())
            } else {
                None
            },
        )
    }
}

impl Evaluatable for BooleanLiteral {
    fn evaluate(
        &self,
        _stack: &mut ExecutorStack,
        _context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        Ok(Value::Boolean(self.value).into())
    }
}
