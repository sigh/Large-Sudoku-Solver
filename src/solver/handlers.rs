use crate::types::CellIndex;
use crate::types::Shape;
use crate::types::ValueSet;

use std::cell::RefCell;

type HandlerIndex = usize;

use crate::solver::all_different::AllDifferentEnforcer;

pub fn enforce_constraints(
    grid: &mut [ValueSet],
    cell_accumulator: &mut CellAccumulator,
    handler_set: &HandlerSet,
) -> bool {
    let mut all_different_enforcer = handler_set.all_diff_enforcer.borrow_mut();

    while let Some(handler_index) = cell_accumulator.pop() {
        cell_accumulator.hold(handler_index);
        let handler = &handler_set.handlers[handler_index];
        if !handler.enforce_consistency(grid, cell_accumulator, &mut all_different_enforcer) {
            cell_accumulator.clear();
            return false;
        }
        cell_accumulator.clear_hold();
    }
    true
}

pub struct HouseHandler {
    cells: Vec<CellIndex>,
    all_values: ValueSet,
    num_values: u32,
}

impl HouseHandler {
    pub fn new(cells: Vec<CellIndex>, shape: &Shape) -> HouseHandler {
        HouseHandler {
            cells,
            num_values: shape.num_values,
            all_values: ValueSet::full(shape.num_values),
        }
    }

    fn enforce_consistency(
        &self,
        grid: &mut [ValueSet],
        cell_accumulator: &mut CellAccumulator,
        all_diff_enforcer: &mut AllDifferentEnforcer,
    ) -> bool {
        let mut all_values = ValueSet::empty();
        let mut total_count = 0;

        for &cell in &self.cells {
            let v = grid[cell];
            all_values |= v;
            total_count += v.count();
        }

        if all_values != self.all_values {
            return false;
        }
        if total_count == self.num_values {
            return true;
        }

        all_diff_enforcer.enforce_all_different(grid, &self.cells, cell_accumulator)
    }

    fn cells(&self) -> &[CellIndex] {
        &self.cells
    }
}

pub struct HandlerSet {
    handlers: Vec<HouseHandler>,
    all_diff_enforcer: RefCell<AllDifferentEnforcer>,
}

impl HandlerSet {
    fn new(shape: &Shape) -> HandlerSet {
        HandlerSet {
            handlers: Vec::new(),
            all_diff_enforcer: RefCell::new(AllDifferentEnforcer::new(shape.num_values)),
        }
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
    let mut handler_set = HandlerSet::new(shape);

    let houses = make_houses(shape);
    for house in houses {
        let handler = HouseHandler::new(house, shape);
        handler_set.handlers.push(handler);
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
        for (index, handler) in handler_set.handlers.iter().enumerate() {
            for cell in handler.cells() {
                cell_to_handlers[*cell].push(index);
            }
        }

        CellAccumulator {
            cell_to_handlers,
            linked_list: IndexLinkedList::new(handler_set.handlers.len()),
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
