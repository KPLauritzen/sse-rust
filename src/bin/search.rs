use std::process::ExitCode;

use sse_core::matrix::{DynMatrix, SqMatrix};
use sse_core::search::{
    search_sse_2x2_with_telemetry, search_sse_2x2_with_telemetry_and_observer,
    search_sse_with_telemetry_dyn, search_sse_with_telemetry_dyn_and_observer,
};
#[cfg(not(target_arch = "wasm32"))]
use sse_core::sqlite_graph::SqliteGraphRecorder;
use sse_core::types::{DynSseResult, SearchConfig, SearchMode, SearchTelemetry, SseResult};

#[derive(Debug)]
struct Cli {
    a: DynMatrix,
    b: DynMatrix,
    config: SearchConfig,
    json: bool,
    telemetry: bool,
    visited_db: Option<String>,
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
    let a_sq = cli.a.to_sq::<2>();
    let b_sq = cli.b.to_sq::<2>();

    if let Some(path) = cli.visited_db.as_deref() {
        if let (Some(a), Some(b)) = (a_sq.as_ref(), b_sq.as_ref()) {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut recorder = SqliteGraphRecorder::new(path)?;
                let (result, telemetry) = search_sse_2x2_with_telemetry_and_observer(
                    a,
                    b,
                    &cli.config,
                    Some(&mut recorder),
                );
                if let Some(err) = recorder.error() {
                    return Err(format!("failed to persist visited graph to {path}: {err}"));
                }
                if cli.json {
                    print_json_2x2(a, b, &result, &telemetry, cli.telemetry);
                } else {
                    print_pretty_2x2(a, b, &result, &telemetry, cli.telemetry);
                }
                return exit_code_2x2(result);
            }
            #[cfg(target_arch = "wasm32")]
            {
                return Err("--visited-db is not supported on wasm32 targets".to_string());
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut recorder = SqliteGraphRecorder::new(path)?;
            let (result, telemetry) = search_sse_with_telemetry_dyn_and_observer(
                &cli.a,
                &cli.b,
                &cli.config,
                Some(&mut recorder),
            );
            if let Some(err) = recorder.error() {
                return Err(format!("failed to persist visited graph to {path}: {err}"));
            }
            if cli.json {
                print_json_dyn(&cli.a, &cli.b, &result, &telemetry, cli.telemetry);
            } else {
                print_pretty_dyn(&cli.a, &cli.b, &result, &telemetry, cli.telemetry);
            }
            return exit_code_dyn(result);
        }
        #[cfg(target_arch = "wasm32")]
        {
            return Err("--visited-db is not supported on wasm32 targets".to_string());
        }
    }

    if let (Some(a), Some(b)) = (a_sq.as_ref(), b_sq.as_ref()) {
        let (result, telemetry) = search_sse_2x2_with_telemetry(a, b, &cli.config);
        if cli.json {
            print_json_2x2(a, b, &result, &telemetry, cli.telemetry);
        } else {
            print_pretty_2x2(a, b, &result, &telemetry, cli.telemetry);
        }
        exit_code_2x2(result)
    } else {
        let (result, telemetry) = search_sse_with_telemetry_dyn(&cli.a, &cli.b, &cli.config);
        if cli.json {
            print_json_dyn(&cli.a, &cli.b, &result, &telemetry, cli.telemetry);
        } else {
            print_pretty_dyn(&cli.a, &cli.b, &result, &telemetry, cli.telemetry);
        }
        exit_code_dyn(result)
    }
}

fn exit_code_2x2(result: SseResult<2>) -> Result<ExitCode, String> {
    match result {
        SseResult::Equivalent(_) | SseResult::EquivalentByConcreteShift(_) => Ok(ExitCode::SUCCESS),
        SseResult::NotEquivalent(_) => Ok(ExitCode::from(1)),
        SseResult::Unknown => Ok(ExitCode::from(3)),
    }
}

fn exit_code_dyn(result: DynSseResult) -> Result<ExitCode, String> {
    match result {
        DynSseResult::Equivalent(_) => Ok(ExitCode::SUCCESS),
        DynSseResult::NotEquivalent(_) => Ok(ExitCode::from(1)),
        DynSseResult::Unknown => Ok(ExitCode::from(3)),
    }
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut a: Option<DynMatrix> = None;
    let mut b: Option<DynMatrix> = None;
    let mut config = SearchConfig::default();
    let mut json = false;
    let mut telemetry = false;
    let mut visited_db = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                return Err("usage: search <A> <B> [options]\n\n\
                     Matrices are given either as 4 comma-separated 2x2 entries,\n\
                     or as NxN-prefixed row-major data:\n\
                     \n\
                       search 1,2,3,4 5,6,7,8\n\
                       search 3x3:0,1,0,1,0,1,0,1,0 4x4:...\n\
                     \n\
                     Options:\n\
                       --max-lag N              max elementary SSE steps (default: 4)\n\
                       --max-intermediate-dim N max intermediate dimension (default: 2)\n\
                       --max-entry N            max entry value in U,V (default: 25)\n\
                       --search-mode MODE       mixed | graph-only (default: mixed)\n\
                       --visited-db PATH        write visited nodes and SSE edges to a sqlite db (2x2 endpoints only)\n\
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
            "--visited-db" => {
                visited_db = Some(args.next().ok_or("--visited-db requires a path")?);
            }
            "--json" => json = true,
            "--telemetry" => telemetry = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            positional => {
                let mat = parse_matrix(positional)?;
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
        visited_db,
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
    let (rows, cols) = s
        .split_once('x')
        .ok_or_else(|| format!("invalid matrix prefix '{s}' (expected NxN)"))?;
    let rows: usize = rows
        .parse()
        .map_err(|err| format!("invalid row count in '{s}': {err}"))?;
    let cols: usize = cols
        .parse()
        .map_err(|err| format!("invalid column count in '{s}': {err}"))?;
    Ok((rows, cols))
}

fn parse_entries(s: &str) -> Result<Vec<u32>, String> {
    s.split(',')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|err| format!("invalid matrix entry '{part}': {err}"))
        })
        .collect()
}

fn print_pretty_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    result: &SseResult<2>,
    telemetry: &SearchTelemetry,
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
        print_telemetry(telemetry);
    }
}

fn print_pretty_dyn(
    a: &DynMatrix,
    b: &DynMatrix,
    result: &DynSseResult,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) {
    println!("A = {}", format_dyn_matrix(a));
    println!("B = {}", format_dyn_matrix(b));
    println!();

    match result {
        DynSseResult::Equivalent(path) => {
            println!("Result: EQUIVALENT ({} step(s))", path.steps.len());
            println!();
            for (i, step) in path.steps.iter().enumerate() {
                println!("Step {}:", i + 1);
                println!("  U = {}", format_dyn_matrix(&step.u));
                println!("  V = {}", format_dyn_matrix(&step.v));
            }
        }
        DynSseResult::NotEquivalent(reason) => {
            println!("Result: NOT EQUIVALENT");
            println!("Reason: {reason}");
        }
        DynSseResult::Unknown => {
            println!("Result: UNKNOWN (search exhausted)");
        }
    }

    if show_telemetry {
        print_telemetry(telemetry);
    }
}

fn print_telemetry(telemetry: &SearchTelemetry) {
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

fn print_json_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    result: &SseResult<2>,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) {
    let (outcome, steps, reason) = match result {
        SseResult::Equivalent(path) => (
            "equivalent",
            Some(
                path.steps
                    .iter()
                    .map(step_json)
                    .collect::<Vec<serde_json::Value>>(),
            ),
            None,
        ),
        SseResult::EquivalentByConcreteShift(_) => ("equivalent_by_concrete_shift", None, None),
        SseResult::NotEquivalent(reason) => ("not_equivalent", None, Some(reason.clone())),
        SseResult::Unknown => ("unknown", None, None),
    };

    print_json_value(
        serde_json::json!(a.data),
        serde_json::json!(b.data),
        outcome,
        steps,
        reason,
        telemetry,
        show_telemetry,
    );
}

fn print_json_dyn(
    a: &DynMatrix,
    b: &DynMatrix,
    result: &DynSseResult,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) {
    let (outcome, steps, reason) = match result {
        DynSseResult::Equivalent(path) => (
            "equivalent",
            Some(
                path.steps
                    .iter()
                    .map(step_json)
                    .collect::<Vec<serde_json::Value>>(),
            ),
            None,
        ),
        DynSseResult::NotEquivalent(reason) => ("not_equivalent", None, Some(reason.clone())),
        DynSseResult::Unknown => ("unknown", None, None),
    };

    print_json_value(
        serde_json::json!(dyn_matrix_to_vecs(a)),
        serde_json::json!(dyn_matrix_to_vecs(b)),
        outcome,
        steps,
        reason,
        telemetry,
        show_telemetry,
    );
}

fn print_json_value(
    a: serde_json::Value,
    b: serde_json::Value,
    outcome: &str,
    steps: Option<Vec<serde_json::Value>>,
    reason: Option<String>,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) {
    let mut obj = serde_json::json!({
        "a": a,
        "b": b,
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

fn step_json(step: &sse_core::types::EsseStep) -> serde_json::Value {
    serde_json::json!({
        "u": dyn_matrix_to_vecs(&step.u),
        "v": dyn_matrix_to_vecs(&step.v),
    })
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

#[cfg(test)]
mod tests {
    use super::parse_matrix;

    #[test]
    fn parse_bare_2x2_matrix() {
        let matrix = parse_matrix("1,2,3,4").unwrap();
        assert_eq!(matrix.rows, 2);
        assert_eq!(matrix.cols, 2);
        assert_eq!(matrix.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn parse_prefixed_square_matrix() {
        let matrix = parse_matrix("3x3:0,1,0,1,0,1,0,1,0").unwrap();
        assert_eq!(matrix.rows, 3);
        assert_eq!(matrix.cols, 3);
        assert_eq!(matrix.data, vec![0, 1, 0, 1, 0, 1, 0, 1, 0]);
    }
}
