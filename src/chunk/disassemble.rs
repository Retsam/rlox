use crate::{chunk::Chunk, instructions::opcode};

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
        let ins = read_byte(&mut offset);
        match ins {
            opcode::RETURN => {
                print!("OP_RETURN")
            }
            opcode::CONSTANT => {
                let const_idx = read_byte(&mut offset);
                let val = self.get_constant_unwrap(const_idx);
                print!("{:16} {const_idx:4} '{val}'", "OP_CONSTANT")
            }
            _ => print!("Unknown opcode {ins}"),
        }
        println!();
        offset
    }
}
