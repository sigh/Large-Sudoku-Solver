pub mod all_different;
mod cell_accumulator;
mod engine;
mod handlers;

use crate::types::{Constraint, FixedValues, RngType, Solution};

pub const VALID_NUM_VALUE_RANGE: std::ops::RangeInclusive<u32> = engine::VALID_NUM_VALUE_RANGE;

pub type ProgressCallback = dyn FnMut(&Counters);
pub type MinimizerProgressCallback = dyn FnMut(&MinimizerCounters);

#[derive(Default)]
pub struct Config {
    pub no_guesses: bool,
    pub progress_callback: Option<Box<ProgressCallback>>,
    pub search_randomizer: Option<RngType>,
    pub output_type: OutputType,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Counters {
    pub solutions: u64,
    pub guesses: u64,
    pub constraints_processed: u64,
    pub values_tried: u64,
    pub cells_searched: u64,
    pub backtracks: u64,
    pub progress_ratio: f64,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct MinimizerCounters {
    pub cells_tried: u64,
    pub cells_removed: u64,
}

#[derive(Default, PartialEq)]
pub enum OutputType {
    #[default]
    Solution,
    Guesses,
    Empty,
}

pub enum Output {
    Solution(Solution),
    Guesses(FixedValues),
    Empty,
}

pub struct Solutions {
    runner: Box<dyn engine::Runner>,
}
impl Iterator for Solutions {
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        self.runner.next()
    }
}

pub fn solution_iter(constraint: &Constraint, config: Config) -> Solutions {
    return Solutions {
        runner: engine::make_runner(constraint, config),
    };
}

pub fn minimize(
    constraint: &Constraint,
    mut config: Config,
    progress_callback: Option<Box<MinimizerProgressCallback>>,
) -> Box<dyn Iterator<Item = FixedValues>> {
    config.output_type = OutputType::Empty;
    Box::new(Minimizer {
        runner: engine::make_runner(constraint, config),
        remaining_values: constraint.fixed_values.clone(),
        required_values: Vec::new(),
        progress_callback,
        counters: MinimizerCounters::default(),
    })
}

struct Minimizer {
    runner: Box<dyn engine::Runner>,
    remaining_values: FixedValues,
    required_values: FixedValues,
    progress_callback: Option<Box<MinimizerProgressCallback>>,
    counters: MinimizerCounters,
}

impl Iterator for Minimizer {
    type Item = FixedValues;

    fn next(&mut self) -> Option<Self::Item> {
        let fixed_values = loop {
            maybe_call_callback(&mut self.progress_callback, &self.counters);

            let item = self.remaining_values.pop()?;
            let fixed_values =
                [self.remaining_values.clone(), self.required_values.clone()].concat();

            self.runner.reset_fixed_values(&fixed_values);

            self.counters.cells_tried += 1;

            if self.runner.next().is_none() {
                // No solutions, this is usually because it aborted early due to
                // the no_guesses requirement - so keep the value.
                // If this puzzle was already inconsistent, then we don't care.
                self.required_values.push(item);
            } else if self.runner.next().is_none() {
                // One solution, return it!
                self.counters.cells_removed += 1;
                break fixed_values;
            } else {
                // Multiple solutions - this was required.
                self.required_values.push(item);
            }
        };

        Some(fixed_values)
    }
}

fn maybe_call_callback<A, F: FnMut(A)>(f: &mut Option<F>, arg: A) {
    if let Some(f) = f {
        (f)(arg);
    }
}
