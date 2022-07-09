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

type CellConflicts = Vec<CellIndex>;

fn make_cell_conflicts(handler_set: &HandlerSet, shape: &Shape) -> Vec<CellConflicts> {
    let mut result: Vec<CellConflicts> = vec![Vec::new(); shape.num_cells];

    for handler in handler_set.iter() {
        let conflicts = handler.conflict_set();
        for c1 in conflicts {
            for c2 in conflicts {
                if c1 != c2 {
                    result[*c1].push(*c2);
                }
            }
        }
    }

    result
}

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
    shape: Shape,
    stack: Vec<CellIndex>,
    grids: Vec<Grid>,
    cell_conflicts: Vec<CellConflicts>,
    handler_set: HandlerSet,
    cell_accumulator: CellAccumulator,
    backtrack_triggers: Vec<u32>,
    progress_ratio_stack: Vec<f64>,
    counters: Counters,
    done: bool,
    depth: usize,
    progress_callback: ProgressCallback,
}

struct SolverOutput {
    solution: Vec<CellValue>,
}

impl Iterator for Solver {
    type Item = SolverOutput;

    fn next(&mut self) -> Option<Self::Item> {
        self.run();
        if self.done {
            return None;
        }

        let solution = self.grids.last().unwrap().iter().map(|vs| vs.value());

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
        let handler_set = handlers::make_handlers(shape);
        let cell_accumulator = CellAccumulator::new(shape.num_cells, &handler_set);

        let empty_grid = vec![ValueSet::full(shape.num_values); shape.num_cells];
        let mut grids = vec![empty_grid; shape.num_cells + 1];

        for (cell, value) in fixed_values {
            grids[0][*cell] = ValueSet::from_value(*value);
        }

        Solver {
            shape: *shape,
            stack: (0..shape.num_cells).collect(),
            grids,
            cell_conflicts: make_cell_conflicts(&handler_set, shape),
            handler_set,
            cell_accumulator,
            backtrack_triggers: vec![0; shape.num_cells],
            progress_ratio_stack: vec![1.0; shape.num_cells + 1],
            counters: Counters::default(),
            done: false,
            depth: 0,
            progress_callback,
        }
    }

    fn run(&mut self) {
        if self.done {
            return;
        }

        if self.counters.cells_searched == 0 {
            let count = self.update_cell_order(self.depth);
            self.depth += 1;
            self.progress_ratio_stack[self.depth] =
                self.progress_ratio_stack[self.depth - 1] / (count as f64);
            self.counters.cells_searched += 1;
        }

        let progress_frequency_mask = self.progress_callback.frequency_mask;

        while self.depth > 0 {
            let (grids_front, grids_back) = self.grids.split_at_mut(self.depth);
            self.depth -= 1;

            let mut grid = &mut grids_front[self.depth];
            let cell = self.stack[self.depth];
            let values = &mut grid[cell];

            // No more values to try.
            if values.is_empty() {
                continue;
            }

            // Find the next value to try.
            let value = values.min();
            self.counters.values_tried += 1;
            self.counters.guesses += (*values != value) as u64;
            values.remove(value);

            // Copy the current cell values.
            self.depth += 1;
            grids_back[0].copy_from_slice(grid);

            // Update the grid with the trial value.
            grid = &mut grids_back[0];
            grid[cell] = value;

            // Propograte constraints.
            let has_contradiction = !Self::enforce_value(
                &mut grid,
                value,
                cell,
                &self.cell_conflicts,
                &mut self.cell_accumulator,
                &self.handler_set,
            );
            if has_contradiction {
                self.counters.progress_ratio += self.progress_ratio_stack[self.depth];
                self.record_backtrack(cell);
            }
            if 0 == self.counters.values_tried & progress_frequency_mask {
                (self.progress_callback.callback)(&self.counters);
            }
            if has_contradiction {
                continue;
            }

            // Check if we have a solution.
            if self.depth == self.shape.num_cells {
                self.counters.solutions += 1;
                self.counters.progress_ratio += self.progress_ratio_stack[self.depth];
                return;
            }

            // Find the next cell to try.
            let count = self.update_cell_order(self.depth);
            self.depth += 1;
            self.progress_ratio_stack[self.depth] =
                self.progress_ratio_stack[self.depth - 1] / (count as f64);
            self.counters.cells_searched += 1;
        }

        self.done = true;
        (self.progress_callback.callback)(&self.counters);
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

    // Find the best cell and bring it to the front. This means that it will
    // be processed next.
    fn update_cell_order(&mut self, depth: usize) -> u32 {
        let mut best_index = 0;
        let mut min_score = u32::MAX;

        let stack = &mut self.stack;
        let grid = &mut self.grids[depth];

        for (i, cell) in stack.iter().enumerate().skip(depth) {
            let count = grid[*cell].count();

            // If we have a single value then just use it - as it will involve no
            // guessing.
            if count <= 1 {
                best_index = i;
                break;
            }

            let bt = self.backtrack_triggers[*cell];
            let score = if bt > 1 { count / bt } else { count };

            if score < min_score {
                best_index = i;
                min_score = score;
            }
        }

        // Swap the best cell into place.
        (stack[best_index], stack[depth]) = (stack[depth], stack[best_index]);

        grid[stack[depth]].count()
    }

    fn enforce_value(
        grid: &mut Grid,
        value: ValueSet,
        cell: CellIndex,
        cell_conflicts: &[CellConflicts],
        cell_accumulator: &mut CellAccumulator,
        handler_set: &HandlerSet,
    ) -> bool {
        cell_accumulator.clear();
        cell_accumulator.add(cell);

        for conflict_cell in &cell_conflicts[cell] {
            let values = &mut grid[*conflict_cell];
            if *values != value {
                cell_accumulator.add(*conflict_cell);
            }
            values.remove(value);
            if values.is_empty() {
                return false;
            }
        }

        Self::enforce_constraints(grid, cell_accumulator, handler_set)
    }

    fn enforce_constraints(
        grid: &mut Grid,
        cell_accumulator: &mut CellAccumulator,
        handler_set: &HandlerSet,
    ) -> bool {
        while let Some(handler_index) = cell_accumulator.pop() {
            cell_accumulator.hold(handler_index);
            let handler = &handler_set[handler_index];
            if !handler.enforce_consistency(grid, cell_accumulator) {
                return false;
            }
            cell_accumulator.clear_hold();
        }
        true
    }
}
