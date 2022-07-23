use std::process::ExitCode;

use clap::Parser as _;
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;

use large_sudoku_solver::io::{input, output, parser};
use large_sudoku_solver::solver;
use large_sudoku_solver::solver::SolutionTrait;
use large_sudoku_solver::types::Constraint;

fn run_solver(
    constraint: &Constraint,
    num_solutions: usize,
) -> Result<Vec<solver::Solution>, String> {
    let mut solutions = Vec::new();

    const SCALE: u64 = 10000;
    output::with_progress_bar(SCALE, |bar| {
        let progress_callback = Box::new(move |counters: &solver::Counters| {
            bar.set_position((counters.progress_ratio * (SCALE as f64)) as u64);
            bar.set_message(output::counters(counters));
        });

        for solution in
            solver::solution_iter(constraint, false, Some(progress_callback)).take(num_solutions)
        {
            output::print_above_progress_bar(&output::solution_as_grid(constraint, &solution));
            // Separate solutions by a new line.
            println!();

            solutions.push(solution);
        }
    });

    Ok(solutions)
}

fn run_minimizer(
    mut constraint: Constraint,
    no_guesses: bool,
    rng: &mut StdRng,
) -> Result<(), String> {
    constraint.fixed_values.shuffle(rng);

    output::with_progress_bar(constraint.fixed_values.len() as u64, |bar| {
        let progress_callback = Box::new(move |counters: &solver::MinimizerCounters| {
            bar.set_position(counters.cells_tried);
            bar.set_message(format!("{:?}", counters));
        });

        for fixed_values in solver::minimize(&constraint, no_guesses, Some(progress_callback)) {
            output::print_above_progress_bar(&output::fixed_values_as_grid(
                &constraint,
                &fixed_values,
            ));
            // Separate solutions by a new line.
            println!();
        }
    });

    Ok(())
}

fn run_generator(
    mut constraint: Constraint,
    no_guesses: bool,
    rng: &mut StdRng,
) -> Result<(), String> {
    let mut solutions = run_solver(&constraint, 1)?;
    if solutions.is_empty() {
        return Err("Input has no solution - puzzle could not be generated.".to_string());
    }

    let candidate = &mut solutions[0];
    candidate.permute(constraint.shape.num_values as u16, rng);
    constraint.fixed_values = candidate.to_fixed_values();

    run_minimizer(constraint, no_guesses, rng)
}

fn get_rng(args: &CliArgs) -> StdRng {
    match args.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_entropy(),
    }
}

fn main_with_result(args: CliArgs) -> Result<(), String> {
    let input = input::read(&args.filename)
        .map_err(|e| format!("Could not read file {}: {}", args.filename, e))?;
    let constraint = parser::parse_text(&input)?;
    let mut rng = get_rng(&args);

    match args.action {
        CliAction::Solve => run_solver(&constraint, 2).map(|_| ()),
        CliAction::Minimize => run_minimizer(constraint, args.no_guesses, &mut rng),
        CliAction::Generate => run_generator(constraint, args.no_guesses, &mut rng),
    }
}

#[derive(clap::Parser, Debug)]
#[clap(
    arg_required_else_help = true,
    about = "Solves and generates sudoku puzzles with large grids (up to 512x512)"
)]
struct CliArgs {
    #[clap(
        value_enum,
        hide_possible_values = true,
        help = "Supported actions:

solve:    Solve the input and prove uniqueness
minimize: Attempt to remove as many set values from the puzzle as possible
          while keeping the solution unique
generate: Generate a new puzzle using the input as a template"
    )]
    action: CliAction,

    #[clap(
        value_parser,
        help = "Filename to read puzzle from, or from stdio if '-'"
    )]
    filename: String,

    #[clap(long, help = "RNG seed for generator/minimizer")]
    seed: Option<u64>,

    #[clap(long, help = "Don't allow guessing when generating/minimizing")]
    no_guesses: bool,
}

#[derive(clap::ValueEnum, Debug, Clone)]
enum CliAction {
    Solve,
    Minimize,
    Generate,
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
