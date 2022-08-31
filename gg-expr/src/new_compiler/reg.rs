use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct RegId(pub u16);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RegSeq {
    pub base: RegId,
    pub len: u16,
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

#[derive(Clone, Debug, Default)]
pub struct RegAlloc {
    free: Vec<RegId>,
    slots: u16,
}

impl RegAlloc {
    pub fn new() -> RegAlloc {
        Default::default()
    }

    pub fn alloc(&mut self) -> RegId {
        if let Some(reg) = self.free.pop() {
            reg
        } else {
            self.slots += 1;
            RegId(self.slots - 1)
        }
    }

    pub fn free(&mut self, id: RegId) {
        self.free.push(id);
    }

    pub fn alloc_seq(&mut self, len: u16) -> RegSeq {
        if self.free.len() >= usize::from(len) {
            self.free.sort_unstable();

            let mut seq_len = 0;
            let mut seq_start = None;

            for window in self.free.windows(2) {
                let (a, b) = (window[0].0, window[1].0);
                if b == a + 1 {
                    seq_len += 1;

                    let start = seq_start.unwrap_or(a);
                    seq_start = Some(start);

                    if seq_len == len {
                        let base = RegId(start);
                        return RegSeq { base, len };
                    }
                } else {
                    seq_len = 0;
                    seq_start = None;
                }
            }
        }

        self.slots += len;
        let base = RegId(self.slots - len);
        RegSeq { base, len }
    }

    pub fn free_seq(&mut self, seq: RegSeq) {
        self.free.extend(seq.into_iter());
    }
}
