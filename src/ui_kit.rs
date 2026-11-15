//! ui_kit.rs — Shared Responsive Layout Primitives & Feature Gate Components for EigenGraph.
//! Replaces rigid `ui.columns(...)` calls with viewport-aware grids, grouped navigation bars,
//! adaptive split workspaces, and standardized locked-feature preview slots.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::features::{FeatureEntry, FeatureStatus};
use crate::theme::MacTheme;

/// Computes responsive column count for card grids based on available width and minimum card width.
pub fn compute_responsive_columns(avail_width: f32, item_count: usize, min_col_width: f32) -> usize {
    if item_count <= 1 || avail_width <= 0.0 || min_col_width <= 0.0 {
        return 1;
    }
    let fit = (avail_width / min_col_width).floor() as usize;
    fit.clamp(1, item_count)
}

/// Renders `count` items inside a responsive multi-row column grid that adapts cleanly
/// from desktop (3-4 columns) -> tablet (2 columns) -> mobile (1 stacked column).
pub fn responsive_grid_n<F>(ui: &mut Ui, count: usize, min_col_width: f32, mut render_cell: F)
where
    F: FnMut(&mut Ui, usize),
{
    if count == 0 {
        return;
    }
    let cols = compute_responsive_columns(ui.available_width(), count, min_col_width);
    if cols == 1 {
        for idx in 0..count {
            render_cell(ui, idx);
            if idx + 1 < count {
                ui.add_space(10.0);
            }
        }
        return;
    }

    let mut idx = 0;
    while idx < count {
        let row_len = (count - idx).min(cols);
        let avail = ui.available_width();
        let gap = ui.spacing().item_spacing.x;
        let col_w = ((avail - gap * (row_len.saturating_sub(1)) as f32) / row_len as f32)
            .floor()
            .max(60.0);
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for c in 0..row_len {
                if c > 0 {
                    ui.add_space(gap);
                }
                ui.allocate_ui_with_layout(
                    Vec2::new(col_w, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(col_w);
                        ui.set_max_width(col_w);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                        render_cell(ui, idx + c);
                    },
                );
            }
        });
        idx += row_len;
        if idx < count {
            ui.add_space(10.0);
        }
    }
}

/// Renders an equal 50/50 two-column layout on wide screens (`available_width >= breakpoint`),
/// automatically stacking vertically on narrow tablets and phones.
pub fn responsive_two_columns<L, R>(ui: &mut Ui, breakpoint: f32, left_fn: L, right_fn: R)
where
    L: FnOnce(&mut Ui),
    R: FnOnce(&mut Ui),
{
    if ui.available_width() >= breakpoint {
        let mut left_opt = Some(left_fn);
        let mut right_opt = Some(right_fn);
        ui.columns(2, |cols| {
            if let Some(f) = left_opt.take() {
                f(&mut cols[0]);
            }
            if let Some(f) = right_opt.take() {
                f(&mut cols[1]);
            }
        });
    } else {
        left_fn(ui);
        ui.add_space(12.0);
        right_fn(ui);
    }
}

/// Single-closure equal 2-pane responsive helper (`pane == 0` left/top, `pane == 1` right/bottom).
/// Safe when both panes mutate `&mut self` fields.
pub fn responsive_two_panes<F>(ui: &mut Ui, breakpoint: f32, mut render_pane: F)
where
    F: FnMut(&mut Ui, usize),
{
    if ui.available_width() >= breakpoint {
        ui.columns(2, |cols| {
            render_pane(&mut cols[0], 0);
            render_pane(&mut cols[1], 1);
        });
    } else {
        render_pane(ui, 0);
        ui.add_space(12.0);
        render_pane(ui, 1);
    }
}

/// Renders two panes with an asymmetric ratio side-by-side when `available_width >= breakpoint`
/// or stacked vertically on narrower screens.
pub fn split_workspace<L, R>(
    ui: &mut Ui,
    breakpoint: f32,
    left_ratio: f32,
    left_min: f32,
    left_max: f32,
    min_height: f32,
    left_fn: L,
    right_fn: R,
) where
    L: FnOnce(&mut Ui, Option<f32>),
    R: FnOnce(&mut Ui, Option<f32>),
{
    let avail_w = ui.available_width();
    if avail_w >= breakpoint {
        let gap = 12.0;
        let max_allowed = (avail_w - 220.0).max(left_min);
        let left_w = (avail_w * left_ratio).clamp(left_min, left_max.min(max_allowed));
        let right_w = (avail_w - left_w - gap).max(200.0);
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(left_w, min_height),
                Layout::top_down(Align::Min),
                |ui| left_fn(ui, Some(left_w)),
            );
            ui.add_space(gap);
            ui.allocate_ui_with_layout(
                Vec2::new(right_w, min_height),
                Layout::top_down(Align::Min),
                |ui| right_fn(ui, Some(right_w)),
            );
        });
    } else {
        left_fn(ui, None);
        ui.add_space(12.0);
        right_fn(ui, None);
    }
}

/// Single-closure asymmetric split workspace (`pane == 0` left/top, `pane == 1` right/bottom).
/// Safe when both panes access `&mut self`.
pub fn split_workspace_panes<F>(
    ui: &mut Ui,
    breakpoint: f32,
    left_ratio: f32,
    left_min: f32,
    left_max: f32,
    min_height: f32,
    mut render_pane: F,
) where
    F: FnMut(&mut Ui, usize, Option<f32>),
{
    let avail_w = ui.available_width();
    if avail_w >= breakpoint {
        let gap = 12.0;
        let max_allowed = (avail_w - 220.0).max(left_min);
        let left_w = (avail_w * left_ratio).clamp(left_min, left_max.min(max_allowed));
        let right_w = (avail_w - left_w - gap).max(200.0);
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(left_w, min_height),
                Layout::top_down(Align::Min),
                |ui| render_pane(ui, 0, Some(left_w)),
            );
            ui.add_space(gap);
            ui.allocate_ui_with_layout(
                Vec2::new(right_w, min_height),
                Layout::top_down(Align::Min),
                |ui| render_pane(ui, 1, Some(right_w)),
            );
        });
    } else {
        render_pane(ui, 0, None);
        ui.add_space(12.0);
        render_pane(ui, 1, None);
    }
}

/// Renders a clean, wrapping sub-tab pill selector inside a themed header card.
/// On narrow phone widths (< 540px), uses horizontal smooth scrolling so pills stay single-line and readable.
pub fn subtab_bar<T: Copy + PartialEq>(
    ui: &mut Ui,
    theme: &MacTheme,
    id_salt: &str,
    active: &mut T,
    tabs: &[(T, &str)],
) {
    Frame::new()
        .fill(theme.bg_card)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(6))
        .stroke(Stroke::new(1.0, theme.border_subtle))
        .show(ui, |ui| {
            let full_w = ui.available_width().max(120.0);
            ui.set_width(full_w);
            ui.set_max_width(full_w);
            let is_narrow = full_w < 540.0;
            let render_buttons = |ui: &mut Ui, active: &mut T| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                for &(tab, label) in tabs {
                    let is_active = *active == tab;
                    let bg = if is_active {
                        theme.accent_blue
                    } else {
                        Color32::TRANSPARENT
                    };
                    let fg = if is_active {
                        Color32::WHITE
                    } else {
                        theme.text_secondary
                    };
                    let stroke = if is_active {
                        Stroke::new(1.0, theme.accent_blue)
                    } else {
                        Stroke::new(1.0, theme.border_subtle)
                    };

                    let (maybe_icon, clean_label) = crate::icons::split_icon_and_label(label);
                    let clicked = if let Some(icon) = maybe_icon {
                        crate::icons::icon_button(
                            ui,
                            icon,
                            clean_label,
                            11.5,
                            fg,
                            bg,
                            stroke,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 30.0),
                        )
                        .clicked()
                    } else {
                        ui.add(
                            egui::Button::new(RichText::new(clean_label).size(11.5).strong().color(fg))
                                .fill(bg)
                                .stroke(stroke)
                                .corner_radius(CornerRadius::same(7))
                                .min_size(Vec2::new(0.0, 30.0)),
                        )
                        .clicked()
                    };
                    if clicked {
                        *active = tab;
                    }
                }
            };

            if is_narrow {
                ScrollArea::horizontal()
                    .id_salt(id_salt)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| render_buttons(ui, active));
                    });
            } else {
                ui.horizontal_wrapped(|ui| render_buttons(ui, active));
            }
        });
}

/// Forces a clean row break inside `ui.horizontal_wrapped` when `ui.available_size_before_wrap().x < needed_width`,
/// preventing non-atomic widgets like `ComboBox` or `TextEdit` from overflowing past `max_rect.right()`.
pub fn wrap_before(ui: &mut Ui, needed_width: f32) {
    let rem = ui.available_size_before_wrap().x;
    if rem < needed_width && rem < ui.max_rect().width() - 1.0 {
        ui.end_row();
    }
}

/// Computes a responsive control width bounded by the current `ui.max_rect().width()`.
pub fn clamped_row_width(ui: &Ui, min_w: f32, max_w: f32, reserve_w: f32) -> f32 {
    let row_cap = (ui.max_rect().width() - reserve_w).max(min_w.min(ui.max_rect().width()));
    row_cap.min(max_w).max(min_w.min(row_cap))
}

/// Standardized KPI / Telemetry metric card used across all 15 engineering pillars.
pub fn stat_card(
    ui: &mut Ui,
    theme: &MacTheme,
    title: &str,
    value: &str,
    subtitle: &str,
    accent: Color32,
) {
    Frame::new()
        .fill(theme.bg_card)
        .corner_radius(CornerRadius::same(10))
        .stroke(Stroke::new(1.0, theme.border_card))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            let inner_w = ui.available_width().max(40.0);
            ui.set_width(inner_w);
            ui.set_max_width(inner_w);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.label(RichText::new(title).size(10.0).strong().color(theme.text_muted));
            ui.add_space(4.0);
            ui.label(RichText::new(value).size(18.0).strong().color(accent));
            ui.add_space(3.0);
            ui.label(RichText::new(subtitle).size(10.5).color(theme.text_secondary));
        });
}

/// Renders a compact status badge (`ACTIVE` or `LOCKED • Phase 3x`) for workspace headers and toolbars.
pub fn feature_status_badge(ui: &mut Ui, _theme: &MacTheme, status: FeatureStatus) {
    let (icon, label, color) = match status {
        FeatureStatus::Active => (crate::icons::AppIcon::Unlock, "ACTIVE".to_string(), _theme.accent_green),
        FeatureStatus::Locked { target_phase } => (
            crate::icons::AppIcon::Lock,
            format!("LOCKED • {target_phase}"),
            _theme.accent_orange,
        ),
    };
    crate::icons::pill_badge(ui, icon, label, color);
}

/// Renders a complete final-layout preview card for an unlockable feature slot.
/// All input controls and preview tables stay visible in their exact final resting layout,
/// wrapped inside `add_enabled_ui(false, ...)` with a clear `LOCKED` badge & specification banner.
/// Returns `true` if the user toggled prioritization for this capability.
pub fn locked_feature_slot(
    ui: &mut Ui,
    theme: &MacTheme,
    entry: &mut FeatureEntry,
    preview_body: impl FnOnce(&mut Ui),
) -> bool {
    let mut toggled = false;
    let is_locked = !entry.status.is_active();
    let border_col = if is_locked {
        theme.accent_orange.gamma_multiply(0.45)
    } else {
        theme.accent_green.gamma_multiply(0.45)
    };

    Frame::new()
        .fill(theme.bg_card)
        .corner_radius(CornerRadius::same(10))
        .stroke(Stroke::new(1.0, border_col))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("{} — {}", entry.code, entry.title))
                        .size(12.5)
                        .strong()
                        .color(theme.text_primary),
                );
                feature_status_badge(ui, theme, entry.status);

                if is_locked {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (btn_icon, btn_text) = if entry.prioritized {
                            (crate::icons::AppIcon::Target, "Prioritized for Unlock")
                        } else {
                            (crate::icons::AppIcon::Star, "Queue for Unlock")
                        };
                        let btn_fill = if entry.prioritized {
                            theme.accent_blue.gamma_multiply(0.28)
                        } else {
                            theme.bg_sidebar
                        };
                        let btn_fg = if entry.prioritized {
                            theme.accent_blue
                        } else {
                            theme.text_secondary
                        };
                        if crate::icons::icon_button(
                            ui,
                            btn_icon,
                            btn_text,
                            10.5,
                            btn_fg,
                            btn_fill,
                            Stroke::new(1.0, theme.border_card),
                            CornerRadius::same(6),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        {
                            entry.prioritized = !entry.prioritized;
                            toggled = true;
                        }
                    });
                }
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("{}  •  Target Module: {}", entry.description, entry.module_hint))
                    .size(11.0)
                    .color(theme.text_secondary),
            );
            ui.add_space(10.0);

            Frame::new()
                .fill(theme.bg_sidebar)
                .corner_radius(CornerRadius::same(8))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add_enabled_ui(!is_locked, |ui| {
                        preview_body(ui);
                    });
                });
        });

    toggled
}

/// Convenience wrapper that looks up `id` inside [`crate::features::FeatureRegistry`]
/// and renders its standardized locked/active feature preview card.
pub fn feature_slot_card(
    ui: &mut Ui,
    theme: &MacTheme,
    registry: &mut crate::features::FeatureRegistry,
    id: crate::features::FeatureId,
    preview_body: impl FnOnce(&mut Ui),
) -> bool {
    if let Some(entry) = registry.entry_mut(id) {
        locked_feature_slot(ui, theme, entry, preview_body)
    } else {
        false
    }
}

const CANVAS_FROZEN_MEMORY_ID: &str = "eigengraph_canvas_frozen_state";

/// Publishes the workspace-wide Freeze Canvas flag into `egui::Context` temporary memory
/// so nested 2D/3D/map canvases honor vertical page scrolling without zooming when frozen.
pub fn set_canvas_frozen(ctx: &egui::Context, frozen: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(CANVAS_FROZEN_MEMORY_ID), frozen));
}

/// Returns `true` when the user has enabled Freeze Canvas so scroll wheels pass through to page scroll.
/// Defaults to `true` (frozen) in Phase 35 until the user clicks Unfreeze.
pub fn is_canvas_frozen(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new(CANVAS_FROZEN_MEMORY_ID)).unwrap_or(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_responsive_columns_across_viewports() {
        // Mobile (360px) stacks into 1 column
        assert_eq!(compute_responsive_columns(360.0, 4, 210.0), 1);
        // Narrow tablet (640px) renders 2-up
        assert_eq!(compute_responsive_columns(520.0, 4, 210.0), 2);
        // Wide desktop (1100px) renders full 4-up
        assert_eq!(compute_responsive_columns(1100.0, 4, 210.0), 4);
        // Never exceeds actual item count
        assert_eq!(compute_responsive_columns(1400.0, 3, 210.0), 3);
    }
}
