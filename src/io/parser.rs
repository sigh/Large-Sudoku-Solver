use lazy_static::lazy_static;
use regex::Regex;

use crate::solver;
use crate::types::{CellValue, Constraint, FixedValues, Shape, ValueType};

pub type ParserResult = Result<Constraint, String>;

pub fn parse_text(input: &str) -> ParserResult {
    let mut input = String::from(input);

    remove_comments(&mut input);
    let sudoku_x = extract_sodoku_x(&mut input);

    let parse_fns: Vec<fn(&str) -> ParserResult> = vec![parse_short_text, parse_grid_layout];

    let mut constraint = None;
    let mut errors = Vec::new();
    for parse_fn in parse_fns {
        match (parse_fn)(&input) {
            Ok(parsed) => {
                constraint = Some(parsed);
                break;
            }
            Err(msg) => {
                errors.push(msg);
            }
        }
    }

    match constraint {
        None => Err(errors.join("\n")),
        Some(mut constraint) => {
            constraint.sudoku_x = sudoku_x;
            Ok(constraint)
        }
    }
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

fn guess_dimension(num_cells: usize) -> Result<u32, String> {
    let dim = (num_cells as f64).sqrt().sqrt() as u32;
    let num_values = dim * dim;
    if num_values * num_values != (num_cells as u32) {
        return Err(format!(
            "Cell count does not make a valid grid size: {num_cells}."
        ));
    }

    if !solver::VALID_NUM_VALUE_RANGE.contains(&num_values) {
        return Err(format!(
            "Grid size not supported. Side length: {num_values}."
        ));
    }

    Ok(dim)
}

fn parse_short_text(input: &str) -> ParserResult {
    let mut input = String::from(input);
    remove_whitespace(&mut input);

    let dim = guess_dimension(input.len())?;
    let num_values = dim * dim;
    let radix = num_values + 1;
    if radix > 36 {
        return Err(format!("Too many values for short input: {num_values}."));
    }

    let mut fixed_values = FixedValues::new();

    for (i, c) in input.chars().enumerate() {
        match c {
            '.' | '0' => {}
            c if c.is_digit(radix) => {
                fixed_values.push((
                    i,
                    CellValue::from_display_value(c.to_digit(radix).unwrap().try_into().unwrap()),
                ));
            }
            c => {
                return Err(format!("Unrecognized character: {}", c));
            }
        }
    }

    Ok(Constraint {
        shape: Shape::new(dim),
        fixed_values,
        sudoku_x: false,
    })
}

fn parse_grid_layout(input: &str) -> ParserResult {
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
                let value = part.parse::<ValueType>().expect("Unparsable number.");
                if value == 0 || value > num_values as ValueType {
                    return Err(format!("Value out of range: {value}."));
                }
                fixed_values.push((i, CellValue::from_display_value(value)));
            }
        }
    }

    Ok(Constraint {
        shape: Shape::new(dim),
        fixed_values,
        sudoku_x: false,
    })
}
