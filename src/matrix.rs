use std::fmt;
use std::hash::{Hash, Hasher};

/// Square matrix with nonneg integer entries, parameterised by size.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct SqMatrix<const N: usize> {
    pub data: [[u32; N]; N],
}

impl<const N: usize> Hash for SqMatrix<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for row in &self.data {
            for &val in row {
                val.hash(state);
            }
        }
    }
}

impl<const N: usize> fmt::Debug for SqMatrix<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, row) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", row)?;
        }
        write!(f, "]")
    }
}

impl<const N: usize> SqMatrix<N> {
    pub fn new(data: [[u32; N]; N]) -> Self {
        Self { data }
    }

    pub fn identity() -> Self {
        let mut data = [[0u32; N]; N];
        for i in 0..N {
            data[i][i] = 1;
        }
        Self { data }
    }

    pub fn trace(&self) -> u64 {
        let mut sum = 0u64;
        for i in 0..N {
            sum += self.data[i][i] as u64;
        }
        sum
    }

    /// Multiply two square matrices, returning u64 entries to avoid overflow.
    pub fn mul(&self, other: &Self) -> SqMatrix64<N> {
        let mut result = [[0i64; N]; N];
        for i in 0..N {
            for k in 0..N {
                let a = self.data[i][k] as i64;
                for j in 0..N {
                    result[i][j] += a * other.data[k][j] as i64;
                }
            }
        }
        SqMatrix64 { data: result }
    }

    /// Multiply returning u32 (panics on overflow in debug mode).
    pub fn mul_u32(&self, other: &Self) -> Self {
        let wide = self.mul(other);
        let mut data = [[0u32; N]; N];
        for i in 0..N {
            for j in 0..N {
                assert!(wide.data[i][j] >= 0 && wide.data[i][j] <= u32::MAX as i64);
                data[i][j] = wide.data[i][j] as u32;
            }
        }
        Self { data }
    }

    /// Matrix power (exponentiation by squaring), returns u32 matrix.
    pub fn pow(&self, mut exp: u32) -> Self {
        let mut result = Self::identity();
        let mut base = self.clone();
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.mul_u32(&base);
            }
            base = base.mul_u32(&base);
            exp >>= 1;
        }
        result
    }

    /// Whether all off-diagonal entries are positive (primitive/irreducible for 2x2).
    pub fn is_irreducible(&self) -> bool {
        for i in 0..N {
            for j in 0..N {
                if i != j && self.data[i][j] == 0 {
                    return false;
                }
            }
        }
        true
    }

    /// Sum of all entries.
    pub fn entry_sum(&self) -> u64 {
        let mut sum = 0u64;
        for row in &self.data {
            for &v in row {
                sum += v as u64;
            }
        }
        sum
    }

    /// Maximum entry value.
    pub fn max_entry(&self) -> u32 {
        self.data.iter().flat_map(|row| row.iter()).copied().max().unwrap_or(0)
    }
}

// --- 2x2 specialisations ---

impl SqMatrix<2> {
    /// Determinant for 2x2: ad - bc (as i64 since it can be negative).
    pub fn det(&self) -> i64 {
        let [[a, b], [c, d]] = self.data;
        a as i64 * d as i64 - b as i64 * c as i64
    }

    /// Canonical form: lexicographic min over permutation-similarity orbit.
    /// For 2x2, the orbit under conjugation by the permutation matrix [[0,1],[1,0]]
    /// is {[[a,b],[c,d]], [[d,c],[b,a]]}. Return the lexicographically smaller one.
    pub fn canonical(&self) -> Self {
        let [[a, b], [c, d]] = self.data;
        let conjugated = Self::new([[d, c], [b, a]]);
        if conjugated < *self {
            conjugated
        } else {
            self.clone()
        }
    }

    /// Trace sequence using Newton recurrence for 2x2:
    /// tr(A^k) = tr(A) * tr(A^{k-1}) - det(A) * tr(A^{k-2})
    /// Returns tr(A^1), tr(A^2), ..., tr(A^k) as i64 values.
    pub fn trace_sequence(&self, k: usize) -> Vec<i64> {
        if k == 0 {
            return vec![];
        }
        let tr = self.trace() as i64;
        let det = self.det();
        let mut seq = Vec::with_capacity(k);
        seq.push(tr); // tr(A^1)
        if k == 1 {
            return seq;
        }
        seq.push(tr * tr - 2 * det); // tr(A^2) = tr(A)^2 - 2*det(A)
        for i in 2..k {
            let next = tr * seq[i - 1] - det * seq[i - 2];
            seq.push(next);
        }
        seq
    }
}

// --- i64 square matrix (used as intermediate result) ---

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqMatrix64<const N: usize> {
    pub data: [[i64; N]; N],
}

impl<const N: usize> SqMatrix64<N> {
    pub fn trace(&self) -> i64 {
        let mut sum = 0i64;
        for i in 0..N {
            sum += self.data[i][i];
        }
        sum
    }
}

// --- Dynamic rectangular matrix ---

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DynMatrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<u32>,
}

impl DynMatrix {
    pub fn new(rows: usize, cols: usize, data: Vec<u32>) -> Self {
        assert_eq!(data.len(), rows * cols);
        Self { rows, cols, data }
    }

    pub fn get(&self, i: usize, j: usize) -> u32 {
        self.data[i * self.cols + j]
    }

    pub fn set(&mut self, i: usize, j: usize, val: u32) {
        self.data[i * self.cols + j] = val;
    }

    /// Multiply two dynamic matrices. Panics if dimensions don't match.
    pub fn mul(&self, other: &Self) -> Self {
        assert_eq!(self.cols, other.rows);
        let mut result = vec![0u32; self.rows * other.cols];
        for i in 0..self.rows {
            for k in 0..self.cols {
                let a = self.get(i, k) as u64;
                for j in 0..other.cols {
                    result[i * other.cols + j] += (a * other.get(k, j) as u64) as u32;
                }
            }
        }
        Self::new(self.rows, other.cols, result)
    }

    /// Convert a SqMatrix<N> to a DynMatrix.
    pub fn from_sq<const N: usize>(m: &SqMatrix<N>) -> Self {
        let mut data = Vec::with_capacity(N * N);
        for row in &m.data {
            data.extend_from_slice(row);
        }
        Self::new(N, N, data)
    }

    /// Try to convert to SqMatrix<N>. Returns None if dimensions don't match.
    pub fn to_sq<const N: usize>(&self) -> Option<SqMatrix<N>> {
        if self.rows != N || self.cols != N {
            return None;
        }
        let mut data = [[0u32; N]; N];
        for i in 0..N {
            for j in 0..N {
                data[i][j] = self.get(i, j);
            }
        }
        Some(SqMatrix::new(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_multiply() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let id = SqMatrix::<2>::identity();
        assert_eq!(a.mul_u32(&id), a);
        assert_eq!(id.mul_u32(&a), a);
    }

    #[test]
    fn test_multiply() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let ab = a.mul_u32(&b);
        // [[2,1],[1,1]] * [[1,1],[1,2]] = [[3,4],[2,3]]
        assert_eq!(ab, SqMatrix::new([[3, 4], [2, 3]]));
    }

    #[test]
    fn test_trace() {
        let a = SqMatrix::new([[5, 3], [2, 7]]);
        assert_eq!(a.trace(), 12);
    }

    #[test]
    fn test_det_2x2() {
        let a = SqMatrix::new([[5, 3], [2, 7]]);
        assert_eq!(a.det(), 29);

        let b = SqMatrix::new([[1, 2], [3, 4]]);
        assert_eq!(b.det(), -2);
    }

    #[test]
    fn test_pow() {
        let a = SqMatrix::new([[1, 1], [1, 0]]);
        // A^2 = [[2,1],[1,1]], A^3 = [[3,2],[2,1]]
        assert_eq!(a.pow(0), SqMatrix::<2>::identity());
        assert_eq!(a.pow(1), a);
        assert_eq!(a.pow(2), SqMatrix::new([[2, 1], [1, 1]]));
        assert_eq!(a.pow(3), SqMatrix::new([[3, 2], [2, 1]]));
    }

    #[test]
    fn test_canonical_2x2() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        assert_eq!(a.canonical(), b.canonical());
    }

    #[test]
    fn test_canonical_already_minimal() {
        let a = SqMatrix::new([[1, 2], [3, 5]]);
        // conjugated = [[5,3],[2,1]], which is > a
        assert_eq!(a.canonical(), a);
    }

    #[test]
    fn test_is_irreducible() {
        assert!(SqMatrix::new([[2, 1], [1, 1]]).is_irreducible());
        assert!(!SqMatrix::new([[2, 0], [1, 1]]).is_irreducible());
    }

    #[test]
    fn test_trace_sequence() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let seq = a.trace_sequence(5);
        // Verify against direct computation
        assert_eq!(seq[0], a.trace() as i64); // tr(A)
        assert_eq!(seq[1], a.pow(2).trace() as i64); // tr(A^2)
        assert_eq!(seq[2], a.pow(3).trace() as i64); // tr(A^3)
    }

    #[test]
    fn test_dyn_matrix_roundtrip() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let dyn_a = DynMatrix::from_sq(&a);
        let back: SqMatrix<2> = dyn_a.to_sq().unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn test_dyn_matrix_multiply() {
        // U: 2x2, V: 2x2
        let u = DynMatrix::new(2, 2, vec![1, 1, 0, 1]);
        let v = DynMatrix::new(2, 2, vec![1, 0, 1, 1]);
        let uv = u.mul(&v);
        // [[1,1],[0,1]] * [[1,0],[1,1]] = [[2,1],[1,1]]
        assert_eq!(uv, DynMatrix::new(2, 2, vec![2, 1, 1, 1]));
    }

    #[test]
    fn test_entry_sum_and_max() {
        let a = SqMatrix::new([[5, 3], [2, 7]]);
        assert_eq!(a.entry_sum(), 17);
        assert_eq!(a.max_entry(), 7);
    }
}
