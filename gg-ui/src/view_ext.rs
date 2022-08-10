use gg_math::SideOffsets;

use crate::views::constrain::{MaxHeight, MaxWidth, MinHeight, MinWidth, Stretch};
use crate::views::*;
use crate::{IntoViewSeq, View};

pub trait AppendChild<D, V: View<D>> {
    type Output: View<D>;

    fn child(self, child: V) -> Self::Output;
}

pub trait SetChildren<D, C: IntoViewSeq<D>> {
    type Output: View<D>;

    fn children(self, children: C) -> Self::Output;
}

pub trait ViewExt<D>: View<D> + Sized {
    fn show_if(self, cond: bool) -> Choice<Self, Nothing<D>> {
        choose(cond, self, nothing())
    }

    fn constrain<C>(self, constraint: C) -> Constrain<Self, C> {
        constrain(self, constraint)
    }

    fn min_width(self, width: f32) -> Constrain<Self, MinWidth> {
        self.constrain(MinWidth(width))
    }

    fn min_height(self, height: f32) -> Constrain<Self, MinHeight> {
        self.constrain(MinHeight(height))
    }

    fn max_width(self, width: f32) -> Constrain<Self, MaxWidth> {
        self.constrain(MaxWidth(width))
    }

    fn max_height(self, height: f32) -> Constrain<Self, MaxHeight> {
        self.constrain(MaxHeight(height))
    }

    fn stretch(self, stretch: f32) -> Constrain<Self, Stretch> {
        self.constrain(Stretch(stretch))
    }

    fn padding<O: Into<SideOffsets<f32>>>(self, offsets: O) -> Padding<Self> {
        padding(offsets, self)
    }
}

impl<D, V> ViewExt<D> for V where V: View<D> + Sized {}
