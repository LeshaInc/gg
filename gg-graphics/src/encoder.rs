use gg_math::{Affine2, Rect};

use crate::{Canvas, Color, Command, CommandList, DrawRect, Fill, FillImage};

#[derive(Clone, Debug)]
pub struct GraphicsEncoder {
    canvas: Canvas,
    list: Vec<Command>,
}

impl GraphicsEncoder {
    pub fn new(canvas: Canvas) -> GraphicsEncoder {
        GraphicsEncoder {
            canvas,
            list: Vec::new(),
        }
    }

    pub fn command(&mut self, command: impl Into<Command>) {
        self.list.push(command.into());
    }

    pub fn save(&mut self) {
        self.command(Command::Save);
    }

    pub fn restore(&mut self) {
        self.command(Command::Restore);
    }

    pub fn set_scissor(&mut self, rect: Rect<u32>) {
        self.command(Command::SetScissor(rect));
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
