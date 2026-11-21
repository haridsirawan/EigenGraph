//! ifcx.rs — 100% Pure-Rust IFCX (buildingSMART IFC 5 JSON ECS) Engine & Living Earth Graph (Phase 66 / v0.6.6).
//!
//! Replaces all legacy ISO-10303-21 STEP (`IFC2x3` / `IFC4` / `IFC4.3`) parsers and writers with a
//! single lean, composable, USD-inspired Entity-Component-System (ECS) JSON graph parsed strictly
//! with `serde` + `serde_json` and ZERO external crates.
//!
//! Core capabilities:
//! - Unified Living Earth hierarchy (`/earth/globe/...` + `/earth/sites/<site>/buildings/<bldg>/...`)
//! - `IfcxStage::compose`: non-destructive multi-layer stacking (Earth Base -> AI Real-Building
//!   Capture -> User Continuation Edits -> 6-DOF FEM/ARI Structural Overlay)
//! - `IfcxStage::build_earth_globe_layer`: builds the entire 3D Vector Earth Globe (ocean sphere,
//!   continents, Indonesian archipelago, mountain ridges, major rivers) as native `IfcxNode`s with
//!   `usd::usdgeom::mesh` ready for 1:1 WebGL2 VBO/IBO upload
//! - Lossless `BimProject` <-> `.ifcx` round-trip AND automatic reconstruction of editable 6-DOF
//!   `BimProject` / `StructuralModel` from external or AI-generated `.ifcx` meshes
//! - Pure-JSON IFCX Health Audit, Spatial Clash Detection, Layer Diff, and Undo/Redo `IfcxEditSession`

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::ari::RedundancyAssessment;
use crate::core::bim::{
    BimColumn, BimFooting, BimLevel, BimProject, BimRun, BimSlab, BimWall, RunCategory, RunJustify,
};
use crate::core::geom::P2;
use crate::core::gis::dem_stream::build_tin_from_3d_triangles;
use crate::core::gis::earth_globe::lat_lon_to_unit_sphere;
use crate::core::gis::map_widget::{
    VectorEarthTheme, EARTH_RADIUS_METERS, GLOBAL_CITY_CATALOG, WORLD_LANDMASSES,
    WORLD_MAJOR_RIVERS, WORLD_MOUNTAIN_RIDGES,
};
use crate::core::gis::tin_surface::TinSurfaceMesh;
use crate::core::gis::world_index::GeoPlacementSource;
use crate::core::model::{
    BoundaryFixity, CrossSection, Element3D, ElementType, Material, NodalLoad, Node3D,
    StructuralModel,
};
use crate::models::StructuralMember;

// =============================================================================
// 1. OFFICIAL IFCX (IFC 5 JSON ECS) CORE DATA MODEL
// =============================================================================

pub const IFCX_SCHEMA_VERSION: &str = "ifcx_alpha";
pub const IFCX_DATA_VERSION: &str = "1.0.0";

/// Standard attribute namespaces aligned with buildingSMART IFC 5 / OpenUSD + EigenGraph extensions.
pub mod ns {
    pub const BSI_CLASS: &str = "bsi::ifc::class";
    pub const BSI_PROP: &str = "bsi::ifc::prop";
    pub const USD_MESH: &str = "usd::usdgeom::mesh";
    pub const USD_XFORM: &str = "usd::xformop::transform";
    pub const EG_GEO: &str = "eigengraph::geo::wgs84";
    pub const EG_EARTH: &str = "eigengraph::earth::globe";
    pub const EG_BIM_PROJECT: &str = "eigengraph::bim::project";
    pub const EG_BIM_ELEMENT: &str = "eigengraph::bim::element";
    pub const EG_FEM_NODE: &str = "eigengraph::fem::node";
    pub const EG_FEM_MEMBER: &str = "eigengraph::fem::member";
    pub const EG_ARI: &str = "eigengraph::ari::v2";
}

fn default_ifcx_version() -> String {
    IFCX_SCHEMA_VERSION.to_string()
}

fn default_data_version() -> String {
    IFCX_DATA_VERSION.to_string()
}

/// Unit & Coordinate Reference System metadata embedded in every `.ifcx` header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfcxUnits {
    #[serde(default = "default_unit_m")]
    pub length: String,
    #[serde(default = "default_unit_deg")]
    pub angle: String,
    #[serde(default = "default_unit_kn")]
    pub force: String,
    #[serde(default = "default_unit_crs")]
    pub crs: String,
}

fn default_unit_m() -> String {
    "m".to_string()
}
fn default_unit_deg() -> String {
    "deg".to_string()
}
fn default_unit_kn() -> String {
    "kN".to_string()
}
fn default_unit_crs() -> String {
    "WGS84".to_string()
}

impl Default for IfcxUnits {
    fn default() -> Self {
        Self {
            length: default_unit_m(),
            angle: default_unit_deg(),
            force: default_unit_kn(),
            crs: default_unit_crs(),
        }
    }
}

/// IFCX Document Header (`header` block in `.ifcx` JSON).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfcxHeader {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "ifcxVersion", default = "default_ifcx_version")]
    pub ifcx_version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(rename = "dataVersion", default = "default_data_version")]
    pub data_version: String,
    #[serde(default)]
    pub units: IfcxUnits,
}

impl Default for IfcxHeader {
    fn default() -> Self {
        Self {
            id: "eigengraph-ifcx-layer".to_string(),
            ifcx_version: default_ifcx_version(),
            author: format!("EigenGraph v{}", env!("CARGO_PKG_VERSION")),
            timestamp: crate::core::web_bridge::unix_timestamp_label(),
            data_version: default_data_version(),
            units: IfcxUnits::default(),
        }
    }
}

/// External or stacked layer reference (`imports` array in `.ifcx`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfcxImport {
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Single Entity-Component node inside an `.ifcx` scene graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfcxNode {
    pub path: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub children: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inherits: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, Value>,
}

/// Typed `usd::usdgeom::mesh` payload mapping 1:1 to WebGL2 VBO/IBO buffers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UsdGeomMesh {
    pub points: Vec<[f64; 3]>,
    #[serde(rename = "faceVertexCounts", default)]
    pub face_vertex_counts: Vec<u32>,
    #[serde(rename = "faceVertexIndices", default)]
    pub face_vertex_indices: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normals: Vec<[f64; 3]>,
    #[serde(rename = "displayColor", default, skip_serializing_if = "Option::is_none")]
    pub display_color: Option<[f32; 4]>,
    /// Per-vertex color overrides (optional; falls back to `display_color`).
    #[serde(rename = "vertexColors", default, skip_serializing_if = "Vec::is_empty")]
    pub vertex_colors: Vec<[f32; 4]>,
    /// Analytical shader metadata per vertex (`[material_kind, elev_m, ao, phase]`) or uniform default.
    #[serde(rename = "vertexMeta", default, skip_serializing_if = "Vec::is_empty")]
    pub vertex_meta: Vec<[f32; 4]>,
    #[serde(rename = "materialKind", default)]
    pub material_kind: f32,
}

impl UsdGeomMesh {
    /// Expands polygon faces (`faceVertexCounts` of 3 or 4+) into triangle index triplets.
    pub fn triangulated_indices(&self) -> Vec<[usize; 3]> {
        let mut tris = Vec::new();
        if self.face_vertex_counts.is_empty() {
            for chunk in self.face_vertex_indices.chunks_exact(3) {
                tris.push([chunk[0] as usize, chunk[1] as usize, chunk[2] as usize]);
            }
            return tris;
        }
        let mut cursor = 0usize;
        for &count in &self.face_vertex_counts {
            let c = count as usize;
            if c >= 3 && cursor + c <= self.face_vertex_indices.len() {
                let i0 = self.face_vertex_indices[cursor] as usize;
                for k in 1..(c - 1) {
                    let i1 = self.face_vertex_indices[cursor + k] as usize;
                    let i2 = self.face_vertex_indices[cursor + k + 1] as usize;
                    tris.push([i0, i1, i2]);
                }
            }
            cursor += c;
        }
        tris
    }

    /// Computes axis-aligned 3D bounding box `(min, max)` across mesh points.
    pub fn bbox(&self) -> Option<([f64; 3], [f64; 3])> {
        let first = *self.points.first()?;
        let mut mn = first;
        let mut mx = first;
        for &p in &self.points[1..] {
            for k in 0..3 {
                if p[k] < mn[k] {
                    mn[k] = p[k];
                }
                if p[k] > mx[k] {
                    mx[k] = p[k];
                }
            }
        }
        Some((mn, mx))
    }

    /// Creates an axis-aligned 3D box mesh (in meters) between `min` and `max`.
    pub fn from_box_m(min: [f64; 3], max: [f64; 3], color: [f32; 4], material_kind: f32) -> Self {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;
        let points = vec![
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z0],
            [x0, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
        ];
        let face_vertex_indices = vec![
            0, 2, 1, 0, 3, 2, // bottom
            4, 5, 6, 4, 6, 7, // top
            0, 1, 5, 0, 5, 4, // south
            2, 3, 7, 2, 7, 6, // north
            1, 2, 6, 1, 6, 5, // east
            3, 0, 4, 3, 4, 7, // west
        ];
        let face_vertex_counts = vec![3; 12];
        Self {
            points,
            face_vertex_counts,
            face_vertex_indices,
            normals: Vec::new(),
            display_color: Some(color),
            vertex_colors: Vec::new(),
            vertex_meta: Vec::new(),
            material_kind,
        }
    }

    /// Creates a `UsdGeomMesh` from a list of 3D triangles in millimetres (converted to meters).
    pub fn from_triangles_mm(tris_mm: &[[[f64; 3]; 3]], color: [f32; 4], material_kind: f32) -> Self {
        let mut points = Vec::with_capacity(tris_mm.len() * 3);
        let mut normals = Vec::with_capacity(tris_mm.len() * 3);
        let mut face_vertex_indices = Vec::with_capacity(tris_mm.len() * 3);
        let mut face_vertex_counts = Vec::with_capacity(tris_mm.len());
        for tri in tris_mm {
            let a = [tri[0][0] * 0.001, tri[0][1] * 0.001, tri[0][2] * 0.001];
            let b = [tri[1][0] * 0.001, tri[1][1] * 0.001, tri[1][2] * 0.001];
            let c = [tri[2][0] * 0.001, tri[2][1] * 0.001, tri[2][2] * 0.001];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = norm3_f64(cross3_f64(ab, ac));
            let base = points.len() as u32;
            points.push(a);
            points.push(b);
            points.push(c);
            normals.push(n);
            normals.push(n);
            normals.push(n);
            face_vertex_indices.extend_from_slice(&[base, base + 1, base + 2]);
            face_vertex_counts.push(3);
        }
        Self {
            points,
            face_vertex_counts,
            face_vertex_indices,
            normals,
            display_color: Some(color),
            vertex_colors: Vec::new(),
            vertex_meta: Vec::new(),
            material_kind,
        }
    }
}

/// Geodetic site anchor stored under `eigengraph::geo::wgs84`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfcxGeoWgs84 {
    pub lat: f64,
    pub lon: f64,
    #[serde(rename = "elevM", default)]
    pub elev_m: f64,
    #[serde(rename = "headingDeg", default)]
    pub heading_deg: f64,
    #[serde(default)]
    pub address: String,
    #[serde(rename = "utmZone", default)]
    pub utm_zone: u8,
    #[serde(rename = "southHemisphere", default)]
    pub south_hemisphere: bool,
}

/// Non-destructive 6-DOF FEM & ARI structural screening overlay stored under `eigengraph::ari::v2`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct IfcxAriOverlay {
    #[serde(rename = "ariSystem")]
    pub ari_system: f64,
    #[serde(rename = "ariAverage")]
    pub ari_average: f64,
    #[serde(rename = "criticalMembersCount")]
    pub critical_members_count: usize,
    #[serde(rename = "maxDcrPct")]
    pub max_dcr_pct: f64,
    #[serde(rename = "svdRankDeficiency")]
    pub svd_rank_deficiency: usize,
    #[serde(rename = "isKinematicallyStable")]
    pub is_kinematically_stable: bool,
}

impl IfcxNode {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            children: BTreeMap::new(),
            inherits: BTreeMap::new(),
            attributes: BTreeMap::new(),
        }
    }

    pub fn with_class(mut self, code: &str) -> Self {
        self.set_class(code);
        self
    }

    pub fn set_class(&mut self, code: &str) {
        self.attributes.insert(
            ns::BSI_CLASS.to_string(),
            json!({
                "code": code,
                "uri": format!("https://identifier.buildingsmart.org/uri/buildingsmart/ifc/5/class/{code}")
            }),
        );
    }

    pub fn with_prop(mut self, name: &str, material: &str, description: &str) -> Self {
        self.set_prop(name, material, description);
        self
    }

    pub fn set_prop(&mut self, name: &str, material: &str, description: &str) {
        self.attributes.insert(
            ns::BSI_PROP.to_string(),
            json!({
                "name": name,
                "material": material,
                "description": description
            }),
        );
    }

    pub fn with_mesh(mut self, mesh: &UsdGeomMesh) -> Self {
        if let Ok(v) = serde_json::to_value(mesh) {
            self.attributes.insert(ns::USD_MESH.to_string(), v);
        }
        self
    }

    pub fn with_geo(mut self, geo: &IfcxGeoWgs84) -> Self {
        if let Ok(v) = serde_json::to_value(geo) {
            self.attributes.insert(ns::EG_GEO.to_string(), v);
        }
        self
    }

    pub fn with_attr(mut self, namespace: &str, val: Value) -> Self {
        self.attributes.insert(namespace.to_string(), val);
        self
    }

    pub fn ifc_class(&self) -> Option<&str> {
        self.attributes
            .get(ns::BSI_CLASS)
            .and_then(|v| v.get("code").or_else(|| v.as_str().map(|_| v)))
            .and_then(|v| v.as_str())
    }

    pub fn display_name(&self) -> String {
        if let Some(n) = self
            .attributes
            .get(ns::BSI_PROP)
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
        {
            if !n.trim().is_empty() {
                return n.to_string();
            }
        }
        self.path
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.path)
            .to_string()
    }

    pub fn material_label(&self) -> String {
        self.attributes
            .get(ns::BSI_PROP)
            .and_then(|v| v.get("material"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unspecified")
            .to_string()
    }

    pub fn description_label(&self) -> String {
        self.attributes
            .get(ns::BSI_PROP)
            .and_then(|v| v.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn mesh(&self) -> Option<UsdGeomMesh> {
        self.attributes
            .get(ns::USD_MESH)
            .and_then(|v| serde_json::from_value::<UsdGeomMesh>(v.clone()).ok())
    }

    pub fn geo_wgs84(&self) -> Option<IfcxGeoWgs84> {
        self.attributes
            .get(ns::EG_GEO)
            .and_then(|v| serde_json::from_value::<IfcxGeoWgs84>(v.clone()).ok())
    }

    /// Returns axis-aligned bounding box in meters (accounting for optional `usd::xformop::transform` translation).
    pub fn world_bbox_m(&self) -> Option<([f64; 3], [f64; 3])> {
        let mesh = self.mesh()?;
        let (mut mn, mut mx) = mesh.bbox()?;
        if let Some(tr) = self
            .attributes
            .get(ns::USD_XFORM)
            .and_then(|v| v.get("translation"))
            .and_then(|v| v.as_array())
        {
            if tr.len() >= 3 {
                let tx = tr[0].as_f64().unwrap_or(0.0);
                let ty = tr[1].as_f64().unwrap_or(0.0);
                let tz = tr[2].as_f64().unwrap_or(0.0);
                mn = [mn[0] + tx, mn[1] + ty, mn[2] + tz];
                mx = [mx[0] + tx, mx[1] + ty, mx[2] + tz];
            }
        }
        Some((mn, mx))
    }
}

/// Complete IFCX (IFC 5) JSON Document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfcxDocument {
    #[serde(default)]
    pub header: IfcxHeader,
    #[serde(default)]
    pub imports: Vec<IfcxImport>,
    #[serde(default)]
    pub schemas: BTreeMap<String, Value>,
    #[serde(default)]
    pub data: Vec<IfcxNode>,
}

impl Default for IfcxDocument {
    fn default() -> Self {
        Self::new("eigengraph-ifcx")
    }
}

impl IfcxDocument {
    pub fn new(id: impl Into<String>) -> Self {
        let mut schemas = BTreeMap::new();
        schemas.insert(
            ns::BSI_CLASS.to_string(),
            json!({ "value": { "dataType": "Object", "spec": "buildingSMART IFC 5" } }),
        );
        schemas.insert(
            ns::USD_MESH.to_string(),
            json!({ "value": { "dataType": "Object", "spec": "OpenUSD UsdGeomMesh" } }),
        );
        schemas.insert(
            ns::EG_GEO.to_string(),
            json!({ "value": { "dataType": "Object", "spec": "EigenGraph WGS84 Georeferencing" } }),
        );
        schemas.insert(
            ns::EG_ARI.to_string(),
            json!({ "value": { "dataType": "Object", "spec": "EigenGraph ARI v2 Structural Screening" } }),
        );
        Self {
            header: IfcxHeader {
                id: id.into(),
                ..IfcxHeader::default()
            },
            imports: Vec::new(),
            schemas,
            data: Vec::new(),
        }
    }

    /// Parses an `.ifcx` JSON string cleanly with zero external crates.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let clean = raw.trim_start_matches('\u{FEFF}').trim();
        if clean.is_empty() {
            return Err("Empty IFCX input.".to_string());
        }
        let doc: Self =
            serde_json::from_str(clean).map_err(|e| format!("Invalid IFCX JSON: {e}"))?;
        if doc.data.is_empty() && doc.header.ifcx_version.is_empty() {
            return Err("JSON document does not contain IFCX header or data nodes.".to_string());
        }
        Ok(doc)
    }

    /// Heuristic detection whether a raw text payload is an `.ifcx` JSON document.
    pub fn looks_like_ifcx(raw: &str) -> bool {
        let trimmed = raw.trim_start_matches('\u{FEFF}').trim_start();
        if !trimmed.starts_with('{') {
            return false;
        }
        let head = &trimmed[..trimmed.len().min(4096)];
        head.contains("\"ifcxVersion\"")
            || (head.contains("\"data\"") && (head.contains("bsi::ifc") || head.contains("usd::usdgeom")))
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn to_json_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn add_node(&mut self, node: IfcxNode) {
        self.data.push(node);
    }

    pub fn find_node(&self, path: &str) -> Option<&IfcxNode> {
        self.data.iter().find(|n| n.path == path)
    }

    pub fn find_node_mut(&mut self, path: &str) -> Option<&mut IfcxNode> {
        self.data.iter_mut().find(|n| n.path == path)
    }

    /// Fast O(1)-friendly JSON geo extractor used during Vault folder scanning.
    pub fn extract_geo_fast(raw: &str) -> Option<(f64, f64, f64, String)> {
        if let Ok(doc) = Self::parse(raw) {
            for node in &doc.data {
                if let Some(geo) = node.geo_wgs84() {
                    if (geo.lat.abs() > 1e-4 || geo.lon.abs() > 1e-4)
                        && (-90.0..=90.0).contains(&geo.lat)
                        && (-180.0..=180.0).contains(&geo.lon)
                    {
                        return Some((geo.lat, geo.lon, geo.elev_m, geo.address));
                    }
                }
            }
        }
        None
    }

    /// Extracts pure-vector 3D site topography (`IfcSite` / `IfcGeographicElement` terrain
    /// `usd::usdgeom::mesh`) into a centered `TinSurfaceMesh`.
    pub fn extract_site_terrain_tin(&self) -> Option<TinSurfaceMesh> {
        for node in &self.data {
            let class_code = node.ifc_class().unwrap_or("");
            let is_terrain_node = class_code.eq_ignore_ascii_case("IfcSite")
                || (class_code.eq_ignore_ascii_case("IfcGeographicElement")
                    && (node.path.contains("terrain")
                        || node.path.contains("site")
                        || node.attributes.get(ns::EG_EARTH).is_none()));
            if !is_terrain_node {
                continue;
            }
            if let Some(mesh) = node.mesh() {
                if mesh.points.len() >= 3 {
                    let coords: Vec<(f64, f64, f64)> = mesh
                        .points
                        .iter()
                        .map(|p| (p[0], p[1], p[2]))
                        .collect();
                    let tris = mesh.triangulated_indices();
                    if let Ok(tin) = build_tin_from_3d_triangles(&coords, &tris) {
                        return Some(tin);
                    }
                }
            }
        }
        None
    }

    pub fn extract_site_terrain_tin_from_raw(raw: &str) -> Option<TinSurfaceMesh> {
        Self::parse(raw).ok()?.extract_site_terrain_tin()
    }
}

// =============================================================================
// 2. NON-DESTRUCTIVE MULTI-LAYER COMPOSER (`IfcxStage`) & LIVING EARTH GLOBE
// =============================================================================

/// Non-destructive composed IFCX stage merging stacked `.ifcx` layers in order:
/// `Layer 0: earth_globe.ifcx` -> `Layer 1: ai_city_capture.ifcx` -> `Layer 2: user_edits.ifcx`
/// -> `Layer 3: ari_analysis.ifcx`.
#[derive(Debug, Clone, Default)]
pub struct IfcxStage {
    pub layer_ids: Vec<String>,
    pub schemas: BTreeMap<String, Value>,
    pub nodes: BTreeMap<String, IfcxNode>,
}

impl IfcxStage {
    /// Composes an ordered slice of `.ifcx` documents non-destructively.
    /// Later layers override specific attribute namespaces or append child links without
    /// erasing untouched nodes or attributes from earlier layers.
    pub fn compose(layers: &[IfcxDocument]) -> Self {
        let mut stage = Self::default();
        for layer in layers {
            stage.apply_layer(layer);
        }
        stage
    }

    pub fn apply_layer(&mut self, layer: &IfcxDocument) {
        self.layer_ids.push(layer.header.id.clone());
        for (k, v) in &layer.schemas {
            self.schemas.insert(k.clone(), v.clone());
        }
        for node in &layer.data {
            let entry = self
                .nodes
                .entry(node.path.clone())
                .or_insert_with(|| IfcxNode::new(&node.path));
            for (ck, cv) in &node.children {
                entry.children.insert(ck.clone(), cv.clone());
            }
            for (ik, iv) in &node.inherits {
                entry.inherits.insert(ik.clone(), iv.clone());
            }
            for (ak, av) in &node.attributes {
                // Deep-merge JSON objects per attribute namespace so partial overrides preserve
                // untouched sibling fields (e.g. overriding only `material` inside `bsi::ifc::prop`).
                match (entry.attributes.get_mut(ak), av) {
                    (Some(Value::Object(base_map)), Value::Object(patch_map)) => {
                        for (pk, pv) in patch_map {
                            base_map.insert(pk.clone(), pv.clone());
                        }
                    }
                    _ => {
                        entry.attributes.insert(ak.clone(), av.clone());
                    }
                }
            }
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, path: &str) -> Option<&IfcxNode> {
        self.nodes.get(path)
    }

    /// Flattens the composed stage into a single self-contained `.ifcx` document with provenance imports.
    pub fn to_composed_document(&self, id: &str) -> IfcxDocument {
        let mut doc = IfcxDocument::new(id);
        doc.schemas = self.schemas.clone();
        doc.imports = self
            .layer_ids
            .iter()
            .enumerate()
            .map(|(idx, lid)| IfcxImport {
                uri: format!("eigengraph://layers/{lid}.ifcx"),
                integrity: None,
                role: Some(format!("layer-{idx}")),
            })
            .collect();
        doc.data = self.nodes.values().cloned().collect();
        doc
    }

    /// Builds the **entire 3D Pure-Rust Vector Earth Globe** as a native `.ifcx` layer
    /// (`earth_globe.ifcx`) with `IfcGeographicElement` + `usd::usdgeom::mesh` nodes for:
    /// - Smooth 64×32 spherical ocean dome (`/earth/globe/ocean`)
    /// - Subdivided spherical landmasses & detailed Indonesian archipelago (`/earth/globe/landmasses/land_NNN`)
    /// - Major tectonic mountain ridges (`/earth/globe/ridges/ridge_NNN`)
    /// - Major hydrological river corridors (`/earth/globe/rivers/river_NNN`)
    pub fn build_earth_globe_layer(theme: VectorEarthTheme) -> IfcxDocument {
        let pal = theme.palette();
        let mut doc = IfcxDocument::new(format!("earth_globe_{}", theme.short_name().to_lowercase().replace(' ', "_")));

        let mut earth_root = IfcxNode::new("/earth")
            .with_class("IfcProject")
            .with_prop("Living Earth", "Planetary WGS84", "EigenGraph 100% IFCX Living Earth Root")
            .with_attr(
                ns::EG_EARTH,
                json!({
                    "theme": theme.display_name(),
                    "radiusM": EARTH_RADIUS_METERS,
                    "kind": "planet_root"
                }),
            );
        earth_root
            .children
            .insert("globe".to_string(), "/earth/globe".to_string());

        let mut globe_site = IfcxNode::new("/earth/globe")
            .with_class("IfcSite")
            .with_prop("Vector Earth Globe", "Planetary Sphere", "Pure-Vector IFCX Planetary Base Layer")
            .with_attr(
                ns::EG_EARTH,
                json!({
                    "theme": theme.display_name(),
                    "radiusM": EARTH_RADIUS_METERS,
                    "kind": "globe_site"
                }),
            );
        globe_site
            .children
            .insert("ocean".to_string(), "/earth/globe/ocean".to_string());

        // 1. Spherical Ocean Dome (64×32 UV-Sphere, radius = 1.0 unit sphere)
        const LON_SEGS: usize = 64;
        const LAT_SEGS: usize = 32;
        let ocean_rgba = color32_to_rgba(pal.deep_ocean);
        let mut ocean_pts = Vec::with_capacity((LAT_SEGS + 1) * (LON_SEGS + 1));
        let mut ocean_norms = Vec::with_capacity((LAT_SEGS + 1) * (LON_SEGS + 1));
        let mut ocean_meta = Vec::with_capacity((LAT_SEGS + 1) * (LON_SEGS + 1));
        for iy in 0..=LAT_SEGS {
            let v = iy as f64 / LAT_SEGS as f64;
            let lat = -std::f64::consts::FRAC_PI_2 + v * std::f64::consts::PI;
            let (slat, clat) = (lat.sin(), lat.cos());
            for ix in 0..=LON_SEGS {
                let u = ix as f64 / LON_SEGS as f64;
                let lon = -std::f64::consts::PI + u * std::f64::consts::TAU;
                let p = [clat * lon.sin(), slat, clat * lon.cos()];
                ocean_pts.push(p);
                ocean_norms.push(p);
                ocean_meta.push([0.0, 0.0, 1.0, (u * 8.0) as f32]);
            }
        }
        let mut ocean_indices = Vec::with_capacity(LAT_SEGS * LON_SEGS * 6);
        let mut ocean_counts = Vec::with_capacity(LAT_SEGS * LON_SEGS * 2);
        let stride = (LON_SEGS + 1) as u32;
        for iy in 0..LAT_SEGS as u32 {
            for ix in 0..LON_SEGS as u32 {
                let i00 = iy * stride + ix;
                let i10 = i00 + 1;
                let i01 = i00 + stride;
                let i11 = i01 + 1;
                ocean_indices.extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
                ocean_counts.push(3);
                ocean_counts.push(3);
            }
        }
        let ocean_mesh = UsdGeomMesh {
            points: ocean_pts,
            face_vertex_counts: ocean_counts,
            face_vertex_indices: ocean_indices,
            normals: ocean_norms,
            display_color: Some(ocean_rgba),
            vertex_colors: Vec::new(),
            vertex_meta: ocean_meta,
            material_kind: 0.0,
        };
        let ocean_node = IfcxNode::new("/earth/globe/ocean")
            .with_class("IfcGeographicElement")
            .with_prop("Planetary Ocean", "Procedural Hydrosphere", "64x32 Unit-Sphere Ocean Dome")
            .with_attr(ns::EG_EARTH, json!({ "kind": "ocean", "theme": theme.display_name() }))
            .with_mesh(&ocean_mesh);

        // 2. Spherical Extruded Continents & Detailed Indonesian Archipelago (`WORLD_LANDMASSES`)
        let land_base = color32_to_rgba(pal.land_base);
        let land_arid = color32_to_rgba(pal.land_arid);
        let land_polar = color32_to_rgba(pal.land_polar);
        let coast_col = color32_to_rgba(pal.coast_stroke);
        const LAND_R: f64 = 1.0022;
        const SKIRT_R: f64 = 0.9995;

        let mut landmass_nodes = Vec::with_capacity(WORLD_LANDMASSES.len());
        for (idx, land) in WORLD_LANDMASSES.iter().enumerate() {
            if land.len() < 3 {
                continue;
            }
            let unit_pts: Vec<[f64; 3]> = land
                .iter()
                .map(|&(la, lo)| lat_lon_to_unit_sphere(la, lo))
                .collect();
            let avg_lat = land.iter().map(|&(la, _)| la).sum::<f64>() / land.len() as f64;
            let avg_lon = land.iter().map(|&(_, lo)| lo).sum::<f64>() / land.len() as f64;
            let is_antarctica = avg_lat < -62.0;

            let proj_2d: Vec<[f64; 2]> = if is_antarctica {
                unit_pts
                    .iter()
                    .map(|p| {
                        let denom = (1.0 - p[1]).max(0.15);
                        [p[0] / denom, p[2] / denom]
                    })
                    .collect()
            } else {
                let mut c = [0.0_f64; 3];
                for p in &unit_pts {
                    c[0] += p[0];
                    c[1] += p[1];
                    c[2] += p[2];
                }
                let c_n = norm3_f64(c);
                let ref_up = if c_n[1].abs() < 0.92 {
                    [0.0, 1.0, 0.0]
                } else {
                    [1.0, 0.0, 0.0]
                };
                let u_ax = norm3_f64(cross3_f64(ref_up, c_n));
                let v_ax = norm3_f64(cross3_f64(c_n, u_ax));
                unit_pts
                    .iter()
                    .map(|p| {
                        let denom = (1.0 + p[0] * c_n[0] + p[1] * c_n[1] + p[2] * c_n[2]).max(0.25);
                        let u = (p[0] * u_ax[0] + p[1] * u_ax[1] + p[2] * u_ax[2]) / denom;
                        let v = (p[0] * v_ax[0] + p[1] * v_ax[1] + p[2] * v_ax[2]) / denom;
                        [u, v]
                    })
                    .collect()
            };

            let abs_lat = avg_lat.abs();
            let fill_col = if abs_lat > 65.0 {
                mix_rgba(land_base, land_polar, 0.72)
            } else if (18.0..=31.0).contains(&abs_lat) {
                mix_rgba(land_base, land_arid, 0.52)
            } else {
                land_base
            };

            let mut pts = Vec::new();
            let mut norms = Vec::new();
            let mut vcols = Vec::new();
            let mut vmeta = Vec::new();
            let mut idxs = Vec::new();
            let mut counts = Vec::new();

            for [i0, i1, i2] in ear_clip_indices_2d(&proj_2d) {
                push_subdivided_spherical_tri(
                    &mut pts,
                    &mut norms,
                    &mut vcols,
                    &mut vmeta,
                    &mut idxs,
                    &mut counts,
                    unit_pts[i0],
                    unit_pts[i1],
                    unit_pts[i2],
                    LAND_R,
                    fill_col,
                    0,
                );
            }

            // 3D coastal cliff bevel rim
            for i in 0..unit_pts.len() {
                let a = unit_pts[i];
                let b = unit_pts[(i + 1) % unit_pts.len()];
                let chord = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let mid = norm3_f64([a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
                let edge_n = norm3_f64(cross3_f64(chord, mid));
                let base = pts.len() as u32;
                for (pt, r, ao) in [
                    (a, LAND_R, 1.0_f32),
                    (b, LAND_R, 1.0),
                    (b, SKIRT_R, 0.70),
                    (a, SKIRT_R, 0.70),
                ] {
                    pts.push([pt[0] * r, pt[1] * r, pt[2] * r]);
                    norms.push(edge_n);
                    vcols.push(coast_col);
                    vmeta.push([1.0, 0.0, ao, 0.0]);
                }
                idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                counts.push(3);
                counts.push(3);
            }

            let mesh = UsdGeomMesh {
                points: pts,
                face_vertex_counts: counts,
                face_vertex_indices: idxs,
                normals: norms,
                display_color: Some(fill_col),
                vertex_colors: vcols,
                vertex_meta: vmeta,
                material_kind: 1.0,
            };
            let key = format!("land_{idx:03}");
            let path = format!("/earth/globe/landmasses/{key}");
            globe_site.children.insert(key.clone(), path.clone());
            landmass_nodes.push(
                IfcxNode::new(path)
                    .with_class("IfcGeographicElement")
                    .with_prop(&format!("Landmass #{idx}"), "Continental Crust", "Spherical Extruded Vector Landmass")
                    .with_attr(
                        ns::EG_EARTH,
                        json!({
                            "kind": "landmass",
                            "index": idx,
                            "centroidLat": avg_lat,
                            "centroidLon": avg_lon
                        }),
                    )
                    .with_mesh(&mesh),
            );
        }

        // 3. Mountain Ridges & Major Rivers as Spherical Ribbons
        let ridge_col = color32_to_rgba(pal.ridge_stroke);
        let river_col = color32_to_rgba(pal.river_stroke);

        let build_ribbon_mesh = |chain: &[(f64, f64)], radius: f64, half_w_rad: f64, col: [f32; 4]| -> UsdGeomMesh {
            let mut pts = Vec::new();
            let mut norms = Vec::new();
            let mut vcols = Vec::new();
            let mut vmeta = Vec::new();
            let mut idxs = Vec::new();
            let mut counts = Vec::new();
            for seg in chain.windows(2) {
                let a = lat_lon_to_unit_sphere(seg[0].0, seg[0].1);
                let b = lat_lon_to_unit_sphere(seg[1].0, seg[1].1);
                let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let mid = norm3_f64([a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
                let side = norm3_f64(cross3_f64(dir, mid));
                let base = pts.len() as u32;
                let quad = [
                    norm3_f64([a[0] - side[0] * half_w_rad, a[1] - side[1] * half_w_rad, a[2] - side[2] * half_w_rad]),
                    norm3_f64([a[0] + side[0] * half_w_rad, a[1] + side[1] * half_w_rad, a[2] + side[2] * half_w_rad]),
                    norm3_f64([b[0] + side[0] * half_w_rad, b[1] + side[1] * half_w_rad, b[2] + side[2] * half_w_rad]),
                    norm3_f64([b[0] - side[0] * half_w_rad, b[1] - side[1] * half_w_rad, b[2] - side[2] * half_w_rad]),
                ];
                for q in quad {
                    pts.push([q[0] * radius, q[1] * radius, q[2] * radius]);
                    norms.push(q);
                    vcols.push(col);
                    vmeta.push([3.0, 0.0, 1.0, 0.0]);
                }
                idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                counts.push(3);
                counts.push(3);
            }
            UsdGeomMesh {
                points: pts,
                face_vertex_counts: counts,
                face_vertex_indices: idxs,
                normals: norms,
                display_color: Some(col),
                vertex_colors: vcols,
                vertex_meta: vmeta,
                material_kind: 3.0,
            }
        };

        let mut feature_nodes = Vec::new();
        for (idx, ridge) in WORLD_MOUNTAIN_RIDGES.iter().enumerate() {
            let mesh = build_ribbon_mesh(ridge, 1.0032, 0.0024, ridge_col);
            let key = format!("ridge_{idx:03}");
            let path = format!("/earth/globe/ridges/{key}");
            globe_site.children.insert(key, path.clone());
            feature_nodes.push(
                IfcxNode::new(path)
                    .with_class("IfcGeographicElement")
                    .with_prop(&format!("Mountain Ridge #{idx}"), "Orogenic Spine", "Planetary Tectonic Ridge")
                    .with_attr(ns::EG_EARTH, json!({ "kind": "ridge", "index": idx }))
                    .with_mesh(&mesh),
            );
        }
        for (idx, river) in WORLD_MAJOR_RIVERS.iter().enumerate() {
            let mesh = build_ribbon_mesh(river, 1.0028, 0.0018, river_col);
            let key = format!("river_{idx:03}");
            let path = format!("/earth/globe/rivers/{key}");
            globe_site.children.insert(key, path.clone());
            feature_nodes.push(
                IfcxNode::new(path)
                    .with_class("IfcGeographicElement")
                    .with_prop(&format!("Major River #{idx}"), "Hydrology", "Continental River Corridor")
                    .with_attr(ns::EG_EARTH, json!({ "kind": "river", "index": idx }))
                    .with_mesh(&mesh),
            );
        }

        doc.add_node(earth_root);
        doc.add_node(globe_site);
        doc.add_node(ocean_node);
        for n in landmass_nodes {
            doc.add_node(n);
        }
        for n in feature_nodes {
            doc.add_node(n);
        }
        doc
    }

    /// Composes the Living Earth base layer (`earth_globe.ifcx`) with an active `BimProject` site
    /// layer, optional `IfcxCityScene` (`IfcRoad`/`IfcBridge`/`IfcSpatialZone`), and ARI overlay.
    #[allow(clippy::too_many_arguments)]
    pub fn compose_earth_with_site_and_city(
        theme: VectorEarthTheme,
        project: &BimProject,
        geo_opt: Option<(f64, f64, f64)>,
        heading_deg: f64,
        address: Option<&str>,
        members: Option<&[StructuralMember]>,
        ari: Option<&RedundancyAssessment>,
        city: Option<&crate::core::gis::IfcxCityScene>,
    ) -> IfcxDocument {
        let earth = Self::build_earth_globe_layer(theme);
        let mut site = IfcxDocument::from_bim_project(project, geo_opt, heading_deg, address, members, ari);
        if let Some(c) = city.filter(|c| !c.is_empty()) {
            let site_path = format!("/earth/sites/{}", sanitize_slug(&project.project_name));
            for cn in c.to_ifcx_nodes(&site_path) {
                site.add_node(cn);
            }
        }
        let stage = Self::compose(&[earth, site]);
        stage.to_composed_document(&format!(
            "living_earth_{}",
            sanitize_slug(&project.project_name)
        ))
    }

    /// Composes the Living Earth base layer (`earth_globe.ifcx`) with an active `BimProject` site
    /// layer and optional ARI screening overlay into one unified planetary `.ifcx` graph.
    pub fn compose_earth_with_site(
        theme: VectorEarthTheme,
        project: &BimProject,
        geo_opt: Option<(f64, f64, f64)>,
        heading_deg: f64,
        address: Option<&str>,
        members: Option<&[StructuralMember]>,
        ari: Option<&RedundancyAssessment>,
    ) -> IfcxDocument {
        Self::compose_earth_with_site_and_city(theme, project, geo_opt, heading_deg, address, members, ari, None)
    }
}

// =============================================================================
// 3. BIMPROJECT <-> IFCX LOSSLESS BRIDGE & AI REAL-BUILDING INGESTION
// =============================================================================

impl IfcxDocument {
    /// Serializes a complete `BimProject` into a composable `.ifcx` site/building layer rooted
    /// under `/earth/sites/<site_slug>/buildings/<bldg_slug>`.
    ///
    /// Every wall, column, beam/run, slab, and footing is emitted as an individual `IfcxNode` with:
    /// - `bsi::ifc::class` (`IfcWall`, `IfcColumn`, `IfcBeam`, `IfcSlab`, `IfcFooting`)
    /// - `bsi::ifc::prop` (name, material, dimensions)
    /// - `usd::usdgeom::mesh` (real 3D triangulated geometry in meters!)
    /// - `eigengraph::bim::element` (parametric descriptor)
    /// Plus `eigengraph::bim::project` at the building node for 100% lossless continuation editing.
    pub fn from_bim_project(
        project: &BimProject,
        geo_opt: Option<(f64, f64, f64)>,
        heading_deg: f64,
        address: Option<&str>,
        members: Option<&[StructuralMember]>,
        ari: Option<&RedundancyAssessment>,
    ) -> Self {
        let slug = sanitize_slug(&project.project_name);
        let mut doc = Self::new(format!("site_{slug}"));
        doc.imports.push(IfcxImport {
            uri: "eigengraph://earth_globe.ifcx".to_string(),
            integrity: None,
            role: Some("earth-base".to_string()),
        });

        let site_path = format!("/earth/sites/{slug}");
        let bldg_path = format!("{site_path}/buildings/{slug}");

        // Root `/earth` link so stacking with `earth_globe.ifcx` merges cleanly
        let mut earth_root = IfcxNode::new("/earth").with_class("IfcProject");
        earth_root.children.insert(slug.clone(), site_path.clone());
        doc.add_node(earth_root);

        // Site Node with WGS84 georeferencing
        let mut site_node = IfcxNode::new(&site_path)
            .with_class("IfcSite")
            .with_prop(
                &format!("{} Site", project.project_name),
                "Georeferenced Site",
                address.unwrap_or("EigenGraph Living Earth Parcel"),
            );
        site_node
            .children
            .insert(slug.clone(), bldg_path.clone());
        if let Some((lat, lon, elev_m)) = geo_opt {
            let utm_zone = (((lon + 180.0) / 6.0).floor() as i32).clamp(1, 60) as u8;
            let geo = IfcxGeoWgs84 {
                lat,
                lon,
                elev_m,
                heading_deg: heading_deg.rem_euclid(360.0),
                address: address.unwrap_or("").to_string(),
                utm_zone,
                south_hemisphere: lat < 0.0,
            };
            site_node = site_node.with_geo(&geo);
        }
        doc.add_node(site_node);

        // Building Node with lossless `eigengraph::bim::project` snapshot
        let mut bldg_node = IfcxNode::new(&bldg_path)
            .with_class("IfcBuilding")
            .with_prop(
                &project.project_name,
                "Composite Structure",
                &format!(
                    "{} Levels, {} Columns, {} Beams/Runs, {} Walls, {} Slabs, {} Footings",
                    project.levels.len(),
                    project.columns.len(),
                    project.runs.len(),
                    project.walls.len(),
                    project.slabs.len(),
                    project.footings.len()
                ),
            );
        if let Ok(v) = serde_json::to_value(project) {
            bldg_node
                .attributes
                .insert(ns::EG_BIM_PROJECT.to_string(), v);
        }

        // Helper to resolve level elevation in mm
        let level_elev = |lvl_id: usize| -> f64 {
            project
                .levels
                .iter()
                .find(|l| l.id == lvl_id)
                .map(|l| l.elevation_mm)
                .unwrap_or(0.0)
        };
        let next_level_elev = |base_id: usize, fallback_h: f64| -> f64 {
            let z0 = level_elev(base_id);
            project
                .levels
                .iter()
                .filter(|l| l.elevation_mm > z0 + 10.0)
                .map(|l| l.elevation_mm)
                .fold(f64::INFINITY, f64::min)
                .min(z0 + fallback_h)
        };

        let mut storey_nodes: HashMap<usize, IfcxNode> = HashMap::new();
        for lvl in &project.levels {
            let sp = format!("{bldg_path}/storeys/L{}", lvl.id);
            bldg_node
                .children
                .insert(format!("L{}", lvl.id), sp.clone());
            let sn = IfcxNode::new(&sp)
                .with_class("IfcBuildingStorey")
                .with_prop(
                    &lvl.name,
                    "Datum Storey",
                    &format!("Elevation {:+.2} m", lvl.elevation_mm * 0.001),
                )
                .with_attr(
                    ns::USD_XFORM,
                    json!({ "translation": [0.0, 0.0, lvl.elevation_mm * 0.001] }),
                );
            storey_nodes.insert(lvl.id, sn);
        }

        let default_lvl_id = project.levels.first().map(|l| l.id).unwrap_or(1);
        let attach_to_storey = |map: &mut HashMap<usize, IfcxNode>, lvl_id: usize, key: String, path: String| {
            let target = if map.contains_key(&lvl_id) { lvl_id } else { default_lvl_id };
            if let Some(sn) = map.get_mut(&target) {
                sn.children.insert(key, path);
            }
        };
        let mut element_nodes = Vec::new();

        // Columns
        for col in &project.columns {
            let z0 = level_elev(col.base_level_id);
            let z1 = project
                .levels
                .iter()
                .find(|l| l.id == col.top_level_id)
                .map(|l| l.elevation_mm)
                .unwrap_or_else(|| next_level_elev(col.base_level_id, 3500.0));
            let tris = col.mesh_3d(z0, z1.max(z0 + 300.0));
            let mesh = UsdGeomMesh::from_triangles_mm(&tris, [0.36, 0.57, 0.65, 1.0], 2.0);
            let key = format!("col_{}", col.id);
            let sp = format!("{bldg_path}/storeys/L{}", col.base_level_id);
            let path = format!("{sp}/columns/{key}");
            attach_to_storey(&mut storey_nodes, col.base_level_id, key, path.clone());
            element_nodes.push(
                IfcxNode::new(path)
                    .with_class("IfcColumn")
                    .with_prop(&col.name, &col.profile_name, "Structural Column")
                    .with_attr(
                        ns::EG_BIM_ELEMENT,
                        json!({
                            "kind": "column",
                            "id": col.id,
                            "widthMm": col.width_mm,
                            "depthMm": col.depth_mm,
                            "profile": col.profile_name
                        }),
                    )
                    .with_mesh(&mesh),
            );
        }

        // Beams / Runs
        for run in &project.runs {
            let z_lvl = level_elev(run.level_id);
            let tris = run.mesh_3d(z_lvl);
            let class_code = match run.category {
                RunCategory::StructuralBeam => "IfcBeam",
                RunCategory::StructuralBrace => "IfcMember",
                RunCategory::StripFooting => "IfcFooting",
                RunCategory::PlumbingPipe => "IfcPipeSegment",
                RunCategory::ElectricalCableTray | RunCategory::ElectricalConduit => {
                    "IfcCableCarrierSegment"
                }
                RunCategory::LightningConductor => "IfcCableSegment",
            };
            let mesh = UsdGeomMesh::from_triangles_mm(&tris, [0.46, 0.68, 0.76, 1.0], 2.0);
            let key = format!("run_{}", run.id);
            let sp = format!("{bldg_path}/storeys/L{}", run.level_id);
            let path = format!("{sp}/runs/{key}");
            attach_to_storey(&mut storey_nodes, run.level_id, key, path.clone());
            element_nodes.push(
                IfcxNode::new(path)
                    .with_class(class_code)
                    .with_prop(&run.name, &run.profile_name, &run.system_name)
                    .with_attr(
                        ns::EG_BIM_ELEMENT,
                        json!({
                            "kind": "run",
                            "id": run.id,
                            "widthMm": run.width_mm,
                            "depthMm": run.depth_mm,
                            "profile": run.profile_name
                        }),
                    )
                    .with_mesh(&mesh),
            );
        }

        // Walls
        for wall in &project.walls {
            let z0 = level_elev(wall.base_level_id);
            let z1 = wall
                .top_level_id
                .and_then(|tid| project.levels.iter().find(|l| l.id == tid).map(|l| l.elevation_mm))
                .unwrap_or(z0 + wall.unconnected_height_mm.max(2400.0));
            let tris = wall.mesh_3d(z0, z1.max(z0 + 300.0));
            let mesh = UsdGeomMesh::from_triangles_mm(&tris, [0.76, 0.80, 0.84, 1.0], 2.0);
            let key = format!("wall_{}", wall.id);
            let sp = format!("{bldg_path}/storeys/L{}", wall.base_level_id);
            let path = format!("{sp}/walls/{key}");
            attach_to_storey(&mut storey_nodes, wall.base_level_id, key, path.clone());
            element_nodes.push(
                IfcxNode::new(path)
                    .with_class("IfcWall")
                    .with_prop(
                        &wall.name,
                        if wall.is_structural {
                            "Reinforced Concrete Shear Wall"
                        } else {
                            "Architectural Compound Wall"
                        },
                        &format!("t = {:.0} mm, {} openings", wall.thickness_mm, wall.openings.len()),
                    )
                    .with_attr(
                        ns::EG_BIM_ELEMENT,
                        json!({
                            "kind": "wall",
                            "id": wall.id,
                            "thicknessMm": wall.thickness_mm,
                            "isStructural": wall.is_structural
                        }),
                    )
                    .with_mesh(&mesh),
            );
        }

        // Slabs & Roofs
        for slab in &project.slabs {
            let z_lvl = level_elev(slab.level_id);
            let tris = slab.mesh_3d(z_lvl);
            let class_code = if slab.is_roof { "IfcRoof" } else { "IfcSlab" };
            let mesh = UsdGeomMesh::from_triangles_mm(&tris, [0.64, 0.72, 0.78, 1.0], 2.0);
            let key = format!("slab_{}", slab.id);
            let sp = format!("{bldg_path}/storeys/L{}", slab.level_id);
            let path = format!("{sp}/slabs/{key}");
            attach_to_storey(&mut storey_nodes, slab.level_id, key, path.clone());
            element_nodes.push(
                IfcxNode::new(path)
                    .with_class(class_code)
                    .with_prop(
                        &slab.name,
                        "Reinforced Concrete Slab",
                        &format!("t = {:.0} mm, Area = {:.1} m2", slab.thickness_mm, slab.net_area_m2()),
                    )
                    .with_attr(
                        ns::EG_BIM_ELEMENT,
                        json!({
                            "kind": "slab",
                            "id": slab.id,
                            "thicknessMm": slab.thickness_mm,
                            "isRoof": slab.is_roof
                        }),
                    )
                    .with_mesh(&mesh),
            );
        }

        // Footings
        for ftg in &project.footings {
            let z_lvl = level_elev(ftg.level_id);
            let tris = ftg.mesh_3d(z_lvl);
            let mesh = UsdGeomMesh::from_triangles_mm(&tris, [0.50, 0.56, 0.62, 1.0], 2.0);
            let key = format!("footing_{}", ftg.id);
            let sp = format!("{bldg_path}/storeys/L{}", ftg.level_id);
            let path = format!("{sp}/footings/{key}");
            attach_to_storey(&mut storey_nodes, ftg.level_id, key, path.clone());
            element_nodes.push(
                IfcxNode::new(path)
                    .with_class("IfcFooting")
                    .with_prop(
                        &ftg.name,
                        "Reinforced Concrete Pad Footing",
                        &format!("{:.0}x{:.0}x{:.0} mm", ftg.width_mm, ftg.length_mm, ftg.thickness_mm),
                    )
                    .with_attr(
                        ns::EG_BIM_ELEMENT,
                        json!({
                            "kind": "footing",
                            "id": ftg.id,
                            "widthMm": ftg.width_mm,
                            "lengthMm": ftg.length_mm,
                            "thicknessMm": ftg.thickness_mm
                        }),
                    )
                    .with_mesh(&mesh),
            );
        }

        // Optional Non-Destructive 6-DOF FEM & ARI Screening Overlay
        if ari.is_some() || members.is_some_and(|m| !m.is_empty()) {
            let analysis_path = format!("{bldg_path}/analysis/ari_v2");
            bldg_node
                .children
                .insert("ari_v2".to_string(), analysis_path.clone());
            let max_dcr = members
                .map(|ms| {
                    ms.iter()
                        .map(|m| m.utilization.max(m.dcr_interaction) * 100.0)
                        .fold(0.0_f64, f64::max)
                })
                .unwrap_or(0.0);
            let overlay = if let Some(a) = ari {
                IfcxAriOverlay {
                    ari_system: a.min_system_ari,
                    ari_average: a.average_ari,
                    critical_members_count: a.critical_members_count,
                    max_dcr_pct: max_dcr.max(
                        a.member_results
                            .iter()
                            .map(|r| r.utilization_initial * 100.0)
                            .fold(0.0_f64, f64::max),
                    ),
                    svd_rank_deficiency: a.kinematic_guard.nullspace_dimension,
                    is_kinematically_stable: a.kinematic_guard.is_stable,
                }
            } else {
                IfcxAriOverlay {
                    ari_system: members
                        .and_then(|ms| ms.iter().map(|m| m.ari).reduce(f64::min))
                        .unwrap_or(1.0),
                    ari_average: members
                        .filter(|ms| !ms.is_empty())
                        .map(|ms| ms.iter().map(|m| m.ari).sum::<f64>() / ms.len() as f64)
                        .unwrap_or(1.0),
                    critical_members_count: members
                        .map(|ms| {
                            ms.iter()
                                .filter(|m| m.utilization.max(m.dcr_interaction) >= 0.85 || m.ari < 0.50)
                                .count()
                        })
                        .unwrap_or(0),
                    max_dcr_pct: max_dcr,
                    svd_rank_deficiency: 0,
                    is_kinematically_stable: true,
                }
            };
            let mut an = IfcxNode::new(analysis_path)
                .with_class("IfcStructuralAnalysisModel")
                .with_prop(
                    "ARI v2 & 6-DOF FEM Screening",
                    "Analytical Overlay",
                    &format!("System ARI {:.3} | Max DCR {:.1}%", overlay.ari_system, overlay.max_dcr_pct),
                );
            if let Ok(v) = serde_json::to_value(&overlay) {
                an.attributes.insert(ns::EG_ARI.to_string(), v);
            }
            element_nodes.push(an);
        }

        doc.add_node(bldg_node);
        for lvl in &project.levels {
            if let Some(sn) = storey_nodes.remove(&lvl.id) {
                doc.add_node(sn);
            }
        }
        for en in element_nodes {
            doc.add_node(en);
        }
        doc
    }

    /// Parses an `.ifcx` JSON file into an editable `BimProject` + resolved WGS84 placement.
    ///
    /// Supports TWO seamless paths:
    /// 1. **Lossless EigenGraph Continuation**: restores `eigengraph::bim::project` directly and
    ///    applies any non-destructive attribute overrides from stacked IFCX layers.
    /// 2. **External / AI Real-Building Capture**: reconstructs an editable `BimProject` (levels,
    ///    columns, beams, walls, slabs, footings) directly from standard `bsi::ifc::class` +
    ///    `usd::usdgeom::mesh` bounding volumes!
    pub fn parse_to_bim_project_with_geo(
        raw: &str,
        filename: &str,
        rel_path: &str,
    ) -> Result<(BimProject, Option<(f64, f64)>, GeoPlacementSource, String, String), String> {
        let doc = Self::parse(raw)?;
        let (proj, summary) = doc.to_bim_project(filename)?;

        // 1. Native `eigengraph::geo::wgs84` anchor
        let mut geo_opt: Option<(f64, f64)> = None;
        let mut source = GeoPlacementSource::StagingCampusAuto;
        let mut addr_hint = String::new();

        for node in &doc.data {
            if let Some(geo) = node.geo_wgs84() {
                if !geo.address.trim().is_empty() && addr_hint.is_empty() {
                    addr_hint = geo.address.clone();
                }
                if (geo.lat.abs() > 1e-4 || geo.lon.abs() > 1e-4)
                    && (-90.0..=90.0).contains(&geo.lat)
                    && (-180.0..=180.0).contains(&geo.lon)
                {
                    geo_opt = Some((geo.lat, geo.lon));
                    source = GeoPlacementSource::FileNativeGeo;
                    break;
                }
            }
        }

        // 2. Fallback: city hint inside address, filename, or Vault folder path
        if geo_opt.is_none() {
            let probe = format!("{} {} {}", addr_hint, rel_path, filename).to_lowercase();
            for &(city_name, city_lat, city_lon, _zoom) in GLOBAL_CITY_CATALOG {
                let cname = city_name.to_lowercase();
                if probe.contains(&cname) {
                    geo_opt = Some((city_lat, city_lon));
                    source = GeoPlacementSource::AddressOrFolderHint;
                    if addr_hint.is_empty() {
                        addr_hint = city_name.to_string();
                    }
                    break;
                }
            }
        }

        Ok((proj, geo_opt, source, addr_hint, summary))
    }

    /// Converts an `IfcxDocument` into an editable `BimProject`.
    pub fn to_bim_project(&self, fallback_name: &str) -> Result<(BimProject, String), String> {
        // Path A: Lossless EigenGraph BIM continuation snapshot
        for node in &self.data {
            if let Some(val) = node.attributes.get(ns::EG_BIM_PROJECT) {
                if let Ok(mut proj) = serde_json::from_value::<BimProject>(val.clone()) {
                    // Apply any non-destructive building-name override from `bsi::ifc::prop`
                    if let Some(n) = node
                        .attributes
                        .get(ns::BSI_PROP)
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                    {
                        if !n.trim().is_empty() {
                            proj.project_name = n.trim().to_string();
                        }
                    }
                    let summary = format!(
                        "Loaded lossless IFCX '{}' ({}L, {}C, {}B, {}W, {}S, {}F).",
                        proj.project_name,
                        proj.levels.len(),
                        proj.columns.len(),
                        proj.runs.len(),
                        proj.walls.len(),
                        proj.slabs.len(),
                        proj.footings.len()
                    );
                    return Ok((proj, summary));
                }
            }
        }

        // Path B: External or AI-captured `.ifcx` (reconstruct editable BIM directly from `bsi::ifc::class` + `usd::usdgeom::mesh`)
        let clean_name = self
            .data
            .iter()
            .find(|n| n.ifc_class() == Some("IfcBuilding"))
            .map(|n| n.display_name())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                fallback_name
                    .trim_end_matches(".ifcx")
                    .trim_end_matches(".json")
                    .to_string()
            });

        let mut proj = BimProject::empty();
        proj.project_name = clean_name;

        // Collect storey elevations (in mm) from IfcBuildingStorey nodes or mesh Z bands
        let mut level_zs_mm: Vec<f64> = Vec::new();
        for node in &self.data {
            if node.ifc_class() == Some("IfcBuildingStorey") {
                if let Some(tz) = node
                    .attributes
                    .get(ns::USD_XFORM)
                    .and_then(|v| v.get("translation"))
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.get(2))
                    .and_then(|v| v.as_f64())
                {
                    level_zs_mm.push((tz * 1000.0).round());
                }
            }
        }
        for node in &self.data {
            if matches!(
                node.ifc_class(),
                Some("IfcColumn" | "IfcBeam" | "IfcWall" | "IfcSlab" | "IfcRoof")
            ) {
                if let Some((mn, mx)) = node.world_bbox_m() {
                    level_zs_mm.push((mn[2] * 1000.0).round());
                    if matches!(node.ifc_class(), Some("IfcColumn" | "IfcWall")) {
                        level_zs_mm.push((mx[2] * 1000.0).round());
                    }
                }
            }
        }
        level_zs_mm.push(0.0);
        level_zs_mm.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut unique_zs: Vec<f64> = Vec::new();
        for z in level_zs_mm {
            if unique_zs.last().is_none_or(|&prev| (z - prev).abs() > 450.0) {
                unique_zs.push(z);
            }
        }
        if unique_zs.len() < 2 {
            unique_zs.push(3600.0);
        }
        proj.levels = unique_zs
            .iter()
            .enumerate()
            .map(|(i, &z)| BimLevel {
                id: i + 1,
                name: if i == 0 {
                    format!("Ground Level ({:+.2}m)", z * 0.001)
                } else {
                    format!("Level {} ({:+.2}m)", i + 1, z * 0.001)
                },
                elevation_mm: z,
            })
            .collect();

        let nearest_level_id = |z_mm: f64, levels: &[BimLevel]| -> usize {
            levels
                .iter()
                .min_by(|a, b| {
                    (a.elevation_mm - z_mm)
                        .abs()
                        .partial_cmp(&(b.elevation_mm - z_mm).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|l| l.id)
                .unwrap_or(1)
        };

        let mut next_id = proj.levels.len() + 2;
        for node in &self.data {
            let Some(class_code) = node.ifc_class() else {
                continue;
            };
            let Some((mn, mx)) = node.world_bbox_m() else {
                continue;
            };
            let dx_mm = ((mx[0] - mn[0]) * 1000.0).max(120.0);
            let dy_mm = ((mx[1] - mn[1]) * 1000.0).max(120.0);
            let dz_mm = ((mx[2] - mn[2]) * 1000.0).max(120.0);
            let cx_mm = 0.5 * (mn[0] + mx[0]) * 1000.0;
            let cy_mm = 0.5 * (mn[1] + mx[1]) * 1000.0;
            let z0_mm = mn[2] * 1000.0;
            let z1_mm = mx[2] * 1000.0;
            let name = node.display_name();
            let mat = node.material_label();

            match class_code {
                "IfcColumn" => {
                    let base_id = nearest_level_id(z0_mm, &proj.levels);
                    let mut top_id = nearest_level_id(z1_mm, &proj.levels);
                    if top_id == base_id {
                        top_id = (base_id + 1).min(proj.levels.len());
                    }
                    let w = dx_mm.clamp(200.0, 1500.0);
                    let d = dy_mm.clamp(200.0, 1500.0);
                    let id = next_id;
                    next_id += 1;
                    proj.columns.push(BimColumn {
                        id,
                        name,
                        base_level_id: base_id,
                        top_level_id: top_id,
                        pos: P2(cx_mm, cy_mm),
                        width_mm: w,
                        depth_mm: d,
                        rotation_deg: 0.0,
                        is_round: false,
                        is_wf_profile: mat.to_ascii_uppercase().contains("WF")
                            || mat.to_ascii_uppercase().contains("STEEL"),
                        profile_name: if mat == "Unspecified" {
                            format!("RC {:.0}x{:.0}", w, d)
                        } else {
                            mat
                        },
                    });
                }
                "IfcBeam" | "IfcMember" => {
                    let lvl_id = nearest_level_id(z1_mm, &proj.levels);
                    let (start, end, w_mm) = if dx_mm >= dy_mm {
                        (P2(mn[0] * 1000.0, cy_mm), P2(mx[0] * 1000.0, cy_mm), dy_mm.clamp(180.0, 800.0))
                    } else {
                        (P2(cx_mm, mn[1] * 1000.0), P2(cx_mm, mx[1] * 1000.0), dx_mm.clamp(180.0, 800.0))
                    };
                    let d_mm = dz_mm.clamp(250.0, 1200.0);
                    let id = next_id;
                    next_id += 1;
                    proj.runs.push(BimRun {
                        id,
                        name,
                        category: if class_code == "IfcMember" {
                            RunCategory::StructuralBrace
                        } else {
                            RunCategory::StructuralBeam
                        },
                        level_id: lvl_id,
                        start,
                        end,
                        start_offset_mm: 0.0,
                        end_offset_mm: 0.0,
                        justify: RunJustify::Top,
                        profile_name: if mat == "Unspecified" {
                            format!("RC {:.0}x{:.0}", w_mm, d_mm)
                        } else {
                            mat
                        },
                        width_mm: w_mm,
                        depth_mm: d_mm,
                        is_round: false,
                        is_wf_profile: false,
                        system_name: "IFCX Primary Frame".to_string(),
                    });
                }
                "IfcWall" => {
                    let base_id = nearest_level_id(z0_mm, &proj.levels);
                    let top_id = nearest_level_id(z1_mm, &proj.levels);
                    let (start, end, t_mm) = if dx_mm >= dy_mm {
                        (P2(mn[0] * 1000.0, cy_mm), P2(mx[0] * 1000.0, cy_mm), dy_mm.clamp(120.0, 600.0))
                    } else {
                        (P2(cx_mm, mn[1] * 1000.0), P2(cx_mm, mx[1] * 1000.0), dx_mm.clamp(120.0, 600.0))
                    };
                    let id = next_id;
                    next_id += 1;
                    proj.walls.push(BimWall {
                        id,
                        name,
                        base_level_id: base_id,
                        top_level_id: if top_id != base_id { Some(top_id) } else { None },
                        unconnected_height_mm: dz_mm.max(2400.0),
                        start,
                        end,
                        thickness_mm: t_mm,
                        is_structural: true,
                        layers: BimWall::default_compound_layers(t_mm),
                        openings: Vec::new(),
                    });
                }
                "IfcSlab" | "IfcRoof" => {
                    let lvl_id = nearest_level_id(z1_mm, &proj.levels);
                    let id = next_id;
                    next_id += 1;
                    proj.slabs.push(BimSlab {
                        id,
                        name,
                        level_id: lvl_id,
                        boundary: vec![
                            P2(mn[0] * 1000.0, mn[1] * 1000.0),
                            P2(mx[0] * 1000.0, mn[1] * 1000.0),
                            P2(mx[0] * 1000.0, mx[1] * 1000.0),
                            P2(mn[0] * 1000.0, mx[1] * 1000.0),
                        ],
                        openings: Vec::new(),
                        thickness_mm: dz_mm.clamp(120.0, 600.0),
                        is_roof: class_code == "IfcRoof",
                        roof_slope_deg: 0.0,
                    });
                }
                "IfcFooting" => {
                    let lvl_id = nearest_level_id(z1_mm, &proj.levels);
                    let id = next_id;
                    next_id += 1;
                    proj.footings.push(BimFooting {
                        id,
                        name,
                        level_id: lvl_id,
                        pos: P2(cx_mm, cy_mm),
                        width_mm: dx_mm.clamp(800.0, 5000.0),
                        length_mm: dy_mm.clamp(800.0, 5000.0),
                        thickness_mm: dz_mm.clamp(300.0, 1500.0),
                    });
                }
                _ => {}
            }
        }
        proj.next_id = next_id;

        if proj.is_empty() {
            return Err("IFCX document contains no reconstructable building geometry (IfcColumn, IfcBeam, IfcWall, IfcSlab, or IfcFooting).".to_string());
        }

        let summary = format!(
            "Reconstructed editable BIM from IFCX '{}' ({}L, {}C, {}B, {}W, {}S, {}F).",
            proj.project_name,
            proj.levels.len(),
            proj.columns.len(),
            proj.runs.len(),
            proj.walls.len(),
            proj.slabs.len(),
            proj.footings.len()
        );
        Ok((proj, summary))
    }

    // =========================================================================
    // 4. STRUCTURALMODEL (6-DOF FEM) <-> IFCX BRIDGE (REPLACES LEGACY STEP)
    // =========================================================================

    /// Exports a 6-DOF `StructuralModel` (and optional ARI assessment) into a pure `.ifcx` document.
    /// Replaces legacy `IfcExporter` and `IfcSamExporter`.
    pub fn from_structural_model(
        model: &StructuralModel,
        ari: Option<&RedundancyAssessment>,
        sam_mode: bool,
    ) -> Self {
        let slug = sanitize_slug(&model.name);
        let mut doc = Self::new(if sam_mode {
            format!("sam_{slug}")
        } else {
            format!("model_{slug}")
        });
        let root_path = format!("/earth/sites/{slug}/buildings/{slug}");
        let mut root = IfcxNode::new(&root_path)
            .with_class(if sam_mode {
                "IfcStructuralAnalysisModel"
            } else {
                "IfcBuilding"
            })
            .with_prop(
                &model.name,
                "6-DOF Structural Model",
                &format!("{} Nodes, {} Elements", model.nodes.len(), model.elements.len()),
            );

        let mut node_entries = Vec::new();
        for n in &model.nodes {
            let key = format!("N{}", n.id);
            let path = format!("{root_path}/nodes/{key}");
            root.children.insert(key, path.clone());
            node_entries.push(
                IfcxNode::new(path)
                    .with_class("IfcStructuralPointConnection")
                    .with_prop(&format!("Node {}", n.id), "Analytical Node", "")
                    .with_attr(
                        ns::EG_FEM_NODE,
                        json!({
                            "id": n.id,
                            "pos": [n.x, n.y, n.z],
                            "fixity": n.fixity
                        }),
                    ),
            );
        }

        let mut elem_entries = Vec::new();
        for e in &model.elements {
            let (Some(n1), Some(n2)) = (model.find_node(e.start_node), model.find_node(e.end_node))
            else {
                continue;
            };
            let dz = (n2.z - n1.z).abs();
            let dh = (n2.x - n1.x).hypot(n2.y - n1.y);
            let class_code = if sam_mode {
                "IfcStructuralCurveMember"
            } else if dz > dh * 1.5 {
                "IfcColumn"
            } else {
                "IfcBeam"
            };
            let half_w = (e.section.width * 0.5).max(0.12);
            let half_d = (e.section.depth * 0.5).max(0.12);
            let mn = [
                n1.x.min(n2.x) - half_w,
                n1.y.min(n2.y) - half_w,
                n1.z.min(n2.z) - if dz <= dh * 1.5 { half_d } else { 0.0 },
            ];
            let mx = [
                n1.x.max(n2.x) + half_w,
                n1.y.max(n2.y) + half_w,
                n1.z.max(n2.z) + if dz <= dh * 1.5 { half_d } else { 0.0 },
            ];
            let mesh = UsdGeomMesh::from_box_m(mn, mx, [0.36, 0.57, 0.65, 1.0], 2.0);
            let key = format!("E{}", e.id);
            let path = format!("{root_path}/members/{key}");
            root.children.insert(key, path.clone());
            let mut en = IfcxNode::new(path)
                .with_class(class_code)
                .with_prop(&e.name, &e.material.name, &e.section.name)
                .with_attr(
                    ns::EG_FEM_MEMBER,
                    json!({
                        "id": e.id,
                        "startNode": e.start_node,
                        "endNode": e.end_node,
                        "start": [n1.x, n1.y, n1.z],
                        "end": [n2.x, n2.y, n2.z],
                        "elementType": e.element_type,
                        "section": e.section,
                        "material": e.material
                    }),
                )
                .with_mesh(&mesh);

            if let Some(a) = ari {
                if let Some(mr) = a.member_results.iter().find(|r| r.member_id == e.id) {
                    en.attributes.insert(
                        ns::EG_ARI.to_string(),
                        json!({
                            "ari": mr.ari,
                            "dcr": mr.utilization_initial,
                            "svdMargin": mr.svd_margin,
                            "isCritical": mr.is_critical
                        }),
                    );
                }
            }
            elem_entries.push(en);
        }

        for shell in &model.shells {
            let key = format!("S{}", shell.id);
            let path = format!("{root_path}/shells/{key}");
            root.children.insert(key, path.clone());
            let class_code = if sam_mode {
                "IfcStructuralSurfaceMember"
            } else if shell.is_shear_wall {
                "IfcWall"
            } else {
                "IfcSlab"
            };
            elem_entries.push(
                IfcxNode::new(path)
                    .with_class(class_code)
                    .with_prop(
                        &shell.name,
                        &shell.material.name,
                        if shell.is_shear_wall {
                            "Shear Wall Surface"
                        } else {
                            "Slab Surface"
                        },
                    )
                    .with_attr(
                        ns::EG_FEM_MEMBER,
                        json!({
                            "id": shell.id,
                            "nodes": shell.nodes,
                            "thicknessM": shell.thickness_m,
                            "isShearWall": shell.is_shear_wall
                        }),
                    ),
            );
        }

        if let Some(a) = ari {
            let overlay = IfcxAriOverlay {
                ari_system: a.min_system_ari,
                ari_average: a.average_ari,
                critical_members_count: a.critical_members_count,
                max_dcr_pct: a
                    .member_results
                    .iter()
                    .map(|r| r.utilization_initial * 100.0)
                    .fold(0.0_f64, f64::max),
                svd_rank_deficiency: a.kinematic_guard.nullspace_dimension,
                is_kinematically_stable: a.kinematic_guard.is_stable,
            };
            if let Ok(v) = serde_json::to_value(&overlay) {
                root.attributes.insert(ns::EG_ARI.to_string(), v);
            }
        }

        doc.add_node(root);
        for n in node_entries {
            doc.add_node(n);
        }
        for e in elem_entries {
            doc.add_node(e);
        }
        doc
    }

    /// Reconstructs a 6-DOF `StructuralModel` directly from an `.ifcx` document.
    /// Replaces legacy STEP `IfcImporter`.
    pub fn to_structural_model(&self, fallback_name: &str) -> Result<StructuralModel, String> {
        // 1. Direct analytical FEM nodes & members if present
        let mut nodes_by_id: BTreeMap<usize, Node3D> = BTreeMap::new();
        let mut elements: Vec<Element3D> = Vec::new();
        let mut model_name = fallback_name.to_string();

        for node in &self.data {
            if matches!(
                node.ifc_class(),
                Some("IfcBuilding" | "IfcStructuralAnalysisModel")
            ) {
                let dn = node.display_name();
                if !dn.trim().is_empty() {
                    model_name = dn;
                }
            }
            if let Some(fn_val) = node.attributes.get(ns::EG_FEM_NODE) {
                if let (Some(id), Some(pos)) = (
                    fn_val.get("id").and_then(|v| v.as_u64()).map(|v| v as usize),
                    fn_val.get("pos").and_then(|v| v.as_array()),
                ) {
                    if pos.len() >= 3 {
                        let fixity = fn_val
                            .get("fixity")
                            .and_then(|v| serde_json::from_value::<BoundaryFixity>(v.clone()).ok())
                            .unwrap_or_else(BoundaryFixity::free);
                        nodes_by_id.insert(
                            id,
                            Node3D::new(
                                id,
                                pos[0].as_f64().unwrap_or(0.0),
                                pos[1].as_f64().unwrap_or(0.0),
                                pos[2].as_f64().unwrap_or(0.0),
                            )
                            .with_fixity(fixity),
                        );
                    }
                }
            }
            if let Some(em_val) = node.attributes.get(ns::EG_FEM_MEMBER) {
                if let (Some(id), Some(s_id), Some(e_id)) = (
                    em_val.get("id").and_then(|v| v.as_u64()).map(|v| v as usize),
                    em_val.get("startNode").and_then(|v| v.as_u64()).map(|v| v as usize),
                    em_val.get("endNode").and_then(|v| v.as_u64()).map(|v| v as usize),
                ) {
                    let elem_type = em_val
                        .get("elementType")
                        .and_then(|v| serde_json::from_value::<ElementType>(v.clone()).ok())
                        .unwrap_or(ElementType::Frame3D);
                    let section = em_val
                        .get("section")
                        .and_then(|v| serde_json::from_value::<CrossSection>(v.clone()).ok())
                        .unwrap_or_else(CrossSection::wf_300x150);
                    let material = em_val
                        .get("material")
                        .and_then(|v| serde_json::from_value::<Material>(v.clone()).ok())
                        .unwrap_or_else(Material::steel_q355);
                    elements.push(
                        Element3D::new(id, node.display_name(), s_id, e_id)
                            .with_type(elem_type)
                            .with_section(section)
                            .with_material(material),
                    );
                }
            }
        }

        if !nodes_by_id.is_empty() && !elements.is_empty() {
            let mut sm = StructuralModel::new(model_name);
            for (_, n) in nodes_by_id {
                sm.add_node(n);
            }
            for e in elements {
                sm.add_element(e);
            }
            // Ensure at least one gravity load exists if none specified
            if let Some(max_z) = sm.nodes.iter().map(|n| n.z).reduce(f64::max) {
                let top_ids: Vec<usize> = sm
                    .nodes
                    .iter()
                    .filter(|n| (n.z - max_z).abs() < 0.25 && n.fixity.is_free())
                    .map(|n| n.id)
                    .collect();
                for nid in top_ids {
                    sm.add_load(NodalLoad::force_z(nid, -25_000.0));
                }
            }
            return Ok(sm);
        }

        // 2. Fallback: convert through BimProject -> 6-DOF StructuralModel
        let (bim_proj, _) = self.to_bim_project(fallback_name)?;
        Ok(bim_proj.to_structural_model())
    }

    // =========================================================================
    // 5. BUILT-IN SAMPLE IFCX MODELS (DUPLEX VILLA + AI REAL-BUILDING CAPTURE)
    // =========================================================================

    /// Clean 2-Storey Structural Duplex Villa in 100% pure IFCX JSON.
    pub fn sample_villa_ifcx() -> Self {
        let mut proj = BimProject::empty();
        proj.project_name = "OpenBIM Duplex Villa (IFCX)".to_string();
        proj.levels = vec![
            BimLevel {
                id: 1,
                name: "Ground Level (+0.00m)".to_string(),
                elevation_mm: 0.0,
            },
            BimLevel {
                id: 2,
                name: "Level 2 (+3.60m)".to_string(),
                elevation_mm: 3600.0,
            },
            BimLevel {
                id: 3,
                name: "Roof Level (+7.20m)".to_string(),
                elevation_mm: 7200.0,
            },
        ];
        let mut id = 10usize;
        let grid_x = [-4000.0, 0.0, 4000.0];
        let grid_y = [-3000.0, 3000.0];
        for (ix, &x) in grid_x.iter().enumerate() {
            for (iy, &y) in grid_y.iter().enumerate() {
                id += 1;
                proj.footings.push(BimFooting {
                    id,
                    name: format!("F{}-{}", ix + 1, iy + 1),
                    level_id: 1,
                    pos: P2(x, y),
                    width_mm: 1400.0,
                    length_mm: 1400.0,
                    thickness_mm: 450.0,
                });
                for &(bl, tl, storey_tag) in &[(1, 2, "L1"), (2, 3, "L2")] {
                    id += 1;
                    proj.columns.push(BimColumn {
                        id,
                        name: format!("C{}-{}_{}", ix + 1, iy + 1, storey_tag),
                        base_level_id: bl,
                        top_level_id: tl,
                        pos: P2(x, y),
                        width_mm: 400.0,
                        depth_mm: 400.0,
                        rotation_deg: 0.0,
                        is_round: false,
                        is_wf_profile: false,
                        profile_name: "RC 400x400".to_string(),
                    });
                }
            }
        }
        for &(lvl_id, tag) in &[(2, "L2"), (3, "RF")] {
            for &y in &grid_y {
                for w in grid_x.windows(2) {
                    id += 1;
                    proj.runs.push(BimRun {
                        id,
                        name: format!("BX_{tag}_{id}"),
                        category: RunCategory::StructuralBeam,
                        level_id: lvl_id,
                        start: P2(w[0], y),
                        end: P2(w[1], y),
                        start_offset_mm: 0.0,
                        end_offset_mm: 0.0,
                        justify: RunJustify::Top,
                        profile_name: "RC 300x500".to_string(),
                        width_mm: 300.0,
                        depth_mm: 500.0,
                        is_round: false,
                        is_wf_profile: false,
                        system_name: "Primary RC Frame".to_string(),
                    });
                }
            }
            for &x in &grid_x {
                id += 1;
                proj.runs.push(BimRun {
                    id,
                    name: format!("BY_{tag}_{id}"),
                    category: RunCategory::StructuralBeam,
                    level_id: lvl_id,
                    start: P2(x, grid_y[0]),
                    end: P2(x, grid_y[1]),
                    start_offset_mm: 0.0,
                    end_offset_mm: 0.0,
                    justify: RunJustify::Top,
                    profile_name: "RC 300x500".to_string(),
                    width_mm: 300.0,
                    depth_mm: 500.0,
                    is_round: false,
                    is_wf_profile: false,
                    system_name: "Primary RC Frame".to_string(),
                });
            }
            id += 1;
            proj.slabs.push(BimSlab {
                id,
                name: format!("Slab_{tag}"),
                level_id: lvl_id,
                boundary: vec![
                    P2(-4200.0, -3200.0),
                    P2(4200.0, -3200.0),
                    P2(4200.0, 3200.0),
                    P2(-4200.0, 3200.0),
                ],
                openings: Vec::new(),
                thickness_mm: 160.0,
                is_roof: lvl_id == 3,
                roof_slope_deg: 0.0,
            });
        }
        proj.next_id = id + 1;
        Self::from_bim_project(
            &proj,
            Some((-6.9175, 107.6191, 768.0)),
            0.0,
            Some("Bandung"),
            None,
            None,
        )
    }

    /// Demonstrates an external / AI-reconstructed real-world building `.ifcx` containing ONLY
    /// standard `bsi::ifc::class` + `usd::usdgeom::mesh` + `eigengraph::geo::wgs84` (no EigenGraph
    /// proprietary project blob) — proving future AI scans drop directly onto Earth and convert
    /// into editable 6-DOF BIM/FEM models!
    pub fn sample_ai_real_building_capture_ifcx() -> Self {
        let mut doc = Self::new("ai_capture_bandung_pavilion");
        doc.imports.push(IfcxImport {
            uri: "eigengraph://earth_globe.ifcx".to_string(),
            integrity: None,
            role: Some("earth-base".to_string()),
        });
        let site = "/earth/sites/bandung_ai_capture";
        let bldg = format!("{site}/buildings/pavilion_01");
        let geo = IfcxGeoWgs84 {
            lat: -6.8915,
            lon: 107.6107,
            elev_m: 775.0,
            heading_deg: 12.0,
            address: "Bandung".to_string(),
            utm_zone: 48,
            south_hemisphere: true,
        };
        doc.add_node(IfcxNode::new("/earth").with_class("IfcProject"));
        doc.add_node(
            IfcxNode::new(site)
                .with_class("IfcSite")
                .with_prop("Bandung AI Capture Parcel", "Real-World Scan", "AI Vision -> IFCX")
                .with_geo(&geo),
        );
        doc.add_node(
            IfcxNode::new(&bldg)
                .with_class("IfcBuilding")
                .with_prop("AI Captured Bandung Pavilion", "Reinforced Concrete", "Reconstructed from Real-World Capture"),
        );
        // 4 corners of a 6m x 6m x 3.6m bay
        for (idx, &(x, y)) in [(-3.0, -3.0), (3.0, -3.0), (3.0, 3.0), (-3.0, 3.0)].iter().enumerate() {
            let mesh = UsdGeomMesh::from_box_m([x - 0.2, y - 0.2, 0.0], [x + 0.2, y + 0.2, 3.6], [0.36, 0.57, 0.65, 1.0], 2.0);
            doc.add_node(
                IfcxNode::new(format!("{bldg}/columns/C{}", idx + 1))
                    .with_class("IfcColumn")
                    .with_prop(&format!("AI_Col_{}", idx + 1), "RC 400x400", "AI-Detected Column")
                    .with_mesh(&mesh),
            );
        }
        // Perimeter roof beams
        for (idx, (mn, mx)) in [
            ([-3.0, -3.15, 3.1], [3.0, -2.85, 3.6]),
            ([-3.0, 2.85, 3.1], [3.0, 3.15, 3.6]),
            ([-3.15, -3.0, 3.1], [-2.85, 3.0, 3.6]),
            ([2.85, -3.0, 3.1], [3.15, 3.0, 3.6]),
        ]
        .iter()
        .enumerate()
        {
            let mesh = UsdGeomMesh::from_box_m(*mn, *mx, [0.46, 0.68, 0.76, 1.0], 2.0);
            doc.add_node(
                IfcxNode::new(format!("{bldg}/beams/B{}", idx + 1))
                    .with_class("IfcBeam")
                    .with_prop(&format!("AI_Beam_{}", idx + 1), "RC 300x500", "AI-Detected Beam")
                    .with_mesh(&mesh),
            );
        }
        // Roof slab
        let slab_mesh = UsdGeomMesh::from_box_m([-3.2, -3.2, 3.6], [3.2, 3.2, 3.76], [0.64, 0.72, 0.78, 1.0], 2.0);
        doc.add_node(
            IfcxNode::new(format!("{bldg}/slabs/Roof"))
                .with_class("IfcRoof")
                .with_prop("AI_Roof_Slab", "Concrete Slab 160mm", "AI-Detected Roof Deck")
                .with_mesh(&slab_mesh),
        );
        doc
    }
}

// =============================================================================
// 6. PURE-JSON IFCX HEALTH AUDIT, SPATIAL CLASH, LAYER DIFF & EDIT SESSION
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfcxAuditStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone)]
pub struct IfcxAuditItem {
    pub code: String,
    pub title: String,
    pub status: IfcxAuditStatus,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct IfcxHealthReport {
    pub total_nodes: usize,
    pub building_elements: usize,
    pub mesh_nodes: usize,
    pub has_georeferencing: bool,
    pub conformance_score_pct: f64,
    pub verdict: String,
    pub checks: Vec<IfcxAuditItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfcxClashSeverity {
    Critical,
    Major,
    Clearance,
}

#[derive(Debug, Clone)]
pub struct IfcxClashItem {
    pub path_a: String,
    pub path_b: String,
    pub class_a: String,
    pub class_b: String,
    pub overlap_mm: f64,
    pub severity: IfcxClashSeverity,
}

#[derive(Debug, Clone)]
pub struct IfcxClashReport {
    pub checked_pairs: usize,
    pub clashes: Vec<IfcxClashItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfcxDiffKind {
    Added,
    Deleted,
    Modified,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct IfcxNodeDiff {
    pub path: String,
    pub class_code: String,
    pub kind: IfcxDiffKind,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct IfcxDiffReport {
    pub added: usize,
    pub deleted: usize,
    pub modified: usize,
    pub unchanged: usize,
    pub items: Vec<IfcxNodeDiff>,
}

impl IfcxDocument {
    /// Runs an OpenBIM IFCX (IFC 5) schema conformance and geometry health audit.
    pub fn audit_health(&self) -> IfcxHealthReport {
        let mut checks = Vec::new();
        let has_version = !self.header.ifcx_version.trim().is_empty();
        checks.push(IfcxAuditItem {
            code: "IFCX-01".to_string(),
            title: "IFCX Header & Schema Version".to_string(),
            status: if has_version {
                IfcxAuditStatus::Pass
            } else {
                IfcxAuditStatus::Fail
            },
            detail: format!("ifcxVersion = '{}', units = {}", self.header.ifcx_version, self.header.units.length),
        });

        let has_root = self.data.iter().any(|n| n.path.starts_with("/earth") || n.ifc_class() == Some("IfcProject"));
        checks.push(IfcxAuditItem {
            code: "IFCX-02".to_string(),
            title: "Unified Living Earth Root (/earth)".to_string(),
            status: if has_root {
                IfcxAuditStatus::Pass
            } else {
                IfcxAuditStatus::Warning
            },
            detail: if has_root {
                "Rooted cleanly under /earth Living Earth hierarchy.".to_string()
            } else {
                "Standalone IFCX graph; recommend rooting under /earth/sites/...".to_string()
            },
        });

        let has_geo = self.data.iter().any(|n| n.geo_wgs84().is_some());
        checks.push(IfcxAuditItem {
            code: "IFCX-03".to_string(),
            title: "WGS84 Georeferencing (eigengraph::geo::wgs84)".to_string(),
            status: if has_geo {
                IfcxAuditStatus::Pass
            } else {
                IfcxAuditStatus::Warning
            },
            detail: if has_geo {
                "Geodetic lat/lon/elev anchor present for Earth Globe placement.".to_string()
            } else {
                "No explicit WGS84 anchor; will resolve via city hint or staging campus.".to_string()
            },
        });

        let mesh_nodes = self.data.iter().filter(|n| n.attributes.contains_key(ns::USD_MESH)).count();
        let building_elements = self
            .data
            .iter()
            .filter(|n| {
                matches!(
                    n.ifc_class(),
                    Some("IfcColumn" | "IfcBeam" | "IfcWall" | "IfcSlab" | "IfcRoof" | "IfcFooting" | "IfcMember")
                )
            })
            .count();
        checks.push(IfcxAuditItem {
            code: "IFCX-04".to_string(),
            title: "Vector Geometry Payloads (usd::usdgeom::mesh)".to_string(),
            status: if mesh_nodes > 0 {
                IfcxAuditStatus::Pass
            } else {
                IfcxAuditStatus::Warning
            },
            detail: format!("{mesh_nodes} nodes carry GPU-ready usd::usdgeom::mesh geometry."),
        });

        let pass_pts: f64 = checks
            .iter()
            .map(|c| match c.status {
                IfcxAuditStatus::Pass => 1.0,
                IfcxAuditStatus::Warning => 0.65,
                IfcxAuditStatus::Fail => 0.0,
            })
            .sum();
        let score = (pass_pts / checks.len() as f64) * 100.0;
        let verdict = if score >= 90.0 {
            "CERTIFIED IFCX READY".to_string()
        } else if score >= 65.0 {
            "VALID IFCX (ADVISORIES)".to_string()
        } else {
            "REQUIRES SCHEMA REMEDIATION".to_string()
        };

        IfcxHealthReport {
            total_nodes: self.data.len(),
            building_elements,
            mesh_nodes,
            has_georeferencing: has_geo,
            conformance_score_pct: score,
            verdict,
            checks,
        }
    }

    /// Runs AABB spatial clash detection across building element `usd::usdgeom::mesh` nodes.
    pub fn detect_clashes(&self, tolerance_mm: f64) -> IfcxClashReport {
        let tol_m = (tolerance_mm.max(0.0)) * 0.001;
        let elems: Vec<(&IfcxNode, [f64; 3], [f64; 3])> = self
            .data
            .iter()
            .filter(|n| {
                matches!(
                    n.ifc_class(),
                    Some("IfcColumn" | "IfcBeam" | "IfcWall" | "IfcSlab" | "IfcRoof" | "IfcFooting")
                )
            })
            .filter_map(|n| {
                let (mn, mx) = n.world_bbox_m()?;
                Some((n, mn, mx))
            })
            .collect();

        let mut clashes = Vec::new();
        let mut checked_pairs = 0usize;
        for i in 0..elems.len() {
            for j in (i + 1)..elems.len() {
                checked_pairs += 1;
                let (na, mna, mxa) = elems[i];
                let (nb, mnb, mxb) = elems[j];
                let ox = (mxa[0].min(mxb[0]) - mna[0].max(mnb[0])) - tol_m;
                let oy = (mxa[1].min(mxb[1]) - mna[1].max(mnb[1])) - tol_m;
                let oz = (mxa[2].min(mxb[2]) - mna[2].max(mnb[2])) - tol_m;
                if ox > 0.0 && oy > 0.0 && oz > 0.0 {
                    let min_ov_mm = ox.min(oy).min(oz) * 1000.0;
                    // Ignore standard column-beam or column-footing structural seating interfaces (< 40mm or same-family joint)
                    if min_ov_mm < 25.0 {
                        continue;
                    }
                    let severity = if min_ov_mm > 150.0 {
                        IfcxClashSeverity::Critical
                    } else if min_ov_mm > 60.0 {
                        IfcxClashSeverity::Major
                    } else {
                        IfcxClashSeverity::Clearance
                    };
                    clashes.push(IfcxClashItem {
                        path_a: na.path.clone(),
                        path_b: nb.path.clone(),
                        class_a: na.ifc_class().unwrap_or("Unknown").to_string(),
                        class_b: nb.ifc_class().unwrap_or("Unknown").to_string(),
                        overlap_mm: min_ov_mm,
                        severity,
                    });
                }
            }
        }
        IfcxClashReport {
            checked_pairs,
            clashes,
        }
    }

    /// Computes a path-by-path diff between a baseline `.ifcx` document and an updated `.ifcx` layer.
    pub fn diff_against(&self, baseline: &IfcxDocument) -> IfcxDiffReport {
        let base_map: BTreeMap<&str, &IfcxNode> =
            baseline.data.iter().map(|n| (n.path.as_str(), n)).collect();
        let curr_map: BTreeMap<&str, &IfcxNode> =
            self.data.iter().map(|n| (n.path.as_str(), n)).collect();

        let mut items = Vec::new();
        let (mut added, mut deleted, mut modified, mut unchanged) = (0, 0, 0, 0);

        for (p, cn) in &curr_map {
            match base_map.get(p) {
                None => {
                    added += 1;
                    items.push(IfcxNodeDiff {
                        path: (*p).to_string(),
                        class_code: cn.ifc_class().unwrap_or("Node").to_string(),
                        kind: IfcxDiffKind::Added,
                        detail: "Added in current IFCX layer".to_string(),
                    });
                }
                Some(bn) => {
                    if *bn == *cn {
                        unchanged += 1;
                        items.push(IfcxNodeDiff {
                            path: (*p).to_string(),
                            class_code: cn.ifc_class().unwrap_or("Node").to_string(),
                            kind: IfcxDiffKind::Unchanged,
                            detail: "Identical".to_string(),
                        });
                    } else {
                        modified += 1;
                        items.push(IfcxNodeDiff {
                            path: (*p).to_string(),
                            class_code: cn.ifc_class().unwrap_or("Node").to_string(),
                            kind: IfcxDiffKind::Modified,
                            detail: format!("{} attributes updated", cn.attributes.len()),
                        });
                    }
                }
            }
        }
        for (p, bn) in &base_map {
            if !curr_map.contains_key(p) {
                deleted += 1;
                items.push(IfcxNodeDiff {
                    path: (*p).to_string(),
                    class_code: bn.ifc_class().unwrap_or("Node").to_string(),
                    kind: IfcxDiffKind::Deleted,
                    detail: "Removed compared to baseline".to_string(),
                });
            }
        }

        IfcxDiffReport {
            added,
            deleted,
            modified,
            unchanged,
            items,
        }
    }
}

/// Lightweight transactional Undo/Redo editor for `.ifcx` documents in OpenBIM IFCX Studio.
#[derive(Debug, Clone, Default)]
pub struct IfcxEditSession {
    undo_stack: Vec<IfcxDocument>,
    redo_stack: Vec<IfcxDocument>,
}

impl IfcxEditSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn push_snapshot(&mut self, prev: IfcxDocument) {
        self.undo_stack.push(prev);
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self, current: &mut IfcxDocument) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            let old = std::mem::replace(current, prev);
            self.redo_stack.push(old);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, current: &mut IfcxDocument) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let old = std::mem::replace(current, next);
            self.undo_stack.push(old);
            true
        } else {
            false
        }
    }
}

// =============================================================================
// 7. INTERNAL PURE-VECTOR MATH HELPERS
// =============================================================================

fn sanitize_slug(raw: &str) -> String {
    let s: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        "eigengraph_site".to_string()
    } else {
        trimmed.to_string()
    }
}

#[inline]
fn color32_to_rgba(c: egui::Color32) -> [f32; 4] {
    [
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
        c.a() as f32 / 255.0,
    ]
}

#[inline]
fn mix_rgba(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let k = t.clamp(0.0, 1.0);
    [
        a[0] * (1.0 - k) + b[0] * k,
        a[1] * (1.0 - k) + b[1] * k,
        a[2] * (1.0 - k) + b[2] * k,
        1.0,
    ]
}

#[inline]
fn norm3_f64(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 1e-12 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0, 1.0, 0.0]
    }
}

#[inline]
fn cross3_f64(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn ear_clip_indices_2d(pts: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    let mut area2 = 0.0_f64;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        area2 += a[0] * b[1] - b[0] * a[1];
    }
    if area2.abs() < 1e-12 {
        return Vec::new();
    }
    let mut rem: Vec<usize> = if area2 > 0.0 {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };

    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| -> f64 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let in_tri = |p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]| -> bool {
        cross(a, b, p) >= -1e-9 && cross(b, c, p) >= -1e-9 && cross(c, a, p) >= -1e-9
    };

    let mut tris = Vec::with_capacity(n.saturating_sub(2));
    let mut guard = 0usize;
    let max_iters = n * n + 8;
    while rem.len() > 3 && guard < max_iters {
        guard += 1;
        let m = rem.len();
        let mut clipped = false;
        for i in 0..m {
            let i0 = rem[(i + m - 1) % m];
            let i1 = rem[i];
            let i2 = rem[(i + 1) % m];
            let (a, b, c) = (pts[i0], pts[i1], pts[i2]);
            if cross(a, b, c) <= 1e-10 {
                continue;
            }
            let any_inside = rem
                .iter()
                .any(|&idx| idx != i0 && idx != i1 && idx != i2 && in_tri(pts[idx], a, b, c));
            if !any_inside {
                tris.push([i0, i1, i2]);
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
        tris.push([rem[0], rem[1], rem[2]]);
    }
    tris
}

#[allow(clippy::too_many_arguments)]
fn push_subdivided_spherical_tri(
    out_pts: &mut Vec<[f64; 3]>,
    out_norms: &mut Vec<[f64; 3]>,
    out_cols: &mut Vec<[f32; 4]>,
    out_meta: &mut Vec<[f32; 4]>,
    out_indices: &mut Vec<u32>,
    out_counts: &mut Vec<u32>,
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
    radius: f64,
    color: [f32; 4],
    depth: u8,
) {
    let dist_sq = |u: [f64; 3], v: [f64; 3]| -> f64 {
        (u[0] - v[0]).powi(2) + (u[1] - v[1]).powi(2) + (u[2] - v[2]).powi(2)
    };
    let max_e2 = dist_sq(a, b).max(dist_sq(b, c)).max(dist_sq(c, a));
    if depth < 3 && max_e2 > 0.022 {
        let ab = norm3_f64([a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
        let bc = norm3_f64([b[0] + c[0], b[1] + c[1], b[2] + c[2]]);
        let ca = norm3_f64([c[0] + a[0], c[1] + a[1], c[2] + a[2]]);
        push_subdivided_spherical_tri(out_pts, out_norms, out_cols, out_meta, out_indices, out_counts, a, ab, ca, radius, color, depth + 1);
        push_subdivided_spherical_tri(out_pts, out_norms, out_cols, out_meta, out_indices, out_counts, ab, b, bc, radius, color, depth + 1);
        push_subdivided_spherical_tri(out_pts, out_norms, out_cols, out_meta, out_indices, out_counts, ca, bc, c, radius, color, depth + 1);
        push_subdivided_spherical_tri(out_pts, out_norms, out_cols, out_meta, out_indices, out_counts, ab, bc, ca, radius, color, depth + 1);
        return;
    }
    let base = out_pts.len() as u32;
    for n in [a, b, c] {
        out_pts.push([n[0] * radius, n[1] * radius, n[2] * radius]);
        out_norms.push(n);
        out_cols.push(color);
        out_meta.push([1.0, 0.0, 1.0, 0.0]);
    }
    out_indices.extend_from_slice(&[base, base + 1, base + 2]);
    out_counts.push(3);
}

// =============================================================================
// 8. UNIT TESTS (PHASE 66 / v0.6.6)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase66_earth_globe_layer_is_valid_ifcx_with_meshes() {
        let earth = IfcxStage::build_earth_globe_layer(VectorEarthTheme::CoastalBlueprint);
        assert_eq!(earth.header.ifcx_version, IFCX_SCHEMA_VERSION);
        assert!(earth.find_node("/earth").is_some());
        assert!(earth.find_node("/earth/globe").is_some());
        let ocean = earth.find_node("/earth/globe/ocean").expect("ocean node");
        assert_eq!(ocean.ifc_class(), Some("IfcGeographicElement"));
        let ocean_mesh = ocean.mesh().expect("ocean usd::usdgeom::mesh");
        assert!(ocean_mesh.points.len() > 2000);
        assert!(ocean_mesh.triangulated_indices().len() >= 4000);

        let land_count = earth
            .data
            .iter()
            .filter(|n| n.path.starts_with("/earth/globe/landmasses/"))
            .count();
        assert_eq!(land_count, WORLD_LANDMASSES.len());

        let report = earth.audit_health();
        assert!(report.conformance_score_pct >= 85.0);
    }

    #[test]
    fn phase66_non_destructive_multilayer_composition_earth_ai_user_ari() {
        // Layer 0: Living Earth Base
        let earth = IfcxStage::build_earth_globe_layer(VectorEarthTheme::CityDaylight);
        // Layer 1: Future AI Real-World Building Capture
        let ai_capture = IfcxDocument::sample_ai_real_building_capture_ifcx();
        // Layer 2: User Continuation Edit (overrides only material on C1 without touching its mesh!)
        let mut user_delta = IfcxDocument::new("user_strengthening_delta");
        user_delta.add_node(
            IfcxNode::new("/earth/sites/bandung_ai_capture/buildings/pavilion_01/columns/C1")
                .with_attr(ns::BSI_PROP, json!({ "material": "High-Strength Concrete fc' 50 MPa" })),
        );
        // Layer 3: Non-Destructive ARI Analysis Overlay
        let mut ari_delta = IfcxDocument::new("ari_v2_overlay");
        ari_delta.add_node(
            IfcxNode::new("/earth/sites/bandung_ai_capture/buildings/pavilion_01")
                .with_attr(ns::EG_ARI, json!({ "ariSystem": 0.91, "maxDcrPct": 58.4 })),
        );

        let stage = IfcxStage::compose(&[earth, ai_capture, user_delta, ari_delta]);
        assert_eq!(stage.layer_ids.len(), 4);
        assert!(stage.get("/earth/globe/ocean").is_some());

        let c1 = stage
            .get("/earth/sites/bandung_ai_capture/buildings/pavilion_01/columns/C1")
            .expect("composed C1");
        // User edit updated material while preserving original name AND usd::usdgeom::mesh!
        assert_eq!(c1.material_label(), "High-Strength Concrete fc' 50 MPa");
        assert_eq!(c1.display_name(), "AI_Col_1");
        assert!(c1.mesh().is_some());

        let bldg = stage
            .get("/earth/sites/bandung_ai_capture/buildings/pavilion_01")
            .expect("composed building");
        assert!(bldg.attributes.contains_key(ns::EG_ARI));
    }

    #[test]
    fn phase66_ai_capture_ifcx_converts_to_editable_bim_and_georeferences() {
        let ai_doc = IfcxDocument::sample_ai_real_building_capture_ifcx();
        let json_str = ai_doc.to_json_pretty();
        assert!(IfcxDocument::looks_like_ifcx(&json_str));

        let (proj, geo_opt, src, addr, _summary) = IfcxDocument::parse_to_bim_project_with_geo(
            &json_str,
            "bandung_pavilion.ifcx",
            "Imports/bandung_pavilion.ifcx",
        )
        .expect("AI capture IFCX should convert cleanly to BimProject");

        assert_eq!(proj.columns.len(), 4);
        assert_eq!(proj.runs.len(), 4);
        assert_eq!(proj.slabs.len(), 1);
        assert_eq!(src, GeoPlacementSource::FileNativeGeo);
        assert_eq!(addr, "Bandung");
        let (lat, lon) = geo_opt.expect("native WGS84 geo");
        assert!((lat - (-6.8915)).abs() < 1e-4);
        assert!((lon - 107.6107).abs() < 1e-4);

        // And converts directly into a solvable 6-DOF StructuralModel!
        let sm = ai_doc
            .to_structural_model("AI Pavilion")
            .expect("IFCX -> StructuralModel");
        assert!(!sm.nodes.is_empty());
        assert!(!sm.elements.is_empty());
    }

    #[test]
    fn phase66_bim_project_ifcx_lossless_roundtrip_and_site_terrain_tin() {
        let villa = IfcxDocument::sample_villa_ifcx();
        let raw = villa.to_json_pretty();
        let (proj, geo_opt, _, _, _) =
            IfcxDocument::parse_to_bim_project_with_geo(&raw, "villa.ifcx", "Projects/villa.ifcx")
                .expect("villa roundtrip");
        assert_eq!(proj.columns.len(), 12);
        assert_eq!(proj.footings.len(), 6);
        assert_eq!(proj.slabs.len(), 2);
        assert!(geo_opt.is_some());

        // Site terrain TIN extraction from IfcSite usd::usdgeom::mesh
        let mut site_doc = IfcxDocument::new("site_tin_test");
        let terrain_mesh = UsdGeomMesh {
            points: vec![
                [0.0, 0.0, 100.0],
                [50.0, 0.0, 105.0],
                [50.0, 50.0, 112.0],
                [0.0, 50.0, 103.0],
            ],
            face_vertex_counts: vec![3, 3],
            face_vertex_indices: vec![0, 1, 2, 0, 2, 3],
            normals: Vec::new(),
            display_color: None,
            vertex_colors: Vec::new(),
            vertex_meta: Vec::new(),
            material_kind: 1.0,
        };
        site_doc.add_node(
            IfcxNode::new("/earth/sites/hillside/terrain")
                .with_class("IfcSite")
                .with_mesh(&terrain_mesh),
        );
        let tin = site_doc
            .extract_site_terrain_tin()
            .expect("IfcSite TIN extraction");
        assert_eq!(tin.points.len(), 4);
        assert_eq!(tin.triangles.len(), 2);
    }

    #[test]
    fn phase66_structural_model_ifcx_roundtrip_and_diff_session() {
        let mut sm = StructuralModel::new("Portal Frame");
        sm.add_node(Node3D::new(1, 0.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        sm.add_node(Node3D::new(2, 0.0, 0.0, 4.0));
        sm.add_node(Node3D::new(3, 6.0, 0.0, 4.0));
        sm.add_node(Node3D::new(4, 6.0, 0.0, 0.0).with_fixity(BoundaryFixity::fixed()));
        sm.add_element(Element3D::new(1, "C1", 1, 2).with_type(ElementType::Frame3D));
        sm.add_element(Element3D::new(2, "B1", 2, 3).with_type(ElementType::Frame3D));
        sm.add_element(Element3D::new(3, "C2", 4, 3).with_type(ElementType::Frame3D));

        let mut doc = IfcxDocument::from_structural_model(&sm, None, false);
        let parsed = IfcxDocument::parse(&doc.to_json_pretty()).expect("parse");
        let restored = parsed.to_structural_model("Restored").expect("restore");
        assert_eq!(restored.nodes.len(), 4);
        assert_eq!(restored.elements.len(), 3);

        let baseline = doc.clone();
        let mut session = IfcxEditSession::new();
        session.push_snapshot(doc.clone());
        doc.add_node(IfcxNode::new("/earth/sites/portal_frame/buildings/portal_frame/extra").with_class("IfcAnnotation"));
        let diff = doc.diff_against(&baseline);
        assert_eq!(diff.added, 1);
        assert!(session.undo(&mut doc));
        assert_eq!(doc.data.len(), baseline.data.len());
    }
}
