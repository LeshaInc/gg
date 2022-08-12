use std::marker::PhantomData;

use gg_math::{Rect, Vec2};

use crate::view_seq::{Append, HasMetaSeq};
use crate::{
    AppendChild, Bounds, DrawCtx, Event, Hover, IntoViewSeq, LayoutCtx, LayoutHints, SetChildren,
    UpdateCtx, View, ViewSeq,
};

pub fn stack<D>(config: StackConfig) -> Stack<D, ()> {
    Stack {
        phantom: PhantomData,
        children: (),
        meta: <()>::new_meta_seq(Meta::default),
        config,
        size: Vec2::zero(),
    }
}

pub fn vstack<D>() -> Stack<D, ()> {
    stack(StackConfig {
        orientation: Orientation::Vertical,
        major_align: MajorAlign::SpaceEvenly,
        minor_align: MinorAlign::Center,
    })
}

pub fn hstack<D>() -> Stack<D, ()> {
    stack(StackConfig {
        orientation: Orientation::Horizontal,
        major_align: MajorAlign::SpaceEvenly,
        minor_align: MinorAlign::Center,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Orientation {
    fn indices(self) -> (usize, usize) {
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

pub struct Stack<D, C: HasMetaSeq<Meta>> {
    phantom: PhantomData<fn(&mut D)>,
    children: C,
    meta: C::MetaSeq,
    config: StackConfig,
    size: Vec2<f32>,
}

#[derive(Clone, Copy)]
pub struct Meta {
    hints: LayoutHints,
    stretch: f32,
    pos: Vec2<f32>,
    size: Vec2<f32>,
    changed: bool,
    hover: Hover,
}

impl Default for Meta {
    fn default() -> Meta {
        Meta {
            hints: LayoutHints::default(),
            stretch: 0.0,
            pos: Vec2::zero(),
            size: Vec2::zero(),
            changed: true,
            hover: Hover::None,
        }
    }
}

impl<D, C, V> AppendChild<D, V> for Stack<D, C>
where
    C: HasMetaSeq<Meta>,
    C: Append<V>,
    C::Output: ViewSeq<D> + HasMetaSeq<Meta>,
    V: View<D>,
{
    type Output = Stack<D, C::Output>;

    fn child(self, child: V) -> Self::Output {
        Stack {
            phantom: PhantomData,
            children: self.children.append(child),
            meta: C::Output::new_meta_seq(Meta::default),
            config: self.config,
            size: self.size,
        }
    }
}

impl<D, C> SetChildren<D, C> for Stack<D, ()>
where
    C: IntoViewSeq<D>,
    C::ViewSeq: HasMetaSeq<Meta>,
{
    type Output = Stack<D, C::ViewSeq>;

    fn children(self, children: C) -> Self::Output {
        Stack {
            phantom: PhantomData,
            children: children.into_view_seq(),
            meta: C::ViewSeq::new_meta_seq(Meta::default),
            config: self.config,
            size: self.size,
        }
    }
}

impl<D, C> View<D> for Stack<D, C>
where
    C: ViewSeq<D> + HasMetaSeq<Meta>,
{
    fn init(&mut self, old: &mut Self) -> bool {
        let meta = self.meta.as_mut();
        let old_meta = old.meta.as_ref();
        let mut changed = false;

        for (i, (child, old_child)) in meta.iter_mut().zip(old_meta).enumerate() {
            *child = *old_child;
            child.hover = Hover::None;
            child.changed = self.children.init(&mut old.children, i);
            changed |= child.changed;
        }

        self.size = old.size;

        changed
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        let meta = self.meta.as_mut();
        let (maj, min) = self.config.orientation.indices();

        let mut res = LayoutHints::default();

        for (i, child) in meta.iter_mut().enumerate() {
            if child.changed {
                child.hints = self.children.pre_layout(ctx, i);
            }

            res.min_size[maj] += child.hints.min_size[maj];
            res.min_size[min] = res.min_size[min].max(child.hints.min_size[min]);

            res.num_layers = res.num_layers.max(child.hints.num_layers);
        }

        res
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, adviced: Vec2<f32>) -> Vec2<f32> {
        let meta = self.meta.as_mut();
        let (maj, min) = self.config.orientation.indices();

        let mut total_stretch = 0.0;
        let mut used = Vec2::splat(0.0);

        let mut is_incremental = adviced == self.size;
        let mut was_incremental = !is_incremental;

        let max_iters = self.children.len() + 1;
        let mut rem_iters = max_iters;

        while rem_iters > 0 {
            if is_incremental != was_incremental {
                total_stretch = 0.0;
                used[min] = 0.0;
                used[maj] = 0.0;

                for child in meta.iter_mut() {
                    if !is_incremental {
                        child.size = child.hints.min_size;
                        child.stretch = child.hints.stretch;
                        child.changed = true;
                    }

                    used[maj] += child.size[maj];
                    total_stretch += child.stretch;
                }

                rem_iters = max_iters;
                was_incremental = is_incremental;
                continue;
            }

            rem_iters -= 1;

            let mut remaining = (adviced[maj] - used[maj]).max(0.0);
            let stretch_unit = remaining / total_stretch.max(1.0);

            for (i, child) in meta.iter_mut().enumerate() {
                let old_size = child.size;

                child.size[maj] += ((stretch_unit * child.stretch).ceil()).min(remaining);
                child.size[min] = adviced[min].max(used[min]);

                if child.size[min] >= child.hints.max_size[min] - 0.5 {
                    child.size[min] = child.hints.max_size[min];
                }

                if child.size[maj] >= child.hints.max_size[maj] - 0.5 {
                    child.size[maj] = child.hints.max_size[maj];

                    total_stretch -= child.stretch;
                    child.stretch = 0.0;
                }

                if child.size != old_size || child.changed {
                    child.size = self.children.layout(ctx, child.size, i);
                }

                let change = child.size[maj] - old_size[maj];
                used[maj] += change;
                remaining -= change;
                used[min] = used[min].max(child.size[min]);

                if change > 0.0 {
                    is_incremental = false;
                }
            }

            if used[maj] > adviced[maj] {
                is_incremental = false;
            }

            if used[maj] >= adviced[maj] - 0.5 && rem_iters > 1 {
                rem_iters = 1;
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

        self.size = used;

        used
    }

    fn hover(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) -> Hover {
        let meta = self.meta.as_mut();

        for (i, child) in meta.iter_mut().enumerate().rev() {
            if ctx.layer >= child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(child.pos + bounds.rect.min, child.size);
            let bounds = bounds.child(rect, Hover::None);

            child.hover = self.children.hover(ctx, bounds, i);

            if child.hover.is_some() {
                return Hover::Indirect;
            }
        }

        if ctx.layer == 0 && bounds.clip_rect.contains(ctx.input.mouse_pos()) {
            Hover::Direct
        } else {
            Hover::None
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) {
        let meta = self.meta.as_mut();

        for (i, child) in meta.iter_mut().enumerate().rev() {
            if ctx.layer > child.hints.num_layers {
                continue;
            }

            let rect = Rect::new(child.pos + bounds.rect.min, child.size);
            let bounds = bounds.child(rect, child.hover);
            self.children.update(ctx, bounds, i);
        }
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) {
        let meta = self.meta.as_ref();

        for (i, child) in meta.iter().enumerate().rev() {
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
