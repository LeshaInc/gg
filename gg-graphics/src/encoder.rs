use gg_math::{Affine2, Rect, Vec2};

use crate::{Canvas, Color, Command, CommandList, DrawGlyph, DrawRect, Fill, FillImage};

#[derive(Clone, Debug)]
pub struct GraphicsEncoder {
    canvas: Canvas,
    list: Vec<Command>,
    saved_scissors: Vec<Rect<f32>>,
    scissor: Rect<f32>,
}

impl GraphicsEncoder {
    pub fn new(canvas: &Canvas) -> GraphicsEncoder {
        GraphicsEncoder {
            canvas: canvas.clone(),
            list: Vec::new(),
            saved_scissors: Vec::new(),
            scissor: full_scissor(),
        }
    }

    pub fn new_recycled(canvas: &Canvas, mut list: CommandList) -> GraphicsEncoder {
        list.list.clear();
        GraphicsEncoder {
            canvas: canvas.clone(),
            list: list.list,
            saved_scissors: Vec::new(),
            scissor: full_scissor(),
        }
    }

    pub fn command(&mut self, command: impl Into<Command>) {
        let command = command.into();

        match command {
            Command::SetScissor(rect) => self.scissor = rect.f_intersection(&self.scissor),
            Command::ClearScissor => self.scissor = full_scissor(),
            Command::Save => self.saved_scissors.push(self.scissor),
            Command::Restore => {
                self.scissor = self.saved_scissors.pop().unwrap_or_else(full_scissor);
            }
            _ => {}
        }

        self.list.push(command);
    }

    pub fn save(&mut self) {
        self.command(Command::Save);
    }

    pub fn restore(&mut self) {
        self.command(Command::Restore);
    }

    pub fn set_scissor(&mut self, rect: Rect<f32>) {
        self.command(Command::SetScissor(rect));
    }

    pub fn get_scissor(&self) -> Rect<f32> {
        self.scissor
    }

    pub fn clear_scissor(&mut self) {
        self.command(Command::ClearScissor);
    }

    pub fn pre_transform(&mut self, affine: Affine2<f32>) {
        self.command(Command::PreTransform(affine));
    }

    pub fn post_transform(&mut self, affine: Affine2<f32>) {
        self.command(Command::PostTransform(affine));
    }

    pub fn clear(&mut self, color: impl Into<Color>) {
        self.command(Command::Clear(color.into()));
    }

    pub fn rect(&mut self, rect: impl Into<Rect<f32>>) -> RectEncoder<'_> {
        RectEncoder {
            encoder: self,
            cmd: DrawRect {
                rect: rect.into(),
                fill: Fill {
                    color: Color::WHITE,
                    image: None,
                },
            },
        }
    }

    pub fn glyph(&mut self, glyph: DrawGlyph) {
        self.command(Command::DrawGlyph(glyph));
    }

    pub fn finish(self) -> CommandList {
        CommandList {
            canvas: self.canvas,
            list: self.list,
        }
    }
}

#[derive(Debug)]
pub struct RectEncoder<'a> {
    encoder: &'a mut GraphicsEncoder,
    cmd: DrawRect,
}

impl RectEncoder<'_> {
    pub fn fill_color(mut self, color: impl Into<Color>) -> Self {
        self.cmd.fill.color = color.into();
        self
    }

    pub fn fill_image(mut self, image: impl Into<FillImage>) -> Self {
        self.cmd.fill.image = Some(image.into());
        self
    }
}

impl Drop for RectEncoder<'_> {
    fn drop(&mut self) {
        self.encoder.command(Command::DrawRect(self.cmd.clone()));
    }
}

fn full_scissor() -> Rect<f32> {
    Rect::new(Vec2::zero(), Vec2::splat(f32::INFINITY))
}
