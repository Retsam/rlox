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
    chunk.add_constant(Value::of_float(3.4f64));
    chunk.add_constant(Value::of_float(5.6f64));
    chunk.write(Op::Constant(0), 123);
    chunk.write(Op::Constant(1), 123);
    chunk.write(Op::Add, 123);
    chunk.write(Op::Constant(2), 123);
    chunk.write(Op::Divide, 123);
    chunk.write(Op::Negate, 123);
    chunk.write(Op::Return, 123);

    chunk.disassemble("test chunk");
    let _ = vm.interpret(&chunk);
}
