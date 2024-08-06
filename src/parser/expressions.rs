use crate::{
    keywords::VAR,
    lexer::{Token, TokenValue},
};

use super::{
    code_blocks::{parse_code_block, CodeBlock},
    literals::{parse_integer, parse_string_literal, StringLiteral},
    operators::{parse_operator, Operator},
    Backtrackable, Identifier, ParserError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    StringLiteral(StringLiteral),
    BooleanLiteral(bool),
    IntegerLiteral(i32),
    Tuple(Vec<Expression>),
    Variable(Identifier),
    Command(Vec<StringLiteral>),
    Operation(Box<Expression>, Operator, Box<Expression>),
    If(Vec<(Expression, CodeBlock)>, Option<CodeBlock>),
    FunctionCall(Identifier, Vec<Expression>),
    Execute(Box<Expression>, Option<CaptureExitCode>),
}

#[derive(Debug, Clone, PartialEq, Eq)]

pub enum CaptureExitCode {
    Assignment(Identifier),
    Declaration(Identifier),
}

pub(super) fn parse_expression<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Result<Expression, ParserError> {
    let mut expression = parse_base_expression(tokens)?;
    while let Some(operator) = parse_operator(tokens) {
        expression = Expression::Operation(
            Box::new(expression),
            operator,
            Box::new(parse_base_expression(tokens)?),
        );
    }

    return Ok(expression);
}

fn parse_base_expression<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Result<Expression, ParserError> {
    let token = tokens.next().map(|x| x.value);
    if let Some(TokenValue::IntegerLiteral(literal)) = token {
        return Ok(Expression::IntegerLiteral(parse_integer(literal)?));
    }

    if let Some(TokenValue::DoubleQuote()) = token {
        return Ok(Expression::StringLiteral(parse_string_literal(tokens)?));
    }

    if let Some(TokenValue::Backtick()) = token {
        let mut command = Vec::new();
        loop {
            let next = tokens.next().map(|x| x.value);
            command.push(if let Some(TokenValue::DoubleQuote()) = next {
                parse_string_literal(tokens)?
            } else if let Some(TokenValue::StringLiteral(value)) = next {
                value.into()
            } else if let Some(TokenValue::Backtick()) = next {
                break;
            } else {
                return Err("Unable to parse command".into());
            });
        }

        return Ok(Expression::Command(command));
    }

    if let Some(TokenValue::Identifier(identifier)) = token {
        let identifier = Identifier {
            value: identifier.to_owned(),
        };

        if matches!(
            tokens.next().map(|x| x.value),
            Some(TokenValue::LeftBracket())
        ) {
            let mut args = Vec::new();
            let mut next = tokens.next().map(|x| x.value);
            while !matches!(next, Some(TokenValue::RightBracket())) {
                tokens.backtrack();
                args.push(parse_expression(tokens)?);
                next = tokens.next().map(|x| x.value);

                if !matches!(
                    next,
                    Some(TokenValue::RightBracket()) | Some(TokenValue::Comma())
                ) {
                    return Err("Expected function argument to be followed by `,` or `)`".into());
                }
            }

            return Ok(Expression::FunctionCall(identifier, args));
        } else {
            tokens.backtrack();
            return Ok(Expression::Variable(identifier));
        }
    }

    if let Some(TokenValue::Keyword(ref keyword)) = token {
        if *keyword == "if" {
            let expression = parse_expression(tokens)?;
            let block = parse_code_block(tokens, false)?;
            if let Some(TokenValue::Keyword("else")) = tokens.next().map(|x| x.value) {
                return Ok(Expression::If(
                    vec![(expression, block)],
                    Some(parse_code_block(tokens, false)?),
                ));
            } else {
                tokens.backtrack();
                return Ok(Expression::If(vec![(expression, block)], None));
            }
        }
        if *keyword == "exec" {
            let expression = parse_expression(tokens)?;
            if let Some(TokenValue::Question()) = tokens.next().map(|x| x.value) {
                let token = tokens.next().map(|x| x.value);
                if let Some(TokenValue::Keyword(VAR)) = token {
                    let Some(TokenValue::Identifier(identifier)) = tokens.next().map(|x| x.value)
                    else {
                        return Err("var must be followed by identifier".into());
                    };
                    return Ok(Expression::Execute(
                        Box::new(expression),
                        Some(CaptureExitCode::Declaration(identifier.into())),
                    ));
                } else if let Some(TokenValue::Identifier(identifier)) = token {
                    return Ok(Expression::Execute(
                        Box::new(expression),
                        Some(CaptureExitCode::Assignment(identifier.into())),
                    ));
                } else {
                    return Err("? must be followed by an var or identifier".into());
                }
            } else {
                tokens.backtrack();
                return Ok(Expression::Execute(Box::new(expression), None));
            }
        }

        if *keyword == "false" {
            return Ok(Expression::BooleanLiteral(false));
        }

        if *keyword == "true" {
            return Ok(Expression::BooleanLiteral(true));
        }
    }

    if let Some(TokenValue::LeftBracket()) = token {
        let mut expressions = Vec::new();
        if let Some(TokenValue::RightBracket()) = tokens.next().map(|x| x.value) {
        } else {
            tokens.backtrack();
            loop {
                expressions.push(parse_expression(tokens)?);

                let next = tokens.next().map(|x| x.value);
                let Some(TokenValue::Comma()) = next else {
                    if let Some(TokenValue::RightBracket()) = next {
                        break;
                    }
                    return Err("Expected , or ) after tuple value".into());
                };
            }
        }

        return Ok(Expression::Tuple(expressions));
    }

    return Err("Unable to parse expression".into());
}
