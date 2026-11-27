//! ribbon_ui.rs — Update `v0.8.6.2` (`2/10` -> `v0.8.7`):
//! Smart Layered 10-Category AEC Engineering Lifecycle Ribbon & Layer-2 Focus Breadcrumb Header.
//!
//! Renders:
//! - Layer 1: Quick-Access Bar (`Vault`, `Save`, `Undo`, `Redo`, `3D`, `Project · Level`, utilities) +
//!   10 scrollable AEC category tabs (`Planning` .. `Industrialization`) +
//!   horizontal grouped Icon-Above-Caption tool tiles with soft-pill bottom group footers.
//! - Layer 2: Compact breadcrumb header (`<- Ribbon | Active Focus Layer | x`) when any overlay,
//!   Project Browser, drawer, inspector, or chat is active so layers swap in-place cleanly.

use super::*;
use egui::{
    Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Layout, Margin, Pos2, Rect,
    Response, RichText, ScrollArea, Sense, Stroke, Ui, Vec2,
};

use crate::icons::{self, AppIcon};
use crate::models::{NavigationPillar, StructuralMember};
use crate::theme::MacTheme;
use crate::views::build::Ground3DAuthorTool;
use crate::views::universal_canvas::UniversalCanvasTool;
use crate::views::{BuildView, UniversalCanvasState};

impl SpatialWorkspaceState {
    /// Renders a Revit-inspired ribbon tool tile: centered vector icon on top,
    /// concise English caption underneath, arranged horizontally within its group.
    pub(super) fn ribbon_tool_tile(
        ui: &mut Ui,
        theme: &MacTheme,
        icon: AppIcon,
        label: &str,
        active: bool,
        accent: Color32,
        tooltip: &str,
    ) -> Response {
        let font = FontId::proportional(9.8);
        let text_w = ui.fonts_mut(|f| {
            f.layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE)
                .size()
                .x
        });
        let w = (text_w + 16.0).clamp(52.0, 96.0);
        let size = Vec2::new(w, 50.0);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

        let hovered = resp.hovered();
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        let fill = if active {
            accent.gamma_multiply(if theme.is_dark() { 0.24 } else { 0.16 })
        } else if hovered {
            theme.text_primary.gamma_multiply(0.08)
        } else {
            Color32::TRANSPARENT
        };
        let stroke = if active {
            Stroke::new(1.1, accent.gamma_multiply(0.78))
        } else if hovered {
            Stroke::new(0.8, theme.border_subtle.gamma_multiply(0.85))
        } else {
            Stroke::NONE
        };

        let painter = ui.painter();
        if fill != Color32::TRANSPARENT || stroke != Stroke::NONE {
            painter.rect(rect, CornerRadius::same(7), fill, stroke, egui::StrokeKind::Inside);
        }

        let fg = if active { accent } else { theme.text_primary };
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + 17.5),
            Vec2::splat(19.0),
        );
        icons::paint_icon(painter, icon_rect, icon, fg);

        painter.text(
            Pos2::new(rect.center().x, rect.bottom() - 10.0),
            Align2::CENTER_CENTER,
            label,
            font,
            fg,
        );

        if tooltip.is_empty() {
            resp
        } else {
            resp.on_hover_text(tooltip)
        }
    }

    /// Renders a horizontal cluster of ribbon tool tiles with a centered bottom category pill
    /// and a subtle vertical divider on the right edge.
    pub(super) fn ribbon_group(
        ui: &mut Ui,
        theme: &MacTheme,
        group_label: &str,
        add_tiles: impl FnOnce(&mut Ui),
    ) {
        let footer_bg = if theme.is_dark() {
            Color32::from_rgba_unmultiplied(148, 163, 184, 34)
        } else {
            Color32::from_rgba_unmultiplied(15, 23, 42, 16)
        };
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let tiles_resp = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                add_tiles(ui);
            });
            let footer_font = FontId::proportional(8.8);
            let label_w = ui.fonts_mut(|f| {
                f.layout_no_wrap(group_label.to_string(), footer_font.clone(), Color32::WHITE)
                    .size()
                    .x
            });
            let footer_w = tiles_resp.response.rect.width().max(label_w + 16.0);
            let (footer_rect, _) = ui.allocate_exact_size(Vec2::new(footer_w, 15.5), Sense::hover());
            ui.painter().rect_filled(footer_rect, CornerRadius::same(4), footer_bg);
            ui.painter().text(
                footer_rect.center(),
                Align2::CENTER_CENTER,
                group_label,
                footer_font,
                theme.text_muted,
            );
        });

        let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(6.0, 64.0), Sense::hover());
        let x = sep_rect.center().x;
        ui.painter().line_segment(
            [
                Pos2::new(x, sep_rect.top() + 4.0),
                Pos2::new(x, sep_rect.bottom() - 4.0),
            ],
            Stroke::new(1.0, theme.border_subtle.gamma_multiply(0.55)),
        );
    }

    /// Renders the sleek 1-row Layer 2 breadcrumb header (`<- Ribbon | Title | x`) above the
    /// active exclusive focus overlay so switching into Project Browser or a Studio Drawer never stacks height.
    pub(super) fn render_smart_layer2_header(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        title: &str,
        icon: AppIcon,
        cap_w: f32,
        mut close_embedded: Option<&mut bool>,
    ) -> bool {
        let mut returned_to_ribbon = false;
        let row_w = (cap_w - 8.0).max(160.0);
        ui.set_max_width(row_w);
        ui.horizontal(|ui| {
            ui.set_max_width(row_w);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            if Self::glass_pill_button(
                ui,
                theme,
                Some(AppIcon::ChevronLeft),
                "Ribbon",
                false,
                theme.accent_blue,
                "Return to Categorized Ribbon Deck",
            )
            .clicked()
            {
                self.close_focus_layer_return_to_ribbon();
                if let Some(flag) = close_embedded.as_deref_mut() {
                    *flag = true;
                }
                returned_to_ribbon = true;
            }
            ui.separator();
            icons::icon_label(
                ui,
                icon,
                13.0,
                theme.accent_blue,
                title,
                11.0,
                theme.text_primary,
                true,
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if Self::glass_icon_button(
                    ui,
                    theme,
                    AppIcon::Close,
                    false,
                    theme.text_secondary,
                    "Close overlay (Esc)",
                )
                .clicked()
                {
                    if let Some(flag) = close_embedded.as_deref_mut() {
                        *flag = true;
                    }
                    self.collapse_dynamic_island();
                }
            });
        });
        ui.add_space(2.0);
        ui.separator();
        returned_to_ribbon
    }

    /// Renders Layer 1 of the Smart Dynamic Island:
    /// Quick-Access Top Bar + 10 scrollable AEC Lifecycle Category Tabs +
    /// Horizontal Grouped Icon-Above-Caption Ribbon Strip.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_categorized_ribbon_deck(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        members: &[StructuralMember],
        vault_connected: bool,
        vault_name: &str,
        actions: &mut SpatialHudActions,
        cap_w: f32,
        is_mobile: bool,
    ) {
        let deck_w = (cap_w - 6.0).max(165.0);
        ui.set_width(deck_w);
        ui.set_max_width(deck_w);

        let can_undo =
            !build_view.bim_bus.undo_stack.is_empty() || !universal_canvas.undo_stack.is_empty();
        let can_redo =
            !build_view.bim_bus.redo_stack.is_empty() || !universal_canvas.redo_stack.is_empty();
        let has_key = !self.gemini_api_key_draft.trim().is_empty();
        let needs_key = (matches!(
            self.agent_model,
            AgentModelChoice::Gemini25Flash | AgentModelChoice::Gemini25Pro
        ) || !self.custom_gemini_model_id.trim().is_empty())
            && !has_key;
        let (_, _, story_lbl, _) = self.active_base_and_top_levels(&build_view.bim_bus.project);
        let lvl_cnt = build_view.bim_bus.project.levels.len().max(1);
        let proj_name = if build_view.bim_bus.project.project_name.trim().is_empty() {
            "EigenGraph Model".to_string()
        } else {
            build_view.bim_bus.project.project_name.clone()
        };

        // Row A: Quick-Access Top Bar (Vault / Save / Undo / Redo / 3D + Centered Model Title + Utilities)
        ui.horizontal(|ui| {
            ui.set_max_width(deck_w);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            ui.spacing_mut().item_spacing.x = 3.0;
            if vault_connected {
                if Self::glass_icon_button(
                    ui,
                    theme,
                    AppIcon::Folder,
                    self.vault_drawer_open,
                    theme.accent_green,
                    &format!("Vault Project Browser: {vault_name}"),
                )
                .clicked()
                {
                    self.toggle_exclusive_drawer("vault");
                }
            } else if Self::glass_icon_button(
                ui,
                theme,
                AppIcon::Folder,
                false,
                theme.accent_blue,
                "Open Project Browser / Mount Local Vault Folder",
            )
            .clicked()
            {
                self.toggle_exclusive_drawer("vault");
            }
            if Self::glass_icon_button(
                ui,
                theme,
                AppIcon::SaveDisk,
                false,
                theme.accent_blue,
                "Save / Export .eigen.json workspace",
            )
            .clicked()
            {
                actions.export_archive = true;
            }
            ui.separator();
            if Self::glass_icon_button(
                ui,
                theme,
                AppIcon::Undo,
                false,
                theme.accent_blue,
                "Undo last BIM or 2D canvas action (Cmd/Ctrl+Z)",
            )
            .clicked()
                && can_undo
            {
                actions.undo = true;
            }
            if Self::glass_icon_button(
                ui,
                theme,
                AppIcon::Redo,
                false,
                theme.accent_blue,
                "Redo undone BIM or 2D canvas action (Cmd/Ctrl+Y)",
            )
            .clicked()
                && can_redo
            {
                actions.redo = true;
            }
            ui.separator();
            if Self::glass_icon_button(
                ui,
                theme,
                AppIcon::Cube3D,
                self.active_canvas_mode == SpatialCanvasMode::EarthGlobe,
                theme.accent_blue,
                "Default 3D Isometric Earth / BIM View",
            )
            .clicked()
            {
                self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                universal_canvas.globe_mode = true;
                self.apply_camera_preset(build_view, universal_canvas, "iso");
                actions.fit_camera = true;
            }

            if !is_mobile && deck_w >= 540.0 {
                ui.add_space(6.0);
                let title_max_w = (deck_w - 340.0).max(60.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(title_max_w, 22.0),
                    Layout::top_down(Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{proj_name}  ·  {story_lbl}"))
                                    .size(10.0)
                                    .color(theme.text_muted),
                            )
                            .truncate(),
                        );
                    },
                );
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                if Self::glass_icon_button(
                    ui,
                    theme,
                    AppIcon::ChevronDown,
                    false,
                    theme.text_secondary,
                    "Minimize Dynamic Island",
                )
                .clicked()
                {
                    self.collapse_dynamic_island();
                }
                if Self::glass_icon_button(
                    ui,
                    theme,
                    AppIcon::SettingsGear,
                    false,
                    theme.text_secondary,
                    "Open Workspace Settings inside Dynamic Island",
                )
                .clicked()
                {
                    actions.open_settings = true;
                }
                let key_accent = if has_key {
                    theme.accent_green
                } else if needs_key {
                    theme.accent_orange
                } else {
                    theme.accent_blue
                };
                if Self::glass_icon_button(
                    ui,
                    theme,
                    AppIcon::Lock,
                    self.api_key_popover_open || has_key || needs_key,
                    key_accent,
                    if has_key {
                        "Gemini API Key configured"
                    } else {
                        "Configure Gemini API Key"
                    },
                )
                .clicked()
                {
                    self.toggle_exclusive_drawer("api_key");
                }
                if Self::glass_icon_button(
                    ui,
                    theme,
                    AppIcon::Clock,
                    self.history_popover_open,
                    theme.accent_blue,
                    "Conversation History",
                )
                .clicked()
                {
                    self.toggle_exclusive_drawer("history");
                }
                if Self::glass_icon_button(
                    ui,
                    theme,
                    AppIcon::SlidersPanel,
                    true,
                    theme.accent_blue,
                    &format!("{}: {}", self.agent_mode.label(), self.agent_mode.description()),
                )
                .clicked()
                {
                    self.agent_mode = self.agent_mode.next();
                }
                let model_lbl = self.active_model_display_label();
                if Self::glass_icon_button(
                    ui,
                    theme,
                    AppIcon::Bot,
                    false,
                    theme.accent_blue,
                    &format!("AI Model: {model_lbl} (click to cycle preset)"),
                )
                .clicked()
                {
                    self.agent_model = self.agent_model.next();
                }
            });
        });

        ui.add_space(3.0);

        // Row B: 10 Scrollable AEC Engineering Lifecycle Category Tabs
        ScrollArea::horizontal()
            .id_salt("smart_ribbon_category_tabs")
            .max_width(deck_w)
            .max_height(30.0)
            .auto_shrink([false, true])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for tab in RibbonCategoryTab::ALL {
                        let active = self.active_ribbon_tab == tab;
                        let fill = if active {
                            if theme.is_dark() {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 22)
                            } else {
                                Color32::WHITE
                            }
                        } else {
                            Color32::TRANSPARENT
                        };
                        let stroke = if active {
                            Stroke::new(1.0, theme.border_subtle)
                        } else {
                            Stroke::NONE
                        };
                        let fg = if active {
                            theme.text_primary
                        } else {
                            theme.text_muted
                        };
                        let mut rt = RichText::new(tab.label()).size(10.5).color(fg);
                        if active {
                            rt = rt.strong();
                        }
                        if ui
                            .add(
                                egui::Button::new(rt)
                                    .min_size(Vec2::new(0.0, 24.0))
                                    .fill(fill)
                                    .stroke(stroke)
                                    .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            self.active_ribbon_tab = tab;
                        }
                    }
                });
            });

        ui.add_space(3.0);

        // Row C: Frosted Ribbon Tray hosting Horizontal-Scrollable Grouped Icon-Above-Caption Tiles
        let (_, max_dcr, min_eigengate, _) = self.structural_health_summary_cached(
            &build_view.bim_bus.project,
            members,
            &universal_canvas.earth_fem_cache,
        );
        let tray_bg = if theme.is_dark() {
            Color32::from_rgba_unmultiplied(15, 23, 42, 118)
        } else {
            Color32::from_rgba_unmultiplied(248, 250, 252, 225)
        };
        Frame::new()
            .fill(tray_bg)
            .corner_radius(CornerRadius::same(10))
            .stroke(Stroke::new(0.8, theme.border_subtle.gamma_multiply(0.7)))
            .inner_margin(Margin {
                left: 8,
                right: 8,
                top: 5,
                bottom: 4,
            })
            .show(ui, |ui| {
                let inner_w = (deck_w - 18.0).max(145.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                let mut scroll_style = egui::style::ScrollStyle::solid();
                scroll_style.bar_width = 5.0;
                scroll_style.bar_inner_margin = 5.0;
                scroll_style.bar_outer_margin = 1.5;
                ui.style_mut().spacing.scroll = scroll_style;
                ScrollArea::horizontal()
                    .id_salt(("smart_ribbon_tools", self.active_ribbon_tab as usize))
                    .max_width(inner_w)
                    .max_height(98.0)
                    .auto_shrink([false, true])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            self.render_ribbon_tab_groups(
                                ui,
                                theme,
                                universal_canvas,
                                build_view,
                                members,
                                story_lbl,
                                lvl_cnt,
                                actions,
                            );
                        });
                    });
            });

        if let Some(active_brush) = universal_canvas.terrain_sculpt.active_brush {
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(deck_w);
                ui.spacing_mut().item_spacing.x = 6.0;
                icons::icon_label(
                    ui,
                    AppIcon::Mountain,
                    12.0,
                    theme.accent_orange,
                    &format!("Terraform: {}", active_brush.label()),
                    10.5,
                    theme.text_primary,
                    true,
                );
                ui.label(
                    egui::RichText::new("R:")
                        .size(10.0)
                        .color(theme.text_secondary),
                );
                ui.add(
                    egui::DragValue::new(&mut universal_canvas.terrain_sculpt.brush_radius_m)
                        .range(10.0..=180.0)
                        .speed(1.5)
                        .suffix(" m"),
                );
                ui.label(
                    egui::RichText::new("Pow:")
                        .size(10.0)
                        .color(theme.text_secondary),
                );
                ui.add(
                    egui::DragValue::new(&mut universal_canvas.terrain_sculpt.brush_strength_m)
                        .range(0.5..=12.0)
                        .speed(0.2)
                        .suffix(" m"),
                );
                ui.label(
                    egui::RichText::new("Sea:")
                        .size(10.0)
                        .color(theme.text_secondary),
                );
                ui.add(
                    egui::DragValue::new(&mut universal_canvas.sea_level_m)
                        .range(-8.0..=12.0)
                        .speed(0.25)
                        .suffix(" m"),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Cut {:.0} / Fill {:.0} m³",
                        universal_canvas.terrain_sculpt.total_cut_m3,
                        universal_canvas.terrain_sculpt.total_fill_m3
                    ))
                    .size(10.0)
                    .color(theme.text_secondary),
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::SaveDisk),
                    "IFCX",
                    false,
                    theme.accent_green,
                    "Export sculpted IfcSite topography & cut/fill balance to .ifcx",
                )
                .clicked()
                {
                    let _ = universal_canvas.export_site_terraform_ifcx(&build_view.bim_bus.project);
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Close),
                    "Done",
                    false,
                    theme.text_secondary,
                    "Exit live terrain sculpt brush",
                )
                .clicked()
                {
                    universal_canvas.set_terrain_sculpt_brush(None);
                }
            });
        }

        if matches!(
            universal_canvas.earth_bim_author.active_tool,
            crate::views::universal_canvas::EarthBimTool::Road
                | crate::views::universal_canvas::EarthBimTool::Bridge
        ) {
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(deck_w);
                ui.spacing_mut().item_spacing.x = 6.0;
                let is_bridge = universal_canvas.earth_bim_author.active_tool
                    == crate::views::universal_canvas::EarthBimTool::Bridge;
                let step_lbl = if universal_canvas.earth_bim_author.pending_start_mm.is_none() {
                    "Click 1: Start"
                } else if universal_canvas.city_scene.road_draw_mode
                    == crate::core::gis::RoadDrawMode::CurvedSpline
                    && universal_canvas
                        .earth_bim_author
                        .pending_road_control_mm
                        .is_none()
                {
                    "Click 2: Bend"
                } else {
                    "Click End"
                };
                icons::icon_label(
                    ui,
                    if is_bridge {
                        AppIcon::BridgeSpan
                    } else {
                        AppIcon::ArcCurve
                    },
                    12.0,
                    theme.accent_blue,
                    &format!(
                        "{} ({}) • {}",
                        if is_bridge { "Bridge" } else { "Road" },
                        universal_canvas.city_scene.road_draw_mode.label(),
                        step_lbl
                    ),
                    10.5,
                    theme.text_primary,
                    true,
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    universal_canvas.city_scene.active_road_kind.label(),
                    false,
                    theme.accent_blue,
                    "Cycle roadway class (Arterial / Collector / Local / Boulevard)",
                )
                .clicked()
                {
                    universal_canvas.city_scene.active_road_kind =
                        universal_canvas.city_scene.active_road_kind.cycle_next();
                }
                ui.label(
                    egui::RichText::new("Elev:")
                        .size(10.0)
                        .color(theme.text_secondary),
                );
                ui.add(
                    egui::DragValue::new(
                        &mut universal_canvas.earth_bim_author.road_elevation_offset_m,
                    )
                    .range(0.0..=24.0)
                    .speed(0.5)
                    .suffix(" m"),
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Auto-Bridge",
                    universal_canvas.city_scene.auto_bridge_enabled,
                    theme.accent_green,
                    "Automatically span water and bay crossings with cable-stayed IfcBridge decks & piers",
                )
                .clicked()
                {
                    universal_canvas.city_scene.auto_bridge_enabled =
                        !universal_canvas.city_scene.auto_bridge_enabled;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Sparkles),
                    "Demo Hwy",
                    false,
                    theme.accent_orange,
                    "Build a curved coastal arterial boulevard + cable-stayed auto-bridge across the bay",
                )
                .clicked()
                {
                    universal_canvas.synthesize_demo_coastal_highway();
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Close),
                    "Done",
                    false,
                    theme.text_secondary,
                    "Exit road/bridge authoring tool",
                )
                .clicked()
                {
                    universal_canvas.earth_bim_author.active_tool =
                        crate::views::universal_canvas::EarthBimTool::None;
                    universal_canvas.earth_bim_author.pending_start_mm = None;
                    universal_canvas.earth_bim_author.pending_road_control_mm = None;
                }
            });
        }

        if universal_canvas.earth_bim_author.active_tool
            == crate::views::universal_canvas::EarthBimTool::Zone
        {
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(deck_w);
                ui.spacing_mut().item_spacing.x = 6.0;
                let metrics = universal_canvas.city_scene.district_metrics();
                icons::icon_label(
                    ui,
                    AppIcon::Building,
                    12.0,
                    theme.accent_blue,
                    &format!("Zoning: {}", universal_canvas.city_scene.active_zone_kind.label()),
                    10.5,
                    theme.text_primary,
                    true,
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    universal_canvas.city_scene.active_zone_density.label(),
                    false,
                    theme.accent_blue,
                    "Cycle density tier (Low -> Medium -> High-Rise Towers)",
                )
                .clicked()
                {
                    universal_canvas.city_scene.active_zone_density =
                        universal_canvas.city_scene.active_zone_density.cycle_next();
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Auto-Towers",
                    universal_canvas.city_scene.synthesize_zone_towers,
                    theme.accent_green,
                    "Automatically spawn procedural IfcBuildings inside newly zoned parcels",
                )
                .clicked()
                {
                    universal_canvas.city_scene.synthesize_zone_towers =
                        !universal_canvas.city_scene.synthesize_zone_towers;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Sparkles),
                    "Auto-Zone",
                    false,
                    theme.accent_orange,
                    "Generate road-snapped zoning parcels & procedural IfcBuildings along all roads",
                )
                .clicked()
                {
                    universal_canvas.auto_populate_road_parcels();
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{} Bldgs • Pop {} • Jobs {} • {:.0} m²",
                        metrics.building_count,
                        metrics.total_residents,
                        metrics.total_jobs,
                        metrics.total_gfa_m2
                    ))
                    .size(10.0)
                    .color(theme.text_secondary),
                );
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Undo),
                    "Undo",
                    false,
                    theme.text_secondary,
                    "Remove last authored zone parcel",
                )
                .clicked()
                {
                    universal_canvas.city_scene.zones.pop();
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Close),
                    "Done",
                    false,
                    theme.text_secondary,
                    "Exit parcel zoning brush",
                )
                .clicked()
                {
                    universal_canvas.earth_bim_author.active_tool =
                        crate::views::universal_canvas::EarthBimTool::None;
                }
            });
        }

        if universal_canvas.section_box.enabled {
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(deck_w);
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().item_spacing.y = 2.0;
                let lvl_cut = universal_canvas.section_box.level_cut_mode;
                icons::icon_label(
                    ui,
                    if lvl_cut {
                        AppIcon::Layers
                    } else {
                        AppIcon::SectionCut
                    },
                    12.0,
                    theme.accent_blue,
                    if lvl_cut { "Level Cut" } else { "Section Box" },
                    10.5,
                    theme.text_primary,
                    true,
                );
                let mut axis_drag = |lbl: &str, val: &mut f32, range: std::ops::RangeInclusive<f32>| {
                    ui.label(
                        egui::RichText::new(lbl)
                            .size(9.5)
                            .color(theme.text_secondary),
                    );
                    ui.add(
                        egui::DragValue::new(val)
                            .range(range)
                            .speed(0.25)
                            .max_decimals(1)
                            .suffix("m"),
                    );
                };
                axis_drag("X0", &mut universal_canvas.section_box.min_enu_m[0], -200.0..=200.0);
                axis_drag("X1", &mut universal_canvas.section_box.max_enu_m[0], -200.0..=200.0);
                axis_drag("Y0", &mut universal_canvas.section_box.min_enu_m[1], -200.0..=200.0);
                axis_drag("Y1", &mut universal_canvas.section_box.max_enu_m[1], -200.0..=200.0);
                axis_drag("Z0", &mut universal_canvas.section_box.min_enu_m[2], -50.0..=250.0);
                axis_drag("Z1", &mut universal_canvas.section_box.max_enu_m[2], -50.0..=250.0);
                for i in 0..3 {
                    if universal_canvas.section_box.max_enu_m[i]
                        < universal_canvas.section_box.min_enu_m[i] + 0.5
                    {
                        universal_canvas.section_box.max_enu_m[i] =
                            universal_canvas.section_box.min_enu_m[i] + 0.5;
                    }
                }
                if lvl_cut {
                    ui.label(
                        egui::RichText::new("Off")
                            .size(9.5)
                            .color(theme.text_secondary),
                    );
                    if ui
                        .add(
                            egui::DragValue::new(
                                &mut universal_canvas.section_box.cut_plane_offset_m,
                            )
                            .range(0.2..=4.0)
                            .speed(0.1)
                            .max_decimals(1)
                            .suffix("m"),
                        )
                        .changed()
                    {
                        universal_canvas
                            .section_box
                            .apply_level_cut(&build_view.bim_bus.project, self.active_story_idx);
                    }
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Wire",
                    universal_canvas.section_box.show_wireframe,
                    theme.accent_blue,
                    "Toggle 3D Section Box wireframe cage overlay",
                )
                .clicked()
                {
                    universal_canvas.section_box.show_wireframe =
                        !universal_canvas.section_box.show_wireframe;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    None,
                    "Site",
                    universal_canvas.section_box.clip_terrain_and_city,
                    theme.accent_orange,
                    "Also clip IfcSite terrain, water, and city buildings outside Section Box",
                )
                .clicked()
                {
                    universal_canvas.section_box.clip_terrain_and_city =
                        !universal_canvas.section_box.clip_terrain_and_city;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::ZoomFit),
                    "Fit",
                    false,
                    theme.accent_green,
                    "Fit Section Box to active BIM structure extents",
                )
                .clicked()
                {
                    universal_canvas
                        .section_box
                        .fit_to_bim_project(&build_view.bim_bus.project, 2.5);
                    universal_canvas.section_box.level_cut_mode = false;
                }
                if Self::glass_pill_button(
                    ui,
                    theme,
                    Some(AppIcon::Close),
                    "Off",
                    false,
                    theme.text_secondary,
                    "Disable 3D Section Box / Level Cut Plane",
                )
                .clicked()
                {
                    universal_canvas.section_box.enabled = false;
                    universal_canvas.section_box.level_cut_mode = false;
                }
            });
        }

        if self.active_lens != SpatialLens::Clean {
            ui.add_space(3.0);
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
    }

    #[allow(clippy::too_many_arguments)]
    fn render_ribbon_tab_groups(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        _members: &[StructuralMember],
        story_lbl: String,
        lvl_cnt: usize,
        actions: &mut SpatialHudActions,
    ) {
        match self.active_ribbon_tab {
            RibbonCategoryTab::Planning => {
                Self::ribbon_group(ui, theme, "Site & GIS", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Globe,
                        "Earth 3D",
                        self.active_canvas_mode == SpatialCanvasMode::EarthGlobe,
                        theme.accent_blue,
                        "Switch to Living 3D Vector Earth Globe",
                    )
                    .clicked()
                    {
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        universal_canvas.globe_mode = true;
                        actions.fit_camera = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Mountain,
                        "Terrain",
                        universal_canvas.show_terrain_contours,
                        theme.accent_green,
                        "Toggle pure-vector 3D TIN terrain contours & SNI 8460 slope overlay",
                    )
                    .clicked()
                    {
                        universal_canvas.show_terrain_contours =
                            !universal_canvas.show_terrain_contours;
                        if universal_canvas.show_terrain_contours {
                            universal_canvas.ensure_site_tin_generated();
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Palette,
                        "Basemap",
                        false,
                        theme.accent_blue,
                        &format!(
                            "Cycle Earth Vector Theme ({})",
                            universal_canvas.world_map.visual_layer_label()
                        ),
                    )
                    .clicked()
                    {
                        universal_canvas.world_map.cycle_basemap_visual_mode();
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Target,
                        "Neighbors",
                        false,
                        theme.accent_orange,
                        "Run District Hazard & Neighboring Pounding Audit",
                    )
                    .clicked()
                    {
                        actions.run_district_hazard_audit = true;
                    }
                });
                Self::ribbon_group(ui, theme, "Landscape", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Mountain,
                        "Bay & Peak",
                        universal_canvas.show_terrain_contours
                            && universal_canvas.site_tin_preset
                                == crate::core::gis::DemTerrainPreset::ArchipelagoBayAndVolcano,
                        theme.accent_green,
                        "Generate realistic tropical Bay, Sandy Beach, Urban Terrace & Volcanic Peak landscape",
                    )
                    .clicked()
                    {
                        universal_canvas.apply_landscape_preset(
                            crate::core::gis::DemTerrainPreset::ArchipelagoBayAndVolcano,
                        );
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        universal_canvas.globe_mode = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Waves,
                        "Fjord",
                        universal_canvas.show_terrain_contours
                            && universal_canvas.site_tin_preset
                                == crate::core::gis::DemTerrainPreset::FjordRiverValley,
                        theme.accent_blue,
                        "Generate alpine Fjord River Valley with navigable waterway and steep coastal cliffs",
                    )
                    .clicked()
                    {
                        universal_canvas.apply_landscape_preset(
                            crate::core::gis::DemTerrainPreset::FjordRiverValley,
                        );
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        universal_canvas.globe_mode = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Globe,
                        "Island",
                        universal_canvas.show_terrain_contours
                            && universal_canvas.site_tin_preset
                                == crate::core::gis::DemTerrainPreset::CoastalIslandMetropolis,
                        theme.accent_blue,
                        "Generate sculpted Coastal Island Harbor surrounded by animated ocean waters",
                    )
                    .clicked()
                    {
                        universal_canvas.apply_landscape_preset(
                            crate::core::gis::DemTerrainPreset::CoastalIslandMetropolis,
                        );
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        universal_canvas.globe_mode = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Waves,
                        "Ocean",
                        universal_canvas.show_water_surface,
                        theme.accent_blue,
                        "Toggle animated coastal ocean waves, Beer-Lambert depth absorption & shoreline foam",
                    )
                    .clicked()
                    {
                        universal_canvas.show_water_surface = !universal_canvas.show_water_surface;
                        if universal_canvas.show_water_surface {
                            universal_canvas.show_terrain_contours = true;
                            universal_canvas.realistic_landscape_enabled = true;
                            universal_canvas.ensure_site_tin_generated();
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Trees,
                        "Biomes",
                        universal_canvas.realistic_landscape_enabled,
                        theme.accent_green,
                        "Toggle realistic elevation/slope biomes (sand, grass, forest, stratified basalt rock, snow)",
                    )
                    .clicked()
                    {
                        universal_canvas.realistic_landscape_enabled =
                            !universal_canvas.realistic_landscape_enabled;
                        if universal_canvas.realistic_landscape_enabled {
                            universal_canvas.show_terrain_contours = true;
                            universal_canvas.ensure_site_tin_generated();
                        }
                    }
                });
                Self::ribbon_group(ui, theme, "Terraform", |ui| {
                    let cur_b = universal_canvas.terrain_sculpt.active_brush;
                    let sculpt_btns = [
                        (
                            crate::core::gis::TerrainSculptBrush::Raise,
                            AppIcon::Mountain,
                            theme.accent_orange,
                        ),
                        (
                            crate::core::gis::TerrainSculptBrush::Lower,
                            AppIcon::ArrowDown,
                            theme.accent_red,
                        ),
                        (
                            crate::core::gis::TerrainSculptBrush::Flatten,
                            AppIcon::Slab,
                            theme.accent_green,
                        ),
                        (
                            crate::core::gis::TerrainSculptBrush::Smooth,
                            AppIcon::Sparkles,
                            theme.accent_blue,
                        ),
                        (
                            crate::core::gis::TerrainSculptBrush::CarveWater,
                            AppIcon::Waves,
                            theme.accent_blue,
                        ),
                    ];
                    for (brush, ic, col) in sculpt_btns {
                        let active = cur_b == Some(brush);
                        if Self::ribbon_tool_tile(
                            ui,
                            theme,
                            ic,
                            brush.short_label(),
                            active,
                            col,
                            brush.tooltip(),
                        )
                        .clicked()
                        {
                            let next = if active { None } else { Some(brush) };
                            universal_canvas.set_terrain_sculpt_brush(next);
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Undo,
                        "Undo TIN",
                        false,
                        theme.text_secondary,
                        "Revert the last 3D terrain sculpt stroke",
                    )
                    .clicked()
                    {
                        universal_canvas.undo_terrain_sculpt();
                    }
                });
                Self::ribbon_group(ui, theme, "Roads", |ui| {
                    let et = universal_canvas.earth_bim_author.active_tool;
                    let rm = universal_canvas.city_scene.road_draw_mode;
                    let curve_active = et == crate::views::universal_canvas::EarthBimTool::Road
                        && rm == crate::core::gis::RoadDrawMode::CurvedSpline;
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::ArcCurve,
                        "Curve Rd",
                        curve_active,
                        theme.accent_blue,
                        "Cities: Skylines II 3-Click Curved Bézier Road (auto-bridges water)",
                    )
                    .clicked()
                    {
                        if curve_active {
                            universal_canvas.earth_bim_author.active_tool =
                                crate::views::universal_canvas::EarthBimTool::None;
                        } else {
                            universal_canvas.set_road_author_mode(
                                false,
                                crate::core::gis::RoadDrawMode::CurvedSpline,
                            );
                        }
                    }
                    let straight_active = et == crate::views::universal_canvas::EarthBimTool::Road
                        && rm == crate::core::gis::RoadDrawMode::Straight;
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::LineSeg,
                        "Straight",
                        straight_active,
                        theme.accent_blue,
                        "Draw 2-Click Straight Terrain-Following Road (auto-bridges water)",
                    )
                    .clicked()
                    {
                        if straight_active {
                            universal_canvas.earth_bim_author.active_tool =
                                crate::views::universal_canvas::EarthBimTool::None;
                        } else {
                            universal_canvas.set_road_author_mode(
                                false,
                                crate::core::gis::RoadDrawMode::Straight,
                            );
                        }
                    }
                    let bridge_active = et == crate::views::universal_canvas::EarthBimTool::Bridge;
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::BridgeSpan,
                        "Bridge",
                        bridge_active,
                        theme.accent_green,
                        "Draw Elevated Cable-Stayed / Pier IfcBridge Spline",
                    )
                    .clicked()
                    {
                        if bridge_active {
                            universal_canvas.earth_bim_author.active_tool =
                                crate::views::universal_canvas::EarthBimTool::None;
                        } else {
                            universal_canvas.set_road_author_mode(
                                true,
                                crate::core::gis::RoadDrawMode::CurvedSpline,
                            );
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Grid,
                        "Road Type",
                        false,
                        theme.accent_orange,
                        &format!(
                            "Cycle Road Class ({})",
                            universal_canvas.city_scene.active_road_kind.label()
                        ),
                    )
                    .clicked()
                    {
                        universal_canvas.city_scene.active_road_kind =
                            universal_canvas.city_scene.active_road_kind.cycle_next();
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Undo,
                        "Undo Rd",
                        false,
                        theme.text_secondary,
                        "Remove the most recently authored IfcRoad / IfcBridge segment",
                    )
                    .clicked()
                    {
                        universal_canvas.city_scene.roads.pop();
                    }
                });
                Self::ribbon_group(ui, theme, "City Zoning", |ui| {
                    let zone_active = universal_canvas.earth_bim_author.active_tool
                        == crate::views::universal_canvas::EarthBimTool::Zone;
                    let cur_zk = universal_canvas.city_scene.active_zone_kind;
                    let cur_dn = universal_canvas.city_scene.active_zone_density;
                    let zone_btns = [
                        (
                            crate::core::gis::IfcxZoneKind::Residential,
                            AppIcon::Building,
                            "Res",
                            theme.accent_green,
                            "Zone Residential Housing Parcels (snaps to roads)",
                        ),
                        (
                            crate::core::gis::IfcxZoneKind::Commercial,
                            AppIcon::BoxPackage,
                            "Com",
                            theme.accent_blue,
                            "Zone Commercial Office & Retail Towers (snaps to roads)",
                        ),
                        (
                            crate::core::gis::IfcxZoneKind::Industrial,
                            AppIcon::Wrench,
                            "Ind",
                            theme.accent_orange,
                            "Zone Industrial & Logistics Halls (snaps to roads)",
                        ),
                        (
                            crate::core::gis::IfcxZoneKind::Park,
                            AppIcon::Trees,
                            "Park",
                            theme.accent_green,
                            "Zone Urban Parks & Pocket Greenery",
                        ),
                    ];
                    for (zk, ic, lbl, col, tip) in zone_btns {
                        let active = zone_active && cur_zk == zk;
                        if Self::ribbon_tool_tile(ui, theme, ic, lbl, active, col, tip).clicked() {
                            if active {
                                universal_canvas.earth_bim_author.active_tool =
                                    crate::views::universal_canvas::EarthBimTool::None;
                            } else {
                                universal_canvas.set_zone_author_mode(zk, cur_dn);
                            }
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Sparkles,
                        "Auto-Zone",
                        false,
                        theme.accent_blue,
                        "Automatically lay road-snapped zoning parcels & spawn procedural IfcBuildings",
                    )
                    .clicked()
                    {
                        universal_canvas.auto_populate_road_parcels();
                    }
                });
                Self::ribbon_group(ui, theme, "City -> BIM", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Building,
                        "Promote",
                        self.city_hub_drawer_open && self.city_hub_drawer_tab == 0,
                        theme.accent_green,
                        "Promote any procedural city building parcel into an editable 6-DOF OpenBIM project",
                    )
                    .clicked()
                    {
                        self.open_city_hub_drawer(0);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::NodesGraph,
                        "Junctions",
                        (self.city_hub_drawer_open && self.city_hub_drawer_tab == 1)
                            || universal_canvas.city_scene.prefer_roundabouts,
                        theme.accent_blue,
                        "Detect Smart Road Intersections (4-Way, T-Junc, Roundabout) & evaluate HCM LOS",
                    )
                    .clicked()
                    {
                        self.open_city_hub_drawer(1);
                        actions.run_road_intersections = true;
                    }
                });
                Self::ribbon_group(ui, theme, "Utilities", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Layers,
                        "Utilities",
                        self.utilities_drawer_open || universal_canvas.urban_utilities_overlay_visible,
                        theme.accent_blue,
                        "Open Subsurface Urban Utilities (Water, Sewer/Storm, MV Power) & evaluate city network",
                    )
                    .clicked()
                    {
                        self.open_utilities_drawer(0);
                        actions.run_urban_utilities = true;
                    }
                    let xray_active = universal_canvas.urban_utilities_overlay_visible
                        && universal_canvas.urban_utilities_config.xray_underground;
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SectionCut,
                        "X-Ray",
                        xray_active,
                        theme.accent_green,
                        "Toggle Underground X-Ray view for buried Water, Sewer & MV Power mains",
                    )
                    .clicked()
                    {
                        actions.toggle_urban_utilities_xray = true;
                    }
                });
                Self::ribbon_group(ui, theme, "Ambience", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Trees,
                        "Forests",
                        universal_canvas.city_scene.ambience.show_procedural_forests,
                        theme.accent_green,
                        "Toggle procedural biome hillside & coastal forests",
                    )
                    .clicked()
                    {
                        universal_canvas.city_scene.ambience.show_procedural_forests =
                            !universal_canvas.city_scene.ambience.show_procedural_forests;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Bolt,
                        "Traffic",
                        universal_canvas.city_scene.ambience.show_animated_traffic,
                        theme.accent_orange,
                        "Toggle animated bidirectional 3D road traffic with night headlights",
                    )
                    .clicked()
                    {
                        universal_canvas.city_scene.ambience.show_animated_traffic =
                            !universal_canvas.city_scene.ambience.show_animated_traffic;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Sun,
                        "Shadows",
                        universal_canvas.city_scene.ambience.show_dynamic_shadows,
                        theme.accent_blue,
                        "Toggle dynamic astronomical sun-azimuth cast building shadows",
                    )
                    .clicked()
                    {
                        universal_canvas.city_scene.ambience.show_dynamic_shadows =
                            !universal_canvas.city_scene.ambience.show_dynamic_shadows;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Clock,
                        "Day/Dusk",
                        false,
                        theme.accent_orange,
                        &format!(
                            "Cycle Solar Lighting ({})",
                            universal_canvas.earth_world.camera.lighting_preset_label()
                        ),
                    )
                    .clicked()
                    {
                        universal_canvas.cycle_living_earth_time_of_day();
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Sparkles,
                        "Live City",
                        false,
                        theme.accent_green,
                        "Generate full Cities: Skylines II coastal bay, auto-bridge, high-rises, forests & traffic",
                    )
                    .clicked()
                    {
                        universal_canvas.synthesize_full_living_coastal_city();
                        actions.fit_camera = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SaveDisk,
                        "Export City",
                        false,
                        theme.accent_blue,
                        "Export unified Living City IFCX bundle (IfcSite + IfcRoad + IfcBridge + IfcSpatialZone + IfcBuilding + SHA-256)",
                    )
                    .clicked()
                    {
                        let _ = universal_canvas
                            .export_living_city_ifcx_bundle(&build_view.bim_bus.project);
                    }
                });
                Self::ribbon_group(ui, theme, "Zoning & Brief", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Compass,
                        "Zoning",
                        self.zoning_drawer_open || universal_canvas.zoning_overlay_visible,
                        theme.accent_green,
                        "Open Site Zoning & 3D Buildable Envelope Studio (KDB/KLB/GSB)",
                    )
                    .clicked()
                    {
                        self.open_zoning_drawer(0);
                        actions.run_zoning_agent = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::BoxPackage,
                        "Program",
                        self.program_drawer_open || universal_canvas.program_overlay_visible,
                        theme.accent_green,
                        "Open Spatial Program, Adjacency & 3D Stack/Pack Studio",
                    )
                    .clicked()
                    {
                        self.open_program_drawer(0);
                        actions.run_spatial_program = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Sparkles,
                        "AI Agent",
                        self.design_agent_drawer_open,
                        theme.accent_blue,
                        "Open Chained 6-Stage AI Building Design Agent (+ Sheet S-115)",
                    )
                    .clicked()
                    {
                        self.open_design_agent_drawer();
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Document,
                        "Sheet 1D",
                        self.active_overlay_pillar == Some(NavigationPillar::ResearchConcept),
                        theme.accent_blue,
                        "Open 1. Planning & GIS Inception module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::ResearchConcept);
                    }
                });
            }

            RibbonCategoryTab::Drafting => {
                Self::ribbon_group(ui, theme, "Select", |ui| {
                    let act = SpatialManualTool::OrbitPan
                        .is_active(self.active_canvas_mode, universal_canvas, build_view);
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SelectArrow,
                        "Modify",
                        act,
                        theme.accent_blue,
                        SpatialManualTool::OrbitPan.tooltip(),
                    )
                    .clicked()
                    {
                        let sub =
                            SpatialManualTool::OrbitPan.apply(self, universal_canvas, build_view);
                        actions.merge(sub);
                    }
                    for ext in [UniversalCanvasTool::BoxSelect, UniversalCanvasTool::Eraser] {
                        let ext_act = universal_canvas.active_tool == ext;
                        if Self::ribbon_tool_tile(
                            ui,
                            theme,
                            ext.app_icon(),
                            ext.short_label(),
                            ext_act,
                            theme.accent_orange,
                            ext.tooltip(),
                        )
                        .clicked()
                        {
                            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                            universal_canvas.globe_mode = true;
                            build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                            universal_canvas.earth_bim_author.active_tool =
                                crate::views::universal_canvas::EarthBimTool::None;
                            universal_canvas.active_tool = ext;
                        }
                    }
                });
                Self::ribbon_group(ui, theme, "2D Sketch", |ui| {
                    for tool in [
                        SpatialManualTool::Line2D,
                        SpatialManualTool::Rect2D,
                        SpatialManualTool::Measure,
                    ] {
                        let act = tool.is_active(self.active_canvas_mode, universal_canvas, build_view);
                        if Self::ribbon_tool_tile(
                            ui,
                            theme,
                            tool.icon(),
                            tool.short_label(),
                            act,
                            theme.accent_orange,
                            tool.tooltip(),
                        )
                        .clicked()
                        {
                            let sub = tool.apply(self, universal_canvas, build_view);
                            actions.merge(sub);
                        }
                    }
                    for ext in [
                        UniversalCanvasTool::Freehand,
                        UniversalCanvasTool::Circle,
                        UniversalCanvasTool::Dimension,
                        UniversalCanvasTool::Callout,
                    ] {
                        let ext_act = universal_canvas.active_tool == ext;
                        if Self::ribbon_tool_tile(
                            ui,
                            theme,
                            ext.app_icon(),
                            ext.short_label(),
                            ext_act,
                            theme.accent_orange,
                            ext.tooltip(),
                        )
                        .clicked()
                        {
                            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                            universal_canvas.globe_mode = true;
                            build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                            universal_canvas.earth_bim_author.active_tool =
                                crate::views::universal_canvas::EarthBimTool::None;
                            universal_canvas.active_tool = ext;
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::ArrowRight,
                        "To BIM",
                        false,
                        theme.accent_green,
                        "Promote selected 2D vector lines/rects on Earth 3D into BIM Walls/Beams/Slabs",
                    )
                    .clicked()
                    {
                        let (w, r, s) =
                            universal_canvas.promote_selection_to_bim(&mut build_view.bim_bus.project);
                        if w + r + s > 0 {
                            actions.sync_fem_and_earth = true;
                            self.push_trace(format!("Promoted +{w}W +{r}B +{s}S on Earth 3D"), true);
                        }
                    }
                });
                Self::ribbon_group(ui, theme, "Sheets & Diff", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::RulerSquare,
                        "Sheet 2D",
                        self.active_overlay_pillar == Some(NavigationPillar::ParametricDED),
                        theme.accent_blue,
                        "Open 2. Drafting & Parametric DED Studio sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::ParametricDED);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Document,
                        "Brief S-115",
                        false,
                        theme.accent_green,
                        "Export Archival A3 Sheet S-115 Feasibility Brief (.svg)",
                    )
                    .clicked()
                    {
                        actions.export_feasibility_s115 = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Book,
                        "Index S-000",
                        false,
                        theme.accent_green,
                        "Export Master Engineering Dossier Drawing Index S-000 (.svg)",
                    )
                    .clicked()
                    {
                        actions.export_master_dossier_svg = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wrench,
                        "MEP S-116",
                        false,
                        theme.accent_blue,
                        "Export Archival A3 Sheet S-116 Building MEP Systems Annex (.svg)",
                    )
                    .clicked()
                    {
                        actions.export_mep_s116 = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::AlertTriangle,
                        "Clash S-117",
                        false,
                        theme.accent_orange,
                        "Export Archival A3 Sheet S-117 3D Clash & BCF 2.1 Coordination Annex (.svg)",
                    )
                    .clicked()
                    {
                        actions.export_clash_s117 = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Layers,
                        "City S-118",
                        false,
                        theme.accent_green,
                        "Export Archival A3 Sheet S-118 Subsurface Utilities & Smart Intersections Annex (.svg)",
                    )
                    .clicked()
                    {
                        actions.export_city_infra_s118 = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Layers,
                        "IFCX Diff",
                        self.ifcx_diff_drawer_open,
                        theme.accent_orange,
                        "Open Multi-Layer IFCX Visual Diff Inspector",
                    )
                    .clicked()
                    {
                        self.toggle_exclusive_drawer("ifcx_diff");
                        actions.run_ifcx_visual_diff = true;
                    }
                });
            }

            RibbonCategoryTab::Modeling => {
                Self::ribbon_group(ui, theme, "Select", |ui| {
                    let act = SpatialManualTool::OrbitPan
                        .is_active(self.active_canvas_mode, universal_canvas, build_view);
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SelectArrow,
                        "Modify",
                        act,
                        theme.accent_blue,
                        "Orbit Earth / Select & inspect 3D BIM elements",
                    )
                    .clicked()
                    {
                        let sub =
                            SpatialManualTool::OrbitPan.apply(self, universal_canvas, build_view);
                        actions.merge(sub);
                    }
                });

                Self::ribbon_group(ui, theme, "Build", |ui| {
                    for (tool, lbl) in [
                        (SpatialManualTool::Wall, "Wall"),
                        (SpatialManualTool::Door, "Door"),
                        (SpatialManualTool::Window, "Window"),
                        (SpatialManualTool::Column, "Column"),
                        (SpatialManualTool::Beam, "Beam"),
                        (SpatialManualTool::Slab, "Floor"),
                        (SpatialManualTool::Footing, "Footing"),
                    ] {
                        let act = tool.is_active(self.active_canvas_mode, universal_canvas, build_view);
                        if Self::ribbon_tool_tile(
                            ui,
                            theme,
                            tool.icon(),
                            lbl,
                            act,
                            theme.accent_orange,
                            tool.tooltip(),
                        )
                        .clicked()
                        {
                            let sub = tool.apply(self, universal_canvas, build_view);
                            actions.merge(sub);
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Building,
                        "Massing",
                        self.massing_drawer_open || universal_canvas.massing_overlay_visible,
                        theme.accent_blue,
                        "Open Multi-Option Massing Explorer & 1-Click Mass-to-BIM",
                    )
                    .clicked()
                    {
                        self.open_massing_drawer(0);
                        actions.run_massing_study = true;
                    }
                });

                Self::ribbon_group(ui, theme, "Analysis", |ui| {
                    for (lens, icon, lbl, tip) in [
                        (SpatialLens::Clean, AppIcon::Globe, "Clean", "Lens: Clean Architectural"),
                        (SpatialLens::DcrHeatmap, AppIcon::ChartBar, "DCR%", "Lens: LRFD DCR% Heatmap"),
                        (SpatialLens::EigenGateRedundancy, AppIcon::Shield, "EigenGate", "Lens: EigenGate Redundancy"),
                        (SpatialLens::BendingMoment, AppIcon::ChartLine, "Moment", "Lens: Bending Moment BMD"),
                        (SpatialLens::DeformedShape, AppIcon::NodesGraph, "Deformed", "Lens: 6-DOF Deformed"),
                        (SpatialLens::ModalVibration, AppIcon::Pulse, "Modal", "Lens: Modal Eigenmode"),
                    ] {
                        if Self::ribbon_tool_tile(
                            ui,
                            theme,
                            icon,
                            lbl,
                            self.active_lens == lens,
                            theme.accent_blue,
                            tip,
                        )
                        .clicked()
                        {
                            self.apply_lens_to_views(lens, build_view, universal_canvas);
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Pulse,
                        "Seismic",
                        self.ground_motion_drawer_open,
                        theme.accent_orange,
                        "Open Live Earthquake Replay & Async StructGNN-CRC drawer",
                    )
                    .clicked()
                    {
                        self.open_ground_motion_drawer();
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wind,
                        "Wind",
                        self.wind_drawer_open,
                        theme.accent_blue,
                        "Open Wind Aeroelasticity & ISO 10137 Comfort Studio",
                    )
                    .clicked()
                    {
                        self.open_wind_drawer(0);
                        actions.run_wind_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wall,
                        "ShearWall",
                        self.shear_wall_drawer_open,
                        theme.accent_orange,
                        "Open Special RC Shear Wall & Coupling Beam Studio",
                    )
                    .clicked()
                    {
                        self.open_shear_wall_drawer(0);
                        actions.run_shear_wall_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wrench,
                        "SteelConn",
                        self.connection_drawer_open,
                        theme.accent_orange,
                        "Open Steel Seismic Connection & Panel-Zone Studio",
                    )
                    .clicked()
                    {
                        self.open_connection_drawer(0);
                        actions.run_connection_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::ArcCurve,
                        "PT Tendon",
                        self.prestress_drawer_open,
                        theme.accent_blue,
                        "Open Post-Tensioned Tendon Drape & Camber Studio",
                    )
                    .clicked()
                    {
                        self.open_prestress_drawer(0);
                        actions.run_prestress_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Grid,
                        "Composite",
                        self.composite_drawer_open,
                        theme.accent_blue,
                        "Open Composite Deck & AISC DG11 Studio",
                    )
                    .clicked()
                    {
                        self.open_composite_drawer(0);
                        actions.run_composite_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Hammer,
                        "Foundation",
                        self.foundation_drawer_open,
                        theme.accent_green,
                        "Open Deep Foundation Pile Group & Punching Studio",
                    )
                    .clicked()
                    {
                        self.open_foundation_drawer(0);
                        actions.run_foundation_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Bolt,
                        "Sim Sheet",
                        self.active_overlay_pillar == Some(NavigationPillar::AnalysisSimulation),
                        theme.accent_orange,
                        "Open Geo-Structural Analysis & Multi-Hazard module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::AnalysisSimulation);
                    }
                });

                Self::ribbon_group(ui, theme, "Datum", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Layers,
                        &story_lbl,
                        false,
                        theme.accent_blue,
                        &format!("Active Story: {story_lbl} (click to cycle levels)"),
                    )
                    .clicked()
                    {
                        self.active_story_idx = (self.active_story_idx + 1) % lvl_cnt.max(1);
                        if universal_canvas.section_box.enabled
                            && universal_canvas.section_box.level_cut_mode
                        {
                            universal_canvas
                                .section_box
                                .apply_level_cut(&build_view.bim_bus.project, self.active_story_idx);
                        }
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Plus,
                        "Level",
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
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::ZoomFit,
                        "Fit View",
                        false,
                        theme.accent_blue,
                        "Fit Earth 3D camera to active BIM structure",
                    )
                    .clicked()
                    {
                        actions.fit_camera = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Cpu,
                        "WebGPU",
                        universal_canvas.webgpu_earth.is_active(),
                        theme.accent_blue,
                        "Toggle 100% Pure WebGPU (WGSL) 3D Vector Earth pass",
                    )
                    .clicked()
                    {
                        universal_canvas.webgpu_earth.enabled =
                            !universal_canvas.webgpu_earth.enabled;
                    }
                });

                Self::ribbon_group(ui, theme, "Section", |ui| {
                    let sb = &universal_canvas.section_box;
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SectionCut,
                        "Sec Box",
                        sb.enabled && !sb.level_cut_mode,
                        theme.accent_blue,
                        "Toggle real-time WebGPU 3D Section Box clipping planes",
                    )
                    .clicked()
                    {
                        universal_canvas.toggle_section_box(&build_view.bim_bus.project);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Layers,
                        "Lvl Cut",
                        universal_canvas.section_box.enabled
                            && universal_canvas.section_box.level_cut_mode,
                        theme.accent_green,
                        "Toggle horizontal Active Level Cut Plane above the active story",
                    )
                    .clicked()
                    {
                        universal_canvas
                            .toggle_level_cut(&build_view.bim_bus.project, self.active_story_idx);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::ZoomFit,
                        "Fit Box",
                        false,
                        theme.accent_blue,
                        "Fit 3D Section Box extents around the active BIM building",
                    )
                    .clicked()
                    {
                        universal_canvas
                            .section_box
                            .fit_to_bim_project(&build_view.bim_bus.project, 2.5);
                        universal_canvas.section_box.enabled = true;
                        universal_canvas.section_box.level_cut_mode = false;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Mountain,
                        "Site Cut",
                        universal_canvas.section_box.clip_terrain_and_city,
                        theme.accent_orange,
                        "Toggle clipping of IfcSite terrain, coastal ocean & city buildings",
                    )
                    .clicked()
                    {
                        let next = !universal_canvas.section_box.clip_terrain_and_city;
                        universal_canvas.section_box.clip_terrain_and_city = next;
                        if next && !universal_canvas.section_box.enabled {
                            universal_canvas
                                .section_box
                                .fit_to_bim_project(&build_view.bim_bus.project, 6.0);
                            universal_canvas.section_box.enabled = true;
                        }
                    }
                });

                Self::ribbon_group(ui, theme, "MEP Systems", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wrench,
                        "MEP Studio",
                        self.mep_drawer_open,
                        theme.accent_blue,
                        "Open Building MEP Systems Studio (HVAC, Plumbing, Electrical & Fire)",
                    )
                    .clicked()
                    {
                        self.open_mep_drawer(0);
                        actions.run_mep_systems = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Layers,
                        "MEP 3D",
                        universal_canvas.mep_overlay_visible,
                        theme.accent_green,
                        "Toggle 3D Pure WebGPU MEP ducts, pipes, cable trays & core risers",
                    )
                    .clicked()
                    {
                        actions.toggle_mep_3d = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wind,
                        "HVAC",
                        self.mep_drawer_open && self.mep_drawer_tab == 0,
                        theme.accent_blue,
                        "Evaluate ASHRAE HVAC cooling & supply duct sizing",
                    )
                    .clicked()
                    {
                        self.open_mep_drawer(0);
                        actions.run_mep_systems = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Flame,
                        "Fire",
                        self.mep_drawer_open && self.mep_drawer_tab == 1,
                        theme.accent_red,
                        "Inspect NFPA 13 Fire Sprinkler heads, pump & 3D risers",
                    )
                    .clicked()
                    {
                        self.open_mep_drawer(1);
                        actions.run_mep_systems = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::AlertTriangle,
                        "Clash",
                        self.clash_drawer_open,
                        theme.accent_orange,
                        "Open 3D Structure × MEP Clash Detection & BCF Guardrail",
                    )
                    .clicked()
                    {
                        self.open_clash_drawer(0);
                        actions.run_clash_guardrail = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Target,
                        "Clash 3D",
                        universal_canvas.clash_overlay_visible,
                        theme.accent_red,
                        "Toggle 3D Pure WebGPU Clash Beacons & interference prisms",
                    )
                    .clicked()
                    {
                        actions.toggle_clash_3d = true;
                    }
                });
            }

            RibbonCategoryTab::Scheduling => {
                Self::ribbon_group(ui, theme, "4D Phasing", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Play,
                        "4D Lens",
                        self.active_lens == SpatialLens::Erection4D,
                        theme.accent_blue,
                        "Switch 3D model to 4D Phased Erection Lens",
                    )
                    .clicked()
                    {
                        self.apply_lens_to_views(
                            SpatialLens::Erection4D,
                            build_view,
                            universal_canvas,
                        );
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Clock,
                        "4D CPM",
                        self.construction_lca_drawer_open
                            && self.construction_lca_drawer_tab == 0,
                        theme.accent_blue,
                        "Open 4D CPM Construction Sequence & 5D S-Curve Studio",
                    )
                    .clicked()
                    {
                        self.open_construction_lca_drawer(0);
                        actions.run_construction_lca_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Cube3D,
                        "3D Phase",
                        false,
                        theme.accent_green,
                        "Toggle 3D Pure WebGPU Construction Sequence overlay",
                    )
                    .clicked()
                    {
                        actions.toggle_construction_3d = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Document,
                        "Sheet S-114",
                        false,
                        theme.accent_green,
                        "Export 4D/5D/6D Construction Sequence Sheet S-114 (.svg)",
                    )
                    .clicked()
                    {
                        actions.export_construction_svg = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SlidersPanel,
                        "4D Sheet",
                        self.active_overlay_pillar == Some(NavigationPillar::TimeScheduling),
                        theme.accent_blue,
                        "Open 4. Scheduling & 4D Phased Erection module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::TimeScheduling);
                    }
                });
            }

            RibbonCategoryTab::Budgeting => {
                Self::ribbon_group(ui, theme, "5D Costing", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Coins,
                        "S-Curve",
                        self.construction_lca_drawer_open,
                        theme.accent_orange,
                        "Open 5D Cost S-Curve & QTO Studio",
                    )
                    .clicked()
                    {
                        self.open_construction_lca_drawer(0);
                        actions.run_construction_lca_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::ChartLine,
                        "Portfolio",
                        self.portfolio_drawer_open,
                        theme.accent_green,
                        "Open District Portfolio Budget Capex Pareto Optimizer",
                    )
                    .clicked()
                    {
                        universal_canvas.portfolio_retrofit_config.overlay_visible = true;
                        self.toggle_exclusive_drawer("portfolio");
                        actions.run_portfolio_retrofit = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wrench,
                        "Rebar BBS",
                        self.field_qaqc_drawer_open,
                        theme.accent_blue,
                        "Open 1D Rebar Bar-Bending Schedule & Stock Optimization",
                    )
                    .clicked()
                    {
                        self.open_field_qaqc_drawer(0);
                        actions.run_field_fabrication =
                            Some(universal_canvas.field_fab_config.stock_length_m);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Document,
                        "5D Sheet",
                        self.active_overlay_pillar == Some(NavigationPillar::CostBudgeting),
                        theme.accent_blue,
                        "Open 5. Budgeting & 5D Heatmap Costing module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::CostBudgeting);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Book,
                        "Reports",
                        self.active_overlay_pillar == Some(NavigationPillar::ReportPublishing),
                        theme.accent_green,
                        "Open Report Publishing Suite",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::ReportPublishing);
                    }
                });
            }

            RibbonCategoryTab::Sustainability => {
                Self::ribbon_group(ui, theme, "Solar & OTTV", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Sun,
                        "Solar OTTV",
                        self.solar_drawer_open && self.solar_drawer_tab == 0,
                        theme.accent_orange,
                        "Open 3D Solar Irradiation, Cast Shadow & SNI 6389 OTTV Studio",
                    )
                    .clicked()
                    {
                        self.open_solar_drawer(0);
                        actions.run_solar_envelope = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::ChartBar,
                        "Orientations",
                        self.solar_drawer_open && self.solar_drawer_tab == 1,
                        theme.accent_orange,
                        "Inspect N/E/S/W/Roof OTTV & Irradiation breakdown",
                    )
                    .clicked()
                    {
                        self.open_solar_drawer(1);
                        actions.run_solar_envelope = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Cube3D,
                        "3D Heatmap",
                        universal_canvas.solar_overlay_visible,
                        theme.accent_orange,
                        "Toggle 3D Pure WebGPU Solar Thermal Heatmap & Cast Shadow polygon",
                    )
                    .clicked()
                    {
                        actions.toggle_solar_3d = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SaveDisk,
                        "Solar IFCX",
                        false,
                        theme.accent_green,
                        "Export non-destructive eigengraph::design::solar_ottv IFCX",
                    )
                    .clicked()
                    {
                        actions.export_solar_ifcx = true;
                    }
                });
                Self::ribbon_group(ui, theme, "Daylight & Interior", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Sun,
                        "Daylight",
                        self.daylight_drawer_open && self.daylight_drawer_tab == 0,
                        theme.accent_green,
                        "Open Interior Daylight Autonomy (sDA/ASE/DGP), LEED v4.1 & Finishes Studio",
                    )
                    .clicked()
                    {
                        self.open_daylight_drawer(0);
                        actions.run_daylight_interior = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Grid,
                        "Lux Grid",
                        self.daylight_drawer_open && self.daylight_drawer_tab == 1,
                        theme.accent_green,
                        "Inspect Work-Plane Lux Heatmap & Room Architectural Finishes BOQ",
                    )
                    .clicked()
                    {
                        self.open_daylight_drawer(1);
                        actions.run_daylight_interior = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Cube3D,
                        "3D Lux",
                        universal_canvas.daylight_overlay_visible,
                        theme.accent_green,
                        "Toggle 3D Pure WebGPU Interior Work-Plane Lux Heatmap overlay",
                    )
                    .clicked()
                    {
                        actions.toggle_daylight_3d = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SaveDisk,
                        "Lux IFCX",
                        false,
                        theme.accent_blue,
                        "Export non-destructive eigengraph::design::daylight_interior IFCX",
                    )
                    .clicked()
                    {
                        actions.export_daylight_ifcx = true;
                    }
                });
                Self::ribbon_group(ui, theme, "Carbon LCA", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Leaf,
                        "6D LCA",
                        self.construction_lca_drawer_open
                            && self.construction_lca_drawer_tab == 1,
                        theme.accent_green,
                        "Open 6D Embodied Carbon LCA & Material Substitution Studio",
                    )
                    .clicked()
                    {
                        self.open_construction_lca_drawer(1);
                        actions.run_construction_lca_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Document,
                        "6D Sheet",
                        self.active_overlay_pillar == Some(NavigationPillar::SustainabilityEnergy),
                        theme.accent_green,
                        "Open 6. Sustainability & 6D Heliodon LCA module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar =
                            Some(NavigationPillar::SustainabilityEnergy);
                    }
                });
            }

            RibbonCategoryTab::Facility => {
                Self::ribbon_group(ui, theme, "Digital Twin", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Pulse,
                        "SHM Lens",
                        self.active_lens == SpatialLens::ShmDigitalTwin,
                        theme.accent_blue,
                        "Switch 3D view to Live SHM Digital Twin Lens",
                    )
                    .clicked()
                    {
                        self.apply_lens_to_views(
                            SpatialLens::ShmDigitalTwin,
                            build_view,
                            universal_canvas,
                        );
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Shield,
                        "SHM Twin",
                        self.shm_drawer_open,
                        theme.accent_green,
                        "Open Live SHM Digital Twin & Automated ATC-20 Placard Studio",
                    )
                    .clicked()
                    {
                        self.open_shm_drawer();
                        actions.run_shm_digital_twin = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Layers,
                        "Collab",
                        self.collab_drawer_open,
                        theme.accent_blue,
                        "Open SHA-256 Checkpoints, Trade Study & Governance drawer",
                    )
                    .clicked()
                    {
                        self.open_collab_drawer(0);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Building,
                        "7D Sheet",
                        self.active_overlay_pillar == Some(NavigationPillar::FacilityManagement),
                        theme.accent_blue,
                        "Open 7. Facility & 7D Digital Twin module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::FacilityManagement);
                    }
                });
                Self::ribbon_group(ui, theme, "Prediction", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Target,
                        "Reliability",
                        self.reliability_drawer_open,
                        theme.accent_orange,
                        "Open Structural Reliability FORM/SORM & Fragility Studio",
                    )
                    .clicked()
                    {
                        self.open_reliability_drawer(0);
                        actions.run_reliability_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::ChartLine,
                        "AI Aging",
                        self.active_overlay_pillar == Some(NavigationPillar::Prediction),
                        theme.accent_blue,
                        "Open AI Structural Aging & Prediction module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::Prediction);
                    }
                });
            }

            RibbonCategoryTab::Safety => {
                Self::ribbon_group(ui, theme, "Life Safety", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Shield,
                        "Evacuation",
                        self.evacuation_drawer_open,
                        theme.accent_green,
                        "Compute A* post-disaster safe evacuation & responder routes",
                    )
                    .clicked()
                    {
                        self.open_evacuation_drawer();
                        actions.run_evacuation_routing = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Flame,
                        "Extreme",
                        self.extreme_events_drawer_open,
                        theme.accent_red,
                        "Open Extreme Events Studio (Blast UFC 3-340-02 & Fire ISO 834)",
                    )
                    .clicked()
                    {
                        self.open_extreme_events_drawer(0);
                        actions.run_extreme_events = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Target,
                        "Risk Field",
                        self.collab_drawer_open && self.collab_drawer_tab == 1,
                        theme.accent_orange,
                        "Toggle 3D Pure WebGPU District Risk Field Heatmap",
                    )
                    .clicked()
                    {
                        universal_canvas.collab_state.config.risk_heatmap_visible =
                            !universal_canvas.collab_state.config.risk_heatmap_visible;
                        self.open_collab_drawer(1);
                        actions.refresh_district_risk_field = true;
                    }
                });
                Self::ribbon_group(ui, theme, "Resilience", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Lock,
                        "Isolation",
                        self.isolation_drawer_open,
                        theme.accent_blue,
                        "Open Seismic Base Isolation & Damper Studio",
                    )
                    .clicked()
                    {
                        self.open_isolation_drawer(0);
                        actions.run_isolation_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wrench,
                        "Retrofit",
                        universal_canvas.last_retrofit_report.is_some(),
                        theme.accent_orange,
                        "Run Auto-Retrofit structural strengthening synthesis",
                    )
                    .clicked()
                    {
                        actions.run_structural_retrofit = Some(universal_canvas.retrofit_strategy);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::CheckCircle,
                        "AutoRepair",
                        false,
                        theme.accent_green,
                        "Auto-Repair low-redundancy / overstressed elements cleanly",
                    )
                    .clicked()
                    {
                        actions.run_auto_repair = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Document,
                        "8D Sheet",
                        self.active_overlay_pillar == Some(NavigationPillar::SafetyHealth),
                        theme.accent_blue,
                        "Open 8. Safety & 8D Crane Sweep Risk module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::SafetyHealth);
                    }
                });
            }

            RibbonCategoryTab::Construction => {
                Self::ribbon_group(ui, theme, "Field QA/QC", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Clipboard,
                        "QA/QC",
                        self.field_qaqc_drawer_open,
                        theme.accent_blue,
                        "Open Field QA/QC Waypoints, NCR Logs & Sheet S-103",
                    )
                    .clicked()
                    {
                        self.open_field_qaqc_drawer(0);
                        actions.run_field_fabrication =
                            Some(universal_canvas.field_fab_config.stock_length_m);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Waves,
                        "EigenFlow",
                        self.eigenflow_drawer_open,
                        theme.accent_green,
                        "Open EigenFlow Non-Linear Pushover & Solver Studio",
                    )
                    .clicked()
                    {
                        self.open_eigenflow_drawer(0);
                        actions.run_eigenflow_studio = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Recycle,
                        "9D Sheet",
                        self.active_overlay_pillar == Some(NavigationPillar::LeanConstruction),
                        theme.accent_blue,
                        "Open 9. Construction & 9D Site Logistics module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::LeanConstruction);
                    }
                });
                Self::ribbon_group(ui, theme, "Earthworks", |ui| {
                    for ext in [
                        UniversalCanvasTool::EarthUplift,
                        UniversalCanvasTool::EarthCarve,
                        UniversalCanvasTool::EarthSeawall,
                        UniversalCanvasTool::EarthBridge,
                        UniversalCanvasTool::EarthCorridor,
                    ] {
                        let act = build_view.ground_author_tool == Ground3DAuthorTool::Orbit
                            && universal_canvas.active_tool == ext;
                        if Self::ribbon_tool_tile(
                            ui,
                            theme,
                            ext.app_icon(),
                            ext.short_label(),
                            act,
                            theme.accent_orange,
                            ext.tooltip(),
                        )
                        .clicked()
                        {
                            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                            universal_canvas.globe_mode = true;
                            build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                            universal_canvas.earth_bim_author.active_tool =
                                crate::views::universal_canvas::EarthBimTool::None;
                            universal_canvas.active_tool = ext;
                        }
                    }
                });
            }

            RibbonCategoryTab::Industrialization => {
                Self::ribbon_group(ui, theme, "DfMA & BIM", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Building,
                        "Mass-to-BIM",
                        self.massing_drawer_open,
                        theme.accent_blue,
                        "Open Mass-to-BIM Structural Conversion Studio",
                    )
                    .clicked()
                    {
                        self.open_massing_drawer(0);
                        actions.run_massing_study = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Wrench,
                        "Shop S-103",
                        false,
                        theme.accent_green,
                        "Export A3 Sheet S-103 Pure-Vector Rebar BBS & Field QA/QC Shop Annex",
                    )
                    .clicked()
                    {
                        actions.export_shop_drawing_svg = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Bot,
                        "AI Firm",
                        self.active_overlay_pillar == Some(NavigationPillar::AiAgent),
                        theme.accent_blue,
                        "Open Agent & Virtual Firm module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar = Some(NavigationPillar::AiAgent);
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Cpu,
                        "10D Sheet",
                        self.active_overlay_pillar
                            == Some(NavigationPillar::IndustrializedAutomation),
                        theme.accent_blue,
                        "Open 10. Automation & 10D DfMA Prefab module sheet",
                    )
                    .clicked()
                    {
                        self.active_overlay_pillar =
                            Some(NavigationPillar::IndustrializedAutomation);
                    }
                });
                Self::ribbon_group(ui, theme, "IFCX & Export", |ui| {
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Cube3D,
                        "Export IFCX",
                        false,
                        theme.accent_blue,
                        "Export georeferenced IFCX (IFC 5 JSON ECS) site layer",
                    )
                    .clicked()
                    {
                        actions.export_ifcx = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::Globe,
                        "Earth IFCX",
                        false,
                        theme.accent_green,
                        "Export composed Living Earth Globe + Site IFCX graph",
                    )
                    .clicked()
                    {
                        actions.export_earth_ifcx = true;
                    }
                    if Self::ribbon_tool_tile(
                        ui,
                        theme,
                        AppIcon::SaveDisk,
                        "Save JSON",
                        false,
                        theme.accent_blue,
                        "Export .eigen.json workspace archive",
                    )
                    .clicked()
                    {
                        actions.export_archive = true;
                    }
                });
            }
        }
    }
}
