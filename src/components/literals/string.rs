use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    components::{
        errors::{ExecutionError, ParserError},
        Evaluatable, Identifier, Tokens,
    },
    executor::{ExecutorContext, ExecutorStack, Value},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StringLiteral {
    pub parts: Vec<(String, Identifier)>,
    pub end: String,
}

impl StringLiteral {
    pub fn new(parts: Vec<(String, Identifier)>, end: String) -> Self {
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
                let Some(TokenValue::Identifier(value)) = tokens.next_value() else {
                    return Err("Expected identifier after ${ in string".into());
                };
                let Some(TokenValue::RightCurly()) = tokens.next_value() else {
                    return Err("Expected } after template variable".into());
                };
                variables.push((end, (*value).into()));
                end = String::new();
            } else {
                return Err("Unable to parse string literal".into());
            }
        }

        return Ok(StringLiteral::new(variables, end));
    }

    pub fn resolve(&self, stack: &mut ExecutorStack) -> Result<String, ExecutionError> {
        let mut result = String::new();
        for (prefix, identifier) in &self.parts {
            result += &prefix;
            let Value::String(variable_value) = stack.resolve_variable(&identifier.value)? else {
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

impl Evaluatable for StringLiteral {
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

    fn evaluate(
        &self,
        stack: &mut ExecutorStack,
        _context: &mut ExecutorContext,
    ) -> Result<Value, ExecutionError> {
        Ok(Value::String(self.resolve(stack)?))
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
