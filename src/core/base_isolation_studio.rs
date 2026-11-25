//! core/base_isolation_studio.rs — Update `v0.8.4.1` (`1/10` -> `v0.8.5`):
//! Seismic Base Isolation & Damper 3D Studio Engine (`ASCE 7-22 Ch. 17–18` / `SNI 1726 Pasal 12–13`).
//!
//! Couples active `BimProject` column grid, building height/stories, and `SiteEnvironmentalCoupling`
//! MCE_R spectral accelerations (`S_MS`, `S_M1`) into [`SeismicIsolationEngine`] (`LRB`, `HDRB`, `FPS`, `FVD`),
//! generating 3D ENU bearing pads, perimeter seismic moat clearance geometry, supplementary damper struts,
//! and non-destructive `eigengraph::analysis::seismic_isolation` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::isolation::{
    FluidViscousDamperParams, FrictionPendulumParams, IsolatorType, LeadRubberBearingParams,
    SeismicIsolationEngine, SeismicIsolationInput, SeismicIsolationReport,
};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;

/// Canonical EigenGraph IFCX attribute namespace for Seismic Base Isolation & Damper overlays.
pub const NS_EG_SEISMIC_ISOLATION: &str = "eigengraph::analysis::seismic_isolation";

/// Persisted configuration for the Seismic Base Isolation & Damper 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsolationStudioConfig {
    pub isolator_type: IsolatorType,
    pub lrb_outer_diameter_mm: f64,
    pub lrb_lead_core_mm: f64,
    pub fps_radius_mm: f64,
    pub fvd_damping_c: f64,
    pub num_dampers: usize,
    pub superstructure_r_i: f64,
}

impl Default for IsolationStudioConfig {
    fn default() -> Self {
        Self {
            isolator_type: IsolatorType::LeadRubberBearing,
            lrb_outer_diameter_mm: 850.0,
            lrb_lead_core_mm: 160.0,
            fps_radius_mm: 2800.0,
            fvd_damping_c: 650.0,
            num_dampers: 8,
            superstructure_r_i: 1.68,
        }
    }
}

/// 3D ENU-anchored isolation bearing pad located beneath each base structural column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsolatorPad3D {
    pub id: usize,
    pub label: String,
    pub pos_enu_m: [f64; 3],
    pub diameter_m: f64,
    pub height_m: f64,
    pub axial_load_kn: f64,
    pub buckling_sf: f64,
    pub is_safe: bool,
}

/// 3D supplementary Fluid Viscous Damper brace segment on lower frame perimeter bays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DamperStrut3D {
    pub p0_enu_m: [f64; 3],
    pub p1_enu_m: [f64; 3],
    pub peak_force_kn: f64,
}

/// Perimeter seismic moat clearance envelope surrounding the isolated superstructure footprint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeismicMoatRect3D {
    pub center_x_m: f64,
    pub center_y_m: f64,
    pub inner_width_m: f64,
    pub inner_depth_m: f64,
    pub moat_gap_m: f64,
    pub z_m: f64,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::seismic_isolation`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeismicIsolationIfcxOverlay {
    pub schema_version: String,
    pub isolator_type: String,
    pub num_bearings: usize,
    pub num_dampers: usize,
    pub seismic_weight_w_kn: f64,
    pub fixed_period_s: f64,
    pub effective_period_tm_s: f64,
    pub effective_damping_pct: f64,
    pub max_displacement_dm_mm: f64,
    pub total_max_displacement_dtm_mm: f64,
    pub required_moat_gap_mm: f64,
    pub fixed_base_shear_kn: f64,
    pub isolated_base_shear_vs_kn: f64,
    pub shear_reduction_pct: f64,
    pub buckling_safety_factor: f64,
    pub is_design_acceptable: bool,
    pub summary_verdict: String,
}

/// Unified Seismic Base Isolation & Damper Studio Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationStudioReport {
    pub config: IsolationStudioConfig,
    pub input_used: SeismicIsolationInput,
    pub report: SeismicIsolationReport,
    pub pads_3d: Vec<IsolatorPad3D>,
    pub dampers_3d: Vec<DamperStrut3D>,
    pub moat_3d: SeismicMoatRect3D,
    pub fixed_base_shear_kn: f64,
    pub shear_reduction_pct: f64,
    pub period_shift_ratio: f64,
    pub summary_verdict: String,
}

impl IsolationStudioReport {
    /// Converts the isolation studio report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> SeismicIsolationIfcxOverlay {
        let r = &self.report;
        let lbl = match r.isolator_type {
            IsolatorType::LeadRubberBearing => "Lead-Rubber Bearing (LRB)",
            IsolatorType::HighDampingRubberBearing => "High-Damping Rubber Bearing (HDRB)",
            IsolatorType::FrictionPendulum => "Friction Pendulum System (FPS)",
            IsolatorType::FluidViscousDamper => "Fluid Viscous Damper (FVD)",
        };
        SeismicIsolationIfcxOverlay {
            schema_version: "seismic-isolation-ifcx-v1".to_string(),
            isolator_type: lbl.to_string(),
            num_bearings: self.input_used.num_bearings,
            num_dampers: self.input_used.num_dampers,
            seismic_weight_w_kn: self.input_used.seismic_weight_w_kn,
            fixed_period_s: self.input_used.fixed_base_period_s,
            effective_period_tm_s: r.effective_period_tm_s,
            effective_damping_pct: r.combined_total_damping * 100.0,
            max_displacement_dm_mm: r.max_displacement_dm_mm,
            total_max_displacement_dtm_mm: r.total_max_displacement_dtm_mm,
            required_moat_gap_mm: r.required_moat_gap_mm,
            fixed_base_shear_kn: self.fixed_base_shear_kn,
            isolated_base_shear_vs_kn: r.superstructure_design_base_shear_vs_kn,
            shear_reduction_pct: self.shear_reduction_pct,
            buckling_safety_factor: r.buckling_safety_factor,
            is_design_acceptable: r.is_design_acceptable,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Evaluates Seismic Base Isolation & Damper performance coupled to live BIM geometry and site hazard.
pub fn evaluate_isolation_studio(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &IsolationStudioConfig,
) -> IsolationStudioReport {
    let mut cfg = config.clone();
    cfg.lrb_outer_diameter_mm = cfg.lrb_outer_diameter_mm.clamp(450.0, 1_500.0);
    cfg.lrb_lead_core_mm = cfg
        .lrb_lead_core_mm
        .clamp(60.0, cfg.lrb_outer_diameter_mm * 0.35);
    cfg.fps_radius_mm = cfg.fps_radius_mm.clamp(1_200.0, 6_000.0);
    cfg.fvd_damping_c = cfg.fvd_damping_c.clamp(100.0, 3_000.0);
    cfg.superstructure_r_i = cfg.superstructure_r_i.clamp(1.0, 2.5);

    // 1. Extract base column coordinates in local ENU meters
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
        .max(3_600.0);

    let mut base_pts: Vec<(String, [f64; 2])> = Vec::new();
    for c in &bim.columns {
        let base_z = bim.level_elevation(c.base_level_id);
        if (base_z - min_elev_mm).abs() <= 500.0 || bim.columns.len() <= 16 {
            let p = [c.pos.0 * 0.001, c.pos.1 * 0.001];
            let dup = base_pts
                .iter()
                .any(|(_, q)| (q[0] - p[0]).hypot(q[1] - p[1]) < 0.45);
            if !dup {
                base_pts.push((c.name.clone(), p));
            }
        }
    }
    if base_pts.len() < 4 {
        base_pts = vec![
            ("C1".to_string(), [0.0, 0.0]),
            ("C2".to_string(), [6.0, 0.0]),
            ("C3".to_string(), [12.0, 0.0]),
            ("C4".to_string(), [0.0, 6.0]),
            ("C5".to_string(), [6.0, 6.0]),
            ("C6".to_string(), [12.0, 6.0]),
        ];
    }

    let min_x = base_pts.iter().map(|(_, p)| p[0]).fold(f64::INFINITY, f64::min);
    let max_x = base_pts
        .iter()
        .map(|(_, p)| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = base_pts.iter().map(|(_, p)| p[1]).fold(f64::INFINITY, f64::min);
    let max_y = base_pts
        .iter()
        .map(|(_, p)| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let b_plan = (max_x - min_x).max(12.0);
    let d_plan = (max_y - min_y).max(12.0);
    let cx = 0.5 * (min_x + max_x);
    let cy = 0.5 * (min_y + max_y);

    let stories = bim.levels.len().saturating_sub(1).clamp(1, 30) as f64;
    let height_m = ((max_elev_mm - min_elev_mm) * 0.001).max(stories * 3.6);
    let area_m2 = (b_plan * d_plan).max(120.0);
    // Floor gravity seismic weight density ~11.5 kN/m^2 + 30% tributary overhang
    let seismic_weight_w_kn = (stories * area_m2 * 1.3 * 11.5).clamp(4_800.0, 350_000.0);
    let num_bearings = base_pts.len().max(4);
    let axial_load_per_bearing_kn = seismic_weight_w_kn / (num_bearings as f64);
    let fixed_base_period_s = (0.075 * height_m.powf(0.75)).clamp(0.28, 2.40);

    let (sms_g, sm1_g) = if let Some(s) = site {
        (
            (s.seismic_pga_ss_g as f64 * 1.15).clamp(0.40, 2.20),
            (s.seismic_pga_s1_g as f64 * 1.20).clamp(0.18, 1.20),
        )
    } else {
        (1.25, 0.65)
    };

    let lrb_params = LeadRubberBearingParams {
        outer_diameter_mm: cfg.lrb_outer_diameter_mm,
        lead_core_diameter_mm: cfg.lrb_lead_core_mm,
        ..LeadRubberBearingParams::default()
    };
    let fps_params = FrictionPendulumParams {
        radius_of_curvature_mm: cfg.fps_radius_mm,
        ..FrictionPendulumParams::default()
    };
    let fps_housing_diam_mm = fps_params.housing_diameter_mm;
    let fvd_params = FluidViscousDamperParams {
        damping_coeff_c: cfg.fvd_damping_c,
        ..FluidViscousDamperParams::default()
    };

    let input_used = SeismicIsolationInput {
        isolator_type: cfg.isolator_type,
        seismic_weight_w_kn,
        num_bearings,
        axial_load_per_bearing_kn,
        spectral_accel_sm1_g: sm1_g,
        spectral_accel_sms_g: sms_g,
        fixed_base_period_s,
        superstructure_r_i: cfg.superstructure_r_i,
        building_plan_b_m: b_plan,
        building_plan_d_m: d_plan,
        plan_eccentricity_e_m: 0.05 * b_plan,
        lrb_params,
        fps_params,
        fvd_params,
        num_dampers: cfg.num_dampers,
    };

    let report = SeismicIsolationEngine::evaluate(&input_used);

    let pad_diam_m = match cfg.isolator_type {
        IsolatorType::FrictionPendulum => (fps_housing_diam_mm * 0.001).clamp(0.6, 1.8),
        _ => (cfg.lrb_outer_diameter_mm * 0.001).clamp(0.45, 1.5),
    };
    let pad_h_m = ((report.total_rubber_thick_tr_mm * 0.001) + 0.14).clamp(0.28, 0.72);
    let pads_3d: Vec<IsolatorPad3D> = base_pts
        .iter()
        .enumerate()
        .map(|(idx, (nm, p))| IsolatorPad3D {
            id: idx + 1,
            label: nm.clone(),
            pos_enu_m: [p[0], p[1], min_elev_mm * 0.001],
            diameter_m: pad_diam_m,
            height_m: pad_h_m,
            axial_load_kn: axial_load_per_bearing_kn,
            buckling_sf: report.buckling_safety_factor,
            is_safe: report.buckling_safety_factor >= 1.10,
        })
        .collect();

    let moat_gap_m = (report.required_moat_gap_mm * 0.001).clamp(0.25, 1.8);
    let moat_3d = SeismicMoatRect3D {
        center_x_m: cx,
        center_y_m: cy,
        inner_width_m: b_plan + pad_diam_m + 0.4,
        inner_depth_m: d_plan + pad_diam_m + 0.4,
        moat_gap_m,
        z_m: min_elev_mm * 0.001 + 0.08,
    };

    // Diagonal / chevron damper struts along perimeter base story when FVD or dampers active
    let mut dampers_3d = Vec::new();
    if cfg.num_dampers > 0 && base_pts.len() >= 2 {
        let z0 = min_elev_mm * 0.001 + 0.25;
        let z1 = z0 + 3.4;
        for i in 0..base_pts.len().min(cfg.num_dampers).min(8) {
            let a = base_pts[i].1;
            let b = base_pts[(i + 1) % base_pts.len()].1;
            if (a[0] - b[0]).hypot(a[1] - b[1]) >= 1.5 {
                dampers_3d.push(DamperStrut3D {
                    p0_enu_m: [a[0], a[1], z0],
                    p1_enu_m: [b[0], b[1], z1],
                    peak_force_kn: report.damper_peak_force_kn.max(120.0),
                });
            }
        }
    }

    let c_s_fixed = (sm1_g / (fixed_base_period_s * 2.5)).min(sms_g / 2.5);
    let fixed_base_shear_kn = (c_s_fixed * seismic_weight_w_kn)
        .max(0.044 * sms_g * seismic_weight_w_kn)
        .max(1.0);
    let shear_reduction_pct = ((1.0 - (report.superstructure_design_base_shear_vs_kn / fixed_base_shear_kn))
        * 100.0)
        .clamp(0.0, 95.0);
    let period_shift_ratio = report.effective_period_tm_s / fixed_base_period_s.max(0.1);

    let iso_short = match cfg.isolator_type {
        IsolatorType::LeadRubberBearing => "LRB",
        IsolatorType::HighDampingRubberBearing => "HDRB",
        IsolatorType::FrictionPendulum => "FPS",
        IsolatorType::FluidViscousDamper => "FVD",
    };
    let summary_verdict = format!(
        "{iso_short}: T_M={:.2}s ({:.1}x T_fix) | Vs reduced {:.0}% ({:.0}->{:.0} kN) | DTM={:.0} mm | Moat={:.0} mm | SF_cr={:.2}",
        report.effective_period_tm_s,
        period_shift_ratio,
        shear_reduction_pct,
        fixed_base_shear_kn,
        report.superstructure_design_base_shear_vs_kn,
        report.total_max_displacement_dtm_mm,
        report.required_moat_gap_mm,
        report.buckling_safety_factor,
    );

    IsolationStudioReport {
        config: cfg,
        input_used,
        report,
        pads_3d,
        dampers_3d,
        moat_3d,
        fixed_base_shear_kn,
        shear_reduction_pct,
        period_shift_ratio,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_1_isolation_studio_lrb_fps_fvd_and_period_shift() {
        let bim = BimProject::sample_bimcraft_building();
        for iso in [
            IsolatorType::LeadRubberBearing,
            IsolatorType::HighDampingRubberBearing,
            IsolatorType::FrictionPendulum,
            IsolatorType::FluidViscousDamper,
        ] {
            let rep = evaluate_isolation_studio(
                &bim,
                None,
                &IsolationStudioConfig {
                    isolator_type: iso,
                    ..IsolationStudioConfig::default()
                },
            );
            assert!(!rep.pads_3d.is_empty());
            assert!(rep.report.effective_period_tm_s.is_finite());
            match iso {
                IsolatorType::FluidViscousDamper => {
                    assert!(rep.report.effective_period_tm_s >= 0.28);
                    assert!(rep.report.combined_total_damping > 0.05);
                }
                _ => {
                    assert!(rep.report.effective_period_tm_s > rep.input_used.fixed_base_period_s);
                    assert!(rep.report.effective_period_tm_s > 0.5);
                }
            }
            assert!(rep.report.total_max_displacement_dtm_mm.is_finite());
            assert!(!rep.report.hysteresis_loop.is_empty());
            assert!(!rep.report.regulatory_checks.is_empty());
        }
    }

    #[test]
    fn test_phase84_1_isolation_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_isolation_studio(&bim, None, &IsolationStudioConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_isolation_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::seismic_isolation"));
        assert!(json.contains("/earth/analysis/seismic_isolation"));
        assert!(!json.contains("ARI"));
    }
}
