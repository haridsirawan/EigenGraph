//! vector_layers.rs — Streaming OpenStreetMap Vector & GeoJSON Cadastral Layer (Phase 59 / v0.5.9).
//!
//! Provides pure-Rust GeoJSON parsing (roads, rivers, parcel boundaries, and context building
//! footprints), async OSM Overpass vector tile caching (both WASM & Native), and vector edge/vertex
//! OSNAP snapping for crisp zero-blur site engineering overlays on Earth 3D.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::map_widget::SlippyMapState;

/// Category of an urban/cadastral vector geometry drawn in pure Rust over Earth 3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrbanVectorKind {
    RoadMajor,
    RoadMinor,
    RiverWaterway,
    ParcelBoundary,
    ContextBuilding,
}

impl UrbanVectorKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::RoadMajor => "Major Road",
            Self::RoadMinor => "Street",
            Self::RiverWaterway => "Waterway",
            Self::ParcelBoundary => "Parcel",
            Self::ContextBuilding => "Context Building",
        }
    }
}

/// Crisp pure-Rust geographic polyline/polygon feature ready for curved Earth 3D projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrbanVectorFeature {
    pub id: u64,
    pub kind: UrbanVectorKind,
    pub name: String,
    pub points_lat_lon: Vec<(f64, f64)>,
    pub closed: bool,
    pub height_m: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellFetchState {
    Loading,
    Ready,
    Failed,
}

/// Bounded cache managing GeoJSON imports and streamed OSM Overpass urban vector features.
#[derive(Clone, Default)]
pub struct UrbanVectorLayerCache {
    pub features: Arc<Mutex<Vec<UrbanVectorFeature>>>,
    cell_states: Arc<Mutex<HashMap<(i32, i32), CellFetchState>>>,
    pub live_osm_streaming: bool,
}

impl std::fmt::Debug for UrbanVectorLayerCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.features.lock().map(|v| v.len()).unwrap_or(0);
        f.debug_struct("UrbanVectorLayerCache")
            .field("feature_count", &count)
            .field("live_osm_streaming", &self.live_osm_streaming)
            .finish()
    }
}

impl UrbanVectorLayerCache {
    const CELL_DEG: f64 = 0.015; // ~1.6 km per vector tile cell
    const MAX_CACHED_FEATURES: usize = 4500;

    pub fn feature_count(&self) -> usize {
        self.features.lock().map(|v| v.len()).unwrap_or(0)
    }

    pub fn snapshot_visible(
        &self,
        min_lat: f64,
        max_lat: f64,
        min_lon: f64,
        max_lon: f64,
        max_features: usize,
    ) -> Vec<UrbanVectorFeature> {
        let Ok(guard) = self.features.lock() else {
            return Vec::new();
        };
        guard
            .iter()
            .filter(|feat| {
                feat.points_lat_lon.iter().any(|&(la, lo)| {
                    la >= min_lat && la <= max_lat && lo >= min_lon && lo <= max_lon
                })
            })
            .take(max_features)
            .cloned()
            .collect()
    }

    /// Parses GeoJSON (`FeatureCollection`, `Feature`, `Polygon`, `MultiPolygon`, `LineString`, `MultiLineString`)
    /// into crisp `UrbanVectorFeature` items and computes geographic centroid `(lat, lon)`.
    pub fn ingest_geojson(&self, raw: &str, fallback_label: &str) -> Result<(usize, Option<(f64, f64)>), String> {
        let parsed = parse_geojson_features(raw, fallback_label)?;
        if parsed.is_empty() {
            return Err("GeoJSON contained no valid LineString or Polygon features.".to_string());
        }
        let mut sum_lat = 0.0_f64;
        let mut sum_lon = 0.0_f64;
        let mut pt_cnt = 0usize;
        for f in &parsed {
            for &(la, lo) in &f.points_lat_lon {
                sum_lat += la;
                sum_lon += lo;
                pt_cnt += 1;
            }
        }
        let added = parsed.len();
        if let Ok(mut guard) = self.features.lock() {
            guard.extend(parsed);
            if guard.len() > Self::MAX_CACHED_FEATURES {
                let excess = guard.len() - Self::MAX_CACHED_FEATURES;
                guard.drain(0..excess);
            }
        }
        let centroid = if pt_cnt > 0 {
            Some((sum_lat / pt_cnt as f64, sum_lon / pt_cnt as f64))
        } else {
            None
        };
        Ok((added, centroid))
    }

    /// Requests asynchronous OpenStreetMap Overpass vector data around `(lat, lon)` when zoomed in (`>= 14.5`).
    pub fn ensure_osm_cell_loaded(&self, ctx: &egui::Context, lat: f64, lon: f64, zoom_level: f32) {
        if !self.live_osm_streaming || zoom_level < 14.5 {
            return;
        }
        let ci = (lat / Self::CELL_DEG).floor() as i32;
        let cj = (lon / Self::CELL_DEG).floor() as i32;
        {
            let Ok(mut states) = self.cell_states.lock() else {
                return;
            };
            if states.contains_key(&(ci, cj)) {
                return;
            }
            if states.len() > 24 {
                states.clear();
            }
            states.insert((ci, cj), CellFetchState::Loading);
        }

        let s = ci as f64 * Self::CELL_DEG;
        let n = s + Self::CELL_DEG;
        let w = cj as f64 * Self::CELL_DEG;
        let e = w + Self::CELL_DEG;
        let query = format!(
            "[out:json][timeout:12];(way[\"highway\"]({s:.5},{w:.5},{n:.5},{e:.5});way[\"waterway\"]({s:.5},{w:.5},{n:.5},{e:.5});way[\"building\"]({s:.5},{w:.5},{n:.5},{e:.5}););out geom qt;"
        );
        let encoded: String = query
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect();
        let url = format!("https://overpass-api.de/api/interpreter?data={encoded}");
        let _ = (ctx, &url);

        #[cfg(all(not(target_arch = "wasm32"), not(test)))]
        {
            let features_arc = self.features.clone();
            let states_arc = self.cell_states.clone();
            let ctx_clone = ctx.clone();
            std::thread::spawn(move || {
                let body = std::process::Command::new("curl")
                    .args([
                        "-sL",
                        "--max-time",
                        "12",
                        "-A",
                        "EigenGraph-BIM/0.6.2 (https://eigengraph.net)",
                        &url,
                    ])
                    .output()
                    .ok()
                    .and_then(|o| {
                        if o.status.success() {
                            String::from_utf8(o.stdout).ok()
                        } else {
                            None
                        }
                    });
                let parsed = body.as_deref().and_then(|txt| parse_overpass_json(txt).ok());
                if let Some(items) = parsed {
                    if let Ok(mut fg) = features_arc.lock() {
                        fg.extend(items);
                        if fg.len() > Self::MAX_CACHED_FEATURES {
                            let excess = fg.len() - Self::MAX_CACHED_FEATURES;
                            fg.drain(0..excess);
                        }
                    }
                    if let Ok(mut sg) = states_arc.lock() {
                        sg.insert((ci, cj), CellFetchState::Ready);
                    }
                } else if let Ok(mut sg) = states_arc.lock() {
                    sg.insert((ci, cj), CellFetchState::Failed);
                }
                ctx_clone.request_repaint();
            });
        }

        #[cfg(all(target_arch = "wasm32", not(test)))]
        {
            let features_arc = self.features.clone();
            let states_arc = self.cell_states.clone();
            let ctx_clone = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let text = fetch_text_wasm(&url).await;
                let parsed = text.as_deref().and_then(|txt| parse_overpass_json(txt).ok());
                if let Some(items) = parsed {
                    if let Ok(mut fg) = features_arc.lock() {
                        fg.extend(items);
                        if fg.len() > Self::MAX_CACHED_FEATURES {
                            let excess = fg.len() - Self::MAX_CACHED_FEATURES;
                            fg.drain(0..excess);
                        }
                    }
                    if let Ok(mut sg) = states_arc.lock() {
                        sg.insert((ci, cj), CellFetchState::Ready);
                    }
                } else if let Ok(mut sg) = states_arc.lock() {
                    sg.insert((ci, cj), CellFetchState::Failed);
                }
                ctx_clone.request_repaint();
            });
        }
    }

    /// Snaps `(lat, lon)` onto the nearest loaded road or parcel boundary edge within `max_dist_m`,
    /// returning `(snapped_lat, snapped_lon, segment_azimuth_deg)`.
    pub fn snap_lat_lon_to_vector_edge(
        &self,
        target: (f64, f64),
        max_dist_m: f64,
    ) -> Option<(f64, f64, f64)> {
        let Ok(guard) = self.features.lock() else {
            return None;
        };
        let mut best_dist_m = max_dist_m.max(0.5);
        let mut best: Option<(f64, f64, f64)> = None;

        for feat in guard.iter() {
            if feat.points_lat_lon.len() < 2 {
                continue;
            }
            let seg_count = if feat.closed {
                feat.points_lat_lon.len()
            } else {
                feat.points_lat_lon.len() - 1
            };
            for i in 0..seg_count {
                let a = feat.points_lat_lon[i];
                let b = feat.points_lat_lon[(i + 1) % feat.points_lat_lon.len()];
                let a_mm = SlippyMapState::lat_lon_to_world_mm(target, a.0, a.1);
                let b_mm = SlippyMapState::lat_lon_to_world_mm(target, b.0, b.1);
                let ab_x = (b_mm[0] - a_mm[0]) * 0.001;
                let ab_y = (b_mm[1] - a_mm[1]) * 0.001;
                let l2 = ab_x * ab_x + ab_y * ab_y;
                if l2 < 0.01 {
                    continue;
                }
                let t = ((-(a_mm[0] * 0.001) * ab_x - (a_mm[1] * 0.001) * ab_y) / l2).clamp(0.0, 1.0);
                let px_m = (a_mm[0] * 0.001) + t * ab_x;
                let py_m = (a_mm[1] * 0.001) + t * ab_y;
                let dist_m = px_m.hypot(py_m);
                if dist_m < best_dist_m {
                    best_dist_m = dist_m;
                    let snapped = SlippyMapState::world_mm_to_lat_lon(target, [px_m * 1000.0, py_m * 1000.0]);
                    let az = ab_x.atan2(ab_y).to_degrees().rem_euclid(360.0);
                    best = Some((snapped.0, snapped.1, az));
                }
            }
        }
        best
    }
}

#[cfg(all(target_arch = "wasm32", not(test)))]
async fn fetch_text_wasm(url: &str) -> Option<String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window()?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(web_sys::RequestMode::Cors);
    let request = web_sys::Request::new_with_str_and_init(url, &opts).ok()?;
    let resp_val = JsFuture::from(window.fetch_with_request(&request)).await.ok()?;
    let resp: web_sys::Response = resp_val.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let text_promise = resp.text().ok()?;
    let txt_val = JsFuture::from(text_promise).await.ok()?;
    txt_val.as_string()
}

fn stable_vector_id(seed: &str, idx: usize) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ (idx as u64);
    for b in seed.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Parses standard RFC 7946 GeoJSON into `UrbanVectorFeature` records.
pub fn parse_geojson_features(raw: &str, fallback_label: &str) -> Result<Vec<UrbanVectorFeature>, String> {
    let v: Value = serde_json::from_str(raw.trim_start_matches('\u{FEFF}'))
        .map_err(|e| format!("Invalid GeoJSON: {e}"))?;
    let mut out = Vec::new();
    let features = match v.get("type").and_then(|t| t.as_str()) {
        Some("FeatureCollection") => v
            .get("features")
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default(),
        Some("Feature") => vec![v],
        _ => Vec::new(),
    };

    for (idx, feat) in features.iter().enumerate() {
        let props = feat.get("properties").and_then(|p| p.as_object());
        let name = props
            .and_then(|p| p.get("name").or_else(|| p.get("Name")))
            .and_then(|n| n.as_str())
            .unwrap_or(fallback_label)
            .to_string();
        let kind = classify_props_kind(props);
        let height_m = props
            .and_then(|p| {
                p.get("height")
                    .and_then(|h| h.as_f64().or_else(|| h.as_str().and_then(|s| s.parse().ok())))
                    .or_else(|| {
                        p.get("building:levels")
                            .and_then(|l| l.as_f64().or_else(|| l.as_str().and_then(|s| s.parse().ok())))
                            .map(|lvl| lvl * 3.5)
                    })
            })
            .unwrap_or(12.0) as f32;

        let Some(geom) = feat.get("geometry") else {
            continue;
        };
        let gtype = geom.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let Some(coords) = geom.get("coordinates") else {
            continue;
        };
        let push_ring = |ring: &[Value], closed: bool, kind_override: UrbanVectorKind, out_vec: &mut Vec<UrbanVectorFeature>| {
            let pts: Vec<(f64, f64)> = ring
                .iter()
                .filter_map(|c| {
                    let arr = c.as_array()?;
                    let lon = arr.first()?.as_f64()?;
                    let lat = arr.get(1)?.as_f64()?;
                    if (-85.0..=85.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                        Some((lat, lon))
                    } else {
                        None
                    }
                })
                .collect();
            if pts.len() >= 2 {
                out_vec.push(UrbanVectorFeature {
                    id: stable_vector_id(&name, out_vec.len() + idx),
                    kind: kind_override,
                    name: name.clone(),
                    points_lat_lon: pts,
                    closed,
                    height_m,
                });
            }
        };

        match gtype {
            "LineString" => {
                if let Some(arr) = coords.as_array() {
                    push_ring(arr, false, kind, &mut out);
                }
            }
            "MultiLineString" => {
                if let Some(lines) = coords.as_array() {
                    for l in lines {
                        if let Some(arr) = l.as_array() {
                            push_ring(arr, false, kind, &mut out);
                        }
                    }
                }
            }
            "Polygon" => {
                let poly_kind = if kind == UrbanVectorKind::RoadMinor {
                    UrbanVectorKind::ParcelBoundary
                } else {
                    kind
                };
                if let Some(outer) = coords.as_array().and_then(|r| r.first()).and_then(|o| o.as_array()) {
                    push_ring(outer, true, poly_kind, &mut out);
                }
            }
            "MultiPolygon" => {
                let poly_kind = if kind == UrbanVectorKind::RoadMinor {
                    UrbanVectorKind::ParcelBoundary
                } else {
                    kind
                };
                if let Some(polys) = coords.as_array() {
                    for p in polys {
                        if let Some(outer) = p.as_array().and_then(|r| r.first()).and_then(|o| o.as_array()) {
                            push_ring(outer, true, poly_kind, &mut out);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

fn classify_props_kind(props: Option<&serde_json::Map<String, Value>>) -> UrbanVectorKind {
    let Some(p) = props else {
        return UrbanVectorKind::ParcelBoundary;
    };
    if let Some(hw) = p.get("highway").and_then(|v| v.as_str()) {
        return match hw {
            "motorway" | "trunk" | "primary" | "secondary" => UrbanVectorKind::RoadMajor,
            _ => UrbanVectorKind::RoadMinor,
        };
    }
    if p.get("waterway").is_some() {
        return UrbanVectorKind::RiverWaterway;
    }
    if p.get("building").is_some() {
        return UrbanVectorKind::ContextBuilding;
    }
    if p.get("parcel").is_some() || p.get("boundary").is_some() || p.get("landuse").is_some() {
        return UrbanVectorKind::ParcelBoundary;
    }
    UrbanVectorKind::RoadMinor
}

/// Parses OpenStreetMap Overpass JSON `elements` (`out geom`) into `UrbanVectorFeature`.
pub fn parse_overpass_json(raw: &str) -> Result<Vec<UrbanVectorFeature>, String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("Invalid Overpass JSON: {e}"))?;
    let Some(elems) = v.get("elements").and_then(|e| e.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(elems.len().min(600));
    for el in elems {
        let Some(geom) = el.get("geometry").and_then(|g| g.as_array()) else {
            continue;
        };
        let pts: Vec<(f64, f64)> = geom
            .iter()
            .filter_map(|pt| Some((pt.get("lat")?.as_f64()?, pt.get("lon")?.as_f64()?)))
            .collect();
        if pts.len() < 2 {
            continue;
        }
        let tags = el.get("tags").and_then(|t| t.as_object());
        let kind = classify_props_kind(tags);
        let closed = kind == UrbanVectorKind::ContextBuilding || kind == UrbanVectorKind::ParcelBoundary;
        let name = tags
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let height_m = tags
            .and_then(|t| {
                t.get("height")
                    .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    .or_else(|| {
                        t.get("building:levels")
                            .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                            .map(|lv| lv * 3.5)
                    })
            })
            .unwrap_or(10.5) as f32;
        out.push(UrbanVectorFeature {
            id: el.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
            kind,
            name,
            points_lat_lon: pts,
            closed,
            height_m,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase59_geojson_and_overpass_parsing_plus_vector_snap() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": { "name": "Jl. Sudirman Arterial", "highway": "primary" },
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[106.8200, -6.2200], [106.8250, -6.2200]]
                    }
                },
                {
                    "type": "Feature",
                    "properties": { "name": "Lot B-07", "landuse": "commercial", "height": 42.0 },
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[106.8210, -6.2205], [106.8220, -6.2205], [106.8220, -6.2212], [106.8210, -6.2212], [106.8210, -6.2205]]]
                    }
                }
            ]
        }"#;

        let cache = UrbanVectorLayerCache::default();
        let (cnt, centroid) = cache.ingest_geojson(geojson, "Site").expect("Parse GeoJSON");
        assert_eq!(cnt, 2);
        let c = centroid.expect("Centroid computed");
        assert!((c.0 - (-6.2205)).abs() < 0.01);

        // Vector edge OSNAP snaps nearby point smoothly onto East-West road centerline (90 deg azimuth)
        let snapped = cache
            .snap_lat_lon_to_vector_edge((-6.22004, 106.8225), 25.0)
            .expect("Point within 25m of road should snap cleanly");
        assert!((snapped.0 - (-6.2200)).abs() < 1e-5);
        assert!((snapped.2 - 90.0).abs() < 2.0);

        let overpass = r#"{
            "elements": [
                {
                    "id": 9001,
                    "tags": { "name": "Ciliwung River", "waterway": "river" },
                    "geometry": [{ "lat": -6.210, "lon": 106.850 }, { "lat": -6.215, "lon": 106.852 }]
                }
            ]
        }"#;
        let osm_feats = parse_overpass_json(overpass).expect("Parse Overpass JSON");
        assert_eq!(osm_feats.len(), 1);
        assert_eq!(osm_feats[0].kind, UrbanVectorKind::RiverWaterway);
    }
}
