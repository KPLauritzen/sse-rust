use std::cell::RefCell;
use std::collections::HashMap;

use crate::matrix::{DynMatrix, SqMatrix};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

const SOLVE_NONNEG_2X3_CACHE_LIMIT: usize = 16_384;
const SOLVE_OVERDETERMINED_3X2_CACHE_LIMIT: usize = 16_384;

type SolveNonneg2x3Key = ([[i64; 3]; 2], [i64; 2], u32);
type SolveOverdetermined3x2Key = ([[i64; 2]; 3], [i64; 3], u32);

thread_local! {
    // Reuse small linear-system results across nearby frontier states without
    // paying the cloning/synchronization cost of caching full factorizations.
    static SOLVE_NONNEG_2X3_CACHE: RefCell<HashMap<SolveNonneg2x3Key, Vec<[u32; 3]>>> =
        RefCell::new(HashMap::new());
    static SOLVE_OVERDETERMINED_3X2_CACHE:
        RefCell<HashMap<SolveOverdetermined3x2Key, Option<[u32; 2]>>> =
            RefCell::new(HashMap::new());
}

/// Enumerate all 2x2 nonneg integer factorisations A = UV where U and V are 2x2
/// with entries in 0..=max_entry and det(U) > 0.
///
/// Algorithm: iterate over all 2x2 nonneg U with positive determinant,
/// compute V = adj(U) * A / det(U), check that all entries are nonneg integers
/// within the entry bound.
pub fn enumerate_square_factorisations_2x2(
    a: &SqMatrix<2>,
    max_entry: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    let mut results = Vec::new();
    visit_square_factorisations_2x2(a, max_entry, &mut |u, v| results.push((u, v)));
    results
}

fn visit_square_factorisations_2x2<F>(a: &SqMatrix<2>, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    let [[a00, a01], [a10, a11]] = a.data;

    let me = max_entry as i64;
    let det_a = a00 as i64 * a11 as i64 - a01 as i64 * a10 as i64;

    for u00 in 0..=max_entry {
        for u01 in 0..=max_entry {
            for u10 in 0..=max_entry {
                for u11 in 0..=max_entry {
                    let det = u00 as i64 * u11 as i64 - u01 as i64 * u10 as i64;
                    if det <= 0 {
                        continue;
                    }

                    // det(A) = det(U)*det(V), so det(U) must divide det(A).
                    if det_a % det != 0 {
                        continue;
                    }

                    // adj(U) = [[u11, -u01], [-u10, u00]]
                    // V = adj(U) * A / det
                    // v00 = (u11*a00 - u01*a10) / det
                    // v01 = (u11*a01 - u01*a11) / det
                    // v10 = (-u10*a00 + u00*a10) / det
                    // v11 = (-u10*a01 + u00*a11) / det

                    let v00_num = u11 as i64 * a00 as i64 - u01 as i64 * a10 as i64;
                    if v00_num < 0 || v00_num % det != 0 {
                        continue;
                    }
                    let v00 = v00_num / det;
                    if v00 > me {
                        continue;
                    }

                    let v01_num = u11 as i64 * a01 as i64 - u01 as i64 * a11 as i64;
                    if v01_num < 0 || v01_num % det != 0 {
                        continue;
                    }
                    let v01 = v01_num / det;
                    if v01 > me {
                        continue;
                    }

                    let v10_num = -(u10 as i64) * a00 as i64 + u00 as i64 * a10 as i64;
                    if v10_num < 0 || v10_num % det != 0 {
                        continue;
                    }
                    let v10 = v10_num / det;
                    if v10 > me {
                        continue;
                    }

                    let v11_num = -(u10 as i64) * a01 as i64 + u00 as i64 * a11 as i64;
                    if v11_num < 0 || v11_num % det != 0 {
                        continue;
                    }
                    let v11 = v11_num / det;
                    if v11 > me {
                        continue;
                    }

                    let u = DynMatrix::new(2, 2, vec![u00, u01, u10, u11]);
                    let v =
                        DynMatrix::new(2, 2, vec![v00 as u32, v01 as u32, v10 as u32, v11 as u32]);
                    visit(u, v);
                }
            }
        }
    }
}

/// Compute VU as a SqMatrix<2> given DynMatrix factors.
pub fn vu_product_2x2(v: &DynMatrix, u: &DynMatrix) -> SqMatrix<2> {
    let vu = v.mul(u);
    vu.to_sq::<2>().expect("VU product should be 2x2")
}

// --- Nonneg integer linear system solvers ---

/// Solve U·x = b where U is 2×3 (given as rows), b is 2-vector.
/// Returns all nonneg integer 3-vectors x with entries ≤ max_entry.
///
/// Algorithm: find a 2×2 pivot submatrix with nonzero determinant,
/// compute the 1D null space, find a particular solution, then enumerate
/// the free parameter t such that x0 + t*n has all entries in [0, max_entry].
pub fn solve_nonneg_2x3(u: &[[i64; 3]; 2], b: &[i64; 2], max_entry: u32) -> Vec<[u32; 3]> {
    let key = (*u, *b, max_entry);
    if let Some(cached) = SOLVE_NONNEG_2X3_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return cached;
    }

    let solutions = solve_nonneg_2x3_uncached(u, b, max_entry);
    SOLVE_NONNEG_2X3_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= SOLVE_NONNEG_2X3_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, solutions.clone());
    });
    solutions
}

fn solve_nonneg_2x3_uncached(u: &[[i64; 3]; 2], b: &[i64; 2], max_entry: u32) -> Vec<[u32; 3]> {
    let me = max_entry as i64;

    // Compute 2×2 minors: d_ij = u[0][i]*u[1][j] - u[0][j]*u[1][i]
    let d01 = u[0][0] * u[1][1] - u[0][1] * u[1][0];
    let d02 = u[0][0] * u[1][2] - u[0][2] * u[1][0];
    let d12 = u[0][1] * u[1][2] - u[0][2] * u[1][1];

    // Pick pivot (first nonzero minor) and determine free variable index.
    // pivot_cols = the two columns forming the pivot submatrix
    // free_col = the remaining column
    let (det, pivot_cols, free_col) = if d01 != 0 {
        (d01, [0, 1], 2)
    } else if d02 != 0 {
        (d02, [0, 2], 1)
    } else if d12 != 0 {
        (d12, [1, 2], 0)
    } else {
        // Rank < 2, no solutions in general (unless b is in the column space,
        // but we skip this degenerate case).
        return vec![];
    };

    // Null vector: for pivot columns (i, j) and free column k,
    // the null vector is n[i] = cofactor_i, n[j] = cofactor_j, n[k] = det.
    // Specifically, if pivot is columns (p0, p1) with det d:
    //   n[free] = d (or -d, we'll choose sign for convenience)
    //   n[p0] = -(u[0][free]*u[1][p1] - u[0][p1]*u[1][free])
    //   n[p1] = u[0][free]*u[1][p0] - u[0][p0]*u[1][free]
    // But more directly, the null vector of [[a,b,c],[d,e,f]] is:
    //   (bf-ce, cd-af, ae-bd) = (d12, -d02, d01) (up to sign).
    // This is always valid regardless of pivot choice.
    let null = [d12, -d02, d01];

    // Particular solution: solve the 2×2 pivot system with free variable = 0.
    // pivot system: u[r][pivot_cols[0]]*x[p0] + u[r][pivot_cols[1]]*x[p1] = b[r]
    let p0 = pivot_cols[0];
    let p1 = pivot_cols[1];

    // x[p0] = (b[0]*u[1][p1] - b[1]*u[0][p1]) / det
    // x[p1] = (b[1]*u[0][p0] - b[0]*u[1][p0]) / det
    let xp0_num = b[0] * u[1][p1] - b[1] * u[0][p1];
    let xp1_num = b[1] * u[0][p0] - b[0] * u[1][p0];

    if xp0_num % det != 0 || xp1_num % det != 0 {
        return vec![];
    }

    let mut x0 = [0i64; 3];
    x0[p0] = xp0_num / det;
    x0[p1] = xp1_num / det;
    x0[free_col] = 0;

    // General solution: x = x0 + t * null (for integer t).
    // But null might not be primitive — divide by gcd for finer enumeration.
    let g = gcd3(
        null[0].unsigned_abs(),
        null[1].unsigned_abs(),
        null[2].unsigned_abs(),
    );
    if g == 0 {
        // Null vector is zero — unique solution, just check bounds.
        if x0.iter().all(|&v| v >= 0 && v <= me) {
            return vec![[x0[0] as u32, x0[1] as u32, x0[2] as u32]];
        }
        return vec![];
    }
    let n = [null[0] / g as i64, null[1] / g as i64, null[2] / g as i64];

    // Find the range of t such that 0 <= x0[i] + t*n[i] <= max_entry for all i.
    let mut t_min = i64::MIN;
    let mut t_max = i64::MAX;

    for i in 0..3 {
        if n[i] == 0 {
            // x0[i] must be in [0, me] on its own.
            if x0[i] < 0 || x0[i] > me {
                return vec![];
            }
        } else if n[i] > 0 {
            // t >= ceil(-x0[i] / n[i]) and t <= floor((me - x0[i]) / n[i])
            let lo = div_ceil(-x0[i], n[i]);
            let hi = div_floor(me - x0[i], n[i]);
            t_min = t_min.max(lo);
            t_max = t_max.min(hi);
        } else {
            // n[i] < 0
            let lo = div_ceil(me - x0[i], n[i]); // dividing by negative flips
            let hi = div_floor(-x0[i], n[i]);
            t_min = t_min.max(lo);
            t_max = t_max.min(hi);
        }
    }

    let mut results = Vec::new();
    if t_min > t_max {
        return results;
    }

    for t in t_min..=t_max {
        let x = [
            (x0[0] + t * n[0]) as u32,
            (x0[1] + t * n[1]) as u32,
            (x0[2] + t * n[2]) as u32,
        ];
        results.push(x);
    }
    results
}

/// Solve U·x = b where U is 3×2 (given as rows), b is 3-vector.
/// Returns the unique nonneg integer 2-vector x with entries ≤ max_entry, if it exists.
///
/// Overdetermined system: pick a 2×2 submatrix, solve, verify the third equation.
pub fn solve_overdetermined_3x2(
    u: &[[i64; 2]; 3],
    b: &[i64; 3],
    max_entry: u32,
) -> Option<[u32; 2]> {
    let key = (*u, *b, max_entry);
    if let Some(cached) =
        SOLVE_OVERDETERMINED_3X2_CACHE.with(|cache| cache.borrow().get(&key).copied())
    {
        return cached;
    }

    let solution = solve_overdetermined_3x2_uncached(u, b, max_entry);
    SOLVE_OVERDETERMINED_3X2_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= SOLVE_OVERDETERMINED_3X2_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, solution);
    });
    solution
}

fn solve_overdetermined_3x2_uncached(
    u: &[[i64; 2]; 3],
    b: &[i64; 3],
    max_entry: u32,
) -> Option<[u32; 2]> {
    let me = max_entry as i64;

    // Try each pair of rows as pivot.
    let row_pairs = [(0, 1, 2), (0, 2, 1), (1, 2, 0)];
    for &(r0, r1, r_check) in &row_pairs {
        let det = u[r0][0] * u[r1][1] - u[r0][1] * u[r1][0];
        if det == 0 {
            continue;
        }

        let x0_num = b[r0] * u[r1][1] - b[r1] * u[r0][1];
        let x1_num = b[r1] * u[r0][0] - b[r0] * u[r1][0];

        if x0_num % det != 0 || x1_num % det != 0 {
            return None;
        }

        let x0 = x0_num / det;
        let x1 = x1_num / det;

        // Verify third equation.
        if u[r_check][0] * x0 + u[r_check][1] * x1 != b[r_check] {
            return None;
        }

        if x0 >= 0 && x0 <= me && x1 >= 0 && x1 <= me {
            return Some([x0 as u32, x1 as u32]);
        }
        return None;
    }

    // All 2×2 submatrices are singular. Check if b is in the column space
    // and there's a nonneg solution. This is rare; skip for now.
    None
}

/// Solve U·x = b where U is 4×3 (given as rows), b is 4-vector.
/// Returns the unique nonneg integer 3-vector x with entries ≤ max_entry, if it exists.
fn solve_overdetermined_4x3(u: &[[i64; 3]; 4], b: &[i64; 4], max_entry: u32) -> Option<[u32; 3]> {
    let row_sets = [
        (0usize, 1usize, 2usize, 3usize),
        (0, 1, 3, 2),
        (0, 2, 3, 1),
        (1, 2, 3, 0),
    ];

    for &(r0, r1, r2, r_check) in &row_sets {
        let system = [u[r0], u[r1], u[r2]];
        let rhs = [b[r0], b[r1], b[r2]];
        for solution in solve_nonneg_3x3(&system, &rhs, max_entry) {
            let check = u[r_check][0] * solution[0] as i64
                + u[r_check][1] * solution[1] as i64
                + u[r_check][2] * solution[2] as i64;
            if check == b[r_check] {
                return Some(solution);
            }
        }
    }

    None
}

// --- Rectangular factorisation enumerators ---

/// Enumerate all factorisations A = UV where U is 2×3, V is 3×2,
/// with all entries in 0..=max_entry.
pub fn enumerate_rect_factorisations_2x3(
    a: &SqMatrix<2>,
    max_entry: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    let mut results = Vec::new();
    visit_rect_factorisations_2x3(a, max_entry, &mut |u, v| results.push((u, v)));
    results
}

fn visit_rect_factorisations_2x3<F>(a: &SqMatrix<2>, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    let me = max_entry;
    let a_cols: [[i64; 2]; 2] = [
        [a.data[0][0] as i64, a.data[1][0] as i64],
        [a.data[0][1] as i64, a.data[1][1] as i64],
    ];

    // Minimum row sum for each row of U: since a_{ij} <= row_sum(U_i) * max_entry,
    // we need row_sum(U_i) >= ceil(max(a_{i0}, a_{i1}) / max_entry).
    let max_a_row0 = a.data[0][0].max(a.data[0][1]) as u64;
    let max_a_row1 = a.data[1][0].max(a.data[1][1]) as u64;
    let min_row_sum_0 = ((max_a_row0 + me as u64 - 1) / me as u64) as u32;
    let min_row_sum_1 = ((max_a_row1 + me as u64 - 1) / me as u64) as u32;

    let mut valid_row0s = Vec::new();
    for u00 in 0..=me {
        for u01 in 0..=me {
            for u02 in 0..=me {
                let row0_sum = u00 + u01 + u02;
                // Row 0 must be able to produce A's entries via V with entries <= max_entry.
                if row0_sum < min_row_sum_0 {
                    continue;
                }
                // Row 0 all zeros can't produce nonzero A entries.
                if row0_sum == 0 && (a.data[0][0] > 0 || a.data[0][1] > 0) {
                    continue;
                }

                // Constraint propagation: for U·x = A[:,j] to have a nonneg integer
                // solution, gcd(row0) must divide every A[0,j].
                let g0 = gcd3(u00 as u64, u01 as u64, u02 as u64);
                if g0 > 1 && (a.data[0][0] as u64 % g0 != 0 || a.data[0][1] as u64 % g0 != 0) {
                    continue;
                }

                valid_row0s.push([u00, u01, u02]);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let per_row0: Vec<Vec<(DynMatrix, DynMatrix)>> = valid_row0s
            .par_iter()
            .map(|&row0| {
                enumerate_rect_factorisations_2x3_from_row0(
                    row0,
                    &a_cols,
                    a,
                    max_entry,
                    min_row_sum_1,
                )
            })
            .collect();
        for row_results in per_row0 {
            for (u, v) in row_results {
                visit(u, v);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        for row0 in valid_row0s {
            for (u, v) in enumerate_rect_factorisations_2x3_from_row0(
                row0,
                &a_cols,
                a,
                max_entry,
                min_row_sum_1,
            ) {
                visit(u, v);
            }
        }
    }
}

fn enumerate_rect_factorisations_2x3_from_row0(
    row0: [u32; 3],
    a_cols: &[[i64; 2]; 2],
    a: &SqMatrix<2>,
    max_entry: u32,
    min_row_sum_1: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    let [u00, u01, u02] = row0;
    let me = max_entry;
    let mut results = Vec::new();

    for u10 in 0..=me {
        for u11 in 0..=me {
            for u12 in 0..=me {
                let row1_sum = u10 + u11 + u12;
                if row1_sum < min_row_sum_1 {
                    continue;
                }
                if row1_sum == 0 && (a.data[1][0] > 0 || a.data[1][1] > 0) {
                    continue;
                }

                // Constraint propagation: gcd(row1) must divide every A[1,j].
                let g1 = gcd3(u10 as u64, u11 as u64, u12 as u64);
                if g1 > 1 && (a.data[1][0] as u64 % g1 != 0 || a.data[1][1] as u64 % g1 != 0) {
                    continue;
                }

                let u_rows: [[i64; 3]; 2] = [
                    [u00 as i64, u01 as i64, u02 as i64],
                    [u10 as i64, u11 as i64, u12 as i64],
                ];

                // Quick rank check: all 2x2 minors zero -> rank < 2.
                let d01 = u_rows[0][0] * u_rows[1][1] - u_rows[0][1] * u_rows[1][0];
                let d02 = u_rows[0][0] * u_rows[1][2] - u_rows[0][2] * u_rows[1][0];
                let d12 = u_rows[0][1] * u_rows[1][2] - u_rows[0][2] * u_rows[1][1];
                if d01 == 0 && d02 == 0 && d12 == 0 {
                    continue;
                }

                let col0_solutions = solve_nonneg_2x3(&u_rows, &a_cols[0], max_entry);
                if col0_solutions.is_empty() {
                    continue;
                }
                let col1_solutions = solve_nonneg_2x3(&u_rows, &a_cols[1], max_entry);
                if col1_solutions.is_empty() {
                    continue;
                }

                let u_mat = DynMatrix::new(2, 3, vec![u00, u01, u02, u10, u11, u12]);
                for vc0 in &col0_solutions {
                    for vc1 in &col1_solutions {
                        let v_mat = DynMatrix::new(
                            3,
                            2,
                            vec![vc0[0], vc1[0], vc0[1], vc1[1], vc0[2], vc1[2]],
                        );
                        results.push((u_mat.clone(), v_mat));
                    }
                }
            }
        }
    }

    results
}

/// Enumerate all factorisations C = UV where C is 3×3, U is 3×2, V is 2×3,
/// with all entries in 0..=max_entry.
pub fn enumerate_factorisations_3x3_to_2(
    c: &DynMatrix,
    max_entry: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    let mut results = Vec::new();
    visit_factorisations_3x3_to_2(c, max_entry, &mut |u, v| results.push((u, v)));
    results
}

fn visit_factorisations_3x3_to_2<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);
    let me = max_entry;
    let me_i64 = max_entry as i64;

    // c_cols[j][i] = C[i, j]  (column-major view for the solvers).
    let c_cols: [[i64; 3]; 3] = [
        [c.get(0, 0) as i64, c.get(1, 0) as i64, c.get(2, 0) as i64],
        [c.get(0, 1) as i64, c.get(1, 1) as i64, c.get(2, 1) as i64],
        [c.get(0, 2) as i64, c.get(1, 2) as i64, c.get(2, 2) as i64],
    ];

    // Minimum row sum for each row of U (3×2): row_sum(U_i) >= ceil(max(c[i][*]) / max_entry).
    let min_row_sum: [u32; 3] = [
        {
            let mx = c.get(0, 0).max(c.get(0, 1)).max(c.get(0, 2)) as u64;
            ((mx + me as u64 - 1) / me as u64) as u32
        },
        {
            let mx = c.get(1, 0).max(c.get(1, 1)).max(c.get(1, 2)) as u64;
            ((mx + me as u64 - 1) / me as u64) as u32
        },
        {
            let mx = c.get(2, 0).max(c.get(2, 1)).max(c.get(2, 2)) as u64;
            ((mx + me as u64 - 1) / me as u64) as u32
        },
    ];

    // Row 2 of C as a flat array, used in the fast path to derive U's row 2.
    // c_row2[j] = C[2, j] = c_cols[j][2].
    let c_row2: [i64; 3] = [c_cols[0][2], c_cols[1][2], c_cols[2][2]];

    let mut valid_row0s = Vec::new();
    for u00 in 0..=me {
        for u01 in 0..=me {
            if u00 + u01 < min_row_sum[0] {
                continue;
            }

            // Constraint propagation: gcd(row0) must divide every C[0, j].
            let g0 = gcd(u00 as u64, u01 as u64);
            if g0 > 1 {
                let mut skip = false;
                for j in 0..3 {
                    if c_cols[j][0] as u64 % g0 != 0 {
                        skip = true;
                        break;
                    }
                }
                if skip {
                    continue;
                }
            }

            valid_row0s.push([u00, u01]);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let per_row0: Vec<Vec<(DynMatrix, DynMatrix)>> = valid_row0s
            .par_iter()
            .map(|&row0| {
                enumerate_factorisations_3x3_to_2_from_row0(
                    row0,
                    &c_cols,
                    &c_row2,
                    max_entry,
                    me_i64,
                    min_row_sum,
                )
            })
            .collect();
        for row_results in per_row0 {
            for (u, v) in row_results {
                visit(u, v);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        for row0 in valid_row0s {
            for (u, v) in enumerate_factorisations_3x3_to_2_from_row0(
                row0,
                &c_cols,
                &c_row2,
                max_entry,
                me_i64,
                min_row_sum,
            ) {
                visit(u, v);
            }
        }
    }
}

fn enumerate_factorisations_3x3_to_2_from_row0(
    row0: [u32; 2],
    c_cols: &[[i64; 3]; 3],
    c_row2: &[i64; 3],
    max_entry: u32,
    me_i64: i64,
    min_row_sum: [u32; 3],
) -> Vec<(DynMatrix, DynMatrix)> {
    let [u00, u01] = row0;
    let me = max_entry;
    let mut results = Vec::new();

    for u10 in 0..=me {
        for u11 in 0..=me {
            if u10 + u11 < min_row_sum[1] {
                continue;
            }

            let g1 = gcd(u10 as u64, u11 as u64);
            if g1 > 1 {
                let mut skip = false;
                for j in 0..3 {
                    if c_cols[j][1] as u64 % g1 != 0 {
                        skip = true;
                        break;
                    }
                }
                if skip {
                    continue;
                }
            }

            let det01 = u00 as i64 * u11 as i64 - u01 as i64 * u10 as i64;

            if det01 != 0 {
                let mut v_cols = [[0i64; 2]; 3];
                let mut v_valid = true;
                for j in 0..3 {
                    let b0 = c_cols[j][0];
                    let b1 = c_cols[j][1];
                    let v0_num = (u11 as i64) * b0 - (u01 as i64) * b1;
                    let v1_num = (u00 as i64) * b1 - (u10 as i64) * b0;
                    if v0_num % det01 != 0 || v1_num % det01 != 0 {
                        v_valid = false;
                        break;
                    }
                    let v0 = v0_num / det01;
                    let v1 = v1_num / det01;
                    if v0 < 0 || v0 > me_i64 || v1 < 0 || v1 > me_i64 {
                        v_valid = false;
                        break;
                    }
                    v_cols[j] = [v0, v1];
                }
                if !v_valid {
                    continue;
                }

                let v_rows: [[i64; 2]; 3] = [
                    [v_cols[0][0], v_cols[0][1]],
                    [v_cols[1][0], v_cols[1][1]],
                    [v_cols[2][0], v_cols[2][1]],
                ];
                if let Some([u20, u21]) = solve_overdetermined_3x2(&v_rows, c_row2, max_entry) {
                    if u20 + u21 >= min_row_sum[2] {
                        let u_mat = DynMatrix::new(3, 2, vec![u00, u01, u10, u11, u20, u21]);
                        let v_mat = DynMatrix::new(
                            2,
                            3,
                            vec![
                                v_cols[0][0] as u32,
                                v_cols[1][0] as u32,
                                v_cols[2][0] as u32,
                                v_cols[0][1] as u32,
                                v_cols[1][1] as u32,
                                v_cols[2][1] as u32,
                            ],
                        );
                        results.push((u_mat, v_mat));
                    }
                } else {
                    let m01 = v_rows[0][0] * v_rows[1][1] - v_rows[0][1] * v_rows[1][0];
                    let m02 = v_rows[0][0] * v_rows[2][1] - v_rows[0][1] * v_rows[2][0];
                    let m12 = v_rows[1][0] * v_rows[2][1] - v_rows[1][1] * v_rows[2][0];
                    if m01 == 0 && m02 == 0 && m12 == 0 {
                        for u20 in 0..=me {
                            for u21 in 0..=me {
                                if u20 + u21 < min_row_sum[2] {
                                    continue;
                                }
                                let mut ok = true;
                                for j in 0..3 {
                                    if (u20 as i64) * v_cols[j][0] + (u21 as i64) * v_cols[j][1]
                                        != c_cols[j][2]
                                    {
                                        ok = false;
                                        break;
                                    }
                                }
                                if ok {
                                    let u_mat =
                                        DynMatrix::new(3, 2, vec![u00, u01, u10, u11, u20, u21]);
                                    let v_mat = DynMatrix::new(
                                        2,
                                        3,
                                        vec![
                                            v_cols[0][0] as u32,
                                            v_cols[1][0] as u32,
                                            v_cols[2][0] as u32,
                                            v_cols[0][1] as u32,
                                            v_cols[1][1] as u32,
                                            v_cols[2][1] as u32,
                                        ],
                                    );
                                    results.push((u_mat, v_mat));
                                }
                            }
                        }
                    }
                }
            } else {
                for u20 in 0..=me {
                    for u21 in 0..=me {
                        if u20 + u21 < min_row_sum[2] {
                            continue;
                        }

                        let g2 = gcd(u20 as u64, u21 as u64);
                        if g2 > 1 {
                            let mut skip = false;
                            for j in 0..3 {
                                if c_cols[j][2] as u64 % g2 != 0 {
                                    skip = true;
                                    break;
                                }
                            }
                            if skip {
                                continue;
                            }
                        }

                        let u_rows: [[i64; 2]; 3] = [
                            [u00 as i64, u01 as i64],
                            [u10 as i64, u11 as i64],
                            [u20 as i64, u21 as i64],
                        ];

                        let d02 = u_rows[0][0] * u_rows[2][1] - u_rows[0][1] * u_rows[2][0];
                        let d12 = u_rows[1][0] * u_rows[2][1] - u_rows[1][1] * u_rows[2][0];
                        if d02 == 0 && d12 == 0 {
                            continue;
                        }

                        let v0 = match solve_overdetermined_3x2(&u_rows, &c_cols[0], max_entry) {
                            Some(v) => v,
                            None => continue,
                        };
                        let v1 = match solve_overdetermined_3x2(&u_rows, &c_cols[1], max_entry) {
                            Some(v) => v,
                            None => continue,
                        };
                        let v2 = match solve_overdetermined_3x2(&u_rows, &c_cols[2], max_entry) {
                            Some(v) => v,
                            None => continue,
                        };

                        let u_mat = DynMatrix::new(3, 2, vec![u00, u01, u10, u11, u20, u21]);
                        let v_mat =
                            DynMatrix::new(2, 3, vec![v0[0], v1[0], v2[0], v0[1], v1[1], v2[1]]);
                        results.push((u_mat, v_mat));
                    }
                }
            }
        }
    }

    results
}

fn binary_sparse_rows_len3() -> Vec<[u32; 3]> {
    vec![
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
        [1, 1, 0],
        [1, 0, 1],
        [0, 1, 1],
    ]
}

fn is_binary_sparse_row_len3(row: [u32; 3]) -> bool {
    matches!(
        row,
        [1, 0, 0] | [0, 1, 0] | [0, 0, 1] | [1, 1, 0] | [1, 0, 1] | [0, 1, 1]
    )
}

fn push_unique_row_len3(rows: &mut Vec<[u32; 3]>, row: [u32; 3]) {
    if !rows.contains(&row) {
        rows.push(row);
    }
}

fn weighted_sparse_rows_len3(max_entry: u32) -> Vec<[u32; 3]> {
    let binary_rows = binary_sparse_rows_len3();
    let mut rows = binary_rows.clone();

    for row in binary_rows {
        let nonzero_positions: Vec<usize> = row
            .iter()
            .enumerate()
            .filter_map(|(idx, &entry)| if entry > 0 { Some(idx) } else { None })
            .collect();

        for weight in 2..=max_entry {
            let mut scaled_row = row;
            for &idx in &nonzero_positions {
                scaled_row[idx] = weight;
            }
            push_unique_row_len3(&mut rows, scaled_row);

            for &idx in &nonzero_positions {
                let mut weighted_row = row;
                weighted_row[idx] = weight;
                push_unique_row_len3(&mut rows, weighted_row);
            }
        }
    }

    rows
}

fn binary_sparse_rows_len4() -> Vec<[u32; 4]> {
    vec![
        // Support size 1: C(4,1) = 4
        [1, 0, 0, 0],
        [0, 1, 0, 0],
        [0, 0, 1, 0],
        [0, 0, 0, 1],
        // Support size 2: C(4,2) = 6
        [1, 1, 0, 0],
        [1, 0, 1, 0],
        [1, 0, 0, 1],
        [0, 1, 1, 0],
        [0, 1, 0, 1],
        [0, 0, 1, 1],
    ]
}

fn is_binary_sparse_row_len4(row: [u32; 4]) -> bool {
    let nonzero = row.iter().filter(|&&v| v != 0).count();
    nonzero >= 1 && nonzero <= 2 && row.iter().all(|&v| v <= 1)
}

fn push_unique_row_len4(rows: &mut Vec<[u32; 4]>, row: [u32; 4]) {
    if !rows.contains(&row) {
        rows.push(row);
    }
}

fn weighted_sparse_rows_len4(max_entry: u32) -> Vec<[u32; 4]> {
    let binary_rows = binary_sparse_rows_len4();
    let mut rows = binary_rows.clone();

    for row in binary_rows {
        let nonzero_positions: Vec<usize> = row
            .iter()
            .enumerate()
            .filter_map(|(idx, &entry)| if entry > 0 { Some(idx) } else { None })
            .collect();

        for weight in 2..=max_entry {
            let mut scaled_row = row;
            for &idx in &nonzero_positions {
                scaled_row[idx] = weight;
            }
            push_unique_row_len4(&mut rows, scaled_row);

            for &idx in &nonzero_positions {
                let mut weighted_row = row;
                weighted_row[idx] = weight;
                push_unique_row_len4(&mut rows, weighted_row);
            }
        }
    }

    rows
}

/// Enumerate a narrow structured family of 4x4 -> 3x3 factorisations A = UV
/// where the 4x3 factor U has binary rows with support size 1 or 2.
fn visit_binary_sparse_factorisations_4x4_to_3<F>(a: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 4);
    assert_eq!(a.cols, 4);

    let rows = binary_sparse_rows_len3();
    let a_cols: [[i64; 3]; 4] = [
        [a.get(0, 0) as i64, a.get(1, 0) as i64, a.get(2, 0) as i64],
        [a.get(0, 1) as i64, a.get(1, 1) as i64, a.get(2, 1) as i64],
        [a.get(0, 2) as i64, a.get(1, 2) as i64, a.get(2, 2) as i64],
        [a.get(0, 3) as i64, a.get(1, 3) as i64, a.get(2, 3) as i64],
    ];
    let last_row = [
        a.get(3, 0) as i64,
        a.get(3, 1) as i64,
        a.get(3, 2) as i64,
        a.get(3, 3) as i64,
    ];

    for &row0 in &rows {
        for &row1 in &rows {
            for &row2 in &rows {
                let u_top = [
                    [row0[0] as i64, row0[1] as i64, row0[2] as i64],
                    [row1[0] as i64, row1[1] as i64, row1[2] as i64],
                    [row2[0] as i64, row2[1] as i64, row2[2] as i64],
                ];

                let det = u_top[0][0] * (u_top[1][1] * u_top[2][2] - u_top[1][2] * u_top[2][1])
                    - u_top[0][1] * (u_top[1][0] * u_top[2][2] - u_top[1][2] * u_top[2][0])
                    + u_top[0][2] * (u_top[1][0] * u_top[2][1] - u_top[1][1] * u_top[2][0]);
                if det == 0 {
                    continue;
                }

                let mut v_cols = Vec::with_capacity(4);
                let mut ok = true;
                for col in &a_cols {
                    let solutions = solve_nonneg_3x3(&u_top, col, max_entry);
                    if solutions.len() != 1 {
                        ok = false;
                        break;
                    }
                    v_cols.push(solutions[0]);
                }
                if !ok {
                    continue;
                }

                let vt = [
                    [
                        v_cols[0][0] as i64,
                        v_cols[0][1] as i64,
                        v_cols[0][2] as i64,
                    ],
                    [
                        v_cols[1][0] as i64,
                        v_cols[1][1] as i64,
                        v_cols[1][2] as i64,
                    ],
                    [
                        v_cols[2][0] as i64,
                        v_cols[2][1] as i64,
                        v_cols[2][2] as i64,
                    ],
                    [
                        v_cols[3][0] as i64,
                        v_cols[3][1] as i64,
                        v_cols[3][2] as i64,
                    ],
                ];

                let Some(row3) = solve_overdetermined_4x3(&vt, &last_row, max_entry) else {
                    continue;
                };
                if !is_binary_sparse_row_len3(row3) {
                    continue;
                }

                let u = DynMatrix::new(
                    4,
                    3,
                    vec![
                        row0[0], row0[1], row0[2], row1[0], row1[1], row1[2], row2[0], row2[1],
                        row2[2], row3[0], row3[1], row3[2],
                    ],
                );
                let v = DynMatrix::new(
                    3,
                    4,
                    vec![
                        v_cols[0][0],
                        v_cols[1][0],
                        v_cols[2][0],
                        v_cols[3][0],
                        v_cols[0][1],
                        v_cols[1][1],
                        v_cols[2][1],
                        v_cols[3][1],
                        v_cols[0][2],
                        v_cols[1][2],
                        v_cols[2][2],
                        v_cols[3][2],
                    ],
                );
                visit(u, v);
            }
        }
    }
}

/// Enumerate a narrow structured family of 3x3 -> 4x4 factorisations A = UV
/// where the 3x4 factor U has three binary-sparse columns and one distinguished
/// weighted column, and the solved 4x3 factor V stays within the same support
/// vocabulary with at most one weighted row.
fn visit_binary_sparse_factorisations_3x3_to_4<F>(a: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 3);
    assert_eq!(a.cols, 3);

    let binary_rows = binary_sparse_rows_len3();
    let weighted_rows = weighted_sparse_rows_len3(max_entry);
    let a_cols: [[i64; 3]; 3] = [
        [a.get(0, 0) as i64, a.get(1, 0) as i64, a.get(2, 0) as i64],
        [a.get(0, 1) as i64, a.get(1, 1) as i64, a.get(2, 1) as i64],
        [a.get(0, 2) as i64, a.get(1, 2) as i64, a.get(2, 2) as i64],
    ];

    for distinguished_slot in 0..4 {
        for &distinguished_col in &weighted_rows {
            for &distinguished_row in &binary_rows {
                let mut residual_cols = [[0i64; 3]; 3];
                let mut residual_valid = true;
                for col in 0..3 {
                    for row in 0..3 {
                        let residual = a_cols[col][row]
                            - (distinguished_col[row] as i64) * (distinguished_row[col] as i64);
                        if residual < 0 {
                            residual_valid = false;
                            break;
                        }
                        residual_cols[col][row] = residual;
                    }
                    if !residual_valid {
                        break;
                    }
                }
                if !residual_valid {
                    continue;
                }

                for &core_col0 in &binary_rows {
                    for &core_col1 in &binary_rows {
                        for &core_col2 in &binary_rows {
                            let core_cols = [core_col0, core_col1, core_col2];
                            let core = [
                                [
                                    core_cols[0][0] as i64,
                                    core_cols[1][0] as i64,
                                    core_cols[2][0] as i64,
                                ],
                                [
                                    core_cols[0][1] as i64,
                                    core_cols[1][1] as i64,
                                    core_cols[2][1] as i64,
                                ],
                                [
                                    core_cols[0][2] as i64,
                                    core_cols[1][2] as i64,
                                    core_cols[2][2] as i64,
                                ],
                            ];

                            let det = core[0][0]
                                * (core[1][1] * core[2][2] - core[1][2] * core[2][1])
                                - core[0][1] * (core[1][0] * core[2][2] - core[1][2] * core[2][0])
                                + core[0][2] * (core[1][0] * core[2][1] - core[1][1] * core[2][0]);
                            if det == 0 {
                                continue;
                            }

                            let mut core_row_cols = Vec::with_capacity(3);
                            let mut core_valid = true;
                            for residual_col in &residual_cols {
                                let solutions = solve_nonneg_3x3(&core, residual_col, max_entry);
                                if solutions.len() != 1 {
                                    core_valid = false;
                                    break;
                                }
                                core_row_cols.push(solutions[0]);
                            }
                            if !core_valid {
                                continue;
                            }

                            let core_rows = [
                                [
                                    core_row_cols[0][0],
                                    core_row_cols[1][0],
                                    core_row_cols[2][0],
                                ],
                                [
                                    core_row_cols[0][1],
                                    core_row_cols[1][1],
                                    core_row_cols[2][1],
                                ],
                                [
                                    core_row_cols[0][2],
                                    core_row_cols[1][2],
                                    core_row_cols[2][2],
                                ],
                            ];

                            if core_rows.iter().any(|row| !weighted_rows.contains(row)) {
                                continue;
                            }
                            if core_rows
                                .iter()
                                .filter(|&&row| !is_binary_sparse_row_len3(row))
                                .count()
                                > 1
                            {
                                continue;
                            }

                            let mut u_data = Vec::with_capacity(12);
                            for row in 0..3 {
                                let mut core_idx = 0usize;
                                for slot in 0..4 {
                                    if slot == distinguished_slot {
                                        u_data.push(distinguished_col[row]);
                                    } else {
                                        u_data.push(core_cols[core_idx][row]);
                                        core_idx += 1;
                                    }
                                }
                            }

                            let mut v_rows = [[0u32; 3]; 4];
                            let mut core_idx = 0usize;
                            for (slot, row) in v_rows.iter_mut().enumerate() {
                                if slot == distinguished_slot {
                                    *row = distinguished_row;
                                } else {
                                    *row = core_rows[core_idx];
                                    core_idx += 1;
                                }
                            }

                            let v = DynMatrix::new(
                                4,
                                3,
                                v_rows.iter().flat_map(|row| row.iter().copied()).collect(),
                            );
                            let u = DynMatrix::new(3, 4, u_data);
                            visit(u, v);
                        }
                    }
                }
            }
        }
    }
}

/// Enumerate a narrow structured family of 5x5 -> 4x4 factorisations A = UV
/// where the 5x4 factor U has binary-sparse rows with support size 1 or 2.
fn visit_binary_sparse_factorisations_5x5_to_4<F>(a: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 5);
    assert_eq!(a.cols, 5);

    let rows = binary_sparse_rows_len4();
    // A's columns as length-4 vectors (top 4 rows only), for solving V.
    let a_cols: [[i64; 4]; 5] = [
        [
            a.get(0, 0) as i64,
            a.get(1, 0) as i64,
            a.get(2, 0) as i64,
            a.get(3, 0) as i64,
        ],
        [
            a.get(0, 1) as i64,
            a.get(1, 1) as i64,
            a.get(2, 1) as i64,
            a.get(3, 1) as i64,
        ],
        [
            a.get(0, 2) as i64,
            a.get(1, 2) as i64,
            a.get(2, 2) as i64,
            a.get(3, 2) as i64,
        ],
        [
            a.get(0, 3) as i64,
            a.get(1, 3) as i64,
            a.get(2, 3) as i64,
            a.get(3, 3) as i64,
        ],
        [
            a.get(0, 4) as i64,
            a.get(1, 4) as i64,
            a.get(2, 4) as i64,
            a.get(3, 4) as i64,
        ],
    ];
    let last_row = [
        a.get(4, 0) as i64,
        a.get(4, 1) as i64,
        a.get(4, 2) as i64,
        a.get(4, 3) as i64,
        a.get(4, 4) as i64,
    ];

    for &row0 in &rows {
        for &row1 in &rows {
            for &row2 in &rows {
                for &row3 in &rows {
                    let u_top = [
                        [
                            row0[0] as i64,
                            row0[1] as i64,
                            row0[2] as i64,
                            row0[3] as i64,
                        ],
                        [
                            row1[0] as i64,
                            row1[1] as i64,
                            row1[2] as i64,
                            row1[3] as i64,
                        ],
                        [
                            row2[0] as i64,
                            row2[1] as i64,
                            row2[2] as i64,
                            row2[3] as i64,
                        ],
                        [
                            row3[0] as i64,
                            row3[1] as i64,
                            row3[2] as i64,
                            row3[3] as i64,
                        ],
                    ];

                    // Quick determinant check via cofactor expansion.
                    let m00 = det3x3(&[
                        [u_top[1][1], u_top[1][2], u_top[1][3]],
                        [u_top[2][1], u_top[2][2], u_top[2][3]],
                        [u_top[3][1], u_top[3][2], u_top[3][3]],
                    ]);
                    let m01 = det3x3(&[
                        [u_top[1][0], u_top[1][2], u_top[1][3]],
                        [u_top[2][0], u_top[2][2], u_top[2][3]],
                        [u_top[3][0], u_top[3][2], u_top[3][3]],
                    ]);
                    let m02 = det3x3(&[
                        [u_top[1][0], u_top[1][1], u_top[1][3]],
                        [u_top[2][0], u_top[2][1], u_top[2][3]],
                        [u_top[3][0], u_top[3][1], u_top[3][3]],
                    ]);
                    let m03 = det3x3(&[
                        [u_top[1][0], u_top[1][1], u_top[1][2]],
                        [u_top[2][0], u_top[2][1], u_top[2][2]],
                        [u_top[3][0], u_top[3][1], u_top[3][2]],
                    ]);
                    let det = u_top[0][0] * m00 - u_top[0][1] * m01 + u_top[0][2] * m02
                        - u_top[0][3] * m03;
                    if det == 0 {
                        continue;
                    }

                    let mut v_cols = Vec::with_capacity(5);
                    let mut ok = true;
                    for col in &a_cols {
                        let solutions = solve_nonneg_4x4(&u_top, col, max_entry);
                        if solutions.len() != 1 {
                            ok = false;
                            break;
                        }
                        v_cols.push(solutions[0]);
                    }
                    if !ok {
                        continue;
                    }

                    // Build V^T (transposed view for overdetermined solve).
                    let vt: [[i64; 4]; 5] = [
                        [
                            v_cols[0][0] as i64,
                            v_cols[0][1] as i64,
                            v_cols[0][2] as i64,
                            v_cols[0][3] as i64,
                        ],
                        [
                            v_cols[1][0] as i64,
                            v_cols[1][1] as i64,
                            v_cols[1][2] as i64,
                            v_cols[1][3] as i64,
                        ],
                        [
                            v_cols[2][0] as i64,
                            v_cols[2][1] as i64,
                            v_cols[2][2] as i64,
                            v_cols[2][3] as i64,
                        ],
                        [
                            v_cols[3][0] as i64,
                            v_cols[3][1] as i64,
                            v_cols[3][2] as i64,
                            v_cols[3][3] as i64,
                        ],
                        [
                            v_cols[4][0] as i64,
                            v_cols[4][1] as i64,
                            v_cols[4][2] as i64,
                            v_cols[4][3] as i64,
                        ],
                    ];

                    let Some(row4) = solve_overdetermined_5x4(&vt, &last_row, max_entry) else {
                        continue;
                    };
                    if !is_binary_sparse_row_len4(row4) {
                        continue;
                    }

                    let u = DynMatrix::new(
                        5,
                        4,
                        vec![
                            row0[0], row0[1], row0[2], row0[3], row1[0], row1[1], row1[2], row1[3],
                            row2[0], row2[1], row2[2], row2[3], row3[0], row3[1], row3[2], row3[3],
                            row4[0], row4[1], row4[2], row4[3],
                        ],
                    );
                    let v = DynMatrix::new(
                        4,
                        5,
                        vec![
                            v_cols[0][0],
                            v_cols[1][0],
                            v_cols[2][0],
                            v_cols[3][0],
                            v_cols[4][0],
                            v_cols[0][1],
                            v_cols[1][1],
                            v_cols[2][1],
                            v_cols[3][1],
                            v_cols[4][1],
                            v_cols[0][2],
                            v_cols[1][2],
                            v_cols[2][2],
                            v_cols[3][2],
                            v_cols[4][2],
                            v_cols[0][3],
                            v_cols[1][3],
                            v_cols[2][3],
                            v_cols[3][3],
                            v_cols[4][3],
                        ],
                    );
                    visit(u, v);
                }
            }
        }
    }
}

/// Enumerate a narrow structured family of 4x4 -> 5x5 factorisations A = UV
/// where the 4x5 factor U has four binary-sparse columns and one distinguished
/// weighted column, and the solved 5x4 factor V stays within the same support
/// vocabulary with at most one weighted row.
fn visit_binary_sparse_factorisations_4x4_to_5<F>(a: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 4);
    assert_eq!(a.cols, 4);

    let binary_rows = binary_sparse_rows_len4();
    let weighted_rows = weighted_sparse_rows_len4(max_entry);
    let a_cols: [[i64; 4]; 4] = [
        [
            a.get(0, 0) as i64,
            a.get(1, 0) as i64,
            a.get(2, 0) as i64,
            a.get(3, 0) as i64,
        ],
        [
            a.get(0, 1) as i64,
            a.get(1, 1) as i64,
            a.get(2, 1) as i64,
            a.get(3, 1) as i64,
        ],
        [
            a.get(0, 2) as i64,
            a.get(1, 2) as i64,
            a.get(2, 2) as i64,
            a.get(3, 2) as i64,
        ],
        [
            a.get(0, 3) as i64,
            a.get(1, 3) as i64,
            a.get(2, 3) as i64,
            a.get(3, 3) as i64,
        ],
    ];

    for distinguished_slot in 0..5 {
        for &distinguished_col in &weighted_rows {
            for &distinguished_row in &binary_rows {
                // Subtract the outer product of the distinguished column and row from A.
                let mut residual_cols = [[0i64; 4]; 4];
                let mut residual_valid = true;
                for col in 0..4 {
                    for row in 0..4 {
                        let residual = a_cols[col][row]
                            - (distinguished_col[row] as i64) * (distinguished_row[col] as i64);
                        if residual < 0 {
                            residual_valid = false;
                            break;
                        }
                        residual_cols[col][row] = residual;
                    }
                    if !residual_valid {
                        break;
                    }
                }
                if !residual_valid {
                    continue;
                }

                // Enumerate 4 core columns (binary-sparse, length 4).
                for &core_col0 in &binary_rows {
                    for &core_col1 in &binary_rows {
                        for &core_col2 in &binary_rows {
                            for &core_col3 in &binary_rows {
                                let core_cols = [core_col0, core_col1, core_col2, core_col3];
                                let core = [
                                    [
                                        core_cols[0][0] as i64,
                                        core_cols[1][0] as i64,
                                        core_cols[2][0] as i64,
                                        core_cols[3][0] as i64,
                                    ],
                                    [
                                        core_cols[0][1] as i64,
                                        core_cols[1][1] as i64,
                                        core_cols[2][1] as i64,
                                        core_cols[3][1] as i64,
                                    ],
                                    [
                                        core_cols[0][2] as i64,
                                        core_cols[1][2] as i64,
                                        core_cols[2][2] as i64,
                                        core_cols[3][2] as i64,
                                    ],
                                    [
                                        core_cols[0][3] as i64,
                                        core_cols[1][3] as i64,
                                        core_cols[2][3] as i64,
                                        core_cols[3][3] as i64,
                                    ],
                                ];

                                // Quick det check.
                                let m00 = det3x3(&[
                                    [core[1][1], core[1][2], core[1][3]],
                                    [core[2][1], core[2][2], core[2][3]],
                                    [core[3][1], core[3][2], core[3][3]],
                                ]);
                                let m01 = det3x3(&[
                                    [core[1][0], core[1][2], core[1][3]],
                                    [core[2][0], core[2][2], core[2][3]],
                                    [core[3][0], core[3][2], core[3][3]],
                                ]);
                                let m02 = det3x3(&[
                                    [core[1][0], core[1][1], core[1][3]],
                                    [core[2][0], core[2][1], core[2][3]],
                                    [core[3][0], core[3][1], core[3][3]],
                                ]);
                                let m03 = det3x3(&[
                                    [core[1][0], core[1][1], core[1][2]],
                                    [core[2][0], core[2][1], core[2][2]],
                                    [core[3][0], core[3][1], core[3][2]],
                                ]);
                                let det = core[0][0] * m00 - core[0][1] * m01 + core[0][2] * m02
                                    - core[0][3] * m03;
                                if det == 0 {
                                    continue;
                                }

                                let mut core_row_cols = Vec::with_capacity(4);
                                let mut core_valid = true;
                                for residual_col in &residual_cols {
                                    let solutions =
                                        solve_nonneg_4x4(&core, residual_col, max_entry);
                                    if solutions.len() != 1 {
                                        core_valid = false;
                                        break;
                                    }
                                    core_row_cols.push(solutions[0]);
                                }
                                if !core_valid {
                                    continue;
                                }

                                let core_rows = [
                                    [
                                        core_row_cols[0][0],
                                        core_row_cols[1][0],
                                        core_row_cols[2][0],
                                        core_row_cols[3][0],
                                    ],
                                    [
                                        core_row_cols[0][1],
                                        core_row_cols[1][1],
                                        core_row_cols[2][1],
                                        core_row_cols[3][1],
                                    ],
                                    [
                                        core_row_cols[0][2],
                                        core_row_cols[1][2],
                                        core_row_cols[2][2],
                                        core_row_cols[3][2],
                                    ],
                                    [
                                        core_row_cols[0][3],
                                        core_row_cols[1][3],
                                        core_row_cols[2][3],
                                        core_row_cols[3][3],
                                    ],
                                ];

                                if core_rows.iter().any(|row| !weighted_rows.contains(row)) {
                                    continue;
                                }
                                if core_rows
                                    .iter()
                                    .filter(|&&row| !is_binary_sparse_row_len4(row))
                                    .count()
                                    > 1
                                {
                                    continue;
                                }

                                // Build U (4×5): insert distinguished_col at distinguished_slot.
                                let mut u_data = Vec::with_capacity(20);
                                for row in 0..4 {
                                    let mut core_idx = 0usize;
                                    for slot in 0..5 {
                                        if slot == distinguished_slot {
                                            u_data.push(distinguished_col[row]);
                                        } else {
                                            u_data.push(core_cols[core_idx][row]);
                                            core_idx += 1;
                                        }
                                    }
                                }

                                // Build V (5×4): insert distinguished_row at distinguished_slot.
                                let mut v_rows = [[0u32; 4]; 5];
                                let mut core_idx = 0usize;
                                for (slot, row) in v_rows.iter_mut().enumerate() {
                                    if slot == distinguished_slot {
                                        *row = distinguished_row;
                                    } else {
                                        *row = core_rows[core_idx];
                                        core_idx += 1;
                                    }
                                }

                                let v = DynMatrix::new(
                                    5,
                                    4,
                                    v_rows.iter().flat_map(|row| row.iter().copied()).collect(),
                                );
                                let u = DynMatrix::new(4, 5, u_data);
                                visit(u, v);
                            }
                        }
                    }
                }
            }
        }
    }
}

// --- Elementary conjugation moves for 3×3 ---

/// Generate 3x3 -> 3x3 elementary SSE steps via conjugation by
/// P = I + k*e_i*e_j^T (an elementary matrix with det=1).
///
/// Row-operation direction: U = P, V = P^{-1}*C (subtract k*row_j from row_i).
///   UV = C, VU = P^{-1}*C*P (the new node).
///
/// Column-operation direction: V = P, U = C*P^{-1} (subtract k*col_i from col_j).
///   UV = C, VU = P*C*P^{-1} (the new node).
fn visit_elementary_conjugations_3x3<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let me = max_entry as i64;
    let n = 3usize;

    let mut cm = [[0i64; 3]; 3];
    for i in 0..n {
        for j in 0..n {
            cm[i][j] = c.get(i, j) as i64;
        }
    }

    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }

            // --- Row-operation direction: U = P = I + k*e_i*e_j^T ---
            // V = P^{-1}*C = C with row_i -= k*row_j.
            // Nonneg iff row_i(C) >= k*row_j(C) entrywise.
            let max_k_row = if cm[j].iter().all(|&v| v == 0) {
                0 // row_j zero -> V=C, trivial
            } else {
                let mut mk = i64::MAX;
                for col in 0..n {
                    if cm[j][col] > 0 {
                        mk = mk.min(cm[i][col] / cm[j][col]);
                    }
                }
                mk.min(me)
            };

            for k in 1..=max_k_row {
                if k > me {
                    break;
                }
                let mut v_arr = cm;
                for col in 0..n {
                    v_arr[i][col] = cm[i][col] - k * cm[j][col];
                }
                // V entries are nonneg by construction; check max_entry.
                if v_arr[i].iter().any(|&e| e > me) {
                    continue;
                }
                // U = P = I + k*e_i*e_j^T
                let mut u_arr = [[0i64; 3]; 3];
                for d in 0..n {
                    u_arr[d][d] = 1;
                }
                u_arr[i][j] = k;

                let u_mat = mat3_i64_to_dyn(&u_arr);
                let v_mat = mat3_i64_to_dyn(&v_arr);
                visit(u_mat, v_mat);
            }

            // --- Column-operation direction: V = P = I + k*e_i*e_j^T ---
            // U = C*P^{-1} = C with col_j -= k*col_i.
            // Nonneg iff col_j(C) >= k*col_i(C) entrywise.
            let max_k_col = {
                let mut mk = i64::MAX;
                let mut any_pos = false;
                for row in 0..n {
                    if cm[row][i] > 0 {
                        any_pos = true;
                        mk = mk.min(cm[row][j] / cm[row][i]);
                    }
                }
                if !any_pos {
                    0
                } else {
                    mk.min(me)
                }
            };

            for k in 1..=max_k_col {
                if k > me {
                    break;
                }
                let mut u_arr = cm;
                for row in 0..n {
                    u_arr[row][j] = cm[row][j] - k * cm[row][i];
                }
                if u_arr.iter().flat_map(|r| r.iter()).any(|&e| e > me) {
                    continue;
                }
                // V = P = I + k*e_i*e_j^T
                let mut v_arr = [[0i64; 3]; 3];
                for d in 0..n {
                    v_arr[d][d] = 1;
                }
                v_arr[i][j] = k;

                let u_mat = mat3_i64_to_dyn(&u_arr);
                let v_mat = mat3_i64_to_dyn(&v_arr);
                visit(u_mat, v_mat);
            }
        }
    }
}

/// Generate 3x3 -> 3x3 elementary SSE steps via conjugation by
/// P = (I + k*e_i*e_j^T)(I + l*e_j*e_i^T) for ordered pairs i != j.
///
/// These opposite-shear products stay unimodular but can realize diagonal
/// weights above the capped square-factorisation bound, while remaining much
/// narrower than a general diagonal-conjugation sweep.
fn visit_opposite_shear_conjugations_3x3<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let me = max_entry as i64;
    let sq3_cap = max_entry.min(4) as i64;
    let mut cm = [[0i64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            cm[i][j] = c.get(i, j) as i64;
        }
    }

    for i in 0..3 {
        for j in 0..3 {
            if i == j {
                continue;
            }

            for k in 1..=max_entry {
                for l in 1..=max_entry {
                    let (k, l) = (k as i64, l as i64);
                    let boosted_diag = 1 + k * l;
                    if boosted_diag > me {
                        continue;
                    }

                    let mut p = identity_mat3_i64();
                    p[i][i] = boosted_diag;
                    p[i][j] = k;
                    p[j][i] = l;

                    if max_entry_mat3_i64(&p) <= sq3_cap {
                        continue;
                    }

                    let mut pinv = identity_mat3_i64();
                    pinv[i][j] = -k;
                    pinv[j][i] = -l;
                    pinv[j][j] = boosted_diag;

                    let row_v = mul_mat3_i64(&pinv, &cm);
                    if entries_fit_nonnegative_bound_mat3_i64(&row_v, me) {
                        visit(mat3_i64_to_dyn(&p), mat3_i64_to_dyn(&row_v));
                    }

                    let col_u = mul_mat3_i64(&cm, &pinv);
                    if entries_fit_nonnegative_bound_mat3_i64(&col_u, me) {
                        visit(mat3_i64_to_dyn(&col_u), mat3_i64_to_dyn(&p));
                    }
                }
            }
        }
    }
}

/// Generate 3x3 -> 3x3 elementary SSE steps via paired shears sharing a pivot:
/// P = I + a*e_i*e_j^T + b*e_i*e_k^T for distinct i, j, k.
///
/// Because the two nilpotent terms have the same source row, they square to
/// zero and commute, so P^{-1} = I - a*e_i*e_j^T - b*e_i*e_k^T.
///
/// This reaches a structured family of "subtract two rows/columns at once"
/// moves that the capped square 3x3 enumeration only sees when all entries of P
/// stay within the cap.
fn visit_parallel_shear_conjugations_3x3<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let me = max_entry as i64;
    let sq3_cap = max_entry.min(4) as i64;
    let mut cm = [[0i64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            cm[i][j] = c.get(i, j) as i64;
        }
    }

    for pivot in 0..3 {
        let support: Vec<usize> = (0..3).filter(|&idx| idx != pivot).collect();
        let j = support[0];
        let k = support[1];

        for a in 1..=max_entry {
            for b in 1..=max_entry {
                let (a, b) = (a as i64, b as i64);

                let mut p = identity_mat3_i64();
                p[pivot][j] = a;
                p[pivot][k] = b;
                if max_entry_mat3_i64(&p) <= sq3_cap {
                    continue;
                }

                let mut pinv = identity_mat3_i64();
                pinv[pivot][j] = -a;
                pinv[pivot][k] = -b;

                let row_v = mul_mat3_i64(&pinv, &cm);
                if entries_fit_nonnegative_bound_mat3_i64(&row_v, me) {
                    visit(mat3_i64_to_dyn(&p), mat3_i64_to_dyn(&row_v));
                }

                let col_u = mul_mat3_i64(&cm, &pinv);
                if entries_fit_nonnegative_bound_mat3_i64(&col_u, me) {
                    visit(mat3_i64_to_dyn(&col_u), mat3_i64_to_dyn(&p));
                }
            }
        }
    }
}

/// Generate 3x3 -> 3x3 elementary SSE steps via paired shears sharing a
/// common target:
/// P = I + a*e_j*e_i^T + b*e_k*e_i^T for distinct i, j, k.
///
/// As in the parallel-source case, the nilpotent terms square to zero and
/// commute, so P^{-1} = I - a*e_j*e_i^T - b*e_k*e_i^T.
fn visit_convergent_shear_conjugations_3x3<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let me = max_entry as i64;
    let sq3_cap = max_entry.min(4) as i64;
    let mut cm = [[0i64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            cm[i][j] = c.get(i, j) as i64;
        }
    }

    for target in 0..3 {
        let support: Vec<usize> = (0..3).filter(|&idx| idx != target).collect();
        let j = support[0];
        let k = support[1];

        for a in 1..=max_entry {
            for b in 1..=max_entry {
                let (a, b) = (a as i64, b as i64);

                let mut p = identity_mat3_i64();
                p[j][target] = a;
                p[k][target] = b;
                if max_entry_mat3_i64(&p) <= sq3_cap {
                    continue;
                }

                let mut pinv = identity_mat3_i64();
                pinv[j][target] = -a;
                pinv[k][target] = -b;

                let row_v = mul_mat3_i64(&pinv, &cm);
                if entries_fit_nonnegative_bound_mat3_i64(&row_v, me) {
                    visit(mat3_i64_to_dyn(&p), mat3_i64_to_dyn(&row_v));
                }

                let col_u = mul_mat3_i64(&cm, &pinv);
                if entries_fit_nonnegative_bound_mat3_i64(&col_u, me) {
                    visit(mat3_i64_to_dyn(&col_u), mat3_i64_to_dyn(&p));
                }
            }
        }
    }
}

fn identity_mat3_i64() -> [[i64; 3]; 3] {
    let mut m = [[0i64; 3]; 3];
    for idx in 0..3 {
        m[idx][idx] = 1;
    }
    m
}

fn mul_mat3_i64(a: &[[i64; 3]; 3], b: &[[i64; 3]; 3]) -> [[i64; 3]; 3] {
    let mut out = [[0i64; 3]; 3];
    for i in 0..3 {
        for k in 0..3 {
            if a[i][k] == 0 {
                continue;
            }
            for j in 0..3 {
                out[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    out
}

fn entries_fit_nonnegative_bound_mat3_i64(m: &[[i64; 3]; 3], max_entry: i64) -> bool {
    m.iter()
        .flat_map(|row| row.iter())
        .all(|&entry| (0..=max_entry).contains(&entry))
}

fn max_entry_mat3_i64(m: &[[i64; 3]; 3]) -> i64 {
    m.iter()
        .flat_map(|row| row.iter())
        .copied()
        .max()
        .unwrap_or(0)
}

fn mat3_i64_to_dyn(m: &[[i64; 3]; 3]) -> DynMatrix {
    let data: Vec<u32> = m.iter().flat_map(|r| r.iter()).map(|&e| e as u32).collect();
    DynMatrix::new(3, 3, data)
}

// --- Square 3×3 factorisation ---

/// Solve A·x = b where A is 3×3 (given as rows), for nonneg integer x with
/// entries ≤ max_entry. If the system is full-rank, there is at most one solution.
/// If rank-2, reduces to solve_nonneg_2x3 + verification of the remaining equation.
fn solve_nonneg_3x3(a: &[[i64; 3]; 3], b: &[i64; 3], max_entry: u32) -> Vec<[u32; 3]> {
    let me = max_entry as i64;

    // Compute determinant via cofactor expansion along row 0.
    let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);

    if det != 0 {
        // Unique solution: x = adj(A)·b / det.
        let adj = [
            [
                a[1][1] * a[2][2] - a[1][2] * a[2][1],
                a[0][2] * a[2][1] - a[0][1] * a[2][2],
                a[0][1] * a[1][2] - a[0][2] * a[1][1],
            ],
            [
                a[1][2] * a[2][0] - a[1][0] * a[2][2],
                a[0][0] * a[2][2] - a[0][2] * a[2][0],
                a[0][2] * a[1][0] - a[0][0] * a[1][2],
            ],
            [
                a[1][0] * a[2][1] - a[1][1] * a[2][0],
                a[0][1] * a[2][0] - a[0][0] * a[2][1],
                a[0][0] * a[1][1] - a[0][1] * a[1][0],
            ],
        ];
        let mut x = [0i64; 3];
        for i in 0..3 {
            let num = adj[i][0] * b[0] + adj[i][1] * b[1] + adj[i][2] * b[2];
            if num % det != 0 {
                return vec![];
            }
            x[i] = num / det;
            if x[i] < 0 || x[i] > me {
                return vec![];
            }
        }
        return vec![[x[0] as u32, x[1] as u32, x[2] as u32]];
    }

    // det = 0: find a rank-2 row pair and reduce to solve_nonneg_2x3.
    for &(r0, r1, r_check) in &[(0, 1, 2), (0, 2, 1), (1, 2, 0)] {
        let rows: [[i64; 3]; 2] = [a[r0], a[r1]];
        let d01 = rows[0][0] * rows[1][1] - rows[0][1] * rows[1][0];
        let d02 = rows[0][0] * rows[1][2] - rows[0][2] * rows[1][0];
        let d12 = rows[0][1] * rows[1][2] - rows[0][2] * rows[1][1];
        if d01 == 0 && d02 == 0 && d12 == 0 {
            continue;
        }

        let b_sub = [b[r0], b[r1]];
        let solutions = solve_nonneg_2x3(&rows, &b_sub, max_entry);
        let mut results = Vec::new();
        for x in solutions {
            let check = a[r_check][0] * x[0] as i64
                + a[r_check][1] * x[1] as i64
                + a[r_check][2] * x[2] as i64;
            if check == b[r_check] {
                results.push(x);
            }
        }
        return results;
    }

    // Rank ≤ 1: skip (degenerate, rarely contributes useful factorisations).
    vec![]
}

/// Compute the determinant of a 3×3 matrix given as rows.
fn det3x3(m: &[[i64; 3]; 3]) -> i64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Solve U·x = b where U is 4×4 (given as rows), b is 4-vector.
/// Returns all nonneg integer 4-vectors x with entries ≤ max_entry.
///
/// Algorithm: cofactor expansion for the determinant and adjugate.
/// Falls back to rank-3 reduction via `solve_nonneg_3x3` when singular.
fn solve_nonneg_4x4(a: &[[i64; 4]; 4], b: &[i64; 4], max_entry: u32) -> Vec<[u32; 4]> {
    let me = max_entry as i64;

    // 3×3 minor: delete row r and column c from the 4×4 matrix.
    let minor = |r: usize, c: usize| -> [[i64; 3]; 3] {
        let mut m = [[0i64; 3]; 3];
        let mut mi = 0;
        for i in 0..4 {
            if i == r {
                continue;
            }
            let mut mj = 0;
            for j in 0..4 {
                if j == c {
                    continue;
                }
                m[mi][mj] = a[i][j];
                mj += 1;
            }
            mi += 1;
        }
        m
    };

    // Determinant via cofactor expansion along row 0.
    let c00 = det3x3(&minor(0, 0));
    let c01 = -det3x3(&minor(0, 1));
    let c02 = det3x3(&minor(0, 2));
    let c03 = -det3x3(&minor(0, 3));
    let det = a[0][0] * c00 + a[0][1] * c01 + a[0][2] * c02 + a[0][3] * c03;

    if det != 0 {
        // Compute full adjugate (transpose of cofactor matrix).
        let cofactors: [[i64; 4]; 4] = [
            [c00, c01, c02, c03],
            [
                -det3x3(&minor(1, 0)),
                det3x3(&minor(1, 1)),
                -det3x3(&minor(1, 2)),
                det3x3(&minor(1, 3)),
            ],
            [
                det3x3(&minor(2, 0)),
                -det3x3(&minor(2, 1)),
                det3x3(&minor(2, 2)),
                -det3x3(&minor(2, 3)),
            ],
            [
                -det3x3(&minor(3, 0)),
                det3x3(&minor(3, 1)),
                -det3x3(&minor(3, 2)),
                det3x3(&minor(3, 3)),
            ],
        ];

        // x = adj(A)·b / det, where adj = cofactors^T.
        let mut x = [0i64; 4];
        for i in 0..4 {
            let num = cofactors[0][i] * b[0]
                + cofactors[1][i] * b[1]
                + cofactors[2][i] * b[2]
                + cofactors[3][i] * b[3];
            if num % det != 0 {
                return vec![];
            }
            x[i] = num / det;
            if x[i] < 0 || x[i] > me {
                return vec![];
            }
        }
        return vec![[x[0] as u32, x[1] as u32, x[2] as u32, x[3] as u32]];
    }

    // det = 0: find a rank-3 triple of rows and reduce to solve_nonneg_3x3.
    let triples = [
        (0usize, 1usize, 2usize, 3usize),
        (0, 1, 3, 2),
        (0, 2, 3, 1),
        (1, 2, 3, 0),
    ];
    for &(r0, r1, r2, r_check) in &triples {
        // Extract the 3×4 submatrix and try each 3×3 column subset.
        let sub = [a[r0], a[r1], a[r2]];
        let b_sub = [b[r0], b[r1], b[r2]];

        // Try each of the 4 possible 3×3 column subsets.
        for free_col in 0..4 {
            let cols: Vec<usize> = (0..4).filter(|&c| c != free_col).collect();
            let system = [
                [sub[0][cols[0]], sub[0][cols[1]], sub[0][cols[2]]],
                [sub[1][cols[0]], sub[1][cols[1]], sub[1][cols[2]]],
                [sub[2][cols[0]], sub[2][cols[1]], sub[2][cols[2]]],
            ];
            let d = det3x3(&system);
            if d == 0 {
                continue;
            }

            // Enumerate free_col values 0..=max_entry.
            for fv in 0..=max_entry {
                let fv = fv as i64;
                let rhs = [
                    b_sub[0] - sub[0][free_col] * fv,
                    b_sub[1] - sub[1][free_col] * fv,
                    b_sub[2] - sub[2][free_col] * fv,
                ];
                let solutions = solve_nonneg_3x3(&system, &rhs, max_entry);
                for sol in solutions {
                    // Reassemble full 4-vector.
                    let mut x = [0u32; 4];
                    x[free_col] = fv as u32;
                    for (si, &ci) in cols.iter().enumerate() {
                        x[ci] = sol[si];
                    }
                    // Verify the check row.
                    let check = a[r_check][0] * x[0] as i64
                        + a[r_check][1] * x[1] as i64
                        + a[r_check][2] * x[2] as i64
                        + a[r_check][3] * x[3] as i64;
                    if check == b[r_check] {
                        return vec![x];
                    }
                }
            }
            // Found a non-singular 3x3 subset; no need to try others.
            return vec![];
        }
    }

    // Rank ≤ 2: skip.
    vec![]
}

/// Solve U·x = b where U is 5×4 (given as rows), b is 5-vector.
/// Returns the unique nonneg integer 4-vector x with entries ≤ max_entry, if it exists.
fn solve_overdetermined_5x4(u: &[[i64; 4]; 5], b: &[i64; 5], max_entry: u32) -> Option<[u32; 4]> {
    // Try each of the C(5,4)=5 subsets of 4 rows.
    let subsets: [(usize, usize, usize, usize, usize); 5] = [
        (0, 1, 2, 3, 4),
        (0, 1, 2, 4, 3),
        (0, 1, 3, 4, 2),
        (0, 2, 3, 4, 1),
        (1, 2, 3, 4, 0),
    ];

    for &(r0, r1, r2, r3, r_check) in &subsets {
        let system = [u[r0], u[r1], u[r2], u[r3]];
        let rhs = [b[r0], b[r1], b[r2], b[r3]];
        let solutions = solve_nonneg_4x4(&system, &rhs, max_entry);
        if solutions.len() != 1 {
            continue;
        }
        let x = solutions[0];
        let check = u[r_check][0] * x[0] as i64
            + u[r_check][1] * x[1] as i64
            + u[r_check][2] * x[2] as i64
            + u[r_check][3] * x[3] as i64;
        if check == b[r_check] {
            return Some(x);
        }
    }

    None
}

/// Enumerate all square 3×3 nonneg integer factorisations C = UV where U and V
/// are both 3×3 with entries in 0..=max_entry.
///
/// Algorithm: enumerate rows 0 and 1 of U, solve for each column of V using
/// `solve_nonneg_2x3`, then derive row 2 of U via `solve_nonneg_3x3`.
fn visit_square_factorisations_3x3<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let me = max_entry;

    // C as columns: c_cols[j][i] = C[i,j].
    let c_cols: [[i64; 3]; 3] = [
        [c.get(0, 0) as i64, c.get(1, 0) as i64, c.get(2, 0) as i64],
        [c.get(0, 1) as i64, c.get(1, 1) as i64, c.get(2, 1) as i64],
        [c.get(0, 2) as i64, c.get(1, 2) as i64, c.get(2, 2) as i64],
    ];

    let c_row2 = [c.get(2, 0) as i64, c.get(2, 1) as i64, c.get(2, 2) as i64];

    // Minimum row sum: row_sum(U_i) >= ceil(max(C[i,*]) / max_entry).
    let min_row_sum: [u32; 3] = [
        {
            let mx = c.get(0, 0).max(c.get(0, 1)).max(c.get(0, 2)) as u64;
            ((mx + me as u64 - 1) / me as u64) as u32
        },
        {
            let mx = c.get(1, 0).max(c.get(1, 1)).max(c.get(1, 2)) as u64;
            ((mx + me as u64 - 1) / me as u64) as u32
        },
        {
            let mx = c.get(2, 0).max(c.get(2, 1)).max(c.get(2, 2)) as u64;
            ((mx + me as u64 - 1) / me as u64) as u32
        },
    ];

    // Collect valid row-0 candidates with early GCD pruning.
    let mut valid_row0s = Vec::new();
    for u00 in 0..=me {
        for u01 in 0..=me {
            for u02 in 0..=me {
                if u00 + u01 + u02 < min_row_sum[0] {
                    continue;
                }
                let g = gcd3(u00 as u64, u01 as u64, u02 as u64);
                if g > 1 {
                    let mut skip = false;
                    for j in 0..3 {
                        if c_cols[j][0] as u64 % g != 0 {
                            skip = true;
                            break;
                        }
                    }
                    if skip {
                        continue;
                    }
                }
                valid_row0s.push([u00, u01, u02]);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let per_row0: Vec<Vec<(DynMatrix, DynMatrix)>> = valid_row0s
            .par_iter()
            .map(|&row0| enumerate_sq3_from_row0(row0, &c_cols, &c_row2, max_entry, &min_row_sum))
            .collect();
        for row_results in per_row0 {
            for (u, v) in row_results {
                visit(u, v);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        for row0 in valid_row0s {
            for (u, v) in enumerate_sq3_from_row0(row0, &c_cols, &c_row2, max_entry, &min_row_sum) {
                visit(u, v);
            }
        }
    }
}

fn enumerate_sq3_from_row0(
    row0: [u32; 3],
    c_cols: &[[i64; 3]; 3],
    c_row2: &[i64; 3],
    max_entry: u32,
    min_row_sum: &[u32; 3],
) -> Vec<(DynMatrix, DynMatrix)> {
    let [u00, u01, u02] = row0;
    let me = max_entry;
    let mut results = Vec::new();

    for u10 in 0..=me {
        for u11 in 0..=me {
            for u12 in 0..=me {
                if u10 + u11 + u12 < min_row_sum[1] {
                    continue;
                }

                let g1 = gcd3(u10 as u64, u11 as u64, u12 as u64);
                if g1 > 1 {
                    let mut skip = false;
                    for j in 0..3 {
                        if c_cols[j][1] as u64 % g1 != 0 {
                            skip = true;
                            break;
                        }
                    }
                    if skip {
                        continue;
                    }
                }

                let u_top: [[i64; 3]; 2] = [
                    [u00 as i64, u01 as i64, u02 as i64],
                    [u10 as i64, u11 as i64, u12 as i64],
                ];

                // Solve for each column of V: u_top · v_j = [C[0,j], C[1,j]].
                let v_col0 = solve_nonneg_2x3(&u_top, &[c_cols[0][0], c_cols[0][1]], max_entry);
                if v_col0.is_empty() {
                    continue;
                }

                let v_col1 = solve_nonneg_2x3(&u_top, &[c_cols[1][0], c_cols[1][1]], max_entry);
                if v_col1.is_empty() {
                    continue;
                }

                let v_col2 = solve_nonneg_2x3(&u_top, &[c_cols[2][0], c_cols[2][1]], max_entry);
                if v_col2.is_empty() {
                    continue;
                }

                // For each combination of column solutions, derive row 2 of U.
                for vc0 in &v_col0 {
                    for vc1 in &v_col1 {
                        for vc2 in &v_col2 {
                            // V^T rows = V columns. System: V^T · u2^T = c_row2^T.
                            let vt: [[i64; 3]; 3] = [
                                [vc0[0] as i64, vc0[1] as i64, vc0[2] as i64],
                                [vc1[0] as i64, vc1[1] as i64, vc1[2] as i64],
                                [vc2[0] as i64, vc2[1] as i64, vc2[2] as i64],
                            ];

                            let row2_solutions = solve_nonneg_3x3(&vt, c_row2, max_entry);

                            for [u20, u21, u22] in row2_solutions {
                                if u20 + u21 + u22 < min_row_sum[2] {
                                    continue;
                                }

                                let u_mat = DynMatrix::new(
                                    3,
                                    3,
                                    vec![u00, u01, u02, u10, u11, u12, u20, u21, u22],
                                );
                                let v_mat = DynMatrix::new(
                                    3,
                                    3,
                                    vec![
                                        vc0[0], vc1[0], vc2[0], vc0[1], vc1[1], vc2[1], vc0[2],
                                        vc1[2], vc2[2],
                                    ],
                                );
                                results.push((u_mat, v_mat));
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

/// Unified factorisation dispatcher for any square matrix (given as DynMatrix).
/// Enumerates factorisations A = UV for intermediate dimensions m = 2, ..., max_intermediate_dim.
/// For k×k input:
///   - k=2, m=2: square factorisations
///   - k=2, m=3: rectangular 2×3 × 3×2
// --- Dimension-generic elementary conjugations ---

/// Dimension-generic elementary conjugations: P = I + k·e_i·e_j^T.
///
/// Works for any n×n DynMatrix. For each ordered pair (i,j) with i≠j,
/// tries both row-operation (U=P, V=P⁻¹C) and column-operation (U=CP⁻¹, V=P)
/// directions.
fn visit_elementary_conjugations_generic<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert!(c.is_square());
    let n = c.rows;
    let me = max_entry as i64;

    // Load matrix into i64 vec-of-vecs for arithmetic.
    let cm: Vec<Vec<i64>> = (0..n)
        .map(|i| (0..n).map(|j| c.get(i, j) as i64).collect())
        .collect();

    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }

            // --- Row-operation direction: U = P = I + k*e_i*e_j^T ---
            // V = P^{-1}*C = C with row_i -= k*row_j.
            // Nonneg iff row_i(C) >= k*row_j(C) entrywise.
            let max_k_row = if cm[j].iter().all(|&v| v == 0) {
                0
            } else {
                let mut mk = i64::MAX;
                for col in 0..n {
                    if cm[j][col] > 0 {
                        mk = mk.min(cm[i][col] / cm[j][col]);
                    }
                }
                mk.min(me)
            };

            for k in 1..=max_k_row {
                if k > me {
                    break;
                }
                // V = C with row_i -= k*row_j
                let mut v_data = Vec::with_capacity(n * n);
                for r in 0..n {
                    for col in 0..n {
                        let val = if r == i {
                            cm[i][col] - k * cm[j][col]
                        } else {
                            cm[r][col]
                        };
                        if val < 0 || val > me {
                            v_data.clear();
                            break;
                        }
                        v_data.push(val as u32);
                    }
                    if v_data.len() != (r + 1) * n {
                        break;
                    }
                }
                if v_data.len() != n * n {
                    continue;
                }

                // U = P = I + k*e_i*e_j^T
                let mut u_data = vec![0u32; n * n];
                for d in 0..n {
                    u_data[d * n + d] = 1;
                }
                u_data[i * n + j] = k as u32;

                visit(DynMatrix::new(n, n, u_data), DynMatrix::new(n, n, v_data));
            }

            // --- Column-operation direction: V = P = I + k*e_i*e_j^T ---
            // U = C*P^{-1} = C with col_j -= k*col_i.
            // Nonneg iff col_j(C) >= k*col_i(C) entrywise.
            let max_k_col = {
                let mut mk = i64::MAX;
                let mut any_pos = false;
                for row in 0..n {
                    if cm[row][i] > 0 {
                        any_pos = true;
                        mk = mk.min(cm[row][j] / cm[row][i]);
                    }
                }
                if !any_pos {
                    0
                } else {
                    mk.min(me)
                }
            };

            for k in 1..=max_k_col {
                if k > me {
                    break;
                }
                // U = C with col_j -= k*col_i
                let mut u_data = Vec::with_capacity(n * n);
                for row in 0..n {
                    for col in 0..n {
                        let val = if col == j {
                            cm[row][j] - k * cm[row][i]
                        } else {
                            cm[row][col]
                        };
                        if val < 0 || val > me {
                            u_data.clear();
                            break;
                        }
                        u_data.push(val as u32);
                    }
                    if u_data.len() != (row + 1) * n {
                        break;
                    }
                }
                if u_data.len() != n * n {
                    continue;
                }

                // V = P = I + k*e_i*e_j^T
                let mut v_data = vec![0u32; n * n];
                for d in 0..n {
                    v_data[d * n + d] = 1;
                }
                v_data[i * n + j] = k as u32;

                visit(DynMatrix::new(n, n, u_data), DynMatrix::new(n, n, v_data));
            }
        }
    }
}

///   - k=3, m=2: rectangular 3×2 × 2×3 (the return trip)
///   - k=3, m=3: square 3×3 factorisations
pub fn enumerate_all_factorisations(
    a: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    let mut results = Vec::new();
    visit_all_factorisations(a, max_intermediate_dim, max_entry, |u, v| {
        results.push((u, v));
    });
    results
}

pub fn visit_all_factorisations<F>(
    a: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
    mut visit: F,
) where
    F: FnMut(DynMatrix, DynMatrix),
{
    visit_all_factorisations_with_family(a, max_intermediate_dim, max_entry, |_family, u, v| {
        visit(u, v);
    });
}

pub fn visit_all_factorisations_with_family<F>(
    a: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
    mut visit: F,
) where
    F: FnMut(&'static str, DynMatrix, DynMatrix),
{
    assert!(a.is_square());
    let k = a.rows;

    if k == 2 {
        // Square factorisations (m=2).
        let sq: SqMatrix<2> = a.to_sq().unwrap();
        visit_square_factorisations_2x2(&sq, max_entry, &mut |u, v| {
            visit("square_factorisation_2x2", u, v);
        });

        // Rectangular factorisations for m=3..=max_intermediate_dim.
        if max_intermediate_dim >= 3 {
            visit_rect_factorisations_2x3(&sq, max_entry, &mut |u, v| {
                visit("rectangular_factorisation_2x3", u, v);
            });
        }
    } else if k == 3 {
        // Rectangular 3×2 × 2×3 factorisations (the return trip to 2×2).
        visit_factorisations_3x3_to_2(a, max_entry, &mut |u, v| {
            visit("rectangular_factorisation_3x3_to_2", u, v);
        });
        if max_intermediate_dim >= 4 {
            visit_binary_sparse_factorisations_3x3_to_4(a, max_entry, &mut |u, v| {
                visit("binary_sparse_rectangular_factorisation_3x3_to_4", u, v);
            });
        }
        // Square 3×3 factorisations (allows chaining through 3×3 space).
        // Factor entry bound is capped to keep enumeration tractable: the cost
        // is O((cap+1)^6) per node, so cap=4 gives ~15K iterations per node.
        if max_intermediate_dim >= 3 {
            let sq3_cap = max_entry.min(4);
            visit_square_factorisations_3x3(a, sq3_cap, &mut |u, v| {
                visit("square_factorisation_3x3", u, v);
            });
            // Elementary conjugation moves C = P·(P⁻¹C), where P = I ± k·eᵢeⱼᵀ.
            // These are O(1) per move and reach 3×3 nodes that the capped
            // square enumeration misses (factor entries > cap).
            visit_elementary_conjugations_3x3(a, max_entry, &mut |u, v| {
                visit("elementary_conjugation_3x3", u, v);
            });
            visit_opposite_shear_conjugations_3x3(a, max_entry, &mut |u, v| {
                visit("opposite_shear_conjugation_3x3", u, v);
            });
            visit_parallel_shear_conjugations_3x3(a, max_entry, &mut |u, v| {
                visit("parallel_shear_conjugation_3x3", u, v);
            });
            visit_convergent_shear_conjugations_3x3(a, max_entry, &mut |u, v| {
                visit("convergent_shear_conjugation_3x3", u, v);
            });
        }
    } else if k >= 4 {
        if k == 4 && max_intermediate_dim >= 4 {
            visit_binary_sparse_factorisations_4x4_to_3(a, max_entry, &mut |u, v| {
                visit("binary_sparse_rectangular_factorisation_4x3_to_3", u, v);
            });
        }
        if k == 4 && max_intermediate_dim >= 5 {
            visit_binary_sparse_factorisations_4x4_to_5(a, max_entry, &mut |u, v| {
                visit("binary_sparse_rectangular_factorisation_4x4_to_5", u, v);
            });
        }
        if k == 5 && max_intermediate_dim >= 5 {
            visit_binary_sparse_factorisations_5x5_to_4(a, max_entry, &mut |u, v| {
                visit("binary_sparse_rectangular_factorisation_5x5_to_4", u, v);
            });
        }
        // For dimensions ≥ 4: elementary conjugations (same-dimension moves).
        // These use the dimension-generic implementation.
        if max_intermediate_dim >= k {
            visit_elementary_conjugations_generic(a, max_entry, &mut |u, v| {
                visit("elementary_conjugation", u, v);
            });
        }
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn gcd3(a: u64, b: u64, c: u64) -> u64 {
    gcd(gcd(a, b), c)
}

/// Integer division rounding towards negative infinity.
fn div_floor(a: i64, b: i64) -> i64 {
    let d = a / b;
    let r = a % b;
    if (r != 0) && ((r ^ b) < 0) {
        d - 1
    } else {
        d
    }
}

/// Integer division rounding towards positive infinity.
fn div_ceil(a: i64, b: i64) -> i64 {
    let d = a / b;
    let r = a % b;
    if (r != 0) && ((r ^ b) > 0) {
        d + 1
    } else {
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factorisations_identity() {
        let id = SqMatrix::new([[1, 0], [0, 1]]);
        let facts = enumerate_square_factorisations_2x2(&id, 5);
        // Every factorisation should satisfy UV = I
        for (u, v) in &facts {
            let uv = u.mul(v);
            assert_eq!(uv, DynMatrix::from_sq(&id));
        }
        // Identity = UV means V = U^{-1}, so U must be a nonneg integer matrix
        // with nonneg integer inverse. These are exactly the permutation matrices
        // and the matrices [[1,k],[0,1]], [[1,0],[k,1]] with k >= 0, plus
        // their permutation conjugates. With max_entry=5, there should be many.
        assert!(!facts.is_empty());
    }

    #[test]
    fn test_factorisations_verify_product() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let facts = enumerate_square_factorisations_2x2(&a, 5);
        assert!(!facts.is_empty());
        for (u, v) in &facts {
            let uv = u.mul(v);
            assert_eq!(
                uv,
                DynMatrix::from_sq(&a),
                "UV != A for U={:?}, V={:?}",
                u,
                v
            );
        }
    }

    #[test]
    fn test_known_factorisation() {
        // [[2,1],[1,1]] = [[1,1],[0,1]] * [[1,0],[1,1]]
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let facts = enumerate_square_factorisations_2x2(&a, 5);
        let u_expected = DynMatrix::new(2, 2, vec![1, 1, 0, 1]);
        let v_expected = DynMatrix::new(2, 2, vec![1, 0, 1, 1]);
        assert!(
            facts.contains(&(u_expected, v_expected)),
            "Expected factorisation not found"
        );
    }

    #[test]
    fn test_vu_product() {
        let u = DynMatrix::new(2, 2, vec![1, 1, 0, 1]);
        let v = DynMatrix::new(2, 2, vec![1, 0, 1, 1]);
        let vu = vu_product_2x2(&v, &u);
        assert_eq!(vu, SqMatrix::new([[1, 1], [1, 2]]));
    }

    // --- Solver tests ---

    #[test]
    fn test_solve_nonneg_2x3_basic() {
        // U = [[1,0,1],[0,1,1]], b = [3,2]
        // Solutions: x0 + t*n where Ux=b
        // Null vector of U: (1,1,-1) (up to sign)
        let u = [[1, 0, 1], [0, 1, 1]];
        let b = [3, 2];
        let solutions = solve_nonneg_2x3(&u, &b, 10);
        assert!(!solutions.is_empty());
        for x in &solutions {
            let check0 = u[0][0] * x[0] as i64 + u[0][1] * x[1] as i64 + u[0][2] * x[2] as i64;
            let check1 = u[1][0] * x[0] as i64 + u[1][1] * x[1] as i64 + u[1][2] * x[2] as i64;
            assert_eq!(check0, b[0], "Failed for x={:?}", x);
            assert_eq!(check1, b[1], "Failed for x={:?}", x);
        }
    }

    #[test]
    fn test_solve_nonneg_2x3_repeated_calls_match() {
        let u = [[1, 0, 1], [0, 1, 1]];
        let b = [3, 2];
        let first = solve_nonneg_2x3(&u, &b, 10);
        let second = solve_nonneg_2x3(&u, &b, 10);
        assert_eq!(first, second);
    }

    #[test]
    fn test_solve_nonneg_2x3_no_solution() {
        // U = [[1,0,0],[0,1,0]], b = [1,1]
        // Null vector: (0,0,1). Particular solution: (1,1,0).
        // But solutions are (1,1,t) for t>=0. This should have solutions.
        let u = [[1, 0, 0], [0, 1, 0]];
        let b = [1, 1];
        let solutions = solve_nonneg_2x3(&u, &b, 5);
        assert_eq!(solutions.len(), 6); // t = 0,1,2,3,4,5
        assert_eq!(solutions[0], [1, 1, 0]);
        assert_eq!(solutions[5], [1, 1, 5]);
    }

    #[test]
    fn test_solve_nonneg_2x3_rank_deficient() {
        // Both rows identical → rank 1 → no solutions returned.
        let u = [[1, 2, 3], [1, 2, 3]];
        let b = [6, 7]; // inconsistent
        let solutions = solve_nonneg_2x3(&u, &b, 10);
        assert!(solutions.is_empty());
    }

    #[test]
    fn test_solve_overdetermined_3x2_basic() {
        // U = [[1,0],[0,1],[1,1]], b = [2,3,5]
        // x = [2,3], check: 1*2+0*3=2, 0*2+1*3=3, 1*2+1*3=5. ✓
        let u = [[1, 0], [0, 1], [1, 1]];
        let b = [2, 3, 5];
        let result = solve_overdetermined_3x2(&u, &b, 10);
        assert_eq!(result, Some([2, 3]));
    }

    #[test]
    fn test_solve_overdetermined_3x2_inconsistent() {
        // U = [[1,0],[0,1],[1,1]], b = [2,3,6]
        // x = [2,3] but 2+3=5≠6 → inconsistent.
        let u = [[1, 0], [0, 1], [1, 1]];
        let b = [2, 3, 6];
        let result = solve_overdetermined_3x2(&u, &b, 10);
        assert_eq!(result, None);
    }

    #[test]
    fn test_solve_overdetermined_3x2_negative() {
        // Solution would be negative.
        let u = [[1, 0], [0, 1], [1, 1]];
        let b = [-1, 3, 2];
        let result = solve_overdetermined_3x2(&u, &b, 10);
        assert_eq!(result, None);
    }

    #[test]
    fn test_solve_overdetermined_4x3_basic() {
        let u = [[1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 1, 0]];
        let b = [2, 3, 5, 5];
        let result = solve_overdetermined_4x3(&u, &b, 10);
        assert_eq!(result, Some([2, 3, 5]));
    }

    #[test]
    fn test_visit_all_factorisations_includes_opposite_shear_conjugation() {
        let u = DynMatrix::new(3, 3, vec![5, 2, 0, 2, 1, 0, 0, 0, 1]);
        let v = DynMatrix::new(3, 3, vec![1, 1, 0, 0, 1, 0, 0, 0, 1]);
        let c = u.mul(&v);
        let mut found = false;

        visit_all_factorisations(&c, 3, 6, |cand_u, cand_v| {
            if cand_u == u && cand_v == v {
                found = true;
            }
        });

        assert!(found, "expected opposite-shear conjugation factorisation");
    }

    #[test]
    fn test_visit_all_factorisations_includes_parallel_shear_conjugation() {
        let u = DynMatrix::new(3, 3, vec![1, 5, 1, 0, 1, 0, 0, 0, 1]);
        let v = DynMatrix::new(3, 3, vec![1, 0, 0, 1, 1, 0, 1, 0, 1]);
        let c = u.mul(&v);
        let mut found = false;

        visit_all_factorisations(&c, 3, 6, |cand_u, cand_v| {
            if cand_u == u && cand_v == v {
                found = true;
            }
        });

        assert!(found, "expected parallel-shear conjugation factorisation");
    }

    #[test]
    fn test_visit_all_factorisations_includes_convergent_shear_conjugation() {
        let u = DynMatrix::new(3, 3, vec![1, 0, 0, 5, 1, 0, 1, 0, 1]);
        let v = DynMatrix::new(3, 3, vec![1, 1, 0, 0, 1, 0, 0, 0, 1]);
        let c = u.mul(&v);
        let mut found = false;

        visit_all_factorisations(&c, 3, 6, |cand_u, cand_v| {
            if cand_u == u && cand_v == v {
                found = true;
            }
        });

        assert!(found, "expected convergent-shear conjugation factorisation");
    }

    #[test]
    fn test_binary_sparse_factorisations_reach_baker_step_6() {
        let current = DynMatrix::new(4, 4, vec![1, 1, 1, 1, 3, 0, 2, 2, 1, 0, 0, 0, 0, 1, 1, 1]);
        let target = DynMatrix::new(3, 3, vec![1, 1, 1, 5, 0, 5, 1, 0, 1]);
        let mut found = false;

        visit_binary_sparse_factorisations_4x4_to_3(&current, 5, &mut |u, v| {
            if v.mul(&u) == target {
                found = true;
            }
        });

        assert!(
            found,
            "expected binary sparse 4x4->3x3 factorisation for Baker step 6"
        );
    }

    #[test]
    fn test_binary_sparse_factorisations_reach_hidden_baker_step_5_bridge() {
        let current = DynMatrix::new(4, 4, vec![1, 2, 2, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 2, 1, 0]);
        let bridge = DynMatrix::new(3, 3, vec![1, 1, 1, 3, 0, 2, 1, 1, 1]);
        let mut found = false;

        visit_binary_sparse_factorisations_4x4_to_3(&current, 5, &mut |u, v| {
            if v.mul(&u) == bridge {
                found = true;
            }
        });

        assert!(
            found,
            "expected binary sparse 4x4->3x3 factorisation for the hidden Baker step 5 bridge"
        );
    }

    #[test]
    fn test_binary_sparse_factorisations_reach_baker_step_2() {
        let current = DynMatrix::new(3, 3, vec![1, 2, 2, 2, 1, 1, 1, 0, 0]);
        let target = DynMatrix::new(4, 4, vec![1, 2, 2, 0, 1, 0, 2, 0, 0, 1, 1, 1, 1, 1, 2, 0]);
        let mut found = false;

        visit_binary_sparse_factorisations_3x3_to_4(&current, 5, &mut |u, v| {
            if v.mul(&u) == target {
                found = true;
            }
        });

        assert!(
            found,
            "expected binary sparse 3x3->4x4 factorisation for Baker step 2"
        );
    }

    #[test]
    fn test_visit_all_factorisations_includes_binary_sparse_3x3_to_4_family() {
        let current = DynMatrix::new(3, 3, vec![1, 2, 2, 2, 1, 1, 1, 0, 0]);
        let target = DynMatrix::new(4, 4, vec![1, 2, 2, 0, 1, 0, 2, 0, 0, 1, 1, 1, 1, 1, 2, 0]);
        let mut found = false;

        visit_all_factorisations_with_family(&current, 4, 5, |family, u, v| {
            if family == "binary_sparse_rectangular_factorisation_3x3_to_4" && v.mul(&u) == target {
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the binary sparse 3x3->4x4 family"
        );
    }

    // --- Rectangular factorisation tests ---

    #[test]
    fn test_rect_factorisations_verify_product() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let facts = enumerate_rect_factorisations_2x3(&a, 3);
        assert!(
            !facts.is_empty(),
            "Should find at least one rectangular factorisation"
        );
        for (u, v) in &facts {
            assert_eq!(u.rows, 2);
            assert_eq!(u.cols, 3);
            assert_eq!(v.rows, 3);
            assert_eq!(v.cols, 2);
            let uv = u.mul(v);
            assert_eq!(
                uv,
                DynMatrix::from_sq(&a),
                "UV != A for U={:?}, V={:?}",
                u,
                v
            );
        }
    }

    #[test]
    fn test_rect_factorisations_known() {
        // [[2,1],[1,1]] = [[1,1,0],[0,0,1]] * [[1,0],[1,0],[1,1]]
        // Check: [[1*1+1*1+0*1, 1*0+1*0+0*1],[0*1+0*1+1*1, 0*0+0*0+1*1]] = [[2,0],[1,1]]
        // Hmm that doesn't work. Let me construct a valid one:
        // U = [[1,0,1],[0,1,0]], V = [[1,0],[1,1],[1,1]]
        // UV = [[1+0+1, 0+0+1],[0+1+0, 0+1+0]] = [[2,1],[1,1]] ✓
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let facts = enumerate_rect_factorisations_2x3(&a, 3);
        let u_expected = DynMatrix::new(2, 3, vec![1, 0, 1, 0, 1, 0]);
        let v_expected = DynMatrix::new(3, 2, vec![1, 0, 1, 1, 1, 1]);
        assert!(
            facts.contains(&(u_expected, v_expected)),
            "Expected rectangular factorisation not found"
        );
    }

    #[test]
    fn test_3x3_to_2_verify_product() {
        // Create a 3×3 matrix that factors as (3×2)(2×3).
        let u = DynMatrix::new(3, 2, vec![1, 0, 0, 1, 1, 1]);
        let v = DynMatrix::new(2, 3, vec![1, 0, 1, 0, 1, 1]);
        let c = u.mul(&v);
        // c = [[1,0,1],[0,1,1],[1,1,2]]

        let facts = enumerate_factorisations_3x3_to_2(&c, 5);
        assert!(!facts.is_empty(), "Should find at least one factorisation");
        for (u_found, v_found) in &facts {
            assert_eq!(u_found.rows, 3);
            assert_eq!(u_found.cols, 2);
            assert_eq!(v_found.rows, 2);
            assert_eq!(v_found.cols, 3);
            let product = u_found.mul(v_found);
            assert_eq!(product, c, "UV != C for U={:?}, V={:?}", u_found, v_found);
        }
    }

    #[test]
    fn test_enumerate_all_2x2() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let facts = enumerate_all_factorisations(&a_dyn, 3, 3);
        // Should include both square and rectangular factorisations.
        let has_square = facts.iter().any(|(u, _)| u.cols == 2);
        let has_rect = facts.iter().any(|(u, _)| u.cols == 3);
        assert!(has_square, "Should include square factorisations");
        assert!(has_rect, "Should include rectangular factorisations");
        for (u, v) in &facts {
            let uv = u.mul(v);
            assert_eq!(uv, a_dyn, "UV != A");
        }
    }

    // --- 4×4 solver tests ---

    #[test]
    fn test_solve_nonneg_4x4_identity() {
        let a = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]];
        let b = [2, 3, 5, 7];
        let solutions = solve_nonneg_4x4(&a, &b, 10);
        assert_eq!(solutions, vec![[2, 3, 5, 7]]);
    }

    #[test]
    fn test_solve_nonneg_4x4_no_solution() {
        let a = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]];
        let b = [2, 3, 5, -1]; // negative RHS
        let solutions = solve_nonneg_4x4(&a, &b, 10);
        assert!(solutions.is_empty());
    }

    #[test]
    fn test_solve_nonneg_4x4_nonsingular() {
        // A = [[1,1,0,0],[0,1,1,0],[0,0,1,1],[1,0,0,1]]
        // x = [1,1,1,1]: Ax = [2,2,2,2]
        let a = [[1, 1, 0, 0], [0, 1, 1, 0], [0, 0, 1, 1], [1, 0, 0, 1]];
        let b = [2, 2, 2, 2];
        let solutions = solve_nonneg_4x4(&a, &b, 10);
        assert_eq!(solutions.len(), 1);
        // Verify
        for x in &solutions {
            for row in 0..4 {
                let check: i64 = (0..4).map(|c| a[row][c] * x[c] as i64).sum();
                assert_eq!(check, b[row]);
            }
        }
    }

    #[test]
    fn test_solve_overdetermined_5x4_basic() {
        let u = [
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
            [1, 1, 0, 0],
        ];
        let b = [2, 3, 5, 7, 5];
        let result = solve_overdetermined_5x4(&u, &b, 10);
        assert_eq!(result, Some([2, 3, 5, 7]));
    }

    #[test]
    fn test_solve_overdetermined_5x4_inconsistent() {
        let u = [
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
            [1, 1, 0, 0],
        ];
        let b = [2, 3, 5, 7, 99]; // 2+3=5≠99
        let result = solve_overdetermined_5x4(&u, &b, 10);
        assert_eq!(result, None);
    }

    // --- 5x5→4x4 and 4x4→5x5 factorisation tests ---

    #[test]
    fn test_binary_sparse_5x5_to_4_verify_product() {
        // Construct a 5×5 matrix that factors as (5×4)(4×5).
        let u = DynMatrix::new(
            5,
            4,
            vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0],
        );
        let v = DynMatrix::new(
            4,
            5,
            vec![1, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1],
        );
        let a = u.mul(&v);

        let mut results = Vec::new();
        visit_binary_sparse_factorisations_5x5_to_4(&a, 5, &mut |u, v| {
            results.push((u, v));
        });

        assert!(
            !results.is_empty(),
            "Should find at least one 5x5->4x4 factorisation"
        );
        for (u_found, v_found) in &results {
            assert_eq!(u_found.rows, 5);
            assert_eq!(u_found.cols, 4);
            assert_eq!(v_found.rows, 4);
            assert_eq!(v_found.cols, 5);
            let product = u_found.mul(v_found);
            assert_eq!(product, a, "UV != A for U={:?}, V={:?}", u_found, v_found);
        }
    }

    #[test]
    fn test_binary_sparse_4x4_to_5_verify_product() {
        // Construct a 4×4 matrix that factors as (4×5)(5×4).
        let u = DynMatrix::new(
            4,
            5,
            vec![1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1],
        );
        let v = DynMatrix::new(
            5,
            4,
            vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0],
        );
        let a = u.mul(&v);

        let mut results = Vec::new();
        visit_binary_sparse_factorisations_4x4_to_5(&a, 5, &mut |u, v| {
            results.push((u, v));
        });

        assert!(
            !results.is_empty(),
            "Should find at least one 4x4->5x5 factorisation"
        );
        for (u_found, v_found) in &results {
            assert_eq!(u_found.rows, 4);
            assert_eq!(u_found.cols, 5);
            assert_eq!(v_found.rows, 5);
            assert_eq!(v_found.cols, 4);
            let product = u_found.mul(v_found);
            assert_eq!(product, a, "UV != A for U={:?}, V={:?}", u_found, v_found);
        }
    }

    #[test]
    fn test_dispatcher_exposes_4x4_to_5_family() {
        // Construct a 4×4 matrix that has a binary-sparse 4→5 factorisation.
        let u = DynMatrix::new(
            4,
            5,
            vec![1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1],
        );
        let v = DynMatrix::new(
            5,
            4,
            vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0],
        );
        let a = u.mul(&v);
        let mut found = false;

        visit_all_factorisations_with_family(&a, 5, 5, |family, u, v| {
            if family == "binary_sparse_rectangular_factorisation_4x4_to_5" {
                assert_eq!(u.mul(&v), a);
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the binary sparse 4x4->5x5 family"
        );
    }

    #[test]
    fn test_dispatcher_exposes_5x5_to_4_family() {
        let u = DynMatrix::new(
            5,
            4,
            vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0],
        );
        let v = DynMatrix::new(
            4,
            5,
            vec![1, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1],
        );
        let a = u.mul(&v);
        let mut found = false;

        visit_all_factorisations_with_family(&a, 5, 5, |family, u, v| {
            if family == "binary_sparse_rectangular_factorisation_5x5_to_4" {
                assert_eq!(u.mul(&v), a);
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the binary sparse 5x5->4x4 family"
        );
    }
}
