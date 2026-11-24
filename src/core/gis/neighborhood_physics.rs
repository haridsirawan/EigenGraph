//! neighborhood_physics.rs — Multi-Building Inter-Structure Coupled Hazard Engine (Phase 59 / v0.5.9).
//!
//! Evaluates neighborhood interactions directly across `WorldSpatialIndex`:
//! - **Seismic Pounding Separation (SNI 1726-2019 / ASCE 7-22 Section 12.12.3)**: Measures true
//!   rotated polygon clear gap against all neighboring buildings inside 200 m.
//! - **Urban Wind Exposure Calibration & Venturi Channeling (ASCE 7-22 Sec. 26.7)**:
//!   Determines effective Exposure B/C/D and speed-up factor from local built massing density.
//! - **Collapse & Crane-Swing Proximity Envelope**: Flags adjacent low-EigenGate structures inside
//!   adjacent debris fall / heavy swing clearance radii.

use super::map_widget::SlippyMapState;
use super::world_index::{WorldBuildingProxy, WorldSpatialIndex};
use crate::core::wind::ExposureCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoundingSeverity {
    SafeClearance,
    TightBuffer,
    PoundingRisk,
}

impl PoundingSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::SafeClearance => "Safe Gap",
            Self::TightBuffer => "Tight Buffer",
            Self::PoundingRisk => "POUNDING RISK",
        }
    }
}

/// Pairwise seismic pounding separation assessment between active building and a nearby neighbor.
#[derive(Debug, Clone)]
pub struct NeighborPoundingContact {
    pub neighbor_rel_path: String,
    pub neighbor_name: String,
    pub neighbor_lat: f64,
    pub neighbor_lon: f64,
    pub neighbor_height_m: f64,
    pub neighbor_min_eigengate: f64,
    pub clear_gap_m: f64,
    pub required_gap_m: f64,
    pub severity: PoundingSeverity,
    pub closest_active_ll: (f64, f64),
    pub closest_neighbor_ll: (f64, f64),
    pub venturi_channel: bool,
    pub inside_collapse_fall_zone: bool,
}

/// Complete coupled multi-building neighborhood physics summary around the active structure.
#[derive(Debug, Clone)]
pub struct NeighborhoodAuditReport {
    pub neighbors_evaluated: usize,
    pub pounding_risk_count: usize,
    pub tight_buffer_count: usize,
    pub venturi_corridor_count: usize,
    pub cascade_overlap_count: usize,
    pub min_clear_gap_m: Option<f64>,
    pub recommended_exposure: ExposureCategory,
    pub venturi_speedup_factor: f64,
    pub contacts: Vec<NeighborPoundingContact>,
}

impl Default for NeighborhoodAuditReport {
    fn default() -> Self {
        Self {
            neighbors_evaluated: 0,
            pounding_risk_count: 0,
            tight_buffer_count: 0,
            venturi_corridor_count: 0,
            cascade_overlap_count: 0,
            min_clear_gap_m: None,
            recommended_exposure: ExposureCategory::ExposureC,
            venturi_speedup_factor: 1.0,
            contacts: Vec::new(),
        }
    }
}

/// Evaluates Seismic Pounding separation (SNI 1726 / ASCE 7), Urban Wind Exposure & Venturi
/// channeling, and Collapse Fallback Proximity for the active building against `world_index`.
pub fn evaluate_neighborhood_coupling(
    active_origin: (f64, f64),
    active_footprint_local_m: &[[f64; 2]],
    active_heading_deg: f64,
    active_height_m: f64,
    active_rel_path: Option<&str>,
    world_index: &WorldSpatialIndex,
) -> NeighborhoodAuditReport {
    let mut report = NeighborhoodAuditReport::default();
    if world_index.buildings.is_empty() {
        return report;
    }

    // Query nearby buildings within ~0.004 deg (~440 m) around active origin
    let nearby_indices = world_index.query_visible_indices(
        active_origin.0 - 0.004,
        active_origin.0 + 0.004,
        active_origin.1 - 0.004,
        active_origin.1 + 0.004,
        active_origin.0,
        active_origin.1,
        120,
    );

    let active_poly_m = to_local_rotated_polygon_m(
        active_origin,
        active_origin,
        active_footprint_local_m,
        active_heading_deg,
    );
    let mut dense_urban_neighbors = 0usize;
    let mut tall_neighbors = 0usize;

    for idx in nearby_indices {
        let b = &world_index.buildings[idx];
        if active_rel_path == Some(b.rel_path.as_str()) {
            continue;
        }
        let nb_poly_m = to_local_rotated_polygon_m(
            active_origin,
            (b.lat, b.lon),
            &b.footprint_local_m,
            b.heading_deg,
        );
        let (clear_gap_m, p_act, p_nb) = polygon_min_distance_m(&active_poly_m, &nb_poly_m);
        if clear_gap_m > 220.0 {
            continue;
        }
        report.neighbors_evaluated += 1;
        if clear_gap_m <= 160.0 {
            dense_urban_neighbors += 1;
            if b.total_height_m >= 18.0 {
                tall_neighbors += 1;
            }
        }

        // SNI 1726-2019 / ASCE 7-22 SRSS inelastic seismic separation:
        // delta_MT = sqrt(delta_M1^2 + delta_M2^2) with typical allowable inelastic drift ~1.5%
        let d1 = active_height_m.max(3.5) * 0.015;
        let d2 = b.total_height_m.max(3.5) * 0.015;
        let required_gap_m = (d1 * d1 + d2 * d2).sqrt();

        let severity = if clear_gap_m < required_gap_m {
            report.pounding_risk_count += 1;
            PoundingSeverity::PoundingRisk
        } else if clear_gap_m < (required_gap_m * 1.5).max(required_gap_m + 0.5) {
            report.tight_buffer_count += 1;
            PoundingSeverity::TightBuffer
        } else {
            PoundingSeverity::SafeClearance
        };

        let venturi_channel =
            clear_gap_m <= 42.0 && active_height_m.min(b.total_height_m) >= 16.0;
        if venturi_channel {
            report.venturi_corridor_count += 1;
        }

        let max_h = active_height_m.max(b.total_height_m);
        let inside_collapse_fall_zone =
            clear_gap_m <= (0.35 * max_h) && (b.min_eigengate < 0.72 || b.max_dcr >= 0.90);
        if inside_collapse_fall_zone {
            report.cascade_overlap_count += 1;
        }

        report.min_clear_gap_m = Some(
            report
                .min_clear_gap_m
                .map_or(clear_gap_m, |cur| cur.min(clear_gap_m)),
        );

        if clear_gap_m <= 95.0 || severity != PoundingSeverity::SafeClearance || venturi_channel {
            let closest_active_ll =
                SlippyMapState::world_mm_to_lat_lon(active_origin, [p_act[0] * 1000.0, p_act[1] * 1000.0]);
            let closest_neighbor_ll =
                SlippyMapState::world_mm_to_lat_lon(active_origin, [p_nb[0] * 1000.0, p_nb[1] * 1000.0]);
            report.contacts.push(NeighborPoundingContact {
                neighbor_rel_path: b.rel_path.clone(),
                neighbor_name: b.name.clone(),
                neighbor_lat: b.lat,
                neighbor_lon: b.lon,
                neighbor_height_m: b.total_height_m,
                neighbor_min_eigengate: b.min_eigengate,
                clear_gap_m,
                required_gap_m,
                severity,
                closest_active_ll,
                closest_neighbor_ll,
                venturi_channel,
                inside_collapse_fall_zone,
            });
        }
    }

    report.contacts.sort_by(|a, b| {
        a.clear_gap_m
            .partial_cmp(&b.clear_gap_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    report.contacts.truncate(12);

    report.recommended_exposure = if dense_urban_neighbors >= 4 || tall_neighbors >= 2 {
        ExposureCategory::ExposureB
    } else if report.neighbors_evaluated >= 1 {
        ExposureCategory::ExposureC
    } else {
        ExposureCategory::ExposureD
    };
    report.venturi_speedup_factor = if report.venturi_corridor_count > 0 {
        (1.0 + 0.08 * (report.venturi_corridor_count as f64).min(3.0)).clamp(1.0, 1.24)
    } else {
        1.0
    };

    report
}

fn to_local_rotated_polygon_m(
    ref_origin: (f64, f64),
    center: (f64, f64),
    local_hull_m: &[[f64; 2]],
    heading_deg: f64,
) -> Vec<[f64; 2]> {
    let hull = if local_hull_m.len() >= 3 {
        local_hull_m
    } else {
        &[[-9.0, -7.0], [9.0, -7.0], [9.0, 7.0], [-9.0, 7.0]]
    };
    let center_mm = SlippyMapState::lat_lon_to_world_mm(ref_origin, center.0, center.1);
    let cx = center_mm[0] * 0.001;
    let cy = center_mm[1] * 0.001;
    let rad = heading_deg.to_radians();
    let (sh, ch) = (rad.sin(), rad.cos());
    hull.iter()
        .map(|&[x, y]| [cx + (x * ch - y * sh), cy + (x * sh + y * ch)])
        .collect()
}

fn closest_point_on_seg_2d(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let abx = b[0] - a[0];
    let aby = b[1] - a[1];
    let l2 = abx * abx + aby * aby;
    if l2 < 1e-9 {
        return a;
    }
    let t = (((p[0] - a[0]) * abx + (p[1] - a[1]) * aby) / l2).clamp(0.0, 1.0);
    [a[0] + t * abx, a[1] + t * aby]
}

fn point_in_poly_2d(pt: [f64; 2], poly: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let mut j = poly.len().saturating_sub(1);
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        if ((pi[1] > pt[1]) != (pj[1] > pt[1]))
            && (pt[0] < (pj[0] - pi[0]) * (pt[1] - pi[1]) / ((pj[1] - pi[1]) + 1e-9) + pi[0])
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Computes minimum edge-to-edge Euclidean distance (m) between two closed 2D polygons (`0.0` if overlapping).
pub fn polygon_min_distance_m(a: &[[f64; 2]], b: &[[f64; 2]]) -> (f64, [f64; 2], [f64; 2]) {
    if a.is_empty() || b.is_empty() {
        return (f64::INFINITY, [0.0, 0.0], [0.0, 0.0]);
    }
    for &p in a {
        if point_in_poly_2d(p, b) {
            return (0.0, p, p);
        }
    }
    for &q in b {
        if point_in_poly_2d(q, a) {
            return (0.0, q, q);
        }
    }

    let mut min_d = f64::INFINITY;
    let mut best_a = a[0];
    let mut best_b = b[0];

    for i in 0..a.len() {
        let p = a[i];
        for j in 0..b.len() {
            let b0 = b[j];
            let b1 = b[(j + 1) % b.len()];
            let q = closest_point_on_seg_2d(p, b0, b1);
            let d = (p[0] - q[0]).hypot(p[1] - q[1]);
            if d < min_d {
                min_d = d;
                best_a = p;
                best_b = q;
            }
        }
    }
    for j in 0..b.len() {
        let q = b[j];
        for i in 0..a.len() {
            let a0 = a[i];
            let a1 = a[(i + 1) % a.len()];
            let p = closest_point_on_seg_2d(q, a0, a1);
            let d = (p[0] - q[0]).hypot(p[1] - q[1]);
            if d < min_d {
                min_d = d;
                best_a = p;
                best_b = q;
            }
        }
    }
    (min_d, best_a, best_b)
}

/// Helper to build local rectangular hull around centroid from a `BimProject` in meters.
pub fn bim_footprint_hull_m(bim: &crate::core::bim::BimProject) -> (Vec<[f64; 2]>, f64) {
    let proxy = WorldBuildingProxy::from_bim_project(
        "",
        "",
        0.0,
        0.0,
        0.0,
        bim,
        &[],
        super::world_index::GeoPlacementSource::UserPinnedOnEarth,
        String::new(),
    );
    (proxy.footprint_local_m, proxy.total_height_m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::gis::GeoPlacementSource;

    #[test]
    fn test_phase59_seismic_pounding_venturi_and_exposure_calibration() {
        let origin = (-6.2088, 106.8456);
        let mut idx = WorldSpatialIndex::default();

        // Neighbor 1 placed 25.0 m East center-to-center; with +/-10m + +/-12m half-widths => 3.0 m clear gap!
        // For 40m + 40m towers: delta_MT = sqrt(0.6^2 + 0.6^2) = 0.85m, but Wait: let's test tight 0.6m clear gap (pounding!)
        let tight_ll = SlippyMapState::world_mm_to_lat_lon(origin, [20_600.0, 0.0]);
        idx.upsert_building(
            WorldBuildingProxy {
                id: 101,
                rel_path: "Projects/TightNeighbor.ifc".to_string(),
                name: "TightNeighbor".to_string(),
                filename: "TightNeighbor.ifc".to_string(),
                lat: tight_ll.0,
                lon: tight_ll.1,
                elev_m: 12.0,
                heading_deg: 0.0,
                footprint_local_m: vec![[-10.0, -8.0], [10.0, -8.0], [10.0, 8.0], [-10.0, 8.0]],
                total_height_m: 40.0,
                story_count: 11,
                member_count: 180,
                max_dcr: 0.96,
                min_eigengate: 0.62,
                source: GeoPlacementSource::UserPinnedOnEarth,
                address_hint: "Jakarta".to_string(),
            },
            true,
        );

        let active_hull = vec![[-10.0, -8.0], [10.0, -8.0], [10.0, 8.0], [-10.0, 8.0]];
        let rep = evaluate_neighborhood_coupling(origin, &active_hull, 0.0, 40.0, None, &idx);

        assert_eq!(rep.neighbors_evaluated, 1);
        assert_eq!(rep.pounding_risk_count, 1);
        assert_eq!(rep.venturi_corridor_count, 1);
        assert!(rep.venturi_speedup_factor > 1.0);
        assert_eq!(rep.cascade_overlap_count, 1);
        assert!((rep.min_clear_gap_m.unwrap() - 0.6).abs() < 0.1);
        assert_eq!(rep.contacts[0].severity, PoundingSeverity::PoundingRisk);
    }
}
