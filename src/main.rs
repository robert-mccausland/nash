use std::{
    env,
    error::Error,
    fs::File,
    io::{stderr, stdin, stdout, BufReader, Read},
    path::PathBuf,
};

use components::errors::ParserError;
use executor::commands::SystemCommandExecutor;

mod components;
mod constants;
mod executor;
mod lexer;
mod parser;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    let args = get_args()?;
    let mut file = File::open(args.file_path)?;
    execute_script(&mut file)?;
    Ok(())
}

fn execute_script<R: Read>(file: &mut R) -> Result<(), Box<dyn Error>> {
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let tokens = lexer::lex(content.as_str()).collect::<Result<Vec<_>, _>>()?;

    let root = match parser::parse(tokens.iter()) {
        Ok(root) => root,
        Err(err) => {
            println!("{}", format_error(&err, &content));
            return Err(err.into());
        }
    };

    let mut executor = executor::Executor::new(
        SystemCommandExecutor::new(),
        BufReader::new(stdin()),
        stdout(),
        stderr(),
    );

    executor.execute(&root)?;

    return Ok(());
}

struct Arguments {
    file_path: PathBuf,
}

fn get_args() -> Result<Arguments, Box<dyn Error>> {
    let args = env::args().into_iter().collect::<Vec<_>>();
    let file = args.get(1).ok_or("First argument must be path to script")?;
    return Ok(Arguments {
        file_path: PathBuf::from(file),
    });
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
