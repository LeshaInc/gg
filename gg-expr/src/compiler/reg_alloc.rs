use crate::vm::{RegId, RegSeq};

#[derive(Clone, Debug, Default)]
pub struct RegAlloc {
    free: Vec<RegId>,
    slots: u16,
}

impl RegAlloc {
    pub fn alloc(&mut self) -> RegId {
        if let Some(reg) = self.free.pop() {
            reg
        } else {
            self.slots += 1;
            RegId(self.slots - 1)
        }
    }

    pub fn advance(&mut self, num: u16) {
        self.slots += num;
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
