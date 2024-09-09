use serde::Serialize;
use statement::Statement;

use crate::{errors::PostProcessError, lexer::Token, utils::iterators::Backtrackable, Executor};

use super::{
    errors::{ExecutionError, ParserError},
    stack::Stack,
    ControlFlowOptions, EvaluationException, PostProcessContext, Scope, ScopeType, Tokens,
};

pub use function::Function;

mod block;
mod expressions;
mod function;
mod identifier;
mod operator;
mod statement;
mod type_definition;

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

    pub fn post_process(&self, context: &mut PostProcessContext) -> Result<(), PostProcessError> {
        // Add new variable scope for the root block
        context.scopes.push(Scope::new(ScopeType::Root));

        for function in &self.functions {
            function.code.post_process_with_initializer(
                |context| {
                    for (name, value_type) in &function.arguments {
                        context.declare_variable(name.value.clone(), value_type.value.clone())
                    }

                    Ok(())
                },
                ScopeType::Function(function.return_type.value.clone()),
                context,
            )?;

            let arguments = function
                .arguments
                .iter()
                .map(|(_, type_definition)| type_definition.value.clone())
                .collect();
            context.functions.insert(
                function.name.value.clone(),
                (arguments, function.return_type.value.clone()),
            );
        }

        for statement in &self.statements {
            statement.post_process(context)?;
        }

        context.scopes.pop();

        Ok(())
    }

    pub fn execute<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> Result<u8, ExecutionError> {
        for function in &self.functions {
            stack.declare_function(&function.name.value, function.clone())?;
        }

        stack.push_scope();
        let mut exit_code = 0;
        for statement in &self.statements {
            if let Err(exception) = statement.execute(stack, executor) {
                match exception {
                    EvaluationException::ControlFlow(ControlFlowOptions::Exit(value)) => {
                        exit_code = value;
                        break;
                    }
                    EvaluationException::ControlFlow(ControlFlowOptions::Return(_)) => {
                        return Err("Return must be used in a function block".into())
                    }
                    EvaluationException::ControlFlow(ControlFlowOptions::Break()) => {
                        return Err("Break must be used in a loop block".into())
                    }
                    EvaluationException::ControlFlow(ControlFlowOptions::Continue()) => {
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
