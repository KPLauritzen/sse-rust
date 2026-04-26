use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use sse_core::endpoint_local_parity::mass_support_signature;
use sse_core::matrix::DynMatrix;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    let samples = selected_samples();
    let pairs = selected_pairs(&samples);
    let report = OrbitReport {
        model: ModelDescription {
            name: "weighted_bipartite_active_block".to_string(),
            description: "Delete all-zero rows and columns, keep separate row and column vertex colors, add one edge for every nonzero active-block entry, and use the entry value as the edge label. Stabilizers and transporters are brute-forced over S_r x S_c; the support-shadow fields repeat the same computation after replacing each positive entry by 1.".to_string(),
            max_permutation_pairs_per_profile: 576,
        },
        samples: samples.iter().map(build_sample_report).collect(),
        pairs: pairs
            .iter()
            .map(|pair| build_pair_report(pair, &samples))
            .collect::<Result<Vec<_>, _>>()?,
    };
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize report: {err}"))?;

    if let Some(path) = cli.json_out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
        }
        fs::write(&path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("wrote {}", path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    json_out: Option<PathBuf>,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut json_out = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json-out" => {
                json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--help" | "-h" => {
                return Err(
                    "usage: diagnose_active_block_orbit_profiles [--json-out PATH]".to_string(),
                );
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    Ok(Cli { json_out })
}

#[derive(Serialize)]
struct OrbitReport {
    model: ModelDescription,
    samples: Vec<SampleReport>,
    pairs: Vec<PairReport>,
}

#[derive(Serialize)]
struct ModelDescription {
    name: String,
    description: String,
    max_permutation_pairs_per_profile: usize,
}

#[derive(Clone)]
struct Sample {
    id: &'static str,
    label: &'static str,
    source: &'static str,
    matrix: DynMatrix,
}

#[derive(Clone)]
struct Pair {
    id: &'static str,
    label: &'static str,
    pair_kind: &'static str,
    left_id: &'static str,
    right_id: &'static str,
}

#[derive(Serialize)]
struct SampleReport {
    id: String,
    label: String,
    source: String,
    original_shape: String,
    active_shape: String,
    active_rows: Vec<usize>,
    active_cols: Vec<usize>,
    matrix: DynMatrix,
    active_block: DynMatrix,
    coarse_signature: String,
    support_profile: StabilizerProfile,
    weighted_profile: StabilizerProfile,
}

#[derive(Serialize)]
struct PairReport {
    id: String,
    label: String,
    pair_kind: String,
    left_id: String,
    right_id: String,
    active_shapes_match: bool,
    same_coarse_signature: bool,
    support_transporter_count: usize,
    weighted_transporter_count: usize,
    min_weighted_l1_over_all_row_col_perms: Option<u64>,
    min_weighted_l1_over_support_transporters: Option<u64>,
    left_support_stabilizer_size: usize,
    right_support_stabilizer_size: usize,
    left_weighted_stabilizer_size: usize,
    right_weighted_stabilizer_size: usize,
    diagnostic_reading: String,
}

#[derive(Serialize)]
struct StabilizerProfile {
    label_policy: &'static str,
    ambient_group_size: usize,
    stabilizer_size: usize,
    stabilizer_fraction: String,
    row_orbits: Vec<Vec<usize>>,
    col_orbits: Vec<Vec<usize>>,
    row_orbit_sizes: Vec<usize>,
    col_orbit_sizes: Vec<usize>,
    nonzero_edge_orbits: Vec<Vec<[usize; 2]>>,
    nonzero_edge_orbit_sizes: Vec<usize>,
}

#[derive(Clone)]
struct ActiveBlock {
    rows: Vec<usize>,
    cols: Vec<usize>,
    matrix: DynMatrix,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LabelPolicy {
    Support,
    Weighted,
}

#[derive(Clone, Debug)]
struct PermPair {
    rows: Vec<usize>,
    cols: Vec<usize>,
}

fn selected_samples() -> Vec<Sample> {
    vec![
        Sample {
            id: "brix_rank4_frontier",
            label: "Brix-Ruiz k=4 retained rank-4 diagonal near-hit frontier",
            source: "research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md rank 4 to_matrix",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
        },
        Sample {
            id: "brix_rank4_counterpart",
            label: "Brix-Ruiz k=4 retained rank-4 closest opposite-side counterpart",
            source: "research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md rank 4 counterpart_matrix",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 12, 0, 1, 1, 1, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
        },
        Sample {
            id: "brix_rank6_frontier",
            label: "Brix-Ruiz k=4 retained rank-6 diagonal near-hit frontier",
            source: "src/bin/diagnose_brix_ruiz_k4_active_block_switches.rs rank-6 cluster fixture",
            matrix: DynMatrix::new(
                4,
                4,
                vec![0, 2, 3, 0, 0, 2, 1, 0, 0, 11, 0, 0, 0, 2, 2, 0],
            ),
        },
        Sample {
            id: "brix_rank6_counterpart",
            label: "Brix-Ruiz k=4 retained rank-6 closest opposite-side counterpart",
            source: "src/bin/diagnose_brix_ruiz_k4_active_block_switches.rs rank-6 cluster fixture",
            matrix: DynMatrix::new(
                4,
                4,
                vec![0, 2, 1, 0, 0, 1, 4, 0, 0, 3, 1, 0, 0, 11, 0, 0],
            ),
        },
        Sample {
            id: "baker_a4",
            label: "Baker/Lind-Marcus k=3 hard same-size control A4",
            source: "research/guide_artifacts/k3_shortcut_round1.json path.matrices[4]",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 2, 2, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 2, 1, 0],
            ),
        },
        Sample {
            id: "baker_a5",
            label: "Baker/Lind-Marcus k=3 hard same-size control A5",
            source: "research/guide_artifacts/k3_shortcut_round1.json path.matrices[5]",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 1, 1, 1, 3, 0, 2, 2, 1, 0, 0, 0, 0, 1, 1, 1],
            ),
        },
        Sample {
            id: "k3_baker_step2",
            label: "Baker/Lind-Marcus replay overlap calibration step 2",
            source: "research/guide_artifacts/k3_shortcut_round1.json path.matrices[2]",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 2, 2, 0, 1, 0, 2, 0, 0, 1, 1, 1, 1, 1, 2, 0],
            ),
        },
        Sample {
            id: "k3_non_baker_step2",
            label: "Non-Baker exact replay overlap calibration step 2",
            source: "research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json path.matrices[2]",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 0, 1, 1, 2, 1, 0, 2, 2, 1, 0, 1, 2, 1, 0, 0],
            ),
        },
    ]
}

fn selected_pairs(_samples: &[Sample]) -> Vec<Pair> {
    vec![
        Pair {
            id: "brix_rank4_frontier_vs_counterpart",
            label: "retained Brix-Ruiz k=4 rank-4 frontier vs closest counterpart",
            pair_kind: "coarse_only_near_miss",
            left_id: "brix_rank4_frontier",
            right_id: "brix_rank4_counterpart",
        },
        Pair {
            id: "brix_rank6_frontier_vs_counterpart",
            label: "retained Brix-Ruiz k=4 rank-6 frontier vs closest counterpart",
            pair_kind: "coarse_only_near_miss",
            left_id: "brix_rank6_frontier",
            right_id: "brix_rank6_counterpart",
        },
        Pair {
            id: "baker_a4_to_a5",
            label: "Baker/Lind-Marcus hard same-size 4x4 A4 -> A5 control",
            pair_kind: "known_local_transfer_not_one_current_family",
            left_id: "baker_a4",
            right_id: "baker_a5",
        },
        Pair {
            id: "k3_replay_overlap_step2",
            label: "known k=3 Baker/non-Baker replay overlap step 2",
            pair_kind: "known_reuse_calibration",
            left_id: "k3_baker_step2",
            right_id: "k3_non_baker_step2",
        },
    ]
}

fn build_sample_report(sample: &Sample) -> SampleReport {
    let active = active_block(&sample.matrix);
    SampleReport {
        id: sample.id.to_string(),
        label: sample.label.to_string(),
        source: sample.source.to_string(),
        original_shape: shape_label(&sample.matrix),
        active_shape: shape_label(&active.matrix),
        active_rows: active.rows,
        active_cols: active.cols,
        matrix: sample.matrix.clone(),
        active_block: active.matrix.clone(),
        coarse_signature: mass_support_signature(&sample.matrix),
        support_profile: stabilizer_profile(&active.matrix, LabelPolicy::Support),
        weighted_profile: stabilizer_profile(&active.matrix, LabelPolicy::Weighted),
    }
}

fn build_pair_report(pair: &Pair, samples: &[Sample]) -> Result<PairReport, String> {
    let left = samples
        .iter()
        .find(|sample| sample.id == pair.left_id)
        .ok_or_else(|| format!("unknown sample id {}", pair.left_id))?;
    let right = samples
        .iter()
        .find(|sample| sample.id == pair.right_id)
        .ok_or_else(|| format!("unknown sample id {}", pair.right_id))?;
    let left_active = active_block(&left.matrix);
    let right_active = active_block(&right.matrix);
    let active_shapes_match = left_active.matrix.rows == right_active.matrix.rows
        && left_active.matrix.cols == right_active.matrix.cols;
    let support_transporter_count = transporter_count(
        &left_active.matrix,
        &right_active.matrix,
        LabelPolicy::Support,
    );
    let weighted_transporter_count = transporter_count(
        &left_active.matrix,
        &right_active.matrix,
        LabelPolicy::Weighted,
    );
    let min_weighted_l1_over_all_row_col_perms =
        min_l1_over_transporters(&left_active.matrix, &right_active.matrix, None);
    let min_weighted_l1_over_support_transporters = min_l1_over_transporters(
        &left_active.matrix,
        &right_active.matrix,
        Some(LabelPolicy::Support),
    );
    let left_support_stabilizer_size =
        automorphisms(&left_active.matrix, LabelPolicy::Support).len();
    let right_support_stabilizer_size =
        automorphisms(&right_active.matrix, LabelPolicy::Support).len();
    let left_weighted_stabilizer_size =
        automorphisms(&left_active.matrix, LabelPolicy::Weighted).len();
    let right_weighted_stabilizer_size =
        automorphisms(&right_active.matrix, LabelPolicy::Weighted).len();

    Ok(PairReport {
        id: pair.id.to_string(),
        label: pair.label.to_string(),
        pair_kind: pair.pair_kind.to_string(),
        left_id: pair.left_id.to_string(),
        right_id: pair.right_id.to_string(),
        active_shapes_match,
        same_coarse_signature: mass_support_signature(&left.matrix)
            == mass_support_signature(&right.matrix),
        support_transporter_count,
        weighted_transporter_count,
        min_weighted_l1_over_all_row_col_perms,
        min_weighted_l1_over_support_transporters,
        left_support_stabilizer_size,
        right_support_stabilizer_size,
        left_weighted_stabilizer_size,
        right_weighted_stabilizer_size,
        diagnostic_reading: diagnostic_reading(
            pair.pair_kind,
            support_transporter_count,
            weighted_transporter_count,
            min_weighted_l1_over_support_transporters,
        ),
    })
}

fn diagnostic_reading(
    pair_kind: &str,
    support_transporters: usize,
    weighted_transporters: usize,
    support_l1: Option<u64>,
) -> String {
    if weighted_transporters > 0 {
        return "weighted active-block orbit recognizes exact row/column reuse".to_string();
    }
    if support_transporters > 0 {
        return format!(
            "support orbit matches but weights do not; best weighted L1 inside support transporters is {}",
            support_l1
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unavailable".to_string())
        );
    }
    if pair_kind == "known_local_transfer_not_one_current_family" {
        "known local transfer is invisible to the active-block orbit profile".to_string()
    } else {
        "no active-block orbit overlap under this model".to_string()
    }
}

fn stabilizer_profile(block: &DynMatrix, policy: LabelPolicy) -> StabilizerProfile {
    let autos = automorphisms(block, policy);
    let row_orbits = vertex_orbits(block.rows, autos.iter().map(|auto| auto.rows.as_slice()));
    let col_orbits = vertex_orbits(block.cols, autos.iter().map(|auto| auto.cols.as_slice()));
    let nonzero_edge_orbits = cell_orbits(block, &autos, true);
    let ambient_group_size = factorial(block.rows) * factorial(block.cols);
    let stabilizer_size = autos.len();

    StabilizerProfile {
        label_policy: policy.label(),
        ambient_group_size,
        stabilizer_size,
        stabilizer_fraction: format!("{stabilizer_size}/{ambient_group_size}"),
        row_orbit_sizes: row_orbits.iter().map(Vec::len).collect(),
        col_orbit_sizes: col_orbits.iter().map(Vec::len).collect(),
        nonzero_edge_orbit_sizes: nonzero_edge_orbits.iter().map(Vec::len).collect(),
        row_orbits,
        col_orbits,
        nonzero_edge_orbits,
    }
}

fn active_block(matrix: &DynMatrix) -> ActiveBlock {
    let rows = (0..matrix.rows)
        .filter(|&row| (0..matrix.cols).any(|col| matrix.get(row, col) != 0))
        .collect::<Vec<_>>();
    let cols = (0..matrix.cols)
        .filter(|&col| (0..matrix.rows).any(|row| matrix.get(row, col) != 0))
        .collect::<Vec<_>>();
    let mut data = Vec::with_capacity(rows.len() * cols.len());
    for &row in &rows {
        for &col in &cols {
            data.push(matrix.get(row, col));
        }
    }
    let active_row_count = rows.len();
    let active_col_count = cols.len();
    ActiveBlock {
        rows,
        cols,
        matrix: DynMatrix::new(active_row_count, active_col_count, data),
    }
}

fn automorphisms(block: &DynMatrix, policy: LabelPolicy) -> Vec<PermPair> {
    let row_perms = permutations(block.rows);
    let col_perms = permutations(block.cols);
    let mut autos = Vec::new();
    for row_perm in &row_perms {
        for col_perm in &col_perms {
            if preserves(block, block, row_perm, col_perm, policy) {
                autos.push(PermPair {
                    rows: row_perm.clone(),
                    cols: col_perm.clone(),
                });
            }
        }
    }
    autos
}

fn transporter_count(left: &DynMatrix, right: &DynMatrix, policy: LabelPolicy) -> usize {
    if left.rows != right.rows || left.cols != right.cols {
        return 0;
    }
    let row_perms = permutations(left.rows);
    let col_perms = permutations(left.cols);
    row_perms
        .iter()
        .map(|row_perm| {
            col_perms
                .iter()
                .filter(|col_perm| preserves(left, right, row_perm, col_perm, policy))
                .count()
        })
        .sum()
}

fn min_l1_over_transporters(
    left: &DynMatrix,
    right: &DynMatrix,
    required_policy: Option<LabelPolicy>,
) -> Option<u64> {
    if left.rows != right.rows || left.cols != right.cols {
        return None;
    }
    let row_perms = permutations(left.rows);
    let col_perms = permutations(left.cols);
    let mut best = None;
    for row_perm in &row_perms {
        for col_perm in &col_perms {
            if required_policy
                .is_some_and(|policy| !preserves(left, right, row_perm, col_perm, policy))
            {
                continue;
            }
            let distance = permuted_l1(left, right, row_perm, col_perm);
            best = Some(best.map_or(distance, |current: u64| current.min(distance)));
        }
    }
    best
}

fn preserves(
    left: &DynMatrix,
    right: &DynMatrix,
    row_perm: &[usize],
    col_perm: &[usize],
    policy: LabelPolicy,
) -> bool {
    if left.rows != right.rows || left.cols != right.cols {
        return false;
    }
    for row in 0..left.rows {
        for col in 0..left.cols {
            if policy.label_value(left.get(row_perm[row], col_perm[col]))
                != policy.label_value(right.get(row, col))
            {
                return false;
            }
        }
    }
    true
}

fn permuted_l1(left: &DynMatrix, right: &DynMatrix, row_perm: &[usize], col_perm: &[usize]) -> u64 {
    let mut distance = 0u64;
    for row in 0..left.rows {
        for col in 0..left.cols {
            distance += left
                .get(row_perm[row], col_perm[col])
                .abs_diff(right.get(row, col)) as u64;
        }
    }
    distance
}

fn vertex_orbits<'a, I>(count: usize, perms: I) -> Vec<Vec<usize>>
where
    I: Iterator<Item = &'a [usize]>,
{
    let mut uf = UnionFind::new(count);
    for perm in perms {
        for (idx, &mapped) in perm.iter().enumerate() {
            uf.union(idx, mapped);
        }
    }
    uf.partitions()
}

fn cell_orbits(block: &DynMatrix, autos: &[PermPair], nonzero_only: bool) -> Vec<Vec<[usize; 2]>> {
    let mut uf = UnionFind::new(block.rows * block.cols);
    for auto in autos {
        for row in 0..block.rows {
            for col in 0..block.cols {
                let left = row * block.cols + col;
                let right = auto.rows[row] * block.cols + auto.cols[col];
                uf.union(left, right);
            }
        }
    }
    let mut by_root = BTreeMap::<usize, Vec<[usize; 2]>>::new();
    for row in 0..block.rows {
        for col in 0..block.cols {
            if nonzero_only && block.get(row, col) == 0 {
                continue;
            }
            by_root
                .entry(uf.find_const(row * block.cols + col))
                .or_default()
                .push([row, col]);
        }
    }
    by_root.into_values().collect()
}

fn permutations(n: usize) -> Vec<Vec<usize>> {
    let mut perm = (0..n).collect::<Vec<_>>();
    let mut out = Vec::new();
    loop {
        out.push(perm.clone());
        if !next_permutation(&mut perm) {
            break;
        }
    }
    out
}

fn next_permutation(perm: &mut [usize]) -> bool {
    if perm.len() <= 1 {
        return false;
    }
    let mut i = perm.len() - 1;
    while i > 0 && perm[i - 1] >= perm[i] {
        i -= 1;
    }
    if i == 0 {
        return false;
    }
    let mut j = perm.len() - 1;
    while perm[j] <= perm[i - 1] {
        j -= 1;
    }
    perm.swap(i - 1, j);
    perm[i..].reverse();
    true
}

fn factorial(n: usize) -> usize {
    (1..=n).product()
}

fn shape_label(matrix: &DynMatrix) -> String {
    format!("{}x{}", matrix.rows, matrix.cols)
}

impl LabelPolicy {
    fn label(self) -> &'static str {
        match self {
            Self::Support => "support_shadow",
            Self::Weighted => "exact_entry_weighted",
        }
    }

    fn label_value(self, value: u32) -> u32 {
        match self {
            Self::Support => u32::from(value != 0),
            Self::Weighted => value,
        }
    }
}

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            let root = self.find(self.parent[value]);
            self.parent[value] = root;
        }
        self.parent[value]
    }

    fn find_const(&self, value: usize) -> usize {
        let mut current = value;
        while self.parent[current] != current {
            current = self.parent[current];
        }
        current
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parent[right_root] = left_root;
        }
    }

    fn partitions(mut self) -> Vec<Vec<usize>> {
        let mut by_root = BTreeMap::<usize, Vec<usize>>::new();
        for idx in 0..self.parent.len() {
            by_root.entry(self.find(idx)).or_default().push(idx);
        }
        by_root.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank4_support_shadow_collapses_weighted_miss() {
        let samples = selected_samples();
        let report = build_pair_report(
            &Pair {
                id: "rank4",
                label: "rank4",
                pair_kind: "coarse_only_near_miss",
                left_id: "brix_rank4_frontier",
                right_id: "brix_rank4_counterpart",
            },
            &samples,
        )
        .expect("rank-4 pair should be present");

        assert_eq!(report.support_transporter_count, 6);
        assert_eq!(report.weighted_transporter_count, 0);
        assert_eq!(report.left_support_stabilizer_size, 6);
        assert_eq!(report.right_support_stabilizer_size, 6);
        assert_eq!(report.left_weighted_stabilizer_size, 1);
        assert_eq!(report.right_weighted_stabilizer_size, 1);
    }

    #[test]
    fn rank6_support_shadow_collapses_weighted_miss() {
        let samples = selected_samples();
        let report = build_pair_report(
            &Pair {
                id: "rank6",
                label: "rank6",
                pair_kind: "coarse_only_near_miss",
                left_id: "brix_rank6_frontier",
                right_id: "brix_rank6_counterpart",
            },
            &samples,
        )
        .expect("rank-6 pair should be present");

        assert_eq!(report.support_transporter_count, 6);
        assert_eq!(report.weighted_transporter_count, 0);
        assert_eq!(report.left_support_stabilizer_size, 6);
        assert_eq!(report.right_support_stabilizer_size, 6);
        assert_eq!(report.left_weighted_stabilizer_size, 1);
        assert_eq!(report.right_weighted_stabilizer_size, 1);
    }

    #[test]
    fn known_replay_overlap_has_weighted_transporter() {
        let samples = selected_samples();
        let report = build_pair_report(
            &Pair {
                id: "overlap",
                label: "overlap",
                pair_kind: "known_reuse_calibration",
                left_id: "k3_baker_step2",
                right_id: "k3_non_baker_step2",
            },
            &samples,
        )
        .expect("k3 overlap pair should be present");

        assert!(report.weighted_transporter_count > 0);
        assert_eq!(report.min_weighted_l1_over_all_row_col_perms, Some(0));
    }

    #[test]
    fn baker_same_size_control_has_no_active_block_transporter() {
        let samples = selected_samples();
        let report = build_pair_report(
            &Pair {
                id: "baker",
                label: "baker",
                pair_kind: "known_local_transfer_not_one_current_family",
                left_id: "baker_a4",
                right_id: "baker_a5",
            },
            &samples,
        )
        .expect("Baker A4 -> A5 pair should be present");

        assert_eq!(report.support_transporter_count, 0);
        assert_eq!(report.weighted_transporter_count, 0);
        assert_eq!(report.min_weighted_l1_over_all_row_col_perms, Some(5));
    }
}
