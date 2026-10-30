//! settings.rs — Pillar 5: macOS System Preferences Style.

use egui::{RichText, ScrollArea, Ui};
use crate::theme::MacTheme;

pub struct SettingsView {
    pub unit_system: String,
    pub design_standard: String,
    pub solver_threads: usize,
    pub enable_svd_guard: bool,
    pub enable_anti_leakage: bool,
    pub gemini_api_key: String,
}

impl Default for SettingsView {
    fn default() -> Self {
        Self {
            unit_system: "kN, mm, MPa (Structural Metric)".to_string(),
            design_standard: "SNI 1729 / AISC 360-16".to_string(),
            solver_threads: 8,
            enable_svd_guard: true,
            enable_anti_leakage: true,
            gemini_api_key: String::new(),
        }
    }
}

impl SettingsView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            ui.heading(RichText::new("⚙️ System Preferences").size(22.0).strong().color(theme.text_primary));
            ui.label(RichText::new("Configure units, appearance, solver tolerance, and AI copilot integration.").size(13.0).color(theme.text_secondary));

            ui.add_space(16.0);

            // Group 1: Units & Standards
            theme.card_frame().show(ui, |ui| {
                ui.label(RichText::new("ENGINEERING UNITS & DESIGN STANDARDS").strong().size(12.0).color(theme.accent_blue));
                ui.separator();
                ui.add_space(8.0);

                ui.label(RichText::new("Active Unit System:").size(13.0).color(theme.text_primary));
                ui.label(RichText::new(&self.unit_system).size(12.0).color(theme.text_secondary));

                ui.add_space(8.0);
                ui.label(RichText::new("Design Standard:").size(13.0).color(theme.text_primary));
                ui.label(RichText::new(&self.design_standard).size(12.0).color(theme.text_secondary));
            });

            ui.add_space(16.0);

            // Group 2: Solver Engine & SVD Kinematic Guard
            theme.card_frame().show(ui, |ui| {
                ui.label(RichText::new("SOLVER & NUMERICAL SAFEGUARDS").strong().size(12.0).color(theme.accent_blue));
                ui.separator();
                ui.add_space(8.0);

                ui.checkbox(&mut self.enable_svd_guard, RichText::new("Enable SVD Kinematic Guard (Sherman-Morrison margin check)").color(theme.text_primary));
                ui.label(RichText::new("Instantly detects mechanism formation upon N-1 member damage without pseudoinverse.").size(11.0).color(theme.text_muted));

                ui.add_space(8.0);
                ui.checkbox(&mut self.enable_anti_leakage, RichText::new("Enforce Strict Pre-Analysis Protocol (Leakage-Free)").color(theme.text_primary));
                ui.label(RichText::new("Guarantees screening upper bounds are computed 100% from intact structure facts.").size(11.0).color(theme.text_muted));

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Parallel Solver Threads:").color(theme.text_primary));
                    ui.add(egui::Slider::new(&mut self.solver_threads, 1..=32));
                });
            });

            ui.add_space(16.0);

            // Group 3: AI Copilot & Cloud
            theme.card_frame().show(ui, |ui| {
                ui.label(RichText::new("AI COPILOT & CLOUD SYNCHRONIZATION").strong().size(12.0).color(theme.accent_blue));
                ui.separator();
                ui.add_space(8.0);

                ui.label(RichText::new("Google Gemini API Key:").size(13.0).color(theme.text_primary));
                ui.text_edit_singleline(&mut self.gemini_api_key);
                ui.label(RichText::new("Used for autonomous BIM modeling, load path diagnostics, and structural reports.").size(11.0).color(theme.text_muted));
            });

            ui.add_space(24.0);
        });
    }
}
