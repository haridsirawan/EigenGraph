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
}
