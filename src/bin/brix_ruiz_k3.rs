// Retired from Cargo targets in RFC Phase 6.
// Kept in-tree as a historical reference for older research notes.

use std::time::Instant;

use sse_core::matrix::SqMatrix;
use sse_core::search::search_sse_2x2_with_telemetry;
use sse_core::types::{SearchConfig, SearchMode, SseResult, DEFAULT_BEAM_WIDTH};

fn main() {
    let a = SqMatrix::new([[1, 3], [2, 1]]);
    let b = SqMatrix::new([[1, 6], [1, 1]]);

    let mut max_lag = 7usize;
    let mut max_intermediate_dim = 4usize;
    let mut max_entry = 10u32;
    let mut search_mode = SearchMode::Mixed;
    let mut beam_width = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-lag" => {
                max_lag = args
                    .next()
                    .expect("--max-lag requires a value")
                    .parse()
                    .expect("invalid max lag");
            }
            "--max-dim" => {
                max_intermediate_dim = args
                    .next()
                    .expect("--max-dim requires a value")
                    .parse()
                    .expect("invalid max dim");
            }
            "--max-entry" => {
                max_entry = args
                    .next()
                    .expect("--max-entry requires a value")
                    .parse()
                    .expect("invalid max entry");
            }
            "--graph-only" => {
                search_mode = SearchMode::GraphOnly;
            }
            "--search-mode" => {
                let mode = args.next().expect("--search-mode requires a value");
                search_mode = match mode.as_str() {
                    "mixed" => SearchMode::Mixed,
                    "graph-only" => SearchMode::GraphOnly,
                    "beam" => {
                        beam_width = Some(beam_width.unwrap_or(DEFAULT_BEAM_WIDTH));
                        SearchMode::Mixed
                    }
                    _ => panic!("invalid --search-mode value: {mode}"),
                };
            }
            "--beam-width" => {
                let width = args
                    .next()
                    .expect("--beam-width requires a value")
                    .parse::<usize>()
                    .expect("invalid beam width");
                assert!(width > 0, "--beam-width must be at least 1");
                beam_width = Some(width);
            }
            "--help" | "-h" => {
                println!(
                    "usage: brix_ruiz_k3 [--max-lag N] [--max-dim N] [--max-entry N] [--graph-only] [--search-mode mixed|graph-only|beam] [--beam-width N]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    let config = SearchConfig {
        max_lag,
        max_intermediate_dim,
        max_entry,
        search_mode,
        beam_width,
    };

    println!("Brix-Ruiz k=3: A = {:?}, B = {:?}", a, b);
    println!(
        "Config: max_lag={}, max_intermediate_dim={}, max_entry={}, search_mode={:?}",
        config.max_lag, config.max_intermediate_dim, config.max_entry, config.search_mode
    );
    println!();

    let start = Instant::now();
    let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
    let elapsed = start.elapsed();

    match &result {
        SseResult::Equivalent(path) => {
            println!("FOUND SSE path with {} steps!", path.steps.len());
            println!();
            for (i, step) in path.steps.iter().enumerate() {
                let uv = step.u.mul(&step.v);
                let vu = step.v.mul(&step.u);
                println!(
                    "Step {}: {}x{} -> {}x{}",
                    i + 1,
                    uv.rows,
                    uv.cols,
                    vu.rows,
                    vu.cols
                );
                println!("  UV = {:?}", uv.data);
                println!("  VU = {:?}", vu.data);
            }
        }
        SseResult::EquivalentByConcreteShift(witness) => {
            println!(
                "Found via concrete shift witness: lag={}",
                witness.shift.lag
            );
        }
        SseResult::NotEquivalent(reason) => {
            println!("NOT equivalent: {reason}");
        }
        SseResult::Unknown => {
            println!("UNKNOWN (search exhausted without finding path)");
        }
    }

    println!();
    println!("Elapsed: {:.3}s", elapsed.as_secs_f64());
    println!();
    println!("--- Telemetry ---");
    println!(
        "Frontier nodes expanded: {}",
        telemetry.frontier_nodes_expanded
    );
    println!("Factorisation calls: {}", telemetry.factorisation_calls);
    println!(
        "Factorisations enumerated: {}",
        telemetry.factorisations_enumerated
    );
    println!("Candidates generated: {}", telemetry.candidates_generated);
    println!("Pruned by size: {}", telemetry.pruned_by_size);
    println!("Pruned by spectrum: {}", telemetry.pruned_by_spectrum);
    println!(
        "Candidates after pruning: {}",
        telemetry.candidates_after_pruning
    );
    println!("Collisions with seen: {}", telemetry.collisions_with_seen);
    println!(
        "Collisions with other frontier: {}",
        telemetry.collisions_with_other_frontier
    );
    println!("Discovered nodes: {}", telemetry.discovered_nodes);
    println!("Dead-end nodes: {}", telemetry.dead_end_nodes);
    println!("Enqueued nodes: {}", telemetry.enqueued_nodes);
    println!("Max frontier size: {}", telemetry.max_frontier_size);
    println!("Total visited nodes: {}", telemetry.total_visited_nodes);
    println!();
    println!("--- Move family breakdown ---");
    for (family, stats) in &telemetry.move_family_telemetry {
        println!(
            "  {}: candidates={} after_pruning={} discovered={} exact_meets={} approx_hits={}",
            family,
            stats.candidates_generated,
            stats.candidates_after_pruning,
            stats.discovered_nodes,
            stats.exact_meets,
            stats.approximate_other_side_hits,
        );
    }
    println!();
    println!("--- Per-layer ---");
    for layer in &telemetry.layers {
        println!(
            "  Layer {} ({:?}): frontier={} candidates={} discovered={} next_frontier={} visited={}",
            layer.layer_index,
            layer.direction,
            layer.frontier_nodes,
            layer.candidates_generated,
            layer.discovered_nodes,
            layer.next_frontier_nodes,
            layer.total_visited_nodes,
        );
    }
}
