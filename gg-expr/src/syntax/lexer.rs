use super::SyntaxKind;

pub struct Lexer<'s> {
    lexer: logos::Lexer<'s, SyntaxKind>,
}

impl Lexer<'_> {
    pub fn new(source: &str) -> Lexer<'_> {
        Lexer {
            lexer: logos::Lexer::new(source),
        }
    }
}

impl<'s> Iterator for Lexer<'s> {
    type Item = (&'s str, SyntaxKind);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.lexer.next()?;
        let slice = self.lexer.slice();
        Some((slice, token))
    }
}
