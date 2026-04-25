use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use sse_core::factorisation::visit_factorisations_with_family_for_policy;
use sse_core::matrix::DynMatrix;
use sse_core::types::MoveFamilyPolicy;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    let report = analyze_pair(&cli);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize report: {err}"))?;

    println!("Finite-depth shadow signature");
    println!(
        "  case={} policy={} depth={} max_dim={} max_entry={} max_states_per_side={}",
        report.case_id,
        report.config.move_family_policy,
        report.config.max_depth,
        report.config.max_intermediate_dim,
        report.config.max_entry,
        report.config.max_states_per_side
    );
    println!(
        "  exact bridge depth: {} ({})",
        format_optional_usize(report.bridge_summary.min_exact_bridge_depth),
        completeness_label(report.bridge_summary.complete)
    );
    println!(
        "  shadow bridge depth: {} ({})",
        format_optional_usize(report.bridge_summary.min_shadow_bridge_depth),
        completeness_label(report.bridge_summary.complete)
    );
    println!(
        "  states: source={} target={} truncated_source={} truncated_target={}",
        report.source_shadow.total_states,
        report.target_shadow.total_states,
        report.source_shadow.truncated,
        report.target_shadow.truncated
    );
    println!(
        "  shadow keys: source={} target={} cumulative_overlap={} exact_overlap={}",
        report.source_shadow.total_shadow_keys,
        report.target_shadow.total_shadow_keys,
        report.bridge_summary.cumulative_shadow_overlap,
        report.bridge_summary.cumulative_exact_overlap
    );

    if let Some(path) = cli.json_out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
        }
        fs::write(&path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("  wrote {}", path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    case_id: String,
    source: DynMatrix,
    target: DynMatrix,
    max_depth: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: MoveFamilyPolicy,
    max_states_per_side: usize,
    json_out: Option<PathBuf>,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut case_id = "finite_depth_shadow".to_string();
    let mut source = None;
    let mut target = None;
    let mut max_depth = 2usize;
    let mut max_intermediate_dim = 4usize;
    let mut max_entry = 6u32;
    let mut move_family_policy = MoveFamilyPolicy::GraphPlusStructured;
    let mut max_states_per_side = 200_000usize;
    let mut json_out = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case-id" => {
                case_id = args.next().ok_or("--case-id requires a value")?;
            }
            "--source" => {
                source = Some(parse_matrix(
                    &args.next().ok_or("--source requires a matrix")?,
                )?);
            }
            "--target" => {
                target = Some(parse_matrix(
                    &args.next().ok_or("--target requires a matrix")?,
                )?);
            }
            "--max-depth" => {
                max_depth = parse_usize_arg(&mut args, "--max-depth")?;
            }
            "--max-intermediate-dim" => {
                max_intermediate_dim = parse_usize_arg(&mut args, "--max-intermediate-dim")?;
            }
            "--max-entry" => {
                max_entry = parse_u32_arg(&mut args, "--max-entry")?;
            }
            "--move-policy" => {
                move_family_policy = parse_move_family_policy(
                    &args.next().ok_or("--move-policy requires a value")?,
                )?;
            }
            "--max-states-per-side" => {
                max_states_per_side = parse_usize_arg(&mut args, "--max-states-per-side")?;
            }
            "--json-out" => {
                json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--help" | "-h" => {
                return Err(
                    "usage: finite_depth_shadow_signature --source MATRIX --target MATRIX [options]\n\n\
                     Matrix syntax: bare 2x2 entries like 1,3,2,1 or NxN:entries.\n\
                     Options:\n\
                       --case-id ID\n\
                       --max-depth N                  one-sided BFS depth (default: 2)\n\
                       --max-intermediate-dim N       max intermediate square dimension (default: 4)\n\
                       --max-entry N                  factor-entry cap (default: 6)\n\
                       --move-policy POLICY           graph-only, graph-plus-structured, or mixed\n\
                       --max-states-per-side N        truncation guard (default: 200000)\n\
                       --json-out PATH"
                        .to_string(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let source = source.ok_or("--source is required")?;
    let target = target.ok_or("--target is required")?;
    if !source.is_square() || !target.is_square() {
        return Err("source and target must be square".to_string());
    }
    if max_states_per_side == 0 {
        return Err("--max-states-per-side must be at least 1".to_string());
    }

    Ok(Cli {
        case_id,
        source,
        target,
        max_depth,
        max_intermediate_dim,
        max_entry,
        move_family_policy,
        max_states_per_side,
        json_out,
    })
}

fn analyze_pair(cli: &Cli) -> PairShadowReport {
    let config = ShadowConfig {
        max_depth: cli.max_depth,
        max_intermediate_dim: cli.max_intermediate_dim,
        max_entry: cli.max_entry,
        move_family_policy: cli.move_family_policy.snake_case_label().to_string(),
        max_states_per_side: cli.max_states_per_side,
    };
    let source_shadow = compute_shadow(
        &cli.source,
        cli.max_depth,
        cli.max_intermediate_dim,
        cli.max_entry,
        cli.move_family_policy,
        cli.max_states_per_side,
    );
    let target_shadow = compute_shadow(
        &cli.target,
        cli.max_depth,
        cli.max_intermediate_dim,
        cli.max_entry,
        cli.move_family_policy,
        cli.max_states_per_side,
    );
    let bridge_summary = summarize_bridges(&source_shadow, &target_shadow, cli.max_depth);
    let layer_comparisons = compare_layers(&source_shadow, &target_shadow, cli.max_depth);

    PairShadowReport {
        case_id: cli.case_id.clone(),
        source: cli.source.clone(),
        target: cli.target.clone(),
        config,
        source_shadow,
        target_shadow,
        bridge_summary,
        layer_comparisons,
    }
}

fn compute_shadow(
    root: &DynMatrix,
    max_depth: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: MoveFamilyPolicy,
    max_states_per_side: usize,
) -> EndpointShadowReport {
    let root_canonical = root.canonical_perm();
    let mut visited_depths = BTreeMap::<DynMatrix, usize>::new();
    let mut layers = vec![vec![root_canonical.clone()]];
    let mut family_counts = BTreeMap::<String, FamilyCount>::new();
    let mut truncated = false;

    visited_depths.insert(root_canonical, 0);

    for depth in 0..max_depth {
        if truncated {
            layers.push(Vec::new());
            continue;
        }

        let current_layer = layers[depth].clone();
        let mut next_layer = Vec::<DynMatrix>::new();
        for current in current_layer {
            if truncated {
                break;
            }
            visit_factorisations_with_family_for_policy(
                &current,
                max_intermediate_dim,
                max_entry,
                move_family_policy,
                |family, u, v| {
                    if truncated {
                        return;
                    }

                    let stats = family_counts.entry(family.to_string()).or_default();
                    stats.candidates += 1;

                    let next = v.mul(&u).canonical_perm();
                    if visited_depths.contains_key(&next) {
                        stats.seen_collisions += 1;
                        return;
                    }
                    if visited_depths.len() >= max_states_per_side {
                        truncated = true;
                        return;
                    }

                    stats.discovered += 1;
                    visited_depths.insert(next.clone(), depth + 1);
                    next_layer.push(next);
                },
            );
        }
        next_layer.sort();
        next_layer.dedup();
        layers.push(next_layer);
    }

    let layer_reports = build_layer_reports(&layers);
    let shadow_by_depth = shadow_keys_by_depth(&layers);
    let cumulative_shadow_keys = shadow_by_depth
        .iter()
        .flat_map(|keys| keys.iter().cloned())
        .collect::<BTreeSet<_>>();

    EndpointShadowReport {
        root_canonical: root.canonical_perm(),
        total_states: visited_depths.len(),
        total_shadow_keys: cumulative_shadow_keys.len(),
        truncated,
        layers: layer_reports,
        family_counts: family_counts
            .into_iter()
            .map(|(family, counts)| FamilyCountReport {
                family,
                candidates: counts.candidates,
                discovered: counts.discovered,
                seen_collisions: counts.seen_collisions,
            })
            .collect(),
        matrices_by_depth: layers,
        shadow_keys_by_depth: shadow_by_depth,
    }
}

fn build_layer_reports(layers: &[Vec<DynMatrix>]) -> Vec<LayerReport> {
    layers
        .iter()
        .enumerate()
        .map(|(depth, matrices)| {
            let shadow_keys = matrices
                .iter()
                .map(shadow_key)
                .collect::<BTreeSet<_>>()
                .len();
            LayerReport {
                depth,
                states: matrices.len(),
                shadow_keys,
            }
        })
        .collect()
}

fn shadow_keys_by_depth(layers: &[Vec<DynMatrix>]) -> Vec<BTreeSet<ShadowKey>> {
    layers
        .iter()
        .map(|matrices| matrices.iter().map(shadow_key).collect::<BTreeSet<_>>())
        .collect()
}

fn summarize_bridges(
    source_shadow: &EndpointShadowReport,
    target_shadow: &EndpointShadowReport,
    max_depth: usize,
) -> BridgeSummary {
    let mut min_exact_bridge_depth: Option<usize> = None;
    let mut min_shadow_bridge_depth: Option<usize> = None;

    for source_depth in 0..=max_depth {
        for target_depth in 0..=max_depth {
            let bridge_depth = source_depth + target_depth;
            if !set_intersection_is_empty(
                &source_shadow.matrices_by_depth[source_depth],
                &target_shadow.matrices_by_depth[target_depth],
            ) {
                min_exact_bridge_depth = Some(
                    min_exact_bridge_depth
                        .map(|min| min.min(bridge_depth))
                        .unwrap_or(bridge_depth),
                );
            }
            if !source_shadow.shadow_keys_by_depth[source_depth]
                .is_disjoint(&target_shadow.shadow_keys_by_depth[target_depth])
            {
                min_shadow_bridge_depth = Some(
                    min_shadow_bridge_depth
                        .map(|min| min.min(bridge_depth))
                        .unwrap_or(bridge_depth),
                );
            }
        }
    }

    let source_cumulative_matrices = cumulative_matrix_set(&source_shadow.matrices_by_depth);
    let target_cumulative_matrices = cumulative_matrix_set(&target_shadow.matrices_by_depth);
    let source_cumulative_shadow = cumulative_shadow_set(&source_shadow.shadow_keys_by_depth);
    let target_cumulative_shadow = cumulative_shadow_set(&target_shadow.shadow_keys_by_depth);

    BridgeSummary {
        complete: !source_shadow.truncated && !target_shadow.truncated,
        source_truncated: source_shadow.truncated,
        target_truncated: target_shadow.truncated,
        min_exact_bridge_depth,
        min_shadow_bridge_depth,
        cumulative_exact_overlap: source_cumulative_matrices
            .intersection(&target_cumulative_matrices)
            .count(),
        cumulative_shadow_overlap: source_cumulative_shadow
            .intersection(&target_cumulative_shadow)
            .count(),
    }
}

fn compare_layers(
    source_shadow: &EndpointShadowReport,
    target_shadow: &EndpointShadowReport,
    max_depth: usize,
) -> Vec<LayerComparison> {
    (0..=max_depth)
        .map(|depth| {
            let source_states = source_shadow.matrices_by_depth[depth].len();
            let target_states = target_shadow.matrices_by_depth[depth].len();
            let source_shadow_keys = source_shadow.shadow_keys_by_depth[depth].len();
            let target_shadow_keys = target_shadow.shadow_keys_by_depth[depth].len();
            let exact_overlap = source_shadow.matrices_by_depth[depth]
                .iter()
                .collect::<BTreeSet<_>>()
                .intersection(
                    &target_shadow.matrices_by_depth[depth]
                        .iter()
                        .collect::<BTreeSet<_>>(),
                )
                .count();
            let shadow_overlap = source_shadow.shadow_keys_by_depth[depth]
                .intersection(&target_shadow.shadow_keys_by_depth[depth])
                .count();

            LayerComparison {
                depth,
                source_states,
                target_states,
                state_count_delta: source_states.abs_diff(target_states),
                source_shadow_keys,
                target_shadow_keys,
                shadow_key_count_delta: source_shadow_keys.abs_diff(target_shadow_keys),
                exact_overlap,
                shadow_overlap,
            }
        })
        .collect()
}

fn set_intersection_is_empty(left: &[DynMatrix], right: &[DynMatrix]) -> bool {
    let right = right.iter().collect::<BTreeSet<_>>();
    left.iter().all(|item| !right.contains(item))
}

fn cumulative_matrix_set(layers: &[Vec<DynMatrix>]) -> BTreeSet<DynMatrix> {
    layers
        .iter()
        .flat_map(|layer| layer.iter().cloned())
        .collect()
}

fn cumulative_shadow_set(layers: &[BTreeSet<ShadowKey>]) -> BTreeSet<ShadowKey> {
    layers
        .iter()
        .flat_map(|layer| layer.iter().cloned())
        .collect()
}

fn shadow_key(m: &DynMatrix) -> ShadowKey {
    let mut row_sums = vec![0u64; m.rows];
    let mut col_sums = vec![0u64; m.cols];
    let mut row_supports = vec![0usize; m.rows];
    let mut col_supports = vec![0usize; m.cols];
    let mut entry_sum = 0u64;
    let mut support_count = 0usize;

    for row in 0..m.rows {
        for col in 0..m.cols {
            let value = u64::from(m.get(row, col));
            row_sums[row] += value;
            col_sums[col] += value;
            entry_sum += value;
            if value > 0 {
                support_count += 1;
                row_supports[row] += 1;
                col_supports[col] += 1;
            }
        }
    }

    row_sums.sort_unstable();
    col_sums.sort_unstable();
    row_supports.sort_unstable();
    col_supports.sort_unstable();

    ShadowKey {
        dim: m.rows,
        entry_sum,
        support_count,
        row_sums,
        col_sums,
        row_supports,
        col_supports,
    }
}

#[derive(Default)]
struct FamilyCount {
    candidates: usize,
    discovered: usize,
    seen_collisions: usize,
}

#[derive(Serialize)]
struct PairShadowReport {
    case_id: String,
    source: DynMatrix,
    target: DynMatrix,
    config: ShadowConfig,
    source_shadow: EndpointShadowReport,
    target_shadow: EndpointShadowReport,
    bridge_summary: BridgeSummary,
    layer_comparisons: Vec<LayerComparison>,
}

#[derive(Serialize)]
struct ShadowConfig {
    max_depth: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: String,
    max_states_per_side: usize,
}

#[derive(Serialize)]
struct EndpointShadowReport {
    root_canonical: DynMatrix,
    total_states: usize,
    total_shadow_keys: usize,
    truncated: bool,
    layers: Vec<LayerReport>,
    family_counts: Vec<FamilyCountReport>,
    matrices_by_depth: Vec<Vec<DynMatrix>>,
    shadow_keys_by_depth: Vec<BTreeSet<ShadowKey>>,
}

#[derive(Serialize)]
struct LayerReport {
    depth: usize,
    states: usize,
    shadow_keys: usize,
}

#[derive(Serialize)]
struct FamilyCountReport {
    family: String,
    candidates: usize,
    discovered: usize,
    seen_collisions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
struct ShadowKey {
    dim: usize,
    entry_sum: u64,
    support_count: usize,
    row_sums: Vec<u64>,
    col_sums: Vec<u64>,
    row_supports: Vec<usize>,
    col_supports: Vec<usize>,
}

#[derive(Serialize)]
struct BridgeSummary {
    complete: bool,
    source_truncated: bool,
    target_truncated: bool,
    min_exact_bridge_depth: Option<usize>,
    min_shadow_bridge_depth: Option<usize>,
    cumulative_exact_overlap: usize,
    cumulative_shadow_overlap: usize,
}

#[derive(Serialize)]
struct LayerComparison {
    depth: usize,
    source_states: usize,
    target_states: usize,
    state_count_delta: usize,
    source_shadow_keys: usize,
    target_shadow_keys: usize,
    shadow_key_count_delta: usize,
    exact_overlap: usize,
    shadow_overlap: usize,
}

fn parse_matrix(s: &str) -> Result<DynMatrix, String> {
    if let Some((dims, entries)) = s.split_once(':') {
        let (rows, cols) = parse_dims(dims)?;
        if rows != cols {
            return Err(format!("matrix must be square, got {rows}x{cols}"));
        }
        let nums = parse_entries(entries)?;
        if nums.len() != rows * cols {
            return Err(format!(
                "expected {} comma-separated entries for a {}x{} matrix, got {}",
                rows * cols,
                rows,
                cols,
                nums.len()
            ));
        }
        return Ok(DynMatrix::new(rows, cols, nums));
    }

    let nums = parse_entries(s)?;
    if nums.len() != 4 {
        return Err(format!(
            "expected 4 entries for a bare 2x2 matrix, got {}; use NxN:... for larger endpoints",
            nums.len()
        ));
    }
    Ok(DynMatrix::new(2, 2, nums))
}

fn parse_dims(s: &str) -> Result<(usize, usize), String> {
    let Some((rows, cols)) = s.split_once('x') else {
        return Err(format!(
            "invalid matrix dimension prefix {s:?}; expected NxN"
        ));
    };
    let rows = rows
        .parse::<usize>()
        .map_err(|err| format!("invalid row dimension {rows:?}: {err}"))?;
    let cols = cols
        .parse::<usize>()
        .map_err(|err| format!("invalid column dimension {cols:?}: {err}"))?;
    Ok((rows, cols))
}

fn parse_entries(s: &str) -> Result<Vec<u32>, String> {
    s.split(',')
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|err| format!("invalid matrix entry {value:?}: {err}"))
        })
        .collect()
}

fn parse_usize_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<usize, String> {
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse::<usize>()
        .map_err(|err| format!("failed to parse {flag} value {value:?}: {err}"))
}

fn parse_u32_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<u32, String> {
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse::<u32>()
        .map_err(|err| format!("failed to parse {flag} value {value:?}: {err}"))
}

fn parse_move_family_policy(raw: &str) -> Result<MoveFamilyPolicy, String> {
    match raw {
        "mixed" => Ok(MoveFamilyPolicy::Mixed),
        "graph-plus-structured" | "graph_plus_structured" => {
            Ok(MoveFamilyPolicy::GraphPlusStructured)
        }
        "graph-only" | "graph_only" => Ok(MoveFamilyPolicy::GraphOnly),
        other => Err(format!(
            "invalid --move-policy {other:?}; expected graph-only, graph-plus-structured, or mixed"
        )),
    }
}

fn format_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn completeness_label(complete: bool) -> &'static str {
    if complete {
        "complete"
    } else {
        "partial"
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_matrix, shadow_key};
    use sse_core::matrix::DynMatrix;

    #[test]
    fn bare_matrix_parser_reads_2x2_row_major_entries() {
        assert_eq!(
            parse_matrix("1,2,3,4").unwrap(),
            DynMatrix::new(2, 2, vec![1, 2, 3, 4])
        );
    }

    #[test]
    fn shadow_key_is_insensitive_to_simultaneous_permutation() {
        let matrix = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 3, 4, 0, 1]);
        let permuted = matrix.conjugate_by_perm(&[2, 0, 1]);
        assert_eq!(shadow_key(&matrix), shadow_key(&permuted));
    }
}
