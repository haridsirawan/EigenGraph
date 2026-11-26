//! core/massing_agent.rs — Update `v0.8.5.8` (`8/10` -> `v0.8.6`):
//! Multi-Option Architectural Massing Explorer & 1-Click Mass-to-Engineered-BIM.
//!
//! Couples `SiteZoningReport`, `SpatialProgramReport`, and `SiteEnvironmentalCoupling`
//! into four structurally & architecturally distinct 3D massing options
//! (`PodiumTower`, `CourtyardBlock`, `LinearSlabBar`, `SteppedTerrace`), evaluates
//! live zoning compliance, daylighting, `EigenGate` redundancy index (`ARI`), DCR%,
//! 5D cost, and 6D embodied carbon, and synthesizes a complete parametric `BimProject`
//! (levels, structural grids, columns, beams, shear-wall core, compound walls with
//! openings, slabs, and pad footings) on 1-click adoption.

use serde::{Deserialize, Serialize};

use crate::core::bim::{
    BimColumn, BimFooting, BimGrid, BimLevel, BimOpening, BimProject, BimRun, BimSlab, BimWall,
    OpeningKind, RunCategory, RunJustify, ViewRange,
};
use crate::core::geom::P2;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::program_agent::SpatialProgramReport;
use crate::core::zoning_agent::SiteZoningReport;

/// Canonical EigenGraph IFCX attribute namespace for Multi-Option Massing Explorer overlays.
pub const NS_EG_MASSING_STUDY: &str = "eigengraph::design::massing_study";

/// Architectural massing archetypes evaluated in the Snaptrude-style Massing Explorer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MassingArchetype {
    PodiumTower,
    CourtyardBlock,
    LinearSlabBar,
    SteppedTerrace,
}

impl MassingArchetype {
    pub const ALL: [Self; 4] = [
        Self::PodiumTower,
        Self::CourtyardBlock,
        Self::LinearSlabBar,
        Self::SteppedTerrace,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::PodiumTower => "Podium + Central Tower",
            Self::CourtyardBlock => "Courtyard Perimeter Block",
            Self::LinearSlabBar => "Linear Double-Loaded Slab",
            Self::SteppedTerrace => "Stepped Biophilic Terrace",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::PodiumTower => "Podium-Tower",
            Self::CourtyardBlock => "Courtyard",
            Self::LinearSlabBar => "Bar/Slab",
            Self::SteppedTerrace => "Stepped",
        }
    }

    pub fn accent_rgba(self) -> [f32; 4] {
        match self {
            Self::PodiumTower => [0.22, 0.74, 0.97, 0.62],
            Self::CourtyardBlock => [0.13, 0.77, 0.56, 0.62],
            Self::LinearSlabBar => [0.96, 0.68, 0.16, 0.62],
            Self::SteppedTerrace => [0.66, 0.52, 0.98, 0.62],
        }
    }
}

/// Primary lateral-force & gravity framing system applied during Mass-to-BIM conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MassingStructuralSystem {
    RcMomentCore,
    CompositeSteelCore,
    DiagridHybrid,
}

impl MassingStructuralSystem {
    pub const ALL: [Self; 3] = [
        Self::RcMomentCore,
        Self::CompositeSteelCore,
        Self::DiagridHybrid,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::RcMomentCore => "RC Moment Frame + Shear Core",
            Self::CompositeSteelCore => "Composite Steel WF + RC Core",
            Self::DiagridHybrid => "Composite Perimeter + Braced Core",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::RcMomentCore => "RC Core",
            Self::CompositeSteelCore => "Composite",
            Self::DiagridHybrid => "Diagrid",
        }
    }
}

/// Persisted configuration for Multi-Option Massing Explorer & Mass-to-BIM Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MassingStudyConfig {
    pub selected_archetype: MassingArchetype,
    pub structural_system: MassingStructuralSystem,
    pub bay_x_m: f64,
    pub bay_y_m: f64,
    pub core_wall_thick_mm: f64,
    pub facade_wwr_pct: f64,
    pub include_basement_footings: bool,
    pub show_structural_grid_preview: bool,
}

impl Default for MassingStudyConfig {
    fn default() -> Self {
        Self {
            selected_archetype: MassingArchetype::PodiumTower,
            structural_system: MassingStructuralSystem::RcMomentCore,
            bay_x_m: 7.2,
            bay_y_m: 7.2,
            core_wall_thick_mm: 300.0,
            facade_wwr_pct: 42.0,
            include_basement_footings: true,
            show_structural_grid_preview: true,
        }
    }
}

/// 3D ENU massing volume block belonging to a specific archetype tier or structural core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MassingVolumeBlock3D {
    pub label: String,
    pub tier_index: usize,
    pub is_core: bool,
    pub is_courtyard_void: bool,
    pub start_story: usize,
    pub end_story: usize,
    pub min_enu_m: [f64; 3],
    pub max_enu_m: [f64; 3],
    pub color_rgba: [f32; 4],
}

/// Evaluated feasibility, structural redundancy, 5D cost & 6D carbon metrics for one option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MassingOptionMetrics {
    pub archetype: MassingArchetype,
    pub stories: usize,
    pub podium_stories: usize,
    pub total_height_m: f64,
    pub footprint_area_m2: f64,
    pub gfa_m2: f64,
    pub nfa_m2: f64,
    pub efficiency_pct: f64,
    pub kdb_pct: f64,
    pub klb_far: f64,
    pub daylight_score_pct: f64,
    pub estimated_eigengate_ari: f64,
    pub estimated_max_dcr_pct: f64,
    pub estimated_cost_usd: f64,
    pub estimated_cost_idr_billion: f64,
    pub embodied_carbon_tco2e: f64,
    pub carbon_intensity_kg_m2: f64,
    pub zoning_compliant: bool,
    pub program_fit_pct: f64,
    pub composite_score: f64,
    pub blocks_3d: Vec<MassingVolumeBlock3D>,
    pub courtyard_hole_enu_m: Option<([f64; 2], [f64; 2])>,
    pub rationale: String,
}

/// Compact serializable IFCX overlay payload under `eigengraph::design::massing_study`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MassingIfcxOverlay {
    pub schema_version: String,
    pub selected_archetype: String,
    pub structural_system: String,
    pub bay_x_m: f64,
    pub bay_y_m: f64,
    pub selected_gfa_m2: f64,
    pub selected_stories: usize,
    pub selected_height_m: f64,
    pub selected_eigengate_ari: f64,
    pub selected_max_dcr_pct: f64,
    pub selected_cost_usd: f64,
    pub selected_carbon_tco2e: f64,
    pub recommended_archetype: String,
    pub option_count: usize,
    pub summary_verdict: String,
}

/// Complete comparative Massing Explorer report across 4 architectural archetypes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassingStudyReport {
    pub config: MassingStudyConfig,
    pub site_label: String,
    pub story_height_m: f64,
    pub target_gfa_m2: f64,
    pub zoning_max_gfa_m2: f64,
    pub lot_area_m2: f64,
    pub buildable_pad_width_m: f64,
    pub buildable_pad_depth_m: f64,
    pub options: Vec<MassingOptionMetrics>,
    pub selected_index: usize,
    pub recommended_index: usize,
    pub summary_verdict: String,
}

impl MassingStudyReport {
    pub fn active_option(&self) -> Option<&MassingOptionMetrics> {
        self.options
            .get(self.selected_index)
            .or_else(|| self.options.first())
    }

    pub fn recommended_option(&self) -> Option<&MassingOptionMetrics> {
        self.options
            .get(self.recommended_index)
            .or_else(|| self.options.first())
    }

    pub fn to_ifcx_overlay(&self) -> MassingIfcxOverlay {
        let active = self.active_option();
        let rec = self.recommended_option();
        MassingIfcxOverlay {
            schema_version: "massing-study-ifcx-v1".to_string(),
            selected_archetype: active
                .map(|o| o.archetype.label().to_string())
                .unwrap_or_else(|| self.config.selected_archetype.label().to_string()),
            structural_system: self.config.structural_system.label().to_string(),
            bay_x_m: self.config.bay_x_m,
            bay_y_m: self.config.bay_y_m,
            selected_gfa_m2: active.map_or(0.0, |o| o.gfa_m2),
            selected_stories: active.map_or(0, |o| o.stories),
            selected_height_m: active.map_or(0.0, |o| o.total_height_m),
            selected_eigengate_ari: active.map_or(0.0, |o| o.estimated_eigengate_ari),
            selected_max_dcr_pct: active.map_or(0.0, |o| o.estimated_max_dcr_pct),
            selected_cost_usd: active.map_or(0.0, |o| o.estimated_cost_usd),
            selected_carbon_tco2e: active.map_or(0.0, |o| o.embodied_carbon_tco2e),
            recommended_archetype: rec
                .map(|o| o.archetype.label().to_string())
                .unwrap_or_else(|| "Podium + Central Tower".to_string()),
            option_count: self.options.len(),
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

/// Summary returned after converting an active massing option into a full `BimProject`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassToBimSummary {
    pub archetype_label: String,
    pub levels: usize,
    pub grids: usize,
    pub columns: usize,
    pub beams: usize,
    pub walls: usize,
    pub openings: usize,
    pub slabs: usize,
    pub footings: usize,
    pub gfa_m2: f64,
    pub summary_line: String,
}

fn snap_span_to_bays(raw_span_m: f64, bay_m: f64, min_bays: usize, max_bays: usize) -> f64 {
    let bay = bay_m.clamp(4.0, 12.0);
    let raw_bays = (raw_span_m / bay).floor().max(min_bays as f64) as usize;
    let clamped = raw_bays.clamp(min_bays, max_bays);
    (clamped as f64) * bay
}

fn add_tier_block(
    blocks: &mut Vec<MassingVolumeBlock3D>,
    label: &str,
    tier_index: usize,
    start_story: usize,
    end_story: usize,
    story_h: f64,
    min_xy: [f64; 2],
    max_xy: [f64; 2],
    color_rgba: [f32; 4],
) {
    blocks.push(MassingVolumeBlock3D {
        label: label.to_string(),
        tier_index,
        is_core: false,
        is_courtyard_void: false,
        start_story,
        end_story,
        min_enu_m: [min_xy[0], min_xy[1], (start_story as f64) * story_h],
        max_enu_m: [max_xy[0], max_xy[1], (end_story as f64) * story_h],
        color_rgba,
    });
}

fn add_core_block(
    blocks: &mut Vec<MassingVolumeBlock3D>,
    stories: usize,
    story_h: f64,
    core_w: f64,
    core_d: f64,
    center_x: f64,
    center_y: f64,
) {
    blocks.push(MassingVolumeBlock3D {
        label: "Shear-Wall Core".to_string(),
        tier_index: 99,
        is_core: true,
        is_courtyard_void: false,
        start_story: 0,
        end_story: stories,
        min_enu_m: [center_x - core_w * 0.5, center_y - core_d * 0.5, 0.0],
        max_enu_m: [
            center_x + core_w * 0.5,
            center_y + core_d * 0.5,
            (stories as f64) * story_h + 1.2,
        ],
        color_rgba: [0.82, 0.89, 0.96, 0.82],
    });
}

fn evaluate_single_option(
    archetype: MassingArchetype,
    zoning: &SiteZoningReport,
    program: &SpatialProgramReport,
    site: &SiteEnvironmentalCoupling,
    config: &MassingStudyConfig,
) -> MassingOptionMetrics {
    let bay_x = config.bay_x_m.clamp(4.8, 10.8);
    let bay_y = config.bay_y_m.clamp(4.8, 10.8);
    let story_h = zoning.config.typical_story_height_m.clamp(2.8, 6.0);
    let lot_area = zoning.lot_area_m2.max(150.0);
    let pad_w = zoning.buildable_pad_width_m.max(bay_x * 2.0);
    let pad_d = zoning.buildable_pad_depth_m.max(bay_y * 2.0);
    let max_bays_x = ((pad_w / bay_x).floor() as usize).clamp(2, 12);
    let max_bays_y = ((pad_d / bay_y).floor() as usize).clamp(2, 12);
    let max_stories_zoning = zoning.max_allowable_stories.clamp(2, 60);
    let target_gfa = program.total_gfa_m2.max(600.0);
    let eff_ratio = (program.overall_efficiency_pct / 100.0).clamp(0.62, 0.86);

    let full_w = snap_span_to_bays(pad_w, bay_x, 2, max_bays_x);
    let full_d = snap_span_to_bays(pad_d, bay_y, 2, max_bays_y);

    let base_tint = archetype.accent_rgba();
    let mut blocks = Vec::new();
    let mut courtyard_hole_enu_m: Option<([f64; 2], [f64; 2])> = None;

    let (stories, podium_stories, footprint_m2, gfa_m2, symmetry_bonus, daylight_base, rationale) =
        match archetype {
            MassingArchetype::PodiumTower => {
                let pod_s = zoning.config.podium_stories.clamp(1, (max_stories_zoning / 2).max(1));
                let pod_area = (full_w * full_d).min(zoning.effective_footprint_limit_m2.max(full_w * full_d * 0.7));
                let pod_scale = (pod_area / (full_w * full_d)).sqrt().clamp(0.65, 1.0);
                let pod_w = snap_span_to_bays(full_w * pod_scale, bay_x, 2, max_bays_x);
                let pod_d = snap_span_to_bays(full_d * pod_scale, bay_y, 2, max_bays_y);
                let pod_fp = pod_w * pod_d;
                let pod_gfa = pod_fp * (pod_s as f64);

                let tow_w = snap_span_to_bays(
                    (pod_w - 2.0 * zoning.config.tower_stepback_m).max(bay_x * 2.0),
                    bay_x,
                    2,
                    max_bays_x,
                );
                let tow_d = snap_span_to_bays(
                    (pod_d - 2.0 * zoning.config.tower_stepback_m).max(bay_y * 2.0),
                    bay_y,
                    2,
                    max_bays_y,
                );
                let tow_fp = tow_w * tow_d;
                let rem_gfa = (target_gfa - pod_gfa).max(0.0);
                let tow_s = if tow_fp > 1.0 {
                    (rem_gfa / tow_fp).ceil() as usize
                } else {
                    0
                };
                let total_s = (pod_s + tow_s).clamp(pod_s + 1, max_stories_zoning);
                let actual_gfa = pod_fp * (pod_s as f64) + tow_fp * ((total_s - pod_s) as f64);

                add_tier_block(
                    &mut blocks,
                    "Podium Mass",
                    0,
                    0,
                    pod_s,
                    story_h,
                    [-pod_w * 0.5, -pod_d * 0.5],
                    [pod_w * 0.5, pod_d * 0.5],
                    [base_tint[0], base_tint[1], base_tint[2], 0.52],
                );
                add_tier_block(
                    &mut blocks,
                    "Tower Shaft",
                    1,
                    pod_s,
                    total_s,
                    story_h,
                    [-tow_w * 0.5, -tow_d * 0.5],
                    [tow_w * 0.5, tow_d * 0.5],
                    [
                        (base_tint[0] + 0.14).min(1.0),
                        (base_tint[1] + 0.12).min(1.0),
                        base_tint[2],
                        0.66,
                    ],
                );
                let core_w = (bay_x * 0.92).clamp(4.8, tow_w * 0.45);
                let core_d = (bay_y * 0.92).clamp(4.8, tow_d * 0.45);
                add_core_block(&mut blocks, total_s, story_h, core_w, core_d, 0.0, 0.0);
                (
                    total_s,
                    pod_s,
                    pod_fp,
                    actual_gfa,
                    0.96,
                    82.0,
                    format!(
                        "Concentric {pod_s}F podium ({pod_w:.1}x{pod_d:.1}m) + stepped tower ({tow_w:.1}x{tow_d:.1}m) centers stiffness against seismic torsion."
                    ),
                )
            }
            MassingArchetype::CourtyardBlock => {
                let outer_w = full_w.max(bay_x * 3.0);
                let outer_d = full_d.max(bay_y * 3.0);
                let hole_w = (outer_w - 2.0 * bay_x).max(bay_x).min(outer_w * 0.46);
                let hole_d = (outer_d - 2.0 * bay_y).max(bay_y).min(outer_d * 0.46);
                let hx0 = -hole_w * 0.5;
                let hx1 = hole_w * 0.5;
                let hy0 = -hole_d * 0.5;
                let hy1 = hole_d * 0.5;
                courtyard_hole_enu_m = Some(([hx0, hy0], [hx1, hy1]));

                let ring_fp = (outer_w * outer_d - hole_w * hole_d).max(bay_x * bay_y * 2.0);
                let est_s = ((target_gfa / ring_fp).ceil() as usize).clamp(2, max_stories_zoning);
                let actual_gfa = ring_fp * (est_s as f64);

                // Split courtyard perimeter into 4 clean rectangular wings around the central void.
                add_tier_block(
                    &mut blocks,
                    "South Wing",
                    0,
                    0,
                    est_s,
                    story_h,
                    [-outer_w * 0.5, -outer_d * 0.5],
                    [outer_w * 0.5, hy0],
                    base_tint,
                );
                add_tier_block(
                    &mut blocks,
                    "North Wing",
                    1,
                    0,
                    est_s,
                    story_h,
                    [-outer_w * 0.5, hy1],
                    [outer_w * 0.5, outer_d * 0.5],
                    base_tint,
                );
                add_tier_block(
                    &mut blocks,
                    "West Wing",
                    2,
                    0,
                    est_s,
                    story_h,
                    [-outer_w * 0.5, hy0],
                    [hx0, hy1],
                    base_tint,
                );
                add_tier_block(
                    &mut blocks,
                    "East Wing",
                    3,
                    0,
                    est_s,
                    story_h,
                    [hx1, hy0],
                    [outer_w * 0.5, hy1],
                    base_tint,
                );
                let core_w = (bay_x * 0.65).clamp(3.6, (outer_w - hole_w) * 0.45);
                let core_d = (bay_y * 0.65).clamp(3.6, (outer_d - hole_d) * 0.45);
                add_core_block(
                    &mut blocks,
                    est_s,
                    story_h,
                    core_w,
                    core_d,
                    0.0,
                    (hy1 + outer_d * 0.5) * 0.5,
                );
                (
                    est_s,
                    0,
                    ring_fp,
                    actual_gfa,
                    0.91,
                    94.0,
                    format!(
                        "Perimeter courtyard ring ({outer_w:.1}x{outer_d:.1}m w/ {hole_w:.1}x{hole_d:.1}m atrium) maximizes dual-aspect daylight."
                    ),
                )
            }
            MassingArchetype::LinearSlabBar => {
                let bar_w = full_w;
                let bar_d = snap_span_to_bays((full_d * 0.58).max(bay_y * 2.0), bay_y, 2, max_bays_y)
                    .min(full_d);
                let bar_fp = bar_w * bar_d;
                let est_s = ((target_gfa / bar_fp).ceil() as usize).clamp(2, max_stories_zoning);
                let actual_gfa = bar_fp * (est_s as f64);

                add_tier_block(
                    &mut blocks,
                    "Linear Slab Mass",
                    0,
                    0,
                    est_s,
                    story_h,
                    [-bar_w * 0.5, -bar_d * 0.5],
                    [bar_w * 0.5, bar_d * 0.5],
                    base_tint,
                );
                let core_w = (bay_x * 0.90).clamp(4.5, bar_w * 0.35);
                let core_d = (bay_y * 0.85).clamp(4.2, bar_d * 0.48);
                add_core_block(&mut blocks, est_s, story_h, core_w, core_d, 0.0, 0.0);
                (
                    est_s,
                    0,
                    bar_fp,
                    actual_gfa,
                    0.85,
                    89.0,
                    format!(
                        "Double-loaded rectangular bar ({bar_w:.1}x{bar_d:.1}m) simplifies formwork repetition and solar north/south exposure."
                    ),
                )
            }
            MassingArchetype::SteppedTerrace => {
                let t1_w = full_w;
                let t1_d = full_d;
                let t2_w = snap_span_to_bays(full_w * 0.78, bay_x, 2, max_bays_x);
                let t2_d = snap_span_to_bays(full_d * 0.78, bay_y, 2, max_bays_y);
                let t3_w = snap_span_to_bays(full_w * 0.55, bay_x, 2, max_bays_x);
                let t3_d = snap_span_to_bays(full_d * 0.55, bay_y, 2, max_bays_y);

                let avg_fp = 0.38 * (t1_w * t1_d) + 0.35 * (t2_w * t2_d) + 0.27 * (t3_w * t3_d);
                let total_s = ((target_gfa / avg_fp).ceil() as usize).clamp(3, max_stories_zoning);
                let s1 = (total_s / 3).max(1);
                let s2 = (total_s / 3).max(1);
                let s3 = (total_s - s1 - s2).max(1);
                let final_s = (s1 + s2 + s3).min(max_stories_zoning);
                let actual_gfa = (t1_w * t1_d) * (s1 as f64)
                    + (t2_w * t2_d) * (s2 as f64)
                    + (t3_w * t3_d) * ((final_s - s1 - s2).max(1) as f64);

                add_tier_block(
                    &mut blocks,
                    "Terrace Base",
                    0,
                    0,
                    s1,
                    story_h,
                    [-t1_w * 0.5, -t1_d * 0.5],
                    [t1_w * 0.5, t1_d * 0.5],
                    [base_tint[0], base_tint[1], base_tint[2], 0.50],
                );
                add_tier_block(
                    &mut blocks,
                    "Terrace Mid",
                    1,
                    s1,
                    s1 + s2,
                    story_h,
                    [-t2_w * 0.5, -t2_d * 0.5],
                    [t2_w * 0.5, t2_d * 0.5],
                    [base_tint[0], (base_tint[1] + 0.14).min(1.0), base_tint[2], 0.60],
                );
                add_tier_block(
                    &mut blocks,
                    "Terrace Crown",
                    2,
                    s1 + s2,
                    final_s,
                    story_h,
                    [-t3_w * 0.5, -t3_d * 0.5],
                    [t3_w * 0.5, t3_d * 0.5],
                    [(base_tint[0] + 0.15).min(1.0), (base_tint[1] + 0.22).min(1.0), base_tint[2], 0.70],
                );
                let core_w = (bay_x * 0.85).clamp(4.4, t3_w * 0.48);
                let core_d = (bay_y * 0.85).clamp(4.4, t3_d * 0.48);
                add_core_block(&mut blocks, final_s, story_h, core_w, core_d, 0.0, 0.0);
                (
                    final_s,
                    s1,
                    t1_w * t1_d,
                    actual_gfa,
                    0.89,
                    87.0,
                    format!(
                        "Three cascading setback tiers ({s1}F+{s2}F+{}F) soften urban wind downwash and add green roof terraces.",
                        final_s.saturating_sub(s1 + s2)
                    ),
                )
            }
        };

    let total_height_m = (stories as f64) * story_h;
    let kdb_pct = (footprint_m2 / lot_area) * 100.0;
    let klb_far = gfa_m2 / lot_area;
    let wwr_bonus = ((config.facade_wwr_pct - 25.0) * 0.18).clamp(-6.0, 6.0);
    let daylight_score_pct = (daylight_base + wwr_bonus).clamp(52.0, 98.0);
    let nfa_m2 = gfa_m2 * eff_ratio;
    let efficiency_pct = (nfa_m2 / gfa_m2.max(1.0)) * 100.0;

    // Physics-coupled EigenGate ARI & Max DCR% estimation
    let sys_ari_boost = match config.structural_system {
        MassingStructuralSystem::RcMomentCore => 0.0,
        MassingStructuralSystem::CompositeSteelCore => 0.04,
        MassingStructuralSystem::DiagridHybrid => 0.07,
    };
    let sys_dcr_factor = match config.structural_system {
        MassingStructuralSystem::RcMomentCore => 1.0,
        MassingStructuralSystem::CompositeSteelCore => 0.91,
        MassingStructuralSystem::DiagridHybrid => 0.84,
    };
    let span_avg = 0.5 * (bay_x + bay_y);
    let span_penalty = ((span_avg - 7.2) * 0.018).clamp(-0.04, 0.09);
    let height_slenderness_penalty = ((total_height_m / full_w.min(full_d).max(8.0)) - 1.5).max(0.0) * 0.035;
    let seismic_factor = (site.seismic_pga_ss_g as f64).clamp(0.25, 1.55);
    let core_bonus = ((config.core_wall_thick_mm - 250.0) / 1000.0).clamp(-0.02, 0.05);

    let estimated_eigengate_ari = (0.76 * symmetry_bonus + sys_ari_boost + core_bonus + 0.14
        - span_penalty
        - height_slenderness_penalty
        - (seismic_factor - 0.6).max(0.0) * 0.08)
        .clamp(0.42, 0.96);

    let wind_factor = (site.design_wind_ms as f64 / 38.0).clamp(0.75, 1.45);
    let estimated_max_dcr_pct = ((52.0
        + (stories as f64) * 1.45
        + (span_avg - 6.0).max(0.0) * 3.8
        + (seismic_factor * 14.0)
        + (wind_factor * 5.0))
        * sys_dcr_factor
        / symmetry_bonus.max(0.75))
    .clamp(38.0, 118.0);

    // 5D Cost & 6D Embodied Carbon coupled with site logistics & framing system
    let (unit_cost_usd_m2, carbon_kg_m2_base) = match config.structural_system {
        MassingStructuralSystem::RcMomentCore => (1080.0, 385.0),
        MassingStructuralSystem::CompositeSteelCore => (1240.0, 415.0),
        MassingStructuralSystem::DiagridHybrid => (1390.0, 445.0),
    };
    let height_premium = 1.0 + ((stories.saturating_sub(6) as f64) * 0.018);
    let logistics = (site.remote_logistics_multiplier as f64).clamp(1.0, 1.4);
    let estimated_cost_usd = gfa_m2 * unit_cost_usd_m2 * height_premium * logistics;
    let estimated_cost_idr_billion = (estimated_cost_usd * 16_000.0) / 1_000_000_000.0;
    let carbon_intensity_kg_m2 = carbon_kg_m2_base * (1.0 + (stories as f64) * 0.008) * (0.94 + (span_avg / 60.0));
    let embodied_carbon_tco2e = (gfa_m2 * carbon_intensity_kg_m2) / 1000.0;

    let zoning_compliant = kdb_pct <= zoning.config.kdb_bcr_pct + 0.5
        && klb_far <= zoning.config.klb_far + 0.05
        && total_height_m <= zoning.config.max_height_m + 0.2;

    let program_fit_pct = ((gfa_m2 / target_gfa).min(target_gfa / gfa_m2.max(1.0)) * 100.0).clamp(50.0, 100.0);

    let ari_score = (estimated_eigengate_ari * 100.0).clamp(0.0, 100.0);
    let dcr_score = if estimated_max_dcr_pct <= 95.0 {
        100.0 - (estimated_max_dcr_pct - 65.0).abs() * 0.6
    } else {
        (100.0 - (estimated_max_dcr_pct - 95.0) * 3.0).max(20.0)
    };
    let carbon_score = (100.0 - (carbon_intensity_kg_m2 - 340.0) * 0.25).clamp(35.0, 98.0);
    let compliance_bonus = if zoning_compliant { 100.0 } else { 42.0 };
    let composite_score = (0.24 * ari_score
        + 0.18 * dcr_score
        + 0.20 * compliance_bonus
        + 0.18 * daylight_score_pct
        + 0.12 * program_fit_pct
        + 0.08 * carbon_score)
        .clamp(25.0, 99.0);

    MassingOptionMetrics {
        archetype,
        stories,
        podium_stories,
        total_height_m,
        footprint_area_m2: footprint_m2,
        gfa_m2,
        nfa_m2,
        efficiency_pct,
        kdb_pct,
        klb_far,
        daylight_score_pct,
        estimated_eigengate_ari,
        estimated_max_dcr_pct,
        estimated_cost_usd,
        estimated_cost_idr_billion,
        embodied_carbon_tco2e,
        carbon_intensity_kg_m2,
        zoning_compliant,
        program_fit_pct,
        composite_score,
        blocks_3d: blocks,
        courtyard_hole_enu_m,
        rationale,
    }
}

/// Evaluates all 4 architectural massing options coupled to Zoning, Program & Earth hazards.
pub fn evaluate_massing_options(
    zoning: &SiteZoningReport,
    program: &SpatialProgramReport,
    site: &SiteEnvironmentalCoupling,
    config: &MassingStudyConfig,
) -> MassingStudyReport {
    let options: Vec<MassingOptionMetrics> = MassingArchetype::ALL
        .iter()
        .map(|&arch| evaluate_single_option(arch, zoning, program, site, config))
        .collect();

    let selected_index = MassingArchetype::ALL
        .iter()
        .position(|&a| a == config.selected_archetype)
        .unwrap_or(0);

    let recommended_index = options
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.composite_score.total_cmp(&b.composite_score))
        .map(|(idx, _)| idx)
        .unwrap_or(0);

    let active = &options[selected_index];
    let summary_verdict = format!(
        "{}: {}F ({:.1}m) • GFA {:.0} m² • KLB {:.2} • ARI {:.2} • DCR {:.0}% • ${:.1}M (Rp {:.1}B)",
        active.archetype.short_label(),
        active.stories,
        active.total_height_m,
        active.gfa_m2,
        active.klb_far,
        active.estimated_eigengate_ari,
        active.estimated_max_dcr_pct,
        active.estimated_cost_usd / 1_000_000.0,
        active.estimated_cost_idr_billion,
    );

    MassingStudyReport {
        config: config.clone(),
        site_label: zoning.site_label.clone(),
        story_height_m: zoning.config.typical_story_height_m.clamp(2.8, 6.0),
        target_gfa_m2: program.total_gfa_m2,
        zoning_max_gfa_m2: zoning.max_gfa_klb_m2,
        lot_area_m2: zoning.lot_area_m2,
        buildable_pad_width_m: zoning.buildable_pad_width_m,
        buildable_pad_depth_m: zoning.buildable_pad_depth_m,
        options,
        selected_index,
        recommended_index,
        summary_verdict,
    }
}

fn story_envelope_bounds_m(opt: &MassingOptionMetrics, story: usize) -> ([f64; 2], [f64; 2]) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for blk in &opt.blocks_3d {
        if !blk.is_core && story >= blk.start_story && story < blk.end_story {
            min_x = min_x.min(blk.min_enu_m[0]);
            max_x = max_x.max(blk.max_enu_m[0]);
            min_y = min_y.min(blk.min_enu_m[1]);
            max_y = max_y.max(blk.max_enu_m[1]);
        }
    }
    if !min_x.is_finite() {
        ([-10.0, -10.0], [10.0, 10.0])
    } else {
        ([min_x, min_y], [max_x, max_y])
    }
}

fn point_active_in_story(
    x_m: f64,
    y_m: f64,
    story: usize,
    opt: &MassingOptionMetrics,
) -> bool {
    const EPS: f64 = 0.08;
    opt.blocks_3d.iter().any(|blk| {
        !blk.is_core
            && story >= blk.start_story
            && story < blk.end_story
            && x_m >= blk.min_enu_m[0] - EPS
            && x_m <= blk.max_enu_m[0] + EPS
            && y_m >= blk.min_enu_m[1] - EPS
            && y_m <= blk.max_enu_m[1] + EPS
    })
}

fn populate_facade_openings(
    wall: &mut BimWall,
    next_id: &mut usize,
    is_ground: bool,
    is_south_front: bool,
    wwr_pct: f64,
    floor_h_mm: f64,
) -> usize {
    let len_mm = wall.length_mm();
    if len_mm < 2800.0 {
        return 0;
    }
    let mut count = 0usize;
    if is_ground && is_south_front {
        let door_w = 2200.0;
        let oid = *next_id;
        *next_id += 1;
        wall.openings.push(BimOpening {
            id: oid,
            mark: format!("D{oid}"),
            kind: OpeningKind::DoubleDoor,
            offset_along_mm: ((len_mm - door_w) * 0.5).max(300.0),
            width_mm: door_w,
            height_mm: 2400.0_f64.min(floor_h_mm - 300.0),
            sill_height_mm: 0.0,
            flip_facing: false,
            flip_hand: false,
        });
        return 1;
    }
    let bays = ((len_mm / 4200.0).floor() as usize).clamp(1, 6);
    let step = len_mm / (bays as f64);
    let wwr_scale = (wwr_pct / 50.0).clamp(0.45, 1.35);
    let win_w = (step * 0.48 * wwr_scale).clamp(1100.0, step - 700.0);
    let win_h = (floor_h_mm * 0.46 * wwr_scale).clamp(1100.0, floor_h_mm - 1100.0);
    if win_w > 600.0 && win_h > 600.0 {
        for b in 0..bays {
            let off = (b as f64) * step + (step - win_w) * 0.5;
            if off > 250.0 && off + win_w < len_mm - 250.0 {
                let oid = *next_id;
                *next_id += 1;
                wall.openings.push(BimOpening {
                    id: oid,
                    mark: format!("W{oid}"),
                    kind: OpeningKind::FixedWindow,
                    offset_along_mm: off,
                    width_mm: win_w,
                    height_mm: win_h,
                    sill_height_mm: 900.0,
                    flip_facing: false,
                    flip_hand: false,
                });
                count += 1;
            }
        }
    }
    count
}

/// 1-Click Mass-to-Engineered-BIM: synthesizes a full parametric `BimProject` from
/// the chosen massing archetype (levels, grids, columns, beams, core walls, facade walls & slabs).
pub fn convert_massing_option_to_bim(
    report: &MassingStudyReport,
    option_idx: usize,
) -> (BimProject, MassToBimSummary) {
    let opt = report
        .options
        .get(option_idx)
        .or_else(|| report.active_option())
        .cloned()
        .unwrap_or_else(|| {
            MassingOptionMetrics {
                archetype: MassingArchetype::PodiumTower,
                stories: 6,
                podium_stories: 2,
                total_height_m: 21.6,
                footprint_area_m2: 600.0,
                gfa_m2: 3600.0,
                nfa_m2: 2800.0,
                efficiency_pct: 78.0,
                kdb_pct: 45.0,
                klb_far: 2.5,
                daylight_score_pct: 84.0,
                estimated_eigengate_ari: 0.84,
                estimated_max_dcr_pct: 68.0,
                estimated_cost_usd: 4_000_000.0,
                estimated_cost_idr_billion: 64.0,
                embodied_carbon_tco2e: 1400.0,
                carbon_intensity_kg_m2: 388.0,
                zoning_compliant: true,
                program_fit_pct: 95.0,
                composite_score: 88.0,
                blocks_3d: Vec::new(),
                courtyard_hole_enu_m: None,
                rationale: String::new(),
            }
        });

    let story_h_m = report.story_height_m.clamp(2.8, 6.0);
    let floor_h_mm = story_h_m * 1000.0;
    let bay_x_m = report.config.bay_x_m.clamp(4.8, 10.8);
    let bay_y_m = report.config.bay_y_m.clamp(4.8, 10.8);

    let mut next_id = 1usize;
    let mut alloc_id = || {
        let id = next_id;
        next_id += 1;
        id
    };

    // 1. Levels (1 = Ground +0.00m .. N+1 = Roof)
    let mut levels = Vec::with_capacity(opt.stories + 1);
    for s in 0..=opt.stories {
        let elev_m = (s as f64) * story_h_m;
        let name = if s == 0 {
            "Level 1 (+0.00m)".to_string()
        } else if s == opt.stories {
            format!("Roof (+{elev_m:.2}m)")
        } else {
            format!("Level {} (+{elev_m:.2}m)", s + 1)
        };
        levels.push(BimLevel {
            id: alloc_id(),
            name,
            elevation_mm: elev_m * 1000.0,
        });
    }

    // 2. Determine outer bounds & build regular structural grids in meters
    let (base_min_m, base_max_m) = story_envelope_bounds_m(&opt, 0);
    let mut xs_m = Vec::new();
    let nx = (((base_max_m[0] - base_min_m[0]) / bay_x_m).round() as usize).max(2);
    for i in 0..=nx {
        xs_m.push(base_min_m[0] + (i as f64) * ((base_max_m[0] - base_min_m[0]) / (nx as f64)));
    }
    let mut ys_m = Vec::new();
    let ny = (((base_max_m[1] - base_min_m[1]) / bay_y_m).round() as usize).max(2);
    for j in 0..=ny {
        ys_m.push(base_min_m[1] + (j as f64) * ((base_max_m[1] - base_min_m[1]) / (ny as f64)));
    }

    let mut grids = Vec::with_capacity(xs_m.len() + ys_m.len());
    let ext_pad_mm = 1800.0;
    for (idx, &xm) in xs_m.iter().enumerate() {
        grids.push(BimGrid {
            id: alloc_id(),
            label: format!("{}", idx + 1),
            start: P2(xm * 1000.0, base_min_m[1] * 1000.0 - ext_pad_mm),
            end: P2(xm * 1000.0, base_max_m[1] * 1000.0 + ext_pad_mm),
        });
    }
    for (idx, &ym) in ys_m.iter().enumerate() {
        let lbl = char::from_u32(('A' as u32) + (idx as u32))
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("G{idx}"));
        grids.push(BimGrid {
            id: alloc_id(),
            label: lbl,
            start: P2(base_min_m[0] * 1000.0 - ext_pad_mm, ym * 1000.0),
            end: P2(base_max_m[0] * 1000.0 + ext_pad_mm, ym * 1000.0),
        });
    }

    let use_wf_steel = report.config.structural_system != MassingStructuralSystem::RcMomentCore;
    let mut columns = Vec::new();
    let mut runs = Vec::new();
    let mut walls = Vec::new();
    let mut slabs = Vec::new();
    let mut footings = Vec::new();
    let mut openings_count = 0usize;

    // Ground slab-on-grade at Level 1
    let ground_ring = vec![
        P2(base_min_m[0] * 1000.0, base_min_m[1] * 1000.0),
        P2(base_max_m[0] * 1000.0, base_min_m[1] * 1000.0),
        P2(base_max_m[0] * 1000.0, base_max_m[1] * 1000.0),
        P2(base_min_m[0] * 1000.0, base_max_m[1] * 1000.0),
    ];
    let mut ground_openings = Vec::new();
    if let Some((h0, h1)) = opt.courtyard_hole_enu_m {
        ground_openings.push(vec![
            P2(h0[0] * 1000.0, h0[1] * 1000.0),
            P2(h1[0] * 1000.0, h0[1] * 1000.0),
            P2(h1[0] * 1000.0, h1[1] * 1000.0),
            P2(h0[0] * 1000.0, h1[1] * 1000.0),
        ]);
    }
    slabs.push(BimSlab {
        id: alloc_id(),
        name: "Ground Slab S0".to_string(),
        level_id: levels[0].id,
        boundary: ground_ring,
        openings: ground_openings,
        thickness_mm: 200.0,
        is_roof: false,
        roof_slope_deg: 0.0,
    });

    // Core footprint
    let core_block = opt.blocks_3d.iter().find(|b| b.is_core);
    let (core_min_x_mm, core_max_x_mm, core_min_y_mm, core_max_y_mm) = if let Some(cb) = core_block {
        (
            cb.min_enu_m[0] * 1000.0,
            cb.max_enu_m[0] * 1000.0,
            cb.min_enu_m[1] * 1000.0,
            cb.max_enu_m[1] * 1000.0,
        )
    } else {
        (-2800.0, 2800.0, -2800.0, 2800.0)
    };
    let core_t = report.config.core_wall_thick_mm.clamp(200.0, 500.0);

    // 3. Generate per-story Columns, Beams, Slabs, Core Walls, Facade Compound Walls & Footings
    for s in 0..opt.stories {
        let base_lvl_id = levels[s].id;
        let top_lvl_id = levels[s + 1].id;
        let stories_above = opt.stories - s;
        let col_dim_mm = (420.0 + (stories_above as f64) * 26.0).clamp(450.0, 800.0);

        let mut active_grid = vec![vec![false; ys_m.len()]; xs_m.len()];
        for (ix, &xm) in xs_m.iter().enumerate() {
            for (iy, &ym) in ys_m.iter().enumerate() {
                if point_active_in_story(xm, ym, s, &opt) {
                    active_grid[ix][iy] = true;
                    let cid = alloc_id();
                    let pos = P2(xm * 1000.0, ym * 1000.0);
                    columns.push(BimColumn {
                        id: cid,
                        name: format!("C-L{}-{}{}", s + 1, ix + 1, (b'A' + (iy as u8)) as char),
                        base_level_id: base_lvl_id,
                        top_level_id: top_lvl_id,
                        pos,
                        width_mm: if use_wf_steel { 380.0 } else { col_dim_mm },
                        depth_mm: if use_wf_steel { 380.0 } else { col_dim_mm },
                        rotation_deg: 0.0,
                        is_round: false,
                        is_wf_profile: use_wf_steel,
                        profile_name: if use_wf_steel {
                            if stories_above > 8 {
                                "W14x159".to_string()
                            } else {
                                "W14x90".to_string()
                            }
                        } else {
                            format!("RC-{:.0}x{:.0}", col_dim_mm, col_dim_mm)
                        },
                    });

                    if s == 0 && report.config.include_basement_footings {
                        let f_dim = (1900.0 + (opt.stories as f64) * 140.0).clamp(2000.0, 3600.0);
                        let f_thick = (550.0 + (opt.stories as f64) * 35.0).clamp(550.0, 1000.0);
                        footings.push(BimFooting {
                            id: alloc_id(),
                            name: format!("F-{}{}", ix + 1, (b'A' + (iy as u8)) as char),
                            level_id: base_lvl_id,
                            pos,
                            width_mm: f_dim,
                            length_mm: f_dim,
                            thickness_mm: f_thick,
                        });
                    }
                }
            }
        }

        // Floor beams at `top_lvl_id` spanning between adjacent active columns
        let beam_w = if use_wf_steel { 230.0 } else { 350.0 };
        let beam_d = if use_wf_steel { 530.0 } else { 620.0 };
        let beam_prof = if use_wf_steel {
            "W21x62".to_string()
        } else {
            "RC-350x620".to_string()
        };
        for ix in 0..xs_m.len() {
            for iy in 0..ys_m.len() {
                if !active_grid[ix][iy] {
                    continue;
                }
                if ix + 1 < xs_m.len() && active_grid[ix + 1][iy] {
                    let mid_x = 0.5 * (xs_m[ix] + xs_m[ix + 1]);
                    if point_active_in_story(mid_x, ys_m[iy], s, &opt) {
                        runs.push(BimRun {
                            id: alloc_id(),
                            name: format!("BX-L{}", s + 2),
                            category: RunCategory::StructuralBeam,
                            level_id: top_lvl_id,
                            start: P2(xs_m[ix] * 1000.0, ys_m[iy] * 1000.0),
                            end: P2(xs_m[ix + 1] * 1000.0, ys_m[iy] * 1000.0),
                            start_offset_mm: 0.0,
                            end_offset_mm: 0.0,
                            justify: RunJustify::Top,
                            profile_name: beam_prof.clone(),
                            width_mm: beam_w,
                            depth_mm: beam_d,
                            is_round: false,
                            is_wf_profile: use_wf_steel,
                            system_name: report.config.structural_system.short_label().to_string(),
                        });
                    }
                }
                if iy + 1 < ys_m.len() && active_grid[ix][iy + 1] {
                    let mid_y = 0.5 * (ys_m[iy] + ys_m[iy + 1]);
                    if point_active_in_story(xs_m[ix], mid_y, s, &opt) {
                        runs.push(BimRun {
                            id: alloc_id(),
                            name: format!("BY-L{}", s + 2),
                            category: RunCategory::StructuralBeam,
                            level_id: top_lvl_id,
                            start: P2(xs_m[ix] * 1000.0, ys_m[iy] * 1000.0),
                            end: P2(xs_m[ix] * 1000.0, ys_m[iy + 1] * 1000.0),
                            start_offset_mm: 0.0,
                            end_offset_mm: 0.0,
                            justify: RunJustify::Top,
                            profile_name: beam_prof.clone(),
                            width_mm: beam_w,
                            depth_mm: beam_d,
                            is_round: false,
                            is_wf_profile: use_wf_steel,
                            system_name: report.config.structural_system.short_label().to_string(),
                        });
                    }
                }
            }
        }

        // Story Floor / Roof Slab at `top_lvl_id`
        let (smin_m, smax_m) = story_envelope_bounds_m(&opt, s);
        let x0 = smin_m[0] * 1000.0;
        let x1 = smax_m[0] * 1000.0;
        let y0 = smin_m[1] * 1000.0;
        let y1 = smax_m[1] * 1000.0;
        let mut slab_openings = Vec::new();
        if let Some((h0, h1)) = opt.courtyard_hole_enu_m {
            slab_openings.push(vec![
                P2(h0[0] * 1000.0, h0[1] * 1000.0),
                P2(h1[0] * 1000.0, h0[1] * 1000.0),
                P2(h1[0] * 1000.0, h1[1] * 1000.0),
                P2(h0[0] * 1000.0, h1[1] * 1000.0),
            ]);
        }
        slabs.push(BimSlab {
            id: alloc_id(),
            name: if s + 1 == opt.stories {
                "Roof Slab".to_string()
            } else {
                format!("Floor Slab L{}", s + 2)
            },
            level_id: top_lvl_id,
            boundary: vec![P2(x0, y0), P2(x1, y0), P2(x1, y1), P2(x0, y1)],
            openings: slab_openings,
            thickness_mm: if use_wf_steel { 150.0 } else { 180.0 },
            is_roof: s + 1 == opt.stories,
            roof_slope_deg: 0.0,
        });

        // Structural Shear-Wall Core box
        let core_edges = [
            ("Core-S", P2(core_min_x_mm, core_min_y_mm), P2(core_max_x_mm, core_min_y_mm), true),
            ("Core-E", P2(core_max_x_mm, core_min_y_mm), P2(core_max_x_mm, core_max_y_mm), false),
            ("Core-N", P2(core_max_x_mm, core_max_y_mm), P2(core_min_x_mm, core_max_y_mm), false),
            ("Core-W", P2(core_min_x_mm, core_max_y_mm), P2(core_min_x_mm, core_min_y_mm), false),
        ];
        for (lbl, ws, we, has_access) in core_edges {
            let mut cw = BimWall {
                id: alloc_id(),
                name: format!("{lbl}-L{}", s + 1),
                base_level_id: base_lvl_id,
                top_level_id: Some(top_lvl_id),
                unconnected_height_mm: floor_h_mm,
                start: ws,
                end: we,
                thickness_mm: core_t,
                is_structural: true,
                layers: Vec::new(),
                openings: Vec::new(),
            };
            if has_access && cw.length_mm() > 2600.0 {
                let oid = alloc_id();
                cw.openings.push(BimOpening {
                    id: oid,
                    mark: format!("CD{oid}"),
                    kind: OpeningKind::DoubleDoor,
                    offset_along_mm: (cw.length_mm() - 1800.0) * 0.5,
                    width_mm: 1800.0,
                    height_mm: 2300.0_f64.min(floor_h_mm - 300.0),
                    sill_height_mm: 0.0,
                    flip_facing: false,
                    flip_hand: false,
                });
                openings_count += 1;
            }
            walls.push(cw);
        }

        // Exterior Compound Facade Walls with hosted windows & ground lobby entry
        let facade_edges = [
            ("Facade-S", P2(x0, y0), P2(x1, y0), true),
            ("Facade-E", P2(x1, y0), P2(x1, y1), false),
            ("Facade-N", P2(x1, y1), P2(x0, y1), false),
            ("Facade-W", P2(x0, y1), P2(x0, y0), false),
        ];
        for (lbl, ws, we, is_south) in facade_edges {
            let mut fw = BimWall {
                id: alloc_id(),
                name: format!("{lbl}-L{}", s + 1),
                base_level_id: base_lvl_id,
                top_level_id: Some(top_lvl_id),
                unconnected_height_mm: floor_h_mm,
                start: ws,
                end: we,
                thickness_mm: 220.0,
                is_structural: false,
                layers: BimWall::default_compound_layers(220.0),
                openings: Vec::new(),
            };
            openings_count += populate_facade_openings(
                &mut fw,
                &mut next_id,
                s == 0,
                is_south,
                report.config.facade_wwr_pct,
                floor_h_mm,
            );
            walls.push(fw);
        }

        // Inner courtyard perimeter walls for CourtyardBlock
        if let Some((h0, h1)) = opt.courtyard_hole_enu_m {
            let hx0 = h0[0] * 1000.0;
            let hx1 = h1[0] * 1000.0;
            let hy0 = h0[1] * 1000.0;
            let hy1 = h1[1] * 1000.0;
            let inner_edges = [
                ("Atrium-S", P2(hx0, hy0), P2(hx1, hy0)),
                ("Atrium-E", P2(hx1, hy0), P2(hx1, hy1)),
                ("Atrium-N", P2(hx1, hy1), P2(hx0, hy1)),
                ("Atrium-W", P2(hx0, hy1), P2(hx0, hy0)),
            ];
            for (lbl, ws, we) in inner_edges {
                let mut aw = BimWall {
                    id: alloc_id(),
                    name: format!("{lbl}-L{}", s + 1),
                    base_level_id: base_lvl_id,
                    top_level_id: Some(top_lvl_id),
                    unconnected_height_mm: floor_h_mm,
                    start: ws,
                    end: we,
                    thickness_mm: 200.0,
                    is_structural: false,
                    layers: BimWall::default_compound_layers(200.0),
                    openings: Vec::new(),
                };
                openings_count += populate_facade_openings(
                    &mut aw,
                    &mut next_id,
                    false,
                    false,
                    report.config.facade_wwr_pct,
                    floor_h_mm,
                );
                walls.push(aw);
            }
        }
    }

    let project = BimProject {
        project_name: format!("{} — {}", report.site_label, opt.archetype.label()),
        next_id,
        levels: levels.clone(),
        grids: grids.clone(),
        walls: walls.clone(),
        runs: runs.clone(),
        columns: columns.clone(),
        footings: footings.clone(),
        slabs: slabs.clone(),
        rooms: Vec::new(),
        dimensions: Vec::new(),
        rebars: Vec::new(),
        view_range: ViewRange::default(),
        section_line: (
            P2(base_min_m[0] * 1000.0 - 2000.0, 0.0),
            P2(base_max_m[0] * 1000.0 + 2000.0, 0.0),
        ),
    };

    let summary_line = format!(
        "Mass-to-BIM ({}): {} levels, {} cols, {} beams, {} walls ({} openings), {} slabs, {} footings",
        opt.archetype.short_label(),
        levels.len(),
        columns.len(),
        runs.len(),
        walls.len(),
        openings_count,
        slabs.len(),
        footings.len(),
    );

    let summary = MassToBimSummary {
        archetype_label: opt.archetype.label().to_string(),
        levels: levels.len(),
        grids: grids.len(),
        columns: columns.len(),
        beams: runs.len(),
        walls: walls.len(),
        openings: openings_count,
        slabs: slabs.len(),
        footings: footings.len(),
        gfa_m2: opt.gfa_m2,
        summary_line,
    };

    (project, summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};
    use crate::core::program_agent::{evaluate_spatial_program, SpatialProgramConfig};
    use crate::core::zoning_agent::{evaluate_site_zoning, SiteZoningConfig};

    #[test]
    fn evaluate_massing_produces_four_distinct_options() {
        let dummy = BimProject::empty();
        let site = SiteEnvironmentalCoupling::default();
        let zoning = evaluate_site_zoning(&dummy, Some(&site), &SiteZoningConfig::default());
        let program = evaluate_spatial_program(Some(&zoning), &SpatialProgramConfig::default());
        let rep = evaluate_massing_options(&zoning, &program, &site, &MassingStudyConfig::default());

        assert_eq!(rep.options.len(), 4);
        for opt in &rep.options {
            assert!(opt.gfa_m2 > 500.0);
            assert!(opt.stories >= 2);
            assert!((0.3..=1.0).contains(&opt.estimated_eigengate_ari));
            assert!(opt.estimated_cost_usd > 0.0);
            assert!(opt.embodied_carbon_tco2e > 0.0);
            assert!(!opt.blocks_3d.is_empty());
        }
    }

    #[test]
    fn convert_all_massing_archetypes_to_valid_engineered_bim() {
        let dummy = BimProject::empty();
        let site = SiteEnvironmentalCoupling::default();
        let zoning = evaluate_site_zoning(&dummy, Some(&site), &SiteZoningConfig::default());
        let program = evaluate_spatial_program(Some(&zoning), &SpatialProgramConfig::default());
        let rep = evaluate_massing_options(&zoning, &program, &site, &MassingStudyConfig::default());

        for idx in 0..rep.options.len() {
            let (bim, summary) = convert_massing_option_to_bim(&rep, idx);
            assert!(summary.levels >= 3);
            assert!(summary.columns > 0);
            assert!(summary.beams > 0);
            assert!(summary.walls > 0);
            assert!(summary.openings > 0);
            assert!(summary.slabs >= 2);
            assert!(summary.footings > 0);
            let fem = bim.to_structural_model();
            assert!(!fem.nodes.is_empty());
            assert!(!fem.elements.is_empty());
        }
    }

    #[test]
    fn massing_ifcx_overlay_composes_cleanly() {
        let dummy = BimProject::empty();
        let site = SiteEnvironmentalCoupling::default();
        let zoning = evaluate_site_zoning(&dummy, Some(&site), &SiteZoningConfig::default());
        let program = evaluate_spatial_program(Some(&zoning), &SpatialProgramConfig::default());
        let rep = evaluate_massing_options(&zoning, &program, &site, &MassingStudyConfig::default());

        let base = IfcxDocument::from_bim_project(&dummy, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_massing_study_overlay(&base, &rep.to_ifcx_overlay());
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_MASSING_STUDY)));
    }
}
