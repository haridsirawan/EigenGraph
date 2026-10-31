//! linalg.rs — Robust self-contained linear algebra, equation solvers, and SVD decomposition for EigenGraph.

use std::ops::{Index, IndexMut};

/// Dense Matrix of 64-bit floating point numbers
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl Matrix {
    /// Creates a matrix filled with zeros
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    /// Creates an identity matrix of size n x n
    pub fn identity(n: usize) -> Self {
        let mut mat = Self::zeros(n, n);
        for i in 0..n {
            mat[(i, i)] = 1.0;
        }
        mat
    }

    /// Creates a matrix from 1D row-major slice
    pub fn from_slice(rows: usize, cols: usize, slice: &[f64]) -> Self {
        assert_eq!(rows * cols, slice.len(), "Slice length does not match rows * cols");
        Self {
            rows,
            cols,
            data: slice.to_vec(),
        }
    }

    /// Matrix multiplication C = A * B
    pub fn mul(&self, other: &Matrix) -> Self {
        assert_eq!(self.cols, other.rows, "Matrix dimension mismatch for multiplication: ({}x{}) * ({}x{})", self.rows, self.cols, other.rows, other.cols);
        let mut result = Self::zeros(self.rows, other.cols);
        for i in 0..self.rows {
            for k in 0..self.cols {
                let a_ik = self[(i, k)];
                if a_ik.abs() > 1e-18 {
                    for j in 0..other.cols {
                        result[(i, j)] += a_ik * other[(k, j)];
                    }
                }
            }
        }
        result
    }

    /// Matrix-vector multiplication y = A * x
    pub fn mul_vec(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(self.cols, x.len(), "Dimension mismatch in mul_vec: {} cols vs {} vector length", self.cols, x.len());
        let mut y = vec![0.0; self.rows];
        for i in 0..self.rows {
            let mut sum = 0.0;
            for j in 0..self.cols {
                sum += self[(i, j)] * x[j];
            }
            y[i] = sum;
        }
        y
    }

    /// Matrix transpose A^T
    pub fn transpose(&self) -> Self {
        let mut t = Self::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                t[(j, i)] = self[(i, j)];
            }
        }
        t
    }

    /// Frobenius norm of the matrix
    pub fn norm(&self) -> f64 {
        self.data.iter().map(|&x| x * x).sum::<f64>().sqrt()
    }

    /// Element access helper: mat.get(r, c)
    #[inline]
    pub fn get(&self, r: usize, c: usize) -> f64 {
        self[(r, c)]
    }

    /// Computes matrix inverse A^(-1).
    /// Automatically attempts fast Symmetric Positive-Definite (SPD) Cholesky inversion first,
    /// falling back to column-by-column Gaussian elimination with partial pivoting for general matrices.
    /// Returns None if matrix is singular or non-square.
    pub fn inverse(&self) -> Option<Matrix> {
        if self.rows != self.cols {
            return None;
        }
        if let Some(chol) = self.cholesky_decompose() {
            return Some(chol.inverse());
        }
        let n = self.rows;
        let mut inv = Matrix::zeros(n, n);
        let mut e = vec![0.0; n];
        for j in 0..n {
            e[j] = 1.0;
            let col_sol = self.solve_gaussian(&e)?;
            for i in 0..n {
                inv[(i, j)] = col_sol[i];
            }
            e[j] = 0.0;
        }
        Some(inv)
    }

    /// Computes the Cholesky decomposition A = L * L^T for a Symmetric Positive-Definite (SPD) matrix.
    /// Returns None if the matrix is not square, not symmetric within tolerance, or not positive-definite.
    pub fn cholesky_decompose(&self) -> Option<CholeskyFactor> {
        if self.rows != self.cols || self.rows == 0 {
            return None;
        }
        let n = self.rows;
        let mut l = Matrix::zeros(n, n);

        for i in 0..n {
            for j in 0..=i {
                // Quick symmetry check on upper/lower entries
                let a_ij = self[(i, j)];
                let a_ji = self[(j, i)];
                if (a_ij - a_ji).abs() > 1e-7 * (a_ij.abs() + a_ji.abs() + 1.0) {
                    return None;
                }

                let mut sum = 0.0;
                for k in 0..j {
                    sum += l[(i, k)] * l[(j, k)];
                }

                if i == j {
                    let diag = a_ij - sum;
                    if diag <= 1e-13 {
                        return None;
                    }
                    l[(i, i)] = diag.sqrt();
                } else {
                    l[(i, j)] = (a_ij - sum) / l[(j, j)];
                }
            }
        }

        Some(CholeskyFactor { l })
    }

    /// Solves a Symmetric Positive-Definite (SPD) system A * x = b using Cholesky L * L^T factorization
    /// with automatic fallback to Sparse PCG (for large matrices) or Gaussian elimination.
    pub fn solve_spd(&self, b: &[f64]) -> Option<Vec<f64>> {
        if self.rows != self.cols || self.rows != b.len() {
            return None;
        }
        if self.rows > 240 {
            let csr = CsrMatrix::from_dense(self, 1e-13);
            if let Some(sol) = csr.solve_pcg(b, 1e-10, self.rows * 2) {
                return Some(sol);
            }
        }
        if let Some(chol) = self.cholesky_decompose() {
            return Some(chol.solve(b));
        }
        self.solve_gaussian(b)
    }

    /// Solves linear system A * x = b using Gaussian elimination with partial pivoting.
    /// Returns None if matrix is singular.
    pub fn solve_gaussian(&self, b: &[f64]) -> Option<Vec<f64>> {
        assert_eq!(self.rows, self.cols, "Matrix must be square to solve A*x = b");
        assert_eq!(self.rows, b.len(), "Dimension mismatch between matrix rows and RHS vector");
        let n = self.rows;

        // Augmented matrix [A | b]
        let mut aug = Matrix::zeros(n, n + 1);
        for i in 0..n {
            for j in 0..n {
                aug[(i, j)] = self[(i, j)];
            }
            aug[(i, n)] = b[i];
        }

        // Forward elimination with partial pivoting
        for col in 0..n {
            let mut max_row = col;
            let mut max_val = aug[(col, col)].abs();
            for r in (col + 1)..n {
                let val = aug[(r, col)].abs();
                if val > max_val {
                    max_val = val;
                    max_row = r;
                }
            }

            if max_val < 1e-12 {
                // Singular matrix
                return None;
            }

            // Swap rows
            if max_row != col {
                for c in 0..=n {
                    let tmp = aug[(col, c)];
                    aug[(col, c)] = aug[(max_row, c)];
                    aug[(max_row, c)] = tmp;
                }
            }

            // Pivot normalization
            let pivot = aug[(col, col)];
            for r in (col + 1)..n {
                let factor = aug[(r, col)] / pivot;
                aug[(r, col)] = 0.0;
                for c in (col + 1)..=n {
                    aug[(r, c)] -= factor * aug[(col, c)];
                }
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = aug[(i, n)];
            for j in (i + 1)..n {
                sum -= aug[(i, j)] * x[j];
            }
            let pivot = aug[(i, i)];
            if pivot.abs() < 1e-12 {
                return None;
            }
            x[i] = sum / pivot;
        }

        Some(x)
    }

    /// Computes the Singular Value Decomposition (SVD): A = U * Sigma * V^T
    /// using Two-Sided Jacobi Rotation algorithm (unconditionally stable for symmetric & general matrices).
    /// Returns (U, singular_values, V).
    pub fn svd(&self) -> (Matrix, Vec<f64>, Matrix) {
        let m = self.rows;
        let n = self.cols;
        let mut u = self.clone();
        let mut v = Matrix::identity(n);
        let mut singular_values = vec![0.0; n];

        // One-sided Hestenes Jacobi SVD on U
        let max_sweeps = 50;
        let tol = 1e-14;

        for _sweep in 0..max_sweeps {
            let mut max_err: f64 = 0.0;

            for i in 0..n {
                for j in (i + 1)..n {
                    // Compute inner products of columns i and j
                    let mut alpha = 0.0;
                    let mut beta = 0.0;
                    let mut gamma = 0.0;

                    for k in 0..m {
                        let ui = u[(k, i)];
                        let uj = u[(k, j)];
                        alpha += ui * ui;
                        beta += uj * uj;
                        gamma += ui * uj;
                    }

                    max_err = max_err.max(gamma.abs() / ((alpha * beta).sqrt() + 1e-20));

                    if gamma.abs() > 1e-15 {
                        let zeta = (beta - alpha) / (2.0 * gamma);
                        let t = if zeta >= 0.0 {
                            1.0 / (zeta + (1.0 + zeta * zeta).sqrt())
                        } else {
                            -1.0 / (-zeta + (1.0 + zeta * zeta).sqrt())
                        };
                        let c = 1.0 / (1.0 + t * t).sqrt();
                        let s = t * c;

                        // Rotate columns of U
                        for k in 0..m {
                            let ui = u[(k, i)];
                            let uj = u[(k, j)];
                            u[(k, i)] = c * ui - s * uj;
                            u[(k, j)] = s * ui + c * uj;
                        }

                        // Rotate columns of V
                        for k in 0..n {
                            let vi = v[(k, i)];
                            let vj = v[(k, j)];
                            v[(k, i)] = c * vi - s * vj;
                            v[(k, j)] = s * vi + c * vj;
                        }
                    }
                }
            }

            if max_err < tol {
                break;
            }
        }

        // Normalize columns of U to get singular values
        for j in 0..n {
            let mut col_norm = 0.0;
            for i in 0..m {
                col_norm += u[(i, j)] * u[(i, j)];
            }
            col_norm = col_norm.sqrt();
            singular_values[j] = col_norm;

            if col_norm > 1e-14 {
                for i in 0..m {
                    u[(i, j)] /= col_norm;
                }
            }
        }

        // Sort singular values descending
        for i in 0..n {
            let mut max_idx = i;
            for j in (i + 1)..n {
                if singular_values[j] > singular_values[max_idx] {
                    max_idx = j;
                }
            }
            if max_idx != i {
                singular_values.swap(i, max_idx);
                // Swap columns of U
                for k in 0..m {
                    let tmp = u[(k, i)];
                    u[(k, i)] = u[(k, max_idx)];
                    u[(k, max_idx)] = tmp;
                }
                // Swap columns of V
                for k in 0..n {
                    let tmp = v[(k, i)];
                    v[(k, i)] = v[(k, max_idx)];
                    v[(k, max_idx)] = tmp;
                }
            }
        }

        (u, singular_values, v)
    }

    /// Evaluates the rank and nullspace dimension based on tolerance epsilon
    pub fn rank_and_nullspace(&self, tol: f64) -> (usize, usize, Vec<Vec<f64>>) {
        let (_, s, v) = self.svd();
        let max_s = s.first().copied().unwrap_or(0.0);
        let cutoff = tol * max_s;

        let mut rank = 0;
        let mut null_vectors = Vec::new();

        for (idx, &sigma) in s.iter().enumerate() {
            if sigma > cutoff {
                rank += 1;
            } else {
                // Vector in nullspace (column of V)
                let mut null_v = vec![0.0; v.rows];
                for r in 0..v.rows {
                    null_v[r] = v[(r, idx)];
                }
                null_vectors.push(null_v);
            }
        }

        let nullspace_dim = s.len() - rank;
        (rank, nullspace_dim, null_vectors)
    }

    /// Evaluates the 2x2 Woodbury Capacitance Matrix C = I - H for Rank-2 (N-2) simultaneous member loss:
    /// C = [[1 - h11, -h12], [-h12, 1 - h22]]
    /// Returns (det(C), Option<C^(-1)>).
    pub fn woodbury_rank2_det_and_inv(h11: f64, h22: f64, h12: f64, tol: f64) -> (f64, Option<[[f64; 2]; 2]>) {
        let c00 = 1.0 - h11;
        let c11 = 1.0 - h22;
        let c01 = -h12;
        let det = c00 * c11 - c01 * c01;
        if det <= tol {
            (det, None)
        } else {
            let inv_det = 1.0 / det;
            let inv = [
                [c11 * inv_det, -c01 * inv_det],
                [-c01 * inv_det, c00 * inv_det],
            ];
            (det, Some(inv))
        }
    }
}

/// Pre-factored Cholesky Decomposition (A = L * L^T) for rapid O(N^2) forward/backward substitution
/// across multiple right-hand sides (e.g., Newmark-Beta transient time-stepping and Sherman-Morrison inversion).
#[derive(Debug, Clone, PartialEq)]
pub struct CholeskyFactor {
    pub l: Matrix,
}

impl CholeskyFactor {
    /// Solves A * x = (L * L^T) * x = b in O(N^2) time using forward substitution (L * y = b)
    /// followed by backward substitution (L^T * x = y).
    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = self.l.rows;
        assert_eq!(n, b.len(), "Dimension mismatch in CholeskyFactor::solve");

        // 1. Forward substitution: L * y = b
        let mut y = vec![0.0; n];
        for i in 0..n {
            let mut sum = b[i];
            for j in 0..i {
                sum -= self.l[(i, j)] * y[j];
            }
            y[i] = sum / self.l[(i, i)];
        }

        // 2. Backward substitution: L^T * x = y
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = y[i];
            for j in (i + 1)..n {
                sum -= self.l[(j, i)] * x[j];
            }
            x[i] = sum / self.l[(i, i)];
        }

        x
    }

    /// Computes the exact inverse A^(-1) = (L * L^T)^(-1) using column-wise Cholesky back-substitution.
    pub fn inverse(&self) -> Matrix {
        let n = self.l.rows;
        let mut inv = Matrix::zeros(n, n);
        let mut e = vec![0.0; n];
        for j in 0..n {
            e[j] = 1.0;
            let col = self.solve(&e);
            for i in 0..n {
                inv[(i, j)] = col[i];
            }
            e[j] = 0.0;
        }
        inv
    }

    /// Computes log(det(A)) = 2 * sum(ln(L_ii)) without floating-point overflow.
    pub fn log_determinant(&self) -> f64 {
        let mut sum_ln = 0.0;
        for i in 0..self.l.rows {
            sum_ln += self.l[(i, i)].ln();
        }
        2.0 * sum_ln
    }
}

/// Compressed Sparse Row (CSR) Matrix for high-speed O(nnz) sparse linear algebra
#[derive(Debug, Clone, PartialEq)]
pub struct CsrMatrix {
    pub rows: usize,
    pub cols: usize,
    pub row_ptr: Vec<usize>,
    pub col_idx: Vec<usize>,
    pub values: Vec<f64>,
}

impl CsrMatrix {
    /// Converts a dense Matrix into Compressed Sparse Row (CSR) format, dropping entries with |a_ij| <= drop_tol.
    pub fn from_dense(mat: &Matrix, drop_tol: f64) -> Self {
        let mut row_ptr = Vec::with_capacity(mat.rows + 1);
        let mut col_idx = Vec::new();
        let mut values = Vec::new();

        row_ptr.push(0);
        for i in 0..mat.rows {
            for j in 0..mat.cols {
                let val = mat[(i, j)];
                if val.abs() > drop_tol {
                    col_idx.push(j);
                    values.push(val);
                }
            }
            row_ptr.push(values.len());
        }

        Self {
            rows: mat.rows,
            cols: mat.cols,
            row_ptr,
            col_idx,
            values,
        }
    }

    /// Converts CSR matrix back to a dense Matrix.
    pub fn to_dense(&self) -> Matrix {
        let mut mat = Matrix::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            let start = self.row_ptr[i];
            let end = self.row_ptr[i + 1];
            for idx in start..end {
                mat[(i, self.col_idx[idx])] = self.values[idx];
            }
        }
        mat
    }

    /// Number of non-zero elements stored.
    #[inline]
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// Sparse Matrix-Vector Multiplication: y = A * x in O(nnz) time.
    pub fn spmv(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(self.cols, x.len(), "Dimension mismatch in CsrMatrix::spmv");
        let mut y = vec![0.0; self.rows];
        for i in 0..self.rows {
            let start = self.row_ptr[i];
            let end = self.row_ptr[i + 1];
            let mut sum = 0.0;
            for idx in start..end {
                sum += self.values[idx] * x[self.col_idx[idx]];
            }
            y[i] = sum;
        }
        y
    }

    /// Extracts main diagonal vector for Jacobi preconditioning.
    pub fn diagonal(&self) -> Vec<f64> {
        let n = self.rows.min(self.cols);
        let mut diag = vec![0.0; n];
        for i in 0..n {
            let start = self.row_ptr[i];
            let end = self.row_ptr[i + 1];
            for idx in start..end {
                if self.col_idx[idx] == i {
                    diag[i] = self.values[idx];
                    break;
                }
            }
        }
        diag
    }

    /// Solves Symmetric Positive-Definite (SPD) system A * x = b using Jacobi-Preconditioned Conjugate Gradient (PCG).
    pub fn solve_pcg(&self, b: &[f64], tol: f64, max_iter: usize) -> Option<Vec<f64>> {
        if self.rows != self.cols || self.rows != b.len() {
            return None;
        }
        let n = self.rows;
        if n == 0 {
            return Some(vec![]);
        }

        let diag = self.diagonal();
        let mut inv_diag = vec![1.0; n];
        for i in 0..n {
            if diag[i].abs() < 1e-14 {
                return None; // Zero diagonal on SPD matrix -> singular
            }
            inv_diag[i] = 1.0 / diag[i];
        }

        let mut x = vec![0.0; n];
        let ax = self.spmv(&x);
        let mut r: Vec<f64> = b.iter().zip(ax.iter()).map(|(&bi, &axi)| bi - axi).collect();

        let b_norm: f64 = b.iter().map(|&v| v * v).sum::<f64>().sqrt();
        if b_norm < 1e-15 {
            return Some(x);
        }

        let mut z: Vec<f64> = r.iter().zip(inv_diag.iter()).map(|(&ri, &di)| ri * di).collect();
        let mut p = z.clone();
        let mut rz_old: f64 = r.iter().zip(z.iter()).map(|(&ri, &zi)| ri * zi).sum();

        for _iter in 0..max_iter {
            let ap = self.spmv(&p);
            let p_ap: f64 = p.iter().zip(ap.iter()).map(|(&pi, &api)| pi * api).sum();
            if p_ap.abs() < 1e-20 {
                break;
            }

            let alpha = rz_old / p_ap;
            let mut r_norm_sq = 0.0;
            for i in 0..n {
                x[i] += alpha * p[i];
                r[i] -= alpha * ap[i];
                r_norm_sq += r[i] * r[i];
            }

            if r_norm_sq.sqrt() / b_norm < tol {
                return Some(x);
            }

            for i in 0..n {
                z[i] = r[i] * inv_diag[i];
            }
            let rz_new: f64 = r.iter().zip(z.iter()).map(|(&ri, &zi)| ri * zi).sum();
            let beta = rz_new / (rz_old + 1e-30);
            for i in 0..n {
                p[i] = z[i] + beta * p[i];
            }
            rz_old = rz_new;
        }

        let final_r_norm: f64 = r.iter().map(|&ri| ri * ri).sum::<f64>().sqrt();
        if final_r_norm / b_norm < tol * 100.0 {
            Some(x)
        } else {
            None
        }
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = f64;
    #[inline]
    fn index(&self, (r, c): (usize, usize)) -> &Self::Output {
        assert!(r < self.rows && c < self.cols, "Index out of bounds: ({}, {}) in ({}x{}) matrix", r, c, self.rows, self.cols);
        &self.data[r * self.cols + c]
    }
}

impl IndexMut<(usize, usize)> for Matrix {
    #[inline]
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut Self::Output {
        assert!(r < self.rows && c < self.cols, "Index out of bounds: ({}, {}) in ({}x{}) matrix", r, c, self.rows, self.cols);
        &mut self.data[r * self.cols + c]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_mul() {
        let a = Matrix::from_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let b = Matrix::from_slice(2, 2, &[2.0, 0.0, 1.0, 2.0]);
        let c = a.mul(&b);
        assert_eq!(c.data, vec![4.0, 4.0, 10.0, 8.0]);
    }

    #[test]
    fn test_gaussian_solve() {
        let a = Matrix::from_slice(2, 2, &[2.0, 1.0, 1.0, 3.0]);
        let b = vec![5.0, 10.0];
        let x = a.solve_gaussian(&b).expect("System should be solvable");
        assert!((x[0] - 1.0).abs() < 1e-6);
        assert!((x[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_svd_rank() {
        // Singular matrix with identical rows (rank = 1)
        let a = Matrix::from_slice(2, 2, &[2.0, 2.0, 2.0, 2.0]);
        let (rank, null_dim, _) = a.rank_and_nullspace(1e-7);
        assert_eq!(rank, 1);
        assert_eq!(null_dim, 1);
    }

    #[test]
    fn test_csr_sparse_and_pcg_solver() {
        // 3x3 Symmetric Positive-Definite Tridiagonal Stiffness Matrix
        let dense = Matrix::from_slice(3, 3, &[
            4.0, -1.0,  0.0,
           -1.0,  4.0, -1.0,
            0.0, -1.0,  4.0,
        ]);
        let csr = CsrMatrix::from_dense(&dense, 1e-12);
        assert_eq!(csr.nnz(), 7);
        assert_eq!(csr.to_dense(), dense);

        let b = vec![3.0, 2.0, 3.0];
        let x_gauss = dense.solve_gaussian(&b).unwrap();
        let x_pcg = csr.solve_pcg(&b, 1e-10, 100).unwrap();

        for i in 0..3 {
            assert!((x_gauss[i] - x_pcg[i]).abs() < 1e-7);
        }
    }

    #[test]
    fn test_woodbury_rank2_determinant() {
        // Two independent members (h12 = 0, h11 = 0.5, h22 = 0.5) -> det = 0.25
        let (det, inv) = Matrix::woodbury_rank2_det_and_inv(0.5, 0.5, 0.0, 1e-8);
        assert!((det - 0.25).abs() < 1e-9);
        assert!(inv.is_some());

        // Coupled parallel mechanism: h11 = 0.5, h22 = 0.5, h12 = 0.5 -> det = 0.25 - 0.25 = 0.0 (singular!)
        let (det_sing, inv_sing) = Matrix::woodbury_rank2_det_and_inv(0.5, 0.5, 0.5, 1e-8);
        assert!(det_sing.abs() < 1e-9);
        assert!(inv_sing.is_none());
    }

    #[test]
    fn test_cholesky_decomposition_and_spd_inverse() {
        let spd = Matrix::from_slice(3, 3, &[
            6.0, -2.0,  1.0,
           -2.0,  5.0, -1.0,
            1.0, -1.0,  4.0,
        ]);
        let chol = spd.cholesky_decompose().expect("SPD matrix should factorize via Cholesky");
        let reconstructed = chol.l.mul(&chol.l.transpose());
        for i in 0..3 {
            for j in 0..3 {
                assert!((spd[(i, j)] - reconstructed[(i, j)]).abs() < 1e-11);
            }
        }

        let b = vec![5.0, 2.0, 4.0];
        let x_chol = chol.solve(&b);
        let x_gauss = spd.solve_gaussian(&b).unwrap();
        for i in 0..3 {
            assert!((x_chol[i] - x_gauss[i]).abs() < 1e-11);
        }

        let inv = chol.inverse();
        let prod = spd.mul(&inv);
        let eye = Matrix::identity(3);
        for i in 0..3 {
            for j in 0..3 {
                assert!((prod[(i, j)] - eye[(i, j)]).abs() < 1e-11);
            }
        }
    }
}
