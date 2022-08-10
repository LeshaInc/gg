use gg_math::{Rect, Vec2};
use wgpu::{Device, Queue, TextureFormat, TextureView};

use super::{AllocationId, AllocatorKind, Atlas};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtlasId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoolAllocationId {
    pub atlas_id: AtlasId,
    pub alloc_id: AllocationId,
}

#[derive(Clone, Copy, Debug)]
pub struct PoolAllocation {
    pub id: PoolAllocationId,
    pub rect: Rect<u32>,
}

#[derive(Debug)]
pub struct PoolImage {
    pub size: Vec2<u32>,
    pub data: Vec<u8>,
    pub format: TextureFormat,
    pub preferred_allocator: Option<AllocatorKind>,
}

#[derive(Clone, Copy, Debug)]
pub struct PoolConfig {
    pub max_size: Vec2<u32>,
}

#[derive(Debug)]
pub struct AtlasPool {
    config: PoolConfig,
    atlases: Vec<Atlas>,
}

impl AtlasPool {
    pub fn new(config: PoolConfig) -> AtlasPool {
        AtlasPool {
            config,
            atlases: Vec::new(),
        }
    }

    pub fn alloc(&mut self, image: PoolImage) -> PoolAllocation {
        self.alloc_inner(image, 0)
    }

    pub fn get(&self, atlas_id: AtlasId) -> &Atlas {
        &self.atlases[atlas_id.0 as usize]
    }

    pub fn get_normalized_rect(&self, alloc: &PoolAllocation) -> Rect<f32> {
        let atlas = self.get(alloc.id.atlas_id);
        let size = atlas.size().cast::<f32>();
        alloc.rect.map(|v| v.cast::<f32>() / size)
    }

    fn alloc_inner(&mut self, mut image: PoolImage, start_idx: usize) -> PoolAllocation {
        for (idx, atlas) in self.atlases.iter_mut().enumerate().skip(start_idx) {
            let atlas_id = AtlasId(idx as u32);

            if let Some(kind) = image.preferred_allocator {
                if atlas.allocator().kind() != kind {
                    continue;
                }
            }

            if atlas.format() != image.format {
                continue;
            }

            if let Ok(alloc) = atlas.alloc(self.config.max_size, image.size, &mut image.data) {
                return PoolAllocation {
                    id: PoolAllocationId {
                        atlas_id,
                        alloc_id: alloc.id,
                    },
                    rect: alloc.rect,
                };
            } else {
                continue;
            }
        }

        self.create_new_atlas(&image);

        let start_idx = self.atlases.len() - 1;
        self.alloc_inner(image, start_idx)
    }

    fn create_new_atlas(&mut self, image: &PoolImage) {
        let side = image.size.map(|v| v.next_power_of_two()).max_component();
        let size = Vec2::splat(side);
        let alloc = image
            .preferred_allocator
            .unwrap_or(AllocatorKind::Tree)
            .new_allocator(size);
        let atlas = Atlas::new(image.format, alloc);
        self.atlases.push(atlas);
    }

    pub fn free(&mut self, id: PoolAllocationId) {
        self.atlases[id.atlas_id.0 as usize].free(id.alloc_id);
    }

    pub fn upload(&mut self, device: &Device, queue: &Queue) {
        for atlas in &mut self.atlases {
            atlas.upload(device, queue);
        }
    }

    pub fn texture_views(&self) -> impl ExactSizeIterator<Item = &TextureView> + '_ {
        self.atlases.iter().map(|atlas| atlas.texture_view())
    }
}
