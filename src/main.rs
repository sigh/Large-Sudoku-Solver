use std::process::ExitCode;

use clap::Parser as _;
use rand::prelude::SliceRandom;
use rand::SeedableRng;

use large_sudoku_solver::io::{input, output, parser};
use large_sudoku_solver::solver;
use large_sudoku_solver::types::Constraint;
use large_sudoku_solver::types::RngType;

fn run_solver(
    constraint: &Constraint,
    mut writer: output::ProgressWriter,
    mut config: solver::Config,
    num_solutions: usize,
) -> Result<usize, String> {
    let mut solutions_found = 0;

    const SCALE: u64 = 10000;
    output::with_progress_bar(SCALE, |bar| {
        config.progress_callback = Some(Box::new(move |counters: &solver::Counters| {
            bar.set_position((counters.progress_ratio * (SCALE as f64)) as u64);
            bar.set_message(format!(
                "{{ solutions: {} guesses: {} values_tried: {} constraints_processed: {} progress_ratio: {} }}",
                counters.solutions,
                counters.guesses,
                counters.values_tried,
                counters.constraints_processed,
                counters.progress_ratio
            ));
        }));

        for solution in solver::solution_iter(constraint, config).take(num_solutions) {
            writer.write(&output::solver_item_as_grid(constraint, &solution));

            solutions_found += 1;
        }

        drop(writer);
    });

    Ok(solutions_found)
}

fn run_minimizer(
    mut constraint: Constraint,
    mut writer: output::ProgressWriter,
    no_guesses: bool,
    mut rng: RngType,
) -> Result<(), String> {
    constraint.fixed_values.shuffle(&mut rng);

    let num_fixed_values = constraint.fixed_values.len();
    output::with_progress_bar(num_fixed_values as u64, |bar| {
        let progress_callback = Box::new(move |counters: &solver::MinimizerCounters| {
            bar.set_position(counters.cells_tried);
            bar.set_message(format!(
                "{{ progress: {}/{} cells cells_removed: {} total_guesses: {} }} {{ solver_progress: {} }}",
                counters.cells_tried,
                num_fixed_values,
                counters.cells_removed,
                counters.solver_counters.guesses,
                counters.solver_counters.progress_ratio
            ));
        });

        let config = solver::Config {
            no_guesses,
            ..solver::Config::default()
        };

        for fixed_values in solver::minimize(&constraint, config, Some(progress_callback)) {
            writer.write(&output::fixed_values_as_grid(&constraint, &fixed_values));
        }

        drop(writer);
    });

    Ok(())
}

fn run_generator(
    constraint: Constraint,
    writer: output::ProgressWriter,
    _rng: RngType,
) -> Result<(), String> {
    let config = solver::Config {
        output_type: solver::OutputType::Guesses,
        ..solver::Config::default()
    };
    let num_results = run_solver(&constraint, writer, config, 1)?;
    if num_results == 0 {
        return Err("Input has no solution - puzzle could not be generated.".to_string());
    }

    Ok(())
}

fn run_count(constraint: Constraint) -> Result<(), String> {
    let config = solver::Config {
        output_type: solver::OutputType::Empty,
        ..solver::Config::default()
    };

    run_solver(
        &constraint,
        Box::new(output::EmptyWriter {}),
        config,
        usize::MAX,
    )
    .map(|_| ())
}

fn get_rng(args: &CliArgs) -> RngType {
    match args.seed {
        Some(seed) => RngType::seed_from_u64(seed),
        None => RngType::from_entropy(),
    }
}

fn main_with_result(args: CliArgs) -> Result<(), String> {
    let input = input::load(&args.input)
        .map_err(|e| format!("Could not read file {}: {}", args.input, e))?;

    let mut constraint = parser::parse_text(&input)?;
    if args.x_sudoku {
        constraint.x_sudoku = true;
    }

    let rng = get_rng(&args);

    let writer = output::get_writer(args.output_last);

    match args.action {
        CliAction::Solve => {
            run_solver(&constraint, writer, solver::Config::default(), 2).map(|_| ())
        }
        CliAction::Minimize => run_minimizer(constraint, writer, args.no_guesses, rng),
        CliAction::Generate => run_generator(constraint, writer, rng),
        CliAction::Count => run_count(constraint),
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
  generate: Generate a new puzzle using the input as a template (not efficient)
  count:    Count the number of solutions without printing them"
    )]
    action: CliAction,

    #[clap(
        value_parser,
        help = "One of:
  Filename to read puzzle from
  '-' to read from stdin
  'NxN' size specification for empty grid"
    )]
    input: String,

    #[clap(
        short,
        long,
        help = "Add x-sudoku constraints
(This can also be specified by adding 'X-Sudoku' inside the puzzle file)"
    )]
    x_sudoku: bool,

    #[clap(
        long,
        help = "Only output the last solution/puzzle
(Works even if program is aborted with ctrl-c)"
    )]
    output_last: bool,

    #[clap(long, help = "Don't allow guessing when generating/minimizing")]
    no_guesses: bool,

    #[clap(long, help = "RNG seed for generator/minimizer")]
    seed: Option<u64>,
}

#[derive(clap::ValueEnum, Debug, Clone)]
enum CliAction {
    Solve,
    Minimize,
    Generate,
    Count,
}

fn main() -> ExitCode {
    let args = CliArgs::parse();
    output::set_ctrlc_handler();
    match main_with_result(args) {
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
        Ok(_) => ExitCode::SUCCESS,
    }
}
