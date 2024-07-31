use crate::lexer::{Token, TokenValue};

use super::{
    expressions::{parse_expression, Expression},
    Backtrackable, Identifier, ParserError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Declaration(Identifier, Expression),
    Assignment(Identifier, Expression),
    Expression(Expression),
}

pub(super) fn parse_statement<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Result<Statement, ParserError> {
    let statement = parse_statement_content(tokens)?;
    let Some(TokenValue::Semicolon()) = tokens.next().map(|x| x.value) else {
        return Err("statement must end with ;".into());
    };
    return Ok(statement);
}

pub(super) fn parse_statement_content<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Result<Statement, ParserError> {
    let next = tokens.next().map(|x| x.value);
    // Any statement starting with var must be a declaration
    if let Some(TokenValue::Keyword("var")) = next {
        let Some(TokenValue::Identifier(identifier)) = tokens.next().map(|x| x.value) else {
            return Err("Expected identifier after var".into());
        };
        let identifier = Identifier {
            value: identifier.to_owned(),
        };

        // Maybe we should allow declarations without initializers
        if let Some(TokenValue::Equals()) = tokens.next().map(|x| x.value) {
            let expression = parse_expression(tokens)?;
            return Ok(Statement::Declaration(identifier, expression));
        };

        return Err("Expected = after identifier".into());
    }

    // Starting with a keyword followed by = means its an assignment
    if let Some(TokenValue::Identifier(identifier)) = next {
        let identifier = Identifier {
            value: identifier.to_owned(),
        };
        if let Some(TokenValue::Equals()) = tokens.next().map(|x| x.value) {
            let expression = parse_expression(tokens)?;
            return Ok(Statement::Assignment(identifier, expression));
        };
        tokens.backtrack();
    }

    // Otherwise it might be a bare expression
    tokens.backtrack();
    let expression = parse_expression(tokens)?;

    return Ok(Statement::Expression(expression));
}
