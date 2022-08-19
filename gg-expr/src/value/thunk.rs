use std::fmt::{self, Debug};
use std::sync::Arc;

use arc_swap::ArcSwapOption;

use crate::{Value, Vm};

#[derive(Clone)]
pub struct Thunk {
    pub func: Value,
    pub value: Arc<ArcSwapOption<Value>>,
}

impl Thunk {
    pub fn new(func: Value) -> Thunk {
        Thunk {
            func,
            value: Arc::new(ArcSwapOption::new(None)),
        }
    }

    pub fn force_eval(&self) {
        let value = self.value.load();
        if value.is_some() {
            return;
        }

        let mut vm = Vm::new();
        let res = vm.eval(self.func.clone());
        self.value.store(Some(Arc::new(res)));
    }
}

impl Debug for Thunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opt = self.value.load();
        if let Some(val) = &*opt {
            val.fmt(f)
        } else {
            writeln!(f, "thunk: {:?}", self.func)
        }
    }
}
