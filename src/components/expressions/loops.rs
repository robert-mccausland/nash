use serde::Serialize;

use crate::{
    components::{block::Block, Evaluatable, Identifier, Tokens},
    constants::{FOR, IN},
    executor::Value,
    lexer::TokenValue,
};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForLoopExpression {
    item_name: Identifier,
    array_expression: Box<Expression>,
    loop_body: Block,
}

impl Evaluatable for ForLoopExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        if let Some(TokenValue::Keyword(FOR)) = tokens.peek_value() {
            tokens.next();

            let Some(TokenValue::Identifier(item_name)) = tokens.next_value() else {
                return Err("expected identifier".into());
            };

            let Some(TokenValue::Keyword(IN)) = tokens.next_value() else {
                return Err("expected keyword in".into());
            };

            let array_expression = Expression::parse(tokens)?;
            let loop_body = Block::parse(tokens)?;

            return Ok(Some(ForLoopExpression {
                item_name: (*item_name).into(),
                array_expression: Box::new(array_expression),
                loop_body,
            }));
        }

        return Ok(None);
    }

    fn evaluate(
        &self,
        stack: &mut crate::executor::ExecutorStack,
        context: &mut crate::executor::ExecutorContext,
    ) -> Result<crate::executor::Value, crate::errors::ExecutionError> {
        let Value::Array(array) = self.array_expression.evaluate(stack, context)? else {
            return Err("for ... in loop must be used on an array value".into());
        };

        // Make sure variable is defined before we assign in
        stack.declare_variable(Value::Void, &self.item_name.value)?;
        for item in array.as_ref().borrow().iter() {
            stack.assign_variable(item.clone(), &self.item_name.value)?;
            self.loop_body.execute(stack, context)?;
        }

        Ok(Value::Void)
    }
}
