use std::io::Read;

use errors::{NashError, ParserError};

mod components;
mod constants;
mod errors;
mod executor;
mod lexer;
mod parser;
mod utils;

pub use executor::commands::{Command, CommandExecutor, CommandResult, SystemCommandExecutor};
pub use executor::Executor;

pub fn execute<R: Read>(script: &mut R, executor: &mut Executor) -> Result<(), NashError> {
    let mut content = String::new();
    script
        .read_to_string(&mut content)
        .map_err(|err| format!("Unable to read script: {err}"))?;

    let tokens = lexer::lex(content.as_str()).collect::<Result<Vec<_>, _>>()?;

    let root = match parser::parse(tokens) {
        Ok(root) => root,
        Err(err) => {
            println!("{}", format_error(&err, &content));
            return Err(err.into());
        }
    };

    executor.execute(&root)?;

    return Ok(());
}

fn format_error(error: &ParserError, source_file: &str) -> String {
    let mut result = String::new();
    if let Some(start) = &error.start {
        if let Some(end) = &error.end {
            let mut line = String::new();
            let mut underline_start = 0;

            for (index, char) in source_file.chars().enumerate() {
                if char == '\n' {
                    if index >= *end {
                        break;
                    }
                    line = String::new();
                    continue;
                }

                line += &String::from(char);
                underline_start = start - index;
            }

            let underline = " ".repeat(underline_start) + &"^".repeat(end - start);
            result += &line;
            result += &underline;
        }
    }
    return result;
}
