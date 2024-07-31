use std::{
    env,
    fs::File,
    io::{self, stderr, stdin, stdout, BufReader, Read},
    path::PathBuf,
};

use executer::{commands::SystemCommandExecutor, ExecutionError, Executor};
use lexer::LexerError;
use parser::ParserError;

mod executer;
mod lexer;
mod parser;

fn main() {
    let args = get_args().unwrap();
    let mut file = File::open(args.file_path).unwrap();
    execute_script(&mut file).unwrap();
}

#[derive(Debug)]
enum Error {
    LexerError(LexerError),
    ParserError(ParserError),
    ExecutionError(ExecutionError),
    Io(io::Error),
}

impl From<LexerError> for Error {
    fn from(value: LexerError) -> Self {
        Error::LexerError(value)
    }
}

impl From<ParserError> for Error {
    fn from(value: ParserError) -> Self {
        Error::ParserError(value)
    }
}

impl From<ExecutionError> for Error {
    fn from(value: ExecutionError) -> Self {
        Error::ExecutionError(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}

fn execute_script<R: Read>(file: &mut R) -> Result<(), Error> {
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    let tokens = lexer::lex(string.as_str()).collect::<Result<Vec<_>, _>>()?;
    let syntax_tree_root = parser::parse(tokens)?;
    let mut executor = Executor::new(
        SystemCommandExecutor {},
        BufReader::new(stdin()),
        stdout(),
        stderr(),
    );
    executor.execute(&syntax_tree_root)?;
    return Ok(());
}

struct Arguments {
    file_path: PathBuf,
}

fn get_args() -> Result<Arguments, Box<dyn std::error::Error>> {
    let args = env::args().into_iter().collect::<Vec<_>>();
    let file = args.get(1).ok_or("No file arg provided")?;
    return Ok(Arguments {
        file_path: PathBuf::from(file),
    });
}
