//! views/build/mod.rs — Pillar 5 (3D BIM): Clean BIM Spatial Assembly & 360-deg CSG B-Rep Viewport.
//! Features a True 360° Interactive 3D Turntable/Orbit Viewport, Solid Profile Extrusions,
//! Depth-Sorted Shaded Rendering (Painter's Algorithm), and Stress/EigenGate Gradient Contours.

pub mod bottom_deck;
pub mod clash_bcf;
pub mod ribbon;
pub mod sidebar;
pub mod viewport_3d;

use egui::{Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};
use crate::icons::{self, AppIcon};
use crate::models::{
    DatabaseComponent, NavigationPillar, ParametricVariable, SidebarTab, StructuralMember,
};
use crate::features::{FeatureId, FeatureRegistry};
use crate::theme::MacTheme;
use crate::ui_kit;
use crate::views::universal_canvas::{UniversalCanvasState, WorkspaceCanvasSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelArchetype {
    BimCraftBuilding3D,      // BIMCraft LOD 350 Relational Building (CSG Walls, WF Mesh, Slabs & Rebar)
    DiagridTower3D,          // 48-Storey Diagrid Skyscraper IFC
    TrussBridge3D,           // 3D Space Truss Arch Bridge
    SpaceDome3D,             // Double-Layer Reticulated Dome
    WarrenTruss2D,           // Warren 24m Standard
    DoubleLayerSpaceTruss3D, // 3D Double-Layer Space Truss (MERO Octet Lattice)
    GeodesicDome3D,          // 3D Schwedler Geodesic Dome
    HyparRuledShell3D,       // 3D Hyperbolic Paraboloid Ruled Shell
    MultiStoryTower3D,       // 3D Multi-Story Building Frame with Core Walls
    PrestressedGirderBridge3D, // 3D Prestressed Post-Tensioned Concrete Bridge Span
    CoupledShearWallCore3D,  // 3D Coupled Special Structural Shear Wall Core Tower
    BaseIsolatedHospital3D,  // 3D Base-Isolated Essential Facility (8-Story Hospital with Base Isolators)
    TimeHistoryTower3D,      // 3D 16-Story NL-THA Seismic Test Tower Archetype
    FemaP58HospitalArchetype, // 3D 10-Story FEMA P-58 Resilience & Loss Assessment Archetype
    BlastResistantBunker3D,  // 3D Hardened Protective Blast Bunker & Standoff Barrier Archetype (UFC 3-340-02)
    FireCompartmentFrame3D,  // 3D Multi-Bay Steel Frame with Localized Fire Compartment (AISC 360-16 / EN 1993-1-2)
    OpenBimIfcModel3D,       // 3D OpenBIM Residential Duplex Villa authored in pure IFCX (IFC 5)
}

impl ModelArchetype {
    pub fn label(&self) -> &'static str {
        match self {
            Self::BimCraftBuilding3D => "BIMCraft LOD350 (CSG + Rebar)",
            Self::OpenBimIfcModel3D => "OpenBIM Duplex Villa (IFCX)",
            Self::FireCompartmentFrame3D => "Fire Compartment Steel Frame (AISC/EC3)",
            Self::BlastResistantBunker3D => "Blast Protective Bunker (UFC 3-340-02)",
            Self::FemaP58HospitalArchetype => "FEMA P-58 Hospital (10-Story)",
            Self::BaseIsolatedHospital3D => "Base-Isolated Hospital (8-Story)",
            Self::TimeHistoryTower3D => "NL-THA Tower (16-Story)",
            Self::DiagridTower3D => "Diagrid Frame Architecture",
            Self::TrussBridge3D => "Long-Span Steel Truss Bridge",
            Self::SpaceDome3D => "Geodesic Space Frame Dome",
            Self::WarrenTruss2D => "Planar Warren Roof Truss",
            Self::DoubleLayerSpaceTruss3D => "Space Truss (24m MERO Octet)",
            Self::GeodesicDome3D => "Geodesic Dome (18m Lattice)",
            Self::HyparRuledShell3D => "Hypar Umbrella (16m Saddle)",
            Self::MultiStoryTower3D => "12-Story Tower (Core Wall)",
            Self::PrestressedGirderBridge3D => "Prestressed Bridge (30m PT Girder)",
            Self::CoupledShearWallCore3D => "Coupled Shear Wall Core (12-Story)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground3DAuthorTool {
    Orbit,
    Column,
    Wall,
    Beam,
    Footing,
}

impl Ground3DAuthorTool {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Orbit => "Orbit",
            Self::Column => "+Col",
            Self::Wall => "+Wall",
            Self::Beam => "+Beam",
            Self::Footing => "+Foot",
        }
    }

    pub fn app_icon(&self) -> AppIcon {
        match self {
            Self::Orbit => AppIcon::CursorPan,
            Self::Column => AppIcon::Column,
            Self::Wall => AppIcon::LineSeg,
            Self::Beam => AppIcon::Beam,
            Self::Footing => AppIcon::BoxPackage,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Render3DMode {
    CsgSolidBrep,     // True 3D CSG B-Rep Triangulated Mesh (Boolean Openings, WF Flanges & Rebar)
    SolidShaded,      // 3D Extruded Shaded Polygonal Beams with Lighting
    Wireframe,        // Crisp Vector Wireframe Lines
    AxialForceNFD,    // Normal / Axial Force Diagram Envelope (Tension red / Compression blue)
    ShearForceSFD,    // Shear Force Diagram Envelope (Green stepped polygon)
    BendingMomentBMD, // Bending Moment Diagram Envelope (Amber parabolic curve with peak callouts)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BimRibbonTab {
    Architecture,
    Structure,
    MepSystems,
    CivilInfra,
    Time4D,
    Cost5D,
    Sustainability6D,
    FacilityTwin7D,
    CoordinationAudit,
    OpenBimSpec,
    SheetsDed,
}

impl BimRibbonTab {
    pub fn app_icon(&self) -> AppIcon {
        match self {
            Self::Architecture => AppIcon::Building,
            Self::Structure => AppIcon::Column,
            Self::MepSystems => AppIcon::Bolt,
            Self::CivilInfra => AppIcon::Globe,
            Self::Time4D => AppIcon::Clock,
            Self::Cost5D => AppIcon::Coins,
            Self::Sustainability6D => AppIcon::Leaf,
            Self::FacilityTwin7D => AppIcon::Cube3D,
            Self::CoordinationAudit => AppIcon::Shield,
            Self::OpenBimSpec => AppIcon::LinkChain,
            Self::SheetsDed => AppIcon::Document,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Architecture => "Architecture",
            Self::Structure => "Structure",
            Self::MepSystems => "MEP Systems",
            Self::CivilInfra => "Civil & Terrain",
            Self::Time4D => "4D Phased",
            Self::Cost5D => "5D Cost",
            Self::Sustainability6D => "6D Green",
            Self::FacilityTwin7D => "7D Twin",
            Self::CoordinationAudit => "Coordination & Clash",
            Self::OpenBimSpec => "OpenBIM",
            Self::SheetsDed => "Sheets & DED",
        }
    }
}

/// Phase 45: Below-toolbar external Structural Analysis sub-menus in Geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryAnalysisSubTab {
    DiagramsAndHeatmap,
    SolverAndN1,
    CriticalMembersBridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomDeckTab {
    ScheduleQto,
    TimeSimulation4D,
    ClashManager,
    BimConsole,
    PointCloudDeviation,
}

impl BottomDeckTab {
    pub fn app_icon(&self) -> AppIcon {
        match self {
            Self::ScheduleQto => AppIcon::ChartBar,
            Self::TimeSimulation4D => AppIcon::Clock,
            Self::ClashManager => AppIcon::AlertTriangle,
            Self::BimConsole => AppIcon::Cpu,
            Self::PointCloudDeviation => AppIcon::Target,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ScheduleQto => "Live Schedules & BoQ (QTO)",
            Self::TimeSimulation4D => "4D Construction Simulation",
            Self::ClashManager => "Clash & Coordination Deck",
            Self::BimConsole => "BIM Terminal & Automation",
            Self::PointCloudDeviation => "LiDAR Point Cloud & Deviation",
        }
    }
}

pub struct BuildView {
    pub active_sidebar_tab: SidebarTab,
    pub active_model: ModelArchetype,
    pub render_mode: Render3DMode,
    pub perspective: bool,
    pub selected_member_id: Option<usize>,
    pub simulated_eliminated_ids: Vec<usize>,
    pub color_mode: String,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_node_labels: bool,
    pub show_deformed_shape: bool,
    pub deformation_scale: f32,
    pub animate_vibration: bool,
    pub vibration_phase: f32,
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    pub cam_zoom: f32,
    pub cam_pan: Vec2,
    pub bim_bus: crate::core::BimCommandBus,
    pub show_rebar_cage: bool,
    pub section_box_enabled: bool,
    pub section_box_max_z_m: f32,
    pub modify_status_msg: Option<String>,
    // Phase 26: Master BIM World-Class Suite
    pub active_ribbon_tab: BimRibbonTab,
    pub active_bottom_deck_tab: BottomDeckTab,
    pub bottom_deck_expanded: bool,
    pub active_tool_status: String,
    pub view_scale: String,
    pub lod_level: String,
    pub show_level_datums: bool,
    pub show_grid_bubbles: bool,
    pub show_sun_shadows: bool,
    pub temporary_isolate_active: bool,
    pub section_box_min_x: f32,
    pub section_box_max_x: f32,
    pub section_box_min_y: f32,
    pub section_box_max_y: f32,
    pub section_box_min_z: f32,
    pub simulation_4d_playing: bool,
    pub simulation_4d_day: usize,
    pub simulation_4d_speed: f32,
    pub bim_console_input: String,
    pub bim_console_logs: Vec<String>,
    pub clash_tolerance_mm: f64,
    pub clash_filter_discipline: String,
    pub selected_clash_id: Option<usize>,
    // Phase 29: Stateful Clash Detection & BCF 2.1 Coordination Hub
    pub clash_items: Vec<crate::models::ClashDetectionItem>,
    pub show_3d_clash_markers: bool,
    pub clash_filter_severity: Option<crate::models::ClashSeverity>,
    pub clash_last_run_label: String,
    pub bcf_topics: Vec<crate::models::BcfTopicRecord>,
    pub selected_bcf_topic_id: Option<usize>,
    pub show_new_bcf_form: bool,
    pub new_bcf_title: String,
    pub new_bcf_assignee: String,
    pub new_bcf_priority: crate::models::BcfPriority,
    pub bcf_comment_draft: String,
    pub schedule_search_query: String,
    pub schedule_category_filter: String,
    pub active_workset_visibility: [bool; 7],
    // Phase 9: Modeling & Spatial Geometry Suite
    pub wizard_space_truss_lx: f64,
    pub wizard_space_truss_ly: f64,
    pub wizard_space_truss_depth: f64,
    pub wizard_dome_radius: f64,
    pub wizard_dome_rings: usize,
    pub wizard_dome_sectors: usize,
    pub wizard_hypar_lx: f64,
    pub wizard_hypar_ly: f64,
    pub wizard_hypar_sagitta: f64,
    pub wizard_tower_stories: usize,
    pub wizard_tower_story_h: f64,
    pub wizard_tower_bays_x: usize,
    pub wizard_tower_bays_y: usize,
    pub section_preset_idx: usize,
    pub sec_dim_d: f64,
    pub sec_dim_bf: f64,
    pub sec_dim_tf: f64,
    pub sec_dim_tw: f64,
    pub draft_node_x: f64,
    pub draft_node_y: f64,
    pub draft_node_z: f64,
    pub draft_member_n1: usize,
    pub draft_member_n2: usize,
    pub last_viewport_rect: Option<Rect>,
    pub floating_properties_open: bool,
    pub floating_project_open: bool,
    pub floating_deck_open: bool,
    pub show_discipline_ribbon: bool,
    pub sketch_mode_active: bool,
    pub sketch_overlay_synced: bool,
    pub properties_canvas_sync_tab: bool,
    pub last_floating_rects: Vec<Rect>,
    pub ground_author_tool: Ground3DAuthorTool,
    pub pending_3d_start_mm: Option<[f64; 2]>,
    pub hover_3d_ground_snap_mm: Option<[f64; 2]>,
    // Phase 49: Precision Manual Authoring Dimensions
    pub author_col_width_mm: f64,
    pub author_col_depth_mm: f64,
    pub author_beam_width_mm: f64,
    pub author_beam_depth_mm: f64,
    pub author_wall_thickness_mm: f64,
    pub author_footing_size_mm: f64,
    // Phase 38: Federated Discipline Layers & 4D Erection Highlights
    pub show_layer_arch: bool,
    pub show_layer_struct: bool,
    pub show_layer_mep: bool,
    pub show_layer_geo: bool,
    pub highlight_4d_erection: bool,
    // Phase 41: Point Cloud & As-Built Deviation Analyzer (3D-06)
    pub point_cloud_tol_mm: f64,
    pub point_cloud_overlay_enabled: bool,
    pub point_cloud_raw_ascii: String,
    pub point_cloud_status: Option<String>,
    // Phase 45: Below-toolbar external Structural Analysis Sub-Menu + Bidirectional Bridge to Analysis
    pub active_analysis_subtab: GeometryAnalysisSubTab,
    pub analysis_panel_open: bool,
    pub requested_analysis_jump: Option<(crate::views::analyze::AnalyzeSubTab, Option<usize>, String)>,
    // Phase 46: Spatial Fullscreen BIM Picking State
    pub last_spatial_bim_pick: Option<(crate::core::BimElementKind, usize)>,
    pub spatial_pick_seq: u64,
    // Phase 52: Optional 3D Earth Map Backdrop toggle for uncluttered clean BIM inspection
    pub show_3d_map_backdrop: bool,
}

impl Default for BuildView {
    fn default() -> Self {
        Self {
            active_sidebar_tab: SidebarTab::Properties,
            active_model: ModelArchetype::BimCraftBuilding3D,
            render_mode: Render3DMode::CsgSolidBrep,
            perspective: true,
            selected_member_id: None,
            simulated_eliminated_ids: Vec::new(),
            color_mode: "EigenGate".to_string(),
            show_grid: true,
            show_axes: true,
            show_node_labels: true,
            show_deformed_shape: false,
            deformation_scale: 50.0,
            animate_vibration: false,
            vibration_phase: 0.0,
            cam_yaw: 0.65,
            cam_pitch: 0.38,
            cam_zoom: 1.15,
            cam_pan: Vec2::ZERO,
            bim_bus: crate::core::BimCommandBus::new(),
            show_rebar_cage: true,
            section_box_enabled: false,
            section_box_max_z_m: 25.0,
            modify_status_msg: None,
            // Phase 26 Defaults
            active_ribbon_tab: BimRibbonTab::Architecture,
            active_bottom_deck_tab: BottomDeckTab::ScheduleQto,
            bottom_deck_expanded: false,
            active_tool_status: "Ready • Unfreeze 3D Viewport to orbit Earth ground & draw geometry.".to_string(),
            view_scale: "1:100".to_string(),
            lod_level: "LOD 350 (Fine BIM Detail)".to_string(),
            show_level_datums: true,
            show_grid_bubbles: true,
            show_sun_shadows: true,
            temporary_isolate_active: false,
            section_box_min_x: -30.0,
            section_box_max_x: 30.0,
            section_box_min_y: -30.0,
            section_box_max_y: 30.0,
            section_box_min_z: -5.0,
            simulation_4d_playing: false,
            simulation_4d_day: 0,
            simulation_4d_speed: 1.0,
            bim_console_input: String::new(),
            bim_console_logs: vec![
                "[BIM Kernel] Blank 3D Earth Ground Workspace initialized (Frozen by default).".to_string(),
            ],
            clash_tolerance_mm: 15.0,
            clash_filter_discipline: "All (Arch vs Struct vs MEP)".to_string(),
            selected_clash_id: None,
            clash_items: Vec::new(),
            show_3d_clash_markers: true,
            clash_filter_severity: None,
            clash_last_run_label: "No interference check run yet".to_string(),
            bcf_topics: Vec::new(),
            selected_bcf_topic_id: None,
            show_new_bcf_form: false,
            new_bcf_title: String::new(),
            new_bcf_assignee: "Structural Lead".to_string(),
            new_bcf_priority: crate::models::BcfPriority::High,
            bcf_comment_draft: String::new(),
            schedule_search_query: String::new(),
            schedule_category_filter: "All Elements".to_string(),
            active_workset_visibility: [true, true, true, true, true, true, true],
            // Phase 9 Defaults
            wizard_space_truss_lx: 24.0,
            wizard_space_truss_ly: 24.0,
            wizard_space_truss_depth: 2.0,
            wizard_dome_radius: 18.0,
            wizard_dome_rings: 6,
            wizard_dome_sectors: 16,
            wizard_hypar_lx: 16.0,
            wizard_hypar_ly: 16.0,
            wizard_hypar_sagitta: 3.0,
            wizard_tower_stories: 12,
            wizard_tower_story_h: 3.5,
            wizard_tower_bays_x: 4,
            wizard_tower_bays_y: 4,
            section_preset_idx: 0,
            sec_dim_d: 300.0,
            sec_dim_bf: 150.0,
            sec_dim_tf: 9.0,
            sec_dim_tw: 6.5,
            draft_node_x: 0.0,
            draft_node_y: 0.0,
            draft_node_z: 0.0,
            draft_member_n1: 0,
            draft_member_n2: 1,
            last_viewport_rect: None,
            floating_properties_open: false,
            floating_project_open: false,
            floating_deck_open: false,
            show_discipline_ribbon: false,
            sketch_mode_active: false,
            sketch_overlay_synced: true,
            properties_canvas_sync_tab: false,
            last_floating_rects: Vec::new(),
            ground_author_tool: Ground3DAuthorTool::Orbit,
            pending_3d_start_mm: None,
            hover_3d_ground_snap_mm: None,
            author_col_width_mm: 400.0,
            author_col_depth_mm: 400.0,
            author_beam_width_mm: 300.0,
            author_beam_depth_mm: 550.0,
            author_wall_thickness_mm: 200.0,
            author_footing_size_mm: 1600.0,
            show_layer_arch: true,
            show_layer_struct: true,
            show_layer_mep: true,
            show_layer_geo: true,
            highlight_4d_erection: false,
            point_cloud_tol_mm: 10.0,
            point_cloud_overlay_enabled: true,
            point_cloud_raw_ascii: "0.0,0.0,0.0,+2.8\n6000.0,0.0,3200.0,-4.1\n3000.0,4500.0,0.0,+11.4\n9000.0,2000.0,3500.0,+3.2\n12000.0,6000.0,0.0,-12.6".to_string(),
            point_cloud_status: None,
            active_analysis_subtab: GeometryAnalysisSubTab::DiagramsAndHeatmap,
            analysis_panel_open: true,
            requested_analysis_jump: None,
            last_spatial_bim_pick: None,
            spatial_pick_seq: 0,
            show_3d_map_backdrop: true,
        }
    }
}


impl BuildView {
    /// Returns true while the pointer hovers the 3D BIM viewport (outside floating HUD panels)
    /// and scrolling is active (unfrozen).
    pub fn is_pointer_in_live_canvas(&self, ctx: &egui::Context, is_frozen: bool) -> bool {
        if is_frozen {
            return false;
        }
        let Some(rect) = self.last_viewport_rect else {
            return false;
        };
        let Some(pointer) = ctx.input(|i| i.pointer.hover_pos()) else {
            return false;
        };
        let in_canvas = rect.contains(pointer);
        let in_floating = self.last_floating_rects.iter().any(|r| r.contains(pointer));
        in_canvas && !in_floating
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        db_components: &[DatabaseComponent],
        features: &mut FeatureRegistry,
        universal_canvas: &mut crate::views::universal_canvas::UniversalCanvasState,
        parametric_variables: &[crate::models::ParametricVariable],
        workspace_settings: crate::views::universal_canvas::WorkspaceCanvasSettings,
        sidebar_toggle_inset_px: f32,
    ) {
        ui_kit::set_canvas_frozen(ui.ctx(), universal_canvas.is_frozen);
        self.last_viewport_rect = None;

        Frame::new()
            .fill(theme.bg_sidebar)
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, theme.border_card))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                let outer_w = ui.available_width().max(160.0);
                ui.set_width(outer_w);
                ui.set_max_width(outer_w);
                self.render_canvas(
                    ui,
                    theme,
                    members,
                    db_components,
                    features,
                    universal_canvas,
                    parametric_variables,
                    workspace_settings,
                    sidebar_toggle_inset_px,
                );
            });

        ui.add_space(10.0);

        // External Below-Toolbar Structural Analysis Sub-Menu Panel (Diagrams/Heatmap, FEA Solver & N-1, Critical Bridge)
        self.render_below_toolbar_structural_analysis(ui, theme, members, universal_canvas);
    }
}
