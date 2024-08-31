use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    process::{self, Child, ChildStdout, Command, Stdio},
};

use super::commands::{
    CommandDefinition, CommandOutput, Pipeline, PipelineDestination, PipelineOutput, PipelineSource,
};

enum InputType {
    Null(),
    Literal(String),
    File(File),
    ChildStdout(ChildStdout),
}

impl InputType {
    fn write_to_command(self, command: &mut Command) -> ProcessCallback {
        let mut callback_data = None;
        let stdin = match self {
            InputType::Null() => Stdio::null(),
            InputType::Literal(value) => {
                callback_data = Some(value);
                Stdio::piped()
            }
            InputType::File(file) => Stdio::from(file),
            InputType::ChildStdout(stdout) => Stdio::from(stdout),
        };

        command.stdin(stdin);

        return ProcessCallback {
            data: callback_data,
        };
    }

    fn write_to_destination(self, destination: &PipelineDestination) -> io::Result<()> {
        match self {
            InputType::Null() => {}
            InputType::Literal(value) => {
                std::io::copy(
                    &mut &value.into_bytes()[..],
                    &mut destination_to_file(destination)?,
                )?;
            }
            InputType::File(mut source_file) => {
                std::io::copy(&mut source_file, &mut destination_to_file(destination)?)?;
            }
            InputType::ChildStdout(_) => {
                panic!("Process output should be directly sent to destination, instead of using a pipe")
            }
        }

        Ok(())
    }

    fn write_to_string(self) -> io::Result<String> {
        let mut buf = String::new();
        match self {
            InputType::Null() => {}
            InputType::Literal(value) => {
                buf = value;
            }
            InputType::File(mut file) => {
                file.read_to_string(&mut buf)?;
            }
            InputType::ChildStdout(mut stdout) => {
                stdout.read_to_string(&mut buf)?;
            }
        }

        Ok(buf)
    }
}

struct ProcessCallback {
    data: Option<String>,
}

impl ProcessCallback {
    fn write_to_process(self, process: &mut Child) -> io::Result<()> {
        if let Some(data) = self.data {
            std::io::copy(
                &mut &data.into_bytes()[..],
                &mut process.stdin.take().unwrap(),
            )?;
        };

        Ok(())
    }
}

enum OutputType {
    File(File),
    Pipe(),
}

pub fn run_pipeline(pipeline: &Pipeline) -> io::Result<PipelineOutput> {
    let (mut processes, final_output) = spawn_processes(pipeline)?;

    let stdout = if let Some(destination) = &pipeline.destination {
        final_output.write_to_destination(destination)?;
        None
    } else {
        Some(final_output.write_to_string()?)
    };

    let mut outputs = Vec::new();
    for process in &mut processes {
        let mut stderr_data = None;
        if let Some(mut stderr) = process.stderr.take() {
            let mut buffer = String::new();
            stderr.read_to_string(&mut buffer)?;
            stderr_data = Some(buffer);
        }

        let status = process.wait()?;
        let status_code = status
            .code()
            .ok_or(io::Error::other("Unable to get exit code for command"))?
            .try_into()
            .map_err(|_| io::Error::other("Exit code was not between 0 and 255"))?;

        outputs.push(CommandOutput::new(status_code, stderr_data));
    }

    return Ok(PipelineOutput::new(stdout, outputs));
}

fn spawn_processes(pipeline: &Pipeline) -> io::Result<(Vec<Child>, InputType)> {
    let mut processes = Vec::new();
    let mut input = get_input_type(pipeline)?;
    let mut command_definitions = pipeline.commands.iter().peekable();
    while let Some(command_definition) = command_definitions.next() {
        // If we are the last command we might need to output to a file or whatever which we
        // need to setup when we are spawning the process
        let output = if command_definitions.peek().is_some() {
            OutputType::Pipe()
        } else {
            get_output_type(pipeline)?
        };

        let mut process = spawn_process(command_definition, input, output)?;

        if let Some(stdout) = process.stdout.take() {
            input = InputType::ChildStdout(stdout);
        } else {
            input = InputType::Null();
        }

        processes.push(process);
    }

    return Ok((processes, input));
}

fn spawn_process(
    definition: &CommandDefinition,
    input: InputType,
    output: OutputType,
) -> io::Result<Child> {
    let mut command = process::Command::new(definition.program.to_owned());
    command.args(definition.arguments.to_owned());
    let process_callback = input.write_to_command(&mut command);

    // Stdout depends on what kind of output we need to provide
    command.stdout(match output {
        OutputType::File(file) => Stdio::from(file),
        OutputType::Pipe() => Stdio::piped(),
    });

    if definition.capture_stderr {
        command.stderr(Stdio::piped());
    }

    let mut process = command.spawn()?;
    process_callback.write_to_process(&mut process)?;

    return Ok(process);
}

fn get_input_type(pipeline: &Pipeline) -> io::Result<InputType> {
    Ok(if let Some(source) = &pipeline.source {
        match source {
            PipelineSource::File(file) => InputType::File(File::open(file)?),
            PipelineSource::Literal(literal) => InputType::Literal(literal.to_owned()),
        }
    } else {
        InputType::Null()
    })
}

fn get_output_type(pipeline: &Pipeline) -> io::Result<OutputType> {
    Ok(match &pipeline.destination {
        Some(destination) => OutputType::File(destination_to_file(destination)?),
        None => OutputType::Pipe(),
    })
}

fn destination_to_file(destination: &PipelineDestination) -> io::Result<File> {
    Ok(match destination {
        PipelineDestination::FileWrite(path) => File::create(path)?,
        PipelineDestination::FileAppend(path) => OpenOptions::new().append(true).open(path)?,
    })
}
