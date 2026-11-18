//! theme.rs — Modern macOS Sequoia / Pro Dark & Engineering Daylight Visual System.

use egui::{Color32, CornerRadius, Frame, Margin, Stroke, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    ProDark,
    EngineeringLight,
}

impl ThemeMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::ProDark => "Pro Obsidian Dark",
            Self::EngineeringLight => "Engineering Daylight",
        }
    }

    pub fn app_icon(self) -> crate::icons::AppIcon {
        match self {
            Self::ProDark => crate::icons::AppIcon::Moon,
            Self::EngineeringLight => crate::icons::AppIcon::Sun,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MacTheme {
    pub mode: ThemeMode,
    pub bg_canvas: Color32,
    pub bg_sidebar: Color32,
    pub bg_card: Color32,
    pub bg_card_hover: Color32,
    pub bg_hero: Color32,
    pub accent_blue: Color32,
    pub accent_green: Color32,
    pub accent_orange: Color32,
    pub accent_red: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub border_subtle: Color32,
    pub border_card: Color32,
}

impl Default for MacTheme {
    fn default() -> Self {
        Self::for_mode(ThemeMode::ProDark)
    }
}

impl MacTheme {
    /// Builds a theme palette for the requested [`ThemeMode`].
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::ProDark => Self {
                mode,
                // Coastal Marine Teal & Abyssal Navy palette (#0B1427 -> #2B4963 -> #5B92A5 -> #D0EBE1)
                bg_canvas: Color32::from_rgb(11, 20, 39),
                bg_sidebar: Color32::from_rgb(16, 29, 51),
                bg_card: Color32::from_rgb(24, 43, 68),
                bg_card_hover: Color32::from_rgb(43, 73, 99),
                bg_hero: Color32::from_rgb(32, 58, 82),
                // Coastal marine cyan-teal accents & high-legibility engineering semantics
                accent_blue: Color32::from_rgb(91, 146, 165),
                accent_green: Color32::from_rgb(96, 198, 168),
                accent_orange: Color32::from_rgb(238, 168, 72),
                accent_red: Color32::from_rgb(232, 92, 92),
                // Seafoam-mint typographic hierarchy
                text_primary: Color32::from_rgb(208, 235, 225),
                text_secondary: Color32::from_rgb(160, 200, 198),
                text_muted: Color32::from_rgb(91, 146, 165),
                // Deep steel-teal architectural hairline borders
                border_subtle: Color32::from_rgb(43, 73, 99),
                border_card: Color32::from_rgb(66, 108, 132),
            },
            ThemeMode::EngineeringLight => Self {
                mode,
                // Seafoam daylight drafting paper & Coastal Marine Teal palette (#D0EBE1 -> #5B92A5 -> #2B4963 -> #0B1427)
                bg_canvas: Color32::from_rgb(208, 235, 225),
                bg_sidebar: Color32::from_rgb(192, 223, 213),
                bg_card: Color32::from_rgb(232, 246, 240),
                bg_card_hover: Color32::from_rgb(172, 210, 203),
                bg_hero: Color32::from_rgb(182, 218, 214),
                // Deep steel-blue & teal engineering accents on daylight seafoam surfaces
                accent_blue: Color32::from_rgb(43, 73, 99),
                accent_green: Color32::from_rgb(28, 132, 108),
                accent_orange: Color32::from_rgb(196, 114, 18),
                accent_red: Color32::from_rgb(198, 52, 52),
                // Abyssal navy ink typography
                text_primary: Color32::from_rgb(11, 20, 39),
                text_secondary: Color32::from_rgb(43, 73, 99),
                text_muted: Color32::from_rgb(72, 116, 135),
                // Clean steel-teal drafting hairlines
                border_subtle: Color32::from_rgb(91, 146, 165),
                border_card: Color32::from_rgb(43, 73, 99),
            },
        }
    }

    pub fn is_dark(&self) -> bool {
        self.mode == ThemeMode::ProDark
    }
    /// Card frame with modern rounded corners and subtle architectural border
    pub fn card_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_card)
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, self.border_card))
            .inner_margin(Margin::same(16))
    }

    /// Hero Featured Banner Frame
    pub fn hero_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_hero)
            .corner_radius(CornerRadius::same(14))
            .stroke(Stroke::new(1.2, self.accent_blue.gamma_multiply(0.55)))
            .inner_margin(Margin::same(20))
    }

    /// Sidebar Translucent Frame
    pub fn sidebar_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_sidebar)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .inner_margin(Margin::symmetric(12, 14))
    }

    /// Full-height Apple macOS Left Navigation Sidebar Frame
    pub fn mac_sidebar_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_sidebar)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .inner_margin(Margin::symmetric(12, 12))
    }

    /// Content Top Header Bar Frame (Clean macOS Toolbar)
    pub fn content_top_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_sidebar)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .inner_margin(Margin::symmetric(18, 10))
    }

    /// Bottom Status Bar Frame
    pub fn status_bar_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_sidebar)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .inner_margin(Margin::symmetric(16, 7))
    }

    /// Top Navigation Bar Frame
    pub fn top_bar_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_sidebar)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .inner_margin(Margin::symmetric(16, 10))
    }

    /// Floating Dynamic Island / Pill Frame (Collapsed Liquid Glass Capsule)
    pub fn floating_pill_frame(&self) -> Frame {
        self.glass_capsule_frame(22, Margin::symmetric(14, 7))
    }

    /// Floating Dynamic Island Expanded Console Frame (Liquid Glass Console)
    pub fn floating_island_expanded_frame(&self) -> Frame {
        self.glass_capsule_frame(16, Margin::same(14))
    }

    /// Frosted Liquid Glass outer capsule with higher-opacity translucent backdrop,
    /// Coastal Marine Teal specular rim, and soft ambient drop shadow.
    pub fn glass_capsule_frame(&self, corner_r: u8, pad: Margin) -> Frame {
        let (fill, rim_col, shadow) = if self.is_dark() {
            (
                Color32::from_rgba_unmultiplied(11, 20, 39, 222),
                Color32::from_rgba_unmultiplied(91, 146, 165, 160),
                egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 22,
                    spread: 1,
                    color: Color32::from_black_alpha(125),
                },
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(208, 235, 225, 228),
                Color32::from_rgba_unmultiplied(43, 73, 99, 145),
                egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: Color32::from_rgba_unmultiplied(11, 20, 39, 44),
                },
            )
        };
        Frame::new()
            .fill(fill)
            .corner_radius(CornerRadius::same(corner_r))
            .stroke(Stroke::new(1.15, rim_col))
            .shadow(shadow)
            .inner_margin(pad)
    }

    /// Recessed frosted glass sub-container for grouping related controls inside Dynamic Island.
    pub fn glass_subpill_frame(&self, corner_r: u8, pad: Margin) -> Frame {
        let (fill, rim) = if self.is_dark() {
            (
                Color32::from_rgba_unmultiplied(43, 73, 99, 132),
                Color32::from_rgba_unmultiplied(91, 146, 165, 105),
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(91, 146, 165, 56),
                Color32::from_rgba_unmultiplied(43, 73, 99, 84),
            )
        };
        Frame::new()
            .fill(fill)
            .corner_radius(CornerRadius::same(corner_r))
            .stroke(Stroke::new(0.85, rim))
            .inner_margin(pad)
    }

    /// Returns `(text_color, bg_fill, border_stroke)` for pill chips rendered inside Liquid Glass containers.
    pub fn glass_chip_style(&self, active: bool, active_accent: Color32) -> (Color32, Color32, Stroke) {
        if active {
            if self.is_dark() {
                (
                    Color32::from_rgb(208, 235, 225),
                    active_accent.gamma_multiply(0.90),
                    Stroke::new(1.1, Color32::from_rgba_unmultiplied(208, 235, 225, 210)),
                )
            } else {
                (
                    Color32::WHITE,
                    active_accent.gamma_multiply(0.90),
                    Stroke::new(1.1, Color32::from_rgba_unmultiplied(255, 255, 255, 200)),
                )
            }
        } else if self.is_dark() {
            (
                self.text_primary,
                Color32::from_rgba_unmultiplied(43, 73, 99, 140),
                Stroke::new(0.85, Color32::from_rgba_unmultiplied(91, 146, 165, 95)),
            )
        } else {
            (
                self.text_primary,
                Color32::from_rgba_unmultiplied(232, 246, 240, 195),
                Stroke::new(0.85, Color32::from_rgba_unmultiplied(43, 73, 99, 70)),
            )
        }
    }

    /// Paints a delicate top refractive specular sheen along a Liquid Glass capsule border.
    pub fn paint_glass_specular_sheen(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        corner_r: f32,
    ) {
        if !rect.is_positive() || rect.width() <= corner_r * 2.0 + 4.0 {
            return;
        }
        let sheen_col = if self.is_dark() {
            Color32::from_rgba_unmultiplied(208, 235, 225, 115)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 210)
        };
        let y = rect.top() + 1.2;
        painter.line_segment(
            [
                egui::pos2(rect.left() + corner_r, y),
                egui::pos2(rect.right() - corner_r, y),
            ],
            Stroke::new(1.0, sheen_col),
        );
    }

    /// Apply modern Pro Dark or Engineering Daylight visuals to the global egui Context
    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let is_dark = self.is_dark();
        let mut visuals = if is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = self.bg_canvas;
        visuals.window_fill = self.bg_card;
        visuals.extreme_bg_color = if is_dark {
            Color32::from_rgb(8, 15, 30)
        } else {
            Color32::from_rgb(192, 223, 213)
        };
        visuals.faint_bg_color = if is_dark {
            Color32::from_rgb(18, 33, 55)
        } else {
            Color32::from_rgb(220, 241, 233)
        };

        visuals.selection.bg_fill = self.accent_blue.gamma_multiply(0.32);
        visuals.selection.stroke = Stroke::new(1.0, self.accent_blue);

        visuals.widgets.noninteractive.bg_fill = self.bg_card;
        visuals.widgets.noninteractive.weak_bg_fill = self.bg_card;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_secondary);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);

        visuals.widgets.inactive.bg_fill = if is_dark {
            Color32::from_rgb(31, 54, 77)
        } else {
            Color32::from_rgb(184, 219, 210)
        };
        visuals.widgets.inactive.weak_bg_fill = if is_dark {
            Color32::from_rgb(24, 43, 68)
        } else {
            Color32::from_rgb(198, 228, 219)
        };
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.border_card);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(8);

        let hover_fg = if is_dark {
            Color32::WHITE
        } else {
            self.text_primary
        };
        visuals.widgets.hovered.bg_fill = self.bg_card_hover;
        visuals.widgets.hovered.weak_bg_fill = self.bg_card_hover;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_blue);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.2, hover_fg);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(8);

        visuals.widgets.active.bg_fill = self.accent_blue;
        visuals.widgets.active.weak_bg_fill = self.accent_blue;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
        visuals.widgets.active.corner_radius = CornerRadius::same(8);

        visuals.widgets.open.bg_fill = self.bg_card_hover;
        visuals.widgets.open.weak_bg_fill = self.bg_card_hover;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, self.accent_blue);
        visuals.widgets.open.fg_stroke = Stroke::new(1.2, hover_fg);
        visuals.widgets.open.corner_radius = CornerRadius::same(8);

        ctx.set_visuals(visuals);
    }

    /// Viewport width threshold below which phone-first compact stacking applies.
    pub const MOBILE_BREAKPOINT: f32 = 640.0;

    /// Viewport width threshold below which the sidebar collapses into an overlay drawer.
    pub const COMPACT_BREAKPOINT: f32 = 960.0;

    /// Returns the safe content viewport size, clamped against transient zero-sized frames.
    pub fn viewport_size(ctx: &egui::Context) -> Vec2 {
        ctx.content_rect().size().max(Vec2::new(280.0, 320.0))
    }

    /// Returns `true` on narrow mobile viewports (`width < 640px`).
    pub fn is_mobile(ctx: &egui::Context) -> bool {
        Self::viewport_size(ctx).x < Self::MOBILE_BREAKPOINT
    }

    /// Returns `true` on mobile and portrait/narrow tablet viewports (`width < 960px`).
    pub fn is_compact(ctx: &egui::Context) -> bool {
        Self::viewport_size(ctx).x < Self::COMPACT_BREAKPOINT
    }

    /// Clamps a desired modal/window size against an explicit viewport dimension.
    pub fn clamp_modal_for_viewport(vp: Vec2, desired: Vec2) -> Vec2 {
        let safe_vp = vp.max(Vec2::new(280.0, 320.0));
        Vec2::new(
            (safe_vp.x - 24.0).clamp(240.0, desired.x.max(240.0)),
            (safe_vp.y - 48.0).clamp(240.0, desired.y.max(240.0)),
        )
    }

    /// Clamps a desired modal/window size so it always fits inside the current viewport.
    pub fn clamp_modal_size(ctx: &egui::Context, desired: Vec2) -> Vec2 {
        Self::clamp_modal_for_viewport(Self::viewport_size(ctx), desired)
    }

    /// Maximum modal size bounded cleanly inside the active viewport.
    pub fn modal_max_size(ctx: &egui::Context) -> Vec2 {
        let vp = Self::viewport_size(ctx);
        Vec2::new((vp.x - 16.0).max(260.0), (vp.y - 32.0).max(240.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_size_clamps_for_mobile_tablet_and_desktop() {
        let desired = Vec2::new(620.0, 560.0);

        // Mobile viewport (375x667) must clamp horizontally below 375px
        let mobile = MacTheme::clamp_modal_for_viewport(Vec2::new(375.0, 667.0), desired);
        assert_eq!(mobile, Vec2::new(351.0, 560.0));

        // Ultra-compact landscape phone (320x300) still clamps safely
        let small = MacTheme::clamp_modal_for_viewport(Vec2::new(320.0, 300.0), desired);
        assert_eq!(small, Vec2::new(296.0, 272.0));

        // Desktop (1440x900) keeps desired size untouched
        let desktop = MacTheme::clamp_modal_for_viewport(Vec2::new(1440.0, 900.0), desired);
        assert_eq!(desktop, desired);
    }
}
