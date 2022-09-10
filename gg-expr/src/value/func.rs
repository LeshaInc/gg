use std::collections::HashMap;
use std::fmt::{self, Debug, Write};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use indenter::indented;

use crate::syntax::TextRange;
use crate::vm::{CompiledConsts, CompiledInstrs, InstrIdx};
use crate::Source;

#[derive(Clone)]
pub struct Func {
    pub arity: u16,
    pub slots: u16,
    pub instrs: CompiledInstrs,
    pub consts: CompiledConsts,
    pub debug_info: Option<Arc<DebugInfo>>,
}

impl Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.debug_info.as_ref().and_then(|di| di.name.as_ref()) {
            write!(f, "fn {}", name)?;
        } else {
            write!(f, "fn")?;
        }

        write!(f, "({} args) {{", self.arity)?;

        if let Some(di) = &self.debug_info {
            let range = di.source.text.range_to_line_col(di.range);
            write!(f, " // in {} at {} ", di.source.name, range)?;
        }

        writeln!(f)?;

        let mut orig_f = f;
        let mut f = indented(&mut orig_f);

        for (i, val) in self.consts.0.iter().enumerate() {
            writeln!(f, "{}: {:?}", i, val)?;
        }

        for (i, instr) in self.instrs.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }

            write!(f, "{:35}", format!("{:?}", instr))?;

            let (di, ranges) = match &self.debug_info {
                Some(di) => match di.instruction_ranges.get(&InstrIdx(i as u32)) {
                    Some(ranges) if !ranges.is_empty() => (di, ranges),
                    _ => continue,
                },
                _ => continue,
            };

            write!(f, " // ")?;

            for (i, &range) in ranges.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }

                let range = di.source.text.range_to_line_col(range);
                write!(f, "{}", range)?;
            }
        }

        writeln!(f)?;
        write!(orig_f, "}}")?;

        Ok(())
    }
}

impl PartialEq for Func {
    fn eq(&self, other: &Self) -> bool {
        self.slots == other.slots && self.instrs == other.instrs && self.consts == other.consts
    }
}

impl Eq for Func {}

impl Hash for Func {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.slots.hash(state);
        self.instrs.hash(state);
        self.consts.hash(state);
    }
}

#[derive(Clone, Debug)]
pub struct DebugInfo {
    pub source: Arc<Source>,
    pub range: TextRange,
    pub name: Option<String>,
    pub instruction_ranges: HashMap<InstrIdx, Vec<TextRange>>,
}

impl DebugInfo {
    pub fn new(source: Arc<Source>) -> DebugInfo {
        DebugInfo {
            source,
            range: TextRange::default(),
            name: None,
            instruction_ranges: HashMap::new(),
        }
    }
}
