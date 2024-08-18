use serde::Serialize;

use crate::{
    errors::{self, ExecutionError, ParserError},
    executor::{ExecutorContext, ExecutorStack, Value},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

pub mod block;
pub mod expressions;
pub mod function;
pub mod literals;
pub mod operator;
pub mod root;
pub mod statement;

trait Tokens<'a> {
    fn next_value(&mut self) -> Option<&'a TokenValue<'a>>;
    fn peek_value(&mut self) -> Option<&'a TokenValue<'a>>;
    fn backtrack_if_none<T, F: FnOnce(&mut Self) -> Option<T>>(&mut self, action: F) -> Option<T>
    where
        Self: Sized;
}

impl<'a, I: Iterator<Item = &'a Token<'a>>> Tokens<'a> for Backtrackable<I> {
    fn next_value(&mut self) -> Option<&'a TokenValue<'a>> {
        self.next().map(|x| &x.value)
    }

    fn peek_value(&mut self) -> Option<&'a TokenValue<'a>> {
        self.peek().map(|x| &x.value)
    }

    fn backtrack_if_none<T, F: FnOnce(&mut Self) -> Option<T>>(&mut self, action: F) -> Option<T>
    where
        Self: Sized,
    {
        let checkpoint = self.checkpoint();
        let result = action(self);
        if result.is_none() {
            self.backtrack(checkpoint);
        }
        return result;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identifier {
    pub value: String,
}

impl From<&str> for Identifier {
    fn from(value: &str) -> Self {
        Identifier {
            value: value.to_owned(),
        }
    }
}

trait Evaluatable
where
    Self: Sized,
{
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError>;

    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> Result<Value, ExecutionError>;
}
