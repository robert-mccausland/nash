use crate::{
    components::root::Root, errors::ParserError, lexer::Token, utils::iterators::Backtrackable,
};

pub fn parse<'a>(tokens: Vec<Token>) -> Result<Root, ParserError> {
    let tokens = &mut Backtrackable::new(tokens.iter());
    return Ok(Root::parse(tokens).map_err(|mut err| {
        if let Some(current) = tokens.peek() {
            err.set_position(current);
        }
        return err;
    })?);
}

#[cfg(test)]
mod tests {
    use insta::assert_yaml_snapshot;

    use crate::{
        constants::{EXEC, TRUE, VAR},
        lexer::TokenValue,
    };

    use super::*;

    macro_rules! tokens {
        ($($token:expr,)*) => {
            vec![
                $(
                    Token::new(
                        $token,0,0
                    ),
                )*
            ]
        };
    }

    #[test]
    fn should_parse_tuple_declaration() {
        let tokens = tokens![
            TokenValue::Keyword("var"),
            TokenValue::LeftBracket(),
            TokenValue::Identifier("one"),
            TokenValue::Comma(),
            TokenValue::Identifier("two"),
            TokenValue::RightBracket(),
            TokenValue::Equals(),
            TokenValue::LeftBracket(),
            TokenValue::DoubleQuote(),
            TokenValue::StringLiteral("hi"),
            TokenValue::DoubleQuote(),
            TokenValue::Comma(),
            TokenValue::IntegerLiteral("123"),
            TokenValue::RightBracket(),
            TokenValue::Semicolon(),
        ];

        assert_yaml_snapshot!(parse(tokens).unwrap());
    }

    #[test]
    fn should_parse_exec_exit_code_capture() {
        let tokens = tokens![
            TokenValue::Keyword(EXEC),
            TokenValue::Backtick(),
            TokenValue::StringLiteral("test"),
            TokenValue::Backtick(),
            TokenValue::Question(),
            TokenValue::Identifier("exit_code"),
            TokenValue::Semicolon(),
        ];

        assert_yaml_snapshot!(parse(tokens).unwrap());
    }

    #[test]
    fn should_parse_booleans() {
        let tokens = tokens![
            TokenValue::Keyword(VAR),
            TokenValue::Identifier("my_variable"),
            TokenValue::Equals(),
            TokenValue::Keyword(TRUE),
            TokenValue::Semicolon(),
        ];

        assert_yaml_snapshot!(parse(tokens).unwrap());
    }

    #[test]
    fn should_parse_instance_method() {
        let tokens = tokens![
            TokenValue::Identifier("array"),
            TokenValue::Dot(),
            TokenValue::Identifier("push"),
            TokenValue::LeftBracket(),
            TokenValue::Identifier("value"),
            TokenValue::RightBracket(),
            TokenValue::Semicolon(),
        ];

        assert_yaml_snapshot!(parse(tokens).unwrap());
    }
}
