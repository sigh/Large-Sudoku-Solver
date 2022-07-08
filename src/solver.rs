use crate::types::CellIndex;
use crate::types::Shape;
use crate::types::ValueSet;
use crate::types::Grid;
use crate::types::FixedValues;

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

pub fn solve(shape: &Shape, fixed_values: &FixedValues) {
    let houses = make_houses(shape);
    let cell_conflicts = make_cell_conflicts(&houses, &shape);

    let mut stack: Vec<CellIndex> = (0..shape.num_cells).collect();
    let mut grids: Vec<Grid> =
        (0..shape.num_cells+1).map(|_| Grid::new(shape)).collect();

    for grid in &mut grids {
        grid.cells.fill(ValueSet::full(shape.num_values));
    }

    let mut depth = 0;
    for (cell, value) in fixed_values {
        grids[depth].cells[*cell] = ValueSet::from_value(*value);
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
        let has_contradiction = !enforce_value(&mut grid, value, cell, &cell_conflicts);
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