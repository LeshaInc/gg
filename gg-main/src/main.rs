mod fps_counter;

use std::time::Instant;

use gg_assets::{Assets, DirSource};
use gg_graphics::{
    Backend, Color, FontDb, FontFamily, FontStyle, FontWeight, GraphicsEncoder, TextHAlign,
    TextLayoutProperties, TextLayouter, TextProperties, TextVAlign,
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

    let mut fonts = FontDb::new();
    fonts.add_collection(&assets.load("fonts/OpenSans-Regular.ttf"));
    fonts.add_collection(&assets.load("fonts/OpenSans-Italic.ttf"));
    fonts.add_collection(&assets.load("fonts/OpenSans-Bold.ttf"));
    fonts.add_collection(&assets.load("fonts/OpenSans-BoldItalic.ttf"));
    fonts.add_collection(&assets.load("fonts/NotoColorEmoji.ttf"));
    fonts.add_collection(&assets.load("fonts/NotoSans-Regular.ttf"));
    fonts.add_collection(&assets.load("fonts/NotoSansJP-Regular.otf"));

    let font_family = FontFamily::new("Open Sans")
        .push("Noto Color Emoji")
        .push("Noto Sans")
        .push("Noto Sans JP");

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
            fonts.update(&assets);

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
                h_align: TextHAlign::Start,
                v_align: TextVAlign::Start,
                ..Default::default()
            });

            let mut text_props = TextProperties {
                font_family: font_family.clone(),
                weight: FontWeight::Normal,
                style: FontStyle::Normal,
                size: 20.0,
                color: Color::WHITE,
            };

            let text = format!("fps: {}\n\n", fps_counter.fps());
            text_layouter.append(&text_props, &text);

            let text = "सभमन 日本語\n\n";
            text_layouter.append(&text_props, &text);

            let text = "kill⁠😀⁠me hello 🤬🪖\n";
            text_layouter.append(&text_props, &text);

            text_props.style = FontStyle::Italic;
            text_layouter.append(&text_props, &text);

            text_props.weight = FontWeight::Bold;
            text_props.style = FontStyle::Normal;
            text_layouter.append(&text_props, &text);

            text_props.weight = FontWeight::Bold;
            text_props.style = FontStyle::Italic;
            text_layouter.append(&text_props, &text);

            text_layouter.draw(&assets, &fonts, &mut encoder, ui_bounds);

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
