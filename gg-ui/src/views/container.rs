use std::marker::PhantomData;

use gg_input::Event;
use gg_math::{Rect, Vec2};

use crate::view_seq::{Append, HasMetaSeq};
use crate::{
    AppendChild, Bounds, Hover, IntoViewSeq, LayoutCtx, LayoutHints, SetChildren, UpdateCtx, View,
    ViewSeq,
};

#[derive(Clone, Copy)]
pub struct ChildMeta {
    pub hints: LayoutHints,
    pub changed: bool,
    pub pos: Vec2<f32>,
    pub size: Vec2<f32>,
    pub hover: Hover,
}

impl Default for ChildMeta {
    fn default() -> ChildMeta {
        ChildMeta {
            hints: LayoutHints::default(),
            changed: true,
            pos: Vec2::zero(),
            size: Vec2::zero(),
            hover: Hover::None,
        }
    }
}

pub trait Layout<D, C> {
    fn pre_layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut C,
        meta: &mut [ChildMeta],
    ) -> LayoutHints;

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut C,
        meta: &mut [ChildMeta],
        adviced: Vec2<f32>,
    ) -> Vec2<f32>;

    fn allow_multi_hover(&self) -> bool {
        false
    }
}

pub struct Container<D, L, C>
where
    L: Layout<D, C>,
    C: HasMetaSeq<ChildMeta>,
{
    phantom: PhantomData<fn(&mut D)>,
    children: C,
    meta: C::MetaSeq,
    layout: L,
}

pub fn container<D, L>(layout: L) -> Container<D, L, ()>
where
    L: Layout<D, ()>,
{
    Container {
        phantom: PhantomData,
        children: (),
        meta: <() as HasMetaSeq<ChildMeta>>::new_meta_seq(ChildMeta::default),
        layout,
    }
}

impl<D, L, C, V> AppendChild<D, V> for Container<D, L, C>
where
    L: Layout<D, C> + Layout<D, C::Output>,
    C: Append<V> + HasMetaSeq<ChildMeta>,
    C::Output: ViewSeq<D> + HasMetaSeq<ChildMeta>,
    V: View<D>,
{
    type Output = Container<D, L, C::Output>;

    fn child(self, child: V) -> Self::Output {
        Container {
            phantom: PhantomData,
            children: self.children.append(child),
            meta: C::Output::new_meta_seq(ChildMeta::default),
            layout: self.layout,
        }
    }
}

impl<D, L, C> SetChildren<D, C> for Container<D, L, ()>
where
    L: Layout<D, ()> + Layout<D, C::ViewSeq>,
    C: IntoViewSeq<D>,
    C::ViewSeq: HasMetaSeq<ChildMeta>,
{
    type Output = Container<D, L, C::ViewSeq>;

    fn children(self, children: C) -> Self::Output {
        Container {
            phantom: PhantomData,
            children: children.into_view_seq(),
            meta: C::ViewSeq::new_meta_seq(ChildMeta::default),
            layout: self.layout,
        }
    }
}

impl<D, L, C> View<D> for Container<D, L, C>
where
    L: Layout<D, C>,
    C: ViewSeq<D> + HasMetaSeq<ChildMeta>,
{
    fn init(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        let meta = self.meta.as_mut();

        let mut changed = false;

        for (i, (child, old_child)) in meta.iter_mut().zip(old.meta.as_mut()).enumerate() {
            *child = *old_child;
            child.changed = self.children.init(&mut old.children, i);
            changed |= child.changed;
            child.hover = Hover::None;
        }

        changed
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        self.layout
            .pre_layout(ctx, &mut self.children, self.meta.as_mut())
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.layout
            .layout(ctx, &mut self.children, self.meta.as_mut(), size)
    }

    fn hover(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) -> Hover {
        let meta = self.meta.as_mut();
        let mut hover = Hover::None;

        for (i, child) in meta.iter_mut().enumerate().rev() {
            if ctx.layer >= child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(bounds.rect.min + child.pos, child.size);
            let bounds = bounds.child(rect, Hover::None);

            child.hover = self.children.hover(ctx, bounds, i);

            if child.hover.is_some() {
                hover = Hover::Indirect;

                if !self.layout.allow_multi_hover() {
                    return hover;
                }
            }
        }

        hover
    }

    fn update(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) {
        let meta = self.meta.as_mut();

        for (i, child) in meta.iter().enumerate().rev() {
            if ctx.layer >= child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(bounds.rect.min + child.pos, child.size);
            let bounds = bounds.child(rect, child.hover);
            self.children.update(ctx, bounds, i);
        }
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) {
        let meta = self.meta.as_mut();

        for (i, child) in meta.iter().enumerate().rev() {
            if ctx.layer >= child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(bounds.rect.min + child.pos, child.size);
            let bounds = bounds.child(rect, child.hover);
            self.children.handle(ctx, bounds, event, i);
        }
    }

    fn draw(&mut self, ctx: &mut crate::DrawCtx, bounds: Bounds) {
        let meta = self.meta.as_mut();

        for (i, child) in meta.iter().enumerate() {
            if ctx.layer >= child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(bounds.rect.min + child.pos, child.size);

            let bounds = bounds.child(rect, child.hover);
            self.children.draw(ctx, bounds, i);

            if child.hover.is_some() && ctx.debug_draw {
                let color = if child.hover.is_direct() {
                    [0.0, 1.0, 0.0, 0.08]
                } else {
                    [1.0, 0.0, 0.0, 0.02]
                };
                ctx.encoder.rect(bounds.rect).fill_color(color);
            }
        }
    }
}
