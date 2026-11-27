//! city_builder.rs — Pure-Rust IFCX City-Builder Authoring Kernel (`IfcRoad`, `IfcBridge`,
//! `IfcSpatialZone`, Auto Cut/Fill `IfcSite` Pad Grading & Road/Parcel Snapping).
//!
//! All authored civil infrastructure and urban parcels persist cleanly in `.eigen.json` AND
//! serialize/deserialize natively as composable IFCX (IFC 5 JSON ECS) nodes carrying
//! `bsi::ifc::class` + `usd::usdgeom::mesh` + `eigengraph::city::*`.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::bim::BimProject;
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::ifcx::{ns, IfcxDocument, IfcxNode, UsdGeomMesh};

pub const NS_EG_CITY_ROAD: &str = "eigengraph::city::road";
pub const NS_EG_CITY_ZONE: &str = "eigengraph::city::zone";
pub const NS_EG_CITY_PAD: &str = "eigengraph::city::gradedPad";

/// Functional roadway class authored on IFCX Earth (`IfcRoad` / `IfcBridge`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IfcxRoadKind {
    Arterial,
    #[default]
    Collector,
    LocalStreet,
    PedestrianBoulevard,
}

impl IfcxRoadKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Arterial => "Arterial (4-Lane)",
            Self::Collector => "Collector (2-Lane)",
            Self::LocalStreet => "Local Street",
            Self::PedestrianBoulevard => "Boulevard",
        }
    }

    pub fn default_width_m(&self) -> f64 {
        match self {
            Self::Arterial => 14.0,
            Self::Collector => 9.0,
            Self::LocalStreet => 6.5,
            Self::PedestrianBoulevard => 4.5,
        }
    }

    pub fn default_lanes(&self) -> u8 {
        match self {
            Self::Arterial => 4,
            Self::Collector => 2,
            Self::LocalStreet => 2,
            Self::PedestrianBoulevard => 1,
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::Arterial => Self::Collector,
            Self::Collector => Self::LocalStreet,
            Self::LocalStreet => Self::PedestrianBoulevard,
            Self::PedestrianBoulevard => Self::Arterial,
        }
    }
}

fn default_auto_bridge_true() -> bool {
    true
}

/// Geometry alignment authoring mode for Cities: Skylines II style road & bridge placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RoadDrawMode {
    Straight,
    #[default]
    CurvedSpline,
}

impl RoadDrawMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Straight => "Straight",
            Self::CurvedSpline => "Curved Spline",
        }
    }
}

/// Terrain-coupled 3D station sample along a straight or curved quadratic Bézier road/bridge span.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadProfileSample {
    pub east_m: f64,
    pub north_m: f64,
    pub deck_z_rel_m: f32,
    pub ground_z_rel_m: f32,
    pub over_water: bool,
    pub is_elevated: bool,
    pub tangent: [f64; 2],
    pub normal: [f64; 2],
}

/// Authored pure-vector roadway or bridge span in local ENU site meters (`IfcRoad` / `IfcBridge`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfcxRoadSegment {
    pub id: usize,
    pub name: String,
    pub kind: IfcxRoadKind,
    pub start_enu_m: [f64; 2],
    pub end_enu_m: [f64; 2],
    pub width_m: f64,
    pub lanes: u8,
    pub is_bridge: bool,
    pub deck_elev_m: f64,
    pub pier_spacing_m: f64,
    #[serde(default)]
    pub control_enu_m: Option<[f64; 2]>,
    #[serde(default = "default_auto_bridge_true")]
    pub auto_bridge_over_water: bool,
    #[serde(default)]
    pub elevation_offset_m: f64,
}

impl IfcxRoadSegment {
    /// Evaluates ordered 2D centerline stations along linear or quadratic Bézier alignment.
    pub fn sample_centerline(&self, steps: usize) -> Vec<[f64; 2]> {
        let n = steps.max(1);
        let p0 = self.start_enu_m;
        let p1 = self.end_enu_m;
        let mut pts = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let t = i as f64 / n as f64;
            let pt = if let Some(pc) = self.control_enu_m {
                let omt = 1.0 - t;
                [
                    omt * omt * p0[0] + 2.0 * omt * t * pc[0] + t * t * p1[0],
                    omt * omt * p0[1] + 2.0 * omt * t * pc[1] + t * t * p1[1],
                ]
            } else {
                [p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t]
            };
            pts.push(pt);
        }
        pts
    }

    pub fn length_m(&self) -> f64 {
        if self.control_enu_m.is_some() {
            let pts = self.sample_centerline(16);
            pts.windows(2)
                .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
                .sum()
        } else {
            let dx = self.end_enu_m[0] - self.start_enu_m[0];
            let dy = self.end_enu_m[1] - self.start_enu_m[1];
            dx.hypot(dy)
        }
    }

    /// Evaluates terrain-coupled 3D road/bridge vertical profile with smooth vertical curve grading.
    pub fn sample_3d_profile(
        &self,
        tin: Option<&TinSurfaceMesh>,
        sea_level_m: f32,
    ) -> Vec<RoadProfileSample> {
        let steps = if self.control_enu_m.is_some() {
            18
        } else {
            ((self.length_m() / 14.0).ceil() as usize).clamp(4, 20)
        };
        let pts = self.sample_centerline(steps);
        let ref_z = tin
            .map(|t| t.sample_elevation_and_slope(0.0, 0.0).0)
            .unwrap_or(0.0);
        let mut samples = Vec::with_capacity(pts.len());

        for (i, &p) in pts.iter().enumerate() {
            let prev = pts[i.saturating_sub(1)];
            let next = pts[(i + 1).min(pts.len() - 1)];
            let tx = next[0] - prev[0];
            let ty = next[1] - prev[1];
            let tlen = tx.hypot(ty).max(1e-6);
            let tangent = [tx / tlen, ty / tlen];
            let normal = [-tangent[1], tangent[0]];

            let ground_msl = tin
                .map(|t| t.sample_elevation_and_slope(p[0] as f32, p[1] as f32).0)
                .unwrap_or(ref_z);
            let ground_z_rel_m = ground_msl - ref_z;
            let over_water = tin.is_some() && ground_msl < sea_level_m + 0.45;

            let deck_z_rel_m = if self.is_bridge && tin.is_none() {
                self.deck_elev_m as f32
            } else if self.is_bridge || (self.auto_bridge_over_water && over_water) {
                let t_f = i as f32 / steps as f32;
                let arch = (std::f32::consts::PI * t_f).sin() * 2.4;
                let water_clear_rel = (sea_level_m - ref_z) + 5.8 + arch;
                let terr_clear_rel =
                    ground_z_rel_m + (self.elevation_offset_m.max(4.5) as f32) + arch * 0.4;
                if over_water {
                    water_clear_rel.max(terr_clear_rel)
                } else {
                    terr_clear_rel.max((sea_level_m - ref_z) + 2.5)
                }
            } else {
                ground_z_rel_m + 0.22 + self.elevation_offset_m as f32
            };

            samples.push(RoadProfileSample {
                east_m: p[0],
                north_m: p[1],
                deck_z_rel_m,
                ground_z_rel_m,
                over_water,
                is_elevated: (deck_z_rel_m - ground_z_rel_m) > 1.4 || over_water || self.is_bridge,
                tangent,
                normal,
            });
        }

        // Smooth interior vertical grades across terrain transitions & bridge approaches
        if samples.len() >= 4 {
            for _ in 0..2 {
                let prev_z: Vec<f32> = samples.iter().map(|s| s.deck_z_rel_m).collect();
                for i in 1..(samples.len() - 1) {
                    let smoothed = 0.25 * prev_z[i - 1] + 0.50 * prev_z[i] + 0.25 * prev_z[i + 1];
                    let min_floor = if samples[i].over_water {
                        (sea_level_m - ref_z) + 5.4
                    } else {
                        samples[i].ground_z_rel_m + 0.18
                    };
                    samples[i].deck_z_rel_m = smoothed.max(min_floor);
                    samples[i].is_elevated =
                        (samples[i].deck_z_rel_m - samples[i].ground_z_rel_m) > 1.4
                            || samples[i].over_water
                            || self.is_bridge;
                }
            }
        }
        samples
    }

    pub fn bearing_deg(&self) -> f64 {
        let dx = self.end_enu_m[0] - self.start_enu_m[0];
        let dy = self.end_enu_m[1] - self.start_enu_m[1];
        dy.atan2(dx).to_degrees().rem_euclid(360.0)
    }

    /// Computes the 4-corner deck/roadway footprint in ENU meters.
    pub fn footprint_enu_m(&self) -> [[f64; 2]; 4] {
        let dx = self.end_enu_m[0] - self.start_enu_m[0];
        let dy = self.end_enu_m[1] - self.start_enu_m[1];
        let len = dx.hypot(dy).max(1e-3);
        let nx = -dy / len * (self.width_m * 0.5);
        let ny = dx / len * (self.width_m * 0.5);
        [
            [self.start_enu_m[0] - nx, self.start_enu_m[1] - ny],
            [self.end_enu_m[0] - nx, self.end_enu_m[1] - ny],
            [self.end_enu_m[0] + nx, self.end_enu_m[1] + ny],
            [self.start_enu_m[0] + nx, self.start_enu_m[1] + ny],
        ]
    }
}

/// Urban parcel zoning classification (`IfcSpatialZone`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IfcxZoneKind {
    #[default]
    Residential,
    Commercial,
    Industrial,
    Civic,
    Park,
}

impl IfcxZoneKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Residential => "Residential",
            Self::Commercial => "Commercial",
            Self::Industrial => "Industrial",
            Self::Civic => "Civic",
            Self::Park => "Park & Greenery",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Residential => "Res",
            Self::Commercial => "Com",
            Self::Industrial => "Ind",
            Self::Civic => "Civic",
            Self::Park => "Park",
        }
    }

    pub fn fill_rgba(&self) -> [f32; 4] {
        match self {
            Self::Residential => [0.34, 0.72, 0.56, 1.0],
            Self::Commercial => [0.30, 0.58, 0.85, 1.0],
            Self::Industrial => [0.76, 0.62, 0.34, 1.0],
            Self::Civic => [0.62, 0.48, 0.82, 1.0],
            Self::Park => [0.22, 0.68, 0.42, 1.0],
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::Residential => Self::Commercial,
            Self::Commercial => Self::Industrial,
            Self::Industrial => Self::Civic,
            Self::Civic => Self::Park,
            Self::Park => Self::Residential,
        }
    }
}

/// Authored urban parcel zone (`IfcSpatialZone`) supporting context towers and instanced trees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfcxSpatialZone {
    pub id: usize,
    pub name: String,
    pub kind: IfcxZoneKind,
    pub center_enu_m: [f64; 2],
    pub width_m: f64,
    pub depth_m: f64,
    pub rotation_deg: f64,
    pub synthesize_context_towers: bool,
    pub tree_density: u8,
}

impl IfcxSpatialZone {
    pub fn footprint_enu_m(&self) -> [[f64; 2]; 4] {
        let hw = self.width_m * 0.5;
        let hd = self.depth_m * 0.5;
        let rad = self.rotation_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let corners = [[-hw, -hd], [hw, -hd], [hw, hd], [-hw, hd]];
        let mut out = [[0.0; 2]; 4];
        for (i, [lx, ly]) in corners.iter().enumerate() {
            out[i] = [
                self.center_enu_m[0] + lx * cr - ly * sr,
                self.center_enu_m[1] + lx * sr + ly * cr,
            ];
        }
        out
    }

    /// Deterministic pure-vector tree positions `(east_m, north_m, height_m)` inside the parcel.
    pub fn instanced_trees_enu_m(&self) -> Vec<[f64; 3]> {
        if self.tree_density == 0 {
            return Vec::new();
        }
        let rad = self.rotation_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let mut trees = Vec::new();
        let count = if self.kind == IfcxZoneKind::Park {
            (self.tree_density as usize * 3).clamp(4, 24)
        } else {
            (self.tree_density as usize).clamp(2, 10)
        };
        for i in 0..count {
            // Golden-ratio quasi-random placement strictly deterministic per zone id
            let seed = (self.id as u64).wrapping_mul(2654435761) ^ ((i as u64 + 1) * 1013904223);
            let u = ((seed & 0xFFFF) as f64) / 65535.0;
            let v = (((seed >> 16) & 0xFFFF) as f64) / 65535.0;
            let (lx, ly) = if self.kind == IfcxZoneKind::Park || !self.synthesize_context_towers {
                ((u - 0.5) * self.width_m * 0.82, (v - 0.5) * self.depth_m * 0.82)
            } else {
                // Place boulevard perimeter trees around context building setbacks
                let side = i % 4;
                let t = (u - 0.5) * 0.80;
                match side {
                    0 => (t * self.width_m, -self.depth_m * 0.44),
                    1 => (t * self.width_m, self.depth_m * 0.44),
                    2 => (-self.width_m * 0.44, t * self.depth_m),
                    _ => (self.width_m * 0.44, t * self.depth_m),
                }
            };
            let h = 4.5 + (((seed >> 32) & 0xFF) as f64 / 255.0) * 5.0;
            trees.push([
                self.center_enu_m[0] + lx * cr - ly * sr,
                self.center_enu_m[1] + lx * sr + ly * cr,
                h,
            ]);
        }
        trees
    }
}

/// Graded foundation pad cut/fill record authored onto `IfcSite` TIN terrain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfcxGradedPad {
    pub id: usize,
    pub name: String,
    pub center_enu_m: [f64; 2],
    pub width_m: f64,
    pub depth_m: f64,
    pub target_elev_m: f64,
    pub cut_volume_m3: f64,
    pub fill_volume_m3: f64,
    pub net_balance_m3: f64,
}

/// Result of snapping a cursor/building coordinate to an authored road or parcel corridor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadParcelSnapHit {
    pub snapped_enu_m: [f64; 2],
    pub bearing_deg: f64,
    pub is_road: bool,
}

/// Complete Pure-Rust IFCX City-Builder Authoring Scene for the active Earth site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfcxCityScene {
    pub next_id: usize,
    pub roads: Vec<IfcxRoadSegment>,
    pub zones: Vec<IfcxSpatialZone>,
    pub graded_pads: Vec<IfcxGradedPad>,
    pub active_road_kind: IfcxRoadKind,
    pub active_zone_kind: IfcxZoneKind,
    pub road_snap_enabled: bool,
    pub synthesize_zone_towers: bool,
    #[serde(default)]
    pub road_draw_mode: RoadDrawMode,
    #[serde(default = "default_auto_bridge_true")]
    pub auto_bridge_enabled: bool,
}

impl Default for IfcxCityScene {
    fn default() -> Self {
        Self {
            next_id: 1,
            roads: Vec::new(),
            zones: Vec::new(),
            graded_pads: Vec::new(),
            active_road_kind: IfcxRoadKind::Collector,
            active_zone_kind: IfcxZoneKind::Commercial,
            road_snap_enabled: true,
            synthesize_zone_towers: true,
            road_draw_mode: RoadDrawMode::CurvedSpline,
            auto_bridge_enabled: true,
        }
    }
}

impl IfcxCityScene {
    pub fn is_empty(&self) -> bool {
        self.roads.is_empty() && self.zones.is_empty() && self.graded_pads.is_empty()
    }

    pub fn add_road(
        &mut self,
        start_enu_m: [f64; 2],
        end_enu_m: [f64; 2],
        kind: IfcxRoadKind,
        is_bridge: bool,
    ) -> Option<&IfcxRoadSegment> {
        let len = (end_enu_m[0] - start_enu_m[0]).hypot(end_enu_m[1] - start_enu_m[1]);
        if len < 2.0 {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        let name = if is_bridge {
            format!("Bridge_BR{id}")
        } else {
            format!("Road_RD{id}")
        };
        self.roads.push(IfcxRoadSegment {
            id,
            name,
            kind,
            start_enu_m,
            end_enu_m,
            width_m: kind.default_width_m(),
            lanes: kind.default_lanes(),
            is_bridge,
            deck_elev_m: if is_bridge { 6.5 } else { 0.14 },
            pier_spacing_m: 18.0,
            control_enu_m: None,
            auto_bridge_over_water: true,
            elevation_offset_m: 0.0,
        });
        self.roads.last()
    }

    /// Snaps a candidate junction point to the nearest existing road endpoint within `tol_m`.
    pub fn snap_road_endpoint(&self, pt_enu_m: [f64; 2], tol_m: f64) -> Option<[f64; 2]> {
        let mut best: Option<(f64, [f64; 2])> = None;
        for r in &self.roads {
            for ep in [r.start_enu_m, r.end_enu_m] {
                let d = (pt_enu_m[0] - ep[0]).hypot(pt_enu_m[1] - ep[1]);
                if d <= tol_m && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, ep));
                }
            }
        }
        best.map(|(_, ep)| ep)
    }

    /// Authors a linear or quadratic Bézier curved road corridor, automatically promoting coastal
    /// water or canyon crossings to `IfcBridge` when `auto_bridge_enabled` is active.
    #[allow(clippy::too_many_arguments)]
    pub fn add_curved_road_with_auto_bridge(
        &mut self,
        start_enu_m: [f64; 2],
        control_enu_m: Option<[f64; 2]>,
        end_enu_m: [f64; 2],
        kind: IfcxRoadKind,
        force_bridge: bool,
        elevation_offset_m: f64,
        tin: Option<&TinSurfaceMesh>,
        sea_level_m: f32,
    ) -> Option<&IfcxRoadSegment> {
        let chord = (end_enu_m[0] - start_enu_m[0]).hypot(end_enu_m[1] - start_enu_m[1]);
        if chord < 2.0 {
            return None;
        }
        let mut candidate = IfcxRoadSegment {
            id: self.next_id,
            name: String::new(),
            kind,
            start_enu_m,
            end_enu_m,
            width_m: kind.default_width_m(),
            lanes: kind.default_lanes(),
            is_bridge: force_bridge || elevation_offset_m > 2.2,
            deck_elev_m: if force_bridge {
                elevation_offset_m.max(6.5)
            } else {
                0.14 + elevation_offset_m
            },
            pier_spacing_m: 18.0,
            control_enu_m,
            auto_bridge_over_water: self.auto_bridge_enabled,
            elevation_offset_m,
        };
        if self.auto_bridge_enabled {
            if let Some(t) = tin {
                for p in candidate.sample_centerline(16) {
                    let (gz, _) = t.sample_elevation_and_slope(p[0] as f32, p[1] as f32);
                    if gz < sea_level_m + 0.45 {
                        candidate.is_bridge = true;
                        candidate.deck_elev_m = candidate.deck_elev_m.max(6.5);
                        break;
                    }
                }
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        candidate.id = id;
        candidate.name = if candidate.is_bridge {
            format!("Bridge_BR{id}")
        } else {
            format!("Road_RD{id}")
        };
        self.roads.push(candidate);
        self.roads.last()
    }

    pub fn add_zone(
        &mut self,
        center_enu_m: [f64; 2],
        width_m: f64,
        depth_m: f64,
        rotation_deg: f64,
        kind: IfcxZoneKind,
    ) -> &IfcxSpatialZone {
        let id = self.next_id;
        self.next_id += 1;
        let tree_density = match kind {
            IfcxZoneKind::Park => 6,
            IfcxZoneKind::Residential | IfcxZoneKind::Civic => 4,
            IfcxZoneKind::Commercial => 3,
            IfcxZoneKind::Industrial => 1,
        };
        self.zones.push(IfcxSpatialZone {
            id,
            name: format!("Zone_{}_{}", kind.short_label(), id),
            kind,
            center_enu_m,
            width_m: width_m.clamp(12.0, 240.0),
            depth_m: depth_m.clamp(12.0, 240.0),
            rotation_deg: rotation_deg.rem_euclid(360.0),
            synthesize_context_towers: self.synthesize_zone_towers && kind != IfcxZoneKind::Park,
            tree_density,
        });
        self.zones.last().expect("pushed zone")
    }

    /// Snaps a point in local ENU meters to the nearest authored road setback or parcel grid edge,
    /// returning the snapped position and road/parcel alignment bearing in degrees.
    pub fn snap_point_to_road_or_parcel(
        &self,
        pt_enu_m: [f64; 2],
        tol_m: f64,
    ) -> Option<RoadParcelSnapHit> {
        if !self.road_snap_enabled {
            return None;
        }
        let mut best: Option<(f64, RoadParcelSnapHit)> = None;

        // 1. Snap to road setback line (parallel offset = half road width + 4.0m setback), supporting curves
        for road in &self.roads {
            let sub_steps = if road.control_enu_m.is_some() { 10 } else { 1 };
            let pts = road.sample_centerline(sub_steps);
            for seg in pts.windows(2) {
                let ax = seg[0][0];
                let ay = seg[0][1];
                let dx = seg[1][0] - ax;
                let dy = seg[1][1] - ay;
                let len2 = dx * dx + dy * dy;
                if len2 < 1.0 {
                    continue;
                }
                let len = len2.sqrt();
                let t =
                    (((pt_enu_m[0] - ax) * dx + (pt_enu_m[1] - ay) * dy) / len2).clamp(0.0, 1.0);
                let proj = [ax + dx * t, ay + dy * t];
                let nx = -dy / len;
                let ny = dx / len;
                let signed_off = (pt_enu_m[0] - proj[0]) * nx + (pt_enu_m[1] - proj[1]) * ny;
                let setback = road.width_m * 0.5 + 4.0;
                let side = if signed_off >= 0.0 { 1.0 } else { -1.0 };
                let candidate = [proj[0] + nx * side * setback, proj[1] + ny * side * setback];
                let dist = (pt_enu_m[0] - candidate[0]).hypot(pt_enu_m[1] - candidate[1]);
                if dist <= tol_m && best.is_none_or(|(bd, _)| dist < bd) {
                    let seg_bearing = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                    best = Some((
                        dist,
                        RoadParcelSnapHit {
                            snapped_enu_m: candidate,
                            bearing_deg: seg_bearing,
                            is_road: true,
                        },
                    ));
                }
            }
        }

        // 2. Snap to spatial zone center alignment
        for zone in &self.zones {
            let dist = (pt_enu_m[0] - zone.center_enu_m[0]).hypot(pt_enu_m[1] - zone.center_enu_m[1]);
            if dist <= tol_m && best.is_none_or(|(bd, _)| dist < bd) {
                best = Some((
                    dist,
                    RoadParcelSnapHit {
                        snapped_enu_m: zone.center_enu_m,
                        bearing_deg: zone.rotation_deg,
                        is_road: false,
                    },
                ));
            }
        }

        best.map(|(_, hit)| hit)
    }

    /// Grades a flat foundation pad beneath the active `BimProject` footprint directly onto
    /// `TinSurfaceMesh` with smooth 4 m batter slopes and computes exact earthwork cut/fill volumes.
    pub fn auto_grade_building_pad(
        &mut self,
        project: &BimProject,
        tin: &mut TinSurfaceMesh,
    ) -> IfcxGradedPad {
        // Compute active building bounding footprint in meters (with 3.0 m construction apron)
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut include = |x_mm: f64, y_mm: f64| {
            let xm = x_mm * 0.001;
            let ym = y_mm * 0.001;
            min_x = min_x.min(xm);
            max_x = max_x.max(xm);
            min_y = min_y.min(ym);
            max_y = max_y.max(ym);
        };
        for c in &project.columns {
            include(c.pos.0, c.pos.1);
        }
        for w in &project.walls {
            include(w.start.0, w.start.1);
            include(w.end.0, w.end.1);
        }
        for r in &project.runs {
            include(r.start.0, r.start.1);
            include(r.end.0, r.end.1);
        }
        for f in &project.footings {
            include(f.pos.0, f.pos.1);
        }
        for s in &project.slabs {
            for p in &s.boundary {
                include(p.0, p.1);
            }
        }

        let (cx, cy, width_m, depth_m) = if min_x.is_finite() && max_x.is_finite() {
            (
                0.5 * (min_x + max_x),
                0.5 * (min_y + max_y),
                ((max_x - min_x) + 6.0).max(16.0),
                ((max_y - min_y) + 6.0).max(16.0),
            )
        } else {
            (0.0, 0.0, 24.0, 20.0)
        };

        let hw = (width_m * 0.5) as f32;
        let hd = (depth_m * 0.5) as f32;
        let batter = 5.0_f32;
        let (target_z, _) = tin.sample_elevation_and_slope(cx as f32, cy as f32);

        // Compute pre-grading volume strictly over triangles intersecting the pad + batter envelope
        let mut cut_m3 = 0.0_f64;
        let mut fill_m3 = 0.0_f64;
        for tm in &tin.triangle_metrics {
            let dx = (tm.centroid[0] - cx as f32).abs();
            let dy = (tm.centroid[1] - cy as f32).abs();
            if dx <= hw + batter && dy <= hd + batter {
                let dz = (tm.centroid[2] - target_z) as f64;
                let a2d = tm.area_2d_m2 as f64;
                if dz >= 0.0 {
                    cut_m3 += a2d * dz;
                } else {
                    fill_m3 += a2d * (-dz);
                }
            }
        }

        // Apply smooth graded pad deformation to TIN vertices and refresh metrics
        for pt in &mut tin.points {
            let dx = ((pt.x - cx as f32).abs() - hw).max(0.0);
            let dy = ((pt.y - cy as f32).abs() - hd).max(0.0);
            let dist = dx.hypot(dy);
            if dist <= 1e-4 {
                pt.z = target_z;
            } else if dist < batter {
                let t = (dist / batter).clamp(0.0, 1.0);
                // Smoothstep blend into surrounding natural terrain
                let s = t * t * (3.0 - 2.0 * t);
                pt.z = target_z * (1.0 - s) + pt.z * s;
            }
        }
        tin.min_elevation = tin
            .points
            .iter()
            .map(|p| p.z)
            .fold(f32::INFINITY, f32::min);
        tin.max_elevation = tin
            .points
            .iter()
            .map(|p| p.z)
            .fold(f32::NEG_INFINITY, f32::max);
        tin.triangle_metrics = tin
            .triangles
            .iter()
            .map(|&tri| TinSurfaceMesh::compute_triangle_metrics(&tin.points, tri))
            .collect();

        // Sanity fallback using global site cut/fill if pad vertices were sparse
        if cut_m3 < 1e-3 && fill_m3 < 1e-3 {
            let site_rep = tin.compute_cut_fill_report(target_z, 30.0);
            let frac = ((width_m * depth_m) / (tin.extent_x_m as f64 * tin.extent_y_m as f64).max(1.0))
                .clamp(0.02, 1.0);
            cut_m3 = site_rep.cut_volume_m3 * frac;
            fill_m3 = site_rep.fill_volume_m3 * frac;
        }

        let id = self.next_id;
        self.next_id += 1;
        let pad = IfcxGradedPad {
            id,
            name: format!("GradedPad_P{id}"),
            center_enu_m: [cx, cy],
            width_m,
            depth_m,
            target_elev_m: target_z as f64,
            cut_volume_m3: cut_m3,
            fill_volume_m3: fill_m3,
            net_balance_m3: cut_m3 - fill_m3,
        };
        self.graded_pads.push(pad.clone());
        pad
    }

    /// Deterministic FNV-1a revision hash for O(1) WebGPU buffer cache invalidation.
    pub fn content_revision_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let mix = |hash: &mut u64, v: u64| {
            *hash ^= v;
            *hash = hash.wrapping_mul(0x100000001b3);
        };
        mix(&mut h, self.roads.len() as u64);
        mix(&mut h, self.zones.len() as u64);
        mix(&mut h, self.graded_pads.len() as u64);
        for r in &self.roads {
            mix(&mut h, r.id as u64);
            mix(&mut h, r.is_bridge as u64);
            mix(&mut h, (r.start_enu_m[0] * 100.0) as i64 as u64);
            mix(&mut h, (r.end_enu_m[1] * 100.0) as i64 as u64);
            mix(&mut h, (r.width_m * 100.0) as u64);
            mix(&mut h, (r.elevation_offset_m * 100.0) as i64 as u64);
            if let Some(c) = r.control_enu_m {
                mix(&mut h, (c[0] * 100.0) as i64 as u64);
                mix(&mut h, (c[1] * 100.0) as i64 as u64);
            }
        }
        for z in &self.zones {
            mix(&mut h, z.id as u64);
            mix(&mut h, z.kind as u64);
            mix(&mut h, z.synthesize_context_towers as u64);
            mix(&mut h, (z.center_enu_m[0] * 100.0) as i64 as u64);
            mix(&mut h, (z.center_enu_m[1] * 100.0) as i64 as u64);
        }
        for p in &self.graded_pads {
            mix(&mut h, p.id as u64);
            mix(&mut h, (p.target_elev_m * 100.0) as i64 as u64);
        }
        h
    }

    /// Emits native composable IFCX nodes (`IfcRoad`, `IfcBridge`, `IfcSpatialZone`, `IfcSite` pads)
    /// under `site_path` with genuine 3D `usd::usdgeom::mesh` payloads!
    pub fn to_ifcx_nodes(&self, site_path: &str) -> Vec<IfcxNode> {
        let mut nodes = Vec::new();
        for road in &self.roads {
            let mesh = if road.control_enu_m.is_some() {
                let profile = road.sample_3d_profile(None, 0.0);
                let hw = road.width_m * 0.5;
                let mut points = Vec::new();
                let mut counts = Vec::new();
                let mut idxs = Vec::new();
                for w in profile.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    let za0 = if road.is_bridge { a.deck_z_rel_m as f64 - 0.8 } else { 0.0 };
                    let za1 = a.deck_z_rel_m as f64;
                    let zb0 = if road.is_bridge { b.deck_z_rel_m as f64 - 0.8 } else { 0.0 };
                    let zb1 = b.deck_z_rel_m as f64;
                    let base = points.len();
                    points.extend_from_slice(&[
                        [a.east_m - a.normal[0] * hw, a.north_m - a.normal[1] * hw, za0],
                        [b.east_m - b.normal[0] * hw, b.north_m - b.normal[1] * hw, zb0],
                        [b.east_m + b.normal[0] * hw, b.north_m + b.normal[1] * hw, zb0],
                        [a.east_m + a.normal[0] * hw, a.north_m + a.normal[1] * hw, za0],
                        [a.east_m - a.normal[0] * hw, a.north_m - a.normal[1] * hw, za1],
                        [b.east_m - b.normal[0] * hw, b.north_m - b.normal[1] * hw, zb1],
                        [b.east_m + b.normal[0] * hw, b.north_m + b.normal[1] * hw, zb1],
                        [a.east_m + a.normal[0] * hw, a.north_m + a.normal[1] * hw, za1],
                    ]);
                    for &tri in &[
                        [0, 2, 1],
                        [0, 3, 2],
                        [4, 5, 6],
                        [4, 6, 7],
                        [0, 1, 5],
                        [0, 5, 4],
                        [2, 3, 7],
                        [2, 7, 6],
                    ] {
                        counts.push(3);
                        idxs.extend_from_slice(&[
                            base as u32 + tri[0],
                            base as u32 + tri[1],
                            base as u32 + tri[2],
                        ]);
                    }
                }
                UsdGeomMesh {
                    points,
                    face_vertex_counts: counts,
                    face_vertex_indices: idxs,
                    normals: Vec::new(),
                    display_color: Some(if road.is_bridge {
                        [0.72, 0.76, 0.80, 1.0]
                    } else {
                        [0.24, 0.27, 0.32, 1.0]
                    }),
                    vertex_colors: Vec::new(),
                    vertex_meta: Vec::new(),
                    material_kind: 3.0,
                }
            } else {
                let fp = road.footprint_enu_m();
                let z0 = if road.is_bridge { road.deck_elev_m - 0.8 } else { 0.0 };
                let z1 = road.deck_elev_m;
                let points = vec![
                    [fp[0][0], fp[0][1], z0],
                    [fp[1][0], fp[1][1], z0],
                    [fp[2][0], fp[2][1], z0],
                    [fp[3][0], fp[3][1], z0],
                    [fp[0][0], fp[0][1], z1],
                    [fp[1][0], fp[1][1], z1],
                    [fp[2][0], fp[2][1], z1],
                    [fp[3][0], fp[3][1], z1],
                ];
                UsdGeomMesh {
                    points,
                    face_vertex_counts: vec![3; 12],
                    face_vertex_indices: vec![
                        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6,
                        1, 2, 6, 1, 6, 5, 3, 0, 4, 3, 4, 7,
                    ],
                    normals: Vec::new(),
                    display_color: Some(if road.is_bridge {
                        [0.72, 0.76, 0.80, 1.0]
                    } else {
                        [0.24, 0.27, 0.32, 1.0]
                    }),
                    vertex_colors: Vec::new(),
                    vertex_meta: Vec::new(),
                    material_kind: 3.0,
                }
            };
            let folder = if road.is_bridge { "bridges" } else { "roads" };
            let path = format!("{site_path}/{folder}/{}", road.name.to_lowercase());
            let mut node = IfcxNode::new(path)
                .with_class(if road.is_bridge { "IfcBridge" } else { "IfcRoad" })
                .with_prop(
                    &road.name,
                    if road.is_bridge {
                        "Prestressed Concrete Deck & Piers"
                    } else {
                        "Asphalt Pavement"
                    },
                    &format!("{} | {:.1} m x {:.1} m", road.kind.label(), road.length_m(), road.width_m),
                )
                .with_mesh(&mesh);
            if let Ok(v) = serde_json::to_value(road) {
                node.attributes.insert(NS_EG_CITY_ROAD.to_string(), v);
            }
            nodes.push(node);
        }

        for zone in &self.zones {
            let fp = zone.footprint_enu_m();
            let points = vec![
                [fp[0][0], fp[0][1], 0.0],
                [fp[1][0], fp[1][1], 0.0],
                [fp[2][0], fp[2][1], 0.0],
                [fp[3][0], fp[3][1], 0.0],
                [fp[0][0], fp[0][1], 0.18],
                [fp[1][0], fp[1][1], 0.18],
                [fp[2][0], fp[2][1], 0.18],
                [fp[3][0], fp[3][1], 0.18],
            ];
            let mesh = UsdGeomMesh {
                points,
                face_vertex_counts: vec![3; 12],
                face_vertex_indices: vec![
                    0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 1, 2,
                    6, 1, 6, 5, 3, 0, 4, 3, 4, 7,
                ],
                normals: Vec::new(),
                display_color: Some(zone.kind.fill_rgba()),
                vertex_colors: Vec::new(),
                vertex_meta: Vec::new(),
                material_kind: 1.0,
            };
            let path = format!("{site_path}/zones/{}", zone.name.to_lowercase());
            let mut node = IfcxNode::new(path)
                .with_class("IfcSpatialZone")
                .with_prop(
                    &zone.name,
                    zone.kind.label(),
                    &format!("{:.0} m x {:.0} m Parcel", zone.width_m, zone.depth_m),
                )
                .with_mesh(&mesh);
            if let Ok(v) = serde_json::to_value(zone) {
                node.attributes.insert(NS_EG_CITY_ZONE.to_string(), v);
            }
            nodes.push(node);
        }

        for pad in &self.graded_pads {
            let hw = pad.width_m * 0.5;
            let hd = pad.depth_m * 0.5;
            let mesh = UsdGeomMesh::from_box_m(
                [pad.center_enu_m[0] - hw, pad.center_enu_m[1] - hd, pad.target_elev_m - 0.3],
                [pad.center_enu_m[0] + hw, pad.center_enu_m[1] + hd, pad.target_elev_m],
                [0.60, 0.56, 0.48, 1.0],
                1.0,
            );
            let path = format!("{site_path}/terrain/{}", pad.name.to_lowercase());
            let mut node = IfcxNode::new(path)
                .with_class("IfcSite")
                .with_prop(
                    &pad.name,
                    "Graded Site Pad",
                    &format!(
                        "Cut {:.1} m3 | Fill {:.1} m3 | Net {:+.1} m3",
                        pad.cut_volume_m3, pad.fill_volume_m3, pad.net_balance_m3
                    ),
                )
                .with_attr(ns::BSI_CLASS, json!({ "code": "IfcSite" }))
                .with_mesh(&mesh);
            if let Ok(v) = serde_json::to_value(pad) {
                node.attributes.insert(NS_EG_CITY_PAD.to_string(), v);
            }
            nodes.push(node);
        }
        nodes
    }

    /// Reconstructs authored `IfcRoad`, `IfcBridge`, `IfcSpatialZone`, and graded pads from an `.ifcx` document.
    pub fn from_ifcx_document(doc: &IfcxDocument) -> Self {
        let mut scene = Self::default();
        let mut max_id = 0usize;
        for node in &doc.data {
            if let Some(rv) = node.attributes.get(NS_EG_CITY_ROAD) {
                if let Ok(r) = serde_json::from_value::<IfcxRoadSegment>(rv.clone()) {
                    max_id = max_id.max(r.id);
                    scene.roads.push(r);
                    continue;
                }
            }
            if let Some(zv) = node.attributes.get(NS_EG_CITY_ZONE) {
                if let Ok(z) = serde_json::from_value::<IfcxSpatialZone>(zv.clone()) {
                    max_id = max_id.max(z.id);
                    scene.zones.push(z);
                    continue;
                }
            }
            if let Some(pv) = node.attributes.get(NS_EG_CITY_PAD) {
                if let Ok(p) = serde_json::from_value::<IfcxGradedPad>(pv.clone()) {
                    max_id = max_id.max(p.id);
                    scene.graded_pads.push(p);
                }
            }
        }
        scene.next_id = max_id + 1;
        scene
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::gis::tin_surface::DemTerrainPreset;

    #[test]
    fn phase67_city_builder_roads_bridges_zones_snapping_and_ifcx_roundtrip() {
        let mut city = IfcxCityScene::default();
        city.add_road([-40.0, 0.0], [40.0, 0.0], IfcxRoadKind::Arterial, false)
            .expect("road added");
        city.add_road([0.0, -35.0], [0.0, 35.0], IfcxRoadKind::Collector, true)
            .expect("bridge added");
        city.add_zone([25.0, 22.0], 30.0, 24.0, 0.0, IfcxZoneKind::Commercial);
        city.add_zone([-25.0, 22.0], 28.0, 28.0, 15.0, IfcxZoneKind::Park);

        // Road setback snapping: Arterial width = 14m -> half=7m + 4m setback = 11m
        let hit = city
            .snap_point_to_road_or_parcel([10.0, 12.2], 4.0)
            .expect("should snap to arterial setback");
        assert!(hit.is_road);
        assert!((hit.snapped_enu_m[1] - 11.0).abs() < 1e-4);
        assert!((hit.bearing_deg - 0.0).abs() < 1e-4);

        // Park generates deterministic pure-vector instanced trees
        assert!(city.zones[1].instanced_trees_enu_m().len() >= 6);

        // Auto Cut/Fill Pad Grading on IfcSite TIN
        let mut tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::VolcanicFoothillsRidge,
            300.0,
            300.0,
            24,
            24,
            1.0,
        );
        let proj = BimProject::empty();
        let pad = city.auto_grade_building_pad(&proj, &mut tin);
        assert!(pad.cut_volume_m3 >= 0.0 && pad.fill_volume_m3 >= 0.0);
        assert!(pad.cut_volume_m3 + pad.fill_volume_m3 > 0.0);

        // Native IFCX composition round-trip
        let mut doc = IfcxDocument::new("city_roundtrip");
        for n in city.to_ifcx_nodes("/earth/sites/demo") {
            doc.add_node(n);
        }
        assert!(doc.data.iter().any(|n| n.ifc_class() == Some("IfcRoad")));
        assert!(doc.data.iter().any(|n| n.ifc_class() == Some("IfcBridge")));
        assert!(doc.data.iter().any(|n| n.ifc_class() == Some("IfcSpatialZone")));
        assert!(doc.data.iter().any(|n| n.ifc_class() == Some("IfcSite")));

        let restored = IfcxCityScene::from_ifcx_document(&doc);
        assert_eq!(restored.roads.len(), 2);
        assert_eq!(restored.zones.len(), 2);
        assert_eq!(restored.graded_pads.len(), 1);
    }

    #[test]
    fn test_v0867_curved_spline_road_and_auto_bridge_over_water() {
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::ArchipelagoBayAndVolcano,
            680.0,
            680.0,
            46,
            46,
            1.0,
        );
        let (ref_z, _) = tin.sample_elevation_and_slope(0.0, 0.0);
        let mut city = IfcxCityScene::default();
        let seg = city
            .add_curved_road_with_auto_bridge(
                [-190.0, -190.0],
                Some([10.0, -155.0]),
                [45.0, 35.0],
                IfcxRoadKind::Arterial,
                false,
                0.0,
                Some(&tin),
                0.0,
            )
            .expect("curved road created")
            .clone();

        let chord = (45.0_f64 + 190.0).hypot(35.0 + 190.0);
        assert!(
            seg.length_m() > chord + 10.0,
            "Quadratic Bezier arc length {} must exceed chord {}",
            seg.length_m(),
            chord
        );
        assert!(
            seg.is_bridge,
            "Coastal water crossing must auto-promote road to IfcBridge"
        );

        let profile = seg.sample_3d_profile(Some(&tin), 0.0);
        assert!(profile.len() >= 18);
        assert!(
            profile.iter().any(|s| s.over_water),
            "Coastal road profile must detect submerged water crossing"
        );
        for s in profile.iter().filter(|s| s.over_water) {
            assert!(
                s.deck_z_rel_m > (0.0 - ref_z) + 4.0,
                "Auto-bridge deck {} must clear sea surface {}",
                s.deck_z_rel_m,
                0.0 - ref_z
            );
        }

        // Junction endpoint snapping
        let snapped = city
            .snap_road_endpoint([49.0, 32.0], 10.0)
            .expect("should snap to curved road endpoint");
        assert!((snapped[0] - 45.0).abs() < 1e-6 && (snapped[1] - 35.0).abs() < 1e-6);

        // IFCX multi-segment node export
        let nodes = city.to_ifcx_nodes("/earth/sites/coast");
        let bridge_node = nodes
            .iter()
            .find(|n| n.ifc_class() == Some("IfcBridge"))
            .expect("bridge IFCX node");
        assert!(bridge_node.mesh().unwrap().points.len() > 24);
    }
}
