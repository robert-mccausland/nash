use serde::Serialize;

use crate::{
    components::values::Type,
    constants::MUT,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    ParserError,
};

use super::Tokens;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeDefinition {
    pub value: Type,
}

impl TypeDefinition {
    pub fn parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Self, ParserError> {
        Ok(Self::parse_impl(tokens)?.into())
    }

    fn parse_impl<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Type, ParserError> {
        let mutable = if let Some(TokenValue::Keyword(MUT)) = tokens.peek_value() {
            tokens.next();
            true
        } else {
            false
        };

        if let Some(TokenValue::LeftSquare()) = tokens.peek_value() {
            tokens.next();
            let inner_type = Self::parse_impl(tokens)?;
            let Some(TokenValue::RightSquare()) = tokens.peek_value() else {
                return Err("Expected ] after inner type in array type definition".into());
            };
            tokens.next();
            return Ok(Type::Array(inner_type.into(), mutable));
        }

        if mutable {
            return Err("Only array types can be mutable".into());
        }

        return Ok(Self::parse_base_type(tokens)?);
    }

    fn parse_base_type<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Type, ParserError> {
        let next = tokens.peek_value();

        if let Some(TokenValue::Identifier(type_name)) = next {
            tokens.next();

            return Ok(match *type_name {
                "void" => Type::Void,
                "string" => Type::String,
                "integer" => Type::Integer,
                "boolean" => Type::Boolean,
                "command" => Type::Command,
                _ => return Err(format!("{type_name} is not a valid type name").into()),
            });
        }

        if let Some(TokenValue::LeftBracket()) = next {
            tokens.next();
            let mut types = Vec::new();
            loop {
                types.push(Self::parse_impl(tokens)?);

                let next = tokens.peek_value();
                if let Some(TokenValue::RightBracket()) = next {
                    tokens.next();
                    break;
                }
                let Some(TokenValue::Comma()) = next else {
                    return Err("Expected , after type definition".into());
                };
                tokens.next();
            }

            return Ok(Type::Tuple(types));
        }

        return Err("Unable to parse type definition".into());
    }
}

impl From<Type> for TypeDefinition {
    fn from(value: Type) -> Self {
        Self { value }
    }
}
