use serde::Serialize;

use crate::{
    components::{Evaluatable, EvaluationResult, Identifier, Parsable, Tokens},
    executor::{ExecutorContext, ExecutorStack, Value},
    lexer::TokenValue,
};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VariableExpression {
    name: Identifier,
    arguments: Option<Vec<Expression>>,
}

impl Parsable for VariableExpression {
    fn try_parse<'a, I: Iterator<Item = &'a crate::lexer::Token<'a>>>(
        tokens: &mut crate::utils::iterators::Backtrackable<I>,
    ) -> Result<Option<Self>, crate::errors::ParserError> {
        let token = tokens.peek_value();
        if let Some(TokenValue::Identifier(identifier)) = token {
            tokens.next();
            let name = (*identifier).into();
            let mut arguments = None;

            if matches!(tokens.peek_value(), Some(TokenValue::LeftBracket())) {
                tokens.next();
                let mut args = Vec::new();
                if let Some(TokenValue::RightBracket()) = tokens.peek_value() {
                    tokens.next();
                } else {
                    loop {
                        args.push(Expression::parse(tokens)?);
                        let next = tokens.next_value();
                        if let Some(TokenValue::RightBracket()) = next {
                            break;
                        } else if let Some(TokenValue::Comma()) = next {
                        } else {
                            return Err(
                                "Expected function argument to be followed by `,` or `)`".into()
                            );
                        }
                    }
                }

                arguments = Some(args)
            }

            return Ok(Some(VariableExpression { name, arguments }));
        }

        return Ok(None);
    }
}

impl Evaluatable for VariableExpression {
    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        Ok(if let Some(arguments) = &self.arguments {
            let mut evaluated_arguments = Vec::new();
            for argument in arguments {
                evaluated_arguments.push(argument.evaluate(stack, context)?)
            }

            stack
                .execute_function(&self.name.value, evaluated_arguments, context)?
                .into()
        } else {
            stack.resolve_variable(&self.name.value)?.into()
        })
    }
}
