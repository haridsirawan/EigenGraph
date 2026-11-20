//! earth_globe.rs — 3D Interactive Spherical Earth Engine & Planetary Terraforming Game (Phase 43 / v0.4.3).
//!
//! Provides a true Google Earth-style 3D spherical globe renderer and camera (`GlobeCamera`),
//! double-precision (`f64`) ray-sphere cursor picking, Great-Circle geodesic drawing,
//! interactive planetary terraforming brushes (uplift, carve/canal, reclaim, forest, solar, lake, seawall),
//! trans-continental megastructures (bridges, tunnels, mega-towers), sea-level flood simulation,
//! climate/carbon/habitability vitals, and playable planetary engineering scenarios.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, Vec2};
use serde::{Deserialize, Serialize};

use super::map_widget::{
    ear_clip_polygon, SlippyMapState, VectorEarthPalette, VectorEarthTheme, EARTH_RADIUS_METERS,
    GLOBAL_CITY_CATALOG, WORLD_LANDMASSES, WORLD_MAJOR_RIVERS, WORLD_MOUNTAIN_RIDGES,
};
use super::tin_surface::TinSurfaceMesh;
use crate::theme::MacTheme;

/// Angular resolution (degrees) of each sparse planetary modification cell (~111 km per deg at Equator).
pub const EARTH_CELL_DEG: f64 = 1.5;

/// Land cover / planetary engineering biome painted onto an `EarthCellMod`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EarthBiome {
    #[default]
    Natural,
    ForestBelt,
    Cropland,
    SolarFarm,
    ReservoirLake,
    ReclaimedLand,
    MegacityArcology,
    GlacierIce,
}

impl EarthBiome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Natural => "Natural",
            Self::ForestBelt => "Mega-Forest",
            Self::Cropland => "Agri-Belt",
            Self::SolarFarm => "Solar Array",
            Self::ReservoirLake => "Reservoir Lake",
            Self::ReclaimedLand => "Reclaimed Land",
            Self::MegacityArcology => "Arcology City",
            Self::GlacierIce => "Ice Sheet",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Natural => Color32::from_rgb(104, 142, 92),
            Self::ForestBelt => Color32::from_rgb(22, 163, 74),
            Self::Cropland => Color32::from_rgb(132, 184, 58),
            Self::SolarFarm => Color32::from_rgb(245, 158, 11),
            Self::ReservoirLake => Color32::from_rgb(14, 165, 233),
            Self::ReclaimedLand => Color32::from_rgb(52, 211, 153),
            Self::MegacityArcology => Color32::from_rgb(56, 189, 248),
            Self::GlacierIce => Color32::from_rgb(226, 240, 252),
        }
    }

    /// Annual CO2 sink/source contribution in MtCO2/yr per 1.5-deg cell.
    pub fn annual_carbon_mt(&self) -> f64 {
        match self {
            Self::ForestBelt => -42.0,
            Self::Cropland => -8.0,
            Self::SolarFarm => -55.0,
            Self::ReclaimedLand => 4.0,
            Self::MegacityArcology => 26.0,
            Self::GlacierIce => -2.0,
            Self::Natural | Self::ReservoirLake => 0.0,
        }
    }

    /// Local planetary cooling/warming offset (deg C contribution per cell).
    pub fn thermal_factor(&self) -> f64 {
        match self {
            Self::ForestBelt => -0.006,
            Self::SolarFarm => -0.003,
            Self::ReservoirLake => -0.002,
            Self::GlacierIce => -0.010,
            Self::MegacityArcology => 0.005,
            Self::ReclaimedLand => 0.001,
            Self::Natural | Self::Cropland => 0.0,
        }
    }
}

/// Interactive brush & construction actions used in Earth Sandbox mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EarthBrushAction {
    UpliftMountain,
    CarveCanal,
    ReclaimLand,
    PlantForest,
    BuildSolarHub,
    CarveLake,
    BuildSeawall,
    BuildArcology,
}

impl EarthBrushAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::UpliftMountain => "Uplift",
            Self::CarveCanal => "Carve",
            Self::ReclaimLand => "Reclaim",
            Self::PlantForest => "Forest",
            Self::BuildSolarHub => "Solar",
            Self::CarveLake => "Lake",
            Self::BuildSeawall => "Seawall",
            Self::BuildArcology => "CityHub",
        }
    }

    pub fn cost_billion(&self) -> f64 {
        match self {
            Self::UpliftMountain => 1.8,
            Self::CarveCanal => 1.5,
            Self::ReclaimLand => 2.4,
            Self::PlantForest => 0.6,
            Self::BuildSolarHub => 1.4,
            Self::CarveLake => 1.2,
            Self::BuildSeawall => 1.6,
            Self::BuildArcology => 3.0,
        }
    }

    pub fn ring_color(&self) -> Color32 {
        match self {
            Self::UpliftMountain => Color32::from_rgb(217, 119, 6),
            Self::CarveCanal => Color32::from_rgb(59, 130, 246),
            Self::ReclaimLand => Color32::from_rgb(52, 211, 153),
            Self::PlantForest => Color32::from_rgb(34, 197, 94),
            Self::BuildSolarHub => Color32::from_rgb(250, 204, 21),
            Self::CarveLake => Color32::from_rgb(14, 165, 233),
            Self::BuildSeawall => Color32::from_rgb(56, 189, 248),
            Self::BuildArcology => Color32::from_rgb(168, 85, 247),
        }
    }
}

/// Planetary megastructure archetypes drawn directly across the curved globe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MegastructureKind {
    TransStraitBridge,
    CoastalSeawallBarrier,
    ArcologyMegaTower,
    SolarCleanCorridor,
}

impl MegastructureKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TransStraitBridge => "Mega-Bridge",
            Self::CoastalSeawallBarrier => "Sea Barrier",
            Self::ArcologyMegaTower => "Mega-Tower",
            Self::SolarCleanCorridor => "Clean Grid",
        }
    }

    pub fn accent_color(&self) -> Color32 {
        match self {
            Self::TransStraitBridge => Color32::from_rgb(245, 158, 11),
            Self::CoastalSeawallBarrier => Color32::from_rgb(56, 189, 248),
            Self::ArcologyMegaTower => Color32::from_rgb(168, 85, 247),
            Self::SolarCleanCorridor => Color32::from_rgb(34, 197, 94),
        }
    }
}

/// Sparse terrain/biome modification record stored per `(lat_idx, lon_idx)` planetary cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarthCellMod {
    pub lat_idx: i32,
    pub lon_idx: i32,
    pub elev_delta_m: f32,
    pub biome: EarthBiome,
    pub protected_seawall: bool,
    pub urban_density: f32,
}

impl EarthCellMod {
    pub fn center_lat_lon(&self) -> (f64, f64) {
        (
            (self.lat_idx as f64 + 0.5) * EARTH_CELL_DEG,
            (self.lon_idx as f64 + 0.5) * EARTH_CELL_DEG,
        )
    }
}

/// Engineered planetary megastructure anchored on geodetic Earth coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarthMegastructure {
    pub id: usize,
    pub kind: MegastructureKind,
    pub name: String,
    pub start_lat_lon: (f64, f64),
    pub end_lat_lon: (f64, f64),
    pub height_m: f64,
    pub length_km: f64,
    pub cost_b_usd: f64,
    pub carbon_delta_mt_yr: f64,
    pub ari_resilience: f64,
}

/// Curated planetary engineering challenge scenarios alongside open Sandbox mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EarthScenario {
    #[default]
    FreeplaySandbox,
    CoastalDeltaDefense,
    GreenNusantaraCorridor,
    SaharaSolarOasis,
}

impl EarthScenario {
    pub fn all() -> &'static [EarthScenario] {
        &[
            Self::FreeplaySandbox,
            Self::CoastalDeltaDefense,
            Self::GreenNusantaraCorridor,
            Self::SaharaSolarOasis,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::FreeplaySandbox => "Freeplay Earth",
            Self::CoastalDeltaDefense => "Coast +3m Defense",
            Self::GreenNusantaraCorridor => "Green Nusantara",
            Self::SaharaSolarOasis => "Sahara Solar Oasis",
        }
    }

    pub fn goal_summary(&self) -> &'static str {
        match self {
            Self::FreeplaySandbox => "Freeform terraforming, sea-level sculpt & global megaprojects",
            Self::CoastalDeltaDefense => "At +3.0m Sea Level: build >=8 Seawall/Reclaim tiles & keep Habitability >=75",
            Self::GreenNusantaraCorridor => "Plant >=12 Forest/Solar cells, >=2 Mega-Bridges & drop CO2 <=415 ppm",
            Self::SaharaSolarOasis => "Deploy >=10 Solar/Lake/Forest cells in North Africa & reach >=20 GW clean power",
        }
    }

    pub fn preset_focus(&self) -> (f64, f64, f32) {
        match self {
            Self::FreeplaySandbox => (5.0, 98.0, 1.65),
            Self::CoastalDeltaDefense => (-6.2, 106.8, 5.2),
            Self::GreenNusantaraCorridor => (-1.5, 112.0, 4.4),
            Self::SaharaSolarOasis => (23.0, 13.0, 3.8),
        }
    }
}

/// Real-time planetary health, climate, sea-level, and engineering economy telemetry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryVitals {
    pub year: u32,
    pub sea_level_m: f32,
    pub co2_ppm: f64,
    pub temp_anomaly_c: f64,
    pub habitability_score: f64,
    pub budget_billion_usd: f64,
    pub flooded_coast_cells: usize,
    pub seawall_count: usize,
    pub forest_cells: usize,
    pub clean_gw: f64,
    pub reclaimed_km2: f64,
}

impl Default for PlanetaryVitals {
    fn default() -> Self {
        Self {
            year: 2026,
            sea_level_m: 0.0,
            co2_ppm: 424.0,
            temp_anomaly_c: 1.35,
            habitability_score: 78.0,
            budget_billion_usd: 250.0,
            flooded_coast_cells: 0,
            seawall_count: 0,
            forest_cells: 0,
            clean_gw: 12.0,
            reclaimed_km2: 0.0,
        }
    }
}

/// Smooth fly-to camera animation state between globe viewpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobeFlyTarget {
    pub start_lat_lon: (f64, f64),
    pub end_lat_lon: (f64, f64),
    pub start_zoom: f32,
    pub end_zoom: f32,
    pub progress: f32,
    pub label: String,
}

/// 3D Google Earth-style orbital camera orientation and atmospheric lighting controls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobeCamera {
    pub tilt_deg: f32,
    pub heading_deg: f32,
    pub auto_spin: bool,
    pub spin_deg_per_sec: f32,
    pub angular_vel_lat: f32,
    pub angular_vel_lon: f32,
    pub day_night_lighting: bool,
    pub sun_lon_deg: f32,
    #[serde(skip)]
    pub fly_to: Option<GlobeFlyTarget>,
}

impl Default for GlobeCamera {
    fn default() -> Self {
        Self {
            tilt_deg: 18.0,
            heading_deg: 0.0,
            auto_spin: false,
            spin_deg_per_sec: 4.5,
            angular_vel_lat: 0.0,
            angular_vel_lon: 0.0,
            day_night_lighting: true,
            sun_lon_deg: 105.0,
            fly_to: None,
        }
    }
}

/// JSON-safe serializer for `BTreeMap<(i32, i32), EarthCellMod>` (emits ordered array of cells).
mod cell_map_serde {
    use super::EarthCellMod;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<(i32, i32), EarthCellMod>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let vec: Vec<&EarthCellMod> = map.values().collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<(i32, i32), EarthCellMod>, D::Error> {
        let vec = Vec::<EarthCellMod>::deserialize(deserializer)?;
        Ok(vec
            .into_iter()
            .map(|c| ((c.lat_idx, c.lon_idx), c))
            .collect())
    }
}

/// Snapshot stored in Earth Undo/Redo stacks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarthEditSnapshot {
    #[serde(with = "cell_map_serde")]
    pub cells: BTreeMap<(i32, i32), EarthCellMod>,
    pub megastructures: Vec<EarthMegastructure>,
    pub next_mega_id: usize,
    pub sea_level_m: f32,
    pub budget_billion_usd: f64,
}

/// Complete persistent planetary game & modification world state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryWorldState {
    pub camera: GlobeCamera,
    #[serde(with = "cell_map_serde")]
    pub cells: BTreeMap<(i32, i32), EarthCellMod>,
    pub megastructures: Vec<EarthMegastructure>,
    pub next_mega_id: usize,
    pub scenario: EarthScenario,
    pub sim_playing: bool,
    pub sim_accum_secs: f32,
    pub brush_radius_cells: usize,
    pub unlimited_sandbox_budget: bool,
    pub show_atmosphere: bool,
    pub show_hotspots: bool,
    pub vitals: PlanetaryVitals,
    #[serde(skip)]
    pub undo_stack: Vec<EarthEditSnapshot>,
    #[serde(skip)]
    pub redo_stack: Vec<EarthEditSnapshot>,
}

impl Default for PlanetaryWorldState {
    fn default() -> Self {
        let mut world = Self {
            camera: GlobeCamera::default(),
            cells: BTreeMap::new(),
            megastructures: Vec::new(),
            next_mega_id: 1,
            scenario: EarthScenario::FreeplaySandbox,
            sim_playing: false,
            sim_accum_secs: 0.0,
            brush_radius_cells: 1,
            unlimited_sandbox_budget: true,
            show_atmosphere: true,
            show_hotspots: true,
            vitals: PlanetaryVitals::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        world.recompute_vitals();
        world
    }
}

/// Precomputed double-precision spherical projection & inverse ray-sphere picker for a viewport `Rect`.
#[derive(Debug, Clone)]
pub struct GlobeScreenTransform {
    pub rect: Rect,
    pub center_lat_deg: f64,
    pub center_lon_deg: f64,
    pub zoom_level: f32,
    pub tilt_rad: f64,
    pub heading_rad: f64,
    pub d_surf: f64,
    pub f_px: f64,
    pub cam_dir_world: [f64; 3],
    pub cam_pos_h: [f64; 3],
    pub sun_dir_world: [f64; 3],
}

impl GlobeScreenTransform {
    /// Builds a calibrated `f64` perspective camera targeting `(center_lat, center_lon)` at `zoom_level`.
    pub fn from_view(
        rect: Rect,
        center_lat: f64,
        center_lon: f64,
        zoom_level: f32,
        camera: &GlobeCamera,
    ) -> Self {
        let min_dim = rect.width().min(rect.height()).max(120.0) as f64;
        let f_px = min_dim * 2.15;
        let z_clamped = (zoom_level as f64).clamp(1.0, 20.5);
        // Calibrated so zoom=1.65 frames the full Earth limb at ~85% of viewport height and z=17.5 reaches ~0.4 m/px site scale
        let d_surf = (4.15 / 2.0_f64.powf(z_clamped - 1.65)).max(8.0e-6);
        let tilt_deg = if z_clamped < 2.2 {
            (camera.tilt_deg * ((z_clamped - 1.0) / 1.2).clamp(0.25, 1.0) as f32).clamp(0.0, 68.0)
        } else {
            camera.tilt_deg.clamp(0.0, 68.0)
        };
        let tilt_rad = (tilt_deg as f64).to_radians();
        let heading_rad = (camera.heading_deg as f64).to_radians();

        let sun_lon_rad = (camera.sun_lon_deg as f64).to_radians();
        let sun_lat_rad = 18.0_f64.to_radians();
        let sun_dir_world = [
            sun_lat_rad.cos() * sun_lon_rad.sin(),
            sun_lat_rad.sin(),
            sun_lat_rad.cos() * sun_lon_rad.cos(),
        ];

        let cam_pos_h = [
            0.0,
            -d_surf * tilt_rad.sin(),
            1.0 + d_surf * tilt_rad.cos(),
        ];
        let mut xform = Self {
            rect,
            center_lat_deg: center_lat.clamp(-88.0, 88.0),
            center_lon_deg: center_lon,
            zoom_level,
            tilt_rad,
            heading_rad,
            d_surf,
            f_px,
            cam_dir_world: [0.0, 0.0, 1.0],
            cam_pos_h,
            sun_dir_world,
        };
        let raw_cam_dir = xform.tangent_h_to_world(normalize3(cam_pos_h));
        xform.cam_dir_world = normalize3(raw_cam_dir);
        xform
    }

    /// Apparent Earth limb radius on screen in pixels when `d_surf > 0` (`R = 1` unit sphere).
    pub fn globe_limb_radius_px(&self) -> f32 {
        let dist = 1.0 + self.d_surf;
        let denom = (dist * dist - 1.0).max(1e-6).sqrt();
        (self.f_px / denom) as f32
    }

    /// Exact horizontal pan degrees per dragged screen pixel at the current orbital surface distance.
    /// Scales linearly with `d_surf / f_px` so surface drag stays 1:1 with the cursor across all zooms.
    pub fn deg_per_pixel(&self) -> f64 {
        let raw = (self.d_surf / self.f_px.max(120.0)) * (180.0 / std::f64::consts::PI);
        raw.clamp(1e-8, 0.45)
    }

    /// Vertical pan degrees per dragged pixel compensating oblique camera tilt foreshortening.
    pub fn deg_per_pixel_y(&self) -> f64 {
        let cos_tilt = self.tilt_rad.cos().max(0.32);
        (self.deg_per_pixel() / cos_tilt).clamp(1e-8, 0.65)
    }

    fn world_to_tangent_h(&self, v: [f64; 3]) -> [f64; 3] {
        let c_lon = self.center_lon_deg.to_radians();
        let c_lat = self.center_lat_deg.to_radians();
        let (slon, clon) = (c_lon.sin(), c_lon.cos());
        let (slat, clat) = (c_lat.sin(), c_lat.cos());

        // 1. Rotate around Y by -c_lon
        let x1 = v[0] * clon - v[2] * slon;
        let y1 = v[1];
        let z1 = v[0] * slon + v[2] * clon;

        // 2. Rotate around X by +c_lat
        let x2 = x1;
        let y2 = y1 * clat - z1 * slat;
        let z2 = y1 * slat + z1 * clat;

        // 3. Heading rotation around Z
        let (sh, ch) = (self.heading_rad.sin(), self.heading_rad.cos());
        let xh = x2 * ch - y2 * sh;
        let yh = x2 * sh + y2 * ch;
        [xh, yh, z2]
    }

    fn tangent_h_to_world(&self, v_h: [f64; 3]) -> [f64; 3] {
        let (sh, ch) = (self.heading_rad.sin(), self.heading_rad.cos());
        let x2 = v_h[0] * ch + v_h[1] * sh;
        let y2 = -v_h[0] * sh + v_h[1] * ch;
        let z2 = v_h[2];

        let c_lon = self.center_lon_deg.to_radians();
        let c_lat = self.center_lat_deg.to_radians();
        let (slon, clon) = (c_lon.sin(), c_lon.cos());
        let (slat, clat) = (c_lat.sin(), c_lat.cos());

        let x1 = x2;
        let y1 = y2 * clat + z2 * slat;
        let z1 = -y2 * slat + z2 * clat;

        [x1 * clon + z1 * slon, y1, -x1 * slon + z1 * clon]
    }

    fn project_tangent_h(&self, p_h: [f64; 3]) -> Option<(Pos2, f64, bool)> {
        // Front-facing test on unit sphere horizon: dot(P_h, C_h) >= |P_h|^2 - eps
        let r_sq = p_h[0] * p_h[0] + p_h[1] * p_h[1] + p_h[2] * p_h[2];
        let horizon_dot =
            p_h[0] * self.cam_pos_h[0] + p_h[1] * self.cam_pos_h[1] + p_h[2] * self.cam_pos_h[2];
        let front = horizon_dot >= r_sq - 0.015;

        let qx = p_h[0];
        let qy = p_h[1];
        let qz = p_h[2] - 1.0;

        let (st, ct) = (self.tilt_rad.sin(), self.tilt_rad.cos());
        let xt = qx;
        let yt = qy * ct + qz * st;
        let zt = -qy * st + qz * ct;

        let cam_z = self.d_surf - zt;
        if cam_z <= 1e-6 {
            return None;
        }
        let center = self.rect.center();
        let sx = center.x as f64 + self.f_px * (xt / cam_z);
        let sy = center.y as f64 - self.f_px * (yt / cam_z);
        Some((Pos2::new(sx as f32, sy as f32), cam_z, front))
    }

    /// Projects geographic `(lat_deg, lon_deg, elev_m)` to `(screen_pos, cam_depth, is_visible_front_hemisphere)`.
    pub fn project_lat_lon_elev(
        &self,
        lat_deg: f64,
        lon_deg: f64,
        elev_m: f64,
    ) -> Option<(Pos2, f64, bool)> {
        let mut p_world = lat_lon_to_unit_sphere(lat_deg, lon_deg);
        let r_scale = 1.0 + (elev_m / EARTH_RADIUS_METERS);
        p_world[0] *= r_scale;
        p_world[1] *= r_scale;
        p_world[2] *= r_scale;
        let p_h = self.world_to_tangent_h(p_world);
        self.project_tangent_h(p_h)
    }

    /// Exact double-precision ray-sphere unprojection from screen pixel to `(lat_deg, lon_deg, hit_globe)`.
    pub fn screen_to_lat_lon(&self, pos: Pos2) -> (f64, f64, bool) {
        let center = self.rect.center();
        let dx = (pos.x as f64 - center.x as f64) / self.f_px;
        let dy = (center.y as f64 - pos.y as f64) / self.f_px;
        let d_t = normalize3([dx, dy, -1.0]);

        let (st, ct) = (self.tilt_rad.sin(), self.tilt_rad.cos());
        let d_h = [
            d_t[0],
            d_t[1] * ct - d_t[2] * st,
            d_t[1] * st + d_t[2] * ct,
        ];

        let o_w = self.tangent_h_to_world(self.cam_pos_h);
        let d_w = normalize3(self.tangent_h_to_world(d_h));

        let b = 2.0 * (o_w[0] * d_w[0] + o_w[1] * d_w[1] + o_w[2] * d_w[2]);
        let c = (o_w[0] * o_w[0] + o_w[1] * o_w[1] + o_w[2] * o_w[2]) - 1.0;
        let disc = b * b - 4.0 * c;
        if disc >= 0.0 {
            let t = 0.5 * (-b - disc.sqrt());
            if t > 0.0 {
                let hit = [
                    o_w[0] + t * d_w[0],
                    o_w[1] + t * d_w[1],
                    o_w[2] + t * d_w[2],
                ];
                let (lat, lon) = unit_sphere_to_lat_lon(hit);
                return (lat, lon, true);
            }
        }
        // Outside limb -> project closest limb horizon point smoothly
        let t_limb = -0.5 * b;
        let closest = normalize3([
            o_w[0] + t_limb * d_w[0],
            o_w[1] + t_limb * d_w[1],
            o_w[2] + t_limb * d_w[2],
        ]);
        let (lat, lon) = unit_sphere_to_lat_lon(closest);
        (lat, lon, false)
    }

    /// Solar illumination factor `[0.0..1.0]` for unit normal `world_unit`.
    pub fn solar_dot(&self, world_unit: [f64; 3]) -> f64 {
        (world_unit[0] * self.sun_dir_world[0]
            + world_unit[1] * self.sun_dir_world[1]
            + world_unit[2] * self.sun_dir_world[2])
            .clamp(-1.0, 1.0)
    }
}

/// Converts WGS84 `(lat_deg, lon_deg)` to unit sphere `[x, y, z]` (`Y` = North Pole, `Z` = Prime Meridian).
pub fn lat_lon_to_unit_sphere(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let lat = lat_deg.clamp(-89.999, 89.999).to_radians();
    let lon = lon_deg.to_radians();
    [lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos()]
}

/// Inverse of `lat_lon_to_unit_sphere`.
pub fn unit_sphere_to_lat_lon(v: [f64; 3]) -> (f64, f64) {
    let n = normalize3(v);
    let lat = n[1].clamp(-1.0, 1.0).asin().to_degrees();
    let lon = n[0].atan2(n[2]).to_degrees();
    (lat, lon)
}

/// Samples `steps` waypoints along the spherical Great-Circle geodesic between `p1` and `p2`.
pub fn great_circle_arc(p1: (f64, f64), p2: (f64, f64), steps: usize) -> Vec<(f64, f64)> {
    let n = steps.max(1);
    let a = lat_lon_to_unit_sphere(p1.0, p1.1);
    let b = lat_lon_to_unit_sphere(p2.0, p2.1);
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    let omega = dot.acos();
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let v = if omega < 1e-6 {
            normalize3([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ])
        } else {
            let sin_o = omega.sin();
            let wa = ((1.0 - t) * omega).sin() / sin_o;
            let wb = (t * omega).sin() / sin_o;
            [
                wa * a[0] + wb * b[0],
                wa * a[1] + wb * b[1],
                wa * a[2] + wb * b[2],
            ]
        };
        pts.push(unit_sphere_to_lat_lon(v));
    }
    pts
}

fn normalize3(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-12 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Quantizes `(lat, lon)` to sparse planetary modification cell `(lat_idx, lon_idx)`.
pub fn lat_lon_to_cell_key(lat_deg: f64, lon_deg: f64) -> (i32, i32) {
    let lat_c = lat_deg.clamp(-88.0, 88.0);
    let lon_w = crate::core::gis::WebMercator::normalize_lon(lon_deg);
    let li = (lat_c / EARTH_CELL_DEG).floor() as i32;
    let lj = (lon_w / EARTH_CELL_DEG).floor() as i32;
    (li, lj)
}

/// Fast point-in-landmass test + deterministic planetary baseline elevation estimator (meters).
pub fn estimate_baseline_elevation_m(lat_deg: f64, lon_deg: f64) -> f32 {
    if is_point_on_landmass(lat_deg, lon_deg) {
        // Major mountain chains & plateau reliefs
        let himalaya = gaussian_bump(lat_deg, lon_deg, 32.0, 85.0, 9.0, 16.0) * 4200.0;
        let andes = gaussian_bump(lat_deg, lon_deg, -18.0, -68.0, 16.0, 4.5) * 3200.0;
        let rockies = gaussian_bump(lat_deg, lon_deg, 42.0, -110.0, 11.0, 6.5) * 2100.0;
        let alps = gaussian_bump(lat_deg, lon_deg, 46.5, 10.0, 3.0, 5.5) * 1800.0;
        let papua_highland = gaussian_bump(lat_deg, lon_deg, -4.0, 138.0, 2.5, 4.5) * 1900.0;
        let bukit_barisan = gaussian_bump(lat_deg, lon_deg, -1.5, 101.0, 5.5, 1.8) * 1100.0;
        let ripple = ((lat_deg * 0.65).sin() * (lon_deg * 0.45).cos()).abs() * 320.0 + 45.0;
        (ripple + himalaya + andes + rockies + alps + papua_highland + bukit_barisan) as f32
    } else {
        let shelf = (((lat_deg * 0.4).cos() * (lon_deg * 0.3).sin()) * 420.0 - 1400.0) as f32;
        shelf.clamp(-3800.0, -80.0)
    }
}

fn gaussian_bump(lat: f64, lon: f64, clat: f64, clon: f64, slat: f64, slon: f64) -> f64 {
    let dy = (lat - clat) / slat;
    let dx = crate::core::gis::WebMercator::normalize_lon(lon - clon) / slon;
    (-(dx * dx + dy * dy)).exp()
}

/// Ray-cast point-in-polygon check against `WORLD_LANDMASSES` coastline dataset.
pub fn is_point_on_landmass(lat_deg: f64, lon_deg: f64) -> bool {
    let lon_w = crate::core::gis::WebMercator::normalize_lon(lon_deg);
    for poly in WORLD_LANDMASSES {
        if poly.len() < 3 {
            continue;
        }
        let mut inside = false;
        let mut j = poly.len() - 1;
        for i in 0..poly.len() {
            let (yi, xi) = poly[i];
            let (yj, xj) = poly[j];
            let intersects = ((yi > lat_deg) != (yj > lat_deg))
                && (lon_w < (xj - xi) * (lat_deg - yi) / ((yj - yi) + 1e-12) + xi);
            if intersects {
                inside = !inside;
            }
            j = i;
        }
        if inside {
            return true;
        }
    }
    false
}

/// Deterministic natural coastal sample checkpoints used to measure global sea-level inundation.
const COASTAL_BENCHMARKS: &[(&str, f64, f64, f32)] = &[
    ("Jakarta Delta", -6.15, 106.85, 1.8),
    ("Semarang Coast", -6.95, 110.42, 2.2),
    ("Bangkok Chao Phraya", 13.70, 100.50, 2.0),
    ("Ho Chi Minh Mekong", 10.65, 106.70, 2.1),
    ("Dhaka Bengal", 22.80, 89.80, 2.5),
    ("Shanghai Yangtze", 31.20, 121.50, 3.0),
    ("Tokyo Bay", 35.60, 139.78, 3.5),
    ("Netherlands Rhine", 52.10, 4.50, 1.5),
    ("Miami Biscayne", 25.78, -80.18, 2.0),
    ("Alexandria Nile", 31.20, 29.92, 2.4),
    ("Venice Lagoon", 45.43, 12.33, 1.6),
    ("Singapore Marina", 1.28, 103.85, 3.2),
];

/// Notable world locations highlighted when `show_hotspots` is enabled on the 3D Globe.
const GLOBE_HOTSPOTS: &[(&str, f64, f64)] = &[
    ("Jakarta", -6.2088, 106.8456),
    ("IKN Nusantara", -0.9730, 116.7090),
    ("Singapore", 1.3521, 103.8198),
    ("Tokyo", 35.6762, 139.6503),
    ("London", 51.5072, -0.1276),
    ("New York", 40.7128, -74.0060),
    ("Cairo", 30.0444, 31.2357),
    ("Sydney", -33.8688, 151.2093),
];

impl PlanetaryWorldState {
    pub fn snapshot(&self) -> EarthEditSnapshot {
        EarthEditSnapshot {
            cells: self.cells.clone(),
            megastructures: self.megastructures.clone(),
            next_mega_id: self.next_mega_id,
            sea_level_m: self.vitals.sea_level_m,
            budget_billion_usd: self.vitals.budget_billion_usd,
        }
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push(self.snapshot());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo_earth(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.snapshot());
            self.restore_snapshot(prev);
            true
        } else {
            false
        }
    }

    pub fn redo_earth(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.snapshot());
            self.restore_snapshot(next);
            true
        } else {
            false
        }
    }

    fn restore_snapshot(&mut self, snap: EarthEditSnapshot) {
        self.cells = snap.cells;
        self.megastructures = snap.megastructures;
        self.next_mega_id = snap.next_mega_id;
        self.vitals.sea_level_m = snap.sea_level_m;
        self.vitals.budget_billion_usd = snap.budget_billion_usd;
        self.recompute_vitals();
    }

    pub fn reset_earth(&mut self) {
        self.push_undo();
        self.cells.clear();
        self.megastructures.clear();
        self.next_mega_id = 1;
        self.vitals = PlanetaryVitals::default();
        self.recompute_vitals();
    }

    /// Loads one of the playable scenarios (`CoastalDeltaDefense`, `GreenNusantaraCorridor`, etc.) and sets sea/target baseline.
    pub fn load_scenario(&mut self, scenario: EarthScenario) -> (f64, f64, f32) {
        self.push_undo();
        self.scenario = scenario;
        self.cells.clear();
        self.megastructures.clear();
        self.next_mega_id = 1;
        self.vitals = PlanetaryVitals::default();
        if scenario == EarthScenario::CoastalDeltaDefense {
            self.vitals.sea_level_m = 3.0;
        }
        self.recompute_vitals();
        scenario.preset_focus()
    }

    /// Total effective elevation `(baseline + local terraform delta)` at `(lat_deg, lon_deg)`.
    pub fn effective_elevation_m(&self, lat_deg: f64, lon_deg: f64) -> f32 {
        let key = lat_lon_to_cell_key(lat_deg, lon_deg);
        let base = estimate_baseline_elevation_m(lat_deg, lon_deg);
        let delta = self.cells.get(&key).map_or(0.0, |c| c.elev_delta_m);
        base + delta
    }

    /// Applies an Earth terraforming brush stroke centered at `(lat_deg, lon_deg)`.
    /// Returns the number of planetary cells modified.
    pub fn apply_brush(
        &mut self,
        lat_deg: f64,
        lon_deg: f64,
        action: EarthBrushAction,
        push_history: bool,
    ) -> usize {
        if !self.unlimited_sandbox_budget && self.vitals.budget_billion_usd < action.cost_billion() {
            return 0;
        }
        if push_history {
            self.push_undo();
        }
        let (ci, cj) = lat_lon_to_cell_key(lat_deg, lon_deg);
        let r = (self.brush_radius_cells as i32).clamp(0, 3);
        let mut modified = 0usize;

        for di in -r..=r {
            for dj in -r..=r {
                if r > 0 && (di * di + dj * dj) > (r * r + r) {
                    continue;
                }
                let falloff = 1.0_f32 / (1.0 + 0.45 * ((di * di + dj * dj) as f32).sqrt());
                let li = (ci + di).clamp(-58, 57);
                let lon_cells = (360.0 / EARTH_CELL_DEG).round() as i32;
                let lj_raw = cj + dj;
                let lj = (lj_raw + lon_cells / 2).rem_euclid(lon_cells) - lon_cells / 2;
                let entry = self.cells.entry((li, lj)).or_insert_with(|| EarthCellMod {
                    lat_idx: li,
                    lon_idx: lj,
                    elev_delta_m: 0.0,
                    biome: EarthBiome::Natural,
                    protected_seawall: false,
                    urban_density: 0.0,
                });
                let (cell_lat, cell_lon) = entry.center_lat_lon();
                let base_el = estimate_baseline_elevation_m(cell_lat, cell_lon);
                match action {
                    EarthBrushAction::UpliftMountain => {
                        entry.elev_delta_m = (entry.elev_delta_m + 950.0 * falloff).clamp(-8000.0, 7500.0);
                    }
                    EarthBrushAction::CarveCanal => {
                        entry.elev_delta_m = (entry.elev_delta_m - 850.0 * falloff).clamp(-8000.0, 7500.0);
                    }
                    EarthBrushAction::ReclaimLand => {
                        let min_needed = (self.vitals.sea_level_m + 18.0) - base_el;
                        if entry.elev_delta_m < min_needed {
                            entry.elev_delta_m = min_needed.clamp(-2000.0, 6500.0);
                        } else {
                            entry.elev_delta_m = (entry.elev_delta_m + 120.0 * falloff).min(6500.0);
                        }
                        entry.biome = EarthBiome::ReclaimedLand;
                        entry.protected_seawall = true;
                    }
                    EarthBrushAction::PlantForest => {
                        entry.biome = EarthBiome::ForestBelt;
                    }
                    EarthBrushAction::BuildSolarHub => {
                        entry.biome = EarthBiome::SolarFarm;
                    }
                    EarthBrushAction::CarveLake => {
                        entry.biome = EarthBiome::ReservoirLake;
                        entry.elev_delta_m = (entry.elev_delta_m - 180.0 * falloff).clamp(-8000.0, 7500.0);
                    }
                    EarthBrushAction::BuildSeawall => {
                        entry.protected_seawall = true;
                        entry.elev_delta_m = (entry.elev_delta_m + 45.0 * falloff).clamp(-8000.0, 7500.0);
                    }
                    EarthBrushAction::BuildArcology => {
                        entry.biome = EarthBiome::MegacityArcology;
                        entry.urban_density = (entry.urban_density + 0.5).min(1.0);
                        let min_needed = (self.vitals.sea_level_m + 25.0) - base_el;
                        if entry.elev_delta_m < min_needed {
                            entry.elev_delta_m = min_needed.clamp(0.0, 6500.0);
                        }
                        entry.protected_seawall = true;
                    }
                }
                modified += 1;
            }
        }
        if !self.unlimited_sandbox_budget && modified > 0 {
            self.vitals.budget_billion_usd =
                (self.vitals.budget_billion_usd - action.cost_billion()).max(0.0);
        }
        self.recompute_vitals();
        modified
    }

    /// Places a planetary megastructure (`Mega-Bridge`, `Seawall`, `Mega-Tower`, `Clean Grid`) on the globe.
    pub fn add_megastructure(
        &mut self,
        kind: MegastructureKind,
        start_lat_lon: (f64, f64),
        end_lat_lon: (f64, f64),
    ) -> usize {
        self.push_undo();
        let len_m = SlippyMapState::geodetic_distance_m(start_lat_lon, end_lat_lon);
        let len_km = (len_m / 1000.0).max(1.0);
        let (height_m, cost_b, carbon_mt, ari) = match kind {
            MegastructureKind::TransStraitBridge => (
                260.0 + (len_km * 1.4).min(600.0),
                (len_km * 0.085).clamp(2.5, 85.0),
                -3.5,
                0.86,
            ),
            MegastructureKind::CoastalSeawallBarrier => {
                // Protect touched cells along barrier arc
                for pt in great_circle_arc(start_lat_lon, end_lat_lon, 10) {
                    let key = lat_lon_to_cell_key(pt.0, pt.1);
                    let cell = self.cells.entry(key).or_insert_with(|| EarthCellMod {
                        lat_idx: key.0,
                        lon_idx: key.1,
                        elev_delta_m: 40.0,
                        biome: EarthBiome::Natural,
                        protected_seawall: true,
                        urban_density: 0.0,
                    });
                    cell.protected_seawall = true;
                }
                (48.0, (len_km * 0.035).clamp(1.2, 35.0), -1.2, 0.91)
            }
            MegastructureKind::ArcologyMegaTower => (1450.0, 8.5, 4.0, 0.89),
            MegastructureKind::SolarCleanCorridor => (
                24.0,
                (len_km * 0.045).clamp(2.0, 40.0),
                -(len_km * 0.18).clamp(8.0, 95.0),
                0.93,
            ),
        };
        let id = self.next_mega_id;
        self.next_mega_id += 1;
        let name = format!("{} #{id}", kind.label());
        self.megastructures.push(EarthMegastructure {
            id,
            kind,
            name,
            start_lat_lon,
            end_lat_lon,
            height_m,
            length_km: len_km,
            cost_b_usd: cost_b,
            carbon_delta_mt_yr: carbon_mt,
            ari_resilience: ari,
        });
        if !self.unlimited_sandbox_budget {
            self.vitals.budget_billion_usd =
                (self.vitals.budget_billion_usd - cost_b).max(0.0);
        }
        self.recompute_vitals();
        id
    }

    /// Recomputes live planetary climate, sea-level flooding, carbon balance, and habitability score.
    pub fn recompute_vitals(&mut self) {
        let mut net_carbon_mt = 0.0_f64;
        let mut thermal_delta = 0.0_f64;
        let mut seawall_count = 0usize;
        let mut forest_cells = 0usize;
        let mut clean_gw = 12.0_f64;
        let mut reclaimed_km2 = 0.0_f64;

        for cell in self.cells.values() {
            net_carbon_mt += cell.biome.annual_carbon_mt();
            thermal_delta += cell.biome.thermal_factor();
            if cell.protected_seawall {
                seawall_count += 1;
            }
            if cell.biome == EarthBiome::ForestBelt {
                forest_cells += 1;
            }
            if cell.biome == EarthBiome::SolarFarm {
                clean_gw += 4.5;
            }
            if cell.biome == EarthBiome::ReclaimedLand {
                reclaimed_km2 += 420.0;
            }
        }
        for mega in &self.megastructures {
            net_carbon_mt += mega.carbon_delta_mt_yr;
            if mega.kind == MegastructureKind::SolarCleanCorridor {
                clean_gw += (mega.length_km * 0.25).clamp(2.0, 28.0);
            }
        }

        // Baseline atmospheric CO2 424 ppm shifted by planetary interventions
        let co2_ppm = (424.0 + net_carbon_mt * 0.045).clamp(280.0, 620.0);
        let co2_temp = (co2_ppm - 280.0) * 0.0094;
        let temp_anomaly = (co2_temp + thermal_delta).clamp(-0.5, 4.8);

        // Evaluate coastal benchmarks for sea-level flood defense
        let mut flooded = 0usize;
        for &(_, lat, lon, coast_el) in COASTAL_BENCHMARKS {
            let key = lat_lon_to_cell_key(lat, lon);
            let (delta_m, walled) = self
                .cells
                .get(&key)
                .map_or((0.0, false), |c| (c.elev_delta_m, c.protected_seawall));
            if !walled && (coast_el + delta_m) < self.vitals.sea_level_m {
                flooded += 1;
            }
        }
        // Plus check modified low coastal cells that aren't seawalled
        for cell in self.cells.values() {
            let (clat, clon) = cell.center_lat_lon();
            if is_point_on_landmass(clat, clon) && !cell.protected_seawall {
                let el = estimate_baseline_elevation_m(clat, clon) + cell.elev_delta_m;
                if el < self.vitals.sea_level_m {
                    flooded += 1;
                }
            }
        }

        let flood_penalty = (flooded as f64) * 3.8;
        let carbon_bonus = ((424.0 - co2_ppm) * 0.25).clamp(-18.0, 18.0);
        let infra_bonus = (self.megastructures.len() as f64 * 1.4).min(12.0);
        let hab = (82.0 + carbon_bonus + infra_bonus - flood_penalty - (temp_anomaly - 1.35).max(0.0) * 6.5)
            .clamp(10.0, 100.0);

        self.vitals.co2_ppm = co2_ppm;
        self.vitals.temp_anomaly_c = temp_anomaly;
        self.vitals.habitability_score = hab;
        self.vitals.flooded_coast_cells = flooded;
        self.vitals.seawall_count = seawall_count;
        self.vitals.forest_cells = forest_cells;
        self.vitals.clean_gw = clean_gw;
        self.vitals.reclaimed_km2 = reclaimed_km2;
    }

    /// Advances planetary game simulation by `years` ticks (updating climate drift, budget growth & ocean spin).
    pub fn tick_simulation_years(&mut self, years: u32) {
        for _ in 0..years {
            self.vitals.year += 1;
            let drift = ((self.vitals.temp_anomaly_c - 1.2).max(0.0) * 0.04) as f32;
            if drift > 0.0 {
                self.vitals.sea_level_m = (self.vitals.sea_level_m + drift).clamp(-50.0, 50.0);
            }
            self.vitals.budget_billion_usd = (self.vitals.budget_billion_usd + 18.0).min(9999.0);
        }
        self.recompute_vitals();
    }

    /// Evaluates active scenario objective completion `(progress 0..1, status_label, completed)`.
    pub fn scenario_progress(&self) -> (f32, String, bool) {
        match self.scenario {
            EarthScenario::FreeplaySandbox => (
                1.0,
                format!(
                    "Sandbox: {} cells sculpted • {} megastructures",
                    self.cells.len(),
                    self.megastructures.len()
                ),
                true,
            ),
            EarthScenario::CoastalDeltaDefense => {
                let walled = self.vitals.seawall_count.min(8) as f32 / 8.0;
                let hab_ok = self.vitals.habitability_score >= 75.0;
                let done = self.vitals.seawall_count >= 8 && hab_ok;
                let prog = (0.7 * walled + if hab_ok { 0.3 } else { 0.0 }).clamp(0.0, 1.0);
                (
                    prog,
                    format!(
                        "Defenses {}/8 • Hab {:.0}/75{}",
                        self.vitals.seawall_count,
                        self.vitals.habitability_score,
                        if done { " [COMPLETE]" } else { "" }
                    ),
                    done,
                )
            }
            EarthScenario::GreenNusantaraCorridor => {
                let bio = self
                    .cells
                    .values()
                    .filter(|c| matches!(c.biome, EarthBiome::ForestBelt | EarthBiome::SolarFarm))
                    .count();
                let bridges = self
                    .megastructures
                    .iter()
                    .filter(|m| m.kind == MegastructureKind::TransStraitBridge)
                    .count();
                let co2_ok = self.vitals.co2_ppm <= 415.0;
                let done = bio >= 12 && bridges >= 2 && co2_ok;
                let prog = (0.4 * (bio.min(12) as f32 / 12.0)
                    + 0.3 * (bridges.min(2) as f32 / 2.0)
                    + if co2_ok { 0.3 } else { 0.0 })
                .clamp(0.0, 1.0);
                (
                    prog,
                    format!(
                        "Green {}/12 • Bridges {}/2 • CO2 {:.0}<=415{}",
                        bio,
                        bridges,
                        self.vitals.co2_ppm,
                        if done { " [COMPLETE]" } else { "" }
                    ),
                    done,
                )
            }
            EarthScenario::SaharaSolarOasis => {
                let sahara_cells = self
                    .cells
                    .values()
                    .filter(|c| {
                        let (la, lo) = c.center_lat_lon();
                        (12.0..=34.0).contains(&la)
                            && (-15.0..=35.0).contains(&lo)
                            && matches!(
                                c.biome,
                                EarthBiome::SolarFarm
                                    | EarthBiome::ReservoirLake
                                    | EarthBiome::ForestBelt
                            )
                    })
                    .count();
                let done = sahara_cells >= 10 && self.vitals.clean_gw >= 20.0;
                let prog = (0.6 * (sahara_cells.min(10) as f32 / 10.0)
                    + 0.4 * ((self.vitals.clean_gw / 20.0).clamp(0.0, 1.0) as f32))
                    .clamp(0.0, 1.0);
                (
                    prog,
                    format!(
                        "Sahara Zones {}/10 • Clean {:.0}/20 GW{}",
                        sahara_cells,
                        self.vitals.clean_gw,
                        if done { " [COMPLETE]" } else { "" }
                    ),
                    done,
                )
            }
        }
    }

    /// Steps smooth globe spin, inertial momentum, animated fly-to camera arc, and planetary year ticks.
    pub fn step_animations(
        &mut self,
        ctx: &egui::Context,
        world_map: &mut SlippyMapState,
    ) {
        let dt = ctx.input(|i| i.unstable_dt).clamp(0.001, 0.05);
        let mut need_repaint = false;

        if let Some(mut fly) = self.camera.fly_to.take() {
            fly.progress = (fly.progress + dt * 1.45).min(1.0);
            let t = fly.progress;
            let smooth = t * t * (3.0 - 2.0 * t);
            let lat = fly.start_lat_lon.0 + (fly.end_lat_lon.0 - fly.start_lat_lon.0) * smooth as f64;
            let mut dlon = fly.end_lat_lon.1 - fly.start_lat_lon.1;
            if dlon > 180.0 {
                dlon -= 360.0;
            } else if dlon < -180.0 {
                dlon += 360.0;
            }
            let lon = fly.start_lat_lon.1 + dlon * smooth as f64;
            world_map.set_center(lat, lon);
            world_map.zoom_level = fly.start_zoom + (fly.end_zoom - fly.start_zoom) * smooth;
            if fly.progress < 1.0 {
                self.camera.fly_to = Some(fly);
                need_repaint = true;
            }
        } else {
            if self.camera.auto_spin {
                let next_lon = world_map.center_lat_lon.1 + (self.camera.spin_deg_per_sec * dt) as f64;
                world_map.set_center(world_map.center_lat_lon.0, next_lon);
                need_repaint = true;
            }
            let min_vel = (0.05 / 2.0_f32.powf((world_map.zoom_level - 2.0).max(0.0))).clamp(1e-6, 0.08);
            if world_map.zoom_level >= 11.0 {
                self.camera.angular_vel_lat = 0.0;
                self.camera.angular_vel_lon = 0.0;
            } else if self.camera.angular_vel_lat.abs() > min_vel
                || self.camera.angular_vel_lon.abs() > min_vel
            {
                let nlat = world_map.center_lat_lon.0 + (self.camera.angular_vel_lat * dt) as f64;
                let nlon = world_map.center_lat_lon.1 + (self.camera.angular_vel_lon * dt) as f64;
                world_map.set_center(nlat, nlon);
                let decay = (1.0 - dt * 6.0).max(0.0);
                self.camera.angular_vel_lat *= decay;
                self.camera.angular_vel_lon *= decay;
                need_repaint = true;
            } else {
                self.camera.angular_vel_lat = 0.0;
                self.camera.angular_vel_lon = 0.0;
            }
        }

        if self.sim_playing {
            self.sim_accum_secs += dt;
            if self.sim_accum_secs >= 1.2 {
                self.sim_accum_secs -= 1.2;
                self.tick_simulation_years(1);
            }
            need_repaint = true;
        }

        if need_repaint {
            ctx.request_repaint();
        }
    }

    /// Triggers a smooth animated Google Earth fly-to arc from the current view to `(lat, lon, target_zoom)`.
    pub fn start_fly_to(
        &mut self,
        world_map: &SlippyMapState,
        lat: f64,
        lon: f64,
        target_zoom: f32,
        label: impl Into<String>,
    ) {
        self.camera.angular_vel_lat = 0.0;
        self.camera.angular_vel_lon = 0.0;
        self.camera.fly_to = Some(GlobeFlyTarget {
            start_lat_lon: world_map.center_lat_lon,
            end_lat_lon: (lat, lon),
            start_zoom: world_map.zoom_level,
            end_zoom: target_zoom.clamp(1.0, 19.0),
            progress: 0.0,
            label: label.into(),
        });
    }

    /// Renders the interactive 3D Pure-Rust Vector Earth Globe with cosmos backdrop,
    /// atmosphere glow, sunlit day/night shading, embedded coastlines/ridges/rivers,
    /// buildable city-builder parcel grid, terraformed cells, megastructures, and brush reticle.
    #[allow(clippy::too_many_arguments)]
    pub fn render_3d_globe(
        &self,
        _ui: &egui::Ui,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        world_map: &SlippyMapState,
        ground_origin_lat_lon: (f64, f64),
        ground_origin_anchored: bool,
        show_grid: bool,
        active_brush: Option<EarthBrushAction>,
        cursor_lat_lon: (f64, f64),
        is_frozen: bool,
    ) -> GlobeScreenTransform {
        let gp = painter.with_clip_rect(rect);
        let xform = GlobeScreenTransform::from_view(
            rect,
            world_map.center_lat_lon.0,
            world_map.center_lat_lon.1,
            world_map.zoom_level,
            &self.camera,
        );

        let pal = world_map.tile_provider.palette();

        // 1. Deep Space Starfield Backdrop + Outer Atmosphere Limb Glow
        gp.rect_filled(rect, CornerRadius::same(10), pal.space_bg);
        paint_starfield(&gp, rect, world_map.center_lat_lon);

        let r_limb = xform.globe_limb_radius_px();
        let [ar, ag, ab] = pal.atmosphere_rgb;
        if self.show_atmosphere && xform.d_surf > 0.25 && r_limb < rect.height() * 2.2 {
            // Soft outer Rayleigh halo around the planetary limb
            let limb_center = xform
                .project_tangent_h([0.0, 0.0, 0.0])
                .map_or(rect.center(), |(p, _, _)| p);
            for (scale, alpha) in [(1.09_f32, 16_u8), (1.045, 34), (1.015, 64)] {
                gp.circle_filled(
                    limb_center,
                    r_limb * scale,
                    Color32::from_rgba_unmultiplied(ar, ag, ab, alpha),
                );
            }
        }

        // 2. Tessellated Lit Ocean Sphere Dome (handles arbitrary tilt, poles & horizon curvature)
        self.paint_ocean_sphere_dome(&gp, &xform, theme, world_map);

        // 3. Spherical Pure-Rust Vector Continents & Indonesian Archipelago (`WORLD_LANDMASSES`)
        self.paint_spherical_continents(&gp, &xform, theme, world_map);

        // 3b. Stylized Pure-Vector Topographic Mountain Ridges & Major River Corridors
        if world_map.zoom_level < 13.5 {
            self.paint_spherical_ridges_and_rivers(&gp, &xform, &pal, world_map.zoom_level);
        }

        // 4. Crisp Pure-Rust Vector Urban Corridors, Buildable Parcel Grid & Engineering Cadastre
        self.paint_vector_urban_and_cadastral_layer(
            &gp,
            &xform,
            theme,
            world_map,
            ground_origin_lat_lon,
            ground_origin_anchored,
        );

        // 5. Geodetic Graticule Arcs (parallels & meridians on front hemisphere)
        if show_grid {
            self.paint_spherical_graticule(&gp, &xform, &pal, world_map.zoom_level);
        }

        // 6. Planetary Terraformed Cells, Biomes, Flooded Zones & Seawalls
        self.paint_terraformed_cells(&gp, &xform);

        // 7. Engineered Planetary Megastructures (Great-Circle Bridges, Barriers, Mega-Towers)
        self.paint_megastructures(&gp, &xform);

        // 8. Geographic Hotspot Labels & Active BIM Site Beacon
        if self.show_hotspots && world_map.zoom_level < 11.5 {
            self.paint_hotspots_and_site_beacon(
                &gp,
                &xform,
                theme,
                world_map,
                ground_origin_lat_lon,
                ground_origin_anchored,
            );
        }

        // 9. Atmospheric Rim Crisp Stroke
        if self.show_atmosphere && xform.d_surf > 0.35 && r_limb < rect.height() * 2.0 {
            if let Some((limb_center, _, _)) = xform.project_tangent_h([0.0, 0.0, 0.0]) {
                gp.circle_stroke(
                    limb_center,
                    r_limb,
                    Stroke::new(1.5, Color32::from_rgba_unmultiplied(ar, ag, ab, 128)),
                );
            }
        }

        // 10. Geodesic Brush Footprint Ring when hovering with an Earth Terraform brush
        if !is_frozen {
            if let Some(brush) = active_brush {
                self.paint_brush_footprint(
                    &gp,
                    &xform,
                    cursor_lat_lon,
                    brush.ring_color(),
                );
            }
        }

        xform
    }

    /// Renders zero-blur full-Rust vector city corridors, buildable city-builder parcel blocks,
    /// adaptive metric cadastral grids (`150m`/`50m`/`20m`/`5m`/`2m`/`1m`), range rings, and
    /// directional compass bearings onto the 3D Pure Vector Earth surface.
    pub fn paint_vector_urban_and_cadastral_layer(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        theme: &MacTheme,
        world_map: &SlippyMapState,
        ground_origin_lat_lon: (f64, f64),
        ground_origin_anchored: bool,
    ) {
        let pal = world_map.tile_provider.palette();
        if world_map.zoom_level >= 3.0 && world_map.zoom_level < 13.5 {
            // Crisp continental vector city hubs & inter-city regional corridors
            let mut visible_hubs: Vec<(&str, f64, f64, Pos2)> = Vec::new();
            for &(city, lat, lon, _) in GLOBAL_CITY_CATALOG {
                if let Some((sp, _, true)) = xform.project_lat_lon_elev(lat, lon, 0.0) {
                    if xform.rect.expand(24.0).contains(sp) {
                        visible_hubs.push((city, lat, lon, sp));
                    }
                }
            }
            let corr_stroke = Stroke::new(1.0, pal.corridor_stroke);
            for i in 0..visible_hubs.len() {
                let (c1, la1, lo1, sp1) = visible_hubs[i];
                for &(c2, la2, lo2, sp2) in visible_hubs.iter().skip(i + 1).take(3) {
                    let dist_sq = (la2 - la1).powi(2) + (lo2 - lo1).powi(2);
                    if dist_sq < 64.0 && c1 != c2 {
                        painter.line_segment([sp1, sp2], corr_stroke);
                    }
                }
                painter.circle_filled(sp1, 2.7, pal.city_dot);
                if world_map.zoom_level >= 4.6 {
                    painter.text(
                        sp1 + Vec2::new(0.0, -5.0),
                        Align2::CENTER_BOTTOM,
                        c1,
                        FontId::proportional(9.4),
                        pal.city_label,
                    );
                }
            }
        }

        if world_map.zoom_level < 12.8 {
            return;
        }

        let origin = if ground_origin_anchored {
            ground_origin_lat_lon
        } else {
            (xform.center_lat_deg, xform.center_lon_deg)
        };
        let (step_m, half_span_m) = if world_map.zoom_level >= 19.2 {
            (1.0_f64, 16.0_f64)
        } else if world_map.zoom_level >= 18.2 {
            (2.0_f64, 30.0_f64)
        } else if world_map.zoom_level >= 17.0 {
            (5.0, 75.0)
        } else if world_map.zoom_level >= 15.6 {
            (20.0, 200.0)
        } else if world_map.zoom_level >= 14.2 {
            (50.0, 450.0)
        } else {
            (150.0, 1200.0)
        };

        let major_stroke = Stroke::new(1.0, pal.grid_major);
        let minor_stroke = Stroke::new(0.7, pal.grid_minor);
        let axis_n_stroke = Stroke::new(1.35, Color32::from_rgba_unmultiplied(34, 197, 94, 175));
        let axis_e_stroke = Stroke::new(1.35, Color32::from_rgba_unmultiplied(239, 68, 68, 175));

        let proj_m = |east_m: f64, north_m: f64| -> Option<Pos2> {
            let ll = SlippyMapState::world_mm_to_lat_lon(origin, [east_m * 1000.0, north_m * 1000.0]);
            xform
                .project_lat_lon_elev(ll.0, ll.1, 0.0)
                .and_then(|(p, _, v)| if v { Some(p) } else { None })
        };

        // Stylized buildable city-builder parcel pads & arterial/local vector avenues at city/site zoom
        if world_map.zoom_level >= 13.2 {
            let block_step = if world_map.zoom_level >= 16.5 { 60.0 } else { 180.0 };
            let road_half = block_step * 0.065;
            for bx in -2..=1 {
                for by in -2..=1 {
                    let x0 = bx as f64 * block_step + road_half;
                    let x1 = (bx + 1) as f64 * block_step - road_half;
                    let y0 = by as f64 * block_step + road_half;
                    let y1 = (by + 1) as f64 * block_step - road_half;
                    if let (Some(p0), Some(p1), Some(p2), Some(p3)) = (
                        proj_m(x0, y0),
                        proj_m(x1, y0),
                        proj_m(x1, y1),
                        proj_m(x0, y1),
                    ) {
                        painter.add(egui::Shape::convex_polygon(
                            vec![p0, p1, p2, p3],
                            pal.parcel_fill,
                            Stroke::new(0.7, pal.road_local),
                        ));
                    }
                }
            }
            let arterial_stroke = Stroke::new(2.2, pal.road_arterial);
            let local_stroke = Stroke::new(1.2, pal.road_local);
            let span = block_step * 3.0;
            for b in -3..=3 {
                let d = b as f64 * block_step;
                let st = if b % 2 == 0 { arterial_stroke } else { local_stroke };
                if let (Some(a), Some(b_pt)) = (proj_m(d, -span), proj_m(d, span)) {
                    painter.line_segment([a, b_pt], st);
                }
                if let (Some(a), Some(b_pt)) = (proj_m(-span, d), proj_m(span, d)) {
                    painter.line_segment([a, b_pt], st);
                }
            }
        }

        let lines_per_side = (half_span_m / step_m).round() as i32;
        for k in -lines_per_side..=lines_per_side {
            let d = k as f64 * step_m;
            let st_ns = if k == 0 {
                axis_n_stroke
            } else if k % 5 == 0 {
                major_stroke
            } else {
                minor_stroke
            };
            if let (Some(a), Some(b)) = (proj_m(d, -half_span_m), proj_m(d, half_span_m)) {
                painter.line_segment([a, b], st_ns);
            }
            let st_ew = if k == 0 {
                axis_e_stroke
            } else if k % 5 == 0 {
                major_stroke
            } else {
                minor_stroke
            };
            if let (Some(a), Some(b)) = (proj_m(-half_span_m, d), proj_m(half_span_m, d)) {
                painter.line_segment([a, b], st_ew);
            }
        }

        // Metric range ring & True North / East vector compass labels
        let ring_r_m = step_m * 3.0;
        let mut ring_pts = Vec::with_capacity(33);
        for seg in 0..=32 {
            let th = (seg as f64 / 32.0) * std::f64::consts::TAU;
            if let Some(p) = proj_m(ring_r_m * th.cos(), ring_r_m * th.sin()) {
                ring_pts.push(p);
            }
        }
        if ring_pts.len() >= 12 {
            painter.add(egui::Shape::line(
                ring_pts,
                Stroke::new(0.95, pal.grid_major),
            ));
        }
        for (lbl, ex, ny, col) in [
            (format!("N +{:.0}m", ring_r_m), 0.0, ring_r_m, theme.accent_green),
            (format!("E +{:.0}m", ring_r_m), ring_r_m, 0.0, theme.accent_red),
        ] {
            if let Some(p) = proj_m(ex, ny) {
                painter.text(p, Align2::CENTER_CENTER, lbl, FontId::monospace(9.0), col);
            }
        }
    }

    /// Drapes `TinSurfaceMesh` contours (`ContourPolyline`) and SNI 8460:2017 steep-slope hazard
    /// triangles (`> 30°`) cleanly onto the curved Earth 3D surface around `origin`.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_site_tin_contours_and_hazards(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        theme: &MacTheme,
        origin: (f64, f64),
        base_elev_m: f64,
        tin: &TinSurfaceMesh,
    ) {
        if xform.zoom_level < 12.0 || tin.points.is_empty() {
            return;
        }
        let (ref_z, _) = tin.sample_elevation_and_slope(0.0, 0.0);
        let project_local = |xm: f32, ym: f32, zm: f32| -> Option<Pos2> {
            let ll = SlippyMapState::world_mm_to_lat_lon(origin, [xm as f64 * 1000.0, ym as f64 * 1000.0]);
            let elev = base_elev_m + (zm - ref_z) as f64;
            xform
                .project_lat_lon_elev(ll.0, ll.1, elev)
                .and_then(|(p, _, v)| if v { Some(p) } else { None })
        };

        // 1. Highlight SNI 8460:2017 steep slope instability facets (> 30 deg)
        for tm in tin.triangle_metrics.iter().filter(|t| t.is_steep_hazard).take(900) {
            let p0 = tin.points[tm.indices[0]];
            let p1 = tin.points[tm.indices[1]];
            let p2 = tin.points[tm.indices[2]];
            if let (Some(s0), Some(s1), Some(s2)) = (
                project_local(p0.x, p0.y, p0.z),
                project_local(p1.x, p1.y, p1.z),
                project_local(p2.x, p2.y, p2.z),
            ) {
                painter.add(egui::Shape::convex_polygon(
                    vec![s0, s1, s2],
                    Color32::from_rgba_unmultiplied(239, 68, 68, 42),
                    Stroke::new(0.4, Color32::from_rgba_unmultiplied(239, 68, 68, 70)),
                ));
            }
        }

        // 2. Render iso-elevation contour lines
        let span_z = (tin.max_elevation - tin.min_elevation).max(1.0);
        let contours = tin.extract_contours(2.0, 10.0);
        for contour in contours {
            let t = ((contour.elevation_m - tin.min_elevation) / span_z).clamp(0.0, 1.0);
            let col = Color32::from_rgb(
                (56.0 + 185.0 * t) as u8,
                (189.0 - 95.0 * t) as u8,
                (148.0 - 110.0 * t) as u8,
            )
            .gamma_multiply(if contour.is_index_contour { 0.88 } else { 0.52 });
            let stroke = Stroke::new(if contour.is_index_contour { 1.4 } else { 0.7 }, col);
            for &(a, b) in contour.segments.iter().take(600) {
                if let (Some(sa), Some(sb)) = (
                    project_local(a[0], a[1], contour.elevation_m),
                    project_local(b[0], b[1], contour.elevation_m),
                ) {
                    painter.line_segment([sa, sb], stroke);
                }
            }
            if contour.is_index_contour {
                if let Some(&(a, _)) = contour.segments.first() {
                    if let Some(sa) = project_local(a[0], a[1], contour.elevation_m) {
                        painter.text(
                            sa,
                            Align2::LEFT_BOTTOM,
                            format!("{:.0}m", contour.elevation_m),
                            FontId::monospace(8.5),
                            theme.text_primary.gamma_multiply(0.85),
                        );
                    }
                }
            }
        }
    }

    /// Projects pure-Rust OSM Overpass & GeoJSON urban vector roads, rivers, cadastral parcels,
    /// and 3D context building footprints sharply onto Earth 3D (`zero-blur at any zoom`).
    pub fn paint_urban_vector_features(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        theme: &MacTheme,
        features: &[super::vector_layers::UrbanVectorFeature],
    ) {
        use super::vector_layers::UrbanVectorKind;
        if xform.zoom_level < 12.5 || features.is_empty() {
            return;
        }
        let road_scale = ((xform.zoom_level - 13.0) * 0.45).clamp(1.0, 3.2);
        for feat in features {
            if feat.points_lat_lon.len() < 2 {
                continue;
            }
            let mut base_pts = Vec::with_capacity(feat.points_lat_lon.len());
            for &(la, lo) in &feat.points_lat_lon {
                if let Some((sp, _, true)) = xform.project_lat_lon_elev(la, lo, 0.0) {
                    base_pts.push(sp);
                }
            }
            if base_pts.len() < 2 {
                continue;
            }
            if !base_pts.iter().any(|p| xform.rect.expand(40.0).contains(*p)) {
                continue;
            }
            match feat.kind {
                UrbanVectorKind::RoadMajor => {
                    painter.add(egui::Shape::line(
                        base_pts,
                        Stroke::new(2.2 * road_scale, Color32::from_rgba_unmultiplied(245, 158, 11, 200)),
                    ));
                }
                UrbanVectorKind::RoadMinor => {
                    painter.add(egui::Shape::line(
                        base_pts,
                        Stroke::new(1.1 * road_scale, Color32::from_rgba_unmultiplied(203, 213, 225, 150)),
                    ));
                }
                UrbanVectorKind::RiverWaterway => {
                    painter.add(egui::Shape::line(
                        base_pts,
                        Stroke::new(2.6 * road_scale, Color32::from_rgba_unmultiplied(56, 189, 248, 195)),
                    ));
                }
                UrbanVectorKind::ParcelBoundary => {
                    if feat.closed && base_pts.len() >= 3 {
                        painter.add(egui::Shape::closed_line(
                            base_pts,
                            Stroke::new(1.5, theme.accent_orange.gamma_multiply(0.92)),
                        ));
                    } else {
                        painter.add(egui::Shape::line(
                            base_pts,
                            Stroke::new(1.5, theme.accent_orange.gamma_multiply(0.92)),
                        ));
                    }
                }
                UrbanVectorKind::ContextBuilding => {
                    if base_pts.len() >= 3 {
                        let h = feat.height_m.max(4.0) as f64;
                        let mut roof_pts = Vec::with_capacity(feat.points_lat_lon.len());
                        for &(la, lo) in &feat.points_lat_lon {
                            if let Some((rp, _, true)) = xform.project_lat_lon_elev(la, lo, h) {
                                roof_pts.push(rp);
                            }
                        }
                        if roof_pts.len() == base_pts.len() {
                            painter.add(egui::Shape::convex_polygon(
                                roof_pts,
                                Color32::from_rgba_unmultiplied(100, 116, 139, 95),
                                Stroke::new(1.0, Color32::from_rgba_unmultiplied(148, 163, 184, 170)),
                            ));
                        } else {
                            painter.add(egui::Shape::closed_line(
                                base_pts,
                                Stroke::new(1.0, Color32::from_rgba_unmultiplied(148, 163, 184, 150)),
                            ));
                        }
                    }
                }
            }
        }
    }

    fn shade_color(&self, base: Color32, world_unit: [f64; 3], xform: &GlobeScreenTransform) -> Color32 {
        if !self.camera.day_night_lighting {
            return base;
        }
        let ndotl = xform.solar_dot(world_unit);
        // Smooth twilight terminator transition with subtle night-side ambient starlight
        let light = if ndotl >= 0.0 {
            0.62 + 0.38 * ndotl.sqrt()
        } else {
            (0.62 + ndotl * 0.55).clamp(0.18, 0.62)
        };
        Color32::from_rgb(
            ((base.r() as f64) * light).clamp(0.0, 255.0) as u8,
            ((base.g() as f64) * light).clamp(0.0, 255.0) as u8,
            ((base.b() as f64) * light).clamp(0.0, 255.0) as u8,
        )
    }

    fn paint_ocean_sphere_dome(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        _theme: &MacTheme,
        world_map: &SlippyMapState,
    ) {
        let pal = world_map.tile_provider.palette();
        let deep_ocean = pal.deep_ocean;
        let [ar, ag, ab] = pal.atmosphere_rgb;
        let atmo_col = Color32::from_rgb(ar, ag, ab);

        // Camera-centered horizon cap so limb is always circular even at polar latitudes
        let max_alpha = (1.0_f64 / (1.0 + xform.d_surf).max(1.0001)).acos().min(PI * 0.498);
        const RINGS: usize = 12;
        const WEDGES: usize = 40;

        // Orthonormal basis around cam_dir_world
        let up_ref = if xform.cam_dir_world[1].abs() < 0.92 {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let u_axis = normalize3(cross3(up_ref, xform.cam_dir_world));
        let v_axis = normalize3(cross3(xform.cam_dir_world, u_axis));

        let mut mesh = egui::epaint::Mesh::default();
        let center_w = xform.cam_dir_world;
        let Some((c_pos, _, _)) = xform.project_tangent_h(xform.world_to_tangent_h(center_w)) else {
            return;
        };
        mesh.vertices.push(egui::epaint::Vertex {
            pos: c_pos,
            uv: Pos2::ZERO,
            color: self.shade_color(deep_ocean, center_w, xform),
        });

        let mut ring_indices: Vec<Vec<u32>> = Vec::with_capacity(RINGS);
        for ri in 1..=RINGS {
            let alpha = max_alpha * (ri as f64 / RINGS as f64);
            let ca = alpha.cos();
            let sa = alpha.sin();
            let mut row = Vec::with_capacity(WEDGES);
            for wi in 0..WEDGES {
                let phi = 2.0 * PI * (wi as f64 / WEDGES as f64);
                let (sp, cp) = (phi.sin(), phi.cos());
                let w = normalize3([
                    xform.cam_dir_world[0] * ca + (u_axis[0] * cp + v_axis[0] * sp) * sa,
                    xform.cam_dir_world[1] * ca + (u_axis[1] * cp + v_axis[1] * sp) * sa,
                    xform.cam_dir_world[2] * ca + (u_axis[2] * cp + v_axis[2] * sp) * sa,
                ]);
                let p_h = xform.world_to_tangent_h(w);
                if let Some((sp_pos, _, _)) = xform.project_tangent_h(p_h) {
                    let v_idx = mesh.vertices.len() as u32;
                    let rim_tint = if ri == RINGS && self.show_atmosphere {
                        blend_colors(self.shade_color(deep_ocean, w, xform), atmo_col, 0.35)
                    } else {
                        self.shade_color(deep_ocean, w, xform)
                    };
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: sp_pos,
                        uv: Pos2::ZERO,
                        color: rim_tint,
                    });
                    row.push(v_idx);
                } else {
                    row.push(0);
                }
            }
            ring_indices.push(row);
        }

        // First ring fans around center vertex 0
        if let Some(first) = ring_indices.first() {
            for wi in 0..WEDGES {
                let wn = (wi + 1) % WEDGES;
                mesh.indices.extend_from_slice(&[0, first[wi], first[wn]]);
            }
        }
        for r in 0..(RINGS - 1) {
            let r0 = &ring_indices[r];
            let r1 = &ring_indices[r + 1];
            for wi in 0..WEDGES {
                let wn = (wi + 1) % WEDGES;
                mesh.indices.extend_from_slice(&[r0[wi], r1[wi], r1[wn]]);
                mesh.indices.extend_from_slice(&[r0[wi], r1[wn], r0[wn]]);
            }
        }
        painter.add(egui::Shape::mesh(mesh));
    }

    fn paint_spherical_continents(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        _theme: &MacTheme,
        world_map: &SlippyMapState,
    ) {
        let pal = world_map.tile_provider.palette();
        let draw_shelf = world_map.zoom_level < 12.5;
        let shelf_width = if world_map.zoom_level >= 4.0 { 3.4 } else { 2.2 };

        for land in WORLD_LANDMASSES {
            // Subdivide long continent edges every ~5 deg so arcs follow Earth curvature cleanly
            let mut dense_world: Vec<([f64; 3], f64, f64)> = Vec::new();
            for i in 0..land.len() {
                let a = land[i];
                let b = land[(i + 1) % land.len()];
                let dlat = (b.0 - a.0).abs();
                let dlon = (b.1 - a.1).abs();
                let steps = ((dlat.max(dlon) / 5.0).ceil() as usize).clamp(1, 16);
                for s in 0..steps {
                    let t = s as f64 / steps as f64;
                    let la = a.0 + (b.0 - a.0) * t;
                    let lo = a.1 + (b.1 - a.1) * t;
                    let ph = xform.world_to_tangent_h(lat_lon_to_unit_sphere(la, lo));
                    dense_world.push((ph, la, lo));
                }
            }
            if dense_world.len() < 3 {
                continue;
            }
            // Sutherland-Hodgman horizon clip in tangent frame: dot(P_h, C_h) >= 1.002
            let clipped_h = clip_poly_to_horizon(&dense_world, xform.cam_pos_h);
            if clipped_h.len() < 3 {
                continue;
            }
            let mut screen_pts = Vec::with_capacity(clipped_h.len());
            let mut avg_lat = 0.0_f64;
            let mut sum_h = [0.0_f64; 3];
            for (ph, la) in &clipped_h {
                if let Some((sp, _, _)) = xform.project_tangent_h(*ph) {
                    screen_pts.push(sp);
                    avg_lat += *la;
                    sum_h[0] += ph[0];
                    sum_h[1] += ph[1];
                    sum_h[2] += ph[2];
                }
            }
            if screen_pts.len() < 3 {
                continue;
            }
            avg_lat /= screen_pts.len() as f64;
            let n_w = normalize3(xform.tangent_h_to_world(sum_h));
            let abs_lat = avg_lat.abs();
            let raw_land = if abs_lat > 66.0 {
                blend_colors(pal.land_base, pal.land_polar, 0.72)
            } else if (18.0..=31.0).contains(&abs_lat) {
                blend_colors(pal.land_base, pal.land_arid, 0.55)
            } else {
                pal.land_base
            };
            let shaded_land = self.shade_color(raw_land, n_w, xform);
            if draw_shelf {
                painter.add(egui::epaint::PathShape::closed_line(
                    screen_pts.clone(),
                    Stroke::new(shelf_width, pal.shallow_shelf),
                ));
            }
            if let Some(mesh) = ear_clip_polygon(&screen_pts, shaded_land) {
                painter.add(egui::Shape::mesh(mesh));
            }
            let shaded_coast = self.shade_color(pal.coast_stroke, n_w, xform);
            let coast_w = if world_map.tile_provider == VectorEarthTheme::CoastalBlueprint {
                1.05
            } else {
                0.95
            };
            painter.add(egui::epaint::PathShape::closed_line(
                screen_pts,
                Stroke::new(coast_w, shaded_coast),
            ));
        }
    }

    fn paint_spherical_ridges_and_rivers(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        pal: &VectorEarthPalette,
        zoom_level: f32,
    ) {
        if zoom_level < 1.35 {
            return;
        }
        let project_chain = |chain: &[(f64, f64)], stroke: Stroke| {
            let mut seg = Vec::with_capacity(chain.len() * 2);
            for w in chain.windows(2) {
                let a = w[0];
                let b = w[1];
                let steps = (((b.0 - a.0).abs().max((b.1 - a.1).abs()) / 2.5).ceil() as usize).clamp(1, 6);
                for s in 0..=steps {
                    let t = s as f64 / steps as f64;
                    let la = a.0 + (b.0 - a.0) * t;
                    let lo = a.1 + (b.1 - a.1) * t;
                    if let Some((sp, _, front)) = xform.project_lat_lon_elev(la, lo, 0.0) {
                        if front {
                            seg.push(sp);
                            continue;
                        }
                    }
                    if seg.len() >= 2 {
                        painter.add(egui::Shape::line(std::mem::take(&mut seg), stroke));
                    } else {
                        seg.clear();
                    }
                }
            }
            if seg.len() >= 2 {
                painter.add(egui::Shape::line(seg, stroke));
            }
        };

        let ridge_w = if zoom_level >= 4.5 { 1.4 } else { 1.0 };
        let river_w = if zoom_level >= 4.5 { 1.6 } else { 1.1 };
        let ridge_stroke = Stroke::new(ridge_w, pal.ridge_stroke);
        let river_stroke = Stroke::new(river_w, pal.river_stroke);

        for ridge in WORLD_MOUNTAIN_RIDGES {
            project_chain(ridge, ridge_stroke);
        }
        for river in WORLD_MAJOR_RIVERS {
            project_chain(river, river_stroke);
        }
    }

    fn paint_spherical_graticule(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        pal: &VectorEarthPalette,
        zoom_level: f32,
    ) {
        let step_deg = if zoom_level < 3.5 {
            15.0
        } else if zoom_level < 7.0 {
            5.0
        } else {
            1.0
        };
        let stroke = Stroke::new(
            0.7,
            if zoom_level > 9.0 {
                pal.graticule_stroke.gamma_multiply(0.65)
            } else {
                pal.graticule_stroke
            },
        );
        let equator_stroke = Stroke::new(1.15, pal.equator_stroke);

        let mut lat = -75.0_f64;
        while lat <= 75.0 {
            let st = if lat.abs() < 1e-3 { equator_stroke } else { stroke };
            let mut seg = Vec::new();
            for step in 0..=72 {
                let lon = -180.0 + (step as f64) * 5.0;
                if let Some((sp, _, front)) = xform.project_lat_lon_elev(lat, lon, 0.0) {
                    if front {
                        seg.push(sp);
                        continue;
                    }
                }
                if seg.len() >= 2 {
                    painter.add(egui::Shape::line(std::mem::take(&mut seg), st));
                } else {
                    seg.clear();
                }
            }
            if seg.len() >= 2 {
                painter.add(egui::Shape::line(seg, st));
            }
            lat += step_deg;
        }

        let mut lon = -180.0_f64;
        while lon < 180.0 {
            let st = if lon.abs() < 1e-3 { equator_stroke } else { stroke };
            let mut seg = Vec::new();
            for step in 0..=36 {
                let la = -84.0 + (step as f64) * (168.0 / 36.0);
                if let Some((sp, _, front)) = xform.project_lat_lon_elev(la, lon, 0.0) {
                    if front {
                        seg.push(sp);
                        continue;
                    }
                }
                if seg.len() >= 2 {
                    painter.add(egui::Shape::line(std::mem::take(&mut seg), st));
                } else {
                    seg.clear();
                }
            }
            if seg.len() >= 2 {
                painter.add(egui::Shape::line(seg, st));
            }
            lon += step_deg;
        }
    }

    fn paint_terraformed_cells(&self, painter: &egui::Painter, xform: &GlobeScreenTransform) {
        for cell in self.cells.values() {
            let (lat0, lon0) = (
                cell.lat_idx as f64 * EARTH_CELL_DEG,
                cell.lon_idx as f64 * EARTH_CELL_DEG,
            );
            let lat1 = lat0 + EARTH_CELL_DEG;
            let lon1 = lon0 + EARTH_CELL_DEG;
            let (clat, clon) = cell.center_lat_lon();
            let base_el = estimate_baseline_elevation_m(clat, clon);
            let total_el = base_el + cell.elev_delta_m;
            // Exaggerate vertical mountain relief subtly at orbital zoom so sculpted peaks stand out clearly
            let vis_exag = if xform.zoom_level < 6.0 { 16.0 } else { 2.5 };
            let elev_draw = (cell.elev_delta_m.max(0.0) as f64) * vis_exag;

            let corners = [
                (lat0, lon0),
                (lat0, lon1),
                (lat1, lon1),
                (lat1, lon0),
            ];
            let mut poly = Vec::with_capacity(4);
            let mut all_front = true;
            for (la, lo) in corners {
                if let Some((sp, _, front)) = xform.project_lat_lon_elev(la, lo, elev_draw) {
                    if !front {
                        all_front = false;
                        break;
                    }
                    poly.push(sp);
                } else {
                    all_front = false;
                    break;
                }
            }
            if !all_front || poly.len() < 3 {
                continue;
            }

            let color = if !cell.protected_seawall && total_el < self.vitals.sea_level_m && cell.elev_delta_m < 0.0 {
                Color32::from_rgba_unmultiplied(14, 116, 200, 175)
            } else if cell.elev_delta_m > 2200.0 && cell.biome == EarthBiome::Natural {
                Color32::from_rgba_unmultiplied(235, 242, 250, 195) // Snow-capped uplifted peaks
            } else if cell.elev_delta_m > 300.0 && cell.biome == EarthBiome::Natural {
                Color32::from_rgba_unmultiplied(180, 120, 68, 180)
            } else {
                let c = cell.biome.color();
                Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 185)
            };

            if let Some(mesh) = ear_clip_polygon(&poly, color) {
                painter.add(egui::Shape::mesh(mesh));
            }
            if cell.protected_seawall {
                painter.add(egui::epaint::PathShape::closed_line(
                    poly,
                    Stroke::new(1.8, Color32::from_rgb(56, 189, 248)),
                ));
            }
        }

        // Flood alert rings on coastal benchmark deltas when inundated by sea level rise
        if self.vitals.sea_level_m > 0.25 {
            for &(name, lat, lon, base_el) in COASTAL_BENCHMARKS {
                let key = lat_lon_to_cell_key(lat, lon);
                let (delta, walled) = self
                    .cells
                    .get(&key)
                    .map_or((0.0, false), |c| (c.elev_delta_m, c.protected_seawall));
                let flooded = !walled && (base_el + delta) < self.vitals.sea_level_m;
                if let Some((sp, _, front)) = xform.project_lat_lon_elev(lat, lon, 0.0) {
                    if front && xform.rect.expand(20.0).contains(sp) {
                        let ring_col = if flooded {
                            Color32::from_rgb(239, 68, 68)
                        } else if walled {
                            Color32::from_rgb(56, 189, 248)
                        } else {
                            continue;
                        };
                        painter.circle_stroke(sp, 6.5, Stroke::new(1.8, ring_col));
                        if xform.zoom_level >= 3.2 {
                            painter.text(
                                sp + Vec2::new(8.0, -4.0),
                                Align2::LEFT_BOTTOM,
                                if flooded {
                                    format!("{name} (FLOOD)")
                                } else {
                                    format!("{name} (SEAWALL)")
                                },
                                FontId::proportional(9.5),
                                ring_col,
                            );
                        }
                    }
                }
            }
        }
    }

    fn paint_megastructures(&self, painter: &egui::Painter, xform: &GlobeScreenTransform) {
        let vis_exag = if xform.zoom_level < 6.0 { 18.0 } else { 2.0 };
        for mega in &self.megastructures {
            let color = mega.kind.accent_color();
            if mega.kind == MegastructureKind::ArcologyMegaTower {
                let Some((base, _, front)) =
                    xform.project_lat_lon_elev(mega.start_lat_lon.0, mega.start_lat_lon.1, 0.0)
                else {
                    continue;
                };
                if !front {
                    continue;
                }
                let tip_elev = mega.height_m * vis_exag;
                let tip = xform
                    .project_lat_lon_elev(mega.start_lat_lon.0, mega.start_lat_lon.1, tip_elev)
                    .map_or(base + Vec2::new(0.0, -24.0), |(p, _, _)| p);
                painter.line_segment([base, tip], Stroke::new(3.5, color));
                painter.circle_filled(tip, 4.0, Color32::WHITE);
                painter.circle_filled(base, 4.5, color);
                painter.text(
                    tip + Vec2::new(6.0, -4.0),
                    Align2::LEFT_CENTER,
                    &mega.name,
                    FontId::proportional(9.5),
                    Color32::WHITE,
                );
                continue;
            }

            let arc = great_circle_arc(mega.start_lat_lon, mega.end_lat_lon, 16);
            let mut pts = Vec::with_capacity(arc.len());
            for (idx, (la, lo)) in arc.iter().enumerate() {
                let t = idx as f64 / (arc.len().saturating_sub(1).max(1) as f64);
                let arch_profile = (t * PI).sin() * mega.height_m * vis_exag;
                if let Some((sp, _, front)) = xform.project_lat_lon_elev(*la, *lo, arch_profile) {
                    if front {
                        pts.push(sp);
                    }
                }
            }
            if pts.len() >= 2 {
                painter.add(egui::Shape::line(pts.clone(), Stroke::new(2.8, color)));
                if let (Some(&p0), Some(&p1)) = (pts.first(), pts.last()) {
                    painter.circle_filled(p0, 3.8, color);
                    painter.circle_filled(p1, 3.8, color);
                }
            }
        }
    }

    fn paint_hotspots_and_site_beacon(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        theme: &MacTheme,
        world_map: &SlippyMapState,
        ground_origin: (f64, f64),
        ground_anchored: bool,
    ) {
        for &(label, lat, lon) in GLOBE_HOTSPOTS {
            if let Some((sp, _, front)) = xform.project_lat_lon_elev(lat, lon, 0.0) {
                if front && xform.rect.contains(sp) {
                    painter.circle_filled(sp, 2.4, Color32::from_rgba_unmultiplied(248, 250, 252, 210));
                    painter.text(
                        sp + Vec2::new(5.0, -3.0),
                        Align2::LEFT_BOTTOM,
                        label,
                        FontId::proportional(9.5),
                        Color32::from_rgba_unmultiplied(226, 232, 240, 210),
                    );
                }
            }
        }
        if ground_anchored || world_map.active_pin.is_some() {
            let (plat, plon) = world_map.active_pin.unwrap_or(ground_origin);
            if let Some((base, _, front)) = xform.project_lat_lon_elev(plat, plon, 0.0) {
                if front && xform.rect.expand(18.0).contains(base) {
                    let tip = xform
                        .project_lat_lon_elev(plat, plon, if xform.zoom_level < 6.0 { 180_000.0 } else { 25.0 })
                        .map_or(base + Vec2::new(0.0, -18.0), |(p, _, _)| p);
                    painter.line_segment([base, tip], Stroke::new(2.0, theme.accent_blue));
                    painter.circle_filled(tip, 4.0, theme.accent_blue);
                    painter.circle_stroke(base, 7.0, Stroke::new(1.5, theme.accent_blue));
                }
            }
        }
    }

    fn paint_brush_footprint(
        &self,
        painter: &egui::Painter,
        xform: &GlobeScreenTransform,
        center_lat_lon: (f64, f64),
        color: Color32,
    ) {
        let rad_deg = (self.brush_radius_cells as f64 + 0.5) * EARTH_CELL_DEG * 0.85;
        let cos_lat = center_lat_lon.0.to_radians().cos().abs().max(0.2);
        let mut pts = Vec::with_capacity(24);
        for i in 0..24 {
            let a = 2.0 * PI * (i as f64 / 24.0);
            let la = (center_lat_lon.0 + rad_deg * a.sin()).clamp(-88.0, 88.0);
            let lo = center_lat_lon.1 + (rad_deg / cos_lat) * a.cos();
            if let Some((sp, _, front)) = xform.project_lat_lon_elev(la, lo, 0.0) {
                if front {
                    pts.push(sp);
                }
            }
        }
        if pts.len() >= 3 {
            painter.add(egui::epaint::PathShape::closed_line(
                pts,
                Stroke::new(1.8, color),
            ));
        }
    }
}

fn clip_poly_to_horizon(
    poly: &[([f64; 3], f64, f64)],
    cam_pos_h: [f64; 3],
) -> Vec<([f64; 3], f64)> {
    let mut out = Vec::new();
    let thresh = 1.002_f64;
    let eval = |p: [f64; 3]| -> f64 {
        p[0] * cam_pos_h[0] + p[1] * cam_pos_h[1] + p[2] * cam_pos_h[2] - thresh
    };
    let n = poly.len();
    for i in 0..n {
        let cur = poly[i];
        let nxt = poly[(i + 1) % n];
        let dc = eval(cur.0);
        let dn = eval(nxt.0);
        if dc >= 0.0 {
            out.push((cur.0, cur.1));
            if dn < 0.0 {
                let t = dc / (dc - dn + 1e-12);
                let inter = normalize3([
                    cur.0[0] + t * (nxt.0[0] - cur.0[0]),
                    cur.0[1] + t * (nxt.0[1] - cur.0[1]),
                    cur.0[2] + t * (nxt.0[2] - cur.0[2]),
                ]);
                out.push((inter, cur.1 + t * (nxt.1 - cur.1)));
            }
        } else if dn >= 0.0 {
            let t = dc / (dc - dn + 1e-12);
            let inter = normalize3([
                cur.0[0] + t * (nxt.0[0] - cur.0[0]),
                cur.0[1] + t * (nxt.0[1] - cur.0[1]),
                cur.0[2] + t * (nxt.0[2] - cur.0[2]),
            ]);
            out.push((inter, cur.1 + t * (nxt.1 - cur.1)));
        }
    }
    out
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn blend_colors(a: Color32, b: Color32, t: f32) -> Color32 {
    let k = t.clamp(0.0, 1.0);
    let mix = |ca: u8, cb: u8| -> u8 { ((ca as f32) * (1.0 - k) + (cb as f32) * k).round() as u8 };
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

fn paint_starfield(painter: &egui::Painter, rect: Rect, center: (f64, f64)) {
    let ox = (center.1 * 1.3) as f32;
    let oy = (center.0 * 0.9) as f32;
    for i in 0..72_u32 {
        let h1 = ((i.wrapping_mul(2654435761)) ^ 0x9E37_79B9) as f32 / u32::MAX as f32;
        let h2 = ((i.wrapping_mul(2246822519)) ^ 0x85EB_CA6B) as f32 / u32::MAX as f32;
        let px = rect.left() + ((h1 * rect.width() - ox).rem_euclid(rect.width().max(1.0)));
        let py = rect.top() + ((h2 * rect.height() + oy).rem_euclid(rect.height().max(1.0)));
        let alpha = 75 + ((i * 37) % 130) as u8;
        let radius = if i % 7 == 0 { 1.3 } else { 0.85 };
        painter.circle_filled(
            Pos2::new(px, py),
            radius,
            Color32::from_rgba_unmultiplied(226, 238, 255, alpha),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_globe_projection_and_ray_sphere_roundtrip() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 540.0));
        let cam = GlobeCamera {
            tilt_deg: 28.0,
            heading_deg: 22.0,
            ..GlobeCamera::default()
        };
        for &(center_lat, center_lon, zoom, probe_lat, probe_lon) in &[
            (-6.2088, 106.8456, 2.5_f32, -6.9175, 107.6191),
            (35.6762, 139.6503, 14.0, 35.6802, 139.6555),
            (40.7128, -74.0060, 17.5, 40.7131, -74.0054),
        ] {
            let xform = GlobeScreenTransform::from_view(rect, center_lat, center_lon, zoom, &cam);
            let (sp, depth, front) = xform
                .project_lat_lon_elev(probe_lat, probe_lon, 0.0)
                .expect("point should project cleanly");
            assert!(front && depth > 0.0);
            let (lat_back, lon_back, hit) = xform.screen_to_lat_lon(sp);
            assert!(hit);
            assert!(
                (lat_back - probe_lat).abs() < 1e-4 && (lon_back - probe_lon).abs() < 1e-4,
                "roundtrip mismatch at z={zoom}: ({lat_back}, {lon_back}) vs ({probe_lat}, {probe_lon})"
            );
        }
    }

    #[test]
    fn test_great_circle_arc_preserves_mid_latitude_and_unit_norm() {
        let arc = great_circle_arc((51.5, -0.1), (40.7, -74.0), 12);
        assert_eq!(arc.len(), 13);
        let mid = arc[6];
        // North Atlantic great circle bends north of linear lat midpoint (46.1 -> ~51.5+)
        assert!(mid.0 > 49.0, "great circle arc should bulge northward: got {:.2}", mid.0);
    }

    #[test]
    fn test_terraforming_brushes_seawall_flood_protection_and_undo_redo() {
        let mut earth = PlanetaryWorldState::default();
        earth.brush_radius_cells = 0;

        // 1. Raise Jakarta sea level +3.0m -> Jakarta Delta benchmark is flooded
        earth.vitals.sea_level_m = 3.0;
        earth.recompute_vitals();
        assert!(earth.vitals.flooded_coast_cells > 0);
        let baseline_flooded = earth.vitals.flooded_coast_cells;

        // 2. Defend Jakarta Delta with Seawall brush -> flooded count drops
        earth.apply_brush(-6.15, 106.85, EarthBrushAction::BuildSeawall, true);
        assert!(earth.vitals.seawall_count >= 1);
        assert!(earth.vitals.flooded_coast_cells < baseline_flooded);

        // 3. Plant forests -> atmospheric CO2 drops below 424 ppm
        earth.apply_brush(-1.0, 114.0, EarthBrushAction::PlantForest, true);
        assert!(earth.vitals.co2_ppm < 424.0);

        // 4. Add trans-strait megabridge across Sunda Strait
        let mid = earth.add_megastructure(
            MegastructureKind::TransStraitBridge,
            (-5.90, 105.88),
            (-6.02, 105.98),
        );
        assert_eq!(mid, 1);
        assert_eq!(earth.megastructures.len(), 1);

        // 5. Undo / Redo consistency
        assert!(earth.undo_earth());
        assert_eq!(earth.megastructures.len(), 0);
        assert!(earth.redo_earth());
        assert_eq!(earth.megastructures.len(), 1);
    }

    #[test]
    fn test_scenario_progress_and_simulation_tick() {
        let mut earth = PlanetaryWorldState::default();
        earth.brush_radius_cells = 1;
        let _focus = earth.load_scenario(EarthScenario::GreenNusantaraCorridor);
        assert_eq!(earth.scenario, EarthScenario::GreenNusantaraCorridor);

        // Plant 2 3x3 forest patches (9+ cells) + 1 solar corridor + 2 bridges -> reaches completion
        earth.apply_brush(-1.2, 112.0, EarthBrushAction::PlantForest, true);
        earth.apply_brush(-2.5, 116.0, EarthBrushAction::BuildSolarHub, true);
        earth.add_megastructure(
            MegastructureKind::TransStraitBridge,
            (-5.9, 105.8),
            (-6.1, 106.1),
        );
        earth.add_megastructure(
            MegastructureKind::TransStraitBridge,
            (-7.1, 112.7),
            (-7.0, 113.0),
        );
        let (prog, _txt, done) = earth.scenario_progress();
        assert!(done && prog >= 0.99);

        let y0 = earth.vitals.year;
        earth.tick_simulation_years(3);
        assert_eq!(earth.vitals.year, y0 + 3);
    }

    #[test]
    fn test_phase47_globe_drag_sensitivity_invariant_across_zoom() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
        let cam = GlobeCamera {
            tilt_deg: 20.0,
            heading_deg: 0.0,
            ..GlobeCamera::default()
        };

        // 1. Over +10 zoom levels (+1024x surface magnification), deg_per_pixel must shrink by ~1024x
        let xf_low = GlobeScreenTransform::from_view(rect, 0.0, 100.0, 2.0, &cam);
        let xf_mid = GlobeScreenTransform::from_view(rect, 0.0, 100.0, 12.0, &cam);
        let xf_site = GlobeScreenTransform::from_view(rect, 0.0, 100.0, 18.0, &cam);

        let ratio = xf_low.deg_per_pixel() / xf_mid.deg_per_pixel();
        assert!(
            (ratio - 1024.0).abs() < 2.0,
            "deg_per_pixel must scale strictly as 2^-zoom: expected ~1024, got {ratio:.3}"
        );
        assert!(
            xf_site.deg_per_pixel() < 5e-6 && xf_site.deg_per_pixel() > 1e-7,
            "site-scale deg_per_pixel out of calibrated bounds: {}",
            xf_site.deg_per_pixel()
        );

        // 2. A 24px equatorial pan at zoom 16.0 must displace screen projection by ~24px (1:1 feel)
        let pan_px = 24.0_f64;
        let xf_high = GlobeScreenTransform::from_view(rect, 0.0, 106.8456, 16.0, &cam);
        let target_lon = 106.8456 + pan_px * xf_high.deg_per_pixel();
        let (p_before, _, _) = xf_high.project_lat_lon_elev(0.0, target_lon, 0.0).unwrap();
        let dx_from_center = (p_before.x - rect.center().x).abs() as f64;
        assert!(
            (dx_from_center - pan_px).abs() < 0.75,
            "24px pan delta should equal ~24px on screen at z=16.0, got {dx_from_center:.2}px"
        );
    }
}
