use crate::chunk::Chunk;
use crate::instructions::Op;
use crate::scanner::{Scanner, Token, TokenKind};
use crate::value::{StringInterns, Value};

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
// The specific, language structure related stuff
impl<'a> Parser<'a> {
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

    fn declaration(&mut self) {
        if self.match_t(TokenKind::Var) {
            self.variable_declaration();
        } else {
            self.statement();
        }
        if self.panic_mode {
            self.synchronize()
        }
    }
    fn variable_declaration(&mut self) {
        let variable_constant = self.parse_variable("Expect variable name.");
        if self.match_t(TokenKind::Equal) {
            self.expression();
        } else {
            self.emit_ins(Op::Nil);
        }
        self.consume(TokenKind::Semicolon, "Expect ';' after assignment.");
        if let Some(idx) = variable_constant {
            self.define_variable(idx)
        } else {
            // only hit this case if we have too many constants - just emit a pop to ignore the value that would be defined
            self.emit_ins(Op::Pop);
        }
    }
    fn parse_variable(&mut self, err: &str) -> Option<u8> {
        self.consume(TokenKind::Identifier, err);
        self.identifier_constant()
    }
    fn define_variable(&mut self, const_idx: u8) {
        self.emit_ins(Op::DefineGlobal(const_idx));
    }
    fn statement(&mut self) {
        if self.match_t(TokenKind::Print) {
            return self.print_statement();
        }
        self.expression_statement();
    }
    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after value.");
        self.emit_ins(Op::Print);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after expression.");
        self.emit_ins(Op::Pop);
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
    fn string(&mut self) {
        let raw_str = &self.previous.as_ref().expect("foo").lexeme;
        let val = self
            .strings
            .build_string_value(&raw_str[1..raw_str.len() - 1]); // slice off quotes
        self.emit_constant(val);
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

type ParseFn<'a> = fn(&mut Parser<'a>);
struct ParseRule<'a> {
    prefix: Option<ParseFn<'a>>,
    infix: Option<ParseFn<'a>>,
    precedence: Option<ParsePrecedence>,
}
impl<'a> ParseRule<'a> {
    fn new(
        prefix: Option<ParseFn<'a>>,
        infix: Option<ParseFn<'a>>,
        precedence: Option<ParsePrecedence>,
    ) -> ParseRule<'a> {
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

impl<'a> Parser<'a> {
    fn get_rule(kind: TokenKind) -> ParseRule<'a> {
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
            TokenKind::String => {
                parse_rule!(string, None, None)
            }
            _ => parse_rule!(None, None, None),
        }
    }
}
