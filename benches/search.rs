use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sse_core::matrix::SqMatrix;
use sse_core::search::{search_sse_2x2, search_sse_2x2_with_telemetry};
use sse_core::types::{FrontierMode, MoveFamilyPolicy, SearchConfig};

/// Fast equivalent endpoint sanity check.
fn bench_endpoint_equivalent_fast(c: &mut Criterion) {
    let a = SqMatrix::new([[2, 1], [1, 1]]);
    let b = SqMatrix::new([[1, 1], [1, 2]]);
    let config = SearchConfig {
        max_lag: 4,
        max_intermediate_dim: 2,
        max_entry: 10,
        frontier_mode: FrontierMode::Bfs,
        move_family_policy: MoveFamilyPolicy::Mixed,
        beam_width: None,
        beam_bfs_handoff_depth: None,
    };
    c.bench_function("endpoint_equivalent_fast", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &config));
    });
}

/// Fast invariant rejection sanity check.
fn bench_endpoint_invariant_reject_fast(c: &mut Criterion) {
    let a = SqMatrix::new([[14, 2], [1, 0]]);
    let b = SqMatrix::new([[13, 5], [3, 1]]);
    let config = SearchConfig {
        max_lag: 3,
        max_intermediate_dim: 2,
        max_entry: 15,
        frontier_mode: FrontierMode::Bfs,
        move_family_policy: MoveFamilyPolicy::Mixed,
        beam_width: None,
        beam_bfs_handoff_depth: None,
    };
    c.bench_function("endpoint_invariant_reject_fast", |bencher| {
        bencher.iter(|| search_sse_2x2(&a, &b, &config));
    });
}

struct ExpandNextNCase {
    name: &'static str,
    a: SqMatrix<2>,
    b: SqMatrix<2>,
    config: SearchConfig,
    target_expanded_nodes: usize,
}

fn run_expand_next_n(case: &ExpandNextNCase) -> usize {
    let mut expanded_nodes = 0usize;
    while expanded_nodes < case.target_expanded_nodes {
        let (_result, telemetry) = search_sse_2x2_with_telemetry(
            black_box(&case.a),
            black_box(&case.b),
            black_box(&case.config),
        );
        let expanded = telemetry.frontier_nodes_expanded;
        assert!(
            expanded > 0,
            "expand_next_n case '{}' must expand at least one node",
            case.name
        );
        expanded_nodes = expanded_nodes.saturating_add(expanded);
        black_box(telemetry.factorisations_enumerated);
        black_box(telemetry.candidates_after_pruning);
    }
    expanded_nodes
}

/// Throughput benches for frontier expansion.
///
/// These are deterministic, telemetry-driven microbenches: each sample repeats
/// the same endpoint search until expanded_nodes >= N, then reports throughput
/// in expanded nodes. Lag-sensitive literature families such as Riedel/Baker
/// remain in research_harness because they are better treated as bounded
/// scenario probes than low-noise Criterion surfaces.
fn bench_expand_next_n(c: &mut Criterion) {
    let cases = [
        ExpandNextNCase {
            name: "mixed_k3_lag3_dim3_n2048",
            a: SqMatrix::new([[1, 3], [2, 1]]),
            b: SqMatrix::new([[1, 6], [1, 1]]),
            config: SearchConfig {
                max_lag: 3,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::Mixed,
                beam_width: None,
                beam_bfs_handoff_depth: None,
            },
            target_expanded_nodes: 2_048,
        },
        ExpandNextNCase {
            name: "graph_only_k3_lag8_dim4_n8192",
            a: SqMatrix::new([[1, 3], [2, 1]]),
            b: SqMatrix::new([[1, 6], [1, 1]]),
            config: SearchConfig {
                max_lag: 8,
                max_intermediate_dim: 4,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
            },
            target_expanded_nodes: 8_192,
        },
    ];

    let mut group = c.benchmark_group("expand_next_n");
    group.sample_size(10);

    for case in &cases {
        // Calibrate throughput units from the actual expanded work done by one
        // benchmark sample (which may overshoot the target threshold).
        let sample_expanded_nodes = run_expand_next_n(case);
        group.throughput(Throughput::Elements(sample_expanded_nodes as u64));
        group.bench_with_input(
            BenchmarkId::new("frontier_expansion", case.name),
            case,
            |bencher, case| {
                bencher.iter(|| black_box(run_expand_next_n(case)));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_endpoint_equivalent_fast,
    bench_endpoint_invariant_reject_fast,
    bench_expand_next_n,
);
criterion_main!(benches);
