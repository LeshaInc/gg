use wgpu::{
    BlendState, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineLayout, PipelineLayoutDescriptor, PrimitiveState, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, TextureFormat, VertexState,
};

use crate::batch::Vertex;
use crate::bindings::Bindings;

#[derive(Debug)]
pub struct Pipelines {
    pipeline_layout: PipelineLayout,
    shader: ShaderModule,
    pipeline: RenderPipeline,
}

impl Pipelines {
    pub fn new(device: &Device, bindings: &Bindings) -> Pipelines {
        let pipeline_layout = create_pipeline_layout(device, bindings);
        let shader = create_shader(device);
        let pipeline = create_pipeline(device, &pipeline_layout, &shader);
        Pipelines {
            pipeline_layout,
            shader,
            pipeline,
        }
    }

    pub fn recreate(&mut self, device: &Device, bindings: &Bindings) {
        self.pipeline_layout = create_pipeline_layout(device, bindings);
        self.pipeline = create_pipeline(device, &self.pipeline_layout, &self.shader);
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }
}

fn create_shader(device: &Device) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    })
}

fn create_pipeline_layout(device: &Device, bindings: &Bindings) -> PipelineLayout {
    device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[bindings.bind_group_layout()],
        push_constant_ranges: &[],
    })
}

fn create_pipeline(
    device: &Device,
    layout: &PipelineLayout,
    shader: &ShaderModule,
) -> RenderPipeline {
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(layout),
        vertex: VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: &[Vertex::LAYOUT],
        },
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: TextureFormat::Bgra8UnormSrgb,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::default(),
            })],
        }),
        multiview: None,
    })
}
