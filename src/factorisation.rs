use crate::matrix::{DynMatrix, SqMatrix};

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
                    results.push((u, v));
                }
            }
        }
    }
    results
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

// --- Rectangular factorisation enumerators ---

/// Enumerate all factorisations A = UV where U is 2×3, V is 3×2,
/// with all entries in 0..=max_entry.
pub fn enumerate_rect_factorisations_2x3(
    a: &SqMatrix<2>,
    max_entry: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    let mut results = Vec::new();
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
                if g0 > 1
                    && (a.data[0][0] as u64 % g0 != 0 || a.data[0][1] as u64 % g0 != 0)
                {
                    continue;
                }

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
                            if g1 > 1
                                && (a.data[1][0] as u64 % g1 != 0
                                    || a.data[1][1] as u64 % g1 != 0)
                            {
                                continue;
                            }

                            let u_rows: [[i64; 3]; 2] = [
                                [u00 as i64, u01 as i64, u02 as i64],
                                [u10 as i64, u11 as i64, u12 as i64],
                            ];

                            // Quick rank check: all 2x2 minors zero → rank < 2.
                            let d01 = u_rows[0][0] * u_rows[1][1] - u_rows[0][1] * u_rows[1][0];
                            let d02 = u_rows[0][0] * u_rows[1][2] - u_rows[0][2] * u_rows[1][0];
                            let d12 = u_rows[0][1] * u_rows[1][2] - u_rows[0][2] * u_rows[1][1];
                            if d01 == 0 && d02 == 0 && d12 == 0 {
                                continue;
                            }

                            // Solve for each column of V.
                            let col0_solutions = solve_nonneg_2x3(&u_rows, &a_cols[0], max_entry);
                            if col0_solutions.is_empty() {
                                continue;
                            }
                            let col1_solutions = solve_nonneg_2x3(&u_rows, &a_cols[1], max_entry);
                            if col1_solutions.is_empty() {
                                continue;
                            }

                            // Cartesian product of column solutions.
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
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);
    let mut results = Vec::new();
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

            for u10 in 0..=me {
                for u11 in 0..=me {
                    if u10 + u11 < min_row_sum[1] {
                        continue;
                    }

                    // Constraint propagation: gcd(row1) must divide every C[1, j].
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
                        // FAST PATH: rows 0 and 1 of U are linearly independent.
                        //
                        // V is uniquely determined by the 2×2 sub-system
                        //   [u00, u01; u10, u11] · V[:, j] = C[0..2, j]   (Cramer's rule).
                        // Once V is known, row 2 of U is determined (O(1)) by solving the
                        // overdetermined system  V^T · [u20; u21] = C[2, :].
                        // This eliminates the inner (u20, u21) enumeration loop entirely.
                        let mut v_cols = [[0i64; 2]; 3];
                        let mut v_valid = true;
                        for j in 0..3 {
                            let b0 = c_cols[j][0]; // C[0, j]
                            let b1 = c_cols[j][1]; // C[1, j]
                            // Cramer: v0 = (u11·b0 − u01·b1) / det01
                            //         v1 = (u00·b1 − u10·b0) / det01
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

                        // Derive row 2 of U: solve V^T · [u20; u21] = C[2, :],
                        // i.e. for each j: V[0,j]·u20 + V[1,j]·u21 = C[2, j].
                        // v_rows[j] = [V[0, j], V[1, j]] = v_cols[j].
                        let v_rows: [[i64; 2]; 3] = [
                            [v_cols[0][0], v_cols[0][1]],
                            [v_cols[1][0], v_cols[1][1]],
                            [v_cols[2][0], v_cols[2][1]],
                        ];
                        if let Some([u20, u21]) =
                            solve_overdetermined_3x2(&v_rows, &c_row2, max_entry)
                        {
                            if u20 + u21 >= min_row_sum[2] {
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
                        } else {
                            // solve_overdetermined_3x2 returned None.  This happens when
                            // V is rank-deficient (all 2×2 minors of v_rows are zero),
                            // giving infinitely many candidate row-2 values.  Fall back to
                            // explicit enumeration in that rare case.
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
                                            if (u20 as i64) * v_cols[j][0]
                                                + (u21 as i64) * v_cols[j][1]
                                                != c_cols[j][2]
                                            {
                                                ok = false;
                                                break;
                                            }
                                        }
                                        if ok {
                                            let u_mat = DynMatrix::new(
                                                3,
                                                2,
                                                vec![u00, u01, u10, u11, u20, u21],
                                            );
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
                            // If v_rows has rank 2 but None was returned the system is
                            // inconsistent; no valid row 2 exists for this (row0, row1).
                        }
                    } else {
                        // FALLBACK: rows 0 and 1 of U are linearly dependent (det01 = 0).
                        // Enumerate row 2 and use the full overdetermined solver for V.
                        for u20 in 0..=me {
                            for u21 in 0..=me {
                                if u20 + u21 < min_row_sum[2] {
                                    continue;
                                }

                                // Constraint propagation: gcd(row2) must divide every C[2,j].
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

                                // det01 = 0 already; rank requires d02 or d12 nonzero.
                                let d02 = u_rows[0][0] * u_rows[2][1]
                                    - u_rows[0][1] * u_rows[2][0];
                                let d12 = u_rows[1][0] * u_rows[2][1]
                                    - u_rows[1][1] * u_rows[2][0];
                                if d02 == 0 && d12 == 0 {
                                    continue;
                                }

                                // Solve for each column of V (overdetermined: 3 eqns, 2 unknowns).
                                let v0 = match solve_overdetermined_3x2(
                                    &u_rows,
                                    &c_cols[0],
                                    max_entry,
                                ) {
                                    Some(v) => v,
                                    None => continue,
                                };
                                let v1 = match solve_overdetermined_3x2(
                                    &u_rows,
                                    &c_cols[1],
                                    max_entry,
                                ) {
                                    Some(v) => v,
                                    None => continue,
                                };
                                let v2 = match solve_overdetermined_3x2(
                                    &u_rows,
                                    &c_cols[2],
                                    max_entry,
                                ) {
                                    Some(v) => v,
                                    None => continue,
                                };

                                let u_mat =
                                    DynMatrix::new(3, 2, vec![u00, u01, u10, u11, u20, u21]);
                                let v_mat = DynMatrix::new(
                                    2,
                                    3,
                                    vec![v0[0], v1[0], v2[0], v0[1], v1[1], v2[1]],
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
///   - k=3, m=2: rectangular 3×2 × 2×3 (the return trip)
///   - k=3, m=3: skipped (too expensive)
pub fn enumerate_all_factorisations(
    a: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    assert!(a.is_square());
    let k = a.rows;
    let mut results = Vec::new();

    if k == 2 {
        // Square factorisations (m=2).
        let sq: SqMatrix<2> = a.to_sq().unwrap();
        results.extend(enumerate_square_factorisations_2x2(&sq, max_entry));

        // Rectangular factorisations for m=3..=max_intermediate_dim.
        if max_intermediate_dim >= 3 {
            results.extend(enumerate_rect_factorisations_2x3(&sq, max_entry));
        }
    } else if k == 3 {
        // Only rectangular to dimension 2 (the return trip). Skip square 3×3.
        results.extend(enumerate_factorisations_3x3_to_2(a, max_entry));
    }

    results
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
}
