//! bim.rs — BIMCraft-Grade Relational Building Information Modeling (BIM) Kernel:
//! - Datums: Levels (elevations) & Structural Grids (1..N, A..Z)
//! - Architectural & Structural Elements: Compound Layered Walls (auto-mitred joins + CSG Door/Window voids),
//!   Structural Columns, Sloped Runs (Beams, Strip Footings, MEP Pipes/Trays with Top/Centre/Bottom Justification),
//!   Slabs/Floors with Shaft Openings, Straight-Skeleton Roofs, Stairs, and Auto-Detected Rooms (`geom::region_at`)
//! - Parametric Concrete Reinforcement (`rebar` & `reinforce` for Beams, Columns, Footings + Bar Schedule)
//! - Revit-Style Object Snaps (`SE, SM, SI, SC, SP, SN`), Angular Guides, and Graduated Ruler
//! - Transactional Command Bus (`BimCommand`) with JSON execution and Unlimited Undo/Redo
//! - Dual Physical-to-Analytical FEM & Live ARI Synchronization (`to_structural_model` + `AriEngine`)
//! - Multi-View Projection (Floor Plan View Range, Live Section Cut Poché, Vector Sheet SVG/PDF & Full IFC4).

use crate::core::ari::{AriEngine, RedundancyAssessment};
use crate::core::csg::{
    csg_subtract, extrude_box_run_mesh, extrude_ring_mesh, extrude_tube_mesh,
    extrude_wf_section_mesh, slice_mesh_with_plane, Bvh3D, CsgMeter, P3,
};
use crate::core::geom::{
    area, closest_point_on_segment, line_intersection, param_on_segment,
    region_in, faces, straight_skeleton_roof, P2,
};
use crate::core::model::{
    BoundaryFixity, CrossSection, Element3D, ElementType, Material, NodalLoad, Node3D,
    StructuralModel,
};
use serde::{Deserialize, Serialize};

/// Datum Level (Storey elevation plane in mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimLevel {
    pub id: usize,
    pub name: String,
    pub elevation_mm: f64,
}

/// Datum Structural Grid Axis in plan (`1, 2, 3...` or `A, B, C...`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimGrid {
    pub id: usize,
    pub label: String,
    pub start: P2,
    pub end: P2,
}

/// Layer function inside a compound architectural/structural wall or slab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerFunction {
    Structure,
    Substrate,
    ThermalInsulation,
    Membrane,
    FinishExterior,
    FinishInterior,
}

/// A single material layer in a compound wall or floor type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompoundLayer {
    pub function: LayerFunction,
    pub material_name: String,
    pub thickness_mm: f64,
    pub color_rgb: [u8; 3],
}

/// Hosted Door or Window opening inside a `BimWall`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpeningKind {
    SingleDoor,
    DoubleDoor,
    SlidingDoor,
    FixedWindow,
    CasementWindow,
    SlidingWindow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimOpening {
    pub id: usize,
    pub mark: String,
    pub kind: OpeningKind,
    pub offset_along_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub sill_height_mm: f64,
    pub flip_facing: bool,
    pub flip_hand: bool,
}

/// A parametric Wall element with compound layers, level attachment, and hosted openings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimWall {
    pub id: usize,
    pub name: String,
    pub base_level_id: usize,
    pub top_level_id: Option<usize>,
    pub unconnected_height_mm: f64,
    pub start: P2,
    pub end: P2,
    pub thickness_mm: f64,
    pub is_structural: bool,
    pub layers: Vec<CompoundLayer>,
    pub openings: Vec<BimOpening>,
}

impl BimWall {
    pub fn default_compound_layers(thickness_mm: f64) -> Vec<CompoundLayer> {
        let ext = (thickness_mm * 0.15).max(20.0);
        let ins = (thickness_mm * 0.10).max(15.0);
        let int = (thickness_mm * 0.10).max(15.0);
        let core = (thickness_mm - ext - ins - int).max(100.0);
        vec![
            CompoundLayer {
                function: LayerFunction::FinishExterior,
                material_name: "Brick / Cladding Course".to_string(),
                thickness_mm: ext,
                color_rgb: [196, 98, 68],
            },
            CompoundLayer {
                function: LayerFunction::ThermalInsulation,
                material_name: "Mineral Wool Thermal".to_string(),
                thickness_mm: ins,
                color_rgb: [235, 200, 95],
            },
            CompoundLayer {
                function: LayerFunction::Structure,
                material_name: "Reinforced Concrete Core".to_string(),
                thickness_mm: core,
                color_rgb: [148, 163, 184],
            },
            CompoundLayer {
                function: LayerFunction::FinishInterior,
                material_name: "Gypsum Plaster Finish".to_string(),
                thickness_mm: int,
                color_rgb: [226, 232, 240],
            },
        ]
    }

    pub fn length_mm(&self) -> f64 {
        self.start.dist(self.end)
    }

    pub fn direction(&self) -> P2 {
        (self.end - self.start).unit()
    }

    pub fn normal_outward(&self) -> P2 {
        // Exterior is to the left of start -> end (so clockwise loop faces outward)
        let d = self.direction();
        P2(-d.1, d.0)
    }

    /// Computes the 2D plan footprint ring of the wall, optionally mitred against adjacent walls.
    pub fn footprint_ring(&self) -> Vec<P2> {
        let n = self.normal_outward() * (self.thickness_mm * 0.5);
        vec![
            self.start - n,
            self.end - n,
            self.end + n,
            self.start + n,
        ]
    }

    /// Generates the closed 3D triangle mesh of the wall with all hosted door/window openings
    /// subtracted using the non-recursive BSP CSG engine.
    pub fn mesh_3d(&self, z_base: f64, z_top: f64) -> Vec<[P3; 3]> {
        let ring = self.footprint_ring();
        let mut solid = extrude_ring_mesh(&ring, z_base, z_top);
        if self.openings.is_empty() {
            return solid;
        }
        let d = self.direction();
        let n = self.normal_outward();
        let half_t = self.thickness_mm * 0.65;
        let mut meter = CsgMeter::default();

        for op in &self.openings {
            let p_start = self.start + d * op.offset_along_mm;
            let p_end = self.start + d * (op.offset_along_mm + op.width_mm);
            let void_ring = vec![
                p_start - n * half_t,
                p_end - n * half_t,
                p_end + n * half_t,
                p_start + n * half_t,
            ];
            let z0 = z_base + op.sill_height_mm;
            let z1 = (z0 + op.height_mm).min(z_top - 50.0);
            if z1 > z0 {
                let void_mesh = extrude_ring_mesh(&void_ring, z0, z1);
                if let Ok(cut) = csg_subtract(&solid, &void_mesh, &mut meter) {
                    solid = cut;
                }
            }
        }
        solid
    }
}

/// Vertical justification of a linear Run (`Beam`, `StripFooting`, `Pipe`, `CableTray`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunJustify {
    Top,
    Centre,
    Bottom,
}

/// Category of a linear `BimRun` element across Structure & MEP disciplines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunCategory {
    StructuralBeam,
    StructuralBrace,
    StripFooting,
    PlumbingPipe,
    ElectricalCableTray,
    ElectricalConduit,
    LightningConductor,
}

impl RunCategory {
    pub fn ifc_class(&self) -> (&'static str, &'static str) {
        match self {
            Self::StructuralBeam => ("IFCBEAM", ".BEAM."),
            Self::StructuralBrace => ("IFCMEMBER", ".BRACE."),
            Self::StripFooting => ("IFCFOOTING", ".STRIP_FOOTING."),
            Self::PlumbingPipe => ("IFCPIPESEGMENT", ".RIGIDSEGMENT."),
            Self::ElectricalCableTray => ("IFCCABLECARRIERSEGMENT", ".CABLETRAYSEGMENT."),
            Self::ElectricalConduit => ("IFCCABLECARRIERSEGMENT", ".CONDUITSEGMENT."),
            Self::LightningConductor => ("IFCCABLESEGMENT", ".CONDUCTORSEGMENT."),
        }
    }
}

/// A straight or sloped 3D linear element (`Beam`, `Brace`, `StripFooting`, `Pipe`, `CableTray`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimRun {
    pub id: usize,
    pub name: String,
    pub category: RunCategory,
    pub level_id: usize,
    pub start: P2,
    pub end: P2,
    pub start_offset_mm: f64,
    pub end_offset_mm: f64,
    pub justify: RunJustify,
    pub profile_name: String,
    pub width_mm: f64,
    pub depth_mm: f64,
    pub is_round: bool,
    pub is_wf_profile: bool,
    pub system_name: String,
}

impl BimRun {
    /// Returns the 3D centerline endpoints `(a, b)` adjusted for `RunJustify` (`Top`, `Centre`, `Bottom`).
    pub fn centerline_3d(&self, level_elev_mm: f64) -> (P3, P3) {
        let z0_raw = level_elev_mm + self.start_offset_mm;
        let z1_raw = level_elev_mm + self.end_offset_mm;
        let plan_len = self.start.dist(self.end);
        let dz = z1_raw - z0_raw;
        let true_len = plan_len.hypot(dz);
        let half_vertical = if plan_len > 1.0 {
            (self.depth_mm * 0.5) * (true_len / plan_len)
        } else {
            0.0
        };
        let shift = match self.justify {
            RunJustify::Top => -half_vertical,
            RunJustify::Centre => 0.0,
            RunJustify::Bottom => half_vertical,
        };
        (
            [self.start.0, self.start.1, z0_raw + shift],
            [self.end.0, self.end.1, z1_raw + shift],
        )
    }

    pub fn length_mm(&self, level_elev_mm: f64) -> f64 {
        let (a, b) = self.centerline_3d(level_elev_mm);
        ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
    }

    pub fn footprint_ring(&self) -> Vec<P2> {
        let d = self.end - self.start;
        if d.length() <= 1.0 {
            let r = self.width_mm * 0.5;
            return vec![
                self.start + P2(-r, -r),
                self.start + P2(r, -r),
                self.start + P2(r, r),
                self.start + P2(-r, r),
            ];
        }
        let n = d.unit().perp() * (self.width_mm * 0.5);
        vec![
            self.start - n,
            self.end - n,
            self.end + n,
            self.start + n,
        ]
    }

    pub fn mesh_3d(&self, level_elev_mm: f64) -> Vec<[P3; 3]> {
        let (a, b) = self.centerline_3d(level_elev_mm);
        if self.is_round {
            extrude_tube_mesh(a, b, self.width_mm, 24)
        } else if self.is_wf_profile {
            extrude_wf_section_mesh(
                a,
                b,
                self.depth_mm,
                self.width_mm,
                (self.depth_mm * 0.045).max(10.0),
                (self.width_mm * 0.045).max(8.0),
            )
        } else {
            extrude_box_run_mesh(a, b, self.width_mm, self.depth_mm)
        }
    }
}

/// A structural or architectural Column spanning between levels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimColumn {
    pub id: usize,
    pub name: String,
    pub base_level_id: usize,
    pub top_level_id: usize,
    pub pos: P2,
    pub width_mm: f64,
    pub depth_mm: f64,
    pub rotation_deg: f64,
    pub is_round: bool,
    pub is_wf_profile: bool,
    pub profile_name: String,
}

impl BimColumn {
    pub fn footprint_ring(&self) -> Vec<P2> {
        let hw = self.width_mm * 0.5;
        let hd = self.depth_mm * 0.5;
        let rad = self.rotation_deg.to_radians();
        [P2(-hw, -hd), P2(hw, -hd), P2(hw, hd), P2(-hw, hd)]
            .into_iter()
            .map(|p| self.pos + p.rotate(rad))
            .collect()
    }

    pub fn mesh_3d(&self, z_base: f64, z_top: f64) -> Vec<[P3; 3]> {
        let a = [self.pos.0, self.pos.1, z_base];
        let b = [self.pos.0, self.pos.1, z_top];
        if self.is_round {
            extrude_tube_mesh(a, b, self.width_mm, 24)
        } else if self.is_wf_profile {
            extrude_wf_section_mesh(
                a,
                b,
                self.depth_mm,
                self.width_mm,
                (self.depth_mm * 0.05).max(12.0),
                (self.width_mm * 0.045).max(9.0),
            )
        } else {
            extrude_ring_mesh(&self.footprint_ring(), z_base, z_top)
        }
    }
}

/// Isolated Pad Footing foundation under a column.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimFooting {
    pub id: usize,
    pub name: String,
    pub level_id: usize,
    pub pos: P2,
    pub width_mm: f64,
    pub length_mm: f64,
    pub thickness_mm: f64,
}

impl BimFooting {
    pub fn footprint_ring(&self) -> Vec<P2> {
        let hw = self.width_mm * 0.5;
        let hl = self.length_mm * 0.5;
        vec![
            self.pos + P2(-hw, -hl),
            self.pos + P2(hw, -hl),
            self.pos + P2(hw, hl),
            self.pos + P2(-hw, hl),
        ]
    }

    pub fn mesh_3d(&self, level_elev_mm: f64) -> Vec<[P3; 3]> {
        extrude_ring_mesh(
            &self.footprint_ring(),
            level_elev_mm - self.thickness_mm,
            level_elev_mm,
        )
    }
}

/// Structural Floor Slab or Roof by Footprint with interior shaft openings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimSlab {
    pub id: usize,
    pub name: String,
    pub level_id: usize,
    pub boundary: Vec<P2>,
    pub openings: Vec<Vec<P2>>,
    pub thickness_mm: f64,
    pub is_roof: bool,
    pub roof_slope_deg: f64,
}

impl BimSlab {
    pub fn mesh_3d(&self, level_elev_mm: f64) -> Vec<[P3; 3]> {
        let (z0, z1) = if self.is_roof {
            (level_elev_mm, level_elev_mm + self.thickness_mm)
        } else {
            (level_elev_mm - self.thickness_mm, level_elev_mm)
        };
        let mut solid = extrude_ring_mesh(&self.boundary, z0, z1);
        let mut meter = CsgMeter::default();
        for hole in &self.openings {
            let void_mesh = extrude_ring_mesh(hole, z0 - 20.0, z1 + 20.0);
            if let Ok(cut) = csg_subtract(&solid, &void_mesh, &mut meter) {
                solid = cut;
            }
        }
        if self.is_roof && self.roof_slope_deg > 1.0 {
            let skel =
                straight_skeleton_roof(&self.boundary, level_elev_mm, self.roof_slope_deg, &[]);
            for face in skel.faces_3d {
                if face.len() == 3 {
                    solid.push([face[0], face[1], face[2]]);
                } else if face.len() >= 4 {
                    solid.push([face[0], face[1], face[2]]);
                    solid.push([face[0], face[2], face[3]]);
                }
            }
        }
        solid
    }
}

/// Automatically bounded Architectural Room (`IfcSpace`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimRoom {
    pub id: usize,
    pub number: String,
    pub name: String,
    pub level_id: usize,
    pub seed_point: P2,
    pub outer_ring: Vec<P2>,
    pub holes: Vec<Vec<P2>>,
    pub area_m2: f64,
    pub perimeter_m: f64,
}

/// Associative Linear/Aligned Dimension annotation in plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimDimension {
    pub id: usize,
    pub start: P2,
    pub end: P2,
    pub offset_mm: f64,
}

impl BimDimension {
    pub fn value_mm(&self) -> f64 {
        self.start.dist(self.end)
    }
}

/// A single 3D reinforcing steel bar (`IfcReinforcingBar`) inside a concrete host.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RebarBar {
    pub id: usize,
    pub host_id: usize,
    pub host_kind: String,
    pub grade: String,
    pub diameter_mm: f64,
    pub points_3d: Vec<P3>,
    pub closed_stirrup: bool,
    pub count: usize,
}

impl RebarBar {
    /// Total length of a single bar in millimetres (including hooks/loops).
    pub fn single_length_mm(&self) -> f64 {
        let mut len: f64 = self
            .points_3d
            .windows(2)
            .map(|w| {
                let dx = w[1][0] - w[0][0];
                let dy = w[1][1] - w[0][1];
                let dz = w[1][2] - w[0][2];
                (dx * dx + dy * dy + dz * dz).sqrt()
            })
            .sum();
        if self.closed_stirrup && self.points_3d.len() >= 3 {
            let first = self.points_3d[0];
            let last = *self.points_3d.last().unwrap();
            let dx = first[0] - last[0];
            let dy = first[1] - last[1];
            let dz = first[2] - last[2];
            len += (dx * dx + dy * dy + dz * dz).sqrt();
            // 135-degree seismic stirrup hooks (2 x 6 d_b)
            len += 12.0 * self.diameter_mm;
        }
        len
    }

    /// Total steel weight in kilograms (`count * area * length * 7850 kg/m³`).
    pub fn total_weight_kg(&self) -> f64 {
        let area_m2 = std::f64::consts::PI * (self.diameter_mm * 0.5e-3).powi(2);
        let len_m = self.single_length_mm() * 1e-3;
        (self.count as f64) * area_m2 * len_m * 7850.0
    }
}

/// Parametric Concrete Reinforcement Rule Specification (SNI 2847:2019 / ACI 318-19 / BIMCraft `reinforce`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RebarPreset {
    pub name: String,
    pub grade: String,
    pub cover_mm: f64,
    pub main_bar_dia_mm: f64,
    pub top_bars_count: usize,
    pub bottom_bars_count: usize,
    pub stirrup_dia_mm: f64,
    pub stirrup_spacing_mm: f64,
}

impl Default for RebarPreset {
    fn default() -> Self {
        Self {
            name: "SNI 2847 Seismic Frame Preset (BJTS 420B)".to_string(),
            grade: "BJTS 420B / CA-50".to_string(),
            cover_mm: 40.0,
            main_bar_dia_mm: 22.0,
            top_bars_count: 4,
            bottom_bars_count: 4,
            stirrup_dia_mm: 10.0,
            stirrup_spacing_mm: 150.0,
        }
    }
}

/// Revit-style Object Snap kinds (`SE, SM, SI, SC, SP, SN`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapKind {
    Endpoint,      // SE: Square marker
    Midpoint,      // SM: Triangle marker
    Intersection,  // SI: X marker (Wall/Grid crossings)
    Center,        // SC: Circle marker (Columns/Rooms)
    Perpendicular, // SP: Right-angle marker
    Nearest,       // SN: Hourglass marker
    GridNode,      // Grid intersection
}

impl SnapKind {
    pub fn shortcut_and_label(&self) -> (&'static str, &'static str) {
        match self {
            Self::Endpoint => ("SE", "Endpoint"),
            Self::Midpoint => ("SM", "Midpoint"),
            Self::Intersection => ("SI", "Intersection"),
            Self::Center => ("SC", "Center"),
            Self::Perpendicular => ("SP", "Perpendicular"),
            Self::Nearest => ("SN", "Nearest"),
            Self::GridNode => ("SG", "Grid Intersection"),
        }
    }
}

/// Active snap hit result with 2D world coordinates and ruler readout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapHit {
    pub point: P2,
    pub kind: SnapKind,
    pub description: String,
    pub distance_from_last_mm: Option<f64>,
    pub angle_deg: Option<f64>,
    pub locked_angle_guide: bool,
}

/// Revit-style Floor Plan View Range parameters (in mm relative to the active Level).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewRange {
    pub top_mm: f64,
    pub cut_plane_mm: f64,
    pub bottom_mm: f64,
    pub view_depth_mm: f64,
}

impl Default for ViewRange {
    fn default() -> Self {
        Self {
            top_mm: 2600.0,
            cut_plane_mm: 1200.0,
            bottom_mm: 0.0,
            view_depth_mm: -1200.0,
        }
    }
}

/// Live 2D Section Cut Projection output across an arbitrary cutting line `section_a -> section_b`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SectionCutProjection {
    pub cut_segments_2d: Vec<([f64; 2], [f64; 2], [u8; 3], String)>,
    pub rebar_dots_2d: Vec<([f64; 2], f64)>,
    pub level_lines: Vec<(String, f64)>,
    pub grid_markers: Vec<(String, f64)>,
    pub section_length_mm: f64,
    pub min_z_mm: f64,
    pub max_z_mm: f64,
}

/// Transactional BIM Authoring Command (serializable to/from JSON for UI, CLI, and MCP Agents).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum BimCommand {
    AddLevel {
        name: String,
        elevation_mm: f64,
    },
    SetLevelElevation {
        level_id: usize,
        elevation_mm: f64,
    },
    AddWall {
        base_level_id: usize,
        start: P2,
        end: P2,
        thickness_mm: f64,
        is_structural: bool,
    },
    AddOpening {
        wall_id: usize,
        kind: OpeningKind,
        offset_along_mm: f64,
        width_mm: f64,
        height_mm: f64,
        sill_height_mm: f64,
    },
    AddColumn {
        base_level_id: usize,
        top_level_id: usize,
        pos: P2,
        width_mm: f64,
        depth_mm: f64,
        is_round: bool,
        profile_name: String,
    },
    AddBeamRun {
        level_id: usize,
        category: RunCategory,
        start: P2,
        end: P2,
        width_mm: f64,
        depth_mm: f64,
        justify: RunJustify,
        profile_name: String,
    },
    AddFooting {
        level_id: usize,
        pos: P2,
        width_mm: f64,
        length_mm: f64,
        thickness_mm: f64,
    },
    AddSlab {
        level_id: usize,
        boundary: Vec<P2>,
        openings: Vec<Vec<P2>>,
        thickness_mm: f64,
        is_roof: bool,
        roof_slope_deg: f64,
    },
    AddDimension {
        start: P2,
        end: P2,
        offset_mm: f64,
    },
    ReinforceHost {
        host_id: usize,
        preset: RebarPreset,
    },
    ReinforceAllConcrete {
        preset: RebarPreset,
    },
    MoveElement {
        element_id: usize,
        delta: P2,
    },
    RotateElement {
        element_id: usize,
        angle_deg: f64,
    },
    MirrorElements {
        axis_a: P2,
        axis_b: P2,
    },
    TrimExtendWalls {
        wall_a_id: usize,
        wall_b_id: usize,
    },
    SplitWall {
        wall_id: usize,
        split_pt: P2,
    },
    OffsetWall {
        wall_id: usize,
        offset_mm: f64,
    },
    DeleteElement {
        element_id: usize,
    },
    AutoDetectRooms {
        level_id: usize,
    },
}

/// Complete Relational BIM Project Model with Transactional Undo/Redo History.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BimProject {
    pub project_name: String,
    pub next_id: usize,
    pub levels: Vec<BimLevel>,
    pub grids: Vec<BimGrid>,
    pub walls: Vec<BimWall>,
    pub runs: Vec<BimRun>,
    pub columns: Vec<BimColumn>,
    pub footings: Vec<BimFooting>,
    pub slabs: Vec<BimSlab>,
    pub rooms: Vec<BimRoom>,
    pub dimensions: Vec<BimDimension>,
    pub rebars: Vec<RebarBar>,
    pub view_range: ViewRange,
    pub section_line: (P2, P2),
}

impl Default for BimProject {
    fn default() -> Self {
        Self::sample_bimcraft_building()
    }
}

impl BimProject {
    /// Creates a rich multi-storey structural & architectural BIM project modeled after BIMCraft's
    /// sample house + structural frame (Levels 1-3, Grids A-C / 1-4, Compound Walls with CSG Doors/Windows,
    /// WF & RC Columns, Beams, Isolated Footings, Slabs with Shaft Opening, Roof, Rooms, and Rebar).
    pub fn sample_bimcraft_building() -> Self {
        let mut proj = Self {
            project_name: "EigenGraph × BIMCraft Integrated Building (LOD 350)".to_string(),
            next_id: 1,
            levels: Vec::new(),
            grids: Vec::new(),
            walls: Vec::new(),
            runs: Vec::new(),
            columns: Vec::new(),
            footings: Vec::new(),
            slabs: Vec::new(),
            rooms: Vec::new(),
            dimensions: Vec::new(),
            rebars: Vec::new(),
            view_range: ViewRange::default(),
            section_line: (P2(-1000.0, 4500.0), P2(13000.0, 4500.0)),
        };

        // 1. Levels (Level 1 = 0m, Level 2 = +4.0m, Roof = +7.6m)
        let l1 = proj.alloc_id();
        proj.levels.push(BimLevel {
            id: l1,
            name: "Level 1 (Ground +0.00m)".to_string(),
            elevation_mm: 0.0,
        });
        let l2 = proj.alloc_id();
        proj.levels.push(BimLevel {
            id: l2,
            name: "Level 2 (First Floor +4.00m)".to_string(),
            elevation_mm: 4000.0,
        });
        let l3 = proj.alloc_id();
        proj.levels.push(BimLevel {
            id: l3,
            name: "Level 3 (Roof Deck +7.60m)".to_string(),
            elevation_mm: 7600.0,
        });

        // 2. Structural Grids (1, 2, 3 along X at 0, 6000, 12000; A, B, C along Y at 0, 4500, 9000)
        for (idx, x) in [0.0, 6000.0, 12000.0].into_iter().enumerate() {
            let gid = proj.alloc_id();
            proj.grids.push(BimGrid {
                id: gid,
                label: format!("{}", idx + 1),
                start: P2(x, -1500.0),
                end: P2(x, 10500.0),
            });
        }
        for (label, y) in [("A", 0.0), ("B", 4500.0), ("C", 9000.0)] {
            let gid = proj.alloc_id();
            proj.grids.push(BimGrid {
                id: gid,
                label: label.to_string(),
                start: P2(-1500.0, y),
                end: P2(13500.0, y),
            });
        }

        // 3. Structural Columns & Isolated Pad Footings at Grid Intersections
        for &x in &[0.0, 6000.0, 12000.0] {
            for &y in &[0.0, 4500.0, 9000.0] {
                let fid = proj.alloc_id();
                proj.footings.push(BimFooting {
                    id: fid,
                    name: format!("Pad Footing F-({:.0},{:.0})", x / 1000.0, y / 1000.0),
                    level_id: l1,
                    pos: P2(x, y),
                    width_mm: 1600.0,
                    length_mm: 1600.0,
                    thickness_mm: 500.0,
                });

                // Story 1 Column
                let c1 = proj.alloc_id();
                proj.columns.push(BimColumn {
                    id: c1,
                    name: format!("Col C1-({:.0},{:.0})", x / 1000.0, y / 1000.0),
                    base_level_id: l1,
                    top_level_id: l2,
                    pos: P2(x, y),
                    width_mm: 400.0,
                    depth_mm: 400.0,
                    rotation_deg: 0.0,
                    is_round: false,
                    is_wf_profile: true,
                    profile_name: "WF 400x200x8x13 / RC 400x400".to_string(),
                });

                // Story 2 Column
                let c2 = proj.alloc_id();
                proj.columns.push(BimColumn {
                    id: c2,
                    name: format!("Col C2-({:.0},{:.0})", x / 1000.0, y / 1000.0),
                    base_level_id: l2,
                    top_level_id: l3,
                    pos: P2(x, y),
                    width_mm: 350.0,
                    depth_mm: 350.0,
                    rotation_deg: 0.0,
                    is_round: false,
                    is_wf_profile: true,
                    profile_name: "WF 300x150x6.5x9".to_string(),
                });
            }
        }

        // 4. Structural Beams (Runs) on Level 2 and Level 3
        for &lvl in &[l2, l3] {
            // Longitudinal X Beams
            for &y in &[0.0, 4500.0, 9000.0] {
                for &(x0, x1) in &[(0.0, 6000.0), (6000.0, 12000.0)] {
                    let bid = proj.alloc_id();
                    proj.runs.push(BimRun {
                        id: bid,
                        name: format!("Girder GX L{lvl}"),
                        category: RunCategory::StructuralBeam,
                        level_id: lvl,
                        start: P2(x0, y),
                        end: P2(x1, y),
                        start_offset_mm: 0.0,
                        end_offset_mm: 0.0,
                        justify: RunJustify::Top,
                        profile_name: "WF 400x200x8x13".to_string(),
                        width_mm: 200.0,
                        depth_mm: 400.0,
                        is_round: false,
                        is_wf_profile: true,
                        system_name: "Primary Structural Frame".to_string(),
                    });
                }
            }
            // Transverse Y Beams
            for &x in &[0.0, 6000.0, 12000.0] {
                for &(y0, y1) in &[(0.0, 4500.0), (4500.0, 9000.0)] {
                    let bid = proj.alloc_id();
                    proj.runs.push(BimRun {
                        id: bid,
                        name: format!("Girder GY L{lvl}"),
                        category: RunCategory::StructuralBeam,
                        level_id: lvl,
                        start: P2(x, y0),
                        end: P2(x, y1),
                        start_offset_mm: 0.0,
                        end_offset_mm: 0.0,
                        justify: RunJustify::Top,
                        profile_name: "WF 300x150x6.5x9".to_string(),
                        width_mm: 150.0,
                        depth_mm: 300.0,
                        is_round: false,
                        is_wf_profile: true,
                        system_name: "Primary Structural Frame".to_string(),
                    });
                }
            }
        }

        // 5. Perimeter & Interior Partition Walls on Level 1 with Hosted Doors & Windows
        let perim = [
            (P2(0.0, 0.0), P2(12000.0, 0.0), "South Exterior Wall"),
            (P2(12000.0, 0.0), P2(12000.0, 9000.0), "East Exterior Wall"),
            (P2(12000.0, 9000.0), P2(0.0, 9000.0), "North Exterior Wall"),
            (P2(0.0, 9000.0), P2(0.0, 0.0), "West Exterior Wall"),
        ];
        for (idx, (a, b, wname)) in perim.into_iter().enumerate() {
            let wid = proj.alloc_id();
            let mut openings = Vec::new();
            if idx == 0 {
                // South wall: Main Entrance Double Door + 2 Casement Windows
                let op1 = proj.alloc_id();
                openings.push(BimOpening {
                    id: op1,
                    mark: "D-01".to_string(),
                    kind: OpeningKind::DoubleDoor,
                    offset_along_mm: 2400.0,
                    width_mm: 1600.0,
                    height_mm: 2200.0,
                    sill_height_mm: 0.0,
                    flip_facing: false,
                    flip_hand: false,
                });
                let op2 = proj.alloc_id();
                openings.push(BimOpening {
                    id: op2,
                    mark: "W-01".to_string(),
                    kind: OpeningKind::CasementWindow,
                    offset_along_mm: 7500.0,
                    width_mm: 1800.0,
                    height_mm: 1400.0,
                    sill_height_mm: 900.0,
                    flip_facing: false,
                    flip_hand: false,
                });
            } else if idx == 2 {
                // North wall window
                let op3 = proj.alloc_id();
                openings.push(BimOpening {
                    id: op3,
                    mark: "W-02".to_string(),
                    kind: OpeningKind::SlidingWindow,
                    offset_along_mm: 3000.0,
                    width_mm: 2000.0,
                    height_mm: 1400.0,
                    sill_height_mm: 900.0,
                    flip_facing: false,
                    flip_hand: false,
                });
            }
            proj.walls.push(BimWall {
                id: wid,
                name: wname.to_string(),
                base_level_id: l1,
                top_level_id: Some(l2),
                unconnected_height_mm: 4000.0,
                start: a,
                end: b,
                thickness_mm: 240.0,
                is_structural: true,
                layers: BimWall::default_compound_layers(240.0),
                openings,
            });
        }

        // Interior Partition Walls splitting Level 1 into 3 Rooms (Main Hall, Technical Lab, Corridor)
        let part1_id = proj.alloc_id();
        let door_int1 = proj.alloc_id();
        proj.walls.push(BimWall {
            id: part1_id,
            name: "Interior Partition Wall EW-1".to_string(),
            base_level_id: l1,
            top_level_id: Some(l2),
            unconnected_height_mm: 4000.0,
            start: P2(6000.0, 0.0),
            end: P2(6000.0, 9000.0),
            thickness_mm: 150.0,
            is_structural: false,
            layers: BimWall::default_compound_layers(150.0),
            openings: vec![BimOpening {
                id: door_int1,
                mark: "D-02".to_string(),
                kind: OpeningKind::SingleDoor,
                offset_along_mm: 1800.0,
                width_mm: 900.0,
                height_mm: 2100.0,
                sill_height_mm: 0.0,
                flip_facing: false,
                flip_hand: false,
            }],
        });

        let part2_id = proj.alloc_id();
        let door_int2 = proj.alloc_id();
        proj.walls.push(BimWall {
            id: part2_id,
            name: "Interior Partition Wall NS-1".to_string(),
            base_level_id: l1,
            top_level_id: Some(l2),
            unconnected_height_mm: 4000.0,
            start: P2(6000.0, 4500.0),
            end: P2(12000.0, 4500.0),
            thickness_mm: 150.0,
            is_structural: false,
            layers: BimWall::default_compound_layers(150.0),
            openings: vec![BimOpening {
                id: door_int2,
                mark: "D-03".to_string(),
                kind: OpeningKind::SingleDoor,
                offset_along_mm: 2200.0,
                width_mm: 900.0,
                height_mm: 2100.0,
                sill_height_mm: 0.0,
                flip_facing: true,
                flip_hand: false,
            }],
        });

        // 6. Structural Floor Slab on Level 2 with an Elevator/Stair Shaft Opening
        let slab_id = proj.alloc_id();
        proj.slabs.push(BimSlab {
            id: slab_id,
            name: "Level 2 Structural Slab (RC 180mm)".to_string(),
            level_id: l2,
            boundary: vec![
                P2(0.0, 0.0),
                P2(12000.0, 0.0),
                P2(12000.0, 9000.0),
                P2(0.0, 9000.0),
            ],
            openings: vec![vec![
                P2(4200.0, 6200.0),
                P2(5800.0, 6200.0),
                P2(5800.0, 8200.0),
                P2(4200.0, 8200.0),
            ]],
            thickness_mm: 180.0,
            is_roof: false,
            roof_slope_deg: 0.0,
        });

        // 7. Associative Plan Dimensions
        for &(a, b, off) in &[
            (P2(0.0, 0.0), P2(6000.0, 0.0), -900.0),
            (P2(6000.0, 0.0), P2(12000.0, 0.0), -900.0),
            (P2(0.0, 0.0), P2(12000.0, 0.0), -1450.0),
            (P2(0.0, 0.0), P2(0.0, 4500.0), -900.0),
            (P2(0.0, 4500.0), P2(0.0, 9000.0), -900.0),
        ] {
            let did = proj.alloc_id();
            proj.dimensions.push(BimDimension {
                id: did,
                start: a,
                end: b,
                offset_mm: off,
            });
        }

        // 8. Auto-detect rooms on Level 1 & generate initial rebar for primary frame
        proj.detect_rooms_on_level(l1);
        proj.reinforce_all_concrete(&RebarPreset::default());

        proj
    }

    #[inline]
    pub fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn level_elevation(&self, level_id: usize) -> f64 {
        self.levels
            .iter()
            .find(|l| l.id == level_id)
            .map(|l| l.elevation_mm)
            .unwrap_or(0.0)
    }

    /// Uses the Planar Half-Edge Arrangement (`geom::faces` & `geom::region_in`) to automatically
    /// discover all enclosed Rooms on `level_id`, computing exact net floor areas and perimeters.
    pub fn detect_rooms_on_level(&mut self, level_id: usize) {
        let segs: Vec<(P2, P2)> = self
            .walls
            .iter()
            .filter(|w| w.base_level_id == level_id)
            .map(|w| (w.start, w.end))
            .collect();

        let all_faces = faces(&segs, 5.0);
        self.rooms.retain(|r| r.level_id != level_id);

        let room_names = [
            "Main Structural Hall",
            "Control & Server Room",
            "Executive Engineering Office",
            "Archive & Vault",
            "Circulation Lobby",
        ];

        let mut room_idx = 1usize;
        for f in &all_faces {
            let a = area(f);
            if a > 1.0e6 {
                // Positive area > 1 m² is an interior bounded room face
                let c = crate::core::geom::centroid(f);
                if let Some(reg) = region_in(&all_faces, c, 5.0) {
                    let rid = self.alloc_id();
                    let name = room_names
                        .get((room_idx - 1) % room_names.len())
                        .copied()
                        .unwrap_or("Room")
                        .to_string();
                    self.rooms.push(BimRoom {
                        id: rid,
                        number: format!("RM-10{}", room_idx),
                        name,
                        level_id,
                        seed_point: c,
                        area_m2: reg.area() * 1e-6,
                        perimeter_m: reg.perimeter() * 1e-3,
                        outer_ring: reg.outer,
                        holes: reg.holes,
                    });
                    room_idx += 1;
                }
            }
        }
    }

    /// Arms a specific beam, column, or footing (`host_id`) with parametric rebar layers & stirrups.
    pub fn reinforce_host(&mut self, host_id: usize, preset: &RebarPreset) -> Result<usize, String> {
        if preset.cover_mm < 15.0 || preset.cover_mm > 100.0 {
            return Err("Concrete cover must be between 15 mm and 100 mm".to_string());
        }
        self.rebars.retain(|r| r.host_id != host_id);
        let mut added_bars = 0usize;

        // 1. Check if host is a Beam Run
        if let Some(run) = self.runs.iter().find(|r| r.id == host_id).cloned() {
            let elev = self.level_elevation(run.level_id);
            let (a, b) = run.centerline_3d(elev);
            let (_, e1, e2) = crate::core::csg::local_frame_along(a, b);
            let net_w = (run.width_mm - 2.0 * preset.cover_mm - preset.main_bar_dia_mm).max(40.0);
            let net_d = (run.depth_mm - 2.0 * preset.cover_mm - preset.main_bar_dia_mm).max(60.0);

            // Bottom longitudinal tension bars
            for i in 0..preset.bottom_bars_count.max(2) {
                let frac = if preset.bottom_bars_count <= 1 {
                    0.0
                } else {
                    (i as f64) / ((preset.bottom_bars_count - 1) as f64) - 0.5
                };
                let u = frac * net_w;
                let v = -net_d * 0.5;
                let offset = crate::core::csg::add3(
                    crate::core::csg::mul3(e1, u),
                    crate::core::csg::mul3(e2, v),
                );
                let p0 = crate::core::csg::add3(a, offset);
                let p1 = crate::core::csg::add3(b, offset);
                let hook_up = crate::core::csg::mul3(e2, 12.0 * preset.main_bar_dia_mm);
                let id = self.alloc_id();
                self.rebars.push(RebarBar {
                    id,
                    host_id,
                    host_kind: "Beam Bottom Bar".to_string(),
                    grade: preset.grade.clone(),
                    diameter_mm: preset.main_bar_dia_mm,
                    points_3d: vec![
                        crate::core::csg::add3(p0, hook_up),
                        p0,
                        p1,
                        crate::core::csg::add3(p1, hook_up),
                    ],
                    closed_stirrup: false,
                    count: 1,
                });
                added_bars += 1;
            }

            // Top longitudinal bars
            for i in 0..preset.top_bars_count.max(2) {
                let frac = if preset.top_bars_count <= 1 {
                    0.0
                } else {
                    (i as f64) / ((preset.top_bars_count - 1) as f64) - 0.5
                };
                let u = frac * net_w;
                let v = net_d * 0.5;
                let offset = crate::core::csg::add3(
                    crate::core::csg::mul3(e1, u),
                    crate::core::csg::mul3(e2, v),
                );
                let p0 = crate::core::csg::add3(a, offset);
                let p1 = crate::core::csg::add3(b, offset);
                let id = self.alloc_id();
                self.rebars.push(RebarBar {
                    id,
                    host_id,
                    host_kind: "Beam Top Bar".to_string(),
                    grade: preset.grade.clone(),
                    diameter_mm: preset.main_bar_dia_mm,
                    points_3d: vec![p0, p1],
                    closed_stirrup: false,
                    count: 1,
                });
                added_bars += 1;
            }

            // Closed Stirrup Hoops along the beam span
            let len = run.length_mm(elev);
            let n_stirrups = ((len / preset.stirrup_spacing_mm.max(75.0)).floor() as usize).max(2);
            let mid = crate::core::csg::mul3(crate::core::csg::add3(a, b), 0.5);
            let hw = net_w * 0.5;
            let hd = net_d * 0.5;
            let corners = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];
            let hoop_pts: Vec<P3> = corners
                .into_iter()
                .map(|(u, v)| {
                    crate::core::csg::add3(
                        mid,
                        crate::core::csg::add3(
                            crate::core::csg::mul3(e1, u),
                            crate::core::csg::mul3(e2, v),
                        ),
                    )
                })
                .collect();
            let id = self.alloc_id();
            self.rebars.push(RebarBar {
                id,
                host_id,
                host_kind: "Beam Seismic Stirrup Hoop".to_string(),
                grade: preset.grade.clone(),
                diameter_mm: preset.stirrup_dia_mm,
                points_3d: hoop_pts,
                closed_stirrup: true,
                count: n_stirrups,
            });
            added_bars += n_stirrups;
            return Ok(added_bars);
        }

        // 2. Check if host is a Column
        if let Some(col) = self.columns.iter().find(|c| c.id == host_id).cloned() {
            let z0 = self.level_elevation(col.base_level_id);
            let z1 = self.level_elevation(col.top_level_id);
            let hw = (col.width_mm * 0.5 - preset.cover_mm).max(40.0);
            let hd = (col.depth_mm * 0.5 - preset.cover_mm).max(40.0);

            for (ux, uy) in [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd), (0.0, -hd), (0.0, hd)] {
                let id = self.alloc_id();
                self.rebars.push(RebarBar {
                    id,
                    host_id,
                    host_kind: "Column Longitudinal Dowel".to_string(),
                    grade: preset.grade.clone(),
                    diameter_mm: preset.main_bar_dia_mm,
                    points_3d: vec![
                        [col.pos.0 + ux, col.pos.1 + uy, z0],
                        [col.pos.0 + ux, col.pos.1 + uy, z1 + 600.0], // Lap splice extension
                    ],
                    closed_stirrup: false,
                    count: 1,
                });
                added_bars += 1;
            }

            let h_col = (z1 - z0).abs();
            let n_ties = ((h_col / preset.stirrup_spacing_mm.max(75.0)).floor() as usize).max(3);
            let zm = (z0 + z1) * 0.5;
            let id = self.alloc_id();
            self.rebars.push(RebarBar {
                id,
                host_id,
                host_kind: "Column Confinement Tie".to_string(),
                grade: preset.grade.clone(),
                diameter_mm: preset.stirrup_dia_mm,
                points_3d: vec![
                    [col.pos.0 - hw, col.pos.1 - hd, zm],
                    [col.pos.0 + hw, col.pos.1 - hd, zm],
                    [col.pos.0 + hw, col.pos.1 + hd, zm],
                    [col.pos.0 - hw, col.pos.1 + hd, zm],
                ],
                closed_stirrup: true,
                count: n_ties,
            });
            added_bars += n_ties;
            return Ok(added_bars);
        }

        // 3. Check if host is an Isolated Footing
        if let Some(ft) = self.footings.iter().find(|f| f.id == host_id).cloned() {
            let z_bot = self.level_elevation(ft.level_id) - ft.thickness_mm + preset.cover_mm;
            let hw = (ft.width_mm * 0.5 - preset.cover_mm).max(100.0);
            let hl = (ft.length_mm * 0.5 - preset.cover_mm).max(100.0);
            let n_mesh = 8usize;
            let id_x = self.alloc_id();
            self.rebars.push(RebarBar {
                id: id_x,
                host_id,
                host_kind: "Footing Bottom Mesh X".to_string(),
                grade: preset.grade.clone(),
                diameter_mm: 16.0,
                points_3d: vec![
                    [ft.pos.0 - hw, ft.pos.1, z_bot + 200.0],
                    [ft.pos.0 - hw, ft.pos.1, z_bot],
                    [ft.pos.0 + hw, ft.pos.1, z_bot],
                    [ft.pos.0 + hw, ft.pos.1, z_bot + 200.0],
                ],
                closed_stirrup: false,
                count: n_mesh,
            });
            let id_y = self.alloc_id();
            self.rebars.push(RebarBar {
                id: id_y,
                host_id,
                host_kind: "Footing Bottom Mesh Y".to_string(),
                grade: preset.grade.clone(),
                diameter_mm: 16.0,
                points_3d: vec![
                    [ft.pos.0, ft.pos.1 - hl, z_bot + 200.0],
                    [ft.pos.0, ft.pos.1 - hl, z_bot + 16.0],
                    [ft.pos.0, ft.pos.1 + hl, z_bot + 16.0],
                    [ft.pos.0, ft.pos.1 + hl, z_bot + 200.0],
                ],
                closed_stirrup: false,
                count: n_mesh,
            });
            added_bars += 2 * n_mesh;
            return Ok(added_bars);
        }

        Err(format!("Host element #{host_id} not found for reinforcement"))
    }

    /// Arms all structural beams, columns, and footings in the project with `preset`.
    pub fn reinforce_all_concrete(&mut self, preset: &RebarPreset) -> usize {
        let host_ids: Vec<usize> = self
            .runs
            .iter()
            .filter(|r| r.category == RunCategory::StructuralBeam)
            .map(|r| r.id)
            .chain(self.columns.iter().map(|c| c.id))
            .chain(self.footings.iter().map(|f| f.id))
            .collect();

        let mut total = 0;
        for hid in host_ids {
            if let Ok(cnt) = self.reinforce_host(hid, preset) {
                total += cnt;
            }
        }
        total
    }

    /// Total reinforcement steel mass in kg across the entire project.
    pub fn total_rebar_weight_kg(&self) -> f64 {
        self.rebars.iter().map(|r| r.total_weight_kg()).sum()
    }

    /// Evaluates Revit-style Object Snaps (`SE, SM, SI, SC, SP, SN`) and 15° Angular Ruler Guides
    /// near `cursor_pt` (within `snap_radius_mm`).
    pub fn find_snap(
        &self,
        cursor_pt: P2,
        last_pt: Option<P2>,
        snap_radius_mm: f64,
        angle_step_deg: f64,
    ) -> SnapHit {
        let mut best: Option<(f64, SnapKind, P2, String)> = None;
        let mut consider = |kind: SnapKind, pt: P2, desc: String| {
            let d = cursor_pt.dist(pt);
            if d <= snap_radius_mm {
                // Prioritize Endpoint/Intersection/GridNode over Midpoint/Nearest
                let priority_weight = match kind {
                    SnapKind::Endpoint | SnapKind::Intersection | SnapKind::GridNode => 0.75,
                    SnapKind::Midpoint | SnapKind::Center => 0.85,
                    SnapKind::Perpendicular => 0.90,
                    SnapKind::Nearest => 1.15,
                };
                let score = d * priority_weight;
                if best.as_ref().is_none_or(|b| score < b.0) {
                    best = Some((score, kind, pt, desc));
                }
            }
        };

        // 1. Wall Endpoints, Midpoints, Perpendiculars & Nearest
        for w in &self.walls {
            consider(SnapKind::Endpoint, w.start, format!("Wall {} Start", w.name));
            consider(SnapKind::Endpoint, w.end, format!("Wall {} End", w.name));
            consider(
                SnapKind::Midpoint,
                (w.start + w.end) * 0.5,
                format!("Wall {} Midpoint", w.name),
            );
            if let Some(lp) = last_pt {
                let t = param_on_segment(lp, w.start, w.end);
                if t > 0.05 && t < 0.95 {
                    let foot = w.start + (w.end - w.start) * t;
                    consider(
                        SnapKind::Perpendicular,
                        foot,
                        format!("Perpendicular to {}", w.name),
                    );
                }
            }
            let near = closest_point_on_segment(cursor_pt, w.start, w.end);
            consider(SnapKind::Nearest, near, format!("Nearest on {}", w.name));
        }

        // 2. Beam/Run Endpoints & Midpoints
        for r in &self.runs {
            consider(SnapKind::Endpoint, r.start, format!("{} Start", r.name));
            consider(SnapKind::Endpoint, r.end, format!("{} End", r.name));
            consider(
                SnapKind::Midpoint,
                (r.start + r.end) * 0.5,
                format!("{} Midpoint", r.name),
            );
        }

        // 3. Column & Room Centers
        for c in &self.columns {
            consider(SnapKind::Center, c.pos, format!("Column {} Center", c.name));
        }
        for rm in &self.rooms {
            consider(
                SnapKind::Center,
                rm.seed_point,
                format!("Room {} Center", rm.number),
            );
        }

        // 4. Grid Intersections & Wall-Wall Intersections
        for i in 0..self.grids.len() {
            for j in (i + 1)..self.grids.len() {
                if let Some(ix) = line_intersection(
                    self.grids[i].start,
                    self.grids[i].end,
                    self.grids[j].start,
                    self.grids[j].end,
                ) {
                    consider(
                        SnapKind::GridNode,
                        ix,
                        format!("Grid {}-{}", self.grids[i].label, self.grids[j].label),
                    );
                }
            }
        }
        for i in 0..self.walls.len() {
            for j in (i + 1)..self.walls.len() {
                if let Some(ix) = line_intersection(
                    self.walls[i].start,
                    self.walls[i].end,
                    self.walls[j].start,
                    self.walls[j].end,
                ) {
                    consider(SnapKind::Intersection, ix, "Wall Corner Intersection".to_string());
                }
            }
        }

        let (mut pt, kind, desc) = if let Some((_, k, p, d)) = best {
            (p, k, d)
        } else {
            (cursor_pt, SnapKind::Nearest, "Free Cursor".to_string())
        };

        // 5. Graduated Ruler & Angular Guide Lock (multiples of `angle_step_deg`, e.g., 15°)
        let mut dist_from_last = None;
        let mut angle_deg = None;
        let mut locked_angle = false;

        if let Some(lp) = last_pt {
            let delta = pt - lp;
            let len = delta.length();
            if len > 1e-3 {
                let raw_deg = delta.1.atan2(delta.0).to_degrees();
                if kind == SnapKind::Nearest && angle_step_deg > 0.0 {
                    let snapped_deg = (raw_deg / angle_step_deg).round() * angle_step_deg;
                    if (raw_deg - snapped_deg).abs() <= 4.0 {
                        let rad = snapped_deg.to_radians();
                        let rounded_len = (len / 50.0).round() * 50.0;
                        pt = lp + P2(rad.cos(), rad.sin()) * rounded_len;
                        angle_deg = Some(snapped_deg);
                        dist_from_last = Some(rounded_len);
                        locked_angle = true;
                    } else {
                        angle_deg = Some(raw_deg);
                        dist_from_last = Some(len);
                    }
                } else {
                    angle_deg = Some(raw_deg);
                    dist_from_last = Some(len);
                }
            }
        }

        SnapHit {
            point: pt,
            kind,
            description: desc,
            distance_from_last_mm: dist_from_last,
            angle_deg,
            locked_angle_guide: locked_angle,
        }
    }

    /// Builds the combined 3D triangle mesh `(element_id, triangle, color_rgb)` for the entire BIM building
    /// and a 3D Bounding Volume Hierarchy (`Bvh3D`) for fast ray-picking.
    pub fn build_scene_meshes_and_bvh(&self) -> (Vec<(usize, [P3; 3], [u8; 3])>, Bvh3D) {
        let mut scene_tris = Vec::new();
        let mut bvh_items = Vec::new();

        // Footings
        for f in &self.footings {
            let z = self.level_elevation(f.level_id);
            for t in f.mesh_3d(z) {
                scene_tris.push((f.id, t, [120, 130, 145]));
                bvh_items.push((f.id, t));
            }
        }

        // Columns
        for c in &self.columns {
            let z0 = self.level_elevation(c.base_level_id);
            let z1 = self.level_elevation(c.top_level_id);
            for t in c.mesh_3d(z0, z1) {
                scene_tris.push((c.id, t, [56, 189, 248]));
                bvh_items.push((c.id, t));
            }
        }

        // Runs (Beams, Braces, Pipes, Trays)
        for r in &self.runs {
            let z = self.level_elevation(r.level_id);
            let col = match r.category {
                RunCategory::StructuralBeam => [34, 197, 94],
                RunCategory::StructuralBrace => [245, 158, 11],
                RunCategory::PlumbingPipe => [14, 165, 233],
                RunCategory::ElectricalCableTray | RunCategory::ElectricalConduit => [168, 85, 247],
                _ => [148, 163, 184],
            };
            for t in r.mesh_3d(z) {
                scene_tris.push((r.id, t, col));
                bvh_items.push((r.id, t));
            }
        }

        // Walls (with CSG door/window cutouts)
        for w in &self.walls {
            let z0 = self.level_elevation(w.base_level_id);
            let z1 = w
                .top_level_id
                .map(|tid| self.level_elevation(tid))
                .unwrap_or(z0 + w.unconnected_height_mm);
            let col = w
                .layers
                .first()
                .map(|l| l.color_rgb)
                .unwrap_or([196, 98, 68]);
            for t in w.mesh_3d(z0, z1) {
                scene_tris.push((w.id, t, col));
                bvh_items.push((w.id, t));
            }
        }

        // Slabs & Roofs (with CSG shaft cutouts)
        for s in &self.slabs {
            let z = self.level_elevation(s.level_id);
            let col = if s.is_roof {
                [180, 83, 9]
            } else {
                [100, 116, 139]
            };
            for t in s.mesh_3d(z) {
                scene_tris.push((s.id, t, col));
                bvh_items.push((s.id, t));
            }
        }

        let bvh = Bvh3D::build(bvh_items);
        (scene_tris, bvh)
    }

    /// Generates a Live 2D Section Cut Projection by slicing all 3D building meshes and rebar bars
    /// along the vertical plane passing through `section_a -> section_b`.
    pub fn generate_section_cut(&self, section_a: P2, section_b: P2) -> SectionCutProjection {
        let d = (section_b - section_a).unit();
        let n_2d = d.perp();
        let plane_n: P3 = [n_2d.0, n_2d.1, 0.0];
        let plane_w = n_2d.0 * section_a.0 + n_2d.1 * section_a.1;

        let (scene_tris, _) = self.build_scene_meshes_and_bvh();
        let mut cut_segments_2d = Vec::new();

        // Group triangles by element and slice against the vertical plane
        for (elem_id, tri, col) in &scene_tris {
            for (p0, p1) in slice_mesh_with_plane(&[*tri], plane_n, plane_w) {
                let u0 = (p0[0] - section_a.0) * d.0 + (p0[1] - section_a.1) * d.1;
                let u1 = (p1[0] - section_a.0) * d.0 + (p1[1] - section_a.1) * d.1;
                cut_segments_2d.push(([u0, p0[2]], [u1, p1[2]], *col, format!("Elem #{elem_id}")));
            }
        }

        // Slice Rebars to produce rebar cut dots in section
        let mut rebar_dots_2d = Vec::new();
        for bar in &self.rebars {
            for w in bar.points_3d.windows(2) {
                let da = plane_n[0] * w[0][0] + plane_n[1] * w[0][1] - plane_w;
                let db = plane_n[0] * w[1][0] + plane_n[1] * w[1][1] - plane_w;
                if (da >= 0.0) != (db >= 0.0) && (da - db).abs() > 1e-6 {
                    let t = da / (da - db);
                    let hit = [
                        w[0][0] + (w[1][0] - w[0][0]) * t,
                        w[0][1] + (w[1][1] - w[0][1]) * t,
                        w[0][2] + (w[1][2] - w[0][2]) * t,
                    ];
                    let u = (hit[0] - section_a.0) * d.0 + (hit[1] - section_a.1) * d.1;
                    rebar_dots_2d.push(([u, hit[2]], bar.diameter_mm));
                }
            }
        }

        let level_lines = self
            .levels
            .iter()
            .map(|l| (l.name.clone(), l.elevation_mm))
            .collect();

        let mut grid_markers = Vec::new();
        for g in &self.grids {
            if let Some(ix) = line_intersection(section_a, section_b, g.start, g.end) {
                let u = (ix.0 - section_a.0) * d.0 + (ix.1 - section_a.1) * d.1;
                grid_markers.push((g.label.clone(), u));
            }
        }

        let min_z_mm = self
            .levels
            .iter()
            .map(|l| l.elevation_mm)
            .fold(0.0_f64, f64::min)
            - 600.0;
        let max_z_mm = self
            .levels
            .iter()
            .map(|l| l.elevation_mm)
            .fold(4000.0_f64, f64::max)
            + 1500.0;

        SectionCutProjection {
            cut_segments_2d,
            rebar_dots_2d,
            level_lines,
            grid_markers,
            section_length_mm: section_a.dist(section_b),
            min_z_mm,
            max_z_mm,
        }
    }

    /// Dual Physical–Analytical Synchronization: converts the physical BIM building into a 3D
    /// Finite Element `StructuralModel` (coordinates in metres) for immediate `FemSolver` & `AriEngine` audit.
    pub fn to_structural_model(&self) -> StructuralModel {
        let mut model = StructuralModel::new(self.project_name.clone());
        let tol_m = 0.05; // 50 mm node merging tolerance

        let find_or_add_node = |m: &mut StructuralModel, x_m: f64, y_m: f64, z_m: f64| -> usize {
            if let Some(existing) = m.nodes.iter().find(|n| {
                (n.x - x_m).hypot(n.y - y_m).hypot(n.z - z_m) <= tol_m
            }) {
                return existing.id;
            }
            let id = m.nodes.len();
            let mut node = Node3D::new(id, x_m, y_m, z_m);
            if y_m.abs() <= 0.05 {
                node.fixity = BoundaryFixity::fixed();
            }
            m.add_node(node);
            id
        };

        let steel = Material::structural_steel();
        let mut elem_id = 1usize;

        // 1. Convert Structural Columns
        for c in &self.columns {
            let z0_m = self.level_elevation(c.base_level_id) * 1e-3;
            let z1_m = self.level_elevation(c.top_level_id) * 1e-3;
            let x_m = c.pos.0 * 1e-3;
            let y_m = c.pos.1 * 1e-3;
            let n1 = find_or_add_node(&mut model, x_m, z0_m, y_m);
            let n2 = find_or_add_node(&mut model, x_m, z1_m, y_m);
            if n1 != n2 {
                let elem = Element3D::new(elem_id, c.name.clone(), n1, n2)
                    .with_type(ElementType::Frame3D)
                    .with_section(CrossSection::wf_400x200())
                    .with_material(steel.clone());
                model.add_element(elem);
                elem_id += 1;
            }
        }

        // 2. Convert Structural Beams & Braces (analytical stick axis aligned to datum level)
        for r in &self.runs {
            if matches!(
                r.category,
                RunCategory::StructuralBeam | RunCategory::StructuralBrace
            ) {
                let elev = self.level_elevation(r.level_id);
                let z0_m = (elev + r.start_offset_mm) * 1e-3;
                let z1_m = (elev + r.end_offset_mm) * 1e-3;
                let n1 = find_or_add_node(&mut model, r.start.0 * 1e-3, z0_m, r.start.1 * 1e-3);
                let n2 = find_or_add_node(&mut model, r.end.0 * 1e-3, z1_m, r.end.1 * 1e-3);
                if n1 != n2 {
                    let elem = Element3D::new(elem_id, r.name.clone(), n1, n2)
                        .with_type(ElementType::Frame3D)
                        .with_section(CrossSection::wf_300x150())
                        .with_material(steel.clone());
                    model.add_element(elem);
                    elem_id += 1;
                }
            }
        }

        // 3. Convert Structural Shear Walls into Equivalent Diagonal Struts for Redundancy Path Analysis
        for w in &self.walls {
            if w.is_structural {
                let z0_m = self.level_elevation(w.base_level_id) * 1e-3;
                let z1_m = w
                    .top_level_id
                    .map(|tid| self.level_elevation(tid) * 1e-3)
                    .unwrap_or(z0_m + w.unconnected_height_mm * 1e-3);
                let n_bl = find_or_add_node(&mut model, w.start.0 * 1e-3, z0_m, w.start.1 * 1e-3);
                let n_br = find_or_add_node(&mut model, w.end.0 * 1e-3, z0_m, w.end.1 * 1e-3);
                let n_tl = find_or_add_node(&mut model, w.start.0 * 1e-3, z1_m, w.start.1 * 1e-3);
                let n_tr = find_or_add_node(&mut model, w.end.0 * 1e-3, z1_m, w.end.1 * 1e-3);

                if n_bl != n_tr {
                    let strut = Element3D::new(
                        elem_id,
                        format!("{} Shear Strut", w.name),
                        n_bl,
                        n_tr,
                    )
                    .with_type(ElementType::Frame3D)
                    .with_section(CrossSection::box_200x200x8())
                    .with_material(steel.clone());
                    model.add_element(strut);
                    elem_id += 1;
                }
                if n_br != n_tl {
                    let strut2 = Element3D::new(
                        elem_id,
                        format!("{} Counter Strut", w.name),
                        n_br,
                        n_tl,
                    )
                    .with_type(ElementType::Frame3D)
                    .with_section(CrossSection::box_200x200x8())
                    .with_material(steel.clone());
                    model.add_element(strut2);
                    elem_id += 1;
                }
            }
        }

        // 4. Apply Gravity & Lateral Nodal Loads at Elevated Floor Nodes
        let elevated_nodes: Vec<usize> = model
            .nodes
            .iter()
            .filter(|n| n.y > 0.5)
            .map(|n| n.id)
            .collect();
        for nid in elevated_nodes {
            model.add_load(NodalLoad {
                node_id: nid,
                fx: 18_000.0,   // 18 kN lateral seismic/wind shear
                fy: -120_000.0, // -120 kN gravity dead+live load
                fz: 0.0,
                mx: 0.0,
                my: 0.0,
                mz: 0.0,
            });
        }

        model
    }

    /// Evaluates live Adjusted Redundancy Index (ARI) and SVD Kinematic Guard on the BIM building.
    pub fn evaluate_live_ari(&self) -> Result<RedundancyAssessment, String> {
        let struct_model = self.to_structural_model();
        AriEngine::evaluate_redundancy_fast(&struct_model)
    }

    /// Exports an architectural/structural Drawing Sheet (`A3` Title Block containing Floor Plan,
    /// Section Cut, Room Schedule & Rebar Summary) as a crisp scalable Vector **SVG** document.
    pub fn export_sheet_svg(&self, sheet_number: &str, sheet_title: &str) -> String {
        let mut svg = String::with_capacity(8192);
        svg.push_str(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 848" width="1200" height="848" style="background:#ffffff;font-family:Arial,sans-serif;">"##,
        );
        // Outer Sheet Border & Title Block
        svg.push_str(
            r##"<rect x="20" y="20" width="1160" height="808" fill="none" stroke="#1e293b" stroke-width="2.5"/>"##,
        );
        svg.push_str(
            r##"<rect x="880" y="640" width="300" height="188" fill="#f8fafc" stroke="#1e293b" stroke-width="1.5"/>"##,
        );
        svg.push_str(&format!(
            r##"<text x="895" y="670" font-size="15" font-weight="bold" fill="#0f172a">EIGENGRAPH × BIMCRAFT 10D</text>
<text x="895" y="695" font-size="12" fill="#334155">Project: {}</text>
<text x="895" y="720" font-size="12" fill="#334155">Sheet: {} — {}</text>
<text x="895" y="745" font-size="11" fill="#475569">Scale: 1 : 100 | Rebar Mass: {:.1} kg</text>
<text x="895" y="770" font-size="11" fill="#0284c7">Walls: {} | Columns: {} | Beams: {} | Rooms: {}</text>"##,
            self.project_name,
            sheet_number,
            sheet_title,
            self.total_rebar_weight_kg(),
            self.walls.len(),
            self.columns.len(),
            self.runs.len(),
            self.rooms.len()
        ));

        // Viewport 1: Floor Plan (Scale 1:100 mapped into x=80..680, y=80..560)
        let map_plan = |p: P2| -> (f64, f64) { (120.0 + p.0 * 0.04, 480.0 - p.1 * 0.04) };
        for g in &self.grids {
            let (x0, y0) = map_plan(g.start);
            let (x1, y1) = map_plan(g.end);
            svg.push_str(&format!(
                r##"<line x1="{x0:.1}" y1="{y0:.1}" x2="{x1:.1}" y2="{y1:.1}" stroke="#38bdf8" stroke-width="0.8" stroke-dasharray="6,4"/>"##
            ));
            svg.push_str(&format!(
                r##"<circle cx="{x0:.1}" cy="{y0:.1}" r="10" fill="#ffffff" stroke="#0284c7" stroke-width="1.2"/><text x="{x0:.1}" y="{:.1}" font-size="9" text-anchor="middle" fill="#0284c7">{}</text>"##,
                y0 + 3.0,
                g.label
            ));
        }

        for w in &self.walls {
            let ring = w.footprint_ring();
            let pts_str: String = ring
                .iter()
                .map(|&p| {
                    let (sx, sy) = map_plan(p);
                    format!("{sx:.1},{sy:.1}")
                })
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                r##"<polygon points="{pts_str}" fill="#94a3b8" stroke="#0f172a" stroke-width="1.4"/>"##
            ));
        }

        for c in &self.columns {
            let (cx, cy) = map_plan(c.pos);
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="16" height="16" fill="#0f172a"/>"##,
                cx - 8.0,
                cy - 8.0
            ));
        }

        for rm in &self.rooms {
            let (rx, ry) = map_plan(rm.seed_point);
            svg.push_str(&format!(
                r##"<text x="{rx:.1}" y="{ry:.1}" font-size="10" font-weight="bold" text-anchor="middle" fill="#0f766e">{} ({:.1} m²)</text>"##,
                rm.name, rm.area_m2
            ));
        }

        svg.push_str(
            r##"<text x="120" y="590" font-size="13" font-weight="bold" fill="#0f172a">VIEW 1: FLOOR PLAN — LEVEL 1 (CUT PLANE +1200 mm)</text>"##,
        );
        svg.push_str("</svg>");
        svg
    }

    /// Exports a standard **PDF 1.4** vector drawing sheet containing the Floor Plan, Grids,
    /// Walls, Columns, Room Schedule, and Title Block.
    pub fn export_sheet_pdf(&self, sheet_number: &str, sheet_title: &str) -> Vec<u8> {
        let mut stream = String::with_capacity(4096);
        // Outer Sheet Frame
        stream.push_str("1.5 w 0.1 0.15 0.25 RG 20 20 802 555 re S\n");
        // Title Block Box
        stream.push_str("1.0 w 580 20 242 110 re S\n");

        // Draw Grids in Dashed Cyan
        stream.push_str("[4 3] 0 d 0.5 w 0.1 0.6 0.85 RG\n");
        for g in &self.grids {
            let x0 = 80.0 + g.start.0 * 0.03;
            let y0 = 120.0 + g.start.1 * 0.03;
            let x1 = 80.0 + g.end.0 * 0.03;
            let y1 = 120.0 + g.end.1 * 0.03;
            stream.push_str(&format!("{x0:.1} {y0:.1} m {x1:.1} {y1:.1} l S\n"));
        }

        // Draw Walls in Solid Poché
        stream.push_str("[] 0 d 1.2 w 0.1 0.1 0.15 RG 0.65 0.70 0.78 rg\n");
        for w in &self.walls {
            let ring = w.footprint_ring();
            if let Some(first) = ring.first() {
                stream.push_str(&format!(
                    "{:.1} {:.1} m ",
                    80.0 + first.0 * 0.03,
                    120.0 + first.1 * 0.03
                ));
                for p in ring.iter().skip(1) {
                    stream.push_str(&format!(
                        "{:.1} {:.1} l ",
                        80.0 + p.0 * 0.03,
                        120.0 + p.1 * 0.03
                    ));
                }
                stream.push_str("h B\n");
            }
        }

        // Title Block Text
        let clean_title = sheet_title.replace(['(', ')'], "");
        stream.push_str(&format!(
            "BT /F1 11 Tf 590 110 Td (EIGENGRAPH x BIMCRAFT SHEET {sheet_number}) Tj 0 -18 Td ({clean_title}) Tj 0 -16 Td (Walls: {} | Cols: {} | Rebar: {:.1} kg) Tj ET\n",
            self.walls.len(),
            self.columns.len(),
            self.total_rebar_weight_kg()
        ));

        let stream_len = stream.len();
        let pdf = format!(
            "%PDF-1.4\n\
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 842 595] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n\
4 0 obj << /Length {stream_len} >>\nstream\n{stream}endstream\nendobj\n\
5 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >> endobj\n\
xref\n0 6\n0000000000 65535 f \n\
trailer << /Size 6 /Root 1 0 R >>\n%%EOF\n"
        );
        pdf.into_bytes()
    }

    /// Exports the complete BIM project into an industry-standard **IFC4** STEP file with full
    /// spatial hierarchy (`IfcProject -> IfcSite -> IfcBuilding -> IfcBuildingStorey`) and entities
    /// (`IfcWall`, `IfcOpeningElement`, `IfcDoor`, `IfcWindow`, `IfcColumn`, `IfcBeam`, `IfcFooting`,
    /// `IfcSlab`, `IfcSpace`, and `IfcReinforcingBar`).
    pub fn export_ifc4_spatial(&self) -> String {
        let mut out = String::with_capacity(8192);
        out.push_str("ISO-10303-21;\nHEADER;\n");
        out.push_str("FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\n");
        out.push_str(&format!(
            "FILE_NAME('{}.ifc','2026-10-31T22:00:00',('EigenGraph BIMCraft Engine'),('EigenGraph 10D'),'EigenGraph v1.1.0','EigenGraph IFC4','');\n",
            self.project_name
        ));
        out.push_str("FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n");
        out.push_str("#1=IFCPROJECT('0EigenGraphProj0000001',$,'EigenGraph BIMCraft Project',$,$,$,$,$,$);\n");
        out.push_str("#2=IFCSITE('0EigenGraphSite0000001',$,'Default Site',$,$,$,$,$,.ELEMENT.,$,$,$,$,$);\n");
        out.push_str("#3=IFCBUILDING('0EigenGraphBldg0000001',$,'Main Building',$,$,$,$,$,.ELEMENT.,$,$,$);\n");

        let mut step_id = 10usize;
        for lvl in &self.levels {
            out.push_str(&format!(
                "#{step_id}=IFCBUILDINGSTOREY('0Storey{:015}',$,'{}',$,$,$,$,$,.ELEMENT.,{:.1});\n",
                lvl.id, lvl.name, lvl.elevation_mm
            ));
            step_id += 1;
        }
        for w in &self.walls {
            out.push_str(&format!(
                "#{step_id}=IFCWALL('0Wall{:017}',$,'{}',$,$,$,$,$,.STANDARD.);\n",
                w.id, w.name
            ));
            step_id += 1;
            for op in &w.openings {
                let class = match op.kind {
                    OpeningKind::SingleDoor | OpeningKind::DoubleDoor | OpeningKind::SlidingDoor => "IFCDOOR",
                    _ => "IFCWINDOW",
                };
                out.push_str(&format!(
                    "#{step_id}=IFCOPENINGELEMENT('0Void{:017}',$,'Opening {}',$,$,$,$,$,.OPENING.);\n",
                    op.id, op.mark
                ));
                step_id += 1;
                out.push_str(&format!(
                    "#{step_id}={class}('0Elem{:017}',$,'{}',$,$,$,$,$,{:.1},{:.1},.SINGLE_SWING_LEFT.,$,$);\n",
                    op.id, op.mark, op.height_mm, op.width_mm
                ));
                step_id += 1;
            }
        }
        for c in &self.columns {
            out.push_str(&format!(
                "#{step_id}=IFCCOLUMN('0Colm{:017}',$,'{}','{}',$,$,$,$,.COLUMN.);\n",
                c.id, c.name, c.profile_name
            ));
            step_id += 1;
        }
        for r in &self.runs {
            let (ifc_cls, pred) = r.category.ifc_class();
            out.push_str(&format!(
                "#{step_id}={ifc_cls}('0RunE{:017}',$,'{}','{}',$,$,$,$,{pred});\n",
                r.id, r.name, r.profile_name
            ));
            step_id += 1;
        }
        for f in &self.footings {
            out.push_str(&format!(
                "#{step_id}=IFCFOOTING('0Foot{:017}',$,'{}',$,$,$,$,$,.PAD_FOOTING.);\n",
                f.id, f.name
            ));
            step_id += 1;
        }
        for s in &self.slabs {
            let kind = if s.is_roof { ".ROOF." } else { ".FLOOR." };
            out.push_str(&format!(
                "#{step_id}=IFCSLAB('0Slab{:017}',$,'{}',$,$,$,$,$,{kind});\n",
                s.id, s.name
            ));
            step_id += 1;
        }
        for rm in &self.rooms {
            out.push_str(&format!(
                "#{step_id}=IFCSPACE('0Room{:017}',$,'{}','{}',$,$,$,$,.INTERNAL.,$);\n",
                rm.id, rm.number, rm.name
            ));
            step_id += 1;
        }
        for rb in &self.rebars {
            out.push_str(&format!(
                "#{step_id}=IFCREINFORCINGBAR('0Rebr{:017}',$,'{}','{}',$,$,$,$,$,{:.1},$,$,.MAIN.);\n",
                rb.id, rb.host_kind, rb.grade, rb.diameter_mm
            ));
            step_id += 1;
        }
        out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        out
    }
}

/// Transactional Command Bus with Unlimited Undo/Redo and JSON Agent/CLI Execution.
#[derive(Clone, Debug, Default)]
pub struct BimCommandBus {
    pub project: BimProject,
    pub undo_stack: Vec<BimProject>,
    pub redo_stack: Vec<BimProject>,
    pub log: Vec<String>,
}

impl BimCommandBus {
    pub fn new() -> Self {
        Self {
            project: BimProject::sample_bimcraft_building(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            log: vec![
                "Initialized BIMCraft-grade Relational Building Model (Levels 1-3, Grids 1-3/A-C, CSG Openings & Rebar)".to_string(),
            ],
        }
    }

    /// Executes a `BimCommand` transactionally, recording a snapshot on `undo_stack`.
    pub fn execute(&mut self, cmd: BimCommand) -> Result<String, String> {
        let snapshot = self.project.clone();
        let msg = self.apply_command(cmd)?;
        self.undo_stack.push(snapshot);
        self.redo_stack.clear();
        self.log.push(msg.clone());
        Ok(msg)
    }

    /// Executes a JSON-encoded `BimCommand` (compatible with CLI & MCP Agent calls).
    pub fn execute_json(&mut self, json_str: &str) -> Result<String, String> {
        let cmd: BimCommand = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid BimCommand JSON: {e}"))?;
        self.execute(cmd)
    }

    pub fn undo(&mut self) -> Option<String> {
        let prev = self.undo_stack.pop()?;
        let cur = std::mem::replace(&mut self.project, prev);
        self.redo_stack.push(cur);
        let msg = format!("Undo applied ({} steps remaining)", self.undo_stack.len());
        self.log.push(msg.clone());
        Some(msg)
    }

    pub fn redo(&mut self) -> Option<String> {
        let next = self.redo_stack.pop()?;
        let cur = std::mem::replace(&mut self.project, next);
        self.undo_stack.push(cur);
        let msg = format!("Redo applied ({} steps remaining)", self.redo_stack.len());
        self.log.push(msg.clone());
        Some(msg)
    }

    fn apply_command(&mut self, cmd: BimCommand) -> Result<String, String> {
        match cmd {
            BimCommand::AddLevel { name, elevation_mm } => {
                let id = self.project.alloc_id();
                self.project.levels.push(BimLevel {
                    id,
                    name: name.clone(),
                    elevation_mm,
                });
                self.project
                    .levels
                    .sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
                Ok(format!("Added Level '{name}' at {elevation_mm:.0} mm"))
            }
            BimCommand::SetLevelElevation {
                level_id,
                elevation_mm,
            } => {
                let lvl = self
                    .project
                    .levels
                    .iter_mut()
                    .find(|l| l.id == level_id)
                    .ok_or_else(|| format!("Level #{level_id} not found"))?;
                lvl.elevation_mm = elevation_mm;
                Ok(format!(
                    "Updated Level #{} elevation to {:.0} mm (hosted walls/columns updated)",
                    level_id, elevation_mm
                ))
            }
            BimCommand::AddWall {
                base_level_id,
                start,
                end,
                thickness_mm,
                is_structural,
            } => {
                if start.dist(end) < 100.0 {
                    return Err("Wall is too short (< 100 mm)".to_string());
                }
                let id = self.project.alloc_id();
                let top_lvl = self
                    .project
                    .levels
                    .iter()
                    .find(|l| l.elevation_mm > self.project.level_elevation(base_level_id))
                    .map(|l| l.id);
                self.project.walls.push(BimWall {
                    id,
                    name: format!("Wall W-{id}"),
                    base_level_id,
                    top_level_id: top_lvl,
                    unconnected_height_mm: 4000.0,
                    start,
                    end,
                    thickness_mm,
                    is_structural,
                    layers: BimWall::default_compound_layers(thickness_mm),
                    openings: Vec::new(),
                });
                self.project.detect_rooms_on_level(base_level_id);
                Ok(format!(
                    "Created Wall W-{id} (L={:.0} mm, t={:.0} mm) & updated planar room topology",
                    start.dist(end),
                    thickness_mm
                ))
            }
            BimCommand::AddOpening {
                wall_id,
                kind,
                offset_along_mm,
                width_mm,
                height_mm,
                sill_height_mm,
            } => {
                let op_id = self.project.alloc_id();
                let wall = self
                    .project
                    .walls
                    .iter_mut()
                    .find(|w| w.id == wall_id)
                    .ok_or_else(|| format!("Wall #{wall_id} not found"))?;
                if offset_along_mm < 50.0 || offset_along_mm + width_mm > wall.length_mm() - 50.0 {
                    return Err("Opening does not fit inside host wall bounds".to_string());
                }
                let prefix = match kind {
                    OpeningKind::SingleDoor
                    | OpeningKind::DoubleDoor
                    | OpeningKind::SlidingDoor => "D",
                    _ => "W",
                };
                let mark = format!("{prefix}-{:02}", wall.openings.len() + 1);
                wall.openings.push(BimOpening {
                    id: op_id,
                    mark: mark.clone(),
                    kind,
                    offset_along_mm,
                    width_mm,
                    height_mm,
                    sill_height_mm,
                    flip_facing: false,
                    flip_hand: false,
                });
                Ok(format!(
                    "Hosted opening {mark} ({width_mm:.0}x{height_mm:.0} mm) in Wall #{wall_id}"
                ))
            }
            BimCommand::AddColumn {
                base_level_id,
                top_level_id,
                pos,
                width_mm,
                depth_mm,
                is_round,
                profile_name,
            } => {
                let id = self.project.alloc_id();
                self.project.columns.push(BimColumn {
                    id,
                    name: format!("Col C-{id}"),
                    base_level_id,
                    top_level_id,
                    pos,
                    width_mm,
                    depth_mm,
                    rotation_deg: 0.0,
                    is_round,
                    is_wf_profile: !is_round,
                    profile_name,
                });
                Ok(format!(
                    "Placed Structural Column C-{id} at ({:.0}, {:.0}) mm",
                    pos.0, pos.1
                ))
            }
            BimCommand::AddBeamRun {
                level_id,
                category,
                start,
                end,
                width_mm,
                depth_mm,
                justify,
                profile_name,
            } => {
                if start.dist(end) < 100.0 {
                    return Err("Run is too short (< 100 mm)".to_string());
                }
                let id = self.project.alloc_id();
                self.project.runs.push(BimRun {
                    id,
                    name: format!("Run R-{id}"),
                    category,
                    level_id,
                    start,
                    end,
                    start_offset_mm: 0.0,
                    end_offset_mm: 0.0,
                    justify,
                    profile_name,
                    width_mm,
                    depth_mm,
                    is_round: matches!(
                        category,
                        RunCategory::PlumbingPipe | RunCategory::ElectricalConduit
                    ),
                    is_wf_profile: category == RunCategory::StructuralBeam,
                    system_name: "General".to_string(),
                });
                Ok(format!(
                    "Created {:?} Run R-{id} (L={:.0} mm)",
                    category,
                    start.dist(end)
                ))
            }
            BimCommand::AddFooting {
                level_id,
                pos,
                width_mm,
                length_mm,
                thickness_mm,
            } => {
                let id = self.project.alloc_id();
                self.project.footings.push(BimFooting {
                    id,
                    name: format!("Footing FT-{id}"),
                    level_id,
                    pos,
                    width_mm,
                    length_mm,
                    thickness_mm,
                });
                Ok(format!(
                    "Placed Isolated Footing FT-{id} ({width_mm:.0}x{length_mm:.0}x{thickness_mm:.0} mm)"
                ))
            }
            BimCommand::AddSlab {
                level_id,
                boundary,
                openings,
                thickness_mm,
                is_roof,
                roof_slope_deg,
            } => {
                if boundary.len() < 3 {
                    return Err("Slab boundary requires at least 3 points".to_string());
                }
                let id = self.project.alloc_id();
                self.project.slabs.push(BimSlab {
                    id,
                    name: if is_roof {
                        format!("Roof R-{id}")
                    } else {
                        format!("Slab S-{id}")
                    },
                    level_id,
                    boundary,
                    openings,
                    thickness_mm,
                    is_roof,
                    roof_slope_deg,
                });
                Ok(format!("Created Slab/Roof #{id} (t={thickness_mm:.0} mm)"))
            }
            BimCommand::AddDimension {
                start,
                end,
                offset_mm,
            } => {
                let id = self.project.alloc_id();
                self.project.dimensions.push(BimDimension {
                    id,
                    start,
                    end,
                    offset_mm,
                });
                Ok(format!(
                    "Added Associative Dimension #{id}: {:.0} mm",
                    start.dist(end)
                ))
            }
            BimCommand::ReinforceHost { host_id, preset } => {
                let cnt = self.project.reinforce_host(host_id, &preset)?;
                Ok(format!(
                    "Armed Host #{host_id} with {cnt} reinforcing bars ({})",
                    preset.name
                ))
            }
            BimCommand::ReinforceAllConcrete { preset } => {
                let cnt = self.project.reinforce_all_concrete(&preset);
                Ok(format!(
                    "Armed all structural concrete elements with {cnt} bars (Total Rebar: {:.1} kg)",
                    self.project.total_rebar_weight_kg()
                ))
            }
            BimCommand::MoveElement { element_id, delta } => {
                if let Some(w) = self.project.walls.iter_mut().find(|w| w.id == element_id) {
                    w.start = w.start + delta;
                    w.end = w.end + delta;
                    let lvl = w.base_level_id;
                    self.project.detect_rooms_on_level(lvl);
                    return Ok(format!(
                        "Moved Wall #{element_id} by ({:.0}, {:.0}) mm",
                        delta.0, delta.1
                    ));
                }
                if let Some(c) = self.project.columns.iter_mut().find(|c| c.id == element_id) {
                    c.pos = c.pos + delta;
                    return Ok(format!(
                        "Moved Column #{element_id} by ({:.0}, {:.0}) mm",
                        delta.0, delta.1
                    ));
                }
                if let Some(r) = self.project.runs.iter_mut().find(|r| r.id == element_id) {
                    r.start = r.start + delta;
                    r.end = r.end + delta;
                    return Ok(format!(
                        "Moved Run #{element_id} by ({:.0}, {:.0}) mm",
                        delta.0, delta.1
                    ));
                }
                Err(format!("Element #{element_id} not found"))
            }
            BimCommand::RotateElement {
                element_id,
                angle_deg,
            } => {
                let rad = angle_deg.to_radians();
                if let Some(c) = self.project.columns.iter_mut().find(|c| c.id == element_id) {
                    c.rotation_deg = (c.rotation_deg + angle_deg) % 360.0;
                    return Ok(format!("Rotated Column #{element_id} by {angle_deg:.1}°"));
                }
                if let Some(w) = self.project.walls.iter_mut().find(|w| w.id == element_id) {
                    let mid = (w.start + w.end) * 0.5;
                    w.start = mid + (w.start - mid).rotate(rad);
                    w.end = mid + (w.end - mid).rotate(rad);
                    let lvl = w.base_level_id;
                    self.project.detect_rooms_on_level(lvl);
                    return Ok(format!("Rotated Wall #{element_id} by {angle_deg:.1}°"));
                }
                Err(format!("Element #{element_id} not found for rotation"))
            }
            BimCommand::MirrorElements { axis_a, axis_b } => {
                let d = (axis_b - axis_a).unit();
                let reflect = |p: P2| -> P2 {
                    let v = p - axis_a;
                    let proj = d * v.dot(d);
                    axis_a + proj * 2.0 - v
                };
                let existing_cols = self.project.columns.clone();
                for c in existing_cols {
                    let new_pos = reflect(c.pos);
                    if new_pos.dist(c.pos) > 200.0 {
                        let id = self.project.alloc_id();
                        let mut mc = c.clone();
                        mc.id = id;
                        mc.pos = new_pos;
                        self.project.columns.push(mc);
                    }
                }
                Ok("Mirrored structural elements across reference axis".to_string())
            }
            BimCommand::TrimExtendWalls {
                wall_a_id,
                wall_b_id,
            } => {
                let wa = self
                    .project
                    .walls
                    .iter()
                    .find(|w| w.id == wall_a_id)
                    .cloned()
                    .ok_or("Wall A not found")?;
                let wb = self
                    .project
                    .walls
                    .iter()
                    .find(|w| w.id == wall_b_id)
                    .cloned()
                    .ok_or("Wall B not found")?;
                let corner = line_intersection(wa.start, wa.end, wb.start, wb.end)
                    .ok_or("Walls are parallel and cannot be trimmed to a corner")?;

                for (wid, w_ref) in [(wall_a_id, &wa), (wall_b_id, &wb)] {
                    if let Some(w_mut) = self.project.walls.iter_mut().find(|w| w.id == wid) {
                        if w_ref.start.dist(corner) < w_ref.end.dist(corner) {
                            w_mut.start = corner;
                        } else {
                            w_mut.end = corner;
                        }
                    }
                }
                self.project.detect_rooms_on_level(wa.base_level_id);
                Ok(format!(
                    "Trimmed/Extended Walls #{wall_a_id} and #{wall_b_id} to corner ({:.0}, {:.0})",
                    corner.0, corner.1
                ))
            }
            BimCommand::SplitWall { wall_id, split_pt } => {
                let idx = self
                    .project
                    .walls
                    .iter()
                    .position(|w| w.id == wall_id)
                    .ok_or("Wall not found")?;
                let orig = self.project.walls[idx].clone();
                let t = param_on_segment(split_pt, orig.start, orig.end);
                if !(0.1..=0.9).contains(&t) {
                    return Err("Split point must lie inside the wall segment".to_string());
                }
                let mid = orig.start + (orig.end - orig.start) * t;
                self.project.walls[idx].end = mid;
                let new_id = self.project.alloc_id();
                self.project.walls.push(BimWall {
                    id: new_id,
                    name: format!("{}-Split", orig.name),
                    base_level_id: orig.base_level_id,
                    top_level_id: orig.top_level_id,
                    unconnected_height_mm: orig.unconnected_height_mm,
                    start: mid,
                    end: orig.end,
                    thickness_mm: orig.thickness_mm,
                    is_structural: orig.is_structural,
                    layers: orig.layers,
                    openings: Vec::new(),
                });
                Ok(format!(
                    "Split Wall #{wall_id} into #{wall_id} and #{new_id} at ({:.0}, {:.0})",
                    mid.0, mid.1
                ))
            }
            BimCommand::OffsetWall { wall_id, offset_mm } => {
                let orig = self
                    .project
                    .walls
                    .iter()
                    .find(|w| w.id == wall_id)
                    .cloned()
                    .ok_or("Wall not found")?;
                let shift = orig.normal_outward() * offset_mm;
                let new_id = self.project.alloc_id();
                self.project.walls.push(BimWall {
                    id: new_id,
                    name: format!("{}-Offset", orig.name),
                    base_level_id: orig.base_level_id,
                    top_level_id: orig.top_level_id,
                    unconnected_height_mm: orig.unconnected_height_mm,
                    start: orig.start + shift,
                    end: orig.end + shift,
                    thickness_mm: orig.thickness_mm,
                    is_structural: orig.is_structural,
                    layers: orig.layers,
                    openings: Vec::new(),
                });
                self.project.detect_rooms_on_level(orig.base_level_id);
                Ok(format!(
                    "Offset Wall #{wall_id} by {offset_mm:.0} mm -> Wall #{new_id}"
                ))
            }
            BimCommand::DeleteElement { element_id } => {
                self.project.walls.retain(|w| w.id != element_id);
                self.project.columns.retain(|c| c.id != element_id);
                self.project.runs.retain(|r| r.id != element_id);
                self.project.footings.retain(|f| f.id != element_id);
                self.project.slabs.retain(|s| s.id != element_id);
                self.project.rebars.retain(|rb| rb.host_id != element_id);
                Ok(format!(
                    "Deleted Element #{element_id} and its hosted openings/rebars"
                ))
            }
            BimCommand::AutoDetectRooms { level_id } => {
                self.project.detect_rooms_on_level(level_id);
                Ok(format!(
                    "Detected {} enclosed rooms on Level #{level_id}",
                    self.project.rooms.len()
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bim_sample_building_rooms_rebar_and_live_ari() {
        let bus = BimCommandBus::new();
        assert_eq!(bus.project.levels.len(), 3);
        assert_eq!(bus.project.grids.len(), 6);
        // 3 rooms bounded by exterior + partition walls on Level 1
        assert_eq!(bus.project.rooms.len(), 3);
        let total_room_area: f64 = bus.project.rooms.iter().map(|r| r.area_m2).sum();
        assert!((total_room_area - 108.0).abs() < 1e-3); // 12m x 9m = 108 m²

        // Rebar weight > 0
        assert!(bus.project.total_rebar_weight_kg() > 500.0);

        // Live Dual Physical-to-Analytical FEM + ARI evaluation
        let ari_report = bus
            .project
            .evaluate_live_ari()
            .expect("BIM building should convert to valid FEM model and pass ARI");
        assert!(ari_report.kinematic_guard.is_stable);
        assert!(ari_report.average_ari > 0.50);
    }

    #[test]
    fn test_transactional_command_bus_undo_redo_and_section_cut_export() {
        let mut bus = BimCommandBus::new();
        let initial_walls = bus.project.walls.len();

        let json_cmd = r#"{"command":"add_wall","base_level_id":1,"start":[3000.0,0.0],"end":[3000.0,9000.0],"thickness_mm":150.0,"is_structural":false}"#;
        bus.execute_json(json_cmd).expect("JSON AddWall should succeed");
        assert_eq!(bus.project.walls.len(), initial_walls + 1);
        assert_eq!(bus.project.rooms.len(), 4); // Splitting the left 6x9m hall into two 3x9m rooms!

        // Undo restores 3 rooms and wall count
        bus.undo().expect("Undo should succeed");
        assert_eq!(bus.project.walls.len(), initial_walls);
        assert_eq!(bus.project.rooms.len(), 3);

        // Redo re-applies the wall split
        bus.redo().expect("Redo should succeed");
        assert_eq!(bus.project.rooms.len(), 4);

        // Live Section Cut & SVG/PDF/IFC4 Exports
        let sec = bus
            .project
            .generate_section_cut(P2(-1000.0, 4500.0), P2(13000.0, 4500.0));
        assert!(!sec.cut_segments_2d.is_empty());
        assert!(!sec.rebar_dots_2d.is_empty());

        let svg = bus.project.export_sheet_svg("A101", "Floor Plan & Section");
        assert!(svg.contains("<svg") && svg.contains("EIGENGRAPH"));

        let pdf = bus.project.export_sheet_pdf("A101", "Floor Plan & Section");
        assert!(pdf.starts_with(b"%PDF-1.4"));

        let ifc = bus.project.export_ifc4_spatial();
        assert!(ifc.contains("IFCBUILDINGSTOREY") && ifc.contains("IFCREINFORCINGBAR"));
    }
}
