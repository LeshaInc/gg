use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Weak};

use gg_graphics::RawCanvas;
use gg_math::Vec2;
use wgpu::{
    Device, Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView,
};

#[derive(Debug)]
pub enum Canvas {
    MainWindow,
    Texture {
        size: Vec2<u32>,
        view: TextureView,
        view_index: AtomicU32,
        has_cleared: AtomicBool,
    },
}

impl RawCanvas for Canvas {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct Canvases {
    list: Vec<Weak<Canvas>>,
    strong_list: Vec<Arc<Canvas>>,
}

impl Canvases {
    pub fn new() -> Canvases {
        Canvases {
            list: Vec::new(),
            strong_list: Vec::new(),
        }
    }

    pub fn create_canvas(&mut self, device: &Device, size: Vec2<u32>) -> Arc<Canvas> {
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        });

        let view = texture.create_view(&Default::default());
        let canvas = Arc::new(Canvas::Texture {
            size,
            view,
            view_index: AtomicU32::new(0),
            has_cleared: AtomicBool::new(false),
        });

        self.list.push(Arc::downgrade(&canvas));

        canvas
    }

    pub fn update(&mut self) {
        self.strong_list.clear();
        self.list.retain(|v| match v.upgrade() {
            Some(strong) => {
                self.strong_list.push(strong);
                true
            }
            _ => false,
        });
    }

    pub fn texture_views(&self) -> impl ExactSizeIterator<Item = &TextureView> + '_ {
        let mut idx = 0;
        self.strong_list.iter().map(move |canvas| match &**canvas {
            Canvas::MainWindow => unreachable!(),
            Canvas::Texture {
                view, view_index, ..
            } => {
                view_index.store(idx, Ordering::SeqCst);
                idx += 1;
                view
            }
        })
    }
}
