//! facility.rs — Pillar 7: 7D BIM Facility Management.
//! Lifecycle operation and maintenance data used by building owners to manage assets, repairs, and components after construction is complete.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::models::StructuralMember;
use crate::theme::MacTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacilitySubTab {
    DigitalTwinShm,
    AssetRegistry,
    PredictiveMaintenance,
    LifecycleCosting,
}

pub struct FacilityView {
    pub active_subtab: FacilitySubTab,
    pub live_telemetry_active: bool,
    pub simulated_strain_micro: f32,
    pub simulated_freq_hz: f32,
    pub simulated_degradation_pct: f32,
}

impl Default for FacilityView {
    fn default() -> Self {
        Self {
            active_subtab: FacilitySubTab::DigitalTwinShm,
            live_telemetry_active: true,
            simulated_strain_micro: 342.0,
            simulated_freq_hz: 3.48,
            simulated_degradation_pct: 1.8,
        }
    }
}

impl FacilityView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
        ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::symmetric(20, 12)).show(ui, |ui| {
                // 1. TOP SUBTABS SELECTOR
                self.render_header_bar(ui, theme);

                ui.add_space(16.0);

                // 2. ACTIVE SUB-TAB CONTENT
                match self.active_subtab {
                    FacilitySubTab::DigitalTwinShm => self.render_digital_twin_shm(ui, theme, members),
                    FacilitySubTab::AssetRegistry => self.render_asset_registry(ui, theme, members),
                    FacilitySubTab::PredictiveMaintenance => self.render_predictive_maintenance(ui, theme),
                    FacilitySubTab::LifecycleCosting => self.render_lifecycle_costing(ui, theme),
                }

                ui.add_space(24.0);
            });
        });
    }

    fn render_header_bar(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(6))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                let tabs = [
                    (FacilitySubTab::DigitalTwinShm, "📡 Digital Twin & Sensor SHM"),
                    (FacilitySubTab::AssetRegistry, "📑 Registri Aset COBie"),
                    (FacilitySubTab::PredictiveMaintenance, "🛠️ Pemeliharaan Prediktif (ARI Alert)"),
                    (FacilitySubTab::LifecycleCosting, "💵 30-Year Lifecycle Cost (LCC)"),
                ];

                ui.columns(4, |cols| {
                    for (idx, (tab, label)) in tabs.into_iter().enumerate() {
                        let is_active = self.active_subtab == tab;
                        let bg = if is_active { theme.accent_blue } else { Color32::TRANSPARENT };
                        let fg = if is_active { Color32::WHITE } else { theme.text_secondary };

                        let w = cols[idx].available_width();
                        if cols[idx].add(egui::Button::new(RichText::new(label).size(12.0).strong().color(fg))
                            .fill(bg)
                            .corner_radius(CornerRadius::same(7))
                            .min_size(Vec2::new(w, 30.0))).clicked() {
                            self.active_subtab = tab;
                        }
                    }
                });
            });
    }

    fn render_digital_twin_shm(&mut self, ui: &mut Ui, theme: &MacTheme, _members: &[StructuralMember]) {
        // Top Real-time Sensor Status Row
        ui.columns(4, |cols| {
            self.render_sensor_card(&mut cols[0], theme, "STRAIN GAUGE SG-01", &format!("{:.0} µε", self.simulated_strain_micro), "Tegangan Batang E1 (Aman < 1200 µε)", theme.accent_green);
            self.render_sensor_card(&mut cols[1], theme, "FREKUENSI ALAMI (ACC-01)", &format!("{:.2} Hz", self.simulated_freq_hz), "Mode 1 Transversal (Bebas Resonansi)", theme.accent_blue);
            self.render_sensor_card(&mut cols[2], theme, "LIVE ARI DEGRADATION", "0.938", "ARI(t) Degradasi Realtime (-0.4%)", theme.accent_green);
            self.render_sensor_card(&mut cols[3], theme, "STATUS KESEHATAN (SHM)", "🟢 NORMAL", "16 Sensor IoT Aktif Beroperasi", theme.accent_green);
        });

        ui.add_space(16.0);

        // Digital Twin Bridge Telemetry Canvas
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📡 Telemetri Sensor Digital Twin & Pemetaan ARI Dinamis").size(13.5).strong().color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Sampling: 100 Hz • Protokol MQTT/TLS").size(11.0).color(theme.accent_blue));
                    });
                });
                ui.add_space(12.0);

                let sensors = [
                    ("SG-01 (Bottom Chord Tengah)", "Strain Gauge Serat Optik", "342 µε", "68.4 MPa", "ARI: 0.94", "🟢 Normal Operasional"),
                    ("SG-02 (Top Chord Tengah)", "Strain Gauge Tahanan Listrik", "-285 µε", "-57.0 MPa", "ARI: 0.94", "🟢 Tekan Elastis Aman"),
                    ("ACC-01 (Gelagar Tengah)", "Akselerometer Piezoelektrik Triaksial", "0.042 g", "3.48 Hz", "Kondisi: Kokoh", "🟢 Vibrasi Dinamis Terkendali"),
                    ("TILT-01 (Tumpuan Rol Kanan)", "Inklinometer Digital Dual-Axis", "0.018°", "Defleksi 1.4 mm", "Rotasi Aman", "🟢 Tumpuan Bergerak Bebas"),
                    ("UT-01 (Pelat Buhul Kritis)", "Sensor Ketebalan Korosi Ultrasonik", "15.94 mm", "Laju Korosi: 0.01 mm/th", "Sisa Umur: 94 Th", "🟢 Proteksi Cat Masih Prima"),
                ];

                for (id, stype, val, derived, ari_info, status) in sensors {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(id).size(12.0).strong().color(theme.text_primary));
                            ui.label(RichText::new(stype).size(11.0).color(theme.text_secondary));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(status).size(11.5).color(theme.accent_green));
                            ui.add_space(16.0);
                            ui.label(RichText::new(ari_info).size(11.5).strong().color(theme.accent_blue));
                            ui.add_space(16.0);
                            ui.label(RichText::new(derived).size(11.5).color(theme.text_secondary));
                            ui.add_space(16.0);
                            ui.label(RichText::new(val).size(11.5).strong().color(theme.text_primary));
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_asset_registry(&mut self, ui: &mut Ui, theme: &MacTheme, _members: &[StructuralMember]) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📑 Registri Aset Fasilitas (Standar COBie / IFC 4.3)").size(13.5).strong().color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Total Aset: 48 Komponen Terindeks").size(11.5).color(theme.text_muted));
                    });
                });
                ui.add_space(12.0);

                let assets = [
                    ("AST-CH-001", "Bottom Chord W14x90", "Struktur Baja", "ASTM A992", "2026-10-15", "100 Tahun", "Tiap 2 Tahun", "🟢 Grade 1 (Baru)"),
                    ("AST-CH-002", "Top Chord W14x90", "Struktur Baja", "ASTM A992", "2026-10-15", "100 Tahun", "Tiap 2 Tahun", "🟢 Grade 1 (Baru)"),
                    ("AST-DG-003", "Diagonal Web HSS8x8x1/2", "Bracing Sekunder", "ASTM A500 Gr.B", "2026-10-15", "100 Tahun", "Tiap 3 Tahun", "🟢 Grade 1 (Baru)"),
                    ("AST-BRG-01", "Elastomeric Pot Bearing 150T", "Bantalan Jembatan", "Neoprene/Baja", "2026-10-15", "25 Tahun", "Tiap 1 Tahun", "🟢 Grade 1 (Prima)"),
                    ("AST-EXP-01", "Modular Expansion Joint 80mm", "Dilatasi Lantai", "Baja Galvanis", "2026-10-15", "20 Tahun", "Tiap 6 Bulan", "🟢 Grade 1 (Bebas Celah)"),
                ];

                for (tag, name, cat, spec, comm, life, interval, cond) in assets {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(format!("{} • {}", tag, name)).size(12.0).strong().color(theme.text_primary));
                            ui.label(RichText::new(format!("{} | {} | Masa Desain: {}", cat, spec, life)).size(11.0).color(theme.text_secondary));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(cond).size(11.5).color(theme.accent_green));
                            ui.add_space(16.0);
                            ui.label(RichText::new(interval).size(11.5).color(theme.accent_blue));
                            ui.add_space(16.0);
                            ui.label(RichText::new(comm).size(11.0).color(theme.text_muted));
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_predictive_maintenance(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("🛠️ Protokol Pemeliharaan Prediktif Berbasis Indeks Redundansi (ARI Alert)").size(13.5).strong().color(theme.text_primary));
                ui.add_space(6.0);
                ui.label(RichText::new("Sistem otomatis menerbitkan Surat Perintah Kerja (SPK) saat indikator degradasi material atau kelelahan (fatigue) mendekati ambang batas kritis.").size(11.5).color(theme.text_secondary));
                ui.add_space(14.0);

                let work_orders = [
                    ("SPK-2026-088", "Inspeksi Visual & Pengukuran Torsi Baut Gusset Joint J-04", "Rutin 6-Bulanan", "Dijadwalkan: 15 Apr 2027", "🟢 Terencana", theme.accent_blue),
                    ("SPK-2026-089", "Pengujian Ultrasonik NDT Batang Tarik Kritis E1 (Bottom Chord)", "Berdasarkan Sensitivitas ARI (CI=0.42)", "Dijadwalkan: 10 Okt 2027", "🟢 Siap Dilaksanakan", theme.accent_green),
                    ("SPK-2026-090", "Pembersihan Saluran Drainase & Celah Bearing Tumpuan", "Cegah Korosi Air Garam", "Dijadwalkan: 01 Des 2026", "🟡 Mendatang", theme.text_secondary),
                ];

                for (code, task, reason, sched, status, col) in work_orders {
                    Frame::new()
                        .fill(theme.bg_canvas)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(12))
                        .stroke(Stroke::new(1.0, theme.border_subtle))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(format!("{} — {}", code, task)).size(12.5).strong().color(theme.text_primary));
                                    ui.label(RichText::new(format!("Alasan: {} | {}", reason, sched)).size(11.0).color(theme.text_secondary));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(RichText::new(status).size(12.0).strong().color(col));
                                });
                            });
                        });
                    ui.add_space(8.0);
                }
            });
    }

    fn render_lifecycle_costing(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("💵 Analisis Biaya Siklus Hidup 30 Tahun (Life Cycle Costing - LCC)").size(13.5).strong().color(theme.text_primary));
                ui.add_space(10.0);

                ui.columns(3, |cols| {
                    self.render_sensor_card(&mut cols[0], theme, "BIAYA AWAL (CAPEX)", "Rp 3.84 M", "Konstruksi Awal Terverifikasi RAB 5D", theme.text_primary);
                    self.render_sensor_card(&mut cols[1], theme, "ESTIMASI OPEX (30 TH)", "Rp 645 Jt", "Pemeliharaan Rutin & Sensor SHM", theme.accent_blue);
                    self.render_sensor_card(&mut cols[2], theme, "PENGHEMATAN DINI ARI", "Rp 1.45 M", "Pencegahan Kerusakan Fatal Katastropik", theme.accent_green);
                });
            });
    }

    fn render_sensor_card(&self, ui: &mut Ui, theme: &MacTheme, title: &str, val: &str, subtitle: &str, val_color: Color32) {
        Frame::new()
            .fill(theme.bg_canvas)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(title).size(10.0).strong().color(theme.text_muted));
                ui.add_space(4.0);
                ui.label(RichText::new(val).size(18.0).strong().color(val_color));
                ui.add_space(4.0);
                ui.label(RichText::new(subtitle).size(10.5).color(theme.text_secondary));
            });
    }
}
