use serde::Serialize;

use crate::{
    components::{Evaluatable, EvaluationResult, Identifier, Parsable, Tokens},
    constants::EXEC,
    errors::ExecutionError,
    executor::{commands::StatusCode, Value},
    lexer::TokenValue,
};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecuteExpression {
    inner: Expression,
    capture_exit_code: Option<Identifier>,
}

impl Parsable for ExecuteExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        if let Some(TokenValue::Keyword(EXEC)) = tokens.peek_value() {
            tokens.next();
            let expression = Expression::parse(tokens)?;
            let mut capture_exit_code = None;

            if let Some(TokenValue::Question()) = tokens.peek_value() {
                tokens.next();

                let Some(TokenValue::Identifier(identifier)) = tokens.peek_value() else {
                    return Err("Expected identifier after ?".into());
                };
                tokens.next();

                capture_exit_code = Some((*identifier).into());
            }

            return Ok(Some(ExecuteExpression {
                inner: expression,
                capture_exit_code,
            }));
        }

        return Ok(None);
    }
}

impl Evaluatable for ExecuteExpression {
    fn evaluate(
        &self,
        stack: &mut crate::executor::ExecutorStack,
        context: &mut crate::executor::ExecutorContext,
    ) -> EvaluationResult<Value> {
        if let Value::Command(command) = self.inner.evaluate(stack, context)? {
            let result = context
                .command_executor
                .run(&command)
                .map_err::<ExecutionError, _>(|err| {
                    format!("Error running command: {:}", err).into()
                })?;

            let (return_value, exit_code) = match result.status_code {
                StatusCode::Terminated => return Err(format!("Command was terminated").into()),
                StatusCode::Value(code) => (Value::String(result.stdout), code),
            };

            match &self.capture_exit_code {
                Some(identifier) => {
                    stack.declare_and_assign_variable(
                        &identifier.value,
                        Value::Integer(exit_code.into()),
                    )?;
                }
                None => {
                    if exit_code != 0 {
                        return Err(format!(
                            "Command returned non-zero exit code: ({:})",
                            exit_code
                        )
                        .into());
                    }
                }
            }

            return Ok(return_value.into());
        }

        return Err("Value being executed must be a command".into());
    }
}
