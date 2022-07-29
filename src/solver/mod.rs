pub mod all_different;
mod cell_accumulator;
mod engine;
mod handlers;
mod minimizer;

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
    config: Config,
    progress_callback: Option<Box<MinimizerProgressCallback>>,
) -> Box<dyn Iterator<Item = FixedValues>> {
    minimizer::make(constraint, config, progress_callback)
}

fn maybe_call_callback<A, F: FnMut(A)>(f: &mut Option<F>, arg: A) {
    if let Some(f) = f {
        (f)(arg);
    }
}
