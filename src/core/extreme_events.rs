//! core/extreme_events.rs — Phase 84 (`v0.8.4`): Extreme Events Studio Engine.
//!
//! Couples physical 3D BIM geometry with:
//! - [`BlastEngine`] (`UFC 3-340-02` / `ASCE 59-11` / Kingery–Bulmash shockwave & SDOF response)
//! - [`FireEngine`] (`ISO 834` / `ASTM E119` / Eurocode `HC` thermal compartment curves & stiffness retention)
//! - Closed-loop **EigenFlow** Woodbury (`P1 -> P2`) progressive collapse screening (`UFC 4-023-03`)
//! - Non-destructive `eigengraph::hazard::extreme_events` `.ifcx` overlay emission.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::blast::{
    BlastAnalysisInput, BlastAnalysisReport, BlastComponentKind, BlastEngine, BurstType,
    ExplosiveType, SupportCondition, UfcDamageLevel,
};
use crate::core::eigenflow::{evaluate_eigenflow_studio, EigenFlowConfig};
use crate::core::fire::{
    ConcreteAggregateKind, FireAnalysisInput, FireAnalysisReport, FireCurveKind, FireEngine,
    FireMemberKind, FireProtectionKind, ThermalRestraintKind,
};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::model::{
    BoundaryFixity, CrossSection, Element3D, ElementType, Material, NodalLoad, Node3D,
    StructuralModel,
};

/// Canonical EigenGraph IFCX attribute namespace for Phase 84 Extreme Events overlays.
pub const NS_EG_EXTREME_EVENTS: &str = "eigengraph::hazard::extreme_events";

/// Active visualization / evaluation mode for the Extreme Events Studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtremeEventMode {
    Blast,
    StructuralFire,
    Combined,
}

impl Default for ExtremeEventMode {
    fn default() -> Self {
        Self::Blast
    }
}

impl ExtremeEventMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Blast => "Blast (UFC 3-340-02)",
            Self::StructuralFire => "Structural Fire (ISO 834 / EC)",
            Self::Combined => "Combined Blast + Fire",
        }
    }
}

/// Persisted configuration state for Extreme Events (Blast & Structural Fire).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtremeEventsConfig {
    pub mode: ExtremeEventMode,
    pub explosive_type: ExplosiveType,
    pub burst_type: BurstType,
    pub explosive_mass_kg: f64,
    pub standoff_distance_m: f64,
    pub charge_azimuth_deg: f64,
    pub charge_height_m: f64,
    pub fire_curve: FireCurveKind,
    pub protection_kind: FireProtectionKind,
    pub protection_thickness_mm: f64,
    pub fire_story_idx: usize,
    pub exposure_minutes: f64,
    pub target_frr_minutes: f64,
}

impl Default for ExtremeEventsConfig {
    fn default() -> Self {
        Self {
            mode: ExtremeEventMode::Blast,
            explosive_type: ExplosiveType::C4,
            burst_type: BurstType::SurfaceBurst,
            explosive_mass_kg: 250.0,
            standoff_distance_m: 12.0,
            charge_azimuth_deg: 225.0,
            charge_height_m: 1.2,
            fire_curve: FireCurveKind::Iso834Standard,
            protection_kind: FireProtectionKind::SprayedMineralFiber,
            protection_thickness_mm: 20.0,
            fire_story_idx: 0,
            exposure_minutes: 90.0,
            target_frr_minutes: 90.0,
        }
    }
}

/// Per-member 3D spatial blast impact audit derived from Kingery–Bulmash scaling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlastMemberImpact3D {
    pub element_id: usize,
    pub element_name: String,
    pub p0_enu_m: [f64; 3],
    pub p1_enu_m: [f64; 3],
    pub distance_m: f64,
    pub reflected_overpressure_kpa: f64,
    pub reflected_impulse_kpa_ms: f64,
    pub ductility_ratio: f64,
    pub damage_level: UfcDamageLevel,
    pub residual_stiffness_lambda: f64,
}

/// 3D Fire Compartment volume anchored onto an active BIM building story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FireCompartmentZone3D {
    pub story_idx: usize,
    pub story_label: String,
    pub center_x_m: f64,
    pub center_y_m: f64,
    pub z_bottom_m: f64,
    pub z_top_m: f64,
    pub width_m: f64,
    pub depth_m: f64,
    pub gas_temp_c: f64,
    pub member_temp_c: f64,
    pub ky_factor: f64,
    pub ke_factor: f64,
    pub thermal_dcr: f64,
    pub frr_minutes: f64,
    pub is_target_frr_met: bool,
}

/// Closed-loop Branch 2 EigenFlow Woodbury assessment triggered by extreme hazard damage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtremeEventEigenFlowCoupling {
    pub trigger_label: String,
    pub critical_p1_element_id: usize,
    pub critical_p1_element_name: String,
    pub derived_lambda: f64,
    pub woodbury_ms: f64,
    pub woodbury_rrr: f64,
    pub woodbury_mu: f64,
    pub total_shed_force_kn: f64,
    pub eigenflow_score: f64,
    pub is_progressive_collapse_risk: bool,
    pub recommendation: String,
}

/// Compact serializable IFCX overlay payload under `eigengraph::hazard::extreme_events`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtremeEventsIfcxOverlay {
    pub schema_version: String,
    pub mode: ExtremeEventMode,
    pub explosive_type: String,
    pub explosive_mass_kg: f64,
    pub standoff_distance_m: f64,
    pub scaled_distance_z: f64,
    pub peak_reflected_pressure_kpa: f64,
    pub reflected_impulse_kpa_ms: f64,
    pub worst_blast_damage: String,
    pub blast_p1_element_id: usize,
    pub blast_residual_lambda: f64,
    pub blast_woodbury_ms: f64,
    pub fire_curve: String,
    pub fire_story_idx: usize,
    pub fire_exposure_minutes: f64,
    pub fire_gas_temp_c: f64,
    pub fire_member_temp_c: f64,
    pub fire_ke_retention: f64,
    pub fire_frr_minutes: f64,
    pub fire_woodbury_ms: f64,
    pub summary_verdict: String,
}

/// Unified Extreme Events Studio report combining 3D Blast, Structural Fire, and EigenFlow coupling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtremeEventsReport {
    pub config: ExtremeEventsConfig,
    pub charge_pos_enu_m: [f64; 3],
    pub target_face_enu_m: [f64; 3],
    pub blast_report: BlastAnalysisReport,
    pub blast_member_impacts: Vec<BlastMemberImpact3D>,
    pub worst_blast_damage: UfcDamageLevel,
    pub fire_report: FireAnalysisReport,
    pub fire_compartment: FireCompartmentZone3D,
    /// Thermal exposed BIM members `(p0_enu_m, p1_enu_m, severity 0..1)`.
    pub exposed_fire_members: Vec<([f64; 3], [f64; 3], f64)>,
    pub blast_eigenflow: ExtremeEventEigenFlowCoupling,
    pub fire_eigenflow: ExtremeEventEigenFlowCoupling,
    pub summary_verdict: String,
}

impl ExtremeEventsReport {
    /// Emits a non-destructive IFCX overlay payload for `eigengraph::hazard::extreme_events`.
    pub fn to_ifcx_overlay(&self) -> ExtremeEventsIfcxOverlay {
        ExtremeEventsIfcxOverlay {
            schema_version: "extreme-events-ifcx-v1".to_string(),
            mode: self.config.mode,
            explosive_type: self.config.explosive_type.label().to_string(),
            explosive_mass_kg: self.config.explosive_mass_kg,
            standoff_distance_m: self.config.standoff_distance_m,
            scaled_distance_z: self.blast_report.blast_wave.scaled_distance_z,
            peak_reflected_pressure_kpa: self.blast_report.blast_wave.peak_reflected_overpressure_kpa,
            reflected_impulse_kpa_ms: self.blast_report.blast_wave.reflected_impulse_kpa_ms,
            worst_blast_damage: self.worst_blast_damage.label().to_string(),
            blast_p1_element_id: self.blast_eigenflow.critical_p1_element_id,
            blast_residual_lambda: self.blast_eigenflow.derived_lambda,
            blast_woodbury_ms: self.blast_eigenflow.woodbury_ms,
            fire_curve: self.config.fire_curve.label().to_string(),
            fire_story_idx: self.fire_compartment.story_idx,
            fire_exposure_minutes: self.config.exposure_minutes,
            fire_gas_temp_c: self.fire_compartment.gas_temp_c,
            fire_member_temp_c: self.fire_compartment.member_temp_c,
            fire_ke_retention: self.fire_compartment.ke_factor,
            fire_frr_minutes: self.fire_compartment.frr_minutes,
            fire_woodbury_ms: self.fire_eigenflow.woodbury_ms,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

#[inline]
fn fem_node_to_enu(n: &Node3D) -> [f64; 3] {
    [n.x, n.z, n.y]
}

fn build_fallback_portal(name: &str) -> StructuralModel {
    let mut model = StructuralModel::new(format!("{name} (Reference Frame)"));
    let col = CrossSection::wf_400x200();
    let beam = CrossSection::wf_300x150();
    let steel = Material::structural_steel();
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
    for (id, nm, a, b, is_col) in [
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
    ] {
        model.add_element(
            Element3D::new(id, nm, a, b)
                .with_type(ElementType::Frame3D)
                .with_section(if is_col { col.clone() } else { beam.clone() })
                .with_material(steel.clone()),
        );
    }
    for nid in 3..=8 {
        model.add_load(NodalLoad {
            node_id: nid,
            fx: 20_000.0,
            fy: -115_000.0,
            fz: 0.0,
            mx: 0.0,
            my: 0.0,
            mz: 0.0,
        });
    }
    model
}

fn damage_to_residual_lambda(dmg: UfcDamageLevel) -> f64 {
    match dmg {
        UfcDamageLevel::Superficial => 0.90,
        UfcDamageLevel::Low => 0.75,
        UfcDamageLevel::Medium => 0.45,
        UfcDamageLevel::High => 0.20,
        UfcDamageLevel::Hazardous => 0.0,
    }
}

/// Evaluates Extreme Events (Blast `UFC 3-340-02` & Structural Fire `ISO 834/EC`) coupled to live BIM + EigenFlow.
pub fn evaluate_extreme_events(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &ExtremeEventsConfig,
) -> ExtremeEventsReport {
    let mut cfg = config.clone();
    cfg.explosive_mass_kg = cfg.explosive_mass_kg.clamp(10.0, 5_000.0);
    cfg.standoff_distance_m = cfg.standoff_distance_m.clamp(3.0, 120.0);
    cfg.protection_thickness_mm = cfg.protection_thickness_mm.clamp(0.0, 80.0);
    cfg.exposure_minutes = cfg.exposure_minutes.clamp(0.0, 180.0);
    cfg.target_frr_minutes = cfg.target_frr_minutes.clamp(30.0, 240.0);

    let mut model = bim.to_structural_model();
    if model.elements.len() < 2 || model.nodes.len() < 3 {
        model = build_fallback_portal(&bim.project_name);
    }

    let node_enu = |id: usize| -> [f64; 3] {
        model
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(fem_node_to_enu)
            .unwrap_or([0.0, 0.0, 0.0])
    };

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut max_z = 3.8_f64;
    for n in &model.nodes {
        let p = fem_node_to_enu(n);
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_y = min_y.min(p[1]);
        max_y = max_y.max(p[1]);
        max_z = max_z.max(p[2]);
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        min_x = 0.0;
        max_x = 12.0;
        min_y = 0.0;
        max_y = 8.0;
    }
    let cx = 0.5 * (min_x + max_x);
    let cy = 0.5 * (min_y + max_y);
    let span_x = (max_x - min_x).max(6.0);
    let span_y = (max_y - min_y).max(6.0);

    // 1. Charge position along azimuth outside the perimeter face
    let az_rad = cfg.charge_azimuth_deg.to_radians();
    let (ux, uy) = (az_rad.cos(), az_rad.sin());
    let half_proj = 0.5 * (span_x * ux.abs() + span_y * uy.abs());
    let target_face_enu_m = [cx + ux * half_proj, cy + uy * half_proj, cfg.charge_height_m];
    let charge_pos_enu_m = [
        cx + ux * (half_proj + cfg.standoff_distance_m),
        cy + uy * (half_proj + cfg.standoff_distance_m),
        cfg.charge_height_m,
    ];

    // 2. Baseline UFC 3-340-02 Blast Assessment
    let blast_input = BlastAnalysisInput {
        project_name: bim.project_name.clone(),
        explosive_mass_kg: cfg.explosive_mass_kg,
        explosive_type: cfg.explosive_type,
        burst_type: cfg.burst_type,
        standoff_distance_m: cfg.standoff_distance_m,
        component_kind: BlastComponentKind::RcWall,
        support_condition: SupportCondition::FixedFixed,
        span_length_m: 4.0,
        tributary_width_m: 1.5,
        thickness_m: 0.40,
        f_c_mpa: 35.0,
        f_y_mpa: 420.0,
        rebar_ratio_percent: 1.2,
        damping_ratio: 0.02,
        target_damage_level: UfcDamageLevel::Low,
    };
    let blast_report = BlastEngine::run_assessment(&blast_input);
    let base_pr = blast_report.blast_wave.peak_reflected_overpressure_kpa.max(1.0);
    let base_mu = blast_report.dynamic_ductility_ratio;
    let base_rot = blast_report.support_rotation_deg;

    // 3. Per-member Kingery-Bulmash distance triage (up to 12 closest members)
    let mut impacts: Vec<BlastMemberImpact3D> = model
        .elements
        .iter()
        .map(|elem| {
            let p0 = node_enu(elem.start_node);
            let p1 = node_enu(elem.end_node);
            let mid = [
                0.5 * (p0[0] + p1[0]),
                0.5 * (p0[1] + p1[1]),
                0.5 * (p0[2] + p1[2]),
            ];
            let dist = ((mid[0] - charge_pos_enu_m[0]).powi(2)
                + (mid[1] - charge_pos_enu_m[1]).powi(2)
                + (mid[2] - charge_pos_enu_m[2]).powi(2))
            .sqrt()
            .max(1.5);
            let wave = BlastEngine::evaluate_blast_wave(
                cfg.explosive_mass_kg,
                cfg.explosive_type,
                cfg.burst_type,
                dist,
                3.8,
                1.5,
            );
            let ratio = (wave.peak_reflected_overpressure_kpa / base_pr).clamp(0.05, 8.0);
            let mu_local = (base_mu * ratio).clamp(0.2, 25.0);
            let rot_local = (base_rot * ratio).clamp(0.05, 14.0);
            let kind = if elem.name.to_ascii_lowercase().contains("col") {
                BlastComponentKind::SteelColumn
            } else {
                BlastComponentKind::RcWall
            };
            let dmg = BlastEngine::classify_damage_level(mu_local, rot_local, kind);
            let lam = damage_to_residual_lambda(dmg);
            BlastMemberImpact3D {
                element_id: elem.id,
                element_name: elem.name.clone(),
                p0_enu_m: p0,
                p1_enu_m: p1,
                distance_m: dist,
                reflected_overpressure_kpa: wave.peak_reflected_overpressure_kpa,
                reflected_impulse_kpa_ms: wave.reflected_impulse_kpa_ms,
                ductility_ratio: mu_local,
                damage_level: dmg,
                residual_stiffness_lambda: lam,
            }
        })
        .collect();
    impacts.sort_by(|a, b| {
        a.distance_m
            .partial_cmp(&b.distance_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    impacts.truncate(12);

    let worst_blast_idx = impacts
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.damage_level
                .cmp(&b.damage_level)
                .then_with(|| {
                    a.reflected_overpressure_kpa
                        .partial_cmp(&b.reflected_overpressure_kpa)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    let (blast_p1_id, blast_p1_name, blast_lam, worst_blast_damage) = impacts
        .get(worst_blast_idx)
        .map(|m| (m.element_id, m.element_name.clone(), m.residual_stiffness_lambda, m.damage_level))
        .unwrap_or((1, "P1-Col".to_string(), 0.45, blast_report.evaluated_damage_level));

    let ef_blast = evaluate_eigenflow_studio(
        bim,
        site,
        &EigenFlowConfig {
            disrupted_element_id: Some(blast_p1_id),
            stiffness_retention_lambda: blast_lam,
            ..EigenFlowConfig::default()
        },
    );
    let blast_eigenflow = ExtremeEventEigenFlowCoupling {
        trigger_label: format!(
            "Blast {:.0}kg {} @ {:.1}m",
            cfg.explosive_mass_kg,
            cfg.explosive_type.label(),
            cfg.standoff_distance_m
        ),
        critical_p1_element_id: blast_p1_id,
        critical_p1_element_name: blast_p1_name,
        derived_lambda: blast_lam,
        woodbury_ms: ef_blast.woodbury.stability_margin_ms,
        woodbury_rrr: ef_blast.woodbury.residual_reserve_ratio_rrr,
        woodbury_mu: ef_blast.woodbury.max_ductility_mu,
        total_shed_force_kn: ef_blast.woodbury.total_shed_force_kn,
        eigenflow_score: ef_blast.non_linear_eigenflow_system,
        is_progressive_collapse_risk: ef_blast.woodbury.is_mechanism
            || ef_blast.woodbury.non_linear_ci >= 0.40,
        recommendation: ef_blast.woodbury.prescriptive_recommendation.clone(),
    };

    // 4. Structural Fire Evaluation (ISO 834 / ASTM E119 / Eurocode HC)
    let mut story_elevs_m: Vec<f64> = bim.levels.iter().map(|l| l.elevation_mm * 0.001).collect();
    story_elevs_m.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    story_elevs_m.dedup_by(|a, b| (*a - *b).abs() < 0.25);
    if story_elevs_m.len() < 2 {
        story_elevs_m = vec![0.0, 3.8, max_z.max(7.6)];
    }
    let max_story_idx = story_elevs_m.len().saturating_sub(2);
    cfg.fire_story_idx = cfg.fire_story_idx.min(max_story_idx);
    let z_bot = story_elevs_m[cfg.fire_story_idx];
    let z_top = story_elevs_m[cfg.fire_story_idx + 1].max(z_bot + 2.8);

    let fire_input = FireAnalysisInput {
        project_name: bim.project_name.clone(),
        fire_curve: cfg.fire_curve,
        member_kind: FireMemberKind::SteelColumn,
        protection_kind: cfg.protection_kind,
        protection_thickness_mm: cfg.protection_thickness_mm,
        fire_duration_minutes: 180.0,
        target_frr_minutes: cfg.target_frr_minutes,
        clear_span_m: (z_top - z_bot).clamp(2.8, 8.0),
        section_depth_mm: 400.0,
        section_width_mm: 400.0,
        web_thickness_mm: 12.0,
        flange_thickness_mm: 18.0,
        concrete_cover_mm: 40.0,
        aggregate_kind: ConcreteAggregateKind::Siliceous,
        steel_fy_mpa: 355.0,
        concrete_fc_mpa: 35.0,
        service_moment_demand_knm: 180.0,
        service_axial_demand_kn: 720.0,
        thermal_restraint: ThermalRestraintKind::ModeratelyRestrained,
    };
    let fire_report = FireEngine::run_assessment(&fire_input);
    let sample_step = fire_report
        .time_history_samples
        .iter()
        .min_by(|a, b| {
            (a.time_minutes - cfg.exposure_minutes)
                .abs()
                .partial_cmp(&(b.time_minutes - cfg.exposure_minutes).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    let (gas_temp_c, member_temp_c, ky_factor, ke_factor, thermal_dcr) = if let Some(st) = sample_step {
        (
            st.gas_temp_c,
            st.member_temp_c,
            st.ky_factor.clamp(0.0, 1.0),
            st.ke_factor.clamp(0.0, 1.0),
            st.utilization_ratio.max(0.0),
        )
    } else {
        (842.0, 510.0, 0.74, 0.58, 0.88)
    };

    let fire_compartment = FireCompartmentZone3D {
        story_idx: cfg.fire_story_idx,
        story_label: format!("L{} Compartment", cfg.fire_story_idx + 1),
        center_x_m: cx,
        center_y_m: cy,
        z_bottom_m: z_bot,
        z_top_m: z_top,
        width_m: span_x,
        depth_m: span_y,
        gas_temp_c,
        member_temp_c,
        ky_factor,
        ke_factor,
        thermal_dcr,
        frr_minutes: fire_report.fire_endurance_rating_minutes,
        is_target_frr_met: fire_report.fire_endurance_rating_minutes >= cfg.exposure_minutes,
    };

    let thermal_sev = (1.0 - ke_factor).clamp(0.05, 1.0);
    let mut exposed_fire_members = Vec::new();
    let mut fire_p1_elem: Option<(usize, String)> = None;
    for elem in &model.elements {
        let p0 = node_enu(elem.start_node);
        let p1 = node_enu(elem.end_node);
        let min_ez = p0[2].min(p1[2]);
        let max_ez = p0[2].max(p1[2]);
        if max_ez >= z_bot - 0.25 && min_ez <= z_top + 0.25 {
            if fire_p1_elem.is_none() {
                fire_p1_elem = Some((elem.id, elem.name.clone()));
            }
            exposed_fire_members.push((p0, p1, thermal_sev));
        }
    }
    let (fire_p1_id, fire_p1_name) = fire_p1_elem.unwrap_or((blast_p1_id, "Fire-Col".to_string()));
    let fire_lam = ke_factor.clamp(0.0, 0.90);
    let ef_fire = evaluate_eigenflow_studio(
        bim,
        site,
        &EigenFlowConfig {
            disrupted_element_id: Some(fire_p1_id),
            stiffness_retention_lambda: fire_lam,
            ..EigenFlowConfig::default()
        },
    );
    let fire_eigenflow = ExtremeEventEigenFlowCoupling {
        trigger_label: format!(
            "Fire {} @ {:.0} min (L{})",
            cfg.fire_curve.label(),
            cfg.exposure_minutes,
            cfg.fire_story_idx + 1
        ),
        critical_p1_element_id: fire_p1_id,
        critical_p1_element_name: fire_p1_name,
        derived_lambda: fire_lam,
        woodbury_ms: ef_fire.woodbury.stability_margin_ms,
        woodbury_rrr: ef_fire.woodbury.residual_reserve_ratio_rrr,
        woodbury_mu: ef_fire.woodbury.max_ductility_mu,
        total_shed_force_kn: ef_fire.woodbury.total_shed_force_kn,
        eigenflow_score: ef_fire.non_linear_eigenflow_system,
        is_progressive_collapse_risk: ef_fire.woodbury.is_mechanism
            || ef_fire.woodbury.non_linear_ci >= 0.40,
        recommendation: ef_fire.woodbury.prescriptive_recommendation.clone(),
    };

    let summary_verdict = format!(
        "Blast ({:.0}kg {} @ {:.1}m): Pr={:.0} kPa ({:?}, m_s={:.3}) | Fire ({:.0} min @ L{}): Tg={:.0}C, kE={:.2} (m_s={:.3})",
        cfg.explosive_mass_kg,
        cfg.explosive_type.label(),
        cfg.standoff_distance_m,
        blast_report.blast_wave.peak_reflected_overpressure_kpa,
        worst_blast_damage,
        blast_eigenflow.woodbury_ms,
        cfg.exposure_minutes,
        cfg.fire_story_idx + 1,
        gas_temp_c,
        ke_factor,
        fire_eigenflow.woodbury_ms,
    );

    ExtremeEventsReport {
        config: cfg,
        charge_pos_enu_m,
        target_face_enu_m,
        blast_report,
        blast_member_impacts: impacts,
        worst_blast_damage,
        fire_report,
        fire_compartment,
        exposed_fire_members,
        blast_eigenflow,
        fire_eigenflow,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_blast_standoff_scaling_and_eigenflow_coupling() {
        let bim = BimProject::sample_bimcraft_building();
        let rep_far = evaluate_extreme_events(
            &bim,
            None,
            &ExtremeEventsConfig {
                standoff_distance_m: 30.0,
                explosive_mass_kg: 100.0,
                ..ExtremeEventsConfig::default()
            },
        );
        let rep_close = evaluate_extreme_events(
            &bim,
            None,
            &ExtremeEventsConfig {
                standoff_distance_m: 5.0,
                explosive_mass_kg: 500.0,
                ..ExtremeEventsConfig::default()
            },
        );
        assert!(
            rep_close.blast_report.blast_wave.peak_reflected_overpressure_kpa
                > rep_far.blast_report.blast_wave.peak_reflected_overpressure_kpa
        );
        assert!(
            rep_close.blast_eigenflow.derived_lambda <= rep_far.blast_eigenflow.derived_lambda + 1e-6
        );
        assert!(!rep_close.blast_member_impacts.is_empty());
        assert!(rep_close.blast_eigenflow.woodbury_ms.is_finite());
    }

    #[test]
    fn test_phase84_fire_thermal_degradation_and_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep_protected = evaluate_extreme_events(
            &bim,
            None,
            &ExtremeEventsConfig {
                mode: ExtremeEventMode::StructuralFire,
                protection_kind: FireProtectionKind::SprayedMineralFiber,
                protection_thickness_mm: 30.0,
                exposure_minutes: 120.0,
                ..ExtremeEventsConfig::default()
            },
        );
        let rep_bare = evaluate_extreme_events(
            &bim,
            None,
            &ExtremeEventsConfig {
                mode: ExtremeEventMode::StructuralFire,
                protection_kind: FireProtectionKind::Unprotected,
                protection_thickness_mm: 0.0,
                exposure_minutes: 120.0,
                ..ExtremeEventsConfig::default()
            },
        );
        assert!(
            rep_bare.fire_compartment.member_temp_c >= rep_protected.fire_compartment.member_temp_c
        );
        assert!(rep_bare.fire_compartment.ke_factor <= rep_protected.fire_compartment.ke_factor + 1e-6);
        assert!(!rep_bare.exposed_fire_members.is_empty());

        let base_ifcx = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed =
            IfcxStage::compose_with_extreme_events_overlay(&base_ifcx, &rep_bare.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::hazard::extreme_events"));
        assert!(json.contains("/earth/hazard/extreme_events"));
        assert!(!json.contains("ARI"));
    }
}
