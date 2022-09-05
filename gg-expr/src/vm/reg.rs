use std::fmt::{self, Debug};
use std::ops::Range;

#[derive(Clone, Copy, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct RegId(pub u16);

impl Debug for RegId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct RegSeq {
    pub base: RegId,
    pub len: u16,
}

impl Debug for RegSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}:{}", self.base.0, self.base.0 + self.len - 1)
    }
}

impl IntoIterator for RegSeq {
    type Item = RegId;
    type IntoIter = RegSeqIter;

    fn into_iter(self) -> Self::IntoIter {
        RegSeqIter {
            range: self.base.0..(self.base.0 + self.len),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegSeqIter {
    range: Range<u16>,
}

impl Iterator for RegSeqIter {
    type Item = RegId;

    fn next(&mut self) -> Option<RegId> {
        self.range.next().map(RegId)
    }
}
