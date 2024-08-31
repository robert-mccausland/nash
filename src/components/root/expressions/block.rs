use serde::Serialize;

use crate::{
    components::{
        root::block::Block, stack::Stack, values::Value, Evaluatable, EvaluationResult, Parsable,
        Tokens,
    },
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    Executor, ParserError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockExpression {
    inner: Block,
}

impl Parsable for BlockExpression {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        let Some(TokenValue::LeftCurly()) = tokens.peek_value() else {
            return Ok(None);
        };

        Ok(Some(BlockExpression {
            inner: Block::parse(tokens)?,
        }))
    }
}

impl Evaluatable for BlockExpression {
    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E
,
    ) -> EvaluationResult<Value> {
        Ok(self.inner.execute(stack, executor)?)
    }
}
