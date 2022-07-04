use std::fmt;

#[derive(Copy, Clone, Debug)]
struct ValueSet(u128);

impl ValueSet {
    pub fn from_value(value: u32) -> ValueSet {
        ValueSet(1<<(value-1))
    }

    pub fn value(&self) -> u32 {
        self.0.trailing_zeros()+1
    }

    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

impl fmt::Display for ValueSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
struct Grid {
    cells: Vec<ValueSet>,
    num_values: u32,
}

impl Grid {
    pub fn make(num_values: u32) -> Grid {
        let num_cells = (num_values*num_values).try_into().unwrap();
        Grid{
            num_values: num_values,
            cells: vec![ValueSet(0); num_cells],
        }
    }

    pub fn load_from_str(&mut self, input: &str) {
        const RADIX: u32 = 10;

        for (i, c) in input.chars().enumerate() {
            if c.is_digit(RADIX) {
                self.cells[i] = ValueSet::from_value(c.to_digit(RADIX).unwrap())
            }
        }
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in &self.cells {
            if c.count() == 1 {
                write!(f, "{} ", c.value())?;
            } else {
                write!(f, ". ")?;
            }
        }
        Ok(())
    }
}

fn main() {
    let input = ".76.9..8...2..3..9.3.6.....1..5......69.2.43......6..8.....1.5.6..2..8...2..5.17.";
    let mut grid = Grid::make(9);
    grid.load_from_str(&input);
    println!("{}", grid);
    println!("{} {}", grid.cells[0], grid.cells[0].value());
}
