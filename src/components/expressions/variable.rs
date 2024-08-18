use serde::Serialize;

use crate::{
    components::{Evaluatable, Identifier, Tokens},
    executor::{builtins, Value},
    lexer::TokenValue,
};

use super::Expression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VariableExpression {
    name: Identifier,
    arguments: Option<Vec<Expression>>,
}

impl Evaluatable for VariableExpression {
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

    fn evaluate(
        &self,
        stack: &mut crate::executor::ExecutorStack,
        context: &mut crate::executor::ExecutorContext,
    ) -> Result<crate::executor::Value, crate::errors::ExecutionError> {
        if let Some(arguments) = &self.arguments {
            let arguments = arguments
                .iter()
                .map(|arg| arg.evaluate(stack, context))
                .collect::<Result<Vec<_>, _>>()?;

            // Remove function from stack when calling it to avoid double borrowing, means
            // recursion won't work, but that needs stack frames to work anyway.
            if let Some(function) = stack.functions.remove(&self.name.value) {
                function.code.execute(stack, context)?;
                stack.functions.insert(self.name.value.to_owned(), function);
                Ok(Value::Void)
            } else {
                builtins::call_builtin(&self.name.value, arguments.as_slice(), context)
            }
        } else {
            stack.resolve_variable(&self.name.value)
        }
    }
}
