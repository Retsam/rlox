use crate::chunk::Chunk;
use crate::scanner::{ScanErr, Scanner, Token, TokenKind};

type CompileErr = ScanErr;

pub fn compile(str: String) -> Result<Chunk, CompileErr> {
    let mut parser = Parser::new(Scanner::new(str));

    let mut chunk = Chunk::new();
    parser.advance();

    Ok(chunk)
}

struct Parser {
    scanner: Scanner,
    previous: Option<Token>,
    current: Option<Token>,
    had_error: bool,
    panic_mode: bool,
}
impl Parser {
    fn new(scanner: Scanner) -> Parser {
        Parser {
            scanner,
            previous: None,
            current: None,
            had_error: false,
            panic_mode: false,
        }
    }
    fn advance(&mut self) {
        self.previous = self.current.take();
        self.current = loop {
            match self.scanner.scan_token() {
                Ok(token) => {
                    break Some(token);
                }
                Err(err) => {
                    self.error_at_current(err);
                }
            }
        };
    }
    fn consume(&mut self, expected: TokenKind, err: CompileErr) {
        if self.current.as_ref().is_some_and(|t| t.kind == expected) {
            self.advance();
            return;
        }
        self.error_at_current(err);
    }
    fn error_at_current(&mut self, err: CompileErr) {
        self.error_at(err, &self.current);
        // Ideally would put this in self.errorAt, but that requires a mutable borrow which conflicts with the immutable borrow of &self.current
        self.had_error = true;
        self.panic_mode = true;
    }
    fn error_at(&self, err: CompileErr, token: &Option<Token>) {
        if self.panic_mode {
            return;
        }
        eprint!("[line {}] Error", err.line);
        match token {
            Some(token) => eprint!(
                " at '{}'",
                if token.kind == TokenKind::Eof {
                    "end"
                } else {
                    &token.lexeme
                }
            ),
            None => (),
        }
        eprintln!(": {}", err.msg);
    }
}
