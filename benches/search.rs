use criterion::{criterion_group, criterion_main, Criterion};
use sse_core::aligned::{
    search_aligned_module_shift_equivalence_2x2, AlignedModuleSearchConfig2x2,
};
use sse_core::matrix::SqMatrix;
use sse_core::search::search_sse_2x2;
use sse_core::types::SearchConfig;

/// Elementary SSE pair: [[2,1],[1,1]] <-> [[1,1],[1,2]].
/// Should be found in 1 step with small bounds.
fn bench_elementary_pair(c: &mut Criterion) {
    let a = SqMatrix::new([[2, 1], [1, 1]]);
    let b = SqMatrix::new([[1, 1], [1, 2]]);
    let config = SearchConfig {
        max_lag: 4,
        max_intermediate_dim: 2,
        max_entry: 10,
    };
    c.bench_function("elementary_pair", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &config));
    });
}

/// Pair requiring a 3x3 intermediate: [[2,1],[1,1]] <-> [[1,0],[1,2]].
fn bench_rectangular_pair(c: &mut Criterion) {
    let a = SqMatrix::new([[2, 1], [1, 1]]);
    let b = SqMatrix::new([[1, 0], [1, 2]]);
    let config = SearchConfig {
        max_lag: 4,
        max_intermediate_dim: 3,
        max_entry: 5,
    };
    c.bench_function("rectangular_pair", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &config));
    });
}

/// Not-equivalent pair detected by Eilers-Kiming invariant.
/// Measures invariant pre-filter speed.
fn bench_not_equivalent_invariant(c: &mut Criterion) {
    let a = SqMatrix::new([[14, 2], [1, 0]]);
    let b = SqMatrix::new([[13, 5], [3, 1]]);
    let config = SearchConfig {
        max_lag: 3,
        max_intermediate_dim: 2,
        max_entry: 15,
    };
    c.bench_function("not_equivalent_invariant", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &config));
    });
}

/// Hard known-SSE pair (Brix-Ruiz k=3). Search space is large.
/// This is the target for optimisation work.
fn bench_brix_ruiz_k3(c: &mut Criterion) {
    let a = SqMatrix::new([[1, 3], [2, 1]]);
    let b = SqMatrix::new([[1, 6], [1, 1]]);
    let config = SearchConfig {
        max_lag: 6,
        max_intermediate_dim: 3,
        max_entry: 6,
    };
    let mut group = c.benchmark_group("brix_ruiz_k3");
    group.sample_size(20);
    group.bench_function("search", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &config));
    });
    group.finish();
}

/// Larger entry bound search to stress-test BFS frontier expansion.
fn bench_large_entry_bound(c: &mut Criterion) {
    let a = SqMatrix::new([[2, 1], [1, 1]]);
    let b = SqMatrix::new([[1, 1], [1, 2]]);
    let config = SearchConfig {
        max_lag: 4,
        max_intermediate_dim: 2,
        max_entry: 25,
    };
    c.bench_function("large_entry_bound", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &config));
    });
}

/// Compare BFS and aligned-module witness search on an easy pair.
fn bench_elementary_pair_compare(c: &mut Criterion) {
    let a = SqMatrix::new([[2, 1], [1, 1]]);
    let b = SqMatrix::new([[1, 1], [1, 2]]);
    let bfs = SearchConfig {
        max_lag: 4,
        max_intermediate_dim: 2,
        max_entry: 10,
    };
    let aligned = AlignedModuleSearchConfig2x2 {
        max_lag: 1,
        max_entry: 3,
        max_module_witnesses: 500,
    };

    let mut group = c.benchmark_group("elementary_compare");
    group.bench_function("bfs", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &bfs));
    });
    group.bench_function("aligned_module", |bencher| {
        bencher.iter(|| search_aligned_module_shift_equivalence_2x2(&a, &b, &aligned));
    });
    group.finish();
}

/// Compare BFS and aligned-module witness search on the hard Brix-Ruiz k=3 example.
fn bench_brix_ruiz_k3_compare(c: &mut Criterion) {
    let a = SqMatrix::new([[1, 3], [2, 1]]);
    let b = SqMatrix::new([[1, 6], [1, 1]]);
    let bfs = SearchConfig {
        max_lag: 6,
        max_intermediate_dim: 3,
        max_entry: 6,
    };
    let aligned = AlignedModuleSearchConfig2x2 {
        max_lag: 3,
        max_entry: 6,
        max_module_witnesses: 5_000,
    };

    let mut group = c.benchmark_group("brix_ruiz_k3_compare");
    group.sample_size(10);
    group.bench_function("bfs", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &bfs));
    });
    group.bench_function("aligned_module", |bencher| {
        bencher.iter(|| search_aligned_module_shift_equivalence_2x2(&a, &b, &aligned));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_elementary_pair,
    bench_rectangular_pair,
    bench_not_equivalent_invariant,
    bench_brix_ruiz_k3,
    bench_large_entry_bound,
    bench_elementary_pair_compare,
    bench_brix_ruiz_k3_compare,
);
criterion_main!(benches);
