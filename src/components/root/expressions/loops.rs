use serde::Serialize;

use crate::{
    components::{
        root::identifier::Identifier,
        stack::Stack,
        values::{Type, Value},
        ControlFlowOptions, EvaluationException, EvaluationResult, PostProcessContext, ScopeType,
        Tokens,
    },
    constants::{FOR, IN, WHILE},
    errors::PostProcessError,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    Executor, ParserError,
};

use super::{Block, Expression, ExpressionComponent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForLoopExpression {
    item_name: Identifier,
    array_expression: Box<Expression>,
    loop_body: Block,
}

impl ExpressionComponent for ForLoopExpression {
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

    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        let Value::Array(array, _, _) = self.array_expression.evaluate(stack, executor)? else {
            return Err("for ... in loop must be used on an array value".into());
        };

        for item in array.as_ref().borrow().iter() {
            let result = self.loop_body.execute_with_initializer(
                |stack| stack.declare_variable_init(&self.item_name.value, item.clone(), false),
                stack,
                executor,
            );

            if let Err(EvaluationException::ControlFlow(ControlFlowOptions::Break())) = result {
                break;
            }

            if let Err(EvaluationException::ControlFlow(ControlFlowOptions::Continue())) = result {
                continue;
            }

            result?;
        }

        Ok(Value::Void.into())
    }

    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        let Type::Array(inner_type, _) = self.array_expression.get_type(context)? else {
            return Err("For expression must evaluate to array".into());
        };

        self.loop_body.post_process_with_initializer(
            |context| {
                context.declare_variable(self.item_name.value.clone(), *inner_type);
                Ok(())
            },
            ScopeType::Looped,
            context,
        )?;

        Ok(Type::Void)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhileLoopExpression {
    check_expression: Box<Expression>,
    loop_body: Block,
}

impl ExpressionComponent for WhileLoopExpression {
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

    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        loop {
            let Value::Boolean(check_result) = self.check_expression.evaluate(stack, executor)?
            else {
                return Err("while loop check expression must return a boolean value".into());
            };

            if !check_result {
                return Ok(Value::Void.into());
            }

            let result = self.loop_body.execute(stack, executor);
            if let Err(EvaluationException::ControlFlow(ControlFlowOptions::Break())) = result {
                return Ok(Value::Void.into());
            }

            if let Err(EvaluationException::ControlFlow(ControlFlowOptions::Continue())) = result {
                continue;
            }

            result?;
        }
    }

    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        let Type::Boolean = self.check_expression.get_type(context)? else {
            return Err("This expression must resolve to a boolean".into());
        };

        self.loop_body
            .post_process_with_initializer(|_| Ok(()), ScopeType::Looped, context)?;

        Ok(Type::Void)
    }
}
