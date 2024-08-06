use crate::executer::{ExecutionError, Value};

use super::ExecutorContext;

const READ_BUF_SIZE: usize = 256;

pub fn call_builtin(
    name: &str,
    args: &[Value],
    context: &mut ExecutorContext,
) -> Result<Value, ExecutionError> {
    match (name, args) {
        ("parse_int", [Value::String(arg1)]) => Ok(Value::Integer(parse_int(arg1)?)),
        ("in", []) => r#in(context),
        ("out", [Value::String(arg1)]) => out(context, arg1),
        ("out", [Value::ExitCode(arg1)]) => out(context, &format!("{:}", arg1)),
        ("err", [Value::String(arg1)]) => err(context, arg1),
        _ => Err(ExecutionError::new(
            "No function found with given name and arguments".to_owned(),
        )),
    }
}

fn parse_int(value: &str) -> Result<i32, ExecutionError> {
    i32::from_str_radix(value, 10)
        .map_err(|_| ExecutionError::new(format!("Could not parse string {:} into integer", value)))
}

fn r#in(context: &mut ExecutorContext) -> Result<Value, ExecutionError> {
    let mut buf = vec![0; READ_BUF_SIZE];
    let mut value = String::new();
    loop {
        let size = match context.stdin.read_until(b'\n', &mut buf) {
            Ok(n) => n,
            Err(err) => {
                return Err(ExecutionError::new(format!(
                    "Error reading from stdin: {err}"
                )))
            }
        };

        let buffer = match std::str::from_utf8(&buf[0..size]) {
            Ok(result) => result,
            Err(err) => {
                return Err(ExecutionError::new(format!(
                    "Bytes read from stdin was not valid utf8: {err}"
                )))
            }
        };

        value.push_str(buffer);

        if size < READ_BUF_SIZE {
            return Ok(Value::String(value));
        }
    }
}

fn out(context: &mut ExecutorContext, value: &str) -> Result<Value, ExecutionError> {
    if let Err(err) = writeln!(&mut context.stdout, "{:}", value) {
        return Err(ExecutionError::new(format!(
            "Error writing to stdout: {err}"
        )));
    }

    if let Err(err) = context.stdout.flush() {
        return Err(ExecutionError::new(format!(
            "Error writing to stdout: {err}"
        )));
    }

    return Ok(Value::Void);
}

fn err(context: &mut ExecutorContext, value: &str) -> Result<Value, ExecutionError> {
    if let Err(err) = writeln!(&mut context.stderr, "{:}", value) {
        return Err(ExecutionError::new(format!(
            "Error writing to stderr: {err}"
        )));
    }

    if let Err(err) = context.stderr.flush() {
        return Err(ExecutionError::new(format!(
            "Error writing to stderr: {err}"
        )));
    }

    return Ok(Value::Void);
}
