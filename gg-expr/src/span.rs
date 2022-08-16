use std::fmt::{self, Display};

use miette::SourceSpan;

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

impl From<Span> for SourceSpan {
    fn from(v: Span) -> Self {
        (v.start as usize..v.end as usize).into()
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

impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.item.fmt(f)
    }
}
