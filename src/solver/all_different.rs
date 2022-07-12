use std::iter::zip;

use crate::types::CellIndex;
use crate::value_set::ValueSet;

use super::handlers::CellAccumulator;
use super::{Contradition, SolverResult};

pub struct AllDifferentEnforcer {
    assignees: Vec<usize>,
    ids: Vec<ValueSet>,
    lowlinks: Vec<ValueSet>,
    rec_stack: Vec<usize>,
    data_stack: Vec<usize>,
    cell_nodes: Vec<ValueSet>,
}

impl AllDifferentEnforcer {
    pub fn new(num_values: u32) -> AllDifferentEnforcer {
        let num_values = num_values as usize;
        AllDifferentEnforcer {
            assignees: vec![0; num_values],
            ids: vec![ValueSet::empty(); num_values],
            lowlinks: vec![ValueSet::empty(); num_values],
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

        // Remove any remaining edges as they are impossible assignments.
        for (i, cell_node) in self.cell_nodes.iter().enumerate() {
            if !cell_node.is_empty() {
                cell_accumulator.add(cells[i]);
                grid[cells[i]] &= !*cell_node;
            }
        }

        Ok(())
    }

    // https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
    fn remove_scc(&mut self, assignees_inv: &[ValueSet]) {
        let rec_stack = &mut self.rec_stack;
        let scc_stack = &mut self.data_stack;
        let cell_nodes = &mut self.cell_nodes;
        let assignees = &self.assignees;
        let ids = &mut self.ids;
        let lowlinks = &mut self.lowlinks;

        rec_stack.clear();
        scc_stack.clear();

        let mut inv_seen = ValueSet::empty();
        let mut inv_stack_member = ValueSet::empty();
        let mut index_set = ValueSet::from_value0(0);

        let mut unseen_cells = ValueSet::full(cell_nodes.len() as u32);

        while let Some(i_set) = unseen_cells.pop() {
            // Try the next unseen node.
            let i = i_set.value0() as usize;

            // If it has no edges, ignore it (it's a fixed value).
            if cell_nodes[i].is_empty() {
                continue;
            }

            rec_stack.push(i);
            let mut returned_value = None;

            while let Some(&u) = rec_stack.last() {
                match returned_value {
                    None => {
                        // First time we've seen u.
                        let u_set = ValueSet::from_value0(u as u32);
                        unseen_cells.remove_set(u_set);
                        let u_inv = assignees_inv[u];
                        inv_stack_member |= u_inv;
                        inv_seen |= u_inv;
                        scc_stack.push(u);

                        // ids and lowlinks are represented as ValueSets, so that
                        // the min operation can be done by a bitwise or.
                        ids[u] = index_set;
                        lowlinks[u] = index_set;
                        index_set <<= 1;
                    }
                    Some(n) => {
                        // We returned from a recursive call.
                        // n is the value on the stack above our current position.
                        lowlinks[u] = lowlinks[u] | lowlinks[n];
                    }
                }

                // Recurse into the next unseen node.
                let unseen_adj = cell_nodes[u] & !inv_seen;
                if !unseen_adj.is_empty() {
                    let n = assignees[unseen_adj.value0() as usize];
                    rec_stack.push(n);
                    returned_value = None;
                    continue;
                }

                // Handle any adjacent nodes already in the stack.
                let stack_adj = cell_nodes[u] & inv_stack_member;
                for id in stack_adj.map(|v| ids[assignees[v.value0() as usize]]) {
                    lowlinks[u] |= id;
                }

                // We have looked at all the relavent edges.
                // If u is a root node, pop the scc_stack and generate an SCC.
                if lowlinks[u].value0() == ids[u].value0() {
                    // Determine the edges to remove.
                    let mut mask = ValueSet::max();
                    for scc_index in (0..scc_stack.len()).rev() {
                        let w = scc_stack[scc_index];
                        let inv_mask = !assignees_inv[w];
                        inv_stack_member &= inv_mask;
                        mask &= inv_mask;
                        if w == u {
                            break;
                        }
                    }

                    // Remove the edges.
                    while let Some(w) = scc_stack.pop() {
                        cell_nodes[w] &= mask;
                        if w == u {
                            break;
                        }
                    }
                }

                returned_value = Some(u);
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
        for i in 0..num_cells {
            let candidate = candidate_matching[i];
            if !(candidate & self.cell_nodes[i]).is_empty() {
                assigned_values |= candidate;
                self.assignees[candidate.value0() as usize] = i;
            }
        }

        // If we assigned all the values we can bail early.
        if assigned_values.count() == num_cells {
            return Ok(());
        }

        for i in 0..num_cells {
            // Skip assigned nodes.
            if !(candidate_matching[i] & self.cell_nodes[i]).is_empty() {
                continue;
            }

            let values = self.cell_nodes[i] & !assigned_values;
            assigned_values |= if !values.is_empty() {
                // If there is a free assignment, take it.
                let value = values.min_set();
                let v = value.value0();
                self.assignees[v as usize] = i;
                value
            } else {
                // Otherwise, find a free value and update the matching.
                self.update_matching(i, assigned_values)?
            };
        }

        for (i, &assignee) in self.assignees.iter().enumerate() {
            let i_set = ValueSet::from_value0(i as u32);
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
            let v = value.value0();
            v_stack.push(v as usize);

            // Check if the next assignee is free.
            // If so then we can assign everything in the stack and return.
            let next_c = self.assignees[v as usize];
            let next_values = self.cell_nodes[next_c] & !assigned;
            if !next_values.is_empty() {
                let next_v = next_values.value0();
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
