//! clash_engine.rs — Update `v0.8.7.6` (`6/10` -> `v0.8.8`):
//! Unified 3D Spatial Clash Detection & BCF Coordination Guardrail (`Structure × MEP × Openings`),
//! Automatic Plenum Clearance Solver & IFCX Overlay (`eigengraph::coordination::clash_bcf`).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::bim::BimProject;
use crate::core::mep_engine::{MepConduitSegment3D, MepSystemsReport};

/// Canonical EigenGraph IFCX attribute namespace for Unified 3D Clash & BCF Guardrails.
pub const NS_EG_CLASH_GUARDRAIL: &str = "eigengraph::coordination::clash_bcf";

/// Severity tier for spatial interference & code clearance checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpatialClashSeverity {
    HardClash,
    ClearanceWarning,
    SleeveAdvisory,
}

impl SpatialClashSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::HardClash => "Hard Clash (Physical Penetration)",
            Self::ClearanceWarning => "Clearance Tolerance Warning",
            Self::SleeveAdvisory => "Slab / Wall Core Sleeve Advisory",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::HardClash => "HARD",
            Self::ClearanceWarning => "WARN",
            Self::SleeveAdvisory => "SLEEVE",
        }
    }

    pub fn rgba(self) -> [f32; 4] {
        match self {
            Self::HardClash => [0.96, 0.22, 0.24, 0.94],
            Self::ClearanceWarning => [0.98, 0.68, 0.14, 0.90],
            Self::SleeveAdvisory => [0.22, 0.85, 0.96, 0.85],
        }
    }

    pub fn severity_norm(self) -> f32 {
        match self {
            Self::HardClash => 1.0,
            Self::ClearanceWarning => 0.6,
            Self::SleeveAdvisory => 0.25,
        }
    }
}

/// Discipline pair classification for a detected 3D interference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpatialClashPairKind {
    MepVsBeam,
    MepVsColumn,
    MepVsWall,
    MepVsMep,
    MepVsSlabSleeve,
}

impl SpatialClashPairKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::MepVsBeam => "MEP × Structural Beam",
            Self::MepVsColumn => "MEP × Structural Column",
            Self::MepVsWall => "MEP × Architectural/Core Wall",
            Self::MepVsMep => "MEP × MEP Cross-Service",
            Self::MepVsSlabSleeve => "MEP Riser × Floor Slab Sleeve",
        }
    }
}

/// Configuration parameters for unified 3D Structure × MEP × Openings clash detection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialClashConfig {
    pub clearance_tolerance_mm: f64,
    pub check_structure_vs_mep: bool,
    pub check_mep_vs_mep: bool,
    pub include_sleeve_advisories: bool,
    pub show_hard_only: bool,
}

impl Default for SpatialClashConfig {
    fn default() -> Self {
        Self {
            clearance_tolerance_mm: 50.0,
            check_structure_vs_mep: true,
            check_mep_vs_mep: true,
            include_sleeve_advisories: true,
            show_hard_only: false,
        }
    }
}

/// Single 3D spatial interference / BCF 2.1 issue pin in local RTC ENU coordinates (meters).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialClashRecord3D {
    pub id: usize,
    pub bcf_guid: String,
    pub severity: SpatialClashSeverity,
    pub pair_kind: SpatialClashPairKind,
    pub story_index: usize,
    pub element_a_tag: String,
    pub element_b_tag: String,
    pub center_enu_m: [f64; 3],
    pub overlap_size_enu_m: [f64; 3],
    pub penetration_mm: f64,
    pub clearance_gap_mm: f64,
    pub suggested_delta_z_m: f64,
    pub recommended_action: String,
}

/// Compact IFCX attribute payload exported under `eigengraph::coordination::clash_bcf`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialClashIfcxOverlay {
    pub site_label: String,
    pub total_clashes: usize,
    pub hard_clashes: usize,
    pub clearance_warnings: usize,
    pub sleeve_advisories: usize,
    pub max_penetration_mm: f64,
    pub recommended_plenum_drop_m: f64,
    pub coordination_pass: bool,
}

/// Full 3D Spatial Clash Detection & BCF Guardrail Report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialClashReport {
    pub config: SpatialClashConfig,
    pub site_label: String,
    pub total_checked_pairs: usize,
    pub hard_clashes: usize,
    pub clearance_warnings: usize,
    pub sleeve_advisories: usize,
    pub max_penetration_mm: f64,
    pub recommended_plenum_drop_m: f64,
    pub coordination_pass: bool,
    pub clashes: Vec<SpatialClashRecord3D>,
    pub summary_verdict: String,
}

impl SpatialClashReport {
    /// Builds a compact IFCX attribute overlay for `IfcxStage::compose_with_clash_guardrail_overlay`.
    pub fn to_ifcx_overlay(&self) -> SpatialClashIfcxOverlay {
        SpatialClashIfcxOverlay {
            site_label: self.site_label.clone(),
            total_clashes: self.clashes.len(),
            hard_clashes: self.hard_clashes,
            clearance_warnings: self.clearance_warnings,
            sleeve_advisories: self.sleeve_advisories,
            max_penetration_mm: self.max_penetration_mm,
            recommended_plenum_drop_m: self.recommended_plenum_drop_m,
            coordination_pass: self.coordination_pass,
        }
    }

    /// Exports detected issues as structured BCF 2.1 topic JSON records.
    pub fn to_bcf_topics_json(&self) -> Value {
        let topics: Vec<Value> = self
            .clashes
            .iter()
            .map(|c| {
                json!({
                    "guid": c.bcf_guid,
                    "topic_type": "Clash",
                    "topic_status": "Open",
                    "priority": if c.severity == SpatialClashSeverity::HardClash { "Critical" } else { "Normal" },
                    "title": format!("[{}] {} vs {}", c.severity.short_label(), c.element_a_tag, c.element_b_tag),
                    "description": c.recommended_action,
                    "penetration_mm": c.penetration_mm,
                    "viewpoint_enu_m": c.center_enu_m,
                })
            })
            .collect();
        json!({
            "bcf_version": "2.1",
            "site_label": self.site_label,
            "coordination_pass": self.coordination_pass,
            "topics": topics
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct Aabb3D {
    min: [f64; 3],
    max: [f64; 3],
}

impl Aabb3D {
    fn from_segment(p0: [f64; 3], p1: [f64; 3], half_xy: f64, half_z: f64) -> Self {
        Self {
            min: [
                p0[0].min(p1[0]) - half_xy,
                p0[1].min(p1[1]) - half_xy,
                p0[2].min(p1[2]) - half_z,
            ],
            max: [
                p0[0].max(p1[0]) + half_xy,
                p0[1].max(p1[1]) + half_xy,
                p0[2].max(p1[2]) + half_z,
            ],
        }
    }

    /// Returns `(center_enu_m, overlap_size_m, min_penetration_m, clearance_gap_m)` if within `tol_m`.
    fn check_interference(&self, other: &Self, tol_m: f64) -> Option<([f64; 3], [f64; 3], f64, f64)> {
        let mut ov_min = [0.0_f64; 3];
        let mut ov_max = [0.0_f64; 3];
        let mut raw_overlap = [0.0_f64; 3];
        for i in 0..3 {
            let a_min = self.min[i];
            let a_max = self.max[i];
            let b_min = other.min[i];
            let b_max = other.max[i];
            let lo = a_min.max(b_min);
            let hi = a_max.min(b_max);
            raw_overlap[i] = hi - lo;
            if raw_overlap[i] < -tol_m {
                return None;
            }
            ov_min[i] = lo.min(hi);
            ov_max[i] = lo.max(hi);
        }
        let center = [
            0.5 * (ov_min[0] + ov_max[0]),
            0.5 * (ov_min[1] + ov_max[1]),
            0.5 * (ov_min[2] + ov_max[2]),
        ];
        let size = [
            (ov_max[0] - ov_min[0]).max(0.18),
            (ov_max[1] - ov_min[1]).max(0.18),
            (ov_max[2] - ov_min[2]).max(0.18),
        ];
        let min_ov = raw_overlap[0].min(raw_overlap[1]).min(raw_overlap[2]);
        if min_ov > 0.002 {
            Some((center, size, min_ov, 0.0))
        } else {
            Some((center, size, 0.0, (-min_ov).max(0.0)))
        }
    }
}

fn conduit_aabb(seg: &MepConduitSegment3D) -> Aabb3D {
    let hw = (seg.width_m * 0.5).max(0.05);
    let hh = (seg.height_m * 0.5).max(0.05);
    if seg.is_riser {
        Aabb3D::from_segment(seg.start_enu_m, seg.end_enu_m, hw.max(hh), 0.02)
    } else {
        Aabb3D::from_segment(seg.start_enu_m, seg.end_enu_m, hw, hh)
    }
}

fn level_elev_m(bim: &BimProject, level_id: usize) -> f64 {
    bim.levels
        .iter()
        .find(|l| l.id == level_id)
        .map(|l| l.elevation_mm * 0.001)
        .unwrap_or(0.0)
}

fn wall_opening_clears_point(
    wall: &crate::core::bim::BimWall,
    z_base_m: f64,
    pt_enu_m: [f64; 3],
    half_h_m: f64,
) -> bool {
    if wall.openings.is_empty() {
        return false;
    }
    let sx = wall.start.0 * 0.001;
    let sy = wall.start.1 * 0.001;
    let ex = wall.end.0 * 0.001;
    let ey = wall.end.1 * 0.001;
    let dx = ex - sx;
    let dy = ey - sy;
    let len = dx.hypot(dy).max(1e-6);
    let ux = dx / len;
    let uy = dy / len;
    let along_m = (pt_enu_m[0] - sx) * ux + (pt_enu_m[1] - sy) * uy;

    for op in &wall.openings {
        let op_start = op.offset_along_mm * 0.001;
        let op_end = op_start + op.width_mm * 0.001;
        let op_z0 = z_base_m + op.sill_height_mm * 0.001;
        let op_z1 = op_z0 + op.height_mm * 0.001;
        if along_m >= op_start + 0.05
            && along_m <= op_end - 0.05
            && (pt_enu_m[2] - half_h_m) >= op_z0
            && (pt_enu_m[2] + half_h_m) <= op_z1
        {
            return true;
        }
    }
    false
}

/// Evaluates unified 3D Structure × MEP × Openings clashes and computes an automated
/// ceiling-plenum soffit clearance recommendation.
pub fn evaluate_spatial_clashes(
    bim: &BimProject,
    mep: &MepSystemsReport,
    config: &SpatialClashConfig,
) -> SpatialClashReport {
    let mut cfg = config.clone();
    cfg.clearance_tolerance_mm = cfg.clearance_tolerance_mm.clamp(10.0, 250.0);
    let tol_m = cfg.clearance_tolerance_mm * 0.001;

    let mut clashes = Vec::new();
    let mut checked_pairs = 0_usize;
    let mut next_id = 1_usize;
    let mut recommended_drop_m = mep.config.plenum_drop_below_slab_m.max(0.35);

    if cfg.check_structure_vs_mep {
        // 1. MEP Conduits vs Structural Beams (`BimRun`)
        for seg in &mep.segments_3d {
            if seg.is_riser {
                continue;
            }
            let s_box = conduit_aabb(seg);
            for run in &bim.runs {
                checked_pairs += 1;
                let z_top = level_elev_m(bim, run.level_id);
                let beam_depth_m = (run.depth_mm * 0.001).clamp(0.18, 1.8);
                let beam_hw = (run.width_mm * 0.5 * 0.001).max(0.10);
                let p0 = [run.start.0 * 0.001, run.start.1 * 0.001, z_top - beam_depth_m];
                let p1 = [run.end.0 * 0.001, run.end.1 * 0.001, z_top];
                let b_box = Aabb3D {
                    min: [
                        p0[0].min(p1[0]) - beam_hw,
                        p0[1].min(p1[1]) - beam_hw,
                        z_top - beam_depth_m,
                    ],
                    max: [
                        p0[0].max(p1[0]) + beam_hw,
                        p0[1].max(p1[1]) + beam_hw,
                        z_top,
                    ],
                };
                if let Some((center, size, pen_m, gap_m)) = s_box.check_interference(&b_box, tol_m) {
                    let req_drop = beam_depth_m + seg.height_m * 0.5 + tol_m + 0.04;
                    recommended_drop_m = recommended_drop_m.max(req_drop);
                    let sev = if pen_m > 0.0 {
                        SpatialClashSeverity::HardClash
                    } else {
                        SpatialClashSeverity::ClearanceWarning
                    };
                    let delta_z = (z_top - beam_depth_m - seg.height_m * 0.5 - tol_m - seg.start_enu_m[2])
                        .min(-0.05);
                    clashes.push(SpatialClashRecord3D {
                        id: next_id,
                        bcf_guid: format!("EG-BCF-L{}-{:04}", seg.story_index + 1, next_id),
                        severity: sev,
                        pair_kind: SpatialClashPairKind::MepVsBeam,
                        story_index: seg.story_index,
                        element_a_tag: seg.tag.clone(),
                        element_b_tag: format!("Beam #{} ({})", run.id, run.profile_name),
                        center_enu_m: center,
                        overlap_size_enu_m: size,
                        penetration_mm: (pen_m * 1000.0).round(),
                        clearance_gap_mm: (gap_m * 1000.0).round(),
                        suggested_delta_z_m: delta_z,
                        recommended_action: format!(
                            "Lower ceiling plenum drop to {:.2} m below slab soffit to clear beam ({:.0} mm depth).",
                            req_drop, run.depth_mm
                        ),
                    });
                    next_id += 1;
                }
            }

            // 2. MEP Conduits vs Structural Columns (`BimColumn`)
            for col in &bim.columns {
                checked_pairs += 1;
                let z0 = level_elev_m(bim, col.base_level_id);
                let z1 = level_elev_m(bim, col.top_level_id).max(z0 + 2.8);
                let hw = (col.width_mm * 0.5 * 0.001).max(0.12);
                let hd = (col.depth_mm * 0.5 * 0.001).max(0.12);
                let cx = col.pos.0 * 0.001;
                let cy = col.pos.1 * 0.001;
                let c_box = Aabb3D {
                    min: [cx - hw, cy - hd, z0],
                    max: [cx + hw, cy + hd, z1],
                };
                if let Some((center, size, pen_m, gap_m)) = s_box.check_interference(&c_box, tol_m) {
                    let sev = if pen_m > 0.0 {
                        SpatialClashSeverity::HardClash
                    } else {
                        SpatialClashSeverity::ClearanceWarning
                    };
                    clashes.push(SpatialClashRecord3D {
                        id: next_id,
                        bcf_guid: format!("EG-BCF-L{}-{:04}", seg.story_index + 1, next_id),
                        severity: sev,
                        pair_kind: SpatialClashPairKind::MepVsColumn,
                        story_index: seg.story_index,
                        element_a_tag: seg.tag.clone(),
                        element_b_tag: format!("Column #{} ({})", col.id, col.profile_name),
                        center_enu_m: center,
                        overlap_size_enu_m: size,
                        penetration_mm: (pen_m * 1000.0).round(),
                        clearance_gap_mm: (gap_m * 1000.0).round(),
                        suggested_delta_z_m: 0.0,
                        recommended_action: format!(
                            "Offset {} horizontally by {:.2} m to maintain {:.0} mm column clearance.",
                            seg.tag,
                            hw + seg.width_m * 0.5 + tol_m,
                            cfg.clearance_tolerance_mm
                        ),
                    });
                    next_id += 1;
                }
            }

            // 3. MEP Conduits vs Walls (`BimWall`, respecting hosted Doors/Windows)
            for wall in &bim.walls {
                checked_pairs += 1;
                let z0 = level_elev_m(bim, wall.base_level_id);
                let z1 = wall
                    .top_level_id
                    .map(|id| level_elev_m(bim, id))
                    .unwrap_or(z0 + wall.unconnected_height_mm * 0.001)
                    .max(z0 + 2.4);
                let ht = (wall.thickness_mm * 0.5 * 0.001).max(0.08);
                let wx0 = wall.start.0 * 0.001;
                let wy0 = wall.start.1 * 0.001;
                let wx1 = wall.end.0 * 0.001;
                let wy1 = wall.end.1 * 0.001;
                let w_box = Aabb3D {
                    min: [wx0.min(wx1) - ht, wy0.min(wy1) - ht, z0],
                    max: [wx0.max(wx1) + ht, wy0.max(wy1) + ht, z1],
                };
                if let Some((center, size, pen_m, gap_m)) = s_box.check_interference(&w_box, tol_m) {
                    if wall_opening_clears_point(wall, z0, center, seg.height_m * 0.5) {
                        continue;
                    }
                    let sev = if pen_m > 0.0 {
                        SpatialClashSeverity::HardClash
                    } else {
                        SpatialClashSeverity::ClearanceWarning
                    };
                    clashes.push(SpatialClashRecord3D {
                        id: next_id,
                        bcf_guid: format!("EG-BCF-L{}-{:04}", seg.story_index + 1, next_id),
                        severity: sev,
                        pair_kind: SpatialClashPairKind::MepVsWall,
                        story_index: seg.story_index,
                        element_a_tag: seg.tag.clone(),
                        element_b_tag: format!("Wall #{} ({})", wall.id, wall.name),
                        center_enu_m: center,
                        overlap_size_enu_m: size,
                        penetration_mm: (pen_m * 1000.0).round(),
                        clearance_gap_mm: (gap_m * 1000.0).round(),
                        suggested_delta_z_m: 0.0,
                        recommended_action: format!(
                            "Add fire-rated wall sleeve ({:.0}×{:.0} mm) through Wall #{}.",
                            (seg.width_m * 1000.0 + 100.0),
                            (seg.height_m * 1000.0 + 100.0),
                            wall.id
                        ),
                    });
                    next_id += 1;
                }
            }
        }
    }

    // 4. Cross-Discipline MEP × MEP Interference Checks
    if cfg.check_mep_vs_mep {
        let segs = &mep.segments_3d;
        for i in 0..segs.len() {
            let a = &segs[i];
            let a_box = conduit_aabb(a);
            for b in segs.iter().skip(i + 1) {
                if a.discipline == b.discipline || (a.is_riser && b.is_riser) {
                    continue;
                }
                checked_pairs += 1;
                let b_box = conduit_aabb(b);
                if let Some((center, size, pen_m, gap_m)) = a_box.check_interference(&b_box, tol_m * 0.5) {
                    let sev = if pen_m > 0.0 {
                        SpatialClashSeverity::HardClash
                    } else {
                        SpatialClashSeverity::ClearanceWarning
                    };
                    clashes.push(SpatialClashRecord3D {
                        id: next_id,
                        bcf_guid: format!("EG-BCF-MEP-{:04}", next_id),
                        severity: sev,
                        pair_kind: SpatialClashPairKind::MepVsMep,
                        story_index: a.story_index.max(b.story_index),
                        element_a_tag: a.tag.clone(),
                        element_b_tag: b.tag.clone(),
                        center_enu_m: center,
                        overlap_size_enu_m: size,
                        penetration_mm: (pen_m * 1000.0).round(),
                        clearance_gap_mm: (gap_m * 1000.0).round(),
                        suggested_delta_z_m: -0.18,
                        recommended_action: format!(
                            "Stagger {} vs {} elevation by 180 mm inside ceiling plenum.",
                            a.tag, b.tag
                        ),
                    });
                    next_id += 1;
                }
            }
        }
    }

    // 5. Core Riser Slab Penetration Sleeve Advisories
    if cfg.include_sleeve_advisories {
        let mut lvl_elevs: Vec<f64> = bim.levels.iter().map(|l| l.elevation_mm * 0.001).collect();
        lvl_elevs.sort_by(|a, b| a.total_cmp(b));
        lvl_elevs.dedup_by(|a, b| (*a - *b).abs() < 0.2);
        for seg in mep.segments_3d.iter().filter(|s| s.is_riser) {
            for (li, &ez) in lvl_elevs.iter().enumerate().skip(1) {
                checked_pairs += 1;
                if ez >= seg.start_enu_m[2] && ez <= seg.end_enu_m[2] {
                    clashes.push(SpatialClashRecord3D {
                        id: next_id,
                        bcf_guid: format!("EG-SLV-L{}-{:04}", li + 1, next_id),
                        severity: SpatialClashSeverity::SleeveAdvisory,
                        pair_kind: SpatialClashPairKind::MepVsSlabSleeve,
                        story_index: li,
                        element_a_tag: seg.tag.clone(),
                        element_b_tag: format!("Level {} Slab", li + 1),
                        center_enu_m: [seg.start_enu_m[0], seg.start_enu_m[1], ez],
                        overlap_size_enu_m: [
                            (seg.width_m + 0.12).max(0.24),
                            (seg.height_m + 0.12).max(0.24),
                            0.24,
                        ],
                        penetration_mm: 0.0,
                        clearance_gap_mm: 0.0,
                        suggested_delta_z_m: 0.0,
                        recommended_action: format!(
                            "Coordinate firestop slab sleeve ({:.0}×{:.0} mm) at Level {} shaft.",
                            (seg.width_m * 1000.0 + 100.0),
                            (seg.height_m * 1000.0 + 100.0),
                            li + 1
                        ),
                    });
                    next_id += 1;
                }
            }
        }
    }

    let hard_clashes = clashes
        .iter()
        .filter(|c| c.severity == SpatialClashSeverity::HardClash)
        .count();
    let clearance_warnings = clashes
        .iter()
        .filter(|c| c.severity == SpatialClashSeverity::ClearanceWarning)
        .count();
    let sleeve_advisories = clashes
        .iter()
        .filter(|c| c.severity == SpatialClashSeverity::SleeveAdvisory)
        .count();
    let max_penetration_mm = clashes
        .iter()
        .map(|c| c.penetration_mm)
        .fold(0.0_f64, f64::max);
    let recommended_plenum_drop_m = recommended_drop_m.clamp(0.35, 1.05);
    let coordination_pass = hard_clashes == 0;

    let summary_verdict = format!(
        "Clash Guardrail: {} Hard • {} Clearance Warn • {} Sleeve Adv ({} pairs checked, max pen {:.0} mm) [{}]",
        hard_clashes,
        clearance_warnings,
        sleeve_advisories,
        checked_pairs,
        max_penetration_mm,
        if coordination_pass { "CLEAR" } else { "ACTION REQUIRED" }
    );

    SpatialClashReport {
        config: cfg,
        site_label: mep.site_label.clone(),
        total_checked_pairs: checked_pairs,
        hard_clashes,
        clearance_warnings,
        sleeve_advisories,
        max_penetration_mm,
        recommended_plenum_drop_m,
        coordination_pass,
        clashes,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};
    use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
    use crate::core::mep_engine::{evaluate_mep_systems, MepSystemsConfig};

    #[test]
    fn test_v0876_clash_detection_structure_mep_auto_resolve_and_ifcx_overlay() {
        let bim = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();

        // Force a tight ceiling plenum drop so MEP ducts intersect structural beams
        let mut mep_cfg = MepSystemsConfig::default();
        mep_cfg.plenum_drop_below_slab_m = 0.25;
        let mep_shallow = evaluate_mep_systems(&bim, Some(&site), None, &mep_cfg);
        let clash_rep = evaluate_spatial_clashes(&bim, &mep_shallow, &SpatialClashConfig::default());

        assert!(clash_rep.total_checked_pairs > 10);
        assert!(
            clash_rep
                .clashes
                .iter()
                .any(|c| c.pair_kind == SpatialClashPairKind::MepVsBeam),
            "Expected shallow 0.25m plenum drop to trigger MepVsBeam clash against sample beams"
        );
        assert!(clash_rep.recommended_plenum_drop_m > 0.25);

        // Apply recommended plenum drop -> beam clashes must drop strictly
        mep_cfg.plenum_drop_below_slab_m = clash_rep.recommended_plenum_drop_m;
        let mep_fixed = evaluate_mep_systems(&bim, Some(&site), None, &mep_cfg);
        let fixed_rep = evaluate_spatial_clashes(&bim, &mep_fixed, &SpatialClashConfig::default());
        let beam_before = clash_rep
            .clashes
            .iter()
            .filter(|c| c.pair_kind == SpatialClashPairKind::MepVsBeam)
            .count();
        let beam_after = fixed_rep
            .clashes
            .iter()
            .filter(|c| c.pair_kind == SpatialClashPairKind::MepVsBeam)
            .count();
        assert!(
            beam_after < beam_before,
            "Auto-resolve plenum drop ({:.2}m) should reduce MepVsBeam clashes ({beam_after} < {beam_before})",
            clash_rep.recommended_plenum_drop_m
        );

        let bcf = clash_rep.to_bcf_topics_json();
        assert_eq!(bcf["bcf_version"], "2.1");
        assert!(!bcf["topics"].as_array().expect("topics").is_empty());

        let base_doc = IfcxDocument::new("clash_test");
        let ov_doc =
            IfcxStage::compose_with_clash_guardrail_overlay(&base_doc, &clash_rep.to_ifcx_overlay());
        let json = ov_doc.to_json_pretty();
        assert!(json.contains(NS_EG_CLASH_GUARDRAIL));
        assert!(json.contains("hardClashes"));
    }
}
