use criterion::{criterion_group, criterion_main, Criterion};

use large_sudoku_solver::solver::all_different;
use large_sudoku_solver::types::{CellIndex, ValueType};
use large_sudoku_solver::value_set::{IntBitSet, ValueSet};

type ValueSetType = IntBitSet<i64>;

fn criterion_benchmark(c: &mut Criterion) {
    const NUM_VALUES: usize = 16;

    let full_set = ValueSetType::full(NUM_VALUES as ValueType);

    let mut enforcer = all_different::AllDifferentEnforcer::new(NUM_VALUES as u32);

    let mut grid = vec![ValueSetType::empty(); NUM_VALUES];
    let cells = (0..NUM_VALUES).collect::<Vec<CellIndex>>();
    let mut candidates = vec![ValueSetType::empty(); NUM_VALUES];

    c.bench_function("enforce_all_different full", |b| {
        grid.fill(full_set);
        b.iter(|| {
            candidates.fill(ValueSetType::empty());
            enforcer.enforce_all_different_internal(&grid, &cells, &mut candidates)
        });
    });

    c.bench_function("enforce_all_different solved", |b| {
        grid.splice(
            0..NUM_VALUES,
            (0..NUM_VALUES).map(|v| ValueSetType::from_value(v as ValueType)),
        );

        b.iter(|| {
            candidates.fill(ValueSetType::empty());
            enforcer.enforce_all_different_internal(&grid, &cells, &mut candidates)
        });
    });

    c.bench_function("enforce_all_different partial", |b| {
        grid.fill(full_set);
        grid[5] = ValueSetType::from_iter([0, 1]);
        grid[7] = ValueSetType::from_iter([0, 1, 3]);
        grid[0] = ValueSetType::from_iter(0..9);

        b.iter(|| {
            candidates.fill(ValueSetType::empty());
            enforcer.enforce_all_different_internal(&grid, &cells, &mut candidates)
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
