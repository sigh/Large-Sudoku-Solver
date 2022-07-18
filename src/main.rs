use std::env;
use std::fs;
use std::rc::Rc;

use large_sudoku_solver::io::{output, parser};
use large_sudoku_solver::solver;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use large_sudoku_solver::types::Constraint;

fn run_solver(constraint: &Constraint) {
    const SCALE: u64 = 10000;
    let bar = Rc::new(ProgressBar::new(SCALE));
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {wide_bar:cyan/blue} {percent}%\n{wide_msg}"),
    );

    let closure_bar = bar.clone();
    let progress_callback = Box::new(move |counters: &solver::Counters| {
        closure_bar.set_position((counters.progress_ratio * (SCALE as f64)) as u64);
        closure_bar.set_message(output::counters(counters));
    });

    for solution in solver::solution_iter(constraint, Some(progress_callback)).take(2) {
        print_above_progress_bar(&output::solution_as_grid(constraint, &solution));
        // Separate solutions by a new line.
        println!();
    }

    bar.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {msg}"));
    bar.finish();
}

fn print_above_progress_bar(output: &str) {
    // Erase the bar (two lines).
    eprint!("\x1b[A\x1b[2K"); // Erase line above.
    eprint!("\x1b[A\x1b[2K"); // Erase line above.
    eprint!("\r"); // Bring cursor to start.

    // Write the output.
    println!("{}", output);

    // Write another newline so that the output is not cleared by the bar.
    eprintln!();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Must specify an input filename.");
    }

    let filename = &args[1];
    let input = fs::read_to_string(filename).expect("Something went wrong reading the input.");
    let constraint = parser::parse_text(&input).expect("Could not parse input file.");

    run_solver(&constraint);
}
