//! core/wind_studio.rs — Update `v0.8.4.2` (`2/10` -> `v0.8.5`):
//! Wind Aeroelasticity, Facade Cladding Pressure & ISO 10137 Comfort 3D Studio
//! (`ASCE 7-22 Ch. 26–30` / `SNI 1727:2020`).
//!
//! Couples live `BimProject` footprint/elevation geometry and `SiteEnvironmentalCoupling`
//! into [`WindEngine`], generating per-story 3D ENU facade pressure panels (`Windward`,
//! `Leeward`, `Sidewalls`), directional wind flow vectors, ISO 10137 occupant comfort audits,
//! Wind vs. Seismic lateral governing checks, and non-destructive `eigengraph::analysis::wind_aeroelasticity`
//! IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::wind::{
    EnclosureClassification, ExposureCategory, RiskCategory, WindAnalysisReport, WindEngine,
};

/// Canonical EigenGraph IFCX attribute namespace for Wind Aeroelasticity overlays.
pub const NS_EG_WIND_AEROELASTICITY: &str = "eigengraph::analysis::wind_aeroelasticity";

/// Persisted configuration for the Wind Aeroelasticity & ISO 10137 Comfort 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindStudioConfig {
    pub basic_wind_speed_ms: f64,
    pub exposure: ExposureCategory,
    pub risk_category: RiskCategory,
    pub enclosure: EnclosureClassification,
    pub wind_azimuth_deg: f64,
    pub occupancy_residential: bool,
    pub damping_ratio: f64,
}

impl Default for WindStudioConfig {
    fn default() -> Self {
        Self {
            basic_wind_speed_ms: 42.0,
            exposure: ExposureCategory::ExposureC,
            risk_category: RiskCategory::CategoryII,
            enclosure: EnclosureClassification::Enclosed,
            wind_azimuth_deg: 270.0,
            occupancy_residential: false,
            damping_ratio: 0.02,
        }
    }
}

/// 3D ENU facade pressure panel attached to a building face per story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacadePressurePanel3D {
    pub story_num: usize,
    pub face_label: String,
    pub center_enu_m: [f64; 3],
    pub width_m: f64,
    pub height_m: f64,
    pub normal_xy: [f64; 2],
    pub pressure_pa: f64,
    pub is_windward: bool,
}

/// 3D ENU directional wind streamline / suction wake arrow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindVectorArrow3D {
    pub p0_enu_m: [f64; 3],
    pub p1_enu_m: [f64; 3],
    pub pressure_pa: f64,
    pub is_suction: bool,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::wind_aeroelasticity`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindAeroIfcxOverlay {
    pub schema_version: String,
    pub basic_wind_speed_ms: f64,
    pub wind_azimuth_deg: f64,
    pub exposure: String,
    pub risk_category: String,
    pub enclosure: String,
    pub qh_pa: f64,
    pub gust_factor_gf: f64,
    pub total_base_shear_wind_kn: f64,
    pub seismic_base_shear_ref_kn: f64,
    pub wind_governs_lateral: bool,
    pub overturning_moment_knm: f64,
    pub peak_along_wind_mg: f64,
    pub peak_cross_wind_mg: f64,
    pub resultant_accel_mg: f64,
    pub comfort_limit_mg: f64,
    pub comfort_acceptable: bool,
    pub summary_verdict: String,
}

/// Unified Wind Aeroelasticity & ISO 10137 Comfort 3D Studio Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindStudioReport {
    pub config: WindStudioConfig,
    pub report: WindAnalysisReport,
    pub panels_3d: Vec<FacadePressurePanel3D>,
    pub arrows_3d: Vec<WindVectorArrow3D>,
    pub footprint_center_xy: [f64; 2],
    pub footprint_size_xy: [f64; 2],
    pub base_elev_m: f64,
    pub comfort_limit_mg: f64,
    pub comfort_acceptable: bool,
    pub summary_verdict: String,
}

impl WindStudioReport {
    /// Converts the wind studio report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> WindAeroIfcxOverlay {
        let r = &self.report;
        WindAeroIfcxOverlay {
            schema_version: "wind-aeroelasticity-ifcx-v1".to_string(),
            basic_wind_speed_ms: r.basic_wind_speed_v_ms,
            wind_azimuth_deg: self.config.wind_azimuth_deg,
            exposure: r.exposure_category.label().to_string(),
            risk_category: r.risk_category.label().to_string(),
            enclosure: r.enclosure.label().to_string(),
            qh_pa: r.qh_mean_roof_velocity_pressure_pa,
            gust_factor_gf: r.comfort_audit.gust_effect_factor_g,
            total_base_shear_wind_kn: r.total_base_shear_wind_kn,
            seismic_base_shear_ref_kn: r.seismic_base_shear_ref_kn,
            wind_governs_lateral: r.wind_governs_lateral_system,
            overturning_moment_knm: r.total_overturning_moment_wind_knm,
            peak_along_wind_mg: r.comfort_audit.peak_acceleration_along_wind_mg,
            peak_cross_wind_mg: r.comfort_audit.peak_acceleration_cross_wind_mg,
            resultant_accel_mg: r.comfort_audit.resultant_acceleration_mg,
            comfort_limit_mg: self.comfort_limit_mg,
            comfort_acceptable: self.comfort_acceptable,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Evaluates Wind Aeroelasticity, Facade Cladding Pressure & ISO 10137 Comfort coupled to BIM geometry.
pub fn evaluate_wind_studio(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &WindStudioConfig,
) -> WindStudioReport {
    let mut cfg = config.clone();
    if cfg.basic_wind_speed_ms <= 0.0 {
        cfg.basic_wind_speed_ms = site.map(|s| s.design_wind_ms as f64).unwrap_or(42.0);
    }
    cfg.basic_wind_speed_ms = cfg.basic_wind_speed_ms.clamp(20.0, 90.0);
    cfg.wind_azimuth_deg = cfg.wind_azimuth_deg.rem_euclid(360.0);
    cfg.damping_ratio = cfg.damping_ratio.clamp(0.01, 0.05);

    // Extract building vertical & plan envelope in local ENU meters
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
        .max(7_200.0);

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for c in &bim.columns {
        let x = c.pos.0 * 0.001;
        let y = c.pos.1 * 0.001;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    for w in &bim.walls {
        for p in [w.start, w.end] {
            min_x = min_x.min(p.0 * 0.001);
            max_x = max_x.max(p.0 * 0.001);
            min_y = min_y.min(p.1 * 0.001);
            max_y = max_y.max(p.1 * 0.001);
        }
    }
    if !min_x.is_finite() || (max_x - min_x) < 1.0 || (max_y - min_y) < 1.0 {
        min_x = 0.0;
        max_x = 18.0;
        min_y = 0.0;
        max_y = 14.0;
    }

    let size_x = (max_x - min_x).max(12.0);
    let size_y = (max_y - min_y).max(12.0);
    let cx = 0.5 * (min_x + max_x);
    let cy = 0.5 * (min_y + max_y);
    let base_elev_m = min_elev_mm * 0.001;

    let num_stories = bim.levels.len().saturating_sub(1).clamp(2, 40);
    let height_m = ((max_elev_mm - min_elev_mm) * 0.001).max(num_stories as f64 * 3.6);
    let t1_s = (0.075 * height_m.powf(0.75)).clamp(0.28, 4.50);

    // Flow unit vector: wind blows FROM azimuth_deg clockwise from North (+Y)
    let az_rad = cfg.wind_azimuth_deg.to_radians();
    let flow_dir = [-az_rad.sin(), -az_rad.cos()];
    let b_eff = (size_x * flow_dir[1].abs() + size_y * flow_dir[0].abs()).max(10.0);
    let l_eff = (size_x * flow_dir[0].abs() + size_y * flow_dir[1].abs()).max(10.0);

    // Reference seismic base shear V_s for Wind vs Seismic governing envelope
    let seismic_weight_w_kn =
        ((num_stories as f64) * (size_x * size_y).max(120.0) * 1.3 * 11.5).clamp(4_800.0, 350_000.0);
    let (sms_g, sm1_g) = if let Some(s) = site {
        (
            (s.seismic_pga_ss_g as f64 * 1.15).clamp(0.40, 2.20),
            (s.seismic_pga_s1_g as f64 * 1.20).clamp(0.18, 1.20),
        )
    } else {
        (1.00, 0.50)
    };
    let c_s_fixed = (sm1_g / (t1_s * 6.0)).min(sms_g / 6.0);
    let seismic_vs_kn = (c_s_fixed * seismic_weight_w_kn)
        .max(0.044 * sms_g * seismic_weight_w_kn)
        .max(100.0);

    let mut report = WindEngine::run_wind_analysis(
        &bim.project_name,
        cfg.basic_wind_speed_ms,
        cfg.exposure,
        cfg.risk_category,
        cfg.enclosure,
        height_m,
        b_eff,
        l_eff,
        num_stories,
        t1_s,
        seismic_vs_kn,
    );

    // Refine comfort audit with user-configured damping_ratio and occupancy threshold
    let (g_custom, q_custom, r_custom) = WindEngine::evaluate_gust_factor(
        height_m,
        b_eff,
        cfg.exposure,
        cfg.basic_wind_speed_ms,
        t1_s,
        cfg.damping_ratio,
    );
    let mut comfort = WindEngine::evaluate_comfort_acceleration(
        height_m,
        b_eff,
        l_eff,
        cfg.basic_wind_speed_ms,
        t1_s,
        cfg.damping_ratio,
        g_custom,
        r_custom,
    );
    comfort.background_factor_q = q_custom;
    let comfort_limit_mg = if cfg.occupancy_residential { 15.0 } else { 20.0 };
    let comfort_acceptable = comfort.resultant_acceleration_mg <= comfort_limit_mg;
    comfort.occupant_comfort_threshold_mg = comfort_limit_mg;
    comfort.is_comfort_acceptable = comfort_acceptable;
    comfort.evaluation_verdict = if comfort_acceptable {
        format!(
            "PASSED: Peak acceleration {:.2} mg <= {:.1} mg limit (ISO 10137 {})",
            comfort.resultant_acceleration_mg,
            comfort_limit_mg,
            if cfg.occupancy_residential {
                "Residential"
            } else {
                "Office"
            }
        )
    } else {
        format!(
            "WARNING: Peak acceleration {:.2} mg > {:.1} mg limit (ISO 10137 {}). Consider TMD or outrigger stiffening.",
            comfort.resultant_acceleration_mg,
            comfort_limit_mg,
            if cfg.occupancy_residential {
                "Residential"
            } else {
                "Office"
            }
        )
    };
    report.comfort_audit = comfort;

    // 3D facade pressure panels along 4 orthogonal faces per story
    let story_h = height_m / (num_stories as f64);
    let sidewall_pressure_pa = -(report.qh_mean_roof_velocity_pressure_pa
        * report.comfort_audit.gust_effect_factor_g
        * 0.70);
    let faces = [
        ("East (+X)", [cx + 0.5 * size_x, cy], size_y, [1.0_f64, 0.0]),
        ("West (-X)", [cx - 0.5 * size_x, cy], size_y, [-1.0_f64, 0.0]),
        ("North (+Y)", [cx, cy + 0.5 * size_y], size_x, [0.0_f64, 1.0]),
        ("South (-Y)", [cx, cy - 0.5 * size_y], size_x, [0.0_f64, -1.0]),
    ];

    let mut panels_3d = Vec::with_capacity(num_stories * 4);
    for lvl in &report.story_levels {
        let z_mid = base_elev_m + (lvl.height_z_m - 0.5 * story_h);
        for (fname, fxy, fwidth, normal) in &faces {
            // Wind blows along `flow_dir`; a face is windward if outward normal opposes `flow_dir`
            let align = -(normal[0] * flow_dir[0] + normal[1] * flow_dir[1]);
            let (pressure_pa, is_windward, kind_tag) = if align > 0.35 {
                (lvl.windward_pressure_pa * align, true, "Windward")
            } else if align < -0.35 {
                (-lvl.leeward_pressure_pa * (-align), false, "Leeward")
            } else {
                (sidewall_pressure_pa, false, "Sidewall")
            };
            panels_3d.push(FacadePressurePanel3D {
                story_num: lvl.story_num,
                face_label: format!("{fname} ({kind_tag})"),
                center_enu_m: [fxy[0], fxy[1], z_mid],
                width_m: *fwidth,
                height_m: story_h * 0.88,
                normal_xy: *normal,
                pressure_pa,
                is_windward,
            });
        }
    }

    // 3D wind vector arrows (windward inflow streamlines + leeward suction wake arrows)
    let perp = [-flow_dir[1], flow_dir[0]];
    let stand_off = 0.5 * l_eff + 3.2;
    let arrow_len = 5.4;
    let mut arrows_3d = Vec::new();
    for &lat_off in &[-0.32 * b_eff, 0.0, 0.32 * b_eff] {
        for frac in [0.35_f64, 0.68, 0.96] {
            let z = base_elev_m + height_m * frac;
            let qz_pa = report.qh_mean_roof_velocity_pressure_pa * frac.powf(0.28);
            // Inflow arrow on windward side
            let end_x = cx - flow_dir[0] * stand_off + perp[0] * lat_off;
            let end_y = cy - flow_dir[1] * stand_off + perp[1] * lat_off;
            let start_x = end_x - flow_dir[0] * arrow_len;
            let start_y = end_y - flow_dir[1] * arrow_len;
            arrows_3d.push(WindVectorArrow3D {
                p0_enu_m: [start_x, start_y, z],
                p1_enu_m: [end_x, end_y, z],
                pressure_pa: qz_pa,
                is_suction: false,
            });
            // Leeward suction wake arrow pulling outward behind leeward wall
            let wake_start_x = cx + flow_dir[0] * stand_off + perp[0] * lat_off;
            let wake_start_y = cy + flow_dir[1] * stand_off + perp[1] * lat_off;
            let wake_end_x = wake_start_x + flow_dir[0] * (arrow_len * 0.78);
            let wake_end_y = wake_start_y + flow_dir[1] * (arrow_len * 0.78);
            arrows_3d.push(WindVectorArrow3D {
                p0_enu_m: [wake_start_x, wake_start_y, z],
                p1_enu_m: [wake_end_x, wake_end_y, z],
                pressure_pa: report.qh_mean_roof_velocity_pressure_pa * 0.5,
                is_suction: true,
            });
        }
    }

    let driver = if report.wind_governs_lateral_system {
        "WIND GOVERNS"
    } else {
        "SEISMIC GOVERNS"
    };
    let summary_verdict = format!(
        "V={:.0} m/s ({:.0} deg) | qh={:.0} Pa | Gf={:.2} | Vw={:.0} kN vs Vs={:.0} kN ({}) | ISO 10137 a_res={:.2} mg ({})",
        report.basic_wind_speed_v_ms,
        cfg.wind_azimuth_deg,
        report.qh_mean_roof_velocity_pressure_pa,
        report.comfort_audit.gust_effect_factor_g,
        report.total_base_shear_wind_kn,
        report.seismic_base_shear_ref_kn,
        driver,
        report.comfort_audit.resultant_acceleration_mg,
        if comfort_acceptable { "PASS" } else { "WARN" }
    );

    WindStudioReport {
        config: cfg,
        report,
        panels_3d,
        arrows_3d,
        footprint_center_xy: [cx, cy],
        footprint_size_xy: [size_x, size_y],
        base_elev_m,
        comfort_limit_mg,
        comfort_acceptable,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_2_wind_studio_exposures_azimuth_and_iso10137() {
        let bim = BimProject::sample_bimcraft_building();
        let rep_b = evaluate_wind_studio(
            &bim,
            None,
            &WindStudioConfig {
                basic_wind_speed_ms: 48.0,
                exposure: ExposureCategory::ExposureB,
                wind_azimuth_deg: 270.0,
                occupancy_residential: false,
                ..WindStudioConfig::default()
            },
        );
        let rep_d = evaluate_wind_studio(
            &bim,
            None,
            &WindStudioConfig {
                basic_wind_speed_ms: 48.0,
                exposure: ExposureCategory::ExposureD,
                wind_azimuth_deg: 45.0,
                occupancy_residential: true,
                ..WindStudioConfig::default()
            },
        );
        assert!(!rep_b.panels_3d.is_empty());
        assert!(!rep_b.arrows_3d.is_empty());
        assert!(
            rep_d.report.qh_mean_roof_velocity_pressure_pa
                > rep_b.report.qh_mean_roof_velocity_pressure_pa
        );
        assert!(rep_d.report.total_base_shear_wind_kn > rep_b.report.total_base_shear_wind_kn);
        assert!((rep_d.comfort_limit_mg - 15.0).abs() < 1e-6);
        assert!((rep_b.comfort_limit_mg - 20.0).abs() < 1e-6);
        assert!(rep_d.report.comfort_audit.resultant_acceleration_mg.is_finite());
    }

    #[test]
    fn test_phase84_2_wind_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_wind_studio(&bim, None, &WindStudioConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_wind_studio_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::wind_aeroelasticity"));
        assert!(json.contains("/earth/analysis/wind_aeroelasticity"));
        assert!(!json.contains("ARI"));
    }
}
