use serde::Serialize;

use crate::{
    components::{block::Block, Evaluatable, Identifier, Tokens},
    constants::{FOR, IN, WHILE},
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
        let Value::Array(array, _) = self.array_expression.evaluate(stack, context)? else {
            return Err("for ... in loop must be used on an array value".into());
        };

        for item in array.as_ref().borrow().iter() {
            self.loop_body.execute_with_initializer(
                |stack| stack.declare_and_assign_variable(&self.item_name.value, item.clone()),
                stack,
                context,
            )?;
        }

        Ok(Value::Void)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhileLoopExpression {
    check_expression: Box<Expression>,
    loop_body: Block,
}

impl Evaluatable for WhileLoopExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::ParserError> {
        if let Some(TokenValue::Keyword(WHILE)) = tokens.peek_value() {
            tokens.next();

            let check_expression = Expression::parse(tokens)?;
            let loop_body = Block::parse(tokens)?;

            return Ok(Some(WhileLoopExpression {
                check_expression: Box::new(check_expression),
                loop_body,
            }));
        }

        return Ok(None);
    }

    fn evaluate(
        &self,
        stack: &mut crate::executor::ExecutorStack,
        context: &mut crate::executor::ExecutorContext,
    ) -> Result<Value, crate::ExecutionError> {
        loop {
            let Value::Boolean(check_result) = self.check_expression.evaluate(stack, context)?
            else {
                return Err("while loop check expression must return a boolean value".into());
            };

            if !check_result {
                return Ok(Value::Void);
            }

            self.loop_body.execute(stack, context)?;
        }
    }
}
