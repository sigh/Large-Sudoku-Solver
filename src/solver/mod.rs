pub mod all_different;
mod cell_accumulator;
mod handlers;
mod runner;

use crate::types::{CellValue, Constraint, FixedValues};
use crate::value_set::IntBitSet;
#[cfg(not(feature = "i64_value_set"))]
use crate::value_set::RecValueSet;

use runner::Runner;

pub const VALID_NUM_VALUE_RANGE: std::ops::RangeInclusive<u32> = 2..=512;

pub type Solution = Vec<CellValue>;
pub type ProgressCallback = dyn FnMut(&Counters);
pub type MinimizerProgressCallback = dyn FnMut(&MinimizerCounters);

pub trait ToFixedValues {
    fn to_fixed_values(&self) -> FixedValues;
}
impl ToFixedValues for Solution {
    fn to_fixed_values(&self) -> FixedValues {
        self.iter().copied().enumerate().collect::<FixedValues>()
    }
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

pub trait SolutionIter: Iterator<Item = Solution> {
    fn reset_fixed_values(&mut self, fixed_values: &FixedValues);
}

pub fn solution_iter(
    constraint: &Constraint,
    progress_callback: Option<Box<ProgressCallback>>,
) -> Box<dyn SolutionIter> {
    const LOG_UPDATE_FREQUENCY: u64 = 10;
    let frequency_mask = match &progress_callback {
        Some(_) => (1 << LOG_UPDATE_FREQUENCY) - 1,
        None => u64::MAX,
    };

    let progress_config = runner::ProgressConfig {
        frequency_mask,
        callback: progress_callback,
    };

    match constraint.shape.num_values {
        #[cfg(not(feature = "i64_value_set"))]
        2..=32 => Box::new(Runner::<IntBitSet<i32>>::new(constraint, progress_config)),
        #[cfg(not(feature = "i64_value_set"))]
        33..=64 => Box::new(Runner::<IntBitSet<i64>>::new(constraint, progress_config)),
        #[cfg(feature = "i64_value_set")]
        2..=64 => Box::new(Runner::<IntBitSet<i64>>::new(constraint, progress_config)),
        #[cfg(not(feature = "i64_value_set"))]
        65..=128 => Box::new(Runner::<IntBitSet<i128>>::new(constraint, progress_config)),
        #[cfg(not(feature = "i64_value_set"))]
        129..=256 => Box::new(Runner::<RecValueSet<IntBitSet<i128>>>::new(
            constraint,
            progress_config,
        )),
        #[cfg(not(feature = "i64_value_set"))]
        257..=512 => Box::new(Runner::<RecValueSet<RecValueSet<IntBitSet<i128>>>>::new(
            constraint,
            progress_config,
        )),
        _ => panic!(
            "Grid too large. num_values: {}",
            constraint.shape.num_values
        ),
    }
}

pub fn minimize(
    constraint: &Constraint,
    progress_callback: Option<Box<MinimizerProgressCallback>>,
) -> Box<dyn Iterator<Item = FixedValues>> {
    Box::new(Minimizer {
        runner: solution_iter(constraint, None),
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
                // No solutions - keep item removed.
                self.counters.cells_removed += 1;
                continue;
            } else if self.runner.next().is_none() {
                // One solution, return it!
                self.counters.cells_removed += 1;
                break fixed_values;
            } else {
                // Multiple solutions - this was required.
                self.required_values.push(item);
            }
        };

        maybe_call_callback(&mut self.progress_callback, &self.counters);
        Some(fixed_values)
    }
}

fn maybe_call_callback<A, F: FnMut(A)>(f: &mut Option<F>, arg: A) {
    if let Some(f) = f {
        (f)(arg);
    }
}
