//! build.rs — Pillar 3: 3D CAD/BIM Canvas Viewport, Dedicated Left Sidebar & Floating Dynamic Island AI Copilot (Option B).

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Ui};
use crate::models::{ChatMessage, DatabaseComponent, SidebarTab, StructuralMember};
use crate::theme::MacTheme;

pub struct BuildView {
    pub active_sidebar_tab: SidebarTab,
    pub selected_member_id: Option<usize>,

    // Option B: Floating Dynamic Island AI Copilot State
    pub ai_expanded: bool,
    pub ai_chat_history: Vec<ChatMessage>,
    pub ai_input: String,
}

impl Default for BuildView {
    fn default() -> Self {
        Self {
            active_sidebar_tab: SidebarTab::Properties,
            selected_member_id: Some(1),
            ai_expanded: false,
            ai_chat_history: vec![
                ChatMessage {
                    is_user: false,
                    message: "Welcome to EigenAI Copilot. Ask me to optimize load paths, check SVD mechanism risks, or resize members.".to_string(),
                    time: "10:00 AM".to_string(),
                }
            ],
            ai_input: String::new(),
        }
    }
}

impl BuildView {
    pub fn show(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember], db_components: &[DatabaseComponent]) {
        // Global Keyboard Shortcut: Cmd+K toggles the Floating AI Copilot, Esc closes it
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.ai_expanded = !self.ai_expanded;
        }
        if self.ai_expanded && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.ai_expanded = false;
        }

        ui.horizontal(|ui| {
            // 1. UNIFIED LEFT SIDEBAR (CAD/BIM Modeling & Database Component Inspector)
            self.render_unified_sidebar(ui, theme, members, db_components);

            // 2. 3D VIEWPORT CANVAS & FLOATING DYNAMIC ISLAND COPILOT (Option B)
            self.render_canvas_and_island(ui, theme, members);
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

    fn render_canvas_and_island(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
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
                        ui.add_space(110.0);
                        ui.label(RichText::new("🌐 Interactive 3D BIM / Structural Graph Canvas").size(18.0).strong().color(theme.text_primary));
                        ui.label(RichText::new("Hardware accelerated by wgpu (Metal / Vulkan / DirectX)").size(13.0).color(theme.text_secondary));
                        ui.add_space(6.0);
                        ui.label(RichText::new("[ Orbit: Right-Click Drag | Pan: Middle-Click | Zoom: Scroll ]").size(11.0).color(theme.text_muted));
                        ui.add_space(16.0);

                        // Floating Dynamic Island Copilot (Option B)
                        self.render_floating_copilot(ui, theme, members);
                    });
                });
        });
    }

    /// Renders Option B: Floating Dynamic Island Copilot at the bottom center of the viewport
    fn render_floating_copilot(&mut self, ui: &mut Ui, theme: &MacTheme, members: &[StructuralMember]) {
        if self.ai_expanded {
            // EXPANDED DYNAMIC ISLAND CONSOLE
            theme.floating_island_expanded_frame().show(ui, |ui| {
                ui.set_width(620.0);
                ui.vertical(|ui| {
                    // Header Bar
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🤖 EigenAI Copilot").size(14.0).strong().color(theme.text_primary));
                        ui.add_space(8.0);
                        ui.label(RichText::new("🟢 Active SVD Guard & ARI Engine").size(11.0).color(theme.accent_green));

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(egui::Button::new(RichText::new("⤡ Minimize").size(11.0).color(theme.text_secondary))
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(6))).clicked() {
                                self.ai_expanded = false;
                            }
                            if ui.add(egui::Button::new(RichText::new("🧹 Clear").size(11.0).color(theme.text_muted))
                                .fill(Color32::TRANSPARENT)).clicked() {
                                self.ai_chat_history.clear();
                            }
                        });
                    });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Quick Prompt Action Chips
                    ui.horizontal_wrapped(|ui| {
                        let chips = [
                            ("⚡ Check SVD Mechanism", "Run full singular value decomposition rank verification on the current structure."),
                            ("🎯 Screen Critical ARI", "Screen all members to find the lowest Adjusted Redundancy Index contributions."),
                            ("📐 Warren Truss 24m", "Instantiate a 24m span Warren truss with 6 panels into workspace."),
                            ("🛡️ Collapse Risk Check", "Perform step-by-step alternate load path analysis for critical members."),
                        ];
                        for (chip, query) in chips {
                            if ui.add(egui::Button::new(RichText::new(chip).size(11.0).color(theme.accent_blue))
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(12))).on_hover_text(query).clicked() {
                                self.execute_prompt(query, members);
                            }
                        }
                    });

                    ui.add_space(8.0);

                    // Scrollable Chat Message Stream
                    egui::ScrollArea::vertical().max_height(180.0).stick_to_bottom(true).show(ui, |ui| {
                        for chat in &self.ai_chat_history {
                            if chat.is_user {
                                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                    Frame::new()
                                        .fill(theme.accent_blue)
                                        .corner_radius(CornerRadius::same(10))
                                        .inner_margin(Margin::symmetric(12, 6))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(&chat.message).size(12.0).color(Color32::WHITE));
                                        });
                                });
                            } else {
                                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                                    Frame::new()
                                        .fill(theme.bg_card)
                                        .corner_radius(CornerRadius::same(10))
                                        .stroke(Stroke::new(1.0, theme.border_card))
                                        .inner_margin(Margin::symmetric(12, 8))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(&chat.message).size(12.0).color(theme.text_primary));
                                        });
                                });
                            }
                            ui.add_space(4.0);
                        }
                    });

                    ui.add_space(8.0);

                    // Bottom Prompt Input Bar
                    ui.horizontal(|ui| {
                        let input_response = ui.add(
                            egui::TextEdit::singleline(&mut self.ai_input)
                                .hint_text("Ask EigenAI or type modeling command (e.g. 'check stability')...")
                                .desired_width(ui.available_width() - 85.0)
                        );

                        let enter_pressed = input_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let send_clicked = ui.add(
                            egui::Button::new(RichText::new("Send ↵").size(12.0).strong().color(Color32::WHITE))
                                .fill(theme.accent_blue)
                                .corner_radius(CornerRadius::same(6))
                        ).clicked();

                        if (enter_pressed || send_clicked) && !self.ai_input.trim().is_empty() {
                            let text = self.ai_input.trim().to_string();
                            self.execute_prompt(&text, members);
                            self.ai_input.clear();
                        }
                    });
                });
            });
        } else {
            // COLLAPSED SLEEK CAPSULE PILL
            theme.floating_pill_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚛️ EigenAI").size(13.0).strong().color(theme.accent_blue));
                    ui.add_space(4.0);
                    ui.label(RichText::new("•").size(14.0).color(theme.accent_green));
                    ui.add_space(4.0);

                    // Quick clickable query bar
                    let input_field = ui.add(
                        egui::TextEdit::singleline(&mut self.ai_input)
                            .hint_text("Ask EigenAI Copilot or type command (⌘K)...")
                            .desired_width(280.0)
                    );

                    if input_field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.ai_input.trim().is_empty() {
                        let text = self.ai_input.trim().to_string();
                        self.ai_expanded = true;
                        self.execute_prompt(&text, members);
                        self.ai_input.clear();
                    }

                    // Quick Action Pills
                    if ui.add(egui::Button::new(RichText::new("⚡ SVD Check").size(11.0).color(theme.text_secondary))
                        .fill(theme.bg_card)
                        .corner_radius(CornerRadius::same(12))).clicked() {
                        self.ai_expanded = true;
                        self.execute_prompt("Run SVD check on current model", members);
                    }

                    if ui.add(egui::Button::new(RichText::new("🎯 Screen ARI").size(11.0).color(theme.text_secondary))
                        .fill(theme.bg_card)
                        .corner_radius(CornerRadius::same(12))).clicked() {
                        self.ai_expanded = true;
                        self.execute_prompt("Screen all members for critical ARI values", members);
                    }

                    ui.add_space(6.0);
                    if ui.add(egui::Button::new(RichText::new("⤢ Expand").size(11.0).color(theme.accent_blue))
                        .fill(Color32::TRANSPARENT)).on_hover_text("Open Copilot Console (⌘K)").clicked() {
                        self.ai_expanded = true;
                    }
                });
            });
        }
    }

    fn execute_prompt(&mut self, prompt: &str, members: &[StructuralMember]) {
        self.ai_chat_history.push(ChatMessage {
            is_user: true,
            message: prompt.to_string(),
            time: "Just now".to_string(),
        });

        // Context-aware response simulation
        let response_msg = if prompt.to_lowercase().contains("svd") {
            "✅ [SVD Kinematic Guard]: rank(K) = 14 (Full Rank for 14 active DOFs). Zero null-space kinematic mechanisms detected. Structural stability is mechanically certified.".to_string()
        } else if prompt.to_lowercase().contains("screen") || prompt.to_lowercase().contains("ari") {
            let critical_count = members.iter().filter(|m| m.is_critical || m.ari < 0.7).count();
            format!("🎯 [ARI Screening]: Analyzed {} structural elements. Found {} critical member(s) with ARI < 0.70. Member #1 (Bottom Chord E1) has lowest redundancy contribution (ARI = 0.58). Consider increasing section capacity or adding redundant load paths.", members.len(), critical_count)
        } else if prompt.to_lowercase().contains("truss") || prompt.to_lowercase().contains("warren") {
            "📐 [Parametric Generator]: Warren Truss (24m Span, 6 panels, Height 3.2m) instantiated. Ground support nodes fixed at Node 0 and Node 6.".to_string()
        } else if prompt.to_lowercase().contains("collapse") {
            "🛡️ [Progressive Collapse Safeguard]: Dynamic load redistribution check complete. Reserve capacity is 1.84x above minimum collapse threshold.".to_string()
        } else {
            format!("⚛️ [EigenAI Copilot]: Command '{}' received. Model state verified against SNI 1729 / AISC 360 standards. Geometry and member topology remain consistent.", prompt)
        };

        self.ai_chat_history.push(ChatMessage {
            is_user: false,
            message: response_msg,
            time: "Just now".to_string(),
        });
    }
}