//! settings.rs — Workspace Settings, Design Standards & Vault Connection Preferences.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::features::FeatureRegistry;
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
    About,
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
    pub remember_gemini_key: bool,
    pub export_feedback: Option<String>,
    pub vault_path_input: String,
    pub vault_new_name_input: String,
    pub request_open_vault_popup: bool,
    pub vault_state_changed: bool,
}

impl Default for SettingsView {
    fn default() -> Self {
        let (remember_gemini_key, gemini_api_key) =
            crate::core::web_bridge::gemini_key_load_device_opt_in();
        Self {
            active_subtab: SettingsSubTab::WorkspaceSettings,
            freeze_by_default: true,
            unit_system: "kN, mm, MPa (Structural Metric)".to_string(),
            design_standard: "SNI 1729 / AISC 360-16".to_string(),
            solver_threads: 8,
            enable_svd_guard: true,
            enable_anti_leakage: true,
            gemini_api_key,
            remember_gemini_key,
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

    /// Renders Consolidated Workspace Settings & About.
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
        _registry: &mut FeatureRegistry,
        _active_pillar: &mut NavigationPillar,
    ) -> bool {
        ui_kit::subtab_bar(
            ui,
            theme,
            "settings_subtabs",
            &mut self.active_subtab,
            &[
                (SettingsSubTab::WorkspaceSettings, "⚙️ Workspace Settings"),
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
                SettingsSubTab::About => {
                    self.render_about(ui, theme);
                }
            });

        false
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
            let key_resp = ui.add(
                egui::TextEdit::singleline(&mut self.gemini_api_key)
                    .password(true)
                    .desired_width(ui.available_width().min(380.0)),
            );
            let rem_resp = ui.checkbox(
                &mut self.remember_gemini_key,
                "Remember API key on this device (local storage only; never exported into .eigen archives)",
            );
            if rem_resp.changed() || (self.remember_gemini_key && key_resp.changed()) {
                crate::core::web_bridge::gemini_key_save_device_opt_in(
                    self.remember_gemini_key,
                    &self.gemini_api_key,
                );
            }
            ui.label(
                RichText::new(
                    "Powers live Gemini 2.5 Flash & Live Stream orchestration inside the Dynamic Island.",
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

            let outcome = crate::ui_kit::render_obsidian_vault_panel(
                ui,
                theme,
                &mut self.vault_path_input,
                &mut self.vault_new_name_input,
                false,
                false,
                true,
            );
            if outcome.state_changed {
                self.vault_state_changed = true;
            }
            if let Some(fb) = outcome.feedback {
                self.export_feedback = Some(fb);
            }
            if outcome.open_startup_modal {
                self.request_open_vault_popup = true;
            }
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
                let phase_num: u32 = {
                    let mut parts = env!("CARGO_PKG_VERSION").split('.');
                    let major: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    let minor: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    let patch: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    major * 100 + minor * 10 + patch
                };
                ui.label(
                    RichText::new(format!(
                        "v{} — Phase {} 100% Pure-Rust Vector & IFCX (IFC 5 JSON ECS) Living Earth Globe (WebGL2 24-Bit Z-Buffer, f64 RTC, Multi-Layer Composition, Zero Legacy STEP, 0% Raster), 6-DOF FEM & ARI",
                        env!("CARGO_PKG_VERSION"),
                        phase_num
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
