use serde::Serialize;

use crate::{
    components::{
        stack::Stack, values::Value, EvaluationResult, PostProcessContext, Scope, ScopeType,
    },
    errors::PostProcessError,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    ExecutionError, Executor, ParserError,
};

use super::{statement::Statement, Tokens};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    pub statements: Vec<Statement>,
}

impl Block {
    pub(super) fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Block, ParserError> {
        let mut statements = Vec::new();

        let Some(TokenValue::LeftCurly()) = tokens.next_value() else {
            return Err("code block must start with {".into());
        };

        loop {
            if let Some(TokenValue::RightCurly()) = tokens.peek_value() {
                tokens.next();
                break;
            };

            statements.push(Statement::parse(tokens)?);
        }

        return Ok(Block { statements });
    }

    pub fn execute<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        self.execute_with_initializer(|_| Ok(()), stack, executor)
    }

    pub fn execute_with_initializer<
        E: Executor,
        F: FnOnce(&mut Stack) -> Result<(), ExecutionError>,
    >(
        &self,
        initialize: F,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        stack.push_scope();
        initialize(stack)?;
        for statement in &self.statements {
            statement.execute(stack, executor)?;
        }
        stack.pop_scope();
        return Ok(Value::Void);
    }

    pub fn post_process(&self, context: &mut PostProcessContext) -> Result<(), PostProcessError> {
        self.post_process_with_initializer(|_| Ok(()), ScopeType::Block, context)
    }

    pub fn post_process_with_initializer<
        F: FnOnce(&mut PostProcessContext) -> Result<(), PostProcessError>,
    >(
        &self,
        initialize: F,
        scope_type: ScopeType,
        context: &mut PostProcessContext,
    ) -> Result<(), PostProcessError> {
        // Add new variable scope for the block
        context.scopes.push(Scope::new(scope_type));
        initialize(context)?;
        for statement in &self.statements {
            statement.post_process(context)?;
        }
        context.scopes.pop();

        Ok(())
    }
}
