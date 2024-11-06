use crate::scanner::Token;

const UINT8_COUNT: usize = 256;

fn safe_decrement(val: usize) -> Option<usize> {
    if val == 0 {
        None
    } else {
        Some(val - 1)
    }
}

#[derive(Debug)]
pub struct Compiler {
    pub scope_depth: usize,
    local_count: usize,
    locals: [Option<Local>; UINT8_COUNT],
}

#[derive(Debug)]
struct Local {
    // depth is None for uninitialized variables
    depth: Option<usize>,
    // In the book this would be a borrow, but I think tricky to prove that everything lives long enough
    name: Token,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            scope_depth: 0,
            local_count: 0,
            locals: [const { None }; UINT8_COUNT],
        }
    }

    pub fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }
    pub fn end_scope(&mut self) -> usize {
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
    pub fn add_local(&mut self, name: &Token) -> Result<(), &'static str> {
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
    pub fn mark_initialized(&mut self) {
        let depth = self.scope_depth;
        let local = self
            .peek_local()
            .expect("Attempted to mark initialized when no variable is being defined");
        local.depth = Some(depth);
    }
    pub fn resolve_local(&self, name: &Token) -> Option<(u8, bool)> {
        self.iter_locals()
            .enumerate()
            .find(|(_, local)| local.name.lexeme == name.lexeme)
            .map(|(i, local)| (i as u8, local.depth.is_some()))
    }

    fn peek_local(&mut self) -> Option<&mut Local> {
        safe_decrement(self.local_count).and_then(|c| self.locals[c].as_mut())
    }
    fn pop_local(&mut self) {
        self.local_count -= 1;
        self.locals[self.local_count].take();
    }
}

impl Compiler {
    fn iter_same_depth_locals(&self) -> LocalWalker {
        LocalWalker {
            idx: safe_decrement(self.local_count),
            depth: Some(self.scope_depth),
            locals: &self.locals,
        }
    }
    fn iter_locals(&self) -> LocalWalker {
        LocalWalker {
            idx: safe_decrement(self.local_count),
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
