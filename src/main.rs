mod chunk;
mod instructions;
mod value;
mod vm;

use chunk::Chunk;
use instructions::Op;
use value::Value;
use vm::VM;

fn main() {
    let mut vm = VM::new();

    let mut chunk = Chunk::new();
    chunk.add_constant(Value::of_float(1.2f64));
    chunk.write(Op::Constant(0), 123);
    chunk.write(Op::Negate, 123);
    chunk.write(Op::Return, 123);

    chunk.disassemble("test chunk");
    let _ = vm.interpret(&chunk);
}
