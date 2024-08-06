use std::{
    error::Error,
    fmt::{Debug, Display},
    iter::Peekable,
};

use token_kinds::is_newline;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

mod token_kinds;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub value: TokenValue<'a>,
    pub start: FilePosition,
    pub end: FilePosition,
}

impl<'a> Token<'a> {
    pub fn new(value: TokenValue<'a>, start: FilePosition, end: FilePosition) -> Self {
        Self { value, start, end }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenValue<'a> {
    StringLiteral(&'a str),
    IntegerLiteral(&'a str),
    Identifier(&'a str),
    Keyword(&'a str),
    Equals(),
    Plus(),
    LeftBracket(),
    RightBracket(),
    LeftCurly(),
    RightCurly(),
    LeftAngle(),
    RightAngle(),
    Question(),
    Semicolon(),
    DoubleQuote(),
    Comma(),
    Dollar(),
    Backtick(),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePosition {
    pub line: usize,
    pub column: usize,
}

impl FilePosition {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum LexerContext {
    Root,
    Comment,
    String,
    Command,
    TemplateVariable,
}

#[derive(Debug)]
pub struct LexerError {
    pub message: String,
    pub position: Option<FilePosition>,
}

impl LexerError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            position: None,
        }
    }
}

impl From<&str> for LexerError {
    fn from(value: &str) -> Self {
        Self::new(value.to_owned())
    }
}

impl Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)?;
        self.position.fmt(f)?;

        Ok(())
    }
}

impl Error for LexerError {
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

pub fn lex<'a>(file: &'a str) -> Tokens<'a> {
    Tokens::new(file)
}

pub struct Tokens<'a> {
    buffer: &'a str,
    next: (usize, &'a str),
    iterator: Peekable<GraphemeIndices<'a>>,
    context_stack: Vec<LexerContext>,
    current_position: FilePosition,
}

impl<'a> Tokens<'a> {
    pub fn new(file: &'a str) -> Self {
        Self {
            buffer: file,
            next: (0, ""),
            iterator: file.grapheme_indices(true).peekable(),
            context_stack: vec![LexerContext::Root],
            current_position: FilePosition::new(0, 0),
        }
    }
}

impl<'a> Tokens<'a> {
    fn next(&mut self) -> Option<Result<Token<'a>, LexerError>> {
        loop {
            // If we are at the end of the file then the iterator is finished.
            let Some(next) = self.iterator.peek() else {
                return None;
            };

            self.next = *next;

            // If the advance method returns None it means that we need to call it again
            // as it found a token that should be ignored (e.g. whitespace)
            let start = self.current_position.clone();
            let next = self.advance();
            let end = self.current_position.clone();
            if let Ok(Some(token)) = next {
                return Some(Ok(Token::new(token, start, end)));
            }

            if let Err(mut err) = next {
                if err.position.is_none() {
                    err.position = Some(self.current_position.clone());
                }
                return Some(Err(err));
            }
        }
    }

    fn advance(&mut self) -> Result<Option<TokenValue<'a>>, LexerError> {
        let start = self.next.0;
        let mut end = start + self.next.1.len();
        let mut result = None;

        loop {
            let value = &self.buffer[start..end];

            // Try to parse the next token, returning the previously parsed token if we can't parse
            // it this time.
            let Some(new_result) = token_kinds::try_get_token_kind(&mut self.context_stack, value)
            else {
                return match result {
                    Some(result) => Ok(result),
                    None => Err("Could not parse token.".into()),
                };
            };

            result = Some(new_result.into_token(&value));

            // This is the point where we have accepted that next is part of the token, so we
            // advance the current position
            if is_newline(self.next.1) {
                self.current_position.line += 1;
                self.current_position.column = 0;
            } else {
                self.current_position.column += 1;
            }

            // Advance though the file if we did successfully parse the previous token to see if
            // the next character also makes a valid token.
            self.iterator.next();
            let Some(next) = self.iterator.peek() else {
                return match result {
                    Some(result) => Ok(result),
                    None => Err("Unexpected end of file.".into()),
                };
            };
            self.next = *next;
            end += self.next.1.len();
        }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Result<Token<'a>, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lexer(test_file: &str, expected_tokens: Vec<Token>) {
        let lexer = Tokens::new(test_file);
        let tokens = lexer.collect::<Result<Vec<_>, _>>().unwrap();

        let actual_len = tokens.len();
        let expected_len = expected_tokens.len();

        // Improve formatting of errors by comparing items individually
        let mut i = 0;
        for (actual, expected) in tokens.into_iter().zip(expected_tokens) {
            assert_eq!(
                actual, expected,
                "Checking that token at index {i} matches expected value (left value is actual)"
            );
            i += 1;
        }

        // Zip will stop when any of the iterators ends, so we still need to check that they are
        // both the same length. Checking afterwards makes the errors a bit nicer.
        assert_eq!(
            actual_len, expected_len,
            "Checking that the correct number of tokens was returned (left value is actual)"
        )
    }

    #[test]
    fn should_tokenize_valid_file() {
        let test_file = r#"
# Comments are fun!
func main() {
  var test_identifier = "Blue \"cheese\" and rice!";
  if 1 + 1 == 2 {
    out(test_identifier);
  };

  exec `echo something`;
}
"#;
        let expected_tokens = vec![
            Token::new(
                TokenValue::Keyword("func"),
                FilePosition::new(2, 0),
                FilePosition::new(2, 4),
            ),
            Token::new(
                TokenValue::Identifier("main"),
                FilePosition::new(2, 5),
                FilePosition::new(2, 9),
            ),
            Token::new(
                TokenValue::LeftBracket(),
                FilePosition::new(2, 9),
                FilePosition::new(2, 10),
            ),
            Token::new(
                TokenValue::RightBracket(),
                FilePosition::new(2, 10),
                FilePosition::new(2, 11),
            ),
            Token::new(
                TokenValue::LeftCurly(),
                FilePosition::new(2, 12),
                FilePosition::new(2, 13),
            ),
            Token::new(
                TokenValue::Keyword("var"),
                FilePosition::new(3, 2),
                FilePosition::new(3, 5),
            ),
            Token::new(
                TokenValue::Identifier("test_identifier"),
                FilePosition::new(3, 6),
                FilePosition::new(3, 21),
            ),
            Token::new(
                TokenValue::Equals(),
                FilePosition::new(3, 22),
                FilePosition::new(3, 23),
            ),
            Token::new(
                TokenValue::DoubleQuote(),
                FilePosition::new(3, 24),
                FilePosition::new(3, 25),
            ),
            Token::new(
                TokenValue::StringLiteral("Blue \\\"cheese\\\" and rice!"),
                FilePosition::new(3, 25),
                FilePosition::new(3, 50),
            ),
            Token::new(
                TokenValue::DoubleQuote(),
                FilePosition::new(3, 50),
                FilePosition::new(3, 51),
            ),
            Token::new(
                TokenValue::Semicolon(),
                FilePosition::new(3, 51),
                FilePosition::new(3, 52),
            ),
            Token::new(
                TokenValue::Keyword("if"),
                FilePosition::new(4, 2),
                FilePosition::new(4, 4),
            ),
            Token::new(
                TokenValue::IntegerLiteral("1"),
                FilePosition::new(4, 5),
                FilePosition::new(4, 6),
            ),
            Token::new(
                TokenValue::Plus(),
                FilePosition::new(4, 7),
                FilePosition::new(4, 8),
            ),
            Token::new(
                TokenValue::IntegerLiteral("1"),
                FilePosition::new(4, 9),
                FilePosition::new(4, 10),
            ),
            Token::new(
                TokenValue::Equals(),
                FilePosition::new(4, 11),
                FilePosition::new(4, 12),
            ),
            Token::new(
                TokenValue::Equals(),
                FilePosition::new(4, 12),
                FilePosition::new(4, 13),
            ),
            Token::new(
                TokenValue::IntegerLiteral("2"),
                FilePosition::new(4, 14),
                FilePosition::new(4, 15),
            ),
            Token::new(
                TokenValue::LeftCurly(),
                FilePosition::new(4, 16),
                FilePosition::new(4, 17),
            ),
            Token::new(
                TokenValue::Identifier("out"),
                FilePosition::new(5, 4),
                FilePosition::new(5, 7),
            ),
            Token::new(
                TokenValue::LeftBracket(),
                FilePosition::new(5, 7),
                FilePosition::new(5, 8),
            ),
            Token::new(
                TokenValue::Identifier("test_identifier"),
                FilePosition::new(5, 8),
                FilePosition::new(5, 23),
            ),
            Token::new(
                TokenValue::RightBracket(),
                FilePosition::new(5, 23),
                FilePosition::new(5, 24),
            ),
            Token::new(
                TokenValue::Semicolon(),
                FilePosition::new(5, 24),
                FilePosition::new(5, 25),
            ),
            Token::new(
                TokenValue::RightCurly(),
                FilePosition::new(6, 2),
                FilePosition::new(6, 3),
            ),
            Token::new(
                TokenValue::Semicolon(),
                FilePosition::new(6, 3),
                FilePosition::new(6, 4),
            ),
            Token::new(
                TokenValue::Keyword("exec"),
                FilePosition::new(8, 2),
                FilePosition::new(8, 6),
            ),
            Token::new(
                TokenValue::Backtick(),
                FilePosition::new(8, 7),
                FilePosition::new(8, 8),
            ),
            Token::new(
                TokenValue::StringLiteral("echo"),
                FilePosition::new(8, 8),
                FilePosition::new(8, 12),
            ),
            Token::new(
                TokenValue::StringLiteral("something"),
                FilePosition::new(8, 13),
                FilePosition::new(8, 22),
            ),
            Token::new(
                TokenValue::Backtick(),
                FilePosition::new(8, 22),
                FilePosition::new(8, 23),
            ),
            Token::new(
                TokenValue::Semicolon(),
                FilePosition::new(8, 23),
                FilePosition::new(8, 24),
            ),
            Token::new(
                TokenValue::RightCurly(),
                FilePosition::new(9, 0),
                FilePosition::new(9, 1),
            ),
        ];

        test_lexer(&test_file, expected_tokens);
    }

    #[test]

    fn should_parse_empty_string() {
        let test_file = r#"var test = "";"#;
        let expected_tokens = vec![
            Token::new(
                TokenValue::Keyword("var"),
                FilePosition::new(0, 0),
                FilePosition::new(0, 3),
            ),
            Token::new(
                TokenValue::Identifier("test"),
                FilePosition::new(0, 4),
                FilePosition::new(0, 8),
            ),
            Token::new(
                TokenValue::Equals(),
                FilePosition::new(0, 9),
                FilePosition::new(0, 10),
            ),
            Token::new(
                TokenValue::DoubleQuote(),
                FilePosition::new(0, 11),
                FilePosition::new(0, 12),
            ),
            Token::new(
                TokenValue::DoubleQuote(),
                FilePosition::new(0, 12),
                FilePosition::new(0, 13),
            ),
            Token::new(
                TokenValue::Semicolon(),
                FilePosition::new(0, 13),
                FilePosition::new(0, 14),
            ),
        ];

        test_lexer(&test_file, expected_tokens);
    }

    #[test]

    fn should_parse_template_string() {
        let test_file = r#"var test = "hello ${value}!";"#;
        let expected_tokens = vec![
            Token::new(
                TokenValue::Keyword("var"),
                FilePosition::new(0, 0),
                FilePosition::new(0, 3),
            ),
            Token::new(
                TokenValue::Identifier("test"),
                FilePosition::new(0, 4),
                FilePosition::new(0, 8),
            ),
            Token::new(
                TokenValue::Equals(),
                FilePosition::new(0, 9),
                FilePosition::new(0, 10),
            ),
            Token::new(
                TokenValue::DoubleQuote(),
                FilePosition::new(0, 11),
                FilePosition::new(0, 12),
            ),
            Token::new(
                TokenValue::StringLiteral("hello "),
                FilePosition::new(0, 12),
                FilePosition::new(0, 18),
            ),
            Token::new(
                TokenValue::Dollar(),
                FilePosition::new(0, 18),
                FilePosition::new(0, 19),
            ),
            Token::new(
                TokenValue::LeftCurly(),
                FilePosition::new(0, 19),
                FilePosition::new(0, 20),
            ),
            Token::new(
                TokenValue::Identifier("value"),
                FilePosition::new(0, 20),
                FilePosition::new(0, 25),
            ),
            Token::new(
                TokenValue::RightCurly(),
                FilePosition::new(0, 25),
                FilePosition::new(0, 26),
            ),
            Token::new(
                TokenValue::StringLiteral("!"),
                FilePosition::new(0, 26),
                FilePosition::new(0, 27),
            ),
            Token::new(
                TokenValue::DoubleQuote(),
                FilePosition::new(0, 27),
                FilePosition::new(0, 28),
            ),
            Token::new(
                TokenValue::Semicolon(),
                FilePosition::new(0, 28),
                FilePosition::new(0, 29),
            ),
        ];

        test_lexer(&test_file, expected_tokens);
    }
}
