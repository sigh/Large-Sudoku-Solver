use std::iter::zip;

use crate::types::{CellIndex, ValueType};
use crate::value_set::ValueSet;

use super::cell_accumulator::CellAccumulator;
use super::handlers;
use super::handlers::Contradition;

pub struct AllDifferentEnforcer<VS: ValueSet> {
    assignees: Vec<usize>,
    ids: Vec<ValueType>,
    scc_set: Vec<SccSet<VS>>,
    rec_stack: Vec<usize>,
    data_stack: Vec<usize>,
    cell_nodes: Vec<VS>,
}

#[derive(Copy, Clone, Debug)]
struct SccSet<VS: ValueSet> {
    low: VS,
    values: VS,
}

impl<VS: ValueSet> SccSet<VS> {
    fn union_update(&mut self, other: &SccSet<VS>) {
        self.low.add_set(&other.low);
        self.values.add_set(&other.values);
    }

    fn low_id(&self) -> Option<ValueType> {
        self.low.min()
    }
}

impl<VS: ValueSet> AllDifferentEnforcer<VS> {
    pub fn new(num_values: u32) -> Self {
        let num_values = num_values as usize;
        Self {
            assignees: vec![0; num_values],
            ids: vec![0; num_values],
            scc_set: vec![
                SccSet {
                    low: VS::empty(),
                    values: VS::empty()
                };
                num_values
            ],
            rec_stack: Vec::with_capacity(num_values),
            data_stack: Vec::with_capacity(num_values),
            cell_nodes: vec![VS::empty(); num_values],
        }
    }

    // Algorithm: http://www.constraint-programming.com/people/regin/papers/alldiff.pdf
    pub fn enforce_all_different(
        &mut self,
        grid: &mut [VS],
        cells: &[CellIndex],
        candidate_matching: &mut [VS],
        cell_accumulator: &mut CellAccumulator,
    ) -> handlers::Result {
        self.enforce_all_different_internal(grid, cells, candidate_matching)?;

        // Remove the remaining edges as they are impossible assignments.
        for (i, cell_node) in self.cell_nodes.iter().enumerate() {
            if !cell_node.is_empty() {
                cell_accumulator.add(cells[i]);
                grid[cells[i]].remove_set(cell_node);
            }
        }

        Ok(())
    }

    // Internal section for benchmarking.
    pub fn enforce_all_different_internal(
        &mut self,
        grid: &[VS],
        cells: &[CellIndex],
        candidate_matching: &mut [VS],
    ) -> handlers::Result {
        println!("Initial state: ");
        for (i, &cell) in cells.iter().enumerate() {
            println!("  {}: {:?}", i, grid[cell].values());
        }

        // Copy over the cell values.
        for (i, &cell) in cells.iter().enumerate() {
            self.cell_nodes[i] = grid[cell];
        }

        // Find a maximum matching.
        // A candidate mapping is taken in as a hint. The updated mapping is
        // returned to the caller so that we can use the hint next iteration.
        self.max_matching(candidate_matching)?;

        // Remove the forward edges in the maximum matching.
        for (cell_node, candidate) in zip(self.cell_nodes.iter_mut(), candidate_matching.iter()) {
            cell_node.remove_set(candidate);
        }

        // Find and remove strongly-connected components in the
        // implicit directed graph.
        self.remove_scc(candidate_matching);

        Ok(())
    }

    // https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
    // With simplifications as per https://www.cs.cmu.edu/~15451-f18/lectures/lec19-DFS-strong-components.pdf
    fn remove_scc(&mut self, assignees_inv: &[VS]) {
        let rec_stack = &mut self.rec_stack;
        let scc_stack = &mut self.data_stack;
        let cell_nodes = &mut self.cell_nodes;
        let assignees = &self.assignees;
        let ids = &mut self.ids;
        let scc_set = &mut self.scc_set;

        rec_stack.clear();
        scc_stack.clear();

        let mut stack_cell_values = VS::empty();
        let mut index = 0;

        let full_set = VS::full(cell_nodes.len() as ValueType);
        let mut unseen_cells = full_set;
        let mut unseen_values = full_set;
        let mut used_values = VS::empty();

        while let Some(i) = unseen_cells.pop() {
            // Try the next unseen node.

            // If it has no edges, ignore it (it's a fixed value).
            if cell_nodes[i as usize].is_empty() {
                continue;
            }

            rec_stack.push(i as usize);
            enum StackState {
                NewCall,
                NoResult,
                SCCNodeResult(usize),
            }
            let mut stack_state = StackState::NewCall;

            while let Some(&u) = rec_stack.last() {
                match stack_state {
                    StackState::NewCall => {
                        // First time we've seen u.
                        let u_set = VS::from_value(u as ValueType);
                        unseen_cells.remove_set(&u_set);
                        let u_inv = assignees_inv[u];
                        stack_cell_values.add_set(&u_inv);
                        unseen_values.remove_set(&u_inv);
                        scc_stack.push(u);

                        ids[u] = index;
                        // scc_set tells us what we know about the set that `u`
                        // is in.
                        scc_set[u] = SccSet {
                            // low is represented as a VS, so that
                            // bitwise OR preserves the min of the sets.
                            low: VS::from_value(index as ValueType),
                            values: u_inv,
                        };
                        index += 1;
                    }
                    StackState::NoResult => {}
                    StackState::SCCNodeResult(n) => {
                        // This is not necessary for correctness (as n in an
                        // adjacency which will be handled below).
                        // However it is vital for performance to skip over
                        // the seen values. ~2x performance increase.
                        let scc_set_n = scc_set[n];
                        scc_set[u].union_update(&scc_set_n);
                    }
                }

                // Recurse into the next unseen node.
                let unseen_adj = cell_nodes[u].intersection(&unseen_values);
                if let Some(value) = unseen_adj.min() {
                    let n = assignees[value as usize];
                    rec_stack.push(n);
                    stack_state = StackState::NewCall;
                    continue;
                }

                // Handle any adjacent nodes already in the stack.
                // Ignore any that we already know are in the same scc set as u,
                // as they add no new information.
                let mut scc_set_u = scc_set[u];
                let mut stack_adj = cell_nodes[u]
                    .intersection(&stack_cell_values)
                    .without(&scc_set_u.values);
                scc_set_u.values.add_set(&stack_adj);
                while let Some(value) = stack_adj.pop() {
                    let n = assignees[value as usize];
                    // We preserve the invariant that
                    // `low_set[u].value0() = lowlinks[u]`. This is because
                    // bitwise OR preserves the min of two sets.
                    scc_set_u.union_update(&scc_set[n]);
                    // NOTE: We could remove `scc_set[n].values` from
                    // `stack_adj` here, but it is only helpful a minority of
                    // the time.
                    // This is because `stack_adj` already contained
                    // `v = assignees_inv[n]` so we need extra edges not unique
                    // to `n`. We've also found a bunch from our recursion.
                }

                // We have looked at all the relavent edges.
                // If u is a root node, pop the scc_stack and generate an SCC.
                if scc_set_u.low_id() == Some(ids[u]) {
                    // Remove the edges and truncate the stack.
                    let mask = scc_set_u.values;
                    stack_cell_values.remove_set(&mask);
                    used_values.add_set(&mask);

                    // We know exactly how many cells are in this scc.
                    // NOTE: count seem more efficient than searching for
                    //       `u` in the scc_stack.
                    let set_size = scc_set_u.values.count();
                    let remaining_size = scc_stack.len() - set_size;

                    let scc_cells = &scc_stack[remaining_size..];
                    print!("Found SCC - Values: {:?} Cells: {:?}", mask, scc_cells);
                    // If any of the cells in the scc have values not in the
                    // mask, we have a hidden tuple.
                    if scc_cells
                        .iter()
                        .any(|&w| !cell_nodes[w].without(&used_values).is_empty())
                    {
                        print!(" (hidden tuple)");
                        print!(" {:?}", &used_values);
                    }
                    // If any cells not in scc_cells contain values in mask, we
                    // have a naked tuple.
                    if cell_nodes.iter().enumerate().any(|(i, &cell)| {
                        !scc_cells.contains(&i) && !cell.intersection(&mask).is_empty()
                    }) {
                        print!(" (naked tuple)");
                    }
                    println!();

                    for w in scc_stack.drain(remaining_size..) {
                        // let removed = cell_nodes[w].intersection(&mask);
                        cell_nodes[w].remove_set(&mask);
                    }
                    stack_state = StackState::NoResult;
                } else {
                    stack_state = StackState::SCCNodeResult(u);
                }

                scc_set[u] = scc_set_u;
                rec_stack.pop();
            }
        }
    }

    // Max bipartite matching algorith from:
    // Implementation of the Ford–Fulkerson algorithm method.
    // https://en.wikipedia.org/wiki/Ford%E2%80%93Fulkerson_algorithm
    // See also https://www.geeksforgeeks.org/maximum-bipartite-matching/
    fn max_matching(&mut self, candidate_matching: &mut [VS]) -> handlers::Result {
        let num_cells = self.cell_nodes.len();

        let mut assigned_values = VS::empty();

        // Prefill using the candidate mapping.
        for (i, (&candidate, &cell_node)) in candidate_matching
            .iter()
            .zip(self.cell_nodes.iter())
            .enumerate()
        {
            if let Some(candidate_value) = candidate.intersection(&cell_node).min() {
                assigned_values.add_set(&candidate);
                self.assignees[candidate_value as usize] = i;
            }
        }

        // If we assigned all the values we can bail early.
        if assigned_values == ValueSet::full(num_cells as ValueType) {
            return Ok(());
        }

        for (i, candidate) in candidate_matching.iter().enumerate() {
            // Skip assigned nodes.
            if !(candidate.intersection(&self.cell_nodes[i])).is_empty() {
                continue;
            }

            let values = self.cell_nodes[i].without(&assigned_values);
            assigned_values.add_set(&match values.min() {
                Some(v) => {
                    // If there is a free assignment, take it.
                    self.assignees[v as usize] = i;
                    VS::from_value(v)
                }

                None => {
                    // Otherwise, find a free value and update the matching.
                    self.update_matching(i, &assigned_values)?
                }
            });
        }

        for (i, &assignee) in self.assignees.iter().enumerate() {
            let i_set = VS::from_value(i as ValueType);
            candidate_matching[assignee] = i_set;
        }

        Ok(())
    }

    fn update_matching(&mut self, cell: CellIndex, assigned: &VS) -> Result<VS, Contradition> {
        let c_stack = &mut self.rec_stack;
        let v_stack = &mut self.data_stack;
        c_stack.clear();
        v_stack.clear();

        c_stack.push(cell);

        let mut seen = VS::empty();

        while let Some(&c) = c_stack.last() {
            // Check any unseen values.
            let values = self.cell_nodes[c].without(&seen);

            // Find the next value, or backtrack if we are out of legal values.
            let v = match values.min() {
                None => {
                    // Backtrack.
                    c_stack.pop();
                    v_stack.pop();
                    continue;
                }
                Some(v) => v,
            };

            v_stack.push(v as usize);

            // Check if the next assignee is free.
            // If so then we can assign everything in the stack and return.
            let next_c = self.assignees[v as usize];
            let next_values = self.cell_nodes[next_c].without(assigned);
            if let Some(next_v) = next_values.min() {
                self.assignees[next_v as usize] = next_c;
                for (&iv, &ic) in zip(v_stack.iter(), c_stack.iter()) {
                    self.assignees[iv] = ic;
                }

                return Ok(VS::from_value(next_v));
            }

            // Otherwise we need to recurse because v is assigned, and that
            // cell needs to find a new assignment.
            seen.add_set(&VS::from_value(v));
            c_stack.push(next_c);
        }

        Err(Contradition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_set::IntBitSet;

    type ValueSetType = IntBitSet<i64>;
    const NUM_VALUES: usize = 9;

    fn run_enforcer(grid: &[ValueSetType]) -> handlers::Result {
        let cells: Vec<CellIndex> = (0..NUM_VALUES).collect::<Vec<CellIndex>>();
        let mut candidates = vec![ValueSetType::empty(); NUM_VALUES];
        let mut enforcer = AllDifferentEnforcer::new(NUM_VALUES as u32);
        return enforcer.enforce_all_different_internal(&grid, &cells, &mut candidates);
    }

    fn make_grid() -> Vec<ValueSetType> {
        let full_set: ValueSetType = ValueSetType::full(NUM_VALUES as ValueType);
        let mut grid = vec![ValueSetType::empty(); NUM_VALUES];
        grid.fill(full_set);

        return grid;
    }

    #[test]
    fn all_values() {
        let grid = make_grid();
        let _ = run_enforcer(&grid);
    }

    #[test]
    fn partial() {
        let mut grid = make_grid();
        grid[5] = ValueSetType::from_iter([0, 1]);
        grid[7] = ValueSetType::from_iter([0, 1]);

        let _ = run_enforcer(&grid);
    }

    #[test]
    fn progressive() {
        let mut grid = make_grid();
        grid[0] = ValueSetType::from_iter([1]);
        grid[1] = ValueSetType::from_iter([1, 2]);
        grid[2] = ValueSetType::from_iter([2, 3]);
        grid[3] = ValueSetType::from_iter([3, 4]);

        let _ = run_enforcer(&grid);
    }

    #[test]
    fn hidden_tuple() {
        let mut grid = make_grid();
        grid[2] = ValueSetType::from_iter(2..9);
        grid[3] = ValueSetType::from_iter(2..9);
        grid[4] = ValueSetType::from_iter(2..9);
        grid[5] = ValueSetType::from_iter(2..9);
        grid[6] = ValueSetType::from_iter(2..9);
        grid[7] = ValueSetType::from_iter(2..9);
        grid[8] = ValueSetType::from_iter(2..9);

        let _ = run_enforcer(&grid);
    }

    #[test]
    fn disjoint() {
        let mut grid = make_grid();
        grid[0] = ValueSetType::from_iter(0..4);
        grid[1] = ValueSetType::from_iter(0..4);
        grid[2] = ValueSetType::from_iter(0..4);
        grid[3] = ValueSetType::from_iter(0..4);
        grid[4] = ValueSetType::from_iter(4..9);
        grid[5] = ValueSetType::from_iter(4..9);
        grid[6] = ValueSetType::from_iter(4..9);
        grid[7] = ValueSetType::from_iter(4..9);
        grid[8] = ValueSetType::from_iter(4..9);

        let _ = run_enforcer(&grid);
    }
}
