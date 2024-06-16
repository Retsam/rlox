use crate::{chunk::Chunk, instructions::opcode, value::Value};

const STACK_MAX: usize = 256;

pub struct VM {
    // The book uses raw pointers, this is an index because I think I'd have to jump into unsafe to make that work
    ip: usize,
    values: ValueStack,
}

pub enum InterpretError {
    CompileError(String),
    RuntimeError,
}
type InterpretResult = Result<(), InterpretError>;

// ValueStack, only $5 at burger king with fries
struct ValueStack {
    values: [Option<Value>; STACK_MAX],
    stack_top: usize,
}
impl ValueStack {
    pub fn new() -> ValueStack {
        ValueStack {
            values: [None; STACK_MAX],
            stack_top: 0,
        }
    }
    pub fn push(&mut self, value: Value) {
        self.values[self.stack_top] = Some(value);
        self.stack_top += 1;
    }
    pub fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.values[self.stack_top].expect("stack should not be empty")
    }
    pub fn debug(&self) {
        print!("[");
        for i in 0..self.stack_top {
            print!("{}", self.values[i].expect("stack should not be empty"))
        }
        println!("]");
    }
}

impl VM {
    pub fn new() -> VM {
        VM {
            ip: 0,
            values: ValueStack::new(),
        }
    }
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.ip = 0;
        self.run(chunk)
    }

    fn read_byte(&mut self, chunk: &Chunk) -> u8 {
        // Might be worth the danger of get_unchecked here
        let val = chunk.code.get(self.ip);
        self.ip += 1;
        *val.unwrap()
    }
    fn read_constant<'a>(&mut self, chunk: &'a Chunk) -> &'a Value {
        chunk.get_constant_unwrap(self.read_byte(chunk))
    }

    pub fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        loop {
            if cfg!(feature = "DEBUG_TRACE_EXECUTION") {
                self.values.debug();
                chunk.disassemble_instruction(self.ip);
            }
            match self.read_byte(chunk) {
                opcode::RETURN => {
                    println!("{}", self.values.pop());
                    return Ok(());
                }
                opcode::CONSTANT => {
                    let val = self.read_constant(chunk);
                    self.values.push(*val);
                }
                other => {
                    return Err(InterpretError::CompileError(format!(
                        "Invalid opcode {other}"
                    )));
                }
            }
        }
    }
}
