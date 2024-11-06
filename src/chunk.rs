mod disassemble;

use crate::{
    instructions::{Op, Opcode},
    value::Value,
};

pub struct Chunk {
    // Using normal, built-in Vec here instead of building my own array like the book does in C++
    // Did choose to represent it as raw bytes, rather than something like Vec<Op>, that would simplify the
    //   reading and writing, but would be sized to the largest enum variant
    pub code: Vec<u8>,
    constants: Vec<Value>,
    // Public because runtime_error reads this to report the line
    pub lines: Vec<usize>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: vec![],
            constants: vec![],
            lines: vec![],
        }
    }
    pub fn write(&mut self, ins: Op, line: usize) {
        macro_rules! simple_op {
            ($kind: ident) => {
                self.write_code(Opcode::$kind.into(), line)
            };
        }
        macro_rules! double_op {
            ($kind: ident, $val: ident) => {{
                simple_op!($kind);
                self.write_code($val, line)
            }};
        }
        macro_rules! u16_op {
            ($kind: ident, $val: ident) => {{
                simple_op!($kind);
                self.write_u16($val, line)
            }};
        }
        match ins {
            Op::Constant(val) => double_op!(Constant, val),
            Op::DefineGlobal(val) => double_op!(DefineGlobal, val),
            Op::GetGlobal(val) => double_op!(GetGlobal, val),
            Op::SetGlobal(val) => double_op!(SetGlobal, val),
            Op::GetLocal(val) => double_op!(GetLocal, val),
            Op::SetLocal(val) => double_op!(SetLocal, val),

            Op::Jump(val) => u16_op!(Jump, val),
            Op::JumpIfFalse(val) => u16_op!(JumpIfFalse, val),

            Op::Return => simple_op!(Return),
            Op::Print => simple_op!(Print),
            Op::Pop => simple_op!(Pop),
            Op::Negate => simple_op!(Negate),
            Op::Not => simple_op!(Not),
            Op::Equal => simple_op!(Equal),
            Op::Greater => simple_op!(Greater),
            Op::Less => simple_op!(Less),
            Op::Add => simple_op!(Add),
            Op::Subtract => simple_op!(Subtract),
            Op::Multiply => simple_op!(Multiply),
            Op::Divide => simple_op!(Divide),
            Op::True => simple_op!(True),
            Op::False => simple_op!(False),
            Op::Nil => simple_op!(Nil),
        }
    }
    fn write_code(&mut self, code: u8, line: usize) {
        self.code.push(code);
        self.lines.push(line);
    }
    fn write_u16(&mut self, value: u16, line: usize) {
        // Writes the u16 as two separate u8s (big endian order)
        for byte in value.to_be_bytes() {
            self.write_code(byte, line);
        }
    }
    pub fn add_constant(&mut self, value: Value) -> Option<u8> {
        self.constants.push(value);
        (self.constants.len() - 1).try_into().ok()
    }
    pub fn get_constant_unwrap(&self, const_idx: u8) -> &Value {
        self.constants
            .get(const_idx as usize)
            .unwrap_or_else(|| panic!("Invalid constant index {const_idx}"))
    }
}
