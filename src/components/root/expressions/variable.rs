use serde::Serialize;

use crate::{
    components::{
        root::identifier::Identifier, stack::Stack, values::Value, Evaluatable, EvaluationResult,
        Parsable, Tokens,
    },
    lexer::TokenValue,
    Executor,
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

impl VariableExpression {
    pub fn evaluate_on_instance<E: Executor>(
        &self,
        instance: Option<Value>,
        stack: &mut Stack,
        executor: &mut E
,
    ) -> EvaluationResult<Value> {
        Ok(if let Some(arguments) = &self.arguments {
            let arguments = arguments
                .iter()
                .map(|x| x.evaluate(stack, executor))
                .collect::<Result<Vec<_>, _>>()?;

            stack
                .execute_function(&self.name.value, instance, arguments, executor)?
                .into()
        } else if instance.is_none() {
            stack.resolve_variable(&self.name.value)?.into()
        } else {
            return Err("Instance variables are not yet implemented".into());
        })
    }
}

impl Evaluatable for VariableExpression {
    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E
,
    ) -> EvaluationResult<Value> {
        self.evaluate_on_instance(None, stack, executor)
    }
}