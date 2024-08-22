use serde::Serialize;

use crate::{
    executor::{ExecutorContext, ExecutorStack},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

use super::{
    errors::{ExecutionError, ParserError},
    statement::{ControlFlowOptions, Statement},
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
    ) -> Result<Option<ControlFlowOptions>, ExecutionError> {
        self.execute_with_initializer(|_| Ok(()), stack, context)
    }

    pub fn execute_with_initializer<F: FnOnce(&mut ExecutorStack) -> Result<(), ExecutionError>>(
        &self,
        initialize: F,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> Result<Option<ControlFlowOptions>, ExecutionError> {
        stack.push_scope();
        initialize(stack)?;
        for statement in &self.statements {
            if let Some(control_flow) = statement.execute(stack, context)? {
                return Ok(Some(control_flow));
            }
        }
        stack.pop_scope();
        return Ok(None);
    }
}
