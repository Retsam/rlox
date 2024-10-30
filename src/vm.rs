use std::{collections::HashMap, rc::Rc};

use crate::{
    chunk::Chunk,
    compiler,
    instructions::Opcode,
    value::{InternString, StringInterns, Value},
};

const STACK_MAX: usize = 256;

pub struct VM {
    // The book uses raw pointers, this is an index because I think I'd have to jump into unsafe to make that work
    ip: usize,
    values: ValueStack,
    strings: StringInterns,
    // TODO - see if we can leverage interning
    globals: HashMap<String, Value>,
}

pub enum InterpretError {
    CompileError,
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
            values: [const { None }; STACK_MAX],
            stack_top: 0,
        }
    }
    pub fn push(&mut self, value: Value) {
        self.values[self.stack_top] = Some(value);
        self.stack_top += 1;
    }
    pub fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        let val = self.values[self.stack_top].take();
        val.expect("stack should not be empty")
    }
    pub fn peek(&mut self) -> &Value {
        let val = self.values[self.stack_top].as_ref();
        val.expect("stack should not be empty")
    }
    pub fn debug(&self) {
        print!("[ ");
        for i in 0..self.stack_top {
            print!(
                "{} ",
                self.values[i].as_ref().expect("stack should not be empty")
            )
        }
        println!("]");
    }
}

impl VM {
    pub fn new() -> VM {
        VM {
            ip: 0,
            values: ValueStack::new(),
            // Shared between the VM (for strings defined at runtime)
            // and the compiler, for constants
            strings: StringInterns::new(),
            globals: HashMap::new(),
        }
    }
    pub fn interpret(&mut self, source: String) -> InterpretResult {
        let Some(chunk) = compiler::compile(source, &mut self.strings) else {
            return Err(InterpretError::CompileError);
        };
        self.ip = 0;
        self.run(&chunk)
    }

    fn read_byte(&mut self, chunk: &Chunk) -> u8 {
        let val = chunk.code[self.ip];
        self.ip += 1;
        val
    }
    fn read_constant<'a>(&mut self, chunk: &'a Chunk) -> &'a Value {
        chunk.get_constant_unwrap(self.read_byte(chunk))
    }
    // This is used in places where only a string could be - e.g. variable names
    fn read_string_constant<'a>(&mut self, chunk: &'a Chunk) -> &'a Rc<InternString> {
        if let Value::String(val) = self.read_constant(chunk) {
            val
        } else {
            panic!("Got non-string constant")
        }
    }
    fn runtime_err(&self, msg: &str, chunk: &Chunk) -> InterpretResult {
        let line = chunk.lines[self.ip];
        println!("{msg}\n[line {line}] in script");
        Err(InterpretError::RuntimeError)
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
        macro_rules! runtime_err {
            ($msg: expr) => {
                return self.runtime_err($msg, chunk)
            };
        }
        loop {
            if cfg!(feature = "DEBUG_TRACE_EXECUTION") {
                self.values.debug();
                chunk.disassemble_instruction(self.ip);
            }
            // Using a macro, allows returning from outer function
            macro_rules! binary_op {
                ($oper:tt, $out_kind:ident) => {
                    // The book's version does `peek` instead of pop - but that complicates things here and I'm not sure why it'd be necessary
                    if let (Value::Number(b_val), Value::Number(a_val)) = (pop!(), pop!()) {
                        push!(Value::$out_kind(a_val $oper b_val))
                    } else {
                        runtime_err!("Operands must be numbers.");
                    }
                };
            }
            match self.read_byte(chunk).try_into() {
                Ok(Opcode::Return) => {
                    return Ok(());
                }
                Ok(Opcode::Constant) => {
                    let val = self.read_constant(chunk);
                    push!(val.clone());
                }
                Ok(Opcode::DefineGlobal) => {
                    let var_name = self.read_string_constant(chunk);
                    // book does peek() here, too
                    let val = pop!();
                    self.globals.insert(var_name.to_string(), val);
                }
                Ok(Opcode::GetGlobal) => {
                    let var_name = self.read_string_constant(chunk);
                    match self.globals.get(&var_name.to_string()) {
                        Some(val) => push!(val.clone()),
                        None => {
                            runtime_err!(&format!("Undefined variable '{var_name}'."))
                        }
                    }
                }
                Ok(Opcode::SetGlobal) => {
                    let var_name = self.read_string_constant(chunk);
                    let val = self.values.peek();

                    if self
                        .globals
                        .insert(var_name.to_string(), val.clone())
                        .is_none()
                    {
                        self.globals.remove(&var_name.to_string());
                        runtime_err!(&format!("Undefined variable '{var_name}'."))
                    }
                }
                Ok(Opcode::Pop) => {
                    pop!();
                }
                Ok(Opcode::Print) => println!("{}", pop!()),
                Ok(Opcode::True) => push!(Value::Bool(true)),
                Ok(Opcode::False) => push!(Value::Bool(false)),
                Ok(Opcode::Nil) => push!(Value::Nil),
                Ok(Opcode::Negate) => match pop!() {
                    Value::Number(v) => push!(Value::Number(-v)),
                    _ => {
                        runtime_err!("Attempted to negate non-number");
                    }
                },
                Ok(Opcode::Not) => match pop!() {
                    Value::Nil | Value::Bool(false) => push!(Value::Bool(true)),
                    _ => push!(Value::Bool(false)),
                },
                Ok(Opcode::Equal) => {
                    let (v2, v1) = (pop!(), pop!());
                    push!(Value::Bool(v1 == v2));
                }
                Ok(Opcode::Greater) => binary_op!(>, Bool),
                Ok(Opcode::Less) => binary_op!(<, Bool),
                Ok(Opcode::Add) => match (pop!(), pop!()) {
                    (Value::Number(v2), Value::Number(v1)) => push!(Value::Number(v2 + v1)),
                    (Value::String(v2), Value::String(v1)) => {
                        push!(self.strings.build_string_value(&format!("{v1}{v2}")))
                    }
                    _ => runtime_err!("Operands must be two numbers or two strings."),
                },
                Ok(Opcode::Subtract) => binary_op!(-, Number),
                Ok(Opcode::Multiply) => binary_op!(*, Number),
                Ok(Opcode::Divide) => binary_op!(/, Number),
                Err(code) => {
                    println!("Invalid opcode {code}");
                    return Err(InterpretError::CompileError);
                }
            }
        }
    }
    pub fn garbage_collect(&mut self) {
        self.strings.clean();
    }
}
