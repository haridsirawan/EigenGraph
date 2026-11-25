//! core/composite_studio.rs — Update `v0.8.4.6` (`6/10` -> `v0.8.5`):
//! Composite Steel-Concrete Deck, Headed Shear Stud Layout & AISC DG11 Floor Vibration
//! 3D Studio (`AISC 360-16 Ch. I` / `AISC Design Guide 11` / `SNI 1729:2020`).
//!
//! Couples live `BimProject` floor bays/elevations and `SiteEnvironmentalCoupling` into
//! [`CompositeBeamEngine::evaluate_composite_beam`], computing headed shear stud capacities (`Qn`),
//! Plastic Neutral Axis (`PNA`), unshored wet concrete construction safety, AISC DG11 walking
//! vibration (`f_n`, `%g` acceleration), 3D ENU slab strips & stud pin arrays, and non-destructive
//! `eigengraph::analysis::composite_deck` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::composite::{
    CompositeBeamEngine, CompositeBeamReport, CompositeSlabDeck, DeckOrientation, ShearStud,
};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;

/// Canonical EigenGraph IFCX attribute namespace for Composite Steel-Concrete Deck overlays.
pub const NS_EG_COMPOSITE_DECK: &str = "eigengraph::analysis::composite_deck";

/// Persisted configuration for the Composite Deck, Shear Stud & DG11 Vibration 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeStudioConfig {
    pub deck_orientation: DeckOrientation,
    pub studs_per_rib: usize,
    pub studs_provided: usize,
    pub stud_diameter_mm: f64,
    pub slab_fc_mpa: f64,
    pub total_slab_h_mm: f64,
    pub steel_d_mm: f64,
    pub unshored: bool,
    pub floor_damping_beta: f64,
    pub live_load_kn_m: f64,
}

impl Default for CompositeStudioConfig {
    fn default() -> Self {
        Self {
            deck_orientation: DeckOrientation::Perpendicular,
            studs_per_rib: 1,
            studs_provided: 40,
            stud_diameter_mm: 19.0,
            slab_fc_mpa: 30.0,
            total_slab_h_mm: 160.0,
            steel_d_mm: 450.0,
            unshored: true,
            floor_damping_beta: 0.03,
            live_load_kn_m: 14.0,
        }
    }
}

/// AISC Design Guide 11 Walking Excitation Floor Vibration Assessment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorVibrationDg11 {
    pub fundamental_freq_fn_hz: f64,
    pub peak_accel_pct_g: f64,
    pub tolerance_limit_pct_g: f64,
    pub damping_ratio: f64,
    pub is_vibration_ok: bool,
}

/// 3D ENU composite floor deck strip anchored between structural supports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeBay3D {
    pub bay_id: String,
    pub p0_enu_m: [f64; 3],
    pub p1_enu_m: [f64; 3],
    pub eff_width_m: f64,
    pub slab_thick_m: f64,
    pub degree_pct: f64,
    pub is_compliant: bool,
}

/// 3D ENU headed shear stud pin positioned along the composite beam top flange.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShearStudPin3D {
    pub pos_enu_m: [f64; 3],
    pub height_m: f64,
    pub is_double_rib: bool,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::composite_deck`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeDeckIfcxOverlay {
    pub schema_version: String,
    pub span_length_m: f64,
    pub deck_orientation: String,
    pub studs_provided: usize,
    pub total_studs_building: usize,
    pub degree_of_composite_action_pct: f64,
    pub pna_location: String,
    pub flexure_dcr: f64,
    pub construction_stage_dcr: f64,
    pub live_load_deflection_mm: f64,
    pub dg11_freq_hz: f64,
    pub dg11_accel_pct_g: f64,
    pub dg11_vibration_ok: bool,
    pub is_overall_compliant: bool,
    pub summary_verdict: String,
}

/// Unified Composite Steel-Concrete Deck, Shear Studs & DG11 Vibration Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeStudioReport {
    pub config: CompositeStudioConfig,
    pub report: CompositeBeamReport,
    pub vibration: FloorVibrationDg11,
    pub bays_3d: Vec<CompositeBay3D>,
    pub studs_3d: Vec<ShearStudPin3D>,
    pub total_studs_building: usize,
    pub summary_verdict: String,
}

impl CompositeStudioReport {
    /// Converts the composite deck studio report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> CompositeDeckIfcxOverlay {
        let r = &self.report;
        let v = &self.vibration;
        CompositeDeckIfcxOverlay {
            schema_version: "composite-deck-ifcx-v1".to_string(),
            span_length_m: r.span_length_l_m,
            deck_orientation: self.config.deck_orientation.label().to_string(),
            studs_provided: r.total_studs_provided,
            total_studs_building: self.total_studs_building,
            degree_of_composite_action_pct: r.degree_of_composite_action_pct,
            pna_location: r.pna_location.label().to_string(),
            flexure_dcr: r.flexure_dcr,
            construction_stage_dcr: r.construction_stage_dcr,
            live_load_deflection_mm: r.live_load_deflection_delta_ll_mm,
            dg11_freq_hz: v.fundamental_freq_fn_hz,
            dg11_accel_pct_g: v.peak_accel_pct_g,
            dg11_vibration_ok: v.is_vibration_ok,
            is_overall_compliant: r.is_overall_compliant && v.is_vibration_ok,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Evaluates Composite Steel-Concrete Deck, Headed Shear Studs, Construction Stage Safety,
/// and AISC DG11 Floor Vibration coupled to live `BimProject` bays.
pub fn evaluate_composite_studio(
    bim: &BimProject,
    _site: Option<&SiteEnvironmentalCoupling>,
    config: &CompositeStudioConfig,
) -> CompositeStudioReport {
    let mut cfg = config.clone();
    cfg.studs_per_rib = cfg.studs_per_rib.clamp(1, 2);
    cfg.studs_provided = cfg.studs_provided.clamp(12, 160);
    cfg.stud_diameter_mm = cfg.stud_diameter_mm.clamp(16.0, 22.0);
    cfg.slab_fc_mpa = cfg.slab_fc_mpa.clamp(20.0, 60.0);
    cfg.total_slab_h_mm = cfg.total_slab_h_mm.clamp(110.0, 250.0);
    cfg.steel_d_mm = cfg.steel_d_mm.clamp(250.0, 800.0);
    cfg.floor_damping_beta = cfg.floor_damping_beta.clamp(0.015, 0.060);
    cfg.live_load_kn_m = cfg.live_load_kn_m.clamp(4.0, 40.0);

    // 1. Extract floor bay span & spacing from BIM
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
    if !min_x.is_finite() || (max_x - min_x) < 3.0 {
        min_x = 0.0;
        max_x = 12.0;
        min_y = 0.0;
        max_y = 9.0;
    }
    let span_l_m = ((max_x - min_x) * 0.75).clamp(6.0, 18.0);
    let cy = 0.5 * (min_y + max_y);
    let spacing_s_m = ((max_y - min_y).max(6.0) * 0.36).clamp(2.2, 4.5);

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

    let hr_mm = 75.0_f64;
    let tc_mm = (cfg.total_slab_h_mm - hr_mm).max(50.0);
    let slab = CompositeSlabDeck {
        deck_rib_height_hr_mm: hr_mm,
        deck_rib_width_wr_mm: 150.0,
        slab_thickness_above_deck_tc_mm: tc_mm,
        total_slab_thickness_h_mm: hr_mm + tc_mm,
        concrete_fc_mpa: cfg.slab_fc_mpa,
        concrete_density_kg_m3: 2400.0,
        deck_orientation: cfg.deck_orientation,
        studs_per_rib: cfg.studs_per_rib,
    };
    let stud = ShearStud {
        diameter_ds_mm: cfg.stud_diameter_mm,
        length_hs_mm: (hr_mm + 40.0).max(100.0),
        tensile_strength_fu_mpa: 450.0,
    };

    // Scale W-section proportions from steel_d_mm
    let steel_bf = (cfg.steel_d_mm * 0.44).clamp(160.0, 320.0);
    let steel_tf = (cfg.steel_d_mm * 0.036).clamp(11.0, 28.0);
    let steel_tw = (cfg.steel_d_mm * 0.022).clamp(7.5, 18.0);
    let steel_area_mm2 = 2.0 * steel_bf * steel_tf + (cfg.steel_d_mm - 2.0 * steel_tf) * steel_tw;
    let hw = (cfg.steel_d_mm - 2.0 * steel_tf).max(100.0);
    let steel_ix_mm4 = (steel_tw * hw.powi(3)) / 12.0
        + 2.0
            * ((steel_bf * steel_tf.powi(3)) / 12.0
                + steel_bf * steel_tf * (0.5 * (hw + steel_tf)).powi(2));
    let steel_ix_cm4 = steel_ix_mm4 / 1e4;
    let steel_zx_cm3 = (steel_bf * steel_tf * (hw + steel_tf) + steel_tw * hw * hw * 0.25) / 1e3;

    let w_dl_kn_m = (spacing_s_m * ((slab.total_slab_thickness_h_mm * 0.001) * 24.0 + 1.5)) + 0.85;
    let w_u = 1.2 * w_dl_kn_m + 1.6 * cfg.live_load_kn_m;
    let applied_mu_knm = (w_u * span_l_m * span_l_m / 8.0).max(80.0);

    let report = CompositeBeamEngine::evaluate_composite_beam(
        &bim.project_name,
        span_l_m,
        spacing_s_m,
        &format!("WF {:.0}x{:.0}", cfg.steel_d_mm, steel_bf),
        cfg.steel_d_mm,
        steel_bf,
        steel_tf,
        steel_tw,
        steel_area_mm2,
        345.0,
        steel_ix_cm4,
        steel_zx_cm3,
        &slab,
        &stud,
        cfg.studs_provided,
        cfg.unshored,
        applied_mu_knm,
        cfg.live_load_kn_m,
    );

    // 2. AISC Design Guide 11 Walking Excitation Vibration Check
    let delta_dl_mm = (report.live_load_deflection_delta_ll_mm
        * (w_dl_kn_m / cfg.live_load_kn_m.max(1.0)))
    .clamp(1.2, 42.0);
    let delta_j_m = (delta_dl_mm * 0.85 * 0.001).clamp(0.0015, 0.060);
    let fundamental_freq_fn_hz = (0.18 * (9.81 / delta_j_m).sqrt()).clamp(2.0, 18.0);
    let panel_weight_kn = (w_dl_kn_m * span_l_m * 2.2).max(120.0);
    let p0_kn = 0.29_f64;
    let peak_accel_pct_g =
        ((p0_kn * (-0.35 * fundamental_freq_fn_hz).exp()) / (cfg.floor_damping_beta * panel_weight_kn))
            * 100.0;
    let tolerance_limit_pct_g = 0.50; // AISC DG11 Office / Residential ISO curve baseline
    let is_vibration_ok = peak_accel_pct_g <= tolerance_limit_pct_g;
    let vibration = FloorVibrationDg11 {
        fundamental_freq_fn_hz,
        peak_accel_pct_g,
        tolerance_limit_pct_g,
        damping_ratio: cfg.floor_damping_beta,
        is_vibration_ok,
    };

    // 3. 3D Composite Floor Bays & Headed Shear Stud Array
    let eff_width_m = (report.effective_slab_width_beff_mm * 0.001).clamp(1.2, 4.5);
    let slab_thick_m = (slab.total_slab_thickness_h_mm * 0.001).clamp(0.11, 0.25);
    let overall_ok = report.is_overall_compliant && is_vibration_ok;

    let mut bays_3d = Vec::new();
    let mut studs_3d = Vec::new();
    for (ei, &z_m) in elevs_m.iter().take(2).enumerate() {
        for (yi, dy) in [-0.5 * spacing_s_m, 0.5 * spacing_s_m].iter().enumerate() {
            let y_m = cy + *dy;
            let p0 = [min_x, y_m, z_m];
            let p1 = [min_x + span_l_m, y_m, z_m];
            bays_3d.push(CompositeBay3D {
                bay_id: format!("L{}-COMP{}", ei + 1, yi + 1),
                p0_enu_m: p0,
                p1_enu_m: p1,
                eff_width_m,
                slab_thick_m,
                degree_pct: report.degree_of_composite_action_pct,
                is_compliant: overall_ok,
            });
            // Place representative stud pins along first floor bays
            if ei == 0 {
                let sample_n = (cfg.studs_provided / 2).clamp(6, 20);
                for si in 0..sample_n {
                    let t = (si as f64 + 0.5) / (sample_n as f64);
                    studs_3d.push(ShearStudPin3D {
                        pos_enu_m: [p0[0] + (p1[0] - p0[0]) * t, y_m, z_m + 0.02],
                        height_m: stud.length_hs_mm * 0.001,
                        is_double_rib: cfg.studs_per_rib > 1,
                    });
                }
            }
        }
    }

    let total_studs_building = bays_3d.len() * cfg.studs_provided;
    let summary_verdict = format!(
        "Composite ({:.0}% action, {} studs/bay): Flex DCR={:.2} | Const DCR={:.2} | DG11 fn={:.2} Hz ({:.2}%g <= 0.50%g) [{}]",
        report.degree_of_composite_action_pct,
        report.total_studs_provided,
        report.flexure_dcr,
        report.construction_stage_dcr,
        vibration.fundamental_freq_fn_hz,
        vibration.peak_accel_pct_g,
        if overall_ok { "PASS" } else { "REVIEW" }
    );

    CompositeStudioReport {
        config: cfg,
        report,
        vibration,
        bays_3d,
        studs_3d,
        total_studs_building,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_6_composite_studio_studs_pna_and_dg11_vibration() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_composite_studio(
            &bim,
            None,
            &CompositeStudioConfig {
                studs_provided: 48,
                studs_per_rib: 2,
                deck_orientation: DeckOrientation::Perpendicular,
                ..CompositeStudioConfig::default()
            },
        );
        assert!(!rep.bays_3d.is_empty());
        assert!(!rep.studs_3d.is_empty());
        assert!(rep.report.degree_of_composite_action_pct >= 25.0);
        assert!(rep.vibration.fundamental_freq_fn_hz > 2.0);
        assert!(rep.vibration.peak_accel_pct_g.is_finite());
        assert!(rep.total_studs_building >= 48);
    }

    #[test]
    fn test_phase84_6_composite_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_composite_studio(&bim, None, &CompositeStudioConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_composite_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::composite_deck"));
        assert!(json.contains("/earth/analysis/composite_deck"));
        assert!(!json.contains("ARI"));
    }
}
