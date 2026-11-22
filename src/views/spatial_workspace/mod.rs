//! views/spatial_workspace.rs — Phase 46: Antigravity Spatial Canvas Shell,
//! Unified Command Palette (`/`) & Smart Context Pop-Up Inspector.
//!
//! Provides a zero-chrome full-viewport Earth + BIM/CAD spatial workspace where all 15
//! engineering modules, structural lenses, generative BIM commands, and element-level
//! calculations live behind an Antigravity-style AI Command Bar and context-sensitive
//! floating element pop-ups.

use egui::Rect;

use crate::core::{BimCommand, BimElementKind, BimProject};
use crate::models::{NavigationPillar, StructuralMember};
use crate::views::build::Ground3DAuthorTool;
use crate::views::universal_canvas::UniversalCanvasTool;
use crate::views::{AiAssistantView, BuildView, UniversalCanvasState};


mod commands;
mod drawers;
mod island_ui;
mod smart_popup;
mod types;
pub use island_ui::SpatialIslandHost;
pub use types::*;

/// Complete state for the Phase 48 Full-Canvas + Hover Dynamic Island UI.
pub struct SpatialWorkspaceState {
    pub spatial_mode_enabled: bool,
    pub active_canvas_mode: SpatialCanvasMode,
    pub active_lens: SpatialLens,
    pub selected_target: Option<SmartSelectionTarget>,
    pub popup_pinned: bool,
    pub popup_tab: SmartPopupTab,
    pub command_bar_expanded: bool,
    pub dynamic_island_rect: Rect,
    pub island_expanded_prev: bool,
    pub agent_mode: AgentExecutionMode,
    pub agent_model: AgentModelChoice,
    pub attached_contexts: Vec<AiContextAttachment>,
    pub context_picker_open: bool,
    pub context_search_query: String,
    pub vault_drawer_open: bool,
    pub vault_path_popover_open: bool,
    pub vault_inline_path: String,
    pub vault_inline_name: String,
    pub vault_feedback: Option<String>,
    pub history_popover_open: bool,
    pub conversation_title: String,
    pub conversation_archives: Vec<AgentThreadArchive>,
    pub turn_metadata: Vec<AntigravityTurnMeta>,
    pub pending_context_file_pick: bool,
    pub command_input: String,
    pub request_focus_command: bool,
    pub active_overlay_pillar: Option<NavigationPillar>,
    pub recent_trace_chips: Vec<AgentTraceChip>,
    pub last_uc_pick_seq: u64,
    pub last_build_pick_seq: u64,
    pub hud_rects: Vec<Rect>,
    composer_had_focus: bool,
    last_command_len: usize,

    // Phase 49 & 50: Live Gemini API, Planning Proposals, Obsidian Notes & Precision Placement
    pub gemini_api_key_draft: String,
    pub remember_gemini_key: bool,
    pub api_key_popover_open: bool,
    pub gemini_inflight_req_id: Option<u64>,
    pub pending_proposals: Vec<PlannedAgentProposal>,
    next_proposal_id: u64,
    pub sessions_dirty: bool,
    pub vault_sessions_hydrated: bool,
    pub vault_note_composer_open: bool,
    pub vault_note_title_input: String,
    pub vault_note_body_input: String,
    pub previewed_vault_note: Option<(String, String)>,
    pub preview_note_editing: bool,
    pub manual_place_x_mm: f64,
    pub manual_place_y_mm: f64,
    pub manual_polar_len_m: f64,
    pub manual_polar_deg: f64,

    // Editable element inspector draft state synced lazily when selection changes
    last_loaded_target: Option<SmartSelectionTarget>,
    pub edit_width_mm: f64,
    pub edit_depth_mm: f64,
    pub edit_length_mm: f64,
    pub edit_thickness_mm: f64,
    pub edit_rot_deg: f64,
    pub edit_is_structural: bool,
    pub edit_profile_name: String,
    pub active_4d_day: u32,
    // Phase 52: Mobile/touch island pin toggle, precision 2D draw drafts, & 2D entity style editor
    pub mobile_deck_pinned: bool,
    pub draw_2d_len_m: f64,
    pub draw_2d_deg: f64,
    pub draw_2d_rect_w_m: f64,
    pub draw_2d_rect_h_m: f64,
    pub draw_2d_radius_m: f64,
    pub sketch_edit_color: [u8; 3],
    pub sketch_edit_width: f32,
    // Phase 53: Chronological mixed Undo/Redo, Story/Slab authoring, & Obsidian Vault organizer
    pub undo_timeline: Vec<SpatialUndoDomain>,
    pub redo_timeline: Vec<SpatialUndoDomain>,
    last_bim_undo_len: usize,
    last_uc_undo_len: usize,
    pub active_story_idx: usize,
    pub vault_filter_query: String,
    pub vault_new_folder_name: String,
    pub vault_rename_target: Option<(String, String)>,
    // Phase 54: Restructured essential Dynamic Island drawer, subfolder note save, & structural triage
    pub hover_more_tools_open: bool,
    pub studio_tab: DynamicIslandStudioTab,
    pub vault_note_folder: String,
    // Phase 58: Planetary Portfolio triage filter inside Vault drawer & Dynamic Island
    pub world_portfolio_filter: crate::core::gis::WorldPortfolioFilter,
    // Phase 59: Batch City Geo-Assign UI state inside Vault drawer
    pub batch_pin_query: String,
    pub batch_pin_city_idx: usize,
    // Phase 72: Multi-Layer IFCX Visual Diff Drawer inside Single Dynamic Island
    pub ifcx_diff_drawer_open: bool,
}

impl Default for SpatialWorkspaceState {
    fn default() -> Self {
        Self {
            spatial_mode_enabled: true,
            active_canvas_mode: SpatialCanvasMode::EarthGlobe,
            active_lens: SpatialLens::Clean,
            selected_target: None,
            popup_pinned: false,
            popup_tab: SmartPopupTab::Properties,
            command_bar_expanded: false,
            dynamic_island_rect: Rect::NOTHING,
            island_expanded_prev: false,
            agent_mode: AgentExecutionMode::Planning,
            agent_model: AgentModelChoice::EigenAiProFem,
            attached_contexts: Vec::new(),
            context_picker_open: false,
            context_search_query: String::new(),
            vault_drawer_open: false,
            vault_path_popover_open: false,
            vault_inline_path: crate::core::InternalVault::default_vault_dir()
                .to_string_lossy()
                .to_string(),
            vault_inline_name: "EigenGraph_Vault".to_string(),
            vault_feedback: None,
            history_popover_open: false,
            conversation_title: "EigenAI Spatial Agent".to_string(),
            conversation_archives: Vec::new(),
            turn_metadata: Vec::new(),
            pending_context_file_pick: false,
            command_input: String::new(),
            request_focus_command: false,
            active_overlay_pillar: None,
            recent_trace_chips: vec![AgentTraceChip {
                label: "Earth Globe Ready • Hover Dynamic Island for Vault, AI & Draw".to_string(),
                ok: true,
            }],
            last_uc_pick_seq: 0,
            last_build_pick_seq: 0,
            hud_rects: Vec::new(),
            composer_had_focus: false,
            last_command_len: 0,
            gemini_api_key_draft: String::new(),
            remember_gemini_key: false,
            api_key_popover_open: false,
            gemini_inflight_req_id: None,
            pending_proposals: Vec::new(),
            next_proposal_id: 1,
            sessions_dirty: false,
            vault_sessions_hydrated: false,
            vault_note_composer_open: false,
            vault_note_title_input: String::new(),
            vault_note_body_input: String::new(),
            previewed_vault_note: None,
            preview_note_editing: false,
            manual_place_x_mm: 0.0,
            manual_place_y_mm: 0.0,
            manual_polar_len_m: 6.0,
            manual_polar_deg: 0.0,
            last_loaded_target: None,
            edit_width_mm: 400.0,
            edit_depth_mm: 400.0,
            edit_length_mm: 1400.0,
            edit_thickness_mm: 450.0,
            edit_rot_deg: 0.0,
            edit_is_structural: true,
            edit_profile_name: "RC 400x400".to_string(),
            active_4d_day: 90,
            mobile_deck_pinned: false,
            draw_2d_len_m: 6.0,
            draw_2d_deg: 0.0,
            draw_2d_rect_w_m: 8.0,
            draw_2d_rect_h_m: 5.0,
            draw_2d_radius_m: 3.0,
            sketch_edit_color: [59, 130, 246],
            sketch_edit_width: 2.2,
            undo_timeline: Vec::new(),
            redo_timeline: Vec::new(),
            last_bim_undo_len: 0,
            last_uc_undo_len: 0,
            active_story_idx: 0,
            vault_filter_query: String::new(),
            vault_new_folder_name: String::new(),
            vault_rename_target: None,
            hover_more_tools_open: false,
            studio_tab: DynamicIslandStudioTab::ViewAndLens,
            vault_note_folder: "KnowledgeBase".to_string(),
            world_portfolio_filter: crate::core::gis::WorldPortfolioFilter::All,
            batch_pin_query: String::new(),
            batch_pin_city_idx: 0,
            ifcx_diff_drawer_open: false,
        }
    }
}

impl SpatialWorkspaceState {
    /// Returns true when the Floating Dynamic Island is hovered, has an active composer/popover,
    /// or displays the active Earth Site Location card.
    pub fn is_dynamic_island_expanded(&self, ctx: &egui::Context) -> bool {
        let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
        let hover_margin = if self.island_expanded_prev { 18.0 } else { 4.0 };
        let hovered_island = self.dynamic_island_rect.is_positive()
            && pointer_pos.is_some_and(|p| {
                self.dynamic_island_rect.expand(hover_margin).contains(p)
            });
        let engaged = self.composer_had_focus
            || !self.command_input.trim().is_empty()
            || self.context_picker_open
            || self.vault_drawer_open
            || self.vault_path_popover_open
            || self.history_popover_open
            || self.api_key_popover_open
            || self.ifcx_diff_drawer_open
            || self.gemini_inflight_req_id.is_some()
            || self.mobile_deck_pinned
            || self.selected_target.is_some()
            || self.command_bar_expanded;
        hovered_island || engaged
    }

    /// Collapses any pinned Dynamic Island drawers/popovers so moving cursor to Earth folds it back.
    pub fn collapse_dynamic_island(&mut self) {
        self.command_bar_expanded = false;
        self.context_picker_open = false;
        self.vault_drawer_open = false;
        self.vault_path_popover_open = false;
        self.history_popover_open = false;
        self.api_key_popover_open = false;
        self.ifcx_diff_drawer_open = false;
        self.mobile_deck_pinned = false;
        self.hover_more_tools_open = false;
        self.composer_had_focus = false;
        self.selected_target = None;
        self.popup_pinned = false;
    }

    /// Toggles `"vault"`, `"context"`, `"history"`, or `"api_key"` exclusively so
    /// multiple popovers never stack vertically inside the Dynamic Island.
    pub fn toggle_exclusive_drawer(&mut self, drawer: &'static str) {
        match drawer {
            "vault" => {
                let next = !self.vault_drawer_open;
                self.vault_drawer_open = next;
                if next {
                    self.context_picker_open = false;
                    self.history_popover_open = false;
                    self.api_key_popover_open = false;
                    self.ifcx_diff_drawer_open = false;
                } else {
                    self.vault_path_popover_open = false;
                }
            }
            "context" => {
                let next = !self.context_picker_open;
                self.context_picker_open = next;
                if next {
                    self.vault_drawer_open = false;
                    self.vault_path_popover_open = false;
                    self.history_popover_open = false;
                    self.api_key_popover_open = false;
                    self.ifcx_diff_drawer_open = false;
                }
            }
            "history" => {
                let next = !self.history_popover_open;
                self.history_popover_open = next;
                if next {
                    self.vault_drawer_open = false;
                    self.vault_path_popover_open = false;
                    self.context_picker_open = false;
                    self.api_key_popover_open = false;
                    self.ifcx_diff_drawer_open = false;
                }
            }
            "api_key" => {
                let next = !self.api_key_popover_open;
                self.api_key_popover_open = next;
                if next {
                    self.vault_drawer_open = false;
                    self.vault_path_popover_open = false;
                    self.context_picker_open = false;
                    self.history_popover_open = false;
                    self.ifcx_diff_drawer_open = false;
                }
            }
            "ifcx_diff" => {
                let next = !self.ifcx_diff_drawer_open;
                self.ifcx_diff_drawer_open = next;
                if next {
                    self.vault_drawer_open = false;
                    self.vault_path_popover_open = false;
                    self.context_picker_open = false;
                    self.history_popover_open = false;
                    self.api_key_popover_open = false;
                }
            }
            _ => {}
        }
    }

    /// Computes global structural health telemetry + worst critical element target for 1-click triage.
    /// Fast O(M) structural health summary using the cached `EarthStructuralRenderCache` lookup table.
    pub fn structural_health_summary_cached(
        &self,
        project: &BimProject,
        members: &[StructuralMember],
        fem_cache: &crate::views::universal_canvas::EarthStructuralRenderCache,
    ) -> (Option<SmartSelectionTarget>, f64, f64, usize) {
        if !fem_cache.fem_to_bim.is_empty() {
            let mut worst_target: Option<SmartSelectionTarget> = None;
            let mut worst_score = -1.0_f64;
            let mut max_dcr = 0.0_f64;
            let mut min_ari = 1.0_f64;
            let mut crit_count = 0usize;
            for m in members {
                let dcr_pct = m.utilization * 100.0;
                max_dcr = max_dcr.max(dcr_pct);
                min_ari = min_ari.min(m.ari);
                if m.is_critical || m.ari < 0.70 || m.utilization > 0.95 {
                    crit_count += 1;
                }
                let severity = m.utilization + (1.0 - m.ari).max(0.0);
                if severity > worst_score {
                    if let Some((kind, id)) = fem_cache.bim_ref(m.id) {
                        worst_score = severity;
                        worst_target = Some(Self::target_from_bim(kind, id));
                    }
                }
            }
            return (worst_target, max_dcr, min_ari, crit_count);
        }
        self.structural_health_summary(project, members)
    }

    pub fn structural_health_summary(
        &self,
        project: &BimProject,
        members: &[StructuralMember],
    ) -> (Option<SmartSelectionTarget>, f64, f64, usize) {
        let mut worst_target: Option<SmartSelectionTarget> = None;
        let mut worst_score = -1.0_f64;
        let mut max_dcr = 0.0_f64;
        let mut min_ari = 1.0_f64;
        let mut crit_count = 0usize;
        let (_, fem_map) = project.to_structural_model_with_map();
        let mut fem_to_bim = std::collections::HashMap::with_capacity(fem_map.len());
        for (fid, kind, id) in fem_map {
            fem_to_bim.insert(fid, (kind, id));
        }
        for m in members {
            let dcr_pct = m.utilization * 100.0;
            max_dcr = max_dcr.max(dcr_pct);
            min_ari = min_ari.min(m.ari);
            if m.is_critical || m.ari < 0.70 || m.utilization > 0.95 {
                crit_count += 1;
            }
            let severity = m.utilization + (1.0 - m.ari).max(0.0);
            if severity > worst_score {
                if let Some(&(kind, id)) = fem_to_bim.get(&m.id) {
                    worst_score = severity;
                    worst_target = Some(Self::target_from_bim(kind, id));
                }
            }
        }
        (worst_target, max_dcr, min_ari, crit_count)
    }

    /// Formats top-6 bottleneck BIM elements with exact IDs & section dimensions for Gemini prompts.
    pub fn format_critical_bim_context(
        project: &BimProject,
        members: &[StructuralMember],
    ) -> String {
        let (_, fem_map) = project.to_structural_model_with_map();
        let mut fem_to_bim = std::collections::HashMap::with_capacity(fem_map.len());
        for (fid, kind, id) in fem_map {
            fem_to_bim.insert(fid, (kind, id));
        }
        let mut ranked: Vec<(f64, String)> = Vec::new();
        for m in members {
            if let Some(&(kind, id)) = fem_to_bim.get(&m.id) {
                let tag = match kind {
                    BimElementKind::Column => project
                        .columns
                        .iter()
                        .find(|c| c.id == id)
                        .map(|c| format!("Column #{id} ({:.0}x{:.0}mm)", c.width_mm, c.depth_mm)),
                    BimElementKind::Run => project
                        .runs
                        .iter()
                        .find(|r| r.id == id)
                        .map(|r| format!("Beam #{id} ({:.0}x{:.0}mm)", r.width_mm, r.depth_mm)),
                    BimElementKind::Wall => Some(format!("Wall #{id}")),
                    BimElementKind::Footing => Some(format!("Footing #{id}")),
                    BimElementKind::Slab => Some(format!("Slab #{id}")),
                };
                if let Some(desc) = tag {
                    ranked.push((
                        m.utilization + (1.0 - m.ari).max(0.0),
                        format!("{desc}: DCR={:.1}%, ARI={:.3}", m.utilization * 100.0, m.ari),
                    ));
                }
            }
        }
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        let top_list: Vec<String> = ranked.into_iter().take(6).map(|(_, s)| s).collect();
        if top_list.is_empty() {
            "No FEM elements synced yet.".to_string()
        } else {
            top_list.join("; ")
        }
    }

    /// Records new undo push events from either BIM or 2D Canvas in strict chronological LIFO order.
    pub fn observe_undo_stacks(&mut self, bim_undo_len: usize, uc_undo_len: usize) {
        if bim_undo_len > self.last_bim_undo_len {
            let delta = bim_undo_len - self.last_bim_undo_len;
            for _ in 0..delta {
                self.undo_timeline.push(SpatialUndoDomain::Bim3D);
            }
            self.redo_timeline.clear();
        }
        if uc_undo_len > self.last_uc_undo_len {
            let delta = uc_undo_len - self.last_uc_undo_len;
            for _ in 0..delta {
                self.undo_timeline.push(SpatialUndoDomain::Canvas2D);
            }
            self.redo_timeline.clear();
        }
        self.last_bim_undo_len = bim_undo_len;
        self.last_uc_undo_len = uc_undo_len;
    }

    /// Synchronizes undo baseline counters after commit/undo/redo so no spurious growth is logged.
    pub fn sync_undo_baseline(&mut self, bim_undo_len: usize, uc_undo_len: usize) {
        self.last_bim_undo_len = bim_undo_len;
        self.last_uc_undo_len = uc_undo_len;
    }

    /// Resolves the most recent domain to undo chronologically (LIFO) across BIM & 2D Canvas.
    pub fn pop_undo_domain(
        &mut self,
        bim_undo_len: usize,
        uc_undo_len: usize,
    ) -> Option<SpatialUndoDomain> {
        while let Some(dom) = self.undo_timeline.pop() {
            match dom {
                SpatialUndoDomain::Bim3D if bim_undo_len > 0 => {
                    self.redo_timeline.push(SpatialUndoDomain::Bim3D);
                    return Some(SpatialUndoDomain::Bim3D);
                }
                SpatialUndoDomain::Canvas2D if uc_undo_len > 0 => {
                    self.redo_timeline.push(SpatialUndoDomain::Canvas2D);
                    return Some(SpatialUndoDomain::Canvas2D);
                }
                _ => {}
            }
        }
        if bim_undo_len > 0 {
            self.redo_timeline.push(SpatialUndoDomain::Bim3D);
            Some(SpatialUndoDomain::Bim3D)
        } else if uc_undo_len > 0 {
            self.redo_timeline.push(SpatialUndoDomain::Canvas2D);
            Some(SpatialUndoDomain::Canvas2D)
        } else {
            None
        }
    }

    /// Resolves the next domain to redo chronologically (LIFO) across BIM & 2D Canvas.
    pub fn pop_redo_domain(
        &mut self,
        bim_redo_len: usize,
        uc_redo_len: usize,
    ) -> Option<SpatialUndoDomain> {
        while let Some(dom) = self.redo_timeline.pop() {
            match dom {
                SpatialUndoDomain::Bim3D if bim_redo_len > 0 => {
                    self.undo_timeline.push(SpatialUndoDomain::Bim3D);
                    return Some(SpatialUndoDomain::Bim3D);
                }
                SpatialUndoDomain::Canvas2D if uc_redo_len > 0 => {
                    self.undo_timeline.push(SpatialUndoDomain::Canvas2D);
                    return Some(SpatialUndoDomain::Canvas2D);
                }
                _ => {}
            }
        }
        if bim_redo_len > 0 {
            self.undo_timeline.push(SpatialUndoDomain::Bim3D);
            Some(SpatialUndoDomain::Bim3D)
        } else if uc_redo_len > 0 {
            self.undo_timeline.push(SpatialUndoDomain::Canvas2D);
            Some(SpatialUndoDomain::Canvas2D)
        } else {
            None
        }
    }

    /// Resolves sorted active story `(base_level_id, top_level_id, label, count)` for manual BIM placement.
    pub fn active_base_and_top_levels(&self, project: &BimProject) -> (usize, usize, String, usize) {
        let mut lvls = project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        if lvls.is_empty() {
            return (1, 1, "Level 1".to_string(), 1);
        }
        let idx = self.active_story_idx.min(lvls.len().saturating_sub(1));
        let base = &lvls[idx];
        let top = lvls.get(idx + 1).unwrap_or(base);
        (
            base.id,
            top.id,
            format!("{} ({:.1}m)", base.name, base.elevation_mm * 0.001),
            lvls.len(),
        )
    }

    /// Adds a new floor story `3500 mm` above the highest existing BIM level and selects it.
    pub fn add_story_above_top(&mut self, build_view: &mut BuildView) -> Result<String, String> {
        let mut lvls = build_view.bim_bus.project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        let max_elev = lvls.last().map(|l| l.elevation_mm).unwrap_or(0.0);
        let next_num = lvls.len() + 1;
        let msg = build_view.bim_bus.execute(BimCommand::AddLevel {
            name: format!("Level {next_num}"),
            elevation_mm: max_elev + 3500.0,
        })?;
        self.active_story_idx = lvls.len().saturating_sub(1);
        self.push_trace(msg.clone(), true);
        Ok(msg)
    }

    /// Creates a structural concrete slab on the target elevated story spanning project extents.
    pub fn add_slab_at_active_story(&mut self, build_view: &mut BuildView) -> Result<String, String> {
        let mut lvls = build_view.bim_bus.project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        let level_id = if lvls.is_empty() {
            1
        } else {
            let idx = (self.active_story_idx + 1).min(lvls.len() - 1);
            lvls[idx].id
        };
        let (min_x, max_x, min_y, max_y) =
            if let Some((pmin, pmax)) = build_view.bim_bus.project.plan_extents_mm() {
                if (pmax[0] - pmin[0]).abs() < 800.0 || (pmax[1] - pmin[1]).abs() < 800.0 {
                    (-4000.0, 4000.0, -3000.0, 3000.0)
                } else {
                    (pmin[0], pmax[0], pmin[1], pmax[1])
                }
            } else {
                (-4000.0, 4000.0, -3000.0, 3000.0)
            };
        let msg = build_view.bim_bus.execute(BimCommand::AddSlab {
            level_id,
            boundary: vec![
                crate::core::P2(min_x, min_y),
                crate::core::P2(max_x, min_y),
                crate::core::P2(max_x, max_y),
                crate::core::P2(min_x, max_y),
            ],
            openings: Vec::new(),
            thickness_mm: 180.0,
            is_roof: false,
            roof_slope_deg: 0.0,
        })?;
        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
        self.push_trace(msg.clone(), true);
        Ok(msg)
    }

    /// Clones all columns, beams, walls, and slabs on the active floor story one story upward (`+3500 mm`).
    pub fn clone_active_story_upward(&mut self, build_view: &mut BuildView) -> Result<String, String> {
        let (base_id, top_id, story_lbl, _) =
            self.active_base_and_top_levels(&build_view.bim_bus.project);
        let mut lvls = build_view.bim_bus.project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        let top_elev = lvls
            .iter()
            .find(|l| l.id == top_id)
            .map(|l| l.elevation_mm)
            .unwrap_or(3500.0);

        let next_top_id = if let Some(existing) = lvls.iter().find(|l| l.elevation_mm > top_elev + 500.0)
        {
            existing.id
        } else {
            let next_num = lvls.len() + 1;
            build_view.bim_bus.execute(BimCommand::AddLevel {
                name: format!("Level {next_num}"),
                elevation_mm: top_elev + 3500.0,
            })?;
            build_view
                .bim_bus
                .project
                .levels
                .iter()
                .max_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm))
                .map(|l| l.id)
                .unwrap_or(top_id + 1)
        };

        let cols: Vec<_> = build_view
            .bim_bus
            .project
            .columns
            .iter()
            .filter(|c| c.base_level_id == base_id)
            .cloned()
            .collect();
        let runs: Vec<_> = build_view
            .bim_bus
            .project
            .runs
            .iter()
            .filter(|r| r.level_id == base_id || r.level_id == top_id)
            .cloned()
            .collect();
        let walls: Vec<_> = build_view
            .bim_bus
            .project
            .walls
            .iter()
            .filter(|w| w.base_level_id == base_id)
            .cloned()
            .collect();
        let slabs: Vec<_> = build_view
            .bim_bus
            .project
            .slabs
            .iter()
            .filter(|s| s.level_id == top_id || s.level_id == base_id)
            .cloned()
            .collect();

        let mut cloned_c = 0usize;
        for c in cols {
            if build_view
                .bim_bus
                .execute(BimCommand::AddColumn {
                    base_level_id: top_id,
                    top_level_id: next_top_id,
                    pos: c.pos,
                    width_mm: c.width_mm,
                    depth_mm: c.depth_mm,
                    is_round: c.is_round,
                    profile_name: c.profile_name,
                })
                .is_ok()
            {
                cloned_c += 1;
            }
        }
        let mut cloned_b = 0usize;
        for r in runs {
            if build_view
                .bim_bus
                .execute(BimCommand::AddBeamRun {
                    level_id: top_id,
                    category: r.category,
                    start: r.start,
                    end: r.end,
                    width_mm: r.width_mm,
                    depth_mm: r.depth_mm,
                    justify: r.justify,
                    profile_name: r.profile_name,
                })
                .is_ok()
            {
                cloned_b += 1;
            }
        }
        let mut cloned_w = 0usize;
        for w in walls {
            if build_view
                .bim_bus
                .execute(BimCommand::AddWall {
                    base_level_id: top_id,
                    start: w.start,
                    end: w.end,
                    thickness_mm: w.thickness_mm,
                    is_structural: w.is_structural,
                })
                .is_ok()
            {
                cloned_w += 1;
            }
        }
        let next_has_slab = build_view
            .bim_bus
            .project
            .slabs
            .iter()
            .any(|s| s.level_id == next_top_id);
        let mut cloned_s = 0usize;
        if !next_has_slab {
            for s in slabs {
                if build_view
                    .bim_bus
                    .execute(BimCommand::AddSlab {
                        level_id: next_top_id,
                        boundary: s.boundary,
                        openings: s.openings,
                        thickness_mm: s.thickness_mm,
                        is_roof: false,
                        roof_slope_deg: 0.0,
                    })
                    .is_ok()
                {
                    cloned_s += 1;
                    break;
                }
            }
        }

        let mut sorted_after = build_view.bim_bus.project.levels.clone();
        sorted_after.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        if let Some(pos_idx) = sorted_after.iter().position(|l| l.id == top_id) {
            self.active_story_idx = pos_idx;
        }
        let msg = format!(
            "Cloned {story_lbl} upward: +{cloned_c}C +{cloned_b}B +{cloned_w}W +{cloned_s}S"
        );
        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
        self.push_trace(msg.clone(), true);
        Ok(msg)
    }

    /// Auto-generates sized isolated pile caps / pad footings beneath every ground-story column
    /// that does not already sit atop an existing footing (`dist <= 400 mm`).
    pub fn auto_generate_missing_footings(
        &mut self,
        build_view: &mut BuildView,
    ) -> Result<String, String> {
        let mut lvls = build_view.bim_bus.project.levels.clone();
        lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        let ground_id = lvls.first().map(|l| l.id).unwrap_or(1);

        let mut cols: Vec<_> = build_view
            .bim_bus
            .project
            .columns
            .iter()
            .filter(|c| c.base_level_id == ground_id)
            .cloned()
            .collect();
        if cols.is_empty() {
            cols = build_view.bim_bus.project.columns.clone();
        }
        if cols.is_empty() {
            return Err("No columns found to generate footings.".to_string());
        }

        let existing_pts: Vec<crate::core::P2> = build_view
            .bim_bus
            .project
            .footings
            .iter()
            .map(|f| f.pos)
            .collect();
        let mut created = 0usize;
        for c in cols {
            let already = existing_pts.iter().any(|p| p.dist(c.pos) <= 400.0);
            if already {
                continue;
            }
            let w_mm = (c.width_mm.max(c.depth_mm) * 3.0).clamp(1200.0, 3600.0);
            let t_mm = (w_mm * 0.25).clamp(450.0, 900.0);
            if build_view
                .bim_bus
                .execute(BimCommand::AddFooting {
                    level_id: ground_id,
                    pos: c.pos,
                    width_mm: w_mm,
                    length_mm: w_mm,
                    thickness_mm: t_mm,
                })
                .is_ok()
            {
                created += 1;
            }
        }
        let msg = if created == 0 {
            "All ground columns already have footings.".to_string()
        } else {
            format!("Generated +{created} subsurface foundation pile caps")
        };
        self.push_trace(msg.clone(), true);
        Ok(msg)
    }

    /// 1-click capacity auto-sizing for the currently selected BIM element (`DCR <= 85%` target).
    pub fn auto_size_selected_element(
        &mut self,
        build_view: &mut BuildView,
    ) -> Result<String, String> {
        let Some(target) = self.selected_target else {
            return Err("No BIM element selected for Auto-Size.".to_string());
        };
        match target {
            SmartSelectionTarget::Column(col_id) => {
                let col = build_view
                    .bim_bus
                    .project
                    .columns
                    .iter()
                    .find(|c| c.id == col_id)
                    .cloned()
                    .ok_or_else(|| "Column not found.".to_string())?;
                let nw = ((col.width_mm + 100.0) / 50.0).ceil() * 50.0;
                let nd = ((col.depth_mm + 100.0) / 50.0).ceil() * 50.0;
                let nw = nw.clamp(350.0, 2000.0);
                let nd = nd.clamp(350.0, 2000.0);
                self.edit_width_mm = nw;
                self.edit_depth_mm = nd;
                self.edit_profile_name = format!("RC {nw:.0}x{nd:.0}");
                let msg = build_view.bim_bus.execute(BimCommand::ResizeColumn {
                    column_id: col_id,
                    width_mm: nw,
                    depth_mm: nd,
                    rotation_deg: col.rotation_deg,
                    profile_name: self.edit_profile_name.clone(),
                })?;
                let _ = build_view.bim_bus.execute(BimCommand::ReinforceHost {
                    host_id: col_id,
                    preset: Default::default(),
                });
                self.push_trace(format!("Auto-Sized Column -> {nw:.0}x{nd:.0}mm"), true);
                Ok(msg)
            }
            SmartSelectionTarget::Beam(run_id) => {
                let run = build_view
                    .bim_bus
                    .project
                    .runs
                    .iter()
                    .find(|r| r.id == run_id)
                    .cloned()
                    .ok_or_else(|| "Beam not found.".to_string())?;
                let nw = ((run.width_mm + 50.0) / 50.0).ceil() * 50.0;
                let nd = ((run.depth_mm + 100.0) / 50.0).ceil() * 50.0;
                let nw = nw.clamp(250.0, 1500.0);
                let nd = nd.clamp(400.0, 2500.0);
                self.edit_width_mm = nw;
                self.edit_depth_mm = nd;
                self.edit_profile_name = format!("RC {nw:.0}x{nd:.0}");
                let msg = build_view.bim_bus.execute(BimCommand::ResizeRun {
                    run_id,
                    width_mm: nw,
                    depth_mm: nd,
                    profile_name: self.edit_profile_name.clone(),
                })?;
                let _ = build_view.bim_bus.execute(BimCommand::ReinforceHost {
                    host_id: run_id,
                    preset: Default::default(),
                });
                self.push_trace(format!("Auto-Sized Beam -> {nw:.0}x{nd:.0}mm"), true);
                Ok(msg)
            }
            SmartSelectionTarget::Wall(wall_id) => {
                let wall = build_view
                    .bim_bus
                    .project
                    .walls
                    .iter()
                    .find(|w| w.id == wall_id)
                    .cloned()
                    .ok_or_else(|| "Wall not found.".to_string())?;
                let nt = (((wall.thickness_mm + 50.0) / 50.0).ceil() * 50.0).clamp(200.0, 1000.0);
                self.edit_thickness_mm = nt;
                self.edit_is_structural = true;
                let msg = build_view.bim_bus.execute(BimCommand::ResizeWall {
                    wall_id,
                    thickness_mm: nt,
                    is_structural: true,
                })?;
                self.push_trace(format!("Auto-Sized Shear Wall -> t={nt:.0}mm"), true);
                Ok(msg)
            }
            SmartSelectionTarget::Footing(foot_id) => {
                let ft = build_view
                    .bim_bus
                    .project
                    .footings
                    .iter()
                    .find(|f| f.id == foot_id)
                    .cloned()
                    .ok_or_else(|| "Footing not found.".to_string())?;
                let nw = (((ft.width_mm + 400.0) / 100.0).ceil() * 100.0).clamp(1200.0, 6000.0);
                let nl = (((ft.length_mm + 400.0) / 100.0).ceil() * 100.0).clamp(1200.0, 6000.0);
                let nt = (((ft.thickness_mm + 100.0) / 50.0).ceil() * 50.0).clamp(450.0, 2000.0);
                self.edit_width_mm = nw;
                self.edit_length_mm = nl;
                self.edit_thickness_mm = nt;
                let msg = build_view.bim_bus.execute(BimCommand::ResizeFooting {
                    footing_id: foot_id,
                    width_mm: nw,
                    length_mm: nl,
                    thickness_mm: nt,
                })?;
                self.push_trace(
                    format!("Auto-Sized Footing -> {nw:.0}x{nl:.0}x{nt:.0}mm"),
                    true,
                );
                Ok(msg)
            }
            _ => Err("Auto-Size applies to Column, Beam, Wall, or Footing.".to_string()),
        }
    }

    /// Applies an instant Earth 3D architectural camera orientation preset (`iso`, `top`, `front`).
    pub fn apply_camera_preset(
        &mut self,
        build_view: &mut BuildView,
        universal_canvas: &mut UniversalCanvasState,
        preset: &str,
    ) {
        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
        universal_canvas.globe_mode = true;
        let target = if universal_canvas.ground_origin_anchored {
            universal_canvas.ground_origin_lat_lon
        } else {
            universal_canvas.world_map.center_lat_lon
        };
        let zoom_tgt = universal_canvas.world_map.zoom_level.max(18.2);
        match preset {
            "top" => {
                universal_canvas.earth_world.camera.tilt_deg = 0.0;
                universal_canvas.earth_world.camera.heading_deg = 0.0;
                build_view.cam_yaw = 0.0;
                build_view.cam_pitch = 1.48;
                build_view.perspective = false;
                universal_canvas.earth_world.start_fly_to(
                    &universal_canvas.world_map,
                    target.0,
                    target.1,
                    zoom_tgt,
                    "Earth Top View",
                );
                self.push_trace("Earth 3D Camera: Top Plan (0°)", true);
            }
            "front" => {
                universal_canvas.earth_world.camera.tilt_deg = 64.0;
                universal_canvas.earth_world.camera.heading_deg = 0.0;
                build_view.cam_yaw = 0.0;
                build_view.cam_pitch = 0.04;
                build_view.perspective = false;
                universal_canvas.earth_world.start_fly_to(
                    &universal_canvas.world_map,
                    target.0,
                    target.1,
                    zoom_tgt,
                    "Earth Elevation View",
                );
                self.push_trace("Earth 3D Camera: Low Elevation", true);
            }
            _ => {
                universal_canvas.earth_world.camera.tilt_deg = 42.0;
                universal_canvas.earth_world.camera.heading_deg = 28.0;
                build_view.cam_yaw = 0.68;
                build_view.cam_pitch = 0.44;
                build_view.perspective = true;
                universal_canvas.earth_world.start_fly_to(
                    &universal_canvas.world_map,
                    target.0,
                    target.1,
                    zoom_tgt,
                    "Earth Isometric 3D",
                );
                self.push_trace("Earth 3D Camera: Isometric (42°)", true);
            }
        }
    }

    /// Returns true if any manual 3D authoring, 2D CAD drawing, or geodesic measure tool is currently active.
    pub fn has_active_manual_tool(
        &self,
        build_view: &BuildView,
        universal_canvas: &UniversalCanvasState,
    ) -> bool {
        universal_canvas.earth_bim_author.active_tool != crate::views::universal_canvas::EarthBimTool::None
            || universal_canvas.placing_unlocated_rel_path.is_some()
            || build_view.ground_author_tool != Ground3DAuthorTool::Orbit
            || build_view.pending_3d_start_mm.is_some()
            || universal_canvas.earth_bim_author.pending_start_mm.is_some()
            || universal_canvas.active_tool != UniversalCanvasTool::SelectPan
            || universal_canvas.measure_active
    }

    /// Resets any active manual BIM/2D CAD/Measure tool cleanly back to pointer Orbit / Select-Pan.
    pub fn reset_manual_tools_to_orbit(
        &mut self,
        build_view: &mut BuildView,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
        build_view.pending_3d_start_mm = None;
        universal_canvas.earth_bim_author.active_tool =
            crate::views::universal_canvas::EarthBimTool::None;
        universal_canvas.earth_bim_author.pending_start_mm = None;
        universal_canvas.placing_unlocated_rel_path = None;
        universal_canvas.placing_hover_lat_lon = None;
        universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
        universal_canvas.measure_active = false;
        self.push_trace("Returned to Earth 3D Orbit / Select", true);
    }

    /// Attaches a context source into the AI composer strip without duplicates.
    pub fn attach_context(&mut self, attachment: AiContextAttachment) {
        let key = attachment.dedup_key();
        if let Some(pos) = self
            .attached_contexts
            .iter()
            .position(|a| a.dedup_key() == key)
        {
            self.attached_contexts[pos] = attachment;
        } else {
            self.attached_contexts.push(attachment);
        }
    }

    /// Removes an attached context chip by deduplication key.
    pub fn detach_context(&mut self, key: &str) {
        self.attached_contexts.retain(|a| a.dedup_key() != key);
    }

    /// Attaches a picked or dropped local file directly as AI Context and reveals the panel.
    pub fn attach_local_file_context(&mut self, filename: String, content: &str) {
        let snippet: String = content.chars().take(1600).collect();
        self.attach_context(AiContextAttachment::LocalFile {
            filename: filename.clone(),
            size_bytes: content.len(),
            snippet,
        });
        self.request_focus_command = true;
        self.command_bar_expanded = true;
        self.push_trace(format!("Attached context @{filename}"), true);
    }

    /// Archives the current conversation (if non-empty) and starts a clean Antigravity thread.
    pub fn start_new_conversation(&mut self, ai_assistant: &mut AiAssistantView) {
        if ai_assistant.chat_history.len() > 1 {
            let title = ai_assistant
                .chat_history
                .iter()
                .find(|m| m.is_user)
                .map(|m| {
                    let s: String = m.message.chars().take(34).collect();
                    if m.message.chars().count() > 34 {
                        format!("{s}...")
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| self.conversation_title.clone());
            self.conversation_archives.push(AgentThreadArchive {
                title,
                messages: ai_assistant.chat_history.clone(),
                turn_meta: self.turn_metadata.clone(),
            });
            if self.conversation_archives.len() > 12 {
                let drop_n = self.conversation_archives.len() - 12;
                self.conversation_archives.drain(0..drop_n);
            }
        }
        ai_assistant.chat_history.clear();
        ai_assistant.chat_history.push(crate::models::ChatMessage {
            is_user: false,
            message: "Ready in Antigravity Agent Studio. Add a Vault folder, attach @ context, or ask me to fly Earth, generate BIM frames, or verify FEM/ARI redundancy.".to_string(),
            time: crate::core::web_bridge::unix_timestamp_label(),
        });
        self.turn_metadata.clear();
        self.conversation_title = "New Spatial Thread".to_string();
        self.context_picker_open = false;
        self.history_popover_open = false;
        self.sessions_dirty = true;
        self.push_trace("Started new Antigravity conversation", true);
    }

    /// Serializes Dynamic Island thread archives and active history for `Vault/AI_Context/`.
    pub fn export_sessions_json(&self, ai_assistant: &AiAssistantView) -> String {
        let bundle = DynamicIslandPersistedState {
            conversation_title: self.conversation_title.clone(),
            conversation_archives: self.conversation_archives.clone(),
            active_messages: ai_assistant.chat_history.clone(),
            turn_metadata: self.turn_metadata.clone(),
        };
        serde_json::to_string_pretty(&bundle).unwrap_or_default()
    }

    /// Restores Dynamic Island thread archives and active history from `Vault/AI_Context/`.
    pub fn restore_sessions_json(&mut self, json: &str, ai_assistant: &mut AiAssistantView) -> bool {
        let Ok(bundle) = serde_json::from_str::<DynamicIslandPersistedState>(json) else {
            return false;
        };
        if !bundle.conversation_title.trim().is_empty() {
            self.conversation_title = bundle.conversation_title;
        }
        self.conversation_archives = bundle.conversation_archives;
        if !bundle.active_messages.is_empty() {
            ai_assistant.chat_history = bundle.active_messages;
        }
        self.turn_metadata = bundle.turn_metadata;
        self.sessions_dirty = false;
        true
    }

    /// Pushes a concise Antigravity execution trace badge, capping history to 4 chips.
    pub fn push_trace(&mut self, label: impl Into<String>, ok: bool) {
        self.recent_trace_chips.push(AgentTraceChip {
            label: label.into(),
            ok,
        });
        if self.recent_trace_chips.len() > 4 {
            let drain = self.recent_trace_chips.len() - 4;
            self.recent_trace_chips.drain(0..drain);
        }
    }

    /// Returns true if the pointer currently hovers any floating Antigravity HUD panel.
    pub fn is_pointer_over_hud(&self, ctx: &egui::Context) -> bool {
        if self.active_overlay_pillar.is_some() {
            return true;
        }
        ctx.input(|i| i.pointer.hover_pos())
            .is_some_and(|p| self.hud_rects.iter().any(|r| r.contains(p)))
    }

    /// Pulls fresh pick events from either `UniversalCanvasState` or `BuildView` into `selected_target`.
    pub fn sync_pick_events(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
    ) {
        if universal_canvas.spatial_pick_seq != self.last_uc_pick_seq {
            self.last_uc_pick_seq = universal_canvas.spatial_pick_seq;
            if let Some((kind, id)) = universal_canvas.last_picked_bim_element {
                self.selected_target = Some(Self::target_from_bim(kind, id));
                self.popup_tab = SmartPopupTab::Properties;
            } else if let Some(eid) = universal_canvas.selected_entity_id {
                self.selected_target = Some(SmartSelectionTarget::SketchEntity(eid));
            } else if let Some((lat, lon)) = universal_canvas.last_picked_earth_lat_lon {
                self.selected_target = Some(SmartSelectionTarget::EarthSite { lat, lon });
            } else if !self.popup_pinned {
                self.selected_target = None;
            }
        }

        if build_view.spatial_pick_seq != self.last_build_pick_seq {
            self.last_build_pick_seq = build_view.spatial_pick_seq;
            if let Some((kind, id)) = build_view.last_spatial_bim_pick {
                let target = Self::target_from_bim(kind, id);
                self.selected_target = Some(target);
                universal_canvas.highlighted_bim_element = Some((kind, id));
                self.popup_tab = SmartPopupTab::Properties;
            } else if !self.popup_pinned {
                self.selected_target = None;
                universal_canvas.highlighted_bim_element = None;
            }
        }
    }

    fn target_from_bim(kind: BimElementKind, id: usize) -> SmartSelectionTarget {
        match kind {
            BimElementKind::Column => SmartSelectionTarget::Column(id),
            BimElementKind::Run => SmartSelectionTarget::Beam(id),
            BimElementKind::Wall => SmartSelectionTarget::Wall(id),
            BimElementKind::Footing => SmartSelectionTarget::Footing(id),
            BimElementKind::Slab => SmartSelectionTarget::Slab(id),
        }
    }

    /// Returns a contextual `@Mention` label when an element or Earth location is selected.
    pub fn active_context_mention(&self, project: &BimProject) -> Option<String> {
        match self.selected_target? {
            SmartSelectionTarget::Column(id) => project
                .columns
                .iter()
                .find(|c| c.id == id)
                .map(|c| format!("@{}", c.name)),
            SmartSelectionTarget::Beam(id) => project
                .runs
                .iter()
                .find(|r| r.id == id)
                .map(|r| format!("@{}", r.name)),
            SmartSelectionTarget::Wall(id) => project
                .walls
                .iter()
                .find(|w| w.id == id)
                .map(|w| format!("@{}", w.name)),
            SmartSelectionTarget::Footing(id) => project
                .footings
                .iter()
                .find(|f| f.id == id)
                .map(|f| format!("@{}", f.name)),
            SmartSelectionTarget::Slab(id) => project
                .slabs
                .iter()
                .find(|s| s.id == id)
                .map(|s| format!("@{}", s.name)),
            SmartSelectionTarget::SketchEntity(id) => Some(format!("@Entity#{id}")),
            SmartSelectionTarget::EarthSite { lat, lon } => {
                Some(format!("@Site({lat:.2},{lon:.2})"))
            }
        }
    }
}

#[cfg(test)]
mod tests;
