use serde::Serialize;

use crate::{
    executor::{ExecutorContext, ExecutorStack},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

use super::{
    errors::{ExecutionError, ParserError},
    statement::Statement,
    Tokens,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    pub statements: Vec<Statement>,
}

impl Block {
    pub(super) fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Block, ParserError> {
        let mut statements = Vec::new();

        let Some(TokenValue::LeftCurly()) = tokens.next_value() else {
            return Err("code block must start with {".into());
        };

        loop {
            if let Some(TokenValue::RightCurly()) = tokens.peek_value() {
                tokens.next();
                break;
            };

            statements.push(Statement::parse(tokens)?);
        }

        return Ok(Block { statements });
    }

    pub fn execute(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> Result<(), ExecutionError> {
        for statement in &self.statements {
            statement.execute(stack, context)?;
        }
        return Ok(());
    }
}
