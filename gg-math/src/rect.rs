use num_traits::{Num, NumCast};

use crate::{SideOffsets, Vec2};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Rect<T> {
    pub min: Vec2<T>,
    pub max: Vec2<T>,
}

impl<T> Rect<T> {
    #[inline]
    pub fn from_min_max(min: Vec2<T>, max: Vec2<T>) -> Rect<T> {
        Rect { min, max }
    }

    #[inline]
    pub fn map<U, F>(self, mut f: F) -> Rect<U>
    where
        F: FnMut(Vec2<T>) -> Vec2<U>,
    {
        Rect::from_min_max(f(self.min), f(self.max))
    }

    #[inline]
    pub fn try_map<U, E, F>(self, mut f: F) -> Result<Rect<U>, E>
    where
        F: FnMut(Vec2<T>) -> Result<Vec2<U>, E>,
    {
        Ok(Rect::from_min_max(f(self.min)?, f(self.max)?))
    }

    #[inline]
    pub fn try_cast<U>(self) -> Option<Rect<U>>
    where
        T: NumCast,
        U: NumCast,
    {
        self.try_map(|v| v.try_cast().ok_or(())).ok()
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
    pub fn new(pos: Vec2<T>, size: Vec2<T>) -> Rect<T> {
        Rect::from_min_max(pos, pos + size)
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
    pub fn size(&self) -> Vec2<T> {
        Vec2::new(self.width(), self.height())
    }

    #[inline]
    pub fn area(&self) -> T {
        self.width() * self.height()
    }

    #[inline]
    pub fn shrink(&self, offsets: &SideOffsets<T>) -> Rect<T> {
        Rect::from_min_max(
            self.min + offsets.top_left(),
            self.max - offsets.bottom_right(),
        )
    }

    #[inline]
    pub fn grow(&self, offsets: &SideOffsets<T>) -> Rect<T> {
        Rect::from_min_max(
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
        Rect::from_min_max(min, max)
    }
}

impl<T: PartialOrd + Copy> Rect<T> {
    #[inline]
    pub fn contains(&self, point: Vec2<T>) -> bool {
        self.min.cmp_le(point).all() && self.max.cmp_ge(point).all()
    }
}

impl<T: Num + Copy> From<[T; 4]> for Rect<T> {
    #[inline]
    fn from([x, y, w, h]: [T; 4]) -> Self {
        Rect::new(Vec2::new(x, y), Vec2::new(w, h))
    }
}
