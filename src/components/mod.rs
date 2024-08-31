use root::Root;
use stack::Stack;
use values::Value;

use crate::{
    errors::{self, ExecutionError, ParserError},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    Executor,
};

mod builtins;
mod root;
mod stack;
mod values;

pub struct ComponentTree {
    root: Root,
}

pub struct ExecutionOutput {
    exit_code: u8,
}

impl ExecutionOutput {
    fn new(exit_code: u8) -> Self {
        Self { exit_code }
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

pub fn parse<'a, I: IntoIterator<Item = &'a Token<'a>>>(
    tokens: I,
) -> Result<ComponentTree, ParserError> {
    let tokens = &mut Backtrackable::new(tokens.into_iter());
    let root = Root::parse(tokens).map_err(|mut err| {
        if let Some(current) = tokens.peek() {
            err.set_position(current);
        }
        return err;
    })?;
    Ok(ComponentTree { root })
}

impl ComponentTree {
    pub fn execute<E: Executor>(
        &mut self,
        executor: &mut E
,
    ) -> Result<ExecutionOutput, ExecutionError> {
        let mut stack = Stack::new();

        let exit_code = self.root.execute(&mut stack, executor).map_err(|mut err| {
            err.set_call_stack(stack.get_call_stack().clone());
            return err;
        })?;

        return Ok(ExecutionOutput::new(exit_code));
    }
}

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
    fn evaluate<E: Executor>(&self, stack: &mut Stack, executor: &mut E
,
)
        -> EvaluationResult<Value>;
}

pub type EvaluationResult<T> = Result<T, EvaluationException>;

pub enum ControlFlowOptions {
    Exit(u8),
    Return(Value),
    Break(),
    Continue(),
}

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
