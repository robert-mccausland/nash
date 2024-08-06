use crate::parser::operators::Operator;

use super::{commands::OutputSource, ExecutionError, Value};

pub fn execute_operator_expression(
    operator: &Operator,
    left: Value,
    right: Value,
) -> Result<Value, ExecutionError> {
    match (operator, left, right) {
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

        (operator, left, right) => Err(ExecutionError::new(format!(
            "Invalid operator expression {left:?} {operator:?} {right:?}."
        ))),
    }
}
