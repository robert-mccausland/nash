use serde::Serialize;

use crate::{
    components::{
        root::identifier::Identifier, stack::ExecutorStack, values::Value, ControlFlowOptions,
        Evaluatable, EvaluationException, EvaluationResult, Parsable, Tokens,
    },
    constants::{FOR, IN, WHILE},
    executor::ExecutorContext,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    ParserError,
};

use super::{Block, Expression};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForLoopExpression {
    item_name: Identifier,
    array_expression: Box<Expression>,
    loop_body: Block,
}

impl Parsable for ForLoopExpression {
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
}

impl Evaluatable for ForLoopExpression {
    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        let Value::Array(array, _) = self.array_expression.evaluate(stack, context)? else {
            return Err("for ... in loop must be used on an array value".into());
        };

        for item in array.as_ref().borrow().iter() {
            let result = self.loop_body.execute_with_initializer(
                |stack| stack.declare_and_assign_variable(&self.item_name.value, item.clone()),
                stack,
                context,
            );

            if let Err(EvaluationException::AlterControlFlow(ControlFlowOptions::Break())) = result
            {
                break;
            }

            if let Err(EvaluationException::AlterControlFlow(ControlFlowOptions::Continue())) =
                result
            {
                continue;
            }

            result?;
        }

        Ok(Value::Void.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhileLoopExpression {
    check_expression: Box<Expression>,
    loop_body: Block,
}

impl Parsable for WhileLoopExpression {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
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
}

impl Evaluatable for WhileLoopExpression {
    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        loop {
            let Value::Boolean(check_result) = self.check_expression.evaluate(stack, context)?
            else {
                return Err("while loop check expression must return a boolean value".into());
            };

            if !check_result {
                return Ok(Value::Void.into());
            }

            let result = self.loop_body.execute(stack, context);
            if let Err(EvaluationException::AlterControlFlow(ControlFlowOptions::Break())) = result
            {
                return Ok(Value::Void.into());
            }

            if let Err(EvaluationException::AlterControlFlow(ControlFlowOptions::Continue())) =
                result
            {
                continue;
            }

            result?;
        }
    }
}
