mod parser;
mod solver;
mod types;
mod value_set;

use std::env;
use std::fs;

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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Must specify an input filename.");
    }
    let filename = &args[1];
    let input = fs::read_to_string(filename).expect("Something went wrong reading the input.");

    let constraint = parser::parse_text(&input).expect("Could not parse input file.");

    let solutions = solver::solve(&constraint);

    for solution in solutions {
        println!("{}", grid_to_string(&constraint, &solution));
    }
}
