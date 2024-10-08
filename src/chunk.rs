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
    lines: Vec<usize>,
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
        match ins {
            Op::Return => self.write_code(Opcode::Return.into(), line),
            Op::Constant(val) => {
                self.write_code(Opcode::Constant.into(), line);
                self.write_code(val, line)
            }
            Op::Negate => self.write_code(Opcode::Negate.into(), line),
            Op::Add => self.write_code(Opcode::Add.into(), line),
            Op::Subtract => self.write_code(Opcode::Subtract.into(), line),
            Op::Multiply => self.write_code(Opcode::Multiply.into(), line),
            Op::Divide => self.write_code(Opcode::Divide.into(), line),
        }
    }
    fn write_code(&mut self, code: u8, line: usize) {
        self.code.push(code);
        self.lines.push(line);
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
