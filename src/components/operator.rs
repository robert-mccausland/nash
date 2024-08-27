use serde::Serialize;

use crate::{
    executor::Value,
    lexer::{Token, TokenValue},
    Command, ParserError,
};

use super::{errors::ExecutionError, Backtrackable, Tokens};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Operator {
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Remainder,
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Arrow,
    And,
    Or,
}

impl Operator {
    pub(super) fn try_parse<'a, I: Iterator<Item = &'a Token<'a>>>(
        tokens: &mut Backtrackable<I>,
    ) -> Result<Option<Operator>, ParserError> {
        macro_rules! match_tokens {
            ([$($token:ident),+] => $result:expr) => {
                {
                    'block: {
                        let checkpoint = tokens.checkpoint();
                        $(
                            let Some(TokenValue::$token()) = tokens.next_value() else {
                                tokens.backtrack(checkpoint);
                                break 'block
                            };
                        )*

                        {
                            use Operator::*;
                            return Ok(Some($result));
                        }
                    };
                }
            };
        }

        match_tokens!([Plus] => Addition);
        match_tokens!([Dash] => Subtraction);
        match_tokens!([Star] => Multiplication);
        match_tokens!([ForwardSlash] => Division);
        match_tokens!([Percent] => Remainder);
        match_tokens!([LeftAngle] => LessThan);
        match_tokens!([LeftAngle, Equals] => LessThanOrEqual);
        match_tokens!([RightAngle] => GreaterThan);
        match_tokens!([RightAngle, Equals] => GreaterThanOrEqual);
        match_tokens!([Equals, RightAngle] => Arrow);
        match_tokens!([Equals, Equals] => Equal);
        match_tokens!([Bang, Equals] => NotEqual);
        match_tokens!([And, And] => And);
        match_tokens!([Pipe, Pipe] => Or);

        return Ok(None);
    }

    pub fn execute(&self, left: Value, right: Value) -> Result<Value, ExecutionError> {
        use Operator::*;
        use Value::*;

        // Trust me its really not that bad
        match (self, left, right) {
            (Addition, Integer(left), Integer(right)) => Ok((left + right).into()),
            (Subtraction, Integer(left), Integer(right)) => Ok((left - right).into()),
            (Multiplication, Integer(left), Integer(right)) => Ok((left * right).into()),
            (Division, Integer(left), Integer(right)) => Ok((left / right).into()),
            (Remainder, Integer(left), Integer(right)) => Ok((left % right).into()),
            (Equal, left, right) => Ok((left == right).into()),
            (NotEqual, left, right) => Ok((left != right).into()),
            (LessThan, Integer(left), Integer(right)) => Ok((left < right).into()),
            (GreaterThan, Integer(left), Integer(right)) => Ok((left > right).into()),
            (LessThanOrEqual, Integer(left), Integer(right)) => Ok((left <= right).into()),
            (GreaterThanOrEqual, Integer(left), Integer(right)) => Ok((left >= right).into()),
            (Arrow, Command(left), Command(right)) => Ok(Command(left.pipe(right)?)),
            (Arrow, String(left), Command(right)) => {
                Ok(Command(crate::Command::literal(left).pipe(right)?))
            }
            (And, Boolean(left), Boolean(right)) => Ok((left && right).into()),
            (Or, Boolean(left), Boolean(right)) => Ok((left || right).into()),
            (operator, left, right) => {
                Err(format!("Invalid operator expression {left:?} {operator:?} {right:?}.").into())
            }
        }
    }

    pub fn chains_with(&self, value: &Self) -> bool {
        macro_rules! return_true_if_match {
            ($pattern:pat) => {{
                if matches!(self, $pattern) && matches!(value, $pattern) {
                    return true;
                }
            }};
        }

        return_true_if_match!(Self::Multiplication);
        return_true_if_match!(Self::Addition | Self::Subtraction);
        return_true_if_match!(Self::And | Self::Or);
        return_true_if_match!(Self::Arrow);

        return false;
    }
}
