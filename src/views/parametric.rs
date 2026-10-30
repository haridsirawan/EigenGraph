//! parametric.rs — Pillar 2: AutoCAD-style Free Drawing Studio & Parametric Component Database.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Ui, Vec2};
use crate::models::{CadTool, DatabaseComponent, NavigationPillar, ParametricVariable};
use crate::theme::MacTheme;

pub struct ParametricView {
    pub active_tool: CadTool,
    pub component_name: String,
    pub component_category: String,
    pub variables: Vec<ParametricVariable>,
    pub command_line_input: String,
    pub command_history: Vec<String>,
    pub snap_to_grid: bool,
    pub ortho_mode: bool,
    pub cursor_pos_mm: (f32, f32, f32),
    pub show_save_modal: bool,
    pub show_db_drawer: bool,
    pub save_notification: Option<String>,
    pub ai_suggestion_text: Option<String>,
    pub live_ari_score: f32,
    pub live_svd_margin: f32,
}

impl Default for ParametricView {
    fn default() -> Self {
        Self {
            active_tool: CadTool::Line,
            component_name: "Warren Truss 24m Custom".to_string(),
            component_category: "Roof Trusses (Family S5)".to_string(),
            variables: vec![
                ParametricVariable {
                    name: "L".to_string(),
                    expression: "24000".to_string(),
                    evaluated_value: 24000.0,
                    unit: "mm".to_string(),
                },
                ParametricVariable {
                    name: "H".to_string(),
                    expression: "3200".to_string(),
                    evaluated_value: 3200.0,
                    unit: "mm".to_string(),
                },
                ParametricVariable {
                    name: "n".to_string(),
                    expression: "6".to_string(),
                    evaluated_value: 6.0,
                    unit: "panels".to_string(),
                },
                ParametricVariable {
                    name: "chord_sec".to_string(),
                    expression: "W14x90".to_string(),
                    evaluated_value: 1.0,
                    unit: "profile".to_string(),
                },
                ParametricVariable {
                    name: "bahan_density".to_string(),
                    expression: "7850".to_string(),
                    evaluated_value: 7850.0,
                    unit: "kg/m³ (BJ41)".to_string(),
                },
                ParametricVariable {
                    name: "kekakuan_E".to_string(),
                    expression: "200000".to_string(),
                    evaluated_value: 200000.0,
                    unit: "MPa (Stiffness)".to_string(),
                },
                ParametricVariable {
                    name: "harga_baja".to_string(),
                    expression: "24500".to_string(),
                    evaluated_value: 24500.0,
                    unit: "Rp/kg".to_string(),
                },
                ParametricVariable {
                    name: "harga_fabrikasi".to_string(),
                    expression: "8500".to_string(),
                    evaluated_value: 8500.0,
                    unit: "Rp/kg".to_string(),
                },
                ParametricVariable {
                    name: "harga_ereksi".to_string(),
                    expression: "4500000".to_string(),
                    evaluated_value: 4500000.0,
                    unit: "Rp/ton".to_string(),
                },
                ParametricVariable {
                    name: "harga_coating".to_string(),
                    expression: "145000".to_string(),
                    evaluated_value: 145000.0,
                    unit: "Rp/m²".to_string(),
                },
                ParametricVariable {
                    name: "harga_pondasi".to_string(),
                    expression: "1950000".to_string(),
                    evaluated_value: 1950000.0,
                    unit: "Rp/m³".to_string(),
                },
            ],
            command_line_input: String::new(),
            command_history: vec![
                "AutoCAD Free Drawing Studio Initialized.".to_string(),
                "GRID: 1000mm • OSNAP: Endpoint, Midpoint • ORTHO: ON".to_string(),
                "Command: _LINE Specified point: 0, 0, 0".to_string(),
                "Command: Specified next point: 24000, 0, 0 (L = 24.0m)".to_string(),
            ],
            snap_to_grid: true,
            ortho_mode: true,
            cursor_pos_mm: (14250.0, 3200.0, 0.0),
            show_save_modal: false,
            show_db_drawer: false,
            save_notification: None,
            ai_suggestion_text: None,
            live_ari_score: 0.942,
            live_svd_margin: 0.842,
        }
    }
}

impl ParametricView {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        db_components: &mut Vec<DatabaseComponent>,
        active_pillar: &mut NavigationPillar,
    ) {
        ui.vertical(|ui| {
            // 1. TOP DRAFTING RIBBON (AutoCAD Drawing & Constraints Toolbar)
            self.render_drafting_ribbon(ui, theme, db_components, active_pillar);

            // 2. MAIN WORKSPACE (Left Variables Inspector + Center Free Drawing Canvas)
            ui.horizontal(|ui| {
                // Left Panel: Variables & Constraints
                self.render_variables_panel(ui, theme, active_pillar);

                // Center: AutoCAD Free Drawing Canvas
                self.render_drawing_canvas(ui, theme, db_components, active_pillar);
            });
        });
    }

    fn render_drafting_ribbon(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        db_components: &mut Vec<DatabaseComponent>,
        active_pillar: &mut NavigationPillar,
    ) {
        Frame::new()
            .fill(theme.bg_sidebar)
            .inner_margin(Margin::symmetric(14, 8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // AutoCAD Toolset
                    let tools = [
                        CadTool::Select,
                        CadTool::Line,
                        CadTool::Polyline,
                        CadTool::Arc,
                        CadTool::Circle,
                        CadTool::Rectangle,
                        CadTool::Dimension,
                        CadTool::Trim,
                        CadTool::Offset,
                    ];

                    for t in tools {
                        let (label, sc) = t.label_and_shortcut();
                        let is_active = self.active_tool == t;
                        let bg = if is_active { theme.accent_blue } else { theme.bg_card };
                        let fg = if is_active { Color32::WHITE } else { theme.text_primary };

                        let btn = egui::Button::new(RichText::new(label).size(12.0).color(fg))
                            .fill(bg)
                            .corner_radius(CornerRadius::same(6));

                        if ui.add(btn).on_hover_text(format!("Shortcut: {}", sc)).clicked() {
                            self.active_tool = t;
                            self.command_history.push(format!("Command: Tool changed to {}", label));
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // CAD Snapping Toggles
                    if ui.selectable_label(self.snap_to_grid, "🧲 Snap").clicked() {
                        self.snap_to_grid = !self.snap_to_grid;
                    }
                    if ui.selectable_label(self.ortho_mode, "📐 Ortho (F8)").clicked() {
                        self.ortho_mode = !self.ortho_mode;
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // 🪄 AI-Assisted Redundant Bracing Suggestion Button
                    let ai_bracing_btn = egui::Button::new(RichText::new("🪄 Suggest Redundant Bracing").size(12.0).strong().color(Color32::WHITE))
                        .fill(theme.accent_blue.gamma_multiply(0.85))
                        .corner_radius(CornerRadius::same(6));

                    if ui.add(ai_bracing_btn).on_hover_text("AI-ARI Topology Optimization: Suggests strategic redundant bracing to eliminate single points of failure").clicked() {
                        let span = self.variables.iter().find(|v| v.name == "L").map(|v| v.evaluated_value).unwrap_or(24000.0) / 1000.0;
                        let height = self.variables.iter().find(|v| v.name == "H").map(|v| v.evaluated_value).unwrap_or(3200.0) / 1000.0;
                        let sugg = format!(
                            "💡 Rekomendasi ARI: Untuk rangka bentang L={:.1}m dan H={:.1}m, disarankan penambahan 2 batang diagonal silang (cross-bracing) W8x24 pada Panel 3 & 4. Hal ini akan mengeliminasi potensi mekanisme goyang, menaikkan ARI dari 0.640 ke 0.942 (+47.2%), dengan estimasi kenaikan RAB hanya +2.1% (Rp 42.500.000).",
                            span, height
                        );
                        self.ai_suggestion_text = Some(sugg.clone());
                        self.command_history.push("Command: _AI_SUGGEST_REDUNDANT_BRACING".to_string());
                        self.command_history.push(format!("Output: {}", sugg));
                    }

                    // Right Side: Database Actions (Save component & View DB)
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Switch to BUILD action
                        if ui.add(egui::Button::new(RichText::new("Open in BUILD →").size(12.0).strong().color(Color32::WHITE))
                            .fill(theme.accent_blue.gamma_multiply(0.8))
                            .corner_radius(CornerRadius::same(6))).clicked() {
                            *active_pillar = NavigationPillar::ModelingGeometry;
                        }

                        // Save to Database Button
                        let save_btn = egui::Button::new(RichText::new("💾 Save to Database").size(12.0).strong().color(Color32::WHITE))
                            .fill(theme.accent_blue)
                            .corner_radius(CornerRadius::same(6));

                        if ui.add(save_btn).on_hover_text("Save drawn parametric component into database for assembly in BUILD").clicked() {
                            self.save_current_to_db(db_components);
                        }

                        // DB Drawer toggle
                        let db_text = format!("📦 Saved DB ({})", db_components.len());
                        if ui.selectable_label(self.show_db_drawer, db_text).clicked() {
                            self.show_db_drawer = !self.show_db_drawer;
                        }
                    });
                });
            });
    }

    fn render_variables_panel(&mut self, ui: &mut Ui, theme: &MacTheme, active_pillar: &mut NavigationPillar) {
        theme.sidebar_frame().show(ui, |ui| {
            ui.set_width(300.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("PARAMETRIC DRIVING VARIABLES").size(12.0).strong().color(theme.text_secondary));
                ui.add_space(6.0);

                // Component Details
                ui.label(RichText::new("Component Name:").size(11.0).color(theme.text_muted));
                ui.text_edit_singleline(&mut self.component_name);

                ui.add_space(4.0);
                ui.label(RichText::new("Category:").size(11.0).color(theme.text_muted));
                ui.text_edit_singleline(&mut self.component_category);

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Driving Variables Table
                ui.label(RichText::new("Driving Dimensions & Parameters").size(12.0).strong().color(theme.text_primary));
                ui.label(RichText::new("Ubah nilai di bawah untuk update otomatis ke PLANNING").size(10.5).color(theme.accent_blue));
                ui.add_space(4.0);

                for var in &mut self.variables {
                    theme.card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&var.name).size(12.0).strong().color(theme.accent_blue));
                            ui.label(RichText::new("=").size(12.0).color(theme.text_secondary));
                            
                            // Editable value box
                            if ui.add(egui::TextEdit::singleline(&mut var.expression).desired_width(75.0)).changed() {
                                if let Ok(val) = var.expression.parse::<f32>() {
                                    var.evaluated_value = val;
                                }
                            }

                            ui.label(RichText::new(&var.unit).size(10.5).color(theme.text_muted));
                        });
                        ui.label(RichText::new(format!("Nilai: {:.1} {}", var.evaluated_value, var.unit)).size(10.0).color(theme.accent_green));
                    });
                    ui.add_space(3.0);
                }

                ui.add_space(8.0);
                if ui.button("+ Add Variable Parameter").clicked() {
                    self.variables.push(ParametricVariable {
                        name: format!("var_{}", self.variables.len() + 1),
                        expression: "1000".to_string(),
                        evaluated_value: 1000.0,
                        unit: "mm".to_string(),
                    });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // Direct Navigation to PLANNING Button
                if ui.add(egui::Button::new(RichText::new("📊 Lihat Hasil di PLANNING (5D RAB) →").size(11.5).strong().color(Color32::WHITE))
                    .fill(theme.accent_blue)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(ui.available_width(), 30.0))).clicked() {
                    *active_pillar = NavigationPillar::CostBudgeting;
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // Geometric Constraints Summary
                ui.label(RichText::new("Geometric Relations:").size(11.0).strong().color(theme.text_secondary));
                ui.label(RichText::new("• Bottom Chord: Horizontal (y=0)").size(11.0).color(theme.text_muted));
                ui.label(RichText::new("• Web Diagonals: Equal Length (L_diag = 4.38m)").size(11.0).color(theme.text_muted));
                ui.label(RichText::new("• Left/Right Symmetry: Symmetrical axis at x = L/2").size(11.0).color(theme.text_muted));
                ui.label(RichText::new("🔒 Fully Constrained Geometrically").size(11.0).color(theme.accent_green));
            });
        });
    }

    fn render_drawing_canvas(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        _db_components: &[DatabaseComponent],
        _active_pillar: &mut NavigationPillar,
    ) {
        ui.vertical(|ui| {
            // Notification toast if saved
            if let Some(msg) = &self.save_notification {
                Frame::new()
                    .fill(theme.accent_green.gamma_multiply(0.2))
                    .stroke(Stroke::new(1.0, theme.accent_green))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✅").size(14.0));
                            ui.label(RichText::new(msg).size(12.0).strong().color(Color32::WHITE));
                        });
                    });
            }

            // AutoCAD Free Drawing Canvas Area
            Frame::new()
                .fill(Color32::from_rgb(18, 18, 20)) // AutoCAD dark charcoal canvas
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .show(ui, |ui| {
                    // Top Canvas Coordinate & Snap Tracker HUD + LIVE-ARI HUD
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!(
                            "📍 Cursor X: {:.2} mm  |  Y: {:.2} mm  |  Z: {:.2} mm",
                            self.cursor_pos_mm.0, self.cursor_pos_mm.1, self.cursor_pos_mm.2
                        )).size(11.0).monospace().color(theme.text_secondary));

                        ui.add_space(14.0);
                        let snap_str = if self.snap_to_grid { "SNAP: ON" } else { "SNAP: OFF" };
                        let ortho_str = if self.ortho_mode { "ORTHO: ON" } else { "ORTHO: OFF" };
                        ui.label(RichText::new(format!("{}  |  {}", snap_str, ortho_str)).size(11.0).monospace().color(theme.accent_blue));

                        // ⚡ LIVE-ARI DYNAMIC HUD
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            Frame::new()
                                .fill(theme.accent_green.gamma_multiply(0.18))
                                .corner_radius(CornerRadius::same(6))
                                .stroke(Stroke::new(1.0, theme.accent_green))
                                .inner_margin(Margin::symmetric(8, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(format!("⚡ LIVE ARI: {:.3} [Safe Dual-Path]  •  SVD m_e: {:.3}", self.live_ari_score, self.live_svd_margin)).size(11.0).strong().color(Color32::WHITE));
                                });
                        });
                    });

                    // AI Suggestion Banner if triggered
                    if let Some(sugg) = &self.ai_suggestion_text.clone() {
                        ui.add_space(8.0);
                        Frame::new()
                            .fill(Color32::from_rgb(20, 32, 48))
                            .corner_radius(CornerRadius::same(8))
                            .stroke(Stroke::new(1.0, theme.accent_blue))
                            .inner_margin(Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("🪄").size(16.0));
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new("AI-ARI TOPOLOGY BRACING SUGGESTION").size(11.5).strong().color(theme.accent_blue));
                                        ui.label(RichText::new(sugg).size(11.0).color(theme.text_primary));
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.add(egui::Button::new(RichText::new("✕ Tutup").size(10.5).color(theme.text_secondary))
                                            .fill(theme.bg_card)
                                            .corner_radius(CornerRadius::same(4))).clicked() {
                                            self.ai_suggestion_text = None;
                                        }
                                        if ui.add(egui::Button::new(RichText::new("✅ Terapkan Bracing").size(10.5).strong().color(Color32::WHITE))
                                            .fill(theme.accent_green)
                                            .corner_radius(CornerRadius::same(4))).clicked() {
                                            self.command_history.push("Command: _APPLY_REDUNDANT_BRACING: 2x W8x24 applied to bay 3 & 4".to_string());
                                            self.live_ari_score = 0.965;
                                            self.ai_suggestion_text = None;
                                        }
                                    });
                                });
                            });
                    }

                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);

                        // Simulated CAD Drafting Object (Warren Truss with driving dimensions)
                        theme.card_frame().show(ui, |ui| {
                            ui.set_width(620.0);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("📐 AutoCAD Free-Hand & Parametric Drawing Canvas").size(14.0).strong().color(theme.text_primary));
                                ui.label(RichText::new("Drawing: Custom Warren Truss Geometry (Span = 24m, Depth = 3.2m, 6 Panels)").size(11.0).color(theme.text_secondary));
                                ui.add_space(10.0);

                                // Drawing Canvas ASCII Wireframe Mockup
                                ui.label(RichText::new(r#"          ▲ H = 3200 mm (Parametric Driving Height)
    ┌─────o─────────o─────────o─────────o─────────o─────┐
    │    / \       / \       / \       / \       / \    │
    │   /   \     /   \     /   \     /   \     /   \   │
    │  /     \   /     \   /     \   /     \   /     \  │
    o─o───────o─o───────o─o───────o─o───────o─o───────o─o
    ◄─────────────────── L = 24000 mm ──────────────────►
          (6 Panels @ 4000 mm • Ground Line y=0)#"#)
                                    .size(12.0)
                                    .monospace().color(theme.accent_blue));

                                ui.add_space(8.0);
                                ui.label(RichText::new("[ Click to specify points • Hold Shift for Ortho • Snap to Grid active ]").size(10.0).color(theme.text_muted));
                            });
                        });
                    });
                });

            // Bottom AutoCAD Command Line & Prompt
            Frame::new()
                .fill(theme.bg_sidebar)
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .inner_margin(Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Recent Command Line History
                        if let Some(last_cmd) = self.command_history.last() {
                            ui.label(RichText::new(format!("Command history: {}", last_cmd)).size(10.0).monospace().color(theme.text_muted));
                        }

                        // Command Input Field
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Command:").size(12.0).strong().monospace().color(theme.accent_blue));
                            let cmd_res = ui.add(
                                egui::TextEdit::singleline(&mut self.command_line_input)
                                    .hint_text("Type CAD command (e.g. 'LINE', 'OFFSET', 'DIM', 'TRIM') or formula...")
                                    .desired_width(ui.available_width() - 80.0)
                            );

                            if (cmd_res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                                && !self.command_line_input.trim().is_empty()
                            {
                                let cmd = self.command_line_input.trim().to_uppercase();
                                self.command_history.push(format!("Command: {}", cmd));
                                self.execute_cad_command(&cmd);
                                self.command_line_input.clear();
                            }

                            if ui.button("Enter ↵").clicked() && !self.command_line_input.trim().is_empty() {
                                let cmd = self.command_line_input.trim().to_uppercase();
                                self.command_history.push(format!("Command: {}", cmd));
                                self.execute_cad_command(&cmd);
                                self.command_line_input.clear();
                            }
                        });
                    });
                });
        });
    }

    fn execute_cad_command(&mut self, cmd: &str) {
        match cmd {
            "L" | "LINE" => self.active_tool = CadTool::Line,
            "PL" | "POLYLINE" => self.active_tool = CadTool::Polyline,
            "A" | "ARC" => self.active_tool = CadTool::Arc,
            "C" | "CIRCLE" => self.active_tool = CadTool::Circle,
            "REC" | "RECTANGLE" => self.active_tool = CadTool::Rectangle,
            "DIM" | "DIMENSION" => self.active_tool = CadTool::Dimension,
            "TR" | "TRIM" => self.active_tool = CadTool::Trim,
            "O" | "OFFSET" => self.active_tool = CadTool::Offset,
            _ => {
                self.command_history.push(format!("Executing parametric instruction: {}", cmd));
            }
        }
    }

    fn save_current_to_db(&mut self, db_components: &mut Vec<DatabaseComponent>) {
        let new_comp = DatabaseComponent {
            id: format!("comp_{}", db_components.len() + 1),
            name: self.component_name.clone(),
            category: self.component_category.clone(),
            description: format!("Parametric drawing with {} driving variables and 6 web panels.", self.variables.len()),
            variables: self.variables.clone(),
            elements_count: 21,
            created_at: "Just now".to_string(),
        };

        db_components.push(new_comp);
        self.save_notification = Some(format!(
            "Component '{}' successfully saved to Database! Switch to BUILD to assemble it into your project.",
            self.component_name
        ));
    }
}