use serde::Serialize;

use crate::{
    components::{stack::Stack, values::Value, Evaluatable, Parsable, Tokens},
    lexer::TokenValue,
    Executor,
};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BracketExpression {
    inner: Expression,
}

impl Parsable for BracketExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::ParserError> {
        let Some(TokenValue::LeftBracket()) = tokens.peek_value() else {
            return Ok(None);
        };
        let checkpoint = tokens.checkpoint();
        tokens.next();

        let inner = Expression::parse(tokens)?;

        let Some(TokenValue::RightBracket()) = tokens.peek_value() else {
            // Tuple expressions may match up to here here so we return None instead of erroring,
            // to let them flow down to the correct matcher.
            tokens.backtrack(checkpoint);
            return Ok(None);
        };
        tokens.next();

        return Ok(Some(Self { inner }));
    }
}

impl Evaluatable for BracketExpression {
    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E
,
    ) -> crate::components::EvaluationResult<Value> {
        self.inner.evaluate(stack, executor)
    }
}
