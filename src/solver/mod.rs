pub mod all_different;
mod cell_accumulator;
mod handlers;
mod runner;

use crate::types::{CellValue, Constraint, FixedValues, RngType, Solution};
use crate::value_set::IntBitSet;
#[cfg(not(feature = "i64_value_set"))]
use crate::value_set::RecValueSet;

use rand::prelude::SliceRandom;
use runner::Runner;

pub const VALID_NUM_VALUE_RANGE: std::ops::RangeInclusive<u32> = 2..=512;

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

pub trait SolutionIter: Iterator<Item = Output> {
    fn reset_fixed_values(&mut self, fixed_values: &FixedValues);
}

pub fn solution_iter(constraint: &Constraint, config: Config) -> Box<dyn SolutionIter> {
    match constraint.shape.num_values {
        #[cfg(not(feature = "i64_value_set"))]
        2..=32 => Box::new(Runner::<IntBitSet<i32>>::new(constraint, config)),
        #[cfg(not(feature = "i64_value_set"))]
        33..=64 => Box::new(Runner::<IntBitSet<i64>>::new(constraint, config)),
        #[cfg(feature = "i64_value_set")]
        2..=64 => Box::new(Runner::<IntBitSet<i64>>::new(constraint, config)),
        #[cfg(not(feature = "i64_value_set"))]
        65..=128 => Box::new(Runner::<IntBitSet<i128>>::new(constraint, config)),
        #[cfg(not(feature = "i64_value_set"))]
        129..=256 => Box::new(Runner::<RecValueSet<IntBitSet<i128>>>::new(
            constraint, config,
        )),
        #[cfg(not(feature = "i64_value_set"))]
        257..=512 => Box::new(Runner::<RecValueSet<RecValueSet<IntBitSet<i128>>>>::new(
            constraint, config,
        )),
        _ => panic!(
            "Grid too large. num_values: {}",
            constraint.shape.num_values
        ),
    }
}

pub fn minimize(
    constraint: &Constraint,
    config: Config,
    progress_callback: Option<Box<MinimizerProgressCallback>>,
) -> Box<dyn Iterator<Item = FixedValues>> {
    Box::new(Minimizer {
        runner: solution_iter(constraint, config),
        remaining_values: constraint.fixed_values.clone(),
        required_values: Vec::new(),
        progress_callback,
        counters: MinimizerCounters::default(),
    })
}

struct Minimizer {
    runner: Box<dyn SolutionIter>,
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

pub trait SolutionTrait {
    fn to_fixed_values(&self) -> FixedValues;
    fn permute<R: rand::Rng>(&mut self, num_values: u16, rng: &mut R);
}
impl SolutionTrait for Solution {
    fn to_fixed_values(&self) -> FixedValues {
        self.iter().copied().enumerate().collect::<FixedValues>()
    }

    fn permute<R: rand::Rng>(&mut self, num_values: u16, rng: &mut R) {
        let mut permutation = (0..num_values).collect::<Vec<_>>();
        permutation.shuffle(rng);
        for v in self {
            *v = CellValue::from_index(permutation[v.index() as usize]);
        }
    }
}
