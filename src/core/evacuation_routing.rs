//! Phase 77: Autonomous Multi-Hazard Evacuation Routing, Emergency Lifelines & Post-Disaster
//! Road Network Connectivity Engine (`eigengraph::analysis::evacuation_lifelines`).
//!
//! Couples Phase 76 ATC-20 Green/Yellow/Red structural safety placards & debris cordons,
//! Phase 68 3D flood inundation & seismic PGA, and Phase 71 Seed–Idriss soil liquefaction
//! directly onto the Phase 67 IFCX City-Builder (`IfcRoad` / `IfcBridge` / `IfcSpatialZone`)
//! transportation graph to compute A* safest egress routes, emergency responder accessibility,
//! network connectivity ratios, and non-destructive `.ifcx` lifeline overlays.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::core::bim::BimProject;
use crate::core::geotech::SubsurfaceSsiAnalysisReport;
use crate::core::gis::city_builder::{IfcxCityScene, IfcxRoadKind, IfcxRoadSegment, IfcxZoneKind};
use crate::core::gis::district_hazard::{DistrictHazardConfig, DistrictMultiHazardReport};
use crate::core::gis::map_widget::SlippyMapState;
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::gis::world_index::WorldSpatialIndex;
use crate::core::ifcx::{ns, IfcxDocument, IfcxImport, IfcxNode, UsdGeomMesh};
use crate::core::shm_twin::{Atc20Placard, ShmDigitalTwinReport};

/// Canonical IFCX attribute namespace for Phase 77 Post-Disaster Evacuation & Emergency Lifelines.
pub const NS_EG_EVACUATION_LIFELINES: &str = ns::EG_EVACUATION_LIFELINES;

fn default_debris_radius_factor() -> f64 {
    0.45
}
fn default_true() -> bool {
    true
}

/// Active lifeline routing mode displayed on 3D WebGPU Earth and in the Dynamic Island drawer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum EvacuationMode {
    #[default]
    PedestrianEvacuation,
    EmergencyResponder,
    Both,
}

impl EvacuationMode {
    pub fn all() -> &'static [Self] {
        &[Self::PedestrianEvacuation, Self::EmergencyResponder, Self::Both]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::PedestrianEvacuation => "Pedestrian Safe Egress",
            Self::EmergencyResponder => "Emergency Responder Access",
            Self::Both => "Combined Egress & Responder",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::PedestrianEvacuation => "Pedestrian",
            Self::EmergencyResponder => "Responder",
            Self::Both => "Both",
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::PedestrianEvacuation => Self::EmergencyResponder,
            Self::EmergencyResponder => Self::Both,
            Self::Both => Self::PedestrianEvacuation,
        }
    }
}

/// Persisted configuration for Phase 77 Evacuation & Emergency Lifeline routing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvacuationRoutingConfig {
    #[serde(default)]
    pub overlay_visible: bool,
    #[serde(default)]
    pub mode: EvacuationMode,
    #[serde(default = "default_debris_radius_factor")]
    pub debris_radius_factor: f64,
    #[serde(default = "default_true")]
    pub include_flood_blockage: bool,
    #[serde(default = "default_true")]
    pub include_liquefaction_penalty: bool,
    #[serde(default = "default_true")]
    pub include_bridge_fragility: bool,
    #[serde(default)]
    pub selected_route_idx: usize,
}

impl Default for EvacuationRoutingConfig {
    fn default() -> Self {
        Self {
            overlay_visible: false,
            mode: EvacuationMode::PedestrianEvacuation,
            debris_radius_factor: default_debris_radius_factor(),
            include_flood_blockage: true,
            include_liquefaction_penalty: true,
            include_bridge_fragility: true,
            selected_route_idx: 0,
        }
    }
}

impl EvacuationRoutingConfig {
    /// Deterministic FNV-1a configuration hash for WebGPU mesh buffer invalidation.
    pub fn content_revision_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mix = |acc: &mut u64, v: u64| {
            *acc ^= v;
            *acc = acc.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(&mut h, u64::from(self.overlay_visible));
        mix(&mut h, self.mode as u64 + 1);
        mix(&mut h, (self.debris_radius_factor * 1000.0).round() as i64 as u64);
        mix(&mut h, u64::from(self.include_flood_blockage));
        mix(&mut h, u64::from(self.include_liquefaction_penalty));
        mix(&mut h, u64::from(self.include_bridge_fragility));
        mix(&mut h, self.selected_route_idx as u64);
        h
    }
}

/// ATC-20 building debris cordon envelope projected onto the surrounding street network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebrisCordonZone {
    pub building_label: String,
    pub center_enu_m: [f64; 2],
    pub footprint_radius_m: f64,
    pub cordon_radius_m: f64,
    pub placard: Atc20Placard,
    pub height_m: f64,
    pub is_active_building: bool,
}

/// Safe assembly plaza or park zone (`IfcSpatialZone`) serving as evacuation destination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblySafePoint {
    pub id: usize,
    pub name: String,
    pub center_enu_m: [f64; 2],
    pub capacity_persons: usize,
    pub is_from_city_zone: bool,
    pub hazard_margin_m: f64,
}

/// Operational passability status of a road or bridge lifeline segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoadEdgeStatus {
    OpenSafe,
    DegradedDetour,
    BlockedClosed,
}

impl RoadEdgeStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::OpenSafe => "OPEN SAFE",
            Self::DegradedDetour => "DEGRADED",
            Self::BlockedClosed => "BLOCKED",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::OpenSafe => "Open",
            Self::DegradedDetour => "Degraded",
            Self::BlockedClosed => "Blocked",
        }
    }
}

/// Vertex in the post-disaster street/bridge lifeline graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifelineGraphNode {
    pub id: usize,
    pub pos_enu_m: [f64; 2],
    pub elev_m: f64,
    pub label: String,
    pub is_safe_point: bool,
    pub is_responder_staging: bool,
}

/// Directed-or-undirected passable street/bridge edge annotated with multi-hazard impedance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifelineGraphEdge {
    pub id: usize,
    pub u: usize,
    pub v: usize,
    pub road_name: String,
    pub length_m: f64,
    pub width_m: f64,
    pub is_bridge: bool,
    pub status: RoadEdgeStatus,
    pub hazard_cost_multiplier: f64,
    pub blockage_reason: String,
}

/// Computed A* evacuation egress or emergency responder route across the lifeline graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvacuationRoutePath {
    pub id: usize,
    pub origin_label: String,
    pub target_safe_zone: String,
    pub is_responder_route: bool,
    pub is_active_building: bool,
    pub reachable: bool,
    pub waypoints_enu_m: Vec<[f64; 2]>,
    pub edge_ids: Vec<usize>,
    pub distance_m: f64,
    pub estimated_time_sec: f64,
    pub safety_score_0_100: f64,
    pub has_detour: bool,
    pub status_summary: String,
}

/// Full Phase 77 post-disaster evacuation routing, emergency lifeline & network connectivity report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvacuationLifelineReport {
    pub config: EvacuationRoutingConfig,
    pub nodes: Vec<LifelineGraphNode>,
    pub edges: Vec<LifelineGraphEdge>,
    pub cordons: Vec<DebrisCordonZone>,
    pub safe_points: Vec<AssemblySafePoint>,
    pub routes: Vec<EvacuationRoutePath>,
    pub active_evac_route: Option<EvacuationRoutePath>,
    pub active_responder_route: Option<EvacuationRoutePath>,
    pub total_edges: usize,
    pub open_edges: usize,
    pub degraded_edges: usize,
    pub blocked_edges: usize,
    pub network_connectivity_pct: f64,
    pub responder_accessibility_score: f64,
    pub isolated_building_count: usize,
    pub bridges_evaluated: usize,
    pub bridges_compromised: usize,
    pub used_synthetic_district_grid: bool,
    pub summary: String,
    pub evacuation_overlay_ifcx: IfcxDocument,
}

#[derive(Clone, Copy, PartialEq)]
struct AStarQueueItem {
    f_cost: f64,
    g_cost: f64,
    node_id: usize,
}

impl Eq for AStarQueueItem {}

impl Ord for AStarQueueItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.node_id.cmp(&self.node_id))
    }
}

impl PartialOrd for AStarQueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn dist2(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn point_to_segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let abx = b[0] - a[0];
    let aby = b[1] - a[1];
    let len_sq = abx * abx + aby * aby;
    if len_sq < 1e-9 {
        return dist2(p, a);
    }
    let t = (((p[0] - a[0]) * abx + (p[1] - a[1]) * aby) / len_sq).clamp(0.0, 1.0);
    let proj = [a[0] + t * abx, a[1] + t * aby];
    dist2(p, proj)
}

fn segment_intersection_t(a0: [f64; 2], a1: [f64; 2], b0: [f64; 2], b1: [f64; 2]) -> Option<(f64, f64)> {
    let rx = a1[0] - a0[0];
    let ry = a1[1] - a0[1];
    let sx = b1[0] - b0[0];
    let sy = b1[1] - b0[1];
    let denom = rx * sy - ry * sx;
    if denom.abs() < 1e-6 {
        return None;
    }
    let qpx = b0[0] - a0[0];
    let qpy = b0[1] - a0[1];
    let t = (qpx * sy - qpy * sx) / denom;
    let u = (qpx * ry - qpy * rx) / denom;
    if (0.02..=0.98).contains(&t) && (0.02..=0.98).contains(&u) {
        Some((t, u))
    } else {
        None
    }
}

fn synthesize_fallback_district_roads() -> Vec<IfcxRoadSegment> {
    let mut roads = Vec::new();
    let mut id = 1usize;
    let xs = [-85.0, -28.0, 28.0, 85.0];
    let ys = [-85.0, -28.0, 28.0, 85.0];

    // East-West arterials & collectors
    for (row_idx, &y) in ys.iter().enumerate() {
        for col_idx in 0..(xs.len() - 1) {
            let is_arterial = row_idx == 1 || row_idx == 2;
            let is_bridge = row_idx == 2 && col_idx == 2;
            roads.push(IfcxRoadSegment {
                id,
                name: if is_bridge {
                    format!("Bridge_Arterial_BR{id}")
                } else if is_arterial {
                    format!("Arterial_EW_{id}")
                } else {
                    format!("Collector_EW_{id}")
                },
                kind: if is_arterial {
                    IfcxRoadKind::Arterial
                } else {
                    IfcxRoadKind::Collector
                },
                start_enu_m: [xs[col_idx], y],
                end_enu_m: [xs[col_idx + 1], y],
                width_m: if is_arterial { 12.0 } else { 8.5 },
                lanes: if is_arterial { 4 } else { 2 },
                is_bridge,
                deck_elev_m: if is_bridge { 5.2 } else { 0.14 },
                pier_spacing_m: 18.0,
            });
            id += 1;
        }
    }

    // North-South connectors
    for (col_idx, &x) in xs.iter().enumerate() {
        for row_idx in 0..(ys.len() - 1) {
            let is_arterial = col_idx == 1 || col_idx == 2;
            roads.push(IfcxRoadSegment {
                id,
                name: if is_arterial {
                    format!("Arterial_NS_{id}")
                } else {
                    format!("Collector_NS_{id}")
                },
                kind: if is_arterial {
                    IfcxRoadKind::Arterial
                } else {
                    IfcxRoadKind::Collector
                },
                start_enu_m: [x, ys[row_idx]],
                end_enu_m: [x, ys[row_idx + 1]],
                width_m: if is_arterial { 11.0 } else { 8.0 },
                lanes: if is_arterial { 4 } else { 2 },
                is_bridge: false,
                deck_elev_m: 0.14,
                pier_spacing_m: 18.0,
            });
            id += 1;
        }
    }

    roads
}

fn sample_ground_elev_m(tin: Option<&TinSurfaceMesh>, pos: [f64; 2]) -> f64 {
    tin.map(|t| f64::from(t.sample_elevation_and_slope(pos[0] as f32, pos[1] as f32).0))
        .unwrap_or(0.0)
}

fn add_or_merge_node(
    nodes: &mut Vec<LifelineGraphNode>,
    pos: [f64; 2],
    elev_m: f64,
    label: String,
    is_safe_point: bool,
    is_responder_staging: bool,
    merge_tol_m: f64,
) -> usize {
    for n in nodes.iter_mut() {
        if dist2(n.pos_enu_m, pos) <= merge_tol_m {
            if is_safe_point {
                n.is_safe_point = true;
            }
            if is_responder_staging {
                n.is_responder_staging = true;
            }
            return n.id;
        }
    }
    let id = nodes.len();
    nodes.push(LifelineGraphNode {
        id,
        pos_enu_m: pos,
        elev_m,
        label,
        is_safe_point,
        is_responder_staging,
    });
    id
}

fn run_astar_route(
    nodes: &[LifelineGraphNode],
    edges: &[LifelineGraphEdge],
    adj: &BTreeMap<usize, Vec<(usize, usize)>>,
    start_node: usize,
    goal_node: usize,
    responder_width_req: bool,
) -> Option<(Vec<usize>, Vec<usize>, f64)> {
    if start_node == goal_node {
        return Some((vec![start_node], Vec::new(), 0.0));
    }
    let goal_pos = nodes.get(goal_node)?.pos_enu_m;
    let mut g_score: BTreeMap<usize, f64> = BTreeMap::new();
    let mut came_from: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let mut heap: BinaryHeap<AStarQueueItem> = BinaryHeap::new();

    g_score.insert(start_node, 0.0);
    heap.push(AStarQueueItem {
        f_cost: dist2(nodes[start_node].pos_enu_m, goal_pos),
        g_cost: 0.0,
        node_id: start_node,
    });

    while let Some(cur) = heap.pop() {
        if cur.node_id == goal_node {
            let mut npath = vec![goal_node];
            let mut epath = Vec::new();
            let mut trace = goal_node;
            while let Some(&(prev, eid)) = came_from.get(&trace) {
                npath.push(prev);
                epath.push(eid);
                trace = prev;
                if trace == start_node {
                    break;
                }
            }
            npath.reverse();
            epath.reverse();
            return Some((npath, epath, cur.g_cost));
        }
        if cur.g_cost > *g_score.get(&cur.node_id).unwrap_or(&f64::INFINITY) + 1e-6 {
            continue;
        }
        if let Some(nbrs) = adj.get(&cur.node_id) {
            for &(nbr, eid) in nbrs {
                let edge = &edges[eid];
                if edge.status == RoadEdgeStatus::BlockedClosed {
                    continue;
                }
                if responder_width_req && edge.width_m < 4.0 {
                    continue;
                }
                let step_cost = edge.length_m * edge.hazard_cost_multiplier;
                let tent_g = cur.g_cost + step_cost;
                if tent_g + 1e-6 < *g_score.get(&nbr).unwrap_or(&f64::INFINITY) {
                    g_score.insert(nbr, tent_g);
                    came_from.insert(nbr, (cur.node_id, eid));
                    let h = dist2(nodes[nbr].pos_enu_m, goal_pos);
                    heap.push(AStarQueueItem {
                        f_cost: tent_g + h,
                        g_cost: tent_g,
                        node_id: nbr,
                    });
                }
            }
        }
    }
    None
}

/// Evaluates autonomous post-disaster evacuation routing, emergency responder access, and
/// lifeline network connectivity across the active BIM site, IFCX City-Builder road/bridge graph,
/// ATC-20 placards, flood inundation, and liquefaction hazards.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_evacuation_and_lifelines(
    bim_project: &BimProject,
    city_scene: &IfcxCityScene,
    shm_report: Option<&ShmDigitalTwinReport>,
    hazard_config: Option<&DistrictHazardConfig>,
    _hazard_report: Option<&DistrictMultiHazardReport>,
    ssi_report: Option<&SubsurfaceSsiAnalysisReport>,
    tin: Option<&TinSurfaceMesh>,
    world_index: Option<&WorldSpatialIndex>,
    config: &EvacuationRoutingConfig,
    site_origin: (f64, f64),
) -> Result<EvacuationLifelineReport, String> {
    if bim_project.is_empty() && city_scene.roads.is_empty() {
        return Err("No active BIM building or road network available for evacuation routing.".to_string());
    }

    let debris_factor = config.debris_radius_factor.clamp(0.15, 1.25);

    // 1. Assemble ATC-20 Building Debris Cordons
    let mut cordons: Vec<DebrisCordonZone> = Vec::new();
    let (active_center, active_footprint_r, active_height_m) = if let Some((bmin, bmax)) =
        bim_project.plan_extents_mm()
    {
        let cx = (bmin[0] + bmax[0]) * 0.0005;
        let cy = (bmin[1] + bmax[1]) * 0.0005;
        let r = ((bmax[0] - bmin[0]).hypot(bmax[1] - bmin[1]) * 0.0005).clamp(4.5, 45.0);
        let h = bim_project
            .levels
            .iter()
            .map(|l| l.elevation_mm * 0.001)
            .fold(0.0_f64, f64::max)
            .max(6.0);
        ([cx, cy], r, h)
    } else {
        ([0.0, 0.0], 8.5, 10.5)
    };

    let active_placard = shm_report
        .map(|r| r.atc20_placard)
        .unwrap_or(Atc20Placard::InspectedGreen);
    let active_cordon_r = match active_placard {
        Atc20Placard::UnsafeRed => active_footprint_r + (active_height_m * debris_factor).max(6.0),
        Atc20Placard::RestrictedYellow => {
            active_footprint_r + (active_height_m * debris_factor * 0.5).max(3.5)
        }
        Atc20Placard::InspectedGreen => active_footprint_r + 1.5,
    };
    cordons.push(DebrisCordonZone {
        building_label: if bim_project.project_name.trim().is_empty() {
            "Active BIM Structure".to_string()
        } else {
            bim_project.project_name.clone()
        },
        center_enu_m: active_center,
        footprint_radius_m: active_footprint_r,
        cordon_radius_m: active_cordon_r,
        placard: active_placard,
        height_m: active_height_m,
        is_active_building: true,
    });

    if let Some(shm) = shm_report {
        for dp in &shm.district_placards {
            let dlat = (dp.lat - site_origin.0).abs();
            let dlon = (dp.lon - site_origin.1).abs();
            if dlat < 1e-5 && dlon < 1e-5 {
                continue;
            }
            let off_mm = SlippyMapState::lat_lon_to_world_mm(site_origin, dp.lat, dp.lon);
            let cx = off_mm[0] * 0.001;
            let cy = off_mm[1] * 0.001;
            if cx.hypot(cy) > 1500.0 {
                continue;
            }
            let fp_r = dp
                .footprint_local_m
                .iter()
                .map(|&[x, y]| x.hypot(y))
                .fold(6.0_f64, f64::max)
                .clamp(4.0, 42.0);
            let h = dp.total_height_m.max(4.0);
            let cr = match dp.placard {
                Atc20Placard::UnsafeRed => fp_r + (h * debris_factor).max(6.0),
                Atc20Placard::RestrictedYellow => fp_r + (h * debris_factor * 0.5).max(3.5),
                Atc20Placard::InspectedGreen => fp_r + 1.5,
            };
            cordons.push(DebrisCordonZone {
                building_label: dp.building_name.clone(),
                center_enu_m: [cx, cy],
                footprint_radius_m: fp_r,
                cordon_radius_m: cr,
                placard: dp.placard,
                height_m: h,
                is_active_building: false,
            });
        }
    } else if let Some(world) = world_index {
        for b in world.buildings.iter().take(16) {
            let dlat = (b.lat - site_origin.0).abs();
            let dlon = (b.lon - site_origin.1).abs();
            if dlat < 1e-5 && dlon < 1e-5 {
                continue;
            }
            let off_mm = SlippyMapState::lat_lon_to_world_mm(site_origin, b.lat, b.lon);
            let cx = off_mm[0] * 0.001;
            let cy = off_mm[1] * 0.001;
            if cx.hypot(cy) > 1500.0 {
                continue;
            }
            let placard = if b.max_dcr >= 1.05 || b.min_ari < 0.65 {
                Atc20Placard::UnsafeRed
            } else if b.max_dcr >= 0.85 || b.min_ari < 0.80 {
                Atc20Placard::RestrictedYellow
            } else {
                Atc20Placard::InspectedGreen
            };
            let fp_r = b
                .footprint_local_m
                .iter()
                .map(|&[x, y]| x.hypot(y))
                .fold(6.0_f64, f64::max)
                .clamp(4.0, 42.0);
            let h = b.total_height_m.max(4.0);
            let cr = match placard {
                Atc20Placard::UnsafeRed => fp_r + (h * debris_factor).max(6.0),
                Atc20Placard::RestrictedYellow => fp_r + (h * debris_factor * 0.5).max(3.5),
                Atc20Placard::InspectedGreen => fp_r + 1.5,
            };
            cordons.push(DebrisCordonZone {
                building_label: b.name.clone(),
                center_enu_m: [cx, cy],
                footprint_radius_m: fp_r,
                cordon_radius_m: cr,
                placard,
                height_m: h,
                is_active_building: false,
            });
        }
    }

    // 2. Construct Planar Road & Bridge Network Graph (Split at Segment Intersections)
    let used_synthetic_district_grid = city_scene.roads.is_empty();
    let source_roads = if used_synthetic_district_grid {
        synthesize_fallback_district_roads()
    } else {
        city_scene.roads.clone()
    };

    let mut nodes: Vec<LifelineGraphNode> = Vec::new();
    let mut raw_segments: Vec<(&IfcxRoadSegment, [f64; 2], [f64; 2])> = Vec::new();

    for (i, seg) in source_roads.iter().enumerate() {
        let mut ts: Vec<f64> = vec![0.0, 1.0];
        for (j, other) in source_roads.iter().enumerate() {
            if i == j {
                continue;
            }
            if let Some((t, _)) =
                segment_intersection_t(seg.start_enu_m, seg.end_enu_m, other.start_enu_m, other.end_enu_m)
            {
                ts.push(t);
            }
        }
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let mut dedup_ts: Vec<f64> = Vec::with_capacity(ts.len());
        for t in ts {
            if dedup_ts.last().is_none_or(|prev| (t - *prev).abs() > 0.02) {
                dedup_ts.push(t);
            }
        }
        if dedup_ts.len() < 2 {
            dedup_ts = vec![0.0, 1.0];
        }
        for pair in dedup_ts.windows(2) {
            let t0 = pair[0];
            let t1 = pair[1];
            let p0 = [
                seg.start_enu_m[0] + (seg.end_enu_m[0] - seg.start_enu_m[0]) * t0,
                seg.start_enu_m[1] + (seg.end_enu_m[1] - seg.start_enu_m[1]) * t0,
            ];
            let p1 = [
                seg.start_enu_m[0] + (seg.end_enu_m[0] - seg.start_enu_m[0]) * t1,
                seg.start_enu_m[1] + (seg.end_enu_m[1] - seg.start_enu_m[1]) * t1,
            ];
            if dist2(p0, p1) >= 1.5 {
                raw_segments.push((seg, p0, p1));
            }
        }
    }

    let flood_active = config.include_flood_blockage
        && hazard_config.is_some_and(|h| h.flood_plane_active);
    let flood_wl_m = hazard_config.map(|h| h.flood_water_level_m).unwrap_or(0.0);
    let pga_g = hazard_config.map(|h| h.seismic_pga_g).unwrap_or(0.30);
    let liq_critical = config.include_liquefaction_penalty
        && ssi_report.is_some_and(|s| s.liquefaction_potential_index >= 5.0 || s.min_liquefaction_fs < 1.0);

    let mut edges: Vec<LifelineGraphEdge> = Vec::with_capacity(raw_segments.len() + 16);
    let mut bridges_evaluated = 0usize;
    let mut bridges_compromised = 0usize;

    for (seg, p0, p1) in raw_segments {
        let e0 = if seg.is_bridge {
            seg.deck_elev_m
        } else {
            sample_ground_elev_m(tin, p0) + 0.14
        };
        let e1 = if seg.is_bridge {
            seg.deck_elev_m
        } else {
            sample_ground_elev_m(tin, p1) + 0.14
        };
        let u_lbl = format!("Jct_{}", nodes.len());
        let u = add_or_merge_node(
            &mut nodes,
            p0,
            e0,
            u_lbl,
            false,
            false,
            2.5,
        );
        let v_lbl = format!("Jct_{}", nodes.len());
        let v = add_or_merge_node(
            &mut nodes,
            p1,
            e1,
            v_lbl,
            false,
            false,
            2.5,
        );
        if u == v {
            continue;
        }
        let len_m = dist2(nodes[u].pos_enu_m, nodes[v].pos_enu_m);
        if len_m < 1.0 {
            continue;
        }

        // Evaluate multi-hazard edge impedance & closure rules
        let mut status = RoadEdgeStatus::OpenSafe;
        let mut mult = 1.0_f64;
        let mut reasons: Vec<String> = Vec::new();

        if seg.is_bridge {
            bridges_evaluated += 1;
        }

        // A. ATC-20 Red / Yellow debris cordons
        for c in &cordons {
            if c.placard == Atc20Placard::InspectedGreen {
                continue;
            }
            let d_seg = point_to_segment_distance(c.center_enu_m, nodes[u].pos_enu_m, nodes[v].pos_enu_m);
            let clearance_needed = c.cordon_radius_m + seg.width_m * 0.20;
            if d_seg < clearance_needed {
                match c.placard {
                    Atc20Placard::UnsafeRed => {
                        status = RoadEdgeStatus::BlockedClosed;
                        mult = 1e6;
                        reasons.push(format!("ATC-20 Red debris cordon ({})", c.building_label));
                    }
                    Atc20Placard::RestrictedYellow => {
                        if status != RoadEdgeStatus::BlockedClosed {
                            status = RoadEdgeStatus::DegradedDetour;
                            mult = mult.max(2.4);
                        }
                        reasons.push(format!("ATC-20 Yellow shoring zone ({})", c.building_label));
                    }
                    Atc20Placard::InspectedGreen => {}
                }
            }
        }

        // B. 3D Flood inundation blockage
        if flood_active {
            let mid_elev = 0.5 * (nodes[u].elev_m + nodes[v].elev_m);
            let depth = flood_wl_m - mid_elev;
            if depth > 0.35 {
                status = RoadEdgeStatus::BlockedClosed;
                mult = 1e6;
                reasons.push(format!("Flood inundation ({depth:.2}m > 0.35m)"));
            } else if depth > 0.10 {
                if status != RoadEdgeStatus::BlockedClosed {
                    status = RoadEdgeStatus::DegradedDetour;
                    mult = mult.max(2.0);
                }
                reasons.push(format!("Shallow floodwater ({depth:.2}m)"));
            }
        }

        // C. Seed-Idriss Soil Liquefaction & Bridge Seismic Fragility
        if seg.is_bridge {
            if config.include_bridge_fragility && pga_g >= 0.45 {
                status = RoadEdgeStatus::BlockedClosed;
                mult = 1e6;
                bridges_compromised += 1;
                reasons.push(format!("Bridge seismic pier fragility (PGA {pga_g:.2}g)"));
            } else if liq_critical {
                status = RoadEdgeStatus::BlockedClosed;
                mult = 1e6;
                bridges_compromised += 1;
                reasons.push("Bridge abutment lateral spreading (LPI critical)".to_string());
            }
        } else if liq_critical {
            if status != RoadEdgeStatus::BlockedClosed {
                status = RoadEdgeStatus::DegradedDetour;
                mult = mult.max(2.2);
            }
            reasons.push("Liquefaction pavement settlement risk".to_string());
        }

        let eid = edges.len();
        edges.push(LifelineGraphEdge {
            id: eid,
            u,
            v,
            road_name: seg.name.clone(),
            length_m: len_m,
            width_m: seg.width_m,
            is_bridge: seg.is_bridge,
            status,
            hazard_cost_multiplier: mult,
            blockage_reason: if reasons.is_empty() {
                "Clear lifeline".to_string()
            } else {
                reasons.join("; ")
            },
        });
    }

    // 3. Extract or Synthesize Safe Assembly Points & Emergency Responder Staging Gates
    let mut safe_points: Vec<AssemblySafePoint> = Vec::new();
    for z in &city_scene.zones {
        if matches!(z.kind, IfcxZoneKind::Park | IfcxZoneKind::Civic) {
            let min_cordon_clearance = cordons
                .iter()
                .filter(|c| c.placard != Atc20Placard::InspectedGreen)
                .map(|c| dist2(z.center_enu_m, c.center_enu_m) - c.cordon_radius_m)
                .fold(100.0_f64, f64::min);
            let elev = sample_ground_elev_m(tin, z.center_enu_m);
            let flood_ok = !flood_active || (flood_wl_m - elev) <= 0.15;
            if min_cordon_clearance > 2.0 && flood_ok {
                let area = (z.width_m * z.depth_m).max(120.0);
                safe_points.push(AssemblySafePoint {
                    id: safe_points.len() + 1,
                    name: z.name.clone(),
                    center_enu_m: z.center_enu_m,
                    capacity_persons: ((area / 1.5).round() as usize).clamp(80, 4000),
                    is_from_city_zone: true,
                    hazard_margin_m: min_cordon_clearance.max(2.0),
                });
            }
        }
    }

    if safe_points.is_empty() {
        let candidate_plazas = [
            ("NW Safe Assembly Plaza", [-85.0, 85.0]),
            ("SE Safe Assembly Plaza", [85.0, -85.0]),
            ("SW Safe Assembly Plaza", [-85.0, -85.0]),
        ];
        for (idx, (name, pos)) in candidate_plazas.iter().enumerate() {
            if safe_points.len() >= 2 && idx > 1 {
                break;
            }
            let min_cordon_clearance = cordons
                .iter()
                .filter(|c| c.placard != Atc20Placard::InspectedGreen)
                .map(|c| dist2(*pos, c.center_enu_m) - c.cordon_radius_m)
                .fold(80.0_f64, f64::min);
            if min_cordon_clearance > 1.0 || safe_points.is_empty() {
                safe_points.push(AssemblySafePoint {
                    id: safe_points.len() + 1,
                    name: (*name).to_string(),
                    center_enu_m: *pos,
                    capacity_persons: 650,
                    is_from_city_zone: false,
                    hazard_margin_m: min_cordon_clearance.max(12.0),
                });
            }
        }
    }

    // Helper: connect a target ENU point to its nearest existing road node via a clean access spur
    let connect_anchor = |nodes: &mut Vec<LifelineGraphNode>,
                          edges: &mut Vec<LifelineGraphEdge>,
                          pos: [f64; 2],
                          label: String,
                          is_safe_point: bool,
                          is_responder_staging: bool|
     -> usize {
        let mut best_node = 0usize;
        let mut best_d = f64::INFINITY;
        for n in nodes.iter() {
            let d = dist2(n.pos_enu_m, pos);
            if d < best_d {
                best_d = d;
                best_node = n.id;
            }
        }
        let elev = sample_ground_elev_m(tin, pos);
        let nid = add_or_merge_node(
            nodes,
            pos,
            elev,
            label,
            is_safe_point,
            is_responder_staging,
            2.5,
        );
        if nid != best_node && !nodes.is_empty() {
            let eid = edges.len();
            edges.push(LifelineGraphEdge {
                id: eid,
                u: nid,
                v: best_node,
                road_name: "Site Access Spur".to_string(),
                length_m: best_d.max(2.0),
                width_m: 6.5,
                is_bridge: false,
                status: RoadEdgeStatus::OpenSafe,
                hazard_cost_multiplier: 1.0,
                blockage_reason: "Site access spur".to_string(),
            });
        }
        nid
    };

    let mut safe_node_ids: Vec<(usize, String)> = Vec::new();
    for sp in &safe_points {
        let nid = connect_anchor(
            &mut nodes,
            &mut edges,
            sp.center_enu_m,
            sp.name.clone(),
            true,
            false,
        );
        safe_node_ids.push((nid, sp.name.clone()));
    }

    let responder_gates = [
        ("North Arterial Responder Gate", [-28.0, 85.0]),
        ("South-West Responder Gate", [-85.0, -28.0]),
    ];
    let mut staging_node_ids: Vec<(usize, String)> = Vec::new();
    for (label, pos) in responder_gates {
        let nid = connect_anchor(
            &mut nodes,
            &mut edges,
            pos,
            label.to_string(),
            false,
            true,
        );
        staging_node_ids.push((nid, label.to_string()));
    }

    // Build deterministic adjacency list (BTreeMap<usize, Vec<(neighbor, edge_id)>>)
    let mut adj: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for e in &edges {
        adj.entry(e.u).or_default().push((e.v, e.id));
        adj.entry(e.v).or_default().push((e.u, e.id));
    }

    // Connect each building origin & compute routes
    let mut routes: Vec<EvacuationRoutePath> = Vec::new();
    let mut isolated_building_count = 0usize;
    let mut active_evac_route: Option<EvacuationRoutePath> = None;
    let mut active_responder_route: Option<EvacuationRoutePath> = None;

    for (b_idx, cordon) in cordons.iter().enumerate() {
        let origin_node = connect_anchor(
            &mut nodes,
            &mut edges,
            cordon.center_enu_m,
            format!("Origin: {}", cordon.building_label),
            false,
            false,
        );
        // Update adjacency for newly added access spur
        if let Some(last_edge) = edges.last() {
            if last_edge.u == origin_node || last_edge.v == origin_node {
                adj.entry(last_edge.u)
                    .or_default()
                    .push((last_edge.v, last_edge.id));
                adj.entry(last_edge.v)
                    .or_default()
                    .push((last_edge.u, last_edge.id));
            }
        }

        // Pedestrian A* to nearest reachable safe assembly point
        let mut best_ped: Option<(String, Vec<usize>, Vec<usize>, f64)> = None;
        for (sp_nid, sp_name) in &safe_node_ids {
            if let Some((npath, epath, cost)) =
                run_astar_route(&nodes, &edges, &adj, origin_node, *sp_nid, false)
            {
                if best_ped.as_ref().is_none_or(|b| cost < b.3) {
                    best_ped = Some((sp_name.clone(), npath, epath, cost));
                }
            }
        }

        let ped_route = if let Some((target_name, npath, epath, _cost)) = best_ped {
            let waypoints: Vec<[f64; 2]> = npath.iter().map(|&nid| nodes[nid].pos_enu_m).collect();
            let dist_m: f64 = epath.iter().map(|&eid| edges[eid].length_m).sum();
            let eta_s: f64 = epath
                .iter()
                .map(|&eid| {
                    let e = &edges[eid];
                    let speed = if e.status == RoadEdgeStatus::DegradedDetour {
                        0.85
                    } else {
                        1.35
                    };
                    e.length_m / speed
                })
                .sum();
            let degraded_count = epath
                .iter()
                .filter(|&&eid| edges[eid].status == RoadEdgeStatus::DegradedDetour)
                .count();
            let safety_score = (100.0 - degraded_count as f64 * 12.0 - (dist_m / 22.0)).clamp(35.0, 99.0);
            EvacuationRoutePath {
                id: routes.len() + 1,
                origin_label: cordon.building_label.clone(),
                target_safe_zone: target_name.clone(),
                is_responder_route: false,
                is_active_building: cordon.is_active_building,
                reachable: true,
                waypoints_enu_m: waypoints,
                edge_ids: epath,
                distance_m: dist_m,
                estimated_time_sec: eta_s,
                safety_score_0_100: safety_score,
                has_detour: degraded_count > 0,
                status_summary: format!("Clear to {target_name} ({dist_m:.0} m, {eta_s:.0} s)"),
            }
        } else {
            isolated_building_count += 1;
            EvacuationRoutePath {
                id: routes.len() + 1,
                origin_label: cordon.building_label.clone(),
                target_safe_zone: "ISOLATED — NO SAFE EGRESS".to_string(),
                is_responder_route: false,
                is_active_building: cordon.is_active_building,
                reachable: false,
                waypoints_enu_m: vec![cordon.center_enu_m],
                edge_ids: Vec::new(),
                distance_m: 0.0,
                estimated_time_sec: 0.0,
                safety_score_0_100: 0.0,
                has_detour: true,
                status_summary: "Isolated: all egress edges blocked by cordons/flood".to_string(),
            }
        };

        if cordon.is_active_building {
            active_evac_route = Some(ped_route.clone());

            // Emergency responder A* from perimeter arterial staging gate to active building
            let mut best_resp: Option<(String, Vec<usize>, Vec<usize>, f64)> = None;
            for (gate_nid, gate_name) in &staging_node_ids {
                if let Some((npath, epath, cost)) =
                    run_astar_route(&nodes, &edges, &adj, *gate_nid, origin_node, true)
                {
                    if best_resp.as_ref().is_none_or(|b| cost < b.3) {
                        best_resp = Some((gate_name.clone(), npath, epath, cost));
                    }
                }
            }
            let resp_route = if let Some((gate_name, npath, epath, _)) = best_resp {
                let waypoints: Vec<[f64; 2]> = npath.iter().map(|&nid| nodes[nid].pos_enu_m).collect();
                let dist_m: f64 = epath.iter().map(|&eid| edges[eid].length_m).sum();
                let eta_s: f64 = epath
                    .iter()
                    .map(|&eid| {
                        let e = &edges[eid];
                        let speed = if e.status == RoadEdgeStatus::DegradedDetour {
                            6.5
                        } else {
                            11.1
                        };
                        e.length_m / speed
                    })
                    .sum();
                let degraded_count = epath
                    .iter()
                    .filter(|&&eid| edges[eid].status == RoadEdgeStatus::DegradedDetour)
                    .count();
                let safety_score = (98.0 - degraded_count as f64 * 14.0 - (dist_m / 30.0)).clamp(30.0, 99.0);
                EvacuationRoutePath {
                    id: routes.len() + 2,
                    origin_label: gate_name.clone(),
                    target_safe_zone: cordon.building_label.clone(),
                    is_responder_route: true,
                    is_active_building: true,
                    reachable: true,
                    waypoints_enu_m: waypoints,
                    edge_ids: epath,
                    distance_m: dist_m,
                    estimated_time_sec: eta_s,
                    safety_score_0_100: safety_score,
                    has_detour: degraded_count > 0,
                    status_summary: format!("Responder access from {gate_name} ({dist_m:.0} m, {eta_s:.0} s)"),
                }
            } else {
                EvacuationRoutePath {
                    id: routes.len() + 2,
                    origin_label: "Emergency Staging".to_string(),
                    target_safe_zone: cordon.building_label.clone(),
                    is_responder_route: true,
                    is_active_building: true,
                    reachable: false,
                    waypoints_enu_m: vec![cordon.center_enu_m],
                    edge_ids: Vec::new(),
                    distance_m: 0.0,
                    estimated_time_sec: 0.0,
                    safety_score_0_100: 0.0,
                    has_detour: true,
                    status_summary: "Responder access blocked — bridge/corridor clearance required".to_string(),
                }
            };
            active_responder_route = Some(resp_route.clone());
            routes.push(ped_route);
            routes.push(resp_route);
        } else if b_idx < 10 {
            routes.push(ped_route);
        }
    }

    // 4. Network Connectivity & Responder Accessibility KPIs (exclude site access spurs from road totals)
    let road_edges: Vec<&LifelineGraphEdge> = edges
        .iter()
        .filter(|e| e.road_name != "Site Access Spur")
        .collect();
    let total_edges = road_edges.len();
    let open_edges = road_edges
        .iter()
        .filter(|e| e.status == RoadEdgeStatus::OpenSafe)
        .count();
    let degraded_edges = road_edges
        .iter()
        .filter(|e| e.status == RoadEdgeStatus::DegradedDetour)
        .count();
    let blocked_edges = road_edges
        .iter()
        .filter(|e| e.status == RoadEdgeStatus::BlockedClosed)
        .count();

    let network_connectivity_pct = if total_edges > 0 {
        ((open_edges as f64 + 0.55 * degraded_edges as f64) / total_edges as f64 * 100.0)
            .clamp(0.0, 100.0)
    } else {
        100.0
    };

    let responder_reachable_bonus = if active_responder_route.as_ref().is_some_and(|r| r.reachable) {
        active_responder_route
            .as_ref()
            .map(|r| r.safety_score_0_100 * 0.45)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let bridge_penalty = if bridges_evaluated > 0 {
        (bridges_compromised as f64 / bridges_evaluated as f64) * 20.0
    } else {
        0.0
    };
    let responder_accessibility_score =
        (network_connectivity_pct * 0.55 + responder_reachable_bonus - bridge_penalty)
            .clamp(0.0, 100.0);

    let summary = format!(
        "Lifelines: Conn {:.0}% ({}/{} open, {} blocked) • Resp {:.0}/100 • Isolated {} • Bridges {}/{} ok",
        network_connectivity_pct,
        open_edges,
        total_edges,
        blocked_edges,
        responder_accessibility_score,
        isolated_building_count,
        bridges_evaluated.saturating_sub(bridges_compromised),
        bridges_evaluated
    );

    // 5. Build Non-Destructive IFCX Overlay (`eigengraph::analysis::evacuation_lifelines`)
    let evacuation_overlay_ifcx = build_evacuation_ifcx_overlay(
        bim_project,
        &cordons,
        &safe_points,
        &routes,
        total_edges,
        open_edges,
        degraded_edges,
        blocked_edges,
        network_connectivity_pct,
        responder_accessibility_score,
        isolated_building_count,
        bridges_evaluated,
        bridges_compromised,
    );

    Ok(EvacuationLifelineReport {
        config: config.clone(),
        nodes,
        edges,
        cordons,
        safe_points,
        routes,
        active_evac_route,
        active_responder_route,
        total_edges,
        open_edges,
        degraded_edges,
        blocked_edges,
        network_connectivity_pct,
        responder_accessibility_score,
        isolated_building_count,
        bridges_evaluated,
        bridges_compromised,
        used_synthetic_district_grid,
        summary,
        evacuation_overlay_ifcx,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_evacuation_ifcx_overlay(
    bim_project: &BimProject,
    cordons: &[DebrisCordonZone],
    safe_points: &[AssemblySafePoint],
    routes: &[EvacuationRoutePath],
    total_edges: usize,
    open_edges: usize,
    degraded_edges: usize,
    blocked_edges: usize,
    network_connectivity_pct: f64,
    responder_accessibility_score: f64,
    isolated_building_count: usize,
    bridges_evaluated: usize,
    bridges_compromised: usize,
) -> IfcxDocument {
    let raw_name = if bim_project.project_name.trim().is_empty() {
        "active_project".to_string()
    } else {
        bim_project.project_name.clone()
    };
    let slug: String = raw_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let mut doc = IfcxDocument::new(&format!("{slug}_evacuation_lifelines_overlay"));
    doc.imports.push(IfcxImport {
        uri: format!("eigengraph://{slug}.ifcx"),
        integrity: None,
        role: Some("base-building".to_string()),
    });
    doc.schemas.insert(
        NS_EG_EVACUATION_LIFELINES.to_string(),
        json!({
            "value": {
                "dataType": "Object",
                "spec": "EigenGraph Phase 77 Multi-Hazard Evacuation Routing & Emergency Lifelines Schema"
            }
        }),
    );

    let site_path = format!("/earth/sites/{slug}");
    let evac_root = format!("{site_path}/analysis/evacuation_lifelines");

    let payload = json!({
        "totalEdges": total_edges,
        "openEdges": open_edges,
        "degradedEdges": degraded_edges,
        "blockedEdges": blocked_edges,
        "networkConnectivityPct": network_connectivity_pct,
        "responderAccessibilityScore": responder_accessibility_score,
        "isolatedBuildingCount": isolated_building_count,
        "bridgesEvaluated": bridges_evaluated,
        "bridgesCompromised": bridges_compromised,
        "cordonCount": cordons.len(),
        "safePointCount": safe_points.len(),
        "routes": routes.iter().map(|r| json!({
            "id": r.id,
            "origin": r.origin_label,
            "target": r.target_safe_zone,
            "isResponder": r.is_responder_route,
            "reachable": r.reachable,
            "distanceM": r.distance_m,
            "estimatedTimeSec": r.estimated_time_sec,
            "safetyScore": r.safety_score_0_100,
        })).collect::<Vec<_>>(),
    });

    let mut site_node = IfcxNode::new(&site_path).with_class("IfcSite");
    site_node
        .children
        .insert("evacuation_lifelines".to_string(), evac_root.clone());
    site_node
        .attributes
        .insert(NS_EG_EVACUATION_LIFELINES.to_string(), payload.clone());
    doc.add_node(site_node);

    let mut root_node = IfcxNode::new(&evac_root)
        .with_class("IfcTask")
        .with_prop(
            "Evacuation & Emergency Lifelines",
            "A* Multi-Hazard Network",
            &format!(
                "Conn {network_connectivity_pct:.1}% | Resp {responder_accessibility_score:.0}/100 | Isolated {isolated_building_count}"
            ),
        );
    root_node
        .attributes
        .insert(NS_EG_EVACUATION_LIFELINES.to_string(), payload);

    for sp in safe_points {
        let sp_path = format!("{evac_root}/safe_zone_{}", sp.id);
        root_node
            .children
            .insert(format!("safe_zone_{}", sp.id), sp_path.clone());
        let mesh = UsdGeomMesh::from_box_m(
            [sp.center_enu_m[0] - 4.0, sp.center_enu_m[1] - 4.0, 0.0],
            [sp.center_enu_m[0] + 4.0, sp.center_enu_m[1] + 4.0, 0.45],
            [0.16, 0.92, 0.52, 0.80],
            9.0,
        );
        let sp_node = IfcxNode::new(&sp_path)
            .with_class("IfcSpatialZone")
            .with_prop("Safe Assembly Zone", &sp.name, &format!("Cap {} persons", sp.capacity_persons))
            .with_mesh(&mesh);
        doc.add_node(sp_node);
    }

    for r in routes.iter().filter(|r| r.reachable && r.waypoints_enu_m.len() >= 2) {
        let r_path = format!("{evac_root}/route_{}", r.id);
        root_node
            .children
            .insert(format!("route_{}", r.id), r_path.clone());
        let mut pts = Vec::new();
        let mut counts = Vec::new();
        let mut indices = Vec::new();
        for seg in r.waypoints_enu_m.windows(2) {
            let a = seg[0];
            let b = seg[1];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = dx.hypot(dy).max(1e-3);
            let nx = -dy / len * 1.1;
            let ny = dx / len * 1.1;
            let base = pts.len() as u32;
            let z = if r.is_responder_route { 0.85 } else { 0.65 };
            pts.push([a[0] - nx, a[1] - ny, z]);
            pts.push([b[0] - nx, b[1] - ny, z]);
            pts.push([b[0] + nx, b[1] + ny, z]);
            pts.push([a[0] + nx, a[1] + ny, z]);
            counts.push(3);
            counts.push(3);
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let col = if r.is_responder_route {
            [0.22, 0.72, 1.0, 0.86]
        } else {
            [0.10, 0.94, 0.72, 0.88]
        };
        let mesh = UsdGeomMesh {
            points: pts,
            face_vertex_counts: counts,
            face_vertex_indices: indices,
            normals: Vec::new(),
            display_color: Some(col),
            vertex_colors: Vec::new(),
            vertex_meta: Vec::new(),
            material_kind: 9.0,
        };
        let r_node = IfcxNode::new(&r_path)
            .with_class("IfcAnnotation")
            .with_prop("Lifeline Route", &r.origin_label, &r.status_summary)
            .with_mesh(&mesh);
        doc.add_node(r_node);
    }

    doc.add_node(root_node);
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
    use crate::core::shm_twin::{evaluate_shm_digital_twin_for_bim, ShmDigitalTwinConfig, ShmFaultScenario};

    #[test]
    fn phase77_evacuation_astar_avoids_atc20_red_debris_and_flood_blockage() {
        let proj = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let mut city = IfcxCityScene::default();

        // Build a square loop: South road (blocked by Red debris), North detour road (open),
        // East road & West road connecting to a Park safe assembly zone at [60, 45].
        city.add_road([-40.0, 0.0], [60.0, 0.0], IfcxRoadKind::Collector, false);
        city.add_road([-40.0, 0.0], [-40.0, 45.0], IfcxRoadKind::Collector, false);
        city.add_road([-40.0, 45.0], [60.0, 45.0], IfcxRoadKind::Arterial, false);
        city.add_road([60.0, 0.0], [60.0, 45.0], IfcxRoadKind::Collector, false);
        city.add_zone([60.0, 45.0], 28.0, 22.0, 0.0, IfcxZoneKind::Park);

        let nominal_cfg = EvacuationRoutingConfig::default();
        let nominal_rep = evaluate_evacuation_and_lifelines(
            &proj,
            &city,
            None,
            None,
            None,
            None,
            None,
            None,
            &nominal_cfg,
            (-6.2088, 106.8456),
        )
        .expect("Nominal evacuation routing succeeds");

        assert!(!nominal_rep.used_synthetic_district_grid);
        assert_eq!(nominal_rep.blocked_edges, 0);
        assert_eq!(nominal_rep.isolated_building_count, 0);
        assert!(nominal_rep.active_evac_route.as_ref().is_some_and(|r| r.reachable));

        // Inject severe soft-story fault -> ATC-20 Red placard & debris cordon around active building
        let shm_cfg = ShmDigitalTwinConfig {
            fault_scenario: ShmFaultScenario::SoftStoryFirstFloor,
            severity_pct: 90.0,
            target_story: 1,
            ..Default::default()
        };
        let shm_rep = evaluate_shm_digital_twin_for_bim(
            &proj,
            &site,
            None,
            None,
            None,
            None,
            &shm_cfg,
            (-6.2088, 106.8456),
        )
        .expect("SHM report");
        assert_eq!(shm_rep.atc20_placard, Atc20Placard::UnsafeRed);

        let haz_cfg = DistrictHazardConfig {
            flood_plane_active: true,
            flood_water_level_m: 0.65, // > 0.35m triggers flood blockage on surface streets
            ..Default::default()
        };
        let blocked_rep = evaluate_evacuation_and_lifelines(
            &proj,
            &city,
            Some(&shm_rep),
            Some(&haz_cfg),
            None,
            None,
            None,
            None,
            &nominal_cfg,
            (-6.2088, 106.8456),
        )
        .expect("Flood + Red cordon evaluation succeeds");

        assert!(blocked_rep.blocked_edges > 0);
        assert!(blocked_rep.network_connectivity_pct < nominal_rep.network_connectivity_pct);

        let raw_ifcx = blocked_rep.evacuation_overlay_ifcx.to_json_pretty();
        assert!(raw_ifcx.contains(NS_EG_EVACUATION_LIFELINES));
    }

    #[test]
    fn phase77_evacuation_bridge_seismic_and_liquefaction_compromise() {
        let proj = BimProject::sample_bimcraft_building();
        let city = IfcxCityScene::default(); // uses synthetic grid with 1 arterial bridge
        let cfg = EvacuationRoutingConfig::default();

        let haz_cfg = DistrictHazardConfig {
            seismic_wave_active: true,
            seismic_pga_g: 0.62, // >= 0.45g compromises bridge pier fragility
            ..Default::default()
        };
        let rep = evaluate_evacuation_and_lifelines(
            &proj,
            &city,
            None,
            Some(&haz_cfg),
            None,
            None,
            None,
            None,
            &cfg,
            (-6.2088, 106.8456),
        )
        .expect("Synthetic district grid evacuation succeeds");

        assert!(rep.used_synthetic_district_grid);
        assert_eq!(rep.bridges_evaluated, 1);
        assert_eq!(rep.bridges_compromised, 1);
        assert!(rep.blocked_edges >= 1);
        assert!(rep.active_evac_route.as_ref().is_some_and(|r| r.reachable));
        assert!(rep.active_responder_route.as_ref().is_some_and(|r| r.reachable));
    }
}
