use std::ops::Mul;

use num_traits::Float;

use crate::Vec2;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Rotation2<T> {
    pub cos: T,
    pub sin: T,
}

impl<T> Rotation2<T> {
    #[inline]
    pub fn new(cos: T, sin: T) -> Rotation2<T> {
        Rotation2 { cos, sin }
    }

    /// Constructs a rotation that would point x-axis to `vec`.
    ///
    /// `vec` must be normalized.
    #[inline]
    pub fn from_vec2(vec: Vec2<T>) -> Rotation2<T> {
        Rotation2::new(vec.x, vec.y)
    }

    #[inline]
    pub fn as_vec2(self) -> Vec2<T> {
        Vec2::new(self.cos, self.sin)
    }

    #[inline]
    pub fn from_vec(vec: Vec2<T>) -> Rotation2<T> {
        Rotation2::new(vec.x, vec.y)
    }
}

impl<T: Float> Rotation2<T> {
    #[inline]
    pub fn from_angle(angle: T) -> Rotation2<T> {
        let (sin, cos) = angle.sin_cos();
        Rotation2::new(sin, cos)
    }

    #[inline]
    pub fn inverse(self) -> Rotation2<T> {
        Rotation2::new(self.cos, -self.sin)
    }

    #[inline]
    pub fn transform(self, vec: Vec2<T>) -> Vec2<T> {
        self * vec
    }

    #[inline]
    pub fn slerp(self, rhs: Rotation2<T>, t: T) -> Rotation2<T> {
        let start = self.as_vec2();
        let end = rhs.as_vec2();

        let angle = start.dot(end).acos();
        let a = ((T::one() - t) * angle).sin() / angle.sin();
        let b = (t * angle).sin() / angle.sin();

        let res = start * a + end * b;
        Rotation2::from_vec2(res)
    }
}

impl<T: Float> Mul<Vec2<T>> for Rotation2<T> {
    type Output = Vec2<T>;

    #[inline]
    fn mul(self, rhs: Vec2<T>) -> Vec2<T> {
        Vec2::new(
            rhs.x * self.cos - rhs.y * self.sin,
            rhs.x * self.sin + rhs.y * self.cos,
        )
    }
}

impl<T: Float> Mul for Rotation2<T> {
    type Output = Rotation2<T>;

    #[inline]
    fn mul(self, rhs: Rotation2<T>) -> Rotation2<T> {
        Rotation2::new(
            self.cos * rhs.cos - self.sin * rhs.sin,
            self.sin * rhs.cos + self.cos * rhs.sin,
        )
    }
}
