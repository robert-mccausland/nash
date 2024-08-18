use serde::Serialize;

use crate::{
    components::{errors::ParserError, Evaluatable, Tokens},
    errors::ExecutionError,
    executor::{commands::Command, ExecutorContext, ExecutorStack, Value},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

use super::string::StringLiteral;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]

pub struct CommandLiteral {
    pub command: StringLiteral,
    pub arguments: Vec<StringLiteral>,
}

impl CommandLiteral {
    pub fn new(command: StringLiteral, arguments: Vec<StringLiteral>) -> Self {
        Self { command, arguments }
    }

    fn parse_impl<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Self, ParserError> {
        let command =
            Self::parse_next_literal(tokens)?.ok_or("Command literal must contain command")?;
        let mut arguments = Vec::new();
        while let Some(next) = Self::parse_next_literal(tokens)? {
            arguments.push(next);
        }

        return Ok(CommandLiteral::new(command, arguments));
    }

    fn parse_next_literal<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<StringLiteral>, ParserError> {
        if let Some(literal) = StringLiteral::try_parse(tokens)? {
            return Ok(Some(literal));
        }

        let next = tokens.next_value();
        if let Some(TokenValue::StringLiteral(value)) = next {
            Ok(Some((*value).into()))
        } else if let Some(TokenValue::Backtick()) = next {
            Ok(None)
        } else {
            Err("Unable to parse command".into())
        }
    }
}

impl<'a, I: IntoIterator<Item = &'a str>> From<I> for CommandLiteral {
    fn from(value: I) -> Self {
        let mut iter = value.into_iter();
        CommandLiteral::new(
            iter.next().unwrap_or_default().into(),
            iter.map(|x| x.into()).collect::<Vec<_>>(),
        )
    }
}

impl Evaluatable for CommandLiteral {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        Ok(if let Some(TokenValue::Backtick()) = tokens.peek_value() {
            tokens.next();
            Some(Self::parse_impl(tokens)?)
        } else {
            None
        })
    }
    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        _context: &mut ExecutorContext,
    ) -> Result<Value, ExecutionError> {
        Ok(Value::Command(Command::new(
            self.command.resolve(stack)?,
            self.arguments
                .iter()
                .map(|argument| argument.resolve(stack))
                .collect::<Result<Vec<_>, _>>()?,
        )))
    }
}
