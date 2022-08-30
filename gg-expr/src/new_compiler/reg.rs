#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RegId(pub u16);

#[derive(Clone, Debug, Default)]
pub struct RegAlloc {
    free: Vec<u16>,
    slots: u16,
}

impl RegAlloc {
    pub fn new() -> RegAlloc {
        Default::default()
    }

    pub fn alloc(&mut self) -> RegId {
        if let Some(id) = self.free.pop() {
            RegId(id)
        } else {
            self.slots += 1;
            RegId(self.slots - 1)
        }
    }

    pub fn free(&mut self, id: RegId) {
        self.free.push(id.0);
    }

    pub fn alloc_seq(&mut self, len: u16) -> RegId {
        if self.free.len() >= usize::from(len) {
            self.free.sort_unstable();

            let mut seq_len = 0;
            let mut seq_start = None;

            for window in self.free.windows(2) {
                let (a, b) = (window[0], window[1]);
                if b == a + 1 {
                    seq_len += 1;

                    let start = seq_start.unwrap_or(a);
                    seq_start = Some(start);

                    if seq_len == len {
                        return RegId(start);
                    }
                } else {
                    seq_len = 0;
                    seq_start = None;
                }
            }
        }

        self.slots += len;
        RegId(self.slots - len)
    }

    pub fn free_seq(&mut self, reg: RegId, len: u16) {
        self.free.extend(reg.0..(reg.0 + len));
    }
}
