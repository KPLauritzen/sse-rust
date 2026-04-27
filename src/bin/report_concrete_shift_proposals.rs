use serde::Serialize;
use sse_core::concrete_shift::{
    concrete_shift_profile_2x2, concrete_shift_proposal_data_2x2,
    search_concrete_shift_equivalence_2x2, ConcreteShiftProfile2x2, ConcreteShiftProfileConfig2x2,
    ConcreteShiftProfileResiduals2x2, ConcreteShiftProfileStatus2x2,
    ConcreteShiftProposalBounds2x2, ConcreteShiftProposalData2x2, ConcreteShiftRelation2x2,
    ConcreteShiftSearchConfig2x2, ConcreteShiftSearchResult2x2,
};
use sse_core::matrix::SqMatrix;

#[derive(Clone)]
struct ControlCase2x2 {
    id: &'static str,
    description: &'static str,
    source: SqMatrix<2>,
    target: SqMatrix<2>,
}

#[derive(Debug)]
struct Cli {
    case_ids: Vec<String>,
    surface: ReportSurface,
    relation: ConcreteShiftRelation2x2,
    bounds: ConcreteShiftProposalBounds2x2,
    profile_config: ConcreteShiftProfileConfig2x2,
    bridge_sample_limit: usize,
}

#[derive(Debug)]
enum CliAction {
    Run(Cli),
    Help,
}

/// Schema v4 adds bounded per-profile residual counts.
///
/// Schema v3 adds per-case bounded concrete-shift profile telemetry.
///
/// Schema v2 renamed negative bounded exhaustion from `exhausted` to
/// `bounded_exhausted` in `result_status`.
const REPORT_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Serialize)]
struct Report {
    schema_version: u32,
    artifact_kind: &'static str,
    relation: &'static str,
    witness_class: &'static str,
    search_restriction: &'static str,
    bounds: ConcreteShiftProposalBounds2x2,
    profile_config: ProfileConfigReport,
    bridge_sample_limit: usize,
    cases: Vec<CaseReport>,
}

#[derive(Debug, Serialize)]
struct ProfileConfigReport {
    relation: &'static str,
    max_lag: u32,
    max_entry: u32,
    max_witnesses: usize,
}

#[derive(Debug, Serialize)]
struct ProfileReport {
    relation: &'static str,
    max_lag: u32,
    max_entry: u32,
    max_witnesses: usize,
    status: &'static str,
    shift_witnesses: usize,
    concrete_witness_lag: Option<u32>,
    limit_reached: bool,
    residuals: ConcreteShiftProfileResiduals2x2,
}

#[derive(Debug, Serialize)]
struct CaseReport {
    case_id: String,
    description: &'static str,
    source: [[u32; 2]; 2],
    target: [[u32; 2]; 2],
    profile: ProfileReport,
    result_status: &'static str,
    proposal: Option<ConcreteShiftProposalData2x2>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReportSurface {
    General,
    BooleanBridgeAligned,
}

impl ReportSurface {
    fn artifact_kind(self) -> &'static str {
        match self {
            Self::General => "concrete_shift_proposal_report",
            Self::BooleanBridgeAligned => "boolean_bridge_aligned_concrete_shift_proposal_report",
        }
    }

    fn witness_class(self) -> &'static str {
        match self {
            Self::General => "bounded concrete-shift witness surface",
            Self::BooleanBridgeAligned => {
                "restricted boolean-bridge aligned concrete-shift witness class"
            }
        }
    }

    fn search_restriction(self) -> &'static str {
        match self {
            Self::General => "none",
            Self::BooleanBridgeAligned => "relation=aligned and bridge matrices R,S are boolean",
        }
    }

    fn equivalent_status(self) -> &'static str {
        match self {
            Self::General => "equivalent_by_concrete_shift",
            Self::BooleanBridgeAligned => "equivalent_by_boolean_bridge_aligned_concrete_shift",
        }
    }
}

fn main() -> Result<(), String> {
    let cli = match parse_cli(std::env::args().skip(1))? {
        CliAction::Run(cli) => cli,
        CliAction::Help => {
            println!("{}", usage());
            return Ok(());
        }
    };
    let cases = load_cases(&cli.case_ids)?;
    let config = ConcreteShiftSearchConfig2x2 {
        relation: cli.relation,
        max_lag: cli.bounds.max_lag,
        max_entry: cli.bounds.max_entry,
        max_witnesses: cli.bounds.max_witnesses,
    };

    let cases = cases
        .into_iter()
        .map(|case| {
            run_case(
                case,
                &config,
                &cli.bounds,
                &cli.profile_config,
                cli.bridge_sample_limit,
                cli.surface,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let report = Report {
        schema_version: REPORT_SCHEMA_VERSION,
        artifact_kind: cli.surface.artifact_kind(),
        relation: cli.relation.as_str(),
        witness_class: cli.surface.witness_class(),
        search_restriction: cli.surface.search_restriction(),
        bounds: cli.bounds,
        profile_config: profile_config_report(&cli.profile_config),
        bridge_sample_limit: cli.bridge_sample_limit,
        cases,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to serialize proposal report: {err}"))?
    );
    Ok(())
}

fn parse_cli(args: impl Iterator<Item = String>) -> Result<CliAction, String> {
    let mut case_ids = Vec::new();
    let mut surface = ReportSurface::General;
    let mut relation = ConcreteShiftRelation2x2::Aligned;
    let mut relation_set = false;
    let mut bounds = ConcreteShiftProposalBounds2x2 {
        max_lag: 1,
        max_entry: 6,
        max_witnesses: 10_000,
    };
    let mut profile_config = ConcreteShiftProfileConfig2x2::default();
    let mut max_entry_set = false;
    let mut bridge_sample_limit = 2usize;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => case_ids.push(args.next().ok_or("--case requires a value".to_string())?),
            "--relation" => {
                let value = args
                    .next()
                    .ok_or("--relation requires a value".to_string())?;
                let parsed = parse_relation(&value)?;
                if surface == ReportSurface::BooleanBridgeAligned
                    && parsed != ConcreteShiftRelation2x2::Aligned
                {
                    return Err(
                        "--boolean-bridge-aligned only supports --relation aligned".to_string()
                    );
                }
                relation = parsed;
                relation_set = true;
            }
            "--max-lag" => {
                bounds.max_lag = parse_u32_arg(&mut args, "--max-lag")?;
            }
            "--max-entry" => {
                let parsed = parse_u32_arg(&mut args, "--max-entry")?;
                if surface == ReportSurface::BooleanBridgeAligned && parsed != 1 {
                    return Err("--boolean-bridge-aligned only supports --max-entry 1".to_string());
                }
                bounds.max_entry = parsed;
                max_entry_set = true;
            }
            "--max-witnesses" => {
                bounds.max_witnesses = parse_usize_arg(&mut args, "--max-witnesses")?;
            }
            "--profile-relation" => {
                let value = args
                    .next()
                    .ok_or("--profile-relation requires a value".to_string())?;
                profile_config.relation = parse_relation(&value)?;
            }
            "--profile-max-lag" => {
                profile_config.max_lag = parse_u32_arg(&mut args, "--profile-max-lag")?;
            }
            "--profile-max-entry" => {
                profile_config.max_entry = parse_u32_arg(&mut args, "--profile-max-entry")?;
            }
            "--profile-max-witnesses" => {
                profile_config.max_witnesses =
                    parse_usize_arg(&mut args, "--profile-max-witnesses")?;
            }
            "--bridge-sample-limit" => {
                bridge_sample_limit = parse_usize_arg(&mut args, "--bridge-sample-limit")?;
            }
            "--boolean-bridge-aligned" => {
                if relation_set && relation != ConcreteShiftRelation2x2::Aligned {
                    return Err(
                        "--boolean-bridge-aligned only supports --relation aligned".to_string()
                    );
                }
                if max_entry_set && bounds.max_entry != 1 {
                    return Err("--boolean-bridge-aligned only supports --max-entry 1".to_string());
                }
                surface = ReportSurface::BooleanBridgeAligned;
                relation = ConcreteShiftRelation2x2::Aligned;
                bounds.max_entry = 1;
            }
            "--list-cases" => {
                print_cases();
                std::process::exit(0);
            }
            "--help" | "-h" => {
                return Ok(CliAction::Help);
            }
            other => {
                return Err(format!("unrecognized argument: {other}"));
            }
        }
    }

    if case_ids.is_empty() {
        case_ids.push("lag_one_shortcut_control".to_string());
    }
    if bounds.max_lag == 0 {
        return Err("--max-lag must be at least 1".to_string());
    }
    if bounds.max_witnesses == 0 {
        return Err("--max-witnesses must be at least 1".to_string());
    }
    if profile_config.max_lag == 0 {
        return Err("--profile-max-lag must be at least 1".to_string());
    }
    if profile_config.max_witnesses == 0 {
        return Err("--profile-max-witnesses must be at least 1".to_string());
    }

    Ok(CliAction::Run(Cli {
        case_ids,
        surface,
        relation,
        bounds,
        profile_config,
        bridge_sample_limit,
    }))
}

fn usage() -> &'static str {
    "Usage: report_concrete_shift_proposals [--case CASE ...]\
\n       [--boolean-bridge-aligned]\
\n       [--relation aligned|balanced|compatible] [--max-lag N] [--max-entry N]\
\n       [--max-witnesses N]\
\n       [--profile-relation aligned|balanced|compatible] [--profile-max-lag N]\
\n       [--profile-max-entry N] [--profile-max-witnesses N]\
\n       [--bridge-sample-limit N] [--list-cases]"
}

fn parse_relation(value: &str) -> Result<ConcreteShiftRelation2x2, String> {
    match value {
        "aligned" => Ok(ConcreteShiftRelation2x2::Aligned),
        "balanced" => Ok(ConcreteShiftRelation2x2::Balanced),
        "compatible" => Ok(ConcreteShiftRelation2x2::Compatible),
        _ => Err(format!(
            "unsupported relation {value:?}; expected aligned, balanced, or compatible"
        )),
    }
}

fn parse_u32_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<u32, String> {
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse::<u32>()
        .map_err(|err| format!("failed to parse {flag} value {value:?}: {err}"))
}

fn parse_usize_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<usize, String> {
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse::<usize>()
        .map_err(|err| format!("failed to parse {flag} value {value:?}: {err}"))
}

fn print_cases() {
    for case in available_cases() {
        println!("{}\t{}", case.id, case.description);
    }
}

fn load_cases(case_ids: &[String]) -> Result<Vec<ControlCase2x2>, String> {
    let available = available_cases();
    let mut selected = Vec::new();

    for case_id in case_ids {
        match case_id.as_str() {
            "all" => selected.extend(available.clone()),
            _ => {
                let case = available
                    .iter()
                    .find(|case| case.id == case_id)
                    .cloned()
                    .ok_or_else(|| format!("unknown case {case_id:?}"))?;
                selected.push(case);
            }
        }
    }

    Ok(selected)
}

fn available_cases() -> Vec<ControlCase2x2> {
    vec![
        ControlCase2x2 {
            id: "identity",
            description: "identity sanity control",
            source: SqMatrix::identity(),
            target: SqMatrix::identity(),
        },
        ControlCase2x2 {
            id: "lag_one_shortcut_control",
            description:
                "nontrivial lag-1 aligned concrete-shift control from search fallback tests",
            source: SqMatrix::new([[0, 1], [1, 2]]),
            target: SqMatrix::new([[1, 1], [2, 1]]),
        },
        ControlCase2x2 {
            id: "brix_ruiz_k3",
            description: "known Brix-Ruiz k=3 endpoint control from research/cases.json",
            source: SqMatrix::new([[1, 3], [2, 1]]),
            target: SqMatrix::new([[1, 6], [1, 1]]),
        },
        ControlCase2x2 {
            id: "brix_ruiz_k3_seeded_start_transpose",
            description:
                "Brix-Ruiz k=3 source to first 2x2 seeded-guide waypoint from fixture data",
            source: SqMatrix::new([[1, 3], [2, 1]]),
            target: SqMatrix::new([[1, 2], [3, 1]]),
        },
        ControlCase2x2 {
            id: "brix_ruiz_k4_probe",
            description: "open Brix-Ruiz k=4 evidence lane from research/cases.json",
            source: SqMatrix::new([[1, 4], [3, 1]]),
            target: SqMatrix::new([[1, 12], [1, 1]]),
        },
    ]
}

fn run_case(
    case: ControlCase2x2,
    config: &ConcreteShiftSearchConfig2x2,
    bounds: &ConcreteShiftProposalBounds2x2,
    profile_config: &ConcreteShiftProfileConfig2x2,
    bridge_sample_limit: usize,
    surface: ReportSurface,
) -> Result<CaseReport, String> {
    let profile = profile_report(concrete_shift_profile_2x2(
        &case.source,
        &case.target,
        profile_config,
    ));
    let result = search_concrete_shift_equivalence_2x2(&case.source, &case.target, config);
    let (result_status, proposal) = match result {
        ConcreteShiftSearchResult2x2::Equivalent(witness) => (
            surface.equivalent_status(),
            Some(concrete_shift_proposal_data_2x2(
                &case.source,
                &case.target,
                config.relation,
                &witness,
                bounds.clone(),
                bridge_sample_limit,
            )?),
        ),
        ConcreteShiftSearchResult2x2::Exhausted => ("bounded_exhausted", None),
        ConcreteShiftSearchResult2x2::SearchLimitReached => ("search_limit_reached", None),
    };

    Ok(CaseReport {
        case_id: case.id.to_string(),
        description: case.description,
        source: case.source.data,
        target: case.target.data,
        profile,
        result_status,
        proposal,
    })
}

fn profile_config_report(config: &ConcreteShiftProfileConfig2x2) -> ProfileConfigReport {
    ProfileConfigReport {
        relation: config.relation.as_str(),
        max_lag: config.max_lag,
        max_entry: config.max_entry,
        max_witnesses: config.max_witnesses,
    }
}

fn profile_report(profile: ConcreteShiftProfile2x2) -> ProfileReport {
    ProfileReport {
        relation: profile.relation.as_str(),
        max_lag: profile.max_lag,
        max_entry: profile.max_entry,
        max_witnesses: profile.max_witnesses,
        status: profile_status_label(profile.status),
        shift_witnesses: profile.shift_witnesses,
        concrete_witness_lag: profile.concrete_witness_lag,
        limit_reached: profile.limit_reached,
        residuals: profile.residuals,
    }
}

fn profile_status_label(status: ConcreteShiftProfileStatus2x2) -> &'static str {
    match status {
        ConcreteShiftProfileStatus2x2::Equivalent => "equivalent",
        ConcreteShiftProfileStatus2x2::ShiftWitnessOnly => "shift_witness_only",
        ConcreteShiftProfileStatus2x2::Exhausted => "bounded_exhausted",
        ConcreteShiftProfileStatus2x2::SearchLimitReached => "search_limit_reached",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        available_cases, parse_cli, profile_status_label, run_case, CliAction, ControlCase2x2,
        ReportSurface, REPORT_SCHEMA_VERSION,
    };
    use sse_core::concrete_shift::{
        ConcreteShiftProfileConfig2x2, ConcreteShiftProfileStatus2x2,
        ConcreteShiftProposalBounds2x2, ConcreteShiftSearchConfig2x2,
    };
    use sse_core::matrix::SqMatrix;

    #[test]
    fn parse_cli_defaults_to_nontrivial_control() {
        let CliAction::Run(cli) = parse_cli(Vec::<String>::new().into_iter()).unwrap() else {
            panic!("expected runnable cli");
        };
        assert_eq!(cli.case_ids, vec!["lag_one_shortcut_control"]);
        assert_eq!(cli.bridge_sample_limit, 2);
        assert_eq!(cli.bounds.max_lag, 1);
        assert_eq!(cli.profile_config.max_lag, 1);
        assert_eq!(cli.profile_config.max_entry, 1);
        assert_eq!(cli.profile_config.max_witnesses, 32);
    }

    #[test]
    fn report_schema_version_tracks_bounded_status_rename() {
        assert!(REPORT_SCHEMA_VERSION >= 2);
    }

    #[test]
    fn report_schema_version_tracks_profile_telemetry() {
        assert!(REPORT_SCHEMA_VERSION >= 3);
    }

    #[test]
    fn report_schema_version_tracks_profile_residuals() {
        assert_eq!(REPORT_SCHEMA_VERSION, 4);
    }

    #[test]
    fn parse_cli_accepts_relation_and_bounds() {
        let CliAction::Run(cli) = parse_cli(
            vec![
                "--case".to_string(),
                "identity".to_string(),
                "--relation".to_string(),
                "compatible".to_string(),
                "--max-lag".to_string(),
                "2".to_string(),
                "--max-entry".to_string(),
                "3".to_string(),
                "--max-witnesses".to_string(),
                "64".to_string(),
                "--bridge-sample-limit".to_string(),
                "1".to_string(),
                "--profile-relation".to_string(),
                "compatible".to_string(),
                "--profile-max-lag".to_string(),
                "3".to_string(),
                "--profile-max-entry".to_string(),
                "2".to_string(),
                "--profile-max-witnesses".to_string(),
                "128".to_string(),
            ]
            .into_iter(),
        )
        .unwrap() else {
            panic!("expected runnable cli");
        };

        assert_eq!(cli.case_ids, vec!["identity"]);
        assert_eq!(cli.relation.as_str(), "compatible");
        assert_eq!(cli.bounds.max_lag, 2);
        assert_eq!(cli.bounds.max_entry, 3);
        assert_eq!(cli.bounds.max_witnesses, 64);
        assert_eq!(cli.bridge_sample_limit, 1);
        assert_eq!(cli.profile_config.relation.as_str(), "compatible");
        assert_eq!(cli.profile_config.max_lag, 3);
        assert_eq!(cli.profile_config.max_entry, 2);
        assert_eq!(cli.profile_config.max_witnesses, 128);
    }

    #[test]
    fn parse_cli_accepts_boolean_bridge_aligned_surface() {
        let CliAction::Run(cli) = parse_cli(
            vec![
                "--case".to_string(),
                "identity".to_string(),
                "--boolean-bridge-aligned".to_string(),
            ]
            .into_iter(),
        )
        .unwrap() else {
            panic!("expected runnable cli");
        };

        assert_eq!(cli.case_ids, vec!["identity"]);
        assert_eq!(cli.surface, ReportSurface::BooleanBridgeAligned);
        assert_eq!(cli.relation.as_str(), "aligned");
        assert_eq!(cli.bounds.max_entry, 1);
    }

    #[test]
    fn parse_cli_rejects_conflicting_boolean_bridge_relation() {
        let err = parse_cli(
            vec![
                "--boolean-bridge-aligned".to_string(),
                "--relation".to_string(),
                "compatible".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert!(err.contains("--boolean-bridge-aligned"));
        assert!(err.contains("--relation aligned"));
    }

    #[test]
    fn boolean_bridge_aligned_surface_keeps_positive_controls() {
        let config = ConcreteShiftSearchConfig2x2 {
            relation: sse_core::concrete_shift::ConcreteShiftRelation2x2::Aligned,
            max_lag: 1,
            max_entry: 1,
            max_witnesses: 10_000,
        };
        let bounds = ConcreteShiftProposalBounds2x2 {
            max_lag: 1,
            max_entry: 1,
            max_witnesses: 10_000,
        };

        for case_id in ["identity", "lag_one_shortcut_control"] {
            let case = available_cases()
                .into_iter()
                .find(|case| case.id == case_id)
                .expect("positive control should exist");
            let report = run_case(
                case,
                &config,
                &bounds,
                &ConcreteShiftProfileConfig2x2::default(),
                1,
                ReportSurface::BooleanBridgeAligned,
            )
            .expect("expected case report");

            assert_eq!(
                report.result_status,
                "equivalent_by_boolean_bridge_aligned_concrete_shift"
            );

            let proposal = report.proposal.expect("expected proposal");
            assert_eq!(proposal.relation, "aligned");
            assert_eq!(proposal.lag, 1);
            assert!(proposal.bridge_r.max_entry <= 1);
            assert!(proposal.bridge_s.max_entry <= 1);
        }
    }

    #[test]
    fn available_cases_include_brix_ruiz_profile_controls() {
        let case_ids = available_cases()
            .into_iter()
            .map(|case| case.id)
            .collect::<Vec<_>>();

        assert!(case_ids.contains(&"brix_ruiz_k3"));
        assert!(case_ids.contains(&"brix_ruiz_k3_seeded_start_transpose"));
        assert!(case_ids.contains(&"brix_ruiz_k4_probe"));
    }

    #[test]
    fn profile_status_labels_bounded_exhaustion_explicitly() {
        assert_eq!(
            profile_status_label(ConcreteShiftProfileStatus2x2::Exhausted),
            "bounded_exhausted"
        );
    }

    fn bounded_exhausted_control() -> (
        ControlCase2x2,
        ConcreteShiftProposalBounds2x2,
        ConcreteShiftSearchConfig2x2,
    ) {
        let case = ControlCase2x2 {
            id: "bounded_exhausted_control",
            description: "control with no bounded shift witness",
            source: SqMatrix::identity(),
            target: SqMatrix::new([[0, 0], [0, 0]]),
        };
        let bounds = ConcreteShiftProposalBounds2x2 {
            max_lag: 1,
            max_entry: 0,
            max_witnesses: 16,
        };
        let config = ConcreteShiftSearchConfig2x2 {
            relation: sse_core::concrete_shift::ConcreteShiftRelation2x2::Aligned,
            max_lag: bounds.max_lag,
            max_entry: bounds.max_entry,
            max_witnesses: bounds.max_witnesses,
        };
        (case, bounds, config)
    }

    #[test]
    fn general_surface_labels_negative_exhaustion_as_bounded() {
        let (case, bounds, config) = bounded_exhausted_control();

        let report = run_case(
            case,
            &config,
            &bounds,
            &ConcreteShiftProfileConfig2x2::default(),
            1,
            ReportSurface::General,
        )
        .expect("expected case report");

        assert_eq!(report.result_status, "bounded_exhausted");
        assert!(report.proposal.is_none());
        assert_eq!(report.profile.max_lag, 1);
        assert!(report.profile.residuals.r_intertwiner_candidates > 0);
    }

    #[test]
    fn boolean_bridge_surface_labels_negative_exhaustion_as_bounded() {
        let (case, bounds, config) = bounded_exhausted_control();

        let report = run_case(
            case,
            &config,
            &bounds,
            &ConcreteShiftProfileConfig2x2::default(),
            1,
            ReportSurface::BooleanBridgeAligned,
        )
        .expect("expected case report");

        assert_eq!(report.result_status, "bounded_exhausted");
        assert!(report.proposal.is_none());
        assert!(report.profile.residuals.r_intertwiner_candidates > 0);
    }

    #[test]
    fn parse_cli_treats_help_as_successful_action() {
        let action = parse_cli(vec!["--help".to_string()].into_iter()).unwrap();
        assert!(matches!(action, CliAction::Help));
    }
}
