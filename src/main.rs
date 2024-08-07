use std::{
    env,
    error::Error,
    fs::File,
    io::{stderr, stdin, stdout, BufReader, Read},
    path::PathBuf,
};

use executer::{commands::SystemCommandExecutor, Executor};

mod executer;
mod constants;
mod lexer;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let args = get_args()?;
    let mut file = File::open(args.file_path)?;
    execute_script(&mut file)?;
    Ok(())
}

fn execute_script<R: Read>(file: &mut R) -> Result<(), Box<dyn Error>> {
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

fn get_args() -> Result<Arguments, Box<dyn Error>> {
    let args = env::args().into_iter().collect::<Vec<_>>();
    let file = args.get(1).ok_or("First argument must be path to script")?;
    return Ok(Arguments {
        file_path: PathBuf::from(file),
    });
}
