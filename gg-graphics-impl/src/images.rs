use ahash::AHashMap;
use gg_assets::{Assets, EventKind, EventReceiver, Id};
use gg_graphics::Image;
use gg_math::{Rect, Vec2};
use wgpu::TextureFormat;

use crate::atlas::{AllocatorKind, AtlasId, AtlasPool, PoolAllocation, PoolImage};

#[derive(Debug)]
pub struct Images {
    cell_size: Vec2<u16>,
    map: AHashMap<Id<Image>, PoolAllocation>,
    event_receiver: EventReceiver<Image>,
}

impl Images {
    pub fn new(assets: &Assets, cell_size: Vec2<u16>) -> Images {
        Images {
            cell_size,
            map: AHashMap::new(),
            event_receiver: assets.subscribe(),
        }
    }

    pub fn get(&self, atlases: &AtlasPool, id: Id<Image>) -> Option<(AtlasId, Rect<f32>)> {
        let alloc = self.map.get(&id)?;
        let rect = atlases.get_normalized_rect(alloc);
        Some((alloc.id.atlas_id, rect))
    }

    pub fn alloc(&mut self, atlases: &mut AtlasPool, assets: &mut Assets, id: Id<Image>) {
        let (size, data) = match assets.get_by_id_mut(id) {
            Some(image) => {
                let data = match image.data.take() {
                    Some(v) => v,
                    None => {
                        if self.map.contains_key(&id) {
                            return;
                        }

                        checkerboard(image.size)
                    }
                };

                (image.size, data)
            }
            None => {
                if self.map.contains_key(&id) {
                    return;
                }

                let size = Vec2::new(16, 16);
                (size, checkerboard(size))
            }
        };

        let preferred_allocator = if size == self.cell_size.cast() {
            Some(AllocatorKind::Grid {
                cell_size: self.cell_size,
            })
        } else {
            None
        };

        if let Some(old_alloc) = self.map.get(&id) {
            atlases.free(old_alloc.id);
        }

        let new_alloc = atlases.alloc(PoolImage {
            size,
            data,
            format: TextureFormat::Rgba8UnormSrgb,
            preferred_allocator,
        });

        self.map.insert(id, new_alloc);
    }

    pub fn cleanup(&mut self, atlases: &mut AtlasPool) {
        for event in self.event_receiver.try_iter() {
            if event.kind == EventKind::Removed {
                if let Some(alloc) = self.map.remove(&event.asset) {
                    atlases.free(alloc.id);
                }
            }
        }
    }
}

fn checkerboard(size: Vec2<u32>) -> Vec<u8> {
    let mut pixels = vec![0; 4 * size.cast::<usize>().product()];

    let mut pos = Vec2::splat(0);
    for pixel in pixels.chunks_exact_mut(4) {
        if (pos / 4).sum() % 2 == 0 {
            pixel[0] = 255;
            pixel[2] = 255;
        }

        pixel[3] = 255;

        pos.x += 1;
        if pos.x == size.x {
            pos.x = 0;
            pos.y += 1;
        }
    }

    pixels
}
