use std::collections::BTreeMap;

use sse_core::conjugacy::{
    derive_positive_conjugacy_proposals_2x2, find_positive_conjugacy_2x2,
    rank_positive_conjugacy_seed_candidates_2x2, PositiveConjugacyProposalConfig2x2,
    PositiveConjugacySearchConfig2x2, PositiveConjugacySearchResult2x2,
    PositiveConjugacySeedCandidate2x2, PositiveConjugacySeedConfig2x2,
};
use sse_core::factorisation::visit_factorisations_with_family_for_policy;
use sse_core::matrix::{DynMatrix, SqMatrix};
use sse_core::search::search_sse_2x2_with_telemetry;
use sse_core::types::{FrontierMode, MoveFamilyPolicy, SearchConfig, SearchTelemetry, SseResult};

#[derive(Clone)]
struct Cli {
    case: String,
    max_conjugator_entry: u32,
    sample_points: usize,
    proposal_top_k: usize,
    seed_top_k: usize,
    local_seed_lag: usize,
    max_lag: usize,
    max_dim: usize,
    max_entry: u32,
}

#[derive(Clone)]
struct Case2x2 {
    name: String,
    description: String,
    source: SqMatrix<2>,
    target: SqMatrix<2>,
}

#[derive(Clone)]
struct LocalSeed {
    matrix: SqMatrix<2>,
    local_lag: usize,
    path_families: Vec<String>,
}

#[derive(Clone)]
struct SeedEvaluation {
    seed: PositiveConjugacySeedCandidate2x2,
    local_lag: usize,
    path_families: Vec<String>,
    result: SseResult<2>,
    telemetry: SearchTelemetry,
}

fn main() -> Result<(), String> {
    let cli = parse_args()?;
    let case = load_case(&cli.case);
    let witness_search = PositiveConjugacySearchConfig2x2 {
        max_conjugator_entry: cli.max_conjugator_entry,
        sample_points: cli.sample_points,
    };
    let proposal_config = PositiveConjugacyProposalConfig2x2 {
        max_proposals: cli.proposal_top_k,
        include_endpoints: false,
    };
    let seed_config = PositiveConjugacySeedConfig2x2 {
        max_candidates: cli.seed_top_k,
    };
    let search_config = SearchConfig {
        max_lag: cli.max_lag,
        max_intermediate_dim: cli.max_dim,
        max_entry: cli.max_entry,
        frontier_mode: FrontierMode::Bfs,
        move_family_policy: MoveFamilyPolicy::Mixed,
        beam_width: None,
    };

    let (direct_result, direct_telemetry) =
        search_sse_2x2_with_telemetry(&case.source, &case.target, &search_config);

    println!("Positive-conjugacy local seed probe");
    println!("Case: {} ({})", case.name, case.description);
    println!("A = {:?}", case.source);
    println!("B = {:?}", case.target);
    println!(
        "Witness search: max_conjugator_entry={}, sample_points={}",
        witness_search.max_conjugator_entry, witness_search.sample_points
    );
    println!(
        "Bounded probe search: lag<= {}, dim<= {}, entry<= {}, moves={}",
        search_config.max_lag,
        search_config.max_intermediate_dim,
        search_config.max_entry,
        search_config.move_family_policy.kebab_case_label()
    );
    println!(
        "Shortlists: proposal_top_k={}, seed_top_k={}, local_seed_lag<={}",
        proposal_config.max_proposals, seed_config.max_candidates, cli.local_seed_lag
    );
    println!();

    println!("Direct bounded baseline:");
    print_result_summary("direct", &direct_result, &direct_telemetry, None);
    println!();

    let witness = match find_positive_conjugacy_2x2(&case.source, &case.target, &witness_search) {
        PositiveConjugacySearchResult2x2::Equivalent(witness) => witness,
        PositiveConjugacySearchResult2x2::Exhausted => {
            println!("No positive-conjugacy witness found under the requested bounds.");
            return Ok(());
        }
    };

    let proposals = derive_positive_conjugacy_proposals_2x2(
        &case.source,
        &case.target,
        &witness,
        &proposal_config,
    );
    println!("Positive-conjugacy witness:");
    println!("  G = {:?}", witness.conjugator);
    println!("  proposal shortlist = {}", proposals.len());
    for (index, proposal) in proposals.iter().enumerate() {
        println!(
            "  P{} {:?} shadow_l1={:.3} t={:.3} endpoint_l1={}",
            index + 1,
            proposal.matrix,
            proposal.shadow_l1_distance,
            proposal.nearest_sample_t,
            proposal.endpoint_l1_distance
        );
    }
    if proposals.is_empty() {
        println!();
        println!("No rounded sampled proposals survived the requested top-k cutoff.");
        return Ok(());
    }
    println!();

    let seeds =
        enumerate_same_dimension_local_seeds(&case.source, &search_config, cli.local_seed_lag);
    println!(
        "Actual local same-dimension seed candidates from A within lag <= {}:",
        cli.local_seed_lag
    );
    println!("  candidates = {}", seeds.len());
    println!(
        "  lag breakdown = {}",
        format_lag_breakdown(seeds.iter().map(|seed| seed.local_lag))
    );
    if seeds.is_empty() {
        println!("  no same-dimension candidates survived under the requested search policy");
        return Ok(());
    }
    println!();

    let seed_matrices = seeds
        .iter()
        .map(|seed| seed.matrix.clone())
        .collect::<Vec<_>>();
    let all_seed_scores = rank_positive_conjugacy_seed_candidates_2x2(
        &case.target,
        &proposals,
        &seed_matrices,
        &PositiveConjugacySeedConfig2x2 {
            max_candidates: seed_matrices.len(),
        },
    );
    let score_by_matrix = all_seed_scores
        .iter()
        .map(|seed| (seed.matrix.clone(), seed.clone()))
        .collect::<BTreeMap<_, _>>();
    let seed_by_matrix = seeds
        .iter()
        .map(|seed| (seed.matrix.clone(), seed.clone()))
        .collect::<BTreeMap<_, _>>();

    let seeded_shortlist = rank_positive_conjugacy_seed_candidates_2x2(
        &case.target,
        &proposals,
        &seed_matrices,
        &seed_config,
    );
    let blind_shortlist = build_blind_shortlist(&seeds, &score_by_matrix, cli.seed_top_k);

    let seeded_evaluations = seeded_shortlist
        .iter()
        .filter_map(|seed| {
            seed_by_matrix
                .get(&seed.matrix)
                .map(|local_seed| evaluate_seed(local_seed, seed, &case.target, &search_config))
        })
        .collect::<Vec<_>>();
    let blind_evaluations = blind_shortlist
        .iter()
        .filter_map(|seed| {
            seed_by_matrix
                .get(&seed.matrix)
                .map(|local_seed| evaluate_seed(local_seed, seed, &case.target, &search_config))
        })
        .collect::<Vec<_>>();

    println!("Proposal-guided local seed shortlist:");
    print_seed_evaluations("S", &seeded_evaluations);
    println!();
    println!("Blind target-nearest control shortlist:");
    print_seed_evaluations("C", &blind_evaluations);
    println!();

    print_comparison_summary(&seeded_evaluations, &blind_evaluations);

    Ok(())
}

fn parse_args() -> Result<Cli, String> {
    let mut case = "brix_k3".to_string();
    let mut max_conjugator_entry = 4u32;
    let mut sample_points = 64usize;
    let mut proposal_top_k = 4usize;
    let mut seed_top_k = 4usize;
    let mut local_seed_lag = 2usize;
    let mut max_lag = 4usize;
    let mut max_dim = 4usize;
    let mut max_entry = 8u32;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().ok_or("--case requires a value")?;
            }
            "--max-conjugator-entry" => {
                max_conjugator_entry = args
                    .next()
                    .ok_or("--max-conjugator-entry requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-conjugator-entry".to_string())?;
            }
            "--sample-points" => {
                sample_points = args
                    .next()
                    .ok_or("--sample-points requires a value")?
                    .parse()
                    .map_err(|_| "invalid --sample-points".to_string())?;
            }
            "--proposal-top-k" => {
                proposal_top_k = args
                    .next()
                    .ok_or("--proposal-top-k requires a value")?
                    .parse()
                    .map_err(|_| "invalid --proposal-top-k".to_string())?;
            }
            "--seed-top-k" => {
                seed_top_k = args
                    .next()
                    .ok_or("--seed-top-k requires a value")?
                    .parse()
                    .map_err(|_| "invalid --seed-top-k".to_string())?;
            }
            "--local-seed-lag" => {
                local_seed_lag = args
                    .next()
                    .ok_or("--local-seed-lag requires a value")?
                    .parse()
                    .map_err(|_| "invalid --local-seed-lag".to_string())?;
            }
            "--max-lag" => {
                max_lag = args
                    .next()
                    .ok_or("--max-lag requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-lag".to_string())?;
            }
            "--max-dim" => {
                max_dim = args
                    .next()
                    .ok_or("--max-dim requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-dim".to_string())?;
            }
            "--max-entry" => {
                max_entry = args
                    .next()
                    .ok_or("--max-entry requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-entry".to_string())?;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    Ok(Cli {
        case,
        max_conjugator_entry,
        sample_points,
        proposal_top_k: proposal_top_k.max(1),
        seed_top_k: seed_top_k.max(1),
        local_seed_lag,
        max_lag,
        max_dim,
        max_entry,
    })
}

fn print_usage() {
    println!(
        "usage: probe_positive_conjugacy_seeds [options]\n\n\
         options:\n\
           --case CASE                 case name (default: brix_k3)\n\
           --max-conjugator-entry N    bounded positive-conjugacy search cap (default: 4)\n\
           --sample-points N           positive-conjugacy path samples (default: 64)\n\
           --proposal-top-k N          positive-conjugacy proposal shortlist size (default: 4)\n\
           --seed-top-k N              actual local seed shortlist size (default: 4)\n\
           --local-seed-lag N          local exact 2x2 seed lag cap from the source (default: 2)\n\
           --max-lag N                 total bounded search lag for baseline and suffix runs (default: 4)\n\
           --max-dim N                 max intermediate dimension for local/suffix search (default: 4)\n\
           --max-entry N               max entry cap for local/suffix search (default: 8)"
    );
}

fn load_case(case: &str) -> Case2x2 {
    match case {
        "brix_k3" => Case2x2 {
            name: "brix_k3".to_string(),
            description: "Brix-Ruiz witness-known calibration, k=3".to_string(),
            source: SqMatrix::new([[1, 3], [2, 1]]),
            target: SqMatrix::new([[1, 6], [1, 1]]),
        },
        "brix_k4" => Case2x2 {
            name: "brix_k4".to_string(),
            description: "Brix-Ruiz witness-known calibration, k=4".to_string(),
            source: SqMatrix::new([[1, 4], [3, 1]]),
            target: SqMatrix::new([[1, 12], [1, 1]]),
        },
        "simple_diag" => Case2x2 {
            name: "simple_diag".to_string(),
            description: "simple diagonal scaling calibration".to_string(),
            source: SqMatrix::new([[1, 2], [2, 1]]),
            target: SqMatrix::new([[1, 4], [1, 1]]),
        },
        _ => load_riedel_baker_case(case).unwrap_or_else(|| panic!("unsupported case: {case}")),
    }
}

fn load_riedel_baker_case(case: &str) -> Option<Case2x2> {
    let k = case.strip_prefix("riedel_baker_k")?.parse::<u32>().ok()?;
    if k < 2 {
        return None;
    }

    Some(Case2x2 {
        name: case.to_string(),
        description: format!(
            "Riedel/Baker literature family, k={} (Boyle-Schmieding Example `riedelexample`)",
            k
        ),
        source: SqMatrix::new([[k, 2], [1, k]]),
        target: SqMatrix::new([[k - 1, 1], [1, k + 1]]),
    })
}

fn enumerate_same_dimension_local_seeds(
    source: &SqMatrix<2>,
    search_config: &SearchConfig,
    max_local_lag: usize,
) -> Vec<LocalSeed> {
    if max_local_lag == 0 {
        return Vec::new();
    }

    let mut best_by_matrix = BTreeMap::<SqMatrix<2>, LocalSeed>::new();
    let mut best_depth = BTreeMap::<SqMatrix<2>, usize>::new();
    best_depth.insert(source.clone(), 0);

    let mut frontier = vec![LocalSeed {
        matrix: source.clone(),
        local_lag: 0,
        path_families: Vec::new(),
    }];

    for next_depth in 1..=max_local_lag {
        let mut next_frontier = Vec::new();
        for current in frontier {
            for (matrix, move_family) in
                enumerate_direct_same_dimension_successors(&current.matrix, search_config)
            {
                if best_depth
                    .get(&matrix)
                    .is_some_and(|best| *best <= next_depth)
                {
                    continue;
                }

                let mut path_families = current.path_families.clone();
                path_families.push(move_family);
                best_depth.insert(matrix.clone(), next_depth);
                let successor = LocalSeed {
                    matrix: matrix.clone(),
                    local_lag: next_depth,
                    path_families,
                };
                best_by_matrix
                    .entry(matrix.clone())
                    .or_insert_with(|| successor.clone());
                next_frontier.push(successor);
            }
        }

        frontier = next_frontier;
        if frontier.is_empty() {
            break;
        }
    }

    best_by_matrix.into_values().collect()
}

fn enumerate_direct_same_dimension_successors(
    source: &SqMatrix<2>,
    search_config: &SearchConfig,
) -> Vec<(SqMatrix<2>, String)> {
    let source_dyn = DynMatrix::from_sq(source);
    let mut by_matrix = BTreeMap::<SqMatrix<2>, String>::new();
    visit_factorisations_with_family_for_policy(
        &source_dyn,
        search_config.max_intermediate_dim,
        search_config.max_entry,
        search_config.move_family_policy,
        |family, u, v| {
            let next = v.mul(&u);
            if next.rows != 2 || next.cols != 2 || next.max_entry() > search_config.max_entry {
                return;
            }
            let Some(matrix) = next.to_sq::<2>() else {
                return;
            };
            if matrix == *source {
                return;
            }
            by_matrix
                .entry(matrix)
                .or_insert_with(|| family.to_string());
        },
    );
    by_matrix.into_iter().collect()
}

fn build_blind_shortlist(
    seeds: &[LocalSeed],
    score_by_matrix: &BTreeMap<SqMatrix<2>, PositiveConjugacySeedCandidate2x2>,
    limit: usize,
) -> Vec<PositiveConjugacySeedCandidate2x2> {
    let mut ranked = seeds
        .iter()
        .filter_map(|seed| score_by_matrix.get(&seed.matrix).cloned())
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        left.target_l1_distance
            .cmp(&right.target_l1_distance)
            .then(left.proposal_l1_distance.cmp(&right.proposal_l1_distance))
            .then(left.nearest_proposal_rank.cmp(&right.nearest_proposal_rank))
            .then(left.matrix.max_entry().cmp(&right.matrix.max_entry()))
            .then(left.matrix.cmp(&right.matrix))
    });
    ranked.truncate(limit);
    ranked
}

fn evaluate_seed(
    local_seed: &LocalSeed,
    seed: &PositiveConjugacySeedCandidate2x2,
    target: &SqMatrix<2>,
    search_config: &SearchConfig,
) -> SeedEvaluation {
    let candidate_config = SearchConfig {
        max_lag: search_config.max_lag.saturating_sub(local_seed.local_lag),
        ..search_config.clone()
    };
    let (result, telemetry) =
        search_sse_2x2_with_telemetry(&local_seed.matrix, target, &candidate_config);
    SeedEvaluation {
        seed: seed.clone(),
        local_lag: local_seed.local_lag,
        path_families: local_seed.path_families.clone(),
        result,
        telemetry,
    }
}

fn print_result_summary(
    label: &str,
    result: &SseResult<2>,
    telemetry: &SearchTelemetry,
    total_lag_offset: Option<usize>,
) {
    let outcome = match result {
        SseResult::Equivalent(path) => match total_lag_offset {
            Some(offset) => format!("Equivalent, total lag {}", offset + path.steps.len()),
            None => format!("Equivalent, lag {}", path.steps.len()),
        },
        SseResult::EquivalentByConcreteShift(proof) => {
            format!("Equivalent by {}", proof.description())
        }
        SseResult::NotEquivalent(reason) => format!("NotEquivalent ({reason})"),
        SseResult::Unknown => "Unknown".to_string(),
    };
    println!(
        "  {}: {} | expanded={} candidates={} pruned={} factorisations={} max_frontier={}",
        label,
        outcome,
        telemetry.frontier_nodes_expanded,
        telemetry.candidates_generated,
        telemetry.candidates_after_pruning,
        telemetry.factorisations_enumerated,
        telemetry.max_frontier_size
    );
}

fn print_seed_evaluations(prefix: &str, evaluations: &[SeedEvaluation]) {
    if evaluations.is_empty() {
        println!("  none");
        return;
    }

    for (index, evaluation) in evaluations.iter().enumerate() {
        println!(
            "  {}{} {:?} local_lag={} path={} nearest=P{} proposal_l1={} target_l1={}",
            prefix,
            index + 1,
            evaluation.seed.matrix,
            evaluation.local_lag,
            evaluation.path_families.join(" -> "),
            evaluation.seed.nearest_proposal_rank,
            evaluation.seed.proposal_l1_distance,
            evaluation.seed.target_l1_distance
        );
        print_result_summary(
            "suffix",
            &evaluation.result,
            &evaluation.telemetry,
            Some(evaluation.local_lag),
        );
    }
}

fn print_comparison_summary(seeded: &[SeedEvaluation], blind: &[SeedEvaluation]) {
    println!("Summary:");
    println!("  seeded: {}", summarize_attempts(seeded));
    println!("  blind: {}", summarize_attempts(blind));

    let seeded_best = best_total_lag(seeded);
    let blind_best = best_total_lag(blind);
    match (seeded_best, blind_best) {
        (Some(seed_lag), Some(blind_lag)) if seed_lag < blind_lag => {
            println!(
                "  bounded improvement: seeded shortlist found a strictly shorter realized total lag ({seed_lag} < {blind_lag})"
            );
        }
        (Some(seed_lag), Some(blind_lag)) if seed_lag > blind_lag => {
            println!(
                "  bounded improvement: none; blind controls realized a shorter total lag ({blind_lag} < {seed_lag})"
            );
        }
        (Some(seed_lag), Some(_)) => {
            println!(
                "  bounded improvement: none; seeded and blind realized the same best total lag ({seed_lag})"
            );
        }
        (Some(seed_lag), None) => {
            println!(
                "  bounded improvement: seeded shortlist realized a bounded suffix while blind controls did not (best total lag {seed_lag})"
            );
        }
        (None, Some(blind_lag)) => {
            println!(
                "  bounded improvement: none; blind controls realized a bounded suffix while seeded candidates did not (best total lag {blind_lag})"
            );
        }
        (None, None) => {
            println!("  bounded improvement: none observed under the requested lag bound");
        }
    }
}

fn summarize_attempts(attempts: &[SeedEvaluation]) -> String {
    let realized = attempts
        .iter()
        .filter(|attempt| matches!(attempt.result, SseResult::Equivalent(_)))
        .count();
    let concrete_shift = attempts
        .iter()
        .filter(|attempt| matches!(attempt.result, SseResult::EquivalentByConcreteShift(_)))
        .count();
    let best = best_total_lag(attempts)
        .map(|lag| lag.to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "{} attempts, {} realized path suffixes, {} concrete-shift proofs, best_total_lag={}",
        attempts.len(),
        realized,
        concrete_shift,
        best
    )
}

fn best_total_lag(attempts: &[SeedEvaluation]) -> Option<usize> {
    attempts
        .iter()
        .filter_map(|attempt| match &attempt.result {
            SseResult::Equivalent(path) => Some(attempt.local_lag + path.steps.len()),
            _ => None,
        })
        .min()
}

fn format_lag_breakdown(lags: impl Iterator<Item = usize>) -> String {
    let mut counts = BTreeMap::<usize, usize>::new();
    for lag in lags {
        *counts.entry(lag).or_default() += 1;
    }
    if counts.is_empty() {
        return "none".to_string();
    }
    counts
        .into_iter()
        .map(|(lag, count)| format!("{lag}:{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}
