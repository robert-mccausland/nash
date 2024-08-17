use unicode_segmentation::UnicodeSegmentation;

use crate::{
    constants::{FALSE, TRUE},
    executor::{ExecutorStack, Value},
    lexer::{Token, TokenValue},
};

use super::{errors::ExecutionError, Backtrackable, Identifier, ParserError, Tokens};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub parts: Vec<(String, Identifier)>,
    pub end: String,
}

impl StringLiteral {
    pub fn new(parts: Vec<(String, Identifier)>, end: String) -> Self {
        Self { parts, end }
    }

    pub fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        Ok(
            if let Some(TokenValue::DoubleQuote()) = tokens.peek_value() {
                tokens.next();
                Some(Self::parse(tokens)?)
            } else {
                None
            },
        )
    }

    fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
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

#[derive(Debug, Clone, PartialEq, Eq)]

pub struct CommandLiteral {
    pub command: StringLiteral,
    pub arguments: Vec<StringLiteral>,
}

impl CommandLiteral {
    pub fn new(command: StringLiteral, arguments: Vec<StringLiteral>) -> Self {
        Self { command, arguments }
    }

    pub fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        Ok(if let Some(TokenValue::Backtick()) = tokens.peek_value() {
            tokens.next();
            Some(Self::parse_impl(tokens)?)
        } else {
            None
        })
    }

    fn parse_impl<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Self, ParserError> {
        let command =
            Self::parse_next_literal(tokens)?.ok_or("Command literal must contain command")?;
        let mut arguments = Vec::new();
        while let Some(next) = Self::parse_next_literal(tokens)? {
            arguments.push(next);
        }

        return Ok(CommandLiteral::new(command, arguments));
    }

    fn parse_next_literal<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<StringLiteral>, ParserError> {
        let next = tokens.next_value();
        if let Some(TokenValue::DoubleQuote()) = next {
            Ok(Some(StringLiteral::parse(tokens)?))
        } else if let Some(TokenValue::StringLiteral(value)) = next {
            Ok(Some((*value).into()))
        } else if let Some(TokenValue::Backtick()) = next {
            Ok(None)
        } else {
            Err("Unable to parse command".into())
        }
    }
}

impl<'a, I: IntoIterator<Item = &'a str>> From<I> for CommandLiteral {
    fn from(value: I) -> Self {
        let mut iter = value.into_iter();
        CommandLiteral::new(
            iter.next().unwrap_or_default().into(),
            iter.map(|x| x.into()).collect::<Vec<_>>(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerLiteral {
    pub value: i32,
}

impl IntegerLiteral {
    pub fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        Ok(
            if let Some(TokenValue::IntegerLiteral(value)) = tokens.peek_value() {
                tokens.next();
                Some(Self::parse_impl(value)?)
            } else {
                None
            },
        )
    }

    fn parse_impl(value: &str) -> Result<IntegerLiteral, ParserError> {
        let Ok(value) = i32::from_str_radix(value, 10) else {
            return Err(format!("Unable to parse {value} as a number").into());
        };
        return Ok(value.into());
    }
}

impl From<i32> for IntegerLiteral {
    fn from(value: i32) -> Self {
        IntegerLiteral { value }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BooleanLiteral {
    pub value: bool,
}

impl BooleanLiteral {
    pub fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Option<Self> {
        if let Some(TokenValue::Keyword(TRUE)) = tokens.peek_value() {
            tokens.next();
            return Some(true.into());
        } else if let Some(TokenValue::Keyword(FALSE)) = tokens.peek_value() {
            tokens.next();
            return Some(false.into());
        } else {
            None
        }
    }
}

impl From<bool> for BooleanLiteral {
    fn from(value: bool) -> Self {
        BooleanLiteral { value }
    }
}
