use std::iter::zip;

use crate::types::CellIndex;
use crate::value_set::ValueSet;

use super::handlers::CellAccumulator;
use super::{Contradition, SolverResult};

pub struct AllDifferentEnforcer {
    assignees: Vec<usize>,
    ids: Vec<u8>,
    scc_set: Vec<SccSet>,
    rec_stack: Vec<usize>,
    data_stack: Vec<usize>,
    cell_nodes: Vec<ValueSet>,
}

#[derive(Copy, Clone, Debug)]
struct SccSet {
    low: ValueSet,
    values: ValueSet,
}

impl SccSet {
    fn union_update(&mut self, other: &SccSet) {
        self.low |= other.low;
        self.values |= other.values;
    }

    fn low_id(&self) -> u8 {
        self.low.value()
    }
}

impl AllDifferentEnforcer {
    pub fn new(num_values: u32) -> AllDifferentEnforcer {
        let num_values = num_values as usize;
        AllDifferentEnforcer {
            assignees: vec![0; num_values],
            ids: vec![0; num_values],
            scc_set: vec![
                SccSet {
                    low: ValueSet::empty(),
                    values: ValueSet::empty()
                };
                num_values
            ],
            rec_stack: Vec::with_capacity(num_values),
            data_stack: Vec::with_capacity(num_values),
            cell_nodes: vec![ValueSet::empty(); num_values],
        }
    }

    // Algorithm: http://www.constraint-programming.com/people/regin/papers/alldiff.pdf
    pub fn enforce_all_different(
        &mut self,
        grid: &mut [ValueSet],
        cells: &[CellIndex],
        candidate_matching: &mut [ValueSet],
        cell_accumulator: &mut CellAccumulator,
    ) -> SolverResult {
        self.enforce_all_different_internal(grid, cells, candidate_matching)?;

        // Remove the remaining edges as they are impossible assignments.
        for (i, cell_node) in self.cell_nodes.iter().enumerate() {
            if !cell_node.is_empty() {
                cell_accumulator.add(cells[i]);
                grid[cells[i]] &= !*cell_node;
            }
        }

        Ok(())
    }

    // Internal section for benchmarking.
    pub fn enforce_all_different_internal(
        &mut self,
        grid: &[ValueSet],
        cells: &[CellIndex],
        candidate_matching: &mut [ValueSet],
    ) -> SolverResult {
        // Copy over the cell values.
        for (i, &cell) in cells.iter().enumerate() {
            self.cell_nodes[i] = grid[cell];
        }

        // Find a maximum matching.
        // A candidate mapping is taken in as a hint. The updated mapping is
        // returned to the caller so that we can use the hint next iteration.
        self.max_matching(candidate_matching)?;

        // Remove the forward edges in the maximum matching.
        for (cell_node, &candidate) in zip(self.cell_nodes.iter_mut(), candidate_matching.iter()) {
            *cell_node &= !candidate;
        }

        // Find and remove strongly-connected components in the
        // implicit directed graph.
        self.remove_scc(candidate_matching);

        Ok(())
    }

    // https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
    // With simplifications as per https://www.cs.cmu.edu/~15451-f18/lectures/lec19-DFS-strong-components.pdf
    fn remove_scc(&mut self, assignees_inv: &[ValueSet]) {
        let rec_stack = &mut self.rec_stack;
        let scc_stack = &mut self.data_stack;
        let cell_nodes = &mut self.cell_nodes;
        let assignees = &self.assignees;
        let ids = &mut self.ids;
        let scc_set = &mut self.scc_set;

        rec_stack.clear();
        scc_stack.clear();

        let mut stack_cell_values = ValueSet::empty();
        let mut index = 0;

        let full_set = ValueSet::full(cell_nodes.len() as u8);
        let mut unseen_cells = full_set;
        let mut unseen_values = full_set;

        while let Some(i_set) = unseen_cells.pop() {
            // Try the next unseen node.
            let i = i_set.value() as usize;

            // If it has no edges, ignore it (it's a fixed value).
            if cell_nodes[i].is_empty() {
                continue;
            }

            rec_stack.push(i);
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
                        let u_set = ValueSet::from_value(u as u8);
                        unseen_cells.remove_set(u_set);
                        let u_inv = assignees_inv[u];
                        stack_cell_values |= u_inv;
                        unseen_values.remove_set(u_inv);
                        scc_stack.push(u);

                        ids[u] = index;
                        // scc_set tells us what we know about the set that `u`
                        // is in.
                        scc_set[u] = SccSet {
                            // low is represented as a ValueSet, so that
                            // bitwise OR preserves the min of the sets.
                            low: ValueSet::from_value(index as u8),
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
                let unseen_adj = cell_nodes[u] & unseen_values;
                if !unseen_adj.is_empty() {
                    let n = assignees[unseen_adj.value() as usize];
                    rec_stack.push(n);
                    stack_state = StackState::NewCall;
                    continue;
                }

                // Handle any adjacent nodes already in the stack.
                // Ignore any that we already know are in the same scc set as u,
                // as they add no new information.
                let mut scc_set_u = scc_set[u];
                let mut stack_adj = cell_nodes[u] & stack_cell_values & !scc_set_u.values;
                scc_set_u.values |= stack_adj;
                while let Some(v) = stack_adj.pop() {
                    let n = assignees[v.value() as usize];
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
                if scc_set_u.low_id() == ids[u] {
                    // Remove the edges and truncate the stack.
                    let mask = !scc_set_u.values;
                    stack_cell_values &= mask;

                    // We know exactly how many cells are in this scc.
                    let set_size = scc_set_u.values.count();
                    let remaining_size = scc_stack.len() - set_size;

                    for w in scc_stack.drain(remaining_size..) {
                        cell_nodes[w] &= mask;
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
    fn max_matching(&mut self, candidate_matching: &mut [ValueSet]) -> SolverResult {
        let num_cells = self.cell_nodes.len();

        let mut assigned_values = ValueSet::empty();

        // Prefill using the candidate mapping.
        for (i, (&candidate, &cell_node)) in candidate_matching
            .iter()
            .zip(self.cell_nodes.iter())
            .enumerate()
        {
            if !(candidate & cell_node).is_empty() {
                assigned_values |= candidate;
                self.assignees[candidate.value() as usize] = i;
            }
        }

        // If we assigned all the values we can bail early.
        if assigned_values.count() == num_cells {
            return Ok(());
        }

        for (i, &candidate) in candidate_matching.iter().enumerate() {
            // Skip assigned nodes.
            if !(candidate & self.cell_nodes[i]).is_empty() {
                continue;
            }

            let values = self.cell_nodes[i] & !assigned_values;
            assigned_values |= if !values.is_empty() {
                // If there is a free assignment, take it.
                let value = values.min_set();
                let v = value.value();
                self.assignees[v as usize] = i;
                value
            } else {
                // Otherwise, find a free value and update the matching.
                self.update_matching(i, assigned_values)?
            };
        }

        for (i, &assignee) in self.assignees.iter().enumerate() {
            let i_set = ValueSet::from_value(i as u8);
            candidate_matching[assignee] = i_set;
        }

        Ok(())
    }

    fn update_matching(
        &mut self,
        cell: CellIndex,
        assigned: ValueSet,
    ) -> Result<ValueSet, Contradition> {
        let c_stack = &mut self.rec_stack;
        let v_stack = &mut self.data_stack;
        c_stack.clear();
        v_stack.clear();

        c_stack.push(cell);

        let mut seen = ValueSet::empty();

        while let Some(&c) = c_stack.last() {
            // Check any unseen values.
            let values = self.cell_nodes[c] & !seen;

            // We've run out of legal values, backtrack.
            if values.is_empty() {
                c_stack.pop();
                v_stack.pop();
                continue;
            }

            // Find the next value. We know this is already assigned.
            let value = values.min_set();
            let v = value.value();
            v_stack.push(v as usize);

            // Check if the next assignee is free.
            // If so then we can assign everything in the stack and return.
            let next_c = self.assignees[v as usize];
            let next_values = self.cell_nodes[next_c] & !assigned;
            if !next_values.is_empty() {
                let next_v = next_values.value();
                self.assignees[next_v as usize] = next_c;
                for (&iv, &ic) in zip(v_stack.iter(), c_stack.iter()) {
                    self.assignees[iv] = ic;
                }

                return Ok(next_values.min_set());
            }

            // Otherwise we need to recurse because v is assigned, and that
            // cell needs to find a new assignment.
            seen |= value;
            c_stack.push(next_c);
        }

        Err(Contradition)
    }
}
