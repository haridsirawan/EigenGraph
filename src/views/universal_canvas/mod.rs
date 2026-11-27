//! views/universal_canvas.rs — Unified Synchronized 15-Pillar Drawing Canvas & Single-File Project Archive.
//! Provides a canvas-first interactive 2D drafting & domain-overlay workspace across all 15 sidebar
//! navigation modules, featuring a Freeze Canvas guard for uninterrupted vertical page scrolling,
//! a floating collapsible domain inspector, and tamper-evident single-file `.eigen.json` sync.

use egui::{Align2, Color32, CornerRadius, Frame, Margin, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::core::gis::earth_globe::{GlobeScreenTransform, PlanetaryWorldState};
use crate::core::gis::SlippyMapState;
use crate::core::{
    compute_site_environmental_coupling, BimProject, DiskExporter, LifecycleOpsState,
    SiteEnvironmentalCoupling,
};
use crate::models::{NavigationPillar, ParametricVariable, StructuralMember};
use crate::theme::MacTheme;
use crate::ui_kit;


mod cad_ops;
mod interaction;
mod render_layers;
mod toolbars;
mod types;
pub use types::*;

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
    // Phase 59 & 64: Offline Pure-Vector Site Layer, OpenBIM IFC Site TIN & Neighborhood Physics
    pub urban_vectors: crate::core::gis::UrbanVectorLayerCache,
    pub site_tin_from_ifc: bool,
    pub neighborhood_report: crate::core::gis::NeighborhoodAuditReport,
    pub show_neighborhood_hazards: bool,
    pub active_vector_snap_az_deg: Option<f64>,
    // Phase 63: O(1) Cached BIM-FEM Lookup, True 6-DOF Displacements, Modal Eigenmodes & N-1 Disruption
    pub earth_fem_cache: EarthStructuralRenderCache,
    // Phase 65 / 75: 100% Pure WebGPU (`wgpu` + WGSL) 3D Vector Earth Renderer (0% Raster Pixels, 0% WebGL)
    pub webgpu_earth: crate::views::webgpu_earth::WebGpuEarthState,
    // Phase 67: IFCX City-Builder Authoring (`IfcRoad`, `IfcBridge`, `IfcSpatialZone`, Auto Cut/Fill Pads)
    pub city_scene: crate::core::gis::IfcxCityScene,
    // Phase 68: District-Scale Multi-Hazard & AI Real-Building Frame Inference Telemetry
    pub district_hazard: crate::core::gis::DistrictHazardConfig,
    pub district_hazard_report: crate::core::gis::DistrictMultiHazardReport,
    pub last_ai_capture_report: Option<crate::core::ifcx::AiBuildingFrameInferenceReport>,
    // Phase 69: Automated Structural Retrofit & Non-Destructive `.ifcx` Strengthening State
    pub retrofit_strategy: crate::core::retrofit::RetrofitStrategy,
    pub last_retrofit_report: Option<crate::core::retrofit::StructuralRetrofitReport>,
    pub pre_retrofit_baseline: Option<BimProject>,
    pub viewing_pre_retrofit_baseline: bool,
    // Phase 70: Two-Stage `EigenGate + StructGNN-CRC` Screening & Animated Progressive Collapse Cascade
    pub last_crc_cascade_report: Option<crate::core::cascade::TwoStageCrcCascadeReport>,
    pub active_cascade_step: usize,
    pub cascade_daf: f64,
    // Phase 71: 3D Subsurface Geotechnical Stratigraphy, Liquefaction & Coupled 6-DOF Winkler SSI
    pub subsurface_ssi: crate::core::geotech::SubsurfaceSsiConfig,
    pub last_subsurface_ssi_report: Option<crate::core::geotech::SubsurfaceSsiAnalysisReport>,
    // Phase 72: Multi-Layer IFCX Visual Diffing & Pure-Vector Engineering Dossier Exporter
    pub ifcx_baseline_snapshot: Option<crate::core::ifcx::IfcxDocument>,
    pub ifcx_baseline_label: String,
    pub last_ifcx_visual_diff: Option<crate::core::ifcx::IfcxVisualDiffReport>,
    pub ifcx_diff_overlay_active: bool,
    pub ifcx_diff_filter: crate::core::ifcx::IfcxDiffLayerFilter,
    pub last_vector_dossier_seal: Option<String>,
    // Phase 73: Autonomous Portfolio-Scale `EigenGate` Retrofit Optimization & Pareto Overlay
    pub portfolio_retrofit_config: crate::core::portfolio_retrofit::PortfolioRetrofitConfig,
    pub last_portfolio_retrofit_report: Option<crate::core::portfolio_retrofit::DistrictPortfolioRetrofitReport>,
    // Phase 74: Real-Time Async `StructGNN-CRC` & Live 60 FPS Earthquake Ground-Motion Replay
    pub ground_motion_replay_config: crate::core::ground_motion_replay::GroundMotionReplayConfig,
    pub last_ground_motion_replay_report: Option<crate::core::ground_motion_replay::GroundMotionReplayReport>,
    pub replay_async_manager: crate::core::ground_motion_replay::AsyncReplayJobManager,
    pub replay_anim_playing: bool,
    pub replay_cursor_time_s: f64,
    // Phase 76: Live SHM Digital Twin & Automated Post-Earthquake ATC-20 Placard Triage
    pub shm_twin_config: crate::core::shm_twin::ShmDigitalTwinConfig,
    pub last_shm_twin_report: Option<crate::core::shm_twin::ShmDigitalTwinReport>,
    // Phase 77: Autonomous Multi-Hazard Evacuation Routing & Emergency Lifelines
    pub evacuation_config: crate::core::evacuation_routing::EvacuationRoutingConfig,
    pub last_evacuation_report: Option<crate::core::evacuation_routing::EvacuationLifelineReport>,
    // Phase 78: Live WebSocket SHM Telemetry Streaming & Rolling-Window Watchdog
    pub shm_watchdog: crate::core::shm_watchdog::ShmWatchdogState,
    // Phase 79: WebGPU Compute Shaders (`@compute` WGSL) & FEMA P-58 Monte Carlo Accelerator
    pub monte_carlo_config: crate::core::gpu_fragility_mc::MonteCarloFragilityConfig,
    pub last_monte_carlo_report: Option<crate::core::gpu_fragility_mc::MonteCarloFragilityReport>,
    // Phase 80: Collaborative Multi-Layer IFCX Governance, Signed Checkpoints & 3D Risk Field
    pub collab_state: crate::core::ifcx_collab::IfcxCollabState,
    pub last_risk_field_report: Option<crate::core::ifcx_collab::DistrictRiskFieldReport>,
    // Phase 81: Parametric Trade-Study Matrix & Automated SNI/ASCE Permit Compliance Annex
    pub trade_study_config: crate::core::trade_study::TradeStudyConfig,
    pub last_trade_study_report: Option<crate::core::trade_study::TradeStudyReport>,
    pub last_permit_report: Option<crate::core::trade_study::PermitComplianceReport>,
    // Phase 82: Field Construction QA/QC Spatial Waypoints, Pour-Strip Tracker & 1D Rebar BBS Cut-List
    pub field_qaqc_overlay_visible: bool,
    pub field_fab_config: crate::core::field_fabrication::FieldFabricationConfig,
    pub field_qaqc_ledger: crate::core::lifecycle_ops::FieldQualityControlLedger,
    pub last_field_fab_report: Option<crate::core::field_fabrication::FieldFabricationReport>,
    pub active_waypoint_idx: usize,
    // Phase 83: EigenFlow Non-Linear Studio (Woodbury P1 -> P2 & Pushover ADRS)
    pub eigenflow_overlay_visible: bool,
    pub eigenflow_config: crate::core::eigenflow::EigenFlowConfig,
    pub last_eigenflow_report: Option<crate::core::eigenflow::EigenFlowStudioReport>,
    pub eigenflow_pushover_playing: bool,
    pub eigenflow_play_accum_sec: f32,
    // Phase 84: Extreme Events Studio (Blast UFC 3-340-02 & Structural Fire ISO 834/EC)
    pub extreme_events_overlay_visible: bool,
    pub extreme_events_config: crate::core::extreme_events::ExtremeEventsConfig,
    pub last_extreme_events_report: Option<crate::core::extreme_events::ExtremeEventsReport>,
    pub fire_sim_playing: bool,
    pub fire_play_accum_sec: f32,
    // Update v0.8.4.1 (1/10 -> v0.8.5): Seismic Base Isolation & Damper 3D Studio
    pub isolation_overlay_visible: bool,
    pub isolation_studio_config: crate::core::base_isolation_studio::IsolationStudioConfig,
    pub last_isolation_report: Option<crate::core::base_isolation_studio::IsolationStudioReport>,
    // Update v0.8.4.2 (2/10 -> v0.8.5): Wind Aeroelasticity, Facade Pressure & ISO 10137 Comfort
    pub wind_overlay_visible: bool,
    pub wind_studio_config: crate::core::wind_studio::WindStudioConfig,
    pub last_wind_report: Option<crate::core::wind_studio::WindStudioReport>,
    // Update v0.8.4.3 (3/10 -> v0.8.5): Special RC Shear Wall, SBE & Diagonal Coupling Beam
    pub shear_wall_overlay_visible: bool,
    pub shear_wall_studio_config: crate::core::shear_wall_studio::ShearWallStudioConfig,
    pub last_shear_wall_report: Option<crate::core::shear_wall_studio::ShearWallStudioReport>,
    // Update v0.8.4.4 (4/10 -> v0.8.5): Steel Seismic Connection, Bolts & Panel Zone
    pub connection_overlay_visible: bool,
    pub connection_studio_config: crate::core::connection_studio::ConnectionStudioConfig,
    pub last_connection_report: Option<crate::core::connection_studio::ConnectionStudioReport>,
    // Update v0.8.4.5 (5/10 -> v0.8.5): Post-Tensioned Tendon Drape & Losses
    pub prestress_overlay_visible: bool,
    pub prestress_studio_config: crate::core::prestress_studio::PrestressStudioConfig,
    pub last_prestress_report: Option<crate::core::prestress_studio::PrestressStudioReport>,
    // Update v0.8.4.6 (6/10 -> v0.8.5): Composite Deck, Shear Studs & AISC DG11 Vibration
    pub composite_overlay_visible: bool,
    pub composite_studio_config: crate::core::composite_studio::CompositeStudioConfig,
    pub last_composite_report: Option<crate::core::composite_studio::CompositeStudioReport>,
    // Update v0.8.4.7 (7/10 -> v0.8.5): Structural Reliability FORM/SORM & FEMA P-58 Fragility
    pub reliability_overlay_visible: bool,
    pub reliability_studio_config: crate::core::reliability_studio::ReliabilityStudioConfig,
    pub last_reliability_report: Option<crate::core::reliability_studio::ReliabilityStudioReport>,
    // Update v0.8.4.8 (8/10 -> v0.8.5): Deep Foundation Pile Group, Winkler Springs & Punching Shear
    pub foundation_overlay_visible: bool,
    pub foundation_studio_config: crate::core::foundation_studio::FoundationStudioConfig,
    pub last_foundation_report: Option<crate::core::foundation_studio::FoundationStudioReport>,
    // Update v0.8.4.9 (9/10 -> v0.8.5): 4D/5D/6D Construction Sequence, Cost S-Curve & Embodied Carbon LCA
    pub construction_lca_overlay_visible: bool,
    pub construction_lca_config: crate::core::construction_lca_studio::ConstructionLcaConfig,
    pub last_construction_lca_report:
        Option<crate::core::construction_lca_studio::ConstructionLcaStudioReport>,
    // Update v0.8.5.6 (6/10 -> v0.8.6): Site Zoning & 3D Buildable Envelope + Sun Path
    pub zoning_overlay_visible: bool,
    pub zoning_config: crate::core::zoning_agent::SiteZoningConfig,
    pub last_zoning_report: Option<crate::core::zoning_agent::SiteZoningReport>,
    // Update v0.8.5.7 (7/10 -> v0.8.6): Spatial Programming, Adjacency & 3D Stack/Pack
    pub program_overlay_visible: bool,
    pub program_config: crate::core::program_agent::SpatialProgramConfig,
    pub last_program_report: Option<crate::core::program_agent::SpatialProgramReport>,
    // Update v0.8.5.8 (8/10 -> v0.8.6): Multi-Option Massing Explorer & 1-Click Mass-to-Engineered-BIM
    pub massing_overlay_visible: bool,
    pub massing_config: crate::core::massing_agent::MassingStudyConfig,
    pub last_massing_report: Option<crate::core::massing_agent::MassingStudyReport>,
    // Update v0.8.5.9 (9/10 -> v0.8.6): Chained 6-Stage AI Building Design Pipeline
    pub last_design_pipeline_report: Option<crate::core::design_agent::DesignPipelineReport>,
    // Update v0.8.6.1 (1/10 -> v0.8.7): 3D Solar Irradiation, Cast Shadow & SNI 6389 OTTV Engine
    pub solar_overlay_visible: bool,
    pub solar_envelope_config: crate::core::solar_envelope::SolarEnvelopeConfig,
    pub last_solar_envelope_report: Option<crate::core::solar_envelope::SolarEnvelopeReport>,
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
            site_tin_from_ifc: false,
            neighborhood_report: crate::core::gis::NeighborhoodAuditReport::default(),
            show_neighborhood_hazards: true,
            active_vector_snap_az_deg: None,
            earth_fem_cache: EarthStructuralRenderCache::default(),
            webgpu_earth: crate::views::webgpu_earth::WebGpuEarthState::default(),
            city_scene: crate::core::gis::IfcxCityScene::default(),
            district_hazard: crate::core::gis::DistrictHazardConfig::default(),
            district_hazard_report: crate::core::gis::DistrictMultiHazardReport::default(),
            last_ai_capture_report: None,
            retrofit_strategy: crate::core::retrofit::RetrofitStrategy::default(),
            last_retrofit_report: None,
            pre_retrofit_baseline: None,
            viewing_pre_retrofit_baseline: false,
            last_crc_cascade_report: None,
            active_cascade_step: 1,
            cascade_daf: 1.50,
            subsurface_ssi: crate::core::geotech::SubsurfaceSsiConfig::default(),
            last_subsurface_ssi_report: None,
            ifcx_baseline_snapshot: None,
            ifcx_baseline_label: "Baseline Snapshot".to_string(),
            last_ifcx_visual_diff: None,
            ifcx_diff_overlay_active: false,
            ifcx_diff_filter: crate::core::ifcx::IfcxDiffLayerFilter::default(),
            last_vector_dossier_seal: None,
            portfolio_retrofit_config: crate::core::portfolio_retrofit::PortfolioRetrofitConfig::default(),
            last_portfolio_retrofit_report: None,
            ground_motion_replay_config: crate::core::ground_motion_replay::GroundMotionReplayConfig::default(),
            last_ground_motion_replay_report: None,
            replay_async_manager: crate::core::ground_motion_replay::AsyncReplayJobManager::default(),
            replay_anim_playing: true,
            replay_cursor_time_s: 0.0,
            shm_twin_config: crate::core::shm_twin::ShmDigitalTwinConfig::default(),
            last_shm_twin_report: None,
            evacuation_config: crate::core::evacuation_routing::EvacuationRoutingConfig::default(),
            last_evacuation_report: None,
            shm_watchdog: crate::core::shm_watchdog::ShmWatchdogState::default(),
            monte_carlo_config: crate::core::gpu_fragility_mc::MonteCarloFragilityConfig::default(),
            last_monte_carlo_report: None,
            collab_state: crate::core::ifcx_collab::IfcxCollabState::default(),
            last_risk_field_report: None,
            trade_study_config: crate::core::trade_study::TradeStudyConfig::default(),
            last_trade_study_report: None,
            last_permit_report: None,
            field_qaqc_overlay_visible: false,
            field_fab_config: crate::core::field_fabrication::FieldFabricationConfig::default(),
            field_qaqc_ledger: crate::core::lifecycle_ops::FieldQualityControlLedger::default(),
            last_field_fab_report: None,
            active_waypoint_idx: 0,
            eigenflow_overlay_visible: false,
            eigenflow_config: crate::core::eigenflow::EigenFlowConfig::default(),
            last_eigenflow_report: None,
            eigenflow_pushover_playing: false,
            eigenflow_play_accum_sec: 0.0,
            extreme_events_overlay_visible: false,
            extreme_events_config: crate::core::extreme_events::ExtremeEventsConfig::default(),
            last_extreme_events_report: None,
            fire_sim_playing: false,
            fire_play_accum_sec: 0.0,
            isolation_overlay_visible: false,
            isolation_studio_config: crate::core::base_isolation_studio::IsolationStudioConfig::default(),
            last_isolation_report: None,
            wind_overlay_visible: false,
            wind_studio_config: crate::core::wind_studio::WindStudioConfig::default(),
            last_wind_report: None,
            shear_wall_overlay_visible: false,
            shear_wall_studio_config: crate::core::shear_wall_studio::ShearWallStudioConfig::default(),
            last_shear_wall_report: None,
            connection_overlay_visible: false,
            connection_studio_config: crate::core::connection_studio::ConnectionStudioConfig::default(),
            last_connection_report: None,
            prestress_overlay_visible: false,
            prestress_studio_config: crate::core::prestress_studio::PrestressStudioConfig::default(),
            last_prestress_report: None,
            composite_overlay_visible: false,
            composite_studio_config: crate::core::composite_studio::CompositeStudioConfig::default(),
            last_composite_report: None,
            reliability_overlay_visible: false,
            reliability_studio_config: crate::core::reliability_studio::ReliabilityStudioConfig::default(),
            last_reliability_report: None,
            foundation_overlay_visible: false,
            foundation_studio_config: crate::core::foundation_studio::FoundationStudioConfig::default(),
            last_foundation_report: None,
            construction_lca_overlay_visible: false,
            construction_lca_config: crate::core::construction_lca_studio::ConstructionLcaConfig::default(),
            last_construction_lca_report: None,
            zoning_overlay_visible: false,
            zoning_config: crate::core::zoning_agent::SiteZoningConfig::default(),
            last_zoning_report: None,
            program_overlay_visible: false,
            program_config: crate::core::program_agent::SpatialProgramConfig::default(),
            last_program_report: None,
            massing_overlay_visible: false,
            massing_config: crate::core::massing_agent::MassingStudyConfig::default(),
            last_massing_report: None,
            last_design_pipeline_report: None,
            solar_overlay_visible: false,
            solar_envelope_config: crate::core::solar_envelope::SolarEnvelopeConfig::default(),
            last_solar_envelope_report: None,
        }
    }
}

impl UniversalCanvasState {
    /// Initializes native Pure WebGPU (`wgpu` + WGSL) 3D Vector Earth pipelines from `egui_wgpu::RenderState`.
    pub fn init_webgpu_earth(&mut self, render_state: &egui_wgpu::RenderState) {
        self.webgpu_earth.init_wgpu(render_state);
    }
    /// Computes crisp directional sun-lighting depth shading for 3D extruded Earth faces
    /// against an SE solar azimuth `(0.707, -0.707)`.
    pub fn directional_face_shade(base: Color32, p0: Pos2, p1: Pos2, is_top_cap: bool) -> Color32 {
        if is_top_cap {
            return base.gamma_multiply(0.82);
        }
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let len = dx.hypot(dy);
        if len < 1e-3 {
            return base.gamma_multiply(0.46);
        }
        let nx = dy / len;
        let ny = -dx / len;
        let sun_dot = nx * 0.707 + ny * -0.707;
        let factor = if sun_dot > 0.25 {
            0.64
        } else if sun_dot < -0.25 {
            0.30
        } else {
            0.46
        };
        base.gamma_multiply(factor)
    }

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

    /// Generates `active_site_tin` on demand using pure-Rust procedural `site_tin_preset`
    /// when no OpenBIM IFC `IfcSite` or local XYZ survey TIN has been loaded yet.
    pub fn ensure_site_tin_generated(&mut self) {
        if self.active_site_tin.is_none() {
            self.active_site_tin = Some(crate::core::gis::TinSurfaceMesh::generate_from_dem_preset(
                self.site_tin_preset,
                400.0,
                400.0,
                28,
                28,
                1.0,
            ));
            self.site_tin_from_ifc = false;
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

    /// Evaluates coupled district multi-hazard metrics (SNI 1726 seismic wave & pounding phase-lag,
    /// ASCE 7 Venturi wind channeling streamlines, 3D flood inundation vs. `IfcSite` TIN contours,
    /// and overall resilience score).
    pub fn refresh_district_hazard(&mut self, bim_project: &BimProject) {
        self.refresh_neighborhood_coupling(bim_project);
        self.district_hazard_report = crate::core::gis::evaluate_district_multi_hazard(
            &self.district_hazard,
            &self.neighborhood_report,
            self.active_site_tin.as_ref(),
            &self.world_index,
            self.ground_origin_lat_lon,
            bim_project,
            &self.city_scene,
        );
    }

    /// Evaluates coupled 3D subsurface stratigraphy, Seed–Idriss liquefaction screening,
    /// 6-DOF Winkler foundation springs, and modal SSI period shift for the active BIM project.
    pub fn refresh_subsurface_ssi(&mut self, bim_project: &BimProject) {
        if bim_project.is_empty() {
            self.last_subsurface_ssi_report = None;
            return;
        }
        let site = self.active_site_coupling();
        match crate::core::geotech::evaluate_coupled_subsurface_ssi_for_bim(
            bim_project,
            &site,
            &self.subsurface_ssi,
        ) {
            Ok(rep) => self.last_subsurface_ssi_report = Some(rep),
            Err(_) => self.last_subsurface_ssi_report = None,
        }
    }

    /// Builds a composed multi-layer `.ifcx` snapshot containing base BIM geometry and active overlays.
    pub fn build_active_layered_ifcx(
        &self,
        bim_project: &BimProject,
        members: &[StructuralMember],
    ) -> crate::core::ifcx::IfcxDocument {
        let site = self.active_site_coupling();
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let hazard_opt = self
            .district_hazard
            .any_active()
            .then_some(&self.district_hazard_report);
        let mut doc = crate::core::ifcx::IfcxDocument::build_layered_snapshot_for_workspace(
            bim_project,
            Some((
                self.ground_origin_lat_lon.0,
                self.ground_origin_lat_lon.1,
                site.elevation_m as f64,
            )),
            heading_deg,
            (!members.is_empty()).then_some(members),
            self.last_retrofit_report.as_ref(),
            self.last_crc_cascade_report.as_ref(),
            self.last_subsurface_ssi_report.as_ref(),
            hazard_opt,
        );
        if let Some(ref replay) = self.last_ground_motion_replay_report {
            doc = crate::core::ifcx::IfcxStage::compose_with_time_history_replay_overlay(
                &doc,
                replay,
            );
            doc = crate::core::ifcx::IfcxStage::compose_with_fragility_loss_overlay(
                &doc,
                replay,
            );
        }
        if let Some(ref shm) = self.last_shm_twin_report {
            doc = crate::core::ifcx::IfcxStage::compose_with_shm_atc20_overlay(&doc, shm);
        }
        if let Some(ref evac) = self.last_evacuation_report {
            doc = crate::core::ifcx::IfcxStage::compose_with_evacuation_lifelines_overlay(
                &doc, evac,
            );
        }
        if self.shm_watchdog.total_packets_ingested > 0 {
            let wd_overlay = self.shm_watchdog.build_ifcx_overlay(bim_project);
            doc = crate::core::ifcx::IfcxStage::compose_with_shm_watchdog_overlay(
                &doc,
                &wd_overlay,
            );
        }
        if let Some(ref mc_rep) = self.last_monte_carlo_report {
            let mc_overlay = mc_rep.build_ifcx_overlay(bim_project);
            doc = crate::core::ifcx::IfcxStage::compose_with_monte_carlo_loss_overlay(
                &doc,
                &mc_overlay,
            );
        }
        if let Some(gov_overlay) = self.collab_state.build_governance_seal_overlay(bim_project) {
            doc = crate::core::ifcx::IfcxStage::compose_with_governance_seal_overlay(
                &doc,
                &gov_overlay,
            );
        }
        if let Some(ref ts_rep) = self.last_trade_study_report {
            let ts_overlay = ts_rep.build_ifcx_overlay(bim_project);
            doc = crate::core::ifcx::IfcxStage::compose_with_trade_study_overlay(&doc, &ts_overlay);
        }
        doc
    }

    /// Captures the current layered `.ifcx` workspace graph as the baseline for visual diffing.
    pub fn capture_ifcx_baseline(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
        label: &str,
    ) {
        self.ifcx_baseline_snapshot = Some(self.build_active_layered_ifcx(bim_project, members));
        self.ifcx_baseline_label = label.to_string();
        self.refresh_ifcx_visual_diff(bim_project, members);
    }

    /// Evaluates the multi-layer semantic & geometric diff against the captured baseline snapshot.
    pub fn refresh_ifcx_visual_diff(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
    ) {
        if bim_project.is_empty() {
            self.last_ifcx_visual_diff = None;
            return;
        }
        if self.ifcx_baseline_snapshot.is_none() {
            if let Some(ref pre) = self.pre_retrofit_baseline {
                self.ifcx_baseline_snapshot = Some(crate::core::ifcx::IfcxDocument::from_bim_project(
                    pre, None, 0.0, None, None, None,
                ));
                self.ifcx_baseline_label = "Pre-Retrofit Baseline".to_string();
            } else {
                let site = self.active_site_coupling();
                self.ifcx_baseline_snapshot = Some(crate::core::ifcx::IfcxDocument::from_bim_project(
                    bim_project,
                    Some((
                        self.ground_origin_lat_lon.0,
                        self.ground_origin_lat_lon.1,
                        site.elevation_m as f64,
                    )),
                    0.0,
                    None,
                    None,
                    None,
                ));
                self.ifcx_baseline_label = "Base BIM Layer".to_string();
            }
        }
        let target_doc = self.build_active_layered_ifcx(bim_project, members);
        if let Some(ref base_doc) = self.ifcx_baseline_snapshot {
            self.last_ifcx_visual_diff = Some(crate::core::ifcx::IfcxStage::compute_visual_diff(
                base_doc,
                &self.ifcx_baseline_label,
                &target_doc,
                "Active Layered Stage",
            ));
        }
    }

    /// Evaluates autonomous multi-building portfolio `EigenGate` retrofit Pareto optimization.
    pub fn refresh_portfolio_retrofit(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
    ) {
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let site = self.active_site_coupling();
        let rep = crate::core::portfolio_retrofit::optimize_district_portfolio_retrofit(
            &self.world_index,
            bim_project,
            self.active_world_project_rel_path.as_deref(),
            members,
            origin,
            &site,
            &self.portfolio_retrofit_config,
        );
        self.portfolio_retrofit_config.active_frontier_index = rep.selected_frontier_idx;
        self.last_portfolio_retrofit_report = Some(rep);
    }

    /// Runs Phase 76 Live SHM Digital Twin, Modal Fingerprint Identification & Automated ATC-20 Placard Triage.
    pub fn run_shm_digital_twin(&mut self, bim_project: &BimProject) -> Result<String, String> {
        let site = self.active_site_coupling();
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let report = crate::core::shm_twin::evaluate_shm_digital_twin_for_bim(
            bim_project,
            &site,
            Some(&self.subsurface_ssi),
            self.last_subsurface_ssi_report.as_ref(),
            self.last_ground_motion_replay_report.as_ref(),
            Some(&self.world_index),
            &self.shm_twin_config,
            origin,
        )?;
        let summary = report.summary.clone();
        self.last_shm_twin_report = Some(report);
        Ok(summary)
    }

    /// Runs Phase 77 Autonomous Multi-Hazard Evacuation Routing & Emergency Lifelines evaluation.
    pub fn run_evacuation_lifeline_routing(
        &mut self,
        bim_project: &BimProject,
    ) -> Result<String, String> {
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        self.evacuation_config.overlay_visible = true;
        let report = crate::core::evacuation_routing::evaluate_evacuation_and_lifelines(
            bim_project,
            &self.city_scene,
            self.last_shm_twin_report.as_ref(),
            Some(&self.district_hazard),
            Some(&self.district_hazard_report),
            self.last_subsurface_ssi_report.as_ref(),
            self.active_site_tin.as_ref(),
            Some(&self.world_index),
            &self.evacuation_config,
            origin,
        )?;
        let summary = report.summary.clone();
        self.last_evacuation_report = Some(report);
        Ok(summary)
    }

    /// Steps the Phase 78 Live WebSocket / Simulator SHM Watchdog and triggers closed-loop
    /// ATC-20 placard evaluation + A* evacuation rerouting on structural warning/critical triggers.
    pub fn tick_live_shm_watchdog(
        &mut self,
        dt_s: f64,
        bim_project: &BimProject,
    ) -> crate::core::shm_watchdog::ShmWatchdogTickOutcome {
        let base_f1 = self
            .last_shm_twin_report
            .as_ref()
            .map(|r| r.baseline_freq_1_hz)
            .unwrap_or(2.25);
        let num_stories = bim_project.levels.len().saturating_sub(1).max(1);
        let outcome = self.shm_watchdog.step_live_tick(dt_s, base_f1, num_stories);

        if outcome.should_refresh_shm && !bim_project.is_empty() {
            if let Some(custom_log) = self.shm_watchdog.to_custom_telemetry_log() {
                self.shm_twin_config.custom_telemetry = Some(custom_log);
                self.shm_twin_config.fault_scenario =
                    crate::core::shm_twin::ShmFaultScenario::CustomTelemetryStream;
            }
            if self.run_shm_digital_twin(bim_project).is_ok() {
                let placard = self
                    .last_shm_twin_report
                    .as_ref()
                    .map(|r| r.atc20_placard);
                let mut rerouted = false;
                if outcome.should_reroute_evacuation
                    && placard.is_some_and(|p| p >= crate::core::shm_twin::Atc20Placard::RestrictedYellow)
                    && self.run_evacuation_lifeline_routing(bim_project).is_ok()
                {
                    rerouted = true;
                    self.shm_watchdog.auto_reroute_count += 1;
                }
                if let Some(last_ev) = self.shm_watchdog.events.last_mut() {
                    last_ev.triggered_atc20 = placard;
                    last_ev.triggered_evac_reroute = rerouted;
                }
            }
        }
        outcome
    }

    /// Runs FEMA P-58 Monte Carlo fragility assessment using hardware WebGPU `@compute` when
    /// available on native GPUs, falling back seamlessly to deterministic CPU parity on CI/WASM.
    pub fn run_monte_carlo_fragility_assessment(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::gpu_fragility_mc::MonteCarloFragilityReport {
        let replay_opt = self.last_ground_motion_replay_report.as_ref();
        let haz_opt = self
            .district_hazard
            .any_active()
            .then_some(&self.district_hazard_report);
        let ssi_opt = self.last_subsurface_ssi_report.as_ref();
        let report = self
            .webgpu_earth
            .try_run_mc_on_gpu(
                bim_project,
                replay_opt,
                haz_opt,
                ssi_opt,
                &self.monte_carlo_config,
            )
            .unwrap_or_else(|| {
                crate::core::gpu_fragility_mc::run_fema_p58_monte_carlo_cpu(
                    bim_project,
                    replay_opt,
                    haz_opt,
                    ssi_opt,
                    &self.monte_carlo_config,
                )
            });
        self.last_monte_carlo_report = Some(report.clone());
        report
    }

    /// Evaluates the spatial IDW District Risk Field Heatmap draped over `IfcSite` TIN terrain.
    pub fn refresh_district_risk_field(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
    ) -> crate::core::ifcx_collab::DistrictRiskFieldReport {
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let rep = crate::core::ifcx_collab::evaluate_district_risk_field(
            &self.collab_state.config,
            origin,
            bim_project,
            members,
            &self.world_index,
            self.active_site_tin.as_ref(),
            self.last_subsurface_ssi_report.as_ref(),
            self.last_shm_twin_report.as_ref(),
            self.last_monte_carlo_report.as_ref(),
            self.last_evacuation_report.as_ref(),
        );
        self.last_risk_field_report = Some(rep.clone());
        rep
    }

    /// Captures a SHA-256 chained IFCX governance checkpoint of all active layers + KPI metrics.
    pub fn create_signed_ifcx_checkpoint(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
        label: &str,
    ) -> String {
        let doc = self.build_active_layered_ifcx(bim_project, members);
        let min_eigengate = members.iter().map(|m| m.eigengate).fold(1.0_f64, f64::min);
        let max_dcr_pct = members.iter().map(|m| m.utilization * 100.0).fold(0.0_f64, f64::max);
        let atc20_placard = self
            .last_shm_twin_report
            .as_ref()
            .map(|s| s.atc20_placard.short_label().to_string())
            .unwrap_or_else(|| "GREEN".to_string());
        let mc_pml90_usd = self
            .last_monte_carlo_report
            .as_ref()
            .map(|m| m.p90_loss_usd)
            .unwrap_or(0.0);
        let evacuation_conn_pct = self
            .last_evacuation_report
            .as_ref()
            .map(|e| e.network_connectivity_pct)
            .unwrap_or(100.0);
        let kpi = crate::core::ifcx_collab::IfcxCheckpointKpi {
            min_eigengate,
            max_dcr_pct,
            atc20_placard,
            mc_pml90_usd,
            evacuation_conn_pct,
            node_count: doc.data.len(),
        };
        let disc = self.collab_state.config.active_discipline;
        let cp = self.collab_state.create_checkpoint(label, disc, doc, kpi);
        format!(
            "Created {} '{}' ({}) • Chain SHA-256: {}",
            cp.checkpoint_id,
            cp.label,
            cp.discipline.short_label(),
            cp.short_sha256()
        )
    }

    /// Runs a deterministic Three-Way IFCX Merge between the base checkpoint, current active layer,
    /// and a multi-discipline peer branch revision.
    pub fn run_peer_three_way_ifcx_merge(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
    ) -> String {
        let local_doc = self.build_active_layered_ifcx(bim_project, members);
        if self.collab_state.checkpoints.is_empty() {
            let _ = self.create_signed_ifcx_checkpoint(bim_project, members, "Merge Base");
        }
        let (base_doc, base_lbl) = match self.collab_state.checkpoints.first() {
            Some(cp) => (cp.snapshot.clone(), cp.checkpoint_id.clone()),
            None => (local_doc.clone(), "Base".to_string()),
        };
        let peer_disc = self.collab_state.peer_discipline;
        let peer_doc = crate::core::ifcx_collab::synthesize_peer_branch_doc(&base_doc, peer_disc);
        let mut local_edited = local_doc.clone();
        if let Some(node) = local_edited
            .data
            .iter_mut()
            .find(|n| n.attributes.contains_key(crate::core::ifcx::ns::EG_BIM_PROJECT) || n.attributes.contains_key(crate::core::ifcx::ns::BSI_PROP))
        {
            let mut prop = node
                .attributes
                .get(crate::core::ifcx::ns::BSI_PROP)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(obj) = prop.as_object_mut() {
                obj.entry("peer_review_note".to_string())
                    .or_insert_with(|| serde_json::json!("Local Structural: ductile boundary verified"));
                obj.entry("discipline_revision".to_string())
                    .or_insert_with(|| serde_json::json!("STRUCT-R2"));
            }
            node.attributes
                .insert(crate::core::ifcx::ns::BSI_PROP.to_string(), prop);
        }
        let merge = crate::core::ifcx_collab::three_way_merge_ifcx(
            &base_doc,
            &local_edited,
            &peer_doc,
            &base_lbl,
            "Local Structural",
            &self.collab_state.peer_branch_label,
        );
        let msg = merge.summary.clone();
        self.collab_state.last_merge = Some(merge);
        msg
    }

    /// Evaluates the 6-scenario Parametric Trade-Study Matrix (`EigenGate`, DCR%, Drift, Cost, CO2, PML-90).
    pub fn run_trade_study_analysis(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
    ) -> crate::core::trade_study::TradeStudyReport {
        let site = self.active_site_coupling();
        let rep = crate::core::trade_study::run_parametric_trade_study(
            bim_project,
            members,
            &site,
            self.last_subsurface_ssi_report.as_ref(),
            self.last_monte_carlo_report.as_ref(),
            &self.trade_study_config,
        );
        self.last_trade_study_report = Some(rep.clone());
        rep
    }

    /// Evaluates the 10-clause SNI 1726 / SNI 2847 / ASCE 7 / UFC 4-023-03 Permit Compliance Matrix.
    pub fn refresh_permit_compliance(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
    ) -> crate::core::trade_study::PermitComplianceReport {
        let site = self.active_site_coupling();
        let haz_opt = self
            .district_hazard
            .any_active()
            .then_some(&self.district_hazard_report);
        let rep = crate::core::trade_study::evaluate_permit_compliance_matrix(
            bim_project,
            members,
            &site,
            self.last_subsurface_ssi_report.as_ref(),
            haz_opt,
            self.last_shm_twin_report.as_ref(),
            self.last_evacuation_report.as_ref(),
            self.last_monte_carlo_report.as_ref(),
            Some(&self.collab_state),
        );
        self.last_permit_report = Some(rep.clone());
        rep
    }

    /// Evaluates Rebar BBS Cut-List Optimization (1D BFD) and Field QA/QC spatial waypoints.
    pub fn run_field_fabrication_audit(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::field_fabrication::FieldFabricationReport {
        let rep = crate::core::field_fabrication::evaluate_field_fabrication(
            bim_project,
            &self.field_qaqc_ledger,
            &self.field_fab_config,
        );
        self.last_field_fab_report = Some(rep.clone());
        rep
    }

    /// Evaluates Branch 2 EigenFlow Non-Linear Woodbury (`P1 -> P2`) & Pushover ADRS Capacity Spectrum.
    pub fn run_eigenflow_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::eigenflow::EigenFlowStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::eigenflow::evaluate_eigenflow_studio(
            bim_project,
            Some(&site),
            &self.eigenflow_config,
        );
        let n_steps = rep.pushover_csm.raw_pushover.steps.len().max(1);
        self.eigenflow_config.active_step_idx =
            self.eigenflow_config.active_step_idx.min(n_steps.saturating_sub(1));
        self.last_eigenflow_report = Some(rep.clone());
        rep
    }

    /// Evaluates Phase 84 Extreme Events (Blast `UFC 3-340-02` & Fire `ISO 834/EC`) + EigenFlow coupling.
    pub fn run_extreme_events_audit(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::extreme_events::ExtremeEventsReport {
        let site = self.active_site_coupling();
        let rep = crate::core::extreme_events::evaluate_extreme_events(
            bim_project,
            Some(&site),
            &self.extreme_events_config,
        );
        self.last_extreme_events_report = Some(rep.clone());
        rep
    }

    /// Evaluates Seismic Base Isolation & Damper (`LRB`, `HDRB`, `FPS`, `FVD`) coupled to active BIM & site.
    pub fn run_isolation_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::base_isolation_studio::IsolationStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::base_isolation_studio::evaluate_isolation_studio(
            bim_project,
            Some(&site),
            &self.isolation_studio_config,
        );
        self.last_isolation_report = Some(rep.clone());
        rep
    }

    /// Evaluates Wind Aeroelasticity, Facade Cladding Pressure & ISO 10137 Comfort coupled to BIM & site.
    pub fn run_wind_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::wind_studio::WindStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::wind_studio::evaluate_wind_studio(
            bim_project,
            Some(&site),
            &self.wind_studio_config,
        );
        self.last_wind_report = Some(rep.clone());
        rep
    }

    /// Evaluates Special RC Shear Wall, SBE & Diagonal Coupling Beam coupled to BIM & site.
    pub fn run_shear_wall_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::shear_wall_studio::ShearWallStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::shear_wall_studio::evaluate_shear_wall_studio(
            bim_project,
            Some(&site),
            &self.shear_wall_studio_config,
        );
        self.last_shear_wall_report = Some(rep.clone());
        rep
    }

    /// Evaluates Steel Seismic Connection, High-Strength Bolts & Panel Zone coupled to BIM & site.
    pub fn run_connection_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::connection_studio::ConnectionStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::connection_studio::evaluate_connection_studio(
            bim_project,
            Some(&site),
            &self.connection_studio_config,
        );
        self.last_connection_report = Some(rep.clone());
        rep
    }

    /// Evaluates Post-Tensioned / Prestressed Concrete Tendon Drape, Losses & Camber coupled to BIM & site.
    pub fn run_prestress_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::prestress_studio::PrestressStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::prestress_studio::evaluate_prestress_studio(
            bim_project,
            Some(&site),
            &self.prestress_studio_config,
        );
        self.last_prestress_report = Some(rep.clone());
        rep
    }

    /// Evaluates Composite Deck, Headed Shear Studs & AISC DG11 Vibration coupled to BIM & site.
    pub fn run_composite_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::composite_studio::CompositeStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::composite_studio::evaluate_composite_studio(
            bim_project,
            Some(&site),
            &self.composite_studio_config,
        );
        self.last_composite_report = Some(rep.clone());
        rep
    }

    /// Evaluates Structural Reliability (`FORM/SORM` beta-index, Sensitivity & FEMA P-58 Fragility) coupled to BIM & site.
    pub fn run_reliability_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::reliability_studio::ReliabilityStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::reliability_studio::evaluate_reliability_studio(
            bim_project,
            Some(&site),
            &self.reliability_studio_config,
        );
        self.last_reliability_report = Some(rep.clone());
        rep
    }

    /// Evaluates Deep Foundation Pile Group, 6-DOF Winkler Springs & Punching Shear coupled to BIM & site.
    pub fn run_foundation_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::foundation_studio::FoundationStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::foundation_studio::evaluate_foundation_studio(
            bim_project,
            Some(&site),
            &self.foundation_studio_config,
        );
        self.last_foundation_report = Some(rep.clone());
        rep
    }

    /// Evaluates 4D CPM Sequence, 5D AHSP Cost S-Curve & 6D EN 15978 Carbon LCA coupled to BIM & site.
    pub fn run_construction_lca_studio(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::construction_lca_studio::ConstructionLcaStudioReport {
        let site = self.active_site_coupling();
        let rep = crate::core::construction_lca_studio::evaluate_construction_lca_studio(
            bim_project,
            Some(&site),
            &self.construction_lca_config,
        );
        self.last_construction_lca_report = Some(rep.clone());
        rep
    }

    /// Evaluates Site Zoning (`KDB/KLB/KDH/GSB`), 3D Buildable Envelope & Sun Path coupled to BIM & site.
    pub fn run_zoning_agent(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::zoning_agent::SiteZoningReport {
        let site = self.active_site_coupling();
        let rep = crate::core::zoning_agent::evaluate_site_zoning(
            bim_project,
            Some(&site),
            &self.zoning_config,
        );
        self.last_zoning_report = Some(rep.clone());
        rep
    }

    /// Evaluates Spatial Programming, Adjacency Matrix & 3D Stack/Pack coupled to Site Zoning envelope.
    pub fn run_spatial_program(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::program_agent::SpatialProgramReport {
        if self.last_zoning_report.is_none() {
            let _ = self.run_zoning_agent(bim_project);
        }
        let rep = crate::core::program_agent::evaluate_spatial_program(
            self.last_zoning_report.as_ref(),
            &self.program_config,
        );
        self.last_program_report = Some(rep.clone());
        rep
    }

    /// Evaluates Multi-Option Massing Explorer coupled to Site Zoning, Spatial Program & Earth Hazards.
    pub fn run_massing_study(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::massing_agent::MassingStudyReport {
        let zoning = self.run_zoning_agent(bim_project);
        let program = self.run_spatial_program(bim_project);
        let site = self.active_site_coupling();
        let rep = crate::core::massing_agent::evaluate_massing_options(
            &zoning,
            &program,
            &site,
            &self.massing_config,
        );
        self.last_massing_report = Some(rep.clone());
        rep
    }

    /// Executes the chained 6-stage AI Building Design Pipeline from a natural-language brief/RFP.
    pub fn run_design_pipeline(
        &mut self,
        prompt: &str,
        base_bim: &BimProject,
    ) -> (
        crate::core::design_agent::DesignPipelineReport,
        Option<BimProject>,
    ) {
        let site = self.active_site_coupling();
        let (rep, gen_bim) = crate::core::design_agent::execute_chained_design_pipeline(
            prompt,
            base_bim,
            &site,
            &self.zoning_config,
            &self.program_config,
            &self.massing_config,
        );
        self.zoning_config = rep.zoning_config.clone();
        self.program_config = rep.program_config.clone();
        self.massing_config = rep.massing_config.clone();
        self.last_zoning_report = Some(rep.zoning_report.clone());
        self.last_program_report = Some(rep.program_report.clone());
        self.last_massing_report = Some(rep.massing_report.clone());
        self.last_design_pipeline_report = Some(rep.clone());
        (rep, gen_bim)
    }

    /// Evaluates 3D Solar Irradiation, Cast Ground Shadows & SNI 6389 / ASHRAE 90.1 OTTV Envelope.
    pub fn run_solar_envelope(
        &mut self,
        bim_project: &BimProject,
    ) -> crate::core::solar_envelope::SolarEnvelopeReport {
        let site = self.active_site_coupling();
        let rep = crate::core::solar_envelope::evaluate_solar_envelope(
            bim_project,
            Some(&site),
            self.last_massing_report.as_ref(),
            &self.solar_envelope_config,
        );
        self.last_solar_envelope_report = Some(rep.clone());
        rep
    }

    /// Synchronizes an incoming peer `.ifcx` delta through the collaborative sync relay buffer
    /// (auto-synthesizing a discipline peer delta if the inbound relay queue is empty).
    pub fn simulate_peer_relay_sync(
        &mut self,
        bim_project: &BimProject,
        members: &[StructuralMember],
    ) -> String {
        let local_doc = self.build_active_layered_ifcx(bim_project, members);
        if self.collab_state.checkpoints.is_empty() {
            let _ = self.create_signed_ifcx_checkpoint(bim_project, members, "Relay Sync Base");
        }
        let base_doc = self
            .collab_state
            .checkpoints
            .first()
            .map(|c| c.snapshot.clone())
            .unwrap_or_else(|| local_doc.clone());

        let mut events = crate::core::web_bridge::collab_relay_poll_events();
        if events.is_empty() {
            let peer_doc = crate::core::ifcx_collab::synthesize_peer_branch_doc(
                &base_doc,
                self.collab_state.peer_discipline,
            );
            crate::core::web_bridge::collab_relay_inject_event(
                crate::core::web_bridge::CollabRelayEvent {
                    peer_author: self.collab_state.peer_branch_label.clone(),
                    discipline: self.collab_state.peer_discipline.short_label().to_string(),
                    checkpoint_id: "RELAY-SYNC".to_string(),
                    payload_json: peer_doc.to_json_pretty(),
                },
            );
            events = crate::core::web_bridge::collab_relay_poll_events();
        }
        let mut last_msg = "No incoming relay deltas.".to_string();
        for ev in events {
            if let Ok(incoming) = crate::core::ifcx::IfcxDocument::parse(&ev.payload_json) {
                last_msg = self.collab_state.ingest_relay_peer_doc(
                    &base_doc,
                    &local_doc,
                    &ev.peer_author,
                    self.collab_state.peer_discipline,
                    &incoming,
                );
            }
        }
        last_msg
    }

    /// Builds frame parameters for the Pure WebGPU `@compute` 60 FPS multi-hazard particle pass.
    pub fn build_hazard_compute_input(
        &self,
        bim_project: &BimProject,
        time_secs: f32,
    ) -> crate::views::webgpu_earth::HazardComputeInput {
        use crate::views::webgpu_earth::{
            HazardComputeInput, HazardComputeParams, HazardObstacleGpu, MAX_GPU_HAZARD_OBSTACLES,
        };
        let enabled = self.monte_carlo_config.gpu_particles_enabled && self.world_map.zoom_level >= 11.5;
        if !enabled {
            return HazardComputeInput::default();
        }
        let mode_flags = match (
            self.district_hazard.seismic_wave_active,
            self.district_hazard.venturi_wind_active,
        ) {
            (true, true) => 3.0_f32,
            (true, false) => 1.0_f32,
            (false, true) => 2.0_f32,
            (false, false) => 3.0_f32,
        };
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let (bmin, bmax) = bim_project
            .plan_extents_mm()
            .unwrap_or(([-7000.0, -6000.0], [7000.0, 6000.0]));
        let active_cx = ((bmin[0] + bmax[0]) * 0.5e-3) as f32;
        let active_cy = ((bmin[1] + bmax[1]) * 0.5e-3) as f32;
        let active_hw = ((bmax[0] - bmin[0]).max(4000.0) * 0.5e-3) as f32;
        let active_hd = ((bmax[1] - bmin[1]).max(4000.0) * 0.5e-3) as f32;

        let mut obstacles = Vec::with_capacity(16);
        obstacles.push(HazardObstacleGpu {
            xy_r_h: [active_cx, active_cy, active_hw.hypot(active_hd).max(8.0), 18.0],
        });
        for b in &self.world_index.buildings {
            if obstacles.len() >= MAX_GPU_HAZARD_OBSTACLES || b.is_unlocated() {
                break;
            }
            let [wx_mm, wy_mm] = crate::core::gis::SlippyMapState::lat_lon_to_world_mm(
                origin, b.lat, b.lon,
            );
            let ex = (wx_mm * 1e-3) as f32;
            let ny = (wy_mm * 1e-3) as f32;
            if ex.hypot(ny) <= 260.0 {
                let rad = b
                    .footprint_local_m
                    .iter()
                    .map(|p| p[0].hypot(p[1]))
                    .fold(8.0_f64, f64::max)
                    .clamp(6.0, 45.0) as f32;
                obstacles.push(HazardObstacleGpu {
                    xy_r_h: [ex, ny, rad, b.total_height_m.max(6.0) as f32],
                });
            }
        }
        let vs30 = self
            .last_subsurface_ssi_report
            .as_ref()
            .map(|s| s.vs30_m_s as f32)
            .unwrap_or(280.0);
        HazardComputeInput {
            enabled: true,
            params: HazardComputeParams {
                time_secs,
                pga_g: self.district_hazard.seismic_pga_g as f32,
                wave_speed_ms: self.district_hazard.wave_speed_ms as f32,
                wave_azimuth_rad: (self.district_hazard.wave_azimuth_deg as f32).to_radians(),
                wind_speed_ms: self.district_hazard.base_wind_ms as f32,
                wind_azimuth_rad: (self.district_hazard.wind_azimuth_deg as f32).to_radians(),
                vs30_ms: vs30,
                particle_count_f32: self.monte_carlo_config.normalized_particle_count() as f32,
                mode_flags_f32: mode_flags,
                domain_radius_m: 140.0,
                obstacle_count_f32: obstacles.len() as f32,
                intensity_scale: 1.0,
                active_cx,
                active_cy,
                active_half_w: active_hw,
                active_half_d: active_hd,
            },
            obstacles,
        }
    }

    /// Launches a non-blocking background Ground-Motion Replay & Async `StructGNN-CRC` worker job.
    pub fn start_async_ground_motion_replay(&mut self, bim_project: &BimProject) {
        let site = self.active_site_coupling();
        let ssi_opt = if self.ground_motion_replay_config.use_ssi_period_coupling {
            self.last_subsurface_ssi_report.clone()
        } else {
            None
        };
        self.replay_async_manager.start_job(
            bim_project.clone(),
            site,
            ssi_opt,
            self.ground_motion_replay_config.clone(),
        );
    }

    /// Polls the active background replay job non-blockingly and commits completed reports.
    pub fn poll_async_ground_motion_replay(&mut self) -> Option<Result<String, String>> {
        let res = self.replay_async_manager.poll()?;
        match res {
            Ok(rep) => {
                let summary = rep.summary.clone();
                if self.replay_anim_playing {
                    self.replay_cursor_time_s = 0.0;
                } else {
                    self.replay_cursor_time_s = rep.peak_time_s;
                }
                self.last_ground_motion_replay_report = Some(rep);
                Some(Ok(summary))
            }
            Err(err) => Some(Err(err)),
        }
    }

    /// Advances the 60 FPS live earthquake playback cursor smoothly by `dt_s`.
    pub fn advance_ground_motion_replay_clock(&mut self, dt_s: f64) {
        let Some(ref rep) = self.last_ground_motion_replay_report else {
            return;
        };
        if !self.replay_anim_playing || rep.total_duration_s <= 0.05 {
            return;
        }
        let step = dt_s.clamp(0.0, 0.05) * (self.ground_motion_replay_config.playback_speed as f64);
        self.replay_cursor_time_s += step;
        if self.replay_cursor_time_s > rep.total_duration_s {
            if self.ground_motion_replay_config.replay_loop {
                self.replay_cursor_time_s = 0.0;
            } else {
                self.replay_cursor_time_s = rep.total_duration_s;
                self.replay_anim_playing = false;
            }
        }
    }

    /// Samples the interpolated earthquake replay frame at the active playback cursor time.
    pub fn current_replay_frame(
        &self,
    ) -> Option<crate::core::ground_motion_replay::GroundMotionReplayFrame> {
        self.last_ground_motion_replay_report
            .as_ref()
            .map(|rep| rep.sample_interpolated(self.replay_cursor_time_s))
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


    // Methods split across cad_ops.rs, interaction.rs, render_layers.rs, and toolbars.rs
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
            city_scene: self.city_scene.clone(),
            district_hazard: self.district_hazard.clone(),
            subsurface_ssi: self.subsurface_ssi.clone(),
            portfolio_retrofit: self.portfolio_retrofit_config.clone(),
            ground_motion_replay: self.ground_motion_replay_config.clone(),
            shm_digital_twin: self.shm_twin_config.clone(),
            evacuation_routing: self.evacuation_config.clone(),
            shm_watchdog: self.shm_watchdog.config.clone(),
            monte_carlo_config: self.monte_carlo_config.clone(),
            collab_config: self.collab_state.config.clone(),
            trade_study_config: self.trade_study_config.clone(),
            eigenflow_config: self.eigenflow_config.clone(),
            extreme_events_config: self.extreme_events_config.clone(),
            isolation_studio_config: self.isolation_studio_config.clone(),
            wind_studio_config: self.wind_studio_config.clone(),
            shear_wall_studio_config: self.shear_wall_studio_config.clone(),
            connection_studio_config: self.connection_studio_config.clone(),
            prestress_studio_config: self.prestress_studio_config.clone(),
            composite_studio_config: self.composite_studio_config.clone(),
            reliability_studio_config: self.reliability_studio_config.clone(),
            foundation_studio_config: self.foundation_studio_config.clone(),
            construction_lca_config: self.construction_lca_config.clone(),
            zoning_config: self.zoning_config.clone(),
            program_config: self.program_config.clone(),
            massing_config: self.massing_config.clone(),
            solar_envelope_config: self.solar_envelope_config.clone(),
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
        self.city_scene = archive.city_scene;
        self.district_hazard = archive.district_hazard;
        if self.district_hazard.any_active() {
            self.refresh_district_hazard(bim_project);
        } else {
            self.district_hazard_report = crate::core::gis::DistrictMultiHazardReport::default();
        }
        self.subsurface_ssi = archive.subsurface_ssi;
        if self.subsurface_ssi.ssi_coupling_enabled || self.subsurface_ssi.xray_subsurface_visible {
            self.refresh_subsurface_ssi(bim_project);
        } else {
            self.last_subsurface_ssi_report = None;
        }
        self.portfolio_retrofit_config = archive.portfolio_retrofit;
        if self.portfolio_retrofit_config.overlay_visible {
            self.refresh_portfolio_retrofit(bim_project, members);
        } else {
            self.last_portfolio_retrofit_report = None;
        }
        self.ground_motion_replay_config = archive.ground_motion_replay;
        self.shm_twin_config = archive.shm_digital_twin;
        self.last_shm_twin_report = None;
        self.evacuation_config = archive.evacuation_routing;
        self.last_evacuation_report = None;
        self.shm_watchdog = crate::core::shm_watchdog::ShmWatchdogState {
            config: archive.shm_watchdog,
            ..Default::default()
        };
        self.monte_carlo_config = archive.monte_carlo_config;
        self.last_monte_carlo_report = None;
        self.collab_state.config = archive.collab_config;
        self.trade_study_config = archive.trade_study_config;
        self.last_trade_study_report = None;
        self.last_permit_report = None;
        self.eigenflow_config = archive.eigenflow_config;
        self.last_eigenflow_report = None;
        self.extreme_events_config = archive.extreme_events_config;
        self.last_extreme_events_report = None;
        self.isolation_studio_config = archive.isolation_studio_config;
        self.last_isolation_report = None;
        self.wind_studio_config = archive.wind_studio_config;
        self.last_wind_report = None;
        self.shear_wall_studio_config = archive.shear_wall_studio_config;
        self.last_shear_wall_report = None;
        self.connection_studio_config = archive.connection_studio_config;
        self.last_connection_report = None;
        self.prestress_studio_config = archive.prestress_studio_config;
        self.last_prestress_report = None;
        self.composite_studio_config = archive.composite_studio_config;
        self.last_composite_report = None;
        self.reliability_studio_config = archive.reliability_studio_config;
        self.last_reliability_report = None;
        self.foundation_studio_config = archive.foundation_studio_config;
        self.last_foundation_report = None;
        self.construction_lca_config = archive.construction_lca_config;
        self.last_construction_lca_report = None;
        self.zoning_config = archive.zoning_config;
        self.last_zoning_report = None;
        self.program_config = archive.program_config;
        self.last_program_report = None;
        self.massing_config = archive.massing_config;
        self.last_massing_report = None;
        self.last_design_pipeline_report = None;
        self.solar_envelope_config = archive.solar_envelope_config;
        self.last_solar_envelope_report = None;
        if self.collab_state.config.risk_heatmap_visible {
            let _ = self.refresh_district_risk_field(bim_project, members);
        } else {
            self.last_risk_field_report = None;
        }
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

        // 1a. Phase 64–75: 100% Pure WebGPU (`wgpu` / WGSL) 3D Pure-Vector IFCX Earth + City-Builder + Unified BIM/FEM/Modal/N-1
        let active_globe_xform = if self.globe_mode {
            let active_heading_deg = self
                .active_world_project_rel_path
                .as_deref()
                .and_then(|r| self.world_index.get(r))
                .map(|b| b.heading_deg)
                .unwrap_or(0.0);
            let modal_phase_sin = if self.earth_bim_author.modal_anim_playing {
                ((ui.ctx().input(|i| i.time) * 3.8).sin()) as f32
            } else {
                1.0_f32
            };
            if self.district_hazard.any_active() {
                self.district_hazard.sim_time_sec = ui.ctx().input(|i| i.time);
                if self.district_hazard_report.streamlines.is_empty() {
                    self.refresh_district_hazard(bim_project);
                }
            }
            let active_bldg_h_m = bim_project
                .levels
                .iter()
                .map(|l| l.elevation_mm * 0.001)
                .fold(0.0_f64, f64::max)
                .max(6.0);
            let hazard_site_origin = if self.ground_origin_anchored {
                self.ground_origin_lat_lon
            } else {
                self.world_map.center_lat_lon
            };
            let hazard_gpu_input = if self.district_hazard.any_active() {
                Some(crate::views::webgpu_earth::DistrictHazardGpuInput {
                    config: &self.district_hazard,
                    report: &self.district_hazard_report,
                    neighborhood: &self.neighborhood_report,
                    origin_lat_lon: hazard_site_origin,
                    active_building_height_m: active_bldg_h_m,
                    active_site_tin: self.active_site_tin.as_ref(),
                })
            } else {
                None
            };
            let n1_removed_bim = self
                .earth_fem_cache
                .n1_simulation
                .as_ref()
                .map(|ov| (ov.removed_kind, ov.removed_bim_id));
            let (cascade_failed_bim, cascade_step_progress) =
                if let Some(ref crc) = self.last_crc_cascade_report {
                    let step = self.active_cascade_step.clamp(1, crc.total_steps.max(1));
                    let progress = if crc.total_steps > 0 {
                        (step as f32 / crc.total_steps as f32).clamp(0.15, 1.0)
                    } else {
                        1.0
                    };
                    (crc.cumulative_failed_bim_up_to_step(step), progress)
                } else {
                    (Vec::new(), 0.0_f32)
                };
            let subsurface_pile_depth_m = self
                .last_subsurface_ssi_report
                .as_ref()
                .map(|r| r.recommended_pile_depth_m as f32)
                .unwrap_or(match self.subsurface_ssi.borehole_preset {
                    crate::core::geotech::BoreholePreset::AlluvialSoftClay => 22.0,
                    crate::core::geotech::BoreholePreset::CoastalLiquefiableSand => 18.0,
                    crate::core::geotech::BoreholePreset::StiffVolcanicAlluvium => 10.0,
                    crate::core::geotech::BoreholePreset::DenseGravelBedrock => 4.5,
                });
            let (diff_added, diff_modified, diff_deleted_ghosts): (
                Vec<(crate::core::BimElementKind, usize)>,
                Vec<(crate::core::BimElementKind, usize)>,
                Vec<([f64; 3], [f64; 3], String)>,
            ) = if self.ifcx_diff_overlay_active {
                if let Some(ref d) = self.last_ifcx_visual_diff {
                    (
                        d.added_bim_elements.clone(),
                        d.modified_bim_elements.clone(),
                        d.deleted_ghost_boxes_m.clone(),
                    )
                } else {
                    (Vec::new(), Vec::new(), Vec::new())
                }
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            };
            let replay_frame_opt = if self.earth_bim_author.active_lens == EarthBimLens::TimeHistoryReplay {
                self.current_replay_frame()
            } else {
                None
            };
            let replay_elevs: Vec<f64> = replay_frame_opt
                .as_ref()
                .map(|f| f.story_elevations_m.clone())
                .unwrap_or_default();
            let replay_disp_triples: Vec<[f64; 3]> = replay_frame_opt
                .as_ref()
                .map(|f| {
                    (0..f.story_elevations_m.len())
                        .map(|i| {
                            [
                                f.story_disp_x_m.get(i).copied().unwrap_or(0.0),
                                f.story_disp_y_m.get(i).copied().unwrap_or(0.0),
                                f.story_disp_z_m.get(i).copied().unwrap_or(0.0),
                            ]
                        })
                        .collect()
                })
                .unwrap_or_default();
            let replay_ground_shake = replay_frame_opt
                .as_ref()
                .map(|f| [f.ground_disp_m, f.ground_disp_y_m, f.ground_disp_z_m])
                .unwrap_or([0.0, 0.0, 0.0]);
            let replay_hinge_states: Vec<(
                crate::core::BimElementKind,
                usize,
                crate::core::nonlinear::PlasticHingeState,
            )> = if self.earth_bim_author.active_lens == EarthBimLens::TimeHistoryReplay {
                self.last_ground_motion_replay_report
                    .as_ref()
                    .map(|rep| rep.worst_hinge_states_at(self.replay_cursor_time_s))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let shm_story_retention: Vec<(f64, f64)> =
                if self.earth_bim_author.active_lens == EarthBimLens::ShmDigitalTwin {
                    self.last_shm_twin_report
                        .as_ref()
                        .map(|r| r.story_retention_pairs())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
            let active_bim_gpu = if !bim_project.is_empty() {
                Some(crate::views::webgpu_earth::ActiveBimGpuInput {
                    project: bim_project,
                    members,
                    lens: self.earth_bim_author.active_lens,
                    erection_day_limit: self.earth_bim_author.erection_day_limit,
                    deform_scale: self.earth_bim_author.deform_scale,
                    active_modal_mode: self.earth_bim_author.active_modal_mode,
                    modal_phase_sin,
                    n1_removed_bim,
                    cascade_failed_bim: &cascade_failed_bim,
                    cascade_step_progress,
                    heading_deg: active_heading_deg,
                    subsurface_pile_depth_m,
                    diff_overlay_active: self.ifcx_diff_overlay_active,
                    diff_added_bim: &diff_added,
                    diff_modified_bim: &diff_modified,
                    diff_deleted_ghosts: &diff_deleted_ghosts,
                    replay_story_elevs_m: &replay_elevs,
                    replay_story_disp_xyz_m: &replay_disp_triples,
                    replay_ground_shake_xyz_m: replay_ground_shake,
                    replay_hinge_states: &replay_hinge_states,
                    shm_story_retention: &shm_story_retention,
                })
            } else {
                None
            };
            let hinge_gpu_input = if !bim_project.is_empty() {
                replay_frame_opt.as_ref().map(|fr| {
                    crate::views::webgpu_earth::PlasticHingeGpuInput {
                        project: bim_project,
                        frame: fr,
                        show_hinges: self.ground_motion_replay_config.show_plastic_hinges,
                        deform_scale: self.earth_bim_author.deform_scale,
                        heading_deg: active_heading_deg,
                    }
                })
            } else {
                None
            };
            let fragility_gpu_input = if !bim_project.is_empty()
                && self.ground_motion_replay_config.show_fragility_loss_overlay
            {
                self.last_ground_motion_replay_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::FragilityLossGpuInput {
                        project: bim_project,
                        synthesis: &rep.fragility_loss,
                        visible: true,
                        heading_deg: active_heading_deg,
                    }
                })
            } else {
                None
            };
            let shm_origin = if self.ground_origin_anchored {
                self.ground_origin_lat_lon
            } else {
                self.world_map.center_lat_lon
            };
            let shm_gpu_input = if !bim_project.is_empty()
                && self.earth_bim_author.active_lens == EarthBimLens::ShmDigitalTwin
            {
                self.last_shm_twin_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::ShmTriageGpuInput {
                        project: bim_project,
                        report: rep,
                        visible: true,
                        heading_deg: active_heading_deg,
                        site_origin: shm_origin,
                    }
                })
            } else {
                None
            };
            let subsurface_gpu_input = if self.subsurface_ssi.xray_subsurface_visible {
                Some(crate::views::webgpu_earth::SubsurfaceSsiGpuInput {
                    project: bim_project,
                    config: &self.subsurface_ssi,
                    report: self.last_subsurface_ssi_report.as_ref(),
                    heading_deg: active_heading_deg,
                })
            } else {
                None
            };
            let portfolio_origin = if self.ground_origin_anchored {
                self.ground_origin_lat_lon
            } else {
                self.world_map.center_lat_lon
            };
            let portfolio_gpu_input = if self.portfolio_retrofit_config.overlay_visible {
                self.last_portfolio_retrofit_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::PortfolioGpuOverlayInput {
                        report: rep,
                        origin_lat_lon: portfolio_origin,
                    }
                })
            } else {
                None
            };
            let evacuation_gpu_input = if self.evacuation_config.overlay_visible {
                self.last_evacuation_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::EvacuationLifelineGpuInput {
                        report: rep,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let risk_heatmap_gpu_input = if self.collab_state.config.risk_heatmap_visible {
                self.last_risk_field_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::DistrictRiskHeatmapGpuInput {
                        report: rep,
                        opacity: self.collab_state.config.risk_opacity,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let field_qaqc_gpu_input = if self.field_qaqc_overlay_visible {
                self.last_field_fab_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::FieldQaqcGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let eigenflow_gpu_input = if self.eigenflow_overlay_visible {
                self.last_eigenflow_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::EigenFlowGpuInput {
                        report: rep,
                        active_step_idx: self.eigenflow_config.active_step_idx,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let extreme_events_gpu_input = if self.extreme_events_overlay_visible {
                self.last_extreme_events_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::ExtremeEventsGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let isolation_gpu_input = if self.isolation_overlay_visible {
                self.last_isolation_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::IsolationGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let wind_gpu_input = if self.wind_overlay_visible {
                self.last_wind_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::WindGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let shear_wall_gpu_input = if self.shear_wall_overlay_visible {
                self.last_shear_wall_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::ShearWallGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let connection_gpu_input = if self.connection_overlay_visible {
                self.last_connection_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::ConnectionGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let prestress_gpu_input = if self.prestress_overlay_visible {
                self.last_prestress_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::PrestressGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let composite_gpu_input = if self.composite_overlay_visible {
                self.last_composite_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::CompositeGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let reliability_gpu_input = if self.reliability_overlay_visible {
                self.last_reliability_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::ReliabilityGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let foundation_gpu_input = if self.foundation_overlay_visible {
                self.last_foundation_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::FoundationGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let construction_lca_gpu_input = if self.construction_lca_overlay_visible {
                self.last_construction_lca_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::ConstructionLcaGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let zoning_envelope_gpu_input = if self.zoning_overlay_visible {
                self.last_zoning_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::ZoningEnvelopeGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let spatial_program_gpu_input = if self.program_overlay_visible {
                self.last_program_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::SpatialProgramGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let massing_study_gpu_input = if self.massing_overlay_visible {
                self.last_massing_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::MassingStudyGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let solar_envelope_gpu_input = if self.solar_overlay_visible {
                self.last_solar_envelope_report.as_ref().map(|rep| {
                    crate::views::webgpu_earth::SolarEnvelopeGpuInput {
                        report: rep,
                        heading_deg: active_heading_deg,
                        visible: true,
                    }
                })
            } else {
                None
            };
            let compute_time_secs = (ui.ctx().input(|i| i.time) % 3600.0) as f32;
            let hazard_compute_input =
                self.build_hazard_compute_input(bim_project, compute_time_secs);
            let hazard_compute_opt = hazard_compute_input
                .enabled
                .then_some(hazard_compute_input.clone());
            let gpu_globe_active = self.webgpu_earth.schedule_paint_pass(
                ui,
                &painter,
                rect,
                &self.earth_world,
                &self.world_map,
                self.ground_origin_lat_lon,
                self.ground_origin_anchored,
                self.show_terrain_contours,
                self.active_site_tin.as_ref(),
                &self.world_index,
                self.active_world_project_rel_path.as_deref(),
                &self.urban_vectors,
                Some(&self.city_scene),
                active_bim_gpu,
                hazard_gpu_input,
                subsurface_gpu_input,
                portfolio_gpu_input,
                hinge_gpu_input,
                fragility_gpu_input,
                shm_gpu_input,
                evacuation_gpu_input,
                risk_heatmap_gpu_input,
                field_qaqc_gpu_input,
                eigenflow_gpu_input,
                extreme_events_gpu_input,
                isolation_gpu_input,
                wind_gpu_input,
                shear_wall_gpu_input,
                connection_gpu_input,
                prestress_gpu_input,
                composite_gpu_input,
                reliability_gpu_input,
                foundation_gpu_input,
                construction_lca_gpu_input,
                zoning_envelope_gpu_input,
                spatial_program_gpu_input,
                massing_study_gpu_input,
                solar_envelope_gpu_input,
                hazard_compute_opt,
            );
            let xform = self.earth_world.render_3d_globe_with_gpu_mode(
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
                gpu_globe_active,
            );
            if !gpu_globe_active && hazard_compute_input.enabled {
                self.paint_cpu_hazard_particles_fallback(&painter, rect, &xform, &hazard_compute_input);
            }
            if !gpu_globe_active && self.district_hazard.any_active() {
                self.paint_cpu_district_hazard_fallback(&painter, rect, theme, &xform);
            }
            if !gpu_globe_active && self.subsurface_ssi.xray_subsurface_visible {
                self.paint_cpu_subsurface_ssi_fallback(&painter, rect, theme, &xform, bim_project);
            }
            if !gpu_globe_active && self.ifcx_diff_overlay_active {
                self.paint_cpu_ifcx_diff_ghosts_fallback(&painter, rect, theme, &xform);
            }
            if !gpu_globe_active && self.portfolio_retrofit_config.overlay_visible {
                self.paint_cpu_portfolio_retrofit_fallback(&painter, rect, theme, &xform);
            }
            if !gpu_globe_active
                && self.earth_bim_author.active_lens == EarthBimLens::TimeHistoryReplay
            {
                self.paint_cpu_plastic_hinges_fallback(&painter, rect, theme, &xform, bim_project);
                if self.ground_motion_replay_config.show_fragility_loss_overlay {
                    self.paint_cpu_fragility_loss_fallback(&painter, rect, theme, &xform, bim_project);
                }
            } else if !gpu_globe_active
                && self.ground_motion_replay_config.show_fragility_loss_overlay
                && self.last_ground_motion_replay_report.is_some()
            {
                self.paint_cpu_fragility_loss_fallback(&painter, rect, theme, &xform, bim_project);
            }
            if !gpu_globe_active
                && self.earth_bim_author.active_lens == EarthBimLens::ShmDigitalTwin
            {
                self.paint_cpu_shm_triage_fallback(&painter, rect, &xform, bim_project);
            }
            if !gpu_globe_active && self.evacuation_config.overlay_visible {
                self.paint_cpu_evacuation_lifelines_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.collab_state.config.risk_heatmap_visible {
                self.paint_cpu_district_risk_heatmap_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.field_qaqc_overlay_visible {
                self.paint_cpu_field_qaqc_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.eigenflow_overlay_visible {
                self.paint_cpu_eigenflow_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.extreme_events_overlay_visible {
                self.paint_cpu_extreme_events_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.isolation_overlay_visible {
                self.paint_cpu_isolation_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.wind_overlay_visible {
                self.paint_cpu_wind_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.shear_wall_overlay_visible {
                self.paint_cpu_shear_wall_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.connection_overlay_visible {
                self.paint_cpu_connection_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.prestress_overlay_visible {
                self.paint_cpu_prestress_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.composite_overlay_visible {
                self.paint_cpu_composite_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.reliability_overlay_visible {
                self.paint_cpu_reliability_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.foundation_overlay_visible {
                self.paint_cpu_foundation_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.construction_lca_overlay_visible {
                self.paint_cpu_construction_lca_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.zoning_overlay_visible {
                self.paint_cpu_zoning_envelope_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.program_overlay_visible {
                self.paint_cpu_spatial_program_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.massing_overlay_visible {
                self.paint_cpu_massing_study_fallback(&painter, rect, &xform);
            }
            if !gpu_globe_active && self.solar_overlay_visible {
                self.paint_cpu_solar_envelope_fallback(&painter, rect, &xform);
            }
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

        // 1b. Render Pure-Vector Grid, Local Vault Vector Layers, IFC/Survey TIN & World Buildings
        let world_to_screen = |w: [f64; 2]| self.world_to_screen(rect, w);
        if self.show_grid && self.world_map.zoom_level >= 15.5 {
            self.paint_grid(&painter, rect, theme, &world_to_screen);
        }
        if self.globe_mode {
            if let Some(ref xform) = active_globe_xform {
                if self.world_map.zoom_level >= 12.5 && self.urban_vectors.feature_count() > 0 {
                    let (min_la, max_la, min_lo, max_lo) = self.visible_geo_bbox(xform);
                    let vis_feats = self.urban_vectors.snapshot_visible(min_la, max_la, min_lo, max_lo, 900);
                    self.earth_world.paint_urban_vector_features(&painter, xform, theme, &vis_feats);
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

}

pub(super) fn point_to_segment_dist(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
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

pub(super) fn segment_intersection_2d(
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
mod tests;
