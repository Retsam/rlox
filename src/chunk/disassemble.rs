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
        macro_rules! op_with_const_idx {
            ($op_code: literal) => {{
                let const_idx = read_byte(&mut offset);
                let val = self.get_constant_unwrap(const_idx);
                print!("{:16} {const_idx:4} '{val}'", $op_code);
            }};
        }
        macro_rules! op_with_byte_arg {
            ($op_code: literal) => {{
                let byte = read_byte(&mut offset);
                print!("{:16} {byte:4}", $op_code);
            }};
        }
        macro_rules! op_with_u16_arg {
            ($op_code: literal) => {{
                let val = u16::from_be_bytes([read_byte(&mut offset), read_byte(&mut offset)]);

                print!("{:16} {val:4}", $op_code);
            }};
        }
        match read_byte(&mut offset).try_into() {
            // double match saves `Ok()` wrapping on all the cases
            Ok(op) => match op {
                Opcode::Return => print!("OP_RETURN"),
                Opcode::Jump => op_with_u16_arg!("OP_JUMP"),
                Opcode::JumpIfFalse => op_with_u16_arg!("OP_JUMP_IF_FALSE"),
                Opcode::Loop => op_with_u16_arg!("OP_LOOP"),
                Opcode::Constant => op_with_const_idx!("OP_CONSTANT"),
                Opcode::DefineGlobal => op_with_const_idx!("OP_DEFINE_GLOBAL"),
                Opcode::GetGlobal => op_with_const_idx!("OP_GET_GLOBAL"),
                Opcode::SetGlobal => op_with_const_idx!("OP_SET_GLOBAL"),
                Opcode::GetLocal => op_with_byte_arg!("OP_GET_LOCAL"),
                Opcode::SetLocal => op_with_byte_arg!("OP_SET_LOCAL"),
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
