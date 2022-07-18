use std::ops::Range;

use gg_graphics::Color;
use gg_math::{Affine2, Rect, Vec2};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Buffer, BufferUsages, Device, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub scissor: Rect<u32>,
    pub view_proj: Affine2<f32>,
    pub view: Affine2<f32>,
    pub proj: Affine2<f32>,
}

impl State {
    fn requires_flush(&self, other: &State) -> bool {
        self.scissor != other.scissor
    }
}

impl Default for State {
    fn default() -> State {
        State {
            scissor: Rect::new(Vec2::new(0, 0), Vec2::new(800, 600)),
            view_proj: Affine2::identity(),
            view: Affine2::identity(),
            proj: Affine2::identity(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec2<f32>,
    pub tex: Vec2<f32>,
    pub tex_id: u32,
    pub color: Color,
}

impl Vertex {
    pub const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: 36,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 16,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 20,
                shader_location: 3,
            },
        ],
    };
}

#[derive(Clone, Debug, Default)]
pub struct Batch {
    pub indices: Range<u32>,
    pub state: State,
}

#[derive(Clone, Debug)]
pub struct Batcher {
    batches: Vec<Batch>,
    saved_states: Vec<State>,
    batch: Batch,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Batcher {
    pub fn new() -> Batcher {
        Batcher {
            batches: Vec::new(),
            saved_states: Vec::new(),
            batch: Batch::default(),
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn reset(&mut self, state: State) {
        if !self.saved_states.is_empty() {
            self.saved_states.clear();
            tracing::error!("reset() called with nonempty state stack");
        }

        self.batches.clear();
        self.batch = Batch {
            state,
            ..Batch::default()
        };

        self.vertices.clear();
        self.indices.clear();
    }

    pub fn create_vertex_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: slice_as_bytes(&self.vertices),
            usage: BufferUsages::VERTEX,
        })
    }

    pub fn create_index_buffer(&self, device: &Device) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: slice_as_bytes(&self.indices),
            usage: BufferUsages::INDEX,
        })
    }

    pub fn batches(&self) -> &[Batch] {
        &self.batches
    }

    pub fn flush(&mut self) {
        if !self.batch.indices.is_empty() {
            let batch = self.batch.clone();
            self.batches.push(batch);
        }

        let index = self.indices.len() as u32;
        self.batch.indices = index..index;
    }

    pub fn state(&self) -> &State {
        &self.batch.state
    }

    pub fn modify_state(&mut self, f: impl FnOnce(&mut State)) {
        let old_state = self.batch.state;
        f(&mut self.batch.state);
        if self.batch.state.requires_flush(&old_state) {
            self.flush();
        }
    }

    pub fn save_state(&mut self) {
        self.saved_states.push(self.batch.state);
    }

    pub fn restore_state(&mut self) {
        if let Some(state) = self.saved_states.pop() {
            self.batch.state = state;
        } else {
            tracing::error!("restore() called with empty state stack");
        }
    }

    pub fn next_vertex_index(&mut self) -> u32 {
        self.vertices.len() as u32
    }

    pub fn emit_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }

    pub fn emit_indices(&mut self, indices: &[u32]) {
        self.indices.extend(indices);
        self.batch.indices.end += indices.len() as u32;
    }
}

fn slice_as_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe {
        let ptr = slice.as_ptr() as *const u8;
        let len = slice.len() * std::mem::size_of::<T>();
        std::slice::from_raw_parts(ptr, len)
    }
}
