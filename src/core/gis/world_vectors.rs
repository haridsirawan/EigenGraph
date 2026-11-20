//! world_vectors.rs — 100% Pure-Rust Embedded Natural Earth Vector Cartography & Themes (Phase 64 / v0.6.4).
//!
//! Replaces all external raster tile providers (Satellite, OSM, OpenTopo, CartoDB) with zero-dependency,
//! zero-network stylized vector palettes, high-clarity global continents & Indonesian archipelago
//! polygons, topographic mountain ridges, and major hydrological river corridors.

use egui::Color32;

/// Pure-Rust Vector Earth cartographic themes replacing external raster basemaps (Phase 64 / v0.6.4).
///
/// Includes `serde` aliases for all legacy `TileSource` variants so existing `.eigen.json` archives
/// deserialize transparently without error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum VectorEarthTheme {
    /// EigenGraph signature Coastal Marine Teal engineering blueprint palette (default).
    #[default]
    #[serde(alias = "SyntheticVectorTerrain", alias = "EsriWorldImagery")]
    CoastalBlueprint,
    /// Bright stylized city-builder daylight simulator palette.
    #[serde(alias = "OpenStreetMap")]
    CityDaylight,
    /// Deep twilight cyber-twin palette with luminous urban corridors.
    #[serde(alias = "CartoDbDarkMatter")]
    NightTwin,
    /// Clean architectural topographic engineering site-plan palette.
    #[serde(alias = "OpenTopoMap")]
    Topographic,
}

/// Backward-compatible type alias retained for legacy API and archive references.
pub type TileSource = VectorEarthTheme;

/// Complete color palette governing 3D Vector Earth globe, continents, rivers, ridges,
/// city-builder parcels, roads, and engineering grids for a given `VectorEarthTheme`.
#[derive(Debug, Clone, Copy)]
pub struct VectorEarthPalette {
    pub space_bg: Color32,
    pub sky_horizon: Color32,
    pub atmosphere_rgb: [u8; 3],
    pub deep_ocean: Color32,
    pub shallow_shelf: Color32,
    pub land_base: Color32,
    pub land_arid: Color32,
    pub land_polar: Color32,
    pub coast_stroke: Color32,
    pub river_stroke: Color32,
    pub ridge_stroke: Color32,
    pub graticule_stroke: Color32,
    pub equator_stroke: Color32,
    pub grid_major: Color32,
    pub grid_minor: Color32,
    pub parcel_fill: Color32,
    pub road_arterial: Color32,
    pub road_local: Color32,
    pub city_dot: Color32,
    pub city_label: Color32,
    pub corridor_stroke: Color32,
}

impl VectorEarthTheme {
    pub const ALL: [VectorEarthTheme; 4] = [
        VectorEarthTheme::CoastalBlueprint,
        VectorEarthTheme::CityDaylight,
        VectorEarthTheme::NightTwin,
        VectorEarthTheme::Topographic,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CoastalBlueprint => "Coastal Blueprint (Vector)",
            Self::CityDaylight => "City Daylight (Simulator)",
            Self::NightTwin => "Night Cyber-Twin (Vector)",
            Self::Topographic => "Topographic Engineering (Vector)",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::CoastalBlueprint => "Blueprint",
            Self::CityDaylight => "Daylight",
            Self::NightTwin => "Night Twin",
            Self::Topographic => "Topo Vec",
        }
    }

    pub fn attribution_text(&self) -> &'static str {
        "EigenGraph Pure-Rust Vector Earth (Offline OpenBIM)"
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::CoastalBlueprint => Self::CityDaylight,
            Self::CityDaylight => Self::NightTwin,
            Self::NightTwin => Self::Topographic,
            Self::Topographic => Self::CoastalBlueprint,
        }
    }

    pub fn max_zoom(&self) -> i32 {
        20
    }

    /// Returns the curated pure-vector color palette for this theme.
    pub fn palette(&self) -> VectorEarthPalette {
        match self {
            Self::CoastalBlueprint => VectorEarthPalette {
                space_bg: Color32::from_rgb(7, 13, 25),
                sky_horizon: Color32::from_rgb(11, 20, 39),
                atmosphere_rgb: [91, 146, 165],
                deep_ocean: Color32::from_rgb(13, 28, 48),
                shallow_shelf: Color32::from_rgba_unmultiplied(91, 146, 165, 58),
                land_base: Color32::from_rgb(35, 62, 86),
                land_arid: Color32::from_rgb(47, 73, 94),
                land_polar: Color32::from_rgb(154, 188, 202),
                coast_stroke: Color32::from_rgb(122, 186, 200),
                river_stroke: Color32::from_rgba_unmultiplied(91, 184, 212, 155),
                ridge_stroke: Color32::from_rgba_unmultiplied(208, 235, 225, 76),
                graticule_stroke: Color32::from_rgba_unmultiplied(120, 174, 198, 44),
                equator_stroke: Color32::from_rgba_unmultiplied(91, 186, 212, 115),
                grid_major: Color32::from_rgba_unmultiplied(91, 182, 204, 96),
                grid_minor: Color32::from_rgba_unmultiplied(134, 174, 194, 42),
                parcel_fill: Color32::from_rgba_unmultiplied(43, 73, 99, 68),
                road_arterial: Color32::from_rgba_unmultiplied(208, 235, 225, 130),
                road_local: Color32::from_rgba_unmultiplied(91, 146, 165, 95),
                city_dot: Color32::from_rgb(208, 235, 225),
                city_label: Color32::from_rgb(200, 226, 236),
                corridor_stroke: Color32::from_rgba_unmultiplied(91, 182, 208, 72),
            },
            Self::CityDaylight => VectorEarthPalette {
                space_bg: Color32::from_rgb(16, 30, 52),
                sky_horizon: Color32::from_rgb(186, 220, 244),
                atmosphere_rgb: [104, 192, 255],
                deep_ocean: Color32::from_rgb(42, 118, 168),
                shallow_shelf: Color32::from_rgba_unmultiplied(120, 212, 226, 82),
                land_base: Color32::from_rgb(122, 168, 118),
                land_arid: Color32::from_rgb(204, 188, 144),
                land_polar: Color32::from_rgb(234, 242, 248),
                coast_stroke: Color32::from_rgb(218, 236, 216),
                river_stroke: Color32::from_rgba_unmultiplied(64, 164, 218, 185),
                ridge_stroke: Color32::from_rgba_unmultiplied(126, 116, 94, 105),
                graticule_stroke: Color32::from_rgba_unmultiplied(226, 242, 255, 52),
                equator_stroke: Color32::from_rgba_unmultiplied(245, 198, 92, 130),
                grid_major: Color32::from_rgba_unmultiplied(46, 112, 152, 95),
                grid_minor: Color32::from_rgba_unmultiplied(84, 128, 106, 46),
                parcel_fill: Color32::from_rgba_unmultiplied(224, 226, 214, 92),
                road_arterial: Color32::from_rgba_unmultiplied(244, 184, 72, 190),
                road_local: Color32::from_rgba_unmultiplied(242, 245, 248, 140),
                city_dot: Color32::from_rgb(255, 214, 102),
                city_label: Color32::from_rgb(248, 252, 255),
                corridor_stroke: Color32::from_rgba_unmultiplied(255, 224, 130, 96),
            },
            Self::NightTwin => VectorEarthPalette {
                space_bg: Color32::from_rgb(4, 7, 14),
                sky_horizon: Color32::from_rgb(6, 10, 20),
                atmosphere_rgb: [56, 152, 248],
                deep_ocean: Color32::from_rgb(8, 15, 28),
                shallow_shelf: Color32::from_rgba_unmultiplied(46, 128, 212, 46),
                land_base: Color32::from_rgb(21, 29, 44),
                land_arid: Color32::from_rgb(28, 34, 48),
                land_polar: Color32::from_rgb(76, 96, 120),
                coast_stroke: Color32::from_rgb(64, 142, 214),
                river_stroke: Color32::from_rgba_unmultiplied(56, 189, 248, 130),
                ridge_stroke: Color32::from_rgba_unmultiplied(128, 158, 212, 62),
                graticule_stroke: Color32::from_rgba_unmultiplied(78, 118, 168, 36),
                equator_stroke: Color32::from_rgba_unmultiplied(56, 189, 248, 108),
                grid_major: Color32::from_rgba_unmultiplied(56, 189, 248, 86),
                grid_minor: Color32::from_rgba_unmultiplied(100, 128, 168, 36),
                parcel_fill: Color32::from_rgba_unmultiplied(30, 42, 64, 72),
                road_arterial: Color32::from_rgba_unmultiplied(245, 158, 11, 165),
                road_local: Color32::from_rgba_unmultiplied(96, 165, 250, 92),
                city_dot: Color32::from_rgb(251, 191, 36),
                city_label: Color32::from_rgb(224, 238, 255),
                corridor_stroke: Color32::from_rgba_unmultiplied(251, 191, 36, 78),
            },
            Self::Topographic => VectorEarthPalette {
                space_bg: Color32::from_rgb(22, 32, 44),
                sky_horizon: Color32::from_rgb(220, 232, 242),
                atmosphere_rgb: [132, 178, 206],
                deep_ocean: Color32::from_rgb(174, 206, 224),
                shallow_shelf: Color32::from_rgba_unmultiplied(208, 232, 242, 92),
                land_base: Color32::from_rgb(232, 230, 216),
                land_arid: Color32::from_rgb(226, 214, 188),
                land_polar: Color32::from_rgb(245, 248, 250),
                coast_stroke: Color32::from_rgb(104, 132, 148),
                river_stroke: Color32::from_rgba_unmultiplied(74, 146, 192, 185),
                ridge_stroke: Color32::from_rgba_unmultiplied(146, 122, 96, 120),
                graticule_stroke: Color32::from_rgba_unmultiplied(96, 122, 142, 54),
                equator_stroke: Color32::from_rgba_unmultiplied(46, 124, 168, 125),
                grid_major: Color32::from_rgba_unmultiplied(74, 114, 138, 92),
                grid_minor: Color32::from_rgba_unmultiplied(128, 144, 152, 48),
                parcel_fill: Color32::from_rgba_unmultiplied(212, 214, 204, 86),
                road_arterial: Color32::from_rgba_unmultiplied(186, 124, 68, 170),
                road_local: Color32::from_rgba_unmultiplied(136, 148, 156, 125),
                city_dot: Color32::from_rgb(43, 73, 99),
                city_label: Color32::from_rgb(24, 38, 54),
                corridor_stroke: Color32::from_rgba_unmultiplied(91, 146, 165, 92),
            },
        }
    }
}

/// High-clarity pure-Rust embedded Natural Earth vector coastline & island polygons `(lat_deg, lon_deg)`.
///
/// Includes detailed global continents alongside an extra-sharp Indonesian & Southeast Asian archipelago
/// (Sumatra, Java, Madura, Bangka, Belitung, Kalimantan/Borneo, 4-peninsula Sulawesi, Bali, Lombok,
/// Sumbawa, Sumba, Flores, Timor, Halmahera, Seram, Buru, Papua/New Guinea).
pub const WORLD_LANDMASSES: &[&[(f64, f64)]] = &[
    // North & Central America
    &[
        (70.2, -162.5), (71.3, -156.5), (70.3, -146.0), (69.6, -135.0), (68.5, -120.0),
        (68.2, -96.0), (63.5, -90.5), (56.5, -88.0), (54.5, -79.5), (62.5, -78.0),
        (61.0, -64.5), (53.5, -56.0), (47.0, -53.5), (44.5, -63.5), (42.0, -70.0),
        (39.0, -74.8), (35.2, -75.5), (30.5, -81.4), (25.1, -80.2), (26.5, -82.2),
        (29.8, -84.5), (29.2, -90.0), (28.8, -95.5), (25.5, -97.4), (21.5, -97.3),
        (18.5, -94.0), (21.3, -89.8), (21.4, -86.8), (16.2, -88.4), (15.8, -84.0),
        (11.0, -83.6), (9.3, -79.9), (8.6, -77.4), (7.4, -80.4), (9.8, -85.5),
        (13.2, -88.2), (15.0, -93.0), (16.6, -99.8), (20.2, -105.6), (25.2, -108.5),
        (31.5, -114.2), (24.0, -110.2), (23.0, -110.0), (28.0, -114.2), (32.6, -117.2),
        (35.0, -120.8), (38.2, -123.0), (42.5, -124.5), (48.4, -124.7), (54.5, -131.0),
        (58.5, -137.5), (60.5, -145.5), (58.0, -154.0), (55.0, -162.5), (60.5, -165.5),
        (65.6, -168.0),
    ],
    // Cuba & Hispaniola
    &[(23.1, -82.4), (22.6, -79.0), (20.1, -74.2), (19.9, -77.6), (21.8, -82.8), (22.2, -84.9)],
    &[(19.9, -71.7), (19.2, -69.2), (18.4, -68.4), (18.2, -71.6), (18.3, -74.4), (19.8, -73.4)],
    // Greenland
    &[
        (82.8, -32.0), (81.5, -17.5), (76.0, -19.0), (70.0, -22.0), (65.5, -37.0),
        (59.9, -43.8), (64.5, -52.0), (70.5, -54.5), (75.5, -58.5), (78.5, -72.5),
        (81.8, -61.5),
    ],
    // Iceland
    &[(66.2, -23.5), (66.4, -16.0), (65.1, -13.6), (63.4, -19.0), (64.1, -22.7)],
    // South America
    &[
        (12.2, -71.6), (10.6, -66.8), (10.3, -62.0), (6.5, -57.5), (4.5, -52.0),
        (0.0, -49.8), (-2.5, -42.0), (-5.5, -35.2), (-9.5, -35.5), (-13.0, -38.6),
        (-19.5, -39.8), (-23.0, -42.2), (-25.5, -48.2), (-33.5, -53.2), (-38.0, -57.6),
        (-41.5, -64.5), (-46.5, -66.8), (-52.2, -68.5), (-55.4, -67.2), (-52.8, -74.5),
        (-46.5, -75.0), (-41.5, -73.8), (-33.2, -71.6), (-23.5, -70.5), (-18.4, -70.4),
        (-14.0, -76.3), (-8.0, -79.2), (-4.6, -81.3), (1.0, -79.8), (4.5, -77.4),
        (8.8, -77.2), (10.8, -75.2),
    ],
    // Europe Mainland + Scandinavia + Italy + Greece
    &[
        (36.0, -5.6), (37.0, -9.0), (43.3, -9.2), (43.4, -1.8), (47.5, -3.2),
        (48.5, -4.7), (49.7, -1.7), (51.0, 2.0), (53.2, 5.2), (54.0, 8.6),
        (57.7, 10.6), (55.4, 10.8), (54.5, 11.2), (55.5, 13.0), (59.2, 10.6),
        (58.0, 7.0), (62.5, 5.4), (66.5, 12.5), (69.5, 18.0), (71.1, 25.8),
        (69.8, 31.2), (66.5, 28.5), (65.0, 25.4), (60.5, 21.3), (60.0, 24.8),
        (59.6, 28.5), (59.4, 24.5), (57.5, 21.5), (54.7, 19.0), (54.2, 13.8),
        (53.8, 10.2), (49.0, 14.0), (45.6, 13.7), (44.0, 12.6), (40.3, 18.4),
        (38.0, 15.8), (38.8, 16.0), (41.8, 12.3), (44.4, 8.9), (43.3, 5.3),
        (41.4, 2.2), (38.5, -0.2), (36.6, -4.4),
    ],
    // Italy (Sicily) & Sardinia
    &[(38.2, 15.6), (36.7, 15.1), (37.6, 12.5), (38.1, 13.4)],
    &[(41.2, 9.4), (39.1, 9.5), (39.0, 8.4), (40.9, 8.2)],
    // Balkans / Greece Peninsula
    &[(45.4, 13.8), (42.5, 18.5), (39.5, 20.0), (36.6, 22.5), (38.0, 24.0), (40.6, 23.0), (40.8, 26.2), (42.2, 27.8), (44.5, 28.8), (46.5, 30.8), (48.0, 24.0)],
    // Great Britain & Ireland
    &[
        (50.1, -5.6), (51.6, -5.1), (53.4, -3.2), (55.0, -4.8), (58.6, -5.0),
        (58.6, -3.1), (57.6, -1.8), (55.8, -2.0), (53.6, 0.1), (52.7, 1.7),
        (51.1, 1.4), (50.6, -1.8),
    ],
    &[(51.5, -9.8), (53.2, -9.9), (54.5, -8.5), (55.3, -6.5), (53.4, -6.1), (52.1, -6.4)],
    // Africa Mainland
    &[
        (35.8, -5.6), (36.8, 3.0), (37.2, 10.2), (33.5, 11.2), (32.5, 15.2),
        (31.2, 20.0), (31.5, 25.2), (31.2, 32.3), (27.5, 33.8), (22.0, 36.9),
        (15.6, 39.5), (12.0, 43.4), (11.9, 51.3), (5.0, 48.2), (1.8, 45.2),
        (-1.8, 41.2), (-5.2, 39.2), (-10.8, 40.6), (-15.5, 40.5), (-20.0, 34.8),
        (-25.8, 32.8), (-31.5, 29.5), (-34.8, 20.0), (-33.9, 18.4), (-26.5, 15.0),
        (-18.0, 11.8), (-12.5, 13.3), (-5.5, 12.1), (1.0, 9.4), (4.0, 9.5),
        (4.5, 6.2), (6.4, 2.5), (4.8, -2.0), (4.5, -7.5), (8.5, -13.2),
        (12.2, -16.6), (14.8, -17.5), (21.0, -17.0), (27.8, -13.2), (31.5, -9.8),
    ],
    // Madagascar
    &[(-12.0, 49.3), (-15.5, 50.3), (-19.8, 48.8), (-25.2, 47.0), (-25.0, 44.0), (-20.0, 44.2), (-15.8, 46.2)],
    // Asia Mainland (Siberia + Middle East + India + Indochina + Malay Peninsula + Korea + China)
    &[
        (69.5, 32.0), (68.5, 50.0), (72.5, 70.0), (77.5, 104.2), (73.5, 126.0),
        (70.5, 152.0), (69.5, 172.0), (66.0, 179.2), (61.5, 172.0), (59.5, 163.5),
        (51.5, 156.6), (57.5, 156.0), (60.0, 148.0), (54.5, 137.0), (46.0, 138.0),
        (42.8, 131.0), (38.8, 128.4), (35.1, 129.1), (34.8, 126.4), (37.5, 126.2),
        (39.8, 124.2), (39.0, 121.6), (40.5, 120.5), (38.5, 117.8), (37.3, 122.4),
        (35.0, 119.5), (31.2, 121.9), (28.0, 121.4), (24.5, 118.2), (22.3, 114.2),
        (20.4, 110.2), (21.5, 108.0), (19.5, 105.8), (16.0, 108.3), (12.5, 109.4),
        (10.2, 107.0), (8.6, 104.8), (10.5, 104.0), (13.4, 100.6), (9.5, 99.8),
        (6.2, 102.2), (1.3, 104.1), (1.4, 103.5), (3.1, 101.3), (6.5, 100.1),
        (10.0, 98.5), (16.5, 97.4), (16.0, 94.3), (21.5, 92.0), (22.0, 89.0),
        (19.5, 85.2), (15.8, 80.5), (10.5, 79.8), (8.1, 77.5), (12.8, 74.8),
        (19.0, 72.8), (22.5, 69.0), (24.8, 67.0), (25.2, 61.5), (26.6, 56.5),
        (30.0, 48.2), (26.5, 50.2), (24.2, 54.0), (25.8, 56.3), (22.5, 59.8),
        (17.0, 54.5), (12.8, 45.0), (12.7, 43.4), (20.0, 40.5), (28.2, 34.6),
        (31.2, 34.2), (36.4, 36.0), (36.8, 27.5), (41.0, 29.0), (41.2, 36.5),
        (41.8, 41.6), (45.2, 37.5), (47.2, 39.2), (55.0, 38.0),
    ],
    // Sri Lanka, Hainan, Taiwan
    &[(9.8, 80.2), (8.4, 81.4), (6.0, 80.6), (6.8, 79.8), (8.5, 79.8)],
    &[(20.0, 110.3), (19.0, 110.6), (18.2, 109.5), (19.2, 108.6)],
    &[(25.3, 121.6), (23.8, 121.6), (21.9, 120.8), (23.0, 120.1), (24.8, 120.9)],
    // Philippines (Luzon, Visayas, Mindanao)
    &[(18.5, 121.0), (18.3, 122.3), (16.0, 122.0), (13.0, 124.1), (12.6, 123.5), (13.8, 121.2), (14.6, 120.3), (16.4, 119.9)],
    &[(12.3, 124.4), (11.0, 125.6), (9.6, 124.2), (9.9, 122.4), (11.7, 122.0)],
    &[(9.8, 125.5), (7.5, 126.5), (5.6, 125.4), (6.8, 122.0), (8.4, 123.0), (8.9, 124.8)],
    // Japan (Hokkaido, Honshu, Shikoku, Kyushu)
    &[(45.5, 141.9), (44.0, 145.2), (43.3, 145.6), (42.0, 143.2), (41.5, 140.1), (43.4, 140.4)],
    &[
        (41.4, 140.8), (40.3, 141.9), (38.2, 141.5), (35.7, 140.9), (35.0, 139.8),
        (34.5, 137.0), (33.5, 135.8), (34.4, 133.5), (34.0, 130.9), (35.5, 133.0),
        (37.4, 137.2), (39.6, 140.0),
    ],
    &[(34.3, 134.5), (33.3, 134.1), (32.8, 132.7), (33.8, 132.4)],
    &[(33.9, 131.0), (32.5, 131.7), (31.0, 130.7), (32.1, 130.0), (33.4, 129.6)],
    // === INDONESIAN ARCHIPELAGO (HIGH-CLARITY PURE VECTOR) ===
    // Sumatra
    &[
        (5.55, 95.32), (5.20, 97.45), (3.85, 98.70), (2.70, 99.95), (1.95, 101.20),
        (1.10, 102.50), (0.15, 103.60), (-0.95, 103.90), (-1.95, 104.90), (-2.95, 105.95),
        (-4.35, 105.85), (-5.85, 105.75), (-5.50, 104.55), (-4.70, 103.30), (-3.75, 102.25),
        (-2.60, 101.10), (-1.35, 100.45), (-0.15, 99.85), (1.25, 98.80), (2.65, 97.60),
        (3.85, 96.45), (4.85, 95.40),
    ],
    // Bangka & Belitung
    &[(-1.52, 105.85), (-2.15, 106.25), (-3.05, 106.65), (-2.90, 106.05), (-2.05, 105.20)],
    &[(-2.55, 107.65), (-2.75, 108.25), (-3.25, 108.15), (-3.15, 107.55)],
    // Java (Ujung Kulon -> Jakarta Bay -> Cirebon -> Semarang -> Jepara -> Surabaya -> Banyuwangi -> South Coast)
    &[
        (-6.75, 105.22), (-6.02, 105.90), (-5.96, 106.45), (-6.10, 106.86), (-5.95, 107.40),
        (-6.22, 108.35), (-6.78, 108.65), (-6.86, 109.65), (-6.94, 110.42), (-6.44, 110.90),
        (-6.72, 111.65), (-6.92, 112.55), (-7.26, 112.82), (-7.65, 113.55), (-7.78, 114.42),
        (-8.42, 114.36), (-8.65, 113.85), (-8.38, 112.60), (-8.25, 111.40), (-8.12, 110.55),
        (-7.98, 110.05), (-7.74, 109.00), (-7.76, 108.35), (-7.42, 107.10), (-7.02, 106.45),
        (-6.88, 105.65),
    ],
    // Madura
    &[(-6.88, 112.75), (-6.90, 113.95), (-7.20, 114.08), (-7.22, 113.25), (-7.14, 112.70)],
    // Kalimantan / Borneo (with Balikpapan / IKN Nusantara Bay, Pontianak, Banjarmasin, Sabah, Sarawak)
    &[
        (7.02, 116.85), (5.95, 118.10), (5.25, 119.25), (4.20, 117.90), (2.80, 117.85),
        (1.05, 118.95), (0.35, 117.55), (-0.65, 117.35), (-1.26, 116.82), (-2.25, 116.45),
        (-3.35, 116.05), (-4.18, 114.65), (-3.45, 114.10), (-3.15, 112.90), (-3.45, 111.75),
        (-3.00, 110.20), (-1.85, 109.90), (-0.85, 109.30), (0.15, 109.10), (1.35, 109.05),
        (1.85, 110.45), (2.75, 111.40), (3.45, 113.10), (4.85, 114.95), (5.95, 116.05),
    ],
    // Sulawesi (Authentic 4-Peninsula K-Shape: Minahasa, East Luwuk, Southeast Kendari, South Makassar)
    &[
        (1.68, 125.15), (1.35, 124.55), (0.55, 123.30), (0.45, 121.45), (0.65, 120.15),
        (-0.55, 120.05), (-1.20, 120.75), (-0.95, 122.40), (-0.80, 123.35), (-1.35, 123.10),
        (-1.95, 121.55), (-2.60, 121.95), (-3.85, 122.65), (-4.75, 122.55), (-4.55, 121.60),
        (-3.45, 121.05), (-2.70, 120.55), (-3.45, 120.35), (-4.55, 120.42), (-5.58, 120.30),
        (-5.65, 119.45), (-5.12, 119.38), (-3.95, 119.60), (-3.52, 118.95), (-2.55, 118.85),
        (-1.25, 119.35), (-0.15, 119.80), (0.98, 120.85), (1.15, 122.90), (1.48, 124.80),
    ],
    // Bali, Lombok, Sumbawa, Sumba, Flores, Timor
    &[(-8.08, 114.44), (-8.12, 115.20), (-8.38, 115.70), (-8.84, 115.18), (-8.62, 114.62)],
    &[(-8.22, 116.10), (-8.28, 116.68), (-8.88, 116.58), (-8.86, 116.02)],
    &[(-8.36, 116.78), (-8.15, 118.00), (-8.30, 119.15), (-8.92, 119.05), (-9.05, 117.20), (-8.86, 116.75)],
    &[(-9.35, 119.25), (-9.65, 120.55), (-10.28, 120.45), (-9.85, 119.05)],
    &[(-8.35, 119.85), (-8.45, 121.85), (-8.22, 123.02), (-8.72, 122.85), (-8.86, 121.15), (-8.82, 119.88)],
    &[(-8.38, 127.28), (-8.95, 126.40), (-9.55, 124.95), (-10.28, 123.58), (-9.75, 123.62), (-9.02, 124.85), (-8.52, 125.60)],
    // Maluku (Halmahera, Seram, Buru)
    &[(2.15, 127.95), (1.45, 128.65), (0.35, 128.75), (-0.80, 128.40), (-0.20, 127.70), (0.85, 127.55)],
    &[(-2.85, 128.05), (-2.95, 130.70), (-3.55, 130.85), (-3.45, 128.15)],
    &[(-3.12, 126.10), (-3.22, 127.15), (-3.80, 127.05), (-3.68, 126.05)],
    // Papua / New Guinea (Bird's Head -> Cenderawasih Bay -> Jayapura -> PNG -> Merauke -> Mimika)
    &[
        (-0.85, 130.85), (-0.42, 132.45), (-1.45, 134.15), (-2.95, 134.55), (-2.25, 136.50),
        (-1.85, 138.80), (-2.55, 140.75), (-3.55, 143.65), (-5.20, 145.80), (-6.65, 147.85),
        (-8.55, 148.45), (-10.45, 150.65), (-9.95, 147.60), (-7.95, 144.80), (-9.15, 142.85),
        (-8.65, 140.65), (-8.20, 138.05), (-5.95, 138.25), (-4.55, 136.40), (-4.05, 134.45),
        (-3.35, 132.60), (-2.45, 132.35), (-1.95, 131.15),
    ],
    // Australia & Tasmania
    &[
        (-11.2, 130.6), (-12.3, 136.8), (-15.6, 136.5), (-17.5, 140.8), (-10.7, 142.5),
        (-16.5, 145.8), (-20.2, 148.9), (-25.2, 153.2), (-28.8, 153.6), (-33.9, 151.3),
        (-37.6, 149.9), (-38.8, 146.4), (-38.4, 141.5), (-35.6, 138.5), (-34.9, 135.6),
        (-31.7, 131.0), (-32.2, 126.0), (-34.0, 121.8), (-35.1, 117.9), (-34.2, 115.0),
        (-31.9, 115.7), (-26.5, 113.5), (-21.8, 114.1), (-19.8, 119.8), (-16.4, 123.0),
        (-14.0, 126.2),
    ],
    &[(-40.8, 144.7), (-41.0, 148.3), (-43.5, 147.2), (-43.1, 145.4)],
    // New Zealand (North & South Islands)
    &[(-34.4, 172.7), (-36.8, 175.8), (-37.7, 178.5), (-41.3, 175.3), (-41.1, 174.1), (-39.2, 173.8), (-36.8, 174.3)],
    &[(-40.6, 172.6), (-41.4, 174.1), (-43.8, 172.9), (-46.5, 168.5), (-45.8, 166.5), (-42.7, 171.0)],
    // Antarctica Continental Rim
    &[
        (-65.2, -179.9), (-77.5, -165.0), (-73.2, -100.0), (-63.5, -58.0),
        (-70.0, -5.0), (-68.5, 45.0), (-66.5, 95.0), (-66.5, 140.0),
        (-72.0, 170.0), (-65.2, 179.9), (-84.5, 179.9), (-84.5, -179.9),
    ],
];

/// Pure-Rust vector topographic mountain ridge spines `(lat_deg, lon_deg)` rendered subtly over continents.
pub const WORLD_MOUNTAIN_RIDGES: &[&[(f64, f64)]] = &[
    // Bukit Barisan (Sumatra)
    &[(5.1, 95.9), (3.2, 98.1), (1.1, 99.4), (-0.9, 100.6), (-2.8, 101.8), (-4.9, 104.2)],
    // Java Volcanic Arc (Salak -> Slamet -> Merapi -> Semeru -> Ijen)
    &[(-6.8, 106.8), (-7.2, 107.9), (-7.3, 109.2), (-7.55, 110.44), (-7.85, 111.9), (-8.1, 112.92), (-8.05, 114.2)],
    // Müller / Schwaner & Meratus Ranges (Kalimantan)
    &[(2.8, 115.6), (0.9, 114.5), (-0.7, 113.0), (-1.5, 111.5)],
    &[(-1.8, 115.9), (-2.9, 115.6), (-3.8, 115.2)],
    // Central Papua Jayawijaya / Maoke Range
    &[(-3.7, 135.5), (-4.08, 137.18), (-4.4, 139.5), (-5.2, 141.5), (-6.3, 145.0), (-8.5, 147.6)],
    // Himalayas & Tibetan Arc
    &[(35.5, 74.5), (32.2, 78.0), (28.5, 84.0), (27.98, 86.92), (27.6, 91.5), (29.2, 96.5)],
    // European Alps
    &[(44.5, 6.8), (46.2, 7.8), (46.8, 10.5), (47.2, 13.5), (47.6, 15.8)],
    // Rocky Mountains
    &[(59.0, -127.0), (52.0, -117.5), (46.0, -112.0), (40.0, -106.0), (35.0, -105.5)],
    // Andes Cordillera
    &[(9.0, -72.0), (0.0, -78.2), (-10.0, -77.2), (-18.0, -69.0), (-32.6, -70.0), (-45.0, -72.5), (-53.0, -71.5)],
    // Japanese Alps
    &[(37.0, 138.0), (36.2, 137.6), (35.36, 138.73)],
    // East African Rift Highlands
    &[(9.0, 39.0), (0.0, 36.2), (-3.07, 37.35), (-8.5, 34.5)],
];

/// Pure-Rust vector major hydrological river ribbons `(lat_deg, lon_deg)`.
pub const WORLD_MAJOR_RIVERS: &[&[(f64, f64)]] = &[
    // Kapuas (West Kalimantan)
    &[(1.1, 113.8), (0.6, 112.2), (0.2, 110.8), (-0.05, 109.35)],
    // Mahakam (East Kalimantan / IKN corridor)
    &[(1.0, 114.8), (0.2, 115.7), (-0.3, 116.5), (-0.52, 117.25)],
    // Barito (South Kalimantan)
    &[(-0.3, 114.6), (-1.4, 114.9), (-2.6, 114.7), (-3.55, 114.5)],
    // Musi (South Sumatra)
    &[(-3.6, 102.7), (-3.1, 103.8), (-2.98, 104.75), (-2.35, 104.95)],
    // Bengawan Solo & Brantas (Java)
    &[(-7.8, 110.9), (-7.4, 111.4), (-7.15, 111.9), (-6.92, 112.55)],
    &[(-8.0, 112.4), (-7.8, 112.0), (-7.45, 112.45), (-7.26, 112.8)],
    // Mekong (Southeast Asia)
    &[(25.0, 99.5), (20.2, 100.5), (17.9, 102.6), (15.1, 105.8), (11.5, 104.9), (9.8, 106.5)],
    // Yangtze
    &[(33.0, 96.0), (28.5, 104.0), (30.6, 114.3), (31.4, 121.6)],
    // Ganges
    &[(30.0, 78.3), (26.5, 80.5), (25.6, 85.1), (22.5, 89.2)],
    // Nile
    &[(0.4, 33.2), (9.5, 31.6), (15.6, 32.5), (24.0, 32.9), (30.1, 31.2), (31.3, 31.0)],
    // Amazon
    &[(-4.5, -73.5), (-3.5, -66.0), (-3.1, -60.0), (-2.0, -54.5), (-0.2, -49.6)],
    // Mississippi
    &[(47.2, -95.2), (41.5, -90.5), (35.1, -90.1), (29.9, -90.0), (29.1, -89.3)],
    // Danube
    &[(48.0, 8.5), (48.2, 16.4), (47.5, 19.0), (44.8, 20.4), (44.0, 26.0), (45.2, 29.5)],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase64_legacy_tile_source_serde_aliases_map_to_vector_earth_theme() {
        let cases = [
            ("\"EsriWorldImagery\"", VectorEarthTheme::CoastalBlueprint),
            ("\"SyntheticVectorTerrain\"", VectorEarthTheme::CoastalBlueprint),
            ("\"OpenStreetMap\"", VectorEarthTheme::CityDaylight),
            ("\"CartoDbDarkMatter\"", VectorEarthTheme::NightTwin),
            ("\"OpenTopoMap\"", VectorEarthTheme::Topographic),
            ("\"CoastalBlueprint\"", VectorEarthTheme::CoastalBlueprint),
            ("\"CityDaylight\"", VectorEarthTheme::CityDaylight),
            ("\"NightTwin\"", VectorEarthTheme::NightTwin),
            ("\"Topographic\"", VectorEarthTheme::Topographic),
        ];
        for (raw, expected) in cases {
            let parsed: VectorEarthTheme =
                serde_json::from_str(raw).expect("Legacy TileSource JSON should deserialize cleanly");
            assert_eq!(parsed, expected, "Mismatch for legacy payload {raw}");
        }
    }

    #[test]
    fn test_phase64_embedded_world_vectors_validity_and_theme_cycling() {
        assert!(
            WORLD_LANDMASSES.len() >= 30,
            "Expected detailed global + Indonesian archipelago polygons, got {}",
            WORLD_LANDMASSES.len()
        );
        for (idx, poly) in WORLD_LANDMASSES.iter().enumerate() {
            assert!(poly.len() >= 3, "Landmass polygon #{idx} has < 3 vertices");
            for &(lat, lon) in *poly {
                assert!((-90.0..=90.0).contains(&lat), "Invalid lat {lat} in polygon #{idx}");
                assert!((-180.0..=180.0).contains(&lon), "Invalid lon {lon} in polygon #{idx}");
            }
        }
        assert!(WORLD_MOUNTAIN_RIDGES.len() >= 10);
        assert!(WORLD_MAJOR_RIVERS.len() >= 10);

        let mut theme = VectorEarthTheme::default();
        assert_eq!(theme, VectorEarthTheme::CoastalBlueprint);
        let mut seen = Vec::new();
        for _ in 0..4 {
            let _pal = theme.palette();
            seen.push(theme);
            theme = theme.cycle_next();
        }
        assert_eq!(theme, VectorEarthTheme::CoastalBlueprint);
        assert_eq!(seen.len(), 4);
    }
}
