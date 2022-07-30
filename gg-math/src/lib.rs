mod affine2;
mod rect;
mod rotation2;
mod side_offsets;
mod vec2;

use num_traits::Float;

pub use self::affine2::Affine2;
pub use self::rect::Rect;
pub use self::rotation2::Rotation2;
pub use self::side_offsets::SideOffsets;
pub use self::vec2::Vec2;

#[inline]
pub fn lerp<T: Float>(start: T, end: T, time: T) -> T {
    start + (end - start) * time
}
