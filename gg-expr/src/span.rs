#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Spanned<T> {
    pub span: Span,
    pub item: T,
}
