//! app.rs — Core application struct implementing eframe::App.

use eframe::App;
use egui::{Align, Color32, CornerRadius, Frame, Layout, Panel, CentralPanel, RichText, Vec2};

use crate::models::{DatabaseComponent, NavigationPillar, ParametricVariable, ProjectItem, ProjectStatus, StructuralMember};
use crate::theme::MacTheme;
use crate::views::{AnalyzeView, BuildView, MyProjectView, ParametricView, SettingsView};

pub struct EigenGraphApp {
    pub theme: MacTheme,
    pub active_pillar: NavigationPillar,
    pub search_query: String,

    // Views
    pub my_project_view: MyProjectView,
    pub parametric_view: ParametricView,
    pub build_view: BuildView,
    pub analyze_view: AnalyzeView,
    pub settings_view: SettingsView,

    // Domain Data & Parametric Component Database
    pub projects: Vec<ProjectItem>,
    pub db_components: Vec<DatabaseComponent>,
    pub members: Vec<StructuralMember>,
}

impl Default for EigenGraphApp {
    fn default() -> Self {
        Self {
            theme: MacTheme::default(),
            active_pillar: NavigationPillar::MyProject,
            search_query: String::new(),

            my_project_view: MyProjectView::default(),
            parametric_view: ParametricView::default(),
            build_view: BuildView::default(),
            analyze_view: AnalyzeView::default(),
            settings_view: SettingsView::default(),

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
        // 1. TOP NAVIGATION BAR (Mac App Store Style)
        Panel::top("top_navigation_bar").frame(self.theme.top_bar_frame()).show(ui, |ui| {
            ui.horizontal(|ui| {
                // Brand Logo
                ui.label(RichText::new("⚛️ EigenGraph").size(16.0).strong().color(self.theme.accent_blue));
                ui.add_space(20.0);

                // 5 PILLARS NAVIGATION CONTROLLER
                let pillars = [
                    NavigationPillar::MyProject,
                    NavigationPillar::Parametric,
                    NavigationPillar::Build,
                    NavigationPillar::Analyze,
                    NavigationPillar::Settings,
                ];

                for p in pillars {
                    let is_active = self.active_pillar == p;
                    let bg = if is_active { self.theme.accent_blue } else { Color32::TRANSPARENT };
                    let fg = if is_active { Color32::WHITE } else { self.theme.text_secondary };

                    let btn = egui::Button::new(RichText::new(p.title()).size(12.0).strong().color(fg))
                        .fill(bg)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(Vec2::new(110.0, 28.0));

                    if ui.add(btn).clicked() {
                        self.active_pillar = p;
                    }
                }

                // Global Search & User Profile
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("👤 HI").size(12.0).strong().color(self.theme.text_secondary));
                    ui.add_space(10.0);
                    ui.add(egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍 Search projects (⌘K)...")
                        .desired_width(180.0));
                });
            });
        });

        // 2. BOTTOM STATUS BAR
        Panel::bottom("bottom_status_bar").frame(self.theme.sidebar_frame()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🟢 Model: Jembatan Busur Pelengkung Baja Merah").size(11.0).color(self.theme.text_secondary));
                ui.add_space(16.0);
                ui.label(RichText::new("Units: kN, mm, MPa").size(11.0).color(self.theme.text_muted));
                ui.add_space(16.0);
                ui.label(RichText::new(format!("Database Components: {} ready", self.db_components.len())).size(11.0).color(self.theme.accent_blue));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("60 FPS • Metal GPU").size(11.0).color(self.theme.text_muted));
                });
            });
        });

        // 3. CENTRAL PANEL (Views Router)
        CentralPanel::default().frame(Frame::new().fill(self.theme.bg_canvas)).show(ui, |ui| {
            match self.active_pillar {
                NavigationPillar::MyProject => {
                    self.my_project_view.show(ui, &self.theme, &self.projects, &mut self.active_pillar);
                }
                NavigationPillar::Parametric => {
                    self.parametric_view.show(ui, &self.theme, &mut self.db_components, &mut self.active_pillar);
                }
                NavigationPillar::Build => {
                    self.build_view.show(ui, &self.theme, &self.members, &self.db_components);
                }
                NavigationPillar::Analyze => {
                    self.analyze_view.show(ui, &self.theme, &self.members);
                }
                NavigationPillar::Settings => {
                    self.settings_view.show(ui, &self.theme);
                }
            }
        });
    }
}
