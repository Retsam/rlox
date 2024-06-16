// Should probably not use an enum and just use a raw u8 for perf reasons, the Enum -> u8 lookup is unfortunate
//  and probably adds to overhead for the v8 runner... but it's nice to have exhaustive checking for development
//   (other than the TryFrom)
// Can go back and remove it later and see if it's a perf win
#[repr(u8)]
pub enum Opcode {
    Return = 1,
    Constant = 2,
}
impl TryFrom<u8> for Opcode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Opcode::Return),
            2 => Ok(Opcode::Constant),
            other => Err(other),
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
}
