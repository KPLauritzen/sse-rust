use std::cell::RefCell;
use std::collections::HashMap;

use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::MoveFamilyPolicy;

use rayon::prelude::*;

const VALID_2X3_U_ROW_CACHE_LIMIT: usize = 1024;
const VALID_3X2_U_ROW_CACHE_LIMIT: usize = 1024;

type Valid2x3URowKey = ([u32; 2], u32);
type Valid3x2URowKey = ([u32; 3], u32);

thread_local! {
    static VALID_2X3_U_ROW_CACHE: RefCell<HashMap<Valid2x3URowKey, Vec<[u32; 3]>>> =
        RefCell::new(HashMap::new());
    static VALID_3X2_U_ROW_CACHE: RefCell<HashMap<Valid3x2URowKey, Vec<[u32; 2]>>> =
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

fn valid_2x3_u_rows_for_target_row(target_row: [u32; 2], max_entry: u32) -> Vec<[u32; 3]> {
    let key = (target_row, max_entry);
    if let Some(cached) = VALID_2X3_U_ROW_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return cached;
    }

    let me = max_entry;
    let max_target = target_row[0].max(target_row[1]) as u64;
    let min_row_sum = ((max_target + me as u64 - 1) / me as u64) as u32;
    let mut valid_rows = Vec::new();

    for u0 in 0..=me {
        for u1 in 0..=me {
            for u2 in 0..=me {
                let row_sum = u0 + u1 + u2;
                if row_sum < min_row_sum {
                    continue;
                }
                if row_sum == 0 && (target_row[0] > 0 || target_row[1] > 0) {
                    continue;
                }

                let row_gcd = gcd3(u0 as u64, u1 as u64, u2 as u64);
                if row_gcd > 1
                    && (target_row[0] as u64 % row_gcd != 0 || target_row[1] as u64 % row_gcd != 0)
                {
                    continue;
                }

                valid_rows.push([u0, u1, u2]);
            }
        }
    }

    VALID_2X3_U_ROW_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= VALID_2X3_U_ROW_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, valid_rows.clone());
    });
    valid_rows
}

fn valid_3x2_u_rows_for_target_row(target_row: [u32; 3], max_entry: u32) -> Vec<[u32; 2]> {
    let key = (target_row, max_entry);
    if let Some(cached) = VALID_3X2_U_ROW_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return cached;
    }

    let me = max_entry;
    let max_target = target_row[0].max(target_row[1]).max(target_row[2]) as u64;
    let min_row_sum = ((max_target + me as u64 - 1) / me as u64) as u32;
    let mut valid_rows = Vec::new();

    for u0 in 0..=me {
        for u1 in 0..=me {
            if u0 + u1 < min_row_sum {
                continue;
            }

            let row_gcd = gcd(u0 as u64, u1 as u64);
            if row_gcd > 1 && target_row.iter().any(|&value| value as u64 % row_gcd != 0) {
                continue;
            }

            valid_rows.push([u0, u1]);
        }
    }

    VALID_3X2_U_ROW_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= VALID_3X2_U_ROW_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, valid_rows.clone());
    });
    valid_rows
}

// --- Nonneg integer linear system solvers ---

/// Solve U·x = b where U is 2×3 (given as rows), b is 2-vector.
/// Returns all nonneg integer 3-vectors x with entries ≤ max_entry.
///
/// Algorithm: find a 2×2 pivot submatrix with nonzero determinant,
/// compute the 1D null space, find a particular solution, then enumerate
/// the free parameter t such that x0 + t*n has all entries in [0, max_entry].
pub fn solve_nonneg_2x3(u: &[[i64; 3]; 2], b: &[i64; 2], max_entry: u32) -> Vec<[u32; 3]> {
    let mut results = Vec::new();
    solve_nonneg_2x3_into(u, b, max_entry, &mut results);
    results
}

fn solve_nonneg_2x3_into(
    u: &[[i64; 3]; 2],
    b: &[i64; 2],
    max_entry: u32,
    results: &mut Vec<[u32; 3]>,
) {
    results.clear();
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
        return;
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
        return;
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
            results.push([x0[0] as u32, x0[1] as u32, x0[2] as u32]);
        }
        return;
    }
    let n = [null[0] / g as i64, null[1] / g as i64, null[2] / g as i64];

    // Find the range of t such that 0 <= x0[i] + t*n[i] <= max_entry for all i.
    let mut t_min = i64::MIN;
    let mut t_max = i64::MAX;

    for i in 0..3 {
        if n[i] == 0 {
            // x0[i] must be in [0, me] on its own.
            if x0[i] < 0 || x0[i] > me {
                return;
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

    if t_min > t_max {
        return;
    }

    for t in t_min..=t_max {
        let x = [
            (x0[0] + t * n[0]) as u32,
            (x0[1] + t * n[1]) as u32,
            (x0[2] + t * n[2]) as u32,
        ];
        results.push(x);
    }
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
    let a_cols: [[i64; 2]; 2] = [
        [a.data[0][0] as i64, a.data[1][0] as i64],
        [a.data[0][1] as i64, a.data[1][1] as i64],
    ];
    let valid_row0s = valid_2x3_u_rows_for_target_row(a.data[0], max_entry);
    let valid_row1s = valid_2x3_u_rows_for_target_row(a.data[1], max_entry);

    let per_row0: Vec<Vec<(DynMatrix, DynMatrix)>> = valid_row0s
        .par_iter()
        .map(|&row0| {
            enumerate_rect_factorisations_2x3_from_row0(row0, &a_cols, &valid_row1s, max_entry)
        })
        .collect();
    for row_results in per_row0 {
        for (u, v) in row_results {
            visit(u, v);
        }
    }
}

fn enumerate_rect_factorisations_2x3_from_row0(
    row0: [u32; 3],
    a_cols: &[[i64; 2]; 2],
    valid_row1s: &[[u32; 3]],
    max_entry: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    let [u00, u01, u02] = row0;
    let mut results = Vec::new();

    for &[u10, u11, u12] in valid_row1s {
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
                let v_mat =
                    DynMatrix::new(3, 2, vec![vc0[0], vc1[0], vc0[1], vc1[1], vc0[2], vc1[2]]);
                results.push((u_mat.clone(), v_mat));
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

    let valid_row0s =
        valid_3x2_u_rows_for_target_row([c.get(0, 0), c.get(0, 1), c.get(0, 2)], max_entry);
    let valid_row1s =
        valid_3x2_u_rows_for_target_row([c.get(1, 0), c.get(1, 1), c.get(1, 2)], max_entry);
    let valid_row2s =
        valid_3x2_u_rows_for_target_row([c.get(2, 0), c.get(2, 1), c.get(2, 2)], max_entry);

    let per_row0: Vec<Vec<(DynMatrix, DynMatrix)>> = valid_row0s
        .par_iter()
        .map(|&row0| {
            enumerate_factorisations_3x3_to_2_from_row0(
                row0,
                &c_cols,
                &c_row2,
                &valid_row1s,
                &valid_row2s,
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

fn enumerate_factorisations_3x3_to_2_from_row0(
    row0: [u32; 2],
    c_cols: &[[i64; 3]; 3],
    c_row2: &[i64; 3],
    valid_row1s: &[[u32; 2]],
    valid_row2s: &[[u32; 2]],
    max_entry: u32,
    me_i64: i64,
    min_row_sum: [u32; 3],
) -> Vec<(DynMatrix, DynMatrix)> {
    let [u00, u01] = row0;
    let mut results = Vec::new();

    for &[u10, u11] in valid_row1s {
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
                    for &[u20, u21] in valid_row2s {
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
                    }
                }
            }
        } else {
            for &[u20, u21] in valid_row2s {
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
                let v_mat = DynMatrix::new(2, 3, vec![v0[0], v1[0], v2[0], v0[1], v1[1], v2[1]]);
                results.push((u_mat, v_mat));
            }
        }
    }

    results
}

fn build_contiguous_row_split_duplication_matrix(clone_counts: &[usize]) -> DynMatrix {
    let rows = clone_counts.len();
    let cols = clone_counts.iter().sum();
    let mut data = vec![0u32; rows * cols];
    let mut col_start = 0usize;

    for (row, &count) in clone_counts.iter().enumerate() {
        for col in col_start..(col_start + count) {
            data[row * cols + col] = 1;
        }
        col_start += count;
    }

    DynMatrix::new(rows, cols, data)
}

fn single_row_split_3x3_to_4x4_family_is_nonempty(a: &DynMatrix) -> bool {
    assert_eq!(a.rows, 3);
    assert_eq!(a.cols, 3);

    (0..3).any(|row| {
        u64::from(a.get(row, 0)) + u64::from(a.get(row, 1)) + u64::from(a.get(row, 2)) >= 2
    })
}

fn single_column_split_3x3_to_4x4_family_is_nonempty(a: &DynMatrix) -> bool {
    assert_eq!(a.rows, 3);
    assert_eq!(a.cols, 3);

    (0..3).any(|col| {
        u64::from(a.get(0, col)) + u64::from(a.get(1, col)) + u64::from(a.get(2, col)) >= 2
    })
}

/// Enumerate a bounded explicit 3x3 -> 4x4 row-splitting family.
///
/// One chosen source row is split into two contiguous clones, while the other
/// rows stay fixed. The matching source column is duplicated by a fixed
/// 3x4 duplication matrix, so the target vocabulary stays much smaller than
/// the generic 3x3 -> 4x4 rectangular families.
fn visit_single_row_split_factorisations_3x3_to_4<F>(a: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 3);
    assert_eq!(a.cols, 3);

    if !single_row_split_3x3_to_4x4_family_is_nonempty(a) {
        return;
    }

    let rows = [
        [a.get(0, 0), a.get(0, 1), a.get(0, 2)],
        [a.get(1, 0), a.get(1, 1), a.get(1, 2)],
        [a.get(2, 0), a.get(2, 1), a.get(2, 2)],
    ];

    for split_row in 0..3 {
        let mut clone_counts = [1usize; 3];
        clone_counts[split_row] = 2;
        let u = build_contiguous_row_split_duplication_matrix(&clone_counts);
        let original = rows[split_row];

        let lower0 = original[0].saturating_sub(max_entry);
        let upper0 = original[0].min(max_entry);
        let lower1 = original[1].saturating_sub(max_entry);
        let upper1 = original[1].min(max_entry);
        let lower2 = original[2].saturating_sub(max_entry);
        let upper2 = original[2].min(max_entry);

        for split0 in lower0..=upper0 {
            for split1 in lower1..=upper1 {
                for split2 in lower2..=upper2 {
                    let split = [split0, split1, split2];
                    let twin = [
                        original[0] - split0,
                        original[1] - split1,
                        original[2] - split2,
                    ];
                    if split == [0, 0, 0] || twin == [0, 0, 0] {
                        continue;
                    }
                    if split > twin {
                        continue;
                    }

                    let mut v_data = Vec::with_capacity(12);
                    for row in 0..split_row {
                        v_data.extend_from_slice(&rows[row]);
                    }
                    v_data.extend_from_slice(&split);
                    v_data.extend_from_slice(&twin);
                    for row in (split_row + 1)..3 {
                        v_data.extend_from_slice(&rows[row]);
                    }

                    visit(u.clone(), DynMatrix::new(4, 3, v_data));
                }
            }
        }
    }
}

/// Enumerate a bounded explicit 4x4 -> 5x5 row-splitting family.
///
/// One chosen source row is split into two contiguous clones, while the other
/// rows stay fixed. The matching source column is duplicated by a fixed
/// 4x5 duplication matrix, so the target vocabulary stays much smaller than
/// the generic 4x4 -> 5x5 rectangular families.
fn visit_single_row_split_factorisations_4x4_to_5<F>(a: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 4);
    assert_eq!(a.cols, 4);

    let rows = [
        [a.get(0, 0), a.get(0, 1), a.get(0, 2), a.get(0, 3)],
        [a.get(1, 0), a.get(1, 1), a.get(1, 2), a.get(1, 3)],
        [a.get(2, 0), a.get(2, 1), a.get(2, 2), a.get(2, 3)],
        [a.get(3, 0), a.get(3, 1), a.get(3, 2), a.get(3, 3)],
    ];

    for split_row in 0..4 {
        let mut clone_counts = [1usize; 4];
        clone_counts[split_row] = 2;
        let u = build_contiguous_row_split_duplication_matrix(&clone_counts);
        let original = rows[split_row];

        let lower0 = original[0].saturating_sub(max_entry);
        let upper0 = original[0].min(max_entry);
        let lower1 = original[1].saturating_sub(max_entry);
        let upper1 = original[1].min(max_entry);
        let lower2 = original[2].saturating_sub(max_entry);
        let upper2 = original[2].min(max_entry);
        let lower3 = original[3].saturating_sub(max_entry);
        let upper3 = original[3].min(max_entry);

        for split0 in lower0..=upper0 {
            for split1 in lower1..=upper1 {
                for split2 in lower2..=upper2 {
                    for split3 in lower3..=upper3 {
                        let split = [split0, split1, split2, split3];
                        let twin = [
                            original[0] - split0,
                            original[1] - split1,
                            original[2] - split2,
                            original[3] - split3,
                        ];
                        if split == [0, 0, 0, 0] || twin == [0, 0, 0, 0] {
                            continue;
                        }
                        if split > twin {
                            continue;
                        }

                        let mut v_data = Vec::with_capacity(20);
                        for row in 0..split_row {
                            v_data.extend_from_slice(&rows[row]);
                        }
                        v_data.extend_from_slice(&split);
                        v_data.extend_from_slice(&twin);
                        for row in (split_row + 1)..4 {
                            v_data.extend_from_slice(&rows[row]);
                        }

                        visit(u.clone(), DynMatrix::new(5, 4, v_data));
                    }
                }
            }
        }
    }
}

/// Enumerate a bounded explicit 5x5 -> 4x4 row-amalgamation family.
///
/// One chosen contiguous source-row pair is amalgamated into a single target
/// row, while the other rows stay fixed. The matching contiguous source-column
/// pair must already be duplicated, so the factorisation uses the fixed
/// 4x5 duplication matrix from the `4x4 -> 5x5` row-splitting sibling.
fn visit_single_row_amalgamation_factorisations_5x5_to_4<F>(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut F,
) where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 5);
    assert_eq!(a.cols, 5);

    for merge_row in 0..4 {
        let mut clone_counts = [1usize; 4];
        clone_counts[merge_row] = 2;
        let v = build_contiguous_row_split_duplication_matrix(&clone_counts);
        let mut u_data = Vec::with_capacity(20);
        let mut valid = true;

        for row in 0..5 {
            if a.get(row, merge_row) != a.get(row, merge_row + 1) {
                valid = false;
                break;
            }

            let recovered = match merge_row {
                0 => [a.get(row, 0), a.get(row, 2), a.get(row, 3), a.get(row, 4)],
                1 => [a.get(row, 0), a.get(row, 1), a.get(row, 3), a.get(row, 4)],
                2 => [a.get(row, 0), a.get(row, 1), a.get(row, 2), a.get(row, 4)],
                3 => [a.get(row, 0), a.get(row, 1), a.get(row, 2), a.get(row, 3)],
                _ => unreachable!("merge_row is always in 0..4"),
            };
            if recovered.into_iter().any(|entry| entry > max_entry) {
                valid = false;
                break;
            }
            if (row == merge_row || row == merge_row + 1) && recovered == [0, 0, 0, 0] {
                valid = false;
                break;
            }
            u_data.extend_from_slice(&recovered);
        }

        if valid {
            visit(DynMatrix::new(5, 4, u_data), v);
        }
    }
}

/// Enumerate a bounded explicit 5x5 -> 4x4 column-amalgamation family.
///
/// This is the transpose-dual of the bounded row-amalgamation slice: one
/// chosen contiguous source-column pair is amalgamated into a single target
/// column, while the other columns stay fixed. The matching contiguous source
/// row pair must already be duplicated, so the factorisation uses the fixed
/// transposed 5x4 duplication matrix from the `4x4 -> 5x5` column-splitting
/// sibling.
fn visit_single_column_amalgamation_factorisations_5x5_to_4<F>(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut F,
) where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 5);
    assert_eq!(a.cols, 5);

    visit_single_row_amalgamation_factorisations_5x5_to_4(
        &a.transpose(),
        max_entry,
        &mut |u_transposed, v_transposed| {
            visit(v_transposed.transpose(), u_transposed.transpose());
        },
    );
}

/// Enumerate a bounded explicit 3x3 -> 4x4 column-splitting family.
///
/// This is the transpose-dual of the bounded row-splitting slice: one chosen
/// source column is split into two contiguous clones, while the other columns
/// stay fixed. The matching source row is duplicated by the fixed transposed
/// 4x3 duplication matrix, so the target vocabulary stays explicit and bounded.
fn visit_single_column_split_factorisations_3x3_to_4<F>(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut F,
) where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 3);
    assert_eq!(a.cols, 3);

    if !single_column_split_3x3_to_4x4_family_is_nonempty(a) {
        return;
    }

    visit_single_row_split_factorisations_3x3_to_4(
        &a.transpose(),
        max_entry,
        &mut |u_transposed, v_transposed| {
            visit(v_transposed.transpose(), u_transposed.transpose());
        },
    );
}

/// Enumerate a bounded explicit 4x4 -> 5x5 column-splitting family.
///
/// This is the transpose-dual of the bounded row-splitting slice: one chosen
/// source column is split into two contiguous clones, while the other columns
/// stay fixed. The matching source row is duplicated by the fixed transposed
/// 5x4 duplication matrix, so the target vocabulary stays explicit and bounded.
fn visit_single_column_split_factorisations_4x4_to_5<F>(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut F,
) where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(a.rows, 4);
    assert_eq!(a.cols, 4);

    visit_single_row_split_factorisations_4x4_to_5(
        &a.transpose(),
        max_entry,
        &mut |u_transposed, v_transposed| {
            visit(v_transposed.transpose(), u_transposed.transpose());
        },
    );
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

fn is_weighted_sparse_row_len3(row: [u32; 3], max_entry: u32) -> bool {
    let mut first = None;
    let mut second = None;
    for value in row {
        if value == 0 {
            continue;
        }
        if value > max_entry {
            return false;
        }
        if first.is_none() {
            first = Some(value);
        } else if second.is_none() {
            second = Some(value);
        } else {
            return false;
        }
    }
    let Some(first) = first else {
        return false;
    };
    let Some(second) = second else {
        return true;
    };
    first == 1 || second == 1 || first == second
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

                let u_top_adjugate = adjugate_matrix_3x3(&u_top);

                let mut v_cols = [[0u32; 3]; 4];
                let mut ok = true;
                for (idx, col) in a_cols.iter().enumerate() {
                    let Some(solution) =
                        solve_nonneg_3x3_with_adjugate(&u_top_adjugate, det, col, max_entry)
                    else {
                        ok = false;
                        break;
                    };
                    v_cols[idx] = solution;
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

                            let det = det3x3(&core);
                            if det == 0 {
                                continue;
                            }
                            let core_adjugate = adjugate_matrix_3x3(&core);

                            let mut core_row_cols = [[0u32; 3]; 3];
                            let mut core_valid = true;
                            for (idx, residual_col) in residual_cols.iter().enumerate() {
                                let Some(solution) = solve_nonneg_3x3_with_adjugate(
                                    &core_adjugate,
                                    det,
                                    residual_col,
                                    max_entry,
                                ) else {
                                    core_valid = false;
                                    break;
                                };
                                core_row_cols[idx] = solution;
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

                    let (u_top_cofactors, det) = cofactor_matrix_and_det_4x4(&u_top);
                    if det == 0 {
                        continue;
                    }

                    let mut v_cols = [[0u32; 4]; 5];
                    let mut ok = true;
                    for (idx, col) in a_cols.iter().enumerate() {
                        let Some(solution) =
                            solve_nonneg_4x4_with_cofactors(&u_top_cofactors, det, col, max_entry)
                        else {
                            ok = false;
                            break;
                        };
                        v_cols[idx] = solution;
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

                                let (core_cofactors, det) = cofactor_matrix_and_det_4x4(&core);
                                if det == 0 {
                                    continue;
                                }

                                let mut core_row_cols = [[0u32; 4]; 4];
                                let mut core_valid = true;
                                for (idx, residual_col) in residual_cols.iter().enumerate() {
                                    let Some(solution) = solve_nonneg_4x4_with_cofactors(
                                        &core_cofactors,
                                        det,
                                        residual_col,
                                        max_entry,
                                    ) else {
                                        core_valid = false;
                                        break;
                                    };
                                    core_row_cols[idx] = solution;
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

const DIAGONAL_REFACTORIZATION_3X3_MAX_DIAG_ENTRY: u32 = 3;
const DIAGONAL_REFACTORIZATION_4X4_MAX_DIAG_ENTRY: u32 = 2;

fn row_admits_diagonal_divisor<const N: usize>(
    row: &[u32; N],
    divisor: u32,
    max_entry: u32,
) -> bool {
    row.iter()
        .all(|&value| value % divisor == 0 && value / divisor <= max_entry)
}

fn diagonal_refactorization_3x3_family_is_nonempty(c: &DynMatrix, max_entry: u32) -> bool {
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let diag_cap = max_entry.min(DIAGONAL_REFACTORIZATION_3X3_MAX_DIAG_ENTRY);
    if diag_cap <= 1 {
        return false;
    }

    let cm = [
        [c.get(0, 0), c.get(0, 1), c.get(0, 2)],
        [c.get(1, 0), c.get(1, 1), c.get(1, 2)],
        [c.get(2, 0), c.get(2, 1), c.get(2, 2)],
    ];

    let row_divisors = [
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&cm[0], divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&cm[1], divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&cm[2], divisor, max_entry))
            .collect::<Vec<_>>(),
    ];
    for &d0 in &row_divisors[0] {
        for &d1 in &row_divisors[1] {
            for &d2 in &row_divisors[2] {
                let diag = [d0, d1, d2];
                if d0 == d1 && d1 == d2 {
                    continue;
                }
                if divide_rows_by_diag_3x3(&cm, &diag, max_entry)
                    .is_some_and(|x| scale_cols_by_diag_3x3(&x, &diag) != cm)
                {
                    return true;
                }
            }
        }
    }

    let col0 = [cm[0][0], cm[1][0], cm[2][0]];
    let col1 = [cm[0][1], cm[1][1], cm[2][1]];
    let col2 = [cm[0][2], cm[1][2], cm[2][2]];
    let col_divisors = [
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&col0, divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&col1, divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&col2, divisor, max_entry))
            .collect::<Vec<_>>(),
    ];
    for &d0 in &col_divisors[0] {
        for &d1 in &col_divisors[1] {
            for &d2 in &col_divisors[2] {
                let diag = [d0, d1, d2];
                if d0 == d1 && d1 == d2 {
                    continue;
                }
                if divide_cols_by_diag_3x3(&cm, &diag, max_entry)
                    .is_some_and(|x| scale_rows_by_diag_3x3(&x, &diag) != cm)
                {
                    return true;
                }
            }
        }
    }

    false
}

fn diagonal_refactorization_4x4_family_is_nonempty(c: &DynMatrix, max_entry: u32) -> bool {
    assert_eq!(c.rows, 4);
    assert_eq!(c.cols, 4);

    let diag_cap = max_entry.min(DIAGONAL_REFACTORIZATION_4X4_MAX_DIAG_ENTRY);
    if diag_cap <= 1 {
        return false;
    }

    let cm = [
        [c.get(0, 0), c.get(0, 1), c.get(0, 2), c.get(0, 3)],
        [c.get(1, 0), c.get(1, 1), c.get(1, 2), c.get(1, 3)],
        [c.get(2, 0), c.get(2, 1), c.get(2, 2), c.get(2, 3)],
        [c.get(3, 0), c.get(3, 1), c.get(3, 2), c.get(3, 3)],
    ];

    let row_divisors = [
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&cm[0], divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&cm[1], divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&cm[2], divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&cm[3], divisor, max_entry))
            .collect::<Vec<_>>(),
    ];
    for &d0 in &row_divisors[0] {
        for &d1 in &row_divisors[1] {
            for &d2 in &row_divisors[2] {
                for &d3 in &row_divisors[3] {
                    let diag = [d0, d1, d2, d3];
                    if d0 == d1 && d1 == d2 && d2 == d3 {
                        continue;
                    }
                    if divide_rows_by_diag_4x4(&cm, &diag, max_entry)
                        .is_some_and(|x| scale_cols_by_diag_4x4(&x, &diag) != cm)
                    {
                        return true;
                    }
                }
            }
        }
    }

    let col0 = [cm[0][0], cm[1][0], cm[2][0], cm[3][0]];
    let col1 = [cm[0][1], cm[1][1], cm[2][1], cm[3][1]];
    let col2 = [cm[0][2], cm[1][2], cm[2][2], cm[3][2]];
    let col3 = [cm[0][3], cm[1][3], cm[2][3], cm[3][3]];
    let col_divisors = [
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&col0, divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&col1, divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&col2, divisor, max_entry))
            .collect::<Vec<_>>(),
        (1..=diag_cap)
            .filter(|&divisor| row_admits_diagonal_divisor(&col3, divisor, max_entry))
            .collect::<Vec<_>>(),
    ];
    for &d0 in &col_divisors[0] {
        for &d1 in &col_divisors[1] {
            for &d2 in &col_divisors[2] {
                for &d3 in &col_divisors[3] {
                    let diag = [d0, d1, d2, d3];
                    if d0 == d1 && d1 == d2 && d2 == d3 {
                        continue;
                    }
                    if divide_cols_by_diag_4x4(&cm, &diag, max_entry)
                        .is_some_and(|x| scale_rows_by_diag_4x4(&x, &diag) != cm)
                    {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Generate a narrow 3x3 -> 3x3 diagonal-refactorization family:
/// A = D*X -> B = X*D or A = X*D -> B = D*X, where D is positive diagonal.
///
/// This deliberately stays small. It only tries tiny diagonal entries, skips
/// scalar diagonals, and only emits nontrivial same-size moves whose factors
/// stay within the standard entry bound.
fn visit_diagonal_refactorizations_3x3<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    if !diagonal_refactorization_3x3_family_is_nonempty(c, max_entry) {
        return;
    }
    let diag_cap = max_entry.min(DIAGONAL_REFACTORIZATION_3X3_MAX_DIAG_ENTRY);

    let mut cm = [[0u32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            cm[i][j] = c.get(i, j);
        }
    }

    for d0 in 1..=diag_cap {
        for d1 in 1..=diag_cap {
            for d2 in 1..=diag_cap {
                let diag = [d0, d1, d2];
                if diag == [1, 1, 1] || (d0 == d1 && d1 == d2) {
                    continue;
                }

                if let Some(x) = divide_rows_by_diag_3x3(&cm, &diag, max_entry) {
                    if scale_cols_by_diag_3x3(&x, &diag) != cm {
                        visit(diag3_to_dyn(diag), mat3_u32_to_dyn(&x));
                    }
                }

                if let Some(x) = divide_cols_by_diag_3x3(&cm, &diag, max_entry) {
                    if scale_rows_by_diag_3x3(&x, &diag) != cm {
                        visit(mat3_u32_to_dyn(&x), diag3_to_dyn(diag));
                    }
                }
            }
        }
    }
}

/// Generate a narrow 4x4 -> 4x4 diagonal-refactorization family:
/// A = D*X -> B = X*D or A = X*D -> B = D*X, where D is positive diagonal.
///
/// The 4x4 follow-up stays intentionally tighter than the landed 3x3 slice:
/// it only tries binary diagonals with entries in {1, 2}, skips the scalar
/// case, and only emits nontrivial same-size moves whose factors stay within
/// the standard entry bound.
fn visit_diagonal_refactorizations_4x4<F>(c: &DynMatrix, max_entry: u32, visit: &mut F)
where
    F: FnMut(DynMatrix, DynMatrix),
{
    assert_eq!(c.rows, 4);
    assert_eq!(c.cols, 4);

    if !diagonal_refactorization_4x4_family_is_nonempty(c, max_entry) {
        return;
    }
    let diag_cap = max_entry.min(DIAGONAL_REFACTORIZATION_4X4_MAX_DIAG_ENTRY);

    let mut cm = [[0u32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            cm[i][j] = c.get(i, j);
        }
    }

    for d0 in 1..=diag_cap {
        for d1 in 1..=diag_cap {
            for d2 in 1..=diag_cap {
                for d3 in 1..=diag_cap {
                    let diag = [d0, d1, d2, d3];
                    if diag == [1, 1, 1, 1] || (d0 == d1 && d1 == d2 && d2 == d3) {
                        continue;
                    }

                    if let Some(x) = divide_rows_by_diag_4x4(&cm, &diag, max_entry) {
                        if scale_cols_by_diag_4x4(&x, &diag) != cm {
                            visit(diag4_to_dyn(diag), mat4_u32_to_dyn(&x));
                        }
                    }

                    if let Some(x) = divide_cols_by_diag_4x4(&cm, &diag, max_entry) {
                        if scale_rows_by_diag_4x4(&x, &diag) != cm {
                            visit(mat4_u32_to_dyn(&x), diag4_to_dyn(diag));
                        }
                    }
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

fn mat3_u32_to_dyn(m: &[[u32; 3]; 3]) -> DynMatrix {
    let data: Vec<u32> = m.iter().flat_map(|r| r.iter()).copied().collect();
    DynMatrix::new(3, 3, data)
}

fn diag3_to_dyn(diag: [u32; 3]) -> DynMatrix {
    DynMatrix::new(3, 3, vec![diag[0], 0, 0, 0, diag[1], 0, 0, 0, diag[2]])
}

fn mat4_u32_to_dyn(m: &[[u32; 4]; 4]) -> DynMatrix {
    let data: Vec<u32> = m.iter().flat_map(|r| r.iter()).copied().collect();
    DynMatrix::new(4, 4, data)
}

fn diag4_to_dyn(diag: [u32; 4]) -> DynMatrix {
    DynMatrix::new(
        4,
        4,
        vec![
            diag[0], 0, 0, 0, 0, diag[1], 0, 0, 0, 0, diag[2], 0, 0, 0, 0, diag[3],
        ],
    )
}

fn divide_rows_by_diag_3x3(
    m: &[[u32; 3]; 3],
    diag: &[u32; 3],
    max_entry: u32,
) -> Option<[[u32; 3]; 3]> {
    let mut out = [[0u32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let value = m[i][j];
            let scale = diag[i];
            if value % scale != 0 {
                return None;
            }
            let quotient = value / scale;
            if quotient > max_entry {
                return None;
            }
            out[i][j] = quotient;
        }
    }
    Some(out)
}

fn divide_cols_by_diag_3x3(
    m: &[[u32; 3]; 3],
    diag: &[u32; 3],
    max_entry: u32,
) -> Option<[[u32; 3]; 3]> {
    let mut out = [[0u32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let value = m[i][j];
            let scale = diag[j];
            if value % scale != 0 {
                return None;
            }
            let quotient = value / scale;
            if quotient > max_entry {
                return None;
            }
            out[i][j] = quotient;
        }
    }
    Some(out)
}

fn scale_rows_by_diag_3x3(m: &[[u32; 3]; 3], diag: &[u32; 3]) -> [[u32; 3]; 3] {
    let mut out = [[0u32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = m[i][j] * diag[i];
        }
    }
    out
}

fn scale_cols_by_diag_3x3(m: &[[u32; 3]; 3], diag: &[u32; 3]) -> [[u32; 3]; 3] {
    let mut out = [[0u32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = m[i][j] * diag[j];
        }
    }
    out
}

fn divide_rows_by_diag_4x4(
    m: &[[u32; 4]; 4],
    diag: &[u32; 4],
    max_entry: u32,
) -> Option<[[u32; 4]; 4]> {
    let mut out = [[0u32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let value = m[i][j];
            let scale = diag[i];
            if value % scale != 0 {
                return None;
            }
            let quotient = value / scale;
            if quotient > max_entry {
                return None;
            }
            out[i][j] = quotient;
        }
    }
    Some(out)
}

fn divide_cols_by_diag_4x4(
    m: &[[u32; 4]; 4],
    diag: &[u32; 4],
    max_entry: u32,
) -> Option<[[u32; 4]; 4]> {
    let mut out = [[0u32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let value = m[i][j];
            let scale = diag[j];
            if value % scale != 0 {
                return None;
            }
            let quotient = value / scale;
            if quotient > max_entry {
                return None;
            }
            out[i][j] = quotient;
        }
    }
    Some(out)
}

fn scale_rows_by_diag_4x4(m: &[[u32; 4]; 4], diag: &[u32; 4]) -> [[u32; 4]; 4] {
    let mut out = [[0u32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            out[i][j] = m[i][j] * diag[i];
        }
    }
    out
}

fn scale_cols_by_diag_4x4(m: &[[u32; 4]; 4], diag: &[u32; 4]) -> [[u32; 4]; 4] {
    let mut out = [[0u32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            out[i][j] = m[i][j] * diag[j];
        }
    }
    out
}

// --- Square 3×3 factorisation ---

/// Solve A·x = b where A is 3×3 (given as rows), for nonneg integer x with
/// entries ≤ max_entry. If the system is full-rank, there is at most one solution.
/// If rank-2, reduces to solve_nonneg_2x3 + verification of the remaining equation.
fn solve_nonneg_3x3(a: &[[i64; 3]; 3], b: &[i64; 3], max_entry: u32) -> Vec<[u32; 3]> {
    let (adjugate, det) = adjugate_matrix_and_det_3x3(a);

    if det != 0 {
        return solve_nonneg_3x3_with_adjugate(&adjugate, det, b, max_entry)
            .map_or_else(Vec::new, |x| vec![x]);
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
        let mut filtered = Vec::new();
        solve_nonneg_2x3_into(&rows, &b_sub, max_entry, &mut filtered);
        filtered.retain(|x| {
            let check = a[r_check][0] * x[0] as i64
                + a[r_check][1] * x[1] as i64
                + a[r_check][2] * x[2] as i64;
            check == b[r_check]
        });
        return filtered;
    }

    // Rank ≤ 1: skip (degenerate, rarely contributes useful factorisations).
    vec![]
}

fn adjugate_matrix_3x3(a: &[[i64; 3]; 3]) -> [[i64; 3]; 3] {
    [
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
    ]
}

fn adjugate_matrix_and_det_3x3(a: &[[i64; 3]; 3]) -> ([[i64; 3]; 3], i64) {
    let adjugate = adjugate_matrix_3x3(a);
    let det = a[0][0] * adjugate[0][0] + a[0][1] * adjugate[1][0] + a[0][2] * adjugate[2][0];
    (adjugate, det)
}

fn solve_nonneg_3x3_with_adjugate(
    adjugate: &[[i64; 3]; 3],
    det: i64,
    b: &[i64; 3],
    max_entry: u32,
) -> Option<[u32; 3]> {
    if det == 0 {
        return None;
    }

    let me = max_entry as i64;
    let mut x = [0u32; 3];
    for i in 0..3 {
        let num = adjugate[i][0] * b[0] + adjugate[i][1] * b[1] + adjugate[i][2] * b[2];
        if num % det != 0 {
            return None;
        }
        let value = num / det;
        if value < 0 || value > me {
            return None;
        }
        x[i] = value as u32;
    }
    Some(x)
}

/// Compute the determinant of a 3×3 matrix given as rows.
fn det3x3(m: &[[i64; 3]; 3]) -> i64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn cofactor_matrix_and_det_4x4(a: &[[i64; 4]; 4]) -> ([[i64; 4]; 4], i64) {
    let a00 = a[0][0];
    let a01 = a[0][1];
    let a02 = a[0][2];
    let a03 = a[0][3];
    let a10 = a[1][0];
    let a11 = a[1][1];
    let a12 = a[1][2];
    let a13 = a[1][3];
    let a20 = a[2][0];
    let a21 = a[2][1];
    let a22 = a[2][2];
    let a23 = a[2][3];
    let a30 = a[3][0];
    let a31 = a[3][1];
    let a32 = a[3][2];
    let a33 = a[3][3];

    let m23_23 = a22 * a33 - a23 * a32;
    let m23_13 = a21 * a33 - a23 * a31;
    let m23_12 = a21 * a32 - a22 * a31;
    let m23_03 = a20 * a33 - a23 * a30;
    let m23_02 = a20 * a32 - a22 * a30;
    let m23_01 = a20 * a31 - a21 * a30;

    let m13_23 = a12 * a33 - a13 * a32;
    let m13_13 = a11 * a33 - a13 * a31;
    let m13_12 = a11 * a32 - a12 * a31;
    let m13_03 = a10 * a33 - a13 * a30;
    let m13_02 = a10 * a32 - a12 * a30;
    let m13_01 = a10 * a31 - a11 * a30;

    let m12_23 = a12 * a23 - a13 * a22;
    let m12_13 = a11 * a23 - a13 * a21;
    let m12_12 = a11 * a22 - a12 * a21;
    let m12_03 = a10 * a23 - a13 * a20;
    let m12_02 = a10 * a22 - a12 * a20;
    let m12_01 = a10 * a21 - a11 * a20;

    let cofactors = [
        [
            a11 * m23_23 - a12 * m23_13 + a13 * m23_12,
            -a10 * m23_23 + a12 * m23_03 - a13 * m23_02,
            a10 * m23_13 - a11 * m23_03 + a13 * m23_01,
            -a10 * m23_12 + a11 * m23_02 - a12 * m23_01,
        ],
        [
            -a01 * m23_23 + a02 * m23_13 - a03 * m23_12,
            a00 * m23_23 - a02 * m23_03 + a03 * m23_02,
            -a00 * m23_13 + a01 * m23_03 - a03 * m23_01,
            a00 * m23_12 - a01 * m23_02 + a02 * m23_01,
        ],
        [
            a01 * m13_23 - a02 * m13_13 + a03 * m13_12,
            -a00 * m13_23 + a02 * m13_03 - a03 * m13_02,
            a00 * m13_13 - a01 * m13_03 + a03 * m13_01,
            -a00 * m13_12 + a01 * m13_02 - a02 * m13_01,
        ],
        [
            -a01 * m12_23 + a02 * m12_13 - a03 * m12_12,
            a00 * m12_23 - a02 * m12_03 + a03 * m12_02,
            -a00 * m12_13 + a01 * m12_03 - a03 * m12_01,
            a00 * m12_12 - a01 * m12_02 + a02 * m12_01,
        ],
    ];
    let det = a[0][0] * cofactors[0][0]
        + a[0][1] * cofactors[0][1]
        + a[0][2] * cofactors[0][2]
        + a[0][3] * cofactors[0][3];
    (cofactors, det)
}

fn solve_nonneg_4x4_with_cofactors(
    cofactors: &[[i64; 4]; 4],
    det: i64,
    b: &[i64; 4],
    max_entry: u32,
) -> Option<[u32; 4]> {
    if det == 0 {
        return None;
    }

    let me = max_entry as i64;
    let mut x = [0u32; 4];
    for i in 0..4 {
        let num = cofactors[0][i] * b[0]
            + cofactors[1][i] * b[1]
            + cofactors[2][i] * b[2]
            + cofactors[3][i] * b[3];
        if num % det != 0 {
            return None;
        }
        let value = num / det;
        if value < 0 || value > me {
            return None;
        }
        x[i] = value as u32;
    }

    Some(x)
}

/// Solve U·x = b where U is 4×4 (given as rows), b is 4-vector.
/// Returns all nonneg integer 4-vectors x with entries ≤ max_entry.
///
/// Algorithm: cofactor expansion for the determinant and adjugate.
/// Falls back to rank-3 reduction via `solve_nonneg_3x3` when singular.
fn solve_nonneg_4x4(a: &[[i64; 4]; 4], b: &[i64; 4], max_entry: u32) -> Vec<[u32; 4]> {
    let (cofactors, det) = cofactor_matrix_and_det_4x4(a);

    if det != 0 {
        return solve_nonneg_4x4_with_cofactors(&cofactors, det, b, max_entry)
            .map_or_else(Vec::new, |x| vec![x]);
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
        const COL_SUBSETS: [[usize; 3]; 4] = [[1, 2, 3], [0, 2, 3], [0, 1, 3], [0, 1, 2]];
        for (free_col, cols) in COL_SUBSETS.iter().enumerate() {
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
    let mut v_col0 = Vec::new();
    let mut v_col1 = Vec::new();
    let mut v_col2 = Vec::new();

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
                solve_nonneg_2x3_into(
                    &u_top,
                    &[c_cols[0][0], c_cols[0][1]],
                    max_entry,
                    &mut v_col0,
                );
                if v_col0.is_empty() {
                    continue;
                }

                solve_nonneg_2x3_into(
                    &u_top,
                    &[c_cols[1][0], c_cols[1][1]],
                    max_entry,
                    &mut v_col1,
                );
                if v_col1.is_empty() {
                    continue;
                }

                solve_nonneg_2x3_into(
                    &u_top,
                    &[c_cols[2][0], c_cols[2][1]],
                    max_entry,
                    &mut v_col2,
                );
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

#[cfg(feature = "research-tools")]
#[derive(Clone, Debug, Default)]
pub struct Square3x3FactorisationBreakdown {
    pub valid_row0_candidates: usize,
    pub row1_candidates_total: usize,
    pub row1_pruned_min_sum: usize,
    pub row1_pruned_gcd: usize,
    pub row1_pruned_col0_empty: usize,
    pub row1_pruned_col1_empty: usize,
    pub row1_pruned_col2_empty: usize,
    pub row1_survived_all_cols: usize,
    pub v_column_combinations: usize,
    pub row2_solution_candidates: usize,
    pub row2_pruned_min_sum: usize,
    pub emitted_factorisations: usize,
}

#[cfg(feature = "research-tools")]
pub fn profile_square_factorisations_3x3_breakdown(
    c: &DynMatrix,
    max_entry: u32,
) -> Square3x3FactorisationBreakdown {
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let me = max_entry;
    let c_cols: [[i64; 3]; 3] = [
        [c.get(0, 0) as i64, c.get(1, 0) as i64, c.get(2, 0) as i64],
        [c.get(0, 1) as i64, c.get(1, 1) as i64, c.get(2, 1) as i64],
        [c.get(0, 2) as i64, c.get(1, 2) as i64, c.get(2, 2) as i64],
    ];
    let c_row2 = [c.get(2, 0) as i64, c.get(2, 1) as i64, c.get(2, 2) as i64];
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

    let mut breakdown = Square3x3FactorisationBreakdown::default();
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
                    for col in &c_cols {
                        if col[0] as u64 % g != 0 {
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
    breakdown.valid_row0_candidates = valid_row0s.len();

    for row0 in valid_row0s {
        profile_sq3_from_row0(
            row0,
            &c_cols,
            &c_row2,
            max_entry,
            &min_row_sum,
            &mut breakdown,
        );
    }

    breakdown
}

#[cfg(feature = "research-tools")]
fn profile_sq3_from_row0(
    row0: [u32; 3],
    c_cols: &[[i64; 3]; 3],
    c_row2: &[i64; 3],
    max_entry: u32,
    min_row_sum: &[u32; 3],
    breakdown: &mut Square3x3FactorisationBreakdown,
) {
    let [u00, u01, u02] = row0;
    let me = max_entry;
    let mut v_col0 = Vec::new();
    let mut v_col1 = Vec::new();
    let mut v_col2 = Vec::new();

    for u10 in 0..=me {
        for u11 in 0..=me {
            for u12 in 0..=me {
                breakdown.row1_candidates_total += 1;
                if u10 + u11 + u12 < min_row_sum[1] {
                    breakdown.row1_pruned_min_sum += 1;
                    continue;
                }

                let g1 = gcd3(u10 as u64, u11 as u64, u12 as u64);
                if g1 > 1 {
                    let mut skip = false;
                    for col in &c_cols[..] {
                        if col[1] as u64 % g1 != 0 {
                            skip = true;
                            break;
                        }
                    }
                    if skip {
                        breakdown.row1_pruned_gcd += 1;
                        continue;
                    }
                }

                let u_top: [[i64; 3]; 2] = [
                    [u00 as i64, u01 as i64, u02 as i64],
                    [u10 as i64, u11 as i64, u12 as i64],
                ];

                solve_nonneg_2x3_into(
                    &u_top,
                    &[c_cols[0][0], c_cols[0][1]],
                    max_entry,
                    &mut v_col0,
                );
                if v_col0.is_empty() {
                    breakdown.row1_pruned_col0_empty += 1;
                    continue;
                }

                solve_nonneg_2x3_into(
                    &u_top,
                    &[c_cols[1][0], c_cols[1][1]],
                    max_entry,
                    &mut v_col1,
                );
                if v_col1.is_empty() {
                    breakdown.row1_pruned_col1_empty += 1;
                    continue;
                }

                solve_nonneg_2x3_into(
                    &u_top,
                    &[c_cols[2][0], c_cols[2][1]],
                    max_entry,
                    &mut v_col2,
                );
                if v_col2.is_empty() {
                    breakdown.row1_pruned_col2_empty += 1;
                    continue;
                }

                breakdown.row1_survived_all_cols += 1;
                breakdown.v_column_combinations += v_col0.len() * v_col1.len() * v_col2.len();

                for vc0 in &v_col0 {
                    for vc1 in &v_col1 {
                        for vc2 in &v_col2 {
                            let vt: [[i64; 3]; 3] = [
                                [vc0[0] as i64, vc0[1] as i64, vc0[2] as i64],
                                [vc1[0] as i64, vc1[1] as i64, vc1[2] as i64],
                                [vc2[0] as i64, vc2[1] as i64, vc2[2] as i64],
                            ];
                            let row2_solutions = solve_nonneg_3x3(&vt, c_row2, max_entry);
                            breakdown.row2_solution_candidates += row2_solutions.len();
                            for [u20, u21, u22] in row2_solutions {
                                if u20 + u21 + u22 < min_row_sum[2] {
                                    breakdown.row2_pruned_min_sum += 1;
                                    continue;
                                }
                                breakdown.emitted_factorisations += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

// --- Family selection seam ---

type FactorisationEnumerator = fn(&DynMatrix, u32, &mut dyn FnMut(DynMatrix, DynMatrix));
type FactorisationFamilyEnabled = fn(usize, usize, MoveFamilyPolicy) -> bool;

#[derive(Clone, Copy)]
struct FactorisationFamilyDescriptor {
    label: &'static str,
    enabled: FactorisationFamilyEnabled,
    enumerate: FactorisationEnumerator,
}

impl FactorisationFamilyDescriptor {
    const fn new(
        label: &'static str,
        enabled: FactorisationFamilyEnabled,
        enumerate: FactorisationEnumerator,
    ) -> Self {
        Self {
            label,
            enabled,
            enumerate,
        }
    }

    fn is_enabled(
        &self,
        input_dim: usize,
        max_intermediate_dim: usize,
        move_family_policy: MoveFamilyPolicy,
    ) -> bool {
        (self.enabled)(input_dim, max_intermediate_dim, move_family_policy)
    }

    fn visit<F>(&self, a: &DynMatrix, max_entry: u32, visit: &mut F)
    where
        F: FnMut(&'static str, DynMatrix, DynMatrix),
    {
        let label = self.label;
        (self.enumerate)(a, max_entry, &mut |u, v| visit(label, u, v));
    }
}

const TWO_BY_TWO_FACTORISATION_FAMILIES: [FactorisationFamilyDescriptor; 2] = [
    FactorisationFamilyDescriptor::new(
        "square_factorisation_2x2",
        enabled_square_factorisation_2x2,
        enumerate_square_factorisation_2x2_family,
    ),
    FactorisationFamilyDescriptor::new(
        "rectangular_factorisation_2x3",
        enabled_rectangular_factorisation_2x3,
        enumerate_rectangular_factorisation_2x3_family,
    ),
];

const THREE_BY_THREE_RECTANGULAR_FAMILIES: [FactorisationFamilyDescriptor; 4] = [
    FactorisationFamilyDescriptor::new(
        "rectangular_factorisation_3x3_to_2",
        enabled_rectangular_factorisation_3x3_to_2,
        enumerate_rectangular_factorisation_3x3_to_2_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_row_split_3x3_to_4x4",
        enabled_single_row_split_3x3_to_4x4,
        enumerate_single_row_split_3x3_to_4x4_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_column_split_3x3_to_4x4",
        enabled_single_column_split_3x3_to_4x4,
        enumerate_single_column_split_3x3_to_4x4_family,
    ),
    FactorisationFamilyDescriptor::new(
        "binary_sparse_rectangular_factorisation_3x3_to_4",
        enabled_binary_sparse_factorisation_3x3_to_4,
        enumerate_binary_sparse_factorisation_3x3_to_4_family,
    ),
];

const THREE_BY_THREE_SAME_DIMENSION_FAMILIES: [FactorisationFamilyDescriptor; 6] = [
    FactorisationFamilyDescriptor::new(
        "square_factorisation_3x3",
        enabled_square_factorisation_3x3,
        enumerate_square_factorisation_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "diagonal_refactorization_3x3",
        enabled_three_by_three_same_dimension_family,
        enumerate_diagonal_refactorization_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "elementary_conjugation_3x3",
        enabled_three_by_three_same_dimension_family,
        enumerate_elementary_conjugation_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "opposite_shear_conjugation_3x3",
        enabled_three_by_three_same_dimension_family,
        enumerate_opposite_shear_conjugation_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "parallel_shear_conjugation_3x3",
        enabled_three_by_three_same_dimension_family,
        enumerate_parallel_shear_conjugation_3x3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "convergent_shear_conjugation_3x3",
        enabled_three_by_three_same_dimension_family,
        enumerate_convergent_shear_conjugation_3x3_family,
    ),
];

const FOUR_BY_FOUR_FACTORISATION_FAMILIES: [FactorisationFamilyDescriptor; 5] = [
    FactorisationFamilyDescriptor::new(
        "binary_sparse_rectangular_factorisation_4x3_to_3",
        enabled_binary_sparse_factorisation_4x4_to_3,
        enumerate_binary_sparse_factorisation_4x4_to_3_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_row_split_4x4_to_5x5",
        enabled_single_row_split_4x4_to_5x5,
        enumerate_single_row_split_4x4_to_5x5_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_column_split_4x4_to_5x5",
        enabled_single_column_split_4x4_to_5x5,
        enumerate_single_column_split_4x4_to_5x5_family,
    ),
    FactorisationFamilyDescriptor::new(
        "binary_sparse_rectangular_factorisation_4x4_to_5",
        enabled_binary_sparse_factorisation_4x4_to_5,
        enumerate_binary_sparse_factorisation_4x4_to_5_family,
    ),
    FactorisationFamilyDescriptor::new(
        "diagonal_refactorization_4x4",
        enabled_four_by_four_same_dimension_family,
        enumerate_diagonal_refactorization_4x4_family,
    ),
];

const FIVE_BY_FIVE_FACTORISATION_FAMILIES: [FactorisationFamilyDescriptor; 3] = [
    FactorisationFamilyDescriptor::new(
        "single_row_amalgamation_5x5_to_4x4",
        enabled_single_row_amalgamation_5x5_to_4x4,
        enumerate_single_row_amalgamation_5x5_to_4x4_family,
    ),
    FactorisationFamilyDescriptor::new(
        "single_column_amalgamation_5x5_to_4x4",
        enabled_single_column_amalgamation_5x5_to_4x4,
        enumerate_single_column_amalgamation_5x5_to_4x4_family,
    ),
    FactorisationFamilyDescriptor::new(
        "binary_sparse_rectangular_factorisation_5x5_to_4",
        enabled_binary_sparse_factorisation_5x5_to_4,
        enumerate_binary_sparse_factorisation_5x5_to_4_family,
    ),
];

const GENERIC_SAME_DIMENSION_CONJUGATION_FAMILIES: [FactorisationFamilyDescriptor; 1] =
    [FactorisationFamilyDescriptor::new(
        "elementary_conjugation",
        enabled_generic_same_dimension_conjugation,
        enumerate_generic_same_dimension_conjugation_family,
    )];

fn enabled_square_factorisation_2x2(
    input_dim: usize,
    _max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 2
}

fn enabled_rectangular_factorisation_2x3(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 2 && max_intermediate_dim >= 3
}

fn enabled_rectangular_factorisation_3x3_to_2(
    input_dim: usize,
    _max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3
}

fn enabled_binary_sparse_factorisation_3x3_to_4(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3 && max_intermediate_dim >= 4
}

fn enabled_single_row_split_3x3_to_4x4(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3 && max_intermediate_dim >= 4
}

fn enabled_single_column_split_3x3_to_4x4(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3 && max_intermediate_dim >= 4
}

fn enabled_square_factorisation_3x3(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3
        && max_intermediate_dim >= 3
        && move_family_policy.includes_square_factorisation_3x3()
}

fn enabled_three_by_three_same_dimension_family(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 3 && max_intermediate_dim >= 3
}

fn enabled_binary_sparse_factorisation_4x4_to_3(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 4
}

fn enabled_single_row_split_4x4_to_5x5(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 5
}

fn enabled_single_column_split_4x4_to_5x5(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 5
}

fn enabled_binary_sparse_factorisation_4x4_to_5(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 5
}

fn enabled_four_by_four_same_dimension_family(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 4 && max_intermediate_dim >= 4
}

fn enabled_single_row_amalgamation_5x5_to_4x4(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 5 && max_intermediate_dim >= 5
}

fn enabled_single_column_amalgamation_5x5_to_4x4(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 5 && max_intermediate_dim >= 5
}

fn enabled_binary_sparse_factorisation_5x5_to_4(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim == 5 && max_intermediate_dim >= 5
}

fn enabled_generic_same_dimension_conjugation(
    input_dim: usize,
    max_intermediate_dim: usize,
    _move_family_policy: MoveFamilyPolicy,
) -> bool {
    input_dim >= 4 && max_intermediate_dim >= input_dim
}

fn enumerate_square_factorisation_2x2_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    let sq: SqMatrix<2> = a
        .to_sq()
        .expect("2x2 family descriptor should only be used for 2x2 inputs");
    visit_square_factorisations_2x2(&sq, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_rectangular_factorisation_2x3_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    let sq: SqMatrix<2> = a
        .to_sq()
        .expect("2x3 family descriptor should only be used for 2x2 inputs");
    visit_rect_factorisations_2x3(&sq, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_rectangular_factorisation_3x3_to_2_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_factorisations_3x3_to_2(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_single_row_split_3x3_to_4x4_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_single_row_split_factorisations_3x3_to_4(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_single_column_split_3x3_to_4x4_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_single_column_split_factorisations_3x3_to_4(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_binary_sparse_factorisation_3x3_to_4_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_binary_sparse_factorisations_3x3_to_4(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_square_factorisation_3x3_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    let sq3_cap = max_entry.min(4);
    visit_square_factorisations_3x3(a, sq3_cap, &mut |u, v| visit(u, v));
}

fn enumerate_diagonal_refactorization_3x3_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_diagonal_refactorizations_3x3(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_elementary_conjugation_3x3_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_elementary_conjugations_3x3(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_opposite_shear_conjugation_3x3_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_opposite_shear_conjugations_3x3(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_parallel_shear_conjugation_3x3_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_parallel_shear_conjugations_3x3(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_convergent_shear_conjugation_3x3_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_convergent_shear_conjugations_3x3(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_binary_sparse_factorisation_4x4_to_3_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_binary_sparse_factorisations_4x4_to_3(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_single_row_split_4x4_to_5x5_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_single_row_split_factorisations_4x4_to_5(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_single_column_split_4x4_to_5x5_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_single_column_split_factorisations_4x4_to_5(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_binary_sparse_factorisation_4x4_to_5_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_binary_sparse_factorisations_4x4_to_5(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_diagonal_refactorization_4x4_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_diagonal_refactorizations_4x4(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_single_row_amalgamation_5x5_to_4x4_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_single_row_amalgamation_factorisations_5x5_to_4(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_single_column_amalgamation_5x5_to_4x4_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_single_column_amalgamation_factorisations_5x5_to_4(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_binary_sparse_factorisation_5x5_to_4_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_binary_sparse_factorisations_5x5_to_4(a, max_entry, &mut |u, v| visit(u, v));
}

fn enumerate_generic_same_dimension_conjugation_family(
    a: &DynMatrix,
    max_entry: u32,
    visit: &mut dyn FnMut(DynMatrix, DynMatrix),
) {
    visit_elementary_conjugations_generic(a, max_entry, &mut |u, v| visit(u, v));
}

fn visit_enabled_factorisation_family_descriptors<F>(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
    mut visit: F,
) where
    F: FnMut(&FactorisationFamilyDescriptor),
{
    let mut visit_group = |families: &[FactorisationFamilyDescriptor]| {
        for family in families {
            if family.is_enabled(input_dim, max_intermediate_dim, move_family_policy) {
                visit(family);
            }
        }
    };

    match input_dim {
        2 => visit_group(&TWO_BY_TWO_FACTORISATION_FAMILIES),
        3 => {
            visit_group(&THREE_BY_THREE_RECTANGULAR_FAMILIES);
            visit_group(&THREE_BY_THREE_SAME_DIMENSION_FAMILIES);
        }
        4 => visit_group(&FOUR_BY_FOUR_FACTORISATION_FAMILIES),
        5 => visit_group(&FIVE_BY_FIVE_FACTORISATION_FAMILIES),
        _ => {}
    }

    if input_dim >= 4 {
        visit_group(&GENERIC_SAME_DIMENSION_CONJUGATION_FAMILIES);
    }
}

fn visit_selected_factorisation_families<F>(
    a: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: MoveFamilyPolicy,
    visit: &mut F,
) where
    F: FnMut(&'static str, DynMatrix, DynMatrix),
{
    visit_enabled_factorisation_family_descriptors(
        a.rows,
        max_intermediate_dim,
        move_family_policy,
        |family| family.visit(a, max_entry, visit),
    );
}

#[cfg(test)]
fn selected_factorisation_family_labels(
    input_dim: usize,
    max_intermediate_dim: usize,
    move_family_policy: MoveFamilyPolicy,
) -> Vec<&'static str> {
    let mut labels = Vec::new();
    visit_enabled_factorisation_family_descriptors(
        input_dim,
        max_intermediate_dim,
        move_family_policy,
        |family| labels.push(family.label),
    );
    labels
}

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
    visit_factorisations_with_family_for_policy(
        a,
        max_intermediate_dim,
        max_entry,
        MoveFamilyPolicy::Mixed,
        |family, u, v| visit(family, u, v),
    );
}

/// Canonicalize a 3x3 square-factorisation witness pair up to simultaneous
/// permutation of the intermediate basis.
///
/// If `(U', V') = (U P, P^{-1} V)` for a 3x3 permutation matrix `P`, then
/// `U'V' = UV` and `V'U'` is permutation-similar to `VU`. This orbit key lets
/// callers drop exact duplicate raw witnesses before materializing `VU`.
pub fn square_factorisation_3x3_permutation_orbit_key(
    u: &DynMatrix,
    v: &DynMatrix,
) -> Option<[u32; 18]> {
    if u.rows != 3 || u.cols != 3 || v.rows != 3 || v.cols != 3 {
        return None;
    }

    const PERMS: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];

    let mut best = permuted_square_factorisation_3x3_pair_data(u, v, &PERMS[0]);
    for perm in PERMS.iter().skip(1) {
        let candidate = permuted_square_factorisation_3x3_pair_data(u, v, perm);
        if candidate < best {
            best = candidate;
        }
    }
    Some(best)
}

fn permuted_square_factorisation_3x3_pair_data(
    u: &DynMatrix,
    v: &DynMatrix,
    perm: &[usize; 3],
) -> [u32; 18] {
    let mut data = [0u32; 18];

    for row in 0..3 {
        let base = row * 3;
        data[base] = u.get(row, perm[0]);
        data[base + 1] = u.get(row, perm[1]);
        data[base + 2] = u.get(row, perm[2]);
    }

    for row in 0..3 {
        let source_row = perm[row];
        let base = 9 + row * 3;
        data[base] = v.get(source_row, 0);
        data[base + 1] = v.get(source_row, 1);
        data[base + 2] = v.get(source_row, 2);
    }

    data
}

/// Canonicalize a `binary_sparse_rectangular_factorisation_3x3_to_4` witness
/// pair up to simultaneous renaming of the intermediate basis that keeps the
/// pair inside the same exact structured family.
///
/// This family has one distinguished slot and three symmetric core slots. Some
/// witnesses also admit multiple valid distinguished-slot presentations. The
/// key therefore minimizes over every intermediate-basis permutation whose
/// renamed witness still satisfies the family's exact structural vocabulary.
pub fn binary_sparse_factorisation_3x3_to_4_orbit_key(
    u: &DynMatrix,
    v: &DynMatrix,
    max_entry: u32,
) -> Option<[u32; 24]> {
    if u.rows != 3 || u.cols != 4 || v.rows != 4 || v.cols != 3 {
        return None;
    }

    const PERMS: [[usize; 4]; 24] = [
        [0, 1, 2, 3],
        [0, 1, 3, 2],
        [0, 2, 1, 3],
        [0, 2, 3, 1],
        [0, 3, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 2, 3],
        [1, 0, 3, 2],
        [1, 2, 0, 3],
        [1, 2, 3, 0],
        [1, 3, 0, 2],
        [1, 3, 2, 0],
        [2, 0, 1, 3],
        [2, 0, 3, 1],
        [2, 1, 0, 3],
        [2, 1, 3, 0],
        [2, 3, 0, 1],
        [2, 3, 1, 0],
        [3, 0, 1, 2],
        [3, 0, 2, 1],
        [3, 1, 0, 2],
        [3, 1, 2, 0],
        [3, 2, 0, 1],
        [3, 2, 1, 0],
    ];

    let mut best = None;
    for perm in PERMS {
        if !binary_sparse_factorisation_3x3_to_4_permuted_pair_is_witness(u, v, &perm, max_entry) {
            continue;
        }
        let candidate = permuted_binary_sparse_factorisation_3x3_to_4_pair_data(u, v, &perm);
        if best.map_or(true, |best_candidate| candidate < best_candidate) {
            best = Some(candidate);
        }
    }

    best
}

fn binary_sparse_factorisation_3x3_to_4_permuted_pair_is_witness(
    u: &DynMatrix,
    v: &DynMatrix,
    perm: &[usize; 4],
    max_entry: u32,
) -> bool {
    let col = |slot: usize| -> [u32; 3] {
        let source = perm[slot];
        [u.get(0, source), u.get(1, source), u.get(2, source)]
    };
    let row = |slot: usize| -> [u32; 3] {
        let source = perm[slot];
        [v.get(source, 0), v.get(source, 1), v.get(source, 2)]
    };

    for distinguished_slot in 0..4 {
        if !is_weighted_sparse_row_len3(col(distinguished_slot), max_entry)
            || !is_binary_sparse_row_len3(row(distinguished_slot))
        {
            continue;
        }

        let mut weighted_core_rows = 0usize;
        let mut ok = true;
        for slot in 0..4 {
            if slot == distinguished_slot {
                continue;
            }
            if !is_binary_sparse_row_len3(col(slot))
                || !is_weighted_sparse_row_len3(row(slot), max_entry)
            {
                ok = false;
                break;
            }
            if !is_binary_sparse_row_len3(row(slot)) {
                weighted_core_rows += 1;
            }
        }

        if ok && weighted_core_rows <= 1 {
            return true;
        }
    }

    false
}

fn permuted_binary_sparse_factorisation_3x3_to_4_pair_data(
    u: &DynMatrix,
    v: &DynMatrix,
    perm: &[usize; 4],
) -> [u32; 24] {
    let mut data = [0u32; 24];

    for row in 0..3 {
        let base = row * 4;
        data[base] = u.get(row, perm[0]);
        data[base + 1] = u.get(row, perm[1]);
        data[base + 2] = u.get(row, perm[2]);
        data[base + 3] = u.get(row, perm[3]);
    }

    for row in 0..4 {
        let source_row = perm[row];
        let base = 12 + row * 3;
        data[base] = v.get(source_row, 0);
        data[base + 1] = v.get(source_row, 1);
        data[base + 2] = v.get(source_row, 2);
    }

    data
}

/// Unified factorisation dispatcher for square matrices.
///
/// Family selection is described by dimension-grouped descriptors so policy
/// gates, stable labels, and enumeration entrypoints stay centralized instead
/// of being repeated in one large `if k == ...` block.
pub fn visit_factorisations_with_family_for_policy<F>(
    a: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
    move_family_policy: MoveFamilyPolicy,
    mut visit: F,
) where
    F: FnMut(&'static str, DynMatrix, DynMatrix),
{
    if !move_family_policy.permits_factorisations() {
        return;
    }

    assert!(a.is_square());
    visit_selected_factorisation_families(
        a,
        max_intermediate_dim,
        max_entry,
        move_family_policy,
        &mut visit,
    );
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
    use crate::types::MoveFamilyPolicy;
    use std::collections::BTreeSet;

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
    fn test_solve_nonneg_3x3_with_adjugate_matches_solver() {
        let cases = [
            (
                [[1, 1, 0], [0, 1, 1], [1, 0, 1]],
                [3, 5, 4],
                10,
                Some([1, 2, 3]),
            ),
            (
                [[2, 0, 1], [1, 1, 0], [0, 1, 1]],
                [7, 3, 4],
                10,
                Some([2, 1, 3]),
            ),
            ([[1, 1, 0], [0, 1, 1], [1, 0, 1]], [1, 1, 1], 10, None),
        ];

        for (a, b, max_entry, expected) in cases {
            let (adjugate, det) = adjugate_matrix_and_det_3x3(&a);
            assert_eq!(
                solve_nonneg_3x3_with_adjugate(&adjugate, det, &b, max_entry),
                expected
            );
            let expected_vec = expected.map_or_else(Vec::new, |x| vec![x]);
            assert_eq!(solve_nonneg_3x3(&a, &b, max_entry), expected_vec);
        }
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
    fn test_visit_all_factorisations_includes_diagonal_refactorization() {
        let u = DynMatrix::new(3, 3, vec![3, 0, 0, 0, 1, 0, 0, 0, 2]);
        let v = DynMatrix::new(3, 3, vec![1, 1, 0, 2, 1, 1, 1, 0, 1]);
        let c = u.mul(&v);
        let mut found = false;

        assert!(diagonal_refactorization_3x3_family_is_nonempty(&c, 6));
        visit_all_factorisations(&c, 3, 6, |cand_u, cand_v| {
            if cand_u == u && cand_v == v {
                found = true;
            }
        });

        assert!(found, "expected diagonal refactorization factorisation");
    }

    #[test]
    fn test_square_factorisation_3x3_permutation_orbit_key_collapses_middle_basis_renaming() {
        let u = DynMatrix::new(3, 3, vec![2, 1, 0, 0, 1, 1, 1, 0, 2]);
        let v = DynMatrix::new(3, 3, vec![1, 0, 1, 2, 1, 0, 0, 1, 1]);
        let perm = [1usize, 2, 0];
        let permuted_u = DynMatrix::new(
            3,
            3,
            vec![
                u.get(0, perm[0]),
                u.get(0, perm[1]),
                u.get(0, perm[2]),
                u.get(1, perm[0]),
                u.get(1, perm[1]),
                u.get(1, perm[2]),
                u.get(2, perm[0]),
                u.get(2, perm[1]),
                u.get(2, perm[2]),
            ],
        );
        let permuted_v = DynMatrix::new(
            3,
            3,
            vec![
                v.get(perm[0], 0),
                v.get(perm[0], 1),
                v.get(perm[0], 2),
                v.get(perm[1], 0),
                v.get(perm[1], 1),
                v.get(perm[1], 2),
                v.get(perm[2], 0),
                v.get(perm[2], 1),
                v.get(perm[2], 2),
            ],
        );

        assert_eq!(u.mul(&v), permuted_u.mul(&permuted_v));
        assert_eq!(
            v.mul(&u).canonical_perm(),
            permuted_v.mul(&permuted_u).canonical_perm()
        );
        assert_eq!(
            square_factorisation_3x3_permutation_orbit_key(&u, &v),
            square_factorisation_3x3_permutation_orbit_key(&permuted_u, &permuted_v)
        );
    }

    #[test]
    fn test_binary_sparse_factorisation_3x3_to_4_orbit_key_collapses_family_preserving_slot_renaming(
    ) {
        let u = DynMatrix::new(3, 4, vec![2, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1]);
        let v = DynMatrix::new(4, 3, vec![1, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1, 0]);
        let perm = [0usize, 2, 3, 1];
        let permuted_u = DynMatrix::new(
            3,
            4,
            vec![
                u.get(0, perm[0]),
                u.get(0, perm[1]),
                u.get(0, perm[2]),
                u.get(0, perm[3]),
                u.get(1, perm[0]),
                u.get(1, perm[1]),
                u.get(1, perm[2]),
                u.get(1, perm[3]),
                u.get(2, perm[0]),
                u.get(2, perm[1]),
                u.get(2, perm[2]),
                u.get(2, perm[3]),
            ],
        );
        let permuted_v = DynMatrix::new(
            4,
            3,
            vec![
                v.get(perm[0], 0),
                v.get(perm[0], 1),
                v.get(perm[0], 2),
                v.get(perm[1], 0),
                v.get(perm[1], 1),
                v.get(perm[1], 2),
                v.get(perm[2], 0),
                v.get(perm[2], 1),
                v.get(perm[2], 2),
                v.get(perm[3], 0),
                v.get(perm[3], 1),
                v.get(perm[3], 2),
            ],
        );

        assert_eq!(u.mul(&v), permuted_u.mul(&permuted_v));
        assert_eq!(
            v.mul(&u).canonical_perm(),
            permuted_v.mul(&permuted_u).canonical_perm()
        );
        assert_eq!(
            binary_sparse_factorisation_3x3_to_4_orbit_key(&u, &v, 6),
            binary_sparse_factorisation_3x3_to_4_orbit_key(&permuted_u, &permuted_v, 6)
        );
    }

    #[test]
    fn test_graph_plus_structured_policy_excludes_square_factorisation_3x3() {
        let u = DynMatrix::new(3, 3, vec![5, 2, 0, 2, 1, 0, 0, 0, 1]);
        let v = DynMatrix::new(3, 3, vec![1, 1, 0, 0, 1, 0, 0, 0, 1]);
        let c = u.mul(&v);
        let mut families = BTreeSet::new();

        visit_factorisations_with_family_for_policy(
            &c,
            3,
            6,
            MoveFamilyPolicy::GraphPlusStructured,
            |family, _, _| {
                families.insert(family);
            },
        );

        assert!(families.contains("opposite_shear_conjugation_3x3"));
        assert!(!families.contains("square_factorisation_3x3"));
    }

    #[test]
    fn test_graph_plus_structured_policy_exposes_diagonal_refactorization_witness() {
        let u = DynMatrix::new(3, 3, vec![3, 0, 0, 0, 1, 0, 0, 0, 2]);
        let v = DynMatrix::new(3, 3, vec![1, 1, 0, 2, 1, 1, 1, 0, 1]);
        let c = u.mul(&v);
        let mut families = BTreeSet::new();

        assert!(diagonal_refactorization_3x3_family_is_nonempty(&c, 6));
        visit_factorisations_with_family_for_policy(
            &c,
            3,
            6,
            MoveFamilyPolicy::GraphPlusStructured,
            |family, _, _| {
                families.insert(family);
            },
        );

        assert!(families.contains("diagonal_refactorization_3x3"));
        assert!(!families.contains("square_factorisation_3x3"));
    }

    #[test]
    fn test_graph_plus_structured_policy_exposes_diagonal_refactorization_4x4_witness() {
        let u = DynMatrix::new(4, 4, vec![2, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1]);
        let v = DynMatrix::new(4, 4, vec![1, 1, 0, 1, 2, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1]);
        let c = u.mul(&v);
        let mut families = BTreeSet::new();

        visit_factorisations_with_family_for_policy(
            &c,
            4,
            4,
            MoveFamilyPolicy::GraphPlusStructured,
            |family, _, _| {
                families.insert(family);
            },
        );

        assert!(families.contains("diagonal_refactorization_4x4"));
    }

    #[test]
    fn test_graph_plus_structured_policy_exposes_single_row_split_4x4_to_5x5_witness() {
        let u = DynMatrix::new(
            4,
            5,
            vec![1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1],
        );
        let v = DynMatrix::new(
            5,
            4,
            vec![1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 2, 1],
        );
        let c = u.mul(&v);
        let mut families = BTreeSet::new();

        visit_factorisations_with_family_for_policy(
            &c,
            5,
            3,
            MoveFamilyPolicy::GraphPlusStructured,
            |family, _, _| {
                families.insert(family);
            },
        );

        assert!(families.contains("single_row_split_4x4_to_5x5"));
    }

    #[test]
    fn test_graph_plus_structured_policy_exposes_single_column_split_4x4_to_5x5_witness() {
        let u = DynMatrix::new(
            4,
            5,
            vec![1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 2, 1, 1, 1, 0, 1],
        );
        let v = DynMatrix::new(
            5,
            4,
            vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
        );
        let c = u.mul(&v);
        let mut families = BTreeSet::new();

        visit_factorisations_with_family_for_policy(
            &c,
            5,
            3,
            MoveFamilyPolicy::GraphPlusStructured,
            |family, _, _| {
                families.insert(family);
            },
        );

        assert!(families.contains("single_column_split_4x4_to_5x5"));
    }

    #[test]
    fn test_graph_plus_structured_policy_exposes_single_row_amalgamation_5x5_to_4x4_witness() {
        let current = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 0, 1, 1, //
                1, 1, 1, 0, 1, //
                0, 1, 1, 1, 0, //
                1, 0, 0, 2, 1,
            ],
        );
        let mut families = BTreeSet::new();

        visit_factorisations_with_family_for_policy(
            &current,
            5,
            3,
            MoveFamilyPolicy::GraphPlusStructured,
            |family, _, _| {
                families.insert(family);
            },
        );

        assert!(families.contains("single_row_amalgamation_5x5_to_4x4"));
    }

    #[test]
    fn test_graph_plus_structured_policy_exposes_single_column_amalgamation_5x5_to_4x4_witness() {
        let current = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 1, 1, 0, //
                1, 0, 1, 1, 0, //
                0, 1, 0, 1, 2, //
                1, 1, 1, 0, 1,
            ],
        );
        let mut families = BTreeSet::new();

        visit_factorisations_with_family_for_policy(
            &current,
            5,
            3,
            MoveFamilyPolicy::GraphPlusStructured,
            |family, _, _| {
                families.insert(family);
            },
        );

        assert!(families.contains("single_column_amalgamation_5x5_to_4x4"));
    }

    #[test]
    fn test_selected_family_labels_for_mixed_3x3_follow_group_order() {
        assert_eq!(
            selected_factorisation_family_labels(3, 4, MoveFamilyPolicy::Mixed),
            vec![
                "rectangular_factorisation_3x3_to_2",
                "single_row_split_3x3_to_4x4",
                "single_column_split_3x3_to_4x4",
                "binary_sparse_rectangular_factorisation_3x3_to_4",
                "square_factorisation_3x3",
                "diagonal_refactorization_3x3",
                "elementary_conjugation_3x3",
                "opposite_shear_conjugation_3x3",
                "parallel_shear_conjugation_3x3",
                "convergent_shear_conjugation_3x3",
            ]
        );
    }

    #[test]
    fn test_selected_family_labels_for_graph_plus_structured_3x3_skip_square_family() {
        assert_eq!(
            selected_factorisation_family_labels(3, 4, MoveFamilyPolicy::GraphPlusStructured),
            vec![
                "rectangular_factorisation_3x3_to_2",
                "single_row_split_3x3_to_4x4",
                "single_column_split_3x3_to_4x4",
                "binary_sparse_rectangular_factorisation_3x3_to_4",
                "diagonal_refactorization_3x3",
                "elementary_conjugation_3x3",
                "opposite_shear_conjugation_3x3",
                "parallel_shear_conjugation_3x3",
                "convergent_shear_conjugation_3x3",
            ]
        );
    }

    #[test]
    fn test_selected_family_labels_for_4x4_keep_specific_before_generic() {
        assert_eq!(
            selected_factorisation_family_labels(4, 5, MoveFamilyPolicy::Mixed),
            vec![
                "binary_sparse_rectangular_factorisation_4x3_to_3",
                "single_row_split_4x4_to_5x5",
                "single_column_split_4x4_to_5x5",
                "binary_sparse_rectangular_factorisation_4x4_to_5",
                "diagonal_refactorization_4x4",
                "elementary_conjugation",
            ]
        );
    }

    #[test]
    fn test_selected_family_labels_for_5x5_keep_specific_before_generic() {
        assert_eq!(
            selected_factorisation_family_labels(5, 5, MoveFamilyPolicy::Mixed),
            vec![
                "single_row_amalgamation_5x5_to_4x4",
                "single_column_amalgamation_5x5_to_4x4",
                "binary_sparse_rectangular_factorisation_5x5_to_4",
                "elementary_conjugation",
            ]
        );
    }

    #[test]
    fn test_contiguous_row_split_duplication_matrix_matches_literature_2x2_to_5x5_template() {
        assert_eq!(
            build_contiguous_row_split_duplication_matrix(&[3, 2]),
            DynMatrix::new(2, 5, vec![1, 1, 1, 0, 0, 0, 0, 0, 1, 1])
        );
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
    fn test_single_row_split_factorisations_reach_expected_3x3_to_4x4_target() {
        let current = DynMatrix::new(3, 3, vec![2, 1, 1, 1, 0, 2, 0, 1, 1]);
        let target = DynMatrix::new(4, 4, vec![1, 1, 0, 1, 1, 1, 1, 0, 1, 1, 0, 2, 0, 0, 1, 1]);
        let mut found = false;

        visit_single_row_split_factorisations_3x3_to_4(&current, 3, &mut |u, v| {
            if u == DynMatrix::new(3, 4, vec![1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])
                && v == DynMatrix::new(4, 3, vec![1, 0, 1, 1, 1, 0, 1, 0, 2, 0, 1, 1])
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected bounded 3x3->4x4 single-row split factorisation"
        );
    }

    #[test]
    fn test_single_row_split_3x3_to_4x4_gate_rejects_low_mass_source() {
        let current = DynMatrix::new(3, 3, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]);
        let mut callbacks = 0usize;

        assert!(!single_row_split_3x3_to_4x4_family_is_nonempty(&current));
        visit_single_row_split_factorisations_3x3_to_4(&current, 3, &mut |_u, _v| {
            callbacks += 1;
        });

        assert_eq!(callbacks, 0);
    }

    #[test]
    fn test_single_row_split_3x3_to_4x4_gate_accepts_known_positive_source() {
        let current = DynMatrix::new(3, 3, vec![2, 1, 1, 1, 0, 2, 0, 1, 1]);
        let mut callbacks = 0usize;

        assert!(single_row_split_3x3_to_4x4_family_is_nonempty(&current));
        visit_single_row_split_factorisations_3x3_to_4(&current, 3, &mut |_u, _v| {
            callbacks += 1;
        });

        assert!(callbacks > 0);
    }

    #[test]
    fn test_single_column_split_factorisations_reach_expected_3x3_to_4x4_target() {
        let current = DynMatrix::new(3, 3, vec![2, 1, 0, 1, 0, 1, 1, 2, 1]);
        let target = DynMatrix::new(4, 4, vec![1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 2, 1]);
        let mut found = false;

        visit_single_column_split_factorisations_3x3_to_4(&current, 3, &mut |u, v| {
            if u == DynMatrix::new(3, 4, vec![1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 2, 1])
                && v == DynMatrix::new(4, 3, vec![1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1])
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected bounded 3x3->4x4 single-column split factorisation"
        );
    }

    #[test]
    fn test_single_column_split_3x3_to_4x4_gate_rejects_low_mass_source() {
        let current = DynMatrix::new(3, 3, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]);
        let mut callbacks = 0usize;

        assert!(!single_column_split_3x3_to_4x4_family_is_nonempty(&current));
        visit_single_column_split_factorisations_3x3_to_4(&current, 3, &mut |_u, _v| {
            callbacks += 1;
        });

        assert_eq!(callbacks, 0);
    }

    #[test]
    fn test_single_column_split_3x3_to_4x4_gate_accepts_known_positive_source() {
        let current = DynMatrix::new(3, 3, vec![2, 1, 0, 1, 0, 1, 1, 2, 1]);
        let mut callbacks = 0usize;

        assert!(single_column_split_3x3_to_4x4_family_is_nonempty(&current));
        visit_single_column_split_factorisations_3x3_to_4(&current, 3, &mut |_u, _v| {
            callbacks += 1;
        });

        assert!(callbacks > 0);
    }

    #[test]
    fn test_diagonal_refactorizations_4x4_reach_expected_target() {
        let current = DynMatrix::new(4, 4, vec![2, 2, 0, 2, 2, 0, 1, 1, 2, 2, 2, 0, 0, 1, 1, 1]);
        let target = DynMatrix::new(4, 4, vec![2, 1, 0, 1, 4, 0, 2, 1, 2, 1, 2, 0, 0, 1, 2, 1]);
        let mut found = false;

        assert!(diagonal_refactorization_4x4_family_is_nonempty(&current, 4));
        visit_diagonal_refactorizations_4x4(&current, 4, &mut |u, v| {
            if u == DynMatrix::new(4, 4, vec![2, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1])
                && v == DynMatrix::new(4, 4, vec![1, 1, 0, 1, 2, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1])
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected bounded 4x4 same-size diagonal refactorization factorisation"
        );
    }

    #[test]
    fn test_diagonal_refactorization_3x3_gate_rejects_impossible_source() {
        let current = DynMatrix::new(3, 3, vec![1, 2, 1, 1, 1, 2, 2, 1, 1]);
        let mut callbacks = 0usize;

        assert!(!diagonal_refactorization_3x3_family_is_nonempty(
            &current, 6
        ));
        visit_diagonal_refactorizations_3x3(&current, 6, &mut |_u, _v| {
            callbacks += 1;
        });

        assert_eq!(callbacks, 0);
    }

    #[test]
    fn test_diagonal_refactorization_3x3_gate_rejects_trivial_commuting_source() {
        let current = DynMatrix::new(3, 3, vec![2, 0, 0, 0, 1, 0, 0, 0, 1]);
        let mut callbacks = 0usize;

        assert!(!diagonal_refactorization_3x3_family_is_nonempty(
            &current, 6
        ));
        visit_diagonal_refactorizations_3x3(&current, 6, &mut |_u, _v| {
            callbacks += 1;
        });

        assert_eq!(callbacks, 0);
    }

    #[test]
    fn test_diagonal_refactorization_4x4_gate_rejects_impossible_source() {
        let current = DynMatrix::new(4, 4, vec![1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 0]);
        let mut callbacks = 0usize;

        assert!(!diagonal_refactorization_4x4_family_is_nonempty(
            &current, 4
        ));
        visit_diagonal_refactorizations_4x4(&current, 4, &mut |_u, _v| {
            callbacks += 1;
        });

        assert_eq!(callbacks, 0);
    }

    #[test]
    fn test_single_row_split_factorisations_reach_expected_4x4_to_5x5_target() {
        let current = DynMatrix::new(4, 4, vec![1, 1, 0, 1, 2, 1, 1, 2, 0, 1, 1, 0, 1, 0, 2, 1]);
        let target = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 0, 1, 1, //
                1, 1, 1, 0, 1, //
                0, 1, 1, 1, 0, //
                1, 0, 0, 2, 1,
            ],
        );
        let mut found = false;

        visit_single_row_split_factorisations_4x4_to_5(&current, 3, &mut |u, v| {
            if u == DynMatrix::new(
                4,
                5,
                vec![1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1],
            ) && v
                == DynMatrix::new(
                    5,
                    4,
                    vec![1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 2, 1],
                )
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected bounded 4x4->5x5 single-row split factorisation"
        );
    }

    #[test]
    fn test_single_column_split_factorisations_reach_expected_4x4_to_5x5_target() {
        let current = DynMatrix::new(4, 4, vec![1, 2, 0, 1, 1, 1, 1, 0, 0, 1, 1, 2, 1, 2, 0, 1]);
        let target = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 1, 1, 0, //
                1, 0, 1, 1, 0, //
                0, 1, 0, 1, 2, //
                1, 1, 1, 0, 1,
            ],
        );
        let mut found = false;

        visit_single_column_split_factorisations_4x4_to_5(&current, 3, &mut |u, v| {
            if u == DynMatrix::new(
                4,
                5,
                vec![1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 2, 1, 1, 1, 0, 1],
            ) && v
                == DynMatrix::new(
                    5,
                    4,
                    vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
                )
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected bounded 4x4->5x5 single-column split factorisation"
        );
    }

    #[test]
    fn test_single_row_amalgamation_factorisations_reach_expected_5x5_to_4x4_target() {
        let current = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 0, 1, 1, //
                1, 1, 1, 0, 1, //
                0, 1, 1, 1, 0, //
                1, 0, 0, 2, 1,
            ],
        );
        let target = DynMatrix::new(4, 4, vec![1, 1, 0, 1, 2, 1, 1, 2, 0, 1, 1, 0, 1, 0, 2, 1]);
        let mut found = false;

        visit_single_row_amalgamation_factorisations_5x5_to_4(&current, 3, &mut |u, v| {
            if u == DynMatrix::new(
                5,
                4,
                vec![1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 2, 1],
            ) && v
                == DynMatrix::new(
                    4,
                    5,
                    vec![1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1],
                )
                && u.mul(&v) == current
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected bounded 5x5->4x4 single-row amalgamation factorisation"
        );
    }

    #[test]
    fn test_single_column_amalgamation_factorisations_reach_expected_5x5_to_4x4_target() {
        let current = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 1, 1, 0, //
                1, 0, 1, 1, 0, //
                0, 1, 0, 1, 2, //
                1, 1, 1, 0, 1,
            ],
        );
        let target = DynMatrix::new(4, 4, vec![1, 2, 0, 1, 1, 1, 1, 0, 0, 1, 1, 2, 1, 2, 0, 1]);
        let mut found = false;

        visit_single_column_amalgamation_factorisations_5x5_to_4(&current, 3, &mut |u, v| {
            if u == DynMatrix::new(
                5,
                4,
                vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
            ) && v
                == DynMatrix::new(
                    4,
                    5,
                    vec![1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 2, 1, 1, 1, 0, 1],
                )
                && u.mul(&v) == current
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected bounded 5x5->4x4 single-column amalgamation factorisation"
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

    #[test]
    fn test_visit_all_factorisations_includes_single_row_split_3x3_to_4x4_family() {
        let current = DynMatrix::new(3, 3, vec![2, 1, 1, 1, 0, 2, 0, 1, 1]);
        let target = DynMatrix::new(4, 4, vec![1, 1, 0, 1, 1, 1, 1, 0, 1, 1, 0, 2, 0, 0, 1, 1]);
        let mut found = false;

        visit_all_factorisations_with_family(&current, 4, 3, |family, u, v| {
            if family == "single_row_split_3x3_to_4x4"
                && u == DynMatrix::new(3, 4, vec![1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])
                && v == DynMatrix::new(4, 3, vec![1, 0, 1, 1, 1, 0, 1, 0, 2, 0, 1, 1])
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the bounded 3x3->4x4 row-split family"
        );
    }

    #[test]
    fn test_visit_all_factorisations_includes_single_column_split_3x3_to_4x4_family() {
        let current = DynMatrix::new(3, 3, vec![2, 1, 0, 1, 0, 1, 1, 2, 1]);
        let target = DynMatrix::new(4, 4, vec![1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 2, 1]);
        let mut found = false;

        visit_all_factorisations_with_family(&current, 4, 3, |family, u, v| {
            if family == "single_column_split_3x3_to_4x4"
                && u == DynMatrix::new(3, 4, vec![1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 2, 1])
                && v == DynMatrix::new(4, 3, vec![1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1])
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the bounded 3x3->4x4 column-split family"
        );
    }

    #[test]
    fn test_visit_all_factorisations_includes_diagonal_refactorization_4x4_family() {
        let current = DynMatrix::new(4, 4, vec![2, 2, 0, 2, 2, 0, 1, 1, 2, 2, 2, 0, 0, 1, 1, 1]);
        let target = DynMatrix::new(4, 4, vec![2, 1, 0, 1, 4, 0, 2, 1, 2, 1, 2, 0, 0, 1, 2, 1]);
        let mut found = false;

        visit_all_factorisations_with_family(&current, 4, 4, |family, u, v| {
            if family == "diagonal_refactorization_4x4"
                && u == DynMatrix::new(4, 4, vec![2, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1])
                && v == DynMatrix::new(4, 4, vec![1, 1, 0, 1, 2, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1])
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the bounded 4x4 same-size diagonal family"
        );
    }

    #[test]
    fn test_visit_all_factorisations_includes_single_row_split_4x4_to_5x5_family() {
        let current = DynMatrix::new(4, 4, vec![1, 1, 0, 1, 2, 1, 1, 2, 0, 1, 1, 0, 1, 0, 2, 1]);
        let target = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 0, 1, 1, //
                1, 1, 1, 0, 1, //
                0, 1, 1, 1, 0, //
                1, 0, 0, 2, 1,
            ],
        );
        let mut found = false;

        visit_all_factorisations_with_family(&current, 5, 3, |family, u, v| {
            if family == "single_row_split_4x4_to_5x5"
                && u == DynMatrix::new(
                    4,
                    5,
                    vec![1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1],
                )
                && v == DynMatrix::new(
                    5,
                    4,
                    vec![1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 2, 1],
                )
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the bounded 4x4->5x5 row-split family"
        );
    }

    #[test]
    fn test_visit_all_factorisations_includes_single_column_split_4x4_to_5x5_family() {
        let current = DynMatrix::new(4, 4, vec![1, 2, 0, 1, 1, 1, 1, 0, 0, 1, 1, 2, 1, 2, 0, 1]);
        let target = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 1, 1, 0, //
                1, 0, 1, 1, 0, //
                0, 1, 0, 1, 2, //
                1, 1, 1, 0, 1,
            ],
        );
        let mut found = false;

        visit_all_factorisations_with_family(&current, 5, 3, |family, u, v| {
            if family == "single_column_split_4x4_to_5x5"
                && u == DynMatrix::new(
                    4,
                    5,
                    vec![1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 2, 1, 1, 1, 0, 1],
                )
                && v == DynMatrix::new(
                    5,
                    4,
                    vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
                )
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the bounded 4x4->5x5 column-split family"
        );
    }

    #[test]
    fn test_visit_all_factorisations_includes_single_row_amalgamation_5x5_to_4x4_family() {
        let current = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 0, 1, 1, //
                1, 1, 1, 0, 1, //
                0, 1, 1, 1, 0, //
                1, 0, 0, 2, 1,
            ],
        );
        let target = DynMatrix::new(4, 4, vec![1, 1, 0, 1, 2, 1, 1, 2, 0, 1, 1, 0, 1, 0, 2, 1]);
        let mut found = false;

        visit_all_factorisations_with_family(&current, 5, 3, |family, u, v| {
            if family == "single_row_amalgamation_5x5_to_4x4"
                && u == DynMatrix::new(
                    5,
                    4,
                    vec![1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 2, 1],
                )
                && v == DynMatrix::new(
                    4,
                    5,
                    vec![1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1],
                )
                && u.mul(&v) == current
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the bounded 5x5->4x4 row-amalgamation family"
        );
    }

    #[test]
    fn test_visit_all_factorisations_includes_single_column_amalgamation_5x5_to_4x4_family() {
        let current = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 0, 1, //
                1, 0, 1, 1, 0, //
                1, 0, 1, 1, 0, //
                0, 1, 0, 1, 2, //
                1, 1, 1, 0, 1,
            ],
        );
        let target = DynMatrix::new(4, 4, vec![1, 2, 0, 1, 1, 1, 1, 0, 0, 1, 1, 2, 1, 2, 0, 1]);
        let mut found = false;

        visit_all_factorisations_with_family(&current, 5, 3, |family, u, v| {
            if family == "single_column_amalgamation_5x5_to_4x4"
                && u == DynMatrix::new(
                    5,
                    4,
                    vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
                )
                && v == DynMatrix::new(
                    4,
                    5,
                    vec![1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 2, 1, 1, 1, 0, 1],
                )
                && u.mul(&v) == current
                && v.mul(&u) == target
            {
                found = true;
            }
        });

        assert!(
            found,
            "expected main dispatcher to expose the bounded 5x5->4x4 column-amalgamation family"
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
    fn test_cofactor_matrix_and_det_4x4_matches_reference() {
        fn reference(a: &[[i64; 4]; 4]) -> ([[i64; 4]; 4], i64) {
            let minor = |r: usize, c: usize| -> [[i64; 3]; 3] {
                let mut m = [[0i64; 3]; 3];
                let mut mi = 0;
                for (i, row) in a.iter().enumerate() {
                    if i == r {
                        continue;
                    }
                    let mut mj = 0;
                    for (j, &value) in row.iter().enumerate() {
                        if j == c {
                            continue;
                        }
                        m[mi][mj] = value;
                        mj += 1;
                    }
                    mi += 1;
                }
                m
            };

            let cofactors = [
                [
                    det3x3(&minor(0, 0)),
                    -det3x3(&minor(0, 1)),
                    det3x3(&minor(0, 2)),
                    -det3x3(&minor(0, 3)),
                ],
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
            let det = a[0][0] * cofactors[0][0]
                + a[0][1] * cofactors[0][1]
                + a[0][2] * cofactors[0][2]
                + a[0][3] * cofactors[0][3];
            (cofactors, det)
        }

        let cases = [
            [[1, 0, 2, 3], [0, 1, 4, 5], [6, 7, 1, 0], [2, 3, 4, 1]],
            [[1, 2, 3, 4], [2, 4, 6, 8], [0, 1, 1, 0], [3, 1, 0, 2]],
            [[0, 1, 0, 1], [2, 0, 3, 0], [1, 4, 0, 2], [0, 1, 5, 1]],
        ];

        for case in cases {
            assert_eq!(cofactor_matrix_and_det_4x4(&case), reference(&case));
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
