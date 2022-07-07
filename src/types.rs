use std::fmt;

pub type CellIndex = usize;
pub type CellValue = u32;

#[derive(Debug, Copy, Clone)]
pub struct Shape {
    pub box_size: u32,
    pub num_values: u32,
    pub num_cells: usize,
    pub side_len: u32,
}

impl Shape {
    pub fn new(dim: u32) -> Shape {
        let num_values = dim*dim;
        Shape{
            box_size: dim,
            num_values: num_values,
            num_cells: (num_values*num_values).try_into().unwrap(),
            side_len: num_values,
        }
    }

    pub fn make_cell_index(&self, row: u32, col: u32) -> CellIndex {
        ((row*self.side_len)+col).try_into().unwrap()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ValueSet(i128);

impl ValueSet {
    pub fn from_value(value: CellValue) -> ValueSet {
        ValueSet(1<<(value-1))
    }

    pub fn full(num_values: u32) -> ValueSet {
        ValueSet((1<<num_values)-1)
    }

    pub fn value(&self) -> CellValue {
        self.0.trailing_zeros()+1
    }

    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }

    pub fn empty(&self) -> bool {
        self.0 == 0
    }

    pub fn min(&self) -> ValueSet {
        ValueSet(self.0 & -self.0)
    }

    pub fn remove(&mut self, other: ValueSet) {
        self.0 &= !other.0
    }
}

impl fmt::Display for ValueSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct Grid {
    pub cells: Vec<ValueSet>,
}

impl Grid {
    pub fn new(shape: &Shape) -> Grid {
        Grid{
            cells: vec![ValueSet(0); shape.num_cells],
        }
    }

    pub fn copy_from(&mut self, other: &Grid) {
        self.cells.clone_from_slice(&other.cells[..]);
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in &self.cells {
            if c.count() == 1 {
                write!(f, "{} ", c.value())?;
            } else if c.count() == 0 {
                write!(f, ". ")?;
            } else {
                write!(f, "x ")?;
            }
        }
        Ok(())
    }
}

pub type FixedValues = Vec<(CellIndex, CellValue)>;