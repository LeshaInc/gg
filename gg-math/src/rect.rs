use num_traits::{Num, NumCast};

use crate::{SideOffsets, Vec2};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Rect<T> {
    pub min: Vec2<T>,
    pub max: Vec2<T>,
}

impl<T> Rect<T> {
    #[inline]
    pub fn new(min: Vec2<T>, max: Vec2<T>) -> Rect<T> {
        Rect { min, max }
    }

    #[inline]
    pub fn try_cast<U>(self) -> Option<Rect<U>>
    where
        T: NumCast,
        U: NumCast,
    {
        Some(Rect {
            min: self.min.try_cast()?,
            max: self.max.try_cast()?,
        })
    }

    #[inline]
    pub fn cast<U>(self) -> Rect<U>
    where
        T: NumCast,
        U: NumCast,
    {
        self.try_cast().expect("cast failed")
    }
}

impl<T: Num + Copy> Rect<T> {
    #[inline]
    pub fn from_pos_extents(pos: Vec2<T>, extents: Vec2<T>) -> Rect<T> {
        Rect::new(pos, pos + extents)
    }

    #[inline]
    pub fn center(&self) -> Vec2<T> {
        let two = T::one() + T::one();
        (self.min + self.max) / two
    }

    #[inline]
    pub fn width(&self) -> T {
        self.max.x - self.min.x
    }

    #[inline]
    pub fn height(&self) -> T {
        self.max.y - self.min.y
    }

    #[inline]
    pub fn extents(&self) -> Vec2<T> {
        Vec2::new(self.width(), self.height())
    }

    #[inline]
    pub fn area(&self) -> T {
        self.width() * self.height()
    }

    #[inline]
    pub fn shrink(&self, offsets: &SideOffsets<T>) -> Rect<T> {
        Rect::new(
            self.min + offsets.top_left(),
            self.max - offsets.bottom_right(),
        )
    }

    #[inline]
    pub fn grow(&self, offsets: &SideOffsets<T>) -> Rect<T> {
        Rect::new(
            self.min - offsets.top_left(),
            self.max + offsets.bottom_right(),
        )
    }

    #[inline]
    pub fn vertices(&self) -> [Vec2<T>; 4] {
        [
            self.min,
            Vec2::new(self.max.x, self.min.y),
            self.max,
            Vec2::new(self.min.x, self.max.y),
        ]
    }
}

impl<T: Ord + Copy> Rect<T> {
    #[inline]
    pub fn intersect(&self, rhs: &Rect<T>) -> Rect<T> {
        let min = self.min.max(rhs.min);
        let max = self.max.min(rhs.max).max(min);
        Rect::new(min, max)
    }
}

impl<T: Num + Copy> From<[T; 4]> for Rect<T> {
    #[inline]
    fn from([x, y, w, h]: [T; 4]) -> Self {
        Rect::from_pos_extents(Vec2::new(x, y), Vec2::new(w, h))
    }
}
