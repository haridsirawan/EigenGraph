//! research.rs — Pillar 1D: Research & Concept.
//! The foundational stage involving initial ideas, feasibility studies, regulatory checks, surveying data, and project guidelines.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::models::NavigationPillar;
use crate::theme::MacTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchSubTab {
    ConceptFeasibility,
    RegulatoryCodes,
    SurveyingSite,
    AriFormulations,
}

pub struct ResearchConceptView {
    pub active_subtab: ResearchSubTab,
    pub selected_truss_family: String,
    pub target_design_span_m: f32,
}

impl Default for ResearchConceptView {
    fn default() -> Self {
        Self {
            active_subtab: ResearchSubTab::ConceptFeasibility,
            selected_truss_family: "Warren with Verticals (Family S1)".to_string(),
            target_design_span_m: 24.0,
        }
    }
}

impl ResearchConceptView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, active_pillar: &mut NavigationPillar) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // 1. TOP SUBTABS SELECTOR
            self.render_header_bar(ui, theme);

            ui.add_space(14.0);

            // 2. ACTIVE SUB-TAB CONTENT
            match self.active_subtab {
                ResearchSubTab::ConceptFeasibility => self.render_concept_feasibility(ui, theme, active_pillar),
                ResearchSubTab::RegulatoryCodes => self.render_regulatory_codes(ui, theme),
                ResearchSubTab::SurveyingSite => self.render_surveying_site(ui, theme),
                ResearchSubTab::AriFormulations => self.render_ari_formulations(ui, theme),
            }

            ui.add_space(24.0);
        });
    }

    fn render_header_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let tabs = [
                        (ResearchSubTab::ConceptFeasibility, "💡 Studi Kelayakan & Gagasan Awal"),
                        (ResearchSubTab::RegulatoryCodes, "📜 Regulasi & Standar Desain SNI/AISC"),
                        (ResearchSubTab::SurveyingSite, "🗺️ Data Survei Tapak & Geoteknik"),
                        (ResearchSubTab::AriFormulations, "🔬 Formulasi Resmi ARI EigenGraph"),
                    ];

                    for (tab, label) in tabs {
                        let is_active = self.active_subtab == tab;
                        let bg = if is_active { theme.accent_blue } else { Color32::TRANSPARENT };
                        let fg = if is_active { Color32::WHITE } else { theme.text_secondary };

                        if ui.add(egui::Button::new(RichText::new(label).size(12.0).strong().color(fg))
                            .fill(bg)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(110.0, 28.0))).clicked() {
                            self.active_subtab = tab;
                        }
                    }
                });
            });
    }

    fn render_concept_feasibility(&mut self, ui: &mut Ui, theme: &MacTheme, active_pillar: &mut NavigationPillar) {
        // Hero Card
        Frame::new()
            .fill(theme.accent_blue.gamma_multiply(0.12))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.4)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("💡 TAHAP KONSEPTUAL AWAL (1D BIM)").size(11.0).strong().color(theme.accent_blue));
                        ui.add_space(4.0);
                        ui.label(RichText::new("Studi Kelayakan Teknis, Batasan Regulasi & Panduan Proyek").size(18.0).strong().color(theme.text_primary));
                        ui.add_space(4.0);
                        ui.label(RichText::new("Tahap awal integrasi gagasan mula, batasan bentang bebas sungai, penetapan target redundansi ARI minimal 0.85, serta kesiapan anggaran.").size(12.0).color(theme.text_secondary));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("Mulai Gambar 2D ➔").size(12.0).strong().color(Color32::WHITE))
                            .fill(theme.accent_blue)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(140.0, 32.0))).clicked() {
                            *active_pillar = NavigationPillar::DraftingDrawings;
                        }
                    });
                });
            });

        ui.add_space(16.0);

        // KPI Cards Row
        ui.horizontal(|ui| {
            self.render_stat_card(ui, theme, "KELAYAKAN TEKNIS", "96.5% LAYAK", "Bentang Bebas 24m Tanpa Pilar Tengah", theme.accent_green);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "TARGET REDUNDANSI", "ARI ≥ 0.85", "Kriteria Grade A Bebas Single Point of Failure", theme.accent_blue);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "KELAYAKAN BIAYA", "Rp 3.84 M", "Pagu Anggaran Maks: Rp 4.20 M (Aman)", theme.text_primary);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "WAKTU PELAKSANAAN", "140 Hari", "6 Tahap Ereksi Terverifikasi SVD", theme.text_primary);
        });

        ui.add_space(16.0);

        // Comparative Feasibility Matrix
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("📊 Matriks Evaluasi Alternatif Konsep Awal Struktur").size(13.5).strong().color(theme.text_primary));
                ui.add_space(10.0);

                let rows = [
                    ("Opsi 1: Rangka Busur Warren Baja Merah (Dipilih)", "Bentang 24.0m, Tinggi 3.2m", "ARI = 0.94", "Rp 3.84 M", "⭐ Optimum: Nilai ARI tertinggi, biaya terkendali, estetika ikonik", true),
                    ("Opsi 2: Gelagar Komposit I-Girder Kembar", "Bentang 24.0m, 4 Gelagar", "ARI = 0.62", "Rp 3.65 M", "⚠️ Rawan: Defleksi besar di tengah bentang, redundansi rendah", false),
                    ("Opsi 3: Truss Cantilever Pratt", "Bentang 24.0m, Sub-Truss 12m", "ARI = 0.74", "Rp 4.05 M", "🟡 Cukup: Efisien dalam ereksi namun biaya material lebih tinggi", false),
                ];

                for (name, spec, ari, cost, notes, is_best) in rows {
                    let fill = if is_best { theme.accent_blue.gamma_multiply(0.10) } else { theme.bg_canvas };
                    let stroke = if is_best { Stroke::new(1.5, theme.accent_blue) } else { Stroke::new(1.0, theme.border_subtle) };

                    Frame::new()
                        .fill(fill)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(12))
                        .stroke(stroke)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(name).size(12.5).strong().color(if is_best { theme.accent_blue } else { theme.text_primary }));
                                        if is_best {
                                            ui.label(RichText::new("⭐ REKOMENDASI TERPILIH").size(10.0).strong().color(theme.accent_green));
                                        }
                                    });
                                    ui.add_space(2.0);
                                    ui.label(RichText::new(format!("{} | Catatan: {}", spec, notes)).size(11.0).color(theme.text_secondary));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(ari).size(13.0).strong().color(if is_best { theme.accent_green } else { theme.text_primary }));
                                        ui.label(RichText::new(cost).size(11.0).color(theme.text_muted));
                                    });
                                });
                            });
                        });
                    ui.add_space(8.0);
                }
            });
    }

    fn render_regulatory_codes(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("📜 Verifikasi Kepatuhan Regulasi & Standar Desain Nasional / Internasional").size(13.5).strong().color(theme.text_primary));
                ui.add_space(10.0);

                let codes = [
                    ("SNI 1729:2020", "Spesifikasi untuk Bangunan Gedung Baja Struktural (Adopsi AISC 360-16 LRFD)", "Faktor ketahanan: ϕ_t = 0.90, ϕ_c = 0.90, ϕ_b = 0.90. Terverifikasi aman elastis.", "🟢 Terpenuhi"),
                    ("SNI 2833:2016", "Perencanaan Jembatan terhadap Beban Gempa", "Kategori kepentingan jembatan penting, faktor keutamaan I = 1.20.", "🟢 Terpenuhi"),
                    ("AISC 360-16 Section H1", "Interaction of Flexure and Axial Force for Members under Combined Stresses", "Evaluasi interaksi P-M-V formula H1-1a dan H1-1b. DCR maksimal 0.528.", "🟢 Terpenuhi"),
                    ("DoD UFC 4-023-03", "Design of Buildings to Resist Progressive Collapse (Alternate Path Method)", "Uji disrupsi pelepasan elemen kunci N-1 dan N-2. Rank SVD K_ff penuh.", "🟢 Terpenuhi"),
                    ("Peraturan Menteri PUPR No. 10/2021", "Pedoman Sistem Manajemen Keselamatan Konstruksi (SMKK)", "Integrasi matriks risiko K3L dan SVD Kinematic Mechanism Guard pada tahap rancang bangun.", "🟢 Terpenuhi"),
                ];

                for (code, title, details, status) in codes {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(code).size(12.5).strong().color(theme.accent_blue));
                            ui.label(RichText::new(title).size(11.5).strong().color(theme.text_primary));
                            ui.label(RichText::new(details).size(11.0).color(theme.text_secondary));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(status).size(11.5).strong().color(theme.accent_green));
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_surveying_site(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("🗺️ Data Survei Topografi, Tapak & Geoteknik Pondasi").size(13.5).strong().color(theme.text_primary));
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    self.render_stat_card(ui, theme, "DAYA DUKUNG TANAH (qa)", "285.0 kPa", "Lempung Keras (Hard Clay) N-SPT > 35", theme.text_primary);
                    ui.add_space(8.0);
                    self.render_stat_card(ui, theme, "ZONA GEMPA DASAR", "Kelas Situs D", "Ss = 0.85g, S1 = 0.42g (Wilayah Jawa)", theme.accent_blue);
                    ui.add_space(8.0);
                    self.render_stat_card(ui, theme, "KECEPATAN ANGIN RENCANA", "32.0 m/s", "Tekanan Dinamis q = 0.627 kN/m²", theme.text_primary);
                    ui.add_space(8.0);
                    self.render_stat_card(ui, theme, "KELAS KOROSI TAPAK", "C5-High Marine", "Proteksi Cat Epoxy Sa 2.5 Polyurethane", theme.accent_green);
                });
            });
    }

    fn render_ari_formulations(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔬 Landasan Teori Resmi: Adjusted Redundancy Index (EigenGraph 2026)").size(13.5).strong().color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Lead Engineer: Harids Irawan, S.T., M.T. (EigenGraph Systems)").size(11.0).color(theme.accent_blue));
                    });
                });
                ui.add_space(10.0);

                let formulas = [
                    ("1. Normalisasi Saturasi Hiperbolik (Persamaan 2.5a–2.5b)", "f(k) = k / (k + s_k)   dan   g(R) = R / (R + s_R)", "Mencegah distorsi skala kekakuan dan kapasitas tanpa batas atas."),
                    ("2. Bobot Mekanis Sisi Batang (Persamaan 2.7)", "w_e^M = α_k · f(k_e) + α_R · g(R_e)", "Menggabungkan kontribusi kekakuan elastis dan kapasitas batas penampang."),
                    ("3. Multigraf & K-Lintasan Bebas Sisi (Teorema Menger, Persamaan 2.9)", "S(G^M) = (1/K) · Σ w_e^M", "Menelusuri redundansi topologi lintasan transfer beban alternatif."),
                    ("4. Inti Multiplikatif Adjusted Redundancy Index (Persamaan 2.17)", "ARI_e = RR_e × (1.0 - U_e^*)", "Memadukan rasio redundansi topologis dengan sisa cadangan elastis elemen."),
                    ("5. SVD Kinematic Mechanism Guard (Gate 1, Persamaan 4.1)", "K_ff = U · Σ · V^T   ==>   deteksi σ_min = 0 (Nullspace = mekanisme)", "Menjamin struktur bebas dari mekanisme keruntuhan goyang labil."),
                ];

                for (title, eq, desc) in formulas {
                    Frame::new()
                        .fill(theme.bg_canvas)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(12))
                        .stroke(Stroke::new(1.0, theme.border_subtle))
                        .show(ui, |ui| {
                            ui.label(RichText::new(title).size(12.0).strong().color(theme.text_primary));
                            ui.add_space(4.0);
                            ui.label(RichText::new(eq).size(13.0).strong().color(theme.accent_blue));
                            ui.add_space(2.0);
                            ui.label(RichText::new(desc).size(11.0).color(theme.text_secondary));
                        });
                    ui.add_space(8.0);
                }
            });
    }

    fn render_stat_card(&self, ui: &mut Ui, theme: &MacTheme, title: &str, val: &str, subtitle: &str, val_color: Color32) {
        Frame::new()
            .fill(theme.bg_canvas)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.set_width(175.0);
                ui.label(RichText::new(title).size(9.5).strong().color(theme.text_muted));
                ui.add_space(2.0);
                ui.label(RichText::new(val).size(18.0).strong().color(val_color));
                ui.add_space(2.0);
                ui.label(RichText::new(subtitle).size(10.0).color(theme.text_secondary));
            });
    }
}
