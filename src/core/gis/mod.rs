//! gis/mod.rs — GIS Site Inception & Geotechnical Foundation Suite (Phases 23–26).
//!
//! Provides Web Mercator Slippy Map Quadtree Engine (`walkers`), Geodetic CRS transformations (`proj4rs`),
//! Digital Elevation Model (DEM) TIN Surface Meshing, Borehole Stratigraphy, and PusGeN Seismic Spectrum Integration.

pub mod map_widget;
pub mod crs_projection;
pub mod tin_surface;
pub mod earth_globe;
pub mod world_index;
pub mod vector_layers;
pub mod dem_stream;
pub mod neighborhood_physics;
pub mod world_vectors;

pub use world_vectors::{
    TileSource, VectorEarthPalette, VectorEarthTheme, WORLD_LANDMASSES, WORLD_MAJOR_RIVERS,
    WORLD_MOUNTAIN_RIDGES,
};

pub use map_widget::{
    SiteGeodeticPin, SlippyMapState, SlippyMapTelemetry, WebMercator,
    EARTH_HALF_CIRCUMFERENCE, EARTH_RADIUS_METERS, GLOBAL_CITY_CATALOG, MAX_LATITUDE, MIN_LATITUDE,
};

pub use world_index::{
    GeoPlacementSource, WorldBuildingProxy, WorldClusterBeacon, WorldPortfolioFilter,
    WorldPortfolioSummary, WorldSpatialIndex, STAGING_CAMPUS_ORIGIN, WORLD_INDEX_FILENAME,
};

pub use vector_layers::{
    parse_geojson_features, UrbanVectorFeature, UrbanVectorKind, UrbanVectorLayerCache,
};

pub use dem_stream::{build_tin_from_3d_triangles, parse_survey_xyz_to_tin};

pub use neighborhood_physics::{
    bim_footprint_hull_m, evaluate_neighborhood_coupling, polygon_min_distance_m,
    NeighborPoundingContact, NeighborhoodAuditReport, PoundingSeverity,
};

pub use earth_globe::{
    estimate_baseline_elevation_m, great_circle_arc, is_point_on_landmass,
    lat_lon_to_unit_sphere, unit_sphere_to_lat_lon, EarthBiome, EarthBrushAction,
    EarthCellMod, EarthMegastructure, EarthScenario, GlobeCamera, GlobeFlyTarget,
    GlobeScreenTransform, MegastructureKind, PlanetaryVitals, PlanetaryWorldState,
};

pub use crs_projection::{
    GeoProjectionEngine, IfcMapConversionMatrix, UtmZoneInfo,
    UTM_FALSE_EASTING, UTM_FALSE_NORTHING_SOUTH, UTM_SCALE_FACTOR_CENTRAL,
    WGS84_FIRST_ECCENTRICITY_SQ, WGS84_FLATTENING, WGS84_SECOND_ECCENTRICITY_SQ,
    WGS84_SEMI_MAJOR_AXIS, WGS84_SEMI_MINOR_AXIS,
};

pub use tin_surface::{
    ContourPolyline, CutFillVolumeReport, DemTerrainPreset, TinPoint, TinSurfaceMesh, TinTriangle,
};
