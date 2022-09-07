use std::fmt::{self, Debug, Write};
use std::sync::Arc;

use indenter::indented;

use crate::syntax::TextRange;
use crate::vm::{CompiledConsts, CompiledInstrs};
use crate::Source;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Func {
    pub slots: u16,
    pub instrs: CompiledInstrs,
    pub consts: CompiledConsts,
    pub debug_info: Option<Arc<DebugInfo>>,
}

impl Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.debug_info.as_ref().and_then(|di| di.name.as_ref()) {
            write!(f, "fn {}(...):", name)?;
        } else {
            write!(f, "fn(...):")?;
        }

        if let Some(di) = &self.debug_info {
            let range = di.source.text.range_to_line_col(di.range);
            write!(f, " // in {} at {} ", di.source.name, range)?;
        }

        writeln!(f)?;

        let mut f = indented(f);

        for (i, val) in self.consts.0.iter().enumerate() {
            writeln!(f, "{}: {:?}", i, val)?;
        }

        for (i, instr) in self.instrs.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }

            write!(f, "{:20}", format!("{:?}", instr))?;

            let (di, ranges) = match &self.debug_info {
                Some(di) => match di.instruction_ranges.get(i) {
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

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DebugInfo {
    pub source: Arc<Source>,
    pub range: TextRange,
    pub name: Option<String>,
    pub instruction_ranges: Vec<Vec<TextRange>>,
}

impl DebugInfo {
    pub fn new(source: Arc<Source>) -> DebugInfo {
        DebugInfo {
            source,
            range: TextRange::default(),
            name: None,
            instruction_ranges: Vec::new(),
        }
    }
}
