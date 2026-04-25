use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;
use sse_core::graph_moves::{
    enumerate_same_future_insplits, partition_refined_same_future_past_gap_total,
    same_future_past_signature, same_future_past_signature_gap, SameFuturePastSignatureGap,
};
use sse_core::matrix::DynMatrix;
use sse_core::search::search_sse_with_telemetry_dyn;
use sse_core::types::{DynSseResult, FrontierMode, MoveFamilyPolicy, SearchConfig};

const SCOUT_CASE_ID: &str = "rank4_diagonal_sparse_4x4";
const SOURCE_NOTE: &str =
    "research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md";
const HYPOTHESIS: &str = "Use the smallest higher-power proxy, the squared matrix M^2, as a compressed 2-block shadow to rank one-step same-future in-split proposals from a retained near-cap Brix-Ruiz k=4 scout pair. This stays distinct from blind split widening because it never broadens the main lane or keeps the widened 5x5 frontier alive; it only shortlists a tiny in-split proposal set for direct bounded realization checks.";

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) if err == usage() => println!("{err}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Err(usage());
    }
    let cli = parse_cli(args.into_iter())?;
    let scout = scout_case();
    let report = evaluate_case(&scout, &cli)?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize report: {err}"))?;

    if let Some(path) = cli.json_out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
        }
        fs::write(&path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("wrote {}", path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    json_out: Option<PathBuf>,
    shortlist_size: usize,
    probe_lag: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut cli = Cli {
        json_out: None,
        shortlist_size: 6,
        probe_lag: 3,
        max_intermediate_dim: 5,
        max_entry: 12,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json-out" => {
                cli.json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--shortlist-size" => {
                cli.shortlist_size = parse_positive_usize(args.next(), "--shortlist-size")?;
            }
            "--probe-lag" => {
                cli.probe_lag = parse_positive_usize(args.next(), "--probe-lag")?;
            }
            "--max-intermediate-dim" => {
                cli.max_intermediate_dim =
                    parse_positive_usize(args.next(), "--max-intermediate-dim")?;
            }
            "--max-entry" => {
                cli.max_entry = parse_positive_u32(args.next(), "--max-entry")?;
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if cli.shortlist_size > 24 {
        return Err("--shortlist-size must be at most 24".to_string());
    }
    if cli.probe_lag > 6 {
        return Err("--probe-lag must be at most 6 for this bounded scout".to_string());
    }
    if cli.max_intermediate_dim > 5 {
        return Err("--max-intermediate-dim must be at most 5".to_string());
    }
    if cli.max_entry > 12 {
        return Err("--max-entry must be at most 12".to_string());
    }

    Ok(cli)
}

fn parse_positive_usize(value: Option<String>, label: &str) -> Result<usize, String> {
    let parsed = value
        .ok_or_else(|| format!("{label} requires a value"))?
        .parse::<usize>()
        .map_err(|_| format!("invalid {label}"))?;
    if parsed == 0 {
        return Err(format!("{label} must be at least 1"));
    }
    Ok(parsed)
}

fn parse_positive_u32(value: Option<String>, label: &str) -> Result<u32, String> {
    let parsed = value
        .ok_or_else(|| format!("{label} requires a value"))?
        .parse::<u32>()
        .map_err(|_| format!("invalid {label}"))?;
    if parsed == 0 {
        return Err(format!("{label} must be at least 1"));
    }
    Ok(parsed)
}

fn usage() -> String {
    "usage: evaluate_brix_ruiz_k4_higher_power_insplit_proposals [--shortlist-size N<=24] [--probe-lag N<=6] [--max-intermediate-dim N<=5] [--max-entry N<=12] [--json-out PATH]".to_string()
}

#[derive(Clone)]
struct ScoutCase {
    id: &'static str,
    label: &'static str,
    current: DynMatrix,
    target: DynMatrix,
}

fn scout_case() -> ScoutCase {
    ScoutCase {
        id: SCOUT_CASE_ID,
        label: "retained rank-4 diagonal approximate pair",
        current: DynMatrix::new(4, 4, vec![1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0]),
        target: DynMatrix::new(4, 4, vec![1, 12, 0, 1, 1, 1, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0]),
    }
}

#[derive(Clone)]
struct Candidate {
    matrix: DynMatrix,
    raw_gap: SameFuturePastSignatureGap,
    raw_partition_gap: u64,
    power_gap: SameFuturePastSignatureGap,
    power_partition_gap: u64,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct SignatureGapSnapshot {
    dimension_gap: usize,
    row_class_gap: usize,
    col_class_gap: usize,
    entry_sum_gap: u64,
}

impl From<SameFuturePastSignatureGap> for SignatureGapSnapshot {
    fn from(gap: SameFuturePastSignatureGap) -> Self {
        Self {
            dimension_gap: gap.dimension_gap,
            row_class_gap: gap.row_class_gap,
            col_class_gap: gap.col_class_gap,
            entry_sum_gap: gap.entry_sum_gap,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum StrategyKind {
    BlindCoarseGap,
    HigherPowerGap,
}

#[derive(Clone)]
struct CachedProbe {
    admitted: bool,
    outcome: String,
    outcome_reason: Option<String>,
    lag: Option<usize>,
    approximate_hits: usize,
    frontier_nodes_expanded: usize,
    total_visited_nodes: usize,
    elapsed_ms: u64,
}

#[derive(Serialize)]
struct Report {
    scout_case_id: &'static str,
    scout_label: &'static str,
    source_note: &'static str,
    hypothesis: &'static str,
    current: DynMatrix,
    target: DynMatrix,
    proposal_count: usize,
    unique_proposal_count: usize,
    search_config: SearchConfigSnapshot,
    blind_strategy: StrategyReport,
    higher_power_strategy: StrategyReport,
    keep_decision: String,
}

#[derive(Serialize)]
struct SearchConfigSnapshot {
    probe_lag: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    frontier_mode: &'static str,
    move_family_policy: &'static str,
}

#[derive(Serialize)]
struct StrategyReport {
    strategy: StrategyKind,
    shortlist_size: usize,
    admitted_count: usize,
    equivalent_count: usize,
    approximate_hit_count: usize,
    proposals_with_approx_hit_count: usize,
    max_frontier_nodes_expanded: usize,
    max_total_visited_nodes: usize,
    total_elapsed_ms: u64,
    proposals: Vec<ProposalReport>,
}

#[derive(Serialize)]
struct ProposalReport {
    rank: usize,
    matrix: DynMatrix,
    raw_gap: SignatureGapSnapshot,
    raw_partition_gap: u64,
    power_gap: SignatureGapSnapshot,
    power_partition_gap: u64,
    admitted: bool,
    outcome: String,
    outcome_reason: Option<String>,
    lag: Option<usize>,
    approximate_hits: usize,
    frontier_nodes_expanded: usize,
    total_visited_nodes: usize,
    elapsed_ms: u64,
}

fn evaluate_case(scout: &ScoutCase, cli: &Cli) -> Result<Report, String> {
    let target_signature = same_future_past_signature(&scout.target)
        .expect("square target should always produce a signature");
    let target_square = scout.target.mul(&scout.target);
    let target_square_signature = same_future_past_signature(&target_square)
        .expect("square target power should always produce a signature");

    let raw_proposals = enumerate_same_future_insplits(&scout.current);
    let proposal_count = raw_proposals.len();
    let mut unique = BTreeMap::<DynMatrix, Candidate>::new();
    for witness in raw_proposals {
        let matrix = witness.outsplit.canonical_perm();
        let signature = same_future_past_signature(&matrix)
            .expect("square proposal should always produce a signature");
        let square = matrix.mul(&matrix);
        let square_signature = same_future_past_signature(&square)
            .expect("square proposal power should always produce a signature");
        unique.entry(matrix.clone()).or_insert_with(|| Candidate {
            matrix: matrix.clone(),
            raw_gap: same_future_past_signature_gap(&signature, &target_signature),
            raw_partition_gap: partition_refined_same_future_past_gap_total(&matrix, &scout.target),
            power_gap: same_future_past_signature_gap(&square_signature, &target_square_signature),
            power_partition_gap: partition_refined_same_future_past_gap_total(
                &square,
                &target_square,
            ),
        });
    }

    let unique_candidates = unique.into_values().collect::<Vec<_>>();
    let search_config = SearchConfig {
        max_lag: cli.probe_lag,
        max_intermediate_dim: cli.max_intermediate_dim,
        max_entry: cli.max_entry,
        frontier_mode: FrontierMode::Bfs,
        move_family_policy: MoveFamilyPolicy::GraphOnly,
        beam_width: None,
        beam_bfs_handoff_depth: None,
        beam_bfs_handoff_deferred_cap: None,
        endpoint_multi_meet_cap: None,
    };

    let mut probe_cache = BTreeMap::<DynMatrix, CachedProbe>::new();
    let blind_strategy = evaluate_strategy(
        StrategyKind::BlindCoarseGap,
        unique_candidates.clone(),
        cli.shortlist_size,
        &search_config,
        &scout.target,
        &mut probe_cache,
    );
    let higher_power_strategy = evaluate_strategy(
        StrategyKind::HigherPowerGap,
        unique_candidates.clone(),
        cli.shortlist_size,
        &search_config,
        &scout.target,
        &mut probe_cache,
    );

    let keep_decision = decide_keep_or_reject(&blind_strategy, &higher_power_strategy);

    Ok(Report {
        scout_case_id: scout.id,
        scout_label: scout.label,
        source_note: SOURCE_NOTE,
        hypothesis: HYPOTHESIS,
        current: scout.current.clone(),
        target: scout.target.clone(),
        proposal_count,
        unique_proposal_count: unique_candidates.len(),
        search_config: SearchConfigSnapshot {
            probe_lag: cli.probe_lag,
            max_intermediate_dim: cli.max_intermediate_dim,
            max_entry: cli.max_entry,
            frontier_mode: "bfs",
            move_family_policy: "graph_only",
        },
        blind_strategy,
        higher_power_strategy,
        keep_decision,
    })
}

fn evaluate_strategy(
    strategy: StrategyKind,
    mut proposals: Vec<Candidate>,
    shortlist_size: usize,
    search_config: &SearchConfig,
    target: &DynMatrix,
    probe_cache: &mut BTreeMap<DynMatrix, CachedProbe>,
) -> StrategyReport {
    sort_candidates(&mut proposals, strategy);
    proposals.truncate(shortlist_size);

    let mut admitted_count = 0usize;
    let mut equivalent_count = 0usize;
    let mut approximate_hit_count = 0usize;
    let mut proposals_with_approx_hit_count = 0usize;
    let mut max_frontier_nodes_expanded = 0usize;
    let mut max_total_visited_nodes = 0usize;
    let mut total_elapsed_ms = 0u64;
    let mut reports = Vec::with_capacity(proposals.len());

    for (rank, proposal) in proposals.into_iter().enumerate() {
        let probe = probe_cache
            .entry(proposal.matrix.clone())
            .or_insert_with(|| run_probe(&proposal.matrix, target, search_config));
        admitted_count += usize::from(probe.admitted);
        equivalent_count += usize::from(probe.outcome == "equivalent");
        approximate_hit_count += probe.approximate_hits;
        proposals_with_approx_hit_count += usize::from(probe.approximate_hits > 0);
        max_frontier_nodes_expanded =
            max_frontier_nodes_expanded.max(probe.frontier_nodes_expanded);
        max_total_visited_nodes = max_total_visited_nodes.max(probe.total_visited_nodes);
        total_elapsed_ms += probe.elapsed_ms;

        reports.push(ProposalReport {
            rank: rank + 1,
            matrix: proposal.matrix,
            raw_gap: proposal.raw_gap.into(),
            raw_partition_gap: proposal.raw_partition_gap,
            power_gap: proposal.power_gap.into(),
            power_partition_gap: proposal.power_partition_gap,
            admitted: probe.admitted,
            outcome: probe.outcome.clone(),
            outcome_reason: probe.outcome_reason.clone(),
            lag: probe.lag,
            approximate_hits: probe.approximate_hits,
            frontier_nodes_expanded: probe.frontier_nodes_expanded,
            total_visited_nodes: probe.total_visited_nodes,
            elapsed_ms: probe.elapsed_ms,
        });
    }

    StrategyReport {
        strategy,
        shortlist_size: reports.len(),
        admitted_count,
        equivalent_count,
        approximate_hit_count,
        proposals_with_approx_hit_count,
        max_frontier_nodes_expanded,
        max_total_visited_nodes,
        total_elapsed_ms,
        proposals: reports,
    }
}

fn sort_candidates(proposals: &mut [Candidate], strategy: StrategyKind) {
    proposals.sort_by(|left, right| match strategy {
        StrategyKind::BlindCoarseGap => left
            .raw_gap
            .cmp(&right.raw_gap)
            .then_with(|| left.raw_partition_gap.cmp(&right.raw_partition_gap))
            .then_with(|| left.matrix.cmp(&right.matrix)),
        StrategyKind::HigherPowerGap => left
            .power_gap
            .cmp(&right.power_gap)
            .then_with(|| left.power_partition_gap.cmp(&right.power_partition_gap))
            .then_with(|| left.raw_gap.cmp(&right.raw_gap))
            .then_with(|| left.matrix.cmp(&right.matrix)),
    });
}

fn run_probe(proposal: &DynMatrix, target: &DynMatrix, config: &SearchConfig) -> CachedProbe {
    let started = Instant::now();
    let (result, telemetry) = search_sse_with_telemetry_dyn(proposal, target, config);
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let (outcome, outcome_reason, lag) = summarize_result(&result);
    CachedProbe {
        admitted: telemetry.invalid_config.is_none() && !telemetry.invariant_filtered,
        outcome,
        outcome_reason,
        lag,
        approximate_hits: telemetry.approximate_other_side_hits,
        frontier_nodes_expanded: telemetry.frontier_nodes_expanded,
        total_visited_nodes: telemetry.total_visited_nodes,
        elapsed_ms,
    }
}

fn decide_keep_or_reject(
    blind_strategy: &StrategyReport,
    higher_power_strategy: &StrategyReport,
) -> String {
    if higher_power_strategy.equivalent_count > blind_strategy.equivalent_count
        || (higher_power_strategy.equivalent_count == blind_strategy.equivalent_count
            && higher_power_strategy.approximate_hit_count > blind_strategy.approximate_hit_count
            && higher_power_strategy.admitted_count >= blind_strategy.admitted_count)
        || (higher_power_strategy.approximate_hit_count == blind_strategy.approximate_hit_count
            && higher_power_strategy.equivalent_count == blind_strategy.equivalent_count
            && higher_power_strategy.admitted_count >= blind_strategy.admitted_count
            && higher_power_strategy.max_frontier_nodes_expanded
                <= blind_strategy.max_frontier_nodes_expanded
            && higher_power_strategy.max_total_visited_nodes
                <= blind_strategy.max_total_visited_nodes
            && higher_power_strategy.total_elapsed_ms <= blind_strategy.total_elapsed_ms
            && (higher_power_strategy.admitted_count > blind_strategy.admitted_count
                || higher_power_strategy.max_frontier_nodes_expanded
                    < blind_strategy.max_frontier_nodes_expanded
                || higher_power_strategy.max_total_visited_nodes
                    < blind_strategy.max_total_visited_nodes
                || higher_power_strategy.total_elapsed_ms < blind_strategy.total_elapsed_ms))
    {
        "keep for larger retained-lane scout follow-up".to_string()
    } else {
        "reject for now on this scout surface".to_string()
    }
}

fn summarize_result(result: &DynSseResult) -> (String, Option<String>, Option<usize>) {
    match result {
        DynSseResult::Equivalent(path) => ("equivalent".to_string(), None, Some(path.steps.len())),
        DynSseResult::NotEquivalent(reason) => {
            ("not_equivalent".to_string(), Some(reason.clone()), None)
        }
        DynSseResult::Unknown => ("unknown".to_string(), None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_power_sort_prefers_smaller_power_gap() {
        let matrix_a = DynMatrix::new(1, 1, vec![1]);
        let matrix_b = DynMatrix::new(1, 1, vec![2]);
        let worse = Candidate {
            matrix: matrix_b.clone(),
            raw_gap: SameFuturePastSignatureGap::default(),
            raw_partition_gap: 0,
            power_gap: SameFuturePastSignatureGap {
                entry_sum_gap: 5,
                ..SameFuturePastSignatureGap::default()
            },
            power_partition_gap: 0,
        };
        let better = Candidate {
            matrix: matrix_a.clone(),
            raw_gap: SameFuturePastSignatureGap::default(),
            raw_partition_gap: 0,
            power_gap: SameFuturePastSignatureGap {
                entry_sum_gap: 1,
                ..SameFuturePastSignatureGap::default()
            },
            power_partition_gap: 0,
        };
        let mut proposals = vec![worse, better.clone()];
        sort_candidates(&mut proposals, StrategyKind::HigherPowerGap);
        assert_eq!(proposals[0].matrix, better.matrix);
    }

    #[test]
    fn scout_case_matrix_is_square() {
        let scout = scout_case();
        assert_eq!(scout.current.rows, scout.current.cols);
        assert_eq!(scout.target.rows, scout.target.cols);
    }

    #[test]
    fn keep_decision_rejects_lower_admission_tie_break() {
        let blind = StrategyReport {
            strategy: StrategyKind::BlindCoarseGap,
            shortlist_size: 6,
            admitted_count: 6,
            equivalent_count: 0,
            approximate_hit_count: 0,
            proposals_with_approx_hit_count: 0,
            max_frontier_nodes_expanded: 6,
            max_total_visited_nodes: 846,
            total_elapsed_ms: 78,
            proposals: Vec::new(),
        };
        let higher_power = StrategyReport {
            strategy: StrategyKind::HigherPowerGap,
            shortlist_size: 6,
            admitted_count: 5,
            equivalent_count: 0,
            approximate_hit_count: 0,
            proposals_with_approx_hit_count: 0,
            max_frontier_nodes_expanded: 5,
            max_total_visited_nodes: 800,
            total_elapsed_ms: 70,
            proposals: Vec::new(),
        };

        assert_eq!(
            decide_keep_or_reject(&blind, &higher_power),
            "reject for now on this scout surface"
        );
    }

    #[test]
    fn keep_decision_rejects_approximate_hit_gain_with_fewer_equivalents() {
        let blind = StrategyReport {
            strategy: StrategyKind::BlindCoarseGap,
            shortlist_size: 6,
            admitted_count: 6,
            equivalent_count: 1,
            approximate_hit_count: 0,
            proposals_with_approx_hit_count: 0,
            max_frontier_nodes_expanded: 6,
            max_total_visited_nodes: 846,
            total_elapsed_ms: 78,
            proposals: Vec::new(),
        };
        let higher_power = StrategyReport {
            strategy: StrategyKind::HigherPowerGap,
            shortlist_size: 6,
            admitted_count: 6,
            equivalent_count: 0,
            approximate_hit_count: 2,
            proposals_with_approx_hit_count: 1,
            max_frontier_nodes_expanded: 5,
            max_total_visited_nodes: 800,
            total_elapsed_ms: 70,
            proposals: Vec::new(),
        };

        assert_eq!(
            decide_keep_or_reject(&blind, &higher_power),
            "reject for now on this scout surface"
        );
    }

    #[test]
    fn keep_decision_rejects_exact_tie() {
        let blind = StrategyReport {
            strategy: StrategyKind::BlindCoarseGap,
            shortlist_size: 6,
            admitted_count: 6,
            equivalent_count: 0,
            approximate_hit_count: 0,
            proposals_with_approx_hit_count: 0,
            max_frontier_nodes_expanded: 6,
            max_total_visited_nodes: 846,
            total_elapsed_ms: 78,
            proposals: Vec::new(),
        };
        let higher_power = StrategyReport {
            strategy: StrategyKind::HigherPowerGap,
            shortlist_size: 6,
            admitted_count: 6,
            equivalent_count: 0,
            approximate_hit_count: 0,
            proposals_with_approx_hit_count: 0,
            max_frontier_nodes_expanded: 6,
            max_total_visited_nodes: 846,
            total_elapsed_ms: 78,
            proposals: Vec::new(),
        };

        assert_eq!(
            decide_keep_or_reject(&blind, &higher_power),
            "reject for now on this scout surface"
        );
    }
}
