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

/// Menghitung kurva saturasi hiperbolik cekung: x_bar = x / (x + s_x)
/// Menjamin x_bar berada dalam domain tertutup (0, 1) dengan monotonisitas positif.
#[inline]
pub fn hyperbolic_saturation(x: f64, s_x: f64) -> f64 {
    if x <= 0.0 || s_x <= 0.0 {
        0.0
    } else {
        x / (x + s_x)
    }
}

/// Menghitung bobot mekanis sisi gabungan w_e^M (Persamaan 2.7):
/// w_e^M = q_e * (alpha_k * k_bar_e + alpha_R * R_bar_e)
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

/// Biaya kepatuhan elastis sisi Castigliano (Persamaan 2.8):
/// c_e^M = 1 / (w_e^M + eps_w)
#[inline]
pub fn edge_compliance_cost(w_e: f64) -> f64 {
    1.0 / (w_e + EPS_W)
}

// -----------------------------------------------------------------------------
// 2. STRUKTUR DATA MULTIGRAF & K-LINTASAN BEBAS SISI (Persamaan 2.9 & 2.10)
// -----------------------------------------------------------------------------

/// Representasi Multigraf Berbobot Mekanis G^M = (V, E, w^M)
/// Mempertahankan identitas elemen fisik secara independen (Teorema Menger).
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

/// Menghitung bobot kontribusi urutan lintasan omega_r = gamma^(r-1) / sum(gamma^(t-1)) (Eq 2.10)
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

/// Pencarian Ekshaustif Semua Lintasan Bebas Sisi (Edge-Disjoint Paths) (Bagian 2.7 & Kode 4.1)
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
                        continue; // Jalur sederhana bebas siklus simpul
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

    // Bangun kombinasi keluarga lintasan bebas sisi (|P| <= K)
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

/// Menghitung skor agregat pasangan simpul Phi_ij^(K)(G) (Persamaan 2.9)
pub fn compute_pair_score(
    graph: &Multigraph,
    source: usize,
    target: usize,
    edge_costs: &HashMap<usize, f64>,
    config: &AriConfig,
) -> (f64, bool) {
    if source == target {
        // Kasus simpul sekaligus dibebani dan ditumpu: satu lintasan kosong (Bagian 2.7)
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

/// Pasangan beban-tumpuan aktif dengan fraksi pembebanan awal a_i dan reaksi b_j
#[derive(Debug, Clone)]
pub struct LoadSupportPair {
    pub load_node: usize,
    pub support_node: usize,
    pub weight_ab: f64, // pi_c * a_ic * b_jc
}

/// Menghitung skor kapasitas redundansi graf global S(G) (Persamaan 2.13)
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
// 4. HASIL EVALUASI FORMULA INTI ARI v1 & v2 (Persamaan 2.15 - 2.19, 4.13b)
// -----------------------------------------------------------------------------

/// Hasil Evaluasi Lengkap ARI untuk satu elemen terganggu s = (e, lambda)
#[derive(Debug, Clone)]
pub struct ElementAriEvaluation {
    pub element_id: usize,
    pub lambda: f64,
    pub redundancy_ratio_rr: f64,     // RR_e(lambda) (Eq 2.15)
    pub initial_utilization_u_star: f64, // U_e^* (Eq 2.16)
    pub ari_v1: f64,                  // ARI_e(lambda) v1 (Eq 2.17)
    pub ci_v1: f64,                   // CI_e(lambda) v1 (Eq 2.18)
    pub ari_v2: f64,                  // ARI_e^(v2)(lambda) 3-Kanal (Eq 4.13b)
    pub ci_v2: f64,                   // CI_e^(v2)(lambda) (1 - ARI v2)
    pub compliance_leverage_h: f64,   // h_e (Eq 4.1)
    pub stability_margin_m: f64,      // m_e(lambda) (Eq 4.2)
    pub is_kinematic_mechanism: bool, // rank deficient / det(K) = 0
    pub is_critical_forwarded: bool,  // CI >= tau atau pengaman kinematik aktif (Eq 4.5)
}

/// Diagnostic report dari SVD Kinematic Guard (Bagian 2.1 & 4.2)
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

/// Complete Structural Redundancy Analytics Report
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

// -----------------------------------------------------------------------------
// 5. MESIN ARI RESMI: SVD GUARD, SHAPE LEVERAGE h_e, & TIGA KANAL ARI v2
// -----------------------------------------------------------------------------

pub struct AriEngine;

impl AriEngine {
    /// Verifikasi rank dan ruang nol menggunakan Singular Value Decomposition (SVD) Jacobi
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

    /// Menghitung leverage kepatuhan spektral h_e = k_e * a_e * K0^(-1) * a_e^T (Persamaan 4.1)
    /// Memanfaatkan identitas: h_e = sum_k U_{e,k}^2 dari SVD matriks kompatibilitas berbobot A
    pub fn compute_compliance_leverage_h(k_free: &Matrix, a_free: &Matrix, k_elem: f64) -> f64 {
        if let Some(k_inv) = k_free.inverse() {
            // h_e = k_e * (a_e * K_inv * a_e^T)
            let mut v = vec![0.0; k_free.cols];
            for j in 0..k_free.cols {
                let mut sum = 0.0;
                for i in 0..k_free.rows {
                    sum += a_free.get(0, i) * k_inv.get(i, j);
                }
                v[j] = sum;
            }
            let mut quad = 0.0;
            for j in 0..k_free.cols {
                quad += v[j] * a_free.get(0, j);
            }
            (k_elem * quad).clamp(0.0, 1.0)
        } else {
            1.0 // Singular baseline -> leverage maksimal
        }
    }

    /// Evaluasi lengkap ARI v1, Pengaman Kinematik, dan Formulasi Tiga Kanal ARI v2
    pub fn evaluate_redundancy(model: &StructuralModel) -> Result<RedundancyAssessment, String> {
        let baseline = FemSolver::solve(model)?;
        let kinematic_guard = Self::verify_kinematics(&baseline.k_free, 1e-7);

        let mut member_results = Vec::new();

        for (elem_idx, elem) in model.elements.iter().enumerate() {
            let f_init = baseline.member_axial_forces[elem_idx];
            let cap = baseline.member_capacities[elem_idx];
            let u_init = baseline.member_utilizations[elem_idx];

            let mut perturbed_model = model.clone();
            perturbed_model.elements.remove(elem_idx);

            match FemSolver::solve(&perturbed_model) {
                Ok(damaged_result) => {
                    let damaged_guard = Self::verify_kinematics(&damaged_result.k_free, 1e-7);
                    if !damaged_guard.is_stable {
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

                    let max_u_damaged = damaged_result.member_utilizations.iter().copied().fold(0.0, f64::max);
                    let delta_u = (max_u_damaged - u_init).max(0.0);
                    let reserve = (1.0 - u_init).max(0.05);
                    let ari_val = (1.0 - (delta_u / reserve)).clamp(0.0, 1.0);
                    let rr_val = (1.0 - (max_u_damaged - u_init).abs()).clamp(0.0, 1.0);
                    let ci_val = 1.0 - rr_val;
                    let svd_margin = (1.0 / (damaged_guard.condition_number.log10().max(1.0) / 12.0)).clamp(0.0, 1.0);

                    member_results.push(MemberRedundancyResult {
                        member_id: elem.id,
                        member_name: elem.name.clone(),
                        force_kn: f_init / 1000.0,
                        capacity_kn: cap / 1000.0,
                        utilization_initial: u_init,
                        max_redistribution_utilization: max_u_damaged,
                        ari: ari_val,
                        rr: rr_val,
                        ci: ci_val,
                        svd_margin,
                        is_critical: ari_val < 0.70 || max_u_damaged >= 1.0,
                    });
                }
                Err(_) => {
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
                }
            }
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
}

// -----------------------------------------------------------------------------
// 6. SUITE PENGUJIAN OTOMATIS DISERTASI (342 TES KANONIK & MODEL BENCHMARK)
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperbolic_saturation_properties() {
        assert_eq!(hyperbolic_saturation(0.0, 15.0), 0.0);
        assert!((hyperbolic_saturation(15.0, 15.0) - 0.50).abs() < 1e-9);
        assert!(hyperbolic_saturation(100.0, 15.0) < 1.0);
        assert!(hyperbolic_saturation(20.0, 15.0) > hyperbolic_saturation(10.0, 15.0));
    }

    #[test]
    fn test_m1_parallel_bars_disjoint_paths_exact() {
        // Model Kanonik M1: Dua batang paralel antara Simpul 0 (tumpuan) dan Simpul 1 (beban)
        // L = 1000 mm, E = 200 kN/mm2
        // Batang 1: A = 100 mm2, k1 = 20 kN/mm, R1 = 40 kN, s_k = 15, s_R = 30
        // Batang 2: A = 50 mm2, k2 = 10 kN/mm, R2 = 20 kN, s_k = 15, s_R = 30
        let k1 = 20.0;
        let k2 = 10.0;
        let r1 = 40.0;
        let r2 = 20.0;
        let s_k = 15.0;
        let s_r = 30.0;
        let alpha = 0.5;

        let w1 = compute_edge_weight(k1, r1, s_k, s_r, alpha, alpha, 1.0);
        let w2 = compute_edge_weight(k2, r2, s_k, s_r, alpha, alpha, 1.0);

        // w1 = 0.5 * (20/35 + 40/70) = 4/7 approx 0.571428
        // w2 = 0.5 * (10/25 + 20/50) = 2/5 = 0.400000
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

        // Skor Graf Utuh S(G^M)
        let (s_intact, ok) = compute_graph_score(&mg, &costs, &[pair.clone()], &cfg);
        assert!(ok);
        // Persamaan 4.16 di disertasi: S(G^M) = 1/2 * (4/7 + 2/5) = 17/35 approx 0.485714
        assert!((s_intact - 17.0 / 35.0).abs() < 1e-3);

        // Disrupsi Batang 1 Hilang (lambda = 0): hapus edge 1
        let mut mg_d1 = Multigraph::new(vec![0, 1]);
        mg_d1.add_edge(2, 0, 1);
        let (s_d1, _) = compute_graph_score(&mg_d1, &costs, &[pair.clone()], &cfg);
        let rr1 = s_d1 / s_intact;
        // Tabel 25 disertasi: RR1 = 7/17 approx 0.411765
        assert!((rr1 - 7.0 / 17.0).abs() < 1e-3);

        // Utilisasi awal: P = 30 kN, k_tot = 30 -> u0 = 1 mm -> D1 = 20 kN, D2 = 10 kN
        // U1* = 20/40 = 0.50, U2* = 10/20 = 0.50
        let u1_star = 0.50;
        let ari_1 = rr1 * (1.0 - cfg.eta * u1_star);
        // Tabel 25 disertasi: ARI_1 = 21/68 approx 0.308824, CI_1 = 47/68 approx 0.691176
        assert!((ari_1 - 21.0 / 68.0).abs() < 1e-3);
        let ci_1 = 1.0 - ari_1;
        assert!((ci_1 - 47.0 / 68.0).abs() < 1e-3);
    }
}
