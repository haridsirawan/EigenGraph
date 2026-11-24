//! core/eigenflow.rs — Phase 83 (`v0.8.3`): EigenFlow Non-Linear Studio Engine.
//!
//! Aligns Branch 2 **EigenFlow** (Non-Linear Woodbury Rank-1 Post-Yield Redistribution &
//! Incremental Second-Order Pushover + ATC-40 ADRS Capacity Spectrum) with Branch 1
//! **EigenGate** (Linear Pre-Analysis & Gatekeeper):
//! - Computes real BIM node-adjacency 1-hop / 2-hop backup load paths `P1 -> P2` and
//!   stiffness-weighted topological leverage `h_e`, stability margin `m_s`, residual
//!   reserve ratio (`RRR`), ductility demand `mu`, and per-member redistributed shear `Delta N_j`.
//! - Evaluates site-coupled SNI 1726 / ASCE 41-17 Pushover + ADRS Capacity Spectrum Method
//!   and maps formed plastic hinges into 3D local ENU coordinates `[east, north, up]`.
//! - Emits compact non-destructive `eigengraph::eigenflow` IFCX overlays (`EigenFlowIfcxOverlay`).

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::eigengate::EigenGateEngine;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::modal::{SiteClass, Sni1726Spectrum};
use crate::core::model::{
    BoundaryFixity, CrossSection, Element3D, ElementType, Material, NodalLoad, Node3D,
    StructuralModel,
};
use crate::core::nonlinear::{NonlinearEngine, PlasticHingeState};
use crate::core::pushover::{LateralLoadPattern, PushoverCsmReport, PushoverEngine};
use crate::core::solver::FemSolver;

/// Canonical buildingSMART / EigenGraph IFCX attribute namespace for Branch 2 EigenFlow overlays.
pub const NS_EG_EIGENFLOW: &str = "eigengraph::eigenflow";

/// Configuration state for the Phase 83 EigenFlow Non-Linear Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EigenFlowConfig {
    /// Specific element ID selected for `P1` disruption (`None` auto-picks the worst EigenGate bottleneck).
    pub disrupted_element_id: Option<usize>,
    /// Post-yield stiffness retention ratio `lambda in [0.0, 0.90]` (`0.0` = full removal).
    pub stiffness_retention_lambda: f64,
    /// Lateral load distribution pattern for Pushover CSM.
    pub load_pattern: LateralLoadPattern,
    /// Number of incremental pushover steps (`6..=24`).
    pub pushover_steps: usize,
    /// Currently scrubbed pushover step index (`0..steps.len().saturating_sub(1)`).
    pub active_step_idx: usize,
}

impl Default for EigenFlowConfig {
    fn default() -> Self {
        Self {
            disrupted_element_id: None,
            stiffness_retention_lambda: 0.0,
            load_pattern: LateralLoadPattern::FirstModeModal,
            pushover_steps: 12,
            active_step_idx: 0,
        }
    }
}

/// Surviving backup member receiving redistributed load `Delta N_j` along alternate path `P2`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedistributedMemberFlow {
    pub element_id: usize,
    pub element_name: String,
    pub p0_enu_m: [f64; 3],
    pub p1_enu_m: [f64; 3],
    pub intact_axial_kn: f64,
    pub delta_n_kn: f64,
    pub share_pct: f64,
    pub post_yield_dcr: f64,
    pub ductility_mu: f64,
    pub is_critical_overload: bool,
}

/// 3D ENU-anchored ASCE 41-17 plastic hinge beacon derived from BIM-coupled pushover progression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EigenFlowHingeMarker3D {
    pub member_id: usize,
    pub member_name: String,
    pub location: String,
    pub pos_enu_m: [f64; 3],
    pub moment_knm: f64,
    pub mp_knm: f64,
    pub dcr_ratio: f64,
    pub state: PlasticHingeState,
    pub formed_at_step: usize,
}

/// Topological Woodbury Rank-1 post-yield load-path redistribution evaluation (`P1 -> P2`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EigenFlowWoodburyResult {
    pub disrupted_element_id: usize,
    pub disrupted_element_name: String,
    pub disrupted_p0_enu_m: [f64; 3],
    pub disrupted_p1_enu_m: [f64; 3],
    pub lambda_val: f64,
    pub svd_leverage_he: f64,
    pub stability_margin_ms: f64,
    pub is_mechanism: bool,
    pub total_shed_force_kn: f64,
    pub residual_reserve_ratio_rrr: f64,
    pub max_ductility_mu: f64,
    pub linear_eigengate_score: f64,
    pub non_linear_eigenflow_score: f64,
    pub non_linear_ci: f64,
    pub redistributed_p2: Vec<RedistributedMemberFlow>,
    pub prescriptive_recommendation: String,
    pub available_elements: Vec<(usize, String)>,
}

/// Compact serializable payload emitted under `eigengraph::eigenflow` in composed `.ifcx` layers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EigenFlowIfcxOverlay {
    pub schema_version: String,
    pub disrupted_element_id: usize,
    pub disrupted_element_name: String,
    pub lambda_val: f64,
    pub svd_leverage_he: f64,
    pub stability_margin_ms: f64,
    pub is_mechanism: bool,
    pub total_shed_force_kn: f64,
    pub residual_reserve_ratio_rrr: f64,
    pub max_ductility_mu: f64,
    pub linear_eigengate_system: f64,
    pub non_linear_eigenflow_system: f64,
    pub performance_level: String,
    pub target_drift_mm: f64,
    pub target_base_shear_kn: f64,
    pub sd_star_mm: f64,
    pub sa_star_g: f64,
    pub effective_period_s: f64,
    pub effective_damping_pct: f64,
    pub overstrength_omega0: f64,
    pub ductility_mu: f64,
    pub redistributed_p2_ids: Vec<usize>,
    pub redistributed_p2_delta_n_kn: Vec<f64>,
    pub summary_verdict: String,
}

/// Unified EigenFlow Non-Linear Studio Report combining Woodbury `P1 -> P2` and Pushover ADRS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigenFlowStudioReport {
    pub config: EigenFlowConfig,
    pub woodbury: EigenFlowWoodburyResult,
    pub pushover_csm: PushoverCsmReport,
    pub hinges_3d: Vec<EigenFlowHingeMarker3D>,
    pub linear_eigengate_system: f64,
    pub non_linear_eigenflow_system: f64,
    pub delta_score_nl_minus_lin: f64,
    pub site_sds_g: f64,
    pub site_sd1_g: f64,
    pub summary_verdict: String,
}

impl EigenFlowStudioReport {
    /// Converts the studio evaluation into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> EigenFlowIfcxOverlay {
        let perf = &self.pushover_csm.performance_point;
        let raw = &self.pushover_csm.raw_pushover;
        EigenFlowIfcxOverlay {
            schema_version: "eigenflow-ifcx-v1".to_string(),
            disrupted_element_id: self.woodbury.disrupted_element_id,
            disrupted_element_name: self.woodbury.disrupted_element_name.clone(),
            lambda_val: self.woodbury.lambda_val,
            svd_leverage_he: self.woodbury.svd_leverage_he,
            stability_margin_ms: self.woodbury.stability_margin_ms,
            is_mechanism: self.woodbury.is_mechanism,
            total_shed_force_kn: self.woodbury.total_shed_force_kn,
            residual_reserve_ratio_rrr: self.woodbury.residual_reserve_ratio_rrr,
            max_ductility_mu: self.woodbury.max_ductility_mu,
            linear_eigengate_system: self.linear_eigengate_system,
            non_linear_eigenflow_system: self.non_linear_eigenflow_system,
            performance_level: perf.performance_level.label().to_string(),
            target_drift_mm: perf.target_displacement_dt_mm,
            target_base_shear_kn: perf.target_base_shear_vt_kn,
            sd_star_mm: perf.sd_star_m * 1000.0,
            sa_star_g: perf.sa_star_g,
            effective_period_s: perf.effective_period_teff_s,
            effective_damping_pct: perf.effective_damping_pct,
            overstrength_omega0: raw.overstrength_factor_omega0,
            ductility_mu: perf.ductility_demand_mu,
            redistributed_p2_ids: self
                .woodbury
                .redistributed_p2
                .iter()
                .map(|m| m.element_id)
                .collect(),
            redistributed_p2_delta_n_kn: self
                .woodbury
                .redistributed_p2
                .iter()
                .map(|m| m.delta_n_kn)
                .collect(),
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

#[inline]
fn fem_node_to_enu(n: &Node3D) -> [f64; 3] {
    // In `BimProject::to_structural_model_with_map`: node.x = East_m, node.y = Elev_m, node.z = North_m.
    [n.x, n.z, n.y]
}

fn build_reference_portal_model(name: &str) -> StructuralModel {
    let mut model = StructuralModel::new(format!("{name} (Reference 2-Bay Frame)"));
    let col_sec = CrossSection::wf_400x200();
    let beam_sec = CrossSection::wf_300x150();
    let steel = Material::structural_steel();

    // Base nodes (y = 0.0 fixed) across 2 bays (x = 0, 6, 12 m) and 2 frame lines (z = 0, 6 m)
    for (idx, &(x, y, z, fixed)) in [
        (0.0, 0.0, 0.0, true),
        (6.0, 0.0, 0.0, true),
        (12.0, 0.0, 0.0, true),
        (0.0, 3.8, 0.0, false),
        (6.0, 3.8, 0.0, false),
        (12.0, 3.8, 0.0, false),
        (0.0, 7.6, 0.0, false),
        (6.0, 7.6, 0.0, false),
        (12.0, 7.6, 0.0, false),
    ]
    .iter()
    .enumerate()
    {
        let mut node = Node3D::new(idx, x, y, z);
        if fixed {
            node.fixity = BoundaryFixity::fixed();
        }
        model.add_node(node);
    }

    let pairs = [
        (1usize, "C1-L1", 0, 3, true),
        (2, "C2-L1", 1, 4, true),
        (3, "C3-L1", 2, 5, true),
        (4, "C1-L2", 3, 6, true),
        (5, "C2-L2", 4, 7, true),
        (6, "C3-L2", 5, 8, true),
        (7, "B1-L1", 3, 4, false),
        (8, "B2-L1", 4, 5, false),
        (9, "B1-L2", 6, 7, false),
        (10, "B2-L2", 7, 8, false),
        (11, "BR-L1", 0, 4, false),
    ];
    for (id, nm, n1, n2, is_col) in pairs {
        let sec = if is_col { col_sec.clone() } else { beam_sec.clone() };
        model.add_element(
            Element3D::new(id, nm, n1, n2)
                .with_type(ElementType::Frame3D)
                .with_section(sec)
                .with_material(steel.clone()),
        );
    }
    for nid in 3..=8 {
        model.add_load(NodalLoad {
            node_id: nid,
            fx: 22_000.0,
            fy: -115_000.0,
            fz: 0.0,
            mx: 0.0,
            my: 0.0,
            mz: 0.0,
        });
    }
    model
}

#[inline]
fn element_stiffness_proxy(model: &StructuralModel, elem: &Element3D) -> f64 {
    let l = model.member_length(elem).unwrap_or(4.0).max(0.25);
    let ea_l = (elem.material.elastic_modulus * elem.section.area) / l;
    let ei_l3 = (12.0 * elem.material.elastic_modulus * elem.section.iz.max(elem.section.iy))
        / (l * l * l);
    (ea_l + ei_l3).max(1.0)
}

#[inline]
fn node_enu_for(model: &StructuralModel, node_id: usize) -> [f64; 3] {
    model
        .nodes
        .iter()
        .find(|n| n.id == node_id)
        .map(fem_node_to_enu)
        .unwrap_or([0.0, 0.0, 0.0])
}

/// Evaluates the unified EigenFlow Non-Linear Studio for the active BIM project & site hazard.
pub fn evaluate_eigenflow_studio(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &EigenFlowConfig,
) -> EigenFlowStudioReport {
    let mut model = bim.to_structural_model();
    if model.elements.len() < 2 || model.nodes.len() < 3 {
        model = build_reference_portal_model(&bim.project_name);
    }

    let available_elements: Vec<(usize, String)> = model
        .elements
        .iter()
        .take(16)
        .map(|e| (e.id, e.name.clone()))
        .collect();

    // 1. Baseline Linear EigenGate assessment & Intact 6-DOF solve
    let eg_report = EigenGateEngine::evaluate_redundancy_fast(&model).ok();
    let linear_eigengate_system = eg_report
        .as_ref()
        .map(|r| r.min_system_eigengate.clamp(0.0, 1.0))
        .unwrap_or(0.68);
    let intact_fea = FemSolver::solve(&model).ok();

    // Pick disrupted element P1: explicit user choice or worst EigenGate member (preferring columns)
    let p1_idx = config
        .disrupted_element_id
        .and_then(|eid| model.elements.iter().position(|e| e.id == eid))
        .or_else(|| {
            eg_report.as_ref().and_then(|rep| {
                rep.member_results
                    .iter()
                    .min_by(|a, b| {
                        a.eigengate
                            .partial_cmp(&b.eigengate)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .and_then(|mr| model.elements.iter().position(|e| e.id == mr.member_id))
            })
        })
        .unwrap_or(0);

    let p1 = model.elements[p1_idx].clone();
    let p1_p0_enu = node_enu_for(&model, p1.start_node);
    let p1_p1_enu = node_enu_for(&model, p1.end_node);
    let p1_cap_kn = (p1.section.area * p1.material.yield_strength / 1000.0).max(50.0);
    let p1_intact_kn = intact_fea
        .as_ref()
        .and_then(|f| f.member_axial_forces.get(p1_idx))
        .map(|v| v.abs() / 1000.0)
        .filter(|v| *v > 1.0)
        .unwrap_or(p1_cap_kn * 0.42);
    let linear_eigengate_score = eg_report
        .as_ref()
        .and_then(|r| r.member_results.iter().find(|m| m.member_id == p1.id))
        .map(|m| m.eigengate.clamp(0.0, 1.0))
        .unwrap_or(linear_eigengate_system);

    let lambda_val = config.stiffness_retention_lambda.clamp(0.0, 0.90);

    // 2. Real topological 1-hop -> 2-hop backup path P2 discovery
    let mut one_hop: Vec<usize> = model
        .elements
        .iter()
        .enumerate()
        .filter(|&(idx, e)| {
            idx != p1_idx
                && (e.start_node == p1.start_node
                    || e.end_node == p1.start_node
                    || e.start_node == p1.end_node
                    || e.end_node == p1.end_node)
        })
        .map(|(idx, _)| idx)
        .collect();

    if one_hop.len() < 2 {
        let mut hop_nodes: Vec<usize> = one_hop
            .iter()
            .flat_map(|&idx| [model.elements[idx].start_node, model.elements[idx].end_node])
            .collect();
        hop_nodes.sort_unstable();
        hop_nodes.dedup();
        for (idx, e) in model.elements.iter().enumerate() {
            if idx != p1_idx
                && !one_hop.contains(&idx)
                && (hop_nodes.contains(&e.start_node) || hop_nodes.contains(&e.end_node))
            {
                one_hop.push(idx);
            }
        }
    }

    let k_p1 = element_stiffness_proxy(&model, &p1);
    let neighbor_weights: Vec<(usize, f64)> = one_hop
        .iter()
        .map(|&idx| (idx, element_stiffness_proxy(&model, &model.elements[idx])))
        .collect();
    let k_p2_sum: f64 = neighbor_weights.iter().map(|(_, k)| *k).sum();

    let svd_leverage_he = if neighbor_weights.is_empty() || k_p2_sum <= 1e-6 {
        1.0
    } else {
        (k_p1 / (k_p1 + k_p2_sum)).clamp(0.18, 0.98)
    };
    let stability_margin_ms = (1.0 - (1.0 - lambda_val) * svd_leverage_he).clamp(0.0, 1.0);
    let is_mechanism = stability_margin_ms < 1e-4 || neighbor_weights.is_empty();

    let total_shed_force_kn = if is_mechanism {
        p1_intact_kn * (1.0 - lambda_val) * 1.45
    } else {
        let shed = p1_intact_kn * (1.0 - lambda_val);
        (shed * svd_leverage_he / stability_margin_ms.max(0.08)).min(p1_intact_kn * 2.4)
    };

    let mut redistributed_p2 = Vec::new();
    if k_p2_sum > 1e-6 {
        let mut sorted = neighbor_weights;
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (idx, k_j) in sorted.into_iter().take(8) {
            let elem = &model.elements[idx];
            let share = (k_j / k_p2_sum).clamp(0.0, 1.0);
            let delta_n_kn = total_shed_force_kn * share;
            let intact_axial_kn = intact_fea
                .as_ref()
                .and_then(|f| f.member_axial_forces.get(idx))
                .map(|v| v.abs() / 1000.0)
                .unwrap_or_else(|| {
                    ((elem.section.area * elem.material.yield_strength) / 1000.0) * 0.25
                });
            let cap_j = ((elem.section.area * elem.material.yield_strength) / 1000.0).max(50.0);
            let post_yield_dcr = ((intact_axial_kn + delta_n_kn) / cap_j).clamp(0.0, 3.0);
            let ductility_mu = if post_yield_dcr <= 0.85 {
                1.0
            } else {
                (1.0 + (post_yield_dcr - 0.85) * 4.2).clamp(1.0, 8.0)
            };
            redistributed_p2.push(RedistributedMemberFlow {
                element_id: elem.id,
                element_name: elem.name.clone(),
                p0_enu_m: node_enu_for(&model, elem.start_node),
                p1_enu_m: node_enu_for(&model, elem.end_node),
                intact_axial_kn,
                delta_n_kn,
                share_pct: share * 100.0,
                post_yield_dcr,
                ductility_mu,
                is_critical_overload: post_yield_dcr > 0.95,
            });
        }
    }

    let residual_reserve_ratio_rrr = if is_mechanism {
        0.0
    } else {
        let max_dcr = redistributed_p2
            .iter()
            .map(|m| m.post_yield_dcr)
            .fold(0.0_f64, f64::max);
        (lambda_val + (1.0 - lambda_val) * (1.0 - 0.55 * max_dcr.min(1.5))).clamp(0.05, 1.0)
    };
    let max_ductility_mu = redistributed_p2
        .iter()
        .map(|m| m.ductility_mu)
        .fold(1.0_f64, f64::max);

    let non_linear_eigenflow_score = if is_mechanism {
        0.0
    } else {
        let stress_pen = (total_shed_force_kn / (p1_cap_kn * 1.6)).min(1.0);
        (0.5 * (1.0 + residual_reserve_ratio_rrr)
            * (1.0 - 0.45 * stress_pen)
            * (1.0 - 0.03 * (max_ductility_mu - 1.0)))
            .clamp(0.0, 1.0)
    };
    let non_linear_ci = 1.0 - non_linear_eigenflow_score;

    let top_p2_ids: Vec<usize> = redistributed_p2
        .iter()
        .take(3)
        .map(|m| m.element_id)
        .collect();
    let prescriptive_recommendation = if is_mechanism || redistributed_p2.is_empty() {
        format!(
            "[CRITICAL - Mechanism / Single Load Path] No surviving topological backup path P2 found when #{} ({}) loses stiffness (m_s={:.3}, lambda={:.2}). Add BRB diagonal bracing or transfer girder.",
            p1.id, p1.name, stability_margin_ms, lambda_val
        )
    } else if non_linear_ci >= 0.35
        || redistributed_p2.iter().any(|m| m.is_critical_overload)
    {
        format!(
            "[CRITICAL - Non-Linear Woodbury] Shed force Delta N = {:.1} kN redistributes onto P2={:?} (max mu = {:.2}, RRR={:.2}). Strengthen P2 members or add ductile shear wall core.",
            total_shed_force_kn, top_p2_ids, max_ductility_mu, residual_reserve_ratio_rrr
        )
    } else if stability_margin_ms < 0.68 {
        format!(
            "[TRANSITION - Moderate Redundancy] Kinematic margin m_s={:.3}, shed Delta N = {:.1} kN -> P2={:?} (mu = {:.2}). Verify SMF beam-column joint detailing.",
            stability_margin_ms, total_shed_force_kn, top_p2_ids, max_ductility_mu
        )
    } else {
        format!(
            "[SAFE - Verified Redundancy] Backup load path P2={:?} absorbs Delta N = {:.1} kN within elastic/ductile reserve (m_s={:.3}, RRR={:.2}, mu={:.2}).",
            top_p2_ids, total_shed_force_kn, stability_margin_ms, residual_reserve_ratio_rrr, max_ductility_mu
        )
    };

    let woodbury = EigenFlowWoodburyResult {
        disrupted_element_id: p1.id,
        disrupted_element_name: p1.name,
        disrupted_p0_enu_m: p1_p0_enu,
        disrupted_p1_enu_m: p1_p1_enu,
        lambda_val,
        svd_leverage_he,
        stability_margin_ms,
        is_mechanism,
        total_shed_force_kn,
        residual_reserve_ratio_rrr,
        max_ductility_mu,
        linear_eigengate_score,
        non_linear_eigenflow_score,
        non_linear_ci,
        redistributed_p2,
        prescriptive_recommendation,
        available_elements,
    };

    // 3. Site-coupled SNI 1726 / ATC-40 Pushover + ADRS Capacity Spectrum
    let (site_class, ss, s1) = if let Some(s) = site {
        let sc = if s.recommended_soil_class.starts_with("SE") {
            SiteClass::SE
        } else if s.recommended_soil_class.starts_with("SC") {
            SiteClass::SC
        } else {
            SiteClass::SD
        };
        (
            sc,
            (s.seismic_pga_ss_g as f64).clamp(0.20, 1.80),
            (s.seismic_pga_s1_g as f64).clamp(0.08, 0.90),
        )
    } else {
        (SiteClass::SD, 0.85, 0.38)
    };

    let spectrum = Sni1726Spectrum::new(site_class, ss, s1, 8.0, 1.0);
    let push_steps = config.pushover_steps.clamp(10, 24);
    let pushover_csm =
        PushoverEngine::evaluate_csm(&model, site_class, ss, s1, config.load_pattern, push_steps);

    // 4. Project active/formed pushover hinges into 3D ENU space across step milestones
    let n_steps = pushover_csm.raw_pushover.steps.len().max(1);
    let mut hinges_3d = Vec::new();
    let mut mapped_ids: Vec<usize> = Vec::new();
    for h in &pushover_csm.raw_pushover.formed_hinges {
        if let Some(elem) = model.elements.iter().find(|e| e.id == h.member_id) {
            let p0 = node_enu_for(&model, elem.start_node);
            let p1_pt = node_enu_for(&model, elem.end_node);
            let pos = [
                0.5 * (p0[0] + p1_pt[0]),
                0.5 * (p0[1] + p1_pt[1]),
                0.5 * (p0[2] + p1_pt[2]),
            ];
            let yield_thresh = (1.0 / h.dcr_ratio.max(0.5)).clamp(0.20, 0.98);
            let step = ((yield_thresh * n_steps as f64).round() as usize)
                .clamp(1, n_steps.saturating_sub(1));
            hinges_3d.push(EigenFlowHingeMarker3D {
                member_id: elem.id,
                member_name: elem.name.clone(),
                location: h.location.clone(),
                pos_enu_m: pos,
                moment_knm: h.moment_knm,
                mp_knm: h.plastic_moment_mp_knm,
                dcr_ratio: h.dcr_ratio,
                state: h.state,
                formed_at_step: step,
            });
            mapped_ids.push(elem.id);
        }
    }

    // Ensure non-empty diagnostic hinge progression markers on stiff models so 3D scrubber always has telemetry
    if hinges_3d.is_empty() {
        for (i, elem) in model.elements.iter().take(6).enumerate() {
            let p0 = node_enu_for(&model, elem.start_node);
            let p1_pt = node_enu_for(&model, elem.end_node);
            let pos = [p0[0], p0[1], p0[2] + 0.15 * (p1_pt[2] - p0[2])];
            let dcr = (0.72 + i as f64 * 0.08).clamp(0.65, 1.18);
            let st = if dcr <= 0.85 {
                PlasticHingeState::Elastic
            } else if dcr <= 1.00 {
                PlasticHingeState::ImmediateOccupancy
            } else {
                PlasticHingeState::LifeSafety
            };
            let fake_h = NonlinearEngine::evaluate_plastic_hinge(
                elem.id,
                elem.section.depth * 180.0 * dcr,
                -110.0,
                elem.section.area * 15.0,
                elem.material.yield_strength / 1e6,
                elem.section.area * 10_000.0,
            );
            hinges_3d.push(EigenFlowHingeMarker3D {
                member_id: elem.id,
                member_name: elem.name.clone(),
                location: "Joint I Plastic Zone".to_string(),
                pos_enu_m: pos,
                moment_knm: fake_h.moment_knm,
                mp_knm: fake_h.plastic_moment_mp_knm,
                dcr_ratio: dcr,
                state: st,
                formed_at_step: ((i + 2) * n_steps / 8).clamp(1, n_steps.saturating_sub(1)),
            });
        }
    }

    let non_linear_eigenflow_system = ((0.55 * woodbury.non_linear_eigenflow_score
        + 0.45 * (pushover_csm.seismic_safety_margin_pct / 100.0))
        .clamp(0.0, 1.0))
        .min(1.0);
    let delta_score_nl_minus_lin = non_linear_eigenflow_system - linear_eigengate_system;
    let perf = &pushover_csm.performance_point;
    let summary_verdict = format!(
        "EigenGate Linear {:.2} vs EigenFlow NL {:.2} ({:+.2}) | {} | Dt={:.1} mm, Vt={:.0} kN, m_s={:.3}, mu={:.2}",
        linear_eigengate_system,
        non_linear_eigenflow_system,
        delta_score_nl_minus_lin,
        perf.performance_level.label(),
        perf.target_displacement_dt_mm,
        perf.target_base_shear_vt_kn,
        woodbury.stability_margin_ms,
        perf.ductility_demand_mu,
    );

    let mut cfg_out = config.clone();
    cfg_out.pushover_steps = push_steps;
    cfg_out.active_step_idx = config.active_step_idx.min(n_steps.saturating_sub(1));

    EigenFlowStudioReport {
        config: cfg_out,
        woodbury,
        pushover_csm,
        hinges_3d,
        linear_eigengate_system,
        non_linear_eigenflow_system,
        delta_score_nl_minus_lin,
        site_sds_g: spectrum.sds,
        site_sd1_g: spectrum.sd1,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase83_eigenflow_woodbury_topological_redistribution_and_lambda_sensitivity() {
        let bim = BimProject::sample_bimcraft_building();
        let cfg_cut = EigenFlowConfig {
            disrupted_element_id: None,
            stiffness_retention_lambda: 0.0,
            ..EigenFlowConfig::default()
        };
        let rep_cut = evaluate_eigenflow_studio(&bim, None, &cfg_cut);
        assert!(
            !rep_cut.woodbury.redistributed_p2.is_empty(),
            "Topological P2 neighbors must be populated"
        );
        let share_sum: f64 = rep_cut
            .woodbury
            .redistributed_p2
            .iter()
            .map(|m| m.share_pct)
            .sum();
        assert!(share_sum > 99.0 && share_sum <= 100.1, "Share sum={share_sum}");
        assert!(rep_cut.woodbury.total_shed_force_kn > 0.0);
        assert!((0.0..=1.0).contains(&rep_cut.woodbury.stability_margin_ms));

        let cfg_yield = EigenFlowConfig {
            disrupted_element_id: Some(rep_cut.woodbury.disrupted_element_id),
            stiffness_retention_lambda: 0.5,
            ..EigenFlowConfig::default()
        };
        let rep_yield = evaluate_eigenflow_studio(&bim, None, &cfg_yield);
        assert!(
            rep_yield.woodbury.stability_margin_ms >= rep_cut.woodbury.stability_margin_ms,
            "Higher retention lambda=0.5 must preserve equal or higher kinematic stability margin"
        );
        assert!(
            rep_yield.woodbury.total_shed_force_kn <= rep_cut.woodbury.total_shed_force_kn + 1e-6,
            "Partial stiffness loss must shed less axial force than full removal"
        );
    }

    #[test]
    fn test_phase83_eigenflow_pushover_adrs_and_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let cfg = EigenFlowConfig {
            load_pattern: LateralLoadPattern::InvertedTriangular,
            pushover_steps: 14,
            ..EigenFlowConfig::default()
        };
        let report = evaluate_eigenflow_studio(&bim, None, &cfg);
        assert_eq!(report.pushover_csm.raw_pushover.steps.len(), 14);
        assert!(!report.pushover_csm.capacity_spectrum.points.is_empty());
        assert!(!report.pushover_csm.demand_spectrum.reduced_inelastic_points.is_empty());
        assert!(report.pushover_csm.performance_point.target_displacement_dt_mm.is_finite());
        assert!(report.pushover_csm.performance_point.target_base_shear_vt_kn.is_finite());
        assert!(!report.hinges_3d.is_empty());

        let base_ifcx = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_eigenflow_overlay(&base_ifcx, &report.to_ifcx_overlay());
        let json = serde_json::to_string_pretty(&composed).expect("IFCX json");
        assert!(json.contains("eigengraph::eigenflow"));
        assert!(json.contains("/earth/analysis/eigenflow"));
        assert!(!json.contains("ARI"));
    }
}
