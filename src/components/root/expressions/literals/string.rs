use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    components::{
        errors::ParserError,
        root::expressions::{Expression, ExpressionComponent},
        stack::Stack,
        values::{Type, Value},
        EvaluationResult, PostProcessContext, Tokens,
    },
    errors::PostProcessError,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    Executor,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StringLiteral {
    pub parts: Vec<(String, Expression)>,
    pub end: String,
}

impl StringLiteral {
    pub fn new(parts: Vec<(String, Expression)>, end: String) -> Self {
        Self { parts, end }
    }

    fn parse_impl<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Self, ParserError> {
        let mut variables = Vec::new();
        let mut end = String::new();

        loop {
            let next = tokens.next_value();
            if let Some(TokenValue::DoubleQuote()) = next {
                break;
            } else if let Some(TokenValue::StringLiteral(value)) = next {
                end = parse_string(value)?;
            } else if let Some(TokenValue::Dollar()) = next {
                let Some(TokenValue::LeftCurly()) = tokens.next_value() else {
                    return Err("Expected { after $ in string".into());
                };
                let value = Expression::parse(tokens)?;
                let Some(TokenValue::RightCurly()) = tokens.next_value() else {
                    return Err("Expected } after template variable".into());
                };
                variables.push((end, value.into()));
                end = String::new();
            } else {
                return Err("Unable to parse string literal".into());
            }
        }

        return Ok(StringLiteral::new(variables, end));
    }

    pub fn resolve<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<String> {
        let mut result = String::new();
        for (prefix, expression) in &self.parts {
            result += &prefix;
            let Value::String(variable_value) = expression.evaluate(stack, executor)? else {
                return Err("Template variable in strings must resolve to a string".into());
            };
            result += &variable_value;
        }
        result += &self.end;
        Ok(result)
    }
}

impl From<String> for StringLiteral {
    fn from(value: String) -> Self {
        StringLiteral::new(vec![], value)
    }
}

impl From<&str> for StringLiteral {
    fn from(value: &str) -> Self {
        value.to_owned().into()
    }
}

impl ExpressionComponent for StringLiteral {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        Ok(
            if let Some(TokenValue::DoubleQuote()) = tokens.peek_value() {
                tokens.next();
                Some(Self::parse_impl(tokens)?)
            } else {
                None
            },
        )
    }
    fn evaluate<E: Executor>(
        &self,
        stack: &mut Stack,
        executor: &mut E,
    ) -> EvaluationResult<Value> {
        Ok(Value::String(self.resolve(stack, executor)?).into())
    }

    fn get_type(&self, _context: &mut PostProcessContext) -> Result<Type, PostProcessError> {
        return Ok(Type::String);
    }
}

fn parse_string(value: &str) -> Result<String, ParserError> {
    let mut result = String::new();
    let mut escape = false;
    for grapheme in value.graphemes(true) {
        if grapheme == "\\" && !escape {
            escape = true;
        } else {
            result += grapheme;
            escape = false;
        }
    }

    if escape {
        return Err(format!("Unterminated escape sequence in \"{value}\"").into());
    }

    return Ok(result);
}
