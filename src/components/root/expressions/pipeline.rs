use serde::Serialize;

use crate::{
    components::{
        root::identifier::Identifier,
        stack::Stack,
        values::{FileMode, Value},
        Evaluatable, EvaluationResult, Parsable, Tokens,
    },
    constants::{AS, CAP, EXEC},
    errors::ExecutionError,
    executor::Executor,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    CommandDefinition, ParserError, Pipeline, PipelineDestination, PipelineSource,
};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PipelineCommand {
    expression: Expression,
    capture_stderr: Option<Identifier>,
    capture_exit_code: Option<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PipelineExpression {
    commands: Vec<PipelineCommand>,
}

impl PipelineCommand {
    fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Self, ParserError> {
        let expression = Expression::parse(tokens)?;
        let mut capture_stderr = None;
        let mut capture_exit_code = None;
        if let Some(TokenValue::LeftSquare()) = tokens.peek_value() {
            tokens.next();
            loop {
                if let Some(TokenValue::RightSquare()) = tokens.peek_value() {
                    tokens.next();
                    break;
                }

                let (identifier, alias) = Self::parse_option(tokens)?;
                match identifier.value.as_str() {
                    "exit_code" => capture_exit_code = Some(alias),
                    "stderr" => capture_stderr = Some(alias),
                    other => {
                        return Err(format!(
                            "Trying to capture unrecognized item: {other} in command options"
                        )
                        .into())
                    }
                }

                if let Some(TokenValue::Comma()) = tokens.peek_value() {
                    tokens.next();
                } else if let Some(TokenValue::RightSquare()) = tokens.peek_value() {
                    // Allow omitting trailing comma
                } else {
                    return Err("Expected , or as after item in command options".into());
                }
            }
        }

        return Ok(Self {
            expression,
            capture_exit_code,
            capture_stderr,
        });
    }

    fn parse_option<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<(Identifier, Identifier), ParserError> {
        let Some(TokenValue::Keyword(CAP)) = tokens.peek_value() else {
            return Err(format!("Expected cap after [ or , in command options").into());
        };
        tokens.next();

        let Some(identifier) = Identifier::try_parse(tokens)? else {
            return Err(format!("Expected identifier after cap").into());
        };

        let alias = if let Some(TokenValue::Keyword(AS)) = tokens.peek_value() {
            tokens.next();
            let Some(alias) = Identifier::try_parse(tokens)? else {
                return Err(format!("Expected identifier after as in command options").into());
            };

            alias
        } else {
            identifier.clone()
        };

        return Ok((identifier, alias));
    }
}

impl Parsable for PipelineExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        let Some(TokenValue::Keyword(EXEC)) = tokens.peek_value() else {
            return Ok(None);
        };
        tokens.next();

        let mut commands = Vec::new();
        commands.push(PipelineCommand::parse(tokens)?);

        loop {
            let Some(TokenValue::Equals()) = tokens.peek_value() else {
                break;
            };
            tokens.next();

            let Some(TokenValue::RightAngle()) = tokens.peek_value() else {
                return Err(format!("Expected > after = in execute pipeline").into());
            };
            tokens.next();

            commands.push(PipelineCommand::parse(tokens)?);
        }

        return Ok(Some(Self { commands }));
    }
}

impl Evaluatable for PipelineExpression {
    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        let mut pipeline = Pipeline {
            commands: Vec::new(),
            source: None,
            destination: None,
        };

        let mut commands = self.commands.iter();
        let first = commands.next().unwrap();
        let first_value = first.expression.evaluate(stack, executor)?;

        if let Value::String(literal) = first_value {
            pipeline.source = Some(PipelineSource::Literal(literal + "\n"))
        } else if let Value::FileHandle(path, mode) = first_value {
            match mode {
                FileMode::Open => pipeline.source = Some(PipelineSource::File(path)),
                _ => {
                    return Err(
                        format!("File must be in open mode to be used as data source").into(),
                    )
                }
            }
        } else if let Value::Command(program, arguments) = first_value {
            pipeline.commands.push(CommandDefinition {
                program,
                arguments,
                capture_stderr: first.capture_stderr.is_some(),
            })
        } else {
            return Err(format!("Invalid type used in command pipeline").into());
        }

        for command in commands {
            if pipeline.destination.is_some() {
                return Err(
                    format!("Destination must be the last element of a command pipeline").into(),
                );
            }
            let command_value = command.expression.evaluate(stack, executor)?;
            if let Value::Command(program, arguments) = command_value {
                pipeline.commands.push(CommandDefinition {
                    program,
                    arguments,
                    capture_stderr: command.capture_stderr.is_some(),
                })
            } else if let Value::FileHandle(path, mode) = command_value {
                match mode {
                    FileMode::Write => {
                        pipeline.destination = Some(PipelineDestination::FileWrite(path))
                    }
                    FileMode::Append => {
                        pipeline.destination = Some(PipelineDestination::FileAppend(path))
                    }
                    _ => {
                        return Err(format!(
                            "File must be in write or append mode to be used as a destination"
                        )
                        .into())
                    }
                }
            } else {
                return Err(format!("Invalid type used in command pipeline").into());
            }
        }

        let result = executor
            .run_pipeline(&pipeline)
            .map_err::<ExecutionError, _>(|err| {
                format!("Error running command: {:}", err).into()
            })?;

        let mut local_commands = self.commands.iter();
        if pipeline.source.is_some() {
            local_commands.next();
        }

        for (command_output, command) in result.command_outputs.into_iter().zip(local_commands) {
            if let Some(capture_exit_code) = &command.capture_exit_code {
                stack.declare_variable_init(
                    &capture_exit_code.value,
                    (command_output.exit_code as i32).into(),
                    true,
                )?;
            } else if command_output.exit_code != 0 {
                return Err(format!(
                    "Command returned non-zero exit code: ({})",
                    command_output.exit_code
                )
                .into());
            }

            if let Some(capture_stderr) = &command.capture_stderr {
                stack.declare_variable_init(
                    &capture_stderr.value,
                    command_output.stderr.unwrap_or_default().into(),
                    true,
                )?;
            }
        }

        return Ok(result.stdout.unwrap_or_default().into());
    }
}
