use serde::Serialize;

use crate::{
    components::{
        root::block::Block,
        stack::Stack,
        values::{Type, Value},
        EvaluationResult, PostProcessContext, Tokens,
    },
    errors::PostProcessError,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    Executor, ParserError,
};

use super::ExpressionComponent;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockExpression {
    inner: Block,
}

impl ExpressionComponent for BlockExpression {
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

    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        Ok(self.inner.execute(stack, executor)?)
    }

    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        // Make sure to post process the block itself
        self.inner.post_process(context)?;

        Ok(Type::Void)
    }
}
