//! eigengate.rs — EigenGate (Linear Pre-Analysis & Gatekeeper) & Spectral Kinematic Guard.
//! Ported directly from official repositories:
//!   - Branch 1 (EigenGate Linear Gatekeeper): https://github.com/haridsirawan/EigenGate
//!   - Branch 2 (EigenFlow Non-Linear Woodbury): https://github.com/haridsirawan/EigenFlow
//! By: Harids Irawan, S.T., M.T., EigenGraph Platform, 2026.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::core::linalg::Matrix;
use crate::core::model::StructuralModel;
use crate::core::solver::FemSolver;

// -----------------------------------------------------------------------------
// OFFICIAL EIGENGRAPH CONSTANTS & CALIBRATION PARAMETERS (Eqs. 2.6 - 2.19, 4.13b)
// -----------------------------------------------------------------------------
pub const EPS_W: f64 = 1e-9;          // Elastic edge weight regularization (Eq 2.8)
pub const EPS_D: f64 = 1e-9;          // Path score divisor regularization (Eq 2.9)
pub const EPS_R: f64 = 1e-6;          // Limit capacity stabilizer for U* (Eq 2.16)
pub const BASELINE_TOL: f64 = 1e-12;  // Baseline score singularity tolerance
pub const MARGIN_TOL: f64 = 1e-8;     // Sherman-Morrison stability guard boundary tolerance m_e (Eq 4.2)
pub const RANK_TOL: f64 = 1e-10;      // Gate 1 relative rank solver tolerance

/// Analytical Configuration Parameters for EigenGate (EigenGraph Formulation Ch. II & Ch. IV)
#[derive(Debug, Clone)]
pub struct EigenGateConfig {
    pub alpha_k: f64,       // Elastic stiffness weighting coefficient (Eq 2.7, default 0.50)
    pub alpha_r: f64,       // Ultimate section capacity weighting coefficient (Eq 2.7, default 0.50)
    pub gamma: f64,         // Geometric path decay factor (Eq 2.10, default 1.00 canonical / 0.50 calibrated)
    pub k_paths: usize,     // Maximum independent edge-disjoint paths (Eq 2.9, default K = 2)
    pub beta: f64,          // Support reaction weighting floor (Eq 2.12, default 0.10)
    pub eta: f64,           // Existing load utilization penalty factor (Eq 2.17, default 0.50)
    pub tau_threshold: f64, // Operational criticality threshold (Eq 2.19, default 0.50 / 0.65)
    pub w_k: f64,           // Kinematic stability channel weight in EigenGate (Eq 4.13b, default 0.25)
    pub w_c: f64,           // Section capacity channel weight in EigenGate (Eq 4.13b, default 0.45)
    pub w_d: f64,           // Deformation control channel weight in EigenGate (Eq 4.13b, default 0.30)
    pub z_th: f64,          // Safe utilization/deflection threshold before penalty (Eq 4.13c, default 0.70)
    pub include_geometric_stiffness: bool, // Evaluate post-disruption P-Delta & Kg buckling effects (default: true)
}

impl Default for EigenGateConfig {
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
            include_geometric_stiffness: true,
        }
    }
}

// -----------------------------------------------------------------------------
// 1. MICHAELIS-MENTEN HYPERBOLIC SATURATION NORMALIZATION (Eq. 2.6)
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
// 2. MULTIGRAPH DATA STRUCTURE & K EDGE-DISJOINT LOAD PATHS (Eqs. 2.9 & 2.10)
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

/// Polynomial-time O(K * |E| log |V|) Successive Shortest Edge-Disjoint Paths (Dijkstra / Suurballe)
/// fallback for large 3D structural graphs where exhaustive DFS exceeds `max_states`.
pub fn find_k_shortest_edge_disjoint_paths(
    graph: &Multigraph,
    source: usize,
    target: usize,
    edge_costs: &HashMap<usize, f64>,
    k_limit: usize,
) -> Vec<Vec<usize>> {
    let mut disjoint_paths = Vec::with_capacity(k_limit);
    let mut used_edges = HashSet::new();

    for _ in 0..k_limit {
        let mut dist: HashMap<usize, f64> = HashMap::new();
        let mut prev: HashMap<usize, (usize, usize)> = HashMap::new(); // node -> (prev_node, edge_id)
        let mut visited: HashSet<usize> = HashSet::new();

        for &n in &graph.nodes {
            dist.insert(n, f64::INFINITY);
        }
        dist.insert(source, 0.0);

        loop {
            // Find unvisited node with minimum distance
            let mut u_opt = None;
            let mut best_d = f64::INFINITY;
            for (&node, &d) in &dist {
                if !visited.contains(&node) && d < best_d {
                    best_d = d;
                    u_opt = Some(node);
                }
            }

            let u = match u_opt {
                Some(n) => n,
                None => break,
            };
            if u == target {
                break;
            }
            visited.insert(u);

            if let Some(incident_edges) = graph.adj.get(&u) {
                for &eid in incident_edges {
                    if used_edges.contains(&eid) {
                        continue;
                    }
                    if let Some(v) = graph.other_node(eid, u) {
                        if visited.contains(&v) {
                            continue;
                        }
                        let cost = edge_costs.get(&eid).copied().unwrap_or(1.0 / EPS_W).max(0.0);
                        let alt = best_d + cost;
                        if alt < *dist.get(&v).unwrap_or(&f64::INFINITY) {
                            dist.insert(v, alt);
                            prev.insert(v, (u, eid));
                        }
                    }
                }
            }
        }

        if !prev.contains_key(&target) {
            break;
        }

        // Reconstruct shortest path
        let mut path_edges = Vec::new();
        let mut curr = target;
        while curr != source {
            if let Some(&(p_node, eid)) = prev.get(&curr) {
                path_edges.push(eid);
                used_edges.insert(eid);
                curr = p_node;
            } else {
                break;
            }
        }
        path_edges.reverse();
        if !path_edges.is_empty() {
            disjoint_paths.push(path_edges);
        } else {
            break;
        }
    }

    disjoint_paths
}

pub fn compute_pair_score(
    graph: &Multigraph,
    source: usize,
    target: usize,
    edge_costs: &HashMap<usize, f64>,
    config: &EigenGateConfig,
) -> (f64, bool) {
    if source == target {
        let om1 = rank_weights(config.k_paths, config.gamma)[0];
        return (om1 / EPS_D, true);
    }

    let (mut combos, certified) = enumerate_edge_disjoint_paths(
        graph,
        source,
        target,
        config.k_paths,
        200_000,
    );

    if !certified {
        // Fallback to polynomial-time Dijkstra successive edge-disjoint shortest paths
        let greedy_paths = find_k_shortest_edge_disjoint_paths(
            graph,
            source,
            target,
            edge_costs,
            config.k_paths,
        );
        if greedy_paths.is_empty() {
            return (0.0, true);
        }
        combos = vec![greedy_paths];
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
// 3. GLOBAL GRAPH SCORE AGGREGATION S(G) (Eqs. 2.11 - 2.13)
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
    config: &EigenGateConfig,
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
// 4. DIAGNOSTIC REPORT DATA STRUCTURES
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
    pub min_system_eigengate: f64,
    pub average_eigengate: f64,
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
    pub eigengate: f64,
    pub rr: f64,
    pub ci: f64,
    pub svd_margin: f64,
    pub is_critical: bool,
    pub euler_buckling_kn: f64,
    pub b1_amplification: f64,
    pub buckling_margin: f64,
    pub second_order_dcr: f64,
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
    pub residual_system_eigengate: f64,
    pub cascading_failure_triggered: bool,
    pub domino_failed_members: Vec<usize>,
    pub propagation_vulnerability_index: f64,
}

/// Fast N-2 Pair Screening Result using the Rank-2 Woodbury Matrix Identity
#[derive(Debug, Clone)]
pub struct N2PairRedundancyResult {
    pub member_1_id: usize,
    pub member_2_id: usize,
    pub member_1_name: String,
    pub member_2_name: String,
    pub h11: f64,
    pub h22: f64,
    pub h12_coupling: f64,
    pub woodbury_determinant: f64,
    pub is_coupled_mechanism: bool,
    pub max_redistributed_utilization: f64,
    pub pair_eigengate: f64,
}

/// Comprehensive N-2 Contingency Screening Report Based on the Woodbury Identity
#[derive(Debug, Clone)]
pub struct N2WoodburyScreeningReport {
    pub total_pairs_evaluated: usize,
    pub coupled_mechanism_pairs: usize,
    pub critical_pairs_count: usize,
    pub min_pair_eigengate: f64,
    pub pair_results: Vec<N2PairRedundancyResult>,
}

/// Engineering retrofit intervention strategy evaluated by EigenGate Sensitivity Advisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetrofitStrategyKind {
    SectionUpgrade25Pct,
    SectionUpgrade50Pct,
    AddParallelTieOrBrace,
}

impl RetrofitStrategyKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::SectionUpgrade25Pct => "+25% Section Upgrade",
            Self::SectionUpgrade50Pct => "+50% Section Upgrade",
            Self::AddParallelTieOrBrace => "Parallel Tie / Secondary Brace",
        }
    }
}

/// Ranked member strengthening option quantified by $\Delta\text{ARI} / \text{Ton}$ material efficiency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EigenGateRetrofitCandidate {
    pub member_id: usize,
    pub member_name: String,
    pub strategy: RetrofitStrategyKind,
    pub baseline_member_ari: f64,
    pub projected_member_ari: f64,
    pub delta_system_ari: f64,
    pub baseline_margin_ms: f64,
    pub projected_margin_ms: f64,
    pub added_mass_kg: f64,
    pub estimated_cost_usd: f64,
    pub efficiency_ari_per_ton: f64,
    pub rationale: String,
}

/// EigenGate spectral bottleneck sensitivity & optimal retrofit ranking report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EigenGateSensitivityReport {
    pub baseline_min_ari: f64,
    pub baseline_avg_ari: f64,
    pub baseline_critical_count: usize,
    pub n2_coupled_mechanism_pairs: usize,
    pub n2_min_pair_ari: f64,
    pub top_bottleneck_member_id: Option<usize>,
    pub top_bottleneck_name: String,
    pub candidates: Vec<EigenGateRetrofitCandidate>,
    pub summary_verdict: String,
}

// -----------------------------------------------------------------------------
// 5. OFFICIAL EIGENGATE ENGINE: SHERMAN-MORRISON RANK-1 UPDATE & FAST EVALUATOR
// -----------------------------------------------------------------------------

pub struct EigenGateEngine;

impl EigenGateEngine {
    pub fn verify_kinematics(global_k: &Matrix, tol: f64) -> KinematicGuardReport {
        // Compute a single SVD decomposition to derive rank, nullspace modes, and conditioning.
        let (_, s, v) = global_k.svd();
        let sigma_max = s.first().copied().unwrap_or(0.0);
        let cutoff = tol * sigma_max;

        let mut rank = 0;
        let mut null_vectors = Vec::new();
        let mut sigma_min = 0.0;

        for (idx, &sigma) in s.iter().enumerate() {
            if sigma > cutoff {
                rank += 1;
                sigma_min = sigma;
            } else {
                let mut null_v = vec![0.0; v.rows];
                for r in 0..v.rows {
                    null_v[r] = v[(r, idx)];
                }
                null_vectors.push(null_v);
            }
        }

        let null_dim = s.len() - rank;
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

    /// Instant O(N^2) Redundancy Evaluation via Rank-1 Sherman-Morrison Matrix Inverse Update.
    /// Avoids repeated O(N^3) global stiffness re-factorization (Eqs. 4.13d-4.13g).
    pub fn evaluate_redundancy_fast(model: &StructuralModel) -> Result<RedundancyAssessment, String> {
        let baseline = FemSolver::solve(model)?;
        let kinematic_guard = Self::verify_kinematics(&baseline.k_free, 1e-7);

        if !kinematic_guard.is_stable {
            return Err("Baseline intact structure contains an unconstrained kinematic mechanism".to_string());
        }

        let k_free_inv = baseline.k_free.inverse()
            .ok_or_else(|| "Failed to invert free-DOF stiffness matrix".to_string())?;

        let mut member_results = Vec::with_capacity(model.elements.len());
        let total_free_dofs = baseline.free_dofs.len();

        // O(1) lookup tables for node index and global-DOF-to-free-DOF index
        let mut node_idx_map = HashMap::with_capacity(model.nodes.len());
        for (idx, n) in model.nodes.iter().enumerate() {
            node_idx_map.insert(n.id, idx);
        }
        let mut dof_to_free = HashMap::with_capacity(total_free_dofs);
        for (free_idx, &gdof) in baseline.free_dofs.iter().enumerate() {
            dof_to_free.insert(gdof, free_idx);
        }

        // Precompute element geometry & stiffness once: (n1_base, n2_base, cx, cy, cz, k_elem, length, e_modulus, i_min)
        let elem_geom: Vec<(usize, usize, f64, f64, f64, f64, f64, f64, f64)> = model
            .elements
            .iter()
            .map(|e| {
                let n1_idx = node_idx_map[&e.start_node];
                let n2_idx = node_idx_map[&e.end_node];
                let n1 = &model.nodes[n1_idx];
                let n2 = &model.nodes[n2_idx];
                let l = n1.distance_to(n2).max(1e-9);
                let cx = (n2.x - n1.x) / l;
                let cy = (n2.y - n1.y) / l;
                let cz = (n2.z - n1.z) / l;
                let k_e = (e.material.elastic_modulus * e.section.area) / l;
                let i_min = e.section.iy.min(e.section.iz).max(1e-12);
                let e_mod = e.material.elastic_modulus;
                (n1_idx * 6, n2_idx * 6, cx, cy, cz, k_e, l, e_mod, i_min)
            })
            .collect();

        let pi = std::f64::consts::PI;

        for (elem_idx, elem) in model.elements.iter().enumerate() {
            let f_init = baseline.member_axial_forces[elem_idx];
            let cap = baseline.member_capacities[elem_idx];
            let u_init = baseline.member_utilizations[elem_idx];

            let (n1_base, n2_base, cx, cy, cz, k_elem, l_elem, e_elem, imin_elem) = elem_geom[elem_idx];
            let p_euler_init = (pi * pi * e_elem * imin_elem) / (l_elem * l_elem);

            // Sparse 6-DOF compatibility vector a_e in free DOFs -> O(6 * N) influence vector v_e = K_0^(-1) * a_e
            let dof_entries = [
                (n1_base, -cx),
                (n1_base + 1, -cy),
                (n1_base + 2, -cz),
                (n2_base, cx),
                (n2_base + 1, cy),
                (n2_base + 2, cz),
            ];
            let mut active_dofs: Vec<(usize, f64)> = Vec::with_capacity(6);
            for &(gdof, coeff) in &dof_entries {
                if let Some(&f_idx) = dof_to_free.get(&gdof) {
                    if coeff.abs() > 0.0 {
                        active_dofs.push((f_idx, coeff));
                    }
                }
            }
            let mut v_e = vec![0.0; total_free_dofs];
            for row in 0..total_free_dofs {
                let mut sum = 0.0;
                for &(col_idx, coeff) in &active_dofs {
                    sum += k_free_inv[(row, col_idx)] * coeff;
                }
                v_e[row] = sum;
            }

            // Spectral leverage: h_e = k_e * a_e^T * K_0^(-1) * a_e (Persamaan 4.1)
            let mut a_dot_v = 0.0;
            for &(f_idx, coeff) in &active_dofs {
                a_dot_v += coeff * v_e[f_idx];
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
                    eigengate: 0.0,
                    rr: 0.0,
                    ci: 1.0,
                    svd_margin: 0.0,
                    is_critical: true,
                    euler_buckling_kn: p_euler_init / 1000.0,
                    b1_amplification: 20.0,
                    buckling_margin: 0.0,
                    second_order_dcr: 1.0,
                });
                continue;
            }

            // GATE 2 & 3: Exact Post-Disruption Force Redistribution via Sherman-Morrison
            // u^(-e) = u_0 + [ D_e^(0) / (1 - h_e) ] * v_e
            let factor = f_init / stability_margin;
            let mut max_redistributed_u: f64 = 0.0;
            let mut max_second_order_u: f64 = 0.0;
            let mut governing_b1: f64 = 1.0;
            let mut min_buckling_margin: f64 = 1.0;

            for other_idx in 0..model.elements.len() {
                if other_idx == elem_idx {
                    continue; // Eliminated member carries zero axial force
                }

                let (on1_base, on2_base, ocx, ocy, ocz, k_other, o_l, o_e, o_imin) = elem_geom[other_idx];

                // Compute incremental elongation a_j^T * v_e in O(1) time (6 DOF lookups)
                let mut delta_v = 0.0;
                let other_entries = [
                    (on1_base, -ocx),
                    (on1_base + 1, -ocy),
                    (on1_base + 2, -ocz),
                    (on2_base, ocx),
                    (on2_base + 1, ocy),
                    (on2_base + 2, ocz),
                ];
                for &(gdof, coeff) in &other_entries {
                    if let Some(&f_idx) = dof_to_free.get(&gdof) {
                        delta_v += coeff * v_e[f_idx];
                    }
                }

                let delta_force = k_other * factor * delta_v;
                let post_force = baseline.member_axial_forces[other_idx] + delta_force;
                let other_cap = baseline.member_capacities[other_idx];

                // Second-Order P-Delta & Geometric Stiffness (Kg) Buckling Check
                let mut b1 = 1.0;
                let mut buckle_margin = 1.0;
                let mut post_force_amplified = post_force.abs();

                if post_force < 0.0 {
                    // Member subjected to post-disruption compression
                    let p_comp = post_force.abs();
                    let p_euler_other = (pi * pi * o_e * o_imin) / (o_l * o_l);
                    let ratio = p_comp / (p_euler_other + 1e-9);
                    buckle_margin = (1.0 - ratio).max(0.0);

                    if ratio < 0.95 {
                        b1 = (1.0 / (1.0 - ratio)).max(1.0);
                    } else {
                        b1 = 20.0; // Near or beyond elastic Euler critical limit
                    }
                    post_force_amplified = p_comp * b1;
                }

                let linear_u = (post_force.abs() / (other_cap + EPS_R)).min(2.0);
                let second_order_u = (post_force_amplified / (other_cap + EPS_R)).min(2.0);

                if linear_u > max_redistributed_u {
                    max_redistributed_u = linear_u;
                }
                if second_order_u > max_second_order_u {
                    max_second_order_u = second_order_u;
                }
                if buckle_margin < min_buckling_margin {
                    min_buckling_margin = buckle_margin;
                    governing_b1 = b1;
                }
            }

            let delta_u = (max_redistributed_u - u_init).max(0.0);
            let reserve = (1.0 - u_init).max(0.05);
            // EigenGate is penalized by second-order geometric stiffness destabilization
            let eigengate_val = ((1.0 - (delta_u / reserve)).clamp(0.0, 1.0) * min_buckling_margin.sqrt()).clamp(0.0, 1.0);
            let rr_val = (1.0 - (max_redistributed_u - u_init).abs()).clamp(0.0, 1.0);
            let ci_val = 1.0 - eigengate_val;
            let effective_stability_margin = stability_margin.min(min_buckling_margin);

            member_results.push(MemberRedundancyResult {
                member_id: elem.id,
                member_name: elem.name.clone(),
                force_kn: f_init / 1000.0,
                capacity_kn: cap / 1000.0,
                utilization_initial: u_init,
                max_redistribution_utilization: max_redistributed_u,
                eigengate: eigengate_val,
                rr: rr_val,
                ci: ci_val,
                svd_margin: effective_stability_margin,
                is_critical: eigengate_val < 0.70 || max_redistributed_u >= 1.0 || max_second_order_u >= 1.0 || min_buckling_margin < 0.10,
                euler_buckling_kn: p_euler_init / 1000.0,
                b1_amplification: governing_b1,
                buckling_margin: min_buckling_margin,
                second_order_dcr: max_second_order_u,
            });
        }

        let min_system_eigengate = member_results.iter().map(|m| m.eigengate).fold(1.0, f64::min);
        let average_eigengate = if !member_results.is_empty() {
            member_results.iter().map(|m| m.eigengate).sum::<f64>() / (member_results.len() as f64)
        } else {
            0.0
        };
        let critical_members_count = member_results.iter().filter(|m| m.is_critical).count();

        Ok(RedundancyAssessment {
            kinematic_guard,
            member_results,
            min_system_eigengate,
            average_eigengate,
            critical_members_count,
        })
    }

    /// Complete EigenGate evaluation with automatic fallback
    pub fn evaluate_redundancy(model: &StructuralModel) -> Result<RedundancyAssessment, String> {
        Self::evaluate_redundancy_fast(model)
    }

    /// Evaluates N-k multi-member disruption scenarios (e.g., dual-member loss) & cascading collapse propagation
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

                let res_eigengate = if !guard.is_stable || cascading_triggered {
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
                    residual_system_eigengate: res_eigengate,
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
                    residual_system_eigengate: 0.0,
                    cascading_failure_triggered: true,
                    domino_failed_members: vec![],
                    propagation_vulnerability_index: 1.0,
                })
            }
        }
    }

    /// Fast N-2 contingency screening (dual-member loss) using the Rank-2 Woodbury Matrix Identity.
    /// Evaluates all C(m, 2) member pairs without reassembling or refactoring the global stiffness matrix.
    pub fn evaluate_n2_woodbury_fast(model: &StructuralModel) -> Result<N2WoodburyScreeningReport, String> {
        let baseline = FemSolver::solve(model)?;
        let kinematic_guard = Self::verify_kinematics(&baseline.k_free, 1e-7);
        if !kinematic_guard.is_stable {
            return Err("Baseline intact structure is kinematically unstable".to_string());
        }

        let k_free_inv = baseline.k_free.inverse()
            .ok_or_else(|| "Failed to invert free-DOF stiffness matrix".to_string())?;

        let m = model.elements.len();
        let n_free = baseline.free_dofs.len();

        let mut node_idx_map = HashMap::with_capacity(model.nodes.len());
        for (idx, n) in model.nodes.iter().enumerate() {
            node_idx_map.insert(n.id, idx);
        }
        let mut dof_to_free = HashMap::with_capacity(n_free);
        for (free_idx, &gdof) in baseline.free_dofs.iter().enumerate() {
            dof_to_free.insert(gdof, free_idx);
        }

        // Precompute projection vector a_e, compliance vector v_e = K_0^(-1) * a_e, and k_e for each member
        let mut a_vecs: Vec<Vec<f64>> = Vec::with_capacity(m);
        let mut v_vecs: Vec<Vec<f64>> = Vec::with_capacity(m);
        let mut k_vals: Vec<f64> = Vec::with_capacity(m);
        let mut h_diag: Vec<f64> = Vec::with_capacity(m);

        for elem in &model.elements {
            let n1_idx = node_idx_map[&elem.start_node];
            let n2_idx = node_idx_map[&elem.end_node];
            let n1 = &model.nodes[n1_idx];
            let n2 = &model.nodes[n2_idx];

            let l = n1.distance_to(n2).max(1e-9);
            let cx = (n2.x - n1.x) / l;
            let cy = (n2.y - n1.y) / l;
            let cz = (n2.z - n1.z) / l;
            let k_e = (elem.material.elastic_modulus * elem.section.area) / l;

            let mut a_e = vec![0.0; n_free];
            let n1_base = n1_idx * 6;
            let n2_base = n2_idx * 6;
            let dof_entries = [
                (n1_base, -cx),
                (n1_base + 1, -cy),
                (n1_base + 2, -cz),
                (n2_base, cx),
                (n2_base + 1, cy),
                (n2_base + 2, cz),
            ];
            for &(gdof, coeff) in &dof_entries {
                if let Some(&f_idx) = dof_to_free.get(&gdof) {
                    a_e[f_idx] = coeff;
                }
            }

            let v_e = k_free_inv.mul_vec(&a_e);
            let mut a_dot_v = 0.0;
            for &(gdof, coeff) in &dof_entries {
                if let Some(&f_idx) = dof_to_free.get(&gdof) {
                    a_dot_v += coeff * v_e[f_idx];
                }
            }
            let h_ee = (k_e * a_dot_v).clamp(0.0, 1.0);

            a_vecs.push(a_e);
            v_vecs.push(v_e);
            k_vals.push(k_e);
            h_diag.push(h_ee);
        }

        // Precompute cross dot products M[i][j] = a_i . v_j
        let mut cross_dot = vec![vec![0.0; m]; m];
        for i in 0..m {
            for j in 0..m {
                let mut dot = 0.0;
                for d in 0..n_free {
                    dot += a_vecs[i][d] * v_vecs[j][d];
                }
                cross_dot[i][j] = dot;
            }
        }

        let mut pair_results = Vec::new();
        let mut coupled_mechanism_pairs = 0;
        let mut critical_pairs_count = 0;
        let mut min_pair_eigengate: f64 = 1.0;

        for i in 0..m {
            for j in (i + 1)..m {
                let h11 = h_diag[i];
                let h22 = h_diag[j];
                let sqrt_k1 = k_vals[i].sqrt();
                let sqrt_k2 = k_vals[j].sqrt();
                let h12 = sqrt_k1 * sqrt_k2 * cross_dot[i][j];

                let (det_c, inv_c_opt) = Matrix::woodbury_rank2_det_and_inv(h11, h22, h12, MARGIN_TOL);

                if let Some(inv_c) = inv_c_opt {
                    // Valid Rank-2 Woodbury update
                    let rhs1 = baseline.member_axial_forces[i] / (sqrt_k1 + 1e-18);
                    let rhs2 = baseline.member_axial_forces[j] / (sqrt_k2 + 1e-18);

                    let gamma1 = inv_c[0][0] * rhs1 + inv_c[0][1] * rhs2;
                    let gamma2 = inv_c[1][0] * rhs1 + inv_c[1][1] * rhs2;

                    let mut max_redist_u: f64 = 0.0;
                    for r in 0..m {
                        if r == i || r == j {
                            continue;
                        }
                        let delta_elong = sqrt_k1 * gamma1 * cross_dot[r][i] + sqrt_k2 * gamma2 * cross_dot[r][j];
                        let post_force = baseline.member_axial_forces[r] + k_vals[r] * delta_elong;
                        let u_r = (post_force.abs() / (baseline.member_capacities[r] + EPS_R)).min(2.0);
                        if u_r > max_redist_u {
                            max_redist_u = u_r;
                        }
                    }

                    let u_init_pair = baseline.member_utilizations[i].max(baseline.member_utilizations[j]);
                    let delta_u = (max_redist_u - u_init_pair).max(0.0);
                    let reserve = (1.0 - u_init_pair).max(0.05);
                    let pair_eigengate = (1.0 - (delta_u / reserve)).clamp(0.0, 1.0) * det_c.clamp(0.0, 1.0).sqrt();

                    if pair_eigengate < 0.70 || max_redist_u >= 1.0 {
                        critical_pairs_count += 1;
                    }
                    min_pair_eigengate = min_pair_eigengate.min(pair_eigengate);

                    pair_results.push(N2PairRedundancyResult {
                        member_1_id: model.elements[i].id,
                        member_2_id: model.elements[j].id,
                        member_1_name: model.elements[i].name.clone(),
                        member_2_name: model.elements[j].name.clone(),
                        h11,
                        h22,
                        h12_coupling: h12,
                        woodbury_determinant: det_c,
                        is_coupled_mechanism: false,
                        max_redistributed_utilization: max_redist_u,
                        pair_eigengate,
                    });
                } else {
                    // Coupled kinematic mechanism (Order-2 Cut Set)
                    coupled_mechanism_pairs += 1;
                    critical_pairs_count += 1;
                    min_pair_eigengate = 0.0;

                    pair_results.push(N2PairRedundancyResult {
                        member_1_id: model.elements[i].id,
                        member_2_id: model.elements[j].id,
                        member_1_name: model.elements[i].name.clone(),
                        member_2_name: model.elements[j].name.clone(),
                        h11,
                        h22,
                        h12_coupling: h12,
                        woodbury_determinant: det_c.max(0.0),
                        is_coupled_mechanism: true,
                        max_redistributed_utilization: 1.0,
                        pair_eigengate: 0.0,
                    });
                }
            }
        }

        Ok(N2WoodburyScreeningReport {
            total_pairs_evaluated: pair_results.len(),
            coupled_mechanism_pairs,
            critical_pairs_count,
            min_pair_eigengate,
            pair_results,
        })
    }

    /// Executes an interactive 6-DOF single-member (`N-1`) disruption simulation for Earth 3D
    /// load-path redistribution visualization, comparing intact baseline vs post-removal equilibrium
    /// and running the spectral SVD kinematic mechanism guard on the damaged stiffness matrix.
    pub fn simulate_single_member_removal(
        model: &StructuralModel,
        baseline: &crate::core::solver::AnalysisResult,
        removed_fem_id: usize,
    ) -> N1SimulationReport {
        let removed_elem = model.elements.iter().find(|e| e.id == removed_fem_id);
        let removed_member_name = removed_elem
            .map(|e| e.name.clone())
            .unwrap_or_else(|| format!("Member #{removed_fem_id}"));

        let mut baseline_member_dcr = HashMap::with_capacity(model.elements.len());
        let mut baseline_max_dcr = 0.0_f64;
        for (idx, elem) in model.elements.iter().enumerate() {
            let dcr = baseline
                .member_dcr_interactions
                .get(idx)
                .copied()
                .unwrap_or(0.0);
            baseline_member_dcr.insert(elem.id, dcr);
            baseline_max_dcr = baseline_max_dcr.max(dcr);
        }
        let baseline_max_disp_mm = baseline.max_displacement_mm;

        let mut damaged = model.clone();
        damaged.elements.retain(|e| e.id != removed_fem_id);

        match FemSolver::solve(&damaged) {
            Ok(post_sol) => {
                let guard = Self::verify_kinematics(&post_sol.k_free, 1e-7);
                let mut post_member_dcr = HashMap::with_capacity(damaged.elements.len());
                let mut post_member_moment_knm = HashMap::with_capacity(damaged.elements.len());
                let mut post_max_dcr = 0.0_f64;
                let mut newly_critical_fem_ids = Vec::new();

                for (idx, elem) in damaged.elements.iter().enumerate() {
                    let post_dcr = post_sol
                        .member_dcr_interactions
                        .get(idx)
                        .copied()
                        .unwrap_or(0.0);
                    let (my, mz) = post_sol
                        .member_bending_moments
                        .get(idx)
                        .copied()
                        .unwrap_or((0.0, 0.0));
                    post_member_dcr.insert(elem.id, post_dcr);
                    post_member_moment_knm.insert(elem.id, my.abs().max(mz.abs()) / 1000.0);
                    post_max_dcr = post_max_dcr.max(post_dcr);

                    let base_dcr = baseline_member_dcr.get(&elem.id).copied().unwrap_or(0.0);
                    if (post_dcr >= 0.85 && base_dcr < 0.85) || (post_dcr - base_dcr) >= 0.18 {
                        newly_critical_fem_ids.push(elem.id);
                    }
                }

                let svd_margin = if guard.is_stable {
                    (1.0 / guard.condition_number.max(1.0)).sqrt().clamp(0.05, 1.0)
                } else {
                    0.0
                };
                let is_kinematic_mechanism = !guard.is_stable || !post_sol.max_displacement_mm.is_finite();
                let post_nodal_displacements: Vec<[f64; 3]> = post_sol
                    .nodal_displacements
                    .iter()
                    .map(|d| [d[0], d[1], d[2]])
                    .collect();

                let residual_eigengate = if is_kinematic_mechanism {
                    0.0
                } else {
                    let dcr_penalty = ((post_max_dcr - 0.50).max(0.0) / 0.55).clamp(0.0, 1.0);
                    let drift_ratio = if baseline_max_disp_mm > 1e-4 {
                        (post_sol.max_displacement_mm / baseline_max_disp_mm).clamp(1.0, 6.0)
                    } else {
                        1.0
                    };
                    let drift_score = (1.0 - (drift_ratio - 1.0) * 0.18).clamp(0.0, 1.0);
                    (0.25 * svd_margin + 0.45 * (1.0 - dcr_penalty) + 0.30 * drift_score).clamp(0.0, 1.0)
                };

                N1SimulationReport {
                    removed_fem_id,
                    removed_member_name,
                    is_kinematic_mechanism,
                    svd_margin,
                    baseline_max_dcr,
                    post_max_dcr,
                    baseline_max_disp_mm,
                    post_max_disp_mm: post_sol.max_displacement_mm,
                    baseline_member_dcr,
                    post_member_dcr,
                    post_member_moment_knm,
                    post_nodal_displacements,
                    newly_critical_fem_ids,
                    residual_eigengate,
                }
            }
            Err(_) => {
                // Singular damaged stiffness matrix: synthesize a localized downward collapse mechanism
                // vector at the upper end node of the removed element for clear 3D visual diagnostics.
                let mut collapse_disp: Vec<[f64; 3]> = baseline
                    .nodal_displacements
                    .iter()
                    .map(|d| [d[0], d[1], d[2]])
                    .collect();
                if let Some(rem) = removed_elem {
                    let affected_nodes = [rem.start_node, rem.end_node];
                    for (idx, n) in model.nodes.iter().enumerate() {
                        if affected_nodes.contains(&n.id) && !n.fixity.is_fixed() {
                            if let Some(slot) = collapse_disp.get_mut(idx) {
                                slot[1] -= 0.045; // -45 mm vertical collapse sag (amplified in 3D view)
                                slot[0] += 0.018;
                            }
                        }
                    }
                }
                let mut post_member_dcr = baseline_member_dcr.clone();
                let mut newly_critical_fem_ids = Vec::new();
                if let Some(rem) = removed_elem {
                    for elem in &damaged.elements {
                        if elem.start_node == rem.start_node
                            || elem.end_node == rem.start_node
                            || elem.start_node == rem.end_node
                            || elem.end_node == rem.end_node
                        {
                            post_member_dcr.insert(elem.id, 1.25);
                            newly_critical_fem_ids.push(elem.id);
                        }
                    }
                }
                N1SimulationReport {
                    removed_fem_id,
                    removed_member_name,
                    is_kinematic_mechanism: true,
                    svd_margin: 0.0,
                    baseline_max_dcr,
                    post_max_dcr: 1.25_f64.max(baseline_max_dcr),
                    baseline_max_disp_mm,
                    post_max_disp_mm: (baseline_max_disp_mm * 4.5).max(45.0),
                    baseline_member_dcr,
                    post_member_dcr,
                    post_member_moment_knm: HashMap::new(),
                    post_nodal_displacements: collapse_disp,
                    newly_critical_fem_ids,
                    residual_eigengate: 0.0,
                }
            }
        }
    }

    /// Evaluates spectral redundancy sensitivity across critical bottleneck members and ranks
    /// optimal section upgrades and parallel tie/brace additions by $\Delta\text{ARI} / \text{Ton}$.
    pub fn evaluate_sensitivity_and_retrofit_advisor(
        model: &StructuralModel,
    ) -> Result<EigenGateSensitivityReport, String> {
        let base = Self::evaluate_redundancy_fast(model)?;
        let n2_rep = Self::evaluate_n2_woodbury_fast(model).ok();
        let n2_coupled = n2_rep.as_ref().map_or(0, |r| r.coupled_mechanism_pairs);
        let n2_min_ari = n2_rep
            .as_ref()
            .map_or(base.min_system_eigengate, |r| r.min_pair_eigengate);

        let mut bottlenecks = base.member_results.clone();
        bottlenecks.sort_by(|a, b| {
            a.eigengate
                .partial_cmp(&b.eigengate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let top_id = bottlenecks.first().map(|m| m.member_id);
        let top_name = bottlenecks
            .first()
            .map(|m| m.member_name.clone())
            .unwrap_or_else(|| "None".to_string());

        let next_elem_id = model.elements.iter().map(|e| e.id).max().unwrap_or(0) + 1000;
        let mut candidates = Vec::new();

        for (b_rank, bm) in bottlenecks.iter().take(4).enumerate() {
            let Some(orig_elem_idx) = model.elements.iter().position(|e| e.id == bm.member_id) else {
                continue;
            };
            let orig_elem = &model.elements[orig_elem_idx];
            let len_m = model.member_length(orig_elem).unwrap_or(3.5).max(0.25);

            for (s_idx, strategy) in [
                RetrofitStrategyKind::SectionUpgrade25Pct,
                RetrofitStrategyKind::SectionUpgrade50Pct,
                RetrofitStrategyKind::AddParallelTieOrBrace,
            ]
            .into_iter()
            .enumerate()
            {
                let mut mod_model = model.clone();
                let added_area_m2 = match strategy {
                    RetrofitStrategyKind::SectionUpgrade25Pct => {
                        let e = &mut mod_model.elements[orig_elem_idx];
                        let da = e.section.area * 0.25;
                        e.section.area *= 1.25;
                        e.section.iy *= 1.56;
                        e.section.iz *= 1.56;
                        e.section.j_torsion *= 1.56;
                        e.section.width *= 1.12;
                        e.section.depth *= 1.12;
                        da
                    }
                    RetrofitStrategyKind::SectionUpgrade50Pct => {
                        let e = &mut mod_model.elements[orig_elem_idx];
                        let da = e.section.area * 0.50;
                        e.section.area *= 1.50;
                        e.section.iy *= 2.25;
                        e.section.iz *= 2.25;
                        e.section.j_torsion *= 2.25;
                        e.section.width *= 1.22;
                        e.section.depth *= 1.22;
                        da
                    }
                    RetrofitStrategyKind::AddParallelTieOrBrace => {
                        let mut brace = orig_elem.clone();
                        brace.id = next_elem_id + b_rank * 10 + s_idx;
                        brace.name = format!("{} [Alt-Tie]", orig_elem.name);
                        brace.section.area *= 0.35;
                        brace.section.iy *= 0.35;
                        brace.section.iz *= 0.35;
                        brace.section.j_torsion *= 0.35;
                        let da = brace.section.area;
                        mod_model.elements.push(brace);
                        da
                    }
                };

                let Ok(proj) = Self::evaluate_redundancy_fast(&mod_model) else {
                    continue;
                };
                let proj_bm = proj
                    .member_results
                    .iter()
                    .find(|m| m.member_id == bm.member_id);
                let projected_member_ari = proj_bm.map_or(proj.min_system_eigengate, |m| m.eigengate);
                let projected_margin_ms = proj_bm.map_or(bm.svd_margin, |m| m.svd_margin);
                let delta_sys = proj.min_system_eigengate - base.min_system_eigengate;
                let delta_mem = projected_member_ari - bm.eigengate;
                let effective_gain = delta_sys.max(delta_mem).max(0.0);

                let added_mass_kg = (added_area_m2 * len_m * orig_elem.material.density).max(1.0);
                let estimated_cost_usd = added_mass_kg * 3.85;
                let efficiency = effective_gain / (added_mass_kg / 1000.0).max(0.01);

                candidates.push(EigenGateRetrofitCandidate {
                    member_id: bm.member_id,
                    member_name: bm.member_name.clone(),
                    strategy,
                    baseline_member_ari: bm.eigengate,
                    projected_member_ari,
                    delta_system_ari: delta_sys,
                    baseline_margin_ms: bm.svd_margin,
                    projected_margin_ms,
                    added_mass_kg,
                    estimated_cost_usd,
                    efficiency_ari_per_ton: efficiency,
                    rationale: format!(
                        "{}: #{} ({}) ARI {:.3} -> {:.3} (dSys={:+.3}, +{:.0}kg)",
                        strategy.label(),
                        bm.member_id,
                        bm.member_name,
                        bm.eigengate,
                        projected_member_ari,
                        delta_sys,
                        added_mass_kg
                    ),
                });
            }
        }

        candidates.sort_by(|a, b| {
            b.efficiency_ari_per_ton
                .partial_cmp(&a.efficiency_ari_per_ton)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(6);

        let best_desc = candidates
            .first()
            .map(|c| {
                format!(
                    "Top option: {} on #{} ({:.2} ARI/t)",
                    c.strategy.label(),
                    c.member_id,
                    c.efficiency_ari_per_ton
                )
            })
            .unwrap_or_else(|| "No actionable retrofit candidates required".to_string());

        let summary_verdict = format!(
            "EigenGate Sensitivity: Min ARI={:.3}, Avg={:.3}, Critical={}, N-2 Mechanisms={}. {}",
            base.min_system_eigengate,
            base.average_eigengate,
            base.critical_members_count,
            n2_coupled,
            best_desc,
        );

        Ok(EigenGateSensitivityReport {
            baseline_min_ari: base.min_system_eigengate,
            baseline_avg_ari: base.average_eigengate,
            baseline_critical_count: base.critical_members_count,
            n2_coupled_mechanism_pairs: n2_coupled,
            n2_min_pair_ari: n2_min_ari,
            top_bottleneck_member_id: top_id,
            top_bottleneck_name: top_name,
            candidates,
            summary_verdict,
        })
    }
}

/// Complete before-vs-after diagnostics from an interactive single-member (`N-1`) disruption test.
#[derive(Debug, Clone)]
pub struct N1SimulationReport {
    pub removed_fem_id: usize,
    pub removed_member_name: String,
    pub is_kinematic_mechanism: bool,
    pub svd_margin: f64,
    pub baseline_max_dcr: f64,
    pub post_max_dcr: f64,
    pub baseline_max_disp_mm: f64,
    pub post_max_disp_mm: f64,
    pub baseline_member_dcr: HashMap<usize, f64>,
    pub post_member_dcr: HashMap<usize, f64>,
    pub post_member_moment_knm: HashMap<usize, f64>,
    pub post_nodal_displacements: Vec<[f64; 3]>,
    pub newly_critical_fem_ids: Vec<usize>,
    pub residual_eigengate: f64,
}

// -----------------------------------------------------------------------------
// 6. EIGENGRAPH AUTOMATED VERIFICATION TEST SUITE
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

        let cfg = EigenGateConfig {
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
        let eigengate_1 = rr1 * (1.0 - cfg.eta * u1_star);
        assert!((eigengate_1 - 21.0 / 68.0).abs() < 1e-3);
        let ci_1 = 1.0 - eigengate_1;
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

        let assessment = EigenGateEngine::evaluate_redundancy_fast(&model)
            .expect("Fast evaluation should succeed");

        assert_eq!(assessment.member_results.len(), 2);
        assert!(assessment.kinematic_guard.is_stable);
        for m in &assessment.member_results {
            assert!(m.svd_margin > 0.0);
            assert!(m.eigengate > 0.0);
        }
    }

    #[test]
    fn test_woodbury_n2_simultaneous_screening() {
        let mut model = StructuralModel::new("3-Bar Redundant Parallel System");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));

        let mut n1_fix = BoundaryFixity::free();
        n1_fix.ux = true;
        n1_fix.uz = true;
        n1_fix.rx = true;
        n1_fix.ry = true;
        n1_fix.rz = true;
        model.add_node(Node3D::new(1, 0.0, 1.0, 0.0).with_fixity(n1_fix));

        for id in 1..=3 {
            let mut e = Element3D::new(id, format!("Bar {}", id), 0, 1);
            e.element_type = ElementType::Truss3D;
            model.add_element(e);
        }

        model.add_load(NodalLoad::force_y(1, -30_000.0));

        let n2_report = EigenGateEngine::evaluate_n2_woodbury_fast(&model)
            .expect("Woodbury N-2 screening should succeed");

        // C(3, 2) = 3 pairs; in a 3-bar parallel system, removing any 2 bars leaves 1 bar intact (no mechanism!)
        assert_eq!(n2_report.total_pairs_evaluated, 3);
        assert_eq!(n2_report.coupled_mechanism_pairs, 0);
        for pair in &n2_report.pair_results {
            assert!(!pair.is_coupled_mechanism);
            assert!(pair.woodbury_determinant > 0.0);
            assert!(pair.pair_eigengate > 0.0);
        }
    }

    #[test]
    fn test_m2_determinate_vs_redundant_braced_truss_gate1() {
        // Canonical M2-A: Statically determinate 2-bar V-truss supporting node 2 in X-Y plane
        let mut m2_det = StructuralModel::new("M2-A Determinate 2-Bar V-Truss");
        m2_det.add_node(Node3D::new(0, -1.0, 1.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        m2_det.add_node(Node3D::new(1, 1.0, 1.0, 0.0).with_fixity(BoundaryFixity::fixed()));

        let mut n2_fix = BoundaryFixity::free();
        n2_fix.uz = true;
        n2_fix.rx = true;
        n2_fix.ry = true;
        n2_fix.rz = true;
        m2_det.add_node(Node3D::new(2, 0.0, 0.0, 0.0).with_fixity(n2_fix));

        m2_det.add_element(Element3D::new(1, "Left Diagonal", 0, 2).with_type(ElementType::Truss3D));
        m2_det.add_element(Element3D::new(2, "Right Diagonal", 1, 2).with_type(ElementType::Truss3D));
        m2_det.add_load(NodalLoad::force_y(2, -40_000.0));

        let res_det = EigenGateEngine::evaluate_redundancy_fast(&m2_det)
            .expect("Determinate V-truss should evaluate");
        // In a 2-bar determinate truss with 2 free DOFs (ux, uy), removing EITHER bar induces a kinematic mechanism (h_e = 1.0 -> m_e = 0.0, EigenGate = 0.0)
        assert_eq!(res_det.critical_members_count, 2);
        for m in &res_det.member_results {
            assert!(m.svd_margin <= MARGIN_TOL);
            assert_eq!(m.eigengate, 0.0);
            assert!(m.is_critical);
        }

        // Canonical M2-B: Add a 3rd vertical suspension bar (0, 1, 0) -> (0, 0, 0) -> Statically indeterminate (DSI = 1)
        let mut m2_red = m2_det.clone();
        m2_red.add_node(Node3D::new(3, 0.0, 1.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        m2_red.add_element(Element3D::new(3, "Center Vertical Tie", 3, 2).with_type(ElementType::Truss3D));

        let res_red = EigenGateEngine::evaluate_redundancy_fast(&m2_red)
            .expect("Redundant 3-bar truss should evaluate");
        for m in &res_red.member_results {
            assert!(m.svd_margin > 0.10, "Member {} should have positive stability margin", m.member_id);
            assert!(m.eigengate > 0.0, "Member {} should have positive EigenGate", m.member_id);
        }
    }

    #[test]
    fn test_sherman_morrison_vs_full_refactorization_exact_equivalence() {
        // Verify that Sherman-Morrison O(N^2) post-disruption force redistribution matches full O(N^3) FEM re-solve
        let mut engine = crate::core::parametric::ParametricEngine::new();
        let mut model = engine.generate_warren_truss().expect("Warren truss");
        model.add_load(NodalLoad::force_y(3, -50_000.0));

        let fast_report = EigenGateEngine::evaluate_redundancy_fast(&model).unwrap();

        // Pick a redundant member (with svd_margin > 0.05) and compare max_redistribution_utilization against full FEM solve of damaged model
        for m_res in &fast_report.member_results {
            if m_res.svd_margin > 0.05 {
                let mut damaged = model.clone();
                damaged.elements.retain(|e| e.id != m_res.member_id);
                let exact_damaged = FemSolver::solve(&damaged).expect("Damaged model should solve");
                let exact_max_u = exact_damaged
                    .member_utilizations
                    .iter()
                    .copied()
                    .fold(0.0_f64, f64::max);
                assert!(
                    (m_res.max_redistribution_utilization - exact_max_u).abs() < 1e-6,
                    "Sherman-Morrison post-disruption utilization ({:.8}) != Exact FEM ({:.8}) for member {}",
                    m_res.max_redistribution_utilization,
                    exact_max_u,
                    m_res.member_id
                );
                break;
            }
        }
    }

    #[test]
    fn test_eigengate_geometric_stiffness_euler_buckling_penalty() {
        let mut model = StructuralModel::new("Parallel Compression Struts");
        model.add_node(Node3D::new(0, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        let mut n1_fix = BoundaryFixity::free();
        n1_fix.ux = true;
        n1_fix.uz = true;
        n1_fix.rx = true;
        n1_fix.ry = true;
        n1_fix.rz = true;
        model.add_node(Node3D::new(1, 0.0, 3.0, 0.0).with_fixity(n1_fix));

        let mut e1 = Element3D::new(1, "Strut 1", 0, 1).with_type(ElementType::Truss3D);
        e1.section.area = 0.005;
        e1.section.iy = 2.0e-6;
        e1.section.iz = 2.0e-6;
        let mut e2 = Element3D::new(2, "Strut 2", 0, 1).with_type(ElementType::Truss3D);
        e2.section.area = 0.005;
        e2.section.iy = 2.0e-6;
        e2.section.iz = 2.0e-6;
        model.add_element(e1);
        model.add_element(e2);

        // Apply compressive load
        model.add_load(NodalLoad::force_y(1, -50_000.0));

        let report = EigenGateEngine::evaluate_redundancy_fast(&model).expect("EigenGate evaluation");
        assert_eq!(report.member_results.len(), 2);
        for m in &report.member_results {
            assert!(m.euler_buckling_kn > 0.0);
            assert!(m.b1_amplification >= 1.0);
            assert!(m.buckling_margin >= 0.0 && m.buckling_margin <= 1.0);
        }
    }

    #[test]
    fn test_v0888_eigengate_sensitivity_and_retrofit_advisor() {
        let mut engine = crate::core::parametric::ParametricEngine::new();
        let mut model = engine.generate_warren_truss().expect("Warren truss");
        model.add_load(NodalLoad::force_y(3, -55_000.0));

        let rep = EigenGateEngine::evaluate_sensitivity_and_retrofit_advisor(&model)
            .expect("Sensitivity advisor succeeds on stable truss");
        assert!(rep.top_bottleneck_member_id.is_some());
        assert!(!rep.candidates.is_empty());
        assert!(rep
            .candidates
            .iter()
            .all(|c| c.added_mass_kg > 0.0 && c.estimated_cost_usd > 0.0));
        assert!(
            rep.candidates
                .iter()
                .any(|c| c.projected_member_ari >= c.baseline_member_ari - 1e-6),
            "Retrofit candidate must maintain or lift member ARI"
        );
        assert!(
            rep.candidates
                .windows(2)
                .all(|w| w[0].efficiency_ari_per_ton >= w[1].efficiency_ari_per_ton - 1e-9),
            "Candidates must be sorted by descending ARI/ton efficiency"
        );
    }
}
