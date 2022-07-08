mod types;
mod parser;
mod solver;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 { panic!("Must specify an input filename."); }
    let filename = &args[1];
    let input = fs::read_to_string(filename)
        .expect("Something went wrong reading the input.");

    let (shape, fixed_values) = parser::parse_text(&input)
        .expect("Could not parse input file.");

    solver::solve(&shape, &fixed_values);
}
