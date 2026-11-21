//! district_hazard.rs — District-Scale Multi-Hazard Simulation Kernel.
//!
//! Pure-Rust analytical physics engine evaluating coupled district-scale hazards across IFCX Earth
//! sites, road/bridge infrastructure, and neighborhood buildings:
//! - **SNI 1726 / ASCE 7 Traveling Seismic Wavefield & Phase-Lag Pounding**
//! - **Venturi Urban Wind Channeling & Dynamic Pressure Streamlines**
//! - **3D Flood / Sea-Level Inundation vs. `IfcSite` TIN Contours & Ground Floor Freeboard**
//! - **Non-Destructive IFCX Overlay Export (`eigengraph::hazard::district`)**

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::city_builder::IfcxCityScene;
use super::map_widget::SlippyMapState;
use super::neighborhood_physics::{NeighborhoodAuditReport, PoundingSeverity};
use super::tin_surface::TinSurfaceMesh;
use super::world_index::WorldSpatialIndex;
use crate::core::bim::BimProject;
use crate::core::ifcx::IfcxNode;

pub const NS_EG_DISTRICT_HAZARD: &str = "eigengraph::hazard::district";

fn default_pga_g() -> f64 {
    0.35
}
fn default_wave_speed_ms() -> f64 {
    280.0
}
fn default_wave_azimuth_deg() -> f64 {
    35.0
}
fn default_wind_azimuth_deg() -> f64 {
    240.0
}
fn default_base_wind_ms() -> f64 {
    32.0
}
fn default_flood_water_level_m() -> f64 {
    1.5
}

/// Interactive district multi-hazard simulation configuration persisted inside workspaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistrictHazardConfig {
    #[serde(default)]
    pub seismic_wave_active: bool,
    #[serde(default = "default_pga_g")]
    pub seismic_pga_g: f64,
    #[serde(default = "default_wave_speed_ms")]
    pub wave_speed_ms: f64,
    #[serde(default = "default_wave_azimuth_deg")]
    pub wave_azimuth_deg: f64,
    #[serde(default)]
    pub pounding_overlay_active: bool,
    #[serde(default)]
    pub venturi_wind_active: bool,
    #[serde(default = "default_wind_azimuth_deg")]
    pub wind_azimuth_deg: f64,
    #[serde(default = "default_base_wind_ms")]
    pub base_wind_ms: f64,
    #[serde(default)]
    pub flood_plane_active: bool,
    #[serde(default = "default_flood_water_level_m")]
    pub flood_water_level_m: f64,
    #[serde(skip)]
    pub sim_time_sec: f64,
}

impl Default for DistrictHazardConfig {
    fn default() -> Self {
        Self {
            seismic_wave_active: false,
            seismic_pga_g: default_pga_g(),
            wave_speed_ms: default_wave_speed_ms(),
            wave_azimuth_deg: default_wave_azimuth_deg(),
            pounding_overlay_active: false,
            venturi_wind_active: false,
            wind_azimuth_deg: default_wind_azimuth_deg(),
            base_wind_ms: default_base_wind_ms(),
            flood_plane_active: false,
            flood_water_level_m: default_flood_water_level_m(),
            sim_time_sec: 0.0,
        }
    }
}

impl DistrictHazardConfig {
    pub fn any_active(&self) -> bool {
        self.seismic_wave_active
            || self.pounding_overlay_active
            || self.venturi_wind_active
            || self.flood_plane_active
    }

    pub fn is_animated(&self) -> bool {
        self.seismic_wave_active || self.venturi_wind_active || self.flood_plane_active
    }

    /// Deterministic hash used to avoid regenerating WebGL2 buffers unless hazard parameters or
    /// quantized animation frames advance.
    pub fn content_revision_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |acc: &mut u64, v: u64| {
            *acc ^= v;
            *acc = acc.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(&mut h, u64::from(self.seismic_wave_active));
        mix(&mut h, (self.seismic_pga_g * 1000.0).round() as i64 as u64);
        mix(&mut h, (self.wave_speed_ms * 10.0).round() as i64 as u64);
        mix(&mut h, (self.wave_azimuth_deg * 10.0).round() as i64 as u64);
        mix(&mut h, u64::from(self.pounding_overlay_active));
        mix(&mut h, u64::from(self.venturi_wind_active));
        mix(&mut h, (self.wind_azimuth_deg * 10.0).round() as i64 as u64);
        mix(&mut h, (self.base_wind_ms * 10.0).round() as i64 as u64);
        mix(&mut h, u64::from(self.flood_plane_active));
        mix(&mut h, (self.flood_water_level_m * 100.0).round() as i64 as u64);
        if self.is_animated() {
            // Quantize continuous wave/wind streamlines at 30 Hz
            let q = ((self.sim_time_sec.max(0.0)) * 30.0).round() as u64;
            mix(&mut h, q);
        }
        h
    }
}

/// 3D flood plane vs. `IfcSite` TIN terrain and urban asset inundation metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DistrictFloodAssessment {
    pub water_level_m: f64,
    pub inundated_triangles: usize,
    pub total_tin_triangles: usize,
    pub inundated_area_pct: f64,
    pub max_depth_m: f64,
    pub active_building_inundated: bool,
    pub active_freeboard_m: f64,
    pub flooded_neighbors: usize,
    pub flooded_city_assets: usize,
}

/// Single aerodynamic streamline segment used for GPU Venturi urban wind visualization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VenturiStreamlineSegment {
    pub start_enu_m: [f64; 3],
    pub end_enu_m: [f64; 3],
    pub local_speed_ms: f64,
    pub is_corridor_accelerated: bool,
}

/// Integrated district multi-hazard audit report covering seismic waves, pounding, Venturi wind, and 3D flood.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DistrictMultiHazardReport {
    pub pga_g: f64,
    pub spectral_sds_g: f64,
    pub wave_length_m: f64,
    pub phase_amplified_pounding_pairs: usize,
    pub pounding_risk_count: usize,
    pub effective_wind_ms: f64,
    pub wind_dynamic_pressure_pa: f64,
    pub venturi_speedup_factor: f64,
    pub venturi_corridors: usize,
    pub streamlines: Vec<VenturiStreamlineSegment>,
    pub flood: DistrictFloodAssessment,
    pub resilience_score: f64,
    pub triage_label: String,
}

/// Evaluates analytical traveling Rayleigh/shear seismic ground displacement `[dx, dy, dz]` (m)
/// at ENU coordinate `(x_m, y_m)` and simulation time `t_sec`.
pub fn evaluate_seismic_wave_displacement_m(
    x_m: f64,
    y_m: f64,
    t_sec: f64,
    cfg: &DistrictHazardConfig,
) -> [f32; 3] {
    if !cfg.seismic_wave_active || cfg.seismic_pga_g <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let period_s = 0.85_f64;
    let wave_speed = cfg.wave_speed_ms.clamp(80.0, 1200.0);
    let lambda = (wave_speed * period_s).max(40.0);
    let k = std::f64::consts::TAU / lambda;
    let omega = std::f64::consts::TAU / period_s;
    let az = cfg.wave_azimuth_deg.to_radians();
    let dir_x = az.sin();
    let dir_y = az.cos();
    let s = x_m * dir_x + y_m * dir_y;
    let phase = k * s - omega * t_sec;
    // Scale physical ground wave amplitude cleanly for visual + phase-lag mechanics
    let amp_xy = (cfg.seismic_pga_g * 0.22).clamp(0.01, 0.45);
    let amp_z = amp_xy * 1.45;
    let dx = (amp_xy * phase.sin() * dir_x) as f32;
    let dy = (amp_xy * phase.sin() * dir_y) as f32;
    let dz = (amp_z * phase.cos()) as f32;
    [dx, dy, dz]
}

/// Evaluates 3D water surface elevation against `IfcSite` TIN facets, active BIM ground floor,
/// neighborhood proxies, and IFCX City-Builder infrastructure.
pub fn evaluate_district_flood(
    tin: Option<&TinSurfaceMesh>,
    world_index: &WorldSpatialIndex,
    active_origin: (f64, f64),
    active_bim: &BimProject,
    city_scene: &IfcxCityScene,
    water_level_m: f64,
) -> DistrictFloodAssessment {
    let mut inundated_triangles = 0usize;
    let mut total_tin_triangles = 0usize;
    let mut max_depth_m = 0.0_f64;

    if let Some(mesh) = tin {
        total_tin_triangles = mesh.triangles.len();
        for tri in &mesh.triangles {
            let z0 = mesh.points.get(tri[0]).map(|p| f64::from(p.z)).unwrap_or(0.0);
            let z1 = mesh.points.get(tri[1]).map(|p| f64::from(p.z)).unwrap_or(0.0);
            let z2 = mesh.points.get(tri[2]).map(|p| f64::from(p.z)).unwrap_or(0.0);
            let avg_z = (z0 + z1 + z2) / 3.0;
            if avg_z < water_level_m {
                inundated_triangles += 1;
                max_depth_m = max_depth_m.max(water_level_m - avg_z);
            }
        }
    }

    let inundated_area_pct = if total_tin_triangles > 0 {
        (inundated_triangles as f64 / total_tin_triangles as f64) * 100.0
    } else if water_level_m > 0.0 {
        // Flat apron baseline at z = 0.0 m
        max_depth_m = max_depth_m.max(water_level_m);
        100.0
    } else {
        0.0
    };

    let base_z_m = if active_bim.levels.is_empty() {
        0.0
    } else {
        let min_lvl_z = active_bim
            .levels
            .iter()
            .map(|l| l.elevation_mm * 0.001)
            .fold(f64::INFINITY, f64::min);
        if min_lvl_z.is_finite() {
            min_lvl_z
        } else {
            0.0
        }
    };
    let active_freeboard_m = base_z_m - water_level_m;
    let active_building_inundated = active_freeboard_m < 0.0;

    let mut flooded_neighbors = 0usize;
    for nb in &world_index.buildings {
        let mm = SlippyMapState::lat_lon_to_world_mm(active_origin, nb.lat, nb.lon);
        let ex = (mm[0] * 0.001) as f32;
        let ny = (mm[1] * 0.001) as f32;
        if (ex.hypot(ny)) > 260.0 {
            continue;
        }
        let ground_z = tin
            .map(|t| f64::from(t.sample_elevation_and_slope(ex, ny).0))
            .unwrap_or(0.0);
        if ground_z < water_level_m {
            flooded_neighbors += 1;
        }
    }

    let mut flooded_city_assets = 0usize;
    for road in &city_scene.roads {
        if road.deck_elev_m < water_level_m {
            flooded_city_assets += 1;
        }
    }
    for zone in &city_scene.zones {
        let ground_z = tin
            .map(|t| {
                f64::from(
                    t.sample_elevation_and_slope(
                        zone.center_enu_m[0] as f32,
                        zone.center_enu_m[1] as f32,
                    )
                    .0,
                )
            })
            .unwrap_or(0.0);
        if ground_z < water_level_m {
            flooded_city_assets += 1;
        }
    }

    DistrictFloodAssessment {
        water_level_m,
        inundated_triangles,
        total_tin_triangles,
        inundated_area_pct,
        max_depth_m,
        active_building_inundated,
        active_freeboard_m,
        flooded_neighbors,
        flooded_city_assets,
    }
}

/// Evaluates the full coupled district multi-hazard state (SNI 1726 seismic wave + phase-lag
/// pounding, ASCE 7 Venturi channeling streamlines, 3D flood vs TIN contours, and resilience index).
#[allow(clippy::too_many_arguments)]
pub fn evaluate_district_multi_hazard(
    cfg: &DistrictHazardConfig,
    neighborhood: &NeighborhoodAuditReport,
    tin: Option<&TinSurfaceMesh>,
    world_index: &WorldSpatialIndex,
    active_origin: (f64, f64),
    active_bim: &BimProject,
    city_scene: &IfcxCityScene,
) -> DistrictMultiHazardReport {
    let pga_g = cfg.seismic_pga_g.clamp(0.0, 2.0);
    // SNI 1726-2019 / ASCE 7-22 short-period design spectral acceleration S_DS ~ 2/3 * Fa * S_S (S_S ~ 2.5 * PGA)
    let spectral_sds_g = (2.0 / 3.0) * 1.2 * (pga_g * 2.5);
    let wave_length_m = (cfg.wave_speed_ms.clamp(80.0, 1200.0) * 0.85).max(40.0);

    let mut phase_amplified_pounding_pairs = 0usize;
    for contact in &neighborhood.contacts {
        if contact.severity == PoundingSeverity::SafeClearance {
            continue;
        }
        let a_mm = SlippyMapState::lat_lon_to_world_mm(
            active_origin,
            contact.closest_active_ll.0,
            contact.closest_active_ll.1,
        );
        let b_mm = SlippyMapState::lat_lon_to_world_mm(
            active_origin,
            contact.closest_neighbor_ll.0,
            contact.closest_neighbor_ll.1,
        );
        let dx = (b_mm[0] - a_mm[0]) * 0.001;
        let dy = (b_mm[1] - a_mm[1]) * 0.001;
        let center_sep_m = dx.hypot(dy).max(contact.clear_gap_m);
        let d_phi = std::f64::consts::TAU * (center_sep_m / wave_length_m);
        let out_of_phase_gain = (0.5 * d_phi).sin().abs();
        if contact.severity == PoundingSeverity::PoundingRisk || out_of_phase_gain > 0.65 {
            phase_amplified_pounding_pairs += 1;
        }
    }

    let venturi_speedup_factor = neighborhood.venturi_speedup_factor.clamp(1.0, 1.5);
    let effective_wind_ms = cfg.base_wind_ms.max(0.0) * venturi_speedup_factor;
    // ASCE 7 velocity pressure q_z = 0.613 * V^2 (Pa, sea-level air density)
    let wind_dynamic_pressure_pa = 0.613 * effective_wind_ms * effective_wind_ms;

    // Synthesize 3D ENU streamlines for ambient urban wind & accelerated Venturi channels
    let mut streamlines = Vec::new();
    let az = cfg.wind_azimuth_deg.to_radians();
    let ux = az.sin();
    let uy = az.cos();
    let px = -uy;
    let py = ux;

    for lane in -3_i32..=3 {
        let offset = (lane as f64) * 24.0;
        let elev = 9.0 + ((lane.abs() as f64) * 2.5);
        streamlines.push(VenturiStreamlineSegment {
            start_enu_m: [px * offset - ux * 95.0, py * offset - uy * 95.0, elev],
            end_enu_m: [px * offset + ux * 95.0, py * offset + uy * 95.0, elev],
            local_speed_ms: cfg.base_wind_ms.max(1.0),
            is_corridor_accelerated: false,
        });
    }

    for contact in &neighborhood.contacts {
        if !contact.venturi_channel {
            continue;
        }
        let a_mm = SlippyMapState::lat_lon_to_world_mm(
            active_origin,
            contact.closest_active_ll.0,
            contact.closest_active_ll.1,
        );
        let b_mm = SlippyMapState::lat_lon_to_world_mm(
            active_origin,
            contact.closest_neighbor_ll.0,
            contact.closest_neighbor_ll.1,
        );
        let mid_x = 0.5 * (a_mm[0] + b_mm[0]) * 0.001;
        let mid_y = 0.5 * (a_mm[1] + b_mm[1]) * 0.001;
        let z_ch = contact.neighbor_height_m.clamp(6.0, 26.0) * 0.55;
        streamlines.push(VenturiStreamlineSegment {
            start_enu_m: [mid_x - ux * 42.0, mid_y - uy * 42.0, z_ch],
            end_enu_m: [mid_x + ux * 42.0, mid_y + uy * 42.0, z_ch],
            local_speed_ms: effective_wind_ms,
            is_corridor_accelerated: true,
        });
    }

    let flood = evaluate_district_flood(
        tin,
        world_index,
        active_origin,
        active_bim,
        city_scene,
        cfg.flood_water_level_m,
    );

    let mut score = 100.0_f64;
    if cfg.seismic_wave_active {
        score -= (pga_g * 22.0).min(24.0);
    }
    score -= (neighborhood.pounding_risk_count as f64) * 14.0;
    score -= (phase_amplified_pounding_pairs.saturating_sub(neighborhood.pounding_risk_count) as f64)
        * 7.0;
    if cfg.venturi_wind_active || neighborhood.venturi_corridor_count > 0 {
        score -= ((venturi_speedup_factor - 1.0) * 45.0).clamp(0.0, 14.0);
        if effective_wind_ms > 42.0 {
            score -= 8.0;
        }
    }
    if cfg.flood_plane_active {
        score -= (flood.inundated_area_pct * 0.25).clamp(0.0, 22.0);
        if flood.active_building_inundated {
            score -= 22.0;
        }
    }
    let resilience_score = score.clamp(0.0, 100.0);
    let triage_label = if resilience_score >= 75.0 {
        "Resilient".to_string()
    } else if resilience_score >= 45.0 {
        "Caution".to_string()
    } else {
        "Critical Multi-Hazard".to_string()
    };

    DistrictMultiHazardReport {
        pga_g,
        spectral_sds_g,
        wave_length_m,
        phase_amplified_pounding_pairs,
        pounding_risk_count: neighborhood.pounding_risk_count,
        effective_wind_ms,
        wind_dynamic_pressure_pa,
        venturi_speedup_factor,
        venturi_corridors: neighborhood.venturi_corridor_count,
        streamlines,
        flood,
        resilience_score,
        triage_label,
    }
}

/// Emits a non-destructive IFCX overlay node representing district multi-hazard metrics.
pub fn to_ifcx_hazard_node(site_path: &str, report: &DistrictMultiHazardReport) -> IfcxNode {
    let mut node = IfcxNode::new(format!("{site_path}/hazard/district_multi_hazard"))
        .with_class("IfcSpatialZone")
        .with_prop(
            "District Multi-Hazard Audit",
            "Multi-Hazard Envelope",
            &format!(
                "Resilience {:.0}/100 ({}) | S_DS {:.2}g | Wind {:.1} m/s | Flood {:.0}%",
                report.resilience_score,
                report.triage_label,
                report.spectral_sds_g,
                report.effective_wind_ms,
                report.flood.inundated_area_pct
            ),
        );
    node.attributes.insert(
        NS_EG_DISTRICT_HAZARD.to_string(),
        serde_json::to_value(report).unwrap_or_else(|_| json!({})),
    );
    node
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimLevel;
    use crate::core::gis::neighborhood_physics::NeighborPoundingContact;
    use crate::core::gis::tin_surface::DemTerrainPreset;

    #[test]
    fn phase68_district_seismic_wave_venturi_wind_flood_and_ifcx_node() {
        let mut cfg = DistrictHazardConfig {
            seismic_wave_active: true,
            seismic_pga_g: 0.45,
            wave_speed_ms: 240.0,
            pounding_overlay_active: true,
            venturi_wind_active: true,
            base_wind_ms: 36.0,
            flood_plane_active: true,
            flood_water_level_m: 12.0,
            sim_time_sec: 0.25,
            ..Default::default()
        };
        let d0 = evaluate_seismic_wave_displacement_m(20.0, -10.0, 0.25, &cfg);
        let d1 = evaluate_seismic_wave_displacement_m(20.0, -10.0, 0.45, &cfg);
        assert!(d0.iter().all(|v| v.is_finite()));
        assert!((d0[2] - d1[2]).abs() > 1e-4, "Wave should propagate in time");
        let h0 = cfg.content_revision_hash();
        cfg.sim_time_sec = 0.55;
        assert_ne!(h0, cfg.content_revision_hash());

        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::CoastalAlluvialBasin,
            300.0,
            300.0,
            16,
            16,
            1.0,
        );
        let mut bim = BimProject::empty();
        bim.levels.push(BimLevel {
            id: 1,
            name: "L0".to_string(),
            elevation_mm: 0.0,
        });

        let nb = NeighborhoodAuditReport {
            neighbors_evaluated: 2,
            pounding_risk_count: 1,
            tight_buffer_count: 1,
            venturi_corridor_count: 1,
            venturi_speedup_factor: 1.08,
            contacts: vec![NeighborPoundingContact {
                neighbor_rel_path: "neighbor.ifcx".to_string(),
                neighbor_name: "Tower B".to_string(),
                neighbor_lat: -6.2089,
                neighbor_lon: 106.8458,
                neighbor_height_m: 28.0,
                neighbor_min_ari: 0.62,
                clear_gap_m: 0.25,
                required_gap_m: 0.60,
                severity: PoundingSeverity::PoundingRisk,
                closest_active_ll: (-6.2088, 106.8456),
                closest_neighbor_ll: (-6.2089, 106.8458),
                venturi_channel: true,
                inside_collapse_fall_zone: true,
            }],
            ..Default::default()
        };

        let report = evaluate_district_multi_hazard(
            &cfg,
            &nb,
            Some(&tin),
            &WorldSpatialIndex::default(),
            (-6.2088, 106.8456),
            &bim,
            &IfcxCityScene::default(),
        );
        assert!(report.spectral_sds_g > 0.5);
        assert!(report.effective_wind_ms > cfg.base_wind_ms);
        assert!(report.streamlines.iter().any(|s| s.is_corridor_accelerated));
        assert!(report.flood.inundated_triangles > 0);
        assert!(report.flood.active_building_inundated);
        assert!(report.resilience_score < 65.0);

        let node = to_ifcx_hazard_node("/earth/sites/demo", &report);
        assert!(node.attributes.contains_key(NS_EG_DISTRICT_HAZARD));
    }
}
