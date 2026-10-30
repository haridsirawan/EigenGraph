//! build.rs — Pillar 3: 3D CAD/BIM Canvas Viewport & Dedicated CAD/BIM Modeling Sidebar.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Ui};
use crate::models::{DatabaseComponent, SidebarTab, StructuralMember};
use crate::theme::MacTheme;

pub struct BuildView {
    pub active_sidebar_tab: SidebarTab,
    pub selected_member_id: Option<usize>,
}

impl Default for BuildView {
    fn default() -> Self {
        Self {
            active_sidebar_tab: SidebarTab::Properties,
            selected_member_id: Some(1),
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

    fn render_properties_tab(&self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
        ui.label(RichText::new("PROPERTIES INSPECTOR").size(12.0).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        if let Some(mid) = self.selected_member_id {
            if let Some(m) = members.iter().find(|item| item.id == mid) {
                theme.card_frame().show(ui, |ui| {
                    ui.label(RichText::new(&m.name).strong().size(15.0).color(theme.text_primary));
                    ui.label(RichText::new(format!("Section Profile: {}", m.section)).size(12.0).color(theme.accent_blue));
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(RichText::new(format!("Axial Force (N_e): {:.1} kN", m.force_kn)).size(12.0).color(theme.text_secondary));
                    ui.label(RichText::new(format!("Capacity (R_e): {:.1} kN", m.capacity_kn)).size(12.0).color(theme.text_secondary));
                    ui.label(RichText::new(format!("Utilization (U*): {:.2}%", m.utilization * 100.0)).size(12.0).color(theme.accent_green));
                    ui.label(RichText::new(format!("SVD Margin (m_e): {:.4}", m.svd_margin)).size(12.0).color(theme.accent_orange));
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

    fn render_layers_tab(&self, ui: &mut Ui, theme: &MacTheme) {
        ui.label(RichText::new("DISPLAY LAYERS").size(12.0).strong().color(theme.text_secondary));
        ui.add_space(6.0);

        let layers = ["☑️ 3D Ground Grid", "☑️ Global Coordinate Axes", "☑️ Element Centroid Wireframe", "☑️ Node IDs & Coordinates", "☑️ Member Axial Force Heatmap", "☐ Deformed Shape Mesh"];
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
                    });
                });

            // 3D Canvas Viewport Container
            Frame::new()
                .fill(theme.bg_canvas)
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .show(ui, |ui| {
                    // 3D Canvas Visual Centerpiece
                    ui.vertical_centered(|ui| {
                        ui.add_space(140.0);
                        ui.label(RichText::new("🌐 Interactive 3D BIM / Structural Graph Canvas").size(18.0).strong().color(theme.text_primary));
                        ui.label(RichText::new("Hardware accelerated by wgpu (Metal / Vulkan / DirectX)").size(13.0).color(theme.text_secondary));
                        ui.add_space(8.0);
                        ui.label(RichText::new("[ Orbit: Right-Click Drag | Pan: Middle-Click | Zoom: Scroll ]").size(11.0).color(theme.text_muted));
                        ui.add_space(20.0);
                        ui.label(RichText::new("Press ⌘K or use the floating EigenAI Copilot below to inspect elements or run SVD").size(11.0).color(theme.accent_blue));
                    });
                });
        });
    }
}