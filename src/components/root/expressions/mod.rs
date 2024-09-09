use crate::{
    components::{
        stack::Stack,
        values::{Type, Value},
        EvaluationResult, PostProcessContext,
    },
    errors::PostProcessError,
    lexer::Token,
    utils::iterators::Backtrackable,
    Executor, ParserError,
};

mod accessor;
mod block;
mod brackets;
mod branch;
mod collections;
mod index;
mod literals;
mod loops;
mod pipeline;
mod variable;

use super::{block::Block, operator::Operator};

use accessor::AccessorExpression;
use block::BlockExpression;
use brackets::BracketExpression;
use branch::BranchExpression;
use collections::{ArrayExpression, TupleExpression};
use index::IndexExpression;
use literals::{BooleanLiteral, CommandLiteral, IntegerLiteral, StringLiteral};
use loops::{ForLoopExpression, WhileLoopExpression};
use pipeline::PipelineExpression;
use serde::Serialize;
use variable::VariableExpression;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Expression {
    pub operations: Vec<(Operator, BaseExpression)>,
    pub first: BaseExpression,
}

impl Expression {
    pub fn new(first: BaseExpression, operations: Vec<(Operator, BaseExpression)>) -> Self {
        Self {
            first: first.into(),
            operations,
        }
    }

    pub(super) fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Expression, ParserError> {
        let expression = BaseExpression::parse(tokens)?;
        let mut operations = Vec::new();
        while let Some(operator) = Operator::try_parse(tokens)? {
            operations.push((operator, BaseExpression::parse(tokens)?));
        }

        return Ok(Expression::new(expression, operations));
    }

    pub fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        let mut left = self.first.get_type(context)?;
        for (operator, right) in &self.operations {
            let right = right.get_type(context)?;
            left = operator.get_type(left, right)?;
        }

        return Ok(left);
    }

    pub fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        let mut result = self.first.evaluate(stack, executor)?;
        let mut previous: Option<&Operator> = None;

        for (operator, expression) in &self.operations {
            let right = expression.evaluate(stack, executor)?;
            if let Some(previous) = previous {
                if !previous.chains_with(operator) {
                    return Err(format!(
                        "Chaining {previous:?} with {operator:?} is not supported."
                    ))?;
                }
            }

            result = operator.execute(result, right)?;
            previous = Some(operator);
        }
        return Ok(result.into());
    }
}

macro_rules! expression_content {
    ([$($expression_type:ident,)*], [$($dependent_expression:ident,)*]) => {

        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        pub enum BaseExpression {
            $(
                $expression_type($expression_type),
            )*
            $(
                $dependent_expression($dependent_expression),
            )*
        }

        impl BaseExpression {
            fn parse_inner<'a, I: Iterator<Item = &'a Token<'a>>>(
                tokens: &mut Backtrackable<I>,
            ) -> Result<Option<Self>, ParserError> {
                $(
                    if let Some(value) = $expression_type::try_parse(tokens)? {
                        return Ok(Some(Self::$expression_type(value)));
                    };
                )*
                return Ok(None);
            }

            fn parse_dependent<'a, I: Iterator<Item = &'a Token<'a>>>(
                mut inner: Self,
                tokens: &mut Backtrackable<I>,
            ) -> Result<Result<Self, Self>, ParserError> {
                $(
                    match $dependent_expression::try_parse(inner, tokens)? {
                        Ok(result) => return Ok(Ok(Self::$dependent_expression(result))),
                        Err(recovered_inner) => {
                            inner = recovered_inner
                        }
                    };
                )*

                return Ok(Err(inner))
            }

            fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
                match self {
                    $(
                        Self::$expression_type(value) => value.get_type(context),
                    )*
                    $(
                        Self::$dependent_expression(value) => value.get_type(context),
                    )*
                }
            }

            fn evaluate<E: Executor>(
                &self,
                stack: &mut Stack,
                executor: &mut E,
            ) -> EvaluationResult<Value> {
                match self {
                    $(
                        Self::$expression_type(value) => value.evaluate(stack, executor),
                    )*
                    $(
                        Self::$dependent_expression(value) => value.evaluate(stack, executor),
                    )*
                }
            }
        }
    };
}

expression_content!(
    [
        StringLiteral,
        BooleanLiteral,
        IntegerLiteral,
        CommandLiteral,
        ArrayExpression,
        // Note: brackets must be matched before tuples, as the tuple matcher will also match expressions
        // that should be bracket expressions.
        BracketExpression,
        TupleExpression,
        VariableExpression,
        PipelineExpression,
        WhileLoopExpression,
        ForLoopExpression,
        BranchExpression,
        BlockExpression,
    ],
    // These expressions are special as they all start with a BaseExpression, to avoid parsing the expressions multiple times
    // and to allow them to be nested within themselves, we have a special flow for them where they are matched after all
    // the other types.
    [AccessorExpression, IndexExpression,]
);

impl BaseExpression {
    fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Self, ParserError> {
        let Some(mut value) = Self::parse_inner(tokens)? else {
            return Err("Could not parse a valid expression".into());
        };

        loop {
            match Self::parse_dependent(value, tokens)? {
                Ok(dependent_value) => value = dependent_value,
                Err(value) => return Ok(value),
            }
        }
    }
}

trait ExpressionComponent {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError>
    where
        Self: Sized;
    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError>;
    fn evaluate<E: Executor>(&self, stack: &mut Stack, executor: &mut E)
        -> EvaluationResult<Value>;
}

trait DependentExpressionComponent {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        inner: BaseExpression,
        tokens: &mut Backtrackable<I>,
    ) -> Result<Result<Self, BaseExpression>, ParserError>
    where
        Self: Sized;
    fn get_type(&self, context: &mut PostProcessContext) -> Result<Type, PostProcessError>;
    fn evaluate<E: Executor>(&self, stack: &mut Stack, executor: &mut E)
        -> EvaluationResult<Value>;
}
