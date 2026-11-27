//! urban_utilities.rs — Update `v0.8.7.7` (`7/10` -> `v0.8.8`): Subsurface Urban Utilities Network
//! (`WaterSupply`, `SanitaryStormSewer`, `UndergroundPower`) following `IfcRoad` splines.
//!
//! Routes 3D underground trunk mains along straight and curved `IfcRoad` centerlines at staggered
//! burial depths beneath the `IfcSite` TIN surface, automatically sleeves beneath `IfcBridge`
//! decks across water/canyons, connects lateral service drops to zoned `IfcSpatialZone` parcels and
//! the active `BimProject`, sizes municipal plant nodes (`WTP-01`, `SWD-01`, `SUB-01`), evaluates
//! city-scale Demand vs. Supply hydraulic/power balances, and composes a non-destructive IFCX overlay.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::gis::city_builder::IfcxCityScene;
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;

pub const NS_EG_URBAN_UTILITIES: &str = "eigengraph::gis::urban_utilities";

/// Municipal subsurface utility corridor discipline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrbanUtilityDiscipline {
    WaterSupply,
    SanitaryStormSewer,
    UndergroundPower,
}

impl UrbanUtilityDiscipline {
    pub fn label(&self) -> &'static str {
        match self {
            Self::WaterSupply => "Potable Water Main",
            Self::SanitaryStormSewer => "Sanitary & Storm Sewer",
            Self::UndergroundPower => "MV Underground Power",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::WaterSupply => "Water",
            Self::SanitaryStormSewer => "Sewer",
            Self::UndergroundPower => "Power",
        }
    }

    pub fn rgba(&self) -> [f32; 4] {
        match self {
            Self::WaterSupply => [0.12, 0.72, 0.98, 0.94],
            Self::SanitaryStormSewer => [0.18, 0.86, 0.56, 0.94],
            Self::UndergroundPower => [0.98, 0.76, 0.16, 0.94],
        }
    }

    pub fn discipline_code(&self) -> f32 {
        match self {
            Self::WaterSupply => 0.0,
            Self::SanitaryStormSewer => 0.5,
            Self::UndergroundPower => 1.0,
        }
    }
}

fn default_util_true() -> bool {
    true
}
fn default_water_cap_lps() -> f64 {
    65.0
}
fn default_sewer_cap_lps() -> f64 {
    120.0
}
fn default_substation_mva() -> f64 {
    15.0
}
fn default_rainfall_mm_hr() -> f64 {
    95.0
}
fn default_water_depth_m() -> f64 {
    1.35
}
fn default_sewer_depth_m() -> f64 {
    2.45
}
fn default_power_depth_m() -> f64 {
    0.95
}

/// Configuration & municipal plant capacity targets for subsurface utility evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrbanUtilitiesConfig {
    #[serde(default = "default_util_true")]
    pub show_water: bool,
    #[serde(default = "default_util_true")]
    pub show_sewer: bool,
    #[serde(default = "default_util_true")]
    pub show_power: bool,
    #[serde(default = "default_util_true")]
    pub xray_underground: bool,
    #[serde(default = "default_water_cap_lps")]
    pub water_supply_capacity_lps: f64,
    #[serde(default = "default_sewer_cap_lps")]
    pub sewer_plant_capacity_lps: f64,
    #[serde(default = "default_substation_mva")]
    pub substation_capacity_mva: f64,
    #[serde(default = "default_rainfall_mm_hr")]
    pub design_rainfall_mm_hr: f64,
    #[serde(default = "default_water_depth_m")]
    pub water_burial_depth_m: f64,
    #[serde(default = "default_sewer_depth_m")]
    pub sewer_burial_depth_m: f64,
    #[serde(default = "default_power_depth_m")]
    pub power_burial_depth_m: f64,
}

impl Default for UrbanUtilitiesConfig {
    fn default() -> Self {
        Self {
            show_water: true,
            show_sewer: true,
            show_power: true,
            xray_underground: true,
            water_supply_capacity_lps: default_water_cap_lps(),
            sewer_plant_capacity_lps: default_sewer_cap_lps(),
            substation_capacity_mva: default_substation_mva(),
            design_rainfall_mm_hr: default_rainfall_mm_hr(),
            water_burial_depth_m: default_water_depth_m(),
            sewer_burial_depth_m: default_sewer_depth_m(),
            power_burial_depth_m: default_power_depth_m(),
        }
    }
}

/// 3D ENU pipe, culvert, or underground MV cable conduit segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrbanUtilitySegment3D {
    pub tag: String,
    pub discipline: UrbanUtilityDiscipline,
    pub is_lateral: bool,
    pub is_bridge_sleeve: bool,
    pub start_enu_m: [f64; 3],
    pub end_enu_m: [f64; 3],
    pub diameter_mm: f64,
    pub burial_depth_m: f64,
    pub flow_or_load_label: String,
}

/// Municipal utility infrastructure plant or pump/substation facility node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrbanUtilityPlantNode3D {
    pub tag: String,
    pub name: String,
    pub discipline: UrbanUtilityDiscipline,
    pub center_enu_m: [f64; 3],
    pub size_enu_m: [f64; 3],
    pub capacity_label: String,
}

/// City-scale hydraulic & electrical Demand vs. Supply capacity balance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrbanUtilityCapacityBalance {
    pub water_avg_demand_lps: f64,
    pub water_peak_demand_lps: f64,
    pub water_fire_reserve_lps: f64,
    pub water_supply_capacity_lps: f64,
    pub water_utilization_pct: f64,
    pub water_trunk_dn_mm: u32,
    pub water_headloss_m_per_km: f64,
    pub water_adequate: bool,

    pub sanitary_flow_lps: f64,
    pub storm_runoff_lps: f64,
    pub total_drainage_demand_lps: f64,
    pub sewer_plant_capacity_lps: f64,
    pub sewer_utilization_pct: f64,
    pub sewer_trunk_dn_mm: u32,
    pub sewer_min_slope_pct: f64,
    pub sewer_adequate: bool,

    pub power_demand_mva: f64,
    pub substation_capacity_mva: f64,
    pub power_utilization_pct: f64,
    pub mv_cable_mm2: u32,
    pub voltage_drop_pct: f64,
    pub power_adequate: bool,
}

/// Compact IFCX serialization payload for non-destructive urban utilities overlay composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrbanUtilitiesIfcxOverlay {
    pub road_corridor_count: usize,
    pub connected_parcels: usize,
    pub total_water_length_m: f64,
    pub total_sewer_length_m: f64,
    pub total_power_length_m: f64,
    pub water_utilization_pct: f64,
    pub sewer_utilization_pct: f64,
    pub power_utilization_pct: f64,
    pub water_trunk_dn_mm: u32,
    pub sewer_trunk_dn_mm: u32,
    pub mv_cable_mm2: u32,
    pub plant_count: usize,
    pub estimated_infra_capex_usd: f64,
    pub all_utilities_adequate: bool,
}

/// Complete Subsurface Urban Utilities evaluation report & 3D network geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrbanUtilitiesReport {
    pub config: UrbanUtilitiesConfig,
    pub balance: UrbanUtilityCapacityBalance,
    pub segments_3d: Vec<UrbanUtilitySegment3D>,
    pub plants_3d: Vec<UrbanUtilityPlantNode3D>,
    pub road_corridor_count: usize,
    pub connected_parcels: usize,
    pub total_water_length_m: f64,
    pub total_sewer_length_m: f64,
    pub total_power_length_m: f64,
    pub estimated_infra_capex_usd: f64,
    pub all_utilities_adequate: bool,
    pub summary_verdict: String,
}

impl UrbanUtilitiesReport {
    /// Builds a compact IFCX attribute overlay for `IfcxStage::compose_with_urban_utilities_overlay`.
    pub fn to_ifcx_overlay(&self) -> UrbanUtilitiesIfcxOverlay {
        UrbanUtilitiesIfcxOverlay {
            road_corridor_count: self.road_corridor_count,
            connected_parcels: self.connected_parcels,
            total_water_length_m: self.total_water_length_m,
            total_sewer_length_m: self.total_sewer_length_m,
            total_power_length_m: self.total_power_length_m,
            water_utilization_pct: self.balance.water_utilization_pct,
            sewer_utilization_pct: self.balance.sewer_utilization_pct,
            power_utilization_pct: self.balance.power_utilization_pct,
            water_trunk_dn_mm: self.balance.water_trunk_dn_mm,
            sewer_trunk_dn_mm: self.balance.sewer_trunk_dn_mm,
            mv_cable_mm2: self.balance.mv_cable_mm2,
            plant_count: self.plants_3d.len(),
            estimated_infra_capex_usd: self.estimated_infra_capex_usd,
            all_utilities_adequate: self.all_utilities_adequate,
        }
    }
}

fn select_standard_water_dn_mm(peak_lps: f64) -> u32 {
    // Target design velocity ~1.25 m/s -> D_m = sqrt(4*Q / (pi * 1.25))
    let q_m3s = (peak_lps / 1000.0).max(0.01);
    let d_mm = ((4.0 * q_m3s / (std::f64::consts::PI * 1.25)).sqrt()) * 1000.0;
    for dn in [150_u32, 200, 250, 300, 400, 500, 600] {
        if d_mm <= dn as f64 {
            return dn;
        }
    }
    600
}

fn select_standard_sewer_dn_mm(flow_lps: f64) -> u32 {
    // Manning full-bore estimate at 0.6% grade, n=0.013 -> approx capacity Q = 0.24 * D^(8/3) * sqrt(S)
    let q_m3s = (flow_lps / 1000.0).max(0.02);
    for dn in [300_u32, 400, 500, 600, 800, 1000, 1200] {
        let d_m = dn as f64 / 1000.0;
        let q_cap = (1.0 / 0.013)
            * (std::f64::consts::PI * d_m * d_m * 0.25)
            * (d_m * 0.25).powf(2.0 / 3.0)
            * (0.006_f64).sqrt();
        if q_cap >= q_m3s * 1.15 {
            return dn;
        }
    }
    1200
}

fn ground_rel_z_at(tin: Option<&TinSurfaceMesh>, east_m: f64, north_m: f64) -> f64 {
    let Some(t) = tin else {
        return 0.0;
    };
    let ref_z = t.sample_elevation_and_slope(0.0, 0.0).0;
    let z = t.sample_elevation_and_slope(east_m as f32, north_m as f32).0;
    (z - ref_z) as f64
}

fn seg_len_3d(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Evaluates road-following 3D subsurface utility trunks, bridge utility sleeves, lateral
/// parcel/building connections, and city Demand vs. Supply balance (`wasm32`-safe).
pub fn evaluate_urban_utilities(
    city: &IfcxCityScene,
    bim_project: &BimProject,
    tin: Option<&TinSurfaceMesh>,
    sea_level_m: f32,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &UrbanUtilitiesConfig,
) -> UrbanUtilitiesReport {
    let mut cfg = config.clone();
    cfg.water_supply_capacity_lps = cfg.water_supply_capacity_lps.clamp(10.0, 1500.0);
    cfg.sewer_plant_capacity_lps = cfg.sewer_plant_capacity_lps.clamp(20.0, 4000.0);
    cfg.substation_capacity_mva = cfg.substation_capacity_mva.clamp(1.0, 250.0);
    cfg.design_rainfall_mm_hr = cfg.design_rainfall_mm_hr.clamp(20.0, 240.0);
    cfg.water_burial_depth_m = cfg.water_burial_depth_m.clamp(0.6, 4.5);
    cfg.sewer_burial_depth_m = cfg.sewer_burial_depth_m.clamp(1.2, 6.5);
    cfg.power_burial_depth_m = cfg.power_burial_depth_m.clamp(0.5, 3.0);

    // 1. Active BIM Project footprint & GFA
    let (bmin_m, bmax_m, bim_footprint_m2) = if let Some((mn, mx)) = bim_project.plan_extents_mm() {
        let mn_m = [mn[0] * 0.001, mn[1] * 0.001];
        let mx_m = [mx[0] * 0.001, mx[1] * 0.001];
        let area = ((mx_m[0] - mn_m[0]).abs() * (mx_m[1] - mn_m[1]).abs()).max(180.0);
        (mn_m, mx_m, area)
    } else {
        ([-9.0, -7.5], [9.0, 7.5], 270.0)
    };
    let stories = bim_project.levels.len().saturating_sub(1).max(2);
    let bim_gfa_m2 = (bim_footprint_m2 * stories as f64).max(800.0);
    let bim_occupants = (bim_gfa_m2 / 18.0).round();

    // 2. Urban District Demand Aggregation
    let metrics = city.district_metrics();
    let urban_res = metrics.total_residents as f64;
    let urban_jobs = metrics.total_jobs as f64;
    let avg_water_l_day = urban_res * 180.0 + urban_jobs * 60.0 + bim_occupants * 95.0;
    let water_avg_demand_lps = (avg_water_l_day / 86_400.0).max(1.2);
    let water_fire_reserve_lps = 15.0; // NFPA municipal hydrant reserve
    let water_peak_demand_lps = water_avg_demand_lps * 1.85 + water_fire_reserve_lps;
    let water_utilization_pct = (water_peak_demand_lps / cfg.water_supply_capacity_lps) * 100.0;
    let water_trunk_dn_mm = select_standard_water_dn_mm(water_peak_demand_lps);
    let q_w_m3s = water_peak_demand_lps / 1000.0;
    let d_w_m = water_trunk_dn_mm as f64 / 1000.0;
    // Hazen-Williams headloss gradient (C=130 HDPE/DI) in m/km
    let water_headloss_m_per_km =
        10.67 * (q_w_m3s / 130.0).powf(1.852) / d_w_m.powf(4.87) * 1000.0;
    let water_adequate = water_utilization_pct <= 95.0;

    // 3. Sanitary + Rational Stormwater Runoff (Q = C * i * A_ha / 0.36)
    let sanitary_flow_lps = water_avg_demand_lps * 1.35 * 0.85;
    let road_area_m2: f64 = city
        .roads
        .iter()
        .map(|r| r.length_m() * r.width_m)
        .sum();
    let parcel_area_m2: f64 = city
        .zones
        .iter()
        .map(|z| z.width_m * z.depth_m * 0.65)
        .sum();
    let impervious_area_m2 = (road_area_m2 + parcel_area_m2 + bim_footprint_m2).max(1500.0);
    let rainfall_boost = site.map_or(1.0, |s| if s.lat_deg.abs() < 15.0 { 1.12 } else { 1.0 });
    let eff_rain_mm_hr = cfg.design_rainfall_mm_hr * rainfall_boost;
    let runoff_coeff = 0.72;
    let storm_runoff_lps = (runoff_coeff * eff_rain_mm_hr * impervious_area_m2) / 3600.0;
    let total_drainage_demand_lps = sanitary_flow_lps + storm_runoff_lps;
    let sewer_utilization_pct = (total_drainage_demand_lps / cfg.sewer_plant_capacity_lps) * 100.0;
    let sewer_trunk_dn_mm = select_standard_sewer_dn_mm(total_drainage_demand_lps);
    let sewer_adequate = sewer_utilization_pct <= 95.0;

    // 4. Underground MV Power Grid (20kV Feeder Demand)
    let mut connected_load_kw = bim_gfa_m2 * 0.068;
    for b in &city.zones {
        if let Some(pb) = b.synthesize_procedural_building() {
            let density_kw_m2 = match pb.zone_kind {
                crate::core::gis::city_builder::IfcxZoneKind::Commercial => 0.075,
                crate::core::gis::city_builder::IfcxZoneKind::Industrial => 0.095,
                crate::core::gis::city_builder::IfcxZoneKind::Civic => 0.060,
                crate::core::gis::city_builder::IfcxZoneKind::Residential => 0.042,
                crate::core::gis::city_builder::IfcxZoneKind::Park => 0.005,
            };
            connected_load_kw += pb.gfa_m2 * density_kw_m2;
        }
    }
    let power_demand_kva = (connected_load_kw * 0.72) / 0.90;
    let power_demand_mva = power_demand_kva / 1000.0;
    let power_utilization_pct = (power_demand_mva / cfg.substation_capacity_mva) * 100.0;
    let mv_cable_mm2 = if power_demand_mva > 12.0 {
        300
    } else if power_demand_mva > 6.0 {
        240
    } else {
        150
    };
    let corridor_km = (city
        .roads
        .iter()
        .map(|r| r.length_m())
        .sum::<f64>()
        / 1000.0)
        .max(0.15);
    let voltage_drop_pct = ((power_demand_mva * corridor_km * 0.28).clamp(0.15, 6.5) * 100.0)
        .round()
        / 100.0;
    let power_adequate = power_utilization_pct <= 92.0 && voltage_drop_pct <= 5.0;

    // 5. Generate 3D Trunk Segments along `IfcRoad` splines (or fallback site perimeter corridor)
    let mut segments_3d = Vec::new();
    let mut trunk_anchor_pts: Vec<[f64; 3]> = Vec::new();

    let corridor_specs = [
        (
            UrbanUtilityDiscipline::WaterSupply,
            -0.24_f64,
            cfg.water_burial_depth_m,
            water_trunk_dn_mm as f64,
            format!("DN{water_trunk_dn_mm} ({water_peak_demand_lps:.1} L/s)"),
        ),
        (
            UrbanUtilityDiscipline::SanitaryStormSewer,
            0.0_f64,
            cfg.sewer_burial_depth_m,
            sewer_trunk_dn_mm as f64,
            format!("DN{sewer_trunk_dn_mm} ({total_drainage_demand_lps:.1} L/s)"),
        ),
        (
            UrbanUtilityDiscipline::UndergroundPower,
            0.24_f64,
            cfg.power_burial_depth_m,
            (mv_cable_mm2 as f64 * 0.95).clamp(160.0, 320.0),
            format!("20kV {mv_cable_mm2}mm² ({power_demand_mva:.2} MVA)"),
        ),
    ];

    if !city.roads.is_empty() {
        for road in &city.roads {
            let samples = road.sample_3d_profile(tin, sea_level_m);
            if samples.len() < 2 {
                continue;
            }
            for w in samples.windows(2) {
                let a = w[0];
                let b = w[1];
                let is_bridge_sleeve = a.is_elevated || b.is_elevated || a.over_water || b.over_water;
                trunk_anchor_pts.push([a.east_m, a.north_m, a.ground_z_rel_m as f64]);
                for &(disc, lat_frac, depth_m, diam_mm, ref flow_lbl) in &corridor_specs {
                    let lat_off = road.width_m * lat_frac;
                    let ax = a.east_m + a.normal[0] * lat_off;
                    let ay = a.north_m + a.normal[1] * lat_off;
                    let bx = b.east_m + b.normal[0] * lat_off;
                    let by = b.north_m + b.normal[1] * lat_off;
                    let az = if a.is_elevated || a.over_water {
                        a.deck_z_rel_m as f64 - 0.72
                    } else {
                        a.ground_z_rel_m as f64 - depth_m
                    };
                    let bz = if b.is_elevated || b.over_water {
                        b.deck_z_rel_m as f64 - 0.72
                    } else {
                        b.ground_z_rel_m as f64 - depth_m
                    };
                    segments_3d.push(UrbanUtilitySegment3D {
                        tag: format!("{}-{}", disc.short_label(), road.name),
                        discipline: disc,
                        is_lateral: false,
                        is_bridge_sleeve,
                        start_enu_m: [ax, ay, az],
                        end_enu_m: [bx, by, bz],
                        diameter_mm: diam_mm,
                        burial_depth_m: if is_bridge_sleeve { 0.72 } else { depth_m },
                        flow_or_load_label: flow_lbl.clone(),
                    });
                }
            }
            if let Some(last) = samples.last() {
                trunk_anchor_pts.push([last.east_m, last.north_m, last.ground_z_rel_m as f64]);
            }
        }
    } else {
        // Fallback site frontage utility loop when no roads have been authored yet
        let min_x = (bmin_m[0] - 12.0).min(-20.0);
        let max_x = (bmax_m[0] + 12.0).max(20.0);
        let min_y = (bmin_m[1] - 12.0).min(-18.0);
        let max_y = (bmax_m[1] + 12.0).max(18.0);
        let loop_pts = [
            [min_x, min_y],
            [max_x, min_y],
            [max_x, max_y],
            [min_x, max_y],
            [min_x, min_y],
        ];
        for seg in loop_pts.windows(2) {
            let p0 = seg[0];
            let p1 = seg[1];
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            let len = dx.hypot(dy).max(1e-4);
            let n = [-dy / len, dx / len];
            let gz0 = ground_rel_z_at(tin, p0[0], p0[1]);
            let gz1 = ground_rel_z_at(tin, p1[0], p1[1]);
            trunk_anchor_pts.push([p0[0], p0[1], gz0]);
            for &(disc, lat_frac, depth_m, diam_mm, ref flow_lbl) in &corridor_specs {
                let lat_off = 9.0 * lat_frac;
                segments_3d.push(UrbanUtilitySegment3D {
                    tag: format!("{}-SiteMain", disc.short_label()),
                    discipline: disc,
                    is_lateral: false,
                    is_bridge_sleeve: false,
                    start_enu_m: [p0[0] + n[0] * lat_off, p0[1] + n[1] * lat_off, gz0 - depth_m],
                    end_enu_m: [p1[0] + n[0] * lat_off, p1[1] + n[1] * lat_off, gz1 - depth_m],
                    diameter_mm: diam_mm,
                    burial_depth_m: depth_m,
                    flow_or_load_label: flow_lbl.clone(),
                });
            }
        }
    }

    // 6. Lateral service connections to origin BIM project & zoned parcels
    let mut connected_parcels = 0usize;
    let bim_cx = 0.5 * (bmin_m[0] + bmax_m[0]);
    let bim_cy = 0.5 * (bmin_m[1] + bmax_m[1]);
    let mut service_targets: Vec<(String, [f64; 2])> =
        vec![(bim_project.project_name.clone(), [bim_cx, bim_cy])];
    for z in &city.zones {
        if z.kind == crate::core::gis::city_builder::IfcxZoneKind::Park {
            continue;
        }
        connected_parcels += 1;
        service_targets.push((z.name.clone(), z.center_enu_m));
    }

    for (target_name, t_xy) in &service_targets {
        let Some(anchor) = trunk_anchor_pts.iter().min_by(|a, b| {
            let da = (a[0] - t_xy[0]).hypot(a[1] - t_xy[1]);
            let db = (b[0] - t_xy[0]).hypot(b[1] - t_xy[1]);
            da.total_cmp(&db)
        }) else {
            continue;
        };
        let dist = (anchor[0] - t_xy[0]).hypot(anchor[1] - t_xy[1]);
        if dist < 1.0 || dist > 160.0 {
            continue;
        }
        let target_gz = ground_rel_z_at(tin, t_xy[0], t_xy[1]);
        for &(disc, _lat, depth_m, diam_mm, _) in &corridor_specs {
            let lat_dn = (diam_mm * 0.48).clamp(90.0, 250.0);
            segments_3d.push(UrbanUtilitySegment3D {
                tag: format!("{}-Lat-{}", disc.short_label(), target_name),
                discipline: disc,
                is_lateral: true,
                is_bridge_sleeve: false,
                start_enu_m: [anchor[0], anchor[1], anchor[2] - depth_m],
                end_enu_m: [t_xy[0], t_xy[1], target_gz - depth_m * 0.82],
                diameter_mm: lat_dn,
                burial_depth_m: depth_m * 0.9,
                flow_or_load_label: format!("Service Drop DN{lat_dn:.0}"),
            });
        }
    }

    // 7. Municipal Utility Plant Nodes (`WTP-01`, `SWD-01`, `SUB-01`)
    let mut plants_3d = Vec::with_capacity(3);
    let base_pt = trunk_anchor_pts.first().copied().unwrap_or([-18.0, -14.0, 0.0]);
    let mid_pt = trunk_anchor_pts
        .get(trunk_anchor_pts.len() / 2)
        .copied()
        .unwrap_or([22.0, -14.0, 0.0]);
    let end_pt = trunk_anchor_pts.last().copied().unwrap_or([18.0, 18.0, 0.0]);
    let plant_specs = [
        (
            "WTP-01",
            "District Water Booster Station",
            UrbanUtilityDiscipline::WaterSupply,
            [base_pt[0] - 8.5, base_pt[1] - 6.5],
            [6.5, 5.2, 4.4],
            format!("{:.1} / {:.1} L/s", water_peak_demand_lps, cfg.water_supply_capacity_lps),
        ),
        (
            "SWD-01",
            "Stormwater Detention & Lift Vault",
            UrbanUtilityDiscipline::SanitaryStormSewer,
            [mid_pt[0] + 8.5, mid_pt[1] - 6.5],
            [7.5, 5.8, 3.8],
            format!("{:.1} / {:.1} L/s", total_drainage_demand_lps, cfg.sewer_plant_capacity_lps),
        ),
        (
            "SUB-01",
            "20kV GIS District Substation",
            UrbanUtilityDiscipline::UndergroundPower,
            [end_pt[0] + 8.5, end_pt[1] + 6.5],
            [6.8, 5.5, 5.0],
            format!("{power_demand_mva:.2} / {:.1} MVA", cfg.substation_capacity_mva),
        ),
    ];
    for (tag, name, disc, xy, size, cap_lbl) in plant_specs {
        let gz = ground_rel_z_at(tin, xy[0], xy[1]);
        plants_3d.push(UrbanUtilityPlantNode3D {
            tag: tag.to_string(),
            name: name.to_string(),
            discipline: disc,
            center_enu_m: [xy[0], xy[1], gz + size[2] * 0.5 - 0.6],
            size_enu_m: size,
            capacity_label: cap_lbl,
        });
    }

    // 8. Total network lengths & Civil CAPEX estimation
    let mut total_water_length_m = 0.0;
    let mut total_sewer_length_m = 0.0;
    let mut total_power_length_m = 0.0;
    for s in &segments_3d {
        let l = seg_len_3d(s.start_enu_m, s.end_enu_m);
        match s.discipline {
            UrbanUtilityDiscipline::WaterSupply => total_water_length_m += l,
            UrbanUtilityDiscipline::SanitaryStormSewer => total_sewer_length_m += l,
            UrbanUtilityDiscipline::UndergroundPower => total_power_length_m += l,
        }
    }
    let estimated_infra_capex_usd = total_water_length_m * (185.0 + water_trunk_dn_mm as f64 * 0.65)
        + total_sewer_length_m * (240.0 + sewer_trunk_dn_mm as f64 * 0.72)
        + total_power_length_m * (210.0 + mv_cable_mm2 as f64 * 0.90)
        + 420_000.0;

    let all_utilities_adequate = water_adequate && sewer_adequate && power_adequate;
    let summary_verdict = format!(
        "Urban Utilities ({} corridors, {} parcels): Water DN{water_trunk_dn_mm} {:.0}% | Sewer DN{sewer_trunk_dn_mm} {:.0}% | Power {mv_cable_mm2}mm² {:.0}% [{}]",
        city.roads.len(),
        connected_parcels,
        water_utilization_pct,
        sewer_utilization_pct,
        power_utilization_pct,
        if all_utilities_adequate { "BALANCED" } else { "CAPACITY OVERLOAD" }
    );

    UrbanUtilitiesReport {
        config: cfg,
        balance: UrbanUtilityCapacityBalance {
            water_avg_demand_lps,
            water_peak_demand_lps,
            water_fire_reserve_lps,
            water_supply_capacity_lps: config.water_supply_capacity_lps.clamp(10.0, 1500.0),
            water_utilization_pct,
            water_trunk_dn_mm,
            water_headloss_m_per_km,
            water_adequate,
            sanitary_flow_lps,
            storm_runoff_lps,
            total_drainage_demand_lps,
            sewer_plant_capacity_lps: config.sewer_plant_capacity_lps.clamp(20.0, 4000.0),
            sewer_utilization_pct,
            sewer_trunk_dn_mm,
            sewer_min_slope_pct: 0.60,
            sewer_adequate,
            power_demand_mva,
            substation_capacity_mva: config.substation_capacity_mva.clamp(1.0, 250.0),
            power_utilization_pct,
            mv_cable_mm2,
            voltage_drop_pct,
            power_adequate,
        },
        segments_3d,
        plants_3d,
        road_corridor_count: city.roads.len(),
        connected_parcels,
        total_water_length_m,
        total_sewer_length_m,
        total_power_length_m,
        estimated_infra_capex_usd,
        all_utilities_adequate,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::gis::city_builder::{IfcxRoadKind, IfcxZoneKind};
    use crate::core::gis::tin_surface::DemTerrainPreset;
    use crate::core::ifcx::{ns, IfcxDocument, IfcxStage};

    #[test]
    fn test_v0877_urban_utilities_road_splines_capacity_and_ifcx_overlay() {
        let mut city = IfcxCityScene::default();
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::CoastalIslandMetropolis,
            400.0,
            400.0,
            24,
            24,
            1.0,
        );
        let _ = city.add_curved_road_with_auto_bridge(
            [-70.0, -25.0],
            Some([0.0, -10.0]),
            [70.0, 15.0],
            IfcxRoadKind::Arterial,
            false,
            0.0,
            Some(&tin),
            0.0,
        );
        let _ = city.add_zone([-35.0, 12.0], 28.0, 24.0, 0.0, IfcxZoneKind::Commercial);
        let _ = city.add_zone([25.0, -24.0], 24.0, 22.0, 0.0, IfcxZoneKind::Residential);

        let proj = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let cfg = UrbanUtilitiesConfig::default();

        let rep = evaluate_urban_utilities(&city, &proj, Some(&tin), 0.0, Some(&site), &cfg);
        assert_eq!(rep.road_corridor_count, 1);
        assert_eq!(rep.connected_parcels, 2);
        assert!(rep.segments_3d.len() >= 15);
        assert!(rep.total_water_length_m > 100.0);
        assert!(rep.total_sewer_length_m > 100.0);
        assert!(rep.total_power_length_m > 100.0);
        assert!(rep.segments_3d.iter().any(|s| s.is_lateral));
        assert_eq!(rep.plants_3d.len(), 3);
        assert!(rep.balance.water_trunk_dn_mm >= 150);
        assert!(rep.balance.sewer_trunk_dn_mm >= 300);

        // Verify overload triggers when supply capacity is severely throttled
        let mut tight_cfg = cfg.clone();
        tight_cfg.water_supply_capacity_lps = 10.0;
        tight_cfg.sewer_plant_capacity_lps = 20.0;
        tight_cfg.substation_capacity_mva = 1.0;
        let tight_rep =
            evaluate_urban_utilities(&city, &proj, Some(&tin), 0.0, Some(&site), &tight_cfg);
        assert!(!tight_rep.all_utilities_adequate);

        // Verify IFCX overlay composition
        let base_doc = IfcxDocument::from_bim_project(&proj, None, 0.0, None, None, None);
        let composed =
            IfcxStage::compose_with_urban_utilities_overlay(&base_doc, &rep.to_ifcx_overlay());
        let util_node = composed
            .data
            .iter()
            .find(|n| n.path == "/earth/city/urban_utilities")
            .expect("urban utilities IFCX node");
        assert!(util_node.attributes.contains_key(ns::EG_URBAN_UTILITIES));
    }
}
