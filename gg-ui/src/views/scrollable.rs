use gg_input::Event;
use gg_math::{Rect, Vec2};

use crate::{Bounds, DrawCtx, LayoutCtx, LayoutHints, UiAction, UpdateCtx, View};

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

impl<V> Scrollable<V> {
    fn inner_bounds(&self, outer: Bounds) -> Bounds {
        outer.with_scissor(outer.rect).child(Rect::new(
            outer.rect.min + self.offset.floor(),
            self.inner_size,
        ))
    }
}

impl<D, V: View<D>> View<D> for Scrollable<V> {
    fn init(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        self.hints = old.hints;
        self.offset = old.offset;
        self.target_offset = old.target_offset;
        self.inner_size = old.inner_size;

        let diff = self.target_offset - self.offset;
        self.offset += diff.map(|v| (v.abs() * 0.2).copysign(v));

        self.view.init(&mut old.view)
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        self.hints = self.view.pre_layout(ctx);
        self.inner_size = self.hints.min_size;
        LayoutHints {
            min_size: Vec2::zero(),
            stretch: 1.0,
            ..self.hints
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.inner_size = size.fclamp(self.hints.min_size, self.hints.max_size);
        self.inner_size = self.view.layout(ctx, self.inner_size);

        let size = size.fmin(self.inner_size);

        let min = size - self.inner_size;
        let max = Vec2::zero();
        self.offset = self.offset.fclamp(min, max);
        self.target_offset = self.target_offset.fclamp(min, max);

        size
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) {
        if let Event::Scroll(ev) = event {
            if bounds.clip_rect.contains(ctx.input.mouse_pos()) {
                let delta = if ctx.input.is_action_pressed(UiAction::TransposeScroll) {
                    Vec2::new(ev.delta.y, ev.delta.x)
                } else {
                    ev.delta
                };

                self.target_offset += delta * 60.0;
                self.target_offset = self
                    .target_offset
                    .fmax(bounds.rect.size() - self.inner_size)
                    .fmin(Vec2::zero());
            }
        }

        self.view.handle(ctx, self.inner_bounds(bounds), event)
    }

    fn draw(&mut self, ctx: &mut DrawCtx, outer_bounds: Bounds) {
        let inner_bounds = self.inner_bounds(outer_bounds);
        let outer = outer_bounds.rect;
        let inner = inner_bounds.rect;

        ctx.encoder.save();
        ctx.encoder.set_scissor(outer.cast());

        self.view.draw(ctx, inner_bounds);

        let mut thumb_factor = outer.size() / inner.size();
        if thumb_factor.x < 1.0 && thumb_factor.y < 1.0 {
            thumb_factor = (outer.size() - Vec2::new(7.0, 0.0)) / inner.size();
        }

        let thumb_size = outer.size() * thumb_factor;
        let thumb_offset = -self.offset * thumb_factor;

        if thumb_factor.x < 1.0 {
            ctx.encoder
                .rect([
                    outer.min.x + thumb_offset.x,
                    outer.max.y - 4.0,
                    thumb_size.x,
                    3.0,
                ])
                .fill_color([1.0, 0.0, 0.0, 0.3]);
        }

        if thumb_factor.y < 1.0 {
            ctx.encoder
                .rect([
                    outer.max.x - 4.0,
                    outer.min.y + thumb_offset.y,
                    3.0,
                    thumb_size.y,
                ])
                .fill_color([1.0, 0.0, 0.0, 0.3]);
        }

        ctx.encoder.restore();
    }
}
