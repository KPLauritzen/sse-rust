use serde::Serialize;
use sse_core::concrete_shift::{
    concrete_shift_proposal_data_2x2, search_concrete_shift_equivalence_2x2,
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
    bridge_sample_limit: usize,
}

#[derive(Debug)]
enum CliAction {
    Run(Cli),
    Help,
}

/// Schema v2 renames negative bounded exhaustion from `exhausted` to
/// `bounded_exhausted` in `result_status`.
const REPORT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Serialize)]
struct Report {
    schema_version: u32,
    artifact_kind: &'static str,
    relation: &'static str,
    witness_class: &'static str,
    search_restriction: &'static str,
    bounds: ConcreteShiftProposalBounds2x2,
    bridge_sample_limit: usize,
    cases: Vec<CaseReport>,
}

#[derive(Debug, Serialize)]
struct CaseReport {
    case_id: String,
    description: &'static str,
    source: [[u32; 2]; 2],
    target: [[u32; 2]; 2],
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

    Ok(CliAction::Run(Cli {
        case_ids,
        surface,
        relation,
        bounds,
        bridge_sample_limit,
    }))
}

fn usage() -> &'static str {
    "Usage: report_concrete_shift_proposals [--case CASE ...]\
\n       [--boolean-bridge-aligned]\
\n       [--relation aligned|balanced|compatible] [--max-lag N] [--max-entry N]\
\n       [--max-witnesses N] [--bridge-sample-limit N] [--list-cases]"
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
    ]
}

fn run_case(
    case: ControlCase2x2,
    config: &ConcreteShiftSearchConfig2x2,
    bounds: &ConcreteShiftProposalBounds2x2,
    bridge_sample_limit: usize,
    surface: ReportSurface,
) -> Result<CaseReport, String> {
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
        result_status,
        proposal,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        available_cases, parse_cli, run_case, CliAction, ControlCase2x2, ReportSurface,
        REPORT_SCHEMA_VERSION,
    };
    use sse_core::concrete_shift::{ConcreteShiftProposalBounds2x2, ConcreteShiftSearchConfig2x2};
    use sse_core::matrix::SqMatrix;

    #[test]
    fn parse_cli_defaults_to_nontrivial_control() {
        let CliAction::Run(cli) = parse_cli(Vec::<String>::new().into_iter()).unwrap() else {
            panic!("expected runnable cli");
        };
        assert_eq!(cli.case_ids, vec!["lag_one_shortcut_control"]);
        assert_eq!(cli.bridge_sample_limit, 2);
        assert_eq!(cli.bounds.max_lag, 1);
    }

    #[test]
    fn report_schema_version_tracks_bounded_status_rename() {
        assert_eq!(REPORT_SCHEMA_VERSION, 2);
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

        for case in available_cases() {
            let report = run_case(
                case.clone(),
                &config,
                &bounds,
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

        let report = run_case(case, &config, &bounds, 1, ReportSurface::General)
            .expect("expected case report");

        assert_eq!(report.result_status, "bounded_exhausted");
        assert!(report.proposal.is_none());
    }

    #[test]
    fn boolean_bridge_surface_labels_negative_exhaustion_as_bounded() {
        let (case, bounds, config) = bounded_exhausted_control();

        let report = run_case(
            case,
            &config,
            &bounds,
            1,
            ReportSurface::BooleanBridgeAligned,
        )
        .expect("expected case report");

        assert_eq!(report.result_status, "bounded_exhausted");
        assert!(report.proposal.is_none());
    }

    #[test]
    fn parse_cli_treats_help_as_successful_action() {
        let action = parse_cli(vec!["--help".to_string()].into_iter()).unwrap();
        assert!(matches!(action, CliAction::Help));
    }
}
