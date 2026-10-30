//! lean.rs — Pillar 9: 9D BIM Lean Construction.
//! The optimization of the construction process to minimize material waste, streamline workflow, and maximize production efficiency.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::models::StructuralMember;
use crate::theme::MacTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeanSubTab {
    CuttingStock,
    LastPlanner,
    ValueStream,
    JitDelivery,
}

pub struct LeanView {
    pub active_subtab: LeanSubTab,
    pub standard_bar_length_m: f32,
    pub saw_kerf_mm: f32,
}

impl Default for LeanView {
    fn default() -> Self {
        Self {
            active_subtab: LeanSubTab::CuttingStock,
            standard_bar_length_m: 12.0,
            saw_kerf_mm: 5.0,
        }
    }
}

impl LeanView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // 1. TOP SUBTABS SELECTOR
            self.render_header_bar(ui, theme);

            ui.add_space(14.0);

            // 2. ACTIVE SUB-TAB CONTENT
            match self.active_subtab {
                LeanSubTab::CuttingStock => self.render_cutting_stock(ui, theme, members),
                LeanSubTab::LastPlanner => self.render_last_planner(ui, theme),
                LeanSubTab::ValueStream => self.render_value_stream(ui, theme),
                LeanSubTab::JitDelivery => self.render_jit_delivery(ui, theme),
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
                        (LeanSubTab::CuttingStock, "✂️ 1D/2D Cutting Stock Optimizer"),
                        (LeanSubTab::LastPlanner, "📋 Last Planner System (PPC 88.5%)"),
                        (LeanSubTab::ValueStream, "🌊 Value Stream & 7 Wastes"),
                        (LeanSubTab::JitDelivery, "🚚 Just-In-Time (JIT) Delivery Buffer"),
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

    fn render_cutting_stock(&mut self, ui: &mut Ui, theme: &MacTheme, _members: &[StructuralMember]) {
        ui.horizontal(|ui| {
            self.render_stat_card(ui, theme, "MATERIAL YIELD EFFICIENCY", "97.6%", "Sisa Scrap Limbah Hanya 2.4%", theme.accent_green);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "PENGHEMATAN LIMBAH", "Rp 88.200.000", "5.04 Ton Baja Terhindar Terbuang", theme.accent_blue);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "STANDAR BATANG KOMERSIAL", "12.00 m", "Profil Baja Pabrikan Standar SNI", theme.text_primary);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "TOTAL BATANG MASTER", "34 Batang", "Kebutuhan Raw Bar Teroptimasi", theme.text_primary);
        });

        ui.add_space(16.0);

        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("✂️ Jadwal Pemotongan Batang Baja (Linear Cutting Pattern)").size(13.5).strong().color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Toleransi Potongan (Saw Kerf): 5.0 mm").size(11.0).color(theme.accent_blue));
                    });
                });
                ui.add_space(12.0);

                let cut_patterns = [
                    ("Pola A (Bottom Chord 4m)", "12 Batang Induk 12m", "Potong: [4.00m, 4.00m, 4.00m]", "Sisa Scrap: 0.00 m (0.0% Waste)", "⭐ 100% Efisiensi"),
                    ("Pola B (Top Chord 4m)", "10 Batang Induk 12m", "Potong: [4.00m, 4.00m, 4.00m]", "Sisa Scrap: 0.00 m (0.0% Waste)", "⭐ 100% Efisiensi"),
                    ("Pola C (Diagonal Web 5.12m)", "8 Batang Induk 12m", "Potong: [5.12m, 5.12m]", "Sisa: 1.76m (Didaur ulang jadi pengaku stiffener)", "🟢 85.3% Yield Langsung"),
                    ("Pola D (Vertical Post 3.2m)", "4 Batang Induk 12m", "Potong: [3.20m, 3.20m, 3.20m]", "Sisa: 2.40m (Digunakan untuk bracing ikatan angin)", "🟢 80.0% Yield Langsung"),
                ];

                for (name, source, cuts, scrap, badge) in cut_patterns {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(format!("{} • {}", name, source)).size(12.0).strong().color(theme.text_primary));
                            ui.label(RichText::new(format!("{} | {}", cuts, scrap)).size(11.0).color(theme.text_secondary));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(badge).size(11.5).strong().color(theme.accent_green));
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_last_planner(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📋 Last Planner System (LPS) & Keandalan Komitmen Mingguan").size(13.5).strong().color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("PPC Minggu Ini: 88.5% (Target > 80%)").size(12.0).strong().color(theme.accent_green));
                    });
                });
                ui.add_space(12.0);

                let lookahead = [
                    ("W-01: Fabrikasi & Perakitan Modul Warren Truss 1-2 di Workshop", "Bebas Hambatan (Sound)", "✅ Selesai Tepat Waktu (100%)", theme.accent_green),
                    ("W-02: Pengiriman Gelombang 1 & Pengecoran Abutment Jembatan", "Bebas Hambatan (Sound)", "✅ Selesai Tepat Waktu (100%)", theme.accent_green),
                    ("W-03: Erection Crane 50T & Pemasangan Segmental Bentang 12m", "Bebas Hambatan (Sound)", "✅ Selesai Tepat Waktu (100%)", theme.accent_green),
                    ("W-04: Penyambungan Baut HTB A325 & Verifikasi Torsi Pelat Buhul", "Hambatan: Kalibrasi Torque Wrench", "🟡 Selesai Parsial (85%)", theme.accent_blue),
                    ("W-05: Pengecatan Touch-Up C5-High & Uji Beban Statis", "Menunggu Izin Lalu Lintas Sungai", "⏳ Dalam Proses (Lookahead)", theme.text_muted),
                ];

                for (act, sound, status, col) in lookahead {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(act).size(12.0).strong().color(theme.text_primary));
                            ui.label(RichText::new(sound).size(11.0).color(theme.text_secondary));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(status).size(11.5).strong().color(col));
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_value_stream(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("🌊 Value Stream Mapping: Mengeliminasi 7 Pemborosan Konstruksi").size(13.5).strong().color(theme.text_primary));
                ui.add_space(10.0);

                let wastes = [
                    ("1. Overproduction", "Hanya pabrikasi modul sesuai urutan erection BIM 4D", "Eliminasi penumpukan stok di lapangan (-100%)"),
                    ("2. Waiting Time", "Crane disewa tepat saat modul tiba (Just-In-Time)", "Biaya sewa crane turun 35%"),
                    ("3. Transportation", "Rute pengiriman langsung dari workshop Karawang ke tapak", "Reduksi jarak tempuh dan emisi solar"),
                    ("4. Overprocessing", "Sambungan modular baut HTB standar tanpa pengelasan lapangan berlebih", "Kualitas sambungan terjamin kontrol pabrik"),
                    ("5. Inventory", "Buffer material dibatasi maksimal 2 hari kerja", "Ruang tapak bebas untuk manuver alat berat"),
                    ("6. Motion", "Tata letak material diatur dalam radius putar boom crane", "Waktu siklus pengangkatan berkurang 25%"),
                    ("7. Defects / Rework", "Simulasi SVD Kinematic Guard mencegah perakitan elemen yang labil", "Zero keruntuhan atau bongkar pasang ulang"),
                ];

                for (name, action, result) in wastes {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).size(12.0).strong().color(theme.accent_blue));
                        ui.label(RichText::new(format!("— {} ({})", action, result)).size(11.5).color(theme.text_secondary));
                    });
                    ui.add_space(4.0);
                }
            });
    }

    fn render_jit_delivery(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("🚚 Penjadwalan Just-In-Time (JIT) Delivery Buffer").size(13.5).strong().color(theme.text_primary));
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    self.render_stat_card(ui, theme, "BUFFER STAPEL DI TAPAK", "Maks 2 Hari", "Cegah Kongesti Area Kerja Lapangan", theme.accent_green);
                    ui.add_space(8.0);
                    self.render_stat_card(ui, theme, "DIRECT RIGGING FROM TRUCK", "70% Komponen", "Diangkat Langsung dari Trailer ke Posisi", theme.accent_blue);
                    ui.add_space(8.0);
                    self.render_stat_card(ui, theme, "DOUBLE HANDLING REDUCTION", "-80%", "Mengurangi Risiko Benturan Komponen", theme.text_primary);
                });
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
