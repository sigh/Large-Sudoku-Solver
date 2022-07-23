use std::rc::Rc;

use crate::solver;
use crate::types;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;

pub fn solver_item_as_grid(constraint: &types::Constraint, item: &solver::Output) -> String {
    match item {
        solver::Output::Solution(solution) => solution_as_grid(constraint, solution),
        solver::Output::Guesses(fixed_values) => fixed_values_as_grid(constraint, fixed_values),
        solver::Output::Empty => String::new(),
    }
}

fn solution_as_grid(constraint: &types::Constraint, solution: &types::Solution) -> String {
    render_grid(
        constraint,
        &solution.iter().map(|&v| Some(v)).collect::<Vec<_>>(),
    )
}

pub fn fixed_values_as_grid(
    constraint: &types::Constraint,
    fixed_values: &types::FixedValues,
) -> String {
    let shape = &constraint.shape;
    let mut grid = vec![None; shape.num_cells];
    for (cell, value) in fixed_values {
        grid[*cell] = Some(*value);
    }
    render_grid(constraint, &grid)
}

fn render_grid(constraint: &types::Constraint, grid: &[Option<types::CellValue>]) -> String {
    let mut output = String::new();

    let shape = &constraint.shape;
    assert_eq!(shape.num_cells, grid.len());

    let pad_size = shape.num_values.to_string().len() + 1;

    for r in 0..shape.side_len {
        for c in 0..shape.side_len {
            let index = shape.make_cell_index(r, c);
            let display = match grid[index] {
                None => ".".to_string(),
                Some(v) => v.to_string(),
            };
            (0..pad_size - display.len()).for_each(|_| output.push(' '));
            output.push_str(&display);
        }
        output.push('\n');
    }

    output
}

pub fn solution_compact(solution: &types::Solution) -> String {
    format!(
        "[{}]",
        solution
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

pub fn counters(counters: &solver::Counters) -> String {
    format!(
        "{{ solutions: {} guesses: {} values_tried: {} constraints_processed: {} progress_ratio: {} }}",
        counters.solutions, counters.guesses, counters.values_tried, counters.constraints_processed, counters.progress_ratio
    )
}

pub fn with_progress_bar<F: FnOnce(Rc<ProgressBar>)>(scale: u64, f: F) {
    let bar = Rc::new(ProgressBar::new(scale));
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {wide_bar:cyan/blue} {percent}%\n{wide_msg}"),
    );
    bar.enable_steady_tick(1000);
    bar.set_position(0);
    bar.set_message("Initializing...");

    f(bar.clone());

    bar.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {msg}"));
    bar.finish();
}

pub fn print_above_progress_bar(output: &str) {
    if output.is_empty() {
        return;
    }

    if atty::is(atty::Stream::Stdout) {
        // We only need to worry about the bar if stdout is going to a tty.

        // Erase the bar (two lines).
        eprint!("\x1b[A\x1b[2K"); // Erase line above.
        eprint!("\x1b[A\x1b[2K"); // Erase line above.
        eprint!("\r"); // Bring cursor to start.

        // Write the output.
        println!("{}", output);

        // Write another newline so that the output is not cleared by the bar.
        eprintln!();
        eprintln!();
    } else {
        println!("{}", output);
    }

    // Print another line between solutions.
    println!();
}
