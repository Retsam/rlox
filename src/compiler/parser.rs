use crate::{instructions::Op, scanner::TokenKind, value::Value};

use super::Parser;

/// This mod contains the majority of the actual language grammar parsing logic
/// The 'parser API' lives in compiler.rs (oddly Parser is the central struct, not Compiler),
/// This mod leverages it to create the language

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

    // This is a bit of a workaround - we need to emit a Pop for each variable that goes out of scope
    //  but I don't want to have to nest all the compiler logic in here with instruction emitting
    fn end_scope(&mut self) {
        let removed_count = self.compiler.end_scope();
        for _ in 0..removed_count {
            self.emit_ins(Op::Pop);
        }
    }

    pub fn expression(&mut self) {
        self.parse_precedence(ParsePrecedence::Assignment);
    }

    pub fn block(&mut self) {
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.declaration();
        }
        self.consume(TokenKind::RightBrace, "Expect '}' after block.");
    }

    pub fn declaration(&mut self) {
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
            self.end_scope();
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
