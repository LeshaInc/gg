#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Span {
        Span { start, end }
    }

    pub fn slice(self, s: &str) -> &str {
        &s[self.start as usize..self.end as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Spanned<T> {
    pub span: Span,
    pub item: T,
}

impl<T> Spanned<T> {
    pub fn new(span: Span, item: T) -> Spanned<T> {
        Spanned { span, item }
    }
}
