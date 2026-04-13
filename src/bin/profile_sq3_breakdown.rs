use std::time::Instant;

use sse_core::factorisation::profile_square_factorisations_3x3_breakdown;
use sse_core::matrix::DynMatrix;

fn main() {
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
        let started = Instant::now();
        let breakdown = profile_square_factorisations_3x3_breakdown(&matrix, 4);
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
        println!("  matrix={}", format_matrix(&matrix));
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
