//! solver.rs — Linear Elastic Finite Element Analysis (FEA) & Stiffness Assembly Engine.

use crate::core::linalg::Matrix;
use crate::core::model::{ElementType, LoadCombination, StructuralModel};

/// Governing LRFD Load Combination Envelope Result for a single structural member
#[derive(Debug, Clone)]
pub struct MemberEnvelopeResult {
    pub member_id: usize,
    pub member_name: String,
    pub max_tension_n: f64,
    pub max_compression_n: f64,
    pub max_shear_n: f64,
    pub max_moment_nm: f64,
    pub governing_dcr: f64,
    pub governing_combination: String,
}

/// Complete Multi-Load Combination Envelope Analysis Result (SNI 1727:2020 / ASCE 7-22)
#[derive(Debug, Clone)]
pub struct EnvelopeAnalysisResult {
    pub combinations_evaluated: usize,
    pub max_envelope_displacement_mm: f64,
    pub governing_system_combination: String,
    pub member_envelopes: Vec<MemberEnvelopeResult>,
}

/// Result of a completed Finite Element Analysis
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Nodal Displacements (6 DOFs per node: [ux, uy, uz, rx, ry, rz])
    pub nodal_displacements: Vec<[f64; 6]>,
    /// Member internal axial forces in Newtons (Positive = Tension, Negative = Compression)
    pub member_axial_forces: Vec<f64>,
    /// Member internal shear forces in Newtons: V_max = max(|Vy|, |Vz|)
    pub member_shear_forces: Vec<f64>,
    /// Member internal bending moments in N.m: (max |My|, max |Mz|)
    pub member_bending_moments: Vec<(f64, f64)>,
    /// Member capacity in Newtons
    pub member_capacities: Vec<f64>,
    /// Member axial utilization ratio U = |N| / (phi * Pn)
    pub member_utilizations: Vec<f64>,
    /// AISC 360-16 / SNI 1729:2020 Combined P-M-V Interaction Ratio
    pub member_dcr_interactions: Vec<f64>,
    /// Global stiffness matrix (assembled)
    pub global_k: Matrix,
    /// Partitioned Free-DOF Stiffness Matrix K_ff
    pub k_free: Matrix,
    /// Maximum nodal displacement magnitude in mm
    pub max_displacement_mm: f64,
    /// Indices of unconstrained degrees of freedom
    pub free_dofs: Vec<usize>,
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

        // O(1) Node ID -> Index lookup table
        let mut node_idx_map = std::collections::HashMap::with_capacity(n_nodes);
        for (idx, node) in model.nodes.iter().enumerate() {
            node_idx_map.insert(node.id, idx);
        }

        // 1. Assemble Global Stiffness Matrix K
        let mut k_global = Matrix::zeros(total_dofs, total_dofs);

        for elem in &model.elements {
            let &node1_idx = node_idx_map
                .get(&elem.start_node)
                .ok_or_else(|| format!("Start node {} not found", elem.start_node))?;
            let &node2_idx = node_idx_map
                .get(&elem.end_node)
                .ok_or_else(|| format!("End node {} not found", elem.end_node))?;

            let n1 = &model.nodes[node1_idx];
            let n2 = &model.nodes[node2_idx];

            let l = n1.distance_to(n2);
            if l < 1e-6 {
                return Err(format!("Element {} has near-zero length", elem.id));
            }

            let cx = (n2.x - n1.x) / l;
            let cy = (n2.y - n1.y) / l;
            let cz = (n2.z - n1.z) / l;

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
            if let Some(&idx) = node_idx_map.get(&load.node_id) {
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
                if !is_fixed && k_global[(dof, dof)].abs() > 1e-12 {
                    free_dofs.push(dof);
                }
            }
        }

        let n_free = free_dofs.len();
        if n_free == 0 {
            return Err("Model has no free degrees of freedom".to_string());
        }

        let mut k_ff = Matrix::zeros(n_free, n_free);
        let mut f_ff = vec![0.0; n_free];

        for (i_f, &dof_i) in free_dofs.iter().enumerate() {
            f_ff[i_f] = f_global[dof_i];
            for (j_f, &dof_j) in free_dofs.iter().enumerate() {
                k_ff[(i_f, j_f)] = k_global[(dof_i, dof_j)];
            }
        }

        // 4. Solve K_ff * u_ff = F_ff via SPD Cholesky / Sparse PCG / Gaussian fallback
        let u_ff = k_ff.solve_spd(&f_ff)
            .ok_or_else(|| "Stiffness matrix is singular. The structure contains an unconstrained kinematic mechanism.".to_string())?;

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

        // 6. Post-Process Member Internal Forces, Moments & AISC P-M-V Interactions
        let mut member_axial_forces = Vec::new();
        let mut member_capacities = Vec::new();
        let mut member_utilizations = Vec::new();
        let mut member_shear_forces = Vec::new();
        let mut member_bending_moments = Vec::new();
        let mut member_dcr_interactions = Vec::new();

        for elem in &model.elements {
            let n1_idx = node_idx_map[&elem.start_node];
            let n2_idx = node_idx_map[&elem.end_node];
            let n1 = &model.nodes[n1_idx];
            let n2 = &model.nodes[n2_idx];

            let l = n1.distance_to(n2);
            let cx = (n2.x - n1.x) / l;
            let cy = (n2.y - n1.y) / l;
            let cz = (n2.z - n1.z) / l;

            let u1 = &nodal_displacements[n1_idx];
            let u2 = &nodal_displacements[n2_idx];

            let delta = (u2[0] - u1[0]) * cx + (u2[1] - u1[1]) * cy + (u2[2] - u1[2]) * cz;
            let axial_force = (elem.material.elastic_modulus * elem.section.area / l) * delta;
            let axial_capacity = 0.90 * elem.section.area * elem.material.yield_strength;
            let p_ratio = (axial_force.abs() / (axial_capacity + 1e-12)).min(2.0);

            let (max_my, max_mz, max_v, dcr) = match elem.element_type {
                ElementType::Truss3D => (0.0, 0.0, 0.0, p_ratio),
                ElementType::Frame3D => {
                    let e = elem.material.elastic_modulus;
                    let fy = elem.material.yield_strength;
                    let iy = elem.section.iy;
                    let iz = elem.section.iz;

                    // Project global displacements and rotations into local member axes (x_l, y_l, z_l)
                    let r = Self::frame_rotation_3x3(cx, cy, cz);
                    let du_g = [u2[0] - u1[0], u2[1] - u1[1], u2[2] - u1[2]];
                    let delta_vy = r[1][0] * du_g[0] + r[1][1] * du_g[1] + r[1][2] * du_g[2];
                    let delta_vz = r[2][0] * du_g[0] + r[2][1] * du_g[1] + r[2][2] * du_g[2];

                    let theta1y = r[1][0] * u1[3] + r[1][1] * u1[4] + r[1][2] * u1[5];
                    let theta2y = r[1][0] * u2[3] + r[1][1] * u2[4] + r[1][2] * u2[5];
                    let theta1z = r[2][0] * u1[3] + r[2][1] * u1[4] + r[2][2] * u1[5];
                    let theta2z = r[2][0] * u2[3] + r[2][1] * u2[4] + r[2][2] * u2[5];

                    let mz1 = (2.0 * e * iz / l) * (2.0 * theta1z + theta2z - (3.0 * delta_vy / l));
                    let mz2 = (2.0 * e * iz / l) * (theta1z + 2.0 * theta2z - (3.0 * delta_vy / l));
                    let my1 = (2.0 * e * iy / l) * (2.0 * theta1y + theta2y + (3.0 * delta_vz / l));
                    let my2 = (2.0 * e * iy / l) * (theta1y + 2.0 * theta2y + (3.0 * delta_vz / l));

                    let max_mz = mz1.abs().max(mz2.abs());
                    let max_my = my1.abs().max(my2.abs());
                    let max_v = ((mz1 + mz2).abs() / l).max((my1 + my2).abs() / l);

                    let c_depth = (elem.section.depth / 2.0).max(0.01);
                    let c_width = (elem.section.width / 2.0).max(0.01);
                    let mpz = 0.90 * (1.15 * iz / c_depth) * fy;
                    let mpy = 0.90 * (1.15 * iy / c_width) * fy;

                    let mz_ratio = max_mz / (mpz + 1e-12);
                    let my_ratio = max_my / (mpy + 1e-12);

                    let dcr_comb = if p_ratio >= 0.20 {
                        p_ratio + (8.0 / 9.0) * (mz_ratio + my_ratio)
                    } else {
                        (p_ratio / 2.0) + (mz_ratio + my_ratio)
                    };

                    (max_my, max_mz, max_v, dcr_comb.min(2.0))
                }
            };

            member_axial_forces.push(axial_force);
            member_capacities.push(axial_capacity);
            member_utilizations.push(p_ratio);
            member_shear_forces.push(max_v);
            member_bending_moments.push((max_my, max_mz));
            member_dcr_interactions.push(dcr);
        }

        Ok(AnalysisResult {
            nodal_displacements,
            member_axial_forces,
            member_shear_forces,
            member_bending_moments,
            member_capacities,
            member_utilizations,
            member_dcr_interactions,
            global_k: k_global,
            k_free: k_ff,
            max_displacement_mm: max_disp * 1000.0,
            free_dofs,
        })
    }

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

    /// Constructs the 3x3 orthogonal direction cosine matrix R mapping global (X, Y, Z) vectors
    /// into local member coordinates (x_axial, y_major, z_minor).
    pub fn frame_rotation_3x3(cx: f64, cy: f64, cz: f64) -> [[f64; 3]; 3] {
        let c_xy = (cx * cx + cy * cy).sqrt();
        if c_xy < 1e-5 {
            // Member is vertical along global Z-axis
            let sign = if cz >= 0.0 { 1.0 } else { -1.0 };
            [
                [0.0, 0.0, sign],
                [0.0, 1.0, 0.0],
                [-sign, 0.0, 0.0],
            ]
        } else {
            [
                [cx, cy, cz],
                [-cy / c_xy, cx / c_xy, 0.0],
                [-cx * cz / c_xy, -cy * cz / c_xy, c_xy],
            ]
        }
    }

    fn element_stiffness_frame(
        area: f64, iy: f64, iz: f64, j_tor: f64,
        e: f64, g: f64, l: f64,
        cx: f64, cy: f64, cz: f64
    ) -> Matrix {
        let mut k_local = Matrix::zeros(12, 12);

        let k_ea = (e * area) / l;
        let k_gj = (g * j_tor) / l;

        let l2 = l * l;
        let l3 = l2 * l;

        let k_12eiz = (12.0 * e * iz) / l3;
        let k_6eiz = (6.0 * e * iz) / l2;
        let k_4eiz = (4.0 * e * iz) / l;
        let k_2eiz = (2.0 * e * iz) / l;

        let k_12eiy = (12.0 * e * iy) / l3;
        let k_6eiy = (6.0 * e * iy) / l2;
        let k_4eiy = (4.0 * e * iy) / l;
        let k_2eiy = (2.0 * e * iy) / l;

        // 1. Axial stiffness (u_x1, u_x2 -> indices 0, 6)
        k_local[(0, 0)] = k_ea;
        k_local[(0, 6)] = -k_ea;
        k_local[(6, 0)] = -k_ea;
        k_local[(6, 6)] = k_ea;

        // 2. Saint-Venant Torsional stiffness (theta_x1, theta_x2 -> indices 3, 9)
        let reg_tor = k_gj * 1e-3 + 1.0;
        k_local[(3, 3)] = k_gj + reg_tor;
        k_local[(3, 9)] = -k_gj;
        k_local[(9, 3)] = -k_gj;
        k_local[(9, 9)] = k_gj + reg_tor;

        // 3. Major-axis bending in local x-y plane (u_y1, theta_z1, u_y2, theta_z2 -> indices 1, 5, 7, 11)
        let reg_rz = k_4eiz * 1e-3;
        k_local[(1, 1)] = k_12eiz;
        k_local[(1, 5)] = k_6eiz;
        k_local[(1, 7)] = -k_12eiz;
        k_local[(1, 11)] = k_6eiz;

        k_local[(5, 1)] = k_6eiz;
        k_local[(5, 5)] = k_4eiz + reg_rz;
        k_local[(5, 7)] = -k_6eiz;
        k_local[(5, 11)] = k_2eiz;

        k_local[(7, 1)] = -k_12eiz;
        k_local[(7, 5)] = -k_6eiz;
        k_local[(7, 7)] = k_12eiz;
        k_local[(7, 11)] = -k_6eiz;

        k_local[(11, 1)] = k_6eiz;
        k_local[(11, 5)] = k_2eiz;
        k_local[(11, 7)] = -k_6eiz;
        k_local[(11, 11)] = k_4eiz + reg_rz;

        // 4. Minor-axis bending in local x-z plane (u_z1, theta_y1, u_z2, theta_y2 -> indices 2, 4, 8, 10)
        let reg_ry = k_4eiy * 1e-3;
        k_local[(2, 2)] = k_12eiy;
        k_local[(2, 4)] = -k_6eiy;
        k_local[(2, 8)] = -k_12eiy;
        k_local[(2, 10)] = -k_6eiy;

        k_local[(4, 2)] = -k_6eiy;
        k_local[(4, 4)] = k_4eiy + reg_ry;
        k_local[(4, 8)] = k_6eiy;
        k_local[(4, 10)] = k_2eiy;

        k_local[(8, 2)] = -k_12eiy;
        k_local[(8, 4)] = k_6eiy;
        k_local[(8, 8)] = k_12eiy;
        k_local[(8, 10)] = k_6eiy;

        k_local[(10, 2)] = -k_6eiy;
        k_local[(10, 4)] = k_2eiy;
        k_local[(10, 8)] = k_6eiy;
        k_local[(10, 10)] = k_4eiy + reg_ry;

        // 5. Orthogonal 3D transformation K_global = T^T * K_local * T
        let r = Self::frame_rotation_3x3(cx, cy, cz);
        let mut t_mat = Matrix::zeros(12, 12);
        for block in 0..4 {
            let base = block * 3;
            for i in 0..3 {
                for j in 0..3 {
                    t_mat[(base + i, base + j)] = r[i][j];
                }
            }
        }

        let kt = k_local.mul(&t_mat);
        t_mat.transpose().mul(&kt)
    }

    /// Evaluates the Governing LRFD Load Combination Envelope (SNI 1727:2020 / ASCE 7-22)
    /// across all specified load combinations without re-assembling or re-factoring K_ff.
    pub fn solve_lrfd_envelope(
        model: &StructuralModel,
        combinations: &[LoadCombination],
    ) -> Result<EnvelopeAnalysisResult, String> {
        let base_res = Self::solve(model)?;
        let combos = if combinations.is_empty() {
            LoadCombination::standard_sni_1727_lrfd()
        } else {
            combinations.to_vec()
        };

        let m = model.elements.len();
        let mut member_envelopes: Vec<MemberEnvelopeResult> = model
            .elements
            .iter()
            .map(|e| MemberEnvelopeResult {
                member_id: e.id,
                member_name: e.name.clone(),
                max_tension_n: 0.0,
                max_compression_n: 0.0,
                max_shear_n: 0.0,
                max_moment_nm: 0.0,
                governing_dcr: 0.0,
                governing_combination: String::new(),
            })
            .collect();

        let mut max_envelope_displacement_mm = 0.0;
        let mut governing_system_combination = String::new();

        for combo in &combos {
            let mult = combo.effective_load_multiplier();
            let combo_disp = base_res.max_displacement_mm * mult;
            if combo_disp > max_envelope_displacement_mm {
                max_envelope_displacement_mm = combo_disp;
                governing_system_combination = combo.name.clone();
            }

            for i in 0..m {
                let f_axial = base_res.member_axial_forces[i] * mult;
                let v_shear = base_res.member_shear_forces[i] * mult;
                let (my, mz) = base_res.member_bending_moments[i];
                let m_tot = (my.max(mz)) * mult;
                let dcr = (base_res.member_dcr_interactions[i] * mult).min(2.0);

                let env = &mut member_envelopes[i];
                if f_axial > env.max_tension_n {
                    env.max_tension_n = f_axial;
                }
                if f_axial < -env.max_compression_n {
                    env.max_compression_n = -f_axial;
                }
                if v_shear > env.max_shear_n {
                    env.max_shear_n = v_shear;
                }
                if m_tot > env.max_moment_nm {
                    env.max_moment_nm = m_tot;
                }
                if dcr >= env.governing_dcr {
                    env.governing_dcr = dcr;
                    env.governing_combination = combo.name.clone();
                }
            }
        }

        Ok(EnvelopeAnalysisResult {
            combinations_evaluated: combos.len(),
            max_envelope_displacement_mm,
            governing_system_combination,
            member_envelopes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{BoundaryFixity, CrossSection, Element3D, NodalLoad, Node3D};

    #[test]
    fn test_sni_1727_lrfd_load_combination_envelope() {
        let mut model = StructuralModel::new("Simple Cantilever Column");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        model.add_node(Node3D::new(1, 0.0, 3.0, 0.0));
        model.add_element(
            Element3D::new(1, "Col-1", 0, 1)
                .with_type(ElementType::Frame3D)
                .with_section(CrossSection::h_300x300()),
        );
        model.add_load(NodalLoad::force_y(1, -150_000.0));

        let combos = LoadCombination::standard_sni_1727_lrfd();
        assert_eq!(combos.len(), 6);

        let env = FemSolver::solve_lrfd_envelope(&model, &combos)
            .expect("LRFD envelope solver should succeed");

        assert_eq!(env.combinations_evaluated, 6);
        assert_eq!(env.member_envelopes.len(), 1);
        assert!(env.member_envelopes[0].max_compression_n > 150_000.0);
        assert!(env.member_envelopes[0].governing_dcr > 0.0);
        assert_eq!(env.member_envelopes[0].governing_combination, "LC2: 1.2D + 1.6L + 0.5Lr");
    }

    #[test]
    fn test_standard_steel_catalog_properties() {
        let catalog = CrossSection::catalog_jis_aisc();
        assert_eq!(catalog.len(), 11);
        for sec in &catalog {
            assert!(sec.area > 0.0);
            assert!(sec.iy > 0.0);
            assert!(sec.iz > 0.0);
            assert!(sec.radius_of_gyration_y() > 0.0);
            assert!(sec.radius_of_gyration_z() > 0.0);
        }
    }

    #[test]
    fn test_3d_frame_euler_bernoulli_cantilever_bending_and_equilibrium() {
        let mut model = StructuralModel::new("3D Cantilever Beam along X-Axis");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        model.add_node(Node3D::new(1, 4.0, 0.0, 0.0));

        let sec = CrossSection::wf_300x150();
        let elem = Element3D::new(1, "Cantilever-WF300", 0, 1)
            .with_type(ElementType::Frame3D)
            .with_section(sec.clone());
        let e_mod = elem.material.elastic_modulus;
        model.add_element(elem);

        let p_transverse = -20_000.0; // 20 kN downward along Y
        model.add_load(NodalLoad::force_y(1, p_transverse));

        let res = FemSolver::solve(&model).expect("3D Frame cantilever should solve");

        // Theoretical Euler-Bernoulli tip deflection: delta = P * L^3 / (3 * E * Iz)
        let l = 4.0_f64;
        let delta_theory_m = (p_transverse.abs() * l.powi(3)) / (3.0 * e_mod * sec.iz);
        let delta_fem_m = res.nodal_displacements[1][1].abs();
        let rel_err = (delta_fem_m - delta_theory_m).abs() / delta_theory_m;
        assert!(rel_err < 0.01, "Cantilever tip deflection error too large: {:.4}%", rel_err * 100.0);

        // Fixed-end bending moment Mz = |P| * L = 80,000 N.m
        let (_, mz_max) = res.member_bending_moments[0];
        let m_theory = p_transverse.abs() * l;
        assert!((mz_max - m_theory).abs() / m_theory < 0.01);
        assert!((res.member_shear_forces[0] - p_transverse.abs()).abs() / p_transverse.abs() < 0.01);
    }
}
