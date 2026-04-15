use ahash::AHashMap as HashMap;

use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::{
    DynSsePath, EsseStep, GuideArtifact, GuideArtifactCompatibility, GuideArtifactEndpoints,
    GuideArtifactPayload, GuideArtifactProvenance, GuideArtifactQuality, GuideArtifactValidation,
    SsePath,
};

/// Validate a 2x2 witness path against its endpoints.
pub fn validate_sse_path_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    path: &SsePath<2>,
) -> Result<(), String> {
    validate_sse_path_dyn(
        &DynMatrix::from_sq(a),
        &DynMatrix::from_sq(b),
        &path.clone().into(),
    )
}

/// Validate a dynamic witness path against its endpoints.
pub fn validate_sse_path_dyn(
    a: &DynMatrix,
    b: &DynMatrix,
    path: &DynSsePath,
) -> Result<(), String> {
    if path.matrices.len() != path.steps.len() + 1 {
        return Err(format!(
            "path contains {} matrices but {} steps",
            path.matrices.len(),
            path.steps.len()
        ));
    }

    if path.steps.is_empty() {
        if path.matrices.len() != 1 {
            return Err(format!(
                "empty-step path should contain exactly one matrix, got {}",
                path.matrices.len()
            ));
        }
        if path.matrices[0] != *a || path.matrices[0] != *b {
            return Err("empty-step path does not match the endpoint matrices".to_string());
        }
        return Ok(());
    }

    if path.matrices.first() != Some(a) {
        return Err("path.matrices does not start at A".to_string());
    }
    if path.matrices.last() != Some(b) {
        return Err("path.matrices does not end at B".to_string());
    }

    for (idx, step) in path.steps.iter().enumerate() {
        let uv = step.u.mul(&step.v);
        let vu = step.v.mul(&step.u);
        if uv != path.matrices[idx] {
            return Err(format!("step {idx} does not start at path.matrices[{idx}]"));
        }
        if vu != path.matrices[idx + 1] {
            return Err(format!(
                "step {idx} does not end at path.matrices[{}]",
                idx + 1
            ));
        }
    }

    Ok(())
}

/// Build a reusable `full_path` guide artifact from a validated witness path.
pub fn build_full_path_guide_artifact(
    source: &DynMatrix,
    target: &DynMatrix,
    path: &DynSsePath,
) -> Result<GuideArtifact, String> {
    validate_sse_path_dyn(source, target, path)?;
    Ok(GuideArtifact {
        artifact_id: None,
        endpoints: GuideArtifactEndpoints {
            source: source.clone(),
            target: target.clone(),
        },
        payload: GuideArtifactPayload::FullPath { path: path.clone() },
        provenance: GuideArtifactProvenance::default(),
        validation: GuideArtifactValidation::WitnessValidated,
        compatibility: GuideArtifactCompatibility::default(),
        quality: GuideArtifactQuality {
            lag: Some(path.steps.len()),
            cost: Some(path.steps.len()),
            score: None,
        },
    })
}

pub(super) fn reverse_dyn_sse_path(path: &DynSsePath) -> DynSsePath {
    DynSsePath {
        matrices: path.matrices.iter().cloned().rev().collect(),
        steps: path
            .steps
            .iter()
            .rev()
            .map(|step| EsseStep {
                u: step.v.clone(),
                v: step.u.clone(),
            })
            .collect(),
    }
}

pub(super) fn reanchor_dyn_sse_path(
    path: &DynSsePath,
    source: &DynMatrix,
    target: &DynMatrix,
) -> Result<DynSsePath, String> {
    let mut path = path.clone();
    if path.matrices.is_empty() {
        return Err("guide path contains no matrices".to_string());
    }

    if path.matrices.first() != Some(source) {
        let first = path
            .matrices
            .first()
            .expect("non-empty path should have a first matrix")
            .clone();
        let step = permutation_step_between(source, &first).ok_or_else(|| {
            "guide start is not permutation-compatible with the request".to_string()
        })?;
        path.steps.insert(0, step);
        path.matrices.insert(0, source.clone());
    }

    if path.matrices.last() != Some(target) {
        let last = path
            .matrices
            .last()
            .expect("non-empty path should have a last matrix")
            .clone();
        let step = permutation_step_between(&last, target).ok_or_else(|| {
            "guide end is not permutation-compatible with the request".to_string()
        })?;
        path.steps.push(step);
        path.matrices.push(target.clone());
    }

    Ok(path)
}

/// Create a permutation similarity step: given matrices M and M' = PMP
/// where P is the swap permutation, return an EsseStep with U = MP, V = P
/// so that UV = M and VU = M'.
fn permutation_step(m: &DynMatrix) -> EsseStep {
    let n = m.rows;
    let mut p_data = vec![0u32; n * n];
    for i in 0..n {
        p_data[i * n + (n - 1 - i)] = 1;
    }
    let p = DynMatrix::new(n, n, p_data);
    let mp = m.mul(&p);
    EsseStep { u: mp, v: p }
}

pub(super) fn permutation_step_between(from: &DynMatrix, to: &DynMatrix) -> Option<EsseStep> {
    if from.rows != from.cols || to.rows != to.cols || from.rows != to.rows {
        return None;
    }
    let n = from.rows;
    let mut perm: Vec<usize> = (0..n).collect();
    let mut result = None;
    for_each_permutation(&mut perm, 0, &mut |perm| {
        if result.is_some() {
            return;
        }
        let (p, pinv) = permutation_matrices(perm);
        let candidate = pinv.mul(from).mul(&p);
        if candidate == *to {
            let u = from.mul(&p);
            result = Some(EsseStep { u, v: pinv });
        }
    });
    result
}

fn permutation_matrices(perm: &[usize]) -> (DynMatrix, DynMatrix) {
    let n = perm.len();
    let mut p_data = vec![0u32; n * n];
    let mut pinv_data = vec![0u32; n * n];
    for (row, &col) in perm.iter().enumerate() {
        p_data[row * n + col] = 1;
        pinv_data[col * n + row] = 1;
    }
    (
        DynMatrix::new(n, n, p_data),
        DynMatrix::new(n, n, pinv_data),
    )
}

fn for_each_permutation<F>(perm: &mut [usize], start: usize, visit: &mut F)
where
    F: FnMut(&[usize]),
{
    if start == perm.len() {
        visit(perm);
        return;
    }
    for idx in start..perm.len() {
        perm.swap(start, idx);
        for_each_permutation(perm, start + 1, visit);
        perm.swap(start, idx);
    }
}

/// Walk a parent chain from `node` back to the root, returning
/// (matrices, steps) in root-to-node order.
fn walk_parent_chain(
    node: &DynMatrix,
    parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    orig: &HashMap<DynMatrix, DynMatrix>,
) -> (Vec<DynMatrix>, Vec<EsseStep>) {
    let mut matrices = Vec::new();
    let mut steps = Vec::new();
    let mut current = node.clone();

    matrices.push(orig[&current].clone());

    while let Some(Some((prev, step))) = parent.get(&current) {
        steps.push(step.clone());
        matrices.push(orig[prev].clone());
        current = prev.clone();
    }

    matrices.reverse();
    steps.reverse();
    (matrices, steps)
}

/// Reconstruct a path from the forward and backward BFS trees that meet
/// at `meeting_canon`.
///
/// Forward chain: A -> ... -> M (steps recorded as current=UV, neighbor=VU).
/// Backward chain: B -> ... -> M (same convention).
/// We reverse the backward chain to get M -> ... -> B, flipping each step's
/// (U,V) to (V,U) since the direction of the elementary SSE is reversed.
pub(super) fn reconstruct_bidirectional_path(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    meeting_canon: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    fwd_orig: &HashMap<DynMatrix, DynMatrix>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_orig: &HashMap<DynMatrix, DynMatrix>,
) -> SsePath<2> {
    let (fwd_matrices, fwd_steps) = walk_parent_chain(meeting_canon, fwd_parent, fwd_orig);
    let (bwd_matrices, bwd_steps) = walk_parent_chain(meeting_canon, bwd_parent, bwd_orig);

    let fwd_meeting = fwd_matrices
        .last()
        .expect("forward chain should end at the meeting node")
        .clone();
    let bwd_meeting = bwd_matrices
        .last()
        .expect("backward chain should end at the meeting node")
        .clone();

    let mut all_steps = fwd_steps;

    if fwd_meeting != bwd_meeting {
        let step = permutation_step_between(&fwd_meeting, &bwd_meeting)
            .expect("meeting representatives should be permutation-similar");
        all_steps.push(step);
    }

    for step in bwd_steps.into_iter().rev() {
        all_steps.push(EsseStep {
            u: step.v,
            v: step.u,
        });
    }

    let mut all_dyn_matrices: Vec<DynMatrix> = fwd_matrices;
    if fwd_meeting != bwd_meeting {
        all_dyn_matrices.push(bwd_meeting);
    }
    for m in bwd_matrices.into_iter().rev().skip(1) {
        all_dyn_matrices.push(m);
    }

    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);

    if *all_dyn_matrices.first().unwrap() != a_dyn {
        all_steps.insert(0, permutation_step(&a_dyn));
        all_dyn_matrices.insert(0, a_dyn);
    }

    if *all_dyn_matrices.last().unwrap() != b_dyn {
        let last = all_dyn_matrices.last().unwrap().clone();
        all_steps.push(permutation_step(&last));
        all_dyn_matrices.push(b_dyn);
    }

    let sq_matrices: Vec<SqMatrix<2>> = all_dyn_matrices
        .iter()
        .filter_map(|dm| dm.to_sq::<2>())
        .collect();

    SsePath {
        matrices: sq_matrices,
        steps: all_steps,
    }
}

pub(super) fn reconstruct_bidirectional_dyn_path(
    a: &DynMatrix,
    b: &DynMatrix,
    meeting_canon: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    fwd_orig: &HashMap<DynMatrix, DynMatrix>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_orig: &HashMap<DynMatrix, DynMatrix>,
) -> DynSsePath {
    let (fwd_matrices, fwd_steps) = walk_parent_chain(meeting_canon, fwd_parent, fwd_orig);
    let (bwd_matrices, bwd_steps) = walk_parent_chain(meeting_canon, bwd_parent, bwd_orig);

    let fwd_meeting = fwd_matrices
        .last()
        .expect("forward chain should end at the meeting node")
        .clone();
    let bwd_meeting = bwd_matrices
        .last()
        .expect("backward chain should end at the meeting node")
        .clone();

    let mut all_steps = fwd_steps;
    if fwd_meeting != bwd_meeting {
        let step = permutation_step_between(&fwd_meeting, &bwd_meeting)
            .expect("meeting representatives should be permutation-similar");
        all_steps.push(step);
    }

    for step in bwd_steps.into_iter().rev() {
        all_steps.push(EsseStep {
            u: step.v,
            v: step.u,
        });
    }

    let mut all_dyn_matrices: Vec<DynMatrix> = fwd_matrices;
    if fwd_meeting != bwd_meeting {
        all_dyn_matrices.push(bwd_meeting);
    }
    for m in bwd_matrices.into_iter().rev().skip(1) {
        all_dyn_matrices.push(m);
    }

    if *all_dyn_matrices.first().unwrap() != *a {
        let first = all_dyn_matrices.first().unwrap().clone();
        let step =
            permutation_step_between(a, &first).expect("start should be permutation-similar");
        all_steps.insert(0, step);
        all_dyn_matrices.insert(0, a.clone());
    }

    if *all_dyn_matrices.last().unwrap() != *b {
        let last = all_dyn_matrices.last().unwrap().clone();
        let step = permutation_step_between(&last, b).expect("end should be permutation-similar");
        all_steps.push(step);
        all_dyn_matrices.push(b.clone());
    }

    DynSsePath {
        matrices: all_dyn_matrices,
        steps: all_steps,
    }
}
