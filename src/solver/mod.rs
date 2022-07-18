pub mod all_different;
mod cell_accumulator;
mod handlers;
mod runner;

use crate::types::CellValue;
use crate::types::Constraint;
use crate::value_set::{IntBitSet, RecValueSet};

use runner::Runner;

pub const VALID_NUM_VALUE_RANGE: std::ops::RangeInclusive<u32> = 2..=256;

pub type Solution = Vec<CellValue>;
pub type ProgressCallback = dyn FnMut(&Counters);

#[derive(Copy, Clone, Debug, Default)]
pub struct Counters {
    pub values_tried: u64,
    pub cells_searched: u64,
    pub backtracks: u64,
    pub guesses: u64,
    pub solutions: u64,
    pub progress_ratio: f64,
}

pub fn solution_iter(
    constraint: &Constraint,
    progress_callback: Option<Box<ProgressCallback>>,
) -> Box<dyn Iterator<Item = Solution>> {
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
        2..=32 => Box::new(Runner::<IntBitSet<i32>>::new(constraint, progress_config)),
        33..=64 => Box::new(Runner::<IntBitSet<i64>>::new(constraint, progress_config)),
        65..=128 => Box::new(Runner::<IntBitSet<i128>>::new(constraint, progress_config)),
        129..=256 => Box::new(Runner::<RecValueSet<IntBitSet<i128>>>::new(
            constraint,
            progress_config,
        )),
        _ => panic!(
            "Grid too large. num_values: {}",
            constraint.shape.num_values
        ),
    }
}
