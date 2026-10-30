//! app.rs — Core application struct implementing eframe::App.

use eframe::App;
use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, Panel, CentralPanel, RichText, Stroke, Vec2};

use crate::models::{DatabaseComponent, NavigationPillar, ParametricVariable, ProjectItem, ProjectStatus, StructuralMember};
use crate::theme::MacTheme;
use crate::views::{
    AiAssistantView, AnalyzeView, BuildView, FacilityView, IndustrializedView, LeanView,
    MyProjectView, ParametricView, PlanningView, ResearchConceptView, SettingsView, SustainabilityView,
};

pub struct EigenGraphApp {
    pub theme: MacTheme,
    pub active_pillar: NavigationPillar,
    pub search_query: String,

    // Library & 10D BIM Lifecycle Views
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
    pub settings_view: SettingsView,
    pub ai_assistant_view: AiAssistantView,

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
            settings_view: SettingsView::default(),
            ai_assistant_view: AiAssistantView::default(),

            projects: vec![
                ProjectItem {
                    id: "p1".to_string(),
                    title: "Jembatan Busur Pelengkung Baja Merah".to_string(),
                    category: "Bridges (Family S5)".to_string(),
                    description: "3D Space Truss Arch Bridge with Redundant Cable Hangers".to_string(),
                    elements_count: 432,
                    nodes_count: 128,
                    min_ari: 0.94,
                    max_deflection_mm: 1.20,
                    status: ProjectStatus::Safe,
                    last_modified: "10 mins ago".to_string(),
                    is_favorite: true,
                    is_featured: true,
                    tag: "Priority Research".to_string(),
                },
                ProjectItem {
                    id: "p2".to_string(),
                    title: "Struktur Kubah Bandara Internasional".to_string(),
                    category: "Domes & Shells".to_string(),
                    description: "Double-layer reticulated space frame dome with SVD mechanism certification".to_string(),
                    elements_count: 856,
                    nodes_count: 240,
                    min_ari: 0.91,
                    max_deflection_mm: 2.10,
                    status: ProjectStatus::Safe,
                    last_modified: "Yesterday".to_string(),
                    is_favorite: true,
                    is_featured: false,
                    tag: "Aviation".to_string(),
                },
                ProjectItem {
                    id: "p3".to_string(),
                    title: "Warehouse Transfer Truss Gedebage".to_string(),
                    category: "Industrial".to_string(),
                    description: "High-bay steel warehouse with critical transfer girders undergoing collapse evaluation".to_string(),
                    elements_count: 218,
                    nodes_count: 64,
                    min_ari: 0.64,
                    max_deflection_mm: 4.80,
                    status: ProjectStatus::ReviewNeeded,
                    last_modified: "3 days ago".to_string(),
                    is_favorite: false,
                    is_featured: false,
                    tag: "Screening".to_string(),
                },
            ],

            db_components: vec![
                DatabaseComponent {
                    id: "comp_1".to_string(),
                    name: "Warren Truss 24m Standard".to_string(),
                    category: "Roof Trusses".to_string(),
                    description: "Equilateral alternating web diagonals with L=24m, H=3.2m".to_string(),
                    variables: vec![
                        ParametricVariable { name: "L".to_string(), expression: "24000".to_string(), evaluated_value: 24000.0, unit: "mm".to_string() },
                        ParametricVariable { name: "H".to_string(), expression: "3200".to_string(), evaluated_value: 3200.0, unit: "mm".to_string() },
                    ],
                    elements_count: 21,
                    created_at: "Earlier today".to_string(),
                },
                DatabaseComponent {
                    id: "comp_2".to_string(),
                    name: "Cross-Composite Box Column".to_string(),
                    category: "Columns".to_string(),
                    description: "Built-up cross-shaped steel box column for high axial redundancy".to_string(),
                    variables: vec![
                        ParametricVariable { name: "B".to_string(), expression: "800".to_string(), evaluated_value: 800.0, unit: "mm".to_string() },
                        ParametricVariable { name: "t".to_string(), expression: "25".to_string(), evaluated_value: 25.0, unit: "mm".to_string() },
                    ],
                    elements_count: 8,
                    created_at: "Yesterday".to_string(),
                },
            ],

            members: vec![
                StructuralMember {
                    id: 1,
                    name: "Bottom Chord E1".to_string(),
                    section: "W14x90".to_string(),
                    force_kn: -45.2,
                    capacity_kn: 120.0,
                    utilization: 0.376,
                    moment_my_knm: 12.4,
                    moment_mz_knm: 34.8,
                    shear_vy_kn: 18.2,
                    dcr_interaction: 0.485,
                    rr: 0.5800,
                    ari: 0.5800,
                    ci: 0.4200,
                    svd_margin: 0.8420,
                    is_critical: true,
                },
                StructuralMember {
                    id: 2,
                    name: "Top Chord E2".to_string(),
                    section: "W14x90".to_string(),
                    force_kn: -18.5,
                    capacity_kn: 40.0,
                    utilization: 0.46,
                    moment_my_knm: 8.5,
                    moment_mz_knm: 22.1,
                    shear_vy_kn: 11.4,
                    dcr_interaction: 0.528,
                    rr: 0.7200,
                    ari: 0.6500,
                    ci: 0.3500,
                    svd_margin: 0.7500,
                    is_critical: false,
                },
                StructuralMember {
                    id: 3,
                    name: "Diagonal Brace E3".to_string(),
                    section: "HSS8x8x1/2".to_string(),
                    force_kn: 6.2,
                    capacity_kn: 30.0,
                    utilization: 0.21,
                    moment_my_knm: 2.1,
                    moment_mz_knm: 4.8,
                    shear_vy_kn: 3.5,
                    dcr_interaction: 0.245,
                    rr: 0.8800,
                    ari: 0.8200,
                    ci: 0.1800,
                    svd_margin: 0.8500,
                    is_critical: false,
                },
                StructuralMember {
                    id: 4,
                    name: "Vertical Post E4".to_string(),
                    section: "HSS6x6x3/8".to_string(),
                    force_kn: 2.1,
                    capacity_kn: 25.0,
                    utilization: 0.08,
                    moment_my_knm: 0.8,
                    moment_mz_knm: 1.5,
                    shear_vy_kn: 1.2,
                    dcr_interaction: 0.098,
                    rr: 0.9400,
                    ari: 0.9100,
                    ci: 0.0900,
                    svd_margin: 0.9200,
                    is_critical: false,
                },
            ],
        }
    }
}

impl App for EigenGraphApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 1. APPLE-STYLE FULL-HEIGHT LEFT NAVIGATION SIDEBAR
        // Calling Panel::left FIRST ensures it extends all the way from the top (y=0) to the bottom of the window.
        Panel::left("mac_navigation_sidebar")
            .exact_size(248.0)
            .resizable(false)
            .frame(self.theme.mac_sidebar_frame())
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Window Traffic Light dots space (macOS HIG fullsize content view)
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(2.0);
                        let (rect, _response) = ui.allocate_exact_size(Vec2::new(54.0, 14.0), egui::Sense::hover());
                        let painter = ui.painter();
                        painter.circle_filled(rect.left_top() + Vec2::new(6.0, 7.0), 5.5, Color32::from_rgb(255, 95, 87));
                        painter.circle_filled(rect.left_top() + Vec2::new(22.0, 7.0), 5.5, Color32::from_rgb(254, 188, 46));
                        painter.circle_filled(rect.left_top() + Vec2::new(38.0, 7.0), 5.5, Color32::from_rgb(40, 200, 64));
                    });

                    ui.add_space(14.0);

                    // App Identity / Branding
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚛️ EigenGraph").size(15.5).strong().color(Color32::WHITE));
                    });
                    ui.label(RichText::new("10D BIM & Adjusted Redundancy Index").size(10.0).color(self.theme.accent_blue));

                    ui.add_space(14.0);

                    // 1. LIBRARY SECTION HEADER
                    ui.label(RichText::new("LIBRARY").size(9.5).strong().color(self.theme.text_muted));
                    ui.add_space(4.0);

                    // MY LIBRARY BUTTON
                    {
                        let is_active = self.active_pillar == NavigationPillar::MyLibrary;
                        let bg = if is_active {
                            self.theme.accent_blue
                        } else {
                            Color32::TRANSPARENT
                        };
                        let fg = if is_active {
                            Color32::WHITE
                        } else {
                            self.theme.text_secondary
                        };

                        let badge_bg = if is_active {
                            Color32::from_white_alpha(45)
                        } else {
                            self.theme.bg_card
                        };
                        let badge_fg = if is_active {
                            Color32::WHITE
                        } else {
                            self.theme.accent_blue
                        };

                        let response = Frame::new()
                            .fill(bg)
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(Margin::symmetric(7, 5))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    Frame::new()
                                        .fill(badge_bg)
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(4, 1))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new("LIB").size(9.0).strong().color(badge_fg));
                                        });

                                    ui.add_space(2.0);
                                    ui.label(RichText::new("📁").size(12.0));
                                    ui.add_space(2.0);
                                    ui.label(RichText::new("My Library").size(11.5).strong().color(fg));
                                });
                            });

                        let interact = ui.interact(response.response.rect, response.response.id, egui::Sense::click());
                        if interact.clicked() {
                            self.active_pillar = NavigationPillar::MyLibrary;
                        }
                    }

                    ui.add_space(10.0);

                    // 2. BIM LIFECYCLE (1D–10D) SECTION HEADER
                    ui.label(RichText::new("BIM LIFECYCLE (1D–10D)").size(9.5).strong().color(self.theme.text_muted));
                    ui.add_space(4.0);

                    // 10 Navigation Pillars (Apple Sidebar Capsule Buttons with Dimension Badges)
                    let pillars = [
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
                    ];

                    for p in pillars {
                        let is_active = self.active_pillar == p;
                        let bg = if is_active {
                            self.theme.accent_blue
                        } else {
                            Color32::TRANSPARENT
                        };
                        let fg = if is_active {
                            Color32::WHITE
                        } else {
                            self.theme.text_secondary
                        };

                        let badge_bg = if is_active {
                            Color32::from_white_alpha(45)
                        } else {
                            self.theme.bg_card
                        };
                        let badge_fg = if is_active {
                            Color32::WHITE
                        } else {
                            self.theme.accent_blue
                        };

                        let response = Frame::new()
                            .fill(bg)
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(Margin::symmetric(7, 5))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    // Dimension tag pill
                                    Frame::new()
                                        .fill(badge_bg)
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(4, 1))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(p.dimension_tag()).size(9.0).strong().color(badge_fg));
                                        });

                                    ui.add_space(2.0);
                                    ui.label(RichText::new(p.icon()).size(12.0));
                                    ui.add_space(2.0);
                                    ui.label(RichText::new(p.clean_name()).size(11.5).strong().color(fg));
                                });
                            });

                        let interact = ui.interact(response.response.rect, response.response.id, egui::Sense::click());
                        if interact.clicked() {
                            self.active_pillar = p;
                        }
                        ui.add_space(1.5);
                    }

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // Active Project Quick Insight Card in Sidebar
                    ui.label(RichText::new("ACTIVE PROJECT").size(10.0).strong().color(self.theme.text_muted));
                    ui.add_space(6.0);

                    Frame::new()
                        .fill(self.theme.bg_card)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(10))
                        .stroke(Stroke::new(1.0, self.theme.border_subtle))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Jembatan Busur Pelengkung").size(11.0).strong().color(self.theme.text_primary));
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("ARI:").size(10.0).color(self.theme.text_muted));
                                ui.label(RichText::new("0.94 (Safe)").size(10.0).strong().color(self.theme.accent_green));
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("DB Parts:").size(10.0).color(self.theme.text_muted));
                                ui.label(RichText::new(format!("{} ready", self.db_components.len())).size(10.0).color(self.theme.accent_blue));
                            });
                        });

                    // Push user profile and system status to the bottom of the sidebar
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        ui.add_space(10.0);

                        // Version & Tech stack
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("v1.0.0 • Metal GPU 60 FPS").size(10.0).color(self.theme.text_muted));
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // User Profile (macOS HIG account row)
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("👤").size(16.0));
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Harids Irawan").size(12.0).strong().color(self.theme.text_primary));
                                ui.label(RichText::new("Doctoral Research").size(10.0).color(self.theme.text_muted));
                            });
                        });
                    });
                });
            });

        // 2. BOTTOM STATUS BAR (Runs across content area to right edge)
        Panel::bottom("bottom_status_bar").frame(self.theme.status_bar_frame()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🟢 Model: Jembatan Busur Pelengkung Baja Merah").size(11.0).color(self.theme.text_secondary));
                ui.add_space(16.0);
                ui.label(RichText::new("Units: kN, mm, MPa").size(11.0).color(self.theme.text_muted));
                ui.add_space(16.0);
                ui.label(RichText::new(format!("Database Components: {} ready", self.db_components.len())).size(11.0).color(self.theme.accent_blue));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("Auto-Saved • Ready").size(11.0).color(self.theme.text_muted));
                });
            });
        });

        // 3. TOP CONTENT TOOLBAR (Contextual & Clean macOS Header)
        Panel::top("content_top_header").frame(self.theme.content_top_frame()).show(ui, |ui| {
            ui.horizontal(|ui| {
                // Workspace Title & Subtitle Breadcrumb
                ui.label(RichText::new(self.active_pillar.icon()).size(16.0));
                ui.add_space(4.0);
                ui.label(RichText::new(self.active_pillar.clean_name()).size(14.0).strong().color(self.theme.text_primary));
                ui.label(RichText::new("—").size(12.0).color(self.theme.text_muted));
                ui.label(RichText::new(self.active_pillar.subtitle()).size(12.0).color(self.theme.text_secondary));

                // Global Search Box & Copilot Toggle in Toolbar
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new(RichText::new("🤖 Ask AI (⌘K)").size(11.5).strong().color(self.theme.accent_blue))
                        .fill(self.theme.bg_card)
                        .corner_radius(CornerRadius::same(6))).clicked() {
                        self.ai_assistant_view.expanded = !self.ai_assistant_view.expanded;
                    }

                    ui.add_space(8.0);

                    ui.add(egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍 Filter workspace...")
                        .desired_width(160.0));
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
    }
}
