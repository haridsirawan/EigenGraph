//! world_index.rs — Planetary Digital Twin Spatial Registry & Multi-Building HLOD Index (Phase 57 / v0.5.7).
//!
//! Enables EigenGraph to index and render 10,000+ Geo-BIM buildings simultaneously across
//! the Earth 3D globe without loading every project file into memory at once, plus smart
//! placement for unlocated external CAD/IFC models via sidecar `_eigengraph_world_index.json`
//! and an anti-overlap Staging Campus grid.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::map_widget::SlippyMapState;

/// Vault-root spatial manifest persisting all project geo-anchors, headings, and massing hulls.
pub const WORLD_INDEX_FILENAME: &str = "_eigengraph_world_index.json";

/// Dedicated clean urban staging park for bulk imports lacking any GPS or address metadata.
pub const STAGING_CAMPUS_ORIGIN: (f64, f64) = (-6.2285, 106.8110);

/// Angular size of each fine spatial hash cell (~5.5 km at equator) for fast O(visible_cells) queries.
const SITE_CELL_DEG: f64 = 0.05;

/// Spacing in meters between unlocated staging parcels so bulk imports never stack on top of one another.
const STAGING_PARCEL_SPACING_M: f64 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GeoPlacementSource {
    /// Embedded GPS / CRS inside the model (`IFCSITE` lat/lon, `IFCMAPCONVERSION` UTM, or `.eigen` anchor).
    FileNativeGeo,
    /// Inferred automatically from `IFCPOSTALADDRESS`, folder path, or project filename.
    AddressOrFolderHint,
    /// Explicitly placed / relocated interactively on Earth 3D by the user.
    UserPinnedOnEarth,
    /// Assigned an organized anti-overlap parcel inside the Staging Campus until pinned to its real plot.
    #[default]
    StagingCampusAuto,
}

impl GeoPlacementSource {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::FileNativeGeo => "GPS Verified",
            Self::AddressOrFolderHint => "City Hint",
            Self::UserPinnedOnEarth => "Pinned Site",
            Self::StagingCampusAuto => "Staging (Needs Pin)",
        }
    }

    pub fn is_unlocated(self) -> bool {
        matches!(self, Self::StagingCampusAuto)
    }
}

/// Lightweight Geo-BIM proxy (~300 bytes) kept in RAM for each building on Earth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldBuildingProxy {
    pub id: u64,
    pub rel_path: String,
    pub name: String,
    pub filename: String,
    pub lat: f64,
    pub lon: f64,
    pub elev_m: f64,
    pub heading_deg: f64,
    /// Closed/open polygon hull in local tangent meters `[east_m, north_m]` centered around `(0, 0)`.
    pub footprint_local_m: Vec<[f64; 2]>,
    pub total_height_m: f64,
    pub story_count: usize,
    pub member_count: usize,
    pub max_dcr: f64,
    pub min_eigengate: f64,
    #[serde(default)]
    pub source: GeoPlacementSource,
    #[serde(default)]
    pub address_hint: String,
}

impl WorldBuildingProxy {
    /// Deterministic 64-bit FNV-1a ID across WASM and Native targets from `rel_path`.
    pub fn stable_id_for_path(rel_path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in rel_path.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn default_footprint_m() -> Vec<[f64; 2]> {
        vec![[-9.0, -7.0], [9.0, -7.0], [9.0, 7.0], [-9.0, 7.0]]
    }

    /// Returns the polygon rotated by `heading_deg` and converted to tangent millimetres `[east_mm, north_mm]`.
    pub fn rotated_footprint_world_mm(&self) -> Vec<[f64; 2]> {
        let hull = if self.footprint_local_m.len() >= 3 {
            self.footprint_local_m.as_slice()
        } else {
            &[[-9.0, -7.0], [9.0, -7.0], [9.0, 7.0], [-9.0, 7.0]]
        };
        let rad = self.heading_deg.to_radians();
        let (sin_h, cos_h) = (rad.sin(), rad.cos());
        hull.iter()
            .map(|[x, y]| {
                let rx = (x * cos_h - y * sin_h) * 1000.0;
                let ry = (x * sin_h + y * cos_h) * 1000.0;
                [rx, ry]
            })
            .collect()
    }

    pub fn is_unlocated(&self) -> bool {
        self.source.is_unlocated()
    }

    pub fn is_critical(&self) -> bool {
        self.max_dcr >= 0.90 || self.min_eigengate < 0.70
    }

    /// Builds a proxy directly from an in-memory `BimProject` and structural `members` slice.
    pub fn from_bim_project(
        rel_path: &str,
        filename: &str,
        lat: f64,
        lon: f64,
        heading_deg: f64,
        bim: &crate::core::bim::BimProject,
        members: &[crate::models::StructuralMember],
        source: GeoPlacementSource,
        address_hint: String,
    ) -> Self {
        let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut push_xy = |x_mm: f64, y_mm: f64| {
            let xm = x_mm * 0.001;
            let ym = y_mm * 0.001;
            if xm.is_finite() && ym.is_finite() {
                min_x = min_x.min(xm);
                max_x = max_x.max(xm);
                min_y = min_y.min(ym);
                max_y = max_y.max(ym);
            }
        };
        for c in &bim.columns {
            push_xy(c.pos.0 - c.width_mm * 0.5, c.pos.1 - c.depth_mm * 0.5);
            push_xy(c.pos.0 + c.width_mm * 0.5, c.pos.1 + c.depth_mm * 0.5);
        }
        for w in &bim.walls {
            push_xy(w.start.0, w.start.1);
            push_xy(w.end.0, w.end.1);
        }
        for r in &bim.runs {
            push_xy(r.start.0, r.start.1);
            push_xy(r.end.0, r.end.1);
        }
        for s in &bim.slabs {
            for p in &s.boundary {
                push_xy(p.0, p.1);
            }
        }
        let footprint_local_m = if min_x.is_finite() && max_x > min_x + 0.5 && max_y > min_y + 0.5 {
            let cx = 0.5 * (min_x + max_x);
            let cy = 0.5 * (min_y + max_y);
            let hw = ((max_x - min_x) * 0.5).clamp(2.5, 250.0);
            let hd = ((max_y - min_y) * 0.5).clamp(2.5, 250.0);
            let _ = (cx, cy);
            vec![[-hw, -hd], [hw, -hd], [hw, hd], [-hw, hd]]
        } else {
            Self::default_footprint_m()
        };

        let top_elev_m = bim
            .levels
            .iter()
            .map(|l| l.elevation_mm * 0.001)
            .fold(0.0_f64, f64::max);
        let story_count = bim.levels.len().max(1);
        let total_height_m = top_elev_m.max(3.5 * story_count as f64).clamp(3.2, 650.0);

        let max_dcr = members
            .iter()
            .map(|m| m.utilization)
            .fold(0.0_f64, f64::max)
            .clamp(0.0, 2.5);
        let min_eigengate = if members.is_empty() {
            0.92
        } else {
            members.iter().map(|m| m.eigengate).fold(1.0_f64, f64::min).clamp(0.0, 1.0)
        };

        let display_name = if bim.project_name.trim().is_empty() {
            filename.to_string()
        } else {
            bim.project_name.clone()
        };

        Self {
            id: Self::stable_id_for_path(rel_path),
            rel_path: rel_path.to_string(),
            name: display_name,
            filename: filename.to_string(),
            lat,
            lon,
            elev_m: 0.0,
            heading_deg,
            footprint_local_m,
            total_height_m,
            story_count,
            member_count: bim.element_count().max(members.len()),
            max_dcr,
            min_eigengate,
            source,
            address_hint,
        }
    }
}

/// Aggregated planetary/regional building cluster for LOD 0 (`zoom < 8.5`).
#[derive(Debug, Clone)]
pub struct WorldClusterBeacon {
    pub lat: f64,
    pub lon: f64,
    pub count: usize,
    pub unlocated_count: usize,
    pub max_dcr: f64,
    pub min_eigengate: f64,
    pub label: String,
    pub primary_rel_path: String,
}

/// Portfolio triage filter used inside the Vault drawer and Planetary Tour controls (Phase 58).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorldPortfolioFilter {
    #[default]
    All,
    UnlocatedStaging,
    CriticalRisk,
    VerifiedOrPinned,
}

impl WorldPortfolioFilter {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::UnlocatedStaging => "Unlocated",
            Self::CriticalRisk => "Critical",
            Self::VerifiedOrPinned => "Located",
        }
    }
}

/// Aggregated portfolio telemetry summary across all indexed Vault buildings for Gemini AI & HUD.
#[derive(Debug, Clone, Default)]
pub struct WorldPortfolioSummary {
    pub total_buildings: usize,
    pub unlocated_count: usize,
    pub critical_count: usize,
    pub gps_verified_count: usize,
    pub city_hint_count: usize,
    pub user_pinned_count: usize,
    pub top_regions: Vec<(String, usize)>,
    pub top_critical: Vec<WorldBuildingProxy>,
}

/// Spatial hash registry indexing thousands of Geo-BIM buildings for sub-millisecond Earth 3D queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSpatialIndex {
    #[serde(default = "default_world_index_version")]
    pub format_version: String,
    #[serde(default = "default_staging_origin")]
    pub staging_origin_lat_lon: (f64, f64),
    #[serde(default)]
    pub buildings: Vec<WorldBuildingProxy>,
    #[serde(skip)]
    pub active_rel_path: Option<String>,
    #[serde(skip)]
    by_rel_path: HashMap<String, usize>,
    #[serde(skip)]
    site_grid: HashMap<(i32, i32), Vec<usize>>,
}

fn default_world_index_version() -> String {
    "eigengraph-world-index-v1".to_string()
}

fn default_staging_origin() -> (f64, f64) {
    STAGING_CAMPUS_ORIGIN
}

impl Default for WorldSpatialIndex {
    fn default() -> Self {
        Self {
            format_version: default_world_index_version(),
            staging_origin_lat_lon: STAGING_CAMPUS_ORIGIN,
            buildings: Vec::new(),
            active_rel_path: None,
            by_rel_path: HashMap::new(),
            site_grid: HashMap::new(),
        }
    }
}

impl WorldSpatialIndex {
    fn cell_key(lat: f64, lon: f64) -> (i32, i32) {
        let li = (lat.clamp(-85.0, 85.0) / SITE_CELL_DEG).floor() as i32;
        let lo = (Self::normalize_lon(lon) / SITE_CELL_DEG).floor() as i32;
        (li, lo)
    }

    fn normalize_lon(lon: f64) -> f64 {
        let mut l = lon % 360.0;
        if l > 180.0 {
            l -= 360.0;
        } else if l < -180.0 {
            l += 360.0;
        }
        l
    }

    /// Rebuilds the spatial hash cells and path lookup map cleanly in O(N) time.
    pub fn rebuild_spatial_hash(&mut self) {
        self.by_rel_path.clear();
        self.site_grid.clear();
        for (idx, b) in self.buildings.iter().enumerate() {
            self.by_rel_path.insert(b.rel_path.clone(), idx);
            self.site_grid
                .entry(Self::cell_key(b.lat, b.lon))
                .or_default()
                .push(idx);
        }
    }

    pub fn get(&self, rel_path: &str) -> Option<&WorldBuildingProxy> {
        self.by_rel_path
            .get(rel_path)
            .and_then(|&idx| self.buildings.get(idx))
    }

    pub fn unlocated_count(&self) -> usize {
        self.buildings.iter().filter(|b| b.is_unlocated()).count()
    }

    /// Computes non-overlapping parcel `(lat, lon)` inside the Staging Campus grid (`48 m` pitch, 100 columns wide).
    pub fn staging_slot_lat_lon(&self, slot_idx: usize) -> (f64, f64) {
        let col = (slot_idx % 100) as f64;
        let row = (slot_idx / 100) as f64;
        let east_mm = col * STAGING_PARCEL_SPACING_M * 1000.0;
        let north_mm = -row * STAGING_PARCEL_SPACING_M * 1000.0;
        SlippyMapState::world_mm_to_lat_lon(self.staging_origin_lat_lon, [east_mm, north_mm])
    }

    /// Inserts or updates a building proxy, preserving user-pinned GPS coordinates when `preserve_user_pin` is set.
    pub fn upsert_building(&mut self, mut proxy: WorldBuildingProxy, preserve_user_pin: bool) {
        if self.by_rel_path.len() != self.buildings.len() {
            self.rebuild_spatial_hash();
        }
        if let Some(&existing_idx) = self.by_rel_path.get(&proxy.rel_path) {
            let existing = &self.buildings[existing_idx];
            if preserve_user_pin
                && existing.source == GeoPlacementSource::UserPinnedOnEarth
                && !matches!(proxy.source, GeoPlacementSource::UserPinnedOnEarth)
            {
                proxy.lat = existing.lat;
                proxy.lon = existing.lon;
                proxy.elev_m = existing.elev_m;
                proxy.heading_deg = existing.heading_deg;
                proxy.source = GeoPlacementSource::UserPinnedOnEarth;
            } else if preserve_user_pin
                && existing.source == GeoPlacementSource::StagingCampusAuto
                && proxy.source == GeoPlacementSource::StagingCampusAuto
            {
                // Retain stable staging parcel position across Vault scans
                proxy.lat = existing.lat;
                proxy.lon = existing.lon;
            } else if proxy.source == GeoPlacementSource::StagingCampusAuto {
                let slot = self.unlocated_count();
                let (slat, slon) = self.staging_slot_lat_lon(slot);
                proxy.lat = slat;
                proxy.lon = slon;
            }
            if existing.footprint_local_m.len() >= 3
                && proxy.footprint_local_m == WorldBuildingProxy::default_footprint_m()
            {
                proxy.footprint_local_m = existing.footprint_local_m.clone();
                proxy.total_height_m = existing.total_height_m;
            }
            self.buildings[existing_idx] = proxy;
            self.rebuild_spatial_hash();
            return;
        }

        if proxy.source == GeoPlacementSource::StagingCampusAuto {
            let slot = self.unlocated_count();
            let (slat, slon) = self.staging_slot_lat_lon(slot);
            proxy.lat = slat;
            proxy.lon = slon;
        }
        let idx = self.buildings.len();
        let key = Self::cell_key(proxy.lat, proxy.lon);
        self.by_rel_path.insert(proxy.rel_path.clone(), idx);
        self.site_grid.entry(key).or_default().push(idx);
        self.buildings.push(proxy);
    }

    /// Permanently anchors or relocates a Vault building onto real-world coordinates on Earth 3D.
    pub fn pin_building_at(&mut self, rel_path: &str, lat: f64, lon: f64, heading_deg: f64) -> bool {
        if self.by_rel_path.len() != self.buildings.len() {
            self.rebuild_spatial_hash();
        }
        let Some(&idx) = self.by_rel_path.get(rel_path) else {
            return false;
        };
        if let Some(b) = self.buildings.get_mut(idx) {
            b.lat = lat.clamp(-85.0, 85.0);
            b.lon = Self::normalize_lon(lon);
            b.heading_deg = heading_deg.rem_euclid(360.0);
            b.source = GeoPlacementSource::UserPinnedOnEarth;
            self.rebuild_spatial_hash();
            true
        } else {
            false
        }
    }

    /// Updates building True North azimuth heading (degrees) non-destructively in the world index.
    pub fn update_heading(&mut self, rel_path: &str, heading_deg: f64) -> bool {
        if self.by_rel_path.len() != self.buildings.len() {
            self.rebuild_spatial_hash();
        }
        let Some(&idx) = self.by_rel_path.get(rel_path) else {
            return false;
        };
        if let Some(b) = self.buildings.get_mut(idx) {
            b.heading_deg = heading_deg.rem_euclid(360.0);
            true
        } else {
            false
        }
    }

    pub fn matches_filter(b: &WorldBuildingProxy, filter: WorldPortfolioFilter) -> bool {
        match filter {
            WorldPortfolioFilter::All => true,
            WorldPortfolioFilter::UnlocatedStaging => b.is_unlocated(),
            WorldPortfolioFilter::CriticalRisk => b.is_critical(),
            WorldPortfolioFilter::VerifiedOrPinned => !b.is_unlocated(),
        }
    }

    /// Returns indices of buildings matching `filter` and optional text query (name, path, or city hint).
    pub fn filtered_indices(&self, filter: WorldPortfolioFilter, query: &str) -> Vec<usize> {
        let fq = query.trim().to_ascii_lowercase();
        self.buildings
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                Self::matches_filter(b, filter)
                    && (fq.is_empty()
                        || b.name.to_ascii_lowercase().contains(&fq)
                        || b.rel_path.to_ascii_lowercase().contains(&fq)
                        || b.address_hint.to_ascii_lowercase().contains(&fq))
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Cycles cleanly to the next building matching `filter` after `after_rel` (ordered by risk when `CriticalRisk`).
    pub fn next_building_matching(
        &self,
        filter: WorldPortfolioFilter,
        after_rel: Option<&str>,
    ) -> Option<&WorldBuildingProxy> {
        let mut indices = self.filtered_indices(filter, "");
        if indices.is_empty() {
            return None;
        }
        if filter == WorldPortfolioFilter::CriticalRisk {
            indices.sort_by(|&a, &b| {
                let ba = &self.buildings[a];
                let bb = &self.buildings[b];
                bb.max_dcr
                    .partial_cmp(&ba.max_dcr)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| ba.min_eigengate.partial_cmp(&bb.min_eigengate).unwrap_or(std::cmp::Ordering::Equal))
            });
        }
        if let Some(cur) = after_rel {
            if let Some(pos) = indices.iter().position(|&idx| self.buildings[idx].rel_path == cur) {
                let next_idx = indices[(pos + 1) % indices.len()];
                return self.buildings.get(next_idx);
            }
        }
        self.buildings.get(indices[0])
    }

    /// Summarizes global Vault portfolio counts, regional distribution, and top critical buildings.
    pub fn portfolio_summary(&self) -> WorldPortfolioSummary {
        let mut s = WorldPortfolioSummary {
            total_buildings: self.buildings.len(),
            ..Default::default()
        };
        let mut region_counts: HashMap<String, usize> = HashMap::new();
        let mut crit_indices = Vec::new();
        for (i, b) in self.buildings.iter().enumerate() {
            match b.source {
                GeoPlacementSource::FileNativeGeo => s.gps_verified_count += 1,
                GeoPlacementSource::AddressOrFolderHint => s.city_hint_count += 1,
                GeoPlacementSource::UserPinnedOnEarth => s.user_pinned_count += 1,
                GeoPlacementSource::StagingCampusAuto => s.unlocated_count += 1,
            }
            if b.is_critical() {
                s.critical_count += 1;
                crit_indices.push(i);
            }
            let region = if !b.address_hint.trim().is_empty() {
                b.address_hint.trim().to_string()
            } else if b.is_unlocated() {
                "Staging Campus".to_string()
            } else {
                "Pinned Custom".to_string()
            };
            *region_counts.entry(region).or_default() += 1;
        }
        let mut regions: Vec<(String, usize)> = region_counts.into_iter().collect();
        regions.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        regions.truncate(6);
        s.top_regions = regions;

        crit_indices.sort_by(|&a, &b| {
            self.buildings[b]
                .max_dcr
                .partial_cmp(&self.buildings[a].max_dcr)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crit_indices.truncate(5);
        s.top_critical = crit_indices.into_iter().map(|i| self.buildings[i].clone()).collect();
        s
    }

    /// Batch-pins staging models whose path, name, or hint matches `query` onto `(lat, lon)` with small grid jitter.
    pub fn relocate_city_batch(&mut self, query: &str, lat: f64, lon: f64, city_label: &str) -> usize {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return 0;
        }
        let mut moved = 0usize;
        let lat_clamped = lat.clamp(-85.0, 85.0);
        let lon_norm = Self::normalize_lon(lon);
        for b in &mut self.buildings {
            if b.is_unlocated()
                && (b.rel_path.to_ascii_lowercase().contains(&q)
                    || b.name.to_ascii_lowercase().contains(&q)
                    || b.address_hint.to_ascii_lowercase().contains(&q))
            {
                let col = (moved % 25) as f64;
                let row = (moved / 25) as f64;
                let off = SlippyMapState::world_mm_to_lat_lon(
                    (lat_clamped, lon_norm),
                    [col * 45_000.0, -row * 45_000.0],
                );
                b.lat = off.0;
                b.lon = off.1;
                b.source = GeoPlacementSource::UserPinnedOnEarth;
                if b.address_hint.is_empty() && !city_label.is_empty() {
                    b.address_hint = city_label.to_string();
                }
                moved += 1;
            }
        }
        if moved > 0 {
            self.rebuild_spatial_hash();
        }
        moved
    }

    /// Queries visible building indices inside the given geographic viewport bbox sorted closest to focus.
    pub fn query_visible_indices(
        &self,
        min_lat: f64,
        max_lat: f64,
        min_lon: f64,
        max_lon: f64,
        center_lat: f64,
        center_lon: f64,
        max_count: usize,
    ) -> Vec<usize> {
        if self.buildings.is_empty() || max_count == 0 {
            return Vec::new();
        }
        let c_lat0 = (min_lat.clamp(-85.0, 85.0) / SITE_CELL_DEG).floor() as i32 - 1;
        let c_lat1 = (max_lat.clamp(-85.0, 85.0) / SITE_CELL_DEG).ceil() as i32 + 1;
        let lon_span = (max_lon - min_lon).abs();
        let wraps = lon_span >= 359.0;
        let mut hits: Vec<(usize, f64)> = Vec::new();

        if wraps || ((c_lat1 - c_lat0) as i64 * ((lon_span / SITE_CELL_DEG) as i64 + 3) > 2500) {
            // Very wide view: iterate HashMap keys (populated cells only, far fewer than full cross-product)
            for (&(cli, _clo), list) in &self.site_grid {
                if cli < c_lat0 || cli > c_lat1 {
                    continue;
                }
                for &idx in list {
                    let b = &self.buildings[idx];
                    if b.lat < min_lat || b.lat > max_lat {
                        continue;
                    }
                    let d = (b.lat - center_lat).powi(2) + (b.lon - center_lon).powi(2);
                    hits.push((idx, d));
                }
            }
        } else {
            let c_lon0 = (min_lon / SITE_CELL_DEG).floor() as i32 - 1;
            let c_lon1 = (max_lon / SITE_CELL_DEG).ceil() as i32 + 1;
            let n_lon_cells = (360.0 / SITE_CELL_DEG).round() as i32;
            for cli in c_lat0..=c_lat1 {
                for raw_clo in c_lon0..=c_lon1 {
                    let clo = raw_clo.rem_euclid(n_lon_cells)
                        - if raw_clo.rem_euclid(n_lon_cells) >= n_lon_cells / 2 {
                            n_lon_cells
                        } else {
                            0
                        };
                    if let Some(list) = self.site_grid.get(&(cli, clo)) {
                        for &idx in list {
                            let b = &self.buildings[idx];
                            if b.lat >= min_lat && b.lat <= max_lat {
                                let mut dl = b.lon - center_lon;
                                if dl > 180.0 {
                                    dl -= 360.0;
                                } else if dl < -180.0 {
                                    dl += 360.0;
                                }
                                hits.push((idx, (b.lat - center_lat).powi(2) + dl * dl));
                            }
                        }
                    }
                }
            }
        }

        if hits.len() > max_count {
            hits.select_nth_unstable_by(max_count, |a, b| {
                a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            hits.truncate(max_count);
        }
        hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        hits.into_iter().map(|(idx, _)| idx).collect()
    }

    /// Aggregates buildings into regional cluster beacons for orbital globe scales (`zoom < 8.5`).
    pub fn query_clusters(
        &self,
        min_lat: f64,
        max_lat: f64,
        min_lon: f64,
        max_lon: f64,
        zoom_level: f32,
    ) -> Vec<WorldClusterBeacon> {
        if self.buildings.is_empty() {
            return Vec::new();
        }
        let cell_deg = if zoom_level < 3.0 {
            12.0
        } else if zoom_level < 5.0 {
            5.0
        } else if zoom_level < 7.0 {
            1.5
        } else {
            0.45
        };

        struct Accum {
            sum_lat: f64,
            sum_lon: f64,
            count: usize,
            unlocated_count: usize,
            max_dcr: f64,
            min_eigengate: f64,
            first_name: String,
            first_path: String,
            hint: String,
        }

        let mut map: HashMap<(i32, i32), Accum> = HashMap::new();
        let span_full = (max_lon - min_lon).abs() >= 359.0;
        for b in &self.buildings {
            if b.lat < min_lat || b.lat > max_lat {
                continue;
            }
            if !span_full && (b.lon < min_lon || b.lon > max_lon) {
                continue;
            }
            let key = (
                (b.lat / cell_deg).floor() as i32,
                (b.lon / cell_deg).floor() as i32,
            );
            let entry = map.entry(key).or_insert_with(|| Accum {
                sum_lat: 0.0,
                sum_lon: 0.0,
                count: 0,
                unlocated_count: 0,
                max_dcr: 0.0,
                min_eigengate: 1.0,
                first_name: b.name.clone(),
                first_path: b.rel_path.clone(),
                hint: b.address_hint.clone(),
            });
            entry.sum_lat += b.lat;
            entry.sum_lon += b.lon;
            entry.count += 1;
            if b.is_unlocated() {
                entry.unlocated_count += 1;
            }
            entry.max_dcr = entry.max_dcr.max(b.max_dcr);
            entry.min_eigengate = entry.min_eigengate.min(b.min_eigengate);
            if entry.hint.is_empty() && !b.address_hint.is_empty() {
                entry.hint = b.address_hint.clone();
            }
        }

        map.into_values()
            .map(|a| {
                let n = a.count.max(1) as f64;
                let label = if a.count == 1 {
                    a.first_name
                } else if !a.hint.is_empty() {
                    format!("{} • {} Bldgs", a.hint, a.count)
                } else if a.unlocated_count == a.count {
                    format!("Staging Park • {} Bldgs", a.count)
                } else {
                    format!("{} Buildings", a.count)
                };
                WorldClusterBeacon {
                    lat: a.sum_lat / n,
                    lon: a.sum_lon / n,
                    count: a.count,
                    unlocated_count: a.unlocated_count,
                    max_dcr: a.max_dcr,
                    min_eigengate: a.min_eigengate,
                    label,
                    primary_rel_path: a.first_path,
                }
            })
            .collect()
    }

    pub fn to_manifest_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("WorldSpatialIndex serialize failed: {e}"))
    }

    pub fn from_manifest_json(raw: &str) -> Result<Self, String> {
        let mut idx: Self = serde_json::from_str(raw)
            .map_err(|e| format!("WorldSpatialIndex deserialize failed: {e}"))?;
        idx.rebuild_spatial_hash();
        Ok(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase57_world_spatial_index_10k_buildings_clustering_and_staging_slots() {
        let mut index = WorldSpatialIndex::default();

        // Populate 10,000 buildings across Jakarta, IKN Nusantara, Tokyo, plus 100 unlocated Staging buildings
        for i in 0..10_000_usize {
            let rel = format!("Projects/Batch_{}/Bldg_{i:05}.ifc", i / 100);
            let (lat, lon, src, hint) = if i < 100 {
                (0.0, 0.0, GeoPlacementSource::StagingCampusAuto, String::new())
            } else if i % 3 == 0 {
                let d = (i % 60) as f64 * 0.0012;
                let r = ((i / 60) % 50) as f64 * 0.0012;
                (-6.2088 + d, 106.8456 + r, GeoPlacementSource::FileNativeGeo, "Jakarta".to_string())
            } else if i % 3 == 1 {
                let d = (i % 50) as f64 * 0.0011;
                let r = ((i / 50) % 45) as f64 * 0.0011;
                (-0.9730 + d, 116.7090 + r, GeoPlacementSource::AddressOrFolderHint, "Nusantara IKN".to_string())
            } else {
                let d = (i % 50) as f64 * 0.001;
                let r = ((i / 50) % 45) as f64 * 0.001;
                (35.6762 + d, 139.6503 + r, GeoPlacementSource::FileNativeGeo, "Tokyo".to_string())
            };
            index.upsert_building(
                WorldBuildingProxy {
                    id: WorldBuildingProxy::stable_id_for_path(&rel),
                    rel_path: rel,
                    name: format!("Tower_{i:05}"),
                    filename: format!("Bldg_{i:05}.ifc"),
                    lat,
                    lon,
                    elev_m: 0.0,
                    heading_deg: 0.0,
                    footprint_local_m: WorldBuildingProxy::default_footprint_m(),
                    total_height_m: 18.0 + (i % 20) as f64 * 3.0,
                    story_count: 5 + (i % 20),
                    member_count: 120,
                    max_dcr: 0.45 + (i % 5) as f64 * 0.12,
                    min_eigengate: 0.92 - (i % 4) as f64 * 0.08,
                    source: src,
                    address_hint: hint,
                },
                true,
            );
        }

        assert_eq!(index.buildings.len(), 10_000);
        assert_eq!(index.unlocated_count(), 100);

        // 1. All 100 Staging buildings receive distinct non-overlapping parcels (zero ocean (0,0) stacking!)
        let b0 = &index.buildings[0];
        let b1 = &index.buildings[1];
        assert!(b0.lat.abs() > 1.0 && b0.lon.abs() > 10.0);
        assert!((b0.lon - b1.lon).abs() > 1e-4, "Staging parcels must be spaced cleanly apart");

        // 2. LOD 0 clusters reduce 10,000 buildings into compact regional beacons
        let clusters = index.query_clusters(-85.0, 85.0, -180.0, 180.0, 3.0);
        assert!(clusters.len() >= 3 && clusters.len() <= 20);
        assert_eq!(clusters.iter().map(|c| c.count).sum::<usize>(), 10_000);

        // 3. LOD 1 viewport query near Jakarta returns bounded subset instantaneously without touching Tokyo
        let vis_jkt = index.query_visible_indices(-6.26, -6.14, 106.80, 106.92, -6.2088, 106.8456, 250);
        assert!(!vis_jkt.is_empty() && vis_jkt.len() <= 250);
        assert!(vis_jkt.iter().all(|&idx| (index.buildings[idx].lat - (-6.2088)).abs() < 0.1));

        // 4. Explicit "Place on Earth" pinning moves Staging building permanently & survives upsert refresh
        let staging_path = index.buildings[0].rel_path.clone();
        assert!(index.pin_building_at(&staging_path, -7.2575, 112.7521, 30.0));
        assert_eq!(index.unlocated_count(), 99);
        assert_eq!(index.get(&staging_path).unwrap().source, GeoPlacementSource::UserPinnedOnEarth);

        let mut incoming_rescan = index.get(&staging_path).unwrap().clone();
        incoming_rescan.lat = 0.0;
        incoming_rescan.lon = 0.0;
        incoming_rescan.source = GeoPlacementSource::StagingCampusAuto;
        index.upsert_building(incoming_rescan, true);
        let kept = index.get(&staging_path).unwrap();
        assert_eq!(kept.source, GeoPlacementSource::UserPinnedOnEarth);
        assert!((kept.lat - (-7.2575)).abs() < 1e-6);

        // 5. Manifest serialization roundtrip preserves spatial hash & coordinates
        let json = index.to_manifest_json().expect("serialize world manifest");
        let loaded = WorldSpatialIndex::from_manifest_json(&json).expect("deserialize world manifest");
        assert_eq!(loaded.buildings.len(), 10_000);
        assert!((loaded.get(&staging_path).unwrap().heading_deg - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_phase58_world_index_filtering_cycle_summary_and_heading_update() {
        let mut idx = WorldSpatialIndex::default();
        for (i, (name, path, dcr, eigengate, src, city)) in [
            ("LowRisk", "Projects/Jakarta/A.ifc", 0.55, 0.94, GeoPlacementSource::FileNativeGeo, "Jakarta"),
            ("HighDcr", "Projects/Bandung/B.ifc", 1.12, 0.75, GeoPlacementSource::AddressOrFolderHint, "Bandung"),
            ("StagedOne", "Projects/Bandung/C.ifc", 0.92, 0.62, GeoPlacementSource::StagingCampusAuto, ""),
        ]
        .into_iter()
        .enumerate()
        {
            idx.upsert_building(
                WorldBuildingProxy {
                    id: WorldBuildingProxy::stable_id_for_path(path),
                    rel_path: path.to_string(),
                    name: name.to_string(),
                    filename: format!("{name}.ifc"),
                    lat: -6.2 - i as f64 * 0.1,
                    lon: 106.8 + i as f64 * 0.1,
                    elev_m: 15.0,
                    heading_deg: 0.0,
                    footprint_local_m: WorldBuildingProxy::default_footprint_m(),
                    total_height_m: 20.0,
                    story_count: 5,
                    member_count: 40,
                    max_dcr: dcr,
                    min_eigengate: eigengate,
                    source: src,
                    address_hint: city.to_string(),
                },
                true,
            );
        }

        let summary = idx.portfolio_summary();
        assert_eq!(summary.total_buildings, 3);
        assert_eq!(summary.unlocated_count, 1);
        assert_eq!(summary.critical_count, 2);
        assert_eq!(summary.top_critical[0].name, "HighDcr");

        let next_crit = idx.next_building_matching(WorldPortfolioFilter::CriticalRisk, None).unwrap();
        assert_eq!(next_crit.name, "HighDcr");
        let after_crit = idx
            .next_building_matching(WorldPortfolioFilter::CriticalRisk, Some(&next_crit.rel_path))
            .unwrap();
        assert_eq!(after_crit.name, "StagedOne");

        assert!(idx.update_heading("Projects/Jakarta/A.ifc", -15.0));
        assert!((idx.get("Projects/Jakarta/A.ifc").unwrap().heading_deg - 345.0).abs() < 1e-6);

        let moved = idx.relocate_city_batch("Bandung", -6.9175, 107.6191, "Bandung");
        assert_eq!(moved, 1);
        assert_eq!(idx.unlocated_count(), 0);
    }
}
