use std::env;

use serde::Serialize;
use sse_core::factorisation::visit_factorisations_with_family_for_policy;
use sse_core::graph_moves::find_exact_graph_move_witness_between;
use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::search::search_sse_dyn;
use sse_core::types::{GuideArtifactPayload, MoveFamilyPolicy, SearchConfig};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum StepClassification {
    AlreadyGraphCoded,
    DiagonalRefactorizationLike,
    StructuredFactorizationMatch,
    NeedsLongerSplitAmalgamationExpansion,
    NotRepresentedByCurrentStructuredFamilies,
}

#[derive(Debug, Serialize)]
struct Report {
    factorisation_match: FactorisationMatchConfig,
    graph_probe: GraphProbeConfig,
    artifacts: Vec<ArtifactReport>,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct FactorisationMatchConfig {
    max_entry: Option<u32>,
    match_up_to_permutation: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct GraphProbeConfig {
    max_lag: usize,
    max_intermediate_dim: usize,
    max_entry: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ArtifactReport {
    source_path: String,
    artifact_id: Option<String>,
    endpoint_lag: Option<usize>,
    step_reports: Vec<StepReport>,
}

#[derive(Debug, Serialize)]
struct StepReport {
    step_index: usize,
    from_matrix: DynMatrix,
    to_matrix: DynMatrix,
    exact_graph_family: Option<String>,
    graph_plus_structured_families: Vec<String>,
    mixed_families: Vec<String>,
    graph_probe: Option<GraphProbeResult>,
    classification: StepClassification,
    reasoning: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GraphProbeResult {
    lag: usize,
    matrices: Vec<DynMatrix>,
}

fn main() -> Result<(), String> {
    let cli = parse_cli(env::args().skip(1))?;
    let mut artifacts = Vec::new();

    for path in &cli.guide_paths {
        for artifact in load_guide_artifacts_from_path(path)? {
            let GuideArtifactPayload::FullPath { path: witness_path } = artifact.payload;
            if witness_path.matrices.len() != witness_path.steps.len() + 1 {
                return Err(format!(
                    "guide artifact at {path} contains {} matrices but {} steps",
                    witness_path.matrices.len(),
                    witness_path.steps.len()
                ));
            }
            let step_reports = witness_path
                .steps
                .iter()
                .enumerate()
                .map(|(step_index, _step)| {
                    classify_step(
                        step_index,
                        &witness_path.matrices[step_index],
                        &witness_path.matrices[step_index + 1],
                        cli.factorisation_match,
                        cli.graph_probe,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;

            artifacts.push(ArtifactReport {
                source_path: path.clone(),
                artifact_id: artifact.artifact_id,
                endpoint_lag: artifact.quality.lag,
                step_reports,
            });
        }
    }

    let report = Report {
        factorisation_match: cli.factorisation_match,
        graph_probe: cli.graph_probe,
        artifacts,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to serialize classification report: {err}"))?
    );

    Ok(())
}

struct Cli {
    guide_paths: Vec<String>,
    factorisation_match: FactorisationMatchConfig,
    graph_probe: GraphProbeConfig,
}

fn parse_cli(args: impl Iterator<Item = String>) -> Result<Cli, String> {
    let mut guide_paths = Vec::new();
    let mut factorisation_match = FactorisationMatchConfig {
        max_entry: None,
        match_up_to_permutation: false,
    };
    let mut graph_probe = GraphProbeConfig {
        max_lag: 3,
        max_intermediate_dim: 4,
        max_entry: None,
    };
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--guide-artifact" => {
                guide_paths.push(
                    args.next()
                        .ok_or("--guide-artifact requires a path".to_string())?,
                );
            }
            "--graph-probe-max-lag" => {
                graph_probe.max_lag = parse_usize_arg(&mut args, "--graph-probe-max-lag")?;
            }
            "--graph-probe-max-intermediate-dim" => {
                graph_probe.max_intermediate_dim =
                    parse_usize_arg(&mut args, "--graph-probe-max-intermediate-dim")?;
            }
            "--factorisation-max-entry" => {
                factorisation_match.max_entry =
                    Some(parse_u32_arg(&mut args, "--factorisation-max-entry")?);
            }
            "--match-up-to-permutation" => {
                factorisation_match.match_up_to_permutation = true;
            }
            "--graph-probe-max-entry" => {
                graph_probe.max_entry = Some(parse_u32_arg(&mut args, "--graph-probe-max-entry")?);
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: classify_witness_steps --guide-artifact PATH [--guide-artifact PATH ...]\
\n       [--factorisation-max-entry N] [--graph-probe-max-lag N]\
\n       [--graph-probe-max-intermediate-dim N] [--graph-probe-max-entry N]\
\n       [--match-up-to-permutation]"
                        .to_string(),
                );
            }
            other => {
                return Err(format!("unrecognized argument: {other}"));
            }
        }
    }

    if guide_paths.is_empty() {
        return Err(
            "classify_witness_steps requires at least one --guide-artifact PATH".to_string(),
        );
    }
    if graph_probe.max_lag == 0 {
        return Err("--graph-probe-max-lag must be at least 1".to_string());
    }
    if graph_probe.max_intermediate_dim < 2 {
        return Err("--graph-probe-max-intermediate-dim must be at least 2".to_string());
    }

    Ok(Cli {
        guide_paths,
        factorisation_match,
        graph_probe,
    })
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

fn classify_step(
    step_index: usize,
    from_matrix: &DynMatrix,
    to_matrix: &DynMatrix,
    factorisation_match: FactorisationMatchConfig,
    graph_probe: GraphProbeConfig,
) -> Result<StepReport, String> {
    if !from_matrix.is_square() || !to_matrix.is_square() {
        return Err(format!(
            "step {step_index} is not between square matrices: {}x{} -> {}x{}",
            from_matrix.rows, from_matrix.cols, to_matrix.rows, to_matrix.cols
        ));
    }

    let exact_graph_family = find_exact_graph_move_witness_between(from_matrix, to_matrix)
        .map(|successor| successor.family.to_string());

    let factorisation_max_intermediate_dim =
        factorisation_max_intermediate_dim(from_matrix, to_matrix);
    let factorisation_max_entry =
        factorisation_match_max_entry(from_matrix, to_matrix, factorisation_match);
    let graph_plus_structured_families = matching_factorisation_families(
        from_matrix,
        to_matrix,
        factorisation_match,
        factorisation_max_intermediate_dim,
        factorisation_max_entry,
        MoveFamilyPolicy::GraphPlusStructured,
    )?;
    let mixed_families = matching_factorisation_families(
        from_matrix,
        to_matrix,
        factorisation_match,
        factorisation_max_intermediate_dim,
        factorisation_max_entry,
        MoveFamilyPolicy::Mixed,
    )?;

    let graph_probe_result = if exact_graph_family.is_none() {
        probe_graph_only_expansion(from_matrix, to_matrix, graph_probe)
    } else {
        None
    };

    let diagonal_like = graph_plus_structured_families
        .iter()
        .any(|family| family.starts_with("diagonal_refactorization_"));
    let has_structured_match = !graph_plus_structured_families.is_empty();
    let longer_graph_expansion = graph_probe_result
        .as_ref()
        .is_some_and(|probe| probe.lag > 1);

    let classification = if exact_graph_family.is_some() {
        StepClassification::AlreadyGraphCoded
    } else if diagonal_like {
        StepClassification::DiagonalRefactorizationLike
    } else if has_structured_match {
        StepClassification::StructuredFactorizationMatch
    } else if longer_graph_expansion {
        StepClassification::NeedsLongerSplitAmalgamationExpansion
    } else {
        StepClassification::NotRepresentedByCurrentStructuredFamilies
    };

    let mut reasoning = Vec::new();
    if let Some(family) = exact_graph_family.as_deref() {
        reasoning.push(format!("exact graph-only witness family: {family}"));
    }
    if !graph_plus_structured_families.is_empty() {
        reasoning.push(format!(
            "graph_plus_structured matching families (factorisation max_entry={}, up_to_permutation={}): {}",
            factorisation_max_entry,
            factorisation_match.match_up_to_permutation,
            graph_plus_structured_families.join(", ")
        ));
    }
    if !mixed_families.is_empty() && mixed_families != graph_plus_structured_families {
        reasoning.push(format!(
            "mixed-only additional families: {}",
            mixed_families.join(", ")
        ));
    }
    match &graph_probe_result {
        Some(probe) => reasoning.push(format!(
            "graph-only bounded probe succeeded at lag {} with max_intermediate_dim={} and max_entry={}",
            probe.lag,
            graph_probe.max_intermediate_dim,
            graph_probe_max_entry(from_matrix, to_matrix, graph_probe)
        )),
        None if exact_graph_family.is_none() => reasoning.push(format!(
            "graph-only bounded probe found no path within lag {}, max_intermediate_dim={}, max_entry={}",
            graph_probe.max_lag,
            graph_probe.max_intermediate_dim,
            graph_probe_max_entry(from_matrix, to_matrix, graph_probe)
        )),
        None => {}
    }

    Ok(StepReport {
        step_index,
        from_matrix: from_matrix.clone(),
        to_matrix: to_matrix.clone(),
        exact_graph_family,
        graph_plus_structured_families,
        mixed_families,
        graph_probe: graph_probe_result,
        classification,
        reasoning,
    })
}

fn factorisation_max_intermediate_dim(from_matrix: &DynMatrix, to_matrix: &DynMatrix) -> usize {
    from_matrix
        .rows
        .max(from_matrix.cols)
        .max(to_matrix.rows)
        .max(to_matrix.cols)
}

fn factorisation_max_entry(from_matrix: &DynMatrix, to_matrix: &DynMatrix) -> u32 {
    from_matrix.max_entry().max(to_matrix.max_entry())
}

fn factorisation_match_max_entry(
    from_matrix: &DynMatrix,
    to_matrix: &DynMatrix,
    factorisation_match: FactorisationMatchConfig,
) -> u32 {
    factorisation_match
        .max_entry
        .unwrap_or_else(|| factorisation_max_entry(from_matrix, to_matrix))
}

fn matching_factorisation_families(
    from_matrix: &DynMatrix,
    to_matrix: &DynMatrix,
    factorisation_match: FactorisationMatchConfig,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: MoveFamilyPolicy,
) -> Result<Vec<String>, String> {
    if !from_matrix.is_square() || !to_matrix.is_square() {
        return Err(format!(
            "factorisation matching requires square matrices, got {}x{} and {}x{}",
            from_matrix.rows, from_matrix.cols, to_matrix.rows, to_matrix.cols
        ));
    }

    let mut families = Vec::new();
    let target_canon = factorisation_match
        .match_up_to_permutation
        .then(|| to_matrix.canonical_perm());
    for representative in factorisation_representatives(from_matrix, factorisation_match) {
        visit_factorisations_with_family_for_policy(
            &representative,
            max_intermediate_dim,
            max_entry,
            move_family_policy,
            |family, u, v| {
                let target = v.mul(&u);
                let matches = if let Some(target_canon) = &target_canon {
                    target.canonical_perm() == *target_canon
                } else {
                    u.mul(&v) == *from_matrix && target == *to_matrix
                };
                if matches {
                    families.push(family.to_string());
                }
            },
        );
    }
    families.sort();
    families.dedup();
    Ok(families)
}

fn factorisation_representatives(
    matrix: &DynMatrix,
    factorisation_match: FactorisationMatchConfig,
) -> Vec<DynMatrix> {
    if factorisation_match.match_up_to_permutation {
        permutation_representatives(matrix)
    } else {
        vec![matrix.clone()]
    }
}

fn permutation_representatives(matrix: &DynMatrix) -> Vec<DynMatrix> {
    let n = matrix.rows;
    let mut representatives = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut perm: Vec<usize> = (0..n).collect();

    loop {
        let representative = matrix.conjugate_by_perm(&perm);
        if seen.insert(representative.clone()) {
            representatives.push(representative);
        }
        if !next_permutation(&mut perm) {
            break;
        }
    }

    representatives
}

fn next_permutation(perm: &mut [usize]) -> bool {
    let n = perm.len();
    if n <= 1 {
        return false;
    }

    let mut i = n - 1;
    while i > 0 && perm[i - 1] >= perm[i] {
        i -= 1;
    }
    if i == 0 {
        return false;
    }

    let mut j = n - 1;
    while perm[j] <= perm[i - 1] {
        j -= 1;
    }
    perm.swap(i - 1, j);
    perm[i..].reverse();
    true
}

fn probe_graph_only_expansion(
    from_matrix: &DynMatrix,
    to_matrix: &DynMatrix,
    graph_probe: GraphProbeConfig,
) -> Option<GraphProbeResult> {
    let max_entry = graph_probe_max_entry(from_matrix, to_matrix, graph_probe);
    let result = search_sse_dyn(
        from_matrix,
        to_matrix,
        &SearchConfig {
            max_lag: graph_probe.max_lag,
            max_intermediate_dim: graph_probe.max_intermediate_dim,
            max_entry,
            frontier_mode: Default::default(),
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
            endpoint_multi_meet_cap: None,
        },
    );
    match result {
        sse_core::types::DynSseResult::Equivalent(path) => Some(GraphProbeResult {
            lag: path.steps.len(),
            matrices: path.matrices,
        }),
        _ => None,
    }
}

fn graph_probe_max_entry(
    from_matrix: &DynMatrix,
    to_matrix: &DynMatrix,
    graph_probe: GraphProbeConfig,
) -> u32 {
    graph_probe
        .max_entry
        .unwrap_or_else(|| factorisation_max_entry(from_matrix, to_matrix))
}
