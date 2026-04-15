// Retired from Cargo targets in RFC Phase 6.
// Kept in-tree as a historical reference for older research notes.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use rayon::prelude::*;
use rusqlite::{params, Connection};
use sse_core::graph_moves::{enumerate_graph_move_successors, GraphMoveSuccessors};
use sse_core::matrix::{DynMatrix, SqMatrix};

fn main() {
    let mut max_depth = 22usize;
    let mut max_dim = 5usize;
    let mut max_entry = 6u32;
    let mut max_states = 1_000_000usize;
    let mut max_candidates = 10_000_000usize;
    let mut max_seconds = 30u64;
    let mut k = 3u32;
    let mut use_cache = false;
    let mut seed_depth = 0usize;
    let mut continue_through_found_layer = false;
    let mut visited_db: Option<String> = None;
    let mut print_path_limit = 3usize;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--k" => {
                k = args
                    .next()
                    .expect("--k requires a value")
                    .parse()
                    .expect("invalid k");
            }
            "--use-cache" => {
                use_cache = true;
            }
            "--seed-depth" => {
                seed_depth = args
                    .next()
                    .expect("--seed-depth requires a value")
                    .parse()
                    .expect("invalid seed depth");
            }
            "--continue-through-found-layer" | "--dont-stop-at-layer-depth" => {
                continue_through_found_layer = true;
            }
            "--max-depth" => {
                max_depth = args
                    .next()
                    .expect("--max-depth requires a value")
                    .parse()
                    .expect("invalid max depth");
            }
            "--max-dim" => {
                max_dim = args
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
            "--max-states" => {
                max_states = args
                    .next()
                    .expect("--max-states requires a value")
                    .parse()
                    .expect("invalid max states");
            }
            "--max-candidates" => {
                max_candidates = args
                    .next()
                    .expect("--max-candidates requires a value")
                    .parse()
                    .expect("invalid max candidates");
            }
            "--max-seconds" => {
                max_seconds = args
                    .next()
                    .expect("--max-seconds requires a value")
                    .parse()
                    .expect("invalid max seconds");
            }
            "--visited-db" | "--sqlite" => {
                visited_db = Some(args.next().expect("--visited-db requires a value"));
            }
            "--print-path-limit" => {
                print_path_limit = args
                    .next()
                    .expect("--print-path-limit requires a value")
                    .parse()
                    .expect("invalid print path limit");
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_brix_ruiz_graph_path [--k N] [--max-depth N] [--max-dim N] [--max-entry N] [--max-states N] [--max-candidates N] [--max-seconds N] [--use-cache] [--seed-depth N] [--continue-through-found-layer] [--visited-db PATH] [--print-path-limit N]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    assert!(k >= 2, "Brix-Ruiz family requires k >= 2");
    let start = DynMatrix::from_sq(&SqMatrix::new([[1, k], [k - 1, 1]]));
    let target = DynMatrix::from_sq(&SqMatrix::new([[1, k * (k - 1)], [1, 1]]));

    println!("Blind graph-only Brix-Ruiz k={k} search");
    println!(
        "Config: max_depth={max_depth}, max_dim={max_dim}, max_entry={max_entry}, max_states={max_states}, max_candidates={max_candidates}, max_seconds={max_seconds}, use_cache={use_cache}, seed_depth={seed_depth}, continue_through_found_layer={continue_through_found_layer}, visited_db={}",
        visited_db.as_deref().unwrap_or("none")
    );
    println!("Moves: outsplit, insplit, out_amalgamation, in_amalgamation");
    println!("States are canonicalized up to vertex permutation.");
    println!();

    let mut recorder = visited_db
        .as_deref()
        .map(GraphPathSqliteRecorder::new)
        .transpose()
        .unwrap_or_else(|err| panic!("{err}"));

    if let Some(recorder) = recorder.as_mut() {
        let config = GraphSearchRunConfig {
            k,
            max_depth,
            max_dim,
            max_entry,
            max_states,
            max_candidates,
            max_seconds,
            use_cache,
            seed_depth,
            continue_through_found_layer,
        };
        recorder
            .start_run(&start, &target, &config)
            .unwrap_or_else(|err| panic!("{err}"));
    }

    let result = search_graph_path(
        &start,
        &target,
        max_depth,
        max_dim,
        max_entry,
        max_states,
        max_candidates,
        Duration::from_secs(max_seconds),
        use_cache,
        seed_depth,
        continue_through_found_layer,
    );

    if let Some(recorder) = recorder.as_mut() {
        recorder
            .finish_run(&result)
            .unwrap_or_else(|err| panic!("{err}"));
    }

    match result {
        GraphSearchResult::Found {
            depth,
            paths,
            visited,
            candidates,
            elapsed,
        } => {
            let unique_meetings = paths
                .iter()
                .map(|path| matrix_key(&path.meeting))
                .collect::<HashSet<_>>()
                .len();
            println!();
            println!("FOUND {} graph-only path(s) at depth {depth}", paths.len());
            println!("unique meeting states = {unique_meetings}");
            println!("visited states = {visited}");
            println!("candidates generated = {candidates}");
            println!("elapsed = {:.3}s", elapsed.as_secs_f64());
            for (idx, path) in paths.iter().take(print_path_limit).enumerate() {
                println!();
                println!(
                    "Path {} / {}: meeting={:?}, discovered_from={}, forward_depth={}, backward_depth={}",
                    idx + 1,
                    paths.len(),
                    path.meeting.data,
                    path.discovered_from,
                    path.forward_depth,
                    path.backward_depth
                );
                print_path(&path.path);
            }
            if paths.len() > print_path_limit {
                println!();
                println!(
                    "Omitted {} additional path(s); inspect the sqlite db or rerun with --print-path-limit.",
                    paths.len() - print_path_limit
                );
            }
        }
        GraphSearchResult::NotFound {
            visited,
            candidates,
            elapsed,
        } => {
            println!();
            println!("No graph-only path found within depth {max_depth}");
            println!("visited states = {visited}");
            println!("candidates generated = {candidates}");
            println!("elapsed = {:.3}s", elapsed.as_secs_f64());
        }
        GraphSearchResult::Capped {
            reason,
            visited,
            candidates,
            elapsed,
        } => {
            println!();
            println!("{reason} before exhaustion");
            println!("visited states = {visited}");
            println!("candidates generated = {candidates}");
            println!("elapsed = {:.3}s", elapsed.as_secs_f64());
        }
    }
}

#[derive(Debug)]
enum GraphSearchResult {
    Found {
        depth: usize,
        paths: Vec<FoundGraphPath>,
        visited: usize,
        candidates: usize,
        elapsed: Duration,
    },
    NotFound {
        visited: usize,
        candidates: usize,
        elapsed: Duration,
    },
    Capped {
        reason: &'static str,
        visited: usize,
        candidates: usize,
        elapsed: Duration,
    },
}

#[derive(Clone, Debug)]
struct PathStep {
    family: &'static str,
    from: DynMatrix,
    to: DynMatrix,
}

#[derive(Clone, Debug)]
struct FoundGraphPath {
    meeting: DynMatrix,
    path: Vec<PathStep>,
    discovered_from: &'static str,
    forward_depth: usize,
    backward_depth: usize,
}

#[derive(Clone, Debug)]
struct ParentStep {
    parent: DynMatrix,
    family: &'static str,
}

#[derive(Default)]
struct LayerStats {
    candidates: usize,
    deduped_candidates: usize,
    pruned_by_entry: usize,
    collisions_with_seen: usize,
    discovered: usize,
    successor_cache_hits: usize,
    successor_cache_misses: usize,
    family_counts: BTreeMap<&'static str, usize>,
}

struct LayerOutcome {
    terminal: Option<GraphSearchResult>,
    stats: LayerStats,
}

#[allow(clippy::too_many_arguments)]
fn search_graph_path(
    start: &DynMatrix,
    target: &DynMatrix,
    max_depth: usize,
    max_dim: usize,
    max_entry: u32,
    max_states: usize,
    max_candidates: usize,
    max_runtime: Duration,
    use_cache: bool,
    seed_depth: usize,
    continue_through_found_layer: bool,
) -> GraphSearchResult {
    let started = Instant::now();
    let mut total_candidates = 0usize;
    let start = start.canonical_perm();
    let target = target.canonical_perm();
    if start == target {
        return GraphSearchResult::Found {
            depth: 0,
            paths: vec![FoundGraphPath {
                meeting: start.clone(),
                path: Vec::new(),
                discovered_from: "forward",
                forward_depth: 0,
                backward_depth: 0,
            }],
            visited: 1,
            candidates: 0,
            elapsed: started.elapsed(),
        };
    }

    let mut fwd_seen = HashMap::<DynMatrix, usize>::new();
    let mut bwd_seen = HashMap::<DynMatrix, usize>::new();
    let mut fwd_parent = HashMap::<DynMatrix, ParentStep>::new();
    let mut bwd_parent = HashMap::<DynMatrix, ParentStep>::new();
    let mut fwd_frontier = VecDeque::<DynMatrix>::new();
    let mut bwd_frontier = VecDeque::<DynMatrix>::new();
    let mut successor_cache = HashMap::<DynMatrix, Arc<GraphMoveSuccessors>>::new();
    let mut fwd_candidates_per_node = 1.0f64;
    let mut bwd_candidates_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;
    let mut found_paths = Vec::<FoundGraphPath>::new();
    let mut best_found_depth = None::<usize>;

    fwd_seen.insert(start.clone(), 0);
    bwd_seen.insert(target.clone(), 0);
    fwd_frontier.push_back(start);
    bwd_frontier.push_back(target);
    // Track the union size incrementally to avoid O(|seen| * |other_seen|) per new state.
    let mut total_visited = 2usize;

    // Optionally pre-expand the backward side without max_entry pruning so that
    // a target with large entries (e.g. brix_ruiz_k4 has B = [[1,12],[1,1]]) can
    // be reached from a low-entry universe via a small unbounded neighborhood.
    if seed_depth > 0 {
        seed_backward_neighborhood(
            seed_depth,
            max_dim,
            &mut bwd_seen,
            &mut bwd_parent,
            &mut bwd_frontier,
            &mut total_visited,
        );
        println!(
            "Backward seed expansion to depth {seed_depth}: bwd_seen={}, bwd_frontier={}",
            bwd_seen.len(),
            bwd_frontier.len(),
        );
    }

    loop {
        let next_fwd_depth = fwd_frontier.front().and_then(|m| fwd_seen.get(m)).copied();
        let next_bwd_depth = bwd_frontier.front().and_then(|m| bwd_seen.get(m)).copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_candidates_per_node,
            bwd_candidates_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
        ) else {
            return GraphSearchResult::NotFound {
                visited: total_visited,
                candidates: total_candidates,
                elapsed: started.elapsed(),
            };
        };

        if layer_depth >= max_depth {
            return GraphSearchResult::NotFound {
                visited: total_visited,
                candidates: total_candidates,
                elapsed: started.elapsed(),
            };
        }

        let result = if expand_forward {
            expand_layer(
                "forward",
                layer_depth,
                &mut fwd_frontier,
                &mut fwd_seen,
                &mut fwd_parent,
                &bwd_seen,
                &bwd_parent,
                &mut successor_cache,
                use_cache,
                max_depth,
                max_dim,
                max_entry,
                max_states,
                max_candidates,
                max_runtime,
                started,
                &mut total_candidates,
                &mut total_visited,
                continue_through_found_layer,
                &mut found_paths,
                &mut best_found_depth,
            )
        } else {
            expand_layer(
                "backward",
                layer_depth,
                &mut bwd_frontier,
                &mut bwd_seen,
                &mut bwd_parent,
                &fwd_seen,
                &fwd_parent,
                &mut successor_cache,
                use_cache,
                max_depth,
                max_dim,
                max_entry,
                max_states,
                max_candidates,
                max_runtime,
                started,
                &mut total_candidates,
                &mut total_visited,
                continue_through_found_layer,
                &mut found_paths,
                &mut best_found_depth,
            )
        };

        if result.stats.successor_cache_hits + result.stats.successor_cache_misses > 0 {
            let candidates_per_node = result.stats.candidates.max(1) as f64
                / (result.stats.successor_cache_hits + result.stats.successor_cache_misses) as f64;
            if expand_forward {
                fwd_candidates_per_node = candidates_per_node;
                fwd_cost_sample_nodes =
                    result.stats.successor_cache_hits + result.stats.successor_cache_misses;
            } else {
                bwd_candidates_per_node = candidates_per_node;
                bwd_cost_sample_nodes =
                    result.stats.successor_cache_hits + result.stats.successor_cache_misses;
            }
        }

        if let Some(result) = result.terminal {
            return result;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn expand_layer(
    side_name: &'static str,
    layer_depth: usize,
    frontier: &mut VecDeque<DynMatrix>,
    seen: &mut HashMap<DynMatrix, usize>,
    parent: &mut HashMap<DynMatrix, ParentStep>,
    other_seen: &HashMap<DynMatrix, usize>,
    other_parent: &HashMap<DynMatrix, ParentStep>,
    successor_cache: &mut HashMap<DynMatrix, Arc<GraphMoveSuccessors>>,
    use_cache: bool,
    max_depth: usize,
    max_dim: usize,
    max_entry: u32,
    max_states: usize,
    max_candidates: usize,
    max_runtime: Duration,
    started: Instant,
    total_candidates: &mut usize,
    total_visited: &mut usize,
    continue_through_found_layer: bool,
    found_paths: &mut Vec<FoundGraphPath>,
    best_found_depth: &mut Option<usize>,
) -> LayerOutcome {
    let layer_start = Instant::now();
    let current_layer_len = frontier
        .iter()
        .take_while(|node| seen.get(*node).copied() == Some(layer_depth))
        .count();

    let expand_forward = side_name == "forward";
    let mut stats = LayerStats::default();
    let mut next_frontier = Vec::new();
    let mut current_layer = Vec::with_capacity(current_layer_len);
    for _ in 0..current_layer_len {
        current_layer.push(frontier.pop_front().expect("frontier length should match"));
    }

    if started.elapsed() >= max_runtime {
        print_layer_summary(
            side_name,
            layer_depth,
            current_layer_len,
            &stats,
            *total_visited,
        );
        return LayerOutcome {
            terminal: Some(GraphSearchResult::Capped {
                reason: "Time cap hit",
                visited: *total_visited,
                candidates: *total_candidates,
                elapsed: started.elapsed(),
            }),
            stats,
        };
    }

    let phase_prep = layer_start.elapsed();

    let mut successors_by_node = Vec::with_capacity(current_layer_len);
    let mut missing = Vec::new();
    for (idx, current) in current_layer.iter().enumerate() {
        let cached = if use_cache {
            successor_cache.get(current)
        } else {
            None
        };
        if let Some(successors) = cached {
            stats.successor_cache_hits += 1;
            successors_by_node.push(Some(Arc::clone(successors)));
        } else {
            stats.successor_cache_misses += 1;
            successors_by_node.push(None);
            missing.push((idx, current.clone()));
        }
    }

    let phase_cache_check = layer_start.elapsed() - phase_prep;
    let compute_start = Instant::now();

    let computed_successors: Vec<(usize, DynMatrix, GraphMoveSuccessors)> = missing
        .into_par_iter()
        .map(|(idx, current)| {
            let successors = enumerate_graph_move_successors(&current, max_dim);
            (idx, current, successors)
        })
        .collect();

    let phase_compute = compute_start.elapsed();

    for (idx, current, successors) in computed_successors {
        let successors = Arc::new(successors);
        if use_cache {
            successor_cache.insert(current, Arc::clone(&successors));
        }
        successors_by_node[idx] = Some(successors);
    }

    let phase_cache_insert = Instant::now();
    let seq_start = phase_cache_insert;

    for (current, successors) in current_layer
        .into_iter()
        .zip(successors_by_node.into_iter())
    {
        if started.elapsed() >= max_runtime {
            print_layer_summary(
                side_name,
                layer_depth,
                current_layer_len,
                &stats,
                *total_visited,
            );
            return LayerOutcome {
                terminal: Some(GraphSearchResult::Capped {
                    reason: "Time cap hit",
                    visited: *total_visited,
                    candidates: *total_candidates,
                    elapsed: started.elapsed(),
                }),
                stats,
            };
        }

        let successors = successors.expect("successors should be cached or computed");
        stats.candidates += successors.candidates;
        stats.deduped_candidates += successors.nodes.len();
        *total_candidates += successors.candidates;

        if *total_candidates > max_candidates {
            print_layer_summary(
                side_name,
                layer_depth,
                current_layer_len,
                &stats,
                *total_visited,
            );
            return LayerOutcome {
                terminal: Some(GraphSearchResult::Capped {
                    reason: "Candidate cap hit",
                    visited: *total_visited,
                    candidates: *total_candidates,
                    elapsed: started.elapsed(),
                }),
                stats,
            };
        }

        for successor in &successors.nodes {
            *stats.family_counts.entry(successor.family).or_default() += 1;

            if successor.matrix.max_entry() > max_entry {
                stats.pruned_by_entry += 1;
                continue;
            }
            if seen.contains_key(&successor.matrix) {
                stats.collisions_with_seen += 1;
                continue;
            }

            let next_depth = layer_depth + 1;
            parent.insert(
                successor.matrix.clone(),
                ParentStep {
                    parent: current.clone(),
                    family: successor.family,
                },
            );

            let in_other_seen = other_seen.get(&successor.matrix);
            if let Some(&other_depth) = in_other_seen {
                let depth = next_depth + other_depth;
                if depth <= max_depth {
                    let path = if expand_forward {
                        reconstruct_path(&successor.matrix, parent, other_parent)
                    } else {
                        reconstruct_path(&successor.matrix, other_parent, parent)
                    };
                    let path_record = FoundGraphPath {
                        meeting: successor.matrix.clone(),
                        path,
                        discovered_from: side_name,
                        forward_depth: if expand_forward {
                            next_depth
                        } else {
                            other_depth
                        },
                        backward_depth: if expand_forward {
                            other_depth
                        } else {
                            next_depth
                        },
                    };
                    if continue_through_found_layer {
                        if best_found_depth.is_none_or(|best| depth < best) {
                            *best_found_depth = Some(depth);
                            found_paths.clear();
                        }
                        if Some(depth) == *best_found_depth {
                            found_paths.push(path_record);
                        }
                    } else {
                        print_layer_summary(
                            side_name,
                            layer_depth,
                            current_layer_len,
                            &stats,
                            *total_visited,
                        );
                        return LayerOutcome {
                            terminal: Some(GraphSearchResult::Found {
                                depth,
                                paths: vec![path_record],
                                visited: *total_visited,
                                candidates: *total_candidates,
                                elapsed: started.elapsed(),
                            }),
                            stats,
                        };
                    }
                }
            }

            seen.insert(successor.matrix.clone(), next_depth);
            next_frontier.push(successor.matrix.clone());
            stats.discovered += 1;
            if in_other_seen.is_none() {
                *total_visited += 1;
            }

            if *total_visited > max_states {
                return LayerOutcome {
                    terminal: Some(GraphSearchResult::Capped {
                        reason: "State cap hit",
                        visited: *total_visited,
                        candidates: *total_candidates,
                        elapsed: started.elapsed(),
                    }),
                    stats,
                };
            }
        }
    }

    let phase_seq = seq_start.elapsed();
    let layer_total = layer_start.elapsed();

    print_layer_summary(
        side_name,
        layer_depth,
        current_layer_len,
        &stats,
        *total_visited,
    );
    println!(
        "  timing: total={:.3}s, prep={:.3}ms, cache_check={:.3}ms, compute={:.3}s, seq={:.3}s",
        layer_total.as_secs_f64(),
        phase_prep.as_secs_f64() * 1000.0,
        phase_cache_check.as_secs_f64() * 1000.0,
        phase_compute.as_secs_f64(),
        phase_seq.as_secs_f64(),
    );

    if continue_through_found_layer && !found_paths.is_empty() {
        return LayerOutcome {
            terminal: Some(GraphSearchResult::Found {
                depth: (*best_found_depth).expect("found paths should set a best depth"),
                paths: std::mem::take(found_paths),
                visited: *total_visited,
                candidates: *total_candidates,
                elapsed: started.elapsed(),
            }),
            stats,
        };
    }

    frontier.extend(next_frontier);
    LayerOutcome {
        terminal: None,
        stats,
    }
}

/// Pre-expand the backward side by `seed_depth` graph-move steps without any
/// `max_entry` filtering. This lets the main bounded search use a small
/// `max_entry` even when the literal target has large entries: the seed
/// neighborhood absorbs the high-entry transitions near the target, and the
/// main loop continues from a frontier of states that may already satisfy the
/// bound.
fn seed_backward_neighborhood(
    seed_depth: usize,
    max_dim: usize,
    bwd_seen: &mut HashMap<DynMatrix, usize>,
    bwd_parent: &mut HashMap<DynMatrix, ParentStep>,
    bwd_frontier: &mut VecDeque<DynMatrix>,
    total_visited: &mut usize,
) {
    // bwd_frontier currently holds [target] at depth 0. Expand it BFS-style up
    // to seed_depth without entry pruning, leaving bwd_frontier holding the
    // states at the deepest expanded level.
    let mut current_layer: Vec<DynMatrix> = bwd_frontier.drain(..).collect();
    for layer_depth in 0..seed_depth {
        if current_layer.is_empty() {
            break;
        }
        let computed: Vec<(DynMatrix, GraphMoveSuccessors)> = current_layer
            .par_iter()
            .map(|m| (m.clone(), enumerate_graph_move_successors(m, max_dim)))
            .collect();
        let mut next_layer: Vec<DynMatrix> = Vec::new();
        for (current, successors) in computed {
            for successor in &successors.nodes {
                if bwd_seen.contains_key(&successor.matrix) {
                    continue;
                }
                bwd_parent.insert(
                    successor.matrix.clone(),
                    ParentStep {
                        parent: current.clone(),
                        family: successor.family,
                    },
                );
                bwd_seen.insert(successor.matrix.clone(), layer_depth + 1);
                *total_visited += 1;
                next_layer.push(successor.matrix.clone());
            }
        }
        current_layer = next_layer;
    }
    bwd_frontier.extend(current_layer);
}

fn print_layer_summary(
    side_name: &str,
    layer_depth: usize,
    frontier: usize,
    stats: &LayerStats,
    visited: usize,
) {
    println!(
        "{side_name} depth {layer_depth}: frontier={frontier}, candidates={}, deduped={}, pruned_by_entry={}, collisions={}, discovered={}, cache_hits={}, cache_misses={}, visited={}",
        stats.candidates,
        stats.deduped_candidates,
        stats.pruned_by_entry,
        stats.collisions_with_seen,
        stats.discovered,
        stats.successor_cache_hits,
        stats.successor_cache_misses,
        visited,
    );

    if !stats.family_counts.is_empty() {
        let families = stats
            .family_counts
            .iter()
            .map(|(family, count)| format!("{family}={count}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  families: {families}");
    }
}

fn choose_next_layer(
    fwd_depth: Option<usize>,
    bwd_depth: Option<usize>,
    fwd_frontier_len: usize,
    bwd_frontier_len: usize,
    fwd_candidates_per_node: f64,
    bwd_candidates_per_node: f64,
    fwd_cost_sample_nodes: usize,
    bwd_cost_sample_nodes: usize,
) -> Option<(bool, usize)> {
    match (fwd_depth, bwd_depth) {
        (Some(fwd), Some(bwd)) => {
            if fwd < bwd {
                Some((true, fwd))
            } else if bwd < fwd {
                Some((false, bwd))
            } else {
                let fwd_cost_ready = fwd_cost_sample_nodes >= 8;
                let bwd_cost_ready = bwd_cost_sample_nodes >= 8;
                if fwd_cost_ready && bwd_cost_ready {
                    let fwd_estimated_work =
                        fwd_frontier_len as f64 * fwd_candidates_per_node.max(1.0);
                    let bwd_estimated_work =
                        bwd_frontier_len as f64 * bwd_candidates_per_node.max(1.0);
                    Some((fwd_estimated_work <= bwd_estimated_work, fwd))
                } else {
                    Some((fwd_frontier_len <= bwd_frontier_len, fwd))
                }
            }
        }
        (Some(fwd), None) => Some((true, fwd)),
        (None, Some(bwd)) => Some((false, bwd)),
        (None, None) => None,
    }
}

fn reconstruct_path(
    meeting: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, ParentStep>,
    bwd_parent: &HashMap<DynMatrix, ParentStep>,
) -> Vec<PathStep> {
    let mut left = Vec::new();
    let mut current = meeting.clone();
    while let Some(step) = fwd_parent.get(&current) {
        left.push(PathStep {
            family: step.family,
            from: step.parent.clone(),
            to: current.clone(),
        });
        current = step.parent.clone();
    }
    left.reverse();

    let mut right = Vec::new();
    let mut current = meeting.clone();
    while let Some(step) = bwd_parent.get(&current) {
        right.push(PathStep {
            family: reverse_family(step.family),
            from: current.clone(),
            to: step.parent.clone(),
        });
        current = step.parent.clone();
    }

    left.extend(right);
    left
}

fn reverse_family(family: &'static str) -> &'static str {
    match family {
        "outsplit" => "out_amalgamation",
        "insplit" => "in_amalgamation",
        "out_amalgamation" => "outsplit",
        "in_amalgamation" => "insplit",
        _ => family,
    }
}

fn print_path(path: &[PathStep]) {
    if path.is_empty() {
        println!("empty path");
        return;
    }

    for (idx, step) in path.iter().enumerate() {
        println!(
            "{}. {}: {}x{} {:?} -> {}x{} {:?}",
            idx + 1,
            step.family,
            step.from.rows,
            step.from.cols,
            step.from.data,
            step.to.rows,
            step.to.cols,
            step.to.data
        );
    }
}

#[derive(Clone, Copy)]
struct GraphSearchRunConfig {
    k: u32,
    max_depth: usize,
    max_dim: usize,
    max_entry: u32,
    max_states: usize,
    max_candidates: usize,
    max_seconds: u64,
    use_cache: bool,
    seed_depth: usize,
    continue_through_found_layer: bool,
}

struct GraphPathSqliteRecorder {
    conn: Connection,
    run_id: i64,
    matrix_ids: HashMap<String, i64>,
}

impl GraphPathSqliteRecorder {
    fn new(path: impl AsRef<Path>) -> Result<Self, String> {
        let conn = Connection::open(path.as_ref())
            .map_err(|err| format!("failed to open {}: {err}", path.as_ref().display()))?;
        conn.busy_timeout(Duration::from_secs(30))
            .map_err(|err| format!("failed to configure sqlite busy timeout: {err}"))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|err| format!("failed to configure sqlite pragmas: {err}"))?;
        initialise_sqlite_schema(&conn)?;
        Ok(Self {
            conn,
            run_id: 0,
            matrix_ids: HashMap::new(),
        })
    }

    fn start_run(
        &mut self,
        start: &DynMatrix,
        target: &DynMatrix,
        config: &GraphSearchRunConfig,
    ) -> Result<(), String> {
        let start_id = self.ensure_matrix_id(start)?;
        let target_id = self.ensure_matrix_id(target)?;
        self.conn
            .execute(
                "INSERT INTO graph_path_runs (
                    started_unix_ms,
                    k,
                    source_matrix_id,
                    target_matrix_id,
                    max_depth,
                    max_dim,
                    max_entry,
                    max_states,
                    max_candidates,
                    max_seconds,
                    use_cache,
                    seed_depth,
                    continue_through_found_layer
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    unix_timestamp_ms(),
                    config.k as i64,
                    start_id,
                    target_id,
                    config.max_depth as i64,
                    config.max_dim as i64,
                    config.max_entry as i64,
                    config.max_states as i64,
                    config.max_candidates as i64,
                    config.max_seconds as i64,
                    config.use_cache as i64,
                    config.seed_depth as i64,
                    config.continue_through_found_layer as i64,
                ],
            )
            .map_err(|err| format!("failed to insert graph_path_runs row: {err}"))?;
        self.run_id = self.conn.last_insert_rowid();
        Ok(())
    }

    fn finish_run(&mut self, result: &GraphSearchResult) -> Result<(), String> {
        match result {
            GraphSearchResult::Found {
                depth,
                paths,
                visited,
                candidates,
                elapsed,
            } => {
                let path_rows = paths
                    .iter()
                    .enumerate()
                    .map(|(ordinal, path)| {
                        let meeting_id = self.ensure_matrix_id(&path.meeting)?;
                        let signature = path_signature(&path.path);
                        let steps = path
                            .path
                            .iter()
                            .map(|step| {
                                Ok((
                                    step.family,
                                    self.ensure_matrix_id(&step.from)?,
                                    self.ensure_matrix_id(&step.to)?,
                                ))
                            })
                            .collect::<Result<Vec<_>, String>>()?;
                        Ok((ordinal, path, meeting_id, signature, steps))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let tx = self
                    .conn
                    .transaction()
                    .map_err(|err| format!("failed to start sqlite transaction: {err}"))?;
                tx.execute(
                    "UPDATE graph_path_runs
                     SET finished_unix_ms = ?1,
                         outcome = ?2,
                         found_depth = ?3,
                         found_path_count = ?4,
                         visited_states = ?5,
                         candidates_generated = ?6,
                         elapsed_ms = ?7
                     WHERE id = ?8",
                    params![
                        unix_timestamp_ms(),
                        "found",
                        *depth as i64,
                        paths.len() as i64,
                        *visited as i64,
                        *candidates as i64,
                        elapsed.as_millis() as i64,
                        self.run_id,
                    ],
                )
                .map_err(|err| format!("failed to update graph_path_runs row: {err}"))?;
                for (ordinal, path, meeting_id, signature, steps) in path_rows {
                    tx.execute(
                        "INSERT INTO graph_path_results (
                            run_id,
                            ordinal,
                            depth,
                            meeting_matrix_id,
                            discovered_from,
                            forward_depth,
                            backward_depth,
                            step_count,
                            path_signature
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        params![
                            self.run_id,
                            ordinal as i64,
                            *depth as i64,
                            meeting_id,
                            path.discovered_from,
                            path.forward_depth as i64,
                            path.backward_depth as i64,
                            path.path.len() as i64,
                            signature,
                        ],
                    )
                    .map_err(|err| format!("failed to insert graph_path_results row: {err}"))?;
                    let result_id = tx.last_insert_rowid();
                    for (step_index, (family, from_id, to_id)) in steps.into_iter().enumerate() {
                        tx.execute(
                            "INSERT INTO graph_path_steps (
                                result_id,
                                step_index,
                                family,
                                from_matrix_id,
                                to_matrix_id
                            ) VALUES (?1, ?2, ?3, ?4, ?5)",
                            params![result_id, step_index as i64, family, from_id, to_id],
                        )
                        .map_err(|err| format!("failed to insert graph_path_steps row: {err}"))?;
                    }
                }
                tx.commit()
                    .map_err(|err| format!("failed to commit sqlite transaction: {err}"))?;
            }
            GraphSearchResult::NotFound {
                visited,
                candidates,
                elapsed,
            } => {
                self.finish_nonfound("not_found", None, *visited, *candidates, *elapsed)?;
            }
            GraphSearchResult::Capped {
                reason,
                visited,
                candidates,
                elapsed,
            } => {
                self.finish_nonfound("capped", Some(reason), *visited, *candidates, *elapsed)?;
            }
        }
        Ok(())
    }

    fn finish_nonfound(
        &self,
        outcome: &str,
        reason: Option<&str>,
        visited: usize,
        candidates: usize,
        elapsed: Duration,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE graph_path_runs
                 SET finished_unix_ms = ?1,
                     outcome = ?2,
                     reason = ?3,
                     visited_states = ?4,
                     candidates_generated = ?5,
                     elapsed_ms = ?6
                 WHERE id = ?7",
                params![
                    unix_timestamp_ms(),
                    outcome,
                    reason,
                    visited as i64,
                    candidates as i64,
                    elapsed.as_millis() as i64,
                    self.run_id,
                ],
            )
            .map_err(|err| format!("failed to update graph_path_runs row: {err}"))?;
        Ok(())
    }

    fn ensure_matrix_id(&mut self, matrix: &DynMatrix) -> Result<i64, String> {
        let key = matrix_key(matrix);
        if let Some(&id) = self.matrix_ids.get(&key) {
            return Ok(id);
        }
        self.conn
            .execute(
                "INSERT INTO matrices (
                    matrix_key,
                    rows,
                    cols,
                    data_json,
                    entry_sum,
                    max_entry,
                    trace
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(matrix_key) DO NOTHING",
                params![
                    key,
                    matrix.rows as i64,
                    matrix.cols as i64,
                    matrix_json(matrix)?,
                    matrix.entry_sum() as i64,
                    matrix.max_entry() as i64,
                    if matrix.is_square() {
                        Some(matrix.trace() as i64)
                    } else {
                        None
                    },
                ],
            )
            .map_err(|err| format!("failed to insert matrix row: {err}"))?;
        let id = self
            .conn
            .query_row(
                "SELECT id FROM matrices WHERE matrix_key = ?1",
                params![key],
                |row| row.get(0),
            )
            .map_err(|err| format!("failed to load matrix id: {err}"))?;
        self.matrix_ids.insert(key, id);
        Ok(id)
    }
}

fn initialise_sqlite_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS matrices (
            id INTEGER PRIMARY KEY,
            matrix_key TEXT NOT NULL UNIQUE,
            rows INTEGER NOT NULL,
            cols INTEGER NOT NULL,
            data_json TEXT NOT NULL,
            entry_sum INTEGER NOT NULL,
            max_entry INTEGER NOT NULL,
            trace INTEGER
        );
        CREATE TABLE IF NOT EXISTS graph_path_runs (
            id INTEGER PRIMARY KEY,
            started_unix_ms INTEGER NOT NULL,
            finished_unix_ms INTEGER,
            k INTEGER NOT NULL,
            source_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            target_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            max_depth INTEGER NOT NULL,
            max_dim INTEGER NOT NULL,
            max_entry INTEGER NOT NULL,
            max_states INTEGER NOT NULL,
            max_candidates INTEGER NOT NULL,
            max_seconds INTEGER NOT NULL,
            use_cache INTEGER NOT NULL,
            seed_depth INTEGER NOT NULL,
            continue_through_found_layer INTEGER NOT NULL,
            outcome TEXT,
            reason TEXT,
            found_depth INTEGER,
            found_path_count INTEGER,
            visited_states INTEGER,
            candidates_generated INTEGER,
            elapsed_ms INTEGER
        );
        CREATE TABLE IF NOT EXISTS graph_path_results (
            id INTEGER PRIMARY KEY,
            run_id INTEGER NOT NULL REFERENCES graph_path_runs(id) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL,
            depth INTEGER NOT NULL,
            meeting_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            discovered_from TEXT NOT NULL,
            forward_depth INTEGER NOT NULL,
            backward_depth INTEGER NOT NULL,
            step_count INTEGER NOT NULL,
            path_signature TEXT NOT NULL,
            UNIQUE(run_id, path_signature)
        );
        CREATE TABLE IF NOT EXISTS graph_path_steps (
            id INTEGER PRIMARY KEY,
            result_id INTEGER NOT NULL REFERENCES graph_path_results(id) ON DELETE CASCADE,
            step_index INTEGER NOT NULL,
            family TEXT NOT NULL,
            from_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            to_matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            UNIQUE(result_id, step_index)
        );
        CREATE INDEX IF NOT EXISTS idx_graph_path_results_run ON graph_path_results(run_id, ordinal);
        CREATE INDEX IF NOT EXISTS idx_graph_path_steps_result ON graph_path_steps(result_id, step_index);",
    )
    .map_err(|err| format!("failed to initialise sqlite schema: {err}"))?;
    Ok(())
}

fn matrix_key(matrix: &DynMatrix) -> String {
    let mut key = format!("{}x{}:", matrix.rows, matrix.cols);
    for (idx, value) in matrix.data.iter().enumerate() {
        if idx > 0 {
            key.push(',');
        }
        key.push_str(&value.to_string());
    }
    key
}

fn matrix_json(matrix: &DynMatrix) -> Result<String, String> {
    let rows: Vec<Vec<u32>> = (0..matrix.rows)
        .map(|row| {
            (0..matrix.cols)
                .map(|col| matrix.get(row, col))
                .collect::<Vec<_>>()
        })
        .collect();
    serde_json::to_string(&rows).map_err(|err| format!("failed to serialise matrix: {err}"))
}

fn path_signature(path: &[PathStep]) -> String {
    let mut signature = String::new();
    for (idx, step) in path.iter().enumerate() {
        if idx > 0 {
            signature.push('|');
        }
        signature.push_str(step.family);
        signature.push(':');
        signature.push_str(&matrix_key(&step.from));
        signature.push_str("->");
        signature.push_str(&matrix_key(&step.to));
    }
    signature
}

fn unix_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
