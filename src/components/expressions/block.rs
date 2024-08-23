use serde::Serialize;

use crate::{
    components::{block::Block, Evaluatable, EvaluationResult, Parsable, Tokens},
    executor::{ExecutorContext, ExecutorStack, Value},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    ParserError,
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
    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        Ok(self.inner.execute(stack, context)?)
    }
}
