mod fps_counter;

use std::io::BufRead;
use std::path::PathBuf;
use std::time::Instant;

use eyre::Result;
use gg_assets::{
    Asset, Assets, BytesAssetLoader, DirSource, Handle, Id, LoaderCtx, LoaderRegistry,
};
use gg_graphics::{Backend, DrawGlyph, Font, GraphicsEncoder, Image};
use gg_graphics_impl::BackendImpl;
use gg_math::Vec2;
use rand::Rng;
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

    let pokemon_list: Handle<FileList> = assets.load("pokemon/list.txt");
    let mut pokemons = Vec::new();

    let font: Handle<Font> = assets.load("OpenSans-Regular.ttf");

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .with_inner_size(LogicalSize::new(128.0, 128.0))
        .build(&event_loop)?;

    let mut backend = BackendImpl::new(&assets, &window)?;
    let main_canvas = backend.get_main_canvas();

    let mut rng = rand::thread_rng();
    let mut fps_counter = FpsCounter::new(100);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::MainEventsCleared => {
            let frame_start = Instant::now();

            assets.maintain();

            let mut to_load = Vec::new();
            if let Some(list) = assets.get(&pokemon_list) {
                for _ in 0..100 {
                    let idx = rng.gen_range(0..list.files.len());
                    let file = &list.files[idx];
                    to_load.push(file);
                }
            }

            for file in to_load {
                let pokemon: Handle<Image> = assets.load(&file);
                let pos = Vec2::new(rng.gen_range(-50.0..1920.0), rng.gen_range(-50.0..1080.0));
                pokemons.push((pokemon, pos));
            }

            let size = window.inner_size();
            backend.resize(Vec2::new(size.width, size.height));

            let mut encoder = GraphicsEncoder::new(&main_canvas);

            for (img, pos) in &pokemons {
                encoder
                    .rect([pos.x, pos.y, 64.0, 64.0])
                    .fill_image(img)
                    .fill_color([1.0, 1.0, 1.0, 0.1]);
            }

            let text = format!("fps: {}", fps_counter.fps());
            let pos = Vec2::new(20.0, 20.0);
            draw_text(&assets, &mut encoder, font.id(), pos, 20.0, &text);

            let text = format!("spf: {}", fps_counter.spf());
            let pos = Vec2::new(20.0, 45.0);
            draw_text(&assets, &mut encoder, font.id(), pos, 20.0, &text);

            let text = format!("sprites: {}", pokemons.len());
            let pos = Vec2::new(20.0, 70.0);
            draw_text(&assets, &mut encoder, font.id(), pos, 20.0, &text);

            backend.submit(encoder.finish());
            backend.present(&mut assets);

            fps_counter.add_sample(frame_start.elapsed());
            *control_flow = ControlFlow::Poll;
        }
        _ => (),
    });
}

struct FileList {
    files: Vec<PathBuf>,
}

impl Asset for FileList {
    fn register_loaders(registry: &mut LoaderRegistry) {
        registry.add(FileListLoader);
    }
}

struct FileListLoader;

#[async_trait::async_trait]
impl BytesAssetLoader<FileList> for FileListLoader {
    async fn load(&self, _: &mut LoaderCtx, bytes: Vec<u8>) -> Result<FileList> {
        let mut files = Vec::new();
        for line in bytes.lines() {
            files.push(PathBuf::from(line?));
        }
        Ok(FileList { files })
    }
}

fn draw_text(
    assets: &Assets,
    encoder: &mut GraphicsEncoder,
    font_id: Id<Font>,
    mut pos: Vec2<f32>,
    size: f32,
    text: &str,
) {
    let font = match assets.get_by_id(font_id) {
        Some(v) => v,
        None => return,
    };

    let mut prev_glyph = None;
    for c in text.chars() {
        let glyph = font.lookup_glyph(c);

        if let Some(left) = prev_glyph {
            pos.x += font.kern(left, glyph, size).unwrap_or(0.0);
        }

        encoder.glyph(DrawGlyph {
            font: font_id,
            glyph,
            size,
            pos,
            color: [1.0; 4].into(),
        });

        pos.x += font.glyph_metrics(glyph, 24.0).advance();
        prev_glyph = Some(glyph);
    }
}
