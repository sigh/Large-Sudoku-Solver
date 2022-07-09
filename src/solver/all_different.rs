use std::{cmp, iter::zip};

use crate::types::{CellIndex, ValueSet};

use super::handlers::CellAccumulator;

pub struct AllDifferentEnforcer {
    assignees: Vec<usize>,
    assignees_inv: Vec<ValueSet>,
    ids: Vec<u8>,
    lowlinks: Vec<u8>,
    rec_stack: Vec<usize>,
    data_stack: Vec<usize>,
    cell_nodes: Vec<ValueSet>,
}

impl AllDifferentEnforcer {
    pub fn new(num_values: u32) -> AllDifferentEnforcer {
        let num_values = num_values as usize;
        AllDifferentEnforcer {
            assignees: vec![0; num_values],
            assignees_inv: vec![ValueSet::empty(); num_values],
            ids: vec![0; num_values],
            lowlinks: vec![0; num_values],
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
        cell_accumulator: &mut CellAccumulator,
    ) -> bool {
        // Copy over the cell values.
        for (i, &cell) in cells.iter().enumerate() {
            self.cell_nodes[i] = grid[cell];
        }

        // Find a maximum matching.
        if !self.max_matching() {
            return false;
        }

        // Reverse the edges in the maximum matching.
        for (i, &assignee) in self.assignees.iter().enumerate() {
            let i_set = ValueSet::from_value0(i as u32);
            self.cell_nodes[assignee] &= !i_set;
            self.assignees_inv[assignee] = i_set;
        }

        // Find and remove strongly-connected components in the
        // implicit directed graph.
        self.remove_scc();

        // Remove any remaining edges as they are impossible assignments.
        for (i, &cell) in cells.iter().enumerate() {
            if !self.cell_nodes[i].is_empty() {
                cell_accumulator.add(cell);
                grid[cell] &= !self.cell_nodes[i];
            }
        }

        true
    }

    // https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
    fn remove_scc(&mut self) {
        let rec_stack = &mut self.rec_stack;
        let scc_stack = &mut self.data_stack;
        let ids = &mut self.ids;
        let lowlinks = &mut self.lowlinks;
        let assignees_inv = &mut self.assignees_inv;
        let cell_nodes = &mut self.cell_nodes;
        let assignees = &mut self.assignees;

        rec_stack.clear();
        scc_stack.clear();

        let mut seen = ValueSet::empty();
        let mut inv_seen = ValueSet::empty();
        let mut inv_stack_member = ValueSet::empty();
        let mut index = 0;
        let mut prev_u = 0;

        for i in 0..cell_nodes.len() {
            let cell_node = cell_nodes[i];
            // Try the next unseen node.
            // If it has no edges, ignore it (it's a fixed value).
            if cell_node.is_empty() || !(seen & ValueSet::from_value0(i as u32)).is_empty() {
                continue;
            }

            rec_stack.push(i);

            while let Some(&u) = rec_stack.last() {
                let u_set = ValueSet::from_value0(u as u32);
                if (seen & u_set).is_empty() {
                    // First time we've seen u.
                    ids[u] = index;
                    lowlinks[u] = index;
                    index += 1;
                    seen |= u_set;
                    let u_inv = assignees_inv[u];
                    inv_stack_member |= u_inv;
                    inv_seen |= u_inv;
                    scc_stack.push(u);
                } else {
                    // We returned from a recursive call.
                    // n is the value on the stack above our current position.
                    let n = prev_u;
                    lowlinks[u] = cmp::min(lowlinks[u], lowlinks[n]);
                }

                // Recurse into the next unseen node.
                let unseen_adj = cell_nodes[u] & !inv_seen;
                if !unseen_adj.is_empty() {
                    let n = assignees[unseen_adj.value0() as usize];
                    rec_stack.push(n);
                    continue;
                }

                // Handle any adjacent nodes already in the stack.
                let mut stack_adj = cell_nodes[u] & inv_stack_member;
                while !stack_adj.is_empty() {
                    let node = stack_adj.min();
                    stack_adj.remove(node);
                    let n = assignees[node.value0() as usize];
                    lowlinks[u] = cmp::min(lowlinks[u], ids[n]);
                }

                // We have looked at all the relavent edges.
                // If u is a root node, pop the scc_stack and generate an SCC.
                if lowlinks[u] == ids[u] {
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

                prev_u = u;
                rec_stack.pop();
            }
        }
    }

    // Max bipartite matching algorith from:
    // https://www.geeksforgeeks.org/maximum-bipartite-matching/
    fn max_matching(&mut self) -> bool {
        let mut assigned = ValueSet::empty();

        for i in 0..self.cell_nodes.len() {
            let values = self.cell_nodes[i] & !assigned;
            if !values.is_empty() {
                let value = values.min();
                let v = value.value0();
                self.assignees[v as usize] = i;
                assigned |= value;
            } else {
                let matched = self.update_matching(i, assigned);
                if matched.is_empty() {
                    return false;
                }
                assigned |= matched;
            }
        }

        true
    }

    fn update_matching(&mut self, cell: CellIndex, assigned: ValueSet) -> ValueSet {
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
            let value = values.min();
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

                return next_values.min();
            }

            // Otherwise we need to recurse because v is assigned, and that
            // cell needs to find a new assignment.
            seen |= value;
            c_stack.push(next_c);
        }

        ValueSet::empty()
    }
}
