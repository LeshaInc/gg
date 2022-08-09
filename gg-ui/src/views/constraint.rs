use gg_math::{Rect, Vec2};

use crate::{
    AppendChild, DrawCtx, Event, HandleCtx, IntoViewSeq, LayoutCtx, LayoutHints, SetChildren, View,
};

pub fn constrain<V, C>(view: V, constraint: C) -> ConstraintView<V, C> {
    ConstraintView { view, constraint }
}

pub struct ConstraintView<V, C> {
    view: V,
    constraint: C,
}

impl<D, V, VC, C> AppendChild<D, VC> for ConstraintView<V, C>
where
    V: View<D> + AppendChild<D, VC>,
    VC: View<D>,
    C: Constraint,
{
    type Output = ConstraintView<V::Output, C>;

    fn child(self, child: VC) -> Self::Output {
        ConstraintView {
            view: self.view.child(child),
            constraint: self.constraint,
        }
    }
}

impl<D, V, C, Cons> SetChildren<D, C> for ConstraintView<V, Cons>
where
    V: View<D> + SetChildren<D, C>,
    C: IntoViewSeq<D>,
    Cons: Constraint,
{
    type Output = ConstraintView<V::Output, Cons>;

    fn children(self, children: C) -> Self::Output {
        ConstraintView {
            view: self.view.children(children),
            constraint: self.constraint,
        }
    }
}

impl<D, V, C> View<D> for ConstraintView<V, C>
where
    V: View<D>,
    C: Constraint,
{
    fn update(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        self.view.update(&mut old.view) || old.constraint != self.constraint
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        let mut hints = self.view.pre_layout(ctx);
        self.constraint.constrain(&mut hints);
        hints
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.view.layout(ctx, size)
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>) {
        self.view.draw(ctx, bounds);
    }

    fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event) {
        self.view.handle(ctx, bounds, event);
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
