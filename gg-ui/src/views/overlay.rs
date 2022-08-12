use std::marker::PhantomData;

use gg_math::{Rect, Vec2};

use crate::view_seq::{Append, HasMetaSeq};
use crate::{
    AppendChild, Bounds, DrawCtx, Event, Hover, IntoViewSeq, LayoutCtx, LayoutHints, SetChildren,
    UpdateCtx, View, ViewSeq,
};

pub fn overlay<D>() -> Overlay<D, ()> {
    Overlay {
        phantom: PhantomData,
        children: (),
        meta: <()>::new_meta_seq(Meta::default),
    }
}

pub struct Overlay<D, C: HasMetaSeq<Meta>> {
    phantom: PhantomData<fn(&mut D)>,
    children: C,
    meta: C::MetaSeq,
}

#[derive(Clone, Copy, Default)]
pub struct Meta {
    hints: LayoutHints,
    size: Vec2<f32>,
    pos: Vec2<f32>,
    hover: Hover,
}

impl<D, C, V> AppendChild<D, V> for Overlay<D, C>
where
    C: HasMetaSeq<Meta>,
    C: Append<V>,
    C::Output: ViewSeq<D> + HasMetaSeq<Meta>,
    V: View<D>,
{
    type Output = Overlay<D, C::Output>;

    fn child(self, child: V) -> Self::Output {
        Overlay {
            phantom: PhantomData,
            children: self.children.append(child),
            meta: C::Output::new_meta_seq(Meta::default),
        }
    }
}

impl<D, C> SetChildren<D, C> for Overlay<D, ()>
where
    C: IntoViewSeq<D>,
    C::ViewSeq: HasMetaSeq<Meta>,
{
    type Output = Overlay<D, C::ViewSeq>;

    fn children(self, children: C) -> Self::Output {
        Overlay {
            phantom: PhantomData,
            children: children.into_view_seq(),
            meta: C::ViewSeq::new_meta_seq(Meta::default),
        }
    }
}

impl<D, C> View<D> for Overlay<D, C>
where
    C: ViewSeq<D> + HasMetaSeq<Meta>,
{
    fn init(&mut self, old: &mut Self) -> bool {
        let meta = self.meta.as_mut();
        let old_meta = old.meta.as_mut();

        let mut changed = false;

        for (i, (child, old_child)) in meta.iter_mut().zip(old_meta).enumerate() {
            if self.children.init(&mut old.children, i) {
                changed = true;
            } else {
                *child = *old_child;
                child.hover = Hover::None;
            }
        }

        changed
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        let meta = self.meta.as_mut();
        let mut hints = LayoutHints::default();

        for (i, child) in meta.iter_mut().enumerate() {
            child.hints = self.children.pre_layout(ctx, i);
            hints.min_size = hints.min_size.fmax(child.hints.min_size);
            hints.max_size = hints.max_size.fmin(child.hints.max_size);
            hints.num_layers = hints.num_layers.max(child.hints.num_layers);
        }

        hints.max_size = hints.max_size.fmax(hints.min_size);
        hints
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, mut size: Vec2<f32>) -> Vec2<f32> {
        let meta = self.meta.as_mut();

        let max_iters = 5;
        'outer: for _ in 0..max_iters {
            for (i, child) in meta.iter_mut().enumerate() {
                let adviced = size.fclamp(child.hints.min_size, child.hints.max_size);
                if adviced != child.size {
                    child.size = self.children.layout(ctx, adviced, i);
                }

                if child.size.cmp_gt(adviced).any() {
                    size = size.fmax(child.size);
                    continue 'outer;
                }
            }

            break;
        }

        for child in meta.iter_mut() {
            child.pos = (size - child.size) / 2.0;
        }

        size
    }

    fn hover(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) -> Hover {
        let meta = self.meta.as_mut();
        let mut hover = Hover::None;

        for (i, child) in meta.iter_mut().enumerate().rev() {
            if ctx.layer >= child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(child.pos + bounds.rect.min, child.size);
            let bounds = bounds.child(rect, Hover::None);

            child.hover = self.children.hover(ctx, bounds, i);

            if child.hover.is_some() {
                hover = Hover::Indirect;
            }
        }

        hover
    }

    fn update(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) {
        let meta = self.meta.as_ref();

        for (i, child) in meta.iter().enumerate() {
            let rect = Rect::new(child.pos + bounds.rect.min, child.size);
            let bounds = bounds.child(rect, child.hover);

            self.children.update(ctx, bounds, i);
        }
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) {
        let meta = self.meta.as_ref();

        for (i, child) in meta.iter().enumerate() {
            if ctx.layer >= child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(child.pos + bounds.rect.min, child.size);
            let bounds = bounds.child(rect, child.hover);

            self.children.handle(ctx, bounds, event, i);
        }
    }

    fn draw(&mut self, ctx: &mut DrawCtx, bounds: Bounds) {
        let meta = self.meta.as_ref();

        for (i, child) in meta.iter().enumerate() {
            if ctx.layer >= child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(child.pos + bounds.rect.min, child.size);
            let bounds = bounds.child(rect, child.hover);

            self.children.draw(ctx, bounds, i);

            if child.hover.is_direct() {
                ctx.encoder
                    .rect(bounds.rect)
                    .fill_color([0.0, 1.0, 0.0, 0.08]);
            }

            if child.hover.is_indirect() {
                ctx.encoder
                    .rect(bounds.rect)
                    .fill_color([1.0, 0.0, 0.0, 0.02]);
            }
        }
    }
}
