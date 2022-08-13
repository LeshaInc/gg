use gg_math::Vec2;

use super::container::{container, ChildMeta, Container, Layout};
use crate::{LayoutCtx, LayoutHints, ViewSeq};

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

pub struct Stack {
    config: StackConfig,
    size: Vec2<f32>,
}

pub fn stack<D>(config: StackConfig) -> Container<D, Stack, ()> {
    container(Stack {
        config,
        size: Vec2::zero(),
    })
}

pub fn vstack<D>() -> Container<D, Stack, ()> {
    stack(StackConfig {
        orientation: Orientation::Vertical,
        major_align: MajorAlign::SpaceEvenly,
        minor_align: MinorAlign::Center,
    })
}

pub fn hstack<D>() -> Container<D, Stack, ()> {
    stack(StackConfig {
        orientation: Orientation::Horizontal,
        major_align: MajorAlign::SpaceEvenly,
        minor_align: MinorAlign::Center,
    })
}

impl<D, C> Layout<D, C> for Stack
where
    C: ViewSeq<D>,
{
    fn pre_layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut C,
        meta: &mut [ChildMeta],
    ) -> LayoutHints {
        let (maj, min) = self.config.orientation.indices();
        let mut hints = LayoutHints::default();

        for (i, child) in meta.iter_mut().enumerate() {
            if child.changed {
                child.hints = children.pre_layout(ctx, i);
            }

            hints.min_size[maj] += child.hints.min_size[maj];
            hints.min_size[min] = hints.min_size[min].max(child.hints.min_size[min]);
            hints.num_layers = hints.num_layers.max(child.hints.num_layers);
        }

        hints
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut C,
        meta: &mut [ChildMeta],
        adviced: Vec2<f32>,
    ) -> Vec2<f32> {
        let (maj, min) = self.config.orientation.indices();

        let mut total_stretch = 0.0;
        let mut used = Vec2::splat(0.0);

        let mut is_incremental = adviced == self.size;
        let mut was_incremental = !is_incremental;

        let max_iters = children.len() + 1;
        let mut rem_iters = max_iters;

        while rem_iters > 0 {
            if is_incremental != was_incremental {
                total_stretch = 0.0;
                used[min] = 0.0;
                used[maj] = 0.0;

                for child in meta.iter_mut() {
                    if !is_incremental {
                        child.size = child.hints.min_size;
                        child.changed = true;
                    }

                    used[maj] += child.size[maj];
                    total_stretch += child.hints.stretch;
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

                let stretch = if child.size[maj] >= child.hints.max_size[maj] - 0.5 {
                    0.0
                } else {
                    child.hints.stretch
                };

                child.size[maj] += ((stretch_unit * stretch).ceil()).min(remaining);
                child.size[min] = adviced[min].max(used[min]);

                if child.size[min] >= child.hints.max_size[min] - 0.5 {
                    child.size[min] = child.hints.max_size[min];
                }

                if child.size[maj] >= child.hints.max_size[maj] - 0.5 {
                    child.size[maj] = child.hints.max_size[maj];
                    total_stretch -= stretch;
                }

                if child.size != old_size || child.changed {
                    child.size = children.layout(ctx, child.size, i);
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
        let count = children.len() as f32;

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
