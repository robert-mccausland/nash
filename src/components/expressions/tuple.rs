use serde::Serialize;

use crate::{
    components::{Evaluatable, Tokens},
    executor::Value,
    lexer::TokenValue,
};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TupleExpression {
    values: Vec<Expression>,
}

impl Evaluatable for TupleExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        if let Some(TokenValue::LeftBracket()) = tokens.peek_value() {
            tokens.next();
            let mut values = Vec::new();
            if let Some(TokenValue::RightBracket()) = tokens.peek_value() {
                tokens.next();
            } else {
                loop {
                    values.push(Expression::parse(tokens)?);

                    let next = tokens.next_value();
                    let Some(TokenValue::Comma()) = next else {
                        if let Some(TokenValue::RightBracket()) = next {
                            break;
                        }
                        return Err("Expected , or ) after tuple value".into());
                    };
                }
            }

            return Ok(Some(TupleExpression { values }));
        }

        return Ok(None);
    }

    fn evaluate(
        &self,
        stack: &mut crate::executor::ExecutorStack,
        context: &mut crate::executor::ExecutorContext,
    ) -> Result<crate::executor::Value, crate::errors::ExecutionError> {
        Ok(Value::Tuple(
            self.values
                .iter()
                .map(|expression| expression.evaluate(stack, context))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}
