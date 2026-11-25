//! core/connection_studio.rs — Update `v0.8.4.4` (`4/10` -> `v0.8.5`):
//! Steel Seismic Connection (`AISC 358-16` / `AISC 360-16` / `SNI 1729 & 7860`) &
//! Bolt/Panel-Zone 3D Studio.
//!
//! Couples active `BimProject` beam-column joints and `SiteEnvironmentalCoupling` seismic
//! demands into [`ConnectionEngine`], generating 3D ENU end-plate / shear-tab joint solids,
//! 3D high-strength bolt pin clusters, Panel Zone doubler/continuity plate checks, and
//! non-destructive `eigengraph::analysis::steel_connection` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::connection::{
    BoltGrade, ConnectionEngine, ConnectionReport, ConnectionType,
};
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;

/// Canonical EigenGraph IFCX attribute namespace for Steel Connection & Panel Zone overlays.
pub const NS_EG_STEEL_CONNECTION: &str = "eigengraph::analysis::steel_connection";

/// Persisted configuration for the Steel Seismic Connection & Panel-Zone 3D Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionStudioConfig {
    pub connection_type: ConnectionType,
    pub bolt_grade: BoltGrade,
    pub bolt_diameter_mm: f64,
    pub bolt_rows: usize,
    pub plate_thickness_mm: f64,
    pub plate_width_mm: f64,
    pub weld_size_mm: f64,
    pub beam_depth_mm: f64,
    pub col_depth_mm: f64,
}

impl Default for ConnectionStudioConfig {
    fn default() -> Self {
        Self {
            connection_type: ConnectionType::ExtendedEndPlate4E,
            bolt_grade: BoltGrade::A490X,
            bolt_diameter_mm: 24.0,
            bolt_rows: 4,
            plate_thickness_mm: 24.0,
            plate_width_mm: 240.0,
            weld_size_mm: 10.0,
            beam_depth_mm: 450.0,
            col_depth_mm: 400.0,
        }
    }
}

/// 3D ENU beam-column joint solid with Panel Zone doubler & continuity status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionJoint3D {
    pub joint_id: String,
    pub pos_enu_m: [f64; 3],
    pub plate_height_m: f64,
    pub plate_width_m: f64,
    pub plate_thick_m: f64,
    pub bolt_count: usize,
    pub governing_dcr: f64,
    pub doubler_required: bool,
    pub is_compliant: bool,
}

/// 3D ENU high-strength bolt pin coordinate on the primary connection plate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoltPin3D {
    pub pos_enu_m: [f64; 3],
    pub diameter_m: f64,
    pub is_tension_zone: bool,
}

/// Compact serializable IFCX overlay payload under `eigengraph::analysis::steel_connection`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SteelConnectionIfcxOverlay {
    pub schema_version: String,
    pub connection_type: String,
    pub bolt_grade: String,
    pub bolt_diameter_mm: f64,
    pub bolt_count_per_joint: usize,
    pub total_joints: usize,
    pub total_bolts_building: usize,
    pub plate_thickness_mm: f64,
    pub applied_moment_mu_knm: f64,
    pub applied_shear_vu_kn: f64,
    pub governing_limit_state: String,
    pub governing_dcr: f64,
    pub panel_zone_dcr: f64,
    pub doubler_plate_required: bool,
    pub recommended_doubler_mm: f64,
    pub is_compliant: bool,
    pub summary_verdict: String,
}

/// Unified Steel Seismic Connection & Panel-Zone 3D Studio Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStudioReport {
    pub config: ConnectionStudioConfig,
    pub report: ConnectionReport,
    pub joints_3d: Vec<ConnectionJoint3D>,
    pub bolts_3d: Vec<BoltPin3D>,
    pub total_bolts_building: usize,
    pub summary_verdict: String,
}

impl ConnectionStudioReport {
    /// Converts the steel connection studio report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> SteelConnectionIfcxOverlay {
        let r = &self.report;
        let (pz_dcr, doubler_req, doubler_mm) = r
            .panel_zone
            .as_ref()
            .map(|pz| {
                (
                    pz.panel_zone_dcr,
                    pz.doubler_plate_required,
                    pz.recommended_doubler_thickness_mm,
                )
            })
            .unwrap_or((0.0, false, 0.0));
        SteelConnectionIfcxOverlay {
            schema_version: "steel-connection-ifcx-v1".to_string(),
            connection_type: r.connection_type.label().to_string(),
            bolt_grade: r.bolt_grade.label().to_string(),
            bolt_diameter_mm: r.bolt_diameter_mm,
            bolt_count_per_joint: r.bolt_count_n,
            total_joints: self.joints_3d.len(),
            total_bolts_building: self.total_bolts_building,
            plate_thickness_mm: r.plate_thickness_mm,
            applied_moment_mu_knm: r.applied_moment_mu_knm,
            applied_shear_vu_kn: r.applied_shear_vu_kn,
            governing_limit_state: r.governing_limit_state.clone(),
            governing_dcr: r.governing_dcr,
            panel_zone_dcr: pz_dcr,
            doubler_plate_required: doubler_req,
            recommended_doubler_mm: doubler_mm,
            is_compliant: r.is_compliant,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Evaluates Steel Seismic Connection limit states, Panel Zone shear, and 3D bolt layouts
/// coupled to live `BimProject` beam-column joints and site seismic demands.
pub fn evaluate_connection_studio(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &ConnectionStudioConfig,
) -> ConnectionStudioReport {
    let mut cfg = config.clone();
    cfg.bolt_diameter_mm = cfg.bolt_diameter_mm.clamp(16.0, 36.0);
    cfg.bolt_rows = cfg.bolt_rows.clamp(2, 8);
    cfg.plate_thickness_mm = cfg.plate_thickness_mm.clamp(10.0, 50.0);
    cfg.plate_width_mm = cfg.plate_width_mm.clamp(140.0, 420.0);
    cfg.weld_size_mm = cfg.weld_size_mm.clamp(6.0, 18.0);
    cfg.beam_depth_mm = cfg.beam_depth_mm.clamp(250.0, 900.0);
    cfg.col_depth_mm = cfg.col_depth_mm.clamp(250.0, 800.0);

    // 1. Collect floor elevations & beam-column joints from BIM
    let mut floor_elevs_m: Vec<f64> = bim
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .filter(|z| *z >= 1.5)
        .collect();
    floor_elevs_m.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    floor_elevs_m.dedup_by(|a, b| (*a - *b).abs() < 0.2);
    if floor_elevs_m.is_empty() {
        floor_elevs_m = vec![3.8, 7.6];
    }

    let mut col_xy: Vec<(String, [f64; 2])> = Vec::new();
    for c in &bim.columns {
        let p = [c.pos.0 * 0.001, c.pos.1 * 0.001];
        if !col_xy
            .iter()
            .any(|(_, q)| (q[0] - p[0]).hypot(q[1] - p[1]) < 0.45)
        {
            col_xy.push((c.name.clone(), p));
        }
    }
    if col_xy.len() < 4 {
        col_xy = vec![
            ("C1".to_string(), [0.0, 0.0]),
            ("C2".to_string(), [6.0, 0.0]),
            ("C3".to_string(), [0.0, 6.0]),
            ("C4".to_string(), [6.0, 6.0]),
        ];
    }

    // Estimate bay span & factored seismic joint demands
    let min_x = col_xy.iter().map(|(_, p)| p[0]).fold(f64::INFINITY, f64::min);
    let max_x = col_xy.iter().map(|(_, p)| p[0]).fold(f64::NEG_INFINITY, f64::max);
    let span_m = ((max_x - min_x).max(6.0) / 2.0).clamp(4.5, 11.0);
    let seismic_fac = site
        .map(|s| (s.seismic_pga_ss_g as f64 * 0.85).clamp(0.55, 1.85))
        .unwrap_or(1.0);
    let vu_kn = (145.0 * (span_m / 6.0) * (0.75 + 0.25 * seismic_fac)).clamp(90.0, 680.0);
    let mu_knm = (210.0 * (span_m / 6.0) * seismic_fac).clamp(110.0, 920.0);
    let pu_kn = 25.0;

    let report = match cfg.connection_type {
        ConnectionType::ShearTabSinglePlate | ConnectionType::DoubleAngleShear => {
            ConnectionEngine::evaluate_shear_tab(
                "Typical Bay Beam-to-Column Shear Tab",
                cfg.bolt_grade,
                cfg.bolt_diameter_mm,
                cfg.bolt_rows,
                cfg.plate_thickness_mm,
                cfg.plate_width_mm.clamp(90.0, 220.0),
                250.0,
                410.0,
                cfg.weld_size_mm,
                vu_kn,
                pu_kn,
            )
        }
        _ => ConnectionEngine::evaluate_extended_end_plate(
            "SMF Beam-to-Column Moment Joint",
            cfg.bolt_grade,
            cfg.bolt_diameter_mm,
            cfg.plate_thickness_mm,
            cfg.plate_width_mm,
            250.0,
            410.0,
            cfg.beam_depth_mm,
            (cfg.beam_depth_mm * 0.42).clamp(150.0, 320.0),
            16.0,
            10.0,
            cfg.col_depth_mm,
            20.0,
            13.0,
            mu_knm,
            vu_kn,
        ),
    };

    let doubler_req = report
        .panel_zone
        .as_ref()
        .is_some_and(|pz| pz.doubler_plate_required);
    let plate_h_m = match cfg.connection_type {
        ConnectionType::ShearTabSinglePlate | ConnectionType::DoubleAngleShear => {
            ((cfg.bolt_rows as f64 - 1.0) * 0.075 + 0.08).clamp(0.22, 0.68)
        }
        _ => ((cfg.beam_depth_mm + 210.0) * 0.001).clamp(0.45, 1.15),
    };
    let plate_w_m = (cfg.plate_width_mm * 0.001).clamp(0.14, 0.42);
    let plate_t_m = (cfg.plate_thickness_mm * 0.001).clamp(0.012, 0.06);

    let mut joints_3d = Vec::new();
    for (fi, &z_m) in floor_elevs_m.iter().take(4).enumerate() {
        for (cid, xy) in col_xy.iter().take(8) {
            if joints_3d.len() >= 16 {
                break;
            }
            joints_3d.push(ConnectionJoint3D {
                joint_id: format!("L{}-{cid}", fi + 1),
                pos_enu_m: [xy[0], xy[1], z_m],
                plate_height_m: plate_h_m,
                plate_width_m: plate_w_m,
                plate_thick_m: plate_t_m,
                bolt_count: report.bolt_count_n,
                governing_dcr: report.governing_dcr,
                doubler_required: doubler_req,
                is_compliant: report.is_compliant,
            });
        }
    }

    // 3D bolt pins around up to 4 primary joints for crisp WebGPU visualization
    let mut bolts_3d = Vec::new();
    let d_bolt_m = (cfg.bolt_diameter_mm * 0.001).clamp(0.016, 0.04);
    let rows = report.bolt_rows.clamp(2, 8);
    let cols = report.bolt_cols.clamp(1, 2);
    for j in joints_3d.iter().take(4) {
        for r_idx in 0..rows {
            let z_frac = if rows == 1 {
                0.0
            } else {
                (r_idx as f64) / ((rows - 1) as f64) - 0.5
            };
            let dz = z_frac * (plate_h_m * 0.78);
            let is_tension = r_idx < rows / 2;
            for c_idx in 0..cols {
                let dy = if cols == 1 {
                    0.0
                } else {
                    (c_idx as f64 - 0.5) * (plate_w_m * 0.56)
                };
                bolts_3d.push(BoltPin3D {
                    pos_enu_m: [
                        j.pos_enu_m[0] + 0.5 * (cfg.col_depth_mm * 0.001) + plate_t_m * 0.6,
                        j.pos_enu_m[1] + dy,
                        j.pos_enu_m[2] - 0.5 * (cfg.beam_depth_mm * 0.001) + dz,
                    ],
                    diameter_m: d_bolt_m,
                    is_tension_zone: is_tension,
                });
            }
        }
    }

    let total_bolts_building = joints_3d.len() * report.bolt_count_n;
    let pz_note = report
        .panel_zone
        .as_ref()
        .map(|pz| {
            if pz.doubler_plate_required {
                format!(
                    " | PZ DCR={:.2} (Doubler {:.0}mm)",
                    pz.panel_zone_dcr, pz.recommended_doubler_thickness_mm
                )
            } else {
                format!(" | PZ DCR={:.2}", pz.panel_zone_dcr)
            }
        })
        .unwrap_or_default();
    let short_ty = match cfg.connection_type {
        ConnectionType::ShearTabSinglePlate | ConnectionType::DoubleAngleShear => "Shear Tab",
        _ => "4E End-Plate",
    };
    let summary_verdict = format!(
        "{short_ty} ({}x M{:.0}): Gov '{}' DCR={:.2}{} | {} Joints ({} Bolts) [{}]",
        report.bolt_count_n,
        report.bolt_diameter_mm,
        report.governing_limit_state,
        report.governing_dcr,
        pz_note,
        joints_3d.len(),
        total_bolts_building,
        if report.is_compliant { "PASS" } else { "REVIEW" }
    );

    ConnectionStudioReport {
        config: cfg,
        report,
        joints_3d,
        bolts_3d,
        total_bolts_building,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase84_4_connection_studio_endplate_and_shear_tab() {
        let bim = BimProject::sample_bimcraft_building();
        let rep_ep = evaluate_connection_studio(
            &bim,
            None,
            &ConnectionStudioConfig {
                connection_type: ConnectionType::ExtendedEndPlate4E,
                bolt_diameter_mm: 27.0,
                plate_thickness_mm: 28.0,
                ..ConnectionStudioConfig::default()
            },
        );
        assert!(!rep_ep.joints_3d.is_empty());
        assert!(!rep_ep.bolts_3d.is_empty());
        assert!(rep_ep.report.panel_zone.is_some());
        assert!(rep_ep.report.governing_dcr.is_finite());

        let rep_st = evaluate_connection_studio(
            &bim,
            None,
            &ConnectionStudioConfig {
                connection_type: ConnectionType::ShearTabSinglePlate,
                bolt_rows: 4,
                plate_thickness_mm: 14.0,
                ..ConnectionStudioConfig::default()
            },
        );
        assert!(!rep_st.report.limit_states.is_empty());
        assert!(rep_st.report.panel_zone.is_none());
        assert!(rep_st.total_bolts_building >= 16);
    }

    #[test]
    fn test_phase84_4_connection_ifcx_overlay_roundtrip() {
        let bim = BimProject::sample_bimcraft_building();
        let rep = evaluate_connection_studio(&bim, None, &ConnectionStudioConfig::default());
        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_connection_overlay(&base, &rep.to_ifcx_overlay());
        let json = composed.to_json_pretty();
        assert!(json.contains("eigengraph::analysis::steel_connection"));
        assert!(json.contains("/earth/analysis/steel_connection"));
        assert!(!json.contains("ARI"));
    }
}
