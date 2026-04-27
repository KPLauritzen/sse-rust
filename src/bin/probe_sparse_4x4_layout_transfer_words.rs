use std::env;
use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use sse_core::factorisation::visit_factorisations_with_family_for_policy;
use sse_core::graph_moves::find_exact_graph_move_witness_between;
use sse_core::matrix::DynMatrix;
use sse_core::search::{search_sse_with_telemetry_dyn, validate_sse_path_dyn};
use sse_core::types::{
    DynSsePath, DynSseResult, EsseStep, FrontierMode, MoveFamilyPolicy, SearchConfig,
    SearchTelemetry,
};

const DEFAULT_MAX_DEPTH: usize = 3;
const DEFAULT_MAX_INTERMEDIATE_DIM: usize = 4;
const DEFAULT_MAX_ENTRY: u32 = 12;
const MAX_PERMUTATION_DIM: usize = 6;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{}", usage());
        return Ok(());
    }
    let cli = parse_cli(args.into_iter())?;
    let report = build_report(&cli)?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize report: {err}"))?;

    if let Some(path) = &cli.json_out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
        }
        fs::write(path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("wrote {}", path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    max_depth: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    json_out: Option<PathBuf>,
    selected_cases: Vec<String>,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut max_depth = DEFAULT_MAX_DEPTH;
    let mut max_intermediate_dim = DEFAULT_MAX_INTERMEDIATE_DIM;
    let mut max_entry = DEFAULT_MAX_ENTRY;
    let mut json_out = None;
    let mut selected_cases = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-depth" => {
                max_depth = parse_next(&mut args, "--max-depth")?;
            }
            "--max-intermediate-dim" => {
                max_intermediate_dim = parse_next(&mut args, "--max-intermediate-dim")?;
            }
            "--max-entry" => {
                max_entry = parse_next(&mut args, "--max-entry")?;
            }
            "--case" => {
                selected_cases.push(args.next().ok_or("--case requires an id")?);
            }
            "--json-out" => {
                json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--help" | "-h" => return Err(usage()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if max_depth == 0 {
        return Err("--max-depth must be at least 1".to_string());
    }
    if max_intermediate_dim < 2 {
        return Err("--max-intermediate-dim must be at least 2".to_string());
    }
    if max_entry == 0 {
        return Err("--max-entry must be at least 1".to_string());
    }

    Ok(Cli {
        max_depth,
        max_intermediate_dim,
        max_entry,
        json_out,
        selected_cases,
    })
}

fn parse_next<T>(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse()
        .map_err(|err| format!("failed to parse {flag} value {value:?}: {err}"))
}

fn usage() -> String {
    "usage: probe_sparse_4x4_layout_transfer_words [--max-depth N] \
     [--max-intermediate-dim N] [--max-entry N] [--case ID ...] [--json-out PATH]"
        .to_string()
}

#[derive(Serialize)]
struct Report {
    probe: ProbeSettings,
    cases: Vec<CaseReport>,
    summary: SummaryReport,
}

#[derive(Serialize)]
struct ProbeSettings {
    move_policy: &'static str,
    frontier_mode: &'static str,
    max_depth: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    vocabulary_note: &'static str,
}

#[derive(Serialize)]
struct SummaryReport {
    cases_total: usize,
    exact_words_found: usize,
    exact_words_missing: usize,
    conclusion: String,
}

#[derive(Serialize)]
struct CaseReport {
    id: &'static str,
    title: &'static str,
    source_label: &'static str,
    target_label: &'static str,
    source: MatrixReport,
    target: MatrixReport,
    direct_one_step_families: Vec<String>,
    outcome: &'static str,
    lag: Option<usize>,
    word: Vec<WordStepReport>,
    path_matrices: Vec<MatrixReport>,
    negative_result: Option<String>,
    telemetry: TelemetryReport,
}

#[derive(Serialize)]
struct MatrixReport {
    dimension: usize,
    rows: Vec<Vec<u32>>,
}

#[derive(Serialize)]
struct WordStepReport {
    step_index: usize,
    family: String,
    orientation: &'static str,
    from: MatrixReport,
    to: MatrixReport,
    u: MatrixReport,
    v: MatrixReport,
}

#[derive(Serialize)]
struct TelemetryReport {
    frontier_nodes_expanded: usize,
    factorisations_enumerated: usize,
    candidates_generated: usize,
    candidates_after_pruning: usize,
    discovered_nodes: usize,
    exact_meets: usize,
    approximate_other_side_hits: usize,
    max_frontier_size: usize,
    layers: usize,
}

#[derive(Clone, Debug)]
struct ProbeCase {
    id: &'static str,
    title: &'static str,
    source_label: &'static str,
    target_label: &'static str,
    source: DynMatrix,
    target: DynMatrix,
}

fn build_report(cli: &Cli) -> Result<Report, String> {
    let cases = selected_cases(&cli.selected_cases)?;
    let config = SearchConfig {
        max_lag: cli.max_depth,
        max_intermediate_dim: cli.max_intermediate_dim,
        max_entry: cli.max_entry,
        frontier_mode: FrontierMode::Bfs,
        move_family_policy: MoveFamilyPolicy::GraphPlusStructured,
        beam_width: None,
        beam_bfs_handoff_depth: None,
        beam_bfs_handoff_deferred_cap: None,
        endpoint_multi_meet_cap: None,
    };

    let mut reports = Vec::new();
    for case in cases {
        reports.push(probe_case(&case, &config)?);
    }

    let exact_words_found = reports
        .iter()
        .filter(|case| case.outcome == "equivalent")
        .count();
    let exact_words_missing = reports.len() - exact_words_found;
    let conclusion = if exact_words_missing == 0 {
        "all selected cases have an exact short word under the bounded existing vocabulary"
            .to_string()
    } else {
        format!(
            "{exact_words_missing} selected case(s) still lack an exact short word under the bounded existing vocabulary"
        )
    };

    Ok(Report {
        probe: ProbeSettings {
            move_policy: "graph_plus_structured",
            frontier_mode: "bfs",
            max_depth: cli.max_depth,
            max_intermediate_dim: cli.max_intermediate_dim,
            max_entry: cli.max_entry,
            vocabulary_note:
                "existing graph moves plus currently selected graph_plus_structured factorisation families only",
        },
        summary: SummaryReport {
            cases_total: reports.len(),
            exact_words_found,
            exact_words_missing,
            conclusion,
        },
        cases: reports,
    })
}

fn selected_cases(selected_ids: &[String]) -> Result<Vec<ProbeCase>, String> {
    let all = built_in_cases();
    if selected_ids.is_empty() {
        return Ok(all);
    }

    let mut selected = Vec::new();
    for requested in selected_ids {
        let Some(case) = all.iter().find(|case| case.id == requested.as_str()) else {
            let available = all
                .iter()
                .map(|case| case.id)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "unknown case {requested:?}; available cases: {available}"
            ));
        };
        selected.push(case.clone());
    }
    Ok(selected)
}

fn probe_case(case: &ProbeCase, config: &SearchConfig) -> Result<CaseReport, String> {
    let direct_one_step_families = direct_one_step_families(&case.source, &case.target, config);
    let (result, telemetry) = search_sse_with_telemetry_dyn(&case.source, &case.target, config);
    match result {
        DynSseResult::Equivalent(path) => {
            validate_sse_path_dyn(&case.source, &case.target, &path)
                .map_err(|err| format!("{} returned invalid path: {err}", case.id))?;
            let word = label_path_steps(&path, config);
            Ok(CaseReport {
                id: case.id,
                title: case.title,
                source_label: case.source_label,
                target_label: case.target_label,
                source: matrix_report(&case.source),
                target: matrix_report(&case.target),
                direct_one_step_families,
                outcome: "equivalent",
                lag: Some(path.steps.len()),
                word,
                path_matrices: path.matrices.iter().map(matrix_report).collect(),
                negative_result: None,
                telemetry: telemetry_report(&telemetry),
            })
        }
        DynSseResult::NotEquivalent(reason) => Ok(CaseReport {
            id: case.id,
            title: case.title,
            source_label: case.source_label,
            target_label: case.target_label,
            source: matrix_report(&case.source),
            target: matrix_report(&case.target),
            direct_one_step_families,
            outcome: "not_equivalent",
            lag: None,
            word: Vec::new(),
            path_matrices: Vec::new(),
            negative_result: Some(reason),
            telemetry: telemetry_report(&telemetry),
        }),
        DynSseResult::Unknown => Ok(CaseReport {
            id: case.id,
            title: case.title,
            source_label: case.source_label,
            target_label: case.target_label,
            source: matrix_report(&case.source),
            target: matrix_report(&case.target),
            direct_one_step_families,
            outcome: "unknown",
            lag: None,
            word: Vec::new(),
            path_matrices: Vec::new(),
            negative_result: Some(format!(
                "no exact word found within depth {}, max_intermediate_dim {}, max_entry {}",
                config.max_lag, config.max_intermediate_dim, config.max_entry
            )),
            telemetry: telemetry_report(&telemetry),
        }),
    }
}

fn direct_one_step_families(
    from: &DynMatrix,
    to: &DynMatrix,
    config: &SearchConfig,
) -> Vec<String> {
    let mut families = Vec::new();
    if find_permutation_relabeling(from, to).is_some() {
        families.push("permutation_relabeling".to_string());
    }
    if let Some(successor) = find_exact_graph_move_witness_between(from, to) {
        families.push(successor.family.to_string());
    }
    families.extend(matching_factorisation_families(
        from,
        to,
        config.max_intermediate_dim,
        config.max_entry,
        config.move_family_policy,
    ));
    families.sort();
    families.dedup();
    families
}

fn label_path_steps(path: &DynSsePath, config: &SearchConfig) -> Vec<WordStepReport> {
    path.steps
        .iter()
        .enumerate()
        .map(|(step_index, step)| {
            let from = &path.matrices[step_index];
            let to = &path.matrices[step_index + 1];
            let (family, orientation) = label_step(from, to, step, config);
            WordStepReport {
                step_index,
                family,
                orientation,
                from: matrix_report(from),
                to: matrix_report(to),
                u: matrix_report(&step.u),
                v: matrix_report(&step.v),
            }
        })
        .collect()
}

fn label_step(
    from: &DynMatrix,
    to: &DynMatrix,
    step: &EsseStep,
    config: &SearchConfig,
) -> (String, &'static str) {
    if find_permutation_relabeling(from, to).is_some() {
        return ("permutation_relabeling".to_string(), "forward");
    }
    if let Some(successor) = find_exact_graph_move_witness_between(from, to) {
        return (successor.family.to_string(), "forward");
    }
    if let Some(successor) = find_exact_graph_move_witness_between(to, from) {
        if let Some(inverse) = inverse_graph_family(successor.family) {
            return (inverse.to_string(), "reverse");
        }
    }

    let direct = matching_factorisation_families(
        from,
        to,
        config.max_intermediate_dim,
        config.max_entry,
        config.move_family_policy,
    );
    if !direct.is_empty() {
        return (direct.join("|"), "forward");
    }

    let reverse = matching_factorisation_families(
        to,
        from,
        config.max_intermediate_dim,
        config.max_entry,
        config.move_family_policy,
    );
    if !reverse.is_empty() {
        return (reverse.join("|"), "reverse");
    }

    if step.u.mul(&step.v) == *from && step.v.mul(&step.u) == *to {
        return ("unlabelled_existing_esse_step".to_string(), "forward");
    }
    ("unclassified_step".to_string(), "unknown")
}

fn inverse_graph_family(family: &str) -> Option<&'static str> {
    match family {
        "outsplit" => Some("in_amalgamation"),
        "insplit" => Some("out_amalgamation"),
        "out_amalgamation" => Some("insplit"),
        "in_amalgamation" => Some("outsplit"),
        _ => None,
    }
}

fn matching_factorisation_families(
    from: &DynMatrix,
    to: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: MoveFamilyPolicy,
) -> Vec<String> {
    let mut families = Vec::new();
    visit_factorisations_with_family_for_policy(
        from,
        max_intermediate_dim,
        max_entry,
        move_family_policy,
        |family, u, v| {
            if u.mul(&v) == *from && v.mul(&u) == *to {
                families.push(family.to_string());
            }
        },
    );
    families.sort();
    families.dedup();
    families
}

fn telemetry_report(telemetry: &SearchTelemetry) -> TelemetryReport {
    let exact_meets = telemetry
        .move_family_telemetry
        .values()
        .map(|family| family.exact_meets)
        .sum();
    TelemetryReport {
        frontier_nodes_expanded: telemetry.frontier_nodes_expanded,
        factorisations_enumerated: telemetry.factorisations_enumerated,
        candidates_generated: telemetry.candidates_generated,
        candidates_after_pruning: telemetry.candidates_after_pruning,
        discovered_nodes: telemetry.discovered_nodes,
        exact_meets,
        approximate_other_side_hits: telemetry.approximate_other_side_hits,
        max_frontier_size: telemetry.max_frontier_size,
        layers: telemetry.layers.len(),
    }
}

fn matrix_report(matrix: &DynMatrix) -> MatrixReport {
    MatrixReport {
        dimension: matrix.rows,
        rows: (0..matrix.rows)
            .map(|row| {
                (0..matrix.cols)
                    .map(|col| matrix.get(row, col))
                    .collect::<Vec<_>>()
            })
            .collect(),
    }
}

fn find_permutation_relabeling(from: &DynMatrix, to: &DynMatrix) -> Option<Vec<usize>> {
    if from.rows != from.cols || to.rows != to.cols || from.rows != to.rows {
        return None;
    }
    if from.rows > MAX_PERMUTATION_DIM {
        return None;
    }
    let mut permutation = (0..from.rows).collect::<Vec<_>>();
    let mut result = None;
    for_each_permutation(&mut permutation, 0, &mut |perm| {
        if result.is_some() {
            return;
        }
        let (p, pinv) = permutation_matrices(perm);
        if pinv.mul(from).mul(&p) == *to {
            result = Some(perm.iter().map(|idx| idx + 1).collect());
        }
    });
    result
}

fn permutation_matrices(permutation: &[usize]) -> (DynMatrix, DynMatrix) {
    let n = permutation.len();
    let mut p_data = vec![0u32; n * n];
    let mut pinv_data = vec![0u32; n * n];
    for (row, &col) in permutation.iter().enumerate() {
        p_data[row * n + col] = 1;
        pinv_data[col * n + row] = 1;
    }
    (
        DynMatrix::new(n, n, p_data),
        DynMatrix::new(n, n, pinv_data),
    )
}

fn for_each_permutation<F>(permutation: &mut [usize], start: usize, visit: &mut F)
where
    F: FnMut(&[usize]),
{
    if start == permutation.len() {
        visit(permutation);
        return;
    }
    for idx in start..permutation.len() {
        permutation.swap(start, idx);
        for_each_permutation(permutation, start + 1, visit);
        permutation.swap(start, idx);
    }
}

fn built_in_cases() -> Vec<ProbeCase> {
    vec![
        ProbeCase {
            id: "baker_a4_to_a5",
            title: "Baker/Lind-Marcus A4 -> A5 exact same-size 4x4 control",
            source_label: "Baker A4",
            target_label: "Baker A5",
            source: matrix4([1, 2, 2, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 2, 1, 0]),
            target: matrix4([1, 1, 1, 1, 3, 0, 2, 2, 1, 0, 0, 0, 0, 1, 1, 1]),
        },
        ProbeCase {
            id: "brix_ruiz_k4_rank4_frontier_to_counterpart",
            title: "Brix-Ruiz k=4 retained rank-4 sparse 2x4 active-block near-hit",
            source_label: "rank-4 diagonal-refactorization frontier child",
            target_label: "rank-4 closest opposite-side counterpart",
            source: matrix4([1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0]),
            target: matrix4([1, 12, 0, 1, 1, 1, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0]),
        },
        ProbeCase {
            id: "brix_ruiz_k4_rank6_frontier_to_counterpart",
            title: "Brix-Ruiz k=4 retained rank-6 sparse 4x2 active-block near-hit",
            source_label: "rank-6 diagonal-refactorization frontier child",
            target_label: "rank-6 closest opposite-side counterpart",
            source: matrix4([0, 2, 3, 0, 0, 2, 1, 0, 0, 11, 0, 0, 0, 2, 2, 0]),
            target: matrix4([0, 2, 1, 0, 0, 1, 4, 0, 0, 3, 1, 0, 0, 11, 0, 0]),
        },
    ]
}

fn matrix4(data: [u32; 16]) -> DynMatrix {
    DynMatrix::new(4, 4, data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_cases_rejects_unknown_ids() {
        let err = selected_cases(&["missing".to_string()]).expect_err("unknown case should fail");

        assert!(err.contains("available cases"));
    }

    #[test]
    fn direct_one_step_recognizes_rank4_parent_diagonal_refactorization() {
        let config = SearchConfig {
            max_lag: DEFAULT_MAX_DEPTH,
            max_intermediate_dim: DEFAULT_MAX_INTERMEDIATE_DIM,
            max_entry: DEFAULT_MAX_ENTRY,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::GraphPlusStructured,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
            endpoint_multi_meet_cap: None,
        };
        let parent = matrix4([1, 4, 1, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0]);
        let child = matrix4([1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0]);

        let families = direct_one_step_families(&parent, &child, &config);

        assert!(families.contains(&"diagonal_refactorization_4x4".to_string()));
    }

    #[test]
    fn baker_control_has_bounded_short_word() {
        let cli = Cli {
            max_depth: 3,
            max_intermediate_dim: 4,
            max_entry: 5,
            json_out: None,
            selected_cases: vec!["baker_a4_to_a5".to_string()],
        };

        let report = build_report(&cli).expect("probe should run");
        let case = &report.cases[0];

        assert_eq!(case.outcome, "equivalent");
        assert_eq!(case.lag, Some(3));
        assert_eq!(
            case.word
                .iter()
                .map(|step| step.family.as_str())
                .collect::<Vec<_>>(),
            vec![
                "binary_sparse_rectangular_factorisation_4x3_to_3",
                "permutation_relabeling",
                "insplit"
            ]
        );
    }
}
