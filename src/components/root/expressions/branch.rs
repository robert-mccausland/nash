use serde::Serialize;

use crate::{
    components::{
        stack::Stack,
        values::{Type, Value},
        EvaluationResult, PostProcessContext, ScopeType, Tokens,
    },
    constants::{ELSE, IF},
    errors::PostProcessError,
    lexer::TokenValue,
    Executor,
};

use super::{Block, Expression, ExpressionComponent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BranchExpression {
    conditional_blocks: Vec<(Expression, Block)>,
    default_block: Option<Block>,
}

impl ExpressionComponent for BranchExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        if let Some(TokenValue::Keyword(IF)) = tokens.peek_value() {
            tokens.next();
            let mut conditional_blocks = Vec::new();
            let mut default_block: Option<Block> = None;
            loop {
                let expression = Expression::parse(tokens)?;
                let block = Block::parse(tokens)?;
                conditional_blocks.push((expression, block));

                let Some(TokenValue::Keyword(ELSE)) = tokens.peek_value() else {
                    break;
                };

                tokens.next();
                if let Some(TokenValue::Keyword(IF)) = tokens.peek_value() {
                    tokens.next();
                } else {
                    default_block = Some(Block::parse(tokens)?);
                    break;
                }
            }

            return Ok(Some(Self {
                conditional_blocks,
                default_block,
            }));
        }

        return Ok(None);
    }

    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        for (condition, block) in &self.conditional_blocks {
            let condition_result = condition.evaluate(stack, executor)?;
            if let Value::Boolean(result) = condition_result {
                if result {
                    return Ok(block.execute(stack, executor)?.into());
                }
            } else {
                return Err("If statement condition must evaluate to a boolean".into());
            }
        }

        if let Some(default_block) = &self.default_block {
            Ok(default_block.execute(stack, executor)?.into())
        } else {
            Ok(Value::Void.into())
        }
    }

    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        for (condition, block) in &self.conditional_blocks {
            let Type::Boolean = condition.get_type(context)? else {
                return Err("This expression must return a boolean".into());
            };

            block.post_process_with_initializer(|_| Ok(()), ScopeType::Conditional, context)?;
        }

        if let Some(default_block) = &self.default_block {
            default_block.post_process_with_initializer(
                |_| Ok(()),
                ScopeType::Conditional,
                context,
            )?;
        }

        Ok(Type::Void)
    }
}
