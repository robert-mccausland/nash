use crate::{
    constants::{ELSE, EXEC, IF, VAR},
    executor::{
        builtins,
        commands::{Command, StatusCode},
        ExecutorContext, ExecutorStack, Value,
    },
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

use super::{
    block::Block,
    errors::{ExecutionError, ParserError},
    literals::{BooleanLiteral, CommandLiteral, IntegerLiteral, StringLiteral},
    operator::Operator,
    Identifier, Tokens,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    pub operations: Vec<(Operator, BaseExpression)>,
    pub first: Box<BaseExpression>,
}

impl Expression {
    pub fn new(first: BaseExpression, operations: Vec<(Operator, BaseExpression)>) -> Self {
        Self {
            first: first.into(),
            operations,
        }
    }

    pub(super) fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Expression, ParserError> {
        let expression = BaseExpression::parse(tokens)?;
        let mut operations = Vec::new();
        while let Some(operator) = Operator::try_parse(tokens) {
            operations.push((operator, BaseExpression::parse(tokens)?));
        }

        return Ok(Expression::new(expression, operations));
    }

    pub fn execute(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> Result<Value, ExecutionError> {
        let mut result = self.first.execute(stack, context)?;
        for (operator, expression) in &self.operations {
            let right = expression.execute(stack, context)?;
            result = operator.execute(result, right)?;
        }
        return Ok(result);
    }
}

fn try_parse_accessor<'a, I: Iterator<Item = &'a Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Option<u32> {
    tokens.backtrack_if_none(|tokens| {
        let Some(TokenValue::Dot()) = tokens.next_value() else {
            return None;
        };

        let Some(TokenValue::IntegerLiteral(integer)) = tokens.next_value() else {
            return None;
        };

        return u32::from_str_radix(integer, 10).into_iter().next();
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseExpression {
    StringLiteral(StringLiteral),
    BooleanLiteral(BooleanLiteral),
    IntegerLiteral(IntegerLiteral),
    Tuple(Vec<Expression>),
    Variable(Identifier),
    Command(CommandLiteral),
    If(Vec<(Expression, Block)>, Option<Block>),
    FunctionCall(Identifier, Vec<Expression>),
    Execute(Box<Expression>, Option<CaptureExitCode>),
    Accessor(Box<BaseExpression>, u32),
}

impl BaseExpression {
    fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<BaseExpression, ParserError> {
        let expression = if let Some(literal) = BaseExpression::try_parse_literal(tokens)? {
            literal
        } else if let Some(function_or_variable) =
            BaseExpression::try_parse_function_or_variable(tokens)?
        {
            function_or_variable
        } else if let Some(if_expression) = BaseExpression::try_parse_if(tokens)? {
            if_expression
        } else if let Some(execute) = BaseExpression::try_parse_execute(tokens)? {
            execute
        } else if let Some(tuple) = BaseExpression::try_parse_tuple(tokens)? {
            tuple
        } else {
            return Err("Unable to parse expression".into());
        };

        if let Some(accessor) = try_parse_accessor(tokens) {
            return Ok(BaseExpression::Accessor(Box::new(expression), accessor));
        } else {
            Ok(expression)
        }
    }

    fn execute(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> Result<Value, ExecutionError> {
        match self {
            BaseExpression::StringLiteral(literal) => Ok(Value::String(literal.resolve(stack)?)),
            BaseExpression::BooleanLiteral(boolean) => Ok(Value::Boolean(boolean.value)),
            BaseExpression::IntegerLiteral(integer) => Ok(Value::Integer(integer.value)),
            BaseExpression::Variable(variable_name) => stack.resolve_variable(&variable_name.value),
            BaseExpression::Tuple(elements) => Ok(Value::Tuple(
                elements
                    .iter()
                    .map(|expression| expression.execute(stack, context))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            BaseExpression::Command(command) => {
                return Ok(Value::Command(Command::new(
                    command.command.resolve(stack)?,
                    command
                        .arguments
                        .iter()
                        .map(|argument| argument.resolve(stack))
                        .collect::<Result<Vec<_>, _>>()?,
                )));
            }
            BaseExpression::If(blocks, else_block) => {
                for (condition, block) in blocks {
                    let condition_result = condition.execute(stack, context)?;
                    if let Value::Boolean(result) = condition_result {
                        if result {
                            block.execute(stack, context)?;
                            return Ok(Value::Void);
                        }
                    } else {
                        return Err("If statement condition must evaluate to a boolean".into());
                    }
                }

                if let Some(else_block) = else_block {
                    else_block.execute(stack, context)?;
                }
                return Ok(Value::Void);
            }
            BaseExpression::Execute(expression, capture_exit_code) => {
                if let Value::Command(command) = expression.execute(stack, context)? {
                    let result = context
                        .command_executor
                        .run(&command)
                        .map_err::<ExecutionError, _>(|err| {
                            format!("Error running command: {:}", err).into()
                        })?;

                    let (return_value, exit_code) = match result.status_code {
                        StatusCode::Terminated => {
                            return Err(format!("Command was terminated").into())
                        }
                        StatusCode::Value(code) => (
                            Value::Tuple(vec![
                                Value::String(result.stdout),
                                Value::String(result.stderr),
                            ]),
                            code,
                        ),
                    };

                    match capture_exit_code {
                        Some(CaptureExitCode::Assignment(identifier)) => {
                            stack.assign_variable(
                                Value::Integer(exit_code.into()),
                                &identifier.value,
                            )?;
                        }
                        Some(CaptureExitCode::Declaration(identifier)) => {
                            stack.declare_variable(
                                Value::Integer(exit_code.into()),
                                &identifier.value,
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

                    return Ok(return_value);
                }

                return Err("Value being executed must be a command".into());
            }
            BaseExpression::FunctionCall(name, args) => {
                let args = args
                    .iter()
                    .map(|arg| arg.execute(stack, context))
                    .collect::<Result<Vec<_>, _>>()?;

                // Remove function from stack when calling it to avoid double borrowing, means
                // recursion won't work, but that needs stack frames to work anyway.
                if let Some(function) = stack.functions.remove(&name.value) {
                    function.code.execute(stack, context)?;
                    stack.functions.insert(name.value.to_owned(), function);
                    Ok(Value::Void)
                } else {
                    builtins::call_builtin(&name.value, args.as_slice(), context)
                }
            }
            BaseExpression::Accessor(base_expression, index) => {
                let Value::Tuple(mut values) = base_expression.execute(stack, context)? else {
                    return Err("Cannot use get expression on non-tuple value".into());
                };

                let len = values.len();
                let result = values.get_mut(*index as usize).ok_or(format!(
                    "Cannot get element at index {:} because tuple only has {:} elements",
                    index, len
                ))?;

                return Ok(core::mem::take(result));
            }
        }
    }

    fn try_parse_literal<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<BaseExpression>, ParserError> {
        Ok(Some(
            if let Some(literal) = IntegerLiteral::try_parse(tokens)? {
                BaseExpression::IntegerLiteral(literal)
            } else if let Some(literal) = StringLiteral::try_parse(tokens)? {
                BaseExpression::StringLiteral(literal)
            } else if let Some(literal) = CommandLiteral::try_parse(tokens)? {
                BaseExpression::Command(literal)
            } else if let Some(literal) = BooleanLiteral::try_parse(tokens) {
                BaseExpression::BooleanLiteral(literal)
            } else {
                return Ok(None);
            },
        ))
    }

    fn try_parse_function_or_variable<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<BaseExpression>, ParserError> {
        let token = tokens.peek_value();
        if let Some(TokenValue::Identifier(identifier)) = token {
            tokens.next();
            if matches!(tokens.peek_value(), Some(TokenValue::LeftBracket())) {
                tokens.next();
                let mut args = Vec::new();
                if let Some(TokenValue::RightBracket()) = tokens.peek_value() {
                    tokens.next();
                } else {
                    loop {
                        args.push(Expression::parse(tokens)?);
                        let next = tokens.next_value();
                        if let Some(TokenValue::RightBracket()) = next {
                            break;
                        } else if let Some(TokenValue::Comma()) = next {
                        } else {
                            return Err(
                                "Expected function argument to be followed by `,` or `)`".into()
                            );
                        }
                    }
                }

                return Ok(Some(BaseExpression::FunctionCall(
                    (*identifier).into(),
                    args,
                )));
            } else {
                return Ok(Some(BaseExpression::Variable((*identifier).into())));
            }
        }

        return Ok(None);
    }

    fn try_parse_if<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<BaseExpression>, ParserError> {
        if let Some(TokenValue::Keyword(IF)) = tokens.peek_value() {
            tokens.next();
            let mut blocks = Vec::new();
            let mut else_block: Option<Block> = None;
            loop {
                let expression = Expression::parse(tokens)?;
                let block = Block::parse(tokens)?;
                blocks.push((expression, block));

                let Some(TokenValue::Keyword(ELSE)) = tokens.peek_value() else {
                    break;
                };

                tokens.next();
                if let Some(TokenValue::Keyword(IF)) = tokens.peek_value() {
                    tokens.next();
                } else {
                    else_block = Some(Block::parse(tokens)?);
                    break;
                }
            }

            return Ok(Some(BaseExpression::If(blocks, else_block)));
        }

        return Ok(None);
    }

    fn try_parse_execute<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<BaseExpression>, ParserError> {
        if let Some(TokenValue::Keyword(EXEC)) = tokens.peek_value() {
            tokens.next();
            let expression = Expression::parse(tokens)?;
            if let Some(TokenValue::Question()) = tokens.peek_value() {
                tokens.next();
                let token = tokens.next_value();
                if let Some(TokenValue::Keyword(VAR)) = token {
                    let Some(TokenValue::Identifier(identifier)) = tokens.next_value() else {
                        return Err("var must be followed by identifier".into());
                    };
                    return Ok(Some(BaseExpression::Execute(
                        Box::new(expression),
                        Some(CaptureExitCode::Declaration((*identifier).into())),
                    )));
                } else if let Some(TokenValue::Identifier(identifier)) = token {
                    return Ok(Some(BaseExpression::Execute(
                        Box::new(expression),
                        Some(CaptureExitCode::Assignment((*identifier).into())),
                    )));
                } else {
                    return Err("? must be followed by an var or identifier".into());
                }
            } else {
                return Ok(Some(BaseExpression::Execute(Box::new(expression), None)));
            }
        }

        return Ok(None);
    }

    fn try_parse_tuple<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<BaseExpression>, ParserError> {
        if let Some(TokenValue::LeftBracket()) = tokens.peek_value() {
            tokens.next();
            let mut expressions = Vec::new();
            if let Some(TokenValue::RightBracket()) = tokens.peek_value() {
                tokens.next();
            } else {
                loop {
                    expressions.push(Expression::parse(tokens)?);

                    let next = tokens.next_value();
                    let Some(TokenValue::Comma()) = next else {
                        if let Some(TokenValue::RightBracket()) = next {
                            break;
                        }
                        return Err("Expected , or ) after tuple value".into());
                    };
                }
            }

            return Ok(Some(BaseExpression::Tuple(expressions)));
        }

        return Ok(None);
    }
}

impl Into<Expression> for BaseExpression {
    fn into(self) -> Expression {
        Expression {
            first: self.into(),
            operations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]

pub enum CaptureExitCode {
    Assignment(Identifier),
    Declaration(Identifier),
}
