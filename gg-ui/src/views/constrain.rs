use gg_math::Vec2;

use crate::{
    AppendChild, Bounds, DrawCtx, Event, Hover, IntoViewSeq, LayoutCtx, LayoutHints, SetChildren,
    UpdateCtx, View,
};

pub fn constrain<V, C>(view: V, constraint: C) -> Constrain<V, C> {
    Constrain { view, constraint }
}

pub struct Constrain<V, C> {
    view: V,
    constraint: C,
}

impl<D, V, VC, C> AppendChild<D, VC> for Constrain<V, C>
where
    V: View<D> + AppendChild<D, VC>,
    VC: View<D>,
    C: Constraint,
{
    type Output = Constrain<V::Output, C>;

    fn child(self, child: VC) -> Self::Output {
        Constrain {
            view: self.view.child(child),
            constraint: self.constraint,
        }
    }
}

impl<D, V, C, Cons> SetChildren<D, C> for Constrain<V, Cons>
where
    V: View<D> + SetChildren<D, C>,
    C: IntoViewSeq<D>,
    Cons: Constraint,
{
    type Output = Constrain<V::Output, Cons>;

    fn children(self, children: C) -> Self::Output {
        Constrain {
            view: self.view.children(children),
            constraint: self.constraint,
        }
    }
}

impl<D, V, C> View<D> for Constrain<V, C>
where
    V: View<D>,
    C: Constraint,
{
    fn init(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        self.view.init(&mut old.view) || old.constraint != self.constraint
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        let mut hints = self.view.pre_layout(ctx);
        self.constraint.constrain(&mut hints);
        hints
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.view.layout(ctx, size)
    }

    fn hover(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) -> Hover {
        self.view.hover(ctx, bounds)
    }

    fn update(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) {
        self.view.update(ctx, bounds);
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) {
        self.view.handle(ctx, bounds, event);
    }

    fn draw(&mut self, ctx: &mut DrawCtx, bounds: Bounds) {
        self.view.draw(ctx, bounds);
    }
}

pub trait Constraint: PartialEq {
    fn constrain(&self, hints: &mut LayoutHints);
}

#[derive(PartialEq)]
pub struct MinWidth(pub f32);

impl Constraint for MinWidth {
    fn constrain(&self, hints: &mut LayoutHints) {
        hints.min_size.x = hints.min_size.x.max(self.0)
    }
}

#[derive(PartialEq)]
pub struct MinHeight(pub f32);

impl Constraint for MinHeight {
    fn constrain(&self, hints: &mut LayoutHints) {
        hints.min_size.y = hints.min_size.y.max(self.0)
    }
}

#[derive(PartialEq)]
pub struct MaxWidth(pub f32);

impl Constraint for MaxWidth {
    fn constrain(&self, hints: &mut LayoutHints) {
        hints.max_size.x = hints.max_size.x.min(self.0)
    }
}

#[derive(PartialEq)]
pub struct MaxHeight(pub f32);

impl Constraint for MaxHeight {
    fn constrain(&self, hints: &mut LayoutHints) {
        hints.max_size.y = hints.max_size.y.min(self.0)
    }
}

#[derive(PartialEq)]
pub struct Stretch(pub f32);

impl Constraint for Stretch {
    fn constrain(&self, hints: &mut LayoutHints) {
        hints.stretch = self.0;
    }
}
