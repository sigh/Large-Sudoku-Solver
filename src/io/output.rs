use crate::{solver, types};

pub fn solution_as_grid(constraint: &types::Constraint, solution: &solver::Solution) -> String {
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

pub fn solution_compact(solution: &solver::Solution) -> String {
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
