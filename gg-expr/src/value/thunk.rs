use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::{Error, Value};

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

    pub fn force_eval(&self) -> Result<&Value, Error> {
        // self.value.get_or_try_init(|| {
        //     let mut vm = Vm::new();
        //     vm.eval(&self.func, &[])
        // })
        todo!()
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

impl Eq for Thunk {}

impl PartialEq for Thunk {
    fn eq(&self, other: &Self) -> bool {
        self.func == other.func
    }
}

impl Hash for Thunk {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.func.hash(state)
    }
}
