use std::io::Read;
use std::{fs, io};

use super::parser;

pub fn load(input: &str) -> Result<String, io::Error> {
    if input == "-" {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        return Ok(content);
    }

    if parser::parse_shape_spec(input).is_some() {
        return Ok(input.to_string());
    }

    fs::read_to_string(input)
}
