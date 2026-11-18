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

/// Outcome returned by [`render_obsidian_vault_panel`].
#[derive(Debug, Clone, Default)]
pub struct ObsidianVaultOutcome {
    pub state_changed: bool,
    pub connected_now: bool,
    pub feedback: Option<String>,
    pub enter_guest_mode: bool,
    pub open_startup_modal: bool,
}

fn render_open_folder_block(
    ui: &mut Ui,
    theme: &MacTheme,
    path_input: &mut String,
    compact: bool,
    btn_size: Vec2,
    font_sz: f32,
    desc_sz: f32,
    out: &mut ObsidianVaultOutcome,
) {
    use crate::icons::{self, AppIcon};
    if !compact {
        icons::icon_label(
            ui,
            AppIcon::Folder,
            14.0,
            theme.accent_blue,
            "1. Open Folder as Vault",
            13.0,
            theme.text_primary,
            true,
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(if cfg!(target_arch = "wasm32") {
                "Pick any local Finder/OS folder via Browser Folder Picker (Chrome, Edge, Firefox, Safari) or open Browser Storage."
            } else {
                "Open an existing local directory as your Engineering Vault."
            })
            .size(desc_sz)
            .color(theme.text_secondary),
        );
        ui.add_space(6.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.horizontal_wrapped(|ui| {
            if !compact {
                ui.label(
                    RichText::new("Folder Path:")
                        .size(font_sz)
                        .color(theme.text_secondary),
                );
            }
            let w = if compact {
                (ui.available_width() - 95.0).clamp(110.0, 240.0)
            } else {
                (ui.available_width() - 210.0).clamp(160.0, 360.0)
            };
            ui.add(
                egui::TextEdit::singleline(path_input)
                    .hint_text("/path/to/EigenGraph_Vault")
                    .desired_width(w),
            );
            if icons::icon_button(
                ui,
                AppIcon::Folder,
                "Browse...",
                font_sz,
                theme.text_primary,
                theme.bg_sidebar,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(7),
                btn_size,
            )
            .clicked()
            {
                if let Some(chosen) =
                    crate::core::web_bridge::pick_folder_native("Select EigenGraph Vault Folder")
                {
                    *path_input = chosen;
                }
            }
        });
        ui.add_space(4.0);
    }

    ui.horizontal_wrapped(|ui| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = path_input;
            if icons::icon_button(
                ui,
                AppIcon::Folder,
                "Open / Add Folder",
                font_sz,
                Color32::WHITE,
                theme.accent_blue,
                Stroke::NONE,
                CornerRadius::same(7),
                btn_size,
            )
            .on_hover_text("Select an OS folder (uses Chrome FS Access or Firefox/Safari folder picker)")
            .clicked()
            {
                crate::core::web_bridge::vault_connect_existing();
                out.state_changed = true;
            }
            if !compact
                && icons::icon_button(
                    ui,
                    AppIcon::Database,
                    "Use Browser Storage",
                    desc_sz,
                    theme.text_primary,
                    theme.bg_card,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(7),
                    btn_size,
                )
                .clicked()
            {
                crate::core::web_bridge::vault_connect_indexeddb("Browser IndexedDB Vault");
                out.state_changed = true;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if icons::icon_button(
                ui,
                AppIcon::Folder,
                "Open / Add Folder",
                font_sz,
                Color32::WHITE,
                theme.accent_blue,
                Stroke::NONE,
                CornerRadius::same(7),
                btn_size,
            )
            .clicked()
            {
                let p = std::path::PathBuf::from(path_input.trim());
                match crate::core::InternalVault::connect_existing_vault(p) {
                    Ok(connected) => {
                        out.feedback = Some(format!("Opened Vault at {}", connected.display()));
                        out.connected_now = true;
                        out.state_changed = true;
                    }
                    Err(err) => {
                        out.feedback = Some(err);
                    }
                }
            }
        }
    });
}

fn render_create_vault_block(
    ui: &mut Ui,
    theme: &MacTheme,
    path_input: &mut String,
    name_input: &mut String,
    compact: bool,
    btn_size: Vec2,
    font_sz: f32,
    desc_sz: f32,
    out: &mut ObsidianVaultOutcome,
) {
    use crate::icons::{self, AppIcon};
    #[cfg(target_arch = "wasm32")]
    let _ = path_input;

    if !compact {
        icons::icon_label(
            ui,
            AppIcon::Plus,
            14.0,
            theme.accent_green,
            "2. Create New Vault",
            13.0,
            theme.text_primary,
            true,
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Creates a clean Obsidian-style Vault organized automatically into Projects/, KnowledgeBase/, Exports/, and AI_Context/.")
                .size(desc_sz)
                .color(theme.text_secondary),
        );
        ui.add_space(6.0);
    }

    ui.horizontal_wrapped(|ui| {
        if !compact {
            ui.label(
                RichText::new("Vault Name:")
                    .size(font_sz)
                    .color(theme.text_secondary),
            );
        }
        ui.add(
            egui::TextEdit::singleline(name_input)
                .hint_text("EigenGraph_Vault")
                .desired_width(if compact { 125.0 } else { 180.0 }),
        );

        let clean_name = if name_input.trim().is_empty() {
            "EigenGraph_Vault"
        } else {
            name_input.trim()
        };

        if icons::icon_button(
            ui,
            AppIcon::Plus,
            "Create Vault",
            font_sz,
            Color32::WHITE,
            theme.accent_green,
            Stroke::NONE,
            CornerRadius::same(7),
            btn_size,
        )
        .clicked()
        {
            #[cfg(target_arch = "wasm32")]
            {
                crate::core::web_bridge::vault_create_new(clean_name, false);
                out.feedback = Some(format!(
                    "Creating Obsidian-style Vault '{clean_name}' (Projects/, KnowledgeBase/, Exports/, AI_Context/)."
                ));
                out.state_changed = true;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let base = std::path::PathBuf::from(path_input.trim());
                let target = if base.as_os_str().is_empty() {
                    std::path::PathBuf::from(clean_name)
                } else if base.file_name().and_then(|s| s.to_str()) == Some(clean_name) {
                    base
                } else {
                    base.join(clean_name)
                };
                match crate::core::InternalVault::create_new_vault(
                    target,
                    Some(clean_name.to_string()),
                    false,
                ) {
                    Ok(created) => {
                        *path_input = created.to_string_lossy().to_string();
                        out.feedback = Some(format!("Created Vault at {}", created.display()));
                        out.connected_now = true;
                        out.state_changed = true;
                    }
                    Err(err) => {
                        out.feedback = Some(err);
                    }
                }
            }
        }
    });
}

/// Shared Obsidian-style Engineering Vault controls used across the Startup Modal,
/// Settings Vault card, and Dynamic Island popover.
/// Keeps Vault management cleanly deduplicated with two intuitive workflows:
/// **Open / Add Folder** and **Create Vault** (auto-organizing `Projects/`,
/// `KnowledgeBase/`, `Exports/`, and `AI_Context/` internally).
pub fn render_obsidian_vault_panel(
    ui: &mut Ui,
    theme: &MacTheme,
    path_input: &mut String,
    name_input: &mut String,
    compact: bool,
    show_guest_option: bool,
    show_open_modal_btn: bool,
) -> ObsidianVaultOutcome {
    use crate::icons::{self, AppIcon};
    let mut out = ObsidianVaultOutcome::default();
    let status = crate::core::InternalVault::connection_status();
    let btn_size = if compact { Vec2::new(0.0, 22.0) } else { Vec2::ZERO };
    let font_sz = if compact { 10.0 } else { 11.5 };
    let desc_sz = if compact { 9.8 } else { 11.0 };

    if status.needs_permission {
        ui.horizontal_wrapped(|ui| {
            if icons::icon_button(
                ui,
                AppIcon::Folder,
                "Re-authorize Folder Access",
                font_sz,
                Color32::WHITE,
                theme.accent_orange,
                Stroke::NONE,
                CornerRadius::same(7),
                btn_size,
            )
            .clicked()
            {
                crate::core::web_bridge::vault_reconnect();
                out.state_changed = true;
            }
        });
        ui.add_space(4.0);
    }

    if compact {
        render_open_folder_block(
            ui, theme, path_input, compact, btn_size, font_sz, desc_sz, &mut out,
        );
        ui.add_space(4.0);
        render_create_vault_block(
            ui, theme, path_input, name_input, compact, btn_size, font_sz, desc_sz, &mut out,
        );
    } else {
        Frame::new()
            .fill(theme.bg_sidebar)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                render_open_folder_block(
                    ui, theme, path_input, compact, btn_size, font_sz, desc_sz, &mut out,
                );
            });
        ui.add_space(8.0);
        Frame::new()
            .fill(theme.bg_sidebar)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                render_create_vault_block(
                    ui, theme, path_input, name_input, compact, btn_size, font_sz, desc_sz,
                    &mut out,
                );
            });
    }

    if status.connected || show_open_modal_btn {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if status.connected {
                if icons::icon_button(
                    ui,
                    AppIcon::Undo,
                    "Refresh",
                    desc_sz,
                    theme.text_primary,
                    theme.bg_sidebar,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(7),
                    btn_size,
                )
                .clicked()
                {
                    crate::core::web_bridge::vault_refresh();
                    out.state_changed = true;
                }
                #[cfg(target_arch = "wasm32")]
                if icons::icon_button(
                    ui,
                    AppIcon::SaveDisk,
                    "Export Backup",
                    desc_sz,
                    theme.accent_blue,
                    theme.bg_sidebar,
                    Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.45)),
                    CornerRadius::same(7),
                    btn_size,
                )
                .on_hover_text("Download a complete .eigen-vault.json archive to Mac/Disk (ideal for Firefox & Safari)")
                .clicked()
                {
                    crate::core::web_bridge::vault_download_bundle(&status.vault_name);
                }
                if icons::icon_button(
                    ui,
                    AppIcon::Close,
                    "Disconnect",
                    desc_sz,
                    theme.accent_red,
                    theme.bg_sidebar,
                    Stroke::new(1.0, theme.accent_red.gamma_multiply(0.5)),
                    CornerRadius::same(7),
                    btn_size,
                )
                .clicked()
                {
                    let _ = crate::core::InternalVault::disconnect_vault();
                    out.feedback = Some("Personal Vault disconnected.".to_string());
                    out.state_changed = true;
                }
            }
            if show_open_modal_btn
                && icons::icon_button(
                    ui,
                    AppIcon::Sparkles,
                    "Open Setup Pop-up",
                    desc_sz,
                    theme.accent_blue,
                    theme.bg_sidebar,
                    Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.45)),
                    CornerRadius::same(7),
                    btn_size,
                )
                .clicked()
            {
                out.open_startup_modal = true;
            }
        });
    }

    if show_guest_option {
        ui.add_space(8.0);
        Frame::new()
            .fill(theme.bg_sidebar)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        14.0,
                        theme.accent_orange,
                        "3. Explore Workspace First (Guest Mode)",
                        13.0,
                        theme.text_primary,
                        true,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::ArrowRight,
                            "Explore First",
                            11.5,
                            theme.text_primary,
                            theme.bg_card,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(7),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            crate::core::InternalVault::enter_guest_explore_mode();
                            out.enter_guest_mode = true;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Explore the fullscreen Earth & BIM canvas immediately. You can connect or create an Obsidian-style Vault anytime from Dynamic Island or Settings.")
                        .size(11.0)
                        .color(theme.text_muted),
                );
            });
    }

    out
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
