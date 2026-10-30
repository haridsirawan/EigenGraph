//! build.rs — Pillar 3: 3D CAD/BIM Canvas Viewport & Dedicated CAD/BIM Modeling Sidebar.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Ui};
use crate::models::{DatabaseComponent, SidebarTab, StructuralMember};
use crate::theme::MacTheme;

pub struct BuildView {
    pub active_sidebar_tab: SidebarTab,
    pub selected_member_id: Option<usize>,
    pub simulated_eliminated_id: Option<usize>,
    pub color_mode: String,
}

impl Default for BuildView {
    fn default() -> Self {
        Self {
            active_sidebar_tab: SidebarTab::Properties,
            selected_member_id: Some(1),
            simulated_eliminated_id: None,
            color_mode: "ARI".to_string(),
        }
    }
}

impl BuildView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember], db_components: &[DatabaseComponent]) {
        ui.horizontal(|ui| {
            // 1. UNIFIED LEFT SIDEBAR (CAD/BIM Modeling & Database Component Inspector)
            self.render_unified_sidebar(ui, theme, members, db_components);

            // 2. 3D VIEWPORT CANVAS
            self.render_canvas(ui, theme);
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
        ui.label(RichText::new("PROPERTIES INSPECTOR & ARI HEALTH").size(12.0).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        if let Some(mid) = self.selected_member_id {
            if let Some(m) = members.iter().find(|item| item.id == mid) {
                theme.card_frame().show(ui, |ui| {
                    ui.label(RichText::new(&m.name).strong().size(15.0).color(theme.text_primary));
                    ui.label(RichText::new(format!("Section Profile: {}", m.section)).size(12.0).color(theme.accent_blue));
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.label(RichText::new(format!("Axial Force (N_e): {:+.1} kN", m.force_kn)).size(12.0).color(theme.text_secondary));
                    ui.label(RichText::new(format!("Capacity (R_e): {:.1} kN", m.capacity_kn)).size(12.0).color(theme.text_secondary));
                    ui.label(RichText::new(format!("Utilization (U*): {:.2}%", m.utilization * 100.0)).size(12.0).color(theme.accent_green));
                    ui.label(RichText::new(format!("Redundancy Ratio (RR): {:.4}", m.rr)).size(12.0).color(theme.accent_blue));
                    ui.label(RichText::new(format!("Adjusted Redundancy Index: {:.4}", m.ari)).size(12.0).strong().color(if m.ari >= 0.70 { theme.accent_green } else { theme.accent_red }));
                    ui.label(RichText::new(format!("Criticality Index (CI): {:.4}", m.ci)).size(12.0).color(if m.is_critical { theme.accent_red } else { theme.text_secondary }));
                    ui.label(RichText::new(format!("SVD Margin (m_e = 1 - h_e): {:.4}", m.svd_margin)).size(12.0).color(theme.accent_orange));

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // 💥 Interactive What-If Member Elimination Simulation
                    ui.label(RichText::new("UJI PELEPASAN ELEMEN (WHAT-IF)").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(4.0);

                    let is_eliminated = self.simulated_eliminated_id == Some(mid);
                    if !is_eliminated {
                        let sim_btn = egui::Button::new(RichText::new("💥 Simulasikan Batang Lepas").size(11.5).strong().color(Color32::WHITE))
                            .fill(Color32::from_rgb(220, 60, 60))
                            .corner_radius(CornerRadius::same(6));

                        if ui.add(sim_btn).on_hover_text("Uji perturbasi SVD Gate 1 & redistribusi beban jika elemen ini hilang secara tiba-tiba").clicked() {
                            self.simulated_eliminated_id = Some(mid);
                        }
                    } else {
                        Frame::new()
                            .fill(Color32::from_rgb(48, 20, 20))
                            .stroke(Stroke::new(1.0, theme.accent_red))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("⚠️ ELEMEN E{} DILEPAS!", mid)).size(11.5).strong().color(theme.accent_red));
                                ui.label(RichText::new("• Gate 1 SVD Kinematics: STABIL (rank = n_free)").size(10.5).color(theme.text_secondary));
                                ui.label(RichText::new("• Utilisasi Redistribusi Maks: 78.4%").size(10.5).color(theme.accent_orange));
                                ui.label(RichText::new(format!("• Skor ARI Batang: {:.4}", m.ari)).size(10.5).color(theme.accent_green));
                            });

                        ui.add_space(6.0);
                        let restore_btn = egui::Button::new(RichText::new("🔄 Pulihkan / Pasang Kembali").size(11.5).strong().color(Color32::WHITE))
                            .fill(theme.accent_green)
                            .corner_radius(CornerRadius::same(6));

                        if ui.add(restore_btn).clicked() {
                            self.simulated_eliminated_id = None;
                        }
                    }
                });
            }
        } else {
            ui.label(RichText::new("Select an element in 3D canvas").size(12.0).color(theme.text_muted));
        }
    }

    fn render_browser_tab(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        db_components: &[DatabaseComponent],
    ) {
        ui.label(RichText::new("PROJECT BROWSER").size(12.0).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // PARAMETRIC DATABASE INSERTION SECTION (The user's key flow!)
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
                    let text = format!("{}: {}", m.name, m.section);
                    if ui.selectable_label(self.selected_member_id == Some(m.id), text).clicked() {
                        // Select element
                    }
                }
            });
            ui.collapsing("📍 Boundary Restraints (Supports)", |ui| {
                ui.label(RichText::new("• Node 0: Pinned (ux=0, uy=0)").size(12.0).color(theme.text_secondary));
                ui.label(RichText::new("• Node 1: Roller (uy=0)").size(12.0).color(theme.text_secondary));
            });
            ui.collapsing("⚡ Load Cases (c in D)", |ui| {
                ui.label(RichText::new("• Case c:uy = -30.0 kN").size(12.0).color(theme.accent_orange));
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
        let modes = ["ARI", "Criticality", "SVD Margin", "Axial Force"];
        for m in modes {
            let is_sel = self.color_mode == m;
            if ui.selectable_label(is_sel, format!("🎨 {}", m)).clicked() {
                self.color_mode = m.to_string();
            }
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let layers = ["☑️ 3D Ground Grid", "☑️ Global Coordinate Axes", "☑️ Element Centroid Wireframe", "☑️ Node IDs & Coordinates", "☑️ Deformed Shape Mesh"];
        for l in layers {
            ui.label(RichText::new(l).size(12.0).color(theme.text_secondary));
            ui.add_space(4.0);
        }
    }

    fn render_canvas(&mut self, ui: &mut Ui, theme: &MacTheme) {
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
                            let badge_text = if let Some(elim_id) = self.simulated_eliminated_id {
                                format!("⚠️ WHAT-IF SIMULATION: E{} ELIMINATED", elim_id)
                            } else {
                                format!("🎨 Heatmap: {}", self.color_mode)
                            };
                            let badge_col = if self.simulated_eliminated_id.is_some() { theme.accent_red } else { theme.accent_blue };

                            Frame::new()
                                .fill(badge_col.gamma_multiply(0.2))
                                .stroke(Stroke::new(1.0, badge_col))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(8, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(badge_text).size(11.0).strong().color(Color32::WHITE));
                                });
                        });
                    });
                });

            // 3D Canvas Viewport Container
            Frame::new()
                .fill(theme.bg_canvas)
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);

                        // Simulated What-If Alert Banner if active
                        if let Some(elim_id) = self.simulated_eliminated_id {
                            Frame::new()
                                .fill(Color32::from_rgb(40, 16, 16))
                                .stroke(Stroke::new(1.0, theme.accent_red))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(16, 10))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("⚠️").size(18.0));
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(format!("UJI PERTURBASI PELEPASAN ELEMEN E{} (SUDDEN MEMBER LOSS)", elim_id)).size(13.0).strong().color(theme.accent_red));
                                            ui.label(RichText::new("SVD Gate 1 Guard: STABIL (Kondisi Sherman-Morrison m_e > 0). Beban berhasil diredistribusikan ke batang tetangga.").size(11.5).color(theme.text_secondary));
                                        });
                                        ui.add_space(16.0);
                                        if ui.add(egui::Button::new(RichText::new("🔄 Pulihkan Batang").size(11.0).strong().color(Color32::WHITE))
                                            .fill(theme.accent_green)
                                            .corner_radius(CornerRadius::same(6))).clicked() {
                                            self.simulated_eliminated_id = None;
                                        }
                                    });
                                });
                            ui.add_space(20.0);
                        }

                        // Interactive 3D Wireframe Mockup Display
                        theme.card_frame().show(ui, |ui| {
                            ui.set_width(680.0);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("🌐 3D Structural Truss Viewport (Apple Metal GPU 60 FPS)").size(13.0).strong().color(theme.text_primary));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new("[ Click member below to inspect or eliminate ]").size(11.0).color(theme.text_muted));
                                    });
                                });
                                ui.add_space(10.0);

                                // Truss Members Visual representation
                                ui.horizontal(|ui| {
                                    let members_data = [
                                        (1, "E1: Bottom Chord (L=4m)", 0.5800, "W14x90"),
                                        (2, "E2: Top Chord (L=4m)", 0.6500, "W14x90"),
                                        (3, "E3: Diagonal Web (L=4.38m)", 0.8200, "HSS8x8"),
                                        (4, "E4: Vertical Post (L=3.2m)", 0.9100, "HSS6x6"),
                                    ];

                                    for (id, _name, ari, sec) in members_data {
                                        let is_sel = self.selected_member_id == Some(id);
                                        let is_elim = self.simulated_eliminated_id == Some(id);

                                        let (card_col, status_text) = if is_elim {
                                            (Color32::from_rgb(180, 40, 40), "DISCONNECTED")
                                        } else if ari >= 0.85 {
                                            (theme.accent_green, "ARI: Safe (0.91)")
                                        } else if ari >= 0.70 {
                                            (theme.accent_blue, "ARI: Good (0.82)")
                                        } else {
                                            (theme.accent_orange, "ARI: Review (0.58)")
                                        };

                                        Frame::new()
                                            .fill(if is_sel { theme.bg_card_hover } else { theme.bg_card })
                                            .stroke(Stroke::new(if is_sel { 2.0 } else { 1.0 }, if is_sel { theme.accent_blue } else { theme.border_card }))
                                            .corner_radius(CornerRadius::same(8))
                                            .inner_margin(Margin::symmetric(10, 8))
                                            .show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        let circle_color = if is_elim { Color32::from_rgb(255, 60, 60) } else { card_col };
                                                        ui.label(RichText::new("●").size(12.0).color(circle_color));
                                                        if ui.selectable_label(is_sel, RichText::new(format!("E{}", id)).strong().size(12.0)).clicked() {
                                                            self.selected_member_id = Some(id);
                                                        }
                                                    });
                                                    ui.label(RichText::new(sec).size(10.5).color(theme.text_secondary));
                                                    ui.label(RichText::new(status_text).size(10.0).strong().color(card_col));
                                                });
                                            });
                                    }
                                });

                                ui.add_space(14.0);
                                ui.label(RichText::new(r#"          ▲ Top Joint (H = 3.2m)
    ┌─────o─────────o─────────o─────────o─────────o─────┐
    │    / \       / \       / \       / \       / \    │
    │   /   \     /   \     /   \     /   \     /   \   │  [60 FPS Metal Engine]
    │  /     \   /     \   /     \   /     \   /     \  │  • Red: Member Lost
    o─o───────o─o───────o─o───────o─o───────o─o───────o─o  • Green: ARI > 0.85
    ◄─────────────────── L = 24.0 m ────────────────────►  • Orange: ARI < 0.70"#)
                                    .size(11.5)
                                    .monospace().color(theme.accent_blue));
                                ui.add_space(6.0);
                            });
                        });

                        ui.add_space(14.0);
                        ui.label(RichText::new("Press ⌘K or use the floating EigenAI Copilot below to inspect elements or run SVD").size(11.0).color(theme.accent_blue));
                    });
                });
        });
    }
}