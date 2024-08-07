use crate::{
    constants::FUNC,
    lexer::{Token, TokenValue},
};

use super::{
    functions::{parse_function, Function},
    statements::{parse_statement, Statement},
    Backtrackable, ParserError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub statements: Vec<Statement>,
    pub functions: Vec<Function>,
}

pub(super) fn parse_code_block<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
    root: bool,
) -> Result<CodeBlock, ParserError> {
    let mut statements = Vec::new();
    let mut functions = Vec::new();

    if !root {
        let Some(TokenValue::LeftCurly()) = tokens.next().map(|x| x.value) else {
            return Err("code block must start with {".into());
        };
    }

    loop {
        let token = tokens.next().map(|x| x.value);
        if !root {
            if let Some(TokenValue::RightCurly()) = token {
                break;
            };
        }
        if token.is_none() {
            if root {
                break;
            } else {
                return Err("unexpected end of file".into());
            }
        }

        if let Some(TokenValue::Keyword(FUNC)) = token {
            functions.push(parse_function(tokens)?);
        } else {
            tokens.backtrack();
            statements.push(parse_statement(tokens)?);
        }
    }

    return Ok(CodeBlock {
        statements,
        functions,
    });
}
