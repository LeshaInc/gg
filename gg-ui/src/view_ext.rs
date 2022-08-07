use gg_math::SideOffsets;

use crate::views::*;
use crate::View;

pub trait ViewExt<D>: View<D> + Sized {
    fn show_if(self, cond: bool) -> Choice<Self, Nothing> {
        choose(cond, self, Nothing)
    }

    fn constrain<C>(self, constraint: C) -> ConstraintView<Self, C> {
        constrain(self, constraint)
    }

    fn min_width(self, width: f32) -> ConstraintView<Self, MinWidth> {
        self.constrain(MinWidth(width))
    }

    fn min_height(self, height: f32) -> ConstraintView<Self, MinHeight> {
        self.constrain(MinHeight(height))
    }

    fn max_width(self, width: f32) -> ConstraintView<Self, MaxWidth> {
        self.constrain(MaxWidth(width))
    }

    fn max_height(self, height: f32) -> ConstraintView<Self, MaxHeight> {
        self.constrain(MaxHeight(height))
    }

    fn set_stretch(self, stretch: f32) -> ConstraintView<Self, SetStretch> {
        self.constrain(SetStretch(stretch))
    }

    fn padding<O: Into<SideOffsets<f32>>>(self, offsets: O) -> Padding<Self> {
        padding(offsets, self)
    }
}

impl<D, V> ViewExt<D> for V where V: View<D> + Sized {}
