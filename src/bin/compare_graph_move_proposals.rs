use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use sse_core::graph_moves::{
    enumerate_graph_move_successors, enumerate_graph_proposals, same_future_past_signature,
    same_future_past_signature_gap, GraphProposal, SameFuturePastSignatureGap,
};
use sse_core::matrix::DynMatrix;
use sse_core::search::{probe_graph_proposal_shortlist, GraphProposalProbeConfig};
use sse_core::types::{DynSseResult, FrontierMode, MoveFamilyPolicy, SearchConfig};

#[derive(Deserialize)]
struct EndpointFixtureFile {
    fixtures: Vec<EndpointFixture>,
}

#[derive(Clone, Deserialize)]
struct EndpointFixture {
    id: String,
    a: Vec<Vec<u32>>,
    b: Vec<Vec<u32>>,
    #[serde(default)]
    seeded_guides: Vec<SeededGuide>,
}

#[derive(Clone, Deserialize)]
struct SeededGuide {
    id: String,
    #[serde(default)]
    label: Option<String>,
    matrices: Vec<Vec<Vec<u32>>>,
}

#[derive(Clone)]
struct Cli {
    fixture_ref: String,
    seeded_guide_id: String,
    current_selector: String,
    target_selector: String,
    max_dim: Option<usize>,
    zigzag_bridge_entry: Option<u32>,
    top_k: usize,
    probe_lag: Option<usize>,
    probe_shortlist_k: usize,
}

#[derive(Clone)]
struct ScoredBlindSuccessor {
    family: &'static str,
    matrix: DynMatrix,
    gap: SameFuturePastSignatureGap,
}

fn main() -> Result<(), String> {
    let cli = parse_args()?;
    let fixture = load_endpoint_fixture(&cli.fixture_ref)?;
    let current = select_matrix(&fixture, &cli.seeded_guide_id, &cli.current_selector)?;
    let target = select_matrix(&fixture, &cli.seeded_guide_id, &cli.target_selector)?;
    let max_dim = cli
        .max_dim
        .unwrap_or_else(|| current.rows.max(target.rows).max(current.rows + 1));

    let target_signature = same_future_past_signature(&target)
        .expect("square target should always have a same-future/past signature");
    let blind = enumerate_graph_move_successors(&current, max_dim);
    let mut blind_scored = blind
        .nodes
        .iter()
        .map(|successor| {
            let signature = same_future_past_signature(&successor.matrix)
                .expect("square blind successor should always have a signature");
            ScoredBlindSuccessor {
                family: successor.family,
                matrix: successor.matrix.clone(),
                gap: same_future_past_signature_gap(&signature, &target_signature),
            }
        })
        .collect::<Vec<_>>();
    blind_scored.sort_by(|left, right| {
        left.gap
            .cmp(&right.gap)
            .then_with(|| left.matrix.cmp(&right.matrix))
    });

    let proposals = enumerate_graph_proposals(&current, &target, max_dim, cli.zigzag_bridge_entry);
    let blind_set = blind_scored
        .iter()
        .map(|candidate| candidate.matrix.clone())
        .collect::<BTreeSet<_>>();
    let proposal_overlap = proposals
        .nodes
        .iter()
        .filter(|proposal| blind_set.contains(&proposal.matrix))
        .count();

    println!("Graph move proposal comparison");
    println!("Fixture: {}", cli.fixture_ref);
    println!("Current: {} ({:?})", cli.current_selector, current);
    println!("Target: {} ({:?})", cli.target_selector, target);
    println!("max_dim = {}", max_dim);
    println!(
        "zigzag_bridge_entry = {}",
        cli.zigzag_bridge_entry
            .map(|value| value.to_string())
            .unwrap_or_else(|| "disabled".to_string())
    );
    println!();

    print_blind_summary(&blind, &blind_scored);
    println!();
    print_proposal_summary(&proposals, proposal_overlap);
    println!();
    print_top_blind_successors(&blind_scored, cli.top_k);
    println!();
    print_top_proposals(&proposals.nodes, cli.top_k);
    if let Some(probe_lag) = cli.probe_lag {
        println!();
        let probe_config = GraphProposalProbeConfig {
            shortlist_size: cli.probe_shortlist_k.max(1),
            realization_max_lag: probe_lag,
            max_zigzag_bridge_entry: cli.zigzag_bridge_entry,
        };
        let search_config = SearchConfig {
            max_lag: probe_lag,
            max_intermediate_dim: max_dim,
            max_entry: 8,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: None,
        };
        let probe =
            probe_graph_proposal_shortlist(&current, &target, &search_config, &probe_config)
                .map_err(|err| format!("proposal probe failed: {err}"))?;
        print_proposal_probe(&probe, probe_lag);
    }

    Ok(())
}

fn parse_args() -> Result<Cli, String> {
    let mut fixture_ref = "research/fixtures/brix_ruiz_family.json#brix_ruiz_k3".to_string();
    let mut seeded_guide_id = "endpoint_16_path".to_string();
    let mut current_selector = "source".to_string();
    let mut target_selector = "target".to_string();
    let mut max_dim = None;
    let mut zigzag_bridge_entry = Some(8u32);
    let mut top_k = 6usize;
    let mut probe_lag = None;
    let mut probe_shortlist_k = 4usize;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fixture-ref" => {
                fixture_ref = args.next().ok_or("--fixture-ref requires a value")?;
            }
            "--seeded-guide-id" => {
                seeded_guide_id = args.next().ok_or("--seeded-guide-id requires a value")?;
            }
            "--current" => {
                current_selector = args.next().ok_or("--current requires a value")?;
            }
            "--target" => {
                target_selector = args.next().ok_or("--target requires a value")?;
            }
            "--max-dim" => {
                max_dim = Some(
                    args.next()
                        .ok_or("--max-dim requires a value")?
                        .parse()
                        .map_err(|_| "invalid --max-dim".to_string())?,
                );
            }
            "--zigzag-bridge-entry" => {
                zigzag_bridge_entry = Some(
                    args.next()
                        .ok_or("--zigzag-bridge-entry requires a value")?
                        .parse()
                        .map_err(|_| "invalid --zigzag-bridge-entry".to_string())?,
                );
            }
            "--no-zigzag" => {
                zigzag_bridge_entry = None;
            }
            "--top-k" => {
                top_k = args
                    .next()
                    .ok_or("--top-k requires a value")?
                    .parse()
                    .map_err(|_| "invalid --top-k".to_string())?;
            }
            "--probe-lag" => {
                probe_lag = Some(
                    args.next()
                        .ok_or("--probe-lag requires a value")?
                        .parse()
                        .map_err(|_| "invalid --probe-lag".to_string())?,
                );
            }
            "--probe-shortlist-k" => {
                probe_shortlist_k = args
                    .next()
                    .ok_or("--probe-shortlist-k requires a value")?
                    .parse()
                    .map_err(|_| "invalid --probe-shortlist-k".to_string())?;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    Ok(Cli {
        fixture_ref,
        seeded_guide_id,
        current_selector,
        target_selector,
        max_dim,
        zigzag_bridge_entry,
        top_k: top_k.max(1),
        probe_lag,
        probe_shortlist_k: probe_shortlist_k.max(1),
    })
}

fn print_usage() {
    println!(
        "usage: compare_graph_move_proposals [options]\n\n\
         options:\n\
           --fixture-ref REF           fixture file/id reference (default: research/fixtures/brix_ruiz_family.json#brix_ruiz_k3)\n\
           --seeded-guide-id ID        seeded guide id for guide:N selectors (default: endpoint_16_path)\n\
           --current SELECTOR          source | target | guide:N (default: source)\n\
           --target SELECTOR           source | target | guide:N (default: target)\n\
           --max-dim N                 max dimension for blind/proposal generation\n\
           --zigzag-bridge-entry N     enable bounded 3x3 zig-zag proposals (default: 8)\n\
           --no-zigzag                 disable zig-zag proposal generation\n\
           --top-k N                   number of top candidates to print per surface (default: 6)\n\
           --probe-lag N               graph-only lag bound for realizing best-gap proposals\n\
           --probe-shortlist-k N       cap the probed best-gap shortlist (default: 4)"
    );
}

fn print_blind_summary(
    blind: &sse_core::graph_moves::GraphMoveSuccessors,
    blind_scored: &[ScoredBlindSuccessor],
) {
    println!("Blind one-step graph successors");
    println!("  raw candidates: {}", blind.candidates);
    println!("  unique canonical successors: {}", blind.nodes.len());
    println!(
        "  dimension breakdown: {}",
        format_dimension_breakdown(blind_scored.iter().map(|candidate| &candidate.matrix))
    );
    println!(
        "  family counts: {}",
        format_family_counts(&blind.family_candidates)
    );
    if let Some(best_gap) = blind_scored.first().map(|candidate| candidate.gap) {
        println!("  best target signature gap: {}", format_gap(best_gap));
        println!(
            "  best-gap shortlist: {}",
            blind_scored
                .iter()
                .take_while(|candidate| candidate.gap == best_gap)
                .count()
        );
    }
}

fn print_proposal_summary(proposals: &sse_core::graph_moves::GraphProposals, blind_overlap: usize) {
    println!("Targeted graph proposals");
    println!("  raw proposal candidates: {}", proposals.candidates);
    println!("  unique canonical proposals: {}", proposals.nodes.len());
    println!(
        "  dimension breakdown: {}",
        format_dimension_breakdown(proposals.nodes.iter().map(|proposal| &proposal.matrix))
    );
    println!(
        "  family counts: {}",
        format_family_counts(&proposals.family_candidates)
    );
    println!(
        "  overlap with blind one-step successors: {}",
        blind_overlap
    );
    if let Some(best_gap) = proposals.best_gap() {
        println!("  best target signature gap: {}", format_gap(best_gap));
        println!(
            "  best-gap shortlist: {}",
            proposals.best_gap_shortlist_len()
        );
    }
}

fn print_top_blind_successors(blind_scored: &[ScoredBlindSuccessor], top_k: usize) {
    println!("Top blind successors");
    if blind_scored.is_empty() {
        println!("  none");
        return;
    }

    for (index, candidate) in blind_scored.iter().take(top_k).enumerate() {
        println!(
            "  {}. family={} gap={} {:?}",
            index + 1,
            candidate.family,
            format_gap(candidate.gap),
            candidate.matrix,
        );
    }
}

fn print_top_proposals(proposals: &[GraphProposal], top_k: usize) {
    println!("Top targeted proposals");
    if proposals.is_empty() {
        println!("  none");
        return;
    }

    for (index, proposal) in proposals.iter().take(top_k).enumerate() {
        println!(
            "  {}. families={} gap={} {:?}",
            index + 1,
            proposal.families.join(","),
            format_gap(proposal.target_signature_gap),
            proposal.matrix,
        );
    }
}

fn print_proposal_probe(probe: &sse_core::search::GraphProposalProbeResult, probe_lag: usize) {
    println!("Best-gap proposal probe");
    println!("  raw proposal candidates: {}", probe.raw_candidates);
    println!("  unique canonical proposals: {}", probe.unique_candidates);
    if let Some(best_gap) = probe.best_gap {
        println!("  best target signature gap: {}", format_gap(best_gap));
    }
    println!("  best-gap shortlist: {}", probe.best_gap_candidates);
    println!("  probed proposals: {}", probe.attempts.len());
    println!("  realization lag bound: {}", probe_lag);
    if probe.attempts.is_empty() {
        println!("  none");
        return;
    }

    for (index, attempt) in probe.attempts.iter().enumerate() {
        let outcome = match &attempt.result {
            DynSseResult::Equivalent(path) => format!("realized in {} step(s)", path.steps.len()),
            DynSseResult::NotEquivalent(reason) => format!("not equivalent ({reason})"),
            DynSseResult::Unknown => "not realized within bound".to_string(),
        };
        println!(
            "  {}. {} families={} gap={} frontier_nodes={} visited={} {:?}",
            index + 1,
            outcome,
            attempt.proposal.families.join(","),
            format_gap(attempt.proposal.target_signature_gap),
            attempt.telemetry.frontier_nodes_expanded,
            attempt.telemetry.total_visited_nodes,
            attempt.proposal.matrix,
        );
    }
}

fn format_dimension_breakdown<'a>(matrices: impl Iterator<Item = &'a DynMatrix>) -> String {
    let mut counts = BTreeMap::<usize, usize>::new();
    for matrix in matrices {
        *counts.entry(matrix.rows).or_default() += 1;
    }

    if counts.is_empty() {
        return "none".to_string();
    }

    counts
        .into_iter()
        .map(|(dim, count)| format!("{dim}x{dim}:{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_family_counts(counts: &BTreeMap<&'static str, usize>) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }

    counts
        .iter()
        .map(|(family, count)| format!("{family}:{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_gap(gap: SameFuturePastSignatureGap) -> String {
    format!(
        "dim={} row={} col={} entry_sum={}",
        gap.dimension_gap, gap.row_class_gap, gap.col_class_gap, gap.entry_sum_gap
    )
}

fn load_endpoint_fixture(fixture_ref: &str) -> Result<EndpointFixture, String> {
    let (path, fixture_id) = split_fixture_ref(fixture_ref);
    let json = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read fixture {}: {err}", path.display()))?;
    let parsed: EndpointFixtureFile = serde_json::from_str(&json)
        .map_err(|err| format!("failed to parse fixture {}: {err}", path.display()))?;

    let fixture_id = fixture_id.ok_or_else(|| {
        format!(
            "fixture {} must specify path#fixture_id because this file contains multiple fixtures",
            path.display()
        )
    })?;

    parsed
        .fixtures
        .into_iter()
        .find(|fixture| fixture.id == fixture_id)
        .ok_or_else(|| format!("fixture {} not found in {}", fixture_id, path.display()))
}

fn split_fixture_ref(fixture_ref: &str) -> (PathBuf, Option<String>) {
    match fixture_ref.split_once('#') {
        Some((path, fixture_id)) if !fixture_id.is_empty() => {
            (PathBuf::from(path), Some(fixture_id.to_string()))
        }
        _ => (PathBuf::from(fixture_ref), None),
    }
}

fn select_matrix(
    fixture: &EndpointFixture,
    seeded_guide_id: &str,
    selector: &str,
) -> Result<DynMatrix, String> {
    match selector {
        "source" | "endpoint_a" => case_matrix(&fixture.a),
        "target" | "endpoint_b" => case_matrix(&fixture.b),
        _ if selector.starts_with("guide:") => {
            let index: usize = selector["guide:".len()..]
                .parse()
                .map_err(|_| format!("invalid guide selector: {selector}"))?;
            let guide = fixture
                .seeded_guides
                .iter()
                .find(|guide| guide.id == seeded_guide_id)
                .ok_or_else(|| {
                    let available = fixture
                        .seeded_guides
                        .iter()
                        .map(|guide| {
                            guide
                                .label
                                .as_deref()
                                .map(|label| format!("{} ({label})", guide.id))
                                .unwrap_or_else(|| guide.id.clone())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "seeded guide {} not found in fixture {}; available guides: {}",
                        seeded_guide_id,
                        fixture.id,
                        if available.is_empty() {
                            "none".to_string()
                        } else {
                            available
                        }
                    )
                })?;
            let matrix_rows = guide.matrices.get(index).ok_or_else(|| {
                format!(
                    "guide selector {} is out of range for guide {} ({} matrices)",
                    selector,
                    guide.id,
                    guide.matrices.len()
                )
            })?;
            case_matrix(matrix_rows)
        }
        _ => Err(format!(
            "unsupported selector {} (expected source, target, or guide:N)",
            selector
        )),
    }
}

fn case_matrix(rows: &[Vec<u32>]) -> Result<DynMatrix, String> {
    if rows.is_empty() {
        return Err("matrix must not be empty".to_string());
    }
    let cols = rows[0].len();
    if cols == 0 {
        return Err("matrix must not have empty rows".to_string());
    }
    if rows.iter().any(|row| row.len() != cols) {
        return Err("matrix rows must all have the same length".to_string());
    }
    if rows.len() != cols {
        return Err(format!(
            "matrix must be square, got {}x{}",
            rows.len(),
            cols
        ));
    }

    Ok(DynMatrix::new(
        rows.len(),
        cols,
        rows.iter().flat_map(|row| row.iter()).copied().collect(),
    ))
}
