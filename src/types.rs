use std::fmt;

pub type CellIndex = usize;
pub type ValueType = u16;

#[derive(Debug, Copy, Clone)]
pub struct CellValue(ValueType);

impl CellValue {
    pub fn from_display_value(display_value: ValueType) -> CellValue {
        CellValue(display_value - 1)
    }

    pub fn display_value(&self) -> ValueType {
        self.0 + 1
    }

    pub fn from_index(index: ValueType) -> CellValue {
        CellValue(index)
    }

    pub fn index(&self) -> ValueType {
        self.0
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_value())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Shape {
    pub box_size: u32,
    pub num_values: u32,
    pub num_cells: usize,
    pub side_len: u32,
}

impl Shape {
    pub fn new(dim: u32) -> Shape {
        let num_values = dim * dim;
        Shape {
            box_size: dim,
            num_values,
            num_cells: (num_values * num_values).try_into().unwrap(),
            side_len: num_values,
        }
    }

    pub fn make_cell_index(&self, row: u32, col: u32) -> CellIndex {
        ((row * self.side_len) + col).try_into().unwrap()
    }
}

pub type FixedValues = Vec<(CellIndex, CellValue)>;

#[derive(Debug, Clone)]
pub struct Constraint {
    pub shape: Shape,
    pub fixed_values: FixedValues,
    pub sudoku_x: bool,
}
