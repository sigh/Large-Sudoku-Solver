use crate::types::Shape;
use crate::types::FixedValues;

pub fn parse_text(input: &str) -> (Shape, FixedValues) {
    const RADIX: u32 = 10;

    let mut fixed_values = FixedValues::new();

    for (i, c) in input.chars().enumerate() {
        if c.is_digit(RADIX) {
            fixed_values.push((i, c.to_digit(RADIX).unwrap()));
        }
    }

    (Shape::new(3), fixed_values)
}