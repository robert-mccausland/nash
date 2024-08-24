use serde::Serialize;

use crate::constants::KEYWORDS;

use super::LexerContext;

const HASH: &str = "#";
const BACKSLASH: &str = "\\";
const NEWLINES: [&str; 2] = ["\n", "\r\n"];
const DOUBLE_QUOTE: &str = "\"";
const BACKTICK: &str = "`";
const EQUALS: &str = "=";
const PLUS: &str = "+";
const LEFT_BRACKET: &str = "(";
const RIGHT_BRACKET: &str = ")";
const LEFT_CURLY: &str = "{";
const RIGHT_CURLY: &str = "}";
const LEFT_ANGLE: &str = "<";
const RIGHT_ANGLE: &str = ">";
const LEFT_SQUARE: &str = "[";
const RIGHT_SQUARE: &str = "]";
const QUESTION: &str = "?";
const DOT: &str = ".";
const COLON: &str = ":";
const SEMICOLON: &str = ";";
const COMMA: &str = ",";
const BANG: &str = "!";
const DOLLAR: &str = "$";

macro_rules! define_tokens {
    {complex => [$($complex_name:ident,)*], simple => [$($simple_value:ident => $simple_name:ident,)*]} => {
      #[derive(Debug, Clone, PartialEq, Eq)]
      pub enum TokenKind {
        $(
          $complex_name,
        )*
        $(
          $simple_name,
        )*
      }

      impl TokenKind {
        pub fn into_token<'a>(&self, value: &'a str) -> TokenValue<'a> {
          match self {
            $(
              Self::$complex_name => TokenValue::$complex_name(value),
            )*
            $(
              Self::$simple_name => TokenValue::$simple_name(),
            )*
          }
        }

        pub fn is_greedy(&self) -> bool {
          match self {
            $(
              Self::$complex_name => true,
            )*
            _ => false,
          }
        }

        pub fn match_simple_token(value: &str) -> Option<Self> {
          match value {
            $(
              $simple_value => Some(Self::$simple_name),
            )*
            _ => None,
          }
        }
      }

      #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
      pub enum TokenValue<'a> {
        $(
          $complex_name(&'a str),
        )*
        $(
          $simple_name(),
        )*
      }
    };
}

define_tokens! {
  complex => [
    StringLiteral,
    IntegerLiteral,
    Identifier,
    Keyword,
  ],
  simple => [
    DOUBLE_QUOTE => DoubleQuote,
    BACKTICK => Backtick,
    EQUALS => Equals,
    PLUS => Plus,
    LEFT_BRACKET => LeftBracket,
    RIGHT_BRACKET => RightBracket,
    LEFT_CURLY => LeftCurly,
    RIGHT_CURLY => RightCurly,
    LEFT_ANGLE => LeftAngle,
    RIGHT_ANGLE => RightAngle,
    LEFT_SQUARE => LeftSquare,
    RIGHT_SQUARE => RightSquare,
    QUESTION => Question,
    DOT => Dot,
    COLON => Colon,
    SEMICOLON => Semicolon,
    COMMA => Comma,
    BANG => Bang,
    DOLLAR => Dollar,
  ]
}

pub enum GetTokenResult {
    // The current string matched something and may take extra characters
    Match(TokenKind),

    // The current string couldn't match anything
    NoMatch(),

    // The current string should be skipped over
    Skip(),
}

impl From<TokenKind> for GetTokenResult {
    fn from(value: TokenKind) -> Self {
        Self::Match(value)
    }
}

pub fn try_get_token(context_stack: &mut Vec<LexerContext>, current: &str) -> GetTokenResult {
    match context_stack.last_mut().unwrap() {
        LexerContext::Root => match current {
            HASH => {
                context_stack.push(LexerContext::Comment);
                GetTokenResult::Skip()
            }
            DOUBLE_QUOTE => {
                context_stack.push(LexerContext::String(false));
                TokenKind::DoubleQuote.into()
            }
            BACKTICK => {
                context_stack.push(LexerContext::Command);
                TokenKind::Backtick.into()
            }
            _ => {
                if let Some(value) = TokenKind::match_simple_token(current) {
                    value.into()
                } else if KEYWORDS.contains(&current) {
                    TokenKind::Keyword.into()
                } else if matches_number(current) {
                    TokenKind::IntegerLiteral.into()
                } else if matches_identifier(current) {
                    TokenKind::Identifier.into()
                } else if is_whitespace(current) {
                    GetTokenResult::Skip()
                } else {
                    GetTokenResult::NoMatch()
                }
            }
        },
        LexerContext::Comment => {
            if NEWLINES.contains(&current) {
                context_stack.pop();
            }
            GetTokenResult::Skip()
        }
        LexerContext::String(is_escaped) => {
            if *is_escaped {
                *is_escaped = false;
                TokenKind::StringLiteral.into()
            } else if current.ends_with(BACKSLASH) {
                *is_escaped = true;
                TokenKind::StringLiteral.into()
            } else if current == DOUBLE_QUOTE {
                // Single double quote will happen when we are at the end of a string
                context_stack.pop();
                TokenKind::DoubleQuote.into()
            } else if current == DOLLAR {
                // Single dollar will happen when we are at the start of a template variable
                context_stack.push(LexerContext::TemplateVariable);
                TokenKind::Dollar.into()
            } else if current.ends_with(DOUBLE_QUOTE) || current.ends_with(DOLLAR) {
                // Return no match if we encounter something indicates the literal should end
                GetTokenResult::NoMatch()
            } else {
                TokenKind::StringLiteral.into()
            }
        }
        LexerContext::Command => {
            if current == DOUBLE_QUOTE {
                context_stack.push(LexerContext::String(false));
                TokenKind::DoubleQuote.into()
            } else if current == BACKTICK {
                context_stack.pop();
                TokenKind::Backtick.into()
            } else if is_whitespace(current) {
                GetTokenResult::Skip()
            } else if current.ends_with(BACKTICK)
                || current.ends_with(DOUBLE_QUOTE)
                || current.ends_with(char::is_whitespace)
            {
                GetTokenResult::NoMatch()
            } else {
                TokenKind::StringLiteral.into()
            }
        }
        LexerContext::TemplateVariable => {
            if current == LEFT_CURLY {
                TokenKind::LeftCurly.into()
            } else if current == RIGHT_CURLY {
                context_stack.pop();
                TokenKind::RightCurly.into()
            } else if matches_identifier(current) {
                TokenKind::Identifier.into()
            } else if is_whitespace(current) {
                GetTokenResult::Skip()
            } else {
                GetTokenResult::NoMatch()
            }
        }
    }
}

// TODO - this needs to be more lenient, basically anything that isn't going to conflict with
// other syntax characters, or keywords should be allowed (emojis, non-latin scripts, e.c.t...
// are fine). Currently grapheme clusters won't really work
fn matches_identifier(value: &str) -> bool {
    return value.chars().all(|x| x.is_alphanumeric() || x == '_');
}

fn matches_number(value: &str) -> bool {
    // Individual checking of chars should be fine here, as all digits will be graphemes of one
    // character anyway
    return value.chars().all(|x| x.is_digit(10));
}

fn is_whitespace(value: &str) -> bool {
    // Checking chars individually is fine here, as any valid unicode whitespace should
    // really be a single character, and not a grapheme cluster
    return value.chars().all(|x| x.is_whitespace()) || is_newline(value);
}

pub fn is_newline(value: &str) -> bool {
    NEWLINES.contains(&value)
}
