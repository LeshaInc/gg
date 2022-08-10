use gg_math::{Rect, Vec2};

use super::{Allocation, AllocationId, Allocator};

#[derive(Debug)]
pub struct GridAllocator {
    grid_size: Vec2<u16>,
    cell_size: Vec2<u16>,
    next_cell: Option<Vec2<u16>>,
    reusable_cells: Vec<Vec2<u16>>,
}

impl GridAllocator {
    pub fn new(grid_size: Vec2<u16>, cell_size: Vec2<u16>) -> GridAllocator {
        GridAllocator {
            grid_size,
            cell_size,
            next_cell: Some(Vec2::zero()),
            reusable_cells: Vec::new(),
        }
    }

    pub fn cell_size(&self) -> Vec2<u16> {
        self.cell_size
    }

    fn alloc_cell(&mut self) -> Option<Vec2<u16>> {
        if let Some(cell) = self.next_cell {
            self.next_cell = None
                .or_else(|| (cell.x + 1 != self.grid_size.x).then_some(cell.set_x(cell.x + 1)))
                .or_else(|| (cell.y + 1 != self.grid_size.y).then_some(cell.set_y(cell.y + 1)));

            return Some(cell);
        }

        self.reusable_cells.pop()
    }
}

impl Allocator for GridAllocator {
    fn size(&self) -> Vec2<u32> {
        self.cell_size.cast::<u32>() * self.grid_size.cast::<u32>()
    }

    fn can_grow(&self) -> bool {
        true
    }

    fn grow(&mut self, new_size: Vec2<u32>) {
        self.grid_size = (new_size / self.cell_size.cast()).cast();
    }

    fn alloc(&mut self, size: Vec2<u32>) -> Option<Allocation> {
        if size.cmp_gt(self.cell_size.cast()).any() {
            return None;
        }

        let cell = self.alloc_cell()?;

        Some(Allocation {
            id: cell_to_id(cell),
            rect: Rect::new(cell_offset(cell, self.cell_size), size),
        })
    }

    fn free(&mut self, id: AllocationId) {
        let cell = id_to_cell(id);
        self.reusable_cells.push(cell);
    }
}

fn id_to_cell(id: AllocationId) -> Vec2<u16> {
    let hi = (id.0 >> 16) as u16;
    let lo = id.0 as u16;
    Vec2::new(hi, lo)
}

fn cell_to_id(cell: Vec2<u16>) -> AllocationId {
    let hi = cell.x as u32;
    let lo = cell.y as u32;
    AllocationId((hi << 16) | lo)
}

fn cell_offset(cell: Vec2<u16>, cell_size: Vec2<u16>) -> Vec2<u32> {
    cell.cast::<u32>() * cell_size.cast::<u32>()
}
