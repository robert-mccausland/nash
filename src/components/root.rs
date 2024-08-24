use serde::Serialize;

use crate::{
    executor::{ExecutorContext, ExecutorStack},
    lexer::Token,
    utils::iterators::Backtrackable,
};

use super::{
    errors::{ExecutionError, ParserError},
    function::Function,
    statement::{ControlFlowOptions, Statement},
    EvaluationException, Tokens,
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
    ) -> Result<u8, ExecutionError> {
        for function in &self.functions {
            stack.declare_function(&function.name.value, function.clone())?;
        }

        stack.push_scope();
        let mut exit_code = 0;
        for statement in &self.statements {
            if let Err(exception) = statement.execute(stack, context) {
                match exception {
                    EvaluationException::AlterControlFlow(ControlFlowOptions::Exit(value)) => {
                        exit_code = value;
                        break;
                    }
                    EvaluationException::AlterControlFlow(ControlFlowOptions::Return(_)) => {
                        return Err("Return must be used in a function block".into())
                    }
                    EvaluationException::AlterControlFlow(ControlFlowOptions::Break()) => {
                        return Err("Break must be used in a loop block".into())
                    }
                    EvaluationException::AlterControlFlow(ControlFlowOptions::Continue()) => {
                        return Err("Continue must be used in a loop block".into())
                    }
                    EvaluationException::Error(err) => return Err(err),
                };
            }
        }
        stack.pop_scope();

        return Ok(exit_code);
    }
}
