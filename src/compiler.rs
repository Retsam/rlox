use crate::chunk::Chunk;
use crate::instructions::Op;
use crate::scanner::{Scanner, Token, TokenKind};
use crate::value::{StringInterns, Value};

mod compiler_state;
use compiler_state::Compiler;

mod parser;

pub fn compile(str: String, strings: &mut StringInterns) -> Option<Chunk> {
    let mut parser = Parser::new(Scanner::new(str), strings);

    while !parser.match_t(TokenKind::Eof) {
        parser.declaration();
    }

    // end_compiler functionality
    parser.emit_ins(Op::Return);

    if parser.had_error {
        return None;
    }
    if cfg!(feature = "DEBUG_PRINT_CODE") {
        parser.chunk.disassemble("code");
    }
    Some(parser.chunk)
}

struct Parser<'a> {
    scanner: Scanner,
    chunk: Chunk,
    previous: Option<Token>,
    current: Token,
    had_error: bool,
    panic_mode: bool,
    strings: &'a mut StringInterns,
    compiler: Compiler,
}

fn stub_token() -> Token {
    Token {
        kind: TokenKind::Eof,
        lexeme: "".to_string(),
        line: 0,
    }
}

// The basic parser operations - advance, consume, etc
impl<'a> Parser<'a> {
    fn new(scanner: Scanner, strings: &'a mut StringInterns) -> Parser<'a> {
        let mut p = Parser {
            scanner,
            chunk: Chunk::new(),
            previous: None,
            // We immediately advance the parser which will override this anyway - not worth making this an Option
            current: stub_token(),
            had_error: false,
            panic_mode: false,
            strings,
            compiler: Compiler::new(),
        };
        p.advance();
        p
    }
    fn assert_prev(&self) -> &Token {
        self.previous.as_ref().unwrap()
    }
    fn advance(&mut self) {
        let new_token = loop {
            match self.scanner.scan_token() {
                Ok(token) => {
                    break token;
                }
                Err(err) => {
                    self.print_err(&err.msg, err.line, None);
                }
            }
        };
        self.previous = Some(std::mem::replace(&mut self.current, new_token));
    }
    fn consume(&mut self, expected: TokenKind, err: &str) {
        if self.current.kind == expected {
            self.advance();
            return;
        }
        self.error_at_current(err);
    }
    fn check(&self, expected: TokenKind) -> bool {
        self.current.kind == expected
    }
    fn match_t(&mut self, expected: TokenKind) -> bool {
        if self.check(expected) {
            self.advance();
            return true;
        }
        false
    }

    // Top-level error methods
    fn error_at_current(&mut self, err: &str) {
        self.error_at_token(err, |s| &s.current);
    }
    fn error(&mut self, err: &str) {
        self.error_at_token(err, |s| s.assert_prev());
    }
    fn error_at_token<F>(&mut self, err: &str, get_token: F)
    where
        F: Fn(&Self) -> &Token,
    {
        // This is taken as a callback so the caller doesn't have to borrow self.current or self.previous while also mutably borrowing self for this method
        let token = get_token(self);

        let at = if token.kind == TokenKind::Eof {
            " at end"
        } else {
            &format!(" at '{}'", token.lexeme)
        };

        self.print_err(err, token.line, Some(at));
    }

    fn print_err(&mut self, err: &str, line: usize, at: Option<&str>) {
        if self.panic_mode {
            return;
        }
        let at_str = at.unwrap_or("");
        eprintln!("[line {line}] Error{at_str}: {err}");

        self.had_error = true;
        self.panic_mode = true;
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;
        while !self.match_t(TokenKind::Eof) {
            // Synchronize when we're at a statement boundary:

            // just passed a semicolon
            if self.assert_prev().kind == TokenKind::Semicolon {
                return;
            }
            // or start of an expression
            match self.current.kind {
                TokenKind::Class
                | TokenKind::Fun
                | TokenKind::Var
                | TokenKind::For
                | TokenKind::If
                | TokenKind::While
                | TokenKind::Print
                | TokenKind::Return => return,
                _ => { /* keep going */ }
            }
            self.advance();
        }
    }
}

impl<'a> Parser<'a> {
    fn emit_ins(&mut self, ins: Op) {
        self.chunk.write(ins, self.assert_prev().line);
    }
    fn make_constant(&mut self, val: Value) -> Option<u8> {
        let res = self.chunk.add_constant(val);
        res.or_else(|| {
            self.error("Too many constants in one chunk.");
            None
        })
    }
    fn emit_constant(&mut self, val: Value) {
        match self.make_constant(val) {
            Some(constant) => self.emit_ins(Op::Constant(constant)),
            None => { /* original code emits OP_CONSTANT 0 on error */ }
        }
    }
    fn identifier_constant(&mut self) -> Option<u8> {
        let var_name = &self.previous.as_ref().unwrap().lexeme;
        let val = self.strings.build_string_value(var_name);
        self.make_constant(val)
    }
}
