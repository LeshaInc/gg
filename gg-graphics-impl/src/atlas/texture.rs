use std::num::NonZeroU32;

use gg_math::{Rect, Vec2};
use wgpu::{
    Device, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
};

#[derive(Debug)]
pub struct AtlasTexture {
    texture: Texture,
    texture_view: TextureView,
    size: Vec2<u32>,
}

impl AtlasTexture {
    pub fn new(device: &Device, size: Vec2<u32>, format: TextureFormat) -> AtlasTexture {
        let texture = create_texture(device, size, format);
        let texture_view = texture.create_view(&Default::default());

        AtlasTexture {
            texture,
            texture_view,
            size,
        }
    }

    pub fn view(&self) -> &TextureView {
        &self.texture_view
    }

    pub fn upload(&mut self, queue: &Queue, rect: Rect<u32>, data: &[u8]) {
        let dst = ImageCopyTexture {
            texture: &self.texture,
            mip_level: 0,
            origin: Origin3d {
                x: rect.min.x,
                y: rect.min.y,
                z: 0,
            },
            aspect: TextureAspect::All,
        };

        let bpp = (data.len() as u32) / rect.area();
        let bytes_per_row = NonZeroU32::new(rect.width() * bpp);
        let rows_per_image = NonZeroU32::new(rect.height());

        let layout = ImageDataLayout {
            offset: 0,
            bytes_per_row,
            rows_per_image,
        };

        let size = Extent3d {
            width: rect.width(),
            height: rect.height(),
            depth_or_array_layers: 1,
        };

        queue.write_texture(dst, data, layout, size)
    }

    pub fn resize(
        &mut self,
        device: &Device,
        queue: &Queue,
        new_size: Vec2<u32>,
        format: TextureFormat,
    ) {
        if new_size == self.size {
            return;
        }

        let new_texture = create_texture(device, new_size, format);
        let old_texture = std::mem::replace(&mut self.texture, new_texture);

        self.texture_view = self.texture.create_view(&Default::default());

        let mut encoder = device.create_command_encoder(&Default::default());

        let src = old_texture.as_image_copy();
        let dst = self.texture.as_image_copy();

        let size = Extent3d {
            width: self.size.x,
            height: self.size.y,
            depth_or_array_layers: 1,
        };

        encoder.copy_texture_to_texture(src, dst, size);
        queue.submit(std::iter::once(encoder.finish()));

        self.size = new_size;
    }
}

fn create_texture(device: &Device, size: Vec2<u32>, format: TextureFormat) -> wgpu::Texture {
    device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC,
    })
}
