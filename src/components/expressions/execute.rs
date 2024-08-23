use serde::Serialize;

use crate::{
    components::{Evaluatable, EvaluationResult, Identifier, Parsable, Tokens},
    constants::{EXEC, VAR},
    errors::ExecutionError,
    executor::{commands::StatusCode, Value},
    lexer::TokenValue,
};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecuteExpression {
    inner: Expression,
    capture_exit_code: Option<CaptureExitCode>,
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
                let token = tokens.next_value();
                if let Some(TokenValue::Keyword(VAR)) = token {
                    let Some(TokenValue::Identifier(identifier)) = tokens.next_value() else {
                        return Err("var must be followed by identifier".into());
                    };
                    capture_exit_code = Some(CaptureExitCode::Declaration((*identifier).into()));
                } else if let Some(TokenValue::Identifier(identifier)) = token {
                    capture_exit_code = Some(CaptureExitCode::Assignment((*identifier).into()));
                } else {
                    return Err("? must be followed by an var or identifier".into());
                }
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
                StatusCode::Value(code) => (
                    Value::Tuple(vec![
                        Value::String(result.stdout),
                        Value::String(result.stderr),
                    ]),
                    code,
                ),
            };

            match &self.capture_exit_code {
                Some(CaptureExitCode::Assignment(identifier)) => {
                    stack.assign_variable(&identifier.value, Value::Integer(exit_code.into()))?;
                }
                Some(CaptureExitCode::Declaration(identifier)) => {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CaptureExitCode {
    Assignment(Identifier),
    Declaration(Identifier),
}
