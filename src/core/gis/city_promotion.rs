//! city_promotion.rs — Update `v0.8.7.8` (`8/10` -> `v0.8.8`):
//! Interactive City-to-BIM Promotion & Smart Road Intersections Engine.
//!
//! 1. **Smart Road Intersections**: Automatically detects road endpoint hubs and polyline crossings
//!    across `IfcxCityScene::roads`, classifies them (`TJunction`, `FourWayCross`, `Roundabout`,
//!    `MergeJunction`), computes HCM 6th Edition control delay & Level of Service (`LOS A..F`),
//!    and composes non-destructive IFCX overlays under `eigengraph::gis::city_intersections`.
//! 2. **City-to-BIM Promotion**: Converts any procedural building on an `IfcxSpatialZone` into a
//!    full parametric, 6-DOF FEM-solvable OpenBIM `BimProject` (levels, structural grids, columns,
//!    beams, RC shear core, exterior walls with openings, slabs & footings) anchored at its real WGS84 coordinates.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::gis::city_builder::{IfcxCityScene, IfcxRoadKind, IfcxSpatialZone};
use crate::core::gis::map_widget::SlippyMapState;
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::massing_agent::{
    convert_massing_option_to_bim, MassingArchetype, MassingOptionMetrics, MassingStructuralSystem,
    MassingStudyConfig, MassingStudyReport, MassingVolumeBlock3D,
};

pub const NS_EG_CITY_INTERSECTIONS: &str = "eigengraph::gis::city_intersections";

/// Geometric & control typology of a detected road junction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntersectionKind {
    TJunction,
    FourWayCross,
    Roundabout,
    MergeJunction,
}

impl IntersectionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::TJunction => "Signalized T-Junction",
            Self::FourWayCross => "Signalized 4-Way Cross",
            Self::Roundabout => "Modern Roundabout",
            Self::MergeJunction => "Collector Merge / Bend",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::TJunction => "T-Junc",
            Self::FourWayCross => "4-Way",
            Self::Roundabout => "Roundabout",
            Self::MergeJunction => "Merge",
        }
    }
}

/// Highway Capacity Manual (HCM 6th Ed) Intersection Level-of-Service grade (`LOS A` .. `LOS F`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IntersectionLosGrade {
    LosA,
    LosB,
    LosC,
    LosD,
    LosE,
    LosF,
}

impl IntersectionLosGrade {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::LosA => "LOS A",
            Self::LosB => "LOS B",
            Self::LosC => "LOS C",
            Self::LosD => "LOS D",
            Self::LosE => "LOS E",
            Self::LosF => "LOS F",
        }
    }

    pub fn rgba(self) -> [f32; 4] {
        match self {
            Self::LosA | Self::LosB => [0.16, 0.84, 0.56, 0.95],
            Self::LosC => [0.22, 0.74, 0.96, 0.95],
            Self::LosD => [0.96, 0.72, 0.16, 0.95],
            Self::LosE | Self::LosF => [0.96, 0.24, 0.24, 0.95],
        }
    }

    pub fn is_acceptable(self) -> bool {
        matches!(self, Self::LosA | Self::LosB | Self::LosC | Self::LosD)
    }
}

fn default_true() -> bool {
    true
}
fn default_snap_radius_m() -> f64 {
    8.5
}
fn default_phf() -> f64 {
    0.92
}

/// User-tunable configuration for Smart Road Intersection detection & geometry synthesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CityIntersectionConfig {
    #[serde(default)]
    pub prefer_roundabouts: bool,
    #[serde(default = "default_true")]
    pub auto_crosswalks: bool,
    #[serde(default = "default_snap_radius_m")]
    pub snap_radius_m: f64,
    #[serde(default = "default_phf")]
    pub peak_hour_factor: f64,
}

impl Default for CityIntersectionConfig {
    fn default() -> Self {
        Self {
            prefer_roundabouts: false,
            auto_crosswalks: true,
            snap_radius_m: default_snap_radius_m(),
            peak_hour_factor: default_phf(),
        }
    }
}

/// Evaluated smart intersection node with branch geometry and HCM LOS performance metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CityRoadIntersection {
    pub id: usize,
    pub name: String,
    pub kind: IntersectionKind,
    pub center_enu_m: [f64; 2],
    pub elev_z_rel_m: f32,
    pub connected_road_ids: Vec<usize>,
    pub approach_bearings_deg: Vec<f64>,
    pub branch_count: usize,
    pub max_road_width_m: f64,
    pub apron_radius_m: f64,
    pub island_radius_m: f64,
    pub peak_vph: f64,
    pub capacity_vph: f64,
    pub v_over_c_ratio: f64,
    pub avg_delay_sec: f64,
    pub los: IntersectionLosGrade,
    pub signal_cycle_s: u32,
}

/// Compact IFCX overlay payload for non-destructive `/earth/city/intersections` composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CityIntersectionsIfcxOverlay {
    pub total_intersections: usize,
    pub roundabout_count: usize,
    pub signalized_count: usize,
    pub worst_los: String,
    pub avg_delay_sec: f64,
    pub all_los_acceptable: bool,
}

/// Complete Smart Road Intersection topology & HCM LOS audit report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CityIntersectionNetworkReport {
    pub config: CityIntersectionConfig,
    pub intersections: Vec<CityRoadIntersection>,
    pub total_intersections: usize,
    pub roundabout_count: usize,
    pub signalized_count: usize,
    pub worst_los: IntersectionLosGrade,
    pub avg_delay_sec: f64,
    pub all_los_acceptable: bool,
    pub summary_verdict: String,
}

impl CityIntersectionNetworkReport {
    /// Builds a compact IFCX overlay payload for `IfcxStage::compose_with_city_intersections_overlay`.
    pub fn to_ifcx_overlay(&self) -> CityIntersectionsIfcxOverlay {
        CityIntersectionsIfcxOverlay {
            total_intersections: self.total_intersections,
            roundabout_count: self.roundabout_count,
            signalized_count: self.signalized_count,
            worst_los: self.worst_los.short_label().to_string(),
            avg_delay_sec: self.avg_delay_sec,
            all_los_acceptable: self.all_los_acceptable,
        }
    }
}

/// Outcome summary returned when promoting a procedural city parcel into the editable active `BimProject`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CityToBimPromotionSummary {
    pub zone_id: usize,
    pub building_name: String,
    pub zone_kind_label: String,
    pub floors: usize,
    pub gfa_m2: f64,
    pub target_lat_lon: (f64, f64),
    pub heading_deg: f64,
    pub levels: usize,
    pub columns: usize,
    pub beams: usize,
    pub walls: usize,
    pub openings: usize,
    pub slabs: usize,
    pub footings: usize,
    pub summary_line: String,
}

fn segment_intersection_2d(
    a0: [f64; 2],
    a1: [f64; 2],
    b0: [f64; 2],
    b1: [f64; 2],
) -> Option<[f64; 2]> {
    let dax = a1[0] - a0[0];
    let day = a1[1] - a0[1];
    let dbx = b1[0] - b0[0];
    let dby = b1[1] - b0[1];
    let denom = dax * dby - day * dbx;
    if denom.abs() < 1e-6 {
        return None;
    }
    let rx = b0[0] - a0[0];
    let ry = b0[1] - a0[1];
    let t = (rx * dby - ry * dbx) / denom;
    let u = (rx * day - ry * dax) / denom;
    if (-1e-4..=1.0001).contains(&t) && (-1e-4..=1.0001).contains(&u) {
        Some([a0[0] + t * dax, a0[1] + t * day])
    } else {
        None
    }
}

fn angle_diff_deg(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

fn push_unique_bearing(bearings: &mut Vec<f64>, deg: f64) {
    let norm = deg.rem_euclid(360.0);
    if !bearings.iter().any(|&b| angle_diff_deg(b, norm) < 22.0) {
        bearings.push(norm);
    }
}

/// Detects road junctions (endpoint clusters + 2D crossings), classifies them, and evaluates HCM LOS.
pub fn detect_and_evaluate_road_intersections(
    city: &IfcxCityScene,
    tin: Option<&TinSurfaceMesh>,
    sea_level_m: f32,
    config: &CityIntersectionConfig,
) -> CityIntersectionNetworkReport {
    let mut cfg = config.clone();
    cfg.snap_radius_m = cfg.snap_radius_m.clamp(4.0, 24.0);
    cfg.peak_hour_factor = cfg.peak_hour_factor.clamp(0.65, 1.0);

    let ground_roads: Vec<&crate::core::gis::city_builder::IfcxRoadSegment> =
        city.roads.iter().filter(|r| !r.is_bridge).collect();

    if ground_roads.is_empty() {
        return CityIntersectionNetworkReport {
            config: cfg,
            intersections: Vec::new(),
            total_intersections: 0,
            roundabout_count: 0,
            signalized_count: 0,
            worst_los: IntersectionLosGrade::LosA,
            avg_delay_sec: 0.0,
            all_los_acceptable: true,
            summary_verdict: "Road Intersections (0 junctions): no ground roadways authored yet"
                .to_string(),
        };
    }

    // 1. Gather raw junction candidate centers (road endpoints + interior spline crossings)
    let mut raw_centers: Vec<[f64; 2]> = Vec::new();
    let polylines: Vec<(usize, Vec<[f64; 2]>)> = ground_roads
        .iter()
        .map(|r| {
            let steps = if r.control_enu_m.is_some() { 14 } else { 4 };
            (r.id, r.sample_centerline(steps))
        })
        .collect();

    for r in &ground_roads {
        raw_centers.push(r.start_enu_m);
        raw_centers.push(r.end_enu_m);
    }
    for i in 0..polylines.len() {
        for j in (i + 1)..polylines.len() {
            for sa in polylines[i].1.windows(2) {
                for sb in polylines[j].1.windows(2) {
                    if let Some(hit) = segment_intersection_2d(sa[0], sa[1], sb[0], sb[1]) {
                        raw_centers.push(hit);
                    }
                }
            }
        }
    }

    // 2. Spatial clustering within cluster_tol
    let cluster_tol = cfg.snap_radius_m.max(9.0);
    let mut hubs: Vec<([f64; 2], usize)> = Vec::new();
    for pt in raw_centers {
        if let Some((hc, cnt)) = hubs
            .iter_mut()
            .find(|(c, _)| (c[0] - pt[0]).hypot(c[1] - pt[1]) <= cluster_tol)
        {
            let w = *cnt as f64;
            hc[0] = (hc[0] * w + pt[0]) / (w + 1.0);
            hc[1] = (hc[1] * w + pt[1]) / (w + 1.0);
            *cnt += 1;
        } else {
            hubs.push((pt, 1));
        }
    }

    let ref_z = tin
        .map(|t| t.sample_elevation_and_slope(0.0, 0.0).0)
        .unwrap_or(0.0);
    let metrics = city.district_metrics();
    let urban_trip_vph =
        (metrics.total_residents as f64 * 0.42 + metrics.total_jobs as f64 * 0.58).max(320.0);

    let mut intersections = Vec::new();
    for (hub_pt, _) in hubs {
        let mut road_ids = Vec::new();
        let mut bearings = Vec::new();
        let mut max_w = 0.0_f64;
        let mut branch_cap_vph = 0.0_f64;

        for (r_idx, road) in ground_roads.iter().enumerate() {
            let pts = &polylines[r_idx].1;
            let ds = (road.start_enu_m[0] - hub_pt[0]).hypot(road.start_enu_m[1] - hub_pt[1]);
            let de = (road.end_enu_m[0] - hub_pt[0]).hypot(road.end_enu_m[1] - hub_pt[1]);

            let mut touches = false;
            if ds <= cluster_tol && pts.len() >= 2 {
                let b = (pts[1][1] - pts[0][1]).atan2(pts[1][0] - pts[0][0]).to_degrees();
                push_unique_bearing(&mut bearings, b);
                touches = true;
            }
            if de <= cluster_tol && pts.len() >= 2 {
                let n = pts.len();
                let b = (pts[n - 2][1] - pts[n - 1][1])
                    .atan2(pts[n - 2][0] - pts[n - 1][0])
                    .to_degrees();
                push_unique_bearing(&mut bearings, b);
                touches = true;
            }

            // Also check interior polyline passage through hub_pt
            if !touches {
                for w in pts.windows(2) {
                    let dx = w[1][0] - w[0][0];
                    let dy = w[1][1] - w[0][1];
                    let l2 = dx * dx + dy * dy;
                    if l2 < 1e-3 {
                        continue;
                    }
                    let t = (((hub_pt[0] - w[0][0]) * dx + (hub_pt[1] - w[0][1]) * dy) / l2)
                        .clamp(0.0, 1.0);
                    let px = w[0][0] + dx * t;
                    let py = w[0][1] + dy * t;
                    if (hub_pt[0] - px).hypot(hub_pt[1] - py) <= cluster_tol * 0.85 {
                        let fwd = dy.atan2(dx).to_degrees();
                        push_unique_bearing(&mut bearings, fwd);
                        push_unique_bearing(&mut bearings, fwd + 180.0);
                        touches = true;
                        break;
                    }
                }
            }

            if touches {
                road_ids.push(road.id);
                max_w = max_w.max(road.width_m);
                let per_branch = match road.kind {
                    IfcxRoadKind::Arterial => 1650.0,
                    IfcxRoadKind::Collector => 1050.0,
                    IfcxRoadKind::LocalStreet => 650.0,
                    IfcxRoadKind::PedestrianBoulevard => 280.0,
                };
                branch_cap_vph += per_branch;
            }
        }

        if road_ids.len() < 2 {
            continue;
        }
        let branch_count = bearings.len();
        if branch_count < 2 {
            continue;
        }
        // Skip trivial near-collinear 2-segment continuations (>155° apart)
        if branch_count == 2 && angle_diff_deg(bearings[0], bearings[1]) > 155.0 {
            continue;
        }

        let ground_msl = tin
            .map(|t| {
                t.sample_elevation_and_slope(hub_pt[0] as f32, hub_pt[1] as f32)
                    .0
            })
            .unwrap_or(ref_z);
        if tin.is_some() && ground_msl < sea_level_m + 0.35 {
            continue;
        }
        let elev_z_rel_m = ground_msl - ref_z;

        let kind = match branch_count {
            4.. => {
                if cfg.prefer_roundabouts {
                    IntersectionKind::Roundabout
                } else {
                    IntersectionKind::FourWayCross
                }
            }
            3 => {
                if cfg.prefer_roundabouts {
                    IntersectionKind::Roundabout
                } else {
                    IntersectionKind::TJunction
                }
            }
            _ => IntersectionKind::MergeJunction,
        };

        let apron_radius_m = match kind {
            IntersectionKind::Roundabout => (max_w * 0.95 + 5.5).clamp(9.0, 20.0),
            IntersectionKind::FourWayCross => (max_w * 0.72 + 2.5).clamp(6.5, 16.0),
            IntersectionKind::TJunction => (max_w * 0.65 + 2.0).clamp(5.8, 14.5),
            IntersectionKind::MergeJunction => (max_w * 0.58 + 1.5).clamp(5.0, 12.0),
        };
        let island_radius_m = if kind == IntersectionKind::Roundabout {
            (apron_radius_m * 0.44).clamp(3.6, 8.2)
        } else {
            0.0
        };

        // HCM 6th Edition control delay approximation
        let capacity_vph = (branch_cap_vph * if kind == IntersectionKind::Roundabout { 1.12 } else { 1.0 })
            .max(950.0);
        let peak_vph = (urban_trip_vph * (0.45 + 0.14 * branch_count as f64) / cfg.peak_hour_factor)
            .max(180.0);
        let v_c = (peak_vph / capacity_vph).clamp(0.05, 1.45);
        let base_delay = if kind == IntersectionKind::Roundabout {
            6.5 + 28.0 * v_c.powf(2.2)
        } else if kind == IntersectionKind::MergeJunction {
            5.0 + 18.0 * v_c.powf(2.0)
        } else {
            11.0 + 36.0 * v_c.powf(2.3)
        };
        let avg_delay_sec = (base_delay * 10.0).round() / 10.0;
        let los = if avg_delay_sec <= 10.0 {
            IntersectionLosGrade::LosA
        } else if avg_delay_sec <= 20.0 {
            IntersectionLosGrade::LosB
        } else if avg_delay_sec <= 35.0 {
            IntersectionLosGrade::LosC
        } else if avg_delay_sec <= 55.0 {
            IntersectionLosGrade::LosD
        } else if avg_delay_sec <= 80.0 {
            IntersectionLosGrade::LosE
        } else {
            IntersectionLosGrade::LosF
        };

        let id = intersections.len() + 1;
        intersections.push(CityRoadIntersection {
            id,
            name: format!("INT-{id:02}"),
            kind,
            center_enu_m: hub_pt,
            elev_z_rel_m,
            connected_road_ids: road_ids,
            approach_bearings_deg: bearings,
            branch_count,
            max_road_width_m: max_w,
            apron_radius_m,
            island_radius_m,
            peak_vph,
            capacity_vph,
            v_over_c_ratio: v_c,
            avg_delay_sec,
            los,
            signal_cycle_s: if matches!(
                kind,
                IntersectionKind::FourWayCross | IntersectionKind::TJunction
            ) {
                75 + (branch_count as u32 * 10)
            } else {
                0
            },
        });
    }

    let total_intersections = intersections.len();
    let roundabout_count = intersections
        .iter()
        .filter(|i| i.kind == IntersectionKind::Roundabout)
        .count();
    let signalized_count = intersections
        .iter()
        .filter(|i| matches!(i.kind, IntersectionKind::FourWayCross | IntersectionKind::TJunction))
        .count();
    let worst_los = intersections
        .iter()
        .map(|i| i.los)
        .max()
        .unwrap_or(IntersectionLosGrade::LosA);
    let avg_delay_sec = if intersections.is_empty() {
        0.0
    } else {
        intersections.iter().map(|i| i.avg_delay_sec).sum::<f64>() / intersections.len() as f64
    };
    let all_los_acceptable = intersections.iter().all(|i| i.los.is_acceptable());
    let summary_verdict = if intersections.is_empty() {
        "Road Intersections: 0 junctions detected (intersect or snap >=2 roads)".to_string()
    } else {
        format!(
            "Road Intersections: {} junction(s) ({} signalized, {} roundabout) • Avg Delay={:.1}s • Worst {}",
            total_intersections,
            signalized_count,
            roundabout_count,
            avg_delay_sec,
            worst_los.short_label(),
        )
    };

    CityIntersectionNetworkReport {
        config: cfg,
        intersections,
        total_intersections,
        roundabout_count,
        signalized_count,
        worst_los,
        avg_delay_sec,
        all_los_acceptable,
        summary_verdict,
    }
}

/// Promotes a procedural `IfcxSpatialZone` city building into a full 6-DOF FEM-ready OpenBIM `BimProject`.
pub fn promote_city_building_to_bim(
    zone: &IfcxSpatialZone,
    city_origin_lat_lon: (f64, f64),
) -> Option<(BimProject, CityToBimPromotionSummary)> {
    let bldg = zone.synthesize_procedural_building()?;
    let stories = bldg.floors.clamp(1, 14);
    let story_h_m = (bldg.tower_height_m as f64 / bldg.floors.max(1) as f64).clamp(3.0, 4.5);
    let podium_stories = ((bldg.podium_height_m as f64 / story_h_m).round() as usize)
        .clamp(1, stories);

    let pw = bldg.podium_width_m.clamp(10.0, 48.0);
    let pd = bldg.podium_depth_m.clamp(10.0, 44.0);
    let tw = bldg.tower_width_m.clamp(8.0, pw);
    let td = bldg.tower_depth_m.clamp(8.0, pd);

    let mut blocks_3d = vec![MassingVolumeBlock3D {
        label: "Podium".to_string(),
        tier_index: 0,
        is_core: false,
        is_courtyard_void: false,
        start_story: 0,
        end_story: podium_stories,
        min_enu_m: [-pw * 0.5, -pd * 0.5, 0.0],
        max_enu_m: [pw * 0.5, pd * 0.5, podium_stories as f64 * story_h_m],
        color_rgba: bldg.wall_tint_rgba,
    }];
    if stories > podium_stories {
        blocks_3d.push(MassingVolumeBlock3D {
            label: "Tower".to_string(),
            tier_index: 1,
            is_core: false,
            is_courtyard_void: false,
            start_story: podium_stories,
            end_story: stories,
            min_enu_m: [-tw * 0.5, -td * 0.5, podium_stories as f64 * story_h_m],
            max_enu_m: [tw * 0.5, td * 0.5, stories as f64 * story_h_m],
            color_rgba: bldg.wall_tint_rgba,
        });
    }
    blocks_3d.push(MassingVolumeBlock3D {
        label: "RC Core".to_string(),
        tier_index: 0,
        is_core: true,
        is_courtyard_void: false,
        start_story: 0,
        end_story: stories,
        min_enu_m: [-2.7, -2.7, 0.0],
        max_enu_m: [2.7, 2.7, stories as f64 * story_h_m],
        color_rgba: [0.55, 0.62, 0.72, 0.85],
    });

    let opt = MassingOptionMetrics {
        archetype: MassingArchetype::PodiumTower,
        stories,
        podium_stories,
        total_height_m: stories as f64 * story_h_m,
        footprint_area_m2: pw * pd,
        gfa_m2: bldg.gfa_m2,
        nfa_m2: bldg.gfa_m2 * 0.80,
        efficiency_pct: 80.0,
        kdb_pct: 55.0,
        klb_far: (bldg.gfa_m2 / (zone.width_m * zone.depth_m).max(1.0)),
        daylight_score_pct: 82.0,
        estimated_eigengate_ari: 0.86,
        estimated_max_dcr_pct: 64.0,
        estimated_cost_usd: bldg.gfa_m2 * 1150.0,
        estimated_cost_idr_billion: bldg.gfa_m2 * 0.018,
        embodied_carbon_tco2e: bldg.gfa_m2 * 0.38,
        carbon_intensity_kg_m2: 380.0,
        zoning_compliant: true,
        program_fit_pct: 94.0,
        composite_score: 90.0,
        blocks_3d,
        courtyard_hole_enu_m: None,
        rationale: format!("Promoted from city parcel {}", zone.name),
    };

    let synth_report = MassingStudyReport {
        config: MassingStudyConfig {
            selected_archetype: MassingArchetype::PodiumTower,
            structural_system: MassingStructuralSystem::RcMomentCore,
            bay_x_m: (pw / 3.0).clamp(5.0, 9.0),
            bay_y_m: (pd / 3.0).clamp(5.0, 9.0),
            core_wall_thick_mm: 300.0,
            facade_wwr_pct: 42.0,
            include_basement_footings: true,
            show_structural_grid_preview: true,
        },
        site_label: zone.name.clone(),
        story_height_m: story_h_m,
        target_gfa_m2: bldg.gfa_m2,
        zoning_max_gfa_m2: bldg.gfa_m2 * 1.25,
        lot_area_m2: zone.width_m * zone.depth_m,
        buildable_pad_width_m: pw,
        buildable_pad_depth_m: pd,
        options: vec![opt],
        selected_index: 0,
        recommended_index: 0,
        summary_verdict: format!("Promoted {}", bldg.name),
    };

    let (mut bim, m_sum) = convert_massing_option_to_bim(&synth_report, 0);
    bim.project_name = format!("{} ({})", bldg.name, zone.kind.label());

    let target_lat_lon = SlippyMapState::world_mm_to_lat_lon(
        city_origin_lat_lon,
        [zone.center_enu_m[0] * 1000.0, zone.center_enu_m[1] * 1000.0],
    );

    let summary_line = format!(
        "Promoted {} ({}, {}F, {:.0} m² GFA) -> OpenBIM ({} cols, {} beams, {} walls)",
        bldg.name,
        zone.kind.label(),
        stories,
        m_sum.gfa_m2,
        m_sum.columns,
        m_sum.beams,
        m_sum.walls,
    );
    let summary = CityToBimPromotionSummary {
        zone_id: zone.id,
        building_name: bldg.name,
        zone_kind_label: zone.kind.label().to_string(),
        floors: stories,
        gfa_m2: m_sum.gfa_m2,
        target_lat_lon,
        heading_deg: zone.rotation_deg,
        levels: m_sum.levels,
        columns: m_sum.columns,
        beams: m_sum.beams,
        walls: m_sum.walls,
        openings: m_sum.openings,
        slabs: m_sum.slabs,
        footings: m_sum.footings,
        summary_line,
    };

    Some((bim, summary))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::eigengate::EigenGateEngine;
    use crate::core::gis::city_builder::IfcxZoneKind;
    use crate::core::ifcx::{ns, IfcxDocument, IfcxStage};

    #[test]
    fn test_v0878_city_to_bim_promotion_and_smart_road_intersections() {
        let mut city = IfcxCityScene::default();
        // Cross intersection at (0, 0) + 3-branch T-junction at (45, 0)
        city.add_road([-60.0, 0.0], [80.0, 0.0], IfcxRoadKind::Arterial, false);
        city.add_road([0.0, -55.0], [0.0, 55.0], IfcxRoadKind::Collector, false);
        city.add_road([45.0, 0.0], [45.0, 48.0], IfcxRoadKind::LocalStreet, false);
        let z = city
            .add_zone([24.0, 22.0], 26.0, 22.0, 15.0, IfcxZoneKind::Commercial)
            .clone();

        let rep_sig = detect_and_evaluate_road_intersections(
            &city,
            None,
            0.0,
            &CityIntersectionConfig::default(),
        );
        assert_eq!(rep_sig.total_intersections, 2);
        assert!(rep_sig
            .intersections
            .iter()
            .any(|i| i.kind == IntersectionKind::FourWayCross));
        assert!(rep_sig
            .intersections
            .iter()
            .any(|i| i.kind == IntersectionKind::TJunction));
        assert!(rep_sig.avg_delay_sec > 5.0);

        let rep_rb = detect_and_evaluate_road_intersections(
            &city,
            None,
            0.0,
            &CityIntersectionConfig {
                prefer_roundabouts: true,
                ..Default::default()
            },
        );
        assert_eq!(rep_rb.roundabout_count, 2);
        assert!(rep_rb.intersections.iter().all(|i| i.island_radius_m > 3.0));

        // Promote parcel into full 6-DOF FEM-solvable OpenBIM BimProject
        let (bim, summary) =
            promote_city_building_to_bim(&z, (-6.2088, 106.8456)).expect("promote zone");
        assert!(summary.columns >= 6 && summary.beams >= 6);
        let model = bim.to_structural_model();
        let eval = EigenGateEngine::evaluate_redundancy(&model).expect("6-DOF FEM solves promoted city building");
        assert!(eval.kinematic_guard.is_stable);

        // IFCX overlay composition
        let base_doc = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed =
            IfcxStage::compose_with_city_intersections_overlay(&base_doc, &rep_rb.to_ifcx_overlay());
        let node = composed
            .data
            .iter()
            .find(|n| n.path == "/earth/city/intersections")
            .expect("intersections IFCX node");
        assert!(node.attributes.contains_key(ns::EG_CITY_INTERSECTIONS));
    }
}
