use std::fmt::{Error, Write};
use std::io::Read;

pub use errors::{ExecutionError, LexerError, NashError, ParserError};
pub use executor::commands::{
    CommandDefinition, CommandExecutor, CommandOutput, Pipeline, PipelineDestination,
    PipelineOutput, PipelineSource,
};
pub use executor::ExecutionOutput;
pub use executor::{Executor, ExecutorOptions};

mod components;
mod constants;
mod errors;
mod executor;
mod lexer;
mod parser;
mod utils;

pub fn execute<R: Read>(
    script: &mut R,
    executor: &mut Executor,
) -> Result<ExecutionOutput, NashError> {
    let mut content = String::new();
    script
        .read_to_string(&mut content)
        .map_err(|err| format!("Unable to read script: {err}"))?;

    let tokens = lexer::lex(content.as_str())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            eprintln!("Error parsing script:");
            eprintln!("{}", err.message);
            if let Some(position) = err.position {
                eprintln!("At position {}", position);
            }
            return err;
        })?;

    let root = parser::parse(tokens).map_err(|err| {
        eprintln!("Error parsing script:");
        eprintln!(
            "{}",
            format_error(&err, &content).expect("Unable to write error information")
        );
        return err;
    })?;

    let result = executor.execute(&root).map_err(|err| {
        eprintln!("Error executing script: {err}");
        if let Some(call_stack) = &err.call_stack {
            let formatted_stack = call_stack
                .into_iter()
                .fold("@root".to_owned(), |a, b| format!("{b}\n{a}"));
            eprintln!("Call stack: \n{formatted_stack}");
        }

        return err;
    })?;

    return Ok(result);
}

fn format_error(error: &ParserError, source_file: &str) -> Result<String, Error> {
    let mut result = String::new();

    if let Some(start) = &error.start {
        if let Some(end) = &error.end {
            writeln!(
                result,
                "Unexpected token: {:} at index {:}",
                error.token, start
            )?;

            let mut line = String::new();
            let mut underline_start = 0;

            for (index, char) in source_file.chars().enumerate() {
                if char == '\n' {
                    if index >= *end {
                        break;
                    }
                    line = String::new();
                    underline_start = start - index - 1;
                    continue;
                }

                line += &String::from(char);
            }

            let underline = " ".repeat(underline_start) + &"^".repeat(end - start);
            writeln!(result, "{line}")?;
            writeln!(result, "{underline}")?;
        }
    }

    writeln!(result, "{}", error.message)?;

    return Ok(result);
}
