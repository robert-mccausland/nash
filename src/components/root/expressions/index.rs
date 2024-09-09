use serde::Serialize;

use crate::{
    components::{
        stack::Stack,
        values::{Type, Value},
        EvaluationResult, PostProcessContext, Tokens,
    },
    errors::PostProcessError,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    Executor, ParserError,
};

use super::{BaseExpression, DependentExpressionComponent, Expression};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndexExpression {
    inner: Box<BaseExpression>,
    index: Box<Expression>,
}

impl DependentExpressionComponent for IndexExpression {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        inner: BaseExpression,
        tokens: &mut Backtrackable<I>,
    ) -> Result<Result<Self, BaseExpression>, ParserError> {
        let Some(TokenValue::LeftSquare()) = tokens.peek_value() else {
            return Ok(Err(inner));
        };
        tokens.next();

        let index = Expression::parse(tokens)?;
        let Some(TokenValue::RightSquare()) = tokens.peek_value() else {
            return Err("Expected closing ] after index expression".into());
        };
        tokens.next();

        return Ok(Ok(IndexExpression {
            inner: Box::new(inner),
            index: Box::new(index),
        }));
    }

    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        let index = self.index.evaluate(stack, executor)?;
        let Value::Integer(index) = index else {
            return Err("Index expression must evaluate to integer".into());
        };
        let index = TryInto::<usize>::try_into(index)
            .map_err(|_| "Index expression must be a positive integer")?;

        let inner_value = self.inner.evaluate(stack, executor)?;

        let Value::Array(array, _, _) = inner_value else {
            return Err("Can only index values of type array".into());
        };
        let array = array.borrow();
        let result = array.get(index).ok_or(format!(
            "Index value must be less than array length, array has length {} and got index {}.",
            array.len(),
            index
        ))?;

        return Ok(result.clone());
    }

    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        let Type::Array(inner_type, _) = self.inner.get_type(context)? else {
            return Err("Expression must evaluate to an array".into());
        };

        let Type::Integer = self.index.get_type(context)? else {
            return Err("Index expression must evaluate to integer".into());
        };

        return Ok(*inner_type);
    }
}
