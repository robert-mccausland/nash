use serde::Serialize;

use crate::{
    constants::FUNC,
    executor::Type,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
};

use super::{
    block::Block, errors::ParserError, type_definition::TypeDefinition, Identifier, Tokens,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Function {
    pub name: Identifier,
    pub arguments: Vec<(Identifier, TypeDefinition)>,
    pub code: Block,
    pub return_type: TypeDefinition,
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

                let Some(TokenValue::Colon()) = tokens.peek_value() else {
                    return Err("expected : after argument number".into());
                };
                tokens.next();

                let argument_type = TypeDefinition::parse(tokens)?;
                arguments.push(((*argument).into(), argument_type));

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

        let return_type = if let Some(TokenValue::Colon()) = tokens.peek_value() {
            tokens.next();
            TypeDefinition::parse(tokens)?
        } else {
            Type::Void.into()
        };

        let code = Block::parse(tokens)?;

        return Ok(Some(Function {
            arguments,
            name: (*identifier).into(),
            code,
            return_type,
        }));
    }
}
