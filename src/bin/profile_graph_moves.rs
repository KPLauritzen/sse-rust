use std::collections::HashSet;
use std::time::Instant;

use sse_core::graph_moves::enumerate_graph_move_successors;
use sse_core::matrix::{DynMatrix, SqMatrix};

fn main() {
    let start_matrix = DynMatrix::from_sq(&SqMatrix::new([[1, 3], [2, 1]]));
    let max_dim = 5;
    let max_entry = 6u32;

    // Collect frontier states at each depth.
    println!("Building frontiers...");
    let mut seen = HashSet::new();
    seen.insert(start_matrix.canonical_perm());

    let mut frontier = vec![start_matrix.canonical_perm()];

    for depth in 0..3 {
        let t0 = Instant::now();
        let mut next_frontier = Vec::new();
        for m in &frontier {
            let succ = enumerate_graph_move_successors(m, max_dim);
            for s in succ.nodes {
                if s.matrix.max_entry() <= max_entry && seen.insert(s.matrix.clone()) {
                    next_frontier.push(s.matrix);
                }
            }
        }
        let el = t0.elapsed();
        let n4 = next_frontier.iter().filter(|m| m.rows == 4).count();
        let n3 = next_frontier.iter().filter(|m| m.rows == 3).count();
        let n2 = next_frontier.iter().filter(|m| m.rows == 2).count();
        let n5 = next_frontier.iter().filter(|m| m.rows == 5).count();
        println!(
            "  depth {}: expanded {} -> {} new states ({:.3}s) [2x2:{}, 3x3:{}, 4x4:{}, 5x5:{}]",
            depth,
            frontier.len(),
            next_frontier.len(),
            el.as_secs_f64(),
            n2,
            n3,
            n4,
            n5,
        );
        frontier = next_frontier;
    }

    // Now frontier contains the depth-3 states (the 10489 nodes).
    println!("\nProfiling depth-3 frontier: {} nodes", frontier.len());

    // Dimension breakdown
    let by_dim: Vec<(usize, Vec<&DynMatrix>)> = {
        let mut map = std::collections::BTreeMap::new();
        for m in &frontier {
            map.entry(m.rows).or_insert_with(Vec::new).push(m);
        }
        map.into_iter().collect()
    };
    for (dim, nodes) in &by_dim {
        println!("  {}x{}: {} nodes", dim, dim, nodes.len());
    }

    // Profile per-dimension
    println!("\nPer-dimension successor enumeration:");
    for (dim, nodes) in &by_dim {
        let sample_size = nodes.len().min(2000);
        let sample = &nodes[..sample_size];

        let t0 = Instant::now();
        let mut total_cands = 0usize;
        let mut total_succs = 0usize;
        for m in sample {
            let succ = enumerate_graph_move_successors(m, max_dim);
            total_cands += succ.candidates;
            total_succs += succ.nodes.len();
        }
        let el = t0.elapsed();
        println!(
            "  {}x{}: n={}, {:.3}s, {:.3}ms/node, cands/node={:.1}, succs/node={:.1}",
            dim,
            dim,
            sample_size,
            el.as_secs_f64(),
            el.as_secs_f64() * 1000.0 / sample_size as f64,
            total_cands as f64 / sample_size as f64,
            total_succs as f64 / sample_size as f64,
        );
    }

    // For the dominant dimension, break down phases.
    let dominant_dim = by_dim
        .iter()
        .max_by_key(|(_, nodes)| nodes.len())
        .map(|(d, _)| *d)
        .unwrap_or(4);
    let dominant_nodes: Vec<_> = frontier
        .iter()
        .filter(|m| m.rows == dominant_dim)
        .take(1000)
        .collect();

    if !dominant_nodes.is_empty() {
        println!(
            "\nPhase breakdown for {} {}x{} matrices:",
            dominant_nodes.len(),
            dominant_dim,
            dominant_dim,
        );

        // Phase A: split enumeration (raw, no canonicalization)
        let t0 = Instant::now();
        let mut raw_splits = 0usize;
        for m in &dominant_nodes {
            for parent in 0..dominant_dim {
                let row: Vec<u32> = (0..dominant_dim).map(|c| m.get(parent, c)).collect();
                let splits =
                    sse_core::graph_moves::profiling_helpers::split_row_into_children(&row, 2);
                raw_splits += splits.len();
            }
        }
        let el_splits = t0.elapsed();
        println!(
            "  A. split_row_into_children (outsplit only, {} parents/node): {:.3}s, {:.1} raw_splits/node, {:.3}ms/node",
            dominant_dim,
            el_splits.as_secs_f64(),
            raw_splits as f64 / dominant_nodes.len() as f64,
            el_splits.as_secs_f64() * 1000.0 / dominant_nodes.len() as f64,
        );

        // Phase B: full enumerate_graph_move_successors
        let t0 = Instant::now();
        let mut total_cands = 0;
        let mut total_succs = 0;
        for m in &dominant_nodes {
            let succ = enumerate_graph_move_successors(m, max_dim);
            total_cands += succ.candidates;
            total_succs += succ.nodes.len();
        }
        let el_full = t0.elapsed();
        println!(
            "  B. full enumerate_graph_move_successors: {:.3}s, {:.3}ms/node, cands={}, succs={}",
            el_full.as_secs_f64(),
            el_full.as_secs_f64() * 1000.0 / dominant_nodes.len() as f64,
            total_cands,
            total_succs,
        );

        // Phase C: transpose cost
        let t0 = Instant::now();
        for m in &dominant_nodes {
            for _ in 0..2 {
                let _ = std::hint::black_box(m.transpose());
            }
        }
        let el_transpose = t0.elapsed();
        println!(
            "  C. transpose (2x per node): {:.3}s, {:.3}ms/node",
            el_transpose.as_secs_f64(),
            el_transpose.as_secs_f64() * 1000.0 / dominant_nodes.len() as f64,
        );

        // Phase D: canonical_perm cost by size
        for target_dim in [3, 4, 5] {
            let t0 = Instant::now();
            let mut count = 0usize;
            for m in dominant_nodes.iter().take(200) {
                let succ = enumerate_graph_move_successors(m, max_dim);
                for s in &succ.nodes {
                    if s.matrix.rows == target_dim {
                        let _ = std::hint::black_box(s.matrix.canonical_perm());
                        count += 1;
                    }
                }
            }
            let el = t0.elapsed();
            if count > 0 {
                println!(
                    "  D. {}x{} canonical_perm: {:.3}s for {} calls ({:.1}μs/call) [includes successor computation for 200 nodes]",
                    target_dim, target_dim,
                    el.as_secs_f64(), count,
                    el.as_secs_f64() * 1e6 / count as f64,
                );
            }
        }

        // Phase E: amalgamation enumeration only
        let t0 = Instant::now();
        let mut amal_count = 0;
        for m in &dominant_nodes {
            let out = sse_core::graph_moves::enumerate_out_amalgamations(m);
            let in_ = sse_core::graph_moves::enumerate_in_amalgamations(m);
            amal_count += out.len() + in_.len();
        }
        let el = t0.elapsed();
        println!(
            "  E. amalgamation enumeration: {:.3}s, {:.1} results/node, {:.3}ms/node",
            el.as_secs_f64(),
            amal_count as f64 / dominant_nodes.len() as f64,
            el.as_secs_f64() * 1000.0 / dominant_nodes.len() as f64,
        );
    }
}
