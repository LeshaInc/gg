use gg_input::Event;
use gg_math::{Rect, Vec2};

use crate::{DrawCtx, HandleCtx, LayoutCtx, LayoutHints, UiAction, View};

pub fn scrollable<V>(view: V) -> Scrollable<V> {
    Scrollable {
        view,
        hints: LayoutHints::default(),
        offset: Vec2::zero(),
        target_offset: Vec2::zero(),
        inner_size: Vec2::zero(),
    }
}

pub struct Scrollable<V> {
    view: V,
    hints: LayoutHints,
    offset: Vec2<f32>,
    target_offset: Vec2<f32>,
    inner_size: Vec2<f32>,
}

impl<D, V: View<D>> View<D> for Scrollable<V> {
    fn update(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        self.offset = old.offset;
        self.target_offset = old.target_offset;
        self.inner_size = old.inner_size;

        let diff = self.target_offset - self.offset;
        self.offset += diff.map(|v| (v.abs() * 0.1).copysign(v));

        self.view.update(&mut old.view)
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        self.hints = self.view.pre_layout(ctx);
        self.inner_size = self.hints.min_size;
        LayoutHints {
            min_size: Vec2::zero(),
            stretch: 1.0,
            ..self.hints
        }
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.inner_size = size.fclamp(self.hints.min_size, self.hints.max_size);
        self.inner_size = self.view.layout(ctx, self.inner_size);

        let size = size.fmin(self.inner_size);

        let min = size - self.inner_size;
        let max = Vec2::zero();
        self.offset = self.offset.fclamp(min, max);
        self.target_offset = self.target_offset.fclamp(min, max);

        size
    }

    fn draw(&mut self, mut ctx: DrawCtx, bounds: Rect<f32>) {
        ctx.encoder.save();
        ctx.encoder.set_scissor(bounds.cast());
        let inner = Rect::from_pos_extents(bounds.min + self.offset.floor(), self.inner_size);
        self.view.draw(ctx.reborrow(), inner);
        ctx.encoder.restore();
    }

    fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event) {
        if let Event::Scroll(ev) = event {
            if bounds.contains(ctx.input.mouse_pos()) {
                let delta = if ctx.input.is_action_pressed(UiAction::TransposeScroll) {
                    Vec2::new(ev.delta.y, ev.delta.x)
                } else {
                    ev.delta
                };

                self.target_offset += delta * 50.0;
                self.target_offset = self
                    .target_offset
                    .fmax(bounds.extents() - self.inner_size)
                    .fmin(Vec2::zero());
            }
        }

        let inner = Rect::from_pos_extents(bounds.min + self.offset.floor(), self.inner_size);
        self.view.handle(ctx, inner, event)
    }
}
