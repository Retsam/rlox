use crate::chunk::Chunk;
use crate::instructions::Op;
use crate::scanner::{Scanner, Token, TokenKind};
use crate::value::{StringInterns, Value};

const UINT8_COUNT: usize = 256;

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

#[derive(Debug)]
struct Local {
    // depth is None for uninitialized variables
    depth: Option<usize>,
    // In the book this would be a borrow, but I think tricky to prove that everything lives long enough
    name: Token,
}
#[derive(Debug)]
struct Compiler {
    scope_depth: usize,
    local_count: usize,
    locals: [Option<Local>; UINT8_COUNT],
}
impl Compiler {
    fn new() -> Self {
        Compiler {
            scope_depth: 0,
            local_count: 0,
            locals: [const { None }; UINT8_COUNT],
        }
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }
    fn end_scope(&mut self) -> usize {
        self.scope_depth -= 1;

        let mut count = 0;
        loop {
            let opt_local = self.peek_local();
            if let Some(local) = opt_local {
                if local.depth.expect("Ended scope with uninitialized local") <= self.scope_depth {
                    break;
                }
                self.pop_local();
                count += 1;
            } else {
                break;
            }
        }
        count
    }
    fn add_local(&mut self, name: &Token) -> Result<(), &'static str> {
        if self.local_count == UINT8_COUNT {
            return Err("Too many local variables in function.");
        }
        let existing_local = self
            .iter_same_depth_locals()
            .find(|local| local.name.lexeme == name.lexeme);
        if existing_local.is_some() {
            return Err("Already a variable with this name in this scope.");
        }
        self.locals[self.local_count] = Some(Local {
            name: name.clone(),
            depth: None,
        });
        self.local_count += 1;
        Ok(())
    }
    fn mark_initialized(&mut self) {
        let depth = self.scope_depth;
        let local = self
            .peek_local()
            .expect("Attempted to mark initialized when no variable is being defined");
        local.depth = Some(depth);
    }
    fn peek_local(&mut self) -> Option<&mut Local> {
        safe_decr(self.local_count).and_then(|c| self.locals[c].as_mut())
    }
    fn pop_local(&mut self) {
        self.local_count -= 1;
        self.locals[self.local_count].take();
    }
    fn resolve_local(&self, name: &Token) -> Option<(u8, bool)> {
        self.iter_locals()
            .enumerate()
            .find(|(_, local)| local.name.lexeme == name.lexeme)
            .map(|(i, local)| (i as u8, local.depth.is_some()))
    }
}
fn safe_decr(val: usize) -> Option<usize> {
    if val == 0 {
        None
    } else {
        Some(val - 1)
    }
}
impl Compiler {
    fn iter_same_depth_locals(&self) -> LocalWalker {
        LocalWalker {
            idx: safe_decr(self.local_count),
            depth: Some(self.scope_depth),
            locals: &self.locals,
        }
    }
    fn iter_locals(&self) -> LocalWalker {
        LocalWalker {
            idx: safe_decr(self.local_count),
            depth: None,
            locals: &self.locals,
        }
    }
}

struct LocalWalker<'a> {
    // If none, iters all locals, otherwise iters just the current depth
    depth: Option<usize>,
    idx: Option<usize>,
    locals: &'a [Option<Local>; UINT8_COUNT],
}
impl<'a> Iterator for LocalWalker<'a> {
    type Item = &'a Local;
    fn next(&mut self) -> Option<Self::Item> {
        let option_local = self.idx.and_then(|c| self.locals[c].as_ref());
        option_local.and_then(|local| {
            // Bail out if we're trying to scan the current scope and the local is beneath it
            //      (If local.depth is None it's being initialized and is part of the current scope)
            if let (Some(local_depth), Some(target_depth)) = (local.depth, self.depth) {
                if local_depth < target_depth {
                    return None;
                }
            }
            let idx = self.idx.unwrap();
            self.idx = if idx == 0 { None } else { Some(idx - 1) };
            Some(local)
        })
    }
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
// The specific, language structure related stuff
impl<'a> Parser<'a> {
    // Parses everything at the given precedence level (or higher)
    fn parse_precedence(&mut self, precedence: ParsePrecedence) {
        self.advance();
        let prev = self.assert_prev();
        let Some(prefix) = Parser::get_rule(prev.kind).prefix else {
            self.error("Expect expression.");
            return;
        };
        let can_assign = precedence <= ParsePrecedence::Assignment;
        prefix(self, can_assign);

        while (Parser::get_rule(self.current.kind).precedence)
            .map(|rule_precedence| precedence <= rule_precedence)
            .unwrap_or(false)
        {
            self.advance();
            let infix = Parser::get_rule(self.assert_prev().kind)
                .infix
                .expect("Expect infix rule");

            infix(self, can_assign);
        }

        if can_assign && self.match_t(TokenKind::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(ParsePrecedence::Assignment);
    }

    fn block(&mut self) {
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.declaration();
        }
        self.consume(TokenKind::RightBrace, "Expect '}' after block.");
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
        self.define_variable(variable_constant)
    }
    fn parse_variable(&mut self, err: &str) -> Option<u8> {
        self.consume(TokenKind::Identifier, err);
        self.declare_variable();
        if self.compiler.scope_depth > 0 {
            return None;
        }
        self.identifier_constant()
    }
    fn declare_variable(&mut self) {
        // don't register globals
        if self.compiler.scope_depth == 0 {
            return;
        };
        let name = self.previous.as_ref().unwrap();

        if let Err(str) = self.compiler.add_local(name) {
            self.error(str);
        }
    }

    fn define_variable(&mut self, maybe_global_idx: Option<u8>) {
        if self.compiler.scope_depth > 0 {
            // To 'define' a local variable, just leave the value on top of the ValueStack
            self.compiler.mark_initialized();
            return;
        }
        self.emit_ins(match maybe_global_idx {
            Some(const_idx) => Op::DefineGlobal(const_idx),
            // only hit this case if we have too many constants - just emit a pop to ignore the value that would be defined
            _ => Op::Pop,
        });
    }
    fn statement(&mut self) {
        if self.match_t(TokenKind::Print) {
            return self.print_statement();
        } else if self.match_t(TokenKind::LeftBrace) {
            self.compiler.begin_scope();
            self.block();
            let removed_count = self.compiler.end_scope();
            for _ in 0..removed_count {
                self.emit_ins(Op::Pop);
            }
            return;
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

    fn number(&mut self, _: bool) {
        // kind of awkward that we just read previous and hope it's a Number token, but I don't want to go crazy
        // on architecture changes here
        let val = self
            .assert_prev()
            .lexeme
            .parse::<f64>()
            .expect("Tried to parse a number but failed");
        self.emit_constant(Value::Number(val));
    }
    fn string(&mut self, _: bool) {
        let raw_str = &self.previous.as_ref().unwrap().lexeme;
        let val = self
            .strings
            .build_string_value(&raw_str[1..raw_str.len() - 1]); // slice off quotes
        self.emit_constant(val);
    }
    fn variable(&mut self, can_assign: bool) {
        self.named_variable(can_assign);
    }
    fn named_variable(&mut self, can_assign: bool) {
        let var_name = self.assert_prev();
        let (set_op, get_op) = self
            .compiler
            .resolve_local(var_name)
            .map(|(idx, initialized)| {
                if !initialized {
                    self.error("Can't read local variable in its own initializer.")
                }
                (Op::SetLocal(idx), Op::GetLocal(idx))
            })
            .or_else(|| {
                self.identifier_constant()
                    .map(|idx| (Op::SetGlobal(idx), Op::GetGlobal(idx)))
            })
            .unwrap_or((Op::Pop, Op::Nil));
        if can_assign && self.match_t(TokenKind::Equal) {
            self.expression();
            self.emit_ins(set_op);
        } else {
            self.emit_ins(get_op);
        }
    }
    fn grouping(&mut self, _: bool) {
        self.expression();
        self.consume(TokenKind::RightParen, "Expected ')' after expression.");
    }
    fn unary(&mut self, _: bool) {
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

    fn binary(&mut self, _: bool) {
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
    fn literal(&mut self, _: bool) {
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

type ParseFn<'a> = fn(&mut Parser<'a>, can_assign: bool);
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
            TokenKind::Identifier => {
                parse_rule!(variable, None, None)
            }
            _ => parse_rule!(None, None, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::scanner::{Token, TokenKind};

    use super::Compiler;

    fn id_token(name: String) -> Token {
        Token {
            kind: TokenKind::Identifier,
            lexeme: name,
            line: 0,
        }
    }

    #[test]
    fn locals_test() {
        let mut compiler = Compiler::new();

        let x = id_token("x".to_string());
        let y = id_token("y".to_string());
        let z = id_token("z".to_string());

        compiler.begin_scope();
        compiler.add_local(&x).unwrap();
        compiler.mark_initialized();
        compiler.add_local(&y).unwrap();
        compiler.mark_initialized();

        assert_eq!(compiler.local_count, 2);

        assert_eq!(compiler.locals[0].as_ref().unwrap().name.lexeme, "x");
        assert_eq!(compiler.locals[1].as_ref().unwrap().name.lexeme, "y");

        compiler.begin_scope();
        compiler.add_local(&z).unwrap();
        compiler.mark_initialized();

        assert_eq!(compiler.local_count, 3);
        assert_eq!(compiler.locals[2].as_ref().unwrap().name.lexeme, "z");

        compiler.end_scope();
        assert_eq!(compiler.local_count, 2);
        assert!(compiler.locals[2].is_none());

        compiler.end_scope();
        assert_eq!(compiler.local_count, 0);
        assert!(compiler.locals[1].is_none());
        assert!(compiler.locals[1].is_none());
    }
}
