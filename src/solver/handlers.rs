use crate::types::CellIndex;
use crate::types::Shape;
use crate::types::ValueSet;

use std::cell::RefCell;
use std::ops::Deref;

use super::all_different;

type BoxedHandler = Box<dyn ConstraintHandler>;
type HandlerIndex = usize;

pub trait ConstraintHandler {
    // Remove inconsistent values from grid.
    // Return false if the grid is inconsistent.
    fn enforce_consistency(
        &self,
        grid: &mut [ValueSet],
        cell_accumulator: &mut CellAccumulator,
    ) -> bool;

    // Cells on which to enforce this constraint.
    fn cells(&self) -> &[CellIndex];

    // Cells which must have mutually exclusive values.
    fn conflict_set(&self) -> &[CellIndex];
}

use all_different::AllDifferentEnforcer;
struct HouseHandler {
    cells: Vec<CellIndex>,
    all_values: ValueSet,
    all_diff: RefCell<AllDifferentEnforcer>,
}

impl HouseHandler {
    pub fn new(cells: Vec<CellIndex>, shape: &Shape) -> HouseHandler {
        HouseHandler {
            cells,
            all_values: ValueSet::full(shape.num_values),
            all_diff: RefCell::new(AllDifferentEnforcer::new(shape.num_values)),
        }
    }
}

impl ConstraintHandler for HouseHandler {
    fn enforce_consistency(
        &self,
        grid: &mut [ValueSet],
        cell_accumulator: &mut CellAccumulator,
    ) -> bool {
        let mut all_values = ValueSet::empty();
        let mut fixed_values = ValueSet::empty();

        for cell in &self.cells {
            let v = grid[*cell];
            all_values |= v;
            if v.count() == 1 {
                fixed_values |= v;
            }
        }

        if all_values != self.all_values {
            return false;
        }
        if fixed_values == self.all_values {
            return true;
        }

        self.all_diff
            .borrow_mut()
            .enforce_all_different(grid, &self.cells, cell_accumulator)
    }

    fn cells(&self) -> &[CellIndex] {
        &self.cells
    }

    fn conflict_set(&self) -> &[CellIndex] {
        &self.cells
    }
}

pub struct HandlerSet {
    handlers: Vec<BoxedHandler>,
}

impl Deref for HandlerSet {
    type Target = Vec<BoxedHandler>;

    fn deref(&self) -> &Self::Target {
        &self.handlers
    }
}

impl HandlerSet {
    pub fn new() -> HandlerSet {
        HandlerSet {
            handlers: Vec::new(),
        }
    }

    pub fn add(&mut self, handler: BoxedHandler) {
        self.handlers.push(handler);
    }
}

fn make_houses(shape: &Shape) -> Vec<Vec<CellIndex>> {
    let mut houses = Vec::new();
    let side_len = shape.side_len;
    let box_size = shape.box_size;

    // Make rows.
    for r in 0..side_len {
        let mut house = vec![0; side_len as usize];
        for c in 0..side_len {
            house[c as usize] = shape.make_cell_index(r, c);
        }
        houses.push(house);
    }

    // Make columns.
    for c in 0..side_len {
        let mut house = vec![0; side_len as usize];
        for r in 0..side_len {
            house[r as usize] = shape.make_cell_index(r, c);
        }
        houses.push(house);
    }

    // Make boxes.
    for b in 0..side_len {
        let mut house = vec![0; side_len as usize];
        for i in 0..side_len {
            let r = (b % box_size) * box_size + (i / box_size);
            let c = (b / box_size) * box_size + (i % box_size);
            house[i as usize] = shape.make_cell_index(r, c);
        }
        houses.push(house);
    }

    houses
}

pub fn make_handlers(shape: &Shape) -> HandlerSet {
    let mut handler_set = HandlerSet::new();

    let houses = make_houses(shape);
    for house in houses {
        let handler = HouseHandler::new(house.clone(), shape);
        handler_set.add(Box::new(handler));
    }

    handler_set
}

struct IndexLinkedList {
    linked_list: Vec<usize>,
    head: usize,
    hold: usize,
}

impl IndexLinkedList {
    const NOT_IN_LIST: usize = usize::MAX;
    const NIL: usize = usize::MAX - 1;

    fn new(size: usize) -> IndexLinkedList {
        IndexLinkedList {
            linked_list: vec![Self::NOT_IN_LIST; size],
            head: Self::NIL,
            hold: Self::NIL,
        }
    }

    fn add(&mut self, index: usize) {
        if self.linked_list[index] == Self::NOT_IN_LIST {
            self.linked_list[index] = self.head;
            self.head = index;
        }
    }

    fn clear(&mut self) {
        while self.head != Self::NIL {
            let new_head = self.linked_list[self.head];
            self.linked_list[self.head] = Self::NOT_IN_LIST;
            self.head = new_head;
        }
        self.clear_hold();
    }

    fn clear_hold(&mut self) {
        while self.hold != Self::NIL {
            let new_hold = self.linked_list[self.hold];
            self.linked_list[self.hold] = Self::NOT_IN_LIST;
            self.hold = new_hold;
        }
    }

    fn pop(&mut self) -> Option<usize> {
        match self.head {
            Self::NIL => None,
            index => {
                self.head = self.linked_list[index];
                self.linked_list[index] = Self::NOT_IN_LIST;
                Some(index)
            }
        }
    }

    fn hold(&mut self, index: usize) {
        if self.linked_list[index] == Self::NOT_IN_LIST {
            self.linked_list[index] = self.hold;
            self.hold = index;
        }
    }
}

pub struct CellAccumulator {
    cell_to_handlers: Vec<Vec<HandlerIndex>>,
    linked_list: IndexLinkedList,
}

impl CellAccumulator {
    pub fn new(num_cells: usize, handler_set: &HandlerSet) -> CellAccumulator {
        let mut cell_to_handlers = vec![Vec::new(); num_cells];
        for (index, handler) in handler_set.iter().enumerate() {
            for cell in handler.cells() {
                cell_to_handlers[*cell].push(index);
            }
        }

        CellAccumulator {
            cell_to_handlers,
            linked_list: IndexLinkedList::new(handler_set.len()),
        }
    }

    pub fn add(&mut self, cell: CellIndex) {
        for &handler_index in &self.cell_to_handlers[cell] {
            self.linked_list.add(handler_index);
        }
    }

    pub fn clear(&mut self) {
        self.linked_list.clear();
    }

    pub fn pop(&mut self) -> Option<usize> {
        self.linked_list.pop()
    }

    pub fn hold(&mut self, index: usize) {
        self.linked_list.hold(index)
    }

    pub fn clear_hold(&mut self) {
        self.linked_list.clear_hold()
    }
}
