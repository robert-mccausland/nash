use serde::Serialize;

use crate::{
    components::{Parsable, Tokens},
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    ParserError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identifier {
    pub value: String,
}

impl From<&str> for Identifier {
    fn from(value: &str) -> Self {
        Identifier {
            value: value.to_owned(),
        }
    }
}

impl Parsable for Identifier {
    fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Self>, ParserError> {
        Ok(
            if let Some(TokenValue::Identifier(value)) = tokens.peek_value() {
                tokens.next();
                Some((*value).into())
            } else {
                None
            },
        )
    }
}
