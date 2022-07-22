use std::fs;
use std::process::ExitCode;

use clap::Parser;

use large_sudoku_solver::io::{output, parser};
use large_sudoku_solver::solver;
use large_sudoku_solver::types::Constraint;

fn run_solver(constraint: &Constraint) {
    const SCALE: u64 = 10000;
    output::with_progress_bar(SCALE, |bar| {
        let progress_callback = Box::new(move |counters: &solver::Counters| {
            bar.set_position((counters.progress_ratio * (SCALE as f64)) as u64);
            bar.set_message(output::counters(counters));
        });

        for solution in solver::solution_iter(constraint, Some(progress_callback)).take(2) {
            output::print_above_progress_bar(&output::solution_as_grid(constraint, &solution));
            // Separate solutions by a new line.
            println!();
        }
    });
}

fn run_minimizer(constraint: &Constraint) {
    output::with_progress_bar(constraint.fixed_values.len() as u64, |bar| {
        let progress_callback = Box::new(move |counters: &solver::MinimizerCounters| {
            bar.set_position(counters.cells_tried);
            bar.set_message(format!("{:?}", counters));
        });

        for fixed_values in solver::minimize(constraint, Some(progress_callback)) {
            output::print_above_progress_bar(&output::fixed_values_as_grid(
                constraint,
                &fixed_values,
            ));
            // Separate solutions by a new line.
            println!();
        }
    });
}

fn main_with_result(args: Args) -> Result<(), String> {
    let input = fs::read_to_string(&args.filename)
        .map_err(|e| format!("Could not read file {}: {}", args.filename, e))?;
    let constraint = parser::parse_text(&input)?;

    if args.minimize {
        run_minimizer(&constraint);
    } else {
        run_solver(&constraint);
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[clap(about, long_about = None)]
struct Args {
    #[clap(value_parser)]
    filename: String,

    #[clap(long)]
    minimize: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match main_with_result(args) {
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
        Ok(_) => ExitCode::SUCCESS,
    }
}
