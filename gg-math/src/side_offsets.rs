use num_traits::Num;

use crate::Vec2;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SideOffsets<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T> SideOffsets<T> {
    #[inline]
    pub fn new(top: T, right: T, bottom: T, left: T) -> SideOffsets<T> {
        SideOffsets {
            top,
            right,
            bottom,
            left,
        }
    }

    #[inline]
    pub fn new_symmetric(vert: T, horiz: T) -> SideOffsets<T>
    where
        T: Copy,
    {
        SideOffsets::new(vert, horiz, vert, horiz)
    }

    #[inline]
    pub fn new_equal(v: T) -> SideOffsets<T>
    where
        T: Copy,
    {
        SideOffsets {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    #[inline]
    pub fn top_left(self) -> Vec2<T> {
        Vec2::new(self.left, self.top)
    }

    #[inline]
    pub fn bottom_right(self) -> Vec2<T> {
        Vec2::new(self.right, self.bottom)
    }

    #[inline]
    pub fn size(self) -> Vec2<T>
    where
        T: Num + Copy,
    {
        self.top_left() + self.bottom_right()
    }
}

impl<T> From<[T; 4]> for SideOffsets<T> {
    #[inline]
    fn from([l, r, b, t]: [T; 4]) -> Self {
        Self::new(l, r, b, t)
    }
}

impl<T: Copy> From<T> for SideOffsets<T> {
    #[inline]
    fn from(v: T) -> Self {
        Self::new_equal(v)
    }
}

impl<T: Copy> From<[T; 2]> for SideOffsets<T> {
    #[inline]
    fn from([v, h]: [T; 2]) -> Self {
        Self::new_symmetric(v, h)
    }
}
