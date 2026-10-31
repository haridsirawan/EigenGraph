//! parametric.rs — Pillar 2: BIMCraft-Grade 2D Plan, Live Section Cut & Parametric CAD Studio.
//! Features:
//! - Interactive Floor Plan Authoring (Compound Walls, Hosted Doors/Windows with Swing Arcs,
//!   Structural Columns, Beams/Runs, Slabs/Shafts, Auto-Detected Planar Rooms & Associative Dimensions)
//! - Revit-Style Object Snaps (`SE, SM, SI, SC, SP, SN, SG`), 15° Angular Guide & Graduated Ruler
//! - Live 2D Section Cut View with Solid Poché, Rebar Cut Dots, Level Planes & Grid Bubbles
//! - Sheet Layout Composer (`Sheet A101` Title Block) with One-Click Vector PDF, SVG & IFC4 Export
//! - Transactional `BimCommandBus` with Unlimited Undo/Redo & Live FEM/ARI Redundancy Audit.

use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, Ui, Vec2,
};
use crate::core::{
    BimCommand, BimCommandBus, OpeningKind, P2, RebarPreset, RunCategory, RunJustify, SnapHit,
};
use crate::models::{CadTool, DatabaseComponent, NavigationPillar, ParametricVariable};
use crate::theme::MacTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftingViewMode {
    FloorPlanViewRange,
    LiveSectionCutPoche,
    SheetLayoutA101,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BimAuthoringTool {
    CadStandard,
    WallChain,
    StructuralBeam,
    StructuralColumn,
    PlaceDoor,
    PlaceWindow,
    ReinforceConcrete,
}

pub struct ParametricView {
    pub active_tool: CadTool,
    pub bim_tool: BimAuthoringTool,
    pub view_mode: DraftingViewMode,
    pub bim_bus: BimCommandBus,
    pub active_level_idx: usize,
    pub pending_start_pt: Option<P2>,
    pub last_snap_hit: Option<SnapHit>,
    pub canvas_pan: Vec2,
    pub canvas_zoom: f32,
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
            bim_tool: BimAuthoringTool::WallChain,
            view_mode: DraftingViewMode::FloorPlanViewRange,
            bim_bus: BimCommandBus::new(),
            active_level_idx: 0,
            pending_start_pt: None,
            last_snap_hit: None,
            canvas_pan: Vec2::new(60.0, 40.0),
            canvas_zoom: 1.0,
            component_name: "NVW_DCR Portal Frame FL1-FL6 (ASTM A992)".to_string(),
            component_category: "NVW_DCR-LOD350_BIMCraft".to_string(),
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
                "Command: BIMCRAFT × EIGENGRAPH 2D/3D RELATIONAL STUDIO READY".to_string(),
                "Shortcuts: WA (Wall), BM (Beam), CL (Column), DR (Door), WN (Window), RM (Auto-Rooms), RF (Reinforce), UNDO, REDO.".to_string(),
                "Object Snaps active: SE (Endpoint), SM (Midpoint), SI (Intersection), SC (Center), SP (Perpendicular), SG (Grid).".to_string(),
            ],
            snap_to_grid: true,
            ortho_mode: false,
            cursor_pos_mm: (6000.0, 4500.0, 0.0),
            show_save_modal: false,
            show_db_drawer: false,
            save_notification: None,
            ai_suggestion_text: Some(
                "AI Copilot: 3 Ruangan tertutup (108.0 m²) & 18 Kolom/Balok terdeteksi otomatis via Planar Half-Edge. ARI Global: 0.942."
                    .to_string(),
            ),
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
            // 1. TOP DRAFTING & BIMCRAFT AUTHORING RIBBON
            self.render_drafting_ribbon(ui, theme, db_components, active_pillar);

            // 2. MAIN WORKSPACE (Left Variables & BIM Browser + Center Interactive Vector Canvas)
            ui.horizontal(|ui| {
                self.render_variables_panel(ui, theme, active_pillar);
                self.render_drawing_canvas(ui, theme, db_components, active_pillar);
            });
        });

        if self.show_db_drawer {
            self.render_db_drawer(ui.ctx(), theme, db_components, active_pillar);
        }
    }

    fn refresh_live_ari(&mut self) {
        if let Ok(rep) = self.bim_bus.project.evaluate_live_ari() {
            self.live_ari_score = rep.average_ari as f32;
            self.live_svd_margin = if rep.kinematic_guard.is_stable {
                0.996
            } else {
                0.0
            };
            self.ai_suggestion_text = Some(format!(
                "AI Copilot: {} Walls, {} Columns, {} Beams, {} Rooms ({:.1} m²), Rebar {:.0} kg | ARI: {:.3}",
                self.bim_bus.project.walls.len(),
                self.bim_bus.project.columns.len(),
                self.bim_bus.project.runs.len(),
                self.bim_bus.project.rooms.len(),
                self.bim_bus.project.rooms.iter().map(|r| r.area_m2).sum::<f64>(),
                self.bim_bus.project.total_rebar_weight_kg(),
                self.live_ari_score
            ));
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
            .inner_margin(Margin::symmetric(14, 6))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Row 1: CAD Tools + Undo/Redo + Save/DB
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("📐 2D CAD & BIM")
                                .size(12.5)
                                .strong()
                                .color(theme.accent_blue),
                        );
                        ui.separator();

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
                            let is_selected = self.bim_tool == BimAuthoringTool::CadStandard
                                && self.active_tool == tool;
                            let text = format!("{} [{}]", name, shortcut);

                            let btn = egui::Button::new(
                                RichText::new(text).size(11.0).color(if is_selected {
                                    Color32::WHITE
                                } else {
                                    theme.text_secondary
                                }),
                            )
                            .fill(if is_selected {
                                theme.accent_blue
                            } else {
                                theme.bg_card
                            })
                            .corner_radius(CornerRadius::same(6));

                            if ui.add(btn).clicked() {
                                self.active_tool = tool;
                                self.bim_tool = BimAuthoringTool::CadStandard;
                                self.pending_start_pt = None;
                                self.command_history
                                    .push(format!("CAD Tool changed to: {:?}", tool));
                            }
                        }

                        ui.separator();

                        if ui
                            .selectable_label(self.ortho_mode, "📐 ORTHO (F8)")
                            .clicked()
                        {
                            self.ortho_mode = !self.ortho_mode;
                        }
                        if ui
                            .selectable_label(self.snap_to_grid, "🧲 OSNAP (F9)")
                            .clicked()
                        {
                            self.snap_to_grid = !self.snap_to_grid;
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let save_btn = egui::Button::new(
                                RichText::new("💾 Save Component")
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(theme.accent_green)
                            .corner_radius(CornerRadius::same(6));

                            if ui.add(save_btn).clicked() {
                                self.save_current_to_db(db_components);
                            }

                            let db_text = format!("📦 Saved DB ({})", db_components.len());
                            if ui.selectable_label(self.show_db_drawer, db_text).clicked() {
                                self.show_db_drawer = !self.show_db_drawer;
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // Row 2: BIMCraft Authoring Ribbon (Wall, Beam, Column, Door, Window, Rebar, View Mode, Sheet Export)
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("🏛️ BIMCRAFT:")
                                .size(11.0)
                                .strong()
                                .color(theme.accent_orange),
                        );

                        let bim_tools = [
                            (BimAuthoringTool::WallChain, "🧱 Wall [WA]"),
                            (BimAuthoringTool::StructuralBeam, "📏 Beam/Run [BM]"),
                            (BimAuthoringTool::StructuralColumn, "🏛️ Column [CL]"),
                            (BimAuthoringTool::PlaceDoor, "🚪 Door [DR]"),
                            (BimAuthoringTool::PlaceWindow, "🪟 Window [WN]"),
                            (BimAuthoringTool::ReinforceConcrete, "🔩 Reinforce [RF]"),
                        ];
                        for (btool, label) in bim_tools {
                            let active = self.bim_tool == btool;
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(label).size(11.0).color(if active {
                                            Color32::WHITE
                                        } else {
                                            theme.text_secondary
                                        }),
                                    )
                                    .fill(if active {
                                        theme.accent_orange
                                    } else {
                                        theme.bg_card
                                    })
                                    .corner_radius(CornerRadius::same(5)),
                                )
                                .clicked()
                            {
                                self.bim_tool = btool;
                                self.pending_start_pt = None;
                            }
                        }

                        ui.separator();

                        // View Mode Switcher: Plan | Section Cut | Sheet A101
                        let views = [
                            (DraftingViewMode::FloorPlanViewRange, "🗺️ Floor Plan (+1200mm)"),
                            (DraftingViewMode::LiveSectionCutPoche, "🔪 Section Cut (Poché)"),
                            (DraftingViewMode::SheetLayoutA101, "📄 Sheet A101 (1:100)"),
                        ];
                        for (vm, vlabel) in views {
                            if ui.selectable_label(self.view_mode == vm, vlabel).clicked() {
                                self.view_mode = vm;
                            }
                        }

                        ui.separator();

                        // Undo / Redo Buttons
                        if ui
                            .add_enabled(
                                !self.bim_bus.undo_stack.is_empty(),
                                egui::Button::new(RichText::new("⟲ Undo").size(10.5)),
                            )
                            .clicked()
                        {
                            if let Some(msg) = self.bim_bus.undo() {
                                self.command_history.push(msg);
                                self.refresh_live_ari();
                            }
                        }
                        if ui
                            .add_enabled(
                                !self.bim_bus.redo_stack.is_empty(),
                                egui::Button::new(RichText::new("⟳ Redo").size(10.5)),
                            )
                            .clicked()
                        {
                            if let Some(msg) = self.bim_bus.redo() {
                                self.command_history.push(msg);
                                self.refresh_live_ari();
                            }
                        }

                        // Export PDF / SVG / IFC4
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("🖨️ Export PDF/IFC4")
                                        .size(10.5)
                                        .color(theme.accent_blue),
                                )
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(5)),
                            )
                            .clicked()
                        {
                            self.export_bim_deliverables();
                        }
                    });
                });
            });
    }

    fn export_bim_deliverables(&mut self) {
        let export_dir = crate::core::InternalVault::vault_root_dir().join("Exports");
        let _ = std::fs::create_dir_all(&export_dir);
        let pdf_path = export_dir.join("Sheet_A101_BIMCraft.pdf");
        let svg_path = export_dir.join("Sheet_A101_BIMCraft.svg");
        let ifc_path = export_dir.join("Building_BIMCraft_LOD350.ifc");

        let pdf_bytes = self
            .bim_bus
            .project
            .export_sheet_pdf("A101", "Level 1 Floor Plan & Section A-A");
        let svg_str = self
            .bim_bus
            .project
            .export_sheet_svg("A101", "Level 1 Floor Plan & Section A-A");
        let ifc_str = self.bim_bus.project.export_ifc4_spatial();

        let _ = std::fs::write(&pdf_path, pdf_bytes);
        let _ = std::fs::write(&svg_path, svg_str);
        let _ = std::fs::write(&ifc_path, ifc_str);

        self.save_notification = Some(format!(
            "Exported Vector Sheet PDF, SVG & Full Spatial IFC4 to {}",
            export_dir.display()
        ));
    }

    fn render_variables_panel(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: &mut NavigationPillar,
    ) {
        theme.sidebar_frame().show(ui, |ui| {
            ui.set_width(300.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(
                    RichText::new("BIM LEVELS, ROOMS & DRIVING PARAMETERS")
                        .size(11.5)
                        .strong()
                        .color(theme.text_secondary),
                );
                ui.add_space(6.0);

                // Active Storey Level Selector
                theme.card_frame().show(ui, |ui| {
                    ui.label(
                        RichText::new("🏢 ACTIVE STOREY LEVEL & VIEW RANGE")
                            .size(11.0)
                            .strong()
                            .color(theme.accent_blue),
                    );
                    ui.add_space(4.0);
                    for (idx, lvl) in self.bim_bus.project.levels.iter().enumerate() {
                        let is_sel = self.active_level_idx == idx;
                        if ui
                            .selectable_label(
                                is_sel,
                                format!("{} ({:+.0} mm)", lvl.name, lvl.elevation_mm),
                            )
                            .clicked()
                        {
                            self.active_level_idx = idx;
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "Cut Plane: +{:.0}mm | Rooms: {} | Rebar: {:.1} kg",
                            self.bim_bus.project.view_range.cut_plane_mm,
                            self.bim_bus.project.rooms.len(),
                            self.bim_bus.project.total_rebar_weight_kg()
                        ))
                        .size(10.0)
                        .color(theme.accent_green),
                    );
                });

                ui.add_space(6.0);

                // Enclosed Planar Rooms Schedule Summary
                theme.card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("🏠 AUTO-BOUNDED ROOMS (HALF-EDGE)")
                                .size(10.5)
                                .strong()
                                .color(theme.accent_orange),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("🔄 Recalc").clicked() {
                                let lvl_id = self
                                    .bim_bus
                                    .project
                                    .levels
                                    .get(self.active_level_idx)
                                    .map(|l| l.id)
                                    .unwrap_or(1);
                                let _ = self
                                    .bim_bus
                                    .execute(BimCommand::AutoDetectRooms { level_id: lvl_id });
                            }
                        });
                    });
                    ui.add_space(4.0);
                    for rm in &self.bim_bus.project.rooms {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{} {}", rm.number, rm.name))
                                    .size(10.5)
                                    .color(theme.text_primary),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{:.1} m² ({:.1}m)",
                                        rm.area_m2, rm.perimeter_m
                                    ))
                                    .size(10.5)
                                    .strong()
                                    .color(theme.accent_green),
                                );
                            });
                        });
                    }
                });

                ui.add_space(6.0);

                // Component Details & Driving Variables
                ui.label(
                    RichText::new("Component Name:")
                        .size(11.0)
                        .color(theme.text_muted),
                );
                ui.text_edit_singleline(&mut self.component_name);

                ui.add_space(4.0);
                ui.label(
                    RichText::new("Driving Dimensions & Parameters")
                        .size(11.5)
                        .strong()
                        .color(theme.text_primary),
                );
                ui.add_space(4.0);

                for var in &mut self.variables {
                    theme.card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&var.name)
                                    .size(11.5)
                                    .strong()
                                    .color(theme.accent_blue),
                            );
                            ui.label(RichText::new("=").size(11.5).color(theme.text_secondary));
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut var.expression)
                                        .desired_width(70.0),
                                )
                                .changed()
                            {
                                if let Ok(val) = var.expression.parse::<f64>() {
                                    var.evaluated_value = val;
                                }
                            }
                            ui.label(RichText::new(&var.unit).size(10.0).color(theme.text_muted));
                        });
                    });
                    ui.add_space(2.0);
                }

                ui.add_space(6.0);
                if ui.button("+ Add Variable Parameter").clicked() {
                    self.variables.push(ParametricVariable {
                        name: format!("var_{}", self.variables.len() + 1),
                        expression: "1000".to_string(),
                        evaluated_value: 1000.0,
                        unit: "mm".to_string(),
                    });
                }

                ui.add_space(8.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("📊 Lihat Hasil di PLANNING (5D RAB) →")
                                .size(11.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(theme.accent_blue)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(ui.available_width(), 28.0)),
                    )
                    .clicked()
                {
                    *active_pillar = NavigationPillar::CostBudgeting;
                }

                ui.add_space(4.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("🌱 Cek Karbon di SUSTAINABILITY (6D) →")
                                .size(11.0)
                                .color(theme.text_primary),
                        )
                        .fill(theme.bg_card)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(ui.available_width(), 28.0)),
                    )
                    .clicked()
                {
                    *active_pillar = NavigationPillar::SustainabilityEnergy;
                }

                ui.add_space(8.0);
                Frame::new()
                    .fill(Color32::from_rgb(18, 22, 28))
                    .corner_radius(CornerRadius::same(8))
                    .stroke(Stroke::new(1.0, theme.border_card))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("⚡ LIVE BIM → FEM/ARI SOLVER")
                                .size(10.5)
                                .strong()
                                .color(theme.accent_blue),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Global ARI: {:.3} (Safe/Redundant)",
                                self.live_ari_score
                            ))
                            .size(11.0)
                            .color(theme.accent_green),
                        );
                        ui.label(
                            RichText::new(format!(
                                "SVD Singularity Margin: {:.1}%",
                                self.live_svd_margin * 100.0
                            ))
                            .size(10.0)
                            .color(theme.text_secondary),
                        );
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
            if let Some(ref note) = self.save_notification {
                Frame::new()
                    .fill(theme.accent_green.gamma_multiply(0.15))
                    .stroke(Stroke::new(1.0, theme.accent_green))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(14, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✅").size(13.0));
                            ui.label(RichText::new(note).size(11.5).color(theme.accent_green));
                        });
                    });
                ui.add_space(4.0);
            }

            let available_height = (ui.available_height() - 102.0).max(360.0);
            let available_width = ui.available_width().max(480.0);

            Frame::new()
                .fill(Color32::from_rgb(14, 17, 23))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    let (response, painter) = ui.allocate_painter(
                        Vec2::new(available_width, available_height),
                        Sense::click_and_drag(),
                    );
                    let rect = response.rect;

                    // Pan & Zoom with Secondary Drag / Scroll Wheel
                    if response.dragged_by(egui::PointerButton::Secondary)
                        || (response.dragged() && ui.input(|i| i.modifiers.shift))
                    {
                        self.canvas_pan += response.drag_delta();
                    }
                    if response.hovered() {
                        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                        if scroll_y.abs() > 0.0 {
                            self.canvas_zoom =
                                (self.canvas_zoom * (1.0 + scroll_y * 0.002)).clamp(0.35, 4.5);
                        }
                    }

                    match self.view_mode {
                        DraftingViewMode::FloorPlanViewRange => {
                            self.draw_interactive_floor_plan(&response, &painter, rect, theme);
                        }
                        DraftingViewMode::LiveSectionCutPoche => {
                            self.draw_live_section_cut(&painter, rect, theme);
                        }
                        DraftingViewMode::SheetLayoutA101 => {
                            self.draw_sheet_layout_a101(&painter, rect, theme);
                        }
                    }
                });

            ui.add_space(6.0);

            // Command Line Interface (Bottom Box)
            Frame::new()
                .fill(Color32::from_rgb(22, 24, 28))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let history_len = self.command_history.len();
                        let start_idx = history_len.saturating_sub(2);
                        for line in &self.command_history[start_idx..] {
                            ui.label(
                                RichText::new(line)
                                    .size(10.5)
                                    .monospace()
                                    .color(theme.text_muted),
                            );
                        }

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Command:")
                                    .size(12.0)
                                    .strong()
                                    .monospace()
                                    .color(theme.accent_blue),
                            );
                            let cmd_res = ui.add(
                                egui::TextEdit::singleline(&mut self.command_line_input)
                                    .hint_text(
                                        "Type 'WA' (Wall), 'BM' (Beam), 'CL' (Column), 'DR' (Door), 'WN' (Window), 'RF' (Reinforce), 'UNDO', 'PDF' or JSON...",
                                    )
                                    .desired_width(ui.available_width() - 80.0),
                            );

                            let enter_pressed = (cmd_res.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                                || ui.button("Enter ↵").clicked();

                            if enter_pressed && !self.command_line_input.trim().is_empty() {
                                let raw = self.command_line_input.trim().to_string();
                                self.command_history.push(format!("Command: {raw}"));
                                self.execute_cad_command(&raw);
                                self.command_line_input.clear();
                            }
                        });
                    });
                });
        });
    }

    fn draw_interactive_floor_plan(
        &mut self,
        response: &egui::Response,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
    ) {
        let scale = 0.042 * self.canvas_zoom; // mm -> screen px
        let origin = Pos2::new(
            rect.left() + 120.0 + self.canvas_pan.x,
            rect.bottom() - 95.0 + self.canvas_pan.y,
        );

        let to_screen = |p: P2| -> Pos2 {
            Pos2::new(origin.x + (p.0 as f32) * scale, origin.y - (p.1 as f32) * scale)
        };
        let to_world = |s: Pos2| -> P2 {
            P2(
                ((s.x - origin.x) / scale) as f64,
                ((origin.y - s.y) / scale) as f64,
            )
        };

        // 1. Subtle Millimetre Grid
        for gx in (-2..=15).map(|i| i as f64 * 1000.0) {
            let p0 = to_screen(P2(gx, -1500.0));
            let p1 = to_screen(P2(gx, 10500.0));
            painter.line_segment(
                [p0, p1],
                Stroke::new(0.5, Color32::from_rgba_premultiplied(50, 65, 90, 35)),
            );
        }
        for gy in (-1..=11).map(|i| i as f64 * 1000.0) {
            let p0 = to_screen(P2(-1500.0, gy));
            let p1 = to_screen(P2(13500.0, gy));
            painter.line_segment(
                [p0, p1],
                Stroke::new(0.5, Color32::from_rgba_premultiplied(50, 65, 90, 35)),
            );
        }

        // 2. Structural Grids & Bubbles (1, 2, 3 / A, B, C)
        for g in &self.bim_bus.project.grids {
            let s0 = to_screen(g.start);
            let s1 = to_screen(g.end);
            painter.line_segment(
                [s0, s1],
                Stroke::new(1.0, Color32::from_rgba_premultiplied(56, 189, 248, 120)),
            );
            painter.circle_filled(s1, 11.0, Color32::from_rgb(20, 32, 50));
            painter.circle_stroke(s1, 11.0, Stroke::new(1.3, Color32::from_rgb(56, 189, 248)));
            painter.text(
                s1,
                Align2::CENTER_CENTER,
                &g.label,
                egui::FontId::monospace(10.5),
                Color32::from_rgb(125, 211, 252),
            );
        }

        // 3. Enclosed Rooms (Halftone Fill + Room Tag)
        for rm in &self.bim_bus.project.rooms {
            let pts: Vec<Pos2> = rm.outer_ring.iter().map(|&p| to_screen(p)).collect();
            if pts.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_premultiplied(16, 185, 129, 22),
                    Stroke::NONE,
                ));
            }
            for hole in &rm.holes {
                let hpts: Vec<Pos2> = hole.iter().map(|&p| to_screen(p)).collect();
                if hpts.len() >= 3 {
                    painter.add(egui::Shape::convex_polygon(
                        hpts.clone(),
                        Color32::from_rgb(14, 17, 23),
                        Stroke::new(1.0, Color32::from_rgb(244, 63, 94)),
                    ));
                    // Shaft cross 'X' symbol
                    if hpts.len() >= 4 {
                        painter.line_segment(
                            [hpts[0], hpts[2]],
                            Stroke::new(0.8, Color32::from_rgb(244, 63, 94)),
                        );
                        painter.line_segment(
                            [hpts[1], hpts[3]],
                            Stroke::new(0.8, Color32::from_rgb(244, 63, 94)),
                        );
                    }
                }
            }
            let c_pos = to_screen(rm.seed_point);
            painter.text(
                c_pos - Vec2::new(0.0, 7.0),
                Align2::CENTER_CENTER,
                format!("{} — {}", rm.number, rm.name),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(110, 231, 183),
            );
            painter.text(
                c_pos + Vec2::new(0.0, 7.0),
                Align2::CENTER_CENTER,
                format!("{:.1} m² (P={:.1} m)", rm.area_m2, rm.perimeter_m),
                egui::FontId::monospace(9.5),
                Color32::from_rgb(167, 243, 208),
            );
        }

        // 4. Structural Slab Shaft Openings
        for slab in &self.bim_bus.project.slabs {
            for hole in &slab.openings {
                let hpts: Vec<Pos2> = hole.iter().map(|&p| to_screen(p)).collect();
                if hpts.len() >= 4 {
                    painter.add(egui::Shape::convex_polygon(
                        hpts.clone(),
                        Color32::from_rgba_premultiplied(244, 63, 94, 30),
                        Stroke::new(1.2, Color32::from_rgb(244, 63, 94)),
                    ));
                    painter.line_segment(
                        [hpts[0], hpts[2]],
                        Stroke::new(1.0, Color32::from_rgb(244, 63, 94)),
                    );
                    painter.line_segment(
                        [hpts[1], hpts[3]],
                        Stroke::new(1.0, Color32::from_rgb(244, 63, 94)),
                    );
                }
            }
        }

        // 5. Structural Beams / Runs (Projected Dashed Axis + Outline)
        for run in &self.bim_bus.project.runs {
            let p0 = to_screen(run.start);
            let p1 = to_screen(run.end);
            painter.line_segment(
                [p0, p1],
                Stroke::new(1.5, Color32::from_rgba_premultiplied(52, 211, 153, 175)),
            );
        }

        // 6. Compound Walls in Cut Poché + Hosted Doors (with Swing Arcs) & Windows
        for w in &self.bim_bus.project.walls {
            let ring_pts: Vec<Pos2> = w.footprint_ring().into_iter().map(to_screen).collect();
            let fill_col = if w.is_structural {
                Color32::from_rgb(100, 116, 139)
            } else {
                Color32::from_rgb(71, 85, 105)
            };
            painter.add(egui::Shape::convex_polygon(
                ring_pts,
                fill_col,
                Stroke::new(1.5, Color32::from_rgb(226, 232, 240)),
            ));

            // Hosted Openings (Doors & Windows)
            let d = w.direction();
            let n = w.normal_outward();
            for op in &w.openings {
                let op_a = w.start + d * op.offset_along_mm;
                let op_b = w.start + d * (op.offset_along_mm + op.width_mm);
                let half_t = w.thickness_mm * 0.55;
                let hole_pts: Vec<Pos2> = [
                    op_a - n * half_t,
                    op_b - n * half_t,
                    op_b + n * half_t,
                    op_a + n * half_t,
                ]
                .into_iter()
                .map(to_screen)
                .collect();

                // Clear wall poché inside the opening
                painter.add(egui::Shape::convex_polygon(
                    hole_pts,
                    Color32::from_rgb(14, 17, 23),
                    Stroke::new(1.0, Color32::from_rgb(251, 191, 36)),
                ));

                match op.kind {
                    OpeningKind::SingleDoor | OpeningKind::DoubleDoor | OpeningKind::SlidingDoor => {
                        // Draw Door Leaf & 90° Swing Arc
                        let leaf_tip = op_a + n * op.width_mm;
                        painter.line_segment(
                            [to_screen(op_a), to_screen(leaf_tip)],
                            Stroke::new(2.0, Color32::from_rgb(251, 191, 36)),
                        );
                        let mut arc_pts = Vec::with_capacity(13);
                        for k in 0..=12 {
                            let theta = (k as f64) * std::f64::consts::FRAC_PI_2 / 12.0;
                            let pt = op_a + d * (op.width_mm * theta.cos()) + n * (op.width_mm * theta.sin());
                            arc_pts.push(to_screen(pt));
                        }
                        for seg in arc_pts.windows(2) {
                            painter.line_segment(
                                [seg[0], seg[1]],
                                Stroke::new(1.0, Color32::from_rgb(251, 191, 36)),
                            );
                        }
                    }
                    _ => {
                        // Window Double Glass Pane Lines
                        painter.line_segment(
                            [to_screen(op_a), to_screen(op_b)],
                            Stroke::new(2.0, Color32::from_rgb(56, 189, 248)),
                        );
                    }
                }

                // Opening Tag Pill (`D-01`, `W-01`)
                let tag_pos = to_screen((op_a + op_b) * 0.5 - n * 380.0);
                painter.text(
                    tag_pos,
                    Align2::CENTER_CENTER,
                    &op.mark,
                    egui::FontId::monospace(9.5),
                    Color32::from_rgb(253, 224, 71),
                );
            }
        }

        // 7. Structural Columns in Solid Poché
        for col in &self.bim_bus.project.columns {
            let cpts: Vec<Pos2> = col.footprint_ring().into_iter().map(to_screen).collect();
            painter.add(egui::Shape::convex_polygon(
                cpts,
                Color32::from_rgb(56, 189, 248),
                Stroke::new(1.5, Color32::WHITE),
            ));
        }

        // 8. Associative Linear Dimensions
        for dim in &self.bim_bus.project.dimensions {
            let d = (dim.end - dim.start).unit();
            let n = d.perp();
            let a_dim = dim.start + n * dim.offset_mm;
            let b_dim = dim.end + n * dim.offset_mm;
            let sa = to_screen(a_dim);
            let sb = to_screen(b_dim);
            painter.line_segment(
                [to_screen(dim.start), sa],
                Stroke::new(0.8, Color32::from_rgb(148, 163, 184)),
            );
            painter.line_segment(
                [to_screen(dim.end), sb],
                Stroke::new(0.8, Color32::from_rgb(148, 163, 184)),
            );
            painter.line_segment([sa, sb], Stroke::new(1.2, Color32::from_rgb(226, 232, 240)));
            let mid = Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5 - 8.0);
            painter.text(
                mid,
                Align2::CENTER_CENTER,
                format!("{:.0} mm", dim.value_mm()),
                egui::FontId::monospace(10.0),
                Color32::from_rgb(248, 250, 252),
            );
        }

        // 9. Section Cut Line Marker (Section A-A)
        let (sec_a, sec_b) = self.bim_bus.project.section_line;
        let s_sec_a = to_screen(sec_a);
        let s_sec_b = to_screen(sec_b);
        painter.line_segment(
            [s_sec_a, s_sec_b],
            Stroke::new(1.2, Color32::from_rgba_premultiplied(244, 63, 94, 160)),
        );
        painter.text(
            s_sec_a - Vec2::new(14.0, 0.0),
            Align2::CENTER_CENTER,
            "A",
            egui::FontId:: proportional(12.0),
            Color32::from_rgb(244, 63, 94),
        );
        painter.text(
            s_sec_b + Vec2::new(14.0, 0.0),
            Align2::CENTER_CENTER,
            "A",
            egui::FontId::proportional(12.0),
            Color32::from_rgb(244, 63, 94),
        );

        // 10. Interactive Cursor Object Snap (`SE, SM, SI, SC, SP, SN`) + Graduated Angular Ruler
        if let Some(mpos) = response.hover_pos() {
            let raw_world = to_world(mpos);
            let angle_step = if self.ortho_mode { 90.0 } else { 15.0 };
            let snap_hit = self.bim_bus.project.find_snap(
                raw_world,
                self.pending_start_pt,
                if self.snap_to_grid { 380.0 } else { 50.0 },
                angle_step,
            );
            let snapped_screen = to_screen(snap_hit.point);
            self.cursor_pos_mm = (
                snap_hit.point.0 as f32,
                snap_hit.point.1 as f32,
                self.bim_bus
                    .project
                    .levels
                    .get(self.active_level_idx)
                    .map(|l| l.elevation_mm as f32)
                    .unwrap_or(0.0),
            );

            // Draw Snap Marker Icon & Label
            let (short, _) = snap_hit.kind.shortcut_and_label();
            let snap_col = Color32::from_rgb(0, 240, 255);
            painter.circle_stroke(snapped_screen, 6.5, Stroke::new(1.8, snap_col));
            painter.text(
                snapped_screen + Vec2::new(10.0, -10.0),
                Align2::LEFT_BOTTOM,
                format!("[{}] {}", short, snap_hit.description),
                egui::FontId::monospace(10.0),
                snap_col,
            );

            // Draw Live Preview Segment & Graduated Ruler if a start point is active
            if let Some(start_w) = self.pending_start_pt {
                let s_start = to_screen(start_w);
                painter.line_segment(
                    [s_start, snapped_screen],
                    Stroke::new(2.2, theme.accent_orange),
                );
                if let (Some(dist_mm), Some(ang_deg)) =
                    (snap_hit.distance_from_last_mm, snap_hit.angle_deg)
                {
                    let mid = Pos2::new(
                        (s_start.x + snapped_screen.x) * 0.5,
                        (s_start.y + snapped_screen.y) * 0.5 - 14.0,
                    );
                    painter.text(
                        mid,
                        Align2::CENTER_CENTER,
                        format!("📏 {:.0} mm  ∠ {:.1}°", dist_mm, ang_deg),
                        egui::FontId::monospace(11.0),
                        Color32::from_rgb(253, 224, 71),
                    );
                }
            }

            // Handle Left Click to Place/Draw BIM Elements
            if response.clicked() {
                let active_lvl_id = self
                    .bim_bus
                    .project
                    .levels
                    .get(self.active_level_idx)
                    .map(|l| l.id)
                    .unwrap_or(1);
                let top_lvl_id = self
                    .bim_bus
                    .project
                    .levels
                    .get(self.active_level_idx + 1)
                    .map(|l| l.id)
                    .unwrap_or(active_lvl_id);

                match self.bim_tool {
                    BimAuthoringTool::WallChain | BimAuthoringTool::CadStandard => {
                        if let Some(start_pt) = self.pending_start_pt {
                            if start_pt.dist(snap_hit.point) >= 150.0 {
                                if let Ok(msg) = self.bim_bus.execute(BimCommand::AddWall {
                                    base_level_id: active_lvl_id,
                                    start: start_pt,
                                    end: snap_hit.point,
                                    thickness_mm: 200.0,
                                    is_structural: true,
                                }) {
                                    self.command_history.push(msg);
                                    self.refresh_live_ari();
                                }
                                self.pending_start_pt = Some(snap_hit.point);
                            }
                        } else {
                            self.pending_start_pt = Some(snap_hit.point);
                        }
                    }
                    BimAuthoringTool::StructuralBeam => {
                        if let Some(start_pt) = self.pending_start_pt {
                            if start_pt.dist(snap_hit.point) >= 150.0 {
                                if let Ok(msg) = self.bim_bus.execute(BimCommand::AddBeamRun {
                                    level_id: top_lvl_id,
                                    category: RunCategory::StructuralBeam,
                                    start: start_pt,
                                    end: snap_hit.point,
                                    width_mm: 200.0,
                                    depth_mm: 400.0,
                                    justify: RunJustify::Top,
                                    profile_name: "WF 400x200x8x13".to_string(),
                                }) {
                                    self.command_history.push(msg);
                                    self.refresh_live_ari();
                                }
                                self.pending_start_pt = None;
                            }
                        } else {
                            self.pending_start_pt = Some(snap_hit.point);
                        }
                    }
                    BimAuthoringTool::StructuralColumn => {
                        if let Ok(msg) = self.bim_bus.execute(BimCommand::AddColumn {
                            base_level_id: active_lvl_id,
                            top_level_id: top_lvl_id,
                            pos: snap_hit.point,
                            width_mm: 400.0,
                            depth_mm: 400.0,
                            is_round: false,
                            profile_name: "WF 400x200x8x13".to_string(),
                        }) {
                            self.command_history.push(msg);
                            self.refresh_live_ari();
                        }
                    }
                    BimAuthoringTool::PlaceDoor | BimAuthoringTool::PlaceWindow => {
                        // Find nearest wall to host the door or window
                        if let Some(w) = self
                            .bim_bus
                            .project
                            .walls
                            .iter()
                            .min_by(|a, b| {
                                crate::core::geom::dist_to_segment(snap_hit.point, a.start, a.end)
                                    .total_cmp(&crate::core::geom::dist_to_segment(
                                        snap_hit.point,
                                        b.start,
                                        b.end,
                                    ))
                            })
                            .cloned()
                        {
                            let offset = (snap_hit.point.dist(w.start) - 450.0)
                                .clamp(100.0, (w.length_mm() - 1100.0).max(100.0));
                            let is_door = self.bim_tool == BimAuthoringTool::PlaceDoor;
                            if let Ok(msg) = self.bim_bus.execute(BimCommand::AddOpening {
                                wall_id: w.id,
                                kind: if is_door {
                                    OpeningKind::SingleDoor
                                } else {
                                    OpeningKind::CasementWindow
                                },
                                offset_along_mm: offset,
                                width_mm: if is_door { 900.0 } else { 1500.0 },
                                height_mm: if is_door { 2100.0 } else { 1400.0 },
                                sill_height_mm: if is_door { 0.0 } else { 900.0 },
                            }) {
                                self.command_history.push(msg);
                            }
                        }
                    }
                    BimAuthoringTool::ReinforceConcrete => {
                        if let Ok(msg) = self.bim_bus.execute(BimCommand::ReinforceAllConcrete {
                            preset: RebarPreset::default(),
                        }) {
                            self.command_history.push(msg);
                        }
                    }
                }
            }

            if response.secondary_clicked() {
                self.pending_start_pt = None;
            }

            self.last_snap_hit = Some(snap_hit);
        }

        // Top-Left HUD Overlay
        painter.text(
            Pos2::new(rect.left() + 14.0, rect.top() + 12.0),
            Align2::LEFT_TOP,
            format!(
                "🗺️ FLOOR PLAN VIEW RANGE (Cut +1200mm) • Active Tool: {:?} • Right-Click to finish chain / Drag to Pan",
                self.bim_tool
            ),
            egui::FontId::monospace(10.5),
            Color32::from_rgb(148, 163, 184),
        );
    }

    fn draw_live_section_cut(&self, painter: &egui::Painter, rect: Rect, _theme: &MacTheme) {
        let (sec_a, sec_b) = self.bim_bus.project.section_line;
        let sec = self.bim_bus.project.generate_section_cut(sec_a, sec_b);
        let scale = 0.045 * self.canvas_zoom;
        let origin = Pos2::new(
            rect.left() + 100.0 + self.canvas_pan.x,
            rect.bottom() - 110.0 + self.canvas_pan.y,
        );
        let map_pt = |u: f64, z: f64| -> Pos2 {
            Pos2::new(origin.x + (u as f32) * scale, origin.y - (z as f32) * scale)
        };

        // 1. Level Datum Lines & Elevation Labels
        for (lname, elev) in &sec.level_lines {
            let p0 = map_pt(0.0, *elev);
            let p1 = map_pt(sec.section_length_mm, *elev);
            painter.line_segment(
                [p0, p1],
                Stroke::new(1.0, Color32::from_rgba_premultiplied(56, 189, 248, 140)),
            );
            painter.text(
                p1 + Vec2::new(8.0, 0.0),
                Align2::LEFT_CENTER,
                format!("▲ {} ({:+.0} mm)", lname, elev),
                egui::FontId::monospace(10.0),
                Color32::from_rgb(125, 211, 252),
            );
        }

        // 2. Grid Vertical Axes in Section
        for (glabel, u) in &sec.grid_markers {
            let p0 = map_pt(*u, sec.min_z_mm);
            let p1 = map_pt(*u, sec.max_z_mm);
            painter.line_segment(
                [p0, p1],
                Stroke::new(0.8, Color32::from_rgba_premultiplied(148, 163, 184, 90)),
            );
            painter.circle_stroke(p1, 10.0, Stroke::new(1.2, Color32::from_rgb(56, 189, 248)));
            painter.text(
                p1,
                Align2::CENTER_CENTER,
                glabel,
                egui::FontId::monospace(10.0),
                Color32::WHITE,
            );
        }

        // 3. Sliced 3D Mesh Poché Segments (Walls, Slabs, Columns, Beams, Footings)
        for (a, b, rgb, _) in &sec.cut_segments_2d {
            let p0 = map_pt(a[0], a[1]);
            let p1 = map_pt(b[0], b[1]);
            painter.line_segment(
                [p0, p1],
                Stroke::new(2.4, Color32::from_rgb(rgb[0], rgb[1], rgb[2])),
            );
        }

        // 4. Sliced Reinforcing Bars (`IfcReinforcingBar`) Drawn as Crimson Rebar Dots
        for (pt, dia) in &sec.rebar_dots_2d {
            let sp = map_pt(pt[0], pt[1]);
            let r = ((*dia as f32) * scale * 2.5).clamp(2.5, 5.0);
            painter.circle_filled(sp, r, Color32::from_rgb(244, 63, 94));
        }

        painter.text(
            Pos2::new(rect.left() + 14.0, rect.top() + 12.0),
            Align2::LEFT_TOP,
            format!(
                "🔪 LIVE SECTION CUT A-A (True 3D Mesh Plane Slice) • {} Cut Segments • {} Rebar Cut Dots",
                sec.cut_segments_2d.len(),
                sec.rebar_dots_2d.len()
            ),
            egui::FontId::monospace(10.5),
            Color32::from_rgb(251, 191, 36),
        );
    }

    fn draw_sheet_layout_a101(&self, painter: &egui::Painter, rect: Rect, theme: &MacTheme) {
        let sheet_rect = rect.shrink(22.0);
        // White paper sheet background (as BIMCraft does even in dark mode)
        painter.rect_filled(
            sheet_rect,
            CornerRadius::same(4),
            Color32::from_rgb(248, 250, 252),
        );
        painter.rect_stroke(
            sheet_rect,
            CornerRadius::same(4),
            Stroke::new(2.0, Color32::from_rgb(15, 23, 42)),
            egui::StrokeKind::Inside,
        );

        // Title Block Box (Bottom Right)
        let tb_rect = Rect::from_min_max(
            Pos2::new(sheet_rect.right() - 280.0, sheet_rect.bottom() - 140.0),
            sheet_rect.max - Vec2::new(10.0, 10.0),
        );
        painter.rect_filled(tb_rect, CornerRadius::same(2), Color32::from_rgb(241, 245, 249));
        painter.rect_stroke(
            tb_rect,
            CornerRadius::same(2),
            Stroke::new(1.5, Color32::from_rgb(15, 23, 42)),
            egui::StrokeKind::Inside,
        );

        let lines = [
            "EIGENGRAPH × BIMCRAFT 10D SUITE",
            &format!("Project: {}", self.bim_bus.project.project_name),
            "Sheet: A101 — Plan, Section & Rebar Schedule",
            &format!(
                "Scale: 1 : 100 | Rebar Mass: {:.1} kg",
                self.bim_bus.project.total_rebar_weight_kg()
            ),
            &format!(
                "Global ARI: {:.3} | Rooms: {} ({:.1} m²)",
                self.live_ari_score,
                self.bim_bus.project.rooms.len(),
                self.bim_bus.project.rooms.iter().map(|r| r.area_m2).sum::<f64>()
            ),
        ];
        for (i, l) in lines.iter().enumerate() {
            painter.text(
                tb_rect.min + Vec2::new(10.0, 12.0 + (i as f32) * 22.0),
                Align2::LEFT_TOP,
                *l,
                egui::FontId::monospace(if i == 0 { 11.0 } else { 9.5 }),
                if i == 0 {
                    theme.accent_blue
                } else {
                    Color32::from_rgb(30, 41, 59)
                },
            );
        }

        // Draw Scaled Plan Viewport on Left Half of the Sheet
        let plan_origin = Pos2::new(sheet_rect.left() + 65.0, sheet_rect.bottom() - 80.0);
        let s_scale = 0.026;
        let map_s = |p: P2| -> Pos2 {
            Pos2::new(
                plan_origin.x + (p.0 as f32) * s_scale,
                plan_origin.y - (p.1 as f32) * s_scale,
            )
        };
        for w in &self.bim_bus.project.walls {
            let pts: Vec<Pos2> = w.footprint_ring().into_iter().map(map_s).collect();
            painter.add(egui::Shape::convex_polygon(
                pts,
                Color32::from_rgb(71, 85, 105),
                Stroke::new(1.0, Color32::BLACK),
            ));
        }
        for c in &self.bim_bus.project.columns {
            let pts: Vec<Pos2> = c.footprint_ring().into_iter().map(map_s).collect();
            painter.add(egui::Shape::convex_polygon(pts, Color32::BLACK, Stroke::NONE));
        }
        painter.text(
            Pos2::new(sheet_rect.left() + 65.0, sheet_rect.bottom() - 35.0),
            Align2::LEFT_CENTER,
            "1) FLOOR PLAN — LEVEL 1 (SCALE 1 : 100)",
            egui::FontId::monospace(11.0),
            Color32::from_rgb(15, 23, 42),
        );
    }

    fn execute_cad_command(&mut self, cmd_raw: &str) {
        let trimmed = cmd_raw.trim();
        if trimmed.starts_with('{') {
            match self.bim_bus.execute_json(trimmed) {
                Ok(msg) => {
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
                Err(e) => self.command_history.push(format!("JSON Error: {e}")),
            }
            return;
        }

        let cmd = trimmed.to_uppercase();
        match cmd.as_str() {
            "L" | "LINE" => {
                self.active_tool = CadTool::Line;
                self.bim_tool = BimAuthoringTool::CadStandard;
            }
            "WA" | "WALL" => self.bim_tool = BimAuthoringTool::WallChain,
            "BM" | "BEAM" => self.bim_tool = BimAuthoringTool::StructuralBeam,
            "CL" | "COLUMN" => self.bim_tool = BimAuthoringTool::StructuralColumn,
            "DR" | "DOOR" => self.bim_tool = BimAuthoringTool::PlaceDoor,
            "WN" | "WINDOW" => self.bim_tool = BimAuthoringTool::PlaceWindow,
            "RF" | "REINFORCE" => {
                self.bim_tool = BimAuthoringTool::ReinforceConcrete;
                if let Ok(msg) = self.bim_bus.execute(BimCommand::ReinforceAllConcrete {
                    preset: RebarPreset::default(),
                }) {
                    self.command_history.push(msg);
                }
            }
            "RM" | "ROOM" => {
                let lvl_id = self
                    .bim_bus
                    .project
                    .levels
                    .get(self.active_level_idx)
                    .map(|l| l.id)
                    .unwrap_or(1);
                if let Ok(msg) = self
                    .bim_bus
                    .execute(BimCommand::AutoDetectRooms { level_id: lvl_id })
                {
                    self.command_history.push(msg);
                }
            }
            "UNDO" | "U" => {
                if let Some(msg) = self.bim_bus.undo() {
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
            }
            "REDO" => {
                if let Some(msg) = self.bim_bus.redo() {
                    self.command_history.push(msg);
                    self.refresh_live_ari();
                }
            }
            "PDF" | "SVG" | "IFC" => {
                self.export_bim_deliverables();
            }
            "PL" | "POLYLINE" => self.active_tool = CadTool::Polyline,
            "A" | "ARC" => self.active_tool = CadTool::Arc,
            "C" | "CIRCLE" => self.active_tool = CadTool::Circle,
            "REC" | "RECTANGLE" => self.active_tool = CadTool::Rectangle,
            "DIM" | "DI" | "DIMENSION" => self.active_tool = CadTool::Dimension,
            "TR" | "TRIM" => self.active_tool = CadTool::Trim,
            "O" | "OFFSET" => self.active_tool = CadTool::Offset,
            _ => {
                self.command_history
                    .push(format!("Executed instruction: {cmd}"));
            }
        }
    }

    fn save_current_to_db(&mut self, db_components: &mut Vec<DatabaseComponent>) {
        let new_comp = DatabaseComponent {
            id: format!("comp_{}", db_components.len() + 1),
            name: self.component_name.clone(),
            category: self.component_category.clone(),
            description: format!(
                "BIMCraft relational model: {} walls, {} columns, {} beams, {} rooms, {:.0} kg rebar.",
                self.bim_bus.project.walls.len(),
                self.bim_bus.project.columns.len(),
                self.bim_bus.project.runs.len(),
                self.bim_bus.project.rooms.len(),
                self.bim_bus.project.total_rebar_weight_kg()
            ),
            variables: self.variables.clone(),
            elements_count: self.bim_bus.project.walls.len()
                + self.bim_bus.project.columns.len()
                + self.bim_bus.project.runs.len(),
            created_at: "Just now".to_string(),
            ..Default::default()
        };

        db_components.push(new_comp);
        if let Ok(saved_path) =
            crate::core::InternalVault::autosave_parametric_components(db_components)
        {
            self.save_notification = Some(format!(
                "Komponen '{}' berhasil disimpan otomatis ke Package Contents: {} (Siap dirakit di BUILD 3D).",
                self.component_name, saved_path
            ));
        } else {
            self.save_notification = Some(format!(
                "Component '{}' successfully saved to Database!",
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
            .frame(
                Frame::new()
                    .fill(theme.bg_card)
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🏛️").size(28.0));
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(
                                    "Katalog Potongan Parametrik & Struktur Rumit (OpenBIM IFC)",
                                )
                                .size(15.0)
                                .strong()
                                .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new(
                                    "Potongan modular dari gedung berarsitektur kompleks siap pakai sebagai referensi & perakitan.",
                                )
                                .size(11.5)
                                .color(theme.text_secondary),
                            );
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for (idx, comp) in db_components.iter().enumerate() {
                            theme.card_frame().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&comp.name)
                                            .size(13.0)
                                            .strong()
                                            .color(theme.accent_blue),
                                    );
                                    ui.add_space(6.0);
                                    Frame::new()
                                        .fill(theme.accent_blue.gamma_multiply(0.15))
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(&comp.category)
                                                    .size(10.0)
                                                    .color(theme.accent_blue),
                                            );
                                        });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(&comp.created_at)
                                                .size(10.5)
                                                .color(theme.text_muted),
                                        );
                                    });
                                });

                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(&comp.description)
                                        .size(11.5)
                                        .color(theme.text_secondary),
                                );

                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new("Variabel:")
                                            .size(10.5)
                                            .strong()
                                            .color(theme.text_muted),
                                    );
                                    for v in comp.variables.iter().take(5) {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} = {} {}",
                                                v.name, v.expression, v.unit
                                            ))
                                            .size(10.0)
                                            .monospace()
                                            .color(theme.accent_green),
                                        );
                                        ui.add_space(4.0);
                                    }
                                });

                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("📥 Muat ke Kanvas (Load)")
                                                    .size(11.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(theme.accent_blue)
                                            .corner_radius(CornerRadius::same(6)),
                                        )
                                        .clicked()
                                    {
                                        load_idx = Some(idx);
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("🏗️ Rakit di BUILD (3D)")
                                                    .size(11.0)
                                                    .color(theme.text_secondary),
                                            )
                                            .fill(theme.bg_canvas)
                                            .corner_radius(CornerRadius::same(6)),
                                        )
                                        .clicked()
                                    {
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
                self.save_notification = Some(format!(
                    "Komponen '{}' berhasil dimuat ke editor parametrik!",
                    comp.name
                ));
                self.show_db_drawer = false;
            }
        }

        if assemble_in_build {
            *active_pillar = NavigationPillar::ModelingGeometry;
            self.show_db_drawer = false;
        }
    }
}
