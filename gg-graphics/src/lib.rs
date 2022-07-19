mod backend;
mod canvas;
mod color;
mod command;
mod encoder;
mod font;
mod image;

pub use self::backend::Backend;
pub use self::canvas::{Canvas, RawCanvas};
pub use self::color::Color;
pub use self::command::{Command, CommandList, DrawGlyph, DrawRect, Fill, FillImage};
pub use self::encoder::GraphicsEncoder;
pub use self::font::{Font, GlyphIndex, GlyphMetrics, FontLoader};
pub use self::image::{Image, NinePatchImage, PngLoader};

// #[derive(Debug)]
// pub struct Graphics {
//     backend: Box<dyn Backend>,
// }

// impl Graphics {
//     pub fn new<B: Backend>(backend: B) -> Graphics {
//         Graphics {
//             backend: Box::new(backend),
//         }
//     }

//     pub fn create_canvas(&mut self, size: Vec2<u32>) -> Canvas {
//         self.backend.create_canvas(size)
//     }

//     pub fn submit(&mut self, commands: CommandList) {
//         self.backend.submit(commands);
//     }

//     pub fn present(&mut self) {
//         self.backend.present();
//     }
// }
