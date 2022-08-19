use std::fmt::{self, Debug, Write};
use std::sync::Arc;

use indenter::indented;

use crate::{Instruction, Value};

#[derive(Clone)]
pub struct Func {
    pub instructions: Arc<[Instruction]>,
    pub consts: Arc<[Value]>,
    pub captures: Vec<Value>,
}

impl Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "fn(...):")?;

        let mut f = indented(f);

        for (i, val) in self.consts.iter().enumerate() {
            writeln!(f, "{}: {:?}", i, val)?;
        }

        for (i, instr) in self.instructions.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }

            write!(f, "{:?}", instr)?;
        }

        Ok(())
    }
}
