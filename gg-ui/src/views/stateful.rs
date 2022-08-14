use gg_input::Event;
use gg_math::Vec2;

use crate::{Bounds, DrawCtx, Hover, LayoutCtx, LayoutHints, UpdateCtx, View};

pub fn stateful<D, S, VF, V>(state: S, view_factory: VF) -> Stateful<S, VF, V>
where
    VF: FnOnce(&S) -> V,
    V: View<(D, S)>,
{
    Stateful {
        view: None,
        view_factory: Some(view_factory),
        state,
    }
}

pub struct Stateful<S, VF, V> {
    view: Option<V>,
    view_factory: Option<VF>,
    state: S,
}

impl<S, VF, V> Stateful<S, VF, V>
where
    VF: FnOnce(&S) -> V,
{
    fn ensure_init(&mut self) {
        if let Some(factory) = self.view_factory.take() {
            self.view = Some(factory(&self.state));
        }
    }

    fn with_ctx<D, R>(
        &mut self,
        ctx: &mut UpdateCtx<D>,
        f: impl FnOnce(&mut Option<V>, &mut UpdateCtx<(D, S)>) -> R,
    ) -> R {
        self.ensure_init();

        take_mut::scoped::scope(|s| {
            let (data, data_hole) = s.take(ctx.data);
            let (state, state_hole) = s.take(&mut self.state);

            let mut combined_data = (data, state);
            let mut ctx = UpdateCtx {
                assets: ctx.assets,
                input: ctx.input,
                data: &mut combined_data,
                layer: ctx.layer,
            };

            let res = f(&mut self.view, &mut ctx);

            let (data, state) = combined_data;
            data_hole.fill(data);
            state_hole.fill(state);

            res
        })
    }
}

impl<D, S, VF, V> View<D> for Stateful<S, VF, V>
where
    VF: FnOnce(&S) -> V,
    V: View<(D, S)>,
{
    fn init(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        std::mem::swap(&mut self.state, &mut old.state);

        self.ensure_init();

        if let (Some(view), Some(old_view)) = (&mut self.view, &mut old.view) {
            view.init(old_view)
        } else {
            true
        }
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        self.ensure_init();

        if let Some(view) = &mut self.view {
            view.pre_layout(ctx)
        } else {
            LayoutHints::default()
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.ensure_init();

        if let Some(view) = &mut self.view {
            view.layout(ctx, size)
        } else {
            size
        }
    }

    fn hover(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) -> Hover {
        self.with_ctx(ctx, |view, ctx| {
            if let Some(view) = view {
                view.hover(ctx, bounds)
            } else {
                Hover::None
            }
        })
    }

    fn update(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) {
        self.with_ctx(ctx, |view, ctx| {
            if let Some(view) = view {
                view.update(ctx, bounds);
            }
        })
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) -> bool {
        self.with_ctx(ctx, |view, ctx| {
            if let Some(view) = view {
                view.handle(ctx, bounds, event)
            } else {
                false
            }
        })
    }

    fn draw(&mut self, ctx: &mut DrawCtx, bounds: Bounds) {
        self.ensure_init();

        if let Some(view) = &mut self.view {
            view.draw(ctx, bounds)
        }
    }
}
