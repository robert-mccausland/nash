use crate::{
    executor::{ExecutorContext, ExecutorStack, Value},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

mod block;
mod branch;
mod collections;
mod execute;
mod loops;
mod variable;

use super::{
    block::Block,
    errors::ParserError,
    literals::{BooleanLiteral, CommandLiteral, IntegerLiteral, StringLiteral},
    operator::Operator,
    EvaluationResult, Tokens,
};

use block::BlockExpression;
use branch::BranchExpression;
use collections::{ArrayExpression, TupleExpression};
use execute::ExecuteExpression;
use loops::{ForLoopExpression, WhileLoopExpression};
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
        while let Some(operator) = Operator::try_parse(tokens) {
            operations.push((operator, BaseExpression::parse(tokens)?));
        }

        return Ok(Expression::new(expression, operations));
    }

    pub fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        let mut result = self.first.evaluate(stack, context)?;

        for (operator, expression) in &self.operations {
            let right = expression.evaluate(stack, context)?;
            result = operator.execute(result, right)?;
        }
        return Ok(result.into());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Accessor {
    Integer(u32),
    Variable(VariableExpression),
}

impl Accessor {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        let Some(TokenValue::Dot()) = tokens.peek_value() else {
            return Ok(None);
        };
        tokens.next();

        if let Some(TokenValue::IntegerLiteral(integer)) = tokens.peek_value() {
            tokens.next();
            let value = u32::from_str_radix(integer, 10).map_err::<ParserError, _>(|_| {
                "Numeric accessors must be positive integers {err}".into()
            })?;
            return Ok(Some(Accessor::Integer(value)));
        }

        if let Some(value) = VariableExpression::try_parse(tokens)? {
            return Ok(Some(Accessor::Variable(value)));
        }

        return Err("Unable to parse accessor".into());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaseExpression {
    content: Box<ExpressionContent>,
    accessors: Vec<Accessor>,
}

impl BaseExpression {
    fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Self, ParserError> {
        let content = ExpressionContent::parse(tokens)?.into();
        let mut accessors = Vec::new();
        while let Some(accessor) = Accessor::try_parse(tokens)? {
            accessors.push(accessor);
        }

        return Ok(BaseExpression { content, accessors });
    }

    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        context: &mut ExecutorContext,
    ) -> EvaluationResult<Value> {
        let mut value = self.content.evaluate(stack, context)?;
        for accessor in &self.accessors {
            match accessor {
                Accessor::Integer(integer) => {
                    let Value::Tuple(mut values) = value else {
                        return Err("Cannot use get expression on non-tuple value".into());
                    };

                    let len = values.len();
                    let result = values.get_mut(*integer as usize).ok_or(format!(
                        "Cannot get element at index {:} because tuple only has {:} elements",
                        integer, len
                    ))?;

                    value = core::mem::take(result).into();
                }
                Accessor::Variable(variable) => {
                    value = variable.evaluate_on_instance(Some(value), stack, context)?;
                }
            }
        }

        return Ok(value);
    }
}

macro_rules! expression_content {
    ([$($expression_type:ident,)*]) => {
        use crate::components::Evaluatable;
        use crate::components::Parsable;

        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        enum ExpressionContent {
            $(
                $expression_type($expression_type),
            )*
        }

        impl ExpressionContent {
            fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
                tokens: &mut Backtrackable<I>,
            ) -> Result<Self, ParserError> {
                $(
                    if let Some(value) = $expression_type::try_parse(tokens)? {
                        return Ok(ExpressionContent::$expression_type(value));
                    }
                )*

                return Err("Unable to parse expression".into());
            }

            fn evaluate(
                &self,
                stack: &mut ExecutorStack,
                context: &mut ExecutorContext,
            ) -> EvaluationResult<Value> {
                match self {
                    $(
                        Self::$expression_type(value) => value.evaluate(stack, context),
                    )*
                }
            }
        }
    };
}

expression_content!([
    StringLiteral,
    BooleanLiteral,
    IntegerLiteral,
    CommandLiteral,
    ArrayExpression,
    TupleExpression,
    VariableExpression,
    ExecuteExpression,
    WhileLoopExpression,
    ForLoopExpression,
    BranchExpression,
    BlockExpression,
]);
