use serde::Serialize;

use crate::constants::KEYWORDS;

use super::LexerContext;

const SEMICOLON: &str = ";";
const COMMA: &str = ",";
const HASH: &str = "#";
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
const DOLLAR: &str = "$";
const BANG: &str = "!";
const BACKSLASH: &str = "\\";
const DOUBLE_QUOTE: &str = "\"";
const BACKTICK: &str = "`";
const NEWLINES: [&str; 2] = ["\n", "\r\n"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    StringLiteral,
    IntegerLiteral,
    Identifier,
    Keyword,
    Equals,
    Plus,
    LeftBracket,
    RightBracket,
    LeftCurly,
    RightCurly,
    LeftAngle,
    RightAngle,
    LeftSquare,
    RightSquare,
    Question,
    Dot,
    Colon,
    Semicolon,
    Comma,
    Bang,
    DoubleQuote,
    Dollar,
    Backtick,
}

impl TokenKind {
    pub fn into_token<'a>(&self, value: &'a str) -> TokenValue<'a> {
        match self {
            TokenKind::StringLiteral => TokenValue::StringLiteral(value),
            TokenKind::IntegerLiteral => TokenValue::IntegerLiteral(value),
            TokenKind::Identifier => TokenValue::Identifier(value),
            TokenKind::Keyword => TokenValue::Keyword(value),
            TokenKind::Equals => TokenValue::Equals(),
            TokenKind::Plus => TokenValue::Plus(),
            TokenKind::LeftBracket => TokenValue::LeftBracket(),
            TokenKind::RightBracket => TokenValue::RightBracket(),
            TokenKind::LeftCurly => TokenValue::LeftCurly(),
            TokenKind::RightCurly => TokenValue::RightCurly(),
            TokenKind::LeftAngle => TokenValue::LeftAngle(),
            TokenKind::RightAngle => TokenValue::RightAngle(),
            TokenKind::LeftSquare => TokenValue::LeftSquare(),
            TokenKind::RightSquare => TokenValue::RightSquare(),
            TokenKind::Question => TokenValue::Question(),
            TokenKind::Dot => TokenValue::Dot(),
            TokenKind::Colon => TokenValue::Colon(),
            TokenKind::Semicolon => TokenValue::Semicolon(),
            TokenKind::Comma => TokenValue::Comma(),
            TokenKind::Bang => TokenValue::Bang(),
            TokenKind::DoubleQuote => TokenValue::DoubleQuote(),
            TokenKind::Dollar => TokenValue::Dollar(),
            TokenKind::Backtick => TokenValue::Backtick(),
        }
    }

    pub fn is_greedy(&self) -> bool {
        match self {
            TokenKind::StringLiteral => true,
            TokenKind::IntegerLiteral => true,
            TokenKind::Identifier => true,
            TokenKind::Keyword => true,
            _ => false,
        }
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

pub enum GetTokenResult {
    // The current string matched something and may take extra characters
    Match(TokenKind),

    // The current string couldn't match anything
    NoMatch(),

    // The current string should be skipped over
    Skip(),
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
                GetTokenResult::Match(TokenKind::DoubleQuote)
            }
            BACKTICK => {
                context_stack.push(LexerContext::Command);
                GetTokenResult::Match(TokenKind::Backtick)
            }
            EQUALS => GetTokenResult::Match(TokenKind::Equals),
            PLUS => GetTokenResult::Match(TokenKind::Plus),
            LEFT_BRACKET => GetTokenResult::Match(TokenKind::LeftBracket),
            RIGHT_BRACKET => GetTokenResult::Match(TokenKind::RightBracket),
            LEFT_CURLY => GetTokenResult::Match(TokenKind::LeftCurly),
            RIGHT_CURLY => GetTokenResult::Match(TokenKind::RightCurly),
            LEFT_ANGLE => GetTokenResult::Match(TokenKind::LeftAngle),
            RIGHT_ANGLE => GetTokenResult::Match(TokenKind::RightAngle),
            LEFT_SQUARE => GetTokenResult::Match(TokenKind::LeftSquare),
            RIGHT_SQUARE => GetTokenResult::Match(TokenKind::RightSquare),
            QUESTION => GetTokenResult::Match(TokenKind::Question),
            DOT => GetTokenResult::Match(TokenKind::Dot),
            COLON => GetTokenResult::Match(TokenKind::Colon),
            SEMICOLON => GetTokenResult::Match(TokenKind::Semicolon),
            COMMA => GetTokenResult::Match(TokenKind::Comma),
            BANG => GetTokenResult::Match(TokenKind::Bang),
            _ => {
                if KEYWORDS.contains(&current) {
                    GetTokenResult::Match(TokenKind::Keyword)
                } else if matches_number(current) {
                    GetTokenResult::Match(TokenKind::IntegerLiteral)
                } else if matches_identifier(current) {
                    GetTokenResult::Match(TokenKind::Identifier)
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
                GetTokenResult::Match(TokenKind::StringLiteral)
            } else if current.ends_with(BACKSLASH) {
                *is_escaped = true;
                GetTokenResult::Match(TokenKind::StringLiteral)
            } else if current == DOUBLE_QUOTE {
                // Single double quote will happen when we are at the end of a string
                context_stack.pop();
                GetTokenResult::Match(TokenKind::DoubleQuote)
            } else if current == DOLLAR {
                // Single dollar will happen when we are at the start of a template variable
                context_stack.push(LexerContext::TemplateVariable);
                GetTokenResult::Match(TokenKind::Dollar)
            } else if current.ends_with(DOUBLE_QUOTE) || current.ends_with(DOLLAR) {
                // Return no match if we encounter something indicates the literal should end
                GetTokenResult::NoMatch()
            } else {
                GetTokenResult::Match(TokenKind::StringLiteral)
            }
        }
        LexerContext::Command => {
            if current == DOUBLE_QUOTE {
                context_stack.push(LexerContext::String(false));
                GetTokenResult::Match(TokenKind::DoubleQuote)
            } else if current == BACKTICK {
                context_stack.pop();
                GetTokenResult::Match(TokenKind::Backtick)
            } else if is_whitespace(current) {
                GetTokenResult::Skip()
            } else if current.ends_with(BACKTICK)
                || current.ends_with(DOUBLE_QUOTE)
                || current.ends_with(char::is_whitespace)
            {
                GetTokenResult::NoMatch()
            } else {
                GetTokenResult::Match(TokenKind::StringLiteral)
            }
        }
        LexerContext::TemplateVariable => {
            if current == LEFT_CURLY {
                GetTokenResult::Match(TokenKind::LeftCurly)
            } else if current == RIGHT_CURLY {
                context_stack.pop();
                GetTokenResult::Match(TokenKind::RightCurly)
            } else if matches_identifier(current) {
                GetTokenResult::Match(TokenKind::Identifier)
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
