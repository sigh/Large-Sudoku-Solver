use std::env;
use std::fs;
use std::rc::Rc;

use large_sudoku_solver::io::parser;
use large_sudoku_solver::solver;
use large_sudoku_solver::types;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use large_sudoku_solver::types::Constraint;

fn grid_to_string(constraint: &types::Constraint, solution: &solver::Solution) -> String {
    let mut output = String::new();

    let shape = &constraint.shape;
    assert_eq!(shape.num_cells, solution.len());

    let pad_size = shape.num_values.to_string().len() + 1;

    for r in 0..shape.side_len {
        for c in 0..shape.side_len {
            let index = shape.make_cell_index(r, c);
            let value = solution[index].to_string();
            (0..pad_size - value.len()).for_each(|_| output.push(' '));
            output.push_str(&value);
        }
        output.push('\n');
    }

    output
}

fn run_solver(constraint: &Constraint) -> Vec<solver::Solution> {
    const SCALE: u64 = 10000;
    let bar = Rc::new(ProgressBar::new(SCALE));
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {wide_bar:cyan/blue} {percent}%\n{wide_msg}"),
    );

    let closure_bar = bar.clone();
    let progress_callback = Box::new(move |counters: &solver::Counters| {
        closure_bar.set_position((counters.progress_ratio * (SCALE as f64)) as u64);
        closure_bar.set_message(format!("{:?}", counters));
    });

    let mut solutions = Vec::new();
    for solution in solver::solution_iter(constraint, Some(progress_callback)).take(2) {
        bar.println(format!(
            "[{}]",
            solution
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ));
        solutions.push(solution);
    }

    if solutions.len() > 1 {
        bar.println("Too many solutions.");
    }

    bar.set_style(ProgressStyle::default_bar().template("[{elapsed_precise}] {msg}"));
    bar.finish();

    solutions
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Must specify an input filename.");
    }
    let filename = &args[1];
    let input = fs::read_to_string(filename).expect("Something went wrong reading the input.");
    let constraint = parser::parse_text(&input).expect("Could not parse input file.");

    for solution in run_solver(&constraint) {
        println!("{}", grid_to_string(&constraint, &solution));
    }
}
