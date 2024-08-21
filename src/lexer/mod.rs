use std::iter::Peekable;

use serde::Serialize;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

use crate::errors::LexerError;

mod token_kinds;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token<'a> {
    pub value: TokenValue<'a>,
    pub start: usize,
    pub end: usize,
}

impl<'a> Token<'a> {
    pub fn new(value: TokenValue<'a>, start: usize, end: usize) -> Self {
        Self { value, start, end }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    LeftSquare(),
    RightSquare(),
    Question(),
    Dot(),
    Colon(),
    Semicolon(),
    DoubleQuote(),
    Comma(),
    Dollar(),
    Bang(),
    Backtick(),
}

#[derive(Debug, PartialEq, Eq)]
enum LexerContext {
    Root,
    Comment,
    String,
    Command,
    TemplateVariable,
}

pub fn lex<'a>(file: &'a str) -> Tokens<'a> {
    Tokens::new(file)
}

pub struct Tokens<'a> {
    buffer: &'a str,
    next: (usize, &'a str),
    iterator: Peekable<GraphemeIndices<'a>>,
    context_stack: Vec<LexerContext>,
}

impl<'a> Tokens<'a> {
    pub fn new(file: &'a str) -> Self {
        Self {
            buffer: file,
            next: (0, ""),
            iterator: file.grapheme_indices(true).peekable(),
            context_stack: vec![LexerContext::Root],
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
            let next = self.advance();
            if let Ok(Some(token)) = next {
                return Some(Ok(token));
            }

            if let Err(mut err) = next {
                if err.position.is_none() {
                    err.position = Some(self.next.0);
                }
                return Some(Err(err));
            }
        }
    }

    fn advance(&mut self) -> Result<Option<Token<'a>>, LexerError> {
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

            result = Some(
                new_result
                    .into_token(&value)
                    .map(|value| Token::new(value, start, end)),
            );

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
    use insta::assert_yaml_snapshot;

    use super::*;

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
        let tokens = lex(test_file).collect::<Result<Vec<_>, _>>().unwrap();
        assert_yaml_snapshot!(tokens);
    }

    #[test]
    fn should_parse_empty_string() {
        let test_file = r#"var test = "";"#;
        let tokens = lex(test_file).collect::<Result<Vec<_>, _>>().unwrap();
        assert_yaml_snapshot!(tokens);
    }

    #[test]
    fn should_parse_template_string() {
        let test_file = r#"var test = "hello ${value}!";"#;
        let tokens = lex(test_file).collect::<Result<Vec<_>, _>>().unwrap();
        assert_yaml_snapshot!(tokens);
    }

    #[test]
    fn should_parse_template_string_with_keyword_substring() {
        let test_file = r#""${index}""#;
        let tokens = lex(test_file).collect::<Result<Vec<_>, _>>().unwrap();
        assert_yaml_snapshot!(tokens);
    }
}
