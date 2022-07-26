use crate::types::{CellIndex, CellValue, Constraint, FixedValues, ValueType};
use crate::value_set::ValueSet;
use rand::prelude::SliceRandom;

use super::cell_accumulator::CellAccumulator;
use super::{handlers, SolutionIter};
use super::{Config, Counters, Output, OutputType, ProgressCallback};

pub struct Contradition;
pub type Result = std::result::Result<(), Contradition>;

type Grid<V> = Vec<V>;

pub struct Engine<VS: ValueSet> {
    started: bool,
    cell_order: Vec<CellIndex>,
    rec_stack: Vec<usize>,
    grid_stack: Vec<Grid<VS>>,
    full_cell: VS,
    handler_set: handlers::HandlerSet<VS>,
    cell_accumulator: CellAccumulator,
    backtrack_triggers: Vec<u32>,
    progress_metadata: ProgressMetadata,
    progress_ratio_stack: Vec<f64>,
    counters: Counters,
    config: Config,
}

impl<VS: ValueSet> Iterator for Engine<VS> {
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        let grid_to_solution = |grid: &Grid<VS>| {
            grid.iter()
                .map(|vs| {
                    CellValue::from_index(
                        vs.value()
                            .unwrap_or_else(|| panic!("Bad ValueSet in solution: {:?}", vs))
                            as ValueType,
                    )
                })
                .collect::<Vec<_>>()
        };

        match self.config.output_type {
            OutputType::Empty => self.run().map(|_| Output::Empty),

            OutputType::Solution => self.run().map(grid_to_solution).map(Output::Solution),

            OutputType::Guesses => self.run().map(grid_to_solution).map(|solution| {
                Output::Guesses(
                    self.rec_stack
                        .iter()
                        .map(|i| self.cell_order[*i])
                        .map(|c| (c, solution[c]))
                        .collect(),
                )
            }),
        }
    }
}

impl<VS: ValueSet> Engine<VS> {
    pub fn new(constraint: &Constraint, mut config: Config) -> Self {
        assert!(constraint.shape.num_values <= VS::BITS as u32);

        let num_cells = constraint.shape.num_cells;
        let handler_set = handlers::make_handlers(constraint);
        let cell_accumulator = CellAccumulator::new(num_cells, &handler_set);

        let mut cell_order = (0..num_cells).collect::<Vec<_>>();
        if let Some(rng) = &mut config.search_randomizer {
            cell_order.shuffle(rng);
        };

        let progress_metadata = ProgressMetadata::new(&mut config.progress_callback);

        let mut new = Self {
            started: false,
            cell_order,
            rec_stack: Vec::with_capacity(num_cells),
            grid_stack: vec![vec![VS::empty(); num_cells]],
            full_cell: VS::full(constraint.shape.num_values as ValueType),
            handler_set,
            cell_accumulator,
            backtrack_triggers: vec![0; num_cells],
            progress_ratio_stack: vec![1.0; num_cells + 1],
            counters: Counters::default(),
            progress_metadata,
            config,
        };

        new.reset_fixed_values(&constraint.fixed_values);

        new
    }

    fn run(&mut self) -> Option<&Grid<VS>> {
        let mut new_cell_index = false;
        let mut progress_delta = 1.0;
        let num_cells = self.cell_order.len();
        let remember_guesses = self.config.output_type == OutputType::Guesses;

        if !self.started {
            self.started = true;

            self.progress_metadata.maybe_call(&self.counters);

            // Initialize by finding and running all handlers.
            for i in 0..num_cells {
                self.cell_accumulator.add(i);
            }
            if self.enforce_consistency().is_ok() {
                // Only start the search if we successfully enforced constraints.

                // Handle the no guesses case - the initial enforce constraints round should have found everything.
                let first_cell_index = if self.config.no_guesses {
                    if self.skip_fixed_cells(0) != num_cells {
                        return None;
                    } else {
                        num_cells
                    }
                } else {
                    0
                };
                self.rec_stack.push(first_cell_index);

                new_cell_index = true;
            }
            self.progress_metadata.maybe_call(&self.counters);
        }

        while let Some(mut cell_index) = self.rec_stack.pop() {
            let grid_index = self.grid_index();

            // First time we've seen this cell (on this branch).
            if new_cell_index {
                new_cell_index = false;

                // Skip past all the fixed values.
                // NOTE: We can't have zero values here, as they would have been
                // rejected in the constraint propogation phase.
                cell_index = self.skip_fixed_cells(cell_index);

                // We've reached the end, so output a solution!
                if cell_index == num_cells {
                    self.counters.solutions += 1;
                    self.counters.progress_ratio += progress_delta;
                    self.progress_metadata.maybe_call(&self.counters);
                    return Some(&self.grid_stack[grid_index]);
                }

                // Find the next cell to explore.
                self.update_cell_order(cell_index);

                // Update counters.
                let count = self.grid_stack[grid_index][self.cell_order[cell_index]].count();
                self.progress_ratio_stack[grid_index] = progress_delta / (count as f64);
                self.counters.cells_searched += 1;
            }
            progress_delta = self.progress_ratio_stack[grid_index];

            // Now we know that the next cell has (or had) multiple values.
            let cell = self.cell_order[cell_index];

            // We are trying a new value.
            self.counters.values_tried += 1;

            if remember_guesses || self.grid_stack[grid_index][cell].has_multiple() {
                // There are more values left, so push the current cell onto the
                // stack and copy the grid to create a new stack frame.

                let v = self.grid_stack[grid_index][cell].pop().unwrap_or_default();

                self.counters.guesses += 1;
                self.progress_metadata
                    .maybe_call_thottled(self.counters.constraints_processed, &self.counters);

                self.push_grid_onto_stack();
                self.rec_stack.push(cell_index);

                // Update the grid with the trial value.
                self.grid_stack[grid_index + 1][cell] = VS::from_value(v);
            }

            // Propograte constraints.
            self.cell_accumulator.add(cell);
            match self.enforce_consistency() {
                Ok(()) => {
                    // Recurse to the new cell.
                    self.rec_stack.push(cell_index + 1);
                    new_cell_index = true;
                }
                Err(Contradition) => {
                    // Backtrack.
                    self.counters.progress_ratio += progress_delta;
                    self.record_backtrack(cell);
                }
            }
        }

        // Send the final set of progress counters.
        self.progress_metadata.maybe_call(&self.counters);

        None
    }

    #[inline]
    fn grid_index(&self) -> usize {
        self.rec_stack.len()
    }

    // Copy grid from self.grid_stack[grid_index] to self.grid_stack[grid_index+1].
    fn push_grid_onto_stack(&mut self) {
        let grid_index = self.grid_index();
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

    fn skip_fixed_cells(&mut self, start_cell_index: usize) -> usize {
        let grid_index = self.grid_index();
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
    fn update_cell_order(&mut self, cell_index: usize) {
        let grid_index = self.grid_index();
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

    fn enforce_consistency(&mut self) -> Result {
        let grid_index = self.grid_index();
        let grid = &mut self.grid_stack[grid_index];
        let cell_accumulator = &mut self.cell_accumulator;

        while let Some(handler_index) = cell_accumulator.pop() {
            cell_accumulator.hold(handler_index);
            self.counters.constraints_processed += 1;
            self.handler_set
                .run_handler(handler_index, grid, cell_accumulator)
                .map_err(|e| {
                    cell_accumulator.clear();
                    e
                })?;

            cell_accumulator.clear_hold();
        }

        Ok(())
    }
}

impl<VS: ValueSet> super::SolutionIter for Engine<VS> {
    fn reset_fixed_values(&mut self, fixed_values: &FixedValues) {
        self.started = false;
        self.rec_stack.clear();
        self.grid_stack[0].fill(self.full_cell);
        for (cell, value) in fixed_values {
            self.grid_stack[0][*cell] = VS::from_value(value.index());
        }

        // Both of these counters are confusing when aggregated.
        self.counters.progress_ratio = 0.0;
        self.counters.solutions = 0;
    }
}

struct ProgressMetadata {
    callback: Option<Box<ProgressCallback>>,
    frequency_mask: u64,
    next_check: u64,
}

impl ProgressMetadata {
    fn new(callback: &mut Option<Box<ProgressCallback>>) -> ProgressMetadata {
        const LOG_UPDATE_FREQUENCY: u64 = 21;
        const UPDATE_FREQUENCY_MASK: u64 = (1 << LOG_UPDATE_FREQUENCY) - 1;
        match callback {
            None => ProgressMetadata {
                callback: None,
                frequency_mask: u64::MAX,
                next_check: u64::MAX,
            },
            Some(_) => ProgressMetadata {
                callback: callback.take(),
                frequency_mask: UPDATE_FREQUENCY_MASK,
                next_check: 0,
            },
        }
    }

    #[inline]
    fn maybe_call_thottled(&mut self, progress_counter: u64, payload: &Counters) {
        if progress_counter > self.next_check {
            self.next_check = progress_counter | self.frequency_mask;
            self.maybe_call(payload);
        }
    }

    #[inline]
    fn maybe_call(&mut self, payload: &Counters) {
        super::maybe_call_callback(&mut self.callback, payload);
    }
}
