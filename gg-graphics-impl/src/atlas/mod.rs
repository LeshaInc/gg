mod allocator;
mod pool;
mod texture;

use gg_math::{Rect, Vec2};
use wgpu::{Device, Queue, TextureFormat, TextureView};

pub use self::allocator::{
    Allocation, AllocationId, Allocator, AllocatorKind, AnyAllocator, GridAllocator,
    ShelfAllocator, TreeAllocator,
};
pub use self::pool::{AtlasId, AtlasPool, PoolAllocation, PoolAllocationId, PoolConfig, PoolImage};
pub use self::texture::AtlasTexture;

#[derive(Debug)]
pub struct Atlas {
    format: TextureFormat,
    texture: Option<AtlasTexture>,
    allocator: AnyAllocator,
    upload_queue: Vec<(Rect<u32>, Vec<u8>)>,
}

impl Atlas {
    pub fn new(format: TextureFormat, allocator: impl Into<AnyAllocator>) -> Atlas {
        let allocator = allocator.into();

        Atlas {
            format,
            texture: None,
            allocator,
            upload_queue: Vec::new(),
        }
    }

    pub fn size(&self) -> Vec2<u32> {
        self.allocator.size()
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn allocator(&self) -> &AnyAllocator {
        &self.allocator
    }

    pub fn texture_view(&self) -> &TextureView {
        self.texture.as_ref().unwrap().view()
    }

    fn next_size(&self, max_size: Vec2<u32>) -> Option<Vec2<u32>> {
        if !self.allocator.can_grow() {
            return None;
        }

        let new_size = double_size(self.allocator.size());
        if new_size.cmp_gt(max_size).any() {
            return None;
        }

        Some(new_size)
    }

    pub fn alloc(
        &mut self,
        max_size: Vec2<u32>,
        size: Vec2<u32>,
        data: &mut Vec<u8>,
    ) -> Result<Allocation, NoSpaceError> {
        let alloc = loop {
            match self.allocator.alloc(size) {
                Some(alloc) => break alloc,
                None => {
                    let next_size = self.next_size(max_size).ok_or(NoSpaceError)?;
                    self.allocator.grow(next_size);
                }
            };
        };

        let data = std::mem::take(data);
        self.upload_queue.push((alloc.rect, data));

        Ok(alloc)
    }

    pub fn free(&mut self, id: AllocationId) {
        self.allocator.free(id);
    }

    pub fn upload(&mut self, device: &Device, queue: &Queue) {
        let size = self.allocator.size();
        let texture = self
            .texture
            .get_or_insert_with(|| AtlasTexture::new(device, size, self.format));

        texture.resize(device, queue, size, self.format);

        for (rect, data) in self.upload_queue.drain(..) {
            texture.upload(queue, rect, &data);
        }
    }
}

#[derive(Debug)]
pub struct NoSpaceError;

fn double_size(size: Vec2<u32>) -> Vec2<u32> {
    if size.x < size.y {
        Vec2::new(size.x * 2, size.y)
    } else {
        Vec2::new(size.x, size.y * 2)
    }
}
