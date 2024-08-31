use crate::{
    components::{stack::ExecutorStack, values::Value, Evaluatable, EvaluationResult, Parsable},
    constants::{FALSE, TRUE},
    errors::ParserError,
    executor::ExecutorContext,
    lexer::{Token, TokenValue},
};

mod command;
mod string;

pub use command::CommandLiteral;
use serde::Serialize;
pub use string::StringLiteral;

use super::{Backtrackable, Tokens};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegerLiteral {
    pub value: u32,
}

impl IntegerLiteral {
    fn parse_impl(value: &str) -> Result<IntegerLiteral, ParserError> {
        let Ok(value) = u32::from_str_radix(value, 10) else {
            return Err(format!("Unable to parse {value} as a number").into());
        };

        if TryInto::<i32>::try_into(value).is_err() {
            return Err(format!("Number is out of range for an integer").into());
        }
        return Ok(value.into());
    }
}

impl From<u32> for IntegerLiteral {
    fn from(value: u32) -> Self {
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
        Ok(Value::Integer(self.value.try_into().unwrap()).into())
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
