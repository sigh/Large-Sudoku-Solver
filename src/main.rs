use std::fmt;

type CellIndex = usize;

#[derive(Copy, Clone)]
struct Shape {
    box_size: u32,
    num_values: u32,
    num_cells: usize,
    side_len: u32,
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
struct ValueSet(i128);

impl ValueSet {
    pub fn from_value(value: u32) -> ValueSet {
        ValueSet(1<<(value-1))
    }

    pub fn full(num_values: u32) -> ValueSet {
        ValueSet((1<<num_values)-1)
    }

    pub fn value(&self) -> u32 {
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
struct Grid {
    cells: Vec<ValueSet>,
}

impl Grid {
    pub fn new(shape: &Shape) -> Grid {
        Grid{
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

#[derive(Debug)]
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
            let r = (b%box_size)*box_size+(i/box_size);
            let c = (b/box_size)*box_size+(i%box_size);
            house.cells.push(shape.make_cell_index(r, c));
        }
        houses.push(house);
    }

    return houses;
}

type CellConflicts = Vec<CellIndex>;

fn make_cell_conflicts(houses: &Vec<House>, shape: &Shape) -> Vec<CellConflicts> {
  let mut result: Vec<CellConflicts> =
        (0..shape.num_cells).map(|_| Vec::new()).collect();
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

fn enforce_value(grid: &mut Grid, value: ValueSet, cell: CellIndex, cell_conflicts: &Vec<CellConflicts>) -> bool {
    for conflict_cell in &cell_conflicts[cell] {
        let values = &mut grid.cells[*conflict_cell];
        values.remove(value);
        if values.empty() { return false; }
    }
    true
}

fn update_cell_order(_stack: &mut Vec<CellIndex>, _depth: usize, _grid: &Grid) {
}

fn solve(grid: &Grid, shape: &Shape, cell_conflicts: &Vec<CellConflicts>) {
    let mut stack: Vec<CellIndex> = (0..shape.num_cells).collect();
    let mut grids: Vec<Grid> =
        (0..shape.num_cells+1).map(|_| Grid::new(shape)).collect();

    let mut depth = 0;
    for (cell_index, value) in grid.cells.iter().enumerate() {
        if value.empty() {
            grids[depth].cells[cell_index] = ValueSet::full(shape.num_values);
        } else {
            grids[depth].cells[cell_index] = *value;
        }
    }

    let mut num_solutions = 0;

    depth += 1;
    while depth > 0 {
        let (grids_front, grids_back) = grids.split_at_mut(depth);
        depth -= 1;

        let mut grid = &mut grids_front[depth];
        let cell = stack[depth];
        let values = &mut grid.cells[cell];

        // No more values to try.
        if values.empty() { continue; }

        // Find the next value to try.
        let value = values.min();
        values.remove(value);

        // Copy the current cell values.
        depth += 1;
        grids_back[0].copy_from(grid);

        // Update the grid with the trial value.
        grid = &mut grids_back[0];
        grid.cells[cell] = value;

        // Propograte constraints.
        let has_contradiction = !enforce_value(&mut grid, value, cell, cell_conflicts);
        if has_contradiction { continue; }

        // Check if we have a solution.
        if depth == shape.num_cells {
            num_solutions += 1;
            println!("Solved!");
            println!("{}", grid);
            if num_solutions > 2 { panic!() }
            continue;
        }

        // Find the next cell to try.
        update_cell_order(&mut stack, depth, grid);
        depth += 1;
    }
}

fn main() {
    let input = ".76.9..8...2..3..9.3.6.....1..5......69.2.43......6..8.....1.5.6..2..8...2..5.17.";
    let shape = Shape::new(3);
    let mut grid = Grid::new(&shape);
    grid.load_from_str(&input);

    let houses = make_houses(shape);
    let cell_conflicts = make_cell_conflicts(&houses, &shape);
    println!("{}", grid);
    solve(&grid, &shape, &cell_conflicts);
    println!("{}", grid);
}
