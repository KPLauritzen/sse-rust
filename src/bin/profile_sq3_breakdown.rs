use std::collections::HashMap;
use std::time::Instant;

use sse_core::factorisation::profile_square_factorisations_3x3_breakdown;
use sse_core::graph_moves::{
    partition_refined_same_future_past_gap_total, same_future_past_signature,
    SameFuturePastSignature,
};
use sse_core::matrix::DynMatrix;

fn main() {
    if let Err(err) = run(std::env::args().skip(1)) {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let mut matrix_arg = None;
    let mut scan_dup_row_support = false;
    let mut scan_zero_col_support = false;
    let mut scan_singular_grid = false;
    let mut scan_max_entry = 6u32;
    let mut sq3_cap = 4u32;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--matrix" => {
                matrix_arg = Some(parse_matrix(
                    &args.next().ok_or("--matrix requires a matrix argument")?,
                )?);
            }
            "--scan-dup-row-support" => {
                scan_dup_row_support = true;
            }
            "--scan-zero-col-support" => {
                scan_zero_col_support = true;
            }
            "--scan-singular-grid" => {
                scan_singular_grid = true;
            }
            "--scan-max-entry" => {
                scan_max_entry = args
                    .next()
                    .ok_or("--scan-max-entry requires a value")?
                    .parse()
                    .map_err(|err| format!("invalid --scan-max-entry: {err}"))?;
            }
            "--sq3-cap" => {
                sq3_cap = args
                    .next()
                    .ok_or("--sq3-cap requires a value")?
                    .parse()
                    .map_err(|err| format!("invalid --sq3-cap: {err}"))?;
            }
            "--help" | "-h" => {
                return Err(
                    "usage: profile_sq3_breakdown [--matrix 3x3:...] [--scan-dup-row-support] [--scan-zero-col-support] [--scan-singular-grid] [--scan-max-entry N] [--sq3-cap N]\n\n\
                     With no flags, prints the built-in control cases.\n\
                     --scan-dup-row-support scans singular matrices of the form\n\
                       [0,a,0]\n\
                       [b,c,d]\n\
                       [0,a,0]\n\
                     --scan-zero-col-support scans singular matrices of the form\n\
                       [0,a,b]\n\
                       [0,c,d]\n\
                       [0,e,f]\n\
                     --scan-singular-grid scans all singular 3x3 matrices with entries in 0..=N\n\
                     with positive entries in 1..=N and reports exact square-factorisation collisions.".to_string(),
                );
            }
            _ => {
                return Err(format!("unrecognized argument '{arg}'"));
            }
        }
    }

    if scan_dup_row_support {
        scan_duplicate_row_support_family(scan_max_entry, sq3_cap);
        return Ok(());
    }

    if scan_zero_col_support {
        scan_zero_column_support_family(scan_max_entry, sq3_cap);
        return Ok(());
    }

    if scan_singular_grid {
        scan_singular_grid_family(scan_max_entry, sq3_cap);
        return Ok(());
    }

    if let Some(matrix) = matrix_arg {
        print_case("matrix", &matrix, sq3_cap);
        return Ok(());
    }

    let cases = [
        (
            "waste_dup_row_sum9",
            DynMatrix::new(3, 3, vec![0, 1, 0, 2, 2, 3, 0, 1, 0]),
        ),
        (
            "productive_dup_col_sum11",
            DynMatrix::new(3, 3, vec![0, 1, 0, 1, 1, 1, 1, 5, 1]),
        ),
        (
            "productive_plain_sum9",
            DynMatrix::new(3, 3, vec![0, 3, 0, 1, 2, 2, 0, 1, 0]),
        ),
    ];

    for (label, matrix) in cases {
        print_case(label, &matrix, sq3_cap);
    }

    Ok(())
}

fn print_case(label: &str, matrix: &DynMatrix, sq3_cap: u32) {
    let started = Instant::now();
    let breakdown = profile_square_factorisations_3x3_breakdown(matrix, sq3_cap);
    let elapsed_ms = started.elapsed().as_millis();
    let row1_survival = ratio(
        breakdown.row1_survived_all_cols,
        breakdown.row1_candidates_total,
    );
    let row2_survival = ratio(
        breakdown.emitted_factorisations,
        breakdown.v_column_combinations,
    );
    println!("{label}");
    println!("  matrix={}", format_matrix(matrix));
    println!("  elapsed_ms={elapsed_ms}");
    println!(
        "  valid_row0_candidates={}",
        breakdown.valid_row0_candidates
    );
    println!(
        "  row1_candidates_total={}",
        breakdown.row1_candidates_total
    );
    println!("  row1_pruned_min_sum={}", breakdown.row1_pruned_min_sum);
    println!("  row1_pruned_gcd={}", breakdown.row1_pruned_gcd);
    println!(
        "  row1_pruned_col0_empty={}",
        breakdown.row1_pruned_col0_empty
    );
    println!(
        "  row1_pruned_col1_empty={}",
        breakdown.row1_pruned_col1_empty
    );
    println!(
        "  row1_pruned_col2_empty={}",
        breakdown.row1_pruned_col2_empty
    );
    println!(
        "  row1_survived_all_cols={} ({row1_survival:.3})",
        breakdown.row1_survived_all_cols
    );
    println!(
        "  v_column_combinations={}",
        breakdown.v_column_combinations
    );
    println!(
        "  row2_solution_candidates={}",
        breakdown.row2_solution_candidates
    );
    println!("  row2_pruned_min_sum={}", breakdown.row2_pruned_min_sum);
    println!(
        "  emitted_factorisations={} ({row2_survival:.3})",
        breakdown.emitted_factorisations
    );
    println!();
}

fn scan_duplicate_row_support_family(scan_max_entry: u32, sq3_cap: u32) {
    let cases = (1..=scan_max_entry).flat_map(|a| {
        (1..=scan_max_entry).flat_map(move |b| {
            (1..=scan_max_entry).flat_map(move |c| {
                (1..=scan_max_entry)
                    .map(move |d| DynMatrix::new(3, 3, vec![0, a, 0, b, c, d, 0, a, 0]))
            })
        })
    });
    scan_family(
        "duplicate-row support scan",
        format!("family=[0,a,0; b,c,d; 0,a,0], a,b,c,d in 1..={scan_max_entry}, sq3_cap={sq3_cap}"),
        sq3_cap,
        cases,
    );
}

fn scan_zero_column_support_family(scan_max_entry: u32, sq3_cap: u32) {
    let cases = (1..=scan_max_entry).flat_map(|a| {
        (1..=scan_max_entry).flat_map(move |b| {
            (1..=scan_max_entry).flat_map(move |c| {
                (1..=scan_max_entry).flat_map(move |d| {
                    (1..=scan_max_entry).flat_map(move |e| {
                        (1..=scan_max_entry)
                            .map(move |f| DynMatrix::new(3, 3, vec![0, a, b, 0, c, d, 0, e, f]))
                    })
                })
            })
        })
    });
    scan_family(
        "zero-column support scan",
        format!(
            "family=[0,a,b; 0,c,d; 0,e,f], positive entries in 1..={scan_max_entry}, sq3_cap={sq3_cap}"
        ),
        sq3_cap,
        cases,
    );
}

fn scan_singular_grid_family(scan_max_entry: u32, sq3_cap: u32) {
    let cases = (0..=scan_max_entry).flat_map(|a00| {
        (0..=scan_max_entry).flat_map(move |a01| {
            (0..=scan_max_entry).flat_map(move |a02| {
                (0..=scan_max_entry).flat_map(move |a10| {
                    (0..=scan_max_entry).flat_map(move |a11| {
                        (0..=scan_max_entry).flat_map(move |a12| {
                            (0..=scan_max_entry).flat_map(move |a20| {
                                (0..=scan_max_entry).flat_map(move |a21| {
                                    (0..=scan_max_entry).filter_map(move |a22| {
                                        let matrix = DynMatrix::new(
                                            3,
                                            3,
                                            vec![a00, a01, a02, a10, a11, a12, a20, a21, a22],
                                        );
                                        let det = determinant_3x3(&matrix);
                                        if det == 0 && matrix.data.iter().any(|&value| value > 0) {
                                            Some(matrix)
                                        } else {
                                            None
                                        }
                                    })
                                })
                            })
                        })
                    })
                })
            })
        })
    });
    scan_family(
        "singular-grid scan",
        format!("all singular nonzero 3x3 matrices with entries in 0..={scan_max_entry}, sq3_cap={sq3_cap}"),
        sq3_cap,
        cases,
    );
}

fn scan_family<I>(label: &str, description: String, sq3_cap: u32, cases: I)
where
    I: IntoIterator<Item = DynMatrix>,
{
    let mut total = 0usize;
    let mut factorable = 0usize;
    let mut unfactorable = 0usize;
    let mut by_signature = HashMap::<SameFuturePastSignature, Vec<(DynMatrix, usize)>>::new();

    for matrix in cases {
        let emitted =
            profile_square_factorisations_3x3_breakdown(&matrix, sq3_cap).emitted_factorisations;
        let signature = same_future_past_signature(&matrix)
            .expect("3x3 matrix should always produce a signature");
        total += 1;
        if emitted == 0 {
            unfactorable += 1;
        } else {
            factorable += 1;
        }
        by_signature
            .entry(signature)
            .or_default()
            .push((matrix, emitted));
    }

    println!("{label}: {description}");
    println!(
        "  total={} factorable={} unfactorable={}",
        total, factorable, unfactorable
    );

    let mut coarse_collision = None;
    let mut refined_collision = None;
    for cases in by_signature.values() {
        let mut seen_factorable = None;
        let mut seen_unfactorable = None;
        for (matrix, emitted) in cases {
            if *emitted == 0 {
                seen_unfactorable = Some((matrix, *emitted));
            } else {
                seen_factorable = Some((matrix, *emitted));
            }
        }

        if coarse_collision.is_none() {
            if let (Some((left_matrix, left_emitted)), Some((right_matrix, right_emitted))) =
                (seen_unfactorable, seen_factorable)
            {
                coarse_collision = Some((
                    left_matrix.clone(),
                    left_emitted,
                    right_matrix.clone(),
                    right_emitted,
                ));
            }
        }

        if refined_collision.is_none() {
            'pairs: for left in 0..cases.len() {
                for right in (left + 1)..cases.len() {
                    let left_case = &cases[left];
                    let right_case = &cases[right];
                    let left_factorable = left_case.1 > 0;
                    let right_factorable = right_case.1 > 0;
                    if left_factorable == right_factorable {
                        continue;
                    }
                    if partition_refined_same_future_past_gap_total(&left_case.0, &right_case.0)
                        == 0
                    {
                        refined_collision = Some((
                            left_case.0.clone(),
                            left_case.1,
                            right_case.0.clone(),
                            right_case.1,
                        ));
                        break 'pairs;
                    }
                }
            }
        }

        if coarse_collision.is_some() && refined_collision.is_some() {
            break;
        }
    }

    if let Some((left, left_emitted, right, right_emitted)) = coarse_collision {
        println!("  coarse same-future/past collision:");
        println!(
            "    left emitted={} matrix={}",
            left_emitted,
            format_matrix(&left)
        );
        println!(
            "    right emitted={} matrix={}",
            right_emitted,
            format_matrix(&right)
        );
    } else {
        println!("  coarse same-future/past collision: none found");
    }

    if let Some((left, left_emitted, right, right_emitted)) = refined_collision {
        println!("  partition-refined collision:");
        println!(
            "    left emitted={} matrix={}",
            left_emitted,
            format_matrix(&left)
        );
        println!(
            "    right emitted={} matrix={}",
            right_emitted,
            format_matrix(&right)
        );
    } else {
        println!("  partition-refined collision: none found");
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn format_matrix(matrix: &DynMatrix) -> String {
    (0..matrix.rows)
        .map(|row| {
            let values = (0..matrix.cols)
                .map(|col| matrix.get(row, col).to_string())
                .collect::<Vec<_>>()
                .join(",");
            format!("[{values}]")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_matrix(s: &str) -> Result<DynMatrix, String> {
    let (dims, entries) = s
        .split_once(':')
        .ok_or_else(|| format!("expected NxN:... matrix, got '{s}'"))?;
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
    Ok(DynMatrix::new(rows, cols, nums))
}

fn parse_dims(s: &str) -> Result<(usize, usize), String> {
    let (rows, cols) = s
        .split_once('x')
        .ok_or_else(|| format!("invalid matrix prefix '{s}' (expected NxN)"))?;
    let rows = rows
        .parse()
        .map_err(|err| format!("invalid row count in '{s}': {err}"))?;
    let cols = cols
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

fn determinant_3x3(matrix: &DynMatrix) -> i64 {
    let a00 = matrix.get(0, 0) as i64;
    let a01 = matrix.get(0, 1) as i64;
    let a02 = matrix.get(0, 2) as i64;
    let a10 = matrix.get(1, 0) as i64;
    let a11 = matrix.get(1, 1) as i64;
    let a12 = matrix.get(1, 2) as i64;
    let a20 = matrix.get(2, 0) as i64;
    let a21 = matrix.get(2, 1) as i64;
    let a22 = matrix.get(2, 2) as i64;
    a00 * (a11 * a22 - a12 * a21) - a01 * (a10 * a22 - a12 * a20) + a02 * (a10 * a21 - a11 * a20)
}
