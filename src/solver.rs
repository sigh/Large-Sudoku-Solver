use crate::types::CellIndex;
use crate::types::Shape;
use crate::types::ValueSet;
use crate::types::Grid;
use crate::types::FixedValues;

pub fn solve(shape: &Shape, fixed_values: &FixedValues) -> Counters {
  let mut solver = Solver::new(shape);
  solver.run(fixed_values)
}

#[derive(Debug)]
struct House {
    cells: Vec<CellIndex>,
}

fn make_houses(shape: &Shape) -> Vec<House> {
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

#[derive(Copy, Clone, Debug, Default)]
pub struct Counters {
  values_tried: u64,
  cells_searched: u64,
  backtracks: u64,
  guesses: u64,
  solutions: u64,
}

struct Solver {
  shape: Shape,
  stack: Vec<CellIndex>,
  grids: Vec<Grid>,
  cell_conflicts: Vec<CellConflicts>,
  counters: Counters,
}

impl Solver {
  fn new(shape: &Shape) -> Solver {
    let houses = make_houses(shape);
    Solver{
      shape: *shape,
      stack: (0..shape.num_cells).collect(),
      grids: (0..shape.num_cells+1).map(|_| Grid::new(shape)).collect(),
      cell_conflicts: make_cell_conflicts(&houses, shape),
      counters: Counters::default(),
    }
  }

  fn run(&mut self, fixed_values: &FixedValues) -> Counters {
      for grid in &mut self.grids {
          grid.cells.fill(ValueSet::full(self.shape.num_values));
      }

      let mut depth = 0;
      for (cell, value) in fixed_values {
          self.grids[depth].cells[*cell] = ValueSet::from_value(*value);
      }

      depth += 1;
      while depth > 0 {
          let (grids_front, grids_back) = self.grids.split_at_mut(depth);
          depth -= 1;

          let mut grid = &mut grids_front[depth];
          let cell = self.stack[depth];
          let values = &mut grid.cells[cell];

          // No more values to try.
          if values.empty() { continue; }

          // Find the next value to try.
          let value = values.min();
          self.counters.values_tried += 1;
          self.counters.guesses += (*values != value) as u64;
          values.remove(value);

          // Copy the current cell values.
          depth += 1;
          grids_back[0].copy_from(grid);

          // Update the grid with the trial value.
          grid = &mut grids_back[0];
          grid.cells[cell] = value;

          // Propograte constraints.
          let has_contradiction = !enforce_value(&mut grid, value, cell, &self.cell_conflicts);
          if has_contradiction {
            self.counters.backtracks += 1;
            continue;
          }

          // Check if we have a solution.
          if depth == self.shape.num_cells {
              self.counters.solutions += 1;
              println!("Solved!");
              println!("{}", grid);
              if self.counters.solutions > 2 { panic!() }
              continue;
          }

          // Find the next cell to try.
          update_cell_order(&mut self.stack, depth, grid);
          self.counters.cells_searched += 1;
          depth += 1;
      }

      self.counters
  }
}