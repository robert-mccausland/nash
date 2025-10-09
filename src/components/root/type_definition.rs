use std::collections::HashMap;

use serde::Serialize;

use crate::{
    components::values::{ObjectFieldType, Type},
    constants::MUT,
    lexer::{Token, TokenValue},
    utils::iterators::Backtrackable,
    ParserError,
};

use super::{identifier::Identifier, Tokens};

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

        if let Some(TokenValue::LeftCurly()) = tokens.peek_value() {
            tokens.next();
            let mut fields = HashMap::new();
            loop {
                if let Some(TokenValue::RightCurly()) = tokens.peek_value() {
                    tokens.next();
                    break;
                }

                let field_name = Identifier::try_parse(tokens)?
                    .ok_or::<ParserError>("Expected identifier in object type definition".into())?;
                let Some(TokenValue::Colon()) = tokens.peek_value() else {
                    return Err("Expected : after field name in object type definition".into());
                };
                tokens.next_value();

                let mutable = if let Some(TokenValue::Keyword(MUT)) = tokens.peek_value() {
                    tokens.next_value();
                    true
                } else {
                    false
                };

                let inner_type = Self::parse_impl(tokens)?;
                if let Some(_) = fields.insert(
                    field_name.value.clone(),
                    ObjectFieldType {
                        mutable,
                        value: inner_type,
                    },
                ) {
                    return Err(
                        format!("Found duplicate key '{field_name:?}' in type definition").into(),
                    );
                }

                let Some(TokenValue::Comma()) = tokens.peek_value() else {
                    if let Some(TokenValue::RightCurly()) = tokens.peek_value() {
                        tokens.next();
                        break;
                    }

                    return Err("Expected , or } after field value".into());
                };
            }

            return Ok(Type::Object(fields));
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
