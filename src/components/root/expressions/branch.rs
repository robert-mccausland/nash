use serde::Serialize;

use crate::{
    components::{
        stack::ExecutorStack, values::Value, Evaluatable, EvaluationResult, Parsable, Tokens,
    },
    constants::{ELSE, IF},
    executor::ExecutorContext,
    lexer::TokenValue,
};

use super::{Block, Expression};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BranchExpression {
    conditional_blocks: Vec<(Expression, Block)>,
    default_block: Option<Block>,
}

impl Parsable for BranchExpression {
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
}

impl Evaluatable for BranchExpression {
    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        for (condition, block) in &self.conditional_blocks {
            let condition_result = condition.evaluate(stack, context)?;
            if let Value::Boolean(result) = condition_result {
                if result {
                    return Ok(block.execute(stack, context)?.into());
                }
            } else {
                return Err("If statement condition must evaluate to a boolean".into());
            }
        }

        if let Some(default_block) = &self.default_block {
            Ok(default_block.execute(stack, context)?.into())
        } else {
            Ok(Value::Void.into())
        }
    }
}
