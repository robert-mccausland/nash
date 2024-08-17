use std::{error::Error, fmt::Display};

use crate::lexer::Token;

#[derive(Debug, PartialEq, Eq)]
pub struct ParserError {
    pub message: String,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

impl ParserError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            start: None,
            end: None,
        }
    }

    pub fn set_position<'a>(&mut self, token: &'a Token<'a>) {
        self.start = Some(token.start);
        self.end = Some(token.end);
    }
}

impl From<&str> for ParserError {
    fn from(value: &str) -> Self {
        ParserError::new(value.to_owned())
    }
}

impl From<String> for ParserError {
    fn from(value: String) -> Self {
        ParserError::new(value)
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ParserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionError {
    message: String,
}

impl ExecutionError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for ExecutionError {
    fn from(value: &str) -> Self {
        ExecutionError::new(value.to_owned())
    }
}

impl From<String> for ExecutionError {
    fn from(value: String) -> Self {
        ExecutionError::new(value)
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}
