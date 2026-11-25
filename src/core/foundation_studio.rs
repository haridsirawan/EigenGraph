//! core/foundation_studio.rs — Update `v0.8.4.8` (`8/10` -> `v0.8.5`):
//! Deep Foundation Pile Group, 6-DOF Winkler Springs & Two-Way Punching Shear 3D Studio
//! (`ACI 318-19 §22.6` / `SNI 8460:2017` / `SNI 2847:2019`).
//!
//! Couples live `BimProject` footings/columns/levels and `SiteEnvironmentalCoupling` seismic
//! hazard into [`GeotechEngine::evaluate_pile_group`], [`GeotechEngine::compute_winkler_springs`],
//! [`GeotechEngine::audit_punching_shear`], and [`GeotechEngine::evaluate_ssi_modal_shift`],
//! generating 3D ENU pile caps, deep pile shafts, Winkler spring nodes, and non-destructive
//! `eigengraph::analysis::deep_foundation` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::geotech::{
    GeotechEngine, PileGroupReport, PunchingShearAudit, SoilLayer, SoilType, SsiModalShiftReport,
    WinklerSpringMatrix,
};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;

/// Canonical EigenGraph IFCX attribute namespace for Deep Foundation & Pile Group overlays.
pub const NS_EG_DEEP_FOUNDATION: &str = "eigengraph::analysis::deep_foundation";

/// Persisted configuration for the Deep Foundation Pile Group, Winkler Springs & Punching Shear 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundationStudioConfig {
    pub soil_type: SoilType,
    pub piles_x: usize,
    pub piles_y: usize,
    pub pile_diameter_m: f64,
    pub pile_spacing_m: f64,
    pub pile_length_m: f64,
    pub cap_fc_mpa: f64,
    pub cap_thickness_m: f64,
    pub column_pu_kn: f64,
    pub column_mux_knm: f64,
    pub column_muy_knm: f64,
    pub include_ssi_shift: bool,
}

impl Default for FoundationStudioConfig {
    fn default() -> Self {
        Self {
            soil_type: SoilType::MediumSand,
            piles_x: 2,
            piles_y: 2,
            pile_diameter_m: 0.80,
            pile_spacing_m: 2.40,
            pile_length_m: 18.0,
            cap_fc_mpa: 35.0,
            cap_thickness_m: 1.20,
            column_pu_kn: 3200.0,
            column_mux_knm: 480.0,
            column_muy_knm: 360.0,
            include_ssi_shift: true,
        }
    }
}

/// 3D ENU reinforced concrete pile cap block with punching shear DCR status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PileCap3D {
    pub cap_id: String,
    pub center_enu_m: [f64; 3],
    pub width_m: f64,
    pub length_m: f64,
    pub thickness_m: f64,
    pub punching_dcr: f64,
    pub is_compliant: bool,
}

/// 3D ENU deep foundation pile shaft from pile cap soffit to bearing toe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PileShaft3D {
    pub pile_id: String,
    pub top_enu_m: [f64; 3],
    pub tip_enu_m: [f64; 3],
    pub diameter_m: f64,
    pub axial_load_kn: f64,
    pub axial_dcr: f64,
    pub is_uplift: bool,
    pub is_safe: bool,
}

/// 3D ENU equivalent 6-DOF Winkler subgrade reaction spring anchor node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WinklerSpringNode3D {
    pub pos_enu_m: [f64; 3],
    pub kz_kn_m: f64,
    pub kx_kn_m: f64,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::deep_foundation`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeepFoundationIfcxOverlay {
    pub schema_version: String,
    pub soil_type: String,
    pub pile_layout: String,
    pub total_piles_per_cap: usize,
    pub total_piles_building: usize,
    pub pile_diameter_m: f64,
    pub pile_length_m: f64,
    pub converse_labarre_efficiency: f64,
    pub max_pile_load_kn: f64,
    pub min_pile_load_kn: f64,
    pub pile_governing_dcr: f64,
    pub punching_stress_mpa: f64,
    pub punching_capacity_mpa: f64,
    pub punching_dcr: f64,
    pub winkler_kz_kn_m: f64,
    pub ssi_period_ratio: f64,
    pub total_concrete_m3: f64,
    pub is_overall_compliant: bool,
    pub summary_verdict: String,
}

/// Unified Deep Foundation Pile Group, 6-DOF Winkler Springs & Punching Shear Studio Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundationStudioReport {
    pub config: FoundationStudioConfig,
    pub soil_layer: SoilLayer,
    pub winkler_springs: WinklerSpringMatrix,
    pub pile_group: PileGroupReport,
    pub punching_shear: PunchingShearAudit,
    pub ssi_modal_shift: SsiModalShiftReport,
    pub caps_3d: Vec<PileCap3D>,
    pub piles_3d: Vec<PileShaft3D>,
    pub springs_3d: Vec<WinklerSpringNode3D>,
    pub total_piles_building: usize,
    pub total_concrete_m3: f64,
    pub is_overall_compliant: bool,
    pub summary_verdict: String,
}

impl FoundationStudioReport {
    /// Converts the deep foundation report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> DeepFoundationIfcxOverlay {
        DeepFoundationIfcxOverlay {
            schema_version: "deep-foundation-ifcx-v1".to_string(),
            soil_type: self.soil_layer.soil_type.label().to_string(),
            pile_layout: format!("{}x{}", self.pile_group.piles_count_x, self.pile_group.piles_count_y),
            total_piles_per_cap: self.pile_group.total_piles,
            total_piles_building: self.total_piles_building,
            pile_diameter_m: self.config.pile_diameter_m,
            pile_length_m: self.config.pile_length_m,
            converse_labarre_efficiency: self.pile_group.converse_labarre_efficiency,
            max_pile_load_kn: self.pile_group.max_pile_axial_load_kn,
            min_pile_load_kn: self.pile_group.min_pile_axial_load_kn,
            pile_governing_dcr: self.pile_group.governing_dcr,
            punching_stress_mpa: self.punching_shear.punching_stress_vu_mpa,
            punching_capacity_mpa: self.punching_shear.concrete_capacity_phi_vc_mpa,
            punching_dcr: self.punching_shear.punching_dcr,
            winkler_kz_kn_m: self.winkler_springs.kz_trans_kn_m,
            ssi_period_ratio: if self.config.include_ssi_shift {
                self.ssi_modal_shift.period_lengthening_ratio
            } else {
                1.0
            },
            total_concrete_m3: self.total_concrete_m3,
            is_overall_compliant: self.is_overall_compliant,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Evaluates Deep Foundation Pile Group, Converse-Labarre Efficiency, 6-DOF Winkler Springs,
/// Two-Way Punching Shear, and Wolf/Veletsos-Meek SSI Shift coupled to live `BimProject`.
pub fn evaluate_foundation_studio(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &FoundationStudioConfig,
) -> FoundationStudioReport {
    let mut cfg = config.clone();
    cfg.piles_x = cfg.piles_x.clamp(1, 5);
    cfg.piles_y = cfg.piles_y.clamp(1, 5);
    cfg.pile_diameter_m = cfg.pile_diameter_m.clamp(0.35, 1.80);
    cfg.pile_spacing_m = cfg.pile_spacing_m.clamp(cfg.pile_diameter_m * 2.5, 5.00);
    cfg.pile_length_m = cfg.pile_length_m.clamp(6.0, 48.0);
    cfg.cap_fc_mpa = cfg.cap_fc_mpa.clamp(20.0, 65.0);
    cfg.cap_thickness_m = cfg.cap_thickness_m.clamp(0.60, 3.20);
    cfg.column_pu_kn = cfg.column_pu_kn.clamp(400.0, 24_000.0);
    cfg.column_mux_knm = cfg.column_mux_knm.clamp(0.0, 6_000.0);
    cfg.column_muy_knm = cfg.column_muy_knm.clamp(0.0, 6_000.0);

    let soil_layer = SoilLayer::new_standard(cfg.soil_type, 0.0, cfg.pile_length_m + 6.0);
    let ss_g = site
        .map(|s| f64::from(s.seismic_pga_ss_g).clamp(0.15, 2.0))
        .unwrap_or(0.65);
    let eff_mux = cfg.column_mux_knm * (0.85 + 0.25 * ss_g);
    let eff_muy = cfg.column_muy_knm * (0.85 + 0.25 * ss_g);

    let mut pile_group = GeotechEngine::evaluate_pile_group(
        1,
        1,
        cfg.piles_x,
        cfg.piles_y,
        cfg.pile_diameter_m,
        cfg.pile_spacing_m,
        cfg.pile_length_m,
        cfg.column_pu_kn,
        eff_mux,
        eff_muy,
        &soil_layer,
    );
    let cap_h = cfg.cap_thickness_m.max(pile_group.cap_thickness_m);
    pile_group.cap_thickness_m = cap_h;

    let winkler_springs = GeotechEngine::compute_winkler_springs(
        pile_group.cap_width_m,
        pile_group.cap_length_m,
        &soil_layer,
    );

    let (c1_mm, c2_mm) = bim
        .columns
        .first()
        .map(|c| (c.width_mm.clamp(250.0, 1500.0), c.depth_mm.clamp(250.0, 1500.0)))
        .unwrap_or((600.0, 600.0));
    let d_eff_mm = (cap_h * 1000.0 - 150.0).max(350.0);
    let punching_shear = GeotechEngine::audit_punching_shear(
        "Governing Pile Cap C1",
        c1_mm,
        c2_mm,
        d_eff_mm,
        cfg.cap_fc_mpa,
        cfg.column_pu_kn,
        eff_mux.hypot(eff_muy),
    );

    let max_elev_m = bim
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .fold(0.0_f64, f64::max);
    let h_bldg_m = if max_elev_m >= 3.0 { max_elev_m } else { 14.0 };
    let t_fixed = (0.048 * h_bldg_m.powf(0.75)).clamp(0.25, 4.5);
    let ssi_modal_shift =
        GeotechEngine::evaluate_ssi_modal_shift(t_fixed, h_bldg_m, 45_000.0, &winkler_springs);

    // Extract representative support positions in ENU (meters) from BIM footings or unique columns
    let mut centers_xy: Vec<[f64; 2]> = if !bim.footings.is_empty() {
        bim.footings
            .iter()
            .take(6)
            .map(|f| [f.pos.0 * 0.001, f.pos.1 * 0.001])
            .collect()
    } else {
        let mut pts: Vec<[f64; 2]> = Vec::new();
        for c in &bim.columns {
            let p = [c.pos.0 * 0.001, c.pos.1 * 0.001];
            if !pts
                .iter()
                .any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < 0.5)
            {
                pts.push(p);
                if pts.len() >= 6 {
                    break;
                }
            }
        }
        pts
    };
    if centers_xy.is_empty() {
        centers_xy = vec![[0.0, 0.0], [8.0, 0.0], [8.0, 6.0], [0.0, 6.0]];
    }

    let is_overall_compliant = pile_group.is_group_safe && punching_shear.is_punching_safe;
    let mut caps_3d = Vec::with_capacity(centers_xy.len());
    let mut piles_3d = Vec::with_capacity(centers_xy.len() * pile_group.total_piles);
    let mut springs_3d = Vec::with_capacity(centers_xy.len() * 4);

    let hw = pile_group.cap_width_m * 0.45;
    let hl = pile_group.cap_length_m * 0.45;
    for (ci, [cx, cy]) in centers_xy.iter().enumerate() {
        caps_3d.push(PileCap3D {
            cap_id: format!("PC-{}", ci + 1),
            center_enu_m: [*cx, *cy, -0.5 * cap_h],
            width_m: pile_group.cap_width_m,
            length_m: pile_group.cap_length_m,
            thickness_m: cap_h,
            punching_dcr: punching_shear.punching_dcr,
            is_compliant: punching_shear.is_punching_safe,
        });
        for p in &pile_group.pile_details {
            piles_3d.push(PileShaft3D {
                pile_id: format!("PC{}-P{:02}", ci + 1, p.pile_id),
                top_enu_m: [cx + p.x_coord_m, cy + p.y_coord_m, -cap_h],
                tip_enu_m: [cx + p.x_coord_m, cy + p.y_coord_m, -cap_h - cfg.pile_length_m],
                diameter_m: cfg.pile_diameter_m,
                axial_load_kn: p.factored_axial_demand_pu_kn,
                axial_dcr: p.axial_dcr,
                is_uplift: p.factored_axial_demand_pu_kn < 0.0,
                is_safe: p.is_safe,
            });
        }
        for [sx, sy] in [[-hw, -hl], [hw, -hl], [hw, hl], [-hw, hl]] {
            springs_3d.push(WinklerSpringNode3D {
                pos_enu_m: [cx + sx, cy + sy, -cap_h - 0.15],
                kz_kn_m: winkler_springs.kz_trans_kn_m * 0.25,
                kx_kn_m: winkler_springs.kx_trans_kn_m * 0.25,
            });
        }
    }

    let total_piles_building = caps_3d.len() * pile_group.total_piles;
    let cap_vol = pile_group.cap_width_m * pile_group.cap_length_m * cap_h;
    let single_pile_vol =
        std::f64::consts::FRAC_PI_4 * cfg.pile_diameter_m * cfg.pile_diameter_m * cfg.pile_length_m;
    let total_concrete_m3 =
        (caps_3d.len() as f64) * cap_vol + (total_piles_building as f64) * single_pile_vol;

    let summary_verdict = format!(
        "{}x{} D{:.0}mm L={:.0}m ({} piles total): Eg={:.2} | Pile DCR={:.2} | Punch DCR={:.2} | Kz={:.0} MN/m | SSI T~={:.2}s [{}]",
        pile_group.piles_count_x,
        pile_group.piles_count_y,
        cfg.pile_diameter_m * 1000.0,
        cfg.pile_length_m,
        total_piles_building,
        pile_group.converse_labarre_efficiency,
        pile_group.governing_dcr,
        punching_shear.punching_dcr,
        winkler_springs.kz_trans_kn_m / 1000.0,
        ssi_modal_shift.flexible_ssi_period_t_tilde_s,
        if is_overall_compliant { "PASS" } else { "REVIEW" }
    );

    FoundationStudioReport {
        config: cfg,
        soil_layer,
        winkler_springs,
        pile_group,
        punching_shear,
        ssi_modal_shift,
        caps_3d,
        piles_3d,
        springs_3d,
        total_piles_building,
        total_concrete_m3,
        is_overall_compliant,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_8_foundation_pile_group_winkler_and_punching() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_foundation_studio(
            &bim,
            None,
            &FoundationStudioConfig {
                soil_type: SoilType::SoftClay,
                piles_x: 3,
                piles_y: 3,
                pile_length_m: 24.0,
                ..FoundationStudioConfig::default()
            },
        );
        assert!(!rep.caps_3d.is_empty());
        assert!(!rep.piles_3d.is_empty());
        assert!(!rep.springs_3d.is_empty());
        assert!(rep.pile_group.converse_labarre_efficiency > 0.5);
        assert!(rep.pile_group.converse_labarre_efficiency <= 1.0);
        assert!(rep.winkler_springs.kz_trans_kn_m > 0.0);
        assert!(rep.punching_shear.critical_perimeter_bo_mm > 1000.0);
        assert!(rep.total_concrete_m3 > 1.0);
    }

    #[test]
    fn test_phase84_8_foundation_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_foundation_studio(&bim, None, &FoundationStudioConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_foundation_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::deep_foundation"));
        assert!(json.contains("/earth/analysis/deep_foundation"));
        assert!(!json.contains("ARI"));
    }
}
