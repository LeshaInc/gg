use std::fmt::{self, Display};

use super::TextRange;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Spanned<T> {
    pub range: TextRange,
    pub item: T,
}

impl<T> Spanned<T> {
    pub fn new(range: TextRange, item: T) -> Spanned<T> {
        Spanned { range, item }
    }
}

impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.item.fmt(f)
    }
}
