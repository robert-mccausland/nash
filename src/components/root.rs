use serde::Serialize;

use crate::{
    executor::{ExecutorContext, ExecutorStack},
    lexer::Token,
    utils::iterators::Backtrackable,
};

use super::{
    errors::{ExecutionError, ParserError},
    function::Function,
    statement::Statement,
    Tokens,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Root {
    pub statements: Vec<Statement>,
    pub functions: Vec<Function>,
}

impl Root {
    pub fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Self, ParserError> {
        let mut statements = Vec::new();
        let mut functions = Vec::new();

        loop {
            let token = tokens.peek_value();
            if token.is_none() {
                break;
            }

            if let Some(function) = Function::try_parse(tokens)? {
                functions.push(function);
            } else {
                statements.push(Statement::parse(tokens)?);
            }
        }

        return Ok(Self {
            statements,
            functions,
        });
    }

    pub fn execute(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> Result<(), ExecutionError> {
        for function in &self.functions {
            stack
                .functions
                .insert(function.name.value.to_owned(), function.clone());
        }
        for statement in &self.statements {
            statement.execute(stack, context)?;
        }
        return Ok(());
    }
}
