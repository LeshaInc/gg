use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub struct FontFamily {
    names: Arc<Vec<Box<str>>>,
}

impl FontFamily {
    pub fn new(primary: &str) -> FontFamily {
        FontFamily {
            names: Arc::new(vec![primary.into()]),
        }
    }

    pub fn push(mut self, fallback: &str) -> FontFamily {
        let names = Arc::make_mut(&mut self.names);
        names.push(fallback.into());
        self
    }

    pub fn names(&self) -> impl Iterator<Item = &str> + '_ {
        self.names.iter().map(|v| &**v)
    }
}
