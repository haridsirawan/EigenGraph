//! views/universal_canvas.rs — Unified Synchronized 15-Pillar Drawing Canvas & Single-File Project Archive.
//! Provides a canvas-first interactive 2D drafting & domain-overlay workspace across all 15 sidebar
//! navigation modules, featuring a Freeze Canvas guard for uninterrupted vertical page scrolling,
//! a floating collapsible domain inspector, and tamper-evident single-file `.eigen.json` sync.

use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use serde::{Deserialize, Serialize};

use crate::core::gis::earth_globe::{
    EarthBrushAction, GlobeScreenTransform, MegastructureKind, PlanetaryWorldState,
};
use crate::core::gis::SlippyMapState;
use crate::core::{
    compute_site_environmental_coupling, BimProject, DiskExporter, LifecycleOpsState,
    Sha256Hasher, SiteEnvironmentalCoupling,
};
use crate::icons::{self, AppIcon};
use crate::models::{NavigationPillar, ParametricVariable, StructuralMember};
use crate::theme::{MacTheme, ThemeMode};
use crate::ui_kit;

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
    pub sha256_seal: String,
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
struct ModuleLayerSplitExport {
    format_version: String,
    pillar_badge: String,
    pillar_name: String,
    entities: Vec<CanvasEntity>,
}

const PALETTE_SWATCHES: [(&str, [u8; 3]); 6] = [
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
}

/// Phase 55: Structural visualization overlay rendered directly over 3D BIM buildings on Earth 3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EarthBimLens {
    #[default]
    Clean,
    DcrHeatmap,
    AriRedundancy,
    MomentStress,
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
    pub col_width_mm: f64,
    pub col_depth_mm: f64,
    pub beam_width_mm: f64,
    pub beam_depth_mm: f64,
    pub wall_thickness_mm: f64,
    pub footing_size_mm: f64,
    pub erection_day_limit: usize,
    pub pending_start_mm: Option<[f64; 2]>,
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
            col_width_mm: 400.0,
            col_depth_mm: 400.0,
            beam_width_mm: 300.0,
            beam_depth_mm: 600.0,
            wall_thickness_mm: 200.0,
            footing_size_mm: 1800.0,
            erection_day_limit: 999,
            pending_start_mm: None,
            hover_snap_mm: None,
        }
    }
}

/// Shared interactive drawing canvas state synchronized across all 15 sidebar navigation modules.
pub struct UniversalCanvasState {
    pub pan: Vec2,
    pub zoom: f32,
    pub is_frozen: bool,
    pub active_tool: UniversalCanvasTool,
    pub active_color_idx: usize,
    pub stroke_width: f32,
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub osnap_enabled: bool,
    pub last_osnap_kind: OsnapKind,
    pub last_osnap_radius_mm: f64,
    pub ortho_mode: bool,
    pub grid_spacing_mm: f64,
    pub show_bim_base_layer: bool,
    pub show_domain_overlay: bool,
    pub sync_all_layers: bool,
    pub inspector_open: bool,
    pub canvas_height_ratio: f32,
    pub callout_draft: String,
    pub entities: Vec<CanvasEntity>,
    pub next_entity_id: usize,
    pub undo_stack: Vec<Vec<CanvasEntity>>,
    pub redo_stack: Vec<Vec<CanvasEntity>>,
    pub selected_entity_id: Option<usize>,
    pub selected_entity_ids: Vec<usize>,
    pending_start_mm: Option<[f64; 2]>,
    marquee_anchor_mm: Option<[f64; 2]>,
    marquee_current_mm: Option<[f64; 2]>,
    active_freehand_mm: Vec<[f64; 2]>,
    pub cursor_world_mm: [f64; 2],
    pub cursor_lat_lon: (f64, f64),
    pub world_map: crate::core::gis::SlippyMapState,
    pub ground_origin_lat_lon: (f64, f64),
    pub ground_origin_anchored: bool,
    pub earth_search_query: String,
    pub measure_active: bool,
    pub measure_area_mode: bool,
    pub measure_points_lat_lon: Vec<(f64, f64)>,
    last_canvas_rect: Option<Rect>,
    last_inspector_rect: Option<Rect>,
    pub status_message: Option<String>,
    pub import_json_open: bool,
    pub import_json_buffer: String,
    pub pending_imported_settings: Option<WorkspaceCanvasSettings>,
    pub request_scroll_to_workbench: bool,
    // Phase 41: Precision CAD input, transforms & bottom-toolbar Quick-Jump / Shortcuts
    pub cad_step_mm: f64,
    pub cad_rotate_deg: f64,
    pub cad_cmd_input: String,
    pub shortcuts_modal_open: bool,
    pub quick_jump_query: String,
    // Phase 43: Default 3D Google Earth Globe & Planetary Terraform / Megaproject Sandbox Game
    pub globe_mode: bool,
    pub earth_world: PlanetaryWorldState,
    pub last_globe_xform: Option<GlobeScreenTransform>,
    pub pending_earth_arc_start: Option<(f64, f64)>,
    // Phase 45: Persisted MEP sizing, Commercial Procurement/VO, Site Diary, QA/QC & Earth Site Coupling
    pub lifecycle_ops: LifecycleOpsState,
    // Phase 46: Spatial Canvas Smart Picking & Highlight Sync
    pub last_picked_bim_element: Option<(crate::core::BimElementKind, usize)>,
    pub last_picked_earth_lat_lon: Option<(f64, f64)>,
    pub highlighted_bim_element: Option<(crate::core::BimElementKind, usize)>,
    pub spatial_pick_seq: u64,
    // Phase 55: Direct BIM authoring & command emission straight onto Earth 3D
    pub earth_bim_author: EarthBimAuthorConfig,
    pub pending_earth_bim_commands: Vec<crate::core::BimCommand>,
    // Phase 57: Planetary Digital Twin Multi-Building World Registry, HLOD & Unlocated Placement
    pub world_index: crate::core::gis::WorldSpatialIndex,
    pub active_world_project_rel_path: Option<String>,
    pub placing_unlocated_rel_path: Option<String>,
    pub placing_heading_deg: f64,
    pub placing_hover_lat_lon: Option<(f64, f64)>,
    pub clicked_world_building_rel_path: Option<String>,
    pub pending_world_pin_event: Option<(String, f64, f64, f64)>,
    // Phase 58: 3D TIN Relief Overlay & Neighbor Hover Telemetry
    pub active_site_tin: Option<crate::core::gis::TinSurfaceMesh>,
    pub site_tin_preset: crate::core::gis::DemTerrainPreset,
    pub show_terrain_contours: bool,
    pub hovered_world_building_rel_path: Option<String>,
    // Phase 59: Urban Vector Streaming (OSM/GeoJSON), Live Terrarium DEM & Neighborhood Physics
    pub urban_vectors: crate::core::gis::UrbanVectorLayerCache,
    pub dem_cache: crate::core::gis::DemTileCache,
    pub use_live_dem_tiles: bool,
    pub site_tin_is_live_dem: bool,
    pub neighborhood_report: crate::core::gis::NeighborhoodAuditReport,
    pub show_neighborhood_hazards: bool,
    pub active_vector_snap_az_deg: Option<f64>,
}

/// Result of clicking a planetary cluster beacon or 3D neighbor building on Earth 3D.
#[derive(Debug, Clone, PartialEq)]
pub enum WorldPickOutcome {
    FlyToCluster { lat: f64, lon: f64, label: String },
    HydrateBuilding(String),
}

impl Default for UniversalCanvasState {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            // Phase 35: Frozen by default so vertical page scroll never zooms unintentionally; unfreeze to orbit/draw on 3D Earth
            is_frozen: true,
            active_tool: UniversalCanvasTool::SelectPan,
            active_color_idx: 0,
            stroke_width: 2.2,
            show_grid: true,
            snap_to_grid: true,
            osnap_enabled: true,
            last_osnap_kind: OsnapKind::None,
            last_osnap_radius_mm: 450.0,
            ortho_mode: false,
            grid_spacing_mm: 500.0,
            show_bim_base_layer: true,
            show_domain_overlay: true,
            sync_all_layers: true,
            inspector_open: false,
            canvas_height_ratio: 0.82,
            callout_draft: "Note".to_string(),
            entities: Vec::new(),
            next_entity_id: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selected_entity_id: None,
            selected_entity_ids: Vec::new(),
            pending_start_mm: None,
            marquee_anchor_mm: None,
            marquee_current_mm: None,
            active_freehand_mm: Vec::new(),
            cursor_world_mm: [0.0, 0.0],
            cursor_lat_lon: (5.0, 98.0),
            world_map: crate::core::gis::SlippyMapState::default(),
            ground_origin_lat_lon: (0.0, 0.0),
            ground_origin_anchored: false,
            earth_search_query: String::new(),
            measure_active: false,
            measure_area_mode: false,
            measure_points_lat_lon: Vec::new(),
            last_canvas_rect: None,
            last_inspector_rect: None,
            status_message: None,
            import_json_open: false,
            import_json_buffer: String::new(),
            pending_imported_settings: None,
            request_scroll_to_workbench: false,
            cad_step_mm: 1000.0,
            cad_rotate_deg: 90.0,
            cad_cmd_input: String::new(),
            shortcuts_modal_open: false,
            quick_jump_query: String::new(),
            globe_mode: true,
            earth_world: PlanetaryWorldState::default(),
            last_globe_xform: None,
            pending_earth_arc_start: None,
            lifecycle_ops: LifecycleOpsState::default(),
            last_picked_bim_element: None,
            last_picked_earth_lat_lon: None,
            highlighted_bim_element: None,
            spatial_pick_seq: 0,
            earth_bim_author: EarthBimAuthorConfig::default(),
            pending_earth_bim_commands: Vec::new(),
            world_index: crate::core::gis::WorldSpatialIndex::default(),
            active_world_project_rel_path: None,
            placing_unlocated_rel_path: None,
            placing_heading_deg: 0.0,
            placing_hover_lat_lon: None,
            clicked_world_building_rel_path: None,
            pending_world_pin_event: None,
            active_site_tin: None,
            site_tin_preset: crate::core::gis::DemTerrainPreset::TerracedCutFillPad,
            show_terrain_contours: false,
            hovered_world_building_rel_path: None,
            urban_vectors: crate::core::gis::UrbanVectorLayerCache::default(),
            dem_cache: crate::core::gis::DemTileCache::default(),
            use_live_dem_tiles: true,
            site_tin_is_live_dem: false,
            neighborhood_report: crate::core::gis::NeighborhoodAuditReport::default(),
            show_neighborhood_hazards: true,
            active_vector_snap_az_deg: None,
        }
    }
}

impl UniversalCanvasState {
    /// Computes coupled geotechnical, seismic, wind, flood, and site logistics telemetry
    /// from the currently anchored Earth coordinate (or viewport focus) and planetary terrain.
    pub fn active_site_coupling(&self) -> SiteEnvironmentalCoupling {
        let (lat, lon) = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        compute_site_environmental_coupling(lat, lon, &self.earth_world)
    }

    /// Generates `active_site_tin` on demand, preferring live Terrarium DEM elevation tiles when cached
    /// and falling back cleanly to `site_tin_preset` (local 400m x 400m geotechnical TIN).
    pub fn ensure_site_tin_generated(&mut self) {
        if self.use_live_dem_tiles {
            if let Some(dem_tin) = self
                .dem_cache
                .build_site_tin(self.ground_origin_lat_lon, 400.0, 20)
            {
                self.active_site_tin = Some(dem_tin);
                self.site_tin_is_live_dem = true;
                return;
            }
        }
        if self.active_site_tin.is_none() {
            self.active_site_tin = Some(crate::core::gis::TinSurfaceMesh::generate_from_dem_preset(
                self.site_tin_preset,
                400.0,
                400.0,
                28,
                28,
                1.0,
            ));
            self.site_tin_is_live_dem = false;
        }
    }

    /// Evaluates seismic pounding gaps, urban wind exposure & Venturi channeling, and debris zones
    /// between the active BIM structure and all surrounding neighbors inside `world_index`.
    pub fn refresh_neighborhood_coupling(&mut self, bim_project: &BimProject) {
        if !self.ground_origin_anchored && self.world_index.buildings.is_empty() {
            self.neighborhood_report = crate::core::gis::NeighborhoodAuditReport::default();
            return;
        }
        let (hull, h_m) = crate::core::gis::bim_footprint_hull_m(bim_project);
        let hdg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        self.neighborhood_report = crate::core::gis::evaluate_neighborhood_coupling(
            self.ground_origin_lat_lon,
            &hull,
            hdg,
            h_m,
            self.active_world_project_rel_path.as_deref(),
            &self.world_index,
        );
    }

    /// Local ground relief offset (meters relative to Earth surface datum `0.0` at anchored origin).
    pub fn local_site_relief_m(&self, lat: f64, lon: f64) -> f64 {
        if !self.show_terrain_contours {
            return 0.0;
        }
        let Some(ref tin) = self.active_site_tin else {
            return 0.0;
        };
        let [east_mm, north_mm] = SlippyMapState::lat_lon_to_world_mm(self.ground_origin_lat_lon, lat, lon);
        let east_m = (east_mm * 0.001) as f32;
        let north_m = (north_mm * 0.001) as f32;
        if east_m.abs() > tin.extent_x_m * 0.5 || north_m.abs() > tin.extent_y_m * 0.5 {
            return 0.0;
        }
        let (ref_z, _) = tin.sample_elevation_and_slope(0.0, 0.0);
        let (local_z, _) = tin.sample_elevation_and_slope(east_m, north_m);
        (local_z - ref_z) as f64
    }
    /// Theme-aware canvas viewport background matching Universal Canvas across all modules.
    pub fn canvas_bg(theme: &MacTheme) -> Color32 {
        if theme.is_dark() {
            Color32::from_rgb(12, 15, 21)
        } else {
            Color32::from_rgb(248, 250, 253)
        }
    }

    /// Unified height calculation used by Universal Canvas, Parametric, and Geometry viewports.
    pub fn preferred_canvas_height(&self, ctx: &egui::Context) -> f32 {
        let vp = MacTheme::viewport_size(ctx);
        let is_mobile = vp.x < MacTheme::MOBILE_BREAKPOINT;
        (vp.y * self.canvas_height_ratio).clamp(if is_mobile { 380.0 } else { 540.0 }, 960.0)
    }

    /// Frosted translucent floating HUD overlay frame (matches Universal Canvas inspector).
    pub fn floating_panel_frame(theme: &MacTheme, is_overlay: bool) -> Frame {
        let fill = if is_overlay {
            if theme.is_dark() {
                Color32::from_rgba_premultiplied(20, 24, 34, 240)
            } else {
                Color32::from_rgba_premultiplied(255, 255, 255, 244)
            }
        } else {
            theme.bg_card
        };

        Frame::new()
            .fill(fill)
            .corner_radius(CornerRadius::same(10))
            .stroke(Stroke::new(1.2, theme.accent_blue.gamma_multiply(0.55)))
            .inner_margin(Margin::same(12))
    }

    /// Shared dismissible status toast strip rendered above or below the canvas viewport.
    pub fn render_toast_bar(ui: &mut Ui, theme: &MacTheme, status_message: &mut Option<String>) {
        if let Some(msg) = status_message.clone() {
            Frame::new()
                .fill(theme.accent_green.gamma_multiply(0.14))
                .corner_radius(CornerRadius::same(8))
                .stroke(Stroke::new(1.0, theme.accent_green.gamma_multiply(0.5)))
                .inner_margin(Margin::symmetric(12, 5))
                .show(ui, |ui| {
                    let inner_w = ui.available_width().max(120.0);
                    ui.set_width(inner_w);
                    ui.set_max_width(inner_w);
                    ui.horizontal_wrapped(|ui| {
                        icons::icon_label(
                            ui,
                            AppIcon::CheckCircle,
                            13.0,
                            theme.accent_green,
                            msg,
                            11.0,
                            theme.accent_green,
                            false,
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icons::icon_only_button(
                                ui,
                                AppIcon::Close,
                                theme.text_secondary,
                                Color32::TRANSPARENT,
                                Stroke::NONE,
                                CornerRadius::same(4),
                                Vec2::splat(20.0),
                                "Dismiss notification",
                            )
                            .clicked()
                            {
                                *status_message = None;
                            }
                        });
                    });
                });
        }
    }

    /// Computes the top-left floating pill rect inside any drawing viewport,
    /// shifting past the mobile reopen-sidebar button when the viewport sits at the top.
    pub fn floating_freeze_pill_rect(
        viewport_rect: Rect,
        is_frozen: bool,
        sidebar_toggle_inset_px: f32,
    ) -> Rect {
        let inset_x = if sidebar_toggle_inset_px > 0.0 && viewport_rect.top() < 120.0 {
            sidebar_toggle_inset_px
        } else {
            0.0
        };
        let pill_w: f32 = if is_frozen { 154.0 } else { 116.0 };
        let max_w = (viewport_rect.width() - 28.0 - inset_x).max(96.0);
        Rect::from_min_size(
            Pos2::new(
                viewport_rect.left() + 14.0 + inset_x,
                viewport_rect.top() + 14.0,
            ),
            Vec2::new(pill_w.min(max_w), 28.0),
        )
    }

    /// Registers hit-testing for the top-left floating Freeze / Live toggle BEFORE
    /// underlying canvas pan/zoom/draw handlers so clicks never pass through.
    /// Returns `(pill_rect, pointer_in_pill, hovered)`.
    pub fn interact_floating_freeze_pill(
        ui: &mut Ui,
        viewport_rect: Rect,
        id_salt: &str,
        is_frozen: &mut bool,
        sidebar_toggle_inset_px: f32,
    ) -> (Rect, bool, bool) {
        let pill_rect =
            Self::floating_freeze_pill_rect(viewport_rect, *is_frozen, sidebar_toggle_inset_px);
        let resp = ui
            .interact(
                pill_rect,
                ui.make_persistent_id(("floating_freeze_pill", id_salt)),
                Sense::click(),
            )
            .on_hover_text(if *is_frozen {
                "Canvas frozen (page scrolls normally). Click to unfreeze & draw or zoom."
            } else {
                "Canvas live. Click to freeze zoom/pan so page scrolls freely."
            });
        if resp.clicked() {
            *is_frozen = !*is_frozen;
            ui_kit::set_canvas_frozen(ui.ctx(), *is_frozen);
        }
        let pointer_in_pill = resp.hovered()
            || resp.clicked()
            || ui.ctx().input(|i| {
                i.pointer
                    .hover_pos()
                    .or_else(|| i.pointer.interact_pos())
                    .is_some_and(|p| pill_rect.expand(4.0).contains(p))
            });
        (pill_rect, pointer_in_pill, resp.hovered())
    }

    /// Paints the translucent top-left floating Freeze / Live capsule over the finished viewport.
    pub fn paint_floating_freeze_pill(
        painter: &egui::Painter,
        pill_rect: Rect,
        theme: &MacTheme,
        is_frozen: bool,
        hovered: bool,
    ) {
        let (bg, border, icon, icon_col, text_col, label) = if is_frozen {
            let alpha = if hovered { 244 } else { 226 };
            (
                Color32::from_rgba_unmultiplied(
                    theme.accent_blue.r(),
                    theme.accent_blue.g(),
                    theme.accent_blue.b(),
                    alpha,
                ),
                Color32::from_rgba_unmultiplied(219, 234, 254, 220),
                AppIcon::Snowflake,
                Color32::WHITE,
                Color32::WHITE,
                "Frozen • Unfreeze",
            )
        } else {
            let bg = if theme.is_dark() {
                Color32::from_rgba_unmultiplied(18, 24, 36, if hovered { 238 } else { 214 })
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, if hovered { 242 } else { 222 })
            };
            (
                bg,
                if hovered {
                    theme.accent_orange
                } else {
                    theme.accent_orange.gamma_multiply(0.75)
                },
                AppIcon::Unlock,
                theme.accent_orange,
                theme.text_primary,
                "Live • Freeze",
            )
        };
        let rounding = CornerRadius::same(14);
        painter.rect_filled(pill_rect, rounding, bg);
        painter.rect_stroke(pill_rect, rounding, Stroke::new(1.1, border), StrokeKind::Inside);
        let icon_rect = Rect::from_min_size(
            Pos2::new(pill_rect.left() + 9.0, pill_rect.center().y - 6.5),
            Vec2::splat(13.0),
        );
        icons::paint_icon(painter, icon_rect, icon, icon_col);
        painter.text(
            Pos2::new(pill_rect.left() + 27.0, pill_rect.center().y),
            Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            text_col,
        );
    }

    /// Masks outer sharp corners of an interactive canvas/map viewport to a smooth `radius`
    /// using spandrel triangles in `outer_bg`, then strokes a crisp rounded border over it.
    pub fn paint_rounded_viewport_border(
        painter: &egui::Painter,
        rect: Rect,
        radius: f32,
        outer_bg: Color32,
        stroke_color: Color32,
    ) {
        let r = radius.clamp(2.0, rect.width().min(rect.height()) * 0.45);
        let segs = 7usize;
        let corners = [
            // (outer_corner, arc_center, start_angle, end_angle)
            (
                Pos2::new(rect.left(), rect.top()),
                Pos2::new(rect.left() + r, rect.top() + r),
                std::f32::consts::PI,
                std::f32::consts::FRAC_PI_2 * 3.0,
            ),
            (
                Pos2::new(rect.right(), rect.top()),
                Pos2::new(rect.right() - r, rect.top() + r),
                std::f32::consts::FRAC_PI_2 * 3.0,
                std::f32::consts::TAU,
            ),
            (
                Pos2::new(rect.right(), rect.bottom()),
                Pos2::new(rect.right() - r, rect.bottom() - r),
                0.0,
                std::f32::consts::FRAC_PI_2,
            ),
            (
                Pos2::new(rect.left(), rect.bottom()),
                Pos2::new(rect.left() + r, rect.bottom() - r),
                std::f32::consts::FRAC_PI_2,
                std::f32::consts::PI,
            ),
        ];

        for (outer, center, a0, a1) in corners {
            let mut pts = Vec::with_capacity(segs + 2);
            pts.push(outer);
            for s in 0..=segs {
                let t = s as f32 / segs as f32;
                let ang = a0 + (a1 - a0) * t;
                pts.push(Pos2::new(center.x + r * ang.cos(), center.y + r * ang.sin()));
            }
            painter.add(egui::Shape::convex_polygon(pts, outer_bg, Stroke::NONE));
        }

        painter.rect_stroke(
            rect,
            CornerRadius::same(r.round() as u8),
            Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );
    }

    /// Returns `true` when the pointer hovers over the live, unfrozen canvas area (excluding the floating inspector).
    pub fn is_pointer_in_live_canvas(&self, ctx: &egui::Context) -> bool {
        if self.is_frozen {
            return false;
        }
        let Some(pointer) = ctx.input(|i| i.pointer.hover_pos()) else {
            return false;
        };
        let in_canvas = self.last_canvas_rect.is_some_and(|r| r.contains(pointer));
        let in_inspector = self.inspector_open
            && self
                .last_inspector_rect
                .is_some_and(|r| r.contains(pointer));
        in_canvas && !in_inspector
    }

    pub fn active_color(&self) -> Color32 {
        let [r, g, b] = PALETTE_SWATCHES
            .get(self.active_color_idx)
            .map(|(_, c)| *c)
            .unwrap_or([56, 189, 248]);
        Color32::from_rgb(r, g, b)
    }

    pub fn active_color_rgb(&self) -> [u8; 3] {
        PALETTE_SWATCHES
            .get(self.active_color_idx)
            .map(|(_, c)| *c)
            .unwrap_or([56, 189, 248])
    }

    /// Pushes current entity list to undo history before mutating.
    fn push_undo(&mut self) {
        self.undo_stack.push(self.entities.clone());
        if self.undo_stack.len() > 30 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.entities.clone());
            if self.redo_stack.len() > 30 {
                self.redo_stack.remove(0);
            }
            self.entities = prev;
            if let Some(sel) = self.selected_entity_id {
                if !self.entities.iter().any(|e| e.id == sel) {
                    self.selected_entity_id = None;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.entities.clone());
            if self.undo_stack.len() > 30 {
                self.undo_stack.remove(0);
            }
            self.entities = next;
            if let Some(sel) = self.selected_entity_id {
                if !self.entities.iter().any(|e| e.id == sel) {
                    self.selected_entity_id = None;
                }
            }
            true
        } else {
            false
        }
    }

    /// Polls CAD keyboard shortcuts only when no text field holds focus.
    pub fn poll_cad_shortcuts(ctx: &egui::Context, is_frozen: bool) -> CadShortcutFeedback {
        if ctx.egui_wants_keyboard_input() {
            return CadShortcutFeedback::default();
        }
        ctx.input(|i| {
            let mut fb = CadShortcutFeedback::default();
            if i.key_pressed(egui::Key::Space) && !i.modifiers.any() {
                fb.toggle_freeze = true;
            }
            if is_frozen {
                return fb;
            }
            fb.escape = i.key_pressed(egui::Key::Escape);
            fb.delete_selected =
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace);
            fb.undo = i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z);
            fb.redo = (i.modifiers.command && i.key_pressed(egui::Key::Y))
                || (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z));
            fb.toggle_ortho = i.key_pressed(egui::Key::F8);
            fb.toggle_snap = i.key_pressed(egui::Key::F3);
            fb.toggle_osnap = i.key_pressed(egui::Key::F4);

            if !i.modifiers.command && !i.modifiers.ctrl && !i.modifiers.alt {
                if i.key_pressed(egui::Key::Z) {
                    fb.fit_model = true;
                } else {
                    const KEYS: [(egui::Key, char); 14] = [
                        (egui::Key::W, 'W'),
                        (egui::Key::B, 'B'),
                        (egui::Key::C, 'C'),
                        (egui::Key::F, 'F'),
                        (egui::Key::L, 'L'),
                        (egui::Key::R, 'R'),
                        (egui::Key::D, 'D'),
                        (egui::Key::M, 'M'),
                        (egui::Key::O, 'O'),
                        (egui::Key::X, 'X'),
                        (egui::Key::E, 'E'),
                        (egui::Key::T, 'T'),
                        (egui::Key::A, 'A'),
                        (egui::Key::V, 'V'),
                    ];
                    for (key, ch) in KEYS {
                        if i.key_pressed(key) {
                            fb.letter_key = Some(ch);
                            break;
                        }
                    }
                }
            }
            fb
        })
    }

    /// Constraints `end_mm` orthogonally relative to `start_mm` when `ortho_mode` is enabled.
    pub fn apply_ortho(&self, start_mm: [f64; 2], end_mm: [f64; 2]) -> [f64; 2] {
        if !self.ortho_mode {
            return end_mm;
        }
        let dx = (end_mm[0] - start_mm[0]).abs();
        let dy = (end_mm[1] - start_mm[1]).abs();
        if dx >= dy {
            [end_mm[0], start_mm[1]]
        } else {
            [start_mm[0], end_mm[1]]
        }
    }

    fn target_entity_idx(&self) -> Option<usize> {
        if let Some(sel) = self.selected_entity_id {
            if let Some(pos) = self.entities.iter().position(|e| e.id == sel) {
                return Some(pos);
            }
        }
        self.entities.len().checked_sub(1)
    }

    fn target_entity_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = if !self.selected_entity_ids.is_empty() {
            self.entities
                .iter()
                .enumerate()
                .filter(|(_, e)| self.selected_entity_ids.contains(&e.id))
                .map(|(i, _)| i)
                .collect()
        } else {
            Vec::new()
        };
        if indices.is_empty() {
            if let Some(idx) = self.target_entity_idx() {
                indices.push(idx);
            }
        }
        indices
    }

    fn is_entity_selected(&self, id: usize) -> bool {
        self.selected_entity_id == Some(id) || self.selected_entity_ids.contains(&id)
    }

    fn set_single_selection(&mut self, id: Option<usize>) {
        self.selected_entity_id = id;
        self.selected_entity_ids.clear();
        if let Some(eid) = id {
            self.selected_entity_ids.push(eid);
        }
    }

    fn translate_kind(kind: &mut CanvasShapeKind, dx_mm: f64, dy_mm: f64) {
        match kind {
            CanvasShapeKind::Line { start_mm, end_mm }
            | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                start_mm[0] += dx_mm;
                start_mm[1] += dy_mm;
                end_mm[0] += dx_mm;
                end_mm[1] += dy_mm;
            }
            CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                min_mm[0] += dx_mm;
                min_mm[1] += dy_mm;
                max_mm[0] += dx_mm;
                max_mm[1] += dy_mm;
            }
            CanvasShapeKind::Circle { center_mm, .. } => {
                center_mm[0] += dx_mm;
                center_mm[1] += dy_mm;
            }
            CanvasShapeKind::Freehand { points_mm } => {
                for p in points_mm {
                    p[0] += dx_mm;
                    p[1] += dy_mm;
                }
            }
            CanvasShapeKind::Callout { pos_mm, .. } => {
                pos_mm[0] += dx_mm;
                pos_mm[1] += dy_mm;
            }
        }
    }

    fn entity_aabb(kind: &CanvasShapeKind) -> ([f64; 2], [f64; 2]) {
        match kind {
            CanvasShapeKind::Line { start_mm, end_mm }
            | CanvasShapeKind::Dimension { start_mm, end_mm } => (
                [start_mm[0].min(end_mm[0]), start_mm[1].min(end_mm[1])],
                [start_mm[0].max(end_mm[0]), start_mm[1].max(end_mm[1])],
            ),
            CanvasShapeKind::Rectangle { min_mm, max_mm } => (*min_mm, *max_mm),
            CanvasShapeKind::Circle { center_mm, radius_mm } => (
                [center_mm[0] - radius_mm, center_mm[1] - radius_mm],
                [center_mm[0] + radius_mm, center_mm[1] + radius_mm],
            ),
            CanvasShapeKind::Freehand { points_mm } => {
                if points_mm.is_empty() {
                    ([0.0, 0.0], [0.0, 0.0])
                } else {
                    let mut mn = points_mm[0];
                    let mut mx = points_mm[0];
                    for p in points_mm {
                        mn[0] = mn[0].min(p[0]);
                        mn[1] = mn[1].min(p[1]);
                        mx[0] = mx[0].max(p[0]);
                        mx[1] = mx[1].max(p[1]);
                    }
                    (mn, mx)
                }
            }
            CanvasShapeKind::Callout { pos_mm, .. } => (*pos_mm, *pos_mm),
        }
    }

    /// Selects entities inside marquee `[a_mm, b_mm]`. Left->Right (`b_mm[0] >= a_mm[0]`) uses Window (fully enclosed); Right->Left (`b_mm[0] < a_mm[0]`) uses Crossing (intersecting/overlapping).
    pub fn select_marquee(&mut self, a_mm: [f64; 2], b_mm: [f64; 2]) -> usize {
        let min_x = a_mm[0].min(b_mm[0]);
        let max_x = a_mm[0].max(b_mm[0]);
        let min_y = a_mm[1].min(b_mm[1]);
        let max_y = a_mm[1].max(b_mm[1]);
        let is_window = b_mm[0] >= a_mm[0];

        self.selected_entity_ids.clear();
        for ent in &self.entities {
            let (e_min, e_max) = Self::entity_aabb(&ent.kind);
            let matched = if is_window {
                e_min[0] >= min_x && e_max[0] <= max_x && e_min[1] >= min_y && e_max[1] <= max_y
            } else {
                e_max[0] >= min_x && e_min[0] <= max_x && e_max[1] >= min_y && e_min[1] <= max_y
            };
            if matched {
                self.selected_entity_ids.push(ent.id);
            }
        }
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        self.selected_entity_ids.len()
    }

    /// Translates the selected entities (or latest entity) by `(dx_mm, dy_mm)` with undo support.
    pub fn translate_selected_or_last(&mut self, dx_mm: f64, dy_mm: f64) -> bool {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let mut moved_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            if let Some(ent) = self.entities.get_mut(idx) {
                Self::translate_kind(&mut ent.kind, dx_mm, dy_mm);
                moved_ids.push(ent.id);
            }
        }
        self.selected_entity_ids = moved_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Creates a parallel/outward offset duplicate of the selected entities (or latest entity).
    pub fn offset_selected_or_last(&mut self, offset_mm: f64) -> bool {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let mut new_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            let mut cloned = self.entities[idx].clone();
            match &mut cloned.kind {
                CanvasShapeKind::Line { start_mm, end_mm }
                | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    let vx = end_mm[0] - start_mm[0];
                    let vy = end_mm[1] - start_mm[1];
                    let len = vx.hypot(vy).max(1.0);
                    let nx = -vy / len * offset_mm;
                    let ny = vx / len * offset_mm;
                    start_mm[0] += nx;
                    start_mm[1] += ny;
                    end_mm[0] += nx;
                    end_mm[1] += ny;
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    min_mm[0] -= offset_mm;
                    min_mm[1] -= offset_mm;
                    max_mm[0] += offset_mm;
                    max_mm[1] += offset_mm;
                }
                CanvasShapeKind::Circle { radius_mm, .. } => {
                    *radius_mm = (*radius_mm + offset_mm).max(100.0);
                }
                CanvasShapeKind::Freehand { points_mm } => {
                    for p in points_mm {
                        p[0] += offset_mm;
                        p[1] += offset_mm;
                    }
                }
                CanvasShapeKind::Callout { pos_mm, .. } => {
                    pos_mm[0] += offset_mm;
                    pos_mm[1] += offset_mm;
                }
            }
            let new_id = self.next_entity_id;
            self.next_entity_id += 1;
            cloned.id = new_id;
            self.entities.push(cloned);
            new_ids.push(new_id);
        }
        self.selected_entity_ids = new_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Creates a mirrored duplicate of the selected entities (or latest entity) across vertical axis `x = mirror_x_mm`.
    pub fn mirror_selected_or_last(&mut self, mirror_x_mm: f64) -> bool {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let mx = |x: f64| 2.0 * mirror_x_mm - x;
        let mut new_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            let mut cloned = self.entities[idx].clone();
            match &mut cloned.kind {
                CanvasShapeKind::Line { start_mm, end_mm }
                | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    start_mm[0] = mx(start_mm[0]);
                    end_mm[0] = mx(end_mm[0]);
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    let nx0 = mx(max_mm[0]);
                    let nx1 = mx(min_mm[0]);
                    min_mm[0] = nx0.min(nx1);
                    max_mm[0] = nx0.max(nx1);
                }
                CanvasShapeKind::Circle { center_mm, .. } => {
                    center_mm[0] = mx(center_mm[0]);
                }
                CanvasShapeKind::Freehand { points_mm } => {
                    for p in points_mm {
                        p[0] = mx(p[0]);
                    }
                }
                CanvasShapeKind::Callout { pos_mm, .. } => {
                    pos_mm[0] = mx(pos_mm[0]);
                }
            }
            let new_id = self.next_entity_id;
            self.next_entity_id += 1;
            cloned.id = new_id;
            self.entities.push(cloned);
            new_ids.push(new_id);
        }
        self.selected_entity_ids = new_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Parses CAD Cartesian (`@dx,dy` or `dx,dy`) or polar (`@dist<deg` or `dist<deg`) coordinate vectors in mm.
    pub fn parse_cad_delta_expr(raw: &str) -> Option<(f64, f64)> {
        let s = raw.trim().trim_start_matches('@');
        if let Some((d_str, a_str)) = s.split_once('<') {
            let dist = d_str.trim().parse::<f64>().ok()?;
            let rad = a_str.trim().parse::<f64>().ok()?.to_radians();
            return Some((dist * rad.cos(), dist * rad.sin()));
        }
        if let Some((x_str, y_str)) = s.split_once(',') {
            let dx = x_str.trim().parse::<f64>().ok()?;
            let dy = y_str.trim().parse::<f64>().ok()?;
            return Some((dx, dy));
        }
        None
    }

    /// Creates a displaced duplicate of the selected entities (or latest entity) by `(dx_mm, dy_mm)` with undo support.
    pub fn duplicate_selected_or_last(&mut self, dx_mm: f64, dy_mm: f64) -> bool {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let mut new_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            let mut cloned = self.entities[idx].clone();
            Self::translate_kind(&mut cloned.kind, dx_mm, dy_mm);
            let new_id = self.next_entity_id;
            self.next_entity_id += 1;
            cloned.id = new_id;
            self.entities.push(cloned);
            new_ids.push(new_id);
        }
        self.selected_entity_ids = new_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Computes engineering summary metrics for a single 2D CAD sketch entity by ID.
    pub fn entity_summary(&self, id: usize) -> Option<(&'static str, f64, f64, f64, String, [u8; 3], f32)> {
        let ent = self.entities.iter().find(|e| e.id == id)?;
        let (kind_label, length_m, area_m2, radius_m, detail) = match &ent.kind {
            CanvasShapeKind::Line { start_mm, end_mm } => {
                let dx = (end_mm[0] - start_mm[0]) * 0.001;
                let dy = (end_mm[1] - start_mm[1]) * 0.001;
                let len = dx.hypot(dy);
                (
                    "Line Segment",
                    len,
                    0.0,
                    0.0,
                    format!(
                        "({:.2}, {:.2}) -> ({:.2}, {:.2}) m",
                        start_mm[0] * 0.001,
                        start_mm[1] * 0.001,
                        end_mm[0] * 0.001,
                        end_mm[1] * 0.001
                    ),
                )
            }
            CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                let w = (max_mm[0] - min_mm[0]).abs() * 0.001;
                let h = (max_mm[1] - min_mm[1]).abs() * 0.001;
                ("Rectangle", 2.0 * (w + h), w * h, 0.0, format!("{w:.2}m x {h:.2}m"))
            }
            CanvasShapeKind::Circle { center_mm, radius_mm } => {
                let r = (*radius_mm).abs() * 0.001;
                (
                    "Circle",
                    2.0 * std::f64::consts::PI * r,
                    std::f64::consts::PI * r * r,
                    r,
                    format!(
                        "Center ({:.2}, {:.2}) m",
                        center_mm[0] * 0.001,
                        center_mm[1] * 0.001
                    ),
                )
            }
            CanvasShapeKind::Dimension { start_mm, end_mm } => {
                let len =
                    ((end_mm[0] - start_mm[0]) * 0.001).hypot((end_mm[1] - start_mm[1]) * 0.001);
                ("Dimension", len, 0.0, 0.0, format!("Span {len:.3} m"))
            }
            CanvasShapeKind::Freehand { points_mm } => {
                let mut len = 0.0;
                for w in points_mm.windows(2) {
                    len += ((w[1][0] - w[0][0]) * 0.001).hypot((w[1][1] - w[0][1]) * 0.001);
                }
                ("Polyline", len, 0.0, 0.0, format!("{} vertices", points_mm.len()))
            }
            CanvasShapeKind::Callout { pos_mm, text } => (
                "Annotation Callout",
                0.0,
                0.0,
                0.0,
                format!(
                    "{} @ ({:.2}, {:.2}) m",
                    text,
                    pos_mm[0] * 0.001,
                    pos_mm[1] * 0.001
                ),
            ),
        };
        Some((
            kind_label,
            length_m,
            area_m2,
            radius_m,
            detail,
            ent.color_rgb,
            ent.stroke_width,
        ))
    }

    /// Updates stroke width and RGB color for a single CAD sketch entity with undo support.
    pub fn update_entity_style(&mut self, id: usize, color_rgb: [u8; 3], stroke_width: f32) -> bool {
        if !self.entities.iter().any(|e| e.id == id) {
            return false;
        }
        self.push_undo();
        if let Some(ent) = self.entities.iter_mut().find(|e| e.id == id) {
            ent.color_rgb = color_rgb;
            ent.stroke_width = stroke_width.clamp(0.8, 12.0);
        }
        true
    }

    /// Duplicates a specific CAD sketch entity with `(dx_mm, dy_mm)` offset and undo support.
    pub fn duplicate_offset_entity(&mut self, id: usize, dx_mm: f64, dy_mm: f64) -> Option<usize> {
        let idx = self.entities.iter().position(|e| e.id == id)?;
        self.push_undo();
        let mut cloned = self.entities[idx].clone();
        Self::translate_kind(&mut cloned.kind, dx_mm, dy_mm);
        let new_id = self.next_entity_id;
        self.next_entity_id += 1;
        cloned.id = new_id;
        self.entities.push(cloned);
        self.set_single_selection(Some(new_id));
        Some(new_id)
    }

    /// Deletes a specific CAD sketch entity by ID with undo support.
    pub fn delete_entity_by_id(&mut self, id: usize) -> bool {
        let Some(pos) = self.entities.iter().position(|e| e.id == id) else {
            return false;
        };
        self.push_undo();
        self.entities.remove(pos);
        self.selected_entity_ids.retain(|&eid| eid != id);
        if self.selected_entity_id == Some(id) {
            self.selected_entity_id = self.selected_entity_ids.first().copied();
        }
        true
    }

    /// Clears recorded geodesic measurement vertices.
    pub fn clear_measure(&mut self) {
        self.measure_points_lat_lon.clear();
    }

    /// Computes live geodesic measurement summary: `(polyline_m, optional_polygon_area_m2, vertex_count)`.
    pub fn geodesic_measure_summary(&self) -> (f64, Option<f64>, usize) {
        let total_m = crate::core::gis::SlippyMapState::geodetic_polyline_length_m(
            &self.measure_points_lat_lon,
        );
        let area_m2 = if self.measure_area_mode && self.measure_points_lat_lon.len() >= 3 {
            Some(crate::core::gis::SlippyMapState::geodetic_polygon_area_m2(
                &self.measure_points_lat_lon,
            ))
        } else {
            None
        };
        (total_m, area_m2, self.measure_points_lat_lon.len())
    }

    /// Adds a precision parametric 2D Line anchored at `(0, 0)` with exact length `length_m` and azimuth `angle_deg`.
    pub fn commit_parametric_2d_line(
        &mut self,
        length_m: f64,
        angle_deg: f64,
        pillar_badge: &str,
        pillar_name: &str,
    ) -> usize {
        self.push_undo();
        let len_mm = length_m.abs().max(0.1) * 1000.0;
        let rad = angle_deg.to_radians();
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.push(CanvasEntity {
            id,
            pillar_badge: pillar_badge.to_string(),
            pillar_name: pillar_name.to_string(),
            kind: CanvasShapeKind::Line {
                start_mm: [0.0, 0.0],
                end_mm: [len_mm * rad.cos(), len_mm * rad.sin()],
            },
            color_rgb: [59, 130, 246],
            stroke_width: 2.2,
        });
        self.set_single_selection(Some(id));
        id
    }

    /// Adds a precision parametric 2D Rectangle centered at the origin with `width_m` x `height_m`.
    pub fn commit_parametric_2d_rect(
        &mut self,
        width_m: f64,
        height_m: f64,
        pillar_badge: &str,
        pillar_name: &str,
    ) -> usize {
        self.push_undo();
        let hw = width_m.abs().max(0.1) * 500.0;
        let hh = height_m.abs().max(0.1) * 500.0;
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.push(CanvasEntity {
            id,
            pillar_badge: pillar_badge.to_string(),
            pillar_name: pillar_name.to_string(),
            kind: CanvasShapeKind::Rectangle {
                min_mm: [-hw, -hh],
                max_mm: [hw, hh],
            },
            color_rgb: [16, 185, 129],
            stroke_width: 2.2,
        });
        self.set_single_selection(Some(id));
        id
    }

    /// Adds a precision parametric 2D Circle centered at the origin with `radius_m`.
    pub fn commit_parametric_2d_circle(
        &mut self,
        radius_m: f64,
        pillar_badge: &str,
        pillar_name: &str,
    ) -> usize {
        self.push_undo();
        let r_mm = radius_m.abs().max(0.1) * 1000.0;
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.push(CanvasEntity {
            id,
            pillar_badge: pillar_badge.to_string(),
            pillar_name: pillar_name.to_string(),
            kind: CanvasShapeKind::Circle {
                center_mm: [0.0, 0.0],
                radius_mm: r_mm,
            },
            color_rgb: [249, 115, 22],
            stroke_width: 2.2,
        });
        self.set_single_selection(Some(id));
        id
    }

    /// Rotates the selected entities (or latest entity) around each entity centroid by `angle_deg` degrees with undo support.
    pub fn rotate_selected_or_last(&mut self, angle_deg: f64) -> bool {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let rad = angle_deg.to_radians();
        let (c, s) = (rad.cos(), rad.sin());
        let rot_pt = |pt: [f64; 2], pivot: [f64; 2]| -> [f64; 2] {
            let rx = pt[0] - pivot[0];
            let ry = pt[1] - pivot[1];
            [pivot[0] + rx * c - ry * s, pivot[1] + rx * s + ry * c]
        };
        let mut rot_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            if let Some(ent) = self.entities.get_mut(idx) {
                match &mut ent.kind {
                    CanvasShapeKind::Line { start_mm, end_mm }
                    | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                        let pivot = [
                            0.5 * (start_mm[0] + end_mm[0]),
                            0.5 * (start_mm[1] + end_mm[1]),
                        ];
                        *start_mm = rot_pt(*start_mm, pivot);
                        *end_mm = rot_pt(*end_mm, pivot);
                    }
                    CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                        let pivot = [0.5 * (min_mm[0] + max_mm[0]), 0.5 * (min_mm[1] + max_mm[1])];
                        let corners = [
                            rot_pt([min_mm[0], min_mm[1]], pivot),
                            rot_pt([max_mm[0], min_mm[1]], pivot),
                            rot_pt([max_mm[0], max_mm[1]], pivot),
                            rot_pt([min_mm[0], max_mm[1]], pivot),
                        ];
                        let x0 = corners.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                        let x1 = corners.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
                        let y0 = corners.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
                        let y1 = corners.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
                        *min_mm = [x0, y0];
                        *max_mm = [x1, y1];
                    }
                    CanvasShapeKind::Freehand { points_mm } if !points_mm.is_empty() => {
                        let n = points_mm.len() as f64;
                        let pivot = [
                            points_mm.iter().map(|p| p[0]).sum::<f64>() / n,
                            points_mm.iter().map(|p| p[1]).sum::<f64>() / n,
                        ];
                        for p in points_mm {
                            *p = rot_pt(*p, pivot);
                        }
                    }
                    CanvasShapeKind::Circle { .. }
                    | CanvasShapeKind::Callout { .. }
                    | CanvasShapeKind::Freehand { .. } => {}
                }
                rot_ids.push(ent.id);
            }
        }
        self.selected_entity_ids = rot_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Generates an orthogonal CAD structural grid (`nx` columns by `ny` rows at spacing `dx_mm`, `dy_mm`).
    pub fn generate_structural_grid(
        &mut self,
        nx: usize,
        ny: usize,
        dx_mm: f64,
        dy_mm: f64,
        active_pillar: NavigationPillar,
        origin_mm: [f64; 2],
    ) -> usize {
        let cols = nx.clamp(2, 16);
        let rows = ny.clamp(2, 16);
        let sx = dx_mm.clamp(500.0, 30_000.0);
        let sy = dy_mm.clamp(500.0, 30_000.0);
        self.push_undo();

        let total_w = (cols - 1) as f64 * sx;
        let total_h = (rows - 1) as f64 * sy;
        let badge = active_pillar.badge().to_string();
        let pname = active_pillar.clean_name().to_string();
        let col = self.active_color_rgb();
        let mut created_ids = Vec::with_capacity(cols + rows);

        for ix in 0..cols {
            let x = origin_mm[0] + ix as f64 * sx;
            let id = self.next_entity_id;
            self.next_entity_id += 1;
            self.entities.push(CanvasEntity {
                id,
                pillar_badge: badge.clone(),
                pillar_name: pname.clone(),
                kind: CanvasShapeKind::Line {
                    start_mm: [x, origin_mm[1]],
                    end_mm: [x, origin_mm[1] + total_h],
                },
                color_rgb: col,
                stroke_width: self.stroke_width,
            });
            created_ids.push(id);
        }
        for iy in 0..rows {
            let y = origin_mm[1] + iy as f64 * sy;
            let id = self.next_entity_id;
            self.next_entity_id += 1;
            self.entities.push(CanvasEntity {
                id,
                pillar_badge: badge.clone(),
                pillar_name: pname.clone(),
                kind: CanvasShapeKind::Line {
                    start_mm: [origin_mm[0], y],
                    end_mm: [origin_mm[0] + total_w, y],
                },
                color_rgb: col,
                stroke_width: self.stroke_width,
            });
            created_ids.push(id);
        }
        let n = created_ids.len();
        self.selected_entity_ids = created_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        n
    }

    /// Creates a rectangular CAD `ARRAY` of `cols x rows` clones spaced by `(dx_mm, dy_mm)`.
    pub fn array_selected_or_last(&mut self, cols: usize, rows: usize, dx_mm: f64, dy_mm: f64) -> usize {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return 0;
        }
        let c_cnt = cols.clamp(1, 12);
        let r_cnt = rows.clamp(1, 12);
        self.push_undo();
        let mut new_ids = Vec::new();
        for idx in indices {
            let template = self.entities[idx].clone();
            for r in 0..r_cnt {
                for c in 0..c_cnt {
                    if c == 0 && r == 0 {
                        continue;
                    }
                    let mut cloned = template.clone();
                    Self::translate_kind(&mut cloned.kind, c as f64 * dx_mm, r as f64 * dy_mm);
                    let nid = self.next_entity_id;
                    self.next_entity_id += 1;
                    cloned.id = nid;
                    self.entities.push(cloned);
                    new_ids.push(nid);
                }
            }
        }
        let added = new_ids.len();
        if added > 0 {
            self.selected_entity_ids = new_ids;
            self.selected_entity_id = self.selected_entity_ids.first().copied();
        }
        added
    }

    /// Promotes selected (or latest) 2D sketch lines, rectangles, and dimensions into native 3D BIM elements (`BimWall`, `BimRun`, `BimColumn`, `BimSlab`).
    pub fn promote_selection_to_bim(&mut self, bim_project: &mut BimProject) -> (usize, usize, usize) {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return (0, 0, 0);
        }
        let base_lvl_id = bim_project.levels.first().map(|l| l.id).unwrap_or(1);
        let top_lvl_id = bim_project.levels.get(1).map(|l| l.id).unwrap_or(base_lvl_id);
        let mut walls_added = 0usize;
        let mut runs_added = 0usize;
        let mut slabs_added = 0usize;

        for idx in indices {
            if let Some(ent) = self.entities.get(idx) {
                match &ent.kind {
                    CanvasShapeKind::Line { start_mm, end_mm } => {
                        let p0 = crate::core::P2(start_mm[0], start_mm[1]);
                        let p1 = crate::core::P2(end_mm[0], end_mm[1]);
                        if p0.dist(p1) >= 150.0 {
                            let wid = bim_project.alloc_id();
                            bim_project.walls.push(crate::core::BimWall {
                                id: wid,
                                name: format!("Sketch Wall W-{wid}"),
                                base_level_id: base_lvl_id,
                                top_level_id: Some(top_lvl_id).filter(|&t| t != base_lvl_id),
                                unconnected_height_mm: 3600.0,
                                start: p0,
                                end: p1,
                                thickness_mm: 200.0,
                                is_structural: true,
                                layers: crate::core::BimWall::default_compound_layers(200.0),
                                openings: Vec::new(),
                            });
                            walls_added += 1;

                            let rid = bim_project.alloc_id();
                            bim_project.runs.push(crate::core::BimRun {
                                id: rid,
                                name: format!("Sketch Beam B-{rid}"),
                                category: crate::core::RunCategory::StructuralBeam,
                                level_id: top_lvl_id,
                                start: p0,
                                end: p1,
                                start_offset_mm: 0.0,
                                end_offset_mm: 0.0,
                                justify: crate::core::RunJustify::Top,
                                profile_name: "WF 250x125".to_string(),
                                width_mm: 250.0,
                                depth_mm: 400.0,
                                is_round: false,
                                is_wf_profile: true,
                                system_name: "CAD Promote".to_string(),
                            });
                            runs_added += 1;

                            // Ensure end columns exist near endpoints for full FEA continuity
                            for pt in [p0, p1] {
                                let exists = bim_project.columns.iter().any(|c| c.pos.dist(pt) < 120.0);
                                if !exists {
                                    let cid = bim_project.alloc_id();
                                    bim_project.columns.push(crate::core::BimColumn {
                                        id: cid,
                                        name: format!("Col C-{cid}"),
                                        base_level_id: base_lvl_id,
                                        top_level_id: top_lvl_id,
                                        pos: pt,
                                        width_mm: 350.0,
                                        depth_mm: 350.0,
                                        rotation_deg: 0.0,
                                        is_round: false,
                                        is_wf_profile: false,
                                        profile_name: "RC 350x350".to_string(),
                                    });
                                }
                            }
                        }
                    }
                    CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                        let c0 = crate::core::P2(min_mm[0], min_mm[1]);
                        let c1 = crate::core::P2(max_mm[0], min_mm[1]);
                        let c2 = crate::core::P2(max_mm[0], max_mm[1]);
                        let c3 = crate::core::P2(min_mm[0], max_mm[1]);
                        if c0.dist(c1) >= 200.0 && c1.dist(c2) >= 200.0 {
                            let sid = bim_project.alloc_id();
                            bim_project.slabs.push(crate::core::BimSlab {
                                id: sid,
                                name: format!("Sketch Slab S-{sid}"),
                                level_id: base_lvl_id,
                                boundary: vec![c0, c1, c2, c3],
                                openings: Vec::new(),
                                thickness_mm: 160.0,
                                is_roof: false,
                                roof_slope_deg: 0.0,
                            });
                            slabs_added += 1;
                            for (p0, p1) in [(c0, c1), (c1, c2), (c2, c3), (c3, c0)] {
                                let wid = bim_project.alloc_id();
                                bim_project.walls.push(crate::core::BimWall {
                                    id: wid,
                                    name: format!("Perimeter Wall W-{wid}"),
                                    base_level_id: base_lvl_id,
                                    top_level_id: Some(top_lvl_id).filter(|&t| t != base_lvl_id),
                                    unconnected_height_mm: 3600.0,
                                    start: p0,
                                    end: p1,
                                    thickness_mm: 200.0,
                                    is_structural: true,
                                    layers: crate::core::BimWall::default_compound_layers(200.0),
                                    openings: Vec::new(),
                                });
                                walls_added += 1;
                            }
                            for pt in [c0, c1, c2, c3] {
                                let exists = bim_project.columns.iter().any(|c| c.pos.dist(pt) < 120.0);
                                if !exists {
                                    let cid = bim_project.alloc_id();
                                    bim_project.columns.push(crate::core::BimColumn {
                                        id: cid,
                                        name: format!("Col C-{cid}"),
                                        base_level_id: base_lvl_id,
                                        top_level_id: top_lvl_id,
                                        pos: pt,
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
                    }
                    CanvasShapeKind::Dimension { start_mm, end_mm } => {
                        let did = bim_project.alloc_id();
                        bim_project.dimensions.push(crate::core::BimDimension {
                            id: did,
                            start: crate::core::P2(start_mm[0], start_mm[1]),
                            end: crate::core::P2(end_mm[0], end_mm[1]),
                            offset_mm: 350.0,
                        });
                    }
                    _ => {}
                }
            }
        }
        if walls_added > 0 {
            bim_project.detect_rooms_on_level(base_lvl_id);
        }
        (walls_added, runs_added, slabs_added)
    }

    fn parse_nx_ny_spacing(args: &str, default_n: (usize, usize), default_s: (f64, f64)) -> ((usize, usize), (f64, f64)) {
        let mut toks = args.split_whitespace();
        let (nx, ny) = if let Some(first) = toks.next() {
            if let Some((a, b)) = first.split_once('x') {
                (
                    a.trim().parse::<usize>().unwrap_or(default_n.0),
                    b.trim().parse::<usize>().unwrap_or(default_n.1),
                )
            } else {
                default_n
            }
        } else {
            default_n
        };
        let dx = toks
            .next()
            .and_then(|s| s.trim_end_matches("mm").parse::<f64>().ok())
            .unwrap_or(default_s.0);
        let dy = toks
            .next()
            .and_then(|s| s.trim_end_matches("mm").parse::<f64>().ok())
            .unwrap_or(dx);
        ((nx, ny), (dx, dy))
    }

    /// Executes a typed precision CAD command (`@dx,dy`, `@dist<deg`, `move`, `ro`, `cp`, `o`, `grid`, `array`, `promote`, tools).
    pub fn execute_precision_command(
        &mut self,
        raw: &str,
        active_pillar: NavigationPillar,
        bim_project: &mut BimProject,
    ) -> String {
        let trimmed = raw.trim();
        let lower = trimmed.to_lowercase();
        let anchor = self
            .entities
            .last()
            .and_then(|e| match &e.kind {
                CanvasShapeKind::Line { end_mm, .. } => Some(*end_mm),
                CanvasShapeKind::Rectangle { max_mm, .. } => Some(*max_mm),
                CanvasShapeKind::Circle { center_mm, .. } => Some(*center_mm),
                _ => None,
            })
            .unwrap_or(self.cursor_world_mm);

        if trimmed.starts_with('@') {
            if let Some((dx, dy)) = Self::parse_cad_delta_expr(trimmed) {
                self.push_undo();
                let id = self.next_entity_id;
                self.next_entity_id += 1;
                let end = [anchor[0] + dx, anchor[1] + dy];
                self.entities.push(CanvasEntity {
                    id,
                    pillar_badge: active_pillar.badge().to_string(),
                    pillar_name: active_pillar.clean_name().to_string(),
                    kind: CanvasShapeKind::Line {
                        start_mm: anchor,
                        end_mm: end,
                    },
                    color_rgb: self.active_color_rgb(),
                    stroke_width: self.stroke_width,
                });
                self.set_single_selection(Some(id));
                return format!(
                    "Drew relative segment ({:.0}, {:.0}) -> ({:.0}, {:.0}) mm",
                    anchor[0], anchor[1], end[0], end[1]
                );
            }
        }

        if let Some(rest) = lower.strip_prefix("move ") {
            if let Some((dx, dy)) = Self::parse_cad_delta_expr(rest) {
                return if self.translate_selected_or_last(dx, dy) {
                    format!("Moved selection by ({dx:.0}, {dy:.0}) mm")
                } else {
                    "No entity selected or present to move".to_string()
                };
            }
        } else if let Some(rest) = lower.strip_prefix("cp ").or_else(|| lower.strip_prefix("copy ")) {
            if let Some((dx, dy)) = Self::parse_cad_delta_expr(rest) {
                return if self.duplicate_selected_or_last(dx, dy) {
                    format!("Duplicated selection by ({dx:.0}, {dy:.0}) mm")
                } else {
                    "No entity selected or present to duplicate".to_string()
                };
            }
        } else if let Some(rest) = lower.strip_prefix("ro ").or_else(|| lower.strip_prefix("rotate ")) {
            if let Ok(deg) = rest.trim().parse::<f64>() {
                return if self.rotate_selected_or_last(deg) {
                    format!("Rotated selection {deg:.1}°")
                } else {
                    "No entity selected or present to rotate".to_string()
                };
            }
        } else if let Some(rest) = lower.strip_prefix("o ").or_else(|| lower.strip_prefix("offset ")) {
            if let Ok(d) = rest.trim().parse::<f64>() {
                return if self.offset_selected_or_last(d) {
                    format!("Offset duplicate by {d:.0} mm")
                } else {
                    "No entity selected or present to offset".to_string()
                };
            }
        } else if lower == "grid" || lower.starts_with("grid ") {
            let args = lower.strip_prefix("grid").unwrap_or("").trim();
            let ((nx, ny), (dx, dy)) = Self::parse_nx_ny_spacing(args, (4, 3), (6000.0, 5000.0));
            let lines = self.generate_structural_grid(nx, ny, dx, dy, active_pillar, [0.0, 0.0]);
            return format!("Generated {nx}x{ny} structural grid ({lines} axes @ {dx:.0}x{dy:.0} mm)");
        } else if lower == "array" || lower.starts_with("array ") {
            let args = lower.strip_prefix("array").unwrap_or("").trim();
            let ((c, r), (dx, dy)) = Self::parse_nx_ny_spacing(args, (3, 2), (self.cad_step_mm.max(1000.0), self.cad_step_mm.max(1000.0)));
            let added = self.array_selected_or_last(c, r, dx, dy);
            return if added > 0 {
                format!("Arrayed {c}x{r} (+{added} clones @ {dx:.0}x{dy:.0} mm)")
            } else {
                "Select or draw an entity before running ARRAY".to_string()
            };
        }

        match lower.as_str() {
            "v" | "box" | "marquee" => {
                self.active_tool = UniversalCanvasTool::BoxSelect;
                "Active tool: Box Marquee Select (V) — Window L->R | Crossing R->L".to_string()
            }
            "osnap" | "f4" => {
                self.osnap_enabled = !self.osnap_enabled;
                format!("Object Snap (OSNAP F4) = {}", if self.osnap_enabled { "ON" } else { "OFF" })
            }
            "promote" | "tobim" | "bim" => {
                let (w, r, s) = self.promote_selection_to_bim(bim_project);
                if w + r + s > 0 {
                    format!("Promoted CAD selection -> +{w} Walls, +{r} Beams, +{s} Slabs in 3D BIM")
                } else {
                    "Select Lines or Rectangles first to promote into 3D BIM".to_string()
                }
            }
            "l" | "line" => {
                self.active_tool = UniversalCanvasTool::Line;
                "Active tool: Line (L)".to_string()
            }
            "r" | "rect" | "rectangle" => {
                self.active_tool = UniversalCanvasTool::Rectangle;
                "Active tool: Rectangle (R)".to_string()
            }
            "c" | "circle" => {
                self.active_tool = UniversalCanvasTool::Circle;
                "Active tool: Circle (C)".to_string()
            }
            "d" | "dim" | "dimension" => {
                self.active_tool = UniversalCanvasTool::Dimension;
                "Active tool: Dimension (D)".to_string()
            }
            "t" | "text" | "callout" => {
                self.active_tool = UniversalCanvasTool::Callout;
                "Active tool: Callout (T)".to_string()
            }
            "e" | "erase" => {
                self.active_tool = UniversalCanvasTool::Eraser;
                "Active tool: Eraser (E)".to_string()
            }
            "ro" | "rotate" => {
                if self.rotate_selected_or_last(self.cad_rotate_deg) {
                    format!("Rotated selection {:.1}°", self.cad_rotate_deg)
                } else {
                    "No entity to rotate".to_string()
                }
            }
            "cp" | "copy" | "dup" => {
                if self.duplicate_selected_or_last(self.cad_step_mm, 0.0) {
                    format!("Copied selection +{:.0} mm East", self.cad_step_mm)
                } else {
                    "No entity to duplicate".to_string()
                }
            }
            "m" | "move" => {
                if self.translate_selected_or_last(self.cad_step_mm, 0.0) {
                    format!("Moved selection +{:.0} mm East", self.cad_step_mm)
                } else {
                    "No entity to move".to_string()
                }
            }
            "x" | "mirror" => {
                let mx = bim_project
                    .plan_extents_mm()
                    .map_or(0.0, |(a, b)| 0.5 * (a[0] + b[0]));
                if self.mirror_selected_or_last(mx) {
                    format!("Mirrored across X={mx:.0} mm")
                } else {
                    "No entity to mirror".to_string()
                }
            }
            "undo" | "u" => {
                if self.undo() {
                    "Undo applied".to_string()
                } else {
                    "Nothing to undo".to_string()
                }
            }
            "redo" => {
                if self.redo() {
                    "Redo applied".to_string()
                } else {
                    "Nothing to redo".to_string()
                }
            }
            "fit" | "z" => {
                self.fit_world_to_bim_extents(bim_project.plan_extents_mm());
                "Fitted camera to BIM footprint (Z)".to_string()
            }
            "freeze" => {
                self.is_frozen = !self.is_frozen;
                format!("Canvas Frozen = {}", self.is_frozen)
            }
            _ => format!("Unknown CAD command '{}' — try @5000,0 | grid 4x3 6000 | array 3x2 | promote", trimmed),
        }
    }

    /// Deletes the selected entities (or latest entity) with undo support.
    pub fn delete_selected_or_last(&mut self) -> bool {
        let mut indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        indices.dedup();
        for idx in indices {
            if idx < self.entities.len() {
                self.entities.remove(idx);
            }
        }
        self.set_single_selection(None);
        true
    }

    pub fn clear_entities(&mut self, active_pillar: Option<NavigationPillar>) {
        self.push_undo();
        match active_pillar {
            Some(p) => {
                let badge = p.badge();
                self.entities.retain(|e| e.pillar_badge != badge);
            }
            None => self.entities.clear(),
        }
        self.selected_entity_id = None;
    }

    pub fn count_for_pillar(&self, pillar: NavigationPillar) -> usize {
        let badge = pillar.badge();
        self.entities
            .iter()
            .filter(|e| e.pillar_badge == badge)
            .count()
    }

    /// Builds a single synchronized workspace archive across all 15 modules.
    pub fn build_unified_archive(
        &self,
        bim_project: &BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
        workspace_settings: WorkspaceCanvasSettings,
    ) -> UnifiedWorkspaceArchive {
        UnifiedWorkspaceArchive {
            format_version: "EIGEN-UNIFIED-1.0".to_string(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            project_name: bim_project.project_name.clone(),
            bim_project: bim_project.clone(),
            members: members.to_vec(),
            parametric_variables: variables.to_vec(),
            canvas_entities: self.entities.clone(),
            workspace_settings,
            world_origin_lat_lon: self
                .ground_origin_anchored
                .then_some(self.ground_origin_lat_lon),
            map_center_lat_lon: Some(self.world_map.center_lat_lon),
            map_zoom: Some(self.world_map.zoom_level),
            globe_mode: self.globe_mode,
            planetary_world: self.earth_world.clone(),
            lifecycle_ops: {
                let mut ops = self.lifecycle_ops.clone();
                ops.site_coupling = self.active_site_coupling();
                ops
            },
            sha256_seal: String::new(),
        }
    }

    /// Exports the entire synchronized workspace as 1 unified `.eigen.json` file.
    pub fn export_unified_file(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
        workspace_settings: WorkspaceCanvasSettings,
    ) {
        let archive =
            self.build_unified_archive(bim_project, members, variables, workspace_settings);
        match archive.to_pretty_json() {
            Ok(json) => match DiskExporter::write_export_file(
                "EigenGraph_Unified_Workspace.eigen.json",
                &json,
            ) {
                Ok(dest) => {
                    self.status_message = Some(format!(
                        "Synced 1-file workspace ({} drawings, {} members) -> {dest}",
                        self.entities.len(),
                        members.len()
                    ));
                }
                Err(err) => {
                    self.status_message = Some(format!("Workspace export failed: {err}"));
                }
            },
            Err(err) => {
                self.status_message = Some(format!("Serialization error: {err}"));
            }
        }
    }

    /// Applies a verified unified archive back into the active application state.
    pub fn apply_unified_archive(
        &mut self,
        json_str: &str,
        bim_project: &mut BimProject,
        members: &mut Vec<StructuralMember>,
        variables: &mut Vec<ParametricVariable>,
    ) -> Result<WorkspaceCanvasSettings, String> {
        let archive = UnifiedWorkspaceArchive::from_verified_json(json_str)?;
        self.push_undo();
        *bim_project = archive.bim_project;
        *members = archive.members;
        *variables = archive.parametric_variables;
        self.entities = archive.canvas_entities;
        self.next_entity_id = self
            .entities
            .iter()
            .map(|e| e.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.is_frozen = archive.workspace_settings.freeze_by_default;
        self.canvas_height_ratio = archive
            .workspace_settings
            .canvas_height_ratio
            .clamp(0.45, 1.15);
        self.show_grid = archive.workspace_settings.show_grid;
        self.snap_to_grid = archive.workspace_settings.snap_to_grid;
        self.grid_spacing_mm = archive.workspace_settings.grid_spacing_mm.max(100.0);
        self.show_bim_base_layer = archive.workspace_settings.show_bim_base_layer;
        self.show_domain_overlay = archive.workspace_settings.show_domain_overlay;
        self.sync_all_layers = archive.workspace_settings.sync_all_layers;
        self.inspector_open = archive.workspace_settings.show_floating_inspector;
        if let Some(orig) = archive.world_origin_lat_lon {
            self.ground_origin_lat_lon = orig;
            self.ground_origin_anchored = true;
        }
        if let Some(center) = archive.map_center_lat_lon {
            self.world_map.set_center(center.0, center.1);
        }
        if let Some(zoom) = archive.map_zoom {
            self.world_map.zoom_level = zoom.clamp(1.0, 19.0);
        }
        self.globe_mode = archive.globe_mode;
        self.earth_world = archive.planetary_world;
        self.earth_world.recompute_vitals();
        self.lifecycle_ops = archive.lifecycle_ops;
        self.lifecycle_ops.site_coupling = self.active_site_coupling();
        self.status_message = Some(format!(
            "Imported verified 1-file workspace '{}' ({} canvas objects)",
            archive.project_name,
            self.entities.len()
        ));
        Ok(archive.workspace_settings)
    }

    /// Exports only the active module's entities when the user explicitly chooses to split files.
    pub fn export_split_module_file(&mut self, pillar: NavigationPillar) {
        let badge = pillar.badge();
        let layer_entities: Vec<CanvasEntity> = self
            .entities
            .iter()
            .filter(|e| e.pillar_badge == badge)
            .cloned()
            .collect();
        let payload = ModuleLayerSplitExport {
            format_version: "EIGEN-LAYER-SPLIT-1.0".to_string(),
            pillar_badge: badge.to_string(),
            pillar_name: pillar.clean_name().to_string(),
            entities: layer_entities,
        };
        match serde_json::to_string_pretty(&payload) {
            Ok(json) => {
                let filename = format!("EigenGraph_Layer_{badge}.json");
                match DiskExporter::write_export_file(&filename, &json) {
                    Ok(dest) => {
                        self.status_message = Some(format!(
                            "Split module layer [{badge}] exported -> {dest}"
                        ));
                    }
                    Err(err) => {
                        self.status_message = Some(format!("Split export failed: {err}"));
                    }
                }
            }
            Err(err) => {
                self.status_message = Some(format!("Split serialization failed: {err}"));
            }
        }
    }

    /// Exports synchronized canvas entities and BIM grids to vector SVG.
    pub fn export_svg(&self, bim_project: &BimProject, active_pillar: NavigationPillar) -> String {
        let mut svg = String::from(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-2000 -11500 16000 13500\" width=\"1200\" height=\"900\" style=\"background:#0f1117\">\n",
        );
        svg.push_str(&format!(
            "  <!-- EigenGraph Unified Canvas | Active Pillar: {} -->\n",
            active_pillar.clean_name()
        ));
        for g in &bim_project.grids {
            svg.push_str(&format!(
                "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#38bdf8\" stroke-opacity=\"0.35\" stroke-width=\"18\" stroke-dasharray=\"180,120\"/>\n",
                g.start.0, -g.start.1, g.end.0, -g.end.1
            ));
        }
        for w in &bim_project.walls {
            svg.push_str(&format!(
                "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#94a3b8\" stroke-width=\"{:.1}\"/>\n",
                w.start.0, -w.start.1, w.end.0, -w.end.1, w.thickness_mm.max(80.0)
            ));
        }
        for ent in &self.entities {
            if !self.sync_all_layers && ent.pillar_badge != active_pillar.badge() {
                continue;
            }
            let [r, g, b] = ent.color_rgb;
            let col = format!("rgb({r},{g},{b})");
            let sw = ent.stroke_width * 24.0;
            match &ent.kind {
                CanvasShapeKind::Line { start_mm, end_mm }
                | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    svg.push_str(&format!(
                        "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{col}\" stroke-width=\"{sw:.1}\"/>\n",
                        start_mm[0], -start_mm[1], end_mm[0], -end_mm[1]
                    ));
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    let x = min_mm[0].min(max_mm[0]);
                    let y = (-max_mm[1]).min(-min_mm[1]);
                    let w = (max_mm[0] - min_mm[0]).abs();
                    let h = (max_mm[1] - min_mm[1]).abs();
                    svg.push_str(&format!(
                        "  <rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"none\" stroke=\"{col}\" stroke-width=\"{sw:.1}\"/>\n"
                    ));
                }
                CanvasShapeKind::Circle {
                    center_mm,
                    radius_mm,
                } => {
                    svg.push_str(&format!(
                        "  <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"none\" stroke=\"{col}\" stroke-width=\"{sw:.1}\"/>\n",
                        center_mm[0], -center_mm[1], radius_mm
                    ));
                }
                CanvasShapeKind::Freehand { points_mm } => {
                    if points_mm.len() >= 2 {
                        let pts: Vec<String> = points_mm
                            .iter()
                            .map(|p| format!("{:.1},{:.1}", p[0], -p[1]))
                            .collect();
                        svg.push_str(&format!(
                            "  <polyline fill=\"none\" stroke=\"{col}\" stroke-width=\"{sw:.1}\" points=\"{}\"/>\n",
                            pts.join(" ")
                        ));
                    }
                }
                CanvasShapeKind::Callout { pos_mm, text } => {
                    let safe = text.replace('&', "&amp;").replace('<', "&lt;");
                    svg.push_str(&format!(
                        "  <text x=\"{:.1}\" y=\"{:.1}\" fill=\"{col}\" font-size=\"260\" font-family=\"monospace\">{safe}</text>\n",
                        pos_mm[0], -pos_mm[1]
                    ));
                }
            }
        }
        svg.push_str("</svg>\n");
        svg
    }

    /// Renders the unified canvas-first drawing workspace for the active sidebar module.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &mut BimProject,
        members: &mut Vec<StructuralMember>,
        variables: &mut Vec<ParametricVariable>,
        workspace_settings: WorkspaceCanvasSettings,
        sidebar_toggle_inset_px: f32,
    ) {
        ui_kit::set_canvas_frozen(ui.ctx(), self.is_frozen);

        let canvas_h = self.preferred_canvas_height(ui.ctx());

        Frame::new()
            .fill(theme.bg_sidebar)
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, theme.border_card))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                let outer_w = ui.available_width().max(160.0);
                ui.set_width(outer_w);
                ui.set_max_width(outer_w);

                self.render_viewport_core(
                    ui,
                    theme,
                    active_pillar,
                    bim_project,
                    members,
                    Vec2::new(outer_w.max(200.0), canvas_h),
                    sidebar_toggle_inset_px,
                    false,
                    true,
                    10.0,
                );

                ui.add_space(6.0);

                // 2. Coordinate & CAD Telemetry Bar directly below Viewport and above Bottom Toolbar
                self.render_status_footer(ui, theme, active_pillar, bim_project);

                // Optional status toast
                Self::render_toast_bar(ui, theme, &mut self.status_message);

                ui.add_space(6.0);

                // 3. Bottom Canvas HUD Toolbar (rich drawing + modify buttons below Coordinate Bar)
                self.render_canvas_toolbar(
                    ui,
                    theme,
                    active_pillar,
                    bim_project,
                    members,
                    variables,
                    workspace_settings.clone(),
                    sidebar_toggle_inset_px,
                );

                // 4. Bottom-Docked Collapsible Inspector Panel (exact full toolbar/viewport width)
                if self.inspector_open {
                    ui.add_space(6.0);
                    self.render_floating_inspector_panel(
                        ui,
                        theme,
                        active_pillar,
                        bim_project,
                        members,
                        variables,
                        workspace_settings,
                        false,
                    );
                }
            });
    }

    /// Phase 46: Renders the zero-chrome, edge-to-edge full-viewport Earth & BIM/CAD spatial canvas
    /// without below-viewport toolbars or scroll-locking frames.
    #[allow(clippy::too_many_arguments)]
    pub fn show_spatial_fullscreen(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &mut BimProject,
        members: &[StructuralMember],
        pointer_blocked_by_hud: bool,
    ) -> Rect {
        ui_kit::set_canvas_frozen(ui.ctx(), self.is_frozen);
        let avail = ui.available_size().max(Vec2::new(240.0, 240.0));
        self.render_viewport_core(
            ui,
            theme,
            active_pillar,
            bim_project,
            members,
            avail,
            0.0,
            pointer_blocked_by_hud,
            false,
            0.0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_viewport_core(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &mut BimProject,
        members: &[StructuralMember],
        viewport_size: Vec2,
        sidebar_toggle_inset_px: f32,
        pointer_blocked_by_hud: bool,
        show_freeze_pill: bool,
        corner_radius: f32,
    ) -> Rect {
        // Apply CAD Keyboard Shortcuts (Space freeze, Esc, Del, Ctrl+Z/Y, F8 Ortho, F3 Snap, single keys, G Globe, [ / ] Tilt)
        self.handle_universal_shortcuts(ui.ctx(), bim_project);

        // Step Earth animations (smooth auto-spin, inertia, Google Earth fly-to, planetary sim years)
        if self.globe_mode && !self.is_frozen {
            self.earth_world.step_animations(ui.ctx(), &mut self.world_map);
        }

        // 1. Main Interactive Drawing Viewport
        let (response, painter) = ui.allocate_painter(viewport_size, Sense::click_and_drag());
        let rect = response.rect;
        self.last_canvas_rect = Some(rect);
        self.last_inspector_rect = None;

        if self.globe_mode {
            self.last_globe_xform = Some(GlobeScreenTransform::from_view(
                rect,
                self.world_map.center_lat_lon.0,
                self.world_map.center_lat_lon.1,
                self.world_map.zoom_level,
                &self.earth_world.camera,
            ));
        } else {
            self.last_globe_xform = None;
        }

        painter.rect_filled(
            rect,
            CornerRadius::same(corner_radius.round() as u8),
            Self::canvas_bg(theme),
        );

        let (freeze_pill_rect, pointer_in_freeze_pill, freeze_pill_hovered) = if show_freeze_pill {
            Self::interact_floating_freeze_pill(
                ui,
                rect,
                "universal",
                &mut self.is_frozen,
                sidebar_toggle_inset_px,
            )
        } else {
            (Rect::NOTHING, false, false)
        };

        // Handle Pan / Zoom / Drawing / Smart Element Picking (respecting Freeze Canvas & Floating Overlays)
        self.handle_canvas_interaction(
            ui,
            &response,
            rect,
            active_pillar,
            bim_project,
            pointer_in_freeze_pill || pointer_blocked_by_hud,
        );

        // 1a. Phase 43 / Phase 35: 3D Google Earth Globe (default) or 2D Slippy World Map
        let active_globe_xform = if self.globe_mode {
            let xform = self.earth_world.render_3d_globe(
                ui,
                &painter,
                rect,
                theme,
                &self.world_map,
                self.ground_origin_lat_lon,
                self.ground_origin_anchored,
                self.show_grid,
                self.active_tool.earth_brush_action(),
                self.cursor_lat_lon,
                self.is_frozen,
            );
            self.last_globe_xform = Some(xform.clone());
            Some(xform)
        } else {
            self.last_globe_xform = None;
            self.world_map.render_world_background(
                ui,
                &painter,
                rect,
                theme,
                self.ground_origin_lat_lon,
                self.show_grid,
                false,
            );
            None
        };

        // 1b. Refresh live DEM TIN before immutable canvas borrows, then render Grid, Vector Urban, TIN & World Buildings
        if self.globe_mode && self.show_terrain_contours && self.use_live_dem_tiles {
            self.dem_cache.ensure_tile_for_lat_lon(
                ui.ctx(),
                self.ground_origin_lat_lon.0,
                self.ground_origin_lat_lon.1,
                self.world_map.zoom_level,
            );
            if !self.site_tin_is_live_dem {
                if let Some(live_tin) = self.dem_cache.build_site_tin(self.ground_origin_lat_lon, 400.0, 20) {
                    self.active_site_tin = Some(live_tin);
                    self.site_tin_is_live_dem = true;
                }
            }
        }
        let world_to_screen = |w: [f64; 2]| self.world_to_screen(rect, w);
        if self.show_grid && self.world_map.zoom_level >= 15.5 {
            self.paint_grid(&painter, rect, theme, &world_to_screen);
        }
        if self.globe_mode {
            if let Some(ref xform) = active_globe_xform {
                if self.world_map.vector_site_overlay
                    || self.world_map.tile_provider == crate::core::gis::TileSource::SyntheticVectorTerrain
                {
                    self.urban_vectors.ensure_osm_cell_loaded(
                        ui.ctx(),
                        self.world_map.center_lat_lon.0,
                        self.world_map.center_lat_lon.1,
                        self.world_map.zoom_level,
                    );
                    if self.world_map.zoom_level >= 12.5 && self.urban_vectors.feature_count() > 0 {
                        let (min_la, max_la, min_lo, max_lo) = self.visible_geo_bbox(xform);
                        let vis_feats = self.urban_vectors.snapshot_visible(min_la, max_la, min_lo, max_lo, 900);
                        self.earth_world.paint_urban_vector_features(&painter, xform, theme, &vis_feats);
                    }
                }
            }
            if self.show_terrain_contours {
                if let Some((ref tin, ref xform)) = self.active_site_tin.as_ref().zip(active_globe_xform.as_ref()) {
                    self.earth_world.paint_site_tin_contours_and_hazards(
                        &painter,
                        xform,
                        theme,
                        self.ground_origin_lat_lon,
                        0.0,
                        tin,
                    );
                }
            }
            self.paint_planetary_world_buildings(&painter, rect, theme);
            if self.show_neighborhood_hazards {
                self.paint_neighborhood_hazards(&painter, rect, theme);
            }
        }
        if self.show_bim_base_layer {
            self.paint_bim_base_layer(
                &painter,
                rect,
                theme,
                bim_project,
                members,
                active_pillar,
                &world_to_screen,
            );
        }
        if self.show_domain_overlay {
            self.paint_domain_overlay(
                &painter,
                rect,
                theme,
                active_pillar,
                bim_project,
                members,
                &world_to_screen,
            );
        }
        self.paint_entities(&painter, rect, theme, active_pillar, &world_to_screen);
        self.paint_live_preview(&painter, rect, theme, &world_to_screen);

        // Live Great-Circle Bridge / Corridor arc preview while dragging Earth megastructures
        if let (Some(start_ll), Some(kind)) = (
            self.pending_earth_arc_start,
            self.active_tool.earth_megastructure_kind(),
        ) {
            let arc_pts = crate::core::gis::great_circle_arc(start_ll, self.cursor_lat_lon, 20);
            let arc_color = kind.accent_color();
            let clip_arc = painter.with_clip_rect(rect);
            let proj_ll = |ll: (f64, f64)| -> Option<Pos2> {
                if let Some(ref xf) = active_globe_xform {
                    xf.project_lat_lon_elev(ll.0, ll.1, 0.0)
                        .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
                } else {
                    Some(self.world_map.lat_lon_to_screen(ll.0, ll.1, rect))
                }
            };
            for pair in arc_pts.windows(2) {
                if let (Some(p0), Some(p1)) = (proj_ll(pair[0]), proj_ll(pair[1])) {
                    clip_arc.line_segment([p0, p1], Stroke::new(2.6, arc_color));
                }
            }
            if let Some(p_end) = proj_ll(self.cursor_lat_lon) {
                clip_arc.circle_filled(p_end, 4.5, arc_color);
            }
        }

        let geo_to_screen = |lat: f64, lon: f64| -> Pos2 {
            if let Some(ref xf) = active_globe_xform {
                if let Some((pos, _, vis)) = xf.project_lat_lon_elev(lat, lon, 0.0) {
                    if vis {
                        return pos;
                    }
                }
                Pos2::new(-1e6, -1e6)
            } else {
                self.world_map.lat_lon_to_screen(lat, lon, rect)
            }
        };
        self.render_geodetic_measure_overlay(&painter, rect, theme, &geo_to_screen);

        // Highlight clicked Earth target pin when an open Earth location is inspected
        if let Some((plat, plon)) = self.last_picked_earth_lat_lon {
            let pin_pos = geo_to_screen(plat, plon);
            if rect.expand(8.0).contains(pin_pos) {
                let clip = painter.with_clip_rect(rect);
                clip.circle_stroke(pin_pos, 13.0, Stroke::new(1.8, theme.accent_blue.gamma_multiply(0.85)));
                clip.circle_stroke(pin_pos, 6.5, Stroke::new(2.0, Color32::WHITE));
                clip.circle_filled(pin_pos, 3.8, theme.accent_blue);
            }
        }

        // Highlight geodetic site anchor when zoomed out on World Map / 3D Globe
        if !self.globe_mode
            && self.world_map.zoom_level < 12.5
            && (self.ground_origin_anchored || !self.entities.is_empty() || !bim_project.is_empty())
        {
            let site_pos = world_to_screen([0.0, 0.0]);
            if rect.expand(16.0).contains(site_pos) {
                painter.circle_filled(site_pos, 6.0, theme.accent_blue);
                painter.circle_stroke(site_pos, 11.0, Stroke::new(1.6, theme.accent_blue));
                painter.text(
                    site_pos + Vec2::new(12.0, -6.0),
                    Align2::LEFT_BOTTOM,
                    format!(
                        "Site Origin ({:.3}°, {:.3}°)",
                        self.ground_origin_lat_lon.0, self.ground_origin_lat_lon.1
                    ),
                    egui::FontId::monospace(10.0),
                    theme.text_primary,
                );
            }
        }

        if show_freeze_pill {
            Self::paint_floating_freeze_pill(
                &painter,
                freeze_pill_rect,
                theme,
                self.is_frozen,
                freeze_pill_hovered,
            );
        }

        if corner_radius > 0.0 {
            Self::paint_rounded_viewport_border(
                &painter,
                rect,
                corner_radius,
                theme.bg_sidebar,
                theme.border_subtle,
            );
        }

        rect
    }

    fn handle_universal_shortcuts(&mut self, ctx: &egui::Context, bim_project: &BimProject) {
        let fb = Self::poll_cad_shortcuts(ctx, self.is_frozen);
        if fb.toggle_freeze {
            self.is_frozen = !self.is_frozen;
            self.pending_start_mm = None;
            self.active_freehand_mm.clear();
            self.status_message = Some(if self.is_frozen {
                "Canvas Frozen (Space) • Scroll safely".to_string()
            } else {
                "Canvas Live (Space) • CAD keys & pan/draw active".to_string()
            });
        }
        if fb.escape {
            self.pending_start_mm = None;
            self.pending_earth_arc_start = None;
            self.marquee_anchor_mm = None;
            self.marquee_current_mm = None;
            self.active_freehand_mm.clear();
            self.measure_active = false;
            self.set_single_selection(None);
            self.active_tool = UniversalCanvasTool::SelectPan;
        }
        if fb.undo {
            if self.active_tool.is_earth_tool() && self.earth_world.undo_earth() {
                self.status_message = Some("Undo Earth modification (Ctrl/Cmd+Z)".to_string());
            } else if self.undo() {
                self.status_message = Some("Undo canvas action (Ctrl/Cmd+Z)".to_string());
            } else if self.earth_world.undo_earth() {
                self.status_message = Some("Undo Earth modification (Ctrl/Cmd+Z)".to_string());
            }
        }
        if fb.redo {
            if self.active_tool.is_earth_tool() && self.earth_world.redo_earth() {
                self.status_message = Some("Redo Earth modification (Ctrl/Cmd+Y)".to_string());
            } else if self.redo() {
                self.status_message = Some("Redo canvas action (Ctrl/Cmd+Y)".to_string());
            } else if self.earth_world.redo_earth() {
                self.status_message = Some("Redo Earth modification (Ctrl/Cmd+Y)".to_string());
            }
        }
        if fb.delete_selected && self.delete_selected_or_last() {
            self.status_message = Some("Deleted canvas entity (Del)".to_string());
        }
        if fb.toggle_ortho {
            self.ortho_mode = !self.ortho_mode;
        }
        if fb.toggle_snap {
            self.snap_to_grid = !self.snap_to_grid;
        }
        if fb.toggle_osnap {
            self.osnap_enabled = !self.osnap_enabled;
            self.status_message = Some(format!(
                "Object Snap (OSNAP F4): {}",
                if self.osnap_enabled { "ON" } else { "OFF" }
            ));
        }
        if fb.fit_model {
            if let Some(extents) = bim_project.plan_extents_mm() {
                self.fit_world_to_bim_extents(Some(extents));
            } else {
                self.fit_world_or_site(!bim_project.is_empty());
            }
        }
        if !self.is_frozen && self.globe_mode && !ctx.egui_wants_keyboard_input() {
            let (tilt_down, tilt_up, num_idx) = ctx.input(|i| {
                let d = i.key_pressed(egui::Key::OpenBracket);
                let u = i.key_pressed(egui::Key::CloseBracket);
                let idx = if i.key_pressed(egui::Key::Num1) {
                    Some(0)
                } else if i.key_pressed(egui::Key::Num2) {
                    Some(1)
                } else if i.key_pressed(egui::Key::Num3) {
                    Some(2)
                } else if i.key_pressed(egui::Key::Num4) {
                    Some(3)
                } else if i.key_pressed(egui::Key::Num5) {
                    Some(4)
                } else if i.key_pressed(egui::Key::Num6) {
                    Some(5)
                } else if i.key_pressed(egui::Key::Num7) {
                    Some(6)
                } else if i.key_pressed(egui::Key::Num8) {
                    Some(7)
                } else if i.key_pressed(egui::Key::Num9) {
                    Some(8)
                } else {
                    None
                };
                (d, u, idx)
            });
            if tilt_down {
                self.earth_world.camera.tilt_deg =
                    (self.earth_world.camera.tilt_deg - 5.0).clamp(0.0, 65.0);
            }
            if tilt_up {
                self.earth_world.camera.tilt_deg =
                    (self.earth_world.camera.tilt_deg + 5.0).clamp(0.0, 65.0);
            }
            if let Some(i) = num_idx {
                if let Some(tool) = UniversalCanvasTool::EARTH_TOOLS.get(i).copied() {
                    self.active_tool = tool;
                    self.pending_start_mm = None;
                    self.pending_earth_arc_start = None;
                    self.status_message = Some(format!(
                        "Selected Earth Tool: {} (key {})",
                        tool.short_label(),
                        i + 1
                    ));
                }
            }
        }
        if let Some(ch) = fb.letter_key {
            let mirror_x = bim_project
                .plan_extents_mm()
                .map_or(0.0, |(a, b)| 0.5 * (a[0] + b[0]));
            match ch {
                'G' => {
                    self.globe_mode = !self.globe_mode;
                    self.pending_earth_arc_start = None;
                    self.status_message = Some(if self.globe_mode {
                        "3D Google Earth Globe mode active (G)".to_string()
                    } else {
                        "2D Slippy World Map mode active (G)".to_string()
                    });
                }
                'V' => self.active_tool = UniversalCanvasTool::BoxSelect,
                'L' => self.active_tool = UniversalCanvasTool::Line,
                'R' => self.active_tool = UniversalCanvasTool::Rectangle,
                'C' => self.active_tool = UniversalCanvasTool::Circle,
                'D' => self.active_tool = UniversalCanvasTool::Dimension,
                'T' => self.active_tool = UniversalCanvasTool::Callout,
                'E' => self.active_tool = UniversalCanvasTool::Eraser,
                'M' => {
                    if self.translate_selected_or_last(1000.0, 0.0) {
                        self.status_message =
                            Some("Moved selected/last entity +1000 mm East (M)".to_string());
                    }
                }
                'O' => {
                    if self.offset_selected_or_last(500.0) {
                        self.status_message =
                            Some("Offset duplicate +500 mm (O)".to_string());
                    }
                }
                'X' => {
                    if self.mirror_selected_or_last(mirror_x) {
                        self.status_message =
                            Some(format!("Mirrored duplicate across X={mirror_x:.0} mm (X)"));
                    }
                }
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_canvas_toolbar(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &mut BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
        workspace_settings: WorkspaceCanvasSettings,
        _sidebar_toggle_inset_px: f32,
    ) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                let outer_w = ui.available_width().max(160.0);
                ui.set_width(outer_w);
                ui.set_max_width(outer_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.horizontal_wrapped(|ui| {
                    // Active Module Pill
                    icons::pill_badge(
                        ui,
                        active_pillar.app_icon(),
                        format!(
                            "{} • {}",
                            active_pillar.clean_name(),
                            active_pillar.badge()
                        ),
                        theme.accent_blue,
                    );

                    ui.separator();

                    // Drawing Tools Palette
                    self.render_sketch_tools_bar(ui, theme, 26.0, false);

                    ui.separator();

                    // Color Swatches
                    self.render_color_swatches(ui, theme);

                    ui.separator();

                    // Undo, Redo, Ortho, Grid, Snap, Fit
                    if icons::icon_button(
                        ui,
                        AppIcon::Undo,
                        "Undo",
                        10.5,
                        theme.text_secondary,
                        theme.bg_card,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Undo last canvas change (Ctrl/Cmd+Z)")
                    .clicked()
                    {
                        self.undo();
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Redo,
                        "Redo",
                        10.5,
                        if self.redo_stack.is_empty() {
                            theme.text_muted
                        } else {
                            theme.text_primary
                        },
                        theme.bg_card,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Redo undone canvas change (Ctrl/Cmd+Y)")
                    .clicked()
                    {
                        self.redo();
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::RulerSquare,
                        "Ortho",
                        10.5,
                        if self.ortho_mode {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        },
                        if self.ortho_mode {
                            theme.accent_orange.gamma_multiply(0.85)
                        } else {
                            theme.bg_card
                        },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Lock orthogonal H/V axis while drawing lines & dims (F8)")
                    .clicked()
                    {
                        self.ortho_mode = !self.ortho_mode;
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Grid,
                        "Grid",
                        10.5,
                        if self.show_grid {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        },
                        if self.show_grid {
                            theme.accent_blue.gamma_multiply(0.75)
                        } else {
                            theme.bg_card
                        },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .clicked()
                    {
                        self.show_grid = !self.show_grid;
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::MagnetSnap,
                        "Snap",
                        10.5,
                        if self.snap_to_grid {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        },
                        if self.snap_to_grid {
                            theme.accent_blue.gamma_multiply(0.75)
                        } else {
                            theme.bg_card
                        },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Snap points to metric grid (F3)")
                    .clicked()
                    {
                        self.snap_to_grid = !self.snap_to_grid;
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Target,
                        "OSNAP",
                        10.5,
                        if self.osnap_enabled {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        },
                        if self.osnap_enabled {
                            theme.accent_green.gamma_multiply(0.85)
                        } else {
                            theme.bg_card
                        },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Object Snap (Endpoint, Midpoint, Perpendicular, Intersection) [F4]")
                    .clicked()
                    {
                        self.osnap_enabled = !self.osnap_enabled;
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::ZoomFit,
                        "Fit Model",
                        10.5,
                        theme.text_secondary,
                        theme.bg_card,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Auto-fit view to active BIM footprint or Global World Map (Z)")
                    .clicked()
                    {
                        if let Some(extents) = bim_project.plan_extents_mm() {
                            self.fit_world_to_bim_extents(Some(extents));
                        } else {
                            self.fit_world_or_site(!bim_project.is_empty());
                        }
                    }

                    ui.separator();

                    // CAD Modify Palette (Numeric Step, Move, Offset, Copy, Rotate 90°, Mirror, Delete)
                    ui.label(RichText::new("Step:").size(10.0).color(theme.text_muted));
                    ui.add(
                        egui::DragValue::new(&mut self.cad_step_mm)
                            .range(10.0..=50_000.0)
                            .speed(50.0)
                            .suffix(" mm"),
                    )
                    .on_hover_text("Precision step distance for Move / Copy / Offset");

                    if icons::icon_button(
                        ui,
                        AppIcon::ArrowRight,
                        "Move",
                        10.5,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Move selected or latest canvas entity East by Step mm (M)")
                    .clicked()
                    {
                        let step = self.cad_step_mm;
                        if self.translate_selected_or_last(step, 0.0) {
                            self.status_message =
                                Some(format!("Moved canvas entity +{step:.0} mm East (M)"));
                        }
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Layers,
                        "Copy",
                        10.5,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Duplicate selected or latest entity displaced +Step mm East")
                    .clicked()
                    {
                        let step = self.cad_step_mm;
                        if self.duplicate_selected_or_last(step, 0.0) {
                            self.status_message =
                                Some(format!("Duplicated entity +{step:.0} mm East"));
                        }
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Layers,
                        "Offset",
                        10.5,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Duplicate and offset selected or latest entity +500 mm (O)")
                    .clicked()
                    {
                        if self.offset_selected_or_last(500.0) {
                            self.status_message =
                                Some("Created +500 mm offset duplicate (O)".to_string());
                        }
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Compass,
                        "Rot 90°",
                        10.5,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Rotate selected or latest canvas entity +90° around centroid")
                    .clicked()
                    {
                        if self.rotate_selected_or_last(90.0) {
                            self.status_message =
                                Some("Rotated canvas entity +90° around centroid".to_string());
                        }
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::SectionCut,
                        "Mirror",
                        10.5,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Mirror selected or latest entity across project centerline (X)")
                    .clicked()
                    {
                        let mirror_x = bim_project
                            .plan_extents_mm()
                            .map_or(0.0, |(a, b)| 0.5 * (a[0] + b[0]));
                        if self.mirror_selected_or_last(mirror_x) {
                            self.status_message = Some(format!(
                                "Mirrored duplicate across X={mirror_x:.0} mm (X)"
                            ));
                        }
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Trash,
                        "Delete",
                        10.5,
                        theme.accent_red,
                        theme.bg_card,
                        Stroke::new(1.0, theme.accent_red.gamma_multiply(0.35)),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Delete selected or latest entity (Del)")
                    .clicked()
                    {
                        if self.delete_selected_or_last() {
                            self.status_message =
                                Some("Deleted selected/latest canvas entity (Del)".to_string());
                        }
                    }

                    if (!self.selected_entity_ids.is_empty() || self.selected_entity_id.is_some())
                        && icons::icon_button(
                            ui,
                            AppIcon::Close,
                            "Clear Sel",
                            10.0,
                            theme.text_secondary,
                            theme.bg_card,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Deselect active entities (Esc)")
                        .clicked()
                    {
                        self.set_single_selection(None);
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Grid,
                        "Grid 4x3",
                        10.5,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Generate parametric 4x3 structural grid (@ 6000x5000 mm)")
                    .clicked()
                    {
                        let lines = self.generate_structural_grid(
                            4,
                            3,
                            6000.0,
                            5000.0,
                            active_pillar,
                            [0.0, 0.0],
                        );
                        self.status_message = Some(format!(
                            "Generated 4x3 structural grid ({lines} axes @ 6000x5000 mm)"
                        ));
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Building,
                        "To BIM",
                        10.5,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Promote selected 2D sketch lines/rectangles into native 3D BIM walls, beams, columns & slabs (PROMOTE)")
                    .clicked()
                    {
                        let (w, r, s) = self.promote_selection_to_bim(bim_project);
                        self.status_message = Some(if w + r + s > 0 {
                            format!("Promoted selection -> +{w} Walls, +{r} Beams, +{s} Slabs in 3D BIM")
                        } else {
                            "Draw or select Lines/Rectangles first to promote to 3D BIM".to_string()
                        });
                    }

                    ui.separator();

                    // Phase 35: World Map Basemap & Ground Scale Controls
                    self.render_world_map_toolbar_controls(ui, theme, 26.0);

                    ui.separator();

                    // Collapsible Floating Inspector Panel Toggle
                    let insp_bg = if self.inspector_open {
                        theme.accent_green
                    } else {
                        theme.bg_card
                    };
                    let insp_fg = if self.inspector_open {
                        Color32::WHITE
                    } else {
                        theme.text_secondary
                    };
                    if icons::icon_button(
                        ui,
                        AppIcon::SlidersPanel,
                        "Panel",
                        10.5,
                        insp_fg,
                        insp_bg,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Show or hide floating canvas & domain inspector panel")
                    .clicked()
                    {
                        self.inspector_open = !self.inspector_open;
                    }

                    // Single-File Unified Sync Save
                    if icons::icon_button(
                        ui,
                        AppIcon::SaveDisk,
                        "Save 1-File",
                        10.5,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text(
                        "Save all 15 modules, BIM geometry, and canvas drawings into 1 unified .eigen.json file",
                    )
                    .clicked()
                    {
                        self.export_unified_file(
                            bim_project,
                            members,
                            variables,
                            workspace_settings,
                        );
                    }

                    ui.separator();

                    // Precision Relative/Polar CAD CLI Input + Shortcuts Palette Toggle
                    let cmd_w = crate::ui_kit::clamped_row_width(ui, 90.0, 175.0, 125.0);
                    crate::ui_kit::wrap_before(ui, cmd_w + 150.0);
                    let cmd_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.cad_cmd_input)
                            .desired_width(cmd_w)
                            .hint_text("@dx,dy | grid | array | promote"),
                    );
                    let run_cad = icons::icon_button(
                        ui,
                        AppIcon::Play,
                        "Run",
                        10.0,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .clicked()
                        || (cmd_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    if run_cad && !self.cad_cmd_input.trim().is_empty() {
                        let raw = std::mem::take(&mut self.cad_cmd_input);
                        self.status_message =
                            Some(self.execute_precision_command(&raw, active_pillar, bim_project));
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Shield,
                        "? Shortcuts",
                        10.0,
                        if self.shortcuts_modal_open {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        },
                        if self.shortcuts_modal_open {
                            theme.accent_blue
                        } else {
                            theme.bg_card
                        },
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Open Keyboard Shortcuts, Precision CLI Syntax & Module Quick-Jump (Ctrl/Cmd+K)")
                    .clicked()
                    {
                        self.shortcuts_modal_open = !self.shortcuts_modal_open;
                    }
                });

                if self.globe_mode || self.active_tool.is_earth_tool() {
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(4.0);
                    self.render_earth_sandbox_bar(ui, theme);
                }
            });
    }

    /// Earth Terraforming, Sea-Level Flood Defense & Megaproject Game Dock rendered cleanly outside the 3D Globe viewport.
    fn render_earth_sandbox_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        use crate::core::gis::EarthScenario;

        let inner_w = ui.available_width().max(160.0);
        ui.set_max_width(inner_w);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.spacing_mut().slider_width = crate::ui_kit::clamped_row_width(ui, 52.0, 92.0, 108.0);

        // Row A: Earth Sculpting & Megaproject Brushes (keys 1-9) + Brush Size + Undo/Redo/Reset
        ui.horizontal_wrapped(|ui| {
            icons::icon_label(
                ui,
                AppIcon::Globe,
                12.0,
                theme.accent_blue,
                "Earth Sandbox:",
                10.5,
                theme.text_primary,
                true,
            );

            for tool in UniversalCanvasTool::EARTH_TOOLS {
                let is_sel = self.active_tool == tool;
                let bg = if is_sel {
                    theme.accent_blue
                } else {
                    theme.bg_card
                };
                let fg = if is_sel {
                    Color32::WHITE
                } else {
                    theme.text_secondary
                };
                if icons::icon_button(
                    ui,
                    tool.app_icon(),
                    tool.short_label(),
                    10.0,
                    fg,
                    bg,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .on_hover_text(tool.tooltip())
                .clicked()
                {
                    self.active_tool = tool;
                    self.pending_start_mm = None;
                    self.pending_earth_arc_start = None;
                    self.active_freehand_mm.clear();
                }
            }

            ui.separator();

            for (lbl, cells) in [("S", 0_usize), ("M", 1_usize), ("L", 2_usize)] {
                let sel = self.earth_world.brush_radius_cells == cells;
                if icons::icon_button(
                    ui,
                    AppIcon::Target,
                    lbl,
                    9.5,
                    if sel { Color32::WHITE } else { theme.text_secondary },
                    if sel { theme.accent_green } else { theme.bg_card },
                    Stroke::NONE,
                    CornerRadius::same(5),
                    Vec2::new(0.0, 22.0),
                )
                .on_hover_text(format!(
                    "Planetary brush footprint radius: {}x{} cells",
                    cells * 2 + 1,
                    cells * 2 + 1
                ))
                .clicked()
                {
                    self.earth_world.brush_radius_cells = cells;
                }
            }

            if icons::icon_button(
                ui,
                AppIcon::Undo,
                "Undo Earth",
                9.5,
                theme.text_secondary,
                theme.bg_card,
                Stroke::NONE,
                CornerRadius::same(5),
                Vec2::new(0.0, 22.0),
            )
            .on_hover_text("Undo last planetary terrain or megastructure change")
            .clicked()
                && self.earth_world.undo_earth()
            {
                self.status_message = Some("Undid last planetary Earth modification".to_string());
            }

            if icons::icon_button(
                ui,
                AppIcon::Trash,
                "Reset Earth",
                9.5,
                theme.accent_red,
                theme.accent_red.gamma_multiply(0.12),
                Stroke::NONE,
                CornerRadius::same(5),
                Vec2::new(0.0, 22.0),
            )
            .on_hover_text("Reset Earth terrain cells & megastructures for current scenario")
            .clicked()
            {
                self.earth_world.reset_earth();
                self.pending_earth_arc_start = None;
                self.status_message = Some("Reset Earth planetary edits".to_string());
            }
        });

        ui.add_space(4.0);

        // Row B: Camera Tilt/Spin/Lighting + Sea Level Slider + Mission Selector + Live Earth Vitals
        ui.horizontal_wrapped(|ui| {
            let mission_w = crate::ui_kit::clamped_row_width(ui, 92.0, 148.0, 48.0);
            crate::ui_kit::wrap_before(ui, mission_w + 56.0);
            let current_scenario = self.earth_world.scenario;
            egui::ComboBox::from_id_salt("earth_mission_scenario_combo")
                .width(mission_w)
                .selected_text(current_scenario.label())
                .show_ui(ui, |ui| {
                    for sc in EarthScenario::all() {
                        if ui
                            .selectable_label(current_scenario == *sc, sc.label())
                            .on_hover_text(sc.goal_summary())
                            .clicked()
                        {
                            let (lat, lon, z) = self.earth_world.load_scenario(*sc);
                            self.pending_earth_arc_start = None;
                            self.earth_world
                                .start_fly_to(&self.world_map, lat, lon, z, sc.label());
                            self.status_message =
                                Some(format!("Earth Mission [{}]: {}", sc.label(), sc.goal_summary()));
                        }
                    }
                });

            let slider_w = crate::ui_kit::clamped_row_width(ui, 42.0, 82.0, 120.0);
            ui.spacing_mut().slider_width = slider_w;
            crate::ui_kit::wrap_before(ui, slider_w + 96.0);
            ui.add(
                egui::Slider::new(&mut self.earth_world.camera.tilt_deg, 0.0..=65.0)
                    .text("Tilt°")
                    .max_decimals(0),
            );

            crate::ui_kit::wrap_before(ui, slider_w + 96.0);
            let prev_sea = self.earth_world.vitals.sea_level_m;
            let sea_resp = ui.add(
                egui::Slider::new(&mut self.earth_world.vitals.sea_level_m, -50.0..=50.0)
                    .text("Sea m")
                    .max_decimals(1),
            );
            if sea_resp.changed() && (self.earth_world.vitals.sea_level_m - prev_sea).abs() > 0.01 {
                self.earth_world.recompute_vitals();
            }

            let spin = self.earth_world.camera.auto_spin;
            if icons::icon_button(
                ui,
                AppIcon::Compass,
                if spin { "Spin ON" } else { "Spin" },
                9.5,
                if spin { Color32::WHITE } else { theme.text_secondary },
                if spin { theme.accent_blue } else { theme.bg_card },
                Stroke::NONE,
                CornerRadius::same(5),
                Vec2::new(0.0, 22.0),
            )
            .on_hover_text("Toggle gentle Earth orbital rotation")
            .clicked()
            {
                self.earth_world.camera.auto_spin = !spin;
            }

            let day_night = self.earth_world.camera.day_night_lighting;
            if icons::icon_button(
                ui,
                AppIcon::Sun,
                if day_night { "SunLit" } else { "FlatLit" },
                9.5,
                if day_night { Color32::WHITE } else { theme.text_secondary },
                if day_night { theme.accent_orange } else { theme.bg_card },
                Stroke::NONE,
                CornerRadius::same(5),
                Vec2::new(0.0, 22.0),
            )
            .on_hover_text("Toggle real-time solar terminator day/night globe shading")
            .clicked()
            {
                self.earth_world.camera.day_night_lighting = !day_night;
            }

            let playing = self.earth_world.sim_playing;
            if icons::icon_button(
                ui,
                if playing { AppIcon::Pause } else { AppIcon::Play },
                if playing { "Pause Sim" } else { "Play Sim" },
                9.5,
                if playing { Color32::WHITE } else { theme.text_secondary },
                if playing { theme.accent_green } else { theme.bg_card },
                Stroke::NONE,
                CornerRadius::same(5),
                Vec2::new(0.0, 22.0),
            )
            .on_hover_text("Auto-advance Earth climate & engineering budget years")
            .clicked()
            {
                self.earth_world.sim_playing = !playing;
            }

            if icons::icon_button(
                ui,
                AppIcon::Sparkles,
                "+1 Yr",
                9.5,
                theme.text_primary,
                theme.bg_card,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(5),
                Vec2::new(0.0, 22.0),
            )
            .on_hover_text("Advance planetary simulation by 1 year")
            .clicked()
            {
                self.earth_world.tick_simulation_years(1);
            }

            let v = &self.earth_world.vitals;
            let hab_color = if v.habitability_score >= 78.0 {
                theme.accent_green
            } else if v.habitability_score >= 55.0 {
                theme.accent_orange
            } else {
                theme.accent_red
            };
            for vital in [
                format!("Yr {}", v.year),
                format!("CO2 {:.0}ppm", v.co2_ppm),
                format!("ΔT {:+.2}°C", v.temp_anomaly_c),
                format!("Hab {:.0}/100", v.habitability_score),
                format!("Flood {}", v.flooded_coast_cells),
                format!("Clean {:.0}GW", v.clean_gw),
                format!("${:.0}B", v.budget_billion_usd),
            ] {
                ui.label(
                    RichText::new(vital)
                        .size(10.0)
                        .monospace()
                        .color(hab_color),
                );
            }
        });
        let (prog, prog_text, done) = self.earth_world.scenario_progress();
        ui.add(
            egui::Label::new(
                RichText::new(format!("[{:.0}%] {}", prog * 100.0, prog_text))
                    .size(10.0)
                    .strong()
                    .color(if done {
                        theme.accent_green
                    } else {
                        theme.accent_blue
                    }),
            )
            .wrap(),
        );
    }

    /// World Map basemap selector, Global overview, Ground 1:200 site zoom, and Site Origin anchor controls.
    pub fn render_world_map_toolbar_controls(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        btn_h: f32,
    ) {
        use crate::core::gis::TileSource;
        if icons::icon_button(
            ui,
            AppIcon::Globe,
            if self.globe_mode { "3D Earth" } else { "2D Map" },
            10.5,
            if self.globe_mode {
                Color32::WHITE
            } else {
                theme.text_primary
            },
            if self.globe_mode {
                theme.accent_blue
            } else {
                theme.bg_card
            },
            Stroke::NONE,
            CornerRadius::same(6),
            Vec2::new(0.0, btn_h),
        )
        .on_hover_text("Toggle between 3D Google Earth Globe and 2D Flat World Map (Shortcut: G)")
        .clicked()
        {
            self.globe_mode = !self.globe_mode;
            self.pending_earth_arc_start = None;
            self.status_message = Some(if self.globe_mode {
                "Switched to 3D Google Earth Globe mode (G)".to_string()
            } else {
                "Switched to 2D Flat World Map mode (G)".to_string()
            });
        }

        let short_name = |src: TileSource| -> &'static str {
            match src {
                TileSource::OpenStreetMap => "OSM Streets",
                TileSource::EsriWorldImagery => "Satellite",
                TileSource::OpenTopoMap => "Topo",
                TileSource::CartoDbDarkMatter => "Dark Map",
                TileSource::SyntheticVectorTerrain => "Offline Vector",
            }
        };

        let combo_w = crate::ui_kit::clamped_row_width(ui, 64.0, 102.0, 28.0);
        crate::ui_kit::wrap_before(ui, combo_w + 32.0);
        egui::ComboBox::from_id_salt("universal_world_basemap_combo")
            .width(combo_w)
            .selected_text(short_name(self.world_map.tile_provider))
            .show_ui(ui, |ui| {
                for src in [
                    TileSource::OpenStreetMap,
                    TileSource::EsriWorldImagery,
                    TileSource::OpenTopoMap,
                    TileSource::CartoDbDarkMatter,
                    TileSource::SyntheticVectorTerrain,
                ] {
                    ui.selectable_value(
                        &mut self.world_map.tile_provider,
                        src,
                        src.display_name(),
                    );
                }
            });

        if icons::icon_button(
            ui,
            AppIcon::Globe,
            "World",
            10.5,
            theme.text_secondary,
            theme.bg_card,
            Stroke::NONE,
            CornerRadius::same(6),
            Vec2::new(0.0, btn_h),
        )
        .on_hover_text("Reset view to full Global Earth orbit showing all continents")
        .clicked()
        {
            if self.globe_mode {
                self.earth_world.camera.tilt_deg = 12.0;
                self.earth_world.camera.heading_deg = 0.0;
                self.earth_world
                    .start_fly_to(&self.world_map, 5.0, 98.0, 1.65, "Global Earth");
            } else {
                self.world_map.reset_to_global_view();
            }
        }

        if icons::icon_button(
            ui,
            AppIcon::Target,
            "Ground 1:200",
            10.5,
            theme.text_secondary,
            theme.bg_card,
            Stroke::NONE,
            CornerRadius::same(6),
            Vec2::new(0.0, btn_h),
        )
        .on_hover_text("Zoom directly onto Earth surface at architectural/structural drawing scale")
        .clicked()
        {
            let target = if self.ground_origin_anchored {
                self.ground_origin_lat_lon
            } else {
                self.world_map.center_lat_lon
            };
            self.ground_origin_lat_lon = target;
            self.ground_origin_anchored = true;
            if self.globe_mode {
                self.earth_world
                    .start_fly_to(&self.world_map, target.0, target.1, 17.5, "Ground 1:200");
            } else {
                self.world_map.zoom_to_ground_site(target.0, target.1, 17.5);
            }
        }

        if icons::icon_button(
            ui,
            AppIcon::MapPin,
            "Set Origin",
            10.5,
            if self.ground_origin_anchored {
                theme.accent_blue
            } else {
                theme.text_secondary
            },
            theme.bg_card,
            Stroke::NONE,
            CornerRadius::same(6),
            Vec2::new(0.0, btn_h),
        )
        .on_hover_text("Anchor local ground CAD origin (0, 0) mm at current World Map center")
        .clicked()
        {
            let center = self.world_map.center_lat_lon;
            self.ground_origin_lat_lon = center;
            self.ground_origin_anchored = true;
            self.status_message = Some(format!(
                "Ground origin (0, 0) mm anchored at {:.5}°, {:.5}°",
                center.0, center.1
            ));
        }

        // Phase 37 (1D-04): Earth City / Geodetic Coordinate Search & Distance/Area Measurement
        let search_w = crate::ui_kit::clamped_row_width(ui, 64.0, 110.0, 82.0);
        crate::ui_kit::wrap_before(ui, search_w + 80.0);
        let search_resp = ui.add(
            egui::TextEdit::singleline(&mut self.earth_search_query)
                .hint_text("City or lat,lon...")
                .desired_width(search_w),
        );
        let search_enter =
            search_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let search_click = icons::icon_button(
            ui,
            AppIcon::Search,
            "Go",
            10.5,
            theme.text_primary,
            theme.bg_card,
            Stroke::new(1.0, theme.border_subtle),
            CornerRadius::same(6),
            Vec2::new(0.0, btn_h),
        )
        .on_hover_text("Jump Earth map to a city (e.g. Jakarta, Tokyo, London) or 'lat, lon'")
        .clicked();

        if search_enter || search_click {
            if let Some((label, lat, lon, zoom)) =
                crate::core::gis::SlippyMapState::resolve_location_query(&self.earth_search_query)
            {
                if self.globe_mode {
                    self.earth_world
                        .start_fly_to(&self.world_map, lat, lon, zoom, &label);
                } else {
                    self.world_map.zoom_to_ground_site(lat, lon, zoom);
                }
                self.ground_origin_lat_lon = (lat, lon);
                self.ground_origin_anchored = true;
                self.status_message = Some(format!(
                    "Located '{}' -> {:.4}°, {:.4}° (Origin anchored)",
                    label, lat, lon
                ));
            } else if !self.earth_search_query.trim().is_empty() {
                self.status_message = Some(
                    "Location not matched; enter 'lat, lon' (e.g. -6.2088, 106.8456) or city name."
                        .to_string(),
                );
            }
        }

        if icons::icon_button(
            ui,
            AppIcon::DimensionRuler,
            "Measure",
            10.5,
            if self.measure_active {
                Color32::WHITE
            } else {
                theme.text_secondary
            },
            if self.measure_active {
                theme.accent_orange
            } else {
                theme.bg_card
            },
            Stroke::NONE,
            CornerRadius::same(6),
            Vec2::new(0.0, btn_h),
        )
        .on_hover_text("Click Earth surface points to measure geodesic distance (m/km) or parcel area (m²/ha)")
        .clicked()
        {
            self.measure_active = !self.measure_active;
        }

        if self.measure_active || !self.measure_points_lat_lon.is_empty() {
            let area_lbl = if self.measure_area_mode { "Area" } else { "Dist" };
            if ui
                .add(
                    egui::Button::new(RichText::new(area_lbl).size(10.0).color(Color32::WHITE))
                        .fill(if self.measure_area_mode {
                            theme.accent_green
                        } else {
                            theme.accent_blue
                        })
                        .corner_radius(CornerRadius::same(5))
                        .min_size(Vec2::new(0.0, btn_h)),
                )
                .on_hover_text("Switch between Geodesic Distance and Closed Parcel Area mode")
                .clicked()
            {
                self.measure_area_mode = !self.measure_area_mode;
            }
            if !self.measure_points_lat_lon.is_empty()
                && icons::icon_button(
                    ui,
                    AppIcon::Trash,
                    "Clear Meas",
                    10.0,
                    theme.accent_red,
                    theme.bg_card,
                    Stroke::NONE,
                    CornerRadius::same(5),
                    Vec2::new(0.0, btn_h),
                )
                .clicked()
            {
                self.measure_points_lat_lon.clear();
            }
        }
    }

    /// Fits either the active site drawing footprint (if non-empty) or resets to the Global World Map overview.
    pub fn fit_world_or_site(&mut self, has_bim_elements: bool) {
        if self.ground_origin_anchored || !self.entities.is_empty() || has_bim_elements {
            self.world_map.zoom_to_ground_site(
                self.ground_origin_lat_lon.0,
                self.ground_origin_lat_lon.1,
                17.5,
            );
        } else {
            self.world_map.reset_to_global_view();
        }
        self.pan = Vec2::ZERO;
        self.zoom = 1.0;
    }

    /// Auto-fits the World Map camera smoothly around planar BIM extents `[min_mm, max_mm]`.
    pub fn fit_world_to_bim_extents(&mut self, extents_mm: Option<([f64; 2], [f64; 2])>) {
        if let Some((min_mm, max_mm)) = extents_mm {
            let cx = 0.5 * (min_mm[0] + max_mm[0]);
            let cy = 0.5 * (min_mm[1] + max_mm[1]);
            let (lat, lon) = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                self.ground_origin_lat_lon,
                [cx, cy],
            );
            let span_m = ((max_mm[0] - min_mm[0]).abs().max((max_mm[1] - min_mm[1]).abs())
                / 1000.0)
                .max(6.0);
            let cos_lat = lat.to_radians().cos().abs().max(0.2);
            let desired_res_m_px = (span_m / 420.0).max(0.01);
            let target_zoom = ((156_543.03392 * cos_lat / desired_res_m_px).log2() as f32)
                .clamp(14.0, 19.0);
            self.ground_origin_anchored = true;
            self.world_map.zoom_to_ground_site(lat, lon, target_zoom);
            self.pan = Vec2::ZERO;
            self.zoom = 1.0;
        } else {
            self.fit_world_or_site(false);
        }
    }

    /// Converts a viewport screen position into `(lat_deg, lon_deg, hit_globe)` using exact ray-sphere picking when in 3D Globe mode.
    pub fn screen_to_lat_lon_with_hit(&self, screen: Pos2, rect: Rect) -> (f64, f64, bool) {
        if self.globe_mode {
            let xform = self
                .last_globe_xform
                .as_ref()
                .filter(|x| x.rect == rect)
                .cloned()
                .unwrap_or_else(|| {
                    GlobeScreenTransform::from_view(
                        rect,
                        self.world_map.center_lat_lon.0,
                        self.world_map.center_lat_lon.1,
                        self.world_map.zoom_level,
                        &self.earth_world.camera,
                    )
                });
            xform.screen_to_lat_lon(screen)
        } else {
            let (lat, lon) = self.world_map.screen_to_lat_lon(screen, rect);
            (lat, lon, true)
        }
    }

    /// Anchors ground `(0, 0) mm` origin to `screen_pos` on Earth if the workspace is still blank.
    pub fn ensure_ground_origin_at_screen(&mut self, screen_pos: Pos2, rect: Rect) {
        if !self.ground_origin_anchored && self.entities.is_empty() {
            let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(screen_pos, rect);
            if hit {
                self.ground_origin_lat_lon = (lat, lon);
                self.ground_origin_anchored = true;
            }
        }
    }

    /// Renders Universal Canvas drawing tools palette (optionally omitting `SelectPan`).
    pub fn render_sketch_tools_bar(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        button_height: f32,
        skip_select_pan: bool,
    ) {
        for tool in UniversalCanvasTool::ALL {
            if skip_select_pan && tool == UniversalCanvasTool::SelectPan {
                continue;
            }
            let is_sel = self.active_tool == tool;
            let bg = if is_sel {
                theme.accent_blue
            } else {
                theme.bg_card
            };
            let fg = if is_sel {
                Color32::WHITE
            } else {
                theme.text_secondary
            };
            if icons::icon_button(
                ui,
                tool.app_icon(),
                tool.short_label(),
                10.5,
                fg,
                bg,
                Stroke::NONE,
                CornerRadius::same(6),
                Vec2::new(0.0, button_height),
            )
            .on_hover_text(tool.tooltip())
            .clicked()
            {
                self.active_tool = tool;
                self.pending_start_mm = None;
                self.active_freehand_mm.clear();
            }
        }
    }

    /// Renders compact circular ink color swatches shared across Universal Canvas views.
    pub fn render_color_swatches(&mut self, ui: &mut Ui, theme: &MacTheme) {
        for (idx, (cname, rgb)) in PALETTE_SWATCHES.iter().enumerate() {
            let c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            let sel = self.active_color_idx == idx;
            let stroke = if sel {
                Stroke::new(2.0, theme.text_primary)
            } else {
                Stroke::new(1.0, theme.border_card)
            };
            let (srect, sresp) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::click());
            ui.painter().circle_filled(srect.center(), 7.0, c);
            ui.painter().circle_stroke(srect.center(), 7.5, stroke);
            if sresp.on_hover_text(format!("Ink: {cname}")).clicked() {
                self.active_color_idx = idx;
            }
        }
    }

    pub fn world_origin_and_scale(&self, rect: Rect) -> (Pos2, f32) {
        let origin = self.world_to_screen(rect, [0.0, 0.0]);
        let scale = self.world_map.pixels_per_mm_at_origin(self.ground_origin_lat_lon);
        (origin, scale)
    }

    pub fn world_to_screen(&self, rect: Rect, world_mm: [f64; 2]) -> Pos2 {
        if self.globe_mode {
            let xform = self
                .last_globe_xform
                .as_ref()
                .filter(|x| x.rect == rect)
                .cloned()
                .unwrap_or_else(|| {
                    GlobeScreenTransform::from_view(
                        rect,
                        self.world_map.center_lat_lon.0,
                        self.world_map.center_lat_lon.1,
                        self.world_map.zoom_level,
                        &self.earth_world.camera,
                    )
                });
            let (lat, lon) =
                SlippyMapState::world_mm_to_lat_lon(self.ground_origin_lat_lon, world_mm);
            if let Some((pos, _cam_z, visible)) = xform.project_lat_lon_elev(lat, lon, 0.0) {
                if visible {
                    return pos;
                }
            }
            return Pos2::new(-1e6, -1e6);
        }
        self.world_map
            .world_mm_to_screen(self.ground_origin_lat_lon, world_mm, rect)
    }

    pub fn screen_to_world(&self, rect: Rect, screen: Pos2) -> [f64; 2] {
        if self.globe_mode {
            let (lat, lon, _) = self.screen_to_lat_lon_with_hit(screen, rect);
            return SlippyMapState::lat_lon_to_world_mm(self.ground_origin_lat_lon, lat, lon);
        }
        self.world_map
            .screen_to_world_mm(self.ground_origin_lat_lon, screen, rect)
    }

    /// Snaps `raw_mm` using Object Snap (OSNAP: Endpoint, Midpoint, Perpendicular, Intersection) and grid snap fallback.
    pub fn snap_point(&self, raw_mm: [f64; 2]) -> [f64; 2] {
        self.snap_point_with_osnap(
            raw_mm,
            self.pending_start_mm,
            None,
            self.last_osnap_radius_mm.max(350.0),
        )
        .0
    }

    /// Resolves exact CAD OSNAP (`Endpoint`, `Midpoint`, `Perpendicular`, `Intersection`) against entities and optional `BimProject`, falling back to grid snap.
    pub fn snap_point_with_osnap(
        &self,
        raw_mm: [f64; 2],
        anchor_mm: Option<[f64; 2]>,
        bim_project: Option<&BimProject>,
        tol_mm: f64,
    ) -> ([f64; 2], OsnapKind) {
        if self.osnap_enabled && tol_mm > 1.0 {
            let mut endpoints: Vec<[f64; 2]> = Vec::new();
            let mut segments: Vec<([f64; 2], [f64; 2])> = Vec::new();

            for ent in &self.entities {
                match &ent.kind {
                    CanvasShapeKind::Line { start_mm, end_mm }
                    | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                        endpoints.push(*start_mm);
                        endpoints.push(*end_mm);
                        segments.push((*start_mm, *end_mm));
                    }
                    CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                        let c0 = *min_mm;
                        let c1 = [max_mm[0], min_mm[1]];
                        let c2 = *max_mm;
                        let c3 = [min_mm[0], max_mm[1]];
                        endpoints.extend([c0, c1, c2, c3]);
                        segments.extend([(c0, c1), (c1, c2), (c2, c3), (c3, c0)]);
                    }
                    CanvasShapeKind::Circle {
                        center_mm,
                        radius_mm,
                    } => {
                        endpoints.push(*center_mm);
                        endpoints.push([center_mm[0] + radius_mm, center_mm[1]]);
                        endpoints.push([center_mm[0] - radius_mm, center_mm[1]]);
                        endpoints.push([center_mm[0], center_mm[1] + radius_mm]);
                        endpoints.push([center_mm[0], center_mm[1] - radius_mm]);
                    }
                    CanvasShapeKind::Freehand { points_mm } => {
                        if let (Some(&first), Some(&last)) = (points_mm.first(), points_mm.last()) {
                            endpoints.push(first);
                            endpoints.push(last);
                        }
                    }
                    CanvasShapeKind::Callout { pos_mm, .. } => {
                        endpoints.push(*pos_mm);
                    }
                }
            }

            if let Some(bim) = bim_project {
                for w in &bim.walls {
                    let s = [w.start.0, w.start.1];
                    let e = [w.end.0, w.end.1];
                    endpoints.push(s);
                    endpoints.push(e);
                    segments.push((s, e));
                }
                for r in &bim.runs {
                    let s = [r.start.0, r.start.1];
                    let e = [r.end.0, r.end.1];
                    endpoints.push(s);
                    endpoints.push(e);
                    segments.push((s, e));
                }
                for g in &bim.grids {
                    segments.push(([g.start.0, g.start.1], [g.end.0, g.end.1]));
                }
                for c in &bim.columns {
                    endpoints.push([c.pos.0, c.pos.1]);
                }
            }

            let mut best: Option<([f64; 2], OsnapKind, f64)> = None;
            let consider = |best: &mut Option<([f64; 2], OsnapKind, f64)>,
                            pt: [f64; 2],
                            kind: OsnapKind,
                            score_bias: f64| {
                let d = (pt[0] - raw_mm[0]).hypot(pt[1] - raw_mm[1]);
                if d <= tol_mm {
                    let ranked = d * score_bias;
                    if best.is_none_or(|(_, _, bd)| ranked < bd) {
                        *best = Some((pt, kind, ranked));
                    }
                }
            };

            // 1. Endpoint snaps (highest priority right on vertex)
            for &ep in &endpoints {
                consider(&mut best, ep, OsnapKind::Endpoint, 0.90);
            }

            // 2. Segment intersection snaps (cap pairwise checks for responsiveness)
            let seg_cap = segments.len().min(64);
            for i in 0..seg_cap {
                for j in (i + 1)..seg_cap {
                    if let Some(ix) = segment_intersection_2d(segments[i].0, segments[i].1, segments[j].0, segments[j].1) {
                        consider(&mut best, ix, OsnapKind::Intersection, 0.88);
                    }
                }
            }

            // 3. Midpoint & true Perpendicular (from active anchor) snaps
            for &(a, b) in &segments {
                let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
                consider(&mut best, mid, OsnapKind::Midpoint, 0.98);

                if let Some(perp_ref) = anchor_mm {
                    let vx = b[0] - a[0];
                    let vy = b[1] - a[1];
                    let len_sq = vx * vx + vy * vy;
                    if len_sq > 1e-6 {
                        let t = ((perp_ref[0] - a[0]) * vx + (perp_ref[1] - a[1]) * vy) / len_sq;
                        if (0.02..=0.98).contains(&t) {
                            let foot = [a[0] + t * vx, a[1] + t * vy];
                            consider(&mut best, foot, OsnapKind::Perpendicular, 1.04);
                        }
                    }
                }
            }

            if let Some((pt, kind, _)) = best {
                return (pt, kind);
            }
        }

        if !self.snap_to_grid || self.grid_spacing_mm <= 1.0 {
            return (raw_mm, OsnapKind::None);
        }
        let step = self.grid_spacing_mm;
        (
            [
                (raw_mm[0] / step).round() * step,
                (raw_mm[1] / step).round() * step,
            ],
            OsnapKind::Grid,
        )
    }

    fn apply_globe_zoom_at_screen(&mut self, rect: Rect, raw_dz: f32, focus_pos: Option<Pos2>) {
        let z_curr = self.world_map.zoom_level;
        // Gently taper scroll/pinch increment when zoomed close to city/site scale
        let zoom_taper = 1.0 - ((z_curr - 8.0) / 22.0).clamp(0.0, 0.42);
        let dz = (raw_dz * zoom_taper).clamp(-0.28, 0.28);
        if dz.abs() <= 1e-5 {
            return;
        }

        self.earth_world.camera.fly_to = None;
        self.earth_world.camera.angular_vel_lat = 0.0;
        self.earth_world.camera.angular_vel_lon = 0.0;

        let pre_hit = focus_pos.map(|pos| {
            let pre_xf = self
                .last_globe_xform
                .as_ref()
                .filter(|x| x.rect == rect)
                .cloned()
                .unwrap_or_else(|| {
                    GlobeScreenTransform::from_view(
                        rect,
                        self.world_map.center_lat_lon.0,
                        self.world_map.center_lat_lon.1,
                        self.world_map.zoom_level,
                        &self.earth_world.camera,
                    )
                });
            (pos, pre_xf.screen_to_lat_lon(pos))
        });

        let old_center = self.world_map.center_lat_lon;
        self.world_map.zoom_level = (self.world_map.zoom_level + dz).clamp(1.0, 20.5);

        if let Some((pos, (lat_before, lon_before, hit_before))) = pre_hit {
            if hit_before {
                let post_xf = GlobeScreenTransform::from_view(
                    rect,
                    old_center.0,
                    old_center.1,
                    self.world_map.zoom_level,
                    &self.earth_world.camera,
                );
                let (lat_after, lon_after, hit_after) = post_xf.screen_to_lat_lon(pos);
                if hit_after {
                    let max_step_deg = (post_xf.d_surf * 65.0).clamp(1e-4, 18.0);
                    let dlat = (lat_before - lat_after).clamp(-max_step_deg, max_step_deg);
                    let mut dlon = lon_before - lon_after;
                    if dlon > 180.0 {
                        dlon -= 360.0;
                    } else if dlon < -180.0 {
                        dlon += 360.0;
                    }
                    let dlon = dlon.clamp(-max_step_deg * 1.4, max_step_deg * 1.4);
                    self.world_map
                        .set_center((old_center.0 + dlat).clamp(-85.0, 85.0), old_center.1 + dlon);
                }
            }
        }

        self.last_globe_xform = Some(GlobeScreenTransform::from_view(
            rect,
            self.world_map.center_lat_lon.0,
            self.world_map.center_lat_lon.1,
            self.world_map.zoom_level,
            &self.earth_world.camera,
        ));
    }

    fn orbit_globe_by_pixels(&mut self, rect: Rect, delta: Vec2, alt_held: bool, dt: f32) {
        self.earth_world.camera.fly_to = None;
        if alt_held {
            self.earth_world.camera.tilt_deg =
                (self.earth_world.camera.tilt_deg - delta.y * 0.30).clamp(0.0, 65.0);
            self.earth_world.camera.heading_deg =
                (self.earth_world.camera.heading_deg + delta.x * 0.38).rem_euclid(360.0);
            self.earth_world.camera.angular_vel_lat = 0.0;
            self.earth_world.camera.angular_vel_lon = 0.0;
            return;
        }
        let xform = self
            .last_globe_xform
            .as_ref()
            .filter(|x| x.rect == rect)
            .cloned()
            .unwrap_or_else(|| {
                GlobeScreenTransform::from_view(
                    rect,
                    self.world_map.center_lat_lon.0,
                    self.world_map.center_lat_lon.1,
                    self.world_map.zoom_level,
                    &self.earth_world.camera,
                )
            });
        let dpp_x = xform.deg_per_pixel();
        let dpp_y = xform.deg_per_pixel_y();
        let cos_lat = self
            .world_map
            .center_lat_lon
            .0
            .to_radians()
            .cos()
            .abs()
            .max(0.25);
        let hr = self.earth_world.camera.heading_deg.to_radians() as f64;
        let dx_h = (delta.x as f64) * dpp_x;
        let dy_h = -(delta.y as f64) * dpp_y;
        let d_east = dx_h * hr.cos() + dy_h * hr.sin();
        let d_north = -dx_h * hr.sin() + dy_h * hr.cos();
        let dlon = -d_east / cos_lat;
        let dlat = -d_north;
        let nlat = (self.world_map.center_lat_lon.0 + dlat).clamp(-85.0, 85.0);
        let nlon = self.world_map.center_lat_lon.1 + dlon;
        self.world_map.set_center(nlat, nlon);
        self.last_globe_xform = Some(GlobeScreenTransform::from_view(
            rect,
            self.world_map.center_lat_lon.0,
            self.world_map.center_lat_lon.1,
            self.world_map.zoom_level,
            &self.earth_world.camera,
        ));
        if self.world_map.zoom_level < 11.0 {
            let inv_dt = 1.0 / dt.max(0.008);
            self.earth_world.camera.angular_vel_lat =
                (dlat as f32 * inv_dt * 0.28).clamp(-35.0, 35.0);
            self.earth_world.camera.angular_vel_lon =
                (dlon as f32 * inv_dt * 0.28).clamp(-45.0, 45.0);
        } else {
            self.earth_world.camera.angular_vel_lat = 0.0;
            self.earth_world.camera.angular_vel_lon = 0.0;
        }
    }

    fn handle_canvas_interaction(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        rect: Rect,
        active_pillar: NavigationPillar,
        bim_project: &BimProject,
        pointer_in_inspector: bool,
    ) {
        if self.is_frozen || pointer_in_inspector {
            self.pending_start_mm = None;
            self.pending_earth_arc_start = None;
            self.marquee_anchor_mm = None;
            self.marquee_current_mm = None;
            self.active_freehand_mm.clear();
            return;
        }

        // Scroll Wheel & Pinch Zoom around cursor on 3D Globe or 2D World Map
        if response.hovered() {
            let (scroll_y, pinch_factor) =
                ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let pinch_dz = if (pinch_factor - 1.0).abs() > 0.002 {
                pinch_factor.log2()
            } else {
                0.0
            };
            let wheel_dz = if scroll_y.abs() > 0.2 {
                scroll_y * 0.0022
            } else {
                0.0
            };
            let raw_dz = wheel_dz + pinch_dz;
            if raw_dz.abs() > 1e-4 {
                if self.globe_mode {
                    self.apply_globe_zoom_at_screen(rect, raw_dz, response.hover_pos());
                } else {
                    let zoom_taper =
                        1.0 - ((self.world_map.zoom_level - 8.0) / 22.0).clamp(0.0, 0.42);
                    let dz = (raw_dz * zoom_taper).clamp(-0.28, 0.28);
                    self.world_map
                        .zoom_at_screen_pos(dz, response.hover_pos(), rect);
                }
            }
        }

        let scale = self.world_map.pixels_per_mm_at_origin(self.ground_origin_lat_lon);
        let hit_radius_mm = (18.0 / scale.max(1e-9)) as f64;
        self.last_osnap_radius_mm = hit_radius_mm.clamp(250.0, 2500.0);

        if let Some(hover) = response.hover_pos() {
            let (lat, lon, _) = self.screen_to_lat_lon_with_hit(hover, rect);
            self.cursor_lat_lon = (lat, lon);
            let raw_mm = self.screen_to_world(rect, hover);
            let (snapped, kind) = self.snap_point_with_osnap(
                raw_mm,
                self.pending_start_mm,
                Some(bim_project),
                self.last_osnap_radius_mm,
            );
            self.cursor_world_mm = snapped;
            self.last_osnap_kind = kind;
        }

        let (shift_held, alt_held, dt) =
            ui.input(|i| (i.modifiers.shift, i.modifiers.alt, i.unstable_dt));

        // Right-drag, Middle-drag, Shift-drag, or Alt-drag orbits/tilts 3D Globe or pans 2D map in any tool
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged() && (shift_held || alt_held))
        {
            let delta = response.drag_delta();
            if self.globe_mode {
                self.orbit_globe_by_pixels(rect, delta, alt_held, dt);
            } else {
                self.world_map.pan_by_pixels(delta.x, delta.y);
            }
            return;
        }

        if self.measure_active && response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                if hit {
                    self.measure_points_lat_lon.push((lat, lon));
                }
            }
            return;
        }

        // Phase 43: Earth Terraforming Brushes & Planetary Megaproject Tools
        if let Some(brush) = self.active_tool.earth_brush_action() {
            let stroke_start = response.drag_started_by(egui::PointerButton::Primary)
                || response.clicked_by(egui::PointerButton::Primary);
            let dragging = response.dragged_by(egui::PointerButton::Primary);
            if stroke_start || dragging {
                if let Some(pos) = response
                    .hover_pos()
                    .or_else(|| response.interact_pointer_pos())
                {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        let count = self.earth_world.apply_brush(lat, lon, brush, stroke_start);
                        if count > 0 || stroke_start {
                            let new_el = self.earth_world.effective_elevation_m(lat, lon);
                            self.status_message = Some(format!(
                                "{} @ {:.2}°, {:.2}° (Elev {:+.0}m • CO2 {:.0} ppm • Hab {:.0}/100)",
                                brush.label(),
                                lat,
                                lon,
                                new_el,
                                self.earth_world.vitals.co2_ppm,
                                self.earth_world.vitals.habitability_score,
                            ));
                        }
                    }
                }
            }
            return;
        }

        if let Some(kind) = self.active_tool.earth_megastructure_kind() {
            use crate::core::gis::MegastructureKind;
            if kind == MegastructureKind::ArcologyMegaTower {
                if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                        if hit {
                            let id = self.earth_world.add_megastructure(kind, (lat, lon), (lat, lon));
                            self.status_message = Some(format!(
                                "Constructed Mega-Tower #{id} at {:.2}°, {:.2}° (Habitability {:.0}/100)",
                                lat, lon, self.earth_world.vitals.habitability_score
                            ));
                        }
                    }
                }
                return;
            }
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pos) = response
                    .hover_pos()
                    .or_else(|| response.interact_pointer_pos())
                {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        self.pending_earth_arc_start = Some((lat, lon));
                    }
                }
            } else if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let Some(start_ll) = self.pending_earth_arc_start.take() {
                    if let Some(pos) = response
                        .hover_pos()
                        .or_else(|| response.interact_pointer_pos())
                    {
                        let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                        if hit {
                            let id = self.earth_world.add_megastructure(kind, start_ll, (lat, lon));
                            if let Some(m) = self.earth_world.megastructures.last() {
                                self.status_message = Some(format!(
                                    "Built {} (#{id}): {:.0} km (${:.1}B, {:.1} Mt/yr CO2)",
                                    m.name, m.length_km, m.cost_b_usd, m.carbon_delta_mt_yr
                                ));
                            }
                        }
                    }
                }
            } else if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        if let Some(start_ll) = self.pending_earth_arc_start.take() {
                            let id = self.earth_world.add_megastructure(kind, start_ll, (lat, lon));
                            if let Some(m) = self.earth_world.megastructures.last() {
                                self.status_message = Some(format!(
                                    "Built {} (#{id}): {:.0} km",
                                    m.name, m.length_km
                                ));
                            }
                        } else {
                            self.pending_earth_arc_start = Some((lat, lon));
                            self.status_message = Some(format!(
                                "{} start anchored ({:.2}°, {:.2}°) — click or drag destination",
                                kind.label(),
                                lat,
                                lon
                            ));
                        }
                    }
                }
            }
            return;
        }

        let world_map_snapshot = self.world_map.clone();
        let origin_lat_lon = self.ground_origin_lat_lon;
        let globe_xform_snapshot = if self.globe_mode {
            Some(
                self.last_globe_xform
                    .as_ref()
                    .filter(|x| x.rect == rect)
                    .cloned()
                    .unwrap_or_else(|| {
                        GlobeScreenTransform::from_view(
                            rect,
                            world_map_snapshot.center_lat_lon.0,
                            world_map_snapshot.center_lat_lon.1,
                            world_map_snapshot.zoom_level,
                            &self.earth_world.camera,
                        )
                    }),
            )
        } else {
            None
        };
        let to_world = |pos: Pos2| -> [f64; 2] {
            if let Some(ref xf) = globe_xform_snapshot {
                let (lat, lon, _) = xf.screen_to_lat_lon(pos);
                SlippyMapState::lat_lon_to_world_mm(origin_lat_lon, lat, lon)
            } else {
                world_map_snapshot.screen_to_world_mm(origin_lat_lon, pos, rect)
            }
        };

        // Phase 57 & 59: Interactive "Place on Earth" Ghost Footprint mode + Vector Edge/Parcel OSNAP
        if self.globe_mode && self.placing_unlocated_rel_path.is_some() {
            let resolve_place_coord = |raw_lat: f64, raw_lon: f64, skip_snap: bool| -> ((f64, f64), Option<f64>) {
                if !skip_snap {
                    if let Some((sla, slo, az)) = self
                        .urban_vectors
                        .snap_lat_lon_to_vector_edge((raw_lat, raw_lon), 18.0)
                    {
                        return ((sla, slo), Some(az));
                    }
                }
                ((raw_lat, raw_lon), None)
            };
            if let Some(hover) = response.hover_pos() {
                let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(hover, rect);
                if hit {
                    let ((pla, plo), az_opt) = resolve_place_coord(lat, lon, alt_held);
                    self.placing_hover_lat_lon = Some((pla, plo));
                    self.active_vector_snap_az_deg = az_opt;
                    self.cursor_lat_lon = (pla, plo);
                } else {
                    self.placing_hover_lat_lon = None;
                    self.active_vector_snap_az_deg = None;
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Q)) {
                self.placing_heading_deg = (self.placing_heading_deg - 15.0).rem_euclid(360.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::E)) {
                self.placing_heading_deg = (self.placing_heading_deg + 15.0).rem_euclid(360.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::A)) {
                if let Some(az) = self.active_vector_snap_az_deg {
                    self.placing_heading_deg = az.rem_euclid(360.0);
                }
            }
            if response.secondary_clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.placing_unlocated_rel_path = None;
                self.placing_hover_lat_lon = None;
                self.active_vector_snap_az_deg = None;
                return;
            }
            if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        let ((pla, plo), _) = resolve_place_coord(lat, lon, alt_held);
                        if let Some(rel) = self.placing_unlocated_rel_path.take() {
                            let hdg = self.placing_heading_deg;
                            self.world_index.pin_building_at(&rel, pla, plo, hdg);
                            if self.active_world_project_rel_path.as_deref() == Some(rel.as_str()) {
                                self.ground_origin_lat_lon = (pla, plo);
                                self.ground_origin_anchored = true;
                            }
                            self.pending_world_pin_event = Some((rel.clone(), pla, plo, hdg));
                            self.status_message = Some(format!(
                                "Pinned building '{}' onto Earth @ ({:.4}°, {:.4}°), heading {:.0}°",
                                rel.split('/').next_back().unwrap_or(&rel),
                                pla,
                                plo,
                                hdg
                            ));
                        }
                        self.placing_hover_lat_lon = None;
                        self.active_vector_snap_az_deg = None;
                    }
                }
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                self.orbit_globe_by_pixels(rect, delta, alt_held, dt);
            }
            return;
        }

        // Phase 55: Direct 3D BIM Placement (Column / Beam / Wall / Footing) straight onto Earth 3D surface
        if self.globe_mode && self.earth_bim_author.active_tool != EarthBimTool::None {
            let bim_snap = |uc: &Self, raw: [f64; 2], anchor_start: Option<[f64; 2]>| -> [f64; 2] {
                let (mut pt, _) = uc.snap_point_with_osnap(
                    raw,
                    anchor_start,
                    Some(bim_project),
                    uc.last_osnap_radius_mm.max(450.0),
                );
                if let Some(s) = anchor_start {
                    if uc.ortho_mode {
                        let dx = (pt[0] - s[0]).abs();
                        let dy = (pt[1] - s[1]).abs();
                        if dx >= dy {
                            pt[1] = s[1];
                        } else {
                            pt[0] = s[0];
                        }
                    }
                }
                pt
            };

            if let Some(hover) = response.hover_pos() {
                let snapped = bim_snap(self, to_world(hover), self.earth_bim_author.pending_start_mm);
                self.earth_bim_author.hover_snap_mm = Some(snapped);
                self.cursor_world_mm = snapped;
            } else {
                self.earth_bim_author.hover_snap_mm = None;
            }

            if response.secondary_clicked() {
                self.earth_bim_author.pending_start_mm = None;
            }

            if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let was_anchored = self.ground_origin_anchored;
                    self.ensure_ground_origin_at_screen(pos, rect);
                    let active_origin = self.ground_origin_lat_lon;
                    let raw = if !was_anchored {
                        if let Some(ref xf) = globe_xform_snapshot {
                            let (lat, lon, _) = xf.screen_to_lat_lon(pos);
                            SlippyMapState::lat_lon_to_world_mm(active_origin, lat, lon)
                        } else {
                            world_map_snapshot.screen_to_world_mm(active_origin, pos, rect)
                        }
                    } else {
                        to_world(pos)
                    };
                    let pt = bim_snap(self, raw, self.earth_bim_author.pending_start_mm);
                    let base_level_id = self.earth_bim_author.base_level_id.max(1);
                    let top_level_id = self.earth_bim_author.top_level_id.max(base_level_id);
                    let p2 = crate::core::P2(pt[0], pt[1]);

                    match self.earth_bim_author.active_tool {
                        EarthBimTool::Column => {
                            let w = self.earth_bim_author.col_width_mm.clamp(150.0, 2000.0);
                            let d = self.earth_bim_author.col_depth_mm.clamp(150.0, 2000.0);
                            self.pending_earth_bim_commands.push(crate::core::BimCommand::AddColumn {
                                base_level_id,
                                top_level_id,
                                pos: p2,
                                width_mm: w,
                                depth_mm: d,
                                is_round: false,
                                profile_name: format!("C{:.0}x{:.0}", w, d),
                            });
                            self.status_message = Some(format!(
                                "Placed Column ({w:.0}x{d:.0}mm) on Earth 3D @ ({:.2}m, {:.2}m)",
                                pt[0] * 0.001,
                                pt[1] * 0.001
                            ));
                        }
                        EarthBimTool::Footing => {
                            let fs = self.earth_bim_author.footing_size_mm.clamp(400.0, 6000.0);
                            self.pending_earth_bim_commands.push(crate::core::BimCommand::AddFooting {
                                level_id: base_level_id,
                                pos: p2,
                                width_mm: fs,
                                length_mm: fs,
                                thickness_mm: 450.0,
                            });
                            self.status_message = Some(format!(
                                "Placed Footing ({fs:.0}mm) on Earth 3D @ ({:.2}m, {:.2}m)",
                                pt[0] * 0.001,
                                pt[1] * 0.001
                            ));
                        }
                        EarthBimTool::Wall | EarthBimTool::Beam => {
                            if let Some(s) = self.earth_bim_author.pending_start_mm.take() {
                                if (s[0] - pt[0]).hypot(s[1] - pt[1]) >= 200.0 {
                                    if self.earth_bim_author.active_tool == EarthBimTool::Wall {
                                        let wt = self.earth_bim_author.wall_thickness_mm.clamp(80.0, 1200.0);
                                        self.pending_earth_bim_commands.push(crate::core::BimCommand::AddWall {
                                            base_level_id,
                                            start: crate::core::P2(s[0], s[1]),
                                            end: p2,
                                            thickness_mm: wt,
                                            is_structural: true,
                                        });
                                        self.status_message = Some(format!(
                                            "Placed Wall ({wt:.0}mm) on Earth 3D (L={:.2}m)",
                                            (s[0] - pt[0]).hypot(s[1] - pt[1]) * 0.001
                                        ));
                                    } else {
                                        let bw = self.earth_bim_author.beam_width_mm.clamp(120.0, 1500.0);
                                        let bd = self.earth_bim_author.beam_depth_mm.clamp(150.0, 2500.0);
                                        self.pending_earth_bim_commands.push(crate::core::BimCommand::AddBeamRun {
                                            level_id: top_level_id,
                                            category: crate::core::RunCategory::StructuralBeam,
                                            start: crate::core::P2(s[0], s[1]),
                                            end: p2,
                                            width_mm: bw,
                                            depth_mm: bd,
                                            justify: crate::core::RunJustify::Top,
                                            profile_name: format!("B{:.0}x{:.0}", bw, bd),
                                        });
                                        self.status_message = Some(format!(
                                            "Placed Beam ({bw:.0}x{bd:.0}mm) on Earth 3D (L={:.2}m)",
                                            (s[0] - pt[0]).hypot(s[1] - pt[1]) * 0.001
                                        ));
                                    }
                                }
                            } else {
                                self.earth_bim_author.pending_start_mm = Some(pt);
                            }
                        }
                        EarthBimTool::Slab => {
                            if let Some(s) = self.earth_bim_author.pending_start_mm.take() {
                                let min_x = s[0].min(pt[0]);
                                let max_x = s[0].max(pt[0]);
                                let min_y = s[1].min(pt[1]);
                                let max_y = s[1].max(pt[1]);
                                if (max_x - min_x) >= 300.0 && (max_y - min_y) >= 300.0 {
                                    self.pending_earth_bim_commands.push(crate::core::BimCommand::AddSlab {
                                        level_id: base_level_id,
                                        boundary: vec![
                                            crate::core::P2(min_x, min_y),
                                            crate::core::P2(max_x, min_y),
                                            crate::core::P2(max_x, max_y),
                                            crate::core::P2(min_x, max_y),
                                        ],
                                        openings: Vec::new(),
                                        thickness_mm: 150.0,
                                        is_roof: false,
                                        roof_slope_deg: 0.0,
                                    });
                                    self.status_message = Some(format!(
                                        "Placed Slab ({:.2} x {:.2} m) on Earth 3D Level #{}",
                                        (max_x - min_x) * 0.001,
                                        (max_y - min_y) * 0.001,
                                        base_level_id
                                    ));
                                }
                            } else {
                                self.earth_bim_author.pending_start_mm = Some(pt);
                            }
                        }
                        EarthBimTool::Door | EarthBimTool::Window => {
                            let is_door = self.earth_bim_author.active_tool == EarthBimTool::Door;
                            let target_wall_id = self
                                .pick_bim_element_at_screen(pos, rect, bim_project)
                                .and_then(|(k, id)| {
                                    if k == crate::core::BimElementKind::Wall {
                                        Some(id)
                                    } else {
                                        None
                                    }
                                })
                                .or_else(|| {
                                    // Proximity fallback to closest wall segment in XY world mm
                                    bim_project
                                        .walls
                                        .iter()
                                        .filter_map(|w| {
                                            let ab_x = w.end.0 - w.start.0;
                                            let ab_y = w.end.1 - w.start.1;
                                            let l2 = ab_x * ab_x + ab_y * ab_y;
                                            if l2 < 1.0 {
                                                return None;
                                            }
                                            let t = (((raw[0] - w.start.0) * ab_x + (raw[1] - w.start.1) * ab_y) / l2).clamp(0.0, 1.0);
                                            let d = ((raw[0] - (w.start.0 + t * ab_x)).powi(2)
                                                + (raw[1] - (w.start.1 + t * ab_y)).powi(2))
                                            .sqrt();
                                            if d <= 1200.0 { Some((w.id, d)) } else { None }
                                        })
                                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                                        .map(|(id, _)| id)
                                });
                            if let Some(wid) = target_wall_id {
                                if let Some(wall) = bim_project.walls.iter().find(|w| w.id == wid) {
                                    let wlen = wall.start.dist(wall.end).max(400.0);
                                    let ux = (wall.end.0 - wall.start.0) / wlen;
                                    let uy = (wall.end.1 - wall.start.1) / wlen;
                                    let proj_along = (raw[0] - wall.start.0) * ux + (raw[1] - wall.start.1) * uy;
                                    let (kind, width_mm, height_mm, sill_height_mm) = if is_door {
                                        (crate::core::bim::OpeningKind::SingleDoor, 900.0, 2100.0, 0.0)
                                    } else {
                                        (crate::core::bim::OpeningKind::CasementWindow, 1800.0, 1400.0, 900.0)
                                    };
                                    let offset_along_mm = (proj_along - width_mm * 0.5)
                                        .clamp(120.0, (wlen - width_mm - 120.0).max(120.0));
                                    self.pending_earth_bim_commands.push(crate::core::BimCommand::AddOpening {
                                        wall_id: wid,
                                        kind,
                                        offset_along_mm,
                                        width_mm,
                                        height_mm,
                                        sill_height_mm,
                                    });
                                    self.status_message = Some(format!(
                                        "Added {} to Wall #{} on Earth 3D",
                                        if is_door { "Door" } else { "Window" },
                                        wid
                                    ));
                                }
                            } else {
                                self.status_message = Some(format!(
                                    "Click near a 3D Wall on Earth 3D to insert a {}.",
                                    if is_door { "Door" } else { "Window" }
                                ));
                            }
                        }
                        EarthBimTool::None => {}
                    }
                }
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                self.orbit_globe_by_pixels(rect, delta, alt_held, dt);
            }
            return;
        } else {
            self.earth_bim_author.hover_snap_mm = None;
        }

        if self.active_tool == UniversalCanvasTool::SelectPan {
            if self.globe_mode
                && self.world_map.zoom_level >= 8.5
                && !response.dragged()
            {
                if let Some(hover) = response.hover_pos() {
                    self.hovered_world_building_rel_path =
                        match self.pick_world_building_or_cluster_at_screen(hover, rect) {
                            Some(WorldPickOutcome::HydrateBuilding(rel)) => Some(rel),
                            _ => None,
                        };
                } else {
                    self.hovered_world_building_rel_path = None;
                }
            } else {
                self.hovered_world_building_rel_path = None;
            }
            if self.globe_mode
                && !ui.ctx().egui_wants_keyboard_input()
                && ui.input(|i| i.key_pressed(egui::Key::R))
            {
                if let Some(ref active_rel) = self.active_world_project_rel_path.clone() {
                    let step = if shift_held { -15.0 } else { 15.0 };
                    if let Some(b) = self.world_index.get(active_rel).cloned() {
                        let next_hdg = (b.heading_deg + step).rem_euclid(360.0);
                        self.world_index.update_heading(active_rel, next_hdg);
                        self.pending_world_pin_event = Some((active_rel.clone(), b.lat, b.lon, next_hdg));
                        self.status_message = Some(format!(
                            "Rotated '{}' True North heading -> {:.0}°",
                            b.name, next_hdg
                        ));
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                if self.globe_mode {
                    self.orbit_globe_by_pixels(rect, delta, alt_held, dt);
                } else {
                    self.world_map.pan_by_pixels(delta.x, delta.y);
                }
            } else if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some((kind, id)) = self.pick_bim_element_at_screen(pos, rect, bim_project) {
                        self.last_picked_bim_element = Some((kind, id));
                        self.highlighted_bim_element = Some((kind, id));
                        self.last_picked_earth_lat_lon = None;
                        self.set_single_selection(None);
                        self.spatial_pick_seq = self.spatial_pick_seq.wrapping_add(1);
                    } else {
                        let w = to_world(pos);
                        let picked = self
                            .find_nearest_entity_idx(w, hit_radius_mm)
                            .and_then(|i| self.entities.get(i).map(|e| e.id));
                        self.set_single_selection(picked);
                        self.last_picked_bim_element = None;
                        self.highlighted_bim_element = None;
                        if picked.is_none() {
                            if let Some(outcome) = self.pick_world_building_or_cluster_at_screen(pos, rect) {
                                match outcome {
                                    WorldPickOutcome::FlyToCluster { lat, lon, label } => {
                                        self.earth_world.start_fly_to(
                                            &self.world_map,
                                            lat,
                                            lon,
                                            13.5,
                                            &label,
                                        );
                                        self.last_picked_earth_lat_lon = Some((lat, lon));
                                    }
                                    WorldPickOutcome::HydrateBuilding(rel) => {
                                        self.clicked_world_building_rel_path = Some(rel);
                                        self.last_picked_earth_lat_lon = None;
                                    }
                                }
                            } else {
                                let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                                self.last_picked_earth_lat_lon = if hit { Some((lat, lon)) } else { None };
                            }
                        } else {
                            self.last_picked_earth_lat_lon = None;
                        }
                        self.spatial_pick_seq = self.spatial_pick_seq.wrapping_add(1);
                    }
                }
            }
            return;
        }

        // Auto-anchor ground origin on first drawing stroke anywhere on Earth
        if self.active_tool != UniversalCanvasTool::Eraser
            && (response.drag_started_by(egui::PointerButton::Primary)
                || response.clicked_by(egui::PointerButton::Primary))
        {
            if let Some(pos) = response.interact_pointer_pos() {
                self.ensure_ground_origin_at_screen(pos, rect);
            }
        }

        self.handle_world_sketch_interaction(response, active_pillar, hit_radius_mm, &to_world);
    }

    /// Renders interactive WGS84 geodesic polyline/parcel area measurement overlay across 2D & 3D World canvases.
    pub fn render_geodetic_measure_overlay(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        lat_lon_to_screen: &impl Fn(f64, f64) -> Pos2,
    ) {
        if self.measure_points_lat_lon.is_empty() && !self.measure_active {
            return;
        }
        let clip = painter.with_clip_rect(rect);
        let pts: Vec<Pos2> = self
            .measure_points_lat_lon
            .iter()
            .map(|&(lat, lon)| lat_lon_to_screen(lat, lon))
            .collect();

        if self.measure_area_mode && pts.len() >= 3 {
            clip.add(egui::Shape::convex_polygon(
                pts.clone(),
                theme.accent_orange.gamma_multiply(0.16),
                Stroke::new(2.0, theme.accent_orange),
            ));
        } else if pts.len() >= 2 {
            for win in pts.windows(2) {
                clip.line_segment([win[0], win[1]], Stroke::new(2.4, theme.accent_orange));
            }
        }

        for (idx, p) in pts.iter().enumerate() {
            clip.circle_filled(*p, 4.5, theme.accent_orange);
            clip.circle_stroke(*p, 4.5, Stroke::new(1.5, Color32::WHITE));
            if idx > 0 {
                let seg_m = crate::core::gis::SlippyMapState::geodetic_distance_m(
                    self.measure_points_lat_lon[idx - 1],
                    self.measure_points_lat_lon[idx],
                );
                let mid = Pos2::new(0.5 * (pts[idx - 1].x + p.x), 0.5 * (pts[idx - 1].y + p.y));
                let lbl = if seg_m >= 1000.0 {
                    format!("{:.2} km", seg_m / 1000.0)
                } else {
                    format!("{:.1} m", seg_m)
                };
                clip.text(
                    mid + Vec2::new(0.0, -8.0),
                    Align2::CENTER_BOTTOM,
                    lbl,
                    egui::FontId::monospace(10.0),
                    theme.accent_orange,
                );
            }
        }

        let total_m = crate::core::gis::SlippyMapState::geodetic_polyline_length_m(&self.measure_points_lat_lon);
        let summary = if self.measure_points_lat_lon.is_empty() {
            "Geodesic Measure Active: Click points on Earth ground".to_string()
        } else if self.measure_area_mode && self.measure_points_lat_lon.len() >= 3 {
            let area_m2 = crate::core::gis::SlippyMapState::geodetic_polygon_area_m2(
                &self.measure_points_lat_lon,
            );
            format!(
                "Parcel Area: {:.1} m² ({:.3} ha) • Perimeter {:.1} m ({} pts)",
                area_m2,
                area_m2 / 10_000.0,
                total_m,
                self.measure_points_lat_lon.len()
            )
        } else if total_m >= 1000.0 {
            format!(
                "Geodesic Distance: {:.3} km ({:.0} m) • {} pts",
                total_m / 1000.0,
                total_m,
                self.measure_points_lat_lon.len()
            )
        } else {
            format!(
                "Geodesic Distance: {:.2} m • {} pts",
                total_m,
                self.measure_points_lat_lon.len()
            )
        };

        let chip_pos = Pos2::new(rect.left() + 14.0, rect.bottom() - 26.0);
        clip.rect_filled(
            Rect::from_min_size(chip_pos, Vec2::new((summary.len() as f32 * 6.4 + 20.0).min(rect.width() - 28.0), 22.0)),
            CornerRadius::same(6),
            theme.bg_card.gamma_multiply(0.90),
        );
        clip.text(
            chip_pos + Vec2::new(8.0, 11.0),
            Align2::LEFT_CENTER,
            summary,
            egui::FontId::monospace(10.5),
            theme.accent_orange,
        );
    }

    /// Interactive drawing & erasing handler for synchronized Universal Canvas shapes using any 2D/3D ground projection.
    pub fn handle_world_sketch_interaction(
        &mut self,
        response: &egui::Response,
        active_pillar: NavigationPillar,
        hit_radius_mm: f64,
        screen_to_world: &impl Fn(Pos2) -> [f64; 2],
    ) {
        self.last_osnap_radius_mm = hit_radius_mm.clamp(250.0, 2500.0);
        if let Some(hover) = response.hover_pos() {
            let (snapped, kind) = self.snap_point_with_osnap(
                screen_to_world(hover),
                self.pending_start_mm,
                None,
                self.last_osnap_radius_mm,
            );
            self.cursor_world_mm = snapped;
            self.last_osnap_kind = kind;
        }

        let badge = active_pillar.badge().to_string();
        let pname = active_pillar.clean_name().to_string();
        let color_rgb = self.active_color_rgb();
        let stroke_width = self.stroke_width;

        match self.active_tool {
            UniversalCanvasTool::SelectPan => {
                if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = screen_to_world(pos);
                        let picked = self
                            .find_nearest_entity_idx(w, hit_radius_mm)
                            .and_then(|i| self.entities.get(i).map(|e| e.id));
                        self.set_single_selection(picked);
                    }
                }
            }
            UniversalCanvasTool::BoxSelect => {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = screen_to_world(pos);
                        self.marquee_anchor_mm = Some(w);
                        self.marquee_current_mm = Some(w);
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.marquee_current_mm = Some(screen_to_world(pos));
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    if let (Some(anchor), Some(pos)) =
                        (self.marquee_anchor_mm.take(), response.interact_pointer_pos())
                    {
                        let end_mm = screen_to_world(pos);
                        let count = self.select_marquee(anchor, end_mm);
                        let mode = if end_mm[0] >= anchor[0] { "Window" } else { "Crossing" };
                        self.status_message = Some(format!(
                            "{mode} marquee selected {count} entities"
                        ));
                    }
                    self.marquee_current_mm = None;
                } else if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = screen_to_world(pos);
                        let picked = self
                            .find_nearest_entity_idx(w, hit_radius_mm)
                            .and_then(|i| self.entities.get(i).map(|e| e.id));
                        self.set_single_selection(picked);
                    }
                }
            }
            UniversalCanvasTool::Freehand => {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.active_freehand_mm.clear();
                        self.active_freehand_mm.push(screen_to_world(pos));
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let pt = screen_to_world(pos);
                        let push_it = self
                            .active_freehand_mm
                            .last()
                            .is_none_or(|last| (last[0] - pt[0]).hypot(last[1] - pt[1]) >= 40.0);
                        if push_it {
                            self.active_freehand_mm.push(pt);
                        }
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary)
                    && self.active_freehand_mm.len() >= 2
                {
                    let pts = std::mem::take(&mut self.active_freehand_mm);
                    self.push_undo();
                    let id = self.next_entity_id;
                    self.next_entity_id += 1;
                    self.entities.push(CanvasEntity {
                        id,
                        pillar_badge: badge,
                        pillar_name: pname,
                        kind: CanvasShapeKind::Freehand { points_mm: pts },
                        color_rgb,
                        stroke_width,
                    });
                }
            }
            UniversalCanvasTool::Line
            | UniversalCanvasTool::Rectangle
            | UniversalCanvasTool::Circle
            | UniversalCanvasTool::Dimension => {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.pending_start_mm = Some(self.snap_point(screen_to_world(pos)));
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    if let (Some(start), Some(pos)) =
                        (self.pending_start_mm.take(), response.interact_pointer_pos())
                    {
                        let raw_end = self.snap_point(screen_to_world(pos));
                        let end = if matches!(
                            self.active_tool,
                            UniversalCanvasTool::Line | UniversalCanvasTool::Dimension
                        ) {
                            self.apply_ortho(start, raw_end)
                        } else {
                            raw_end
                        };
                        let dist = (end[0] - start[0]).hypot(end[1] - start[1]);
                        if dist >= 50.0 {
                            let kind = match self.active_tool {
                                UniversalCanvasTool::Line => CanvasShapeKind::Line {
                                    start_mm: start,
                                    end_mm: end,
                                },
                                UniversalCanvasTool::Dimension => CanvasShapeKind::Dimension {
                                    start_mm: start,
                                    end_mm: end,
                                },
                                UniversalCanvasTool::Rectangle => CanvasShapeKind::Rectangle {
                                    min_mm: [start[0].min(end[0]), start[1].min(end[1])],
                                    max_mm: [start[0].max(end[0]), start[1].max(end[1])],
                                },
                                UniversalCanvasTool::Circle => CanvasShapeKind::Circle {
                                    center_mm: start,
                                    radius_mm: dist,
                                },
                                _ => unreachable!(),
                            };
                            self.push_undo();
                            let id = self.next_entity_id;
                            self.next_entity_id += 1;
                            self.set_single_selection(Some(id));
                            self.entities.push(CanvasEntity {
                                id,
                                pillar_badge: badge,
                                pillar_name: pname,
                                kind,
                                color_rgb,
                                stroke_width,
                            });
                        }
                    } else {
                        self.pending_start_mm = None;
                    }
                }
            }
            UniversalCanvasTool::Callout => {
                if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = self.snap_point(screen_to_world(pos));
                        let note = if self.callout_draft.trim().is_empty() {
                            format!("{} Note", active_pillar.badge())
                        } else {
                            self.callout_draft.trim().to_string()
                        };
                        self.push_undo();
                        let id = self.next_entity_id;
                        self.next_entity_id += 1;
                        self.entities.push(CanvasEntity {
                            id,
                            pillar_badge: badge,
                            pillar_name: pname,
                            kind: CanvasShapeKind::Callout {
                                pos_mm: w,
                                text: note,
                            },
                            color_rgb,
                            stroke_width,
                        });
                    }
                }
            }
            UniversalCanvasTool::Eraser => {
                if response.clicked_by(egui::PointerButton::Primary)
                    || response.dragged_by(egui::PointerButton::Primary)
                {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = screen_to_world(pos);
                        if let Some(idx) = self.find_nearest_entity_idx(w, hit_radius_mm) {
                            self.push_undo();
                            self.entities.remove(idx);
                        }
                    }
                }
            }
            UniversalCanvasTool::EarthUplift
            | UniversalCanvasTool::EarthCarve
            | UniversalCanvasTool::EarthReclaim
            | UniversalCanvasTool::EarthForest
            | UniversalCanvasTool::EarthSolar
            | UniversalCanvasTool::EarthLake
            | UniversalCanvasTool::EarthSeawall
            | UniversalCanvasTool::EarthBridge
            | UniversalCanvasTool::EarthTower
            | UniversalCanvasTool::EarthCorridor => {}
        }
    }

    fn find_nearest_entity_idx(&self, pt_mm: [f64; 2], max_dist_mm: f64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (idx, ent) in self.entities.iter().enumerate() {
            let d = match &ent.kind {
                CanvasShapeKind::Line { start_mm, end_mm }
                | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    point_to_segment_dist(pt_mm, *start_mm, *end_mm)
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    let c0 = *min_mm;
                    let c1 = [max_mm[0], min_mm[1]];
                    let c2 = *max_mm;
                    let c3 = [min_mm[0], max_mm[1]];
                    point_to_segment_dist(pt_mm, c0, c1)
                        .min(point_to_segment_dist(pt_mm, c1, c2))
                        .min(point_to_segment_dist(pt_mm, c2, c3))
                        .min(point_to_segment_dist(pt_mm, c3, c0))
                }
                CanvasShapeKind::Circle {
                    center_mm,
                    radius_mm,
                } => ((pt_mm[0] - center_mm[0]).hypot(pt_mm[1] - center_mm[1]) - radius_mm).abs(),
                CanvasShapeKind::Freehand { points_mm } => points_mm
                    .windows(2)
                    .map(|w| point_to_segment_dist(pt_mm, w[0], w[1]))
                    .fold(f64::MAX, f64::min),
                CanvasShapeKind::Callout { pos_mm, .. } => {
                    (pt_mm[0] - pos_mm[0]).hypot(pt_mm[1] - pos_mm[1])
                }
            };
            if d <= max_dist_mm && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((idx, d));
            }
        }
        best.map(|(i, _)| i)
    }

    fn paint_grid(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        let step = self.grid_spacing_mm.max(250.0);
        let top_left_mm = self.screen_to_world(rect, rect.min);
        let bot_right_mm = self.screen_to_world(rect, rect.max);
        let mut min_x = (top_left_mm[0].min(bot_right_mm[0]) / step).floor() as i64 - 1;
        let mut max_x = (top_left_mm[0].max(bot_right_mm[0]) / step).ceil() as i64 + 1;
        let mut min_y = (top_left_mm[1].min(bot_right_mm[1]) / step).floor() as i64 - 1;
        let mut max_y = (top_left_mm[1].max(bot_right_mm[1]) / step).ceil() as i64 + 1;

        if self.globe_mode && ((max_x - min_x).abs() > 120 || (max_y - min_y).abs() > 120) {
            // When Earth 3D camera is pitched steeply toward horizon, draw a crisp local site grid around origin
            let half_cells = 30_i64;
            min_x = -half_cells;
            max_x = half_cells;
            min_y = -half_cells;
            max_y = half_cells;
        }

        let line_col = if theme.is_dark() {
            Color32::from_rgba_premultiplied(70, 90, 125, 28)
        } else {
            Color32::from_rgba_premultiplied(120, 140, 170, 42)
        };
        let major_col = if theme.is_dark() {
            Color32::from_rgba_premultiplied(90, 120, 165, 52)
        } else {
            Color32::from_rgba_premultiplied(100, 125, 165, 70)
        };

        // Limit line count on extreme zoom out
        if (max_x - min_x) <= 120 {
            for ix in min_x..=max_x {
                let x = ix as f64 * step;
                let p0 = to_screen([x, min_y as f64 * step]);
                let p1 = to_screen([x, max_y as f64 * step]);
                let col = if ix % 4 == 0 { major_col } else { line_col };
                painter.line_segment([p0, p1], Stroke::new(0.7, col));
            }
        }
        if (max_y - min_y) <= 120 {
            for iy in min_y..=max_y {
                let y = iy as f64 * step;
                let p0 = to_screen([min_x as f64 * step, y]);
                let p1 = to_screen([max_x as f64 * step, y]);
                let col = if iy % 4 == 0 { major_col } else { line_col };
                painter.line_segment([p0, p1], Stroke::new(0.7, col));
            }
        }

        // World Origin Axes
        let o = to_screen([0.0, 0.0]);
        if rect.expand(40.0).contains(o) {
            painter.line_segment(
                [o, o + Vec2::new(36.0, 0.0)],
                Stroke::new(1.8, theme.accent_red),
            );
            painter.line_segment(
                [o, o - Vec2::new(0.0, 36.0)],
                Stroke::new(1.8, theme.accent_green),
            );
            painter.circle_filled(o, 3.0, theme.accent_blue);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_bim_base_layer(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        bim_project: &BimProject,
        members: &[StructuralMember],
        active_pillar: NavigationPillar,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        let clip_painter = painter.with_clip_rect(rect);

        // 1. Structural Grid Axes & Bubbles
        for g in &bim_project.grids {
            let s0 = to_screen([g.start.0, g.start.1]);
            let s1 = to_screen([g.end.0, g.end.1]);
            clip_painter.line_segment(
                [s0, s1],
                Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.40)),
            );
            clip_painter.circle_filled(s1, 9.5, theme.bg_card);
            clip_painter.circle_stroke(s1, 9.5, Stroke::new(1.2, theme.accent_blue));
            clip_painter.text(
                s1,
                Align2::CENTER_CENTER,
                &g.label,
                egui::FontId::monospace(9.5),
                theme.accent_blue,
            );
        }

        // 2. Rooms Halftone Fill
        for rm in &bim_project.rooms {
            let pts: Vec<Pos2> = rm.outer_ring.iter().map(|p| to_screen([p.0, p.1])).collect();
            if pts.len() >= 3 {
                clip_painter.add(egui::Shape::convex_polygon(
                    pts,
                    theme.accent_green.gamma_multiply(0.08),
                    Stroke::NONE,
                ));
            }
            let c = to_screen([rm.seed_point.0, rm.seed_point.1]);
            clip_painter.text(
                c,
                Align2::CENTER_CENTER,
                format!("{} ({:.0} m²)", rm.number, rm.area_m2),
                egui::FontId::proportional(9.5),
                theme.text_muted,
            );
        }

        if !self.globe_mode {
            // 2B. Isolated Pad Footings (Flat 2D Projection)
            for ft in &bim_project.footings {
                let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Footing, ft.id));
                let ring: Vec<Pos2> = ft
                    .footprint_ring()
                    .into_iter()
                    .map(|p| to_screen([p.0, p.1]))
                    .collect();
                let fill = if is_sel {
                    theme.accent_blue.gamma_multiply(0.35)
                } else {
                    theme.accent_orange.gamma_multiply(0.14)
                };
                let stroke = if is_sel {
                    Stroke::new(2.2, Color32::from_rgb(56, 189, 248))
                } else {
                    Stroke::new(1.0, theme.accent_orange.gamma_multiply(0.45))
                };
                clip_painter.add(egui::Shape::convex_polygon(ring, fill, stroke));
            }

            // 3. Structural Beams / Runs (Flat 2D Projection)
            for run in &bim_project.runs {
                let s0 = to_screen([run.start.0, run.start.1]);
                let s1 = to_screen([run.end.0, run.end.1]);
                let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Run, run.id));
                let col = if is_sel {
                    Color32::from_rgb(56, 189, 248)
                } else if active_pillar == NavigationPillar::AnalysisSimulation {
                    let util = bim_project
                        .fem_id_for_bim_element(crate::core::BimElementKind::Run, run.id)
                        .and_then(|fid| members.iter().find(|m| m.id == fid))
                        .map(|m| m.utilization)
                        .unwrap_or(0.45);
                    if util >= 0.85 {
                        theme.accent_red
                    } else if util >= 0.65 {
                        theme.accent_orange
                    } else {
                        theme.accent_green
                    }
                } else {
                    theme.accent_green.gamma_multiply(0.65)
                };
                clip_painter.line_segment([s0, s1], Stroke::new(if is_sel { 4.2 } else { 1.8 }, col));
            }

            // 4. Compound Walls (Flat 2D Projection)
            for w in &bim_project.walls {
                let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Wall, w.id));
                let ring: Vec<Pos2> = w
                    .footprint_ring()
                    .into_iter()
                    .map(|p| to_screen([p.0, p.1]))
                    .collect();
                let fill = if is_sel {
                    Color32::from_rgb(56, 189, 248).gamma_multiply(0.65)
                } else if theme.is_dark() {
                    Color32::from_rgb(65, 80, 102)
                } else {
                    Color32::from_rgb(148, 163, 184)
                };
                clip_painter.add(egui::Shape::convex_polygon(
                    ring,
                    fill.gamma_multiply(0.75),
                    if is_sel {
                        Stroke::new(2.2, Color32::from_rgb(56, 189, 248))
                    } else {
                        Stroke::new(1.2, theme.text_secondary.gamma_multiply(0.7))
                    },
                ));
            }

            // 5. Structural Columns (Flat 2D Projection)
            for col in &bim_project.columns {
                let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Column, col.id));
                let ring: Vec<Pos2> = col
                    .footprint_ring()
                    .into_iter()
                    .map(|p| to_screen([p.0, p.1]))
                    .collect();
                let fill = if is_sel {
                    Color32::from_rgb(56, 189, 248)
                } else if active_pillar == NavigationPillar::AnalysisSimulation {
                    let util = bim_project
                        .fem_id_for_bim_element(crate::core::BimElementKind::Column, col.id)
                        .and_then(|fid| members.iter().find(|m| m.id == fid))
                        .map(|m| m.utilization)
                        .unwrap_or(0.45);
                    if util >= 0.85 {
                        theme.accent_red.gamma_multiply(0.85)
                    } else if util >= 0.65 {
                        theme.accent_orange.gamma_multiply(0.85)
                    } else {
                        theme.accent_green.gamma_multiply(0.85)
                    }
                } else {
                    theme.accent_blue.gamma_multiply(0.75)
                };
                clip_painter.add(egui::Shape::convex_polygon(
                    ring,
                    fill,
                    if is_sel {
                        Stroke::new(2.2, Color32::WHITE)
                    } else {
                        Stroke::new(1.2, theme.text_primary)
                    },
                ));
                if is_sel {
                    let cp = to_screen([col.pos.0, col.pos.1]);
                    if rect.expand(12.0).contains(cp) {
                        clip_painter.circle_stroke(cp, 14.0, Stroke::new(2.0, Color32::from_rgb(56, 189, 248)));
                        clip_painter.text(
                            cp + Vec2::new(0.0, -18.0),
                            Align2::CENTER_BOTTOM,
                            &col.name,
                            egui::FontId::monospace(10.0),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }

        // 6. Phase 55: Full 3D Extruded BIM Architecture, Structural Lenses & Ghost Authoring directly on Earth 3D
        if self.globe_mode
            && (self.ground_origin_anchored
                || !bim_project.is_empty()
                || self.earth_bim_author.active_tool != EarthBimTool::None)
        {
            if let Some(ref xf) = self.last_globe_xform {
                let lift_scale = if self.world_map.zoom_level < 14.5 { 8.0 } else { 1.0 };
                let proj_mm = |x_mm: f64, y_mm: f64, elev_m: f64| -> Option<Pos2> {
                    self.project_active_bim_mm(xf, x_mm, y_mm, elev_m, lift_scale)
                };
                let lens_color = |kind: crate::core::BimElementKind,
                                  id: usize,
                                  level_idx: usize,
                                  default_col: Color32,
                                  is_sel: bool| -> Color32 {
                    if is_sel {
                        return Color32::from_rgb(56, 189, 248);
                    }
                    let mem_opt = bim_project
                        .fem_id_for_bim_element(kind, id)
                        .and_then(|fid| members.iter().find(|m| m.id == fid));
                    match self.earth_bim_author.active_lens {
                        EarthBimLens::Clean => default_col,
                        EarthBimLens::DcrHeatmap => {
                            let u = mem_opt.map(|m| m.utilization).unwrap_or(0.40);
                            if u >= 0.85 || mem_opt.is_some_and(|m| m.is_critical) {
                                theme.accent_red
                            } else if u >= 0.65 {
                                theme.accent_orange
                            } else {
                                theme.accent_green
                            }
                        }
                        EarthBimLens::AriRedundancy => {
                            let ari = mem_opt.map(|m| m.ari).unwrap_or(0.95);
                            if ari < 0.70 {
                                theme.accent_red
                            } else if ari < 0.85 {
                                theme.accent_orange
                            } else {
                                Color32::from_rgb(16, 185, 129)
                            }
                        }
                        EarthBimLens::MomentStress => {
                            let m_max = mem_opt
                                .map(|m| m.moment_my_knm.abs().max(m.moment_mz_knm.abs()))
                                .unwrap_or(15.0);
                            if m_max > 120.0 {
                                Color32::from_rgb(244, 63, 94)
                            } else if m_max > 55.0 {
                                Color32::from_rgb(217, 70, 239)
                            } else {
                                Color32::from_rgb(129, 140, 248)
                            }
                        }
                        EarthBimLens::Erection4D => {
                            if level_idx == 0 {
                                Color32::from_rgb(16, 185, 129)
                            } else if level_idx == 1 {
                                Color32::from_rgb(56, 189, 248)
                            } else {
                                Color32::from_rgb(251, 191, 36)
                            }
                        }
                    }
                };
                let level_rank = |lvl_id: usize| -> usize {
                    bim_project.levels.iter().position(|l| l.id == lvl_id).unwrap_or(0)
                };
                let isolate = self.earth_bim_author.isolate_active_story;
                let active_lvl_id = self.earth_bim_author.base_level_id.max(1);
                let active_top_id = self.earth_bim_author.top_level_id.max(active_lvl_id);
                let show_on_story = |lvl: usize| -> bool { !isolate || lvl == active_lvl_id };

                // 6A. Isolated Footings (Ground Pads)
                for ft in &bim_project.footings {
                    if !show_on_story(ft.level_id) && !(isolate && active_lvl_id <= 1) {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Footing, ft.id));
                    let z_m = bim_project.level_elevation(ft.level_id) * 0.001;
                    let pts: Vec<Pos2> = ft
                        .footprint_ring()
                        .into_iter()
                        .filter_map(|p| proj_mm(p.0, p.1, z_m))
                        .collect();
                    if pts.len() >= 3 && pts.iter().any(|p| rect.expand(40.0).contains(*p)) {
                        let fill = if is_sel {
                            Color32::from_rgba_unmultiplied(56, 189, 248, 120)
                        } else {
                            Color32::from_rgba_unmultiplied(245, 158, 11, 70)
                        };
                        let stroke = if is_sel {
                            Stroke::new(2.2, Color32::from_rgb(56, 189, 248))
                        } else {
                            Stroke::new(1.2, Color32::from_rgba_unmultiplied(245, 158, 11, 200))
                        };
                        clip_painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
                    }
                }

                // 6B. Floor & Roof Slabs
                for sl in &bim_project.slabs {
                    if sl.boundary.len() < 3 {
                        continue;
                    }
                    if isolate && sl.level_id != active_lvl_id && sl.level_id != active_top_id {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Slab, sl.id));
                    let z_m = bim_project.level_elevation(sl.level_id) * 0.001;
                    let pts: Vec<Pos2> = sl
                        .boundary
                        .iter()
                        .filter_map(|p| proj_mm(p.0, p.1, z_m))
                        .collect();
                    if pts.len() >= 3 && pts.iter().any(|p| rect.expand(60.0).contains(*p)) {
                        let fill = if is_sel {
                            Color32::from_rgba_unmultiplied(56, 189, 248, 95)
                        } else if sl.is_roof {
                            Color32::from_rgba_unmultiplied(234, 88, 12, 55)
                        } else {
                            Color32::from_rgba_unmultiplied(148, 163, 184, 52)
                        };
                        let stroke = if is_sel {
                            Stroke::new(2.2, Color32::from_rgb(56, 189, 248))
                        } else if sl.is_roof {
                            Stroke::new(1.3, Color32::from_rgba_unmultiplied(251, 146, 60, 210))
                        } else {
                            Stroke::new(1.1, Color32::from_rgba_unmultiplied(203, 213, 225, 175))
                        };
                        clip_painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
                    }
                }

                // 6C. Extruded 3D Walls & Door/Window Openings
                for w in &bim_project.walls {
                    if !show_on_story(w.base_level_id) {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Wall, w.id));
                    let z0 = bim_project.level_elevation(w.base_level_id) * 0.001;
                    let z1 = w
                        .top_level_id
                        .map(|tid| bim_project.level_elevation(tid))
                        .unwrap_or_else(|| bim_project.level_elevation(w.base_level_id) + w.unconnected_height_mm)
                        * 0.001;
                    if let (Some(b0), Some(b1), Some(t1), Some(t0)) = (
                        proj_mm(w.start.0, w.start.1, z0),
                        proj_mm(w.end.0, w.end.1, z0),
                        proj_mm(w.end.0, w.end.1, z1),
                        proj_mm(w.start.0, w.start.1, z1),
                    ) {
                        if [b0, b1, t1, t0].iter().any(|p| rect.expand(60.0).contains(*p)) {
                            let wall_fill = if is_sel {
                                Color32::from_rgba_unmultiplied(56, 189, 248, 125)
                            } else if w.is_structural {
                                Color32::from_rgba_unmultiplied(100, 116, 139, 130)
                            } else {
                                Color32::from_rgba_unmultiplied(148, 163, 184, 95)
                            };
                            let wall_stroke = if is_sel {
                                Stroke::new(2.3, Color32::from_rgb(56, 189, 248))
                            } else {
                                Stroke::new(1.3, Color32::from_rgba_unmultiplied(226, 232, 240, 185))
                            };
                            clip_painter.add(egui::Shape::convex_polygon(
                                vec![b0, b1, t1, t0],
                                wall_fill,
                                wall_stroke,
                            ));
                            // Render CSG Door/Window Openings directly on the 3D wall face
                            let len_mm = w.length_mm().max(1.0);
                            for op in &w.openings {
                                let u0 = (op.offset_along_mm / len_mm).clamp(0.0, 1.0);
                                let u1 = ((op.offset_along_mm + op.width_mm) / len_mm).clamp(0.0, 1.0);
                                let oz0 = z0 + op.sill_height_mm * 0.001;
                                let oz1 = oz0 + op.height_mm * 0.001;
                                let x0 = w.start.0 + (w.end.0 - w.start.0) * u0;
                                let y0 = w.start.1 + (w.end.1 - w.start.1) * u0;
                                let x1 = w.start.0 + (w.end.0 - w.start.0) * u1;
                                let y1 = w.start.1 + (w.end.1 - w.start.1) * u1;
                                if let (Some(ob0), Some(ob1), Some(ot1), Some(ot0)) = (
                                    proj_mm(x0, y0, oz0),
                                    proj_mm(x1, y1, oz0),
                                    proj_mm(x1, y1, oz1),
                                    proj_mm(x0, y0, oz1),
                                ) {
                                    let is_door = matches!(
                                        op.kind,
                                        crate::core::OpeningKind::SingleDoor
                                            | crate::core::OpeningKind::DoubleDoor
                                            | crate::core::OpeningKind::SlidingDoor
                                    );
                                    let (ofill, ocol) = if is_door {
                                        (
                                            Color32::from_rgba_unmultiplied(245, 158, 11, 115),
                                            Color32::from_rgb(251, 191, 36),
                                        )
                                    } else {
                                        (
                                            Color32::from_rgba_unmultiplied(56, 189, 248, 125),
                                            Color32::from_rgb(125, 211, 252),
                                        )
                                    };
                                    clip_painter.add(egui::Shape::convex_polygon(
                                        vec![ob0, ob1, ot1, ot0],
                                        ofill,
                                        Stroke::new(1.2, ocol),
                                    ));
                                }
                            }
                        }
                    }
                }

                // 6D. Structural Columns (3D Extruded Prisms at Ground Scale, Bold Shafts from Orbit)
                for col in &bim_project.columns {
                    if isolate && col.base_level_id != active_lvl_id && col.top_level_id != active_lvl_id {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Column, col.id));
                    let base_m = bim_project.level_elevation(col.base_level_id) * 0.001;
                    let top_m = bim_project.level_elevation(col.top_level_id) * 0.001;
                    let col_color = lens_color(
                        crate::core::BimElementKind::Column,
                        col.id,
                        level_rank(col.base_level_id),
                        Color32::from_rgba_unmultiplied(56, 189, 248, 235),
                        is_sel,
                    );
                    if self.world_map.zoom_level >= 16.5 {
                        let ring = col.footprint_ring();
                        let b_pts: Vec<Pos2> = ring.iter().filter_map(|p| proj_mm(p.0, p.1, base_m)).collect();
                        let t_pts: Vec<Pos2> = ring.iter().filter_map(|p| proj_mm(p.0, p.1, top_m)).collect();
                        if b_pts.len() == 4 && t_pts.len() == 4 && t_pts.iter().any(|p| rect.expand(45.0).contains(*p)) {
                            let side_fill = col_color.gamma_multiply(0.32);
                            for i in 0..4 {
                                let ni = (i + 1) % 4;
                                clip_painter.add(egui::Shape::convex_polygon(
                                    vec![b_pts[i], b_pts[ni], t_pts[ni], t_pts[i]],
                                    side_fill,
                                    Stroke::new(if is_sel { 1.8 } else { 1.0 }, col_color),
                                ));
                            }
                            clip_painter.add(egui::Shape::convex_polygon(
                                t_pts,
                                col_color.gamma_multiply(0.55),
                                Stroke::new(if is_sel { 2.2 } else { 1.2 }, Color32::WHITE.gamma_multiply(0.7)),
                            ));
                        }
                    }
                    if let (Some(p0), Some(p1)) = (
                        proj_mm(col.pos.0, col.pos.1, base_m),
                        proj_mm(col.pos.0, col.pos.1, top_m),
                    ) {
                        if rect.expand(45.0).contains(p0) || rect.expand(45.0).contains(p1) {
                            clip_painter.line_segment(
                                [p0, p1],
                                Stroke::new(if is_sel { 4.2 } else { 2.4 }, col_color),
                            );
                            if is_sel {
                                clip_painter.circle_stroke(p1, 10.0, Stroke::new(1.8, Color32::WHITE));
                                clip_painter.text(
                                    p1 + Vec2::new(0.0, -14.0),
                                    Align2::CENTER_BOTTOM,
                                    &col.name,
                                    egui::FontId::monospace(10.0),
                                    Color32::WHITE,
                                );
                            }
                        }
                    }
                }

                // 6E. Structural Beams / Runs
                for run in &bim_project.runs {
                    if isolate && run.level_id != active_lvl_id && run.level_id != active_top_id {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Run, run.id));
                    let elev_m = bim_project.level_elevation(run.level_id) * 0.001;
                    if let (Some(p0), Some(p1)) = (
                        proj_mm(run.start.0, run.start.1, elev_m),
                        proj_mm(run.end.0, run.end.1, elev_m),
                    ) {
                        if rect.expand(45.0).contains(p0) || rect.expand(45.0).contains(p1) {
                            let beam_col = lens_color(
                                crate::core::BimElementKind::Run,
                                run.id,
                                level_rank(run.level_id),
                                theme.accent_green.gamma_multiply(0.92),
                                is_sel,
                            );
                            clip_painter.line_segment(
                                [p0, p1],
                                Stroke::new(if is_sel { 4.2 } else { 2.3 }, beam_col),
                            );
                        }
                    }
                }

                // 6F. Live 3D Wireframe Ghost Preview on Earth 3D Surface
                if self.earth_bim_author.active_tool != EarthBimTool::None {
                    if let Some(snap_pt) = self.earth_bim_author.hover_snap_mm {
                        let base_m = bim_project.level_elevation(self.earth_bim_author.base_level_id) * 0.001;
                        let top_m = bim_project.level_elevation(self.earth_bim_author.top_level_id) * 0.001;
                        let top_z = if (top_m - base_m).abs() < 0.2 { base_m + 3.5 } else { top_m };
                        let ghost_col = Color32::from_rgb(0, 240, 255);
                        let ghost_stroke = Stroke::new(1.8, ghost_col);

                        if let Some(sp) = proj_mm(snap_pt[0], snap_pt[1], base_m) {
                            clip_painter.circle_stroke(sp, 6.5, ghost_stroke);
                            clip_painter.circle_filled(sp, 2.5, ghost_col);
                        }

                        match self.earth_bim_author.active_tool {
                            EarthBimTool::Column => {
                                let hw = self.earth_bim_author.col_width_mm * 0.5;
                                let hd = self.earth_bim_author.col_depth_mm * 0.5;
                                let corners = [
                                    (snap_pt[0] - hw, snap_pt[1] - hd),
                                    (snap_pt[0] + hw, snap_pt[1] - hd),
                                    (snap_pt[0] + hw, snap_pt[1] + hd),
                                    (snap_pt[0] - hw, snap_pt[1] + hd),
                                ];
                                let b_pts: Vec<Pos2> = corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, base_m)).collect();
                                let t_pts: Vec<Pos2> = corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, top_z)).collect();
                                if b_pts.len() == 4 && t_pts.len() == 4 {
                                    for i in 0..4 {
                                        let ni = (i + 1) % 4;
                                        clip_painter.line_segment([b_pts[i], b_pts[ni]], ghost_stroke);
                                        clip_painter.line_segment([t_pts[i], t_pts[ni]], ghost_stroke);
                                        clip_painter.line_segment([b_pts[i], t_pts[i]], ghost_stroke);
                                    }
                                }
                            }
                            EarthBimTool::Footing => {
                                let hs = self.earth_bim_author.footing_size_mm * 0.5;
                                let corners = [
                                    (snap_pt[0] - hs, snap_pt[1] - hs),
                                    (snap_pt[0] + hs, snap_pt[1] - hs),
                                    (snap_pt[0] + hs, snap_pt[1] + hs),
                                    (snap_pt[0] - hs, snap_pt[1] + hs),
                                ];
                                let pts: Vec<Pos2> = corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, base_m)).collect();
                                if pts.len() == 4 {
                                    clip_painter.add(egui::Shape::convex_polygon(
                                        pts,
                                        Color32::from_rgba_unmultiplied(0, 240, 255, 40),
                                        ghost_stroke,
                                    ));
                                }
                            }
                            EarthBimTool::Wall | EarthBimTool::Beam => {
                                if let Some(s) = self.earth_bim_author.pending_start_mm {
                                    let z_draw = if self.earth_bim_author.active_tool == EarthBimTool::Beam {
                                        top_z
                                    } else {
                                        base_m
                                    };
                                    if let (Some(p0), Some(p1)) = (
                                        proj_mm(s[0], s[1], z_draw),
                                        proj_mm(snap_pt[0], snap_pt[1], z_draw),
                                    ) {
                                        clip_painter.line_segment([p0, p1], Stroke::new(2.5, ghost_col));
                                        if self.earth_bim_author.active_tool == EarthBimTool::Wall {
                                            if let (Some(t0), Some(t1)) = (
                                                proj_mm(s[0], s[1], top_z),
                                                proj_mm(snap_pt[0], snap_pt[1], top_z),
                                            ) {
                                                clip_painter.add(egui::Shape::convex_polygon(
                                                    vec![p0, p1, t1, t0],
                                                    Color32::from_rgba_unmultiplied(0, 240, 255, 42),
                                                    ghost_stroke,
                                                ));
                                            }
                                        }
                                        let dx_m = (snap_pt[0] - s[0]) * 0.001;
                                        let dy_m = (snap_pt[1] - s[1]) * 0.001;
                                        let len_m = dx_m.hypot(dy_m);
                                        let ang_deg = dy_m.atan2(dx_m).to_degrees();
                                        let mid = Pos2::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5 - 10.0);
                                        clip_painter.text(
                                            mid,
                                            Align2::CENTER_BOTTOM,
                                            format!("{len_m:.2} m @ {ang_deg:.0}°"),
                                            egui::FontId::monospace(10.5),
                                            ghost_col,
                                        );
                                    }
                                }
                            }
                            EarthBimTool::Slab => {
                                if let Some(s) = self.earth_bim_author.pending_start_mm {
                                    let min_x = s[0].min(snap_pt[0]);
                                    let max_x = s[0].max(snap_pt[0]);
                                    let min_y = s[1].min(snap_pt[1]);
                                    let max_y = s[1].max(snap_pt[1]);
                                    let corners = [
                                        (min_x, min_y),
                                        (max_x, min_y),
                                        (max_x, max_y),
                                        (min_x, max_y),
                                    ];
                                    let pts: Vec<Pos2> = corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, base_m)).collect();
                                    if pts.len() == 4 {
                                        clip_painter.add(egui::Shape::convex_polygon(
                                            pts,
                                            Color32::from_rgba_unmultiplied(0, 240, 255, 48),
                                            ghost_stroke,
                                        ));
                                    }
                                    if let Some(p_lbl) = proj_mm(0.5 * (min_x + max_x), 0.5 * (min_y + max_y), base_m) {
                                        clip_painter.text(
                                            p_lbl,
                                            Align2::CENTER_CENTER,
                                            format!("{:.2} x {:.2} m", (max_x - min_x) * 0.001, (max_y - min_y) * 0.001),
                                            egui::FontId::monospace(10.5),
                                            ghost_col,
                                        );
                                    }
                                }
                            }
                            EarthBimTool::Door | EarthBimTool::Window => {
                                let is_door = self.earth_bim_author.active_tool == EarthBimTool::Door;
                                if let Some(w) = bim_project
                                    .walls
                                    .iter()
                                    .filter(|w| show_on_story(w.base_level_id))
                                    .min_by(|a, b| {
                                        let dist_w = |wl: &crate::core::BimWall| -> f64 {
                                            let ab_x = wl.end.0 - wl.start.0;
                                            let ab_y = wl.end.1 - wl.start.1;
                                            let l2 = (ab_x * ab_x + ab_y * ab_y).max(1.0);
                                            let t = (((snap_pt[0] - wl.start.0) * ab_x + (snap_pt[1] - wl.start.1) * ab_y) / l2).clamp(0.0, 1.0);
                                            ((snap_pt[0] - (wl.start.0 + t * ab_x)).powi(2)
                                                + (snap_pt[1] - (wl.start.1 + t * ab_y)).powi(2))
                                            .sqrt()
                                        };
                                        dist_w(a).partial_cmp(&dist_w(b)).unwrap_or(std::cmp::Ordering::Equal)
                                    })
                                {
                                    let wlen = w.start.dist(w.end).max(400.0);
                                    let ux = (w.end.0 - w.start.0) / wlen;
                                    let uy = (w.end.1 - w.start.1) / wlen;
                                    let proj_along = (snap_pt[0] - w.start.0) * ux + (snap_pt[1] - w.start.1) * uy;
                                    let (ow, oh, osill) = if is_door { (900.0, 2100.0, 0.0) } else { (1800.0, 1400.0, 900.0) };
                                    let off = (proj_along - ow * 0.5).clamp(120.0, (wlen - ow - 120.0).max(120.0));
                                    let u0 = (off / wlen).clamp(0.0, 1.0);
                                    let u1 = ((off + ow) / wlen).clamp(0.0, 1.0);
                                    let wz0 = bim_project.level_elevation(w.base_level_id) * 0.001 + osill * 0.001;
                                    let wz1 = wz0 + oh * 0.001;
                                    if let (Some(p0), Some(p1), Some(p2), Some(p3)) = (
                                        proj_mm(w.start.0 + (w.end.0 - w.start.0) * u0, w.start.1 + (w.end.1 - w.start.1) * u0, wz0),
                                        proj_mm(w.start.0 + (w.end.0 - w.start.0) * u1, w.start.1 + (w.end.1 - w.start.1) * u1, wz0),
                                        proj_mm(w.start.0 + (w.end.0 - w.start.0) * u1, w.start.1 + (w.end.1 - w.start.1) * u1, wz1),
                                        proj_mm(w.start.0 + (w.end.0 - w.start.0) * u0, w.start.1 + (w.end.1 - w.start.1) * u0, wz1),
                                    ) {
                                        clip_painter.add(egui::Shape::convex_polygon(
                                            vec![p0, p1, p2, p3],
                                            Color32::from_rgba_unmultiplied(0, 240, 255, 90),
                                            ghost_stroke,
                                        ));
                                    }
                                }
                            }
                            EarthBimTool::None => {}
                        }
                    }
                }
            }
        }
    }

    /// Screen-space hit-testing against projected BIM Columns, Beams/Runs, Walls, Slabs, and Footings
    /// across both tilted 3D Earth projections and orthographic ground projections.
    pub fn pick_bim_element_at_screen(
        &self,
        pos: Pos2,
        rect: Rect,
        bim_project: &BimProject,
    ) -> Option<(crate::core::BimElementKind, usize)> {
        if bim_project.is_empty() || !rect.contains(pos) {
            return None;
        }
        let seg_dist = |p: Pos2, a: Pos2, b: Pos2| -> f32 {
            let ab = b - a;
            let len_sq = ab.length_sq();
            if len_sq <= 1e-4 {
                return p.distance(a);
            }
            let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
            p.distance(a + ab * t)
        };
        let point_in_poly = |pt: Pos2, poly: &[Pos2]| -> bool {
            if poly.len() < 3 {
                return false;
            }
            let mut inside = false;
            let mut j = poly.len() - 1;
            for i in 0..poly.len() {
                let pi = poly[i];
                let pj = poly[j];
                if ((pi.y > pt.y) != (pj.y > pt.y))
                    && (pt.x < (pj.x - pi.x) * (pt.y - pi.y) / ((pj.y - pi.y) + 1e-6) + pi.x)
                {
                    inside = !inside;
                }
                j = i;
            }
            inside
        };

        // Height-aware 3D Earth Globe picking when rendering in Earth 3D
        if self.globe_mode && self.ground_origin_anchored {
            if let Some(ref xf) = self.last_globe_xform {
                let lift_scale = if self.world_map.zoom_level < 14.5 { 8.0 } else { 1.0 };
                let proj_mm = |x_mm: f64, y_mm: f64, elev_m: f64| -> Option<Pos2> {
                    self.project_active_bim_mm(xf, x_mm, y_mm, elev_m, lift_scale)
                };

                let isolate = self.earth_bim_author.isolate_active_story;
                let active_lvl_id = self.earth_bim_author.base_level_id.max(1);
                let active_top_id = self.earth_bim_author.top_level_id.max(active_lvl_id);

                // 1. Columns (3D vertical shaft)
                let mut best_col: Option<(f32, usize)> = None;
                for c in &bim_project.columns {
                    if isolate && c.base_level_id != active_lvl_id && c.top_level_id != active_lvl_id {
                        continue;
                    }
                    let z0 = bim_project.level_elevation(c.base_level_id) * 0.001;
                    let z1 = bim_project.level_elevation(c.top_level_id) * 0.001;
                    if let (Some(p0), Some(p1)) = (proj_mm(c.pos.0, c.pos.1, z0), proj_mm(c.pos.0, c.pos.1, z1)) {
                        let d = seg_dist(pos, p0, p1);
                        if d <= 18.0 && best_col.is_none_or(|(bd, _)| d < bd) {
                            best_col = Some((d, c.id));
                        }
                    }
                }
                if let Some((_, id)) = best_col {
                    return Some((crate::core::BimElementKind::Column, id));
                }

                // 2. Beams / Runs (at elevated storey height)
                let mut best_run: Option<(f32, usize)> = None;
                for r in &bim_project.runs {
                    if isolate && r.level_id != active_lvl_id && r.level_id != active_top_id {
                        continue;
                    }
                    let z = bim_project.level_elevation(r.level_id) * 0.001;
                    if let (Some(p0), Some(p1)) = (proj_mm(r.start.0, r.start.1, z), proj_mm(r.end.0, r.end.1, z)) {
                        let d = seg_dist(pos, p0, p1);
                        if d <= 14.0 && best_run.is_none_or(|(bd, _)| d < bd) {
                            best_run = Some((d, r.id));
                        }
                    }
                }
                if let Some((_, id)) = best_run {
                    return Some((crate::core::BimElementKind::Run, id));
                }

                // 3. Walls (3D extruded face quad or chords)
                for w in bim_project.walls.iter().rev() {
                    if isolate && w.base_level_id != active_lvl_id {
                        continue;
                    }
                    let z0 = bim_project.level_elevation(w.base_level_id) * 0.001;
                    let z1 = w
                        .top_level_id
                        .map(|tid| bim_project.level_elevation(tid))
                        .unwrap_or_else(|| bim_project.level_elevation(w.base_level_id) + w.unconnected_height_mm)
                        * 0.001;
                    if let (Some(b0), Some(b1), Some(t1), Some(t0)) = (
                        proj_mm(w.start.0, w.start.1, z0),
                        proj_mm(w.end.0, w.end.1, z0),
                        proj_mm(w.end.0, w.end.1, z1),
                        proj_mm(w.start.0, w.start.1, z1),
                    ) {
                        let quad = [b0, b1, t1, t0];
                        if point_in_poly(pos, &quad)
                            || seg_dist(pos, b0, b1) <= 10.0
                            || seg_dist(pos, t0, t1) <= 10.0
                        {
                            return Some((crate::core::BimElementKind::Wall, w.id));
                        }
                    }
                }

                // 4. Footings
                for f in &bim_project.footings {
                    if isolate && active_lvl_id > 1 && f.level_id != active_lvl_id {
                        continue;
                    }
                    let z = bim_project.level_elevation(f.level_id) * 0.001;
                    if let Some(sp) = proj_mm(f.pos.0, f.pos.1, z) {
                        if pos.distance(sp) <= 18.0 {
                            return Some((crate::core::BimElementKind::Footing, f.id));
                        }
                    }
                }

                // 5. Slabs / Roofs (elevated 3D polygon)
                for sl in bim_project.slabs.iter().rev() {
                    if isolate && sl.level_id != active_lvl_id && sl.level_id != active_top_id {
                        continue;
                    }
                    let z = bim_project.level_elevation(sl.level_id) * 0.001;
                    let pts: Vec<Pos2> = sl
                        .boundary
                        .iter()
                        .filter_map(|p| proj_mm(p.0, p.1, z))
                        .collect();
                    if point_in_poly(pos, &pts) {
                        return Some((crate::core::BimElementKind::Slab, sl.id));
                    }
                }
            }
        }

        // Orthographic fallback picking
        // 1. Columns (highest priority at grid intersections)
        let mut best_col: Option<(f32, usize)> = None;
        for c in &bim_project.columns {
            let sp = self.world_to_screen(rect, [c.pos.0, c.pos.1]);
            if !rect.expand(24.0).contains(sp) {
                continue;
            }
            let d = pos.distance(sp);
            if d <= 16.0 && best_col.is_none_or(|(bd, _)| d < bd) {
                best_col = Some((d, c.id));
            }
        }
        if let Some((_, id)) = best_col {
            return Some((crate::core::BimElementKind::Column, id));
        }

        // 2. Beams / Structural Runs
        let mut best_run: Option<(f32, usize)> = None;
        for r in &bim_project.runs {
            let s0 = self.world_to_screen(rect, [r.start.0, r.start.1]);
            let s1 = self.world_to_screen(rect, [r.end.0, r.end.1]);
            if !rect.expand(30.0).contains(s0) && !rect.expand(30.0).contains(s1) {
                continue;
            }
            let d = seg_dist(pos, s0, s1);
            if d <= 11.0 && best_run.is_none_or(|(bd, _)| d < bd) {
                best_run = Some((d, r.id));
            }
        }
        if let Some((_, id)) = best_run {
            return Some((crate::core::BimElementKind::Run, id));
        }

        // 3. Walls
        let mut best_wall: Option<(f32, usize)> = None;
        for w in &bim_project.walls {
            let s0 = self.world_to_screen(rect, [w.start.0, w.start.1]);
            let s1 = self.world_to_screen(rect, [w.end.0, w.end.1]);
            if !rect.expand(30.0).contains(s0) && !rect.expand(30.0).contains(s1) {
                continue;
            }
            let d = seg_dist(pos, s0, s1);
            if d <= 12.0 && best_wall.is_none_or(|(bd, _)| d < bd) {
                best_wall = Some((d, w.id));
            }
        }
        if let Some((_, id)) = best_wall {
            return Some((crate::core::BimElementKind::Wall, id));
        }

        // 4. Isolated Footings
        let mut best_ft: Option<(f32, usize)> = None;
        for f in &bim_project.footings {
            let sp = self.world_to_screen(rect, [f.pos.0, f.pos.1]);
            if !rect.expand(24.0).contains(sp) {
                continue;
            }
            let d = pos.distance(sp);
            if d <= 16.0 && best_ft.is_none_or(|(bd, _)| d < bd) {
                best_ft = Some((d, f.id));
            }
        }
        if let Some((_, id)) = best_ft {
            return Some((crate::core::BimElementKind::Footing, id));
        }

        // 5. Floor Slabs / Roofs (reverse order prefers top-most/latest slab)
        for sl in bim_project.slabs.iter().rev() {
            if sl.boundary.len() < 3 {
                continue;
            }
            let pts: Vec<Pos2> = sl
                .boundary
                .iter()
                .map(|p| self.world_to_screen(rect, [p.0, p.1]))
                .collect();
            if point_in_poly(pos, &pts) {
                return Some((crate::core::BimElementKind::Slab, sl.id));
            }
        }
        None
    }

    /// Projects local active-building `(x_mm, y_mm, elev_m)` onto Earth 3D applying active heading rotation cleanly.
    pub fn project_active_bim_mm(
        &self,
        xf: &GlobeScreenTransform,
        x_mm: f64,
        y_mm: f64,
        elev_m: f64,
        lift_scale: f64,
    ) -> Option<Pos2> {
        let heading = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let (rx_mm, ry_mm) = if heading.abs() > 1e-3 {
            let rad = heading.to_radians();
            let (s, c) = (rad.sin(), rad.cos());
            (x_mm * c - y_mm * s, x_mm * s + y_mm * c)
        } else {
            (x_mm, y_mm)
        };
        let ll = SlippyMapState::world_mm_to_lat_lon(self.ground_origin_lat_lon, [rx_mm, ry_mm]);
        let relief = self.local_site_relief_m(ll.0, ll.1);
        xf.project_lat_lon_elev(ll.0, ll.1, relief + elev_m * lift_scale)
            .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
    }

    /// Computes conservative `(min_lat, max_lat, min_lon, max_lon)` visible across `xform.rect`.
    pub fn visible_geo_bbox(&self, xform: &GlobeScreenTransform) -> (f64, f64, f64, f64) {
        if self.world_map.zoom_level < 4.5 {
            return (-85.0, 85.0, -180.0, 180.0);
        }
        let mut min_lat = 85.0_f64;
        let mut max_lat = -85.0_f64;
        let mut min_lon = 180.0_f64;
        let mut max_lon = -180.0_f64;
        let mut hits = 0usize;
        for gy in 0..=4 {
            let fy = gy as f32 * 0.25;
            for gx in 0..=4 {
                let fx = gx as f32 * 0.25;
                let sp = Pos2::new(
                    xform.rect.min.x + xform.rect.width() * fx,
                    xform.rect.min.y + xform.rect.height() * fy,
                );
                let (la, lo, hit) = xform.screen_to_lat_lon(sp);
                if hit {
                    hits += 1;
                    min_lat = min_lat.min(la);
                    max_lat = max_lat.max(la);
                    min_lon = min_lon.min(lo);
                    max_lon = max_lon.max(lo);
                }
            }
        }
        if hits < 6 || (max_lon - min_lon) > 160.0 {
            return (-85.0, 85.0, -180.0, 180.0);
        }
        let pad = if self.world_map.zoom_level < 10.0 { 0.35 } else { 0.04 };
        (
            (min_lat - pad).clamp(-85.0, 85.0),
            (max_lat + pad).clamp(-85.0, 85.0),
            min_lon - pad,
            max_lon + pad,
        )
    }

    /// Renders all visible Vault Geo-BIM projects simultaneously on Earth 3D across LOD 0/1/2
    /// plus interactive cursor-tracked ghost footprint when placing an unlocated model.
    pub fn paint_planetary_world_buildings(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
    ) {
        if self.world_index.buildings.is_empty() && self.placing_unlocated_rel_path.is_none() {
            return;
        }
        let Some(ref xf) = self.last_globe_xform else {
            return;
        };
        let clip_painter = painter.with_clip_rect(rect);
        let (min_lat, max_lat, min_lon, max_lon) = self.visible_geo_bbox(xf);

        // LOD 0: Continental / Regional Cluster Beacons (`zoom < 8.5`)
        if self.world_map.zoom_level < 8.5 {
            let clusters = self.world_index.query_clusters(
                min_lat,
                max_lat,
                min_lon,
                max_lon,
                self.world_map.zoom_level,
            );
            for cl in clusters.into_iter().take(120) {
                let Some((sp, _, vis)) = xf.project_lat_lon_elev(cl.lat, cl.lon, 0.0) else {
                    continue;
                };
                if !vis || !rect.expand(36.0).contains(sp) {
                    continue;
                }
                let is_staging = cl.unlocated_count > 0 && cl.unlocated_count == cl.count;
                let col = if is_staging {
                    theme.accent_orange
                } else if cl.min_ari < 0.72 || cl.max_dcr >= 0.90 {
                    theme.accent_red
                } else {
                    theme.accent_blue
                };
                let r = if cl.count > 100 {
                    15.0
                } else if cl.count > 10 {
                    12.5
                } else {
                    10.0
                };
                clip_painter.circle_filled(sp, r + 3.0, col.gamma_multiply(0.28));
                clip_painter.circle_filled(sp, r, col);
                clip_painter.circle_stroke(sp, r, Stroke::new(1.5, Color32::WHITE));
                let num_lbl = if cl.count >= 1000 {
                    format!("{:.1}k", cl.count as f64 / 1000.0)
                } else {
                    format!("{}", cl.count)
                };
                clip_painter.text(
                    sp,
                    Align2::CENTER_CENTER,
                    num_lbl,
                    egui::FontId::proportional(9.5),
                    Color32::WHITE,
                );
                if r >= 11.0 || self.world_map.zoom_level >= 5.0 {
                    clip_painter.text(
                        sp + Vec2::new(0.0, r + 4.0),
                        Align2::CENTER_TOP,
                        &cl.label,
                        egui::FontId::proportional(9.5),
                        Color32::WHITE,
                    );
                }
            }
        } else {
            // LOD 1 / 2: Simultaneous 2.5D / 3D City Extruded Massing across visible neighbor buildings
            let lift_scale = if self.world_map.zoom_level < 14.5 { 6.0 } else { 1.0 };
            let visible = self.world_index.query_visible_indices(
                min_lat,
                max_lat,
                min_lon,
                max_lon,
                xf.center_lat_deg,
                xf.center_lon_deg,
                450,
            );
            for idx in visible {
                let b = &self.world_index.buildings[idx];
                if self.active_world_project_rel_path.as_deref() == Some(b.rel_path.as_str()) {
                    continue;
                }
                let is_hovered =
                    self.hovered_world_building_rel_path.as_deref() == Some(b.rel_path.as_str());
                let hull_mm = b.rotated_footprint_world_mm();
                let mut base_pts = Vec::with_capacity(hull_mm.len());
                let mut roof_pts = Vec::with_capacity(hull_mm.len());
                let base_elev = self.local_site_relief_m(b.lat, b.lon);
                let top_elev = base_elev + b.total_height_m * lift_scale;
                let mut ok = true;
                for pt_mm in &hull_mm {
                    let ll = SlippyMapState::world_mm_to_lat_lon((b.lat, b.lon), *pt_mm);
                    if let (Some((bp, _, v0)), Some((tp, _, v1))) = (
                        xf.project_lat_lon_elev(ll.0, ll.1, base_elev),
                        xf.project_lat_lon_elev(ll.0, ll.1, top_elev),
                    ) {
                        if !v0 || !v1 {
                            ok = false;
                            break;
                        }
                        base_pts.push(bp);
                        roof_pts.push(tp);
                    } else {
                        ok = false;
                        break;
                    }
                }
                if !ok || roof_pts.len() < 3 {
                    continue;
                }
                if !roof_pts.iter().any(|p| rect.expand(45.0).contains(*p))
                    && !base_pts.iter().any(|p| rect.expand(45.0).contains(*p))
                {
                    continue;
                }

                let base_col = if b.is_unlocated() {
                    Color32::from_rgb(245, 158, 11)
                } else {
                    match self.earth_bim_author.active_lens {
                        EarthBimLens::Clean => Color32::from_rgb(96, 165, 250),
                        EarthBimLens::DcrHeatmap => {
                            if b.max_dcr >= 0.85 {
                                theme.accent_red
                            } else if b.max_dcr >= 0.65 {
                                theme.accent_orange
                            } else {
                                theme.accent_green
                            }
                        }
                        EarthBimLens::AriRedundancy => {
                            if b.min_ari < 0.70 {
                                theme.accent_red
                            } else if b.min_ari < 0.85 {
                                theme.accent_orange
                            } else {
                                Color32::from_rgb(16, 185, 129)
                            }
                        }
                        EarthBimLens::MomentStress => Color32::from_rgb(129, 140, 248),
                        EarthBimLens::Erection4D => Color32::from_rgb(56, 189, 248),
                    }
                };
                let side_fill = base_col.gamma_multiply(if is_hovered { 0.56 } else { 0.38 });
                let roof_fill = base_col.gamma_multiply(if is_hovered { 0.82 } else { 0.62 });
                let edge_stroke = if is_hovered {
                    Stroke::new(2.0, Color32::from_rgb(0, 240, 255))
                } else {
                    Stroke::new(1.1, base_col.gamma_multiply(0.95))
                };
                for i in 0..base_pts.len() {
                    let ni = (i + 1) % base_pts.len();
                    clip_painter.add(egui::Shape::convex_polygon(
                        vec![base_pts[i], base_pts[ni], roof_pts[ni], roof_pts[i]],
                        side_fill,
                        edge_stroke,
                    ));
                }
                clip_painter.add(egui::Shape::convex_polygon(
                    roof_pts.clone(),
                    roof_fill,
                    Stroke::new(
                        if is_hovered { 2.2 } else { 1.4 },
                        if is_hovered {
                            Color32::from_rgb(0, 240, 255)
                        } else {
                            Color32::WHITE.gamma_multiply(0.75)
                        },
                    ),
                ));

                if is_hovered {
                    let rc = roof_pts
                        .iter()
                        .fold(Vec2::ZERO, |acc, p| acc + p.to_vec2())
                        / (roof_pts.len() as f32);
                    let city_tag = if b.address_hint.is_empty() {
                        b.source.short_label().to_string()
                    } else {
                        b.address_hint.clone()
                    };
                    let tip = format!(
                        "{} • {} • {}F • DCR {:.0}% • ARI {:.2} [Click to Edit]",
                        b.name,
                        city_tag,
                        b.story_count,
                        b.max_dcr * 100.0,
                        b.min_ari
                    );
                    let tip_pos = Pos2::new(rc.x, rc.y - 12.0);
                    let galley = clip_painter.layout_no_wrap(
                        tip,
                        egui::FontId::proportional(10.0),
                        Color32::WHITE,
                    );
                    let pill_rect = Rect::from_center_size(
                        tip_pos - Vec2::new(0.0, galley.size().y * 0.5),
                        galley.size() + Vec2::new(14.0, 8.0),
                    );
                    clip_painter.rect_filled(
                        pill_rect,
                        CornerRadius::same(7),
                        Color32::from_rgba_unmultiplied(15, 23, 42, 232),
                    );
                    clip_painter.rect_stroke(
                        pill_rect,
                        CornerRadius::same(7),
                        Stroke::new(1.1, Color32::from_rgb(0, 240, 255)),
                        egui::StrokeKind::Inside,
                    );
                    clip_painter.galley(
                        pill_rect.min + Vec2::new(7.0, 4.0),
                        galley,
                        Color32::WHITE,
                    );
                } else if self.world_map.zoom_level >= 14.8 {
                    let rc = roof_pts
                        .iter()
                        .fold(Vec2::ZERO, |acc, p| acc + p.to_vec2())
                        / (roof_pts.len() as f32);
                    let lbl = if b.is_unlocated() {
                        format!("{} [Staging]", b.name)
                    } else {
                        b.name.clone()
                    };
                    clip_painter.text(
                        Pos2::new(rc.x, rc.y - 6.0),
                        Align2::CENTER_BOTTOM,
                        lbl,
                        egui::FontId::proportional(9.5),
                        Color32::WHITE,
                    );
                }
            }
        }

        // Interactive "Place on Earth" translucent ghost footprint following cursor
        if let (Some(ref rel), Some((hlat, hlon))) =
            (&self.placing_unlocated_rel_path, self.placing_hover_lat_lon)
        {
            let (hull_m, height_m) = if let Some(b) = self.world_index.get(rel) {
                (b.footprint_local_m.clone(), b.total_height_m)
            } else {
                (crate::core::gis::WorldBuildingProxy::default_footprint_m(), 14.0)
            };
            let base_relief = self.local_site_relief_m(hlat, hlon);
            let msl_elev = self.earth_world.effective_elevation_m(hlat, hlon);
            let rad = self.placing_heading_deg.to_radians();
            let (sh, ch) = (rad.sin(), rad.cos());
            let mut bpts = Vec::with_capacity(hull_m.len());
            let mut rpts = Vec::with_capacity(hull_m.len());
            for [xm, ym] in &hull_m {
                let rx_mm = (xm * ch - ym * sh) * 1000.0;
                let ry_mm = (xm * sh + ym * ch) * 1000.0;
                let ll = SlippyMapState::world_mm_to_lat_lon((hlat, hlon), [rx_mm, ry_mm]);
                if let (Some((bp, _, true)), Some((rp, _, true))) = (
                    xf.project_lat_lon_elev(ll.0, ll.1, base_relief),
                    xf.project_lat_lon_elev(ll.0, ll.1, base_relief + height_m),
                ) {
                    bpts.push(bp);
                    rpts.push(rp);
                }
            }
            if bpts.len() >= 3 && rpts.len() == bpts.len() {
                let ghost_stroke = Stroke::new(1.8, Color32::from_rgb(0, 240, 255));
                for i in 0..bpts.len() {
                    let ni = (i + 1) % bpts.len();
                    clip_painter.add(egui::Shape::convex_polygon(
                        vec![bpts[i], bpts[ni], rpts[ni], rpts[i]],
                        Color32::from_rgba_unmultiplied(0, 240, 255, 50),
                        ghost_stroke,
                    ));
                }
                clip_painter.add(egui::Shape::convex_polygon(
                    rpts.clone(),
                    Color32::from_rgba_unmultiplied(0, 240, 255, 85),
                    ghost_stroke,
                ));
                if let Some((cp, _, true)) = xf.project_lat_lon_elev(hlat, hlon, base_relief + height_m + 1.5) {
                    let snap_note = self
                        .active_vector_snap_az_deg
                        .map(|az| format!(" • Snap {az:.0}° [A]"))
                        .unwrap_or_default();
                    clip_painter.text(
                        cp,
                        Align2::CENTER_BOTTOM,
                        format!(
                            "Place Here ({hlat:.4}°, {hlon:.4}° | Hdg {:.0}° | {:+.0}m MSL{snap_note})",
                            self.placing_heading_deg,
                            msl_elev
                        ),
                        egui::FontId::monospace(10.0),
                        Color32::from_rgb(0, 240, 255),
                    );
                }
            }
        }
    }

    /// Renders inter-building seismic pounding separation gaps, Venturi wind corridors, and
    /// collapse-risk proximity rings directly over neighboring structures on Earth 3D.
    pub fn paint_neighborhood_hazards(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
    ) {
        if self.world_map.zoom_level < 14.2 || self.neighborhood_report.contacts.is_empty() {
            return;
        }
        let Some(ref xf) = self.last_globe_xform else {
            return;
        };
        let clip_painter = painter.with_clip_rect(rect);
        for contact in &self.neighborhood_report.contacts {
            let elev = self.local_site_relief_m(contact.neighbor_lat, contact.neighbor_lon) + 2.0;
            let (Some((pa, _, true)), Some((pb, _, true))) = (
                xf.project_lat_lon_elev(contact.closest_active_ll.0, contact.closest_active_ll.1, elev),
                xf.project_lat_lon_elev(contact.closest_neighbor_ll.0, contact.closest_neighbor_ll.1, elev),
            ) else {
                continue;
            };
            if !rect.expand(30.0).contains(pa) && !rect.expand(30.0).contains(pb) {
                continue;
            }
            use crate::core::gis::PoundingSeverity;
            let col = match contact.severity {
                PoundingSeverity::PoundingRisk => theme.accent_red,
                PoundingSeverity::TightBuffer => theme.accent_orange,
                PoundingSeverity::SafeClearance if contact.venturi_channel => theme.accent_blue,
                PoundingSeverity::SafeClearance => continue,
            };
            clip_painter.line_segment([pa, pb], Stroke::new(2.0, col));
            clip_painter.circle_filled(pa, 3.0, col);
            clip_painter.circle_filled(pb, 3.0, col);
            let mid = Pos2::new(0.5 * (pa.x + pb.x), 0.5 * (pa.y + pb.y) - 6.0);
            let tag = match contact.severity {
                PoundingSeverity::PoundingRisk => format!(
                    "Gap {:.2}m < {:.2}m [POUNDING]",
                    contact.clear_gap_m, contact.required_gap_m
                ),
                PoundingSeverity::TightBuffer => {
                    format!("Gap {:.2}m [Tight]", contact.clear_gap_m)
                }
                PoundingSeverity::SafeClearance => {
                    format!("Wind Venturi ({:.1}m)", contact.clear_gap_m)
                }
            };
            clip_painter.text(
                mid,
                Align2::CENTER_BOTTOM,
                tag,
                egui::FontId::monospace(9.5),
                col,
            );
            if contact.inside_collapse_fall_zone {
                if let Some((nc, _, true)) =
                    xf.project_lat_lon_elev(contact.neighbor_lat, contact.neighbor_lon, elev + contact.neighbor_height_m * 0.5)
                {
                    clip_painter.text(
                        nc,
                        Align2::CENTER_CENTER,
                        "FALL ZONE",
                        egui::FontId::monospace(9.0),
                        theme.accent_red,
                    );
                }
            }
        }
    }

    /// Screen-space click picking against LOD 0 regional clusters or LOD 1/2 3D neighbor buildings.
    pub fn pick_world_building_or_cluster_at_screen(
        &self,
        pos: Pos2,
        rect: Rect,
    ) -> Option<WorldPickOutcome> {
        if self.world_index.buildings.is_empty() || !rect.contains(pos) {
            return None;
        }
        let xf = self.last_globe_xform.as_ref()?;
        let (min_lat, max_lat, min_lon, max_lon) = self.visible_geo_bbox(xf);

        if self.world_map.zoom_level < 8.5 {
            let clusters = self.world_index.query_clusters(
                min_lat,
                max_lat,
                min_lon,
                max_lon,
                self.world_map.zoom_level,
            );
            for cl in clusters {
                if let Some((sp, _, true)) = xf.project_lat_lon_elev(cl.lat, cl.lon, 0.0) {
                    if pos.distance(sp) <= 20.0 {
                        if cl.count == 1 {
                            return Some(WorldPickOutcome::HydrateBuilding(cl.primary_rel_path));
                        } else {
                            return Some(WorldPickOutcome::FlyToCluster {
                                lat: cl.lat,
                                lon: cl.lon,
                                label: cl.label,
                            });
                        }
                    }
                }
            }
            return None;
        }

        let lift_scale = if self.world_map.zoom_level < 14.5 { 6.0 } else { 1.0 };
        let point_in_poly = |pt: Pos2, poly: &[Pos2]| -> bool {
            if poly.len() < 3 {
                return false;
            }
            let mut inside = false;
            let mut j = poly.len() - 1;
            for i in 0..poly.len() {
                let pi = poly[i];
                let pj = poly[j];
                if ((pi.y > pt.y) != (pj.y > pt.y))
                    && (pt.x < (pj.x - pi.x) * (pt.y - pi.y) / ((pj.y - pi.y) + 1e-6) + pi.x)
                {
                    inside = !inside;
                }
                j = i;
            }
            inside
        };

        let visible = self.world_index.query_visible_indices(
            min_lat,
            max_lat,
            min_lon,
            max_lon,
            xf.center_lat_deg,
            xf.center_lon_deg,
            450,
        );
        for idx in visible {
            let b = &self.world_index.buildings[idx];
            if self.active_world_project_rel_path.as_deref() == Some(b.rel_path.as_str()) {
                continue;
            }
            let hull_mm = b.rotated_footprint_world_mm();
            let mut base_pts = Vec::with_capacity(hull_mm.len());
            let mut roof_pts = Vec::with_capacity(hull_mm.len());
            let base_elev = self.local_site_relief_m(b.lat, b.lon);
            let top_elev = base_elev + b.total_height_m * lift_scale;
            let mut ok = true;
            for pt_mm in &hull_mm {
                let ll = SlippyMapState::world_mm_to_lat_lon((b.lat, b.lon), *pt_mm);
                if let (Some((bp, _, true)), Some((tp, _, true))) = (
                    xf.project_lat_lon_elev(ll.0, ll.1, base_elev),
                    xf.project_lat_lon_elev(ll.0, ll.1, top_elev),
                ) {
                    base_pts.push(bp);
                    roof_pts.push(tp);
                } else {
                    ok = false;
                    break;
                }
            }
            if !ok || roof_pts.len() < 3 {
                continue;
            }
            if point_in_poly(pos, &roof_pts) {
                return Some(WorldPickOutcome::HydrateBuilding(b.rel_path.clone()));
            }
            for i in 0..base_pts.len() {
                let ni = (i + 1) % base_pts.len();
                if point_in_poly(pos, &[base_pts[i], base_pts[ni], roof_pts[ni], roof_pts[i]]) {
                    return Some(WorldPickOutcome::HydrateBuilding(b.rel_path.clone()));
                }
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_domain_overlay(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &BimProject,
        members: &[StructuralMember],
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        if bim_project.is_empty() && members.is_empty() && self.entities.is_empty() {
            return;
        }
        let clip_painter = painter.with_clip_rect(rect);

        match active_pillar {
            NavigationPillar::MyLibrary => {
                // Graph topology links connecting structural column nodes
                let pts: Vec<Pos2> = bim_project
                    .columns
                    .iter()
                    .take(12)
                    .map(|c| to_screen([c.pos.0, c.pos.1]))
                    .collect();
                for pair in pts.windows(2) {
                    clip_painter.line_segment(
                        [pair[0], pair[1]],
                        Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.45)),
                    );
                }
            }
            NavigationPillar::AiAgent => {
                // AI focus nodes highlighting primary bays
                for &(x, y, tag) in &[
                    (3000.0, 2250.0, "AI-OPT Bay A1"),
                    (9000.0, 6750.0, "AI-OPT Bay B2"),
                ] {
                    let p = to_screen([x, y]);
                    clip_painter.circle_stroke(p, 22.0, Stroke::new(1.4, theme.accent_blue));
                    clip_painter.text(
                        p + Vec2::new(0.0, -28.0),
                        Align2::CENTER_BOTTOM,
                        tag,
                        egui::FontId::monospace(9.5),
                        theme.accent_blue,
                    );
                }
            }
            NavigationPillar::ResearchConcept => {
                // Site boundary setback & geodetic CRS datum box
                let p0 = to_screen([-1000.0, -1000.0]);
                let p1 = to_screen([13000.0, 10000.0]);
                clip_painter.rect_stroke(
                    Rect::from_two_pos(p0, p1),
                    CornerRadius::ZERO,
                    Stroke::new(1.5, theme.accent_orange),
                    StrokeKind::Inside,
                );
                clip_painter.text(
                    Pos2::new(p0.x.min(p1.x) + 8.0, p0.y.min(p1.y) + 8.0),
                    Align2::LEFT_TOP,
                    "WGS84 / UTM Site Boundary • Seismic SDC-D",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::ParametricDED => {
                let p0 = to_screen([0.0, -500.0]);
                let p1 = to_screen([6000.0, -500.0]);
                clip_painter.line_segment([p0, p1], Stroke::new(1.6, theme.accent_orange));
                clip_painter.text(
                    Pos2::new((p0.x + p1.x) * 0.5, p0.y + 12.0),
                    Align2::CENTER_TOP,
                    "Bay X1 = 6000 mm (Parametric)",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::GeometryAssembly => {
                let p = to_screen([12000.0, 9000.0]);
                clip_painter.text(
                    p + Vec2::new(8.0, -10.0),
                    Align2::LEFT_BOTTOM,
                    format!(
                        "LOD 350 • {} Walls • {} Cols • {} Beams",
                        bim_project.walls.len(),
                        bim_project.columns.len(),
                        bim_project.runs.len()
                    ),
                    egui::FontId::monospace(9.5),
                    theme.accent_green,
                );
            }
            NavigationPillar::AnalysisSimulation => {
                let avg_ari = if members.is_empty() {
                    0.942
                } else {
                    members.iter().map(|m| m.ari_score).sum::<f64>() / members.len() as f64
                };
                for col in &bim_project.columns {
                    let cp = to_screen([col.pos.0, col.pos.1]);
                    clip_painter.circle_stroke(
                        cp,
                        12.0,
                        Stroke::new(1.4, theme.accent_green.gamma_multiply(0.8)),
                    );
                }
                let p = to_screen([6000.0, 9500.0]);
                clip_painter.text(
                    p,
                    Align2::CENTER_BOTTOM,
                    format!("FEA Overlay • Mean ARI {:.3} • SVD Guard Active", avg_ari),
                    egui::FontId::monospace(10.0),
                    theme.accent_green,
                );
            }
            NavigationPillar::TimeScheduling => {
                for (idx, (x, y, label)) in [
                    (3000.0, 2250.0, "4D Phase 1 (W1-W4)"),
                    (9000.0, 2250.0, "4D Phase 2 (W5-W8)"),
                    (3000.0, 6750.0, "4D Phase 3 (W9-W12)"),
                    (9000.0, 6750.0, "4D Phase 4 (W13-W16)"),
                ]
                .into_iter()
                .enumerate()
                {
                    let c = to_screen([x, y]);
                    let col = if idx < 2 {
                        theme.accent_green
                    } else {
                        theme.accent_blue
                    };
                    clip_painter.circle_stroke(c, 16.0, Stroke::new(1.5, col));
                    clip_painter.text(
                        c + Vec2::new(0.0, 20.0),
                        Align2::CENTER_TOP,
                        label,
                        egui::FontId::monospace(9.0),
                        col,
                    );
                }
            }
            NavigationPillar::CostBudgeting => {
                let p0 = to_screen([0.0, 0.0]);
                let p1 = to_screen([6000.0, 4500.0]);
                clip_painter.rect_filled(
                    Rect::from_two_pos(p0, p1),
                    CornerRadius::ZERO,
                    theme.accent_orange.gamma_multiply(0.10),
                );
                clip_painter.text(
                    Rect::from_two_pos(p0, p1).center(),
                    Align2::CENTER_CENTER,
                    "5D High-Cost Core Zone (42% BoQ)",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::SustainabilityEnergy => {
                // 6D Solar Heliodon Arc
                let center = to_screen([6000.0, 4500.0]);
                let edge = to_screen([12500.0, 4500.0]);
                let rad = (edge.x - center.x).abs().max(40.0);
                clip_painter.circle_stroke(
                    center,
                    rad,
                    Stroke::new(1.2, theme.accent_orange.gamma_multiply(0.65)),
                );
                let sun = center + Vec2::new(rad * 0.72, -rad * 0.68);
                clip_painter.circle_filled(sun, 7.0, theme.accent_orange);
                clip_painter.line_segment(
                    [sun, center],
                    Stroke::new(1.2, theme.accent_orange.gamma_multiply(0.8)),
                );
                clip_painter.text(
                    sun + Vec2::new(10.0, -4.0),
                    Align2::LEFT_CENTER,
                    "Heliodon Azimuth 52° • OTTV 31 W/m²",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::FacilityManagement => {
                for (x, y, tag) in [
                    (1500.0, 4500.0, "SHM-01"),
                    (6000.0, 4500.0, "SHM-02"),
                    (10500.0, 4500.0, "MEP Riser"),
                ] {
                    let p = to_screen([x, y]);
                    clip_painter.circle_filled(p, 5.0, theme.accent_green);
                    clip_painter.text(
                        p + Vec2::new(8.0, -8.0),
                        Align2::LEFT_BOTTOM,
                        tag,
                        egui::FontId::monospace(9.5),
                        theme.accent_green,
                    );
                }
            }
            NavigationPillar::SafetyHealth => {
                // 8D Tower Crane Sweep Radius & Boom
                let mast = to_screen([6000.0, 10200.0]);
                let tip = to_screen([12800.0, 10200.0]);
                let rad = (tip.x - mast.x).abs().max(40.0);
                clip_painter.circle_stroke(
                    mast,
                    rad,
                    Stroke::new(1.6, theme.accent_orange.gamma_multiply(0.8)),
                );
                let boom_end = mast + Vec2::new(rad * 0.7, rad * 0.68);
                clip_painter.line_segment([mast, boom_end], Stroke::new(2.6, theme.accent_red));
                clip_painter.circle_filled(mast, 5.5, theme.accent_red);
                clip_painter.text(
                    mast + Vec2::new(10.0, -10.0),
                    Align2::LEFT_BOTTOM,
                    "TC-01 Jib Sweep Radius (45m)",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::LeanConstruction => {
                let l0 = to_screen([-1400.0, 0.0]);
                let l1 = to_screen([-200.0, 4500.0]);
                let laydown = Rect::from_two_pos(l0, l1);
                clip_painter.rect_stroke(
                    laydown,
                    CornerRadius::same(4),
                    Stroke::new(1.5, theme.accent_green),
                    StrokeKind::Inside,
                );
                clip_painter.text(
                    laydown.center(),
                    Align2::CENTER_CENTER,
                    "JIT Laydown",
                    egui::FontId::monospace(9.0),
                    theme.accent_green,
                );
            }
            NavigationPillar::IndustrializedAutomation => {
                let p0 = to_screen([0.0, 0.0]);
                let p1 = to_screen([6000.0, 4500.0]);
                clip_painter.rect_stroke(
                    Rect::from_two_pos(p0, p1),
                    CornerRadius::ZERO,
                    Stroke::new(1.6, theme.accent_blue),
                    StrokeKind::Inside,
                );
                clip_painter.text(
                    Rect::from_two_pos(p0, p1).center(),
                    Align2::CENTER_CENTER,
                    "10D DfMA Pod Module M-01",
                    egui::FontId::monospace(9.5),
                    theme.accent_blue,
                );
            }
            NavigationPillar::Prediction => {
                let p = to_screen([6000.0, 4500.0]);
                clip_painter.circle_stroke(p, 28.0, Stroke::new(1.6, theme.accent_red));
                clip_painter.circle_stroke(p, 42.0, Stroke::new(1.0, theme.accent_orange));
                clip_painter.text(
                    p + Vec2::new(0.0, -48.0),
                    Align2::CENTER_BOTTOM,
                    "Drift Hotspot Watch • Pf < 1e-4",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::ReportPublishing => {
                let margin_rect = rect.shrink(14.0);
                clip_painter.rect_stroke(
                    margin_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.4, theme.text_muted),
                    StrokeKind::Inside,
                );
                clip_painter.text(
                    Pos2::new(margin_rect.right() - 8.0, margin_rect.bottom() - 8.0),
                    Align2::RIGHT_BOTTOM,
                    "ISO A3 SHEET BORDER • EIGENGRAPH DED",
                    egui::FontId::monospace(9.0),
                    theme.text_muted,
                );
            }
        }
    }

    /// Paint synchronized sketch/annotation entities onto any compatible 2D world canvas
    /// (used to sync Universal Canvas markup inside ParametricView's single 2D canvas).
    pub fn paint_entities_world(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        self.paint_entities(painter, rect, theme, active_pillar, to_screen);
    }

    fn paint_entities(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        let clip_painter = painter.with_clip_rect(rect);
        let active_badge = active_pillar.badge();
        let is_screen_valid =
            |p: Pos2| p.x.is_finite() && p.y.is_finite() && p.x > -50_000.0 && p.y > -50_000.0;

        let globe_xform = if self.globe_mode {
            self.last_globe_xform
                .as_ref()
                .filter(|x| x.rect == rect)
                .cloned()
        } else {
            None
        };

        for ent in &self.entities {
            if !self.sync_all_layers && ent.pillar_badge != active_badge {
                continue;
            }
            let base_col = Color32::from_rgb(ent.color_rgb[0], ent.color_rgb[1], ent.color_rgb[2]);
            let is_selected = self.is_entity_selected(ent.id);
            let col = if is_selected {
                theme.accent_orange
            } else if ent.pillar_badge == active_badge {
                base_col
            } else {
                base_col.gamma_multiply(0.82)
            };
            let stroke = Stroke::new(
                if is_selected {
                    ent.stroke_width + 1.4
                } else {
                    ent.stroke_width
                },
                col,
            );

            match &ent.kind {
                CanvasShapeKind::Freehand { points_mm } => {
                    let spts: Vec<Pos2> = points_mm.iter().map(|p| to_screen(*p)).collect();
                    for w in spts.windows(2) {
                        if is_screen_valid(w[0]) && is_screen_valid(w[1]) {
                            clip_painter.line_segment([w[0], w[1]], stroke);
                        }
                    }
                }
                CanvasShapeKind::Line { start_mm, end_mm } => {
                    let len_mm = (end_mm[0] - start_mm[0]).hypot(end_mm[1] - start_mm[1]);
                    if let Some(xf) = globe_xform.as_ref().filter(|_| len_mm > 5_000_000.0) {
                        let a = SlippyMapState::world_mm_to_lat_lon(
                            self.ground_origin_lat_lon,
                            *start_mm,
                        );
                        let b = SlippyMapState::world_mm_to_lat_lon(
                            self.ground_origin_lat_lon,
                            *end_mm,
                        );
                        let arc = crate::core::gis::great_circle_arc(a, b, 16);
                        let proj: Vec<Option<Pos2>> = arc
                            .iter()
                            .map(|&(la, lo)| {
                                xf.project_lat_lon_elev(la, lo, 0.0)
                                    .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
                            })
                            .collect();
                        for pair in proj.windows(2) {
                            if let (Some(p0), Some(p1)) = (pair[0], pair[1]) {
                                clip_painter.line_segment([p0, p1], stroke);
                            }
                        }
                    } else {
                        let p0 = to_screen(*start_mm);
                        let p1 = to_screen(*end_mm);
                        if is_screen_valid(p0) && is_screen_valid(p1) {
                            clip_painter.line_segment([p0, p1], stroke);
                        }
                    }
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    let p0 = to_screen(*min_mm);
                    let p1 = to_screen(*max_mm);
                    if is_screen_valid(p0) && is_screen_valid(p1) {
                        let r = Rect::from_two_pos(p0, p1);
                        clip_painter.rect_filled(r, CornerRadius::ZERO, col.gamma_multiply(0.08));
                        clip_painter.rect_stroke(r, CornerRadius::ZERO, stroke, StrokeKind::Inside);
                    }
                }
                CanvasShapeKind::Circle {
                    center_mm,
                    radius_mm,
                } => {
                    let c = to_screen(*center_mm);
                    if is_screen_valid(c) {
                        let edge = to_screen([center_mm[0] + radius_mm, center_mm[1]]);
                        let r_px = if is_screen_valid(edge) {
                            (edge - c).length().max(2.0)
                        } else {
                            (self.world_map.pixels_per_mm_at_origin(self.ground_origin_lat_lon)
                                * (*radius_mm as f32))
                                .max(2.0)
                        };
                        clip_painter.circle_stroke(c, r_px, stroke);
                    }
                }
                CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    let s0 = to_screen(*start_mm);
                    let s1 = to_screen(*end_mm);
                    let len_mm = (end_mm[0] - start_mm[0]).hypot(end_mm[1] - start_mm[1]);
                    if let Some(xf) = globe_xform.as_ref().filter(|_| len_mm > 5_000_000.0) {
                        let a = SlippyMapState::world_mm_to_lat_lon(
                            self.ground_origin_lat_lon,
                            *start_mm,
                        );
                        let b = SlippyMapState::world_mm_to_lat_lon(
                            self.ground_origin_lat_lon,
                            *end_mm,
                        );
                        let arc = crate::core::gis::great_circle_arc(a, b, 16);
                        let proj: Vec<Option<Pos2>> = arc
                            .iter()
                            .map(|&(la, lo)| {
                                xf.project_lat_lon_elev(la, lo, 0.0)
                                    .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
                            })
                            .collect();
                        for pair in proj.windows(2) {
                            if let (Some(p0), Some(p1)) = (pair[0], pair[1]) {
                                clip_painter.line_segment([p0, p1], stroke);
                            }
                        }
                        if is_screen_valid(s0) {
                            clip_painter.circle_filled(s0, 3.0, col);
                        }
                        if is_screen_valid(s1) {
                            clip_painter.circle_filled(s1, 3.0, col);
                        }
                        if let Some(Some(mid)) = proj.get(proj.len() / 2) {
                            let label = if len_mm >= 1_000_000.0 {
                                format!("[{}] {:.2} km", ent.pillar_badge, len_mm / 1_000_000.0)
                            } else {
                                format!("[{}] {:.0} mm", ent.pillar_badge, len_mm)
                            };
                            clip_painter.text(
                                *mid + Vec2::new(0.0, -10.0),
                                Align2::CENTER_BOTTOM,
                                label,
                                egui::FontId::monospace(10.0),
                                col,
                            );
                        }
                    } else if is_screen_valid(s0) && is_screen_valid(s1) {
                        clip_painter.line_segment([s0, s1], stroke);
                        clip_painter.circle_filled(s0, 3.0, col);
                        clip_painter.circle_filled(s1, 3.0, col);
                        let mid = Pos2::new((s0.x + s1.x) * 0.5, (s0.y + s1.y) * 0.5 - 10.0);
                        clip_painter.text(
                            mid,
                            Align2::CENTER_BOTTOM,
                            format!("[{}] {:.0} mm", ent.pillar_badge, len_mm),
                            egui::FontId::monospace(10.0),
                            col,
                        );
                    }
                }
                CanvasShapeKind::Callout { pos_mm, text } => {
                    let p = to_screen(*pos_mm);
                    if !is_screen_valid(p) {
                        continue;
                    }
                    clip_painter.circle_filled(p, 4.0, col);
                    let box_pos = p + Vec2::new(8.0, -18.0);
                    let bg = if theme.is_dark() {
                        Color32::from_rgba_premultiplied(20, 24, 34, 225)
                    } else {
                        Color32::from_rgba_premultiplied(255, 255, 255, 235)
                    };
                    let tag = format!("[{}] {}", ent.pillar_badge, text);
                    let approx_w = (tag.len() as f32 * 6.3 + 14.0).clamp(80.0, 280.0);
                    let tag_rect = Rect::from_min_size(box_pos, Vec2::new(approx_w, 20.0));
                    clip_painter.rect_filled(tag_rect, CornerRadius::same(5), bg);
                    clip_painter.rect_stroke(
                        tag_rect,
                        CornerRadius::same(5),
                        Stroke::new(1.0, col),
                        StrokeKind::Inside,
                    );
                    clip_painter.text(
                        tag_rect.center(),
                        Align2::CENTER_CENTER,
                        tag,
                        egui::FontId::monospace(9.5),
                        theme.text_primary,
                    );
                }
            }
        }
    }

    /// Paints live rubber-band sketch/dimension/rectangle/circle preview using any world-to-screen mapping.
    pub fn paint_live_preview_world(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        self.paint_live_preview(painter, rect, theme, to_screen);
    }

    fn paint_live_preview(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        let clip_painter = painter.with_clip_rect(rect);
        let preview_col = self.active_color();
        let stroke = Stroke::new(self.stroke_width, preview_col);

        if self.active_freehand_mm.len() >= 2 {
            let spts: Vec<Pos2> = self
                .active_freehand_mm
                .iter()
                .map(|p| to_screen(*p))
                .collect();
            for w in spts.windows(2) {
                clip_painter.line_segment([w[0], w[1]], stroke);
            }
        }

        if let Some(start) = self.pending_start_mm {
            let end = if matches!(
                self.active_tool,
                UniversalCanvasTool::Line | UniversalCanvasTool::Dimension
            ) {
                self.apply_ortho(start, self.cursor_world_mm)
            } else {
                self.cursor_world_mm
            };
            let s0 = to_screen(start);
            let s1 = to_screen(end);
            match self.active_tool {
                UniversalCanvasTool::Line | UniversalCanvasTool::Dimension => {
                    clip_painter.line_segment([s0, s1], stroke);
                }
                UniversalCanvasTool::Rectangle => {
                    clip_painter.rect_stroke(
                        Rect::from_two_pos(s0, s1),
                        CornerRadius::ZERO,
                        stroke,
                        StrokeKind::Inside,
                    );
                }
                UniversalCanvasTool::Circle => {
                    let r_px = (s1 - s0).length().max(2.0);
                    clip_painter.circle_stroke(s0, r_px, stroke);
                }
                _ => {}
            }
            let dist = (end[0] - start[0]).hypot(end[1] - start[1]);
            clip_painter.text(
                s1 + Vec2::new(10.0, -10.0),
                Align2::LEFT_BOTTOM,
                format!("{:.0} mm", dist),
                egui::FontId::monospace(10.0),
                theme.accent_orange,
            );
        }

        // Window (blue L->R) vs Crossing (green R->L) Marquee Box Preview
        if let (Some(anchor), Some(cur)) = (self.marquee_anchor_mm, self.marquee_current_mm) {
            let s0 = to_screen(anchor);
            let s1 = to_screen(cur);
            let mrect = Rect::from_two_pos(s0, s1);
            let is_window = cur[0] >= anchor[0];
            let (mcol, lbl) = if is_window {
                (theme.accent_blue, "Window (Enclosed)")
            } else {
                (theme.accent_green, "Crossing (Touch)")
            };
            clip_painter.rect_filled(mrect, CornerRadius::ZERO, mcol.gamma_multiply(0.14));
            clip_painter.rect_stroke(mrect, CornerRadius::ZERO, Stroke::new(1.5, mcol), StrokeKind::Inside);
            clip_painter.text(
                mrect.left_top() + Vec2::new(4.0, -4.0),
                Align2::LEFT_BOTTOM,
                lbl,
                egui::FontId::monospace(9.5),
                mcol,
            );
        }

        // OSNAP Active Reticle Glyph (outside idle Grid)
        if self.osnap_enabled
            && !matches!(self.last_osnap_kind, OsnapKind::None | OsnapKind::Grid)
        {
            let sp = to_screen(self.cursor_world_mm);
            if rect.expand(10.0).contains(sp) {
                let col = match self.last_osnap_kind {
                    OsnapKind::Endpoint => theme.accent_green,
                    OsnapKind::Midpoint => theme.accent_blue,
                    OsnapKind::Perpendicular => theme.accent_orange,
                    OsnapKind::Intersection => theme.accent_red,
                    OsnapKind::None | OsnapKind::Grid => theme.text_muted,
                };
                let st = Stroke::new(1.6, col);
                match self.last_osnap_kind {
                    OsnapKind::Endpoint => {
                        clip_painter.rect_stroke(
                            Rect::from_center_size(sp, Vec2::splat(9.0)),
                            CornerRadius::ZERO,
                            st,
                            StrokeKind::Middle,
                        );
                    }
                    OsnapKind::Midpoint => {
                        let tri = vec![
                            sp + Vec2::new(0.0, -5.5),
                            sp + Vec2::new(5.5, 4.5),
                            sp + Vec2::new(-5.5, 4.5),
                        ];
                        clip_painter.add(egui::Shape::closed_line(tri, st));
                    }
                    OsnapKind::Perpendicular => {
                        clip_painter.line_segment([sp + Vec2::new(-5.0, -5.0), sp + Vec2::new(-5.0, 5.0)], st);
                        clip_painter.line_segment([sp + Vec2::new(-5.0, 5.0), sp + Vec2::new(5.0, 5.0)], st);
                    }
                    OsnapKind::Intersection => {
                        clip_painter.line_segment([sp + Vec2::new(-5.0, -5.0), sp + Vec2::new(5.0, 5.0)], st);
                        clip_painter.line_segment([sp + Vec2::new(-5.0, 5.0), sp + Vec2::new(5.0, -5.0)], st);
                    }
                    OsnapKind::None | OsnapKind::Grid => {}
                }
                clip_painter.text(
                    sp + Vec2::new(9.0, -7.0),
                    Align2::LEFT_BOTTOM,
                    self.last_osnap_kind.short_tag(),
                    egui::FontId::monospace(9.0),
                    col,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_floating_inspector_panel(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
        workspace_settings: WorkspaceCanvasSettings,
        is_overlay: bool,
    ) {
        Self::floating_panel_frame(theme, is_overlay).show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::vertical()
                .id_salt("universal_canvas_inspector_scroll")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icons::icon_label(
                            ui,
                            active_pillar.app_icon(),
                            14.0,
                            theme.accent_blue,
                            "Canvas Inspector",
                            12.5,
                            theme.text_primary,
                            true,
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icons::icon_only_button(
                                ui,
                                AppIcon::Close,
                                theme.text_secondary,
                                Color32::TRANSPARENT,
                                Stroke::NONE,
                                CornerRadius::same(4),
                                Vec2::splat(20.0),
                                "Close inspector panel",
                            )
                            .clicked()
                            {
                                self.inspector_open = false;
                            }
                        });
                    });
                    ui.separator();
                    ui.add_space(4.0);

                    self.render_canvas_sync_controls(
                        ui,
                        theme,
                        active_pillar,
                        bim_project,
                        members,
                        variables,
                        workspace_settings,
                        true,
                    );
                });
        });
    }

    /// Shared Canvas Sync, ink stroke, layer cleanup, and 1-file export controls reusable inside
    /// floating HUD inspectors across Universal Canvas, Parametric, and Geometry.
    #[allow(clippy::too_many_arguments)]
    pub fn render_canvas_sync_controls(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
        workspace_settings: WorkspaceCanvasSettings,
        show_domain_overlay_toggle: bool,
    ) {
        ui.checkbox(&mut self.sync_all_layers, "Sync drawings across all 15 modules");
        ui.checkbox(&mut self.show_bim_base_layer, "Show synchronized BIM base plan");
        if show_domain_overlay_toggle {
            ui.checkbox(&mut self.show_domain_overlay, "Show active module overlay");
        }
        ui.add_space(6.0);

        ui.label(
            RichText::new("Callout Note Draft:")
                .size(11.0)
                .color(theme.text_secondary),
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.callout_draft)
                .desired_width(ui.available_width().max(120.0)),
        );

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Stroke:").size(11.0).color(theme.text_secondary));
            ui.add(egui::Slider::new(&mut self.stroke_width, 1.0..=6.0));
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Canvas H:").size(11.0).color(theme.text_secondary));
            ui.add(egui::Slider::new(&mut self.canvas_height_ratio, 0.45..=1.15));
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "Layer Count: {} active ({} total synced)",
                self.count_for_pillar(active_pillar),
                self.entities.len()
            ))
            .size(10.5)
            .color(theme.text_muted),
        );

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if icons::icon_button(
                ui,
                AppIcon::Eraser,
                "Clear Active Layer",
                10.5,
                theme.text_primary,
                theme.bg_sidebar,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(5),
                Vec2::new(0.0, 22.0),
            )
            .clicked()
            {
                self.clear_entities(Some(active_pillar));
            }
            if icons::icon_button(
                ui,
                AppIcon::Trash,
                "Clear All",
                10.5,
                theme.accent_red,
                theme.bg_sidebar,
                Stroke::new(1.0, theme.accent_red.gamma_multiply(0.45)),
                CornerRadius::same(5),
                Vec2::new(0.0, 22.0),
            )
            .clicked()
            {
                self.clear_entities(None);
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label(
            RichText::new("SINGLE-FILE & SPLIT EXPORT")
                .size(10.0)
                .strong()
                .color(theme.accent_blue),
        );
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if icons::icon_button(
                ui,
                AppIcon::SaveDisk,
                "Save 1-File (.eigen.json)",
                10.5,
                Color32::WHITE,
                theme.accent_green,
                Stroke::NONE,
                CornerRadius::same(5),
                Vec2::new(0.0, 24.0),
            )
            .clicked()
            {
                self.export_unified_file(bim_project, members, variables, workspace_settings);
            }
            if icons::icon_button(
                ui,
                AppIcon::UploadImport,
                "Import 1-File",
                10.5,
                theme.text_primary,
                theme.bg_sidebar,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(5),
                Vec2::new(0.0, 24.0),
            )
            .clicked()
            {
                self.import_json_open = true;
            }
            if icons::icon_button(
                ui,
                AppIcon::Palette,
                "Export SVG",
                10.5,
                theme.text_primary,
                theme.bg_sidebar,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(5),
                Vec2::new(0.0, 24.0),
            )
            .clicked()
            {
                let svg = self.export_svg(bim_project, active_pillar);
                match DiskExporter::write_export_file("EigenGraph_Unified_Canvas.svg", &svg) {
                    Ok(dest) => {
                        self.status_message = Some(format!("Exported SVG canvas -> {dest}"));
                    }
                    Err(err) => {
                        self.status_message = Some(format!("SVG export failed: {err}"));
                    }
                }
            }
            if icons::icon_button(
                ui,
                AppIcon::Folder,
                "Split Active Module",
                10.5,
                theme.text_primary,
                theme.bg_sidebar,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(5),
                Vec2::new(0.0, 24.0),
            )
            .on_hover_text("Export only this module's drawing layer separately")
            .clicked()
            {
                self.export_split_module_file(active_pillar);
            }
        });
    }

    fn render_status_footer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &BimProject,
    ) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(12, 6))
            .show(ui, |ui| {
                let inner_w = ui.available_width().max(120.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                let telem = self.world_map.telemetry();
                let cur_elev = self
                    .earth_world
                    .effective_elevation_m(self.cursor_lat_lon.0, self.cursor_lat_lon.1);
                let geo_badge = format!(
                    "[{}] {:.3}°, {:.3}°",
                    if self.globe_mode { "3D Earth" } else { "2D Map" },
                    self.cursor_lat_lon.0,
                    self.cursor_lat_lon.1,
                );
                let elev_txt = if self.globe_mode {
                    format!(
                        "Elev {:+.0}m • Tilt {:.0}° • z{:.1} ({:.1} m/px)",
                        cur_elev,
                        self.earth_world.camera.tilt_deg,
                        self.world_map.zoom_level,
                        telem.ground_resolution_m_px,
                    )
                } else {
                    format!(
                        "Elev {:+.0}m • z{:.1} ({:.1} m/px)",
                        cur_elev,
                        self.world_map.zoom_level,
                        telem.ground_resolution_m_px,
                    )
                };
                let coord_txt = format!(
                    "X: {:.0}  Y: {:.0} mm",
                    self.cursor_world_mm[0], self.cursor_world_mm[1],
                );
                let sel_tag = if self.selected_entity_ids.len() > 1 {
                    format!(" • Sel {} objs", self.selected_entity_ids.len())
                } else {
                    self.selected_entity_id
                        .map_or(String::new(), |id| format!(" • Sel #{id}"))
                };
                let osnap_tag = if !self.osnap_enabled {
                    "OFF".to_string()
                } else if matches!(self.last_osnap_kind, OsnapKind::None | OsnapKind::Grid) {
                    "ON".to_string()
                } else {
                    format!("ON [{}]", self.last_osnap_kind.short_tag())
                };
                let tool_txt = format!("Tool: {}{}", self.active_tool.short_label(), sel_tag);
                let snap_txt = format!(
                    "Ortho(F8):{} Snap(F3):{} OSNAP(F4):{}",
                    if self.ortho_mode { "ON" } else { "OFF" },
                    if self.snap_to_grid { "ON" } else { "OFF" },
                    osnap_tag,
                );
                let counts_txt = if !self.earth_world.cells.is_empty()
                    || !self.earth_world.megastructures.is_empty()
                {
                    format!(
                        "{} Draw ({} {}) • {} BIM • {} Earth / {} Mega",
                        self.entities.len(),
                        self.count_for_pillar(active_pillar),
                        active_pillar.badge(),
                        bim_project.walls.len()
                            + bim_project.columns.len()
                            + bim_project.runs.len(),
                        self.earth_world.cells.len(),
                        self.earth_world.megastructures.len(),
                    )
                } else {
                    format!(
                        "{} Draw ({} {}) • {} BIM",
                        self.entities.len(),
                        self.count_for_pillar(active_pillar),
                        active_pillar.badge(),
                        bim_project.walls.len()
                            + bim_project.columns.len()
                            + bim_project.runs.len()
                    )
                };
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        12.0,
                        theme.accent_blue,
                        geo_badge,
                        10.5,
                        theme.text_secondary,
                        false,
                    );
                    ui.label(
                        RichText::new(elev_txt)
                            .size(10.5)
                            .color(theme.text_secondary),
                    );
                    icons::icon_label(
                        ui,
                        AppIcon::Compass,
                        11.0,
                        theme.text_muted,
                        coord_txt,
                        10.5,
                        theme.text_secondary,
                        false,
                    );
                    icons::icon_label(
                        ui,
                        AppIcon::RulerSquare,
                        11.0,
                        if self.ortho_mode {
                            theme.accent_orange
                        } else {
                            theme.text_secondary
                        },
                        tool_txt,
                        10.5,
                        theme.text_secondary,
                        false,
                    );
                    ui.label(
                        RichText::new(snap_txt)
                            .size(10.5)
                            .color(theme.text_secondary),
                    );
                    icons::icon_label(
                        ui,
                        AppIcon::Layers,
                        11.0,
                        theme.accent_blue,
                        counts_txt,
                        10.5,
                        theme.text_secondary,
                        false,
                    );
                    if icons::icon_button(
                        ui,
                        AppIcon::ArrowDown,
                        format!("{} Workbench", active_pillar.clean_name()),
                        10.5,
                        theme.accent_blue,
                        theme.accent_blue.gamma_multiply(0.14),
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 22.0),
                    )
                    .on_hover_text("Scroll down to the detailed module workbench")
                    .clicked()
                    {
                        self.request_scroll_to_workbench = true;
                    }
                });
                ui.add(
                    egui::Label::new(
                        RichText::new("Keys: Space Freeze • G 3D Earth • [ ] Tilt • 1-9 Earth • L/R/C/D Draw • M/O/X Modify")
                            .size(10.0)
                            .color(theme.text_muted),
                    )
                    .wrap(),
                );
            });
    }

    /// Render the Unified Single-File Import Modal whenever `import_json_open` is true.
    /// Invoked from the root app loop so single-file import works seamlessly across all 15 modules,
    /// including single-canvas Parametric and Geometry modules.
    pub fn render_import_modal_if_open(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        bim_project: &mut BimProject,
        members: &mut Vec<StructuralMember>,
        variables: &mut Vec<ParametricVariable>,
    ) {
        if self.import_json_open {
            self.render_import_modal(ctx, theme, bim_project, members, variables);
        }
    }

    fn render_import_modal(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        bim_project: &mut BimProject,
        members: &mut Vec<StructuralMember>,
        variables: &mut Vec<ParametricVariable>,
    ) {
        let mut open = self.import_json_open;
        let mut close_now = false;
        egui::Window::new("Import Unified Workspace (.eigen.json)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_size(MacTheme::clamp_modal_size(ctx, Vec2::new(620.0, 420.0)))
            .max_size(MacTheme::modal_max_size(ctx))
            .frame(
                Frame::new()
                    .fill(theme.bg_card)
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(14)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "Paste the contents of a single-file EigenGraph_Unified_Workspace.eigen.json archive to verify its SHA-256 seal and restore all synchronized modules.",
                    )
                    .size(11.5)
                    .color(theme.text_secondary),
                );
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.import_json_buffer)
                                .desired_width(ui.available_width())
                                .desired_rows(12),
                        );
                    });
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::CheckCircle,
                        "Verify & Restore Workspace",
                        11.5,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 28.0),
                    )
                    .clicked()
                    {
                        match self.apply_unified_archive(
                            &self.import_json_buffer.clone(),
                            bim_project,
                            members,
                            variables,
                        ) {
                            Ok(settings) => {
                                self.pending_imported_settings = Some(settings);
                                close_now = true;
                            }
                            Err(err) => {
                                self.status_message = Some(format!("Import error: {err}"));
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close_now = true;
                    }
                });
            });
        self.import_json_open = open && !close_now;
    }
}

fn point_to_segment_dist(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let vx = b[0] - a[0];
    let vy = b[1] - a[1];
    let len_sq = vx * vx + vy * vy;
    if len_sq <= 1e-9 {
        return (p[0] - a[0]).hypot(p[1] - a[1]);
    }
    let t = (((p[0] - a[0]) * vx + (p[1] - a[1]) * vy) / len_sq).clamp(0.0, 1.0);
    let proj = [a[0] + t * vx, a[1] + t * vy];
    (p[0] - proj[0]).hypot(p[1] - proj[1])
}

fn segment_intersection_2d(
    p: [f64; 2],
    r_end: [f64; 2],
    q: [f64; 2],
    s_end: [f64; 2],
) -> Option<[f64; 2]> {
    let rx = r_end[0] - p[0];
    let ry = r_end[1] - p[1];
    let sx = s_end[0] - q[0];
    let sy = s_end[1] - q[1];
    let denom = rx * sy - ry * sx;
    if denom.abs() < 1e-6 {
        return None;
    }
    let qpx = q[0] - p[0];
    let qpy = q[1] - p[1];
    let t = (qpx * sy - qpy * sx) / denom;
    let u = (qpx * ry - qpy * rx) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some([p[0] + t * rx, p[1] + t * ry])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_settings() -> WorkspaceCanvasSettings {
        WorkspaceCanvasSettings {
            theme_mode: ThemeMode::ProDark,
            canvas_height_ratio: 0.74,
            freeze_by_default: true,
            show_grid: true,
            snap_to_grid: true,
            grid_spacing_mm: 500.0,
            show_bim_base_layer: true,
            show_domain_overlay: true,
            sync_all_layers: true,
            show_floating_inspector: false,
            show_ai_copilot: true,
            unit_system: "kN, mm, MPa (Structural Metric)".to_string(),
            design_standard: "SNI 1729 / AISC 360-16".to_string(),
            solver_threads: 8,
            enable_svd_guard: true,
            enable_anti_leakage: true,
        }
    }

    #[test]
    fn test_unified_single_file_archive_roundtrip_and_tamper_detection() {
        let canvas = UniversalCanvasState::default();
        assert!(canvas.is_frozen);
        assert!(canvas.entities.is_empty());
        assert!(canvas.world_map.zoom_level < 3.0);

        let bim = BimProject::sample_bimcraft_building();
        let members = vec![StructuralMember::default()];
        let vars = vec![ParametricVariable {
            name: "L".to_string(),
            expression: "24000".to_string(),
            evaluated_value: 24000.0,
            unit: "mm".to_string(),
        }];

        let archive = canvas.build_unified_archive(&bim, &members, &vars, sample_settings());
        let json = archive.to_pretty_json().expect("serialize archive");
        let verified = UnifiedWorkspaceArchive::from_verified_json(&json).expect("verify archive");
        assert_eq!(verified.canvas_entities.len(), 0);
        assert_eq!(verified.members.len(), 1);
        assert_eq!(verified.bim_project.walls.len(), bim.walls.len());

        // Tampering with JSON content must fail SHA-256 verification
        let tampered = json.replace("24000", "99999");
        assert!(UnifiedWorkspaceArchive::from_verified_json(&tampered).is_err());
    }

    #[test]
    fn test_canvas_sync_across_pillars_undo_and_svg_export() {
        let mut canvas = UniversalCanvasState::default();
        let initial_len = canvas.entities.len();
        canvas.push_undo();
        canvas.entities.push(CanvasEntity {
            id: 99,
            pillar_badge: NavigationPillar::SafetyHealth.badge().to_string(),
            pillar_name: NavigationPillar::SafetyHealth.clean_name().to_string(),
            kind: CanvasShapeKind::Circle {
                center_mm: [6000.0, 4500.0],
                radius_mm: 3000.0,
            },
            color_rgb: [245, 158, 11],
            stroke_width: 2.5,
        });
        assert_eq!(canvas.count_for_pillar(NavigationPillar::SafetyHealth), 1);
        assert_eq!(canvas.entities.len(), initial_len + 1);

        let bim = BimProject::sample_bimcraft_building();
        let svg = canvas.export_svg(&bim, NavigationPillar::SafetyHealth);
        assert!(svg.contains("<circle"));

        assert!(canvas.undo());
        assert_eq!(canvas.entities.len(), initial_len);
        assert!(canvas.redo());
        assert_eq!(canvas.entities.len(), initial_len + 1);
    }

    #[test]
    fn test_canvas_ortho_translate_offset_mirror_and_delete() {
        let mut canvas = UniversalCanvasState::default();
        canvas.ortho_mode = true;
        assert_eq!(
            canvas.apply_ortho([1000.0, 1000.0], [4500.0, 1200.0]),
            [4500.0, 1000.0]
        );
        assert_eq!(
            canvas.apply_ortho([1000.0, 1000.0], [1150.0, 3800.0]),
            [1000.0, 3800.0]
        );

        canvas.entities.push(CanvasEntity {
            id: 1,
            pillar_badge: "3D".to_string(),
            pillar_name: "Geometry".to_string(),
            kind: CanvasShapeKind::Line {
                start_mm: [0.0, 0.0],
                end_mm: [3000.0, 0.0],
            },
            color_rgb: [56, 189, 248],
            stroke_width: 2.0,
        });
        canvas.next_entity_id = 2;
        canvas.selected_entity_id = Some(1);

        assert!(canvas.translate_selected_or_last(1000.0, 500.0));
        match &canvas.entities[0].kind {
            CanvasShapeKind::Line { start_mm, end_mm } => {
                assert_eq!(*start_mm, [1000.0, 500.0]);
                assert_eq!(*end_mm, [4000.0, 500.0]);
            }
            _ => panic!("expected line"),
        }

        assert!(canvas.offset_selected_or_last(500.0));
        assert_eq!(canvas.entities.len(), 2);
        assert!(canvas.mirror_selected_or_last(6000.0));
        assert_eq!(canvas.entities.len(), 3);
        assert!(canvas.delete_selected_or_last());
        assert_eq!(canvas.entities.len(), 2);
        assert!(canvas.undo());
        assert_eq!(canvas.entities.len(), 3);
    }

    #[test]
    fn test_canvas_precision_polar_relative_duplicate_and_rotate() {
        let mut canvas = UniversalCanvasState::default();
        let mut bim = BimProject::sample_bimcraft_building();

        let (dx, dy) = UniversalCanvasState::parse_cad_delta_expr("@5000<0").expect("polar 0 deg");
        assert!((dx - 5000.0).abs() < 1e-6);
        assert!(dy.abs() < 1e-6);

        let (dx2, dy2) =
            UniversalCanvasState::parse_cad_delta_expr("@1200,-600").expect("relative coords");
        assert_eq!((dx2, dy2), (1200.0, -600.0));

        let msg = canvas.execute_precision_command(
            "@4000<0",
            NavigationPillar::ResearchConcept,
            &mut bim,
        );
        assert!(msg.contains("Drew relative segment"));
        assert_eq!(canvas.entities.len(), 1);

        assert!(canvas.rotate_selected_or_last(90.0));
        match &canvas.entities[0].kind {
            CanvasShapeKind::Line { start_mm, end_mm } => {
                assert!((end_mm[1] - start_mm[1] - 4000.0).abs() < 1e-4);
            }
            _ => panic!("expected rotated line"),
        }

        assert!(canvas.duplicate_selected_or_last(1500.0, 0.0));
        assert_eq!(canvas.entities.len(), 2);
    }

    #[test]
    fn test_canvas_osnap_marquee_grid_array_and_promote_to_bim() {
        let mut canvas = UniversalCanvasState::default();
        let mut bim = BimProject::empty();

        // 1. Structural Grid Generation & OSNAP Intersection + Endpoint
        let created = canvas.generate_structural_grid(
            3,
            2,
            6000.0,
            5000.0,
            NavigationPillar::ParametricDED,
            [0.0, 0.0],
        );
        assert_eq!(created, 5); // 3 vertical + 2 horizontal axes

        let (ix0_pt, ix0_kind) =
            canvas.snap_point_with_osnap([40.0, 35.0], None, None, 200.0);
        assert!((ix0_pt[0]).abs() < 1e-3 && (ix0_pt[1]).abs() < 1e-3);
        assert_eq!(ix0_kind, OsnapKind::Intersection);

        let (mid_pt, mid_kind) =
            canvas.snap_point_with_osnap([25.0, 2520.0], None, None, 200.0);
        assert!(mid_pt[0].abs() < 1e-3 && (mid_pt[1] - 2500.0).abs() < 1e-3);
        assert_eq!(mid_kind, OsnapKind::Midpoint);

        let (perp_pt, perp_kind) = canvas.snap_point_with_osnap(
            [35.0, 1210.0],
            Some([3000.0, 1200.0]),
            None,
            200.0,
        );
        assert!(perp_pt[0].abs() < 1e-3 && (perp_pt[1] - 1200.0).abs() < 1e-3);
        assert_eq!(perp_kind, OsnapKind::Perpendicular);

        let (ix_pt, ix_kind) =
            canvas.snap_point_with_osnap([6030.0, 4985.0], None, None, 200.0);
        assert!((ix_pt[0] - 6000.0).abs() < 1e-3 && (ix_pt[1] - 5000.0).abs() < 1e-3);
        assert!(matches!(ix_kind, OsnapKind::Endpoint | OsnapKind::Intersection));

        // Standalone segment endpoint verifies pure Endpoint OSNAP without crossing axes
        canvas.entities.push(CanvasEntity {
            id: 89,
            pillar_badge: "P4".to_string(),
            pillar_name: "Parametric".to_string(),
            kind: CanvasShapeKind::Line {
                start_mm: [20000.0, 0.0],
                end_mm: [24000.0, 0.0],
            },
            color_rgb: [56, 189, 248],
            stroke_width: 2.0,
        });
        let (pure_end_pt, pure_end_kind) =
            canvas.snap_point_with_osnap([23980.0, 15.0], None, None, 200.0);
        assert!((pure_end_pt[0] - 24000.0).abs() < 1e-3 && pure_end_pt[1].abs() < 1e-3);
        assert_eq!(pure_end_kind, OsnapKind::Endpoint);
        canvas.entities.pop();

        // 2. Marquee Window (L->R) vs Crossing (R->L) selection + Multi-entity Array & Promote
        canvas.entities.push(CanvasEntity {
            id: 90,
            pillar_badge: "P4".to_string(),
            pillar_name: "Parametric".to_string(),
            kind: CanvasShapeKind::Rectangle {
                min_mm: [0.0, 0.0],
                max_mm: [6000.0, 5000.0],
            },
            color_rgb: [56, 189, 248],
            stroke_width: 2.0,
        });
        canvas.next_entity_id = 91;

        // Small Window marquee excluding the 6000x5000 rectangle should find 0 full-enclosed boxes inside [100,100]->[1000,1000]
        let win_cnt = canvas.select_marquee([100.0, 100.0], [1000.0, 1000.0]);
        assert_eq!(win_cnt, 0);

        // Crossing (Right->Left) over the edge intersects rectangle & lines
        let cross_cnt = canvas.select_marquee([1000.0, 1000.0], [-50.0, -50.0]);
        assert!(cross_cnt >= 3);

        // Window selection enclosing [0,0]->[6000,5000] excludes 12m horizontal axes (3 enclosed)
        assert_eq!(canvas.select_marquee([-50.0, -50.0], [6050.0, 5050.0]), 3);
        // Full Window selection enclosing all 5 grid axes + rectangle (6 entities)
        let full_win = canvas.select_marquee([-50.0, -50.0], [12050.0, 5050.0]);
        assert_eq!(full_win, 6);

        // Promote selection into 3D BIM Walls, Beams, Columns & Slabs
        let msg = canvas.execute_precision_command(
            "promote",
            NavigationPillar::GeometryAssembly,
            &mut bim,
        );
        assert!(msg.contains("Walls"));
        assert!(!bim.walls.is_empty());
        assert!(!bim.slabs.is_empty());
        assert!(!bim.columns.is_empty());

        // Rectangular ARRAY command
        let arr_msg = canvas.execute_precision_command(
            "array 2x2 7000 6000",
            NavigationPillar::GeometryAssembly,
            &mut bim,
        );
        assert!(arr_msg.contains("Arrayed 2x2"));
    }

    #[test]
    fn test_phase43_default_3d_google_earth_mode_and_archive_roundtrip() {
        use crate::core::gis::{EarthBrushAction, MegastructureKind, TileSource};

        let mut canvas = UniversalCanvasState::default();
        assert!(canvas.globe_mode);
        assert_eq!(canvas.world_map.tile_provider, TileSource::EsriWorldImagery);
        assert!(canvas.world_map.zoom_level < 3.0);

        let before_co2 = canvas.earth_world.vitals.co2_ppm;
        let touched = canvas
            .earth_world
            .apply_brush(-1.0, 114.0, EarthBrushAction::PlantForest, true);
        assert!(touched > 0);
        assert!(canvas.earth_world.vitals.co2_ppm < before_co2);

        let id = canvas.earth_world.add_megastructure(
            MegastructureKind::TransStraitBridge,
            (-5.9, 105.9),
            (-6.1, 106.1),
        );
        assert!(id >= 1);
        assert_eq!(canvas.earth_world.megastructures.len(), 1);

        canvas.globe_mode = false;
        let bim = BimProject::sample_bimcraft_building();
        let members = vec![StructuralMember::default()];
        let vars = Vec::new();
        let archive = canvas.build_unified_archive(&bim, &members, &vars, sample_settings());
        let json = archive.to_pretty_json().expect("serialize archive");
        let _verified = UnifiedWorkspaceArchive::from_verified_json(&json).expect("verify archive");

        let mut restored = UniversalCanvasState::default();
        let mut out_bim = BimProject::default();
        let mut out_members = Vec::new();
        let mut out_vars = Vec::new();
        let out_settings = restored
            .apply_unified_archive(&json, &mut out_bim, &mut out_members, &mut out_vars)
            .expect("restore unified archive");

        assert!(!restored.globe_mode);
        assert_eq!(out_settings.solver_threads, sample_settings().solver_threads);
        assert_eq!(restored.earth_world.cells.len(), canvas.earth_world.cells.len());
        assert_eq!(restored.earth_world.megastructures.len(), 1);
        assert!((restored.earth_world.vitals.co2_ppm - canvas.earth_world.vitals.co2_ppm).abs() < 1e-6);
    }

    #[test]
    fn test_phase43_3d_globe_projection_and_responsive_viewport() {
        use crate::core::gis::{EarthBrushAction, EarthScenario, MegastructureKind};

        let mut canvas = UniversalCanvasState::default();
        let (slat, slon, szoom) = canvas
            .earth_world
            .load_scenario(EarthScenario::CoastalDeltaDefense);
        canvas.world_map.zoom_to_ground_site(slat, slon, szoom);
        canvas.ground_origin_lat_lon = (slat, slon);
        canvas.ground_origin_anchored = true;
        canvas
            .earth_world
            .apply_brush(slat, slon, EarthBrushAction::BuildSeawall, true);
        canvas.earth_world.add_megastructure(
            MegastructureKind::SolarCleanCorridor,
            (slat, slon),
            (slat + 0.8, slon + 1.2),
        );

        let mut bim = BimProject::sample_bimcraft_building();
        let mut members = vec![StructuralMember::default()];
        let mut vars = Vec::new();
        let theme = MacTheme::for_mode(ThemeMode::ProDark);
        let ws = sample_settings();

        for w in [1000.0_f32, 700.0, 390.0] {
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, 820.0))),
                ..Default::default()
            };
            let mut max_right = 0.0_f32;
            let mut panel_right = 0.0_f32;
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    panel_right = ui.max_rect().right();
                    let resp = ui.scope(|ui| {
                        canvas.show(
                            ui,
                            &theme,
                            NavigationPillar::GeometryAssembly,
                            &mut bim,
                            &mut members,
                            &mut vars,
                            ws.clone(),
                            0.0,
                        );
                    });
                    max_right = resp.response.rect.right();
                });
            });
            out.textures_delta.clear();
            assert!(canvas.last_globe_xform.is_some());
            assert!(
                max_right <= panel_right + 1.0,
                "Overflow at width {w}: content right {max_right} > panel right {panel_right}"
            );
        }

        let rect = canvas.last_canvas_rect.expect("canvas rect");
        let p = canvas.world_to_screen(rect, [0.0, 0.0]);
        assert!(rect.expand(40.0).contains(p));
        let back_mm = canvas.screen_to_world(rect, p);
        assert!(back_mm[0].abs() < 25_000_000.0 && back_mm[1].abs() < 25_000_000.0);
    }

    #[test]
    fn test_phase45_lifecycle_archive_roundtrip() {
        let mut canvas = UniversalCanvasState::default();
        canvas.ground_origin_lat_lon = (-6.12, 106.85);
        canvas.ground_origin_anchored = true;
        canvas.lifecycle_ops.mep_sizing.floor_area_m2 = 2450.0;
        canvas.lifecycle_ops.procurement.retention_pct = 7.5;
        canvas
            .lifecycle_ops
            .daily_diary
            .entries
            .push(crate::core::DailyDiaryEntry {
                day_label: "Day 45 (Thu)".to_string(),
                weather: "Clear".to_string(),
                temp_c: 32.0,
                workable_hours: 9.0,
                carpentry_crew: 22,
                rebar_crew: 18,
                concrete_crew: 16,
                mep_crew: 14,
                crane_util_pct: 91.0,
                pump_m3_poured: 140.0,
                incident_free: true,
                remarks: "Transfer deck pour complete".to_string(),
            });
        canvas.lifecycle_ops.qa_qc.ncrs[0].closed = true;

        let bim = BimProject::sample_bimcraft_building();
        let members = vec![StructuralMember::default()];
        let vars = Vec::new();
        let archive = canvas.build_unified_archive(&bim, &members, &vars, sample_settings());
        let json = archive.to_pretty_json().expect("serialize");

        let mut restored = UniversalCanvasState::default();
        let mut out_bim = BimProject::default();
        let mut out_members = Vec::new();
        let mut out_vars = Vec::new();
        let _ = restored
            .apply_unified_archive(&json, &mut out_bim, &mut out_members, &mut out_vars)
            .expect("restore");

        assert!((restored.lifecycle_ops.mep_sizing.floor_area_m2 - 2450.0).abs() < 1e-6);
        assert!((restored.lifecycle_ops.procurement.retention_pct - 7.5).abs() < 1e-6);
        assert_eq!(restored.lifecycle_ops.daily_diary.entries.len(), 4);
        assert!(restored.lifecycle_ops.qa_qc.ncrs[0].closed);
        assert!(restored.lifecycle_ops.site_coupling.seismic_pga_ss_g >= 0.85);
    }

    #[test]
    fn test_phase47_earth_zoom_drag_sensitivity_and_cursor_lock() {
        let mut canvas = UniversalCanvasState::default();
        canvas.globe_mode = true;
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
        canvas.world_map.set_center(-6.2088, 106.8456);
        canvas.world_map.zoom_level = 16.0;
        canvas.last_globe_xform = Some(GlobeScreenTransform::from_view(
            rect,
            canvas.world_map.center_lat_lon.0,
            canvas.world_map.center_lat_lon.1,
            canvas.world_map.zoom_level,
            &canvas.earth_world.camera,
        ));

        // 1. At zoom=16.0, dragging right by 20px keeps ground movement ~20px on screen (no oversensitivity)
        let focus = Pos2::new(620.0, 360.0);
        let (pin_lat, pin_lon, hit_pin) = canvas.screen_to_lat_lon_with_hit(focus, rect);
        assert!(hit_pin);
        canvas.orbit_globe_by_pixels(rect, Vec2::new(20.0, 0.0), false, 0.016);
        // Inertia disabled above zoom 11.0 so site editing never slides after pointer release
        assert_eq!(canvas.earth_world.camera.angular_vel_lat, 0.0);
        assert_eq!(canvas.earth_world.camera.angular_vel_lon, 0.0);

        let moved_xf = canvas.last_globe_xform.as_ref().unwrap();
        let (after_drag_px, _, _) = moved_xf
            .project_lat_lon_elev(pin_lat, pin_lon, 0.0)
            .unwrap();
        let shift_x = (after_drag_px.x - focus.x) as f64;
        assert!(
            (shift_x - 20.0).abs() < 2.0,
            "20px globe drag at z=16.0 shifted point by {shift_x:.2}px instead of ~20px"
        );

        // 2. Cursor-anchored zoom at z=16.0 keeps the hovered Earth point under `focus`
        let (hover_lat, hover_lon, _) = canvas.screen_to_lat_lon_with_hit(focus, rect);
        canvas.apply_globe_zoom_at_screen(rect, 0.25, Some(focus));
        let post_xf = canvas.last_globe_xform.as_ref().unwrap();
        let (reproj, _, _) = post_xf
            .project_lat_lon_elev(hover_lat, hover_lon, 0.0)
            .unwrap();
        let lock_err = (reproj - focus).length();
        assert!(
            lock_err < 2.0,
            "Cursor-anchored Earth zoom drifted hover point by {lock_err:.2}px"
        );
    }
}
