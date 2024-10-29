use super::*;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
};

/// InternString is a str newtype that does reference equality
#[repr(transparent)]
pub struct InternString(str);
impl InternString {
    pub fn new(s: &str) -> &Self {
        unsafe { &*(s as *const _ as *const Self) }
    }
}
impl PartialEq for InternString {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Deref for InternString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for InternString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Display for InternString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
// Allows .into() to convert it into an rc - copied from the standard library equivalent for str
impl From<&InternString> for Rc<InternString> {
    #[inline]
    fn from(v: &InternString) -> Rc<InternString> {
        let rc = Rc::<[u8]>::from(v.as_bytes());
        unsafe { Rc::from_raw(Rc::into_raw(rc) as *const InternString) }
    }
}

pub struct StringInterns(
    // Stores Weak refs to existing strings so they can be reused without otherwise being retained
    HashMap<String, Weak<InternString>>,
);

impl StringInterns {
    pub fn new() -> StringInterns {
        StringInterns(HashMap::new())
    }
    pub fn get_or_intern(&mut self, string: &str) -> Rc<InternString> {
        self.0
            .get(string)
            .and_then(|weak| weak.upgrade())
            .unwrap_or_else(|| {
                let v: Rc<InternString> = InternString::new(string).into();
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
