//! my_project.rs — Pillar 1: Mac App Store style Portfolio & Project Hub.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui};
use crate::models::{NavigationPillar, ProjectItem};
use crate::theme::MacTheme;

pub struct MyProjectView {
    pub selected_category: String,
    pub search_query: String,
    pub show_dossier_modal: bool,
}

impl Default for MyProjectView {
    fn default() -> Self {
        Self {
            selected_category: "All Projects".to_string(),
            search_query: String::new(),
            show_dossier_modal: false,
        }
    }
}

impl MyProjectView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, projects: &[ProjectItem], active_pillar: &mut NavigationPillar) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // 1. HERO FEATURED BANNER (Mac App Store Style)
            if let Some(featured) = projects.iter().find(|p| p.is_featured) {
                self.render_hero_banner(ui, theme, featured, active_pillar);
            }

            ui.add_space(24.0);

            // 2. CATEGORY FILTER PILLS
            self.render_category_bar(ui, theme);

            ui.add_space(16.0);

            // 3. CURATED CARD GRID (3 Columns)
            self.render_projects_grid(ui, theme, projects, active_pillar);

            ui.add_space(32.0);
        });

        // 4. UNY ARI DISSERTATION AUDIT DOSSIER MODAL
        if self.show_dossier_modal {
            self.render_dossier_modal(ui, theme);
        }
    }

    fn render_hero_banner(&mut self, ui: &mut Ui, theme: &MacTheme, p: &ProjectItem, active_pillar: &mut NavigationPillar) {
        theme.hero_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        Frame::new()
                            .fill(theme.accent_blue.gamma_multiply(0.2))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                ui.label(RichText::new("FEATURED MASTER PROJECT").size(11.0).strong().color(theme.accent_blue));
                            });

                        ui.add_space(6.0);
                        ui.label(RichText::new(p.status.badge_label()).size(12.0).color(theme.text_secondary));
                    });

                    ui.add_space(8.0);
                    ui.heading(RichText::new(&p.title).size(24.0).strong().color(theme.text_primary));
                    ui.label(RichText::new(&p.description).size(14.0).color(theme.text_secondary));

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("📊 Elements: {} | Nodes: {} | Max Disp: {:.2} mm | Min ARI: {:.2}",
                            p.elements_count, p.nodes_count, p.max_deflection_mm, p.min_ari
                        )).size(12.0).color(theme.text_muted));
                    });

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new("🚀 Open in Build Workspace").strong().color(Color32::WHITE))
                            .fill(theme.accent_blue)
                            .corner_radius(CornerRadius::same(8))
                            ).clicked() {
                            *active_pillar = NavigationPillar::ModelingGeometry;
                        }

                        if ui.add(egui::Button::new(RichText::new("📜 Cetak Sertifikat Audit Disertasi").strong().color(Color32::WHITE))
                            .fill(theme.accent_green)
                            .corner_radius(CornerRadius::same(8))
                            ).clicked() {
                            self.show_dossier_modal = true;
                        }
                    });
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    Frame::new()
                        .fill(Color32::from_rgb(20, 24, 32))
                        .corner_radius(CornerRadius::same(12))
                        .stroke(Stroke::new(1.0, theme.border_card))
                        
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(45.0);
                                ui.label(RichText::new("🌐 3D LIVE PREVIEW").size(14.0).strong().color(theme.accent_blue));
                                ui.label(RichText::new("Interactive Viewport").size(11.0).color(theme.text_muted));
                            });
                        });
                });
            });
        });
    }

    fn render_category_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let categories = ["All Projects", "Recent", "Starred ⭐", "Bridges", "High-Rise", "Space Trusses", "Benchmarks"];
        ui.horizontal(|ui| {
            ui.label(RichText::new("Collections:").strong().size(13.0).color(theme.text_secondary));
            ui.add_space(8.0);
            for cat in categories {
                let is_selected = self.selected_category == cat;
                let bg = if is_selected { theme.accent_blue } else { theme.bg_card };
                let fg = if is_selected { Color32::WHITE } else { theme.text_secondary };

                let btn = egui::Button::new(RichText::new(cat).size(12.0).color(fg))
                    .fill(bg)
                    .corner_radius(CornerRadius::same(16))
                    ;

                if ui.add(btn).clicked() {
                    self.selected_category = cat.to_string();
                }
            }
        });
    }

    fn render_projects_grid(&self, ui: &mut Ui, theme: &MacTheme, projects: &[ProjectItem], active_pillar: &mut NavigationPillar) {
        ui.columns(3, |columns| {
            for (idx, p) in projects.iter().enumerate() {
                let col = &mut columns[idx % 3];
                theme.card_frame().show(col, |ui| {
                    // Card 3D Preview Box
                    Frame::new()
                        .fill(Color32::from_rgb(26, 26, 30))
                        .corner_radius(CornerRadius::same(8))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(35.0);
                                ui.label(RichText::new("📐 3D Wireframe Thumbnail").size(12.0).color(theme.text_muted));
                            });
                        });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&p.category).size(11.0).color(theme.accent_blue).strong());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(p.status.badge_label()).size(11.0).color(theme.text_secondary));
                        });
                    });

                    ui.add_space(4.0);
                    ui.label(RichText::new(&p.title).size(15.0).strong().color(theme.text_primary));
                    ui.label(RichText::new(&p.description).size(12.0).color(theme.text_secondary));

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{} El | {} Nd", p.elements_count, p.nodes_count)).size(11.0).color(theme.text_muted));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(egui::Button::new(RichText::new("Open").size(12.0).color(Color32::WHITE))
                                .fill(theme.accent_blue)
                                .corner_radius(CornerRadius::same(6))).clicked() {
                                *active_pillar = NavigationPillar::ModelingGeometry;
                            }
                        });
                    });
                });
                col.add_space(16.0);
            }
        });
    }

    fn render_dossier_modal(&mut self, ui: &mut Ui, theme: &MacTheme) {
        egui::Window::new("📜 SERTIFIKAT AUDIT REDUNDANSI STRUKTURAL (ARI DISERTASI UNY)")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::new(0.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.set_width(620.0);
                Frame::new()
                    .fill(Color32::from_rgb(18, 22, 30))
                    .stroke(Stroke::new(1.0, theme.border_card))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(20, 16))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("UNIVERSITAS NEGERI YOGYAKARTA").size(12.0).strong().color(theme.accent_blue));
                            ui.label(RichText::new("FAKULTAS TEKNIK — PROGRAM DOKTOR ILMU TEKNIK (S3)").size(11.0).color(theme.text_secondary));
                            ui.add_space(4.0);
                            ui.heading(RichText::new("SURAT HASIL AUDIT REDUNDANSI STRUKTURAL & KETAHANAN RUNTUH PROGRESIF").size(14.0).strong().color(Color32::WHITE));
                            ui.label(RichText::new("Verifikasi Berbasis Adjusted Redundancy Index (ARI) & SVD Kinematic Mechanism Guard").size(11.0).color(theme.accent_green));
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);
                        });

                        ui.label(RichText::new("INFORMASI RISET & PENELITI:").size(11.5).strong().color(theme.text_secondary));
                        ui.label(RichText::new("• Nama Peneliti: Harids Irawan, S.T., M.T. (NIM: 23051690001)").size(11.5).color(theme.text_primary));
                        ui.label(RichText::new("• Judul Disertasi: Pengembangan dan Validasi Adjusted Redundancy Index untuk Penapisan Pra-Analisis Skenario Gangguan Elemen pada Struktur Linear-Elastik").size(11.0).color(theme.text_secondary));
                        ui.label(RichText::new("• Tahun Publikasi / Sidang: 2026").size(11.0).color(theme.text_muted));

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.label(RichText::new("HASIL AUDIT METRIK RESMI MODEL AKTIF:").size(11.5).strong().color(theme.text_secondary));
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("• Minimum System ARI:").size(11.5).color(theme.text_secondary));
                            ui.label(RichText::new("0.942 [GRADE A: AMAN MULTI-PATH]").size(11.5).strong().color(theme.accent_green));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("• SVD Kinematic Mechanism Guard:").size(11.5).color(theme.text_secondary));
                            ui.label(RichText::new("CERTIFIED SAFE (Gate 1 Passed, rank = n_free)").size(11.5).strong().color(theme.accent_blue));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("• Maximum Member Criticality (CI_max):").size(11.5).color(theme.text_secondary));
                            ui.label(RichText::new("0.4200 (Bebas Titik Rawan Runtuh Katastropik)").size(11.5).strong().color(theme.accent_orange));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("• Formulasi Acuan Matematis:").size(11.5).color(theme.text_secondary));
                            ui.label(RichText::new("Persamaan (2.5a)–(2.19), (4.1)–(4.4), (4.13b)–(4.13c)").size(11.5).color(theme.text_muted));
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Status Sertifikasi: RESMI TERVALIDASI (UNY 2026)").size(11.5).strong().color(theme.accent_green));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(egui::Button::new(RichText::new("✕ Tutup").size(12.0).color(theme.text_secondary))
                                    .fill(theme.bg_card)
                                    .corner_radius(CornerRadius::same(6))).clicked() {
                                    self.show_dossier_modal = false;
                                }

                                if ui.add(egui::Button::new(RichText::new("📄 Unduh Naskah Dossier (.md)").size(12.0).strong().color(Color32::WHITE))
                                    .fill(theme.accent_green)
                                    .corner_radius(CornerRadius::same(6))).clicked() {
                                    self.show_dossier_modal = false;
                                }

                                if ui.add(egui::Button::new(RichText::new("💾 Simpan PDF Dossier").size(12.0).strong().color(Color32::WHITE))
                                    .fill(theme.accent_blue)
                                    .corner_radius(CornerRadius::same(6))).clicked() {
                                    self.show_dossier_modal = false;
                                }
                            });
                        });
                    });
            });
    }
}
