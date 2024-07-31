use unicode_segmentation::UnicodeSegmentation;

use crate::lexer::{Token, TokenValue};

use super::{Backtrackable, Identifier, ParserError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub parts: Vec<(String, Identifier)>,
    pub end: String,
}

impl StringLiteral {
    pub fn new(parts: Vec<(String, Identifier)>, end: String) -> Self {
        Self { parts, end }
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

pub(super) fn parse_string_literal<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Result<StringLiteral, ParserError> {
    let mut variables = Vec::new();
    let mut end = String::new();

    loop {
        let next = tokens.next().map(|x| x.value);
        if let Some(TokenValue::DoubleQuote()) = next {
            break;
        } else if let Some(TokenValue::StringLiteral(value)) = next {
            end = parse_string(value)?;
        } else if let Some(TokenValue::Dollar()) = next {
            let Some(TokenValue::LeftCurly()) = tokens.next().map(|x| x.value) else {
                return Err("Expected { after $ in string".into());
            };
            let Some(TokenValue::Identifier(value)) = tokens.next().map(|x| x.value) else {
                return Err("Expected identifier after ${ in string".into());
            };
            let Some(TokenValue::RightCurly()) = tokens.next().map(|x| x.value) else {
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

pub(super) fn parse_integer(value: &str) -> Result<i32, ParserError> {
    i32::from_str_radix(value, 10)
        .map_err(|_| format!("Unable to parse {value} as a number").into())
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
