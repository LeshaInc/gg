use std::marker::PhantomData;

use gg_math::{Rect, Vec2};

use crate::view_seq::MetaSeq;
use crate::{DrawCtx, Event, HandleCtx, LayoutCtx, LayoutHints, View, ViewSeq};

pub fn overlay<D, C>(children: C) -> Overlay<D, C>
where
    C: ViewSeq<D> + MetaSeq<Meta>,
{
    Overlay {
        phantom: PhantomData,
        children,
        meta: C::new_meta_seq(Meta::default),
    }
}

pub struct Overlay<D, C: MetaSeq<Meta>> {
    phantom: PhantomData<fn(D)>,
    children: C,
    meta: C::MetaSeq,
}

#[derive(Clone, Copy, Default)]
pub struct Meta {
    hints: LayoutHints,
    size: Vec2<f32>,
    pos: Vec2<f32>,
}

impl<D, C> View<D> for Overlay<D, C>
where
    C: ViewSeq<D> + MetaSeq<Meta>,
{
    fn update(&mut self, old: &mut Self) -> bool {
        let meta = self.meta.as_mut();
        let old_meta = old.meta.as_mut();

        let mut changed = false;

        for (i, (child, old_child)) in meta.iter_mut().zip(old_meta).enumerate() {
            if self.children.update(&mut old.children, i) {
                changed = true;
            } else {
                *child = *old_child;
            }
        }

        changed
    }

    fn pre_layout(&mut self, mut ctx: LayoutCtx) -> LayoutHints {
        let meta = self.meta.as_mut();
        let mut hints = LayoutHints::default();

        for (i, child) in meta.iter_mut().enumerate() {
            child.hints = self.children.pre_layout(ctx.reborrow(), i);
            hints.min_size = hints.min_size.fmax(child.hints.min_size);
            hints.max_size = hints.max_size.fmin(child.hints.max_size);
        }

        hints.max_size = hints.max_size.fmax(hints.min_size);
        hints
    }

    fn layout(&mut self, mut ctx: LayoutCtx, mut size: Vec2<f32>) -> Vec2<f32> {
        let meta = self.meta.as_mut();

        let max_iters = 5;
        'outer: for _ in 0..max_iters {
            for (i, child) in meta.iter_mut().enumerate() {
                let adviced = size.fclamp(child.hints.min_size, child.hints.max_size);
                if adviced != child.size {
                    child.size = self.children.layout(ctx.reborrow(), adviced, i);
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

    fn draw(&mut self, mut ctx: DrawCtx, bounds: Rect<f32>) {
        let meta = self.meta.as_ref();

        for (i, child) in meta.iter().enumerate() {
            let bounds = Rect::from_pos_extents(child.pos + bounds.min, child.size);
            self.children.draw(ctx.reborrow(), bounds, i);
        }
    }

    fn handle(&mut self, mut ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event) {
        let meta = self.meta.as_ref();

        for (i, child) in meta.iter().enumerate() {
            let bounds = Rect::from_pos_extents(child.pos + bounds.min, child.size);
            self.children.handle(ctx.reborrow(), bounds, event, i);
        }
    }
}
