use std::fmt::{self, Debug, Display, Write};

use indenter::indented;
use yansi::Paint;

use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::syntax::TextRange;
use crate::FuncValue;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: Box<ErrorInner>,
}

#[derive(Debug)]
struct ErrorInner {
    diagnostic: Diagnostic,
    stack_trace: Option<StackTrace>,
}

impl Error {
    pub fn new(diagnostic: Diagnostic) -> Error {
        Error {
            inner: Box::new(ErrorInner {
                diagnostic,
                stack_trace: None,
            }),
        }
    }

    pub fn with_stack_trace(mut self, stack_trace: StackTrace) -> Error {
        self.inner.stack_trace = Some(stack_trace);
        self
    }

    pub fn diagnostic(&self) -> &Diagnostic {
        &self.inner.diagnostic
    }

    pub fn stack_trace(&self) -> Option<&StackTrace> {
        self.inner.stack_trace.as_ref()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.diagnostic())?;

        if let Some(st) = self.stack_trace() {
            write!(f, "{}", st)?;
        }

        Ok(())
    }
}

impl From<Diagnostic> for Error {
    fn from(v: Diagnostic) -> Self {
        Self::new(v)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub struct StackTrace {
    pub frames: Vec<StackFrame>,
}

#[derive(Clone, Debug)]
pub struct StackFrame {
    pub range: Option<TextRange>,
    pub func: FuncValue,
}

impl Display for StackTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", Paint::new("stack trace:").bold())?;

        let max_top = 7;
        let max_bottom = 3;

        if self.frames.len() <= max_top + max_bottom {
            for (i, frame) in self.frames.iter().enumerate() {
                write!(indented(f), "{}: {}", i, frame)?;
            }
        } else {
            for (i, frame) in self.frames.iter().enumerate().take(max_top) {
                write!(indented(f), "{}: {}", i, frame)?;
            }

            let omit = self.frames.len() - max_top - max_bottom;
            let msg = format!("... {} frames omitted ...", omit);
            writeln!(indented(f), "{}", Paint::new(msg).dimmed())?;

            for (i, frame) in self.frames.iter().enumerate().rev().take(max_bottom).rev() {
                write!(indented(f), "{}: {}", i, frame)?;
            }
        }

        Ok(())
    }
}

impl Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn ")?;

        let di = match &self.func.debug_info {
            Some(v) => v,
            None => {
                return write!(f, "{}", Paint::new("unknown").dimmed());
            }
        };

        if let Some(name) = &di.name {
            write!(f, "{}", Paint::new(name).bold())?;
        } else {
            write!(f, "{}", Paint::new("unknown").dimmed())?;
        }

        let f_source = Paint::cyan(&di.source.name).underline().bold();
        let f_range = Paint::cyan(di.source.text.range_to_line_col(di.range)).bold();

        if self.range.is_some() {
            writeln!(f, " at {}", f_range)?;
        } else {
            writeln!(f, " in {} at {}", f_source, f_range)?;
        }

        if let Some(range) = self.range {
            let comp =
                SourceComponent::new(di.source.clone()).with_label(Severity::Error, range, "");
            write!(f, "{}", comp)?;
        }

        Ok(())
    }
}
