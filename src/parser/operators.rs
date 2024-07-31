use crate::lexer::{Token, TokenValue};

use super::Backtrackable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    LessThan,
    GreaterThan,
    Addition,
    Equals,
    Pipe,
}

pub(super) fn parse_operator<'a, I: Iterator<Item = Token<'a>>>(
    tokens: &mut Backtrackable<I>,
) -> Option<Operator> {
    let Some(next) = tokens.next().map(|x| x.value) else {
        return None;
    };

    if let TokenValue::LeftAngle() = next {
        return Some(Operator::LessThan);
    }

    if let TokenValue::RightAngle() = next {
        return Some(Operator::GreaterThan);
    }

    if let TokenValue::Plus() = next {
        return Some(Operator::Addition);
    }

    if let TokenValue::Equals() = next {
        let next = tokens.next().map(|x| x.value);
        if let Some(TokenValue::RightAngle()) = next {
            return Some(Operator::Pipe);
        }

        if let Some(TokenValue::Equals()) = next {
            return Some(Operator::Equals);
        }
        tokens.backtrack();
    }

    tokens.backtrack();
    return None;
}
