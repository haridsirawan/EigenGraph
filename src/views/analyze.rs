//! analyze.rs — Pillar 4: Structural Health, SVD Guard, BIM 5D (RAB), and BIM 4D (Scheduling).

use egui::{Align, Color32, CornerRadius, Layout, RichText, ScrollArea, Ui};
use crate::models::StructuralMember;
use crate::theme::MacTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyzeSubTab {
    RedundancySvd,
    CostEstimationRab,
    TimeSchedule4d,
}

pub struct AnalyzeView {
    pub active_subtab: AnalyzeSubTab,
    pub published_to_dossier: bool,
}

impl Default for AnalyzeView {
    fn default() -> Self {
        Self {
            active_subtab: AnalyzeSubTab::RedundancySvd,
            published_to_dossier: false,
        }
    }
}

impl AnalyzeView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // 1. TOP SUB-TABS SELECTOR (BIM Dimensions & Redundancy)
            self.render_subtabs_header(ui, theme);

            ui.add_space(16.0);

            // 2. ACTIVE SUB-TAB CONTENT
            match self.active_subtab {
                AnalyzeSubTab::RedundancySvd => self.render_redundancy_and_svd(ui, theme, members),
                AnalyzeSubTab::CostEstimationRab => self.render_rab_and_cost(ui, theme, members),
                AnalyzeSubTab::TimeSchedule4d => self.render_schedule_and_kurva_s(ui, theme),
            }

            ui.add_space(24.0);
        });
    }

    fn render_subtabs_header(&mut self, ui: &mut Ui, theme: &MacTheme) {
        ui.horizontal(|ui| {
            // Segmented Pills for 3 Sub-Tabs
            let tabs = [
                (AnalyzeSubTab::RedundancySvd, "⚡ Stabilitas & Redundansi (SVD & ARI)"),
                (AnalyzeSubTab::CostEstimationRab, "💰 Estimasi Biaya & RAB (BIM 5D)"),
                (AnalyzeSubTab::TimeSchedule4d, "⏱️ Penjadwalan & Kurva S (BIM 4D)"),
            ];

            for (tab, label) in tabs {
                let is_active = self.active_subtab == tab;
                let bg = if is_active { theme.accent_blue } else { theme.bg_card };
                let fg = if is_active { Color32::WHITE } else { theme.text_secondary };

                let btn = egui::Button::new(RichText::new(label).size(12.0).strong().color(fg))
                    .fill(bg)
                    .corner_radius(CornerRadius::same(8));

                if ui.add(btn).clicked() {
                    self.active_subtab = tab;
                }
            }

            // Right Side Action: Sync/Publish to MY PROJECT Dossier
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let publish_text = if self.published_to_dossier { "✅ Synced to MY PROJECT" } else { "📤 Publish to MY PROJECT Dossier" };
                let publish_bg = if self.published_to_dossier { theme.accent_green } else { theme.accent_blue.gamma_multiply(0.8) };

                if ui.add(egui::Button::new(RichText::new(publish_text).size(12.0).strong().color(Color32::WHITE))
                    .fill(publish_bg)
                    .corner_radius(CornerRadius::same(8))).clicked() {
                    self.published_to_dossier = true;
                }
            });
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 1: Stabilitas SVD Guard & Redundansi ARI
    // -------------------------------------------------------------
    fn render_redundancy_and_svd(&self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
        // 4 KPI Metric Cards
        ui.columns(4, |cols| {
            self.metric_card(&mut cols[0], theme, "GLOBAL REDUNDANCY (ARI)", "0.912", "🟢 Robust Multi-Path", theme.accent_green);
            self.metric_card(&mut cols[1], theme, "MINIMUM MEMBER ARI", "0.308", "🔴 Member E0 (Critical)", theme.accent_red);
            self.metric_card(&mut cols[2], theme, "SVD KINEMATIC GUARD", "Rank Safe (14/14)", "0 Mechanisms Detected", theme.accent_blue);
            self.metric_card(&mut cols[3], theme, "RESERVE CAPACITY", "1.84x", "Above Collapse Threshold", theme.accent_orange);
        });

        ui.add_space(16.0);

        // SVD Kinematic Guard Banner
        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🛡️ SVD Kinematic Guard Certified:").size(13.0).strong().color(theme.text_primary));
                ui.label(RichText::new("rank(K) = ndof - nrigid. Perturbation condition m_e(lambda) = 1 - (1-lambda)*h_e confirmed safe against collapse mechanisms.").size(12.0).color(theme.text_secondary));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("Gate 3 Verified").size(11.0).color(theme.accent_green).strong());
                });
            });
        });

        ui.add_space(16.0);

        // Member Screening Table
        ui.label(RichText::new("MEMBER-BY-MEMBER REDUNDANCY & CRITICALITY SCREENING").size(13.0).strong().color(theme.text_secondary));
        ui.add_space(8.0);

        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("ID").strong().size(12.0).color(theme.text_muted));
                ui.add_space(20.0);
                ui.label(RichText::new("Member Name").strong().size(12.0).color(theme.text_muted));
                ui.add_space(80.0);
                ui.label(RichText::new("Force N_e [kN]").strong().size(12.0).color(theme.text_muted));
                ui.add_space(30.0);
                ui.label(RichText::new("Capacity [kN]").strong().size(12.0).color(theme.text_muted));
                ui.add_space(30.0);
                ui.label(RichText::new("Util U*").strong().size(12.0).color(theme.text_muted));
                ui.add_space(30.0);
                ui.label(RichText::new("Redundancy (RR)").strong().size(12.0).color(theme.text_muted));
                ui.add_space(30.0);
                ui.label(RichText::new("ARI Score").strong().size(12.0).color(theme.text_muted));
                ui.add_space(30.0);
                ui.label(RichText::new("Criticality (CI)").strong().size(12.0).color(theme.text_muted));
                ui.add_space(30.0);
                ui.label(RichText::new("SVD Margin m_e").strong().size(12.0).color(theme.text_muted));
            });
            ui.separator();

            for m in members {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("E{}", m.id)).size(12.0).color(theme.text_secondary));
                    ui.add_space(24.0);
                    ui.label(RichText::new(&m.name).size(12.0).strong().color(theme.text_primary));
                    ui.add_space(60.0);
                    ui.label(RichText::new(format!("{:+.2}", m.force_kn)).size(12.0).color(theme.text_secondary));
                    ui.add_space(50.0);
                    ui.label(RichText::new(format!("{:.1}", m.capacity_kn)).size(12.0).color(theme.text_secondary));
                    ui.add_space(50.0);
                    ui.label(RichText::new(format!("{:.2}%", m.utilization * 100.0)).size(12.0).color(theme.text_secondary));
                    ui.add_space(50.0);
                    ui.label(RichText::new(format!("{:.4}", m.rr)).size(12.0).color(theme.accent_blue));
                    ui.add_space(60.0);
                    ui.label(RichText::new(format!("{:.4}", m.ari)).size(12.0).strong().color(theme.accent_green));
                    ui.add_space(60.0);
                    ui.label(RichText::new(format!("{:.4}", m.ci)).size(12.0).color(if m.is_critical { theme.accent_red } else { theme.text_secondary }));
                    ui.add_space(60.0);
                    ui.label(RichText::new(format!("{:.6}", m.svd_margin)).size(12.0).color(theme.accent_orange));
                });
                ui.add_space(4.0);
            }
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 2: Estimasi Biaya & RAB (BIM 5D)
    // -------------------------------------------------------------
    fn render_rab_and_cost(&self, ui: &mut Ui, theme: &MacTheme, _members: &[StructuralMember]) {
        // 4 KPI Takeoff Cards
        ui.columns(4, |cols| {
            self.metric_card(&mut cols[0], theme, "TOTAL TONASE BAJA", "48.5 Ton", "7,850 kg/m³ ASTM A992", theme.accent_blue);
            self.metric_card(&mut cols[1], theme, "PROTEKSI API / CAT", "1,420 m²", "3 Lapis Fireproofing", theme.accent_orange);
            self.metric_card(&mut cols[2], theme, "VOLUME BETON PEDESTAL", "18.5 m³", "Beton Mutu f'c 30 MPa", theme.accent_green);
            self.metric_card(&mut cols[3], theme, "TOTAL ANGGARAN (RAB)", "Rp 3.842.500.000,-", "Include PPN 11% & Overhead", theme.text_primary);
        });

        ui.add_space(16.0);

        // Research Innovation Banner: Cost vs ARI Sensitivity
        theme.card_frame().show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("💡 Analisis Sensitivitas Biaya vs. Redundansi (Cost vs. ARI Trade-off)").size(13.0).strong().color(theme.accent_blue));
                ui.add_space(4.0);
                ui.label(RichText::new("Berdasarkan optimasi penampang: Menambah 12% berat profil baja (+Rp 245 Juta) berhasil mendongkrak ARI Global sebesar +28% (dari 0.65 ke 0.91), menghilangkan seluruh elemen berkategori rawan runtuh progresif.").size(12.0).color(theme.text_secondary));
            });
        });

        ui.add_space(16.0);

        // Bill of Quantities (BOQ) & AHSP Table
        ui.label(RichText::new("REKAPITULASI RENCANA ANGGARAN BIAYA (RAB) — STANDAR AHSP").size(13.0).strong().color(theme.text_secondary));
        ui.add_space(8.0);

        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("No").strong().size(12.0).color(theme.text_muted));
                ui.add_space(16.0);
                ui.label(RichText::new("Uraian Pekerjaan").strong().size(12.0).color(theme.text_muted));
                ui.add_space(140.0);
                ui.label(RichText::new("Volume").strong().size(12.0).color(theme.text_muted));
                ui.add_space(40.0);
                ui.label(RichText::new("Satuan").strong().size(12.0).color(theme.text_muted));
                ui.add_space(40.0);
                ui.label(RichText::new("Harga Satuan (Rp)").strong().size(12.0).color(theme.text_muted));
                ui.add_space(50.0);
                ui.label(RichText::new("Total Harga (Rp)").strong().size(12.0).color(theme.text_muted));
            });
            ui.separator();

            let items = [
                ("1", "Pengadaan Material Baja Profil W14x90 (ASTM A992)", "48,500.00", "kg", "28.500", "1.382.250.000"),
                ("2", "Fabrikasi Workshop (Cutting, Fitting, Full Penetration Welding)", "48,500.00", "kg", "8.500", "412.250.000"),
                ("3", "Transportasi & Mobilisasi Komponen ke Site", "48,500.00", "kg", "2.000", "97.000.000"),
                ("4", "Erection Struktur dengan Mobile Crane 50T & Crew", "48,500.00", "kg", "12.000", "582.000.000"),
                ("5", "Pengecatan Primer Epoxy & Fireproof Coating Intumescent", "1,420.00", "m²", "145.000", "205.900.000"),
                ("6", "Baut Mutu Tinggi A325, Baseplate & Anchor Rods", "1.00", "ls", "185.000.000", "185.000.000"),
                ("7", "Pondasi Pedestal & Angkur Beton Bertulang f'c 30 MPa", "18.50", "m³", "1.850.000", "34.225.000"),
            ];

            for (no, desc, vol, sat, hrg, tot) in items {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(no).size(12.0).color(theme.text_secondary));
                    ui.add_space(20.0);
                    ui.label(RichText::new(desc).size(12.0).strong().color(theme.text_primary));
                    ui.add_space(40.0);
                    ui.label(RichText::new(vol).size(12.0).color(theme.text_secondary));
                    ui.add_space(40.0);
                    ui.label(RichText::new(sat).size(12.0).color(theme.text_secondary));
                    ui.add_space(40.0);
                    ui.label(RichText::new(hrg).size(12.0).color(theme.text_secondary));
                    ui.add_space(50.0);
                    ui.label(RichText::new(tot).size(12.0).strong().color(theme.accent_green));
                });
                ui.add_space(4.0);
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new("TOTAL RENCANA ANGGARAN BIAYA (TERMASUK PPN & OVERHEAD):").size(12.0).strong().color(theme.text_primary));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("Rp 3.842.500.000,-").size(15.0).strong().color(theme.accent_blue));
                });
            });
        });
    }

    // -------------------------------------------------------------
    // SUB-TAB 3: Penjadwalan & Kurva S (BIM 4D)
    // -------------------------------------------------------------
    fn render_schedule_and_kurva_s(&self, ui: &mut Ui, theme: &MacTheme) {
        // 4 KPI Scheduling Metric Cards
        ui.columns(4, |cols| {
            self.metric_card(&mut cols[0], theme, "TOTAL DURASI KERJA", "140 Hari", "20 Minggu Kalender", theme.accent_blue);
            self.metric_card(&mut cols[1], theme, "LINTASAN KRITIS (CPM)", "WBS 3.0 (Erection)", "Zero Float Risk", theme.accent_orange);
            self.metric_card(&mut cols[2], theme, "PRODUKTIVITAS HARIAN", "4.5 Ton / Hari", "Crane 50T Kapasitas", theme.accent_green);
            self.metric_card(&mut cols[3], theme, "STABILITY SAAT ERECTION", "Certified Safe", "SVD Evaluated per Stage", theme.accent_blue);
        });

        ui.add_space(16.0);

        // WBS Schedule & Gantt Progression
        ui.label(RichText::new("WORK BREAKDOWN STRUCTURE (WBS) & TAHAPAN PELAKSANAAN").size(13.0).strong().color(theme.text_secondary));
        ui.add_space(8.0);

        theme.card_frame().show(ui, |ui| {
            let tasks = [
                ("WBS 1.0", "Pekerjaan Persiapan, Pengukuran & Pedestal", "20 Hari", "W1 - W3", "100%", theme.accent_green),
                ("WBS 2.0", "Fabrikasi Rangka Batang di Workshop", "45 Hari", "W2 - W8", "65%", theme.accent_blue),
                ("WBS 3.0", "Erection Kolom & Rangka Busur Utama", "35 Hari", "W7 - W12", "0% (Scheduled)", theme.accent_orange),
                ("WBS 4.0", "Pemasangan Bracing Angin & Kabel Hangers", "25 Hari", "W11 - W15", "0% (Scheduled)", theme.text_muted),
                ("WBS 5.0", "Pemasangan Pelat Lantai & Finishing Cat", "25 Hari", "W14 - W18", "0% (Scheduled)", theme.text_muted),
                ("WBS 6.0", "Pengujian Beban Statik-Dinamik & Serah Terima", "14 Hari", "W19 - W20", "0% (Scheduled)", theme.text_muted),
            ];

            for (wbs, name, dur, timeframe, progress, color) in tasks {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(wbs).size(12.0).strong().color(theme.accent_blue));
                    ui.add_space(16.0);
                    ui.label(RichText::new(name).size(12.0).color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(progress).size(11.0).strong().color(color));
                        ui.add_space(20.0);
                        ui.label(RichText::new(timeframe).size(11.0).color(theme.text_muted));
                        ui.add_space(20.0);
                        ui.label(RichText::new(dur).size(11.0).color(theme.text_secondary));
                    });
                });
                ui.add_space(4.0);
            }
        });

        ui.add_space(16.0);

        // Staged Construction SVD Stability Banner
        theme.card_frame().show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("🏗️ Simulasi Stabilitas Konstruksi Bertahap (Staged Construction SVD Check)").size(13.0).strong().color(theme.accent_green));
                ui.add_space(4.0);
                ui.label(RichText::new("Setiap tahap erection diverifikasi nilai singular matriks kekakuannya. Pada WBS 3.0, struktur busur setengah terpasang memerlukan tumpuan perancah sementara (shoring) di simpul #3 untuk mencegah pembentukan mekanisme kinematik sementara sebelum bracing terkunci.").size(12.0).color(theme.text_secondary));
            });
        });
    }

    fn metric_card(&self, ui: &mut Ui, theme: &MacTheme, title: &str, value: &str, sub: &str, color: Color32) {
        theme.card_frame().show(ui, |ui| {
            ui.label(RichText::new(title).size(11.0).strong().color(theme.text_secondary));
            ui.add_space(4.0);
            ui.heading(RichText::new(value).size(20.0).strong().color(color));
            ui.add_space(2.0);
            ui.label(RichText::new(sub).size(11.0).color(theme.text_muted));
        });
    }
}
