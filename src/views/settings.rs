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
    pub github_token: String,
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
            github_token: String::new(),
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

            // Group 3: AI Copilot & Cloud / Auto-Updater
            theme.card_frame().show(ui, |ui| {
                ui.label(RichText::new("AI COPILOT & CLOUD / AUTO-UPDATER SYNCHRONIZATION").strong().size(12.0).color(theme.accent_blue));
                ui.separator();
                ui.add_space(8.0);

                ui.label(RichText::new("Google Gemini API Key:").size(13.0).color(theme.text_primary));
                ui.add(egui::TextEdit::singleline(&mut self.gemini_api_key).password(true));
                ui.label(RichText::new("Used for autonomous BIM modeling, load path diagnostics, and structural reports.").size(11.0).color(theme.text_muted));

                ui.add_space(12.0);
                ui.label(RichText::new("GitHub Personal Access Token (PAT) [Opsional]:").size(13.0).color(theme.text_primary));
                ui.add(egui::TextEdit::singleline(&mut self.github_token).password(true));
                ui.label(RichText::new("Digunakan untuk otentikasi update otomatis dari aplikasi jika repositori GitHub berstatus privat.").size(11.0).color(theme.text_muted));
            });

            ui.add_space(16.0);

            // Group 4: About EigenGraph
            theme.card_frame().show(ui, |ui| {
                ui.label(RichText::new("ABOUT EIGENGRAPH").strong().size(12.0).color(theme.accent_blue));
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Version:").strong().size(13.0).color(theme.text_primary));
                    ui.label(RichText::new(format!("v{} (Universal Multi-Platform)", env!("CARGO_PKG_VERSION"))).size(13.0).color(theme.accent_green).strong());
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Author & Lead Developer:").strong().size(12.0).color(theme.text_primary));
                    ui.label(RichText::new("Harids Irawan <harids.irawan@gmail.com>").size(12.0).color(theme.text_secondary));
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Architecture:").strong().size(12.0).color(theme.text_primary));
                    let arch = if cfg!(target_arch = "aarch64") {
                        "Apple Silicon (aarch64)"
                    } else if cfg!(target_arch = "x86_64") {
                        "x86_64 64-bit"
                    } else {
                        "Universal"
                    };
                    let os = if cfg!(target_os = "macos") {
                        "macOS"
                    } else if cfg!(target_os = "windows") {
                        "Windows"
                    } else if cfg!(target_os = "linux") {
                        "Linux"
                    } else {
                        "Desktop"
                    };
                    ui.label(RichText::new(format!("{} • {}", os, arch)).size(12.0).color(theme.text_secondary));
                });

                ui.add_space(8.0);
                ui.label(RichText::new("Theoretical & Computational Foundations:").strong().size(12.0).color(theme.text_primary));
                ui.label(
                    RichText::new(
                        "• Linear Adjusted Redundancy Index (ARI) with Sherman-Morrison O(N²) fast update\n\
                         • Woodbury Rank-2 (N-2) simultaneous multi-element disruption screening\n\
                         • Euler elastic buckling & AISC 360-16 B1 moment amplification geometric stiffness guard\n\
                         • SVD Kinematic Mechanism Guard (m_e = 1 - h_e) for instant singularity detection\n\
                         • Planar Half-Edge Face Arrangement & Non-Recursive BSP Tree Solid CSG Subtraction\n\
                         • 2D Sheet Composer (A4, A3, A2, A1) with Vector PDF 1.4 & SVG Blueprint Generation\n\
                         • Obsidian-Compliant Reactive Autosave Internal Vault with Copilot Integration"
                    )
                    .size(11.0)
                    .color(theme.text_muted)
                );

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Design Standards:").strong().size(11.5).color(theme.text_primary));
                    ui.label(RichText::new("AISC 360-16 • ACI 318-19 • SNI 1729:2020 • SNI 2847:2019 • SNI 1726:2019").size(11.0).color(theme.text_secondary));
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Integrity & License:").strong().size(11.5).color(theme.text_primary));
                    ui.label(RichText::new("NIST FIPS 180-4 SHA-256 Validated • Dual MIT / Apache 2.0").size(11.0).color(theme.text_secondary));
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Repository:").strong().size(11.5).color(theme.text_primary));
                    ui.hyperlink_to("github.com/haridsirawan/EigenGraph", "https://github.com/haridsirawan/EigenGraph");
                });
            });

            ui.add_space(24.0);
        });
    }
}
