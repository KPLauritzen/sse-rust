use std::env;
use std::fs;

use serde::Serialize;
use sse_core::factorisation::visit_factorisations_with_family_for_policy;
use sse_core::graph_moves::find_exact_graph_move_witness_between;
use sse_core::matrix::DynMatrix;
use sse_core::search::search_sse_dyn;
use sse_core::types::{DynSseResult, EsseStep, FrontierMode, MoveFamilyPolicy, SearchConfig};

const MAX_BRUTE_FORCE_PERMUTATION_DIM: usize = 8;

#[derive(Debug)]
struct Cli {
    from: DynMatrix,
    to: DynMatrix,
    graph_max_lag: usize,
    graph_max_intermediate_dim: usize,
    graph_max_entry: u32,
    factorisation_max_entry: u32,
    write_json: Option<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    source: MatrixReport,
    target: MatrixReport,
    bounds: BoundsReport,
    direct_match: DirectMatchReport,
    graph_only_explanation: GraphOnlyExplanation,
}

#[derive(Debug, Serialize)]
struct MatrixReport {
    dimension: usize,
    entries: Vec<Vec<u32>>,
}

#[derive(Debug, Serialize)]
struct BoundsReport {
    graph_only: GraphBoundsReport,
    factorisation_match_max_entry: u32,
}

#[derive(Debug, Serialize)]
struct GraphBoundsReport {
    max_lag: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
}

#[derive(Debug, Serialize)]
struct DirectMatchReport {
    exact_graph_family: Option<String>,
    graph_plus_structured_families: Vec<String>,
    mixed_families: Vec<String>,
    has_diagonal_refactorization_match: bool,
}

#[derive(Debug, Serialize)]
struct GraphOnlyExplanation {
    outcome: String,
    lag: Option<usize>,
    interpretation: Option<String>,
    matrices: Vec<MatrixReport>,
    hops: Vec<HopReport>,
    negative_result: Option<String>,
}

#[derive(Debug, Serialize)]
struct HopReport {
    step_index: usize,
    from_dimension: usize,
    to_dimension: usize,
    family: String,
    interpretation: String,
    permutation: Option<Vec<usize>>,
    duplicate_row_classes: Vec<Vec<usize>>,
    duplicate_column_classes: Vec<Vec<usize>>,
    witness: StepWitnessReport,
}

#[derive(Debug, Serialize)]
struct StepWitnessReport {
    u: Vec<Vec<u32>>,
    v: Vec<Vec<u32>>,
}

fn main() -> Result<(), String> {
    let cli = parse_cli(env::args().skip(1))?;
    let report = build_report(&cli)?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize report: {err}"))?;

    if let Some(path) = &cli.write_json {
        fs::write(path, format!("{json}\n"))
            .map_err(|err| format!("failed to write JSON report to {path}: {err}"))?;
    }

    println!("{json}");
    Ok(())
}

fn build_report(cli: &Cli) -> Result<Report, String> {
    let graph_plus_structured_families = matching_factorisation_families(
        &cli.from,
        &cli.to,
        cli.from.rows.max(cli.to.rows),
        cli.factorisation_max_entry,
        MoveFamilyPolicy::GraphPlusStructured,
    )?;
    let mixed_families = matching_factorisation_families(
        &cli.from,
        &cli.to,
        cli.from.rows.max(cli.to.rows),
        cli.factorisation_max_entry,
        MoveFamilyPolicy::Mixed,
    )?;
    let exact_graph_family = find_exact_graph_move_witness_between(&cli.from, &cli.to)
        .map(|step| step.family.to_string());

    let graph_result = search_sse_dyn(
        &cli.from,
        &cli.to,
        &SearchConfig {
            max_lag: cli.graph_max_lag,
            max_intermediate_dim: cli.graph_max_intermediate_dim,
            max_entry: cli.graph_max_entry,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        },
    );

    let graph_only_explanation = match graph_result {
        DynSseResult::Equivalent(path) => {
            let hops = path
                .steps
                .iter()
                .enumerate()
                .map(|(step_index, step)| {
                    explain_hop(
                        step_index,
                        &path.matrices[step_index],
                        &path.matrices[step_index + 1],
                        step,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let interpretation = path_interpretation(&hops);

            GraphOnlyExplanation {
                outcome: "equivalent".to_string(),
                lag: Some(path.steps.len()),
                interpretation,
                matrices: path.matrices.iter().map(matrix_report).collect(),
                hops,
                negative_result: None,
            }
        }
        DynSseResult::NotEquivalent(reason) => GraphOnlyExplanation {
            outcome: "not_equivalent".to_string(),
            lag: None,
            interpretation: None,
            matrices: vec![matrix_report(&cli.from), matrix_report(&cli.to)],
            hops: Vec::new(),
            negative_result: Some(reason),
        },
        DynSseResult::Unknown => GraphOnlyExplanation {
            outcome: "unknown".to_string(),
            lag: None,
            interpretation: None,
            matrices: vec![matrix_report(&cli.from), matrix_report(&cli.to)],
            hops: Vec::new(),
            negative_result: Some(
                "no graph-only path found within the requested lag/dimension/entry bounds"
                    .to_string(),
            ),
        },
    };

    Ok(Report {
        source: matrix_report(&cli.from),
        target: matrix_report(&cli.to),
        bounds: BoundsReport {
            graph_only: GraphBoundsReport {
                max_lag: cli.graph_max_lag,
                max_intermediate_dim: cli.graph_max_intermediate_dim,
                max_entry: cli.graph_max_entry,
            },
            factorisation_match_max_entry: cli.factorisation_max_entry,
        },
        direct_match: DirectMatchReport {
            exact_graph_family,
            has_diagonal_refactorization_match: graph_plus_structured_families
                .iter()
                .any(|family| family.starts_with("diagonal_refactorization_")),
            graph_plus_structured_families,
            mixed_families,
        },
        graph_only_explanation,
    })
}

fn explain_hop(
    step_index: usize,
    from: &DynMatrix,
    to: &DynMatrix,
    step: &EsseStep,
) -> Result<HopReport, String> {
    let (family, interpretation, permutation) =
        if let Some(successor) = find_exact_graph_move_witness_between(from, to) {
            (
                successor.family.to_string(),
                move_interpretation(successor.family).to_string(),
                None,
            )
        } else if let Some(successor) = find_exact_graph_move_witness_between(to, from) {
            let family = inverse_graph_family(successor.family).ok_or_else(|| {
                format!(
                    "no inverse family mapping for reverse exact graph move family {}",
                    successor.family
                )
            })?;
            (
                family.to_string(),
                move_interpretation(family).to_string(),
                None,
            )
        } else if let Some(permutation) = find_permutation_relabeling(from, to)? {
            (
                "permutation_relabeling".to_string(),
                "graph_isomorphism".to_string(),
                Some(permutation),
            )
        } else {
            (
                "unclassified_step".to_string(),
                "unclassified".to_string(),
                None,
            )
        };

    let duplicate_row_classes = if family == "insplit" {
        duplicate_row_classes(to)
    } else if family == "in_amalgamation" {
        duplicate_row_classes(from)
    } else {
        Vec::new()
    };
    let duplicate_column_classes = if family == "outsplit" {
        duplicate_column_classes(to)
    } else if family == "out_amalgamation" {
        duplicate_column_classes(from)
    } else {
        Vec::new()
    };

    Ok(HopReport {
        step_index,
        from_dimension: from.rows,
        to_dimension: to.rows,
        family,
        interpretation,
        permutation,
        duplicate_row_classes,
        duplicate_column_classes,
        witness: StepWitnessReport {
            u: matrix_rows(&step.u),
            v: matrix_rows(&step.v),
        },
    })
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

fn path_interpretation(hops: &[HopReport]) -> Option<String> {
    if hops.is_empty() {
        return Some("identity".to_string());
    }
    let labels = hops
        .iter()
        .map(|hop| hop.interpretation.as_str())
        .collect::<Vec<_>>();
    Some(labels.join("_then_"))
}

fn move_interpretation(family: &str) -> &'static str {
    match family {
        "outsplit" => "elementary_row_split",
        "insplit" => "elementary_column_split",
        "out_amalgamation" => "elementary_row_amalgamation",
        "in_amalgamation" => "elementary_column_amalgamation",
        _ => "exact_graph_move",
    }
}

fn matching_factorisation_families(
    from: &DynMatrix,
    to: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: MoveFamilyPolicy,
) -> Result<Vec<String>, String> {
    if !from.is_square() || !to.is_square() {
        return Err("factorisation matching requires square endpoints".to_string());
    }

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
    Ok(families)
}

fn matrix_report(matrix: &DynMatrix) -> MatrixReport {
    MatrixReport {
        dimension: matrix.rows,
        entries: matrix_rows(matrix),
    }
}

fn matrix_rows(matrix: &DynMatrix) -> Vec<Vec<u32>> {
    (0..matrix.rows)
        .map(|row| (0..matrix.cols).map(|col| matrix.get(row, col)).collect())
        .collect()
}

fn duplicate_row_classes(matrix: &DynMatrix) -> Vec<Vec<usize>> {
    duplicate_classes(matrix.rows, |left, right| {
        (0..matrix.cols).all(|col| matrix.get(left, col) == matrix.get(right, col))
    })
}

fn duplicate_column_classes(matrix: &DynMatrix) -> Vec<Vec<usize>> {
    duplicate_classes(matrix.cols, |left, right| {
        (0..matrix.rows).all(|row| matrix.get(row, left) == matrix.get(row, right))
    })
}

fn duplicate_classes<F>(count: usize, equals: F) -> Vec<Vec<usize>>
where
    F: Fn(usize, usize) -> bool,
{
    if count == 0 {
        return Vec::new();
    }

    let mut seen = vec![false; count];
    let mut classes = Vec::new();

    for left in 0..count {
        if seen[left] {
            continue;
        }
        let mut class = vec![left + 1];
        seen[left] = true;
        for right in (left + 1)..count {
            if !seen[right] && equals(left, right) {
                seen[right] = true;
                class.push(right + 1);
            }
        }
        if class.len() > 1 {
            classes.push(class);
        }
    }

    classes
}

fn find_permutation_relabeling(
    from: &DynMatrix,
    to: &DynMatrix,
) -> Result<Option<Vec<usize>>, String> {
    if from.rows != from.cols || to.rows != to.cols || from.rows != to.rows {
        return Ok(None);
    }
    if from.rows > MAX_BRUTE_FORCE_PERMUTATION_DIM {
        return Err(format!(
            "permutation relabeling check is only supported up to {}x{} in this bounded helper; got {}x{}",
            MAX_BRUTE_FORCE_PERMUTATION_DIM,
            MAX_BRUTE_FORCE_PERMUTATION_DIM,
            from.rows,
            from.cols
        ));
    }

    let mut permutation: Vec<usize> = (0..from.rows).collect();
    let mut result = None;
    for_each_permutation(&mut permutation, 0, &mut |perm| {
        if result.is_some() {
            return;
        }
        let (permutation_matrix, inverse_permutation_matrix) = permutation_matrices(perm);
        let candidate = inverse_permutation_matrix
            .mul(from)
            .mul(&permutation_matrix);
        if candidate == *to {
            result = Some(perm.iter().map(|idx| idx + 1).collect());
        }
    });
    Ok(result)
}

fn permutation_matrices(permutation: &[usize]) -> (DynMatrix, DynMatrix) {
    let n = permutation.len();
    let mut permutation_data = vec![0u32; n * n];
    let mut inverse_data = vec![0u32; n * n];
    for (row, &col) in permutation.iter().enumerate() {
        permutation_data[row * n + col] = 1;
        inverse_data[col * n + row] = 1;
    }
    (
        DynMatrix::new(n, n, permutation_data),
        DynMatrix::new(n, n, inverse_data),
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

fn parse_cli(args: impl Iterator<Item = String>) -> Result<Cli, String> {
    let mut from = None;
    let mut to = None;
    let mut graph_max_lag = 3usize;
    let mut graph_max_intermediate_dim = 4usize;
    let mut graph_max_entry = None;
    let mut factorisation_max_entry = None;
    let mut write_json = None;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--from" => {
                from = Some(parse_matrix(
                    &args.next().ok_or("--from requires a matrix".to_string())?,
                )?);
            }
            "--to" => {
                to = Some(parse_matrix(
                    &args.next().ok_or("--to requires a matrix".to_string())?,
                )?);
            }
            "--graph-max-lag" => {
                graph_max_lag = parse_usize_arg(&mut args, "--graph-max-lag")?;
            }
            "--graph-max-intermediate-dim" => {
                graph_max_intermediate_dim =
                    parse_usize_arg(&mut args, "--graph-max-intermediate-dim")?;
            }
            "--graph-max-entry" => {
                graph_max_entry = Some(parse_u32_arg(&mut args, "--graph-max-entry")?);
            }
            "--factorisation-max-entry" => {
                factorisation_max_entry =
                    Some(parse_u32_arg(&mut args, "--factorisation-max-entry")?);
            }
            "--write-json" => {
                write_json = Some(
                    args.next()
                        .ok_or("--write-json requires a path".to_string())?,
                );
            }
            "--help" | "-h" => {
                return Err("Usage: explain_witness_step --from MATRIX --to MATRIX\
\n       [--graph-max-lag N] [--graph-max-intermediate-dim N]\
\n       [--graph-max-entry N] [--factorisation-max-entry N]\
\n       [--write-json PATH]"
                    .to_string());
            }
            other => {
                return Err(format!("unrecognized argument: {other}"));
            }
        }
    }

    let from = from.ok_or("missing --from MATRIX".to_string())?;
    let to = to.ok_or("missing --to MATRIX".to_string())?;
    if !from.is_square() || !to.is_square() {
        return Err("explain_witness_step requires square endpoints".to_string());
    }
    let graph_max_entry = graph_max_entry.unwrap_or_else(|| from.max_entry().max(to.max_entry()));
    let factorisation_max_entry = factorisation_max_entry.unwrap_or(graph_max_entry);

    Ok(Cli {
        from,
        to,
        graph_max_lag,
        graph_max_intermediate_dim,
        graph_max_entry,
        factorisation_max_entry,
        write_json,
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
            "expected 4 entries for a bare 2x2 matrix, got {}; use NxN:... for larger matrices",
            nums.len()
        ));
    }
    Ok(DynMatrix::new(2, 2, nums))
}

fn parse_dims(s: &str) -> Result<(usize, usize), String> {
    let (rows, cols) = s
        .split_once('x')
        .ok_or_else(|| format!("invalid matrix prefix {s:?}; expected NxN"))?;
    let rows = rows
        .parse::<usize>()
        .map_err(|err| format!("invalid row count in {s:?}: {err}"))?;
    let cols = cols
        .parse::<usize>()
        .map_err(|err| format!("invalid column count in {s:?}: {err}"))?;
    Ok((rows, cols))
}

fn parse_entries(s: &str) -> Result<Vec<u32>, String> {
    s.split(',')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|err| format!("invalid matrix entry {part:?}: {err}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        duplicate_column_classes, duplicate_row_classes, find_permutation_relabeling,
        inverse_graph_family,
    };
    use sse_core::matrix::DynMatrix;

    #[test]
    fn duplicate_class_helpers_use_one_based_indices() {
        let matrix = DynMatrix::new(4, 4, vec![1, 1, 2, 2, 1, 1, 2, 2, 3, 3, 4, 4, 3, 3, 4, 4]);
        assert_eq!(duplicate_row_classes(&matrix), vec![vec![1, 2], vec![3, 4]]);
        assert_eq!(
            duplicate_column_classes(&matrix),
            vec![vec![1, 2], vec![3, 4]]
        );
    }

    #[test]
    fn find_permutation_relabeling_reports_one_based_target_order() {
        let from = DynMatrix::new(4, 4, vec![1, 1, 1, 2, 1, 0, 0, 3, 2, 4, 4, 2, 1, 0, 0, 3]);
        let to = DynMatrix::new(4, 4, vec![1, 2, 1, 1, 1, 3, 0, 0, 1, 3, 0, 0, 2, 2, 4, 4]);
        assert_eq!(
            find_permutation_relabeling(&from, &to).unwrap(),
            Some(vec![1, 3, 4, 2])
        );
    }

    #[test]
    fn inverse_graph_family_maps_split_amalgamation_pairs() {
        assert_eq!(inverse_graph_family("outsplit"), Some("in_amalgamation"));
        assert_eq!(inverse_graph_family("insplit"), Some("out_amalgamation"));
        assert_eq!(inverse_graph_family("out_amalgamation"), Some("insplit"));
        assert_eq!(inverse_graph_family("in_amalgamation"), Some("outsplit"));
        assert_eq!(inverse_graph_family("unknown"), None);
    }
}
