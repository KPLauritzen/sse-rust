use std::collections::{HashMap, HashSet, VecDeque};

use sse_core::factorisation::visit_all_factorisations_with_family;
use sse_core::graph_moves::enumerate_graph_move_successors;
use sse_core::matrix::DynMatrix;
use sse_core::types::{EsseStep, SearchMode};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

fn main() {
    let mut max_shortcut_lag = 6usize;
    let mut max_dim = 5usize;
    let mut max_entry = 6u32;
    let mut min_gap = 2usize;
    let mut max_gap = usize::MAX;
    let mut search_mode = SearchMode::Mixed;

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
            "--search-mode" => {
                let value = args.next().expect("--search-mode requires a value");
                search_mode = match value.as_str() {
                    "mixed" => SearchMode::Mixed,
                    "graph-only" | "graph_only" => SearchMode::GraphOnly,
                    _ => panic!("unknown search mode: {value}"),
                };
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_brix_ruiz_path_shortcuts [--max-shortcut-lag N] [--max-dim N] [--max-entry N] [--min-gap N] [--max-gap N] [--search-mode mixed|graph-only]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    let guide = endpoint_16_path();
    println!("Brix-Ruiz k=3 shortcut search along the known 16-move graph path");
    println!(
        "config: search_mode={:?}, max_shortcut_lag={}, max_dim={}, max_entry={}, min_gap={}, max_gap={}",
        search_mode, max_shortcut_lag, max_dim, max_entry, min_gap, max_gap
    );
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
    search_mode: SearchMode,
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

    for _layer in 0..max_lag {
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
        let expansions = expand_frontier(
            &current_frontier,
            orig,
            max_dim,
            max_entry,
            search_mode,
            source_trace,
            source_trace_square,
        );
        let next_depth = layer_depth + 1;
        let mut next_frontier = VecDeque::new();

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

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
    }

    SearchResult::NotFound {
        visited: visited_union_size(&fwd_parent, &bwd_parent),
    }
}

fn expand_frontier(
    frontier: &[DynMatrix],
    orig: &HashMap<DynMatrix, DynMatrix>,
    max_dim: usize,
    max_entry: u32,
    search_mode: SearchMode,
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

        if search_mode == SearchMode::Mixed {
            visit_all_factorisations_with_family(current, max_dim, max_entry, |_family, u, v| {
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
            });
        }

        expansions
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let per_node: Vec<Vec<FrontierExpansion>> = frontier.par_iter().map(expand_node).collect();
        deduplicate_expansions(per_node.into_iter().flatten().collect())
    }

    #[cfg(target_arch = "wasm32")]
    {
        deduplicate_expansions(frontier.iter().flat_map(expand_node).collect())
    }
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
