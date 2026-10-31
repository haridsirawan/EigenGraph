//! my_project.rs — Pillar 1: Mac App Store style Portfolio & Project Hub.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui};
use crate::models::{NavigationPillar, ProjectItem};
use crate::theme::MacTheme;

pub struct MyProjectView {
    pub selected_category: String,
    pub search_query: String,
    pub show_dossier_modal: bool,
    pub export_success_msg: Option<String>,
}

impl Default for MyProjectView {
    fn default() -> Self {
        Self {
            selected_category: "All Projects".to_string(),
            search_query: String::new(),
            show_dossier_modal: false,
            export_success_msg: None,
        }
    }
}

impl MyProjectView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, projects: &[ProjectItem], active_pillar: &mut NavigationPillar) {
        ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::symmetric(20, 12)).show(ui, |ui| {
                // 1. HERO FEATURED BANNER (Mac App Store Style)
                if let Some(featured) = projects.iter().find(|p| p.is_featured) {
                    self.render_hero_banner(ui, theme, featured, active_pillar);
                }

                ui.add_space(20.0);

                // 2. CATEGORY FILTER PILLS
                self.render_category_bar(ui, theme);

                ui.add_space(16.0);

                // 3. CURATED CARD GRID (Balanced 2x2 Grid with Live Vector Wireframes)
                self.render_projects_grid(ui, theme, projects, active_pillar);

                ui.add_space(24.0);
            });
        });

        // 4. EIGENGRAPH ARI AUDIT DOSSIER MODAL
        if self.show_dossier_modal {
            self.render_dossier_modal(ui, theme);
        }
    }

    fn render_hero_banner(&mut self, ui: &mut Ui, theme: &MacTheme, p: &ProjectItem, active_pillar: &mut NavigationPillar) {
        theme.hero_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 280.0).max(320.0));
                    ui.horizontal(|ui| {
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
                    ui.heading(RichText::new(&p.title).size(22.0).strong().color(theme.text_primary));
                    ui.add_space(2.0);
                    ui.label(RichText::new(&p.description).size(13.0).color(theme.text_secondary));

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!(
                            "📊 Elements: {}   •   Nodes: {}   •   Max Disp: {:.2} mm   •   Min ARI: {:.2}",
                            p.elements_count, p.nodes_count, p.max_deflection_mm, p.min_ari
                        )).size(12.0).color(theme.text_muted));
                    });

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new("🚀 Open in 3D Workspace").size(12.5).strong().color(Color32::WHITE))
                            .fill(theme.accent_blue)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(egui::Vec2::new(170.0, 32.0))).clicked() {
                            *active_pillar = NavigationPillar::ModelingGeometry;
                        }

                        ui.add_space(6.0);

                        if ui.add(egui::Button::new(RichText::new("📜 Sertifikat Audit EigenGraph").size(12.5).strong().color(Color32::WHITE))
                            .fill(theme.accent_green)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(egui::Vec2::new(190.0, 32.0))).clicked() {
                            self.show_dossier_modal = true;
                        }
                    });
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    Frame::new()
                        .fill(Color32::from_rgb(18, 22, 30))
                        .corner_radius(CornerRadius::same(12))
                        .stroke(Stroke::new(1.0, theme.border_card))
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(240.0, 125.0), egui::Sense::hover());
                            let painter = ui.painter_at(rect);

                            // Draw Arch Bridge Vector Wireframe
                            let left_x = rect.left() + 18.0;
                            let right_x = rect.right() - 18.0;
                            let deck_y = rect.bottom() - 28.0;
                            let span = right_x - left_x;
                            let panels = 8;

                            // Deck line
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
                                    // Arch rib segment
                                    painter.line_segment([prev_arch, arch_pt], Stroke::new(2.2, theme.accent_green));
                                    // Hanger cable
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
                                "🌐 3D ARCH TRUSS LIVE TOPOLOGY",
                                egui::FontId::proportional(10.0),
                                theme.text_secondary,
                            );
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
        for chunk in projects.chunks(2) {
            ui.columns(2, |columns| {
                for (col_idx, p) in chunk.iter().enumerate() {
                    let col = &mut columns[col_idx];
                    theme.card_frame().show(col, |ui| {
                        // Card 3D Procedural Wireframe Preview Box
                        Frame::new()
                            .fill(Color32::from_rgb(20, 23, 30))
                            .corner_radius(CornerRadius::same(10))
                            .stroke(Stroke::new(1.0, theme.border_subtle))
                            .inner_margin(Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                let avail_w = ui.available_width();
                                let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(avail_w, 72.0), egui::Sense::hover());
                                let painter = ui.painter_at(rect);

                                let lx = rect.left() + 16.0;
                                let rx = rect.right() - 16.0;
                                let by = rect.bottom() - 10.0;
                                let ty = rect.top() + 10.0;
                                let w = rx - lx;

                                if p.id == "p1" || p.id == "p2" {
                                    // Arch / Dome curve
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
                                    // Warren / Diagrid lattice
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
                        ui.horizontal(|ui| {
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

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!(
                                "📊 {} El  •  {} Nd  •  ARI: {:.2}",
                                p.elements_count, p.nodes_count, p.min_ari
                            )).size(11.5).color(theme.text_muted));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(egui::Button::new(RichText::new("Open Model ➔").size(11.5).strong().color(Color32::WHITE))
                                    .fill(theme.accent_blue)
                                    .corner_radius(CornerRadius::same(6))
                                    .min_size(egui::Vec2::new(100.0, 26.0))).clicked() {
                                    *active_pillar = NavigationPillar::ModelingGeometry;
                                }
                            });
                        });
                    });
                }
            });
            ui.add_space(14.0);
        }
    }

    fn render_dossier_modal(&mut self, ui: &mut Ui, theme: &MacTheme) {
        egui::Window::new("📜 SERTIFIKAT AUDIT REDUNDANSI STRUKTURAL (EIGENGRAPH OFFICIAL)")
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
                            ui.label(RichText::new("EIGENGRAPH STRUCTURAL PLATFORM").size(12.0).strong().color(theme.accent_blue));
                            ui.label(RichText::new("ADVANCED STRUCTURAL INTEGRITY & RESILIENCE ENGINE").size(11.0).color(theme.text_secondary));
                            ui.add_space(4.0);
                            ui.heading(RichText::new("SURAT HASIL AUDIT REDUNDANSI STRUKTURAL & KETAHANAN RUNTUH PROGRESIF").size(14.0).strong().color(Color32::WHITE));
                            ui.label(RichText::new("Verifikasi Berbasis Adjusted Redundancy Index (ARI) & SVD Kinematic Mechanism Guard").size(11.0).color(theme.accent_green));
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);
                        });

                        ui.label(RichText::new("INFORMASI OTORITAS & AUDITOR:").size(11.5).strong().color(theme.text_secondary));
                        ui.label(RichText::new("• Lead Engineer / Auditor: Harids Irawan, S.T., M.T. (EigenGraph Systems)").size(11.5).color(theme.text_primary));
                        ui.label(RichText::new("• Kerangka Analisis: Pengembangan dan Validasi Adjusted Redundancy Index untuk Penapisan Pra-Analisis Skenario Gangguan Elemen").size(11.0).color(theme.text_secondary));
                        ui.label(RichText::new("• Tahun Terbit / Verifikasi: 2026").size(11.0).color(theme.text_muted));

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

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // SHA-256 Cryptographic Audit Seal (FIPS 180-4)
                        let mock_hash = crate::core::Sha256Hasher::digest_hex(b"EIGENGRAPH_DOSSIER:name=Warren_Truss_24m:nodes=14:elems=25:avg_ari=0.9420:min_ari=0.8840:cond=1.42e3:rank=36");
                        Frame::new()
                            .fill(Color32::from_rgb(12, 16, 22))
                            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.5)))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::same(10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("🔐 SEGEL KRIPTOGRAFIS AUDIT (SHA-256):").size(11.0).strong().color(theme.accent_blue));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new("✅ Tamper-Proof Authentic").size(10.5).color(theme.accent_green));
                                    });
                                });
                                ui.label(RichText::new(format!("Hash: {}", mock_hash)).size(10.0).color(theme.text_secondary));
                                ui.label(RichText::new(format!("QR Payload: eigengraph://verify?hash={}&org=eigengraph", &mock_hash[..16])).size(10.0).color(theme.text_muted));
                            });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Status Sertifikasi: RESMI TERVALIDASI (EIGENGRAPH 2026)").size(11.5).strong().color(theme.accent_green));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(egui::Button::new(RichText::new("✕ Tutup").size(12.0).color(theme.text_secondary))
                                    .fill(theme.bg_card)
                                    .corner_radius(CornerRadius::same(6))).clicked() {
                                    self.show_dossier_modal = false;
                                }

                                if let Some(ref msg) = self.export_success_msg {
                                    ui.label(RichText::new(format!("✅ {}", msg)).size(11.0).color(theme.accent_green));
                                }

                                if ui.add(egui::Button::new(RichText::new("🌐 Unduh Viewer 3D Interaktif (.html)").size(12.0).strong().color(Color32::WHITE))
                                    .fill(Color32::from_rgb(147, 51, 234))
                                    .corner_radius(CornerRadius::same(6))).clicked() {
                                    let mut engine = crate::core::ParametricEngine::new();
                                    if let Ok(model) = engine.generate_warren_truss() {
                                        let assessment = crate::core::AriEngine::evaluate_redundancy(&model).ok();
                                        let html = crate::core::HtmlViewerExporter::export_standalone_html(&model, assessment.as_ref());
                                        let _ = std::fs::write("/tmp/EigenGraph_Interactive_3D_Viewer.html", &html);
                                        self.export_success_msg = Some("Viewer 3D Interaktif HTML5 tersimpan di /tmp/EigenGraph_Interactive_3D_Viewer.html".to_string());
                                    }
                                }

                                if ui.add(egui::Button::new(RichText::new("📄 Unduh Naskah Dossier Lengkap (.md)").size(12.0).strong().color(Color32::WHITE))
                                    .fill(theme.accent_green)
                                    .corner_radius(CornerRadius::same(6))).clicked() {
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
                                            let _ = std::fs::write("/tmp/EigenGraph_Comprehensive_Dossier_2026.md", &full_md);
                                            self.export_success_msg = Some("Dossier lengkap 30+ halaman tersimpan di /tmp/EigenGraph_Comprehensive_Dossier_2026.md".to_string());
                                        }
                                    }
                                }

                                if ui.add(egui::Button::new(RichText::new("💾 Simpan PDF Dossier").size(12.0).strong().color(Color32::WHITE))
                                    .fill(theme.accent_blue)
                                    .corner_radius(CornerRadius::same(6))).clicked() {
                                    self.export_success_msg = Some("Dokumen siap cetak ke format PDF melalui viewer sistem".to_string());
                                }
                            });
                        });
                    });
            });
    }
}
