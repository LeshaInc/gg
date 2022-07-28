mod fps_counter;

use std::time::Instant;

use gg_assets::{Assets, DirSource, Handle};
use gg_graphics::{
    Backend, Color, FontCollection, GraphicsEncoder, TextHAlign, TextLayoutProperties,
    TextLayouter, TextProperties, TextVAlign,
};
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

    let font_collection: Handle<FontCollection> = assets.load("NotoColorEmoji.ttf");

    while !assets.contains(&font_collection) {
        assets.maintain();
    }

    let font = assets[&font_collection].faces[0].clone();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(LogicalSize::new(128.0, 128.0))
        .build(&event_loop)?;

    let mut backend = BackendImpl::new(&assets, &window)?;
    let main_canvas = backend.get_main_canvas();

    let mut fps_counter = FpsCounter::new(100);
    let mut frame_start = Instant::now();

    let mut ui = gg_ui::Driver::new();

    let mut text_layouter = TextLayouter::new();

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
            let ui_bounds = Rect::new(padding, size.cast::<f32>() - padding);
            let ui_ctx = UiContext {
                bounds: ui_bounds,
                assets: &assets,
                encoder: &mut encoder,
            };

            ui.run(build_ui(), ui_ctx);

            text_layouter.reset();
            text_layouter.set_props(&TextLayoutProperties {
                h_align: TextHAlign::Justify,
                v_align: TextVAlign::Start,
                ..Default::default()
            });

            let mut text_props = TextProperties {
                font: font.id(),
                size: 20.0,
                color: Color::WHITE,
            };

            let text = format!("fps: {}\n\n", fps_counter.fps());
            text_layouter.append(text_props, &text);

            let text = "😀😡🤯👺\n";
            text_layouter.append(text_props, &text);

            text_props.size = 128.0;
            text_layouter.append(text_props, &text);

            text_layouter.draw(&assets, &mut encoder, ui_bounds);

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
        views::rect([0.05, 0.0, 0.0])
            .max_width(100.0)
            .max_height(50.0),
        views::rect([0.0, 0.05, 0.0]).max_height(300.0),
        views::rect([0.0, 0.0, 0.05])
            .max_width(200.0)
            .max_height(30.0),
        views::hstack((
            views::rect([0.0, 0.05, 0.05]),
            views::rect([0.05, 0.0, 0.05]),
            views::vstack((
                views::rect([0.05, 0.05, 0.05]).max_height(10.0),
                views::rect([0.05, 0.05, 0.05]).max_height(10.0),
                views::rect([0.05, 0.05, 0.05]).max_height(10.0),
            )),
        ))
        .max_height(100.0),
    ))
}
