use std::sync::atomic::Ordering;
use std::sync::Arc;

use gg_assets::{Assets, Id};
use gg_graphics::{
    Backend, Color, Command, CommandList, DrawGlyph, DrawRect, FillImage, Image, NinePatchImage,
};
use gg_math::{Affine2, Rect, Vec2};
use gg_util::eyre::{eyre, Result};
use wgpu::util::backend_bits_from_env;
use wgpu::{
    Backends, CommandEncoder, Device, DeviceDescriptor, Features, IndexFormat, Instance, LoadOp,
    Operations, PowerPreference, PresentMode, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, Surface, SurfaceConfiguration, TextureFormat,
    TextureUsages, TextureView,
};
use winit::window::Window;

use crate::atlas::{AtlasPool, PoolConfig};
use crate::batch::{Batcher, State, Vertex};
use crate::bindings::Bindings;
use crate::canvas::{Canvas, Canvases};
use crate::glyphs::{GlyphKey, Glyphs};
use crate::images::Images;
use crate::pipeline::Pipelines;

pub struct BackendImpl {
    device: Device,
    queue: Queue,
    surface: Surface,
    batcher: Batcher,
    atlases: AtlasPool,
    images: Images,
    glyphs: Glyphs,
    canvases: Canvases,
    bindings: Bindings,
    pipelines: Pipelines,
    submitted_lists: Vec<CommandList>,
    resolution: Vec2<u32>,
}

impl BackendImpl {
    pub fn new(assets: &Assets, window: &Window) -> Result<BackendImpl> {
        let backend = backend_bits_from_env().unwrap_or(Backends::PRIMARY);
        let instance = Instance::new(backend);
        let surface = unsafe { instance.create_surface(window) };
        let size = window.inner_size();
        let resolution = Vec2::new(size.width, size.height);

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .ok_or_else(|| eyre!("No adapter"))?;

        let limits = adapter.limits();

        let desc = &DeviceDescriptor {
            label: None,
            features: Features::TEXTURE_BINDING_ARRAY
                | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            limits: limits.clone(),
        };

        let (device, queue) = pollster::block_on(adapter.request_device(desc, None))?;

        let batcher = Batcher::new();
        let atlases = AtlasPool::new(PoolConfig {
            max_size: Vec2::splat(limits.max_texture_dimension_2d.min(8192)),
        });

        let images = Images::new(assets, Vec2::splat(8)); // TODO: configure separately
        let glyphs = Glyphs::new();
        let canvases = Canvases::new();
        let bindings = Bindings::new(&device, &queue);
        let pipelines = Pipelines::new(&device, &bindings);

        let mut backend = BackendImpl {
            device,
            queue,
            surface,
            batcher,
            atlases,
            images,
            glyphs,
            canvases,
            bindings,
            pipelines,
            submitted_lists: Vec::new(),
            resolution,
        };

        backend.configure_surface();

        Ok(backend)
    }
}

impl Backend for BackendImpl {
    fn get_main_canvas(&self) -> gg_graphics::Canvas {
        let raw = Arc::new(Canvas::MainWindow);
        gg_graphics::Canvas::from_raw(raw)
    }

    fn create_canvas(&mut self, size: Vec2<u32>) -> gg_graphics::Canvas {
        let raw = self.canvases.create_canvas(&self.device, size);
        gg_graphics::Canvas::from_raw(raw)
    }

    fn submit(&mut self, commands: CommandList) {
        self.submitted_lists.push(commands);
    }

    fn resize(&mut self, new_resolution: Vec2<u32>) {
        let old_resolution = self.resolution;
        if old_resolution != new_resolution {
            self.resolution = new_resolution;
            self.configure_surface();
        }
    }

    fn present(&mut self, assets: &mut Assets) {
        let submitted_lists = std::mem::take(&mut self.submitted_lists);

        self.images.cleanup(&mut self.atlases);

        for list in &submitted_lists {
            self.alloc_list(assets, list);
        }

        self.atlases.upload(&self.device, &self.queue);
        self.canvases.update();

        let surface_texture = match self.surface.get_current_texture() {
            Ok(v) => v,
            Err(_) => {
                self.configure_surface();
                self.surface.get_current_texture().unwrap()
            }
        };

        let main_view = surface_texture.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        for list in &submitted_lists {
            let skip_view = match list.canvas.as_raw() {
                Canvas::MainWindow => None,
                Canvas::Texture { view, .. } => Some(view),
            };

            self.bindings
                .update(&self.device, &self.atlases, &self.canvases, skip_view);

            if self.bindings.bind_group_layout_changed() {
                self.pipelines.recreate(&self.device, &self.bindings);
            }

            let clear_color = self.batch_list(assets, list);
            self.encode_pass(&mut encoder, clear_color, list.canvas.as_raw(), &main_view);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        self.submitted_lists = submitted_lists;
        self.submitted_lists.clear();
    }
}

impl BackendImpl {
    fn alloc_list(&mut self, assets: &mut Assets, commands: &CommandList) {
        for command in &commands.list {
            match command {
                Command::DrawRect(rect) => {
                    if let Some(image) = &rect.fill.image {
                        self.alloc_fill_image(assets, image);
                    }
                }
                Command::DrawGlyph(glyph) => {
                    self.alloc_glyph(assets, glyph);
                }
                _ => {}
            }
        }
    }

    fn alloc_fill_image(&mut self, assets: &mut Assets, image: &FillImage) {
        match image {
            FillImage::Canvas(_) => {}
            FillImage::SingleImage(id) => {
                self.images.alloc(&mut self.atlases, assets, *id);
            }
            FillImage::NinePatchImage(id) => {
                let nine_patch = match assets.get_by_id(*id) {
                    Some(v) => v,
                    None => {
                        return tracing::error!(?id, "nine patch image does not exit");
                    }
                };

                for id in nine_patch.sub_images() {
                    self.images.alloc(&mut self.atlases, assets, id);
                }
            }
        }
    }

    fn alloc_glyph(&mut self, assets: &mut Assets, glyph: &DrawGlyph) {
        self.glyphs.alloc(
            &mut self.atlases,
            assets,
            GlyphKey {
                font: glyph.font,
                glyph: glyph.glyph,
                size: glyph.size,
            },
        );
    }

    fn configure_surface(&mut self) {
        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: TextureFormat::Bgra8UnormSrgb,
                width: self.resolution.x,
                height: self.resolution.y,
                present_mode: PresentMode::AutoNoVsync,
            },
        )
    }

    fn batch_list(&mut self, assets: &Assets, commands: &CommandList) -> Option<Color> {
        let resolution = match *commands.canvas.as_raw() {
            Canvas::MainWindow => self.resolution,
            Canvas::Texture { size, .. } => size,
        };

        let full_scissor = Rect::new(Vec2::zero(), resolution);
        let proj = projection_matrix(resolution);
        self.batcher.reset(State {
            scissor: full_scissor,
            view_proj: proj,
            view: Affine2::identity(),
            proj,
        });

        let it = commands.list.iter().enumerate();
        let (start_idx, clear_color) = it
            .flat_map(|(i, cmd)| match cmd {
                Command::Clear(v) => Some((i + 1, Some(*v))),
                _ => None,
            })
            .next()
            .unwrap_or((0, None));

        for command in &commands.list[start_idx..] {
            match command {
                Command::Save => {
                    self.batcher.save_state();
                }
                Command::Restore => {
                    self.batcher.restore_state();
                }
                Command::SetScissor(rect) => {
                    self.batcher
                        .modify_state(|state| state.scissor = rect.intersect(&full_scissor));
                }
                Command::ClearScissor => {
                    self.batcher
                        .modify_state(|state| state.scissor = full_scissor);
                }
                &Command::PreTransform(v) => {
                    self.batcher.modify_state(|state| {
                        state.view = state.view * v;
                        state.view_proj = state.proj * state.view;
                    });
                }
                &Command::PostTransform(v) => {
                    self.batcher.modify_state(|state| {
                        state.view = v * state.view;
                        state.view_proj = state.proj * state.view;
                    });
                }
                Command::Clear(_) => {}
                Command::DrawRect(rect) => {
                    self.draw_rect(assets, rect);
                }
                Command::DrawGlyph(glyph) => {
                    self.draw_glyph(glyph);
                }
            }
        }

        self.batcher.flush();
        clear_color
    }

    fn draw_rect(&mut self, assets: &Assets, rect: &DrawRect) {
        match &rect.fill.image {
            Some(FillImage::Canvas(canvas)) => {
                let tex_id = self.bindings.canvas_index(canvas.as_raw());
                self.emit_rect(rect.rect, full_tex_rect(), tex_id, rect.fill.color);
            }
            Some(FillImage::NinePatchImage(image)) => {
                self.draw_nine_patch_rect(assets, rect.rect, rect.fill.color, *image);
            }
            Some(FillImage::SingleImage(image)) => {
                self.draw_textured_rect(rect.rect, rect.fill.color, *image);
            }
            None => {
                self.emit_rect(rect.rect, full_tex_rect(), 0, rect.fill.color);
            }
        }
    }

    fn draw_textured_rect(&mut self, rect: Rect<f32>, color: Color, image: Id<Image>) {
        let (atlas_id, tex_rect) = self
            .images
            .get(&self.atlases, image)
            .map(|(id, rect)| (Some(id), rect))
            .unwrap_or((None, full_tex_rect()));

        let tex_id = atlas_id.map(|v| self.bindings.atlas_index(v)).unwrap_or(0);

        self.emit_rect(rect, tex_rect, tex_id, color);
    }

    fn draw_nine_patch_rect(
        &mut self,
        assets: &Assets,
        rect: Rect<f32>,
        color: Color,
        image_id: Id<NinePatchImage>,
    ) {
        let image = match assets.get_by_id(image_id) {
            Some(v) => v,
            None => {
                return self.emit_rect(rect, full_tex_rect(), 0, color);
            }
        };

        let top_left_size = get_image_size(assets, image.top_left.id());
        let bottom_right_size = get_image_size(assets, image.top_left.id());

        let outer = rect;
        let inner = Rect::new(rect.min + top_left_size, rect.max - bottom_right_size);

        self.draw_textured_rect(inner, color, image.center.id());

        let rect = Rect::new(
            Vec2::new(inner.min.x, outer.min.y),
            Vec2::new(inner.max.x, inner.min.y),
        );
        self.draw_textured_rect(rect, color, image.top.id());

        let rect = Rect::new(
            Vec2::new(inner.min.x, inner.max.y),
            Vec2::new(inner.max.x, outer.max.y),
        );
        self.draw_textured_rect(rect, color, image.bottom.id());

        let rect = Rect::new(
            Vec2::new(outer.min.x, inner.min.y),
            Vec2::new(inner.min.x, inner.max.y),
        );
        self.draw_textured_rect(rect, color, image.left.id());

        let rect = Rect::new(
            Vec2::new(inner.max.x, inner.min.y),
            Vec2::new(outer.max.x, inner.max.y),
        );
        self.draw_textured_rect(rect, color, image.right.id());

        let rect = Rect::new(outer.min, inner.min);
        self.draw_textured_rect(rect, color, image.top_left.id());

        let rect = Rect::new(inner.max, outer.max);
        self.draw_textured_rect(rect, color, image.bottom_right.id());

        let rect = Rect::new(
            Vec2::new(inner.max.x, outer.min.y),
            Vec2::new(outer.max.x, inner.min.y),
        );
        self.draw_textured_rect(rect, color, image.top_right.id());

        let rect = Rect::new(
            Vec2::new(outer.min.x, inner.max.y),
            Vec2::new(inner.min.x, outer.max.y),
        );
        self.draw_textured_rect(rect, color, image.bottom_left.id());
    }

    fn draw_glyph(&mut self, cmd: &DrawGlyph) {
        let glyph = match self.glyphs.get(GlyphKey {
            font: cmd.font,
            glyph: cmd.glyph,
            size: cmd.size,
        }) {
            Some(v) => v,
            None => return,
        };

        let tex_rect = self.atlases.get_normalized_rect(&glyph.alloc);

        let size = glyph.size.cast::<f32>();
        let offset = glyph.offset + Vec2::new(0.0, -size.y);
        let rect = Rect::from_pos_extents(cmd.pos + offset, size);
        let tex_id = self.bindings.atlas_index(glyph.alloc.id.atlas_id);
        let color = Color {
            r: cmd.color.r + 2.0,
            ..cmd.color
        };

        self.emit_rect(rect, tex_rect, tex_id, color);
    }

    fn emit_rect(&mut self, rect: Rect<f32>, tex_rect: Rect<f32>, tex_id: u32, color: Color) {
        let i = self.batcher.next_vertex_index();
        self.batcher
            .emit_indices(&[i, i + 1, i + 2, i, i + 2, i + 3]);

        for (pos, tex) in rect.vertices().into_iter().zip(tex_rect.vertices()) {
            self.batcher.emit_vertex(Vertex {
                pos: self.batcher.state().view_proj.transform_point(pos),
                tex,
                tex_id,
                color,
            })
        }
    }

    fn encode_pass(
        &mut self,
        encoder: &mut CommandEncoder,
        clear_color: Option<Color>,
        canvas: &Canvas,
        main_view: &TextureView,
    ) {
        let vbuf = self.batcher.create_vertex_buffer(&self.device);
        let ibuf = self.batcher.create_index_buffer(&self.device);

        let (view, clear_color) = match canvas {
            Canvas::MainWindow => (main_view, clear_color.or(Some(Color::BLACK))),
            Canvas::Texture {
                view, has_cleared, ..
            } => {
                if has_cleared.load(Ordering::SeqCst) {
                    (view, clear_color)
                } else {
                    has_cleared.store(true, Ordering::SeqCst);
                    (view, clear_color.or(Some(Color::BLACK)))
                }
            }
        };

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: match clear_color {
                        Some(col) => LoadOp::Clear(wgpu::Color {
                            r: col.r as f64,
                            g: col.g as f64,
                            b: col.b as f64,
                            a: col.a as f64,
                        }),
                        None => LoadOp::Load,
                    },
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.set_index_buffer(ibuf.slice(..), IndexFormat::Uint32);

        pass.set_bind_group(0, self.bindings.bind_group(), &[]);
        pass.set_pipeline(self.pipelines.pipeline());

        for batch in self.batcher.batches() {
            pass.set_scissor_rect(
                batch.state.scissor.min.x,
                batch.state.scissor.min.y,
                batch.state.scissor.width(),
                batch.state.scissor.height(),
            );

            pass.draw_indexed(batch.indices.clone(), 0, 0..1);
        }
    }
}

fn full_tex_rect() -> Rect<f32> {
    Rect::new(Vec2::zero(), Vec2::new(1.0, 1.0))
}

fn get_image_size(assets: &Assets, id: Id<Image>) -> Vec2<f32> {
    assets
        .get_by_id(id)
        .map(|img| img.size.cast::<f32>())
        .unwrap_or_else(Vec2::zero)
}

fn projection_matrix(res: Vec2<u32>) -> Affine2<f32> {
    let res = res.cast::<f32>();
    Affine2::translation(Vec2::new(-1.0, 1.0)) * Affine2::scaling(Vec2::new(2.0, -2.0) / res)
}
