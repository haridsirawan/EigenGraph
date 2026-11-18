//! settings.rs — Workspace Settings, Design Standards & Centralized Feature Unlock Roadmap.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, ProgressBar, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::core::DiskExporter;
use crate::features::{FeatureRegistry, FeatureStatus};
use crate::models::NavigationPillar;
use crate::theme::MacTheme;
use crate::ui_kit;

use crate::core::BimProject;
use crate::models::{ParametricVariable, StructuralMember};
use crate::theme::ThemeMode;
use crate::views::universal_canvas::{UniversalCanvasState, WorkspaceCanvasSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSubTab {
    WorkspaceSettings,
    FutureRoadmap,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryFilter {
    All,
    LockedOnly,
    PrioritizedOnly,
    ActiveOnly,
}

pub struct SettingsView {
    pub active_subtab: SettingsSubTab,
    pub unit_system: String,
    pub design_standard: String,
    pub solver_threads: usize,
    pub enable_svd_guard: bool,
    pub enable_anti_leakage: bool,
    pub freeze_by_default: bool,
    pub gemini_api_key: String,
    pub registry_search: String,
    pub registry_filter: RegistryFilter,
    pub pillar_filter: Option<NavigationPillar>,
    pub phase_filter: Option<&'static str>,
    pub export_feedback: Option<String>,
    pub vault_path_input: String,
    pub vault_new_name_input: String,
    pub request_open_vault_popup: bool,
    pub vault_state_changed: bool,
}

impl Default for SettingsView {
    fn default() -> Self {
        Self {
            active_subtab: SettingsSubTab::WorkspaceSettings,
            freeze_by_default: true,
            unit_system: "kN, mm, MPa (Structural Metric)".to_string(),
            design_standard: "SNI 1729 / AISC 360-16".to_string(),
            solver_threads: 8,
            enable_svd_guard: true,
            enable_anti_leakage: true,
            gemini_api_key: String::new(),
            registry_search: String::new(),
            registry_filter: RegistryFilter::All,
            pillar_filter: None,
            phase_filter: None,
            export_feedback: None,
            vault_path_input: crate::core::InternalVault::default_vault_dir()
                .to_string_lossy()
                .to_string(),
            vault_new_name_input: "EigenGraph_Vault".to_string(),
            request_open_vault_popup: false,
            vault_state_changed: false,
        }
    }
}

impl SettingsView {
    /// Captures the active workspace settings snapshot for single-file `.eigen.json` synchronization.
    pub fn workspace_settings_snapshot(
        &self,
        theme: &MacTheme,
        canvas: &UniversalCanvasState,
        ai_copilot_visible: bool,
    ) -> WorkspaceCanvasSettings {
        WorkspaceCanvasSettings {
            theme_mode: theme.mode,
            canvas_height_ratio: canvas.canvas_height_ratio,
            freeze_by_default: self.freeze_by_default,
            show_grid: canvas.show_grid,
            snap_to_grid: canvas.snap_to_grid,
            grid_spacing_mm: canvas.grid_spacing_mm,
            show_bim_base_layer: canvas.show_bim_base_layer,
            show_domain_overlay: canvas.show_domain_overlay,
            sync_all_layers: canvas.sync_all_layers,
            show_floating_inspector: canvas.inspector_open,
            show_ai_copilot: ai_copilot_visible,
            unit_system: self.unit_system.clone(),
            design_standard: self.design_standard.clone(),
            solver_threads: self.solver_threads,
            enable_svd_guard: self.enable_svd_guard,
            enable_anti_leakage: self.enable_anti_leakage,
        }
    }

    /// Applies imported single-file `.eigen.json` workspace settings across theme, AI copilot, and preferences.
    pub fn apply_workspace_settings(
        &mut self,
        settings: &WorkspaceCanvasSettings,
        theme: &mut MacTheme,
        ai_copilot_visible: &mut bool,
    ) {
        if theme.mode != settings.theme_mode {
            *theme = MacTheme::for_mode(settings.theme_mode);
        }
        self.freeze_by_default = settings.freeze_by_default;
        self.unit_system = settings.unit_system.clone();
        self.design_standard = settings.design_standard.clone();
        self.solver_threads = settings.solver_threads;
        self.enable_svd_guard = settings.enable_svd_guard;
        self.enable_anti_leakage = settings.enable_anti_leakage;
        *ai_copilot_visible = settings.show_ai_copilot;
    }

    /// Renders Consolidated Workspace Settings, Future Roadmap & About.
    /// Returns `true` if the user clicked an Inspect action to jump straight to a domain workspace.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &mut MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        ai_copilot_visible: &mut bool,
        bim_project: &BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
        registry: &mut FeatureRegistry,
        active_pillar: &mut NavigationPillar,
    ) -> bool {
        let mut requested_jump = false;

        ui_kit::subtab_bar(
            ui,
            theme,
            "settings_subtabs",
            &mut self.active_subtab,
            &[
                (SettingsSubTab::WorkspaceSettings, "⚙️ Workspace Settings"),
                (SettingsSubTab::FutureRoadmap, "🗺️ Future Roadmap"),
                (SettingsSubTab::About, "ℹ️ About EigenGraph"),
            ],
        );
        ui.add_space(10.0);

        ScrollArea::vertical()
            .id_salt("settings_scroll_body")
            .auto_shrink([false, false])
            .show(ui, |ui| match self.active_subtab {
                SettingsSubTab::WorkspaceSettings => {
                    self.render_preferences(
                        ui,
                        theme,
                        universal_canvas,
                        ai_copilot_visible,
                        bim_project,
                        members,
                        variables,
                    );
                }
                SettingsSubTab::FutureRoadmap => {
                    requested_jump = self.render_feature_registry(ui, theme, registry, active_pillar);
                }
                SettingsSubTab::About => {
                    self.render_about(ui, theme);
                }
            });

        requested_jump
    }

    fn render_feature_registry(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        registry: &mut FeatureRegistry,
        active_pillar: &mut NavigationPillar,
    ) -> bool {
        let mut jump_clicked = false;
        let total = registry.total_count().max(1);
        let active = registry.active_count();
        let locked = registry.locked_count();
        let prioritized = registry.prioritized_count();
        let readiness_pct = (active as f32 / total as f32) * 100.0;

        ui.label(
            RichText::new("Centralized Feature Registry & Unlock Roadmap")
                .size(18.0)
                .strong()
                .color(theme.text_primary),
        );
        ui.label(
            RichText::new(
                "Phase 31 locks all 15 workspace UI layouts across 74 capabilities. Queue locked modules below to target Phases 32–35 logic unlocks without layout churn.",
            )
            .size(12.0)
            .color(theme.text_secondary),
        );
        ui.add_space(12.0);

        // Responsive 4-card KPI strip
        ui_kit::responsive_grid_n(ui, 4, 155.0, |ui, idx| match idx {
            0 => ui_kit::stat_card(
                ui,
                theme,
                "ACTIVE MODULES",
                &format!("{active} / {total}"),
                "Operational engineering capabilities",
                theme.accent_green,
            ),
            1 => ui_kit::stat_card(
                ui,
                theme,
                "LOCKED WORKSPACE SLOTS",
                &format!("{locked}"),
                "Frozen UI layouts awaiting logic unlock",
                theme.accent_orange,
            ),
            2 => ui_kit::stat_card(
                ui,
                theme,
                "PRIORITIZED FOR UNLOCK",
                &format!("{prioritized}"),
                "Flagged for next unlock sprint",
                theme.accent_blue,
            ),
            _ => ui_kit::stat_card(
                ui,
                theme,
                "SUITE READINESS",
                &format!("{readiness_pct:.0}%"),
                "Core + preview slot coverage",
                theme.text_primary,
            ),
        });

        ui.add_space(12.0);

        // Action & filter toolbar
        theme.card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.registry_search)
                        .hint_text("Search code, title, module, or pillar...")
                        .desired_width(220.0),
                );

                for (filter, maybe_ico, label) in [
                    (RegistryFilter::All, None, "All"),
                    (RegistryFilter::LockedOnly, Some(crate::icons::AppIcon::Lock), "Locked Only"),
                    (RegistryFilter::PrioritizedOnly, Some(crate::icons::AppIcon::Target), "Prioritized"),
                    (RegistryFilter::ActiveOnly, Some(crate::icons::AppIcon::Unlock), "Active Only"),
                ] {
                    let selected = self.registry_filter == filter;
                    let fill = if selected {
                        theme.accent_blue
                    } else {
                        theme.bg_sidebar
                    };
                    let fg = if selected {
                        Color32::WHITE
                    } else {
                        theme.text_secondary
                    };
                    let clicked = if let Some(ico) = maybe_ico {
                        crate::icons::icon_button(
                            ui,
                            ico,
                            label,
                            11.0,
                            fg,
                            fill,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                    } else {
                        ui.add(
                            egui::Button::new(RichText::new(label).size(11.0).color(fg))
                                .fill(fill)
                                .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    };
                    if clicked {
                        self.registry_filter = filter;
                    }
                }

                let selected_pillar_label = match self.pillar_filter {
                    Some(p) => p.clean_name().to_string(),
                    None => "All 15 Pillars".to_string(),
                };
                egui::ComboBox::from_id_salt("settings_registry_pillar_filter")
                    .selected_text(selected_pillar_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.pillar_filter.is_none(), "All 15 Pillars")
                            .clicked()
                        {
                            self.pillar_filter = None;
                        }
                        for &p in NavigationPillar::all() {
                            let label = format!("{} {}", p.icon(), p.clean_name());
                            if ui
                                .selectable_label(self.pillar_filter == Some(p), label)
                                .clicked()
                            {
                                self.pillar_filter = Some(p);
                            }
                        }
                    });
            });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("TARGET UNLOCK PHASE:")
                        .size(10.5)
                        .strong()
                        .color(theme.text_muted),
                );
                let all_phases_sel = self.phase_filter.is_none();
                if ui
                    .add(
                        egui::Button::new(RichText::new("All Phases").size(10.5).color(
                            if all_phases_sel {
                                Color32::WHITE
                            } else {
                                theme.text_secondary
                            },
                        ))
                        .fill(if all_phases_sel {
                            theme.accent_blue
                        } else {
                            theme.bg_sidebar
                        })
                        .corner_radius(CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    self.phase_filter = None;
                }
                for &phase in FeatureRegistry::distinct_target_phases() {
                    let count = registry.locked_by_phase_count(phase);
                    let selected = self.phase_filter == Some(phase);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(format!("{phase} ({count})")).size(10.5).color(
                                    if selected {
                                        Color32::WHITE
                                    } else {
                                        theme.text_secondary
                                    },
                                ),
                            )
                            .fill(if selected {
                                theme.accent_blue
                            } else {
                                theme.bg_sidebar
                            })
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        self.phase_filter = if selected { None } else { Some(phase) };
                    }
                }
            });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Star,
                    "Prioritize Phase 32",
                    11.0,
                    theme.accent_blue,
                    theme.accent_blue.gamma_multiply(0.15),
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    registry.prioritize_phase("Phase 32");
                }

                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Star,
                    "Prioritize Phase 33",
                    11.0,
                    theme.accent_blue,
                    theme.accent_blue.gamma_multiply(0.15),
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    registry.prioritize_phase("Phase 33");
                }

                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Eraser,
                    "Clear Queue Flags",
                    11.0,
                    theme.text_secondary,
                    theme.bg_sidebar,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    registry.clear_all_priorities();
                }

                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::Document,
                    "Export Unlock Backlog (.md)",
                    11.0,
                    Color32::WHITE,
                    theme.accent_green,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let md = registry.export_unlock_backlog_markdown();
                    match DiskExporter::write_export_file("EigenGraph_Unlock_Backlog.md", &md) {
                        Ok(dest) => {
                            self.export_feedback = Some(format!("Exported unlock backlog -> {dest}"));
                        }
                        Err(err) => {
                            self.export_feedback = Some(format!("Export failed: {err}"));
                        }
                    }
                }

                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::BoxPackage,
                    "Export Unlock Roadmap (.json)",
                    11.0,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let json = registry.export_unlock_roadmap_json();
                    match DiskExporter::write_export_file("EigenGraph_Unlock_Roadmap.json", &json) {
                        Ok(dest) => {
                            self.export_feedback = Some(format!("Exported structured JSON roadmap -> {dest}"));
                        }
                        Err(err) => {
                            self.export_feedback = Some(format!("JSON export failed: {err}"));
                        }
                    }
                }
            });

            if let Some(msg) = &self.export_feedback {
                ui.add_space(6.0);
                ui.label(RichText::new(msg).size(11.0).color(theme.accent_green));
            }
        });

        ui.add_space(12.0);

        // Domain Pillar Readiness Strip (Click to filter by pillar)
        ui.label(
            RichText::new("15-PILLAR READINESS COVERAGE (CLICK TO FILTER)")
                .size(11.0)
                .strong()
                .color(theme.text_muted),
        );
        ui.add_space(6.0);

        let all_pillars = NavigationPillar::all();
        ui_kit::responsive_grid_n(ui, all_pillars.len(), 210.0, |ui, idx| {
            let pillar = all_pillars[idx];
            let (p_active, p_total) = registry.pillar_counts(pillar);
            let frac = if p_total > 0 {
                p_active as f32 / p_total as f32
            } else {
                0.0
            };
            let is_selected = self.pillar_filter == Some(pillar);
            let stroke = if is_selected {
                Stroke::new(1.5, theme.accent_blue)
            } else {
                Stroke::new(1.0, theme.border_subtle)
            };

            Frame::new()
                .fill(if is_selected {
                    theme.accent_blue.gamma_multiply(0.12)
                } else {
                    theme.bg_card
                })
                .corner_radius(CornerRadius::same(8))
                .stroke(stroke)
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} {}", pillar.icon(), pillar.clean_name()))
                                .size(11.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{p_active}/{p_total}"))
                                    .size(10.5)
                                    .strong()
                                    .color(theme.accent_green),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.add(ProgressBar::new(frac).desired_height(5.0));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui
                            .small_button(if is_selected { "Clear Filter" } else { "Filter" })
                            .clicked()
                        {
                            self.pillar_filter = if is_selected { None } else { Some(pillar) };
                        }
                        if crate::icons::icon_button(
                            ui,
                            crate::icons::AppIcon::ArrowRight,
                            "Open",
                            10.0,
                            theme.accent_blue,
                            theme.bg_sidebar,
                            Stroke::NONE,
                            CornerRadius::same(4),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            *active_pillar = pillar;
                            jump_clicked = true;
                        }
                    });
                });
        });

        ui.add_space(14.0);

        // Capability Entries List
        let q = self.registry_search.trim().to_lowercase();
        ui.label(
            RichText::new("CAPABILITY SLOTS & UNLOCK TARGETS")
                .size(11.0)
                .strong()
                .color(theme.text_muted),
        );
        ui.add_space(6.0);

        let mut shown_count = 0usize;
        for entry in &mut registry.entries {
            if let Some(pf) = self.pillar_filter {
                if entry.pillar != pf {
                    continue;
                }
            }
            if let Some(phase_target) = self.phase_filter {
                if !entry
                    .status
                    .target_phase()
                    .is_some_and(|p| p.eq_ignore_ascii_case(phase_target))
                {
                    continue;
                }
            }
            match self.registry_filter {
                RegistryFilter::All => {}
                RegistryFilter::LockedOnly => {
                    if entry.status.is_active() {
                        continue;
                    }
                }
                RegistryFilter::PrioritizedOnly => {
                    if entry.status.is_active() || !entry.prioritized {
                        continue;
                    }
                }
                RegistryFilter::ActiveOnly => {
                    if !entry.status.is_active() {
                        continue;
                    }
                }
            }
            if !q.is_empty() {
                let hay = format!(
                    "{} {} {} {} {}",
                    entry.code,
                    entry.title,
                    entry.description,
                    entry.module_hint,
                    entry.pillar.clean_name()
                )
                .to_lowercase();
                if !hay.contains(&q) {
                    continue;
                }
            }

            shown_count += 1;
            let border = match entry.status {
                FeatureStatus::Active => theme.border_subtle,
                FeatureStatus::Locked { .. } if entry.prioritized => {
                    theme.accent_blue.gamma_multiply(0.7)
                }
                FeatureStatus::Locked { .. } => theme.accent_orange.gamma_multiply(0.45),
            };

            Frame::new()
                .fill(theme.bg_card)
                .corner_radius(CornerRadius::same(10))
                .stroke(Stroke::new(1.0, border))
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("[{}]", entry.code))
                                .size(11.5)
                                .strong()
                                .color(theme.accent_blue),
                        );
                        ui.label(
                            RichText::new(entry.title)
                                .size(12.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui_kit::feature_status_badge(ui, theme, entry.status);

                        Frame::new()
                            .fill(theme.bg_sidebar)
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        entry.pillar.icon(),
                                        entry.pillar.clean_name()
                                    ))
                                    .size(10.0)
                                    .color(theme.text_secondary),
                                );
                            });
                    });

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(entry.description)
                            .size(11.5)
                            .color(theme.text_secondary),
                    );
                    ui.add_space(4.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("Module: {}", entry.module_hint))
                                .size(10.5)
                                .color(theme.text_muted),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if crate::icons::icon_button(
                                ui,
                                crate::icons::AppIcon::ArrowRight,
                                "Inspect Layout",
                                10.5,
                                theme.text_primary,
                                theme.bg_sidebar,
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::ZERO,
                            )
                            .clicked()
                            {
                                *active_pillar = entry.pillar;
                                jump_clicked = true;
                            }

                            if !entry.status.is_active() {
                                let (ico, label) = if entry.prioritized {
                                    (crate::icons::AppIcon::Target, "Prioritized for Unlock")
                                } else {
                                    (crate::icons::AppIcon::Star, "Queue for Unlock")
                                };
                                let fill = if entry.prioritized {
                                    theme.accent_blue.gamma_multiply(0.25)
                                } else {
                                    theme.bg_sidebar
                                };
                                let fg = if entry.prioritized {
                                    theme.accent_blue
                                } else {
                                    theme.text_secondary
                                };
                                if crate::icons::icon_button(
                                    ui,
                                    ico,
                                    label,
                                    10.5,
                                    fg,
                                    fill,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::ZERO,
                                )
                                .clicked()
                                {
                                    entry.prioritized = !entry.prioritized;
                                }
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }

        if shown_count == 0 {
            theme.card_frame().show(ui, |ui| {
                ui.label(
                    RichText::new("No capabilities match the current filter criteria.")
                        .size(12.0)
                        .color(theme.text_muted),
                );
            });
        }

        jump_clicked
    }

    #[allow(clippy::too_many_arguments)]
    fn render_preferences(
        &mut self,
        ui: &mut Ui,
        theme: &mut MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        ai_copilot_visible: &mut bool,
        bim_project: &BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
    ) {
        crate::icons::icon_label(
            ui,
            crate::icons::AppIcon::SettingsGear,
            20.0,
            theme.accent_blue,
            "Workspace Settings",
            20.0,
            theme.text_primary,
            true,
        );
        ui.label(
            RichText::new(
                "Unified theme, universal 15-module canvas defaults, single-file sync, engineering standards, solver safeguards, and AI copilot configuration.",
            )
            .size(12.5)
            .color(theme.text_secondary),
        );
        ui.add_space(12.0);

        let mut pending_theme_mode: Option<ThemeMode> = None;
        ui_kit::responsive_two_columns(
            ui,
            640.0,
            |ui| {
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("APPEARANCE & AI COPILOT")
                            .strong()
                            .size(11.5)
                            .color(theme.accent_blue),
                    );
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        RichText::new("Workspace Theme Mode:")
                            .size(12.5)
                            .color(theme.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        for mode in [ThemeMode::ProDark, ThemeMode::EngineeringLight] {
                            let is_active = theme.mode == mode;
                            if crate::icons::icon_button(
                                ui,
                                mode.app_icon(),
                                mode.label(),
                                11.5,
                                if is_active {
                                    Color32::WHITE
                                } else {
                                    theme.text_primary
                                },
                                if is_active {
                                    theme.accent_blue
                                } else {
                                    theme.bg_sidebar
                                },
                                Stroke::NONE,
                                egui::CornerRadius::same(7),
                                Vec2::ZERO,
                            )
                            .clicked()
                            {
                                pending_theme_mode = Some(mode);
                            }
                        }
                    });

                    ui.add_space(12.0);
                    ui.checkbox(
                        ai_copilot_visible,
                        RichText::new("Show Floating AI Copilot Pill (Cmd+K)")
                            .size(12.0)
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new(
                            "Hide or reveal the bottom-right AI Copilot floating pill across all 15 modules.",
                        )
                        .size(11.0)
                        .color(theme.text_muted),
                    );
                });
            },
            |ui| {
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("UNIFIED 15-MODULE CANVAS DEFAULTS")
                            .strong()
                            .size(11.5)
                            .color(theme.accent_blue),
                    );
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Canvas Viewport Height Ratio:")
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut universal_canvas.canvas_height_ratio,
                                0.45..=1.15,
                            )
                            .fixed_decimals(2),
                        );
                    });
                    ui.add_space(6.0);

                    if ui
                        .checkbox(
                            &mut self.freeze_by_default,
                            RichText::new("Freeze Canvas by Default (Wheel Scrolls Page Only)")
                                .size(12.0)
                                .color(theme.text_primary),
                        )
                        .changed()
                    {
                        universal_canvas.is_frozen = self.freeze_by_default;
                    }
                    ui.checkbox(
                        &mut universal_canvas.sync_all_layers,
                        RichText::new("Synchronize Drawings Across All 15 Modules")
                            .size(12.0)
                            .color(theme.text_primary),
                    );
                    ui.checkbox(
                        &mut universal_canvas.show_bim_base_layer,
                        RichText::new("Show BIM Base Plan Underlay")
                            .size(12.0)
                            .color(theme.text_primary),
                    );
                    ui.checkbox(
                        &mut universal_canvas.show_domain_overlay,
                        RichText::new("Show Module Telemetry HUD Overlay")
                            .size(12.0)
                            .color(theme.text_primary),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(
                            &mut universal_canvas.show_grid,
                            RichText::new("Grid").size(12.0).color(theme.text_primary),
                        );
                        ui.checkbox(
                            &mut universal_canvas.snap_to_grid,
                            RichText::new("Snap").size(12.0).color(theme.text_primary),
                        );
                        ui.checkbox(
                            &mut universal_canvas.inspector_open,
                            RichText::new("Floating Inspector")
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                    });

                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Grid Spacing (mm):")
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        egui::ComboBox::from_id_salt("settings_canvas_grid_spacing")
                            .selected_text(format!("{:.0} mm", universal_canvas.grid_spacing_mm))
                            .show_ui(ui, |ui| {
                                for step in [250.0_f64, 500.0, 1000.0, 1500.0] {
                                    ui.selectable_value(
                                        &mut universal_canvas.grid_spacing_mm,
                                        step,
                                        format!("{step:.0} mm"),
                                    );
                                }
                            });
                    });
                });
            },
        );
        if let Some(mode) = pending_theme_mode {
            *theme = MacTheme::for_mode(mode);
            theme.apply_to_ctx(ui.ctx());
        }

        ui.add_space(12.0);

        self.render_vault_settings_card(ui, theme);

        ui.add_space(12.0);

        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("1-FILE SYNCHRONIZED WORKSPACE ARCHIVE (.eigen.json)")
                    .strong()
                    .size(11.5)
                    .color(theme.accent_blue),
            );
            ui.separator();
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "Synchronized state: {} canvas objects • {} structural members • {} parametric variables • {} BIM elements",
                    universal_canvas.entities.len(),
                    members.len(),
                    variables.len(),
                    bim_project.walls.len() + bim_project.columns.len() + bim_project.runs.len(),
                ))
                .size(11.5)
                .color(theme.text_secondary),
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::SaveDisk,
                    "Export 1-File Unified Workspace (.eigen.json)",
                    11.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    egui::CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let snap = self.workspace_settings_snapshot(
                        theme,
                        universal_canvas,
                        *ai_copilot_visible,
                    );
                    universal_canvas.export_unified_file(bim_project, members, variables, snap);
                }

                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::UploadImport,
                    "Import 1-File Archive (.eigen.json)",
                    11.5,
                    theme.text_primary,
                    theme.bg_sidebar,
                    Stroke::NONE,
                    egui::CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    universal_canvas.import_json_open = true;
                }
            });
            if let Some(status) = &universal_canvas.status_message {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(status)
                        .size(11.0)
                        .color(theme.accent_green),
                );
            }
        });

        ui.add_space(12.0);

        ui_kit::responsive_two_columns(
            ui,
            640.0,
            |ui| {
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("ENGINEERING UNITS & DESIGN STANDARDS")
                            .strong()
                            .size(11.5)
                            .color(theme.accent_blue),
                    );
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(RichText::new("Active Unit System:").size(12.5).color(theme.text_primary));
                    egui::ComboBox::from_id_salt("settings_unit_system")
                        .selected_text(&self.unit_system)
                        .width(ui.available_width().min(280.0))
                        .show_ui(ui, |ui| {
                            for opt in [
                                "kN, mm, MPa (Structural Metric)",
                                "kN, m, kPa (Civil / Geotech Metric)",
                                "kips, in, ksi (US Imperial)",
                            ] {
                                ui.selectable_value(&mut self.unit_system, opt.to_string(), opt);
                            }
                        });

                    ui.add_space(10.0);
                    ui.label(RichText::new("Design Standard:").size(12.5).color(theme.text_primary));
                    egui::ComboBox::from_id_salt("settings_design_standard")
                        .selected_text(&self.design_standard)
                        .width(ui.available_width().min(280.0))
                        .show_ui(ui, |ui| {
                            for opt in [
                                "SNI 1729 / AISC 360-16",
                                "SNI 2847:2019 / ACI 318-19",
                                "Eurocode 2 / Eurocode 3 / EN 1998",
                            ] {
                                ui.selectable_value(&mut self.design_standard, opt.to_string(), opt);
                            }
                        });
                });
            },
            |ui| {
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("SOLVER & NUMERICAL SAFEGUARDS")
                            .strong()
                            .size(11.5)
                            .color(theme.accent_blue),
                    );
                    ui.separator();
                    ui.add_space(8.0);

                    ui.checkbox(
                        &mut self.enable_svd_guard,
                        RichText::new("Enable SVD Kinematic Guard (Sherman-Morrison margin check)")
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new("Instantly detects mechanism formation upon N-1 member damage without pseudoinverse.")
                            .size(11.0)
                            .color(theme.text_muted),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(
                        &mut self.enable_anti_leakage,
                        RichText::new("Enforce Strict Pre-Analysis Protocol (Leakage-Free)")
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new("Guarantees screening upper bounds are computed 100% from intact structure facts.")
                            .size(11.0)
                            .color(theme.text_muted),
                    );

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Parallel Solver Threads:").color(theme.text_primary));
                        ui.add(egui::Slider::new(&mut self.solver_threads, 1..=32));
                    });
                });
            },
        );

        ui.add_space(12.0);
        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("AI COPILOT INTEGRATION")
                    .strong()
                    .size(11.5)
                    .color(theme.accent_blue),
            );
            ui.separator();
            ui.add_space(8.0);

            ui.label(RichText::new("Google Gemini API Key:").size(12.5).color(theme.text_primary));
            ui.add(
                egui::TextEdit::singleline(&mut self.gemini_api_key)
                    .password(true)
                    .desired_width(ui.available_width().min(380.0)),
            );
            ui.label(
                RichText::new(
                    "Stored in-session for upcoming live LLM BIM tool orchestration (Feature AI-03).",
                )
                .size(11.0)
                .color(theme.text_muted),
            );
        });
    }

    fn render_vault_settings_card(&mut self, ui: &mut Ui, theme: &MacTheme) {
        use crate::core::vault::VaultBackendKind;
        use crate::icons::{self, AppIcon};

        let status = crate::core::InternalVault::connection_status();
        let categories_count = if status.connected {
            crate::core::InternalVault::list_all_vault_categories().len()
        } else {
            0
        };
        let projects_count = if status.connected {
            crate::core::InternalVault::list_personal_vault_projects().len()
        } else {
            0
        };

        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Database,
                    14.0,
                    theme.accent_blue,
                    "PERSONAL ENGINEERING VAULT CONNECTION",
                    12.0,
                    theme.accent_blue,
                    true,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (badge_txt, badge_col) = if status.connected && status.needs_permission {
                        ("PERMISSION NEEDED", theme.accent_orange)
                    } else if status.connected {
                        ("CONNECTED", theme.accent_green)
                    } else if status.guest_mode {
                        ("GUEST SESSION", theme.accent_orange)
                    } else {
                        ("DISCONNECTED", theme.text_muted)
                    };
                    Frame::new()
                        .fill(badge_col.gamma_multiply(0.18))
                        .stroke(Stroke::new(1.0, badge_col.gamma_multiply(0.6)))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(RichText::new(badge_txt).size(10.0).strong().color(badge_col));
                        });
                });
            });
            ui.separator();
            ui.add_space(6.0);

            let backend_label = match status.backend {
                VaultBackendKind::LocalFilesystem => "Local Disk Directory",
                VaultBackendKind::WebDirectoryPicker => "Browser File System Access API (OS Folder)",
                VaultBackendKind::BrowserIndexedDb => "Browser IndexedDB Persistent Store",
                VaultBackendKind::Unbound => "No Vault Connected",
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Backend:").size(11.5).strong().color(theme.text_secondary));
                ui.label(RichText::new(backend_label).size(11.5).color(theme.text_primary));
                if status.connected {
                    ui.label(
                        RichText::new(format!(
                            "• {} Projects • {} Folders",
                            projects_count, categories_count
                        ))
                        .size(11.5)
                        .color(theme.accent_blue),
                    );
                }
            });
            ui.horizontal_wrapped(|ui| {
                icons::draw_icon(ui, AppIcon::Folder, 12.5, theme.accent_blue);
                ui.label(
                    RichText::new(&status.location_label)
                        .size(11.0)
                        .monospace()
                        .color(theme.text_secondary),
                );
            });

            if !status.status_msg.is_empty() {
                ui.label(RichText::new(&status.status_msg).size(10.5).color(theme.text_muted));
            }
            if !status.error_msg.is_empty() {
                ui.label(RichText::new(&status.error_msg).size(10.5).color(theme.accent_red));
            }
            if let Some(ref fb) = self.export_feedback {
                ui.label(RichText::new(fb).size(11.0).color(theme.accent_green));
            }

            ui.add_space(8.0);

            #[cfg(target_arch = "wasm32")]
            {
                let snap = crate::core::web_bridge::vault_state_snapshot();
                ui.horizontal_wrapped(|ui| {
                    if status.needs_permission {
                        if icons::icon_button(
                            ui,
                            AppIcon::Undo,
                            "Reconnect Local Folder Permission",
                            11.5,
                            Color32::WHITE,
                            theme.accent_orange,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            crate::core::web_bridge::vault_reconnect();
                            self.vault_state_changed = true;
                        }
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Folder,
                        if snap.fs_supported {
                            "Connect Existing OS Folder"
                        } else {
                            "Connect Browser IndexedDB Vault"
                        },
                        11.5,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        crate::core::web_bridge::vault_connect_existing();
                        self.vault_state_changed = true;
                    }

                    if snap.fs_supported
                        && icons::icon_button(
                            ui,
                            AppIcon::Database,
                            "Connect Browser IndexedDB",
                            11.5,
                            theme.text_primary,
                            theme.bg_sidebar,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(7),
                            Vec2::ZERO,
                        )
                        .clicked()
                    {
                        crate::core::web_bridge::vault_connect_indexeddb("Browser IndexedDB Vault");
                        self.vault_state_changed = true;
                    }
                });

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("New Vault Name:").size(11.5).color(theme.text_secondary));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.vault_new_name_input)
                            .hint_text("EigenGraph_Vault")
                            .desired_width(180.0),
                    );
                    if icons::icon_button(
                        ui,
                        AppIcon::Plus,
                        "Create New Vault (Pick Folder)",
                        11.5,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let name = self.vault_new_name_input.trim();
                        crate::core::web_bridge::vault_create_new(
                            if name.is_empty() { "EigenGraph_Vault" } else { name },
                            false,
                        );
                        self.vault_state_changed = true;
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Plus,
                        "Create in Browser IndexedDB",
                        11.5,
                        theme.text_primary,
                        theme.bg_sidebar,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let name = self.vault_new_name_input.trim();
                        crate::core::web_bridge::vault_create_new(
                            if name.is_empty() { "EigenGraph_Vault" } else { name },
                            true,
                        );
                        self.vault_state_changed = true;
                    }
                });
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Vault Folder Path:").size(11.5).color(theme.text_secondary));
                    let path_w = (ui.available_width() - 220.0).clamp(180.0, 400.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.vault_path_input)
                            .hint_text("/path/to/EigenGraph_Vault")
                            .desired_width(path_w),
                    );
                    if icons::icon_button(
                        ui,
                        AppIcon::Folder,
                        "Browse...",
                        11.5,
                        theme.text_primary,
                        theme.bg_sidebar,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        if let Some(chosen) =
                            crate::core::web_bridge::pick_folder_native("Select EigenGraph Vault Folder")
                        {
                            self.vault_path_input = chosen;
                        }
                    }
                });

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::Folder,
                        "Connect Existing Vault",
                        11.5,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let p = std::path::PathBuf::from(self.vault_path_input.trim());
                        match crate::core::InternalVault::connect_existing_vault(p) {
                            Ok(connected_path) => {
                                self.export_feedback = Some(format!(
                                    "Connected existing Vault at {}",
                                    connected_path.display()
                                ));
                                self.vault_state_changed = true;
                            }
                            Err(err) => {
                                self.export_feedback = Some(err);
                            }
                        }
                    }

                    ui.add_space(6.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.vault_new_name_input)
                            .hint_text("Vault Name")
                            .desired_width(140.0),
                    );
                    if icons::icon_button(
                        ui,
                        AppIcon::Plus,
                        "Create New Vault",
                        11.5,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let base = std::path::PathBuf::from(self.vault_path_input.trim());
                        let name = self.vault_new_name_input.trim();
                        let target = if name.is_empty()
                            || base.file_name().and_then(|s| s.to_str()) == Some(name)
                        {
                            base
                        } else {
                            base.join(name)
                        };
                        match crate::core::InternalVault::create_new_vault(
                            target,
                            Some(self.vault_new_name_input.clone()),
                            false,
                        ) {
                            Ok(created) => {
                                self.vault_path_input = created.to_string_lossy().to_string();
                                self.export_feedback = Some(format!(
                                    "Created new Vault at {}",
                                    created.display()
                                ));
                                self.vault_state_changed = true;
                            }
                            Err(err) => {
                                self.export_feedback = Some(err);
                            }
                        }
                    }
                });
            }

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if status.connected {
                    if icons::icon_button(
                        ui,
                        AppIcon::Undo,
                        "Refresh Vault Index",
                        11.0,
                        theme.text_primary,
                        theme.bg_sidebar,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        crate::core::web_bridge::vault_refresh();
                        self.vault_state_changed = true;
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Close,
                        "Disconnect Vault",
                        11.0,
                        theme.accent_red,
                        theme.bg_sidebar,
                        Stroke::new(1.0, theme.accent_red.gamma_multiply(0.5)),
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        let _ = crate::core::InternalVault::disconnect_vault();
                        self.export_feedback = Some("Personal Vault disconnected.".to_string());
                        self.vault_state_changed = true;
                    }
                }
                if icons::icon_button(
                    ui,
                    AppIcon::Sparkles,
                    "Open Startup Vault Setup Pop-up",
                    11.0,
                    theme.accent_blue,
                    theme.bg_sidebar,
                    Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.45)),
                    CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    self.request_open_vault_popup = true;
                }
            });
        });
    }

    fn render_about(&self, ui: &mut Ui, theme: &MacTheme) {
        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("ABOUT EIGENGRAPH")
                    .strong()
                    .size(12.0)
                    .color(theme.accent_blue),
            );
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Version:").strong().size(13.0).color(theme.text_primary));
                ui.label(
                    RichText::new(format!(
                        "v{} — Phase 47 Antigravity AI Assistant Side Panel (Vault Folder + @ Add Context) & Earth Zoom Calibration (74/74 Capabilities Active)",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .size(13.0)
                    .color(theme.accent_green)
                    .strong(),
                );
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Runtime:").strong().size(12.0).color(theme.text_primary));
                let runtime = if cfg!(target_arch = "wasm32") {
                    "Browser (WebGL2) • WebAssembly (wasm32)"
                } else {
                    "EigenGraph Native & Web Engine"
                };
                ui.label(RichText::new(runtime).size(12.0).color(theme.text_secondary));
            });

            ui.add_space(8.0);
            ui.label(
                RichText::new("Theoretical & Computational Foundations:")
                    .strong()
                    .size(12.0)
                    .color(theme.text_primary),
            );
            ui.label(
                RichText::new(
                    "• Linear Adjusted Redundancy Index (ARI) with Sherman-Morrison O(N^2) fast update\n\
                     • Woodbury Rank-2 (N-2) simultaneous multi-element disruption screening\n\
                     • Euler elastic buckling & AISC 360-16 B1 moment amplification geometric stiffness guard\n\
                     • SVD Kinematic Mechanism Guard (m_e = 1 - h_e) for instant singularity detection\n\
                     • Planar Half-Edge Face Arrangement & Non-Recursive BSP Tree Solid CSG Subtraction\n\
                     • 2D Sheet Composer (A0–A3) with Vector PDF 1.4, DXF & SVG Blueprint Generation\n\
                     • Responsive 15-Pillar Master Workspace with Centralized Feature Unlock Registry",
                )
                .size(11.0)
                .color(theme.text_muted),
            );

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Design Standards:").strong().size(11.5).color(theme.text_primary));
                ui.label(
                    RichText::new("AISC 360-16 • ACI 318-19 • SNI 1729:2020 • SNI 2847:2019 • SNI 1726:2019")
                        .size(11.0)
                        .color(theme.text_secondary),
                );
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Integrity & License:").strong().size(11.5).color(theme.text_primary));
                ui.label(
                    RichText::new("NIST FIPS 180-4 SHA-256 Validated • Dual MIT / Apache 2.0")
                        .size(11.0)
                        .color(theme.text_secondary),
                );
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Repository:").strong().size(11.5).color(theme.text_primary));
                ui.hyperlink_to(
                    "github.com/haridsirawan/EigenGraph",
                    "https://github.com/haridsirawan/EigenGraph",
                );
            });
        });
    }
}
