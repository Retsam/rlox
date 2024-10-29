use super::*;
use std::{
    collections::HashMap,
    rc::{Rc, Weak},
};

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
