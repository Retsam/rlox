use std::mem::transmute;

// Using an enum here is a little sketchy - it's nice to get exhaustive checking on the places I use this enum,
//   but there's probably some overhead to the conversion and error checking, even though they're the same type
//   under-the-hood even when doing some unsafe dark magic to be more efficient.
// Might move this back to raw u8 later when I don't need the exhaustive checking so much.
#[repr(u8)]
pub enum Opcode {
    Return = 1,
    Jump,
    JumpIfFalse,
    Print,
    Pop,
    Constant,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    Nil,
    True,
    False,
    Not,
    Negate,
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    // Remember to change OPCODE_MAX if you add another one here
}
const OPCODE_MAX: u8 = (Opcode::Divide) as u8;

impl TryFrom<u8> for Opcode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // Using transmute here with bounds checking instead of the safer match value block
        //  this logic is used in the vm loop, so minimizing overhead is probably a good idea
        if value > 0 && value <= OPCODE_MAX {
            unsafe { Ok(transmute::<u8, Opcode>(value)) }
        } else {
            Err(value)
        }
    }
}
impl From<Opcode> for u8 {
    fn from(value: Opcode) -> Self {
        value as u8
    }
}

type ConstIdx = u8;
type StackIdx = u8;
// This enum exists for the sake of multi-byte instructions:
//   Instead of `emitByte(OP_CONSTANT)` being followed by `emitByte(idx)` it's `emitOp(Op::Constant(idx))`
//   This might turn out to be overkill
#[derive(Debug)]
pub enum Op {
    Return,
    Jump(u16),
    JumpIfFalse(u16),
    Print,
    Pop,
    Constant(ConstIdx),
    DefineGlobal(ConstIdx),
    GetGlobal(ConstIdx),
    SetGlobal(ConstIdx),
    GetLocal(StackIdx),
    SetLocal(StackIdx),
    Nil,
    True,
    False,
    Not,
    Negate,
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
}
