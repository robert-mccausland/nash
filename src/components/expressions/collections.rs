use serde::Serialize;

use crate::{
    components::{Evaluatable, Tokens},
    executor::Value,
    lexer::TokenValue,
};

use super::Expression;

macro_rules! collection_expression_impl {
    ($expression_type:ident, $start_token:expr, $end_token:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        pub struct $expression_type {
            values: Vec<Expression>,
        }

        impl $expression_type {
            fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
                tokens: &mut crate::utils::iterators::Backtrackable<I>,
            ) -> Result<Option<Self>, crate::errors::ParserError> {
                if let Some(values) = try_parse_collection(tokens, &$start_token, &$end_token)? {
                    Ok(Some($expression_type { values }))
                } else {
                    Ok(None)
                }
            }
        }
    };
}

fn try_parse_collection<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
    tokens: &mut crate::utils::iterators::Backtrackable<I>,
    start: &TokenValue,
    end: &TokenValue,
) -> Result<Option<Vec<Expression>>, crate::errors::ParserError> {
    if Some(start) == tokens.peek_value() {
        tokens.next();
        let mut values = Vec::new();
        if Some(end) == tokens.peek_value() {
            tokens.next();
        } else {
            loop {
                values.push(Expression::parse(tokens)?);

                let next = tokens.next_value();
                let Some(TokenValue::Comma()) = next else {
                    if Some(end) == next {
                        break;
                    }
                    return Err("Unable to parse collection".into());
                };
            }
        }

        return Ok(Some(values));
    }

    return Ok(None);
}

fn evaluate_collection(
    values: &Vec<Expression>,
    stack: &mut crate::executor::ExecutorStack,
    context: &mut crate::executor::ExecutorContext,
) -> Result<Vec<crate::executor::Value>, crate::errors::ExecutionError> {
    values
        .iter()
        .map(|expression| expression.evaluate(stack, context))
        .collect::<Result<Vec<_>, _>>()
}

collection_expression_impl!(
    TupleExpression,
    TokenValue::LeftBracket(),
    TokenValue::RightBracket()
);

impl Evaluatable for TupleExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        TupleExpression::try_parse(tokens)
    }

    fn evaluate(
        &self,
        stack: &mut crate::executor::ExecutorStack,
        context: &mut crate::executor::ExecutorContext,
    ) -> Result<Value, crate::errors::ExecutionError> {
        Ok(Value::Tuple(evaluate_collection(
            &self.values,
            stack,
            context,
        )?))
    }
}

collection_expression_impl!(
    ArrayExpression,
    TokenValue::LeftSquare(),
    TokenValue::RightSquare()
);

impl Evaluatable for ArrayExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        ArrayExpression::try_parse(tokens)
    }

    fn evaluate(
        &self,
        stack: &mut crate::executor::ExecutorStack,
        context: &mut crate::executor::ExecutorContext,
    ) -> Result<Value, crate::errors::ExecutionError> {
        let values = evaluate_collection(&self.values, stack, context)?;
        let mut array_types = values.iter().map(|x| x.get_type());
        let Some(array_type) = array_types.next() else {
            return Err("Unable to determine array type for empty array".into());
        };
        for item in array_types {
            if item != array_type {
                return Err("Array must have all values of the same type".into());
            }
        }

        return Ok(Value::new_array(values, array_type)?);
    }
}
