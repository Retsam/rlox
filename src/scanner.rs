mod identifier_identifier;
mod token_kind;
pub use token_kind::TokenKind;

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
}

impl From<TokenKind> for u8 {
    fn from(value: TokenKind) -> Self {
        value as u8
    }
}

pub struct Scanner {
    source: String,
    line: usize,
    start: usize,
    current: usize,
}

#[derive(Clone)]
pub struct ScanErr {
    pub line: usize,
    pub msg: String,
}
type ScanResult = Result<Token, ScanErr>;
impl Scanner {
    pub fn new(source: String) -> Self {
        Self {
            source,
            line: 1,
            start: 0,
            current: 0,
        }
    }

    // char-aware slice of source
    fn slice(&self, start: usize, end: usize) -> String {
        let mut str = String::new();
        for i in start..end {
            str.push(self.source.chars().nth(i).unwrap());
        }
        str
    }

    fn make_token(&self, kind: TokenKind) -> ScanResult {
        Ok(Token {
            kind,
            lexeme: self.slice(self.start, self.current),
            line: self.line,
        })
    }
    fn make_error(&self, msg: &str) -> ScanResult {
        Err(ScanErr {
            msg: msg.to_string(),
            line: self.line,
        })
    }

    pub fn scan_token(&mut self) -> ScanResult {
        self.skip_whitespace();
        self.start = self.current;
        match self.advance() {
            Some('(') => self.make_token(TokenKind::LeftParen),
            Some(')') => self.make_token(TokenKind::RightParen),
            Some('{') => self.make_token(TokenKind::LeftBrace),
            Some('}') => self.make_token(TokenKind::RightBrace),
            Some(';') => self.make_token(TokenKind::Semicolon),
            Some(',') => self.make_token(TokenKind::Comma),
            Some('.') => self.make_token(TokenKind::Dot),
            Some('-') => self.make_token(TokenKind::Minus),
            Some('+') => self.make_token(TokenKind::Plus),
            Some('/') => self.make_token(TokenKind::Slash),
            Some('*') => self.make_token(TokenKind::Star),
            Some('!') => {
                if self.try_match('=') {
                    self.make_token(TokenKind::BangEqual)
                } else {
                    self.make_token(TokenKind::Bang)
                }
            }
            Some('=') => {
                if self.try_match('=') {
                    self.make_token(TokenKind::EqualEqual)
                } else {
                    self.make_token(TokenKind::Equal)
                }
            }
            Some('<') => {
                if self.try_match('=') {
                    self.make_token(TokenKind::LessEqual)
                } else {
                    self.make_token(TokenKind::Less)
                }
            }
            Some('>') => {
                if self.try_match('=') {
                    self.make_token(TokenKind::GreaterEqual)
                } else {
                    self.make_token(TokenKind::Greater)
                }
            }
            Some('"') => self.string(),
            Some('0'..='9') => self.number(),
            Some('a'..='z' | 'A'..='Z') => self.identifier(),
            Some(_) => self.make_error("Unexpected character."),
            None => self.make_token(TokenKind::Eof),
        }
    }

    // Extended scanning logic for complex cases

    fn string(&mut self) -> ScanResult {
        loop {
            match self.advance() {
                None => return self.make_error("Unterminated string."),
                Some('"') => return self.make_token(TokenKind::String),
                Some('\n') => self.line += 1,
                Some(_) => {}
            }
        }
    }
    fn is_digit(opt: Option<char>) -> bool {
        opt.is_some_and(|x| x.is_ascii_digit())
    }
    fn number(&mut self) -> ScanResult {
        while Self::is_digit(self.peek()) {
            self.advance();
        }
        // Fractional part
        if matches!(self.peek(), Some('.')) && Self::is_digit(self.peek_next()) {
            // Consume the "."
            self.advance();
            while Self::is_digit(self.peek()) {
                self.advance();
            }
        }

        self.make_token(TokenKind::Number)
    }
    fn identifier(&mut self) -> ScanResult {
        while self.peek().is_some_and(|x| x.is_ascii_alphanumeric()) {
            self.advance();
        }
        let id_type = self.identifier_type();
        self.make_token(id_type)
    }
    // Utils

    fn skip_whitespace(&mut self) {
        loop {
            match (self.peek(), self.peek_next()) {
                (Some(' ' | '\r' | '\t'), _) => {
                    self.advance();
                }
                (Some('\n'), _) => {
                    self.line += 1;
                    self.advance();
                }
                (Some('/'), Some('/')) => loop {
                    if let None | Some('\n') = self.peek() {
                        break;
                    }
                    self.advance();
                },
                _ => return,
            };
        }
    }

    fn advance(&mut self) -> Option<char> {
        let r = self.peek();
        if r.is_some() {
            self.current += 1;
        }
        r
    }
    fn try_match(&mut self, expected: char) -> bool {
        let res = self.source.chars().nth(self.current) == Some(expected);
        if res {
            self.current += 1;
        }
        res
    }
    fn peek(&self) -> Option<char> {
        self.source.chars().nth(self.current)
    }
    fn peek_next(&self) -> Option<char> {
        self.source.chars().nth(self.current + 1)
    }
}
