use std::fmt::{self, Debug, Write};
use std::sync::Arc;

use indenter::indented;

use crate::new_parser::TextRange;
use crate::{Instruction, Source, Value};

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Func {
    pub instructions: Arc<[Instruction]>,
    pub consts: Arc<[Value]>,
    pub captures: Vec<Value>,
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
            let range = di.source.range_to_line_col(di.range);
            write!(f, " // in {} at {} ", di.source.name, range)?;
        }

        writeln!(f)?;

        let mut f = indented(f);

        for (i, val) in self.consts.iter().enumerate() {
            writeln!(f, "{}: {:?}", i, val)?;
        }

        for (i, instr) in self.instructions.iter().enumerate() {
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

                let range = di.source.range_to_line_col(range);
                write!(f, "{}", range)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
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
