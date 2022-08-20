use std::fmt::{self, Debug};
use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::{Value, Vm};

#[derive(Clone)]
pub struct Thunk {
    pub func: Value,
    pub value: Arc<OnceCell<Value>>,
}

impl Thunk {
    pub fn new(func: Value) -> Thunk {
        Thunk {
            func,
            value: Arc::new(OnceCell::new()),
        }
    }

    pub fn force_eval(&self) -> &Value {
        self.value.get_or_init(|| {
            let mut vm = Vm::new();
            vm.eval(&self.func, &[])
        })
    }
}

impl Debug for Thunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(val) = self.value.get() {
            val.fmt(f)
        } else {
            writeln!(f, "thunk: {:?}", self.func)
        }
    }
}
