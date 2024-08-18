use std::{error::Error, fmt::Display};

use serde::Serialize;

use crate::lexer::Token;

macro_rules! impl_error {
    ($error:ident) => {
        impl Error for $error {
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

        impl From<&str> for $error {
            fn from(value: &str) -> Self {
                value.to_owned().into()
            }
        }

        impl From<String> for $error {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

macro_rules! nash_error {
    ($($error:ident,)*) => {
        #[derive(Debug, PartialEq, Eq, Serialize)]
        pub enum NashError {
            $(
                $error($error),
            )*
            Other(String)
        }

        impl NashError {
            fn new(value: String) -> Self {
                Self::Other(value)
            }
        }

        impl Display for NashError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$error(value) => value.fmt(f),
                    )*
                    Self::Other(value) => value.fmt(f),
                }
            }
        }

        $(
            impl From<$error> for NashError {
                fn from(value: $error) -> Self {
                    Self::$error(value)
                }
            }
        )*

        impl_error!(NashError);
    };
}

nash_error![LexerError, ParserError, ExecutionError,];

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct LexerError {
    pub message: String,
    pub position: Option<usize>,
}

impl LexerError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            position: None,
        }
    }
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl_error!(LexerError);

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ParserError {
    pub message: String,
    pub token: String,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

impl ParserError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            token: String::new(),
            start: None,
            end: None,
        }
    }

    pub fn set_position<'a>(&mut self, token: &'a Token<'a>) {
        self.token = format!("{:?}", token.value);
        self.start = Some(token.start);
        self.end = Some(token.end);
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl_error!(ParserError);

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ExecutionError {
    message: String,
}

impl ExecutionError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl_error!(ExecutionError);
