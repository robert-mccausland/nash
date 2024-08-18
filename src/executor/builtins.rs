use std::cell::RefCell;

use crate::errors::ExecutionError;

use super::{ExecutorContext, Value};

const READ_BUF_SIZE: usize = 256;

pub fn call_builtin(
    name: &str,
    args: &[Value],
    context: &mut ExecutorContext,
) -> Result<Value, ExecutionError> {
    match (name, args) {
        ("parse_int", [Value::String(arg1)]) => Ok(Value::Integer(parse_int(arg1)?)),
        ("in", []) => r#in(context),
        ("err", [Value::String(arg1)]) => err(context, arg1),
        ("out", [Value::String(arg1)]) => out(context, arg1),
        ("fmt", [arg1]) => fmt(context, arg1),
        ("push", [Value::Array(arg1), arg2]) => push(context, arg1.as_ref(), arg2),
        ("pop", [Value::Array(arg1)]) => pop(context, arg1.as_ref()),
        (name, args) => {
            let args = args
                .iter()
                .map(|arg| format!("{arg}"))
                .reduce(|value, acc| format!("{acc}, {value}"))
                .unwrap_or(String::new());
            return Err(
                format!("No function found with name: '{name}' and arguments: {args}").into(),
            );
        }
    }
}

fn parse_int(value: &str) -> Result<i32, ExecutionError> {
    i32::from_str_radix(value, 10)
        .map_err(|_| format!("Could not parse string {:} into integer", value).into())
}

fn r#in(context: &mut ExecutorContext) -> Result<Value, ExecutionError> {
    let mut buf = vec![0; READ_BUF_SIZE];
    let mut value = String::new();
    loop {
        let size = match context.stdin.read_until(b'\n', &mut buf) {
            Ok(n) => n,
            Err(err) => return Err(format!("Error reading from stdin: {err}").into()),
        };

        let buffer = match std::str::from_utf8(&buf[0..size]) {
            Ok(result) => result,
            Err(err) => {
                return Err(format!("Bytes read from stdin was not valid utf8: {err}").into())
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
        return Err(format!("Error writing to stdout: {err}").into());
    }

    return Ok(Value::Void);
}

fn err(context: &mut ExecutorContext, value: &str) -> Result<Value, ExecutionError> {
    if let Err(err) = writeln!(&mut context.stderr, "{:}", value) {
        return Err(format!("Error writing to stderr: {err}").into());
    }

    return Ok(Value::Void);
}

fn fmt(_: &mut ExecutorContext, value: &Value) -> Result<Value, ExecutionError> {
    return Ok(Value::String(format!("{value:}")));
}

fn push(
    _context: &mut ExecutorContext,
    array: &RefCell<Vec<Value>>,
    value: &Value,
) -> Result<Value, ExecutionError> {
    array
        .try_borrow_mut()
        .map_err::<ExecutionError, _>(|_| {
            format!("Cannot mutate array that is already being used").into()
        })?
        .push(value.clone());
    Ok(Value::Void)
}

fn pop(
    _context: &mut ExecutorContext,
    array: &RefCell<Vec<Value>>,
) -> Result<Value, ExecutionError> {
    Ok(array
        .try_borrow_mut()
        .map_err::<ExecutionError, _>(|_| {
            format!("Cannot mutate array that is already being used").into()
        })?
        .pop()
        .ok_or::<ExecutionError>("Unable to pop array with no elements".into())?)
}
