use crate::lexer::{Token, TokenValue};

use super::{
    expressions::{parse_expression, Expression},
    Backtrackable, Identifier, ParserError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Declaration(Assignment, Expression),
    Assignment(Assignment, Expression),
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assignment {
    Simple(Identifier),
    Tuple(Vec<Identifier>),
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
        let assignment = try_parse_assignment(tokens)
            .ok_or::<ParserError>("var must be followed by an assignment".into())?;
        return Ok(Statement::Declaration(
            assignment,
            parse_expression(tokens)?,
        ));
    }
    tokens.backtrack();

    if let Some(assignment) = try_parse_assignment(tokens) {
        return Ok(Statement::Assignment(assignment, parse_expression(tokens)?));
    }

    // Otherwise it might be a bare expression
    let expression = parse_expression(tokens)?;
    return Ok(Statement::Expression(expression));
}

fn try_parse_assignment<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Option<Assignment> {
    let next = tokens.next().map(|x| x.value);

    if let Some(TokenValue::Identifier(identifier)) = next {
        return if let Some(TokenValue::Equals()) = tokens.next().map(|x| x.value) {
            Some(Assignment::Simple(identifier.into()))
        } else {
            tokens.backtrack_n(2);
            None
        };
    };

    if let Some(TokenValue::LeftBracket()) = next {
        let mut identifiers = Vec::new();
        let mut next = tokens.next().map(|x| x.value);
        if let Some(TokenValue::RightBracket()) = next {
        } else {
            let mut n = 0;
            loop {
                let Some(TokenValue::Identifier(identifier)) = next else {
                    tokens.backtrack_n(n);
                    return None;
                };
                identifiers.push(identifier.into());
                next = tokens.next().map(|x| x.value);
                n += 1;
                let Some(TokenValue::Comma()) = next else {
                    if let Some(TokenValue::RightBracket()) = next {
                        break;
                    }
                    tokens.backtrack_n(n);
                    return None;
                };
                next = tokens.next().map(|x| x.value);
                n += 1;
            }
        }
        return if let Some(TokenValue::Equals()) = tokens.next().map(|x| x.value) {
            Some(Assignment::Tuple(identifiers))
        } else {
            tokens.backtrack();
            None
        };
    };

    tokens.backtrack();
    return None;
}
