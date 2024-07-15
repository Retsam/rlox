use crate::{chunk::Chunk, compiler, instructions::Opcode, value::Value};

const STACK_MAX: usize = 256;

pub struct VM {
    // The book uses raw pointers, this is an index because I think I'd have to jump into unsafe to make that work
    ip: usize,
    values: ValueStack,
}

pub enum InterpretError {
    CompileError(String),
    RuntimeError(String),
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
        print!("[ ");
        for i in 0..self.stack_top {
            print!("{} ", self.values[i].expect("stack should not be empty"))
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
    pub fn interpret(&mut self, source: String) -> InterpretResult {
        let chunk = compiler::compile(source).map_err(|e| {
            InterpretError::CompileError(format!("Failed to compile at {}: {}", e.line, e.msg))
        })?;
        self.ip = 0;
        self.run(&chunk)
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
        macro_rules! push {
            ($expression:expr) => {
                self.values.push($expression)
            };
        }
        macro_rules! pop {
            () => {
                self.values.pop()
            };
        }
        loop {
            if cfg!(feature = "DEBUG_TRACE_EXECUTION") {
                self.values.debug();
                chunk.disassemble_instruction(self.ip);
            }
            // Using a macro, allows returning from outer function
            macro_rules! binary_op {
                ($oper:tt) => {
                    if let (Some(b_val), Some(a_val)) = (pop!().as_float(), pop!().as_float()) {
                        push!(Value::of_float(a_val $oper b_val))
                    } else {
                        return Err(InterpretError::RuntimeError(
                            "Attempted to apply $oper to non-number operands".to_string(),
                        ));
                    }
                };
            }
            match self.read_byte(chunk).try_into() {
                Ok(Opcode::Return) => {
                    println!("{}", pop!());
                    return Ok(());
                }
                Ok(Opcode::Constant) => {
                    let val = self.read_constant(chunk);
                    push!(*val);
                }
                Ok(Opcode::Negate) => match pop!().as_float() {
                    Some(v) => push!(Value::of_float(-v)),
                    None => {
                        return Err(InterpretError::RuntimeError(
                            "Attempted to negate non-number".to_string(),
                        ))
                    }
                },
                Ok(Opcode::Add) => binary_op!(+),
                Ok(Opcode::Subtract) => binary_op!(-),
                Ok(Opcode::Multiply) => binary_op!(*),
                Ok(Opcode::Divide) => binary_op!(/),
                Err(code) => {
                    return Err(InterpretError::CompileError(format!(
                        "Invalid opcode {code}"
                    )));
                }
            }
        }
    }
}
