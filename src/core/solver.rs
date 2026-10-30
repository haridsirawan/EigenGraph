//! solver.rs — Linear Elastic Finite Element Analysis (FEA) & Stiffness Assembly Engine.

use crate::core::linalg::Matrix;
use crate::core::model::{ElementType, StructuralModel};

/// Result of a completed Finite Element Analysis
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Nodal Displacements (6 DOFs per node: [ux, uy, uz, rx, ry, rz])
    pub nodal_displacements: Vec<[f64; 6]>,
    /// Member internal axial forces in Newtons (Positive = Tension, Negative = Compression)
    pub member_axial_forces: Vec<f64>,
    /// Member capacity in Newtons
    pub member_capacities: Vec<f64>,
    /// Member utilization ratio U = |N| / (phi * Pn)
    pub member_utilizations: Vec<f64>,
    /// Global stiffness matrix (assembled)
    pub global_k: Matrix,
    /// Partitioned Free-DOF Stiffness Matrix K_ff
    pub k_free: Matrix,
    /// Maximum nodal displacement magnitude in mm
    pub max_displacement_mm: f64,
}

pub struct FemSolver;

impl FemSolver {
    /// Solves the linear elastic equilibrium of a StructuralModel
    pub fn solve(model: &StructuralModel) -> Result<AnalysisResult, String> {
        let n_nodes = model.nodes.len();
        if n_nodes == 0 {
            return Err("Model has no nodes to analyze".to_string());
        }

        let dofs_per_node = 6;
        let total_dofs = n_nodes * dofs_per_node;

        // 1. Assemble Global Stiffness Matrix K
        let mut k_global = Matrix::zeros(total_dofs, total_dofs);

        for elem in &model.elements {
            let n1 = model.find_node(elem.start_node)
                .ok_or_else(|| format!("Start node {} not found", elem.start_node))?;
            let n2 = model.find_node(elem.end_node)
                .ok_or_else(|| format!("End node {} not found", elem.end_node))?;

            let l = n1.distance_to(n2);
            if l < 1e-6 {
                return Err(format!("Element {} has near-zero length", elem.id));
            }

            // Direction cosines
            let cx = (n2.x - n1.x) / l;
            let cy = (n2.y - n1.y) / l;
            let cz = (n2.z - n1.z) / l;

            // Element local-to-global stiffness
            let k_elem = match elem.element_type {
                ElementType::Truss3D => Self::element_stiffness_truss(elem.section.area, elem.material.elastic_modulus, l, cx, cy, cz),
                ElementType::Frame3D => Self::element_stiffness_frame(
                    elem.section.area,
                    elem.section.iy,
                    elem.section.iz,
                    elem.section.j_torsion,
                    elem.material.elastic_modulus,
                    elem.material.shear_modulus(),
                    l, cx, cy, cz
                ),
            };

            // Map into global DOFs
            let node1_idx = model.nodes.iter().position(|n| n.id == elem.start_node).unwrap();
            let node2_idx = model.nodes.iter().position(|n| n.id == elem.end_node).unwrap();

            let dof_map = [
                node1_idx * 6, node1_idx * 6 + 1, node1_idx * 6 + 2, node1_idx * 6 + 3, node1_idx * 6 + 4, node1_idx * 6 + 5,
                node2_idx * 6, node2_idx * 6 + 1, node2_idx * 6 + 2, node2_idx * 6 + 3, node2_idx * 6 + 4, node2_idx * 6 + 5,
            ];

            for i in 0..12 {
                for j in 0..12 {
                    k_global[(dof_map[i], dof_map[j])] += k_elem[(i, j)];
                }
            }
        }

        // 2. Assemble Load Vector F
        let mut f_global = vec![0.0; total_dofs];
        for load in &model.loads {
            if let Some(idx) = model.nodes.iter().position(|n| n.id == load.node_id) {
                f_global[idx * 6] += load.fx;
                f_global[idx * 6 + 1] += load.fy;
                f_global[idx * 6 + 2] += load.fz;
                f_global[idx * 6 + 3] += load.mx;
                f_global[idx * 6 + 4] += load.my;
                f_global[idx * 6 + 5] += load.mz;
            }
        }

        // 3. Partition Free Degrees of Freedom (K_ff * u_ff = F_f)
        let mut free_dofs = Vec::new();
        for (idx, node) in model.nodes.iter().enumerate() {
            let base_dof = idx * 6;
            let restraints = [
                (base_dof, node.fixity.ux),
                (base_dof + 1, node.fixity.uy),
                (base_dof + 2, node.fixity.uz),
                (base_dof + 3, node.fixity.rx),
                (base_dof + 4, node.fixity.ry),
                (base_dof + 5, node.fixity.rz),
            ];

            for (dof, is_fixed) in restraints {
                if !is_fixed {
                    free_dofs.push(dof);
                }
            }
        }

        let n_free = free_dofs.len();
        if n_free == 0 {
            return Err("Model has no free degrees of freedom".to_string());
        }

        // Submatrix K_ff and subvector F_f
        let mut k_ff = Matrix::zeros(n_free, n_free);
        let mut f_ff = vec![0.0; n_free];

        for (i_f, &dof_i) in free_dofs.iter().enumerate() {
            f_ff[i_f] = f_global[dof_i];
            for (j_f, &dof_j) in free_dofs.iter().enumerate() {
                k_ff[(i_f, j_f)] = k_global[(dof_i, dof_j)];
            }
        }

        // 4. Solve K_ff * u_ff = F_ff
        let u_ff = k_ff.solve_gaussian(&f_ff)
            .ok_or_else(|| "Stiffness matrix is singular. The structure contains an unconstrained kinematic mechanism.".to_string())?;

        // Expand back to full nodal displacement vector
        let mut u_vec = vec![0.0; total_dofs];
        for (i_f, &dof_i) in free_dofs.iter().enumerate() {
            u_vec[dof_i] = u_ff[i_f];
        }

        // 5. Post-Process Displacements
        let mut nodal_displacements = Vec::new();
        let mut max_disp: f64 = 0.0;
        for i in 0..n_nodes {
            let base = i * 6;
            let disp = [
                u_vec[base],
                u_vec[base + 1],
                u_vec[base + 2],
                u_vec[base + 3],
                u_vec[base + 4],
                u_vec[base + 5],
            ];
            let mag = (disp[0] * disp[0] + disp[1] * disp[1] + disp[2] * disp[2]).sqrt();
            max_disp = max_disp.max(mag);
            nodal_displacements.push(disp);
        }

        // 6. Post-Process Member Internal Axial Forces & Utilizations
        let mut member_axial_forces = Vec::new();
        let mut member_capacities = Vec::new();
        let mut member_utilizations = Vec::new();

        for elem in &model.elements {
            let n1_idx = model.nodes.iter().position(|n| n.id == elem.start_node).unwrap();
            let n2_idx = model.nodes.iter().position(|n| n.id == elem.end_node).unwrap();
            let n1 = &model.nodes[n1_idx];
            let n2 = &model.nodes[n2_idx];

            let l = n1.distance_to(n2);
            let cx = (n2.x - n1.x) / l;
            let cy = (n2.y - n1.y) / l;
            let cz = (n2.z - n1.z) / l;

            let u1 = &nodal_displacements[n1_idx];
            let u2 = &nodal_displacements[n2_idx];

            // Member elongation delta = (u2 - u1) . dir
            let delta = (u2[0] - u1[0]) * cx + (u2[1] - u1[1]) * cy + (u2[2] - u1[2]) * cz;
            let axial_force = (elem.material.elastic_modulus * elem.section.area / l) * delta;

            // Capacity = phi * A * Fy (phi = 0.90 for steel LRFD)
            let capacity = 0.90 * elem.section.area * elem.material.yield_strength;
            let util = (axial_force.abs() / (capacity + 1e-12)).min(2.0);

            member_axial_forces.push(axial_force);
            member_capacities.push(capacity);
            member_utilizations.push(util);
        }

        Ok(AnalysisResult {
            nodal_displacements,
            member_axial_forces,
            member_capacities,
            member_utilizations,
            global_k: k_global,
            k_free: k_ff,
            max_displacement_mm: max_disp * 1000.0,
        })
    }

    /// Forms local-to-global 12x12 stiffness matrix for a 3D Truss element
    fn element_stiffness_truss(area: f64, e: f64, l: f64, cx: f64, cy: f64, cz: f64) -> Matrix {
        let k_axial = (e * area) / l;
        let mut k = Matrix::zeros(12, 12);

        let t = [cx, cy, cz];
        for i in 0..3 {
            for j in 0..3 {
                let val = k_axial * t[i] * t[j];
                k[(i, j)] += val;
                k[(i + 6, j + 6)] += val;
                k[(i, j + 6)] -= val;
                k[(i + 6, j)] -= val;
            }
        }
        k
    }

    /// Forms local-to-global 12x12 stiffness matrix for a 3D Frame element
    fn element_stiffness_frame(
        area: f64, iy: f64, iz: f64, j_tor: f64,
        e: f64, g: f64, l: f64,
        cx: f64, cy: f64, cz: f64
    ) -> Matrix {
        // Simplified transformation assuming primary longitudinal axis along (cx, cy, cz)
        // plus bending and torsional stiffness
        let mut k = Matrix::zeros(12, 12);

        let k_ea = (e * area) / l;
        let k_gj = (g * j_tor) / l;
        let k_12eiz = (12.0 * e * iz) / (l * l * l);
        let k_4eiz = (4.0 * e * iz) / l;
        let k_12eiy = (12.0 * e * iy) / (l * l * l);
        let k_4eiy = (4.0 * e * iy) / l;

        let t = [cx, cy, cz];
        for i in 0..3 {
            for j in 0..3 {
                let val_axial = k_ea * t[i] * t[j];
                k[(i, j)] += val_axial;
                k[(i + 6, j + 6)] += val_axial;
                k[(i, j + 6)] -= val_axial;
                k[(i + 6, j)] -= val_axial;
            }
        }

        // Torsional and bending stabilization
        for i in 0..3 {
            k[(i + 3, i + 3)] += k_gj + k_4eiz + k_4eiy;
            k[(i + 9, i + 9)] += k_gj + k_4eiz + k_4eiy;
            k[(i, i)] += k_12eiz + k_12eiy;
            k[(i + 6, i + 6)] += k_12eiz + k_12eiy;
        }

        k
    }
}
