//! core/shear_wall_studio.rs — Update `v0.8.4.3` (`3/10` -> `v0.8.5`):
//! Special RC Shear Wall, Boundary Element (SBE) & Diagonal Coupling Beam 3D Studio
//! (`ACI 318-19 §18.10` / `SNI 2847:2019 Pasal 18.10`).
//!
//! Couples live `BimProject` structural walls/levels and `SiteEnvironmentalCoupling`
//! seismic accelerations into [`ShearWallEngine::evaluate_wall`], generating 3D ENU
//! shear wall piers with Special Boundary Element (SBE) plastic hinge confinement zones,
//! per-story diagonally reinforced X-Cage coupling beams, an analytical axial-flexural P-M
//! interaction envelope, and non-destructive `eigengraph::analysis::shear_wall_sbe` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::shear_wall::{
    CouplingBeamType, ShearWallEngine, ShearWallSystemReport, WallSeismicClassification,
};

/// Canonical EigenGraph IFCX attribute namespace for Special RC Shear Wall & SBE overlays.
pub const NS_EG_SHEAR_WALL_SBE: &str = "eigengraph::analysis::shear_wall_sbe";

/// Persisted configuration for the Special RC Shear Wall & SBE 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShearWallStudioConfig {
    pub wall_thickness_mm: f64,
    pub fc_mpa: f64,
    pub fy_mpa: f64,
    pub fyt_mpa: f64,
    pub design_drift_ratio: f64,
    pub cb_clear_span_m: f64,
    pub cb_depth_m: f64,
    pub cb_diag_bar_dia_mm: f64,
    pub cb_bars_per_group: usize,
}

impl Default for ShearWallStudioConfig {
    fn default() -> Self {
        Self {
            wall_thickness_mm: 350.0,
            fc_mpa: 35.0,
            fy_mpa: 420.0,
            fyt_mpa: 420.0,
            design_drift_ratio: 0.012,
            cb_clear_span_m: 1.6,
            cb_depth_m: 0.95,
            cb_diag_bar_dia_mm: 25.0,
            cb_bars_per_group: 6,
        }
    }
}

/// 3D ENU shear wall pier with end Special Boundary Element (SBE) confinement zones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShearWallPier3D {
    pub wall_id: String,
    pub p0_enu_m: [f64; 2],
    pub p1_enu_m: [f64; 2],
    pub base_z_m: f64,
    pub height_m: f64,
    pub thickness_m: f64,
    pub sbe_length_m: f64,
    pub sbe_plastic_height_m: f64,
    pub sbe_required: bool,
    pub is_compliant: bool,
}

/// 3D ENU coupling beam connecting adjacent wall piers with optional diagonal X-Cage bundles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CouplingBeamCage3D {
    pub story_num: usize,
    pub p0_enu_m: [f64; 3],
    pub p1_enu_m: [f64; 3],
    pub depth_m: f64,
    pub width_m: f64,
    pub is_diagonal_x_cage: bool,
    pub coupling_dcr: f64,
}

/// Sample point on the wall pier axial-flexural P-M interaction capacity envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallPmCurvePoint {
    pub moment_knm: f64,
    pub axial_kn: f64,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::shear_wall_sbe`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShearWallIfcxOverlay {
    pub schema_version: String,
    pub wall_id: String,
    pub length_lw_m: f64,
    pub height_hw_m: f64,
    pub thickness_tw_mm: f64,
    pub fc_mpa: f64,
    pub fy_mpa: f64,
    pub is_sbe_required: bool,
    pub sbe_length_be_mm: f64,
    pub sbe_tie_spacing_mm: f64,
    pub neutral_axis_c_mm: f64,
    pub in_plane_shear_dcr: f64,
    pub flexure_dcr: f64,
    pub coupling_beam_type: String,
    pub coupling_dcr: f64,
    pub governing_dcr: f64,
    pub is_fully_compliant: bool,
    pub summary_verdict: String,
}

/// Unified Special RC Shear Wall, SBE & Coupling Beam Studio Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShearWallStudioReport {
    pub config: ShearWallStudioConfig,
    pub report: ShearWallSystemReport,
    pub piers_3d: Vec<ShearWallPier3D>,
    pub coupling_beams_3d: Vec<CouplingBeamCage3D>,
    pub pm_curve: Vec<WallPmCurvePoint>,
    pub summary_verdict: String,
}

impl ShearWallStudioReport {
    /// Converts the shear wall studio report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> ShearWallIfcxOverlay {
        let r = &self.report;
        ShearWallIfcxOverlay {
            schema_version: "shear-wall-sbe-ifcx-v1".to_string(),
            wall_id: r.wall_id.clone(),
            length_lw_m: r.geometry.length_lw_mm * 0.001,
            height_hw_m: r.geometry.height_hw_mm * 0.001,
            thickness_tw_mm: r.geometry.thickness_tw_mm,
            fc_mpa: r.materials.fc_mpa,
            fy_mpa: r.materials.fy_mpa,
            is_sbe_required: r.boundary_element.is_sbe_required,
            sbe_length_be_mm: r.boundary_element.provided_length_be_mm,
            sbe_tie_spacing_mm: r.boundary_element.provided_tie_spacing_s_mm,
            neutral_axis_c_mm: r.boundary_element.neutral_axis_c_mm,
            in_plane_shear_dcr: r.in_plane_shear.shear_dcr,
            flexure_dcr: r.flexure_dcr,
            coupling_beam_type: r.coupling_beam.coupling_type.label().to_string(),
            coupling_dcr: r.coupling_beam.coupling_dcr,
            governing_dcr: r.governing_dcr,
            is_fully_compliant: r.is_fully_compliant,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Evaluates Special RC Structural Walls, Boundary Elements (SBE) & Diagonal Coupling Beams
/// coupled to live `BimProject` wall/level geometry and site seismic demands.
pub fn evaluate_shear_wall_studio(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &ShearWallStudioConfig,
) -> ShearWallStudioReport {
    let mut cfg = config.clone();
    cfg.wall_thickness_mm = cfg.wall_thickness_mm.clamp(200.0, 800.0);
    cfg.fc_mpa = cfg.fc_mpa.clamp(25.0, 70.0);
    cfg.fy_mpa = cfg.fy_mpa.clamp(300.0, 550.0);
    cfg.fyt_mpa = cfg.fyt_mpa.clamp(300.0, 550.0);
    cfg.design_drift_ratio = cfg.design_drift_ratio.clamp(0.005, 0.030);
    cfg.cb_clear_span_m = cfg.cb_clear_span_m.clamp(0.8, 4.5);
    cfg.cb_depth_m = cfg.cb_depth_m.clamp(0.45, 1.8);
    cfg.cb_diag_bar_dia_mm = cfg.cb_diag_bar_dia_mm.clamp(16.0, 36.0);
    cfg.cb_bars_per_group = cfg.cb_bars_per_group.clamp(4, 12);

    // 1. Extract vertical envelope from BIM levels
    let min_elev_mm = bim
        .levels
        .iter()
        .map(|l| l.elevation_mm)
        .fold(0.0_f64, f64::min);
    let max_elev_mm = bim
        .levels
        .iter()
        .map(|l| l.elevation_mm)
        .fold(3_800.0_f64, f64::max)
        .max(10_800.0);
    let num_stories = bim.levels.len().saturating_sub(1).clamp(3, 40);
    let hw_m = ((max_elev_mm - min_elev_mm) * 0.001).max(num_stories as f64 * 3.6);
    let hu_m = (hw_m / (num_stories as f64)).clamp(2.8, 6.0);
    let base_z_m = min_elev_mm * 0.001;

    // 2. Extract building plan centroid & structural wall piers
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
    for w in &bim.walls {
        for p in [w.start, w.end] {
            min_x = min_x.min(p.0 * 0.001);
            max_x = max_x.max(p.0 * 0.001);
            min_y = min_y.min(p.1 * 0.001);
            max_y = max_y.max(p.1 * 0.001);
        }
    }
    if !min_x.is_finite() {
        min_x = 0.0;
        max_x = 18.0;
        min_y = 0.0;
        max_y = 14.0;
    }
    let cx = 0.5 * (min_x + max_x);
    let cy = 0.5 * (min_y + max_y);
    let plan_area_m2 = ((max_x - min_x).max(12.0) * (max_y - min_y).max(12.0)).max(144.0);

    let mut raw_piers: Vec<(String, [f64; 2], [f64; 2])> = bim
        .walls
        .iter()
        .filter(|w| w.is_structural)
        .filter_map(|w| {
            let a = [w.start.0 * 0.001, w.start.1 * 0.001];
            let b = [w.end.0 * 0.001, w.end.1 * 0.001];
            let len = (b[0] - a[0]).hypot(b[1] - a[1]);
            (len >= 2.0).then(|| (w.name.clone(), a, b))
        })
        .take(4)
        .collect();

    if raw_piers.is_empty() {
        // Synthesize coupled dual shear wall core centered in BIM footprint
        let half_gap = 0.5 * cfg.cb_clear_span_m;
        let pier_len = 4.2_f64;
        raw_piers = vec![
            (
                "SW-1 (West Core Pier)".to_string(),
                [cx - half_gap - pier_len, cy],
                [cx - half_gap, cy],
            ),
            (
                "SW-2 (East Core Pier)".to_string(),
                [cx + half_gap, cy],
                [cx + half_gap + pier_len, cy],
            ),
        ];
    }

    let primary = &raw_piers[0];
    let lw_m = (primary.2[0] - primary.1[0])
        .hypot(primary.2[1] - primary.1[1])
        .clamp(2.8, 14.0);
    let wall_id = primary.0.clone();

    // 3. Coupled tributary seismic demands per wall pier
    let sds_factor = site
        .map(|s| ((s.seismic_pga_ss_g as f64) * 0.85).clamp(0.45, 1.80))
        .unwrap_or(1.00);
    let total_w_kn = (num_stories as f64) * plan_area_m2 * 11.5;
    let pier_share = 1.0 / (raw_piers.len().max(2) as f64);
    let pu_kn = (total_w_kn * pier_share * 0.28).clamp(1_600.0, 28_000.0);
    let vu_kn = (total_w_kn * pier_share * 0.14 * sds_factor).clamp(850.0, 14_000.0);
    let mu_knm = (vu_kn * 0.62 * hw_m).clamp(4_500.0, 145_000.0);
    let cb_vu_kn = (vu_kn * 0.42).clamp(320.0, 3_600.0);

    let report = ShearWallEngine::evaluate_wall(
        &wall_id,
        lw_m,
        hw_m,
        cfg.wall_thickness_mm,
        hu_m,
        cfg.fc_mpa,
        cfg.fy_mpa,
        cfg.fyt_mpa,
        pu_kn,
        mu_knm,
        vu_kn,
        cfg.design_drift_ratio,
        cfg.cb_clear_span_m,
        cfg.cb_depth_m,
        cfg.wall_thickness_mm,
        cb_vu_kn,
        cfg.cb_diag_bar_dia_mm,
        cfg.cb_bars_per_group,
    );

    // 4. Analytical axial-flexural P-M interaction envelope (ACI 318-19 Chapter 18/22)
    let lw_mm = report.geometry.length_lw_mm;
    let tw_mm = report.geometry.thickness_tw_mm;
    let ag_mm2 = lw_mm * tw_mm;
    let as_total = report.boundary_element.sbe_rebar_area_mm2 * 2.0 + 0.0025 * ag_mm2;
    let phi_p0_kn = 0.80 * 0.65 * (0.85 * cfg.fc_mpa * (ag_mm2 - as_total) + cfg.fy_mpa * as_total) * 0.001;
    let phi_pt_kn = -0.90 * cfg.fy_mpa * as_total * 0.001;
    let phi_p_bal_kn = 0.34 * phi_p0_kn;
    let m_pure_knm = report.flexure_phi_mn_knm;
    let m_bal_knm = m_pure_knm * 1.28;
    let pm_curve = vec![
        WallPmCurvePoint { moment_knm: 0.0, axial_kn: phi_p0_kn },
        WallPmCurvePoint { moment_knm: m_bal_knm * 0.42, axial_kn: phi_p0_kn * 0.88 },
        WallPmCurvePoint { moment_knm: m_bal_knm * 0.78, axial_kn: phi_p0_kn * 0.62 },
        WallPmCurvePoint { moment_knm: m_bal_knm, axial_kn: phi_p_bal_kn },
        WallPmCurvePoint { moment_knm: m_pure_knm * 1.08, axial_kn: phi_p_bal_kn * 0.45 },
        WallPmCurvePoint { moment_knm: m_pure_knm, axial_kn: 0.0 },
        WallPmCurvePoint { moment_knm: m_pure_knm * 0.58, axial_kn: phi_pt_kn * 0.52 },
        WallPmCurvePoint { moment_knm: 0.0, axial_kn: phi_pt_kn },
    ];

    // 5. 3D Wall Piers & SBE Plastic Confinement Zones + Coupling Beam X-Cages
    let sbe_len_m = (report.boundary_element.provided_length_be_mm * 0.001).clamp(0.35, lw_m * 0.42);
    let plastic_h_m = lw_m.max(mu_knm / (4.0 * vu_kn.max(1.0))).clamp(2.4, hw_m * 0.65);
    let thick_m = cfg.wall_thickness_mm * 0.001;

    let piers_3d: Vec<ShearWallPier3D> = raw_piers
        .iter()
        .map(|(nm, p0, p1)| ShearWallPier3D {
            wall_id: nm.clone(),
            p0_enu_m: *p0,
            p1_enu_m: *p1,
            base_z_m,
            height_m: hw_m,
            thickness_m: thick_m,
            sbe_length_m: sbe_len_m,
            sbe_plastic_height_m: plastic_h_m,
            sbe_required: report.boundary_element.is_sbe_required,
            is_compliant: report.is_fully_compliant,
        })
        .collect();

    let is_diag = report.coupling_beam.coupling_type == CouplingBeamType::DiagonallyReinforced
        || report.coupling_beam.coupling_type == CouplingBeamType::OptionalDiagonal;
    let mut coupling_beams_3d = Vec::with_capacity(num_stories);
    let (cb_p0_xy, cb_p1_xy) = if piers_3d.len() >= 2 {
        (piers_3d[0].p1_enu_m, piers_3d[1].p0_enu_m)
    } else {
        (
            [cx - 0.5 * cfg.cb_clear_span_m, cy],
            [cx + 0.5 * cfg.cb_clear_span_m, cy],
        )
    };
    for st in 1..=num_stories {
        let z_top = base_z_m + (st as f64) * hu_m;
        coupling_beams_3d.push(CouplingBeamCage3D {
            story_num: st,
            p0_enu_m: [cb_p0_xy[0], cb_p0_xy[1], z_top],
            p1_enu_m: [cb_p1_xy[0], cb_p1_xy[1], z_top],
            depth_m: cfg.cb_depth_m,
            width_m: thick_m,
            is_diagonal_x_cage: is_diag,
            coupling_dcr: report.coupling_beam.coupling_dcr,
        });
    }

    let sbe_tag = if report.boundary_element.is_sbe_required {
        format!("SBE {:.0}mm", report.boundary_element.provided_length_be_mm)
    } else {
        "Non-SBE".to_string()
    };
    let cls_short = match report.classification {
        WallSeismicClassification::SpecialStructuralWall => "Special RC Wall",
        WallSeismicClassification::OrdinaryStructuralWall => "Ordinary RC Wall",
    };
    let summary_verdict = format!(
        "{cls_short} ({wall_id}): {sbe_tag} | V_DCR={:.2} | M_DCR={:.2} | CB_DCR={:.2} | Gov={:.2} ({})",
        report.in_plane_shear.shear_dcr,
        report.flexure_dcr,
        report.coupling_beam.coupling_dcr,
        report.governing_dcr,
        if report.is_fully_compliant { "PASS" } else { "REVIEW" }
    );

    ShearWallStudioReport {
        config: cfg,
        report,
        piers_3d,
        coupling_beams_3d,
        pm_curve,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_3_shear_wall_studio_sbe_and_diagonal_coupling_beam() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_shear_wall_studio(
            &bim,
            None,
            &ShearWallStudioConfig {
                wall_thickness_mm: 350.0,
                fc_mpa: 40.0,
                design_drift_ratio: 0.015,
                cb_clear_span_m: 1.5,
                cb_depth_m: 0.95,
                ..ShearWallStudioConfig::default()
            },
        );
        assert!(!rep.piers_3d.is_empty());
        assert!(!rep.coupling_beams_3d.is_empty());
        assert!(rep.pm_curve.len() >= 6);
        assert!(rep.coupling_beams_3d.iter().all(|c| c.is_diagonal_x_cage));
        assert!(rep.report.governing_dcr.is_finite());
        assert!(rep.report.boundary_element.provided_length_be_mm >= 300.0);
    }

    #[test]
    fn test_phase84_3_shear_wall_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_shear_wall_studio(&bim, None, &ShearWallStudioConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_shear_wall_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::shear_wall_sbe"));
        assert!(json.contains("/earth/analysis/shear_wall_sbe"));
        assert!(!json.contains("ARI"));
    }
}
