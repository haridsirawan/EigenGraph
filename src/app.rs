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

        // 1. UNIFIED FULL-HEIGHT LEFT SIDEBAR (Native macOS App Store Style)
        Panel::left("unified_sidebar")
            .resizable(false)
            .exact_size(240.0)
            .frame(Frame::new()
                .fill(Color32::from_rgb(34, 34, 36))
                .stroke(Stroke::new(1.0, Color32::from_rgb(48, 48, 52)))
                .inner_margin(Margin::symmetric(12, 10)))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Window Controls Header (macOS Traffic Lights or Multi-platform Spacing)
                    #[cfg(target_os = "macos")]
                    {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(2.0);
                            ui.label(RichText::new("🔴  🟡  🟢").size(9.5));
                        });
                        ui.add_space(14.0);
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        ui.add_space(10.0);
                    }

                    // Native Search Bar (App Store Style)
                    Frame::new()
                        .fill(Color32::from_rgb(44, 44, 48))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(8, 5))
                        .stroke(Stroke::new(0.8, Color32::from_rgb(58, 58, 64)))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.search_query)
                                        .hint_text("🔍 Search")
                                        .desired_width(ui.available_width() - 4.0)
                                        .frame(Frame::NONE),
                                );
                            });
                        });
                    ui.add_space(14.0);

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

                    // Fixed height for bottom section (user profile + utilities)
                    let bottom_section_height = 84.0;
                    let nav_area_height = (ui.available_height() - bottom_section_height).max(100.0);

                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), nav_area_height),
                        Layout::top_down(Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                                for pillar in pillars {
                                    // Search filter
                                    if !self.search_query.trim().is_empty() {
                                        let q = self.search_query.trim().to_lowercase();
                                        let name = pillar.clean_name().to_lowercase();
                                        if !name.contains(&q) {
                                            continue;
                                        }
                                    }

                                    let is_active = self.active_pillar == pillar;
                                    let (rect, response) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), 32.0),
                                        egui::Sense::click(),
                                    );

                                    if response.clicked() {
                                        self.active_pillar = pillar;
                                    }

                                    let is_hovered = response.hovered();
                                    let fill_color = if is_active {
                                        Color32::from_rgba_premultiplied(255, 255, 255, 32)
                                    } else if is_hovered {
                                        Color32::from_rgba_premultiplied(255, 255, 255, 14)
                                    } else {
                                        Color32::TRANSPARENT
                                    };

                                    if fill_color != Color32::TRANSPARENT {
                                        ui.painter().rect_filled(rect, CornerRadius::same(6), fill_color);
                                    }

                                    let text_color = if is_active {
                                        Color32::WHITE
                                    } else if is_hovered {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(200, 200, 206)
                                    };

                                    let item_label = format!("{}   {}", pillar.icon(), pillar.clean_name());
                                    ui.painter().text(
                                        rect.left_center() + Vec2::new(10.0, 0.0),
                                        Align2::LEFT_CENTER,
                                        item_label,
                                        egui::FontId::proportional(12.5),
                                        text_color,
                                    );

                                    ui.add_space(2.0);
                                }
                            });
                        },
                    );

                    // Bottom: User Profile & System Preferences (macOS App Store Style)
                    ui.separator();
                    ui.add_space(4.0);

                    let (prof_rect, prof_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 36.0),
                        egui::Sense::click(),
                    );
                    let prof_hovered = prof_resp.hovered();
                    let prof_fill = if self.show_settings {
                        Color32::from_rgba_premultiplied(255, 255, 255, 32)
                    } else if prof_hovered {
                        Color32::from_rgba_premultiplied(255, 255, 255, 14)
                    } else {
                        Color32::TRANSPARENT
                    };
                    if prof_fill != Color32::TRANSPARENT {
                        ui.painter().rect_filled(prof_rect, CornerRadius::same(6), prof_fill);
                    }

                    // Avatar Circle
                    let avatar_pos = prof_rect.left_center() + Vec2::new(14.0, 0.0);
                    ui.painter().circle_filled(avatar_pos, 11.5, Color32::from_rgb(52, 92, 160));
                    ui.painter().text(
                        avatar_pos,
                        Align2::CENTER_CENTER,
                        "HI",
                        egui::FontId::proportional(9.0),
                        Color32::WHITE,
                    );

                    // User Name
                    ui.painter().text(
                        prof_rect.left_center() + Vec2::new(32.0, 0.0),
                        Align2::LEFT_CENTER,
                        "Harids Irawan",
                        egui::FontId::proportional(12.5),
                        Color32::WHITE,
                    );

                    // Gear icon on right
                    ui.painter().text(
                        prof_rect.right_center() - Vec2::new(10.0, 0.0),
                        Align2::RIGHT_CENTER,
                        "⚙️",
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(160, 160, 165),
                    );

                    if prof_resp.on_hover_text("System Preferences (⌘,)").clicked() {
                        self.show_settings = !self.show_settings;
                    }

                    ui.add_space(4.0);

                    // Secondary Tools (AI Copilot & Update Trigger)
                    ui.horizontal(|ui| {
                        let copilot_active = self.ai_assistant_view.is_visible;
                        if ui.add(
                            egui::Button::new(
                                RichText::new("✨ AI Copilot")
                                    .size(10.5)
                                    .color(if copilot_active { Color32::WHITE } else { self.theme.accent_blue }),
                            )
                            .fill(if copilot_active { self.theme.accent_blue } else { Color32::TRANSPARENT })
                            .corner_radius(CornerRadius::same(4)),
                        )
                        .on_hover_text("Buka AI Copilot Assistant (⌘K)")
                        .clicked()
                        {
                            self.ai_assistant_view.is_visible = !self.ai_assistant_view.is_visible;
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(
                                egui::Button::new(RichText::new("🚀 Update").size(10.0).color(self.theme.text_muted))
                                    .fill(Color32::TRANSPARENT),
                            )
                            .on_hover_text("Periksa pembaruan perangkat lunak")
                            .clicked()
                            {
                                self.updater.github_token = self.settings_view.github_token.clone();
                                self.updater.check_for_updates();
                            }
                        });
                    });
                });
            });

        // 2. CENTRAL PANEL (Clean Minimalist Workspace without Strip Bar)
        CentralPanel::default().frame(Frame::new().fill(self.theme.bg_canvas)).show(ui, |ui| {
            // Elegant App Store Style View Header
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    RichText::new(self.active_pillar.clean_name())
                        .size(26.0)
                        .strong()
                        .color(self.theme.text_primary),
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.separator();
            });
            ui.add_space(12.0);

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
                .resizable(true)
                .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
                .default_size(Vec2::new(540.0, 580.0))
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
