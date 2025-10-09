use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

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
    Array(Rc<RefCell<Vec<Value>>>, Type, bool),
    Tuple(Vec<Value>),
    FileHandle(String, FileMode),
    Object(Rc<RefCell<HashMap<String, ObjectField>>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectField {
    pub value: Value,
    pub mutable: bool,
}

impl ObjectField {
    fn get_type(&self) -> ObjectFieldType {
        ObjectFieldType {
            value: self.value.get_type(),
            mutable: self.mutable,
        }
    }
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
            Value::Array(_, value_type, mutable) => {
                Type::Array(value_type.clone().into(), *mutable)
            }
            Value::Tuple(values) => {
                Type::Tuple(values.iter().map(|x| x.get_type()).collect::<Vec<_>>())
            }
            Value::FileHandle(_, _) => Type::FileHandle,
            Value::Object(fields) => Type::Object(
                fields
                    .borrow()
                    .iter()
                    .map(|(key, value)| (key.clone(), value.get_type()))
                    .collect::<HashMap<_, _>>(),
            ),
        }
    }

    pub fn new_array<I: IntoIterator<Item = T>, T: Into<Value>>(
        values: I,
        array_type: Type,
        mutable: bool,
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

        Ok(Self::Array(
            Rc::new(RefCell::new(values)),
            array_type,
            mutable,
        ))
    }

    pub fn fmt(&self, f: &mut std::fmt::Formatter<'_>, nesting: usize) -> std::fmt::Result {
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
                let combined = Some(program)
                    .into_iter()
                    .chain(arguments.iter())
                    .map(|x| Value::String(x.to_owned()));
                fmt_collection("`", " ", "`", combined, f)?
            }
            Value::Array(data, _, _) => fmt_collection("[", ",", "]", data.borrow().iter(), f)?,
            Value::Tuple(data) => fmt_collection("(", ",", ")", data.iter(), f)?,
            Value::FileHandle(path, mode) => {
                match mode {
                    FileMode::Open => f.write_str("<file_handle:open(")?,
                    FileMode::Write => f.write_str("<file_handle:write(")?,
                    FileMode::Append => f.write_str("<file_handle:append(")?,
                };
                Value::String(path.to_owned()).fmt(f, 0)?;
                f.write_str(")>")?;
            }
            Value::Object(fields) => {
                f.write_str("{\n")?;
                let fields = fields.borrow();
                let mut fields = fields.iter().collect::<Vec<_>>();
                fields.sort_by_key(|(name, _)| *name);
                for (key, field) in fields {
                    let nesting = nesting + 1;
                    f.write_str("  ".repeat(nesting).as_str())?;
                    f.write_str(&key)?;
                    f.write_str(": ")?;
                    field.value.fmt(f, nesting)?;
                    f.write_str(",\n")?;
                }

                f.write_str("  ".repeat(nesting).as_str())?;
                f.write_str("}")?;
            }
        };

        return Ok(());
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt(f, 0)
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
    Array(Box<Self>, bool),
    Tuple(Vec<Self>),
    FileHandle,
    Object(HashMap<String, ObjectFieldType>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectFieldType {
    pub value: Type,
    pub mutable: bool,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Void => f.write_str("void"),
            Type::String => f.write_str("string"),
            Type::Integer => f.write_str("integer"),
            Type::Boolean => f.write_str("boolean"),
            Type::Command => f.write_str("command"),
            Type::Array(array_type, mutable) => {
                if *mutable {
                    f.write_str("mut ")?;
                }
                f.write_str("[")?;
                array_type.fmt(f)?;
                f.write_str("]")?;

                Ok(())
            }
            Type::Tuple(item_types) => fmt_collection("(", ",", ")", item_types.iter(), f),
            Type::FileHandle => f.write_str("file_handle"),
            Type::Object(fields) => {
                f.write_str("{ ")?;
                let mut fields = fields.iter().collect::<Vec<_>>();
                fields.sort_by_key(|(name, _)| *name);
                let mut first = true;
                for (key, field) in fields {
                    if !first {
                        f.write_str(", ")?;
                    } else {
                        first = false
                    }
                    f.write_str(&key)?;
                    f.write_str(": ")?;
                    field.value.fmt(f)?;
                }

                f.write_str(" }")?;

                Ok(())
            }
        }
    }
}

impl Type {
    pub fn is_assignable_to(&self, other: &Type) -> bool {
        // This method is dumb right now, but keeping it in to make it easier when we implement
        // mutable types being able to be assigned to non-mutable values
        self == other
    }
}
