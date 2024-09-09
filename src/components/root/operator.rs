use serde::Serialize;

use crate::{
    components::values::{Type, Value},
    lexer::{Token, TokenValue},
    ParserError,
};

use super::{Backtrackable, ExecutionError, Tokens};

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
        match_tokens!([Equals, Equals] => Equal);
        match_tokens!([Bang, Equals] => NotEqual);
        match_tokens!([And, And] => And);
        match_tokens!([Pipe, Pipe] => Or);

        return Ok(None);
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

        return false;
    }
}

macro_rules! impl_operator {
    [$($operator:ident($left:ident, $right:ident) -> $return_type:ident { $operation:expr }),*] => {
        impl Operator {
            pub fn execute(&self, left: Value, right: Value) -> Result<Value, ExecutionError> {
                match (self, left, right) {
                    $(
                        (Operator::$operator, Value::$left(left), Value::$right(right)) => Ok(($operation)(left, right).into()),
                    )*

                    // Manually implement operators that act on all types for now to make the macro simpler
                    (Operator::Equal, left, right) => Ok((left == right).into()),
                    (Operator::NotEqual, left, right) => Ok((left != right).into()),

                    (operator, left, right) => {
                        Err(format!("Invalid operator expression {left:?} {operator:?} {right:?}.").into())
                    }
                }
            }

            pub fn get_type(&self, left: Type, right: Type) -> Result<Type, String> {
                match (self, left, right) {
                    $(
                        (Operator::$operator, Type::$left, Type::$right) => Ok(Type::$return_type),
                    )*
                    (Operator::Equal, _, _) => Ok(Type::Boolean),
                    (Operator::NotEqual, _, _) => Ok(Type::Boolean),
                    (operator, left, right) => {
                        Err(format!("Invalid operator expression {left:?} {operator:?} {right:?}.").into())
                    }
                }
            }
        }
    };
}

impl_operator![
    Addition(Integer, Integer) -> Integer {
        |left, right| left + right
    },
    Addition(String, String) -> String {
        |left, right: String| left + right.as_str()
    },
    Subtraction(Integer, Integer) -> Integer {
        |left, right| left - right
    },
    Multiplication(Integer, Integer) -> Integer {
        |left, right| left * right
    },
    Division(Integer, Integer) -> Integer {
        |left, right| left / right
    },
    Remainder(Integer, Integer) -> Integer {
        |left, right| left % right
    },
    LessThan(Integer, Integer) -> Boolean{
        |left, right| left < right
    },
    GreaterThan(Integer, Integer) -> Boolean{
        |left, right| left > right
    },
    LessThanOrEqual(Integer, Integer) -> Boolean{
        |left, right| left <= right
    },
    GreaterThanOrEqual(Integer, Integer) -> Boolean{
        |left, right| left >= right
    },
    And(Boolean, Boolean) -> Boolean{
        |left, right| left && right
    },
    Or(Boolean, Boolean) -> Boolean{
        |left, right| left || right
    }
];
