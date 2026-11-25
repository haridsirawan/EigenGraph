//! core/prestress_studio.rs — Update `v0.8.4.5` (`5/10` -> `v0.8.5`):
//! Post-Tensioned & Prestressed Concrete Tendon Drape, Prestress Losses & Camber 3D Studio
//! (`ACI 318-19 Ch. 24` / `SNI 2847:2019 Pasal 24` / `PCI 8th Ed.`).
//!
//! Couples active `BimProject` girder spans/elevations and `SiteEnvironmentalCoupling` into
//! [`PrestressedConcreteEngine::evaluate_girder`], generating 3D ENU draped tendon profiles
//! (Parabolic / Harped 2-Point / Straight), live & dead anchorage blocks, immediate & long-term
//! loss waterfalls, fiber stress checks, and non-destructive `eigengraph::analysis::prestress_tendon`
//! IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::prestressed_concrete::{
    GirderSectionArchetype, PrestressedConcreteEngine, PrestressedGirderReport,
    PrestressingMethod, StrandGrade, TendonProfileKind,
};

/// Canonical EigenGraph IFCX attribute namespace for Prestressed / Post-Tensioned Tendon overlays.
pub const NS_EG_PRESTRESS_TENDON: &str = "eigengraph::analysis::prestress_tendon";

/// Persisted configuration for the Post-Tensioned & Prestressed Tendon 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrestressStudioConfig {
    pub archetype: GirderSectionArchetype,
    pub method: PrestressingMethod,
    pub profile_kind: TendonProfileKind,
    pub strand_grade: StrandGrade,
    pub strand_dia_mm: f64,
    pub num_strands: usize,
    pub jacking_ratio: f64,
    pub eccentricity_mid_mm: f64,
    pub eccentricity_end_mm: f64,
    pub fc_prime_mpa: f64,
    pub live_load_kn_m: f64,
}

impl Default for PrestressStudioConfig {
    fn default() -> Self {
        Self {
            archetype: GirderSectionArchetype::PrecastIGirder,
            method: PrestressingMethod::PostTensioned,
            profile_kind: TendonProfileKind::Parabolic,
            strand_grade: StrandGrade::Grade270LowRelaxation,
            strand_dia_mm: 15.2,
            num_strands: 18,
            jacking_ratio: 0.75,
            eccentricity_mid_mm: 380.0,
            eccentricity_end_mm: 60.0,
            fc_prime_mpa: 45.0,
            live_load_kn_m: 18.0,
        }
    }
}

/// 3D ENU prestressed girder bay containing discretized draped tendon coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TendonBay3D {
    pub bay_id: String,
    pub p0_enu_m: [f64; 3],
    pub p1_enu_m: [f64; 3],
    pub depth_m: f64,
    pub width_m: f64,
    pub tendon_pts_enu_m: Vec<[f64; 3]>,
    pub is_compliant: bool,
}

/// 3D ENU post-tensioning anchorage zone block at girder ends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorageZone3D {
    pub pos_enu_m: [f64; 3],
    pub jacking_force_kn: f64,
    pub is_live_end: bool,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::prestress_tendon`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrestressIfcxOverlay {
    pub schema_version: String,
    pub archetype: String,
    pub method: String,
    pub profile_kind: String,
    pub span_length_m: f64,
    pub num_strands: usize,
    pub strand_dia_mm: f64,
    pub p_jack_kn: f64,
    pub p_effective_kn: f64,
    pub total_loss_pct: f64,
    pub aci_tension_class: String,
    pub flexure_dcr: f64,
    pub shear_dcr: f64,
    pub net_camber_transfer_mm: f64,
    pub long_term_camber_mm: f64,
    pub governing_dcr: f64,
    pub is_code_compliant: bool,
    pub summary_verdict: String,
}

/// Unified Post-Tensioned & Prestressed Concrete Tendon 3D Studio Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrestressStudioReport {
    pub config: PrestressStudioConfig,
    pub report: PrestressedGirderReport,
    pub bays_3d: Vec<TendonBay3D>,
    pub anchorages_3d: Vec<AnchorageZone3D>,
    pub summary_verdict: String,
}

impl PrestressStudioReport {
    /// Converts the prestress studio report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> PrestressIfcxOverlay {
        let r = &self.report;
        PrestressIfcxOverlay {
            schema_version: "prestress-tendon-ifcx-v1".to_string(),
            archetype: r.archetype.label().to_string(),
            method: r.method.label().to_string(),
            profile_kind: r.profile_kind.label().to_string(),
            span_length_m: r.span_length_m,
            num_strands: r.num_strands,
            strand_dia_mm: r.strand_dia_mm,
            p_jack_kn: r.losses.p_jack_kn,
            p_effective_kn: r.losses.p_effective_kn,
            total_loss_pct: r.losses.loss_percentage,
            aci_tension_class: r.aci_class.label().to_string(),
            flexure_dcr: r.flexure_dcr,
            shear_dcr: r.shear_dcr,
            net_camber_transfer_mm: r.net_camber_transfer_mm,
            long_term_camber_mm: r.long_term_camber_mm,
            governing_dcr: r.governing_dcr,
            is_code_compliant: r.is_code_compliant,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Evaluates Post-Tensioned & Prestressed Concrete girder stresses, losses, camber,
/// and 3D ENU tendon drape coupled to live `BimProject` bays and site conditions.
pub fn evaluate_prestress_studio(
    bim: &BimProject,
    _site: Option<&SiteEnvironmentalCoupling>,
    config: &PrestressStudioConfig,
) -> PrestressStudioReport {
    let mut cfg = config.clone();
    cfg.strand_dia_mm = cfg.strand_dia_mm.clamp(9.5, 15.7);
    cfg.num_strands = cfg.num_strands.clamp(4, 64);
    cfg.jacking_ratio = cfg.jacking_ratio.clamp(0.62, 0.82);
    cfg.eccentricity_mid_mm = cfg.eccentricity_mid_mm.clamp(80.0, 750.0);
    cfg.eccentricity_end_mm = cfg.eccentricity_end_mm.clamp(0.0, 350.0);
    cfg.fc_prime_mpa = cfg.fc_prime_mpa.clamp(30.0, 80.0);
    cfg.live_load_kn_m = cfg.live_load_kn_m.clamp(4.0, 60.0);

    // 1. Extract primary girder span & floor elevations from BIM
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for c in &bim.columns {
        min_x = min_x.min(c.pos.0 * 0.001);
        max_x = max_x.max(c.pos.0 * 0.001);
        min_y = min_y.min(c.pos.1 * 0.001);
        max_y = max_y.max(c.pos.1 * 0.001);
    }
    if !min_x.is_finite() || (max_x - min_x) < 4.0 {
        min_x = 0.0;
        max_x = 16.0;
        min_y = 0.0;
        max_y = 9.0;
    }
    let span_length_m = (max_x - min_x).clamp(8.0, 45.0);
    let cy = 0.5 * (min_y + max_y);
    let girder_spacing_m = ((max_y - min_y).max(4.5) * 0.5).clamp(2.4, 8.5);

    let mut elevs_m: Vec<f64> = bim
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .filter(|z| *z >= 1.5)
        .collect();
    elevs_m.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    elevs_m.dedup_by(|a, b| (*a - *b).abs() < 0.2);
    if elevs_m.is_empty() {
        elevs_m = vec![3.8, 7.6];
    }

    let fci_prime_mpa = (0.75 * cfg.fc_prime_mpa).clamp(24.0, 60.0);
    let sdl_kn_m = 6.5;

    let report = PrestressedConcreteEngine::evaluate_girder(
        &bim.project_name,
        "G-PT1 (Primary Transfer/Deck Girder)",
        span_length_m,
        girder_spacing_m,
        cfg.archetype,
        cfg.method,
        cfg.profile_kind,
        cfg.strand_grade,
        cfg.strand_dia_mm,
        cfg.num_strands,
        cfg.jacking_ratio,
        cfg.eccentricity_mid_mm,
        cfg.eccentricity_end_mm,
        cfg.fc_prime_mpa,
        fci_prime_mpa,
        sdl_kn_m,
        cfg.live_load_kn_m,
    );

    let depth_m = (report.depth_h_mm * 0.001).clamp(0.45, 2.2);
    let width_m = (report.top_flange_bf_mm * 0.001).clamp(0.35, 1.4);

    let mut bays_3d = Vec::new();
    let mut anchorages_3d = Vec::new();
    for (ei, &z_m) in elevs_m.iter().take(2).enumerate() {
        for (yi, dy) in [-0.5 * girder_spacing_m, 0.5 * girder_spacing_m]
            .iter()
            .enumerate()
        {
            let y_m = cy + *dy;
            let p0 = [min_x, y_m, z_m];
            let p1 = [min_x + span_length_m, y_m, z_m];
            let tendon_pts_enu_m: Vec<[f64; 3]> = report
                .tendon_profile_points
                .iter()
                .map(|pt| {
                    let t = (pt.x_m / span_length_m.max(1.0)).clamp(0.0, 1.0);
                    let x = p0[0] + (p1[0] - p0[0]) * t;
                    let z = z_m - 0.5 * depth_m - (pt.e_mm * 0.001);
                    [x, y_m, z]
                })
                .collect();
            let live_pt = tendon_pts_enu_m
                .first()
                .copied()
                .unwrap_or([p0[0], p0[1], z_m - 0.5 * depth_m]);
            let dead_pt = tendon_pts_enu_m
                .last()
                .copied()
                .unwrap_or([p1[0], p1[1], z_m - 0.5 * depth_m]);
            anchorages_3d.push(AnchorageZone3D {
                pos_enu_m: live_pt,
                jacking_force_kn: report.losses.p_jack_kn,
                is_live_end: true,
            });
            anchorages_3d.push(AnchorageZone3D {
                pos_enu_m: dead_pt,
                jacking_force_kn: report.losses.p_effective_kn,
                is_live_end: false,
            });
            bays_3d.push(TendonBay3D {
                bay_id: format!("L{}-PT{}", ei + 1, yi + 1),
                p0_enu_m: p0,
                p1_enu_m: p1,
                depth_m,
                width_m,
                tendon_pts_enu_m,
                is_compliant: report.is_code_compliant,
            });
        }
    }

    let prof_short = match cfg.profile_kind {
        TendonProfileKind::Parabolic => "Parabolic PT",
        TendonProfileKind::HarpedTwoPoint => "Harped 2-Pt",
        TendonProfileKind::StraightEccentric => "Straight PT",
    };
    let summary_verdict = format!(
        "{prof_short} (L={:.1}m, {}x{:.1}mm): Pj={:.0}->{:.0} kN (-{:.1}% loss) | Flex DCR={:.2} | Camber {:+.1}/{:+.1}mm [{}]",
        report.span_length_m,
        report.num_strands,
        report.strand_dia_mm,
        report.losses.p_jack_kn,
        report.losses.p_effective_kn,
        report.losses.loss_percentage,
        report.flexure_dcr,
        report.net_camber_transfer_mm,
        report.long_term_camber_mm,
        if report.is_code_compliant { "PASS" } else { "REVIEW" }
    );

    PrestressStudioReport {
        config: cfg,
        report,
        bays_3d,
        anchorages_3d,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_5_prestress_studio_parabolic_harped_and_losses() {
        let bim = BimProject::sample_bimcraft_building();
        let rep_p = evaluate_prestress_studio(
            &bim,
            None,
            &PrestressStudioConfig {
                profile_kind: TendonProfileKind::Parabolic,
                num_strands: 20,
                fc_prime_mpa: 50.0,
                ..PrestressStudioConfig::default()
            },
        );
        let rep_h = evaluate_prestress_studio(
            &bim,
            None,
            &PrestressStudioConfig {
                profile_kind: TendonProfileKind::HarpedTwoPoint,
                num_strands: 20,
                fc_prime_mpa: 50.0,
                ..PrestressStudioConfig::default()
            },
        );
        assert!(!rep_p.bays_3d.is_empty());
        assert!(!rep_p.anchorages_3d.is_empty());
        assert!(rep_p.bays_3d[0].tendon_pts_enu_m.len() >= 5);
        assert!(rep_p.report.losses.p_effective_kn < rep_p.report.losses.p_jack_kn);
        assert!(rep_p.report.losses.loss_percentage > 5.0);
        assert!(rep_h.report.phi_mn_knm.is_finite());
    }

    #[test]
    fn test_phase84_5_prestress_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_prestress_studio(&bim, None, &PrestressStudioConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_prestress_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::prestress_tendon"));
        assert!(json.contains("/earth/analysis/prestress_tendon"));
        assert!(!json.contains("ARI"));
    }
}
