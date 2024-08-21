use crate::constants::KEYWORDS;

use super::{LexerContext, TokenValue};

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
const DOUBLE_QUOTE: &str = "\"";
const ESCAPED_DOUBLE_QUOTE: &str = "\\\"";
const ESCAPED_DOLLAR: &str = "\\$";
const BACKTICK: &str = "`";
const ESCAPED_BACKTICK: &str = "\\`";
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
    Ignored,
}

impl TokenKind {
    pub fn into_token<'a>(&self, value: &'a str) -> Option<TokenValue<'a>> {
        match self {
            TokenKind::StringLiteral => Some(TokenValue::StringLiteral(value)),
            TokenKind::IntegerLiteral => Some(TokenValue::IntegerLiteral(value)),
            TokenKind::Identifier => Some(TokenValue::Identifier(value)),
            TokenKind::Keyword => Some(TokenValue::Keyword(value)),
            TokenKind::Equals => Some(TokenValue::Equals()),
            TokenKind::Plus => Some(TokenValue::Plus()),
            TokenKind::LeftBracket => Some(TokenValue::LeftBracket()),
            TokenKind::RightBracket => Some(TokenValue::RightBracket()),
            TokenKind::LeftCurly => Some(TokenValue::LeftCurly()),
            TokenKind::RightCurly => Some(TokenValue::RightCurly()),
            TokenKind::LeftAngle => Some(TokenValue::LeftAngle()),
            TokenKind::RightAngle => Some(TokenValue::RightAngle()),
            TokenKind::LeftSquare => Some(TokenValue::LeftSquare()),
            TokenKind::RightSquare => Some(TokenValue::RightSquare()),
            TokenKind::Question => Some(TokenValue::Question()),
            TokenKind::Dot => Some(TokenValue::Dot()),
            TokenKind::Colon => Some(TokenValue::Colon()),
            TokenKind::Semicolon => Some(TokenValue::Semicolon()),
            TokenKind::Comma => Some(TokenValue::Comma()),
            TokenKind::Bang => Some(TokenValue::Bang()),
            TokenKind::DoubleQuote => Some(TokenValue::DoubleQuote()),
            TokenKind::Dollar => Some(TokenValue::Dollar()),
            TokenKind::Backtick => Some(TokenValue::Backtick()),
            TokenKind::Ignored => None,
        }
    }
}

pub fn try_get_token_kind(
    context_stack: &mut Vec<LexerContext>,
    current: &str,
) -> Option<TokenKind> {
    match context_stack.last().unwrap() {
        LexerContext::Root => match current {
            HASH => {
                context_stack.push(LexerContext::Comment);
                Some(TokenKind::Ignored)
            }
            DOUBLE_QUOTE => {
                context_stack.push(LexerContext::String);
                Some(TokenKind::DoubleQuote)
            }
            BACKTICK => {
                context_stack.push(LexerContext::Command);
                Some(TokenKind::Backtick)
            }
            EQUALS => Some(TokenKind::Equals),
            PLUS => Some(TokenKind::Plus),
            LEFT_BRACKET => Some(TokenKind::LeftBracket),
            RIGHT_BRACKET => Some(TokenKind::RightBracket),
            LEFT_CURLY => Some(TokenKind::LeftCurly),
            RIGHT_CURLY => Some(TokenKind::RightCurly),
            LEFT_ANGLE => Some(TokenKind::LeftAngle),
            RIGHT_ANGLE => Some(TokenKind::RightAngle),
            LEFT_SQUARE => Some(TokenKind::LeftSquare),
            RIGHT_SQUARE => Some(TokenKind::RightSquare),
            QUESTION => Some(TokenKind::Question),
            DOT => Some(TokenKind::Dot),
            COLON => Some(TokenKind::Colon),
            SEMICOLON => Some(TokenKind::Semicolon),
            COMMA => Some(TokenKind::Comma),
            BANG => Some(TokenKind::Bang),
            _ => {
                if KEYWORDS.contains(&current) {
                    Some(TokenKind::Keyword)
                } else if matches_number(current) {
                    Some(TokenKind::IntegerLiteral)
                } else if matches_identifier(current) {
                    Some(TokenKind::Identifier)
                } else if is_whitespace(current) {
                    Some(TokenKind::Ignored)
                } else {
                    None
                }
            }
        },
        LexerContext::Comment => {
            if NEWLINES.contains(&current) {
                context_stack.pop();
                Some(TokenKind::Ignored)
            } else if current.ends_with(NEWLINES[0])
                || current.ends_with(NEWLINES[1])
                || current.starts_with(HASH)
            {
                None
            } else {
                Some(TokenKind::Ignored)
            }
        }
        LexerContext::String => {
            if current == DOUBLE_QUOTE {
                // Single double quote will happen when we are at the end of a string
                context_stack.pop();
                Some(TokenKind::DoubleQuote)
            } else if current == DOLLAR {
                // Single dollar will happen when we are at the start of a template variable
                context_stack.push(LexerContext::TemplateVariable);
                Some(TokenKind::Dollar)
            } else if (current.ends_with(DOUBLE_QUOTE) && !current.ends_with(ESCAPED_DOUBLE_QUOTE))
                || (current.ends_with(DOLLAR) && !current.ends_with(ESCAPED_DOLLAR))
                || current.starts_with(DOUBLE_QUOTE)
                || current.starts_with(RIGHT_CURLY)
            {
                // Need to detect all the special cases where we have return the currently matched string
                // Ending with a double quote is the end of the string
                // Ending with a dollar is the start of a template variable
                // Starting with a double quote is the start of the string
                // Starting with a right curly is the end of a template variable
                None
            } else {
                Some(TokenKind::StringLiteral)
            }
        }
        LexerContext::Command => {
            if current == BACKTICK {
                context_stack.pop();
                Some(TokenKind::Backtick)
            } else if current == DOUBLE_QUOTE {
                // Single double quote will happen when we are at the start of a quoted string
                context_stack.push(LexerContext::String);
                Some(TokenKind::DoubleQuote)
            } else if is_whitespace(current) {
                Some(TokenKind::Ignored)
            } else if (current.ends_with(BACKTICK) && !current.ends_with(ESCAPED_BACKTICK))
                || (current.ends_with(DOUBLE_QUOTE) && !current.ends_with(ESCAPED_DOUBLE_QUOTE))
                || current.starts_with(BACKTICK)
                || current.starts_with(DOUBLE_QUOTE)
                || current.contains(char::is_whitespace)
            {
                // Need to detect all the special cases where we have return the currently matched string
                // Ending with a backtick means we are at the end of the command
                // Ending with a double quote means we are at the start of a quoted string
                // Whitespace meaning that we are in between strings
                // Starting with a backtick is the start of the command
                // Starting with a double quote is the end of a quoted string
                None
            } else {
                Some(TokenKind::StringLiteral)
            }
        }
        LexerContext::TemplateVariable => {
            if current == LEFT_CURLY {
                Some(TokenKind::LeftCurly)
            } else if current == RIGHT_CURLY {
                context_stack.pop();
                Some(TokenKind::RightCurly)
            } else if matches_identifier(current) {
                Some(TokenKind::Identifier)
            } else {
                None
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

#[cfg(test)]
mod tests {
    use crate::constants::FUNC;

    use super::*;

    macro_rules! test_case {
        ($name:ident, ($value:expr, $context:expr), ($expected_token_type:pat)) => {
            test_case_impl!($name, $value, $context, $expected_token_type, $context);
        };
        ($name:ident, ($value:expr, $context:expr), ($expected_token_type:pat, $expected_context:expr)) => {
            test_case_impl!(
                $name,
                $value,
                $context,
                $expected_token_type,
                $expected_context
            );
        };
    }

    macro_rules! test_case_impl {
        ($name:ident ,$value:expr, $context:expr, $expected_token_type:pat, $expected_context:expr) => {
            #[test]
            fn $name() {
                let mut context = $context;
                let result = try_get_token_kind(&mut context, $value);

                assert!(matches!(result, $expected_token_type));
                assert_eq!(context, $expected_context);
            }
        };
    }

    test_case!(
        should_parse_identifier,
        ("test_identifier", vec![LexerContext::Root]),
        (Some(TokenKind::Identifier))
    );

    test_case!(
        should_parse_keyword,
        (FUNC, vec![LexerContext::Root]),
        (Some(TokenKind::Keyword))
    );

    test_case!(
        should_parse_number,
        ("123", vec![LexerContext::Root]),
        (Some(TokenKind::IntegerLiteral))
    );

    test_case!(
        should_parse_whitespace,
        (" ", vec![LexerContext::Root]),
        (Some(TokenKind::Ignored))
    );

    test_case!(
        should_parse_comment_start,
        ("#", vec![LexerContext::Root]),
        (
            Some(TokenKind::Ignored),
            vec![LexerContext::Root, LexerContext::Comment]
        )
    );

    test_case!(
        should_parse_comment,
        (
            "this is a comment!",
            vec![LexerContext::Root, LexerContext::Comment]
        ),
        (Some(TokenKind::Ignored))
    );

    test_case!(
        should_terminate_comment,
        (
            "this is a comment!\n",
            vec![LexerContext::Root, LexerContext::Comment]
        ),
        (None)
    );

    test_case!(
        should_parse_comment_end,
        ("\n", vec![LexerContext::Root, LexerContext::Comment]),
        (Some(TokenKind::Ignored), vec![LexerContext::Root])
    );

    test_case!(
        should_parse_string_start,
        ("\"", vec![LexerContext::Root]),
        (
            Some(TokenKind::DoubleQuote),
            vec![LexerContext::Root, LexerContext::String]
        )
    );

    test_case!(
        should_parse_string,
        (
            "I'm a string",
            vec![LexerContext::Root, LexerContext::String]
        ),
        (Some(TokenKind::StringLiteral))
    );

    test_case!(
        should_parse_escaped_string,
        (
            "I'm a string \\\"",
            vec![LexerContext::Root, LexerContext::String]
        ),
        (Some(TokenKind::StringLiteral))
    );

    test_case!(
        should_terminate_string,
        (
            "I'm the end of a string\"",
            vec![LexerContext::Root, LexerContext::String]
        ),
        (None)
    );

    test_case!(
        should_parse_string_end,
        ("\"", vec![LexerContext::Root, LexerContext::String]),
        (Some(TokenKind::DoubleQuote), vec![LexerContext::Root])
    );

    test_case!(
        should_parse_command_start,
        ("`", vec![LexerContext::Root]),
        (
            Some(TokenKind::Backtick),
            vec![LexerContext::Root, LexerContext::Command]
        )
    );

    test_case!(
        should_parse_command,
        ("command", vec![LexerContext::Root, LexerContext::Command]),
        (Some(TokenKind::StringLiteral))
    );

    test_case!(
        should_parse_escaped_command,
        (
            "command\\`",
            vec![LexerContext::Root, LexerContext::Command]
        ),
        (Some(TokenKind::StringLiteral))
    );

    test_case!(
        should_terminate_command,
        (
            "command_end`",
            vec![LexerContext::Root, LexerContext::Command]
        ),
        (None)
    );

    test_case!(
        should_parse_command_end,
        ("`", vec![LexerContext::Root, LexerContext::Command]),
        (Some(TokenKind::Backtick), vec![LexerContext::Root])
    );

    test_case!(
        should_not_start_quoted_string_in_command_if_escaped,
        (
            "command\\\"",
            vec![LexerContext::Root, LexerContext::Command]
        ),
        (Some(TokenKind::StringLiteral))
    );

    test_case!(
        should_start_quoted_string_in_command,
        ("command\"", vec![LexerContext::Root, LexerContext::Command]),
        (None)
    );

    test_case!(
        should_end_quoted_string_in_command,
        ("\"", vec![LexerContext::Root, LexerContext::Command]),
        (
            Some(TokenKind::DoubleQuote),
            vec![
                LexerContext::Root,
                LexerContext::Command,
                LexerContext::String
            ]
        )
    );

    test_case!(
        should_being_next_string_in_command,
        ("\"a", vec![LexerContext::Root, LexerContext::Command]),
        (None)
    );

    test_case!(
        should_split_strings_in_command_from_end,
        ("command ", vec![LexerContext::Root, LexerContext::Command]),
        (None)
    );

    test_case!(
        should_split_strings_in_command_from_start,
        (" command", vec![LexerContext::Root, LexerContext::Command]),
        (None)
    );

    test_case!(
        should_parse_string_end_with_template,
        (
            "test_string$",
            vec![LexerContext::Root, LexerContext::String]
        ),
        (None)
    );

    test_case!(
        should_parse_string_resume_with_template,
        ("}a", vec![LexerContext::Root, LexerContext::String]),
        (None)
    );

    test_case!(
        should_parse_template_start,
        ("$", vec![LexerContext::Root, LexerContext::String]),
        (
            Some(TokenKind::Dollar),
            vec![
                LexerContext::Root,
                LexerContext::String,
                LexerContext::TemplateVariable
            ]
        )
    );

    test_case!(
        should_parse_template_braces,
        (
            "{",
            vec![
                LexerContext::Root,
                LexerContext::String,
                LexerContext::TemplateVariable
            ]
        ),
        (Some(TokenKind::LeftCurly))
    );

    test_case!(
        should_parse_template_variable,
        (
            "var_name",
            vec![
                LexerContext::Root,
                LexerContext::String,
                LexerContext::TemplateVariable
            ]
        ),
        (Some(TokenKind::Identifier))
    );

    test_case!(
        should_parse_template_variable_end,
        (
            "}",
            vec![
                LexerContext::Root,
                LexerContext::String,
                LexerContext::TemplateVariable
            ]
        ),
        (
            Some(TokenKind::RightCurly),
            vec![LexerContext::Root, LexerContext::String]
        )
    );

    test_case!(
        should_parse_question,
        ("?", vec![LexerContext::Root]),
        (Some(TokenKind::Question))
    );

    test_case!(
        should_parse_dot,
        (".", vec![LexerContext::Root]),
        (Some(TokenKind::Dot))
    );

    test_case!(
        should_parse_left_square,
        ("[", vec![LexerContext::Root]),
        (Some(TokenKind::LeftSquare))
    );

    test_case!(
        should_parse_right_square,
        ("]", vec![LexerContext::Root]),
        (Some(TokenKind::RightSquare))
    );

    test_case!(
        should_parse_colon,
        (":", vec![LexerContext::Root]),
        (Some(TokenKind::Colon))
    );

    test_case!(
        should_parse_bang,
        ("!", vec![LexerContext::Root]),
        (Some(TokenKind::Bang))
    );
}
