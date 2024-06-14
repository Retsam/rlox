use crate::{chunk::Chunk, instructions::opcode};

impl Chunk {
    pub fn disassemble(&self, name: &str) {
        println!("== {name} ==");
        let mut last_line: usize = 0;
        let mut iter = self.code.iter().enumerate();

        while let Some((offset, &ins)) = iter.next() {
            print!("{:04} ", offset);
            let line = *self
                .lines
                .get(offset)
                .unwrap_or_else(|| panic!("Missing line number at ${offset}"));
            if line != last_line {
                print!("{:04} ", line);
                last_line = line
            } else {
                print!("   | ")
            }
            match ins {
                opcode::RETURN => {
                    print!("OP_RETURN")
                }
                opcode::CONSTANT => {
                    let (_, &const_idx) = iter.next().expect("Expected a constant");
                    let val = self
                        .constants
                        .get(const_idx as usize)
                        .unwrap_or_else(|| panic!("Invalid constant index {const_idx}"));
                    print!("{:16} {const_idx:4} '{val}'", "OP_CONSTANT")
                }
                _ => print!("Unknown opcode {ins}"),
            }
            println!()
        }
    }
}
