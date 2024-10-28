use std::{
    collections::HashMap,
    fmt::Display,
    rc::{Rc, Weak},
};

#[derive(Clone)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(Rc<str>),
    Nil,
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

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(l0), Self::Number(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => Rc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

pub struct StringInterns(
    // Stores Weak refs to existing strings so they can be reused without otherwise being retained
    HashMap<String, Weak<str>>,
);

impl StringInterns {
    pub fn new() -> StringInterns {
        StringInterns(HashMap::new())
    }
    pub fn get_or_intern(&mut self, string: &str) -> Rc<str> {
        self.0
            .get(string)
            .and_then(|weak| weak.upgrade())
            .unwrap_or_else(|| {
                let v: Rc<str> = string.into();
                self.0.insert(string.to_string(), Rc::downgrade(&v));
                v
            })
    }
    pub fn build_string_value(&mut self, string: &str) -> Value {
        Value::String(self.get_or_intern(string))
    }

    /// Remove any weak refs that no longer point to a string
    pub fn clean(&mut self) {
        self.0.retain(|_, val| val.upgrade().is_some());
    }
}
