use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use sse_core::factorisation::{
    square_factorisation_3x3_permutation_orbit_key, visit_all_factorisations_with_family,
};
use sse_core::matrix::DynMatrix;
use sse_core::search::execute_search_request_and_observer;
use sse_core::search_observer::{SearchEvent, SearchObserver};
use sse_core::types::{FrontierMode, MoveFamilyPolicy, SearchConfig, SearchRequest, SearchStage};

#[derive(Default)]
struct FactorSourceObserver {
    counts: BTreeMap<DynMatrix, SourceStats>,
    family_counts: BTreeMap<DynMatrix, BTreeMap<&'static str, SourceStats>>,
}

#[derive(Clone, Default)]
struct SourceStats {
    total_edges: usize,
    discovered_edges: usize,
    exact_meets: usize,
    seen_collisions: usize,
    approximate_hits: usize,
}

struct RawSquareSummary {
    callbacks: usize,
    unique_permutation_orbits: usize,
    unique_exact_successors: usize,
    unique_canonical_successors: usize,
    elapsed_ms: u128,
}

#[derive(Default)]
struct RawBucketSummary {
    sources: usize,
    post_total_edges: usize,
    post_discovered_edges: usize,
    post_seen_collisions: usize,
    raw_callbacks: usize,
    raw_permutation_orbits: usize,
    raw_exact_successors: usize,
    raw_canonical_successors: usize,
    raw_elapsed_ms: u128,
}

impl SearchObserver for FactorSourceObserver {
    fn on_event(&mut self, event: &SearchEvent) {
        let SearchEvent::Layer(edges) = event else {
            return;
        };

        for edge in edges {
            let family_stats = self
                .family_counts
                .entry(edge.from_orig.clone())
                .or_default()
                .entry(edge.move_family)
                .or_default();
            update_stats(family_stats, edge.approximate_other_side_hit, edge.status);

            if edge.move_family == "square_factorisation_3x3" {
                let stats = self.counts.entry(edge.from_orig.clone()).or_default();
                update_stats(stats, edge.approximate_other_side_hit, edge.status);
            }
        }
    }
}

fn main() -> Result<(), String> {
    let request = SearchRequest {
        source: DynMatrix::new(2, 2, vec![1, 3, 2, 1]),
        target: DynMatrix::new(2, 2, vec![1, 6, 1, 1]),
        config: SearchConfig {
            max_lag: 6,
            max_intermediate_dim: 3,
            max_entry: 6,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        },
        stage: SearchStage::EndpointSearch,
        guide_artifacts: Vec::new(),
        guided_refinement: Default::default(),
        shortcut_search: Default::default(),
    };

    let mut observer = FactorSourceObserver::default();
    let (result, telemetry) = execute_search_request_and_observer(&request, Some(&mut observer))?;
    eprintln!("result={result:?}");
    eprintln!(
        "telemetry: layers={} factorisations={} candidates_after_pruning={} discovered={}",
        telemetry.layers.len(),
        telemetry.factorisations_enumerated,
        telemetry.candidates_after_pruning,
        telemetry.discovered_nodes
    );

    let mut rows = observer
        .counts
        .iter()
        .map(|(matrix, stats)| (matrix.clone(), stats.clone()))
        .collect::<Vec<_>>();
    rows.sort_by(|(left_matrix, left_stats), (right_matrix, right_stats)| {
        right_stats
            .total_edges
            .cmp(&left_stats.total_edges)
            .then_with(|| {
                right_stats
                    .discovered_edges
                    .cmp(&left_stats.discovered_edges)
            })
            .then_with(|| left_matrix.data.cmp(&right_matrix.data))
    });

    println!("Top square_factorisation_3x3 sources");
    for (idx, (matrix, stats)) in rows.into_iter().take(15).enumerate() {
        let features = feature_signature(&matrix);
        let family_summary = observer
            .family_counts
            .get(&matrix)
            .map(format_family_summary)
            .unwrap_or_default();
        let raw_square = measure_raw_square_factorisations(&matrix, 4);
        println!(
            "{}. total={} discovered={} seen={} meets={} approx={} raw_sq={} raw_orbit={} raw_exact={} raw_canon={} raw_ms={} features={} families={} matrix={}",
            idx + 1,
            stats.total_edges,
            stats.discovered_edges,
            stats.seen_collisions,
            stats.exact_meets,
            stats.approximate_hits,
            raw_square.callbacks,
            raw_square.unique_permutation_orbits,
            raw_square.unique_exact_successors,
            raw_square.unique_canonical_successors,
            raw_square.elapsed_ms,
            features,
            family_summary,
            format_matrix(&matrix),
        );
    }

    let mut buckets = BTreeMap::<String, SourceStats>::new();
    for (matrix, stats) in &observer.counts {
        let bucket = buckets.entry(feature_signature(matrix)).or_default();
        bucket.total_edges += stats.total_edges;
        bucket.discovered_edges += stats.discovered_edges;
        bucket.seen_collisions += stats.seen_collisions;
        bucket.exact_meets += stats.exact_meets;
        bucket.approximate_hits += stats.approximate_hits;
    }

    let mut bucket_rows = buckets.into_iter().collect::<Vec<_>>();
    bucket_rows.sort_by(|(left_key, left_stats), (right_key, right_stats)| {
        right_stats
            .total_edges
            .cmp(&left_stats.total_edges)
            .then_with(|| {
                right_stats
                    .discovered_edges
                    .cmp(&left_stats.discovered_edges)
            })
            .then_with(|| left_key.cmp(right_key))
    });

    println!();
    println!("Feature buckets");
    for (bucket, stats) in bucket_rows.into_iter().take(15) {
        println!(
            "total={} discovered={} seen={} meets={} approx={} bucket={}",
            stats.total_edges,
            stats.discovered_edges,
            stats.seen_collisions,
            stats.exact_meets,
            stats.approximate_hits,
            bucket,
        );
    }

    let mut raw_bucket_rows = BTreeMap::<String, RawBucketSummary>::new();
    for (matrix, stats) in &observer.counts {
        let bucket = feature_signature(matrix);
        let raw_square = measure_raw_square_factorisations(matrix, 4);
        let entry = raw_bucket_rows.entry(bucket).or_default();
        entry.sources += 1;
        entry.post_total_edges += stats.total_edges;
        entry.post_discovered_edges += stats.discovered_edges;
        entry.post_seen_collisions += stats.seen_collisions;
        entry.raw_callbacks += raw_square.callbacks;
        entry.raw_permutation_orbits += raw_square.unique_permutation_orbits;
        entry.raw_exact_successors += raw_square.unique_exact_successors;
        entry.raw_canonical_successors += raw_square.unique_canonical_successors;
        entry.raw_elapsed_ms += raw_square.elapsed_ms;
    }

    let mut raw_bucket_rows = raw_bucket_rows.into_iter().collect::<Vec<_>>();
    raw_bucket_rows.sort_by(|(left_key, left_stats), (right_key, right_stats)| {
        right_stats
            .raw_callbacks
            .cmp(&left_stats.raw_callbacks)
            .then_with(|| {
                right_stats
                    .raw_canonical_successors
                    .cmp(&left_stats.raw_canonical_successors)
            })
            .then_with(|| left_key.cmp(right_key))
    });

    println!();
    println!("Raw square-factorisation buckets");
    for (bucket, stats) in raw_bucket_rows.into_iter().take(15) {
        println!(
            "raw_callbacks={} raw_orbit={} raw_exact={} raw_canon={} raw_ms={} post_total={} post_discovered={} post_seen={} sources={} bucket={}",
            stats.raw_callbacks,
            stats.raw_permutation_orbits,
            stats.raw_exact_successors,
            stats.raw_canonical_successors,
            stats.raw_elapsed_ms,
            stats.post_total_edges,
            stats.post_discovered_edges,
            stats.post_seen_collisions,
            stats.sources,
            bucket,
        );
    }

    Ok(())
}

fn format_matrix(matrix: &DynMatrix) -> String {
    let rows = (0..matrix.rows)
        .map(|row| {
            let values = (0..matrix.cols)
                .map(|col| matrix.get(row, col).to_string())
                .collect::<Vec<_>>()
                .join(",");
            format!("[{values}]")
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{rows}")
}

fn feature_signature(matrix: &DynMatrix) -> String {
    let mut row_counts = BTreeMap::new();
    let mut col_counts = BTreeMap::new();
    let mut zero_rows = 0usize;
    let mut zero_cols = 0usize;

    for row in 0..matrix.rows {
        let values = (0..matrix.cols)
            .map(|col| matrix.get(row, col))
            .collect::<Vec<_>>();
        if values.iter().all(|&value| value == 0) {
            zero_rows += 1;
        }
        *row_counts.entry(values).or_insert(0usize) += 1;
    }

    for col in 0..matrix.cols {
        let values = (0..matrix.rows)
            .map(|row| matrix.get(row, col))
            .collect::<Vec<_>>();
        if values.iter().all(|&value| value == 0) {
            zero_cols += 1;
        }
        *col_counts.entry(values).or_insert(0usize) += 1;
    }

    let duplicate_rows = row_counts
        .values()
        .map(|&count| count.saturating_sub(1))
        .sum::<usize>();
    let duplicate_cols = col_counts
        .values()
        .map(|&count| count.saturating_sub(1))
        .sum::<usize>();
    let entry_sum: u32 = matrix.data.iter().copied().sum();
    format!(
        "sum={entry_sum},dup_rows={duplicate_rows},dup_cols={duplicate_cols},zero_rows={zero_rows},zero_cols={zero_cols}"
    )
}

fn update_stats(
    stats: &mut SourceStats,
    approximate_other_side_hit: bool,
    status: sse_core::search_observer::SearchEdgeStatus,
) {
    stats.total_edges += 1;
    stats.approximate_hits += usize::from(approximate_other_side_hit);
    match status {
        sse_core::search_observer::SearchEdgeStatus::Discovered => {
            stats.discovered_edges += 1;
        }
        sse_core::search_observer::SearchEdgeStatus::ExactMeet => {
            stats.exact_meets += 1;
        }
        sse_core::search_observer::SearchEdgeStatus::SeenCollision => {
            stats.seen_collisions += 1;
        }
    }
}

fn format_family_summary(families: &BTreeMap<&'static str, SourceStats>) -> String {
    let mut entries = families.iter().collect::<Vec<_>>();
    entries.sort_by(|(left_name, left_stats), (right_name, right_stats)| {
        right_stats
            .discovered_edges
            .cmp(&left_stats.discovered_edges)
            .then_with(|| right_stats.total_edges.cmp(&left_stats.total_edges))
            .then_with(|| left_name.cmp(right_name))
    });

    entries
        .into_iter()
        .take(4)
        .map(|(family, stats)| format!("{family}:{}/{}", stats.discovered_edges, stats.total_edges))
        .collect::<Vec<_>>()
        .join(",")
}

fn measure_raw_square_factorisations(matrix: &DynMatrix, sq3_cap: u32) -> RawSquareSummary {
    let started = Instant::now();
    let mut callbacks = 0usize;
    let mut permutation_orbits = BTreeSet::new();
    let mut exact_successors = BTreeSet::new();
    let mut canonical_successors = BTreeSet::new();
    visit_all_factorisations_with_family(matrix, 3, sq3_cap, |family, u, v| {
        if family != "square_factorisation_3x3" {
            return;
        }
        callbacks += 1;
        if let Some(key) = square_factorisation_3x3_permutation_orbit_key(&u, &v) {
            permutation_orbits.insert(key);
        }
        let next = v.mul(&u);
        exact_successors.insert(next.data.clone());
        canonical_successors.insert(next.canonical_perm().data);
    });
    RawSquareSummary {
        callbacks,
        unique_permutation_orbits: permutation_orbits.len(),
        unique_exact_successors: exact_successors.len(),
        unique_canonical_successors: canonical_successors.len(),
        elapsed_ms: started.elapsed().as_millis(),
    }
}
