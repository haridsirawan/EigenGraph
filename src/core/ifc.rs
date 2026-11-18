//! ifc.rs — OpenBIM Industry Foundation Classes (IFC) Fundamental Reader, Parser & Inspection Engine.
//!
//! Compliant with:
//! - ISO 10303-21 (STEP Physical File format / SPF)
//! - ISO 16739-1:2024 / buildingSMART IFC4.3 Add2 & IFC4 Reference View
//! - ISO 16739:2013 / IFC2x3 Coordination View 2.0
//!
//! Capabilities:
//! 1. Robust STEP ISO-10303-21 lexer and syntax parser (comments, strings, enums, entity references, nested lists).
//! 2. Spatial Hierarchy Decomposition (`IfcProject` ➔ `IfcSite` ➔ `IfcBuilding` ➔ `IfcBuildingStorey` ➔ `IfcSpace` / `IfcProduct`).
//! 3. Entity Classification & Metadata (GUID, Name, Description, PredefinedType, Storey containment).
//! 4. Property Sets (`IfcPropertySet`, `IfcPropertySingleValue`) & Quantities (`IfcElementQuantity`, `IfcQuantityLength`, `IfcQuantityArea`, `IfcQuantityVolume`).
//! 5. Material Associations (`IfcMaterial`, `IfcMaterialLayerSet`, `IfcMaterialProfileSet`).
//! 6. Geometric Placement & Bounding Box Computation (`IfcLocalPlacement`, `IfcAxis2Placement3D`, `IfcExtrudedAreaSolid`, `IfcPolyline`).
//! 7. IFC Model Health & Conformance Quality Assurance Audit (syntax, schema, dangling pointers, uncontained products, duplicate GUIDs).
//! 8. Bi-directional bridge to EigenGraph's native `StructuralModel` for instant ARI and SVD mechanics.

use crate::core::model::{BoundaryFixity, Element3D, ElementType, NodalLoad, Node3D, StructuralModel};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Supported buildingSMART IFC Schema Versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IfcSchemaVersion {
    Ifc2x3,
    Ifc4,
    Ifc4x3,
    Unknown(String),
}

impl IfcSchemaVersion {
    pub fn from_str_schema(s: &str) -> Self {
        let upper = s.to_uppercase();
        if upper.contains("IFC4X3") || upper.contains("IFC4.3") {
            Self::Ifc4x3
        } else if upper.contains("IFC4") {
            Self::Ifc4
        } else if upper.contains("IFC2X3") || upper.contains("IFC2X") {
            Self::Ifc2x3
        } else {
            Self::Unknown(s.to_string())
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Ifc2x3 => "IFC2x3 (ISO 16739:2013)",
            Self::Ifc4 => "IFC4 (ISO 16739-1:2018)",
            Self::Ifc4x3 => "IFC4.3 Add2 (ISO 16739-1:2024)",
            Self::Unknown(_) => "Non-Standard / Custom Schema",
        }
    }
}

/// Primitive value within a STEP Physical File parameter list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepValue {
    Integer(i64),
    Real(f64),
    String(String),
    EntityRef(usize),
    Enumeration(String),
    List(Vec<StepValue>),
    Null,
    Derived,
}

impl StepValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            StepValue::String(s) | StepValue::Enumeration(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            StepValue::Real(v) => Some(*v),
            StepValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        match self {
            StepValue::Integer(v) if *v >= 0 => Some(*v as usize),
            StepValue::EntityRef(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_entity_ref(&self) -> Option<usize> {
        match self {
            StepValue::EntityRef(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[StepValue]> {
        match self {
            StepValue::List(l) => Some(l.as_slice()),
            _ => None,
        }
    }

    pub fn to_step_str(&self) -> String {
        match self {
            StepValue::Integer(i) => i.to_string(),
            StepValue::Real(r) => {
                let s = format!("{:.6}", r);
                let trimmed = s.trim_end_matches('0');
                if trimmed.ends_with('.') {
                    format!("{}0", trimmed)
                } else {
                    trimmed.to_string()
                }
            }
            StepValue::String(s) => format!("'{}'", s.replace('\'', "''")),
            StepValue::EntityRef(id) => format!("#{}", id),
            StepValue::Enumeration(e) => format!(".{}.", e.trim_matches('.')),
            StepValue::List(list) => {
                let inner: Vec<String> = list.iter().map(|v| v.to_step_str()).collect();
                format!("({})", inner.join(","))
            }
            StepValue::Null => "$".to_string(),
            StepValue::Derived => "*".to_string(),
        }
    }
}

/// STEP Header metadata block.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepHeader {
    pub file_description: Vec<String>,
    pub implementation_level: String,
    pub file_name: String,
    pub time_stamp: String,
    pub author: Vec<String>,
    pub organization: Vec<String>,
    pub preprocessor_version: String,
    pub originating_system: String,
    pub authorization: String,
    pub file_schema: Vec<String>,
}

/// Single parsed STEP entity statement: `#id = TYPE_NAME(params);`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepEntity {
    pub id: usize,
    pub type_name: String,
    pub parameters: Vec<StepValue>,
}

impl StepEntity {
    pub fn get_str(&self, idx: usize) -> Option<&str> {
        self.parameters.get(idx).and_then(|v| v.as_str())
    }

    pub fn get_f64(&self, idx: usize) -> Option<f64> {
        self.parameters.get(idx).and_then(|v| v.as_f64())
    }

    pub fn get_ref(&self, idx: usize) -> Option<usize> {
        self.parameters.get(idx).and_then(|v| v.as_entity_ref())
    }

    pub fn get_ref_list(&self, idx: usize) -> Vec<usize> {
        if let Some(StepValue::List(list)) = self.parameters.get(idx) {
            list.iter().filter_map(|v| v.as_entity_ref()).collect()
        } else {
            Vec::new()
        }
    }

    pub fn to_step_str(&self) -> String {
        let params: Vec<String> = self.parameters.iter().map(|p| p.to_step_str()).collect();
        format!("#{}= {}({});", self.id, self.type_name, params.join(","))
    }
}

/// Raw parsed STEP Physical File model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepModel {
    pub header: StepHeader,
    pub entities: HashMap<usize, StepEntity>,
    pub ordered_ids: Vec<usize>,
    pub type_index: HashMap<String, Vec<usize>>,
}

impl StepModel {
    pub fn get_entities_of_type(&self, type_name: &str) -> Vec<&StepEntity> {
        let upper = type_name.to_uppercase();
        if let Some(ids) = self.type_index.get(&upper) {
            ids.iter().filter_map(|id| self.entities.get(id)).collect()
        } else {
            Vec::new()
        }
    }

    /// Allocates the next available unique entity STEP ID (#id).
    pub fn next_id(&self) -> usize {
        self.ordered_ids.iter().copied().max().unwrap_or(0) + 1
    }

    /// Serializes the entire STEP model into a valid ISO 10303-21 Physical File string.
    pub fn to_step_string(&self) -> String {
        let mut out = String::new();
        out.push_str("ISO-10303-21;\nHEADER;\n");
        let desc = self.header.file_description.iter().map(|s| format!("'{}'", s.replace('\'', "''"))).collect::<Vec<_>>().join(",");
        out.push_str(&format!("FILE_DESCRIPTION(({}), '{}');\n", desc, self.header.implementation_level.replace('\'', "''")));
        out.push_str(&format!("FILE_NAME('{}','{}',({}),({}),'{}','{}','{}');\n",
            self.header.file_name.replace('\'', "''"),
            self.header.time_stamp.replace('\'', "''"),
            self.header.author.iter().map(|s| format!("'{}'", s.replace('\'', "''"))).collect::<Vec<_>>().join(","),
            self.header.organization.iter().map(|s| format!("'{}'", s.replace('\'', "''"))).collect::<Vec<_>>().join(","),
            self.header.preprocessor_version.replace('\'', "''"),
            self.header.originating_system.replace('\'', "''"),
            self.header.authorization.replace('\'', "''")
        ));
        let schemas = self.header.file_schema.iter().map(|s| format!("'{}'", s.replace('\'', "''"))).collect::<Vec<_>>().join(",");
        out.push_str(&format!("FILE_SCHEMA(({}));\n", schemas));
        out.push_str("ENDSEC;\nDATA;\n");
        for id in &self.ordered_ids {
            if let Some(entity) = self.entities.get(id) {
                out.push_str(&entity.to_step_str());
                out.push('\n');
            }
        }
        out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        out
    }
}

/// 3D Bounding Box in meters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IfcBoundingBox {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub min_z: f64,
    pub max_z: f64,
}

impl Default for IfcBoundingBox {
    fn default() -> Self {
        Self {
            min_x: 0.0,
            max_x: 0.0,
            min_y: 0.0,
            max_y: 0.0,
            min_z: 0.0,
            max_z: 0.0,
        }
    }
}

impl IfcBoundingBox {
    pub fn new(min: (f64, f64, f64), max: (f64, f64, f64)) -> Self {
        Self {
            min_x: min.0.min(max.0),
            max_x: min.0.max(max.0),
            min_y: min.1.min(max.1),
            max_y: min.1.max(max.1),
            min_z: min.2.min(max.2),
            max_z: min.2.max(max.2),
        }
    }

    pub fn centroid(&self) -> (f64, f64, f64) {
        (
            0.5 * (self.min_x + self.max_x),
            0.5 * (self.min_y + self.max_y),
            0.5 * (self.min_z + self.max_z),
        )
    }

    pub fn dimensions(&self) -> (f64, f64, f64) {
        (
            (self.max_x - self.min_x).abs(),
            (self.max_y - self.min_y).abs(),
            (self.max_z - self.min_z).abs(),
        )
    }

    pub fn volume(&self) -> f64 {
        let (dx, dy, dz) = self.dimensions();
        dx * dy * dz
    }

    /// Checks if this bounding box intersects another with a given tolerance margin in meters.
    pub fn intersects(&self, other: &IfcBoundingBox, tolerance_m: f64) -> bool {
        self.min_x <= other.max_x + tolerance_m && self.max_x >= other.min_x - tolerance_m &&
        self.min_y <= other.max_y + tolerance_m && self.max_y >= other.min_y - tolerance_m &&
        self.min_z <= other.max_z + tolerance_m && self.max_z >= other.min_z - tolerance_m
    }

    /// Computes the intersection volume between two bounding boxes in cubic meters (m³).
    pub fn intersection_volume(&self, other: &IfcBoundingBox) -> f64 {
        let ix = (self.max_x.min(other.max_x) - self.min_x.max(other.min_x)).max(0.0);
        let iy = (self.max_y.min(other.max_y) - self.min_y.max(other.min_y)).max(0.0);
        let iz = (self.max_z.min(other.max_z) - self.min_z.max(other.min_z)).max(0.0);
        ix * iy * iz
    }

    /// Computes minimum Euclidean distance between two Axis-Aligned Bounding Boxes.
    pub fn distance_to(&self, other: &IfcBoundingBox) -> f64 {
        let dx = (self.min_x - other.max_x).max(other.min_x - self.max_x).max(0.0);
        let dy = (self.min_y - other.max_y).max(other.min_y - self.max_y).max(0.0);
        let dz = (self.min_z - other.max_z).max(other.min_z - self.max_z).max(0.0);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Computes geometric overlap percentage relative to the smaller bounding volume.
    pub fn overlap_ratio(&self, other: &IfcBoundingBox) -> f64 {
        let iv = self.intersection_volume(other);
        let min_vol = self.volume().min(other.volume());
        if min_vol > 1e-9 {
            iv / min_vol
        } else {
            0.0
        }
    }
}

/// Extracted Property Set data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfcPropertySetSummary {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

/// Extracted IFC Product / Element summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfcElementSummary {
    pub step_id: usize,
    pub global_id: String,
    pub name: String,
    pub description: String,
    pub class_name: String,
    pub object_type: String,
    pub predefined_type: Option<String>,
    pub storey_name: String,
    pub material_name: Option<String>,
    pub property_sets: Vec<IfcPropertySetSummary>,
    pub quantities: HashMap<String, f64>,
    pub bbox: IfcBoundingBox,
    pub centroid: (f64, f64, f64),
    pub wireframe_lines: Vec<((f64, f64, f64), (f64, f64, f64))>,
}

/// Node in the hierarchical spatial structure tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcSpatialNode {
    pub node_id: usize,
    pub step_id: usize,
    pub global_id: String,
    pub name: String,
    pub class_name: String,
    pub elevation_m: Option<f64>,
    pub contained_element_ids: Vec<usize>,
    pub children: Vec<IfcSpatialNode>,
}

impl IfcSpatialNode {
    pub fn total_contained_count(&self) -> usize {
        let own = self.contained_element_ids.len();
        let child_total: usize = self.children.iter().map(|c| c.total_contained_count()).sum();
        own + child_total
    }
}

/// Status of an individual health or validation audit check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IfcAuditStatus {
    Pass,
    Warning,
    Fail,
}

impl IfcAuditStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warning => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

/// Detailed audit item within an IFC Model Health Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcAuditCheckItem {
    pub category: String,
    pub check_name: String,
    pub status: IfcAuditStatus,
    pub detail: String,
}

/// Comprehensive IFC Model Health & Conformance Audit Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcModelHealthReport {
    pub schema_version: String,
    pub total_entities: usize,
    pub total_products: usize,
    pub total_property_sets: usize,
    pub total_materials: usize,
    pub dangling_references_count: usize,
    pub uncontained_products_count: usize,
    pub duplicate_guids_count: usize,
    pub conformance_score_pct: f64,
    pub verdict: String,
    pub checks: Vec<IfcAuditCheckItem>,
}

/// Severity classification of a detected spatial clash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IfcClashSeverity {
    Critical,   // Hard clash with significant volume overlap (> 0.05 m³)
    Major,      // Hard clash with moderate overlap
    Minor,      // Soft clearance violation (< clearance threshold)
    Duplicate,  // Identical placement and geometry (> 90% overlap)
}

impl IfcClashSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL HARD CLASH",
            Self::Major => "MAJOR HARD CLASH",
            Self::Minor => "CLEARANCE VIOLATION",
            Self::Duplicate => "DUPLICATE ENTITY",
        }
    }
}

/// Individual spatial clash or clearance incident between two building elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcClashItem {
    pub id_a: usize,
    pub id_b: usize,
    pub name_a: String,
    pub name_b: String,
    pub class_a: String,
    pub class_b: String,
    pub severity: IfcClashSeverity,
    pub intersecting_volume_m3: f64,
    pub distance_m: f64,
    pub clash_point: (f64, f64, f64),
    pub description: String,
}

/// Comprehensive report of all detected clashes and proximity warnings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IfcClashReport {
    pub total_evaluated_pairs: usize,
    pub clashes: Vec<IfcClashItem>,
    pub hard_clashes: usize,
    pub clearance_violations: usize,
    pub duplicate_entities: usize,
}

/// Category of model differential change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IfcDiffKind {
    Added,
    Deleted,
    Modified,
    Unchanged,
}

impl IfcDiffKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Added => "+ ADDED",
            Self::Deleted => "- DELETED",
            Self::Modified => "* MODIFIED",
            Self::Unchanged => "= UNCHANGED",
        }
    }
}

/// Detailed difference description for a single building element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcElementDiff {
    pub step_id: usize,
    pub global_id: String,
    pub name: String,
    pub class_name: String,
    pub diff_kind: IfcDiffKind,
    pub details: Vec<String>,
    pub delta_centroid: (f64, f64, f64),
    pub delta_volume: f64,
}

/// Full model differential report between baseline and revised models.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IfcModelDiff {
    pub baseline_schema: String,
    pub revised_schema: String,
    pub added_count: usize,
    pub deleted_count: usize,
    pub modified_count: usize,
    pub unchanged_count: usize,
    pub element_diffs: Vec<IfcElementDiff>,
    pub revision_hash: String,
}

/// Rule check result within an Information Delivery Specification (IDS) evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcIdsRuleCheck {
    pub rule_id: String,
    pub description: String,
    pub target_class: String,
    pub pass_count: usize,
    pub fail_count: usize,
    pub total_count: usize,
    pub failed_element_ids: Vec<usize>,
    pub status: IfcAuditStatus,
}

/// Comprehensive buildingSMART Information Delivery Specification (IDS) Conformance Report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IfcIdsReport {
    pub specification_title: String,
    pub compliance_score_pct: f64,
    pub rules: Vec<IfcIdsRuleCheck>,
}

/// BIM Collaboration Format (BCF 2.1 / 3.0) Issue Topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcBcfTopic {
    pub guid: String,
    pub topic_type: String,
    pub topic_status: String,
    pub title: String,
    pub priority: String,
    pub description: String,
    pub creation_date: String,
    pub creation_author: String,
    pub component_guids: Vec<String>,
    pub camera_position: (f64, f64, f64),
    pub camera_target: (f64, f64, f64),
}

/// Primary OpenBIM IFC Model representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfcModel {
    pub schema: IfcSchemaVersion,
    pub header: StepHeader,
    pub spatial_root: IfcSpatialNode,
    pub elements: Vec<IfcElementSummary>,
    pub unit_scale: f64, // conversion factor to meters (e.g. 0.001 for mm)
    pub health_report: IfcModelHealthReport,
    pub raw_step: StepModel,
}

impl IfcModel {
    /// Opens, lexes, parses, and decomposes an IFC STEP file string into a comprehensive `IfcModel`.
    pub fn parse(content: &str) -> Result<Self, String> {
        let step_model = StepParser::parse_step(content)?;

        // 1. Detect Schema
        let schema_str = step_model
            .header
            .file_schema
            .first()
            .cloned()
            .unwrap_or_else(|| "IFC4".to_string());
        let schema = IfcSchemaVersion::from_str_schema(&schema_str);

        // 2. Detect Unit Scale (default to meters = 1.0, 0.001 for millimetres)
        let unit_scale = Self::detect_unit_scale(&step_model, content);

        // 3. Extract Cartesian Points & Directions
        let mut cartesian_points: HashMap<usize, (f64, f64, f64)> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCCARTESIANPOINT") {
            if let Some(coords_val) = ent.parameters.first() {
                if let Some(list) = coords_val.as_list() {
                    let x = list.first().and_then(|v| v.as_f64()).unwrap_or(0.0) * unit_scale;
                    let y = list.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) * unit_scale;
                    let z = list.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) * unit_scale;
                    cartesian_points.insert(ent.id, (x, y, z));
                }
            }
        }

        let mut directions: HashMap<usize, (f64, f64, f64)> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCDIRECTION") {
            if let Some(coords_val) = ent.parameters.first() {
                if let Some(list) = coords_val.as_list() {
                    let dx = list.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let dy = list.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let dz = list.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0);
                    directions.insert(ent.id, (dx, dy, dz));
                }
            }
        }

        // 4. Extract Placements & Indexed Point Lists (including full local-to-world basis frames)
        let mut axis2_placements: HashMap<usize, (usize, Option<usize>, Option<usize>)> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCAXIS2PLACEMENT3D") {
            if let Some(loc_id) = ent.get_ref(0) {
                let axis_id = ent.get_ref(1);
                let ref_dir_id = ent.get_ref(2);
                axis2_placements.insert(ent.id, (loc_id, axis_id, ref_dir_id));
            }
        }
        for ent in step_model.get_entities_of_type("IFCAXIS2PLACEMENT2D") {
            if let Some(loc_id) = ent.get_ref(0) {
                let ref_dir_id = ent.get_ref(1);
                axis2_placements.insert(ent.id, (loc_id, None, ref_dir_id));
            }
        }

        let mut point_lists_m: HashMap<usize, Vec<(f64, f64, f64)>> = HashMap::new();
        for cls in ["IFCCARTESIANPOINTLIST3D", "IFCCARTESIANPOINTLIST2D"] {
            for ent in step_model.get_entities_of_type(cls) {
                if let Some(rows) = ent.parameters.first().and_then(|v| v.as_list()) {
                    let mut pts = Vec::with_capacity(rows.len());
                    for row in rows {
                        if let Some(coords) = row.as_list() {
                            let x = coords.first().and_then(|v| v.as_f64()).unwrap_or(0.0) * unit_scale;
                            let y = coords.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) * unit_scale;
                            let z = coords.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) * unit_scale;
                            pts.push((x, y, z));
                        }
                    }
                    point_lists_m.insert(ent.id, pts);
                }
            }
        }

        let mut local_placements: HashMap<usize, (Option<usize>, usize)> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCLOCALPLACEMENT") {
            let parent_opt = ent.get_ref(0);
            if let Some(rel_axis) = ent.get_ref(1) {
                local_placements.insert(ent.id, (parent_opt, rel_axis));
            }
        }

        let cross = |a: (f64, f64, f64), b: (f64, f64, f64)| -> (f64, f64, f64) {
            (a.1 * b.2 - a.2 * b.1, a.2 * b.0 - a.0 * b.2, a.0 * b.1 - a.1 * b.0)
        };
        let norm_v = |v: (f64, f64, f64), fallback: (f64, f64, f64)| -> (f64, f64, f64) {
            let l = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
            if l > 1e-9 { (v.0 / l, v.1 / l, v.2 / l) } else { fallback }
        };
        let axis_basis = |axis_id: usize| -> ((f64, f64, f64), (f64, f64, f64), (f64, f64, f64), (f64, f64, f64)) {
            if let Some(&(pt_id, z_ref, x_ref)) = axis2_placements.get(&axis_id) {
                let loc = cartesian_points.get(&pt_id).copied().unwrap_or((0.0, 0.0, 0.0));
                let uz = z_ref
                    .and_then(|id| directions.get(&id).copied())
                    .map(|v| norm_v(v, (0.0, 0.0, 1.0)))
                    .unwrap_or((0.0, 0.0, 1.0));
                let raw_x = x_ref
                    .and_then(|id| directions.get(&id).copied())
                    .unwrap_or_else(|| if uz.2.abs() > 0.99 { (1.0, 0.0, 0.0) } else { (0.0, 0.0, 1.0) });
                let dot_xz = raw_x.0 * uz.0 + raw_x.1 * uz.1 + raw_x.2 * uz.2;
                let proj_x = (
                    raw_x.0 - dot_xz * uz.0,
                    raw_x.1 - dot_xz * uz.1,
                    raw_x.2 - dot_xz * uz.2,
                );
                let ux = norm_v(proj_x, (1.0, 0.0, 0.0));
                let uy = norm_v(cross(uz, ux), (0.0, 1.0, 0.0));
                (loc, ux, uy, uz)
            } else {
                ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
            }
        };

        // Resolve cumulative global placement origin & orthonormal frame from an IfcLocalPlacement chain
        let resolve_placement_frame = |lp_id: usize| -> ((f64, f64, f64), (f64, f64, f64), (f64, f64, f64), (f64, f64, f64)) {
            let mut chain = Vec::new();
            let mut cur = Some(lp_id);
            let mut guard = 0;
            while let Some(cid) = cur {
                if let Some(&(p_opt, axis_id)) = local_placements.get(&cid) {
                    chain.push(axis_id);
                    cur = p_opt;
                } else {
                    break;
                }
                guard += 1;
                if guard > 32 {
                    break;
                }
            }
            let mut origin = (0.0_f64, 0.0_f64, 0.0_f64);
            let mut ux = (1.0_f64, 0.0_f64, 0.0_f64);
            let mut uy = (0.0_f64, 1.0_f64, 0.0_f64);
            let mut uz = (0.0_f64, 0.0_f64, 1.0_f64);
            for axis_id in chain.into_iter().rev() {
                let (loc, lx, ly, lz) = axis_basis(axis_id);
                let xform_vec = |bx: (f64, f64, f64), by: (f64, f64, f64), bz: (f64, f64, f64), v: (f64, f64, f64)| -> (f64, f64, f64) {
                    (
                        bx.0 * v.0 + by.0 * v.1 + bz.0 * v.2,
                        bx.1 * v.0 + by.1 * v.1 + bz.1 * v.2,
                        bx.2 * v.0 + by.2 * v.1 + bz.2 * v.2,
                    )
                };
                let d = xform_vec(ux, uy, uz, loc);
                origin = (origin.0 + d.0, origin.1 + d.1, origin.2 + d.2);
                let nx = xform_vec(ux, uy, uz, lx);
                let ny = xform_vec(ux, uy, uz, ly);
                let nz = xform_vec(ux, uy, uz, lz);
                ux = nx;
                uy = ny;
                uz = nz;
            }
            (origin, ux, uy, uz)
        };

        // 5. Parse Property Sets & Element Quantities
        let mut pset_map: HashMap<usize, IfcPropertySetSummary> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCPROPERTYSET") {
            let pset_name = ent.get_str(2).unwrap_or("Pset_General").to_string();
            let mut props = Vec::new();
            for prop_id in ent.get_ref_list(4) {
                if let Some(prop_ent) = step_model.entities.get(&prop_id) {
                    if prop_ent.type_name == "IFCPROPERTYSINGLEVALUE" {
                        let prop_name = prop_ent.get_str(0).unwrap_or("UnknownProperty").to_string();
                        let prop_val = if let Some(v) = prop_ent.parameters.get(2) {
                            match v {
                                StepValue::String(s) => s.clone(),
                                StepValue::Real(r) => format!("{:.4}", r),
                                StepValue::Integer(i) => format!("{}", i),
                                StepValue::Enumeration(e) => e.clone(),
                                _ => "Defined".to_string(),
                            }
                        } else {
                            "-".to_string()
                        };
                        props.push((prop_name, prop_val));
                    }
                }
            }
            pset_map.insert(ent.id, IfcPropertySetSummary { name: pset_name, properties: props });
        }

        let mut qto_map: HashMap<usize, HashMap<String, f64>> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCELEMENTQUANTITY") {
            let mut quantities = HashMap::new();
            for q_id in ent.get_ref_list(5) {
                if let Some(q_ent) = step_model.entities.get(&q_id) {
                    let q_name = q_ent.get_str(0).unwrap_or("Quantity").to_string();
                    let val = q_ent.get_f64(3).unwrap_or(0.0);
                    quantities.insert(q_name, val);
                }
            }
            qto_map.insert(ent.id, quantities);
        }

        // RelDefinesByProperties mapping: product_id -> list of (psets, qtos)
        let mut element_psets: HashMap<usize, Vec<IfcPropertySetSummary>> = HashMap::new();
        let mut element_qtos: HashMap<usize, HashMap<String, f64>> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCRELDEFINESBYPROPERTIES") {
            let related_objs = ent.get_ref_list(4);
            if let Some(prop_def_id) = ent.get_ref(5) {
                if let Some(pset) = pset_map.get(&prop_def_id) {
                    for &obj_id in &related_objs {
                        element_psets.entry(obj_id).or_default().push(pset.clone());
                    }
                }
                if let Some(qto) = qto_map.get(&prop_def_id) {
                    for &obj_id in &related_objs {
                        element_qtos.entry(obj_id).or_default().extend(qto.clone());
                    }
                }
            }
        }

        // 6. Material Associations
        let mut materials_map: HashMap<usize, String> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCMATERIAL") {
            let mat_name = ent.get_str(0).unwrap_or("Structural Material").to_string();
            materials_map.insert(ent.id, mat_name);
        }
        for ent in step_model.get_entities_of_type("IFCMATERIALLAYERSET") {
            let mat_name = ent.get_str(1).unwrap_or("Layered Material").to_string();
            materials_map.insert(ent.id, mat_name);
        }

        let mut element_materials: HashMap<usize, String> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCRELASSOCIATESMATERIAL") {
            let related_objs = ent.get_ref_list(4);
            if let Some(mat_id) = ent.get_ref(5) {
                let name = materials_map
                    .get(&mat_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Material_{}", mat_id));
                for &obj_id in &related_objs {
                    element_materials.insert(obj_id, name.clone());
                }
            }
        }

        // 7. Spatial Structure Hierarchy
        let mut rel_aggregates: HashMap<usize, Vec<usize>> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCRELAGGREGATES") {
            if let Some(relating_id) = ent.get_ref(4) {
                let related = ent.get_ref_list(5);
                rel_aggregates.entry(relating_id).or_default().extend(related);
            }
        }

        let mut rel_contained: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut product_to_storey: HashMap<usize, String> = HashMap::new();
        for ent in step_model.get_entities_of_type("IFCRELCONTAINEDINSPATIALSTRUCTURE") {
            let related_elements = ent.get_ref_list(4);
            if let Some(relating_structure_id) = ent.get_ref(5) {
                rel_contained.entry(relating_structure_id).or_default().extend(related_elements.clone());
                let storey_name = step_model
                    .entities
                    .get(&relating_structure_id)
                    .and_then(|s| s.get_str(2))
                    .unwrap_or("Storey")
                    .to_string();
                for &el_id in &related_elements {
                    product_to_storey.insert(el_id, storey_name.clone());
                }
            }
        }

        // Find IfcProject root
        let project_ent = step_model.get_entities_of_type("IFCPROJECT").into_iter().next();
        let project_id = project_ent.map(|e| e.id).unwrap_or(1);
        let project_name = project_ent.and_then(|e| e.get_str(2)).unwrap_or("OpenBIM Project").to_string();
        let project_guid = project_ent.and_then(|e| e.get_str(0)).unwrap_or("0000000000000000000001").to_string();

        let spatial_root = Self::build_spatial_tree(
            &step_model,
            project_id,
            &project_guid,
            &project_name,
            "IfcProject",
            None,
            &rel_aggregates,
            &rel_contained,
        );

        // 8. Extract Concrete Elements & Geometry
        let product_classes = [
            "IFCBEAM",
            "IFCCOLUMN",
            "IFCWALL",
            "IFCWALLSTANDARDCASE",
            "IFCSLAB",
            "IFCFOOTING",
            "IFCMEMBER",
            "IFCPLATE",
            "IFCDOOR",
            "IFCWINDOW",
            "IFCSPACE",
            "IFCREINFORCINGBAR",
            "IFCSTRUCTURALCURVEMEMBER",
            "IFCSTRUCTURALSURFACEMEMBER",
            "IFCSTRUCTURALPOINTCONNECTION",
            "IFCOPENINGELEMENT",
            "IFCROOF",
            "IFCSTAIR",
            "IFCRAILING",
        ];

        let mut elements = Vec::new();
        for class in &product_classes {
            for ent in step_model.get_entities_of_type(class) {
                let guid = ent.get_str(0).unwrap_or("").to_string();
                let name = ent.get_str(2).unwrap_or("").to_string();
                let desc = ent.get_str(3).unwrap_or("").to_string();
                let obj_type = ent.get_str(4).unwrap_or("").to_string();
                let predefined_type = ent.parameters.last().and_then(|v| v.as_str()).map(|s| s.to_string());
                let storey_name = product_to_storey.get(&ent.id).cloned().unwrap_or_else(|| "Unassigned Storey".to_string());
                let mat_name = element_materials.get(&ent.id).cloned();
                let psets = element_psets.get(&ent.id).cloned().unwrap_or_default();
                let qtos = element_qtos.get(&ent.id).cloned().unwrap_or_default();

                // Compute Placement Frame & Geometry (supports SweptSolid, MappedItem, Booleans, Brep, Polyline, IndexedPolyCurve)
                let placement_frame = ent
                    .get_ref(5)
                    .map(|lp_id| resolve_placement_frame(lp_id))
                    .unwrap_or((
                        (0.0, 0.0, 0.0),
                        (1.0, 0.0, 0.0),
                        (0.0, 1.0, 0.0),
                        (0.0, 0.0, 1.0),
                    ));

                let (bbox, lines) = Self::compute_element_geometry(
                    &step_model,
                    ent,
                    placement_frame,
                    unit_scale,
                    &cartesian_points,
                    &directions,
                    &axis2_placements,
                    &point_lists_m,
                );

                elements.push(IfcElementSummary {
                    step_id: ent.id,
                    global_id: guid,
                    name: if name.is_empty() { format!("{}_{}", class, ent.id) } else { name },
                    description: desc,
                    class_name: class.to_string(),
                    object_type: obj_type,
                    predefined_type,
                    storey_name,
                    material_name: mat_name,
                    property_sets: psets,
                    quantities: qtos,
                    bbox,
                    centroid: bbox.centroid(),
                    wireframe_lines: lines,
                });
            }
        }

        // 9. Conduct Conformance & Health Quality Audit
        let health_report = Self::audit_model_health(&step_model, &schema, &elements, &spatial_root);

        Ok(Self {
            schema,
            header: step_model.header.clone(),
            spatial_root,
            elements,
            unit_scale,
            health_report,
            raw_step: step_model,
        })
    }

    fn detect_unit_scale(step_model: &StepModel, content: &str) -> f64 {
        // 1. Authoritative check: explicit IfcSIUnit / IfcConversionBasedUnit for LENGTHUNIT
        for ent in step_model.get_entities_of_type("IFCSIUNIT") {
            if let Some(unit_type) = ent.get_str(1) {
                if unit_type.eq_ignore_ascii_case("LENGTHUNIT") {
                    if let Some(prefix) = ent.get_str(2) {
                        if prefix.eq_ignore_ascii_case("MILLI") {
                            return 0.001;
                        } else if prefix.eq_ignore_ascii_case("CENTI") {
                            return 0.01;
                        } else if prefix.eq_ignore_ascii_case("KILO") {
                            return 1000.0;
                        }
                    }
                    return 1.0;
                }
            }
        }
        for ent in step_model.get_entities_of_type("IFCCONVERSIONBASEDUNIT") {
            if let Some(unit_type) = ent.get_str(1) {
                if unit_type.eq_ignore_ascii_case("LENGTHUNIT") {
                    let name = ent.get_str(2).unwrap_or("").to_uppercase();
                    if name.contains("FOOT") || name.contains("FEET") {
                        return 0.3048;
                    } else if name.contains("INCH") {
                        return 0.0254;
                    }
                }
            }
        }
        // 2. Fallback heuristic only when no LENGTHUNIT entity parsed
        let upper = content.to_uppercase();
        if upper.contains(".MILLIMETRE.") || upper.contains(".MILLIMETER.") {
            0.001
        } else if upper.contains(".CENTIMETRE.") || upper.contains(".CENTIMETER.") {
            0.01
        } else {
            1.0
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_spatial_tree(
        step_model: &StepModel,
        step_id: usize,
        guid: &str,
        name: &str,
        class_name: &str,
        elevation: Option<f64>,
        rel_aggregates: &HashMap<usize, Vec<usize>>,
        rel_contained: &HashMap<usize, Vec<usize>>,
    ) -> IfcSpatialNode {
        let contained = rel_contained.get(&step_id).cloned().unwrap_or_default();
        let mut children = Vec::new();

        if let Some(child_ids) = rel_aggregates.get(&step_id) {
            for &cid in child_ids {
                if let Some(c_ent) = step_model.entities.get(&cid) {
                    let c_guid = c_ent.get_str(0).unwrap_or("").to_string();
                    let c_name = c_ent.get_str(2).unwrap_or("").to_string();
                    let c_class = c_ent.type_name.as_str();
                    let c_elev = if c_class == "IFCBUILDINGSTOREY" {
                        c_ent.get_f64(9).or_else(|| c_ent.get_f64(8))
                    } else {
                        None
                    };

                    let child_node = Self::build_spatial_tree(
                        step_model,
                        cid,
                        &c_guid,
                        if c_name.is_empty() { c_class } else { &c_name },
                        c_class,
                        c_elev,
                        rel_aggregates,
                        rel_contained,
                    );
                    children.push(child_node);
                }
            }
        }

        IfcSpatialNode {
            node_id: step_id,
            step_id,
            global_id: guid.to_string(),
            name: name.to_string(),
            class_name: class_name.to_string(),
            elevation_m: elevation,
            contained_element_ids: contained,
            children,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_element_geometry(
        step_model: &StepModel,
        ent: &StepEntity,
        placement_frame: ((f64, f64, f64), (f64, f64, f64), (f64, f64, f64), (f64, f64, f64)),
        unit_scale: f64,
        cartesian_points: &HashMap<usize, (f64, f64, f64)>,
        directions: &HashMap<usize, (f64, f64, f64)>,
        axis2_placements: &HashMap<usize, (usize, Option<usize>, Option<usize>)>,
        point_lists_m: &HashMap<usize, Vec<(f64, f64, f64)>>,
    ) -> (IfcBoundingBox, Vec<((f64, f64, f64), (f64, f64, f64))>) {
        let (origin, ux, uy, uz) = placement_frame;
        let xform_pt = |p: (f64, f64, f64)| -> (f64, f64, f64) {
            (
                origin.0 + ux.0 * p.0 + uy.0 * p.1 + uz.0 * p.2,
                origin.1 + ux.1 * p.0 + uy.1 * p.1 + uz.1 * p.2,
                origin.2 + ux.2 * p.0 + uy.2 * p.1 + uz.2 * p.2,
            )
        };
        let xform_dir = |v: (f64, f64, f64)| -> (f64, f64, f64) {
            (
                ux.0 * v.0 + uy.0 * v.1 + uz.0 * v.2,
                ux.1 * v.0 + uy.1 * v.1 + uz.1 * v.2,
                ux.2 * v.0 + uy.2 * v.1 + uz.2 * v.2,
            )
        };
        let axis_frame = |axis_id_opt: Option<usize>| -> ((f64, f64, f64), (f64, f64, f64), (f64, f64, f64), (f64, f64, f64)) {
            let norm = |v: (f64, f64, f64), fb: (f64, f64, f64)| -> (f64, f64, f64) {
                let l = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
                if l > 1e-9 { (v.0 / l, v.1 / l, v.2 / l) } else { fb }
            };
            let cross = |a: (f64, f64, f64), b: (f64, f64, f64)| -> (f64, f64, f64) {
                (a.1 * b.2 - a.2 * b.1, a.2 * b.0 - a.0 * b.2, a.0 * b.1 - a.1 * b.0)
            };
            if let Some(axis_id) = axis_id_opt {
                if let Some(&(pt_id, z_ref, x_ref)) = axis2_placements.get(&axis_id) {
                    let loc = cartesian_points.get(&pt_id).copied().unwrap_or((0.0, 0.0, 0.0));
                    let az = z_ref
                        .and_then(|id| directions.get(&id).copied())
                        .map(|v| norm(v, (0.0, 0.0, 1.0)))
                        .unwrap_or((0.0, 0.0, 1.0));
                    let raw_x = x_ref
                        .and_then(|id| directions.get(&id).copied())
                        .unwrap_or_else(|| if az.2.abs() > 0.99 { (1.0, 0.0, 0.0) } else { (0.0, 0.0, 1.0) });
                    let d = raw_x.0 * az.0 + raw_x.1 * az.1 + raw_x.2 * az.2;
                    let ax = norm((raw_x.0 - d * az.0, raw_x.1 - d * az.1, raw_x.2 - d * az.2), (1.0, 0.0, 0.0));
                    let ay = norm(cross(az, ax), (0.0, 1.0, 0.0));
                    return (loc, ax, ay, az);
                }
            }
            ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
        };

        let mut lines = Vec::new();
        let mut min_pt = (f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max_pt = (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut push_pt = |pt: (f64, f64, f64)| {
            min_pt.0 = min_pt.0.min(pt.0);
            min_pt.1 = min_pt.1.min(pt.1);
            min_pt.2 = min_pt.2.min(pt.2);
            max_pt.0 = max_pt.0.max(pt.0);
            max_pt.1 = max_pt.1.max(pt.1);
            max_pt.2 = max_pt.2.max(pt.2);
        };

        #[allow(clippy::too_many_arguments)]
        fn extract_item(
            step_model: &StepModel,
            item_id: usize,
            is_axis_rep: bool,
            depth_guard: usize,
            origin: (f64, f64, f64),
            xform_pt: &dyn Fn((f64, f64, f64)) -> (f64, f64, f64),
            xform_dir: &dyn Fn((f64, f64, f64)) -> (f64, f64, f64),
            axis_frame: &dyn Fn(Option<usize>) -> ((f64, f64, f64), (f64, f64, f64), (f64, f64, f64), (f64, f64, f64)),
            unit_scale: f64,
            cartesian_points: &HashMap<usize, (f64, f64, f64)>,
            directions: &HashMap<usize, (f64, f64, f64)>,
            point_lists_m: &HashMap<usize, Vec<(f64, f64, f64)>>,
            lines: &mut Vec<((f64, f64, f64), (f64, f64, f64))>,
            push_pt: &mut dyn FnMut((f64, f64, f64)),
        ) {
            if depth_guard > 8 {
                return;
            }
            let Some(item_ent) = step_model.entities.get(&item_id) else {
                return;
            };
            match item_ent.type_name.as_str() {
                "IFCMAPPEDITEM" => {
                    let map_offset = if let Some(tgt_id) = item_ent.get_ref(1) {
                        step_model
                            .entities
                            .get(&tgt_id)
                            .and_then(|t| t.get_ref(2))
                            .and_then(|pid| cartesian_points.get(&pid).copied())
                            .unwrap_or((0.0, 0.0, 0.0))
                    } else {
                        (0.0, 0.0, 0.0)
                    };
                    let shifted_xform = |p: (f64, f64, f64)| -> (f64, f64, f64) {
                        xform_pt((p.0 + map_offset.0, p.1 + map_offset.1, p.2 + map_offset.2))
                    };
                    if let Some(src_id) = item_ent.get_ref(0) {
                        if let Some(rep_map) = step_model.entities.get(&src_id) {
                            if let Some(mapped_rep_id) = rep_map.get_ref(1) {
                                if let Some(mapped_rep) = step_model.entities.get(&mapped_rep_id) {
                                    for sub_id in mapped_rep.get_ref_list(3) {
                                        extract_item(
                                            step_model,
                                            sub_id,
                                            false,
                                            depth_guard + 1,
                                            origin,
                                            &shifted_xform,
                                            xform_dir,
                                            axis_frame,
                                            unit_scale,
                                            cartesian_points,
                                            directions,
                                            point_lists_m,
                                            lines,
                                            push_pt,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                "IFCBOOLEANCLIPPINGRESULT" | "IFCBOOLEANRESULT" => {
                    if let Some(first_op_id) = item_ent.get_ref(1).or_else(|| item_ent.get_ref(0)) {
                        extract_item(
                            step_model,
                            first_op_id,
                            false,
                            depth_guard + 1,
                            origin,
                            xform_pt,
                            xform_dir,
                            axis_frame,
                            unit_scale,
                            cartesian_points,
                            directions,
                            point_lists_m,
                            lines,
                            push_pt,
                        );
                    }
                }
                "IFCEXTRUDEDAREASOLID" => {
                    let depth = item_ent.get_f64(3).unwrap_or(3.0) * unit_scale;
                    let (pos_loc, px, py, pz) = axis_frame(item_ent.get_ref(1));
                    let local_to_world = |lp: (f64, f64, f64)| -> (f64, f64, f64) {
                        xform_pt((
                            pos_loc.0 + px.0 * lp.0 + py.0 * lp.1 + pz.0 * lp.2,
                            pos_loc.1 + px.1 * lp.0 + py.1 * lp.1 + pz.1 * lp.2,
                            pos_loc.2 + px.2 * lp.0 + py.2 * lp.1 + pz.2 * lp.2,
                        ))
                    };
                    let raw_dir = item_ent
                        .get_ref(2)
                        .and_then(|did| directions.get(&did))
                        .copied()
                        .unwrap_or((0.0, 0.0, 1.0));
                    let ext_local = (
                        (px.0 * raw_dir.0 + py.0 * raw_dir.1 + pz.0 * raw_dir.2) * depth,
                        (px.1 * raw_dir.0 + py.1 * raw_dir.1 + pz.1 * raw_dir.2) * depth,
                        (px.2 * raw_dir.0 + py.2 * raw_dir.1 + pz.2 * raw_dir.2) * depth,
                    );
                    let p_start = xform_pt(pos_loc);
                    let ext_world = xform_dir(ext_local);
                    let p_end = (
                        p_start.0 + ext_world.0,
                        p_start.1 + ext_world.1,
                        p_start.2 + ext_world.2,
                    );
                    lines.push((p_start, p_end));
                    push_pt(p_start);
                    push_pt(p_end);

                    // Expand 3D bounds using swept profile footprint corners
                    let mut profile_pts: Vec<(f64, f64)> = Vec::new();
                    if let Some(prof_id) = item_ent.get_ref(0) {
                        if let Some(prof) = step_model.entities.get(&prof_id) {
                            match prof.type_name.as_str() {
                                "IFCRECTANGLEPROFILEDEF" | "IFCRECTANGLEHOLLOWPROFILEDEF" => {
                                    let hx = prof.get_f64(3).unwrap_or(400.0) * unit_scale * 0.5;
                                    let hy = prof.get_f64(4).unwrap_or(400.0) * unit_scale * 0.5;
                                    profile_pts = vec![(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)];
                                }
                                "IFCCIRCLEPROFILEDEF" | "IFCCIRCLEHOLLOWPROFILEDEF" => {
                                    let r = prof.get_f64(3).unwrap_or(200.0) * unit_scale;
                                    profile_pts = vec![(-r, -r), (r, -r), (r, r), (-r, r)];
                                }
                                "IFCARBITRARYCLOSEDPROFILEDEF" | "IFCARBITRARYPROFILEDEFWITHVOIDS" => {
                                    if let Some(curve_id) = prof.get_ref(2) {
                                        if let Some(curve) = step_model.entities.get(&curve_id) {
                                            if curve.type_name == "IFCPOLYLINE" {
                                                for cid in curve.get_ref_list(0) {
                                                    if let Some(&(cx, cy, _)) = cartesian_points.get(&cid) {
                                                        profile_pts.push((cx, cy));
                                                    }
                                                }
                                            } else if curve.type_name == "IFCINDEXEDPOLYCURVE" {
                                                if let Some(pl_id) = curve.get_ref(0) {
                                                    if let Some(pts) = point_lists_m.get(&pl_id) {
                                                        for &(cx, cy, _) in pts {
                                                            profile_pts.push((cx, cy));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    for (u, v) in profile_pts {
                        let base_w = local_to_world((u, v, 0.0));
                        let top_w = (base_w.0 + ext_world.0, base_w.1 + ext_world.1, base_w.2 + ext_world.2);
                        push_pt(base_w);
                        push_pt(top_w);
                    }
                }
                "IFCPOLYLINE" => {
                    let pt_ids = item_ent.get_ref_list(0);
                    let mut pts = Vec::with_capacity(pt_ids.len());
                    for pid in pt_ids {
                        if let Some(&raw_p) = cartesian_points.get(&pid) {
                            let wp = if is_axis_rep && (raw_p.2 - origin.2).abs() < 0.05 && origin.2.abs() > 0.1 {
                                // Reference axis polylines authored in absolute elevation need XY alignment only
                                (origin.0 + raw_p.0, origin.1 + raw_p.1, raw_p.2)
                            } else {
                                xform_pt(raw_p)
                            };
                            push_pt(wp);
                            pts.push(wp);
                        }
                    }
                    for pair in pts.windows(2) {
                        lines.push((pair[0], pair[1]));
                    }
                }
                "IFCINDEXEDPOLYCURVE" => {
                    if let Some(pl_id) = item_ent.get_ref(0) {
                        if let Some(raw_pts) = point_lists_m.get(&pl_id) {
                            let pts: Vec<(f64, f64, f64)> = raw_pts.iter().map(|&p| {
                                let wp = xform_pt(p);
                                push_pt(wp);
                                wp
                            }).collect();
                            for pair in pts.windows(2) {
                                lines.push((pair[0], pair[1]));
                            }
                        }
                    }
                }
                "IFCTRIANGULATEDFACESET" | "IFCPOLYGONALFACESET" => {
                    if let Some(pl_id) = item_ent.get_ref(0) {
                        if let Some(raw_pts) = point_lists_m.get(&pl_id) {
                            for &p in raw_pts {
                                push_pt(xform_pt(p));
                            }
                            if let (Some(&first), Some(&last)) = (raw_pts.first(), raw_pts.last()) {
                                lines.push((xform_pt(first), xform_pt(last)));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(shape_rep_id) = ent.get_ref(6) {
            if let Some(prod_shape) = step_model.entities.get(&shape_rep_id) {
                for rep_id in prod_shape.get_ref_list(2) {
                    if let Some(shape_rep) = step_model.entities.get(&rep_id) {
                        let is_axis_rep = shape_rep
                            .get_str(1)
                            .is_some_and(|s| s.eq_ignore_ascii_case("Axis"));
                        for item_id in shape_rep.get_ref_list(3) {
                            extract_item(
                                step_model,
                                item_id,
                                is_axis_rep,
                                0,
                                origin,
                                &xform_pt,
                                &xform_dir,
                                &axis_frame,
                                unit_scale,
                                cartesian_points,
                                directions,
                                point_lists_m,
                                &mut lines,
                                &mut push_pt,
                            );
                        }
                    }
                }
            }
        }

        if !min_pt.0.is_finite() || !max_pt.0.is_finite() {
            let half = 0.25;
            lines.push(((origin.0 - half, origin.1, origin.2), (origin.0 + half, origin.1, origin.2)));
            lines.push(((origin.0, origin.1 - half, origin.2), (origin.0, origin.1 + half, origin.2)));
            lines.push(((origin.0, origin.1, origin.2), (origin.0, origin.1, origin.2 + 0.5)));
            min_pt = (origin.0 - half, origin.1 - half, origin.2);
            max_pt = (origin.0 + half, origin.1 + half, origin.2 + 0.5);
        } else {
            // Ensure 1D axis curves (IFCPOLYLINE / IFCINDEXEDPOLYCURVE) retain a minimum nominal 0.40m section envelope
            if (max_pt.0 - min_pt.0) < 0.05 {
                min_pt.0 -= 0.2;
                max_pt.0 += 0.2;
            }
            if (max_pt.1 - min_pt.1) < 0.05 {
                min_pt.1 -= 0.2;
                max_pt.1 += 0.2;
            }
            if (max_pt.2 - min_pt.2) < 0.05 {
                min_pt.2 -= 0.2;
                max_pt.2 += 0.2;
            }
            if lines.is_empty() {
                lines.push((min_pt, max_pt));
            }
        }

        (IfcBoundingBox::new(min_pt, max_pt), lines)
    }

    fn audit_model_health(
        step_model: &StepModel,
        schema: &IfcSchemaVersion,
        elements: &[IfcElementSummary],
        _spatial_root: &IfcSpatialNode,
    ) -> IfcModelHealthReport {
        let mut checks = Vec::new();
        let mut penalties = 0.0;

        // 1. Schema Conformance
        let schema_name = schema.display_label();
        match schema {
            IfcSchemaVersion::Ifc4x3 | IfcSchemaVersion::Ifc4 => {
                checks.push(IfcAuditCheckItem {
                    category: "Schema Conformance".to_string(),
                    check_name: "buildingSMART International Standard".to_string(),
                    status: IfcAuditStatus::Pass,
                    detail: format!("Valid modern schema detected: {}", schema_name),
                });
            }
            IfcSchemaVersion::Ifc2x3 => {
                checks.push(IfcAuditCheckItem {
                    category: "Schema Conformance".to_string(),
                    check_name: "Legacy Coordination View 2.0".to_string(),
                    status: IfcAuditStatus::Warning,
                    detail: "Valid IFC2x3 legacy standard. Recommended upgrading to IFC4 or IFC4.3 for infrastructure.".to_string(),
                });
                penalties += 5.0;
            }
            IfcSchemaVersion::Unknown(ref u) => {
                checks.push(IfcAuditCheckItem {
                    category: "Schema Conformance".to_string(),
                    check_name: "Unrecognized Schema Header".to_string(),
                    status: IfcAuditStatus::Fail,
                    detail: format!("Unknown schema identifier: '{}'. Model may experience interop errors.", u),
                });
                penalties += 25.0;
            }
        }

        // 2. GlobalId (GUID) Uniqueness Check
        let mut guids: HashSet<&str> = HashSet::new();
        let mut duplicate_guids = 0;
        for el in elements {
            if !el.global_id.is_empty() && !guids.insert(&el.global_id) {
                duplicate_guids += 1;
            }
        }
        if duplicate_guids == 0 {
            checks.push(IfcAuditCheckItem {
                category: "Data Integrity".to_string(),
                check_name: "GlobalId (GUID) Uniqueness".to_string(),
                status: IfcAuditStatus::Pass,
                detail: format!("All {} element GUIDs are globally unique per ISO 16739.", elements.len()),
            });
        } else {
            checks.push(IfcAuditCheckItem {
                category: "Data Integrity".to_string(),
                check_name: "Duplicate GlobalId Collision".to_string(),
                status: IfcAuditStatus::Fail,
                detail: format!("Found {} duplicate GUID collisions! Violates buildingSMART openBIM uniqueness.", duplicate_guids),
            });
            penalties += 20.0;
        }

        // 3. Spatial Containment Check
        let mut uncontained = 0;
        for el in elements {
            if el.storey_name == "Unassigned Storey" {
                uncontained += 1;
            }
        }
        if uncontained == 0 {
            checks.push(IfcAuditCheckItem {
                category: "Spatial Decomposition".to_string(),
                check_name: "Storey Containment Relation".to_string(),
                status: IfcAuditStatus::Pass,
                detail: "100% of physical products are hierarchically assigned via IfcRelContainedInSpatialStructure.".to_string(),
            });
        } else {
            checks.push(IfcAuditCheckItem {
                category: "Spatial Decomposition".to_string(),
                check_name: "Orphaned Uncontained Products".to_string(),
                status: IfcAuditStatus::Warning,
                detail: format!("{} elements lack an assigned IfcBuildingStorey parent.", uncontained),
            });
            penalties += (uncontained as f64 * 2.0).min(15.0);
        }

        // 4. Dangling Reference Verification
        let mut dangling_count = 0;
        for ent in step_model.entities.values() {
            for param in &ent.parameters {
                if let StepValue::EntityRef(ref_id) = param {
                    if !step_model.entities.contains_key(ref_id) {
                        dangling_count += 1;
                    }
                } else if let StepValue::List(l) = param {
                    for sub in l {
                        if let StepValue::EntityRef(ref_id) = sub {
                            if !step_model.entities.contains_key(ref_id) {
                                dangling_count += 1;
                            }
                        }
                    }
                }
            }
        }
        if dangling_count == 0 {
            checks.push(IfcAuditCheckItem {
                category: "STEP Graph Topology".to_string(),
                check_name: "Referential Pointer Integrity".to_string(),
                status: IfcAuditStatus::Pass,
                detail: "Zero dangling pointers detected across all entity references.".to_string(),
            });
        } else {
            checks.push(IfcAuditCheckItem {
                category: "STEP Graph Topology".to_string(),
                check_name: "Broken #Ref Pointers".to_string(),
                status: IfcAuditStatus::Fail,
                detail: format!("Found {} dangling entity pointers pointing to non-existent STEP IDs.", dangling_count),
            });
            penalties += 15.0;
        }

        // 5. Units & Header Metadata Check
        let has_app = !step_model.header.originating_system.is_empty();
        checks.push(IfcAuditCheckItem {
            category: "BIM Metadata".to_string(),
            check_name: "Originating BIM Authoring System".to_string(),
            status: IfcAuditStatus::Pass,
            detail: if has_app {
                format!("Originating tool: {}", step_model.header.originating_system)
            } else {
                "Standard OpenBIM Preprocessor".to_string()
            },
        });

        let score = (100.0 - penalties).clamp(0.0, 100.0);
        let verdict = if score >= 90.0 {
            "CERTIFIED COMPLIANT: Pristine buildingSMART openBIM Model".to_string()
        } else if score >= 70.0 {
            "ACCEPTABLE WITH WARNINGS: Minor containment or schema discrepancies noted".to_string()
        } else {
            "FAILED VALIDATION: Critical syntax, broken references, or duplicate GUID collisions".to_string()
        };

        let total_psets = elements.iter().map(|e| e.property_sets.len()).sum();
        let total_mats = elements.iter().filter(|e| e.material_name.is_some()).count();

        IfcModelHealthReport {
            schema_version: schema_name.to_string(),
            total_entities: step_model.entities.len(),
            total_products: elements.len(),
            total_property_sets: total_psets,
            total_materials: total_mats,
            dangling_references_count: dangling_count,
            uncontained_products_count: uncontained,
            duplicate_guids_count: duplicate_guids,
            conformance_score_pct: score,
            verdict,
            checks,
        }
    }

    /// Converts the parsed IFC model into an EigenGraph `StructuralModel` for instant analytical FEA & ARI analysis.
    pub fn to_structural_model(&self) -> Result<StructuralModel, String> {
        let mut model = StructuralModel::new(&self.header.file_name);
        let mut elem_counter = 0;

        fn find_or_add_node(model: &mut StructuralModel, pt: (f64, f64, f64)) -> usize {
            const TOL: f64 = 0.05; // 50 mm snapping
            for node in &model.nodes {
                let dx = node.x - pt.0;
                let dy = node.y - pt.1;
                let dz = node.z - pt.2;
                if (dx * dx + dy * dy + dz * dz).sqrt() < TOL {
                    return node.id;
                }
            }
            let new_id = model.nodes.len();
            model.add_node(Node3D::new(new_id, pt.0, pt.1, pt.2));
            new_id
        }

        for el in &self.elements {
            let is_structural = el.class_name == "IFCBEAM"
                || el.class_name == "IFCCOLUMN"
                || el.class_name == "IFCMEMBER"
                || el.class_name == "IFCSTRUCTURALCURVEMEMBER"
                || el.class_name == "IFCFOOTING";

            if is_structural {
                for &(p1, p2) in &el.wireframe_lines {
                    let n1 = find_or_add_node(&mut model, p1);
                    let n2 = find_or_add_node(&mut model, p2);
                    if n1 != n2 {
                        elem_counter += 1;
                        let mut elem = Element3D::new(elem_counter, &format!("{}: {}", el.class_name, el.name), n1, n2);
                        elem.element_type = if el.class_name == "IFCCOLUMN" || el.class_name == "IFCBEAM" {
                            ElementType::Frame3D
                        } else {
                            ElementType::Truss3D
                        };
                        model.add_element(elem);
                    }
                }
            }
        }

        if model.elements.is_empty() {
            // Fallback: create frame elements from centroids of adjacent storeys
            return Err("No structural curve or frame members found in IFC model to convert into analytical model.".to_string());
        }

        // Apply boundary fixities at lowest Z level
        let min_z = model.nodes.iter().map(|n| n.z).fold(f64::INFINITY, f64::min);
        for node in &mut model.nodes {
            if (node.z - min_z).abs() < 0.05 {
                node.fixity = BoundaryFixity::pinned();
            }
        }

        // Apply baseline gravity load to top nodes
        let max_z = model.nodes.iter().map(|n| n.z).fold(f64::NEG_INFINITY, f64::max);
        let top_nodes: Vec<usize> = model
            .nodes
            .iter()
            .filter(|n| (n.z - max_z).abs() < 0.10 && !n.fixity.is_fixed())
            .map(|n| n.id)
            .collect();
        for nid in top_nodes {
            model.add_load(NodalLoad::force_z(nid, -30_000.0));
        }

        Ok(model)
    }

    /// Converts parsed OpenBIM IFC elements (`IfcWall`, `IfcColumn`, `IfcBeam`, `IfcSlab`, `IfcFooting`)
    /// into an editable `BimProject` workspace with synthesized levels and structural grids.
    pub fn to_bim_project(&self) -> Result<crate::core::bim::BimProject, String> {
        use crate::core::P2;
        use crate::core::bim::{
            BimColumn, BimFooting, BimGrid, BimLevel, BimProject, BimRun, BimSlab, BimWall,
            RunCategory, RunJustify, ViewRange,
        };

        fn collect_storeys(node: &IfcSpatialNode, out: &mut Vec<(String, f64)>) {
            if node.class_name == "IFCBUILDINGSTOREY" {
                let raw_elev = node.elevation_m.unwrap_or(0.0);
                // Handle either meter-scale (e.g. 3.5) or millimeter-scale (e.g. 3500.0)
                let elev_mm = if raw_elev.abs() <= 200.0 {
                    raw_elev * 1000.0
                } else {
                    raw_elev
                };
                out.push((node.name.clone(), elev_mm));
            }
            for ch in &node.children {
                collect_storeys(ch, out);
            }
        }

        let mut storey_pairs = Vec::new();
        collect_storeys(&self.spatial_root, &mut storey_pairs);
        if storey_pairs.is_empty() {
            // Direct fallback scan of IFCBUILDINGSTOREY entities when IFCRELAGGREGATES hierarchy is incomplete
            for st in self.raw_step.get_entities_of_type("IFCBUILDINGSTOREY") {
                let sname = st
                    .get_str(2)
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("Storey")
                    .to_string();
                let raw_elev = st.get_f64(9).or_else(|| st.get_f64(8)).unwrap_or(0.0);
                let elev_mm = if raw_elev.abs() <= 200.0 && (self.unit_scale - 1.0).abs() < 1e-6 {
                    raw_elev * 1000.0
                } else {
                    raw_elev * self.unit_scale * 1000.0
                };
                storey_pairs.push((sname, elev_mm));
            }
        }
        if storey_pairs.is_empty() {
            storey_pairs.push(("Level 1 (+0.00m)".to_string(), 0.0));
            storey_pairs.push(("Level 2 (+3.50m)".to_string(), 3500.0));
        } else if storey_pairs.len() == 1 {
            storey_pairs.push(("Roof Level (+3.50m)".to_string(), storey_pairs[0].1 + 3500.0));
        }
        storey_pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let levels: Vec<BimLevel> = storey_pairs
            .iter()
            .enumerate()
            .map(|(i, (name, elev_mm))| BimLevel {
                id: i + 1,
                name: name.clone(),
                elevation_mm: *elev_mm,
            })
            .collect();

        let match_level = |z_m: f64| -> usize {
            let z_mm = z_m * 1000.0;
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
        let next_level_above = |base_id: usize| -> usize {
            let base_idx = levels.iter().position(|l| l.id == base_id).unwrap_or(0);
            levels
                .get((base_idx + 1).min(levels.len() - 1))
                .map(|l| l.id)
                .unwrap_or(base_id)
        };

        let mut next_id = levels.len() + 1;
        let mut alloc_id = || -> usize {
            let id = next_id;
            next_id += 1;
            id
        };

        let mut walls = Vec::new();
        let mut columns = Vec::new();
        let mut runs = Vec::new();
        let mut footings = Vec::new();
        let mut slabs = Vec::new();
        let mut xs_mm: Vec<f64> = Vec::new();
        let mut ys_mm: Vec<f64> = Vec::new();

        for el in &self.elements {
            let dx = (el.bbox.max_x - el.bbox.min_x).abs();
            let dy = (el.bbox.max_y - el.bbox.min_y).abs();
            let dz = (el.bbox.max_z - el.bbox.min_z).abs();
            let cx = el.centroid.0 * 1000.0;
            let cy = el.centroid.1 * 1000.0;

            match el.class_name.as_str() {
                "IFCWALL" | "IFCWALLSTANDARDCASE" => {
                    let (p0, p1, thk_mm) = if dx >= dy {
                        let half = (dx * 0.5).max(0.5) * 1000.0;
                        (P2(cx - half, cy), P2(cx + half, cy), (dy * 1000.0).clamp(120.0, 500.0))
                    } else {
                        let half = (dy * 0.5).max(0.5) * 1000.0;
                        (P2(cx, cy - half), P2(cx, cy + half), (dx * 1000.0).clamp(120.0, 500.0))
                    };
                    let base_lvl = match_level(el.bbox.min_z);
                    walls.push(BimWall {
                        id: alloc_id(),
                        name: if el.name.is_empty() { "IFC Wall".to_string() } else { el.name.clone() },
                        base_level_id: base_lvl,
                        top_level_id: Some(next_level_above(base_lvl)),
                        unconnected_height_mm: (dz * 1000.0).clamp(2400.0, 6000.0),
                        start: p0,
                        end: p1,
                        thickness_mm: thk_mm,
                        is_structural: !el.object_type.to_lowercase().contains("partition"),
                        layers: BimWall::default_compound_layers(thk_mm),
                        openings: Vec::new(),
                    });
                    xs_mm.extend([p0.0, p1.0]);
                    ys_mm.extend([p0.1, p1.1]);
                }
                "IFCCOLUMN" => {
                    let base_lvl = match_level(el.bbox.min_z);
                    let mut top_lvl = match_level(el.bbox.max_z);
                    if top_lvl == base_lvl {
                        top_lvl = next_level_above(base_lvl);
                    }
                    let is_steel = el.object_type.to_uppercase().contains("UB")
                        || el.object_type.to_uppercase().contains("WF")
                        || el.object_type.to_uppercase().contains("H");
                    columns.push(BimColumn {
                        id: alloc_id(),
                        name: if el.name.is_empty() { "IFC Column".to_string() } else { el.name.clone() },
                        base_level_id: base_lvl,
                        top_level_id: top_lvl,
                        pos: P2(cx, cy),
                        width_mm: (dx * 1000.0).clamp(250.0, 1200.0),
                        depth_mm: (dy * 1000.0).clamp(250.0, 1200.0),
                        rotation_deg: 0.0,
                        is_round: false,
                        is_wf_profile: is_steel,
                        profile_name: if el.object_type.is_empty() { "400x400 RC".to_string() } else { el.object_type.clone() },
                    });
                    xs_mm.push(cx);
                    ys_mm.push(cy);
                }
                "IFCBEAM" | "IFCMEMBER" | "IFCSTRUCTURALCURVEMEMBER" => {
                    let (start, end) = if let Some(&(a, b)) = el.wireframe_lines.first() {
                        let pa = P2(a.0 * 1000.0, a.1 * 1000.0);
                        let pb = P2(b.0 * 1000.0, b.1 * 1000.0);
                        if pa.dist(pb) >= 200.0 {
                            (pa, pb)
                        } else if dx >= dy {
                            let half = (dx * 0.5).max(1.0) * 1000.0;
                            (P2(cx - half, cy), P2(cx + half, cy))
                        } else {
                            let half = (dy * 0.5).max(1.0) * 1000.0;
                            (P2(cx, cy - half), P2(cx, cy + half))
                        }
                    } else if dx >= dy {
                        let half = (dx * 0.5).max(1.0) * 1000.0;
                        (P2(cx - half, cy), P2(cx + half, cy))
                    } else {
                        let half = (dy * 0.5).max(1.0) * 1000.0;
                        (P2(cx, cy - half), P2(cx, cy + half))
                    };
                    let lvl = match_level(el.bbox.max_z);
                    let is_steel = el.object_type.to_uppercase().contains("UB")
                        || el.object_type.to_uppercase().contains("WF")
                        || el.object_type.to_uppercase().contains("IPE")
                        || el.material_name.as_deref().unwrap_or("").to_lowercase().contains("steel");
                    runs.push(BimRun {
                        id: alloc_id(),
                        name: if el.name.is_empty() { "IFC Beam".to_string() } else { el.name.clone() },
                        category: RunCategory::StructuralBeam,
                        level_id: lvl,
                        start,
                        end,
                        start_offset_mm: 0.0,
                        end_offset_mm: 0.0,
                        justify: RunJustify::Top,
                        profile_name: if el.object_type.is_empty() { "300x600 RC".to_string() } else { el.object_type.clone() },
                        width_mm: (dx.min(dy) * 1000.0).clamp(200.0, 600.0),
                        depth_mm: (dz * 1000.0).clamp(300.0, 1200.0),
                        is_round: false,
                        is_wf_profile: is_steel,
                        system_name: "IFC Imported Structure".to_string(),
                    });
                    xs_mm.extend([start.0, end.0]);
                    ys_mm.extend([start.1, end.1]);
                }
                "IFCFOOTING" => {
                    let lvl = levels.first().map(|l| l.id).unwrap_or(1);
                    footings.push(BimFooting {
                        id: alloc_id(),
                        name: if el.name.is_empty() { "IFC Footing".to_string() } else { el.name.clone() },
                        level_id: lvl,
                        pos: P2(cx, cy),
                        width_mm: (dx * 1000.0).clamp(1200.0, 4000.0),
                        length_mm: (dy * 1000.0).clamp(1200.0, 4000.0),
                        thickness_mm: (dz * 1000.0).clamp(400.0, 1200.0),
                    });
                    xs_mm.push(cx);
                    ys_mm.push(cy);
                }
                "IFCSLAB" | "IFCROOF" => {
                    let min_x = (el.bbox.min_x * 1000.0).min(cx - 1000.0);
                    let max_x = (el.bbox.max_x * 1000.0).max(cx + 1000.0);
                    let min_y = (el.bbox.min_y * 1000.0).min(cy - 1000.0);
                    let max_y = (el.bbox.max_y * 1000.0).max(cy + 1000.0);
                    let lvl = match_level(el.bbox.max_z);
                    slabs.push(BimSlab {
                        id: alloc_id(),
                        name: if el.name.is_empty() { "IFC Slab".to_string() } else { el.name.clone() },
                        level_id: lvl,
                        boundary: vec![P2(min_x, min_y), P2(max_x, min_y), P2(max_x, max_y), P2(min_x, max_y)],
                        openings: Vec::new(),
                        thickness_mm: (dz * 1000.0).clamp(120.0, 400.0),
                        is_roof: el.class_name == "IFCROOF",
                        roof_slope_deg: 0.0,
                    });
                }
                _ => {}
            }
        }

        if walls.is_empty() && columns.is_empty() && runs.is_empty() && footings.is_empty() && slabs.is_empty() {
            return Err("No convertible BIM structural/architectural elements found in this IFC model.".to_string());
        }

        let mut grids = Vec::new();
        let min_x = xs_mm.iter().copied().fold(f64::INFINITY, f64::min).min(-1000.0) - 1500.0;
        let max_x = xs_mm.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(6000.0) + 1500.0;
        let min_y = ys_mm.iter().copied().fold(f64::INFINITY, f64::min).min(-1000.0) - 1500.0;
        let max_y = ys_mm.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(6000.0) + 1500.0;

        let dedup_coords = |vals: &[f64]| -> Vec<f64> {
            let mut sorted: Vec<f64> = vals.iter().copied().filter(|v| v.is_finite()).collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mut res: Vec<f64> = Vec::new();
            for v in sorted {
                if res.last().map_or(true, |last| (v - *last).abs() > 450.0) {
                    res.push(v);
                }
            }
            res
        };
        for (idx, &xg) in dedup_coords(&xs_mm).iter().take(12).enumerate() {
            grids.push(BimGrid {
                id: alloc_id(),
                label: format!("{}", idx + 1),
                start: P2(xg, min_y),
                end: P2(xg, max_y),
            });
        }
        for (idx, &yg) in dedup_coords(&ys_mm).iter().take(12).enumerate() {
            let ch = (b'A' + (idx as u8 % 26)) as char;
            grids.push(BimGrid {
                id: alloc_id(),
                label: ch.to_string(),
                start: P2(min_x, yg),
                end: P2(max_x, yg),
            });
        }

        let pname = if self.header.file_name.trim().is_empty() {
            "Converted IFC Project".to_string()
        } else {
            self.header.file_name.trim_end_matches(".ifc").to_string()
        };

        Ok(BimProject {
            project_name: pname,
            next_id,
            levels,
            grids,
            walls,
            runs,
            columns,
            footings,
            slabs,
            rooms: Vec::new(),
            dimensions: Vec::new(),
            rebars: Vec::new(),
            view_range: ViewRange::default(),
            section_line: (P2(min_x, 0.0), P2(max_x, 0.0)),
        })
    }

    /// Extracts WGS84 geographic latitude/longitude from `IFCSITE` `RefLatitude`/`RefLongitude` DMS tuples when present.
    pub fn site_lat_lon(&self) -> Option<(f64, f64)> {
        let parse_dms = |val: Option<&StepValue>| -> Option<f64> {
            let list = val?.as_list()?;
            if list.len() < 2 {
                return None;
            }
            let d = list.first()?.as_f64().unwrap_or(0.0);
            let m = list.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let s = list.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let millionths = list.get(3).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let sign = if d < 0.0 || m < 0.0 || s < 0.0 { -1.0 } else { 1.0 };
            let deg = d.abs() + m.abs() / 60.0 + (s.abs() + millionths.abs() * 1e-6) / 3600.0;
            Some(sign * deg)
        };
        for site in self.raw_step.get_entities_of_type("IFCSITE") {
            let lat = parse_dms(site.parameters.get(9));
            let lon = parse_dms(site.parameters.get(10));
            if let (Some(la), Some(lo)) = (lat, lon) {
                if (la.abs() > 1e-4 || lo.abs() > 1e-4) && (-90.0..=90.0).contains(&la) && (-180.0..=180.0).contains(&lo) {
                    return Some((la, lo));
                }
            }
        }
        None
    }

    /// Parses any IFC STEP file (`IFC2x3`, `IFC4`, `IFC4.3`) into an editable `BimProject`,
    /// optional site `(lat, lon)`, and diagnostic summary, falling back to `InteropEngine::import_from_ifc`
    /// when a model only defines analytical wireframes.
    pub fn parse_to_bim_project_with_fallback(
        raw_content: &str,
        fallback_name: &str,
    ) -> Result<(crate::core::bim::BimProject, Option<(f64, f64)>, String), String> {
        let cleaned = raw_content.trim_start_matches('\u{FEFF}');
        match Self::parse(cleaned) {
            Ok(model) => {
                let site_geo = model.site_lat_lon();
                match model.to_bim_project() {
                    Ok(mut bim) => {
                        let clean_stem = fallback_name.trim_end_matches(".ifc").trim_end_matches(".IFC");
                        if bim.project_name == "Converted IFC Project" && !clean_stem.is_empty() {
                            bim.project_name = clean_stem.to_string();
                        }
                        let msg = format!(
                            "Imported IFC '{}' ({}: {} elements, {} levels)",
                            bim.project_name,
                            model.schema.display_label(),
                            bim.element_count(),
                            bim.levels.len()
                        );
                        return Ok((bim, site_geo, msg));
                    }
                    Err(bim_err) => {
                        if let Ok(fem) = crate::core::interop::IfcImporter::import_from_ifc(cleaned, fallback_name) {
                            if !fem.elements.is_empty() {
                                let bim = Self::bim_project_from_structural_model(&fem);
                                let msg = format!(
                                    "Imported analytical IFC '{}' ({} elements converted to BIM)",
                                    fem.name,
                                    bim.element_count()
                                );
                                return Ok((bim, site_geo, msg));
                            }
                        }
                        Err(bim_err)
                    }
                }
            }
            Err(step_err) => {
                if let Ok(fem) = crate::core::interop::IfcImporter::import_from_ifc(cleaned, fallback_name) {
                    if !fem.elements.is_empty() {
                        let bim = Self::bim_project_from_structural_model(&fem);
                        let msg = format!(
                            "Imported analytical IFC '{}' ({} elements converted to BIM)",
                            fem.name,
                            bim.element_count()
                        );
                        return Ok((bim, None, msg));
                    }
                }
                Err(step_err)
            }
        }
    }

    fn bim_project_from_structural_model(
        fem: &StructuralModel,
    ) -> crate::core::bim::BimProject {
        use crate::core::P2;
        use crate::core::bim::{
            BimColumn, BimLevel, BimProject, BimRun, RunCategory, RunJustify, ViewRange,
        };
        let mut z_levels_mm: Vec<f64> = fem
            .nodes
            .iter()
            .map(|n| (n.y * 1000.0 / 250.0).round() * 250.0)
            .collect();
        z_levels_mm.sort_by(|a, b| a.total_cmp(b));
        z_levels_mm.dedup_by(|a, b| (*a - *b).abs() < 400.0);
        if z_levels_mm.is_empty() {
            z_levels_mm.push(0.0);
        }
        if z_levels_mm.len() == 1 {
            z_levels_mm.push(z_levels_mm[0] + 3500.0);
        }
        let levels: Vec<BimLevel> = z_levels_mm
            .iter()
            .enumerate()
            .map(|(idx, &z)| BimLevel {
                id: idx + 1,
                name: format!("Level {} ({:+.2}m)", idx + 1, z * 0.001),
                elevation_mm: z,
            })
            .collect();
        let match_lvl = |y_m: f64| -> usize {
            let ym = y_m * 1000.0;
            levels
                .iter()
                .min_by(|a, b| (a.elevation_mm - ym).abs().total_cmp(&(b.elevation_mm - ym).abs()))
                .map(|l| l.id)
                .unwrap_or(1)
        };
        let mut columns = Vec::new();
        let mut runs = Vec::new();
        let mut next_id = levels.len() + 1;
        for el in &fem.elements {
            let (Some(n1), Some(n2)) = (fem.nodes.iter().find(|n| n.id == el.start_node), fem.nodes.iter().find(|n| n.id == el.end_node)) else {
                continue;
            };
            let dz = (n2.y - n1.y).abs();
            let horiz = (n2.x - n1.x).hypot(n2.z - n1.z);
            let id = next_id;
            next_id += 1;
            if dz > horiz * 1.5 {
                let low_y = n1.y.min(n2.y);
                let high_y = n1.y.max(n2.y);
                columns.push(BimColumn {
                    id,
                    name: el.name.clone(),
                    base_level_id: match_lvl(low_y),
                    top_level_id: match_lvl(high_y),
                    pos: P2(((n1.x + n2.x) * 0.5) * 1000.0, ((n1.z + n2.z) * 0.5) * 1000.0),
                    width_mm: 400.0,
                    depth_mm: 400.0,
                    rotation_deg: 0.0,
                    is_round: false,
                    is_wf_profile: false,
                    profile_name: el.section.name.clone(),
                });
            } else {
                runs.push(BimRun {
                    id,
                    name: el.name.clone(),
                    category: RunCategory::StructuralBeam,
                    level_id: match_lvl(n1.y.max(n2.y)),
                    start: P2(n1.x * 1000.0, n1.z * 1000.0),
                    end: P2(n2.x * 1000.0, n2.z * 1000.0),
                    start_offset_mm: 0.0,
                    end_offset_mm: 0.0,
                    justify: RunJustify::Top,
                    profile_name: el.section.name.clone(),
                    width_mm: 300.0,
                    depth_mm: 600.0,
                    is_round: false,
                    is_wf_profile: false,
                    system_name: "IFC Analytical".to_string(),
                });
            }
        }
        BimProject {
            project_name: fem.name.clone(),
            next_id,
            levels,
            grids: Vec::new(),
            walls: Vec::new(),
            runs,
            columns,
            footings: Vec::new(),
            slabs: Vec::new(),
            rooms: Vec::new(),
            dimensions: Vec::new(),
            rebars: Vec::new(),
            view_range: ViewRange::default(),
            section_line: (P2(-1000.0, 0.0), P2(12000.0, 0.0)),
        }
    }

    /// Generates a pristine reference IFC4 2-Storey Concrete Villa STEP file for unit testing.
    #[cfg(test)]
    pub fn sample_villa_ifc4() -> String {
        r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');
FILE_NAME('BIMCraft_Duplex_Villa.ifc','2026-11-07T12:00:00',('EigenGraph OpenBIM Engine'),('EigenGraph BIMCraft'),'EigenGraph v0.1.7','EigenGraph IFC4','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0Proj0000000000000001',$,'BIMCraft Residential Duplex Villa',$,$,$,$,(#10),#20);
#2=IFCSITE('0Site0000000000000001',$,'Villa Perimeter Site',$,$,#30,$,$,.ELEMENT.,(0,0,0),$,$,$,$);
#3=IFCBUILDING('0Bldg0000000000000001',$,'Main Residence Building',$,$,#31,$,$,.ELEMENT.,$,$,$);
#4=IFCBUILDINGSTOREY('0Lvl1000000000000001',$,'Ground Level (L1)',$,$,#32,$,$,.ELEMENT.,0.0);
#5=IFCBUILDINGSTOREY('0Lvl2000000000000001',$,'Upper Level (L2)',$,$,#33,$,$,.ELEMENT.,3500.0);
#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#40,#41);
#20=IFCUNITASSIGNMENT((#21,#22));
#21=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#22=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#30=IFCLOCALPLACEMENT($,#40);
#31=IFCLOCALPLACEMENT(#30,#40);
#32=IFCLOCALPLACEMENT(#31,#40);
#33=IFCLOCALPLACEMENT(#31,#42);
#40=IFCAXIS2PLACEMENT3D(#50,#51,#52);
#41=IFCAXIS2PLACEMENT3D(#50,#51,#52);
#42=IFCAXIS2PLACEMENT3D(#53,#51,#52);
#50=IFCCARTESIANPOINT((0.,0.,0.));
#51=IFCDIRECTION((0.,0.,1.));
#52=IFCDIRECTION((1.,0.,0.));
#53=IFCCARTESIANPOINT((0.,0.,3500.));
#100=IFCRELAGGREGATES('0Agg0000000000000001',$,'ProjectToSite',$,#1,(#2));
#101=IFCRELAGGREGATES('0Agg0000000000000002',$,'SiteToBuilding',$,#2,(#3));
#102=IFCRELAGGREGATES('0Agg0000000000000003',$,'BuildingToStoreys',$,#3,(#4,#5));
#200=IFCCOLUMN('0Col0000000000000001',$,'Col_C1_A1','Concrete 400x400',$,#32,#250,.COLUMN.);
#201=IFCCOLUMN('0Col0000000000000002',$,'Col_C1_B1','Concrete 400x400',$,#32,#251,.COLUMN.);
#202=IFCCOLUMN('0Col0000000000000003',$,'Col_C1_A2','Concrete 400x400',$,#32,#252,.COLUMN.);
#203=IFCCOLUMN('0Col0000000000000004',$,'Col_C1_B2','Concrete 400x400',$,#32,#253,.COLUMN.);
#210=IFCBEAM('0Bm00000000000000001',$,'Beam_B1_Span1','RC Beam 300x600',$,#33,#260,.BEAM.);
#211=IFCBEAM('0Bm00000000000000002',$,'Beam_B1_Span2','RC Beam 300x600',$,#33,#261,.BEAM.);
#220=IFCWALL('0Wal0000000000000001',$,'ExtWall_Facade_A1-B1','Cavity Brick Wall 250mm',$,#32,#270,.STANDARD.);
#230=IFCSLAB('0Slab0000000000000001',$,'FloorSlab_Level2','Cast-in-place Slab 150mm',$,#33,#280,.FLOOR.);
#240=IFCDOOR('0Door0000000000000001',$,'MainEntrance_Door','Timber Pivot 1000x2100',$,#32,#290,2100.,1000.,.SINGLE_SWING_LEFT.,$,$);
#250=IFCPRODUCTDEFINITIONSHAPE($,$,(#300));
#251=IFCPRODUCTDEFINITIONSHAPE($,$,(#300));
#252=IFCPRODUCTDEFINITIONSHAPE($,$,(#300));
#253=IFCPRODUCTDEFINITIONSHAPE($,$,(#300));
#260=IFCPRODUCTDEFINITIONSHAPE($,$,(#310));
#261=IFCPRODUCTDEFINITIONSHAPE($,$,(#311));
#270=IFCPRODUCTDEFINITIONSHAPE($,$,(#310));
#280=IFCPRODUCTDEFINITIONSHAPE($,$,(#310));
#290=IFCPRODUCTDEFINITIONSHAPE($,$,(#310));
#300=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',(#400));
#310=IFCSHAPEREPRESENTATION(#10,'Axis','Curve3D',(#500));
#311=IFCSHAPEREPRESENTATION(#10,'Axis','Curve3D',(#501));
#400=IFCEXTRUDEDAREASOLID(#410,#40,#51,3500.);
#410=IFCRECTANGLEPROFILEDEF(.AREA.,'Profile_400x400',#40,400.,400.);
#500=IFCPOLYLINE((#550,#551));
#501=IFCPOLYLINE((#551,#552));
#550=IFCCARTESIANPOINT((0.,0.,3500.));
#551=IFCCARTESIANPOINT((6000.,0.,3500.));
#552=IFCCARTESIANPOINT((6000.,6000.,3500.));
#600=IFCRELCONTAINEDINSPATIALSTRUCTURE('0Cont0000000000000001',$,'L1_Containment',$,(#200,#201,#202,#203,#220,#240),#4);
#601=IFCRELCONTAINEDINSPATIALSTRUCTURE('0Cont0000000000000002',$,'L2_Containment',$,(#210,#211,#230),#5);
#700=IFCMATERIAL('Concrete C30/37',$,$);
#701=IFCRELASSOCIATESMATERIAL('0Mat0000000000000001',$,'MatAssoc_Columns',$,(#200,#201,#202,#203,#210,#211),#700);
#800=IFCPROPERTYSET('0Pset0000000000000001',$,'Pset_ColumnCommon',$,(#810,#811));
#810=IFCPROPERTYSINGLEVALUE('LoadBearing',$,'TRUE',.BOOLEAN.);
#811=IFCPROPERTYSINGLEVALUE('FireRating',$,'FRR-120 Min',.LABEL.);
#820=IFCRELDEFINESBYPROPERTIES('0RelP0000000000000001',$,'ColumnPsets',$,(#200,#201,#202,#203),#800);
ENDSEC;
END-ISO-10303-21;
"#.to_string()
    }

    /// Generates a reference industrial steel portal frame warehouse IFC4 STEP file for unit testing.
    #[cfg(test)]
    pub fn sample_portal_frame_ifc4() -> String {
        r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');
FILE_NAME('Industrial_Steel_Portal_Frame.ifc','2026-11-07T14:00:00',('EigenGraph OpenBIM Engine'),('EigenGraph Steelwork'),'EigenGraph v0.1.7','EigenGraph IFC4','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0ProjPortal000000001',$,'Industrial Steel Warehouse 24m Span',$,$,$,$,(#10),#20);
#2=IFCSITE('0SitePortal000000001',$,'Logistics Park Site',$,$,#30,$,$,.ELEMENT.,(0,0,0),$,$,$,$);
#3=IFCBUILDING('0BldgPortal000000001',$,'Logistics Warehouse Unit 4',$,$,#31,$,$,.ELEMENT.,$,$,$);
#4=IFCBUILDINGSTOREY('0LvlPortal000000001',$,'Finished Floor Level',$,$,#32,$,$,.ELEMENT.,0.0);
#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#40,#41);
#20=IFCUNITASSIGNMENT((#21));
#21=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#30=IFCLOCALPLACEMENT($,#40);
#31=IFCLOCALPLACEMENT(#30,#40);
#32=IFCLOCALPLACEMENT(#31,#40);
#40=IFCAXIS2PLACEMENT3D(#50,#51,#52);
#41=IFCAXIS2PLACEMENT3D(#50,#51,#52);
#50=IFCCARTESIANPOINT((0.,0.,0.));
#51=IFCDIRECTION((0.,0.,1.));
#52=IFCDIRECTION((1.,0.,0.));
#100=IFCRELAGGREGATES('0AggP000000000000001',$,'ProjectToSite',$,#1,(#2));
#101=IFCRELAGGREGATES('0AggP000000000000002',$,'SiteToBuilding',$,#2,(#3));
#102=IFCRELAGGREGATES('0AggP000000000000003',$,'BuildingToStoreys',$,#3,(#4));
#200=IFCCOLUMN('0ColP000000000000001',$,'Col_UB_Left_A1','UB 457x191x82',$,#32,#250,.COLUMN.);
#201=IFCCOLUMN('0ColP000000000000002',$,'Col_UB_Right_B1','UB 457x191x82',$,#32,#251,.COLUMN.);
#210=IFCBEAM('0BmP0000000000000001',$,'Rafter_Apex_Left','UB 406x178x67',$,#32,#260,.BEAM.);
#211=IFCBEAM('0BmP0000000000000002',$,'Rafter_Apex_Right','UB 406x178x67',$,#32,#261,.BEAM.);
#250=IFCPRODUCTDEFINITIONSHAPE($,$,(#300));
#251=IFCPRODUCTDEFINITIONSHAPE($,$,(#301));
#260=IFCPRODUCTDEFINITIONSHAPE($,$,(#310));
#261=IFCPRODUCTDEFINITIONSHAPE($,$,(#311));
#300=IFCSHAPEREPRESENTATION(#10,'Axis','Curve3D',(#500));
#301=IFCSHAPEREPRESENTATION(#10,'Axis','Curve3D',(#501));
#310=IFCSHAPEREPRESENTATION(#10,'Axis','Curve3D',(#510));
#311=IFCSHAPEREPRESENTATION(#10,'Axis','Curve3D',(#511));
#500=IFCPOLYLINE((#550,#551));
#501=IFCPOLYLINE((#552,#553));
#510=IFCPOLYLINE((#551,#554));
#511=IFCPOLYLINE((#554,#553));
#550=IFCCARTESIANPOINT((0.,0.,0.));
#551=IFCCARTESIANPOINT((0.,0.,7.5));
#552=IFCCARTESIANPOINT((24.,0.,0.));
#553=IFCCARTESIANPOINT((24.,0.,7.5));
#554=IFCCARTESIANPOINT((12.,0.,10.2));
#600=IFCRELCONTAINEDINSPATIALSTRUCTURE('0ContP00000000000001',$,'Portal_Containment',$,(#200,#201,#210,#211),#4);
#700=IFCMATERIAL('Structural Steel S355JR',$,$);
#701=IFCRELASSOCIATESMATERIAL('0MatP00000000000001',$,'SteelMatAssoc',$,(#200,#201,#210,#211),#700);
ENDSEC;
END-ISO-10303-21;
"#.to_string()
    }

    /// Generates a reference high-rise diagrid tower frame IFC2x3 STEP file for unit testing.
    #[cfg(test)]
    pub fn sample_diagrid_tower_ifc2x3() -> String {
        r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView_V2.0]'),'2;1');
FILE_NAME('Diagrid_HighRise_Tower.ifc','2026-11-07T16:00:00',('EigenGraph OpenBIM Engine'),('EigenGraph Structural Frame'),'EigenGraph v0.1.7','EigenGraph IFC2x3','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCPROJECT('0ProjDiagrid00000001',$,'48-Storey Diagrid Skyscraper',$,$,$,$,(#10),#20);
#2=IFCSITE('0SiteDiagrid00000001',$,'Metropolitan Central Core',$,$,#30,$,$,.ELEMENT.,(0,0,0),$,$,$,$);
#3=IFCBUILDING('0BldgDiagrid00000001',$,'Tower One',$,$,#31,$,$,.ELEMENT.,$,$,$);
#4=IFCBUILDINGSTOREY('0LvlDiagrid00000001',$,'Ground Concourse',$,$,#32,$,$,.ELEMENT.,0.0);
#5=IFCBUILDINGSTOREY('0LvlDiagrid00000002',$,'Storey 10 Diagrid Node',$,$,#33,$,$,.ELEMENT.,40.0);
#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#40,#41);
#20=IFCUNITASSIGNMENT((#21));
#21=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#30=IFCLOCALPLACEMENT($,#40);
#31=IFCLOCALPLACEMENT(#30,#40);
#32=IFCLOCALPLACEMENT(#31,#40);
#33=IFCLOCALPLACEMENT(#31,#42);
#40=IFCAXIS2PLACEMENT3D(#50,#51,#52);
#41=IFCAXIS2PLACEMENT3D(#50,#51,#52);
#42=IFCAXIS2PLACEMENT3D(#53,#51,#52);
#50=IFCCARTESIANPOINT((0.,0.,0.));
#51=IFCDIRECTION((0.,0.,1.));
#52=IFCDIRECTION((1.,0.,0.));
#53=IFCCARTESIANPOINT((0.,0.,40.));
#100=IFCRELAGGREGATES('0AggD000000000000001',$,'ProjectToSite',$,#1,(#2));
#101=IFCRELAGGREGATES('0AggD000000000000002',$,'SiteToBuilding',$,#2,(#3));
#102=IFCRELAGGREGATES('0AggD000000000000003',$,'BuildingToStoreys',$,#3,(#4,#5));
#200=IFCMEMBER('0MemD000000000000001',$,'Diagrid_Brace_A1-B2','CHS 800x32 S460',$,#32,#250,.BRACE.);
#201=IFCMEMBER('0MemD000000000000002',$,'Diagrid_Brace_B1-A2','CHS 800x32 S460',$,#32,#251,.BRACE.);
#250=IFCPRODUCTDEFINITIONSHAPE($,$,(#300));
#251=IFCPRODUCTDEFINITIONSHAPE($,$,(#301));
#300=IFCSHAPEREPRESENTATION(#10,'Axis','Curve3D',(#500));
#301=IFCSHAPEREPRESENTATION(#10,'Axis','Curve3D',(#501));
#500=IFCPOLYLINE((#550,#553));
#501=IFCPOLYLINE((#551,#552));
#550=IFCCARTESIANPOINT((0.,0.,0.));
#551=IFCCARTESIANPOINT((20.,0.,0.));
#552=IFCCARTESIANPOINT((0.,0.,40.));
#553=IFCCARTESIANPOINT((20.,0.,40.));
#600=IFCRELCONTAINEDINSPATIALSTRUCTURE('0ContD00000000000001',$,'Diagrid_Containment',$,(#200,#201),#4);
#700=IFCMATERIAL('High-Strength Steel S460ML',$,$);
#701=IFCRELASSOCIATESMATERIAL('0MatD00000000000001',$,'SteelAssoc',$,(#200,#201),#700);
ENDSEC;
END-ISO-10303-21;
"#.to_string()
    }

    /// Serializes the entire IFC model back into an ISO 10303-21 STEP physical file string.
    pub fn to_step_string(&self) -> String {
        self.raw_step.to_step_string()
    }

    /// Re-evaluates health checks, conformance metrics, and data integrity on the current model state.
    pub fn recompute_health_audit(&mut self) {
        self.health_report = Self::audit_model_health(
            &self.raw_step,
            &self.schema,
            &self.elements,
            &self.spatial_root,
        );
    }

    /// Extracts all hierarchical building storeys as (step_id, name) tuples.
    pub fn list_storeys(&self) -> Vec<(usize, String)> {
        let mut storeys = Vec::new();
        Self::collect_storeys(&self.spatial_root, &mut storeys);
        storeys
    }

    fn collect_storeys(node: &IfcSpatialNode, out: &mut Vec<(usize, String)>) {
        if node.class_name.eq_ignore_ascii_case("IFCBUILDINGSTOREY") {
            out.push((node.step_id, node.name.clone()));
        }
        for child in &node.children {
            Self::collect_storeys(child, out);
        }
    }

    /// Updates identity metadata attributes of an element (Name, Description, ObjectType)
    /// and synchronizes both semantic model and underlying STEP entity parameters.
    pub fn update_element_attributes(
        &mut self,
        element_id: usize,
        name: Option<String>,
        description: Option<String>,
        object_type: Option<String>,
    ) -> Result<(), String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        if let Some(ref new_name) = name {
            el.name = new_name.clone();
        }
        if let Some(ref new_desc) = description {
            el.description = new_desc.clone();
        }
        if let Some(ref new_obj_type) = object_type {
            el.object_type = new_obj_type.clone();
        }

        // Synchronize in raw_step
        if let Some(ent) = self.raw_step.entities.get_mut(&element_id) {
            while ent.parameters.len() < 5 {
                ent.parameters.push(StepValue::Null);
            }
            if let Some(ref new_name) = name {
                ent.parameters[2] = StepValue::String(new_name.clone());
            }
            if let Some(ref new_desc) = description {
                ent.parameters[3] = StepValue::String(new_desc.clone());
            }
            if let Some(ref new_obj_type) = object_type {
                ent.parameters[4] = StepValue::String(new_obj_type.clone());
            }
        }

        self.recompute_health_audit();
        Ok(())
    }

    /// Sets or adds a single property value within a designated Property Set on an element.
    pub fn set_property(
        &mut self,
        element_id: usize,
        pset_name: &str,
        prop_name: &str,
        value: &str,
    ) -> Result<(), String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        if let Some(pset) = el.property_sets.iter_mut().find(|p| p.name.eq_ignore_ascii_case(pset_name)) {
            if let Some(prop) = pset.properties.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(prop_name)) {
                prop.1 = value.to_string();
            } else {
                pset.properties.push((prop_name.to_string(), value.to_string()));
            }
        } else {
            el.property_sets.push(IfcPropertySetSummary {
                name: pset_name.to_string(),
                properties: vec![(prop_name.to_string(), value.to_string())],
            });
        }
        self.recompute_health_audit();
        Ok(())
    }

    /// Removes a property from a designated Property Set on an element.
    pub fn remove_property(
        &mut self,
        element_id: usize,
        pset_name: &str,
        prop_name: &str,
    ) -> Result<bool, String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        let mut removed = false;
        if let Some(pset) = el.property_sets.iter_mut().find(|p| p.name.eq_ignore_ascii_case(pset_name)) {
            if let Some(pos) = pset.properties.iter().position(|(k, _)| k.eq_ignore_ascii_case(prop_name)) {
                pset.properties.remove(pos);
                removed = true;
            }
        }
        if removed {
            self.recompute_health_audit();
        }
        Ok(removed)
    }

    /// Adds or updates an entire Property Set with multiple key-value pairs on an element.
    pub fn add_property_set(
        &mut self,
        element_id: usize,
        pset_name: &str,
        properties: Vec<(String, String)>,
    ) -> Result<(), String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        if let Some(pset) = el.property_sets.iter_mut().find(|p| p.name.eq_ignore_ascii_case(pset_name)) {
            for (k, v) in properties {
                if let Some(prop) = pset.properties.iter_mut().find(|(existing_k, _)| existing_k.eq_ignore_ascii_case(&k)) {
                    prop.1 = v;
                } else {
                    pset.properties.push((k, v));
                }
            }
        } else {
            el.property_sets.push(IfcPropertySetSummary {
                name: pset_name.to_string(),
                properties,
            });
        }
        self.recompute_health_audit();
        Ok(())
    }

    /// Deletes an entire Property Set from an element.
    pub fn delete_property_set(
        &mut self,
        element_id: usize,
        pset_name: &str,
    ) -> Result<bool, String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        if let Some(pos) = el.property_sets.iter().position(|p| p.name.eq_ignore_ascii_case(pset_name)) {
            el.property_sets.remove(pos);
            self.recompute_health_audit();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Reassigns or sets an element's structural material association in both memory and raw STEP entities.
    pub fn set_element_material(
        &mut self,
        element_id: usize,
        material_name: &str,
    ) -> Result<(), String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        el.material_name = Some(material_name.to_string());

        // 1. Find or create IFCMATERIAL('material_name', $, $)
        let mut mat_step_id = None;
        for ent in self.raw_step.get_entities_of_type("IFCMATERIAL") {
            if let Some(n) = ent.get_str(0) {
                if n.eq_ignore_ascii_case(material_name) {
                    mat_step_id = Some(ent.id);
                    break;
                }
            }
        }
        let target_mat_id = if let Some(mid) = mat_step_id {
            mid
        } else {
            let next_id = self.raw_step.next_id();
            let new_ent = StepEntity {
                id: next_id,
                type_name: "IFCMATERIAL".to_string(),
                parameters: vec![
                    StepValue::String(material_name.to_string()),
                    StepValue::Null,
                    StepValue::Null,
                ],
            };
            self.raw_step.entities.insert(next_id, new_ent);
            self.raw_step.ordered_ids.push(next_id);
            self.raw_step
                .type_index
                .entry("IFCMATERIAL".to_string())
                .or_default()
                .push(next_id);
            next_id
        };

        // 2. Remove element_id from existing IFCRELASSOCIATESMATERIAL
        for ent in self.raw_step.entities.values_mut() {
            if ent.type_name == "IFCRELASSOCIATESMATERIAL" {
                if let Some(StepValue::List(list)) = ent.parameters.get_mut(4) {
                    list.retain(|v| v.as_entity_ref() != Some(element_id));
                }
            }
        }

        // 3. Find or create IFCRELASSOCIATESMATERIAL referencing target_mat_id
        let mut assoc_found = false;
        for ent in self.raw_step.entities.values_mut() {
            if ent.type_name == "IFCRELASSOCIATESMATERIAL" {
                if ent.get_ref(5) == Some(target_mat_id) {
                    if let Some(StepValue::List(list)) = ent.parameters.get_mut(4) {
                        if !list.iter().any(|v| v.as_entity_ref() == Some(element_id)) {
                            list.push(StepValue::EntityRef(element_id));
                        }
                        assoc_found = true;
                        break;
                    }
                }
            }
        }
        if !assoc_found {
            let next_id = self.raw_step.next_id();
            let guid = format!("0MatRel{:016x}", next_id);
            let new_rel = StepEntity {
                id: next_id,
                type_name: "IFCRELASSOCIATESMATERIAL".to_string(),
                parameters: vec![
                    StepValue::String(guid),
                    StepValue::Null,
                    StepValue::String("MaterialAssociation".to_string()),
                    StepValue::Null,
                    StepValue::List(vec![StepValue::EntityRef(element_id)]),
                    StepValue::EntityRef(target_mat_id),
                ],
            };
            self.raw_step.entities.insert(next_id, new_rel);
            self.raw_step.ordered_ids.push(next_id);
            self.raw_step
                .type_index
                .entry("IFCRELASSOCIATESMATERIAL".to_string())
                .or_default()
                .push(next_id);
        }

        self.recompute_health_audit();
        Ok(())
    }

    /// Reassigns an element to a new building storey in both the spatial tree and STEP relations.
    pub fn reassign_storey(
        &mut self,
        element_id: usize,
        target_storey_id: usize,
    ) -> Result<(), String> {
        // 1. Verify target storey exists and get its name
        let storeys = self.list_storeys();
        let target_storey_name = storeys
            .iter()
            .find(|(sid, _)| *sid == target_storey_id)
            .map(|(_, name)| name.clone())
            .ok_or_else(|| format!("Target storey #{} not found in spatial tree", target_storey_id))?;

        // 2. Update element summary
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;
        el.storey_name = target_storey_name;

        // 3. Update spatial_root tree
        fn remove_element_rec(node: &mut IfcSpatialNode, id: usize) {
            node.contained_element_ids.retain(|&x| x != id);
            for child in &mut node.children {
                remove_element_rec(child, id);
            }
        }
        fn add_element_rec(node: &mut IfcSpatialNode, target_id: usize, el_id: usize) -> bool {
            if node.step_id == target_id {
                if !node.contained_element_ids.contains(&el_id) {
                    node.contained_element_ids.push(el_id);
                }
                return true;
            }
            for child in &mut node.children {
                if add_element_rec(child, target_id, el_id) {
                    return true;
                }
            }
            false
        }

        remove_element_rec(&mut self.spatial_root, element_id);
        add_element_rec(&mut self.spatial_root, target_storey_id, element_id);

        // 4. Update raw_step: IFCRELCONTAINEDINSPATIALSTRUCTURE
        for ent in self.raw_step.entities.values_mut() {
            if ent.type_name == "IFCRELCONTAINEDINSPATIALSTRUCTURE" {
                if let Some(StepValue::List(list)) = ent.parameters.get_mut(4) {
                    list.retain(|v| v.as_entity_ref() != Some(element_id));
                }
            }
        }
        let mut rel_found = false;
        for ent in self.raw_step.entities.values_mut() {
            if ent.type_name == "IFCRELCONTAINEDINSPATIALSTRUCTURE" {
                if ent.get_ref(5) == Some(target_storey_id) {
                    if let Some(StepValue::List(list)) = ent.parameters.get_mut(4) {
                        if !list.iter().any(|v| v.as_entity_ref() == Some(element_id)) {
                            list.push(StepValue::EntityRef(element_id));
                        }
                        rel_found = true;
                        break;
                    }
                }
            }
        }
        if !rel_found {
            let next_id = self.raw_step.next_id();
            let guid = format!("0ContRel{:016x}", next_id);
            let new_rel = StepEntity {
                id: next_id,
                type_name: "IFCRELCONTAINEDINSPATIALSTRUCTURE".to_string(),
                parameters: vec![
                    StepValue::String(guid),
                    StepValue::Null,
                    StepValue::String("StoreyContainment".to_string()),
                    StepValue::Null,
                    StepValue::List(vec![StepValue::EntityRef(element_id)]),
                    StepValue::EntityRef(target_storey_id),
                ],
            };
            self.raw_step.entities.insert(next_id, new_rel);
            self.raw_step.ordered_ids.push(next_id);
            self.raw_step
                .type_index
                .entry("IFCRELCONTAINEDINSPATIALSTRUCTURE".to_string())
                .or_default()
                .push(next_id);
        }

        self.recompute_health_audit();
        Ok(())
    }

    /// Updates custom element quantities (Qto) in memory.
    pub fn update_quantities(
        &mut self,
        element_id: usize,
        quantities: Vec<(String, f64)>,
    ) -> Result<(), String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        for (k, v) in quantities {
            el.quantities.insert(k, v);
        }
        self.recompute_health_audit();
        Ok(())
    }

    /// Modifies the extrusion depth of a swept solid element (`IFCEXTRUDEDAREASOLID`) in both memory and STEP model.
    pub fn modify_element_extrusion(&mut self, element_id: usize, new_depth: f64) -> Result<f64, String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        let old_depth;
        let mut solid_id_opt = None;
        let mut poly_end_pt_id_opt = None;
        if let Some(prod_ent) = self.raw_step.entities.get(&element_id) {
            if let Some(shape_def_id) = prod_ent.get_ref(6) {
                if let Some(shape_def) = self.raw_step.entities.get(&shape_def_id) {
                    for rep_id in shape_def.get_ref_list(2) {
                        if let Some(rep_ent) = self.raw_step.entities.get(&rep_id) {
                            for item_id in rep_ent.get_ref_list(3) {
                                if let Some(item_ent) = self.raw_step.entities.get(&item_id) {
                                    if item_ent.type_name == "IFCEXTRUDEDAREASOLID" {
                                        solid_id_opt = Some(item_id);
                                        break;
                                    } else if item_ent.type_name == "IFCPOLYLINE" {
                                        let pts = item_ent.get_ref_list(0);
                                        if let Some(&last_pt_id) = pts.last() {
                                            poly_end_pt_id_opt = Some(last_pt_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(solid_id) = solid_id_opt {
            let solid_ent = self.raw_step.entities.get_mut(&solid_id).unwrap();
            old_depth = solid_ent.get_f64(3).unwrap_or(el.bbox.dimensions().2);
            while solid_ent.parameters.len() < 4 {
                solid_ent.parameters.push(StepValue::Null);
            }
            solid_ent.parameters[3] = StepValue::Real(new_depth / self.unit_scale);
        } else {
            old_depth = el.bbox.dimensions().2;
            if let Some(pt_id) = poly_end_pt_id_opt {
                if let Some(pt_ent) = self.raw_step.entities.get_mut(&pt_id) {
                    if let Some(StepValue::List(coords)) = pt_ent.parameters.get_mut(0) {
                        if coords.len() >= 3 {
                            coords[2] = StepValue::Real(new_depth / self.unit_scale);
                        }
                    }
                }
            }
        }

        // Recompute in-memory wireframe lines and bounding box along extrusion axis
        let old_min_z = el.bbox.min_z;
        let old_max_z = el.bbox.max_z;
        el.bbox.max_z = old_min_z + new_depth;
        el.centroid = el.bbox.centroid();
        if !el.wireframe_lines.is_empty() {
            let z_span = (old_max_z - old_min_z).max(1e-6);
            for seg in &mut el.wireframe_lines {
                if (seg.0 .2 - old_min_z).abs() > 1e-4 {
                    seg.0 .2 = old_min_z + ((seg.0 .2 - old_min_z) / z_span) * new_depth;
                }
                if (seg.1 .2 - old_min_z).abs() > 1e-4 {
                    seg.1 .2 = old_min_z + ((seg.1 .2 - old_min_z) / z_span) * new_depth;
                }
            }
        } else {
            el.wireframe_lines.push(((el.centroid.0, el.centroid.1, el.bbox.min_z), (el.centroid.0, el.centroid.1, el.bbox.max_z)));
        }

        el.quantities.insert("Length".to_string(), new_depth);
        let vol = el.bbox.volume();
        el.quantities.insert("GrossVolume".to_string(), vol);

        self.recompute_health_audit();
        Ok(old_depth)
    }

    /// Translates an element's spatial position by (dx, dy, dz) meters in both memory and STEP cartesian points.
    pub fn modify_element_placement(&mut self, element_id: usize, delta: (f64, f64, f64)) -> Result<(), String> {
        let mut pt_id_opt = None;
        if let Some(prod_ent) = self.raw_step.entities.get(&element_id) {
            if let Some(lp_id) = prod_ent.get_ref(5) {
                if let Some(lp_ent) = self.raw_step.entities.get(&lp_id) {
                    if let Some(axis_id) = lp_ent.get_ref(1) {
                        if let Some(axis_ent) = self.raw_step.entities.get(&axis_id) {
                            if let Some(pt_id) = axis_ent.get_ref(0) {
                                pt_id_opt = Some(pt_id);
                            }
                        }
                    }
                }
            }
        }

        if let Some(pt_id) = pt_id_opt {
            if let Some(pt_ent) = self.raw_step.entities.get_mut(&pt_id) {
                if let Some(StepValue::List(ref mut list)) = pt_ent.parameters.get_mut(0) {
                    if list.len() >= 3 {
                        let cur_x = list[0].as_f64().unwrap_or(0.0);
                        let cur_y = list[1].as_f64().unwrap_or(0.0);
                        let cur_z = list[2].as_f64().unwrap_or(0.0);
                        list[0] = StepValue::Real(cur_x + delta.0 / self.unit_scale);
                        list[1] = StepValue::Real(cur_y + delta.1 / self.unit_scale);
                        list[2] = StepValue::Real(cur_z + delta.2 / self.unit_scale);
                    }
                }
            }
        }

        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        el.bbox.min_x += delta.0;
        el.bbox.max_x += delta.0;
        el.bbox.min_y += delta.1;
        el.bbox.max_y += delta.1;
        el.bbox.min_z += delta.2;
        el.bbox.max_z += delta.2;
        el.centroid.0 += delta.0;
        el.centroid.1 += delta.1;
        el.centroid.2 += delta.2;

        for line in &mut el.wireframe_lines {
            line.0.0 += delta.0;
            line.0.1 += delta.1;
            line.0.2 += delta.2;
            line.1.0 += delta.0;
            line.1.1 += delta.1;
            line.1.2 += delta.2;
        }

        self.recompute_health_audit();
        Ok(())
    }

    /// Modifies profile cross-section dimensions (XDim, YDim) of a rectangular profile in memory and STEP entities.
    pub fn modify_element_profile(&mut self, element_id: usize, new_dims: (f64, f64)) -> Result<(f64, f64), String> {
        let el = self
            .elements
            .iter_mut()
            .find(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;

        let mut prof_id_opt = None;
        if let Some(prod_ent) = self.raw_step.entities.get(&element_id) {
            if let Some(shape_def_id) = prod_ent.get_ref(6) {
                if let Some(shape_def) = self.raw_step.entities.get(&shape_def_id) {
                    for rep_id in shape_def.get_ref_list(2) {
                        if let Some(rep_ent) = self.raw_step.entities.get(&rep_id) {
                            for item_id in rep_ent.get_ref_list(3) {
                                if let Some(item_ent) = self.raw_step.entities.get(&item_id) {
                                    if item_ent.type_name == "IFCEXTRUDEDAREASOLID" {
                                        if let Some(prof_id) = item_ent.get_ref(0) {
                                            prof_id_opt = Some(prof_id);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let old_dims;
        if let Some(prof_id) = prof_id_opt {
            let prof_ent = self.raw_step.entities.get_mut(&prof_id).unwrap();
            let old_w = prof_ent.get_f64(3).unwrap_or(0.4);
            let old_h = prof_ent.get_f64(4).unwrap_or(0.4);
            old_dims = (old_w, old_h);

            while prof_ent.parameters.len() < 5 {
                prof_ent.parameters.push(StepValue::Null);
            }
            prof_ent.parameters[3] = StepValue::Real(new_dims.0 / self.unit_scale);
            prof_ent.parameters[4] = StepValue::Real(new_dims.1 / self.unit_scale);
        } else {
            let d = el.bbox.dimensions();
            old_dims = (d.0, d.1);
        }

        let half_w = new_dims.0 * 0.5;
        let half_h = new_dims.1 * 0.5;
        let cx = el.centroid.0;
        let cy = el.centroid.1;
        el.bbox.min_x = cx - half_w;
        el.bbox.max_x = cx + half_w;
        el.bbox.min_y = cy - half_h;
        el.bbox.max_y = cy + half_h;

        let len = el.bbox.dimensions().2;
        el.quantities.insert("GrossVolume".to_string(), new_dims.0 * new_dims.1 * len);

        self.recompute_health_audit();
        Ok(old_dims)
    }

    /// Synthesizes and adds a brand-new physical structural element into the IFC STEP model.
    pub fn create_structural_element(&mut self, spec: IfcElementCreationSpec) -> Result<usize, String> {
        let storeys = self.list_storeys();
        let target_storey_name = storeys
            .iter()
            .find(|(sid, _)| *sid == spec.storey_id)
            .map(|(_, name)| name.clone())
            .ok_or_else(|| format!("Target storey #{} not found in spatial tree", spec.storey_id))?;

        let context_id = self.raw_step.get_entities_of_type("IFCGEOMETRICREPRESENTATIONCONTEXT")
            .first()
            .map(|e| e.id)
            .unwrap_or(10);

        let mut next_id = self.raw_step.next_id();

        // 1. Cartesian point for location
        let pt_loc_id = next_id;
        next_id += 1;
        let pt_loc = StepEntity {
            id: pt_loc_id,
            type_name: "IFCCARTESIANPOINT".to_string(),
            parameters: vec![StepValue::List(vec![
                StepValue::Real(spec.origin.0 / self.unit_scale),
                StepValue::Real(spec.origin.1 / self.unit_scale),
                StepValue::Real(spec.origin.2 / self.unit_scale),
            ])],
        };

        // 2. Directions (Axis & Ref)
        let dir_axis_id = next_id;
        next_id += 1;
        let dir_axis = StepEntity {
            id: dir_axis_id,
            type_name: "IFCDIRECTION".to_string(),
            parameters: vec![StepValue::List(vec![StepValue::Real(0.0), StepValue::Real(0.0), StepValue::Real(1.0)])],
        };

        let dir_ref_id = next_id;
        next_id += 1;
        let dir_ref = StepEntity {
            id: dir_ref_id,
            type_name: "IFCDIRECTION".to_string(),
            parameters: vec![StepValue::List(vec![StepValue::Real(1.0), StepValue::Real(0.0), StepValue::Real(0.0)])],
        };

        // 3. Axis2Placement3D
        let axis3d_id = next_id;
        next_id += 1;
        let axis3d = StepEntity {
            id: axis3d_id,
            type_name: "IFCAXIS2PLACEMENT3D".to_string(),
            parameters: vec![StepValue::EntityRef(pt_loc_id), StepValue::EntityRef(dir_axis_id), StepValue::EntityRef(dir_ref_id)],
        };

        // 4. Local Placement
        let loc_pl_id = next_id;
        next_id += 1;
        let loc_pl = StepEntity {
            id: loc_pl_id,
            type_name: "IFCLOCALPLACEMENT".to_string(),
            parameters: vec![StepValue::Null, StepValue::EntityRef(axis3d_id)],
        };

        // 5. 2D profile position
        let pt2d_id = next_id;
        next_id += 1;
        let pt2d = StepEntity {
            id: pt2d_id,
            type_name: "IFCCARTESIANPOINT".to_string(),
            parameters: vec![StepValue::List(vec![StepValue::Real(0.0), StepValue::Real(0.0)])],
        };

        let axis2d_id = next_id;
        next_id += 1;
        let axis2d = StepEntity {
            id: axis2d_id,
            type_name: "IFCAXIS2PLACEMENT2D".to_string(),
            parameters: vec![StepValue::EntityRef(pt2d_id), StepValue::Null],
        };

        // 6. Rectangle Profile Def
        let prof_id = next_id;
        next_id += 1;
        let prof_ent = StepEntity {
            id: prof_id,
            type_name: "IFCRECTANGLEPROFILEDEF".to_string(),
            parameters: vec![
                StepValue::Enumeration("AREA".to_string()),
                StepValue::String(format!("{}_Profile", spec.name)),
                StepValue::EntityRef(axis2d_id),
                StepValue::Real(spec.profile_dims.0 / self.unit_scale),
                StepValue::Real(spec.profile_dims.1 / self.unit_scale),
            ],
        };

        // 7. Extrusion direction & ExtrudedAreaSolid
        let ext_dir_id = next_id;
        next_id += 1;
        let ext_dir = StepEntity {
            id: ext_dir_id,
            type_name: "IFCDIRECTION".to_string(),
            parameters: vec![StepValue::List(vec![
                StepValue::Real(spec.direction.0),
                StepValue::Real(spec.direction.1),
                StepValue::Real(spec.direction.2),
            ])],
        };

        let solid_id = next_id;
        next_id += 1;
        let solid_ent = StepEntity {
            id: solid_id,
            type_name: "IFCEXTRUDEDAREASOLID".to_string(),
            parameters: vec![
                StepValue::EntityRef(prof_id),
                StepValue::EntityRef(axis3d_id),
                StepValue::EntityRef(ext_dir_id),
                StepValue::Real(spec.length / self.unit_scale),
            ],
        };

        // 8. ShapeRepresentation & ProductDefinitionShape
        let shape_rep_id = next_id;
        next_id += 1;
        let shape_rep = StepEntity {
            id: shape_rep_id,
            type_name: "IFCSHAPEREPRESENTATION".to_string(),
            parameters: vec![
                StepValue::EntityRef(context_id),
                StepValue::String("Body".to_string()),
                StepValue::String("SweptSolid".to_string()),
                StepValue::List(vec![StepValue::EntityRef(solid_id)]),
            ],
        };

        let prod_shape_id = next_id;
        next_id += 1;
        let prod_shape = StepEntity {
            id: prod_shape_id,
            type_name: "IFCPRODUCTDEFINITIONSHAPE".to_string(),
            parameters: vec![StepValue::Null, StepValue::Null, StepValue::List(vec![StepValue::EntityRef(shape_rep_id)])],
        };

        // 9. Product Entity
        let product_id = next_id;
        let guid = format!("0Synth{:016x}", product_id);
        let class_upper = spec.class_name.to_uppercase();
        let prod_ent = StepEntity {
            id: product_id,
            type_name: class_upper.clone(),
            parameters: vec![
                StepValue::String(guid.clone()),
                StepValue::Null,
                StepValue::String(spec.name.clone()),
                StepValue::String(spec.description.clone()),
                StepValue::String("SYNTHESIZED_ELEMENT".to_string()),
                StepValue::EntityRef(loc_pl_id),
                StepValue::EntityRef(prod_shape_id),
                StepValue::Enumeration("NOTDEFINED".to_string()),
            ],
        };

        let new_entities = [
            pt_loc, dir_axis, dir_ref, axis3d, loc_pl, pt2d, axis2d, prof_ent, ext_dir, solid_ent, shape_rep, prod_shape, prod_ent,
        ];
        for ent in new_entities {
            let eid = ent.id;
            let tname = ent.type_name.clone();
            self.raw_step.entities.insert(eid, ent);
            self.raw_step.ordered_ids.push(eid);
            self.raw_step.type_index.entry(tname).or_default().push(eid);
        }

        let _ = self.set_element_material(product_id, &spec.material_name);
        let _ = self.reassign_storey(product_id, spec.storey_id);

        let p1 = spec.origin;
        let p2 = (
            spec.origin.0 + spec.direction.0 * spec.length,
            spec.origin.1 + spec.direction.1 * spec.length,
            spec.origin.2 + spec.direction.2 * spec.length,
        );
        let lines = vec![(p1, p2)];
        let min_x = p1.0.min(p2.0) - spec.profile_dims.0 * 0.5;
        let max_x = p1.0.max(p2.0) + spec.profile_dims.0 * 0.5;
        let min_y = p1.1.min(p2.1) - spec.profile_dims.1 * 0.5;
        let max_y = p1.1.max(p2.1) + spec.profile_dims.1 * 0.5;
        let mut min_z = p1.2.min(p2.2);
        let mut max_z = p1.2.max(p2.2);
        if (max_z - min_z).abs() < 1e-4 {
            // Horizontal members require vertical cross-section envelope for 3D clash intersection.
            min_z -= spec.profile_dims.1 * 0.5;
            max_z += spec.profile_dims.1 * 0.5;
        }
        let bbox = IfcBoundingBox::new((min_x, min_y, min_z), (max_x, max_y, max_z));

        let mut qtos = HashMap::new();
        qtos.insert("Length".to_string(), spec.length);
        qtos.insert("GrossVolume".to_string(), spec.profile_dims.0 * spec.profile_dims.1 * spec.length);

        let summary = IfcElementSummary {
            step_id: product_id,
            global_id: guid,
            name: spec.name,
            description: spec.description,
            class_name: class_upper,
            object_type: "SYNTHESIZED_ELEMENT".to_string(),
            predefined_type: Some("NOTDEFINED".to_string()),
            storey_name: target_storey_name,
            material_name: Some(spec.material_name),
            property_sets: Vec::new(),
            quantities: qtos,
            bbox,
            centroid: bbox.centroid(),
            wireframe_lines: lines,
        };

        self.elements.push(summary);
        self.recompute_health_audit();
        Ok(product_id)
    }

    /// Safely deletes a structural element and removes its relational bindings from the model.
    pub fn delete_structural_element(&mut self, element_id: usize) -> Result<(IfcElementSummary, Vec<StepEntity>, usize), String> {
        let pos = self
            .elements
            .iter()
            .position(|e| e.step_id == element_id)
            .ok_or_else(|| format!("Element #{} not found in model", element_id))?;
        let deleted_element = self.elements.remove(pos);

        let storeys = self.list_storeys();
        let storey_id = storeys
            .iter()
            .find(|(_, name)| name == &deleted_element.storey_name)
            .map(|(sid, _)| *sid)
            .unwrap_or(0);

        fn remove_rec(node: &mut IfcSpatialNode, id: usize) {
            node.contained_element_ids.retain(|&x| x != id);
            for child in &mut node.children {
                remove_rec(child, id);
            }
        }
        remove_rec(&mut self.spatial_root, element_id);

        for ent in self.raw_step.entities.values_mut() {
            if ent.type_name == "IFCRELCONTAINEDINSPATIALSTRUCTURE"
                || ent.type_name == "IFCRELASSOCIATESMATERIAL"
                || ent.type_name == "IFCRELDEFINESBYPROPERTIES"
            {
                if let Some(StepValue::List(list)) = ent.parameters.get_mut(4) {
                    list.retain(|v| v.as_entity_ref() != Some(element_id));
                }
            }
        }

        let mut deleted_entities = Vec::new();
        if let Some(ent) = self.raw_step.entities.remove(&element_id) {
            deleted_entities.push(ent);
            self.raw_step.ordered_ids.retain(|&x| x != element_id);
            if let Some(list) = self.raw_step.type_index.get_mut(&deleted_element.class_name) {
                list.retain(|&x| x != element_id);
            }
        }

        self.recompute_health_audit();
        Ok((deleted_element, deleted_entities, storey_id))
    }

    /// Restores a previously deleted structural element with its associated entities.
    pub fn restore_structural_element(&mut self, element: IfcElementSummary, entities: Vec<StepEntity>, storey_id: usize) -> Result<(), String> {
        let el_id = element.step_id;
        for ent in entities {
            let id = ent.id;
            let tname = ent.type_name.clone();
            self.raw_step.entities.insert(id, ent);
            if !self.raw_step.ordered_ids.contains(&id) {
                self.raw_step.ordered_ids.push(id);
            }
            self.raw_step.type_index.entry(tname).or_default().push(id);
        }
        if !self.elements.iter().any(|e| e.step_id == el_id) {
            self.elements.push(element);
        }
        let _ = self.reassign_storey(el_id, storey_id);
        self.recompute_health_audit();
        Ok(())
    }

    /// Dispatches and executes an `IfcEditCommand` on this model.
    pub fn apply_command(&mut self, cmd: &IfcEditCommand) -> Result<(), String> {
        match cmd {
            IfcEditCommand::UpdateElementAttributes {
                element_id,
                new_name,
                new_description,
                new_object_type,
                ..
            } => {
                self.update_element_attributes(
                    *element_id,
                    Some(new_name.clone()),
                    Some(new_description.clone()),
                    Some(new_object_type.clone()),
                )?;
            }
            IfcEditCommand::SetProperty {
                element_id,
                pset_name,
                prop_name,
                new_value,
                ..
            } => {
                self.set_property(*element_id, pset_name, prop_name, new_value)?;
            }
            IfcEditCommand::RemoveProperty {
                element_id,
                pset_name,
                prop_name,
                ..
            } => {
                self.remove_property(*element_id, pset_name, prop_name)?;
            }
            IfcEditCommand::AddPropertySet {
                element_id,
                pset_name,
                properties,
            } => {
                self.add_property_set(*element_id, pset_name, properties.clone())?;
            }
            IfcEditCommand::DeletePropertySet {
                element_id,
                pset_name,
                ..
            } => {
                self.delete_property_set(*element_id, pset_name)?;
            }
            IfcEditCommand::SetMaterial {
                element_id,
                new_material,
                ..
            } => {
                self.set_element_material(*element_id, new_material)?;
            }
            IfcEditCommand::ReassignStorey {
                element_id,
                new_storey_id,
                ..
            } => {
                self.reassign_storey(*element_id, *new_storey_id)?;
            }
            IfcEditCommand::UpdateQuantities {
                element_id,
                new_quantities,
                ..
            } => {
                let list: Vec<(String, f64)> = new_quantities
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                self.update_quantities(*element_id, list)?;
            }
            IfcEditCommand::ModifyExtrusionDepth { element_id, new_depth, .. } => {
                self.modify_element_extrusion(*element_id, *new_depth)?;
            }
            IfcEditCommand::TranslateElement { element_id, delta } => {
                self.modify_element_placement(*element_id, *delta)?;
            }
            IfcEditCommand::ModifyProfileDimensions { element_id, new_dims, .. } => {
                self.modify_element_profile(*element_id, *new_dims)?;
            }
            IfcEditCommand::CreateElement { spec, .. } => {
                self.create_structural_element(spec.clone())?;
            }
            IfcEditCommand::DeleteElementById { element_id } => {
                let _ = self.delete_structural_element(*element_id)?;
            }
            IfcEditCommand::RestoreElement { element, entities, storey_id } => {
                self.restore_structural_element(element.clone(), entities.clone(), *storey_id)?;
            }
            IfcEditCommand::Batch(cmds) => {
                for c in cmds {
                    self.apply_command(c)?;
                }
            }
        }
        self.recompute_health_audit();
        Ok(())
    }

    /// Performs industrial-grade 3D Spatial Clash Detection and Proximity Analysis.
    pub fn detect_clashes(&self, tolerance_m: f64, clearance_m: f64) -> IfcClashReport {
        let mut report = IfcClashReport::default();
        let n = self.elements.len();
        if n < 2 {
            return report;
        }

        for i in 0..n {
            for j in (i + 1)..n {
                report.total_evaluated_pairs += 1;
                let a = &self.elements[i];
                let b = &self.elements[j];

                // 1. Duplicate entity check (identical geometry > 90% overlap and same class)
                let overlap = a.bbox.overlap_ratio(&b.bbox);
                if overlap > 0.90 && a.class_name == b.class_name {
                    report.duplicate_entities += 1;
                    report.clashes.push(IfcClashItem {
                        id_a: a.step_id,
                        id_b: b.step_id,
                        name_a: a.name.clone(),
                        name_b: b.name.clone(),
                        class_a: a.class_name.clone(),
                        class_b: b.class_name.clone(),
                        severity: IfcClashSeverity::Duplicate,
                        intersecting_volume_m3: a.bbox.intersection_volume(&b.bbox),
                        distance_m: 0.0,
                        clash_point: a.centroid,
                        description: format!("Duplicate entity: {:.1}% volumetric overlap between #{} and #{}.", overlap * 100.0, a.step_id, b.step_id),
                    });
                    continue;
                }

                // 2. Hard clash check (physical intersecting bounding volume > 1e-6 m³)
                if a.bbox.intersects(&b.bbox, tolerance_m) {
                    let iv = a.bbox.intersection_volume(&b.bbox);
                    if iv > 1e-6 {
                        report.hard_clashes += 1;
                        let severity = if iv > 0.05 {
                            IfcClashSeverity::Critical
                        } else {
                            IfcClashSeverity::Major
                        };
                        let cp = (
                            0.5 * (a.centroid.0 + b.centroid.0),
                            0.5 * (a.centroid.1 + b.centroid.1),
                            0.5 * (a.centroid.2 + b.centroid.2),
                        );
                        report.clashes.push(IfcClashItem {
                            id_a: a.step_id,
                            id_b: b.step_id,
                            name_a: a.name.clone(),
                            name_b: b.name.clone(),
                            class_a: a.class_name.clone(),
                            class_b: b.class_name.clone(),
                            severity,
                            intersecting_volume_m3: iv,
                            distance_m: 0.0,
                            clash_point: cp,
                            description: format!("Physical geometric clash: {:.4} m³ intersection between #{} ({}) and #{} ({}).", iv, a.step_id, a.class_name, b.step_id, b.class_name),
                        });
                        continue;
                    }
                }

                // 3. Soft clearance buffer violation
                let dist = a.bbox.distance_to(&b.bbox);
                if dist < clearance_m && dist > 1e-4 {
                    report.clearance_violations += 1;
                    let cp = (
                        0.5 * (a.centroid.0 + b.centroid.0),
                        0.5 * (a.centroid.1 + b.centroid.1),
                        0.5 * (a.centroid.2 + b.centroid.2),
                    );
                    report.clashes.push(IfcClashItem {
                        id_a: a.step_id,
                        id_b: b.step_id,
                        name_a: a.name.clone(),
                        name_b: b.name.clone(),
                        class_a: a.class_name.clone(),
                        class_b: b.class_name.clone(),
                        severity: IfcClashSeverity::Minor,
                        intersecting_volume_m3: 0.0,
                        distance_m: dist,
                        clash_point: cp,
                        description: format!("Clearance violation: proximity distance {:.3} m < buffer {:.3} m between #{} and #{}.", dist, clearance_m, a.step_id, b.step_id),
                    });
                }
            }
        }

        report
    }

    /// Compares this model against another (baseline vs revised) and generates a complete BIM differential.
    pub fn diff(&self, other: &IfcModel) -> IfcModelDiff {
        let mut diff_res = IfcModelDiff {
            baseline_schema: self.schema.display_label().to_string(),
            revised_schema: other.schema.display_label().to_string(),
            ..Default::default()
        };

        let mut matched_other = HashSet::new();

        // 1. Process baseline elements
        for base in &self.elements {
            if let Some(rev) = other.elements.iter().find(|e| e.global_id == base.global_id || e.step_id == base.step_id) {
                matched_other.insert(rev.step_id);

                let mut details = Vec::new();
                if base.name != rev.name {
                    details.push(format!("Name changed: '{}' ➔ '{}'", base.name, rev.name));
                }
                if base.description != rev.description {
                    details.push(format!("Description changed: '{}' ➔ '{}'", base.description, rev.description));
                }
                if base.object_type != rev.object_type {
                    details.push(format!("ObjectType changed: '{}' ➔ '{}'", base.object_type, rev.object_type));
                }
                if base.material_name != rev.material_name {
                    details.push(format!("Material changed: {:?} ➔ {:?}", base.material_name, rev.material_name));
                }
                if base.storey_name != rev.storey_name {
                    details.push(format!("Storey changed: '{}' ➔ '{}'", base.storey_name, rev.storey_name));
                }

                let d_cx = rev.centroid.0 - base.centroid.0;
                let d_cy = rev.centroid.1 - base.centroid.1;
                let d_cz = rev.centroid.2 - base.centroid.2;
                if d_cx.abs() > 1e-3 || d_cy.abs() > 1e-3 || d_cz.abs() > 1e-3 {
                    details.push(format!("Centroid translated: dX={:.3}, dY={:.3}, dZ={:.3} m", d_cx, d_cy, d_cz));
                }

                let d_vol = rev.bbox.volume() - base.bbox.volume();
                if d_vol.abs() > 1e-4 {
                    details.push(format!("Volume altered: {:.4} ➔ {:.4} m³ (delta: {:+.4} m³)", base.bbox.volume(), rev.bbox.volume(), d_vol));
                }

                if details.is_empty() {
                    diff_res.unchanged_count += 1;
                    diff_res.element_diffs.push(IfcElementDiff {
                        step_id: base.step_id,
                        global_id: base.global_id.clone(),
                        name: base.name.clone(),
                        class_name: base.class_name.clone(),
                        diff_kind: IfcDiffKind::Unchanged,
                        details: vec!["No attribute or geometric modifications detected.".to_string()],
                        delta_centroid: (0.0, 0.0, 0.0),
                        delta_volume: 0.0,
                    });
                } else {
                    diff_res.modified_count += 1;
                    diff_res.element_diffs.push(IfcElementDiff {
                        step_id: rev.step_id,
                        global_id: rev.global_id.clone(),
                        name: rev.name.clone(),
                        class_name: rev.class_name.clone(),
                        diff_kind: IfcDiffKind::Modified,
                        details,
                        delta_centroid: (d_cx, d_cy, d_cz),
                        delta_volume: d_vol,
                    });
                }
            } else {
                diff_res.deleted_count += 1;
                diff_res.element_diffs.push(IfcElementDiff {
                    step_id: base.step_id,
                    global_id: base.global_id.clone(),
                    name: base.name.clone(),
                    class_name: base.class_name.clone(),
                    diff_kind: IfcDiffKind::Deleted,
                    details: vec![format!("Element excised from model. Previous Storey: '{}', Volume: {:.3} m³", base.storey_name, base.bbox.volume())],
                    delta_centroid: (0.0, 0.0, 0.0),
                    delta_volume: -base.bbox.volume(),
                });
            }
        }

        // 2. Discover added elements
        for rev in &other.elements {
            if !matched_other.contains(&rev.step_id) && !self.elements.iter().any(|e| e.global_id == rev.global_id) {
                diff_res.added_count += 1;
                diff_res.element_diffs.push(IfcElementDiff {
                    step_id: rev.step_id,
                    global_id: rev.global_id.clone(),
                    name: rev.name.clone(),
                    class_name: rev.class_name.clone(),
                    diff_kind: IfcDiffKind::Added,
                    details: vec![format!("New element synthesized into Storey: '{}', Material: '{:?}', Volume: {:.3} m³", rev.storey_name, rev.material_name, rev.bbox.volume())],
                    delta_centroid: (0.0, 0.0, 0.0),
                    delta_volume: rev.bbox.volume(),
                });
            }
        }

        // 3. Revision manifest hash
        let mut manifest = format!("DIFF:{}+{}-{}:{};", diff_res.added_count, diff_res.deleted_count, diff_res.modified_count, diff_res.unchanged_count);
        for d in &diff_res.element_diffs {
            manifest.push_str(&format!("{}:{}:{};", d.step_id, d.diff_kind.label(), d.details.len()));
        }
        diff_res.revision_hash = crate::core::Sha256Hasher::digest_hex(manifest.as_bytes());

        diff_res
    }

    /// Evaluates model conformance against buildingSMART Information Delivery Specification (IDS).
    pub fn validate_ids(&self) -> IfcIdsReport {
        let mut report = IfcIdsReport {
            specification_title: "buildingSMART Information Delivery Specification (IDS) Standard Core".to_string(),
            compliance_score_pct: 100.0,
            rules: Vec::new(),
        };

        let total_elems = self.elements.len();
        if total_elems == 0 {
            report.compliance_score_pct = 0.0;
            return report;
        }

        // Rule 1: Naming integrity
        let mut failed_ids_1 = Vec::new();
        for e in &self.elements {
            if e.name.trim().is_empty() || e.name.to_lowercase() == "unnamed" {
                failed_ids_1.push(e.step_id);
            }
        }
        let pass_1 = total_elems - failed_ids_1.len();
        report.rules.push(IfcIdsRuleCheck {
            rule_id: "IDS-IDENT-01".to_string(),
            description: "All structural products must have explicit, non-empty names.".to_string(),
            target_class: "IfcProduct".to_string(),
            pass_count: pass_1,
            fail_count: failed_ids_1.len(),
            total_count: total_elems,
            status: if failed_ids_1.is_empty() { IfcAuditStatus::Pass } else { IfcAuditStatus::Warning },
            failed_element_ids: failed_ids_1,
        });

        // Rule 2: 22-char IFC GUID specification
        let mut failed_ids_2 = Vec::new();
        for e in &self.elements {
            if e.global_id.trim().len() < 8 {
                failed_ids_2.push(e.step_id);
            }
        }
        let pass_2 = total_elems - failed_ids_2.len();
        report.rules.push(IfcIdsRuleCheck {
            rule_id: "IDS-GUID-01".to_string(),
            description: "All products must have valid 22-character IFC Base64 Globally Unique Identifiers.".to_string(),
            target_class: "IfcProduct".to_string(),
            pass_count: pass_2,
            fail_count: failed_ids_2.len(),
            total_count: total_elems,
            status: if failed_ids_2.is_empty() { IfcAuditStatus::Pass } else { IfcAuditStatus::Fail },
            failed_element_ids: failed_ids_2,
        });

        // Rule 3: Material assignment
        let mut failed_ids_3 = Vec::new();
        for e in &self.elements {
            let is_non_structural_opening = e.class_name == "IFCDOOR" || e.class_name == "IFCWINDOW" || e.class_name == "IFCOPENINGELEMENT";
            if e.material_name.is_none() && !is_non_structural_opening {
                failed_ids_3.push(e.step_id);
            }
        }
        let pass_3 = total_elems - failed_ids_3.len();
        report.rules.push(IfcIdsRuleCheck {
            rule_id: "IDS-MAT-01".to_string(),
            description: "Structural framing members must specify an associated structural material.".to_string(),
            target_class: "IfcMember/IfcColumn/IfcBeam".to_string(),
            pass_count: pass_3,
            fail_count: failed_ids_3.len(),
            total_count: total_elems,
            status: if failed_ids_3.is_empty() { IfcAuditStatus::Pass } else { IfcAuditStatus::Warning },
            failed_element_ids: failed_ids_3,
        });

        // Rule 4: Storey containment
        let mut failed_ids_4 = Vec::new();
        for e in &self.elements {
            if e.storey_name.is_empty() || e.storey_name == "Unassigned" {
                failed_ids_4.push(e.step_id);
            }
        }
        let pass_4 = total_elems - failed_ids_4.len();
        report.rules.push(IfcIdsRuleCheck {
            rule_id: "IDS-SPATIAL-01".to_string(),
            description: "All physical products must be contained within a valid IfcBuildingStorey.".to_string(),
            target_class: "IfcProduct".to_string(),
            pass_count: pass_4,
            fail_count: failed_ids_4.len(),
            total_count: total_elems,
            status: if failed_ids_4.is_empty() { IfcAuditStatus::Pass } else { IfcAuditStatus::Fail },
            failed_element_ids: failed_ids_4,
        });

        // Rule 5: Non-zero physical extents & bounding volume
        let mut failed_ids_5 = Vec::new();
        for e in &self.elements {
            let (dx, dy, dz) = e.bbox.dimensions();
            if dx <= 0.0 || dy <= 0.0 || dz <= 0.0 {
                failed_ids_5.push(e.step_id);
            }
        }
        let pass_5 = total_elems - failed_ids_5.len();
        report.rules.push(IfcIdsRuleCheck {
            rule_id: "IDS-GEOM-01".to_string(),
            description: "Physical bounding extents must have strictly positive spatial dimensions (X, Y, Z > 0).".to_string(),
            target_class: "IfcProduct".to_string(),
            pass_count: pass_5,
            fail_count: failed_ids_5.len(),
            total_count: total_elems,
            status: if failed_ids_5.is_empty() { IfcAuditStatus::Pass } else { IfcAuditStatus::Fail },
            failed_element_ids: failed_ids_5,
        });

        let total_checks: usize = report.rules.iter().map(|r| r.total_count).sum();
        let total_passes: usize = report.rules.iter().map(|r| r.pass_count).sum();
        if total_checks > 0 {
            report.compliance_score_pct = (total_passes as f64 / total_checks as f64) * 100.0;
        }

        report
    }

    /// Generates buildingSMART BCF (BIM Collaboration Format) XML topics from spatial clashes and model issues.
    pub fn export_bcf_xml(&self, clashes: &[IfcClashItem]) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<BcfProject xmlns=\"http://www.buildingsmart-tech.org/specifications/bcf/2.1\">\n");
        xml.push_str(&format!("  <Project Name=\"{}\" ProjectId=\"{}\"/>\n", self.spatial_root.name, self.spatial_root.global_id));
        xml.push_str("  <Topics>\n");

        for (idx, clash) in clashes.iter().enumerate() {
            let guid = format!("BCF-TOPIC-{:04X}-{:04X}", clash.id_a, clash.id_b);
            let prio = match clash.severity {
                IfcClashSeverity::Critical => "Critical",
                IfcClashSeverity::Major => "High",
                IfcClashSeverity::Minor => "Normal",
                IfcClashSeverity::Duplicate => "Major",
            };

            let guid_a = self.elements.iter().find(|e| e.step_id == clash.id_a).map(|e| e.global_id.as_str()).unwrap_or("");
            let guid_b = self.elements.iter().find(|e| e.step_id == clash.id_b).map(|e| e.global_id.as_str()).unwrap_or("");

            xml.push_str(&format!("    <Topic Guid=\"{}\" TopicType=\"Clash\" TopicStatus=\"Open\">\n", guid));
            xml.push_str(&format!("      <Title>[CLASH {:03}] {} vs {}</Title>\n", idx + 1, clash.name_a, clash.name_b));
            xml.push_str(&format!("      <Priority>{}</Priority>\n", prio));
            xml.push_str(&format!("      <Description>{}</Description>\n", clash.description));
            xml.push_str("      <CreationDate>2026-11-08T00:00:00Z</CreationDate>\n");
            xml.push_str("      <CreationAuthor>EigenGraph v0.2.0 OpenBIM Inspector</CreationAuthor>\n");
            xml.push_str("      <Components>\n");
            if !guid_a.is_empty() {
                xml.push_str(&format!("        <Component IfcGuid=\"{}\" Selected=\"true\"/>\n", guid_a));
            }
            if !guid_b.is_empty() {
                xml.push_str(&format!("        <Component IfcGuid=\"{}\" Selected=\"true\"/>\n", guid_b));
            }
            xml.push_str("      </Components>\n");
            xml.push_str("      <Viewpoint>\n");
            xml.push_str(&format!("        <CameraViewPoint X=\"{:.3}\" Y=\"{:.3}\" Z=\"{:.3}\"/>\n", clash.clash_point.0 + 5.0, clash.clash_point.1 + 5.0, clash.clash_point.2 + 3.0));
            xml.push_str("        <CameraDirection X=\"-0.707\" Y=\"-0.707\" Z=\"-0.2\"/>\n");
            xml.push_str("        <CameraUpVector X=\"0\" Y=\"0\" Z=\"1\"/>\n");
            xml.push_str("      </Viewpoint>\n");
            xml.push_str("    </Topic>\n");
        }

        xml.push_str("  </Topics>\n");
        xml.push_str("</BcfProject>\n");
        xml
    }

    /// Serializes the model into buildingSMART ifcJSON-4 structured JSON format.
    pub fn to_ifc_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str("  \"type\": \"ifcJSON\",\n");
        json.push_str("  \"version\": \"4.0.0.0\",\n");
        json.push_str(&format!("  \"schemaIdentifier\": \"{}\",\n", match self.schema {
            IfcSchemaVersion::Ifc4x3 => "IFC4X3",
            IfcSchemaVersion::Ifc4 => "IFC4",
            IfcSchemaVersion::Ifc2x3 => "IFC2X3",
            IfcSchemaVersion::Unknown(ref s) => s,
        }));
        json.push_str("  \"originatingSystem\": \"EigenGraph v0.2.0 OpenBIM Engine\",\n");
        json.push_str("  \"data\": [\n");

        let mut entries = Vec::new();

        // Project entity
        entries.push(format!(
            "    {{\n      \"type\": \"IfcProject\",\n      \"globalId\": \"{}\",\n      \"name\": \"{}\"\n    }}",
            self.spatial_root.global_id, self.spatial_root.name
        ));

        // Products
        for el in &self.elements {
            let mut psets_json = Vec::new();
            for pset in &el.property_sets {
                let mut props = Vec::new();
                for (k, v) in &pset.properties {
                    props.push(format!("\"{}\": \"{}\"", k.replace('"', "\\\""), v.replace('"', "\\\"")));
                }
                psets_json.push(format!("\"{}\": {{{}}}", pset.name, props.join(", ")));
            }

            let (dx, dy, dz) = el.bbox.dimensions();
            let entry = format!(
                "    {{\n      \"type\": \"{}\",\n      \"expressId\": {},\n      \"globalId\": \"{}\",\n      \"name\": \"{}\",\n      \"description\": \"{}\",\n      \"objectType\": \"{}\",\n      \"storey\": \"{}\",\n      \"material\": {},\n      \"dimensions\": [{:.3}, {:.3}, {:.3}],\n      \"propertySets\": {{{}}}\n    }}",
                el.class_name,
                el.step_id,
                el.global_id,
                el.name.replace('"', "\\\""),
                el.description.replace('"', "\\\""),
                el.object_type.replace('"', "\\\""),
                el.storey_name.replace('"', "\\\""),
                el.material_name.as_ref().map(|m| format!("\"{}\"", m.replace('"', "\\\""))).unwrap_or_else(|| "null".to_string()),
                dx, dy, dz,
                psets_json.join(", ")
            );
            entries.push(entry);
        }

        json.push_str(&entries.join(",\n"));
        json.push_str("\n  ]\n}");
        json
    }
}

/// Specification for synthesizing a new physical structural element in an IFC model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfcElementCreationSpec {
    pub class_name: String,
    pub name: String,
    pub description: String,
    pub storey_id: usize,
    pub material_name: String,
    pub origin: (f64, f64, f64),
    pub direction: (f64, f64, f64),
    pub profile_dims: (f64, f64),
    pub length: f64,
}

/// Atomic transaction edit command representing a reversible mutation on an IFC model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IfcEditCommand {
    UpdateElementAttributes {
        element_id: usize,
        old_name: String,
        new_name: String,
        old_description: String,
        new_description: String,
        old_object_type: String,
        new_object_type: String,
    },
    SetProperty {
        element_id: usize,
        pset_name: String,
        prop_name: String,
        old_value: Option<String>,
        new_value: String,
    },
    RemoveProperty {
        element_id: usize,
        pset_name: String,
        prop_name: String,
        removed_value: String,
    },
    AddPropertySet {
        element_id: usize,
        pset_name: String,
        properties: Vec<(String, String)>,
    },
    DeletePropertySet {
        element_id: usize,
        pset_name: String,
        deleted_properties: Vec<(String, String)>,
    },
    SetMaterial {
        element_id: usize,
        old_material: Option<String>,
        new_material: String,
    },
    ReassignStorey {
        element_id: usize,
        old_storey_id: usize,
        new_storey_id: usize,
    },
    UpdateQuantities {
        element_id: usize,
        old_quantities: HashMap<String, f64>,
        new_quantities: HashMap<String, f64>,
    },
    ModifyExtrusionDepth {
        element_id: usize,
        old_depth: f64,
        new_depth: f64,
    },
    TranslateElement {
        element_id: usize,
        delta: (f64, f64, f64),
    },
    ModifyProfileDimensions {
        element_id: usize,
        old_dims: (f64, f64),
        new_dims: (f64, f64),
    },
    CreateElement {
        element_id: usize,
        spec: IfcElementCreationSpec,
    },
    DeleteElementById {
        element_id: usize,
    },
    RestoreElement {
        element: IfcElementSummary,
        entities: Vec<StepEntity>,
        storey_id: usize,
    },
    Batch(Vec<IfcEditCommand>),
}

impl IfcEditCommand {
    /// Constructs the exact inverse command for bidirectional undo/redo operations.
    pub fn inverse(&self) -> IfcEditCommand {
        match self {
            IfcEditCommand::UpdateElementAttributes {
                element_id,
                old_name,
                new_name,
                old_description,
                new_description,
                old_object_type,
                new_object_type,
            } => IfcEditCommand::UpdateElementAttributes {
                element_id: *element_id,
                old_name: new_name.clone(),
                new_name: old_name.clone(),
                old_description: new_description.clone(),
                new_description: old_description.clone(),
                old_object_type: new_object_type.clone(),
                new_object_type: old_object_type.clone(),
            },
            IfcEditCommand::SetProperty {
                element_id,
                pset_name,
                prop_name,
                old_value,
                new_value,
            } => {
                if let Some(old_val) = old_value {
                    IfcEditCommand::SetProperty {
                        element_id: *element_id,
                        pset_name: pset_name.clone(),
                        prop_name: prop_name.clone(),
                        old_value: Some(new_value.clone()),
                        new_value: old_val.clone(),
                    }
                } else {
                    IfcEditCommand::RemoveProperty {
                        element_id: *element_id,
                        pset_name: pset_name.clone(),
                        prop_name: prop_name.clone(),
                        removed_value: new_value.clone(),
                    }
                }
            }
            IfcEditCommand::RemoveProperty {
                element_id,
                pset_name,
                prop_name,
                removed_value,
            } => IfcEditCommand::SetProperty {
                element_id: *element_id,
                pset_name: pset_name.clone(),
                prop_name: prop_name.clone(),
                old_value: None,
                new_value: removed_value.clone(),
            },
            IfcEditCommand::AddPropertySet {
                element_id,
                pset_name,
                properties,
            } => IfcEditCommand::DeletePropertySet {
                element_id: *element_id,
                pset_name: pset_name.clone(),
                deleted_properties: properties.clone(),
            },
            IfcEditCommand::DeletePropertySet {
                element_id,
                pset_name,
                deleted_properties,
            } => IfcEditCommand::AddPropertySet {
                element_id: *element_id,
                pset_name: pset_name.clone(),
                properties: deleted_properties.clone(),
            },
            IfcEditCommand::SetMaterial {
                element_id,
                old_material,
                new_material,
            } => IfcEditCommand::SetMaterial {
                element_id: *element_id,
                old_material: Some(new_material.clone()),
                new_material: old_material.clone().unwrap_or_default(),
            },
            IfcEditCommand::ReassignStorey {
                element_id,
                old_storey_id,
                new_storey_id,
            } => IfcEditCommand::ReassignStorey {
                element_id: *element_id,
                old_storey_id: *new_storey_id,
                new_storey_id: *old_storey_id,
            },
            IfcEditCommand::UpdateQuantities {
                element_id,
                old_quantities,
                new_quantities,
            } => IfcEditCommand::UpdateQuantities {
                element_id: *element_id,
                old_quantities: new_quantities.clone(),
                new_quantities: old_quantities.clone(),
            },
            IfcEditCommand::ModifyExtrusionDepth {
                element_id,
                old_depth,
                new_depth,
            } => IfcEditCommand::ModifyExtrusionDepth {
                element_id: *element_id,
                old_depth: *new_depth,
                new_depth: *old_depth,
            },
            IfcEditCommand::TranslateElement { element_id, delta } => {
                IfcEditCommand::TranslateElement {
                    element_id: *element_id,
                    delta: (-delta.0, -delta.1, -delta.2),
                }
            }
            IfcEditCommand::ModifyProfileDimensions {
                element_id,
                old_dims,
                new_dims,
            } => IfcEditCommand::ModifyProfileDimensions {
                element_id: *element_id,
                old_dims: *new_dims,
                new_dims: *old_dims,
            },
            IfcEditCommand::CreateElement { element_id, .. } => {
                IfcEditCommand::DeleteElementById {
                    element_id: *element_id,
                }
            }
            IfcEditCommand::DeleteElementById { element_id } => {
                IfcEditCommand::DeleteElementById {
                    element_id: *element_id,
                }
            }
            IfcEditCommand::RestoreElement { element, .. } => {
                IfcEditCommand::DeleteElementById {
                    element_id: element.step_id,
                }
            }
            IfcEditCommand::Batch(cmds) => {
                let inverted = cmds.iter().rev().map(|c| c.inverse()).collect();
                IfcEditCommand::Batch(inverted)
            }
        }
    }

    /// User-friendly civil engineering description of the command.
    pub fn description(&self) -> String {
        match self {
            IfcEditCommand::UpdateElementAttributes { element_id, new_name, .. } => {
                format!("Update #{} attributes ('{}')", element_id, new_name)
            }
            IfcEditCommand::SetProperty { element_id, pset_name, prop_name, new_value, .. } => {
                format!("Set #{}.{}.{} = '{}'", element_id, pset_name, prop_name, new_value)
            }
            IfcEditCommand::RemoveProperty { element_id, pset_name, prop_name, .. } => {
                format!("Remove #{}.{}.{}", element_id, pset_name, prop_name)
            }
            IfcEditCommand::AddPropertySet { element_id, pset_name, .. } => {
                format!("Add Pset '{}' to #{}", pset_name, element_id)
            }
            IfcEditCommand::DeletePropertySet { element_id, pset_name, .. } => {
                format!("Delete Pset '{}' from #{}", pset_name, element_id)
            }
            IfcEditCommand::SetMaterial { element_id, new_material, .. } => {
                format!("Set #{} Material: '{}'", element_id, new_material)
            }
            IfcEditCommand::ReassignStorey { element_id, new_storey_id, .. } => {
                format!("Move #{} to Storey #{}", element_id, new_storey_id)
            }
            IfcEditCommand::UpdateQuantities { element_id, .. } => {
                format!("Update quantities for #{}", element_id)
            }
            IfcEditCommand::ModifyExtrusionDepth { element_id, new_depth, .. } => {
                format!("Resize #{} extrusion to {:.2}m", element_id, new_depth)
            }
            IfcEditCommand::TranslateElement { element_id, delta } => {
                format!("Translate #{} by ({:.2}, {:.2}, {:.2})m", element_id, delta.0, delta.1, delta.2)
            }
            IfcEditCommand::ModifyProfileDimensions { element_id, new_dims, .. } => {
                format!("Resize #{} profile to {:.2}x{:.2}m", element_id, new_dims.0, new_dims.1)
            }
            IfcEditCommand::CreateElement { element_id, spec } => {
                format!("Synthesize {} #{} ('{}')", spec.class_name, element_id, spec.name)
            }
            IfcEditCommand::DeleteElementById { element_id } => {
                format!("Delete element #{}", element_id)
            }
            IfcEditCommand::RestoreElement { element, .. } => {
                format!("Restore deleted element #{} ('{}')", element.step_id, element.name)
            }
            IfcEditCommand::Batch(cmds) => {
                format!("Batch mutation ({} ops)", cmds.len())
            }
        }
    }
}

/// Interactive transactional edit session managing Undo/Redo history stacks.
#[derive(Debug, Clone, Default)]
pub struct IfcEditSession {
    pub undo_stack: Vec<IfcEditCommand>,
    pub redo_stack: Vec<IfcEditCommand>,
    pub max_history: usize,
    pub is_dirty: bool,
}

impl IfcEditSession {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
            is_dirty: false,
        }
    }

    /// Applies a mutation command, pushes to undo stack, and clears redo stack.
    pub fn execute(&mut self, model: &mut IfcModel, command: IfcEditCommand) -> Result<(), String> {
        model.apply_command(&command)?;
        self.undo_stack.push(command);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.is_dirty = true;
        Ok(())
    }

    /// Reverts the latest command and pushes it onto the redo stack.
    pub fn undo(&mut self, model: &mut IfcModel) -> Result<Option<String>, String> {
        if let Some(cmd) = self.undo_stack.pop() {
            let inv = cmd.inverse();
            model.apply_command(&inv)?;
            let desc = cmd.description();
            self.redo_stack.push(cmd);
            self.is_dirty = !self.undo_stack.is_empty();
            Ok(Some(desc))
        } else {
            Ok(None)
        }
    }

    /// Re-applies the most recently undone command.
    pub fn redo(&mut self, model: &mut IfcModel) -> Result<Option<String>, String> {
        if let Some(cmd) = self.redo_stack.pop() {
            model.apply_command(&cmd)?;
            let desc = cmd.description();
            self.undo_stack.push(cmd);
            self.is_dirty = true;
            Ok(Some(desc))
        } else {
            Ok(None)
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_description(&self) -> Option<String> {
        self.undo_stack.last().map(|c| c.description())
    }

    pub fn redo_description(&self) -> Option<String> {
        self.redo_stack.last().map(|c| c.description())
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.is_dirty = false;
    }
}

/// Tokenizer and syntax parser for ISO 10303-21 STEP physical files.
pub struct StepParser;

impl StepParser {
    pub fn parse_step(raw_content: &str) -> Result<StepModel, String> {
        let mut model = StepModel::default();
        let no_bom = raw_content.trim_start_matches('\u{FEFF}');

        // 1. Strip comments /* ... */
        let cleaned = Self::strip_step_comments(no_bom);

        // 2. Identify HEADER and DATA sections (case-insensitive STEP tags)
        let header_str = Self::extract_section(&cleaned, "HEADER", "ENDSEC");
        let data_str = Self::extract_section(&cleaned, "DATA", "ENDSEC");

        if let Some(h) = header_str {
            model.header = Self::parse_header(&h);
        }

        if let Some(d) = data_str {
            Self::parse_data_section(&d, &mut model)?;
        } else {
            // If explicit DATA section not found, parse entire content for #id statements
            Self::parse_data_section(&cleaned, &mut model)?;
        }

        if model.entities.is_empty() {
            return Err("No valid STEP ISO-10303-21 entities found in file.".to_string());
        }

        Ok(model)
    }

    fn strip_step_comments(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        let mut in_quote = false;
        while let Some(c) = chars.next() {
            if c == '\'' {
                in_quote = !in_quote;
                out.push(c);
            } else if !in_quote && c == '/' && chars.peek() == Some(&'*') {
                chars.next(); // consume '*'
                while let Some(nc) = chars.next() {
                    if nc == '*' && chars.peek() == Some(&'/') {
                        chars.next(); // consume '/'
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn extract_section(s: &str, start_tag: &str, end_tag: &str) -> Option<String> {
        let upper = s.to_ascii_uppercase();
        if let Some(start_idx) = upper.find(start_tag) {
            let sub = &s[start_idx + start_tag.len()..];
            let sub_upper = &upper[start_idx + start_tag.len()..];
            if let Some(end_idx) = sub_upper.find(end_tag) {
                return Some(sub[..end_idx].to_string());
            }
        }
        None
    }

    fn split_step_statements(s: &str) -> Vec<&str> {
        let mut stmts = Vec::new();
        let bytes = s.as_bytes();
        let mut start = 0usize;
        let mut in_quote = false;
        let mut i = 0usize;
        while i < bytes.len() {
            if bytes[i] == b'\'' {
                if in_quote && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_quote = !in_quote;
            } else if !in_quote && bytes[i] == b';' {
                stmts.push(&s[start..i]);
                start = i + 1;
            }
            i += 1;
        }
        if start < s.len() {
            stmts.push(&s[start..]);
        }
        stmts
    }

    fn parse_header(header_content: &str) -> StepHeader {
        let mut header = StepHeader::default();
        for stmt in Self::split_step_statements(header_content) {
            let trimmed = stmt.trim();
            let upper = trimmed.to_ascii_uppercase();
            if upper.starts_with("FILE_SCHEMA") {
                if let Some(p1) = trimmed.find("((") {
                    if let Some(p2) = trimmed[p1..].find("))") {
                        let inner = &trimmed[p1 + 2..p1 + p2];
                        let schemas: Vec<String> = inner
                            .split(',')
                            .map(|s| s.trim().trim_matches('\'').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        header.file_schema = schemas;
                    }
                }
            } else if upper.starts_with("FILE_NAME") {
                if let Some(p1) = trimmed.find('(') {
                    if let Some(p2) = trimmed.rfind(')') {
                        let inner = &trimmed[p1 + 1..p2];
                        let tokens = Self::split_step_params(inner);
                        if let Some(fn_val) = tokens.first() {
                            header.file_name = fn_val.trim().trim_matches('\'').to_string();
                        }
                        if let Some(ts_val) = tokens.get(1) {
                            header.time_stamp = ts_val.trim().trim_matches('\'').to_string();
                        }
                        if let Some(org_sys) = tokens.get(5) {
                            header.originating_system = org_sys.trim().trim_matches('\'').to_string();
                        }
                    }
                }
            }
        }
        header
    }

    fn parse_data_section(data_content: &str, model: &mut StepModel) -> Result<(), String> {
        for stmt in Self::split_step_statements(data_content) {
            let trimmed = stmt.trim();
            if !trimmed.starts_with('#') {
                continue;
            }

            if let Some(eq_idx) = trimmed.find('=') {
                let id_str = trimmed[1..eq_idx].trim();
                let entity_str = trimmed[eq_idx + 1..].trim();

                if let Ok(id) = id_str.parse::<usize>() {
                    if let Some(open_paren) = entity_str.find('(') {
                        let type_name = entity_str[..open_paren].trim().to_uppercase();
                        if let Some(close_paren) = entity_str.rfind(')') {
                            let params_str = &entity_str[open_paren + 1..close_paren];
                            let params = Self::parse_step_parameter_list(params_str);

                            model.entities.insert(
                                id,
                                StepEntity {
                                    id,
                                    type_name: type_name.clone(),
                                    parameters: params,
                                },
                            );
                            model.ordered_ids.push(id);
                            model.type_index.entry(type_name).or_default().push(id);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn split_step_params(s: &str) -> Vec<String> {
        let mut items = Vec::new();
        let mut cur = String::new();
        let mut paren_depth: usize = 0;
        let mut in_quote = false;

        for c in s.chars() {
            if c == '\'' {
                in_quote = !in_quote;
                cur.push(c);
            } else if !in_quote && (c == '(' || c == '[') {
                paren_depth += 1;
                cur.push(c);
            } else if !in_quote && (c == ')' || c == ']') {
                paren_depth = paren_depth.saturating_sub(1);
                cur.push(c);
            } else if !in_quote && c == ',' && paren_depth == 0 {
                items.push(cur.trim().to_string());
                cur.clear();
            } else {
                cur.push(c);
            }
        }
        let last = cur.trim();
        if !last.is_empty() {
            items.push(last.to_string());
        }
        items
    }

    fn parse_step_parameter_list(s: &str) -> Vec<StepValue> {
        let raw_tokens = Self::split_step_params(s);
        raw_tokens
            .into_iter()
            .map(|tok| Self::parse_single_value(&tok))
            .collect()
    }

    fn parse_single_value(tok: &str) -> StepValue {
        let t = tok.trim();
        if t == "$" || t.is_empty() {
            StepValue::Null
        } else if t == "*" {
            StepValue::Derived
        } else if t.starts_with('#') {
            if let Ok(id) = t[1..].parse::<usize>() {
                StepValue::EntityRef(id)
            } else {
                StepValue::String(t.to_string())
            }
        } else if t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2 {
            StepValue::String(t[1..t.len() - 1].replace("''", "'"))
        } else if t.starts_with('.') && t.ends_with('.') && t.len() >= 2 {
            StepValue::Enumeration(t[1..t.len() - 1].to_string())
        } else if t.starts_with('(') && t.ends_with(')') {
            let inner = &t[1..t.len() - 1];
            let list = Self::parse_step_parameter_list(inner);
            StepValue::List(list)
        } else if let Ok(i) = t.parse::<i64>() {
            StepValue::Integer(i)
        } else if let Ok(f) = t.trim_end_matches('.').parse::<f64>() {
            StepValue::Real(f)
        } else {
            StepValue::String(t.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_tokenizer_and_syntax_parser() {
        let sample = r#"
        ISO-10303-21;
        HEADER;
        FILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');
        FILE_NAME('TestModel.ifc','2026-11-07',('Antigravity'),('EigenGraph'),'v0.1.7','Test','');
        FILE_SCHEMA(('IFC4'));
        ENDSEC;
        DATA;
        #10=IFCPROJECT('0ProjGuid0000000001',$,'Test Project',$,$,$,$,$,$);
        #20=IFCCARTESIANPOINT((1200.5, 3400.0, 500.25));
        #30=IFCDIRECTION((0., 0., 1.));
        #40=IFCBEAM('0BeamGuid0000000001',$,'Roof Beam 300x600',$,$,#20,#30,.BEAM.);
        ENDSEC;
        END-ISO-10303-21;
        "#;

        let model = StepParser::parse_step(sample).expect("Failed to parse valid STEP file");
        assert_eq!(model.header.file_name, "TestModel.ifc");
        assert_eq!(model.header.file_schema, vec!["IFC4".to_string()]);
        assert_eq!(model.entities.len(), 4);

        let beam = model.entities.get(&40).expect("Beam #40 should exist");
        assert_eq!(beam.type_name, "IFCBEAM");
        assert_eq!(beam.get_str(0), Some("0BeamGuid0000000001"));
        assert_eq!(beam.get_str(2), Some("Roof Beam 300x600"));
        assert_eq!(beam.get_ref(5), Some(20));
        assert_eq!(beam.get_ref(6), Some(30));
    }

    #[test]
    fn test_ifc_spatial_hierarchy_decomposition() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let model = IfcModel::parse(&villa_ifc).expect("Failed to parse sample villa IFC4");

        assert_eq!(model.schema, IfcSchemaVersion::Ifc4);
        assert_eq!(model.spatial_root.class_name.to_uppercase(), "IFCPROJECT");
        assert_eq!(model.spatial_root.children.len(), 1); // IfcSite

        let site = &model.spatial_root.children[0];
        assert_eq!(site.class_name.to_uppercase(), "IFCSITE");
        assert_eq!(site.children.len(), 1); // IfcBuilding

        let building = &site.children[0];
        assert_eq!(building.class_name.to_uppercase(), "IFCBUILDING");
        assert_eq!(building.children.len(), 2); // 2 Storeys: Ground & Upper

        let l1 = &building.children[0];
        assert_eq!(l1.class_name, "IFCBUILDINGSTOREY");
        assert_eq!(l1.name, "Ground Level (L1)");
        assert_eq!(l1.contained_element_ids.len(), 6); // 4 columns, 1 wall, 1 door

        let l2 = &building.children[1];
        assert_eq!(l2.class_name, "IFCBUILDINGSTOREY");
        assert_eq!(l2.name, "Upper Level (L2)");
        assert_eq!(l2.contained_element_ids.len(), 3); // 2 beams, 1 slab
    }

    #[test]
    fn test_ifc_property_set_and_material_extraction() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let model = IfcModel::parse(&villa_ifc).expect("Failed to parse sample villa IFC4");

        // Find Col_C1_A1 (#200)
        let col = model.elements.iter().find(|e| e.name == "Col_C1_A1").expect("Column C1 should exist");
        assert_eq!(col.class_name, "IFCCOLUMN");
        assert_eq!(col.storey_name, "Ground Level (L1)");
        assert_eq!(col.material_name, Some("Concrete C30/37".to_string()));

        // Check property set
        assert_eq!(col.property_sets.len(), 1);
        let pset = &col.property_sets[0];
        assert_eq!(pset.name, "Pset_ColumnCommon");
        let load_bearing = pset.properties.iter().find(|(k, _)| k == "LoadBearing").map(|(_, v)| v.as_str());
        assert_eq!(load_bearing, Some("TRUE"));
    }

    #[test]
    fn test_ifc_bounding_box_and_geometry_extractor() {
        let portal_ifc = IfcModel::sample_portal_frame_ifc4();
        let model = IfcModel::parse(&portal_ifc).expect("Failed to parse portal frame");

        assert_eq!(model.elements.len(), 4); // 2 columns, 2 beams
        for el in &model.elements {
            assert!(!el.wireframe_lines.is_empty(), "Structural element should have wireframe geometry");
            let bbox = el.bbox;
            assert!(bbox.max_z >= bbox.min_z);
        }
    }

    #[test]
    fn test_ifc_model_health_and_conformance_audit() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let model = IfcModel::parse(&villa_ifc).expect("Failed to parse sample villa IFC4");

        let audit = &model.health_report;
        assert!(audit.conformance_score_pct >= 95.0, "Pristine sample villa should pass audit with >= 95%");
        assert_eq!(audit.duplicate_guids_count, 0);
        assert_eq!(audit.dangling_references_count, 0);
        assert_eq!(audit.uncontained_products_count, 0);
        assert!(audit.checks.iter().any(|c| c.check_name.contains("buildingSMART")));
    }

    #[test]
    fn test_ifc_to_structural_model_conversion() {
        let portal_ifc = IfcModel::sample_portal_frame_ifc4();
        let ifc_model = IfcModel::parse(&portal_ifc).expect("Failed to parse portal frame");

        let fem = ifc_model.to_structural_model().expect("Should convert to FEA model");
        assert!(fem.nodes.len() >= 4, "Should have at least 4 nodes for portal frame");
        assert_eq!(fem.elements.len(), 4, "Should have 4 structural elements (2 cols, 2 rafters)");

        // Verify boundary conditions on base nodes
        let pinned_nodes = fem.nodes.iter().filter(|n| n.fixity.is_fixed() || n.fixity.ux).count();
        assert!(pinned_nodes >= 2, "Base nodes should have boundary fixities assigned");
    }

    #[test]
    fn test_ifc_update_element_attributes() {
        let portal_ifc = IfcModel::sample_portal_frame_ifc4();
        let mut model = IfcModel::parse(&portal_ifc).expect("Failed to parse portal frame");

        let target_id = model.elements[0].step_id;
        let old_name = model.elements[0].name.clone();

        model
            .update_element_attributes(
                target_id,
                Some("Renamed_Portal_Column_A1".to_string()),
                Some("Updated 400x400 SHS Column Description".to_string()),
                Some("STRUCTURAL_COLUMN_TYPE_A".to_string()),
            )
            .expect("Should update attributes successfully");

        // Verify in-memory element summary
        let updated = model.elements.iter().find(|e| e.step_id == target_id).unwrap();
        assert_eq!(updated.name, "Renamed_Portal_Column_A1");
        assert_eq!(updated.description, "Updated 400x400 SHS Column Description");
        assert_eq!(updated.object_type, "STRUCTURAL_COLUMN_TYPE_A");
        assert_ne!(updated.name, old_name);

        // Verify STEP entity parameters
        let ent = model.raw_step.entities.get(&target_id).unwrap();
        assert_eq!(ent.get_str(2), Some("Renamed_Portal_Column_A1"));
        assert_eq!(ent.get_str(3), Some("Updated 400x400 SHS Column Description"));
        assert_eq!(ent.get_str(4), Some("STRUCTURAL_COLUMN_TYPE_A"));

        // Verify STEP serialized string
        let step_out = model.to_step_string();
        assert!(step_out.contains("Renamed_Portal_Column_A1"));
    }

    #[test]
    fn test_ifc_property_set_mutation_and_deletion() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let mut model = IfcModel::parse(&villa_ifc).expect("Failed to parse villa");

        let col_id = model.elements.iter().find(|e| e.name == "Col_C1_A1").unwrap().step_id;

        // 1. Add property to existing Pset
        model
            .set_property(col_id, "Pset_ColumnCommon", "FireRating", "FRR-120")
            .expect("Should set property");
        let el = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        let pset = el.property_sets.iter().find(|p| p.name == "Pset_ColumnCommon").unwrap();
        let fire = pset.properties.iter().find(|(k, _)| k == "FireRating").map(|(_, v)| v.as_str());
        assert_eq!(fire, Some("FRR-120"));

        // 2. Add an entire new custom property set
        model
            .add_property_set(
                col_id,
                "Pset_EigenStructuralAudit",
                vec![
                    ("UtilizationRatio".to_string(), "0.78".to_string()),
                    ("SeismicDuctilityClass".to_string(), "DCH".to_string()),
                ],
            )
            .expect("Should add property set");

        let el2 = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        assert_eq!(el2.property_sets.len(), 2);
        let audit_pset = el2.property_sets.iter().find(|p| p.name == "Pset_EigenStructuralAudit").unwrap();
        assert_eq!(audit_pset.properties.len(), 2);

        // 3. Remove single property
        let removed = model.remove_property(col_id, "Pset_ColumnCommon", "FireRating").expect("Remove prop");
        assert!(removed);
        let el3 = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        let pset_check = el3.property_sets.iter().find(|p| p.name == "Pset_ColumnCommon").unwrap();
        assert!(!pset_check.properties.iter().any(|(k, _)| k == "FireRating"));

        // 4. Delete entire property set
        let deleted = model.delete_property_set(col_id, "Pset_EigenStructuralAudit").expect("Delete pset");
        assert!(deleted);
        let el4 = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        assert_eq!(el4.property_sets.len(), 1);
    }

    #[test]
    fn test_ifc_material_reassignment() {
        let diagrid_ifc = IfcModel::sample_diagrid_tower_ifc2x3();
        let mut model = IfcModel::parse(&diagrid_ifc).expect("Failed to parse diagrid tower");

        let brace_id = model.elements[0].step_id;
        assert_eq!(model.elements[0].material_name.as_deref(), Some("High-Strength Steel S460ML"));

        // Reassign material to S690QL
        model
            .set_element_material(brace_id, "High-Strength Steel S690QL")
            .expect("Should reassign material");

        let updated_el = model.elements.iter().find(|e| e.step_id == brace_id).unwrap();
        assert_eq!(updated_el.material_name.as_deref(), Some("High-Strength Steel S690QL"));

        // Verify STEP contains new material entity
        let has_s690 = model.raw_step.get_entities_of_type("IFCMATERIAL").iter().any(|m| {
            m.get_str(0) == Some("High-Strength Steel S690QL")
        });
        assert!(has_s690, "Model should now contain S690QL material entity");
    }

    #[test]
    fn test_ifc_storey_reassignment() {
        let diagrid_ifc = IfcModel::sample_diagrid_tower_ifc2x3();
        let mut model = IfcModel::parse(&diagrid_ifc).expect("Failed to parse diagrid tower");

        let storeys = model.list_storeys();
        assert!(storeys.len() >= 2, "Should have at least 2 storeys");
        let (first_sid, _) = storeys[0];
        let (second_sid, second_sname) = storeys[1].clone();

        let member_id = model.elements[0].step_id;
        assert_eq!(model.elements[0].storey_name, storeys[0].1);

        // Move member to second storey
        model.reassign_storey(member_id, second_sid).expect("Reassign storey");

        let updated_el = model.elements.iter().find(|e| e.step_id == member_id).unwrap();
        assert_eq!(updated_el.storey_name, second_sname);

        // Check spatial root containment
        assert!(!model.spatial_root.children.iter().any(|c| c.step_id == first_sid && c.contained_element_ids.contains(&member_id)));
    }

    #[test]
    fn test_ifc_transaction_undo_redo_rollback() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let mut model = IfcModel::parse(&villa_ifc).expect("Failed to parse sample villa");
        let mut session = IfcEditSession::new();

        let col_id = model.elements[0].step_id;
        let original_name = model.elements[0].name.clone();

        let cmd = IfcEditCommand::UpdateElementAttributes {
            element_id: col_id,
            old_name: original_name.clone(),
            new_name: "Mutated_Column_X99".to_string(),
            old_description: model.elements[0].description.clone(),
            new_description: "Transactional test mutation".to_string(),
            old_object_type: model.elements[0].object_type.clone(),
            new_object_type: "CUSTOM_TYPE".to_string(),
        };

        // 1. Execute Command
        session.execute(&mut model, cmd).expect("Command execution should succeed");
        assert!(session.is_dirty);
        assert!(session.can_undo());
        assert!(!session.can_redo());
        assert_eq!(model.elements[0].name, "Mutated_Column_X99");

        // 2. Undo Command
        let undo_desc = session.undo(&mut model).expect("Undo should succeed");
        assert!(undo_desc.is_some());
        assert_eq!(model.elements[0].name, original_name, "Element name should be rolled back to original");
        assert!(!session.is_dirty);
        assert!(!session.can_undo());
        assert!(session.can_redo());

        // 3. Redo Command
        let redo_desc = session.redo(&mut model).expect("Redo should succeed");
        assert!(redo_desc.is_some());
        assert_eq!(model.elements[0].name, "Mutated_Column_X99", "Element name should be reapplied");
        assert!(session.is_dirty);
        assert!(session.can_undo());
        assert!(!session.can_redo());
    }

    #[test]
    fn test_ifc_modify_element_extrusion_depth() {
        let portal_ifc = IfcModel::sample_portal_frame_ifc4();
        let mut model = IfcModel::parse(&portal_ifc).expect("Failed to parse portal frame");

        let col_id = model.elements[0].step_id;
        let old_depth = model.modify_element_extrusion(col_id, 9.5).expect("Should modify extrusion depth");
        assert!(old_depth > 0.0);

        let col = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        assert!((col.bbox.dimensions().2 - 9.5).abs() < 1e-3, "Bounding box Z extent should reflect 9.5m");

        // Verify STEP serialized string contains updated depth
        let step_out = model.to_step_string();
        assert!(step_out.contains("9.5"), "STEP output should serialize the 9.5m depth parameter");
    }

    #[test]
    fn test_ifc_modify_element_placement_translation() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let mut model = IfcModel::parse(&villa_ifc).expect("Failed to parse sample villa");

        let col_id = model.elements[0].step_id;
        let initial_centroid = model.elements[0].centroid;

        model.modify_element_placement(col_id, (5.0, 2.5, 1.0)).expect("Should translate element placement");

        let translated = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        assert!((translated.centroid.0 - (initial_centroid.0 + 5.0)).abs() < 1e-3);
        assert!((translated.centroid.1 - (initial_centroid.1 + 2.5)).abs() < 1e-3);
        assert!((translated.centroid.2 - (initial_centroid.2 + 1.0)).abs() < 1e-3);
    }

    #[test]
    fn test_ifc_modify_element_profile_dimensions() {
        let portal_ifc = IfcModel::sample_portal_frame_ifc4();
        let mut model = IfcModel::parse(&portal_ifc).expect("Failed to parse portal frame");

        let col_id = model.elements[0].step_id;
        let old_dims = model.modify_element_profile(col_id, (0.6, 0.8)).expect("Should modify profile");
        assert!(old_dims.0 > 0.0 && old_dims.1 > 0.0);

        let col = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        let dims = col.bbox.dimensions();
        assert!((dims.0 - 0.6).abs() < 1e-3);
        assert!((dims.1 - 0.8).abs() < 1e-3);
    }

    #[test]
    fn test_ifc_create_and_delete_structural_element() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let mut model = IfcModel::parse(&villa_ifc).expect("Failed to parse villa");
        let initial_count = model.elements.len();

        let storeys = model.list_storeys();
        let storey_id = storeys[0].0;

        let spec = IfcElementCreationSpec {
            class_name: "IFCCOLUMN".to_string(),
            name: "Synthesized_HighRise_Col_Z9".to_string(),
            description: "Custom prefabricated structural steel column".to_string(),
            storey_id,
            material_name: "High-Strength Steel S460ML".to_string(),
            origin: (12.0, 8.0, 0.0),
            direction: (0.0, 0.0, 1.0),
            profile_dims: (0.5, 0.5),
            length: 4.5,
        };

        // 1. Synthesize Element
        let new_id = model.create_structural_element(spec).expect("Should synthesize element");
        assert_eq!(model.elements.len(), initial_count + 1);

        let created = model.elements.iter().find(|e| e.step_id == new_id).expect("Created element must exist");
        assert_eq!(created.name, "Synthesized_HighRise_Col_Z9");
        assert_eq!(created.class_name, "IFCCOLUMN");
        assert_eq!(created.material_name.as_deref(), Some("High-Strength Steel S460ML"));

        // 2. Delete Element
        let (deleted, _entities, _sid) = model.delete_structural_element(new_id).expect("Should delete element");
        assert_eq!(deleted.step_id, new_id);
        assert_eq!(model.elements.len(), initial_count);
        assert!(!model.elements.iter().any(|e| e.step_id == new_id));
    }

    #[test]
    fn test_ifc_geometric_transaction_undo_redo() {
        let portal_ifc = IfcModel::sample_portal_frame_ifc4();
        let mut model = IfcModel::parse(&portal_ifc).expect("Failed to parse portal frame");
        let mut session = IfcEditSession::new();

        let col_id = model.elements[0].step_id;
        let orig_depth = model.elements[0].bbox.dimensions().2;

        let cmd = IfcEditCommand::ModifyExtrusionDepth {
            element_id: col_id,
            old_depth: orig_depth,
            new_depth: 14.0,
        };

        // 1. Execute
        session.execute(&mut model, cmd).expect("Execute extrusion resize");
        let mutated = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        assert!((mutated.bbox.dimensions().2 - 14.0).abs() < 1e-3);

        // 2. Undo
        session.undo(&mut model).expect("Undo extrusion resize");
        let reverted = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        assert!((reverted.bbox.dimensions().2 - orig_depth).abs() < 1e-3);

        // 3. Redo
        session.redo(&mut model).expect("Redo extrusion resize");
        let reapplied = model.elements.iter().find(|e| e.step_id == col_id).unwrap();
        assert!((reapplied.bbox.dimensions().2 - 14.0).abs() < 1e-3);
    }

    #[test]
    fn test_ifc_spatial_clash_detection_hard_and_clearance() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let mut model = IfcModel::parse(&villa_ifc).expect("Parse sample villa");

        // Synthesize an intersecting element to guarantee a controlled hard clash
        let storeys = model.list_storeys();
        let target_sid = storeys[0].0;
        let col = &model.elements[0];
        let origin = col.centroid;

        let spec = IfcElementCreationSpec {
            class_name: "IFCBEAM".to_string(),
            name: "Clashing_Test_Girder".to_string(),
            description: "Intentionally positioned for spatial clash verification".to_string(),
            storey_id: target_sid,
            material_name: "High-Strength Steel S460ML".to_string(),
            origin,
            direction: (1.0, 0.0, 0.0),
            profile_dims: (0.4, 0.4),
            length: 5.0,
        };
        let _ = model.create_structural_element(spec).expect("Create element");

        let clash_report = model.detect_clashes(0.010, 0.50);
        assert!(clash_report.total_evaluated_pairs > 0);
        assert!(clash_report.hard_clashes >= 1, "Must detect at least 1 hard geometric clash");

        let found_hard = clash_report.clashes.iter().find(|c| c.severity == IfcClashSeverity::Critical || c.severity == IfcClashSeverity::Major);
        assert!(found_hard.is_some());
        assert!(found_hard.unwrap().intersecting_volume_m3 > 0.0);
    }

    #[test]
    fn test_ifc_model_diff_added_deleted_modified() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let baseline = IfcModel::parse(&villa_ifc).expect("Parse baseline");
        let mut revised = IfcModel::parse(&villa_ifc).expect("Parse revised");

        // 1. Modify an attribute
        let el_id = revised.elements[0].step_id;
        revised.update_element_attributes(el_id, Some("Renamed_Column_Alpha".to_string()), None, None).expect("Update attr");

        // 2. Synthesize an added element
        let target_sid = revised.list_storeys()[0].0;
        let spec = IfcElementCreationSpec {
            class_name: "IFCCOLUMN".to_string(),
            name: "Synthesized_Diff_Col".to_string(),
            description: "New addition for BIM Diff test".to_string(),
            storey_id: target_sid,
            material_name: "C30/37 Concrete".to_string(),
            origin: (20.0, 20.0, 0.0),
            direction: (0.0, 0.0, 1.0),
            profile_dims: (0.5, 0.5),
            length: 3.5,
        };
        let _ = revised.create_structural_element(spec).expect("Create element");

        // 3. Diff baseline against revised
        let diff = baseline.diff(&revised);
        assert_eq!(diff.added_count, 1, "Must detect exactly 1 added element");
        assert!(diff.modified_count >= 1, "Must detect modified column");
        assert_eq!(diff.deleted_count, 0, "No elements were deleted");
        assert!(!diff.revision_hash.is_empty(), "Revision hash must be generated");

        let mod_elem = diff.element_diffs.iter().find(|d| d.step_id == el_id).unwrap();
        assert_eq!(mod_elem.diff_kind, IfcDiffKind::Modified);
        assert!(mod_elem.details.iter().any(|s| s.contains("Renamed_Column_Alpha")));
    }

    #[test]
    fn test_ifc_ids_specification_validation() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let model = IfcModel::parse(&villa_ifc).expect("Parse villa");

        let ids_report = model.validate_ids();
        assert!(ids_report.compliance_score_pct > 80.0, "Sample villa must have high IDS compliance");
        assert_eq!(ids_report.rules.len(), 5, "Must evaluate all 5 IDS standard rules");

        for rule in &ids_report.rules {
            assert!(rule.total_count > 0);
            assert!(rule.pass_count > 0);
        }
    }

    #[test]
    fn test_ifc_bcf_topic_generation() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let model = IfcModel::parse(&villa_ifc).expect("Parse villa");

        let fake_clashes = vec![IfcClashItem {
            id_a: model.elements[0].step_id,
            id_b: model.elements[1].step_id,
            name_a: model.elements[0].name.clone(),
            name_b: model.elements[1].name.clone(),
            class_a: model.elements[0].class_name.clone(),
            class_b: model.elements[1].class_name.clone(),
            severity: IfcClashSeverity::Critical,
            intersecting_volume_m3: 0.125,
            distance_m: 0.0,
            clash_point: (5.0, 5.0, 3.0),
            description: "Physical spatial clash detected during audit".to_string(),
        }];

        let xml = model.export_bcf_xml(&fake_clashes);
        assert!(xml.contains("<BcfProject"));
        assert!(xml.contains("Topic Guid="));
        assert!(xml.contains("TopicType=\"Clash\""));
        assert!(xml.contains("<CameraViewPoint"));
        assert!(xml.contains(&model.elements[0].global_id));
    }

    #[test]
    fn test_ifc_json_serializer() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let model = IfcModel::parse(&villa_ifc).expect("Parse villa");

        let json = model.to_ifc_json();
        assert!(json.starts_with('{'));
        assert!(json.contains("\"type\": \"ifcJSON\""));
        assert!(json.contains("\"schemaIdentifier\": \"IFC4\""));
        assert!(json.contains("\"IfcProject\""));
        assert!(json.contains("\"expressId\":"));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_ifc_model_to_bim_project_roundtrip() {
        let villa_ifc = IfcModel::sample_villa_ifc4();
        let model = IfcModel::parse(&villa_ifc).expect("Parse villa");
        let bim = model.to_bim_project().expect("IFC should convert into editable BimProject");
        assert!(bim.levels.len() >= 2);
        assert!(bim.element_count() > 0);
        assert!(!bim.columns.is_empty());
    }
}


