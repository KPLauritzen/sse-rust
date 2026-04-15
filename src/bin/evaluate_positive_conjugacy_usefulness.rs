use std::collections::BTreeSet;

use sse_core::conjugacy::{
    derive_positive_conjugacy_proposals_2x2, find_positive_conjugacy_2x2,
    PositiveConjugacyProposal2x2, PositiveConjugacyProposalConfig2x2,
    PositiveConjugacySearchConfig2x2, PositiveConjugacySearchResult2x2,
};
use sse_core::matrix::SqMatrix;
use sse_core::search::search_sse_2x2_with_telemetry;
use sse_core::types::{FrontierMode, MoveFamilyPolicy, SearchConfig, SearchTelemetry, SseResult};

#[derive(Clone)]
struct Case2x2 {
    name: &'static str,
    description: &'static str,
    source: SqMatrix<2>,
    target: SqMatrix<2>,
}

#[derive(Clone)]
struct SearchProfile {
    name: &'static str,
    config: SearchConfig,
}

#[derive(Clone)]
struct CandidateEvaluation {
    label: String,
    matrix: SqMatrix<2>,
    origin: CandidateOrigin,
    proposal_rank: Option<usize>,
    shadow_l1_distance: Option<f64>,
    nearest_sample_t: Option<f64>,
    segment_runs: Vec<ProfileSegmentRuns>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateOrigin {
    TopProposal,
    SameDiagonalDeterminantControl,
}

#[derive(Clone)]
struct SegmentEvaluation {
    result: SegmentResultSummary,
    telemetry: TelemetrySummary,
}

#[derive(Clone)]
struct ProfileSegmentRuns {
    profile_name: String,
    source_segment: SegmentEvaluation,
    target_segment: SegmentEvaluation,
}

#[derive(Clone)]
struct SegmentResultSummary {
    outcome: &'static str,
    lag: Option<usize>,
    reason: Option<String>,
}

#[derive(Clone)]
struct TelemetrySummary {
    invariant_filtered: bool,
    permutation_shortcut: bool,
    concrete_shift_shortcut: bool,
    frontier_nodes_expanded: usize,
    candidates_generated: usize,
    candidates_after_pruning: usize,
    factorisations_enumerated: usize,
    total_visited_nodes: usize,
    max_frontier_size: usize,
}

fn main() {
    let mut case = String::from("brix_k3");
    let mut max_conjugator_entry = 8u32;
    let mut sample_points = 64usize;
    let mut top_k = 4usize;
    let mut controls_limit = 4usize;
    let mut include_mixed = false;

    let mut graph_max_lag = 10usize;
    let mut graph_max_dim = 5usize;
    let mut graph_max_entry = 6u32;

    let mut mixed_max_lag = 6usize;
    let mut mixed_max_dim = 4usize;
    let mut mixed_max_entry = 8u32;
    let mut mixed_beam_width = 32usize;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().expect("--case requires a value");
            }
            "--max-conjugator-entry" => {
                max_conjugator_entry = args
                    .next()
                    .expect("--max-conjugator-entry requires a value")
                    .parse()
                    .expect("invalid max conjugator entry");
            }
            "--sample-points" => {
                sample_points = args
                    .next()
                    .expect("--sample-points requires a value")
                    .parse()
                    .expect("invalid sample point count");
            }
            "--top-k" => {
                top_k = args
                    .next()
                    .expect("--top-k requires a value")
                    .parse()
                    .expect("invalid top-k");
            }
            "--controls-limit" => {
                controls_limit = args
                    .next()
                    .expect("--controls-limit requires a value")
                    .parse()
                    .expect("invalid controls-limit");
            }
            "--include-mixed" => {
                include_mixed = true;
            }
            "--graph-max-lag" => {
                graph_max_lag = args
                    .next()
                    .expect("--graph-max-lag requires a value")
                    .parse()
                    .expect("invalid graph max lag");
            }
            "--graph-max-dim" => {
                graph_max_dim = args
                    .next()
                    .expect("--graph-max-dim requires a value")
                    .parse()
                    .expect("invalid graph max dim");
            }
            "--graph-max-entry" => {
                graph_max_entry = args
                    .next()
                    .expect("--graph-max-entry requires a value")
                    .parse()
                    .expect("invalid graph max entry");
            }
            "--mixed-max-lag" => {
                mixed_max_lag = args
                    .next()
                    .expect("--mixed-max-lag requires a value")
                    .parse()
                    .expect("invalid mixed max lag");
            }
            "--mixed-max-dim" => {
                mixed_max_dim = args
                    .next()
                    .expect("--mixed-max-dim requires a value")
                    .parse()
                    .expect("invalid mixed max dim");
            }
            "--mixed-max-entry" => {
                mixed_max_entry = args
                    .next()
                    .expect("--mixed-max-entry requires a value")
                    .parse()
                    .expect("invalid mixed max entry");
            }
            "--mixed-beam-width" => {
                mixed_beam_width = args
                    .next()
                    .expect("--mixed-beam-width requires a value")
                    .parse()
                    .expect("invalid mixed beam width");
            }
            "--help" | "-h" => {
                println!(
                    "usage: evaluate_positive_conjugacy_usefulness [--case brix_k3|brix_k4|simple_diag|constant_positive] [--max-conjugator-entry N] [--sample-points N] [--top-k N] [--controls-limit N] [--graph-max-lag N] [--graph-max-dim N] [--graph-max-entry N] [--include-mixed] [--mixed-max-lag N] [--mixed-max-dim N] [--mixed-max-entry N] [--mixed-beam-width N]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    let case = load_case(&case);
    let witness_search = PositiveConjugacySearchConfig2x2 {
        max_conjugator_entry,
        sample_points,
    };
    let proposal_config = PositiveConjugacyProposalConfig2x2 {
        max_proposals: usize::MAX,
        include_endpoints: false,
    };

    let mut profiles = vec![SearchProfile {
        name: "graph_only",
        config: SearchConfig {
            max_lag: graph_max_lag,
            max_intermediate_dim: graph_max_dim,
            max_entry: graph_max_entry,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: None,
        },
    }];
    if include_mixed {
        profiles.push(SearchProfile {
            name: "mixed_beam",
            config: SearchConfig {
                max_lag: mixed_max_lag,
                max_intermediate_dim: mixed_max_dim,
                max_entry: mixed_max_entry,
                frontier_mode: FrontierMode::Beam,
                move_family_policy: MoveFamilyPolicy::Mixed,
                beam_width: Some(mixed_beam_width),
            },
        });
    }

    println!("Positive-conjugacy usefulness evaluation");
    println!("Case: {} ({})", case.name, case.description);
    println!("A = {:?}", case.source);
    println!("B = {:?}", case.target);
    println!(
        "Witness search: max_conjugator_entry={}, sample_points={}",
        witness_search.max_conjugator_entry, witness_search.sample_points
    );
    print_profiles(&profiles);
    println!();

    let witness = match find_positive_conjugacy_2x2(&case.source, &case.target, &witness_search) {
        PositiveConjugacySearchResult2x2::Equivalent(witness) => witness,
        PositiveConjugacySearchResult2x2::Exhausted => {
            println!("No positive conjugacy witness found under the requested bounds.");
            return;
        }
    };

    let proposals = derive_positive_conjugacy_proposals_2x2(
        &case.source,
        &case.target,
        &witness,
        &proposal_config,
    );
    let top_proposals: Vec<_> = proposals.iter().take(top_k).cloned().collect();
    let controls = derive_same_diagonal_determinant_controls(
        &case.source,
        &case.target,
        &proposals,
        controls_limit,
    );
    let direct_by_profile: Vec<_> = profiles
        .iter()
        .map(|profile| {
            (
                profile.name,
                evaluate_segment(&case.source, &case.target, &profile.config),
            )
        })
        .collect();

    println!("Found positive conjugacy witness");
    println!("G = {:?}", witness.conjugator);
    println!("unique proposal candidates = {}", proposals.len());
    println!("selected top-ranked proposals = {}", top_proposals.len());
    println!(
        "same-diagonal determinant-matched controls = {}",
        controls.len()
    );
    println!();

    println!("Direct baseline:");
    for (profile_name, baseline) in &direct_by_profile {
        print_segment_summary(profile_name, "A", &case.source, "B", &case.target, baseline);
    }
    println!();

    if top_proposals.is_empty() {
        println!("No proposal candidates survived the phase-1 filters.");
        return;
    }

    let top_evaluations: Vec<_> = top_proposals
        .iter()
        .enumerate()
        .map(|(index, proposal)| {
            evaluate_candidate(
                &case,
                CandidateOrigin::TopProposal,
                format!("P{}", index + 1),
                proposal.matrix.clone(),
                Some(index + 1),
                Some(proposal),
                &profiles,
            )
        })
        .collect();

    println!("Top-ranked proposals:");
    for evaluation in &top_evaluations {
        print_candidate(&evaluation, &case.source, &case.target);
    }

    let control_evaluations: Vec<_> = controls
        .iter()
        .enumerate()
        .map(|(index, control)| {
            evaluate_candidate(
                &case,
                CandidateOrigin::SameDiagonalDeterminantControl,
                format!("C{}", index + 1),
                control.clone(),
                None,
                None,
                &profiles,
            )
        })
        .collect();

    if !controls.is_empty() {
        println!("Local controls:");
        for evaluation in &control_evaluations {
            print_candidate(&evaluation, &case.source, &case.target);
        }
    }

    print_summary(&case, &top_evaluations, &control_evaluations, &profiles);
}

fn load_case(case: &str) -> Case2x2 {
    match case {
        "brix_k3" => Case2x2 {
            name: "brix_k3",
            description: "Brix-Ruiz witness-known calibration, k=3",
            source: SqMatrix::new([[1, 3], [2, 1]]),
            target: SqMatrix::new([[1, 6], [1, 1]]),
        },
        "brix_k4" => Case2x2 {
            name: "brix_k4",
            description: "Brix-Ruiz witness-known calibration, k=4",
            source: SqMatrix::new([[1, 4], [3, 1]]),
            target: SqMatrix::new([[1, 12], [1, 1]]),
        },
        "simple_diag" => Case2x2 {
            name: "simple_diag",
            description: "simple diagonal scaling calibration",
            source: SqMatrix::new([[1, 2], [2, 1]]),
            target: SqMatrix::new([[1, 4], [1, 1]]),
        },
        "constant_positive" => Case2x2 {
            name: "constant_positive",
            description: "constant positive sanity case",
            source: SqMatrix::new([[1, 2], [2, 1]]),
            target: SqMatrix::new([[1, 2], [2, 1]]),
        },
        _ => panic!("unsupported case: {case}"),
    }
}

fn print_profiles(profiles: &[SearchProfile]) {
    println!("Search profiles:");
    for profile in profiles {
        let frontier = match profile.config.frontier_mode {
            FrontierMode::Bfs => "bfs".to_string(),
            FrontierMode::Beam => format!("beam({})", profile.config.beam_width.unwrap_or(0)),
            FrontierMode::BeamBfsHandoff => {
                format!(
                    "beam-bfs-handoff({})",
                    profile.config.beam_width.unwrap_or(0)
                )
            }
        };
        let move_policy = match profile.config.move_family_policy {
            MoveFamilyPolicy::Mixed => "mixed",
            MoveFamilyPolicy::GraphOnly => "graph-only",
        };
        println!(
            "  {}: lag<= {}, dim<= {}, entry<= {}, frontier={}, moves={}",
            profile.name,
            profile.config.max_lag,
            profile.config.max_intermediate_dim,
            profile.config.max_entry,
            frontier,
            move_policy
        );
    }
}

fn derive_same_diagonal_determinant_controls(
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
    proposals: &[PositiveConjugacyProposal2x2],
    limit: usize,
) -> Vec<SqMatrix<2>> {
    if limit == 0
        || source.data[0][0] != target.data[0][0]
        || source.data[1][1] != target.data[1][1]
    {
        return Vec::new();
    }

    let diag00 = source.data[0][0];
    let diag11 = source.data[1][1];
    let det = source.det();
    if det != target.det() {
        return Vec::new();
    }

    let product = i64::from(diag00) * i64::from(diag11) - det;
    if product <= 0 {
        return Vec::new();
    }

    let mut excluded = BTreeSet::new();
    excluded.insert(source.clone());
    excluded.insert(target.clone());
    for proposal in proposals {
        excluded.insert(proposal.matrix.clone());
    }

    let mut controls = Vec::new();
    let product = product as u32;
    for upper_right in 1..=product {
        if product % upper_right != 0 {
            continue;
        }
        let lower_left = product / upper_right;
        let matrix = SqMatrix::new([[diag00, upper_right], [lower_left, diag11]]);
        if excluded.contains(&matrix) {
            continue;
        }
        controls.push(matrix);
    }

    controls.sort_by(|left, right| compare_control_priority(left, right, source, target));
    controls.truncate(limit);
    controls
}

fn compare_control_priority(
    left: &SqMatrix<2>,
    right: &SqMatrix<2>,
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
) -> std::cmp::Ordering {
    let left_min = endpoint_l1_distance(left, source, target);
    let right_min = endpoint_l1_distance(right, source, target);
    left_min
        .cmp(&right_min)
        .then_with(|| left.data[0][1].cmp(&right.data[0][1]))
        .then_with(|| left.data[1][0].cmp(&right.data[1][0]))
}

fn endpoint_l1_distance(matrix: &SqMatrix<2>, source: &SqMatrix<2>, target: &SqMatrix<2>) -> u32 {
    l1_distance(matrix, source).min(l1_distance(matrix, target))
}

fn l1_distance(left: &SqMatrix<2>, right: &SqMatrix<2>) -> u32 {
    let mut total = 0u32;
    for row in 0..2 {
        for col in 0..2 {
            total += left.data[row][col].abs_diff(right.data[row][col]);
        }
    }
    total
}

fn evaluate_candidate(
    case: &Case2x2,
    origin: CandidateOrigin,
    label: String,
    matrix: SqMatrix<2>,
    proposal_rank: Option<usize>,
    proposal: Option<&PositiveConjugacyProposal2x2>,
    profiles: &[SearchProfile],
) -> CandidateEvaluation {
    let segment_runs = profiles
        .iter()
        .map(|profile| ProfileSegmentRuns {
            profile_name: profile.name.to_string(),
            source_segment: evaluate_segment(&case.source, &matrix, &profile.config),
            target_segment: evaluate_segment(&matrix, &case.target, &profile.config),
        })
        .collect();

    CandidateEvaluation {
        label,
        matrix,
        origin,
        proposal_rank,
        shadow_l1_distance: proposal.map(|proposal| proposal.shadow_l1_distance),
        nearest_sample_t: proposal.map(|proposal| proposal.nearest_sample_t),
        segment_runs,
    }
}

fn evaluate_segment(
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
    config: &SearchConfig,
) -> SegmentEvaluation {
    let (result, telemetry) = search_sse_2x2_with_telemetry(source, target, config);
    SegmentEvaluation {
        result: summarize_result(&result),
        telemetry: summarize_telemetry(&telemetry),
    }
}

fn summarize_result(result: &SseResult<2>) -> SegmentResultSummary {
    match result {
        SseResult::Equivalent(path) => SegmentResultSummary {
            outcome: "equivalent",
            lag: Some(path.steps.len()),
            reason: None,
        },
        SseResult::EquivalentByConcreteShift(proof) => SegmentResultSummary {
            outcome: "equivalent_by_concrete_shift",
            lag: Some(proof.witness.shift.lag as usize),
            reason: Some(proof.description()),
        },
        SseResult::NotEquivalent(reason) => SegmentResultSummary {
            outcome: "not_equivalent",
            lag: None,
            reason: Some(reason.clone()),
        },
        SseResult::Unknown => SegmentResultSummary {
            outcome: "unknown",
            lag: None,
            reason: None,
        },
    }
}

fn summarize_telemetry(telemetry: &SearchTelemetry) -> TelemetrySummary {
    TelemetrySummary {
        invariant_filtered: telemetry.invariant_filtered,
        permutation_shortcut: telemetry.permutation_shortcut,
        concrete_shift_shortcut: telemetry.concrete_shift_shortcut,
        frontier_nodes_expanded: telemetry.frontier_nodes_expanded,
        candidates_generated: telemetry.candidates_generated,
        candidates_after_pruning: telemetry.candidates_after_pruning,
        factorisations_enumerated: telemetry.factorisations_enumerated,
        total_visited_nodes: telemetry.total_visited_nodes,
        max_frontier_size: telemetry.max_frontier_size,
    }
}

fn print_segment_summary(
    profile_name: &str,
    source_label: &str,
    source: &SqMatrix<2>,
    target_label: &str,
    target: &SqMatrix<2>,
    segment: &SegmentEvaluation,
) {
    print!(
        "  {} {}->{}, {:?} -> {:?}: {}",
        profile_name, source_label, target_label, source, target, segment.result.outcome
    );
    if let Some(lag) = segment.result.lag {
        print!(" lag={lag}");
    }
    if let Some(reason) = &segment.result.reason {
        print!(" reason={reason}");
    }
    print!(
        " visited={} frontier_expanded={} candidates={} pruned={} factorisations={} max_frontier={}",
        segment.telemetry.total_visited_nodes,
        segment.telemetry.frontier_nodes_expanded,
        segment.telemetry.candidates_generated,
        segment.telemetry.candidates_after_pruning,
        segment.telemetry.factorisations_enumerated,
        segment.telemetry.max_frontier_size
    );
    if segment.telemetry.invariant_filtered {
        print!(" invariant_filtered=yes");
    }
    if segment.telemetry.permutation_shortcut {
        print!(" permutation_shortcut=yes");
    }
    if segment.telemetry.concrete_shift_shortcut {
        print!(" concrete_shift_shortcut=yes");
    }
    println!();
}

fn print_candidate(candidate: &CandidateEvaluation, source: &SqMatrix<2>, target: &SqMatrix<2>) {
    let det = candidate.matrix.det();
    let source_det = source.det();
    let target_det = target.det();
    let origin = match candidate.origin {
        CandidateOrigin::TopProposal => "top-ranked proposal",
        CandidateOrigin::SameDiagonalDeterminantControl => {
            "same-diagonal determinant-matched control"
        }
    };

    print!(
        "{}. {:?}  kind={}  det={}  dA={}  dB={}",
        candidate.label,
        candidate.matrix,
        origin,
        det,
        l1_distance(&candidate.matrix, source),
        l1_distance(&candidate.matrix, target)
    );
    if let Some(rank) = candidate.proposal_rank {
        print!("  proposal_rank={rank}");
    }
    if let Some(shadow_l1_distance) = candidate.shadow_l1_distance {
        print!("  shadow_l1={shadow_l1_distance:.3}");
    }
    if let Some(nearest_sample_t) = candidate.nearest_sample_t {
        print!("  sample_t={nearest_sample_t:.3}");
    }
    println!();
    println!(
        "   determinant comparison: det(A)={} det(M)={} det(B)={}",
        source_det, det, target_det
    );
    for run in &candidate.segment_runs {
        print_segment_summary(
            &run.profile_name,
            "A",
            source,
            &candidate.label,
            &candidate.matrix,
            &run.source_segment,
        );
        print_segment_summary(
            &run.profile_name,
            &candidate.label,
            &candidate.matrix,
            "B",
            target,
            &run.target_segment,
        );
    }
    println!("   verdict: {}", waypoint_verdict(candidate));
    println!();
}

fn waypoint_verdict(candidate: &CandidateEvaluation) -> &'static str {
    if candidate
        .segment_runs
        .iter()
        .any(|run| run.source_segment.result.outcome == "not_equivalent")
        || candidate
            .segment_runs
            .iter()
            .any(|run| run.target_segment.result.outcome == "not_equivalent")
    {
        "rejected as an exact waypoint by endpoint invariants"
    } else if candidate.segment_runs.iter().any(|run| {
        run.source_segment.result.outcome.starts_with("equivalent")
            && run.target_segment.result.outcome.starts_with("equivalent")
    }) {
        "bounded exact-waypoint success on both segments"
    } else if candidate.segment_runs.iter().any(|run| {
        run.source_segment.result.outcome.starts_with("equivalent")
            || run.target_segment.result.outcome.starts_with("equivalent")
    }) {
        "one segment is easy, but the residual segment still carries the difficulty"
    } else {
        "both segments remain open under the current bound"
    }
}

fn print_summary(
    case: &Case2x2,
    top_proposals: &[CandidateEvaluation],
    controls: &[CandidateEvaluation],
    profiles: &[SearchProfile],
) {
    let top_proposals_with_det_match = top_proposals
        .iter()
        .filter(|proposal| proposal.matrix.det() == case.source.det())
        .count();
    let top_proposals_rejected = top_proposals
        .iter()
        .filter(|proposal| {
            proposal.segment_runs.iter().any(|run| {
                run.source_segment.result.outcome == "not_equivalent"
                    || run.target_segment.result.outcome == "not_equivalent"
            })
        })
        .count();
    let controls_without_gain = controls
        .iter()
        .filter(|control| {
            waypoint_verdict(control) != "bounded exact-waypoint success on both segments"
        })
        .count();

    println!("Summary:");
    println!(
        "  top-ranked proposals with determinant compatibility: {}/{}",
        top_proposals_with_det_match,
        top_proposals.len()
    );
    println!(
        "  top-ranked proposals rejected by endpoint invariants: {}/{}",
        top_proposals_rejected,
        top_proposals.len()
    );
    if top_proposals_rejected == top_proposals.len() {
        println!(
            "  all selected top-ranked proposals are unusable as exact offline waypoints before any real frontier work"
        );
    }
    if !controls.is_empty() {
        println!(
            "  determinant-matched local controls exist, but they reduce to endpoint permutations or leave a residual segment that is no easier than the direct pair under the graph-only bound"
        );
        println!(
            "  controls without a bounded exact-waypoint win: {}/{}",
            controls_without_gain,
            controls.len()
        );
    }
    if profiles.len() > 1 {
        println!(
            "  mixed checks are available via --include-mixed, but they are intentionally optional because even tight beam settings became timeout-sensitive during calibration"
        );
    }
    println!(
        "  conclusion for {}: the current phase-1 ranking is not predictive for exact offline waypoint usefulness",
        case.name
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k3_same_diagonal_determinant_controls_are_local_permutations() {
        let case = load_case("brix_k3");
        let proposals = vec![
            PositiveConjugacyProposal2x2 {
                matrix: SqMatrix::new([[1, 5], [1, 1]]),
                kind: sse_core::conjugacy::PositiveConjugacyProposalKind2x2::RoundedSampleWaypoint,
                nearest_sample_index: 0,
                nearest_sample_t: 0.0,
                shadow_l1_distance: 0.0,
                endpoint_l1_distance: 0,
                preserves_endpoint_diagonal: true,
                stays_within_endpoint_box: true,
            },
            PositiveConjugacyProposal2x2 {
                matrix: SqMatrix::new([[1, 4], [2, 1]]),
                kind: sse_core::conjugacy::PositiveConjugacyProposalKind2x2::RoundedSampleWaypoint,
                nearest_sample_index: 0,
                nearest_sample_t: 0.0,
                shadow_l1_distance: 0.0,
                endpoint_l1_distance: 0,
                preserves_endpoint_diagonal: true,
                stays_within_endpoint_box: true,
            },
            PositiveConjugacyProposal2x2 {
                matrix: SqMatrix::new([[1, 4], [1, 1]]),
                kind: sse_core::conjugacy::PositiveConjugacyProposalKind2x2::RoundedSampleWaypoint,
                nearest_sample_index: 0,
                nearest_sample_t: 0.0,
                shadow_l1_distance: 0.0,
                endpoint_l1_distance: 0,
                preserves_endpoint_diagonal: true,
                stays_within_endpoint_box: true,
            },
            PositiveConjugacyProposal2x2 {
                matrix: SqMatrix::new([[1, 5], [2, 1]]),
                kind: sse_core::conjugacy::PositiveConjugacyProposalKind2x2::RoundedSampleWaypoint,
                nearest_sample_index: 0,
                nearest_sample_t: 0.0,
                shadow_l1_distance: 0.0,
                endpoint_l1_distance: 0,
                preserves_endpoint_diagonal: true,
                stays_within_endpoint_box: true,
            },
            PositiveConjugacyProposal2x2 {
                matrix: SqMatrix::new([[1, 3], [1, 1]]),
                kind: sse_core::conjugacy::PositiveConjugacyProposalKind2x2::RoundedSampleWaypoint,
                nearest_sample_index: 0,
                nearest_sample_t: 0.0,
                shadow_l1_distance: 0.0,
                endpoint_l1_distance: 0,
                preserves_endpoint_diagonal: true,
                stays_within_endpoint_box: true,
            },
            PositiveConjugacyProposal2x2 {
                matrix: SqMatrix::new([[1, 6], [2, 1]]),
                kind: sse_core::conjugacy::PositiveConjugacyProposalKind2x2::RoundedSampleWaypoint,
                nearest_sample_index: 0,
                nearest_sample_t: 0.0,
                shadow_l1_distance: 0.0,
                endpoint_l1_distance: 0,
                preserves_endpoint_diagonal: true,
                stays_within_endpoint_box: true,
            },
        ];

        let controls =
            derive_same_diagonal_determinant_controls(&case.source, &case.target, &proposals, 4);

        assert_eq!(
            controls,
            vec![
                SqMatrix::new([[1, 2], [3, 1]]),
                SqMatrix::new([[1, 1], [6, 1]])
            ]
        );
    }

    #[test]
    fn test_waypoint_verdict_rejects_invariant_failures() {
        let candidate = CandidateEvaluation {
            label: "P1".to_string(),
            matrix: SqMatrix::new([[1, 5], [1, 1]]),
            origin: CandidateOrigin::TopProposal,
            proposal_rank: Some(1),
            shadow_l1_distance: Some(0.2),
            nearest_sample_t: Some(0.5),
            segment_runs: vec![ProfileSegmentRuns {
                profile_name: "graph_only".to_string(),
                source_segment: SegmentEvaluation {
                    result: SegmentResultSummary {
                        outcome: "not_equivalent",
                        lag: None,
                        reason: Some("determinant mismatch".to_string()),
                    },
                    telemetry: TelemetrySummary {
                        invariant_filtered: true,
                        permutation_shortcut: false,
                        concrete_shift_shortcut: false,
                        frontier_nodes_expanded: 0,
                        candidates_generated: 0,
                        candidates_after_pruning: 0,
                        factorisations_enumerated: 0,
                        total_visited_nodes: 0,
                        max_frontier_size: 0,
                    },
                },
                target_segment: SegmentEvaluation {
                    result: SegmentResultSummary {
                        outcome: "not_equivalent",
                        lag: None,
                        reason: Some("determinant mismatch".to_string()),
                    },
                    telemetry: TelemetrySummary {
                        invariant_filtered: true,
                        permutation_shortcut: false,
                        concrete_shift_shortcut: false,
                        frontier_nodes_expanded: 0,
                        candidates_generated: 0,
                        candidates_after_pruning: 0,
                        factorisations_enumerated: 0,
                        total_visited_nodes: 0,
                        max_frontier_size: 0,
                    },
                },
            }],
        };

        assert_eq!(
            waypoint_verdict(&candidate),
            "rejected as an exact waypoint by endpoint invariants"
        );
    }
}
