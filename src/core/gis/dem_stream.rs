//! dem_stream.rs — Real-World Terrarium RGB DEM Tile Cache & Survey-to-TIN Builder (Phase 59 / v0.5.9).
//!
//! Decodes AWS/Mapzen 256x256 Terrarium elevation PNG tiles (`elev_m = (R * 256 + G + B / 256) - 32768`)
//! into local `TinSurfaceMesh` surfaces on Earth 3D, with both async browser WASM `fetch`
//! and desktop background `curl` streaming plus CSV/XYZ topographic point-cloud conversion.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::map_widget::{SlippyMapState, WebMercator};
use super::tin_surface::{TinPoint, TinSurfaceMesh};

#[derive(Clone)]
pub struct DecodedDemTile {
    pub width: u32,
    pub height: u32,
    pub elev_m: Vec<f32>,
}

#[allow(dead_code)]
#[derive(Clone)]
enum DemTileEntry {
    Loading,
    Ready(DecodedDemTile),
    Failed,
}

/// Asynchronous Terrarium DEM raster tile cache & local terrain TIN triangulator.
#[derive(Clone, Default)]
pub struct DemTileCache {
    inner: Arc<Mutex<HashMap<(u8, u32, u32), DemTileEntry>>>,
}

impl std::fmt::Debug for DemTileCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.inner.lock().map(|g| g.len()).unwrap_or(0);
        f.debug_struct("DemTileCache").field("cached_tiles", &n).finish()
    }
}

/// Decodes AWS/Mapzen Terrarium RGB triplet `(r, g, b)` to orthometric elevation in meters MSL.
#[inline]
pub fn decode_terrarium_rgb(r: u8, g: u8, b: u8) -> f32 {
    (r as f32) * 256.0 + (g as f32) + (b as f32) / 256.0 - 32768.0
}

pub fn decode_terrarium_png_bytes(bytes: &[u8]) -> Option<DecodedDemTile> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let width = img.width();
    let height = img.height();
    let raw = img.as_raw();
    let mut elev_m = Vec::with_capacity((width * height) as usize);
    for px in raw.chunks_exact(4) {
        elev_m.push(decode_terrarium_rgb(px[0], px[1], px[2]));
    }
    Some(DecodedDemTile {
        width,
        height,
        elev_m,
    })
}

impl DemTileCache {
    const DEM_ZOOM: u8 = 13;

    pub fn insert_decoded_for_test(&self, z: u8, x: u32, y: u32, tile: DecodedDemTile) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert((z, x, y), DemTileEntry::Ready(tile));
        }
    }

    /// Requests asynchronous Terrarium DEM tile fetch for `(lat, lon)` when zoom >= 11.5.
    pub fn ensure_tile_for_lat_lon(&self, ctx: &egui::Context, lat: f64, lon: f64, zoom: f32) {
        if zoom < 11.5 {
            return;
        }
        let z = Self::DEM_ZOOM;
        let (tx_f, ty_f) = WebMercator::lat_lon_to_tile_coords(lat, lon, z as f32);
        let n = 1u32 << z;
        let x = (tx_f.floor() as u32).min(n - 1);
        let y = (ty_f.floor() as u32).min(n - 1);
        {
            let Ok(mut guard) = self.inner.lock() else {
                return;
            };
            if guard.contains_key(&(z, x, y)) {
                return;
            }
            if guard.len() > 64 {
                guard.clear();
            }
            guard.insert((z, x, y), DemTileEntry::Loading);
        }
        let url = format!("https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png");
        let _ = (ctx, &url);

        #[cfg(all(not(target_arch = "wasm32"), not(test)))]
        {
            let inner = self.inner.clone();
            let ctx_clone = ctx.clone();
            std::thread::spawn(move || {
                let decoded = std::process::Command::new("curl")
                    .args(["-sL", "--max-time", "10", &url])
                    .output()
                    .ok()
                    .and_then(|o| {
                        if o.status.success() && !o.stdout.is_empty() {
                            decode_terrarium_png_bytes(&o.stdout)
                        } else {
                            None
                        }
                    });
                if let Ok(mut guard) = inner.lock() {
                    if let Some(tile) = decoded {
                        guard.insert((z, x, y), DemTileEntry::Ready(tile));
                    } else {
                        guard.insert((z, x, y), DemTileEntry::Failed);
                    }
                }
                ctx_clone.request_repaint();
            });
        }

        #[cfg(all(target_arch = "wasm32", not(test)))]
        {
            let inner = self.inner.clone();
            let ctx_clone = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let bytes = fetch_bytes_wasm(&url).await;
                let decoded = bytes.as_deref().and_then(decode_terrarium_png_bytes);
                if let Ok(mut guard) = inner.lock() {
                    if let Some(tile) = decoded {
                        guard.insert((z, x, y), DemTileEntry::Ready(tile));
                    } else {
                        guard.insert((z, x, y), DemTileEntry::Failed);
                    }
                }
                ctx_clone.request_repaint();
            });
        }
    }

    /// Samples elevation (m MSL) at `(lat, lon)` from cached Terrarium tiles if already ready.
    pub fn sample_elevation_m(&self, lat: f64, lon: f64) -> Option<f32> {
        let z = Self::DEM_ZOOM;
        let (tx_f, ty_f) = WebMercator::lat_lon_to_tile_coords(lat, lon, z as f32);
        let n = 1u32 << z;
        let x = (tx_f.floor() as u32).min(n - 1);
        let y = (ty_f.floor() as u32).min(n - 1);
        let Ok(guard) = self.inner.lock() else {
            return None;
        };
        let DemTileEntry::Ready(ref tile) = guard.get(&(z, x, y))? else {
            return None;
        };
        let u = (tx_f - x as f64).clamp(0.0, 0.9999);
        let v = (ty_f - y as f64).clamp(0.0, 0.9999);
        let px = ((u * tile.width as f64) as u32).min(tile.width.saturating_sub(1));
        let py = ((v * tile.height as f64) as u32).min(tile.height.saturating_sub(1));
        tile.elev_m.get((py * tile.width + px) as usize).copied()
    }

    /// Builds a Delaunay-triangulated `TinSurfaceMesh` centered at `origin` (`span_m` × `span_m`)
    /// directly from live cached Terrarium DEM elevation samples.
    pub fn build_site_tin(&self, origin: (f64, f64), span_m: f32, grid_n: usize) -> Option<TinSurfaceMesh> {
        let n = grid_n.clamp(8, 28);
        let half = span_m * 0.5;
        let step = span_m / (n - 1) as f32;
        let mut pts = Vec::with_capacity(n * n);
        let mut sampled_any = false;
        let mut id = 0usize;
        for row in 0..n {
            let y_m = -half + row as f32 * step;
            for col in 0..n {
                let x_m = -half + col as f32 * step;
                let ll = SlippyMapState::world_mm_to_lat_lon(origin, [x_m as f64 * 1000.0, y_m as f64 * 1000.0]);
                let z = match self.sample_elevation_m(ll.0, ll.1) {
                    Some(ez) => {
                        sampled_any = true;
                        ez
                    }
                    None => return None,
                };
                pts.push(TinPoint { id, x: x_m, y: y_m, z });
                id += 1;
            }
        }
        if !sampled_any || pts.len() < 4 {
            return None;
        }
        Some(TinSurfaceMesh::triangulate_bowyer_watson(pts))
    }
}

/// Converts raw CSV/XYZ (`x, y, z`) topographic survey text directly into a `TinSurfaceMesh`
/// centered around `(0, 0)` local tangent meters for Earth 3D draping.
pub fn parse_survey_xyz_to_tin(content: &str) -> Result<TinSurfaceMesh, String> {
    let mut raw: Vec<(f32, f32, f32)> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let cleaned = trimmed.replace([',', ';', '\t'], " ");
        let nums: Vec<f32> = cleaned
            .split_whitespace()
            .filter_map(|t| t.parse::<f32>().ok())
            .collect();
        if nums.len() >= 3 {
            raw.push((nums[0], nums[1], nums[2]));
        }
    }
    if raw.len() < 3 {
        return Err("Survey file requires at least 3 numeric (X, Y, Z) points.".to_string());
    }
    let min_x = raw.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let max_x = raw.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let min_y = raw.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max_y = raw.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    let mid_x = 0.5 * (min_x + max_x);
    let mid_y = 0.5 * (min_y + max_y);

    let step = (raw.len() / 600).max(1);
    let pts: Vec<TinPoint> = raw
        .into_iter()
        .step_by(step)
        .enumerate()
        .map(|(id, (x, y, z))| TinPoint {
            id,
            x: x - mid_x,
            y: y - mid_y,
            z,
        })
        .collect();
    Ok(TinSurfaceMesh::triangulate_bowyer_watson(pts))
}

#[cfg(all(target_arch = "wasm32", not(test)))]
async fn fetch_bytes_wasm(url: &str) -> Option<Vec<u8>> {
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
    let buf_promise = resp.array_buffer().ok()?;
    let buf_val = JsFuture::from(buf_promise).await.ok()?;
    Some(js_sys::Uint8Array::new(&buf_val).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase59_terrarium_decode_and_survey_tin_conversion() {
        // Sea level RGB = (128, 0, 0) => 128*256 - 32768 = 0.0m
        assert!((decode_terrarium_rgb(128, 0, 0) - 0.0).abs() < 1e-3);
        // 150.5m MSL => 32918.5 => R=128 (32768), G=150, B=128
        assert!((decode_terrarium_rgb(128, 150, 128) - 150.5).abs() < 0.02);

        let cache = DemTileCache::default();
        let (lat, lon) = (-6.9175, 107.6191);
        let (tx_f, ty_f) = WebMercator::lat_lon_to_tile_coords(lat, lon, 13.0);
        let tx = tx_f.floor() as u32;
        let ty = ty_f.floor() as u32;
        let mut elev_m = vec![0.0_f32; 256 * 256];
        for r in 0..256 {
            for c in 0..256 {
                elev_m[r * 256 + c] = 700.0 + (r as f32) * 0.12 + (c as f32) * 0.05;
            }
        }
        cache.insert_decoded_for_test(
            13,
            tx,
            ty,
            DecodedDemTile {
                width: 256,
                height: 256,
                elev_m,
            },
        );
        let s = cache.sample_elevation_m(lat, lon).expect("Cached DEM tile sample");
        assert!(s > 700.0 && s < 760.0);

        let tin = cache
            .build_site_tin((lat, lon), 240.0, 10)
            .expect("Build DEM TIN");
        assert!(tin.triangles.len() >= 100);
        assert!(tin.max_elevation >= tin.min_elevation);

        let xyz = "0,0,10.0\n50,0,12.5\n50,50,18.0\n0,50,14.0\n25,25,21.0\n";
        let survey_tin = parse_survey_xyz_to_tin(xyz).expect("Parse survey XYZ");
        assert_eq!(survey_tin.points.len(), 5);
        assert_eq!(survey_tin.triangles.len(), 4);
    }
}
