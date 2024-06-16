use std::mem::transmute;

// Using an enum here is a little sketchy - it's nice to get exhaustive checking on the places I use this enum,
//   but there's probably some overhead to the conversion and error checking, even though they're the same type
//   under-the-hood even when doing some unsafe dark magic to be more efficient.
// Might move this back to raw u8 later when I don't need the exhaustive checking so much.
#[repr(u8)]
pub enum Opcode {
    Return = 1,
    Constant = 2,
    Negate = 3,
    Add = 4,
    Subtract = 5,
    Multiply = 6,
    Divide = 7,
}
const OPCODE_MAX: u8 = (Opcode::Divide) as u8;

impl TryFrom<u8> for Opcode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // Using transmute here with bounds checking instead of the safer match value block
        //  this logic is used in the vm loop, so minimizing overhead is probably a good idea
        if value > 0 && value <= OPCODE_MAX {
            unsafe { Ok(transmute(value)) }
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

pub enum Op {
    Return,
    Constant(/* the index of the constant */ u8),
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
}
