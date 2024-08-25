use serde::Serialize;

use crate::{
    executor::{commands::OutputSource, Value},
    lexer::{Token, TokenValue},
};

use super::{errors::ExecutionError, Backtrackable, Tokens};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Operator {
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Remainder,
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Pipe,
    And,
    Or,
}

impl Operator {
    pub(super) fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Option<Operator> {
        tokens.backtrack_if_none(|tokens| {
            let Some(next) = tokens.next_value() else {
                return None;
            };

            if let TokenValue::Plus() = next {
                return Some(Operator::Addition);
            }

            if let TokenValue::Dash() = next {
                return Some(Operator::Subtraction);
            }

            if let TokenValue::Star() = next {
                return Some(Operator::Multiplication);
            }

            if let TokenValue::ForwardSlash() = next {
                return Some(Operator::Division);
            }

            if let TokenValue::Percent() = next {
                return Some(Operator::Remainder);
            }

            if let TokenValue::LeftAngle() = next {
                if let Some(TokenValue::Equals()) = tokens.peek_value() {
                    tokens.next();
                    return Some(Operator::LessThanOrEqual);
                }
                return Some(Operator::LessThan);
            }

            if let TokenValue::RightAngle() = next {
                if let Some(TokenValue::Equals()) = tokens.peek_value() {
                    tokens.next();
                    return Some(Operator::GreaterThanOrEqual);
                }
                return Some(Operator::GreaterThan);
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

            if let TokenValue::And() = next {
                if let Some(TokenValue::And()) = tokens.peek_value() {
                    tokens.next();
                    return Some(Operator::And);
                }
            }

            if let TokenValue::Pipe() = next {
                if let Some(TokenValue::Pipe()) = tokens.peek_value() {
                    tokens.next();
                    return Some(Operator::Or);
                }
            }

            return None;
        })
    }

    pub fn execute(&self, left: Value, right: Value) -> Result<Value, ExecutionError> {
        match (self, left, right) {
            (Operator::Addition, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left + right))
            }
            (Operator::Subtraction, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left - right))
            }
            (Operator::Multiplication, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left * right))
            }
            (Operator::Division, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left / right))
            }
            (Operator::Remainder, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left % right))
            }
            (Operator::Equal, left, right) => Ok(Value::Boolean(left == right)),
            (Operator::NotEqual, left, right) => Ok(Value::Boolean(left != right)),
            (Operator::LessThan, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Boolean(left < right))
            }
            (Operator::GreaterThan, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Boolean(left > right))
            }
            (Operator::LessThanOrEqual, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Boolean(left <= right))
            }
            (Operator::GreaterThanOrEqual, Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Boolean(left >= right))
            }
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
            (Operator::And, Value::Boolean(left), Value::Boolean(right)) => {
                Ok(Value::Boolean(left && right))
            }
            (Operator::Or, Value::Boolean(left), Value::Boolean(right)) => {
                Ok(Value::Boolean(left || right))
            }
            (operator, left, right) => {
                Err(format!("Invalid operator expression {left:?} {operator:?} {right:?}.").into())
            }
        }
    }

    pub fn commutes_with(&self, value: &Self) -> bool {
        macro_rules! return_true_if_match {
            ($pattern:pat) => {{
                if matches!(self, $pattern) && matches!(value, $pattern) {
                    return true;
                }
            }};
        }

        return_true_if_match!(Self::Multiplication);
        return_true_if_match!(Self::Addition | Self::Subtraction);
        return_true_if_match!(Self::And | Self::Or);
        return_true_if_match!(Self::Pipe);

        return false;
    }
}
