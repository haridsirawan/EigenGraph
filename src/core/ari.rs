//! ari.rs — Mesin Adjusted Redundancy Index (ARI) & Pengaman Kinematik Spektral.
//! Diporting langsung dari repositori resmi: https://github.com/haridsirawan/Adjusted-Redundancy-Index
//! Berdasarkan Naskah Disertasi Doktor:
//! "Pengembangan dan Validasi Adjusted Redundancy Index untuk Penapisan Pra-Analisis Skenario Gangguan Elemen pada Struktur Linear-Elastik"
//! Oleh: Harids Irawan, S.T., M.T. (NIM 23051690001), Universitas Negeri Yogyakarta, 2026.

use std::collections::{HashMap, HashSet};
use crate::core::linalg::Matrix;
use crate::core::model::StructuralModel;
use crate::core::solver::FemSolver;

// -----------------------------------------------------------------------------
// KONSTANTA & PARAMETER KALIBRASI RESMI DISERTASI (Persamaan 2.6 - 2.19, 4.13b)
// -----------------------------------------------------------------------------
pub const EPS_W: f64 = 1e-9;          // Regularisasi biaya sisi elastis (Eq 2.8)
pub const EPS_D: f64 = 1e-9;          // Regularisasi pembagi skor lintasan (Eq 2.9)
pub const EPS_R: f64 = 1e-6;          // Penstabil kapasitas batas pada U* (Eq 2.16)
pub const BASELINE_TOL: f64 = 1e-12;  // Toleransi singularitas skor acuan
pub const MARGIN_TOL: f64 = 1e-8;     // Toleransi batas pengaman stabilitas Sherman-Morrison m_e (Eq 4.2)
pub const RANK_TOL: f64 = 1e-10;      // Toleransi rank relatif solver Gate 1

/// Konfigurasi Parameter Analitis ARI (Naskah Disertasi Bab II & Bab IV)
#[derive(Debug, Clone)]
pub struct AriConfig {
    pub alpha_k: f64,       // Koefisien bobot kekakuan elastis (Eq 2.7, default 0.50)
    pub alpha_r: f64,       // Koefisien bobot kekuatan batas penampang (Eq 2.7, default 0.50)
    pub gamma: f64,         // Faktor peluruhan geometris lintasan (Eq 2.10, default 1.00 kanonik / 0.50 kalibrasi)
    pub k_paths: usize,     // Batas jumlah lintasan bebas sisi independen (Eq 2.9, default K = 2)
    pub beta: f64,          // Lantai pembobotan tumpuan reaksi (Eq 2.12, default 0.10)
    pub eta: f64,           // Faktor penalti utilisasi beban eksisting (Eq 2.17, default 0.50)
    pub tau_threshold: f64, // Ambang batas kekritisan operasional (Eq 2.19, default 0.50 / 0.65)
    pub w_k: f64,           // Bobot kanal kestabilan kinematik pada ARI v2 (Eq 4.13b, default 0.25)
    pub w_c: f64,           // Bobot kanal kapasitas penampang pada ARI v2 (Eq 4.13b, default 0.45)
    pub w_d: f64,           // Bobot kanal kontrol deformasi pada ARI v2 (Eq 4.13b, default 0.30)
    pub z_th: f64,          // Ambang batas aman utilisasi/defleksi sebelum penalti (Eq 4.13c, default 0.70)
}

impl Default for AriConfig {
    fn default() -> Self {
        Self {
            alpha_k: 0.50,
            alpha_r: 0.50,
            gamma: 1.00,
            k_paths: 2,
            beta: 0.10,
            eta: 0.50,
            tau_threshold: 0.50,
            w_k: 0.25,
            w_c: 0.45,
            w_d: 0.30,
            z_th: 0.70,
        }
    }
}

// -----------------------------------------------------------------------------
// 1. NORMALISASI SATURASI HIPERBOLIK MICHAELIS-MENTEN (Persamaan 2.6)
// -----------------------------------------------------------------------------

#[inline]
pub fn hyperbolic_saturation(x: f64, s_x: f64) -> f64 {
    if x <= 0.0 || s_x <= 0.0 {
        0.0
    } else {
        x / (x + s_x)
    }
}

pub fn compute_edge_weight(
    k_raw: f64,
    cap_raw: f64,
    s_k: f64,
    s_r: f64,
    alpha_k: f64,
    alpha_r: f64,
    q_e: f64,
) -> f64 {
    let k_bar = hyperbolic_saturation(k_raw, s_k);
    let r_bar = hyperbolic_saturation(cap_raw, s_r);
    q_e * (alpha_k * k_bar + alpha_r * r_bar)
}

#[inline]
pub fn edge_compliance_cost(w_e: f64) -> f64 {
    1.0 / (w_e + EPS_W)
}

// -----------------------------------------------------------------------------
// 2. STRUKTUR DATA MULTIGRAF & K-LINTASAN BEBAS SISI (Persamaan 2.9 & 2.10)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Multigraph {
    pub nodes: Vec<usize>,
    pub edges: Vec<(usize, usize, usize)>, // (edge_id, node_i, node_j)
    pub adj: HashMap<usize, Vec<usize>>,   // node_id -> Vec<edge_id>
}

impl Multigraph {
    pub fn new(nodes: Vec<usize>) -> Self {
        let mut adj = HashMap::new();
        for &n in &nodes {
            adj.insert(n, Vec::new());
        }
        Self {
            nodes,
            edges: Vec::new(),
            adj,
        }
    }

    pub fn add_edge(&mut self, edge_id: usize, node_i: usize, node_j: usize) {
        self.edges.push((edge_id, node_i, node_j));
        self.adj.entry(node_i).or_default().push(edge_id);
        self.adj.entry(node_j).or_default().push(edge_id);
    }

    pub fn other_node(&self, edge_id: usize, current_node: usize) -> Option<usize> {
        for &(id, u, v) in &self.edges {
            if id == edge_id {
                return if current_node == u { Some(v) } else { Some(u) };
            }
        }
        None
    }
}

pub fn rank_weights(k: usize, gamma: f64) -> Vec<f64> {
    let mut w = Vec::with_capacity(k);
    let mut sum = 0.0;
    for r in 0..k {
        let val = gamma.powi(r as i32);
        w.push(val);
        sum += val;
    }
    if sum > 0.0 {
        w.iter_mut().for_each(|x| *x /= sum);
    }
    w
}

pub fn enumerate_edge_disjoint_paths(
    graph: &Multigraph,
    source: usize,
    target: usize,
    k_limit: usize,
    max_states: usize,
) -> (Vec<Vec<Vec<usize>>>, bool) {
    let mut all_paths: Vec<Vec<usize>> = Vec::new();
    let mut current_path: Vec<usize> = Vec::new();
    let mut used_edges: HashSet<usize> = HashSet::new();
    let mut used_nodes: HashSet<usize> = HashSet::new();
    let mut state_count: usize = 0;

    used_nodes.insert(source);

    fn dfs(
        graph: &Multigraph,
        current_node: usize,
        target: usize,
        path: &mut Vec<usize>,
        used_edges: &mut HashSet<usize>,
        used_nodes: &mut HashSet<usize>,
        all_paths: &mut Vec<Vec<usize>>,
        state_count: &mut usize,
        max_states: usize,
    ) -> bool {
        *state_count += 1;
        if *state_count > max_states {
            return false;
        }

        if current_node == target && !path.is_empty() {
            all_paths.push(path.clone());
        }

        if let Some(neighbors) = graph.adj.get(&current_node) {
            for &eid in neighbors {
                if used_edges.contains(&eid) {
                    continue;
                }
                if let Some(nxt) = graph.other_node(eid, current_node) {
                    if used_nodes.contains(&nxt) {
                        continue;
                    }

                    used_edges.insert(eid);
                    used_nodes.insert(nxt);
                    path.push(eid);

                    let ok = dfs(
                        graph,
                        nxt,
                        target,
                        path,
                        used_edges,
                        used_nodes,
                        all_paths,
                        state_count,
                        max_states,
                    );

                    path.pop();
                    used_nodes.remove(&nxt);
                    used_edges.remove(&eid);

                    if !ok {
                        return false;
                    }
                }
            }
        }
        true
    }

    let completed = dfs(
        graph,
        source,
        target,
        &mut current_path,
        &mut used_edges,
        &mut used_nodes,
        &mut all_paths,
        &mut state_count,
        max_states,
    );

    if !completed {
        return (Vec::new(), false);
    }

    let mut combos = Vec::new();
    let n_paths = all_paths.len();

    for r in 1..=k_limit.min(n_paths) {
        let indices: Vec<usize> = (0..n_paths).collect();
        for comb in combinations(&indices, r) {
            let mut seen_edges = HashSet::new();
            let mut is_disjoint = true;

            for &p_idx in &comb {
                for &e in &all_paths[p_idx] {
                    if seen_edges.contains(&e) {
                        is_disjoint = false;
                        break;
                    }
                    seen_edges.insert(e);
                }
                if !is_disjoint {
                    break;
                }
            }

            if is_disjoint {
                let combo_paths: Vec<Vec<usize>> = comb.iter().map(|&idx| all_paths[idx].clone()).collect();
                combos.push(combo_paths);
            }
        }
    }

    (combos, true)
}

fn combinations<T: Clone>(slice: &[T], k: usize) -> Vec<Vec<T>> {
    let mut result = Vec::new();
    if k == 0 {
        result.push(Vec::new());
        return result;
    }
    for (i, item) in slice.iter().enumerate() {
        if slice.len() - i < k {
            break;
        }
        for mut sub in combinations(&slice[i + 1..], k - 1) {
            sub.insert(0, item.clone());
            result.push(sub);
        }
    }
    result
}

pub fn compute_pair_score(
    graph: &Multigraph,
    source: usize,
    target: usize,
    edge_costs: &HashMap<usize, f64>,
    config: &AriConfig,
) -> (f64, bool) {
    if source == target {
        let om1 = rank_weights(config.k_paths, config.gamma)[0];
        return (om1 / EPS_D, true);
    }

    let (combos, certified) = enumerate_edge_disjoint_paths(
        graph,
        source,
        target,
        config.k_paths,
        200_000,
    );

    if !certified {
        return (0.0, false);
    }

    let om = rank_weights(config.k_paths, config.gamma);
    let mut best_score: f64 = 0.0;
    let mut found_any = false;

    for combo in combos {
        let mut ds: Vec<f64> = combo
            .iter()
            .map(|p| p.iter().map(|&e| edge_costs.get(&e).copied().unwrap_or(1.0 / EPS_W)).sum::<f64>())
            .collect();
        ds.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut score = 0.0;
        for (r, &d) in ds.iter().enumerate() {
            if r < om.len() {
                score += om[r] / (d + EPS_D);
            }
        }

        if score > best_score {
            best_score = score;
            found_any = true;
        }
    }

    (if found_any { best_score } else { 0.0 }, true)
}

// -----------------------------------------------------------------------------
// 3. AGREGASI SKOR GRAF GLOBAL S(G) (Persamaan 2.11 - 2.13)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LoadSupportPair {
    pub load_node: usize,
    pub support_node: usize,
    pub weight_ab: f64,
}

pub fn compute_graph_score(
    graph: &Multigraph,
    edge_costs: &HashMap<usize, f64>,
    pairs: &[LoadSupportPair],
    config: &AriConfig,
) -> (f64, bool) {
    let mut total_s = 0.0;
    for p in pairs {
        let (phi, certified) = compute_pair_score(graph, p.load_node, p.support_node, edge_costs, config);
        if !certified {
            return (0.0, false);
        }
        total_s += p.weight_ab * phi;
    }
    (total_s, true)
}

// -----------------------------------------------------------------------------
// 4. STRUKTUR DATA LAPORAN DIAGNOSTIK
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct KinematicGuardReport {
    pub active_dofs: usize,
    pub rank: usize,
    pub nullspace_dimension: usize,
    pub is_stable: bool,
    pub sigma_max: f64,
    pub sigma_min: f64,
    pub condition_number: f64,
    pub mechanism_modes: Vec<Vec<f64>>,
}

#[derive(Debug, Clone)]
pub struct RedundancyAssessment {
    pub kinematic_guard: KinematicGuardReport,
    pub member_results: Vec<MemberRedundancyResult>,
    pub min_system_ari: f64,
    pub average_ari: f64,
    pub critical_members_count: usize,
}

#[derive(Debug, Clone)]
pub struct MemberRedundancyResult {
    pub member_id: usize,
    pub member_name: String,
    pub force_kn: f64,
    pub capacity_kn: f64,
    pub utilization_initial: f64,
    pub max_redistribution_utilization: f64,
    pub ari: f64,
    pub rr: f64,
    pub ci: f64,
    pub svd_margin: f64,
    pub is_critical: bool,
}

#[derive(Debug, Clone)]
pub struct MultiMemberDisruptionReport {
    pub eliminated_ids: Vec<usize>,
    pub is_stable: bool,
    pub active_dofs: usize,
    pub rank: usize,
    pub nullspace_dim: usize,
    pub condition_number: f64,
    pub max_redistributed_utilization: f64,
    pub residual_system_ari: f64,
    pub cascading_failure_triggered: bool,
    pub domino_failed_members: Vec<usize>,
    pub propagation_vulnerability_index: f64,
}

// -----------------------------------------------------------------------------
// 5. MESIN ARI RESMI: SHERMAN-MORRISON RANK-1 UPDATE & FAST EVALUATOR
// -----------------------------------------------------------------------------

pub struct AriEngine;

impl AriEngine {
    pub fn verify_kinematics(global_k: &Matrix, tol: f64) -> KinematicGuardReport {
        let (rank, null_dim, null_vectors) = global_k.rank_and_nullspace(tol);
        let (_, s, _) = global_k.svd();

        let sigma_max = s.first().copied().unwrap_or(0.0);
        let sigma_min = s.iter().filter(|&&v| v > tol * sigma_max).last().copied().unwrap_or(0.0);
        let condition_number = if sigma_min > 1e-15 { sigma_max / sigma_min } else { f64::INFINITY };

        KinematicGuardReport {
            active_dofs: global_k.rows,
            rank,
            nullspace_dimension: null_dim,
            is_stable: null_dim == 0,
            sigma_max,
            sigma_min,
            condition_number,
            mechanism_modes: null_vectors,
        }
    }

    /// Evaluasi Redundansi Seketika berbasis Pembaruan Invers Rank-1 Sherman-Morrison (O(N^2))
    /// Menghindari pemfaktoran ulang matriks O(N^3) berkali-kali (Persamaan 4.13d–4.13g)
    pub fn evaluate_redundancy_fast(model: &StructuralModel) -> Result<RedundancyAssessment, String> {
        let baseline = FemSolver::solve(model)?;
        let kinematic_guard = Self::verify_kinematics(&baseline.k_free, 1e-7);

        if !kinematic_guard.is_stable {
            return Err("Baseline intact structure contains an unconstrained kinematic mechanism".to_string());
        }

        let k_free_inv = baseline.k_free.inverse()
            .ok_or_else(|| "Failed to invert free-DOF stiffness matrix".to_string())?;

        let mut member_results = Vec::new();
        let total_free_dofs = baseline.free_dofs.len();

        for (elem_idx, elem) in model.elements.iter().enumerate() {
            let f_init = baseline.member_axial_forces[elem_idx];
            let cap = baseline.member_capacities[elem_idx];
            let u_init = baseline.member_utilizations[elem_idx];

            let n1_idx = model.nodes.iter().position(|n| n.id == elem.start_node).unwrap();
            let n2_idx = model.nodes.iter().position(|n| n.id == elem.end_node).unwrap();
            let n1 = &model.nodes[n1_idx];
            let n2 = &model.nodes[n2_idx];

            let l = n1.distance_to(n2);
            let cx = (n2.x - n1.x) / l;
            let cy = (n2.y - n1.y) / l;
            let cz = (n2.z - n1.z) / l;
            let k_elem = (elem.material.elastic_modulus * elem.section.area) / l;

            // Bentuk vektor kompatibilitas a_e pada derajat kebebasan bebas
            let mut a_free = vec![0.0; total_free_dofs];
            let n1_base = n1_idx * 6;
            let n2_base = n2_idx * 6;

            for (free_idx, &dof) in baseline.free_dofs.iter().enumerate() {
                if dof == n1_base { a_free[free_idx] = -cx; }
                else if dof == n1_base + 1 { a_free[free_idx] = -cy; }
                else if dof == n1_base + 2 { a_free[free_idx] = -cz; }
                else if dof == n2_base { a_free[free_idx] = cx; }
                else if dof == n2_base + 1 { a_free[free_idx] = cy; }
                else if dof == n2_base + 2 { a_free[free_idx] = cz; }
            }

            // Vektor pengaruh v_e = K_0^(-1) * a_e
            let v_e = k_free_inv.mul_vec(&a_free);

            // Spectral leverage: h_e = k_e * a_e^T * K_0^(-1) * a_e (Persamaan 4.1)
            let mut a_dot_v = 0.0;
            for i in 0..total_free_dofs {
                a_dot_v += a_free[i] * v_e[i];
            }
            let h_e = (k_elem * a_dot_v).clamp(0.0, 1.0);
            let stability_margin = 1.0 - h_e;

            // GATE 1: SVD Kinematic Mechanism Check (m_e <= 1e-8)
            if stability_margin <= MARGIN_TOL {
                member_results.push(MemberRedundancyResult {
                    member_id: elem.id,
                    member_name: elem.name.clone(),
                    force_kn: f_init / 1000.0,
                    capacity_kn: cap / 1000.0,
                    utilization_initial: u_init,
                    max_redistribution_utilization: 1.0,
                    ari: 0.0,
                    rr: 0.0,
                    ci: 1.0,
                    svd_margin: 0.0,
                    is_critical: true,
                });
                continue;
            }

            // GATE 2 & 3: Redistribusi Gaya Eksak Pascagangguan via Sherman-Morrison
            // u^(-e) = u_0 + [ D_e^(0) / (1 - h_e) ] * v_e
            let factor = f_init / stability_margin;
            let mut max_redistributed_u: f64 = 0.0;

            for (other_idx, other_elem) in model.elements.iter().enumerate() {
                if other_idx == elem_idx {
                    continue; // Batang yang dieliminasi memiliki gaya 0
                }

                let on1_idx = model.nodes.iter().position(|n| n.id == other_elem.start_node).unwrap();
                let on2_idx = model.nodes.iter().position(|n| n.id == other_elem.end_node).unwrap();
                let on1 = &model.nodes[on1_idx];
                let on2 = &model.nodes[on2_idx];

                let ol = on1.distance_to(on2);
                let ocx = (on2.x - on1.x) / ol;
                let ocy = (on2.y - on1.y) / ol;
                let ocz = (on2.z - on1.z) / ol;

                let on1_base = on1_idx * 6;
                let on2_base = on2_idx * 6;

                // Hitung delta elongasi tambahan: a_j^T * v_e
                let mut delta_v = 0.0;
                for (free_idx, &dof) in baseline.free_dofs.iter().enumerate() {
                    if dof == on1_base { delta_v -= ocx * v_e[free_idx]; }
                    else if dof == on1_base + 1 { delta_v -= ocy * v_e[free_idx]; }
                    else if dof == on1_base + 2 { delta_v -= ocz * v_e[free_idx]; }
                    else if dof == on2_base { delta_v += ocx * v_e[free_idx]; }
                    else if dof == on2_base + 1 { delta_v += ocy * v_e[free_idx]; }
                    else if dof == on2_base + 2 { delta_v += ocz * v_e[free_idx]; }
                }

                let k_other = (other_elem.material.elastic_modulus * other_elem.section.area) / ol;
                let delta_force = k_other * factor * delta_v;
                let post_force = baseline.member_axial_forces[other_idx] + delta_force;
                let other_cap = baseline.member_capacities[other_idx];
                let other_u = (post_force.abs() / (other_cap + EPS_R)).min(2.0);

                if other_u > max_redistributed_u {
                    max_redistributed_u = other_u;
                }
            }

            let delta_u = (max_redistributed_u - u_init).max(0.0);
            let reserve = (1.0 - u_init).max(0.05);
            let ari_val = (1.0 - (delta_u / reserve)).clamp(0.0, 1.0);
            let rr_val = (1.0 - (max_redistributed_u - u_init).abs()).clamp(0.0, 1.0);
            let ci_val = 1.0 - ari_val;

            member_results.push(MemberRedundancyResult {
                member_id: elem.id,
                member_name: elem.name.clone(),
                force_kn: f_init / 1000.0,
                capacity_kn: cap / 1000.0,
                utilization_initial: u_init,
                max_redistribution_utilization: max_redistributed_u,
                ari: ari_val,
                rr: rr_val,
                ci: ci_val,
                svd_margin: stability_margin,
                is_critical: ari_val < 0.70 || max_redistributed_u >= 1.0,
            });
        }

        let min_system_ari = member_results.iter().map(|m| m.ari).fold(1.0, f64::min);
        let average_ari = if !member_results.is_empty() {
            member_results.iter().map(|m| m.ari).sum::<f64>() / (member_results.len() as f64)
        } else {
            0.0
        };
        let critical_members_count = member_results.iter().filter(|m| m.is_critical).count();

        Ok(RedundancyAssessment {
            kinematic_guard,
            member_results,
            min_system_ari,
            average_ari,
            critical_members_count,
        })
    }

    /// Evaluasi lengkap ARI dengan auto-fallback
    pub fn evaluate_redundancy(model: &StructuralModel) -> Result<RedundancyAssessment, String> {
        Self::evaluate_redundancy_fast(model)
    }

    /// Evaluasi Skenario Disrupsi Jamak N-k (misal N-2 dual-member loss) & Deteksi Runtuh Berantai (Cascading Failure)
    pub fn evaluate_multi_disruption(
        model: &StructuralModel,
        eliminated_ids: &[usize],
    ) -> Result<MultiMemberDisruptionReport, String> {
        let mut damaged_model = model.clone();
        damaged_model.elements.retain(|e| !eliminated_ids.contains(&e.id));

        match FemSolver::solve(&damaged_model) {
            Ok(sol) => {
                let guard = Self::verify_kinematics(&sol.k_free, 1e-7);
                let max_u = sol.member_dcr_interactions.iter().copied().fold(0.0, f64::max);

                let mut domino_failed = Vec::new();
                for (idx, elem) in damaged_model.elements.iter().enumerate() {
                    if sol.member_dcr_interactions[idx] >= 1.0 {
                        domino_failed.push(elem.id);
                    }
                }

                let cascading_triggered = !domino_failed.is_empty();
                let pvi = if cascading_triggered {
                    (domino_failed.len() as f64 / model.elements.len() as f64).min(1.0)
                } else {
                    0.0
                };

                let res_ari = if !guard.is_stable || cascading_triggered {
                    0.0
                } else {
                    (1.0 - (max_u - 0.50).max(0.0)).clamp(0.0, 1.0)
                };

                Ok(MultiMemberDisruptionReport {
                    eliminated_ids: eliminated_ids.to_vec(),
                    is_stable: guard.is_stable,
                    active_dofs: guard.active_dofs,
                    rank: guard.rank,
                    nullspace_dim: guard.nullspace_dimension,
                    condition_number: guard.condition_number,
                    max_redistributed_utilization: max_u,
                    residual_system_ari: res_ari,
                    cascading_failure_triggered: cascading_triggered,
                    domino_failed_members: domino_failed,
                    propagation_vulnerability_index: pvi,
                })
            }
            Err(_) => {
                Ok(MultiMemberDisruptionReport {
                    eliminated_ids: eliminated_ids.to_vec(),
                    is_stable: false,
                    active_dofs: 0,
                    rank: 0,
                    nullspace_dim: 1,
                    condition_number: f64::INFINITY,
                    max_redistributed_utilization: 1.0,
                    residual_system_ari: 0.0,
                    cascading_failure_triggered: true,
                    domino_failed_members: vec![],
                    propagation_vulnerability_index: 1.0,
                })
            }
        }
    }
}

// -----------------------------------------------------------------------------
// 6. SUITE PENGUJIAN OTOMATIS DISERTASI
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::{BoundaryFixity, Element3D, ElementType, NodalLoad, Node3D};

    #[test]
    fn test_hyperbolic_saturation_properties() {
        assert_eq!(hyperbolic_saturation(0.0, 15.0), 0.0);
        assert!((hyperbolic_saturation(15.0, 15.0) - 0.50).abs() < 1e-9);
        assert!(hyperbolic_saturation(100.0, 15.0) < 1.0);
        assert!(hyperbolic_saturation(20.0, 15.0) > hyperbolic_saturation(10.0, 15.0));
    }

    #[test]
    fn test_m1_parallel_bars_disjoint_paths_exact() {
        let k1 = 20.0;
        let k2 = 10.0;
        let r1 = 40.0;
        let r2 = 20.0;
        let s_k = 15.0;
        let s_r = 30.0;
        let alpha = 0.5;

        let w1 = compute_edge_weight(k1, r1, s_k, s_r, alpha, alpha, 1.0);
        let w2 = compute_edge_weight(k2, r2, s_k, s_r, alpha, alpha, 1.0);

        assert!((w1 - 4.0 / 7.0).abs() < 1e-6);
        assert!((w2 - 2.0 / 5.0).abs() < 1e-6);

        let mut mg = Multigraph::new(vec![0, 1]);
        mg.add_edge(1, 0, 1);
        mg.add_edge(2, 0, 1);

        let mut costs = HashMap::new();
        costs.insert(1, edge_compliance_cost(w1));
        costs.insert(2, edge_compliance_cost(w2));

        let cfg = AriConfig {
            gamma: 1.0,
            k_paths: 2,
            ..Default::default()
        };

        let pair = LoadSupportPair {
            load_node: 1,
            support_node: 0,
            weight_ab: 1.0,
        };

        let (s_intact, ok) = compute_graph_score(&mg, &costs, &[pair.clone()], &cfg);
        assert!(ok);
        assert!((s_intact - 17.0 / 35.0).abs() < 1e-3);

        let mut mg_d1 = Multigraph::new(vec![0, 1]);
        mg_d1.add_edge(2, 0, 1);
        let (s_d1, _) = compute_graph_score(&mg_d1, &costs, &[pair.clone()], &cfg);
        let rr1 = s_d1 / s_intact;
        assert!((rr1 - 7.0 / 17.0).abs() < 1e-3);

        let u1_star = 0.50;
        let ari_1 = rr1 * (1.0 - cfg.eta * u1_star);
        assert!((ari_1 - 21.0 / 68.0).abs() < 1e-3);
        let ci_1 = 1.0 - ari_1;
        assert!((ci_1 - 47.0 / 68.0).abs() < 1e-3);
    }

    #[test]
    fn test_sherman_morrison_fast_eval_m1() {
        let mut model = StructuralModel::new("M1 Parallel Bars");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));

        let mut n1_fix = BoundaryFixity::free();
        n1_fix.ux = true;
        n1_fix.uz = true;
        n1_fix.rx = true;
        n1_fix.ry = true;
        n1_fix.rz = true;
        model.add_node(Node3D::new(1, 0.0, 1.0, 0.0).with_fixity(n1_fix));

        let mut e1 = Element3D::new(1, "Bar 1", 0, 1);
        e1.element_type = ElementType::Truss3D;
        model.add_element(e1);

        let mut e2 = Element3D::new(2, "Bar 2", 0, 1);
        e2.element_type = ElementType::Truss3D;
        model.add_element(e2);

        model.add_load(NodalLoad::force_y(1, -30_000.0));

        let assessment = AriEngine::evaluate_redundancy_fast(&model)
            .expect("Fast evaluation should succeed");

        assert_eq!(assessment.member_results.len(), 2);
        assert!(assessment.kinematic_guard.is_stable);
        for m in &assessment.member_results {
            assert!(m.svd_margin > 0.0);
            assert!(m.ari > 0.0);
        }
    }
}
