mod grid;
mod tree;

use gg_math::{Rect, Vec2};

pub use self::grid::GridAllocator;
pub use self::tree::TreeAllocator;

pub trait Allocator: std::fmt::Debug + Send + Sync + 'static {
    fn size(&self) -> Vec2<u32>;

    fn can_grow(&self) -> bool {
        false
    }

    fn grow(&mut self, new_size: Vec2<u32>) {
        let _ = new_size;
    }

    fn alloc(&mut self, size: Vec2<u32>) -> Option<Allocation>;

    fn free(&mut self, id: AllocationId);
}

#[derive(Clone, Copy, Debug)]
pub struct Allocation {
    pub id: AllocationId,
    pub rect: Rect<u32>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocationId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AllocatorKind {
    Tree,
    Grid { cell_size: Vec2<u16> },
}

impl AllocatorKind {
    pub fn new_allocator(self, size: Vec2<u32>) -> AnyAllocator {
        match self {
            AllocatorKind::Tree => TreeAllocator::new(size).into(),
            AllocatorKind::Grid { cell_size } => {
                let grid_size = size.cast().zip_map(cell_size, |a, b| (a + b - 1) / b);
                GridAllocator::new(grid_size, cell_size).into()
            }
        }
    }
}

impl AnyAllocator {
    pub fn kind(&self) -> AllocatorKind {
        match self {
            AnyAllocator::Tree(_) => AllocatorKind::Tree,
            AnyAllocator::Grid(v) => AllocatorKind::Grid {
                cell_size: v.cell_size(),
            },
        }
    }
}

macro_rules! any_allocator {
    ($( $name:ident($ty:ty), )+) => {
        #[derive(Debug)]
        pub enum AnyAllocator {
            $( $name($ty), )+
        }

        $(
        impl From<$ty> for AnyAllocator {
            fn from(v: $ty) -> Self {
                Self::$name(v)
            }
        }
        )+

        impl Allocator for AnyAllocator {
            fn size(&self) -> Vec2<u32> {
                match self {
                    $( Self::$name(v) => v.size(), )+
                }
            }

            fn can_grow(&self) -> bool {
                match self {
                    $( Self::$name(v) => v.can_grow(), )+
                }
            }

            fn grow(&mut self, new_size: Vec2<u32>) {
                match self {
                    $( Self::$name(v) => v.grow(new_size), )+
                }
            }

            fn alloc(&mut self, size: Vec2<u32>) -> Option<Allocation> {
                match self {
                    $( Self::$name(v) => v.alloc(size), )+
                }
            }

            fn free(&mut self, id: AllocationId) {
                match self {
                    $( Self::$name(v) => v.free(id), )+
                }
            }
        }
    }
}

any_allocator! {
    Tree(TreeAllocator),
    Grid(GridAllocator),
}
