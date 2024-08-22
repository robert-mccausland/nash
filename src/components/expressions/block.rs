use serde::Serialize;

use crate::{
    components::{block::Block, Evaluatable, Tokens},
    executor::Value,
    lexer::TokenValue,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockExpression {
    inner: Block,
}

impl Evaluatable for BlockExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::ParserError> {
        let Some(TokenValue::LeftCurly()) = tokens.peek_value() else {
            return Ok(None);
        };

        return Ok(Some(BlockExpression {
            inner: Block::parse(tokens)?,
        }));
    }

    fn evaluate(
        &self,
        stack: &mut crate::executor::ExecutorStack,
        context: &mut crate::executor::ExecutorContext,
    ) -> Result<crate::executor::Value, crate::ExecutionError> {
        if let Some(_) = self.inner.execute(stack, context)? {
            return Err("Flow control options not supported in block expressions".into());
        }
        Ok(Value::Void)
    }
}
