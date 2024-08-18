use serde::Serialize;

use crate::{
    constants::FUNC,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

use super::{block::Block, errors::ParserError, Identifier, Tokens};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Function {
    pub name: Identifier,
    pub arguments: Vec<Identifier>,
    pub code: Block,
}

impl Function {
    pub(super) fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Function>, ParserError> {
        let Some(TokenValue::Keyword(FUNC)) = tokens.peek_value() else {
            return Ok(None);
        };

        tokens.next();
        let Some(TokenValue::Identifier(identifier)) = tokens.next_value() else {
            return Err("func must be followed by an identifier".into());
        };

        let Some(TokenValue::LeftBracket()) = tokens.next_value() else {
            return Err("arguments must be followed by )".into());
        };

        // TODO implement function arguments

        let Some(TokenValue::RightBracket()) = tokens.next_value() else {
            return Err("arguments must be followed by )".into());
        };
        let code = Block::parse(tokens)?;

        return Ok(Some(Function {
            arguments: Vec::new(),
            name: (*identifier).into(),
            code,
        }));
    }
}
