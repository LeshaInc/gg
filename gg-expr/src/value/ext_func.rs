use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};

use crate::{Result, Value, VmContext};

pub struct ExtFunc {
    pub arity: u16,
    pub name: Option<String>,
    pub func: Box<DynFn>,
}

impl ExtFunc {
    pub fn new<const N: usize, F>(func: F) -> ExtFunc
    where
        F: Fn(&VmContext, &[Value; N]) -> Result<Value> + 'static,
    {
        ExtFunc {
            arity: N as u16,
            name: None,
            func: Box::new(move |ctx, args| {
                let args = <&[Value; N]>::try_from(args).unwrap();
                func(ctx, args)
            }),
        }
    }
}

type DynFn = dyn Fn(&VmContext, &[Value]) -> Result<Value>;

impl Hash for ExtFunc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&*self.func as *const DynFn).hash(state);
    }
}

impl PartialEq for ExtFunc {
    fn eq(&self, other: &Self) -> bool {
        (&*self.func as *const _) == (&*other.func as *const _)
    }
}

impl Eq for ExtFunc {}

impl Debug for ExtFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "fn {}", name)?;
        } else {
            write!(f, "fn")?;
        }

        write!(f, "({} args)", self.arity)
    }
}
