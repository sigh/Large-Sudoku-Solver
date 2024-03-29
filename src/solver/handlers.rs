use std::ops::Deref;

use crate::types::{CellIndex, Constraint, Shape, ValueType};
use crate::value_set::ValueSet;

use super::all_different::AllDifferentEnforcer;
use super::cell_accumulator::{CellAccumulator, CellContainer};

pub struct Contradition;
pub type Result = std::result::Result<(), Contradition>;

pub struct HouseHandler<VS> {
    cells: Vec<CellIndex>,
    all_values: VS,
    num_values: usize,
    candidate_matching: Vec<VS>,
}

impl<VS: ValueSet> HouseHandler<VS> {
    pub fn new(cells: Vec<CellIndex>, shape: &Shape) -> Self {
        Self {
            cells,
            num_values: shape.num_values as usize,
            all_values: VS::full(shape.num_values as ValueType),
            candidate_matching: vec![VS::empty(); shape.num_values as usize],
        }
    }

    fn enforce_consistency(
        &mut self,
        grid: &mut [VS],
        cell_accumulator: &mut CellAccumulator,
        all_diff_enforcer: &mut AllDifferentEnforcer<VS>,
    ) -> Result {
        let mut all_values = VS::empty();
        // Counts the number of cells with only a single values.
        let mut num_fixed = 0;

        for &cell in &self.cells {
            let v = grid[cell];
            all_values.add_set(&v);
            // Assumes that no cells have zero values.
            num_fixed += (!v.has_multiple()) as usize;
        }

        if all_values != self.all_values {
            return Err(Contradition);
        }
        if num_fixed == self.num_values {
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

    fn enforce_consistency<VS: ValueSet>(
        &self,
        grid: &mut [VS],
        cell_accumulator: &mut CellAccumulator,
    ) -> Result {
        // Find the values in each cell set.
        let values0 = self
            .cells0
            .iter()
            .map(|&c| grid[c])
            .fold(VS::empty(), |a, b| a.union(&b));
        let values1 = self
            .cells1
            .iter()
            .map(|&c| grid[c])
            .fold(VS::empty(), |a, b| a.union(&b));

        if values0 == values1 {
            return Ok(());
        }

        // Determine all available values.
        let values = values0.intersection(&values1);

        // Check if we have enough values.
        if (values.count() as usize) < self.cells0.len() {
            return Err(Contradition);
        }

        // Enforce the constrained value set.
        if values0 != values {
            Self::remove_extra_values(grid, &values, &self.cells0, cell_accumulator)?
        }
        if values1 != values {
            Self::remove_extra_values(grid, &values, &self.cells1, cell_accumulator)?
        }

        Ok(())
    }

    fn remove_extra_values<VS: ValueSet>(
        grid: &mut [VS],
        allowed_values: &VS,
        cells: &[CellIndex],
        cell_accumulator: &mut CellAccumulator,
    ) -> Result {
        for &c0 in cells {
            let v = grid[c0].intersection(allowed_values);
            if v.is_empty() {
                return Err(Contradition);
            }
            if v != grid[c0] {
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

pub enum ConstraintHandler<VS> {
    House(HouseHandler<VS>),
    SameValue(SameValueHandler),
}

impl<VS: ValueSet> CellContainer for ConstraintHandler<VS> {
    fn cells(&self) -> &[CellIndex] {
        match self {
            ConstraintHandler::House(h) => h.cells(),
            ConstraintHandler::SameValue(h) => h.cells(),
        }
    }
}

pub struct HandlerSet<VS: ValueSet> {
    handlers: Vec<ConstraintHandler<VS>>,
    all_diff_enforcer: AllDifferentEnforcer<VS>,
}

impl<VS: ValueSet> HandlerSet<VS> {
    fn new(shape: &Shape) -> Self {
        Self {
            handlers: Vec::new(),
            all_diff_enforcer: AllDifferentEnforcer::new(shape.num_values),
        }
    }

    pub fn run_handler(
        &mut self,
        index: usize,
        grid: &mut [VS],
        cell_accumulator: &mut CellAccumulator,
    ) -> Result {
        match &mut self.handlers[index] {
            ConstraintHandler::House(h) => {
                h.enforce_consistency(grid, cell_accumulator, &mut self.all_diff_enforcer)
            }
            ConstraintHandler::SameValue(h) => h.enforce_consistency(grid, cell_accumulator),
        }
    }
}

impl<VS: ValueSet> Deref for HandlerSet<VS> {
    type Target = [ConstraintHandler<VS>];

    fn deref(&self) -> &Self::Target {
        &self.handlers
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

    if constraint.x_sudoku {
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

pub fn make_handlers<VS: ValueSet>(constraint: &Constraint) -> HandlerSet<VS> {
    const MAX_SIZE_FOR_INTERSECTIONS: u32 = 100;

    let shape = &constraint.shape;

    let mut handler_set = HandlerSet::new(shape);

    let houses = make_houses(constraint);

    // Don't create intersections if the grid is too large.
    // The process below is quadratic, so it gets really slow. In addition,
    // the only grids people create of this size are trivially solvable without
    // backtracking.
    let mut intersection_handlers = if shape.num_values <= MAX_SIZE_FOR_INTERSECTIONS {
        make_house_intersections(&houses, shape)
    } else {
        Vec::new()
    };

    for house in houses {
        let handler = ConstraintHandler::House(HouseHandler::new(house, shape));
        handler_set.handlers.push(handler);
    }

    handler_set.handlers.append(&mut intersection_handlers);

    handler_set
}
