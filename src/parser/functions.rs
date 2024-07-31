use crate::lexer::{Token, TokenValue};

use super::{
    code_blocks::{parse_code_block, CodeBlock},
    Backtrackable, Identifier, ParserError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: Identifier,
    pub arguments: Vec<Identifier>,
    pub code: CodeBlock,
}

pub(super) fn parse_function<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Result<Function, ParserError> {
    let Some(TokenValue::Identifier(identifier)) = tokens.next().map(|x| x.value) else {
        return Err("func must be followed by an identifier".into());
    };
    let name = Identifier {
        value: identifier.to_owned(),
    };

    let Some(TokenValue::LeftBracket()) = tokens.next().map(|x| x.value) else {
        return Err("arguments must be followed by )".into());
    };

    // TODO implement function arguments

    let Some(TokenValue::RightBracket()) = tokens.next().map(|x| x.value) else {
        return Err("arguments must be followed by )".into());
    };
    let code = parse_code_block(tokens, false)?;

    return Ok(Function {
        arguments: Vec::new(),
        name,
        code,
    });
}
