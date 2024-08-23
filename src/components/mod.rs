use serde::Serialize;
use statement::ControlFlowOptions;

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
pub mod type_definition;

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

trait Parsable
where
    Self: Sized,
{
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError>;
}

trait Evaluatable
where
    Self: Parsable,
{
    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value>;
}

type EvaluationResult<T> = Result<T, EvaluationException>;

pub enum EvaluationException {
    AlterControlFlow(ControlFlowOptions),
    Error(ExecutionError),
}

impl From<ControlFlowOptions> for EvaluationException {
    fn from(value: ControlFlowOptions) -> Self {
        EvaluationException::AlterControlFlow(value.into())
    }
}

impl<T: Into<ExecutionError>> From<T> for EvaluationException {
    fn from(value: T) -> Self {
        EvaluationException::Error(value.into())
    }
}
