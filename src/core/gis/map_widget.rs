//! map_widget.rs — Pure-Rust Vector Map & Web Mercator Geodetic Engine (Phase 64 / v0.6.4).
//!
//! Provides mathematically rigorous Web Mercator (EPSG:3857) projection, geodetic site pinning,
//! smooth continuous panning/zooming, and 100% pure-Rust vector cartography with zero external
//! raster tiles or image decoding dependencies.

use std::f64::consts::PI;
use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Stroke, Ui, Vec2};

pub use super::world_vectors::{
    TileSource, VectorEarthPalette, VectorEarthTheme, WORLD_LANDMASSES, WORLD_MAJOR_RIVERS,
    WORLD_MOUNTAIN_RIDGES,
};

/// WGS84 Earth semi-major axis radius in meters (EPSG:3857 / WGS84 ellipsoid spherical approximation).
pub const EARTH_RADIUS_METERS: f64 = 6378137.0;

/// Half circumference of the Earth in Web Mercator projection (meters).
pub const EARTH_HALF_CIRCUMFERENCE: f64 = PI * EARTH_RADIUS_METERS; // ≈ 20,037,508.34 m

/// Maximum valid latitude for Web Mercator projection to avoid asymptotic infinity: ~85.05112878°.
pub const MAX_LATITUDE: f64 = 85.0511287798066;
pub const MIN_LATITUDE: f64 = -85.0511287798066;

/// Curated global engineering cities & infrastructure hubs shared across geocoding and World Spatial Indexing.
pub const GLOBAL_CITY_CATALOG: &[(&str, f64, f64, f32)] = &[
    ("Jakarta", -6.2088, 106.8456, 15.0),
    ("Bandung", -6.9175, 107.6191, 15.0),
    ("Surabaya", -7.2575, 112.7521, 15.0),
    ("Yogyakarta", -7.7956, 110.3695, 15.0),
    ("Semarang", -6.9667, 110.4167, 15.0),
    ("Medan", 3.5952, 98.6722, 15.0),
    ("Makassar", -5.1477, 119.4327, 15.0),
    ("Balikpapan", -1.2379, 116.8529, 15.0),
    ("Palu", -0.8917, 119.8707, 15.0),
    ("Nusantara IKN", -0.9730, 116.7090, 15.0),
    ("IKN", -0.9730, 116.7090, 15.0),
    ("Bali Denpasar", -8.6705, 115.2126, 15.0),
    ("Singapore", 1.3521, 103.8198, 15.0),
    ("Tokyo", 35.6762, 139.6503, 15.0),
    ("Osaka", 34.6937, 135.5023, 15.0),
    ("Seoul", 37.5665, 126.9780, 15.0),
    ("Taipei", 25.0330, 121.5654, 15.0),
    ("Kuala Lumpur", 3.1390, 101.6869, 15.0),
    ("Bangkok", 13.7563, 100.5018, 15.0),
    ("Manila", 14.5995, 120.9842, 15.0),
    ("San Francisco", 37.7749, -122.4194, 15.0),
    ("Los Angeles", 34.0522, -118.2437, 15.0),
    ("Seattle", 47.6062, -122.3321, 15.0),
    ("New York", 40.7128, -74.0060, 15.0),
    ("Chicago", 41.8781, -87.6298, 15.0),
    ("London", 51.5072, -0.1276, 15.0),
    ("Zurich", 47.3769, 8.5417, 15.0),
    ("Paris", 48.8566, 2.3522, 15.0),
    ("Berlin", 52.5200, 13.4050, 15.0),
    ("Istanbul", 41.0082, 28.9784, 15.0),
    ("Dubai", 25.2048, 55.2708, 15.0),
    ("Riyadh", 24.7136, 46.6753, 15.0),
    ("Sydney", -33.8688, 151.2093, 15.0),
    ("Melbourne", -37.8136, 144.9631, 15.0),
    ("Auckland", -36.8509, 174.7645, 15.0),
    ("Christchurch", -43.5320, 172.6306, 15.0),
    ("Santiago", -33.4489, -70.6693, 15.0),
    ("Mexico City", 19.4326, -99.1332, 15.0),
    ("Mumbai", 19.0760, 72.8777, 15.0),
    ("Cairo", 30.0444, 31.2357, 15.0),
];

/// Triangulates a simple concave 2D screen polygon via ear-clipping for clean continent landmass fills.
pub fn ear_clip_polygon(pts: &[Pos2], fill_color: Color32) -> Option<egui::epaint::Mesh> {
    triangulate_polygon_2d(pts, fill_color)
}

fn triangulate_polygon_2d(pts: &[Pos2], fill_color: Color32) -> Option<egui::epaint::Mesh> {
    if pts.len() < 3 {
        return None;
    }
    let mut ring: Vec<Pos2> = pts.to_vec();
    if ring.len() >= 2 && (ring[0] - ring[ring.len() - 1]).length_sq() < 1e-4 {
        ring.pop();
    }
    if ring.len() < 3 {
        return None;
    }

    // Signed area (screen coordinates: +y down)
    let mut area2 = 0.0_f32;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        area2 += a.x * b.y - b.x * a.y;
    }
    if area2.abs() < 1e-3 {
        return None;
    }
    if area2 < 0.0 {
        ring.reverse();
    }

    let mut mesh = egui::epaint::Mesh::default();
    for p in &ring {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: *p,
            uv: egui::epaint::WHITE_UV,
            color: fill_color,
        });
    }

    let mut rem: Vec<u32> = (0..ring.len() as u32).collect();
    let cross = |a: Pos2, b: Pos2, c: Pos2| -> f32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    };
    let point_in_tri = |p: Pos2, a: Pos2, b: Pos2, c: Pos2| -> bool {
        let c1 = cross(a, b, p);
        let c2 = cross(b, c, p);
        let c3 = cross(c, a, p);
        c1 >= -1e-4 && c2 >= -1e-4 && c3 >= -1e-4
    };

    let mut guard = 0usize;
    let max_iters = ring.len() * ring.len() + 8;
    while rem.len() > 3 && guard < max_iters {
        guard += 1;
        let m = rem.len();
        let mut clipped = false;
        for i in 0..m {
            let i0 = rem[(i + m - 1) % m] as usize;
            let i1 = rem[i] as usize;
            let i2 = rem[(i + 1) % m] as usize;
            let (a, b, c) = (ring[i0], ring[i1], ring[i2]);
            if cross(a, b, c) <= 1e-4 {
                continue;
            }
            let any_inside = rem.iter().enumerate().any(|(k, &idx_u)| {
                let idx = idx_u as usize;
                idx != i0 && idx != i1 && idx != i2 && {
                    let _ = k;
                    point_in_tri(ring[idx], a, b, c)
                }
            });
            if !any_inside {
                mesh.indices.extend_from_slice(&[i0 as u32, i1 as u32, i2 as u32]);
                rem.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            break;
        }
    }
    if rem.len() == 3 {
        mesh.indices.extend_from_slice(&[rem[0], rem[1], rem[2]]);
    }
    if mesh.indices.is_empty() {
        None
    } else {
        Some(mesh)
    }
}

/// A pinned geodetic site location for BIM project placement, soil investigation, and seismic zoning.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SiteGeodeticPin {
    pub id: usize,
    pub label: String,
    pub lat: f64,
    pub lon: f64,
    pub elevation_m: f32,
    pub project_name: String,
    pub soil_class: String,
    pub ss_g: f32,
    pub s1_g: f32,
    pub notes: String,
}

/// Mathematical Web Mercator (EPSG:3857) and Slippy Map Quadtree Projection Routines.
pub struct WebMercator;

impl WebMercator {
    /// Clamps latitude to valid Web Mercator bounds [-85.051129°, +85.051129°].
    #[inline]
    pub fn clamp_lat(lat: f64) -> f64 {
        lat.clamp(MIN_LATITUDE, MAX_LATITUDE)
    }

    /// Normalizes longitude to [-180.0°, +180.0°].
    #[inline]
    pub fn normalize_lon(lon: f64) -> f64 {
        let mut l = lon % 360.0;
        if l > 180.0 {
            l -= 360.0;
        } else if l < -180.0 {
            l += 360.0;
        }
        l
    }

    /// Converts WGS84 Geodetic coordinates (lat, lon) in degrees to Projected Web Mercator (x, y) in meters.
    pub fn lat_lon_to_meters(lat: f64, lon: f64) -> (f64, f64) {
        let lat = Self::clamp_lat(lat);
        let lon = Self::normalize_lon(lon);

        let x = lon.to_radians() * EARTH_RADIUS_METERS;
        let lat_rad = lat.to_radians();
        let y = EARTH_RADIUS_METERS * ((PI / 4.0 + lat_rad / 2.0).tan().ln());
        (x, y)
    }

    /// Converts Projected Web Mercator meters (x, y) back to WGS84 Geodetic (lat, lon) in degrees.
    pub fn meters_to_lat_lon(x: f64, y: f64) -> (f64, f64) {
        let lon = (x / EARTH_RADIUS_METERS).to_degrees();
        let lat = (2.0 * (y / EARTH_RADIUS_METERS).exp().atan() - PI / 2.0).to_degrees();
        (Self::clamp_lat(lat), Self::normalize_lon(lon))
    }

    /// Converts WGS84 Geodetic (lat, lon) to continuous floating-point tile coordinates (X, Y) at zoom level z.
    pub fn lat_lon_to_tile_coords(lat: f64, lon: f64, zoom: f32) -> (f64, f64) {
        let lat = Self::clamp_lat(lat);
        let lon = Self::normalize_lon(lon);
        let n = 2.0_f64.powf(zoom as f64);

        let x = (lon + 180.0) / 360.0 * n;
        let lat_rad = lat.to_radians();
        let y = (1.0 - ((lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI)) / 2.0 * n;
        (x, y)
    }

    /// Converts floating-point tile coordinates (X, Y) at zoom level z back to WGS84 Geodetic (lat, lon).
    pub fn tile_coords_to_lat_lon(x: f64, y: f64, zoom: f32) -> (f64, f64) {
        let n = 2.0_f64.powf(zoom as f64);
        let lon = x / n * 360.0 - 180.0;
        let sinh_val = (PI * (1.0 - 2.0 * y / n)).sinh();
        let lat = sinh_val.atan().to_degrees();
        (Self::clamp_lat(lat), Self::normalize_lon(lon))
    }

    /// Calculates ground resolution in meters per pixel at a given latitude and zoom level (assuming 256px tiles).
    pub fn ground_resolution_m_px(lat: f64, zoom: f32) -> f64 {
        let lat = Self::clamp_lat(lat);
        (156543.03392 * lat.to_radians().cos()) / 2.0_f64.powf(zoom as f64)
    }

    /// Calculates the integer discrete tile index (tile_x, tile_y, zoom_int) for tile fetching.
    pub fn get_tile_index(lat: f64, lon: f64, zoom_int: u8) -> (u32, u32, u8) {
        let (tx, ty) = Self::lat_lon_to_tile_coords(lat, lon, zoom_int as f32);
        let max_tile = (1u64 << zoom_int).saturating_sub(1) as u32;
        let x = (tx.floor() as i64).clamp(0, max_tile as i64) as u32;
        let y = (ty.floor() as i64).clamp(0, max_tile as i64) as u32;
        (x, y, zoom_int)
    }
}

/// Real-time HUD Telemetry information for inspection.
#[derive(Debug, Clone)]
pub struct SlippyMapTelemetry {
    pub lat: f64,
    pub lon: f64,
    pub mercator_x: f64,
    pub mercator_y: f64,
    pub zoom: f32,
    pub tile_x: u32,
    pub tile_y: u32,
    pub tile_z: u8,
    pub ground_resolution_m_px: f64,
    pub pins_count: usize,
}

/// Interactive Pure-Vector Map State Container for Pillar 1D & Earth Vector 3D.
#[derive(Debug, Clone)]
pub struct SlippyMapState {
    pub center_lat_lon: (f64, f64),
    pub zoom_level: f32, // Continuous zoom 1.0 to 19.0
    pub tile_provider: VectorEarthTheme,
    pub vector_site_overlay: bool,
    pub active_pin: Option<(f64, f64)>,
    pub site_pins: Vec<SiteGeodeticPin>,
    pub crosshair_active: bool,
    pub show_grid: bool,
    pub show_scale_bar: bool,
    pub show_elevation_contours: bool,
    pub is_dragging: bool,
    pub drag_start_mouse: Option<Pos2>,
    pub drag_start_center: (f64, f64),
    pub next_pin_id: usize,
}

impl Default for SlippyMapState {
    fn default() -> Self {
        Self {
            // Phase 64: Pure-Rust Vector Earth default centered on the Indonesian Archipelago & Asia-Pacific
            center_lat_lon: (-2.0, 114.0),
            zoom_level: 2.15,
            tile_provider: VectorEarthTheme::CoastalBlueprint,
            vector_site_overlay: true,
            active_pin: None,
            site_pins: Vec::new(),
            crosshair_active: true,
            show_grid: true,
            show_scale_bar: true,
            show_elevation_contours: false,
            is_dragging: false,
            drag_start_mouse: None,
            drag_start_center: (-2.0, 114.0),
            next_pin_id: 1,
        }
    }
}

impl SlippyMapState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Short mode label describing the active pure-vector Earth cartography theme.
    pub fn visual_layer_label(&self) -> &'static str {
        self.tile_provider.short_name()
    }

    /// Cycles cleanly through the four built-in Pure-Rust Vector Earth themes:
    /// `CoastalBlueprint` -> `CityDaylight` -> `NightTwin` -> `Topographic`.
    pub fn cycle_basemap_visual_mode(&mut self) {
        self.tile_provider = self.tile_provider.cycle_next();
        self.vector_site_overlay = true;
    }

    /// Pans the map center by delta pixel offset on screen.
    pub fn pan_by_pixels(&mut self, dx: f32, dy: f32) {
        let (center_x, center_y) = WebMercator::lat_lon_to_tile_coords(
            self.center_lat_lon.0,
            self.center_lat_lon.1,
            self.zoom_level,
        );
        // 256 pixels per tile
        let new_tile_x = center_x - (dx as f64) / 256.0;
        let new_tile_y = center_y - (dy as f64) / 256.0;
        self.center_lat_lon = WebMercator::tile_coords_to_lat_lon(new_tile_x, new_tile_y, self.zoom_level);
    }

    /// Zooms in smoothly by delta.
    pub fn zoom_in(&mut self, delta: f32) {
        self.zoom_level = (self.zoom_level + delta).clamp(1.0, 19.0);
    }

    /// Zooms out smoothly by delta.
    pub fn zoom_out(&mut self, delta: f32) {
        self.zoom_level = (self.zoom_level - delta).clamp(1.0, 19.0);
    }

    /// Sets the map center to specific geodetic coordinates.
    pub fn set_center(&mut self, lat: f64, lon: f64) {
        self.center_lat_lon = (WebMercator::clamp_lat(lat), WebMercator::normalize_lon(lon));
    }

    /// Converts screen position within viewport rect to WGS84 Geodetic coordinates (lat, lon).
    pub fn screen_to_lat_lon(&self, pos: Pos2, rect: Rect) -> (f64, f64) {
        let center_pos = rect.center();
        let dx = pos.x - center_pos.x;
        let dy = pos.y - center_pos.y;

        let (center_x, center_y) = WebMercator::lat_lon_to_tile_coords(
            self.center_lat_lon.0,
            self.center_lat_lon.1,
            self.zoom_level,
        );
        let tile_x = center_x + (dx as f64) / 256.0;
        let tile_y = center_y + (dy as f64) / 256.0;

        WebMercator::tile_coords_to_lat_lon(tile_x, tile_y, self.zoom_level)
    }

    /// Converts WGS84 Geodetic coordinates (lat, lon) to screen pixel position within viewport rect.
    pub fn lat_lon_to_screen(&self, lat: f64, lon: f64, rect: Rect) -> Pos2 {
        let (center_x, center_y) = WebMercator::lat_lon_to_tile_coords(
            self.center_lat_lon.0,
            self.center_lat_lon.1,
            self.zoom_level,
        );
        let (target_x, target_y) = WebMercator::lat_lon_to_tile_coords(lat, lon, self.zoom_level);
        let world_tiles = 2.0_f64.powf(self.zoom_level as f64);

        let mut delta_x = target_x - center_x;
        if delta_x > world_tiles * 0.5 {
            delta_x -= world_tiles;
        } else if delta_x < -world_tiles * 0.5 {
            delta_x += world_tiles;
        }

        let dx = (delta_x * 256.0) as f32;
        let dy = ((target_y - center_y) * 256.0) as f32;

        Pos2::new(rect.center().x + dx, rect.center().y + dy)
    }

    /// Converts explicit unwrapped longitude `(lat, lon_deg)` to screen pixel position
    /// for continent boundary polygons spanning `-180..=180` across wrap shifts.
    fn lat_lon_to_screen_unwrapped(&self, lat: f64, lon_unwrapped: f64, rect: Rect) -> Pos2 {
        let (center_x, center_y) = WebMercator::lat_lon_to_tile_coords(
            self.center_lat_lon.0,
            self.center_lat_lon.1,
            self.zoom_level,
        );
        let clamped_lat = WebMercator::clamp_lat(lat);
        let n = 2.0_f64.powf(self.zoom_level as f64);
        let target_x = ((lon_unwrapped + 180.0) / 360.0) * n;
        let lat_rad = clamped_lat.to_radians();
        let target_y = (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI) * 0.5 * n;

        let dx = ((target_x - center_x) * 256.0) as f32;
        let dy = ((target_y - center_y) * 256.0) as f32;
        Pos2::new(rect.center().x + dx, rect.center().y + dy)
    }

    /// Returns 2D Mercator pixel offsets relative to `self.center_lat_lon` without clipping to a rectangle.
    pub fn lat_lon_to_map_px_offset(&self, lat: f64, lon: f64) -> Vec2 {
        let clamped_lat = WebMercator::clamp_lat(lat);
        let n = 2.0_f64.powf(self.zoom_level as f64);

        let (center_x, center_y) = WebMercator::lat_lon_to_tile_coords(
            self.center_lat_lon.0,
            self.center_lat_lon.1,
            self.zoom_level,
        );

        let target_x = (lon + 180.0) / 360.0 * n;
        let lat_rad = clamped_lat.to_radians();
        let target_y = (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI) * 0.5 * n;

        Vec2::new(
            ((target_x - center_x) * 256.0) as f32,
            ((target_y - center_y) * 256.0) as f32,
        )
    }

    /// Returns `(ocean_color, land_color, coast_color)` matching the active `VectorEarthTheme`.
    pub fn basemap_colors(&self, _theme: &crate::theme::MacTheme) -> (Color32, Color32, Color32) {
        let pal = self.tile_provider.palette();
        (pal.deep_ocean, pal.land_base, pal.coast_stroke)
    }

    /// Converts local CAD ground offset `[east_mm, north_mm]` around `origin_lat_lon` to `(lat, lon)`.
    pub fn world_mm_to_lat_lon(origin_lat_lon: (f64, f64), world_mm: [f64; 2]) -> (f64, f64) {
        let origin_lat = WebMercator::clamp_lat(origin_lat_lon.0);
        let cos_lat = origin_lat.to_radians().cos().max(0.1);
        let (ox, oy) = WebMercator::lat_lon_to_meters(origin_lat, origin_lat_lon.1);
        let mx = ox + (world_mm[0] / 1000.0) / cos_lat;
        let my = oy + (world_mm[1] / 1000.0) / cos_lat;
        WebMercator::meters_to_lat_lon(mx, my)
    }

    /// Converts WGS84 `(lat, lon)` into local ground CAD coordinates `[east_mm, north_mm]` around `origin_lat_lon`.
    pub fn lat_lon_to_world_mm(origin_lat_lon: (f64, f64), lat: f64, lon: f64) -> [f64; 2] {
        let origin_lat = WebMercator::clamp_lat(origin_lat_lon.0);
        let cos_lat = origin_lat.to_radians().cos().max(0.1);
        let (ox, oy) = WebMercator::lat_lon_to_meters(origin_lat, origin_lat_lon.1);
        let (mx, my) = WebMercator::lat_lon_to_meters(lat, lon);
        let mut dmx = mx - ox;
        if dmx > EARTH_HALF_CIRCUMFERENCE {
            dmx -= 2.0 * EARTH_HALF_CIRCUMFERENCE;
        } else if dmx < -EARTH_HALF_CIRCUMFERENCE {
            dmx += 2.0 * EARTH_HALF_CIRCUMFERENCE;
        }
        [dmx * cos_lat * 1000.0, (my - oy) * cos_lat * 1000.0]
    }

    /// Maps local ground CAD coordinates `[east_mm, north_mm]` onto screen `Pos2`.
    pub fn world_mm_to_screen(
        &self,
        origin_lat_lon: (f64, f64),
        world_mm: [f64; 2],
        rect: Rect,
    ) -> Pos2 {
        let (lat, lon) = Self::world_mm_to_lat_lon(origin_lat_lon, world_mm);
        self.lat_lon_to_screen(lat, lon, rect)
    }

    /// Maps screen `Pos2` onto local ground CAD coordinates `[east_mm, north_mm]` relative to `origin_lat_lon`.
    pub fn screen_to_world_mm(
        &self,
        origin_lat_lon: (f64, f64),
        pos: Pos2,
        rect: Rect,
    ) -> [f64; 2] {
        let (lat, lon) = self.screen_to_lat_lon(pos, rect);
        Self::lat_lon_to_world_mm(origin_lat_lon, lat, lon)
    }

    /// Screen pixels per engineering millimetre (`px / mm`) at the given site origin & map zoom.
    pub fn pixels_per_mm_at_origin(&self, origin_lat_lon: (f64, f64)) -> f32 {
        let res_m_px = WebMercator::ground_resolution_m_px(origin_lat_lon.0, self.zoom_level).max(1e-6);
        (1.0 / (res_m_px * 1000.0)) as f32
    }

    /// Resets map view to full Global Earth overview showing all continents.
    pub fn reset_to_global_view(&mut self) {
        self.center_lat_lon = (5.0, 98.0);
        self.zoom_level = 1.65;
    }

    /// Jumps to high-detail surveying / architectural drawing scale over a ground coordinate.
    pub fn zoom_to_ground_site(&mut self, lat: f64, lon: f64, zoom: f32) {
        self.set_center(lat, lon);
        self.zoom_level = zoom.clamp(1.0, 19.0);
    }

    /// Zooms the map smoothly while keeping `focus_pos` anchored under the pointer.
    pub fn zoom_at_screen_pos(&mut self, zoom_delta: f32, focus_pos: Option<Pos2>, rect: Rect) {
        if zoom_delta.abs() <= 1e-6 {
            return;
        }
        if let Some(mouse_pos) = focus_pos {
            let (mouse_lat, mouse_lon) = self.screen_to_lat_lon(mouse_pos, rect);
            self.zoom_level = (self.zoom_level + zoom_delta).clamp(1.0, 19.0);
            let new_mouse_screen = self.lat_lon_to_screen(mouse_lat, mouse_lon, rect);
            let corr_x = new_mouse_screen.x - mouse_pos.x;
            let corr_y = new_mouse_screen.y - mouse_pos.y;
            self.pan_by_pixels(corr_x, corr_y);
        } else {
            self.zoom_level = (self.zoom_level + zoom_delta).clamp(1.0, 19.0);
        }
    }

    /// Adds a new site pin at current active coordinates.
    pub fn add_pin(&mut self, lat: f64, lon: f64, label: &str) -> usize {
        let id = self.next_pin_id;
        self.next_pin_id += 1;

        let elevation_est = 10.0 + ((lat.abs() * 100.0).sin() * 25.0).abs() as f32;
        self.site_pins.push(SiteGeodeticPin {
            id,
            label: label.to_string(),
            lat,
            lon,
            elevation_m: elevation_est,
            project_name: format!("Project Lot {:03}", id),
            soil_class: "Site Class D (Standard Stiff Clay)".to_string(),
            ss_g: 0.825,
            s1_g: 0.395,
            notes: "Direct geodetic site pin dropped via EigenGraph GIS Inception Canvas".to_string(),
        });
        self.active_pin = Some((lat, lon));
        id
    }

    /// Great-circle Haversine distance in metres between two WGS84 `(lat, lon)` points.
    pub fn geodetic_distance_m(p1: (f64, f64), p2: (f64, f64)) -> f64 {
        const R_EARTH_M: f64 = 6_371_008.8;
        let lat1 = p1.0.to_radians();
        let lat2 = p2.0.to_radians();
        let dlat = (p2.0 - p1.0).to_radians();
        let dlon = (p2.1 - p1.1).to_radians();
        let a = (dlat * 0.5).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon * 0.5).sin().powi(2);
        let c = 2.0 * a.sqrt().clamp(0.0, 1.0).asin();
        R_EARTH_M * c
    }

    /// Total geodesic path distance in metres across an ordered vertex chain.
    pub fn geodetic_polyline_length_m(pts: &[(f64, f64)]) -> f64 {
        pts.windows(2)
            .map(|w| Self::geodetic_distance_m(w[0], w[1]))
            .sum()
    }

    /// Tangent-plane parcel polygon area in m² (Shoelace formula around vertex centroid).
    pub fn geodetic_polygon_area_m2(pts: &[(f64, f64)]) -> f64 {
        if pts.len() < 3 {
            return 0.0;
        }
        let origin = (
            pts.iter().map(|p| p.0).sum::<f64>() / pts.len() as f64,
            pts.iter().map(|p| p.1).sum::<f64>() / pts.len() as f64,
        );
        let xy_m: Vec<[f64; 2]> = pts
            .iter()
            .map(|&(lat, lon)| {
                let mm = Self::lat_lon_to_world_mm(origin, lat, lon);
                [mm[0] * 0.001, mm[1] * 0.001]
            })
            .collect();
        let mut double_area = 0.0;
        for i in 0..xy_m.len() {
            let j = (i + 1) % xy_m.len();
            double_area += xy_m[i][0] * xy_m[j][1] - xy_m[j][0] * xy_m[i][1];
        }
        0.5 * double_area.abs()
    }

    /// Resolves either `"lat, lon"` coordinates or a global engineering city name into `(label, lat, lon, zoom)`.
    pub fn resolve_location_query(query: &str) -> Option<(String, f64, f64, f32)> {
        let q = query.trim();
        if q.is_empty() {
            return None;
        }
        // 1. Direct "lat, lon" or "lat lon" parsing
        let cleaned = q.replace([',', ';'], " ");
        let nums: Vec<f64> = cleaned
            .split_whitespace()
            .filter_map(|tok| tok.trim_end_matches('°').parse::<f64>().ok())
            .collect();
        if nums.len() >= 2 {
            let lat = nums[0];
            let lon = nums[1];
            if (-85.05..=85.05).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                return Some((format!("{lat:.4}°, {lon:.4}°"), lat, lon, 16.5));
            }
        }

        // 2. Curated Global Engineering Cities & Infrastructure Hubs
        let lower = q.to_lowercase();
        for &(name, lat, lon, z) in GLOBAL_CITY_CATALOG {
            if name.to_lowercase().contains(&lower) {
                return Some((name.to_string(), lat, lon, z));
            }
        }
        None
    }

    /// Scans an address string, Vault subfolder path, or filename for a matching city cue (longest match first).
    pub fn resolve_city_hint_in_text(text: &str) -> Option<(String, f64, f64)> {
        let lower = text.to_lowercase();
        if lower.trim().is_empty() {
            return None;
        }
        let mut best: Option<(usize, &str, f64, f64)> = None;
        for &(city, lat, lon, _) in GLOBAL_CITY_CATALOG {
            let c_low = city.to_lowercase();
            // Require word-like boundary separation so "ikn" or "palu" avoids false substring noise inside generic words
            let bytes = lower.as_bytes();
            let cb = c_low.as_bytes();
            if let Some(pos) = lower.find(&c_low) {
                let left_ok = pos == 0 || !bytes[pos - 1].is_ascii_alphanumeric();
                let right_idx = pos + cb.len();
                let right_ok = right_idx >= bytes.len() || !bytes[right_idx].is_ascii_alphanumeric();
                if left_ok && right_ok && best.is_none_or(|(blen, _, _, _)| city.len() > blen) {
                    best = Some((city.len(), city, lat, lon));
                }
            }
        }
        best.map(|(_, city, lat, lon)| (city.to_string(), lat, lon))
    }

    /// Removes a site pin by ID.
    pub fn remove_pin(&mut self, id: usize) {
        self.site_pins.retain(|p| p.id != id);
        if let Some(active) = self.active_pin {
            if !self.site_pins.iter().any(|p| (p.lat - active.0).abs() < 1e-6 && (p.lon - active.1).abs() < 1e-6) {
                self.active_pin = self.site_pins.first().map(|p| (p.lat, p.lon));
            }
        }
    }

    /// Extracts real-time telemetry HUD summary.
    pub fn telemetry(&self) -> SlippyMapTelemetry {
        let (mx, my) = WebMercator::lat_lon_to_meters(self.center_lat_lon.0, self.center_lat_lon.1);
        let z_int = self.zoom_level.floor() as u8;
        let (tx, ty, tz) = WebMercator::get_tile_index(self.center_lat_lon.0, self.center_lat_lon.1, z_int);
        let res = WebMercator::ground_resolution_m_px(self.center_lat_lon.0, self.zoom_level);

        SlippyMapTelemetry {
            lat: self.center_lat_lon.0,
            lon: self.center_lat_lon.1,
            mercator_x: mx,
            mercator_y: my,
            zoom: self.zoom_level,
            tile_x: tx,
            tile_y: ty,
            tile_z: tz,
            ground_resolution_m_px: res,
            pins_count: self.site_pins.len(),
        }
    }

    /// Renders the complete interactive slippy map canvas widget with Freeze protection.
    pub fn render(&mut self, ui: &mut Ui, rect: Rect, theme: &crate::theme::MacTheme) {
        let is_frozen = crate::ui_kit::is_canvas_frozen(ui.ctx());
        self.render_with_freeze(ui, rect, theme, is_frozen);
    }

    /// Renders interactive World Map widget, locking pan, zoom, and pin placement when `is_frozen` is true.
    pub fn render_with_freeze(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        theme: &crate::theme::MacTheme,
        is_frozen: bool,
    ) {
        let painter = ui.painter_at(rect);
        let sense = if is_frozen {
            egui::Sense::hover()
        } else {
            egui::Sense::click_and_drag()
        };
        let response = ui.interact(rect, ui.id().with("slippy_map_canvas"), sense);

        if is_frozen {
            self.is_dragging = false;
            self.drag_start_mouse = None;
        } else {
            // 1. Mouse Drag Panning (only when unfrozen)
            if response.drag_started() {
                self.is_dragging = true;
                self.drag_start_mouse = response.interact_pointer_pos();
                self.drag_start_center = self.center_lat_lon;
            }

            if self.is_dragging {
                if response.dragged() {
                    let delta = response.drag_delta();
                    self.pan_by_pixels(delta.x, delta.y);
                }
                if response.drag_stopped() {
                    self.is_dragging = false;
                    self.drag_start_mouse = None;
                }
            }

            // 2. Mouse Wheel Smooth Zooming (Centering on Cursor)
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if response.hovered() && scroll_delta.abs() > 0.5 {
                let zoom_delta = scroll_delta * 0.003;
                self.zoom_at_screen_pos(zoom_delta, response.hover_pos(), rect);
            }

            // 3. Double-Click or Click to Drop Pin
            if response.double_clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (lat, lon) = self.screen_to_lat_lon(pos, rect);
                    self.add_pin(lat, lon, &format!("Site Pin #{:02}", self.site_pins.len() + 1));
                }
            } else if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (lat, lon) = self.screen_to_lat_lon(pos, rect);
                    self.active_pin = Some((lat, lon));
                }
            }
        }

        let ground_origin = self.active_pin.unwrap_or(self.center_lat_lon);
        self.render_world_background(ui, &painter, rect, theme, ground_origin, self.show_grid, true);

        if self.crosshair_active {
            self.render_crosshair(&painter, rect, theme);
        }
    }

    /// Renders pure World Map visuals (ocean, global vector continents, streamed raster tiles,
    /// geodetic & metric site grids, pins, attribution, scale bar) without allocating an overlapping widget input.
    pub fn render_world_background(
        &self,
        ui: &Ui,
        painter: &egui::Painter,
        rect: Rect,
        theme: &crate::theme::MacTheme,
        ground_origin_lat_lon: (f64, f64),
        show_grid: bool,
        draw_pins_and_scale: bool,
    ) {
        self.render_cartographic_basemap(ui, painter, rect, theme);

        if show_grid {
            self.render_geodetic_grid(painter, rect, theme);
            if self.zoom_level >= 11.5 {
                self.render_ground_metric_grid(painter, rect, theme, ground_origin_lat_lon);
            }
        }

        if self.show_elevation_contours && self.zoom_level >= 12.0 {
            self.render_elevation_contours(painter, rect, theme);
        }

        if draw_pins_and_scale {
            self.render_site_pins(painter, rect, theme);
            if self.show_scale_bar {
                self.render_scale_bar(painter, rect, theme);
            }
        }
    }

    fn render_cartographic_basemap(
        &self,
        _ui: &Ui,
        painter: &egui::Painter,
        rect: Rect,
        theme: &crate::theme::MacTheme,
    ) {
        let pal = self.tile_provider.palette();
        let (ocean_color, land_color, coast_color) = self.basemap_colors(theme);
        painter.rect_filled(rect, CornerRadius::same(8), ocean_color);

        // 2. Global Pure-Rust Vector Continents, Indonesian Archipelago, Ridges & Rivers
        for &wrap_deg in &[-360.0_f64, 0.0_f64, 360.0_f64] {
            for land in WORLD_LANDMASSES {
                let pts: Vec<Pos2> = land
                    .iter()
                    .map(|&(lat, lon)| self.lat_lon_to_screen_unwrapped(lat, lon + wrap_deg, rect))
                    .collect();
                if pts.len() < 3 {
                    continue;
                }
                let mut bounds = Rect::NOTHING;
                for &p in &pts {
                    bounds.extend_with(p);
                }
                if !bounds.intersects(rect) {
                    continue;
                }
                if let Some(mesh) = triangulate_polygon_2d(&pts, land_color) {
                    painter.add(egui::Shape::mesh(mesh));
                }
                painter.add(egui::epaint::PathShape::closed_line(
                    pts,
                    Stroke::new(1.0, coast_color),
                ));
            }
            if self.zoom_level >= 2.0 {
                for ridge in WORLD_MOUNTAIN_RIDGES {
                    let pts: Vec<Pos2> = ridge
                        .iter()
                        .map(|&(lat, lon)| self.lat_lon_to_screen_unwrapped(lat, lon + wrap_deg, rect))
                        .collect();
                    if pts.len() >= 2 && pts.iter().any(|p| rect.expand(30.0).contains(*p)) {
                        painter.add(egui::Shape::line(pts, Stroke::new(1.1, pal.ridge_stroke)));
                    }
                }
                for river in WORLD_MAJOR_RIVERS {
                    let pts: Vec<Pos2> = river
                        .iter()
                        .map(|&(lat, lon)| self.lat_lon_to_screen_unwrapped(lat, lon + wrap_deg, rect))
                        .collect();
                    if pts.len() >= 2 && pts.iter().any(|p| rect.expand(30.0).contains(*p)) {
                        painter.add(egui::Shape::line(pts, Stroke::new(1.3, pal.river_stroke)));
                    }
                }
            }
        }
    }

    fn render_geodetic_grid(&self, painter: &egui::Painter, rect: Rect, theme: &crate::theme::MacTheme) {
        let alpha = if self.zoom_level >= 11.0 { 20 } else { 42 };
        let grid_stroke = Stroke::new(0.8, Color32::from_rgba_unmultiplied(180, 200, 225, alpha));
        let equator_stroke = Stroke::new(1.3, theme.accent_blue.gamma_multiply(0.55));

        // Parallels (Equator, ±20°, ±40°, ±60°)
        for lat in [-60.0, -40.0, -20.0, 0.0, 20.0, 40.0, 60.0] {
            let p1 = self.lat_lon_to_screen_unwrapped(lat, 0.0, rect);
            let stroke = if lat.abs() < 1e-3 { equator_stroke } else { grid_stroke };
            if p1.y >= rect.min.y && p1.y <= rect.max.y {
                painter.line_segment([Pos2::new(rect.min.x, p1.y), Pos2::new(rect.max.x, p1.y)], stroke);
                if self.zoom_level < 10.0 {
                    painter.text(
                        Pos2::new(rect.min.x + 8.0, p1.y - 4.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("{:+.0}°", lat),
                        FontId::monospace(9.0),
                        theme.text_muted,
                    );
                }
            }
        }

        // Meridians (every 30°, wrapped cleanly across +/- 360°)
        for &wrap_deg in &[-360.0_f64, 0.0_f64, 360.0_f64] {
            for lon in (-180..180).step_by(30) {
                let lon_f = lon as f64;
                let p = self.lat_lon_to_screen_unwrapped(0.0, lon_f + wrap_deg, rect);
                if p.x >= rect.min.x && p.x <= rect.max.x {
                    painter.line_segment([Pos2::new(p.x, rect.min.y), Pos2::new(p.x, rect.max.y)], grid_stroke);
                    if self.zoom_level < 10.0 {
                        painter.text(
                            Pos2::new(p.x + 4.0, rect.max.y - 24.0),
                            egui::Align2::LEFT_BOTTOM,
                            format!("{:+.0}°", lon_f),
                            FontId::monospace(9.0),
                            theme.text_muted,
                        );
                    }
                }
            }
        }
    }

    /// Renders metric site surveying grid relative to `ground_origin_lat_lon` when zoomed to site level.
    fn render_ground_metric_grid(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &crate::theme::MacTheme,
        ground_origin_lat_lon: (f64, f64),
    ) {
        let res_m_px = WebMercator::ground_resolution_m_px(ground_origin_lat_lon.0, self.zoom_level).max(1e-4);
        let mut step_m = 1000.0_f64;
        for candidate_m in [0.5_f64, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0] {
            let px = candidate_m / res_m_px;
            if px >= 36.0 && px <= 140.0 {
                step_m = candidate_m;
                break;
            }
        }
        let step_mm = step_m * 1000.0;
        let min_mm = self.screen_to_world_mm(ground_origin_lat_lon, Pos2::new(rect.min.x, rect.max.y), rect);
        let max_mm = self.screen_to_world_mm(ground_origin_lat_lon, Pos2::new(rect.max.x, rect.min.y), rect);

        let start_ix = (min_mm[0] / step_mm).floor() as i64 - 1;
        let end_ix = (max_mm[0] / step_mm).ceil() as i64 + 1;
        let start_iy = (min_mm[1] / step_mm).floor() as i64 - 1;
        let end_iy = (max_mm[1] / step_mm).ceil() as i64 + 1;

        if (end_ix - start_ix).abs() > 80 || (end_iy - start_iy).abs() > 80 {
            return;
        }

        let minor_stroke = Stroke::new(0.8, theme.border_subtle.gamma_multiply(0.5));
        let axis_x_stroke = Stroke::new(1.5, theme.accent_red.gamma_multiply(0.6));
        let axis_y_stroke = Stroke::new(1.5, theme.accent_green.gamma_multiply(0.6));

        for ix in start_ix..=end_ix {
            let x_mm = ix as f64 * step_mm;
            let p_screen = self.world_mm_to_screen(ground_origin_lat_lon, [x_mm, 0.0], rect);
            if p_screen.x >= rect.min.x && p_screen.x <= rect.max.x {
                let stroke = if ix == 0 { axis_y_stroke } else { minor_stroke };
                painter.line_segment([Pos2::new(p_screen.x, rect.min.y), Pos2::new(p_screen.x, rect.max.y)], stroke);
            }
        }

        for iy in start_iy..=end_iy {
            let y_mm = iy as f64 * step_mm;
            let p_screen = self.world_mm_to_screen(ground_origin_lat_lon, [0.0, y_mm], rect);
            if p_screen.y >= rect.min.y && p_screen.y <= rect.max.y {
                let stroke = if iy == 0 { axis_x_stroke } else { minor_stroke };
                painter.line_segment([Pos2::new(rect.min.x, p_screen.y), Pos2::new(rect.max.x, p_screen.y)], stroke);
            }
        }
    }

    fn render_elevation_contours(&self, painter: &egui::Painter, rect: Rect, theme: &crate::theme::MacTheme) {
        // High zoom: draw simulated elevation contours around current center
        let (clat, clon) = self.center_lat_lon;
        let contour_stroke = Stroke::new(1.0, theme.accent_green.gamma_multiply(0.4));

        for r in 1..=4 {
            let radius_km = (r as f64) * 0.8;
            let deg_delta = radius_km / 111.0;
            let mut contour_pts = Vec::new();
            for step in 0..=36 {
                let theta = (step as f64) * (2.0 * PI / 36.0);
                // Perturb with harmonic pseudo-terrain ripple
                let perturbation = 1.0 + 0.15 * (4.0 * theta).sin() + 0.08 * (7.0 * theta).cos();
                let lat = clat + (deg_delta * perturbation) * theta.sin();
                let lon = clon + (deg_delta * perturbation) * theta.cos() / clat.to_radians().cos();
                contour_pts.push(self.lat_lon_to_screen(lat, lon, rect));
            }
            if contour_pts.len() >= 2 {
                for window in contour_pts.windows(2) {
                    painter.line_segment([window[0], window[1]], contour_stroke);
                }
            }
        }
    }

    fn render_site_pins(&self, painter: &egui::Painter, rect: Rect, theme: &crate::theme::MacTheme) {
        for pin in &self.site_pins {
            let pos = self.lat_lon_to_screen(pin.lat, pin.lon, rect);
            if rect.contains(pos) {
                let is_active = self.active_pin.map(|p| (p.0 - pin.lat).abs() < 1e-6 && (p.1 - pin.lon).abs() < 1e-6).unwrap_or(false);

                // Glow ring
                if is_active {
                    painter.circle_stroke(pos, 16.0, Stroke::new(2.0, theme.accent_blue.gamma_multiply(0.8)));
                    painter.circle_filled(pos, 14.0, theme.accent_blue.gamma_multiply(0.2));
                }

                // Pin circle and painted vector glyph
                painter.circle_filled(pos, 9.0, theme.accent_blue);
                painter.circle_stroke(pos, 9.0, Stroke::new(1.8, Color32::WHITE));
                crate::icons::AppIcon::MapPin.paint(
                    painter,
                    Rect::from_center_size(pos, Vec2::splat(11.0)),
                    Color32::WHITE,
                );

                // Pin label card
                let label_pos = Pos2::new(pos.x + 12.0, pos.y - 8.0);
                let label_text = format!("#{}: {} ({:.1}m)", pin.id, pin.label, pin.elevation_m);
                let text_color = if is_active { theme.accent_blue } else { theme.text_primary };

                painter.rect_filled(
                    Rect::from_min_size(label_pos, Vec2::new(label_text.len() as f32 * 6.5 + 8.0, 18.0)),
                    CornerRadius::same(4),
                    theme.bg_card.gamma_multiply(0.85),
                );
                painter.text(
                    Pos2::new(label_pos.x + 4.0, label_pos.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    label_text,
                    FontId::proportional(10.5),
                    text_color,
                );
            }
        }
    }

    fn render_crosshair(&self, painter: &egui::Painter, rect: Rect, theme: &crate::theme::MacTheme) {
        let c = rect.center();
        let crosshair_stroke = Stroke::new(1.2, theme.accent_blue.gamma_multiply(0.85));
        let len = 12.0;

        // Central cross lines
        painter.line_segment([Pos2::new(c.x - len, c.y), Pos2::new(c.x - 3.0, c.y)], crosshair_stroke);
        painter.line_segment([Pos2::new(c.x + 3.0, c.y), Pos2::new(c.x + len, c.y)], crosshair_stroke);
        painter.line_segment([Pos2::new(c.x, c.y - len), Pos2::new(c.x, c.y - 3.0)], crosshair_stroke);
        painter.line_segment([Pos2::new(c.x, c.y + 3.0), Pos2::new(c.x, c.y + len)], crosshair_stroke);

        // Center target ring
        painter.circle_stroke(c, 4.0, crosshair_stroke);
    }

    fn render_scale_bar(&self, painter: &egui::Painter, rect: Rect, theme: &crate::theme::MacTheme) {
        let res_m_px = WebMercator::ground_resolution_m_px(self.center_lat_lon.0, self.zoom_level);
        let target_bar_width_px = 120.0;
        let ground_meters = target_bar_width_px * res_m_px;

        // Round to clean metric increments (10m, 50m, 100m, 500m, 1km, 5km, etc.)
        let nice_meters = if ground_meters >= 5000.0 {
            (ground_meters / 5000.0).round() * 5000.0
        } else if ground_meters >= 1000.0 {
            (ground_meters / 1000.0).round() * 1000.0
        } else if ground_meters >= 100.0 {
            (ground_meters / 100.0).round() * 100.0
        } else {
            (ground_meters / 10.0).max(1.0).round() * 10.0
        };

        let actual_bar_width_px = (nice_meters / res_m_px) as f32;
        let bar_origin = Pos2::new(rect.min.x + 16.0, rect.max.y - 28.0);
        let bar_end = Pos2::new(bar_origin.x + actual_bar_width_px, bar_origin.y);

        // Background backing
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(bar_origin.x - 6.0, bar_origin.y - 14.0), Pos2::new(bar_end.x + 6.0, bar_origin.y + 12.0)),
            CornerRadius::same(4),
            theme.bg_card.gamma_multiply(0.85),
        );

        // Scale ticks & line
        let bar_stroke = Stroke::new(1.8, theme.text_primary);
        painter.line_segment([bar_origin, bar_end], bar_stroke);
        painter.line_segment([Pos2::new(bar_origin.x, bar_origin.y - 4.0), Pos2::new(bar_origin.x, bar_origin.y + 4.0)], bar_stroke);
        painter.line_segment([Pos2::new(bar_end.x, bar_end.y - 4.0), Pos2::new(bar_end.x, bar_end.y + 4.0)], bar_stroke);

        let scale_label = if nice_meters >= 1000.0 {
            format!("{:.1} km", nice_meters / 1000.0)
        } else {
            format!("{:.0} m", nice_meters)
        };

        painter.text(
            Pos2::new((bar_origin.x + bar_end.x) / 2.0, bar_origin.y - 6.0),
            egui::Align2::CENTER_BOTTOM,
            scale_label,
            FontId::monospace(10.0),
            theme.text_primary,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_mercator_projection_roundtrip() {
        // Monas Jakarta (-6.1754°, 106.8272°)
        let lat = -6.175392;
        let lon = 106.827153;

        let (x, y) = WebMercator::lat_lon_to_meters(lat, lon);
        let (lat_back, lon_back) = WebMercator::meters_to_lat_lon(x, y);

        assert!((lat - lat_back).abs() < 1e-6, "Latitude roundtrip mismatch: {} vs {}", lat, lat_back);
        assert!((lon - lon_back).abs() < 1e-6, "Longitude roundtrip mismatch: {} vs {}", lon, lon_back);
    }

    #[test]
    fn test_tile_coordinates_conversion() {
        let lat = 0.0;
        let lon = 0.0;
        let zoom = 1.0;

        let (tx, ty) = WebMercator::lat_lon_to_tile_coords(lat, lon, zoom);
        assert!((tx - 1.0).abs() < 1e-5);
        assert!((ty - 1.0).abs() < 1e-5);

        let (lat_back, lon_back) = WebMercator::tile_coords_to_lat_lon(tx, ty, zoom);
        assert!((lat_back - 0.0).abs() < 1e-5);
        assert!((lon_back - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_ground_resolution_and_discrete_tile() {
        let lat = -6.1754;
        let lon = 106.8272;
        let zoom = 14.0;

        let res = WebMercator::ground_resolution_m_px(lat, zoom);
        assert!(res > 5.0 && res < 15.0, "Resolution at zoom 14 near equator should be ~9.5 m/px: got {}", res);

        let (tx, ty, tz) = WebMercator::get_tile_index(lat, lon, 14);
        assert_eq!(tz, 14);
        assert!(tx > 12000 && tx < 14000);
        assert!(ty > 7000 && ty < 9000);
    }

    #[test]
    fn test_slippy_map_state_pan_zoom_and_pins() {
        let mut map = SlippyMapState::new();
        assert_eq!(map.site_pins.len(), 0);
        assert!(map.zoom_level < 3.0); // Global World Map overview by default

        let initial_lat = map.center_lat_lon.0;
        map.pan_by_pixels(100.0, 50.0);
        assert_ne!(map.center_lat_lon.0, initial_lat);

        let initial_zoom = map.zoom_level;
        map.zoom_in(1.0);
        assert_eq!(map.zoom_level, initial_zoom + 1.0);

        map.zoom_out(0.5);
        assert_eq!(map.zoom_level, initial_zoom + 0.5);

        let pin_id = map.add_pin(-6.9175, 107.6191, "Bandung Project Lot");
        assert_eq!(map.site_pins.len(), 1);
        assert_eq!(map.active_pin, Some((-6.9175, 107.6191)));

        map.remove_pin(pin_id);
        assert_eq!(map.site_pins.len(), 0);

        let telem = map.telemetry();
        assert_eq!(telem.pins_count, 0);
        assert_eq!(telem.zoom, initial_zoom + 0.5);
    }

    #[test]
    fn test_world_mm_lat_lon_bijection_roundtrip() {
        let origin = (-6.2, 106.8);
        let world_mm = [25_400.0, -14_250.0];
        let (lat, lon) = SlippyMapState::world_mm_to_lat_lon(origin, world_mm);
        let back = SlippyMapState::lat_lon_to_world_mm(origin, lat, lon);
        assert!((back[0] - world_mm[0]).abs() < 1e-3);
        assert!((back[1] - world_mm[1]).abs() < 1e-3);
    }

    #[test]
    fn test_geodesic_search_and_distance_area() {
        let found = SlippyMapState::resolve_location_query("Jakarta").expect("city resolves");
        assert!((found.1 - (-6.2088)).abs() < 1e-3);

        let coord = SlippyMapState::resolve_location_query("-6.9175, 107.6191").expect("coord parses");
        assert!((coord.1 - (-6.9175)).abs() < 1e-4);
        assert!((coord.2 - 107.6191).abs() < 1e-4);

        let d = SlippyMapState::geodetic_distance_m((-6.2, 106.8), (-6.2, 106.801));
        assert!(d > 100.0 && d < 120.0);

        let poly = [(-6.2, 106.8), (-6.2, 106.801), (-6.201, 106.801), (-6.201, 106.8)];
        let area = SlippyMapState::geodetic_polygon_area_m2(&poly);
        assert!(area > 11_000.0 && area < 13_000.0);
    }

    #[test]
    fn test_phase64_pure_vector_theme_cycling_and_zero_raster_state() {
        let mut map = SlippyMapState::default();
        assert_eq!(map.tile_provider, VectorEarthTheme::CoastalBlueprint);
        assert_eq!(map.visual_layer_label(), "Blueprint");
        map.cycle_basemap_visual_mode();
        assert_eq!(map.tile_provider, VectorEarthTheme::CityDaylight);
        assert_eq!(map.visual_layer_label(), "Daylight");
        map.cycle_basemap_visual_mode();
        assert_eq!(map.tile_provider, VectorEarthTheme::NightTwin);
        map.cycle_basemap_visual_mode();
        assert_eq!(map.tile_provider, VectorEarthTheme::Topographic);
        map.cycle_basemap_visual_mode();
        assert_eq!(map.tile_provider, VectorEarthTheme::CoastalBlueprint);
    }
}
