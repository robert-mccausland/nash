use crate::{
    components::{errors::ParserError, root::Root},
    lexer::Token,
    utils::iterators::Backtrackable,
};

pub fn parse<'a, I: IntoIterator<Item = &'a Token<'a>>>(tokens: I) -> Result<Root, ParserError> {
    let tokens = &mut Backtrackable::new(tokens.into_iter());
    return Ok(Root::parse(tokens).map_err(|mut err| {
        if let Some(current) = tokens.peek() {
            err.set_position(current);
        }
        return err;
    })?);
}

#[cfg(test)]
mod tests {
    use crate::{
        components::{
            block::Block,
            expression::{BaseExpression, CaptureExitCode, Expression},
            function::Function,
            literals::StringLiteral,
            operator::Operator,
            statement::{Assignment, Statement},
        },
        constants::{EXEC, FUNC, IF, TRUE, VAR},
        lexer::TokenValue,
    };

    use super::*;

    #[test]
    fn should_parse_valid_tokens() {
        let tokens = vec![
            TokenValue::Keyword(FUNC),
            TokenValue::Identifier("main"),
            TokenValue::LeftBracket(),
            TokenValue::RightBracket(),
            TokenValue::LeftCurly(),
            TokenValue::Keyword(VAR),
            TokenValue::Identifier("template"),
            TokenValue::Equals(),
            TokenValue::DoubleQuote(),
            TokenValue::StringLiteral("cheese"),
            TokenValue::DoubleQuote(),
            TokenValue::Semicolon(),
            TokenValue::Keyword(VAR),
            TokenValue::Identifier("test_identifier"),
            TokenValue::Equals(),
            TokenValue::DoubleQuote(),
            TokenValue::StringLiteral("Blue \\\""),
            TokenValue::Dollar(),
            TokenValue::LeftCurly(),
            TokenValue::Identifier("template"),
            TokenValue::RightCurly(),
            TokenValue::StringLiteral("\\\" and rice!"),
            TokenValue::DoubleQuote(),
            TokenValue::Semicolon(),
            TokenValue::Keyword(IF),
            TokenValue::IntegerLiteral("1"),
            TokenValue::Plus(),
            TokenValue::IntegerLiteral("1"),
            TokenValue::Equals(),
            TokenValue::Equals(),
            TokenValue::IntegerLiteral("2"),
            TokenValue::LeftCurly(),
            TokenValue::Identifier("out"),
            TokenValue::LeftBracket(),
            TokenValue::Identifier("test_identifier"),
            TokenValue::RightBracket(),
            TokenValue::Semicolon(),
            TokenValue::RightCurly(),
            TokenValue::Semicolon(),
            TokenValue::Keyword(EXEC),
            TokenValue::Backtick(),
            TokenValue::StringLiteral("echo"),
            TokenValue::StringLiteral("something"),
            TokenValue::Backtick(),
            TokenValue::Semicolon(),
            TokenValue::RightCurly(),
            TokenValue::Identifier("main"),
            TokenValue::LeftBracket(),
            TokenValue::RightBracket(),
            TokenValue::Semicolon(),
        ];

        let expected_syntax_tree = Root {
            functions: vec![Function {
                name: "main".into(),
                arguments: vec![],
                code: Block {
                    statements: vec![
                        Statement::Declaration(
                            Assignment::Simple("template".into()),
                            BaseExpression::StringLiteral("cheese".into()).into(),
                        ),
                        Statement::Declaration(
                            Assignment::Simple("test_identifier".into()),
                            BaseExpression::StringLiteral(StringLiteral::new(
                                vec![("Blue \"".to_owned(), "template".into())],
                                "\" and rice!".to_owned(),
                            ))
                            .into(),
                        ),
                        Statement::Expression(
                            BaseExpression::If(
                                vec![(
                                    Expression::new(
                                        BaseExpression::IntegerLiteral(1.into()),
                                        vec![
                                            (
                                                Operator::Addition,
                                                BaseExpression::IntegerLiteral(1.into()),
                                            ),
                                            (
                                                Operator::Equals,
                                                BaseExpression::IntegerLiteral(2.into()),
                                            ),
                                        ],
                                    ),
                                    Block {
                                        statements: vec![Statement::Expression(
                                            BaseExpression::FunctionCall(
                                                "out".into(),
                                                vec![BaseExpression::Variable(
                                                    "test_identifier".into(),
                                                )
                                                .into()],
                                            )
                                            .into(),
                                        )],
                                    },
                                )],
                                None,
                            )
                            .into(),
                        )
                        .into(),
                        Statement::Expression(
                            BaseExpression::Execute(
                                Box::new(
                                    BaseExpression::Command(
                                        vec!["echo".into(), "something".into()].into(),
                                    )
                                    .into(),
                                ),
                                None,
                            )
                            .into(),
                        )
                        .into(),
                    ],
                },
            }],
            statements: vec![Statement::Expression(
                BaseExpression::FunctionCall("main".into(), vec![]).into(),
            )],
        };

        parser_test(tokens, Ok(expected_syntax_tree));
    }

    #[test]
    fn should_parse_tuple_declaration() {
        let tokens = vec![
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

        let expected_tree = Root {
            functions: vec![],
            statements: vec![Statement::Declaration(
                Assignment::Tuple(vec!["one".into(), "two".into()]),
                BaseExpression::Tuple(vec![
                    BaseExpression::StringLiteral("hi".into()).into(),
                    BaseExpression::IntegerLiteral(123.into()).into(),
                ])
                .into(),
            )],
        };

        parser_test(tokens, Ok(expected_tree));
    }

    #[test]
    fn should_parse_exec_exit_code_capture() {
        let tokens = vec![
            TokenValue::Keyword(EXEC),
            TokenValue::Backtick(),
            TokenValue::StringLiteral("test"),
            TokenValue::Backtick(),
            TokenValue::Question(),
            TokenValue::Keyword(VAR),
            TokenValue::Identifier("exit_code"),
            TokenValue::Semicolon(),
        ];

        let expected_tree = Root {
            functions: vec![],
            statements: vec![Statement::Expression(
                BaseExpression::Execute(
                    Box::new(BaseExpression::Command(vec!["test".into()].into()).into()),
                    Some(CaptureExitCode::Declaration("exit_code".into())),
                )
                .into(),
            )],
        };

        parser_test(tokens, Ok(expected_tree));
    }

    #[test]
    fn should_parse_booleans() {
        let tokens = vec![
            TokenValue::Keyword(VAR),
            TokenValue::Identifier("my_variable"),
            TokenValue::Equals(),
            TokenValue::Keyword(TRUE),
            TokenValue::Semicolon(),
        ];

        let expected_tree = Root {
            functions: vec![],
            statements: vec![Statement::Declaration(
                Assignment::Simple("my_variable".into()),
                BaseExpression::BooleanLiteral(true.into()).into(),
            )],
        };

        parser_test(tokens, Ok(expected_tree));
    }

    fn parser_test(tokens: Vec<TokenValue>, expected_tree: Result<Root, ParserError>) {
        let tokens = tokens
            .into_iter()
            .map(|value| Token::new(value, 0, 0))
            .collect::<Vec<_>>();
        let tree = parse(tokens.iter());
        assert_eq!(
            tree, expected_tree,
            "Checking expected tree matches actual tree, actual on left"
        )
    }
}
