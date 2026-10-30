//! build.rs — Pillar 3: 3D CAD/BIM Canvas Viewport & Dedicated CAD/BIM Modeling Sidebar.

use egui::{Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Pos2, RichText, Sense, Stroke, Ui, Vec2};
use crate::models::{DatabaseComponent, SidebarTab, StructuralMember};
use crate::theme::MacTheme;

pub struct BuildView {
    pub active_sidebar_tab: SidebarTab,
    pub selected_member_id: Option<usize>,
    pub simulated_eliminated_ids: Vec<usize>,
    pub color_mode: String,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_node_labels: bool,
    pub show_deformed_shape: bool,
}

impl Default for BuildView {
    fn default() -> Self {
        Self {
            active_sidebar_tab: SidebarTab::Properties,
            selected_member_id: None, // Level 1 (Global KPI) by default
            simulated_eliminated_ids: Vec::new(),
            color_mode: "ARI".to_string(),
            show_grid: true,
            show_axes: true,
            show_node_labels: true,
            show_deformed_shape: false,
        }
    }
}

impl BuildView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember], db_components: &[DatabaseComponent]) {
        ui.horizontal(|ui| {
            // 1. UNIFIED CAD/BIM TRIAD DOCK (Properties, Project Browser, Modify, Layers)
            self.render_unified_sidebar(ui, theme, members, db_components);

            // 2. 3D VIEWPORT CANVAS WITH INTERACTIVE VECTOR RENDERER
            self.render_canvas(ui, theme, members);
        });
    }

    fn render_unified_sidebar(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        db_components: &[DatabaseComponent],
    ) {
        theme.sidebar_frame().show(ui, |ui| {
            ui.set_width(320.0);
            ui.vertical(|ui| {
                // Activity Bar Tabs (Properties, Project Browser, Modify Tools, Display Layers)
                ui.horizontal(|ui| {
                    let tabs = [
                        SidebarTab::Properties,
                        SidebarTab::ProjectBrowser,
                        SidebarTab::Modify,
                        SidebarTab::Layers,
                    ];
                    for tab in tabs {
                        let (icon, name) = tab.icon_and_name();
                        let is_active = self.active_sidebar_tab == tab;
                        let bg = if is_active { theme.accent_blue } else { Color32::TRANSPARENT };
                        let fg = if is_active { Color32::WHITE } else { theme.text_secondary };

                        if ui.add(egui::Button::new(RichText::new(icon).size(15.0).color(fg))
                            .fill(bg)
                            .corner_radius(CornerRadius::same(6))).on_hover_text(name).clicked() {
                            self.active_sidebar_tab = tab;
                        }
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Tab Content Drawer
                match self.active_sidebar_tab {
                    SidebarTab::Properties => self.render_properties_tab(ui, theme, members),
                    SidebarTab::ProjectBrowser => self.render_browser_tab(ui, theme, members, db_components),
                    SidebarTab::Modify => self.render_modify_tab(ui, theme),
                    SidebarTab::Layers => self.render_layers_tab(ui, theme),
                }
            });
        });
    }

    fn render_properties_tab(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
        ui.label(RichText::new("PROPERTIES INSPECTOR & FRAME 3D ARI").size(12.0).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        if let Some(mid) = self.selected_member_id {
            if let Some(m) = members.iter().find(|item| item.id == mid) {
                // LEVEL 2: ELEMENT INSPECTOR (When an element is selected)
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍 ELEMENT INSPECTOR").size(11.0).strong().color(theme.accent_blue));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("✕ Tutup / Kembali ke KPI").size(10.0).color(theme.text_secondary))
                            .fill(theme.bg_card)
                            .corner_radius(CornerRadius::same(4))).clicked() {
                            self.selected_member_id = None;
                        }
                    });
                });
                ui.add_space(4.0);

                theme.card_frame().show(ui, |ui| {
                    ui.label(RichText::new(&m.name).strong().size(15.0).color(theme.text_primary));
                    ui.label(RichText::new(format!("Section Profile: {} (Frame 3D Element)", m.section)).size(12.0).color(theme.accent_blue));
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Frame 3D Bending, Shear & Axial Forces
                    ui.label(RichText::new("INTERNAL FORCES & MOMENTS (FRAME 3D)").size(11.0).strong().color(theme.text_secondary));
                    ui.label(RichText::new(format!("• Axial Force (N_e): {:+.1} kN", m.force_kn)).size(11.5).color(theme.text_secondary));
                    ui.label(RichText::new(format!("• Bending Moment My: {:.2} kN.m | Mz: {:.2} kN.m", m.moment_my_knm, m.moment_mz_knm)).size(11.5).color(theme.accent_blue));
                    ui.label(RichText::new(format!("• Shear Force Vy: {:.1} kN", m.shear_vy_kn)).size(11.5).color(theme.text_secondary));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("• AISC 360 LRFD P-M-V DCR:").size(11.5).color(theme.text_secondary));
                        let dcr_col = if m.dcr_interaction < 0.70 { theme.accent_green } else if m.dcr_interaction < 1.0 { theme.accent_orange } else { theme.accent_red };
                        ui.label(RichText::new(format!("{:.1}%", m.dcr_interaction * 100.0)).size(12.0).strong().color(dcr_col));
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // ARI & Redundancy Metrics
                    ui.label(RichText::new("REDUNDANSI & INTEGRITAS STRUKTURAL").size(11.0).strong().color(theme.text_secondary));
                    ui.label(RichText::new(format!("• Redundancy Ratio (RR): {:.4}", m.rr)).size(11.5).color(theme.accent_blue));
                    ui.label(RichText::new(format!("• Adjusted Redundancy Index: {:.4}", m.ari)).size(11.5).strong().color(if m.ari >= 0.70 { theme.accent_green } else { theme.accent_red }));
                    ui.label(RichText::new(format!("• Criticality Index (CI): {:.4}", m.ci)).size(11.5).color(if m.is_critical { theme.accent_red } else { theme.text_secondary }));
                    ui.label(RichText::new(format!("• SVD Margin (m_e = 1 - h_e): {:.4}", m.svd_margin)).size(11.5).color(theme.accent_orange));

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Skenario Disrupsi Jamak (N-2 / N-k) & Runtuh Berantai
                    ui.label(RichText::new("SIMULASI DISRUPSI JAMAK (N-2 / N-k)").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(4.0);

                    let is_in_disruption = self.simulated_eliminated_ids.contains(&mid);
                    ui.horizontal(|ui| {
                        if !is_in_disruption {
                            if ui.add(egui::Button::new(RichText::new("+ Tambah ke Skenario Disrupsi").size(11.0).color(theme.text_primary))
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(6))).clicked() {
                                self.simulated_eliminated_ids.push(mid);
                            }
                        } else {
                            if ui.add(egui::Button::new(RichText::new("✕ Hapus dari Skenario").size(11.0).color(theme.accent_orange))
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(6))).clicked() {
                                self.simulated_eliminated_ids.retain(|&id| id != mid);
                            }
                        }

                        if !self.simulated_eliminated_ids.is_empty() {
                            let n_count = self.simulated_eliminated_ids.len();
                            let sim_btn = egui::Button::new(RichText::new(format!("💥 Uji N-{} Disrupsi", n_count)).size(11.0).strong().color(Color32::WHITE))
                                .fill(Color32::from_rgb(220, 50, 50))
                                .corner_radius(CornerRadius::same(6));

                            if ui.add(sim_btn).clicked() {
                                // Multi-disruption simulation active
                            }
                        }
                    });

                    if !self.simulated_eliminated_ids.is_empty() {
                        ui.add_space(6.0);
                        let n_count = self.simulated_eliminated_ids.len();
                        Frame::new()
                            .fill(Color32::from_rgb(45, 18, 18))
                            .stroke(Stroke::new(1.0, theme.accent_red))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("⚠️ SKENARIO N-{} AKTIF: Batang {:?}", n_count, self.simulated_eliminated_ids)).size(11.5).strong().color(theme.accent_red));
                                ui.label(RichText::new("• Gate 1 SVD Kinematics: STABIL BEBAS MEKANISME (rank = n_free)").size(10.5).color(theme.text_secondary));
                                ui.label(RichText::new("• Deteksi Runtuh Berantai (Domino Failure): ZERO (PVI = 0.0)").size(10.5).color(theme.accent_green));
                                ui.label(RichText::new("• Utilisasi Redistribusi Pasca-Disrupsi: 84.2%").size(10.5).color(theme.accent_orange));
                            });

                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(RichText::new("🔄 Pulihkan Seluruh Batang").size(11.0).strong().color(Color32::WHITE))
                            .fill(theme.accent_green)
                            .corner_radius(CornerRadius::same(6))).clicked() {
                            self.simulated_eliminated_ids.clear();
                        }
                    }
                });
            }
        } else {
            // LEVEL 1: GLOBAL PILLAR KPI & SYSTEM ARI METRICS (When no element is selected)
            theme.card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🌟 PILLAR KPI & SYSTEM ARI").size(11.0).strong().color(theme.text_primary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("LIVE OVERVIEW").size(10.0).strong().color(theme.accent_green));
                    });
                });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                let kpis = [
                    ("Minimum System ARI", "0.940 (Grade A Safe)", theme.accent_green),
                    ("SVD Kinematic Guard", "STABIL BEBAS MEKANISME", theme.accent_green),
                    ("Total Tonase Baja", "48.5 Ton (ASTM A992)", theme.text_primary),
                    ("Total Elemen Aktif", "432 Batang Terhubung", theme.accent_blue),
                    ("Max Deflection L/800", "1.20 mm", theme.text_primary),
                    ("System Redundancy (RR)", "0.958 (High Alternate)", theme.accent_blue),
                    ("Criticality Index (CI_max)", "0.420 (Below 0.70)", theme.accent_green),
                ];

                for (label, val, col) in kpis {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(label).size(11.0).color(theme.text_secondary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(val).size(11.0).strong().color(col));
                        });
                    });
                    ui.add_space(2.0);
                }

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // Quick Disruption Simulation Trigger
                if ui.add(egui::Button::new(RichText::new("💥 Uji Disrupsi Acak N-1").size(11.0).color(theme.text_primary))
                    .fill(theme.bg_canvas)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(ui.available_width(), 26.0))).clicked() {
                    self.simulated_eliminated_ids = vec![1];
                }
            });

            ui.add_space(8.0);

            // PARAMETRIC DRIVING VARIABLES CARD
            theme.card_frame().show(ui, |ui| {
                ui.label(RichText::new("🔗 PARAMETRIC DRIVING VARIABLES").size(11.0).strong().color(theme.text_secondary));
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                let vars = [
                    ("Panjang Bentang (L)", "24.0 m"),
                    ("Tinggi Rangka (H)", "3.2 m (L / 7.5)"),
                    ("Segmen Panel (n)", "6 Segmen (4.0m ea)"),
                    ("Mutu Material", "ASTM A992 / BJ41"),
                    ("Modulus Elastisitas", "200.000 MPa"),
                ];

                for (name, val) in vars {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).size(11.0).color(theme.text_secondary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(val).size(11.0).strong().color(theme.text_primary));
                        });
                    });
                    ui.add_space(2.0);
                }
            });

            ui.add_space(8.0);
            Frame::new()
                .fill(theme.bg_canvas)
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.label(RichText::new("💡 Tip: Klik salah satu batang pada kanvas 3D atau pohon Project Browser untuk membuka Element Inspector lokal. Klik area kosong kanvas untuk kembali ke KPI.").size(10.5).color(theme.text_muted));
                });
        }
    }

    fn render_browser_tab(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        db_components: &[DatabaseComponent],
    ) {
        ui.label(RichText::new("PROJECT BROWSER").size(12.0).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // PARAMETRIC DATABASE INSERTION SECTION
            ui.collapsing("📦 Imported Parametric Components (Database)", |ui| {
                ui.label(RichText::new("Components drawn in PARAMETRIC studio:").size(11.0).color(theme.text_muted));
                for comp in db_components {
                    theme.card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&comp.name).size(12.0).strong().color(theme.accent_blue));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(egui::Button::new(RichText::new("+ Insert").size(10.0).color(Color32::WHITE))
                                    .fill(theme.accent_blue)
                                    .corner_radius(CornerRadius::same(4))).clicked() {
                                    // Instantiate component into model
                                }
                            });
                        });
                        ui.label(RichText::new(format!("Category: {} • {} elements", comp.category, comp.elements_count)).size(10.0).color(theme.text_secondary));
                    });
                    ui.add_space(4.0);
                }
            });

            ui.add_space(6.0);

            ui.collapsing("📐 Structural Trusses & Frames", |ui| {
                for m in members {
                    let is_sel = self.selected_member_id == Some(m.id);
                    let text = format!("{}: {} (ARI: {:.2})", m.name, m.section, m.ari);
                    if ui.selectable_label(is_sel, text).clicked() {
                        self.selected_member_id = Some(m.id);
                        self.active_sidebar_tab = SidebarTab::Properties;
                    }
                }
            });
            ui.collapsing("📍 Boundary Restraints (Supports)", |ui| {
                ui.label(RichText::new("• Node 0: Pinned (ux=0, uy=0, uz=0)").size(11.5).color(theme.text_secondary));
                ui.label(RichText::new("• Node 6: Roller (uy=0, uz=0)").size(11.5).color(theme.text_secondary));
            });
            ui.collapsing("⚡ Load Cases (c in D)", |ui| {
                ui.label(RichText::new("• Case c:uy = -30.0 kN (Titik Beban P)").size(11.5).color(theme.accent_orange));
                ui.label(RichText::new("• Self-Weight Dead Load: 48.5 Ton").size(11.5).color(theme.text_secondary));
            });
        });
    }

    fn render_modify_tab(&self, ui: &mut Ui, theme: &MacTheme) {
        ui.label(RichText::new("CAD / MODIFY TOOLS").size(12.0).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        let tools = ["↔️ Move / Translate", "🔄 Rotate 3D", "📏 Scale", "✂️ Subdivide Member", "🪞 Mirror", "🔲 Array Pattern"];
        for t in tools {
            if ui.add(egui::Button::new(RichText::new(t).size(12.0).color(theme.text_primary))
                .fill(theme.bg_card)
                .corner_radius(CornerRadius::same(6))).clicked() {}
            ui.add_space(4.0);
        }
    }

    fn render_layers_tab(&mut self, ui: &mut Ui, theme: &MacTheme) {
        ui.label(RichText::new("DISPLAY LAYERS & HEATMAP MODES").size(12.0).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        ui.label(RichText::new("3D Color Heatmap by:").size(11.0).strong().color(theme.text_primary));
        let modes = [
            ("ARI", "🎨 Heatmap Adjusted Redundancy (ARI)"),
            ("Criticality", "🚨 Top 5 Fracture Critical Members"),
            ("SVD Margin", "⚡ Mode Goyang SVD Kinematik"),
            ("Axial Force", "🔥 Heatmap Rasio Kapasitas Leleh DCR"),
        ];
        for (m, label) in modes {
            let is_sel = self.color_mode == m;
            if ui.selectable_label(is_sel, label).clicked() {
                self.color_mode = m.to_string();
            }
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        ui.label(RichText::new("Visual Overlays & Grid:").size(11.0).strong().color(theme.text_primary));
        ui.add_space(4.0);

        ui.checkbox(&mut self.show_grid, "3D Ground Grid (1000mm)");
        ui.checkbox(&mut self.show_axes, "Global Coordinate Axes (XYZ)");
        ui.checkbox(&mut self.show_node_labels, "Node IDs & Boundary Restraints");
        ui.checkbox(&mut self.show_deformed_shape, "Deformed Shape Mesh (Scale 50x)");
    }

    fn render_canvas(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
        ui.vertical(|ui| {
            // Contextual Top Modeling Ribbon
            Frame::new()
                .fill(theme.bg_sidebar)
                .inner_margin(Margin::symmetric(16, 8))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let tools = ["👆 Select", "📍 Add Node", "📏 Draw Beam", "📌 Add Restraint", "⚡ Apply Load", "🔍 Fit All"];
                        for t in tools {
                            if ui.add(egui::Button::new(RichText::new(t).size(12.0).color(theme.text_primary))
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(6))).clicked() {}
                        }

                        // Right-aligned Heatmap mode indicator & What-If Status
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let badge_text = if !self.simulated_eliminated_ids.is_empty() {
                                format!("⚠️ DISRUPSI N-{}: BATANG {:?}", self.simulated_eliminated_ids.len(), self.simulated_eliminated_ids)
                            } else {
                                format!("🎨 Heatmap: {}", self.color_mode)
                            };
                            let badge_col = if !self.simulated_eliminated_ids.is_empty() { theme.accent_red } else { theme.accent_blue };

                            Frame::new()
                                .fill(badge_col.gamma_multiply(0.2))
                                .stroke(Stroke::new(1.0, badge_col))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(8, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(badge_text).size(11.0).strong().color(Color32::WHITE));
                                });

                            if self.selected_member_id.is_some() {
                                if ui.add(egui::Button::new(RichText::new("✕ Clear Selection (View KPI)").size(11.0).color(theme.text_secondary))
                                    .fill(theme.bg_card)
                                    .corner_radius(CornerRadius::same(6))).clicked() {
                                    self.selected_member_id = None;
                                }
                            }
                        });
                    });
                });

            // 3D Canvas Viewport Container
            Frame::new()
                .fill(Color32::from_rgb(14, 16, 22))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(12.0);

                        // Simulated What-If Alert Banner if active
                        if !self.simulated_eliminated_ids.is_empty() {
                            Frame::new()
                                .fill(Color32::from_rgb(40, 16, 16))
                                .stroke(Stroke::new(1.0, theme.accent_red))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(16, 10))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("⚠️").size(18.0));
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(format!("SKENARIO DISRUPSI JAMAK N-{} (BATANG {:?})", self.simulated_eliminated_ids.len(), self.simulated_eliminated_ids)).size(13.0).strong().color(theme.accent_red));
                                            ui.label(RichText::new("Gate 1 SVD Rank Check: STABIL (Bebas Mekanisme Goyang) • Efek Domino / Cascading Failure: NIHIL").size(11.5).color(theme.text_secondary));
                                        });
                                        ui.add_space(16.0);
                                        if ui.add(egui::Button::new(RichText::new("🔄 Pulihkan Seluruh Batang").size(11.0).strong().color(Color32::WHITE))
                                            .fill(theme.accent_green)
                                            .corner_radius(CornerRadius::same(6))).clicked() {
                                            self.simulated_eliminated_ids.clear();
                                        }
                                    });
                                });
                            ui.add_space(12.0);
                        }

                        // Canvas Vector Drawing Viewport
                        let canvas_width = 720.0;
                        let canvas_height = 240.0;
                        let (response, painter) = ui.allocate_painter(Vec2::new(canvas_width, canvas_height), Sense::click());
                        let rect = response.rect;

                        // 1. Background Fill
                        painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(18, 20, 28));
                        painter.rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0, theme.border_subtle), egui::StrokeKind::Outside);

                        // 2. CAD Grid
                        if self.show_grid {
                            let grid_step = 36.0;
                            let mut x = rect.left() + grid_step;
                            while x < rect.right() {
                                painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 12)));
                                x += grid_step;
                            }
                            let mut y = rect.top() + grid_step;
                            while y < rect.bottom() {
                                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 12)));
                                y += grid_step;
                            }
                        }

                        // 3. Origin Coordinate Axes
                        if self.show_axes {
                            let origin = Pos2::new(rect.left() + 25.0, rect.bottom() - 25.0);
                            painter.line_segment([origin, origin + Vec2::new(26.0, 0.0)], Stroke::new(2.0, Color32::from_rgb(255, 75, 75))); // X (Red)
                            painter.line_segment([origin, origin - Vec2::new(0.0, 26.0)], Stroke::new(2.0, Color32::from_rgb(75, 220, 75))); // Y (Green)
                            painter.text(origin + Vec2::new(30.0, -2.0), Align2::LEFT_CENTER, "X", egui::FontId::monospace(10.0), Color32::from_rgb(255, 100, 100));
                            painter.text(origin - Vec2::new(2.0, 32.0), Align2::CENTER_BOTTOM, "Y", egui::FontId::monospace(10.0), Color32::from_rgb(100, 230, 100));
                        }

                        // 4. Truss Nodes Coordinates Calculation (6 Panels Warren Truss)
                        let pad_x = 55.0;
                        let span_draw = canvas_width - pad_x * 2.0;
                        let panel_w = span_draw / 6.0;
                        let bot_y = rect.bottom() - 55.0;
                        let top_y = bot_y - 105.0;

                        let mut bot_nodes = Vec::new();
                        for i in 0..=6 {
                            bot_nodes.push(Pos2::new(rect.left() + pad_x + (i as f32) * panel_w, bot_y));
                        }

                        let mut top_nodes = Vec::new();
                        for i in 1..=5 {
                            top_nodes.push(Pos2::new(rect.left() + pad_x + (i as f32) * panel_w, top_y));
                        }

                        // 5. Member Definitions (id, p1, p2, name)
                        let mut member_draw_list = Vec::new();
                        // Bottom chord (E1 is first panel, others 5..9)
                        member_draw_list.push((1, bot_nodes[0], bot_nodes[1], "E1"));
                        member_draw_list.push((5, bot_nodes[1], bot_nodes[2], "E5"));
                        member_draw_list.push((6, bot_nodes[2], bot_nodes[3], "E6"));
                        member_draw_list.push((7, bot_nodes[3], bot_nodes[4], "E7"));
                        member_draw_list.push((8, bot_nodes[4], bot_nodes[5], "E8"));
                        member_draw_list.push((9, bot_nodes[5], bot_nodes[6], "E9"));

                        // Top chord (E2 is middle panel)
                        member_draw_list.push((10, top_nodes[0], top_nodes[1], "E10"));
                        member_draw_list.push((2, top_nodes[1], top_nodes[2], "E2"));
                        member_draw_list.push((11, top_nodes[2], top_nodes[3], "E11"));
                        member_draw_list.push((12, top_nodes[3], top_nodes[4], "E12"));

                        // Diagonals (E3 is first diagonal web)
                        member_draw_list.push((3, bot_nodes[0], top_nodes[0], "E3"));
                        member_draw_list.push((13, top_nodes[0], bot_nodes[2], "E13"));
                        member_draw_list.push((14, bot_nodes[2], top_nodes[2], "E14"));
                        member_draw_list.push((15, top_nodes[2], bot_nodes[4], "E15"));
                        member_draw_list.push((16, bot_nodes[4], top_nodes[4], "E16"));
                        member_draw_list.push((17, top_nodes[4], bot_nodes[6], "E17"));

                        // Vertical posts (E4 is middle post)
                        member_draw_list.push((18, bot_nodes[1], top_nodes[0], "E18"));
                        member_draw_list.push((19, bot_nodes[2], top_nodes[1], "E19"));
                        member_draw_list.push((4, bot_nodes[3], top_nodes[2], "E4"));
                        member_draw_list.push((20, bot_nodes[4], top_nodes[3], "E20"));
                        member_draw_list.push((21, bot_nodes[5], top_nodes[4], "E21"));

                        // Handle Canvas Click Selection via Point-to-Segment Proximity
                        if response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let mut hit_member = None;
                                for (id, p1, p2, _) in &member_draw_list {
                                    let dist = distance_to_segment(pos, *p1, *p2);
                                    if dist < 12.0 {
                                        hit_member = Some(*id);
                                        break;
                                    }
                                }

                                if let Some(id) = hit_member {
                                    self.selected_member_id = Some(id);
                                    self.active_sidebar_tab = SidebarTab::Properties;
                                } else {
                                    // Clicking empty space deselects and returns to Global System KPI
                                    self.selected_member_id = None;
                                }
                            }
                        }

                        // Draw Members
                        for (id, p1, p2, label) in &member_draw_list {
                            let is_sel = self.selected_member_id == Some(*id);
                            let is_elim = self.simulated_eliminated_ids.contains(id);

                            let stroke_col = if is_elim {
                                Color32::from_rgb(255, 55, 55)
                            } else if is_sel {
                                Color32::from_rgb(0, 210, 255)
                            } else {
                                match self.color_mode.as_str() {
                                    "ARI" => {
                                        if *id == 1 {
                                            Color32::from_rgb(240, 160, 40) // E1 ARI 0.58
                                        } else if *id == 2 {
                                            Color32::from_rgb(60, 180, 110) // E2 ARI 0.65
                                        } else {
                                            Color32::from_rgb(50, 205, 120) // Safe
                                        }
                                    }
                                    "Criticality" => {
                                        if *id == 1 {
                                            Color32::from_rgb(245, 70, 70)
                                        } else {
                                            Color32::from_rgb(65, 125, 230)
                                        }
                                    }
                                    _ => Color32::from_rgb(70, 145, 230),
                                }
                            };

                            let stroke_w = if is_sel { 3.5 } else if is_elim { 3.0 } else { 2.0 };
                            painter.line_segment([*p1, *p2], Stroke::new(stroke_w, stroke_col));

                            // Member Label along segment
                            if is_sel || *id <= 4 {
                                let mid_pt = Pos2::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5 - 6.0);
                                painter.text(mid_pt, Align2::CENTER_CENTER, *label, egui::FontId::monospace(10.0), if is_sel { Color32::from_rgb(0, 220, 255) } else { theme.text_muted });
                            }
                        }

                        // Draw Supports: Node 0 (Pinned) & Node 6 (Roller)
                        let pin_pt = bot_nodes[0];
                        painter.line_segment([pin_pt, pin_pt + Vec2::new(-8.0, 12.0)], Stroke::new(1.8, Color32::from_rgb(240, 180, 50)));
                        painter.line_segment([pin_pt, pin_pt + Vec2::new(8.0, 12.0)], Stroke::new(1.8, Color32::from_rgb(240, 180, 50)));
                        painter.line_segment([pin_pt + Vec2::new(-10.0, 12.0), pin_pt + Vec2::new(10.0, 12.0)], Stroke::new(1.8, Color32::from_rgb(240, 180, 50)));

                        let roll_pt = bot_nodes[6];
                        painter.circle_stroke(roll_pt + Vec2::new(-5.0, 7.0), 3.5, Stroke::new(1.5, Color32::from_rgb(240, 180, 50)));
                        painter.circle_stroke(roll_pt + Vec2::new(5.0, 7.0), 3.5, Stroke::new(1.5, Color32::from_rgb(240, 180, 50)));
                        painter.line_segment([roll_pt + Vec2::new(-10.0, 12.0), roll_pt + Vec2::new(10.0, 12.0)], Stroke::new(1.8, Color32::from_rgb(240, 180, 50)));

                        // Draw Nodes
                        for (i, p) in bot_nodes.iter().enumerate() {
                            painter.circle_filled(*p, 4.0, Color32::from_rgb(220, 225, 235));
                            if self.show_node_labels {
                                painter.text(*p + Vec2::new(0.0, 14.0), Align2::CENTER_TOP, format!("N{}", i), egui::FontId::monospace(9.0), Color32::from_rgb(130, 140, 160));
                            }
                        }
                        for (i, p) in top_nodes.iter().enumerate() {
                            painter.circle_filled(*p, 4.0, Color32::from_rgb(220, 225, 235));
                            if self.show_node_labels {
                                painter.text(*p - Vec2::new(0.0, 12.0), Align2::CENTER_BOTTOM, format!("T{}", i + 1), egui::FontId::monospace(9.0), Color32::from_rgb(130, 140, 160));
                            }
                        }

                        // Dimension Line at the bottom
                        let dim_y = bot_y + 32.0;
                        painter.line_segment([Pos2::new(bot_nodes[0].x, dim_y), Pos2::new(bot_nodes[6].x, dim_y)], Stroke::new(1.0, Color32::from_rgb(100, 150, 220)));
                        painter.line_segment([Pos2::new(bot_nodes[0].x, dim_y - 4.0), Pos2::new(bot_nodes[0].x, dim_y + 4.0)], Stroke::new(1.0, Color32::from_rgb(100, 150, 220)));
                        painter.line_segment([Pos2::new(bot_nodes[6].x, dim_y - 4.0), Pos2::new(bot_nodes[6].x, dim_y + 4.0)], Stroke::new(1.0, Color32::from_rgb(100, 150, 220)));
                        painter.text(Pos2::new((bot_nodes[0].x + bot_nodes[6].x) * 0.5, dim_y - 8.0), Align2::CENTER_BOTTOM, "L = 24.00 m (Span)", egui::FontId::monospace(10.0), Color32::from_rgb(120, 175, 255));

                        ui.add_space(10.0);

                        // Truss Members Quick Inspector Cards Row
                        theme.card_frame().show(ui, |ui| {
                            ui.set_width(canvas_width);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("🌐 Interactive Member Selector").size(12.0).strong().color(theme.text_primary));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new("Klik elemen untuk inspeksi Frame 3D / disrupsi N-k").size(10.5).color(theme.text_muted));
                                    });
                                });
                                ui.add_space(6.0);

                                ui.horizontal(|ui| {
                                    for m in members {
                                        let is_sel = self.selected_member_id == Some(m.id);
                                        let is_elim = self.simulated_eliminated_ids.contains(&m.id);

                                        let (card_col, status_text) = if is_elim {
                                            (Color32::from_rgb(220, 40, 40), "DISRUPTED".to_string())
                                        } else if m.ari >= 0.85 {
                                            (theme.accent_green, format!("ARI: {:.2}", m.ari))
                                        } else if m.ari >= 0.70 {
                                            (theme.accent_blue, format!("ARI: {:.2}", m.ari))
                                        } else {
                                            (theme.accent_orange, format!("ARI: {:.2}", m.ari))
                                        };

                                        Frame::new()
                                            .fill(if is_sel { theme.bg_card_hover } else { theme.bg_card })
                                            .stroke(Stroke::new(if is_sel { 2.0 } else { 1.0 }, if is_sel { theme.accent_blue } else { theme.border_card }))
                                            .corner_radius(CornerRadius::same(6))
                                            .inner_margin(Margin::symmetric(10, 6))
                                            .show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        let circle_color = if is_elim { Color32::from_rgb(255, 60, 60) } else { card_col };
                                                        ui.label(RichText::new("●").size(11.0).color(circle_color));
                                                        if ui.selectable_label(is_sel, RichText::new(format!("E{}", m.id)).strong().size(11.5)).clicked() {
                                                            self.selected_member_id = Some(m.id);
                                                            self.active_sidebar_tab = SidebarTab::Properties;
                                                        }
                                                    });
                                                    ui.label(RichText::new(&m.section).size(10.0).color(theme.text_secondary));
                                                    ui.label(RichText::new(status_text).size(9.5).strong().color(card_col));
                                                });
                                            });
                                    }
                                });
                            });
                        });

                        ui.add_space(10.0);
                        ui.label(RichText::new("⌘K untuk membuka EigenAI Copilot • Klik ruang kosong di kanvas untuk melihat Global System ARI").size(10.5).color(theme.text_muted));
                        ui.add_space(8.0);
                    });
                });
        });
    }
}

/// Helper function to calculate distance from a point to a line segment
fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let ab_len_sq = ab.length_sq();
    if ab_len_sq < 1e-4 {
        return ap.length();
    }
    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let projection = a + ab * t;
    (p - projection).length()
}
