use std::fmt;

use gg_math::{Rect, Vec2};
use guillotiere::{AllocId, AtlasAllocator};

use super::{Allocation, AllocationId, Allocator};

pub struct TreeAllocator {
    inner: AtlasAllocator,
}

impl TreeAllocator {
    pub fn new(size: Vec2<u32>) -> TreeAllocator {
        TreeAllocator {
            inner: AtlasAllocator::new(to_size(size)),
        }
    }
}

impl fmt::Debug for TreeAllocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeAllocator").finish_non_exhaustive()
    }
}

impl Allocator for TreeAllocator {
    fn size(&self) -> Vec2<u32> {
        from_size(self.inner.size())
    }

    fn can_grow(&self) -> bool {
        true
    }

    fn grow(&mut self, new_size: Vec2<u32>) {
        self.inner.grow(to_size(new_size));
    }

    fn alloc(&mut self, size: Vec2<u32>) -> Option<Allocation> {
        let alloc = self.inner.allocate(to_size(size))?;
        Some(Allocation {
            id: AllocationId(alloc.id.serialize()),
            rect: from_rect(alloc.rectangle),
        })
    }

    fn free(&mut self, id: AllocationId) {
        self.inner.deallocate(AllocId::deserialize(id.0));
    }
}

fn to_size(size: Vec2<u32>) -> guillotiere::Size {
    guillotiere::Size::new(size.x as i32, size.y as i32)
}

fn from_size(size: guillotiere::Size) -> Vec2<u32> {
    Vec2::new(size.width, size.height).cast()
}

fn from_rect(rect: guillotiere::Rectangle) -> Rect<u32> {
    let min = Vec2::new(rect.min.x, rect.min.y);
    let max = Vec2::new(rect.max.x, rect.max.y);
    Rect::new(min, max).cast()
}
