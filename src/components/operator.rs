use serde::Serialize;

use crate::{
    executor::{commands::OutputSource, Value},
    lexer::{Token, TokenValue},
};

use super::{errors::ExecutionError, Backtrackable, Tokens};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Operator {
    LessThan,
    GreaterThan,
    Addition,
    Equals,
    Pipe,
}

impl Operator {
    pub(super) fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Option<Operator> {
        tokens.backtrack_if_none(|tokens| {
            let Some(next) = tokens.next_value() else {
                return None;
            };

            if let TokenValue::LeftAngle() = next {
                return Some(Operator::LessThan);
            }

            if let TokenValue::RightAngle() = next {
                return Some(Operator::GreaterThan);
            }

            if let TokenValue::Plus() = next {
                return Some(Operator::Addition);
            }

            if let TokenValue::Equals() = next {
                let next = tokens.next_value();
                if let Some(TokenValue::RightAngle()) = next {
                    return Some(Operator::Pipe);
                }

                if let Some(TokenValue::Equals()) = next {
                    return Some(Operator::Equals);
                }
            }

            return None;
        })
    }

    pub fn execute(&self, left: Value, right: Value) -> Result<Value, ExecutionError> {
        match (self, left, right) {
            (Operator::LessThan, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Boolean(left < right))
            }
            (Operator::GreaterThan, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Boolean(left > right))
            }
            (Operator::Addition, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left + right))
            }
            (Operator::Equals, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Boolean(left == right))
            }
            (Operator::Pipe, Value::Command(left), Value::Command(right)) => Ok(Value::Command(
                left.try_pipe_command(right, OutputSource::Stdout)
                    .ok_or::<ExecutionError>("Command already has a file it is piping to".into())?,
            )),
            (Operator::Pipe, Value::Command(left), Value::String(right)) => Ok(Value::Command(
                left.try_pipe_file(right, OutputSource::Stdout)
                    .ok_or::<ExecutionError>("Command already has a file it is piping to".into())?,
            )),
            (operator, left, right) => {
                Err(format!("Invalid operator expression {left:?} {operator:?} {right:?}.").into())
            }
        }
    }
}
