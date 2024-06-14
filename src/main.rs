mod chunk;
mod instructions;
mod value;

use chunk::Chunk;
use instructions::Op;
use value::Value;

fn main() {
    let mut chunk = Chunk::new();
    chunk.add_constant(Value::of_float(1.2f64));
    chunk.write(Op::Constant(0), 123);
    chunk.write(Op::Return, 123);

    chunk.disassemble("test chunk");
}
