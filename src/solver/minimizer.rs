use std::cell::RefCell;
use std::rc::Rc;

use crate::types::{Constraint, FixedValues};

use super::engine;
use super::{Config, Counters, MinimizerCounters, MinimizerProgressCallback, OutputType};

pub fn make(
    constraint: &Constraint,
    mut config: Config,
    progress_callback: Option<Box<MinimizerProgressCallback>>,
) -> Box<dyn Iterator<Item = FixedValues>> {
    let minimizer_counters = Rc::new(RefCell::new(MinimizerCounters::default()));
    let progress_callback = Rc::new(RefCell::new(progress_callback));

    config.output_type = OutputType::Empty;
    {
        let minimizer_counters = minimizer_counters.clone();
        let progress_callback = progress_callback.clone();
        config.progress_callback = Some(Box::new(move |counters: &Counters| {
            let mut minimizer_counters = minimizer_counters.borrow_mut();
            minimizer_counters.solver_counters = *counters;
            super::maybe_call_callback(
                &mut progress_callback.borrow_mut().as_mut(),
                &minimizer_counters,
            );
        }));
    }

    Box::new(Minimizer {
        runner: engine::make_runner(constraint, config),
        remaining_values: constraint.fixed_values.clone(),
        required_values: Vec::new(),
        progress_callback,
        counters: minimizer_counters,
    })
}

struct Minimizer {
    runner: Box<dyn engine::Runner>,
    remaining_values: FixedValues,
    required_values: FixedValues,
    progress_callback: Rc<RefCell<Option<Box<MinimizerProgressCallback>>>>,
    counters: Rc<RefCell<MinimizerCounters>>,
}

impl Iterator for Minimizer {
    type Item = FixedValues;

    fn next(&mut self) -> Option<Self::Item> {
        let fixed_values = loop {
            super::maybe_call_callback(
                &mut self.progress_callback.borrow_mut().as_mut(),
                &self.counters.borrow(),
            );

            let item = self.remaining_values.pop()?;
            let fixed_values =
                [self.remaining_values.clone(), self.required_values.clone()].concat();

            self.runner.reset_fixed_values(&fixed_values);

            self.counters.borrow_mut().cells_tried += 1;

            if self.runner.next().is_none() {
                // No solutions, this is usually because it aborted early due to
                // the no_guesses requirement - so keep the value.
                // If this puzzle was already inconsistent, then we don't care.
                self.required_values.push(item);
            } else if self.runner.next().is_none() {
                // One solution, return it!
                self.counters.borrow_mut().cells_removed += 1;
                break fixed_values;
            } else {
                // Multiple solutions - this was required.
                self.required_values.push(item);
            }
        };

        Some(fixed_values)
    }
}
