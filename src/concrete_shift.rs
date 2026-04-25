use std::array;
use std::collections::HashMap;
use std::ops::ControlFlow;

use crate::matrix::SqMatrix;

/// Fixed-lag shift equivalence witness for a pair of 2x2 matrices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShiftEquivalenceWitness2x2 {
    pub lag: u32,
    pub r: SqMatrix<2>,
    pub s: SqMatrix<2>,
}

/// A fiberwise bijection, encoded as a permutation for each source/target fiber.
///
/// The four fibers are ordered as `(0,0), (0,1), (1,0), (1,1)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiberwiseBijection2x2 {
    pub mapping: [Vec<usize>; 4],
}

impl FiberwiseBijection2x2 {
    pub fn identity(lengths: [usize; 4]) -> Self {
        Self {
            mapping: lengths.map(|len| (0..len).collect()),
        }
    }
}

/// A concrete matrix shift witness in the sense of Definition 3.3 of
/// Bilich, Dor-On & Ruiz (2024).
///
/// The rectangular matrices `R` and `S` are stored in the underlying
/// shift-equivalence witness. The additional data here records the path
/// isomorphisms
///
/// - `varphi_R : E_A × E_R -> E_R × E_B`
/// - `varphi_S : E_B × E_S -> E_S × E_A`
/// - `psi_A : E_R × E_S -> E_A^m`
/// - `psi_B : E_S × E_R -> E_B^m`
///
/// For compatibility with the older local naming, the four maps still use the
/// `sigma_g`, `sigma_h`, `omega_e`, and `omega_f` field names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleShiftWitness2x2 {
    pub shift: ShiftEquivalenceWitness2x2,
    pub sigma_g: FiberwiseBijection2x2,
    pub sigma_h: FiberwiseBijection2x2,
    pub omega_e: FiberwiseBijection2x2,
    pub omega_f: FiberwiseBijection2x2,
}

/// Configuration for bounded aligned-module search on 2x2 matrices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlignedModuleSearchConfig2x2 {
    pub max_lag: u32,
    pub max_entry: u32,
    /// Maximum number of module witnesses to test before aborting the search.
    pub max_module_witnesses: usize,
}

impl Default for AlignedModuleSearchConfig2x2 {
    fn default() -> Self {
        Self {
            max_lag: 3,
            max_entry: 6,
            max_module_witnesses: 10_000,
        }
    }
}

/// Result of bounded aligned-module search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlignedModuleSearchResult2x2 {
    Equivalent(ModuleShiftWitness2x2),
    Exhausted,
    SearchLimitReached,
}

/// Public alias using the current matrix-level terminology from the papers.
pub type ConcreteShiftWitness2x2 = ModuleShiftWitness2x2;

/// Which concrete shift relation to check on a bounded witness space.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConcreteShiftRelation2x2 {
    Aligned,
    Balanced,
    Compatible,
}

impl ConcreteShiftRelation2x2 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aligned => "aligned",
            Self::Balanced => "balanced",
            Self::Compatible => "compatible",
        }
    }
}

/// Configuration for bounded concrete-shift search on 2x2 matrices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcreteShiftSearchConfig2x2 {
    pub relation: ConcreteShiftRelation2x2,
    pub max_lag: u32,
    pub max_entry: u32,
    /// Maximum number of concrete witnesses to test before aborting the search.
    pub max_witnesses: usize,
}

impl Default for ConcreteShiftSearchConfig2x2 {
    fn default() -> Self {
        Self {
            relation: ConcreteShiftRelation2x2::Aligned,
            max_lag: 3,
            max_entry: 6,
            max_witnesses: 10_000,
        }
    }
}

/// Result of bounded concrete-shift search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConcreteShiftSearchResult2x2 {
    Equivalent(ConcreteShiftWitness2x2),
    Exhausted,
    SearchLimitReached,
}

/// Configuration for a cheap bounded profile used as search-ranking side data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcreteShiftProfileConfig2x2 {
    pub relation: ConcreteShiftRelation2x2,
    pub max_lag: u32,
    pub max_entry: u32,
    pub max_witnesses: usize,
}

impl Default for ConcreteShiftProfileConfig2x2 {
    fn default() -> Self {
        Self {
            relation: ConcreteShiftRelation2x2::Aligned,
            max_lag: 1,
            max_entry: 1,
            max_witnesses: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConcreteShiftProfileStatus2x2 {
    Equivalent,
    ShiftWitnessOnly,
    Exhausted,
    SearchLimitReached,
}

/// Coarse low-lag concrete-shift profile for one 2x2 matrix pair.
///
/// This profile is intentionally heuristic side data. `ShiftWitnessOnly` means
/// bounded shift-equivalence witnesses existed, but the bounded concrete-shift
/// witness search did not find a concrete witness. `limit_reached` records
/// whether that search also hit its local witness limit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcreteShiftProfile2x2 {
    pub relation: ConcreteShiftRelation2x2,
    pub max_lag: u32,
    pub max_entry: u32,
    pub max_witnesses: usize,
    pub shift_witnesses: usize,
    pub concrete_witness_lag: Option<u32>,
    pub limit_reached: bool,
    pub status: ConcreteShiftProfileStatus2x2,
}

/// Search bounds used to obtain a bounded concrete-shift witness surface.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConcreteShiftProposalBounds2x2 {
    pub max_lag: u32,
    pub max_entry: u32,
    pub max_witnesses: usize,
}

/// Compact matrix support summary for report-only proposal surfaces.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct MatrixSupportSummary2x2 {
    pub signature: String,
    pub matrix: [[u32; 2]; 2],
    pub nonzero_positions: Vec<[usize; 2]>,
    pub row_sums: [u32; 2],
    pub col_sums: [u32; 2],
    pub entry_sum: u64,
    pub max_entry: u32,
}

/// Per-map fiber cardinalities from a concrete-shift witness.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConcreteShiftFiberCardinalities2x2 {
    pub sigma_g: [usize; 4],
    pub sigma_h: [usize; 4],
    pub omega_e: [usize; 4],
    pub omega_f: [usize; 4],
}

/// Compact bridge-edge summary used by proposal-only report output.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConcreteShiftEdgeSummary2x2 {
    pub source: usize,
    pub target: usize,
    pub label: usize,
}

/// Stable bounded path summary used for bridge samples.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConcreteShiftPathSummary2x2 {
    pub source: usize,
    pub target: usize,
    pub edge_count: usize,
    pub signature: String,
}

/// Small bridge-pair sample linking an `R/S` edge pair to a lag path.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConcreteShiftBridgePathSample2x2 {
    pub r_edge: ConcreteShiftEdgeSummary2x2,
    pub s_edge: ConcreteShiftEdgeSummary2x2,
    pub mapped_path: ConcreteShiftPathSummary2x2,
}

/// Fiber-level summary of bridge pair counts with bounded samples.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConcreteShiftBridgeFiberSummary2x2 {
    pub fiber: [usize; 2],
    pub bridge_pair_count: usize,
    pub path_count: usize,
    pub samples: Vec<ConcreteShiftBridgePathSample2x2>,
}

/// Report-only bounded proposal data for a positive concrete-shift witness.
///
/// This surface intentionally excludes any replayable SSE path payload.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConcreteShiftProposalData2x2 {
    pub artifact_kind: &'static str,
    pub replayable_as_full_path: bool,
    pub relation: &'static str,
    pub witness_label: &'static str,
    pub lag: u32,
    pub search_bounds: ConcreteShiftProposalBounds2x2,
    pub witness_signature: String,
    pub shift_signature: String,
    pub source: MatrixSupportSummary2x2,
    pub target: MatrixSupportSummary2x2,
    pub bridge_r: MatrixSupportSummary2x2,
    pub bridge_s: MatrixSupportSummary2x2,
    pub source_lag_power: MatrixSupportSummary2x2,
    pub target_lag_power: MatrixSupportSummary2x2,
    pub fiber_cardinalities: ConcreteShiftFiberCardinalities2x2,
    pub rs_to_a_path_fibers: Vec<ConcreteShiftBridgeFiberSummary2x2>,
    pub sr_to_b_path_fibers: Vec<ConcreteShiftBridgeFiberSummary2x2>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct EdgeRecord {
    source: usize,
    target: usize,
    label: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct PathRecord {
    edges: Vec<usize>,
    source: usize,
    target: usize,
}

#[derive(Clone, Debug)]
struct FiberData<T> {
    fibers: [Vec<T>; 4],
    positions: [HashMap<T, usize>; 4],
}

impl<T> FiberData<T>
where
    T: Copy + Eq + std::hash::Hash,
{
    fn from_items(items: impl IntoIterator<Item = (T, usize, usize)>) -> Self {
        let mut fibers: [Vec<T>; 4] = array::from_fn(|_| Vec::new());
        for (item, source, target) in items {
            fibers[fiber_index(source, target)].push(item);
        }

        let mut positions: [HashMap<T, usize>; 4] = array::from_fn(|_| HashMap::new());
        for fiber in 0..4 {
            for (idx, item) in fibers[fiber].iter().copied().enumerate() {
                positions[fiber].insert(item, idx);
            }
        }

        Self { fibers, positions }
    }

    fn lengths(&self) -> [usize; 4] {
        self.fibers.each_ref().map(|fiber| fiber.len())
    }
}

struct ModuleContext {
    lag: usize,
    e_edges: Vec<EdgeRecord>,
    f_edges: Vec<EdgeRecord>,
    g_edges: Vec<EdgeRecord>,
    h_edges: Vec<EdgeRecord>,
    e_paths: Vec<PathRecord>,
    f_paths: Vec<PathRecord>,
    e_path_lookup: HashMap<Vec<usize>, usize>,
    f_path_lookup: HashMap<Vec<usize>, usize>,
    sigma_g_domain: FiberData<(usize, usize)>,
    sigma_g_codomain: FiberData<(usize, usize)>,
    sigma_h_domain: FiberData<(usize, usize)>,
    sigma_h_codomain: FiberData<(usize, usize)>,
    omega_e_domain: FiberData<(usize, usize)>,
    omega_e_codomain: FiberData<usize>,
    omega_f_domain: FiberData<(usize, usize)>,
    omega_f_codomain: FiberData<usize>,
    egh_domain: FiberData<(usize, usize, usize)>,
    e_path_edge_codomain: FiberData<(usize, usize)>,
    fhg_domain: FiberData<(usize, usize, usize)>,
    f_path_edge_codomain: FiberData<(usize, usize)>,
}

/// Verify the classical shift equivalence relations for a proposed witness.
pub fn verify_shift_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ShiftEquivalenceWitness2x2,
) -> Result<(), String> {
    if witness.lag == 0 {
        return Err("shift equivalence lag must be positive".into());
    }

    let a_pow = a.pow(witness.lag);
    let b_pow = b.pow(witness.lag);
    let rs = witness.r.mul_u32(&witness.s);
    let sr = witness.s.mul_u32(&witness.r);
    let ar = a.mul_u32(&witness.r);
    let rb = witness.r.mul_u32(b);
    let bs = b.mul_u32(&witness.s);
    let sa = witness.s.mul_u32(a);

    if rs != a_pow {
        return Err(format!("A^lag != RS: {:?} vs {:?}", a_pow, rs));
    }
    if sr != b_pow {
        return Err(format!("B^lag != SR: {:?} vs {:?}", b_pow, sr));
    }
    if ar != rb {
        return Err(format!("AR != RB: {:?} vs {:?}", ar, rb));
    }
    if bs != sa {
        return Err(format!("BS != SA: {:?} vs {:?}", bs, sa));
    }

    Ok(())
}

/// Construct the canonical module-shift witness obtained by pairing each
/// source/target fiber in lexicographic order.
pub fn canonical_module_shift_witness_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    shift: ShiftEquivalenceWitness2x2,
) -> Result<ModuleShiftWitness2x2, String> {
    let ctx = ModuleContext::new(a, b, &shift)?;

    Ok(ModuleShiftWitness2x2 {
        shift,
        sigma_g: FiberwiseBijection2x2::identity(ctx.sigma_g_domain.lengths()),
        sigma_h: FiberwiseBijection2x2::identity(ctx.sigma_h_domain.lengths()),
        omega_e: FiberwiseBijection2x2::identity(ctx.omega_e_domain.lengths()),
        omega_f: FiberwiseBijection2x2::identity(ctx.omega_f_domain.lengths()),
    })
}

/// Verify that the stored path isomorphisms form a concrete shift witness.
pub fn verify_concrete_shift_witness_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ModuleShiftWitness2x2,
) -> Result<(), String> {
    let ctx = ModuleContext::new(a, b, &witness.shift)?;

    verify_fiberwise_bijection(
        "sigma_g",
        &witness.sigma_g,
        ctx.sigma_g_domain.lengths(),
        ctx.sigma_g_codomain.lengths(),
    )?;
    verify_fiberwise_bijection(
        "sigma_h",
        &witness.sigma_h,
        ctx.sigma_h_domain.lengths(),
        ctx.sigma_h_codomain.lengths(),
    )?;
    verify_fiberwise_bijection(
        "omega_e",
        &witness.omega_e,
        ctx.omega_e_domain.lengths(),
        ctx.omega_e_codomain.lengths(),
    )?;
    verify_fiberwise_bijection(
        "omega_f",
        &witness.omega_f,
        ctx.omega_f_domain.lengths(),
        ctx.omega_f_codomain.lengths(),
    )?;

    Ok(())
}

/// Backwards-compatible wrapper for the older local name.
pub fn verify_module_shift_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ModuleShiftWitness2x2,
) -> Result<(), String> {
    verify_concrete_shift_witness_2x2(a, b, witness)
}

/// Verify the aligned concrete-shift equations from Definition 3.3.
pub fn verify_aligned_concrete_shift_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ModuleShiftWitness2x2,
) -> Result<(), String> {
    let ctx = ModuleContext::new(a, b, &witness.shift)?;
    verify_concrete_shift_witness_2x2(a, b, witness)?;

    for fiber in 0..4 {
        for &(e_idx, g_idx, h_idx) in &ctx.egh_domain.fibers[fiber] {
            let top = aligned_e_top(&ctx, witness, e_idx, g_idx, h_idx)?;
            let bottom = aligned_e_bottom(&ctx, witness, e_idx, g_idx, h_idx)?;
            if top != bottom {
                return Err(format!(
                    "aligned relation (5.3) failed on triple (e={}, g={}, h={}): {:?} != {:?}",
                    e_idx, g_idx, h_idx, top, bottom
                ));
            }
        }
    }

    for fiber in 0..4 {
        for &(f_idx, h_idx, g_idx) in &ctx.fhg_domain.fibers[fiber] {
            let top = aligned_f_top(&ctx, witness, f_idx, h_idx, g_idx)?;
            let bottom = aligned_f_bottom(&ctx, witness, f_idx, h_idx, g_idx)?;
            if top != bottom {
                return Err(format!(
                    "aligned relation (5.4) failed on triple (f={}, h={}, g={}): {:?} != {:?}",
                    f_idx, h_idx, g_idx, top, bottom
                ));
            }
        }
    }

    Ok(())
}

/// Backwards-compatible wrapper for the older local name.
pub fn verify_aligned_module_shift_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ModuleShiftWitness2x2,
) -> Result<(), String> {
    verify_aligned_concrete_shift_2x2(a, b, witness)
}

/// Verify the balanced concrete-shift equations from Definition 3.3.
pub fn verify_balanced_concrete_shift_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ModuleShiftWitness2x2,
) -> Result<(), String> {
    let ctx = ModuleContext::new(a, b, &witness.shift)?;
    verify_concrete_shift_witness_2x2(a, b, witness)?;

    for (path_idx, path) in ctx.e_paths.iter().enumerate() {
        for (r_idx, r_edge) in ctx.g_edges.iter().enumerate() {
            if path.target != r_edge.source {
                continue;
            }
            for (s_idx, s_edge) in ctx.h_edges.iter().enumerate() {
                if r_edge.target != s_edge.source {
                    continue;
                }

                let lhs = (
                    invert_path_bijection(
                        &witness.omega_e,
                        &ctx.omega_e_domain,
                        &ctx.omega_e_codomain,
                        path_idx,
                        path.source,
                        path.target,
                        "omega_e",
                    )?,
                    apply_path_bijection(
                        &witness.omega_e,
                        &ctx.omega_e_domain,
                        &ctx.omega_e_codomain,
                        (r_idx, s_idx),
                        r_edge.source,
                        s_edge.target,
                        "omega_e",
                    )?,
                );
                let rhs = balanced_rhs_a(&ctx, witness, path_idx, r_idx, s_idx)?;

                if lhs != rhs {
                    return Err(format!(
                        "balanced A relation failed on (path={}, r={}, s={}): {:?} != {:?}",
                        path_idx, r_idx, s_idx, lhs, rhs
                    ));
                }
            }
        }
    }

    for (path_idx, path) in ctx.f_paths.iter().enumerate() {
        for (s_idx, s_edge) in ctx.h_edges.iter().enumerate() {
            if path.target != s_edge.source {
                continue;
            }
            for (r_idx, r_edge) in ctx.g_edges.iter().enumerate() {
                if s_edge.target != r_edge.source {
                    continue;
                }

                let lhs = (
                    invert_path_bijection(
                        &witness.omega_f,
                        &ctx.omega_f_domain,
                        &ctx.omega_f_codomain,
                        path_idx,
                        path.source,
                        path.target,
                        "omega_f",
                    )?,
                    apply_path_bijection(
                        &witness.omega_f,
                        &ctx.omega_f_domain,
                        &ctx.omega_f_codomain,
                        (s_idx, r_idx),
                        s_edge.source,
                        r_edge.target,
                        "omega_f",
                    )?,
                );
                let rhs = balanced_rhs_b(&ctx, witness, path_idx, s_idx, r_idx)?;

                if lhs != rhs {
                    return Err(format!(
                        "balanced B relation failed on (path={}, s={}, r={}): {:?} != {:?}",
                        path_idx, s_idx, r_idx, lhs, rhs
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Verify the compatible concrete-shift equations from Definition 3.3.
pub fn verify_compatible_concrete_shift_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ModuleShiftWitness2x2,
) -> Result<(), String> {
    let ctx = ModuleContext::new(a, b, &witness.shift)?;
    verify_concrete_shift_witness_2x2(a, b, witness)?;

    for (path_idx, path) in ctx.e_paths.iter().enumerate() {
        for (r_idx, r_edge) in ctx.g_edges.iter().enumerate() {
            if path.target != r_edge.source {
                continue;
            }

            let lhs = iterate_phi_r(&ctx, witness, path_idx, r_idx)?;
            let rhs = compatible_rhs_r(&ctx, witness, path_idx, r_idx)?;
            if lhs != rhs {
                return Err(format!(
                    "compatible R relation failed on (path={}, r={}): {:?} != {:?}",
                    path_idx, r_idx, lhs, rhs
                ));
            }
        }
    }

    for (path_idx, path) in ctx.f_paths.iter().enumerate() {
        for (s_idx, s_edge) in ctx.h_edges.iter().enumerate() {
            if path.target != s_edge.source {
                continue;
            }

            let lhs = iterate_phi_s(&ctx, witness, path_idx, s_idx)?;
            let rhs = compatible_rhs_s(&ctx, witness, path_idx, s_idx)?;
            if lhs != rhs {
                return Err(format!(
                    "compatible S relation failed on (path={}, s={}): {:?} != {:?}",
                    path_idx, s_idx, lhs, rhs
                ));
            }
        }
    }

    Ok(())
}

/// Enumerate all bounded fixed-lag 2x2 shift-equivalence witnesses.
pub fn enumerate_shift_equivalence_with_lag_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    lag: u32,
    max_entry: u32,
) -> Vec<ShiftEquivalenceWitness2x2> {
    if lag == 0 {
        return Vec::new();
    }

    let a_pow = a.pow(lag);
    let b_pow = b.pow(lag);
    let r_candidates = enumerate_intertwiners_2x2(a, b, max_entry);
    let mut witnesses = Vec::new();

    for r in r_candidates {
        let s_candidates = solve_left_product_2x2(&r, &a_pow, max_entry);
        for s in s_candidates {
            let witness = ShiftEquivalenceWitness2x2 {
                lag,
                r: r.clone(),
                s,
            };

            if witness.s.mul_u32(&witness.r) != b_pow {
                continue;
            }

            if verify_shift_equivalence_2x2(a, b, &witness).is_ok() {
                witnesses.push(witness);
            }
        }
    }

    witnesses
}

/// Search for an aligned module shift-equivalence witness with a fixed lag.
///
/// This is a bounded brute-force search over the fiberwise bijections from the
/// module-aligned definition. It does not certify matrix-level aligned shift
/// equivalence, because that relation is still defined only in forthcoming work.
pub fn search_aligned_module_shift_equivalence_with_lag_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    lag: u32,
    max_entry: u32,
    max_module_witnesses: usize,
) -> AlignedModuleSearchResult2x2 {
    match search_concrete_shift_equivalence_with_lag_2x2(
        a,
        b,
        lag,
        max_entry,
        max_module_witnesses,
        ConcreteShiftRelation2x2::Aligned,
    ) {
        ConcreteShiftSearchResult2x2::Equivalent(witness) => {
            AlignedModuleSearchResult2x2::Equivalent(witness)
        }
        ConcreteShiftSearchResult2x2::Exhausted => AlignedModuleSearchResult2x2::Exhausted,
        ConcreteShiftSearchResult2x2::SearchLimitReached => {
            AlignedModuleSearchResult2x2::SearchLimitReached
        }
    }
}

/// Search for an aligned module shift-equivalence witness up to a lag bound.
pub fn search_aligned_module_shift_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &AlignedModuleSearchConfig2x2,
) -> AlignedModuleSearchResult2x2 {
    let mut any_limit = false;
    for lag in 1..=config.max_lag {
        match search_aligned_module_shift_equivalence_with_lag_2x2(
            a,
            b,
            lag,
            config.max_entry,
            config.max_module_witnesses,
        ) {
            AlignedModuleSearchResult2x2::Equivalent(witness) => {
                return AlignedModuleSearchResult2x2::Equivalent(witness);
            }
            AlignedModuleSearchResult2x2::Exhausted => {}
            AlignedModuleSearchResult2x2::SearchLimitReached => any_limit = true,
        }
    }

    if any_limit {
        AlignedModuleSearchResult2x2::SearchLimitReached
    } else {
        AlignedModuleSearchResult2x2::Exhausted
    }
}

/// Search for a concrete matrix shift witness with a fixed lag and relation.
pub fn search_concrete_shift_equivalence_with_lag_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    lag: u32,
    max_entry: u32,
    max_witnesses: usize,
    relation: ConcreteShiftRelation2x2,
) -> ConcreteShiftSearchResult2x2 {
    let shift_witnesses = enumerate_shift_equivalence_with_lag_2x2(a, b, lag, max_entry);
    search_concrete_shift_equivalence_for_witnesses_2x2(
        a,
        b,
        shift_witnesses,
        max_witnesses,
        relation,
    )
}

fn search_concrete_shift_equivalence_for_witnesses_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    shift_witnesses: Vec<ShiftEquivalenceWitness2x2>,
    max_witnesses: usize,
    relation: ConcreteShiftRelation2x2,
) -> ConcreteShiftSearchResult2x2 {
    let mut checked = 0usize;

    for shift in shift_witnesses {
        match search_concrete_witnesses_for_shift(
            a,
            b,
            shift,
            relation,
            max_witnesses,
            &mut checked,
        ) {
            ModuleWitnessSearchOutcome::Found(witness) => {
                return ConcreteShiftSearchResult2x2::Equivalent(witness);
            }
            ModuleWitnessSearchOutcome::Exhausted => {}
            ModuleWitnessSearchOutcome::LimitReached => {
                return ConcreteShiftSearchResult2x2::SearchLimitReached;
            }
        }
    }

    ConcreteShiftSearchResult2x2::Exhausted
}

/// Search for a concrete matrix shift witness up to a lag bound.
pub fn search_concrete_shift_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &ConcreteShiftSearchConfig2x2,
) -> ConcreteShiftSearchResult2x2 {
    let mut any_limit = false;
    for lag in 1..=config.max_lag {
        match search_concrete_shift_equivalence_with_lag_2x2(
            a,
            b,
            lag,
            config.max_entry,
            config.max_witnesses,
            config.relation,
        ) {
            ConcreteShiftSearchResult2x2::Equivalent(witness) => {
                return ConcreteShiftSearchResult2x2::Equivalent(witness);
            }
            ConcreteShiftSearchResult2x2::Exhausted => {}
            ConcreteShiftSearchResult2x2::SearchLimitReached => any_limit = true,
        }
    }

    if any_limit {
        ConcreteShiftSearchResult2x2::SearchLimitReached
    } else {
        ConcreteShiftSearchResult2x2::Exhausted
    }
}

pub fn concrete_shift_profile_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &ConcreteShiftProfileConfig2x2,
) -> ConcreteShiftProfile2x2 {
    let mut shift_witnesses = 0usize;
    let mut any_limit = false;
    for lag in 1..=config.max_lag {
        let lag_shift_witnesses =
            enumerate_shift_equivalence_with_lag_2x2(a, b, lag, config.max_entry);
        shift_witnesses += lag_shift_witnesses.len();
        match search_concrete_shift_equivalence_for_witnesses_2x2(
            a,
            b,
            lag_shift_witnesses,
            config.max_witnesses,
            config.relation,
        ) {
            ConcreteShiftSearchResult2x2::Equivalent(_) => {
                return ConcreteShiftProfile2x2 {
                    relation: config.relation,
                    max_lag: config.max_lag,
                    max_entry: config.max_entry,
                    max_witnesses: config.max_witnesses,
                    shift_witnesses,
                    concrete_witness_lag: Some(lag),
                    limit_reached: any_limit,
                    status: ConcreteShiftProfileStatus2x2::Equivalent,
                };
            }
            ConcreteShiftSearchResult2x2::Exhausted => {}
            ConcreteShiftSearchResult2x2::SearchLimitReached => any_limit = true,
        }
    }

    let status = if shift_witnesses > 0 {
        ConcreteShiftProfileStatus2x2::ShiftWitnessOnly
    } else if any_limit {
        ConcreteShiftProfileStatus2x2::SearchLimitReached
    } else {
        ConcreteShiftProfileStatus2x2::Exhausted
    };

    ConcreteShiftProfile2x2 {
        relation: config.relation,
        max_lag: config.max_lag,
        max_entry: config.max_entry,
        max_witnesses: config.max_witnesses,
        shift_witnesses,
        concrete_witness_lag: None,
        limit_reached: any_limit,
        status,
    }
}

/// Build bounded report-only proposal data from a concrete-shift witness.
pub fn concrete_shift_proposal_data_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    relation: ConcreteShiftRelation2x2,
    witness: &ConcreteShiftWitness2x2,
    bounds: ConcreteShiftProposalBounds2x2,
    sample_limit_per_fiber: usize,
) -> Result<ConcreteShiftProposalData2x2, String> {
    verify_concrete_shift_relation_2x2(a, b, witness, relation)?;
    if witness.shift.lag > bounds.max_lag {
        return Err(format!(
            "witness lag {} exceeds reported max_lag {}",
            witness.shift.lag, bounds.max_lag
        ));
    }
    let witness_max_entry = witness.shift.r.max_entry().max(witness.shift.s.max_entry());
    if witness_max_entry > bounds.max_entry {
        return Err(format!(
            "witness max entry {} exceeds reported max_entry {}",
            witness_max_entry, bounds.max_entry
        ));
    }
    let ctx = ModuleContext::new(a, b, &witness.shift)?;
    let lag = witness.shift.lag;
    let shift_signature = format!(
        "lag={lag}|R={}|S={}",
        sq_matrix_signature(&witness.shift.r),
        sq_matrix_signature(&witness.shift.s)
    );
    let witness_signature = stable_signature_hash(&format!(
        "relation={}|{}|sigma_g={}|sigma_h={}|omega_e={}|omega_f={}",
        relation.as_str(),
        shift_signature,
        bijection_signature(&witness.sigma_g),
        bijection_signature(&witness.sigma_h),
        bijection_signature(&witness.omega_e),
        bijection_signature(&witness.omega_f),
    ));

    Ok(ConcreteShiftProposalData2x2 {
        artifact_kind: "concrete_shift_proposal_data",
        replayable_as_full_path: false,
        relation: relation.as_str(),
        witness_label: relation_label(relation),
        lag,
        search_bounds: bounds,
        witness_signature,
        shift_signature,
        source: matrix_support_summary(a),
        target: matrix_support_summary(b),
        bridge_r: matrix_support_summary(&witness.shift.r),
        bridge_s: matrix_support_summary(&witness.shift.s),
        source_lag_power: matrix_support_summary(&a.pow(lag)),
        target_lag_power: matrix_support_summary(&b.pow(lag)),
        fiber_cardinalities: ConcreteShiftFiberCardinalities2x2 {
            sigma_g: ctx.sigma_g_domain.lengths(),
            sigma_h: ctx.sigma_h_domain.lengths(),
            omega_e: ctx.omega_e_domain.lengths(),
            omega_f: ctx.omega_f_domain.lengths(),
        },
        rs_to_a_path_fibers: bridge_fiber_summaries_from_rs(&ctx, witness, sample_limit_per_fiber)?,
        sr_to_b_path_fibers: bridge_fiber_summaries_from_sr(&ctx, witness, sample_limit_per_fiber)?,
    })
}

/// Search for a bounded 2x2 shift equivalence witness.
pub fn find_shift_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    max_lag: u32,
    max_entry: u32,
) -> Option<ShiftEquivalenceWitness2x2> {
    for lag in 1..=max_lag {
        if let Some(witness) = find_shift_equivalence_with_lag_2x2(a, b, lag, max_entry) {
            return Some(witness);
        }
    }
    None
}

/// Search for a bounded 2x2 shift equivalence witness with a fixed lag.
pub fn find_shift_equivalence_with_lag_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    lag: u32,
    max_entry: u32,
) -> Option<ShiftEquivalenceWitness2x2> {
    enumerate_shift_equivalence_with_lag_2x2(a, b, lag, max_entry)
        .into_iter()
        .next()
}

impl ModuleContext {
    fn new(
        a: &SqMatrix<2>,
        b: &SqMatrix<2>,
        shift: &ShiftEquivalenceWitness2x2,
    ) -> Result<Self, String> {
        verify_shift_equivalence_2x2(a, b, shift)?;

        let lag = shift.lag as usize;
        let e_edges = enumerate_edges_from_matrix_2x2(a);
        let f_edges = enumerate_edges_from_matrix_2x2(b);
        let g_edges = enumerate_edges_from_matrix_2x2(&shift.r);
        let h_edges = enumerate_edges_from_matrix_2x2(&shift.s);

        let e_paths = enumerate_paths_from_matrix_2x2(a, lag);
        let f_paths = enumerate_paths_from_matrix_2x2(b, lag);
        let e_path_lookup = build_path_lookup(&e_paths);
        let f_path_lookup = build_path_lookup(&f_paths);

        let sigma_g_domain = FiberData::from_items(
            enumerate_edge_edge_pairs(&e_edges, &g_edges)
                .into_iter()
                .map(|(e_idx, g_idx)| {
                    ((e_idx, g_idx), e_edges[e_idx].source, g_edges[g_idx].target)
                }),
        );
        let sigma_g_codomain = FiberData::from_items(
            enumerate_edge_edge_pairs(&g_edges, &f_edges)
                .into_iter()
                .map(|(g_idx, f_idx)| {
                    ((g_idx, f_idx), g_edges[g_idx].source, f_edges[f_idx].target)
                }),
        );

        let sigma_h_domain = FiberData::from_items(
            enumerate_edge_edge_pairs(&f_edges, &h_edges)
                .into_iter()
                .map(|(f_idx, h_idx)| {
                    ((f_idx, h_idx), f_edges[f_idx].source, h_edges[h_idx].target)
                }),
        );
        let sigma_h_codomain = FiberData::from_items(
            enumerate_edge_edge_pairs(&h_edges, &e_edges)
                .into_iter()
                .map(|(h_idx, e_idx)| {
                    ((h_idx, e_idx), h_edges[h_idx].source, e_edges[e_idx].target)
                }),
        );

        let omega_e_domain = FiberData::from_items(
            enumerate_edge_edge_pairs(&g_edges, &h_edges)
                .into_iter()
                .map(|(g_idx, h_idx)| {
                    ((g_idx, h_idx), g_edges[g_idx].source, h_edges[h_idx].target)
                }),
        );
        let omega_e_codomain = FiberData::from_items(
            e_paths
                .iter()
                .enumerate()
                .map(|(idx, path)| (idx, path.source, path.target)),
        );

        let omega_f_domain = FiberData::from_items(
            enumerate_edge_edge_pairs(&h_edges, &g_edges)
                .into_iter()
                .map(|(h_idx, g_idx)| {
                    ((h_idx, g_idx), h_edges[h_idx].source, g_edges[g_idx].target)
                }),
        );
        let omega_f_codomain = FiberData::from_items(
            f_paths
                .iter()
                .enumerate()
                .map(|(idx, path)| (idx, path.source, path.target)),
        );

        let egh_domain = FiberData::from_items(
            enumerate_edge_edge_edge_triples(&e_edges, &g_edges, &h_edges)
                .into_iter()
                .map(|(e_idx, g_idx, h_idx)| {
                    (
                        (e_idx, g_idx, h_idx),
                        e_edges[e_idx].source,
                        h_edges[h_idx].target,
                    )
                }),
        );
        let e_path_edge_codomain = FiberData::from_items(
            enumerate_path_edge_pairs(&e_paths, &e_edges)
                .into_iter()
                .map(|(path_idx, edge_idx)| {
                    (
                        (path_idx, edge_idx),
                        e_paths[path_idx].source,
                        e_edges[edge_idx].target,
                    )
                }),
        );

        let fhg_domain = FiberData::from_items(
            enumerate_edge_edge_edge_triples(&f_edges, &h_edges, &g_edges)
                .into_iter()
                .map(|(f_idx, h_idx, g_idx)| {
                    (
                        (f_idx, h_idx, g_idx),
                        f_edges[f_idx].source,
                        g_edges[g_idx].target,
                    )
                }),
        );
        let f_path_edge_codomain = FiberData::from_items(
            enumerate_path_edge_pairs(&f_paths, &f_edges)
                .into_iter()
                .map(|(path_idx, edge_idx)| {
                    (
                        (path_idx, edge_idx),
                        f_paths[path_idx].source,
                        f_edges[edge_idx].target,
                    )
                }),
        );

        Ok(Self {
            lag,
            e_edges,
            f_edges,
            g_edges,
            h_edges,
            e_paths,
            f_paths,
            e_path_lookup,
            f_path_lookup,
            sigma_g_domain,
            sigma_g_codomain,
            sigma_h_domain,
            sigma_h_codomain,
            omega_e_domain,
            omega_e_codomain,
            omega_f_domain,
            omega_f_codomain,
            egh_domain,
            e_path_edge_codomain,
            fhg_domain,
            f_path_edge_codomain,
        })
    }
}

fn verify_fiberwise_bijection(
    name: &str,
    bijection: &FiberwiseBijection2x2,
    domain_lengths: [usize; 4],
    codomain_lengths: [usize; 4],
) -> Result<(), String> {
    for fiber in 0..4 {
        if domain_lengths[fiber] != codomain_lengths[fiber] {
            return Err(format!(
                "{} fiber {} has incompatible sizes: {} vs {}",
                name, fiber, domain_lengths[fiber], codomain_lengths[fiber]
            ));
        }

        let mapping = &bijection.mapping[fiber];
        if mapping.len() != domain_lengths[fiber] {
            return Err(format!(
                "{} fiber {} has wrong length: {} vs {}",
                name,
                fiber,
                mapping.len(),
                domain_lengths[fiber]
            ));
        }

        let mut seen = vec![false; codomain_lengths[fiber]];
        for &target in mapping {
            if target >= codomain_lengths[fiber] {
                return Err(format!(
                    "{} fiber {} maps outside codomain: {} >= {}",
                    name, fiber, target, codomain_lengths[fiber]
                ));
            }
            if seen[target] {
                return Err(format!(
                    "{} fiber {} is not a permutation: repeated target {}",
                    name, fiber, target
                ));
            }
            seen[target] = true;
        }
    }

    Ok(())
}

fn aligned_e_top(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    e_idx: usize,
    g_idx: usize,
    h_idx: usize,
) -> Result<(usize, usize), String> {
    let (g_prime, f_idx) = apply_pair_bijection(
        &witness.sigma_g,
        &ctx.sigma_g_domain,
        &ctx.sigma_g_codomain,
        (e_idx, g_idx),
        ctx.e_edges[e_idx].source,
        ctx.h_edges[h_idx].source,
        "sigma_g",
    )?;
    let (h_prime, e_prime) = apply_pair_bijection(
        &witness.sigma_h,
        &ctx.sigma_h_domain,
        &ctx.sigma_h_codomain,
        (f_idx, h_idx),
        ctx.f_edges[f_idx].source,
        ctx.h_edges[h_idx].target,
        "sigma_h",
    )?;
    let path_idx = apply_path_bijection(
        &witness.omega_e,
        &ctx.omega_e_domain,
        &ctx.omega_e_codomain,
        (g_prime, h_prime),
        ctx.g_edges[g_prime].source,
        ctx.h_edges[h_prime].target,
        "omega_e",
    )?;

    let result = (path_idx, e_prime);
    ensure_pair_in_codomain(
        &ctx.e_path_edge_codomain,
        result,
        ctx.e_paths[path_idx].source,
        ctx.e_edges[e_prime].target,
        "aligned_e_top",
    )?;
    Ok(result)
}

fn aligned_e_bottom(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    e_idx: usize,
    g_idx: usize,
    h_idx: usize,
) -> Result<(usize, usize), String> {
    let path_idx = apply_path_bijection(
        &witness.omega_e,
        &ctx.omega_e_domain,
        &ctx.omega_e_codomain,
        (g_idx, h_idx),
        ctx.g_edges[g_idx].source,
        ctx.h_edges[h_idx].target,
        "omega_e",
    )?;
    split_prepend_edge(
        e_idx,
        &ctx.e_paths[path_idx],
        &ctx.e_path_lookup,
        ctx.lag,
        "E",
    )
}

fn aligned_f_top(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    f_idx: usize,
    h_idx: usize,
    g_idx: usize,
) -> Result<(usize, usize), String> {
    let (h_prime, e_idx) = apply_pair_bijection(
        &witness.sigma_h,
        &ctx.sigma_h_domain,
        &ctx.sigma_h_codomain,
        (f_idx, h_idx),
        ctx.f_edges[f_idx].source,
        ctx.g_edges[g_idx].source,
        "sigma_h",
    )?;
    let (g_prime, f_prime) = apply_pair_bijection(
        &witness.sigma_g,
        &ctx.sigma_g_domain,
        &ctx.sigma_g_codomain,
        (e_idx, g_idx),
        ctx.e_edges[e_idx].source,
        ctx.g_edges[g_idx].target,
        "sigma_g",
    )?;
    let path_idx = apply_path_bijection(
        &witness.omega_f,
        &ctx.omega_f_domain,
        &ctx.omega_f_codomain,
        (h_prime, g_prime),
        ctx.h_edges[h_prime].source,
        ctx.g_edges[g_prime].target,
        "omega_f",
    )?;

    let result = (path_idx, f_prime);
    ensure_pair_in_codomain(
        &ctx.f_path_edge_codomain,
        result,
        ctx.f_paths[path_idx].source,
        ctx.f_edges[f_prime].target,
        "aligned_f_top",
    )?;
    Ok(result)
}

fn aligned_f_bottom(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    f_idx: usize,
    h_idx: usize,
    g_idx: usize,
) -> Result<(usize, usize), String> {
    let path_idx = apply_path_bijection(
        &witness.omega_f,
        &ctx.omega_f_domain,
        &ctx.omega_f_codomain,
        (h_idx, g_idx),
        ctx.h_edges[h_idx].source,
        ctx.g_edges[g_idx].target,
        "omega_f",
    )?;
    split_prepend_edge(
        f_idx,
        &ctx.f_paths[path_idx],
        &ctx.f_path_lookup,
        ctx.lag,
        "F",
    )
}

fn apply_pair_bijection(
    bijection: &FiberwiseBijection2x2,
    domain: &FiberData<(usize, usize)>,
    codomain: &FiberData<(usize, usize)>,
    item: (usize, usize),
    source: usize,
    target: usize,
    name: &str,
) -> Result<(usize, usize), String> {
    let fiber = fiber_index(source, target);
    let local = domain.positions[fiber].get(&item).copied().ok_or_else(|| {
        format!(
            "{} domain item {:?} missing from fiber {}",
            name, item, fiber
        )
    })?;
    let mapped_local = bijection.mapping[fiber][local];
    Ok(codomain.fibers[fiber][mapped_local])
}

fn apply_path_bijection(
    bijection: &FiberwiseBijection2x2,
    domain: &FiberData<(usize, usize)>,
    codomain: &FiberData<usize>,
    item: (usize, usize),
    source: usize,
    target: usize,
    name: &str,
) -> Result<usize, String> {
    let fiber = fiber_index(source, target);
    let local = domain.positions[fiber].get(&item).copied().ok_or_else(|| {
        format!(
            "{} domain item {:?} missing from fiber {}",
            name, item, fiber
        )
    })?;
    let mapped_local = bijection.mapping[fiber][local];
    Ok(codomain.fibers[fiber][mapped_local])
}

fn invert_path_bijection(
    bijection: &FiberwiseBijection2x2,
    domain: &FiberData<(usize, usize)>,
    codomain: &FiberData<usize>,
    item: usize,
    source: usize,
    target: usize,
    name: &str,
) -> Result<(usize, usize), String> {
    let fiber = fiber_index(source, target);
    let codomain_local = codomain.positions[fiber]
        .get(&item)
        .copied()
        .ok_or_else(|| {
            format!(
                "{} codomain item {:?} missing from fiber {}",
                name, item, fiber
            )
        })?;
    let domain_local = bijection.mapping[fiber]
        .iter()
        .position(|&mapped_local| mapped_local == codomain_local)
        .ok_or_else(|| {
            format!(
                "{} inverse image for item {:?} missing from fiber {}",
                name, item, fiber
            )
        })?;
    Ok(domain.fibers[fiber][domain_local])
}

fn ensure_pair_in_codomain(
    codomain: &FiberData<(usize, usize)>,
    item: (usize, usize),
    source: usize,
    target: usize,
    name: &str,
) -> Result<(), String> {
    let fiber = fiber_index(source, target);
    if codomain.positions[fiber].contains_key(&item) {
        Ok(())
    } else {
        Err(format!(
            "{} produced incompatible pair {:?} in fiber {}",
            name, item, fiber
        ))
    }
}

fn split_prepend_edge(
    first_edge: usize,
    path: &PathRecord,
    path_lookup: &HashMap<Vec<usize>, usize>,
    lag: usize,
    name: &str,
) -> Result<(usize, usize), String> {
    let mut concatenated = Vec::with_capacity(lag + 1);
    concatenated.push(first_edge);
    concatenated.extend_from_slice(&path.edges);

    if concatenated.len() != lag + 1 {
        return Err(format!(
            "{} concatenated path has wrong length: {} vs {}",
            name,
            concatenated.len(),
            lag + 1
        ));
    }

    let last_edge = *concatenated
        .last()
        .ok_or_else(|| format!("{} concatenated path unexpectedly empty", name))?;
    let prefix = concatenated[..lag].to_vec();
    let prefix_idx = path_lookup
        .get(&prefix)
        .copied()
        .ok_or_else(|| format!("{} prefix path {:?} not found", name, prefix))?;
    Ok((prefix_idx, last_edge))
}

fn enumerate_edges_from_matrix_2x2(matrix: &SqMatrix<2>) -> Vec<EdgeRecord> {
    let mut edges = Vec::new();
    for source in 0..2 {
        for target in 0..2 {
            for label in 0..matrix.data[source][target] as usize {
                edges.push(EdgeRecord {
                    source,
                    target,
                    label,
                });
            }
        }
    }
    edges
}

fn enumerate_paths_from_matrix_2x2(matrix: &SqMatrix<2>, length: usize) -> Vec<PathRecord> {
    let edges = enumerate_edges_from_matrix_2x2(matrix);
    let outgoing = build_outgoing_index(&edges);
    let mut paths = Vec::new();

    for source in 0..2 {
        let mut current = Vec::with_capacity(length);
        enumerate_paths_from_source(source, length, &outgoing, &edges, &mut current, &mut paths);
    }

    paths
}

fn enumerate_paths_from_source(
    source: usize,
    remaining: usize,
    outgoing: &[Vec<usize>; 2],
    edges: &[EdgeRecord],
    current: &mut Vec<usize>,
    paths: &mut Vec<PathRecord>,
) {
    if remaining == 0 {
        let target = current
            .last()
            .map(|&edge_idx| edges[edge_idx].target)
            .unwrap_or(source);
        paths.push(PathRecord {
            edges: current.clone(),
            source,
            target,
        });
        return;
    }

    let current_vertex = current
        .last()
        .map(|&edge_idx| edges[edge_idx].target)
        .unwrap_or(source);
    for &edge_idx in &outgoing[current_vertex] {
        current.push(edge_idx);
        enumerate_paths_from_source(source, remaining - 1, outgoing, edges, current, paths);
        current.pop();
    }
}

fn build_outgoing_index(edges: &[EdgeRecord]) -> [Vec<usize>; 2] {
    let mut outgoing: [Vec<usize>; 2] = array::from_fn(|_| Vec::new());
    for (idx, edge) in edges.iter().enumerate() {
        outgoing[edge.source].push(idx);
    }
    outgoing
}

fn build_path_lookup(paths: &[PathRecord]) -> HashMap<Vec<usize>, usize> {
    let mut lookup = HashMap::new();
    for (idx, path) in paths.iter().enumerate() {
        lookup.insert(path.edges.clone(), idx);
    }
    lookup
}

fn enumerate_edge_edge_pairs(left: &[EdgeRecord], right: &[EdgeRecord]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for (left_idx, left_edge) in left.iter().enumerate() {
        for (right_idx, right_edge) in right.iter().enumerate() {
            if left_edge.target == right_edge.source {
                pairs.push((left_idx, right_idx));
            }
        }
    }
    pairs
}

fn enumerate_edge_edge_edge_triples(
    first: &[EdgeRecord],
    second: &[EdgeRecord],
    third: &[EdgeRecord],
) -> Vec<(usize, usize, usize)> {
    let mut triples = Vec::new();
    for (first_idx, first_edge) in first.iter().enumerate() {
        for (second_idx, second_edge) in second.iter().enumerate() {
            if first_edge.target != second_edge.source {
                continue;
            }
            for (third_idx, third_edge) in third.iter().enumerate() {
                if second_edge.target == third_edge.source {
                    triples.push((first_idx, second_idx, third_idx));
                }
            }
        }
    }
    triples
}

fn enumerate_path_edge_pairs(paths: &[PathRecord], edges: &[EdgeRecord]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for (path_idx, path) in paths.iter().enumerate() {
        for (edge_idx, edge) in edges.iter().enumerate() {
            if path.target == edge.source {
                pairs.push((path_idx, edge_idx));
            }
        }
    }
    pairs
}

fn fiber_index(source: usize, target: usize) -> usize {
    debug_assert!(source < 2);
    debug_assert!(target < 2);
    source * 2 + target
}

fn relation_label(relation: ConcreteShiftRelation2x2) -> &'static str {
    match relation {
        ConcreteShiftRelation2x2::Aligned => "aligned concrete-shift witness",
        ConcreteShiftRelation2x2::Balanced => "balanced concrete-shift witness",
        ConcreteShiftRelation2x2::Compatible => "compatible concrete-shift witness",
    }
}

fn matrix_support_summary(matrix: &SqMatrix<2>) -> MatrixSupportSummary2x2 {
    let mut nonzero_positions = Vec::new();
    let mut row_sums = [0u32; 2];
    let mut col_sums = [0u32; 2];

    for source in 0..2 {
        for target in 0..2 {
            let value = matrix.data[source][target];
            row_sums[source] += value;
            col_sums[target] += value;
            if value > 0 {
                nonzero_positions.push([source, target]);
            }
        }
    }

    MatrixSupportSummary2x2 {
        signature: sq_matrix_signature(matrix),
        matrix: matrix.data,
        nonzero_positions,
        row_sums,
        col_sums,
        entry_sum: matrix.entry_sum(),
        max_entry: matrix.max_entry(),
    }
}

fn sq_matrix_signature(matrix: &SqMatrix<2>) -> String {
    format!(
        "2x2:{},{},{},{}",
        matrix.data[0][0], matrix.data[0][1], matrix.data[1][0], matrix.data[1][1]
    )
}

fn bijection_signature(bijection: &FiberwiseBijection2x2) -> String {
    bijection
        .mapping
        .iter()
        .map(|fiber| {
            fiber
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn edge_summary(edge: &EdgeRecord) -> ConcreteShiftEdgeSummary2x2 {
    ConcreteShiftEdgeSummary2x2 {
        source: edge.source,
        target: edge.target,
        label: edge.label,
    }
}

fn path_summary(ctx_edges: &[EdgeRecord], path: &PathRecord) -> ConcreteShiftPathSummary2x2 {
    let signature = path
        .edges
        .iter()
        .map(|&edge_idx| {
            let edge = &ctx_edges[edge_idx];
            format!("{}>{}#{}", edge.source, edge.target, edge.label)
        })
        .collect::<Vec<_>>()
        .join("|");

    ConcreteShiftPathSummary2x2 {
        source: path.source,
        target: path.target,
        edge_count: path.edges.len(),
        signature,
    }
}

fn bridge_fiber_summaries_from_rs(
    ctx: &ModuleContext,
    witness: &ConcreteShiftWitness2x2,
    sample_limit_per_fiber: usize,
) -> Result<Vec<ConcreteShiftBridgeFiberSummary2x2>, String> {
    bridge_fiber_summaries(
        &ctx.omega_e_domain.fibers,
        &ctx.omega_e_codomain.fibers,
        &witness.omega_e.mapping,
        |(r_idx, s_idx), path_idx| ConcreteShiftBridgePathSample2x2 {
            r_edge: edge_summary(&ctx.g_edges[*r_idx]),
            s_edge: edge_summary(&ctx.h_edges[*s_idx]),
            mapped_path: path_summary(&ctx.e_edges, &ctx.e_paths[path_idx]),
        },
        sample_limit_per_fiber,
    )
}

fn bridge_fiber_summaries_from_sr(
    ctx: &ModuleContext,
    witness: &ConcreteShiftWitness2x2,
    sample_limit_per_fiber: usize,
) -> Result<Vec<ConcreteShiftBridgeFiberSummary2x2>, String> {
    bridge_fiber_summaries(
        &ctx.omega_f_domain.fibers,
        &ctx.omega_f_codomain.fibers,
        &witness.omega_f.mapping,
        |(s_idx, r_idx), path_idx| ConcreteShiftBridgePathSample2x2 {
            r_edge: edge_summary(&ctx.g_edges[*r_idx]),
            s_edge: edge_summary(&ctx.h_edges[*s_idx]),
            mapped_path: path_summary(&ctx.f_edges, &ctx.f_paths[path_idx]),
        },
        sample_limit_per_fiber,
    )
}

fn bridge_fiber_summaries<T, F>(
    domain_fibers: &[Vec<T>; 4],
    codomain_fibers: &[Vec<usize>; 4],
    mapping: &[Vec<usize>; 4],
    mut build_sample: F,
    sample_limit_per_fiber: usize,
) -> Result<Vec<ConcreteShiftBridgeFiberSummary2x2>, String>
where
    F: FnMut(&T, usize) -> ConcreteShiftBridgePathSample2x2,
{
    let mut summaries = Vec::new();

    for fiber in 0..4 {
        let fiber_mapping = mapping[fiber].as_slice();
        let fiber_codomain = codomain_fibers[fiber].as_slice();
        let samples = domain_fibers[fiber]
            .iter()
            .enumerate()
            .take(sample_limit_per_fiber)
            .map(|(local_idx, domain_item)| {
                let mapped_local = *fiber_mapping.get(local_idx).ok_or_else(|| {
                    format!(
                        "bridge sample mapping missing for fiber {fiber} at local index {local_idx}"
                    )
                })?;
                let path_idx = *fiber_codomain.get(mapped_local).ok_or_else(|| {
                    format!(
                        "bridge sample codomain missing for fiber {fiber} at mapped index {mapped_local}"
                    )
                })?;
                Ok(build_sample(domain_item, path_idx))
            })
            .collect::<Result<Vec<_>, String>>()?;

        summaries.push(ConcreteShiftBridgeFiberSummary2x2 {
            fiber: [fiber / 2, fiber % 2],
            bridge_pair_count: domain_fibers[fiber].len(),
            path_count: codomain_fibers[fiber].len(),
            samples,
        });
    }

    Ok(summaries)
}

fn stable_signature_hash(signature: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in signature.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn enumerate_intertwiners_2x2(
    left: &SqMatrix<2>,
    right: &SqMatrix<2>,
    max_entry: u32,
) -> Vec<SqMatrix<2>> {
    let mut candidates = Vec::new();

    for x00 in 0..=max_entry {
        for x01 in 0..=max_entry {
            for x10 in 0..=max_entry {
                for x11 in 0..=max_entry {
                    let x = SqMatrix::new([[x00, x01], [x10, x11]]);
                    if left.mul_u32(&x) == x.mul_u32(right) {
                        candidates.push(x);
                    }
                }
            }
        }
    }

    candidates
}

enum ModuleWitnessSearchOutcome {
    Found(ModuleShiftWitness2x2),
    Exhausted,
    LimitReached,
}

fn search_concrete_witnesses_for_shift(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    shift: ShiftEquivalenceWitness2x2,
    relation: ConcreteShiftRelation2x2,
    max_witnesses: usize,
    checked: &mut usize,
) -> ModuleWitnessSearchOutcome {
    let ctx = match ModuleContext::new(a, b, &shift) {
        Ok(ctx) => ctx,
        Err(_) => return ModuleWitnessSearchOutcome::Exhausted,
    };

    let sigma_g_lengths = ctx.sigma_g_domain.lengths();
    let sigma_h_lengths = ctx.sigma_h_domain.lengths();
    let omega_e_lengths = ctx.omega_e_domain.lengths();
    let omega_f_lengths = ctx.omega_f_domain.lengths();

    let search = for_each_fiberwise_bijection(sigma_g_lengths, &mut |sigma_g| {
        for_each_fiberwise_bijection(sigma_h_lengths, &mut |sigma_h| {
            for_each_fiberwise_bijection(omega_e_lengths, &mut |omega_e| {
                for_each_fiberwise_bijection(omega_f_lengths, &mut |omega_f| {
                    if *checked >= max_witnesses {
                        return ControlFlow::Break(ModuleWitnessSearchOutcome::LimitReached);
                    }
                    *checked += 1;

                    let witness = ModuleShiftWitness2x2 {
                        shift: shift.clone(),
                        sigma_g: sigma_g.clone(),
                        sigma_h: sigma_h.clone(),
                        omega_e: omega_e.clone(),
                        omega_f: omega_f.clone(),
                    };

                    if verify_concrete_shift_relation_2x2(a, b, &witness, relation).is_ok() {
                        ControlFlow::Break(ModuleWitnessSearchOutcome::Found(witness))
                    } else {
                        ControlFlow::Continue(())
                    }
                })
            })
        })
    });

    match search {
        ControlFlow::Break(outcome) => outcome,
        ControlFlow::Continue(()) => ModuleWitnessSearchOutcome::Exhausted,
    }
}

fn verify_concrete_shift_relation_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ModuleShiftWitness2x2,
    relation: ConcreteShiftRelation2x2,
) -> Result<(), String> {
    match relation {
        ConcreteShiftRelation2x2::Aligned => verify_aligned_concrete_shift_2x2(a, b, witness),
        ConcreteShiftRelation2x2::Balanced => verify_balanced_concrete_shift_2x2(a, b, witness),
        ConcreteShiftRelation2x2::Compatible => verify_compatible_concrete_shift_2x2(a, b, witness),
    }
}

fn iterate_phi_r(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    a_path_idx: usize,
    r_idx: usize,
) -> Result<(usize, usize), String> {
    let path = &ctx.e_paths[a_path_idx];
    let mut current_r = r_idx;
    let mut b_edges = Vec::with_capacity(path.edges.len());

    for &e_idx in &path.edges {
        let (next_r, b_idx) = apply_pair_bijection(
            &witness.sigma_g,
            &ctx.sigma_g_domain,
            &ctx.sigma_g_codomain,
            (e_idx, current_r),
            ctx.e_edges[e_idx].source,
            ctx.g_edges[current_r].target,
            "sigma_g",
        )?;
        current_r = next_r;
        b_edges.push(b_idx);
    }

    let b_path_idx = ctx
        .f_path_lookup
        .get(&b_edges)
        .copied()
        .ok_or_else(|| format!("sigma_g produced B-path {:?} not found", b_edges))?;
    Ok((current_r, b_path_idx))
}

fn iterate_phi_s(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    b_path_idx: usize,
    s_idx: usize,
) -> Result<(usize, usize), String> {
    let path = &ctx.f_paths[b_path_idx];
    let mut current_s = s_idx;
    let mut a_edges = Vec::with_capacity(path.edges.len());

    for &b_idx in &path.edges {
        let (next_s, a_idx) = apply_pair_bijection(
            &witness.sigma_h,
            &ctx.sigma_h_domain,
            &ctx.sigma_h_codomain,
            (b_idx, current_s),
            ctx.f_edges[b_idx].source,
            ctx.h_edges[current_s].target,
            "sigma_h",
        )?;
        current_s = next_s;
        a_edges.push(a_idx);
    }

    let a_path_idx = ctx
        .e_path_lookup
        .get(&a_edges)
        .copied()
        .ok_or_else(|| format!("sigma_h produced A-path {:?} not found", a_edges))?;
    Ok((current_s, a_path_idx))
}

fn balanced_rhs_a(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    a_path_idx: usize,
    r_idx: usize,
    s_idx: usize,
) -> Result<((usize, usize), usize), String> {
    let (r_prime, b_path_idx) = iterate_phi_r(ctx, witness, a_path_idx, r_idx)?;
    let (s_prime, a_prime_path_idx) = iterate_phi_s(ctx, witness, b_path_idx, s_idx)?;
    Ok(((r_prime, s_prime), a_prime_path_idx))
}

fn balanced_rhs_b(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    b_path_idx: usize,
    s_idx: usize,
    r_idx: usize,
) -> Result<((usize, usize), usize), String> {
    let (s_prime, a_path_idx) = iterate_phi_s(ctx, witness, b_path_idx, s_idx)?;
    let (r_prime, b_prime_path_idx) = iterate_phi_r(ctx, witness, a_path_idx, r_idx)?;
    Ok(((s_prime, r_prime), b_prime_path_idx))
}

fn compatible_rhs_r(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    a_path_idx: usize,
    r_idx: usize,
) -> Result<(usize, usize), String> {
    let path = &ctx.e_paths[a_path_idx];
    let (r_from_path, s_from_path) = invert_path_bijection(
        &witness.omega_e,
        &ctx.omega_e_domain,
        &ctx.omega_e_codomain,
        a_path_idx,
        path.source,
        path.target,
        "omega_e",
    )?;
    let b_path_idx = apply_path_bijection(
        &witness.omega_f,
        &ctx.omega_f_domain,
        &ctx.omega_f_codomain,
        (s_from_path, r_idx),
        ctx.h_edges[s_from_path].source,
        ctx.g_edges[r_idx].target,
        "omega_f",
    )?;
    Ok((r_from_path, b_path_idx))
}

fn compatible_rhs_s(
    ctx: &ModuleContext,
    witness: &ModuleShiftWitness2x2,
    b_path_idx: usize,
    s_idx: usize,
) -> Result<(usize, usize), String> {
    let path = &ctx.f_paths[b_path_idx];
    let (s_from_path, r_from_path) = invert_path_bijection(
        &witness.omega_f,
        &ctx.omega_f_domain,
        &ctx.omega_f_codomain,
        b_path_idx,
        path.source,
        path.target,
        "omega_f",
    )?;
    let a_path_idx = apply_path_bijection(
        &witness.omega_e,
        &ctx.omega_e_domain,
        &ctx.omega_e_codomain,
        (r_from_path, s_idx),
        ctx.g_edges[r_from_path].source,
        ctx.h_edges[s_idx].target,
        "omega_e",
    )?;
    Ok((s_from_path, a_path_idx))
}

fn for_each_fiberwise_bijection<R>(
    lengths: [usize; 4],
    callback: &mut impl FnMut(&FiberwiseBijection2x2) -> ControlFlow<R>,
) -> ControlFlow<R> {
    let mut buffer = FiberwiseBijection2x2::identity(lengths);
    recurse_fiberwise_bijection(0, lengths, &mut buffer, callback)
}

fn recurse_fiberwise_bijection<R>(
    fiber: usize,
    lengths: [usize; 4],
    buffer: &mut FiberwiseBijection2x2,
    callback: &mut impl FnMut(&FiberwiseBijection2x2) -> ControlFlow<R>,
) -> ControlFlow<R> {
    if fiber == 4 {
        return callback(buffer);
    }

    let mut current = (0..lengths[fiber]).collect::<Vec<_>>();
    for_each_permutation(&mut current, &mut |perm| {
        buffer.mapping[fiber] = perm.to_vec();
        recurse_fiberwise_bijection(fiber + 1, lengths, buffer, callback)
    })
}

fn for_each_permutation<R>(
    values: &mut [usize],
    callback: &mut impl FnMut(&[usize]) -> ControlFlow<R>,
) -> ControlFlow<R> {
    recurse_permutation(0, values, callback)
}

fn recurse_permutation<R>(
    start: usize,
    values: &mut [usize],
    callback: &mut impl FnMut(&[usize]) -> ControlFlow<R>,
) -> ControlFlow<R> {
    if start == values.len() {
        return callback(values);
    }

    for idx in start..values.len() {
        values.swap(start, idx);
        if let ControlFlow::Break(result) = recurse_permutation(start + 1, values, callback) {
            return ControlFlow::Break(result);
        }
        values.swap(start, idx);
    }

    ControlFlow::Continue(())
}

fn solve_left_product_2x2(
    left: &SqMatrix<2>,
    target: &SqMatrix<2>,
    max_entry: u32,
) -> Vec<SqMatrix<2>> {
    let first_col =
        solve_left_product_column_2x2(left, [target.data[0][0], target.data[1][0]], max_entry);
    if first_col.is_empty() {
        return Vec::new();
    }

    let second_col =
        solve_left_product_column_2x2(left, [target.data[0][1], target.data[1][1]], max_entry);
    if second_col.is_empty() {
        return Vec::new();
    }

    let mut solutions = Vec::new();
    for c0 in &first_col {
        for c1 in &second_col {
            solutions.push(SqMatrix::new([[c0[0], c1[0]], [c0[1], c1[1]]]));
        }
    }
    solutions
}

fn solve_left_product_column_2x2(
    left: &SqMatrix<2>,
    target_col: [u32; 2],
    max_entry: u32,
) -> Vec<[u32; 2]> {
    let [[a, b], [c, d]] = left.data;
    let [t0, t1] = target_col;
    let det = a as i64 * d as i64 - b as i64 * c as i64;

    if det != 0 {
        let x_num = d as i64 * t0 as i64 - b as i64 * t1 as i64;
        let y_num = a as i64 * t1 as i64 - c as i64 * t0 as i64;

        if x_num % det != 0 || y_num % det != 0 {
            return Vec::new();
        }

        let x = x_num / det;
        let y = y_num / det;
        if x < 0 || y < 0 || x > max_entry as i64 || y > max_entry as i64 {
            return Vec::new();
        }

        return vec![[x as u32, y as u32]];
    }

    let mut solutions = Vec::new();
    for x in 0..=max_entry {
        for y in 0..=max_entry {
            let lhs0 = a as u64 * x as u64 + b as u64 * y as u64;
            let lhs1 = c as u64 * x as u64 + d as u64 * y as u64;
            if lhs0 == t0 as u64 && lhs1 == t1 as u64 {
                solutions.push([x, y]);
            }
        }
    }
    solutions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_shift_equivalence_identity_witness() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let witness = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: a.clone(),
        };

        assert!(verify_shift_equivalence_2x2(&a, &a, &witness).is_ok());
    }

    #[test]
    fn test_verify_shift_equivalence_rejects_bad_witness() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let witness = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: SqMatrix::identity(),
        };

        assert!(verify_shift_equivalence_2x2(&a, &b, &witness).is_err());
    }

    #[test]
    fn test_find_shift_equivalence_identity_case() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let witness = find_shift_equivalence_2x2(&a, &a, 2, 3).expect("expected witness");
        assert!(verify_shift_equivalence_2x2(&a, &a, &witness).is_ok());
    }

    #[test]
    fn test_find_shift_equivalence_permutation_conjugate_case() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let witness = find_shift_equivalence_2x2(&a, &b, 1, 3).expect("expected witness");
        assert_eq!(witness.lag, 1);
        assert!(verify_shift_equivalence_2x2(&a, &b, &witness).is_ok());
    }

    #[test]
    fn test_find_shift_equivalence_zero_matrix_singular_case() {
        let z = SqMatrix::new([[0, 0], [0, 0]]);
        let witness = find_shift_equivalence_2x2(&z, &z, 1, 0).expect("expected witness");
        assert!(verify_shift_equivalence_2x2(&z, &z, &witness).is_ok());
    }

    #[test]
    fn test_concrete_shift_profile_records_low_lag_equivalent_hit() {
        let a = SqMatrix::identity();
        let profile = concrete_shift_profile_2x2(
            &a,
            &a,
            &ConcreteShiftProfileConfig2x2 {
                max_lag: 1,
                max_entry: 1,
                max_witnesses: 16,
                ..ConcreteShiftProfileConfig2x2::default()
            },
        );

        assert_eq!(profile.status, ConcreteShiftProfileStatus2x2::Equivalent);
        assert_eq!(profile.concrete_witness_lag, Some(1));
        assert!(!profile.limit_reached);
        assert!(profile.shift_witnesses > 0);
    }

    #[test]
    fn test_concrete_shift_profile_preserves_shift_signal_when_limit_reached() {
        let a = SqMatrix::identity();
        let profile = concrete_shift_profile_2x2(
            &a,
            &a,
            &ConcreteShiftProfileConfig2x2 {
                max_lag: 1,
                max_entry: 1,
                max_witnesses: 0,
                ..ConcreteShiftProfileConfig2x2::default()
            },
        );

        assert_eq!(
            profile.status,
            ConcreteShiftProfileStatus2x2::ShiftWitnessOnly
        );
        assert!(profile.limit_reached);
        assert!(profile.shift_witnesses > 0);
        assert_eq!(profile.concrete_witness_lag, None);
    }

    #[test]
    fn test_concrete_shift_profile_records_exhausted_without_shift_witness() {
        let a = SqMatrix::identity();
        let b = SqMatrix::new([[0, 0], [0, 0]]);
        let profile = concrete_shift_profile_2x2(
            &a,
            &b,
            &ConcreteShiftProfileConfig2x2 {
                max_lag: 1,
                max_entry: 0,
                max_witnesses: 16,
                ..ConcreteShiftProfileConfig2x2::default()
            },
        );

        assert_eq!(profile.status, ConcreteShiftProfileStatus2x2::Exhausted);
        assert!(!profile.limit_reached);
        assert_eq!(profile.shift_witnesses, 0);
    }

    #[test]
    fn test_canonical_module_shift_witness_self_identity() {
        let a = SqMatrix::identity();
        let shift = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: SqMatrix::identity(),
        };

        let witness = canonical_module_shift_witness_2x2(&a, &a, shift).expect("expected witness");
        assert!(verify_module_shift_equivalence_2x2(&a, &a, &witness).is_ok());
        assert!(verify_aligned_module_shift_equivalence_2x2(&a, &a, &witness).is_ok());
        assert!(verify_balanced_concrete_shift_2x2(&a, &a, &witness).is_ok());
        assert!(verify_compatible_concrete_shift_2x2(&a, &a, &witness).is_ok());
    }

    #[test]
    fn test_concrete_shift_proposal_data_marks_surface_as_non_replayable() {
        let a = SqMatrix::identity();
        let shift = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: SqMatrix::identity(),
        };
        let witness = canonical_module_shift_witness_2x2(&a, &a, shift).expect("expected witness");
        let proposal = concrete_shift_proposal_data_2x2(
            &a,
            &a,
            ConcreteShiftRelation2x2::Compatible,
            &witness,
            ConcreteShiftProposalBounds2x2 {
                max_lag: 1,
                max_entry: 1,
                max_witnesses: 8,
            },
            2,
        )
        .expect("expected proposal data");

        assert_eq!(proposal.artifact_kind, "concrete_shift_proposal_data");
        assert!(!proposal.replayable_as_full_path);
        assert_eq!(proposal.relation, "compatible");
        assert_eq!(proposal.witness_label, "compatible concrete-shift witness");
        assert_eq!(
            proposal.shift_signature,
            "lag=1|R=2x2:1,0,0,1|S=2x2:1,0,0,1"
        );
        assert!(proposal.witness_signature.starts_with("fnv1a64:"));
        assert_eq!(proposal.source.nonzero_positions, vec![[0, 0], [1, 1]]);
        assert_eq!(proposal.bridge_r.row_sums, [1, 1]);
        assert_eq!(proposal.rs_to_a_path_fibers.len(), 4);
        assert!(proposal
            .rs_to_a_path_fibers
            .iter()
            .all(|fiber| fiber.samples.len() <= 2));
    }

    #[test]
    fn test_concrete_shift_proposal_data_serializes_bridge_samples() {
        let a = SqMatrix::new([[0, 1], [1, 2]]);
        let b = SqMatrix::new([[1, 1], [2, 1]]);
        let witness = match search_concrete_shift_equivalence_with_lag_2x2(
            &a,
            &b,
            1,
            6,
            10_000,
            ConcreteShiftRelation2x2::Aligned,
        ) {
            ConcreteShiftSearchResult2x2::Equivalent(witness) => witness,
            other => panic!("expected bounded aligned witness, got {other:?}"),
        };
        let proposal = concrete_shift_proposal_data_2x2(
            &a,
            &b,
            ConcreteShiftRelation2x2::Aligned,
            &witness,
            ConcreteShiftProposalBounds2x2 {
                max_lag: 1,
                max_entry: 6,
                max_witnesses: 10_000,
            },
            1,
        )
        .expect("expected proposal data");
        let json = serde_json::to_value(&proposal).expect("serialize proposal");

        assert_eq!(json["relation"], "aligned");
        assert_eq!(json["lag"], 1);
        assert_eq!(json["search_bounds"]["max_entry"], 6);
        assert_eq!(
            json["rs_to_a_path_fibers"][1]["samples"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(
            json["rs_to_a_path_fibers"][1]["samples"][0]["mapped_path"]["signature"]
                .as_str()
                .unwrap()
                .contains('>')
        );
    }

    #[test]
    fn test_concrete_shift_proposal_data_rejects_mismatched_relation() {
        let a = SqMatrix::new([[2, 0], [0, 1]]);
        let shift = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: a.clone(),
        };
        let mut witness =
            canonical_module_shift_witness_2x2(&a, &a, shift).expect("expected witness");
        witness.sigma_g.mapping[0].swap(0, 1);

        let err = concrete_shift_proposal_data_2x2(
            &a,
            &a,
            ConcreteShiftRelation2x2::Aligned,
            &witness,
            ConcreteShiftProposalBounds2x2 {
                max_lag: 1,
                max_entry: 1,
                max_witnesses: 8,
            },
            1,
        )
        .unwrap_err();

        assert!(err.contains("aligned relation"));
    }

    #[test]
    fn test_concrete_shift_proposal_data_returns_err_for_bad_mapping() {
        let a = SqMatrix::identity();
        let shift = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: SqMatrix::identity(),
        };
        let mut witness =
            canonical_module_shift_witness_2x2(&a, &a, shift).expect("expected witness");
        witness.omega_e.mapping[0].push(0);

        let err = concrete_shift_proposal_data_2x2(
            &a,
            &a,
            ConcreteShiftRelation2x2::Compatible,
            &witness,
            ConcreteShiftProposalBounds2x2 {
                max_lag: 1,
                max_entry: 1,
                max_witnesses: 8,
            },
            1,
        )
        .unwrap_err();

        assert!(err.contains("omega_e fiber 0 has wrong length"));
    }

    #[test]
    fn test_concrete_shift_proposal_data_rejects_inconsistent_bounds() {
        let a = SqMatrix::identity();
        let shift = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: SqMatrix::identity(),
        };
        let witness = canonical_module_shift_witness_2x2(&a, &a, shift).expect("expected witness");
        let err = concrete_shift_proposal_data_2x2(
            &a,
            &a,
            ConcreteShiftRelation2x2::Compatible,
            &witness,
            ConcreteShiftProposalBounds2x2 {
                max_lag: 0,
                max_entry: 1,
                max_witnesses: 8,
            },
            1,
        )
        .unwrap_err();

        assert!(err.contains("exceeds reported max_lag"));
    }

    #[test]
    fn test_module_shift_equivalence_detects_non_permutation() {
        let a = SqMatrix::identity();
        let shift = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: SqMatrix::identity(),
        };
        let mut witness =
            canonical_module_shift_witness_2x2(&a, &a, shift).expect("expected witness");
        witness.sigma_g.mapping[0] = vec![0, 0];

        assert!(verify_module_shift_equivalence_2x2(&a, &a, &witness).is_err());
    }

    #[test]
    fn test_aligned_module_shift_detects_broken_alignment() {
        let a = SqMatrix::new([[2, 0], [0, 1]]);
        let shift = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: a.clone(),
        };
        let mut witness =
            canonical_module_shift_witness_2x2(&a, &a, shift).expect("expected witness");
        witness.sigma_g.mapping[0].swap(0, 1);

        assert!(verify_module_shift_equivalence_2x2(&a, &a, &witness).is_ok());
        assert!(verify_aligned_module_shift_equivalence_2x2(&a, &a, &witness).is_err());
    }

    #[test]
    fn test_enumerate_shift_equivalence_with_lag_identity_case() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let witnesses = enumerate_shift_equivalence_with_lag_2x2(&a, &a, 1, 3);
        assert!(!witnesses.is_empty());
        assert!(witnesses
            .iter()
            .all(|w| verify_shift_equivalence_2x2(&a, &a, w).is_ok()));
    }

    #[test]
    fn test_search_aligned_module_shift_equivalence_identity_case() {
        let a = SqMatrix::identity();
        let result = search_aligned_module_shift_equivalence_with_lag_2x2(&a, &a, 1, 1, 10);
        match result {
            AlignedModuleSearchResult2x2::Equivalent(witness) => {
                assert!(verify_aligned_module_shift_equivalence_2x2(&a, &a, &witness).is_ok());
            }
            other => panic!("expected Equivalent, got {:?}", other),
        }
    }

    #[test]
    fn test_search_aligned_module_shift_equivalence_exhausted() {
        let a = SqMatrix::new([[1, 0], [0, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 0]]);
        let result = search_aligned_module_shift_equivalence_with_lag_2x2(&a, &b, 1, 1, 100);
        assert_eq!(result, AlignedModuleSearchResult2x2::Exhausted);
    }

    #[test]
    fn test_search_aligned_module_shift_equivalence_limit_reached() {
        let a = SqMatrix::identity();
        let result = search_aligned_module_shift_equivalence_with_lag_2x2(&a, &a, 1, 1, 0);
        assert_eq!(result, AlignedModuleSearchResult2x2::SearchLimitReached);
    }

    #[test]
    fn test_search_concrete_shift_equivalence_identity_balanced_case() {
        let a = SqMatrix::identity();
        let result = search_concrete_shift_equivalence_with_lag_2x2(
            &a,
            &a,
            1,
            1,
            10,
            ConcreteShiftRelation2x2::Balanced,
        );
        match result {
            ConcreteShiftSearchResult2x2::Equivalent(witness) => {
                assert!(verify_balanced_concrete_shift_2x2(&a, &a, &witness).is_ok());
            }
            other => panic!("expected Equivalent, got {:?}", other),
        }
    }

    #[test]
    fn test_search_concrete_shift_equivalence_identity_compatible_case() {
        let a = SqMatrix::identity();
        let result = search_concrete_shift_equivalence_with_lag_2x2(
            &a,
            &a,
            1,
            1,
            10,
            ConcreteShiftRelation2x2::Compatible,
        );
        match result {
            ConcreteShiftSearchResult2x2::Equivalent(witness) => {
                assert!(verify_compatible_concrete_shift_2x2(&a, &a, &witness).is_ok());
            }
            other => panic!("expected Equivalent, got {:?}", other),
        }
    }
}
