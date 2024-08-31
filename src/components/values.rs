use std::{cell::RefCell, fmt::Display, rc::Rc};

use serde::Serialize;

use crate::{utils::formatting::fmt_collection, ExecutionError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Value {
    #[default]
    Void,
    String(String),
    Integer(i32),
    Boolean(bool),
    Command(String, Vec<String>),
    Array(Rc<RefCell<Vec<Value>>>, Type),
    Tuple(Vec<Value>),
    FileHandle(String, FileMode),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileMode {
    Open,
    Write,
    Append,
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Value::Void => Type::Void,
            Value::String(_) => Type::String,
            Value::Integer(_) => Type::Integer,
            Value::Boolean(_) => Type::Boolean,
            Value::Command(_, _) => Type::Command,
            Value::Array(_, value_type) => Type::Array(value_type.clone().into()),
            Value::Tuple(values) => {
                Type::Tuple(values.iter().map(|x| x.get_type()).collect::<Vec<_>>())
            }
            Value::FileHandle(_, _) => Type::FileHandle,
        }
    }

    pub fn new_array<I: IntoIterator<Item = T>, T: Into<Value>>(
        values: I,
        array_type: Type,
    ) -> Result<Value, ExecutionError> {
        let values = values
            .into_iter()
            .map(|value| {
                let value = value.into();
                if value.get_type() != array_type {
                    Err("Array item did not match array type")
                } else {
                    Ok(value)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self::Array(Rc::new(RefCell::new(values)), array_type))
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Void => f.write_str("void")?,
            Value::String(data) => {
                f.write_str("\"")?;
                f.write_str(&data.replace("\"", "\\\""))?;
                f.write_str("\"")?;
            }
            Value::Integer(data) => data.fmt(f)?,
            Value::Boolean(data) => data.fmt(f)?,
            Value::Command(program, arguments) => {
                let combined = Some(program).into_iter().chain(arguments.iter());
                fmt_collection("`", " ", "`", combined, f)?
            }
            Value::Array(data, _) => fmt_collection("[", ",", "]", data.borrow().iter(), f)?,
            Value::Tuple(data) => fmt_collection("(", ",", ")", data.iter(), f)?,
            Value::FileHandle(_, _) => f.write_str("file_handle")?,
        };

        return Ok(());
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Integer(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Type {
    Void,
    String,
    Integer,
    Boolean,
    Command,
    Array(Box<Type>),
    Tuple(Vec<Type>),
    FileHandle,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => f.write_str("void"),
            Type::String => f.write_str("string"),
            Type::Integer => f.write_str("integer"),
            Type::Boolean => f.write_str("boolean"),
            Type::Command => f.write_str("command"),
            Type::Array(array_type) => {
                f.write_str("[")?;
                array_type.fmt(f)?;
                f.write_str("]")?;

                Ok(())
            }
            Type::Tuple(item_types) => fmt_collection("(", ",", ")", item_types.iter(), f),
            Type::FileHandle => f.write_str("file_handle"),
        }
    }
}
