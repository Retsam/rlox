use super::{Scanner, TokenKind};

fn match_rest(expected: &'static str, actual: &str, skip: usize, if_match: TokenKind) -> TokenKind {
    if actual.len() - skip == expected.len() && actual.ends_with(expected) {
        if_match
    } else {
        TokenKind::Identifier
    }
}

impl Scanner {
    pub(super) fn identifier_type(&mut self) -> TokenKind {
        let word = &self.slice(self.start, self.current).to_string();

        // The idea is to be efficient, only checking a first letter match, rather than, e.g. looking up in a hash-table, which might be more expensive.
        // TBH, I'm not sure this is significantly more efficient than just doing a bunch of == comparisons; may want to benchmark it, but this is (roughly) the approach the book took
        macro_rules! simple_match {
            ($match:literal, $kind:ident) => {
                // Doing slicing indexing here because we know we're only matching ascii here
                //   don't need unicode aware splitting
                if (&word[0..1] == &$match[0..1]) {
                    return match_rest(&$match[1..], word, 1, TokenKind::$kind);
                }
            };
        }
        simple_match!("and", And);
        simple_match!("class", Class);
        simple_match!("else", Else);
        if &word[0..1] == "f" {
            return match &word[1..2] {
                "a" => match_rest("lse", word, 2, TokenKind::False),
                "o" => match_rest("r", word, 2, TokenKind::For),
                "u" => match_rest("n", word, 2, TokenKind::Fun),
                _ => TokenKind::Identifier,
            };
        }
        simple_match!("if", If);
        simple_match!("nil", Nil);
        simple_match!("or", Or);
        simple_match!("print", Print);
        simple_match!("return", Return);
        if &word[0..1] == "t" {
            return match &word[1..2] {
                "h" => match_rest("is", word, 2, TokenKind::This),
                "r" => match_rest("ue", word, 2, TokenKind::True),
                _ => TokenKind::Identifier,
            };
        }
        simple_match!("var", Var);
        simple_match!("while", While);
        TokenKind::Identifier
    }
}
