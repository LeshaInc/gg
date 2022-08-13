use gg_math::Vec2;

use super::container::{container, ChildMeta, Container, Layout};
use crate::{LayoutCtx, LayoutHints, ViewSeq};

pub fn overlay<D>() -> Container<D, Overlay, ()> {
    container(Overlay)
}

pub struct Overlay;

impl<D, C> Layout<D, C> for Overlay
where
    C: ViewSeq<D>,
{
    fn pre_layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut C,
        meta: &mut [ChildMeta],
    ) -> LayoutHints {
        let mut hints = LayoutHints::default();

        for (i, child) in meta.iter_mut().enumerate() {
            if child.changed {
                child.hints = children.pre_layout(ctx, i);
            }

            hints.min_size = hints.min_size.fmax(child.hints.min_size);
            hints.max_size = hints.max_size.fmin(child.hints.max_size);
            hints.num_layers = hints.num_layers.max(child.hints.num_layers);
        }

        hints.max_size = hints.max_size.fmax(hints.min_size);
        hints
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut C,
        meta: &mut [ChildMeta],
        mut size: Vec2<f32>,
    ) -> Vec2<f32> {
        let max_iters = 5;
        'outer: for _ in 0..max_iters {
            for (i, child) in meta.iter_mut().enumerate() {
                let adviced = size.fclamp(child.hints.min_size, child.hints.max_size);
                if adviced != child.size {
                    child.size = children.layout(ctx, adviced, i);
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

    fn allow_multi_hover(&self) -> bool {
        true
    }
}
