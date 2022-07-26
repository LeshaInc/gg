use std::fmt::{self, Debug};
use std::ops::{
    Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Not, Sub, SubAssign,
};

use num_traits::{Float, Num, NumCast, Signed, Zero};

use crate::lerp;

#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

impl<T: Debug> Debug for Vec2<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}, {:?}]", self.x, self.y)
    }
}

impl<T> Vec2<T> {
    #[inline]
    pub const fn new(x: T, y: T) -> Vec2<T> {
        Vec2 { x, y }
    }

    #[inline]
    pub const fn splat(v: T) -> Vec2<T>
    where
        T: Copy,
    {
        Vec2::new(v, v)
    }

    #[inline]
    pub fn zero() -> Vec2<T>
    where
        T: Zero,
    {
        Vec2::new(T::zero(), T::zero())
    }

    #[inline]
    pub fn from_angle(angle: T) -> Vec2<T>
    where
        T: Float,
    {
        Vec2::new(angle.cos(), angle.sin())
    }

    #[inline]
    pub fn set_x(mut self, x: T) -> Vec2<T> {
        self.x = x;
        self
    }

    #[inline]
    pub fn set_y(mut self, y: T) -> Vec2<T> {
        self.y = y;
        self
    }

    #[inline]
    pub fn map<U, F>(self, mut f: F) -> Vec2<U>
    where
        F: FnMut(T) -> U,
    {
        Vec2::new(f(self.x), f(self.y))
    }

    #[inline]
    pub fn try_map<U, E, F>(self, mut f: F) -> Result<Vec2<U>, E>
    where
        F: FnMut(T) -> Result<U, E>,
    {
        Ok(Vec2::new(f(self.x)?, f(self.y)?))
    }

    #[inline]
    pub fn zip_map<U, F>(self, rhs: Vec2<T>, mut f: F) -> Vec2<U>
    where
        F: FnMut(T, T) -> U,
    {
        Vec2::new(f(self.x, rhs.x), f(self.y, rhs.y))
    }

    #[inline]
    pub fn fold<U, F>(self, mut acc: U, mut f: F) -> U
    where
        F: FnMut(U, T) -> U,
    {
        acc = f(acc, self.x);
        acc = f(acc, self.y);
        acc
    }

    #[inline]
    pub fn reduce<F>(self, f: F) -> T
    where
        F: FnOnce(T, T) -> T,
    {
        f(self.x, self.y)
    }

    #[inline]
    pub fn try_cast<U>(self) -> Option<Vec2<U>>
    where
        T: NumCast,
        U: NumCast,
    {
        self.try_map(|v| U::from(v).ok_or(())).ok()
    }

    #[inline]
    pub fn cast<U>(self) -> Vec2<U>
    where
        T: NumCast,
        U: NumCast,
    {
        self.try_cast().expect("cast failed")
    }
}

impl<T: Num + Copy> Vec2<T> {
    #[inline]
    pub fn sum(self) -> T {
        self.reduce(T::add)
    }

    #[inline]
    pub fn product(self) -> T {
        self.reduce(T::mul)
    }

    #[inline]
    pub fn dot(self, rhs: Vec2<T>) -> T {
        (self * rhs).sum()
    }

    #[inline]
    pub fn perp(self) -> Vec2<T>
    where
        T: Signed,
    {
        Vec2::new(self.y, self.x)
    }

    #[inline]
    pub fn perp_dot(self, rhs: Vec2<T>) -> T
    where
        T: Signed,
    {
        self.perp().dot(rhs)
    }

    #[inline]
    pub fn length_squared(self) -> T {
        (self * self).sum()
    }

    #[inline]
    pub fn abs(self) -> Vec2<T>
    where
        T: Signed,
    {
        self.map(|v| v.abs())
    }
}

impl<T: PartialOrd> Vec2<T> {
    #[inline]
    pub fn cmp_lt(self, rhs: Vec2<T>) -> Vec2<bool> {
        self.zip_map(rhs, |a, b| a < b)
    }

    #[inline]
    pub fn cmp_le(self, rhs: Vec2<T>) -> Vec2<bool> {
        self.zip_map(rhs, |a, b| a <= b)
    }

    #[inline]
    pub fn cmp_eq(self, rhs: Vec2<T>) -> Vec2<bool> {
        self.zip_map(rhs, |a, b| a == b)
    }

    #[inline]
    pub fn cmp_ge(self, rhs: Vec2<T>) -> Vec2<bool> {
        self.zip_map(rhs, |a, b| a >= b)
    }

    #[inline]
    pub fn cmp_gt(self, rhs: Vec2<T>) -> Vec2<bool> {
        self.zip_map(rhs, |a, b| a > b)
    }

    #[inline]
    pub fn cmp_ne(self, rhs: Vec2<T>) -> Vec2<bool> {
        self.zip_map(rhs, |a, b| a != b)
    }
}

impl<T: Ord> Vec2<T> {
    #[inline]
    pub fn min(self, rhs: Vec2<T>) -> Vec2<T> {
        self.zip_map(rhs, std::cmp::min)
    }

    #[inline]
    pub fn max(self, rhs: Vec2<T>) -> Vec2<T> {
        self.zip_map(rhs, std::cmp::max)
    }

    #[inline]
    pub fn clamp(self, lo: Vec2<T>, hi: Vec2<T>) -> Vec2<T> {
        self.max(lo).min(hi)
    }

    #[inline]
    pub fn min_component(self) -> T {
        self.reduce(std::cmp::min)
    }

    #[inline]
    pub fn max_component(self) -> T {
        self.reduce(std::cmp::max)
    }
}

impl<T: Float> Vec2<T> {
    #[inline]
    pub fn fmin(self, rhs: Vec2<T>) -> Vec2<T> {
        self.zip_map(rhs, T::min)
    }

    #[inline]
    pub fn fmax(self, rhs: Vec2<T>) -> Vec2<T> {
        self.zip_map(rhs, T::max)
    }

    #[inline]
    pub fn fclamp(self, lo: Vec2<T>, hi: Vec2<T>) -> Vec2<T> {
        self.fmax(lo).fmin(hi)
    }

    #[inline]
    pub fn length(self) -> T {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn try_normalize(self) -> Option<Vec2<T>> {
        let len_sq = self.length_squared();
        if len_sq < T::epsilon() {
            None
        } else {
            Some(self / len_sq.sqrt())
        }
    }

    #[inline]
    pub fn normalize(self) -> Vec2<T> {
        self / self.length()
    }

    #[inline]
    pub fn round(self) -> Vec2<T> {
        self.map(T::round)
    }

    #[inline]
    pub fn floor(self) -> Vec2<T> {
        self.map(T::floor)
    }

    #[inline]
    pub fn ceil(self) -> Vec2<T> {
        self.map(T::ceil)
    }

    #[inline]
    pub fn lerp(self, rhs: Vec2<T>, time: T) -> Vec2<T> {
        self.zip_map(rhs, |a, b| lerp(a, b, time))
    }
}

impl Vec2<bool> {
    #[inline]
    pub fn all(self) -> bool {
        self.x && self.y
    }

    #[inline]
    pub fn any(self) -> bool {
        self.x || self.y
    }
}

impl<T: Neg<Output = T>> Neg for Vec2<T> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.map(T::neg)
    }
}

impl Not for Vec2<bool> {
    type Output = Self;

    #[inline]
    fn not(self) -> Self {
        self.map(bool::not)
    }
}

impl<T: Add<Output = T>> Add for Vec2<T> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.zip_map(rhs, T::add)
    }
}

impl<T: Sub<Output = T>> Sub for Vec2<T> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        self.zip_map(rhs, T::sub)
    }
}

impl<T: Mul<Output = T>> Mul for Vec2<T> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.zip_map(rhs, T::mul)
    }
}

impl<T: Div<Output = T>> Div for Vec2<T> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self {
        self.zip_map(rhs, T::div)
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vec2<T> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: T) -> Self {
        self.map(|v| v * rhs)
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Vec2<T> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: T) -> Self {
        self.map(|v| v / rhs)
    }
}

impl<T: AddAssign> AddAssign for Vec2<T> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: SubAssign> SubAssign for Vec2<T> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: MulAssign> MulAssign for Vec2<T> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<T: DivAssign> DivAssign for Vec2<T> {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl<T: MulAssign + Copy> MulAssign<T> for Vec2<T> {
    #[inline]
    fn mul_assign(&mut self, rhs: T) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<T: DivAssign + Copy> DivAssign<T> for Vec2<T> {
    #[inline]
    fn div_assign(&mut self, rhs: T) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<T> Index<usize> for Vec2<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("index out of bounds"),
        }
    }
}

impl<T> IndexMut<usize> for Vec2<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("index out of bounds"),
        }
    }
}
