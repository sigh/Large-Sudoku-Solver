mod all_different;
mod handlers;

use crate::solver::handlers::HandlerSet;
use crate::types::CellIndex;
use crate::types::CellValue;
use crate::types::FixedValues;
use crate::types::Shape;
use crate::types::ValueSet;

use self::handlers::CellAccumulator;

pub fn solve(shape: &Shape, fixed_values: &FixedValues) {
    const LOG_UPDATE_FREQUENCY: u64 = 12;
    let progress_callback = ProgressCallback {
        callback: |counters| println!("{:?}", counters),
        frequency_mask: (1 << LOG_UPDATE_FREQUENCY) - 1,
    };
    let solver = Solver::new(shape, fixed_values, progress_callback);

    for (i, result) in solver.enumerate() {
        if i > 1 {
            panic!("Too many solutions.");
        }
        println!("{:?}", result.solution);
    }
}

struct ProgressCallback {
    callback: fn(&Counters),
    frequency_mask: u64,
}

type Grid = Vec<ValueSet>;

#[derive(Copy, Clone, Debug, Default)]
pub struct Counters {
    values_tried: u64,
    cells_searched: u64,
    backtracks: u64,
    guesses: u64,
    solutions: u64,
    progress_ratio: f64,
}

struct Solver {
    num_cells: usize,
    cell_order: Vec<CellIndex>,
    rec_stack: Vec<usize>,
    grid_stack: Vec<Grid>,
    handler_set: HandlerSet,
    cell_accumulator: CellAccumulator,
    backtrack_triggers: Vec<u32>,
    progress_ratio_stack: Vec<f64>,
    counters: Counters,
    progress_callback: ProgressCallback,
}

struct SolverOutput {
    solution: Vec<CellValue>,
}

impl Iterator for Solver {
    type Item = SolverOutput;

    fn next(&mut self) -> Option<Self::Item> {
        let grid = self.run()?;

        let solution = grid.iter().map(|vs| vs.value());

        Some(Self::Item {
            solution: solution.collect(),
        })
    }
}

impl Solver {
    fn new(
        shape: &Shape,
        fixed_values: &FixedValues,
        progress_callback: ProgressCallback,
    ) -> Solver {
        let num_cells = shape.num_cells;
        let handler_set = handlers::make_handlers(shape);
        let cell_accumulator = CellAccumulator::new(num_cells, &handler_set);

        let empty_grid = vec![ValueSet::full(shape.num_values); num_cells];
        let mut grids = vec![empty_grid; num_cells + 1];

        for (cell, value) in fixed_values {
            grids[0][*cell] = ValueSet::from_value(*value);
        }

        Solver {
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

    fn run(&mut self) -> Option<&Grid> {
        let progress_frequency_mask = self.progress_callback.frequency_mask;
        let mut new_cell_index = false;

        if self.counters.cells_searched == 0 {
            // Initialize by finding and running all handlers.
            for i in 0..self.num_cells {
                self.cell_accumulator.add(i);
            }
            if handlers::enforce_constraints(
                &mut self.grid_stack[0],
                &mut self.cell_accumulator,
                &self.handler_set,
            ) {
                // Only start the search if we successfully enforced constraints.
                self.rec_stack.push(0);
                new_cell_index = true;
            }
        }

        while let Some(mut cell_index) = self.rec_stack.pop() {
            let grid_index = self.rec_stack.len();

            // Output a solution if required.
            if cell_index == self.num_cells {
                self.counters.solutions += 1;
                self.counters.progress_ratio += self.progress_ratio_stack[grid_index];
                return Some(&self.grid_stack[grid_index]);
            }

            // First time we've seen this cell (on this branch).
            if new_cell_index {
                // Skip past all the fixed values.
                // NOTE: We can't have zero values here, as they would have been
                // rejected in the constraint propogation phase.
                cell_index = self.skip_fixed_cells(grid_index, cell_index);

                // If we are at the end then continue so that we output a solution.
                if cell_index == self.num_cells {
                    self.rec_stack.push(cell_index);
                    continue;
                }
                // Find the next cell to explore.
                self.update_cell_order(grid_index, cell_index);

                // Update counters.
                let count = self.grid_stack[grid_index][self.cell_order[cell_index]].count();
                self.progress_ratio_stack[grid_index + 1] =
                    self.progress_ratio_stack[grid_index] / (count as f64);
                self.counters.cells_searched += 1;

                new_cell_index = false;
            }

            // Now we know that the next cell has multiple values.

            let (grids_front, grids_back) = self.grid_stack.split_at_mut(grid_index + 1);
            let mut grid = &mut grids_front[grid_index];
            let cell = self.cell_order[cell_index];
            let values = &mut grid[cell];

            // No more values to try.
            if values.is_empty() {
                continue;
            }

            // We know we want to come back to this depth.
            self.rec_stack.push(cell_index);

            // Find the next value to try.
            let value = values.min();
            self.counters.values_tried += 1;
            self.counters.guesses += (*values != value) as u64;
            values.remove(value);

            if 0 == self.counters.guesses & progress_frequency_mask {
                (self.progress_callback.callback)(&self.counters);
            }

            // Copy the current cell values.
            grids_back[0].copy_from_slice(grid);

            // Update the grid with the trial value.
            grid = &mut grids_back[0];
            grid[cell] = value;

            // Propograte constraints.
            self.cell_accumulator.add(cell);
            let has_contradiction =
                !handlers::enforce_constraints(grid, &mut self.cell_accumulator, &self.handler_set);
            if has_contradiction {
                self.counters.progress_ratio += self.progress_ratio_stack[grid_index + 1];
                self.record_backtrack(cell);
                continue;
            }

            // Recurse to the new cell.
            self.rec_stack.push(cell_index + 1);
            new_cell_index = true;
        }

        (self.progress_callback.callback)(&self.counters);

        None
    }

    fn record_backtrack(&mut self, cell: CellIndex) {
        const BACKTRACK_DECAY_INTERVAL: u64 = 100 * 100;
        self.counters.backtracks += 1;
        if 0 == self.counters.backtracks % BACKTRACK_DECAY_INTERVAL {
            for i in 0..self.backtrack_triggers.len() {
                self.backtrack_triggers[i] >>= 1;
            }
        }
        self.backtrack_triggers[cell] += 1;
    }

    fn skip_fixed_cells(&mut self, grid_index: usize, mut cell_index: usize) -> usize {
        let cell_order = &mut self.cell_order;
        let grid = &mut self.grid_stack[grid_index];
        for i in cell_index..cell_order.len() {
            let cell = cell_order[i];

            let count = grid[cell].count();
            if count <= 1 {
                (cell_order[i], cell_order[cell_index]) = (cell_order[cell_index], cell_order[i]);
                cell_index += 1;
                self.counters.values_tried += 1;
            }
        }
        cell_index
    }

    // Find the best cell and bring it to the front. This means that it will
    // be processed next.
    fn update_cell_order(&mut self, grid_index: usize, cell_index: usize) {
        let mut best_index = 0;
        let mut min_score = u32::MAX;

        let cell_order = &mut self.cell_order;
        let grid = &mut self.grid_stack[grid_index];

        for (i, cell) in cell_order.iter().enumerate().skip(cell_index) {
            let count = grid[*cell].count();

            let bt = self.backtrack_triggers[*cell];
            let score = if bt > 1 { count / bt } else { count };

            if score < min_score {
                best_index = i;
                min_score = score;
            }
        }

        // Swap the best cell into place.
        (cell_order[best_index], cell_order[cell_index]) =
            (cell_order[cell_index], cell_order[best_index]);
    }
}
