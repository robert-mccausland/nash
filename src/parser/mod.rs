use std::{error::Error, fmt::Display};

use code_blocks::{parse_code_block, CodeBlock};

use crate::lexer::Token;

pub mod code_blocks;
pub mod expressions;
pub mod functions;
pub mod literals;
pub mod operators;
pub mod statements;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    pub value: String,
}

impl From<&str> for Identifier {
    fn from(value: &str) -> Self {
        Identifier {
            value: value.to_owned(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParserError {
    message: String,
}

impl ParserError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for ParserError {
    fn from(value: &str) -> Self {
        ParserError::new(value.to_owned())
    }
}

impl From<String> for ParserError {
    fn from(value: String) -> Self {
        ParserError::new(value)
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ParserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

struct Backtrackable<I: Iterator>
where
    I::Item: Clone,
{
    position: usize,
    history: Vec<I::Item>,
    source: I,
}

impl<I: Iterator> Backtrackable<I>
where
    I::Item: Clone,
{
    fn new(source: I) -> Self {
        Self {
            position: 0,
            source,
            history: Vec::new(),
        }
    }

    fn backtrack(&mut self) {
        self.backtrack_n(1);
    }

    fn backtrack_n(&mut self, n: usize) {
        self.position += n;

        if self.position > self.history.len() {
            panic!("Cannot backtrack past the start of the iterator")
        }
    }

    fn peek(&mut self) -> Option<I::Item> {
        let next = self.next();
        self.backtrack();
        return next;
    }
}

impl<I: Iterator> Iterator for Backtrackable<I>
where
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position == 0 {
            if let Some(next) = self.source.next() {
                self.history.push(next.clone());
                Some(next)
            } else {
                None
            }
        } else {
            let next = self.history[self.history.len() - self.position].clone();
            self.position -= 1;
            Some(next)
        }
    }
}

pub fn parse<'a>(tokens: impl IntoIterator<Item = Token<'a>>) -> Result<CodeBlock, ParserError> {
    let tokens = &mut Backtrackable::new(tokens.into_iter());
    match parse_code_block(tokens, true) {
        Ok(code) => Ok(code),
        Err(err) => {
            println!("{:?}", tokens.peek());
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use code_blocks::CodeBlock;
    use expressions::Expression;
    use functions::Function;
    use literals::StringLiteral;
    use operators::Operator;
    use statements::{Assignment, Statement};

    use crate::{
        constants::{EXEC, FUNC, IF, TRUE, VAR},
        lexer::{FilePosition, TokenValue},
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

        let expected_syntax_tree = CodeBlock {
            functions: vec![Function {
                name: "main".into(),
                arguments: vec![],
                code: CodeBlock {
                    functions: vec![],
                    statements: vec![
                        Statement::Declaration(
                            Assignment::Simple("template".into()),
                            Expression::StringLiteral("cheese".into()),
                        ),
                        Statement::Declaration(
                            Assignment::Simple("test_identifier".into()),
                            Expression::StringLiteral(StringLiteral::new(
                                vec![("Blue \"".to_owned(), "template".into())],
                                "\" and rice!".to_owned(),
                            )),
                        ),
                        Statement::Expression(Expression::If(
                            vec![(
                                Expression::Operation(
                                    Box::new(Expression::Operation(
                                        Box::new(Expression::IntegerLiteral(1)),
                                        Operator::Addition,
                                        Box::new(Expression::IntegerLiteral(1)),
                                    )),
                                    Operator::Equals,
                                    Box::new(Expression::IntegerLiteral(2)),
                                ),
                                CodeBlock {
                                    functions: vec![],
                                    statements: vec![Statement::Expression(
                                        Expression::FunctionCall(
                                            "out".into(),
                                            vec![Expression::Variable("test_identifier".into())],
                                        ),
                                    )],
                                },
                            )],
                            None,
                        )),
                        Statement::Expression(Expression::Execute(
                            Box::new(Expression::Command(vec!["echo".into(), "something".into()])),
                            None,
                        )),
                    ],
                },
            }],
            statements: vec![Statement::Expression(Expression::FunctionCall(
                "main".into(),
                vec![],
            ))],
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

        let expected_tree = CodeBlock {
            functions: vec![],
            statements: vec![Statement::Declaration(
                Assignment::Tuple(vec!["one".into(), "two".into()]),
                Expression::Tuple(vec![
                    Expression::StringLiteral("hi".into()),
                    Expression::IntegerLiteral(123),
                ]),
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

        let expected_tree = CodeBlock {
            functions: vec![],
            statements: vec![Statement::Expression(Expression::Execute(
                Box::new(Expression::Command(vec!["test".into()])),
                Some(expressions::CaptureExitCode::Declaration(
                    "exit_code".into(),
                )),
            ))],
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

        let expected_tree = CodeBlock {
            functions: vec![],
            statements: vec![Statement::Declaration(
                Assignment::Simple("my_variable".into()),
                Expression::BooleanLiteral(true),
            )],
        };

        parser_test(tokens, Ok(expected_tree));
    }

    fn parser_test(tokens: Vec<TokenValue>, expected_tree: Result<CodeBlock, ParserError>) {
        let tree = parse(
            tokens
                .into_iter()
                .map(|value| Token::new(value, FilePosition::new(0, 0), FilePosition::new(0, 0))),
        );

        assert_eq!(
            tree, expected_tree,
            "Checking expected tree matches actual tree, actual on left"
        )
    }
}
