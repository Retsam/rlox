use crate::{chunk::Chunk, instructions::Opcode};

impl Chunk {
    pub fn disassemble(&self, name: &str) {
        println!("== {name} ==");

        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }
    pub fn disassemble_instruction(&self, mut offset: usize) -> usize {
        let read_line = |o| {
            *self
                .lines
                .get(o)
                .unwrap_or_else(|| panic!("Missing line number at ${o}"))
        };
        let read_byte = |o: &mut usize| {
            let v = *self
                .code
                .get(*o)
                .unwrap_or_else(|| panic!("Disassembled invalid offset {o}"));
            *o += 1;
            v
        };

        print!("{:04} ", offset);
        let line = read_line(offset);
        if offset == 0 || line != read_line(offset - 1) {
            print!("{:04} ", line);
        } else {
            print!("   | ")
        }
        match read_byte(&mut offset).try_into() {
            // double match saves `Ok()` wrapping on all the cases
            Ok(op) => match op {
                Opcode::Return => print!("OP_RETURN"),
                Opcode::Constant => {
                    let const_idx = read_byte(&mut offset);
                    let val = self.get_constant_unwrap(const_idx);
                    print!("{:16} {const_idx:4} '{val}'", "OP_CONSTANT")
                }
                Opcode::Print => print!("OP_PRINT"),
                Opcode::Pop => print!("OP_POP"),
                Opcode::Negate => print!("OP_NEGATE"),
                Opcode::Not => print!("OP_NOT"),
                Opcode::Equal => print!("OP_EQUAL"),
                Opcode::Greater => print!("OP_GREATER"),
                Opcode::Less => print!("OP_LESS"),
                Opcode::Add => print!("OP_ADD"),
                Opcode::Subtract => print!("OP_SUBTRACT"),
                Opcode::Multiply => print!("OP_MULTIPLY"),
                Opcode::Divide => print!("OP_DIVIDE"),
                Opcode::True => print!("OP_TRUE"),
                Opcode::False => print!("OP_FALSE"),
                Opcode::Nil => print!("OP_NIL"),
            },

            Err(ins) => print!("Unknown opcode {ins}"),
        }
        println!();
        offset
    }
}
