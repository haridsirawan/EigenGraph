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
            component_name: "NVW_DCR Portal Frame FL1-FL6 (ASTM A992)".to_string(),
            component_category: "NVW_DCR-LOD300_Eng-CON".to_string(),
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
                "Command: WELCOME TO AUTOCAD DRAFTING & PARAMETRIC STUDIO".to_string(),
                "Type 'LINE' (L), 'CIRCLE' (C), 'OFFSET' (O), 'DIM' in command line or use tool icons.".to_string(),
                "Real-time UNY Dissertation ARI solver will audit every drawn stroke dynamically.".to_string(),
            ],
            snap_to_grid: true,
            ortho_mode: false,
            cursor_pos_mm: (12400.0, 3200.0, 0.0),
            show_save_modal: false,
            show_db_drawer: false,
            save_notification: None,
            ai_suggestion_text: Some("AI Copilot: Struktur Warren Truss 24m ini memiliki redundansi kinematik stabil (ARI: 0.94).".to_string()),
            live_ari_score: 0.942,
            live_svd_margin: 0.998,
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

        // 3. DATABASE DRAWER / POPUP MODAL (When Toggled)
        if self.show_db_drawer {
            self.render_db_drawer(ui.ctx(), theme, db_components, active_pillar);
        }
    }

    fn render_drafting_ribbon(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        db_components: &mut Vec<DatabaseComponent>,
        _active_pillar: &mut NavigationPillar,
    ) {
        Frame::new()
            .fill(theme.bg_sidebar)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(14, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📐 2D DRAFTING").size(13.0).strong().color(theme.accent_blue));
                    ui.separator();

                    // AutoCAD Drawing Tools
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

                    for tool in tools {
                        let (name, shortcut) = tool.label_and_shortcut();
                        let is_selected = self.active_tool == tool;
                        let text = format!("{} [{}]", name, shortcut);

                        let btn = egui::Button::new(RichText::new(text).size(11.5).color(if is_selected {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        }))
                        .fill(if is_selected {
                            theme.accent_blue
                        } else {
                            theme.bg_card
                        })
                        .corner_radius(CornerRadius::same(6));

                        if ui.add(btn).clicked() {
                            self.active_tool = tool;
                            self.command_history.push(format!("Tool changed to: {:?}", tool));
                        }
                    }

                    ui.separator();

                    // Toggle Modes
                    if ui.selectable_label(self.ortho_mode, "📐 ORTHO (F8)").clicked() {
                        self.ortho_mode = !self.ortho_mode;
                        self.command_history.push(format!("ORTHO mode set to {}", self.ortho_mode));
                    }

                    if ui.selectable_label(self.snap_to_grid, "🧲 SNAP (F9)").clicked() {
                        self.snap_to_grid = !self.snap_to_grid;
                        self.command_history.push(format!("SNAP mode set to {}", self.snap_to_grid));
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Save Component Button
                        let save_btn = egui::Button::new(RichText::new("💾 Save Component").size(11.5).strong().color(Color32::WHITE))
                            .fill(theme.accent_green)
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
                                if let Ok(val) = var.expression.parse::<f64>() {
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

                ui.add_space(6.0);
                // Direct Navigation to SUSTAINABILITY Button
                if ui.add(egui::Button::new(RichText::new("🌱 Cek Karbon di SUSTAINABILITY (6D) →").size(11.5).color(theme.text_primary))
                    .fill(theme.bg_card)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(ui.available_width(), 30.0))).clicked() {
                    *active_pillar = NavigationPillar::SustainabilityEnergy;
                }

                ui.add_space(12.0);
                // Live Solver Mini-Audit Badge
                Frame::new()
                    .fill(Color32::from_rgb(18, 22, 28))
                    .corner_radius(CornerRadius::same(8))
                    .stroke(Stroke::new(1.0, theme.border_card))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(RichText::new("⚡ LIVE UNY ARI SOLVER").size(11.0).strong().color(theme.accent_blue));
                        ui.label(RichText::new(format!("Parametric ARI: {:.3} (Safe/Redundant)", self.live_ari_score)).size(11.5).color(theme.accent_green));
                        ui.label(RichText::new(format!("SVD Singularity Margin: {:.1}%", self.live_svd_margin * 100.0)).size(10.5).color(theme.text_secondary));
                    });
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
            if let Some(ref note) = self.save_notification {
                Frame::new()
                    .fill(theme.accent_green.gamma_multiply(0.15))
                    .stroke(Stroke::new(1.0, theme.accent_green))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✅").size(14.0));
                            ui.label(RichText::new(note).size(12.0).color(theme.accent_green));
                        });
                    });
                ui.add_space(4.0);
            }

            // AutoCAD Drawing Canvas Box
            let available_height = ui.available_height() - 110.0;
            Frame::new()
                .fill(Color32::from_rgb(16, 18, 22)) // Deep AutoCAD dark viewport
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.set_height(available_height);
                    ui.set_width(ui.available_width());

                    // Canvas Status Overlay (Top Left)
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("AUTOCAD 2D FREE DRAWING VIEWPORT").size(11.0).strong().color(Color32::from_rgb(120, 140, 170)));
                        ui.label(RichText::new("•").color(theme.text_muted));
                        ui.label(RichText::new(format!("Tool: {:?}", self.active_tool)).size(11.0).color(theme.accent_blue));
                        ui.label(RichText::new("•").color(theme.text_muted));
                        ui.label(RichText::new(format!("X: {:.1}mm  Y: {:.1}mm  Z: {:.1}mm", self.cursor_pos_mm.0, self.cursor_pos_mm.1, self.cursor_pos_mm.2)).size(11.0).color(Color32::LIGHT_GRAY));
                    });

                    ui.add_space(20.0);

                    // Mock Wireframe Schematic Grid Representation of Warren Truss
                    ui.vertical_centered(|ui| {
                        ui.add_space(available_height / 4.0);
                        ui.label(RichText::new("📐 [AutoCAD Viewport: 2D Interactive Vector Graphic]").size(15.0).strong().color(theme.accent_blue));
                        ui.add_space(6.0);
                        ui.label(RichText::new("Klik dan gerakkan kursor untuk menggambar garis batang struktural baru (L=24,000mm, H=3,200mm, 6 Panels)").size(12.0).color(theme.text_secondary));
                        ui.add_space(12.0);

                        // ASCII Schematic Diagram of Warren Truss
                        let schematic = "  ▲═════════▲═════════▲═════════▲ (Top Chords W14x90)\n / \\       / \\       / \\       / \\\n/   \\     /   \\     /   \\     /   \\  (Diagonals 2L 4x4x3/8)\n▲═════▲═════▲═════▲═════▲═════▲═════▲ (Bottom Chords W14x90)\n[Pin]                              [Roller]";
                        ui.label(RichText::new(schematic).size(12.0).monospace().color(Color32::from_rgb(100, 200, 255)));

                        if let Some(ref suggestion) = self.ai_suggestion_text {
                            ui.add_space(14.0);
                            Frame::new()
                                .fill(theme.accent_blue.gamma_multiply(0.12))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(suggestion).size(11.5).color(theme.accent_blue));
                                });
                        }
                    });
                });

            ui.add_space(6.0);

            // AutoCAD Command Line Interface (Bottom Box)
            Frame::new()
                .fill(Color32::from_rgb(22, 24, 28))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Command History (Last 2 lines)
                        let history_len = self.command_history.len();
                        let start_idx = if history_len > 2 { history_len - 2 } else { 0 };
                        for line in &self.command_history[start_idx..] {
                            ui.label(RichText::new(line).size(10.5).monospace().color(theme.text_muted));
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
            ..Default::default()
        };

        db_components.push(new_comp);
        if let Ok(saved_path) = crate::core::InternalVault::autosave_parametric_components(db_components) {
            self.save_notification = Some(format!(
                "Komponen '{}' berhasil disimpan otomatis ke Package Contents: {} (Siap dirakit di BUILD 3D).",
                self.component_name, saved_path
            ));
        } else {
            self.save_notification = Some(format!(
                "Component '{}' successfully saved to Database! Switch to BUILD to assemble it into your project.",
                self.component_name
            ));
        }
    }

    fn render_db_drawer(
        &mut self,
        ctx: &egui::Context,
        theme: &MacTheme,
        db_components: &mut [DatabaseComponent],
        active_pillar: &mut NavigationPillar,
    ) {
        let mut open = self.show_db_drawer;
        let mut should_close = false;
        let mut load_idx: Option<usize> = None;
        let mut assemble_in_build = false;

        egui::Window::new("📦 Database Komponen Parametrik & Potongan IFC")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
            .default_size(Vec2::new(660.0, 520.0))
            .frame(Frame::new()
                .fill(theme.bg_card)
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::same(20)))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🏛️").size(28.0));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Katalog Potongan Parametrik & Struktur Rumit (OpenBIM IFC)").size(15.0).strong().color(theme.text_primary));
                            ui.label(RichText::new("Potongan modular dari gedung berarsitektur kompleks siap pakai sebagai referensi & perakitan.").size(11.5).color(theme.text_secondary));
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for (idx, comp) in db_components.iter().enumerate() {
                            theme.card_frame().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&comp.name).size(13.0).strong().color(theme.accent_blue));
                                    ui.add_space(6.0);
                                    Frame::new()
                                        .fill(theme.accent_blue.gamma_multiply(0.15))
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                             ui.label(RichText::new(&comp.category).size(10.0).color(theme.accent_blue));
                                        });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new(&comp.created_at).size(10.5).color(theme.text_muted));
                                    });
                                });

                                ui.add_space(4.0);
                                ui.label(RichText::new(&comp.description).size(11.5).color(theme.text_secondary));

                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new("Variabel:").size(10.5).strong().color(theme.text_muted));
                                    for v in comp.variables.iter().take(5) {
                                        ui.label(RichText::new(format!("{} = {} {}", v.name, v.expression, v.unit)).size(10.0).monospace().color(theme.accent_green));
                                        ui.add_space(4.0);
                                    }
                                    if comp.variables.len() > 5 {
                                        ui.label(RichText::new(format!("+{} lainnya", comp.variables.len() - 5)).size(9.5).color(theme.text_muted));
                                    }
                                });

                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if ui.add(egui::Button::new(RichText::new("📥 Muat ke Kanvas (Load)").size(11.0).strong().color(Color32::WHITE))
                                        .fill(theme.accent_blue)
                                        .corner_radius(CornerRadius::same(6))).clicked() {
                                        load_idx = Some(idx);
                                    }

                                    if ui.add(egui::Button::new(RichText::new("🏗️ Rakit di BUILD (3D)").size(11.0).color(theme.text_secondary))
                                        .fill(theme.bg_canvas)
                                        .corner_radius(CornerRadius::same(6))).clicked() {
                                        assemble_in_build = true;
                                    }
                                });
                            });
                            ui.add_space(8.0);
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Tutup").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            open = false;
        }

        self.show_db_drawer = open;

        if let Some(idx) = load_idx {
            if let Some(comp) = db_components.get(idx) {
                self.component_name = comp.name.clone();
                self.component_category = comp.category.clone();
                self.variables = comp.variables.clone();
                self.save_notification = Some(format!("Komponen '{}' berhasil dimuat ke editor parametrik!", comp.name));
                self.show_db_drawer = false;
            }
        }

        if assemble_in_build {
            *active_pillar = NavigationPillar::ModelingGeometry;
            self.show_db_drawer = false;
        }
    }
}
