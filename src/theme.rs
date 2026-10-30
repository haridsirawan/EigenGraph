//! theme.rs — macOS Human Interface Guidelines (HIG) visual system.

use egui::{Color32, CornerRadius, Frame, Margin, Stroke};

#[derive(Debug, Clone)]
pub struct MacTheme {
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
        Self {
            bg_canvas: Color32::from_rgb(22, 22, 24),
            bg_sidebar: Color32::from_rgb(28, 28, 30),
            bg_card: Color32::from_rgb(38, 38, 42),
            bg_card_hover: Color32::from_rgb(48, 48, 54),
            bg_hero: Color32::from_rgb(32, 40, 56),
            accent_blue: Color32::from_rgb(10, 132, 255),
            accent_green: Color32::from_rgb(48, 209, 88),
            accent_orange: Color32::from_rgb(255, 159, 10),
            accent_red: Color32::from_rgb(255, 69, 58),
            text_primary: Color32::from_rgb(255, 255, 255),
            text_secondary: Color32::from_rgb(174, 174, 178),
            text_muted: Color32::from_rgb(110, 110, 115),
            border_subtle: Color32::from_rgb(48, 48, 52),
            border_card: Color32::from_rgb(58, 58, 64),
        }
    }
}

impl MacTheme {
    /// Card frame with macOS HIG rounded corners and subtle stroke
    pub fn card_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_card)
            .corner_radius(CornerRadius::same(14))
            .stroke(Stroke::new(1.0, self.border_card))
            .inner_margin(Margin::same(16))
    }

    /// Hero Featured Banner Frame
    pub fn hero_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_hero)
            .corner_radius(CornerRadius::same(18))
            .stroke(Stroke::new(1.5, self.accent_blue.gamma_multiply(0.5)))
            .inner_margin(Margin::same(24))
    }

    /// Sidebar Translucent Frame
    pub fn sidebar_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_sidebar)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .inner_margin(Margin::symmetric(12, 16))
    }

    /// Full-height Apple macOS Left Navigation Sidebar Frame
    pub fn mac_sidebar_frame(&self) -> Frame {
        Frame::new()
            .fill(self.bg_sidebar)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .inner_margin(Margin::symmetric(12, 14))
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

    /// Floating Dynamic Island / Pill Frame (Option B - Collapsed Capsule)
    pub fn floating_pill_frame(&self) -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(28, 28, 32, 235))
            .corner_radius(CornerRadius::same(22))
            .stroke(Stroke::new(1.2, self.accent_blue.gamma_multiply(0.5)))
            .inner_margin(Margin::symmetric(14, 7))
    }

    /// Floating Dynamic Island Expanded Console Frame (Option B - Expanded Console)
    pub fn floating_island_expanded_frame(&self) -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(24, 24, 28, 248))
            .corner_radius(CornerRadius::same(18))
            .stroke(Stroke::new(1.5, self.accent_blue.gamma_multiply(0.7)))
            .inner_margin(Margin::same(14))
    }
}
