use std::iter::Peekable;

use serde::Serialize;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

use crate::errors::LexerError;
pub use tokens::TokenValue;

mod tokens;

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

#[derive(Debug, PartialEq, Eq)]
enum LexerContext {
    Root,
    Comment,
    String(bool),
    Command,
    TemplateExpression(u32),
}

pub fn lex<'a>(code: &'a str) -> Tokens<'a> {
    Tokens::new(code)
}

pub struct Tokens<'a> {
    buffer: &'a str,
    next: (usize, &'a str),
    iterator: Peekable<GraphemeIndices<'a>>,
    context_stack: Vec<LexerContext>,
}

impl<'a> Tokens<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            buffer: code,
            next: (0, ""),
            iterator: code.grapheme_indices(true).peekable(),
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

            // Try to parse the next token
            match tokens::try_get_token(&mut self.context_stack, value) {
                tokens::GetTokenResult::Match(token_kind) => {
                    self.iterator.next();

                    let token = token_kind.into_token(value);
                    if token_kind.is_greedy() {
                        result = Some(token);
                    } else {
                        return Ok(Some(Token::new(token, start, end)));
                    }
                }
                tokens::GetTokenResult::NoMatch() => {
                    return match result {
                        Some(token) => Ok(Some(Token::new(token, start, end - self.next.1.len()))),
                        None => Err("Unable to match token".into()),
                    }
                }
                tokens::GetTokenResult::Skip() => {
                    self.iterator.next();
                    return Ok(None);
                }
            };

            // If we need to get more tokens then match more
            let Some(next) = self.iterator.peek() else {
                return match result {
                    Some(token) => Ok(Some(Token::new(token, start, end))),
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

    fn lex_code<'a>(code: &'a str) -> Vec<Token<'a>> {
        return lex(code).collect::<Result<Vec<_>, _>>().unwrap();
    }

    #[test]
    fn should_tokenize_valid_file() {
        assert_yaml_snapshot!(lex_code(
            r#"
# Comments are fun!
func main() {
  var test_identifier = "Blue \"cheese\" and rice!";
  if 1 + 1 == 2 {
    out(test_identifier);
  };

  exec `echo something`;
}
"#,
        ));
    }

    #[test]
    fn should_parse_empty_string() {
        assert_yaml_snapshot!(lex_code(r#"var test = "";"#));
    }

    #[test]
    fn should_parse_template_string() {
        assert_yaml_snapshot!(lex_code(r#"var test = "hello ${value}!";"#));
    }

    #[test]
    fn should_parse_template_string_with_keyword_substring() {
        assert_yaml_snapshot!(lex_code(r#""${index}""#));
    }

    #[test]
    fn should_parse_nested_expressions_in_string_literal_templates() {
        assert_yaml_snapshot!(lex_code(r#""test ${index.fmt()}!""#));
    }

    #[test]
    fn should_parse_multiline_nested_expressions_in_string_literal_templates() {
        assert_yaml_snapshot!(lex_code(
            r#""test ${if true {
            "yes!";
        } else {
            "false!";
        }}!""#,
        ));
    }
}
