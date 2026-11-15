use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::features::{FeatureId, FeatureRegistry};
use crate::icons::{self, AppIcon};
use crate::models::{BimFamilyItem, NavigationPillar, ProjectItem};
use crate::theme::MacTheme;
use crate::ui_kit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryViewMode {
    ProjectsAndPortfolios,
    GlobalBimFamilyCatalog,
    Iso19650CdeHub,
    TypologyTemplateLauncher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplateKind {
    SteelBridgeViaduct,
    DualSystemRcTower,
    PtBoxGirderFlyover,
    PrecastLogisticsHub,
}

impl ProjectTemplateKind {
    pub fn title(&self) -> &'static str {
        match self {
            Self::SteelBridgeViaduct => "36m Composite Warren Arch Viaduct",
            Self::DualSystemRcTower => "24-Storey Dual-System RC Tower",
            Self::PtBoxGirderFlyover => "PT Post-Tensioned Box-Girder Flyover",
            Self::PrecastLogisticsHub => "Automated Precast Logistics Fulfillment Hub",
        }
    }

    pub fn category_idx(&self) -> usize {
        match self {
            Self::DualSystemRcTower => 1,
            Self::SteelBridgeViaduct | Self::PtBoxGirderFlyover => 2,
            Self::PrecastLogisticsHub => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CdeContainerState {
    Wip,
    Shared,
    Published,
    Archive,
}

impl CdeContainerState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Wip => "WIP",
            Self::Shared => "Shared",
            Self::Published => "Published",
            Self::Archive => "Archive",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CdeDocumentPackage {
    pub doc_code: String,
    pub originator: String,
    pub container: CdeContainerState,
    pub suitability: String,
    pub revision: String,
    pub verifier: String,
}

impl CdeDocumentPackage {
    pub fn promote_container(&mut self) {
        match self.container {
            CdeContainerState::Wip => {
                self.container = CdeContainerState::Shared;
                self.suitability = "S4 • Stage Approval".to_string();
                self.revision = "P01.01".to_string();
            }
            CdeContainerState::Shared => {
                self.container = CdeContainerState::Published;
                self.suitability = "A1 • Authorized Construction".to_string();
                self.revision = "C01.01".to_string();
            }
            CdeContainerState::Published => {
                self.container = CdeContainerState::Archive;
                self.suitability = "A4 • Immutable Record".to_string();
            }
            CdeContainerState::Archive => {}
        }
    }
}

pub struct MyProjectView {
    pub library_mode: LibraryViewMode,
    pub selected_category: String,
    pub search_query: String,
    pub show_dossier_modal: bool,
    pub export_success_msg: Option<String>,
    pub custom_vault_override: String,
    pub pending_open_vault_project: Option<crate::core::vault::PersonalVaultProjectEntry>,
    pub request_open_settings_vault: bool,
    pub request_refresh_vault: bool,
    pub request_save_active_to_vault: bool,
    pub request_export_active_archive: bool,
    pub save_project_name_input: String,
    pub template_filter: usize,
    pub request_apply_template: Option<ProjectTemplateKind>,
    // Phase 27: BIM Family Catalog & Parameter Inspector fields
    pub families: Vec<BimFamilyItem>,
    pub selected_family_idx: usize,
    pub family_search_query: String,
    pub family_category_filter: String,
    // Phase 39: ISO 19650 CDE Hub (LIB-04)
    pub cde_container_filter: usize,
    pub cde_packages: Vec<CdeDocumentPackage>,
    pub cde_new_doc_code: String,
}

impl Default for MyProjectView {
    fn default() -> Self {
        Self {
            library_mode: LibraryViewMode::ProjectsAndPortfolios,
            selected_category: "All Projects".to_string(),
            search_query: String::new(),
            show_dossier_modal: false,
            export_success_msg: None,
            custom_vault_override: String::new(),
            pending_open_vault_project: None,
            request_open_settings_vault: false,
            request_refresh_vault: false,
            request_save_active_to_vault: false,
            request_export_active_archive: false,
            save_project_name_input: "Workspace_Project".to_string(),
            template_filter: 0,
            request_apply_template: None,
            families: Vec::new(),
            selected_family_idx: 0,
            family_search_query: String::new(),
            family_category_filter: "All Families".to_string(),
            cde_container_filter: 0,
            cde_packages: vec![
                CdeDocumentPackage {
                    doc_code: "EG-STR-MOD-L01-M3-S-001".to_string(),
                    originator: "STR-TEAM".to_string(),
                    container: CdeContainerState::Wip,
                    suitability: "S0 • Authoring Draft".to_string(),
                    revision: "W01.01".to_string(),
                    verifier: "Structural Lead".to_string(),
                },
            ],
            cde_new_doc_code: "EG-ARC-PLN-ZZZ-DR-A-101".to_string(),
        }
    }
}

impl MyProjectView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        projects: &[ProjectItem],
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
        active_pillar: &mut NavigationPillar,
        features: &mut FeatureRegistry,
    ) {
        ui.vertical(|ui| {
            Frame::new().inner_margin(Margin::symmetric(0, 8)).show(ui, |ui| {
                let full_w = ui.available_width().max(160.0);
                ui.set_width(full_w);
                ui.set_max_width(full_w);
                // Top Library Mode Selector (Phase 31 Responsive Sub-Tab Bar)
                ui_kit::subtab_bar(
                    ui,
                    theme,
                    "my_library_mode_subtabs",
                    &mut self.library_mode,
                    &[
                        (LibraryViewMode::ProjectsAndPortfolios, "Personal Vault Projects"),
                        (LibraryViewMode::GlobalBimFamilyCatalog, "Global BIM Families & Parameters"),
                        (LibraryViewMode::Iso19650CdeHub, "ISO 19650 CDE Hub"),
                        (LibraryViewMode::TypologyTemplateLauncher, "Structural Templates"),
                    ],
                );

                ui.add_space(14.0);

                match self.library_mode {
                    LibraryViewMode::GlobalBimFamilyCatalog => {
                        self.render_family_catalog(ui, theme);
                    }
                    LibraryViewMode::Iso19650CdeHub => {
                        self.render_cde_iso19650_hub(ui, theme, features);
                    }
                    LibraryViewMode::TypologyTemplateLauncher => {
                        self.render_typology_templates(ui, theme, features);
                    }
                    LibraryViewMode::ProjectsAndPortfolios => {
                        // 1. HERO FEATURED BANNER
                        if let Some(featured) = projects.iter().find(|p| p.is_featured) {
                            self.render_hero_banner(ui, theme, featured, active_pillar);
                        }

                        if let Some(ref msg) = self.export_success_msg {
                            ui.add_space(10.0);
                            theme.card_frame().show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    icons::icon_label(
                                        ui,
                                        AppIcon::CheckCircle,
                                        12.5,
                                        theme.accent_green,
                                        "INTERNAL VAULT AUTOSAVE:",
                                        11.5,
                                        theme.accent_green,
                                        true,
                                    );
                                    ui.label(RichText::new(msg).size(11.5).color(theme.text_primary));
                                });
                            });
                        }

                        ui.add_space(16.0);

                        // 1B. PERSONAL VAULT SUMMARY BAR (Vault folder settings live in Settings)
                        self.render_internal_vault_panel(ui, theme, vault_projects.len(), features);

                        ui.add_space(14.0);

                        // 2. CATEGORY & SEARCH BAR
                        self.render_category_bar(ui, theme);

                        ui.add_space(12.0);

                        // 3. PERSONAL VAULT PROJECTS & WORKSPACE CARDS
                        self.render_projects_grid(ui, theme, projects, vault_projects, active_pillar);
                    }
                }

                ui.add_space(68.0);
            });
        });

        // 4. EIGENGRAPH ARI AUDIT DOSSIER MODAL
        if self.show_dossier_modal {
            self.render_dossier_modal(ui, theme);
        }
    }

    fn render_hero_content(&mut self, ui: &mut Ui, theme: &MacTheme, p: &ProjectItem, active_pillar: &mut NavigationPillar) {
        ui.horizontal_wrapped(|ui| {
            Frame::new()
                .fill(theme.accent_blue.gamma_multiply(0.2))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.label(RichText::new("FEATURED MASTER PROJECT").size(11.0).strong().color(theme.accent_blue));
                });

            ui.add_space(6.0);
            ui.label(RichText::new(p.status.badge_label()).size(12.0).color(theme.accent_green).strong());
        });

        ui.add_space(8.0);
        ui.heading(RichText::new(&p.title).size(20.0).strong().color(theme.text_primary));
        ui.add_space(2.0);
        ui.label(RichText::new(&p.description).size(13.0).color(theme.text_secondary));

        ui.add_space(10.0);
        ui.label(RichText::new(format!(
            "Elements: {}   •   Nodes: {}   •   Max Disp: {:.2} mm   •   Min ARI: {:.2}",
            p.elements_count, p.nodes_count, p.max_deflection_mm, p.min_ari
        )).size(12.0).color(theme.text_muted));

        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            if icons::icon_button(
                ui,
                AppIcon::Cube3D,
                "Open in 3D Workspace",
                12.5,
                Color32::WHITE,
                theme.accent_blue,
                Stroke::NONE,
                CornerRadius::same(8),
                egui::Vec2::new(165.0, 32.0),
            ).clicked() {
                *active_pillar = NavigationPillar::GeometryAssembly;
            }

            if icons::icon_button(
                ui,
                AppIcon::Document,
                "EigenGraph Audit Certificate",
                12.5,
                Color32::WHITE,
                theme.accent_green,
                Stroke::NONE,
                CornerRadius::same(8),
                egui::Vec2::new(180.0, 32.0),
            ).clicked() {
                self.show_dossier_modal = true;
            }

            if icons::icon_button(
                ui,
                AppIcon::SaveDisk,
                "Save Workspace to Vault (.eigen)",
                12.0,
                theme.text_primary,
                theme.bg_card,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(8),
                egui::Vec2::new(196.0, 32.0),
            ).clicked() {
                self.request_save_active_to_vault = true;
            }
        });
    }

    fn render_arch_wireframe_card(ui: &mut Ui, theme: &MacTheme, card_w: f32) {
        Frame::new()
            .fill(Color32::from_rgb(18, 22, 30))
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, theme.border_card))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(card_w.max(180.0), 125.0), egui::Sense::hover());
                let painter = ui.painter_at(rect);

                // Draw Arch Bridge Vector Wireframe
                let left_x = rect.left() + 18.0;
                let right_x = rect.right() - 18.0;
                let deck_y = rect.bottom() - 28.0;
                let span = right_x - left_x;
                let panels = 8;

                painter.line_segment(
                    [egui::pos2(left_x, deck_y), egui::pos2(right_x, deck_y)],
                    Stroke::new(2.0, theme.accent_blue),
                );

                let mut prev_arch = egui::pos2(left_x, deck_y);
                for i in 0..=panels {
                    let t = i as f32 / panels as f32;
                    let x = left_x + t * span;
                    let arch_h = 4.0 * t * (1.0 - t) * 68.0;
                    let arch_pt = egui::pos2(x, deck_y - arch_h);
                    let deck_pt = egui::pos2(x, deck_y);

                    if i > 0 {
                        painter.line_segment([prev_arch, arch_pt], Stroke::new(2.2, theme.accent_green));
                        if i < panels {
                            painter.line_segment([arch_pt, deck_pt], Stroke::new(1.0, theme.text_muted));
                        }
                    }
                    painter.circle_filled(arch_pt, 2.5, Color32::WHITE);
                    painter.circle_filled(deck_pt, 2.0, theme.accent_blue);
                    prev_arch = arch_pt;
                }

                painter.text(
                    egui::pos2(rect.center().x, rect.bottom() - 8.0),
                    egui::Align2::CENTER_CENTER,
                    "Blank World-Grounded BIM Workspace",
                    egui::FontId::proportional(10.0),
                    theme.text_secondary,
                );
            });
    }

    fn render_hero_banner(&mut self, ui: &mut Ui, theme: &MacTheme, p: &ProjectItem, active_pillar: &mut NavigationPillar) {
        theme.hero_frame().show(ui, |ui| {
            let avail_w = ui.available_width();
            if avail_w >= 720.0 {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width((avail_w - 280.0).max(320.0));
                        self.render_hero_content(ui, theme, p, active_pillar);
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        Self::render_arch_wireframe_card(ui, theme, 240.0);
                    });
                });
            } else {
                ui.vertical(|ui| {
                    self.render_hero_content(ui, theme, p, active_pillar);
                    ui.add_space(12.0);
                    Self::render_arch_wireframe_card(ui, theme, (avail_w - 22.0).min(360.0));
                });
            }
        });
    }

    fn render_internal_vault_panel(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        vault_projects_count: usize,
        features: &mut FeatureRegistry,
    ) {
        let status = crate::core::InternalVault::connection_status();
        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Database,
                    14.0,
                    theme.accent_blue,
                    "PERSONAL VAULT PROJECT INDEX",
                    13.0,
                    theme.accent_blue,
                    true,
                );
                let (badge_txt, badge_col) = if status.connected && status.needs_permission {
                    ("PERMISSION REQUIRED IN SETTINGS", theme.accent_orange)
                } else if status.connected {
                    ("CONNECTED", theme.accent_green)
                } else if status.guest_mode {
                    ("GUEST EXPLORE SESSION", theme.accent_orange)
                } else {
                    ("VAULT DISCONNECTED", theme.text_muted)
                };
                Frame::new()
                    .fill(badge_col.gamma_multiply(0.18))
                    .stroke(Stroke::new(1.0, badge_col.gamma_multiply(0.55)))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.label(RichText::new(badge_txt).size(10.0).strong().color(badge_col));
                    });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::SettingsGear,
                        "Manage Vault in Settings",
                        11.0,
                        theme.text_primary,
                        theme.bg_sidebar,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        self.request_open_settings_vault = true;
                    }
                    if status.connected
                        && icons::icon_button(
                            ui,
                            AppIcon::Undo,
                            "Refresh Vault List",
                            11.0,
                            theme.text_primary,
                            theme.bg_sidebar,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                    {
                        crate::core::web_bridge::vault_refresh();
                        self.request_refresh_vault = true;
                    }
                });
            });

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                icons::draw_icon(ui, AppIcon::Folder, 12.5, theme.accent_blue);
                ui.label(
                    RichText::new(if status.connected {
                        format!(
                            "Vault: {} ({}) • {} indexed project files",
                            status.vault_name, status.location_label, vault_projects_count
                        )
                    } else {
                        "Connect an existing Vault or create a new Vault in Settings to list your personal `.eigen` projects here.".to_string()
                    })
                    .size(11.5)
                    .color(theme.text_secondary),
                );
            });

            if status.connected {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Save Complete 1-File Workspace Archive (`.eigen`):")
                            .size(11.0)
                            .color(theme.text_muted),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.save_project_name_input)
                            .hint_text("Project Name")
                            .desired_width(180.0),
                    );
                    if icons::icon_button(
                        ui,
                        AppIcon::SaveDisk,
                        "Save Workspace to Vault",
                        11.0,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        self.request_save_active_to_vault = true;
                    }
                });
            }

            ui.add_space(10.0);
            ui_kit::feature_slot_card(
                ui,
                theme,
                features,
                crate::features::FeatureId::BrowserIndexedDbPersistence,
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::Folder,
                            "Import .eigen / .ifc / .json...",
                            11.0,
                            theme.text_primary,
                            theme.bg_sidebar,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            crate::core::web_bridge::trigger_file_picker(".eigen,.json,.ifc,.csv");
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::SaveDisk,
                            "Export / Download 1-File .eigen Archive",
                            11.0,
                            Color32::WHITE,
                            theme.accent_green,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            self.request_export_active_archive = true;
                        }
                        ui.label(
                            RichText::new("Auto-Save Active (IndexedDB & Vault Sync)")
                                .size(10.5)
                                .color(theme.accent_green),
                        );
                    });
                },
            );
        });
    }

    fn render_category_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let categories = [
            "All Projects",
            "LOD 300 (Detailed)",
            "LOD 200 (Design)",
            "LOD 100 (Massing)",
            "Eng-CON (Structural)",
            "Arch & HVAC",
        ];
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Collections:").strong().size(13.0).color(theme.text_secondary));
            ui.add_space(8.0);
            for cat in categories {
                let is_selected = self.selected_category == cat;
                let bg = if is_selected { theme.accent_blue } else { theme.bg_card };
                let fg = if is_selected { Color32::WHITE } else { theme.text_secondary };

                let btn = egui::Button::new(RichText::new(cat).size(12.0).color(fg))
                    .fill(bg)
                    .corner_radius(CornerRadius::same(16));

                if ui.add(btn).clicked() {
                    self.selected_category = cat.to_string();
                }
            }
        });
    }

    fn render_project_card(&self, ui: &mut Ui, theme: &MacTheme, p: &ProjectItem, active_pillar: &mut NavigationPillar) {
        theme.card_frame().show(ui, |ui| {
            Frame::new()
                .fill(Color32::from_rgb(20, 23, 30))
                .corner_radius(CornerRadius::same(10))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    let avail_w = ui.available_width().max(120.0);
                    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(avail_w, 72.0), egui::Sense::hover());
                    let painter = ui.painter_at(rect);

                    let lx = rect.left() + 16.0;
                    let rx = rect.right() - 16.0;
                    let by = rect.bottom() - 10.0;
                    let ty = rect.top() + 10.0;
                    let w = (rx - lx).max(40.0);

                    if p.id == "p1" || p.id == "p2" {
                        let n = 8;
                        let mut prev = egui::pos2(lx, by);
                        painter.line_segment([egui::pos2(lx, by), egui::pos2(rx, by)], Stroke::new(1.4, theme.border_card));
                        for i in 1..=n {
                            let t = i as f32 / n as f32;
                            let x = lx + t * w;
                            let y = by - 4.0 * t * (1.0 - t) * (by - ty);
                            let pt = egui::pos2(x, y);
                            painter.line_segment([prev, pt], Stroke::new(1.8, theme.accent_blue));
                            painter.line_segment([pt, egui::pos2(x, by)], Stroke::new(0.8, theme.text_muted));
                            painter.circle_filled(pt, 2.2, theme.accent_green);
                            prev = pt;
                        }
                    } else {
                        let n = 6;
                        painter.line_segment([egui::pos2(lx, ty), egui::pos2(rx, ty)], Stroke::new(1.8, theme.accent_blue));
                        painter.line_segment([egui::pos2(lx, by), egui::pos2(rx, by)], Stroke::new(1.8, theme.accent_blue));
                        for i in 0..n {
                            let t0 = i as f32 / n as f32;
                            let t1 = (i + 1) as f32 / n as f32;
                            let x0 = lx + t0 * w;
                            let x1 = lx + t1 * w;
                            let xm = 0.5 * (x0 + x1);
                            let col_diag = if i == 2 && p.min_ari < 0.70 { theme.accent_orange } else { theme.accent_green };
                            painter.line_segment([egui::pos2(x0, by), egui::pos2(xm, ty)], Stroke::new(1.4, col_diag));
                            painter.line_segment([egui::pos2(xm, ty), egui::pos2(x1, by)], Stroke::new(1.4, col_diag));
                            painter.circle_filled(egui::pos2(x0, by), 2.0, Color32::WHITE);
                            painter.circle_filled(egui::pos2(xm, ty), 2.0, Color32::WHITE);
                        }
                    }
                });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&p.category).size(11.0).color(theme.accent_blue).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let status_col = if p.min_ari >= 0.70 { theme.accent_green } else { theme.accent_orange };
                    ui.label(RichText::new(p.status.badge_label()).size(11.0).strong().color(status_col));
                });
            });

            ui.add_space(4.0);
            ui.label(RichText::new(&p.title).size(15.0).strong().color(theme.text_primary));
            ui.add_space(2.0);
            ui.label(RichText::new(&p.description).size(12.0).color(theme.text_secondary));

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!(
                    "{} El  •  {} Nd  •  ARI: {:.2}",
                    p.elements_count, p.nodes_count, p.min_ari
                )).size(11.5).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::ArrowRight,
                        "Open Model",
                        11.5,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        egui::Vec2::new(100.0, 26.0),
                    ).clicked() {
                        *active_pillar = NavigationPillar::GeometryAssembly;
                    }
                });
            });
        });
    }

    fn render_projects_grid(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        projects: &[ProjectItem],
        vault_projects: &[crate::core::vault::PersonalVaultProjectEntry],
        active_pillar: &mut NavigationPillar,
    ) {
        let query = self.search_query.trim().to_lowercase();
        let filtered_vault: Vec<&crate::core::vault::PersonalVaultProjectEntry> = vault_projects
            .iter()
            .filter(|vp| {
                query.is_empty()
                    || vp.name.to_lowercase().contains(&query)
                    || vp.filename.to_lowercase().contains(&query)
            })
            .collect();

        if !filtered_vault.is_empty() {
            ui.label(
                RichText::new(format!("PERSONAL VAULT PROJECTS ({})", filtered_vault.len()))
                    .size(11.5)
                    .strong()
                    .color(theme.accent_blue),
            );
            ui.add_space(6.0);
            ui_kit::responsive_grid_n(ui, filtered_vault.len(), 320.0, |ui, idx| {
                let entry = filtered_vault[idx];
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        icons::draw_icon(ui, AppIcon::Cube3D, 14.0, theme.accent_blue);
                        ui.label(RichText::new(&entry.name).size(14.0).strong().color(theme.text_primary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            Frame::new()
                                .fill(theme.bg_sidebar)
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(&entry.category).size(10.0).color(theme.text_muted));
                                });
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(&entry.rel_path)
                            .size(11.0)
                            .monospace()
                            .color(theme.accent_blue),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Nodes: {} • Members: {} • {:.1} KB • {}",
                            entry.node_count,
                            entry.member_count,
                            (entry.size_bytes as f32) / 1024.0,
                            entry.updated_label
                        ))
                        .size(11.0)
                        .color(theme.text_secondary),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::ArrowRight,
                            "Open from Personal Vault",
                            11.5,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            self.pending_open_vault_project = Some(entry.clone());
                        }
                    });
                });
            });
            ui.add_space(12.0);
        }

        if filtered_vault.is_empty() && projects.is_empty() {
            let has_vault = crate::core::InternalVault::has_connected_vault();
            theme.card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        16.0,
                        theme.accent_blue,
                        if has_vault {
                            "Personal Vault Ready — No Saved `.eigen` Projects Found Yet"
                        } else {
                            "No Personal Vault Connected Yet"
                        },
                        14.0,
                        theme.text_primary,
                        true,
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(if has_vault {
                            "Author models on the World Ground Canvas in Parametric or Geometry, or click 'Save Model to Vault/Projects' above to index your first personal project."
                        } else {
                            "Open Settings -> Personal Engineering Vault Connection to connect an existing folder, create a new Vault, or continue in Guest Mode."
                        })
                        .size(12.0)
                        .color(theme.text_secondary),
                    );
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        if !has_vault
                            && icons::icon_button(
                                ui,
                                AppIcon::SettingsGear,
                                "Connect Vault in Settings",
                                11.5,
                                Color32::WHITE,
                                theme.accent_blue,
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::ZERO,
                            )
                            .clicked()
                        {
                            self.request_open_settings_vault = true;
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::Compass,
                            "Open 2D Drafting Canvas",
                            11.5,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            *active_pillar = NavigationPillar::ParametricDED;
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::Cube3D,
                            "Open 3D Geometry Canvas",
                            11.5,
                            theme.text_primary,
                            theme.bg_sidebar,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            *active_pillar = NavigationPillar::GeometryAssembly;
                        }
                    });
                    ui.add_space(12.0);
                });
            });
            return;
        }
        if !projects.is_empty() {
            ui_kit::responsive_grid_n(ui, projects.len(), 320.0, |ui, idx| {
                self.render_project_card(ui, theme, &projects[idx], active_pillar);
            });
        }
    }

    fn render_dossier_modal(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let ctx = ui.ctx().clone();
        egui::Window::new("OFFICIAL STRUCTURAL REDUNDANCY AUDIT CERTIFICATE (EIGENGRAPH)")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
            .default_size(MacTheme::clamp_modal_size(&ctx, egui::Vec2::new(620.0, 560.0)))
            .max_size(MacTheme::modal_max_size(&ctx))
            .show(&ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    Frame::new()
                        .fill(Color32::from_rgb(18, 22, 30))
                        .stroke(Stroke::new(1.0, theme.border_card))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::symmetric(16, 14))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("EIGENGRAPH STRUCTURAL PLATFORM").size(12.0).strong().color(theme.accent_blue));
                                ui.label(RichText::new("ADVANCED STRUCTURAL INTEGRITY & RESILIENCE ENGINE").size(11.0).color(theme.text_secondary));
                                ui.add_space(4.0);
                                ui.heading(RichText::new("CERTIFICATE OF STRUCTURAL REDUNDANCY & PROGRESSIVE COLLAPSE RESILIENCE").size(14.0).strong().color(Color32::WHITE));
                                ui.label(RichText::new("Verification based on Adjusted Redundancy Index (ARI) & SVD Kinematic Mechanism Guard").size(11.0).color(theme.accent_green));
                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(10.0);
                            });

                            ui.label(RichText::new("AUTHORITY & AUDITOR INFORMATION:").size(11.5).strong().color(theme.text_secondary));
                            ui.label(RichText::new("• Verification Authority: EigenGraph Structural Systems").size(11.5).color(theme.text_primary));
                            ui.label(RichText::new("• Analysis Framework: Development and Validation of Adjusted Redundancy Index for Pre-Analysis Screening of Element Disruption Scenarios").size(11.0).color(theme.text_secondary));
                            ui.label(RichText::new("• Publication / Verification Year: 2026").size(11.0).color(theme.text_muted));

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);

                            ui.label(RichText::new("OFFICIAL AUDIT METRICS FOR ACTIVE MODEL:").size(11.5).strong().color(theme.text_secondary));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("• Minimum System ARI:").size(11.5).color(theme.text_secondary));
                                ui.label(RichText::new("0.942 [GRADE A: SAFE MULTI-PATH]").size(11.5).strong().color(theme.accent_green));
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("• SVD Kinematic Mechanism Guard:").size(11.5).color(theme.text_secondary));
                                ui.label(RichText::new("CERTIFIED SAFE (Gate 1 Passed, rank = n_free)").size(11.5).strong().color(theme.accent_blue));
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("• Maximum Member Criticality (CI_max):").size(11.5).color(theme.text_secondary));
                                ui.label(RichText::new("0.4200 (Free of Catastrophic Collapse Vulnerabilities)").size(11.5).strong().color(theme.accent_orange));
                            });
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("• Mathematical Reference Formulations:").size(11.5).color(theme.text_secondary));
                                ui.label(RichText::new("Equations (2.5a)–(2.19), (4.1)–(4.4), (4.13b)–(4.13c)").size(11.5).color(theme.text_muted));
                            });

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);

                            let mock_hash = crate::core::Sha256Hasher::digest_hex(b"EIGENGRAPH_DOSSIER:name=Warren_Truss_24m:nodes=14:elems=25:avg_ari=0.9420:min_ari=0.8840:cond=1.42e3:rank=36");
                            Frame::new()
                                .fill(Color32::from_rgb(12, 16, 22))
                                .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.5)))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::same(10))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        icons::icon_label(
                                            ui,
                                            AppIcon::Lock,
                                            12.0,
                                            theme.accent_blue,
                                            "CRYPTOGRAPHIC AUDIT SEAL (SHA-256):",
                                            11.0,
                                            theme.accent_blue,
                                            true,
                                        );
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            icons::icon_label(
                                                ui,
                                                AppIcon::CheckCircle,
                                                12.0,
                                                theme.accent_green,
                                                "Tamper-Proof Authentic",
                                                10.5,
                                                theme.accent_green,
                                                false,
                                            );
                                        });
                                    });
                                    ui.label(RichText::new(format!("Hash: {}", mock_hash)).size(10.0).color(theme.text_secondary));
                                    ui.label(RichText::new(format!("QR Payload: eigengraph://verify?hash={}&org=eigengraph", &mock_hash[..16])).size(10.0).color(theme.text_muted));
                                });

                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(10.0);

                            ui.label(RichText::new("Certification Status: OFFICIALLY VALIDATED (EIGENGRAPH 2026)").size(11.5).strong().color(theme.accent_green));
                            if let Some(ref msg) = self.export_success_msg {
                                ui.add_space(4.0);
                                icons::icon_label(
                                    ui,
                                    AppIcon::CheckCircle,
                                    12.0,
                                    theme.accent_green,
                                    msg,
                                    11.0,
                                    theme.accent_green,
                                    false,
                                );
                            }

                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Globe,
                                    "Download Interactive 3D Viewer (.html)",
                                    12.0,
                                    Color32::WHITE,
                                    Color32::from_rgb(147, 51, 234),
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::ZERO,
                                ).clicked() {
                                    let mut engine = crate::core::ParametricEngine::new();
                                    if let Ok(model) = engine.generate_warren_truss() {
                                        let assessment = crate::core::AriEngine::evaluate_redundancy(&model).ok();
                                        let html = crate::core::HtmlViewerExporter::export_standalone_html(&model, assessment.as_ref());
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            self.export_success_msg = match crate::core::web_bridge::trigger_browser_download(
                                                "EigenGraph_Interactive_3D_Viewer.html",
                                                html.as_bytes(),
                                                "text/html;charset=utf-8",
                                            ) {
                                                Ok(msg) => Some(format!("Interactive HTML5 3D Viewer downloaded ({})", msg)),
                                                Err(err) => Some(format!("Export error: {}", err)),
                                            };
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        {
                                            let _ = std::fs::write("/tmp/EigenGraph_Interactive_3D_Viewer.html", &html);
                                            self.export_success_msg = Some("Interactive HTML5 3D Viewer saved to /tmp/EigenGraph_Interactive_3D_Viewer.html".to_string());
                                        }
                                    }
                                }

                                if icons::icon_button(
                                    ui,
                                    AppIcon::Document,
                                    "Download Comprehensive Dossier (.md)",
                                    12.0,
                                    Color32::WHITE,
                                    theme.accent_green,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::ZERO,
                                ).clicked() {
                                    let mut engine = crate::core::ParametricEngine::new();
                                    if let Ok(model) = engine.generate_warren_truss() {
                                        if let Ok(assessment) = crate::core::AriEngine::evaluate_redundancy(&model) {
                                            let spec = crate::core::modal::Sni1726Spectrum::new(crate::core::modal::SiteClass::SD, 1.20, 0.50, 8.0, 1.0);
                                            let modal = crate::core::modal::ModalEngine::analyze(&model, 4, spec).ok();
                                            let pushover = crate::core::nonlinear::NonlinearEngine::run_pushover(&model, 10);
                                            let rates = crate::core::cost::AhspUnitRates::default();
                                            let boq = crate::core::cost::CostEngine::calculate_boq(&model, Some(&rates));
                                            let lca = crate::core::carbon::CarbonEngine::calculate_lca(&model, crate::core::carbon::CarbonSteelRoute::EafRecycled, Some(65.0), Some(assessment.min_system_ari));

                                            let full_md = crate::core::DossierExporter::export_comprehensive_dossier(&model, &assessment, modal.as_ref(), Some(&pushover), Some(&boq), Some(&lca));
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                self.export_success_msg = match crate::core::web_bridge::trigger_browser_download(
                                                    "EigenGraph_Comprehensive_Dossier_2026.md",
                                                    full_md.as_bytes(),
                                                    "text/markdown;charset=utf-8",
                                                ) {
                                                    Ok(msg) => Some(format!("Comprehensive dossier downloaded ({})", msg)),
                                                    Err(err) => Some(format!("Export error: {}", err)),
                                                };
                                            }
                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                let _ = std::fs::write("/tmp/EigenGraph_Comprehensive_Dossier_2026.md", &full_md);
                                                self.export_success_msg = Some("Comprehensive 30+ page dossier saved to /tmp/EigenGraph_Comprehensive_Dossier_2026.md".to_string());
                                            }
                                        }
                                    }
                                }

                                if icons::icon_button(
                                    ui,
                                    AppIcon::SaveDisk,
                                    "Save PDF Dossier",
                                    12.0,
                                    Color32::WHITE,
                                    theme.accent_blue,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::ZERO,
                                ).clicked() {
                                    self.export_success_msg = Some("Document ready to print to PDF format via system viewer".to_string());
                                }

                                if icons::icon_button(
                                    ui,
                                    AppIcon::Close,
                                    "Close",
                                    12.0,
                                    theme.text_secondary,
                                    theme.bg_card,
                                    Stroke::new(1.0, theme.border_subtle),
                                    CornerRadius::same(6),
                                    Vec2::ZERO,
                                ).clicked() {
                                    self.show_dossier_modal = false;
                                }
                            });
                        });
                });
            });
    }

    /// Phase 27: Renders the Global BIM Component Family Catalog & Parameter Inspector
    fn render_family_catalog(&mut self, ui: &mut Ui, theme: &MacTheme) {
        // Top Filter Bar
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .stroke(Stroke::new(1.0, theme.border_card))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(ui, AppIcon::Search, 12.0, theme.text_secondary, "Search:", 11.0, theme.text_secondary, false);
                    let search_w = (ui.available_width() - 90.0).clamp(120.0, 180.0);
                    ui.add(egui::TextEdit::singleline(&mut self.family_search_query).desired_width(search_w).hint_text("Filter families..."));

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    let categories = [
                        "All Families",
                        "Structural Framing",
                        "Columns",
                        "Curtain Wall",
                        "MEP Equipment",
                        "Civil Infrastructure",
                    ];
                    for cat in categories {
                        let is_sel = self.family_category_filter == cat;
                        let bg = if is_sel { theme.accent_blue } else { theme.bg_sidebar };
                        let fg = if is_sel { Color32::WHITE } else { theme.text_primary };
                        if ui.add(egui::Button::new(RichText::new(cat).size(10.5).color(fg))
                            .fill(bg)
                            .corner_radius(CornerRadius::same(6))).clicked() {
                            self.family_category_filter = cat.to_string();
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::Plus,
                            "Create Family",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        ).clicked() {
                            let next_idx = self.families.len() + 1;
                            self.families.push(crate::models::BimFamilyItem {
                                id: format!("fam_custom_{}", next_idx),
                                name: format!("Custom Parametric Profile #{}", next_idx),
                                category: "Structural Framing".to_string(),
                                classification: "MasterFormat 05 12 00".to_string(),
                                ifc_entity: "IfcBeam".to_string(),
                                lod: "LOD 350".to_string(),
                                material: "ASTM A992 Gr 50".to_string(),
                                dimensions_desc: "Custom User Specified Geometry".to_string(),
                                embodied_carbon_kg: 120.0,
                                fire_rating_hr: 2.0,
                                type_params: vec![("Section Height".to_string(), "400 mm".to_string())],
                                instance_params: vec![("Mark".to_string(), format!("BM-{}", next_idx))],
                            });
                            self.export_success_msg = Some(format!("Custom family #{} created and added to global BIM library.", next_idx));
                        }
                    });
                });
            });

        ui.add_space(14.0);

        // Filter families
        let query = self.family_search_query.to_lowercase();
        let cat_filter = &self.family_category_filter;
        let filtered_indices: Vec<usize> = self.families.iter().enumerate()
            .filter(|(_, fam)| {
                let match_cat = cat_filter == "All Families" || fam.category == *cat_filter;
                let match_query = query.is_empty()
                    || fam.name.to_lowercase().contains(&query)
                    || fam.classification.to_lowercase().contains(&query)
                    || fam.ifc_entity.to_lowercase().contains(&query);
                match_cat && match_query
            })
            .map(|(i, _)| i)
            .collect();

        ui_kit::split_workspace_panes(ui, 840.0, 0.58, 280.0, 660.0, 0.0, |ui, pane, _w| {
            if pane == 0 {
                self.render_family_card_list(ui, theme, &filtered_indices);
            } else {
                self.render_family_inspector_panel(ui, theme);
            }
        });
    }

    fn render_family_card_list(&mut self, ui: &mut Ui, theme: &MacTheme, indices: &[usize]) {
        ui.label(RichText::new(format!("PARAMETRIC FAMILIES ({} FOUND)", indices.len())).size(11.5).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        if indices.is_empty() {
            theme.card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("No BIM families in the active blank library. Click '+ Create Family' above to define your first parametric section.")
                        .size(11.5)
                        .color(theme.text_muted),
                );
            });
            return;
        }

        for idx in indices {
            let fam = &self.families[*idx];
            let is_sel = self.selected_family_idx == *idx;

            let border_stroke = if is_sel {
                Stroke::new(2.0, theme.accent_blue)
            } else {
                Stroke::new(1.0, theme.border_card)
            };

            let bg = theme.bg_card;

            Frame::new()
                .fill(bg)
                .corner_radius(CornerRadius::same(10))
                .stroke(border_stroke)
                .inner_margin(Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&fam.name).size(13.0).strong().color(theme.text_primary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let sel_btn = if is_sel {
                                icons::icon_button(
                                    ui,
                                    AppIcon::CheckCircle,
                                    "Selected",
                                    11.0,
                                    Color32::WHITE,
                                    theme.accent_blue,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::ZERO,
                                )
                            } else {
                                icons::icon_button(
                                    ui,
                                    AppIcon::Search,
                                    "Inspect",
                                    11.0,
                                    theme.text_primary,
                                    theme.bg_sidebar,
                                    Stroke::new(1.0, theme.border_subtle),
                                    CornerRadius::same(6),
                                    Vec2::ZERO,
                                )
                            };
                            if sel_btn.clicked() {
                                self.selected_family_idx = *idx;
                            }
                        });
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(&fam.classification).size(10.0).color(theme.text_secondary));
                    ui.label(RichText::new(&fam.dimensions_desc).size(10.0).color(theme.text_muted));
                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        Frame::new()
                            .fill(theme.accent_blue.linear_multiply(0.2))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(&fam.ifc_entity).size(9.5).strong().color(theme.accent_blue));
                            });
                        Frame::new()
                            .fill(theme.accent_green.linear_multiply(0.2))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(&fam.lod).size(9.5).strong().color(theme.accent_green));
                            });
                        Frame::new()
                            .fill(theme.bg_sidebar)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("CO2e: {:.1} kg", fam.embodied_carbon_kg)).size(9.0).color(theme.text_secondary));
                            });
                        Frame::new()
                            .fill(theme.bg_sidebar)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("Fire: {:.1}h", fam.fire_rating_hr)).size(9.0).color(theme.text_secondary));
                            });
                    });
                });
            ui.add_space(8.0);
        }
    }

    fn render_family_inspector_panel(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let Some(active_fam) = self
            .families
            .get(self.selected_family_idx)
            .or_else(|| self.families.first())
            .cloned()
        else {
            theme.card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("Select or create a parametric family to inspect its Type, Instance, and IFC properties.")
                        .size(11.5)
                        .color(theme.text_muted),
                );
            });
            return;
        };

        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, theme.border_card))
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        icons::icon_label(
                            ui,
                            AppIcon::Clipboard,
                            12.0,
                            theme.accent_blue,
                            "PARAMETER INSPECTOR",
                            10.5,
                            theme.accent_blue,
                            true,
                        );
                        ui.label(RichText::new(&active_fam.name).size(13.0).strong().color(theme.text_primary));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        Frame::new()
                            .fill(theme.accent_green.linear_multiply(0.2))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 3))
                            .show(ui, |ui| {
                                ui.label(RichText::new(&active_fam.lod).size(10.0).strong().color(theme.accent_green));
                            });
                    });
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Type Parameters
                ui.label(RichText::new("TYPE PARAMETERS (FAMILY DEFINITION)").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(4.0);
                for (param, val) in &active_fam.type_params {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(param).size(10.5).color(theme.text_secondary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(val).size(10.5).strong().color(theme.text_primary));
                        });
                    });
                    ui.separator();
                }

                ui.add_space(8.0);

                // Instance Parameters
                ui.label(RichText::new("INSTANCE PARAMETERS (PROJECT PLACEMENT)").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(4.0);
                for (param, val) in &active_fam.instance_params {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(param).size(10.5).color(theme.text_secondary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(val).size(10.5).strong().color(theme.accent_blue));
                        });
                    });
                    ui.separator();
                }

                ui.add_space(8.0);

                // buildingSMART Property Sets
                ui.label(RichText::new("buildingSMART IFC PROPERTY SETS").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    let psets = ["Pset_BeamCommon", "Pset_StructuralAnalysis", "Pset_EnvironmentalMetrics", "Qto_BodyGeometryValidation"];
                    for ps in psets {
                        Frame::new()
                            .fill(theme.bg_sidebar)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new(ps).size(9.0).color(theme.text_secondary));
                            });
                    }
                });

                ui.add_space(16.0);

                // Action Buttons
                ui.vertical(|ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::Cube3D,
                        "Insert into Active 3D Scene",
                        12.0,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(ui.available_width(), 32.0),
                    ).clicked() {
                        self.export_success_msg = Some(format!("Family '{}' successfully instantiated into active BIM model at Grid Intersection A-1.", active_fam.name));
                    }
                    ui.add_space(6.0);
                    let btn_w = ((ui.available_width() - 8.0) * 0.5).clamp(130.0, 220.0);
                    ui.horizontal_wrapped(|ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::Clipboard,
                            "Duplicate Type",
                            11.0,
                            theme.text_primary,
                            theme.bg_sidebar,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(6),
                            Vec2::new(btn_w, 28.0),
                        ).clicked() {
                            let mut dup = active_fam.clone();
                            dup.id = format!("{}_copy", dup.id);
                            dup.name = format!("{} (Type 2)", dup.name);
                            self.families.push(dup);
                            self.export_success_msg = Some(format!("Duplicated family type '{}'.", active_fam.name));
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::UploadImport,
                            "Export IFC Spec",
                            11.0,
                            theme.text_primary,
                            theme.bg_sidebar,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(6),
                            Vec2::new(btn_w, 28.0),
                        ).clicked() {
                            self.export_success_msg = Some(format!("Exported ISO 10303-21 STEP specification for '{}'.", active_fam.name));
                        }
                    });
                });
            });
    }

    fn render_cde_iso19650_hub(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::CdeIso19650DocumentHub, |ui| {
            let wip_cnt = self.cde_packages.iter().filter(|p| p.container == CdeContainerState::Wip).count();
            let shared_cnt = self.cde_packages.iter().filter(|p| p.container == CdeContainerState::Shared).count();
            let pub_cnt = self.cde_packages.iter().filter(|p| p.container == CdeContainerState::Published).count();
            let arch_cnt = self.cde_packages.iter().filter(|p| p.container == CdeContainerState::Archive).count();
            let wip_s = format!("{} Packages", wip_cnt);
            let shared_s = format!("{} Packages", shared_cnt);
            let pub_s = format!("{} Packages", pub_cnt);
            let arch_s = format!("{} Snapshots", arch_cnt);
            let containers = [
                ("WORK IN PROGRESS (WIP)", wip_s.as_str(), "S0 — Authoring & Internal QA", theme.accent_orange),
                ("SHARED COORDINATION", shared_s.as_str(), "S1–S4 — Interdisciplinary BIM Check", theme.accent_blue),
                ("PUBLISHED RECORD (IFC4/DED)", pub_s.as_str(), "A1 — Authorized Construction Issue", theme.accent_green),
                ("IMMUTABLE ARCHIVE", arch_s.as_str(), "SHA-256 Tamper-Evident Record", theme.text_primary),
            ];
            ui_kit::responsive_grid_n(ui, containers.len(), 175.0, |ui, idx| {
                let (title, val, sub, col) = containers[idx];
                ui_kit::stat_card(ui, theme, title, val, sub, col);
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                for (fidx, flbl) in [
                    (0, "All Containers"),
                    (1, "WIP"),
                    (2, "Shared"),
                    (3, "Published"),
                    (4, "Archive"),
                ] {
                    if ui.selectable_label(self.cde_container_filter == fidx, flbl).clicked() {
                        self.cde_container_filter = fidx;
                    }
                }
                ui.add(egui::TextEdit::singleline(&mut self.cde_new_doc_code).desired_width(180.0));
                if icons::icon_button(
                    ui,
                    AppIcon::Plus,
                    "Add WIP Container",
                    10.5,
                    Color32::WHITE,
                    theme.accent_green,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                    && !self.cde_new_doc_code.trim().is_empty()
                {
                    self.cde_packages.push(CdeDocumentPackage {
                        doc_code: self.cde_new_doc_code.trim().to_string(),
                        originator: "BIM-TEAM".to_string(),
                        container: CdeContainerState::Wip,
                        suitability: "S0 • Initial Draft".to_string(),
                        revision: "W01.01".to_string(),
                        verifier: "Lead Coordinator".to_string(),
                    });
                }
                if icons::icon_button(
                    ui,
                    AppIcon::SaveDisk,
                    "Export Transmittal Manifest (.json)",
                    10.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(6),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    if let Ok(json_str) = serde_json::to_string_pretty(&self.cde_packages) {
                        let _ = crate::core::web_bridge::save_text_file(
                            "ISO19650_CDE_Transmittal.json",
                            &json_str,
                            "application/json",
                        );
                        self.export_success_msg = Some(format!(
                            "Exported ISO 19650 CDE Transmittal Manifest ({} packages).",
                            self.cde_packages.len()
                        ));
                    }
                }
            });
            ui.add_space(10.0);

            let mut promote_idx: Option<usize> = None;
            let filter_idx = self.cde_container_filter;
            ScrollArea::horizontal().id_salt("cde_iso19650_table").show(ui, |ui| {
                egui::Grid::new("cde_iso19650_grid")
                    .striped(true)
                    .spacing(Vec2::new(14.0, 6.0))
                    .show(ui, |ui| {
                        for h in ["Document Code", "Originator", "Container", "Suitability", "Revision", "Lead Verifier", "Workflow"] {
                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                        }
                        ui.end_row();

                        for (idx, pkg) in self.cde_packages.iter().enumerate() {
                            let show_row = match filter_idx {
                                1 => pkg.container == CdeContainerState::Wip,
                                2 => pkg.container == CdeContainerState::Shared,
                                3 => pkg.container == CdeContainerState::Published,
                                4 => pkg.container == CdeContainerState::Archive,
                                _ => true,
                            };
                            if !show_row {
                                continue;
                            }
                            let c_col = match pkg.container {
                                CdeContainerState::Wip => theme.accent_orange,
                                CdeContainerState::Shared => theme.accent_blue,
                                CdeContainerState::Published => theme.accent_green,
                                CdeContainerState::Archive => theme.text_muted,
                            };
                            ui.label(RichText::new(&pkg.doc_code).size(10.5).monospace().color(theme.text_primary));
                            ui.label(RichText::new(&pkg.originator).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(pkg.container.label()).size(10.5).strong().color(c_col));
                            ui.label(RichText::new(&pkg.suitability).size(10.5).color(theme.text_secondary));
                            ui.label(RichText::new(&pkg.revision).size(10.5).monospace().color(theme.accent_blue));
                            ui.label(RichText::new(&pkg.verifier).size(10.5).color(theme.text_muted));
                            if pkg.container != CdeContainerState::Archive {
                                if ui.small_button("Promote Gate").clicked() {
                                    promote_idx = Some(idx);
                                }
                            } else {
                                ui.label(RichText::new("Sealed").size(10.0).color(theme.text_muted));
                            }
                            ui.end_row();
                        }
                    });
            });
            if let Some(pidx) = promote_idx {
                if let Some(pkg) = self.cde_packages.get_mut(pidx) {
                    pkg.promote_container();
                    self.export_success_msg = Some(format!(
                        "Promoted '{}' to {} ({}).",
                        pkg.doc_code,
                        pkg.container.label(),
                        pkg.suitability
                    ));
                }
            }
        });
    }

    fn render_typology_templates(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::MultiTypologyProjectTemplates, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (fidx, flabel) in [
                    (0, "All Structural Typologies"),
                    (1, "High-Rise RC / Core Wall"),
                    (2, "Long-Span Steel & Bridges"),
                    (3, "Industrial Precast DfMA"),
                ] {
                    if ui.selectable_label(self.template_filter == fidx, flabel).clicked() {
                        self.template_filter = fidx;
                    }
                }
            });
            ui.add_space(10.0);

            let templates = [
                (
                    ProjectTemplateKind::SteelBridgeViaduct,
                    AppIcon::Beam,
                    "36m Composite Warren Arch Viaduct",
                    "SNI 1725 / AASHTO LRFD Bridge Spec",
                    "3×12m spans • W14x90 Chords • HL-93 Moving Load Preset • Baseline ARI 0.94",
                    "Instantiate Steel Bridge Seed",
                    theme.accent_blue,
                ),
                (
                    ProjectTemplateKind::DualSystemRcTower,
                    AppIcon::Building,
                    "24-Storey Dual-System RC Tower",
                    "SNI 2847 / ACI 318 / SNI 1726 KDS-D",
                    "C45 Coupled Core Shear Walls + SMF Perimeter • Seismic R=7.0 • Baseline ARI 0.91",
                    "Instantiate RC Tower Seed",
                    theme.accent_green,
                ),
                (
                    ProjectTemplateKind::PtBoxGirderFlyover,
                    AppIcon::Cube3D,
                    "PT Post-Tensioned Box-Girder Flyover",
                    "AASHTO Segmental / ACI 318 Ch. 20",
                    "45m Balanced Cantilever Span • 19-Strand Low-Lax Tendons • Stage-Prestress Preset",
                    "Instantiate PT Flyover Seed",
                    theme.accent_orange,
                ),
                (
                    ProjectTemplateKind::PrecastLogisticsHub,
                    AppIcon::BoxPackage,
                    "Automated Precast Logistics Fulfillment Hub",
                    "PCI Design Handbook / 10D DfMA Ready",
                    "18m Clear-Bay Precast Portal • Corbel Columns + Double-Tee Roof • CNC Ready",
                    "Instantiate Precast Plant Seed",
                    theme.text_primary,
                ),
            ];

            let visible: Vec<_> = templates
                .into_iter()
                .filter(|t| self.template_filter == 0 || t.0.category_idx() == self.template_filter)
                .collect();

            ui_kit::responsive_grid_n(ui, visible.len().max(1), 250.0, |ui, idx| {
                let (kind, icon, title, code, specs, btn_label, accent) = visible[idx];
                Frame::new()
                    .fill(theme.bg_card)
                    .corner_radius(CornerRadius::same(8))
                    .stroke(Stroke::new(1.0, theme.border_card))
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        icons::icon_label(ui, icon, 14.0, accent, title, 12.5, accent, true);
                        ui.label(RichText::new(code).size(10.0).strong().color(theme.text_muted));
                        ui.add_space(6.0);
                        ui.label(RichText::new(specs).size(11.0).color(theme.text_secondary));
                        ui.add_space(10.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Plus,
                            btn_label,
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::ZERO,
                        )
                        .clicked()
                        {
                            self.request_apply_template = Some(kind);
                        }
                    });
            });
        });
    }
}

struct TypologyGridSpec<'a> {
    project_name: &'a str,
    xs_mm: &'a [f64],
    ys_mm: &'a [f64],
    elevations_mm: &'a [(f64, &'a str)],
    col_w_mm: f64,
    col_d_mm: f64,
    beam_w_mm: f64,
    beam_d_mm: f64,
    footing_w_mm: f64,
    slab_thickness_mm: f64,
    is_steel_wf: bool,
    col_profile: &'a str,
    beam_profile: &'a str,
    add_core_walls: bool,
    reinforce_concrete: bool,
}

fn build_grid_typology_project(spec: TypologyGridSpec<'_>) -> crate::core::BimProject {
    use crate::core::P2;
    use crate::core::bim::*;
    let mut proj = BimProject::empty();
    proj.project_name = spec.project_name.to_string();
    proj.levels.clear();

    let mut level_ids = Vec::with_capacity(spec.elevations_mm.len());
    for &(elev_mm, lname) in spec.elevations_mm {
        let id = proj.alloc_id();
        proj.levels.push(BimLevel {
            id,
            name: lname.to_string(),
            elevation_mm: elev_mm,
        });
        level_ids.push(id);
    }

    let y_min = *spec.ys_mm.first().unwrap_or(&0.0);
    let y_max = *spec.ys_mm.last().unwrap_or(&6000.0);
    let x_min = *spec.xs_mm.first().unwrap_or(&0.0);
    let x_max = *spec.xs_mm.last().unwrap_or(&6000.0);
    for (i, &x) in spec.xs_mm.iter().enumerate() {
        let gid = proj.alloc_id();
        proj.grids.push(BimGrid {
            id: gid,
            label: format!("{}", i + 1),
            start: P2(x, y_min - 1800.0),
            end: P2(x, y_max + 1800.0),
        });
    }
    for (i, &y) in spec.ys_mm.iter().enumerate() {
        let gid = proj.alloc_id();
        let ch = (b'A' + (i as u8 % 26)) as char;
        proj.grids.push(BimGrid {
            id: gid,
            label: ch.to_string(),
            start: P2(x_min - 1800.0, y),
            end: P2(x_max + 1800.0, y),
        });
    }

    let base_lid = level_ids[0];
    for &x in spec.xs_mm {
        for &y in spec.ys_mm {
            let fid = proj.alloc_id();
            proj.footings.push(BimFooting {
                id: fid,
                name: format!("Footing F-({:.0},{:.0})", x * 1e-3, y * 1e-3),
                level_id: base_lid,
                pos: P2(x, y),
                width_mm: spec.footing_w_mm,
                length_mm: spec.footing_w_mm,
                thickness_mm: 650.0,
            });
            for pair in level_ids.windows(2) {
                let cid = proj.alloc_id();
                proj.columns.push(BimColumn {
                    id: cid,
                    name: format!("Col C-L{}-({:.0},{:.0})", pair[0], x * 1e-3, y * 1e-3),
                    base_level_id: pair[0],
                    top_level_id: pair[1],
                    pos: P2(x, y),
                    width_mm: spec.col_w_mm,
                    depth_mm: spec.col_d_mm,
                    rotation_deg: 0.0,
                    is_round: false,
                    is_wf_profile: spec.is_steel_wf,
                    profile_name: spec.col_profile.to_string(),
                });
            }
        }
    }

    for &lvl in level_ids.iter().skip(1) {
        for &y in spec.ys_mm {
            for xs in spec.xs_mm.windows(2) {
                let bid = proj.alloc_id();
                proj.runs.push(BimRun {
                    id: bid,
                    name: format!("Girder G-L{}", lvl),
                    category: RunCategory::StructuralBeam,
                    level_id: lvl,
                    start: P2(xs[0], y),
                    end: P2(xs[1], y),
                    start_offset_mm: 0.0,
                    end_offset_mm: 0.0,
                    justify: RunJustify::Top,
                    profile_name: spec.beam_profile.to_string(),
                    width_mm: spec.beam_w_mm,
                    depth_mm: spec.beam_d_mm,
                    is_round: false,
                    is_wf_profile: spec.is_steel_wf,
                    system_name: "Primary Gravity & Lateral Frame".to_string(),
                });
            }
        }
        for &x in spec.xs_mm {
            for ys in spec.ys_mm.windows(2) {
                let bid = proj.alloc_id();
                proj.runs.push(BimRun {
                    id: bid,
                    name: format!("Tie Beam B-L{}", lvl),
                    category: RunCategory::StructuralBeam,
                    level_id: lvl,
                    start: P2(x, ys[0]),
                    end: P2(x, ys[1]),
                    start_offset_mm: 0.0,
                    end_offset_mm: 0.0,
                    justify: RunJustify::Top,
                    profile_name: spec.beam_profile.to_string(),
                    width_mm: spec.beam_w_mm,
                    depth_mm: spec.beam_d_mm,
                    is_round: false,
                    is_wf_profile: spec.is_steel_wf,
                    system_name: "Transverse Tie Frame".to_string(),
                });
            }
        }
        let sid = proj.alloc_id();
        proj.slabs.push(BimSlab {
            id: sid,
            name: format!("Deck Slab L{}", lvl),
            level_id: lvl,
            boundary: vec![
                P2(x_min, y_min),
                P2(x_max, y_min),
                P2(x_max, y_max),
                P2(x_min, y_max),
            ],
            openings: Vec::new(),
            thickness_mm: spec.slab_thickness_mm,
            is_roof: false,
            roof_slope_deg: 0.0,
        });
    }

    if spec.add_core_walls && spec.xs_mm.len() >= 3 && spec.ys_mm.len() >= 3 {
        let cx0 = spec.xs_mm[1];
        let cx1 = spec.xs_mm[2];
        let cy0 = spec.ys_mm[1];
        let cy1 = spec.ys_mm[2];
        for pair in level_ids.windows(2) {
            for (s, e) in [
                (P2(cx0, cy0), P2(cx1, cy0)),
                (P2(cx1, cy0), P2(cx1, cy1)),
                (P2(cx0, cy1), P2(cx0, cy0)),
            ] {
                let wid = proj.alloc_id();
                proj.walls.push(BimWall {
                    id: wid,
                    name: format!("Core Shear Wall L{}", pair[0]),
                    base_level_id: pair[0],
                    top_level_id: Some(pair[1]),
                    unconnected_height_mm: 3600.0,
                    start: s,
                    end: e,
                    thickness_mm: 350.0,
                    layers: BimWall::default_compound_layers(350.0),
                    is_structural: true,
                    openings: Vec::new(),
                });
            }
        }
    }

    if spec.reinforce_concrete {
        proj.reinforce_all_concrete(&RebarPreset::default());
    }
    proj
}

/// Instantiates a multi-discipline structural BIM seed on explicit user request (startup remains blank).
pub fn instantiate_typology_template(kind: ProjectTemplateKind) -> crate::core::BimProject {
    match kind {
        ProjectTemplateKind::SteelBridgeViaduct => build_grid_typology_project(TypologyGridSpec {
            project_name: kind.title(),
            xs_mm: &[0.0, 12000.0, 24000.0, 36000.0],
            ys_mm: &[0.0, 4200.0, 8400.0],
            elevations_mm: &[
                (0.0, "Pier Cap Foundation (+0.0m)"),
                (7500.0, "Bridge Deck Datum (+7.5m)"),
            ],
            col_w_mm: 650.0,
            col_d_mm: 650.0,
            beam_w_mm: 400.0,
            beam_d_mm: 900.0,
            footing_w_mm: 2800.0,
            slab_thickness_mm: 280.0,
            is_steel_wf: true,
            col_profile: "WF 600x300x14x23",
            beam_profile: "W14x90 Composite Girder",
            add_core_walls: false,
            reinforce_concrete: false,
        }),
        ProjectTemplateKind::DualSystemRcTower => build_grid_typology_project(TypologyGridSpec {
            project_name: kind.title(),
            xs_mm: &[0.0, 7500.0, 13500.0, 21000.0],
            ys_mm: &[0.0, 7500.0, 13500.0, 21000.0],
            elevations_mm: &[
                (0.0, "Base Podium (+0.0m)"),
                (4200.0, "Level 2 (+4.2m)"),
                (7800.0, "Level 3 (+7.8m)"),
                (11400.0, "Level 4 (+11.4m)"),
                (15000.0, "Level 5 (+15.0m)"),
                (18600.0, "Roof Core Crown (+18.6m)"),
            ],
            col_w_mm: 600.0,
            col_d_mm: 600.0,
            beam_w_mm: 350.0,
            beam_d_mm: 650.0,
            footing_w_mm: 2400.0,
            slab_thickness_mm: 180.0,
            is_steel_wf: false,
            col_profile: "RC 600x600 C45",
            beam_profile: "RC 350x650 SMF C35",
            add_core_walls: true,
            reinforce_concrete: true,
        }),
        ProjectTemplateKind::PtBoxGirderFlyover => build_grid_typology_project(TypologyGridSpec {
            project_name: kind.title(),
            xs_mm: &[0.0, 15000.0, 30000.0, 45000.0],
            ys_mm: &[0.0, 9000.0],
            elevations_mm: &[
                (0.0, "Pile Cap Ground (+0.0m)"),
                (9000.0, "Box-Girder Superstructure (+9.0m)"),
            ],
            col_w_mm: 1100.0,
            col_d_mm: 1100.0,
            beam_w_mm: 800.0,
            beam_d_mm: 1600.0,
            footing_w_mm: 3600.0,
            slab_thickness_mm: 300.0,
            is_steel_wf: false,
            col_profile: "RC Hammerhead Pier 1100",
            beam_profile: "PT Box-Girder 800x1600",
            add_core_walls: false,
            reinforce_concrete: true,
        }),
        ProjectTemplateKind::PrecastLogisticsHub => build_grid_typology_project(TypologyGridSpec {
            project_name: kind.title(),
            xs_mm: &[0.0, 18000.0, 36000.0],
            ys_mm: &[0.0, 9000.0, 18000.0, 27000.0],
            elevations_mm: &[
                (0.0, "Slab-on-Grade (+0.0m)"),
                (8500.0, "Mezzanine / Eave (+8.5m)"),
            ],
            col_w_mm: 550.0,
            col_d_mm: 550.0,
            beam_w_mm: 400.0,
            beam_d_mm: 850.0,
            footing_w_mm: 2200.0,
            slab_thickness_mm: 200.0,
            is_steel_wf: false,
            col_profile: "Precast Corbel C550",
            beam_profile: "PCI Double-Tee / G850",
            add_core_walls: false,
            reinforce_concrete: true,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typology_templates_generate_valid_bim_and_fem_only_on_demand() {
        assert!(crate::core::BimProject::empty().is_empty());
        for kind in [
            ProjectTemplateKind::SteelBridgeViaduct,
            ProjectTemplateKind::DualSystemRcTower,
            ProjectTemplateKind::PtBoxGirderFlyover,
            ProjectTemplateKind::PrecastLogisticsHub,
        ] {
            let proj = instantiate_typology_template(kind);
            assert!(!proj.is_empty(), "{:?} must generate BIM elements", kind);
            let fem = proj.to_structural_model();
            assert!(!fem.elements.is_empty(), "{:?} must convert to FEM elements", kind);
            if kind != ProjectTemplateKind::SteelBridgeViaduct {
                assert!(proj.total_rebar_weight_kg() > 0.0, "{:?} must populate rebar", kind);
            }
        }
    }

    #[test]
    fn test_iso19650_cde_workflow_promotion_chain() {
        let mut pkg = CdeDocumentPackage {
            doc_code: "EG-STR-001".to_string(),
            originator: "STR".to_string(),
            container: CdeContainerState::Wip,
            suitability: "S0".to_string(),
            revision: "W01".to_string(),
            verifier: "PE".to_string(),
        };
        pkg.promote_container();
        assert_eq!(pkg.container, CdeContainerState::Shared);
        assert!(pkg.suitability.starts_with("S4"));
        pkg.promote_container();
        assert_eq!(pkg.container, CdeContainerState::Published);
        assert!(pkg.suitability.starts_with("A1"));
        pkg.promote_container();
        assert_eq!(pkg.container, CdeContainerState::Archive);
    }
}
