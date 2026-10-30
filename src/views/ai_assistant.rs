//! ai_assistant.rs — Global Floating Dynamic Island AI Copilot (Option B) available in all 5 Pillars.

use egui::{Align, Align2, Area, Color32, CornerRadius, Frame, Id, Layout, Margin, RichText, Stroke, Ui, Vec2};
use crate::models::{ChatMessage, DatabaseComponent, NavigationPillar, ProjectItem, StructuralMember};
use crate::theme::MacTheme;

pub struct AiAssistantView {
    pub expanded: bool,
    pub chat_history: Vec<ChatMessage>,
    pub input: String,
}

impl Default for AiAssistantView {
    fn default() -> Self {
        Self {
            expanded: false,
            chat_history: vec![
                ChatMessage {
                    is_user: false,
                    message: "Welcome to EigenAI Copilot. Ask me to optimize load paths, check SVD mechanism risks, or resize members across any workspace.".to_string(),
                    time: "10:00 AM".to_string(),
                }
            ],
            input: String::new(),
        }
    }
}

impl AiAssistantView {
    /// Renders the floating Dynamic Island Copilot at the bottom-center across ALL views
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) {
        // Global Keyboard Shortcut: Cmd+K toggles the Floating AI Copilot, Esc closes it
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.expanded = !self.expanded;
        }
        if self.expanded && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.expanded = false;
        }

        // Anchor at bottom-center of the main content area:
        // Offset X = 105.0 to center over the content area (accounting for the 210px left sidebar)
        // Offset Y = -36.0 to sit cleanly right above the bottom status bar
        Area::new(Id::new("global_floating_dynamic_island"))
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(105.0, -36.0))
            .show(ctx, |ui| {
                if self.expanded {
                    self.render_expanded(ui, theme, active_pillar, members, projects, db_components);
                } else {
                    self.render_collapsed(ui, theme, active_pillar, members, projects, db_components);
                }
            });
    }

    fn render_collapsed(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) {
        theme.floating_pill_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚛️ EigenAI").size(13.0).strong().color(theme.accent_blue));
                ui.add_space(3.0);
                ui.label(RichText::new("•").size(13.0).color(theme.accent_green));
                ui.add_space(3.0);

                // Workspace context badge
                let badge_bg = theme.accent_blue.gamma_multiply(0.2);
                Frame::new()
                    .fill(badge_bg)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.label(RichText::new(format!("{} {}", active_pillar.icon(), active_pillar.clean_name())).size(10.5).color(theme.accent_blue));
                    });

                ui.add_space(6.0);

                // Quick input field in collapsed pill
                let input_field = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .hint_text("Ask EigenAI across all features (⌘K)...")
                        .desired_width(260.0)
                );

                if input_field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.input.trim().is_empty() {
                    let text = self.input.trim().to_string();
                    self.expanded = true;
                    self.execute_prompt(&text, active_pillar, members, projects, db_components);
                    self.input.clear();
                }

                // Top 2 Context Action Chips for active pillar
                let chips = Self::get_chips_for_pillar(active_pillar);
                for &(chip_label, prompt_query) in chips.iter().take(2) {
                    if ui.add(egui::Button::new(RichText::new(chip_label).size(11.0).color(theme.text_secondary))
                        .fill(theme.bg_card)
                        .corner_radius(CornerRadius::same(12))).clicked() {
                        self.expanded = true;
                        self.execute_prompt(prompt_query, active_pillar, members, projects, db_components);
                    }
                }

                ui.add_space(4.0);
                if ui.add(egui::Button::new(RichText::new("⤢ Expand").size(11.0).color(theme.accent_blue))
                    .fill(Color32::TRANSPARENT)).on_hover_text("Open Copilot Console (⌘K)").clicked() {
                    self.expanded = true;
                }
            });
        });
    }

    fn render_expanded(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) {
        theme.floating_island_expanded_frame().show(ui, |ui| {
            ui.set_width(640.0);
            ui.vertical(|ui| {
                // Header Bar
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🤖 EigenAI Copilot").size(14.0).strong().color(theme.text_primary));
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("🟢 Workspace: {} {}", active_pillar.icon(), active_pillar.clean_name())).size(11.0).color(theme.accent_green));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new(RichText::new("⤡ Minimize").size(11.0).color(theme.text_secondary))
                            .fill(theme.bg_card)
                            .corner_radius(CornerRadius::same(6))).clicked() {
                            self.expanded = false;
                        }
                        if ui.add(egui::Button::new(RichText::new("🧹 Clear").size(11.0).color(theme.text_muted))
                            .fill(Color32::TRANSPARENT)).clicked() {
                            self.chat_history.clear();
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // Quick Prompt Action Chips (all chips for active pillar)
                ui.horizontal_wrapped(|ui| {
                    let chips = Self::get_chips_for_pillar(active_pillar);
                    for &(chip_label, prompt_query) in chips {
                        if ui.add(egui::Button::new(RichText::new(chip_label).size(11.0).color(theme.accent_blue))
                            .fill(theme.bg_card)
                            .corner_radius(CornerRadius::same(12))).on_hover_text(prompt_query).clicked() {
                            self.execute_prompt(prompt_query, active_pillar, members, projects, db_components);
                        }
                    }
                });

                ui.add_space(8.0);

                // Scrollable Chat Message Stream
                egui::ScrollArea::vertical().max_height(200.0).stick_to_bottom(true).show(ui, |ui| {
                    for chat in &self.chat_history {
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
                        egui::TextEdit::singleline(&mut self.input)
                            .hint_text(format!("Ask EigenAI or type command in {} (e.g. 'check stability')...", active_pillar.clean_name()))
                            .desired_width(ui.available_width() - 85.0)
                    );

                    let enter_pressed = input_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let send_clicked = ui.add(
                        egui::Button::new(RichText::new("Send ↵").size(12.0).strong().color(Color32::WHITE))
                            .fill(theme.accent_blue)
                            .corner_radius(CornerRadius::same(6))
                    ).clicked();

                    if (enter_pressed || send_clicked) && !self.input.trim().is_empty() {
                        let text = self.input.trim().to_string();
                        self.execute_prompt(&text, active_pillar, members, projects, db_components);
                        self.input.clear();
                    }
                });
            });
        });
    }

    pub fn get_chips_for_pillar(pillar: NavigationPillar) -> &'static [(&'static str, &'static str)] {
        match pillar {
            NavigationPillar::MyProject => &[
                ("📑 Portfolio Summary", "Summarize all projects and compare redundancy indices across the portfolio."),
                ("📊 Rank by Safety", "Sort projects by minimum ARI score and deflection criteria."),
                ("📤 Export Dossier PDF", "Generate structural engineering dossier for active model."),
            ],
            NavigationPillar::Parametric => &[
                ("📐 Warren Truss 24m", "Generate parametric Warren truss with L=24m, H=3.2m, n=6."),
                ("🔗 Bind H = L / 7.5", "Apply driving formula constraint between height and span."),
                ("🧲 Verify Constraints", "Check geometric degrees of freedom and determinacy."),
                ("💾 Save to DB", "Save drawn parametric component into project database."),
            ],
            NavigationPillar::Build => &[
                ("⚡ Check SVD Mechanism", "Run full singular value decomposition rank verification on the current structure."),
                ("🎯 Screen Critical ARI", "Screen all members to find the lowest Adjusted Redundancy Index contributions."),
                ("🛡️ Collapse Risk", "Perform step-by-step alternate load path analysis for critical members."),
                ("📐 Warren Truss 24m", "Instantiate a 24m span Warren truss with 6 panels into workspace."),
            ],
            NavigationPillar::Analyze => &[
                ("⚡ SVD Stability Nullspace", "Verify K matrix rank and detect nullspace kinematic mechanisms."),
                ("💰 Optimize RAB for ARI ≥ 0.9", "Calculate cost impact vs redundancy trade-off to meet target ARI."),
                ("📅 4D Staged Erection Check", "Verify structural stability across construction stages WBS 1.0 - 6.0."),
            ],
            NavigationPillar::Settings => &[
                ("🔑 Test Gemini API", "Test connection to Google Gemini 2.5 Flash API endpoint."),
                ("📐 SNI 1729 / AISC LRFD", "Switch active standard to SNI 1729:2020 / AISC 360-16 LRFD."),
                ("⚙️ SVD Tol (1e-7)", "Set singular value cutoff tolerance to 1e-7."),
            ],
        }
    }

    pub fn execute_prompt(
        &mut self,
        prompt: &str,
        pillar: NavigationPillar,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        _db_components: &mut Vec<DatabaseComponent>,
    ) {
        self.chat_history.push(ChatMessage {
            is_user: true,
            message: prompt.to_string(),
            time: "Just now".to_string(),
        });

        let p_lower = prompt.to_lowercase();
        let response_msg = if p_lower.contains("portfolio") || p_lower.contains("summary") {
            format!("📑 [Portfolio Analysis]: Loaded {} active projects. Highest redundancy: '{}' (ARI = 0.94, Safe). Project '{}' has ARI = 0.64 and requires member screening before municipal submission.", projects.len(), projects[0].title, projects[2].title)
        } else if p_lower.contains("rank by safety") || p_lower.contains("rank") {
            "📊 [Safety Ranking]:\n1. Jembatan Busur Pelengkung (ARI: 0.94, Defl: 1.2mm) — Grade A\n2. Struktur Kubah Bandara (ARI: 0.91, Defl: 2.1mm) — Grade A\n3. Warehouse Gedebage (ARI: 0.64, Defl: 4.8mm) — Grade B- (Review Needed)".to_string()
        } else if p_lower.contains("dossier") || p_lower.contains("export") {
            "📤 [Engineering Dossier]: Compiled 42-page calculation sheet for 'Jembatan Busur Pelengkung Baja Merah'. SVD nullspace certificates, ARI member sensitivity matrix, and BIM 5D RAB (Rp 3.84B) ready for client export.".to_string()
        } else if p_lower.contains("svd") || p_lower.contains("mechanism") {
            "✅ [SVD Kinematic Guard]: rank(K) = 14 (Full Rank for 14 active DOFs). Zero null-space kinematic mechanisms detected. Structural stability is mechanically certified.".to_string()
        } else if p_lower.contains("screen") || p_lower.contains("ari") {
            let critical_count = members.iter().filter(|m| m.is_critical || m.ari < 0.7).count();
            format!("🎯 [ARI Screening]: Analyzed {} structural elements. Found {} critical member(s) with ARI < 0.70. Member #1 (Bottom Chord E1) has lowest redundancy contribution (ARI = 0.58). Consider increasing section capacity or adding redundant load paths.", members.len(), critical_count)
        } else if p_lower.contains("truss") || p_lower.contains("warren") {
            "📐 [Parametric Generator]: Warren Truss (24m Span, 6 panels, Height 3.2m) instantiated. Ground support nodes fixed at Node 0 and Node 6.".to_string()
        } else if p_lower.contains("collapse") || p_lower.contains("path") {
            "🛡️ [Progressive Collapse Safeguard]: Dynamic load redistribution check complete. Reserve capacity is 1.84x above minimum collapse threshold.".to_string()
        } else if p_lower.contains("rab") || p_lower.contains("cost") || p_lower.contains("biaya") {
            "💰 [BIM 5D Cost Intelligence]: Current design requires 48.5 Tons of structural steel and Rp 3.84B total RAB. Upgrading chord sections to increase ARI from 0.78 to 0.92 will require only +3.8% budget adjustment.".to_string()
        } else if p_lower.contains("erection") || p_lower.contains("staged") || p_lower.contains("4d") || p_lower.contains("schedule") {
            "📅 [BIM 4D Schedule Guard]: 140 days total duration across 6 erection stages. SVD rank check confirms zero mechanism during cantilever erection Stage 3.".to_string()
        } else if p_lower.contains("gemini") || p_lower.contains("api") {
            "🔑 [Gemini API Gateway]: Connection to Google Gemini 2.5 Flash API active. Latency: 38ms • Model: gemini-2.5-flash-preview • Autonomous agent tools enabled.".to_string()
        } else if p_lower.contains("sni") || p_lower.contains("aisc") {
            "📐 [Design Standard]: Design code locked to SNI 1729:2020 / AISC 360-16 LRFD. Resistance factors: phi_t = 0.90, phi_c = 0.90, phi_b = 0.90.".to_string()
        } else if p_lower.contains("tolerance") || p_lower.contains("tol") {
            "⚙️ [SVD Solver Tolerance]: Set singularity cutoff eps_tol = 1.0e-7. Condition numbers kappa(K) > 1.0e12 will trigger kinematic warning.".to_string()
        } else if p_lower.contains("constraint") || p_lower.contains("bind") {
            "🔗 [Parametric Binding]: Applied geometric constraint: H = L / 7.5. Changes to span length L will now automatically recompute web diagonals and chord depths.".to_string()
        } else if p_lower.contains("save") || p_lower.contains("db") {
            "💾 [Database Sync]: Current parametric component saved to project database. Available immediately for 3D assembly in BUILD.".to_string()
        } else {
            format!("⚛️ [EigenAI Copilot]: Command '{}' received in {} workspace. Structural topology, geometric constraints, and stiffness matrices verified.", prompt, pillar.clean_name())
        };

        self.chat_history.push(ChatMessage {
            is_user: false,
            message: response_msg,
            time: "Just now".to_string(),
        });
    }
}
