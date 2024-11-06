mod string_intern;

use std::{fmt::Display, rc::Rc};
pub use string_intern::{InternString, StringInterns};

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(Rc<InternString>),
    Nil,
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(x) => write!(f, "{x}"),
            Value::String(x) => write!(f, "{x}"),
        }
    }
}
