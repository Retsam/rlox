use std::fmt::Display;

#[derive(Clone, Copy)]
pub struct Value {
    val: f64,
}

impl Value {
    pub fn of_float(val: f64) -> Value {
        Value { val }
    }
    pub fn as_float(&self) -> Option<f64> {
        Some(self.val)
    }
}
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.val, f)
    }
}
