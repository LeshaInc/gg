use eyre::Result;
use gg_assets::{Assets, DirSource};
use gg_graphics::{Backend, GraphicsEncoder};
use gg_graphics_impl::BackendImpl;
use gg_math::{Rect, Vec2};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::new();

    let source = DirSource::new("assets")?;
    let mut assets = Assets::new(source);

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(LogicalSize::new(128.0, 128.0))
        .build(&event_loop)?;

    let mut backend = BackendImpl::new(&window)?;
    let main_canvas = backend.get_main_canvas();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::MainEventsCleared => {
            let size = window.inner_size();
            backend.resize(Vec2::new(size.width, size.height));

            let mut encoder = GraphicsEncoder::new(main_canvas.clone());

            for x in 0..100 {
                let x = x as f32;
                let c = x / 100.0;
                encoder
                    .rect(Rect::new(
                        Vec2::new(x * 10.0, 0.),
                        Vec2::new(x * 10.0 + 10.0, 600.0),
                    ))
                    .fill_color([c, c, c]);
            }

            backend.submit(encoder.finish());

            backend.present(&mut assets);

            *control_flow = ControlFlow::Poll;
        }
        _ => (),
    });
}
