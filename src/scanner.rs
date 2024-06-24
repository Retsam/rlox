mod token_kind;
pub use token_kind::TokenKind;

pub struct Token<'a> {
    pub kind: TokenKind,
    pub lexeme: &'a str,
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

pub struct ScanErr<'a> {
    pub line: usize,
    pub msg: &'a str,
}
type ScanResult<'a> = Result<Token<'a>, ScanErr<'a>>;
impl Scanner {
    pub fn new(source: String) -> Self {
        Self {
            source,
            line: 1,
            start: 0,
            current: 0,
        }
    }
    pub fn scan_token(&mut self) -> ScanResult {
        if self.is_at_end() {
            return self.make_token(TokenKind::Eof);
        }
        self.make_error("Unexpected character.")
    }
    fn make_token(&self, kind: TokenKind) -> ScanResult {
        Ok(Token {
            kind,
            lexeme: &self.source[self.start..self.current],
            line: self.line,
        })
    }
    fn make_error<'a>(&'a self, msg: &'a str) -> ScanResult<'a> {
        Err(ScanErr {
            msg,
            line: self.line,
        })
    }
    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
}
