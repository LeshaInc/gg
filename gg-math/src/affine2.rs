use std::ops::Mul;

use num_traits::Float;

use crate::{Rotation2, Vec2};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Affine2<T> {
    pub x: Vec2<T>,
    pub y: Vec2<T>,
    pub z: Vec2<T>,
}

impl<T> Affine2<T> {
    #[inline]
    pub fn new(x: Vec2<T>, y: Vec2<T>, z: Vec2<T>) -> Affine2<T> {
        Affine2 { x, y, z }
    }
}
impl<T: Float> Affine2<T> {
    #[inline]
    pub fn identity() -> Affine2<T> {
        Affine2::translation(Vec2::zero())
    }

    #[inline]
    pub fn translation(vec: Vec2<T>) -> Affine2<T> {
        Affine2::new(
            Vec2::new(T::one(), T::zero()),
            Vec2::new(T::zero(), T::one()),
            vec,
        )
    }

    #[inline]
    pub fn rotation(rot: Rotation2<T>) -> Affine2<T> {
        Affine2::new(
            Vec2::new(rot.cos, rot.sin),
            Vec2::new(-rot.sin, rot.cos),
            Vec2::zero(),
        )
    }

    #[inline]
    pub fn scaling(vec: Vec2<T>) -> Affine2<T> {
        Affine2::new(
            Vec2::new(vec.x, T::zero()),
            Vec2::new(T::zero(), vec.y),
            Vec2::zero(),
        )
    }

    #[inline]
    pub fn inverse(&self) -> Affine2<T> {
        let det = self.x.x * self.y.y - self.x.y * self.y.x;
        let x = Vec2::new(self.y.y, -self.x.y) / det;
        let y = Vec2::new(-self.y.x, self.x.x) / det;
        let z = Vec2::new(
            -x.x * self.z.x - y.x * self.z.y,
            -x.y * self.z.x - y.y * self.z.y,
        );
        Affine2::new(x, y, z)
    }

    fn x_row(&self) -> Vec2<T> {
        Vec2::new(self.x.x, self.y.x)
    }

    fn y_row(&self) -> Vec2<T> {
        Vec2::new(self.x.y, self.y.y)
    }

    #[inline]
    pub fn transform_vector(&self, vec: Vec2<T>) -> Vec2<T> {
        Vec2::new(self.x_row().dot(vec), self.y_row().dot(vec))
    }

    #[inline]
    pub fn transform_point(&self, vec: Vec2<T>) -> Vec2<T> {
        self.transform_vector(vec) + self.z
    }
}

impl<T: Float> Mul for Affine2<T> {
    type Output = Affine2<T>;

    #[inline]
    fn mul(self, rhs: Affine2<T>) -> Affine2<T> {
        let xx = self.x_row().dot(rhs.x);
        let xy = self.y_row().dot(rhs.x);

        let yx = self.x_row().dot(rhs.y);
        let yy = self.y_row().dot(rhs.y);

        let zx = self.x_row().dot(rhs.z) + self.z.x;
        let zy = self.y_row().dot(rhs.z) + self.z.y;

        Affine2::new(Vec2::new(xx, xy), Vec2::new(yx, yy), Vec2::new(zx, zy))
    }
}

impl<T: Float> Default for Affine2<T> {
    fn default() -> Affine2<T> {
        Affine2::identity()
    }
}
