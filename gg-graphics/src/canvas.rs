use std::any::Any;
use std::sync::Arc;

#[derive(Debug)]
pub struct Canvas(Arc<dyn RawCanvas>);

impl Canvas {
    pub fn from_raw<R: RawCanvas>(raw: Arc<R>) -> Canvas {
        Canvas(raw)
    }

    pub fn as_raw<R: RawCanvas>(&self) -> &R {
        self.0.as_any().downcast_ref().unwrap()
    }
}

impl Clone for Canvas {
    fn clone(&self) -> Self {
        Canvas(self.0.clone())
    }
}

pub trait RawCanvas: std::fmt::Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
}
