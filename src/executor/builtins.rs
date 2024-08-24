use std::cell::RefCell;

use crate::errors::ExecutionError;

use super::{
    values::{Type, Value},
    ExecutorContext,
};

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
        ("glob", [Value::String(arg1)]) => glob(context, arg1),
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

pub fn call_builtin_instance(
    name: &str,
    instance: &Value,
    args: &[Value],
    context: &mut ExecutorContext,
) -> Result<Value, ExecutionError> {
    match (name, instance, args) {
        ("fmt", instance, []) => fmt(context, instance),
        ("push", Value::Array(instance, array_type), [value]) => {
            if array_type != &value.get_type() {
                return Err(format!(
                    "Can not push a value of type {} to an array with type {}",
                    value.get_type(),
                    array_type,
                )
                .into());
            }
            push(context, instance.as_ref(), value)
        }
        ("pop", Value::Array(instance, _), []) => pop(context, instance.as_ref()),
        ("len", Value::Array(instance, _), []) => len(context, instance.as_ref()),
        (name, instance, args) => {
            let args = args
                .iter()
                .map(|arg| format!("{}", arg.get_type()))
                .reduce(|value, acc| format!("{acc}, {value}"))
                .unwrap_or(String::new());
            let instance_type = instance.get_type();
            return Err(
                format!("No function found with name: {name} on type {instance_type} that accepts arguments {args}").into(),
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

fn len(
    _context: &mut ExecutorContext,
    array: &RefCell<Vec<Value>>,
) -> Result<Value, ExecutionError> {
    Ok(Value::Integer(
        array
            .borrow()
            .len()
            .try_into()
            .map_err::<ExecutionError, _>(|err| {
                format!("Unable to convert array length into i32: {err}").into()
            })?,
    ))
}

fn glob(_context: &mut ExecutorContext, pattern: &str) -> Result<Value, ExecutionError> {
    let paths = glob::glob(pattern)
        .map_err::<ExecutionError, _>(|err| {
            format!("Invalid pattern provided to glob: {err}").into()
        })?
        .map(|path| {
            let path = path
                .map_err::<ExecutionError, _>(|err| {
                    format!("Unable to get path while globing: {err}").into()
                })?
                .into_os_string()
                .into_string()
                .map_err::<ExecutionError, _>(|_| {
                    format!("Path is not in valid utf-8 encoding").into()
                })?;
            return Ok::<String, ExecutionError>(path);
        })
        .collect::<Result<Vec<_>, _>>()?;

    return Ok(Value::new_array(paths, Type::String)?);
}
