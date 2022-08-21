use std::fmt::{self, Debug, Display};

use crate::diagnostic::Diagnostic;

pub struct Error {
    inner: Box<ErrorInner>,
}

struct ErrorInner {
    diagnostic: Diagnostic,
}

impl Error {
    pub fn new(diagnostic: Diagnostic) -> Error {
        Error {
            inner: Box::new(ErrorInner { diagnostic }),
        }
    }

    pub fn diagnostic(&self) -> &Diagnostic {
        &self.inner.diagnostic
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.diagnostic())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::error::Error for Error {}
