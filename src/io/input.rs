use std::io::Read;
use std::{fs, io};

pub fn read(filename: &str) -> Result<String, io::Error> {
    if filename == "-" {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        Ok(input)
    } else {
        fs::read_to_string(filename)
    }
}
