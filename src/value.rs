use std::fmt::Display;

pub struct Value {
    val: f64,
}

impl Value {
    pub fn of_float(val: f64) -> Value {
        Value { val }
    }
}
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.val, f)
    }
}
