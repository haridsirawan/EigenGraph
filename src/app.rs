//! app.rs — Core application struct implementing eframe::App.

use eframe::App;
use egui::{
    Align, Align2, Area, CentralPanel, Color32, CornerRadius, Frame, Id, Layout, Margin, Order,
    Panel, Pos2, Rect, RichText, ScrollArea, Stroke, StrokeKind, Vec2,
};

use crate::features::FeatureRegistry;
use crate::icons::{self, AppIcon};
use crate::models::{DatabaseComponent, NavigationPillar, ProjectItem, StructuralMember};
use crate::theme::MacTheme;
use crate::views::{
    AiAssistantView, AiFirmView, AnalyzeView, BuildView, FacilityView,
    IndustrializedView, LeanView, MyProjectView, ParametricView, PlanningView, PredictionView,
    ResearchConceptView, ResultsView, SafetyView, SettingsView, SpatialWorkspaceState,
    SustainabilityView, UniversalCanvasState,
};

pub struct EigenGraphApp {
    pub theme: MacTheme,
    pub active_pillar: NavigationPillar,
    pub last_active_pillar: NavigationPillar,
    pub search_query: String,
    pub show_settings: bool,
    pub sidebar_open: bool,
    layout_initialized: bool,
    last_compact_layout: bool,

    // Centralized Feature Gate & Phase 31+ Unlock Roadmap Registry
    pub feature_registry: FeatureRegistry,

    // 15-Pillar Master Shell High-Fidelity Views
    pub my_project_view: MyProjectView,         // Pillar 1: My Library (Obsidian-Style Vault)
    pub ai_firm_view: AiFirmView,               // Pillar 2: AI Agent (Virtual Engineering Firm)
    pub research_view: ResearchConceptView,     // Pillar 3: Research (GIS, CRS & DEM TIN Mesh)
    pub parametric_view: ParametricView,        // Pillar 4: Parametric (2D CAD & 3D Companion)
    pub build_view: BuildView,                  // Pillar 5: Geometry (Clean BIM Spatial Assembly)
    pub analyze_view: AnalyzeView,              // Pillar 6: Analysis (3D Geo-Structural Solver)
    pub planning_view: PlanningView,            // Pillars 7 & 8: Time (4D) & Cost (5D Heatmap)
    pub sustainability_view: SustainabilityView,// Pillar 9: Sustainability (6D Solar Heliodon)
    pub facility_view: FacilityView,            // Pillar 10: Facility Management (7D Digital Twin)
    pub safety_view: SafetyView,                // Pillar 11: Safety & Health (8D Crane & Fall Risk)
    pub lean_view: LeanView,                    // Pillar 12: Lean Construction (9D Staging)
    pub industrialized_view: IndustrializedView,// Pillar 13: Industrialized & Automation (10D DfMA)
    pub prediction_view: PredictionView,        // Pillar 14: Prediction (AI Aging & Drift)
    pub results_view: ResultsView,              // Pillar 15: Report (Living Docs, Sheets, Slides)
    pub settings_view: SettingsView,
    pub universal_canvas: UniversalCanvasState,
    pub ai_assistant_view: AiAssistantView,
    pub spatial_workspace: SpatialWorkspaceState,

    // Domain Data & Parametric Component Database
    pub projects: Vec<ProjectItem>,
    pub db_components: Vec<DatabaseComponent>,
    pub members: Vec<StructuralMember>,

    // Phase 36: Personal Vault Connection Startup Modal & Indexed Vault Projects
    pub show_vault_startup_modal: bool,
    pub vault_path_input: String,
    pub vault_new_name_input: String,
    pub vault_modal_feedback: Option<String>,
    pub personal_vault_projects: Vec<crate::core::vault::PersonalVaultProjectEntry>,
    pub last_vault_bridge_version: u64,
    autosave_hydrated: bool,
    last_autosave_time: f64,
    last_autosave_hash: u64,
    last_bim_hash: u64,
    last_uc_earth: (f64, f64, f32, crate::core::gis::TileSource),
    last_res_earth: (f64, f64, f32, crate::core::gis::TileSource),
    last_planning_sim_day: u32,
    pending_lazy_vault_entry: Option<crate::core::vault::PersonalVaultProjectEntry>,
}

impl Default for EigenGraphApp {
    fn default() -> Self {
        let _has_vault = crate::core::InternalVault::has_connected_vault();
        let personal_vault_projects = crate::core::InternalVault::list_personal_vault_projects();
        let mut universal_canvas = UniversalCanvasState::default();
        if !personal_vault_projects.is_empty() || _has_vault {
            let mut widx = crate::core::InternalVault::load_world_spatial_index();
            crate::core::InternalVault::sync_world_spatial_index_with_vault(
                &personal_vault_projects,
                &mut widx,
            );
            universal_canvas.world_index = widx;
        }
        let mut spatial_workspace = SpatialWorkspaceState::default();
        if spatial_workspace.spatial_mode_enabled {
            // Spatial full-viewport mode has no outer vertical page scroll conflict.
            universal_canvas.is_frozen = false;
        }
        let research_view = ResearchConceptView::default();
        let uc_earth = (
            universal_canvas.world_map.center_lat_lon.0,
            universal_canvas.world_map.center_lat_lon.1,
            universal_canvas.world_map.zoom_level,
            universal_canvas.world_map.tile_provider,
        );
        let res_earth = (
            research_view.slippy_map.center_lat_lon.0,
            research_view.slippy_map.center_lat_lon.1,
            research_view.slippy_map.zoom_level,
            research_view.slippy_map.tile_provider,
        );
        let settings_view = SettingsView::default();
        spatial_workspace.gemini_api_key_draft = settings_view.gemini_api_key.clone();
        spatial_workspace.remember_gemini_key = settings_view.remember_gemini_key;
        Self {
            theme: MacTheme::default(),
            active_pillar: NavigationPillar::MyLibrary,
            last_active_pillar: NavigationPillar::MyLibrary,
            search_query: String::new(),
            show_settings: false,
            sidebar_open: true,
            layout_initialized: false,
            last_compact_layout: false,

            feature_registry: FeatureRegistry::default(),

            my_project_view: MyProjectView::default(),
            ai_firm_view: AiFirmView::default(),
            research_view,
            parametric_view: ParametricView::default(),
            build_view: BuildView::default(),
            analyze_view: AnalyzeView::default(),
            planning_view: PlanningView::default(),
            sustainability_view: SustainabilityView::default(),
            facility_view: FacilityView::default(),
            safety_view: SafetyView::default(),
            lean_view: LeanView::default(),
            industrialized_view: IndustrializedView::default(),
            prediction_view: PredictionView::default(),
            results_view: ResultsView::default(),
            settings_view,
            universal_canvas,
            ai_assistant_view: AiAssistantView::default(),
            spatial_workspace,

            projects: crate::core::InternalVault::default_nvw_dcr_lod_projects(),
            db_components: crate::core::InternalVault::load_or_init_parametric_components(),
            members: crate::core::InternalVault::default_nvw_dcr_lod_members(),

            show_vault_startup_modal: false,
            vault_path_input: crate::core::InternalVault::default_vault_dir()
                .to_string_lossy()
                .to_string(),
            vault_new_name_input: "EigenGraph_Vault".to_string(),
            vault_modal_feedback: None,
            personal_vault_projects,
            last_vault_bridge_version: crate::core::web_bridge::vault_bridge_version(),
            autosave_hydrated: false,
            last_autosave_time: 0.0,
            last_autosave_hash: 0,
            last_bim_hash: 0,
            last_uc_earth: uc_earth,
            last_res_earth: res_earth,
            last_planning_sim_day: 0,
            pending_lazy_vault_entry: None,
        }
    }
}

const ALL_PILLARS: [NavigationPillar; 15] = [
    NavigationPillar::MyLibrary,
    NavigationPillar::AiAgent,
    NavigationPillar::ResearchConcept,
    NavigationPillar::ParametricDED,
    NavigationPillar::GeometryAssembly,
    NavigationPillar::AnalysisSimulation,
    NavigationPillar::TimeScheduling,
    NavigationPillar::CostBudgeting,
    NavigationPillar::SustainabilityEnergy,
    NavigationPillar::FacilityManagement,
    NavigationPillar::SafetyHealth,
    NavigationPillar::LeanConstruction,
    NavigationPillar::IndustrializedAutomation,
    NavigationPillar::Prediction,
    NavigationPillar::ReportPublishing,
];

impl EigenGraphApp {
    /// Initializes `EigenGraphApp` with 100% Pure WebGPU (`egui_wgpu` / WGSL) 3D Vector Earth pipelines.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        if let Some(render_state) = &cc.wgpu_render_state {
            app.universal_canvas.init_webgpu_earth(render_state);
        }
        app
    }

    /// Renders sidebar navigation contents identically for both the docked panel and mobile/tablet drawer.
    fn render_sidebar_contents(&mut self, ui: &mut egui::Ui, is_overlay: bool, is_mobile: bool) {
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(2.0);
                icons::icon_label(
                    ui,
                    AppIcon::Sparkles,
                    16.0,
                    self.theme.accent_blue,
                    "EigenGraph",
                    13.0,
                    self.theme.text_primary,
                    true,
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let close_icon = if is_overlay {
                        AppIcon::Close
                    } else {
                        AppIcon::ChevronLeft
                    };
                    let close_hint = if is_overlay {
                        "Close navigation drawer"
                    } else {
                        "Collapse sidebar"
                    };
                    if icons::icon_only_button(
                        ui,
                        close_icon,
                        self.theme.text_secondary,
                        self.theme.bg_card,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(26.0, 24.0),
                        close_hint,
                    )
                    .clicked()
                    {
                        self.sidebar_open = false;
                    }

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(9.5)
                            .strong()
                            .color(self.theme.text_muted),
                    );
                });
            });
            ui.add_space(10.0);

            // Search Pill Bar
            Frame::new()
                .fill(self.theme.bg_card)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(10, 6))
                .stroke(Stroke::new(1.0, self.theme.border_card))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icons::draw_icon(ui, AppIcon::Search, 13.0, self.theme.text_muted);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("Search modules...")
                                .desired_width((ui.available_width() - 4.0).max(80.0))
                                .frame(Frame::NONE),
                        );
                    });
                });
            ui.add_space(10.0);

            let bottom_section_height = 52.0;
            let nav_area_height = (ui.available_height() - bottom_section_height).max(100.0);
            let nav_item_h = if is_overlay || is_mobile { 38.0 } else { 32.0 };

            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), nav_area_height),
                Layout::top_down(Align::Min),
                |ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            for pillar in ALL_PILLARS {
                                if !self.search_query.trim().is_empty() {
                                    let q = self.search_query.trim().to_lowercase();
                                    let name = pillar.clean_name().to_lowercase();
                                    if !name.contains(&q) {
                                        continue;
                                    }
                                }

                                let is_active = self.active_pillar == pillar;
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), nav_item_h),
                                    egui::Sense::click(),
                                );

                                if response.clicked() {
                                    self.active_pillar = pillar;
                                    if is_overlay {
                                        self.sidebar_open = false;
                                    }
                                }

                                let is_hovered = response.hovered();
                                let fill_color = if is_active {
                                    self.theme.accent_blue.gamma_multiply(0.22)
                                } else if is_hovered {
                                    self.theme.bg_card_hover
                                } else {
                                    Color32::TRANSPARENT
                                };

                                if fill_color != Color32::TRANSPARENT {
                                    ui.painter().rect_filled(
                                        rect,
                                        CornerRadius::same(8),
                                        fill_color,
                                    );
                                }

                                if is_active {
                                    ui.painter().rect_stroke(
                                        rect,
                                        CornerRadius::same(8),
                                        Stroke::new(
                                            1.0,
                                            self.theme.accent_blue.gamma_multiply(0.55),
                                        ),
                                        StrokeKind::Inside,
                                    );
                                    let accent_bar = Rect::from_min_max(
                                        Pos2::new(rect.left() + 2.0, rect.top() + 6.0),
                                        Pos2::new(rect.left() + 5.5, rect.bottom() - 6.0),
                                    );
                                    ui.painter().rect_filled(
                                        accent_bar,
                                        CornerRadius::same(2),
                                        self.theme.accent_blue,
                                    );
                                }

                                let text_color = if is_active || is_hovered {
                                    self.theme.text_primary
                                } else {
                                    self.theme.text_secondary
                                };

                                let icon_color = if is_active {
                                    self.theme.accent_blue
                                } else {
                                    text_color
                                };
                                let icon_rect = Rect::from_center_size(
                                    Pos2::new(rect.left() + 18.0, rect.center().y),
                                    Vec2::splat(14.0),
                                );
                                icons::paint_icon(
                                    ui.painter(),
                                    icon_rect,
                                    pillar.app_icon(),
                                    icon_color,
                                );

                                ui.painter().text(
                                    rect.left_center() + Vec2::new(32.0, 0.0),
                                    Align2::LEFT_CENTER,
                                    pillar.clean_name(),
                                    egui::FontId::proportional(12.2),
                                    text_color,
                                );

                                ui.add_space(2.0);
                            }
                        });
                },
            );

            // Bottom Pinned Dock: Spatial Mode Switch + Workspace Settings
            ui.separator();
            ui.add_space(6.0);

            if icons::icon_button(
                ui,
                AppIcon::Sparkles,
                "Antigravity Spatial Mode",
                11.5,
                self.theme.accent_blue,
                self.theme.accent_blue.gamma_multiply(0.14),
                Stroke::new(1.0, self.theme.accent_blue.gamma_multiply(0.35)),
                CornerRadius::same(7),
                Vec2::new(ui.available_width(), 30.0),
            )
            .on_hover_text("Return to zero-chrome Full-Canvas Earth & AI Command mode")
            .clicked()
            {
                self.spatial_workspace.spatial_mode_enabled = true;
                self.universal_canvas.is_frozen = false;
                if is_overlay {
                    self.sidebar_open = false;
                }
            }
            ui.add_space(4.0);

            if icons::icon_button(
                ui,
                AppIcon::SettingsGear,
                "Settings",
                11.5,
                self.theme.text_secondary,
                self.theme.bg_card,
                Stroke::NONE,
                CornerRadius::same(7),
                Vec2::new(ui.available_width(), 32.0),
            )
            .on_hover_text("Workspace Settings & Future Roadmap")
            .clicked()
            {
                self.show_settings = !self.show_settings;
                if is_overlay {
                    self.sidebar_open = false;
                }
            }
        });
    }

}

impl App for EigenGraphApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Keep 2D BIM (Parametric) and 3D BIM (Geometry) projects bidirectionally synced on pillar switches
        if self.active_pillar != self.last_active_pillar {
            if self.last_active_pillar == NavigationPillar::ParametricDED {
                self.build_view.bim_bus.project = self.parametric_view.bim_bus.project.clone();
            } else if self.active_pillar == NavigationPillar::ParametricDED {
                self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
            }
            self.last_active_pillar = self.active_pillar;
        }

        if let Some(imported) = self.universal_canvas.pending_imported_settings.take() {
            self.settings_view.apply_workspace_settings(
                &imported,
                &mut self.theme,
                &mut self.ai_assistant_view.is_visible,
            );
            if self.active_pillar == NavigationPillar::ParametricDED {
                self.build_view.bim_bus.project = self.parametric_view.bim_bus.project.clone();
            } else {
                self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
            }
            self.sync_earth_maps();
            self.sync_lifecycle_from_canvas();
        }

        self.theme.apply_to_ctx(&ctx);

        // Synchronize async Web Vault state (File System Access API / IndexedDB) with UI
        let cur_vault_ver = crate::core::web_bridge::vault_bridge_version();
        if cur_vault_ver != self.last_vault_bridge_version {
            self.last_vault_bridge_version = cur_vault_ver;
            self.personal_vault_projects =
                crate::core::InternalVault::list_personal_vault_projects();
            crate::core::InternalVault::sync_world_spatial_index_with_vault(
                &self.personal_vault_projects,
                &mut self.universal_canvas.world_index,
            );
            if crate::core::InternalVault::has_connected_vault() {
                self.show_vault_startup_modal = false;
                if let Some(sess_json) = crate::core::InternalVault::load_ai_sessions_json() {
                    let _ = self
                        .spatial_workspace
                        .restore_sessions_json(&sess_json, &mut self.ai_assistant_view);
                    self.spatial_workspace.vault_sessions_hydrated = true;
                }
            }
        }
        if !self.spatial_workspace.vault_sessions_hydrated
            && crate::core::InternalVault::has_connected_vault()
        {
            self.spatial_workspace.vault_sessions_hydrated = true;
            if let Some(sess_json) = crate::core::InternalVault::load_ai_sessions_json() {
                let _ = self
                    .spatial_workspace
                    .restore_sessions_json(&sess_json, &mut self.ai_assistant_view);
            }
        }
        if self.show_settings {
            if self.spatial_workspace.gemini_api_key_draft != self.settings_view.gemini_api_key
                || self.spatial_workspace.remember_gemini_key
                    != self.settings_view.remember_gemini_key
            {
                self.spatial_workspace.gemini_api_key_draft =
                    self.settings_view.gemini_api_key.clone();
                self.spatial_workspace.remember_gemini_key =
                    self.settings_view.remember_gemini_key;
            }
        }
        if self.spatial_workspace.sessions_dirty && crate::core::InternalVault::has_connected_vault()
        {
            let json = self
                .spatial_workspace
                .export_sessions_json(&self.ai_assistant_view);
            let _ = crate::core::InternalVault::save_ai_sessions_json(&json);
            self.spatial_workspace.sessions_dirty = false;
        }
        #[cfg(target_arch = "wasm32")]
        if crate::core::web_bridge::vault_state_snapshot().busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        // Hydrate workspace autosave snapshot once on startup (or once IndexedDB finishes async hydration)
        if !self.autosave_hydrated {
            let workspace_blank = self.build_view.bim_bus.project.is_empty()
                && self.parametric_view.bim_bus.project.is_empty()
                && self.members.is_empty()
                && self.universal_canvas.entities.is_empty();
            if !workspace_blank {
                self.autosave_hydrated = true;
            } else if let Some(json) = crate::core::InternalVault::load_autosaved_workspace_json() {
                if let Ok(ws_cfg) = self.universal_canvas.apply_unified_archive(
                    &json,
                    &mut self.build_view.bim_bus.project,
                    &mut self.members,
                    &mut self.parametric_view.variables,
                ) {
                    self.settings_view.apply_workspace_settings(
                        &ws_cfg,
                        &mut self.theme,
                        &mut self.ai_assistant_view.is_visible,
                    );
                    self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                    self.sync_bim_to_fem();
                    self.sync_earth_maps();
                    self.sync_lifecycle_from_canvas();
                }
                self.autosave_hydrated = true;
            } else {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.autosave_hydrated = true;
                }
                #[cfg(target_arch = "wasm32")]
                if !crate::core::web_bridge::vault_state_snapshot().busy && cur_vault_ver > 0 {
                    self.autosave_hydrated = true;
                }
            }
        }

        self.spatial_workspace.observe_undo_stacks(
            self.build_view.bim_bus.undo_stack.len(),
            self.universal_canvas.undo_stack.len(),
        );

        // Global Ctrl/Cmd+K, F1, or Esc shortcuts
        if !ctx.egui_wants_keyboard_input() {
            if self.spatial_workspace.spatial_mode_enabled {
                if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
                    let next = !self.spatial_workspace.command_bar_expanded;
                    self.spatial_workspace.command_bar_expanded = next;
                    if next {
                        self.spatial_workspace.request_focus_command = true;
                    } else {
                        self.spatial_workspace.collapse_dynamic_island();
                    }
                }
                if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
                    self.universal_canvas.shortcuts_modal_open =
                        !self.universal_canvas.shortcuts_modal_open;
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    if self.show_settings {
                        self.show_settings = false;
                    } else if self.show_vault_startup_modal {
                        self.show_vault_startup_modal = false;
                    } else if self.universal_canvas.import_json_open {
                        self.universal_canvas.import_json_open = false;
                    } else if self.universal_canvas.shortcuts_modal_open {
                        self.universal_canvas.shortcuts_modal_open = false;
                    } else if self.spatial_workspace.api_key_popover_open {
                        self.spatial_workspace.api_key_popover_open = false;
                    } else if self.spatial_workspace.context_picker_open {
                        self.spatial_workspace.context_picker_open = false;
                    } else if self.spatial_workspace.history_popover_open {
                        self.spatial_workspace.history_popover_open = false;
                    } else if self.spatial_workspace.vault_path_popover_open {
                        self.spatial_workspace.vault_path_popover_open = false;
                    } else if self.spatial_workspace.vault_drawer_open {
                        self.spatial_workspace.vault_drawer_open = false;
                    } else if self.spatial_workspace.active_overlay_pillar.is_some() {
                        self.spatial_workspace.active_overlay_pillar = None;
                    } else if !self.spatial_workspace.command_input.is_empty() {
                        self.spatial_workspace.command_input.clear();
                    } else if self.spatial_workspace.command_bar_expanded
                        || self.spatial_workspace.mobile_deck_pinned
                    {
                        self.spatial_workspace.collapse_dynamic_island();
                    } else if self.spatial_workspace.has_active_manual_tool(
                        &self.build_view,
                        &self.universal_canvas,
                    ) {
                        self.spatial_workspace.reset_manual_tools_to_orbit(
                            &mut self.build_view,
                            &mut self.universal_canvas,
                        );
                    } else if matches!(
                        self.spatial_workspace.selected_target,
                        Some(crate::views::spatial_workspace::SmartSelectionTarget::EarthSite { .. })
                    ) {
                        self.spatial_workspace.selected_target = None;
                        self.universal_canvas.last_picked_earth_lat_lon = None;
                    } else if !self.spatial_workspace.popup_pinned {
                        self.spatial_workspace.selected_target = None;
                        self.universal_canvas.highlighted_bim_element = None;
                        self.universal_canvas.last_picked_earth_lat_lon = None;
                    }
                }
            } else if ctx.input(|i| {
                (i.modifiers.command && i.key_pressed(egui::Key::K))
                    || i.key_pressed(egui::Key::F1)
            }) {
                self.universal_canvas.shortcuts_modal_open =
                    !self.universal_canvas.shortcuts_modal_open;
            }
        }

        // Handle Text-to-BIM commands dispatched from Dynamic Island Copilot or AI Firm
        if let Some(cmd) = self.ai_assistant_view.pending_bim_nl_command.take() {
            let result_msg = if cmd == "__repair_critical__" {
                match self.build_view.bim_bus.repair_critical_members(&self.members) {
                    Ok(rep) => {
                        self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                        self.build_view.bim_console_logs.push(rep.summary.clone());
                        self.build_view.modify_status_msg = Some(format!(
                            "Auto-Repaired: Min EigenGate {:.3}->{:.3}, Max DCR {:.0}%->{:.0}%",
                            rep.before_min_eigengate,
                            rep.after_min_eigengate,
                            rep.before_max_dcr * 100.0,
                            rep.after_max_dcr * 100.0
                        ));
                        self.sync_bim_to_fem();
                        rep.summary
                    }
                    Err(err) => err,
                }
            } else {
                match self.build_view.bim_bus.execute_natural_language(&cmd) {
                    Ok(msg) => {
                        self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                        self.build_view.bim_console_logs.push(msg.clone());
                        self.build_view.modify_status_msg = Some(msg.clone());
                        msg
                    }
                    Err(err) => err,
                }
            };
            self.ai_assistant_view
                .chat_history
                .push(crate::models::ChatMessage {
                    is_user: false,
                    message: format!("[BIM Bus Result]: {}", result_msg),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
        }
        if let Some(cmd) = self.ai_firm_view.pending_bim_nl_command.take() {
            match self.build_view.bim_bus.execute_natural_language(&cmd) {
                Ok(msg) => {
                    self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                    self.build_view.bim_console_logs.push(msg.clone());
                    self.build_view.modify_status_msg = Some(msg.clone());
                    self.ai_firm_view.live_logs.push(crate::views::ai_firm::AgentLogEntry {
                        timestamp: crate::core::web_bridge::unix_timestamp_label(),
                        role: crate::views::ai_firm::FirmRole::ProjectManager,
                        message: msg,
                        tag: "BIM-SYNC".to_string(),
                        is_critical: false,
                    });
                }
                Err(err) => {
                    self.ai_firm_view.live_logs.push(crate::views::ai_firm::AgentLogEntry {
                        timestamp: crate::core::web_bridge::unix_timestamp_label(),
                        role: crate::views::ai_firm::FirmRole::SeniorDrafter,
                        message: err,
                        tag: "BIM-ERR".to_string(),
                        is_critical: true,
                    });
                }
            }
        }

        // Poll local file picker imports & OS drag-and-drop files
        let mut incoming_files: Vec<crate::core::web_bridge::ImportedFilePayload> = Vec::new();
        while let Some(payload) = crate::core::web_bridge::poll_imported_file() {
            incoming_files.push(payload);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            for df in dropped {
                let p = df.path();
                let fname = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "dropped_file".to_string());
                if let Ok(content) = std::fs::read_to_string(p) {
                    incoming_files.push(crate::core::web_bridge::ImportedFilePayload {
                        filename: fname,
                        content,
                    });
                }
            }
        }
        for payload in incoming_files {
            if std::mem::take(&mut self.spatial_workspace.pending_context_file_pick) {
                self.spatial_workspace
                    .attach_local_file_context(payload.filename, &payload.content);
            } else {
                self.handle_imported_file(payload);
            }
        }

        // Phase 81: Poll asynchronous WebSocket Collab IFCX Delta Sync Relay events
        let relay_events = crate::core::web_bridge::collab_relay_poll_events();
        if !relay_events.is_empty() {
            let local_doc = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let base_doc = self
                .universal_canvas
                .collab_state
                .checkpoints
                .first()
                .map(|c| c.snapshot.clone())
                .unwrap_or_else(|| local_doc.clone());
            for ev in relay_events {
                if let Ok(incoming) = crate::core::ifcx::IfcxDocument::parse(&ev.payload_json) {
                    let msg = self.universal_canvas.collab_state.ingest_relay_peer_doc(
                        &base_doc,
                        &local_doc,
                        &ev.peer_author,
                        self.universal_canvas.collab_state.peer_discipline,
                        &incoming,
                    );
                    self.spatial_workspace.push_trace(msg, true);
                }
            }
        }

        // Poll asynchronous chunked binary streaming for arbitrary-size (>4 MB) Vault/Picker files in Browser WASM
        match crate::core::web_bridge::poll_file_stream_event() {
            crate::core::web_bridge::FileStreamPollResult::Idle => {}
            crate::core::web_bridge::FileStreamPollResult::Progress {
                filename,
                loaded_bytes,
                total_bytes,
            } => {
                self.spatial_workspace.vault_feedback = Some(format!(
                    "Streaming '{}': {:.1} / {:.1} MB...",
                    filename,
                    loaded_bytes as f64 / 1_048_576.0,
                    total_bytes.max(1) as f64 / 1_048_576.0
                ));
                ctx.request_repaint();
            }
            crate::core::web_bridge::FileStreamPollResult::Completed {
                filename,
                rel_path,
                mode,
                content,
            } => {
                if mode == "vault_open" {
                    let entry = self
                        .pending_lazy_vault_entry
                        .take()
                        .unwrap_or_else(|| crate::core::vault::PersonalVaultProjectEntry {
                            name: filename.clone(),
                            filename: filename.clone(),
                            rel_path: rel_path.clone(),
                            abs_path: rel_path.clone(),
                            category: "Vault".to_string(),
                            node_count: 0,
                            member_count: 0,
                            size_bytes: content.len(),
                            updated_label: "Streamed".to_string(),
                        });
                    self.open_vault_raw_payload(&entry, &content);
                } else {
                    let payload = crate::core::web_bridge::ImportedFilePayload { filename, content };
                    if std::mem::take(&mut self.spatial_workspace.pending_context_file_pick) {
                        self.spatial_workspace
                            .attach_local_file_context(payload.filename, &payload.content);
                    } else {
                        self.handle_imported_file(payload);
                    }
                }
            }
        }

        // Keep 2D/3D BIM models, analytical FEM members, and Research <-> Ground Earth map bidirectionally synced
        let h2d = self.parametric_view.bim_bus.project.content_revision_hash();
        let h3d = self.build_view.bim_bus.project.content_revision_hash();
        if h2d != h3d {
            if self.active_pillar == NavigationPillar::ParametricDED {
                self.build_view.bim_bus.project = self.parametric_view.bim_bus.project.clone();
            } else {
                self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
            }
        }

        let cur_bim_hash = self.build_view.bim_bus.project.content_revision_hash();
        if cur_bim_hash != self.last_bim_hash
            || std::mem::take(&mut self.analyze_view.request_bim_fem_sync)
        {
            self.last_bim_hash = cur_bim_hash;
            self.sync_bim_to_fem();
            self.sync_active_building_to_world_index(false);
        }
        if let Some((rel, lat, lon, hdg)) = self.universal_canvas.pending_world_pin_event.take() {
            crate::core::InternalVault::save_world_spatial_index(&self.universal_canvas.world_index);
            self.spatial_workspace.push_trace(
                format!("Pinned '{}' on Earth @ ({:.4}°, {:.4}° | Hdg {:.0}°)", rel, lat, lon, hdg),
                true,
            );
        }
        if let Some(clicked_rel) = self.universal_canvas.clicked_world_building_rel_path.take() {
            if let Some(entry) = self
                .personal_vault_projects
                .iter()
                .find(|p| p.rel_path == clicked_rel)
                .cloned()
                .or_else(|| {
                    self.universal_canvas.world_index.get(&clicked_rel).map(|b| {
                        crate::core::vault::PersonalVaultProjectEntry {
                            name: b.name.clone(),
                            filename: b.filename.clone(),
                            rel_path: b.rel_path.clone(),
                            abs_path: crate::core::InternalVault::vault_root_dir()
                                .join(&b.rel_path)
                                .to_string_lossy()
                                .to_string(),
                            category: "World".to_string(),
                            node_count: b.story_count,
                            member_count: b.member_count,
                            size_bytes: 0,
                            updated_label: b.source.short_label().to_string(),
                        }
                    })
                })
            {
                self.open_vault_project_entry(entry);
            }
        }
        self.sync_earth_maps();
        if let Some(res) = self.universal_canvas.poll_async_ground_motion_replay() {
            match res {
                Ok(summary) => {
                    self.spatial_workspace.push_trace(summary.clone(), true);
                    self.ai_assistant_view
                        .chat_history
                        .push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!(
                                "[EigenAI Async StructGNN-CRC & Ground-Motion Replay]: {summary}"
                            ),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    ctx.request_repaint();
                }
                Err(err) => {
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        } else if self.universal_canvas.replay_async_manager.is_running() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        if self.universal_canvas.shm_watchdog.is_streaming()
            || crate::core::web_bridge::shm_connection_snapshot().0
        {
            let dt_s = ctx.input(|i| i.stable_dt).clamp(0.01, 0.25) as f64;
            let wd_out = self
                .universal_canvas
                .tick_live_shm_watchdog(dt_s, &self.build_view.bim_bus.project);
            if let Some(banner) = wd_out.alert_banner {
                self.spatial_workspace.push_trace(banner.clone(), false);
                self.ai_assistant_view
                    .chat_history
                    .push(crate::models::ChatMessage {
                        is_user: false,
                        message: format!("[EigenAI Live SHM Watchdog Escalation]: {banner}"),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
                if self.universal_canvas.earth_bim_author.active_lens
                    != crate::views::universal_canvas::EarthBimLens::ShmDigitalTwin
                {
                    self.spatial_workspace.apply_lens_to_views(
                        crate::views::spatial_workspace::SpatialLens::ShmDigitalTwin,
                        &mut self.build_view,
                        &mut self.universal_canvas,
                    );
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        if self.universal_canvas.earth_bim_author.active_lens
            == crate::views::universal_canvas::EarthBimLens::TimeHistoryReplay
            && self.universal_canvas.replay_anim_playing
            && self.universal_canvas.last_ground_motion_replay_report.is_some()
        {
            let dt_s = ctx.input(|i| i.stable_dt) as f64;
            self.universal_canvas
                .advance_ground_motion_replay_clock(dt_s);
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        if self.universal_canvas.eigenflow_pushover_playing {
            let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.25);
            self.universal_canvas.eigenflow_play_accum_sec += dt;
            if self.universal_canvas.eigenflow_play_accum_sec >= 0.45 {
                self.universal_canvas.eigenflow_play_accum_sec = 0.0;
                let n_steps = self
                    .universal_canvas
                    .last_eigenflow_report
                    .as_ref()
                    .map(|r| r.pushover_csm.raw_pushover.steps.len().max(1))
                    .unwrap_or(12);
                self.universal_canvas.eigenflow_config.active_step_idx =
                    (self.universal_canvas.eigenflow_config.active_step_idx + 1) % n_steps;
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
        if self.universal_canvas.fire_sim_playing {
            let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.25);
            self.universal_canvas.fire_play_accum_sec += dt;
            if self.universal_canvas.fire_play_accum_sec >= 0.35 {
                self.universal_canvas.fire_play_accum_sec = 0.0;
                let mut next_m = self.universal_canvas.extreme_events_config.exposure_minutes + 15.0;
                if next_m > 180.0 {
                    next_m = 0.0;
                }
                self.universal_canvas.extreme_events_config.exposure_minutes = next_m;
                let _ = self
                    .universal_canvas
                    .run_extreme_events_audit(&self.build_view.bim_bus.project);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
        if self.planning_view.simulation_day != self.last_planning_sim_day {
            self.last_planning_sim_day = self.planning_view.simulation_day;
            if self.planning_view.simulation_day > 0 {
                self.build_view.simulation_4d_day = self.planning_view.simulation_day as usize;
                self.build_view.highlight_4d_erection = true;
            }
        }

        // Keyboard shortcut (Cmd+,) toggles Settings modal
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Comma)) {
            self.show_settings = !self.show_settings;
        }

        let screen_rect = ctx.content_rect();
        let is_mobile = MacTheme::is_mobile(&ctx);
        let is_compact = MacTheme::is_compact(&ctx);
        let workspace_snapshot = self.settings_view.workspace_settings_snapshot(
            &self.theme,
            &self.universal_canvas,
            self.ai_assistant_view.is_visible,
        );

        if self.spatial_workspace.spatial_mode_enabled {
            self.render_spatial_mode_shell(ui, &ctx, &workspace_snapshot, is_mobile);
        } else {
            self.render_classic_mode_shell(
                ui,
                &ctx,
                screen_rect,
                is_mobile,
                is_compact,
                &workspace_snapshot,
            );
        }

        self.maybe_autosave_workspace(&ctx, &workspace_snapshot);

        // 5-7. CLASSIC MODE DETACHED MODALS
        // In Spatial Mode, ALL pop-ups (Settings, Vault Setup, Import Archive, Shortcuts/Quick-Jump,
        // Smart Element Inspector, and all 15 Engineering Module Sheets) render natively inside the
        // single dynamically widening Liquid Glass Dynamic Island (ZERO detached pop-ups over Earth 3D).
        if !self.spatial_workspace.spatial_mode_enabled {
            // 5. CONSOLIDATED SETTINGS (WORKSPACE SETTINGS & FUTURE ROADMAP MODAL)
            if self.show_settings {
                let mut open = self.show_settings;
                let mut close_after_jump = false;
                egui::Window::new("Settings — Workspace & Future Roadmap")
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(true)
                    .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
                    .default_size(MacTheme::clamp_modal_size(&ctx, Vec2::new(820.0, 650.0)))
                    .max_size(MacTheme::modal_max_size(&ctx))
                    .frame(
                        Frame::new()
                            .fill(self.theme.bg_card)
                            .stroke(Stroke::new(1.0, self.theme.border_subtle))
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(Margin::same(if is_mobile { 12 } else { 18 })),
                    )
                    .show(&ctx, |ui| {
                        let active_bim_project =
                            if self.active_pillar == NavigationPillar::ParametricDED {
                                &self.parametric_view.bim_bus.project
                            } else {
                                &self.build_view.bim_bus.project
                            };
                        if self.settings_view.show(
                            ui,
                            &mut self.theme,
                            &mut self.universal_canvas,
                            &mut self.ai_assistant_view.is_visible,
                            active_bim_project,
                            &self.members,
                            &self.parametric_view.variables,
                            &mut self.feature_registry,
                            &mut self.active_pillar,
                        ) {
                            close_after_jump = true;
                        }
                    });
                self.show_settings = open && !close_after_jump;
                if std::mem::take(&mut self.settings_view.vault_state_changed) {
                    self.personal_vault_projects =
                        crate::core::InternalVault::list_personal_vault_projects();
                    if crate::core::InternalVault::has_connected_vault() {
                        self.show_vault_startup_modal = false;
                    }
                }
                if std::mem::take(&mut self.settings_view.request_open_vault_popup) {
                    self.show_settings = false;
                    self.show_vault_startup_modal = true;
                }
            }

            // 5B. VAULT CONNECTION STARTUP POP-UP (shown when no Vault is connected yet)
            self.render_vault_startup_modal_if_open(&ctx, is_mobile);

            // 6. UNIFIED SINGLE-FILE IMPORT MODAL (works across all 15 modules)
            {
                let import_bim_project =
                    if self.active_pillar == NavigationPillar::ParametricDED {
                        &mut self.parametric_view.bim_bus.project
                    } else {
                        &mut self.build_view.bim_bus.project
                    };
                self.universal_canvas.render_import_modal_if_open(
                    &ctx,
                    &self.theme,
                    import_bim_project,
                    &mut self.members,
                    &mut self.parametric_view.variables,
                );
            }

            // 7. CAD SHORTCUTS, CLI SYNTAX & MODULE QUICK-JUMP PALETTE (Ctrl/Cmd+K or ? Shortcuts)
            self.render_shortcuts_quick_jump_modal_if_open(&ctx);
        }

        // 8. 100% PURE-VECTOR SVG CAPTURE HOOK (AUTOMATED QA — 0% RASTER PIXELS)
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(path) = std::env::var("EIGENGRAPH_SCREENSHOT_PATH") {
            let svg = self.universal_canvas.export_svg(
                &self.build_view.bim_bus.project,
                self.active_pillar,
            );
            let _ = std::fs::write(&path, svg);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn on_exit(&mut self) {}
}

impl crate::views::spatial_workspace::SpatialIslandHost for EigenGraphApp {
    fn island_theme(&self) -> MacTheme {
        self.theme.clone()
    }

    fn split_island_refs(
        &mut self,
    ) -> (
        &MacTheme,
        &mut UniversalCanvasState,
        &mut BuildView,
        &mut AiAssistantView,
        &[StructuralMember],
        &[ProjectItem],
        &mut Vec<DatabaseComponent>,
        &[crate::core::vault::PersonalVaultProjectEntry],
    ) {
        (
            &self.theme,
            &mut self.universal_canvas,
            &mut self.build_view,
            &mut self.ai_assistant_view,
            &self.members,
            &self.projects,
            &mut self.db_components,
            &self.personal_vault_projects,
        )
    }
}

impl EigenGraphApp {
    /// Renders a true 100% Full-Viewport Spatial IFCX Earth & BIM Canvas with ZERO side panels and
    /// ZERO detached pop-ups: Settings, Vault Setup, Import Archive, Shortcuts/Quick-Jump, Smart
    /// Element Inspector, and all 15 Engineering Module Sheets morph seamlessly inside the single
    /// dynamically widening Liquid Glass Dynamic Island.
    fn render_spatial_mode_shell(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        workspace_snapshot: &crate::views::universal_canvas::WorkspaceCanvasSettings,
        _is_mobile: bool,
    ) {
        use crate::views::spatial_workspace::SpatialLens;

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum SpatialEmbeddedKind {
            Settings,
            VaultSetup,
            ImportWorkspace,
            Shortcuts,
            ModuleSheet(NavigationPillar),
        }

        if !ctx.egui_wants_keyboard_input() {
            if ctx.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z)) {
                let dom = self.spatial_workspace.pop_undo_domain(
                    self.build_view.bim_bus.undo_stack.len(),
                    self.universal_canvas.undo_stack.len(),
                );
                if dom.is_some() {
                    let mut undo_act = crate::views::spatial_workspace::SpatialHudActions::default();
                    undo_act.undo = true;
                    self.apply_spatial_hud_actions_with_domain(undo_act, dom, workspace_snapshot);
                }
            } else if ctx.input(|i| {
                i.modifiers.command
                    && ((i.modifiers.shift && i.key_pressed(egui::Key::Z))
                        || i.key_pressed(egui::Key::Y))
            }) {
                let dom = self.spatial_workspace.pop_redo_domain(
                    self.build_view.bim_bus.redo_stack.len(),
                    self.universal_canvas.redo_stack.len(),
                );
                if dom.is_some() {
                    let mut redo_act = crate::views::spatial_workspace::SpatialHudActions::default();
                    redo_act.redo = true;
                    self.apply_spatial_hud_actions_with_domain(redo_act, dom, workspace_snapshot);
                }
            }
        }

        let modal_open = self.show_settings
            || self.show_vault_startup_modal
            || self.universal_canvas.import_json_open
            || self.universal_canvas.shortcuts_modal_open
            || self.spatial_workspace.active_overlay_pillar.is_some();
        let pointer_blocked = modal_open || self.spatial_workspace.is_pointer_over_hud(ctx);

        let lens_pillar = match self.spatial_workspace.active_lens {
            SpatialLens::Clean => NavigationPillar::GeometryAssembly,
            SpatialLens::DcrHeatmap
            | SpatialLens::EigenGateRedundancy
            | SpatialLens::AxialForce
            | SpatialLens::ShearForce
            | SpatialLens::BendingMoment
            | SpatialLens::DeformedShape
            | SpatialLens::ModalVibration
            | SpatialLens::TimeHistoryReplay
            | SpatialLens::ShmDigitalTwin => NavigationPillar::AnalysisSimulation,
            SpatialLens::Erection4D => NavigationPillar::TimeScheduling,
        };

        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(self.theme.bg_canvas)
                    .inner_margin(Margin::ZERO),
            )
            .show(ui, |ui| {
                self.spatial_workspace.active_canvas_mode =
                    crate::views::spatial_workspace::SpatialCanvasMode::EarthGlobe;
                self.universal_canvas.globe_mode = true;
                self.universal_canvas.show_spatial_fullscreen(
                    ui,
                    &self.theme,
                    lens_pillar,
                    &mut self.build_view.bim_bus.project,
                    &self.members,
                    pointer_blocked,
                );
            });

        // Drain any 3D BIM placement commands created directly on Earth 3D
        let earth_cmds: Vec<_> = self
            .universal_canvas
            .pending_earth_bim_commands
            .drain(..)
            .collect();
        if !earth_cmds.is_empty() {
            let mut changed = false;
            for cmd in earth_cmds {
                if let Ok(msg) = self.build_view.bim_bus.execute(cmd) {
                    changed = true;
                    self.spatial_workspace.push_trace(msg, true);
                }
            }
            if changed {
                self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                self.sync_bim_to_fem();
                self.sync_earth_maps();
            }
        }

        self.spatial_workspace
            .sync_pick_events(&mut self.universal_canvas, &mut self.build_view);

        let gemini_actions = self.spatial_workspace.poll_gemini_replies(
            &mut self.universal_canvas,
            &mut self.build_view,
            &mut self.ai_assistant_view,
        );
        self.apply_spatial_hud_actions(gemini_actions, workspace_snapshot);
        if self.spatial_workspace.gemini_inflight_req_id.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        let active_embedded = if self.show_settings {
            Some((
                SpatialEmbeddedKind::Settings,
                "Settings — Workspace & About".to_string(),
                AppIcon::SettingsGear,
            ))
        } else if self.show_vault_startup_modal {
            Some((
                SpatialEmbeddedKind::VaultSetup,
                "Connect Your Engineering Vault".to_string(),
                AppIcon::Database,
            ))
        } else if self.universal_canvas.import_json_open {
            Some((
                SpatialEmbeddedKind::ImportWorkspace,
                "Import Unified Workspace (.eigen / .ifcx)".to_string(),
                AppIcon::UploadImport,
            ))
        } else if self.universal_canvas.shortcuts_modal_open {
            Some((
                SpatialEmbeddedKind::Shortcuts,
                "CAD Shortcuts, Precision CLI & Quick-Jump".to_string(),
                AppIcon::SlidersPanel,
            ))
        } else {
            self.spatial_workspace.active_overlay_pillar.map(|pillar| {
                (
                    SpatialEmbeddedKind::ModuleSheet(pillar),
                    format!("{} ({})", pillar.clean_name(), pillar.badge()),
                    pillar.app_icon(),
                )
            })
        };

        let mut close_embedded = false;
        let mut settings_jump_pillar: Option<NavigationPillar> = None;
        let mut shortcuts_jump_pillar: Option<NavigationPillar> = None;

        // Move SpatialWorkspaceState out momentarily (O(1)) so the Dynamic Island can host
        // full EigenGraphApp panels inside its single morphing glass capsule without borrow conflicts.
        let mut sw = std::mem::take(&mut self.spatial_workspace);
        let embedded_meta = active_embedded
            .as_ref()
            .map(|(_, title, icon)| (title.as_str(), *icon, &mut close_embedded));
        let embedded_kind = active_embedded.as_ref().map(|(kind, _, _)| *kind);

        let hud_actions = sw.render_hud_overlays_with_host(
            ctx,
            self,
            embedded_meta,
            |ui, app| match embedded_kind {
                Some(SpatialEmbeddedKind::Settings) => {
                    let prev_pillar = app.active_pillar;
                    let active_bim_project =
                        if app.active_pillar == NavigationPillar::ParametricDED {
                            &app.parametric_view.bim_bus.project
                        } else {
                            &app.build_view.bim_bus.project
                        };
                    if app.settings_view.show(
                        ui,
                        &mut app.theme,
                        &mut app.universal_canvas,
                        &mut app.ai_assistant_view.is_visible,
                        active_bim_project,
                        &app.members,
                        &app.parametric_view.variables,
                        &mut app.feature_registry,
                        &mut app.active_pillar,
                    ) {
                        app.show_settings = false;
                        if app.active_pillar != prev_pillar {
                            settings_jump_pillar = Some(app.active_pillar);
                        }
                    }
                    if std::mem::take(&mut app.settings_view.vault_state_changed) {
                        app.personal_vault_projects =
                            crate::core::InternalVault::list_personal_vault_projects();
                        if crate::core::InternalVault::has_connected_vault() {
                            app.show_vault_startup_modal = false;
                        }
                    }
                    if std::mem::take(&mut app.settings_view.request_open_vault_popup) {
                        app.show_settings = false;
                        app.show_vault_startup_modal = true;
                    }
                }
                Some(SpatialEmbeddedKind::VaultSetup) => {
                    if app.render_vault_startup_body(ui) {
                        app.show_vault_startup_modal = false;
                    }
                }
                Some(SpatialEmbeddedKind::ImportWorkspace) => {
                    let import_bim_project =
                        if app.active_pillar == NavigationPillar::ParametricDED {
                            &mut app.parametric_view.bim_bus.project
                        } else {
                            &mut app.build_view.bim_bus.project
                        };
                    if app.universal_canvas.render_import_modal_body(
                        ui,
                        &app.theme,
                        import_bim_project,
                        &mut app.members,
                        &mut app.parametric_view.variables,
                    ) {
                        app.universal_canvas.import_json_open = false;
                    }
                }
                Some(SpatialEmbeddedKind::Shortcuts) => {
                    if let Some(p) = app.render_shortcuts_quick_jump_body(ui) {
                        shortcuts_jump_pillar = Some(p);
                        app.universal_canvas.shortcuts_modal_open = false;
                    }
                }
                Some(SpatialEmbeddedKind::ModuleSheet(overlay_pillar)) => {
                    app.render_pillar_module_body(ui, overlay_pillar, workspace_snapshot, 0.0);
                }
                None => {}
            },
        );

        self.spatial_workspace = sw;

        if close_embedded {
            match embedded_kind {
                Some(SpatialEmbeddedKind::Settings) => self.show_settings = false,
                Some(SpatialEmbeddedKind::VaultSetup) => self.show_vault_startup_modal = false,
                Some(SpatialEmbeddedKind::ImportWorkspace) => {
                    self.universal_canvas.import_json_open = false;
                }
                Some(SpatialEmbeddedKind::Shortcuts) => {
                    self.universal_canvas.shortcuts_modal_open = false;
                }
                Some(SpatialEmbeddedKind::ModuleSheet(_)) => {
                    self.spatial_workspace.active_overlay_pillar = None;
                }
                None => {}
            }
        }
        if let Some(p) = settings_jump_pillar.or(shortcuts_jump_pillar) {
            self.active_pillar = p;
            self.spatial_workspace.active_overlay_pillar = Some(p);
        }

        self.apply_pillar_post_render_actions();

        if !self.show_settings
            && (self.settings_view.gemini_api_key != self.spatial_workspace.gemini_api_key_draft
                || self.settings_view.remember_gemini_key
                    != self.spatial_workspace.remember_gemini_key)
        {
            self.settings_view.gemini_api_key = self.spatial_workspace.gemini_api_key_draft.clone();
            self.settings_view.remember_gemini_key = self.spatial_workspace.remember_gemini_key;
        }
        self.apply_spatial_hud_actions(hud_actions, workspace_snapshot);
    }

    fn apply_spatial_hud_actions(
        &mut self,
        hud_actions: crate::views::spatial_workspace::SpatialHudActions,
        workspace_snapshot: &crate::views::universal_canvas::WorkspaceCanvasSettings,
    ) {
        self.apply_spatial_hud_actions_with_domain(hud_actions, None, workspace_snapshot);
    }

    fn apply_spatial_hud_actions_with_domain(
        &mut self,
        hud_actions: crate::views::spatial_workspace::SpatialHudActions,
        forced_domain: Option<crate::views::spatial_workspace::SpatialUndoDomain>,
        workspace_snapshot: &crate::views::universal_canvas::WorkspaceCanvasSettings,
    ) {
        if hud_actions.sync_fem_and_earth {
            self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
            self.sync_bim_to_fem();
            self.sync_earth_maps();
        }
        if hud_actions.fit_camera {
            self.build_view
                .fit_view_to_project(&mut self.universal_canvas, &self.members);
            if self.spatial_workspace.spatial_mode_enabled {
                let target = if self.universal_canvas.ground_origin_anchored {
                    self.universal_canvas.ground_origin_lat_lon
                } else {
                    self.universal_canvas.world_map.center_lat_lon
                };
                self.universal_canvas.globe_mode = true;
                self.universal_canvas.earth_world.start_fly_to(
                    &self.universal_canvas.world_map,
                    target.0,
                    target.1,
                    self.universal_canvas.world_map.zoom_level.max(18.0),
                    "Site Focus",
                );
            }
        }
        if hud_actions.export_archive {
            self.sync_lifecycle_to_canvas();
            self.universal_canvas.export_unified_file(
                &self.build_view.bim_bus.project,
                &self.members,
                &self.parametric_view.variables,
                workspace_snapshot.clone(),
            );
        }
        if hud_actions.export_ifcx || hud_actions.export_earth_ifcx {
            let active_proxy = self
                .universal_canvas
                .active_world_project_rel_path
                .as_deref()
                .and_then(|r| self.universal_canvas.world_index.get(r))
                .cloned();
            let (lat, lon) = if let Some(ref p) = active_proxy {
                (p.lat, p.lon)
            } else {
                self.universal_canvas.ground_origin_lat_lon
            };
            let has_geo = self.universal_canvas.ground_origin_anchored
                || active_proxy.is_some()
                || lat.abs() > 1e-4
                || lon.abs() > 1e-4;
            let elev_m = if let Some(ref p) = active_proxy {
                if p.elev_m.abs() > 0.1 {
                    p.elev_m
                } else {
                    self.universal_canvas.earth_world.effective_elevation_m(lat, lon) as f64
                }
            } else {
                self.universal_canvas.earth_world.effective_elevation_m(lat, lon) as f64
            };
            let heading_deg = active_proxy.as_ref().map(|p| p.heading_deg).unwrap_or(0.0);
            let city_hint = active_proxy
                .as_ref()
                .map(|p| p.address_hint.as_str())
                .filter(|s| !s.trim().is_empty());
            let geo_opt = if has_geo { Some((lat, lon, elev_m)) } else { None };
            let city_opt = (!self.universal_canvas.city_scene.is_empty())
                .then_some(&self.universal_canvas.city_scene);
            let hazard_opt = self
                .universal_canvas
                .district_hazard
                .any_active()
                .then_some(&self.universal_canvas.district_hazard_report);
            let ifcx_doc = if hud_actions.export_earth_ifcx {
                crate::core::ifcx::IfcxStage::compose_earth_with_site_city_and_hazard(
                    self.universal_canvas.world_map.tile_provider,
                    &self.build_view.bim_bus.project,
                    geo_opt,
                    heading_deg,
                    city_hint,
                    Some(&self.members),
                    None,
                    city_opt,
                    hazard_opt,
                )
            } else {
                let mut doc = crate::core::ifcx::IfcxDocument::from_bim_project(
                    &self.build_view.bim_bus.project,
                    geo_opt,
                    heading_deg,
                    city_hint,
                    Some(&self.members),
                    None,
                );
                let site_path = format!(
                    "/site/{}",
                    self.build_view
                        .bim_bus
                        .project
                        .project_name
                        .chars()
                        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                        .collect::<String>()
                );
                if let Some(c) = city_opt {
                    for node in c.to_ifcx_nodes(&site_path) {
                        doc.data.push(node);
                    }
                }
                if let Some(hz) = hazard_opt {
                    doc.data.push(crate::core::gis::to_ifcx_hazard_node(&site_path, hz));
                }
                doc
            };
            let ifcx = ifcx_doc.to_json_pretty();
            let raw_name = if self.build_view.bim_bus.project.project_name.trim().is_empty() {
                "EigenGraph_BIM".to_string()
            } else {
                self.build_view
                    .bim_bus
                    .project
                    .project_name
                    .trim()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                    .collect()
            };
            let fname = if hud_actions.export_earth_ifcx {
                format!("{raw_name}_LivingEarth.ifcx")
            } else {
                format!("{raw_name}.ifcx")
            };
            match crate::core::web_bridge::save_text_file(&fname, &ifcx, "application/json;charset=utf-8") {
                Ok(_) => {
                    self.spatial_workspace.push_trace(
                        if hud_actions.export_earth_ifcx {
                            format!(
                                "Exported Composed Living Earth + Site IFCX ({fname}, {} nodes)",
                                ifcx_doc.data.len()
                            )
                        } else if has_geo {
                            format!("Exported Georeferenced IFCX ({fname} @ {lat:.4}°, {lon:.4}°)")
                        } else {
                            format!("Exported IFCX ({fname})")
                        },
                        true,
                    );
                }
                Err(err) => {
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        }
        if hud_actions.save_audit_note {
            let (lat, lon) = self.universal_canvas.ground_origin_lat_lon;
            let site = crate::core::compute_site_environmental_coupling(
                lat,
                lon,
                &self.universal_canvas.earth_world,
            );
            let md = crate::core::InternalVault::build_engineering_audit_markdown(
                &self.build_view.bim_bus.project,
                &self.members,
                lat,
                lon,
                f64::from(site.elevation_m),
                f64::from(site.seismic_pga_ss_g),
                f64::from(site.design_wind_ms),
            );
            let folder = if self.spatial_workspace.vault_note_folder.trim().is_empty() {
                "KnowledgeBase"
            } else {
                self.spatial_workspace.vault_note_folder.trim()
            };
            let stem = format!(
                "Audit_{}",
                if self.build_view.bim_bus.project.project_name.trim().is_empty() {
                    "EigenGraph"
                } else {
                    self.build_view.bim_bus.project.project_name.trim()
                }
            );
            match crate::core::InternalVault::save_markdown_note_in_folder(folder, &stem, &md) {
                Ok(rel) => {
                    self.spatial_workspace.vault_feedback = Some(format!("Saved Audit -> {rel}"));
                    self.spatial_workspace.previewed_vault_note = Some((rel.clone(), md));
                    self.spatial_workspace.vault_drawer_open = true;
                    self.spatial_workspace.push_trace(format!("Saved {rel}"), true);
                }
                Err(err) => {
                    self.spatial_workspace.vault_feedback = Some(err);
                }
            }
        }
        if hud_actions.toggle_theme {
            let next_mode = if self.theme.is_dark() {
                crate::theme::ThemeMode::EngineeringLight
            } else {
                crate::theme::ThemeMode::ProDark
            };
            self.theme = MacTheme::for_mode(next_mode);
        }
        if hud_actions.open_settings {
            self.show_settings = true;
        }
        if hud_actions.switch_to_classic_ui {
            self.spatial_workspace.spatial_mode_enabled = false;
        }
        if hud_actions.restore_n1_simulation {
            self.universal_canvas.earth_fem_cache.n1_simulation = None;
            self.build_view.simulated_eliminated_ids.clear();
            self.spatial_workspace
                .push_trace("Restored intact structure (cleared N-1 disruption)", true);
        }
        if let Some(fid) = hud_actions.toggle_n1_member {
            self.toggle_n1_disruption_on_fem_id(fid);
        }
        if hud_actions.run_auto_repair {
            let msg = match self.build_view.bim_bus.repair_critical_members(&self.members) {
                Ok(rep) => {
                    self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                    self.sync_bim_to_fem();
                    rep.summary
                }
                Err(err) => err,
            };
            self.spatial_workspace.push_trace(msg.clone(), true);
            self.ai_assistant_view
                .chat_history
                .push(crate::models::ChatMessage {
                    is_user: false,
                    message: format!("[Antigravity Auto-Repair]: {msg}"),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
        }
        if hud_actions.add_vault_folder {
            #[cfg(target_arch = "wasm32")]
            {
                crate::core::web_bridge::vault_connect_existing();
                self.spatial_workspace.vault_feedback = Some(
                    "Opening browser folder picker (or IndexedDB Vault)...".to_string(),
                );
                self.spatial_workspace
                    .push_trace("Choose local folder for Vault", true);
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(chosen) = crate::core::web_bridge::pick_folder_native(
                    "Select Folder to Connect as EigenGraph Engineering Vault",
                ) {
                    let path = std::path::PathBuf::from(&chosen);
                    match crate::core::InternalVault::connect_existing_vault(path) {
                        Ok(connected) => {
                            let s = connected.to_string_lossy().to_string();
                            self.vault_path_input = s.clone();
                            self.settings_view.vault_path_input = s;
                            self.spatial_workspace.vault_inline_path = chosen;
                            self.spatial_workspace.vault_feedback = None;
                            self.spatial_workspace.vault_drawer_open = true;
                            self.personal_vault_projects =
                                crate::core::InternalVault::list_personal_vault_projects();
                            self.show_vault_startup_modal = false;
                            self.spatial_workspace
                                .push_trace("Connected local Folder as Vault", true);
                        }
                        Err(err) => {
                            self.spatial_workspace.vault_feedback = Some(err);
                            self.spatial_workspace
                                .push_trace("Vault folder connection failed", false);
                        }
                    }
                } else {
                    self.spatial_workspace.vault_path_popover_open = true;
                }
            }
        }
        if let Some(idb_name) = hud_actions.connect_indexeddb_vault {
            crate::core::web_bridge::vault_connect_indexeddb(&idb_name);
            self.personal_vault_projects =
                crate::core::InternalVault::list_personal_vault_projects();
            self.spatial_workspace.vault_path_popover_open = false;
            self.spatial_workspace
                .push_trace(format!("Connected Browser Vault '{idb_name}'"), true);
        }
        if let Some(path_str) = hud_actions.connect_vault_path {
            let p = std::path::PathBuf::from(path_str.trim());
            match crate::core::InternalVault::connect_existing_vault(p) {
                Ok(connected) => {
                    let s = connected.to_string_lossy().to_string();
                    self.vault_path_input = s.clone();
                    self.settings_view.vault_path_input = s;
                    self.spatial_workspace.vault_feedback = None;
                    self.spatial_workspace.vault_path_popover_open = false;
                    self.spatial_workspace.vault_drawer_open = true;
                    self.personal_vault_projects =
                        crate::core::InternalVault::list_personal_vault_projects();
                    self.show_vault_startup_modal = false;
                    self.spatial_workspace
                        .push_trace("Bound custom folder as Vault", true);
                }
                Err(err) => {
                    self.spatial_workspace.vault_feedback = Some(err);
                }
            }
        }
        if let Some((dir_str, name_str)) = hud_actions.create_vault_path_and_name {
            let base = std::path::PathBuf::from(dir_str.trim());
            let clean_name = name_str.trim();
            let target = if clean_name.is_empty()
                || base.file_name().and_then(|s| s.to_str()) == Some(clean_name)
            {
                base
            } else {
                base.join(clean_name)
            };
            match crate::core::InternalVault::create_new_vault(
                target,
                Some(if clean_name.is_empty() {
                    "EigenGraph_Vault".to_string()
                } else {
                    clean_name.to_string()
                }),
                false,
            ) {
                Ok(created) => {
                    let s = created.to_string_lossy().to_string();
                    self.vault_path_input = s.clone();
                    self.settings_view.vault_path_input = s.clone();
                    self.spatial_workspace.vault_inline_path = s;
                    self.spatial_workspace.vault_feedback = None;
                    self.spatial_workspace.vault_path_popover_open = false;
                    self.spatial_workspace.vault_drawer_open = true;
                    self.personal_vault_projects =
                        crate::core::InternalVault::list_personal_vault_projects();
                    self.show_vault_startup_modal = false;
                    self.spatial_workspace
                        .push_trace("Initialized new Engineering Vault", true);
                }
                Err(err) => {
                    self.spatial_workspace.vault_feedback = Some(err);
                }
            }
        }
        if hud_actions.disconnect_vault {
            let _ = crate::core::InternalVault::disconnect_vault();
            self.personal_vault_projects.clear();
            self.universal_canvas.world_index = crate::core::gis::WorldSpatialIndex::default();
            self.universal_canvas.active_world_project_rel_path = None;
            self.spatial_workspace.vault_drawer_open = false;
            self.spatial_workspace.vault_path_popover_open = false;
            self.spatial_workspace
                .push_trace("Disconnected Engineering Vault", true);
        }
        if hud_actions.refresh_vault {
            crate::core::web_bridge::vault_refresh();
            self.personal_vault_projects =
                crate::core::InternalVault::list_personal_vault_projects();
            crate::core::InternalVault::sync_world_spatial_index_with_vault(
                &self.personal_vault_projects,
                &mut self.universal_canvas.world_index,
            );
            crate::core::InternalVault::save_world_spatial_index(&self.universal_canvas.world_index);
            self.spatial_workspace.push_trace("Refreshed Vault & World Spatial Index", true);
        }
        if hud_actions.save_active_to_vault {
            self.sync_lifecycle_to_canvas();
            let archive = self.universal_canvas.build_unified_archive(
                &self.build_view.bim_bus.project,
                &self.members,
                &self.parametric_view.variables,
                workspace_snapshot.clone(),
            );
            if let Ok(json) = archive.to_pretty_json() {
                let raw_name = if self.build_view.bim_bus.project.project_name.trim().is_empty() {
                    "Spatial_Earth_Project"
                } else {
                    self.build_view.bim_bus.project.project_name.trim()
                };
                let sanitized: String = raw_name
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                    .collect();
                let filename = format!("{sanitized}.eigen.json");
                match crate::core::InternalVault::save_project_archive_json(&filename, &json) {
                    Ok(_) => {
                        self.personal_vault_projects =
                            crate::core::InternalVault::list_personal_vault_projects();
                        let saved_rel = format!("Projects/{filename}");
                        self.universal_canvas.active_world_project_rel_path = Some(saved_rel);
                        self.sync_active_building_to_world_index(true);
                        self.spatial_workspace.vault_feedback =
                            Some(format!("Saved '{filename}' into Vault & World Index."));
                        self.spatial_workspace
                            .push_trace(format!("Saved {filename} to Vault"), true);
                    }
                    Err(err) => {
                        self.spatial_workspace.vault_feedback = Some(err);
                    }
                }
            }
        }
        if let Some(entry) = hud_actions.open_vault_project {
            self.open_vault_project_entry(entry);
        }
        if hud_actions.pick_local_context_file {
            self.spatial_workspace.pending_context_file_pick = true;
            crate::core::web_bridge::trigger_file_picker(
                ".eigen,.json,.geojson,.ifcx,.csv,.xyz,.pts,.md,.txt,.yaml,.yml",
            );
        }
        if let Some((lat, lon)) = hud_actions.anchor_and_build_site {
            self.universal_canvas.ground_origin_lat_lon = (lat, lon);
            self.universal_canvas.ground_origin_anchored = true;
            self.universal_canvas.globe_mode = true;
            self.universal_canvas.earth_world.camera.tilt_deg =
                self.universal_canvas.earth_world.camera.tilt_deg.clamp(12.0, 48.0);
            self.universal_canvas.earth_world.start_fly_to(
                &self.universal_canvas.world_map,
                lat,
                lon,
                18.2,
                "Anchored BIM Site",
            );
            if let Some(rel) = self.universal_canvas.active_world_project_rel_path.clone() {
                let hdg = self
                    .universal_canvas
                    .world_index
                    .get(&rel)
                    .map(|b| b.heading_deg)
                    .unwrap_or(0.0);
                self.universal_canvas.world_index.pin_building_at(&rel, lat, lon, hdg);
            }
            self.sync_active_building_to_world_index(true);
            self.spatial_workspace.active_canvas_mode =
                crate::views::spatial_workspace::SpatialCanvasMode::EarthGlobe;
            self.sync_bim_to_fem();
            self.build_view
                .fit_view_to_project(&mut self.universal_canvas, &self.members);
            self.sync_earth_maps();
        }
        if hud_actions.apply_site_seismic {
            self.sync_site_coupling();
            let coupling = self.universal_canvas.active_site_coupling();
            self.sync_bim_to_fem();
            self.spatial_workspace.push_trace(
                format!(
                    "Applied Earth Seismic Ss={:.2}g, Wind={:.0} m/s",
                    coupling.seismic_pga_ss_g, coupling.design_wind_ms
                ),
                true,
            );
        }
        if let Some(filter) = hud_actions.fly_to_next_world_building {
            let next_target = self
                .universal_canvas
                .world_index
                .next_building_matching(
                    filter,
                    self.universal_canvas.active_world_project_rel_path.as_deref(),
                )
                .cloned();
            if let Some(target_b) = next_target {
                self.universal_canvas.globe_mode = true;
                self.universal_canvas.earth_world.camera.tilt_deg = 40.0;
                self.universal_canvas.earth_world.start_fly_to(
                    &self.universal_canvas.world_map,
                    target_b.lat,
                    target_b.lon,
                    17.5,
                    &target_b.name,
                );
                self.universal_canvas.clicked_world_building_rel_path = Some(target_b.rel_path.clone());
                self.spatial_workspace.push_trace(
                    format!(
                        "Flying Earth 3D -> {} ({:.4}°, {:.4}°)",
                        target_b.name, target_b.lat, target_b.lon
                    ),
                    true,
                );
            } else {
                self.spatial_workspace.push_trace(
                    format!("No '{}' projects in World Index", filter.short_label()),
                    false,
                );
            }
        }
        if let Some(delta_deg) = hud_actions.rotate_active_world_heading_deg {
            if let Some(rel) = self.universal_canvas.active_world_project_rel_path.clone() {
                if let Some(b) = self.universal_canvas.world_index.get(&rel).cloned() {
                    let next_hdg = (b.heading_deg + delta_deg).rem_euclid(360.0);
                    self.universal_canvas.world_index.update_heading(&rel, next_hdg);
                    crate::core::InternalVault::save_world_spatial_index(&self.universal_canvas.world_index);
                    self.spatial_workspace.push_trace(
                        format!("True North heading -> {:.0}°", next_hdg),
                        true,
                    );
                }
            }
        }
        if hud_actions.undo {
            let domain = forced_domain.or_else(|| {
                self.spatial_workspace.pop_undo_domain(
                    self.build_view.bim_bus.undo_stack.len(),
                    self.universal_canvas.undo_stack.len(),
                )
            });
            match domain {
                Some(crate::views::spatial_workspace::SpatialUndoDomain::Canvas2D) => {
                    if self.universal_canvas.undo() {
                        self.spatial_workspace.push_trace("Undid 2D canvas action", true);
                    }
                }
                _ => {
                    if let Some(msg) = self.build_view.bim_bus.undo() {
                        self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                        self.sync_bim_to_fem();
                        self.sync_earth_maps();
                        self.spatial_workspace.push_trace(msg, true);
                    } else if self.universal_canvas.undo() {
                        self.spatial_workspace.push_trace("Undid 2D canvas action", true);
                    }
                }
            }
            self.spatial_workspace.sync_undo_baseline(
                self.build_view.bim_bus.undo_stack.len(),
                self.universal_canvas.undo_stack.len(),
            );
        }
        if hud_actions.redo {
            let domain = forced_domain.or_else(|| {
                self.spatial_workspace.pop_redo_domain(
                    self.build_view.bim_bus.redo_stack.len(),
                    self.universal_canvas.redo_stack.len(),
                )
            });
            match domain {
                Some(crate::views::spatial_workspace::SpatialUndoDomain::Canvas2D) => {
                    if self.universal_canvas.redo() {
                        self.spatial_workspace.push_trace("Redid 2D canvas action", true);
                    }
                }
                _ => {
                    if let Some(msg) = self.build_view.bim_bus.redo() {
                        self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                        self.sync_bim_to_fem();
                        self.sync_earth_maps();
                        self.spatial_workspace.push_trace(msg, true);
                    } else if self.universal_canvas.redo() {
                        self.spatial_workspace.push_trace("Redid 2D canvas action", true);
                    }
                }
            }
            self.spatial_workspace.sync_undo_baseline(
                self.build_view.bim_bus.undo_stack.len(),
                self.universal_canvas.undo_stack.len(),
            );
        }
        if hud_actions.run_district_hazard_audit {
            let coupling = self.universal_canvas.active_site_coupling();
            self.universal_canvas.district_hazard.seismic_pga_g =
                f64::from(coupling.seismic_pga_ss_g).clamp(0.15, 1.5);
            self.universal_canvas.district_hazard.base_wind_ms =
                f64::from(coupling.design_wind_ms).clamp(18.0, 65.0);
            self.universal_canvas
                .refresh_district_hazard(&self.build_view.bim_bus.project);
            let rep = &self.universal_canvas.district_hazard_report;
            self.spatial_workspace.push_trace(
                format!(
                    "Multi-Hazard: Resilience {:.0}/100 ({}) | Pounding {} | Wind {:.1}m/s | Flood {:.0}%",
                    rep.resilience_score,
                    rep.triage_label,
                    rep.pounding_risk_count,
                    rep.effective_wind_ms,
                    rep.flood.inundated_area_pct
                ),
                rep.resilience_score >= 45.0,
            );
        }
        if hud_actions.ingest_ai_real_building_capture {
            let (lat, lon) = if self.universal_canvas.ground_origin_anchored {
                self.universal_canvas.ground_origin_lat_lon
            } else {
                self.universal_canvas.world_map.center_lat_lon
            };
            let elev_m = self
                .universal_canvas
                .earth_world
                .effective_elevation_m(lat, lon) as f64;
            let raw_capture = crate::core::ifcx::IfcxDocument::sample_ai_massing_tower_capture_ifcx(
                lat,
                lon,
                elev_m,
                "AI_RealBuilding_Parcel",
            )
            .to_json_pretty();
            match crate::core::ifcx::IfcxDocument::ingest_ai_real_building_with_frame_inference(
                &raw_capture,
                "ai_parcel_capture.ifcx",
                Some((lat, lon)),
                6.0,
            ) {
                Ok((_, proj, rep)) => {
                    self.build_view.bim_bus = crate::core::BimCommandBus::from_project(proj.clone());
                    self.parametric_view.bim_bus = crate::core::BimCommandBus::from_project(proj);
                    self.parametric_view.active_level_idx = 0;
                    self.universal_canvas.ground_origin_lat_lon = (lat, lon);
                    self.universal_canvas.ground_origin_anchored = true;
                    self.universal_canvas.globe_mode = true;
                    self.universal_canvas.last_ai_capture_report = Some(rep.clone());
                    self.sync_bim_to_fem();
                    self.sync_earth_maps();
                    self.build_view
                        .fit_view_to_project(&mut self.universal_canvas, &self.members);
                    self.spatial_workspace.push_trace(
                        format!(
                            "AI -> IFCX '{}': {}F ({}C, {}B, {}S) | min EigenGate {:.2} | max DCR {:.0}% | {}",
                            rep.building_name,
                            rep.inferred_storeys,
                            rep.inferred_columns,
                            rep.inferred_beams,
                            rep.inferred_slabs,
                            rep.min_eigengate,
                            rep.max_dcr_pct,
                            rep.triage_grade
                        ),
                        rep.svd_nullspace_dim == 0,
                    );
                }
                Err(err) => {
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        }
        if let Some(strategy) = hud_actions.run_structural_retrofit {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                }
            }
            let baseline = if self.universal_canvas.viewing_pre_retrofit_baseline {
                self.build_view.bim_bus.project.clone()
            } else if self.universal_canvas.last_retrofit_report.is_some() {
                self.universal_canvas
                    .pre_retrofit_baseline
                    .clone()
                    .unwrap_or_else(|| self.build_view.bim_bus.project.clone())
            } else {
                self.build_view.bim_bus.project.clone()
            };
            let coupling = self.universal_canvas.active_site_coupling();
            match crate::core::retrofit::synthesize_structural_retrofit(
                &baseline,
                strategy,
                &coupling,
            ) {
                Ok(rep) => {
                    let undo_snapshot = self.build_view.bim_bus.project.clone();
                    self.build_view.bim_bus.undo_stack.push(undo_snapshot);
                    self.build_view.bim_bus.redo_stack.clear();
                    self.build_view.bim_bus.project = rep.retrofitted_project.clone();
                    self.build_view.bim_bus.revision =
                        self.build_view.bim_bus.revision.wrapping_add(1);
                    self.build_view.bim_bus.log.push(rep.summary.clone());
                    self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                    self.universal_canvas.pre_retrofit_baseline = Some(baseline);
                    self.universal_canvas.viewing_pre_retrofit_baseline = false;
                    self.universal_canvas.last_retrofit_report = Some(rep.clone());
                    self.sync_bim_to_fem();
                    self.sync_earth_maps();
                    self.build_view
                        .fit_view_to_project(&mut self.universal_canvas, &self.members);
                    self.spatial_workspace
                        .push_trace(rep.summary.clone(), rep.after_svd_mechanisms == 0);
                    self.ai_assistant_view
                        .chat_history
                        .push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[EigenAI Structural Retrofit]: {}", rep.summary),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                }
                Err(err) => {
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        }
        if hud_actions.toggle_retrofit_before_after {
            if let (Some(baseline), Some(rep)) = (
                self.universal_canvas.pre_retrofit_baseline.clone(),
                self.universal_canvas.last_retrofit_report.clone(),
            ) {
                let next_before = !self.universal_canvas.viewing_pre_retrofit_baseline;
                self.universal_canvas.viewing_pre_retrofit_baseline = next_before;
                self.build_view.bim_bus.project = if next_before {
                    baseline
                } else {
                    rep.retrofitted_project
                };
                self.build_view.bim_bus.revision =
                    self.build_view.bim_bus.revision.wrapping_add(1);
                self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
                self.sync_bim_to_fem();
                self.sync_earth_maps();
                self.spatial_workspace.push_trace(
                    if next_before {
                        format!(
                            "Viewing Pre-Retrofit Baseline (EigenGate {:.2}, DCR {:.0}%)",
                            rep.before_min_eigengate, rep.before_max_dcr_pct
                        )
                    } else {
                        format!(
                            "Viewing Retrofitted Structure (EigenGate {:.2}, DCR {:.0}%)",
                            rep.after_min_eigengate, rep.after_max_dcr_pct
                        )
                    },
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("Run /retrofit first to compare Before vs. After.", false);
            }
        }
        if hud_actions.export_retrofit_delta_ifcx {
            if self.universal_canvas.last_retrofit_report.is_none()
                && !self.build_view.bim_bus.project.is_empty()
            {
                let coupling = self.universal_canvas.active_site_coupling();
                if let Ok(rep) = crate::core::retrofit::synthesize_structural_retrofit(
                    &self.build_view.bim_bus.project,
                    self.universal_canvas.retrofit_strategy,
                    &coupling,
                ) {
                    self.universal_canvas.pre_retrofit_baseline =
                        Some(self.build_view.bim_bus.project.clone());
                    self.universal_canvas.last_retrofit_report = Some(rep);
                }
            }
            if let Some(ref rep) = self.universal_canvas.last_retrofit_report {
                let raw_name = Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_retrofit_delta.ifcx");
                let json = rep.retrofit_delta_ifcx.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported Non-Destructive Retrofit Delta IFCX ({fname}, {} nodes)",
                                rep.retrofit_delta_ifcx.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No retrofit delta available to export.", false);
            }
        }
        if let Some(daf) = hud_actions.run_crc_cascade_screening {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                }
            }
            let coupling = self.universal_canvas.active_site_coupling();
            let trigger_opt = self.universal_canvas.highlighted_bim_element;
            match crate::core::cascade::run_two_stage_crc_and_cascade_for_bim(
                &self.build_view.bim_bus.project,
                &coupling,
                trigger_opt,
                daf,
            ) {
                Ok(rep) => {
                    self.universal_canvas.cascade_daf = rep.daf;
                    self.universal_canvas.active_cascade_step = 1;
                    self.universal_canvas.last_crc_cascade_report = Some(rep.clone());
                    let is_safe = rep.cascade_status
                        != crate::core::cascade::CollapseStatus::CatastrophicCollapse;
                    self.spatial_workspace.push_trace(rep.summary.clone(), is_safe);
                    self.ai_assistant_view
                        .chat_history
                        .push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!(
                                "[EigenAI Two-Stage EigenGate + StructGNN-CRC]: {}",
                                rep.summary
                            ),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                }
                Err(err) => {
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        }
        if hud_actions.export_crc_cascade_ifcx {
            if self.universal_canvas.last_crc_cascade_report.is_none()
                && !self.build_view.bim_bus.project.is_empty()
            {
                let coupling = self.universal_canvas.active_site_coupling();
                if let Ok(rep) = crate::core::cascade::run_two_stage_crc_and_cascade_for_bim(
                    &self.build_view.bim_bus.project,
                    &coupling,
                    self.universal_canvas.highlighted_bim_element,
                    self.universal_canvas.cascade_daf,
                ) {
                    self.universal_canvas.active_cascade_step = 1;
                    self.universal_canvas.last_crc_cascade_report = Some(rep);
                }
            }
            if let Some(ref rep) = self.universal_canvas.last_crc_cascade_report {
                let raw_name = Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_crc_cascade.ifcx");
                let json = rep.crc_overlay_ifcx.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported Non-Destructive CRC Cascade IFCX ({fname}, {} nodes)",
                                rep.crc_overlay_ifcx.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No CRC cascade overlay available to export.", false);
            }
        }
        if hud_actions.run_subsurface_ssi_sync {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                }
            }
            self.sync_bim_to_fem();
            if let Some(ref rep) = self.universal_canvas.last_subsurface_ssi_report {
                self.spatial_workspace
                    .push_trace(rep.summary.clone(), !rep.is_liquefaction_critical);
                self.ai_assistant_view
                    .chat_history
                    .push(crate::models::ChatMessage {
                        is_user: false,
                        message: format!(
                            "[EigenAI Subsurface 6-DOF Winkler SSI]: {}",
                            rep.summary
                        ),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
            } else {
                self.spatial_workspace.push_trace(
                    "Subsurface 6-DOF Winkler SSI disabled (Fixed-Base FEM restored)",
                    true,
                );
            }
        }
        if hud_actions.export_subsurface_ssi_ifcx {
            if self.universal_canvas.last_subsurface_ssi_report.is_none()
                && !self.build_view.bim_bus.project.is_empty()
            {
                self.universal_canvas
                    .refresh_subsurface_ssi(&self.build_view.bim_bus.project);
            }
            if let Some(ref rep) = self.universal_canvas.last_subsurface_ssi_report {
                let raw_name =
                    Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_subsurface_ssi.ifcx");
                let base_ifcx = crate::core::IfcxDocument::from_bim_project(
                    &self.build_view.bim_bus.project,
                    None,
                    0.0,
                    None,
                    None,
                    None,
                );
                let composed = crate::core::IfcxStage::compose_with_subsurface_ssi_overlay(
                    &base_ifcx,
                    rep,
                );
                let json = composed.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported Non-Destructive Subsurface SSI IFCX ({fname}, {} nodes)",
                                composed.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No Subsurface SSI overlay available to export.", false);
            }
        }
        if hud_actions.capture_ifcx_baseline {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                    self.sync_bim_to_fem();
                }
            }
            self.universal_canvas.capture_ifcx_baseline(
                &self.build_view.bim_bus.project,
                &self.members,
                "Captured Snapshot",
            );
            let node_cnt = self
                .universal_canvas
                .ifcx_baseline_snapshot
                .as_ref()
                .map(|d| d.data.len())
                .unwrap_or(0);
            self.spatial_workspace.push_trace(
                format!("Captured IFCX Baseline Snapshot ({node_cnt} nodes)"),
                true,
            );
        }
        if hud_actions.run_ifcx_visual_diff {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                    self.sync_bim_to_fem();
                }
            }
            self.universal_canvas
                .refresh_ifcx_visual_diff(&self.build_view.bim_bus.project, &self.members);
            if let Some(ref diff) = self.universal_canvas.last_ifcx_visual_diff {
                self.spatial_workspace.push_trace(diff.summary.clone(), true);
            }
        }
        if hud_actions.export_ifcx_diff_manifest {
            if self.universal_canvas.last_ifcx_visual_diff.is_none()
                && !self.build_view.bim_bus.project.is_empty()
            {
                self.universal_canvas
                    .refresh_ifcx_visual_diff(&self.build_view.bim_bus.project, &self.members);
            }
            if let Some(ref diff) = self.universal_canvas.last_ifcx_visual_diff {
                let raw_name =
                    Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_ifcx_diff.ifcx");
                let layered = self
                    .universal_canvas
                    .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
                let composed = crate::core::IfcxStage::compose_with_diff_manifest(&layered, diff);
                let json = composed.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported IFCX Diff Manifest ({fname}, {} nodes)",
                                composed.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No IFCX Visual Diff manifest available to export.", false);
            }
        }
        if hud_actions.export_vector_dossier_svg {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                    self.sync_bim_to_fem();
                }
            }
            let site = self.universal_canvas.active_site_coupling();
            let lat_lon = if self.universal_canvas.ground_origin_anchored {
                self.universal_canvas.ground_origin_lat_lon
            } else {
                self.universal_canvas.world_map.center_lat_lon
            };
            let haz_opt = self
                .universal_canvas
                .district_hazard
                .any_active()
                .then_some(&self.universal_canvas.district_hazard_report);
            let dossier = crate::core::build_pure_vector_engineering_dossier_svg(
                &crate::core::VectorDossierInput {
                    project: &self.build_view.bim_bus.project,
                    members: &self.members,
                    site: &site,
                    lat_lon,
                    retrofit_report: self.universal_canvas.last_retrofit_report.as_ref(),
                    crc_report: self.universal_canvas.last_crc_cascade_report.as_ref(),
                    ssi_report: self.universal_canvas.last_subsurface_ssi_report.as_ref(),
                    hazard_report: haz_opt,
                    diff_report: self.universal_canvas.last_ifcx_visual_diff.as_ref(),
                    portfolio_report: self.universal_canvas.last_portfolio_retrofit_report.as_ref(),
                    replay_report: self.universal_canvas.last_ground_motion_replay_report.as_ref(),
                    shm_report: self.universal_canvas.last_shm_twin_report.as_ref(),
                    evacuation_report: self.universal_canvas.last_evacuation_report.as_ref(),
                    watchdog_state: (self.universal_canvas.shm_watchdog.total_packets_ingested > 0)
                        .then_some(&self.universal_canvas.shm_watchdog),
                    monte_carlo_report: self.universal_canvas.last_monte_carlo_report.as_ref(),
                    collab_state: Some(&self.universal_canvas.collab_state),
                    sheet_code: "S-101",
                    revision: "REV 01",
                },
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_engineering_dossier.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &dossier.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(dossier.sha256_seal);
                    self.spatial_workspace.push_trace(dossier.summary.clone(), true);
                    self.ai_assistant_view
                        .chat_history
                        .push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!(
                                "[EigenAI Pure-Vector Engineering Dossier]: {} ({fname})",
                                dossier.summary
                            ),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_portfolio_retrofit {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                    self.sync_bim_to_fem();
                }
            }
            self.universal_canvas
                .refresh_portfolio_retrofit(&self.build_view.bim_bus.project, &self.members);
            if let Some(ref rep) = self.universal_canvas.last_portfolio_retrofit_report {
                self.spatial_workspace.push_trace(rep.summary.clone(), true);
                self.ai_assistant_view
                    .chat_history
                    .push(crate::models::ChatMessage {
                        is_user: false,
                        message: format!(
                            "[EigenAI District Portfolio Retrofit]: {}",
                            rep.summary
                        ),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
            }
        }
        if hud_actions.export_portfolio_retrofit_ifcx {
            if self.universal_canvas.last_portfolio_retrofit_report.is_none() {
                self.universal_canvas
                    .refresh_portfolio_retrofit(&self.build_view.bim_bus.project, &self.members);
            }
            if let Some(ref rep) = self.universal_canvas.last_portfolio_retrofit_report {
                let raw_name =
                    Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_portfolio_retrofit.ifcx");
                let layered = self
                    .universal_canvas
                    .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
                let composed =
                    crate::core::IfcxStage::compose_with_portfolio_retrofit_overlay(&layered, rep);
                let json = composed.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported District Portfolio Retrofit IFCX ({fname}, {} nodes)",
                                composed.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No District Portfolio Retrofit overlay available.", false);
            }
        }
        if hud_actions.run_async_ground_motion_replay {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                    self.sync_bim_to_fem();
                }
            }
            self.universal_canvas
                .start_async_ground_motion_replay(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(
                format!(
                    "Streaming async StructGNN-CRC & {} ({} • {:.2}x) NL-THA...",
                    self.universal_canvas
                        .ground_motion_replay_config
                        .active_record_label(),
                    self.universal_canvas
                        .ground_motion_replay_config
                        .tridirectional_mode
                        .short_label(),
                    self.universal_canvas.ground_motion_replay_config.scale_factor
                ),
                true,
            );
        }
        if hud_actions.open_accelerogram_file_picker {
            crate::core::web_bridge::trigger_file_picker(".at2,.csv,.txt");
            self.spatial_workspace.push_trace(
                "Select a PEER NGA (.at2) or 2/3-column (.csv) accelerogram file...",
                true,
            );
        }
        if hud_actions.export_ground_motion_replay_ifcx {
            if self
                .universal_canvas
                .last_ground_motion_replay_report
                .is_none()
                && !self.build_view.bim_bus.project.is_empty()
            {
                let site = self.universal_canvas.active_site_coupling();
                if let Ok(rep) = crate::core::ground_motion_replay::run_ground_motion_replay_for_bim(
                    &self.build_view.bim_bus.project,
                    &site,
                    self.universal_canvas.last_subsurface_ssi_report.as_ref(),
                    &self.universal_canvas.ground_motion_replay_config,
                    "Direct IFCX Export",
                    None,
                ) {
                    self.universal_canvas.last_ground_motion_replay_report = Some(rep);
                }
            }
            if let Some(ref rep) = self.universal_canvas.last_ground_motion_replay_report {
                let raw_name =
                    Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_time_history_replay.ifcx");
                let layered = self
                    .universal_canvas
                    .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
                let composed =
                    crate::core::IfcxStage::compose_with_time_history_replay_overlay(&layered, rep);
                let json = composed.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported Ground-Motion Replay IFCX ({fname}, {} nodes)",
                                composed.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No Ground-Motion Replay overlay available.", false);
            }
        }
        if hud_actions.export_fragility_loss_ifcx {
            if self
                .universal_canvas
                .last_ground_motion_replay_report
                .is_none()
                && !self.build_view.bim_bus.project.is_empty()
            {
                let site = self.universal_canvas.active_site_coupling();
                if let Ok(rep) = crate::core::ground_motion_replay::run_ground_motion_replay_for_bim(
                    &self.build_view.bim_bus.project,
                    &site,
                    self.universal_canvas.last_subsurface_ssi_report.as_ref(),
                    &self.universal_canvas.ground_motion_replay_config,
                    "FEMA P-58 IFCX Export",
                    None,
                ) {
                    self.universal_canvas.last_ground_motion_replay_report = Some(rep);
                }
            }
            if let Some(ref rep) = self.universal_canvas.last_ground_motion_replay_report {
                let raw_name =
                    Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_fema_p58_fragility.ifcx");
                let layered = self
                    .universal_canvas
                    .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
                let composed =
                    crate::core::IfcxStage::compose_with_fragility_loss_overlay(&layered, rep);
                let json = composed.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported FEMA P-58 Fragility & Loss IFCX ({fname}, {} nodes)",
                                composed.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No FEMA P-58 Fragility synthesis available.", false);
            }
        }
        if hud_actions.open_shm_telemetry_picker {
            crate::core::web_bridge::trigger_file_picker(".csv,.json,.txt");
            self.spatial_workspace.push_trace(
                "Select an SHM sensor telemetry CSV or JSON log...",
                true,
            );
        }
        if hud_actions.run_shm_digital_twin {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                    self.sync_bim_to_fem();
                }
            }
            match self
                .universal_canvas
                .run_shm_digital_twin(&self.build_view.bim_bus.project)
            {
                Ok(summary) => {
                    self.spatial_workspace.push_trace(summary.clone(), true);
                    self.ai_assistant_view.chat_history.push(crate::models::ChatMessage {
                        is_user: false,
                        message: format!("[EigenAI Live SHM Digital Twin & ATC-20]: {summary}"),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
                }
                Err(err) => {
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        }
        if hud_actions.export_shm_atc20_ifcx {
            if self.universal_canvas.last_shm_twin_report.is_none()
                && !self.build_view.bim_bus.project.is_empty()
            {
                let _ = self
                    .universal_canvas
                    .run_shm_digital_twin(&self.build_view.bim_bus.project);
            }
            if let Some(ref shm) = self.universal_canvas.last_shm_twin_report {
                let raw_name =
                    Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_shm_atc20.ifcx");
                let layered = self
                    .universal_canvas
                    .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
                let composed =
                    crate::core::IfcxStage::compose_with_shm_atc20_overlay(&layered, shm);
                let json = composed.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported SHM & ATC-20 IFCX ({fname}, {} nodes)",
                                composed.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No Live SHM Digital Twin report available.", false);
            }
        }
        if hud_actions.run_evacuation_routing {
            if self.build_view.bim_bus.project.is_empty() {
                if let Ok(msg) = self
                    .build_view
                    .bim_bus
                    .execute_natural_language("portal 3x2 @6m 2floors")
                {
                    self.universal_canvas.ground_origin_anchored = true;
                    self.spatial_workspace.push_trace(msg, true);
                    self.sync_bim_to_fem();
                }
            }
            if self.universal_canvas.last_shm_twin_report.is_none() {
                let _ = self
                    .universal_canvas
                    .run_shm_digital_twin(&self.build_view.bim_bus.project);
            }
            match self
                .universal_canvas
                .run_evacuation_lifeline_routing(&self.build_view.bim_bus.project)
            {
                Ok(summary) => {
                    self.spatial_workspace.push_trace(summary.clone(), true);
                    self.ai_assistant_view
                        .chat_history
                        .push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!(
                                "[EigenAI Evacuation & Emergency Lifelines]: {summary}"
                            ),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                }
                Err(err) => {
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        }
        if hud_actions.export_evacuation_ifcx {
            if self.universal_canvas.last_evacuation_report.is_none()
                && !self.build_view.bim_bus.project.is_empty()
            {
                let _ = self
                    .universal_canvas
                    .run_evacuation_lifeline_routing(&self.build_view.bim_bus.project);
            }
            if let Some(ref evac) = self.universal_canvas.last_evacuation_report {
                let raw_name =
                    Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_evacuation_lifelines.ifcx");
                let layered = self
                    .universal_canvas
                    .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
                let composed =
                    crate::core::IfcxStage::compose_with_evacuation_lifelines_overlay(
                        &layered, evac,
                    );
                let json = composed.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported Evacuation Lifelines IFCX ({fname}, {} nodes)",
                                composed.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            } else {
                self.spatial_workspace
                    .push_trace("No Evacuation Lifelines report available.", false);
            }
        }
        if let Some(ws_url) = hud_actions.connect_shm_ws.clone() {
            match crate::core::web_bridge::shm_connect_websocket(&ws_url) {
                Ok(msg) => {
                    self.universal_canvas.shm_watchdog.config.ws_url = ws_url;
                    self.universal_canvas.shm_watchdog.ws_connected = true;
                    self.universal_canvas.shm_watchdog.ws_status = msg.clone();
                    self.universal_canvas.shm_watchdog.alert_level =
                        crate::core::shm_watchdog::WatchdogAlertLevel::Monitoring;
                    self.universal_canvas.shm_watchdog.push_event(
                        crate::core::shm_watchdog::WatchdogAlertLevel::Monitoring,
                        msg.clone(),
                        None,
                        false,
                    );
                    self.spatial_workspace.push_trace(msg, true);
                }
                Err(err) => {
                    self.universal_canvas.shm_watchdog.ws_status = err.clone();
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        }
        if hud_actions.disconnect_shm_ws {
            crate::core::web_bridge::shm_disconnect_websocket();
            self.universal_canvas
                .shm_watchdog
                .start_simulator(crate::core::shm_watchdog::WatchdogSimMode::Off);
            self.universal_canvas.shm_watchdog.ws_connected = false;
            self.universal_canvas.shm_watchdog.ws_status =
                "WebSocket & simulator disconnected.".to_string();
            self.spatial_workspace
                .push_trace("Disconnected live SHM telemetry stream.", true);
        }
        if hud_actions.export_watchdog_ifcx {
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_shm_watchdog.ifcx");
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let wd_overlay = self
                .universal_canvas
                .shm_watchdog
                .build_ifcx_overlay(&self.build_view.bim_bus.project);
            let composed = crate::core::IfcxStage::compose_with_shm_watchdog_overlay(
                &layered,
                &wd_overlay,
            );
            let json = composed.to_json_pretty();
            match crate::core::web_bridge::save_text_file(
                &fname,
                &json,
                "application/json;charset=utf-8",
            ) {
                Ok(_) => {
                    self.spatial_workspace.push_trace(
                        format!(
                            "Exported SHM Watchdog IFCX ({fname}, {} nodes)",
                            composed.data.len()
                        ),
                        true,
                    );
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if let Some(n_samples) = hud_actions.run_gpu_monte_carlo {
            self.universal_canvas.monte_carlo_config.sample_count = n_samples.clamp(256, 16_384);
            let rep = self
                .universal_canvas
                .run_monte_carlo_fragility_assessment(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary, true);
        }
        if hud_actions.export_monte_carlo_ifcx {
            if self.universal_canvas.last_monte_carlo_report.is_none() {
                self.universal_canvas
                    .run_monte_carlo_fragility_assessment(&self.build_view.bim_bus.project);
            }
            if let Some(ref mc_rep) = self.universal_canvas.last_monte_carlo_report {
                let raw_name =
                    Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                let fname = format!("{raw_name}_gpu_monte_carlo.ifcx");
                let layered = self
                    .universal_canvas
                    .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
                let mc_overlay = mc_rep.build_ifcx_overlay(&self.build_view.bim_bus.project);
                let composed = crate::core::IfcxStage::compose_with_monte_carlo_loss_overlay(
                    &layered,
                    &mc_overlay,
                );
                let json = composed.to_json_pretty();
                match crate::core::web_bridge::save_text_file(
                    &fname,
                    &json,
                    "application/json;charset=utf-8",
                ) {
                    Ok(_) => {
                        self.spatial_workspace.push_trace(
                            format!(
                                "Exported GPU Monte Carlo IFCX ({fname}, {} nodes)",
                                composed.data.len()
                            ),
                            true,
                        );
                    }
                    Err(err) => self.spatial_workspace.push_trace(err, false),
                }
            }
        }
        if let Some(lbl) = hud_actions.create_checkpoint_label {
            let msg = self.universal_canvas.create_signed_ifcx_checkpoint(
                &self.build_view.bim_bus.project,
                &self.members,
                &lbl,
            );
            self.spatial_workspace.push_trace(msg, true);
        }
        if hud_actions.approve_checkpoint {
            match self
                .universal_canvas
                .collab_state
                .approve_active_checkpoint("Principal Structural Reviewer")
            {
                Ok(msg) => self.spatial_workspace.push_trace(msg, true),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_ifcx_three_way_merge {
            let msg = self
                .universal_canvas
                .run_peer_three_way_ifcx_merge(&self.build_view.bim_bus.project, &self.members);
            self.spatial_workspace.push_trace(msg, true);
        }
        if hud_actions.refresh_district_risk_field {
            let rep = self
                .universal_canvas
                .refresh_district_risk_field(&self.build_view.bim_bus.project, &self.members);
            self.spatial_workspace.push_trace(rep.summary, true);
        }
        if hud_actions.export_collab_bundle_ifcx {
            if self.universal_canvas.collab_state.checkpoints.is_empty() {
                let _ = self.universal_canvas.create_signed_ifcx_checkpoint(
                    &self.build_view.bim_bus.project,
                    &self.members,
                    "Handover Baseline",
                );
            }
            match self
                .universal_canvas
                .collab_state
                .export_bundle_json(&self.build_view.bim_bus.project.project_name)
            {
                Ok(json) => {
                    let raw_name =
                        Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
                    let fname = format!("{raw_name}_collab_checkpoints.ifcx-bundle.json");
                    match crate::core::web_bridge::save_text_file(
                        &fname,
                        &json,
                        "application/json;charset=utf-8",
                    ) {
                        Ok(_) => {
                            self.spatial_workspace.push_trace(
                                format!(
                                    "Exported signed IFCX Collab Bundle ({fname}, {} checkpoints)",
                                    self.universal_canvas.collab_state.checkpoints.len()
                                ),
                                true,
                            );
                        }
                        Err(err) => self.spatial_workspace.push_trace(err, false),
                    }
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if let Some(obj) = hud_actions.run_trade_study {
            self.universal_canvas.trade_study_config.objective = obj;
            let rep = self
                .universal_canvas
                .run_trade_study_analysis(&self.build_view.bim_bus.project, &self.members);
            self.spatial_workspace.push_trace(rep.summary, true);
        }
        if let Some(arch) = hud_actions.adopt_trade_study_scenario {
            let adopt_msg = crate::core::trade_study::apply_trade_study_scenario_to_bim(
                &mut self.build_view.bim_bus.project,
                arch,
            );
            self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
            self.sync_bim_to_fem();
            let cp_msg = self.universal_canvas.create_signed_ifcx_checkpoint(
                &self.build_view.bim_bus.project,
                &self.members,
                &format!("Adopted {}", arch.short_label()),
            );
            let _ = self
                .universal_canvas
                .run_trade_study_analysis(&self.build_view.bim_bus.project, &self.members);
            let _ = self
                .universal_canvas
                .refresh_permit_compliance(&self.build_view.bim_bus.project, &self.members);
            self.spatial_workspace
                .push_trace(format!("{adopt_msg} • {cp_msg}"), true);
        }
        if hud_actions.export_trade_study_ifcx {
            let ts = self
                .universal_canvas
                .run_trade_study_analysis(&self.build_view.bim_bus.project, &self.members);
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_trade_study_matrix.ifcx");
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let ts_overlay = ts.build_ifcx_overlay(&self.build_view.bim_bus.project);
            let composed =
                crate::core::IfcxStage::compose_with_trade_study_overlay(&layered, &ts_overlay);
            let json = composed.to_json_pretty();
            match crate::core::web_bridge::save_text_file(
                &fname,
                &json,
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Trade-Study IFCX ({fname}, {} nodes)", composed.data.len()),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_permit_compliance {
            let rep = self
                .universal_canvas
                .refresh_permit_compliance(&self.build_view.bim_bus.project, &self.members);
            self.spatial_workspace.push_trace(rep.summary, rep.fail_count == 0);
        }
        if hud_actions.export_permit_annex_svg {
            let permit = self
                .universal_canvas
                .refresh_permit_compliance(&self.build_view.bim_bus.project, &self.members);
            let ts = self
                .universal_canvas
                .run_trade_study_analysis(&self.build_view.bim_bus.project, &self.members);
            let annex = crate::core::build_permit_compliance_annex_svg(
                &self.build_view.bim_bus.project,
                &permit,
                Some(&ts),
                Some(&self.universal_canvas.collab_state),
                "S-102",
                "REV 01",
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_permit_compliance_annex_S102.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.open_collab_bundle_picker {
            crate::core::web_bridge::trigger_file_picker(".json");
            self.spatial_workspace
                .push_trace("Select a signed .ifcx-bundle.json handover package...", true);
        }
        if let Some(url) = hud_actions.connect_collab_relay {
            match crate::core::web_bridge::collab_relay_connect(&url) {
                Ok(msg) => {
                    self.universal_canvas.collab_state.relay_connected = true;
                    self.universal_canvas.collab_state.relay_status = msg.clone();
                    self.spatial_workspace.push_trace(msg, true);
                }
                Err(err) => {
                    self.universal_canvas.collab_state.relay_connected = false;
                    self.universal_canvas.collab_state.relay_status = err.clone();
                    self.spatial_workspace.push_trace(err, false);
                }
            }
        }
        if hud_actions.disconnect_collab_relay {
            crate::core::web_bridge::collab_relay_disconnect();
            self.universal_canvas.collab_state.relay_connected = false;
            self.universal_canvas.collab_state.relay_status = "Disconnected".to_string();
            self.spatial_workspace
                .push_trace("Disconnected Collaborative IFCX Relay.", true);
        }
        if hud_actions.sync_peer_relay_delta {
            let msg = self
                .universal_canvas
                .simulate_peer_relay_sync(&self.build_view.bim_bus.project, &self.members);
            self.spatial_workspace.push_trace(msg, true);
        }
        if let Some(stock_m) = hud_actions.run_field_fabrication {
            self.universal_canvas.field_fab_config.stock_length_m = stock_m;
            let rep = self
                .universal_canvas
                .run_field_fabrication_audit(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary, true);
        }
        if hud_actions.toggle_field_qaqc_3d {
            self.universal_canvas.field_qaqc_overlay_visible =
                !self.universal_canvas.field_qaqc_overlay_visible;
            if self.universal_canvas.field_qaqc_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_field_fabrication_audit(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!(
                        "3D QA/QC Overlay ON ({} waypoints, {} pour strips)",
                        rep.waypoints.len(),
                        rep.pour_strips.len()
                    ),
                    true,
                );
            } else {
                self.spatial_workspace.push_trace("3D QA/QC Overlay OFF", true);
            }
        }
        if let Some(wp_idx) = hud_actions.fly_to_waypoint_idx {
            self.universal_canvas.field_qaqc_overlay_visible = true;
            let rep = self
                .universal_canvas
                .run_field_fabrication_audit(&self.build_view.bim_bus.project);
            if let Some(wp) = rep.waypoints.get(wp_idx) {
                self.universal_canvas.active_waypoint_idx = wp_idx;
                let heading_deg = self
                    .universal_canvas
                    .active_world_project_rel_path
                    .as_deref()
                    .and_then(|r| self.universal_canvas.world_index.get(r))
                    .map(|b| b.heading_deg)
                    .unwrap_or(0.0);
                let rad = heading_deg.to_radians();
                let ex = wp.local_x_m * rad.cos() - wp.local_y_m * rad.sin();
                let ny = wp.local_x_m * rad.sin() + wp.local_y_m * rad.cos();
                let origin = if self.universal_canvas.ground_origin_anchored {
                    self.universal_canvas.ground_origin_lat_lon
                } else {
                    self.universal_canvas.world_map.center_lat_lon
                };
                let (w_lat, w_lon) = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                    origin,
                    [ex * 1000.0, ny * 1000.0],
                );
                self.spatial_workspace.active_canvas_mode =
                    crate::views::spatial_workspace::SpatialCanvasMode::EarthGlobe;
                self.universal_canvas.globe_mode = true;
                self.universal_canvas.earth_world.start_fly_to(
                    &self.universal_canvas.world_map,
                    w_lat,
                    w_lon,
                    18.2,
                    &wp.code,
                );
                self.spatial_workspace.push_trace(
                    format!("Flew Camera -> {} @ {}", wp.code, wp.element_ref),
                    true,
                );
            }
        }
        if let Some(ref target_code) = hud_actions.clear_waypoint_hold {
            let code_opt = (!target_code.trim().is_empty()).then_some(target_code.as_str());
            let msg = crate::core::clear_next_or_matching_hold_point(
                &mut self.universal_canvas.field_qaqc_ledger,
                code_opt,
            );
            let _ = self
                .universal_canvas
                .run_field_fabrication_audit(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(msg, true);
        }
        if hud_actions.log_field_ncr_quick {
            let ncr_id = crate::core::log_quick_field_ncr(
                &mut self.universal_canvas.field_qaqc_ledger,
                "Active BIM Structural Bay",
                "Field inspection NCR logged from 3D Earth Spatial HUD.",
            );
            self.universal_canvas.field_qaqc_overlay_visible = true;
            let _ = self
                .universal_canvas
                .run_field_fabrication_audit(&self.build_view.bim_bus.project);
            self.spatial_workspace
                .push_trace(format!("Logged {ncr_id} on active site bay"), true);
        }
        if hud_actions.export_shop_drawing_svg {
            let rep = self
                .universal_canvas
                .run_field_fabrication_audit(&self.build_view.bim_bus.project);
            let shop = crate::core::build_fabrication_bbs_shop_svg(
                &self.build_view.bim_bus.project,
                &rep,
                Some(&self.universal_canvas.collab_state),
                "S-103",
                "REV 01",
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_rebar_bbs_shop_S103.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &shop.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(shop.sha256_seal);
                    self.spatial_workspace.push_trace(shop.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_bbs_csv {
            let rep = self
                .universal_canvas
                .run_field_fabrication_audit(&self.build_view.bim_bus.project);
            let csv = crate::core::CuttingStockOptimizer::export_bbs_csv(&rep.bbs_items);
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_rebar_bbs_schedule.csv");
            match crate::core::web_bridge::save_text_file(&fname, &csv, "text/csv;charset=utf-8") {
                Ok(_) => self
                    .spatial_workspace
                    .push_trace(format!("Exported {} BBS marks -> {fname}", rep.bbs_items.len()), true),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_cutlist_csv {
            let rep = self
                .universal_canvas
                .run_field_fabrication_audit(&self.build_view.bim_bus.project);
            let csv = crate::core::CuttingStockOptimizer::export_cutting_plan_csv(&rep.cso_report);
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_1d_rebar_cutlist.csv");
            match crate::core::web_bridge::save_text_file(&fname, &csv, "text/csv;charset=utf-8") {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!(
                        "Exported {} master stock bars -> {fname}",
                        rep.cso_report.master_bars_used
                    ),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_field_qaqc_ifcx {
            let rep = self
                .universal_canvas
                .run_field_fabrication_audit(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_field_qaqc_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_field_qaqc_bbs.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Field QA/QC & Rebar BBS IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if let Some(eid) = hud_actions.eigenflow_select_element {
            self.universal_canvas.eigenflow_config.disrupted_element_id = Some(eid);
            let rep = self
                .universal_canvas
                .run_eigenflow_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if let Some(lam) = hud_actions.eigenflow_set_lambda {
            self.universal_canvas
                .eigenflow_config
                .stiffness_retention_lambda = lam.clamp(0.0, 0.90);
            let rep = self
                .universal_canvas
                .run_eigenflow_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if let Some(pat) = hud_actions.eigenflow_set_pattern {
            self.universal_canvas.eigenflow_config.load_pattern = pat;
            let rep = self
                .universal_canvas
                .run_eigenflow_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.run_eigenflow_studio {
            let rep = self
                .universal_canvas
                .run_eigenflow_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_eigenflow_3d {
            self.universal_canvas.eigenflow_overlay_visible =
                !self.universal_canvas.eigenflow_overlay_visible;
            if self.universal_canvas.eigenflow_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_eigenflow_studio(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!(
                        "3D EigenFlow Overlay ON (P1 #{}, {} P2 conduits)",
                        rep.woodbury.disrupted_element_id,
                        rep.woodbury.redistributed_p2.len()
                    ),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D EigenFlow Overlay OFF", true);
            }
        }
        if hud_actions.export_eigenflow_ifcx {
            let rep = self
                .universal_canvas
                .run_eigenflow_studio(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_eigenflow_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_eigenflow_nonlinear.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported EigenFlow Non-Linear IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_eigenflow_svg {
            let rep = self
                .universal_canvas
                .run_eigenflow_studio(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_eigenflow_nonlinear_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-104",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S104_eigenflow.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_extreme_events {
            let rep = self
                .universal_canvas
                .run_extreme_events_audit(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_extreme_events_3d {
            self.universal_canvas.extreme_events_overlay_visible =
                !self.universal_canvas.extreme_events_overlay_visible;
            if self.universal_canvas.extreme_events_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_extreme_events_audit(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!("3D Extreme Events Overlay ON — {}", rep.summary_verdict),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D Extreme Events Overlay OFF", true);
            }
        }
        if let Some((eid, lam)) = hud_actions.extreme_send_to_eigenflow {
            self.universal_canvas.eigenflow_config.disrupted_element_id = Some(eid);
            self.universal_canvas
                .eigenflow_config
                .stiffness_retention_lambda = lam.clamp(0.0, 0.90);
            self.universal_canvas.eigenflow_overlay_visible = true;
            let rep = self
                .universal_canvas
                .run_eigenflow_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.open_eigenflow_drawer(0);
            self.spatial_workspace.push_trace(
                format!("Handoff P1 #{eid} (lambda={lam:.2}) -> {}", rep.summary_verdict),
                true,
            );
        }
        if hud_actions.export_extreme_events_ifcx {
            let rep = self
                .universal_canvas
                .run_extreme_events_audit(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_extreme_events_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_extreme_events.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Extreme Events Blast & Fire IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_extreme_events_svg {
            let rep = self
                .universal_canvas
                .run_extreme_events_audit(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_extreme_events_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-105",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S105_extreme_events.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_isolation_studio {
            let rep = self
                .universal_canvas
                .run_isolation_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_isolation_3d {
            self.universal_canvas.isolation_overlay_visible =
                !self.universal_canvas.isolation_overlay_visible;
            if self.universal_canvas.isolation_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_isolation_studio(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!("3D Base Isolation ON — {}", rep.summary_verdict),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D Base Isolation OFF", true);
            }
        }
        if hud_actions.export_isolation_ifcx {
            let rep = self
                .universal_canvas
                .run_isolation_studio(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_isolation_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_seismic_isolation.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Seismic Isolation IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_isolation_svg {
            let rep = self
                .universal_canvas
                .run_isolation_studio(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_seismic_isolation_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-106",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S106_isolation.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_wind_studio {
            let rep = self
                .universal_canvas
                .run_wind_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_wind_3d {
            self.universal_canvas.wind_overlay_visible =
                !self.universal_canvas.wind_overlay_visible;
            if self.universal_canvas.wind_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_wind_studio(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!("3D Wind Facade & Streamlines ON — {}", rep.summary_verdict),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D Wind Facade & Streamlines OFF", true);
            }
        }
        if hud_actions.export_wind_ifcx {
            let rep = self
                .universal_canvas
                .run_wind_studio(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_wind_studio_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_wind_aeroelasticity.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Wind Aeroelasticity IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_wind_svg {
            let rep = self
                .universal_canvas
                .run_wind_studio(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_wind_aero_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-107",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S107_wind.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_shear_wall_studio {
            let rep = self
                .universal_canvas
                .run_shear_wall_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_shear_wall_3d {
            self.universal_canvas.shear_wall_overlay_visible =
                !self.universal_canvas.shear_wall_overlay_visible;
            if self.universal_canvas.shear_wall_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_shear_wall_studio(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!("3D Shear Wall SBE & X-Cage ON — {}", rep.summary_verdict),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D Shear Wall SBE & X-Cage OFF", true);
            }
        }
        if hud_actions.export_shear_wall_ifcx {
            let rep = self
                .universal_canvas
                .run_shear_wall_studio(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_shear_wall_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_shear_wall_sbe.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Shear Wall & SBE IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_shear_wall_svg {
            let rep = self
                .universal_canvas
                .run_shear_wall_studio(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_shear_wall_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-108",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S108_shear_wall.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_connection_studio {
            let rep = self
                .universal_canvas
                .run_connection_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_connection_3d {
            self.universal_canvas.connection_overlay_visible =
                !self.universal_canvas.connection_overlay_visible;
            if self.universal_canvas.connection_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_connection_studio(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!("3D Steel Connections ON — {}", rep.summary_verdict),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D Steel Connections OFF", true);
            }
        }
        if hud_actions.export_connection_ifcx {
            let rep = self
                .universal_canvas
                .run_connection_studio(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_connection_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_steel_connection.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Steel Connection IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_connection_svg {
            let rep = self
                .universal_canvas
                .run_connection_studio(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_steel_connection_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-109",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S109_connection.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_prestress_studio {
            let rep = self
                .universal_canvas
                .run_prestress_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_prestress_3d {
            self.universal_canvas.prestress_overlay_visible =
                !self.universal_canvas.prestress_overlay_visible;
            if self.universal_canvas.prestress_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_prestress_studio(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!("3D PT Tendons ON — {}", rep.summary_verdict),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D PT Tendons OFF", true);
            }
        }
        if hud_actions.export_prestress_ifcx {
            let rep = self
                .universal_canvas
                .run_prestress_studio(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_prestress_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_prestress_tendon.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Prestressed / PT IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_prestress_svg {
            let rep = self
                .universal_canvas
                .run_prestress_studio(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_prestress_tendon_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-110",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S110_prestress.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_composite_studio {
            let rep = self
                .universal_canvas
                .run_composite_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_composite_3d {
            self.universal_canvas.composite_overlay_visible =
                !self.universal_canvas.composite_overlay_visible;
            if self.universal_canvas.composite_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_composite_studio(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!("3D Composite Deck ON — {}", rep.summary_verdict),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D Composite Deck OFF", true);
            }
        }
        if hud_actions.export_composite_ifcx {
            let rep = self
                .universal_canvas
                .run_composite_studio(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_composite_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_composite_deck.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Composite Deck IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_composite_svg {
            let rep = self
                .universal_canvas
                .run_composite_studio(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_composite_deck_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-111",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S111_composite.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.run_reliability_studio {
            let rep = self
                .universal_canvas
                .run_reliability_studio(&self.build_view.bim_bus.project);
            self.spatial_workspace.push_trace(rep.summary_verdict, true);
        }
        if hud_actions.toggle_reliability_3d {
            self.universal_canvas.reliability_overlay_visible =
                !self.universal_canvas.reliability_overlay_visible;
            if self.universal_canvas.reliability_overlay_visible {
                let rep = self
                    .universal_canvas
                    .run_reliability_studio(&self.build_view.bim_bus.project);
                self.spatial_workspace.push_trace(
                    format!("3D Reliability Risk ON — {}", rep.summary_verdict),
                    true,
                );
            } else {
                self.spatial_workspace
                    .push_trace("3D Reliability Risk OFF", true);
            }
        }
        if hud_actions.export_reliability_ifcx {
            let rep = self
                .universal_canvas
                .run_reliability_studio(&self.build_view.bim_bus.project);
            let layered = self
                .universal_canvas
                .build_active_layered_ifcx(&self.build_view.bim_bus.project, &self.members);
            let doc = crate::core::IfcxStage::compose_with_reliability_overlay(
                &layered,
                &rep.to_ifcx_overlay(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_reliability_form.ifcx");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &doc.to_json_pretty(),
                "application/json;charset=utf-8",
            ) {
                Ok(_) => self.spatial_workspace.push_trace(
                    format!("Exported Reliability FORM/SORM IFCX ({fname})"),
                    true,
                ),
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
        if hud_actions.export_reliability_svg {
            let rep = self
                .universal_canvas
                .run_reliability_studio(&self.build_view.bim_bus.project);
            let cp_hash = self
                .universal_canvas
                .collab_state
                .active_checkpoint()
                .map(|c| c.short_sha256());
            let annex = crate::core::build_reliability_form_annex_svg(
                &self.build_view.bim_bus.project.project_name,
                &rep,
                "S-112",
                cp_hash.as_deref(),
            );
            let raw_name =
                Self::safe_ifcx_project_stem(&self.build_view.bim_bus.project.project_name);
            let fname = format!("{raw_name}_sheet_S112_reliability.svg");
            match crate::core::web_bridge::save_text_file(
                &fname,
                &annex.svg_content,
                "image/svg+xml;charset=utf-8",
            ) {
                Ok(_) => {
                    self.universal_canvas.last_vector_dossier_seal = Some(annex.sha256_seal);
                    self.spatial_workspace.push_trace(annex.summary, true);
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
        }
    }

    fn safe_ifcx_project_stem(project_name: &str) -> String {
        let trimmed = project_name.trim();
        if trimmed.is_empty() {
            return "EigenGraph_BIM".to_string();
        }
        trimmed
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn open_vault_project_entry(&mut self, entry: crate::core::vault::PersonalVaultProjectEntry) {
        if crate::core::web_bridge::vault_is_file_lazy(&entry.rel_path) {
            self.pending_lazy_vault_entry = Some(entry.clone());
            if crate::core::web_bridge::vault_request_file_async(&entry.rel_path, "vault_open") {
                let msg = format!(
                    "Streaming '{}' ({:.1} MB) from Vault...",
                    entry.filename,
                    entry.size_bytes.max(1) as f64 / 1_048_576.0
                );
                self.spatial_workspace.vault_feedback = Some(msg.clone());
                self.spatial_workspace.push_trace(msg, true);
                return;
            }
        }
        match crate::core::InternalVault::load_personal_vault_raw(&entry) {
            Ok(raw) => self.open_vault_raw_payload(&entry, &raw),
            Err(err) => {
                self.my_project_view.export_success_msg =
                    Some(format!("Failed to read Vault file: {}", err));
                self.spatial_workspace.vault_feedback =
                    Some(format!("Failed to read Vault file: {}", err));
            }
        }
    }

    fn open_vault_raw_payload(
        &mut self,
        entry: &crate::core::vault::PersonalVaultProjectEntry,
        raw: &str,
    ) {
        let clean_raw = raw.trim_start_matches('\u{FEFF}');
        let is_ifcx = entry.filename.to_ascii_lowercase().ends_with(".ifcx")
            || crate::core::ifcx::IfcxDocument::looks_like_ifcx(clean_raw);
        if is_ifcx {
            self.import_ifcx_raw_onto_earth(&entry.name, &entry.filename, &entry.rel_path, clean_raw);
            return;
        }
        if self.ingest_geojson_or_survey_onto_earth(&entry.filename, clean_raw) {
            return;
        }
        if let Ok(ws_cfg) = self.universal_canvas.apply_unified_archive(
            clean_raw,
            &mut self.build_view.bim_bus.project,
            &mut self.members,
            &mut self.parametric_view.variables,
        ) {
            self.settings_view.apply_workspace_settings(
                &ws_cfg,
                &mut self.theme,
                &mut self.ai_assistant_view.is_visible,
            );
            self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
            self.universal_canvas.active_world_project_rel_path = Some(entry.rel_path.clone());
            if let Some(pinned) = self.universal_canvas.world_index.get(&entry.rel_path).cloned() {
                if pinned.source == crate::core::gis::GeoPlacementSource::UserPinnedOnEarth
                    || (self.universal_canvas.ground_origin_lat_lon.0.abs() < 1e-4
                        && self.universal_canvas.ground_origin_lat_lon.1.abs() < 1e-4)
                {
                    self.universal_canvas.ground_origin_lat_lon = (pinned.lat, pinned.lon);
                }
            }
            self.universal_canvas.ground_origin_anchored = true;
            self.universal_canvas.globe_mode = true;
            self.sync_bim_to_fem();
            self.sync_active_building_to_world_index(true);
            self.build_view
                .fit_view_to_project(&mut self.universal_canvas, &self.members);
            self.sync_earth_maps();
            self.sync_lifecycle_from_canvas();
            let loaded_msg = format!(
                "Loaded workspace '{}' ({} BIM elements, {} drawings, {} FEM members) from Vault.",
                entry.name,
                self.build_view.bim_bus.project.element_count(),
                self.universal_canvas.entities.len(),
                self.members.len(),
            );
            self.my_project_view.export_success_msg = Some(loaded_msg.clone());
            self.active_pillar = NavigationPillar::GeometryAssembly;
            if self.spatial_workspace.spatial_mode_enabled {
                self.spatial_workspace.active_canvas_mode =
                    crate::views::spatial_workspace::SpatialCanvasMode::EarthGlobe;
                self.spatial_workspace.active_overlay_pillar = None;
                self.spatial_workspace.push_trace(loaded_msg, true);
            }
        } else if let Ok(model) =
            crate::core::EigenProjectArchive::deserialize_from_eigen_json(clean_raw)
                .or_else(|_| serde_json::from_str::<crate::core::StructuralModel>(clean_raw))
        {
            self.members = model
                .elements
                .iter()
                .map(|el| StructuralMember {
                    id: el.id,
                    name: el.name.clone(),
                    section: el.section.name.clone(),
                    ..Default::default()
                })
                .collect();
            let legacy_msg = format!(
                "Loaded legacy structural model '{}' ({} nodes, {} elements) from personal Vault.",
                model.name,
                model.nodes.len(),
                model.elements.len()
            );
            self.my_project_view.export_success_msg = Some(legacy_msg.clone());
            self.active_pillar = NavigationPillar::GeometryAssembly;
            if self.spatial_workspace.spatial_mode_enabled {
                self.spatial_workspace.active_overlay_pillar = None;
                self.spatial_workspace.push_trace(legacy_msg, true);
            }
        } else {
            let msg = format!("Failed to parse Vault project '{}'.", entry.filename);
            self.my_project_view.export_success_msg = Some(msg.clone());
            self.spatial_workspace.vault_feedback = Some(msg);
        }
    }

    fn sync_active_building_to_world_index(&mut self, save_manifest: bool) {
        let Some(ref rel) = self.universal_canvas.active_world_project_rel_path else {
            return;
        };
        let fname = rel.split('/').next_back().unwrap_or(rel);
        let existing = self.universal_canvas.world_index.get(rel).cloned();
        let (lat, lon) = self.universal_canvas.ground_origin_lat_lon;
        let heading = existing.as_ref().map(|b| b.heading_deg).unwrap_or(0.0);
        let source = existing
            .as_ref()
            .map(|b| b.source)
            .unwrap_or_else(|| {
                if lat.abs() > 1e-4 || lon.abs() > 1e-4 {
                    crate::core::gis::GeoPlacementSource::UserPinnedOnEarth
                } else {
                    crate::core::gis::GeoPlacementSource::StagingCampusAuto
                }
            });
        let hint = existing.map(|b| b.address_hint).unwrap_or_default();
        let mut proxy = crate::core::gis::WorldBuildingProxy::from_bim_project(
            rel,
            fname,
            lat,
            lon,
            heading,
            &self.build_view.bim_bus.project,
            &self.members,
            source,
            hint,
        );
        proxy.elev_m = self.universal_canvas.earth_world.effective_elevation_m(lat, lon) as f64;
        self.universal_canvas.world_index.upsert_building(proxy, true);
        self.universal_canvas
            .refresh_neighborhood_coupling(&self.build_view.bim_bus.project);
        if save_manifest {
            crate::core::InternalVault::save_world_spatial_index(&self.universal_canvas.world_index);
        }
    }

    fn ingest_geojson_or_survey_onto_earth(&mut self, filename: &str, clean_raw: &str) -> bool {
        let lower = filename.to_ascii_lowercase();
        let trimmed = clean_raw.trim_start();
        let looks_geojson = lower.ends_with(".geojson")
            || (lower.ends_with(".json")
                && (trimmed.contains("\"FeatureCollection\"") || trimmed.contains("\"coordinates\"")));
        if looks_geojson {
            if let Ok((cnt, centroid_opt)) = self.universal_canvas.urban_vectors.ingest_geojson(clean_raw, filename) {
                self.universal_canvas.globe_mode = true;
                self.universal_canvas.world_map.vector_site_overlay = true;
                if let Some((lat, lon)) = centroid_opt {
                    self.universal_canvas.earth_world.start_fly_to(
                        &self.universal_canvas.world_map,
                        lat,
                        lon,
                        16.5,
                        filename,
                    );
                    if !self.universal_canvas.ground_origin_anchored {
                        self.universal_canvas.ground_origin_lat_lon = (lat, lon);
                    }
                }
                let msg = format!("Loaded {cnt} crisp GeoJSON vector feature(s) from '{filename}' onto Earth 3D.");
                self.universal_canvas.status_message = Some(msg.clone());
                self.spatial_workspace.vault_feedback = Some(msg.clone());
                self.spatial_workspace.push_trace(msg, true);
                return true;
            }
        }
        if lower.ends_with(".xyz") || lower.ends_with(".pts") {
            if let Ok(tin) = crate::core::gis::parse_survey_xyz_to_tin(clean_raw) {
                let npts = tin.points.len();
                self.universal_canvas.active_site_tin = Some(tin);
                self.universal_canvas.site_tin_from_ifc = false;
                self.universal_canvas.show_terrain_contours = true;
                self.universal_canvas.globe_mode = true;
                let msg = format!("Loaded {npts} survey terrain point(s) from '{filename}' into Earth Vector TIN.");
                self.universal_canvas.status_message = Some(msg.clone());
                self.spatial_workspace.vault_feedback = Some(msg.clone());
                self.spatial_workspace.push_trace(msg, true);
                return true;
            }
        }
        false
    }

    fn import_ifcx_raw_onto_earth(
        &mut self,
        display_label: &str,
        filename: &str,
        rel_path: &str,
        clean_raw: &str,
    ) {
        match crate::core::ifcx::IfcxDocument::parse_to_bim_project_with_geo(
            clean_raw, filename, rel_path,
        ) {
            Ok((proj, geo_opt, resolved_src, addr_hint, summary_msg)) => {
                self.build_view.bim_bus = crate::core::BimCommandBus::from_project(proj.clone());
                self.parametric_view.bim_bus = crate::core::BimCommandBus::from_project(proj);
                self.parametric_view.active_level_idx = 0;

                let existing_opt = self.universal_canvas.world_index.get(rel_path).cloned();
                let (target_geo, final_src, final_hdg) = if let Some(ref ex) = existing_opt {
                    if ex.source == crate::core::gis::GeoPlacementSource::UserPinnedOnEarth {
                        ((ex.lat, ex.lon), ex.source, ex.heading_deg)
                    } else if let Some(geo) = geo_opt {
                        (geo, resolved_src, ex.heading_deg)
                    } else if ex.source == crate::core::gis::GeoPlacementSource::StagingCampusAuto {
                        ((ex.lat, ex.lon), ex.source, ex.heading_deg)
                    } else {
                        let slot = self.universal_canvas.world_index.unlocated_count();
                        (
                            self.universal_canvas.world_index.staging_slot_lat_lon(slot),
                            crate::core::gis::GeoPlacementSource::StagingCampusAuto,
                            0.0,
                        )
                    }
                } else if let Some(geo) = geo_opt {
                    (geo, resolved_src, 0.0)
                } else {
                    let slot = self.universal_canvas.world_index.unlocated_count();
                    (
                        self.universal_canvas.world_index.staging_slot_lat_lon(slot),
                        crate::core::gis::GeoPlacementSource::StagingCampusAuto,
                        0.0,
                    )
                };

                self.universal_canvas.ground_origin_lat_lon = target_geo;
                self.universal_canvas.ground_origin_anchored = true;
                self.universal_canvas.globe_mode = true;
                if let Some(ifcx_tin) =
                    crate::core::ifcx::IfcxDocument::extract_site_terrain_tin_from_raw(clean_raw)
                {
                    self.universal_canvas.active_site_tin = Some(ifcx_tin);
                    self.universal_canvas.site_tin_from_ifc = true;
                    self.universal_canvas.show_terrain_contours = true;
                }
                if let Ok(doc) = serde_json::from_str::<crate::core::ifcx::IfcxDocument>(clean_raw) {
                    let restored_city = crate::core::gis::IfcxCityScene::from_ifcx_document(&doc);
                    if !restored_city.is_empty() {
                        self.universal_canvas.city_scene = restored_city;
                    }
                }
                self.universal_canvas.active_world_project_rel_path = Some(rel_path.to_string());
                self.universal_canvas.earth_world.camera.tilt_deg = 42.0;
                self.universal_canvas.earth_world.start_fly_to(
                    &self.universal_canvas.world_map,
                    target_geo.0,
                    target_geo.1,
                    18.0,
                    display_label,
                );
                self.sync_bim_to_fem();

                let mut proxy = crate::core::gis::WorldBuildingProxy::from_bim_project(
                    rel_path,
                    filename,
                    target_geo.0,
                    target_geo.1,
                    final_hdg,
                    &self.build_view.bim_bus.project,
                    &self.members,
                    final_src,
                    addr_hint.clone(),
                );
                proxy.elev_m = self
                    .universal_canvas
                    .earth_world
                    .effective_elevation_m(target_geo.0, target_geo.1) as f64;
                self.universal_canvas.world_index.upsert_building(proxy, true);
                self.universal_canvas
                    .refresh_neighborhood_coupling(&self.build_view.bim_bus.project);
                crate::core::InternalVault::save_world_spatial_index(&self.universal_canvas.world_index);

                self.build_view
                    .fit_view_to_project(&mut self.universal_canvas, &self.members);
                self.sync_earth_maps();
                let geo_note = match final_src {
                    crate::core::gis::GeoPlacementSource::StagingCampusAuto => {
                        format!("{summary_msg} • Placed in Staging Campus (click 'Place' to pin real Earth site)")
                    }
                    crate::core::gis::GeoPlacementSource::AddressOrFolderHint if !addr_hint.is_empty() => {
                        format!("{summary_msg} • Auto-geolocated via city '{addr_hint}'")
                    }
                    _ => summary_msg,
                };
                self.my_project_view.export_success_msg = Some(geo_note.clone());
                self.build_view.modify_status_msg = Some(geo_note.clone());
                self.active_pillar = NavigationPillar::GeometryAssembly;
                if self.spatial_workspace.spatial_mode_enabled {
                    self.spatial_workspace.active_canvas_mode =
                        crate::views::spatial_workspace::SpatialCanvasMode::EarthGlobe;
                    self.spatial_workspace.active_overlay_pillar = None;
                    self.spatial_workspace.vault_feedback = Some(geo_note.clone());
                    self.spatial_workspace.push_trace(geo_note, true);
                }
            }
            Err(err) => {
                let msg = format!("Failed importing IFC '{}': {err}", filename);
                self.my_project_view.export_success_msg = Some(msg.clone());
                self.spatial_workspace.vault_feedback = Some(msg.clone());
                self.spatial_workspace.push_trace(msg, false);
            }
        }
    }

    fn render_classic_mode_shell(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        screen_rect: Rect,
        is_mobile: bool,
        is_compact: bool,
        workspace_snapshot: &crate::views::universal_canvas::WorkspaceCanvasSettings,
    ) {
        if !self.layout_initialized {
            self.sidebar_open = !is_compact;
            self.layout_initialized = true;
            self.last_compact_layout = is_compact;
            ctx.request_repaint();
        } else if is_compact != self.last_compact_layout {
            self.sidebar_open = !is_compact;
            self.last_compact_layout = is_compact;
            ctx.request_repaint();
        }

        let mut docked_sidebar_width = 0.0;
        if !is_compact && self.sidebar_open {
            let panel_resp = Panel::left("unified_sidebar")
                .resizable(true)
                .default_size(246.0)
                .size_range(200.0..=320.0)
                .frame(
                    Frame::new()
                        .fill(self.theme.bg_sidebar)
                        .stroke(Stroke::new(1.0, self.theme.border_subtle))
                        .inner_margin(Margin::symmetric(12, 10)),
                )
                .show(ui, |ui| {
                    self.render_sidebar_contents(ui, false, is_mobile);
                });
            docked_sidebar_width = panel_resp.response.rect.width();
        }

        let pointer_in_live_canvas = !self.universal_canvas.is_frozen
            && match self.active_pillar {
                NavigationPillar::ParametricDED => self
                    .parametric_view
                    .is_pointer_in_live_canvas(ctx, self.universal_canvas.is_frozen),
                NavigationPillar::GeometryAssembly => self
                    .build_view
                    .is_pointer_in_live_canvas(ctx, self.universal_canvas.is_frozen),
                _ => self.universal_canvas.is_pointer_in_live_canvas(ctx),
            };
        let scroll_source = if pointer_in_live_canvas {
            egui::scroll_area::ScrollSource::SCROLL_BAR
        } else {
            egui::scroll_area::ScrollSource::default()
        };
        let outer_margin = if is_mobile {
            Margin::symmetric(8, 8)
        } else {
            Margin::symmetric(14, 12)
        };
        let sidebar_inset = if !self.sidebar_open { 44.0 } else { 0.0 };
        let uses_native_single_canvas = matches!(
            self.active_pillar,
            NavigationPillar::ParametricDED | NavigationPillar::GeometryAssembly
        );

        CentralPanel::default()
            .frame(Frame::new().fill(self.theme.bg_canvas))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("workspace_main_vertical_scroll")
                    .auto_shrink([false, false])
                    .scroll_source(scroll_source)
                    .show(ui, |ui| {
                        Frame::new().inner_margin(outer_margin).show(ui, |ui| {
                            let content_w = ui.available_width().max(160.0);
                            ui.set_width(content_w);
                            ui.set_max_width(content_w);
                            if !uses_native_single_canvas {
                                self.universal_canvas.show(
                                    ui,
                                    &self.theme,
                                    self.active_pillar,
                                    &mut self.build_view.bim_bus.project,
                                    &mut self.members,
                                    &mut self.parametric_view.variables,
                                    workspace_snapshot.clone(),
                                    sidebar_inset,
                                );

                                ui.add_space(10.0);

                                let (_anchor_rect, anchor_resp) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width().max(1.0), 1.0),
                                    egui::Sense::hover(),
                                );
                                if std::mem::take(
                                    &mut self.universal_canvas.request_scroll_to_workbench,
                                ) {
                                    anchor_resp.scroll_to_me(Some(Align::TOP));
                                }

                                ui.add_space(4.0);
                            }

                            let target_pillar = self.active_pillar;
                            self.render_pillar_module_body(
                                ui,
                                target_pillar,
                                workspace_snapshot,
                                sidebar_inset,
                            );
                            self.apply_pillar_post_render_actions();
                        });
                    });
            });

        if let Some((tab, member_id, note)) = self.build_view.requested_analysis_jump.take() {
            self.analyze_view.active_subtab = tab;
            if let Some(id) = member_id {
                self.analyze_view.woodbury_disrupt_elem_id = id;
            }
            self.analyze_view.linked_from_geometry_note = Some(note);
            self.active_pillar = NavigationPillar::AnalysisSimulation;
        }
        if let Some((member_id, render_mode, color_mode)) =
            self.analyze_view.requested_geometry_jump.take()
        {
            if let Some(id) = member_id {
                self.build_view.selected_member_id = Some(id);
            }
            if let Some(mode) = render_mode {
                self.build_view.render_mode = mode;
            }
            if let Some(cm) = color_mode {
                self.build_view.color_mode = cm;
            }
            self.build_view.analysis_panel_open = true;
            self.active_pillar = NavigationPillar::GeometryAssembly;
        }
        if let Some(subtab) = self.analyze_view.requested_drafting_jump.take() {
            self.parametric_view.active_drafting_subtab = subtab;
            self.parametric_view.drafting_submenu_open = true;
            self.active_pillar = NavigationPillar::ParametricDED;
        }

        if !self.sidebar_open {
            Area::new(Id::new("floating_sidebar_toggle"))
                .order(Order::Foreground)
                .fixed_pos(Pos2::new(screen_rect.left() + 10.0, screen_rect.top() + 10.0))
                .show(ctx, |ui| {
                    if icons::icon_only_button(
                        ui,
                        AppIcon::Menu,
                        self.theme.text_primary,
                        self.theme.bg_card.gamma_multiply(0.95),
                        Stroke::new(1.0, self.theme.border_card),
                        CornerRadius::same(8),
                        Vec2::new(34.0, 34.0),
                        "Open navigation menu",
                    )
                    .clicked()
                    {
                        self.sidebar_open = true;
                    }
                });
        }

        self.ai_assistant_view.show(
            ctx,
            &self.theme,
            self.active_pillar,
            &self.members,
            &self.projects,
            &mut self.db_components,
            docked_sidebar_width * 0.5,
        );

        if is_compact && self.sidebar_open {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.sidebar_open = false;
            } else {
                let drawer_w = (screen_rect.width() * 0.84).clamp(240.0, 300.0);
                let drawer_h = screen_rect.height().max(260.0);
                let backdrop_w = (screen_rect.width() - drawer_w).max(0.0);

                if backdrop_w > 1.0 {
                    Area::new(Id::new("mobile_sidebar_backdrop"))
                        .order(Order::Foreground)
                        .fixed_pos(Pos2::new(screen_rect.left() + drawer_w, screen_rect.top()))
                        .show(ctx, |ui| {
                            let (rect, resp) = ui.allocate_exact_size(
                                Vec2::new(backdrop_w, drawer_h),
                                egui::Sense::click(),
                            );
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::ZERO,
                                Color32::from_black_alpha(150),
                            );
                            if resp.clicked() {
                                self.sidebar_open = false;
                            }
                        });
                }

                Area::new(Id::new("mobile_nav_drawer"))
                    .order(Order::Foreground)
                    .fixed_pos(screen_rect.min)
                    .show(ctx, |ui| {
                        Frame::new()
                            .fill(self.theme.bg_sidebar)
                            .stroke(Stroke::new(1.0, self.theme.border_card))
                            .inner_margin(Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                let inner_w = (drawer_w - 24.0).max(180.0);
                                let inner_h = (drawer_h - 20.0).max(200.0);
                                ui.set_min_size(Vec2::new(inner_w, inner_h));
                                ui.set_max_size(Vec2::new(inner_w, inner_h));
                                self.render_sidebar_contents(ui, true, is_mobile);
                            });
                    });
            }
        }
    }

    fn render_pillar_module_body(
        &mut self,
        ui: &mut egui::Ui,
        target_pillar: NavigationPillar,
        workspace_snapshot: &crate::views::universal_canvas::WorkspaceCanvasSettings,
        sidebar_inset: f32,
    ) {
        match target_pillar {
            NavigationPillar::MyLibrary => {
                self.my_project_view.show(
                    ui,
                    &self.theme,
                    &self.projects,
                    &self.personal_vault_projects,
                    &mut self.active_pillar,
                    &mut self.feature_registry,
                );
                if std::mem::take(&mut self.my_project_view.request_open_settings_vault) {
                    self.settings_view.active_subtab =
                        crate::views::settings::SettingsSubTab::WorkspaceSettings;
                    self.show_settings = true;
                }
                if std::mem::take(&mut self.my_project_view.request_refresh_vault) {
                    self.personal_vault_projects =
                        crate::core::InternalVault::list_personal_vault_projects();
                }
                if std::mem::take(&mut self.my_project_view.request_save_active_to_vault) {
                    let raw_name = self.my_project_view.save_project_name_input.trim();
                    let proj_name: String = if raw_name.is_empty() {
                        "Workspace_Project".to_string()
                    } else {
                        raw_name.to_string()
                    };
                    self.build_view.bim_bus.project.project_name = proj_name.clone();
                    self.parametric_view.bim_bus.project.project_name = proj_name.clone();
                    self.sync_lifecycle_to_canvas();
                    let mut archive = self.universal_canvas.build_unified_archive(
                        &self.build_view.bim_bus.project,
                        &self.members,
                        &self.parametric_view.variables,
                        workspace_snapshot.clone(),
                    );
                    archive.project_name = proj_name.clone();
                    let safe_file: String = proj_name
                        .chars()
                        .map(|c| {
                            if c.is_alphanumeric() || c == '_' || c == '-' {
                                c
                            } else {
                                '_'
                            }
                        })
                        .collect();
                    let fname = format!("{}.eigen", safe_file);
                    match serde_json::to_string_pretty(&archive) {
                        Ok(json) => match crate::core::InternalVault::save_project_archive_json(&fname, &json) {
                            Ok(saved) => {
                                self.my_project_view.export_success_msg = Some(format!(
                                    "Saved complete workspace '{}' ({} BIM elems, {} drawings) to Vault ({})",
                                    proj_name,
                                    self.build_view.bim_bus.project.element_count(),
                                    self.universal_canvas.entities.len(),
                                    saved
                                ));
                                self.personal_vault_projects =
                                    crate::core::InternalVault::list_personal_vault_projects();
                            }
                            Err(err) => {
                                self.my_project_view.export_success_msg =
                                    Some(format!("Vault save failed: {}", err));
                            }
                        },
                        Err(err) => {
                            self.my_project_view.export_success_msg =
                                Some(format!("Archive serialization failed: {}", err));
                        }
                    }
                }
                if std::mem::take(&mut self.my_project_view.request_export_active_archive) {
                    self.sync_lifecycle_to_canvas();
                    self.universal_canvas.export_unified_file(
                        &self.build_view.bim_bus.project,
                        &self.members,
                        &self.parametric_view.variables,
                        workspace_snapshot.clone(),
                    );
                    self.my_project_view.export_success_msg =
                        self.universal_canvas.status_message.clone();
                }
            }
            NavigationPillar::AiAgent => {
                self.ai_firm_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &self.projects,
                    &mut self.db_components,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::ResearchConcept => {
                self.research_view.show(
                    ui,
                    &self.theme,
                    &mut self.active_pillar,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::ParametricDED => {
                self.parametric_view.show(
                    ui,
                    &self.theme,
                    &mut self.db_components,
                    &mut self.active_pillar,
                    &mut self.feature_registry,
                    &mut self.universal_canvas,
                    &self.members,
                    workspace_snapshot.clone(),
                    sidebar_inset,
                );
            }
            NavigationPillar::GeometryAssembly => {
                self.build_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.db_components,
                    &mut self.feature_registry,
                    &mut self.universal_canvas,
                    &self.parametric_view.variables,
                    workspace_snapshot.clone(),
                    sidebar_inset,
                );
            }
            NavigationPillar::AnalysisSimulation => {
                self.analyze_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::TimeScheduling => {
                if !matches!(
                    self.planning_view.active_subtab,
                    crate::views::planning::PlanningSubTab::Schedule4d
                        | crate::views::planning::PlanningSubTab::StagedErectionSvd
                        | crate::views::planning::PlanningSubTab::ScheduleRiskMonteCarlo
                        | crate::views::planning::PlanningSubTab::ResourceLeveling4d
                        | crate::views::planning::PlanningSubTab::SiteDailyDiaryFleet
                ) {
                    self.planning_view.active_subtab =
                        crate::views::planning::PlanningSubTab::Schedule4d;
                }
                self.planning_view.show(
                    ui,
                    &self.theme,
                    &self.parametric_view.variables,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                    &mut self.active_pillar,
                );
            }
            NavigationPillar::CostBudgeting => {
                if !matches!(
                    self.planning_view.active_subtab,
                    crate::views::planning::PlanningSubTab::DynamicRab
                        | crate::views::planning::PlanningSubTab::TakeoffQto
                        | crate::views::planning::PlanningSubTab::CostEigenGateTradeoff
                        | crate::views::planning::PlanningSubTab::WbsHierarchy
                        | crate::views::planning::PlanningSubTab::EarnedValueEvm
                        | crate::views::planning::PlanningSubTab::ValueEngineeringBid
                        | crate::views::planning::PlanningSubTab::ProcurementContractsClaims
                ) {
                    self.planning_view.active_subtab =
                        crate::views::planning::PlanningSubTab::DynamicRab;
                }
                self.planning_view.show(
                    ui,
                    &self.theme,
                    &self.parametric_view.variables,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                    &mut self.active_pillar,
                );
            }
            NavigationPillar::SustainabilityEnergy => {
                self.sustainability_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::FacilityManagement => {
                self.facility_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::SafetyHealth => {
                self.safety_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::LeanConstruction => {
                self.lean_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::IndustrializedAutomation => {
                self.industrialized_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::Prediction => {
                let shm_feed = crate::views::prediction::ShmPredictionFeed {
                    strain_micro: self.facility_view.simulated_strain_micro,
                    freq_hz: self.facility_view.simulated_freq_hz,
                    degradation_pct: self.facility_view.simulated_degradation_pct,
                    live_packets: self.facility_view.shm_packets_received,
                };
                self.prediction_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    shm_feed,
                    &mut self.feature_registry,
                );
            }
            NavigationPillar::ReportPublishing => {
                self.results_view.show(
                    ui,
                    &self.theme,
                    &self.members,
                    &self.build_view.bim_bus.project,
                    &mut self.feature_registry,
                );
            }
        }
    }

    /// Applies deferred module actions that mutate `self.spatial_workspace` (template load,
    /// vault project open, or IFCX-to-BIM application), ensuring full access to `spatial_workspace`
    /// both in Classic Mode and after the Dynamic Island embedded panel completes.
    fn apply_pillar_post_render_actions(&mut self) {
        if let Some(kind) = self.my_project_view.request_apply_template.take() {
            self.apply_typology_template(kind);
            if self.spatial_workspace.spatial_mode_enabled {
                self.spatial_workspace.active_canvas_mode =
                    crate::views::spatial_workspace::SpatialCanvasMode::EarthGlobe;
                self.universal_canvas.globe_mode = true;
                self.spatial_workspace.active_overlay_pillar = None;
            }
        }
        if let Some(entry) = self.my_project_view.pending_open_vault_project.take() {
            self.open_vault_project_entry(entry);
        }
        if let Some(proj) = self.industrialized_view.request_apply_bim_project.take() {
            let el_cnt = proj.element_count();
            let pname = proj.project_name.clone();
            self.build_view.bim_bus = crate::core::BimCommandBus::from_project(proj.clone());
            self.parametric_view.bim_bus = crate::core::BimCommandBus::from_project(proj);
            self.parametric_view.active_level_idx = 0;
            self.sync_bim_to_fem();
            self.build_view
                .fit_view_to_project(&mut self.universal_canvas, &self.members);
            self.sync_earth_maps();
            self.build_view.modify_status_msg = Some(format!(
                "Loaded IFCX -> Editable BimProject '{}' ({} elements).",
                pname, el_cnt
            ));
            self.active_pillar = NavigationPillar::GeometryAssembly;
            if self.spatial_workspace.spatial_mode_enabled {
                self.spatial_workspace.active_canvas_mode =
                    crate::views::spatial_workspace::SpatialCanvasMode::EarthGlobe;
                self.universal_canvas.globe_mode = true;
                self.universal_canvas.ground_origin_anchored = true;
                self.spatial_workspace.active_overlay_pillar = None;
            }
        }
    }

    fn render_shortcuts_quick_jump_body(&mut self, ui: &mut egui::Ui) -> Option<NavigationPillar> {
        let mut jump_to: Option<NavigationPillar> = None;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Quick-Jump Module:")
                    .size(12.0)
                    .strong()
                    .color(self.theme.accent_blue),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.universal_canvas.quick_jump_query)
                    .desired_width((ui.available_width() - 160.0).max(120.0))
                    .hint_text("Filter any of 15 engineering modules..."),
            );
        });
        ui.add_space(6.0);
        let q = self.universal_canvas.quick_jump_query.trim().to_lowercase();
        ui.horizontal_wrapped(|ui| {
            for pillar in ALL_PILLARS {
                let label = format!("{} ({})", pillar.clean_name(), pillar.badge());
                if !q.is_empty() && !label.to_lowercase().contains(&q) {
                    continue;
                }
                let is_active = self.active_pillar == pillar;
                if icons::icon_button(
                    ui,
                    pillar.app_icon(),
                    label,
                    10.5,
                    if is_active {
                        Color32::WHITE
                    } else {
                        self.theme.text_primary
                    },
                    if is_active {
                        self.theme.accent_blue
                    } else {
                        self.theme.bg_sidebar
                    },
                    Stroke::new(1.0, self.theme.border_subtle),
                    CornerRadius::same(6),
                    Vec2::new(0.0, 24.0),
                )
                .clicked()
                {
                    jump_to = Some(pillar);
                }
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(
            RichText::new("Keyboard Shortcuts (Outside Text Focus)")
                .size(12.0)
                .strong()
                .color(self.theme.text_primary),
        );
        ui.add_space(4.0);
        egui::Grid::new("cad_shortcuts_grid")
            .num_columns(2)
            .spacing([18.0, 5.0])
            .show(ui, |ui| {
                for (keys, desc) in [
                    ("Space", "Freeze / Unfreeze Canvas scroll lock"),
                    ("G / [ ] / 1-9", "Toggle 3D IFCX Earth vs 2D Map (G), Tilt ([/]), Earth Brushes (1-9)"),
                    ("L / R / C / D / T / E / V", "2D Line, Rect, Circle, Dim, Callout, Eraser, Box Marquee"),
                    ("W / B / C / F", "3D Author Wall, Beam, Column, Footing"),
                    ("M / O / X / Del", "Move (+Step), Offset (+500mm), Mirror X, Delete"),
                    ("F8 / F3 / F4 / Z / Esc", "Ortho, Grid Snap, OSNAP, Fit Model, Cancel / Select"),
                    ("Ctrl+Z / Ctrl+Y", "Undo / Redo CAD or BIM command"),
                    ("Ctrl+K or F1", "Open or close Shortcuts & Quick-Jump"),
                ] {
                    ui.label(
                        RichText::new(keys)
                            .size(11.0)
                            .monospace()
                            .strong()
                            .color(self.theme.accent_blue),
                    );
                    ui.label(
                        RichText::new(desc)
                            .size(11.0)
                            .color(self.theme.text_secondary),
                    );
                    ui.end_row();
                }
            });

        ui.add_space(10.0);
        ui.label(
            RichText::new("Precision CAD & IFCX Command Line Syntax")
                .size(12.0)
                .strong()
                .color(self.theme.text_primary),
        );
        ui.label(
            RichText::new(
                "• Relative Cartesian: @6000,0  |  Polar Vector: @4500<45  |  Move: move @0,1200\n\
                 • Rotate: ro 90  |  Copy: cp @1500,0  |  Offset: o 250  |  3D Wall: wall 0,0->6000,0\n\
                 • IFCX City: /road  |  /bridge  |  /zone  |  /gradepad",
            )
            .size(10.5)
            .monospace()
            .color(self.theme.text_muted),
        );
        jump_to
    }

    fn render_shortcuts_quick_jump_modal_if_open(&mut self, ctx: &egui::Context) {
        if !self.universal_canvas.shortcuts_modal_open {
            return;
        }
        let mut open = self.universal_canvas.shortcuts_modal_open;
        let mut jump_to: Option<NavigationPillar> = None;
        egui::Window::new("CAD Shortcuts, Precision CLI & Quick-Jump (Ctrl/Cmd+K)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_size(MacTheme::clamp_modal_size(ctx, Vec2::new(640.0, 500.0)))
            .max_size(MacTheme::modal_max_size(ctx))
            .show(ctx, |ui| {
                jump_to = self.render_shortcuts_quick_jump_body(ui);
            });
        if let Some(p) = jump_to {
            self.active_pillar = p;
            self.universal_canvas.request_scroll_to_workbench = true;
            open = false;
        }
        self.universal_canvas.shortcuts_modal_open = open;
    }

    /// Renders the Vault connection setup body (shared between Classic Mode modal and Spatial Dynamic Island).
    /// Returns `true` when the setup panel should close (connected or guest mode selected).
    fn render_vault_startup_body(&mut self, ui: &mut egui::Ui) -> bool {
        let status = crate::core::InternalVault::connection_status();
        let mut close_now = false;
        ui.horizontal_wrapped(|ui| {
            icons::icon_label(
                ui,
                AppIcon::Database,
                18.0,
                self.theme.accent_blue,
                "Select How to Store Your Personal Projects",
                15.0,
                self.theme.text_primary,
                true,
            );
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("Connect an existing local Engineering Vault folder, create a brand new Vault, or explore the 15-module workspace first.")
                .size(12.0)
                .color(self.theme.text_secondary),
        );
        ui.add_space(8.0);

        if !status.status_msg.is_empty() {
            ui.label(RichText::new(&status.status_msg).size(11.0).color(self.theme.accent_blue));
        }
        if !status.error_msg.is_empty() {
            ui.label(RichText::new(&status.error_msg).size(11.0).color(self.theme.accent_red));
        }
        if let Some(ref fb) = self.vault_modal_feedback {
            ui.label(RichText::new(fb).size(11.0).color(self.theme.accent_orange));
        }

        ui.add_space(10.0);

        let outcome = crate::ui_kit::render_obsidian_vault_panel(
            ui,
            &self.theme,
            &mut self.vault_path_input,
            &mut self.vault_new_name_input,
            false,
            true,
            false,
        );
        if outcome.state_changed {
            self.personal_vault_projects =
                crate::core::InternalVault::list_personal_vault_projects();
            self.settings_view.vault_path_input = self.vault_path_input.clone();
        }
        if let Some(fb) = outcome.feedback {
            self.vault_modal_feedback = Some(fb);
        }
        if outcome.connected_now {
            self.vault_modal_feedback = None;
            close_now = true;
        }
        if outcome.enter_guest_mode {
            close_now = true;
        }
        close_now
    }

    fn render_vault_startup_modal_if_open(&mut self, ctx: &egui::Context, is_mobile: bool) {
        if !self.show_vault_startup_modal {
            return;
        }
        let modal_size = MacTheme::clamp_modal_size(ctx, Vec2::new(760.0, 490.0));
        let mut close_panel = false;

        egui::Window::new("Welcome to EigenGraph — Connect Your Engineering Vault")
            .collapsible(false)
            .resizable(true)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_size(modal_size)
            .max_size(MacTheme::modal_max_size(ctx))
            .frame(
                Frame::new()
                    .fill(self.theme.bg_card)
                    .stroke(Stroke::new(1.0, self.theme.accent_blue.gamma_multiply(0.55)))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::same(if is_mobile { 12 } else { 20 })),
            )
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    if self.render_vault_startup_body(ui) {
                        close_panel = true;
                    }
                });
            });

        if close_panel {
            self.show_vault_startup_modal = false;
        }
    }

    fn toggle_n1_disruption_on_fem_id(&mut self, fid: usize) {
        let already_same = self
            .universal_canvas
            .earth_fem_cache
            .n1_simulation
            .as_ref()
            .is_some_and(|s| s.removed_fem_id == fid);
        if already_same {
            self.universal_canvas.earth_fem_cache.n1_simulation = None;
            self.build_view.simulated_eliminated_ids.clear();
            self.spatial_workspace
                .push_trace("Restored member (N-1 simulation cleared)", true);
            return;
        }
        if self.universal_canvas.earth_fem_cache.cached_fem_model.is_none()
            || self.universal_canvas.earth_fem_cache.cached_baseline_analysis.is_none()
        {
            self.sync_bim_to_fem();
        }
        let Some(fem_model) = self.universal_canvas.earth_fem_cache.cached_fem_model.clone() else {
            return;
        };
        let Some(baseline) = self
            .universal_canvas
            .earth_fem_cache
            .cached_baseline_analysis
            .clone()
        else {
            return;
        };
        let report =
            crate::core::EigenGateEngine::simulate_single_member_removal(&fem_model, &baseline, fid);
        let Some((removed_kind, removed_bim_id)) =
            self.universal_canvas.earth_fem_cache.bim_ref(fid)
        else {
            return;
        };
        let removed_name = report.removed_member_name.clone();
        let summary = if report.is_kinematic_mechanism {
            format!(
                "N-1 Removed {}: KINEMATIC MECHANISM DETECTED (SVD=0, Δ={:.1}mm)",
                removed_name, report.post_max_disp_mm
            )
        } else {
            format!(
                "N-1 Removed {}: Stable (EigenGate={:.2}, DCR {:.0}%→{:.0}%, Δ {:.1}→{:.1}mm, +{} crit)",
                removed_name,
                report.residual_eigengate,
                report.baseline_max_dcr * 100.0,
                report.post_max_dcr * 100.0,
                report.baseline_max_disp_mm,
                report.post_max_disp_mm,
                report.newly_critical_fem_ids.len()
            )
        };
        self.universal_canvas.earth_fem_cache.n1_simulation =
            Some(crate::views::universal_canvas::EarthN1OverlayState {
                removed_kind,
                removed_bim_id,
                removed_fem_id: fid,
                removed_name,
                report,
            });
        self.build_view.simulated_eliminated_ids = vec![fid];
        self.spatial_workspace.push_trace(summary, true);
    }

    fn sync_bim_to_fem(&mut self) {
        let proj = &self.build_view.bim_bus.project;
        if proj.is_empty() {
            self.members.clear();
            self.universal_canvas.earth_fem_cache.clear();
            self.build_view.simulated_eliminated_ids.clear();
            self.analyze_view.synced_bim_element_count = 0;
            self.analyze_view.last_bim_sync_label = "Idle (0 BIM elements)".to_string();
            self.planning_view.sync_from_bim(&self.build_view.bim_bus.project, &self.members);
            return;
        }
        let prev_n1_target = self
            .universal_canvas
            .earth_fem_cache
            .n1_simulation
            .as_ref()
            .map(|s| (s.removed_kind, s.removed_bim_id));

        let site_coupling = self.universal_canvas.active_site_coupling();
        let (fem, fem_map, load_summary) = proj.to_structural_model_with_site_loads_and_ssi(
            &site_coupling,
            Some(&self.universal_canvas.subsurface_ssi),
        );
        let rev_hash = proj.content_revision_hash();

        let cache = &mut self.universal_canvas.earth_fem_cache;
        cache.clear();
        cache.bim_revision_hash = rev_hash;
        cache.total_gravity_kn = load_summary.total_gravity_kn;
        cache.seismic_base_shear_kn = load_summary.seismic_base_shear_kn;
        cache.wind_base_shear_kn = load_summary.wind_base_shear_kn;
        cache.load_case_label = load_summary.load_case_label.clone();
        for &(fid, kind, bid) in &fem_map {
            cache.bim_to_fem.entry((kind, bid)).or_insert(fid);
            cache.fem_to_bim.insert(fid, (kind, bid));
        }
        cache.node_coords_m = fem.nodes.iter().map(|n| [n.x, n.y, n.z]).collect();

        if fem.elements.is_empty() {
            self.members.clear();
            self.analyze_view.synced_bim_element_count = 0;
            self.analyze_view.last_bim_sync_label = "Waiting for frame/wall FEM elements".to_string();
            self.planning_view.sync_from_bim(&self.build_view.bim_bus.project, &self.members);
            return;
        }
        let stamp = crate::core::web_bridge::unix_timestamp_label();

        // 1. Solve Exact 6-DOF Elastic Equilibrium + LRFD Governing Envelope + Inter-Story Drift
        let fea_opt = crate::core::FemSolver::solve(&fem).ok();
        let lrfd_opt = crate::core::FemSolver::solve_lrfd_envelope(&fem, &[]).ok();

        if let Some(ref fea) = fea_opt {
            cache.static_nodal_disp_m = fea
                .nodal_displacements
                .iter()
                .map(|d| [d[0], d[1], d[2]])
                .collect();
            cache.max_disp_mm = fea.max_displacement_mm;
            cache.governing_combo = lrfd_opt
                .as_ref()
                .map(|e| e.governing_system_combination.clone())
                .unwrap_or_else(|| load_summary.load_case_label.clone());

            // Compute exact maximum inter-story drift ratio across all vertical frame members
            let mut node_idx_map = std::collections::HashMap::with_capacity(fem.nodes.len());
            for (idx, n) in fem.nodes.iter().enumerate() {
                node_idx_map.insert(n.id, idx);
            }
            let mut max_drift = 0.0_f64;
            for elem in &fem.elements {
                if let (Some(&i1), Some(&i2)) =
                    (node_idx_map.get(&elem.start_node), node_idx_map.get(&elem.end_node))
                {
                    let dy = (fem.nodes[i2].y - fem.nodes[i1].y).abs();
                    if dy >= 1.5 {
                        let d1 = fea.nodal_displacements[i1];
                        let d2 = fea.nodal_displacements[i2];
                        let rel_horiz = (d2[0] - d1[0]).hypot(d2[2] - d1[2]);
                        max_drift = max_drift.max(rel_horiz / dy);
                    }
                }
            }
            cache.max_story_drift_ratio = max_drift;
            cache.cached_fem_model = Some(fem.clone());
            cache.cached_baseline_analysis = Some(fea.clone());
        }

        // 2. Run Dynamic Modal Analysis (Modes 1..3) for Earth 3D Eigenmode Animation
        if let Ok(modal_rep) = crate::core::ModalEngine::analyze_for_earth_3d(
            &fem,
            3,
            load_summary.spectrum.clone(),
        ) {
            cache.modal_modes = modal_rep.modes;
        }

        // 3. Evaluate EigenGate & Populate True 6-DOF Member Forces
        match crate::core::EigenGateEngine::evaluate_redundancy(&fem) {
            Ok(assessment) => {
                self.members = fem
                    .elements
                    .iter()
                    .enumerate()
                    .map(|(elem_idx, el)| {
                        let res = assessment.member_results.iter().find(|r| r.member_id == el.id);
                        let eigengate_val = res.map(|r| r.eigengate).unwrap_or(0.0);
                        let axial_kn = fea_opt
                            .as_ref()
                            .and_then(|s| s.member_axial_forces.get(elem_idx))
                            .map(|f| f / 1000.0)
                            .or_else(|| res.map(|r| r.force_kn))
                            .unwrap_or(0.0);
                        let cap_kn = fea_opt
                            .as_ref()
                            .and_then(|s| s.member_capacities.get(elem_idx))
                            .map(|c| c / 1000.0)
                            .or_else(|| res.map(|r| r.capacity_kn))
                            .unwrap_or(0.0);
                        let (moment_my, moment_mz) = fea_opt
                            .as_ref()
                            .and_then(|s| s.member_bending_moments.get(elem_idx))
                            .map(|&(my, mz)| (my / 1000.0, mz / 1000.0))
                            .unwrap_or((0.0, 0.0));
                        let shear_vy = fea_opt
                            .as_ref()
                            .and_then(|s| s.member_shear_forces.get(elem_idx))
                            .map(|v| v / 1000.0)
                            .unwrap_or(0.0);
                        let pmv_dcr = fea_opt
                            .as_ref()
                            .and_then(|s| s.member_dcr_interactions.get(elem_idx))
                            .copied()
                            .unwrap_or_else(|| res.map(|r| r.utilization_initial).unwrap_or(0.0));
                        let util_val = pmv_dcr
                            .max(res.map(|r| r.utilization_initial).unwrap_or(0.0))
                            .min(2.0);
                        let second_order = res
                            .map(|r| r.second_order_dcr.max(pmv_dcr))
                            .unwrap_or(pmv_dcr);
                        StructuralMember {
                            id: el.id,
                            name: el.name.clone(),
                            section: el.section.name.clone(),
                            force_kn: axial_kn,
                            capacity_kn: cap_kn,
                            utilization: util_val,
                            moment_my_knm: moment_my,
                            moment_mz_knm: moment_mz,
                            shear_vy_kn: shear_vy,
                            is_critical: res.map(|r| r.is_critical).unwrap_or(false)
                                || util_val >= 0.85,
                            is_active: true,
                            eigengate_score: eigengate_val,
                            eigengate: eigengate_val,
                            svd_sensitivity: res
                                .map(|r| 1.0 - r.svd_margin.clamp(0.0, 1.0))
                                .unwrap_or(0.0),
                            svd_margin: res.map(|r| r.svd_margin).unwrap_or(0.0),
                            ci: res.map(|r| r.ci).unwrap_or(0.0),
                            rr: res.map(|r| r.rr).unwrap_or(0.0),
                            dcr_interaction: second_order,
                            ..Default::default()
                        }
                    })
                    .collect();
                for (idx, m) in self.members.iter().enumerate() {
                    self.universal_canvas
                        .earth_fem_cache
                        .fem_to_member_idx
                        .insert(m.id, idx);
                }
                self.analyze_view.synced_bim_element_count = self.members.len();
                self.analyze_view.last_bim_sync_label = format!(
                    "EigenGate={:.3} | 6-DOF Δ={:.1}mm | {} FEM ({})",
                    assessment.average_eigengate,
                    self.universal_canvas.earth_fem_cache.max_disp_mm,
                    self.members.len(),
                    stamp
                );
            }
            Err(_) => {
                self.members = fem
                    .elements
                    .iter()
                    .enumerate()
                    .map(|(elem_idx, el)| {
                        let (my, mz) = fea_opt
                            .as_ref()
                            .and_then(|s| s.member_bending_moments.get(elem_idx))
                            .map(|&(a, b)| (a / 1000.0, b / 1000.0))
                            .unwrap_or((0.0, 0.0));
                        let vy = fea_opt
                            .as_ref()
                            .and_then(|s| s.member_shear_forces.get(elem_idx))
                            .map(|v| v / 1000.0)
                            .unwrap_or(0.0);
                        let dcr = fea_opt
                            .as_ref()
                            .and_then(|s| s.member_dcr_interactions.get(elem_idx))
                            .copied()
                            .unwrap_or(0.0);
                        StructuralMember {
                            id: el.id,
                            name: el.name.clone(),
                            section: el.section.name.clone(),
                            utilization: dcr,
                            dcr_interaction: dcr,
                            moment_my_knm: my,
                            moment_mz_knm: mz,
                            shear_vy_kn: vy,
                            ..Default::default()
                        }
                    })
                    .collect();
                for (idx, m) in self.members.iter().enumerate() {
                    self.universal_canvas
                        .earth_fem_cache
                        .fem_to_member_idx
                        .insert(m.id, idx);
                }
                self.analyze_view.synced_bim_element_count = self.members.len();
                self.analyze_view.last_bim_sync_label = format!(
                    "Synced {} elems (partial frame) at {}",
                    self.members.len(),
                    stamp
                );
            }
        }

        // Re-apply active N-1 simulation smoothly if the removed element still exists after a resize/edit
        if let Some((rk, rbid)) = prev_n1_target {
            if let Some(new_fid) = self.universal_canvas.earth_fem_cache.fem_id(rk, rbid) {
                self.toggle_n1_disruption_on_fem_id(new_fid);
            } else {
                self.build_view.simulated_eliminated_ids.clear();
            }
        }

        if self.universal_canvas.subsurface_ssi.ssi_coupling_enabled
            || self.universal_canvas.subsurface_ssi.xray_subsurface_visible
        {
            self.universal_canvas
                .refresh_subsurface_ssi(&self.build_view.bim_bus.project);
        } else {
            self.universal_canvas.last_subsurface_ssi_report = None;
        }

        if self.universal_canvas.ifcx_baseline_snapshot.is_some()
            && (self.universal_canvas.ifcx_diff_overlay_active
                || self.spatial_workspace.ifcx_diff_drawer_open)
        {
            self.universal_canvas
                .refresh_ifcx_visual_diff(&self.build_view.bim_bus.project, &self.members);
        }

        self.planning_view
            .sync_from_bim(&self.build_view.bim_bus.project, &self.members);
    }

    fn apply_typology_template(&mut self, kind: crate::views::my_project::ProjectTemplateKind) {
        let proj = crate::views::my_project::instantiate_typology_template(kind);
        self.build_view.bim_bus.project = proj.clone();
        self.parametric_view.bim_bus.project = proj;
        self.universal_canvas.ground_origin_anchored = true;
        self.sync_bim_to_fem();
        self.build_view.fit_view_to_project(&mut self.universal_canvas, &self.members);
        self.sync_earth_maps();
        let msg = format!(
            "Instantiated '{}' template ({} BIM elements, {} FEM members, {:.0} kg rebar).",
            kind.title(),
            self.build_view.bim_bus.project.element_count(),
            self.members.len(),
            self.build_view.bim_bus.project.total_rebar_weight_kg(),
        );
        self.my_project_view.export_success_msg = Some(msg.clone());
        self.universal_canvas.status_message = Some(msg);
        self.active_pillar = NavigationPillar::GeometryAssembly;
    }

    fn sync_earth_maps(&mut self) {
        let cur_uc = (
            self.universal_canvas.world_map.center_lat_lon.0,
            self.universal_canvas.world_map.center_lat_lon.1,
            self.universal_canvas.world_map.zoom_level,
            self.universal_canvas.world_map.tile_provider,
        );
        let cur_res = (
            self.research_view.slippy_map.center_lat_lon.0,
            self.research_view.slippy_map.center_lat_lon.1,
            self.research_view.slippy_map.zoom_level,
            self.research_view.slippy_map.tile_provider,
        );
        let differs = |a: (f64, f64, f32, crate::core::gis::TileSource),
                       b: (f64, f64, f32, crate::core::gis::TileSource)| -> bool {
            (a.0 - b.0).abs() > 1e-5
                || (a.1 - b.1).abs() > 1e-5
                || (a.2 - b.2).abs() > 1e-3
                || a.3 != b.3
        };
        if differs(cur_uc, self.last_uc_earth) {
            self.research_view.slippy_map.set_center(cur_uc.0, cur_uc.1);
            self.research_view.slippy_map.zoom_level = cur_uc.2;
            self.research_view.slippy_map.tile_provider = cur_uc.3;
        } else if differs(cur_res, self.last_res_earth) {
            self.universal_canvas.world_map.set_center(cur_res.0, cur_res.1);
            self.universal_canvas.world_map.zoom_level = cur_res.2;
            self.universal_canvas.world_map.tile_provider = cur_res.3;
        }
        let synced = (
            self.universal_canvas.world_map.center_lat_lon.0,
            self.universal_canvas.world_map.center_lat_lon.1,
            self.universal_canvas.world_map.zoom_level,
            self.universal_canvas.world_map.tile_provider,
        );
        self.last_uc_earth = synced;
        self.last_res_earth = synced;
        self.sync_site_coupling();
        if self.universal_canvas.district_hazard.any_active() {
            self.universal_canvas
                .refresh_district_hazard(&self.build_view.bim_bus.project);
        }
    }

    fn sync_site_coupling(&mut self) {
        let coupling = self.universal_canvas.active_site_coupling();
        self.universal_canvas.lifecycle_ops.site_coupling = coupling.clone();
        self.analyze_view.site_coupling = coupling.clone();
        self.planning_view.site_coupling = coupling.clone();
        self.lean_view.site_coupling = coupling;
    }

    fn sync_lifecycle_to_canvas(&mut self) {
        self.sync_site_coupling();
        self.universal_canvas.lifecycle_ops.procurement = self.planning_view.procurement.clone();
        self.universal_canvas.lifecycle_ops.daily_diary = self.planning_view.daily_diary.clone();
        self.universal_canvas.lifecycle_ops.qa_qc = self.lean_view.qa_qc.clone();
    }

    fn sync_lifecycle_from_canvas(&mut self) {
        self.planning_view.procurement = self.universal_canvas.lifecycle_ops.procurement.clone();
        self.planning_view.daily_diary = self.universal_canvas.lifecycle_ops.daily_diary.clone();
        self.lean_view.qa_qc = self.universal_canvas.lifecycle_ops.qa_qc.clone();
        self.sync_site_coupling();
        if self.universal_canvas.district_hazard.any_active() {
            self.universal_canvas
                .refresh_district_hazard(&self.build_view.bim_bus.project);
        }
    }

    fn handle_imported_file(&mut self, payload: crate::core::web_bridge::ImportedFilePayload) {
        let lower = payload.filename.to_lowercase();
        let trimmed = payload.content.trim_start_matches('\u{FEFF}').trim_start();
        if lower.ends_with(".ifcx-bundle.json")
            || trimmed.contains("\"EIGEN-IFCX-COLLAB-1.0\"")
        {
            match self.universal_canvas.collab_state.import_bundle_json(trimmed) {
                Ok(cnt) => {
                    self.spatial_workspace.open_collab_drawer(0);
                    self.spatial_workspace.push_trace(
                        format!(
                            "Imported & SHA-256 verified '{}' ({cnt} IFCX checkpoints)",
                            payload.filename
                        ),
                        true,
                    );
                }
                Err(err) => self.spatial_workspace.push_trace(err, false),
            }
            return;
        }
        if lower.ends_with(".ifcx") || crate::core::ifcx::IfcxDocument::looks_like_ifcx(trimmed) {
            let clean_raw = payload.content.trim_start_matches('\u{FEFF}');
            if self.spatial_workspace.spatial_mode_enabled {
                let import_rel = format!("Imports/{}", payload.filename);
                self.import_ifcx_raw_onto_earth(&payload.filename, &payload.filename, &import_rel, clean_raw);
                return;
            }
            match crate::core::ifcx::IfcxDocument::parse(clean_raw) {
                Ok(doc) => {
                    self.industrialized_view.load_ifcx_document(doc, &payload.filename);
                    self.industrialized_view.active_subtab =
                        crate::views::industrialized::IndustrializedSubTab::OpenBimIfcxStudio;
                    self.active_pillar = NavigationPillar::IndustrializedAutomation;
                }
                Err(e) => {
                    self.industrialized_view.export_status =
                        Some(format!("IFCX parse error for '{}': {}", payload.filename, e));
                }
            }
            return;
        }

        if crate::core::looks_like_shm_telemetry_payload(trimmed) {
            match crate::core::parse_shm_telemetry_log(trimmed, &payload.filename) {
                Ok(log) => {
                    let ch = log.story_channels;
                    let cnt = log.records.len();
                    let fs_hz = log.sampling_rate_hz;
                    self.universal_canvas.shm_twin_config.custom_telemetry = Some(log);
                    self.universal_canvas.shm_twin_config.fault_scenario =
                        crate::core::ShmFaultScenario::CustomTelemetryStream;
                    if self.build_view.bim_bus.project.is_empty() {
                        if let Ok(msg) = self
                            .build_view
                            .bim_bus
                            .execute_natural_language("portal 3x2 @6m 2floors")
                        {
                            self.universal_canvas.ground_origin_anchored = true;
                            self.spatial_workspace.push_trace(msg, true);
                            self.sync_bim_to_fem();
                        }
                    }
                    self.spatial_workspace.apply_lens_to_views(
                        crate::views::spatial_workspace::SpatialLens::ShmDigitalTwin,
                        &mut self.build_view,
                        &mut self.universal_canvas,
                    );
                    self.spatial_workspace.open_shm_drawer();
                    match self
                        .universal_canvas
                        .run_shm_digital_twin(&self.build_view.bim_bus.project)
                    {
                        Ok(summary) => {
                            self.spatial_workspace.push_trace(
                                format!(
                                    "Ingested SHM telemetry '{}' ({cnt} pts, {ch} stories @ {fs_hz:.0}Hz): {summary}",
                                    payload.filename
                                ),
                                true,
                            );
                        }
                        Err(err) => self.spatial_workspace.push_trace(err, false),
                    }
                    return;
                }
                Err(err) => {
                    self.spatial_workspace.push_trace(err, false);
                    return;
                }
            }
        }

        if crate::core::looks_like_accelerogram_payload(&payload.filename, trimmed) {
            match crate::core::parse_custom_accelerogram(trimmed, &payload.filename) {
                Ok(acc) => {
                    let rec_name = acc.name.clone();
                    let pga = acc.pga_x_g;
                    let dt = acc.dt_s;
                    let n_pts = acc.accel_x_g.len();
                    self.universal_canvas
                        .ground_motion_replay_config
                        .custom_accelerogram = Some(acc);
                    if self.build_view.bim_bus.project.is_empty() {
                        if let Ok(msg) = self
                            .build_view
                            .bim_bus
                            .execute_natural_language("portal 3x2 @6m 2floors")
                        {
                            self.universal_canvas.ground_origin_anchored = true;
                            self.spatial_workspace.push_trace(msg, true);
                            self.sync_bim_to_fem();
                        }
                    }
                    self.spatial_workspace.apply_lens_to_views(
                        crate::views::spatial_workspace::SpatialLens::TimeHistoryReplay,
                        &mut self.build_view,
                        &mut self.universal_canvas,
                    );
                    self.spatial_workspace.open_ground_motion_drawer();
                    self.universal_canvas
                        .start_async_ground_motion_replay(&self.build_view.bim_bus.project);
                    self.spatial_workspace.push_trace(
                        format!(
                            "Loaded PEER/CSV accelerogram '{rec_name}' (PGA={pga:.3}g, dt={dt:.4}s, {n_pts} pts) — streaming async NL-THA..."
                        ),
                        true,
                    );
                    return;
                }
                Err(err) => {
                    if lower.ends_with(".at2") {
                        self.spatial_workspace.push_trace(err, false);
                        return;
                    }
                }
            }
        }

        if self.ingest_geojson_or_survey_onto_earth(&payload.filename, payload.content.trim_start_matches('\u{FEFF}')) {
            if lower.ends_with(".xyz") || lower.ends_with(".pts") {
                let _ = self.research_view.import_survey_points_csv_xyz(&payload.content);
            }
            return;
        }
        if lower.ends_with(".csv") || lower.ends_with(".xyz") || lower.ends_with(".pts") {
            match self.research_view.import_survey_points_csv_xyz(&payload.content) {
                Ok(cnt) => {
                    if let Ok(tin) = crate::core::gis::parse_survey_xyz_to_tin(&payload.content) {
                        self.universal_canvas.active_site_tin = Some(tin);
                        self.universal_canvas.site_tin_from_ifc = false;
                        self.universal_canvas.show_terrain_contours = true;
                    }
                    self.universal_canvas.status_message = Some(format!(
                        "Imported {} survey points from '{}' into Earth 3D & Planning TIN terrain.",
                        cnt, payload.filename
                    ));
                    if !self.spatial_workspace.spatial_mode_enabled {
                        self.active_pillar = NavigationPillar::ResearchConcept;
                    } else {
                        self.spatial_workspace.push_trace(
                            format!("Loaded {cnt} survey terrain points onto Earth 3D"),
                            true,
                        );
                    }
                }
                Err(e) => {
                    self.universal_canvas.status_message =
                        Some(format!("Survey import error '{}': {}", payload.filename, e));
                }
            }
            return;
        }

        let settings_snap = self.settings_view.workspace_settings_snapshot(
            &self.theme,
            &self.universal_canvas,
            self.ai_assistant_view.is_visible,
        );
        if let Ok(ws_cfg) = self.universal_canvas.apply_unified_archive(
            &payload.content,
            &mut self.build_view.bim_bus.project,
            &mut self.members,
            &mut self.parametric_view.variables,
        ) {
            self.settings_view.apply_workspace_settings(
                &ws_cfg,
                &mut self.theme,
                &mut self.ai_assistant_view.is_visible,
            );
            self.parametric_view.bim_bus.project = self.build_view.bim_bus.project.clone();
            self.sync_bim_to_fem();
            self.sync_earth_maps();
            self.sync_lifecycle_from_canvas();
            let msg = format!(
                "Loaded workspace archive '{}' ({} BIM elems, {} drawings).",
                payload.filename,
                self.build_view.bim_bus.project.element_count(),
                self.universal_canvas.entities.len()
            );
            self.my_project_view.export_success_msg = Some(msg.clone());
            self.universal_canvas.status_message = Some(msg);
            self.active_pillar = NavigationPillar::GeometryAssembly;
        } else if let Ok(model) =
            serde_json::from_str::<crate::core::model::StructuralModel>(&payload.content)
        {
            self.members = model
                .elements
                .iter()
                .map(|el| StructuralMember {
                    id: el.id,
                    name: el.name.clone(),
                    section: el.section.name.clone(),
                    ..Default::default()
                })
                .collect();
            let msg = format!(
                "Loaded structural JSON '{}' ({} nodes, {} elements).",
                payload.filename,
                model.nodes.len(),
                model.elements.len()
            );
            self.my_project_view.export_success_msg = Some(msg.clone());
            self.universal_canvas.status_message = Some(msg);
        } else {
            let _ = settings_snap;
            self.my_project_view.export_success_msg = Some(format!(
                "Unrecognized archive format in '{}'. Supported: .ifcx, .eigen, .json, .csv, .xyz.",
                payload.filename
            ));
        }
    }

    fn maybe_autosave_workspace(
        &mut self,
        ctx: &egui::Context,
        workspace_snapshot: &crate::views::universal_canvas::WorkspaceCanvasSettings,
    ) {
        if !self.autosave_hydrated {
            return;
        }
        let now = ctx.input(|i| i.time);
        if (now - self.last_autosave_time) < 2.5 {
            return;
        }
        self.last_autosave_time = now;

        let is_blank = self.build_view.bim_bus.project.is_empty()
            && self.members.is_empty()
            && self.universal_canvas.entities.is_empty()
            && self.parametric_view.variables.is_empty();
        if is_blank && self.last_autosave_hash == 0 {
            return;
        }

        self.sync_lifecycle_to_canvas();
        let archive = self.universal_canvas.build_unified_archive(
            &self.build_view.bim_bus.project,
            &self.members,
            &self.parametric_view.variables,
            workspace_snapshot.clone(),
        );
        if let Ok(json) = serde_json::to_string(&archive) {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in json.as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            if h != self.last_autosave_hash {
                self.last_autosave_hash = h;
                let _ = crate::core::InternalVault::autosave_workspace_json(&json);
            }
        }
    }
}
