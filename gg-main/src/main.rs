mod fps_counter;

use std::time::Instant;

use gg_assets::{Assets, DirSource};
use gg_graphics::{Backend, FontDb, GraphicsEncoder, TextLayouter};
use gg_graphics_impl::{BackendImpl, BackendSettings};
use gg_input::Input;
use gg_math::{Rect, Vec2};
use gg_ui::{views, UiAction, UiContext, View, ViewExt};
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
    let mut input = Input::new();
    input.register_action::<UiAction>();
    input.load("input.json")?;

    let mut fonts = FontDb::new();
    fonts.add_collection(&assets.load("fonts/OpenSans-Regular.ttf"));
    fonts.add_collection(&assets.load("fonts/OpenSans-Italic.ttf"));
    fonts.add_collection(&assets.load("fonts/OpenSans-Bold.ttf"));
    fonts.add_collection(&assets.load("fonts/OpenSans-BoldItalic.ttf"));
    fonts.add_collection(&assets.load("fonts/NotoColorEmoji.ttf"));
    fonts.add_collection(&assets.load("fonts/NotoSans-Regular.ttf"));
    fonts.add_collection(&assets.load("fonts/NotoSansJP-Regular.otf"));

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(LogicalSize::new(128.0, 128.0))
        .build(&event_loop)?;

    let settings = BackendSettings {
        vsync: true,
        prefer_low_power_gpu: true,
        image_cell_size: Vec2::splat(8),
    };

    let mut backend = BackendImpl::new(settings, &assets, &window)?;
    let main_canvas = backend.get_main_canvas();

    let mut fps_counter = FpsCounter::new(300);
    let mut frame_start = Instant::now();

    let mut ui = gg_ui::Driver::new();
    let mut text_layouter = TextLayouter::new();

    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(_) => {
            input.begin_frame();
        }
        Event::WindowEvent { event, .. } => {
            if event == WindowEvent::CloseRequested {
                *control_flow = ControlFlow::Exit;
            }

            input.process_event(event);
        }
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
                fonts: &fonts,
                text_layouter: &mut text_layouter,
                encoder: &mut encoder,
                input: &input,
            };

            ui.run(build_ui(fps_counter.fps()), ui_ctx, &mut ());

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

pub fn build_ui(fps: f32) -> impl View<()> {
    views::vstack((
        views::text(format!("fps: {:.2}", fps)),
        views::hstack((
            views::button("Button A", |_| println!("A")),
            views::button("Button B", |_| println!("B")),
            views::button("Button C", |_| println!("C")),
        )),
        views::hstack((
            views::padding([10.0, 5.0, 10.0, 0.0], views::text(LEFT)).set_stretch(1.0),
            views::padding([10.0, 0.0, 10.0, 5.0], views::text(RIGHT)).set_stretch(1.0),
        )),
        views::hstack((
            views::rect([0.0, 0.05, 0.05]).set_stretch(1.0),
            views::rect([0.05, 0.0, 0.05]).set_stretch(1.0),
            views::vstack((
                views::rect([0.05, 0.05, 0.05])
                    .set_stretch(1.0)
                    .max_height(10.0),
                views::rect([0.05, 0.05, 0.05])
                    .set_stretch(1.0)
                    .max_height(10.0),
                views::rect([0.05, 0.05, 0.05])
                    .set_stretch(1.0)
                    .max_height(10.0),
            ))
            .set_stretch(1.0),
        ))
        .set_stretch(1.0),
    ))
}

const LEFT: &str = "But 🐬😅 I must explain to 💦🙅 you 👉 how all 😱😎 this mistaken idea 👌 of 🎆😂 denouncing pleasure 💋 and 💰 praising pain 😧 was 👏💮 born and I 👁 will 😩 give you 🚫 a complete ✅ account of 🌈 the system, 🤣 and 👏 expound the 👧👌 actual teachings of 👨💦 the great explorer of 🌈💦 the truth, 🙌 the master-builder 🥇🥇 of 👏🚨 human ♀ happiness. 🙏😁 No one ♿☝ rejects, dislikes, or 🅱 avoids pleasure 😝 itself, 👈👈 because it 😂 is 💦👊 pleasure, 😩💦 but ";

const RIGHT: &str = "Nor 🙅🍺 again 😩😳 is 🤔 there anyone who 🔭 loves 💕🍑 or 💁💁 pursues or 😣💰 desires to 💰💰 obtain pain 😞😞 of 👏📰 itself, 👏👈 because it is 🏻 pain, 😩 but because 🚱💁 occasionally 🐶 circumstances ❌ occur 👻👻 in ⏬ which toil and 💮 pain 💥😩 can 💦🗑 procure him 👦 some 👨 great 🤤🌍 pleasure. 💦💦 To 💦 take 👀🐤 a 👌👌 trivial example, which 🎓 of 📆💰 us 💼 ever undertakes laborious physical 👊 exercise, except 😮 to 💦🚶 obtain some 🤔🈯 advantage from 💦 it? 😂💕";
