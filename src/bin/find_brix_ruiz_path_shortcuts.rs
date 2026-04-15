// Retired from Cargo targets in RFC Phase 6.
// Kept in-tree as a historical reference for older research notes.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use sse_core::factorisation::visit_factorisations_with_family_for_policy;
use sse_core::graph_moves::enumerate_graph_move_successors;
use sse_core::matrix::DynMatrix;
use sse_core::types::{EsseStep, MoveFamilyPolicy};

use rayon::prelude::*;
use rusqlite::{params, Connection};

fn main() {
    let mut max_shortcut_lag = 6usize;
    let mut max_dim = 5usize;
    let mut max_entry = 6u32;
    let mut min_gap = 2usize;
    let mut max_gap = usize::MAX;
    let mut refine_rounds = 1usize;
    let mut search_mode = MoveFamilyPolicy::Mixed;
    let mut segment_timeout: Option<Duration> = None;
    let mut paths_db: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-shortcut-lag" => {
                max_shortcut_lag = args
                    .next()
                    .expect("--max-shortcut-lag requires a value")
                    .parse()
                    .expect("invalid max shortcut lag");
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
            "--min-gap" => {
                min_gap = args
                    .next()
                    .expect("--min-gap requires a value")
                    .parse()
                    .expect("invalid min gap");
            }
            "--max-gap" => {
                max_gap = args
                    .next()
                    .expect("--max-gap requires a value")
                    .parse()
                    .expect("invalid max gap");
            }
            "--refine-rounds" => {
                refine_rounds = args
                    .next()
                    .expect("--refine-rounds requires a value")
                    .parse()
                    .expect("invalid refine rounds");
            }
            "--search-mode" => {
                let value = args.next().expect("--search-mode requires a value");
                search_mode = match value.as_str() {
                    "mixed" => MoveFamilyPolicy::Mixed,
                    "graph-plus-structured" | "graph_plus_structured" => {
                        MoveFamilyPolicy::GraphPlusStructured
                    }
                    "graph-only" | "graph_only" => MoveFamilyPolicy::GraphOnly,
                    _ => panic!("unknown search mode: {value}"),
                };
            }
            "--segment-timeout" => {
                let secs: u64 = args
                    .next()
                    .expect("--segment-timeout requires a value (seconds)")
                    .parse()
                    .expect("invalid segment timeout");
                segment_timeout = Some(Duration::from_secs(secs));
            }
            "--paths-db" | "--sqlite" => {
                paths_db = Some(args.next().expect("--paths-db requires a value"));
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_brix_ruiz_path_shortcuts [--max-shortcut-lag N] [--max-dim N] [--max-entry N] [--min-gap N] [--max-gap N] [--segment-timeout SECS] [--refine-rounds N] [--search-mode mixed|graph-plus-structured|graph-only] [--paths-db PATH]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    println!("Brix-Ruiz k=3 shortcut search along guide paths");
    println!(
        "config: search_mode={:?}, max_shortcut_lag={}, max_dim={}, max_entry={}, min_gap={}, max_gap={}, segment_timeout={}, refine_rounds={}, paths_db={}",
        search_mode,
        max_shortcut_lag,
        max_dim,
        max_entry,
        min_gap,
        max_gap,
        segment_timeout.map_or("none".to_string(), |d| format!("{}s", d.as_secs())),
        refine_rounds,
        paths_db.as_deref().unwrap_or("none")
    );

    let mut guides = vec![GuidePath {
        label: "hardcoded endpoint 16-path".to_string(),
        source_kind: "hardcoded".to_string(),
        source_path_signature: matrix_path_signature(&endpoint_16_path()),
        matrices: endpoint_16_path(),
    }];
    if let Some(db_path) = paths_db.as_deref() {
        let loaded = load_guides_from_sqlite(db_path).unwrap_or_else(|err| panic!("{err}"));
        guides.extend(loaded);
    }
    guides = deduplicate_guides(guides);

    println!("loaded guide paths = {}", guides.len());

    let mut recorder = paths_db
        .as_deref()
        .map(ShortcutPathSqliteRecorder::new)
        .transpose()
        .unwrap_or_else(|err| panic!("{err}"));
    if let Some(recorder) = recorder.as_mut() {
        recorder
            .start_run(&ShortcutRunConfig {
                k: 3,
                max_shortcut_lag,
                max_dim,
                max_entry,
                min_gap,
                max_gap,
                refine_rounds,
                search_mode,
                guide_count: guides.len(),
            })
            .unwrap_or_else(|err| panic!("{err}"));
    }

    let mut results = Vec::new();
    for guide in guides {
        println!();
        println!("=== guide: {} ({}) ===", guide.label, guide.source_kind);
        let mut current_guide = guide.matrices.clone();
        let initial_lag = current_guide.len().saturating_sub(1);
        let mut last_round = 0usize;
        let mut best_route = current_guide.clone();

        for round in 0..refine_rounds {
            println!();
            println!("=== refinement round {} ===", round + 1);
            let outcome = search_guide_path(
                &current_guide,
                max_shortcut_lag,
                max_dim,
                max_entry,
                min_gap,
                max_gap,
                segment_timeout,
                search_mode,
            );

            let next_guide = stitch_route(&outcome.best);
            last_round = round;
            best_route = next_guide.clone();
            if next_guide.len() == current_guide.len() {
                println!();
                println!("refinement stalled at {} moves", current_guide.len() - 1);
                break;
            }
            println!();
            println!("refined guide to {} moves", next_guide.len() - 1);
            current_guide = next_guide;
        }

        let final_lag = best_route.len().saturating_sub(1);
        let result = ShortcutGuideResult {
            guide_label: guide.label,
            source_kind: guide.source_kind,
            source_path_signature: guide.source_path_signature,
            final_round_index: last_round,
            initial_lag,
            final_lag,
            improved: final_lag < initial_lag,
            final_route: best_route,
        };
        if let Some(recorder) = recorder.as_mut() {
            recorder
                .record_result(&result)
                .unwrap_or_else(|err| panic!("{err}"));
        }
        results.push(result);
    }

    if let Some(recorder) = recorder.as_mut() {
        recorder
            .finish_run(&results)
            .unwrap_or_else(|err| panic!("{err}"));
    }

    println!();
    println!("Shortcut summary:");
    for result in &results {
        println!(
            "  {} [{}]: {} -> {} moves{}",
            result.guide_label,
            result.source_kind,
            result.initial_lag,
            result.final_lag,
            if result.improved { " improved" } else { "" }
        );
    }
}

#[derive(Clone)]
struct GuidePath {
    label: String,
    source_kind: String,
    source_path_signature: String,
    matrices: Vec<DynMatrix>,
}

#[derive(Clone)]
struct ShortcutPath {
    matrices: Vec<DynMatrix>,
    steps: Vec<EsseStep>,
}

#[derive(Clone)]
struct ShortcutEdge {
    from: usize,
    to: usize,
    lag: usize,
    path: ShortcutPath,
    kind: &'static str,
}

struct FoundPath {
    matrices: Vec<DynMatrix>,
    steps: Vec<EsseStep>,
    visited: usize,
}

enum SearchResult {
    Found(FoundPath),
    NotFound { visited: usize },
    TimedOut { visited: usize },
}

struct GuideSearchOutcome {
    best: Vec<ShortcutEdge>,
}

struct ShortcutGuideResult {
    guide_label: String,
    source_kind: String,
    source_path_signature: String,
    final_round_index: usize,
    initial_lag: usize,
    final_lag: usize,
    improved: bool,
    final_route: Vec<DynMatrix>,
}

#[derive(Clone)]
struct FrontierExpansion {
    parent_canon: DynMatrix,
    next_canon: DynMatrix,
    next_orig: DynMatrix,
    step: EsseStep,
}

fn search_between_waypoints(
    start: &DynMatrix,
    target: &DynMatrix,
    max_lag: usize,
    max_dim: usize,
    max_entry: u32,
    segment_timeout: Option<Duration>,
    search_mode: MoveFamilyPolicy,
) -> SearchResult {
    let start_canon = start.canonical_perm();
    let target_canon = target.canonical_perm();
    if start_canon == target_canon {
        return SearchResult::Found(FoundPath {
            matrices: vec![start.clone(), target.clone()],
            steps: if start == target {
                Vec::new()
            } else {
                vec![permutation_step_between(start, target)
                    .expect("permutation-similar endpoints should have a step")]
            },
            visited: 1,
        });
    }

    let source_trace = start.trace();
    let source_trace_square = trace_square(start);

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(start_canon.clone(), None);
    fwd_depths.insert(start_canon.clone(), 0);
    fwd_orig.insert(start_canon.clone(), start.clone());
    fwd_frontier.push_back(start_canon.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(target_canon.clone(), None);
    bwd_depths.insert(target_canon.clone(), 0);
    bwd_orig.insert(target_canon.clone(), target.clone());
    bwd_frontier.push_back(target_canon);

    let deadline = segment_timeout.map(|d| Instant::now() + d);

    for _layer in 0..max_lag {
        if let Some(dl) = deadline {
            if Instant::now() >= dl {
                return SearchResult::TimedOut {
                    visited: visited_union_size(&fwd_parent, &bwd_parent),
                };
            }
        }

        let expand_forward = fwd_frontier.len() <= bwd_frontier.len();
        let (frontier, parent, depths, orig, other_depths, other_orig, other_parent) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &bwd_depths,
                    &bwd_orig,
                    &bwd_parent,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &fwd_depths,
                    &fwd_orig,
                    &fwd_parent,
                )
            };

        let Some(layer_depth) = frontier.front().and_then(|node| depths.get(node)).copied() else {
            break;
        };
        if layer_depth >= max_lag {
            break;
        }

        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let next_depth = layer_depth + 1;
        let mut next_frontier = VecDeque::new();
        let mut timed_out = false;

        const CHUNK_SIZE: usize = 256;
        for chunk in current_frontier.chunks(CHUNK_SIZE) {
            if let Some(dl) = deadline {
                if Instant::now() >= dl {
                    timed_out = true;
                    break;
                }
            }

            let expansions = expand_frontier(
                chunk,
                orig,
                max_dim,
                max_entry,
                search_mode,
                source_trace,
                source_trace_square,
            );

            for expansion in expansions {
                if parent.contains_key(&expansion.next_canon) {
                    continue;
                }

                parent.insert(
                    expansion.next_canon.clone(),
                    Some((expansion.parent_canon.clone(), expansion.step)),
                );
                depths.insert(expansion.next_canon.clone(), next_depth);
                orig.insert(expansion.next_canon.clone(), expansion.next_orig);

                if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                    let total_lag = next_depth + other_depth;
                    if total_lag <= max_lag {
                        let meeting = expansion.next_canon;
                        let path = if expand_forward {
                            reconstruct_path(
                                start,
                                target,
                                &meeting,
                                &fwd_parent,
                                &fwd_orig,
                                &bwd_parent,
                                &bwd_orig,
                            )
                        } else {
                            reconstruct_path(
                                start,
                                target,
                                &meeting,
                                other_parent,
                                other_orig,
                                parent,
                                orig,
                            )
                        };
                        let visited = visited_union_size(&fwd_parent, &bwd_parent);
                        return SearchResult::Found(FoundPath {
                            matrices: path.matrices,
                            steps: path.steps,
                            visited,
                        });
                    }
                }

                next_frontier.push_back(expansion.next_canon);
            }
        }

        if timed_out {
            return SearchResult::TimedOut {
                visited: visited_union_size(&fwd_parent, &bwd_parent),
            };
        }

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
    }

    SearchResult::NotFound {
        visited: visited_union_size(&fwd_parent, &bwd_parent),
    }
}

fn search_guide_path(
    guide: &[DynMatrix],
    max_shortcut_lag: usize,
    max_dim: usize,
    max_entry: u32,
    min_gap: usize,
    max_gap: usize,
    segment_timeout: Option<Duration>,
    search_mode: MoveFamilyPolicy,
) -> GuideSearchOutcome {
    println!("guide moves = {}", guide.len() - 1);
    println!();

    let mut edges = Vec::new();
    for idx in 0..guide.len() - 1 {
        edges.push(ShortcutEdge {
            from: idx,
            to: idx + 1,
            lag: 1,
            path: ShortcutPath {
                matrices: vec![guide[idx].clone(), guide[idx + 1].clone()],
                steps: Vec::new(),
            },
            kind: "guide",
        });
    }

    for start in 0..guide.len() - 1 {
        for end in start + min_gap..guide.len() {
            let gap = end - start;
            if gap > max_gap {
                break;
            }

            let lag_cap = max_shortcut_lag.min(gap - 1);
            if lag_cap == 0 {
                continue;
            }

            println!(
                "segment {start:>2}->{end:>2} gap={gap:>2} lag_cap={lag_cap:>2} dims={}x{} -> {}x{}",
                guide[start].rows,
                guide[start].cols,
                guide[end].rows,
                guide[end].cols
            );

            let result = search_between_waypoints(
                &guide[start],
                &guide[end],
                lag_cap,
                max_dim,
                max_entry,
                segment_timeout,
                search_mode,
            );
            match result {
                SearchResult::Found(path) => {
                    let lag = path.steps.len();
                    if lag < gap {
                        println!(
                            "  found shortcut: lag={} matrices={} visited={}",
                            lag,
                            path.matrices.len(),
                            path.visited
                        );
                        edges.push(ShortcutEdge {
                            from: start,
                            to: end,
                            lag,
                            path: ShortcutPath {
                                matrices: path.matrices,
                                steps: path.steps,
                            },
                            kind: "shortcut",
                        });
                    } else {
                        println!(
                            "  found non-improving path: lag={} gap={} matrices={} visited={}",
                            lag,
                            gap,
                            path.matrices.len(),
                            path.visited
                        );
                    }
                }
                SearchResult::NotFound { visited } => {
                    println!("  no shortcut found within lag cap; visited={visited}");
                }
                SearchResult::TimedOut { visited } => {
                    println!("  timed out; visited={visited}");
                }
            }
        }
    }

    println!();
    println!("Discovered edges:");
    for edge in &edges {
        if edge.to - edge.from > 1 {
            println!(
                "  {} {:>2}->{:>2} gap={} lag={}",
                edge.kind,
                edge.from,
                edge.to,
                edge.to - edge.from,
                edge.lag
            );
        }
    }

    let best =
        shortest_index_path(guide.len(), &edges).expect("guide path should connect endpoints");
    println!();
    println!("Best route over guide indices:");
    for edge in &best {
        println!(
            "  {:>2}->{:>2} gap={} lag={} kind={} matrices={}",
            edge.from,
            edge.to,
            edge.to - edge.from,
            edge.lag,
            edge.kind,
            edge.path.matrices.len()
        );
    }
    let total_lag: usize = best.iter().map(|edge| edge.lag).sum();
    println!();
    println!("best total lag = {total_lag}");
    println!("best route matrices:");
    for (idx, matrix) in stitch_route(&best).iter().enumerate() {
        println!(
            "  {:>2}: {}x{} {}",
            idx,
            matrix.rows,
            matrix.cols,
            format_matrix(matrix)
        );
    }

    GuideSearchOutcome { best }
}

fn expand_frontier(
    frontier: &[DynMatrix],
    orig: &HashMap<DynMatrix, DynMatrix>,
    max_dim: usize,
    max_entry: u32,
    search_mode: MoveFamilyPolicy,
    source_trace: u64,
    source_trace_square: u64,
) -> Vec<FrontierExpansion> {
    let expand_node = |current_canon: &DynMatrix| {
        let current = orig
            .get(current_canon)
            .expect("frontier node should have an original matrix");
        let mut expansions = Vec::new();
        let mut seen_successors = HashSet::new();

        let graph_successors = enumerate_graph_move_successors(current, max_dim);
        for successor in graph_successors.nodes {
            let next = successor.orig_matrix;
            if !is_trace_consistent(&next, source_trace, source_trace_square) {
                continue;
            }
            if seen_successors.insert(successor.matrix.clone()) {
                expansions.push(FrontierExpansion {
                    parent_canon: current_canon.clone(),
                    next_canon: successor.matrix,
                    next_orig: next,
                    step: successor.step,
                });
            }
        }

        if search_mode.permits_factorisations() {
            visit_factorisations_with_family_for_policy(
                current,
                max_dim,
                max_entry,
                search_mode,
                |_family, u, v| {
                    let next = v.mul(&u);
                    if next.rows > max_dim {
                        return;
                    }
                    if !is_trace_consistent(&next, source_trace, source_trace_square) {
                        return;
                    }
                    let next_canon = next.canonical_perm();
                    if seen_successors.insert(next_canon.clone()) {
                        expansions.push(FrontierExpansion {
                            parent_canon: current_canon.clone(),
                            next_canon,
                            next_orig: next,
                            step: EsseStep { u, v },
                        });
                    }
                },
            );
        }

        expansions
    };

    let per_node: Vec<Vec<FrontierExpansion>> = frontier.par_iter().map(expand_node).collect();
    deduplicate_expansions(per_node.into_iter().flatten().collect())
}

fn deduplicate_expansions(expansions: Vec<FrontierExpansion>) -> Vec<FrontierExpansion> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(expansions.len());
    for expansion in expansions {
        if seen.insert(expansion.next_canon.clone()) {
            deduped.push(expansion);
        }
    }
    deduped
}

fn is_trace_consistent(candidate: &DynMatrix, source_trace: u64, source_trace_square: u64) -> bool {
    candidate.trace() == source_trace && trace_square(candidate) == source_trace_square
}

fn trace_square(m: &DynMatrix) -> u64 {
    m.mul(m).trace()
}

fn walk_parent_chain(
    node: &DynMatrix,
    parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    orig: &HashMap<DynMatrix, DynMatrix>,
) -> (Vec<DynMatrix>, Vec<EsseStep>) {
    let mut matrices = Vec::new();
    let mut steps = Vec::new();
    let mut current = node.clone();

    matrices.push(orig[&current].clone());
    while let Some(Some((prev, step))) = parent.get(&current) {
        steps.push(step.clone());
        matrices.push(orig[prev].clone());
        current = prev.clone();
    }

    matrices.reverse();
    steps.reverse();
    (matrices, steps)
}

fn reconstruct_path(
    start: &DynMatrix,
    target: &DynMatrix,
    meeting_canon: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    fwd_orig: &HashMap<DynMatrix, DynMatrix>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_orig: &HashMap<DynMatrix, DynMatrix>,
) -> ShortcutPath {
    let (fwd_matrices, fwd_steps) = walk_parent_chain(meeting_canon, fwd_parent, fwd_orig);
    let (bwd_matrices, bwd_steps) = walk_parent_chain(meeting_canon, bwd_parent, bwd_orig);

    let fwd_meeting = fwd_matrices.last().expect("forward meeting").clone();
    let bwd_meeting = bwd_matrices.last().expect("backward meeting").clone();

    let mut all_steps = fwd_steps;
    if fwd_meeting != bwd_meeting {
        all_steps.push(
            permutation_step_between(&fwd_meeting, &bwd_meeting)
                .expect("meeting representatives should be permutation-similar"),
        );
    }
    for step in bwd_steps.into_iter().rev() {
        all_steps.push(EsseStep {
            u: step.v,
            v: step.u,
        });
    }

    let mut all_matrices = fwd_matrices;
    if fwd_meeting != bwd_meeting {
        all_matrices.push(bwd_meeting);
    }
    for matrix in bwd_matrices.into_iter().rev().skip(1) {
        all_matrices.push(matrix);
    }

    if all_matrices.first().expect("path has start") != start {
        all_steps.insert(
            0,
            permutation_step_between(start, all_matrices.first().expect("path has start"))
                .expect("start should be permutation-similar to canonical start"),
        );
        all_matrices.insert(0, start.clone());
    }

    if all_matrices.last().expect("path has end") != target {
        all_steps.push(
            permutation_step_between(all_matrices.last().expect("path has end"), target)
                .expect("end should be permutation-similar to target"),
        );
        all_matrices.push(target.clone());
    }

    ShortcutPath {
        matrices: all_matrices,
        steps: all_steps,
    }
}

fn permutation_step_between(from: &DynMatrix, to: &DynMatrix) -> Option<EsseStep> {
    if from.rows != from.cols || to.rows != to.cols || from.rows != to.rows {
        return None;
    }
    let n = from.rows;
    let mut perm: Vec<usize> = (0..n).collect();
    let mut result = None;
    for_each_permutation(&mut perm, 0, &mut |perm| {
        if result.is_some() {
            return;
        }
        let (p, pinv) = permutation_matrices(perm);
        let candidate = pinv.mul(from).mul(&p);
        if candidate == *to {
            result = Some(EsseStep {
                u: from.mul(&p),
                v: pinv,
            });
        }
    });
    result
}

fn permutation_matrices(perm: &[usize]) -> (DynMatrix, DynMatrix) {
    let n = perm.len();
    let mut p_data = vec![0u32; n * n];
    let mut pinv_data = vec![0u32; n * n];
    for (row, &col) in perm.iter().enumerate() {
        p_data[row * n + col] = 1;
        pinv_data[col * n + row] = 1;
    }
    (
        DynMatrix::new(n, n, p_data),
        DynMatrix::new(n, n, pinv_data),
    )
}

fn for_each_permutation<F>(perm: &mut [usize], start: usize, visit: &mut F)
where
    F: FnMut(&[usize]),
{
    if start == perm.len() {
        visit(perm);
        return;
    }
    for idx in start..perm.len() {
        perm.swap(start, idx);
        for_each_permutation(perm, start + 1, visit);
        perm.swap(start, idx);
    }
}

fn visited_union_size(
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
) -> usize {
    fwd_parent.len()
        + bwd_parent
            .keys()
            .filter(|key| !fwd_parent.contains_key(*key))
            .count()
}

fn shortest_index_path(edge_count: usize, edges: &[ShortcutEdge]) -> Option<Vec<ShortcutEdge>> {
    let mut best_cost = vec![usize::MAX; edge_count];
    let mut best_prev: Vec<Option<usize>> = vec![None; edge_count];
    let mut best_edge: Vec<Option<usize>> = vec![None; edge_count];
    best_cost[0] = 0;

    for node in 0..edge_count {
        if best_cost[node] == usize::MAX {
            continue;
        }
        for (idx, edge) in edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.from == node)
        {
            let candidate = best_cost[node] + edge.lag;
            if candidate < best_cost[edge.to] {
                best_cost[edge.to] = candidate;
                best_prev[edge.to] = Some(node);
                best_edge[edge.to] = Some(idx);
            }
        }
    }

    if best_cost[edge_count - 1] == usize::MAX {
        return None;
    }

    let mut route = Vec::new();
    let mut current = edge_count - 1;
    while current != 0 {
        let edge_idx = best_edge[current].expect("reachable node should have an incoming edge");
        route.push(edges[edge_idx].clone());
        current = best_prev[current].expect("reachable node should have a predecessor");
    }
    route.reverse();
    Some(route)
}

fn stitch_route(route: &[ShortcutEdge]) -> Vec<DynMatrix> {
    let mut matrices = Vec::new();
    for (idx, edge) in route.iter().enumerate() {
        if idx == 0 {
            matrices.extend(edge.path.matrices.iter().cloned());
        } else {
            matrices.extend(edge.path.matrices.iter().skip(1).cloned());
        }
    }
    matrices
}

fn format_matrix(matrix: &DynMatrix) -> String {
    let rows: Vec<String> = (0..matrix.rows)
        .map(|row| {
            let entries: Vec<String> = (0..matrix.cols)
                .map(|col| matrix.get(row, col).to_string())
                .collect();
            format!("[{}]", entries.join(","))
        })
        .collect();
    rows.join(" ")
}

fn mat(dim: usize, data: Vec<u32>) -> DynMatrix {
    DynMatrix::new(dim, dim, data).canonical_perm()
}

fn endpoint_16_path() -> Vec<DynMatrix> {
    vec![
        mat(2, vec![1, 2, 3, 1]),
        mat(3, vec![0, 0, 2, 1, 1, 1, 2, 2, 1]),
        mat(4, vec![0, 0, 1, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 2, 0, 1]),
        mat(
            5,
            vec![
                0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 2, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 0, 1, 2, 2, 0, 1, 1, 1, 1, 1, 1, 0]),
        mat(
            5,
            vec![
                0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 0, 2, 0, 2, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 1, 0, 2, 1, 1, 1, 1, 0, 2, 2, 2, 1]),
        mat(
            5,
            vec![
                0, 1, 0, 0, 1, 1, 0, 2, 0, 0, 1, 1, 0, 2, 1, 1, 0, 1, 1, 0, 1, 1, 0, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 2, 1, 2, 1, 1]),
        mat(
            5,
            vec![
                0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 1, 0, 2, 1, 1, 0, 0, 1, 1, 1, 1, 0, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 0, 1, 0, 1, 1, 2, 0, 1, 2, 2, 1, 1]),
        mat(
            5,
            vec![
                0, 1, 0, 0, 1, 1, 0, 1, 2, 0, 2, 1, 0, 2, 1, 0, 0, 1, 1, 0, 2, 1, 0, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 0, 1, 1, 0, 2, 2, 0, 1, 3, 4, 1, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 0, 1, 0, 1, 1, 0, 1, 3, 1, 4, 1,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 0, 4, 4, 1, 1]),
        mat(3, vec![0, 0, 2, 1, 1, 4, 1, 1, 1]),
        mat(2, vec![1, 1, 6, 1]),
    ]
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

fn matrix_path_signature(path: &[DynMatrix]) -> String {
    path.iter().map(matrix_key).collect::<Vec<_>>().join("|")
}

fn deduplicate_guides(guides: Vec<GuidePath>) -> Vec<GuidePath> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for guide in guides {
        if seen.insert(matrix_path_signature(&guide.matrices)) {
            deduped.push(guide);
        }
    }
    deduped
}

fn load_guides_from_sqlite(path: impl AsRef<Path>) -> Result<Vec<GuidePath>, String> {
    let conn = Connection::open(path.as_ref())
        .map_err(|err| format!("failed to open {}: {err}", path.as_ref().display()))?;
    let mut guides = load_graph_guides(&conn)?;
    guides.extend(load_shortcut_guides(&conn)?);
    Ok(guides)
}

fn load_graph_guides(conn: &Connection) -> Result<Vec<GuidePath>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                r.id,
                r.ordinal,
                r.path_signature,
                s.step_index,
                mf.data_json,
                mt.data_json
             FROM graph_path_results r
             JOIN graph_path_steps s ON s.result_id = r.id
             JOIN matrices mf ON mf.id = s.from_matrix_id
             JOIN matrices mt ON mt.id = s.to_matrix_id
             ORDER BY r.id, s.step_index",
        )
        .map_err(|err| format!("failed to prepare graph guide query: {err}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|err| format!("failed to query graph guides: {err}"))?;
    let mut guides = Vec::new();
    let mut current_result_id = None;
    let mut current_ordinal = 0i64;
    let mut current_signature = String::new();
    let mut current_matrices = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read graph guide row: {err}"))?
    {
        let result_id: i64 = row
            .get(0)
            .map_err(|err| format!("bad graph result id: {err}"))?;
        let ordinal: i64 = row
            .get(1)
            .map_err(|err| format!("bad graph ordinal: {err}"))?;
        let signature: String = row
            .get(2)
            .map_err(|err| format!("bad graph path signature: {err}"))?;
        let step_index: i64 = row
            .get(3)
            .map_err(|err| format!("bad graph step index: {err}"))?;
        let from_json: String = row
            .get(4)
            .map_err(|err| format!("bad graph from matrix json: {err}"))?;
        let to_json: String = row
            .get(5)
            .map_err(|err| format!("bad graph to matrix json: {err}"))?;

        if current_result_id != Some(result_id) {
            if let Some(prev_id) = current_result_id {
                guides.push(GuidePath {
                    label: format!("graph_path_result_{prev_id}_ordinal_{current_ordinal}"),
                    source_kind: "graph".to_string(),
                    source_path_signature: current_signature.clone(),
                    matrices: std::mem::take(&mut current_matrices),
                });
            }
            current_result_id = Some(result_id);
            current_ordinal = ordinal;
            current_signature = signature;
        }

        if step_index == 0 {
            current_matrices.push(parse_matrix_json(&from_json)?);
        }
        current_matrices.push(parse_matrix_json(&to_json)?);
    }

    if let Some(result_id) = current_result_id {
        guides.push(GuidePath {
            label: format!("graph_path_result_{result_id}_ordinal_{current_ordinal}"),
            source_kind: "graph".to_string(),
            source_path_signature: current_signature,
            matrices: current_matrices,
        });
    }

    Ok(guides)
}

fn load_shortcut_guides(conn: &Connection) -> Result<Vec<GuidePath>, String> {
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'shortcut_path_results'",
            [],
            |row| row.get(0),
        )
        .map_err(|err| format!("failed to probe shortcut tables: {err}"))?;
    if table_exists == 0 {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT
                r.id,
                r.guide_label,
                r.path_signature,
                m.step_index,
                mm.data_json
             FROM shortcut_path_results r
             JOIN shortcut_path_matrices m ON m.result_id = r.id
             JOIN matrices mm ON mm.id = m.matrix_id
             ORDER BY r.id, m.step_index",
        )
        .map_err(|err| format!("failed to prepare shortcut guide query: {err}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|err| format!("failed to query shortcut guides: {err}"))?;

    let mut guides = Vec::new();
    let mut current_result_id = None;
    let mut current_label = String::new();
    let mut current_signature = String::new();
    let mut current_matrices = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read shortcut guide row: {err}"))?
    {
        let result_id: i64 = row
            .get(0)
            .map_err(|err| format!("bad shortcut result id: {err}"))?;
        let guide_label: String = row
            .get(1)
            .map_err(|err| format!("bad shortcut guide label: {err}"))?;
        let signature: String = row
            .get(2)
            .map_err(|err| format!("bad shortcut path signature: {err}"))?;
        let matrix_json: String = row
            .get(4)
            .map_err(|err| format!("bad shortcut matrix json: {err}"))?;

        if current_result_id != Some(result_id) {
            if let Some(prev_id) = current_result_id {
                guides.push(GuidePath {
                    label: format!("shortcut_path_result_{prev_id}_from_{current_label}"),
                    source_kind: "shortcut".to_string(),
                    source_path_signature: current_signature.clone(),
                    matrices: std::mem::take(&mut current_matrices),
                });
            }
            current_result_id = Some(result_id);
            current_label = guide_label;
            current_signature = signature;
        }

        current_matrices.push(parse_matrix_json(&matrix_json)?);
    }

    if let Some(result_id) = current_result_id {
        guides.push(GuidePath {
            label: format!("shortcut_path_result_{result_id}_from_{current_label}"),
            source_kind: "shortcut".to_string(),
            source_path_signature: current_signature,
            matrices: current_matrices,
        });
    }

    Ok(guides)
}

fn parse_matrix_json(raw: &str) -> Result<DynMatrix, String> {
    let rows: Vec<Vec<u32>> =
        serde_json::from_str(raw).map_err(|err| format!("failed to parse matrix json: {err}"))?;
    let dim = rows.len();
    if dim == 0 {
        return Err("matrix json must not be empty".to_string());
    }
    if rows.iter().any(|row| row.len() != dim) {
        return Err("matrix json must be square".to_string());
    }
    let data = rows.into_iter().flatten().collect();
    Ok(DynMatrix::new(dim, dim, data))
}

#[derive(Clone, Copy)]
struct ShortcutRunConfig {
    k: u32,
    max_shortcut_lag: usize,
    max_dim: usize,
    max_entry: u32,
    min_gap: usize,
    max_gap: usize,
    refine_rounds: usize,
    search_mode: MoveFamilyPolicy,
    guide_count: usize,
}

struct ShortcutPathSqliteRecorder {
    conn: Connection,
    run_id: i64,
    matrix_ids: HashMap<String, i64>,
    inserted_results: usize,
}

impl ShortcutPathSqliteRecorder {
    fn new(path: impl AsRef<Path>) -> Result<Self, String> {
        let conn = Connection::open(path.as_ref())
            .map_err(|err| format!("failed to open {}: {err}", path.as_ref().display()))?;
        conn.busy_timeout(std::time::Duration::from_secs(30))
            .map_err(|err| format!("failed to configure sqlite busy timeout: {err}"))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|err| format!("failed to configure sqlite pragmas: {err}"))?;
        initialise_shortcut_sqlite_schema(&conn)?;
        Ok(Self {
            conn,
            run_id: 0,
            matrix_ids: HashMap::new(),
            inserted_results: 0,
        })
    }

    fn start_run(&mut self, config: &ShortcutRunConfig) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO shortcut_path_runs (
                    started_unix_ms,
                    k,
                    max_shortcut_lag,
                    max_dim,
                    max_entry,
                    min_gap,
                    max_gap,
                    refine_rounds,
                    search_mode,
                    guide_count
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    unix_timestamp_ms(),
                    config.k as i64,
                    config.max_shortcut_lag as i64,
                    config.max_dim as i64,
                    config.max_entry as i64,
                    config.min_gap as i64,
                    config.max_gap as i64,
                    config.refine_rounds as i64,
                    search_mode_label(config.search_mode),
                    config.guide_count as i64,
                ],
            )
            .map_err(|err| format!("failed to insert shortcut_path_runs row: {err}"))?;
        self.run_id = self.conn.last_insert_rowid();
        Ok(())
    }

    fn record_result(&mut self, result: &ShortcutGuideResult) -> Result<(), String> {
        if !result.improved {
            return Ok(());
        }
        let path_signature = matrix_path_signature(&result.final_route);
        let matrix_ids = result
            .final_route
            .iter()
            .map(|matrix| self.ensure_matrix_id(matrix))
            .collect::<Result<Vec<_>, String>>()?;

        let tx = self
            .conn
            .transaction()
            .map_err(|err| format!("failed to start shortcut sqlite transaction: {err}"))?;
        tx.execute(
            "INSERT OR IGNORE INTO shortcut_path_results (
                run_id,
                guide_label,
                source_kind,
                source_path_signature,
                final_round_index,
                initial_lag,
                final_lag,
                improved,
                matrix_count,
                path_signature
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                self.run_id,
                result.guide_label,
                result.source_kind,
                result.source_path_signature,
                result.final_round_index as i64,
                result.initial_lag as i64,
                result.final_lag as i64,
                result.improved as i64,
                result.final_route.len() as i64,
                path_signature,
            ],
        )
        .map_err(|err| format!("failed to insert shortcut_path_results row: {err}"))?;
        let inserted = tx.changes() > 0;
        let result_id = tx.last_insert_rowid();
        if inserted {
            for (step_index, matrix_id) in matrix_ids.into_iter().enumerate() {
                tx.execute(
                    "INSERT INTO shortcut_path_matrices (
                        result_id,
                        step_index,
                        matrix_id
                    ) VALUES (?1, ?2, ?3)",
                    params![result_id, step_index as i64, matrix_id],
                )
                .map_err(|err| format!("failed to insert shortcut_path_matrices row: {err}"))?;
            }
            self.inserted_results += 1;
        }
        tx.commit()
            .map_err(|err| format!("failed to commit shortcut sqlite transaction: {err}"))?;
        Ok(())
    }

    fn finish_run(&self, results: &[ShortcutGuideResult]) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE shortcut_path_runs
                 SET finished_unix_ms = ?1,
                     result_count = ?2,
                     improved_count = ?3,
                     inserted_result_count = ?4
                 WHERE id = ?5",
                params![
                    unix_timestamp_ms(),
                    results.len() as i64,
                    results.iter().filter(|result| result.improved).count() as i64,
                    self.inserted_results as i64,
                    self.run_id,
                ],
            )
            .map_err(|err| format!("failed to update shortcut_path_runs row: {err}"))?;
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
                    Some(matrix.trace() as i64),
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

fn initialise_shortcut_sqlite_schema(conn: &Connection) -> Result<(), String> {
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
        CREATE TABLE IF NOT EXISTS shortcut_path_runs (
            id INTEGER PRIMARY KEY,
            started_unix_ms INTEGER NOT NULL,
            finished_unix_ms INTEGER,
            k INTEGER NOT NULL,
            max_shortcut_lag INTEGER NOT NULL,
            max_dim INTEGER NOT NULL,
            max_entry INTEGER NOT NULL,
            min_gap INTEGER NOT NULL,
            max_gap INTEGER NOT NULL,
            refine_rounds INTEGER NOT NULL,
            search_mode TEXT NOT NULL,
            guide_count INTEGER NOT NULL,
            result_count INTEGER,
            improved_count INTEGER,
            inserted_result_count INTEGER
        );
        CREATE TABLE IF NOT EXISTS shortcut_path_results (
            id INTEGER PRIMARY KEY,
            run_id INTEGER NOT NULL REFERENCES shortcut_path_runs(id) ON DELETE CASCADE,
            guide_label TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            source_path_signature TEXT NOT NULL,
            final_round_index INTEGER NOT NULL,
            initial_lag INTEGER NOT NULL,
            final_lag INTEGER NOT NULL,
            improved INTEGER NOT NULL,
            matrix_count INTEGER NOT NULL,
            path_signature TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS shortcut_path_matrices (
            id INTEGER PRIMARY KEY,
            result_id INTEGER NOT NULL REFERENCES shortcut_path_results(id) ON DELETE CASCADE,
            step_index INTEGER NOT NULL,
            matrix_id INTEGER NOT NULL REFERENCES matrices(id),
            UNIQUE(result_id, step_index)
        );
        CREATE INDEX IF NOT EXISTS idx_shortcut_path_results_run ON shortcut_path_results(run_id);
        CREATE INDEX IF NOT EXISTS idx_shortcut_path_matrices_result ON shortcut_path_matrices(result_id, step_index);",
    )
    .map_err(|err| format!("failed to initialise shortcut sqlite schema: {err}"))?;
    Ok(())
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

fn search_mode_label(search_mode: MoveFamilyPolicy) -> &'static str {
    search_mode.snake_case_label()
}

fn unix_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
