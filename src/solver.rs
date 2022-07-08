use crate::types::CellIndex;
use crate::types::Shape;
use crate::types::ValueSet;
use crate::types::Grid;
use crate::types::FixedValues;

pub fn solve(shape: &Shape, fixed_values: &FixedValues) {
  let solver = Solver::new(shape, fixed_values);

  for (i, result) in solver.enumerate() {
    if i > 1 { panic!("Too many solutions."); }
    println!("{}", result.solution);
    println!("{:?}", result.counters);
  }
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

fn make_cell_conflicts(houses: &[House], shape: &Shape) -> Vec<CellConflicts> {
  let mut result: Vec<CellConflicts> = vec![Vec::new(); shape.num_cells];
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

fn enforce_value(grid: &mut Grid, value: ValueSet, cell: CellIndex, cell_conflicts: &[CellConflicts]) -> bool {
    for conflict_cell in &cell_conflicts[cell] {
        let values = &mut grid.cells[*conflict_cell];
        values.remove(value);
        if values.empty() { return false; }
    }
    true
}


#[derive(Copy, Clone, Debug, Default)]
pub struct Counters {
  values_tried: u64,
  cells_searched: u64,
  backtracks: u64,
  guesses: u64,
  solutions: u64,
}

struct SolverOutput {
  counters: Counters,
  solution: Grid,
}

struct Solver {
  shape: Shape,
  stack: Vec<CellIndex>,
  grids: Vec<Grid>,
  cell_conflicts: Vec<CellConflicts>,
  backtrack_triggers: Vec<u32>,
  counters: Counters,
  done: bool,
  depth: usize,
}

impl Iterator for Solver {
  type Item = SolverOutput;

  fn next(&mut self) -> Option<Self::Item> {
    self.run();
    if self.done { return None; }

    Some(SolverOutput{
      solution: self.grids.last().unwrap().clone(),
      counters: self.counters,
    })
  }
}

impl Solver {
  fn new(shape: &Shape, fixed_values: &FixedValues) -> Solver {
    let houses = make_houses(shape);
    let mut empty_grid = Grid::new(shape);
    empty_grid.cells.fill(ValueSet::full(shape.num_values));

    let mut grids = vec![empty_grid; shape.num_cells+1];
    for (cell, value) in fixed_values {
        grids[0].cells[*cell] = ValueSet::from_value(*value);
    }

    Solver{
      shape: *shape,
      stack: (0..shape.num_cells).collect(),
      grids: grids,
      cell_conflicts: make_cell_conflicts(&houses, shape),
      backtrack_triggers: vec![0; shape.num_cells],
      counters: Counters::default(),
      done: false,
      depth: 1,
    }
  }

  fn run(&mut self) {
      if self.done { return; }

      while self.depth > 0 {
          let (grids_front, grids_back) = self.grids.split_at_mut(self.depth);
          self.depth -= 1;

          let mut grid = &mut grids_front[self.depth];
          let cell = self.stack[self.depth];
          let values = &mut grid.cells[cell];

          // No more values to try.
          if values.empty() { continue; }

          // Find the next value to try.
          let value = values.min();
          self.counters.values_tried += 1;
          self.counters.guesses += (*values != value) as u64;
          values.remove(value);

          // Copy the current cell values.
          self.depth += 1;
          grids_back[0].copy_from(grid);

          // Update the grid with the trial value.
          grid = &mut grids_back[0];
          grid.cells[cell] = value;

          // Propograte constraints.
          let has_contradiction = !enforce_value(&mut grid, value, cell, &self.cell_conflicts);
          if has_contradiction {
            self.record_backtrack(cell);
            continue;
          }

          // Check if we have a solution.
          if self.depth == self.shape.num_cells {
              self.counters.solutions += 1;
              return;
          }

          // Find the next cell to try.
          self.update_cell_order(self.depth);
          self.counters.cells_searched += 1;
          self.depth += 1;
      }

      self.done = true;
  }

  fn record_backtrack(&mut self, cell: CellIndex) {
      const BACKTRACK_DECAY_INTERVAL: u64 = 100*100;
      self.counters.backtracks += 1;
      if 0 == self.counters.backtracks % BACKTRACK_DECAY_INTERVAL {
          for i in 0..self.backtrack_triggers.len() {
              self.backtrack_triggers[i] >>= 1;
          }
      }
      self.backtrack_triggers[cell] += 1;
  }

  // Find the best cell and bring it to the front. This means that it will
  // be processed next.
  fn update_cell_order(&mut self, depth: usize) -> u32 {
      let mut best_index = 0;
      let mut min_score = u32::MAX;

      let stack = &mut self.stack;
      let grid = &mut self.grids[depth];

      for i in depth..stack.len() {
          let cell = stack[i];
          let count = grid.cells[cell].count();

          // If we have a single value then just use it - as it will involve no
          // guessing.
          if count <= 1 {
              best_index = i;
              break;
          }

          let bt = self.backtrack_triggers[cell];
          let score = if bt > 1 { count/bt } else { count };

          if score < min_score {
              best_index= i;
              min_score = score;
          }
      }

      // Swap the best cell into place.
      (stack[best_index], stack[depth]) = (stack[depth], stack[best_index]);

      grid.cells[stack[depth]].count()
  }
}