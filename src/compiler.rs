use crate::chunk::Chunk;
use crate::instructions::Op;
use crate::scanner::{Scanner, Token, TokenKind};
use crate::value::Value;

pub fn compile(str: String) -> Option<Chunk> {
    let mut parser = Parser::new(Scanner::new(str));

    parser.expression();

    parser.consume(TokenKind::Eof, "Expected end of expression.");

    if parser.had_error {
        return None;
    }

    // end_compiler functionality
    parser.emit_ins(Op::Return);
    if cfg!(feature = "DEBUG_PRINT_CODE") && !parser.had_error {
        parser.chunk.disassemble("code");
    }
    Some(parser.chunk)
}

struct Parser {
    scanner: Scanner,
    chunk: Chunk,
    previous: Option<Token>,
    current: Token,
    had_error: bool,
    panic_mode: bool,
}

fn stub_token() -> Token {
    Token {
        kind: TokenKind::Eof,
        lexeme: "".to_string(),
        line: 0,
    }
}

// The basic parser operations - advance, consume, etc
impl Parser {
    fn new(scanner: Scanner) -> Parser {
        let mut p = Parser {
            scanner,
            chunk: Chunk::new(),
            previous: None,
            // We immediately advance the parser which will override this anyway - not worth making this an Option
            current: stub_token(),
            had_error: false,
            panic_mode: false,
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
            &format!(" at {}", token.lexeme)
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
}

impl Parser {
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
}
// The specific, language structure related stuff
impl Parser {
    // Parses everything at the given precedence level (or higher)
    fn parse_precedence(&mut self, precedence: ParsePrecedence) {
        self.advance();
        let prev = self.assert_prev();
        let Some(prefix) = Parser::get_rule(prev.kind).prefix else {
            self.error("Expect expression");
            return;
        };
        prefix(self);

        while precedence
            < Parser::get_rule(self.current.kind)
                .precedence
                .unwrap_or(ParsePrecedence::Assignment)
        {
            self.advance();
            let infix = Parser::get_rule(self.assert_prev().kind)
                .infix
                .expect("Expect infix rule");

            infix(self);
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(ParsePrecedence::Assignment);
    }

    fn number(&mut self) {
        // kind of awkward that we just read previous and hope it's a Number token, but I don't want to go crazy
        // on architecture changes here
        let val = self
            .assert_prev()
            .lexeme
            .parse::<f64>()
            .expect("Tried to parse a number but failed");
        self.emit_constant(Value::Number(val));
    }
    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenKind::RightParen, "Expected ')' after expression.");
    }
    fn unary(&mut self) {
        let op = match self.assert_prev().kind {
            TokenKind::Minus => Op::Negate,
            TokenKind::Bang => Op::Not,
            _ => {
                self.error("Expected unary operator.");
                return;
            }
        };
        self.parse_precedence(ParsePrecedence::Unary);
        self.emit_ins(op);
    }

    fn binary(&mut self) {
        // lhs side has already been parsed

        let operator = self.assert_prev().kind;

        let rule = Parser::get_rule(operator);
        let precedence = rule
            .precedence
            .expect("Couldn't get precedence for binary operator");

        self.parse_precedence(precedence.next());
        self.emit_ins(match operator {
            TokenKind::Plus => Op::Add,
            TokenKind::Minus => Op::Subtract,
            TokenKind::Star => Op::Multiply,
            TokenKind::Slash => Op::Divide,
            // !=, <=, >= are done with two ops, this one, followed by a not
            TokenKind::Less | TokenKind::GreaterEqual => Op::Less,
            TokenKind::Greater | TokenKind::LessEqual => Op::Greater,
            TokenKind::EqualEqual | TokenKind::BangEqual => Op::Equal,
            _ => panic!("Unexpected token as binary operator"),
        });
        if matches!(
            operator,
            TokenKind::GreaterEqual | TokenKind::LessEqual | TokenKind::BangEqual,
        ) {
            self.emit_ins(Op::Not);
        }
    }
    fn literal(&mut self) {
        self.emit_ins(match self.assert_prev().kind {
            TokenKind::True => Op::True,
            TokenKind::False => Op::False,
            TokenKind::Nil => Op::Nil,
            _ => panic!("Unexpected literal token"),
        });
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum ParsePrecedence {
    // None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}
impl ParsePrecedence {
    fn next(&self) -> ParsePrecedence {
        match self {
            ParsePrecedence::Assignment => ParsePrecedence::Or,
            ParsePrecedence::Or => ParsePrecedence::And,
            ParsePrecedence::And => ParsePrecedence::Equality,
            ParsePrecedence::Equality => ParsePrecedence::Comparison,
            ParsePrecedence::Comparison => ParsePrecedence::Term,
            ParsePrecedence::Term => ParsePrecedence::Factor,
            ParsePrecedence::Factor => ParsePrecedence::Unary,
            ParsePrecedence::Unary => ParsePrecedence::Call,
            ParsePrecedence::Call => ParsePrecedence::Primary,
            ParsePrecedence::Primary => ParsePrecedence::Primary,
        }
    }
}

type ParseFn = fn(&mut Parser);
struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Option<ParsePrecedence>,
}
impl ParseRule {
    fn new(
        prefix: Option<ParseFn>,
        infix: Option<ParseFn>,
        precedence: Option<ParsePrecedence>,
    ) -> ParseRule {
        ParseRule {
            prefix,
            infix,
            precedence,
        }
    }
}

macro_rules! parse_rule {
    (None, None, None) => {
        ParseRule::new(None, None, None)
    };
    ($pre:ident, None, None) => {
        ParseRule::new(Some(Parser::$pre), None, None)
    };
    (None, $inf:ident, None) => {
        ParseRule::new(None, Some(Parser::$inf), None)
    };
    (None, $inf:ident, $precedence:ident) => {
        ParseRule::new(None, Some(Parser::$inf), Some(ParsePrecedence::$precedence))
    };
    ($pre:ident, $inf:ident, $precedence:ident) => {
        ParseRule::new(
            Some(Parser::$pre),
            Some(Parser::$inf),
            Some(ParsePrecedence::$precedence),
        )
    };
}

impl Parser {
    fn get_rule(kind: TokenKind) -> ParseRule {
        match kind {
            TokenKind::LeftParen => {
                parse_rule!(grouping, None, None)
            }
            TokenKind::Minus => {
                parse_rule!(unary, binary, Term)
            }
            TokenKind::Plus => {
                parse_rule!(None, binary, Term)
            }
            TokenKind::Slash | TokenKind::Star => {
                parse_rule!(None, binary, Factor)
            }
            TokenKind::Bang => {
                parse_rule!(unary, None, None)
            }
            TokenKind::BangEqual | TokenKind::EqualEqual => {
                parse_rule!(None, binary, Equality)
            }
            TokenKind::Less
            | TokenKind::Greater
            | TokenKind::LessEqual
            | TokenKind::GreaterEqual => {
                parse_rule!(None, binary, Comparison)
            }
            TokenKind::True | TokenKind::False | TokenKind::Nil => {
                parse_rule!(literal, None, None)
            }
            TokenKind::Number => {
                parse_rule!(number, None, None)
            }
            _ => parse_rule!(None, None, None),
        }
    }
}
