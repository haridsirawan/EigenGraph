//! app.rs — Core application struct implementing eframe::App.

use eframe::App;
use egui::{Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Panel, CentralPanel, RichText, Stroke, Vec2};

use crate::models::{DatabaseComponent, NavigationPillar, ProjectItem, StructuralMember};
use crate::theme::MacTheme;
use crate::views::{
    AiAssistantView, AnalyzeView, BuildView, FacilityView, IndustrializedView, LeanView,
    MyProjectView, ParametricView, PlanningView, PredictionView, ResearchConceptView, ResultsView,
    SettingsView, SustainabilityView,
};

pub struct EigenGraphApp {
    pub theme: MacTheme,
    pub active_pillar: NavigationPillar,
    pub search_query: String,
    pub show_settings: bool,

    // Library & 10D BIM Lifecycle Views + Prediction & Results
    pub my_project_view: MyProjectView,         // 📁 My Library (Project Catalog & Components)
    pub research_view: ResearchConceptView,     // 1D: Research & Concept
    pub parametric_view: ParametricView,         // 2D: Drafting & Drawings
    pub build_view: BuildView,                   // 3D: Modeling & Geometry
    pub planning_view: PlanningView,             // 4D (Time & Scheduling) & 5D (Cost & Budgeting)
    pub sustainability_view: SustainabilityView, // 6D: Sustainability & Energy
    pub facility_view: FacilityView,             // 7D: Facility Management
    pub analyze_view: AnalyzeView,               // 8D: Safety/Health & Safety
    pub lean_view: LeanView,                     // 9D: Lean Construction
    pub industrialized_view: IndustrializedView, // 10D: Industrialized Construction & Automation
    pub prediction_view: PredictionView,         // 🔮 Prediction & AI Simulation
    pub results_view: ResultsView,               // 📊 Results & Executive Audit
    pub settings_view: SettingsView,
    pub ai_assistant_view: AiAssistantView,
    pub updater: crate::updater::AppUpdater,

    // Domain Data & Parametric Component Database
    pub projects: Vec<ProjectItem>,
    pub db_components: Vec<DatabaseComponent>,
    pub members: Vec<StructuralMember>,
}

impl Default for EigenGraphApp {
    fn default() -> Self {
        Self {
            theme: MacTheme::default(),
            active_pillar: NavigationPillar::MyLibrary,
            search_query: String::new(),
            show_settings: false,

            my_project_view: MyProjectView::default(),
            research_view: ResearchConceptView::default(),
            parametric_view: ParametricView::default(),
            build_view: BuildView::default(),
            planning_view: PlanningView::default(),
            sustainability_view: SustainabilityView::default(),
            facility_view: FacilityView::default(),
            analyze_view: AnalyzeView::default(),
            lean_view: LeanView::default(),
            industrialized_view: IndustrializedView::default(),
            prediction_view: PredictionView::default(),
            results_view: ResultsView::default(),
            settings_view: SettingsView::default(),
            ai_assistant_view: AiAssistantView::default(),
            updater: crate::updater::AppUpdater::default(),

            projects: crate::core::InternalVault::default_nvw_dcr_lod_projects(),
            db_components: crate::core::InternalVault::load_or_init_parametric_components(),
            members: crate::core::InternalVault::default_nvw_dcr_lod_members(),
        }
    }
}

impl App for EigenGraphApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // macOS Command+, shortcut for System Preferences
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Comma)) {
            self.show_settings = !self.show_settings;
        }

        // 1. TOP TITLEBAR (Native macOS Style with Window Controls, Status & Copilot Trigger)
        Panel::top("top_titlebar").frame(Frame::new().fill(self.theme.bg_canvas)).show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                // macOS Traffic Lights Simulation / Spacing
                ui.add_space(12.0);
                ui.label(RichText::new("🔴  🟡  🟢").size(10.0));
                ui.add_space(14.0);

                // App Brand & Active Document Badge
                ui.label(RichText::new("⚛️ EigenGraph").strong().size(13.5).color(self.theme.text_primary));
                ui.label(RichText::new("—").color(self.theme.text_muted));
                ui.label(RichText::new(format!("{} (10D BIM)", self.active_pillar.clean_name())).size(12.5).color(self.theme.text_secondary));

                // Right side: Status and Shortcuts
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(16.0);

                    // Quick AI Copilot Trigger Pill (Command+K)
                    let copilot_btn = egui::Button::new(
                        RichText::new("✨ AI Copilot (⌘K)")
                            .size(11.5)
                            .strong()
                            .color(if self.ai_assistant_view.is_visible { Color32::WHITE } else { self.theme.accent_blue })
                    )
                    .fill(if self.ai_assistant_view.is_visible { self.theme.accent_blue } else { self.theme.bg_card })
                    .corner_radius(CornerRadius::same(6))
                    .stroke(Stroke::new(1.0, self.theme.accent_blue.gamma_multiply(0.4)));

                    if ui.add(copilot_btn).clicked() {
                        self.ai_assistant_view.is_visible = !self.ai_assistant_view.is_visible;
                    }

                    ui.add_space(8.0);
                    // System Status Indicator
                    ui.label(RichText::new("⚡ SVD Guard: Active").size(11.0).color(self.theme.accent_green));
                    ui.label(RichText::new("•").color(self.theme.text_muted));
                    ui.label(RichText::new("ISO 14040 LCA").size(11.0).color(self.theme.text_muted));
                });
            });
            ui.add_space(6.0);
        });

        // 2. UNIFIED LEFT SIDEBAR (Locked Minimalist Layout)
        Panel::left("unified_sidebar")
            .resizable(false)
            .exact_size(245.0)
            .frame(Frame::new()
                .fill(self.theme.bg_sidebar)
                .stroke(Stroke::new(1.0, self.theme.border_subtle))
                .inner_margin(Margin::symmetric(10, 14)))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Header Branding (Locked: Title only, no subtitles, no section headers)
                    ui.label(RichText::new("⚛️ EigenGraph").strong().size(14.5).color(self.theme.text_primary));
                    ui.add_space(10.0);

                    let pillars = [
                        NavigationPillar::MyLibrary,
                        NavigationPillar::ResearchConcept,
                        NavigationPillar::DraftingDrawings,
                        NavigationPillar::ModelingGeometry,
                        NavigationPillar::TimeScheduling,
                        NavigationPillar::CostBudgeting,
                        NavigationPillar::SustainabilityEnergy,
                        NavigationPillar::FacilityManagement,
                        NavigationPillar::SafetyHealth,
                        NavigationPillar::LeanConstruction,
                        NavigationPillar::IndustrializedAutomation,
                        NavigationPillar::Prediction,
                        NavigationPillar::Results,
                    ];

                    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                        for pillar in pillars {
                            let is_active = self.active_pillar == pillar;
                            let bg = if is_active { self.theme.accent_blue } else { Color32::TRANSPARENT };
                            let fg = if is_active { Color32::WHITE } else { self.theme.text_primary };

                            let row = Frame::new()
                                .fill(bg)
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(10, 7))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("{}   {}", pillar.icon(), pillar.clean_name())).size(12.5).color(fg).strong());
                                    });
                                });

                            if row.response.interact(egui::Sense::click()).clicked() {
                                self.active_pillar = pillar;
                            }
                            ui.add_space(2.0);
                        }
                    });

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Version & Tech stack with Update checker button
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let arch_label = if cfg!(target_arch = "x86_64") {
                                "Intel x86_64"
                            } else if cfg!(target_arch = "aarch64") {
                                "Apple Silicon"
                            } else {
                                "Universal 2"
                            };
                            ui.label(RichText::new(format!("EigenGraph v1.1.0 ({})", arch_label)).size(10.0).color(self.theme.text_muted));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(egui::Button::new(RichText::new("🚀 Update").size(9.5).color(self.theme.accent_blue))
                                    .fill(self.theme.bg_card)
                                    .corner_radius(CornerRadius::same(4))).clicked() {
                                    self.updater.github_token = self.settings_view.github_token.clone();
                                    self.updater.check_for_updates();
                                }
                            });
                        });
                        ui.label(RichText::new("SNI 1729 / AISC 360-16 • EigenGraph Platform").size(9.5).color(self.theme.text_muted));
                    });
                });
            });

        // 3. SECONDARY TOOLBAR & BREADCRUMB
        Panel::top("sub_toolbar").frame(Frame::new().fill(self.theme.bg_card)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new(self.active_pillar.icon()).size(14.0));
                ui.add_space(4.0);
                ui.label(RichText::new(self.active_pillar.title()).strong().size(12.0).color(self.theme.accent_blue));
                ui.label(RichText::new("—").color(self.theme.text_muted));
                ui.label(RichText::new(self.active_pillar.subtitle()).size(11.0).color(self.theme.text_secondary));

                // Global Search Box, Update Button, Settings Button & Copilot Toggle in Toolbar
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(12.0);

                    if ui.selectable_label(self.ai_assistant_view.is_visible, "✨ Copilot").clicked() {
                        self.ai_assistant_view.is_visible = !self.ai_assistant_view.is_visible;
                    }

                    ui.add_space(4.0);

                    if ui.add(egui::Button::new(RichText::new("🚀 Update").size(11.5).color(self.theme.text_secondary))
                        .fill(self.theme.bg_card)
                        .corner_radius(CornerRadius::same(6))).on_hover_text("Check for software updates").clicked() {
                        self.updater.github_token = self.settings_view.github_token.clone();
                        self.updater.check_for_updates();
                    }

                    ui.add_space(4.0);

                    if ui.add(egui::Button::new(RichText::new("⚙️").size(13.0).color(self.theme.text_secondary))
                        .fill(self.theme.bg_card)
                        .corner_radius(CornerRadius::same(6))).on_hover_text("System Preferences (⌘,)").clicked() {
                        self.show_settings = !self.show_settings;
                    }

                    ui.add_space(8.0);

                    ui.add(egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍 Filter workspace...")
                        .desired_width(150.0));
                });
            });
        });

        // 4. CENTRAL PANEL (Views Router for Library & 10D BIM Dimensions)
        CentralPanel::default().frame(Frame::new().fill(self.theme.bg_canvas)).show(ui, |ui| {
            match self.active_pillar {
                NavigationPillar::MyLibrary => {
                    self.my_project_view.show(ui, &self.theme, &self.projects, &mut self.active_pillar);
                }
                NavigationPillar::ResearchConcept => {
                    self.research_view.show(ui, &self.theme, &mut self.active_pillar);
                }
                NavigationPillar::DraftingDrawings => {
                    self.parametric_view.show(ui, &self.theme, &mut self.db_components, &mut self.active_pillar);
                }
                NavigationPillar::ModelingGeometry => {
                    self.build_view.show(ui, &self.theme, &self.members, &self.db_components);
                }
                NavigationPillar::TimeScheduling => {
                    if self.planning_view.active_subtab != crate::views::planning::PlanningSubTab::Schedule4d
                        && self.planning_view.active_subtab != crate::views::planning::PlanningSubTab::StagedErectionSvd
                    {
                        self.planning_view.active_subtab = crate::views::planning::PlanningSubTab::Schedule4d;
                    }
                    self.planning_view.show(ui, &self.theme, &self.parametric_view.variables, &self.members);
                }
                NavigationPillar::CostBudgeting => {
                    if self.planning_view.active_subtab != crate::views::planning::PlanningSubTab::DynamicRab
                        && self.planning_view.active_subtab != crate::views::planning::PlanningSubTab::TakeoffQto
                        && self.planning_view.active_subtab != crate::views::planning::PlanningSubTab::CostAriTradeoff
                    {
                        self.planning_view.active_subtab = crate::views::planning::PlanningSubTab::DynamicRab;
                    }
                    self.planning_view.show(ui, &self.theme, &self.parametric_view.variables, &self.members);
                }
                NavigationPillar::SustainabilityEnergy => {
                    self.sustainability_view.show(ui, &self.theme, &self.members);
                }
                NavigationPillar::FacilityManagement => {
                    self.facility_view.show(ui, &self.theme, &self.members);
                }
                NavigationPillar::SafetyHealth => {
                    self.analyze_view.show(ui, &self.theme, &self.members);
                }
                NavigationPillar::LeanConstruction => {
                    self.lean_view.show(ui, &self.theme, &self.members);
                }
                NavigationPillar::IndustrializedAutomation => {
                    self.industrialized_view.show(ui, &self.theme, &self.members);
                }
                NavigationPillar::Prediction => {
                    self.prediction_view.show(ui, &self.theme, &self.members);
                }
                NavigationPillar::Results => {
                    self.results_view.show(ui, &self.theme, &self.members);
                }
            }
        });

        // 5. GLOBAL FLOATING DYNAMIC ISLAND AI COPILOT (Available Across All 5 Features)
        let ctx = ui.ctx().clone();
        self.ai_assistant_view.show(
            &ctx,
            &self.theme,
            self.active_pillar,
            &self.members,
            &self.projects,
            &mut self.db_components,
        );

        // 6. MACOS SOFTWARE UPDATE MODAL (If Triggered)
        self.updater.render_modal(ui.ctx(), &self.theme);

        // 7. MACOS SYSTEM PREFERENCES MODAL (Cmd+,)
        if self.show_settings {
            let mut open = self.show_settings;
            egui::Window::new("⚙️ System Preferences")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
                .fixed_size(Vec2::new(480.0, 440.0))
                .frame(Frame::new()
                    .fill(self.theme.bg_card)
                    .stroke(Stroke::new(1.0, self.theme.border_subtle))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(20)))
                .show(ui.ctx(), |ui| {
                    self.settings_view.show(ui, &self.theme);
                });
            self.show_settings = open;
        }
    }
}
