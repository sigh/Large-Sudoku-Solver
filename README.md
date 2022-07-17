# Large Sudoku Solver

A solver for large sudoku grids (up to 121x121). Also solves Sudoku-X puzzles.

## Running

Requires [rust](https://www.rust-lang.org/tools/install) to compile and run:

```shell
cargo run --release <input_filename>
```

## Algorithm

The solver works by representing the puzzles as a set of all-different
constraints, then enforcing generalized arc consistency on each constraint.

Generalized arc consistency is enforced efficiently as described in
[(Régin, 1994)](http://cse.unl.edu/~choueiry/Documents/ReginAAAI-1994.pdf):

1. Represent the constraint as a bipirate graph between cells and values, then
    find a maximal matching using the
[Ford-Fulkerson algorithm](https://en.wikipedia.org/wiki/Ford%E2%80%93Fulkerson_algorithm).
1. Convert the bipirate graph into a directed graph by making each edge from
    a cell to a value a forward edge, then reversing those edges in the maximal
    matching found.
1. Find and eliminate edges in the strongly-connected
   components of the directed graph using
   [Tarjan's algorithm](<https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm>).

This is equivalent to resolving all naked and hidden
singles/pairs/triples/etc in a single pass.

Furthermore, the following optimizations are also implemented:

* Store the maximal matching found between each invocation
  for each constraint. This allows us to only have to re-run the
  matching algorithm on the cells where the matching no longer
  applies. In the common case, this let's us skip step (1) altogether.
* Adds redundant same-value constraints for intersecting regions. This is
  equivalent to the pointing pairs/triples technique.

Other than that it is a backtracking solver which uses an approximation of
the **dom/wdeg** heuristic to choose the cell order. The solver state consists
of a vector of bitsets representing the valid values each cell can take. The
algorithms above are implemented efficiently against the bitset representation.
