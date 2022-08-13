mod fps_counter;

use std::time::Instant;

use gg_assets::{Assets, DirSource};
use gg_graphics::{Backend, FontDb, GraphicsEncoder, TextLayouter};
use gg_graphics_impl::{BackendImpl, BackendSettings};
use gg_input::Input;
use gg_math::{Rect, Vec2};
use gg_ui::{views, AppendChild, UiAction, UiContext, View, ViewExt};
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
            let ui_bounds = Rect::from_min_max(padding, size.cast::<f32>() - padding);
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
    views::scrollable(
        views::vstack()
            .child(_build_ui(fps).min_height(300.0))
            .child(_build_ui(fps).min_height(300.0))
            .child(_build_ui(fps).min_height(300.0))
            .child(_build_ui(fps).min_height(300.0)),
    )
}

pub fn _build_ui(fps: f32) -> impl View<()> {
    views::vstack()
        .child(views::text(format!("fps: {:.2}", fps)))
        .child(
            views::hstack()
                .child(views::tooltip(
                    views::button("Button A", |_| println!("A")),
                    views::overlay()
                        .child(views::rect([0.0; 3]))
                        .child(views::tooltip(
                            views::text("test tooltip").wrap(false).padding(4.0),
                            views::overlay()
                                .min_width(300.0)
                                .min_height(30.0)
                                .child(views::rect([0.2, 0.0, 0.0]))
                                .child(views::text("mic check")),
                        )),
                ))
                .child(views::tooltip(
                    views::button("Button B", |_| println!("B")),
                    views::overlay().child(views::rect([0.0; 3])).child(
                        views::text("another test tooltip\n foobar")
                            .wrap(false)
                            .padding(4.0),
                    ),
                ))
                .child(views::button("Button Cool", |_| println!("C"))),
        )
        .child(
            views::hstack()
                .stretch(2.0)
                .child(views::scrollable(
                    views::vstack()
                        .padding([10.0, 5.0, 10.0, 2.5])
                        .child(views::text(TOP_LEFT))
                        .child(views::hstack().child(views::button("Test", |_| ())))
                        .child(views::text(LEFT)),
                ))
                .child(
                    views::scrollable(
                        views::text(RIGHT)
                            .padding([10.0, 2.5, 10.0, 5.0])
                            .min_width(500.0),
                    )
                    .max_width(300.0),
                ),
        )
        .child(
            views::hstack()
                .stretch(1.0)
                .child(views::rect([0.0, 0.05, 0.05]).stretch(1.0))
                .child(views::rect([0.05, 0.0, 0.05]).stretch(1.0))
                .child(
                    views::vstack()
                        .stretch(1.0)
                        .child(views::rect([0.05; 3]).stretch(1.0).max_height(10.0))
                        .child(views::rect([0.05; 3]).stretch(1.0).max_height(10.0))
                        .child(views::rect([0.05; 3]).stretch(1.0).max_height(10.0)),
                ),
        )
}

const TOP_LEFT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum";

const LEFT: &str = "It is not at present our business to treat of empirical illusory appearance (for example, optical illusion), which occurs in the empirical application of otherwise correct rules of the understanding, and in which the judgement is misled by the influence of imagination. Our purpose is to speak of transcendental illusory appearance, which influences principles—that are not even applied to experience, for in this case we should possess a sure test of their correctness—but which leads us, in disregard of all the warnings of criticism, completely beyond the empirical employment of the categories and deludes us with the chimera of an extension of the sphere of the pure understanding. We shall term those principles the application of which is confined entirely within the limits of possible experience, immanent; those, on the other hand, which transgress these limits, we shall call transcendent principles. But by these latter I do not understand principles of the transcendental use or misuse of the categories, which is in reality a mere fault of the judgement when not under due restraint from criticism, and therefore not paying sufficient attention to the limits of the sphere in which the pure understanding is allowed to exercise its functions; but real principles which exhort us to break down all those barriers, and to lay claim to a perfectly new field of cognition, which recognizes no line of demarcation. Thus transcendental and transcendent are not identical terms. The principles of the pure understanding, which we have already propounded, ought to be of empirical and not of transcendental use, that is, they are not applicable to any object beyond the sphere of experience. A principle which removes these limits, nay, which authorizes us to overstep them, is called transcendent. If our criticism can succeed in exposing the illusion in these pretended principles, those which are limited in their employment to the sphere of experience may be called, in opposition to the others, immanent principles of the pure understanding.

Logical illusion, which consists merely in the imitation of the form of reason (the illusion in sophistical syllogisms), arises entirely from a want of due attention to logical rules. So soon as the attention is awakened to the case before us, this illusion totally disappears. Transcendental illusion, on the contrary, does not cease to exist, even after it has been exposed, and its nothingness clearly perceived by means of transcendental criticism. Take, for example, the illusion in the proposition: “The world must have a beginning in time.” The cause of this is as follows. In our reason, subjectively considered as a faculty of human cognition, there exist fundamental rules and maxims of its exercise, which have completely the appearance of objective principles. Now from this cause it happens that the subjective necessity of a certain connection of our conceptions, is regarded as an objective necessity of the determination of things in themselves. This illusion it is impossible to avoid, just as we cannot avoid perceiving that the sea appears to be higher at a distance than it is near the shore, because we see the former by means of higher rays than the latter, or, which is a still stronger case, as even the astronomer cannot prevent himself from seeing the moon larger at its rising than some time afterwards, although he is not deceived by this illusion.

Transcendental dialectic will therefore content itself with exposing the illusory appearance in transcendental judgements, and guarding us against it; but to make it, as in the case of logical illusion, entirely disappear and cease to be illusion is utterly beyond its power. For we have here to do with a natural and unavoidable illusion, which rests upon subjective principles and imposes these upon us as objective, while logical dialectic, in the detection of sophisms, has to do merely with an error in the logical consequence of the propositions, or with an artificially constructed illusion, in imitation of the natural error. There is, therefore, a natural and unavoidable dialectic of pure reason—not that in which the bungler, from want of the requisite knowledge, involves himself, nor that which the sophist devises for the purpose of misleading, but that which is an inseparable adjunct of human reason, and which, even after its illusions have been exposed, does not cease to deceive, and continually to lead reason into momentary errors, which it becomes necessary continually to remove.";

const RIGHT: &str = "Nor 🙅🍺 again 😩😳 is 🤔 there anyone who 🔭 loves 💕🍑 or 💁💁 pursues or 😣💰 desires to 💰💰 obtain pain 😞😞 of 👏📰 itself, 👏👈 because it is 🏻 pain, 😩 but because 🚱💁 occasionally 🐶 circumstances ❌ occur 👻👻 in ⏬ which toil and 💮 pain 💥😩 can 💦🗑 procure him 👦 some 👨 great 🤤🌍 pleasure. 💦💦 To 💦 take 👀🐤 a 👌👌 trivial example, which 🎓 of 📆💰 us 💼 ever undertakes laborious physical 👊 exercise, except 😮 to 💦🚶 obtain some 🤔🈯 advantage from 💦 it? 😂💕";
