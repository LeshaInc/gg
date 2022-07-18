use ahash::AHashMap;
use gg_assets::{Assets, Id};
use gg_graphics::Image;
use gg_math::{Rect, Vec2};
use wgpu::TextureFormat;

use crate::atlas::{AllocatorKind, AtlasId, AtlasPool, PoolAllocation, PoolImage};

#[derive(Debug)]
pub struct Images {
    cell_size: Vec2<u16>,
    map: AHashMap<Id<Image>, PoolAllocation>,
}

impl Images {
    pub fn new(cell_size: Vec2<u16>) -> Images {
        Images {
            cell_size,
            map: AHashMap::new(),
        }
    }

    pub fn get(&self, atlases: &AtlasPool, id: Id<Image>) -> Option<(AtlasId, Rect<f32>)> {
        let alloc = self.map.get(&id)?;
        let atlas_id = alloc.id.atlas_id;
        let atlas = atlases.get(atlas_id);
        let size = atlas.size().cast::<f32>();
        let rect = Rect::new(
            alloc.rect.min.cast::<f32>() / size,
            alloc.rect.max.cast::<f32>() / size,
        );
        Some((atlas_id, rect))
    }

    pub fn alloc(
        &mut self,
        atlases: &mut AtlasPool,
        assets: &mut Assets,
        id: Id<Image>,
    ) -> PoolAllocation {
        let (size, data) = match assets.get_by_id_mut(id) {
            Some(image) => {
                let data = image.data.take().unwrap_or_else(|| {
                    tracing::error!(?id, "image does not have data");
                    checkerboard(image.size)
                });

                (image.size, data)
            }

            None => {
                let size = Vec2::new(32, 32);
                (size, checkerboard(size))
            }
        };

        let alloc = self.map.entry(id).or_insert_with(|| {
            let preferred_allocator = if size == self.cell_size.cast() {
                Some(AllocatorKind::Grid {
                    cell_size: self.cell_size,
                })
            } else {
                None
            };

            atlases.alloc(PoolImage {
                size,
                data,
                format: TextureFormat::Rgba8UnormSrgb,
                preferred_allocator,
            })
        });

        *alloc
    }

    pub fn free(&mut self, atlases: &mut AtlasPool, id: Id<Image>) {
        if let Some(alloc) = self.map.remove(&id) {
            atlases.free(alloc.id);
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

        pixel[3] = 0;

        pos.x += 1;
        if pos.x == size.x {
            pos.x = 0;
            pos.y += 1;
        }
    }

    pixels
}
