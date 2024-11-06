use std::{
    fs,
    io::{self, stdin, stdout, Write},
    process::exit,
};

use rlox::vm::{InterpretError, VM};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        repl().unwrap_or_else(|_| exit(64))
    } else if args.len() == 2 {
        run_file(&args[1]);
    } else {
        eprintln!("Usage: rlox [path]");
        exit(64);
    }
}

fn repl() -> io::Result<()> {
    let mut vm = VM::new();
    loop {
        print!("> ");
        stdout().flush()?;
        let mut buf = String::new();
        stdin().read_line(&mut buf)?;
        if buf == "\n" {
            break;
        }
        let _ = vm.interpret(buf);
        // Clean up between lines, right now just cleans the string intern map a bit
        vm.garbage_collect();
    }
    Ok(())
}

fn run_file(path: &str) {
    let source = fs::read_to_string(path).unwrap_or_else(|_| {
        println!("Could not read file \"{path}\".");
        exit(74)
    });
    let mut vm = VM::new();
    match vm.interpret(source) {
        Err(InterpretError::CompileError) => exit(65),
        Err(InterpretError::RuntimeError) => exit(70),
        Ok(()) => {}
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn main_test() {
        VM::new_and_run("".to_string()).unwrap();
    }
}
