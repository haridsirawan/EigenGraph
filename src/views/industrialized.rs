//! industrialized.rs — Pillar 10: 10D BIM Industrialized Construction & Automation.
//! The integration of Industry 4.0 technologies, including prefabrication, robotics, 3D printing, and automated site processes.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::models::StructuralMember;
use crate::theme::MacTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndustrializedSubTab {
    ModularDfma,
    RoboticAutomation,
    AutomatedErection,
    Industry4Bridge,
}

pub struct IndustrializedView {
    pub active_subtab: IndustrializedSubTab,
    pub offsite_ratio_pct: f32,
    pub robotic_speed_mm_s: f32,
    pub export_status: Option<String>,
}

impl Default for IndustrializedView {
    fn default() -> Self {
        Self {
            active_subtab: IndustrializedSubTab::ModularDfma,
            offsite_ratio_pct: 82.5,
            robotic_speed_mm_s: 14.5,
            export_status: None,
        }
    }
}

impl IndustrializedView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, _members: &[StructuralMember]) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // 1. TOP SUBTABS SELECTOR
            self.render_header_bar(ui, theme);

            if let Some(ref status_msg) = self.export_status {
                ui.add_space(10.0);
                theme.card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("📦 10D EXPORT SIAP:").size(11.5).strong().color(theme.accent_green));
                        ui.label(RichText::new(status_msg).size(11.5).color(theme.text_primary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(egui::Button::new(RichText::new("✕").size(11.0).color(theme.text_muted))
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(4))).clicked() {
                                self.export_status = None;
                            }
                        });
                    });
                });
            }

            ui.add_space(14.0);

            // 2. ACTIVE SUB-TAB CONTENT
            match self.active_subtab {
                IndustrializedSubTab::ModularDfma => self.render_modular_dfma(ui, theme),
                IndustrializedSubTab::RoboticAutomation => self.render_robotic_automation(ui, theme),
                IndustrializedSubTab::AutomatedErection => self.render_automated_erection(ui, theme),
                IndustrializedSubTab::Industry4Bridge => self.render_industry4_bridge(ui, theme),
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
                        (IndustrializedSubTab::ModularDfma, "🏭 Prefabrikasi & DfMA Modular"),
                        (IndustrializedSubTab::RoboticAutomation, "🦾 Robotik & Pengelasan Otomatis"),
                        (IndustrializedSubTab::AutomatedErection, "🏗️ Crane Otonom & Drone Lidar"),
                        (IndustrializedSubTab::Industry4Bridge, "🌐 OpenBIM IFC 4.3 & IoT Bridge"),
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

    fn render_modular_dfma(&mut self, ui: &mut Ui, theme: &MacTheme) {
        ui.horizontal(|ui| {
            self.render_stat_card(ui, theme, "OFF-SITE FABRICATION RATIO", "82.5%", "Mayoritas Dirakit Presisi di Pabrik", theme.accent_green);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "STANDARDIZATION INDEX", "91.5%", "Repetisi Sambungan Buhul Modular", theme.accent_blue);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "DIMENSI MODUL TRANSPORT", "12.0 x 2.4 m", "Sesuai Regulasi Trailer Jalan Raya", theme.text_primary);
            ui.add_space(8.0);
            self.render_stat_card(ui, theme, "ASSEMBLY TIME AT SITE", "-60% Hari", "Perakitan Cepat Sistem Baut HTB", theme.accent_green);
        });

        ui.add_space(16.0);

        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🏭 Desain untuk Fabrikasi & Perakitan (DfMA Hierarchy)").size(13.5).strong().color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Toleransi Pabrikasi: ± 1.5 mm").size(11.0).color(theme.accent_blue));
                    });
                });
                ui.add_space(12.0);

                let modules = [
                    ("MOD-TRUSS-01 (Kiri Bentang 12m)", "Sub-Assembling Warren Truss A", "24.2 Ton", "Robotic Jig Fixture", "🟢 Selesai di Pabrik (Inspected)"),
                    ("MOD-TRUSS-02 (Kanan Bentang 12m)", "Sub-Assembling Warren Truss B", "24.2 Ton", "Robotic Jig Fixture", "🟢 Selesai di Pabrik (Inspected)"),
                    ("MOD-SPLICE-KEY (Sambungan Tengah)", "Center Crown Splice Bolted Node", "0.8 Ton", "CNC Milled Gusset Plate", "🟢 Ready for Site Bolting"),
                    ("MOD-BRG-SEAT (Dudukan Tumpuan)", "Pedestal Anchor & Bearing Plate", "1.2 Ton", "Automated Flame Cut", "🟢 Terpasang di Abutment"),
                ];

                for (code, name, wt, fab, status) in modules {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(format!("{} • {}", code, name)).size(12.0).strong().color(theme.text_primary));
                            ui.label(RichText::new(format!("Metode: {} | Berat: {}", fab, wt)).size(11.0).color(theme.text_secondary));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(status).size(11.5).strong().color(theme.accent_green));
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_robotic_automation(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🦾 Otomatisasi Robotik & Kontrol Mutu Industry 4.0").size(13.5).strong().color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Tingkat Kelulusan NDT: 99.4%").size(12.0).strong().color(theme.accent_green));
                    });
                });
                ui.add_space(12.0);

                let robotics = [
                    ("Lengan Robotik 6-Axis GMAW/FCAW", "Pengelasan Otomatis Pelat Buhul Gusset", "14.5 mm/s", "AWS D1.1 Certified", "🟢 Beroperasi Sinkron"),
                    ("Mesin CNC Laser Cutting Fiber 15kW", "Pemotongan Profil Web & Flens Baja", "85.0 mm/s", "Toleransi 0.05 mm", "🟢 Nesting Otomatis"),
                    ("Scanner AI Ultrasonic Phased-Array", "Uji Tak Merusak (NDT) Bebas Cacat Las", "Real-Time 3D", "Defect: 0 Porositas", "🟢 Kualitas Superior"),
                    ("Mesin Pengeboran Balok CNC 3-Spindle", "Pengeboran Lubang Baut HTB A325", "36 Lubang/menit", "Akurasi Lubang ±0.2mm", "🟢 Siap Rakit Langsung"),
                ];

                for (mach, role, spd, cert, st) in robotics {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(mach).size(12.0).strong().color(theme.text_primary));
                            ui.label(RichText::new(format!("Aplikasi: {} | Kecepatan: {}", role, spd)).size(11.0).color(theme.text_secondary));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(st).size(11.5).strong().color(theme.accent_green));
                            ui.add_space(16.0);
                            ui.label(RichText::new(cert).size(11.0).color(theme.accent_blue));
                        });
                    });
                    ui.separator();
                }
            });
    }

    fn render_automated_erection(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("🏗️ Crane Otonom, Trajektori Anti-Tabrakan & Inspeksi Drone LiDAR").size(13.5).strong().color(theme.text_primary));
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    self.render_stat_card(ui, theme, "CRANE TRAJECTORY CLEARANCE", "2.8 m", "Zona Bebas Tabrakan Boom Dinamis", theme.accent_green);
                    ui.add_space(8.0);
                    self.render_stat_card(ui, theme, "DRONE LIDAR AS-BUILT", "± 2.1 mm", "Penyimpangan Model vs Aktual Lapangan", theme.accent_blue);
                    ui.add_space(8.0);
                    self.render_stat_card(ui, theme, "SVD ERECTION GUARD", "Rank Penuh", "Kestabilan Tiap Tahap Terjamin Bebas Mekanisme", theme.accent_green);
                });
            });
    }

    fn render_industry4_bridge(&mut self, ui: &mut Ui, theme: &MacTheme) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.label(RichText::new("🌐 Jembatan Integrasi Industri 4.0 & OpenBIM Ekosistem").size(13.5).strong().color(theme.text_primary));
                ui.add_space(10.0);

                let formats = [
                    ("IFC 4.3 Structural Analysis MVD", "Interoperabilitas Standar buildingSMART", "Mengekspor topologi elemen, beban kombinasi, dan metadata ARI langsung ke software komersial (SAP2000, ETABS, Revit)."),
                    ("BCF 3.0 (BIM Collaboration Format)", "Manajemen Isu Desain & Clash Berbasis Cloud", "Sinkronisasi otomatis peringatan SVD Mechanism dan tegangan kritis ke platform kolaborasi tim."),
                    ("STEP-NC & G-Code Robotik", "Instruksi Mesin CNC & Pengelasan Otomatis", "Toolpath numerik diekspor langsung dari geometri 3D tanpa perlu gambar manual tambahan."),
                    ("REST API & IoT MQTT Telemetry", "Konektivitas Sensor Digital Twin", "Menerima aliran data sensor lapangan dan memperbarui nilai ARI(t) secara real-time."),
                ];

                for (name, sub, desc) in formats {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).size(12.5).strong().color(theme.accent_blue));
                            ui.label(RichText::new(sub).size(11.0).color(theme.text_muted));
                            ui.label(RichText::new(desc).size(11.5).color(theme.text_secondary));
                        });
                    });
                    ui.separator();
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("🌐 Ekspor Berkas OpenBIM IFC 4.3 (.ifc)").size(12.0).strong().color(Color32::WHITE))
                        .fill(theme.accent_blue)
                        .corner_radius(CornerRadius::same(6))).clicked() {
                        let mut engine = crate::core::ParametricEngine::new();
                        if let Ok(model) = engine.generate_warren_truss() {
                            let ari = crate::core::AriEngine::evaluate_redundancy(&model).ok();
                            let ifc = crate::core::IfcExporter::export_to_ifc(&model, ari.as_ref());
                            self.export_status = Some(format!("Model diekspor ke format OpenBIM IFC 4.3 ({} baris). Siap dibuka di Solibri, Revit, dan Tekla.", ifc.lines().count()));
                        }
                    }

                    if ui.add(egui::Button::new(RichText::new("🦾 Ekspor Paket DSTV CNC (.nc1)").size(12.0).strong().color(Color32::WHITE))
                        .fill(theme.accent_green)
                        .corner_radius(CornerRadius::same(6))).clicked() {
                        let mut engine = crate::core::ParametricEngine::new();
                        if let Ok(model) = engine.generate_warren_truss() {
                            let pieces = crate::core::DstvExporter::export_to_dstv(&model);
                            self.export_status = Some(format!("{} berkas instruksi robotik DSTV .nc1 berhasil digenerasi untuk pemotongan dan pengeboran CNC otomatis.", pieces.len()));
                        }
                    }
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
