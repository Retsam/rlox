use crate::scanner::Scanner;
use crate::scanner::TokenKind;

pub fn compile(str: String) {
    let mut scanner = Scanner::new(str);
    let mut line: Option<usize> = None;
    while let Ok(token) = scanner.scan_token() {
        if line.is_some_and(|line| line == token.line) {
            print!("   | ");
        } else {
            print!("{:04} ", token.line);
            line = Some(token.line);
        }
        println!("{:02} '{}'", u8::from(token.kind), token.lexeme);
        if token.kind == TokenKind::Eof {
            break;
        }
    }
}
