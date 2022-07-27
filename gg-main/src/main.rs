mod fps_counter;

use std::time::Instant;

use gg_assets::{Assets, DirSource, Handle, Id};
use gg_graphics::{Backend, DrawGlyph, Font, GraphicsEncoder};
use gg_graphics_impl::BackendImpl;
use gg_math::{Rect, Vec2};
use gg_ui::{views, UiContext, View, ViewExt};
use gg_util::eyre::Result;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use self::fps_counter::FpsCounter;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::new();

    let source = DirSource::new("assets")?;
    let mut assets = Assets::new(source);

    let font: Handle<Font> = assets.load("OpenSans-Regular.ttf");

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(LogicalSize::new(128.0, 128.0))
        .build(&event_loop)?;

    let mut backend = BackendImpl::new(&assets, &window)?;
    let main_canvas = backend.get_main_canvas();

    let mut fps_counter = FpsCounter::new(100);
    let mut frame_start = Instant::now();

    let mut ui = gg_ui::Driver::new();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::RedrawRequested(_) => {
            assets.maintain();

            let size = window.inner_size();
            let size = Vec2::new(size.width, size.height);
            backend.resize(size);

            let mut encoder = GraphicsEncoder::new(&main_canvas);

            encoder.clear([0.02; 3]);

            let padding = Vec2::splat(30.0);
            let ui_ctx = UiContext {
                bounds: Rect::new(padding, size.cast::<f32>() - padding),
                assets: &assets,
                encoder: &mut encoder,
            };

            ui.run(build_ui(), ui_ctx);

            let text = format!("fps: {}", fps_counter.fps());
            let pos = Vec2::new(20.0, 20.0);
            draw_text(&assets, &mut encoder, font.id(), pos, 20.0, &text);

            backend.submit(encoder.finish());
            backend.present(&mut assets);

            fps_counter.add_sample(frame_start.elapsed());
            frame_start = Instant::now();

            window.request_redraw();
            *control_flow = ControlFlow::Poll;
        }
        _ => (),
    });
}

pub fn build_ui() -> impl View<()> {
    views::vstack((
        views::rect([1.0, 0.0, 0.0])
            .max_width(100.0)
            .max_height(50.0),
        views::rect([0.0, 1.0, 0.0]).max_height(300.0),
        views::rect([0.0, 0.0, 1.0])
            .max_width(200.0)
            .max_height(30.0),
        views::hstack((
            views::rect([0.0, 1.0, 1.0]),
            views::rect([1.0, 0.0, 1.0]),
            views::vstack((
                views::rect([1.0, 1.0, 1.0]).max_height(10.0),
                views::rect([1.0, 1.0, 1.0]).max_height(10.0),
                views::rect([1.0, 1.0, 1.0]).max_height(10.0),
            )),
        ))
        .max_height(100.0),
    ))
}

fn draw_text(
    assets: &Assets,
    encoder: &mut GraphicsEncoder,
    font_id: Id<Font>,
    mut cursor: Vec2<f32>,
    size: f32,
    text: &str,
) {
    let font = match assets.get_by_id(font_id) {
        Some(v) => v,
        None => return,
    };

    for glyph in font.shape(size, text) {
        encoder.glyph(DrawGlyph {
            font: font_id,
            glyph: glyph.glyph,
            size,
            pos: cursor + glyph.offset,
            color: [1.0; 4].into(),
        });

        cursor.x += glyph.advance.x;
    }
}
