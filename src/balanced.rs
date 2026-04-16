use crate::factorisation::enumerate_factorisations_3x3_to_2;
use crate::graph_moves::{
    enumerate_insplits_2x2_to_3x3, enumerate_outsplits_2x2_to_3x3, OutsplitWitness2x2To3x3,
};
use crate::matrix::{DynMatrix, SqMatrix};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A balanced elementary equivalence witness for a pair of 2x2 matrices.
///
/// The matrices satisfy
/// `A = S R_A`, `B = S R_B`, and `R_A S = R_B S`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedElementaryWitness2x2 {
    pub s: DynMatrix,
    pub r_a: DynMatrix,
    pub r_b: DynMatrix,
}

/// A nontrivial same-size balanced-elementary neighbor of a `2x2` matrix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedElementaryNeighbor2x2 {
    pub matrix: SqMatrix<2>,
    pub witness: BalancedElementaryWitness2x2,
}

/// A two-step same-size balanced-elementary zig-zag through a `2x2` bridge state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedElementaryZigzag2x2 {
    pub bridge: SqMatrix<2>,
    pub left_witness: BalancedElementaryWitness2x2,
    pub right_witness: BalancedElementaryWitness2x2,
}

/// A bounded balanced-neighbor hit from one candidate `2x2` state into another.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedElementaryNeighborHit2x2 {
    pub source: SqMatrix<2>,
    pub target: SqMatrix<2>,
    pub witness: BalancedElementaryWitness2x2,
}

/// A canonical `3x3` state reached by a bounded
/// `3x3 -> 2x2 <-balanced-> 2x2 -> 3x3` seam.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedBridgeReturnNeighbor3x3 {
    pub matrix: DynMatrix,
    pub source_bridge: SqMatrix<2>,
    pub target_bridge: SqMatrix<2>,
    pub witness: BalancedElementaryWitness2x2,
}

/// Configuration for bounded balanced-elementary search on 2x2 matrices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedSearchConfig2x2 {
    /// Maximum common intermediate size `m` in `S (2xm)` and `R_A, R_B (mx2)`.
    pub max_common_dim: usize,
    /// Maximum entry allowed in `S`, `R_A`, and `R_B`.
    pub max_entry: u32,
}

impl Default for BalancedSearchConfig2x2 {
    fn default() -> Self {
        Self {
            max_common_dim: 2,
            max_entry: 10,
        }
    }
}

/// Search result for bounded balanced-elementary search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BalancedSearchResult2x2 {
    Equivalent(BalancedElementaryWitness2x2),
    Exhausted,
}

/// Verify the balanced-elementary equations for a proposed witness.
pub fn verify_balanced_elementary_witness_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &BalancedElementaryWitness2x2,
) -> Result<(), String> {
    if witness.s.rows != 2 {
        return Err("S must have 2 rows".into());
    }
    if witness.r_a.rows != witness.s.cols || witness.r_b.rows != witness.s.cols {
        return Err("R_A and R_B must have as many rows as S has columns".into());
    }
    if witness.r_a.cols != 2 || witness.r_b.cols != 2 {
        return Err("R_A and R_B must have 2 columns".into());
    }

    let lhs_a = witness.s.mul(&witness.r_a);
    let lhs_b = witness.s.mul(&witness.r_b);
    if lhs_a != DynMatrix::from_sq(a) {
        return Err(format!(
            "A != S R_A: {:?} vs {:?}",
            DynMatrix::from_sq(a),
            lhs_a
        ));
    }
    if lhs_b != DynMatrix::from_sq(b) {
        return Err(format!(
            "B != S R_B: {:?} vs {:?}",
            DynMatrix::from_sq(b),
            lhs_b
        ));
    }

    let ras = witness.r_a.mul(&witness.s);
    let rbs = witness.r_b.mul(&witness.s);
    if ras != rbs {
        return Err(format!("R_A S != R_B S: {:?} vs {:?}", ras, rbs));
    }

    Ok(())
}

/// Search for a bounded balanced-elementary equivalence witness between two 2x2 matrices.
pub fn find_balanced_elementary_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &BalancedSearchConfig2x2,
) -> BalancedSearchResult2x2 {
    for common_dim in 1..=config.max_common_dim {
        if let Some(witness) = find_balanced_elementary_equivalence_with_common_dim_2x2(
            a,
            b,
            common_dim,
            config.max_entry,
        ) {
            return BalancedSearchResult2x2::Equivalent(witness);
        }
    }

    BalancedSearchResult2x2::Exhausted
}

/// Enumerate distinct nontrivial same-size balanced-elementary neighbors of a `2x2` matrix.
pub fn enumerate_balanced_elementary_neighbors_2x2(
    source: &SqMatrix<2>,
    config: &BalancedSearchConfig2x2,
) -> Vec<BalancedElementaryNeighbor2x2> {
    let mut unique_neighbors = BTreeMap::<SqMatrix<2>, BalancedElementaryNeighbor2x2>::new();
    for common_dim in 1..=config.max_common_dim {
        for neighbor in enumerate_balanced_elementary_neighbors_with_common_dim_2x2(
            source,
            common_dim,
            config.max_entry,
        ) {
            unique_neighbors
                .entry(neighbor.matrix.clone())
                .or_insert(neighbor);
        }
    }
    unique_neighbors.into_values().collect()
}

/// Search for a bounded `2x2 <-balanced-> 2x2 <-balanced-> 2x2` zig-zag meeting.
pub fn find_balanced_elementary_zigzag_meeting_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &BalancedSearchConfig2x2,
) -> Option<BalancedElementaryZigzag2x2> {
    let mut right_neighbors = BTreeMap::<SqMatrix<2>, BalancedElementaryWitness2x2>::new();
    for neighbor in enumerate_balanced_elementary_neighbors_2x2(b, config) {
        right_neighbors
            .entry(neighbor.matrix.clone())
            .or_insert(neighbor.witness);
    }

    for neighbor in enumerate_balanced_elementary_neighbors_2x2(a, config) {
        if let Some(right_witness) = right_neighbors.get(&neighbor.matrix) {
            return Some(BalancedElementaryZigzag2x2 {
                bridge: neighbor.matrix,
                left_witness: neighbor.witness,
                right_witness: right_witness.clone(),
            });
        }
    }

    None
}

/// Enumerate canonical `2x2` bridge states reached by one bounded
/// `2x2 -> 3x3 -> 2x2` out-split/factorisation seam.
pub fn enumerate_outsplit_bridge_states_2x2(
    source: &SqMatrix<2>,
    bridge_max_entry: u32,
) -> Vec<SqMatrix<2>> {
    let mut bridges = BTreeSet::new();
    for witness in enumerate_outsplits_2x2_to_3x3(source) {
        for (u, v) in enumerate_factorisations_3x3_to_2(&witness.outsplit, bridge_max_entry) {
            let bridge = v
                .mul(&u)
                .to_sq::<2>()
                .expect("3x3-to-2 factorisation should produce a 2x2 bridge")
                .canonical();
            bridges.insert(bridge);
        }
    }
    bridges.into_iter().collect()
}

/// Enumerate bounded balanced-neighbor hits from one candidate set into another.
pub fn enumerate_balanced_neighbor_set_hits_2x2(
    source_candidates: &[SqMatrix<2>],
    target_candidates: &[SqMatrix<2>],
    config: &BalancedSearchConfig2x2,
) -> Vec<BalancedElementaryNeighborHit2x2> {
    let target_set = target_candidates.iter().cloned().collect::<BTreeSet<_>>();
    let mut unique_hits =
        BTreeMap::<(SqMatrix<2>, SqMatrix<2>), BalancedElementaryNeighborHit2x2>::new();

    for source in source_candidates {
        for neighbor in enumerate_balanced_elementary_neighbors_2x2(source, config) {
            if !target_set.contains(&neighbor.matrix) {
                continue;
            }
            unique_hits
                .entry((source.clone(), neighbor.matrix.clone()))
                .or_insert_with(|| BalancedElementaryNeighborHit2x2 {
                    source: source.clone(),
                    target: neighbor.matrix,
                    witness: neighbor.witness,
                });
        }
    }

    unique_hits.into_values().collect()
}

/// Enumerate canonical `3x3` neighbors reached by one bounded
/// `3x3 -> 2x2 <-balanced-> 2x2 -> 3x3` seam.
pub fn enumerate_balanced_bridge_return_neighbors_3x3(
    source: &DynMatrix,
    bridge_max_entry: u32,
    config: &BalancedSearchConfig2x2,
) -> Vec<BalancedBridgeReturnNeighbor3x3> {
    enumerate_balanced_bridge_return_neighbors_with_3x3_refinement(
        source,
        bridge_max_entry,
        config,
        enumerate_outsplits_2x2_to_3x3,
    )
}

/// Enumerate canonical `3x3` neighbors reached by one bounded
/// `3x3 -> 2x2 <-balanced-> 2x2 -> 3x3` seam whose return step is an in-split.
pub fn enumerate_balanced_bridge_insplit_return_neighbors_3x3(
    source: &DynMatrix,
    bridge_max_entry: u32,
    config: &BalancedSearchConfig2x2,
) -> Vec<BalancedBridgeReturnNeighbor3x3> {
    enumerate_balanced_bridge_return_neighbors_with_3x3_refinement(
        source,
        bridge_max_entry,
        config,
        enumerate_insplits_2x2_to_3x3,
    )
}

fn enumerate_balanced_bridge_return_neighbors_with_3x3_refinement<F>(
    source: &DynMatrix,
    bridge_max_entry: u32,
    config: &BalancedSearchConfig2x2,
    enumerate_refinements: F,
) -> Vec<BalancedBridgeReturnNeighbor3x3>
where
    F: Fn(&SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3>,
{
    assert_eq!(source.rows, 3);
    assert_eq!(source.cols, 3);

    let mut source_bridges = BTreeSet::new();
    for (u, v) in enumerate_factorisations_3x3_to_2(source, bridge_max_entry) {
        let bridge = v
            .mul(&u)
            .to_sq::<2>()
            .expect("3x3-to-2 factorisation should produce a 2x2 bridge")
            .canonical();
        source_bridges.insert(bridge);
    }

    let mut unique_neighbors = BTreeMap::<DynMatrix, BalancedBridgeReturnNeighbor3x3>::new();
    for source_bridge in source_bridges {
        for neighbor in enumerate_balanced_elementary_neighbors_2x2(&source_bridge, config) {
            for witness in enumerate_refinements(&neighbor.matrix) {
                let matrix = witness.outsplit.canonical_perm();
                unique_neighbors.entry(matrix.clone()).or_insert_with(|| {
                    BalancedBridgeReturnNeighbor3x3 {
                        matrix,
                        source_bridge: source_bridge.clone(),
                        target_bridge: neighbor.matrix.clone(),
                        witness: neighbor.witness.clone(),
                    }
                });
            }
        }
    }

    unique_neighbors.into_values().collect()
}

/// Search for a balanced-elementary equivalence witness with a fixed common dimension.
pub fn find_balanced_elementary_equivalence_with_common_dim_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    common_dim: usize,
    max_entry: u32,
) -> Option<BalancedElementaryWitness2x2> {
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);

    for s_data in enumerate_matrix_data(2 * common_dim, max_entry) {
        let s = DynMatrix::new(2, common_dim, s_data);
        let r_a_candidates = enumerate_right_factorisations_2x2(&s, &a_dyn, max_entry);
        if r_a_candidates.is_empty() {
            continue;
        }
        let r_b_candidates = enumerate_right_factorisations_2x2(&s, &b_dyn, max_entry);
        if r_b_candidates.is_empty() {
            continue;
        }

        for r_a in &r_a_candidates {
            let ras = r_a.mul(&s);
            for r_b in &r_b_candidates {
                if ras == r_b.mul(&s) {
                    let witness = BalancedElementaryWitness2x2 {
                        s: s.clone(),
                        r_a: r_a.clone(),
                        r_b: r_b.clone(),
                    };
                    debug_assert!(verify_balanced_elementary_witness_2x2(a, b, &witness).is_ok());
                    return Some(witness);
                }
            }
        }
    }

    None
}

fn enumerate_balanced_elementary_neighbors_with_common_dim_2x2(
    source: &SqMatrix<2>,
    common_dim: usize,
    max_entry: u32,
) -> Vec<BalancedElementaryNeighbor2x2> {
    let source_dyn = DynMatrix::from_sq(source);
    let mut neighbors = BTreeMap::<SqMatrix<2>, BalancedElementaryNeighbor2x2>::new();

    for s_data in enumerate_matrix_data(2 * common_dim, max_entry) {
        let s = DynMatrix::new(2, common_dim, s_data);
        let r_a_candidates = enumerate_right_factorisations_2x2(&s, &source_dyn, max_entry);
        if r_a_candidates.is_empty() {
            continue;
        }

        let mut row_solution_cache = HashMap::<Vec<u32>, Vec<[u32; 2]>>::new();
        for r_a in r_a_candidates {
            let balanced_product = r_a.mul(&s);
            let r_b_candidates = enumerate_balanced_right_factors_2x2(
                &s,
                &balanced_product,
                max_entry,
                &mut row_solution_cache,
            );
            for r_b in r_b_candidates {
                let matrix = s
                    .mul(&r_b)
                    .to_sq::<2>()
                    .expect("balanced same-size neighbors should stay 2x2");
                if &matrix == source {
                    continue;
                }
                let witness = BalancedElementaryWitness2x2 {
                    s: s.clone(),
                    r_a: r_a.clone(),
                    r_b,
                };
                debug_assert!(
                    verify_balanced_elementary_witness_2x2(source, &matrix, &witness).is_ok()
                );
                neighbors
                    .entry(matrix.clone())
                    .or_insert_with(|| BalancedElementaryNeighbor2x2 { matrix, witness });
            }
        }
    }

    neighbors.into_values().collect()
}

fn enumerate_right_factorisations_2x2(
    s: &DynMatrix,
    target: &DynMatrix,
    max_entry: u32,
) -> Vec<DynMatrix> {
    debug_assert_eq!(s.rows, 2);
    debug_assert_eq!(target.rows, 2);
    debug_assert_eq!(target.cols, 2);

    let col0 = enumerate_column_solutions_2xm(s, [target.get(0, 0), target.get(1, 0)], max_entry);
    if col0.is_empty() {
        return Vec::new();
    }

    let col1 = enumerate_column_solutions_2xm(s, [target.get(0, 1), target.get(1, 1)], max_entry);
    if col1.is_empty() {
        return Vec::new();
    }

    let mut factors = Vec::new();
    for left in &col0 {
        for right in &col1 {
            let mut data = Vec::with_capacity(2 * s.cols);
            for row in 0..s.cols {
                data.push(left[row]);
                data.push(right[row]);
            }
            factors.push(DynMatrix::new(s.cols, 2, data));
        }
    }
    factors
}

fn enumerate_column_solutions_2xm(
    s: &DynMatrix,
    target: [u32; 2],
    max_entry: u32,
) -> Vec<Vec<u32>> {
    let mut solutions = Vec::new();
    let mut current = vec![0u32; s.cols];
    recurse_column_solution(0, s, target, max_entry, &mut current, &mut solutions);
    solutions
}

fn recurse_column_solution(
    idx: usize,
    s: &DynMatrix,
    target: [u32; 2],
    max_entry: u32,
    current: &mut [u32],
    solutions: &mut Vec<Vec<u32>>,
) {
    if idx == s.cols {
        let sum0: u32 = (0..s.cols).map(|j| s.get(0, j) * current[j]).sum();
        let sum1: u32 = (0..s.cols).map(|j| s.get(1, j) * current[j]).sum();
        if sum0 == target[0] && sum1 == target[1] {
            solutions.push(current.to_vec());
        }
        return;
    }

    for value in 0..=max_entry {
        current[idx] = value;

        let partial0: u32 = (0..=idx).map(|j| s.get(0, j) * current[j]).sum();
        let partial1: u32 = (0..=idx).map(|j| s.get(1, j) * current[j]).sum();
        if partial0 > target[0] || partial1 > target[1] {
            continue;
        }

        recurse_column_solution(idx + 1, s, target, max_entry, current, solutions);
    }
}

fn enumerate_balanced_right_factors_2x2(
    s: &DynMatrix,
    balanced_product: &DynMatrix,
    max_entry: u32,
    row_solution_cache: &mut HashMap<Vec<u32>, Vec<[u32; 2]>>,
) -> Vec<DynMatrix> {
    debug_assert_eq!(balanced_product.rows, s.cols);
    debug_assert_eq!(balanced_product.cols, s.cols);

    let mut row_solutions = Vec::with_capacity(s.cols);
    for row in 0..balanced_product.rows {
        let target = (0..balanced_product.cols)
            .map(|col| balanced_product.get(row, col))
            .collect::<Vec<_>>();
        let solutions = row_solution_cache
            .entry(target.clone())
            .or_insert_with(|| enumerate_row_solutions_1x2_times_2xm(s, &target, max_entry))
            .clone();
        if solutions.is_empty() {
            return Vec::new();
        }
        row_solutions.push(solutions);
    }

    let mut factors = Vec::new();
    let mut current_rows = vec![[0u32; 2]; balanced_product.rows];
    recurse_balanced_right_factor_rows(0, &row_solutions, &mut current_rows, &mut factors);
    factors
}

fn enumerate_row_solutions_1x2_times_2xm(
    s: &DynMatrix,
    target: &[u32],
    max_entry: u32,
) -> Vec<[u32; 2]> {
    debug_assert_eq!(s.rows, 2);
    debug_assert_eq!(target.len(), s.cols);

    let mut solutions = Vec::new();
    for left in 0..=max_entry {
        for right in 0..=max_entry {
            let matches =
                (0..s.cols).all(|col| left * s.get(0, col) + right * s.get(1, col) == target[col]);
            if matches {
                solutions.push([left, right]);
            }
        }
    }
    solutions
}

fn recurse_balanced_right_factor_rows(
    idx: usize,
    row_solutions: &[Vec<[u32; 2]>],
    current_rows: &mut [[u32; 2]],
    factors: &mut Vec<DynMatrix>,
) {
    if idx == current_rows.len() {
        let mut data = Vec::with_capacity(current_rows.len() * 2);
        for row in current_rows.iter() {
            data.push(row[0]);
            data.push(row[1]);
        }
        factors.push(DynMatrix::new(current_rows.len(), 2, data));
        return;
    }

    for solution in &row_solutions[idx] {
        current_rows[idx] = *solution;
        recurse_balanced_right_factor_rows(idx + 1, row_solutions, current_rows, factors);
    }
}

fn enumerate_matrix_data(len: usize, max_entry: u32) -> MatrixDataIter {
    MatrixDataIter::new(len, max_entry)
}

struct MatrixDataIter {
    data: Vec<u32>,
    max_entry: u32,
    done: bool,
}

impl MatrixDataIter {
    fn new(len: usize, max_entry: u32) -> Self {
        Self {
            data: vec![0; len],
            max_entry,
            done: false,
        }
    }
}

impl Iterator for MatrixDataIter {
    type Item = Vec<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let current = self.data.clone();
        for idx in (0..self.data.len()).rev() {
            if self.data[idx] < self.max_entry {
                self.data[idx] += 1;
                for reset_idx in idx + 1..self.data.len() {
                    self.data[reset_idx] = 0;
                }
                return Some(current);
            }
        }

        self.done = true;
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_outsplit_states_3x3(source: &SqMatrix<2>) -> Vec<DynMatrix> {
        let mut seen = BTreeSet::new();
        let mut states = Vec::new();
        for witness in enumerate_outsplits_2x2_to_3x3(source) {
            let canon = witness.outsplit.canonical_perm();
            if seen.insert(canon.clone()) {
                states.push(canon);
            }
        }
        states
    }

    fn canonical_insplit_states_3x3(source: &SqMatrix<2>) -> Vec<DynMatrix> {
        let mut seen = BTreeSet::new();
        let mut states = Vec::new();
        for witness in enumerate_insplits_2x2_to_3x3(source) {
            let canon = witness.outsplit.canonical_perm();
            if seen.insert(canon.clone()) {
                states.push(canon);
            }
        }
        states
    }

    fn collect_balanced_bridge_return_hits_3x3(
        source_candidates: &[DynMatrix],
        target_candidates: &[DynMatrix],
        bridge_max_entry: u32,
        config: &BalancedSearchConfig2x2,
    ) -> Vec<(DynMatrix, BalancedBridgeReturnNeighbor3x3)> {
        let target_set = target_candidates.iter().cloned().collect::<BTreeSet<_>>();
        let mut hits = Vec::new();

        for source in source_candidates {
            for neighbor in
                enumerate_balanced_bridge_return_neighbors_3x3(source, bridge_max_entry, config)
            {
                if target_set.contains(&neighbor.matrix) {
                    hits.push((source.clone(), neighbor));
                }
            }
        }

        hits
    }

    fn collect_balanced_bridge_insplit_return_hits_3x3(
        source_candidates: &[DynMatrix],
        target_candidates: &[DynMatrix],
        bridge_max_entry: u32,
        config: &BalancedSearchConfig2x2,
    ) -> Vec<(DynMatrix, BalancedBridgeReturnNeighbor3x3)> {
        let target_set = target_candidates.iter().cloned().collect::<BTreeSet<_>>();
        let mut hits = Vec::new();

        for source in source_candidates {
            for neighbor in enumerate_balanced_bridge_insplit_return_neighbors_3x3(
                source,
                bridge_max_entry,
                config,
            ) {
                if target_set.contains(&neighbor.matrix) {
                    hits.push((source.clone(), neighbor));
                }
            }
        }

        hits
    }

    #[test]
    fn test_verify_balanced_elementary_positive_example() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 1]]);
        let witness = BalancedElementaryWitness2x2 {
            s: DynMatrix::new(2, 1, vec![1, 1]),
            r_a: DynMatrix::new(1, 2, vec![1, 0]),
            r_b: DynMatrix::new(1, 2, vec![0, 1]),
        };
        assert!(verify_balanced_elementary_witness_2x2(&a, &b, &witness).is_ok());
    }

    #[test]
    fn test_search_balanced_elementary_positive_example() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 1]]);
        let result = find_balanced_elementary_equivalence_2x2(
            &a,
            &b,
            &BalancedSearchConfig2x2 {
                max_common_dim: 1,
                max_entry: 1,
            },
        );
        match result {
            BalancedSearchResult2x2::Equivalent(witness) => {
                assert!(verify_balanced_elementary_witness_2x2(&a, &b, &witness).is_ok());
            }
            BalancedSearchResult2x2::Exhausted => panic!("expected a balanced witness"),
        }
    }

    #[test]
    fn test_brix_ruiz_k3_balanced_elementary_exhausted() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let result = find_balanced_elementary_equivalence_2x2(
            &a,
            &b,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );
        assert_eq!(result, BalancedSearchResult2x2::Exhausted);
    }

    #[test]
    fn test_brix_ruiz_k4_balanced_elementary_exhausted() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        let result = find_balanced_elementary_equivalence_2x2(
            &a,
            &b,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );
        assert_eq!(result, BalancedSearchResult2x2::Exhausted);
    }

    #[test]
    fn test_enumerate_balanced_neighbors_toy_contains_swap_partner() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 1]]);
        let neighbors = enumerate_balanced_elementary_neighbors_2x2(
            &a,
            &BalancedSearchConfig2x2 {
                max_common_dim: 1,
                max_entry: 1,
            },
        );
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].matrix, b);
        assert!(verify_balanced_elementary_witness_2x2(&a, &b, &neighbors[0].witness).is_ok());
    }

    #[test]
    fn test_enumerate_balanced_neighbors_excludes_source_matrix() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let neighbors = enumerate_balanced_elementary_neighbors_2x2(
            &a,
            &BalancedSearchConfig2x2 {
                max_common_dim: 1,
                max_entry: 1,
            },
        );
        assert!(neighbors.iter().all(|neighbor| neighbor.matrix != a));
    }

    #[test]
    fn test_brix_ruiz_k3_balanced_zigzag_meeting_exhausted() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let result = find_balanced_elementary_zigzag_meeting_2x2(
            &a,
            &b,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_toy_outsplit_bridge_states_admit_balanced_neighbor_hit() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 1]]);
        let a_bridges = enumerate_outsplit_bridge_states_2x2(&a, 1);
        let b_bridges = enumerate_outsplit_bridge_states_2x2(&b, 1);

        let hits = enumerate_balanced_neighbor_set_hits_2x2(
            &a_bridges,
            &b_bridges,
            &BalancedSearchConfig2x2 {
                max_common_dim: 1,
                max_entry: 1,
            },
        );

        assert!(!hits.is_empty());
        for hit in hits {
            assert!(b_bridges.contains(&hit.target));
            assert!(
                verify_balanced_elementary_witness_2x2(&hit.source, &hit.target, &hit.witness)
                    .is_ok()
            );
        }
    }

    #[test]
    fn test_brix_ruiz_k3_outsplit_bridges_have_no_balanced_neighbor_hit() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let a_bridges = enumerate_outsplit_bridge_states_2x2(&a, 8);
        let b_bridges = enumerate_outsplit_bridge_states_2x2(&b, 8);

        let hits = enumerate_balanced_neighbor_set_hits_2x2(
            &a_bridges,
            &b_bridges,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );

        assert!(hits.is_empty());
    }

    #[test]
    fn test_brix_ruiz_k4_outsplit_bridges_have_no_balanced_neighbor_hit() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        let a_bridges = enumerate_outsplit_bridge_states_2x2(&a, 8);
        let b_bridges = enumerate_outsplit_bridge_states_2x2(&b, 8);

        let hits = enumerate_balanced_neighbor_set_hits_2x2(
            &a_bridges,
            &b_bridges,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );

        assert!(hits.is_empty());
    }

    #[test]
    fn test_toy_balanced_bridge_return_hits_3x3_outsplit_state() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 1]]);
        let a_states = canonical_outsplit_states_3x3(&a);
        let b_states = canonical_outsplit_states_3x3(&b);

        let hits = collect_balanced_bridge_return_hits_3x3(
            &a_states,
            &b_states,
            1,
            &BalancedSearchConfig2x2 {
                max_common_dim: 1,
                max_entry: 1,
            },
        );

        assert_eq!(hits.len(), 4);
        for (_, hit) in hits {
            assert!(b_states.contains(&hit.matrix));
            assert_eq!(hit.matrix.rows, 3);
            assert_eq!(hit.matrix.cols, 3);
            assert!(verify_balanced_elementary_witness_2x2(
                &hit.source_bridge,
                &hit.target_bridge,
                &hit.witness
            )
            .is_ok());
        }
    }

    #[test]
    fn test_brix_ruiz_k3_has_no_balanced_bridge_return_hit() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let a_states = canonical_outsplit_states_3x3(&a);
        let b_states = canonical_outsplit_states_3x3(&b);

        let hits = collect_balanced_bridge_return_hits_3x3(
            &a_states,
            &b_states,
            8,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );

        assert!(hits.is_empty());
    }

    #[test]
    fn test_brix_ruiz_k4_has_no_balanced_bridge_return_hit() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        let a_states = canonical_outsplit_states_3x3(&a);
        let b_states = canonical_outsplit_states_3x3(&b);

        let hits = collect_balanced_bridge_return_hits_3x3(
            &a_states,
            &b_states,
            8,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );

        assert!(hits.is_empty());
    }

    #[test]
    fn test_toy_balanced_bridge_insplit_return_hits_3x3_insplit_state() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 1]]);
        let a_source_states = canonical_outsplit_states_3x3(&a);
        let b_target_states = canonical_insplit_states_3x3(&b);

        let hits = collect_balanced_bridge_insplit_return_hits_3x3(
            &a_source_states,
            &b_target_states,
            1,
            &BalancedSearchConfig2x2 {
                max_common_dim: 1,
                max_entry: 1,
            },
        );

        assert_eq!(hits.len(), 4);
        for (_, hit) in hits {
            assert!(b_target_states.contains(&hit.matrix));
            assert_eq!(hit.matrix.rows, 3);
            assert_eq!(hit.matrix.cols, 3);
            assert!(verify_balanced_elementary_witness_2x2(
                &hit.source_bridge,
                &hit.target_bridge,
                &hit.witness
            )
            .is_ok());
        }
    }

    #[test]
    fn test_brix_ruiz_k3_has_no_balanced_bridge_insplit_return_hit() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let a_source_states = canonical_outsplit_states_3x3(&a);
        let b_target_states = canonical_insplit_states_3x3(&b);

        let hits = collect_balanced_bridge_insplit_return_hits_3x3(
            &a_source_states,
            &b_target_states,
            8,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );

        assert!(hits.is_empty());
    }

    #[test]
    fn test_brix_ruiz_k4_has_no_balanced_bridge_insplit_return_hit() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        let a_source_states = canonical_outsplit_states_3x3(&a);
        let b_target_states = canonical_insplit_states_3x3(&b);

        let hits = collect_balanced_bridge_insplit_return_hits_3x3(
            &a_source_states,
            &b_target_states,
            8,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );

        assert!(hits.is_empty());
    }
}
