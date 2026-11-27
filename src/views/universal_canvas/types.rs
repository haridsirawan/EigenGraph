//! Domain enums, entity types, archives, and cached render state for Universal Canvas.

use serde::{Deserialize, Serialize};

use crate::core::gis::earth_globe::{EarthBrushAction, MegastructureKind, PlanetaryWorldState};
use crate::core::{BimProject, LifecycleOpsState, Sha256Hasher};
use crate::icons::AppIcon;
use crate::models::{ParametricVariable, StructuralMember};
use crate::theme::ThemeMode;

/// Object Snap geometry mode detected near cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OsnapKind {
    #[default]
    None,
    Grid,
    Endpoint,
    Midpoint,
    Perpendicular,
    Intersection,
}

impl OsnapKind {
    pub fn short_tag(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Grid => "GRID",
            Self::Endpoint => "END",
            Self::Midpoint => "MID",
            Self::Perpendicular => "PERP",
            Self::Intersection => "INT",
        }
    }
}

/// Interactive drafting and markup tools available across every sidebar module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UniversalCanvasTool {
    SelectPan,
    BoxSelect,
    Freehand,
    Line,
    Rectangle,
    Circle,
    Dimension,
    Callout,
    Eraser,
    // Phase 43: EigenEarth 3D Globe Terraforming & Megaproject Tools
    EarthUplift,
    EarthCarve,
    EarthReclaim,
    EarthForest,
    EarthSolar,
    EarthLake,
    EarthSeawall,
    EarthBridge,
    EarthTower,
    EarthCorridor,
}

impl UniversalCanvasTool {
    pub const ALL: [Self; 9] = [
        Self::SelectPan,
        Self::BoxSelect,
        Self::Freehand,
        Self::Line,
        Self::Rectangle,
        Self::Circle,
        Self::Dimension,
        Self::Callout,
        Self::Eraser,
    ];

    pub const EARTH_TOOLS: [Self; 10] = [
        Self::EarthUplift,
        Self::EarthCarve,
        Self::EarthReclaim,
        Self::EarthForest,
        Self::EarthSolar,
        Self::EarthLake,
        Self::EarthSeawall,
        Self::EarthBridge,
        Self::EarthTower,
        Self::EarthCorridor,
    ];

    pub fn earth_brush_action(self) -> Option<EarthBrushAction> {
        match self {
            Self::EarthUplift => Some(EarthBrushAction::UpliftMountain),
            Self::EarthCarve => Some(EarthBrushAction::CarveCanal),
            Self::EarthReclaim => Some(EarthBrushAction::ReclaimLand),
            Self::EarthForest => Some(EarthBrushAction::PlantForest),
            Self::EarthSolar => Some(EarthBrushAction::BuildSolarHub),
            Self::EarthLake => Some(EarthBrushAction::CarveLake),
            Self::EarthSeawall => Some(EarthBrushAction::BuildSeawall),
            _ => None,
        }
    }

    pub fn earth_megastructure_kind(self) -> Option<MegastructureKind> {
        match self {
            Self::EarthBridge => Some(MegastructureKind::TransStraitBridge),
            Self::EarthTower => Some(MegastructureKind::ArcologyMegaTower),
            Self::EarthCorridor => Some(MegastructureKind::SolarCleanCorridor),
            _ => None,
        }
    }

    pub fn is_earth_tool(self) -> bool {
        self.earth_brush_action().is_some() || self.earth_megastructure_kind().is_some()
    }

    pub fn app_icon(self) -> AppIcon {
        match self {
            Self::SelectPan => AppIcon::CursorPan,
            Self::BoxSelect => AppIcon::SelectArrow,
            Self::Freehand => AppIcon::Pencil,
            Self::Line => AppIcon::LineSeg,
            Self::Rectangle => AppIcon::RectShape,
            Self::Circle => AppIcon::CircleShape,
            Self::Dimension => AppIcon::DimensionRuler,
            Self::Callout => AppIcon::CommentNote,
            Self::Eraser => AppIcon::Eraser,
            Self::EarthUplift => AppIcon::Mountain,
            Self::EarthCarve => AppIcon::Waves,
            Self::EarthReclaim => AppIcon::Slab,
            Self::EarthForest => AppIcon::Trees,
            Self::EarthSolar => AppIcon::Sun,
            Self::EarthLake => AppIcon::Waves,
            Self::EarthSeawall => AppIcon::Shield,
            Self::EarthBridge => AppIcon::BridgeSpan,
            Self::EarthTower => AppIcon::Building,
            Self::EarthCorridor => AppIcon::Bolt,
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::SelectPan => "Pan",
            Self::BoxSelect => "BoxSel",
            Self::Freehand => "Draw",
            Self::Line => "Line",
            Self::Rectangle => "Rect",
            Self::Circle => "Circle",
            Self::Dimension => "Dim",
            Self::Callout => "Note",
            Self::Eraser => "Erase",
            Self::EarthUplift => "Uplift",
            Self::EarthCarve => "Carve",
            Self::EarthReclaim => "Reclaim",
            Self::EarthForest => "Forest",
            Self::EarthSolar => "Solar",
            Self::EarthLake => "Lake",
            Self::EarthSeawall => "Seawall",
            Self::EarthBridge => "Bridge",
            Self::EarthTower => "Tower",
            Self::EarthCorridor => "CleanGrid",
        }
    }

    pub fn tooltip(self) -> &'static str {
        match self {
            Self::SelectPan => "Orbit 3D Earth / Pan 2D canvas with drag (or right-drag in any tool); click to select",
            Self::BoxSelect => "Marquee select: drag Left->Right (Blue Window enclosed) or Right->Left (Green Crossing) [V]",
            Self::Freehand => "Freehand engineering sketch synced across all 15 modules",
            Self::Line => "Draw straight/geodesic line segment (click-drag)",
            Self::Rectangle => "Draw rectangular zone / bay boundary (click-drag)",
            Self::Circle => "Draw radial envelope / circle (drag center to radius)",
            Self::Dimension => "Measure and pin world distance in millimetres",
            Self::Callout => "Click canvas to place text annotation callout",
            Self::Eraser => "Click any user-drawn object to remove it",
            Self::EarthUplift => "Terraform Brush: Uplift mountains, plateaus & coastal dikes on 3D Earth",
            Self::EarthCarve => "Terraform Brush: Carve valleys, sea canals & trenches into Earth terrain",
            Self::EarthReclaim => "Terraform Brush: Reclaim shallow ocean seabed into habitable protected land",
            Self::EarthForest => "Biome Brush: Plant continental Mega-Forest belt (-CO2, cools Earth)",
            Self::EarthSolar => "Energy Brush: Build utility Solar Hub array (+GW clean power, -CO2)",
            Self::EarthLake => "Hydro Brush: Excavate inland Freshwater Lake / Pumped-Hydro Reservoir",
            Self::EarthSeawall => "Defense Brush: Fortify coastline with Flood Seawall barrier against rising seas",
            Self::EarthBridge => "Megaproject Span: Drag across straits/oceans to build curved Great-Circle Mega-Bridge",
            Self::EarthTower => "Megaproject Beacon: Click Earth to erect 3D Arcology Mega-Tower along zenith normal",
            Self::EarthCorridor => "Clean Energy Span: Drag across continents to deploy HVDC Solar Clean Corridor",
        }
    }
}

/// Geometry primitive stored in world coordinates (millimetres) and synchronized across all modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CanvasShapeKind {
    Freehand { points_mm: Vec<[f64; 2]> },
    Line { start_mm: [f64; 2], end_mm: [f64; 2] },
    Rectangle { min_mm: [f64; 2], max_mm: [f64; 2] },
    Circle { center_mm: [f64; 2], radius_mm: f64 },
    Dimension { start_mm: [f64; 2], end_mm: [f64; 2] },
    Callout { pos_mm: [f64; 2], text: String },
}

/// Synchronized drawing entity tagged with its originating module layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasEntity {
    pub id: usize,
    pub pillar_badge: String,
    pub pillar_name: String,
    pub kind: CanvasShapeKind,
    pub color_rgb: [u8; 3],
    pub stroke_width: f32,
}

/// Workspace and canvas preferences persisted inside the single-file archive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceCanvasSettings {
    pub theme_mode: ThemeMode,
    pub canvas_height_ratio: f32,
    pub freeze_by_default: bool,
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub grid_spacing_mm: f64,
    pub show_bim_base_layer: bool,
    pub show_domain_overlay: bool,
    pub sync_all_layers: bool,
    pub show_floating_inspector: bool,
    pub show_ai_copilot: bool,
    pub unit_system: String,
    pub design_standard: String,
    pub solver_threads: usize,
    pub enable_svd_guard: bool,
    pub enable_anti_leakage: bool,
}

/// Unified single-file `.eigen.json` workspace archive synchronizing BIM, structural analysis,
/// parametric variables, all 15-module canvas drawings, and workspace settings with SHA-256 verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedWorkspaceArchive {
    pub format_version: String,
    pub engine_version: String,
    pub project_name: String,
    pub bim_project: BimProject,
    pub members: Vec<StructuralMember>,
    pub parametric_variables: Vec<ParametricVariable>,
    pub canvas_entities: Vec<CanvasEntity>,
    pub workspace_settings: WorkspaceCanvasSettings,
    #[serde(default)]
    pub world_origin_lat_lon: Option<(f64, f64)>,
    #[serde(default)]
    pub map_center_lat_lon: Option<(f64, f64)>,
    #[serde(default)]
    pub map_zoom: Option<f32>,
    #[serde(default = "default_globe_true")]
    pub globe_mode: bool,
    #[serde(default)]
    pub planetary_world: PlanetaryWorldState,
    #[serde(default)]
    pub lifecycle_ops: LifecycleOpsState,
    #[serde(default)]
    pub city_scene: crate::core::gis::IfcxCityScene,
    #[serde(default)]
    pub district_hazard: crate::core::gis::DistrictHazardConfig,
    #[serde(default)]
    pub subsurface_ssi: crate::core::geotech::SubsurfaceSsiConfig,
    #[serde(default)]
    pub portfolio_retrofit: crate::core::portfolio_retrofit::PortfolioRetrofitConfig,
    #[serde(default)]
    pub ground_motion_replay: crate::core::ground_motion_replay::GroundMotionReplayConfig,
    #[serde(default)]
    pub shm_digital_twin: crate::core::shm_twin::ShmDigitalTwinConfig,
    #[serde(default)]
    pub evacuation_routing: crate::core::evacuation_routing::EvacuationRoutingConfig,
    #[serde(default)]
    pub shm_watchdog: crate::core::shm_watchdog::ShmWatchdogConfig,
    #[serde(default)]
    pub monte_carlo_config: crate::core::gpu_fragility_mc::MonteCarloFragilityConfig,
    #[serde(default)]
    pub collab_config: crate::core::ifcx_collab::IfcxCollabConfig,
    #[serde(default)]
    pub trade_study_config: crate::core::trade_study::TradeStudyConfig,
    #[serde(default)]
    pub eigenflow_config: crate::core::eigenflow::EigenFlowConfig,
    #[serde(default)]
    pub extreme_events_config: crate::core::extreme_events::ExtremeEventsConfig,
    #[serde(default)]
    pub isolation_studio_config: crate::core::base_isolation_studio::IsolationStudioConfig,
    #[serde(default)]
    pub wind_studio_config: crate::core::wind_studio::WindStudioConfig,
    #[serde(default)]
    pub shear_wall_studio_config: crate::core::shear_wall_studio::ShearWallStudioConfig,
    #[serde(default)]
    pub connection_studio_config: crate::core::connection_studio::ConnectionStudioConfig,
    #[serde(default)]
    pub prestress_studio_config: crate::core::prestress_studio::PrestressStudioConfig,
    #[serde(default)]
    pub composite_studio_config: crate::core::composite_studio::CompositeStudioConfig,
    #[serde(default)]
    pub reliability_studio_config: crate::core::reliability_studio::ReliabilityStudioConfig,
    #[serde(default)]
    pub foundation_studio_config: crate::core::foundation_studio::FoundationStudioConfig,
    #[serde(default)]
    pub construction_lca_config: crate::core::construction_lca_studio::ConstructionLcaConfig,
    #[serde(default)]
    pub zoning_config: crate::core::zoning_agent::SiteZoningConfig,
    #[serde(default)]
    pub program_config: crate::core::program_agent::SpatialProgramConfig,
    #[serde(default)]
    pub massing_config: crate::core::massing_agent::MassingStudyConfig,
    #[serde(default)]
    pub solar_envelope_config: crate::core::solar_envelope::SolarEnvelopeConfig,
    #[serde(default)]
    pub daylight_config: crate::core::daylight_interior::DaylightInteriorConfig,
    #[serde(default = "default_globe_true")]
    pub realistic_landscape_enabled: bool,
    #[serde(default)]
    pub sea_level_m: f32,
    #[serde(default = "default_globe_true")]
    pub show_water_surface: bool,
    #[serde(default)]
    pub terrain_sculpt: crate::core::gis::TerrainSculptConfig,
    #[serde(default)]
    pub section_box: SectionBoxConfig,
    pub sha256_seal: String,
}

/// Real-time 3D WebGPU Section Box and active-story horizontal cut plane configuration (RTC ENU meters).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionBoxConfig {
    pub enabled: bool,
    pub level_cut_mode: bool,
    pub clip_terrain_and_city: bool,
    pub show_wireframe: bool,
    pub min_enu_m: [f32; 3],
    pub max_enu_m: [f32; 3],
    pub cut_plane_offset_m: f32,
}

impl Default for SectionBoxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level_cut_mode: false,
            clip_terrain_and_city: false,
            show_wireframe: true,
            min_enu_m: [-30.0, -30.0, -10.0],
            max_enu_m: [30.0, 30.0, 24.0],
            cut_plane_offset_m: 1.2,
        }
    }
}

impl SectionBoxConfig {
    /// Returns axis-ordered `(min_enu_m, max_enu_m)` with a guaranteed `>= 0.5 m` span per axis.
    pub fn normalized_bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [0.0_f32; 3];
        let mut hi = [0.0_f32; 3];
        for i in 0..3 {
            let a = self.min_enu_m[i].min(self.max_enu_m[i]);
            let mut b = self.min_enu_m[i].max(self.max_enu_m[i]);
            if b - a < 0.5 {
                b = a + 0.5;
            }
            lo[i] = a;
            hi[i] = b;
        }
        (lo, hi)
    }

    /// Fits the 3D section box around the active `BimProject` extents with `margin_m` padding.
    pub fn fit_to_bim_project(&mut self, project: &BimProject, margin_m: f32) {
        let margin = margin_m.max(1.0);
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        let mut push_xy_mm = |x_mm: f64, y_mm: f64| {
            let xm = (x_mm * 0.001) as f32;
            let ym = (y_mm * 0.001) as f32;
            min_x = min_x.min(xm);
            max_x = max_x.max(xm);
            min_y = min_y.min(ym);
            max_y = max_y.max(ym);
        };

        for c in &project.columns {
            push_xy_mm(c.pos.0, c.pos.1);
        }
        for w in &project.walls {
            push_xy_mm(w.start.0, w.start.1);
            push_xy_mm(w.end.0, w.end.1);
        }
        for r in &project.runs {
            push_xy_mm(r.start.0, r.start.1);
            push_xy_mm(r.end.0, r.end.1);
        }
        for f in &project.footings {
            push_xy_mm(f.pos.0, f.pos.1);
        }
        for s in &project.slabs {
            for p in &s.boundary {
                push_xy_mm(p.0, p.1);
            }
        }

        if !min_x.is_finite() || !min_y.is_finite() {
            self.min_enu_m = [-18.0, -18.0, -4.0];
            self.max_enu_m = [18.0, 18.0, 14.0];
            return;
        }

        let mut min_z = 0.0_f32;
        let mut max_z = 4.0_f32;
        for lvl in &project.levels {
            let zm = (lvl.elevation_mm * 0.001) as f32;
            min_z = min_z.min(zm);
            max_z = max_z.max(zm);
        }

        self.min_enu_m = [min_x - margin, min_y - margin, min_z - 3.0];
        self.max_enu_m = [max_x + margin, max_y + margin, (max_z + 2.0).max(min_z + 3.0)];
    }

    /// Configures a horizontal level cut plane just above the selected story elevation.
    pub fn apply_level_cut(&mut self, project: &BimProject, story_idx: usize) {
        self.fit_to_bim_project(project, 6.0);
        let mut lvls = project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        let elev_m = if lvls.is_empty() {
            0.0_f32
        } else {
            let idx = story_idx.min(lvls.len().saturating_sub(1));
            (lvls[idx].elevation_mm * 0.001) as f32
        };
        let cut_z = elev_m + self.cut_plane_offset_m.clamp(0.2, 4.0);
        self.min_enu_m[2] = self.min_enu_m[2].min(cut_z - 1.5);
        self.max_enu_m[2] = cut_z;
        self.enabled = true;
        self.level_cut_mode = true;
    }
}

fn default_globe_true() -> bool {
    true
}

#[derive(Serialize)]
struct UnifiedArchivePayloadRef<'a> {
    project_name: &'a str,
    bim_project: &'a BimProject,
    members: &'a [StructuralMember],
    parametric_variables: &'a [ParametricVariable],
    canvas_entities: &'a [CanvasEntity],
    workspace_settings: &'a WorkspaceCanvasSettings,
}

impl UnifiedWorkspaceArchive {
    /// Serializes the synchronized workspace into a single SHA-256 sealed JSON document.
    pub fn to_pretty_json(&self) -> Result<String, String> {
        let payload = UnifiedArchivePayloadRef {
            project_name: &self.project_name,
            bim_project: &self.bim_project,
            members: &self.members,
            parametric_variables: &self.parametric_variables,
            canvas_entities: &self.canvas_entities,
            workspace_settings: &self.workspace_settings,
        };
        let canonical = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        let seal = Sha256Hasher::digest_hex(canonical.as_bytes());
        let mut sealed = self.clone();
        sealed.sha256_seal = seal;
        serde_json::to_string_pretty(&sealed).map_err(|e| e.to_string())
    }

    /// Parses and cryptographically verifies a single-file `.eigen.json` workspace archive.
    pub fn from_verified_json(json_str: &str) -> Result<Self, String> {
        let archive: Self = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
        let payload = UnifiedArchivePayloadRef {
            project_name: &archive.project_name,
            bim_project: &archive.bim_project,
            members: &archive.members,
            parametric_variables: &archive.parametric_variables,
            canvas_entities: &archive.canvas_entities,
            workspace_settings: &archive.workspace_settings,
        };
        let canonical = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        let computed = Sha256Hasher::digest_hex(canonical.as_bytes());
        if computed != archive.sha256_seal {
            return Err(
                "SHA-256 integrity seal mismatch: unified workspace file was modified or corrupted."
                    .to_string(),
            );
        }
        Ok(archive)
    }
}

/// Optional split export payload when the user explicitly requests per-module separation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ModuleLayerSplitExport {
    pub(super) format_version: String,
    pub(super) pillar_badge: String,
    pub(super) pillar_name: String,
    pub(super) entities: Vec<CanvasEntity>,
}

pub(super) const PALETTE_SWATCHES: [(&str, [u8; 3]); 6] = [
    ("Cyan", [56, 189, 248]),
    ("Blue", [59, 130, 246]),
    ("Green", [16, 185, 129]),
    ("Amber", [245, 158, 11]),
    ("Red", [239, 68, 68]),
    ("Slate", [226, 232, 240]),
];

/// Cross-module CAD keyboard shortcut snapshot polled only when no text input has focus.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CadShortcutFeedback {
    pub toggle_freeze: bool,
    pub escape: bool,
    pub delete_selected: bool,
    pub undo: bool,
    pub redo: bool,
    pub toggle_ortho: bool,
    pub toggle_snap: bool,
    pub toggle_osnap: bool,
    pub fit_model: bool,
    pub letter_key: Option<char>,
}

/// Phase 55–56: Direct 3D BIM placement tools inside the unified Earth 3D canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EarthBimTool {
    #[default]
    None,
    Column,
    Beam,
    Wall,
    Footing,
    Slab,
    Door,
    Window,
    Road,
    Bridge,
    Zone,
}

/// Phase 55–74: Structural visualization overlay rendered directly over 3D BIM buildings on Earth 3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EarthBimLens {
    #[default]
    Clean,
    DcrHeatmap,
    EigenGateRedundancy,
    MomentStress,
    DeformedShape,
    ModalVibration,
    TimeHistoryReplay,
    ShmDigitalTwin,
    Erection4D,
}

/// Synchronized BIM authoring & visualization parameters for direct Earth 3D modeling.
#[derive(Debug, Clone)]
pub struct EarthBimAuthorConfig {
    pub active_tool: EarthBimTool,
    pub active_lens: EarthBimLens,
    pub base_level_id: usize,
    pub top_level_id: usize,
    pub isolate_active_story: bool,
    pub section_cut_active: bool,
    pub section_cut_elev_m: f64,
    pub col_width_mm: f64,
    pub col_depth_mm: f64,
    pub beam_width_mm: f64,
    pub beam_depth_mm: f64,
    pub wall_thickness_mm: f64,
    pub footing_size_mm: f64,
    pub erection_day_limit: usize,
    pub deform_scale: f64,
    pub active_modal_mode: usize,
    pub modal_anim_playing: bool,
    pub pending_start_mm: Option<[f64; 2]>,
    pub pending_road_control_mm: Option<[f64; 2]>,
    pub road_elevation_offset_m: f64,
    pub hover_snap_mm: Option<[f64; 2]>,
}

impl Default for EarthBimAuthorConfig {
    fn default() -> Self {
        Self {
            active_tool: EarthBimTool::None,
            active_lens: EarthBimLens::Clean,
            base_level_id: 1,
            top_level_id: 2,
            isolate_active_story: false,
            section_cut_active: false,
            section_cut_elev_m: 6.0,
            col_width_mm: 400.0,
            col_depth_mm: 400.0,
            beam_width_mm: 300.0,
            beam_depth_mm: 600.0,
            wall_thickness_mm: 200.0,
            footing_size_mm: 1800.0,
            erection_day_limit: 999,
            deform_scale: 30.0,
            active_modal_mode: 0,
            modal_anim_playing: true,
            pending_start_mm: None,
            pending_road_control_mm: None,
            road_elevation_offset_m: 0.0,
            hover_snap_mm: None,
        }
    }
}

/// Active interactive single-member (`N-1`) disruption simulation overlay state on Earth 3D.
#[derive(Debug, Clone)]
pub struct EarthN1OverlayState {
    pub removed_kind: crate::core::BimElementKind,
    pub removed_bim_id: usize,
    pub removed_fem_id: usize,
    pub removed_name: String,
    pub report: crate::core::eigengate::N1SimulationReport,
}

/// Phase 63: O(1) cached BIM-to-FEM lookup tables, true 6-DOF displacements, dynamic eigenmodes,
/// and interactive `N-1` disruption state keyed by `BimProject::content_revision_hash`.
/// Eliminates per-frame `to_structural_model_with_map()` rebuilds on Earth 3D.
#[derive(Debug, Clone, Default)]
pub struct EarthStructuralRenderCache {
    pub bim_revision_hash: u64,
    pub bim_to_fem: std::collections::HashMap<(crate::core::BimElementKind, usize), usize>,
    pub fem_to_bim: std::collections::HashMap<usize, (crate::core::BimElementKind, usize)>,
    pub fem_to_member_idx: std::collections::HashMap<usize, usize>,
    pub node_coords_m: Vec<[f64; 3]>,
    pub static_nodal_disp_m: Vec<[f64; 3]>,
    pub max_disp_mm: f64,
    pub max_story_drift_ratio: f64,
    pub governing_combo: String,
    pub load_case_label: String,
    pub seismic_base_shear_kn: f64,
    pub wind_base_shear_kn: f64,
    pub total_gravity_kn: f64,
    pub modal_modes: Vec<crate::core::modal::ModeShape>,
    pub cached_fem_model: Option<crate::core::StructuralModel>,
    pub cached_baseline_analysis: Option<crate::core::AnalysisResult>,
    pub n1_simulation: Option<EarthN1OverlayState>,
}

impl EarthStructuralRenderCache {
    pub fn clear(&mut self) {
        self.bim_revision_hash = 0;
        self.bim_to_fem.clear();
        self.fem_to_bim.clear();
        self.fem_to_member_idx.clear();
        self.node_coords_m.clear();
        self.static_nodal_disp_m.clear();
        self.max_disp_mm = 0.0;
        self.max_story_drift_ratio = 0.0;
        self.governing_combo.clear();
        self.load_case_label.clear();
        self.seismic_base_shear_kn = 0.0;
        self.wind_base_shear_kn = 0.0;
        self.total_gravity_kn = 0.0;
        self.modal_modes.clear();
        self.cached_fem_model = None;
        self.cached_baseline_analysis = None;
        self.n1_simulation = None;
    }

    #[inline]
    pub fn fem_id(&self, kind: crate::core::BimElementKind, bim_id: usize) -> Option<usize> {
        self.bim_to_fem.get(&(kind, bim_id)).copied()
    }

    #[inline]
    pub fn bim_ref(&self, fem_id: usize) -> Option<(crate::core::BimElementKind, usize)> {
        self.fem_to_bim.get(&fem_id).copied()
    }

    #[inline]
    pub fn member_for_bim<'a>(
        &self,
        kind: crate::core::BimElementKind,
        bim_id: usize,
        members: &'a [StructuralMember],
    ) -> Option<&'a StructuralMember> {
        let fid = self.fem_id(kind, bim_id)?;
        let idx = *self.fem_to_member_idx.get(&fid)?;
        members.get(idx).filter(|m| m.id == fid)
    }

    /// Returns whether a given physical BIM element is currently virtually removed in an active `N-1` test.
    #[inline]
    pub fn is_n1_removed(&self, kind: crate::core::BimElementKind, bim_id: usize) -> bool {
        self.n1_simulation
            .as_ref()
            .is_some_and(|s| s.removed_kind == kind && s.removed_bim_id == bim_id)
    }

    /// Samples visual 3D displacement `([dx_mm, dy_mm, dz_elev_m], norm_mag_0_1)` at a BIM point
    /// `(x_mm, y_mm, elev_m)` for `DeformedShape`, `ModalVibration`, or active `N-1` disruption.
    #[allow(clippy::too_many_arguments)]
    pub fn sample_displacement_at_mm(
        &self,
        x_mm: f64,
        y_mm: f64,
        elev_m: f64,
        lens: EarthBimLens,
        deform_scale: f64,
        mode_idx: usize,
        anim_time_s: f64,
        anim_playing: bool,
    ) -> ([f64; 3], f64) {
        if self.node_coords_m.is_empty() || elev_m.abs() < 0.06 {
            return ([0.0, 0.0, 0.0], 0.0);
        }
        let n1_active = self.n1_simulation.is_some();
        let use_modal = lens == EarthBimLens::ModalVibration && !self.modal_modes.is_empty();
        let use_deformed = lens == EarthBimLens::DeformedShape || n1_active;
        if !use_modal && !use_deformed {
            return ([0.0, 0.0, 0.0], 0.0);
        }

        // FEM coordinate convention: (x = BIM x_m, y = BIM elev_m, z = BIM y_m)
        let qx = x_mm * 1e-3;
        let qy = elev_m;
        let qz = y_mm * 1e-3;

        // Locate closest FEM node (prefer same story elevation band)
        let mut best_idx = 0usize;
        let mut best_d2 = f64::INFINITY;
        for (idx, nc) in self.node_coords_m.iter().enumerate() {
            let dy_band = (nc[1] - qy).abs();
            let d2 = (nc[0] - qx).powi(2) + (dy_band * 3.0).powi(2) + (nc[2] - qz).powi(2);
            if d2 < best_d2 {
                best_d2 = d2;
                best_idx = idx;
            }
        }
        // Only apply deformation if node is within reasonable building proximity
        if best_d2 > 225.0 {
            return ([0.0, 0.0, 0.0], 0.0);
        }

        let max_visual_m = 2.4;
        if use_modal {
            let m_slot = mode_idx.min(self.modal_modes.len().saturating_sub(1));
            let mode = &self.modal_modes[m_slot];
            let raw = mode
                .nodal_deformations
                .get(best_idx)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0]);
            // Readable visual oscillation frequency mapped gently from natural frequency
            let vis_hz = (0.55 + (mode.frequency_hz.clamp(0.2, 8.0) / 8.0) * 1.15).clamp(0.55, 1.70);
            let phase = if anim_playing {
                (anim_time_s * 2.0 * std::f64::consts::PI * vis_hz).sin()
            } else {
                1.0
            };
            let amp_m = (0.18 + 0.011 * deform_scale).clamp(0.15, 1.85) * phase;
            let dx_m = (raw[0] * amp_m).clamp(-max_visual_m, max_visual_m);
            let dy_elev_m = (raw[1] * amp_m).clamp(-max_visual_m, max_visual_m);
            let dz_m = (raw[2] * amp_m).clamp(-max_visual_m, max_visual_m);
            let norm_mag = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2])
                .sqrt()
                .clamp(0.0, 1.0);
            return ([dx_m * 1000.0, dz_m * 1000.0, dy_elev_m], norm_mag);
        }

        // Static 6-DOF Deformed Shape (or post-disruption N-1 deflected shape)
        let (disp_slice, peak_mm) = if let Some(ref n1) = self.n1_simulation {
            (&n1.report.post_nodal_displacements, n1.report.post_max_disp_mm.max(1.0))
        } else {
            (&self.static_nodal_disp_m, self.max_disp_mm.max(1.0))
        };
        let raw = disp_slice.get(best_idx).copied().unwrap_or([0.0, 0.0, 0.0]);
        let eff_scale = if lens == EarthBimLens::DeformedShape {
            deform_scale.clamp(1.0, 150.0)
        } else {
            // Subtle readable deflection overlay when N-1 is active on DCR/EigenGate/Clean lenses
            (deform_scale * 0.45).clamp(4.0, 45.0)
        };
        let dx_m = (raw[0] * eff_scale).clamp(-max_visual_m, max_visual_m);
        let dy_elev_m = (raw[1] * eff_scale).clamp(-max_visual_m, max_visual_m);
        let dz_m = (raw[2] * eff_scale).clamp(-max_visual_m, max_visual_m);
        let mag_mm = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt() * 1000.0;
        let norm_mag = (mag_mm / peak_mm).clamp(0.0, 1.0);
        ([dx_m * 1000.0, dz_m * 1000.0, dy_elev_m], norm_mag)
    }
}
