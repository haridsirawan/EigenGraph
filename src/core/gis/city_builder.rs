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
pub const NS_EG_CITY_BUILDING: &str = "eigengraph::city::proceduralBuilding";
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

/// Urban parcel development density tier (`Low`, `Medium`, `High`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ZoneDensityTier {
    LowDensity,
    #[default]
    MediumDensity,
    HighDensity,
}

impl ZoneDensityTier {
    pub fn label(self) -> &'static str {
        match self {
            Self::LowDensity => "Low Density",
            Self::MediumDensity => "Medium Density",
            Self::HighDensity => "High-Rise Density",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::LowDensity => "Low",
            Self::MediumDensity => "Med",
            Self::HighDensity => "High",
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::LowDensity => Self::MediumDensity,
            Self::MediumDensity => Self::HighDensity,
            Self::HighDensity => Self::LowDensity,
        }
    }
}

/// Deterministic architectural building massing spawned onto a zoned parcel (`IfcBuilding`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProceduralCityBuilding {
    pub name: String,
    pub zone_id: usize,
    pub zone_kind: IfcxZoneKind,
    pub density: ZoneDensityTier,
    pub center_enu_m: [f64; 2],
    pub rotation_deg: f64,
    pub podium_width_m: f64,
    pub podium_depth_m: f64,
    pub podium_height_m: f32,
    pub tower_width_m: f64,
    pub tower_depth_m: f64,
    pub tower_height_m: f32,
    pub has_penthouse: bool,
    pub floors: usize,
    pub gfa_m2: f64,
    pub residents: usize,
    pub jobs: usize,
    pub wall_tint_rgba: [f32; 4],
}

/// Aggregated Cities: Skylines II style district population, employment & built-area telemetry.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CityDistrictMetrics {
    pub parcel_count: usize,
    pub building_count: usize,
    pub total_residents: usize,
    pub total_jobs: usize,
    pub total_gfa_m2: f64,
    pub park_parcel_count: usize,
    pub green_ratio_pct: f32,
}

fn default_ambience_true() -> bool {
    true
}

/// Controls for Cities: Skylines II living city ambience (procedural forests, sun shadows & traffic).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CityAmbienceConfig {
    #[serde(default = "default_ambience_true")]
    pub show_procedural_forests: bool,
    #[serde(default)]
    pub forest_density: u8,
    #[serde(default = "default_ambience_true")]
    pub show_dynamic_shadows: bool,
    #[serde(default = "default_ambience_true")]
    pub show_animated_traffic: bool,
    #[serde(default)]
    pub traffic_density: u8,
    #[serde(default)]
    pub traffic_speed_mps: f32,
}

impl Default for CityAmbienceConfig {
    fn default() -> Self {
        Self {
            show_procedural_forests: true,
            forest_density: 3,
            show_dynamic_shadows: true,
            show_animated_traffic: true,
            traffic_density: 3,
            traffic_speed_mps: 11.0,
        }
    }
}

/// Deterministic hillside / coastal forest tree instance placed outside zones & road clearances.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForestTreeInstance {
    pub east_m: f32,
    pub north_m: f32,
    pub base_z_rel_m: f32,
    pub height_m: f32,
    pub canopy_tint: f32,
}

/// Projected 3D ground shadow footprint cast opposite the solar vector from a procedural tower.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildingShadowQuad {
    pub corners_enu_m: [[f32; 3]; 4],
}

/// Animated 3D commuter vehicle sample traveling along an authored `IfcRoad` or `IfcBridge`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficVehicleSample {
    pub pos_enu_m: [f32; 3],
    pub tangent: [f64; 2],
    pub normal: [f64; 2],
    pub length_m: f64,
    pub width_m: f64,
    pub height_m: f32,
    pub body_rgba: [f32; 4],
    pub is_bus_or_truck: bool,
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
    #[serde(default)]
    pub density: ZoneDensityTier,
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

    /// Synthesizes deterministic podium + stepped tower architectural massing for this parcel.
    pub fn synthesize_procedural_building(&self) -> Option<ProceduralCityBuilding> {
        if !self.synthesize_context_towers || self.kind == IfcxZoneKind::Park {
            return None;
        }
        let hash = (self.id as u64).wrapping_mul(2654435761) ^ 0x9E37_79B9_7F4A_7C15;
        let u0 = ((hash & 0xFFFF) as f64) / 65535.0;
        let u1 = (((hash >> 16) & 0xFFFF) as f64) / 65535.0;
        let u2 = (((hash >> 32) & 0xFFFF) as f64) / 65535.0;

        let (floors, story_h_m, has_penthouse, tower_scale) = match (self.kind, self.density) {
            (_, ZoneDensityTier::LowDensity) => {
                let fl = match self.kind {
                    IfcxZoneKind::Industrial => 1 + (hash as usize % 2),
                    _ => 2 + (hash as usize % 2),
                };
                (fl, 3.35_f32, false, 0.88)
            }
            (IfcxZoneKind::Industrial, _) => (2 + (hash as usize % 3), 4.2, false, 0.90),
            (IfcxZoneKind::Commercial, ZoneDensityTier::HighDensity) => {
                (14 + (hash as usize % 11), 3.6, true, 0.64)
            }
            (IfcxZoneKind::Residential, ZoneDensityTier::HighDensity) => {
                (11 + (hash as usize % 9), 3.3, true, 0.66)
            }
            (IfcxZoneKind::Civic, ZoneDensityTier::HighDensity) => {
                (10 + (hash as usize % 6), 3.8, true, 0.68)
            }
            (IfcxZoneKind::Commercial, ZoneDensityTier::MediumDensity) => {
                (6 + (hash as usize % 5), 3.55, true, 0.74)
            }
            (IfcxZoneKind::Civic, ZoneDensityTier::MediumDensity) => {
                (4 + (hash as usize % 4), 3.8, true, 0.76)
            }
            (IfcxZoneKind::Residential, ZoneDensityTier::MediumDensity) => {
                (4 + (hash as usize % 5), 3.25, u2 > 0.45, 0.78)
            }
            (IfcxZoneKind::Park, _) => return None,
        };

        let podium_floors = match self.density {
            ZoneDensityTier::LowDensity => floors,
            ZoneDensityTier::MediumDensity => 2.min(floors),
            ZoneDensityTier::HighDensity => 3.min(floors),
        };
        let podium_width_m = (self.width_m * (0.58 + 0.14 * u0)).clamp(8.0, self.width_m - 2.5);
        let podium_depth_m = (self.depth_m * (0.56 + 0.14 * u1)).clamp(8.0, self.depth_m - 2.5);
        let podium_height_m = podium_floors as f32 * story_h_m;
        let tower_width_m = (podium_width_m * tower_scale).max(6.5);
        let tower_depth_m = (podium_depth_m * tower_scale).max(6.5);
        let tower_height_m = floors as f32 * story_h_m;

        let upper_floors = floors.saturating_sub(podium_floors);
        let gfa_m2 = podium_floors as f64 * (podium_width_m * podium_depth_m)
            + upper_floors as f64 * (tower_width_m * tower_depth_m);
        let (residents, jobs) = match self.kind {
            IfcxZoneKind::Residential => (((gfa_m2 / 36.0).round() as usize).max(4), 0),
            IfcxZoneKind::Commercial => (0, ((gfa_m2 / 26.0).round() as usize).max(6)),
            IfcxZoneKind::Industrial => (0, ((gfa_m2 / 44.0).round() as usize).max(4)),
            IfcxZoneKind::Civic => (0, ((gfa_m2 / 34.0).round() as usize).max(5)),
            IfcxZoneKind::Park => (0, 0),
        };
        let wall_tint_rgba = match self.kind {
            IfcxZoneKind::Residential => [
                0.76 + 0.12 * u0 as f32,
                0.74 + 0.10 * u1 as f32,
                0.68,
                1.0,
            ],
            IfcxZoneKind::Commercial => [0.36, 0.56 + 0.14 * u1 as f32, 0.78 + 0.10 * u2 as f32, 1.0],
            IfcxZoneKind::Industrial => [0.68, 0.58, 0.42, 1.0],
            IfcxZoneKind::Civic => [0.70, 0.66, 0.84, 1.0],
            IfcxZoneKind::Park => [0.24, 0.68, 0.42, 1.0],
        };

        Some(ProceduralCityBuilding {
            name: format!("Bldg_{}_{}", self.kind.short_label(), self.id),
            zone_id: self.id,
            zone_kind: self.kind,
            density: self.density,
            center_enu_m: self.center_enu_m,
            rotation_deg: self.rotation_deg,
            podium_width_m,
            podium_depth_m,
            podium_height_m,
            tower_width_m,
            tower_depth_m,
            tower_height_m,
            has_penthouse,
            floors,
            gfa_m2,
            residents,
            jobs,
            wall_tint_rgba,
        })
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
    #[serde(default)]
    pub active_zone_density: ZoneDensityTier,
    #[serde(default)]
    pub ambience: CityAmbienceConfig,
    /// Update `v0.8.7.8`: ID of an `IfcxSpatialZone` promoted to the editable active OpenBIM project
    /// so its procedural shell is suppressed (preventing Z-fighting with detailed structural BIM).
    #[serde(default)]
    pub promoted_zone_id: Option<usize>,
    /// Update `v0.8.7.8`: Prefer signal-free landscaped Roundabouts at >=3-branch road intersections.
    #[serde(default)]
    pub prefer_roundabouts: bool,
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
            active_zone_density: ZoneDensityTier::MediumDensity,
            ambience: CityAmbienceConfig::default(),
            promoted_zone_id: None,
            prefer_roundabouts: false,
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
            density: self.active_zone_density,
        });
        self.zones.last().expect("pushed zone")
    }

    /// Aggregates live district population, job capacity, built GFA, and green space telemetry.
    pub fn district_metrics(&self) -> CityDistrictMetrics {
        let mut m = CityDistrictMetrics {
            parcel_count: self.zones.len(),
            ..Default::default()
        };
        for z in &self.zones {
            if z.kind == IfcxZoneKind::Park {
                m.park_parcel_count += 1;
            }
            if let Some(b) = z.synthesize_procedural_building() {
                m.building_count += 1;
                m.total_residents += b.residents;
                m.total_jobs += b.jobs;
                m.total_gfa_m2 += b.gfa_m2;
            }
        }
        if m.parcel_count > 0 {
            m.green_ratio_pct = (m.park_parcel_count as f32 / m.parcel_count as f32) * 100.0;
        }
        m
    }

    /// Generates road-snapped `IfcSpatialZone` parcel lots along both sides of ground roads,
    /// skipping submerged coastal water, steep hazard cliffs (>26°), road overlaps, and the origin BIM lot.
    pub fn auto_zone_parcels_along_roads(
        &mut self,
        tin: Option<&TinSurfaceMesh>,
        sea_level_m: f32,
        lot_width_m: f64,
        lot_depth_m: f64,
    ) -> usize {
        let lot_w = lot_width_m.clamp(16.0, 48.0);
        let lot_d = lot_depth_m.clamp(14.0, 40.0);
        let step_spacing = lot_w + 4.0;
        let min_zone_sep = (lot_w.min(lot_d) * 0.85).max(16.0);
        let roads_snapshot = self.roads.clone();
        let mut spawned = 0usize;

        for road in &roads_snapshot {
            if road.is_bridge {
                continue;
            }
            let total_len = road.length_m();
            if total_len < lot_w + 6.0 {
                continue;
            }
            let pts = road.sample_centerline(((total_len / 6.0).ceil() as usize).clamp(8, 36));
            let mut s_arc = 0.0_f64;
            let mut dist_since_prev = step_spacing;

            for seg in pts.windows(2) {
                let ax = seg[0][0];
                let ay = seg[0][1];
                let dx = seg[1][0] - ax;
                let dy = seg[1][1] - ay;
                let slen = dx.hypot(dy);
                if slen < 0.5 {
                    continue;
                }
                s_arc += slen;
                dist_since_prev += slen;
                if s_arc < lot_w * 0.45 || s_arc > total_len - lot_w * 0.35 {
                    continue;
                }
                if dist_since_prev < step_spacing {
                    continue;
                }
                dist_since_prev = 0.0;

                let mid = [ax + dx * 0.5, ay + dy * 0.5];
                let nx = -dy / slen;
                let ny = dx / slen;
                let bearing = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                let setback_center = road.width_m * 0.5 + 3.0 + lot_d * 0.5;

                for side in [-1.0_f64, 1.0] {
                    let cx = mid[0] + nx * side * setback_center;
                    let cy = mid[1] + ny * side * setback_center;
                    if cx.hypot(cy) < 20.0 {
                        continue;
                    }
                    if let Some(t) = tin {
                        let (gz, slope_deg) = t.sample_elevation_and_slope(cx as f32, cy as f32);
                        if gz < sea_level_m + 0.65 || slope_deg > 26.0 {
                            continue;
                        }
                    }
                    let overlaps_zone = self.zones.iter().any(|z| {
                        (z.center_enu_m[0] - cx).hypot(z.center_enu_m[1] - cy) < min_zone_sep
                    });
                    if overlaps_zone {
                        continue;
                    }
                    let overlaps_road = roads_snapshot.iter().any(|r| {
                        r.sample_centerline(12).windows(2).any(|w| {
                            let rx = w[0][0];
                            let ry = w[0][1];
                            let rdx = w[1][0] - rx;
                            let rdy = w[1][1] - ry;
                            let l2 = rdx * rdx + rdy * rdy;
                            if l2 < 1.0 {
                                return false;
                            }
                            let u = (((cx - rx) * rdx + (cy - ry) * rdy) / l2).clamp(0.0, 1.0);
                            let px = rx + rdx * u;
                            let py = ry + rdy * u;
                            (cx - px).hypot(cy - py) < (r.width_m * 0.5 + lot_d * 0.32)
                        })
                    });
                    if overlaps_road {
                        continue;
                    }

                    // Mix occasional neighborhood pocket parks every 6th lot along arterials/collectors
                    let kind = if (self.zones.len() + 1).is_multiple_of(6) {
                        IfcxZoneKind::Park
                    } else {
                        self.active_zone_kind
                    };
                    self.add_zone([cx, cy], lot_w, lot_d, bearing, kind);
                    spawned += 1;
                }
            }
        }
        spawned
    }

    /// Synthesizes biome-filtered hillside & coastal woodland trees across gentle, elevated ground
    /// (`sea_level + 4.2m .. sea_level + 52m`, `slope <= 28°`), avoiding roads, zones, and BIM origin.
    pub fn synthesize_biome_forest_trees(
        &self,
        tin: &TinSurfaceMesh,
        sea_level_m: f32,
    ) -> Vec<ForestTreeInstance> {
        if !self.ambience.show_procedural_forests || tin.points.is_empty() {
            return Vec::new();
        }
        let (ref_z, _) = tin.sample_elevation_and_slope(0.0, 0.0);
        let attempts = (self.ambience.forest_density.max(1) as usize * 90).clamp(40, 320);
        let span_x = (tin.extent_x_m * 0.86).max(40.0);
        let span_y = (tin.extent_y_m * 0.86).max(40.0);
        let mut out = Vec::with_capacity(220);

        for i in 0..attempts {
            if out.len() >= 220 {
                break;
            }
            let h = ((i as u64 + 1).wrapping_mul(2_654_435_761)) ^ 0xA5A5_5A5A_1234_5678;
            let u = ((h & 0xFFFF) as f32) / 65535.0 - 0.5;
            let v = (((h >> 16) & 0xFFFF) as f32) / 65535.0 - 0.5;
            let east_m = u * span_x;
            let north_m = v * span_y;
            if east_m.hypot(north_m) < 24.0 {
                continue;
            }
            let (elev_msl, slope_deg) = tin.sample_elevation_and_slope(east_m, north_m);
            if elev_msl < sea_level_m + 4.2 || elev_msl > sea_level_m + 52.0 || slope_deg > 28.0 {
                continue;
            }
            let inside_zone = self.zones.iter().any(|z| {
                let rad = z.rotation_deg.to_radians();
                let (sr, cr) = (rad.sin(), rad.cos());
                let dx = east_m as f64 - z.center_enu_m[0];
                let dy = north_m as f64 - z.center_enu_m[1];
                let lx = dx * cr + dy * sr;
                let ly = -dx * sr + dy * cr;
                lx.abs() < z.width_m * 0.58 && ly.abs() < z.depth_m * 0.58
            });
            if inside_zone {
                continue;
            }
            let on_road = self.roads.iter().any(|r| {
                let clr = r.width_m * 0.5 + 4.0;
                r.sample_centerline(10).windows(2).any(|w| {
                    let rx = w[0][0];
                    let ry = w[0][1];
                    let rdx = w[1][0] - rx;
                    let rdy = w[1][1] - ry;
                    let l2 = rdx * rdx + rdy * rdy;
                    if l2 < 1.0 {
                        return false;
                    }
                    let u_p = (((east_m as f64 - rx) * rdx + (north_m as f64 - ry) * rdy) / l2)
                        .clamp(0.0, 1.0);
                    let px = rx + rdx * u_p;
                    let py = ry + rdy * u_p;
                    (east_m as f64 - px).hypot(north_m as f64 - py) < clr
                })
            });
            if on_road {
                continue;
            }
            let h_m = 6.5 + ((((h >> 32) & 0xFF) as f32) / 255.0) * 7.0;
            let canopy_tint = (((h >> 40) & 0xFF) as f32) / 255.0;
            out.push(ForestTreeInstance {
                east_m,
                north_m,
                base_z_rel_m: elev_msl - ref_z,
                height_m: h_m,
                canopy_tint,
            });
        }
        out
    }

    /// Computes dynamic ground cast shadow quads opposite the active solar direction vector.
    pub fn compute_dynamic_building_shadows(
        &self,
        tin: Option<&TinSurfaceMesh>,
        sun_dir_site: [f32; 3],
    ) -> Vec<BuildingShadowQuad> {
        if !self.ambience.show_dynamic_shadows || sun_dir_site[2] <= 0.04 {
            return Vec::new();
        }
        let ref_z = tin
            .map(|t| t.sample_elevation_and_slope(0.0, 0.0).0)
            .unwrap_or(0.0);
        let inv_sz = (1.0 / sun_dir_site[2].max(0.22)).min(2.6);
        let horiz_len = sun_dir_site[0].hypot(sun_dir_site[1]).max(1e-4);
        let dir_x = -sun_dir_site[0] / horiz_len;
        let dir_y = -sun_dir_site[1] / horiz_len;
        let perp_x = -dir_y;
        let perp_y = dir_x;

        let mut shadows = Vec::with_capacity(self.zones.len());
        for zone in &self.zones {
            if self.promoted_zone_id == Some(zone.id) {
                continue;
            }
            let Some(bldg) = zone.synthesize_procedural_building() else {
                continue;
            };
            let cx = bldg.center_enu_m[0] as f32;
            let cy = bldg.center_enu_m[1] as f32;
            let proj_len = (bldg.tower_height_m * (1.0 - sun_dir_site[2].powi(2)).sqrt() * inv_sz)
                .clamp(3.0, 95.0);
            let half_w = (0.48 * bldg.tower_width_m.max(bldg.tower_depth_m)) as f32;
            let gz_base = tin
                .map(|t| t.sample_elevation_and_slope(cx, cy).0 - ref_z)
                .unwrap_or(0.0)
                + 0.24;
            let tx = cx + dir_x * proj_len;
            let ty = cy + dir_y * proj_len;
            let gz_tip = tin
                .map(|t| t.sample_elevation_and_slope(tx, ty).0 - ref_z)
                .unwrap_or(0.0)
                + 0.24;

            shadows.push(BuildingShadowQuad {
                corners_enu_m: [
                    [cx - perp_x * half_w, cy - perp_y * half_w, gz_base],
                    [cx + perp_x * half_w, cy + perp_y * half_w, gz_base],
                    [tx + perp_x * half_w * 0.85, ty + perp_y * half_w * 0.85, gz_tip],
                    [tx - perp_x * half_w * 0.85, ty - perp_y * half_w * 0.85, gz_tip],
                ],
            });
        }
        shadows
    }

    /// Evaluates continuous bidirectional commuter, transit & freight vehicles along all authored roads.
    pub fn sample_animated_traffic(
        &self,
        tin: Option<&TinSurfaceMesh>,
        sea_level_m: f32,
        time_secs: f32,
    ) -> Vec<TrafficVehicleSample> {
        if !self.ambience.show_animated_traffic || self.roads.is_empty() {
            return Vec::new();
        }
        let palette = [
            [0.92_f32, 0.94, 0.96, 1.0],
            [0.22, 0.56, 0.90, 1.0],
            [0.92, 0.34, 0.28, 1.0],
            [0.96, 0.74, 0.24, 1.0],
            [0.24, 0.76, 0.56, 1.0],
        ];
        let speed = self.ambience.traffic_speed_mps.clamp(2.0, 28.0) as f64;
        let density_scale = (self.ambience.traffic_density.max(1) as f64) * 0.65;
        let mut vehicles = Vec::with_capacity(64);

        for road in &self.roads {
            if vehicles.len() >= 160 {
                break;
            }
            let profile = road.sample_3d_profile(tin, sea_level_m);
            if profile.len() < 2 {
                continue;
            }
            let mut cum_s = Vec::with_capacity(profile.len());
            cum_s.push(0.0_f64);
            for w in profile.windows(2) {
                let seg_len = (w[1].east_m - w[0].east_m).hypot(w[1].north_m - w[0].north_m);
                cum_s.push(cum_s.last().copied().unwrap_or(0.0) + seg_len);
            }
            let total_len = cum_s.last().copied().unwrap_or(0.0);
            if total_len < 8.0 {
                continue;
            }
            let n_per_dir = ((total_len / 55.0) * density_scale).ceil() as usize;
            let n_per_dir = n_per_dir.clamp(1, 10);
            let lane_off_mag = (road.width_m * 0.23).max(1.25);

            for dir_idx in 0..2 {
                let dir_sign = if dir_idx == 0 { 1.0_f64 } else { -1.0_f64 };
                for v_i in 0..n_per_dir {
                    if vehicles.len() >= 160 {
                        break;
                    }
                    let seed = (road.id as u64).wrapping_mul(1_000_003)
                        ^ ((dir_idx as u64 + 1) * 97)
                        ^ ((v_i as u64 + 1) * 131);
                    let base_s = (v_i as f64 / n_per_dir as f64) * total_len;
                    let phase_s =
                        (base_s + (time_secs as f64) * speed * dir_sign).rem_euclid(total_len);

                    let seg_i = cum_s
                        .windows(2)
                        .position(|w| phase_s >= w[0] && phase_s <= w[1])
                        .unwrap_or(profile.len() - 2);
                    let s0 = cum_s[seg_i];
                    let s1 = cum_s[seg_i + 1];
                    let f = if (s1 - s0) > 1e-5 {
                        ((phase_s - s0) / (s1 - s0)).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let a = &profile[seg_i];
                    let b = &profile[seg_i + 1];
                    let tx = a.tangent[0] * (1.0 - f) + b.tangent[0] * f;
                    let ty = a.tangent[1] * (1.0 - f) + b.tangent[1] * f;
                    let tlen = tx.hypot(ty).max(1e-6);
                    let fwd = [(tx / tlen) * dir_sign, (ty / tlen) * dir_sign];
                    let left_n = [-fwd[1], fwd[0]];

                    let cx = (a.east_m * (1.0 - f) + b.east_m * f) + left_n[0] * lane_off_mag;
                    let cy = (a.north_m * (1.0 - f) + b.north_m * f) + left_n[1] * lane_off_mag;
                    let cz = a.deck_z_rel_m * (1.0 - f as f32) + b.deck_z_rel_m * (f as f32) + 0.10;
                    let is_bus_or_truck = seed.is_multiple_of(5);
                    let (length_m, width_m, height_m) = if is_bus_or_truck {
                        (7.8, 2.2, 2.55)
                    } else {
                        (4.4, 1.85, 1.48)
                    };
                    let body_rgba = palette[(seed as usize) % palette.len()];

                    vehicles.push(TrafficVehicleSample {
                        pos_enu_m: [cx as f32, cy as f32, cz],
                        tangent: fwd,
                        normal: left_n,
                        length_m,
                        width_m,
                        height_m,
                        body_rgba,
                        is_bus_or_truck,
                    });
                }
            }
        }
        vehicles
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

    /// Recenters all city roads, zones, and graded pads in ENU meters when promoting a district
    /// building so the promoted lot becomes `(0, 0)` while surrounding infrastructure stays aligned.
    pub fn recenter_origin_enu(&mut self, delta_enu_m: [f64; 2]) {
        for r in &mut self.roads {
            r.start_enu_m[0] -= delta_enu_m[0];
            r.start_enu_m[1] -= delta_enu_m[1];
            r.end_enu_m[0] -= delta_enu_m[0];
            r.end_enu_m[1] -= delta_enu_m[1];
            if let Some(ref mut c) = r.control_enu_m {
                c[0] -= delta_enu_m[0];
                c[1] -= delta_enu_m[1];
            }
        }
        for z in &mut self.zones {
            z.center_enu_m[0] -= delta_enu_m[0];
            z.center_enu_m[1] -= delta_enu_m[1];
        }
        for p in &mut self.graded_pads {
            p.center_enu_m[0] -= delta_enu_m[0];
            p.center_enu_m[1] -= delta_enu_m[1];
        }
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
        mix(&mut h, self.promoted_zone_id.unwrap_or(0) as u64);
        mix(&mut h, u64::from(self.prefer_roundabouts));
        mix(&mut h, u64::from(self.ambience.show_procedural_forests));
        mix(&mut h, self.ambience.forest_density as u64);
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
            mix(&mut h, z.density as u64);
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

            if let Some(bldg) = zone.synthesize_procedural_building() {
                let hw = bldg.tower_width_m * 0.5;
                let hd = bldg.tower_depth_m * 0.5;
                let rad = bldg.rotation_deg.to_radians();
                let (sr, cr) = (rad.sin(), rad.cos());
                let corners = [[-hw, -hd], [hw, -hd], [hw, hd], [-hw, hd]];
                let mut b_pts = Vec::with_capacity(8);
                for z_h in [0.18_f64, bldg.tower_height_m as f64] {
                    for &[lx, ly] in &corners {
                        b_pts.push([
                            bldg.center_enu_m[0] + lx * cr - ly * sr,
                            bldg.center_enu_m[1] + lx * sr + ly * cr,
                            z_h,
                        ]);
                    }
                }
                let b_mesh = UsdGeomMesh {
                    points: b_pts,
                    face_vertex_counts: vec![3; 12],
                    face_vertex_indices: vec![
                        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6,
                        1, 2, 6, 1, 6, 5, 3, 0, 4, 3, 4, 7,
                    ],
                    normals: Vec::new(),
                    display_color: Some(bldg.wall_tint_rgba),
                    vertex_colors: Vec::new(),
                    vertex_meta: Vec::new(),
                    material_kind: 2.0,
                };
                let b_path = format!("{site_path}/buildings/{}", bldg.name.to_lowercase());
                let mut b_node = IfcxNode::new(b_path)
                    .with_class("IfcBuilding")
                    .with_prop(
                        &bldg.name,
                        bldg.density.label(),
                        &format!(
                            "{} Floors | GFA {:.0} m² | Pop {} | Jobs {}",
                            bldg.floors, bldg.gfa_m2, bldg.residents, bldg.jobs
                        ),
                    )
                    .with_mesh(&b_mesh);
                if let Ok(v) = serde_json::to_value(&bldg) {
                    b_node.attributes.insert(NS_EG_CITY_BUILDING.to_string(), v);
                }
                nodes.push(b_node);
            }
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

    #[test]
    fn test_v0868_road_snapped_parcel_zoning_and_procedural_ifcx_buildings() {
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::ArchipelagoBayAndVolcano,
            680.0,
            680.0,
            46,
            46,
            1.0,
        );
        let mut city = IfcxCityScene::default();
        city.active_zone_kind = IfcxZoneKind::Commercial;
        city.active_zone_density = ZoneDensityTier::HighDensity;
        city.add_curved_road_with_auto_bridge(
            [65.0, -5.0],
            Some([20.0, 55.0]),
            [-45.0, 95.0],
            IfcxRoadKind::Arterial,
            false,
            0.0,
            Some(&tin),
            0.0,
        )
        .expect("on-shore collector");

        let spawned = city.auto_zone_parcels_along_roads(Some(&tin), 0.0, 28.0, 24.0);
        assert!(spawned >= 3, "Expected >=3 road-snapped lots along collector, got {spawned}");

        let metrics = city.district_metrics();
        assert!(metrics.building_count >= 2);
        assert!(metrics.total_gfa_m2 > 5_000.0);
        assert!(metrics.total_residents + metrics.total_jobs > 100);

        let bldgs: Vec<ProceduralCityBuilding> = city
            .zones
            .iter()
            .filter_map(|z| z.synthesize_procedural_building())
            .collect();
        assert!(
            bldgs.iter().any(|b| b.floors >= 10 && b.tower_height_m > 32.0 && b.has_penthouse),
            "HighDensity zones must spawn stepped high-rise buildings"
        );

        let nodes = city.to_ifcx_nodes("/earth/sites/district");
        assert!(
            nodes.iter().any(|n| n.ifc_class() == Some("IfcBuilding")),
            "Procedural city buildings must export native IfcBuilding IFCX nodes"
        );
    }

    #[test]
    fn test_v0869_living_city_forests_shadows_and_animated_traffic() {
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::ArchipelagoBayAndVolcano,
            680.0,
            680.0,
            46,
            46,
            1.0,
        );
        let mut city = IfcxCityScene::default();
        city.active_zone_density = ZoneDensityTier::HighDensity;
        city.add_curved_road_with_auto_bridge(
            [65.0, -5.0],
            Some([20.0, 55.0]),
            [-45.0, 95.0],
            IfcxRoadKind::Arterial,
            false,
            0.0,
            Some(&tin),
            0.0,
        )
        .expect("arterial road");
        let _ = city.auto_zone_parcels_along_roads(Some(&tin), 0.0, 28.0, 24.0);

        let trees = city.synthesize_biome_forest_trees(&tin, 0.0);
        assert!(
            trees.len() >= 10,
            "Volcano hillside biome must spawn procedural trees, got {}",
            trees.len()
        );
        for tr in &trees {
            assert!(tr.east_m.hypot(tr.north_m) >= 24.0);
            assert!(tr.height_m >= 6.0);
        }

        let shadows = city.compute_dynamic_building_shadows(Some(&tin), [0.5, 0.35, 0.65]);
        assert_eq!(shadows.len(), city.district_metrics().building_count);
        let night_shadows = city.compute_dynamic_building_shadows(Some(&tin), [0.5, 0.35, -0.1]);
        assert!(night_shadows.is_empty());

        let v_t0 = city.sample_animated_traffic(Some(&tin), 0.0, 0.0);
        let v_t2 = city.sample_animated_traffic(Some(&tin), 0.0, 2.0);
        assert!(v_t0.len() >= 4, "Should populate bidirectional traffic");
        assert_eq!(v_t0.len(), v_t2.len());
        let moved = v_t0
            .iter()
            .zip(v_t2.iter())
            .any(|(a, b)| (a.pos_enu_m[0] - b.pos_enu_m[0]).hypot(a.pos_enu_m[1] - b.pos_enu_m[1]) > 1.0);
        assert!(moved, "Animated traffic vehicles must move forward over time");
    }

    #[test]
    fn test_v087_full_living_city_ifcx_bundle_roundtrip() {
        let mut tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::ArchipelagoBayAndVolcano,
            680.0,
            680.0,
            46,
            46,
            1.0,
        );
        let _ = tin.apply_sculpt_brush(0.0, 0.0, crate::core::gis::TerrainSculptBrush::Raise, 45.0, 3.0, 0.0, 0.0);

        let mut city = IfcxCityScene::default();
        city.active_zone_density = ZoneDensityTier::HighDensity;
        city.add_curved_road_with_auto_bridge(
            [-190.0, -180.0],
            Some([15.0, -145.0]),
            [45.0, 25.0],
            IfcxRoadKind::Arterial,
            false,
            0.0,
            Some(&tin),
            0.0,
        )
        .expect("coastal bay bridge");
        city.add_curved_road_with_auto_bridge(
            [65.0, -5.0],
            Some([20.0, 55.0]),
            [-45.0, 95.0],
            IfcxRoadKind::Collector,
            false,
            0.0,
            Some(&tin),
            0.0,
        )
        .expect("harbor avenue");
        let spawned = city.auto_zone_parcels_along_roads(Some(&tin), 0.0, 28.0, 24.0);
        assert!(spawned >= 2);

        let proj = BimProject::empty();
        let base = IfcxDocument::from_bim_project(
            &proj,
            Some((-6.2088, 106.8456, 0.0)),
            0.0,
            None,
            None,
            None,
        );
        let overlay = tin.export_ifcx_site_terraform_overlay(0.0, 0.0, 120.0, 1);
        let mut bundle = crate::core::ifcx::IfcxStage::compose_with_site_terraform_overlay(&base, &overlay);
        for n in city.to_ifcx_nodes("/earth/sites/living_city") {
            bundle.add_node(n);
        }

        assert!(bundle.data.iter().any(|n| n.ifc_class() == Some("IfcSite")));
        assert!(bundle.data.iter().any(|n| n.ifc_class() == Some("IfcRoad")));
        assert!(bundle.data.iter().any(|n| n.ifc_class() == Some("IfcBridge")));
        assert!(bundle.data.iter().any(|n| n.ifc_class() == Some("IfcSpatialZone")));
        assert!(bundle.data.iter().any(|n| n.ifc_class() == Some("IfcBuilding")));

        let restored = IfcxCityScene::from_ifcx_document(&bundle);
        assert_eq!(restored.roads.len(), city.roads.len());
        assert_eq!(restored.zones.len(), city.zones.len());
    }
}
