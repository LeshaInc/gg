use eyre::Result;
use gg_assets::{Assets, DirSource};
use gg_graphics::{Backend, GraphicsEncoder};
use gg_graphics_impl::BackendImpl;
use gg_math::{Affine2, Rect, Rotation2, Vec2};
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

    let canvas = backend.create_canvas(Vec2::new(200, 200));

    let mut time: f32 = 0.0;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::MainEventsCleared => {
            let size = window.inner_size();
            backend.resize(Vec2::new(size.width, size.height));

            let mut encoder = GraphicsEncoder::new(canvas.clone());

            for v in 0..10 {
                let v = v as f32;
                encoder.save();
                encoder.pre_transform(Affine2::rotation(Rotation2::from_angle((time + v).sin())));

                encoder
                    .rect(Rect::new(Vec2::new(50.0, 50.0), Vec2::new(150.0, 150.0)))
                    .fill_color([(time + v).cos() * 0.5 + 0.5; 3]);
                encoder.restore();
            }

            backend.submit(encoder.finish());

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

            encoder
                .rect(Rect::new(Vec2::new(200.0, 200.0), Vec2::new(400.0, 400.0)))
                .fill_image(&canvas);

            backend.submit(encoder.finish());

            backend.present(&mut assets);

            time += 1.0 / 60.0;
            *control_flow = ControlFlow::Poll;
        }
        _ => (),
    });
}
