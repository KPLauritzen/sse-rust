use std::collections::BTreeMap;

use ahash::{AHashMap as HashMap, AHashSet as HashSet};

use crate::factorisation::enumerate_factorisations_3x3_to_2;
use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::EsseStep;

/// One explicit one-step out-split witness.
///
/// The matrices satisfy `A = D E` and `C = E D`, where `D` is a division matrix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutsplitWitness {
    pub division: DynMatrix,
    pub edge: DynMatrix,
    pub outsplit: DynMatrix,
}

pub type OutsplitWitness2x2To3x3 = OutsplitWitness;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphMoveSuccessor {
    pub family: &'static str,
    pub matrix: DynMatrix,
    pub orig_matrix: DynMatrix,
    pub step: EsseStep,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphMoveSuccessors {
    pub candidates: usize,
    pub family_candidates: BTreeMap<&'static str, usize>,
    pub nodes: Vec<GraphMoveSuccessor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphMoveNode {
    pub family: &'static str,
    pub matrix: DynMatrix,
    pub orig_matrix: DynMatrix,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphMoveNodes {
    pub candidates: usize,
    pub family_candidates: BTreeMap<&'static str, usize>,
    pub nodes: Vec<GraphMoveNode>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SameFuturePastClassSignature {
    pub multiplicity: usize,
    pub entry_sum: u32,
    pub support: u8,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SameFuturePastSignature {
    pub dim: usize,
    pub entry_sum: u64,
    pub row_classes: Vec<SameFuturePastClassSignature>,
    pub col_classes: Vec<SameFuturePastClassSignature>,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct SameFuturePastSignatureGap {
    pub dimension_gap: usize,
    pub row_class_gap: usize,
    pub col_class_gap: usize,
    pub entry_sum_gap: u64,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct QuotientCellSignature {
    opposite_multiplicity: usize,
    opposite_entry_sum: u32,
    opposite_support: u8,
    value: u32,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct PartitionRefinedClassSignature {
    multiplicity: usize,
    entry_sum: u32,
    support: u8,
    quotient_profile: Vec<QuotientCellSignature>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PartitionRefinedSignature {
    dim: usize,
    entry_sum: u64,
    row_classes: Vec<PartitionRefinedClassSignature>,
    col_classes: Vec<PartitionRefinedClassSignature>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct DuplicateVectorClass {
    signature: SameFuturePastClassSignature,
    representative: Vec<u32>,
    example_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphProposal {
    /// Proposal families that produced this canonical matrix.
    pub families: Vec<&'static str>,
    pub matrix: DynMatrix,
    pub orig_matrix: DynMatrix,
    /// First available one-step witness for the proposal, when the proposal
    /// comes directly from a graph move family.
    pub step: Option<EsseStep>,
    pub target_signature_gap: SameFuturePastSignatureGap,
    pub target_partition_refined_gap: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphProposals {
    pub candidates: usize,
    pub family_candidates: BTreeMap<&'static str, usize>,
    pub nodes: Vec<GraphProposal>,
}

impl GraphProposals {
    pub fn best_gap(&self) -> Option<SameFuturePastSignatureGap> {
        self.nodes
            .first()
            .map(|proposal| proposal.target_signature_gap)
    }

    pub fn best_gap_shortlist_len(&self) -> usize {
        let Some(best_gap) = self.best_gap() else {
            return 0;
        };
        self.nodes
            .iter()
            .take_while(|proposal| proposal.target_signature_gap == best_gap)
            .count()
    }

    pub fn best_gap_shortlist(&self, limit: usize) -> Vec<GraphProposal> {
        let Some(best_gap) = self.best_gap() else {
            return Vec::new();
        };
        self.nodes
            .iter()
            .take_while(|proposal| proposal.target_signature_gap == best_gap)
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn refined_shortlist_from_coarse_prefix(
        &self,
        coarse_prefix: usize,
        limit: usize,
    ) -> Vec<GraphProposal> {
        let mut shortlist = self
            .nodes
            .iter()
            .take(coarse_prefix)
            .cloned()
            .collect::<Vec<_>>();
        shortlist.sort_by(|left, right| {
            left.target_partition_refined_gap
                .cmp(&right.target_partition_refined_gap)
                .then_with(|| left.target_signature_gap.cmp(&right.target_signature_gap))
                .then_with(|| left.matrix.cmp(&right.matrix))
        });
        shortlist.truncate(limit);
        shortlist
    }
}

/// Two successive out-splits starting from a 2x2 matrix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoStepOutsplitChain2x2 {
    pub first: OutsplitWitness,
    pub second: OutsplitWitness,
}

/// Enumerate all one-step out-splits of a square nonnegative matrix.
pub fn enumerate_one_step_outsplits(a: &DynMatrix) -> Vec<OutsplitWitness> {
    assert!(a.is_square());

    let parent_count = a.rows;
    let child_count = parent_count + 1;
    let assignments = child_parent_assignments(child_count, parent_count);
    let parent_rows: Vec<Vec<u32>> = (0..parent_count)
        .map(|row| (0..parent_count).map(|col| a.get(row, col)).collect())
        .collect();

    let mut witnesses = Vec::new();
    for assignment in assignments {
        let division = division_matrix_from_assignment(&assignment, parent_count);
        let child_rows_by_parent = children_by_parent(&assignment, parent_count);
        let split_options_by_parent: Vec<Vec<Vec<Vec<u32>>>> = parent_rows
            .iter()
            .zip(&child_rows_by_parent)
            .map(|(row, children)| split_row_into_children(row, children.len()))
            .collect();
        let mut child_rows = vec![vec![0u32; parent_count]; child_count];
        recurse_outsplit_rows(
            0,
            &child_rows_by_parent,
            &split_options_by_parent,
            &mut child_rows,
            &division,
            a,
            &mut witnesses,
        );
    }

    witnesses
}

/// Enumerate all one-step 2x2 -> 3x3 out-splits of a 2x2 nonnegative matrix.
pub fn enumerate_outsplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    enumerate_one_step_outsplits(&DynMatrix::from_sq(a))
}

/// Enumerate all one-step in-splits of a square nonnegative matrix.
///
/// This is implemented as out-splitting the transpose and transposing back.
pub fn enumerate_one_step_insplits(a: &DynMatrix) -> Vec<OutsplitWitness> {
    enumerate_one_step_outsplits(&a.transpose())
        .into_iter()
        .map(|witness| OutsplitWitness {
            division: witness.division.transpose(),
            edge: witness.edge.transpose(),
            outsplit: witness.outsplit.transpose(),
        })
        .collect()
}

/// Enumerate all one-step 2x2 -> 3x3 in-splits of a 2x2 nonnegative matrix.
pub fn enumerate_insplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    enumerate_one_step_insplits(&DynMatrix::from_sq(a))
}

/// Enumerate one-step in-splits whose output has two vertices with the same future,
/// i.e. two equal rows in the refined adjacency matrix.
pub fn enumerate_same_future_insplits(a: &DynMatrix) -> Vec<OutsplitWitness> {
    enumerate_one_step_insplits(a)
        .into_iter()
        .filter(|witness| has_duplicate_rows(&witness.outsplit))
        .collect()
}

/// Enumerate all one-step 2x2 -> 3x3 same-future in-splits.
pub fn enumerate_same_future_insplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    enumerate_same_future_insplits(&DynMatrix::from_sq(a))
}

/// Enumerate one-step out-splits whose output has two vertices with the same past,
/// i.e. two equal columns in the refined adjacency matrix.
pub fn enumerate_same_past_outsplits(a: &DynMatrix) -> Vec<OutsplitWitness> {
    enumerate_one_step_outsplits(a)
        .into_iter()
        .filter(|witness| has_duplicate_columns(&witness.outsplit))
        .collect()
}

/// Enumerate all one-step 2x2 -> 3x3 same-past out-splits.
pub fn enumerate_same_past_outsplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    enumerate_same_past_outsplits(&DynMatrix::from_sq(a))
}

/// Enumerate all canonical one-step split refinements, allowing either split direction.
pub fn enumerate_one_step_split_refinements(a: &DynMatrix) -> Vec<DynMatrix> {
    let mut seen = HashSet::new();
    let mut refinements = Vec::new();

    for witness in enumerate_one_step_outsplits(a)
        .into_iter()
        .chain(enumerate_one_step_insplits(a).into_iter())
    {
        let canon = witness.outsplit.canonical_perm();
        if seen.insert(canon.clone()) {
            refinements.push(canon);
        }
    }

    refinements
}

/// Compute the coarse same-future/same-past quotient signature of a square matrix.
pub fn same_future_past_signature(m: &DynMatrix) -> Option<SameFuturePastSignature> {
    if !m.is_square() {
        return None;
    }

    let mut entry_sum = 0u64;
    let mut row_vectors = Vec::with_capacity(m.rows);
    for row in 0..m.rows {
        let mut values = Vec::with_capacity(m.cols);
        for col in 0..m.cols {
            let value = m.get(row, col);
            entry_sum += value as u64;
            values.push(value);
        }
        row_vectors.push(values);
    }

    let mut col_vectors = Vec::with_capacity(m.cols);
    for col in 0..m.cols {
        let mut values = Vec::with_capacity(m.rows);
        for row in 0..m.rows {
            values.push(m.get(row, col));
        }
        col_vectors.push(values);
    }

    Some(SameFuturePastSignature {
        dim: m.rows,
        entry_sum,
        row_classes: duplicate_vector_classes(&row_vectors),
        col_classes: duplicate_vector_classes(&col_vectors),
    })
}

/// Coarse lexicographic gap between two same-future/same-past signatures.
pub fn same_future_past_signature_gap(
    left: &SameFuturePastSignature,
    right: &SameFuturePastSignature,
) -> SameFuturePastSignatureGap {
    SameFuturePastSignatureGap {
        dimension_gap: left.dim.abs_diff(right.dim),
        row_class_gap: class_signature_gap(&left.row_classes, &right.row_classes),
        col_class_gap: class_signature_gap(&left.col_classes, &right.col_classes),
        entry_sum_gap: left.entry_sum.abs_diff(right.entry_sum),
    }
}

/// Scalar total for the coarse same-future/same-past quotient gap.
pub fn same_future_past_signature_gap_total(left: &DynMatrix, right: &DynMatrix) -> u64 {
    let left_sig =
        same_future_past_signature(left).expect("square matrix should always have a signature");
    let right_sig =
        same_future_past_signature(right).expect("square matrix should always have a signature");
    let gap = same_future_past_signature_gap(&left_sig, &right_sig);
    10 * gap.dimension_gap as u64
        + gap.row_class_gap as u64
        + gap.col_class_gap as u64
        + gap.entry_sum_gap
}

/// Scalar total for a one-step partition-refined quotient gap.
///
/// This keeps the existing same-future/same-past duplicate-class partition, then
/// compares the quotient-style block values induced between row and column
/// classes. It is intended for analysis and proposal shortlisting only.
pub fn partition_refined_same_future_past_gap_total(left: &DynMatrix, right: &DynMatrix) -> u64 {
    let left_sig = PartitionRefinedSignature::new(left);
    let right_sig = PartitionRefinedSignature::new(right);

    10 * left_sig.dim.abs_diff(right_sig.dim) as u64
        + left_sig.entry_sum.abs_diff(right_sig.entry_sum)
        + refined_class_signature_gap(&left_sig.row_classes, &right_sig.row_classes)
        + refined_class_signature_gap(&left_sig.col_classes, &right_sig.col_classes)
}

/// Enumerate research-only proposal candidates toward `target` from bounded graph families.
///
/// This does not affect default search expansion. It exists so proposal sources can
/// be inspected and compared against the blind one-step graph successor surface.
pub fn enumerate_graph_proposals(
    current: &DynMatrix,
    target: &DynMatrix,
    max_dim: usize,
    max_zigzag_bridge_entry: Option<u32>,
) -> GraphProposals {
    assert!(current.is_square());
    assert!(target.is_square());

    let target_signature = same_future_past_signature(target)
        .expect("square target should always produce a same-future/past signature");
    let target_partition_refined_signature = PartitionRefinedSignature::new(target);
    let current_canon = current.canonical_perm();
    let mut candidates = 0usize;
    let mut family_candidates = BTreeMap::new();
    let mut nodes = BTreeMap::<DynMatrix, GraphProposal>::new();

    if current.rows < max_dim {
        for witness in enumerate_same_future_insplits(current) {
            candidates += 1;
            *family_candidates.entry("same_future_insplit").or_default() += 1;
            record_graph_proposal(
                &mut nodes,
                "same_future_insplit",
                witness.outsplit,
                Some(EsseStep {
                    u: witness.edge,
                    v: witness.division,
                }),
                &current_canon,
                &target_signature,
                &target_partition_refined_signature,
            );
        }

        for witness in enumerate_same_past_outsplits(current) {
            candidates += 1;
            *family_candidates.entry("same_past_outsplit").or_default() += 1;
            record_graph_proposal(
                &mut nodes,
                "same_past_outsplit",
                witness.outsplit,
                Some(EsseStep {
                    u: witness.division,
                    v: witness.edge,
                }),
                &current_canon,
                &target_signature,
                &target_partition_refined_signature,
            );
        }
    }

    if current.rows > 2 {
        for witness in enumerate_out_amalgamations(current) {
            candidates += 1;
            *family_candidates.entry("out_amalgamation").or_default() += 1;
            record_graph_proposal(
                &mut nodes,
                "out_amalgamation",
                witness.outsplit,
                Some(EsseStep {
                    u: witness.edge,
                    v: witness.division,
                }),
                &current_canon,
                &target_signature,
                &target_partition_refined_signature,
            );
        }

        for witness in enumerate_in_amalgamations(current) {
            candidates += 1;
            *family_candidates.entry("in_amalgamation").or_default() += 1;
            record_graph_proposal(
                &mut nodes,
                "in_amalgamation",
                witness.outsplit,
                Some(EsseStep {
                    u: witness.division,
                    v: witness.edge,
                }),
                &current_canon,
                &target_signature,
                &target_partition_refined_signature,
            );
        }
    }

    if let Some(max_bridge_entry) = max_zigzag_bridge_entry {
        if current.rows == 3 && current.cols == 3 {
            for neighbor in enumerate_3x3_outsplit_zigzag_neighbors(current, max_bridge_entry) {
                candidates += 1;
                *family_candidates
                    .entry("outsplit_zigzag_neighbor")
                    .or_default() += 1;
                record_graph_proposal(
                    &mut nodes,
                    "outsplit_zigzag_neighbor",
                    neighbor,
                    None,
                    &current_canon,
                    &target_signature,
                    &target_partition_refined_signature,
                );
            }
        }
    }

    let mut nodes = nodes.into_values().collect::<Vec<_>>();
    nodes.sort_by(|left, right| {
        left.target_signature_gap
            .cmp(&right.target_signature_gap)
            .then_with(|| left.matrix.cmp(&right.matrix))
    });
    for proposal in &mut nodes {
        proposal.families.sort_unstable();
        proposal.families.dedup();
    }

    GraphProposals {
        candidates,
        family_candidates,
        nodes,
    }
}

/// Enumerate canonical successors reached by one graph split or amalgamation.
pub fn enumerate_graph_move_successors(current: &DynMatrix, max_dim: usize) -> GraphMoveSuccessors {
    assert!(current.is_square());

    let mut candidates = 0usize;
    let mut family_candidates = BTreeMap::new();
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();

    if current.rows < max_dim {
        append_representative_outsplit_successors(
            current,
            "outsplit",
            false,
            &mut candidates,
            &mut family_candidates,
            &mut seen,
            &mut nodes,
        );

        append_representative_outsplit_successors(
            &current.transpose(),
            "insplit",
            true,
            &mut candidates,
            &mut family_candidates,
            &mut seen,
            &mut nodes,
        );
    }

    if current.rows > 2 {
        for witness in enumerate_out_amalgamations(current) {
            candidates += 1;
            *family_candidates.entry("out_amalgamation").or_default() += 1;
            let step = EsseStep {
                u: witness.edge,
                v: witness.division,
            };
            push_canonical_graph_successor(
                "out_amalgamation",
                witness.outsplit,
                step,
                &mut seen,
                &mut nodes,
            );
        }
        for witness in enumerate_in_amalgamations(current) {
            candidates += 1;
            *family_candidates.entry("in_amalgamation").or_default() += 1;
            let step = EsseStep {
                u: witness.division,
                v: witness.edge,
            };
            push_canonical_graph_successor(
                "in_amalgamation",
                witness.outsplit,
                step,
                &mut seen,
                &mut nodes,
            );
        }
    }

    GraphMoveSuccessors {
        candidates,
        family_candidates,
        nodes,
    }
}

/// Enumerate canonical graph-move successors without materializing witnesses.
///
/// This is the cheaper surface used by graph-only search. Exact one-step
/// witnesses can be recovered later for a chosen matrix pair.
pub fn enumerate_graph_move_successor_nodes(current: &DynMatrix, max_dim: usize) -> GraphMoveNodes {
    assert!(current.is_square());

    let mut candidates = 0usize;
    let mut family_candidates = BTreeMap::new();
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();

    if current.rows < max_dim {
        append_representative_outsplit_successor_nodes(
            current,
            "outsplit",
            false,
            &mut candidates,
            &mut family_candidates,
            &mut seen,
            &mut nodes,
        );

        append_representative_outsplit_successor_nodes(
            &current.transpose(),
            "insplit",
            true,
            &mut candidates,
            &mut family_candidates,
            &mut seen,
            &mut nodes,
        );
    }

    if current.rows > 2 {
        for witness in enumerate_out_amalgamations(current) {
            candidates += 1;
            *family_candidates.entry("out_amalgamation").or_default() += 1;
            push_canonical_graph_successor_node(
                "out_amalgamation",
                witness.outsplit,
                &mut seen,
                &mut nodes,
            );
        }
        for witness in enumerate_in_amalgamations(current) {
            candidates += 1;
            *family_candidates.entry("in_amalgamation").or_default() += 1;
            push_canonical_graph_successor_node(
                "in_amalgamation",
                witness.outsplit,
                &mut seen,
                &mut nodes,
            );
        }
    }

    GraphMoveNodes {
        candidates,
        family_candidates,
        nodes,
    }
}

/// Return all one-step graph moves from `current` whose target is permutation-similar
/// to `target`.
pub fn find_graph_move_witnesses_between(
    current: &DynMatrix,
    target: &DynMatrix,
) -> Vec<GraphMoveSuccessor> {
    assert!(current.is_square());
    assert!(target.is_square());

    let max_dim = current.rows.max(target.rows);
    let target_canon = target.canonical_perm();

    enumerate_graph_move_successors(current, max_dim)
        .nodes
        .into_iter()
        .filter(|successor| successor.matrix == target_canon)
        .collect()
}

/// Return one exact one-step graph witness from `current` to `target`, when the
/// chosen representative matrix was kept by canonical successor deduplication.
pub fn find_exact_graph_move_witness_between(
    current: &DynMatrix,
    target: &DynMatrix,
) -> Option<GraphMoveSuccessor> {
    find_graph_move_witnesses_between(current, target)
        .into_iter()
        .find(|successor| successor.orig_matrix == *target)
}

fn append_representative_outsplit_successors(
    a: &DynMatrix,
    family: &'static str,
    transpose_result: bool,
    candidates: &mut usize,
    family_candidates: &mut BTreeMap<&'static str, usize>,
    seen: &mut HashSet<DynMatrix>,
    nodes: &mut Vec<GraphMoveSuccessor>,
) {
    debug_assert!(a.is_square());

    let parent_count = a.rows;
    let child_count = parent_count + 1;
    let parent_rows: Vec<Vec<u32>> = (0..parent_count)
        .map(|row| (0..parent_count).map(|col| a.get(row, col)).collect())
        .collect();

    for split_parent in 0..parent_count {
        let assignment: Vec<usize> = (0..child_count)
            .map(|child| {
                if child < parent_count {
                    child
                } else {
                    split_parent
                }
            })
            .collect();
        let division = division_matrix_from_assignment(&assignment, parent_count);

        for split in split_row_into_children(&parent_rows[split_parent], 2) {
            // The two split children are interchangeable up to permutation.
            if split[1] < split[0] {
                continue;
            }

            *candidates += 1;
            *family_candidates.entry(family).or_default() += 1;

            let mut child_rows = parent_rows.clone();
            child_rows[split_parent] = split[0].clone();
            child_rows.push(split[1].clone());

            let mut data = Vec::with_capacity(child_count * child_count);
            for row in &child_rows {
                for &parent in &assignment {
                    data.push(row[parent]);
                }
            }

            let edge = DynMatrix::new(
                child_count,
                parent_count,
                child_rows
                    .iter()
                    .flat_map(|row| row.iter())
                    .copied()
                    .collect(),
            );
            let outsplit = edge.mul(&division);

            if transpose_result {
                let division = division.transpose();
                let edge = edge.transpose();
                let outsplit = outsplit.transpose();
                let step = EsseStep {
                    u: edge.clone(),
                    v: division.clone(),
                };
                push_canonical_graph_successor(family, outsplit, step, seen, nodes);
            } else {
                let step = EsseStep {
                    u: division.clone(),
                    v: edge.clone(),
                };
                push_canonical_graph_successor(family, outsplit, step, seen, nodes);
            }
        }
    }
}

fn append_representative_outsplit_successor_nodes(
    a: &DynMatrix,
    family: &'static str,
    transpose_result: bool,
    candidates: &mut usize,
    family_candidates: &mut BTreeMap<&'static str, usize>,
    seen: &mut HashSet<DynMatrix>,
    nodes: &mut Vec<GraphMoveNode>,
) {
    debug_assert!(a.is_square());

    let parent_count = a.rows;
    let child_count = parent_count + 1;
    let parent_rows: Vec<Vec<u32>> = (0..parent_count)
        .map(|row| (0..parent_count).map(|col| a.get(row, col)).collect())
        .collect();

    for split_parent in 0..parent_count {
        let assignment: Vec<usize> = (0..child_count)
            .map(|child| {
                if child < parent_count {
                    child
                } else {
                    split_parent
                }
            })
            .collect();

        for split in split_row_into_children(&parent_rows[split_parent], 2) {
            // The two split children are interchangeable up to permutation.
            if split[1] < split[0] {
                continue;
            }

            *candidates += 1;
            *family_candidates.entry(family).or_default() += 1;

            let mut child_rows = parent_rows.clone();
            child_rows[split_parent] = split[0].clone();
            child_rows.push(split[1].clone());

            let edge = DynMatrix::new(
                child_count,
                parent_count,
                child_rows
                    .iter()
                    .flat_map(|row| row.iter())
                    .copied()
                    .collect(),
            );
            let outsplit = edge.mul(&division_matrix_from_assignment(&assignment, parent_count));

            if transpose_result {
                push_canonical_graph_successor_node(family, outsplit.transpose(), seen, nodes);
            } else {
                push_canonical_graph_successor_node(family, outsplit, seen, nodes);
            }
        }
    }
}

fn push_canonical_graph_successor(
    family: &'static str,
    matrix: DynMatrix,
    step: EsseStep,
    seen: &mut HashSet<DynMatrix>,
    nodes: &mut Vec<GraphMoveSuccessor>,
) {
    let canon = matrix.canonical_perm();
    if seen.insert(canon.clone()) {
        nodes.push(GraphMoveSuccessor {
            family,
            matrix: canon,
            orig_matrix: matrix,
            step,
        });
    }
}

fn push_canonical_graph_successor_node(
    family: &'static str,
    matrix: DynMatrix,
    seen: &mut HashSet<DynMatrix>,
    nodes: &mut Vec<GraphMoveNode>,
) {
    let canon = matrix.canonical_perm();
    if seen.insert(canon.clone()) {
        nodes.push(GraphMoveNode {
            family,
            matrix: canon,
            orig_matrix: matrix,
        });
    }
}

fn record_graph_proposal(
    nodes: &mut BTreeMap<DynMatrix, GraphProposal>,
    family: &'static str,
    matrix: DynMatrix,
    step: Option<EsseStep>,
    current_canon: &DynMatrix,
    target_signature: &SameFuturePastSignature,
    target_partition_refined_signature: &PartitionRefinedSignature,
) {
    let canon = matrix.canonical_perm();
    if canon == *current_canon {
        return;
    }
    let signature = same_future_past_signature(&canon)
        .expect("square graph proposal should always produce a same-future/past signature");
    let gap = same_future_past_signature_gap(&signature, target_signature);
    let partition_refined_signature = PartitionRefinedSignature::new(&canon);
    let partition_refined_gap = 10
        * partition_refined_signature
            .dim
            .abs_diff(target_partition_refined_signature.dim) as u64
        + partition_refined_signature
            .entry_sum
            .abs_diff(target_partition_refined_signature.entry_sum)
        + refined_class_signature_gap(
            &partition_refined_signature.row_classes,
            &target_partition_refined_signature.row_classes,
        )
        + refined_class_signature_gap(
            &partition_refined_signature.col_classes,
            &target_partition_refined_signature.col_classes,
        );

    match nodes.entry(canon.clone()) {
        std::collections::btree_map::Entry::Occupied(mut entry) => {
            if !entry.get().families.contains(&family) {
                entry.get_mut().families.push(family);
            }
            if entry.get().step.is_none() && step.is_some() {
                entry.get_mut().step = step;
            }
        }
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(GraphProposal {
                families: vec![family],
                matrix: canon,
                orig_matrix: matrix,
                step,
                target_signature_gap: gap,
                target_partition_refined_gap: partition_refined_gap,
            });
        }
    }
}

/// Search for a common one-step 3x3 out-split refinement up to permutation.
pub fn find_common_outsplit_refinement_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
) -> Option<(OutsplitWitness2x2To3x3, OutsplitWitness2x2To3x3)> {
    let a_outsplits = enumerate_outsplits_2x2_to_3x3(a);
    let b_outsplits = enumerate_outsplits_2x2_to_3x3(b);

    for left in &a_outsplits {
        let left_canon = left.outsplit.canonical_perm();
        for right in &b_outsplits {
            if left_canon == right.outsplit.canonical_perm() {
                return Some((left.clone(), right.clone()));
            }
        }
    }

    None
}

/// Search for a common two-step 4x4 out-split refinement up to permutation.
pub fn find_common_two_step_outsplit_refinement_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
) -> Option<(TwoStepOutsplitChain2x2, TwoStepOutsplitChain2x2)> {
    let mut left_refinements = HashMap::new();
    for first in enumerate_outsplits_2x2_to_3x3(a) {
        for second in enumerate_one_step_outsplits(&first.outsplit) {
            left_refinements
                .entry(second.outsplit.canonical_perm())
                .or_insert_with(|| TwoStepOutsplitChain2x2 {
                    first: first.clone(),
                    second,
                });
        }
    }

    for first in enumerate_outsplits_2x2_to_3x3(b) {
        for second in enumerate_one_step_outsplits(&first.outsplit) {
            let canon = second.outsplit.canonical_perm();
            if let Some(left) = left_refinements.get(&canon) {
                return Some((
                    left.clone(),
                    TwoStepOutsplitChain2x2 {
                        first: first.clone(),
                        second,
                    },
                ));
            }
        }
    }

    None
}

/// Enumerate canonical 3x3 neighbors reached by one `3x3 -> 2x2 -> 3x3` zig-zag.
pub fn enumerate_3x3_outsplit_zigzag_neighbors(
    c: &DynMatrix,
    max_bridge_entry: u32,
) -> Vec<DynMatrix> {
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let mut seen = HashSet::new();
    let mut neighbors = Vec::new();

    for (u, v) in enumerate_factorisations_3x3_to_2(c, max_bridge_entry) {
        let bridge = v
            .mul(&u)
            .to_sq::<2>()
            .expect("3x3-to-2 factorisation should produce a 2x2 bridge")
            .canonical();
        for witness in enumerate_outsplits_2x2_to_3x3(&bridge) {
            let canon = witness.outsplit.canonical_perm();
            if seen.insert(canon.clone()) {
                neighbors.push(canon);
            }
        }
    }

    neighbors
}

fn child_parent_assignments(child_count: usize, parent_count: usize) -> Vec<Vec<usize>> {
    let mut assignments = Vec::new();
    let mut current = vec![0usize; child_count];
    recurse_child_parent_assignments(0, child_count, parent_count, &mut current, &mut assignments);
    assignments
}

impl PartitionRefinedSignature {
    fn new(m: &DynMatrix) -> Self {
        assert!(m.is_square());

        let row_vectors = (0..m.rows)
            .map(|row| (0..m.cols).map(|col| m.get(row, col)).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let col_vectors = (0..m.cols)
            .map(|col| (0..m.rows).map(|row| m.get(row, col)).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let row_classes = duplicate_vector_classes_with_examples(&row_vectors);
        let col_classes = duplicate_vector_classes_with_examples(&col_vectors);

        Self {
            dim: m.rows,
            entry_sum: m.data.iter().map(|&value| value as u64).sum(),
            row_classes: partition_refined_classes(&row_classes, &col_classes),
            col_classes: partition_refined_classes(&col_classes, &row_classes),
        }
    }
}

fn duplicate_vector_classes(vectors: &[Vec<u32>]) -> Vec<SameFuturePastClassSignature> {
    duplicate_vector_classes_with_examples(vectors)
        .into_iter()
        .map(|class| class.signature)
        .collect()
}

fn duplicate_vector_classes_with_examples(vectors: &[Vec<u32>]) -> Vec<DuplicateVectorClass> {
    let mut multiplicities = BTreeMap::<Vec<u32>, usize>::new();
    let mut example_indices = BTreeMap::<Vec<u32>, usize>::new();
    for (index, values) in vectors.iter().enumerate() {
        *multiplicities.entry(values.clone()).or_default() += 1;
        example_indices.entry(values.clone()).or_insert(index);
    }

    let mut classes = multiplicities
        .into_iter()
        .map(|(values, multiplicity)| DuplicateVectorClass {
            signature: SameFuturePastClassSignature {
                multiplicity,
                entry_sum: values.iter().copied().sum(),
                support: values.iter().filter(|&&value| value > 0).count() as u8,
            },
            example_index: *example_indices
                .get(&values)
                .expect("duplicate class should retain an example index"),
            representative: values,
        })
        .collect::<Vec<_>>();
    classes.sort_by(|left, right| {
        left.signature
            .cmp(&right.signature)
            .then_with(|| left.representative.cmp(&right.representative))
    });
    classes
}

fn partition_refined_classes(
    classes: &[DuplicateVectorClass],
    opposite_classes: &[DuplicateVectorClass],
) -> Vec<PartitionRefinedClassSignature> {
    let mut refined = classes
        .iter()
        .map(|class| PartitionRefinedClassSignature {
            multiplicity: class.signature.multiplicity,
            entry_sum: class.signature.entry_sum,
            support: class.signature.support,
            quotient_profile: opposite_classes
                .iter()
                .map(|opposite| QuotientCellSignature {
                    opposite_multiplicity: opposite.signature.multiplicity,
                    opposite_entry_sum: opposite.signature.entry_sum,
                    opposite_support: opposite.signature.support,
                    value: class.representative[opposite.example_index],
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    for class in &mut refined {
        class.quotient_profile.sort_unstable();
    }
    refined.sort_unstable();
    refined
}

fn class_signature_gap(
    left: &[SameFuturePastClassSignature],
    right: &[SameFuturePastClassSignature],
) -> usize {
    let shared = left.len().min(right.len());
    let mut gap = 0usize;

    for (left_class, right_class) in left.iter().zip(right.iter()).take(shared) {
        gap += left_class.multiplicity.abs_diff(right_class.multiplicity);
        gap += left_class.entry_sum.abs_diff(right_class.entry_sum) as usize;
        gap += left_class.support.abs_diff(right_class.support) as usize;
    }

    for extra in &left[shared..] {
        gap += extra.multiplicity + extra.entry_sum as usize + extra.support as usize;
    }
    for extra in &right[shared..] {
        gap += extra.multiplicity + extra.entry_sum as usize + extra.support as usize;
    }

    gap
}

fn refined_class_signature_gap(
    left: &[PartitionRefinedClassSignature],
    right: &[PartitionRefinedClassSignature],
) -> u64 {
    let len = left.len().max(right.len());
    let mut gap = 0u64;
    for idx in 0..len {
        let left_class = left.get(idx);
        let right_class = right.get(idx);
        gap += left_class
            .map(|class| class.multiplicity)
            .unwrap_or(0)
            .abs_diff(right_class.map(|class| class.multiplicity).unwrap_or(0))
            as u64;
        gap += left_class
            .map(|class| class.entry_sum)
            .unwrap_or(0)
            .abs_diff(right_class.map(|class| class.entry_sum).unwrap_or(0)) as u64;
        gap += left_class
            .map(|class| class.support)
            .unwrap_or(0)
            .abs_diff(right_class.map(|class| class.support).unwrap_or(0)) as u64;
        gap += quotient_profile_gap(
            left_class
                .map(|class| class.quotient_profile.as_slice())
                .unwrap_or(&[]),
            right_class
                .map(|class| class.quotient_profile.as_slice())
                .unwrap_or(&[]),
        );
    }
    gap
}

fn quotient_profile_gap(left: &[QuotientCellSignature], right: &[QuotientCellSignature]) -> u64 {
    let len = left.len().max(right.len());
    let mut gap = 0u64;
    for idx in 0..len {
        let left_cell = left.get(idx);
        let right_cell = right.get(idx);
        gap += left_cell
            .map(|cell| cell.opposite_multiplicity)
            .unwrap_or(0)
            .abs_diff(
                right_cell
                    .map(|cell| cell.opposite_multiplicity)
                    .unwrap_or(0),
            ) as u64;
        gap += left_cell
            .map(|cell| cell.opposite_entry_sum)
            .unwrap_or(0)
            .abs_diff(right_cell.map(|cell| cell.opposite_entry_sum).unwrap_or(0))
            as u64;
        gap += left_cell
            .map(|cell| cell.opposite_support)
            .unwrap_or(0)
            .abs_diff(right_cell.map(|cell| cell.opposite_support).unwrap_or(0))
            as u64;
        gap += left_cell
            .map(|cell| cell.value)
            .unwrap_or(0)
            .abs_diff(right_cell.map(|cell| cell.value).unwrap_or(0)) as u64;
    }
    gap
}

fn has_duplicate_rows(a: &DynMatrix) -> bool {
    for i in 0..a.rows {
        for j in i + 1..a.rows {
            let equal = (0..a.cols).all(|col| a.get(i, col) == a.get(j, col));
            if equal {
                return true;
            }
        }
    }
    false
}

fn has_duplicate_columns(a: &DynMatrix) -> bool {
    for i in 0..a.cols {
        for j in i + 1..a.cols {
            let equal = (0..a.rows).all(|row| a.get(row, i) == a.get(row, j));
            if equal {
                return true;
            }
        }
    }
    false
}

fn recurse_child_parent_assignments(
    idx: usize,
    child_count: usize,
    parent_count: usize,
    current: &mut [usize],
    assignments: &mut Vec<Vec<usize>>,
) {
    if idx == child_count {
        let mut seen = vec![false; parent_count];
        for &parent in current.iter() {
            seen[parent] = true;
        }
        if seen.iter().all(|used| *used) {
            assignments.push(current.to_vec());
        }
        return;
    }

    for parent in 0..parent_count {
        current[idx] = parent;
        recurse_child_parent_assignments(idx + 1, child_count, parent_count, current, assignments);
    }
}

fn division_matrix_from_assignment(assignment: &[usize], parent_count: usize) -> DynMatrix {
    let child_count = assignment.len();
    let mut data = vec![0u32; parent_count * child_count];
    for (child, &parent) in assignment.iter().enumerate() {
        data[parent * child_count + child] = 1;
    }
    DynMatrix::new(parent_count, child_count, data)
}

fn children_by_parent(assignment: &[usize], parent_count: usize) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); parent_count];
    for (child, &parent) in assignment.iter().enumerate() {
        children[parent].push(child);
    }
    children
}

fn split_row_into_children(row: &[u32], children: usize) -> Vec<Vec<Vec<u32>>> {
    if children == 0 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut current = vec![vec![0u32; row.len()]; children];
    recurse_split_row_columns(0, row, &mut current, &mut results);
    results
}

fn recurse_split_row_columns(
    col_idx: usize,
    row: &[u32],
    current: &mut [Vec<u32>],
    results: &mut Vec<Vec<Vec<u32>>>,
) {
    if col_idx == row.len() {
        results.push(current.to_vec());
        return;
    }

    let compositions = compositions(row[col_idx], current.len());
    for composition in compositions {
        for (child_idx, value) in composition.into_iter().enumerate() {
            current[child_idx][col_idx] = value;
        }
        recurse_split_row_columns(col_idx + 1, row, current, results);
    }
}

fn compositions(total: u32, parts: usize) -> Vec<Vec<u32>> {
    if parts == 0 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut current = vec![0u32; parts];
    recurse_compositions(0, total, &mut current, &mut results);
    results
}

fn recurse_compositions(
    idx: usize,
    remaining: u32,
    current: &mut [u32],
    results: &mut Vec<Vec<u32>>,
) {
    if idx + 1 == current.len() {
        current[idx] = remaining;
        results.push(current.to_vec());
        return;
    }

    for value in 0..=remaining {
        current[idx] = value;
        recurse_compositions(idx + 1, remaining - value, current, results);
    }
}

fn recurse_outsplit_rows(
    parent_idx: usize,
    child_rows_by_parent: &[Vec<usize>],
    split_options_by_parent: &[Vec<Vec<Vec<u32>>>],
    child_rows: &mut [Vec<u32>],
    division: &DynMatrix,
    a: &DynMatrix,
    witnesses: &mut Vec<OutsplitWitness>,
) {
    if parent_idx == child_rows_by_parent.len() {
        let edge = DynMatrix::new(
            child_rows.len(),
            a.cols,
            child_rows
                .iter()
                .flat_map(|row| row.iter())
                .copied()
                .collect(),
        );
        let outsplit = edge.mul(division);
        debug_assert_eq!(division.mul(&edge), *a);
        witnesses.push(OutsplitWitness {
            division: division.clone(),
            edge,
            outsplit,
        });
        return;
    }

    for split in &split_options_by_parent[parent_idx] {
        for (child_idx, row) in child_rows_by_parent[parent_idx]
            .iter()
            .copied()
            .zip(split.iter())
        {
            child_rows[child_idx].clone_from(row);
        }
        recurse_outsplit_rows(
            parent_idx + 1,
            child_rows_by_parent,
            split_options_by_parent,
            child_rows,
            division,
            a,
            witnesses,
        );
    }
}

/// Enumerate all one-step out-amalgamations of a square matrix.
///
/// An out-amalgamation merges two states whose columns are identical
/// ("same past"). This is the reverse of an out-split. For each pair of
/// identical columns (p, q), the merged matrix has dimension m-1.
///
/// The SSE step satisfies C = E·D and result = D·E, where D is a 0-1
/// division matrix.
pub fn enumerate_out_amalgamations(c: &DynMatrix) -> Vec<OutsplitWitness> {
    assert!(c.is_square());
    let m = c.rows;
    if m <= 1 {
        return vec![];
    }

    let mut witnesses = Vec::new();

    for p in 0..m {
        for q in (p + 1)..m {
            // Check if columns p and q are identical.
            let cols_equal = (0..m).all(|row| c.get(row, p) == c.get(row, q));
            if !cols_equal {
                continue;
            }

            let n = m - 1;

            // Build mapping: old state k -> new state f[k].
            // State q merges into state p; states after q shift down by 1.
            let f: Vec<usize> = (0..m)
                .map(|k| {
                    if k < q {
                        k
                    } else if k == q {
                        p
                    } else {
                        k - 1
                    }
                })
                .collect();

            // D (n×m): division matrix, D[f[k], k] = 1.
            let mut d_data = vec![0u32; n * m];
            for k in 0..m {
                d_data[f[k] * m + k] = 1;
            }
            let division = DynMatrix::new(n, m, d_data);

            // E (m×n): E[i, g] = C[i, representative(g)].
            // Representative: for group g, pick the first old state mapping to g.
            let mut e_data = vec![0u32; m * n];
            for i in 0..m {
                for g in 0..n {
                    let repr = if g < q { g } else { g + 1 };
                    e_data[i * n + g] = c.get(i, repr);
                }
            }
            let edge = DynMatrix::new(m, n, e_data);

            debug_assert_eq!(edge.mul(&division), *c);

            let result = division.mul(&edge);

            witnesses.push(OutsplitWitness {
                division,
                edge,
                outsplit: result,
            });
        }
    }

    witnesses
}

/// Enumerate all one-step in-amalgamations of a square matrix.
///
/// An in-amalgamation merges two states whose rows are identical
/// ("same future"). This is the reverse of an in-split.
/// Implemented by transposing, out-amalgamating, and transposing back.
pub fn enumerate_in_amalgamations(c: &DynMatrix) -> Vec<OutsplitWitness> {
    enumerate_out_amalgamations(&c.transpose())
        .into_iter()
        .map(|witness| OutsplitWitness {
            division: witness.division.transpose(),
            edge: witness.edge.transpose(),
            outsplit: witness.outsplit.transpose(),
        })
        .collect()
}

#[doc(hidden)]
pub mod profiling_helpers {
    pub fn split_row_into_children(row: &[u32], children: usize) -> Vec<Vec<Vec<u32>>> {
        super::split_row_into_children(row, children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_outsplits_2x2_to_3x3_nonempty() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let witnesses = enumerate_outsplits_2x2_to_3x3(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.division.rows, 2);
            assert_eq!(witness.division.cols, 3);
            assert_eq!(witness.edge.rows, 3);
            assert_eq!(witness.edge.cols, 2);
            assert_eq!(witness.outsplit.rows, 3);
            assert_eq!(witness.outsplit.cols, 3);
            assert_eq!(witness.division.mul(&witness.edge), DynMatrix::from_sq(&a));
        }
    }

    #[test]
    fn test_enumerate_one_step_outsplits_3x3_nonempty() {
        let a = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 3, 2, 0, 1]);
        let witnesses = enumerate_one_step_outsplits(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.division.rows, 3);
            assert_eq!(witness.division.cols, 4);
            assert_eq!(witness.edge.rows, 4);
            assert_eq!(witness.edge.cols, 3);
            assert_eq!(witness.outsplit.rows, 4);
            assert_eq!(witness.outsplit.cols, 4);
            assert_eq!(witness.division.mul(&witness.edge), a);
        }
    }

    #[test]
    fn test_enumerate_one_step_insplits_3x3_nonempty() {
        let a = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 3, 2, 0, 1]);
        let witnesses = enumerate_one_step_insplits(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.edge.rows, 3);
            assert_eq!(witness.edge.cols, 4);
            assert_eq!(witness.division.rows, 4);
            assert_eq!(witness.division.cols, 3);
            assert_eq!(witness.outsplit.rows, 4);
            assert_eq!(witness.outsplit.cols, 4);
            assert_eq!(witness.edge.mul(&witness.division), a);
        }
    }

    #[test]
    fn test_representative_outsplit_successors_match_full_enumeration() {
        let matrices = [
            DynMatrix::from_sq(&SqMatrix::new([[1, 3], [2, 1]])),
            DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 3, 2, 0, 1]),
        ];

        for a in matrices {
            let expected = enumerate_one_step_outsplits(&a)
                .into_iter()
                .map(|witness| witness.outsplit.canonical_perm())
                .collect::<HashSet<_>>();
            let mut family_candidates = BTreeMap::new();
            let mut seen = HashSet::new();
            let mut nodes = Vec::new();
            let mut candidates = 0;
            append_representative_outsplit_successors(
                &a,
                "outsplit",
                false,
                &mut candidates,
                &mut family_candidates,
                &mut seen,
                &mut nodes,
            );
            let actual = nodes
                .into_iter()
                .map(|node| node.matrix)
                .collect::<HashSet<_>>();
            assert_eq!(actual, expected);
            assert!(candidates <= expected.len() * 2);
        }
    }

    #[test]
    fn test_graph_move_successors_include_valid_steps() {
        let a = DynMatrix::from_sq(&SqMatrix::new([[1, 3], [2, 1]]));
        let successors = enumerate_graph_move_successors(&a, 3);

        assert!(!successors.nodes.is_empty());
        for successor in successors.nodes {
            assert_eq!(successor.step.u.mul(&successor.step.v), a);
            assert_eq!(
                successor.step.v.mul(&successor.step.u),
                successor.orig_matrix
            );
            assert_eq!(successor.orig_matrix.canonical_perm(), successor.matrix);
        }
    }

    #[test]
    fn test_enumerate_same_future_insplits_2x2_to_3x3_nonempty() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let witnesses = enumerate_same_future_insplits_2x2_to_3x3(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.edge.mul(&witness.division), DynMatrix::from_sq(&a));
            assert!(has_duplicate_rows(&witness.outsplit));
        }
    }

    #[test]
    fn test_enumerate_same_past_outsplits_2x2_to_3x3_nonempty() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let witnesses = enumerate_same_past_outsplits_2x2_to_3x3(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.division.mul(&witness.edge), DynMatrix::from_sq(&a));
            assert!(has_duplicate_columns(&witness.outsplit));
        }
    }

    #[test]
    fn test_same_future_past_signature_gap_zero_for_matching_duplicate_profiles() {
        let a = DynMatrix::new(3, 3, vec![1, 1, 0, 1, 1, 0, 0, 1, 1]);
        let b = DynMatrix::new(3, 3, vec![1, 0, 1, 1, 0, 1, 0, 1, 1]);
        let a_signature = same_future_past_signature(&a).unwrap();
        let b_signature = same_future_past_signature(&b).unwrap();

        assert_eq!(
            same_future_past_signature_gap(&a_signature, &b_signature),
            SameFuturePastSignatureGap::default()
        );
    }

    #[test]
    fn test_partition_refined_gap_detects_coarse_duplicate_profile_collision() {
        let a = DynMatrix::new(3, 3, vec![0, 0, 1, 0, 1, 1, 1, 2, 0]);
        let b = DynMatrix::new(3, 3, vec![0, 0, 1, 0, 1, 2, 1, 1, 0]);

        assert_eq!(same_future_past_signature_gap_total(&a, &b), 0);
        assert!(partition_refined_same_future_past_gap_total(&a, &b) > 0);
    }

    #[test]
    fn test_graph_proposals_include_amalgamations_and_sort_by_gap() {
        let source = SqMatrix::new([[1, 3], [2, 1]]);
        let current = enumerate_same_future_insplits_2x2_to_3x3(&source)[0]
            .outsplit
            .clone();
        let target = DynMatrix::from_sq(&SqMatrix::new([[1, 6], [1, 1]]));

        let proposals = enumerate_graph_proposals(&current, &target, 4, Some(8));

        assert!(!proposals.nodes.is_empty());
        assert!(proposals
            .nodes
            .iter()
            .any(|proposal| proposal.families.contains(&"in_amalgamation")));
        assert!(proposals
            .nodes
            .windows(2)
            .all(|window| { window[0].target_signature_gap <= window[1].target_signature_gap }));
    }

    #[test]
    fn test_brix_ruiz_k3_has_no_common_one_step_outsplit_refinement() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        assert!(find_common_outsplit_refinement_2x2(&a, &b).is_none());
    }

    #[test]
    fn test_brix_ruiz_k4_has_no_common_one_step_outsplit_refinement() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        assert!(find_common_outsplit_refinement_2x2(&a, &b).is_none());
    }

    #[test]
    fn test_enumerate_3x3_outsplit_zigzag_neighbors_nonempty() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let first = enumerate_outsplits_2x2_to_3x3(&a);
        let neighbors = enumerate_3x3_outsplit_zigzag_neighbors(&first[0].outsplit, 8);
        assert!(!neighbors.is_empty());
        assert!(neighbors.iter().all(|m| m.rows == 3 && m.cols == 3));
    }

    #[test]
    fn test_out_amalgamation_roundtrips_same_past_outsplit() {
        // A same-past outsplit produces duplicate columns, so its output
        // should be out-amalgamable back to the original matrix.
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let outsplits = enumerate_same_past_outsplits_2x2_to_3x3(&a);
        assert!(!outsplits.is_empty());

        for outsplit_witness in &outsplits {
            let c = &outsplit_witness.outsplit;
            assert!(has_duplicate_columns(c));

            let amalgamations = enumerate_out_amalgamations(c);
            assert!(
                !amalgamations.is_empty(),
                "Same-past outsplit should be out-amalgamable"
            );

            // At least one amalgamation should recover the original matrix
            // (up to permutation).
            let a_dyn = DynMatrix::from_sq(&a);
            let a_canon = a_dyn.canonical_perm();
            let recovered = amalgamations
                .iter()
                .any(|w| w.outsplit.canonical_perm() == a_canon);
            assert!(recovered, "Out-amalgamation should recover original matrix");
        }
    }

    #[test]
    fn test_in_amalgamation_roundtrips_same_future_insplit() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let insplits = enumerate_same_future_insplits_2x2_to_3x3(&a);
        assert!(!insplits.is_empty());

        for insplit_witness in &insplits {
            let c = &insplit_witness.outsplit;
            assert!(has_duplicate_rows(c));

            let amalgamations = enumerate_in_amalgamations(c);
            assert!(
                !amalgamations.is_empty(),
                "Same-future insplit should be in-amalgamable"
            );

            let a_dyn = DynMatrix::from_sq(&a);
            let a_canon = a_dyn.canonical_perm();
            let recovered = amalgamations
                .iter()
                .any(|w| w.outsplit.canonical_perm() == a_canon);
            assert!(recovered, "In-amalgamation should recover original matrix");
        }
    }

    #[test]
    fn test_out_amalgamation_of_4x4_outsplit() {
        // Out-split a 3x3 to 4x4, then out-amalgamate back.
        let a3 = DynMatrix::new(3, 3, vec![1, 2, 2, 2, 1, 1, 1, 0, 0]);
        let outsplits = enumerate_same_past_outsplits(&a3);
        if outsplits.is_empty() {
            return; // skip if no same-past outsplits
        }

        for witness in &outsplits {
            let c4 = &witness.outsplit;
            assert_eq!(c4.rows, 4);
            let amalgamations = enumerate_out_amalgamations(c4);
            assert!(!amalgamations.is_empty());

            let a3_canon = a3.canonical_perm();
            let recovered = amalgamations
                .iter()
                .any(|w| w.outsplit.canonical_perm() == a3_canon);
            assert!(recovered, "4x4 out-amalgamation should recover the 3x3");
        }
    }

    #[test]
    fn test_amalgamation_sse_step_valid() {
        // Verify the SSE step: C = E·D and result = D·E.
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let outsplits = enumerate_same_past_outsplits_2x2_to_3x3(&a);
        for outsplit_w in &outsplits {
            let c = &outsplit_w.outsplit;
            for amal_w in &enumerate_out_amalgamations(c) {
                assert_eq!(amal_w.edge.mul(&amal_w.division), *c, "C should equal E·D");
                assert_eq!(
                    amal_w.division.mul(&amal_w.edge),
                    amal_w.outsplit,
                    "result should equal D·E"
                );
            }
        }
    }

    #[test]
    fn test_find_graph_move_witnesses_between_finds_direct_successor() {
        let a = DynMatrix::from_sq(&SqMatrix::new([[1, 3], [2, 1]]));
        let successor = enumerate_graph_move_successors(&a, 3)
            .nodes
            .into_iter()
            .next()
            .expect("expected at least one graph successor");

        let witnesses = find_graph_move_witnesses_between(&a, &successor.orig_matrix);
        assert!(!witnesses.is_empty());
        assert!(witnesses
            .into_iter()
            .any(|witness| witness.family == successor.family));
    }

    #[test]
    fn test_graph_proposals_best_gap_shortlist_is_tiny_on_waypoint_probe() {
        let current = DynMatrix::new(3, 3, vec![0, 0, 2, 1, 1, 1, 2, 2, 1]);
        let target = DynMatrix::new(3, 3, vec![0, 0, 2, 1, 1, 4, 1, 1, 1]);

        let proposals = enumerate_graph_proposals(&current, &target, 4, Some(8));

        assert_eq!(
            proposals.best_gap(),
            Some(SameFuturePastSignatureGap {
                dimension_gap: 0,
                row_class_gap: 2,
                col_class_gap: 6,
                entry_sum_gap: 0,
            })
        );
        assert_eq!(proposals.best_gap_shortlist_len(), 1);
        assert_eq!(proposals.best_gap_shortlist(4).len(), 1);
        assert_eq!(
            proposals.best_gap_shortlist(4)[0].matrix,
            DynMatrix::new(3, 3, vec![0, 0, 1, 1, 1, 1, 3, 3, 1])
        );
    }

    #[test]
    fn test_graph_proposals_refined_shortlist_reorders_coarse_prefix() {
        let current = DynMatrix::new(3, 3, vec![0, 0, 2, 1, 1, 1, 2, 2, 1]);
        let target = DynMatrix::new(3, 3, vec![0, 0, 2, 1, 1, 4, 1, 1, 1]);

        let proposals = enumerate_graph_proposals(&current, &target, 4, Some(8));
        let shortlist = proposals.refined_shortlist_from_coarse_prefix(4, 4);

        assert_eq!(shortlist.len(), 4);
        assert_eq!(shortlist[0].target_partition_refined_gap, 38);
        assert_eq!(
            shortlist[0].matrix,
            DynMatrix::new(3, 3, vec![0, 0, 1, 1, 1, 2, 2, 2, 1])
        );
        assert_eq!(shortlist[1].target_partition_refined_gap, 42);
        assert_eq!(
            shortlist[1].matrix,
            DynMatrix::new(3, 3, vec![0, 0, 1, 1, 1, 1, 3, 3, 1])
        );
    }
}
