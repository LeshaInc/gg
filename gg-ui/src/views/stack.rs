use std::marker::PhantomData;

use gg_math::{Rect, Vec2};

use crate::view_seq::MetaSeq;
use crate::{DrawCtx, LayoutCtx, LayoutHints, View, ViewSeq};

pub fn stack<D, S>(config: StackConfig, children: S) -> Stack<D, S>
where
    S: ViewSeq<D> + MetaSeq<Meta>,
{
    Stack {
        phantom: PhantomData,
        children,
        meta: S::new_meta_seq(Meta::default),
        config,
    }
}

pub fn vstack<D, S>(children: S) -> Stack<D, S>
where
    S: ViewSeq<D> + MetaSeq<Meta>,
{
    stack(
        StackConfig {
            orientation: Orientation::Vertical,
            major_align: MajorAlign::SpaceEvenly,
            minor_align: MinorAlign::Center,
        },
        children,
    )
}

pub fn hstack<D, S>(children: S) -> Stack<D, S>
where
    S: ViewSeq<D> + MetaSeq<Meta>,
{
    stack(
        StackConfig {
            orientation: Orientation::Horizontal,
            major_align: MajorAlign::SpaceEvenly,
            minor_align: MinorAlign::Center,
        },
        children,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn indices(self) -> (usize, usize) {
        match self {
            Orientation::Horizontal => (0, 1),
            Orientation::Vertical => (1, 0),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinorAlign {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MajorAlign {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackConfig {
    pub orientation: Orientation,
    pub major_align: MajorAlign,
    pub minor_align: MinorAlign,
}

pub struct Stack<D, C: MetaSeq<Meta>> {
    phantom: PhantomData<fn(D)>,
    children: C,
    meta: C::MetaSeq,
    config: StackConfig,
}

#[derive(Default)]
pub struct Meta {
    hints: LayoutHints,
    pos: Vec2<f32>,
    size: Vec2<f32>,
}

impl<D, C> View<D> for Stack<D, C>
where
    C: ViewSeq<D> + MetaSeq<Meta>,
{
    fn update(&mut self, old: &Self) -> bool {
        let meta = self.meta.as_mut();
        let old_meta = old.meta.as_ref();
        let mut changed = false;

        for (i, (child, old_child)) in meta.iter_mut().zip(old_meta).enumerate() {
            changed |= self.children.update(&old.children, i);
            child.size = old_child.size;
            child.pos = old_child.pos;
        }

        changed
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        let meta = self.meta.as_mut();
        let (maj, min) = self.config.orientation.indices();

        let mut res = LayoutHints::default();

        for (i, child) in meta.iter_mut().enumerate() {
            let hints = self.children.pre_layout(ctx.reborrow(), i);

            res.min_size[min] += hints.min_size[min];
            res.min_size[maj] = res.min_size[maj].max(hints.min_size[maj]);

            child.hints = hints;
            child.size = hints.min_size;
        }

        res
    }

    fn layout(&mut self, ctx: LayoutCtx, adviced: Vec2<f32>) -> Vec2<f32> {
        let meta = self.meta.as_mut();
        let (maj, min) = self.config.orientation.indices();

        let mut total_stretch: f32 = meta.iter().map(|v| v.hints.stretch).sum();
        let mut used = Vec2::splat(0.0);

        used[maj] = meta.iter().map(|v| v.size[maj]).sum();

        for _ in 0..10 {
            let mut remaining = (adviced[maj] - used[maj]).max(0.0);
            let stretch_unit = remaining / total_stretch.max(1.0);

            for (i, child) in meta.iter_mut().enumerate() {
                let old_size = child.size;

                child.size[maj] += ((stretch_unit * child.hints.stretch).ceil()).min(remaining);
                child.size[min] = adviced[min].max(used[min]);

                if child.size[min] >= child.hints.max_size[min] - 0.5 {
                    child.size[min] = child.hints.max_size[min];
                }

                if child.size[maj] >= child.hints.max_size[maj] - 0.5 {
                    child.size[maj] = child.hints.max_size[maj];

                    total_stretch -= child.hints.stretch;
                    child.hints.stretch = 0.0;
                }

                child.size = self.children.layout(ctx.reborrow(), child.size, i);

                let change = child.size[maj] - old_size[maj];
                used[maj] += change;
                remaining -= change;
                used[min] = used[min].max(child.size[min]);
            }

            if used[maj] >= adviced[maj] - 0.5 {
                break;
            }
        }

        let remaining = (adviced[maj] - used[maj]).max(0.0);
        let count = self.children.len() as f32;

        used = used.fmax(adviced);

        let (mut offset, pad_child) = major_offset(self.config.major_align, remaining, count);

        for child in meta {
            child.pos[min] = minor_offset(self.config.minor_align, used[min], child.size[min]);
            child.pos[maj] = offset + pad_child;
            offset += child.size[maj] + pad_child * 2.0;
        }

        used
    }

    fn draw(&mut self, mut ctx: DrawCtx, bounds: Rect<f32>) {
        let meta = self.meta.as_ref();

        for (i, child) in meta.iter().enumerate() {
            let bounds = Rect::from_pos_extents(child.pos + bounds.min, child.size);
            self.children.draw(ctx.reborrow(), bounds, i);
        }
    }
}

fn major_offset(align: MajorAlign, rem: f32, count: f32) -> (f32, f32) {
    match align {
        MajorAlign::Start => (0.0, 0.0),
        MajorAlign::Center => (rem / 2.0, 0.0),
        MajorAlign::End => (rem, 0.0),
        MajorAlign::SpaceBetween => {
            let unit = rem / (count - 1.0) / 2.0;
            (-unit, unit)
        }
        MajorAlign::SpaceAround => {
            let unit = rem / count / 2.0;
            (0.0, unit)
        }
        MajorAlign::SpaceEvenly => {
            let unit = rem / (count + 1.0) / 2.0;
            (unit, unit)
        }
    }
}

fn minor_offset(align: MinorAlign, used: f32, size: f32) -> f32 {
    match align {
        MinorAlign::Start => 0.0,
        MinorAlign::Center => (used - size) * 0.5,
        MinorAlign::End => used - size,
    }
}
