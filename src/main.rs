use std::{env, error::Error, fs::File, path::PathBuf, process::ExitCode};

use nash::ExecutorOptions;

fn main() -> ExitCode {
    match main_impl() {
        Ok(code) => code,
        Err(code) => code,
    }
    .into()
}

fn main_impl() -> Result<u8, u8> {
    let args = get_args().map_err(|err| {
        eprintln!("Invalid arguments: {err}");
        100
    })?;

    let mut file = File::open(args.file_path).map_err(|err| {
        eprintln!("Error reading file path: {err}");
        100
    })?;

    let mut executor = nash::SystemExecutor::new(ExecutorOptions::default());
    let result = nash::execute(&mut file, &mut executor).map_err(|err| {
        eprintln!("Error running nash script: {err}");
        err.exit_code()
    })?;

    return Ok(result.exit_code());
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
