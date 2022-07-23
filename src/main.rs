use std::process::ExitCode;

use clap::Parser as _;

use large_sudoku_solver::io::{input, output, parser};
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

fn main_with_result(args: CliArgs) -> Result<(), String> {
    let input = input::read(&args.filename)
        .map_err(|e| format!("Could not read file {}: {}", args.filename, e))?;
    let constraint = parser::parse_text(&input)?;

    match args.action {
        CliAction::Solve => run_solver(&constraint),
        CliAction::Minimize => run_minimizer(&constraint),
    }

    Ok(())
}

#[derive(clap::Parser, Debug)]
#[clap(about = "Solves sudoku puzzles with large grids (up to 512x512)")]
struct CliArgs {
    #[clap(long)]
    minimize: bool,

    #[clap(
        value_enum,
        hide_possible_values = true,
        help = "Supported actions:

solve:    Solve the input and prove uniqueness
minimize: Attempt to remove as many set values from the puzzle as possible
          while keeping the solution unique"
    )]
    action: CliAction,

    #[clap(
        value_parser,
        help = "Filename to read puzzle from, or from stdio if '-'"
    )]
    filename: String,
}

#[derive(clap::ValueEnum, Debug, Clone)]
enum CliAction {
    Solve,
    Minimize,
}

fn main() -> ExitCode {
    let args = CliArgs::parse();
    match main_with_result(args) {
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
        Ok(_) => ExitCode::SUCCESS,
    }
}
