pub mod opcode {
    pub const RETURN: u8 = 0;
    pub const CONSTANT: u8 = 1;
}

pub enum Op {
    Return,
    Constant(/* the index of the constant */ u8),
}
