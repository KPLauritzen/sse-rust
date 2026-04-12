use std::process::ExitCode;

use sse_core::matrix::{DynMatrix, SqMatrix};
use sse_core::search::search_sse_2x2_with_telemetry;
use sse_core::types::{SearchConfig, SearchMode, SseResult};

#[derive(Debug)]
struct Cli {
    a: SqMatrix<2>,
    b: SqMatrix<2>,
    config: SearchConfig,
    json: bool,
    telemetry: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let cli = parse_cli(std::env::args().skip(1))?;

    let (result, telemetry) = search_sse_2x2_with_telemetry(&cli.a, &cli.b, &cli.config);

    if cli.json {
        print_json(&cli.a, &cli.b, &result, &telemetry, cli.telemetry)
    } else {
        print_pretty(&cli.a, &cli.b, &result, &telemetry, cli.telemetry)
    }

    match result {
        SseResult::Equivalent(_) | SseResult::EquivalentByConcreteShift(_) => Ok(ExitCode::SUCCESS),
        SseResult::NotEquivalent(_) => Ok(ExitCode::from(1)),
        SseResult::Unknown => Ok(ExitCode::from(3)),
    }
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut a: Option<SqMatrix<2>> = None;
    let mut b: Option<SqMatrix<2>> = None;
    let mut config = SearchConfig::default();
    let mut json = false;
    let mut telemetry = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                return Err("usage: search <A> <B> [options]\n\n\
                     Matrices are given as 4 comma-separated entries in row-major order:\n\
                     \n\
                       search 1,2,3,4 5,6,7,8\n\
                     \n\
                     Options:\n\
                       --max-lag N              max elementary SSE steps (default: 4)\n\
                       --max-intermediate-dim N max intermediate dimension (default: 2)\n\
                       --max-entry N            max entry value in U,V (default: 25)\n\
                       --search-mode MODE       mixed | graph-only (default: mixed)\n\
                       --json                   output JSON instead of human-readable text\n\
                       --telemetry              include search telemetry in output"
                    .to_string());
            }
            "--max-lag" => {
                config.max_lag = next_parsed(&mut args, "--max-lag")?;
            }
            "--max-intermediate-dim" => {
                config.max_intermediate_dim = next_parsed(&mut args, "--max-intermediate-dim")?;
            }
            "--max-entry" => {
                config.max_entry = next_parsed(&mut args, "--max-entry")?;
            }
            "--search-mode" => {
                let value = args.next().ok_or("--search-mode requires a value")?;
                config.search_mode = match value.as_str() {
                    "mixed" => SearchMode::Mixed,
                    "graph-only" | "graph_only" => SearchMode::GraphOnly,
                    _ => return Err(format!("unknown search mode: {value}")),
                };
            }
            "--json" => json = true,
            "--telemetry" => telemetry = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            positional => {
                let mat = parse_matrix_2x2(positional)?;
                if a.is_none() {
                    a = Some(mat);
                } else if b.is_none() {
                    b = Some(mat);
                } else {
                    return Err(
                        "too many positional arguments (expected exactly 2 matrices)".to_string(),
                    );
                }
            }
        }
    }

    let a = a.ok_or("missing matrix A (first positional argument)")?;
    let b = b.ok_or("missing matrix B (second positional argument)")?;

    Ok(Cli {
        a,
        b,
        config,
        json,
        telemetry,
    })
}

fn next_parsed<I, T>(args: &mut I, flag: &str) -> Result<T, String>
where
    I: Iterator<Item = String>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse()
        .map_err(|err| format!("invalid value for {flag}: {err}"))
}

fn parse_matrix_2x2(s: &str) -> Result<SqMatrix<2>, String> {
    let nums: Vec<u32> = s
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|err| format!("invalid matrix entry '{part}': {err}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if nums.len() != 4 {
        return Err(format!(
            "expected 4 comma-separated entries for a 2x2 matrix, got {}",
            nums.len()
        ));
    }

    Ok(SqMatrix::new([[nums[0], nums[1]], [nums[2], nums[3]]]))
}

fn print_pretty(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    result: &SseResult<2>,
    telemetry: &sse_core::types::SearchTelemetry,
    show_telemetry: bool,
) {
    println!("A = {:?}", a);
    println!("B = {:?}", b);
    println!();

    match result {
        SseResult::Equivalent(path) => {
            println!("Result: EQUIVALENT ({} step(s))", path.steps.len());
            println!();
            for (i, step) in path.steps.iter().enumerate() {
                println!("Step {}:", i + 1);
                println!("  U = {}", format_dyn_matrix(&step.u));
                println!("  V = {}", format_dyn_matrix(&step.v));
            }
        }
        SseResult::EquivalentByConcreteShift(_witness) => {
            println!("Result: EQUIVALENT (concrete shift witness)");
        }
        SseResult::NotEquivalent(reason) => {
            println!("Result: NOT EQUIVALENT");
            println!("Reason: {reason}");
        }
        SseResult::Unknown => {
            println!("Result: UNKNOWN (search exhausted)");
        }
    }

    if show_telemetry {
        println!();
        println!("Telemetry:");
        println!("  layers: {}", telemetry.layers.len());
        println!(
            "  frontier nodes expanded: {}",
            telemetry.frontier_nodes_expanded
        );
        println!(
            "  factorisations enumerated: {}",
            telemetry.factorisations_enumerated
        );
        println!(
            "  candidates after pruning: {}",
            telemetry.candidates_after_pruning
        );
        println!("  discovered nodes: {}", telemetry.discovered_nodes);
        println!("  total visited nodes: {}", telemetry.total_visited_nodes);
        println!("  max frontier size: {}", telemetry.max_frontier_size);
    }
}

fn print_json(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    result: &SseResult<2>,
    telemetry: &sse_core::types::SearchTelemetry,
    show_telemetry: bool,
) {
    let (outcome, steps, reason) = match result {
        SseResult::Equivalent(path) => {
            let steps: Vec<serde_json::Value> = path
                .steps
                .iter()
                .map(|step| {
                    serde_json::json!({
                        "u": dyn_matrix_to_vecs(&step.u),
                        "v": dyn_matrix_to_vecs(&step.v),
                    })
                })
                .collect();
            ("equivalent", Some(steps), None)
        }
        SseResult::EquivalentByConcreteShift(_) => ("equivalent_by_concrete_shift", None, None),
        SseResult::NotEquivalent(reason) => ("not_equivalent", None, Some(reason.clone())),
        SseResult::Unknown => ("unknown", None, None),
    };

    let mut obj = serde_json::json!({
        "a": a.data,
        "b": b.data,
        "outcome": outcome,
    });

    if let Some(steps) = steps {
        obj["steps"] = serde_json::json!(steps);
    }
    if let Some(reason) = reason {
        obj["reason"] = serde_json::json!(reason);
    }
    if show_telemetry {
        obj["telemetry"] = serde_json::to_value(telemetry).unwrap_or_default();
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&obj).expect("json serialization")
    );
}

fn format_dyn_matrix(m: &DynMatrix) -> String {
    let rows: Vec<String> = (0..m.rows)
        .map(|r| {
            let entries: Vec<String> = (0..m.cols)
                .map(|c| m.data[r * m.cols + c].to_string())
                .collect();
            format!("[{}]", entries.join(", "))
        })
        .collect();
    format!("[{}]", rows.join(", "))
}

fn dyn_matrix_to_vecs(m: &DynMatrix) -> Vec<Vec<u32>> {
    (0..m.rows)
        .map(|r| (0..m.cols).map(|c| m.data[r * m.cols + c]).collect())
        .collect()
}
