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
            return Err("function name must be followed by (".into());
        };

        let mut arguments = Vec::new();
        if let Some(TokenValue::RightBracket()) = tokens.peek_value() {
        } else {
            loop {
                let Some(TokenValue::Identifier(argument)) = tokens.peek_value() else {
                    return Err("expected function argument".into());
                };
                tokens.next();

                arguments.push((*argument).into());

                if let Some(TokenValue::RightBracket()) = tokens.peek_value() {
                    break;
                } else {
                    let Some(TokenValue::Comma()) = tokens.peek_value() else {
                        return Err("expected ) or , after function argument".into());
                    };
                    tokens.next_value();
                }
            }
        }

        tokens.next_value();
        let code = Block::parse(tokens)?;

        return Ok(Some(Function {
            arguments,
            name: (*identifier).into(),
            code,
        }));
    }
}
