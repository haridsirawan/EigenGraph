//! app.rs — Core application struct implementing eframe::App.

use eframe::App;
use egui::{Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Panel, CentralPanel, RichText, Stroke, Vec2};

use crate::models::{DatabaseComponent, NavigationPillar, ParametricVariable, ProjectItem, ProjectStatus, StructuralMember};
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
                ProjectItem {
                    id: "p4".to_string(),
                    title: "Menara Diagrid 48-Lantai dengan Outrigger (Demo IFC)".to_string(),
                    category: "High-Rise (Family S8)".to_string(),
                    description: "Kompleksitas tinggi: kisi fasad diagrid berlian, 2 tingkat outrigger transfer, dan kolom komposit CFT berpenegar (Benchmark OpenBIM IFC 4.3)".to_string(),
                    elements_count: 1480,
                    nodes_count: 420,
                    min_ari: 0.89,
                    max_deflection_mm: 12.80,
                    status: ProjectStatus::Safe,
                    last_modified: "Hari ini".to_string(),
                    is_favorite: true,
                    is_featured: false,
                    tag: "OpenBIM IFC4".to_string(),
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
                    ..Default::default()
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
                    ..Default::default()
                },
                DatabaseComponent {
                    id: "comp_3".to_string(),
                    name: "Modul Diagrid Fasad 2-Lantai (Diamond Node)".to_string(),
                    category: "Fasad & Diagrid (IFC)".to_string(),
                    description: "Modul kisi berlian perimeter pemikul beban gravitasi & geser lateral tanpa kolom vertikal".to_string(),
                    variables: vec![
                        ParametricVariable { name: "W_bay".to_string(), expression: "9000".to_string(), evaluated_value: 9000.0, unit: "mm".to_string() },
                        ParametricVariable { name: "H_story".to_string(), expression: "4200".to_string(), evaluated_value: 4200.0, unit: "mm".to_string() },
                        ParametricVariable { name: "theta_diag".to_string(), expression: "67.4".to_string(), evaluated_value: 67.4, unit: "deg".to_string() },
                        ParametricVariable { name: "D_pipe".to_string(), expression: "610".to_string(), evaluated_value: 610.0, unit: "mm (CHS)".to_string() },
                        ParametricVariable { name: "t_pipe".to_string(), expression: "25".to_string(), evaluated_value: 25.0, unit: "mm".to_string() },
                        ParametricVariable { name: "Fy_baja".to_string(), expression: "355".to_string(), evaluated_value: 355.0, unit: "MPa".to_string() },
                    ],
                    elements_count: 8,
                    created_at: "Demo IFC".to_string(),
                    ..Default::default()
                },
                DatabaseComponent {
                    id: "comp_4".to_string(),
                    name: "Rangka Outrigger & Sabuk Transfer 2-Lantai".to_string(),
                    category: "Truss Transfer (IFC)".to_string(),
                    description: "Sabuk pengaku inti geser (core) ke mega-kolom perimeter untuk membatasi drift goyangan gedung tinggi".to_string(),
                    variables: vec![
                        ParametricVariable { name: "Span_core".to_string(), expression: "14000".to_string(), evaluated_value: 14000.0, unit: "mm".to_string() },
                        ParametricVariable { name: "H_truss".to_string(), expression: "8400".to_string(), evaluated_value: 8400.0, unit: "mm".to_string() },
                        ParametricVariable { name: "Chord_Flange".to_string(), expression: "600".to_string(), evaluated_value: 600.0, unit: "mm".to_string() },
                        ParametricVariable { name: "Web_t".to_string(), expression: "40".to_string(), evaluated_value: 40.0, unit: "mm".to_string() },
                        ParametricVariable { name: "Fy_baja".to_string(), expression: "420".to_string(), evaluated_value: 420.0, unit: "MPa".to_string() },
                    ],
                    elements_count: 14,
                    created_at: "Demo IFC".to_string(),
                    ..Default::default()
                },
                DatabaseComponent {
                    id: "comp_5".to_string(),
                    name: "Gelagar Balok Transfer Vierendeel Bebas Kolom".to_string(),
                    category: "Gelagar Momen (IFC)".to_string(),
                    description: "Rangka kaku momen tanpa batang diagonal pengisi untuk bukaan lorong atau entrance lobi bertingkat".to_string(),
                    variables: vec![
                        ParametricVariable { name: "L_clear".to_string(), expression: "18000".to_string(), evaluated_value: 18000.0, unit: "mm".to_string() },
                        ParametricVariable { name: "H_girder".to_string(), expression: "3600".to_string(), evaluated_value: 3600.0, unit: "mm".to_string() },
                        ParametricVariable { name: "D_chord".to_string(), expression: "800".to_string(), evaluated_value: 800.0, unit: "mm".to_string() },
                        ParametricVariable { name: "S_post".to_string(), expression: "3000".to_string(), evaluated_value: 3000.0, unit: "mm".to_string() },
                        ParametricVariable { name: "Mp_cap".to_string(), expression: "4250".to_string(), evaluated_value: 4250.0, unit: "kNm".to_string() },
                    ],
                    elements_count: 17,
                    created_at: "Demo IFC".to_string(),
                    ..Default::default()
                },
                DatabaseComponent {
                    id: "comp_6".to_string(),
                    name: "Kolom Mega Komposit CFT Berpenegar".to_string(),
                    category: "Mega-Columns (IFC)".to_string(),
                    description: "Tabung baja masif diisi beton mutu tinggi f'c=60 MPa dengan pelat penegar internal anti-tekuk lokal".to_string(),
                    variables: vec![
                        ParametricVariable { name: "B_col".to_string(), expression: "1600".to_string(), evaluated_value: 1600.0, unit: "mm".to_string() },
                        ParametricVariable { name: "t_steel".to_string(), expression: "50".to_string(), evaluated_value: 50.0, unit: "mm".to_string() },
                        ParametricVariable { name: "fc_concrete".to_string(), expression: "60".to_string(), evaluated_value: 60.0, unit: "MPa".to_string() },
                        ParametricVariable { name: "Tie_spacing".to_string(), expression: "150".to_string(), evaluated_value: 150.0, unit: "mm".to_string() },
                        ParametricVariable { name: "Pn_axial".to_string(), expression: "54000".to_string(), evaluated_value: 54000.0, unit: "kN".to_string() },
                    ],
                    elements_count: 12,
                    created_at: "Demo IFC".to_string(),
                    ..Default::default()
                },
                DatabaseComponent {
                    id: "comp_7".to_string(),
                    name: "Node Sambungan Bola MERO & Batang Kubah Atrium".to_string(),
                    category: "Space Frame (IFC)".to_string(),
                    description: "Sistem sambungan bola tempa multi-aksial untuk atap kubah kaca atrium berbentang lebar bebas kolom".to_string(),
                    variables: vec![
                        ParametricVariable { name: "D_ball".to_string(), expression: "300".to_string(), evaluated_value: 300.0, unit: "mm".to_string() },
                        ParametricVariable { name: "Strut_OD".to_string(), expression: "219.1".to_string(), evaluated_value: 219.1, unit: "mm".to_string() },
                        ParametricVariable { name: "Bolt_Grade".to_string(), expression: "10.9".to_string(), evaluated_value: 10.9, unit: "ISO".to_string() },
                        ParametricVariable { name: "Max_Angle".to_string(), expression: "45".to_string(), evaluated_value: 45.0, unit: "deg".to_string() },
                        ParametricVariable { name: "Nt_cap".to_string(), expression: "850".to_string(), evaluated_value: 850.0, unit: "kN".to_string() },
                    ],
                    elements_count: 24,
                    created_at: "Demo IFC".to_string(),
                    ..Default::default()
                },
                DatabaseComponent {
                    id: "comp_8".to_string(),
                    name: "Balok Kantilever Tapered Jembatan Layang (Skybridge)".to_string(),
                    category: "Cantilevers (IFC)".to_string(),
                    description: "Gelagar boks tapered pascatarik aerodinamis penghubung layang antar menara gedung bertingkat".to_string(),
                    variables: vec![
                        ParametricVariable { name: "L_cant".to_string(), expression: "22000".to_string(), evaluated_value: 22000.0, unit: "mm".to_string() },
                        ParametricVariable { name: "H_root".to_string(), expression: "4500".to_string(), evaluated_value: 4500.0, unit: "mm".to_string() },
                        ParametricVariable { name: "H_tip".to_string(), expression: "1800".to_string(), evaluated_value: 1800.0, unit: "mm".to_string() },
                        ParametricVariable { name: "P_jack".to_string(), expression: "12500".to_string(), evaluated_value: 12500.0, unit: "kN".to_string() },
                    ],
                    elements_count: 16,
                    created_at: "Demo IFC".to_string(),
                    ..Default::default()
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
                    moment_mz_knm: 2.1,
                    is_critical: false,
                    is_active: true,
                    ari_score: 0.94,
                    svd_sensitivity: 0.02,
                    ari: 0.94,
                    dcr_interaction: 0.376,
                    shear_vy_kn: 12.4,
                    svd_margin: 0.98,
                    ci: 0.06,
                    rr: 0.94,
                },
                StructuralMember {
                    id: 2,
                    name: "Bottom Chord E2".to_string(),
                    section: "W14x90".to_string(),
                    force_kn: -42.8,
                    capacity_kn: 120.0,
                    utilization: 0.356,
                    moment_my_knm: 11.2,
                    moment_mz_knm: 1.8,
                    is_critical: false,
                    is_active: true,
                    ari_score: 0.94,
                    svd_sensitivity: 0.02,
                    ari: 0.94,
                    dcr_interaction: 0.356,
                    shear_vy_kn: 11.2,
                    svd_margin: 0.98,
                    ci: 0.06,
                    rr: 0.94,
                },
                StructuralMember {
                    id: 3,
                    name: "Top Chord T1".to_string(),
                    section: "W14x90".to_string(),
                    force_kn: 38.5,
                    capacity_kn: 95.0,
                    utilization: 0.405,
                    moment_my_knm: 14.5,
                    moment_mz_knm: 3.2,
                    is_critical: false,
                    is_active: true,
                    ari_score: 0.91,
                    svd_sensitivity: 0.04,
                    ari: 0.91,
                    dcr_interaction: 0.405,
                    shear_vy_kn: 14.5,
                    svd_margin: 0.96,
                    ci: 0.09,
                    rr: 0.91,
                },
                StructuralMember {
                    id: 4,
                    name: "Critical Web Diagonal D1 (Disruption Test)".to_string(),
                    section: "2L 4x4x3/8".to_string(),
                    force_kn: 82.4,
                    capacity_kn: 90.0,
                    utilization: 0.915,
                    moment_my_knm: 24.1,
                    moment_mz_knm: 5.6,
                    is_critical: true,
                    is_active: true,
                    ari_score: 0.52,
                    svd_sensitivity: 0.18,
                    ari: 0.52,
                    dcr_interaction: 0.915,
                    shear_vy_kn: 24.1,
                    svd_margin: 0.82,
                    ci: 0.48,
                    rr: 0.52,
                },
                StructuralMember {
                    id: 5,
                    name: "Web Diagonal D2".to_string(),
                    section: "2L 4x4x3/8".to_string(),
                    force_kn: -18.2,
                    capacity_kn: 90.0,
                    utilization: 0.202,
                    moment_my_knm: 5.4,
                    moment_mz_knm: 0.8,
                    is_critical: false,
                    is_active: true,
                    ari_score: 0.96,
                    svd_sensitivity: 0.01,
                    ari: 0.96,
                    dcr_interaction: 0.202,
                    shear_vy_kn: 5.4,
                    svd_margin: 0.99,
                    ci: 0.04,
                    rr: 0.96,
                },
                StructuralMember {
                    id: 6,
                    name: "Web Diagonal D3".to_string(),
                    section: "2L 4x4x3/8".to_string(),
                    force_kn: 22.0,
                    capacity_kn: 90.0,
                    utilization: 0.244,
                    moment_my_knm: 6.8,
                    moment_mz_knm: 1.1,
                    is_critical: false,
                    is_active: true,
                    ari_score: 0.95,
                    svd_sensitivity: 0.01,
                    ari: 0.95,
                    dcr_interaction: 0.244,
                    shear_vy_kn: 6.8,
                    svd_margin: 0.99,
                    ci: 0.05,
                    rr: 0.95,
                },
                StructuralMember {
                    id: 7,
                    name: "Vertical Post V1".to_string(),
                    section: "W10x49".to_string(),
                    force_kn: 12.1,
                    capacity_kn: 80.0,
                    utilization: 0.151,
                    moment_my_knm: 3.1,
                    moment_mz_knm: 0.5,
                    is_critical: false,
                    is_active: true,
                    ari_score: 0.98,
                    svd_sensitivity: 0.01,
                    ari: 0.98,
                    dcr_interaction: 0.151,
                    shear_vy_kn: 3.1,
                    svd_margin: 0.99,
                    ci: 0.02,
                    rr: 0.98,
                },
                StructuralMember {
                    id: 8,
                    name: "Vertical Post V2".to_string(),
                    section: "W10x49".to_string(),
                    force_kn: 11.5,
                    capacity_kn: 80.0,
                    utilization: 0.143,
                    moment_my_knm: 2.9,
                    moment_mz_knm: 0.4,
                    is_critical: false,
                    is_active: true,
                    ari_score: 0.98,
                    svd_sensitivity: 0.01,
                    ari: 0.98,
                    dcr_interaction: 0.143,
                    shear_vy_kn: 2.9,
                    svd_margin: 0.99,
                    ci: 0.02,
                    rr: 0.98,
                },
            ],
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
                ui.label(RichText::new("EigenGraph").strong().size(13.5).color(self.theme.text_primary));
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

        // 2. UNIFIED LEFT SIDEBAR (Activity Bar with 11 Navigation Pillars)
        Panel::left("unified_sidebar")
            .resizable(false)
            .exact_size(245.0)
            .frame(Frame::new()
                .fill(self.theme.bg_sidebar)
                .stroke(Stroke::new(1.0, self.theme.border_subtle))
                .inner_margin(Margin::symmetric(10, 14)))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Sidebar Header
                    ui.label(RichText::new("LIFECYCLE DIMENSIONS").size(11.0).strong().color(self.theme.text_muted));
                    ui.add_space(6.0);

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
                            let tag_color = if is_active { Color32::WHITE } else { self.theme.accent_blue };

                            let row = Frame::new()
                                .fill(bg)
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(10, 7))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(pillar.icon()).size(14.0));
                                        ui.add_space(4.0);
                                        ui.label(RichText::new(pillar.clean_name()).size(12.5).color(fg).strong());
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(RichText::new(pillar.dimension_tag()).size(10.0).strong().color(tag_color));
                                        });
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
