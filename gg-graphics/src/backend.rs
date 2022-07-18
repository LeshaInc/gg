use gg_assets::Assets;
use gg_math::Vec2;

use crate::command::CommandList;
use crate::Canvas;

pub trait Backend: Send + Sync + 'static {
    fn get_main_canvas(&self) -> Canvas;

    fn create_canvas(&mut self, size: Vec2<u32>) -> Canvas;

    fn submit(&mut self, commands: CommandList);

    fn resize(&mut self, new_resolution: Vec2<u32>);

    fn present(&mut self, assets: &mut Assets);
}
