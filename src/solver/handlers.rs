use crate::types::CellIndex;
use crate::types::Constraint;
use crate::types::Shape;
use crate::value_set::ValueSet;

use std::cell::RefCell;

type HandlerIndex = usize;

use crate::solver::all_different::AllDifferentEnforcer;

use super::Contradition;
use super::SolverResult;

pub fn enforce_constraints<VS: ValueSet + Copy>(
    grid: &mut [VS],
    cell_accumulator: &mut CellAccumulator,
    handler_set: &mut HandlerSet<VS>,
) -> SolverResult {
    let mut all_different_enforcer = handler_set.all_diff_enforcer.borrow_mut();

    while let Some(handler_index) = cell_accumulator.pop() {
        cell_accumulator.hold(handler_index);
        let handler = &mut handler_set.handlers[handler_index];
        match handler {
            ConstraintHandler::House(h) => {
                h.enforce_consistency(grid, cell_accumulator, &mut all_different_enforcer)
            }
            ConstraintHandler::SameValue(h) => h.enforce_consistency(grid, cell_accumulator),
        }
        .map_err(|e| {
            cell_accumulator.clear();
            e
        })?;

        cell_accumulator.clear_hold();
    }

    Ok(())
}

pub struct HouseHandler<VS> {
    cells: Vec<CellIndex>,
    all_values: VS,
    num_values: usize,
    candidate_matching: Vec<VS>,
}

impl<VS: ValueSet + Copy> HouseHandler<VS> {
    pub fn new(cells: Vec<CellIndex>, shape: &Shape) -> Self {
        Self {
            cells,
            num_values: shape.num_values as usize,
            all_values: VS::full(shape.num_values as u8),
            candidate_matching: vec![VS::empty(); shape.num_values as usize],
        }
    }

    fn enforce_consistency(
        &mut self,
        grid: &mut [VS],
        cell_accumulator: &mut CellAccumulator,
        all_diff_enforcer: &mut AllDifferentEnforcer<VS>,
    ) -> SolverResult {
        let mut all_values = VS::empty();
        let mut total_count = 0;

        for &cell in &self.cells {
            let v = grid[cell];
            all_values.add_set(&v);
            total_count += v.count();
        }

        if !all_values.equals(&self.all_values) {
            return Err(Contradition);
        }
        if total_count == self.num_values {
            return Ok(());
        }

        all_diff_enforcer.enforce_all_different(
            grid,
            &self.cells,
            &mut self.candidate_matching,
            cell_accumulator,
        )
    }

    fn cells(&self) -> &[CellIndex] {
        &self.cells
    }
}

pub struct SameValueHandler {
    cells: Vec<CellIndex>,
    cells0: Vec<CellIndex>,
    cells1: Vec<CellIndex>,
}

impl SameValueHandler {
    pub fn new(cells0: Vec<CellIndex>, cells1: Vec<CellIndex>) -> Self {
        let mut cells = Vec::new();
        cells.extend(cells0.iter());
        cells.extend(cells1.iter());
        Self {
            cells,
            cells0,
            cells1,
        }
    }

    fn enforce_consistency<VS: ValueSet + Copy>(
        &self,
        grid: &mut [VS],
        cell_accumulator: &mut CellAccumulator,
    ) -> SolverResult {
        // Find the values in each cell set.
        let values0 = self
            .cells0
            .iter()
            .map(|&c| grid[c])
            .reduce(|a, b| a.union(&b))
            .unwrap();
        let values1 = self
            .cells1
            .iter()
            .map(|&c| grid[c])
            .reduce(|a, b| a.union(&b))
            .unwrap();

        if values0.equals(&values1) {
            return Ok(());
        }

        // Determine all available values.
        let values = values0.intersection(&values1);

        // Check if we have enough values.
        if (values.count() as usize) < self.cells0.len() {
            return Err(Contradition);
        }

        // Enforce the constrained value set.
        if !values0.equals(&values) {
            Self::remove_extra_values(grid, &values, &self.cells0, cell_accumulator)?
        }
        if !values1.equals(&values) {
            Self::remove_extra_values(grid, &values, &self.cells1, cell_accumulator)?
        }

        Ok(())
    }

    fn remove_extra_values<VS: ValueSet>(
        grid: &mut [VS],
        allowed_values: &VS,
        cells: &[CellIndex],
        cell_accumulator: &mut CellAccumulator,
    ) -> SolverResult {
        for &c0 in cells {
            let v = grid[c0].intersection(allowed_values);
            if v.is_empty() {
                return Err(Contradition);
            }
            if !v.equals(&grid[c0]) {
                grid[c0] = v;
                cell_accumulator.add(c0);
            }
        }
        Ok(())
    }

    fn cells(&self) -> &[CellIndex] {
        &self.cells
    }
}

pub trait CellContainer {
    fn cells(&self) -> &[CellIndex];
}

pub enum ConstraintHandler<VS> {
    House(HouseHandler<VS>),
    SameValue(SameValueHandler),
}

impl<VS: ValueSet + Copy> CellContainer for ConstraintHandler<VS> {
    fn cells(&self) -> &[CellIndex] {
        match self {
            ConstraintHandler::House(h) => h.cells(),
            ConstraintHandler::SameValue(h) => h.cells(),
        }
    }
}

pub struct HandlerSet<VS: ValueSet> {
    pub handlers: Vec<ConstraintHandler<VS>>,
    all_diff_enforcer: RefCell<AllDifferentEnforcer<VS>>,
}

impl<VS: ValueSet + Copy> HandlerSet<VS> {
    fn new(shape: &Shape) -> Self {
        Self {
            handlers: Vec::new(),
            all_diff_enforcer: RefCell::new(AllDifferentEnforcer::new(shape.num_values)),
        }
    }
}

fn make_houses(constraint: &Constraint) -> Vec<Vec<CellIndex>> {
    let mut houses = Vec::new();
    let shape = &constraint.shape;
    let side_len = shape.side_len;
    let box_size = shape.box_size;

    // Make rows.
    for r in 0..side_len {
        let f = |c| shape.make_cell_index(r, c);
        houses.push((0..side_len).map(f).collect());
    }

    // Make columns.
    for c in 0..side_len {
        let f = |r| shape.make_cell_index(r, c);
        houses.push((0..side_len).map(f).collect());
    }

    // Make boxes.
    for b in 0..side_len {
        let f = |i| {
            let r = (b % box_size) * box_size + (i / box_size);
            let c = (b / box_size) * box_size + (i % box_size);
            shape.make_cell_index(r, c)
        };
        houses.push((0..side_len).map(f).collect());
    }

    if constraint.sudoku_x {
        let f = |r| shape.make_cell_index(r, r);
        houses.push((0..side_len).map(f).collect());

        let f = |r| shape.make_cell_index(r, side_len - r - 1);
        houses.push((0..side_len).map(f).collect());
    }

    houses
}

fn array_intersection_size<T: PartialEq>(v0: &[T], v1: &[T]) -> usize {
    v0.iter().filter(|e| v1.contains(e)).count()
}

fn array_difference<T: PartialEq + Copy>(v0: &[T], v1: &[T]) -> Vec<T> {
    v0.iter().filter(|e| !v1.contains(e)).copied().collect()
}

fn make_house_intersections<VS>(
    houses: &[Vec<CellIndex>],
    shape: &Shape,
) -> Vec<ConstraintHandler<VS>> {
    let box_size = shape.box_size as usize;

    let mut handlers = Vec::new();

    for (i, h0) in houses.iter().enumerate() {
        for h1 in houses.iter().skip(i + 1) {
            if array_intersection_size(h0, h1) == box_size {
                let handler =
                    SameValueHandler::new(array_difference(h0, h1), array_difference(h1, h0));
                handlers.push(ConstraintHandler::SameValue(handler));
            }
        }
    }

    handlers
}

pub fn make_handlers<VS: ValueSet + Copy>(constraint: &Constraint) -> HandlerSet<VS> {
    let shape = &constraint.shape;

    let mut handler_set = HandlerSet::new(shape);

    let houses = make_houses(constraint);
    let mut intersection_handlers = make_house_intersections(&houses, shape);

    for house in houses {
        let handler = ConstraintHandler::House(HouseHandler::new(house, shape));
        handler_set.handlers.push(handler);
    }

    handler_set.handlers.append(&mut intersection_handlers);

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
    pub fn new<H: CellContainer>(num_cells: usize, handlers: &[H]) -> CellAccumulator {
        let mut cell_to_handlers = vec![Vec::new(); num_cells];
        for (index, handler) in handlers.iter().enumerate() {
            for cell in handler.cells() {
                cell_to_handlers[*cell].push(index);
            }
        }

        CellAccumulator {
            cell_to_handlers,
            linked_list: IndexLinkedList::new(handlers.len()),
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
