use lazy_static::lazy_static;
use regex::Regex;

use crate::types::Constraint;
use crate::types::FixedValues;
use crate::types::Shape;

pub fn parse_text(input: &str) -> Option<Constraint> {
    let mut input = String::from(input);

    remove_comments(&mut input);
    let sudoku_x = extract_sodoku_x(&mut input);

    let mut parsed = None;
    parsed = parsed.or_else(|| parse_short_text(&input));
    parsed = parsed.or_else(|| parse_grid_layout(&input));

    let mut constraint = parsed?;
    constraint.sudoku_x = sudoku_x;

    Some(constraint)
}

fn remove_comments(input: &mut String) {
    lazy_static! {
        static ref COMMENT_REGEX: Regex = Regex::new("(?m)#.*$").unwrap();
    }

    *input = COMMENT_REGEX.replace(input, "").to_string();
}

fn extract_sodoku_x(input: &mut String) -> bool {
    lazy_static! {
        static ref SUDOKU_X_REGEX: Regex = Regex::new("(?i)sudoku[ -]x").unwrap();
    }

    if !SUDOKU_X_REGEX.is_match(input) {
        return false;
    }

    *input = SUDOKU_X_REGEX.replace(input, "").to_string();
    true
}

fn remove_whitespace(s: &mut String) {
    s.retain(|c| !c.is_whitespace());
}

fn guess_dimension(num_cells: usize) -> Option<u32> {
    let dim = (num_cells as f64).sqrt().sqrt() as usize;
    if !(2..=11).contains(&dim) {
        return None;
    }
    if dim * dim * dim * dim != num_cells {
        return None;
    }
    Some(dim as u32)
}

fn parse_short_text(input: &str) -> Option<Constraint> {
    let mut input = String::from(input);
    remove_whitespace(&mut input);

    let dim = guess_dimension(input.len())?;
    let num_values = dim * dim;
    let radix = num_values + 1;
    if radix > 36 {
        return None;
    }

    let mut fixed_values = FixedValues::new();

    for (i, c) in input.chars().enumerate() {
        match c {
            '.' | '0' => (),
            c if c.is_digit(radix) => {
                fixed_values.push((i, c.to_digit(radix).unwrap()));
            }
            _ => return None,
        }
    }

    Some(Constraint {
        shape: Shape::new(dim),
        fixed_values,
        sudoku_x: false,
    })
}

fn parse_grid_layout(input: &str) -> Option<Constraint> {
    lazy_static! {
        static ref CELL_REGEX: Regex = Regex::new("[.]|\\d+").unwrap();
    }

    let parts = CELL_REGEX
        .find_iter(input)
        .map(|mat| mat.as_str())
        .collect::<Vec<_>>();
    let dim = guess_dimension(parts.len())?;
    let num_values = dim * dim;

    let mut fixed_values = FixedValues::new();

    for (i, part) in parts.iter().enumerate() {
        match *part {
            "." => (),
            _ => {
                let value = part.parse::<u32>().ok()?;
                if value <= 0 || value > num_values {
                    return None;
                }
                fixed_values.push((i, value));
            }
        }
    }

    Some(Constraint {
        shape: Shape::new(dim),
        fixed_values,
        sudoku_x: false,
    })
}
