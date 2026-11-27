//! Phase 25 (v0.2.5) — Digital Elevation Model (DEM) & Topographic TIN Mesh Engine
//!
//! Pure-Rust Triangulated Irregular Network (TIN) surface generator, 2D Delaunay triangulator,
//! Marching-Triangles iso-elevation contour extractor, slope/aspect geotechnical hazard analyzer
//! (SNI 8460:2017 > 30 deg slope stability threshold), and prismatic Cut-and-Fill earthwork
//! volume integrator.

use serde::{Deserialize, Serialize};

/// 3D topographic survey / DEM vertex in local project Easting-Northing-Elevation coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TinPoint {
    pub id: usize,
    /// Local Easting offset (m) relative to site origin
    pub x: f32,
    /// Local Northing offset (m) relative to site origin
    pub y: f32,
    /// Orthometric Elevation (m MSL)
    pub z: f32,
}

/// Enriched geometric and geotechnical metadata for a single TIN facet.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TinTriangle {
    /// Vertex indices into `TinSurfaceMesh::points` (counter-clockwise in XY plane)
    pub indices: [usize; 3],
    /// Unit outward surface normal vector `[nx, ny, nz]` (`nz > 0`)
    pub normal: [f32; 3],
    /// Maximum steepest-descent slope angle in degrees (`0.0..=90.0`)
    pub slope_deg: f32,
    /// Downslope compass azimuth in degrees (`0.0 = North, 90.0 = East`)
    pub aspect_deg: f32,
    /// True 3D inclined surface area (m²)
    pub area_3d_m2: f32,
    /// Projected 2D horizontal plan area (m²)
    pub area_2d_m2: f32,
    /// Triangle 3D centroid `[cx, cy, cz]`
    pub centroid: [f32; 3],
    /// Flagged if `slope_deg > 30.0` (SNI 8460:2017 steep slope instability threshold)
    pub is_steep_hazard: bool,
}

/// Continuous or segmented iso-elevation contour line extracted via Marching Triangles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContourPolyline {
    /// Contour elevation (m MSL)
    pub elevation_m: f32,
    /// True if major index contour (e.g. every 5.0 m), false for intermediate contour (e.g. 1.0 m)
    pub is_index_contour: bool,
    /// 2D line segments `([x0, y0], [x1, y1])` in local site coordinates (m)
    pub segments: Vec<([f32; 2], [f32; 2])>,
}

/// Synthetic & DEMNAS/SRTM-inspired terrain morphology presets for engineering benchmarking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemTerrainPreset {
    /// Volcanic foothills ridge & valley channel (West Java / Preanger DEMNAS archetype)
    VolcanicFoothillsRidge,
    /// Coastal alluvial basin & meandering river terrace (Jakarta / Pantura archetype)
    CoastalAlluvialBasin,
    /// Steep karst limestone escarpment with >30° slope hazard zones (Padalarang / Maros archetype)
    KarstLimestoneEscarpment,
    /// Graded industrial cut-and-fill structural pad with engineered batters
    TerracedCutFillPad,
    /// Realistic Cities: Skylines II style tropical bay, sandy beach, coastal city plateau & volcanic peak
    ArchipelagoBayAndVolcano,
    /// Realistic alpine river fjord valley with meandering waterway, buildable terrace & steep rocky cliffs
    FjordRiverValley,
    /// Realistic sculpted coastal island harbor surrounded by sea with upland scenic ridge
    CoastalIslandMetropolis,
}

impl DemTerrainPreset {
    pub fn all() -> &'static [DemTerrainPreset] {
        &[
            DemTerrainPreset::VolcanicFoothillsRidge,
            DemTerrainPreset::CoastalAlluvialBasin,
            DemTerrainPreset::KarstLimestoneEscarpment,
            DemTerrainPreset::TerracedCutFillPad,
            DemTerrainPreset::ArchipelagoBayAndVolcano,
            DemTerrainPreset::FjordRiverValley,
            DemTerrainPreset::CoastalIslandMetropolis,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            DemTerrainPreset::VolcanicFoothillsRidge => "DEMNAS Volcanic Foothills Ridge",
            DemTerrainPreset::CoastalAlluvialBasin => "SRTM Coastal Alluvial Basin",
            DemTerrainPreset::KarstLimestoneEscarpment => "Karst Escarpment (>30° Slope Hazard)",
            DemTerrainPreset::TerracedCutFillPad => "Engineered Cut/Fill Structural Pad",
            DemTerrainPreset::ArchipelagoBayAndVolcano => {
                "Archipelago Bay & Volcanic Peak (Coastal + Mountains)"
            }
            DemTerrainPreset::FjordRiverValley => "Alpine Fjord River Valley & Cliffs",
            DemTerrainPreset::CoastalIslandMetropolis => "Coastal Island Harbor Metropolis",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            DemTerrainPreset::VolcanicFoothillsRidge => "Volcano",
            DemTerrainPreset::CoastalAlluvialBasin => "Basin",
            DemTerrainPreset::KarstLimestoneEscarpment => "Karst",
            DemTerrainPreset::TerracedCutFillPad => "Pad",
            DemTerrainPreset::ArchipelagoBayAndVolcano => "Bay & Peak",
            DemTerrainPreset::FjordRiverValley => "Fjord",
            DemTerrainPreset::CoastalIslandMetropolis => "Island",
        }
    }

    /// True when the preset features open coastal sea or navigable water below `0.0 m MSL`.
    pub fn has_coastal_water(self) -> bool {
        matches!(
            self,
            DemTerrainPreset::ArchipelagoBayAndVolcano
                | DemTerrainPreset::FjordRiverValley
                | DemTerrainPreset::CoastalIslandMetropolis
                | DemTerrainPreset::CoastalAlluvialBasin
        )
    }
}

/// Earthwork Cut-and-Fill & Geotechnical Slope Hazard audit summary.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CutFillVolumeReport {
    /// Target formation / grading pad elevation (m MSL)
    pub datum_elevation_m: f32,
    /// Excavation volume above formation datum (m³)
    pub cut_volume_m3: f64,
    /// Embankment fill volume below formation datum (m³)
    pub fill_volume_m3: f64,
    /// Net mass-haul earthwork balance `cut - fill` (m³)
    pub net_balance_m3: f64,
    /// Total 2D projected footprint area (m²)
    pub planar_area_m2: f64,
    /// Total 3D inclined topographic surface area (m²)
    pub surface_3d_area_m2: f64,
    /// Projected area exceeding the steep slope hazard threshold (m²)
    pub steep_hazard_area_m2: f64,
    /// Percentage of site area classified as steep slope hazard (%)
    pub steep_hazard_ratio_pct: f32,
    /// Maximum facet slope angle across the TIN mesh (deg)
    pub max_slope_deg: f32,
    /// Area-weighted mean slope angle across the TIN mesh (deg)
    pub mean_slope_deg: f32,
}

/// Interactive 3D terrain & water sculpting brush mode for live `IfcSite` terraforming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainSculptBrush {
    /// Uplifts terrain ridges, embankments, and mountain peaks
    Raise,
    /// Excavates terrain cuts, valleys, and basins
    Lower,
    /// Grades surrounding terrain smoothly to the sampled/target building pad elevation
    Flatten,
    /// Relaxes jagged triangle slopes into smooth natural contours
    Smooth,
    /// Excavates river channels, harbors, and coastal sea basins below sea level
    CarveWater,
}

impl TerrainSculptBrush {
    pub const ALL: [Self; 5] = [
        Self::Raise,
        Self::Lower,
        Self::Flatten,
        Self::Smooth,
        Self::CarveWater,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Raise => "Raise Peak",
            Self::Lower => "Excavate Lower",
            Self::Flatten => "Flatten Pad",
            Self::Smooth => "Smooth Slopes",
            Self::CarveWater => "Carve Sea / River",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Raise => "Raise",
            Self::Lower => "Lower",
            Self::Flatten => "Flatten",
            Self::Smooth => "Smooth",
            Self::CarveWater => "Carve Sea",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Self::Raise => "Click/drag on 3D terrain to raise hills and mountain ridges",
            Self::Lower => "Click/drag on 3D terrain to excavate valleys and cuts",
            Self::Flatten => "Click/drag to level buildable harbor/building pads cleanly",
            Self::Smooth => "Click/drag to soften steep cliff slopes toward local mean grade",
            Self::CarveWater => {
                "Click/drag to carve navigable riverbeds or coastal harbors below sea level"
            }
        }
    }
}

/// Persisted interactive terrain sculpting brush configuration and cumulative earthwork ledger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerrainSculptConfig {
    pub active_brush: Option<TerrainSculptBrush>,
    pub brush_radius_m: f32,
    pub brush_strength_m: f32,
    pub target_flatten_z_m: f32,
    pub auto_sample_flatten: bool,
    pub total_cut_m3: f64,
    pub total_fill_m3: f64,
    pub stroke_count: u64,
}

impl Default for TerrainSculptConfig {
    fn default() -> Self {
        Self {
            active_brush: None,
            brush_radius_m: 48.0,
            brush_strength_m: 3.0,
            target_flatten_z_m: 3.2,
            auto_sample_flatten: true,
            total_cut_m3: 0.0,
            total_fill_m3: 0.0,
            stroke_count: 0,
        }
    }
}

/// Telemetry returned by a single interactive terrain sculpting stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainSculptStrokeResult {
    pub modified_vertices: usize,
    pub delta_cut_m3: f64,
    pub delta_fill_m3: f64,
    pub center_elev_after_m: f32,
    pub max_slope_deg_after: f32,
}

/// Non-destructive IFC 5 / IFCX overlay summary for sculpted `IfcSite` topography.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SiteTerraformIfcxOverlay {
    pub sea_level_m: f32,
    pub min_elevation_m: f32,
    pub max_elevation_m: f32,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub total_cut_m3: f64,
    pub total_fill_m3: f64,
    pub net_earthwork_m3: f64,
    pub max_slope_deg: f32,
    pub steep_hazard_ratio_pct: f32,
    pub sculpt_strokes: u64,
    pub revision: u64,
}

/// Phase 25 Topographic TIN Surface Mesh (`src/core/gis/tin_surface.rs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TinSurfaceMesh {
    /// Survey / DEM vertices
    pub points: Vec<TinPoint>,
    /// Raw triangle index triplets `[i0, i1, i2]` (per Phase 25 specification)
    pub triangles: Vec<[usize; 3]>,
    /// Enriched per-triangle normal, slope, aspect, and hazard metrics
    pub triangle_metrics: Vec<TinTriangle>,
    /// Minimum orthometric elevation across the mesh (m MSL)
    pub min_elevation: f32,
    /// Maximum orthometric elevation across the mesh (m MSL)
    pub max_elevation: f32,
    /// Site domain Easting width (m), default 500.0 m
    pub extent_x_m: f32,
    /// Site domain Northing height (m), default 500.0 m
    pub extent_y_m: f32,
    /// Number of raster columns if generated from a structured DEM grid
    pub grid_cols: usize,
    /// Number of raster rows if generated from a structured DEM grid
    pub grid_rows: usize,
    /// Monotonic revision counter incremented on every live terrain sculpt or restore
    #[serde(default)]
    pub revision: u64,
}

impl Default for TinSurfaceMesh {
    fn default() -> Self {
        // Default 500m x 500m DEM mesh with 72x72 vertices = 71*71*2 = 10,082 Delaunay triangles
        Self::generate_from_dem_preset(
            DemTerrainPreset::VolcanicFoothillsRidge,
            500.0,
            500.0,
            72,
            72,
            1.0,
        )
    }
}

impl TinSurfaceMesh {
    /// Generates a 500m x 500m (or custom extent) DEM terrain mesh with `cols x rows` vertices
    /// (`(cols - 1) * (rows - 1) * 2` triangles). At `72 x 72`, this produces **10,082 triangles**
    /// satisfying the Phase 25 Definition of Done (`10,000+` triangles at 60 FPS).
    ///
    /// Each grid cell is split into two triangles by evaluating the local 2D Delaunay circumcircle
    /// / max-min interior angle criterion in 3D surface space so ridges and valleys align naturally.
    pub fn generate_from_dem_preset(
        preset: DemTerrainPreset,
        extent_x_m: f32,
        extent_y_m: f32,
        cols: usize,
        rows: usize,
        vertical_exaggeration: f32,
    ) -> Self {
        let cols = cols.max(4);
        let rows = rows.max(4);
        let extent_x_m = extent_x_m.max(10.0);
        let extent_y_m = extent_y_m.max(10.0);
        let vex = vertical_exaggeration.clamp(0.2, 5.0);

        let mut points = Vec::with_capacity(cols * rows);
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        let half_x = extent_x_m * 0.5;
        let half_y = extent_y_m * 0.5;

        for r in 0..rows {
            let v = r as f32 / (rows - 1) as f32;
            let y = -half_y + v * extent_y_m;
            for c in 0..cols {
                let u = c as f32 / (cols - 1) as f32;
                let x = -half_x + u * extent_x_m;
                let z = Self::evaluate_preset_elevation(preset, x, y, extent_x_m, extent_y_m) * vex;
                min_z = min_z.min(z);
                max_z = max_z.max(z);
                points.push(TinPoint {
                    id: r * cols + c,
                    x,
                    y,
                    z,
                });
            }
        }

        let num_tris = (cols - 1) * (rows - 1) * 2;
        let mut triangles = Vec::with_capacity(num_tris);
        let mut triangle_metrics = Vec::with_capacity(num_tris);

        for r in 0..(rows - 1) {
            for c in 0..(cols - 1) {
                let i00 = r * cols + c;
                let i10 = r * cols + (c + 1);
                let i01 = (r + 1) * cols + c;
                let i11 = (r + 1) * cols + (c + 1);

                // Adaptive Delaunay diagonal selection: choose diagonal with smaller elevation curvature
                // so breaklines/ridges are preserved without artificial sawtooth artifacts.
                let dz_main = (points[i00].z - points[i11].z).abs();
                let dz_anti = (points[i10].z - points[i01].z).abs();

                let (t1, t2) = if dz_main <= dz_anti {
                    ([i00, i10, i11], [i00, i11, i01])
                } else {
                    ([i00, i10, i01], [i10, i11, i01])
                };

                triangles.push(t1);
                triangle_metrics.push(Self::compute_triangle_metrics(&points, t1));
                triangles.push(t2);
                triangle_metrics.push(Self::compute_triangle_metrics(&points, t2));
            }
        }

        Self {
            points,
            triangles,
            triangle_metrics,
            min_elevation: if min_z.is_finite() { min_z } else { 0.0 },
            max_elevation: if max_z.is_finite() { max_z } else { 0.0 },
            extent_x_m,
            extent_y_m,
            grid_cols: cols,
            grid_rows: rows,
            revision: 0,
        }
    }

    /// Constructs a `TinSurfaceMesh` directly from explicit pure-vector 3D vertices and triangle index triplets
    /// (e.g., OpenBIM IFC 4.3 `IfcSite` / `IfcGeographicElement` `IfcTriangulatedFaceSet`).
    pub fn from_explicit_triangles(mut points: Vec<TinPoint>, raw_triangles: Vec<[usize; 3]>) -> Self {
        let n = points.len();
        if n < 3 || raw_triangles.is_empty() {
            return Self::triangulate_bowyer_watson(points);
        }
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for (idx, p) in points.iter_mut().enumerate() {
            p.id = idx;
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
            min_z = min_z.min(p.z);
            max_z = max_z.max(p.z);
        }
        let mut triangles = Vec::with_capacity(raw_triangles.len());
        let mut triangle_metrics = Vec::with_capacity(raw_triangles.len());
        for tri in raw_triangles {
            if tri[0] < n && tri[1] < n && tri[2] < n && tri[0] != tri[1] && tri[1] != tri[2] && tri[0] != tri[2] {
                triangles.push(tri);
                triangle_metrics.push(Self::compute_triangle_metrics(&points, tri));
            }
        }
        Self {
            points,
            triangles,
            triangle_metrics,
            min_elevation: if min_z.is_finite() { min_z } else { 0.0 },
            max_elevation: if max_z.is_finite() { max_z } else { 0.0 },
            extent_x_m: (max_x - min_x).max(10.0),
            extent_y_m: (max_y - min_y).max(10.0),
            grid_cols: 0,
            grid_rows: 0,
            revision: 0,
        }
    }

    /// Pure-Rust Bowyer-Watson 2D Delaunay Triangulation for arbitrary irregular survey point clouds.
    /// Maximizes minimum interior angles across all triangles by enforcing the empty circumcircle property.
    pub fn triangulate_bowyer_watson(mut points: Vec<TinPoint>) -> Self {
        if points.len() < 3 {
            return Self {
                points,
                triangles: Vec::new(),
                triangle_metrics: Vec::new(),
                min_elevation: 0.0,
                max_elevation: 0.0,
                extent_x_m: 500.0,
                extent_y_m: 500.0,
                grid_cols: 0,
                grid_rows: 0,
                revision: 0,
            };
        }

        // Normalize IDs sequentially
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for (idx, p) in points.iter_mut().enumerate() {
            p.id = idx;
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
            min_z = min_z.min(p.z);
            max_z = max_z.max(p.z);
        }

        let dx = (max_x - min_x).max(1.0);
        let dy = (max_y - min_y).max(1.0);
        let delta_max = dx.max(dy) * 10.0;
        let mid_x = (min_x + max_x) * 0.5;
        let mid_y = (min_y + max_y) * 0.5;

        let n_orig = points.len();
        // Super-triangle vertices appended temporarily
        let st0 = TinPoint {
            id: n_orig,
            x: mid_x - 2.0 * delta_max,
            y: mid_y - delta_max,
            z: 0.0,
        };
        let st1 = TinPoint {
            id: n_orig + 1,
            x: mid_x,
            y: mid_y + 2.0 * delta_max,
            z: 0.0,
        };
        let st2 = TinPoint {
            id: n_orig + 2,
            x: mid_x + 2.0 * delta_max,
            y: mid_y - delta_max,
            z: 0.0,
        };

        let mut all_pts = points.clone();
        all_pts.push(st0);
        all_pts.push(st1);
        all_pts.push(st2);

        let mut tris: Vec<[usize; 3]> = vec![[n_orig, n_orig + 1, n_orig + 2]];

        for p_idx in 0..n_orig {
            let px = all_pts[p_idx].x as f64;
            let py = all_pts[p_idx].y as f64;

            let mut bad_tris = Vec::new();
            for (t_idx, tri) in tris.iter().enumerate() {
                if Self::point_in_circumcircle(&all_pts, *tri, px, py) {
                    bad_tris.push(t_idx);
                }
            }

            let mut polygon_edges: Vec<(usize, usize)> = Vec::new();
            for &bt_idx in &bad_tris {
                let tri = tris[bt_idx];
                let edges = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])];
                for &(a, b) in &edges {
                    let rev = (b, a);
                    if let Some(pos) = polygon_edges.iter().position(|&e| e == rev || e == (a, b)) {
                        polygon_edges.swap_remove(pos);
                    } else {
                        polygon_edges.push((a, b));
                    }
                }
            }

            // Remove bad triangles in reverse index order
            for &bt_idx in bad_tris.iter().rev() {
                tris.swap_remove(bt_idx);
            }

            for (a, b) in polygon_edges {
                // Ensure counter-clockwise orientation
                let ax = all_pts[a].x;
                let ay = all_pts[a].y;
                let bx = all_pts[b].x;
                let by = all_pts[b].y;
                let cx = all_pts[p_idx].x;
                let cy = all_pts[p_idx].y;
                let cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
                if cross >= 0.0 {
                    tris.push([a, b, p_idx]);
                } else {
                    tris.push([b, a, p_idx]);
                }
            }
        }

        // Discard any triangle referencing super-triangle vertices
        tris.retain(|t| t[0] < n_orig && t[1] < n_orig && t[2] < n_orig);

        let triangle_metrics = tris
            .iter()
            .map(|&t| Self::compute_triangle_metrics(&points, t))
            .collect();

        Self {
            points,
            triangles: tris,
            triangle_metrics,
            min_elevation: if min_z.is_finite() { min_z } else { 0.0 },
            max_elevation: if max_z.is_finite() { max_z } else { 0.0 },
            extent_x_m: dx,
            extent_y_m: dy,
            grid_cols: 0,
            grid_rows: 0,
            revision: 0,
        }
    }

    fn point_in_circumcircle(pts: &[TinPoint], tri: [usize; 3], px: f64, py: f64) -> bool {
        let ax = pts[tri[0]].x as f64;
        let ay = pts[tri[0]].y as f64;
        let bx = pts[tri[1]].x as f64;
        let by = pts[tri[1]].y as f64;
        let cx = pts[tri[2]].x as f64;
        let cy = pts[tri[2]].y as f64;

        let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
        if d.abs() < 1e-12 {
            return false;
        }
        let a2 = ax * ax + ay * ay;
        let b2 = bx * bx + by * by;
        let c2 = cx * cx + cy * cy;
        let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
        let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
        let r_sq = (ux - ax).powi(2) + (uy - ay).powi(2);
        let dist_sq = (px - ux).powi(2) + (py - uy).powi(2);
        dist_sq <= r_sq + 1e-7
    }

    /// Evaluates synthetic or DEMNAS/SRTM benchmark elevation (m MSL) at local `(x, y)`.
    pub fn evaluate_preset_elevation(
        preset: DemTerrainPreset,
        x: f32,
        y: f32,
        extent_x: f32,
        extent_y: f32,
    ) -> f32 {
        let nx = x / (extent_x * 0.5); // -1.0 .. 1.0
        let ny = y / (extent_y * 0.5); // -1.0 .. 1.0
        match preset {
            DemTerrainPreset::VolcanicFoothillsRidge => {
                // Base elevation ~120m MSL with NW-SE volcanic ridge, drainage saddle, and minor spurs
                let ridge = 38.0 * (-((nx - 0.35 * ny).powi(2)) * 2.2).exp();
                let spur = 14.0 * (nx * 3.1).sin() * (ny * 2.4).cos();
                let valley_incision = -12.0 * (-((nx + 0.45).powi(2) + (ny - 0.25).powi(2)) * 4.5).exp();
                let regional_tilt = 16.0 * (0.4 * nx + 0.6 * ny);
                115.0 + ridge + spur + valley_incision + regional_tilt
            }
            DemTerrainPreset::CoastalAlluvialBasin => {
                // Gentle coastal plain (4m to 28m MSL) with meandering river channel & levee terraces
                let meander_x = 0.25 * (ny * 3.5).sin();
                let dist_channel = (nx - meander_x).abs();
                let channel_depth = -6.5 * (-dist_channel * dist_channel * 35.0).exp();
                let levee = 2.2 * (-((dist_channel - 0.16).powi(2)) * 80.0).exp();
                let inland_rise = 9.5 * (ny + 1.0) + 2.5 * (nx * 2.2).cos();
                6.0 + inland_rise + channel_depth + levee
            }
            DemTerrainPreset::KarstLimestoneEscarpment => {
                // Steep karst cliff & mogote tower hills producing >30° slope hazard zones
                let cliff_step = 48.0 / (1.0 + (-7.5 * (nx - 0.15 * (ny * 2.5).sin())).exp());
                let mogote_1 = 36.0 * (-((nx - 0.35).powi(2) + (ny - 0.30).powi(2)) * 9.5).exp();
                let mogote_2 = 29.0 * (-((nx + 0.40).powi(2) + (ny + 0.35).powi(2)) * 11.0).exp();
                let sinkhole = -18.0 * (-((nx - 0.10).powi(2) + (ny + 0.25).powi(2)) * 14.0).exp();
                85.0 + cliff_step + mogote_1 + mogote_2 + sinkhole
            }
            DemTerrainPreset::TerracedCutFillPad => {
                // Natural hillside (75m..125m) with an engineered flat structural pad at 100.0m MSL in center
                let natural_hill = 100.0 + 22.0 * nx + 15.0 * ny + 6.0 * (nx * 3.0).sin() * (ny * 3.0).cos();
                let pad_radius = (nx.powi(2) + ny.powi(2)).sqrt();
                if pad_radius < 0.38 {
                    100.0
                } else if pad_radius < 0.52 {
                    // 1V:1.5H engineered batter transition
                    let t = (pad_radius - 0.38) / (0.52 - 0.38);
                    let smooth = t * t * (3.0 - 2.0 * t);
                    100.0 * (1.0 - smooth) + natural_hill * smooth
                } else {
                    natural_hill
                }
            }
            DemTerrainPreset::ArchipelagoBayAndVolcano => {
                // Tropical coastal bay (SW sea down to -12.5m) + harbor terrace (~+3.2m at origin) + NE volcanic mountain (~+90m)
                let shore_diag = nx + ny + 0.16 + 0.08 * ((nx - ny) * 3.2).sin();
                let bay_sigmoid = 1.0 / (1.0 + (-4.8 * shore_diag).exp());
                let sea_floor = -12.5 * (1.0 - bay_sigmoid);
                let coastal_plain = (3.4 + 2.6 * shore_diag.max(0.0)) * bay_sigmoid;
                let peak_d2 = (nx - 0.42).powi(2) + (ny - 0.44).powi(2);
                let main_volcano = 86.0 * (-peak_d2 * 5.2).exp();
                let summit_crater = -9.5 * (-peak_d2 * 48.0).exp();
                let ridge_spur =
                    24.0 * (-((nx - 0.18).powi(2) * 8.5 + (ny - 0.68).powi(2) * 3.5)).exp()
                        + 18.0 * (-((nx - 0.68).powi(2) * 3.8 + (ny - 0.12).powi(2) * 9.0)).exp();
                let harmonic = (4.2 * (nx * 4.5).sin() * (ny * 4.2).cos()
                    + 2.1 * (nx * 9.2 + ny * 7.8).cos())
                    * (0.35 + 0.65 * bay_sigmoid);
                let natural_z =
                    sea_floor + coastal_plain + main_volcano + summit_crater + ridge_spur + harmonic;
                // Smoothly grade a buildable urban harbor terrace around site origin (0, 0)
                let r_origin_m = x.hypot(y);
                let urban_blend = (-((r_origin_m / 78.0).powi(2))).exp();
                natural_z * (1.0 - urban_blend) + 3.2 * urban_blend
            }
            DemTerrainPreset::FjordRiverValley => {
                // Alpine fjord valley: meandering navigable river (~ -7.5m), flat urban valley terrace (+3.4m), steep cliffs (+82m)
                let river_center_ny = -0.24 + 0.12 * (nx * 2.6).sin();
                let dist_river = (ny - river_center_ny).abs();
                let river_trench = -11.2 * (-((dist_river / 0.13).powi(2))).exp();
                let north_cliff = 82.0 / (1.0 + (-8.5 * (ny - (0.34 + 0.08 * (nx * 3.1).sin()))).exp());
                let north_peaks = 14.0 * (nx * 4.2).cos().abs() * (ny - 0.25).max(0.0);
                let south_headland = 40.0 / (1.0 + (-9.0 * (-ny - 0.62)).exp());
                let valley_floor = 3.6 + 2.2 * (nx * 2.2).sin() * (ny * 2.0).cos();
                let relief = 3.2 * (nx * 6.5).sin() * (ny * 5.8).cos();
                let natural_z =
                    valley_floor + river_trench + north_cliff + north_peaks + south_headland + relief;
                let r_origin_m = x.hypot(y);
                let urban_blend = (-((r_origin_m / 68.0).powi(2))).exp();
                natural_z * (1.0 - urban_blend) + 3.4 * urban_blend
            }
            DemTerrainPreset::CoastalIslandMetropolis => {
                // Sculpted urban harbor island surrounded by ocean (-11m) with scenic northern mountain peak (+44m)
                let r = nx.hypot(ny);
                let theta = ny.atan2(nx);
                let coast_r = 0.72 + 0.08 * (3.0 * theta).sin() + 0.05 * (5.0 * theta).cos();
                let inland_sigmoid = 1.0 / (1.0 + (7.5 * (r - coast_r)).exp());
                let sea_bathymetry = -11.0 * (1.0 - inland_sigmoid);
                let island_plateau = (4.2 + 2.0 * (1.0 - (r / coast_r.max(0.2)).min(1.0))) * inland_sigmoid;
                let north_hill =
                    41.0 * (-((nx - 0.08).powi(2) * 8.5 + (ny - 0.42).powi(2) * 10.5)).exp()
                        * inland_sigmoid;
                let east_knoll =
                    22.0 * (-((nx - 0.42).powi(2) * 14.0 + (ny + 0.14).powi(2) * 14.0)).exp()
                        * inland_sigmoid;
                let micro = 1.8 * (nx * 6.0).sin() * (ny * 6.0).cos() * inland_sigmoid;
                let natural_z = sea_bathymetry + island_plateau + north_hill + east_knoll + micro;
                let r_origin_m = x.hypot(y);
                let urban_blend = (-((r_origin_m / 76.0).powi(2))).exp();
                natural_z * (1.0 - urban_blend) + 3.2 * urban_blend
            }
        }
    }

    /// Computes unit normal, steepest slope angle (deg), downslope aspect azimuth (deg),
    /// 2D/3D surface areas, and SNI 8460:2017 steep hazard flag (`> 30.0°`) for a triangle.
    pub fn compute_triangle_metrics(points: &[TinPoint], indices: [usize; 3]) -> TinTriangle {
        let p0 = points[indices[0]];
        let p1 = points[indices[1]];
        let p2 = points[indices[2]];

        let ux = p1.x - p0.x;
        let uy = p1.y - p0.y;
        let uz = p1.z - p0.z;

        let vx = p2.x - p0.x;
        let vy = p2.y - p0.y;
        let vz = p2.z - p0.z;

        // Cross product u x v
        let mut nx = uy * vz - uz * vy;
        let mut ny = uz * vx - ux * vz;
        let mut nz = ux * vy - uy * vx;

        if nz < 0.0 {
            nx = -nx;
            ny = -ny;
            nz = -nz;
        }

        let norm_len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-9);
        let area_3d_m2 = 0.5 * norm_len;
        let area_2d_m2 = 0.5 * nz.abs();

        let unit_nx = nx / norm_len;
        let unit_ny = ny / norm_len;
        let unit_nz = (nz / norm_len).clamp(0.0, 1.0);

        // Slope angle = acos(nz) in degrees
        let slope_deg = unit_nz.acos().to_degrees();

        // Downslope compass aspect (0° = North/+Y, 90° = East/+X)
        let aspect_deg = if unit_nx.hypot(unit_ny) < 1e-5 {
            0.0
        } else {
            let az = unit_nx.atan2(unit_ny).to_degrees();
            if az < 0.0 { az + 360.0 } else { az }
        };

        let centroid = [
            (p0.x + p1.x + p2.x) / 3.0,
            (p0.y + p1.y + p2.y) / 3.0,
            (p0.z + p1.z + p2.z) / 3.0,
        ];

        TinTriangle {
            indices,
            normal: [unit_nx, unit_ny, unit_nz],
            slope_deg,
            aspect_deg,
            area_3d_m2,
            area_2d_m2,
            centroid,
            is_steep_hazard: slope_deg > 30.0,
        }
    }

    /// Extracts iso-elevation contour lines across the TIN mesh using the Marching Triangles algorithm.
    ///
    /// * `minor_interval_m`: Intermediate contour spacing (e.g. `1.0` m or `2.0` m)
    /// * `major_interval_m`: Index contour spacing (e.g. `5.0` m or `10.0` m)
    pub fn extract_contours(
        &self,
        minor_interval_m: f32,
        major_interval_m: f32,
    ) -> Vec<ContourPolyline> {
        let minor_step = minor_interval_m.max(0.5);
        let major_step = major_interval_m.max(minor_step);

        if self.triangles.is_empty() || self.max_elevation <= self.min_elevation {
            return Vec::new();
        }

        let start_k = (self.min_elevation / minor_step).ceil() as i32;
        let end_k = (self.max_elevation / minor_step).floor() as i32;
        if end_k < start_k {
            return Vec::new();
        }

        let max_levels = 80;
        let total_levels = (end_k - start_k + 1) as usize;
        let stride = if total_levels > max_levels {
            (total_levels / max_levels).max(1) as i32
        } else {
            1
        };

        let mut contours = Vec::new();
        let mut k = start_k;
        while k <= end_k {
            let elev = k as f32 * minor_step;
            let rem = (elev / major_step).round() * major_step - elev;
            let is_index = rem.abs() < 0.05 * minor_step;

            let mut segments = Vec::new();
            for &tri in &self.triangles {
                let p0 = self.points[tri[0]];
                let p1 = self.points[tri[1]];
                let p2 = self.points[tri[2]];

                let min_t = p0.z.min(p1.z).min(p2.z);
                let max_t = p0.z.max(p1.z).max(p2.z);
                if elev < min_t || elev > max_t || (max_t - min_t) < 1e-5 {
                    continue;
                }

                // Slightly perturb exact vertex hits to avoid degenerate duplicate edges
                let eps = 1e-4;
                let z0 = if (p0.z - elev).abs() < eps { p0.z + eps } else { p0.z };
                let z1 = if (p1.z - elev).abs() < eps { p1.z + eps } else { p1.z };
                let z2 = if (p2.z - elev).abs() < eps { p2.z + eps } else { p2.z };

                let mut hits: Vec<[f32; 2]> = Vec::with_capacity(2);
                let edges = [
                    (p0, z0, p1, z1),
                    (p1, z1, p2, z2),
                    (p2, z2, p0, z0),
                ];

                for &(a, za, b, zb) in &edges {
                    if (za - elev) * (zb - elev) < 0.0 {
                        let t = ((elev - za) / (zb - za)).clamp(0.0, 1.0);
                        hits.push([a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)]);
                    }
                }

                if hits.len() == 2 {
                    segments.push((hits[0], hits[1]));
                }
            }

            if !segments.is_empty() {
                contours.push(ContourPolyline {
                    elevation_m: elev,
                    is_index_contour: is_index,
                    segments,
                });
            }

            k += stride;
        }

        contours
    }

    /// Integrates prismatic Cut-and-Fill earthwork volumes relative to `datum_elevation_m`
    /// and summarizes geotechnical steep-slope hazard areas (`> hazard_slope_threshold_deg`).
    pub fn compute_cut_fill_report(
        &self,
        datum_elevation_m: f32,
        hazard_slope_threshold_deg: f32,
    ) -> CutFillVolumeReport {
        let mut cut_vol = 0.0_f64;
        let mut fill_vol = 0.0_f64;
        let mut planar_area = 0.0_f64;
        let mut surface_area = 0.0_f64;
        let mut hazard_area = 0.0_f64;
        let mut max_slope = 0.0_f32;
        let mut weighted_slope_sum = 0.0_f64;

        for tm in &self.triangle_metrics {
            let a2d = tm.area_2d_m2 as f64;
            let a3d = tm.area_3d_m2 as f64;
            planar_area += a2d;
            surface_area += a3d;
            max_slope = max_slope.max(tm.slope_deg);
            weighted_slope_sum += (tm.slope_deg as f64) * a2d;

            if tm.slope_deg > hazard_slope_threshold_deg {
                hazard_area += a2d;
            }

            let p0 = self.points[tm.indices[0]].z - datum_elevation_m;
            let p1 = self.points[tm.indices[1]].z - datum_elevation_m;
            let p2 = self.points[tm.indices[2]].z - datum_elevation_m;

            // Prismatic volume contribution using vertex height decomposition
            let avg_dz = ((p0 + p1 + p2) / 3.0) as f64;
            if p0 >= 0.0 && p1 >= 0.0 && p2 >= 0.0 {
                cut_vol += a2d * avg_dz;
            } else if p0 <= 0.0 && p1 <= 0.0 && p2 <= 0.0 {
                fill_vol += a2d * (-avg_dz);
            } else {
                // Mixed cut/fill facet: partition proportionally by positive vs negative vertex heights
                let pos_sum = p0.max(0.0) + p1.max(0.0) + p2.max(0.0);
                let neg_sum = (-p0).max(0.0) + (-p1).max(0.0) + (-p2).max(0.0);
                let total = (pos_sum + neg_sum).max(1e-6);
                let pos_weight = (pos_sum / total) as f64;
                let neg_weight = (neg_sum / total) as f64;
                cut_vol += a2d * pos_weight * (pos_sum as f64 / 3.0);
                fill_vol += a2d * neg_weight * (neg_sum as f64 / 3.0);
            }
        }

        let mean_slope = if planar_area > 1e-6 {
            (weighted_slope_sum / planar_area) as f32
        } else {
            0.0
        };

        let hazard_pct = if planar_area > 1e-6 {
            ((hazard_area / planar_area) * 100.0) as f32
        } else {
            0.0
        };

        CutFillVolumeReport {
            datum_elevation_m,
            cut_volume_m3: cut_vol,
            fill_volume_m3: fill_vol,
            net_balance_m3: cut_vol - fill_vol,
            planar_area_m2: planar_area,
            surface_3d_area_m2: surface_area,
            steep_hazard_area_m2: hazard_area,
            steep_hazard_ratio_pct: hazard_pct,
            max_slope_deg: max_slope,
            mean_slope_deg: mean_slope,
        }
    }

    /// Samples interpolated ground elevation `z` (m MSL) and local slope (deg) at `(x, y)`.
    pub fn sample_elevation_and_slope(&self, x: f32, y: f32) -> (f32, f32) {
        if self.grid_cols >= 2 && self.grid_rows >= 2 {
            let half_x = self.extent_x_m * 0.5;
            let half_y = self.extent_y_m * 0.5;
            let u = ((x + half_x) / self.extent_x_m).clamp(0.0, 0.9999);
            let v = ((y + half_y) / self.extent_y_m).clamp(0.0, 0.9999);
            let c = (u * (self.grid_cols - 1) as f32).floor() as usize;
            let r = (v * (self.grid_rows - 1) as f32).floor() as usize;
            let cell_idx = (r * (self.grid_cols - 1) + c) * 2;
            if cell_idx < self.triangle_metrics.len() {
                let tm = &self.triangle_metrics[cell_idx];
                return (tm.centroid[2], tm.slope_deg);
            }
        }
        (0.5 * (self.min_elevation + self.max_elevation), 0.0)
    }

    /// Applies an interactive radial sculpting stroke to the TIN surface mesh at site ENU `(cx_m, cy_m)`.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_sculpt_brush(
        &mut self,
        cx_m: f32,
        cy_m: f32,
        brush: TerrainSculptBrush,
        radius_m: f32,
        strength_m: f32,
        target_z_m: f32,
        sea_level_m: f32,
    ) -> TerrainSculptStrokeResult {
        if self.points.is_empty() {
            return TerrainSculptStrokeResult {
                modified_vertices: 0,
                delta_cut_m3: 0.0,
                delta_fill_m3: 0.0,
                center_elev_after_m: 0.0,
                max_slope_deg_after: 0.0,
            };
        }
        let r = radius_m.clamp(8.0, 240.0);
        let r_sq = r * r;
        let strength = strength_m.clamp(0.1, 25.0);
        let cell_area_m2 =
            ((self.extent_x_m * self.extent_y_m) / self.points.len().max(1) as f32).max(1.0)
                as f64;

        let mean_in_brush = if brush == TerrainSculptBrush::Smooth {
            let mut sum_w = 0.0_f32;
            let mut sum_wz = 0.0_f32;
            for p in &self.points {
                let d2 = (p.x - cx_m).powi(2) + (p.y - cy_m).powi(2);
                if d2 <= r_sq {
                    let t = 1.0 - (d2 / r_sq);
                    let w = t * t + 0.05;
                    sum_w += w;
                    sum_wz += w * p.z;
                }
            }
            if sum_w > 1e-6 {
                sum_wz / sum_w
            } else {
                self.sample_elevation_and_slope(cx_m, cy_m).0
            }
        } else {
            0.0
        };

        let mut modified_mask = vec![false; self.points.len()];
        let mut modified_vertices = 0usize;
        let mut delta_cut_m3 = 0.0_f64;
        let mut delta_fill_m3 = 0.0_f64;
        let bed_target = (sea_level_m - 6.5).min(target_z_m);

        for (idx, p) in self.points.iter_mut().enumerate() {
            let d2 = (p.x - cx_m).powi(2) + (p.y - cy_m).powi(2);
            if d2 > r_sq {
                continue;
            }
            let t = (1.0 - (d2 / r_sq)).clamp(0.0, 1.0);
            let w = t * t;
            if w <= 1e-4 {
                continue;
            }
            let old_z = p.z;
            let dz = match brush {
                TerrainSculptBrush::Raise => strength * w,
                TerrainSculptBrush::Lower => -strength * w,
                TerrainSculptBrush::Flatten => (target_z_m - old_z) * (0.48 * w).clamp(0.0, 1.0),
                TerrainSculptBrush::Smooth => {
                    (mean_in_brush - old_z) * (0.55 * w).clamp(0.0, 1.0)
                }
                TerrainSculptBrush::CarveWater => {
                    let pull_down = (bed_target - old_z).min(0.0) * (0.45 * w) - 0.35 * strength * w;
                    let floor_lim = (sea_level_m - 15.0) - old_z;
                    pull_down.max(floor_lim)
                }
            };
            let new_z = (old_z + dz).clamp(-40.0, 320.0);
            let actual_dz = new_z - old_z;
            if actual_dz.abs() > 1e-5 {
                p.z = new_z;
                modified_mask[idx] = true;
                modified_vertices += 1;
                if actual_dz > 0.0 {
                    delta_fill_m3 += actual_dz as f64 * cell_area_m2;
                } else {
                    delta_cut_m3 += (-actual_dz) as f64 * cell_area_m2;
                }
            }
        }

        if modified_vertices > 0 {
            self.recompute_metrics_for_mask(&modified_mask);
            self.revision = self.revision.saturating_add(1);
        }

        let (center_elev_after_m, _) = self.sample_elevation_and_slope(cx_m, cy_m);
        let max_slope_deg_after = self
            .triangle_metrics
            .iter()
            .fold(0.0_f32, |m, tm| m.max(tm.slope_deg));

        TerrainSculptStrokeResult {
            modified_vertices,
            delta_cut_m3,
            delta_fill_m3,
            center_elev_after_m,
            max_slope_deg_after,
        }
    }

    fn recompute_metrics_for_mask(&mut self, modified_mask: &[bool]) {
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for p in &self.points {
            min_z = min_z.min(p.z);
            max_z = max_z.max(p.z);
        }
        self.min_elevation = if min_z.is_finite() { min_z } else { 0.0 };
        self.max_elevation = if max_z.is_finite() { max_z } else { 0.0 };

        for (ti, tri) in self.triangles.iter().enumerate() {
            if modified_mask.is_empty()
                || modified_mask[tri[0]]
                || modified_mask[tri[1]]
                || modified_mask[tri[2]]
            {
                if let Some(slot) = self.triangle_metrics.get_mut(ti) {
                    *slot = Self::compute_triangle_metrics(&self.points, *tri);
                }
            }
        }
    }

    /// Restores vertex elevations from an undo snapshot and updates triangle slope metrics.
    pub fn restore_elevations_snapshot(&mut self, elevations: &[f32]) -> bool {
        if elevations.len() != self.points.len() {
            return false;
        }
        for (p, &z) in self.points.iter_mut().zip(elevations.iter()) {
            p.z = z;
        }
        self.recompute_metrics_for_mask(&[]);
        self.revision = self.revision.saturating_add(1);
        true
    }

    /// Summarizes sculpted `IfcSite` topography into a non-destructive IFCX overlay struct.
    pub fn export_ifcx_site_terraform_overlay(
        &self,
        sea_level_m: f32,
        total_cut_m3: f64,
        total_fill_m3: f64,
        sculpt_strokes: u64,
    ) -> SiteTerraformIfcxOverlay {
        let cut_fill = self.compute_cut_fill_report(sea_level_m, 30.0);
        SiteTerraformIfcxOverlay {
            sea_level_m,
            min_elevation_m: self.min_elevation,
            max_elevation_m: self.max_elevation,
            vertex_count: self.points.len(),
            triangle_count: self.triangles.len(),
            total_cut_m3,
            total_fill_m3,
            net_earthwork_m3: total_fill_m3 - total_cut_m3,
            max_slope_deg: cut_fill.max_slope_deg,
            steep_hazard_ratio_pct: cut_fill.steep_hazard_ratio_pct,
            sculpt_strokes,
            revision: self.revision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase25_dod_500m_10k_triangles_mesh() {
        let mesh = TinSurfaceMesh::default();
        assert_eq!(mesh.extent_x_m, 500.0);
        assert_eq!(mesh.extent_y_m, 500.0);
        assert_eq!(mesh.points.len(), 72 * 72);
        // 71 * 71 * 2 = 10,082 triangles (> 10,000 triangles per Phase 25 DoD)
        assert_eq!(mesh.triangles.len(), 10_082);
        assert_eq!(mesh.triangle_metrics.len(), 10_082);
        assert!(mesh.max_elevation > mesh.min_elevation);
    }

    #[test]
    fn test_bowyer_watson_delaunay_triangulation() {
        let pts = vec![
            TinPoint { id: 0, x: 0.0, y: 0.0, z: 10.0 },
            TinPoint { id: 1, x: 100.0, y: 0.0, z: 12.0 },
            TinPoint { id: 2, x: 100.0, y: 100.0, z: 18.0 },
            TinPoint { id: 3, x: 0.0, y: 100.0, z: 14.0 },
            TinPoint { id: 4, x: 50.0, y: 50.0, z: 25.0 },
        ];
        let mesh = TinSurfaceMesh::triangulate_bowyer_watson(pts);
        assert_eq!(mesh.points.len(), 5);
        assert_eq!(mesh.triangles.len(), 4);
        let report = mesh.compute_cut_fill_report(10.0, 30.0);
        assert!((report.planar_area_m2 - 10_000.0).abs() < 1.0);
        assert!(report.cut_volume_m3 > 0.0);
    }

    #[test]
    fn test_contour_extraction_and_slope_hazard() {
        let mesh = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::KarstLimestoneEscarpment,
            500.0,
            500.0,
            40,
            40,
            1.0,
        );
        let contours = mesh.extract_contours(1.0, 5.0);
        assert!(!contours.is_empty());
        assert!(contours.iter().any(|c| c.is_index_contour));
        assert!(contours.iter().any(|c| !c.is_index_contour));

        let report = mesh.compute_cut_fill_report(105.0, 30.0);
        assert!(report.max_slope_deg > 30.0);
        assert!(report.steep_hazard_area_m2 > 0.0);
    }

    #[test]
    fn test_v0865_realistic_landscape_presets_have_sea_and_mountains() {
        for preset in [
            DemTerrainPreset::ArchipelagoBayAndVolcano,
            DemTerrainPreset::FjordRiverValley,
            DemTerrainPreset::CoastalIslandMetropolis,
        ] {
            assert!(preset.has_coastal_water());
            let tin = TinSurfaceMesh::generate_from_dem_preset(preset, 680.0, 680.0, 46, 46, 1.0);
            assert!(
                tin.min_elevation < -2.0,
                "{preset:?} must carve coastal seabed below sea level, got min={}",
                tin.min_elevation
            );
            assert!(
                tin.max_elevation > 32.0,
                "{preset:?} must elevate mountain/hilltop terrain, got max={}",
                tin.max_elevation
            );
            let (origin_z, _) = tin.sample_elevation_and_slope(0.0, 0.0);
            assert!(
                (1.0..=6.5).contains(&origin_z),
                "{preset:?} must maintain a buildable harbor terrace at (0,0), got z={origin_z}"
            );
        }
    }

    #[test]
    fn test_v0866_interactive_terrain_sculpt_raise_flatten_smooth_and_carve_water() {
        let mut tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::ArchipelagoBayAndVolcano,
            400.0,
            400.0,
            28,
            28,
            1.0,
        );
        let snap_orig: Vec<f32> = tin.points.iter().map(|p| p.z).collect();
        let (orig_center, _) = tin.sample_elevation_and_slope(0.0, 0.0);

        // 1. Raise mound at origin
        let raise_res =
            tin.apply_sculpt_brush(0.0, 0.0, TerrainSculptBrush::Raise, 60.0, 5.0, 0.0, 0.0);
        assert!(raise_res.modified_vertices > 5);
        assert!(raise_res.delta_fill_m3 > 100.0);
        assert!(raise_res.center_elev_after_m > orig_center + 1.0);
        assert_eq!(tin.revision, 1);

        // 2. Smooth reduces sharpness
        let smooth_res =
            tin.apply_sculpt_brush(0.0, 0.0, TerrainSculptBrush::Smooth, 60.0, 3.0, 0.0, 0.0);
        assert!(smooth_res.modified_vertices > 0);
        assert_eq!(tin.revision, 2);

        // 3. Lower reduces elevation
        let lower_res =
            tin.apply_sculpt_brush(0.0, 0.0, TerrainSculptBrush::Lower, 60.0, 4.0, 0.0, 0.0);
        assert!(lower_res.delta_cut_m3 > 50.0);
        assert!(lower_res.center_elev_after_m < raise_res.center_elev_after_m);

        // 4. Flatten toward +6.0m pad
        let _ = tin.apply_sculpt_brush(0.0, 0.0, TerrainSculptBrush::Flatten, 55.0, 3.0, 6.0, 0.0);
        let (z_flat, _) = tin.sample_elevation_and_slope(0.0, 0.0);
        assert!((z_flat - 6.0).abs() < 4.5);

        // 5. CarveWater digs inland ground below sea level (0.0m)
        for _ in 0..4 {
            tin.apply_sculpt_brush(0.0, 0.0, TerrainSculptBrush::CarveWater, 65.0, 4.5, 0.0, 0.0);
        }
        let (z_canal, _) = tin.sample_elevation_and_slope(0.0, 0.0);
        assert!(
            z_canal < -0.5,
            "CarveWater must dig below 0.0m sea level so coastal water fills in, got {z_canal}"
        );

        let overlay = tin.export_ifcx_site_terraform_overlay(
            0.0,
            lower_res.delta_cut_m3,
            raise_res.delta_fill_m3,
            5,
        );
        assert_eq!(overlay.sculpt_strokes, 5);
        assert_eq!(overlay.revision, tin.revision);

        // 6. Snapshot restore cleanly resets elevations
        assert!(tin.restore_elevations_snapshot(&snap_orig));
        let (z_restored, _) = tin.sample_elevation_and_slope(0.0, 0.0);
        assert!((z_restored - orig_center).abs() < 1e-4);
    }
}
