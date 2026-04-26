use serde::{Deserialize, Serialize};

use crate::matrix::DynMatrix;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointLocalParityAction {
    ReuseEndpointLocalParity,
    RankOrProposeInsideCoarseBucket,
    Ignore,
}

impl EndpointLocalParityAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReuseEndpointLocalParity => "reuse_endpoint_local_parity",
            Self::RankOrProposeInsideCoarseBucket => "rank_or_propose_inside_coarse_bucket",
            Self::Ignore => "ignore",
        }
    }
}

pub fn supports_square_endpoint_local_parity(matrix: &DynMatrix) -> bool {
    matrix.rows == matrix.cols && matches!(matrix.rows, 3 | 4)
}

pub fn endpoint_local_parity_action(
    left: &DynMatrix,
    right: &DynMatrix,
) -> EndpointLocalParityAction {
    if !supports_square_endpoint_local_parity(left) || !supports_square_endpoint_local_parity(right)
    {
        return EndpointLocalParityAction::Ignore;
    }

    if mass_support_signature(left) != mass_support_signature(right) {
        return EndpointLocalParityAction::Ignore;
    }

    if trimmed_active_window_signature(left) == trimmed_active_window_signature(right) {
        EndpointLocalParityAction::ReuseEndpointLocalParity
    } else {
        EndpointLocalParityAction::RankOrProposeInsideCoarseBucket
    }
}

pub fn mass_support_signature(matrix: &DynMatrix) -> String {
    let mut row_sums = vec![0u64; matrix.rows];
    let mut col_sums = vec![0u64; matrix.cols];
    let mut row_supports = vec![0u8; matrix.rows];
    let mut col_supports = vec![0u8; matrix.cols];
    let mut entry_sum = 0u64;

    for row in 0..matrix.rows {
        for col in 0..matrix.cols {
            let value = matrix.get(row, col);
            row_sums[row] += value as u64;
            col_sums[col] += value as u64;
            entry_sum += value as u64;
            if value != 0 {
                row_supports[row] += 1;
                col_supports[col] += 1;
            }
        }
    }

    row_sums.sort_unstable();
    col_sums.sort_unstable();
    row_supports.sort_unstable();
    col_supports.sort_unstable();

    format!(
        "d{}|sum{}|rs{}|cs{}|rS{}|cS{}",
        matrix.rows,
        entry_sum,
        join_u64(&row_sums),
        join_u64(&col_sums),
        join_u8(&row_supports),
        join_u8(&col_supports),
    )
}

pub fn trimmed_active_window_signature(matrix: &DynMatrix) -> String {
    let trimmed = trimmed_active_window(matrix);
    format!(
        "{}x{}|{}",
        trimmed.rows,
        trimmed.cols,
        join_u32(&trimmed.data)
    )
}

pub fn trimmed_active_window(matrix: &DynMatrix) -> DynMatrix {
    let canonical = matrix.canonical_perm();
    trim_zero_rows_and_cols(&canonical)
}

pub fn trim_zero_rows_and_cols(matrix: &DynMatrix) -> DynMatrix {
    let active_rows = (0..matrix.rows)
        .filter(|&row| (0..matrix.cols).any(|col| matrix.get(row, col) != 0))
        .collect::<Vec<_>>();
    let active_cols = (0..matrix.cols)
        .filter(|&col| (0..matrix.rows).any(|row| matrix.get(row, col) != 0))
        .collect::<Vec<_>>();

    let mut data = Vec::with_capacity(active_rows.len() * active_cols.len());
    for &row in &active_rows {
        for &col in &active_cols {
            data.push(matrix.get(row, col));
        }
    }

    DynMatrix::new(active_rows.len(), active_cols.len(), data)
}

fn join_u8(values: &[u8]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn join_u32(values: &[u32]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn join_u64(values: &[u64]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::{
        endpoint_local_parity_action, mass_support_signature, trimmed_active_window_signature,
        EndpointLocalParityAction,
    };
    use crate::matrix::DynMatrix;

    #[test]
    fn parity_action_reuses_exact_trimmed_square_match() {
        let left = DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0]);
        let right = DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0]);

        assert_eq!(
            endpoint_local_parity_action(&left, &right),
            EndpointLocalParityAction::ReuseEndpointLocalParity
        );
    }

    #[test]
    fn parity_action_ranks_coarse_only_square_match() {
        let left = DynMatrix::new(4, 4, vec![1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0]);
        let right = DynMatrix::new(4, 4, vec![1, 12, 0, 1, 1, 1, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(
            mass_support_signature(&left),
            mass_support_signature(&right)
        );
        assert_ne!(
            trimmed_active_window_signature(&left),
            trimmed_active_window_signature(&right)
        );
        assert_eq!(
            endpoint_local_parity_action(&left, &right),
            EndpointLocalParityAction::RankOrProposeInsideCoarseBucket
        );
    }

    #[test]
    fn parity_action_ignores_unsupported_dimensions() {
        let left = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let right = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);

        assert_eq!(
            endpoint_local_parity_action(&left, &right),
            EndpointLocalParityAction::Ignore
        );
    }
}
