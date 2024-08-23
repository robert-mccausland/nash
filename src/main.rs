use std::{env, error::Error, fs::File, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args = get_args()?;
    let mut file = File::open(args.file_path)?;

    let mut executor = nash::Executor::default();
    nash::execute(&mut file, &mut executor)?;

    Ok(())
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
