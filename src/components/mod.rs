use std::collections::HashMap;

use root::Root;
use stack::Stack;
use values::{Type, Value};

use crate::{
    errors::{self, ExecutionError, ParserError, PostProcessError},
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
    pub fn post_process(&self) -> Result<(), PostProcessError> {
        let mut context = PostProcessContext::new();
        self.root.post_process(&mut context)?;

        Ok(())
    }

    pub fn execute<E: Executor>(
        &self,
        executor: &mut E,
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

pub type EvaluationResult<T> = Result<T, EvaluationException>;

pub enum ControlFlowOptions {
    Exit(u8),
    Return(Value),
    Break(),
    Continue(),
}

pub enum EvaluationException {
    ControlFlow(ControlFlowOptions),
    Error(ExecutionError),
}

impl From<ControlFlowOptions> for EvaluationException {
    fn from(value: ControlFlowOptions) -> Self {
        EvaluationException::ControlFlow(value.into())
    }
}

impl<T: Into<ExecutionError>> From<T> for EvaluationException {
    fn from(value: T) -> Self {
        EvaluationException::Error(value.into())
    }
}

#[derive(Debug)]
pub struct PostProcessContext {
    functions: HashMap<String, (Vec<Type>, Type)>,
    scopes: Vec<Scope>,
}

impl PostProcessContext {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
            scopes: Vec::new(),
        }
    }

    fn declare_variable(&mut self, name: String, variable_type: Type) {
        self.scopes
            .last_mut()
            .unwrap()
            .variables
            .insert(name, variable_type);
    }

    fn find_variable(&self, name: &str) -> Option<Type> {
        for scope in &self.scopes {
            if let Some(variable_type) = scope.variables.get(name) {
                return Some(variable_type.clone());
            }
        }

        return None;
    }

    fn has_parent_scope(&self, scope_type: &ScopeType) -> bool {
        self.get_matching_parent_scope(|scope| scope_type == scope)
            .is_some()
    }

    fn get_matching_parent_scope<F: FnMut(&ScopeType) -> bool>(
        &self,
        mut predicate: F,
    ) -> Option<&Scope> {
        for scope in &self.scopes {
            if predicate(&scope.scope_type) {
                return Some(scope);
            }
        }

        None
    }
}

#[derive(Debug)]
struct Scope {
    variables: HashMap<String, Type>,
    scope_type: ScopeType,
}

impl Scope {
    pub fn new(scope_type: ScopeType) -> Self {
        Self {
            variables: HashMap::new(),
            scope_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScopeType {
    Root,
    Block,
    Function(Type),
    Looped,
    Conditional,
}
