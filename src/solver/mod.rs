pub mod all_different;
mod handlers;

use crate::solver::handlers::HandlerSet;
use crate::types::CellIndex;
use crate::types::CellValue;
use crate::types::Constraint;
use crate::value_set::IntBitSet;
use crate::value_set::ValueSet;

use self::handlers::CellAccumulator;

pub type Solution = Vec<CellValue>;

pub const VALID_NUM_VALUE_RANGE: std::ops::RangeInclusive<u32> = 2..=128;

pub fn solution_iter(
    constraint: &Constraint,
    progress_callback: Option<Box<dyn FnMut(&Counters)>>,
) -> Box<dyn Iterator<Item = Solution>> {
    const LOG_UPDATE_FREQUENCY: u64 = 10;
    let frequency_mask = match &progress_callback {
        Some(_) => (1 << LOG_UPDATE_FREQUENCY) - 1,
        None => u64::MAX,
    };

    let progress_config = ProgressCallback {
        frequency_mask,
        callback: progress_callback,
    };

    match constraint.shape.num_values {
        2..=32 => Box::new(Solver::<IntBitSet<i32>>::new(constraint, progress_config)),
        33..=64 => Box::new(Solver::<IntBitSet<i64>>::new(constraint, progress_config)),
        65..=128 => Box::new(Solver::<IntBitSet<i128>>::new(constraint, progress_config)),
        _ => panic!(
            "Grid too large. num_values: {}",
            constraint.shape.num_values
        ),
    }
}

pub struct Contradition;

pub type SolverResult = Result<(), Contradition>;

struct ProgressCallback {
    callback: Option<Box<dyn FnMut(&Counters)>>,
    frequency_mask: u64,
}

type Grid<V> = Vec<V>;

#[derive(Copy, Clone, Debug, Default)]
pub struct Counters {
    pub values_tried: u64,
    pub cells_searched: u64,
    pub backtracks: u64,
    pub guesses: u64,
    pub solutions: u64,
    pub progress_ratio: f64,
}

struct Solver<VS: ValueSet> {
    num_cells: usize,
    cell_order: Vec<CellIndex>,
    rec_stack: Vec<usize>,
    grid_stack: Vec<Grid<VS>>,
    handler_set: HandlerSet<VS>,
    cell_accumulator: CellAccumulator,
    backtrack_triggers: Vec<u32>,
    progress_ratio_stack: Vec<f64>,
    counters: Counters,
    progress_callback: ProgressCallback,
}

impl<VS: ValueSet + Copy> Iterator for Solver<VS> {
    type Item = Solution;

    fn next(&mut self) -> Option<Self::Item> {
        let grid = self.run()?;

        let solution = grid
            .iter()
            .map(|vs| CellValue::from_index(vs.value().unwrap() as u8));

        Some(solution.collect())
    }
}

impl<VS: ValueSet + Copy> Solver<VS> {
    fn new(constraint: &Constraint, progress_callback: ProgressCallback) -> Self {
        let num_cells = constraint.shape.num_cells;
        let handler_set = handlers::make_handlers(constraint);
        let cell_accumulator = CellAccumulator::new(num_cells, &handler_set.handlers);

        let empty_grid = vec![VS::full(constraint.shape.num_values as u8); num_cells];
        let mut grids = vec![empty_grid; num_cells + 1];

        for (cell, value) in &constraint.fixed_values {
            grids[0][*cell] = VS::from_value(value.index());
        }

        Self {
            num_cells,
            cell_order: (0..num_cells).collect(),
            rec_stack: Vec::with_capacity(num_cells),
            grid_stack: grids,
            handler_set,
            cell_accumulator,
            backtrack_triggers: vec![0; num_cells],
            progress_ratio_stack: vec![1.0; num_cells + 1],
            counters: Counters::default(),
            progress_callback,
        }
    }

    fn run(&mut self) -> Option<&Grid<VS>> {
        let progress_frequency_mask = self.progress_callback.frequency_mask;
        let mut new_cell_index = false;

        if self.counters.values_tried == 0 {
            // Initialize by finding and running all handlers.
            for i in 0..self.num_cells {
                self.cell_accumulator.add(i);
            }
            if handlers::enforce_constraints(
                &mut self.grid_stack[0],
                &mut self.cell_accumulator,
                &mut self.handler_set,
            )
            .is_ok()
            {
                // Only start the search if we successfully enforced constraints.
                self.rec_stack.push(0);
                new_cell_index = true;
            }
        }

        while let Some(mut cell_index) = self.rec_stack.pop() {
            let grid_index = self.rec_stack.len();

            // First time we've seen this cell (on this branch).
            if new_cell_index {
                new_cell_index = false;

                // Skip past all the fixed values.
                // NOTE: We can't have zero values here, as they would have been
                // rejected in the constraint propogation phase.
                cell_index = self.skip_fixed_cells(grid_index, cell_index);

                // We've reached the end, so output a solution!
                if cell_index == self.num_cells {
                    self.counters.solutions += 1;
                    self.counters.progress_ratio += self.progress_ratio_stack[grid_index];
                    return Some(&self.grid_stack[grid_index]);
                }

                // Find the next cell to explore.
                self.update_cell_order(grid_index, cell_index);

                // Update counters.
                let count = self.grid_stack[grid_index][self.cell_order[cell_index]].count();
                self.progress_ratio_stack[grid_index + 1] =
                    self.progress_ratio_stack[grid_index] / (count as f64);
                self.counters.cells_searched += 1;
            }

            // Now we know that the next cell has (or had) multiple values.

            let (grids_front, grids_back) = self.grid_stack.split_at_mut(grid_index + 1);
            let mut grid = &mut grids_front[grid_index];
            let cell = self.cell_order[cell_index];

            // Find the next value to try.
            let value = match grid[cell].pop() {
                None => continue,
                Some(v) => VS::from_value(v),
            };

            // We know we want to come back to this index.
            self.rec_stack.push(cell_index);

            self.counters.values_tried += 1;
            self.counters.guesses += grid[cell].is_empty() as u64;

            if 0 == self.counters.guesses & progress_frequency_mask {
                if let Some(f) = &mut self.progress_callback.callback {
                    (f)(&self.counters);
                }
            }

            // Copy the current cell values.
            grids_back[0].copy_from_slice(grid);

            // Update the grid with the trial value.
            grid = &mut grids_back[0];
            grid[cell] = value;

            // Propograte constraints.
            self.cell_accumulator.add(cell);
            match handlers::enforce_constraints(
                grid,
                &mut self.cell_accumulator,
                &mut self.handler_set,
            ) {
                Ok(()) => {
                    // Recurse to the new cell.
                    self.rec_stack.push(cell_index + 1);
                    new_cell_index = true;
                }
                Err(Contradition) => {
                    // Backtrack.
                    self.counters.progress_ratio += self.progress_ratio_stack[grid_index + 1];
                    self.record_backtrack(cell);
                }
            }
        }

        // Send the final set of progress counters.
        if let Some(f) = &mut self.progress_callback.callback {
            (f)(&self.counters);
        }

        None
    }

    fn record_backtrack(&mut self, cell: CellIndex) {
        const BACKTRACK_DECAY_INTERVAL: u64 = 100;
        self.counters.backtracks += 1;
        if 0 == self.counters.backtracks % BACKTRACK_DECAY_INTERVAL {
            for bt in &mut self.backtrack_triggers {
                *bt >>= 1;
            }
        }
        self.backtrack_triggers[cell] += 1;
    }

    fn skip_fixed_cells(&mut self, grid_index: usize, start_cell_index: usize) -> usize {
        let cell_order = &mut self.cell_order;
        let grid = &mut self.grid_stack[grid_index];

        let mut cell_index = start_cell_index;
        for i in start_cell_index..cell_order.len() {
            let cell = cell_order[i];

            let count = grid[cell].count();
            if count <= 1 {
                cell_order.swap(i, cell_index);
                cell_index += 1;
                self.counters.values_tried += 1;
            }
        }
        cell_index
    }

    // Find the best cell and bring it to the front. This means that it will
    // be processed next.
    fn update_cell_order(&mut self, grid_index: usize, cell_index: usize) {
        let cell_order = &mut self.cell_order;
        let grid = &mut self.grid_stack[grid_index];

        let (best_index, _) = cell_order
            .iter()
            .enumerate()
            .skip(cell_index)
            .min_by_key(|(_, cell)| {
                let count = grid[**cell].count() as u32;
                let bt = self.backtrack_triggers[**cell];

                #[allow(clippy::let_and_return)]
                let score = if bt > 1 { count / bt } else { count };
                score
            })
            .unwrap();

        // Swap the best cell into place.
        cell_order.swap(best_index, cell_index);
    }
}
