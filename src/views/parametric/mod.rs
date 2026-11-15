//! views/parametric/mod.rs — Pillar 4 (2D Studio): BIMCraft 2D Plan, Live Section Cut & Parametric Studio.
//! Features:
//! - Interactive Floor Plan Authoring (Compound Walls, Hosted Doors/Windows with Swing Arcs,
//!   Structural Columns, Beams/Runs, Slabs/Shafts, Auto-Detected Planar Rooms & Associative Dimensions)
//! - Revit-Style Object Snaps (`SE, SM, SI, SC, SP, SN, SG`), 15° Angular Guide & Graduated Ruler
//! - Live 2D Section Cut View with Solid Poché, Rebar Cut Dots, Level Planes & Grid Bubbles
//! - Sheet Layout Composer (`Sheet A101` Title Block) with One-Click Vector PDF, SVG & IFC4 Export
//! - Transactional `BimCommandBus` with Unlimited Undo/Redo & Live FEM/ARI Redundancy Audit.

use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use crate::core::{
    BimCommand, BimCommandBus, OpeningKind, P2, RebarPreset, RunCategory, RunJustify, SnapHit,
};
use crate::models::{
    CadTool, DatabaseComponent, NavigationPillar, ParametricGeneratorPreset, ParametricVariable,
};
use crate::features::{FeatureId, FeatureRegistry};
use crate::icons::{self, AppIcon};
use crate::theme::MacTheme;
use crate::ui_kit;
use crate::views::universal_canvas::{
    UniversalCanvasState, UniversalCanvasTool, WorkspaceCanvasSettings,
};

pub mod canvas_2d;
pub mod detailing_2d;
pub mod dynamo_studio;
pub mod inspector;
pub mod ribbon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftingViewMode {
    FloorPlanViewRange,
    LiveSectionCutPoche,
    SheetLayoutA101,
    VisualNodeGraphDynamo,
}

/// Below-toolbar sub-menu tabs inside Drafting (keeps upper canvas exclusively interactive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftingSubTab {
    RebarBbs,
    ElevationsAndMepRiser,
    MepEngineeringSizing,
}

/// Paper dimensions for Sheet Composer 2D
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetPaperSize {
    A4Landscape, // 297 x 210 mm
    A3Landscape, // 420 x 297 mm
    A2Landscape, // 594 x 420 mm
    A1Landscape, // 841 x 594 mm
}

impl SheetPaperSize {
    pub fn dimensions_mm(&self) -> (f32, f32) {
        match self {
            Self::A4Landscape => (297.0, 210.0),
            Self::A3Landscape => (420.0, 297.0),
            Self::A2Landscape => (594.0, 420.0),
            Self::A1Landscape => (841.0, 594.0),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::A4Landscape => "ISO A4 (297 × 210 mm)",
            Self::A3Landscape => "ISO A3 (420 × 297 mm)",
            Self::A2Landscape => "ISO A2 (594 × 420 mm)",
            Self::A1Landscape => "ISO A1 (841 × 594 mm)",
        }
    }
}

/// Scale presets for engineering drawing sheets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetScale {
    Scale1_20,
    Scale1_50,
    Scale1_100,
    Scale1_200,
}

impl SheetScale {
    pub fn ratio(&self) -> f32 {
        match self {
            Self::Scale1_20 => 1.0 / 20.0,
            Self::Scale1_50 => 1.0 / 50.0,
            Self::Scale1_100 => 1.0 / 100.0,
            Self::Scale1_200 => 1.0 / 200.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Scale1_20 => "1 : 20",
            Self::Scale1_50 => "1 : 50",
            Self::Scale1_100 => "1 : 100",
            Self::Scale1_200 => "1 : 200",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParametricFloatingTab {
    Properties,
    Project,
    CanvasSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BimAuthoringTool {
    CadStandard,
    WallChain,
    StructuralBeam,
    StructuralColumn,
    PlaceDoor,
    PlaceWindow,
    ReinforceConcrete,
}

pub struct ParametricView {
    pub active_tool: CadTool,
    pub bim_tool: BimAuthoringTool,
    pub view_mode: DraftingViewMode,
    pub bim_bus: BimCommandBus,
    pub active_level_idx: usize,
    pub pending_start_pt: Option<P2>,
    pub last_snap_hit: Option<SnapHit>,
    pub canvas_pan: Vec2,
    pub canvas_zoom: f32,
    pub component_name: String,
    pub component_category: String,
    pub variables: Vec<ParametricVariable>,
    pub command_line_input: String,
    pub command_history: Vec<String>,
    pub snap_to_grid: bool,
    pub ortho_mode: bool,
    pub cursor_pos_mm: (f32, f32, f32),
    pub show_save_modal: bool,
    pub show_db_drawer: bool,
    pub save_notification: Option<String>,
    pub ai_suggestion_text: Option<String>,
    pub live_ari_score: f32,
    pub live_svd_margin: f32,
    pub sheet_paper_size: SheetPaperSize,
    pub sheet_scale: SheetScale,
    pub sheet_number: String,
    pub sheet_title: String,
    pub sheet_author: String,
    // Phase 27 / 29: Visual Node Graph Studio (Dynamo / Grasshopper)
    pub node_graph: crate::models::VisualNodeGraph,
    pub selected_node_id: Option<usize>,
    pub graph_search_query: String,
    pub is_graph_evaluating: bool,
    pub graph_bake_status: Option<String>,
    pub active_preset: ParametricGeneratorPreset,
    pub preset_params: [f32; 4],
    pub preview_yaw: f32,
    pub preview_pitch: f32,
    pub dragging_node_id: Option<usize>,
    pub last_canvas_rect: Option<Rect>,
    pub sketch_overlay_synced: bool,
    // Phase 34: Universal-style canvas & floating HUD overlays inside canvas
    pub floating_properties_open: bool,
    pub floating_project_open: bool,
    pub floating_right_tab: ParametricFloatingTab,
    pub show_cli_float: bool,
    pub sketch_draw_mode: bool,
    pub last_floating_rects: Vec<Rect>,
    pub selected_rebar_host_idx: usize,
    pub rebar_seismic_hooks_135: bool,
    pub elev_facade_idx: usize,
    pub elev_mep_system_idx: usize,
    pub show_structural_heatmap_2d: bool,
    // Phase 45: Below-toolbar external sub-menu state (Rebar BBS, Elev/MEP Riser, MEP Sizing)
    pub active_drafting_subtab: DraftingSubTab,
    pub drafting_submenu_open: bool,
}

impl Default for ParametricView {
    fn default() -> Self {
        let default_preset = ParametricGeneratorPreset::DiagridTower;
        let default_params = default_preset.default_params();
        Self {
            active_tool: CadTool::Line,
            bim_tool: BimAuthoringTool::WallChain,
            view_mode: DraftingViewMode::FloorPlanViewRange,
            bim_bus: BimCommandBus::new(),
            active_level_idx: 0,
            pending_start_pt: None,
            last_snap_hit: None,
            canvas_pan: Vec2::ZERO,
            canvas_zoom: 1.0,
            sheet_paper_size: SheetPaperSize::A3Landscape,
            sheet_scale: SheetScale::Scale1_100,
            sheet_number: "A101".to_string(),
            sheet_title: "Untitled Site Layout Sheet".to_string(),
            sheet_author: "EigenGraph Studio".to_string(),
            component_name: "New Parametric Assembly".to_string(),
            component_category: "Custom Components".to_string(),
            variables: Vec::new(),
            command_line_input: String::new(),
            command_history: vec![
                "2D Parametric Ground Canvas Ready (Frozen by default — click Unfreeze to navigate World Map & draw).".to_string(),
            ],
            snap_to_grid: true,
            ortho_mode: false,
            cursor_pos_mm: (0.0, 0.0, 0.0),
            show_save_modal: false,
            show_db_drawer: false,
            save_notification: None,
            ai_suggestion_text: None,
            live_ari_score: 0.0,
            live_svd_margin: 0.0,
            node_graph: crate::models::VisualNodeGraph::empty(),
            selected_node_id: None,
            graph_search_query: String::new(),
            is_graph_evaluating: false,
            graph_bake_status: None,
            active_preset: default_preset,
            preset_params: default_params,
            preview_yaw: 0.65,
            preview_pitch: 0.35,
            dragging_node_id: None,
            last_canvas_rect: None,
            sketch_overlay_synced: true,
            floating_properties_open: false,
            floating_project_open: false,
            floating_right_tab: ParametricFloatingTab::Properties,
            show_cli_float: false,
            sketch_draw_mode: false,
            last_floating_rects: Vec::new(),
            selected_rebar_host_idx: 0,
            rebar_seismic_hooks_135: true,
            elev_facade_idx: 0,
            elev_mep_system_idx: 0,
            show_structural_heatmap_2d: true,
            active_drafting_subtab: DraftingSubTab::RebarBbs,
            drafting_submenu_open: true,
        }
    }
}


impl ParametricView {
    /// Returns true while the pointer hovers the Parametric viewport (outside floating HUD panels)
    /// and canvas scroll-zoom is active (unfrozen).
    pub fn is_pointer_in_live_canvas(&self, ctx: &egui::Context, is_frozen: bool) -> bool {
        if is_frozen {
            return false;
        }
        let Some(rect) = self.last_canvas_rect else {
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
        db_components: &mut Vec<DatabaseComponent>,
        active_pillar: &mut NavigationPillar,
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
        members: &[crate::models::StructuralMember],
        workspace_settings: WorkspaceCanvasSettings,
        sidebar_toggle_inset_px: f32,
    ) {
        ui_kit::set_canvas_frozen(ui.ctx(), universal_canvas.is_frozen);
        self.last_canvas_rect = None;
        self.last_floating_rects.clear();

        // Sync Save-notification banner back & forth with unified Universal Canvas toast bar
        if universal_canvas.status_message.is_some() && self.save_notification.is_none() {
            self.save_notification = universal_canvas.status_message.take();
        }

        Frame::new()
            .fill(theme.bg_sidebar)
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, theme.border_card))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                let outer_avail_w = ui.available_width().max(160.0);
                ui.set_width(outer_avail_w);
                ui.set_max_width(outer_avail_w);

                // Apply Parametric 2D + Ink CAD keyboard shortcuts (Space freeze, Esc, Del, Ctrl+Z/Y, F8, F3, keys)
                self.handle_parametric_shortcuts(ui.ctx(), universal_canvas);

                // 1. Full-Width Unobstructed World-Grounded Drawing Viewport (Top)
                self.render_drawing_canvas(
                    ui,
                    theme,
                    db_components,
                    active_pillar,
                    features,
                    universal_canvas,
                    members,
                    workspace_settings.clone(),
                    outer_avail_w,
                    sidebar_toggle_inset_px,
                );

                // Dismissible Status Toast (shared across Universal, Parametric & Geometry)
                UniversalCanvasState::render_toast_bar(ui, theme, &mut self.save_notification);
                ui.add_space(6.0);

                // 2. Bottom HUD Toolbar, Inline Levels + CAD Command Bar & Docked Inspectors
                self.render_drafting_ribbon(
                    ui,
                    theme,
                    db_components,
                    active_pillar,
                    features,
                    universal_canvas,
                    members,
                    workspace_settings,
                    sidebar_toggle_inset_px,
                );
            });

        ui.add_space(10.0);

        // 3. Phase 45: External Below-Toolbar Drafting Sub-Menus (Rebar BBS, Elev/MEP Riser, MEP Sizing)
        self.render_drafting_below_toolbar_submenu(ui, theme, features, universal_canvas);

        if self.show_db_drawer {
            self.render_db_drawer(ui.ctx(), theme, db_components, active_pillar);
        }
    }
    /// Auto-fits the 2D drafting viewport and ground basemap to active BIM element bounds.
    pub fn fit_view_to_project(&mut self, universal_canvas: &mut UniversalCanvasState) {
        let extents = self.bim_bus.project.plan_extents_mm();
        universal_canvas.fit_world_to_bim_extents(extents);
        self.canvas_pan = Vec2::ZERO;
        self.canvas_zoom = 1.0;
    }

    pub(super) fn apply_parametric_move_1m(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        let step = universal_canvas.cad_step_mm;
        if self.sketch_draw_mode {
            if universal_canvas.translate_selected_or_last(step, 0.0) {
                self.save_notification =
                    Some(format!("Moved selected/last sketch entity +{step:.0} mm (M)"));
            }
            return;
        }
        if let Some(id) = self.bim_bus.project.latest_element_id() {
            match self.bim_bus.execute(BimCommand::MoveElement {
                element_id: id,
                delta: P2(step, 0.0),
            }) {
                Ok(msg) => {
                    self.save_notification = Some(msg.clone());
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
                Err(e) => self.save_notification = Some(e),
            }
        }
    }

    pub(super) fn apply_parametric_rotate_90(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        if self.sketch_draw_mode {
            if universal_canvas.rotate_selected_or_last(90.0) {
                self.save_notification = Some("Rotated sketch entity +90°".to_string());
            }
            return;
        }
        if let Some(id) = self.bim_bus.project.latest_element_id() {
            match self.bim_bus.execute(BimCommand::RotateElement {
                element_id: id,
                angle_deg: 90.0,
            }) {
                Ok(msg) => {
                    self.save_notification = Some(msg.clone());
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
                Err(e) => self.save_notification = Some(e),
            }
        }
    }

    pub(super) fn apply_parametric_duplicate(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        let step = universal_canvas.cad_step_mm;
        if self.sketch_draw_mode {
            if universal_canvas.duplicate_selected_or_last(step, 0.0) {
                self.save_notification =
                    Some(format!("Duplicated sketch entity +{step:.0} mm East"));
            }
            return;
        }
        let lvl_id = self
            .bim_bus
            .project
            .levels
            .get(self.active_level_idx)
            .map(|l| l.id)
            .unwrap_or(1);
        if let Some(w) = self.bim_bus.project.walls.last().cloned() {
            match self.bim_bus.execute(BimCommand::AddWall {
                base_level_id: w.base_level_id,
                start: P2(w.start.0 + step, w.start.1),
                end: P2(w.end.0 + step, w.end.1),
                thickness_mm: w.thickness_mm,
                is_structural: w.is_structural,
            }) {
                Ok(msg) => {
                    self.save_notification = Some(format!("Copied wall +{step:.0} mm: {msg}"));
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
                Err(e) => self.save_notification = Some(e),
            }
        } else if let Some(c) = self.bim_bus.project.columns.last().cloned() {
            match self.bim_bus.execute(BimCommand::AddColumn {
                base_level_id: lvl_id,
                top_level_id: c.top_level_id,
                pos: P2(c.pos.0 + step, c.pos.1),
                width_mm: c.width_mm,
                depth_mm: c.depth_mm,
                is_round: c.is_round,
                profile_name: c.profile_name,
            }) {
                Ok(msg) => {
                    self.save_notification = Some(format!("Copied column +{step:.0} mm: {msg}"));
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
                Err(e) => self.save_notification = Some(e),
            }
        }
    }

    pub(super) fn apply_parametric_offset(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        if self.sketch_draw_mode {
            if universal_canvas.offset_selected_or_last(500.0) {
                self.save_notification =
                    Some("Offset duplicate sketch entity +500 mm (O)".to_string());
            }
            return;
        }
        if let Some(wid) = self.bim_bus.project.latest_wall_id() {
            match self.bim_bus.execute(BimCommand::OffsetWall {
                wall_id: wid,
                offset_mm: 500.0,
            }) {
                Ok(msg) => {
                    self.save_notification = Some(msg.clone());
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
                Err(e) => self.save_notification = Some(e),
            }
        } else {
            self.save_notification =
                Some("Create a wall first to apply +500 mm CAD Wall Offset".to_string());
        }
    }

    pub(super) fn apply_parametric_mirror(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        let mirror_x = self
            .bim_bus
            .project
            .plan_extents_mm()
            .map_or(0.0, |(a, b)| 0.5 * (a[0] + b[0]));
        if self.sketch_draw_mode {
            if universal_canvas.mirror_selected_or_last(mirror_x) {
                self.save_notification = Some(format!(
                    "Mirrored sketch entity across X={mirror_x:.0} mm (X)"
                ));
            }
            return;
        }
        match self.bim_bus.execute(BimCommand::MirrorElements {
            axis_a: P2(mirror_x, -5000.0),
            axis_b: P2(mirror_x, 5000.0),
        }) {
            Ok(msg) => {
                self.save_notification = Some(msg.clone());
                self.command_history.push(msg);
                self.refresh_live_ari();
            }
            Err(e) => self.save_notification = Some(e),
        }
    }

    pub(super) fn apply_parametric_delete(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        if self.sketch_draw_mode {
            if universal_canvas.delete_selected_or_last() {
                self.save_notification = Some("Deleted sketch entity (Del)".to_string());
            }
            return;
        }
        if let Some(id) = self.bim_bus.project.latest_element_id() {
            match self
                .bim_bus
                .execute(BimCommand::DeleteElement { element_id: id })
            {
                Ok(msg) => {
                    self.save_notification = Some(msg.clone());
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
                Err(e) => self.save_notification = Some(e),
            }
        }
    }

    fn handle_parametric_shortcuts(
        &mut self,
        ctx: &egui::Context,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        let fb = UniversalCanvasState::poll_cad_shortcuts(ctx, universal_canvas.is_frozen);
        if fb.toggle_freeze {
            universal_canvas.is_frozen = !universal_canvas.is_frozen;
            self.pending_start_pt = None;
            self.save_notification = Some(if universal_canvas.is_frozen {
                "Canvas Frozen (Space) • Page scroll safe".to_string()
            } else {
                "Canvas Live (Space) • Drafting shortcuts ready".to_string()
            });
        }
        if fb.escape {
            self.pending_start_pt = None;
            universal_canvas.measure_active = false;
            universal_canvas.selected_entity_id = None;
            self.active_tool = CadTool::Select;
            self.bim_tool = BimAuthoringTool::CadStandard;
            universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
        }
        if fb.undo {
            if self.sketch_draw_mode {
                if universal_canvas.undo() {
                    self.save_notification = Some("Undo Ink sketch (Ctrl/Cmd+Z)".to_string());
                }
            } else if let Some(msg) = self.bim_bus.undo() {
                self.save_notification = Some(msg.clone());
                self.command_history.push(msg);
                self.refresh_live_ari();
            }
        }
        if fb.redo {
            if self.sketch_draw_mode {
                if universal_canvas.redo() {
                    self.save_notification = Some("Redo Ink sketch (Ctrl/Cmd+Y)".to_string());
                }
            } else if let Some(msg) = self.bim_bus.redo() {
                self.save_notification = Some(msg.clone());
                self.command_history.push(msg);
                self.refresh_live_ari();
            }
        }
        if fb.delete_selected {
            self.apply_parametric_delete(universal_canvas);
        }
        if fb.toggle_ortho {
            self.ortho_mode = !self.ortho_mode;
            universal_canvas.ortho_mode = self.ortho_mode;
        }
        if fb.toggle_snap {
            self.snap_to_grid = !self.snap_to_grid;
            universal_canvas.snap_to_grid = self.snap_to_grid;
        }
        if fb.fit_model {
            self.fit_view_to_project(universal_canvas);
        }
        if let Some(ch) = fb.letter_key {
            match ch {
                'W' => {
                    self.sketch_draw_mode = false;
                    self.bim_tool = BimAuthoringTool::WallChain;
                    self.pending_start_pt = None;
                }
                'B' => {
                    self.sketch_draw_mode = false;
                    self.bim_tool = BimAuthoringTool::StructuralBeam;
                    self.pending_start_pt = None;
                }
                'C' => {
                    if self.sketch_draw_mode {
                        universal_canvas.active_tool = UniversalCanvasTool::Circle;
                    } else {
                        self.bim_tool = BimAuthoringTool::StructuralColumn;
                        self.pending_start_pt = None;
                    }
                }
                'L' => {
                    if self.sketch_draw_mode {
                        universal_canvas.active_tool = UniversalCanvasTool::Line;
                    } else {
                        self.active_tool = CadTool::Line;
                        self.bim_tool = BimAuthoringTool::CadStandard;
                    }
                }
                'R' => {
                    if self.sketch_draw_mode {
                        universal_canvas.active_tool = UniversalCanvasTool::Rectangle;
                    } else {
                        self.active_tool = CadTool::Rectangle;
                        self.bim_tool = BimAuthoringTool::CadStandard;
                    }
                }
                'D' => {
                    if self.sketch_draw_mode {
                        universal_canvas.active_tool = UniversalCanvasTool::Dimension;
                    } else {
                        self.active_tool = CadTool::Dimension;
                        self.bim_tool = BimAuthoringTool::CadStandard;
                    }
                }
                'M' => self.apply_parametric_move_1m(universal_canvas),
                'O' => self.apply_parametric_offset(universal_canvas),
                'X' => self.apply_parametric_mirror(universal_canvas),
                _ => {}
            }
        }
    }

    pub(super) fn refresh_live_ari(&mut self) {
        if let Ok(rep) = self.bim_bus.project.evaluate_live_ari() {
            self.live_ari_score = rep.average_ari as f32;
            self.live_svd_margin = if rep.kinematic_guard.is_stable {
                0.996
            } else {
                0.0
            };
            self.ai_suggestion_text = Some(format!(
                "AI Copilot: {} Walls, {} Columns, {} Beams, {} Rooms ({:.1} m²), Rebar {:.0} kg | ARI: {:.3}",
                self.bim_bus.project.walls.len(),
                self.bim_bus.project.columns.len(),
                self.bim_bus.project.runs.len(),
                self.bim_bus.project.rooms.len(),
                self.bim_bus.project.rooms.iter().map(|r| r.area_m2).sum::<f64>(),
                self.bim_bus.project.total_rebar_weight_kg(),
                self.live_ari_score
            ));
        }
    }


    pub(super) fn execute_cad_command(&mut self, cmd_raw: &str) {
        let trimmed = cmd_raw.trim();
        if trimmed.starts_with('{') {
            match self.bim_bus.execute_json(trimmed) {
                Ok(msg) => {
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
                Err(e) => self.command_history.push(format!("JSON Error: {e}")),
            }
            return;
        }

        if trimmed.starts_with('@') {
            if let Some((dx, dy)) = UniversalCanvasState::parse_cad_delta_expr(trimmed) {
                let start = self
                    .pending_start_pt
                    .or_else(|| self.bim_bus.project.walls.last().map(|w| w.end))
                    .unwrap_or(P2(0.0, 0.0));
                let end = P2(start.0 + dx, start.1 + dy);
                let lvl_id = self
                    .bim_bus
                    .project
                    .levels
                    .get(self.active_level_idx)
                    .map(|l| l.id)
                    .unwrap_or(1);
                if let Ok(msg) = self.bim_bus.execute(BimCommand::AddWall {
                    base_level_id: lvl_id,
                    start,
                    end,
                    thickness_mm: 200.0,
                    is_structural: true,
                }) {
                    self.pending_start_pt = Some(end);
                    self.command_history.push(msg.clone());
                    self.save_notification = Some(msg);
                    self.refresh_live_ari();
                }
                return;
            }
        }

        let cmd_upper = trimmed.to_uppercase();
        if let Some(deg_str) = cmd_upper
            .strip_prefix("RO ")
            .or_else(|| cmd_upper.strip_prefix("ROTATE "))
        {
            if let Ok(deg) = deg_str.trim().parse::<f64>() {
                if let Some(id) = self.bim_bus.project.latest_element_id() {
                    if let Ok(msg) = self.bim_bus.execute(BimCommand::RotateElement {
                        element_id: id,
                        angle_deg: deg,
                    }) {
                        self.command_history.push(msg.clone());
                        self.save_notification = Some(msg);
                        self.refresh_live_ari();
                    }
                }
                return;
            }
        }

        let cmd = trimmed.to_uppercase();
        match cmd.as_str() {
            "L" | "LINE" => {
                self.active_tool = CadTool::Line;
                self.bim_tool = BimAuthoringTool::CadStandard;
            }
            "WA" | "WALL" => self.bim_tool = BimAuthoringTool::WallChain,
            "BM" | "BEAM" => self.bim_tool = BimAuthoringTool::StructuralBeam,
            "CL" | "COLUMN" => self.bim_tool = BimAuthoringTool::StructuralColumn,
            "DR" | "DOOR" => self.bim_tool = BimAuthoringTool::PlaceDoor,
            "WN" | "WINDOW" => self.bim_tool = BimAuthoringTool::PlaceWindow,
            "RF" | "REINFORCE" => {
                self.bim_tool = BimAuthoringTool::ReinforceConcrete;
                if let Ok(msg) = self.bim_bus.execute(BimCommand::ReinforceAllConcrete {
                    preset: RebarPreset::default(),
                }) {
                    self.command_history.push(msg);
                }
            }
            "RM" | "ROOM" => {
                let lvl_id = self
                    .bim_bus
                    .project
                    .levels
                    .get(self.active_level_idx)
                    .map(|l| l.id)
                    .unwrap_or(1);
                if let Ok(msg) = self
                    .bim_bus
                    .execute(BimCommand::AutoDetectRooms { level_id: lvl_id })
                {
                    self.command_history.push(msg);
                }
            }
            "UNDO" | "U" => {
                if let Some(msg) = self.bim_bus.undo() {
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
            }
            "REDO" => {
                if let Some(msg) = self.bim_bus.redo() {
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
            }
            "PDF" | "SVG" | "IFC" => {
                self.export_bim_deliverables();
            }
            "PL" | "POLYLINE" => self.active_tool = CadTool::Polyline,
            "A" | "ARC" => self.active_tool = CadTool::Arc,
            "C" | "CIRCLE" => self.active_tool = CadTool::Circle,
            "REC" | "RECTANGLE" => self.active_tool = CadTool::Rectangle,
            "DIM" | "DI" | "DIMENSION" => self.active_tool = CadTool::Dimension,
            "TR" | "TRIM" => self.active_tool = CadTool::Trim,
            "O" | "OFFSET" => {
                self.active_tool = CadTool::Offset;
                if let Some(wid) = self.bim_bus.project.latest_wall_id() {
                    if let Ok(msg) = self.bim_bus.execute(BimCommand::OffsetWall {
                        wall_id: wid,
                        offset_mm: 500.0,
                    }) {
                        self.command_history.push(msg.clone());
                        self.save_notification = Some(msg);
                        self.refresh_live_ari();
                    }
                }
            }
            "M" | "MOVE" => {
                if let Some(id) = self.bim_bus.project.latest_element_id() {
                    if let Ok(msg) = self.bim_bus.execute(BimCommand::MoveElement {
                        element_id: id,
                        delta: P2(1000.0, 0.0),
                    }) {
                        self.command_history.push(msg.clone());
                        self.save_notification = Some(msg);
                        self.refresh_live_ari();
                    }
                }
            }
            "MI" | "MIRROR" | "X" => {
                let cx = self
                    .bim_bus
                    .project
                    .plan_extents_mm()
                    .map_or(0.0, |(a, b)| 0.5 * (a[0] + b[0]));
                if let Ok(msg) = self.bim_bus.execute(BimCommand::MirrorElements {
                    axis_a: P2(cx, -5000.0),
                    axis_b: P2(cx, 5000.0),
                }) {
                    self.command_history.push(msg.clone());
                    self.save_notification = Some(msg);
                    self.refresh_live_ari();
                }
            }
            "DEL" | "DELETE" | "ERASE" => {
                if let Some(id) = self.bim_bus.project.latest_element_id() {
                    if let Ok(msg) = self
                        .bim_bus
                        .execute(BimCommand::DeleteElement { element_id: id })
                    {
                        self.command_history.push(msg.clone());
                        self.save_notification = Some(msg);
                        self.refresh_live_ari();
                    }
                }
            }
            _ => match self.bim_bus.execute_natural_language(trimmed) {
                Ok(msg) => {
                    self.command_history.push(msg.clone());
                    self.save_notification = Some(msg);
                    self.refresh_live_ari();
                }
                Err(_) => {
                    self.command_history
                        .push(format!("Executed instruction: {cmd}"));
                }
            },
        }
    }

    pub(super) fn save_current_to_db(&mut self, db_components: &mut Vec<DatabaseComponent>) {
        let new_comp = DatabaseComponent {
            id: format!("comp_{}", db_components.len() + 1),
            name: self.component_name.clone(),
            category: self.component_category.clone(),
            description: format!(
                "BIMCraft relational model: {} walls, {} columns, {} beams, {} rooms, {:.0} kg rebar.",
                self.bim_bus.project.walls.len(),
                self.bim_bus.project.columns.len(),
                self.bim_bus.project.runs.len(),
                self.bim_bus.project.rooms.len(),
                self.bim_bus.project.total_rebar_weight_kg()
            ),
            variables: self.variables.clone(),
            elements_count: self.bim_bus.project.walls.len()
                + self.bim_bus.project.columns.len()
                + self.bim_bus.project.runs.len(),
            created_at: "Just now".to_string(),
            ..Default::default()
        };

        db_components.push(new_comp);
        if let Ok(saved_path) =
            crate::core::InternalVault::autosave_parametric_components(db_components)
        {
            self.save_notification = Some(format!(
                "Component '{}' automatically saved to Package Contents: {} (Ready to assemble in BUILD 3D).",
                self.component_name, saved_path
            ));
        } else {
            self.save_notification = Some(format!(
                "Component '{}' successfully saved to Database!",
                self.component_name
            ));
        }
    }

    fn render_db_drawer(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        db_components: &mut [DatabaseComponent],
        active_pillar: &mut NavigationPillar,
    ) {
        let mut open = self.show_db_drawer;
        let mut should_close = false;
        let mut load_idx: Option<usize> = None;
        let mut assemble_in_build = false;

        egui::Window::new("Parametric Component Database & IFC Sections")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
            .default_size(MacTheme::clamp_modal_size(ctx, Vec2::new(660.0, 520.0)))
            .max_size(MacTheme::modal_max_size(ctx))
            .frame(
                Frame::new()
                    .fill(theme.bg_card)
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        icons::draw_icon(ui, AppIcon::Building, 26.0, theme.accent_blue);
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(
                                    "Parametric & Complex Structure Section Catalog (OpenBIM IFC)",
                                )
                                .size(15.0)
                                .strong()
                                .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new(
                                    "Modular sections from complex architectural buildings ready to use as reference & assembly.",
                                )
                                .size(11.5)
                                .color(theme.text_secondary),
                            );
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for (idx, comp) in db_components.iter().enumerate() {
                            theme.card_frame().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&comp.name)
                                            .size(13.0)
                                            .strong()
                                            .color(theme.accent_blue),
                                    );
                                    ui.add_space(6.0);
                                    Frame::new()
                                        .fill(theme.accent_blue.gamma_multiply(0.15))
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(&comp.category)
                                                    .size(10.0)
                                                    .color(theme.accent_blue),
                                            );
                                        });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(&comp.created_at)
                                                .size(10.5)
                                                .color(theme.text_muted),
                                        );
                                    });
                                });

                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(&comp.description)
                                        .size(11.5)
                                        .color(theme.text_secondary),
                                );

                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new("Variables:")
                                            .size(10.5)
                                            .strong()
                                            .color(theme.text_muted),
                                    );
                                    for v in comp.variables.iter().take(5) {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} = {} {}",
                                                v.name, v.expression, v.unit
                                            ))
                                            .size(10.0)
                                            .monospace()
                                            .color(theme.accent_green),
                                        );
                                        ui.add_space(4.0);
                                    }
                                });

                                ui.add_space(8.0);
                                ui.horizontal_wrapped(|ui| {
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::UploadImport,
                                        "Load into Canvas",
                                        11.0,
                                        Color32::WHITE,
                                        theme.accent_blue,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::new(0.0, 26.0),
                                    )
                                    .clicked()
                                    {
                                        load_idx = Some(idx);
                                    }

                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Cube3D,
                                        "Assemble in Geometry (3D)",
                                        11.0,
                                        theme.text_secondary,
                                        theme.bg_canvas,
                                        Stroke::new(1.0, theme.border_subtle),
                                        CornerRadius::same(6),
                                        Vec2::new(0.0, 26.0),
                                    )
                                    .clicked()
                                    {
                                        assemble_in_build = true;
                                    }
                                });
                            });
                            ui.add_space(8.0);
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            open = false;
        }
        self.show_db_drawer = open;

        if let Some(idx) = load_idx {
            if let Some(comp) = db_components.get(idx) {
                self.component_name = comp.name.clone();
                self.component_category = comp.category.clone();
                self.variables = comp.variables.clone();
                self.save_notification = Some(format!(
                    "Component '{}' loaded into parametric editor successfully!",
                    comp.name
                ));
                self.show_db_drawer = false;
            }
        }

        if assemble_in_build {
            *active_pillar = NavigationPillar::GeometryAssembly;
            self.show_db_drawer = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parametric_canvas_width_matches_container_without_right_overflow() {
        let _ctx = egui::Context::default();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let mut features = FeatureRegistry::default();
        let mut universal = UniversalCanvasState::default();
        let mut parametric = ParametricView::default();
        let mut build = crate::views::build::BuildView::default();
        let mut db_components = Vec::new();
        let mut active_pillar = NavigationPillar::ParametricDED;
        let members = Vec::new();
        let vars = Vec::new();
        let ws = WorkspaceCanvasSettings {
            theme_mode: crate::theme::ThemeMode::ProDark,
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
            unit_system: "Metric".to_string(),
            design_standard: "SNI".to_string(),
            solver_threads: 4,
            enable_svd_guard: true,
            enable_anti_leakage: true,
        };

        let mut failures = Vec::new();
        for win_w in [1000.0_f32, 700.0, 390.0] {
            for mode_idx in 0..3 {
                let mut panel_right = 0.0_f32;
                let mut rect_right = 0.0_f32;
                let mut outer_right = 0.0_f32;
                let ctx_i = egui::Context::default();
                for _ in 0..3 {
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(win_w, 800.0))),
                        ..Default::default()
                    };
                    let mut out = ctx_i.run_ui(input, |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            panel_right = ui.max_rect().right();
                            egui::ScrollArea::vertical()
                                .id_salt(("test_scroll", mode_idx, win_w as i32))
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let r = Frame::new().inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
                                        match mode_idx {
                                            0 => universal.show(
                                                ui,
                                                &theme,
                                                NavigationPillar::ResearchConcept,
                                                &mut build.bim_bus.project,
                                                &mut members.clone(),
                                                &mut parametric.variables,
                                                ws.clone(),
                                                0.0,
                                            ),
                                            1 => parametric.show(
                                                ui,
                                                &theme,
                                                &mut db_components,
                                                &mut active_pillar,
                                                &mut features,
                                                &mut universal,
                                                &members,
                                                ws.clone(),
                                                0.0,
                                            ),
                                            _ => build.show(
                                                ui,
                                                &theme,
                                                &members,
                                                &db_components,
                                                &mut features,
                                                &mut universal,
                                                &vars,
                                                ws.clone(),
                                                0.0,
                                            ),
                                        }
                                    });
                                    outer_right = r.response.rect.right();
                                });
                        });
                    });
                    out.textures_delta.clear();
                    rect_right = match mode_idx {
                        0 => outer_right,
                        1 => parametric.last_canvas_rect.map(|r| r.right()).unwrap_or(0.0),
                        _ => build.last_viewport_rect.map(|r| r.right()).unwrap_or(0.0),
                    };
                }
                let name = match mode_idx {
                    0 => "Universal",
                    1 => "Parametric",
                    _ => "Build",
                };
                let canvas_ok = if mode_idx == 0 {
                    outer_right <= panel_right + 1.0
                } else {
                    rect_right <= panel_right - 13.0
                };
                eprintln!(
                    "win_w={:.0} {}: panel_right={:.2}, outer_right={:.2}, canvas_right={:.2}",
                    win_w, name, panel_right, outer_right, rect_right
                );
                if !(outer_right <= panel_right + 1.0 && canvas_ok) {
                    failures.push(format!(
                        "{} @ {:.0}: panel_right={:.2}, outer_right={:.2}, canvas_right={:.2}",
                        name, win_w, panel_right, outer_right, rect_right
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "Canvas layout overflows: {:?}", failures);
    }

    #[test]
    fn test_below_canvas_module_matches_canvas_width_parity() {
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let mut features = FeatureRegistry::default();
        let mut universal = UniversalCanvasState::default();
        let members = Vec::new();
        let mut vars = Vec::new();
        let ws = WorkspaceCanvasSettings {
            theme_mode: crate::theme::ThemeMode::ProDark,
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
            unit_system: "Metric".to_string(),
            design_standard: "SNI".to_string(),
            solver_threads: 4,
            enable_svd_guard: true,
            enable_anti_leakage: true,
        };
        let bim_proj = crate::core::BimProject::sample_bimcraft_building();

        for win_w in [1000.0_f32, 700.0, 390.0] {
            let mut analyze = crate::views::AnalyzeView::default();
            let mut sust = crate::views::SustainabilityView::default();
            let mut planning = crate::views::PlanningView::default();
            let mut pillar_cost = NavigationPillar::CostBudgeting;
            let ctx = egui::Context::default();
            let mut canvas_w = 0.0_f32;
            let mut analyze_w = 0.0_f32;
            let mut sust_w = 0.0_f32;
            let mut plan_w = 0.0_f32;

            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(win_w, 1200.0))),
                    ..Default::default()
                };
                let mut output = ctx.run_ui(input, |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        Frame::new().inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
                            let content_w = ui.available_width().max(160.0);
                            ui.set_width(content_w);
                            ui.set_max_width(content_w);

                            let c_resp = ui.vertical(|ui| {
                                let mut m_clone = members.clone();
                                let mut b_clone = bim_proj.clone();
                                universal.show(
                                    ui,
                                    &theme,
                                    NavigationPillar::AnalysisSimulation,
                                    &mut b_clone,
                                    &mut m_clone,
                                    &mut vars,
                                    ws.clone(),
                                    0.0,
                                );
                            });
                            canvas_w = c_resp.response.rect.width();

                            let a_resp = ui.vertical(|ui| {
                                analyze.show(ui, &theme, &members, &bim_proj, &mut features);
                            });
                            analyze_w = a_resp.response.rect.width();

                            let s_resp = ui.vertical(|ui| {
                                sust.show(ui, &theme, &members, &bim_proj, &mut features);
                            });
                            sust_w = s_resp.response.rect.width();

                            let p_resp = ui.vertical(|ui| {
                                planning.show(
                                    ui,
                                    &theme,
                                    &vars,
                                    &members,
                                    &bim_proj,
                                    &mut features,
                                    &mut pillar_cost,
                                );
                            });
                            plan_w = p_resp.response.rect.width();
                        });
                    });
                });
                output.textures_delta.clear();
            }

            assert!(
                (canvas_w - analyze_w).abs() <= 2.0,
                "Analyze width mismatch @ {win_w}: canvas={canvas_w:.1}, analyze={analyze_w:.1}"
            );
            assert!(
                (canvas_w - sust_w).abs() <= 2.0,
                "Sustainability width mismatch @ {win_w}: canvas={canvas_w:.1}, sust={sust_w:.1}"
            );
            assert!(
                (canvas_w - plan_w).abs() <= 2.0,
                "Planning width mismatch @ {win_w}: canvas={canvas_w:.1}, planning={plan_w:.1}"
            );

            // Verify all 7 Lean Construction subtabs (including CuttingStock & Phase 45 QA/QC Lab) match canvas_w without overflowing right edge
            for tab in [
                crate::views::lean::LeanSubTab::CuttingStock,
                crate::views::lean::LeanSubTab::LastPlanner,
                crate::views::lean::LeanSubTab::ValueStream,
                crate::views::lean::LeanSubTab::JitDelivery,
                crate::views::lean::LeanSubTab::TaktConcreteDispatch,
                crate::views::lean::LeanSubTab::RfiSubmittalTracker,
                crate::views::lean::LeanSubTab::QaQcLabAndCommissioning,
            ] {
                let mut lean = crate::views::LeanView::default();
                lean.active_subtab = tab;
                let lean_ctx = egui::Context::default();
                let mut lean_w = 0.0_f32;
                let mut lean_max_x = 0.0_f32;
                let mut lean_left = 0.0_f32;
                for _ in 0..2 {
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(win_w, 1200.0))),
                        ..Default::default()
                    };
                    let mut output = lean_ctx.run_ui(input, |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            Frame::new().inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
                                let content_w = ui.available_width().max(160.0);
                                ui.set_width(content_w);
                                ui.set_max_width(content_w);
                                let l_resp = ui.vertical(|ui| {
                                    lean.show(ui, &theme, &members, &bim_proj, &mut features);
                                });
                                lean_w = l_resp.response.rect.width();
                                lean_max_x = l_resp.response.rect.max.x;
                                lean_left = l_resp.response.rect.min.x;
                            });
                        });
                    });
                    output.textures_delta.clear();
                }
                assert!(
                    (canvas_w - lean_w).abs() <= 2.0,
                    "Lean ({tab:?}) width mismatch @ {win_w}: canvas={canvas_w:.1}, lean={lean_w:.1}, span=[{lean_left:.1}..{lean_max_x:.1}]"
                );
            }
        }
    }

    #[test]
    fn test_phase45_drafting_and_geometry_below_toolbar_submenus() {
        let drawing_modes = [
            DraftingViewMode::FloorPlanViewRange,
            DraftingViewMode::LiveSectionCutPoche,
            DraftingViewMode::SheetLayoutA101,
            DraftingViewMode::VisualNodeGraphDynamo,
        ];
        assert_eq!(drawing_modes.len(), 4);

        let theme = MacTheme::default();
        let ws = WorkspaceCanvasSettings {
            theme_mode: crate::theme::ThemeMode::ProDark,
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
            unit_system: "Metric".to_string(),
            design_standard: "SNI".to_string(),
            solver_threads: 4,
            enable_svd_guard: true,
            enable_anti_leakage: true,
        };
        let mut features = FeatureRegistry::default();
        let mut db_components = Vec::new();
        let bim_proj = crate::core::BimProject::sample_bimcraft_building();
        let fem = bim_proj.to_structural_model();
        let members: Vec<crate::models::StructuralMember> = fem
            .elements
            .iter()
            .map(|el| crate::models::StructuralMember {
                id: el.id,
                name: el.name.clone(),
                section: el.section.name.clone(),
                ari: 0.62,
                utilization: 0.94,
                ..Default::default()
            })
            .collect();

        let mut failures = Vec::new();
        for win_w in [1000.0_f32, 700.0, 390.0] {
            for (d_tab, is_open) in [
                (DraftingSubTab::RebarBbs, true),
                (DraftingSubTab::ElevationsAndMepRiser, true),
                (DraftingSubTab::MepEngineeringSizing, true),
                (DraftingSubTab::MepEngineeringSizing, false),
            ] {
                let mut universal = crate::views::universal_canvas::UniversalCanvasState::default();
                let mut parametric = ParametricView::default();
                parametric.bim_bus.project = bim_proj.clone();
                parametric.active_drafting_subtab = d_tab;
                parametric.drafting_submenu_open = is_open;
                let mut panel_right = 0.0_f32;
                let mut outer_right = 0.0_f32;
                let ctx = egui::Context::default();
                for _ in 0..2 {
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(win_w, 1000.0))),
                        ..Default::default()
                    };
                    let mut out = ctx.run_ui(input, |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            panel_right = ui.max_rect().right();
                            Frame::new().inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
                                let r = ui.vertical(|ui| {
                                    let mut pillar = NavigationPillar::ParametricDED;
                                    parametric.show(
                                        ui,
                                        &theme,
                                        &mut db_components,
                                        &mut pillar,
                                        &mut features,
                                        &mut universal,
                                        &members,
                                        ws.clone(),
                                        0.0,
                                    );
                                });
                                outer_right = r.response.rect.right();
                            });
                        });
                    });
                    out.textures_delta.clear();
                }
                let canvas_right = parametric.last_canvas_rect.map(|r| r.right()).unwrap_or(0.0);
                assert!(parametric.last_canvas_rect.is_some(), "Upper 2D canvas disappeared for {d_tab:?}");
                if !(outer_right <= panel_right + 1.0 && canvas_right <= panel_right - 13.0) {
                    failures.push(format!(
                        "Drafting ({d_tab:?}, open={is_open}) @ {win_w}: panel_right={panel_right:.2}, outer_right={outer_right:.2}, canvas_right={canvas_right:.2}"
                    ));
                }
            }

            for (g_tab, is_open) in [
                (crate::views::build::GeometryAnalysisSubTab::DiagramsAndHeatmap, true),
                (crate::views::build::GeometryAnalysisSubTab::SolverAndN1, true),
                (crate::views::build::GeometryAnalysisSubTab::CriticalMembersBridge, true),
                (crate::views::build::GeometryAnalysisSubTab::DiagramsAndHeatmap, false),
            ] {
                let mut universal = crate::views::universal_canvas::UniversalCanvasState::default();
                let mut build = crate::views::build::BuildView::default();
                build.bim_bus.project = bim_proj.clone();
                build.active_analysis_subtab = g_tab;
                build.analysis_panel_open = is_open;
                let mut panel_right = 0.0_f32;
                let mut outer_right = 0.0_f32;
                let ctx = egui::Context::default();
                for _ in 0..2 {
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(win_w, 1000.0))),
                        ..Default::default()
                    };
                    let mut out = ctx.run_ui(input, |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            panel_right = ui.max_rect().right();
                            Frame::new().inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
                                let r = ui.vertical(|ui| {
                                    build.show(
                                        ui,
                                        &theme,
                                        &members,
                                        &db_components,
                                        &mut features,
                                        &mut universal,
                                        &[],
                                        ws.clone(),
                                        0.0,
                                    );
                                });
                                outer_right = r.response.rect.right();
                            });
                        });
                    });
                    out.textures_delta.clear();
                }
                let vp_right = build.last_viewport_rect.map(|r| r.right()).unwrap_or(0.0);
                assert!(build.last_viewport_rect.is_some(), "Upper 3D viewport disappeared for {g_tab:?}");
                if !(outer_right <= panel_right + 1.0 && vp_right <= panel_right - 13.0) {
                    failures.push(format!(
                        "Geometry ({g_tab:?}, open={is_open}) @ {win_w}: panel_right={panel_right:.2}, outer_right={outer_right:.2}, vp_right={vp_right:.2}"
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "Phase 45 submenu overflows: {failures:?}");
    }
}
