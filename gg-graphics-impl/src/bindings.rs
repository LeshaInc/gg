use std::num::NonZeroU32;
use std::sync::atomic::Ordering;

use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Device, Extent3d, Queue, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDimension,
};

use crate::atlas::{AtlasId, AtlasPool};
use crate::canvas::{Canvas, Canvases};

#[derive(Debug)]
pub struct Bindings {
    layout_num_textures: u32,
    bind_group_layout: BindGroupLayout,
    bind_group_layout_changed: bool,
    bind_group: BindGroup,
    sampler: Sampler,
    white_texture_view: TextureView,
    num_atlases: u32,
}

impl Bindings {
    pub fn new(device: &Device, queue: &Queue) -> Bindings {
        let count = 4;

        let white_texture_view = create_white_texture_view(device, queue);
        let bind_group_layout = create_bind_group_layout(device, count);

        let sampler = create_sampler(device);

        let views = std::iter::repeat(&white_texture_view)
            .take(count as usize)
            .collect::<Vec<_>>();
        let bind_group = create_bind_group(device, &bind_group_layout, &sampler, &views);

        Bindings {
            layout_num_textures: count,
            bind_group_layout,
            bind_group_layout_changed: false,
            bind_group,
            sampler,
            num_atlases: 0,
            white_texture_view,
        }
    }

    pub fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group_layout_changed(&mut self) -> bool {
        let res = self.bind_group_layout_changed;
        self.bind_group_layout_changed = false;
        res
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn atlas_index(&self, atlas: AtlasId) -> u32 {
        atlas.0 + 1
    }

    pub fn canvas_index(&self, canvas: &Canvas) -> u32 {
        match canvas {
            Canvas::MainWindow => 0,
            Canvas::Texture { view_index, .. } => {
                view_index.load(Ordering::SeqCst) + self.num_atlases + 1
            }
        }
    }

    pub fn update(
        &mut self,
        device: &Device,
        atlases: &AtlasPool,
        canvases: &Canvases,
        skip_view: Option<&TextureView>,
    ) {
        let atlas_views = atlases.texture_views();
        let canvas_views = canvases.texture_views();

        self.num_atlases = atlas_views.len() as u32;
        let total_count = self.num_atlases + canvas_views.len() as u32;

        if total_count > self.layout_num_textures {
            self.bind_group_layout = create_bind_group_layout(device, total_count);
            self.layout_num_textures = total_count;
            self.bind_group_layout_changed = true;
        }

        let mut texture_views = Vec::with_capacity(total_count as usize);
        texture_views.push(&self.white_texture_view);
        texture_views.extend(atlas_views);

        if let Some(skip_view) = skip_view {
            texture_views.extend(canvas_views.map(|view| {
                if std::ptr::eq(view, skip_view) {
                    &self.white_texture_view
                } else {
                    view
                }
            }));
        } else {
            texture_views.extend(canvas_views);
        }

        while texture_views.len() < self.layout_num_textures as usize {
            texture_views.push(&self.white_texture_view);
        }

        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            &self.sampler,
            &texture_views,
        );
    }
}

fn create_bind_group_layout(device: &Device, num_textures: u32) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: NonZeroU32::new(num_textures),
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_bind_group(
    device: &Device,
    layout: &BindGroupLayout,
    sampler: &Sampler,
    views: &[&TextureView],
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureViewArray(views),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn create_white_texture_view(device: &Device, queue: &Queue) -> TextureView {
    let texture = device.create_texture_with_data(
        queue,
        &TextureDescriptor {
            label: None,
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING,
        },
        &[255, 255, 255, 255],
    );

    texture.create_view(&Default::default())
}

fn create_sampler(device: &Device) -> Sampler {
    device.create_sampler(&SamplerDescriptor::default())
}
