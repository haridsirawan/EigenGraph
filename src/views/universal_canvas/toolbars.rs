//! Universal Canvas toolbars, freeze pill, inspector panel, status footer, and import modal.

use super::*;
use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};

use crate::core::{BimProject, DiskExporter};
use crate::icons::{self, AppIcon};
use crate::models::{NavigationPillar, ParametricVariable, StructuralMember};
use crate::theme::MacTheme;
use crate::ui_kit;

impl UniversalCanvasState {
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_canvas_toolbar(
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
    pub(super) fn render_earth_sandbox_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
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

        let combo_w = crate::ui_kit::clamped_row_width(ui, 64.0, 102.0, 28.0);
        crate::ui_kit::wrap_before(ui, combo_w + 32.0);
        egui::ComboBox::from_id_salt("universal_world_basemap_combo")
            .width(combo_w)
            .selected_text(self.world_map.tile_provider.short_name())
            .show_ui(ui, |ui| {
                for src in TileSource::ALL {
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
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_floating_inspector_panel(
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

    pub(super) fn render_status_footer(
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

    /// Renders the single-file archive import body inside either the Dynamic Island (Spatial Mode)
    /// or a classic window. Returns `true` when the user completes or cancels import.
    pub fn render_import_modal_body(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        bim_project: &mut BimProject,
        members: &mut Vec<StructuralMember>,
        variables: &mut Vec<ParametricVariable>,
    ) -> bool {
        let mut close_now = false;
        ui.label(
            RichText::new(
                "Paste the contents of a single-file EigenGraph_Unified_Workspace.eigen.json archive to verify its SHA-256 seal and restore all synchronized modules.",
            )
            .size(11.5)
            .color(theme.text_secondary),
        );
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("import_unified_workspace_scroll")
            .max_height(220.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.import_json_buffer)
                        .desired_width(ui.available_width())
                        .desired_rows(10),
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
        close_now
    }

    pub(super) fn render_import_modal(
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
                close_now = self.render_import_modal_body(ui, theme, bim_project, members, variables);
            });
        self.import_json_open = open && !close_now;
    }
}
