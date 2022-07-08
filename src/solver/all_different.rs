use std::cmp;

use crate::types::{CellIndex, ValueSet};

// Algorithm: http://www.constraint-programming.com/people/regin/papers/alldiff.pdf
pub fn enforce_all_different(grid: &mut [ValueSet], cells: &[CellIndex]) -> bool {
    let mut cell_nodes = cells.iter().map(|c| grid[*c]).collect::<Vec<_>>();
    let mut assignees = vec![0; cells.len()];

    if !max_matching(&cell_nodes, &mut assignees) {
        return false;
    }

    remove_scc(&mut cell_nodes, &assignees);

    for (i, cell) in cells.iter().enumerate() {
        grid[*cell] &= !cell_nodes[i];
    }

    true
}

// https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
fn remove_scc(cell_nodes: &mut [ValueSet], assignees: &[usize]) {
    let mut rec_stack = Vec::new();
    let mut scc_stack = Vec::new();
    let mut ids = vec![0; cell_nodes.len()];
    let mut lowlinks = vec![0; cell_nodes.len()];
    let mut assignees_inv = vec![ValueSet::empty(); cell_nodes.len()];

    let mut seen = ValueSet::empty();
    let mut inv_seen = ValueSet::empty();
    let mut inv_stack_member = ValueSet::empty();
    let mut index = 0;
    let mut prev_rec_stack_top = 0;

    for (i, &assignee) in assignees.iter().enumerate() {
        let i_set = ValueSet::from_value0(i as u32);
        cell_nodes[assignee] &= !i_set;
        assignees_inv[assignee] = i_set;
    }

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
                let n = prev_rec_stack_top;
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

                let mut w = u;
                loop {
                    // Remove the edges in the SCC from the graph.
                    cell_nodes[w] &= mask;
                    w = scc_stack.pop().unwrap();
                    if w == u {
                        break;
                    }
                }
            }

            prev_rec_stack_top = *rec_stack.last().unwrap();
            rec_stack.pop();
        }
    }
}

// Max bipartite matching algorith from:
// https://www.geeksforgeeks.org/maximum-bipartite-matching/
fn max_matching(cell_nodes: &[ValueSet], assignees: &mut [usize]) -> bool {
    let mut assigned = ValueSet::empty();

    for (i, cell_node) in cell_nodes.iter().enumerate() {
        let values = *cell_node & !assigned;
        if !values.is_empty() {
            let value = values.min();
            let v = value.value0();
            assignees[v as usize] = i;
            assigned |= value;
        } else {
            let matched = update_matching(cell_nodes, i, assignees, assigned);
            if matched.is_empty() {
                return false;
            }
            assigned |= matched;
        }
    }

    true
}

fn update_matching(
    cell_nodes: &[ValueSet],
    cell: CellIndex,
    assignees: &mut [usize],
    assigned: ValueSet,
) -> ValueSet {
    let mut c_stack = vec![cell; 1];
    let mut v_stack = vec![0; cell_nodes.len()];

    let mut seen = ValueSet::empty();

    while let Some(c) = c_stack.last() {
        // Check any unseen values.
        let values = cell_nodes[*c] & !seen;

        // We've run out of legal values, backtrack.
        if values.is_empty() {
            c_stack.pop();
            continue;
        }

        // Find the next value. We know this is already assigned.
        let value = values.min();
        let v = value.value0();
        v_stack[c_stack.len() - 1] = v;

        // Check if the next assignee is free.
        // If so then we can assign everything in the stack and return.
        let next_c = assignees[v as usize];
        let next_values = cell_nodes[next_c] & !assigned;
        if !next_values.is_empty() {
            let next_v = next_values.value0();
            assignees[next_v as usize] = next_c;
            while let Some(c) = c_stack.pop() {
                assignees[v_stack[c_stack.len()] as usize] = c;
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
