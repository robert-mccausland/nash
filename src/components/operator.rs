use serde::Serialize;

use crate::{
    executor::{commands::OutputSource, Value},
    lexer::{Token, TokenValue},
};

use super::{errors::ExecutionError, Backtrackable, Tokens};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, PartialOrd, Ord)]
pub enum Operator {
    LessThan,
    GreaterThan,
    Addition,
    Equal,
    NotEqual,
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
                    return Some(Operator::Equal);
                }
            }

            if let TokenValue::Bang() = next {
                let next = tokens.next_value();
                if let Some(TokenValue::Equals()) = next {
                    return Some(Operator::NotEqual);
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
            (Operator::Equal, left, right) => Ok(Value::Boolean(left == right)),
            (Operator::NotEqual, left, right) => Ok(Value::Boolean(left != right)),
            (Operator::Pipe, Value::Command(left), Value::Command(right)) => Ok(Value::Command(
                left.try_pipe_command(right, OutputSource::Stdout)
                    .ok_or::<ExecutionError>(
                        "Unable to pipe command if it has already been piped to a file".into(),
                    )?,
            )),
            (Operator::Pipe, Value::Command(left), Value::String(right)) => Ok(Value::Command(
                left.try_pipe_file(right, OutputSource::Stdout)
                    .ok_or::<ExecutionError>(
                        "Unable to pipe command if it has already been piped to a file".into(),
                    )?,
            )),
            (operator, left, right) => {
                Err(format!("Invalid operator expression {left:?} {operator:?} {right:?}.").into())
            }
        }
    }
}
