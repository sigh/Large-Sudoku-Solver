use crate::types::{CellIndex, CellValue, Constraint, ValueType};
use crate::value_set::ValueSet;

use super::cell_accumulator::CellAccumulator;
use super::handlers;
use super::{Counters, ProgressCallback, Solution};

pub struct Contradition;
pub type Result = std::result::Result<(), Contradition>;

type Grid<V> = Vec<V>;

pub struct ProgressConfig {
    pub callback: Option<Box<ProgressCallback>>,
    pub frequency_mask: u64,
}

pub struct Runner<VS: ValueSet> {
    num_cells: usize,
    cell_order: Vec<CellIndex>,
    rec_stack: Vec<usize>,
    grid_stack: Vec<Grid<VS>>,
    handler_set: handlers::HandlerSet<VS>,
    cell_accumulator: CellAccumulator,
    backtrack_triggers: Vec<u32>,
    progress_ratio_stack: Vec<f64>,
    counters: Counters,
    progress_config: ProgressConfig,
}

impl<VS: ValueSet> Iterator for Runner<VS> {
    type Item = Solution;

    fn next(&mut self) -> Option<Self::Item> {
        let grid = self.run()?;

        let solution = grid
            .iter()
            .map(|vs| CellValue::from_index(vs.value().unwrap() as ValueType));

        Some(solution.collect())
    }
}

impl<VS: ValueSet> Runner<VS> {
    pub fn new(constraint: &Constraint, progress_config: ProgressConfig) -> Self {
        assert!(constraint.shape.num_values <= VS::BITS as u32);

        let num_cells = constraint.shape.num_cells;
        let handler_set = handlers::make_handlers(constraint);
        let cell_accumulator = CellAccumulator::new(num_cells, &handler_set.handlers);

        let empty_grid = vec![VS::full(constraint.shape.num_values as ValueType); num_cells];
        let mut grids = vec![empty_grid];

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
            progress_config,
        }
    }

    fn run(&mut self) -> Option<&Grid<VS>> {
        let progress_frequency_mask = self.progress_config.frequency_mask;
        let mut new_cell_index = false;

        if self.counters.values_tried == 0 {
            maybe_call_callback(&mut self.progress_config.callback, &self.counters);

            // Initialize by finding and running all handlers.
            for i in 0..self.num_cells {
                self.cell_accumulator.add(i);
            }
            if handlers::enforce_constraints(
                &mut self.grid_stack[0],
                &mut self.cell_accumulator,
                &mut self.handler_set,
                &mut self.counters,
            )
            .is_ok()
            {
                // Only start the search if we successfully enforced constraints.
                self.rec_stack.push(0);
                new_cell_index = true;
            }
            maybe_call_callback(&mut self.progress_config.callback, &self.counters);
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
                    maybe_call_callback(&mut self.progress_config.callback, &self.counters);
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
            let cell = self.cell_order[cell_index];

            let value = {
                // Find the next value to try.
                // NOTE: Do this inside a block so that grid is only borrowed
                //       in this scope. Otherwise the borrow checker gets in the
                //       way of us copying the grid to the next index.
                let grid = &mut self.grid_stack[grid_index];
                let value = match grid[cell].pop() {
                    None => continue,
                    Some(v) => VS::from_value(v),
                };

                // We know we want to come back to this index.
                self.rec_stack.push(cell_index);

                self.counters.values_tried += 1;
                self.counters.guesses += grid[cell].is_empty() as u64;

                if 0 == self.counters.guesses & progress_frequency_mask {
                    maybe_call_callback(&mut self.progress_config.callback, &self.counters);
                }

                value
            };

            self.push_grid_onto_stack(grid_index);

            // Update the grid with the trial value.
            let grid = &mut self.grid_stack[grid_index + 1];
            grid[cell] = value;

            // Propograte constraints.
            self.cell_accumulator.add(cell);
            match handlers::enforce_constraints(
                grid,
                &mut self.cell_accumulator,
                &mut self.handler_set,
                &mut self.counters,
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
        maybe_call_callback(&mut self.progress_config.callback, &self.counters);

        None
    }

    // Copy grid from self.grid_stack[grid_index] to self.grid_stack[grid_index+1].
    fn push_grid_onto_stack(&mut self, grid_index: usize) {
        if self.grid_stack.len() == grid_index + 1 {
            // We've run out of space on the stack, so we need to push onto the
            // end.
            self.grid_stack.extend_from_within(grid_index..);
        } else {
            // Otherwise we copy over the existing elements.
            let (grids_front, grids_back) = self.grid_stack.split_at_mut(grid_index + 1);
            grids_back[0].copy_from_slice(&grids_front[grid_index]);
        }
    }

    fn record_backtrack(&mut self, cell: CellIndex) {
        const BACKTRACK_DECAY_INTERVAL: u64 = 50;
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

            if !grid[cell].has_multiple() {
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
            .unwrap_or((0, &0));

        // Swap the best cell into place.
        cell_order.swap(best_index, cell_index);
    }
}

fn maybe_call_callback<A, F: FnMut(A)>(f: &mut Option<F>, arg: A) {
    if let Some(f) = f {
        (f)(arg);
    }
}
