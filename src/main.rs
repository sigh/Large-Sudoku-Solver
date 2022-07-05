use std::fmt;

#[derive(Copy, Clone, Debug)]
struct ValueSet(u128);

#[derive(Copy, Clone, Debug)]
struct Shape {
    dim: u32,
    box_size: u32,
    num_values: u32,
    num_cells: usize,
    side_len: u32,
}

type CellIndex = usize;

impl Shape {
    pub fn new(dim: u32) -> Shape {
        let num_values = dim*dim;
        Shape{
            dim: dim,
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
    pub fn new(shape: Shape) -> Grid {
        Grid{
            num_values: shape.num_values,
            cells: vec![ValueSet(0); shape.num_cells],
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
            } else if c.count() == 0 {
                write!(f, ". ")?;
            } else {
                write!(f, "x ")?;
            }
        }
        Ok(())
    }
}

struct House {
    cells: Vec<CellIndex>,
}

fn make_houses(shape: Shape) -> Vec<House> {
    let mut houses = Vec::new();
    let side_len = shape.side_len;
    let box_size = shape.box_size;

    // Make rows.
    for r in 0..side_len {
        let mut house = House{cells: Vec::new()};
        for c in 0..side_len {
            house.cells.push(shape.make_cell_index(r, c));
        }
        houses.push(house);
    }

    // Make columns.
    for c in 0..side_len {
        let mut house = House{cells: Vec::new()};
        for r in 0..side_len {
            house.cells.push(shape.make_cell_index(r, c));
        }
        houses.push(house);
    }

    // Make boxes.
    for b in 0..side_len {
        let mut house = House{cells: Vec::new()};
        for i in 0..side_len {
            let r = (b/box_size)*box_size+(b%box_size);
            let c = (r%box_size)*box_size+(i/box_size);
            house.cells.push(shape.make_cell_index(r, c));
        }
        houses.push(house);
    }

    return houses;
}

type CellConflicts = Vec<CellIndex>;

fn make_cell_conflicts(houses: Vec<House>, shape: Shape) -> Vec<CellConflicts> {
  let mut result = Vec::new();
  for i in 0..shape.num_cells {
    result.push(Vec::new())
  }
  for house in houses {
    for c1 in &house.cells {
        for c2 in &house.cells {
            if c1 != c2 {
                result[*c1].push(*c2);
            }
        }
    }
  }
  return result;
}

fn solve(grid: &Grid, shape: &Shape, cell_conflicts: &Vec<CellConflicts>) {
    let mut stack = Vec::<CellIndex>::with_capacity(shape.num_cells);
    for i in 0..shape.num_cells { stack.push(i) }
}

fn main() {
    let input = ".76.9..8...2..3..9.3.6.....1..5......69.2.43......6..8.....1.5.6..2..8...2..5.17.";
    let shape = Shape::new(3);
    let mut grid = Grid::new(shape);
    grid.load_from_str(&input);

    let houses = make_houses(shape);
    let cell_conflicts = make_cell_conflicts(houses, shape);
    solve(&grid, &shape, &cell_conflicts);
    println!("{}", grid);
    println!("{} {}", grid.cells[0], grid.cells[0].value());
}
