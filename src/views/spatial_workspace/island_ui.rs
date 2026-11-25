//! Single Unified Morphing Liquid Glass Dynamic Island HUD host, studio tabs, and active tool strips.

use super::*;
use egui::{
    Align, Align2, Area, Color32, CornerRadius, Frame, Id, Layout, Margin, Order, RichText,
    ScrollArea, Stroke, Ui, Vec2,
};

use crate::core::BimCommand;
use crate::icons::{self, AppIcon};
use crate::models::{DatabaseComponent, ProjectItem, StructuralMember};
use crate::theme::MacTheme;
use crate::views::build::Ground3DAuthorTool;
use crate::views::universal_canvas::UniversalCanvasTool;
use crate::views::{AiAssistantView, BuildView, UniversalCanvasState};

impl SpatialWorkspaceState {
    /// Liquid Glass frosted translucent surface frame used for all spatial HUD overlays.
    pub(super) fn antigravity_glass_frame(theme: &MacTheme, corner_r: u8, pad: Margin) -> Frame {
        theme.glass_capsule_frame(corner_r, pad)
    }

    /// Renders a compact frosted glass pill button styled via `MacTheme::glass_chip_style`.
    pub(super) fn glass_pill_button(
        ui: &mut Ui,
        theme: &MacTheme,
        icon: Option<AppIcon>,
        label: impl Into<String>,
        active: bool,
        accent: Color32,
        tooltip: &str,
    ) -> egui::Response {
        let (fg, bg, stroke) = theme.glass_chip_style(active, accent);
        let text = label.into();
        let resp = if let Some(ic) = icon {
            icons::icon_button(
                ui,
                ic,
                text,
                9.6,
                fg,
                bg,
                stroke,
                CornerRadius::same(7),
                Vec2::new(0.0, 20.0),
            )
        } else {
            ui.add(
                egui::Button::new(RichText::new(text).size(9.6).color(fg))
                    .min_size(Vec2::new(0.0, 20.0))
                    .fill(bg)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(7)),
            )
        };
        if tooltip.is_empty() {
            resp
        } else {
            resp.on_hover_text(tooltip)
        }
    }

    /// Renders a width-clamped frosted Liquid Glass sub-pill container whose inner widgets
    /// wrap strictly within `max_w` so mobile viewports never overflow horizontally.
    pub(super) fn glass_subpill_group(
        ui: &mut Ui,
        theme: &MacTheme,
        corner_r: u8,
        pad: Margin,
        max_w: f32,
        add_contents: impl FnOnce(&mut Ui),
    ) {
        let clamped_max = max_w.min(ui.available_width().max(130.0));
        theme.glass_subpill_frame(corner_r, pad).show(ui, |ui| {
            let inner_max = (clamped_max - pad.left as f32 - pad.right as f32 - 4.0).max(130.0);
            ui.set_max_width(inner_max);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(inner_max);
                add_contents(ui);
            });
        });
    }

    /// Places two sub-pill groups side-by-side in equal width-bounded columns when `two_col`
    /// is true (wide screens), and stacks them vertically on narrower screens.
    /// Uses a single callback (`is_left = true`, then `false`) to avoid double-borrow conflicts.
    pub(super) fn width_first_pair(
        ui: &mut Ui,
        deck_w: f32,
        two_col: bool,
        mut render_side: impl FnMut(&mut Ui, bool, f32),
    ) {
        if two_col {
            let col_w = ((deck_w - 8.0) * 0.5).floor().max(220.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.vertical(|ui| {
                    ui.set_width(col_w);
                    ui.set_max_width(col_w);
                    render_side(ui, true, col_w);
                });
                ui.vertical(|ui| {
                    ui.set_width(col_w);
                    ui.set_max_width(col_w);
                    render_side(ui, false, col_w);
                });
            });
        } else {
            render_side(ui, true, deck_w);
            ui.add_space(4.0);
            render_side(ui, false, deck_w);
        }
    }

    pub(super) fn render_dynamic_island_row1_left(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        vault_connected: bool,
        vault_name: &str,
        is_mobile: bool,
        actions: &mut SpatialHudActions,
    ) {
        if vault_connected {
            let lbl = if is_mobile {
                "Vault".to_string()
            } else {
                format!("Vault: {vault_name}")
            };
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Folder),
                lbl,
                self.vault_drawer_open,
                theme.accent_green,
                "Browse Vault folders, notes, & folder switch",
            )
            .clicked()
            {
                self.toggle_exclusive_drawer("vault");
            }
        } else {
            let lbl = if is_mobile {
                "+ Vault"
            } else {
                "+ Add Folder (Vault)"
            };
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Folder),
                lbl,
                true,
                theme.accent_blue,
                "Select local folder to mount as Engineering Vault",
            )
            .clicked()
            {
                actions.add_vault_folder = true;
            }
        }
        let ctx_lbl = if is_mobile { "Context" } else { "Add Context" };
        if Self::glass_pill_button(
            ui,
            theme,
            Some(if self.context_picker_open {
                AppIcon::Close
            } else {
                AppIcon::Plus
            }),
            ctx_lbl,
            self.context_picker_open,
            theme.accent_blue,
            "Attach element, Earth site, BIM, FEM, Vault file, or disk file (@)",
        )
        .clicked()
        {
            self.toggle_exclusive_drawer("context");
        }
    }

    pub(super) fn render_dynamic_island_row1_right(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        is_mobile: bool,
        should_expand: bool,
        reverse_order: bool,
    ) {
        let order = if reverse_order { [4, 3, 2, 1, 0] } else { [0, 1, 2, 3, 4] };
        for slot in order {
            match slot {
                0 => {
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        Some(AppIcon::Bot),
                        self.agent_model.short_label(),
                        false,
                        theme.accent_blue,
                        &format!("AI Model: {} (click to cycle)", self.agent_model.label()),
                    )
                    .clicked()
                    {
                        self.agent_model = self.agent_model.next();
                    }
                }
                1 => {
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        Some(AppIcon::SlidersPanel),
                        self.agent_mode.label(),
                        true,
                        theme.accent_blue,
                        self.agent_mode.description(),
                    )
                    .clicked()
                    {
                        self.agent_mode = self.agent_mode.next();
                    }
                }
                2 => {
                    let (hist_fg, hist_bg, hist_st) =
                        theme.glass_chip_style(self.history_popover_open, theme.accent_blue);
                    if icons::icon_only_button(
                        ui,
                        AppIcon::Clock,
                        hist_fg,
                        hist_bg,
                        hist_st,
                        CornerRadius::same(8),
                        Vec2::splat(22.0),
                        "Conversation History",
                    )
                    .clicked()
                    {
                        self.toggle_exclusive_drawer("history");
                    }
                }
                3 => {
                    if is_mobile
                        && !should_expand
                        && Self::glass_pill_button(
                            ui,
                            theme,
                            Some(AppIcon::Layers),
                            "Deck",
                            true,
                            theme.accent_blue,
                            "Expand Dynamic Island controls for touch devices",
                        )
                        .clicked()
                    {
                        self.mobile_deck_pinned = true;
                    }
                }
                _ => {
                    if should_expand
                        && (is_mobile
                            || self.command_bar_expanded
                            || self.context_picker_open
                            || self.vault_drawer_open
                            || self.mobile_deck_pinned
                            || self.selected_target.is_some())
                        && icons::icon_only_button(
                            ui,
                            AppIcon::Close,
                            theme.text_secondary,
                            Color32::TRANSPARENT,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::splat(22.0),
                            "Collapse Dynamic Island",
                        )
                        .clicked()
                    {
                        self.collapse_dynamic_island();
                    }
                }
            }
        }
    }

    /// Renders all Antigravity floating HUD layers (legacy zero-vault wrapper kept compatible for tests).
    #[allow(clippy::too_many_arguments)]
    pub fn render_hud_overlays(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) -> SpatialHudActions {
        self.render_hud_overlays_with_vault(
            ctx,
            theme,
            universal_canvas,
            build_view,
            ai_assistant,
            members,
            projects,
            db_components,
            &[],
        )
    }
}

/// Host abstraction allowing `EigenGraphApp` to render full Settings, Engineering Module Sheets,
/// Vault Setup, Import Archive, and Shortcuts/Quick-Jump natively inside the single Dynamic Island
/// without detached pop-up windows or overlapping mutable borrows.
pub trait SpatialIslandHost {
    fn island_theme(&self) -> MacTheme;
    #[allow(clippy::type_complexity)]
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
    );
}

pub(super) struct BorrowedIslandHost<'a> {
    pub(super) theme: &'a MacTheme,
    pub(super) universal_canvas: &'a mut UniversalCanvasState,
    pub(super) build_view: &'a mut BuildView,
    pub(super) ai_assistant: &'a mut AiAssistantView,
    pub(super) members: &'a [StructuralMember],
    pub(super) projects: &'a [ProjectItem],
    pub(super) db_components: &'a mut Vec<DatabaseComponent>,
    pub(super) vault_projects: &'a [crate::core::vault::PersonalVaultProjectEntry],
}

impl<'a> SpatialIslandHost for BorrowedIslandHost<'a> {
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
            self.theme,
            self.universal_canvas,
            self.build_view,
            self.ai_assistant,
            self.members,
            self.projects,
            self.db_components,
            self.vault_projects,
        )
    }
}

impl SpatialWorkspaceState {
    /// Renders the Single Unified Floating Dynamic Island HUD over the 100% Full-Canvas workspace
    /// (ZERO side panels, ZERO detached pop-ups, ZERO corner chrome).
    #[allow(clippy::too_many_arguments)]
    pub fn render_hud_overlays_with_vault(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
    ) -> SpatialHudActions {
        let mut host = BorrowedIslandHost {
            theme,
            universal_canvas,
            build_view,
            ai_assistant,
            members,
            projects,
            db_components,
            vault_projects,
        };
        self.render_hud_overlays_with_host(ctx, &mut host, None, |_, _| {})
    }

    /// Renders the Single Unified Dynamic Island HUD with an optional embedded wide panel
    /// (Settings, Engineering Module Sheet, Vault Setup, Import Archive, or Shortcuts/Quick-Jump)
    /// hosted seamlessly inside the exact same morphing Liquid Glass Dynamic Island frame.
    #[allow(clippy::too_many_arguments)]
    pub fn render_hud_overlays_with_host<H, F>(
        &mut self,
        ctx: &egui::Context,
        host: &mut H,
        embedded_panel: Option<(&str, AppIcon, &mut bool)>,
        render_embedded_panel: F,
    ) -> SpatialHudActions
    where
        H: SpatialIslandHost,
        F: FnOnce(&mut Ui, &mut H),
    {
        let mut actions = SpatialHudActions::default();
        self.hud_rects.clear();
        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
        {
            let (_, universal_canvas, build_view, _, _, _, _, _) = host.split_island_refs();
            universal_canvas.globe_mode = true;

            // Keep Earth 3D BIM Authoring Config synchronized with active Story & Element Dimensions
            let (sync_base_id, sync_top_id, _, _) =
                self.active_base_and_top_levels(&build_view.bim_bus.project);
            universal_canvas.earth_bim_author.base_level_id = sync_base_id;
            universal_canvas.earth_bim_author.top_level_id = sync_top_id;
            universal_canvas.earth_bim_author.col_width_mm = build_view.author_col_width_mm;
            universal_canvas.earth_bim_author.col_depth_mm = build_view.author_col_depth_mm;
            universal_canvas.earth_bim_author.beam_width_mm = build_view.author_beam_width_mm;
            universal_canvas.earth_bim_author.beam_depth_mm = build_view.author_beam_depth_mm;
            universal_canvas.earth_bim_author.wall_thickness_mm = build_view.author_wall_thickness_mm;
            universal_canvas.earth_bim_author.footing_size_mm = build_view.author_footing_size_mm;
            universal_canvas.earth_bim_author.erection_day_limit = self.active_4d_day as usize;
            universal_canvas.earth_bim_author.active_tool = match build_view.ground_author_tool {
                Ground3DAuthorTool::Orbit => {
                    if matches!(
                        universal_canvas.earth_bim_author.active_tool,
                        crate::views::universal_canvas::EarthBimTool::Slab
                            | crate::views::universal_canvas::EarthBimTool::Door
                            | crate::views::universal_canvas::EarthBimTool::Window
                            | crate::views::universal_canvas::EarthBimTool::Road
                            | crate::views::universal_canvas::EarthBimTool::Bridge
                            | crate::views::universal_canvas::EarthBimTool::Zone
                    ) {
                        universal_canvas.earth_bim_author.active_tool
                    } else {
                        crate::views::universal_canvas::EarthBimTool::None
                    }
                }
                Ground3DAuthorTool::Column => crate::views::universal_canvas::EarthBimTool::Column,
                Ground3DAuthorTool::Beam => crate::views::universal_canvas::EarthBimTool::Beam,
                Ground3DAuthorTool::Wall => crate::views::universal_canvas::EarthBimTool::Wall,
                Ground3DAuthorTool::Footing => crate::views::universal_canvas::EarthBimTool::Footing,
            };
            if let Some(earth_snap) = universal_canvas.earth_bim_author.hover_snap_mm {
                build_view.hover_3d_ground_snap_mm = Some(earth_snap);
            }
            if let Some(earth_start) = universal_canvas.earth_bim_author.pending_start_mm {
                build_view.pending_3d_start_mm = Some(earth_start);
            }
        }

        let vp = MacTheme::viewport_size(ctx);
        let is_mobile = vp.x < MacTheme::MOBILE_BREAKPOINT;

        // Alt+1..4 Quick Studio Tab Shortcuts (opens & switches Dynamic Island Segmented Deck)
        ctx.input_mut(|i| {
            let tab_hotkey = if i.consume_key(egui::Modifiers::ALT, egui::Key::Num1) {
                Some(DynamicIslandStudioTab::ViewAndLens)
            } else if i.consume_key(egui::Modifiers::ALT, egui::Key::Num2) {
                Some(DynamicIslandStudioTab::BimAndStory)
            } else if i.consume_key(egui::Modifiers::ALT, egui::Key::Num3) {
                Some(DynamicIslandStudioTab::GisAndSite)
            } else if i.consume_key(egui::Modifiers::ALT, egui::Key::Num4) {
                Some(DynamicIslandStudioTab::CadAndExport)
            } else {
                None
            };
            if let Some(tab) = tab_hotkey {
                self.studio_tab = tab;
                self.hover_more_tools_open = tab == DynamicIslandStudioTab::CadAndExport;
                self.command_bar_expanded = true;
            }
        });

        // Unified Morphing Floating Dynamic Island (hosts ALL tools, drawers, Smart Inspector,
        // Settings, Vault Setup, Import, Shortcuts & Engineering Sheets in ONE single glass frame!)
        let island_actions = self.render_dynamic_island_with_host(
            ctx,
            host,
            vp,
            is_mobile,
            embedded_panel,
            render_embedded_panel,
        );
        actions.merge(island_actions);

        actions
    }

    #[allow(clippy::too_many_arguments)]
    /// Renders active lens badges, 4D slider, geodesic measure readouts, and precision manual
    /// drafting controls inside a recessed Liquid Glass sub-pill directly within Dynamic Island.
    fn render_active_lens_and_manual_tool_strip(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        actions: &mut SpatialHudActions,
        cap_w: f32,
    ) {
        let has_active = self.active_lens != SpatialLens::Clean
            || universal_canvas.earth_fem_cache.n1_simulation.is_some()
            || universal_canvas.district_hazard.any_active()
            || universal_canvas.subsurface_ssi.ssi_coupling_enabled
            || universal_canvas.subsurface_ssi.xray_subsurface_visible
            || universal_canvas.ifcx_diff_overlay_active
            || universal_canvas.portfolio_retrofit_config.overlay_visible
            || universal_canvas.last_retrofit_report.is_some()
            || universal_canvas.last_crc_cascade_report.is_some()
            || universal_canvas.replay_async_manager.is_running()
            || universal_canvas.last_ground_motion_replay_report.is_some()
            || build_view.ground_author_tool != Ground3DAuthorTool::Orbit
            || universal_canvas.earth_bim_author.active_tool
                != crate::views::universal_canvas::EarthBimTool::None
            || universal_canvas.placing_unlocated_rel_path.is_some()
            || universal_canvas.active_tool != UniversalCanvasTool::SelectPan
            || universal_canvas.measure_active;
        if !has_active {
            return;
        }
        ui.add_space(5.0);
        Self::glass_subpill_group(ui, theme, 10, Margin::symmetric(7, 3), cap_w, |ui| {
            if let Some(n1) = universal_canvas.earth_fem_cache.n1_simulation.clone() {
                let status_col = if n1.report.is_kinematic_mechanism {
                    theme.accent_red
                } else if n1.report.post_max_dcr >= 0.85 {
                    theme.accent_orange
                } else {
                    theme.accent_green
                };
                icons::pill_badge(
                    ui,
                    AppIcon::Bolt,
                    if n1.report.is_kinematic_mechanism {
                        format!("N-1 {} • MECHANISM", n1.removed_name)
                    } else {
                        format!("N-1 {} • EigenGate {:.2}", n1.removed_name, n1.report.residual_eigengate)
                    },
                    status_col,
                );
                ui.label(
                    RichText::new(format!(
                        "DCR {:.0}%→{:.0}% • Δ {:.1}→{:.1}mm • +{} Crit",
                        n1.report.baseline_max_dcr * 100.0,
                        n1.report.post_max_dcr * 100.0,
                        n1.report.baseline_max_disp_mm,
                        n1.report.post_max_disp_mm,
                        n1.report.newly_critical_fem_ids.len()
                    ))
                    .size(9.3)
                    .monospace()
                    .color(theme.text_primary),
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::CheckCircle),
                    "Restore",
                    false,
                    theme.accent_green,
                    "Restore removed member and return to intact 6-DOF equilibrium",
                )
                .clicked()
                {
                    actions.restore_n1_simulation = true;
                }
            }
            if let Some(rel) = universal_canvas.placing_unlocated_rel_path.clone() {
                let short_n = rel.split('/').next_back().unwrap_or(&rel);
                icons::pill_badge(
                    ui,
                    AppIcon::Target,
                    format!(
                        "Place '{}' • {:.0}°",
                        short_n, universal_canvas.placing_heading_deg
                    ),
                    theme.accent_orange,
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "-15°",
                    false,
                    theme.accent_blue,
                    "Rotate North azimuth -15 deg (Q)",
                )
                .clicked()
                {
                    universal_canvas.placing_heading_deg =
                        (universal_canvas.placing_heading_deg - 15.0).rem_euclid(360.0);
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "+15°",
                    false,
                    theme.accent_blue,
                    "Rotate North azimuth +15 deg (E)",
                )
                .clicked()
                {
                    universal_canvas.placing_heading_deg =
                        (universal_canvas.placing_heading_deg + 15.0).rem_euclid(360.0);
                }
                if let Some(az) = universal_canvas.active_vector_snap_az_deg {
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        format!("Align {az:.0}° [A]"),
                        false,
                        theme.accent_green,
                        "Align building azimuth directly to snapped vector road/parcel boundary",
                    )
                    .clicked()
                    {
                        universal_canvas.placing_heading_deg = az.rem_euclid(360.0);
                    }
                }
                if Self::glass_pill_button(ui, theme, None, "Cancel", false, theme.accent_red, "")
                    .clicked()
                {
                    universal_canvas.placing_unlocated_rel_path = None;
                    universal_canvas.placing_hover_lat_lon = None;
                }
            }
            if self.active_lens != SpatialLens::Clean {
                icons::pill_badge(
                    ui,
                    AppIcon::Target,
                    format!("Lens: {}", self.active_lens.label()),
                    theme.accent_blue,
                );
                if self.active_lens == SpatialLens::Erection4D {
                    ui.spacing_mut().slider_width = if cap_w < 380.0 { 88.0 } else { 140.0 };
                    ui.add(egui::Slider::new(&mut self.active_4d_day, 1..=180).text("Day"));
                    build_view.simulation_4d_day = self.active_4d_day as usize;
                } else if self.active_lens == SpatialLens::DeformedShape {
                    ui.spacing_mut().slider_width = if cap_w < 380.0 { 76.0 } else { 118.0 };
                    ui.add(
                        egui::Slider::new(&mut universal_canvas.earth_bim_author.deform_scale, 1.0..=150.0)
                            .suffix("x")
                            .fixed_decimals(0),
                    );
                    let fc = &universal_canvas.earth_fem_cache;
                    ui.label(
                        RichText::new(format!(
                            "Δ={:.1}mm • Drift={:.2}% • Vb={:.0}kN",
                            fc.max_disp_mm,
                            fc.max_story_drift_ratio * 100.0,
                            fc.seismic_base_shear_kn.max(fc.wind_base_shear_kn)
                        ))
                        .size(9.3)
                        .monospace()
                        .color(theme.text_primary),
                    );
                } else if self.active_lens == SpatialLens::ModalVibration {
                    let mode_cnt = universal_canvas.earth_fem_cache.modal_modes.len().clamp(1, 3);
                    for m_idx in 0..mode_cnt {
                        let act = universal_canvas.earth_bim_author.active_modal_mode == m_idx;
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            format!("M{}", m_idx + 1),
                            act,
                            theme.accent_blue,
                            "Select dynamic natural vibration eigenmode",
                        )
                        .clicked()
                        {
                            universal_canvas.earth_bim_author.active_modal_mode = m_idx;
                        }
                    }
                    let playing = universal_canvas.earth_bim_author.modal_anim_playing;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        if playing { "Pause" } else { "Play" },
                        playing,
                        theme.accent_green,
                        "Toggle live sinusoidal eigenmode animation",
                    )
                    .clicked()
                    {
                        universal_canvas.earth_bim_author.modal_anim_playing = !playing;
                    }
                    ui.spacing_mut().slider_width = if cap_w < 380.0 { 64.0 } else { 96.0 };
                    ui.add(
                        egui::Slider::new(&mut universal_canvas.earth_bim_author.deform_scale, 5.0..=120.0)
                            .suffix("x")
                            .fixed_decimals(0),
                    );
                    let m_slot = universal_canvas
                        .earth_bim_author
                        .active_modal_mode
                        .min(universal_canvas.earth_fem_cache.modal_modes.len().saturating_sub(1));
                    if let Some(m) = universal_canvas.earth_fem_cache.modal_modes.get(m_slot) {
                        ui.label(
                            RichText::new(format!(
                                "T={:.2}s • f={:.2}Hz • Mx={:.0}%",
                                m.period_s, m.frequency_hz, m.mass_participation_ratio_x
                            ))
                            .size(9.3)
                            .monospace()
                            .color(theme.text_primary),
                        );
                    }
                } else if self.active_lens == SpatialLens::TimeHistoryReplay {
                    let playing = universal_canvas.replay_anim_playing;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        if playing { "Pause" } else { "Play" },
                        playing,
                        theme.accent_green,
                        "Toggle live 60 FPS earthquake ground-motion replay",
                    )
                    .clicked()
                    {
                        universal_canvas.replay_anim_playing = !playing;
                    }
                    if let Some(ref rep) = universal_canvas.last_ground_motion_replay_report {
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            "Peak t*",
                            false,
                            theme.accent_orange,
                            "Jump to peak seismic inter-story drift instant",
                        )
                        .clicked()
                        {
                            universal_canvas.replay_cursor_time_s = rep.peak_time_s;
                            universal_canvas.replay_anim_playing = false;
                        }
                        let fr = rep.sample_interpolated(universal_canvas.replay_cursor_time_s);
                        ui.label(
                            RichText::new(format!(
                                "t={:.1}/{:.0}s • Drift {:.2}% • Hinges IO:{} LS:{} CP:{} M:{}",
                                fr.time_s,
                                rep.total_duration_s,
                                fr.roof_drift_ratio * 100.0,
                                fr.count_io,
                                fr.count_ls,
                                fr.count_cp,
                                fr.count_mechanism
                            ))
                            .size(9.2)
                            .monospace()
                            .color(theme.text_primary),
                        );
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "Replay UI",
                        self.ground_motion_drawer_open,
                        theme.accent_blue,
                        "Open Live Earthquake Replay & Async StructGNN-CRC drawer",
                    )
                    .clicked()
                    {
                        self.toggle_exclusive_drawer("ground_motion");
                    }
                } else if self.active_lens == SpatialLens::ShmDigitalTwin {
                    if let Some(ref shm) = universal_canvas.last_shm_twin_report {
                        let [pr, pg, pb] = shm.atc20_placard.badge_color_rgb();
                        ui.label(
                            RichText::new(format!(
                                "ATC-20 {} • f1 {:.2}Hz ({:+.1}%) • MAC {:.2} • K_ret {:.0}%",
                                shm.atc20_placard.short_label(),
                                shm.observed_freq_1_hz,
                                shm.freq_shift_pct,
                                shm.mac_mode_1,
                                shm.global_stiffness_retention_pct
                            ))
                            .size(9.2)
                            .monospace()
                            .color(Color32::from_rgb(pr, pg, pb)),
                        );
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "SHM UI",
                        self.shm_drawer_open,
                        theme.accent_green,
                        "Open Live SHM Digital Twin & ATC-20 Placard drawer",
                    )
                    .clicked()
                    {
                        self.toggle_exclusive_drawer("shm");
                    }
                }
                if universal_canvas.evacuation_config.overlay_visible {
                    if let Some(ref evac) = universal_canvas.last_evacuation_report {
                        let col = if evac.isolated_building_count > 0
                            || evac.network_connectivity_pct < 60.0
                        {
                            theme.accent_red
                        } else if evac.degraded_edges > 0 || evac.blocked_edges > 0 {
                            theme.accent_orange
                        } else {
                            theme.accent_green
                        };
                        ui.label(
                            RichText::new(format!(
                                "Evac Conn {:.0}% • Resp {:.0}/100 • Iso {}",
                                evac.network_connectivity_pct,
                                evac.responder_accessibility_score,
                                evac.isolated_building_count
                            ))
                            .size(9.2)
                            .monospace()
                            .color(col),
                        );
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            "Evac UI",
                            self.evacuation_drawer_open,
                            theme.accent_green,
                            "Open Evacuation Routing & Emergency Lifelines drawer",
                        )
                        .clicked()
                        {
                            self.toggle_exclusive_drawer("evacuation");
                        }
                    }
                }
                if universal_canvas.shm_watchdog.is_streaming()
                    || universal_canvas.shm_watchdog.total_packets_ingested > 0
                {
                    let wd = &universal_canvas.shm_watchdog;
                    let [wr, wg, wb] = wd.alert_level.badge_color_rgb();
                    let mode_tag = if wd.ws_connected { "LIVE WS" } else { "SIM" };
                    ui.label(
                        RichText::new(format!(
                            "{mode_tag} {} • STA/LTA {:.2} • {} pkt",
                            wd.alert_level.short_label(),
                            wd.current_sta_lta,
                            wd.total_packets_ingested
                        ))
                        .size(9.2)
                        .monospace()
                        .color(Color32::from_rgb(wr, wg, wb)),
                    );
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "Watchdog",
                        self.shm_drawer_open && self.shm_drawer_tab == 2,
                        Color32::from_rgb(wr, wg, wb),
                        "Open Live WebSocket SHM Watchdog & Rolling Anomaly Stream",
                    )
                    .clicked()
                    {
                        self.shm_drawer_tab = 2;
                        self.open_shm_drawer();
                    }
                }
                if universal_canvas.collab_state.config.risk_heatmap_visible {
                    let metric_lbl = universal_canvas.collab_state.config.risk_metric.short_label();
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        Some(AppIcon::Flame),
                        format!("Risk Map: {metric_lbl}"),
                        self.collab_drawer_open && self.collab_drawer_tab == 1,
                        theme.accent_red,
                        "Open 3D District Risk Field Heatmap & Hotspot drawer",
                    )
                    .clicked()
                    {
                        self.open_collab_drawer(1);
                    }
                }
            }
            if let crate::core::ground_motion_replay::AsyncReplayJobPhase::Running {
                progress,
                stage_label,
                ..
            } = &universal_canvas.replay_async_manager.phase
            {
                icons::pill_badge(
                    ui,
                    AppIcon::Bolt,
                    format!("Async CRC {:.0}% • {}", progress * 100.0, stage_label),
                    theme.accent_orange,
                );
                ui.ctx().request_repaint();
            }
            if build_view.ground_author_tool != Ground3DAuthorTool::Orbit {
                icons::pill_badge(
                    ui,
                    AppIcon::Wrench,
                    format!("Tool: {}", build_view.ground_author_tool.label()),
                    theme.accent_orange,
                );
                match build_view.ground_author_tool {
                    Ground3DAuthorTool::Column => {
                        ui.label(RichText::new("W:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut build_view.author_col_width_mm)
                                .range(150.0..=2000.0)
                                .speed(25.0)
                                .suffix("mm"),
                        );
                        ui.label(RichText::new("D:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut build_view.author_col_depth_mm)
                                .range(150.0..=2000.0)
                                .speed(25.0)
                                .suffix("mm"),
                        );
                        ui.label(RichText::new("X:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.manual_place_x_mm)
                                .speed(100.0)
                                .suffix("mm"),
                        );
                        ui.label(RichText::new("Y:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.manual_place_y_mm)
                                .speed(100.0)
                                .suffix("mm"),
                        );
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            "Place @(X,Y)",
                            true,
                            theme.accent_green,
                            "Place column at exact (X, Y) mm coordinates",
                        )
                        .clicked()
                        {
                            let (base_id, top_id, _, _) =
                                self.active_base_and_top_levels(&build_view.bim_bus.project);
                            let w = build_view.author_col_width_mm.clamp(150.0, 2000.0);
                            let d = build_view.author_col_depth_mm.clamp(150.0, 2000.0);
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::AddColumn {
                                base_level_id: base_id,
                                top_level_id: top_id,
                                pos: crate::core::P2(
                                    self.manual_place_x_mm,
                                    self.manual_place_y_mm,
                                ),
                                width_mm: w,
                                depth_mm: d,
                                is_round: false,
                                profile_name: format!("C{:.0}x{:.0}", w, d),
                            }) {
                                universal_canvas.ground_origin_anchored = true;
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    Ground3DAuthorTool::Beam => {
                        ui.label(RichText::new("b:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut build_view.author_beam_width_mm)
                                .range(120.0..=1500.0)
                                .speed(25.0)
                                .suffix("mm"),
                        );
                        ui.label(RichText::new("h:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut build_view.author_beam_depth_mm)
                                .range(150.0..=2500.0)
                                .speed(25.0)
                                .suffix("mm"),
                        );
                        if let Some(start_mm) = build_view.pending_3d_start_mm {
                            ui.label(RichText::new("L:").size(9.5).color(theme.text_muted));
                            ui.add(
                                egui::DragValue::new(&mut self.manual_polar_len_m)
                                    .range(0.5..=60.0)
                                    .speed(0.25)
                                    .suffix("m"),
                            );
                            ui.label(RichText::new("∠:").size(9.5).color(theme.text_muted));
                            ui.add(
                                egui::DragValue::new(&mut self.manual_polar_deg)
                                    .range(-180.0..=180.0)
                                    .speed(5.0)
                                    .suffix("°"),
                            );
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                None,
                                "Commit Span",
                                true,
                                theme.accent_green,
                                "",
                            )
                            .clicked()
                            {
                                let rad = self.manual_polar_deg.to_radians();
                                let end_x =
                                    start_mm[0] + self.manual_polar_len_m * 1000.0 * rad.cos();
                                let end_y =
                                    start_mm[1] + self.manual_polar_len_m * 1000.0 * rad.sin();
                                let (base_id, _, _, _) =
                                    self.active_base_and_top_levels(&build_view.bim_bus.project);
                                let bw = build_view.author_beam_width_mm.clamp(120.0, 1500.0);
                                let bd = build_view.author_beam_depth_mm.clamp(150.0, 2500.0);
                                if let Ok(msg) = build_view.bim_bus.execute(
                                    crate::core::BimCommand::AddBeamRun {
                                        level_id: base_id,
                                        category: crate::core::RunCategory::StructuralBeam,
                                        start: crate::core::P2(start_mm[0], start_mm[1]),
                                        end: crate::core::P2(end_x, end_y),
                                        width_mm: bw,
                                        depth_mm: bd,
                                        justify: crate::core::RunJustify::Top,
                                        profile_name: format!("B{bw:.0}x{bd:.0}"),
                                    },
                                ) {
                                    build_view.pending_3d_start_mm = None;
                                    actions.sync_fem_and_earth = true;
                                    self.push_trace(msg, true);
                                }
                            }
                        }
                    }
                    Ground3DAuthorTool::Wall => {
                        ui.label(RichText::new("t:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut build_view.author_wall_thickness_mm)
                                .range(80.0..=1200.0)
                                .speed(10.0)
                                .suffix("mm"),
                        );
                        if let Some(start_mm) = build_view.pending_3d_start_mm {
                            ui.label(RichText::new("L:").size(9.5).color(theme.text_muted));
                            ui.add(
                                egui::DragValue::new(&mut self.manual_polar_len_m)
                                    .range(0.5..=60.0)
                                    .speed(0.25)
                                    .suffix("m"),
                            );
                            ui.label(RichText::new("∠:").size(9.5).color(theme.text_muted));
                            ui.add(
                                egui::DragValue::new(&mut self.manual_polar_deg)
                                    .range(-180.0..=180.0)
                                    .speed(5.0)
                                    .suffix("°"),
                            );
                            if Self::glass_pill_button(
                                ui,
                                theme,
                                None,
                                "Commit Wall",
                                true,
                                theme.accent_green,
                                "",
                            )
                            .clicked()
                            {
                                let rad = self.manual_polar_deg.to_radians();
                                let end_x =
                                    start_mm[0] + self.manual_polar_len_m * 1000.0 * rad.cos();
                                let end_y =
                                    start_mm[1] + self.manual_polar_len_m * 1000.0 * rad.sin();
                                let (base_id, _, _, _) =
                                    self.active_base_and_top_levels(&build_view.bim_bus.project);
                                let wt =
                                    build_view.author_wall_thickness_mm.clamp(80.0, 1200.0);
                                if let Ok(msg) = build_view.bim_bus.execute(
                                    crate::core::BimCommand::AddWall {
                                        base_level_id: base_id,
                                        start: crate::core::P2(start_mm[0], start_mm[1]),
                                        end: crate::core::P2(end_x, end_y),
                                        thickness_mm: wt,
                                        is_structural: true,
                                    },
                                ) {
                                    build_view.pending_3d_start_mm = None;
                                    actions.sync_fem_and_earth = true;
                                    self.push_trace(msg, true);
                                }
                            }
                        }
                    }
                    Ground3DAuthorTool::Footing => {
                        ui.label(RichText::new("Size:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut build_view.author_footing_size_mm)
                                .range(400.0..=6000.0)
                                .speed(50.0)
                                .suffix("mm"),
                        );
                        ui.label(RichText::new("X:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.manual_place_x_mm)
                                .speed(100.0)
                                .suffix("mm"),
                        );
                        ui.label(RichText::new("Y:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.manual_place_y_mm)
                                .speed(100.0)
                                .suffix("mm"),
                        );
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            "Place @(X,Y)",
                            true,
                            theme.accent_green,
                            "",
                        )
                        .clicked()
                        {
                            let (base_id, _, _, _) =
                                self.active_base_and_top_levels(&build_view.bim_bus.project);
                            let fs = build_view.author_footing_size_mm.clamp(400.0, 6000.0);
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::AddFooting {
                                level_id: base_id,
                                pos: crate::core::P2(
                                    self.manual_place_x_mm,
                                    self.manual_place_y_mm,
                                ),
                                width_mm: fs,
                                length_mm: fs,
                                thickness_mm: 450.0,
                            }) {
                                universal_canvas.ground_origin_anchored = true;
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    Ground3DAuthorTool::Orbit => {}
                }
                let (_, _, story_lbl, lvl_cnt) =
                    self.active_base_and_top_levels(&build_view.bim_bus.project);
                let story_compact = if cap_w < 380.0 {
                    format!("L{}", self.active_story_idx + 1)
                } else {
                    format!("Story: {story_lbl}")
                };
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    story_compact,
                    false,
                    theme.accent_blue,
                    "Cycle active base level for placement",
                )
                .clicked()
                {
                    self.active_story_idx = (self.active_story_idx + 1) % lvl_cnt.max(1);
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "+ Story",
                    false,
                    theme.accent_green,
                    "Add upper floor story (+3.5m) via undoable BimCommand",
                )
                .clicked()
                    && self.add_story_above_top(build_view).is_ok()
                {
                    actions.sync_fem_and_earth = true;
                    actions.fit_camera = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "+ Slab",
                    false,
                    theme.accent_green,
                    "Add structural concrete floor slab on active upper level",
                )
                .clicked()
                    && self.add_slab_at_active_story(build_view).is_ok()
                {
                    actions.sync_fem_and_earth = true;
                }
                if let Some([hx, hy]) = build_view.hover_3d_ground_snap_mm {
                    ui.label(
                        RichText::new(format!("@ ({:.2}, {:.2}) m", hx * 0.001, hy * 0.001))
                            .size(9.3)
                            .monospace()
                            .color(theme.accent_blue),
                    );
                }
                ui.label(
                    RichText::new("(Esc: Orbit)")
                        .size(9.2)
                        .color(theme.text_muted),
                );
            } else if matches!(
                universal_canvas.earth_bim_author.active_tool,
                crate::views::universal_canvas::EarthBimTool::Slab
                    | crate::views::universal_canvas::EarthBimTool::Door
                    | crate::views::universal_canvas::EarthBimTool::Window
                    | crate::views::universal_canvas::EarthBimTool::Road
                    | crate::views::universal_canvas::EarthBimTool::Bridge
                    | crate::views::universal_canvas::EarthBimTool::Zone
            ) {
                let (_, _, story_lbl, _) =
                    self.active_base_and_top_levels(&build_view.bim_bus.project);
                let hint = match universal_canvas.earth_bim_author.active_tool {
                    crate::views::universal_canvas::EarthBimTool::Slab => {
                        format!("Tool: Slab ({story_lbl}) — click 2 corners")
                    }
                    crate::views::universal_canvas::EarthBimTool::Door => {
                        "Tool: Door (900x2100mm) — click 3D Wall".to_string()
                    }
                    crate::views::universal_canvas::EarthBimTool::Road => {
                        format!(
                            "Tool: IfcRoad ({}) — click start & end on Earth 3D",
                            universal_canvas.city_scene.active_road_kind.label()
                        )
                    }
                    crate::views::universal_canvas::EarthBimTool::Bridge => {
                        "Tool: IfcBridge (6.5m pier/deck) — click start & end on Earth 3D".to_string()
                    }
                    crate::views::universal_canvas::EarthBimTool::Zone => {
                        format!(
                            "Tool: IfcSpatialZone ({}) — click on Earth 3D",
                            universal_canvas.city_scene.active_zone_kind.label()
                        )
                    }
                    _ => "Tool: Window (1800x1400mm) — click 3D Wall".to_string(),
                };
                icons::pill_badge(ui, AppIcon::Wrench, hint, theme.accent_orange);
                ui.label(
                    RichText::new("(Esc: Orbit)")
                        .size(9.2)
                        .color(theme.text_muted),
                );
            } else if universal_canvas.active_tool != UniversalCanvasTool::SelectPan {
                icons::pill_badge(
                    ui,
                    universal_canvas.active_tool.app_icon(),
                    format!("Tool: {}", universal_canvas.active_tool.short_label()),
                    theme.accent_orange,
                );
                match universal_canvas.active_tool {
                    UniversalCanvasTool::Line => {
                        ui.label(RichText::new("L:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.draw_2d_len_m)
                                .range(0.1..=250.0)
                                .speed(0.25)
                                .suffix("m"),
                        );
                        ui.label(RichText::new("@").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.draw_2d_deg)
                                .range(-180.0..=180.0)
                                .speed(1.0)
                                .suffix("°"),
                        );
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            "Add Line",
                            true,
                            theme.accent_green,
                            "",
                        )
                        .clicked()
                        {
                            let eid = universal_canvas.commit_parametric_2d_line(
                                self.draw_2d_len_m,
                                self.draw_2d_deg,
                                "2D",
                                "Drafting",
                            );
                            self.selected_target = Some(SmartSelectionTarget::SketchEntity(eid));
                            self.push_trace(
                                format!(
                                    "Added 2D Line {:.2}m @ {:.0}°",
                                    self.draw_2d_len_m, self.draw_2d_deg
                                ),
                                true,
                            );
                        }
                    }
                    UniversalCanvasTool::Rectangle => {
                        ui.label(RichText::new("W:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.draw_2d_rect_w_m)
                                .range(0.2..=250.0)
                                .speed(0.25)
                                .suffix("m"),
                        );
                        ui.label(RichText::new("H:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.draw_2d_rect_h_m)
                                .range(0.2..=250.0)
                                .speed(0.25)
                                .suffix("m"),
                        );
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            "Add Rect",
                            true,
                            theme.accent_green,
                            "",
                        )
                        .clicked()
                        {
                            let eid = universal_canvas.commit_parametric_2d_rect(
                                self.draw_2d_rect_w_m,
                                self.draw_2d_rect_h_m,
                                "2D",
                                "Drafting",
                            );
                            self.selected_target = Some(SmartSelectionTarget::SketchEntity(eid));
                            self.push_trace(
                                format!(
                                    "Added 2D Rect {:.2}x{:.2}m",
                                    self.draw_2d_rect_w_m, self.draw_2d_rect_h_m
                                ),
                                true,
                            );
                        }
                    }
                    UniversalCanvasTool::Circle => {
                        ui.label(RichText::new("R:").size(9.5).color(theme.text_muted));
                        ui.add(
                            egui::DragValue::new(&mut self.draw_2d_radius_m)
                                .range(0.1..=150.0)
                                .speed(0.2)
                                .suffix("m"),
                        );
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            "Add Circle",
                            true,
                            theme.accent_green,
                            "",
                        )
                        .clicked()
                        {
                            let eid = universal_canvas.commit_parametric_2d_circle(
                                self.draw_2d_radius_m,
                                "2D",
                                "Drafting",
                            );
                            self.selected_target = Some(SmartSelectionTarget::SketchEntity(eid));
                            self.push_trace(
                                format!("Added 2D Circle R={:.2}m", self.draw_2d_radius_m),
                                true,
                            );
                        }
                    }
                    _ => {}
                }
            }
            if universal_canvas.measure_active {
                let (dist_m, area_opt, pts_n) = universal_canvas.geodesic_measure_summary();
                let measure_lbl = if let Some(area_m2) = area_opt {
                    format!(
                        "Measure: {:.0} m² ({:.2} ha) • {:.1} m",
                        area_m2,
                        area_m2 / 10_000.0,
                        dist_m
                    )
                } else if pts_n >= 2 {
                    if dist_m >= 1000.0 {
                        format!("Measure: {:.2} km ({pts_n} pts)", dist_m / 1000.0)
                    } else {
                        format!("Measure: {dist_m:.1} m ({pts_n} pts)")
                    }
                } else {
                    "Geodesic Measure".to_string()
                };
                icons::pill_badge(
                    ui,
                    AppIcon::DimensionRuler,
                    measure_lbl,
                    theme.accent_green,
                );
                if !universal_canvas.measure_points_lat_lon.is_empty()
                    && Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "Clear",
                        false,
                        theme.accent_orange,
                        "",
                    )
                    .clicked()
                {
                    universal_canvas.clear_measure();
                    self.push_trace("Cleared geodesic measurement points", true);
                }
                ui.label(
                    RichText::new(format!(
                        "@ {:.3}°, {:.3}°",
                        universal_canvas.cursor_lat_lon.0, universal_canvas.cursor_lat_lon.1
                    ))
                    .size(9.2)
                    .monospace()
                    .color(theme.text_muted),
                );
            } else if universal_canvas.active_tool.is_earth_tool() {
                ui.label(
                    RichText::new(format!(
                        "@ {:.3}°, {:.3}°",
                        universal_canvas.cursor_lat_lon.0, universal_canvas.cursor_lat_lon.1
                    ))
                    .size(9.2)
                    .monospace()
                    .color(theme.text_muted),
                );
            }
            if universal_canvas.district_hazard.any_active() {
                let rep = &universal_canvas.district_hazard_report;
                let res_col = if rep.resilience_score >= 75.0 {
                    theme.accent_green
                } else if rep.resilience_score >= 45.0 {
                    theme.accent_orange
                } else {
                    theme.accent_red
                };
                icons::pill_badge(
                    ui,
                    AppIcon::AlertTriangle,
                    format!(
                        "Resilience {:.0}/100 • Sds {:.2}g • Wind {:.0}m/s • Flood {:.0}%",
                        rep.resilience_score,
                        rep.spectral_sds_g,
                        rep.effective_wind_ms,
                        rep.flood.inundated_area_pct
                    ),
                    res_col,
                );
            }
            if let Some(ref rep) = universal_canvas.last_retrofit_report {
                let badge_col = if universal_canvas.viewing_pre_retrofit_baseline {
                    theme.accent_orange
                } else {
                    theme.accent_green
                };
                icons::pill_badge(
                    ui,
                    AppIcon::Shield,
                    format!(
                        "Retrofit: EigenGate {:.2}→{:.2} • DCR {:.0}%→{:.0}% • +{} BRB +{} Jkt • ${:.1}k",
                        rep.before_min_eigengate,
                        rep.after_min_eigengate,
                        rep.before_max_dcr_pct,
                        rep.after_max_dcr_pct,
                        rep.added_brb_braces,
                        rep.jacketed_columns,
                        rep.estimated_cost_usd / 1000.0
                    ),
                    badge_col,
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    if universal_canvas.viewing_pre_retrofit_baseline {
                        "View: Before"
                    } else {
                        "View: After"
                    },
                    !universal_canvas.viewing_pre_retrofit_baseline,
                    theme.accent_blue,
                    "Toggle Before vs. After Retrofit 3D structural comparison",
                )
                .clicked()
                {
                    actions.toggle_retrofit_before_after = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Cube3D),
                    "Delta .ifcx",
                    false,
                    theme.accent_green,
                    "Export non-destructive eigengraph::bim::retrofit_delta .ifcx layer",
                )
                .clicked()
                {
                    actions.export_retrofit_delta_ifcx = true;
                }
            }
            if let Some(crc_rep) = universal_canvas.last_crc_cascade_report.clone() {
                let total_steps = crc_rep.total_steps.max(1);
                let cur_step = universal_canvas.active_cascade_step.clamp(1, total_steps);
                let status_col = match crc_rep.cascade_status {
                    crate::core::cascade::CollapseStatus::ArrestedSafe => theme.accent_green,
                    crate::core::cascade::CollapseStatus::ArrestedPartialCollapse => theme.accent_orange,
                    crate::core::cascade::CollapseStatus::CatastrophicCollapse => theme.accent_red,
                };
                icons::pill_badge(
                    ui,
                    AppIcon::Bolt,
                    format!(
                        "CRC -{:.0}% • Recall {:.0}% • PVI {:.2}",
                        crc_rep.scenario_reduction_pct,
                        crc_rep.safety_recall_pct,
                        crc_rep.pvi
                    ),
                    status_col,
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "◀ Step",
                    false,
                    theme.accent_blue,
                    "Rewind 3D progressive collapse domino animation step",
                )
                .clicked()
                {
                    universal_canvas.active_cascade_step = cur_step.saturating_sub(1).max(1);
                }
                ui.label(
                    RichText::new(format!("Step {cur_step}/{total_steps}"))
                        .size(9.3)
                        .monospace()
                        .color(theme.text_primary),
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Step ▶",
                    false,
                    theme.accent_orange,
                    "Advance 3D progressive collapse domino propagation step",
                )
                .clicked()
                {
                    universal_canvas.active_cascade_step = (cur_step + 1).min(total_steps);
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Cube3D),
                    "CRC .ifcx",
                    false,
                    theme.accent_green,
                    "Export non-destructive eigengraph::analysis::crc_cascade .ifcx overlay",
                )
                .clicked()
                {
                    actions.export_crc_cascade_ifcx = true;
                }
            }
            if universal_canvas.subsurface_ssi.ssi_coupling_enabled
                || universal_canvas.subsurface_ssi.xray_subsurface_visible
            {
                if let Some(ref rep) = universal_canvas.last_subsurface_ssi_report {
                    let ssi_col = if rep.is_liquefaction_critical {
                        theme.accent_red
                    } else if rep.period_lengthening_pct >= 15.0 {
                        theme.accent_orange
                    } else {
                        theme.accent_green
                    };
                    icons::pill_badge(
                        ui,
                        AppIcon::Layers,
                        format!(
                            "SSI {:?} • Vs30 {:.0}m/s • T̃ {:.2}s (+{:.0}%)",
                            rep.site_class,
                            rep.vs30_m_s,
                            rep.ssi_coupled_period_t1_s,
                            rep.period_lengthening_pct
                        ),
                        ssi_col,
                    );
                    ui.label(
                        RichText::new(format!(
                            "Lpile {:.0}m • FS_L {:.2} • δ {:.1}mm",
                            rep.recommended_pile_depth_m,
                            rep.min_liquefaction_fs,
                            rep.max_foundation_settlement_mm
                        ))
                        .size(9.3)
                        .monospace()
                        .color(theme.text_primary),
                    );
                } else {
                    icons::pill_badge(
                        ui,
                        AppIcon::Layers,
                        format!(
                            "SSI • {}",
                            universal_canvas.subsurface_ssi.borehole_preset.short_label()
                        ),
                        theme.accent_blue,
                    );
                }
                if universal_canvas.subsurface_ssi.xray_subsurface_visible {
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "GWT -0.5m",
                        false,
                        theme.text_secondary,
                        "Raise groundwater table depth by 0.5m closer to ground surface",
                    )
                    .clicked()
                    {
                        universal_canvas.subsurface_ssi.groundwater_depth_m =
                            (universal_canvas.subsurface_ssi.groundwater_depth_m - 0.5)
                                .clamp(0.5, 15.0);
                        actions.run_subsurface_ssi_sync = true;
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "GWT +0.5m",
                        false,
                        theme.text_secondary,
                        "Lower groundwater table depth by 0.5m",
                    )
                    .clicked()
                    {
                        universal_canvas.subsurface_ssi.groundwater_depth_m =
                            (universal_canvas.subsurface_ssi.groundwater_depth_m + 0.5)
                                .clamp(0.5, 15.0);
                        actions.run_subsurface_ssi_sync = true;
                    }
                }
            }
            if universal_canvas.ifcx_diff_overlay_active {
                let diff_badge = if let Some(ref d) = universal_canvas.last_ifcx_visual_diff {
                    format!("IFCX Diff +{} ~{} -{}", d.added, d.modified, d.deleted)
                } else {
                    "IFCX Diff Ready".to_string()
                };
                icons::pill_badge(ui, AppIcon::Layers, diff_badge, theme.accent_blue);
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Inspect",
                    self.ifcx_diff_drawer_open,
                    theme.accent_blue,
                    "Open Multi-Layer IFCX Visual Diff Inspector inside Dynamic Island",
                )
                .clicked()
                {
                    self.toggle_exclusive_drawer("ifcx_diff");
                    actions.run_ifcx_visual_diff = true;
                }
            }
            if universal_canvas.portfolio_retrofit_config.overlay_visible {
                let pf_badge = if let Some(pt) = universal_canvas
                    .last_portfolio_retrofit_report
                    .as_ref()
                    .and_then(|r| r.active_frontier().map(|p| (r.pre_district_avg_eigengate, p)))
                {
                    let (pre_avg, p) = pt;
                    format!(
                        "Portfolio {}/{} • EigenGate {:.2}→{:.2} • ${:.0}k",
                        p.retrofitted_buildings,
                        p.total_buildings,
                        pre_avg,
                        p.district_avg_eigengate_after,
                        p.total_cost_usd / 1000.0
                    )
                } else {
                    "Portfolio Pareto Ready".to_string()
                };
                icons::pill_badge(ui, AppIcon::ChartLine, pf_badge, theme.accent_green);
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Pareto",
                    self.portfolio_drawer_open,
                    theme.accent_blue,
                    "Open District Portfolio EigenGate Pareto Optimizer inside Dynamic Island",
                )
                .clicked()
                {
                    self.toggle_exclusive_drawer("portfolio");
                    actions.run_portfolio_retrofit = true;
                }
            }
            let (rst_fg, rst_bg, rst_st) = theme.glass_chip_style(false, theme.accent_red);
            if icons::icon_only_button(
                ui,
                AppIcon::Close,
                rst_fg,
                rst_bg,
                rst_st,
                CornerRadius::same(7),
                Vec2::splat(20.0),
                "Reset active lens & manual tool",
            )
            .clicked()
            {
                self.apply_lens_to_views(SpatialLens::Clean, build_view, universal_canvas);
                self.reset_manual_tools_to_orbit(build_view, universal_canvas);
                universal_canvas.district_hazard.seismic_wave_active = false;
                universal_canvas.district_hazard.pounding_overlay_active = false;
                universal_canvas.district_hazard.venturi_wind_active = false;
                universal_canvas.district_hazard.flood_plane_active = false;
                universal_canvas.last_retrofit_report = None;
                universal_canvas.viewing_pre_retrofit_baseline = false;
                universal_canvas.last_crc_cascade_report = None;
                universal_canvas.active_cascade_step = 1;
                universal_canvas.subsurface_ssi.ssi_coupling_enabled = false;
                universal_canvas.subsurface_ssi.xray_subsurface_visible = false;
                universal_canvas.subsurface_ssi.liquefaction_trigger_active = false;
                universal_canvas.last_subsurface_ssi_report = None;
                universal_canvas.ifcx_diff_overlay_active = false;
                self.ifcx_diff_drawer_open = false;
                universal_canvas.portfolio_retrofit_config.overlay_visible = false;
                self.portfolio_drawer_open = false;
                universal_canvas.last_ground_motion_replay_report = None;
                universal_canvas.replay_cursor_time_s = 0.0;
                self.ground_motion_drawer_open = false;
                universal_canvas.last_shm_twin_report = None;
                self.shm_drawer_open = false;
                universal_canvas.evacuation_config.overlay_visible = false;
                universal_canvas.last_evacuation_report = None;
                self.evacuation_drawer_open = false;
                universal_canvas.collab_state.config.risk_heatmap_visible = false;
                self.collab_drawer_open = false;
                actions.sync_fem_and_earth = true;
            }
        });
    }

    /// Renders the single unified Morphing Liquid Glass Dynamic Island at bottom-center:
    /// - Compact Idle Pill when pointer is away on the Earth/BIM canvas (0 side panels, 0 detached pop-ups)
    /// - Smooth Hover-Expanded Studio Deck exposing Vault, Context, AI, Manual Drawing & IFCX City-Builder
    /// - Automatic Wide Expansion when hosting Settings, Engineering Module Sheets, Vault Setup,
    ///   Import Archive, Shortcuts/Quick-Jump, or the Smart Element Inspector inside ONE single capsule.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_dynamic_island_with_host<H, F>(
        &mut self,
        ctx: &egui::Context,
        host: &mut H,
        vp: Vec2,
        is_mobile: bool,
        mut embedded_panel: Option<(&str, AppIcon, &mut bool)>,
        render_embedded_panel: F,
    ) -> SpatialHudActions
    where
        H: SpatialIslandHost,
        F: FnOnce(&mut Ui, &mut H),
    {
        let mut actions = SpatialHudActions::default();
        let slash_mode = self.command_input.trim_start().starts_with('/');
        let vault_status = crate::core::InternalVault::connection_status();
        let has_embedded_panel = embedded_panel.is_some();
        let has_smart_inspector = self.selected_target.is_some();

        let should_expand = has_embedded_panel || self.is_dynamic_island_expanded(ctx);
        self.island_expanded_prev = should_expand;
        let expand_t = ctx.animate_bool(
            Id::new("eigengraph_dynamic_island_expand"),
            should_expand,
        );
        let wide_t = ctx.animate_bool(
            Id::new("eigengraph_dynamic_island_wide"),
            has_embedded_panel
                || has_smart_inspector
                || self.ifcx_diff_drawer_open
                || self.portfolio_drawer_open
                || self.ground_motion_drawer_open
                || self.shm_drawer_open
                || self.evacuation_drawer_open
                || self.collab_drawer_open,
        );

        let compact_w = if is_mobile {
            (vp.x - 16.0).clamp(220.0, 420.0)
        } else {
            (vp.x * 0.48).clamp(500.0, 680.0)
        };
        let expanded_w = if is_mobile {
            (vp.x - 14.0).clamp(220.0, 440.0)
        } else {
            (vp.x - 28.0).clamp(640.0, 1520.0)
        };
        let wide_w = if is_mobile {
            (vp.x - 14.0).clamp(220.0, 440.0)
        } else {
            (vp.x - 24.0).clamp(700.0, 1680.0)
        };
        let base_w = egui::lerp(compact_w..=expanded_w, expand_t);
        let dock_w = egui::lerp(base_w..=wide_w, wide_t).min((vp.x - 14.0).max(220.0));

        let frame_theme = host.island_theme();
        let mut render_embedded_opt = Some(render_embedded_panel);
        let island_left_x = ((vp.x - dock_w) * 0.5).max(6.0);
        let dock_resp = Area::new(Id::new("antigravity_ai_command_dock"))
            .order(Order::Foreground)
            .anchor(Align2::LEFT_BOTTOM, Vec2::new(island_left_x, -14.0))
            .default_width(dock_w)
            .constrain(true)
            .show(ctx, |ui| {
                ui.set_width(dock_w);
                ui.set_max_width(dock_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.vertical(|ui| {
                    // A. Subtle execution trace badge above the resting Dynamic Island
                    if !should_expand && !self.recent_trace_chips.is_empty() && !slash_mode {
                        ui.horizontal_wrapped(|ui| {
                            for chip in self
                                .recent_trace_chips
                                .iter()
                                .rev()
                                .take(if is_mobile { 1 } else { 2 })
                            {
                                let col = if chip.ok {
                                    frame_theme.accent_green
                                } else {
                                    frame_theme.accent_orange
                                };
                                frame_theme
                                    .glass_capsule_frame(12, Margin::symmetric(10, 3))
                                    .stroke(Stroke::new(1.0, col.gamma_multiply(0.55)))
                                    .show(ui, |ui| {
                                        icons::icon_label(
                                            ui,
                                            if chip.ok {
                                                AppIcon::CheckCircle
                                            } else {
                                                AppIcon::AlertTriangle
                                            },
                                            11.0,
                                            col,
                                            &chip.label,
                                            10.0,
                                            frame_theme.text_secondary,
                                            false,
                                        );
                                    });
                            }
                        });
                        ui.add_space(5.0);
                    }

                    // B. ONE UNIFIED GROWING LIQUID GLASS DYNAMIC ISLAND (morphs seamlessly upward & wider when needed)
                    let island_glass = Self::antigravity_glass_frame(&frame_theme, 20, Margin::symmetric(12, 9)).show(
                        ui,
                        |ui| {
                            let cap_w = (dock_w - 30.0).max(180.0);
                            ui.set_width(cap_w);
                            ui.set_max_width(cap_w);

                    // 0. Embedded Full Panel inside Dynamic Island (rendered BEFORE splitting host refs so host has full mutable access!)
                    if let Some((panel_title, panel_icon, ref mut close_panel)) = embedded_panel {
                        let panel_max_h = if is_mobile {
                            (vp.y * 0.46).clamp(160.0, 420.0)
                        } else {
                            (vp.y * 0.64).clamp(210.0, 560.0)
                        };
                        Frame::new()
                            .fill(frame_theme.bg_card.gamma_multiply(0.45))
                            .corner_radius(CornerRadius::same(12))
                            .stroke(Stroke::new(1.0, frame_theme.accent_blue.gamma_multiply(0.38)))
                            .inner_margin(Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                let pw = (cap_w - 34.0).max(160.0);
                                ui.set_width(pw);
                                ui.set_max_width(pw);
                                ui.horizontal(|ui| {
                                    icons::icon_label(
                                        ui,
                                        panel_icon,
                                        15.0,
                                        frame_theme.accent_blue,
                                        panel_title,
                                        13.0,
                                        frame_theme.text_primary,
                                        true,
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if icons::icon_only_button(
                                            ui,
                                            AppIcon::Close,
                                            frame_theme.text_secondary,
                                            Color32::TRANSPARENT,
                                            Stroke::NONE,
                                            CornerRadius::same(6),
                                            Vec2::splat(22.0),
                                            "Close panel (Esc)",
                                        )
                                        .clicked()
                                        {
                                            **close_panel = true;
                                        }
                                    });
                                });
                                ui.separator();
                                ui.add_space(6.0);
                                ScrollArea::vertical()
                                    .id_salt(("dynamic_island_embedded_panel_scroll", panel_title))
                                    .max_width(pw)
                                    .max_height(panel_max_h)
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        ui.set_width(pw);
                                        ui.set_max_width(pw);
                                        if let Some(render_fn) = render_embedded_opt.take() {
                                            render_fn(ui, host);
                                        }
                                    });
                            });
                        ui.add_space(6.0);
                        ui.separator();
                    }

                    let (
                        theme,
                        universal_canvas,
                        build_view,
                        ai_assistant,
                        members,
                        projects,
                        db_components,
                        vault_projects,
                    ) = host.split_island_refs();

                    if should_expand && !has_embedded_panel {
                    let expand_max_h = if is_mobile {
                        (vp.y * 0.42).clamp(150.0, 360.0)
                    } else if has_smart_inspector {
                        (vp.y * 0.56).clamp(190.0, 480.0)
                    } else {
                        (vp.y * 0.52).clamp(170.0, 440.0)
                    };
                    ScrollArea::vertical()
                        .id_salt("dynamic_island_expand_scroll")
                        .max_width(cap_w)
                        .max_height(expand_max_h)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                    ui.set_max_width(cap_w);
                    if let Some(SmartSelectionTarget::EarthSite { lat, lon }) = self.selected_target {
                        let site_actions = self.render_earth_site_inspector_strip(
                            ui,
                            theme,
                            lat,
                            lon,
                            cap_w,
                            universal_canvas,
                            build_view,
                        );
                        actions.merge(site_actions);
                        ui.separator();
                    } else if let Some(target) = self.selected_target {
                        let smart_actions = self.render_embedded_smart_inspector_in_island(
                            ui,
                            theme,
                            target,
                            cap_w,
                            universal_canvas,
                            build_view,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                        actions.merge(smart_actions);
                        ui.add_space(4.0);
                        ui.separator();
                    }
                    if self.history_popover_open {
                        Frame::new()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::symmetric(2, 4))
                            .show(ui, |ui| {
                                let hw = (cap_w - 10.0).max(160.0);
                                ui.set_width(hw);
                                ui.set_max_width(hw);
                                ui.horizontal(|ui| {
                                    icons::icon_label(
                                        ui,
                                        AppIcon::Clock,
                                        12.5,
                                        theme.accent_blue,
                                        "Recent Agent Sessions",
                                        11.0,
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
                                            CornerRadius::same(6),
                                            Vec2::splat(18.0),
                                            "Close history",
                                        )
                                        .clicked()
                                        {
                                            self.history_popover_open = false;
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                                if self.conversation_archives.is_empty() {
                                    ui.label(
                                        RichText::new(
                                            "No archived chats yet. Click + to start a new thread.",
                                        )
                                        .size(10.0)
                                        .color(theme.text_muted),
                                    );
                                } else {
                                    let mut restore_idx = None;
                                    for (idx, thr) in
                                        self.conversation_archives.iter().enumerate().rev()
                                    {
                                        if ui
                                            .button(
                                                RichText::new(format!(
                                                    "{} ({} msgs)",
                                                    thr.title,
                                                    thr.messages.len()
                                                ))
                                                .size(10.5)
                                                .color(theme.text_primary),
                                            )
                                            .clicked()
                                        {
                                            restore_idx = Some(idx);
                                        }
                                    }
                                    if let Some(idx) = restore_idx {
                                        let chosen = self.conversation_archives.remove(idx);
                                        self.conversation_title = chosen.title;
                                        ai_assistant.chat_history = chosen.messages;
                                        self.turn_metadata = chosen.turn_meta;
                                        self.history_popover_open = false;
                                        self.command_bar_expanded = true;
                                    }
                                }
                            });
                        ui.separator();
                    }

                    if self.api_key_popover_open {
                        Frame::new()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::symmetric(2, 4))
                            .show(ui, |ui| {
                                let kw = (cap_w - 10.0).max(160.0);
                                ui.set_width(kw);
                                ui.set_max_width(kw);
                                ui.horizontal(|ui| {
                                    icons::icon_label(
                                        ui,
                                        AppIcon::Lock,
                                        12.5,
                                        theme.accent_blue,
                                        "Gemini API Key (Quick Setup)",
                                        11.0,
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
                                            CornerRadius::same(6),
                                            Vec2::splat(18.0),
                                            "Close API key setup",
                                        )
                                        .clicked()
                                        {
                                            self.api_key_popover_open = false;
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    let key_edit = ui.add(
                                        egui::TextEdit::singleline(&mut self.gemini_api_key_draft)
                                            .password(true)
                                            .hint_text("AIza... (synced with Settings)")
                                            .desired_width((kw - 165.0).max(120.0)),
                                    );
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::CheckCircle,
                                        "Use Gemini 2.5",
                                        10.0,
                                        Color32::WHITE,
                                        theme.accent_green,
                                        Stroke::NONE,
                                        CornerRadius::same(7),
                                        Vec2::new(0.0, 22.0),
                                    )
                                    .clicked()
                                    {
                                        if !self.gemini_api_key_draft.trim().is_empty() {
                                            self.agent_model = AgentModelChoice::Gemini25Flash;
                                            self.push_trace("Gemini 2.5 Flash API Key armed", true);
                                        }
                                        if self.remember_gemini_key {
                                            crate::core::web_bridge::gemini_key_save_device_opt_in(
                                                true,
                                                &self.gemini_api_key_draft,
                                            );
                                        }
                                        self.api_key_popover_open = false;
                                    }
                                    if !self.gemini_api_key_draft.is_empty()
                                        && ui.small_button("Clear").clicked()
                                    {
                                        self.gemini_api_key_draft.clear();
                                        self.agent_model = AgentModelChoice::EigenAiProFem;
                                        crate::core::web_bridge::gemini_key_save_device_opt_in(
                                            false,
                                            "",
                                        );
                                        self.push_trace("Cleared Gemini API key (Offline EigenAI)", true);
                                    }
                                    if self.remember_gemini_key && key_edit.changed() {
                                        crate::core::web_bridge::gemini_key_save_device_opt_in(
                                            true,
                                            &self.gemini_api_key_draft,
                                        );
                                    }
                                });
                                let rem_cb = ui.checkbox(
                                    &mut self.remember_gemini_key,
                                    "Remember key on this device (local storage only, never exported in .eigen)",
                                );
                                if rem_cb.changed() {
                                    crate::core::web_bridge::gemini_key_save_device_opt_in(
                                        self.remember_gemini_key,
                                        &self.gemini_api_key_draft,
                                    );
                                }
                                ui.label(
                                    RichText::new("Used only for live Google Gemini REST calls from Dynamic Island. Empty key automatically falls back to offline EigenAI Pro.")
                                        .size(9.5)
                                        .color(theme.text_muted),
                                );
                            });
                        ui.separator();
                    }

                    if self.ifcx_diff_drawer_open {
                        let diff_opt = universal_canvas.last_ifcx_visual_diff.clone();
                        Frame::new()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::symmetric(2, 4))
                            .show(ui, |ui| {
                                let dw = (cap_w - 10.0).max(160.0);
                                ui.set_width(dw);
                                ui.set_max_width(dw);
                                ui.horizontal(|ui| {
                                    icons::icon_label(
                                        ui,
                                        AppIcon::Layers,
                                        12.5,
                                        theme.accent_blue,
                                        "Multi-Layer IFCX Visual Diff Inspector",
                                        11.0,
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
                                            CornerRadius::same(6),
                                            Vec2::splat(18.0),
                                            "Close IFCX Visual Diff drawer",
                                        )
                                        .clicked()
                                        {
                                            self.ifcx_diff_drawer_open = false;
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        Some(AppIcon::Camera),
                                        "Snapshot Base",
                                        false,
                                        theme.accent_blue,
                                        "Capture current layered .ifcx graph as baseline",
                                    )
                                    .clicked()
                                    {
                                        actions.capture_ifcx_baseline = true;
                                    }
                                    let ov_on = universal_canvas.ifcx_diff_overlay_active;
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        Some(AppIcon::Layers),
                                        "3D Overlay",
                                        ov_on,
                                        theme.accent_green,
                                        "Highlight Added/Modified/Deleted nodes in WebGPU (WGSL) 3D view",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.ifcx_diff_overlay_active = !ov_on;
                                        actions.run_ifcx_visual_diff = true;
                                    }
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        None,
                                        format!("Filter: {}", universal_canvas.ifcx_diff_filter.short_label()),
                                        false,
                                        theme.accent_orange,
                                        "Cycle delta filter (Changes / +Added / ~Modified / -Deleted / All)",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.ifcx_diff_filter =
                                            universal_canvas.ifcx_diff_filter.cycle_next();
                                    }
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        Some(AppIcon::Cube3D),
                                        "Diff .ifcx",
                                        false,
                                        theme.accent_green,
                                        "Export eigengraph::ifcx::diff_manifest .ifcx layer",
                                    )
                                    .clicked()
                                    {
                                        actions.export_ifcx_diff_manifest = true;
                                    }
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        Some(AppIcon::Printer),
                                        "Dossier .svg",
                                        false,
                                        theme.accent_green,
                                        "Export A3 Pure-Vector SVG engineering dossier",
                                    )
                                    .clicked()
                                    {
                                        actions.export_vector_dossier_svg = true;
                                    }
                                });
                                ui.add_space(4.0);
                                if let Some(ref diff) = diff_opt {
                                    ui.horizontal_wrapped(|ui| {
                                        icons::pill_badge(
                                            ui,
                                            AppIcon::Plus,
                                            format!("+{} Added", diff.added),
                                            theme.accent_green,
                                        );
                                        icons::pill_badge(
                                            ui,
                                            AppIcon::Wrench,
                                            format!("~{} Modified", diff.modified),
                                            theme.accent_orange,
                                        );
                                        icons::pill_badge(
                                            ui,
                                            AppIcon::Trash,
                                            format!("-{} Deleted", diff.deleted),
                                            theme.accent_red,
                                        );
                                        icons::pill_badge(
                                            ui,
                                            AppIcon::CheckCircle,
                                            format!("={} Same", diff.unchanged),
                                            theme.text_muted,
                                        );
                                        for (layer, cnt) in &diff.layer_counts {
                                            icons::pill_badge(
                                                ui,
                                                AppIcon::Layers,
                                                format!("{layer}: {cnt}"),
                                                theme.accent_blue,
                                            );
                                        }
                                    });
                                    ui.add_space(4.0);
                                    let filtered = diff.filtered_entries(universal_canvas.ifcx_diff_filter);
                                    if filtered.is_empty() {
                                        ui.label(
                                            RichText::new("No .ifcx deltas match the selected filter (edit BIM or toggle SSI/Retrofit/CRC to see diffs).")
                                                .size(9.5)
                                                .color(theme.text_muted),
                                        );
                                    } else {
                                        ScrollArea::vertical()
                                            .id_salt("ifcx_diff_entries_scroll")
                                            .max_width(dw)
                                            .max_height(155.0)
                                            .auto_shrink([false, true])
                                            .show(ui, |ui| {
                                                for entry in filtered.into_iter().take(40) {
                                                    let (badge, col) = match entry.kind {
                                                        crate::core::ifcx::IfcxDiffKind::Added => {
                                                            ("+", theme.accent_green)
                                                        }
                                                        crate::core::ifcx::IfcxDiffKind::Modified => {
                                                            ("~", theme.accent_orange)
                                                        }
                                                        crate::core::ifcx::IfcxDiffKind::Deleted => {
                                                            ("-", theme.accent_red)
                                                        }
                                                        crate::core::ifcx::IfcxDiffKind::Unchanged => {
                                                            ("=", theme.text_muted)
                                                        }
                                                    };
                                                    let resp = ui.horizontal_wrapped(|ui| {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "[{badge} {} | {}]",
                                                                entry.class_code,
                                                                entry.layer_namespace
                                                            ))
                                                            .size(9.2)
                                                            .strong()
                                                            .color(col),
                                                        );
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "{} ({})",
                                                                entry.path, entry.detail
                                                            ))
                                                            .size(9.0)
                                                            .monospace()
                                                            .color(theme.text_secondary),
                                                        );
                                                    });
                                                    if let Some((kind, id)) = entry.bim_element {
                                                        if resp.response.interact(egui::Sense::click()).clicked() {
                                                            universal_canvas.highlighted_bim_element = Some((kind, id));
                                                            self.push_trace(
                                                                format!("Focused {} #{}", entry.class_code, id),
                                                                true,
                                                            );
                                                        }
                                                    }
                                                }
                                            });
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("No diff report yet — click 'Snapshot Base' or toggle overlays to compare IFCX layers.")
                                            .size(9.5)
                                            .color(theme.text_muted),
                                    );
                                }
                            });
                        ui.separator();
                    }

                    if self.portfolio_drawer_open {
                        let pf_opt = universal_canvas.last_portfolio_retrofit_report.clone();
                        Frame::new()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::symmetric(2, 4))
                            .show(ui, |ui| {
                                let dw = (cap_w - 10.0).max(160.0);
                                ui.set_width(dw);
                                ui.set_max_width(dw);
                                ui.horizontal(|ui| {
                                    icons::icon_label(
                                        ui,
                                        AppIcon::ChartLine,
                                        12.5,
                                        theme.accent_green,
                                        "District Portfolio EigenGate Pareto Optimizer",
                                        11.0,
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
                                            CornerRadius::same(6),
                                            Vec2::splat(18.0),
                                            "Close District Portfolio Pareto drawer",
                                        )
                                        .clicked()
                                        {
                                            self.portfolio_drawer_open = false;
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        None,
                                        "-$50k",
                                        false,
                                        theme.text_secondary,
                                        "Reduce district retrofit budget cap by $50,000",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.portfolio_retrofit_config.budget_cap_usd =
                                            (universal_canvas.portfolio_retrofit_config.budget_cap_usd - 50_000.0)
                                                .clamp(25_000.0, 50_000_000.0);
                                        actions.run_portfolio_retrofit = true;
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "Budget ${:.0}k",
                                            universal_canvas.portfolio_retrofit_config.budget_cap_usd / 1000.0
                                        ))
                                        .size(9.5)
                                        .strong()
                                        .color(theme.text_primary),
                                    );
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        None,
                                        "+$50k",
                                        false,
                                        theme.text_secondary,
                                        "Increase district retrofit budget cap by $50,000",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.portfolio_retrofit_config.budget_cap_usd =
                                            (universal_canvas.portfolio_retrofit_config.budget_cap_usd + 50_000.0)
                                                .clamp(25_000.0, 50_000_000.0);
                                        actions.run_portfolio_retrofit = true;
                                    }
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        None,
                                        "-25t",
                                        false,
                                        theme.text_secondary,
                                        "Reduce embodied carbon cap by 25 tCO2e",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.portfolio_retrofit_config.carbon_cap_kg_co2e =
                                            (universal_canvas.portfolio_retrofit_config.carbon_cap_kg_co2e - 25_000.0)
                                                .clamp(10_000.0, 10_000_000.0);
                                        actions.run_portfolio_retrofit = true;
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "CO2 {:.0}t",
                                            universal_canvas.portfolio_retrofit_config.carbon_cap_kg_co2e / 1000.0
                                        ))
                                        .size(9.5)
                                        .strong()
                                        .color(theme.text_primary),
                                    );
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        None,
                                        "+25t",
                                        false,
                                        theme.text_secondary,
                                        "Increase embodied carbon cap by 25 tCO2e",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.portfolio_retrofit_config.carbon_cap_kg_co2e =
                                            (universal_canvas.portfolio_retrofit_config.carbon_cap_kg_co2e + 25_000.0)
                                                .clamp(10_000.0, 10_000_000.0);
                                        actions.run_portfolio_retrofit = true;
                                    }
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        Some(AppIcon::Target),
                                        format!("Obj: {}", universal_canvas.portfolio_retrofit_config.objective.short_label()),
                                        false,
                                        theme.accent_blue,
                                        "Cycle optimization objective (EigenGate/$ / Balanced / Low-CO2 / Max EigenGate)",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.portfolio_retrofit_config.objective =
                                            universal_canvas.portfolio_retrofit_config.objective.cycle_next();
                                        actions.run_portfolio_retrofit = true;
                                    }
                                    let ov_on = universal_canvas.portfolio_retrofit_config.overlay_visible;
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        Some(AppIcon::Layers),
                                        "3D Overlay",
                                        ov_on,
                                        theme.accent_green,
                                        "Toggle 3D WebGPU (WGSL) district retrofit crown overlays",
                                    )
                                    .clicked()
                                    {
                                        universal_canvas.portfolio_retrofit_config.overlay_visible = !ov_on;
                                        actions.run_portfolio_retrofit = true;
                                    }
                                    if Self::glass_pill_button(
                                        ui,
                                        theme,
                                        Some(AppIcon::Cube3D),
                                        "Portfolio .ifcx",
                                        false,
                                        theme.accent_green,
                                        "Export non-destructive eigengraph::portfolio::retrofit_plan .ifcx",
                                    )
                                    .clicked()
                                    {
                                        actions.export_portfolio_retrofit_ifcx = true;
                                    }
                                });
                                ui.add_space(4.0);
                                if let Some(ref rep) = pf_opt {
                                    // Pure-vector mini Pareto scatter chart (Cost $k vs District Avg EigenGate)
                                    let chart_w = dw.min(440.0);
                                    let chart_h = 92.0_f32;
                                    let (resp, painter) =
                                        ui.allocate_painter(Vec2::new(chart_w, chart_h), egui::Sense::hover());
                                    let r = resp.rect;
                                    painter.rect_filled(
                                        r,
                                        CornerRadius::same(6),
                                        theme.bg_card.gamma_multiply(0.55),
                                    );
                                    let max_cost_k = rep
                                        .pareto_frontier
                                        .iter()
                                        .map(|p| p.total_cost_usd / 1000.0)
                                        .fold(50.0_f64, f64::max)
                                        .max(50.0);
                                    let min_eigengate = rep.pre_district_avg_eigengate.min(0.40);
                                    let max_eigengate = rep
                                        .pareto_frontier
                                        .iter()
                                        .map(|p| p.district_avg_eigengate_after)
                                        .fold(0.85_f64, f64::max)
                                        .max(min_eigengate + 0.10);
                                    let pad_l = 32.0_f32;
                                    let pad_r = 12.0_f32;
                                    let pad_t = 12.0_f32;
                                    let pad_b = 18.0_f32;
                                    let plot_w = (r.width() - pad_l - pad_r).max(40.0);
                                    let plot_h = (r.height() - pad_t - pad_b).max(30.0);
                                    painter.line_segment(
                                        [
                                            egui::Pos2::new(r.left() + pad_l, r.top() + pad_t),
                                            egui::Pos2::new(r.left() + pad_l, r.bottom() - pad_b),
                                        ],
                                        Stroke::new(1.0, theme.border_card),
                                    );
                                    painter.line_segment(
                                        [
                                            egui::Pos2::new(r.left() + pad_l, r.bottom() - pad_b),
                                            egui::Pos2::new(r.right() - pad_r, r.bottom() - pad_b),
                                        ],
                                        Stroke::new(1.0, theme.border_card),
                                    );
                                    painter.text(
                                        egui::Pos2::new(r.left() + pad_l + 4.0, r.top() + 3.0),
                                        Align2::LEFT_TOP,
                                        "Pareto Frontier: Avg EigenGate vs Cost ($k)",
                                        egui::FontId::proportional(9.0),
                                        theme.text_muted,
                                    );
                                    let mut pts_screen = Vec::with_capacity(rep.pareto_frontier.len());
                                    for p in &rep.pareto_frontier {
                                        let nx = ((p.total_cost_usd / 1000.0) / max_cost_k).clamp(0.0, 1.0) as f32;
                                        let ny = ((p.district_avg_eigengate_after - min_eigengate) / (max_eigengate - min_eigengate)).clamp(0.0, 1.0) as f32;
                                        let px = r.left() + pad_l + nx * plot_w;
                                        let py = r.bottom() - pad_b - ny * plot_h;
                                        pts_screen.push(egui::Pos2::new(px, py));
                                    }
                                    for win in pts_screen.windows(2) {
                                        painter.line_segment(
                                            [win[0], win[1]],
                                            Stroke::new(1.5, theme.accent_blue.gamma_multiply(0.8)),
                                        );
                                    }
                                    for (idx, sp) in pts_screen.iter().enumerate() {
                                        let is_sel = idx == rep.selected_frontier_idx;
                                        let p = &rep.pareto_frontier[idx];
                                        let col = if p.total_cost_usd <= rep.config.budget_cap_usd + 1.0 {
                                            theme.accent_green
                                        } else {
                                            theme.accent_orange
                                        };
                                        painter.circle_filled(*sp, if is_sel { 5.0 } else { 3.5 }, col);
                                        if is_sel {
                                            painter.circle_stroke(*sp, 7.5, Stroke::new(1.4, theme.text_primary));
                                        }
                                    }
                                    ui.add_space(4.0);
                                    ui.horizontal_wrapped(|ui| {
                                        for (idx, pt) in rep.pareto_frontier.iter().enumerate() {
                                            let active = idx == rep.selected_frontier_idx;
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                format!(
                                                    "F{} {} (${:.0}k | EigenGate {:.2})",
                                                    pt.frontier_id,
                                                    pt.label,
                                                    pt.total_cost_usd / 1000.0,
                                                    pt.district_avg_eigengate_after
                                                ),
                                                active,
                                                theme.accent_green,
                                                "Select Pareto frontier allocation point",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas.portfolio_retrofit_config.active_frontier_index = idx;
                                                if let Some(ref mut rep_mut) =
                                                    universal_canvas.last_portfolio_retrofit_report
                                                {
                                                    rep_mut.selected_frontier_idx = idx;
                                                }
                                            }
                                        }
                                    });
                                    ui.add_space(4.0);
                                    if let Some(active_pt) = rep.active_frontier() {
                                        ScrollArea::vertical()
                                            .id_salt("portfolio_decisions_scroll")
                                            .max_width(dw)
                                            .max_height(130.0)
                                            .auto_shrink([false, true])
                                            .show(ui, |ui| {
                                                for d in &active_pt.decisions {
                                                    ui.horizontal_wrapped(|ui| {
                                                        let (badge, col) = if let Some(s) = d.selected_strategy {
                                                            (s.short_label(), theme.accent_green)
                                                        } else if d.pre_min_eigengate < rep.config.target_min_eigengate {
                                                            ("Deferred", theme.accent_red)
                                                        } else {
                                                            ("Safe", theme.text_muted)
                                                        };
                                                        icons::pill_badge(
                                                            ui,
                                                            AppIcon::Building,
                                                            format!("#{} {} [{}]", d.priority_rank, d.name, badge),
                                                            col,
                                                        );
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "EigenGate {:.2}->{:.2} | ${:.0}k | {:.1}t",
                                                                d.pre_min_eigengate,
                                                                d.post_min_eigengate,
                                                                d.estimated_cost_usd / 1000.0,
                                                                d.embodied_carbon_kg_co2e / 1000.0
                                                            ))
                                                            .size(9.2)
                                                            .monospace()
                                                            .color(theme.text_secondary),
                                                        );
                                                        if Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            Some(AppIcon::Target),
                                                            "Fly",
                                                            false,
                                                            theme.accent_blue,
                                                            "Fly Earth camera directly to building parcel",
                                                        )
                                                        .clicked()
                                                        {
                                                            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                                                            universal_canvas.globe_mode = true;
                                                            universal_canvas.earth_world.start_fly_to(
                                                                &universal_canvas.world_map,
                                                                d.lat,
                                                                d.lon,
                                                                17.5,
                                                                &d.name,
                                                            );
                                                            self.push_trace(
                                                                format!("Flying Earth -> {}", d.name),
                                                                true,
                                                            );
                                                        }
                                                    });
                                                }
                                            });
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("Click Budget/Obj controls or run /portfolio to synthesize district Pareto frontier.")
                                            .size(9.5)
                                            .color(theme.text_muted),
                                    );
                                }
                            });
                        ui.separator();
                    }

                    if self.ground_motion_drawer_open {
                        self.render_ground_motion_replay_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            build_view,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.shm_drawer_open {
                        self.render_shm_digital_twin_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            build_view,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.evacuation_drawer_open {
                        self.render_evacuation_lifelines_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.collab_drawer_open {
                        self.render_collab_and_risk_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.field_qaqc_drawer_open {
                        self.render_field_qaqc_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.eigenflow_drawer_open {
                        self.render_eigenflow_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.extreme_events_drawer_open {
                        self.render_extreme_events_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.isolation_drawer_open {
                        self.render_isolation_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.wind_drawer_open {
                        self.render_wind_drawer(
                            ui,
                            theme,
                            universal_canvas,
                            &mut actions,
                            cap_w,
                            is_mobile,
                        );
                        ui.separator();
                    }

                    if self.vault_drawer_open || self.vault_path_popover_open {
                        let vault_act = self.render_vault_workspace_bar(
                            ui,
                            theme,
                            vault_projects,
                            universal_canvas,
                            cap_w,
                        );
                        actions.merge(vault_act);
                        ui.separator();
                    }

                    if slash_mode {
                        let sub = self.render_slash_palette_box(
                            ui,
                            theme,
                            universal_canvas,
                            build_view,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                            (cap_w - 4.0).max(170.0),
                            (vp.y * 0.30).clamp(140.0, 230.0),
                            "antigravity_slash_palette_scroll",
                        );
                        actions.merge(sub);
                        ui.separator();
                    } else if self.context_picker_open {
                        let sub = self.render_context_picker_popover(
                            ui,
                            theme,
                            &build_view.bim_bus.project,
                            vault_projects,
                            cap_w,
                        );
                        actions.merge(sub);
                        ui.separator();
                    }

                    if self.command_bar_expanded && !slash_mode {
                        let chat_h = (vp.y * if is_mobile { 0.22 } else { 0.25 }).clamp(90.0, 210.0);
                        Frame::new()
                            .fill(Color32::TRANSPARENT)
                            .inner_margin(Margin::symmetric(2, 4))
                            .show(ui, |ui| {
                            let chat_w = (cap_w - 10.0).max(170.0);
                            ui.set_width(chat_w);
                            ui.set_max_width(chat_w);
                            ui.horizontal(|ui| {
                                icons::icon_label(
                                    ui,
                                    AppIcon::Bot,
                                    14.0,
                                    theme.accent_blue,
                                    &self.conversation_title,
                                    12.0,
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
                                        CornerRadius::same(6),
                                        Vec2::splat(20.0),
                                        "Collapse Chat Drawer",
                                    )
                                    .clicked()
                                    {
                                        self.command_bar_expanded = false;
                                    }
                                    if icons::icon_button(
                                        ui,
                                        AppIcon::Wrench,
                                        "Auto-Repair EigenGate",
                                        9.8,
                                        Color32::WHITE,
                                        theme.accent_green,
                                        Stroke::NONE,
                                        CornerRadius::same(6),
                                        Vec2::new(0.0, 20.0),
                                    )
                                    .clicked()
                                    {
                                        actions.run_auto_repair = true;
                                        self.push_trace("Auto-Repaired Critical Members", true);
                                    }
                                    if icons::icon_only_button(
                                        ui,
                                        AppIcon::Plus,
                                        theme.text_primary,
                                        theme.bg_card,
                                        Stroke::new(0.8, theme.border_subtle),
                                        CornerRadius::same(6),
                                        Vec2::splat(20.0),
                                        "New Chat",
                                    )
                                    .clicked()
                                    {
                                        self.start_new_conversation(ai_assistant);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                            ScrollArea::vertical()
                                .id_salt("antigravity_chat_scroll")
                                .max_height(chat_h)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    let inner_w = (chat_w - 14.0).max(160.0);
                                    ui.set_max_width(inner_w);
                                    for (idx, msg) in
                                        ai_assistant.chat_history.iter().enumerate()
                                    {
                                        let meta =
                                            self.turn_metadata.iter().find(|m| m.msg_index == idx);
                                        if msg.is_user {
                                            Frame::new()
                                                .fill(if theme.is_dark() {
                                                    Color32::from_rgb(33, 42, 60)
                                                } else {
                                                    theme.accent_blue.gamma_multiply(0.12)
                                                })
                                                .corner_radius(CornerRadius::same(10))
                                                .stroke(Stroke::new(
                                                    1.0,
                                                    theme.accent_blue.gamma_multiply(0.32),
                                                ))
                                                .inner_margin(Margin::symmetric(10, 6))
                                                .show(ui, |ui| {
                                                    ui.set_width(
                                                        (inner_w - 14.0).max(140.0),
                                                    );
                                                    if let Some(m) = meta {
                                                        if !m.context_chips.is_empty() {
                                                            ui.horizontal_wrapped(|ui| {
                                                                for chip in &m.context_chips {
                                                                    icons::pill_badge(
                                                                        ui,
                                                                        AppIcon::Target,
                                                                        chip.clone(),
                                                                        theme.accent_blue,
                                                                    );
                                                                }
                                                            });
                                                            ui.add_space(3.0);
                                                        }
                                                    }
                                                    ui.label(
                                                        RichText::new(&msg.message)
                                                            .size(11.2)
                                                            .color(theme.text_primary),
                                                    );
                                                });
                                        } else {
                                            Frame::new()
                                                .fill(theme.bg_card)
                                                .corner_radius(CornerRadius::same(10))
                                                .stroke(Stroke::new(1.0, theme.border_subtle))
                                                .inner_margin(Margin::symmetric(10, 6))
                                                .show(ui, |ui| {
                                                    ui.set_width(
                                                        (inner_w - 14.0).max(140.0),
                                                    );
                                                    if let Some(m) = meta {
                                                        if !m.trace_steps.is_empty() {
                                                            ui.horizontal_wrapped(|ui| {
                                                                for st in &m.trace_steps {
                                                                    icons::pill_badge(
                                                                        ui,
                                                                        AppIcon::CheckCircle,
                                                                        st.clone(),
                                                                        theme.accent_green,
                                                                    );
                                                                }
                                                            });
                                                            ui.add_space(3.0);
                                                        }
                                                    }
                                                    ui.label(
                                                        RichText::new(&msg.message)
                                                            .size(11.0)
                                                            .color(theme.text_primary),
                                                    );
                                                    if idx > 0 {
                                                        ui.horizontal(|ui| {
                                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                                if ui
                                                                    .small_button("Save Note")
                                                                    .on_hover_text("Save response to Obsidian Vault KnowledgeBase/*.md")
                                                                    .clicked()
                                                                {
                                                                    let title = format!("AI_Audit_{idx}");
                                                                    if let Ok(rel) = crate::core::InternalVault::save_markdown_note(&title, &msg.message) {
                                                                        self.vault_feedback = Some(format!("Saved AI reply -> {rel}"));
                                                                        actions.refresh_vault = true;
                                                                    }
                                                                }
                                                            });
                                                        });
                                                    }
                                                });
                                        }
                                        ui.add_space(4.0);
                                    }

                                    if self.gemini_inflight_req_id.is_some() {
                                        Frame::new()
                                            .fill(theme.accent_blue.gamma_multiply(0.12))
                                            .corner_radius(CornerRadius::same(9))
                                            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.4)))
                                            .inner_margin(Margin::symmetric(10, 6))
                                            .show(ui, |ui| {
                                                ui.set_width((inner_w - 14.0).max(140.0));
                                                ui.horizontal(|ui| {
                                                    icons::icon_label(
                                                        ui,
                                                        AppIcon::Sparkles,
                                                        12.0,
                                                        theme.accent_blue,
                                                        format!("{} is thinking...", self.agent_model.short_label()),
                                                        10.8,
                                                        theme.accent_blue,
                                                        true,
                                                    );
                                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                        if ui.small_button("Cancel").clicked() {
                                                            self.gemini_inflight_req_id = None;
                                                            self.push_trace("Cancelled Gemini request", false);
                                                        }
                                                    });
                                                });
                                            });
                                        ui.add_space(4.0);
                                    }

                                    let mut apply_id: Option<u64> = None;
                                    let mut reject_id: Option<u64> = None;
                                    for prop in &self.pending_proposals {
                                        let border_col = match prop.status {
                                            ProposalStatus::Pending => theme.accent_orange.gamma_multiply(0.65),
                                            ProposalStatus::Applied => theme.accent_green.gamma_multiply(0.6),
                                            ProposalStatus::Rejected => theme.border_subtle,
                                        };
                                        Frame::new()
                                            .fill(theme.bg_sidebar)
                                            .corner_radius(CornerRadius::same(10))
                                            .stroke(Stroke::new(1.0, border_col))
                                            .inner_margin(Margin::symmetric(10, 7))
                                            .show(ui, |ui| {
                                                ui.set_width((inner_w - 14.0).max(140.0));
                                                ui.horizontal_wrapped(|ui| {
                                                    icons::icon_label(
                                                        ui,
                                                        AppIcon::Wrench,
                                                        12.0,
                                                        theme.accent_orange,
                                                        &prop.title,
                                                        10.8,
                                                        theme.text_primary,
                                                        true,
                                                    );
                                                    match prop.status {
                                                        ProposalStatus::Pending => icons::pill_badge(
                                                            ui,
                                                            AppIcon::Clock,
                                                            "PLANNING REVIEW",
                                                            theme.accent_orange,
                                                        ),
                                                        ProposalStatus::Applied => icons::pill_badge(
                                                            ui,
                                                            AppIcon::CheckCircle,
                                                            "APPLIED",
                                                            theme.accent_green,
                                                        ),
                                                        ProposalStatus::Rejected => icons::pill_badge(
                                                            ui,
                                                            AppIcon::Close,
                                                            "REJECTED",
                                                            theme.text_muted,
                                                        ),
                                                    }
                                                });
                                                ui.label(
                                                    RichText::new(&prop.rationale)
                                                        .size(10.0)
                                                        .color(theme.text_secondary),
                                                );
                                                ui.label(
                                                    RichText::new(&prop.action_preview)
                                                        .size(9.8)
                                                        .monospace()
                                                        .color(theme.accent_blue),
                                                );
                                                if prop.status == ProposalStatus::Pending {
                                                    ui.add_space(4.0);
                                                    ui.horizontal_wrapped(|ui| {
                                                        if icons::icon_button(
                                                            ui,
                                                            AppIcon::CheckCircle,
                                                            "Apply Changes",
                                                            10.0,
                                                            Color32::WHITE,
                                                            theme.accent_green,
                                                            Stroke::NONE,
                                                            CornerRadius::same(7),
                                                            Vec2::new(0.0, 22.0),
                                                        )
                                                        .clicked()
                                                        {
                                                            apply_id = Some(prop.id);
                                                        }
                                                        if icons::icon_button(
                                                            ui,
                                                            AppIcon::Close,
                                                            "Reject",
                                                            10.0,
                                                            theme.text_secondary,
                                                            theme.bg_card,
                                                            Stroke::new(0.9, theme.border_subtle),
                                                            CornerRadius::same(7),
                                                            Vec2::new(0.0, 22.0),
                                                        )
                                                        .clicked()
                                                        {
                                                            reject_id = Some(prop.id);
                                                        }
                                                    });
                                                }
                                            });
                                        ui.add_space(4.0);
                                    }
                                    if let Some(pid) = apply_id {
                                        let sub = self.apply_proposal_by_id(
                                            pid,
                                            universal_canvas,
                                            build_view,
                                            ai_assistant,
                                        );
                                        actions.merge(sub);
                                    }
                                    if let Some(pid) = reject_id {
                                        self.reject_proposal_by_id(pid, ai_assistant);
                                    }
                                });
                        });
                        ui.separator();
                    }

                    // Liquid Glass 4-Tab Segmented Hover Studio Deck inside ONE continuous Dynamic Island
                    Frame::new()
                        .fill(Color32::TRANSPARENT)
                        .inner_margin(Margin::symmetric(2, 4))
                        .show(ui, |ui| {
                            let deck_w = (cap_w - 10.0).max(170.0);
                            ui.set_width(deck_w);
                            ui.set_max_width(deck_w);

                            let world_total = universal_canvas.world_index.buildings.len();
                            let world_unloc = universal_canvas.world_index.unlocated_count();
                            let world_crit = universal_canvas
                                .world_index
                                .buildings
                                .iter()
                                .filter(|b| b.is_critical())
                                .count();
                            let (worst_target, max_dcr, min_eigengate, crit_cnt) =
                                self.structural_health_summary_cached(
                                    &build_view.bim_bus.project,
                                    members,
                                    &universal_canvas.earth_fem_cache,
                                );
                            let health_col = if max_dcr > 95.0 || min_eigengate < 0.65 {
                                theme.accent_red
                            } else if max_dcr > 75.0 || min_eigengate < 0.78 {
                                theme.accent_orange
                            } else {
                                theme.accent_green
                            };
                            let can_undo = !build_view.bim_bus.undo_stack.is_empty()
                                || !universal_canvas.undo_stack.is_empty();
                            let can_redo = !build_view.bim_bus.redo_stack.is_empty()
                                || !universal_canvas.redo_stack.is_empty();
                            let has_key = !self.gemini_api_key_draft.trim().is_empty();
                            let needs_key = matches!(
                                self.agent_model,
                                AgentModelChoice::Gemini25Flash | AgentModelChoice::Gemini25Pro
                            ) && !has_key;

                            // 1 + 2. Width-First Layout: Segmented Studio Tabs + Utility Bar side-by-side inside one wrapping glass bar on wide screens
                            let effective_tab = if self.hover_more_tools_open {
                                DynamicIslandStudioTab::CadAndExport
                            } else {
                                self.studio_tab
                            };
                            let wide_deck = !is_mobile && deck_w >= 860.0;
                            let two_col_deck = !is_mobile && deck_w >= 1040.0;
                            let mut render_header_strip = |ui: &mut Ui, draw_tabs: bool, draw_util: bool| {
                                if draw_tabs {
                                    for tab in DynamicIslandStudioTab::ALL {
                                        let is_act = effective_tab == tab;
                                        let lbl = if is_mobile {
                                            tab.short_label()
                                        } else {
                                            tab.label()
                                        };
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            Some(tab.icon()),
                                            lbl,
                                            is_act,
                                            theme.accent_blue,
                                            "",
                                        )
                                        .clicked()
                                        {
                                            self.studio_tab = tab;
                                            self.hover_more_tools_open =
                                                tab == DynamicIslandStudioTab::CadAndExport;
                                        }
                                    }
                                }
                                if draw_tabs && draw_util {
                                    ui.separator();
                                }
                                if draw_util {
                                        if world_total > 0 {
                                            icons::pill_badge(
                                                ui,
                                                AppIcon::Layers,
                                                format!("World: {world_total}"),
                                                theme.accent_blue,
                                            );
                                            let rep = &universal_canvas.neighborhood_report;
                                            if rep.neighbors_evaluated > 0 {
                                                let exp_tag = match rep.recommended_exposure {
                                                    crate::core::wind::ExposureCategory::ExposureB => {
                                                        "ExpB"
                                                    }
                                                    crate::core::wind::ExposureCategory::ExposureC => {
                                                        "ExpC"
                                                    }
                                                    crate::core::wind::ExposureCategory::ExposureD => {
                                                        "ExpD"
                                                    }
                                                };
                                                let gap_tag = rep
                                                    .min_clear_gap_m
                                                    .map_or("--".to_string(), |g| format!("{g:.1}m"));
                                                let nb_col = if rep.pounding_risk_count > 0 {
                                                    theme.accent_red
                                                } else if rep.tight_buffer_count > 0 {
                                                    theme.accent_orange
                                                } else {
                                                    theme.accent_green
                                                };
                                                icons::pill_badge(
                                                    ui,
                                                    AppIcon::AlertTriangle,
                                                    format!(
                                                        "Nb:{} Gap {gap_tag} {exp_tag}{}",
                                                        rep.neighbors_evaluated,
                                                        if rep.pounding_risk_count > 0 {
                                                            " POUND"
                                                        } else {
                                                            ""
                                                        }
                                                    ),
                                                    nb_col,
                                                );
                                            }
                                        }
                                        if !build_view.bim_bus.project.is_empty() || !members.is_empty()
                                        {
                                            icons::pill_badge(
                                                ui,
                                                AppIcon::Bolt,
                                                format!(
                                                    "{}C {}B • DCR {:.0}% • EigenGate {:.2}{}",
                                                    build_view.bim_bus.project.columns.len(),
                                                    build_view.bim_bus.project.runs.len(),
                                                    max_dcr,
                                                    min_eigengate,
                                                    if crit_cnt > 0 {
                                                        format!(" • Crit {crit_cnt}")
                                                    } else {
                                                        String::new()
                                                    }
                                                ),
                                                health_col,
                                            );
                                        }
                                        if let Some(target) = worst_target {
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                Some(AppIcon::Target),
                                                "Worst",
                                                false,
                                                theme.accent_orange,
                                                "Select critical / highest-DCR structural element on Earth 3D",
                                            )
                                            .clicked()
                                            {
                                                self.active_canvas_mode =
                                                    SpatialCanvasMode::EarthGlobe;
                                                universal_canvas.globe_mode = true;
                                                self.selected_target = Some(target);
                                                self.popup_tab = SmartPopupTab::Calculations;
                                                actions.fit_camera = true;
                                                self.push_trace(
                                                    "Focused worst bottleneck member",
                                                    true,
                                                );
                                            }
                                        }
                                        if (crit_cnt > 0 || max_dcr > 95.0)
                                            && Self::glass_pill_button(
                                                ui,
                                                theme,
                                                Some(AppIcon::Wrench),
                                                "Auto-Repair",
                                                true,
                                                theme.accent_orange,
                                                "Upgrade low-redundancy / overstressed elements cleanly",
                                            )
                                            .clicked()
                                        {
                                            actions.run_auto_repair = true;
                                        }

                                        ui.separator();

                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            Some(AppIcon::Undo),
                                            "Undo",
                                            false,
                                            theme.accent_blue,
                                            "Undo last BIM or 2D canvas action (Cmd/Ctrl+Z)",
                                        )
                                        .clicked()
                                            && can_undo
                                        {
                                            actions.undo = true;
                                        }
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            Some(AppIcon::Redo),
                                            "Redo",
                                            false,
                                            theme.accent_blue,
                                            "Redo undone BIM or 2D canvas action (Cmd/Ctrl+Y)",
                                        )
                                        .clicked()
                                            && can_redo
                                        {
                                            actions.redo = true;
                                        }
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            Some(AppIcon::ZoomFit),
                                            "Fit",
                                            false,
                                            theme.accent_blue,
                                            "Fit Earth 3D viewport to model",
                                        )
                                        .clicked()
                                        {
                                            actions.fit_camera = true;
                                        }
                                        let key_accent = if has_key {
                                            theme.accent_green
                                        } else if needs_key {
                                            theme.accent_orange
                                        } else {
                                            theme.accent_blue
                                        };
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            Some(AppIcon::Lock),
                                            if has_key { "Key OK" } else { "API Key" },
                                            self.api_key_popover_open || has_key || needs_key,
                                            key_accent,
                                            "Configure Gemini API Key",
                                        )
                                        .clicked()
                                        {
                                            self.toggle_exclusive_drawer("api_key");
                                        }
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            Some(AppIcon::SettingsGear),
                                            "Settings",
                                            false,
                                            theme.accent_blue,
                                            "Open Workspace Settings",
                                        )
                                        .clicked()
                                        {
                                            actions.open_settings = true;
                                        }
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            Some(AppIcon::MapPin),
                                            if self.mobile_deck_pinned { "Pinned" } else { "Pin" },
                                            self.mobile_deck_pinned,
                                            theme.accent_green,
                                            "Pin Studio Deck open while authoring on Earth 3D (Alt+1..4 tabs)",
                                        )
                                        .clicked()
                                        {
                                            self.mobile_deck_pinned = !self.mobile_deck_pinned;
                                        }
                                }
                            };
                            if wide_deck {
                                Self::glass_subpill_group(
                                    ui,
                                    theme,
                                    10,
                                    Margin::symmetric(6, 3),
                                    deck_w,
                                    |ui| render_header_strip(ui, true, true),
                                );
                            } else {
                                Self::glass_subpill_group(
                                    ui,
                                    theme,
                                    10,
                                    Margin::symmetric(6, 3),
                                    deck_w,
                                    |ui| render_header_strip(ui, true, false),
                                );
                                ui.add_space(4.0);
                                Self::glass_subpill_group(
                                    ui,
                                    theme,
                                    9,
                                    Margin::symmetric(6, 3),
                                    deck_w,
                                    |ui| render_header_strip(ui, false, true),
                                );
                            }

                            ui.add_space(4.0);

                            // 3. Active Studio Tab Content (Width-First horizontal flow on wide screens, stacking upward only when horizontal bounds fill)
                            let mut render_tab_body = |ui: &mut Ui, deck_w: f32| match effective_tab {
                                DynamicIslandStudioTab::ViewAndLens => {
                                    let tilt = universal_canvas.earth_world.camera.tilt_deg;
                                    Self::glass_subpill_group(
                                        ui,
                                        theme,
                                        9,
                                        Margin::symmetric(7, 3),
                                        deck_w,
                                        |ui| {
                                            ui.label(
                                                RichText::new("Cam:")
                                                    .size(9.3)
                                                    .strong()
                                                    .color(theme.text_muted),
                                            );
                                            for (preset, lbl, act, tip) in [
                                                (
                                                    "iso",
                                                    "Iso",
                                                    tilt >= 5.0 && tilt <= 55.0,
                                                    "Earth 3D Isometric Camera (42° tilt)",
                                                ),
                                                (
                                                    "top",
                                                    "Top",
                                                    tilt < 5.0,
                                                    "Earth 3D Nadir Top-Down Plan View (0° tilt)",
                                                ),
                                                (
                                                    "front",
                                                    "Elev",
                                                    tilt > 55.0,
                                                    "Earth 3D Low-Horizon Elevation Camera (64° tilt)",
                                                ),
                                            ] {
                                                if Self::glass_pill_button(
                                                    ui,
                                                    theme,
                                                    None,
                                                    lbl,
                                                    act,
                                                    theme.accent_blue,
                                                    tip,
                                                )
                                                .clicked()
                                                {
                                                    self.apply_camera_preset(
                                                        build_view,
                                                        universal_canvas,
                                                        preset,
                                                    );
                                                    actions.fit_camera = true;
                                                }
                                            }
                                            ui.separator();
                                            ui.label(
                                                RichText::new("Lens:")
                                                    .size(9.3)
                                                    .strong()
                                                    .color(theme.text_muted),
                                            );
                                            for (lens, lbl) in [
                                                (SpatialLens::Clean, "Clean"),
                                                (SpatialLens::DcrHeatmap, "DCR%"),
                                                (SpatialLens::EigenGateRedundancy, "EigenGate"),
                                                (SpatialLens::BendingMoment, "Moment"),
                                                (SpatialLens::DeformedShape, "Deform"),
                                                (SpatialLens::ModalVibration, "Modal"),
                                                (SpatialLens::TimeHistoryReplay, "EQ Replay"),
                                                (SpatialLens::ShmDigitalTwin, "SHM Twin"),
                                                (SpatialLens::Erection4D, "4D"),
                                            ] {
                                                if Self::glass_pill_button(
                                                    ui,
                                                    theme,
                                                    None,
                                                    lbl,
                                                    self.active_lens == lens,
                                                    theme.accent_blue,
                                                    "",
                                                )
                                                .clicked()
                                                {
                                                    self.apply_lens_to_views(
                                                        lens,
                                                        build_view,
                                                        universal_canvas,
                                                    );
                                                }
                                            }
                                        },
                                    );
                                    if self.active_lens != SpatialLens::Clean {
                                        ui.add_space(4.0);
                                        self.render_active_lens_colorbar_strip(
                                            ui,
                                            theme,
                                            max_dcr,
                                            min_eigengate,
                                            &universal_canvas.earth_fem_cache,
                                            universal_canvas.earth_bim_author.active_modal_mode,
                                            deck_w,
                                        );
                                    }
                                    ui.add_space(4.0);
                                    Self::width_first_pair(ui, deck_w, two_col_deck, |ui, is_left, col_w| {
                                        if is_left {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("CRC & Cascade:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Bolt),
                                                        "CRC + Cascade",
                                                        universal_canvas.last_crc_cascade_report.is_some(),
                                                        theme.accent_orange,
                                                        "Run Two-Stage EigenGate + StructGNN-CRC Conformal Screening & 3D Progressive Collapse Cascade",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.run_crc_cascade_screening =
                                                            Some(universal_canvas.cascade_daf);
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        None,
                                                        format!("DAF {:.2}x", universal_canvas.cascade_daf),
                                                        false,
                                                        theme.accent_blue,
                                                        "Cycle UFC 4-023-03 Dynamic Amplification Factor: 1.20x / 1.50x / 2.00x",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.cascade_daf =
                                                            if universal_canvas.cascade_daf < 1.35 {
                                                                1.50
                                                            } else if universal_canvas.cascade_daf < 1.75 {
                                                                2.00
                                                            } else {
                                                                1.20
                                                            };
                                                    }
                                                    if let Some(ref crc_rep) =
                                                        universal_canvas.last_crc_cascade_report
                                                    {
                                                        let max_steps = crc_rep.total_steps.clamp(1, 4);
                                                        for s in 1..=max_steps {
                                                            let act = universal_canvas.active_cascade_step == s;
                                                            if Self::glass_pill_button(
                                                                ui,
                                                                theme,
                                                                None,
                                                                format!("S{s}"),
                                                                act,
                                                                theme.accent_red,
                                                                "Inspect domino collapse chronology at step",
                                                            )
                                                            .clicked()
                                                            {
                                                                universal_canvas.active_cascade_step = s;
                                                            }
                                                        }
                                                        if Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            Some(AppIcon::Cube3D),
                                                            "CRC .ifcx",
                                                            false,
                                                            theme.accent_green,
                                                            "Export eigengraph::analysis::crc_cascade .ifcx overlay",
                                                        )
                                                        .clicked()
                                                        {
                                                            actions.export_crc_cascade_ifcx = true;
                                                        }
                                                    }
                                                },
                                            );
                                        } else {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    use crate::core::ground_motion_replay::GroundMotionReplayConfig;
                                                    ui.label(
                                                        RichText::new("Ground Motion & Async CRC:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    let worker_busy = universal_canvas.replay_async_manager.is_running();
                                                    let has_rep = universal_canvas.last_ground_motion_replay_report.is_some();
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Bolt),
                                                        if worker_busy { "Running..." } else { "Async EQ" },
                                                        has_rep || worker_busy,
                                                        theme.accent_orange,
                                                        "Run non-blocking background StructGNN-CRC & live 60 FPS Earthquake Replay",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.apply_lens_to_views(
                                                            SpatialLens::TimeHistoryReplay,
                                                            build_view,
                                                            universal_canvas,
                                                        );
                                                        actions.run_async_ground_motion_replay = true;
                                                    }
                                                    let rec_lbl = format!(
                                                        "Rec: {}",
                                                        GroundMotionReplayConfig::record_short_label(
                                                            universal_canvas.ground_motion_replay_config.record
                                                        )
                                                    );
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        None,
                                                        rec_lbl,
                                                        false,
                                                        theme.accent_blue,
                                                        "Cycle earthquake record (Northridge / Kobe / El Centro / Chi-Chi / Sumatra)",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.ground_motion_replay_config.record =
                                                            GroundMotionReplayConfig::next_record(
                                                                universal_canvas.ground_motion_replay_config.record,
                                                            );
                                                        self.apply_lens_to_views(
                                                            SpatialLens::TimeHistoryReplay,
                                                            build_view,
                                                            universal_canvas,
                                                        );
                                                        actions.run_async_ground_motion_replay = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        None,
                                                        format!(
                                                            "SF {:.2}x",
                                                            universal_canvas.ground_motion_replay_config.scale_factor
                                                        ),
                                                        false,
                                                        theme.text_secondary,
                                                        "Cycle MCE ground motion scale factor (0.75x..2.00x)",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.ground_motion_replay_config.cycle_scale_factor();
                                                        actions.run_async_ground_motion_replay = true;
                                                    }
                                                    let hinges_on = universal_canvas.ground_motion_replay_config.show_plastic_hinges;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Target),
                                                        "Hinges",
                                                        hinges_on,
                                                        theme.accent_red,
                                                        "Toggle 3D ASCE 41-17 plastic hinge beacons",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.ground_motion_replay_config.show_plastic_hinges = !hinges_on;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::ChartLine),
                                                        "Replay UI",
                                                        self.ground_motion_drawer_open,
                                                        theme.accent_blue,
                                                        "Open Live Earthquake Replay & Async StructGNN-CRC drawer",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.toggle_exclusive_drawer("ground_motion");
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Cube3D),
                                                        "Replay .ifcx",
                                                        false,
                                                        theme.accent_green,
                                                        "Export non-destructive eigengraph::analysis::time_history_replay .ifcx overlay",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_ground_motion_replay_ifcx = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Bolt),
                                                        "GPU MC",
                                                        self.ground_motion_drawer_open && self.ground_motion_drawer_tab == 2,
                                                        theme.accent_orange,
                                                        "Open WebGPU @compute Multi-Hazard Particles & FEMA P-58 Monte Carlo drawer",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.ground_motion_drawer_tab = 2;
                                                        if !self.ground_motion_drawer_open {
                                                            self.toggle_exclusive_drawer("ground_motion");
                                                        }
                                                        if universal_canvas.last_monte_carlo_report.is_none() {
                                                            actions.run_gpu_monte_carlo = Some(
                                                                universal_canvas.monte_carlo_config.normalized_sample_count(),
                                                            );
                                                        }
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Shield),
                                                        "SHM UI",
                                                        self.shm_drawer_open,
                                                        theme.accent_green,
                                                        "Open Live SHM Digital Twin & Automated ATC-20 Placard drawer",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.toggle_exclusive_drawer("shm");
                                                        self.apply_lens_to_views(
                                                            SpatialLens::ShmDigitalTwin,
                                                            build_view,
                                                            universal_canvas,
                                                        );
                                                        if universal_canvas.last_shm_twin_report.is_none() {
                                                            actions.run_shm_digital_twin = true;
                                                        }
                                                    }
                                                },
                                            );
                                        }
                                    });
                                }
                                DynamicIslandStudioTab::BimAndStory => {
                                    let (_, _, story_lbl, lvl_cnt) =
                                        self.active_base_and_top_levels(&build_view.bim_bus.project);
                                    Self::glass_subpill_group(
                                        ui,
                                        theme,
                                        9,
                                        Margin::symmetric(7, 3),
                                        deck_w,
                                        |ui| {
                                            ui.label(
                                                RichText::new("Floor:")
                                                    .size(9.3)
                                                    .strong()
                                                    .color(theme.text_muted),
                                            );
                                            let story_btn_lbl = if is_mobile {
                                                format!("Story L{}", self.active_story_idx + 1)
                                            } else {
                                                format!("Story: {story_lbl}")
                                            };
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                story_btn_lbl,
                                                false,
                                                theme.accent_blue,
                                                "Cycle active placement floor level",
                                            )
                                            .clicked()
                                            {
                                                self.active_story_idx =
                                                    (self.active_story_idx + 1) % lvl_cnt.max(1);
                                            }
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                "+ Story",
                                                false,
                                                theme.accent_green,
                                                "Add upper floor story (+3.5m) on Earth 3D",
                                            )
                                            .clicked()
                                                && self.add_story_above_top(build_view).is_ok()
                                            {
                                                actions.sync_fem_and_earth = true;
                                                actions.fit_camera = true;
                                            }
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                "+ Slab",
                                                false,
                                                theme.accent_green,
                                                "Add structural concrete floor slab on Earth 3D",
                                            )
                                            .clicked()
                                                && self.add_slab_at_active_story(build_view).is_ok()
                                            {
                                                actions.sync_fem_and_earth = true;
                                            }
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                "Clone ↑",
                                                false,
                                                theme.accent_blue,
                                                "Clone active floor columns, beams, walls & slabs one story upward (+3.5m)",
                                            )
                                            .clicked()
                                                && self.clone_active_story_upward(build_view).is_ok()
                                            {
                                                actions.sync_fem_and_earth = true;
                                                actions.fit_camera = true;
                                            }
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                "+ Footings",
                                                false,
                                                theme.accent_orange,
                                                "Auto-generate subsurface pile caps under ground-story columns",
                                            )
                                            .clicked()
                                                && self
                                                    .auto_generate_missing_footings(build_view)
                                                    .is_ok()
                                            {
                                                actions.sync_fem_and_earth = true;
                                            }
                                            ui.separator();
                                            let iso_on = universal_canvas
                                                .earth_bim_author
                                                .isolate_active_story;
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                if iso_on { "Isolate ON" } else { "Isolate" },
                                                iso_on,
                                                theme.accent_orange,
                                                "Isolate active Story elements for unobstructed interior BIM authoring",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas
                                                    .earth_bim_author
                                                    .isolate_active_story = !iso_on;
                                            }
                                            let sec_on =
                                                universal_canvas.earth_bim_author.section_cut_active;
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                if sec_on { "Sec Cut ON" } else { "Sec Cut" },
                                                sec_on,
                                                theme.accent_orange,
                                                "Slice 3D building horizontally at target elevation on Earth 3D (/section)",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas
                                                    .earth_bim_author
                                                    .section_cut_active = !sec_on;
                                            }
                                            if universal_canvas.earth_bim_author.section_cut_active
                                            {
                                                ui.add(
                                                    egui::DragValue::new(
                                                        &mut universal_canvas
                                                            .earth_bim_author
                                                            .section_cut_elev_m,
                                                    )
                                                    .range(0.5..=120.0)
                                                    .speed(0.25)
                                                    .prefix("Z:")
                                                    .suffix("m"),
                                                );
                                            }
                                            ui.separator();
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                if universal_canvas.ortho_mode {
                                                    "Ortho ON"
                                                } else {
                                                    "Ortho"
                                                },
                                                universal_canvas.ortho_mode,
                                                theme.accent_blue,
                                                "Toggle orthogonal 90° snap when drawing Beams/Walls/Lines on Earth 3D",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas.ortho_mode =
                                                    !universal_canvas.ortho_mode;
                                            }
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                None,
                                                if build_view.show_grid {
                                                    "Grid ON"
                                                } else {
                                                    "Grid OFF"
                                                },
                                                build_view.show_grid,
                                                theme.accent_blue,
                                                "Show/hide engineering grid on Earth 3D surface",
                                            )
                                            .clicked()
                                            {
                                                let next_grid = !build_view.show_grid;
                                                build_view.show_grid = next_grid;
                                                universal_canvas.show_grid = next_grid;
                                            }
                                        },
                                    );
                                    ui.add_space(4.0);
                                    Self::width_first_pair(ui, deck_w, two_col_deck, |ui, is_left, col_w| {
                                        if is_left {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("Retrofit:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        None,
                                                        format!("Mode: {}", universal_canvas.retrofit_strategy.short_label()),
                                                        false,
                                                        theme.accent_blue,
                                                        "Cycle automated retrofit strategy: Auto / BRB / Jacket / Shear Wall",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.retrofit_strategy =
                                                            universal_canvas.retrofit_strategy.cycle_next();
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Shield),
                                                        "Auto-Retrofit",
                                                        universal_canvas.last_retrofit_report.is_some(),
                                                        theme.accent_green,
                                                        "Synthesize code-verified structural retrofit & non-destructive .ifcx strengthening layer",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.run_structural_retrofit =
                                                            Some(universal_canvas.retrofit_strategy);
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Bolt),
                                                        "+ BRB",
                                                        false,
                                                        theme.accent_orange,
                                                        "Inject 3D Buckling-Restrained Braces (BRB) across low-EigenGate structural bays",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.retrofit_strategy =
                                                            crate::core::retrofit::RetrofitStrategy::BrbBracingOnly;
                                                        actions.run_structural_retrofit = Some(
                                                            crate::core::retrofit::RetrofitStrategy::BrbBracingOnly,
                                                        );
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Wrench),
                                                        "+ Jacket",
                                                        false,
                                                        theme.accent_blue,
                                                        "Apply RC column/beam jacketing & foundation footing enlargement",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.retrofit_strategy =
                                                            crate::core::retrofit::RetrofitStrategy::RcJacketingOnly;
                                                        actions.run_structural_retrofit = Some(
                                                            crate::core::retrofit::RetrofitStrategy::RcJacketingOnly,
                                                        );
                                                    }
                                                    if universal_canvas.last_retrofit_report.is_some() {
                                                        ui.separator();
                                                        if Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            None,
                                                            if universal_canvas.viewing_pre_retrofit_baseline {
                                                                "View: Before"
                                                            } else {
                                                                "View: After"
                                                            },
                                                            !universal_canvas.viewing_pre_retrofit_baseline,
                                                            theme.accent_blue,
                                                            "Compare intact pre-retrofit baseline vs. strengthened 3D structure",
                                                        )
                                                        .clicked()
                                                        {
                                                            actions.toggle_retrofit_before_after = true;
                                                        }
                                                        if Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            Some(AppIcon::Cube3D),
                                                            "Delta .ifcx",
                                                            false,
                                                            theme.accent_green,
                                                            "Export non-destructive eigengraph::bim::retrofit_delta .ifcx strengthening layer",
                                                        )
                                                        .clicked()
                                                        {
                                                            actions.export_retrofit_delta_ifcx = true;
                                                        }
                                                    }
                                                },
                                            );
                                        } else {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("District Portfolio:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    let pf_on = universal_canvas.portfolio_retrofit_config.overlay_visible
                                                        || self.portfolio_drawer_open;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::ChartLine),
                                                        "Portfolio Pareto",
                                                        pf_on,
                                                        theme.accent_green,
                                                        "Run multi-building district EigenGate Pareto optimizer & open frontier drawer",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.portfolio_retrofit_config.overlay_visible = true;
                                                        self.toggle_exclusive_drawer("portfolio");
                                                        actions.run_portfolio_retrofit = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Target),
                                                        format!(
                                                            "Obj: {}",
                                                            universal_canvas
                                                                .portfolio_retrofit_config
                                                                .objective
                                                                .short_label()
                                                        ),
                                                        false,
                                                        theme.accent_blue,
                                                        "Cycle portfolio Pareto selection objective (EigenGate/$ / Balanced / Low-CO2 / Max EigenGate)",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.portfolio_retrofit_config.objective =
                                                            universal_canvas
                                                                .portfolio_retrofit_config
                                                                .objective
                                                                .cycle_next();
                                                        actions.run_portfolio_retrofit = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Coins),
                                                        format!(
                                                            "Cap: ${:.0}k",
                                                            universal_canvas
                                                                .portfolio_retrofit_config
                                                                .budget_cap_usd
                                                                / 1000.0
                                                        ),
                                                        false,
                                                        theme.accent_orange,
                                                        "Cycle district budget cap ($100k / $250k / $500k / $1000k)",
                                                    )
                                                    .clicked()
                                                    {
                                                        let cur = universal_canvas
                                                            .portfolio_retrofit_config
                                                            .budget_cap_usd;
                                                        let next = if cur < 150_000.0 {
                                                            250_000.0
                                                        } else if cur < 375_000.0 {
                                                            500_000.0
                                                        } else if cur < 750_000.0 {
                                                            1_000_000.0
                                                        } else {
                                                            100_000.0
                                                        };
                                                        universal_canvas
                                                            .portfolio_retrofit_config
                                                            .budget_cap_usd = next;
                                                        actions.run_portfolio_retrofit = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Cube3D),
                                                        "Portfolio .ifcx",
                                                        false,
                                                        theme.accent_green,
                                                        "Export non-destructive eigengraph::portfolio::retrofit_plan .ifcx overlay",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_portfolio_retrofit_ifcx = true;
                                                    }
                                                },
                                            );
                                        }
                                    });
                                }
                                DynamicIslandStudioTab::GisAndSite => {
                                    Self::width_first_pair(ui, deck_w, two_col_deck, |ui, is_left, col_w| {
                                        if is_left {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    let map_lbl = format!(
                                                        "Theme: {}",
                                                        universal_canvas.world_map.visual_layer_label()
                                                    );
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        None,
                                                        map_lbl,
                                                        false,
                                                        theme.accent_blue,
                                                        "Cycle Pure-Rust Vector Earth theme: Coastal Blueprint / City Daylight / Night Cyber-Twin / Topographic",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.world_map.cycle_basemap_visual_mode();
                                                        self.push_trace(
                                                            format!(
                                                                "Vector Earth theme: {}",
                                                                universal_canvas.world_map.tile_provider.display_name()
                                                            ),
                                                            true,
                                                        );
                                                    }
                                                    let topo_lbl = if universal_canvas.show_terrain_contours {
                                                        if universal_canvas.site_tin_from_ifc {
                                                            "Topo IFC"
                                                        } else {
                                                            "Topo ON"
                                                        }
                                                    } else {
                                                        "Topo"
                                                    };
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        None,
                                                        topo_lbl,
                                                        universal_canvas.show_terrain_contours,
                                                        theme.accent_green,
                                                        "Toggle pure-vector 3D TIN terrain contours (IFC IfcSite / survey / procedural) & SNI 8460:2017 steep slope (>30°) overlay",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.show_terrain_contours =
                                                            !universal_canvas.show_terrain_contours;
                                                        if universal_canvas.show_terrain_contours {
                                                            universal_canvas.ensure_site_tin_generated();
                                                        }
                                                    }
                                                    let sun_preset = universal_canvas
                                                        .earth_world
                                                        .camera
                                                        .lighting_preset_label();
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(if sun_preset == "Night" {
                                                            AppIcon::Moon
                                                        } else {
                                                            AppIcon::Sun
                                                        }),
                                                        format!("Sun: {sun_preset}"),
                                                        sun_preset != "Day",
                                                        theme.accent_orange,
                                                        "Cycle Pure WebGPU Earth Vector solar lighting: Day (13:30) / Golden Dusk (17:24) / Night (22:00)",
                                                    )
                                                    .clicked()
                                                    {
                                                        let lon = universal_canvas.world_map.center_lat_lon.1;
                                                        let next_h = match sun_preset {
                                                            "Day" => 17.4,
                                                            "Dusk" => 22.0,
                                                            _ => 13.5,
                                                        };
                                                        universal_canvas
                                                            .earth_world
                                                            .camera
                                                            .set_local_sun_hour(next_h, lon);
                                                        self.push_trace(
                                                            format!(
                                                                "Solar lighting: {}",
                                                                universal_canvas
                                                                    .earth_world
                                                                    .camera
                                                                    .lighting_preset_label()
                                                            ),
                                                            true,
                                                        );
                                                    }
                                                    let gpu_on = universal_canvas.webgpu_earth.is_active();
                                                    let gpu_lbl = if gpu_on {
                                                        "WebGPU"
                                                    } else if universal_canvas.webgpu_earth.is_available() {
                                                        "CPU Vec"
                                                    } else {
                                                        "Vec 3D"
                                                    };
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        None,
                                                        gpu_lbl,
                                                        gpu_on,
                                                        theme.accent_blue,
                                                        "Toggle 100% Pure WebGPU (WGSL) 3D Vector Earth pass vs. CPU vector fallback",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.webgpu_earth.enabled =
                                                            !universal_canvas.webgpu_earth.enabled;
                                                    }
                                                    ui.separator();
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::MapPin),
                                                        "Anchor Here",
                                                        false,
                                                        theme.accent_blue,
                                                        "Relocate BIM/IFC origin (0,0) to current Earth 3D center GPS coordinates",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.anchor_and_build_site =
                                                            Some(universal_canvas.world_map.center_lat_lon);
                                                    }
                                                    if let Some(rel) =
                                                        universal_canvas.active_world_project_rel_path.clone()
                                                    {
                                                        if Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            None,
                                                            "Move Pin",
                                                            false,
                                                            theme.accent_orange,
                                                            "Reposition active building on Earth 3D with automatic road/parcel vector OSNAP",
                                                        )
                                                        .clicked()
                                                        {
                                                            universal_canvas.globe_mode = true;
                                                            universal_canvas.placing_unlocated_rel_path =
                                                                Some(rel.clone());
                                                            universal_canvas.placing_heading_deg =
                                                                universal_canvas
                                                                    .world_index
                                                                    .get(&rel)
                                                                    .map(|b| b.heading_deg)
                                                                    .unwrap_or(0.0);
                                                        }
                                                        if Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            None,
                                                            "-15°",
                                                            false,
                                                            theme.accent_blue,
                                                            "Rotate active Earth building -15° counter-clockwise (Shift+R)",
                                                        )
                                                        .clicked()
                                                        {
                                                            actions.rotate_active_world_heading_deg =
                                                                Some(-15.0);
                                                        }
                                                        if Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            None,
                                                            "+15°",
                                                            false,
                                                            theme.accent_blue,
                                                            "Rotate active Earth building +15° clockwise (R)",
                                                        )
                                                        .clicked()
                                                        {
                                                            actions.rotate_active_world_heading_deg =
                                                                Some(15.0);
                                                        }
                                                    }
                                                    if world_total > 0 && (world_crit > 0 || world_unloc > 0)
                                                    {
                                                        ui.separator();
                                                        if world_crit > 0
                                                            && Self::glass_pill_button(
                                                                ui,
                                                                theme,
                                                                None,
                                                                format!("{world_crit} Crit • Fly"),
                                                                false,
                                                                theme.accent_red,
                                                                "Fly Earth 3D camera to the next high-DCR/low-EigenGate structure across the Vault",
                                                            )
                                                            .clicked()
                                                        {
                                                            actions.fly_to_next_world_building = Some(
                                                                crate::core::gis::WorldPortfolioFilter::CriticalRisk,
                                                            );
                                                        }
                                                        if world_unloc > 0 {
                                                            if Self::glass_pill_button(
                                                                ui,
                                                                theme,
                                                                None,
                                                                format!("{world_unloc} Unlocated • Place"),
                                                                false,
                                                                theme.accent_orange,
                                                                "Pick next unlocated Staging model and click anywhere on Earth 3D to pin its real address",
                                                            )
                                                            .clicked()
                                                            {
                                                                let next_rel = universal_canvas
                                                                    .active_world_project_rel_path
                                                                    .clone()
                                                                    .filter(|r| {
                                                                        universal_canvas
                                                                            .world_index
                                                                            .get(r)
                                                                            .is_some_and(|b| {
                                                                                b.is_unlocated()
                                                                            })
                                                                    })
                                                                    .or_else(|| {
                                                                        universal_canvas
                                                                            .world_index
                                                                            .buildings
                                                                            .iter()
                                                                            .find(|b| b.is_unlocated())
                                                                            .map(|b| b.rel_path.clone())
                                                                    });
                                                                if let Some(rel) = next_rel {
                                                                    universal_canvas.globe_mode = true;
                                                                    universal_canvas
                                                                        .placing_unlocated_rel_path =
                                                                        Some(rel.clone());
                                                                    universal_canvas.placing_heading_deg =
                                                                        0.0;
                                                                    self.push_trace(
                                                                        format!(
                                                                            "Place '{rel}' onto Earth 3D (Q/E rotate)"
                                                                        ),
                                                                        true,
                                                                    );
                                                                }
                                                            }
                                                            if Self::glass_pill_button(
                                                                ui,
                                                                theme,
                                                                None,
                                                                "Next Unloc",
                                                                false,
                                                                theme.accent_blue,
                                                                "Fly camera to the next unlocated Staging parcel on Earth 3D",
                                                            )
                                                            .clicked()
                                                            {
                                                                actions.fly_to_next_world_building = Some(
                                                                    crate::core::gis::WorldPortfolioFilter::UnlocatedStaging,
                                                                );
                                                            }
                                                        }
                                                    }
                                                },
                                            );
                                        } else {
                                            // Phase 67: IFCX City-Builder Authoring (`IfcRoad`, `IfcBridge`, `IfcSpatialZone`, Auto Cut/Fill Pad)
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("IFCX City:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    let road_on = universal_canvas.earth_bim_author.active_tool
                                                        == crate::views::universal_canvas::EarthBimTool::Road;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::MapPin),
                                                        format!("Road ({})", universal_canvas.city_scene.roads.iter().filter(|r| !r.is_bridge).count()),
                                                        road_on,
                                                        theme.accent_blue,
                                                        "Draw IFCX IfcRoad multi-lane corridor on Earth 3D (2 clicks)",
                                                    )
                                                    .clicked()
                                                    {
                                                        build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                                                        universal_canvas.earth_bim_author.active_tool = if road_on {
                                                            crate::views::universal_canvas::EarthBimTool::None
                                                        } else {
                                                            crate::views::universal_canvas::EarthBimTool::Road
                                                        };
                                                        universal_canvas.earth_bim_author.pending_start_mm = None;
                                                        universal_canvas.is_frozen = false;
                                                    }
                                                    let bridge_on = universal_canvas.earth_bim_author.active_tool
                                                        == crate::views::universal_canvas::EarthBimTool::Bridge;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::BridgeSpan),
                                                        format!("Bridge ({})", universal_canvas.city_scene.roads.iter().filter(|r| r.is_bridge).count()),
                                                        bridge_on,
                                                        theme.accent_blue,
                                                        "Draw elevated IFCX IfcBridge deck & piers on Earth 3D (2 clicks)",
                                                    )
                                                    .clicked()
                                                    {
                                                        build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                                                        universal_canvas.earth_bim_author.active_tool = if bridge_on {
                                                            crate::views::universal_canvas::EarthBimTool::None
                                                        } else {
                                                            crate::views::universal_canvas::EarthBimTool::Bridge
                                                        };
                                                        universal_canvas.earth_bim_author.pending_start_mm = None;
                                                        universal_canvas.is_frozen = false;
                                                    }
                                                    let zone_on = universal_canvas.earth_bim_author.active_tool
                                                        == crate::views::universal_canvas::EarthBimTool::Zone;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Building),
                                                        format!(
                                                            "Zone: {}",
                                                            universal_canvas.city_scene.active_zone_kind.short_label()
                                                        ),
                                                        zone_on,
                                                        theme.accent_green,
                                                        "Click on Earth 3D to stamp IFCX IfcSpatialZone parcel with context massing & trees",
                                                    )
                                                    .clicked()
                                                    {
                                                        build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                                                        if zone_on {
                                                            universal_canvas.city_scene.active_zone_kind =
                                                                universal_canvas.city_scene.active_zone_kind.cycle_next();
                                                        } else {
                                                            universal_canvas.earth_bim_author.active_tool =
                                                                crate::views::universal_canvas::EarthBimTool::Zone;
                                                            universal_canvas.is_frozen = false;
                                                        }
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Layers),
                                                        "Grade Pad",
                                                        !universal_canvas.city_scene.graded_pads.is_empty(),
                                                        theme.accent_orange,
                                                        "Auto-grade flat foundation pad onto IfcSite TIN & compute Cut/Fill m3",
                                                    )
                                                    .clicked()
                                                    {
                                                        let tin = universal_canvas.active_site_tin.get_or_insert_with(|| {
                                                            crate::core::gis::TinSurfaceMesh::generate_from_dem_preset(
                                                                universal_canvas.site_tin_preset,
                                                                400.0,
                                                                400.0,
                                                                36,
                                                                36,
                                                                1.0,
                                                            )
                                                        });
                                                        let pad = universal_canvas
                                                            .city_scene
                                                            .auto_grade_building_pad(&build_view.bim_bus.project, tin);
                                                        universal_canvas.show_terrain_contours = true;
                                                        self.push_trace(
                                                            format!(
                                                                "Graded {} ({:.0}x{:.0}m): Cut {:.0}m3 | Fill {:.0}m3",
                                                                pad.name,
                                                                pad.width_m,
                                                                pad.depth_m,
                                                                pad.cut_volume_m3,
                                                                pad.fill_volume_m3
                                                            ),
                                                            true,
                                                        );
                                                    }
                                                    let snap_on = universal_canvas.city_scene.road_snap_enabled;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        None,
                                                        if snap_on { "Road Snap" } else { "Snap Off" },
                                                        snap_on,
                                                        theme.accent_blue,
                                                        "Snap BIM columns/walls/footings to road setbacks & parcel alignments",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.city_scene.road_snap_enabled = !snap_on;
                                                    }
                                                },
                                            );
                                        }
                                    });
                                    ui.add_space(4.0);
                                    Self::glass_subpill_group(
                                        ui,
                                        theme,
                                        9,
                                        Margin::symmetric(7, 3),
                                        deck_w,
                                        |ui| {
                                            ui.label(
                                                RichText::new("Multi-Hazard:")
                                                    .size(9.3)
                                                    .strong()
                                                    .color(theme.text_muted),
                                            );
                                            let seis_on = universal_canvas.district_hazard.seismic_wave_active;
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                Some(AppIcon::Pulse),
                                                "Seismic",
                                                seis_on,
                                                theme.accent_orange,
                                                "Toggle SNI 1726 / ASCE 7 traveling seismic wavefield",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas.district_hazard.seismic_wave_active = !seis_on;
                                                actions.run_district_hazard_audit = true;
                                            }
                                            let pound_on = universal_canvas.district_hazard.pounding_overlay_active;
                                            let p_cnt = universal_canvas.district_hazard_report.pounding_risk_count;
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                Some(AppIcon::AlertTriangle),
                                                if p_cnt > 0 {
                                                    format!("Pounding ({p_cnt})")
                                                } else {
                                                    "Pounding".to_string()
                                                },
                                                pound_on,
                                                theme.accent_red,
                                                "Highlight phase-amplified seismic pounding corridors",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas.district_hazard.pounding_overlay_active = !pound_on;
                                                universal_canvas.show_neighborhood_hazards = true;
                                                actions.run_district_hazard_audit = true;
                                            }
                                            let wind_on = universal_canvas.district_hazard.venturi_wind_active;
                                            let v_fac = universal_canvas.district_hazard_report.venturi_speedup_factor;
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                Some(AppIcon::Wind),
                                                if wind_on && v_fac > 1.01 {
                                                    format!("Venturi x{v_fac:.2}")
                                                } else {
                                                    "Venturi".to_string()
                                                },
                                                wind_on,
                                                theme.accent_blue,
                                                "Toggle urban Venturi wind channeling streamlines",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas.district_hazard.venturi_wind_active = !wind_on;
                                                actions.run_district_hazard_audit = true;
                                            }
                                            let flood_on = universal_canvas.district_hazard.flood_plane_active;
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                Some(AppIcon::Waves),
                                                if flood_on {
                                                    format!(
                                                        "Flood {:+.1}m",
                                                        universal_canvas.district_hazard.flood_water_level_m
                                                    )
                                                } else {
                                                    "Flood 3D".to_string()
                                                },
                                                flood_on,
                                                theme.accent_blue,
                                                "Toggle 3D flood plane against IfcSite TIN contours",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas.district_hazard.flood_plane_active = !flood_on;
                                                if !flood_on {
                                                    universal_canvas.ensure_site_tin_generated();
                                                    universal_canvas.show_terrain_contours = true;
                                                }
                                                actions.run_district_hazard_audit = true;
                                            }
                                            if flood_on {
                                                if Self::glass_pill_button(
                                                    ui,
                                                    theme,
                                                    None,
                                                    "-0.5m",
                                                    false,
                                                    theme.text_secondary,
                                                    "Lower 3D flood elevation by 0.5m",
                                                )
                                                .clicked()
                                                {
                                                    universal_canvas.district_hazard.flood_water_level_m =
                                                        (universal_canvas.district_hazard.flood_water_level_m - 0.5)
                                                            .clamp(-5.0, 25.0);
                                                    actions.run_district_hazard_audit = true;
                                                }
                                                if Self::glass_pill_button(
                                                    ui,
                                                    theme,
                                                    None,
                                                    "+0.5m",
                                                    false,
                                                    theme.text_secondary,
                                                    "Raise 3D flood elevation by 0.5m",
                                                )
                                                .clicked()
                                                {
                                                    universal_canvas.district_hazard.flood_water_level_m =
                                                        (universal_canvas.district_hazard.flood_water_level_m + 0.5)
                                                            .clamp(-5.0, 25.0);
                                                    actions.run_district_hazard_audit = true;
                                                }
                                            }
                                            ui.separator();
                                            let gp_on = universal_canvas.monte_carlo_config.gpu_particles_enabled;
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                Some(AppIcon::Bolt),
                                                if gp_on {
                                                    format!(
                                                        "GPU {}P",
                                                        universal_canvas.monte_carlo_config.normalized_particle_count()
                                                    )
                                                } else {
                                                    "GPU Particles".to_string()
                                                },
                                                gp_on,
                                                theme.accent_blue,
                                                "Toggle 60 FPS Pure WebGPU @compute seismic wave & Venturi wind obstacle particles",
                                            )
                                            .clicked()
                                            {
                                                universal_canvas.monte_carlo_config.gpu_particles_enabled = !gp_on;
                                                if universal_canvas.monte_carlo_config.gpu_particles_enabled
                                                    && !universal_canvas.district_hazard.any_active()
                                                {
                                                    universal_canvas.district_hazard.seismic_wave_active = true;
                                                    universal_canvas.district_hazard.venturi_wind_active = true;
                                                }
                                            }
                                            if Self::glass_pill_button(
                                                ui,
                                                theme,
                                                Some(AppIcon::Sparkles),
                                                "AI Scan -> IFCX",
                                                false,
                                                theme.accent_green,
                                                "Stream AI real-building massing .ifcx onto Earth with automatic frame inference & instant 6-DOF EigenGate triage",
                                            )
                                            .clicked()
                                            {
                                                actions.ingest_ai_real_building_capture = true;
                                            }
                                        },
                                    );
                                    ui.add_space(4.0);
                                    Self::width_first_pair(ui, deck_w, two_col_deck, |ui, is_left, col_w| {
                                        if is_left {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("SSI & Soil:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    let ssi_on =
                                                        universal_canvas.subsurface_ssi.ssi_coupling_enabled;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Layers),
                                                        "Winkler SSI",
                                                        ssi_on,
                                                        theme.accent_blue,
                                                        "Toggle coupled 6-DOF Winkler foundation springs & modal period lengthening",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas
                                                            .subsurface_ssi
                                                            .ssi_coupling_enabled = !ssi_on;
                                                        actions.run_subsurface_ssi_sync = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Database),
                                                        format!(
                                                            "Soil: {}",
                                                            universal_canvas
                                                                .subsurface_ssi
                                                                .borehole_preset
                                                                .short_label()
                                                        ),
                                                        ssi_on
                                                            || universal_canvas
                                                                .subsurface_ssi
                                                                .xray_subsurface_visible,
                                                        theme.accent_orange,
                                                        "Cycle borehole soil stratigraphy archetype (Soft Clay / Liquefiable / Stiff Alluvium / Dense Rock)",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.subsurface_ssi.borehole_preset =
                                                            universal_canvas
                                                                .subsurface_ssi
                                                                .borehole_preset
                                                                .cycle_next();
                                                        universal_canvas
                                                            .subsurface_ssi
                                                            .xray_subsurface_visible = true;
                                                        actions.run_subsurface_ssi_sync = true;
                                                    }
                                                    let liq_on = universal_canvas
                                                        .subsurface_ssi
                                                        .liquefaction_trigger_active;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Waves),
                                                        "Liquefy",
                                                        liq_on,
                                                        theme.accent_red,
                                                        "Toggle Seed-Idriss seismic liquefaction trigger & p-y spring degradation",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas
                                                            .subsurface_ssi
                                                            .liquefaction_trigger_active = !liq_on;
                                                        actions.run_subsurface_ssi_sync = true;
                                                    }
                                                    let xray_on = universal_canvas
                                                        .subsurface_ssi
                                                        .xray_subsurface_visible;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::SectionCut),
                                                        "X-Ray Ground",
                                                        xray_on,
                                                        theme.accent_blue,
                                                        "Toggle 3D Pure WebGPU subsurface stratigraphy, borehole casing & groundwater table cutaway",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas
                                                            .subsurface_ssi
                                                            .xray_subsurface_visible = !xray_on;
                                                        actions.run_subsurface_ssi_sync = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Cube3D),
                                                        "SSI .ifcx",
                                                        false,
                                                        theme.accent_green,
                                                        "Export non-destructive eigengraph::geotech::stratigraphy & ssi_springs .ifcx overlay",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_subsurface_ssi_ifcx = true;
                                                    }
                                                },
                                            );
                                        } else {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("Evacuation & Lifelines:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Shield),
                                                        "Evac Route",
                                                        self.evacuation_drawer_open,
                                                        theme.accent_green,
                                                        "Compute A* post-disaster safe evacuation & emergency responder routes",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.open_evacuation_drawer();
                                                        actions.run_evacuation_routing = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Target),
                                                        format!(
                                                            "Mode: {}",
                                                            universal_canvas
                                                                .evacuation_config
                                                                .mode
                                                                .short_label()
                                                        ),
                                                        universal_canvas.evacuation_config.overlay_visible,
                                                        theme.accent_blue,
                                                        "Cycle lifeline routing display (Pedestrian / Responder / Both)",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.evacuation_config.mode =
                                                            universal_canvas
                                                                .evacuation_config
                                                                .mode
                                                                .cycle_next();
                                                        actions.run_evacuation_routing = true;
                                                    }
                                                    let evac_3d =
                                                        universal_canvas.evacuation_config.overlay_visible;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Globe),
                                                        "Evac 3D",
                                                        evac_3d,
                                                        theme.accent_green,
                                                        "Toggle 3D Pure WebGPU evacuation corridors, debris cordons & assembly beacons",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas
                                                            .evacuation_config
                                                            .overlay_visible = !evac_3d;
                                                        if universal_canvas
                                                            .evacuation_config
                                                            .overlay_visible
                                                            && universal_canvas
                                                                .last_evacuation_report
                                                                .is_none()
                                                        {
                                                            actions.run_evacuation_routing = true;
                                                        }
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Cube3D),
                                                        "Evac .ifcx",
                                                        false,
                                                        theme.accent_green,
                                                        "Export non-destructive eigengraph::analysis::evacuation_lifelines .ifcx overlay",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_evacuation_ifcx = true;
                                                    }
                                                    let rm_on = universal_canvas
                                                        .collab_state
                                                        .config
                                                        .risk_heatmap_visible;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Flame),
                                                        "3D Risk Map",
                                                        rm_on || (self.collab_drawer_open && self.collab_drawer_tab == 1),
                                                        theme.accent_red,
                                                        "Toggle 3D Pure WebGPU District Risk Field Heatmap & Hotspots",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas
                                                            .collab_state
                                                            .config
                                                            .risk_heatmap_visible = !rm_on;
                                                        self.open_collab_drawer(1);
                                                        actions.refresh_district_risk_field = true;
                                                    }
                                                },
                                            );
                                        }
                                    });
                                }
                                DynamicIslandStudioTab::CadAndExport => {
                                    Self::width_first_pair(ui, deck_w, two_col_deck, |ui, is_left, col_w| {
                                        if is_left {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("2D & Earth:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    for ext_tool in [
                                                        UniversalCanvasTool::Freehand,
                                                        UniversalCanvasTool::Circle,
                                                        UniversalCanvasTool::Dimension,
                                                        UniversalCanvasTool::Callout,
                                                        UniversalCanvasTool::BoxSelect,
                                                        UniversalCanvasTool::Eraser,
                                                        UniversalCanvasTool::EarthUplift,
                                                        UniversalCanvasTool::EarthCarve,
                                                        UniversalCanvasTool::EarthSeawall,
                                                        UniversalCanvasTool::EarthBridge,
                                                        UniversalCanvasTool::EarthCorridor,
                                                    ] {
                                                        let is_act = build_view.ground_author_tool
                                                            == Ground3DAuthorTool::Orbit
                                                            && universal_canvas.active_tool == ext_tool;
                                                        if Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            Some(ext_tool.app_icon()),
                                                            ext_tool.short_label(),
                                                            is_act,
                                                            theme.accent_orange,
                                                            ext_tool.tooltip(),
                                                        )
                                                        .clicked()
                                                        {
                                                            self.active_canvas_mode =
                                                                SpatialCanvasMode::EarthGlobe;
                                                            universal_canvas.globe_mode = true;
                                                            build_view.ground_author_tool =
                                                                Ground3DAuthorTool::Orbit;
                                                            universal_canvas.earth_bim_author.active_tool =
                                                                crate::views::universal_canvas::EarthBimTool::None;
                                                            universal_canvas.active_tool = ext_tool;
                                                            self.push_trace(
                                                                format!(
                                                                    "Earth 3D Tool: {}",
                                                                    ext_tool.short_label()
                                                                ),
                                                                true,
                                                            );
                                                        }
                                                    }
                                                    if !universal_canvas.entities.is_empty()
                                                        && Self::glass_pill_button(
                                                            ui,
                                                            theme,
                                                            None,
                                                            "Promote -> BIM",
                                                            false,
                                                            theme.accent_green,
                                                            "Promote selected vector Lines/Rects on Earth 3D into 3D BIM Walls, Beams & Slabs",
                                                        )
                                                        .clicked()
                                                    {
                                                        let (w, r, s) = universal_canvas
                                                            .promote_selection_to_bim(
                                                                &mut build_view.bim_bus.project,
                                                            );
                                                        if w + r + s > 0 {
                                                            actions.sync_fem_and_earth = true;
                                                            self.push_trace(
                                                                format!(
                                                                    "Promoted +{w}W +{r}B +{s}S on Earth 3D"
                                                                ),
                                                                true,
                                                            );
                                                        }
                                                    }
                                                    ui.separator();
                                                    ui.label(
                                                        RichText::new("Export:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::SaveDisk),
                                                        "Export",
                                                        false,
                                                        theme.accent_blue,
                                                        "Export .eigen.json workspace",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_archive = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Cube3D),
                                                        "IFCX",
                                                        false,
                                                        theme.accent_blue,
                                                        "Export georeferenced IFCX (IFC 5 JSON ECS) site layer",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_ifcx = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Globe),
                                                        "Earth .ifcx",
                                                        false,
                                                        theme.accent_green,
                                                        "Export composed Living Earth Globe + Site IFCX graph",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_earth_ifcx = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Document),
                                                        "Audit .md",
                                                        false,
                                                        theme.accent_green,
                                                        "Save Engineering Audit Markdown note to Vault",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.save_audit_note = true;
                                                    }
                                                },
                                            );
                                        } else {
                                            Self::glass_subpill_group(
                                                ui,
                                                theme,
                                                9,
                                                Margin::symmetric(7, 3),
                                                col_w,
                                                |ui| {
                                                    ui.label(
                                                        RichText::new("IFCX Diff & Dossier:")
                                                            .size(9.3)
                                                            .strong()
                                                            .color(theme.text_muted),
                                                    );
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Camera),
                                                        "Snapshot Base",
                                                        false,
                                                        theme.accent_blue,
                                                        "Capture baseline .ifcx snapshot for layer visual diffing",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.capture_ifcx_baseline = true;
                                                    }
                                                    let diff_active = universal_canvas.ifcx_diff_overlay_active
                                                        || self.ifcx_diff_drawer_open;
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Layers),
                                                        "Visual Diff",
                                                        diff_active,
                                                        theme.accent_orange,
                                                        "Open Multi-Layer IFCX visual diff inspector & 3D delta highlighting",
                                                    )
                                                    .clicked()
                                                    {
                                                        universal_canvas.ifcx_diff_overlay_active =
                                                            !universal_canvas.ifcx_diff_overlay_active;
                                                        self.toggle_exclusive_drawer("ifcx_diff");
                                                        actions.run_ifcx_visual_diff = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Cube3D),
                                                        "Diff .ifcx",
                                                        false,
                                                        theme.accent_green,
                                                        "Export non-destructive eigengraph::ifcx::diff_manifest .ifcx layer",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_ifcx_diff_manifest = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Printer),
                                                        "Vector Dossier .svg",
                                                        false,
                                                        theme.accent_green,
                                                        "Export publication-grade 100% Pure-Vector SVG engineering dossier",
                                                    )
                                                    .clicked()
                                                    {
                                                        actions.export_vector_dossier_svg = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Shield),
                                                        "Gov / Merge",
                                                        self.collab_drawer_open && self.collab_drawer_tab == 0,
                                                        theme.accent_blue,
                                                        "Open SHA-256 Checkpoints, Sign-Off & 3-Way IFCX Merge drawer",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.open_collab_drawer(0);
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::ChartLine),
                                                        "Trade / Permit",
                                                        self.collab_drawer_open && self.collab_drawer_tab == 2,
                                                        theme.accent_green,
                                                        "Open 6-Scenario Trade-Study Matrix & 10-Clause Permit Compliance Annex",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.open_collab_drawer(2);
                                                        actions.run_trade_study =
                                                            Some(universal_canvas.trade_study_config.objective);
                                                        actions.run_permit_compliance = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Wrench),
                                                        "Field QA/QC",
                                                        self.field_qaqc_drawer_open,
                                                        theme.accent_blue,
                                                        "Open Field QA/QC Waypoints, 1D Rebar BBS Cut-List & Sheet S-103",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.open_field_qaqc_drawer(0);
                                                        actions.run_field_fabrication =
                                                            Some(universal_canvas.field_fab_config.stock_length_m);
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Bolt),
                                                        "EigenFlow",
                                                        self.eigenflow_drawer_open,
                                                        theme.accent_green,
                                                        "Open EigenFlow Non-Linear Studio (Woodbury P1->P2, Pushover ADRS & Sheet S-104)",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.open_eigenflow_drawer(0);
                                                        actions.run_eigenflow_studio = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::AlertTriangle),
                                                        "Blast / Fire",
                                                        self.extreme_events_drawer_open,
                                                        theme.accent_orange,
                                                        "Open Extreme Events Studio (Blast UFC 3-340-02 & Fire ISO 834 + Sheet S-105)",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.open_extreme_events_drawer(0);
                                                        actions.run_extreme_events = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Shield),
                                                        "Isolation",
                                                        self.isolation_drawer_open,
                                                        theme.accent_blue,
                                                        "Open Seismic Base Isolation & Damper Studio (ASCE 7-22 Ch. 17-18 / SNI 1726 + Sheet S-106)",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.open_isolation_drawer(0);
                                                        actions.run_isolation_studio = true;
                                                    }
                                                    if Self::glass_pill_button(
                                                        ui,
                                                        theme,
                                                        Some(AppIcon::Wind),
                                                        "Wind / Aero",
                                                        self.wind_drawer_open,
                                                        theme.accent_blue,
                                                        "Open Wind Aeroelasticity, Facade Pressure & ISO 10137 Comfort Studio (+ Sheet S-107)",
                                                    )
                                                    .clicked()
                                                    {
                                                        self.open_wind_drawer(0);
                                                        actions.run_wind_studio = true;
                                                    }
                                                },
                                            );
                                        }
                                    });
                                }
                            };
                            render_tab_body(ui, deck_w);
                        });
                        });
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    }

                            // Resting 3-Row Liquid Glass Dynamic Island Base Dock
                            // Row 1: Balanced Left (Vault + Context) / Right (Model + Mode + History + Collapse)
                            if is_mobile {
                                ui.horizontal_wrapped(|ui| {
                                    ui.set_max_width(cap_w);
                                    self.render_dynamic_island_row1_left(
                                        ui,
                                        theme,
                                        vault_status.connected,
                                        &vault_status.vault_name,
                                        true,
                                        &mut actions,
                                    );
                                    self.render_dynamic_island_row1_right(
                                        ui,
                                        theme,
                                        true,
                                        should_expand,
                                        false,
                                    );
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    self.render_dynamic_island_row1_left(
                                        ui,
                                        theme,
                                        vault_status.connected,
                                        &vault_status.vault_name,
                                        false,
                                        &mut actions,
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        self.render_dynamic_island_row1_right(
                                            ui,
                                            theme,
                                            false,
                                            should_expand,
                                            true,
                                        );
                                    });
                                });
                            }

                            ui.add_space(5.0);

                            // Row 2: Grouped Glass Quick-Draw Strip with subtle category dividers
                            Self::glass_subpill_group(
                                ui,
                                theme,
                                11,
                                Margin::symmetric(7, 3),
                                cap_w,
                                |ui| {
                                    for tool in SpatialManualTool::ALL {
                                        let active = tool.is_active(
                                            self.active_canvas_mode,
                                            universal_canvas,
                                            build_view,
                                        );
                                        let accent = if *tool == SpatialManualTool::OrbitPan {
                                            theme.accent_blue
                                        } else {
                                            theme.accent_orange
                                        };
                                        if Self::glass_pill_button(
                                            ui,
                                            theme,
                                            Some(tool.icon()),
                                            tool.short_label(),
                                            active,
                                            accent,
                                            tool.tooltip(),
                                        )
                                        .clicked()
                                        {
                                            let sub =
                                                tool.apply(self, universal_canvas, build_view);
                                            actions.merge(sub);
                                            if !self.mobile_deck_pinned {
                                                self.collapse_dynamic_island();
                                            }
                                        }
                                        if matches!(
                                            tool,
                                            SpatialManualTool::OrbitPan
                                                | SpatialManualTool::Slab
                                                | SpatialManualTool::Window
                                        ) {
                                            ui.separator();
                                        }
                                    }
                                },
                            );

                            // Unified Active Lens & Manual Authoring Controls (inside Dynamic Island)
                            self.render_active_lens_and_manual_tool_strip(
                                ui,
                                theme,
                                universal_canvas,
                                build_view,
                                &mut actions,
                                cap_w,
                            );

                            // Row 3: Attached `@Context` Chips (if any)
                            if !self.attached_contexts.is_empty() {
                                ui.add_space(5.0);
                                let mut detach_key: Option<String> = None;
                                ui.horizontal_wrapped(|ui| {
                                    for att in &self.attached_contexts {
                                        let lbl = att.chip_label(
                                            &build_view.bim_bus.project,
                                            self.selected_target,
                                        );
                                        let key = att.dedup_key();
                                        if icons::icon_button(
                                            ui,
                                            att.icon(),
                                            format!("{lbl} ×"),
                                            9.8,
                                            theme.accent_blue,
                                            theme.accent_blue.gamma_multiply(0.14),
                                            Stroke::new(
                                                0.8,
                                                theme.accent_blue.gamma_multiply(0.45),
                                            ),
                                            CornerRadius::same(10),
                                            Vec2::new(0.0, 20.0),
                                        )
                                        .on_hover_text("Click to detach context")
                                        .clicked()
                                        {
                                            detach_key = Some(key);
                                        }
                                    }
                                });
                                if let Some(k) = detach_key {
                                    self.detach_context(&k);
                                }
                            }

                            ui.add_space(5.0);

                            // Row 4: Frosted Coastal Teal Glass Prompt & `/` Slash Command Composer
                            let (composer_fill, composer_stroke) = if theme.is_dark() {
                                (
                                    Color32::from_rgba_unmultiplied(43, 73, 99, 148),
                                    if self.composer_had_focus {
                                        Color32::from_rgba_unmultiplied(208, 235, 225, 195)
                                    } else {
                                        Color32::from_rgba_unmultiplied(91, 146, 165, 118)
                                    },
                                )
                            } else {
                                (
                                    Color32::from_rgba_unmultiplied(232, 246, 240, 218),
                                    if self.composer_had_focus {
                                        theme.accent_blue.gamma_multiply(0.85)
                                    } else {
                                        Color32::from_rgba_unmultiplied(43, 73, 99, 88)
                                    },
                                )
                            };
                            Frame::new()
                                .fill(composer_fill)
                                .corner_radius(CornerRadius::same(12))
                                .stroke(Stroke::new(1.0, composer_stroke))
                                .inner_margin(Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    let row_w = (cap_w - 22.0).max(160.0);
                                    ui.set_width(row_w);
                                    ui.horizontal(|ui| {
                                        if let Some(mention) =
                                            self.active_context_mention(&build_view.bim_bus.project)
                                        {
                                            if !is_mobile {
                                                if icons::icon_button(
                                                    ui,
                                                    AppIcon::Target,
                                                    mention,
                                                    9.5,
                                                    theme.accent_blue,
                                                    theme.accent_blue.gamma_multiply(0.14),
                                                    Stroke::NONE,
                                                    CornerRadius::same(8),
                                                    Vec2::new(0.0, 20.0),
                                                )
                                                .on_hover_text("Attach selected canvas item as @Context")
                                                .clicked()
                                                {
                                                    self.attach_context(
                                                        AiContextAttachment::SelectedElement,
                                                    );
                                                }
                                            }
                                        }

                                        let bare_enter = self.composer_had_focus
                                            && !slash_mode
                                            && ui.ctx().input(|i| !i.modifiers.shift)
                                            && ui.ctx().input_mut(|i| {
                                                i.consume_key(
                                                    egui::Modifiers::NONE,
                                                    egui::Key::Enter,
                                                )
                                            });

                                        let trailing_reserve = 66.0;
                                        let input_w =
                                            (ui.available_width() - trailing_reserve).max(60.0);
                                        let edit_resp = ui.add(
                                            egui::TextEdit::singleline(&mut self.command_input)
                                                .desired_width(input_w)
                                                .frame(Frame::NONE)
                                                .hint_text(
                                                    "Ask EigenAI, @ context, or / commands...",
                                                ),
                                        );
                                        self.composer_had_focus = edit_resp.has_focus();
                                        if std::mem::take(&mut self.request_focus_command) {
                                            edit_resp.request_focus();
                                        }

                                        let cur_len = self.command_input.len();
                                        if cur_len > self.last_command_len
                                            && self.command_input.ends_with('@')
                                        {
                                            self.context_picker_open = true;
                                        }
                                        self.last_command_len = cur_len;

                                        let enter_singleline = edit_resp.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter));

                                        if ui
                                            .add_sized(
                                                Vec2::new(24.0, 23.0),
                                                egui::Button::new(
                                                    RichText::new("/")
                                                        .size(11.5)
                                                        .strong()
                                                        .color(theme.text_secondary),
                                                )
                                                .fill(theme.bg_sidebar)
                                                .stroke(Stroke::new(0.8, theme.border_subtle))
                                                .corner_radius(CornerRadius::same(7)),
                                            )
                                            .on_hover_text("Browse slash commands (/earth, /3d, /vault...)")
                                            .clicked()
                                        {
                                            if self.command_input.trim_start().starts_with('/') {
                                                self.command_input.clear();
                                            } else {
                                                self.command_input = "/".to_string();
                                                self.context_picker_open = false;
                                                self.request_focus_command = true;
                                            }
                                        }

                                        let has_text = !self.command_input.trim().is_empty();
                                        let send_clicked = icons::icon_only_button(
                                            ui,
                                            AppIcon::ArrowRight,
                                            Color32::WHITE,
                                            if has_text {
                                                theme.accent_blue
                                            } else {
                                                theme.accent_blue.gamma_multiply(0.45)
                                            },
                                            Stroke::NONE,
                                            CornerRadius::same(9),
                                            Vec2::splat(24.0),
                                            "Execute Command or AI Prompt",
                                        )
                                        .clicked();

                                        if (bare_enter || enter_singleline || send_clicked)
                                            && has_text
                                        {
                                            let submitted = std::mem::take(&mut self.command_input);
                                            self.last_command_len = 0;
                                            let sub = self.execute_spatial_input(
                                                &submitted,
                                                universal_canvas,
                                                build_view,
                                                ai_assistant,
                                                members,
                                                projects,
                                                db_components,
                                            );
                                            actions.merge(sub);
                                        }
                                    });
                                });
                        },
                    );
                    frame_theme.paint_glass_specular_sheen(
                        ui.painter(),
                        island_glass.response.rect,
                        20.0,
                    );
                });
            });

        self.dynamic_island_rect = dock_resp.response.rect;
        self.hud_rects.push(dock_resp.response.rect);

        actions
    }

    #[allow(clippy::too_many_arguments)]
    fn render_ground_motion_replay_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::ground_motion_replay::{
            AsyncReplayJobPhase, GroundMotionReplayConfig, GroundMotionReplayReport,
        };
        use crate::core::nonlinear::PlasticHingeState;

        let rep_opt: Option<GroundMotionReplayReport> =
            universal_canvas.last_ground_motion_replay_report.clone();

        Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
                let dw = (cap_w - 10.0).max(160.0);
                ui.set_width(dw);
                ui.set_max_width(dw);

                ui.horizontal(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Bolt,
                        12.5,
                        theme.accent_orange,
                        "Live Earthquake Replay, Tri-Dir .AT2 & FEMA P-58",
                        11.0,
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
                            CornerRadius::same(6),
                            Vec2::splat(18.0),
                            "Close Ground-Motion Replay drawer",
                        )
                        .clicked()
                        {
                            self.ground_motion_drawer_open = false;
                        }
                    });
                });
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    for (tab_idx, tab_lbl) in [
                        (0_usize, "Time-History"),
                        (1_usize, "Spectrum & Fragility"),
                        (2_usize, "GPU Compute & MC"),
                    ] {
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            tab_lbl,
                            self.ground_motion_drawer_tab == tab_idx,
                            theme.accent_blue,
                            "Switch between 60 FPS time-history waveforms, response spectrum + FEMA P-58 fragility, and WebGPU @compute Monte Carlo",
                        )
                        .clicked()
                        {
                            self.ground_motion_drawer_tab = tab_idx;
                        }
                    }
                });
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    let custom_active = universal_canvas
                        .ground_motion_replay_config
                        .custom_accelerogram
                        .is_some();
                    for rec in GroundMotionReplayConfig::ALL_RECORDS {
                        let active = !custom_active
                            && universal_canvas.ground_motion_replay_config.record == rec;
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            GroundMotionReplayConfig::record_short_label(rec),
                            active,
                            theme.accent_blue,
                            rec.label(),
                        )
                        .clicked()
                        {
                            universal_canvas
                                .ground_motion_replay_config
                                .custom_accelerogram = None;
                            universal_canvas.ground_motion_replay_config.record = rec;
                            self.apply_lens_to_views(
                                SpatialLens::TimeHistoryReplay,
                                build_view,
                                universal_canvas,
                            );
                            actions.run_async_ground_motion_replay = true;
                        }
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        Some(AppIcon::Document),
                        "Load .at2/.csv",
                        false,
                        theme.accent_green,
                        "Upload custom PEER NGA (.at2) or 2/3-column (.csv) earthquake accelerogram",
                    )
                    .clicked()
                    {
                        actions.open_accelerogram_file_picker = true;
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "Sample .at2",
                        false,
                        theme.accent_blue,
                        "Load built-in PEER NGA-West2 Northridge Sylmar .AT2 accelerogram sample",
                    )
                    .clicked()
                    {
                        if let Ok(acc) = crate::core::time_history::parse_custom_accelerogram(
                            crate::core::time_history::sample_peer_at2_text(),
                            "PEER_Sample_Northridge_Sylmar.at2",
                        ) {
                            universal_canvas
                                .ground_motion_replay_config
                                .custom_accelerogram = Some(acc);
                            self.apply_lens_to_views(
                                SpatialLens::TimeHistoryReplay,
                                build_view,
                                universal_canvas,
                            );
                            actions.run_async_ground_motion_replay = true;
                        }
                    }
                    if let Some(ref custom) =
                        universal_canvas.ground_motion_replay_config.custom_accelerogram
                    {
                        let custom_name = custom.name.clone();
                        icons::pill_badge(
                            ui,
                            AppIcon::Document,
                            format!("{custom_name} ({:.2}g)", custom.pga_x_g),
                            theme.accent_green,
                        );
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            "×",
                            false,
                            theme.accent_red,
                            "Clear custom accelerogram and revert to built-in benchmark suite",
                        )
                        .clicked()
                        {
                            universal_canvas
                                .ground_motion_replay_config
                                .custom_accelerogram = None;
                            actions.run_async_ground_motion_replay = true;
                        }
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "-0.25x",
                        false,
                        theme.text_secondary,
                        "Decrease ground-motion scale factor by 0.25",
                    )
                    .clicked()
                    {
                        universal_canvas.ground_motion_replay_config.scale_factor =
                            (universal_canvas.ground_motion_replay_config.scale_factor - 0.25)
                                .clamp(0.50, 2.50);
                        actions.run_async_ground_motion_replay = true;
                    }
                    ui.label(
                        RichText::new(format!(
                            "SF {:.2}x",
                            universal_canvas.ground_motion_replay_config.scale_factor
                        ))
                        .size(9.4)
                        .strong()
                        .color(theme.text_primary),
                    );
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "+0.25x",
                        false,
                        theme.text_secondary,
                        "Increase ground-motion scale factor by 0.25",
                    )
                    .clicked()
                    {
                        universal_canvas.ground_motion_replay_config.scale_factor =
                            (universal_canvas.ground_motion_replay_config.scale_factor + 0.25)
                                .clamp(0.50, 2.50);
                        actions.run_async_ground_motion_replay = true;
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        format!(
                            "Hyst: {}",
                            GroundMotionReplayConfig::hysteresis_short_label(
                                universal_canvas
                                    .ground_motion_replay_config
                                    .hysteresis_model
                            )
                        ),
                        false,
                        theme.accent_blue,
                        "Cycle hysteretic constitutive rule (Bouc-Wen / Takeda Pinch / Bilinear)",
                    )
                    .clicked()
                    {
                        universal_canvas
                            .ground_motion_replay_config
                            .hysteresis_model = GroundMotionReplayConfig::next_hysteresis(
                            universal_canvas
                                .ground_motion_replay_config
                                .hysteresis_model,
                        );
                        actions.run_async_ground_motion_replay = true;
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        format!(
                            "Tri-Dir: {}",
                            universal_canvas
                                .ground_motion_replay_config
                                .tridirectional_mode
                                .short_label()
                        ),
                        universal_canvas
                            .ground_motion_replay_config
                            .tridirectional_mode
                            != crate::core::ground_motion_replay::TriDirectionalMode::UniaxialX,
                        theme.accent_blue,
                        "Cycle seismic excitation coupling (1D X / 2D XY / 3D XYZ with vertical axial hinge modulation)",
                    )
                    .clicked()
                    {
                        universal_canvas
                            .ground_motion_replay_config
                            .tridirectional_mode = universal_canvas
                            .ground_motion_replay_config
                            .tridirectional_mode
                            .next();
                        actions.run_async_ground_motion_replay = true;
                    }
                    let match_on = universal_canvas.ground_motion_replay_config.auto_match_sa_t1;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "Match Sa(T1)",
                        match_on,
                        theme.accent_orange,
                        "Auto-scale record spectral acceleration Sa(T1) to match site SNI 1726 / ASCE 7-22 design spectrum",
                    )
                    .clicked()
                    {
                        universal_canvas.ground_motion_replay_config.auto_match_sa_t1 = !match_on;
                        actions.run_async_ground_motion_replay = true;
                    }
                    let frag_on = universal_canvas
                        .ground_motion_replay_config
                        .show_fragility_loss_overlay;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "Fragility 3D",
                        frag_on,
                        theme.accent_red,
                        "Toggle 3D translucent per-story FEMA P-58 repair-loss halo overlay",
                    )
                    .clicked()
                    {
                        universal_canvas
                            .ground_motion_replay_config
                            .show_fragility_loss_overlay = !frag_on;
                    }
                    let ssi_on = universal_canvas
                        .ground_motion_replay_config
                        .use_ssi_period_coupling;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        "SSI",
                        ssi_on,
                        theme.accent_green,
                        "Couple Subsurface SSI lengthened period T1 into NL-THA",
                    )
                    .clicked()
                    {
                        universal_canvas
                            .ground_motion_replay_config
                            .use_ssi_period_coupling = !ssi_on;
                        actions.run_async_ground_motion_replay = true;
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        Some(AppIcon::Bolt),
                        "Run Async",
                        universal_canvas.replay_async_manager.is_running(),
                        theme.accent_orange,
                        "Execute background WebWorker / Multi-Thread StructGNN-CRC + NL-THA",
                    )
                    .clicked()
                    {
                        self.apply_lens_to_views(
                            SpatialLens::TimeHistoryReplay,
                            build_view,
                            universal_canvas,
                        );
                        actions.run_async_ground_motion_replay = true;
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        Some(AppIcon::Cube3D),
                        "Replay .ifcx",
                        false,
                        theme.accent_green,
                        "Export non-destructive eigengraph::analysis::time_history_replay .ifcx",
                    )
                    .clicked()
                    {
                        actions.export_ground_motion_replay_ifcx = true;
                    }
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        Some(AppIcon::Shield),
                        "Fragility .ifcx",
                        false,
                        theme.accent_blue,
                        "Export non-destructive eigengraph::analysis::fema_p58_fragility .ifcx overlay",
                    )
                    .clicked()
                    {
                        actions.export_fragility_loss_ifcx = true;
                    }
                });

                if let AsyncReplayJobPhase::Running {
                    progress,
                    stage_label,
                    backend,
                    ..
                } = &universal_canvas.replay_async_manager.phase
                {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        icons::pill_badge(
                            ui,
                            AppIcon::Bolt,
                            format!("{} • {:.0}%", backend, progress * 100.0),
                            theme.accent_orange,
                        );
                        ui.label(
                            RichText::new(stage_label)
                                .size(9.3)
                                .color(theme.text_secondary),
                        );
                    });
                }

                ui.add_space(4.0);
                if self.ground_motion_drawer_tab == 2 {
                    let max_scroll_h = if is_mobile { 240.0 } else { 310.0 };
                    ScrollArea::vertical()
                        .id_salt("gpu_compute_mc_scroll")
                        .max_height(max_scroll_h)
                        .show(ui, |ui| {
                            let panel_w = (dw - 6.0).max(150.0);
                            ui.set_width(panel_w);
                            self.render_gpu_compute_and_mc_panel(
                                ui,
                                theme,
                                universal_canvas,
                                actions,
                                panel_w,
                                is_mobile,
                            );
                        });
                } else if let Some(ref rep) = rep_opt {
                    ui.horizontal_wrapped(|ui| {
                        let playing = universal_canvas.replay_anim_playing;
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            if playing { "Pause" } else { "Play" },
                            playing,
                            theme.accent_green,
                            "Play or pause 60 FPS replay",
                        )
                        .clicked()
                        {
                            universal_canvas.replay_anim_playing = !playing;
                        }
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            format!("Peak {:.1}s", rep.peak_time_s),
                            false,
                            theme.accent_orange,
                            "Jump cursor to peak drift instant",
                        )
                        .clicked()
                        {
                            universal_canvas.replay_cursor_time_s = rep.peak_time_s;
                            universal_canvas.replay_anim_playing = false;
                        }
                        if Self::glass_pill_button(
                            ui,
                            theme,
                            None,
                            format!(
                                "{:.2}x",
                                universal_canvas.ground_motion_replay_config.playback_speed
                            ),
                            false,
                            theme.text_secondary,
                            "Cycle playback speed (0.25x / 0.5x / 1x / 2x)",
                        )
                        .clicked()
                        {
                            universal_canvas
                                .ground_motion_replay_config
                                .cycle_playback_speed();
                        }
                        ui.spacing_mut().slider_width = (dw - 220.0).clamp(80.0, 260.0);
                        ui.add(
                            egui::Slider::new(
                                &mut universal_canvas.replay_cursor_time_s,
                                0.0..=rep.total_duration_s.max(0.1),
                            )
                            .suffix("s")
                            .fixed_decimals(1),
                        );
                    });

                    ui.add_space(4.0);
                    let max_scroll_h = if is_mobile { 220.0 } else { 290.0 };
                    ScrollArea::vertical()
                        .id_salt("ground_motion_replay_scroll")
                        .max_height(max_scroll_h)
                        .show(ui, |ui| {
                            ui.set_width((dw - 4.0).max(150.0));
                            if self.ground_motion_drawer_tab == 1 {
                                self.paint_spectrum_and_fragility_vector_charts(
                                    ui,
                                    theme,
                                    rep,
                                    (dw - 6.0).max(150.0),
                                );
                            } else {
                                self.paint_ground_motion_vector_charts(
                                    ui,
                                    theme,
                                    rep,
                                    universal_canvas.replay_cursor_time_s,
                                    (dw - 6.0).max(150.0),
                                );
                                ui.add_space(5.0);
                                ui.horizontal_wrapped(|ui| {
                                    icons::pill_badge(
                                        ui,
                                        AppIcon::Shield,
                                        format!(
                                            "Async CRC: {}/{} pruned ({:.0}%, 100% recall, -{:.1}g CO2)",
                                            rep.async_crc.stage1_pruned,
                                            rep.async_crc.total_scenarios,
                                            rep.async_crc.scenario_reduction_pct,
                                            rep.async_crc.green_ai_co2_saved_g
                                        ),
                                        theme.accent_green,
                                    );
                                    icons::pill_badge(
                                        ui,
                                        AppIcon::Building,
                                        format!(
                                            "T1={:.2}s{} • {} • Drift {:.2}% ({})",
                                            rep.fundamental_period_t1_s,
                                            if rep.ssi_coupled { " SSI" } else { "" },
                                            rep.tridirectional_mode.short_label(),
                                            rep.peak_interstory_drift_ratio * 100.0,
                                            if rep.is_drift_compliant { "PASS" } else { ">2%" }
                                        ),
                                        if rep.is_drift_compliant {
                                            theme.accent_blue
                                        } else {
                                            theme.accent_red
                                        },
                                    );
                                    icons::pill_badge(
                                        ui,
                                        AppIcon::Coins,
                                        format!(
                                            "P-58 ${:.0}k ({:.1}% | {}d | {})",
                                            rep.fragility_loss.total_repair_cost_usd * 0.001,
                                            rep.fragility_loss.repair_loss_ratio * 100.0,
                                            rep.fragility_loss.downtime_days.round() as i64,
                                            rep.fragility_loss.achieved_redi_tier.label()
                                        ),
                                        theme.accent_orange,
                                    );
                                    ui.label(
                                        RichText::new(format!("Backend: {}", rep.async_crc.execution_backend))
                                            .size(9.0)
                                            .monospace()
                                            .color(theme.text_muted),
                                    );
                                });
                                if !rep.peak_hinges.is_empty() {
                                    ui.add_space(3.0);
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            RichText::new("Critical Hinges:")
                                                .size(9.2)
                                                .strong()
                                                .color(theme.text_secondary),
                                        );
                                        for h in rep.peak_hinges.iter().take(6) {
                                            let col = match h.state {
                                                PlasticHingeState::Elastic => theme.accent_green,
                                                PlasticHingeState::ImmediateOccupancy => {
                                                    Color32::from_rgb(10, 132, 255)
                                                }
                                                PlasticHingeState::LifeSafety => {
                                                    Color32::from_rgb(255, 159, 10)
                                                }
                                                PlasticHingeState::CollapsePrevention => {
                                                    Color32::from_rgb(255, 69, 58)
                                                }
                                                PlasticHingeState::Mechanism => {
                                                    Color32::from_rgb(191, 90, 242)
                                                }
                                            };
                                            let tag = match h.state {
                                                PlasticHingeState::Elastic => "EL",
                                                PlasticHingeState::ImmediateOccupancy => "IO",
                                                PlasticHingeState::LifeSafety => "LS",
                                                PlasticHingeState::CollapsePrevention => "CP",
                                                PlasticHingeState::Mechanism => "MECH",
                                            };
                                            icons::pill_badge(
                                                ui,
                                                AppIcon::Target,
                                                format!(
                                                    "{}.{} {} ({:.2})",
                                                    h.element_name,
                                                    if h.joint_end == 0 { "I" } else { "J" },
                                                    tag,
                                                    h.dcr_dynamic
                                                ),
                                                col,
                                            );
                                        }
                                    });
                                }
                            }
                        });
                } else {
                    ui.label(
                        RichText::new("Click 'Run Async', 'Sample .at2', or run /replay to stream Pure WebGPU & StructGNN-CRC nonlinear time-history.")
                            .size(9.5)
                            .color(theme.text_muted),
                    );
                }
            });
    }

    fn paint_ground_motion_vector_charts(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        rep: &crate::core::ground_motion_replay::GroundMotionReplayReport,
        cursor_time_s: f64,
        avail_w: f32,
    ) {
        let two_col = avail_w >= 540.0;
        let cw = if two_col {
            ((avail_w - 8.0) * 0.5).max(180.0)
        } else {
            avail_w.max(150.0)
        };
        let ch = if two_col { 76.0_f32 } else { 66.0_f32 };

        let draw_accel = |ui: &mut Ui| {
            let (resp, p) = ui.allocate_painter(Vec2::new(cw, ch), egui::Sense::hover());
            let r = resp.rect;
            p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
            p.text(
                egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                Align2::LEFT_TOP,
                format!("Accelerogram ag(t) • PGA {:.2}g", rep.scaled_pga_g),
                egui::FontId::proportional(8.8),
                theme.text_muted,
            );
            let max_a = rep
                .frames
                .iter()
                .map(|f| f.ground_accel_g.abs())
                .fold(0.10_f64, f64::max);
            let t_max = rep.total_duration_s.max(1.0);
            let pad_t = 16.0_f32;
            let pad_b = 6.0_f32;
            let mid_y = r.top() + pad_t + (r.height() - pad_t - pad_b) * 0.5;
            let half_h = ((r.height() - pad_t - pad_b) * 0.46).max(8.0);
            p.line_segment(
                [egui::Pos2::new(r.left() + 6.0, mid_y), egui::Pos2::new(r.right() - 6.0, mid_y)],
                Stroke::new(0.8, theme.border_card),
            );
            let step = (rep.frames.len() / 160).max(1);
            let pts: Vec<egui::Pos2> = rep
                .frames
                .iter()
                .step_by(step)
                .map(|f| {
                    let nx = (f.time_s / t_max).clamp(0.0, 1.0) as f32;
                    let ny = (f.ground_accel_g / max_a).clamp(-1.0, 1.0) as f32;
                    egui::Pos2::new(
                        r.left() + 6.0 + nx * (r.width() - 12.0),
                        mid_y - ny * half_h,
                    )
                })
                .collect();
            for win in pts.windows(2) {
                p.line_segment([win[0], win[1]], Stroke::new(1.2, theme.accent_blue));
            }
            let cx = r.left() + 6.0 + ((cursor_time_s / t_max).clamp(0.0, 1.0) as f32) * (r.width() - 12.0);
            p.line_segment(
                [egui::Pos2::new(cx, r.top() + pad_t), egui::Pos2::new(cx, r.bottom() - pad_b)],
                Stroke::new(1.4, theme.accent_orange),
            );
        };

        let draw_drift = |ui: &mut Ui| {
            let (resp, p) = ui.allocate_painter(Vec2::new(cw, ch), egui::Sense::hover());
            let r = resp.rect;
            p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
            p.text(
                egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                Align2::LEFT_TOP,
                format!("Roof Drift % (Peak {:.2}% vs 2.0% Limit)", rep.peak_interstory_drift_ratio * 100.0),
                egui::FontId::proportional(8.8),
                theme.text_muted,
            );
            let max_d = (rep.peak_roof_drift_ratio * 100.0).max(2.2);
            let t_max = rep.total_duration_s.max(1.0);
            let pad_t = 16.0_f32;
            let pad_b = 6.0_f32;
            let mid_y = r.top() + pad_t + (r.height() - pad_t - pad_b) * 0.5;
            let half_h = ((r.height() - pad_t - pad_b) * 0.45).max(8.0);
            let lim_y_off = ((2.0 / max_d).clamp(0.0, 1.0) as f32) * half_h;
            for sy in [mid_y - lim_y_off, mid_y + lim_y_off] {
                p.line_segment(
                    [egui::Pos2::new(r.left() + 6.0, sy), egui::Pos2::new(r.right() - 6.0, sy)],
                    Stroke::new(0.8, theme.accent_red.gamma_multiply(0.65)),
                );
            }
            let step = (rep.frames.len() / 160).max(1);
            let pts: Vec<egui::Pos2> = rep
                .frames
                .iter()
                .step_by(step)
                .map(|f| {
                    let nx = (f.time_s / t_max).clamp(0.0, 1.0) as f32;
                    let ny = ((f.roof_drift_ratio * 100.0) / max_d).clamp(-1.0, 1.0) as f32;
                    egui::Pos2::new(
                        r.left() + 6.0 + nx * (r.width() - 12.0),
                        mid_y - ny * half_h,
                    )
                })
                .collect();
            for win in pts.windows(2) {
                p.line_segment([win[0], win[1]], Stroke::new(1.3, theme.accent_green));
            }
            let cx = r.left() + 6.0 + ((cursor_time_s / t_max).clamp(0.0, 1.0) as f32) * (r.width() - 12.0);
            p.line_segment(
                [egui::Pos2::new(cx, r.top() + pad_t), egui::Pos2::new(cx, r.bottom() - pad_b)],
                Stroke::new(1.4, theme.accent_orange),
            );
        };

        let draw_hysteresis = |ui: &mut Ui, hw: f32| {
            let (resp, p) = ui.allocate_painter(Vec2::new(hw, ch), egui::Sense::hover());
            let r = resp.rect;
            p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
            p.text(
                egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                Align2::LEFT_TOP,
                format!(
                    "Hysteresis Vb ({:.0} kN) vs Roof Disp ({:.0} mm)",
                    rep.peak_base_shear_kn, rep.peak_roof_disp_mm
                ),
                egui::FontId::proportional(8.8),
                theme.text_muted,
            );
            let max_u = rep.peak_roof_disp_mm.max(5.0);
            let max_v = rep.peak_base_shear_kn.max(50.0);
            let pad_t = 16.0_f32;
            let pad_b = 6.0_f32;
            let mid_x = r.center().x;
            let mid_y = r.top() + pad_t + (r.height() - pad_t - pad_b) * 0.5;
            let half_w = ((r.width() - 16.0) * 0.48).max(20.0);
            let half_h = ((r.height() - pad_t - pad_b) * 0.45).max(8.0);
            p.line_segment(
                [egui::Pos2::new(r.left() + 8.0, mid_y), egui::Pos2::new(r.right() - 8.0, mid_y)],
                Stroke::new(0.7, theme.border_card),
            );
            p.line_segment(
                [egui::Pos2::new(mid_x, r.top() + pad_t), egui::Pos2::new(mid_x, r.bottom() - pad_b)],
                Stroke::new(0.7, theme.border_card),
            );
            let step = (rep.frames.len() / 220).max(1);
            let pts: Vec<egui::Pos2> = rep
                .frames
                .iter()
                .step_by(step)
                .map(|f| {
                    let nx = (f.roof_disp_mm / max_u).clamp(-1.0, 1.0) as f32;
                    let ny = (f.base_shear_kn / max_v).clamp(-1.0, 1.0) as f32;
                    egui::Pos2::new(mid_x + nx * half_w, mid_y - ny * half_h)
                })
                .collect();
            for win in pts.windows(2) {
                p.line_segment(
                    [win[0], win[1]],
                    Stroke::new(1.1, Color32::from_rgb(91, 146, 165)),
                );
            }
            let cur = rep.sample_interpolated(cursor_time_s);
            let cur_pt = egui::Pos2::new(
                mid_x + (cur.roof_disp_mm / max_u).clamp(-1.0, 1.0) as f32 * half_w,
                mid_y - (cur.base_shear_kn / max_v).clamp(-1.0, 1.0) as f32 * half_h,
            );
            p.circle_filled(cur_pt, 3.8, theme.accent_orange);
        };

        if two_col {
            ui.horizontal(|ui| {
                draw_accel(ui);
                draw_drift(ui);
            });
            ui.add_space(4.0);
            draw_hysteresis(ui, avail_w.max(180.0));
        } else {
            ui.vertical(|ui| {
                draw_accel(ui);
                ui.add_space(3.0);
                draw_drift(ui);
                ui.add_space(3.0);
                draw_hysteresis(ui, cw);
            });
        }
    }

    fn paint_spectrum_and_fragility_vector_charts(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        rep: &crate::core::ground_motion_replay::GroundMotionReplayReport,
        avail_w: f32,
    ) {
        let two_col = avail_w >= 540.0;
        let cw = if two_col {
            ((avail_w - 8.0) * 0.5).max(180.0)
        } else {
            avail_w.max(150.0)
        };
        let ch = if two_col { 86.0_f32 } else { 76.0_f32 };

        let draw_spectrum = |ui: &mut Ui| {
            let (resp, p) = ui.allocate_painter(Vec2::new(cw, ch), egui::Sense::hover());
            let r = resp.rect;
            p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
            p.text(
                egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                Align2::LEFT_TOP,
                format!(
                    "5% Spectrum Sa(T1={:.2}s) Rec {:.2}g vs Des {:.2}g ({:.2}x)",
                    rep.fundamental_period_t1_s,
                    rep.sa_record_t1_g,
                    rep.sa_design_t1_g,
                    rep.sa_match_ratio
                ),
                egui::FontId::proportional(8.6),
                theme.text_muted,
            );
            let t_max = rep
                .response_spectrum
                .iter()
                .map(|pt| pt.period_s)
                .fold(4.0_f64, f64::max)
                .max(1.0);
            let max_sa = rep
                .response_spectrum
                .iter()
                .map(|pt| pt.sa_record_g.max(pt.sa_design_g))
                .fold(0.5_f64, f64::max)
                .max(0.2);
            let pad_t = 17.0_f32;
            let pad_b = 7.0_f32;
            let h = (r.height() - pad_t - pad_b).max(12.0);
            let w = (r.width() - 12.0).max(20.0);
            let to_xy = |t: f64, sa: f64| -> egui::Pos2 {
                egui::Pos2::new(
                    r.left() + 6.0 + (t / t_max).clamp(0.0, 1.0) as f32 * w,
                    r.bottom() - pad_b - (sa / max_sa).clamp(0.0, 1.0) as f32 * h,
                )
            };
            let des_pts: Vec<egui::Pos2> = rep
                .response_spectrum
                .iter()
                .map(|pt| to_xy(pt.period_s, pt.sa_design_g))
                .collect();
            for win in des_pts.windows(2) {
                p.line_segment([win[0], win[1]], Stroke::new(1.2, theme.accent_orange));
            }
            let rec_pts: Vec<egui::Pos2> = rep
                .response_spectrum
                .iter()
                .map(|pt| to_xy(pt.period_s, pt.sa_record_g))
                .collect();
            for win in rec_pts.windows(2) {
                p.line_segment([win[0], win[1]], Stroke::new(1.5, theme.accent_blue));
            }
            let t1_x = r.left() + 6.0 + (rep.fundamental_period_t1_s / t_max).clamp(0.0, 1.0) as f32 * w;
            p.line_segment(
                [egui::Pos2::new(t1_x, r.top() + pad_t), egui::Pos2::new(t1_x, r.bottom() - pad_b)],
                Stroke::new(1.1, theme.accent_green),
            );
        };

        let draw_fragility = |ui: &mut Ui| {
            let (resp, p) = ui.allocate_painter(Vec2::new(cw, ch), egui::Sense::hover());
            let r = resp.rect;
            p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
            p.text(
                egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                Align2::LEFT_TOP,
                format!(
                    "FEMA P-58 Fragility P(DS|Sa) @ Sa(T1)={:.2}g (Pcol {:.1}%)",
                    rep.fragility_loss.sa_t1_g,
                    rep.fragility_loss.collapse_probability * 100.0
                ),
                egui::FontId::proportional(8.6),
                theme.text_muted,
            );
            let sa_max = rep
                .fragility_loss
                .fragility_curves
                .iter()
                .map(|c| c.sa_g)
                .fold(2.0_f64, f64::max)
                .max(0.5);
            let pad_t = 17.0_f32;
            let pad_b = 7.0_f32;
            let h = (r.height() - pad_t - pad_b).max(12.0);
            let w = (r.width() - 12.0).max(20.0);
            let to_pt = |sa: f64, prob: f64| -> egui::Pos2 {
                egui::Pos2::new(
                    r.left() + 6.0 + (sa / sa_max).clamp(0.0, 1.0) as f32 * w,
                    r.bottom() - pad_b - prob.clamp(0.0, 1.0) as f32 * h,
                )
            };
            let curves = [
                (
                    theme.accent_blue,
                    rep.fragility_loss
                        .fragility_curves
                        .iter()
                        .map(|c| to_pt(c.sa_g, c.p_io))
                        .collect::<Vec<_>>(),
                ),
                (
                    theme.accent_orange,
                    rep.fragility_loss
                        .fragility_curves
                        .iter()
                        .map(|c| to_pt(c.sa_g, c.p_ls))
                        .collect::<Vec<_>>(),
                ),
                (
                    theme.accent_red,
                    rep.fragility_loss
                        .fragility_curves
                        .iter()
                        .map(|c| to_pt(c.sa_g, c.p_cp))
                        .collect::<Vec<_>>(),
                ),
                (
                    Color32::from_rgb(191, 90, 242),
                    rep.fragility_loss
                        .fragility_curves
                        .iter()
                        .map(|c| to_pt(c.sa_g, c.p_collapse))
                        .collect::<Vec<_>>(),
                ),
            ];
            for (col, pts) in curves {
                for win in pts.windows(2) {
                    p.line_segment([win[0], win[1]], Stroke::new(1.3, col));
                }
            }
            let sa_x =
                r.left() + 6.0 + (rep.fragility_loss.sa_t1_g / sa_max).clamp(0.0, 1.0) as f32 * w;
            p.line_segment(
                [egui::Pos2::new(sa_x, r.top() + pad_t), egui::Pos2::new(sa_x, r.bottom() - pad_b)],
                Stroke::new(1.2, theme.accent_green),
            );
        };

        let draw_story_loss_bars = |ui: &mut Ui, bw: f32| {
            let n_rows = rep.fragility_loss.story_losses.len().clamp(1, 6);
            let bh = 20.0 + (n_rows as f32) * 11.5;
            let (resp, p) = ui.allocate_painter(Vec2::new(bw, bh), egui::Sense::hover());
            let r = resp.rect;
            p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
            p.text(
                egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                Align2::LEFT_TOP,
                format!(
                    "Story Repair Loss Breakdown (Total ${:.0}k • {:.1}% RV)",
                    rep.fragility_loss.total_repair_cost_usd * 0.001,
                    rep.fragility_loss.repair_loss_ratio * 100.0
                ),
                egui::FontId::proportional(8.6),
                theme.text_muted,
            );
            let max_lr = rep
                .fragility_loss
                .story_losses
                .iter()
                .map(|s| s.loss_ratio)
                .fold(0.05_f64, f64::max)
                .max(0.02);
            let bar_x0 = r.left() + 38.0;
            let bar_max_w = (r.width() - 122.0).max(24.0);
            for (i, st) in rep.fragility_loss.story_losses.iter().take(6).enumerate() {
                let y = r.top() + 18.0 + (i as f32) * 11.2;
                let frac = (st.loss_ratio / max_lr).clamp(0.03, 1.0) as f32;
                let col = if st.loss_ratio >= 0.15 {
                    theme.accent_red
                } else if st.loss_ratio >= 0.05 {
                    theme.accent_orange
                } else {
                    theme.accent_green
                };
                p.text(
                    egui::Pos2::new(r.left() + 6.0, y + 4.5),
                    Align2::LEFT_CENTER,
                    format!("L{:02}", st.story_number),
                    egui::FontId::monospace(8.4),
                    theme.text_secondary,
                );
                p.rect_filled(
                    Rect::from_min_size(egui::Pos2::new(bar_x0, y), Vec2::new(bar_max_w * frac, 7.5)),
                    CornerRadius::same(2),
                    col,
                );
                p.text(
                    egui::Pos2::new(bar_x0 + bar_max_w + 5.0, y + 4.0),
                    Align2::LEFT_CENTER,
                    format!("${:.0}k ({:.1}%)", st.repair_cost_usd * 0.001, st.loss_ratio * 100.0),
                    egui::FontId::monospace(8.2),
                    theme.text_muted,
                );
            }
        };

        if two_col {
            ui.horizontal(|ui| {
                draw_spectrum(ui);
                draw_fragility(ui);
            });
            ui.add_space(4.0);
            draw_story_loss_bars(ui, avail_w.max(180.0));
        } else {
            ui.vertical(|ui| {
                draw_spectrum(ui);
                ui.add_space(3.0);
                draw_fragility(ui);
                ui.add_space(3.0);
                draw_story_loss_bars(ui, cw);
            });
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            icons::pill_badge(
                ui,
                AppIcon::Coins,
                format!(
                    "Repair ${:.0}k • Econ ${:.0}k",
                    rep.fragility_loss.total_repair_cost_usd * 0.001,
                    rep.fragility_loss.total_economic_loss_usd * 0.001
                ),
                theme.accent_orange,
            );
            icons::pill_badge(
                ui,
                AppIcon::Shield,
                format!(
                    "REDi {} • {:.0}d Downtime • {:.1}% Pcol",
                    rep.fragility_loss.achieved_redi_tier.label(),
                    rep.fragility_loss.downtime_days,
                    rep.fragility_loss.collapse_probability * 100.0
                ),
                if rep.fragility_loss.collapse_probability < 0.05 {
                    theme.accent_green
                } else {
                    theme.accent_red
                },
            );
            icons::pill_badge(
                ui,
                AppIcon::Leaf,
                format!(
                    "Repair Carbon {:.1}t CO2e",
                    rep.fragility_loss.repair_embodied_carbon_kg * 0.001
                ),
                theme.accent_blue,
            );
        });
    }

    fn render_gpu_compute_and_mc_panel(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        avail_w: f32,
        is_mobile: bool,
    ) {
        ui.horizontal_wrapped(|ui| {
            let gp_on = universal_canvas.monte_carlo_config.gpu_particles_enabled;
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Waves),
                if gp_on { "GPU Particles ON" } else { "GPU Particles OFF" },
                gp_on,
                theme.accent_blue,
                "Toggle 60 FPS WebGPU @compute multi-hazard seismic wavefield & Venturi obstacle particles",
            )
            .clicked()
            {
                universal_canvas.monte_carlo_config.gpu_particles_enabled = !gp_on;
                if universal_canvas.monte_carlo_config.gpu_particles_enabled
                    && !universal_canvas.district_hazard.any_active()
                {
                    universal_canvas.district_hazard.seismic_wave_active = true;
                    universal_canvas.district_hazard.venturi_wind_active = true;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                None,
                format!(
                    "Count: {}",
                    universal_canvas.monte_carlo_config.normalized_particle_count()
                ),
                false,
                theme.text_secondary,
                "Cycle @compute particle budget (512 / 1024 / 2048 / 4096)",
            )
            .clicked()
            {
                universal_canvas.monte_carlo_config.cycle_particle_count();
            }
            ui.separator();
            for n in [1024_u32, 4096, 16_384] {
                let lbl = format!("Run {}k MC", n / 1024);
                let act = universal_canvas
                    .last_monte_carlo_report
                    .as_ref()
                    .is_some_and(|r| r.sample_count == n);
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Bolt),
                    lbl,
                    act,
                    theme.accent_orange,
                    "Run FEMA P-58 Monte Carlo realizations on WebGPU @compute (CPU parity fallback in CI)",
                )
                .clicked()
                {
                    universal_canvas.monte_carlo_config.sample_count = n;
                    actions.run_gpu_monte_carlo = Some(n);
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Cube3D),
                "MC .ifcx",
                false,
                theme.accent_green,
                "Export non-destructive eigengraph::analysis::gpu_monte_carlo_loss .ifcx overlay",
            )
            .clicked()
            {
                actions.export_monte_carlo_ifcx = true;
            }
        });

        ui.add_space(4.0);
        if let Some(ref mc) = universal_canvas.last_monte_carlo_report {
            ui.horizontal_wrapped(|ui| {
                icons::pill_badge(
                    ui,
                    AppIcon::Bolt,
                    format!("{} ({}x)", mc.backend.short_label(), mc.sample_count),
                    theme.accent_blue,
                );
                icons::pill_badge(
                    ui,
                    AppIcon::Coins,
                    format!(
                        "Mean ${:.0}k | P50 ${:.0}k | PML90 ${:.0}k | CVaR95 ${:.0}k",
                        mc.mean_loss_usd * 0.001,
                        mc.p50_loss_usd * 0.001,
                        mc.p90_loss_usd * 0.001,
                        mc.cvar95_loss_usd * 0.001,
                    ),
                    theme.accent_orange,
                );
                icons::pill_badge(
                    ui,
                    AppIcon::Shield,
                    format!(
                        "P90 {:.0}d | P(Col) {:.1}% | RedTag {:.1}%",
                        mc.p90_downtime_days, mc.collapse_prob_pct, mc.red_tag_prob_pct
                    ),
                    if mc.collapse_prob_pct < 2.0 {
                        theme.accent_green
                    } else {
                        theme.accent_red
                    },
                );
            });

            ui.add_space(4.0);
            let two_col = !is_mobile && avail_w >= 560.0;
            let cw = if two_col {
                ((avail_w - 10.0) * 0.5).floor().max(180.0)
            } else {
                avail_w.max(150.0)
            };
            let ch = if two_col { 92.0_f32 } else { 82.0_f32 };

            let draw_exceedance = |ui: &mut Ui| {
                let (resp, p) = ui.allocate_painter(Vec2::new(cw, ch), egui::Sense::hover());
                let r = resp.rect;
                p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
                p.text(
                    egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                    Align2::LEFT_TOP,
                    format!(
                        "Loss Exceedance (1-CDF) • P50 ${:.0}k | PML90 ${:.0}k",
                        mc.p50_loss_usd * 0.001,
                        mc.p90_loss_usd * 0.001
                    ),
                    egui::FontId::proportional(8.5),
                    theme.text_muted,
                );
                if mc.exceedance_curve.len() >= 2 {
                    let max_loss = mc
                        .exceedance_curve
                        .last()
                        .map(|pt| pt.loss_usd)
                        .unwrap_or(mc.p90_loss_usd)
                        .max(1.0);
                    let w = (r.width() - 12.0).max(20.0);
                    let h = (r.height() - 23.0).max(16.0);
                    let pts: Vec<egui::Pos2> = mc
                        .exceedance_curve
                        .iter()
                        .map(|pt| {
                            let x = r.left() + 6.0 + (pt.loss_usd / max_loss).clamp(0.0, 1.0) as f32 * w;
                            let y = r.bottom() - 6.0 - pt.exceedance_prob.clamp(0.0, 1.0) as f32 * h;
                            egui::Pos2::new(x, y)
                        })
                        .collect();
                    p.add(egui::Shape::line(pts, Stroke::new(1.6, theme.accent_orange)));
                    for (val, col) in [
                        (mc.p50_loss_usd, theme.accent_blue),
                        (mc.p90_loss_usd, theme.accent_red),
                    ] {
                        let mx = r.left() + 6.0 + (val / max_loss).clamp(0.0, 1.0) as f32 * w;
                        p.line_segment(
                            [egui::Pos2::new(mx, r.top() + 17.0), egui::Pos2::new(mx, r.bottom() - 6.0)],
                            Stroke::new(1.1, col),
                        );
                    }
                }
            };

            let draw_downtime_redi = |ui: &mut Ui| {
                let (resp, p) = ui.allocate_painter(Vec2::new(cw, ch), egui::Sense::hover());
                let r = resp.rect;
                p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
                p.text(
                    egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                    Align2::LEFT_TOP,
                    format!(
                        "Downtime Bins & REDi (P:{:.0}% G:{:.0}% S:{:.0}%)",
                        mc.redi_probs.platinum_pct,
                        mc.redi_probs.gold_pct,
                        mc.redi_probs.silver_pct
                    ),
                    egui::FontId::proportional(8.5),
                    theme.text_muted,
                );
                let max_p = mc
                    .downtime_bins
                    .iter()
                    .map(|b| b.probability_pct)
                    .fold(10.0_f64, f64::max)
                    .max(5.0);
                let n_bins = mc.downtime_bins.len().max(1);
                let w = (r.width() - 14.0).max(20.0);
                let h = (r.height() - 25.0).max(14.0);
                let step_w = w / (n_bins as f32);
                let bar_w = (step_w - 3.0).max(4.0);
                for (bi, bin) in mc.downtime_bins.iter().enumerate() {
                    let frac = (bin.probability_pct / max_p).clamp(0.03, 1.0) as f32;
                    let bh = frac * h;
                    let bx = r.left() + 7.0 + (bi as f32) * step_w;
                    let col = if bin.max_days <= 7.5 {
                        theme.accent_green
                    } else if bin.max_days <= 30.5 {
                        theme.accent_blue
                    } else if bin.max_days <= 60.5 {
                        theme.accent_orange
                    } else {
                        theme.accent_red
                    };
                    p.rect_filled(
                        Rect::from_min_size(egui::Pos2::new(bx, r.bottom() - 6.0 - bh), Vec2::new(bar_w, bh)),
                        CornerRadius::same(2),
                        col,
                    );
                }
            };

            if two_col {
                ui.horizontal(|ui| {
                    draw_exceedance(ui);
                    draw_downtime_redi(ui);
                });
            } else {
                draw_exceedance(ui);
                ui.add_space(4.0);
                draw_downtime_redi(ui);
            }
        } else {
            ui.label(
                RichText::new("Click 'Run 4k MC', '/mc', or toggle 'GPU Particles' to dispatch Pure WebGPU @compute WGSL shaders.")
                    .size(9.5)
                    .color(theme.text_muted),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_shm_digital_twin_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        let avail_w = ui.available_width().min(cap_w - 16.0).max(180.0);
        let max_story = build_view.bim_bus.project.levels.len().saturating_sub(1).max(1);
        let mut config_changed = false;

        ui.horizontal_wrapped(|ui| {
            icons::pill_badge(
                ui,
                AppIcon::Shield,
                "Phase 76 SHM Digital Twin & ATC-20",
                theme.accent_green,
            );
            if let Some(ref rep) = universal_canvas.last_shm_twin_report {
                let [r, g, b] = rep.atc20_placard.badge_color_rgb();
                icons::pill_badge(
                    ui,
                    AppIcon::Target,
                    rep.atc20_placard.label(),
                    Color32::from_rgb(r, g, b),
                );
            }
            for (idx, lbl) in [
                (0usize, "Live Sensors & Modal Shift"),
                (1usize, "ATC-20 Placard Triage"),
                (2usize, "Live Stream & Watchdog"),
            ] {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    lbl,
                    self.shm_drawer_tab == idx,
                    theme.accent_blue,
                    "Switch SHM Digital Twin view",
                )
                .clicked()
                {
                    self.shm_drawer_tab = idx;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close SHM & ATC-20 drawer",
            )
            .clicked()
            {
                self.shm_drawer_open = false;
            }
        });

        ui.add_space(4.0);

        if self.shm_drawer_tab == 0 {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Fault Scenario:").size(9.5).strong().color(theme.text_secondary));
                for &scen in crate::core::ShmFaultScenario::all() {
                    let active = universal_canvas.shm_twin_config.fault_scenario == scen;
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        scen.short_label(),
                        active,
                        if scen == crate::core::ShmFaultScenario::NominalIntact {
                            theme.accent_green
                        } else {
                            theme.accent_orange
                        },
                        scen.description(),
                    )
                    .clicked()
                    {
                        universal_canvas.shm_twin_config.fault_scenario = scen;
                        config_changed = true;
                    }
                }
            });

            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                let mut sev = universal_canvas.shm_twin_config.severity_pct as f32;
                ui.label(RichText::new("Severity %:").size(9.2).color(theme.text_muted));
                ui.spacing_mut().slider_width = if is_mobile { 90.0 } else { 135.0 };
                let resp = ui.add(
                    egui::Slider::new(&mut sev, 0.0..=100.0)
                        .fixed_decimals(0)
                        .suffix("%"),
                );
                if resp.changed() {
                    universal_canvas.shm_twin_config.severity_pct = sev as f64;
                }
                if resp.drag_stopped() {
                    config_changed = true;
                }

                ui.label(
                    RichText::new(format!("Target Story: L{}", universal_canvas.shm_twin_config.target_story))
                        .size(9.2)
                        .color(theme.text_muted),
                );
                if Self::glass_pill_button(ui, theme, None, "-L", false, theme.accent_blue, "Lower target floor").clicked()
                    && universal_canvas.shm_twin_config.target_story > 1
                {
                    universal_canvas.shm_twin_config.target_story -= 1;
                    config_changed = true;
                }
                if Self::glass_pill_button(ui, theme, None, "+L", false, theme.accent_blue, "Raise target floor").clicked()
                    && universal_canvas.shm_twin_config.target_story < max_story
                {
                    universal_canvas.shm_twin_config.target_story += 1;
                    config_changed = true;
                }
            });

            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Bolt),
                    "Evaluate SHM",
                    false,
                    theme.accent_green,
                    "Run 6-DOF modal identification & ATC-20 evaluation",
                )
                .clicked()
                {
                    config_changed = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Document),
                    "Sample CSV",
                    false,
                    theme.accent_blue,
                    "Load deterministic 3-story triaxial SHM sensor stream",
                )
                .clicked()
                {
                    let csv = crate::core::sample_shm_telemetry_csv();
                    if let Ok(log) = crate::core::parse_shm_telemetry_log(&csv, "sample_shm.csv") {
                        universal_canvas.shm_twin_config.custom_telemetry = Some(log);
                        universal_canvas.shm_twin_config.fault_scenario =
                            crate::core::ShmFaultScenario::CustomTelemetryStream;
                        config_changed = true;
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::UploadImport),
                    "Upload CSV/JSON",
                    false,
                    theme.accent_orange,
                    "Upload recorded accelerometer/inclinometer telemetry log",
                )
                .clicked()
                {
                    actions.open_shm_telemetry_picker = true;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Cube3D),
                    "Export .ifcx",
                    false,
                    theme.accent_green,
                    "Export non-destructive SHM & ATC-20 IFCX schema overlay",
                )
                .clicked()
                {
                    actions.export_shm_atc20_ifcx = true;
                }
            });

            if config_changed {
                self.apply_lens_to_views(SpatialLens::ShmDigitalTwin, build_view, universal_canvas);
                actions.run_shm_digital_twin = true;
            }

            ui.add_space(4.0);
            if let Some(ref rep) = universal_canvas.last_shm_twin_report {
                ui.horizontal_wrapped(|ui| {
                    icons::pill_badge(
                        ui,
                        AppIcon::ChartLine,
                        format!(
                            "f1 {:.2}->{:.2}Hz ({:+.1}%)",
                            rep.baseline_freq_1_hz, rep.observed_freq_1_hz, rep.freq_shift_pct
                        ),
                        if rep.freq_shift_pct.abs() < 5.0 {
                            theme.accent_green
                        } else if rep.freq_shift_pct.abs() < 15.0 {
                            theme.accent_orange
                        } else {
                            theme.accent_red
                        },
                    );
                    icons::pill_badge(
                        ui,
                        AppIcon::Target,
                        format!("MAC1 {:.3} • K_ret {:.0}%", rep.mac_mode_1, rep.global_stiffness_retention_pct),
                        if rep.mac_mode_1 >= 0.90 {
                            theme.accent_green
                        } else if rep.mac_mode_1 >= 0.78 {
                            theme.accent_orange
                        } else {
                            theme.accent_red
                        },
                    );
                    icons::pill_badge(
                        ui,
                        AppIcon::Shield,
                        format!(
                            "Max Drift {:.2}% • Res EigenGate {:.2}",
                            rep.max_transient_drift_pct, rep.residual_eigengate
                        ),
                        theme.accent_blue,
                    );
                });

                ui.add_space(4.0);
                let two_col = !is_mobile && avail_w >= 460.0;
                let chart_w = if two_col {
                    ((avail_w - 8.0) * 0.5).max(160.0)
                } else {
                    (avail_w - 4.0).clamp(160.0, cap_w - 24.0)
                };

                let draw_psd = |ui: &mut Ui| {
                    let (resp, p) =
                        ui.allocate_painter(Vec2::new(chart_w, 96.0), egui::Sense::hover());
                    let r = resp.rect;
                    p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
                    p.text(
                        egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                        Align2::LEFT_TOP,
                        "Welch PSD (Dashed=Baseline, Solid=Observed)",
                        egui::FontId::proportional(8.6),
                        theme.text_muted,
                    );
                    let f_max = rep
                        .psd_spectrum
                        .last()
                        .map(|s| s.freq_hz)
                        .unwrap_or(10.0)
                        .max(1.0);
                    let psd_max = rep
                        .psd_spectrum
                        .iter()
                        .map(|s| s.baseline_psd_g2_hz.max(s.observed_psd_g2_hz))
                        .fold(1e-4_f64, f64::max);
                    let pad_t = 18.0_f32;
                    let pad_b = 6.0_f32;
                    let w = (r.width() - 12.0).max(24.0);
                    let h = (r.height() - pad_t - pad_b).max(18.0);
                    let to_pt = |f: f64, val: f64| -> egui::Pos2 {
                        egui::Pos2::new(
                            r.left() + 6.0 + (f / f_max).clamp(0.0, 1.0) as f32 * w,
                            r.bottom() - pad_b - (val / psd_max).sqrt().clamp(0.0, 1.0) as f32 * h,
                        )
                    };
                    let base_pts: Vec<_> = rep
                        .psd_spectrum
                        .iter()
                        .map(|pt| to_pt(pt.freq_hz, pt.baseline_psd_g2_hz))
                        .collect();
                    let obs_pts: Vec<_> = rep
                        .psd_spectrum
                        .iter()
                        .map(|pt| to_pt(pt.freq_hz, pt.observed_psd_g2_hz))
                        .collect();
                    for (idx, win) in base_pts.windows(2).enumerate() {
                        if idx % 2 == 0 {
                            p.line_segment([win[0], win[1]], Stroke::new(1.2, theme.accent_blue.gamma_multiply(0.8)));
                        }
                    }
                    let [cr, cg, cb] = rep.atc20_placard.badge_color_rgb();
                    let obs_col = Color32::from_rgb(cr, cg, cb);
                    for win in obs_pts.windows(2) {
                        p.line_segment([win[0], win[1]], Stroke::new(1.8, obs_col));
                    }
                };

                let draw_story_bars = |ui: &mut Ui| {
                    let n_rows = rep.story_health.len().clamp(1, 6);
                    let bh = 96.0_f32.max(22.0 + (n_rows as f32) * 12.0);
                    let (resp, p) = ui.allocate_painter(Vec2::new(chart_w, bh), egui::Sense::hover());
                    let r = resp.rect;
                    p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
                    p.text(
                        egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                        Align2::LEFT_TOP,
                        format!("Per-Story Stiffness Retention ({} sensors)", rep.sensors.len()),
                        egui::FontId::proportional(8.6),
                        theme.text_muted,
                    );
                    let bar_x0 = r.left() + 34.0;
                    let bar_max_w = (r.width() - 96.0).max(28.0);
                    for (i, st) in rep.story_health.iter().take(6).enumerate() {
                        let y = r.top() + 20.0 + (i as f32) * 12.0;
                        let frac = (st.observed_stiffness_pct / 100.0).clamp(0.05, 1.0) as f32;
                        let [sr, sg, sb] = st.story_placard.badge_color_rgb();
                        p.text(
                            egui::Pos2::new(r.left() + 6.0, y + 4.0),
                            Align2::LEFT_CENTER,
                            format!("L{:02}", st.story_number),
                            egui::FontId::monospace(8.4),
                            theme.text_secondary,
                        );
                        p.rect_filled(
                            Rect::from_min_size(
                                egui::Pos2::new(bar_x0, y),
                                Vec2::new(bar_max_w * frac, 7.5),
                            ),
                            CornerRadius::same(2),
                            Color32::from_rgb(sr, sg, sb),
                        );
                        p.text(
                            egui::Pos2::new(bar_x0 + bar_max_w + 4.0, y + 4.0),
                            Align2::LEFT_CENTER,
                            format!("{:.0}% | {:.1}%", st.observed_stiffness_pct, st.peak_transient_drift_pct),
                            egui::FontId::monospace(8.0),
                            theme.text_muted,
                        );
                    }
                };

                if two_col {
                    ui.horizontal(|ui| {
                        draw_psd(ui);
                        draw_story_bars(ui);
                    });
                } else {
                    ui.vertical(|ui| {
                        draw_psd(ui);
                        ui.add_space(4.0);
                        draw_story_bars(ui);
                    });
                }
            } else {
                ui.label(
                    RichText::new("Click Evaluate SHM or run /shm to synthesize live 6-DOF modal fingerprints & sensors.")
                        .size(9.5)
                        .color(theme.text_muted),
                );
            }
        } else if self.shm_drawer_tab == 1 {
            if let Some(ref rep) = universal_canvas.last_shm_twin_report {
            let [pr, pg, pb] = rep.atc20_placard.badge_color_rgb();
            let placard_col = Color32::from_rgb(pr, pg, pb);
            Frame::new()
                .fill(placard_col.gamma_multiply(0.14))
                .stroke(Stroke::new(1.2, placard_col))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::same(6))
                .show(ui, |ui| {
                    ui.set_max_width(avail_w);
                    ui.label(
                        RichText::new(format!("ATC-20 POSTING: {}", rep.atc20_placard.label()))
                            .size(10.5)
                            .strong()
                            .color(placard_col),
                    );
                    ui.label(
                        RichText::new(rep.atc20_placard.guidance())
                            .size(9.0)
                            .color(theme.text_secondary),
                    );
                });
            ui.add_space(4.0);
            for item in &rep.atc20_checklist {
                let [cr, cg, cb] = item.outcome.badge_color_rgb();
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(cr, cg, cb),
                        RichText::new(format!("[{}] {}", item.outcome.short_label(), item.code)).size(8.8).strong(),
                    );
                    ui.label(
                        RichText::new(format!("{} = {} ({})", item.criterion, item.observed_value, item.threshold_value))
                            .size(8.8)
                            .color(theme.text_secondary),
                    );
                });
            }
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                icons::pill_badge(ui, AppIcon::CheckCircle, format!("GREEN {}", rep.green_count), Color32::from_rgb(34, 197, 94));
                icons::pill_badge(ui, AppIcon::Bolt, format!("YELLOW {}", rep.yellow_count), Color32::from_rgb(245, 158, 11));
                icons::pill_badge(ui, AppIcon::Shield, format!("RED {}", rep.red_count), Color32::from_rgb(239, 68, 68));
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Cube3D),
                    "Export SHM + ATC-20 .ifcx",
                    false,
                    theme.accent_green,
                    "Export composed IFCX dossier with SHM sensors & ATC-20 posting",
                )
                .clicked()
                {
                    actions.export_shm_atc20_ifcx = true;
                }
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Run ATC-20 Triage to classify active BIM & district portfolio.").size(9.5).color(theme.text_muted));
                if Self::glass_pill_button(ui, theme, Some(AppIcon::Shield), "Run ATC-20 Triage", false, theme.accent_green, "Evaluate ATC-20 safety posting").clicked() {
                    actions.run_shm_digital_twin = true;
                }
            });
        }
        } else {
            // Sub-Tab 2: Phase 78 Live WebSocket SHM Telemetry Streaming & Rolling-Window Watchdog
            use crate::core::shm_watchdog::{WatchdogAlertLevel, WatchdogSimMode};
            let [ar, ag, ab] = universal_canvas.shm_watchdog.alert_level.badge_color_rgb();
            let alert_col = Color32::from_rgb(ar, ag, ab);
            let mut sim_to_start: Option<WatchdogSimMode> = None;

            ui.horizontal_wrapped(|ui| {
                let wd = &mut universal_canvas.shm_watchdog;
                ui.label(RichText::new("WS Endpoint:").size(9.2).strong().color(theme.text_secondary));
                ui.add(
                    egui::TextEdit::singleline(&mut wd.config.ws_url)
                        .desired_width(if is_mobile { (avail_w - 130.0).max(110.0) } else { 250.0 }),
                );
                if wd.ws_connected {
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        Some(AppIcon::Close),
                        "Disconnect",
                        true,
                        theme.accent_red,
                        "Close live WebSocket telemetry connection",
                    )
                    .clicked()
                    {
                        actions.disconnect_shm_ws = true;
                    }
                } else if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Bolt),
                    "Connect WS",
                    false,
                    theme.accent_green,
                    "Connect browser/native WebSocket stream (ws:// or wss://)",
                )
                .clicked()
                {
                    actions.connect_shm_ws = Some(wd.config.ws_url.clone());
                }

                ui.separator();
                ui.label(RichText::new("Live Sim:").size(9.2).color(theme.text_muted));
                for &m in WatchdogSimMode::all() {
                    if Self::glass_pill_button(
                        ui,
                        theme,
                        None,
                        m.short_label(),
                        wd.config.sim_mode == m && m != WatchdogSimMode::Off,
                        if m == WatchdogSimMode::AmbientNominal {
                            theme.accent_green
                        } else if m == WatchdogSimMode::Off {
                            theme.accent_red
                        } else {
                            theme.accent_orange
                        },
                        m.label(),
                    )
                    .clicked()
                    {
                        sim_to_start = Some(m);
                    }
                }
            });
            if let Some(m) = sim_to_start {
                universal_canvas.shm_watchdog.start_simulator(m);
                if m != WatchdogSimMode::Off {
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                }
            }

            ui.add_space(3.0);
            let wd = &mut universal_canvas.shm_watchdog;
            ui.horizontal_wrapped(|ui| {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Auto ATC-20",
                    wd.config.auto_update_atc20,
                    theme.accent_blue,
                    "Automatically re-evaluate ATC-20 placard when STA/LTA or f1 shift triggers",
                )
                .clicked()
                {
                    wd.config.auto_update_atc20 = !wd.config.auto_update_atc20;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Auto Evac Reroute",
                    wd.config.auto_reroute_evacuation,
                    theme.accent_green,
                    "Automatically recompute A* evacuation routes when ATC-20 escalates to Yellow/Red",
                )
                .clicked()
                {
                    wd.config.auto_reroute_evacuation = !wd.config.auto_reroute_evacuation;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Cube3D),
                    "Watchdog .ifcx",
                    false,
                    theme.accent_green,
                    "Export non-destructive eigengraph::analysis::shm_watchdog .ifcx overlay",
                )
                .clicked()
                {
                    actions.export_watchdog_ifcx = true;
                }
                icons::pill_badge(ui, AppIcon::Shield, wd.alert_level.label(), alert_col);
                icons::pill_badge(
                    ui,
                    AppIcon::Waves,
                    format!("STA/LTA {:.2} (Pk {:.2})", wd.current_sta_lta, wd.peak_sta_lta),
                    if wd.current_sta_lta >= wd.config.sta_lta_trigger_ratio {
                        theme.accent_red
                    } else {
                        theme.accent_blue
                    },
                );
                icons::pill_badge(
                    ui,
                    AppIcon::ChartLine,
                    format!(
                        "f1 {:.2}Hz ({:+.1}%) • {} pkt • Reroutes {}",
                        wd.current_freq_1_hz,
                        wd.current_freq_shift_pct,
                        wd.total_packets_ingested,
                        wd.auto_reroute_count
                    ),
                    if wd.alert_level >= WatchdogAlertLevel::Warning {
                        theme.accent_orange
                    } else {
                        theme.accent_green
                    },
                );
            });

            ui.add_space(4.0);
            let two_col = !is_mobile && avail_w >= 620.0;
            let chart_w = if two_col {
                ((avail_w - 12.0) * 0.5).floor().max(180.0)
            } else {
                (avail_w - 4.0).clamp(160.0, cap_w - 24.0)
            };

            let wd_ro = &universal_canvas.shm_watchdog;
            let draw_wave = |ui: &mut Ui| {
                let (resp, p) = ui.allocate_painter(Vec2::new(chart_w, 92.0), egui::Sense::hover());
                let r = resp.rect;
                p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
                p.text(
                    egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                    Align2::LEFT_TOP,
                    "Rolling Triaxial Accel ax(t) & STA/LTA Ratio",
                    egui::FontId::proportional(8.6),
                    theme.text_muted,
                );
                if wd_ro.ring.len() >= 2 {
                    let t_min = wd_ro.ring.first().map(|s| s.time_s).unwrap_or(0.0);
                    let t_max = wd_ro.ring.last().map(|s| s.time_s).unwrap_or(1.0).max(t_min + 0.5);
                    let max_a = wd_ro
                        .ring
                        .iter()
                        .map(|s| s.accel_x_g.abs())
                        .fold(0.02_f64, f64::max);
                    let mid_y = r.center().y + 4.0;
                    let amp_h = (r.height() * 0.34).max(12.0);
                    let w = (r.width() - 12.0).max(20.0);
                    let pts: Vec<egui::Pos2> = wd_ro
                        .ring
                        .iter()
                        .map(|s| {
                            let x = r.left() + 6.0 + ((s.time_s - t_min) / (t_max - t_min)) as f32 * w;
                            let y = mid_y - (s.accel_x_g / max_a) as f32 * amp_h;
                            egui::Pos2::new(x, y)
                        })
                        .collect();
                    p.add(egui::Shape::line(pts, Stroke::new(1.5, theme.accent_blue)));

                    let sta_max = wd_ro
                        .sta_lta_history
                        .iter()
                        .map(|&(_, v)| v)
                        .fold(wd_ro.config.sta_lta_trigger_ratio * 1.4, f64::max)
                        .max(3.5);
                    let trig_y = r.bottom() - 6.0 - ((wd_ro.config.sta_lta_trigger_ratio / sta_max) as f32) * (r.height() - 24.0);
                    p.line_segment(
                        [egui::Pos2::new(r.left() + 6.0, trig_y), egui::Pos2::new(r.right() - 6.0, trig_y)],
                        Stroke::new(1.0, theme.accent_red.gamma_multiply(0.65)),
                    );
                    let sta_pts: Vec<egui::Pos2> = wd_ro
                        .sta_lta_history
                        .iter()
                        .map(|&(t, v)| {
                            let x = r.left() + 6.0 + ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0) as f32 * w;
                            let y = r.bottom() - 6.0 - (v / sta_max).clamp(0.0, 1.0) as f32 * (r.height() - 24.0);
                            egui::Pos2::new(x, y)
                        })
                        .collect();
                    if sta_pts.len() >= 2 {
                        p.add(egui::Shape::line(sta_pts, Stroke::new(1.3, theme.accent_orange)));
                    }
                }
            };

            let draw_freq = |ui: &mut Ui| {
                let (resp, p) = ui.allocate_painter(Vec2::new(chart_w, 92.0), egui::Sense::hover());
                let r = resp.rect;
                p.rect_filled(r, CornerRadius::same(6), theme.bg_card.gamma_multiply(0.55));
                p.text(
                    egui::Pos2::new(r.left() + 6.0, r.top() + 3.0),
                    Align2::LEFT_TOP,
                    "Rolling Fundamental Frequency f1(t) vs Baseline",
                    egui::FontId::proportional(8.6),
                    theme.text_muted,
                );
                if wd_ro.freq_history_hz.len() >= 2 {
                    let t_min = wd_ro.freq_history_hz.first().map(|x| x.0).unwrap_or(0.0);
                    let t_max = wd_ro.freq_history_hz.last().map(|x| x.0).unwrap_or(1.0).max(t_min + 0.5);
                    let f_max = (wd_ro.baseline_freq_1_hz * 1.15).max(1.0);
                    let f_min = (wd_ro.baseline_freq_1_hz * 0.55).max(0.2);
                    let w = (r.width() - 12.0).max(20.0);
                    let h = (r.height() - 24.0).max(16.0);
                    let base_y = r.bottom() - 6.0 - (((wd_ro.baseline_freq_1_hz - f_min) / (f_max - f_min)).clamp(0.0, 1.0) as f32) * h;
                    p.line_segment(
                        [egui::Pos2::new(r.left() + 6.0, base_y), egui::Pos2::new(r.right() - 6.0, base_y)],
                        Stroke::new(1.0, theme.accent_green.gamma_multiply(0.7)),
                    );
                    let pts: Vec<egui::Pos2> = wd_ro
                        .freq_history_hz
                        .iter()
                        .map(|&(t, f)| {
                            let x = r.left() + 6.0 + ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0) as f32 * w;
                            let y = r.bottom() - 6.0 - ((f - f_min) / (f_max - f_min)).clamp(0.0, 1.0) as f32 * h;
                            egui::Pos2::new(x, y)
                        })
                        .collect();
                    p.add(egui::Shape::line(pts, Stroke::new(1.6, alert_col)));
                }
            };

            if two_col {
                ui.horizontal(|ui| {
                    draw_wave(ui);
                    draw_freq(ui);
                });
            } else {
                draw_wave(ui);
                ui.add_space(4.0);
                draw_freq(ui);
            }

            ui.add_space(3.0);
            if wd_ro.events.is_empty() {
                ui.label(
                    RichText::new(format!("Status: {} — No Watchdog escalations yet.", wd_ro.ws_status))
                        .size(8.8)
                        .color(theme.text_muted),
                );
            } else {
                for ev in wd_ro.events.iter().rev().take(4) {
                    let [er, eg, eb] = ev.level.badge_color_rgb();
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            Color32::from_rgb(er, eg, eb),
                            RichText::new(format!("[#{} {}]", ev.seq, ev.level.short_label()))
                                .size(8.8)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {}{}",
                                ev.summary,
                                ev.triggered_atc20
                                    .map(|p| format!("-> ATC-20 {}", p.short_label()))
                                    .unwrap_or_default(),
                                if ev.triggered_evac_reroute {
                                    " -> Evac Rerouted"
                                } else {
                                    ""
                                }
                            ))
                            .size(8.8)
                            .color(theme.text_secondary),
                        );
                    });
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_evacuation_lifelines_drawer(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        use crate::core::evacuation_routing::{EvacuationMode, RoadEdgeStatus};
        use crate::core::shm_twin::Atc20Placard;

        let avail_w = ui.available_width().min(cap_w - 16.0).max(180.0);
        let mut recompute = false;

        ui.horizontal_wrapped(|ui| {
            icons::pill_badge(
                ui,
                AppIcon::Shield,
                "Phase 77 Evacuation & Emergency Lifelines",
                theme.accent_green,
            );
            if let Some(ref rep) = universal_canvas.last_evacuation_report {
                let conn_col = if rep.network_connectivity_pct >= 80.0 {
                    theme.accent_green
                } else if rep.network_connectivity_pct >= 55.0 {
                    theme.accent_orange
                } else {
                    theme.accent_red
                };
                icons::pill_badge(
                    ui,
                    AppIcon::Globe,
                    format!("Conn {:.0}%", rep.network_connectivity_pct),
                    conn_col,
                );
                icons::pill_badge(
                    ui,
                    AppIcon::Bolt,
                    format!("Responder {:.0}/100", rep.responder_accessibility_score),
                    if rep.responder_accessibility_score >= 70.0 {
                        theme.accent_blue
                    } else {
                        theme.accent_orange
                    },
                );
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Close),
                "Close",
                false,
                theme.accent_red,
                "Close Evacuation & Lifelines drawer",
            )
            .clicked()
            {
                self.evacuation_drawer_open = false;
            }
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Mode:").size(9.2).strong().color(theme.text_secondary));
            for &m in EvacuationMode::all() {
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    m.short_label(),
                    universal_canvas.evacuation_config.mode == m,
                    theme.accent_blue,
                    m.label(),
                )
                .clicked()
                {
                    universal_canvas.evacuation_config.mode = m;
                    recompute = true;
                }
            }
            if Self::glass_pill_button(
                ui,
                theme,
                None,
                "Flood Gate",
                universal_canvas.evacuation_config.include_flood_blockage,
                theme.accent_orange,
                "Include 3D floodwater depth > 0.35 m road closures",
            )
            .clicked()
            {
                universal_canvas.evacuation_config.include_flood_blockage =
                    !universal_canvas.evacuation_config.include_flood_blockage;
                recompute = true;
            }
            if Self::glass_pill_button(
                ui,
                theme,
                None,
                "Liq Penalty",
                universal_canvas.evacuation_config.include_liquefaction_penalty,
                theme.accent_orange,
                "Include Seed-Idriss liquefaction lateral spreading & road settlement penalties",
            )
            .clicked()
            {
                universal_canvas.evacuation_config.include_liquefaction_penalty =
                    !universal_canvas.evacuation_config.include_liquefaction_penalty;
                recompute = true;
            }
            if Self::glass_pill_button(
                ui,
                theme,
                None,
                "Bridge Frag",
                universal_canvas.evacuation_config.include_bridge_fragility,
                theme.accent_red,
                "Include seismic pier fragility closures on IfcBridge spans",
            )
            .clicked()
            {
                universal_canvas.evacuation_config.include_bridge_fragility =
                    !universal_canvas.evacuation_config.include_bridge_fragility;
                recompute = true;
            }
        });

        ui.add_space(3.0);
        ui.horizontal_wrapped(|ui| {
            let mut df = universal_canvas.evacuation_config.debris_radius_factor as f32;
            ui.label(RichText::new("Debris Factor:").size(9.2).color(theme.text_muted));
            ui.spacing_mut().slider_width = if is_mobile { 84.0 } else { 120.0 };
            let sresp = ui.add(egui::Slider::new(&mut df, 0.20..=1.00).fixed_decimals(2));
            if sresp.changed() {
                universal_canvas.evacuation_config.debris_radius_factor = df as f64;
            }
            if sresp.drag_stopped() {
                recompute = true;
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Bolt),
                "Recompute A*",
                false,
                theme.accent_green,
                "Re-evaluate ATC-20 debris cordons, road closures, and A* lifeline routes",
            )
            .clicked()
            {
                recompute = true;
            }
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::Cube3D),
                "Export .ifcx",
                false,
                theme.accent_green,
                "Export non-destructive eigengraph::analysis::evacuation_lifelines .ifcx overlay",
            )
            .clicked()
            {
                actions.export_evacuation_ifcx = true;
            }
        });

        if recompute {
            actions.run_evacuation_routing = true;
        }

        ui.add_space(4.0);
        if let Some(ref rep) = universal_canvas.last_evacuation_report {
            ui.horizontal_wrapped(|ui| {
                icons::pill_badge(
                    ui,
                    AppIcon::CheckCircle,
                    format!("Open {}/{}", rep.open_edges, rep.total_edges),
                    theme.accent_green,
                );
                icons::pill_badge(
                    ui,
                    AppIcon::Bolt,
                    format!("Degraded {}", rep.degraded_edges),
                    theme.accent_orange,
                );
                icons::pill_badge(
                    ui,
                    AppIcon::Shield,
                    format!("Blocked {}", rep.blocked_edges),
                    theme.accent_red,
                );
                icons::pill_badge(
                    ui,
                    AppIcon::Target,
                    format!(
                        "Isolated {} • Bridges {}/{}",
                        rep.isolated_building_count,
                        rep.bridges_evaluated.saturating_sub(rep.bridges_compromised),
                        rep.bridges_evaluated
                    ),
                    theme.accent_blue,
                );
            });

            ui.add_space(4.0);
            let two_col = !is_mobile && avail_w >= 860.0;
            let map_w = if two_col {
                (avail_w * 0.48).clamp(240.0, 520.0)
            } else {
                avail_w
            };
            let map_h = if is_mobile { 125.0 } else { 148.0 };
            let draw_minimap = |ui: &mut Ui| {
                let (map_rect, _) = ui.allocate_exact_size(Vec2::new(map_w, map_h), egui::Sense::hover());
                let painter = ui.painter_at(map_rect);
                painter.rect_filled(map_rect, 8.0, theme.bg_sidebar.gamma_multiply(0.85));
                painter.rect_stroke(
                    map_rect,
                    8.0,
                    Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.35)),
                    egui::StrokeKind::Inside,
                );

                let (mut min_x, mut max_x) = (-95.0_f64, 95.0_f64);
                let (mut min_y, mut max_y) = (-95.0_f64, 95.0_f64);
                for n in &rep.nodes {
                    min_x = min_x.min(n.pos_enu_m[0] - 10.0);
                    max_x = max_x.max(n.pos_enu_m[0] + 10.0);
                    min_y = min_y.min(n.pos_enu_m[1] - 10.0);
                    max_y = max_y.max(n.pos_enu_m[1] + 10.0);
                }
                let span = (max_x - min_x).max(max_y - min_y).max(40.0);
                let cx_w = 0.5 * (min_x + max_x);
                let cy_w = 0.5 * (min_y + max_y);
                let scale = ((map_rect.width() - 18.0).min(map_rect.height() - 18.0) as f64) / span;
                let to_pt = |p: [f64; 2]| -> egui::Pos2 {
                    egui::Pos2::new(
                        map_rect.center().x + ((p[0] - cx_w) * scale) as f32,
                        map_rect.center().y - ((p[1] - cy_w) * scale) as f32,
                    )
                };

                for e in &rep.edges {
                    if e.road_name == "Site Access Spur" {
                        continue;
                    }
                    if let (Some(nu), Some(nv)) = (rep.nodes.get(e.u), rep.nodes.get(e.v)) {
                        let col = match e.status {
                            RoadEdgeStatus::OpenSafe => theme.text_muted.gamma_multiply(0.55),
                            RoadEdgeStatus::DegradedDetour => Color32::from_rgb(245, 158, 11),
                            RoadEdgeStatus::BlockedClosed => Color32::from_rgb(239, 68, 68),
                        };
                        painter.line_segment(
                            [to_pt(nu.pos_enu_m), to_pt(nv.pos_enu_m)],
                            Stroke::new(if e.is_bridge { 2.6 } else { 1.6 }, col),
                        );
                    }
                }

                for c in &rep.cordons {
                    let col = match c.placard {
                        Atc20Placard::InspectedGreen => Color32::from_rgb(34, 197, 94),
                        Atc20Placard::RestrictedYellow => Color32::from_rgb(245, 158, 11),
                        Atc20Placard::UnsafeRed => Color32::from_rgb(239, 68, 68),
                    };
                    let rad_px = (c.cordon_radius_m * scale) as f32;
                    let center_px = to_pt(c.center_enu_m);
                    painter.circle_filled(center_px, rad_px.max(3.5), col.gamma_multiply(0.24));
                    painter.circle_stroke(center_px, rad_px.max(3.5), Stroke::new(1.2, col));
                }

                for r in &rep.routes {
                    if !r.reachable || r.waypoints_enu_m.len() < 2 {
                        continue;
                    }
                    let pts: Vec<egui::Pos2> = r.waypoints_enu_m.iter().map(|&p| to_pt(p)).collect();
                    let col = if r.is_responder_route {
                        Color32::from_rgb(56, 189, 248)
                    } else {
                        Color32::from_rgb(34, 211, 153)
                    };
                    painter.add(egui::Shape::line(
                        pts,
                        Stroke::new(if r.is_active_building { 2.6 } else { 1.5 }, col),
                    ));
                }

                for sp in &rep.safe_points {
                    let p = to_pt(sp.center_enu_m);
                    painter.circle_filled(p, 4.5, Color32::from_rgb(34, 197, 94));
                    painter.circle_stroke(p, 6.5, Stroke::new(1.1, Color32::WHITE));
                }
            };
            let draw_routes_list = |ui: &mut Ui| {
                for r in rep.routes.iter().take(4) {
                    let col = if !r.reachable {
                        theme.accent_red
                    } else if r.is_responder_route {
                        theme.accent_blue
                    } else {
                        theme.accent_green
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            col,
                            RichText::new(if r.is_responder_route {
                                "[RESPONDER]"
                            } else if r.reachable {
                                "[EGRESS]"
                            } else {
                                "[ISOLATED]"
                            })
                            .size(8.8)
                            .strong(),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} -> {} | {:.0} m ({:.0} s) | Safety {:.0}/100",
                                r.origin_label,
                                r.target_safe_zone,
                                r.distance_m,
                                r.estimated_time_sec,
                                r.safety_score_0_100
                            ))
                            .size(8.8)
                            .color(theme.text_secondary),
                        );
                    });
                }
            };

            if two_col {
                ui.horizontal(|ui| {
                    draw_minimap(ui);
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.set_max_width((avail_w - map_w - 26.0).max(160.0));
                        draw_routes_list(ui);
                    });
                });
            } else {
                draw_minimap(ui);
                ui.add_space(4.0);
                draw_routes_list(ui);
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(
                        "Click 'Recompute A*' to evaluate post-disaster evacuation routes & lifeline connectivity.",
                    )
                    .size(9.5)
                    .color(theme.text_muted),
                );
            });
        }
    }
}
