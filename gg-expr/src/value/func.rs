use std::fmt::{self, Debug, Write};
use std::sync::Arc;

use indenter::indented;

use crate::syntax::Span;
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
            let span = di.source.span_to_line_col(di.span);
            write!(f, " // in {} at {} ", di.source.name, span)?;
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

            let (di, spans) = match &self.debug_info {
                Some(di) => match di.instruction_spans.get(i) {
                    Some(spans) if !spans.is_empty() => (di, spans),
                    _ => continue,
                },
                _ => continue,
            };

            write!(f, " // ")?;

            for (i, &span) in spans.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }

                let span = di.source.span_to_line_col(span);
                write!(f, "{}", span)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct DebugInfo {
    pub source: Arc<Source>,
    pub span: Span,
    pub name: Option<String>,
    pub instruction_spans: Vec<Vec<Span>>,
}

impl DebugInfo {
    pub fn new(source: Arc<Source>) -> DebugInfo {
        DebugInfo {
            source,
            span: Span::default(),
            name: None,
            instruction_spans: Vec::new(),
        }
    }
}
