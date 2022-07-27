mod fps_counter;

use std::time::Instant;

use gg_assets::{Assets, DirSource, Handle};
use gg_graphics::{
    Backend, Color, Font, GraphicsEncoder, TextLayoutProperties, TextLayouter, TextProperties,
};
use gg_graphics_impl::BackendImpl;
use gg_math::{Rect, Vec2};
use gg_ui::{views, View, ViewExt};
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

    // let mut ui = gg_ui::Driver::new();

    let mut text_layouter = TextLayouter::new();

    let text_props = TextProperties {
        font: font.id(),
        size: 20.0,
        color: Color::WHITE,
    };

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
            // let ui_ctx = UiContext {
            //     bounds: ui_bounds,
            //     assets: &assets,
            //     encoder: &mut encoder,
            // };

            // ui.run(build_ui(), ui_ctx);

            text_layouter.set_props(&TextLayoutProperties {
                max_size: ui_bounds.extents(),
                line_height: 1.2,
            });

            text_layouter.reset();

            let text = format!("fps: {}\n\n", fps_counter.fps());
            text_layouter.append(text_props, &text);

            let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Donec quis libero eros. Nam id risus pharetra, aliquam nisi quis, commodo elit. Morbi elementum fringilla elit id mollis. Ut sed neque condimentum, volutpat libero at, iaculis mauris. Duis est mauris, sagittis nec lacus vitae, suscipit pretium est. Mauris vel sapien nec nulla aliquam blandit sed in elit. Fusce gravida massa massa, id condimentum urna accumsan vel. Phasellus imperdiet quis quam eget euismod. Maecenas eget tempus enim. Nullam tincidunt ut magna vel malesuada. Proin auctor, enim ut tincidunt vehicula, ligula enim tristique turpis, sed ullamcorper eros dui et turpis. Mauris eget bibendum nibh, non convallis elit.

Cras pulvinar sapien id sapien malesuada, a auctor mi mollis. Morbi porta nunc vitae rutrum laoreet. Pellentesque vehicula lobortis nulla, id dictum ex pellentesque et. Nam vel libero nunc. Suspendisse facilisis eros eu venenatis eleifend. Donec vitae iaculis ipsum. Nam congue mi quis vehicula scelerisque.

Aenean mollis, ipsum sed pellentesque fringilla, risus ex maximus tortor, ut vulputate mauris justo vel dolor. Etiam imperdiet nibh non enim accumsan, non pulvinar diam lobortis. Curabitur euismod ac nisl a lacinia. Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Proin pretium, neque porta fermentum congue, turpis velit tincidunt tellus, sit amet venenatis metus nunc nec quam. Aliquam quis metus pretium, feugiat diam sit amet, tristique enim. Phasellus ex leo, aliquam sit amet sapien consequat, egestas pulvinar nisi. Donec sit amet condimentum odio. Aenean elementum dignissim metus sit amet porttitor. Proin accumsan ut sem quis gravida. ";
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
