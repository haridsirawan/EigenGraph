//! views/parametric/ribbon.rs — Authoring Ribbon & Vector Delivery Exports.

use super::*;

fn cad_tool_icon(tool: CadTool) -> AppIcon {
    match tool {
        CadTool::Select => AppIcon::SelectArrow,
        CadTool::Line => AppIcon::LineSeg,
        CadTool::Polyline => AppIcon::NodesGraph,
        CadTool::Arc => AppIcon::ArcCurve,
        CadTool::Circle => AppIcon::CircleShape,
        CadTool::Rectangle => AppIcon::RectShape,
        CadTool::Dimension => AppIcon::DimensionRuler,
        CadTool::Trim => AppIcon::Scissors,
        CadTool::Offset => AppIcon::Layers,
    }
}

impl ParametricView {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_drafting_ribbon(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        db_components: &mut Vec<DatabaseComponent>,
        active_pillar: &mut NavigationPillar,
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
        members: &[crate::models::StructuralMember],
        workspace_settings: WorkspaceCanvasSettings,
        _sidebar_toggle_inset_px: f32,
    ) {
        let _avail_w = ui.available_width();
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                let inner_w = ui.available_width().max(120.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.vertical(|ui| {
                    // Row 1: Universal Canvas HUD Toolbar (Module Pill, Views, Snaps, Panel Toggles, 1-File Sync)
                    ui.horizontal_wrapped(|ui| {
                        icons::pill_badge(
                            ui,
                            AppIcon::RulerSquare,
                            "Parametric • 2D",
                            theme.accent_blue,
                        );
                        ui.separator();

                        // View Mode Switcher (upper interactive canvas views only)
                        let views = [
                            (DraftingViewMode::FloorPlanViewRange, AppIcon::MapPin, "Plan"),
                            (DraftingViewMode::LiveSectionCutPoche, AppIcon::SectionCut, "Section"),
                            (DraftingViewMode::SheetLayoutA101, AppIcon::Document, "Sheet 2D"),
                            (DraftingViewMode::VisualNodeGraphDynamo, AppIcon::NodesGraph, "Dynamo"),
                        ];
                        for (vm, icon, vlabel) in views {
                            let selected = self.view_mode == vm;
                            if icons::icon_button(
                                ui,
                                icon,
                                vlabel,
                                10.5,
                                if selected { Color32::WHITE } else { theme.text_secondary },
                                if selected { theme.accent_blue } else { theme.bg_card },
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::new(0.0, 26.0),
                            )
                            .clicked()
                            {
                                self.view_mode = vm;
                            }
                        }

                        ui.separator();

                        // Undo / Redo / Grid / Ortho / OSNAP / Fit
                        crate::ui_kit::wrap_before(ui, 76.0);
                        ui.add_enabled_ui(!self.bim_bus.undo_stack.is_empty(), |ui| {
                            if icons::icon_button(
                                ui,
                                AppIcon::Undo,
                                "Undo",
                                10.5,
                                theme.text_secondary,
                                theme.bg_card,
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::new(0.0, 26.0),
                            )
                            .clicked()
                            {
                                if let Some(msg) = self.bim_bus.undo() {
                                    self.command_history.push(msg);
                                    self.refresh_live_ari();
                                }
                            }
                        });
                        crate::ui_kit::wrap_before(ui, 76.0);
                        ui.add_enabled_ui(!self.bim_bus.redo_stack.is_empty(), |ui| {
                            if icons::icon_button(
                                ui,
                                AppIcon::Redo,
                                "Redo",
                                10.5,
                                theme.text_secondary,
                                theme.bg_card,
                                Stroke::NONE,
                                CornerRadius::same(6),
                                Vec2::new(0.0, 26.0),
                            )
                            .clicked()
                            {
                                if let Some(msg) = self.bim_bus.redo() {
                                    self.command_history.push(msg);
                                    self.refresh_live_ari();
                                }
                            }
                        });

                        if icons::icon_button(
                            ui,
                            AppIcon::Grid,
                            "Grid",
                            10.5,
                            if universal_canvas.show_grid { Color32::WHITE } else { theme.text_secondary },
                            if universal_canvas.show_grid { theme.accent_blue.gamma_multiply(0.75) } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .clicked()
                        {
                            universal_canvas.show_grid = !universal_canvas.show_grid;
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::RulerSquare,
                            "ORTHO",
                            10.5,
                            if self.ortho_mode { Color32::WHITE } else { theme.text_secondary },
                            if self.ortho_mode { theme.accent_blue } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Toggle Ortho F8 constraint")
                        .clicked()
                        {
                            self.ortho_mode = !self.ortho_mode;
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::MagnetSnap,
                            "OSNAP",
                            10.5,
                            if self.snap_to_grid { Color32::WHITE } else { theme.text_secondary },
                            if self.snap_to_grid { theme.accent_blue } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Toggle Revit Object Snaps (SE, SM, SI, SC, SP, SG)")
                        .clicked()
                        {
                            self.snap_to_grid = !self.snap_to_grid;
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::ZoomFit,
                            "Fit Model",
                            10.5,
                            theme.text_secondary,
                            theme.bg_card,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Auto-fit canvas view to active BIM footprint or Global World Map")
                        .clicked()
                        {
                            self.fit_view_to_project(universal_canvas);
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::Palette,
                            "ARI Heat",
                            10.5,
                            if self.show_structural_heatmap_2d { Color32::WHITE } else { theme.text_secondary },
                            if self.show_structural_heatmap_2d { theme.accent_green } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Toggle live FEM ARI / DCR utilization heatmap overlay on 2D columns and structural runs")
                        .clicked()
                        {
                            self.show_structural_heatmap_2d = !self.show_structural_heatmap_2d;
                        }

                        ui.separator();

                        // Shared World Map Basemap, Globe Reset, Ground 1:200 Zoom, Set Origin
                        universal_canvas.render_world_map_toolbar_controls(ui, theme, 26.0);

                        ui.separator();

                        // Toolbar Dock Panel Toggles (below canvas)
                        if icons::icon_button(
                            ui,
                            AppIcon::Folder,
                            "Project & Levels",
                            10.5,
                            if self.floating_project_open { Color32::WHITE } else { theme.text_secondary },
                            if self.floating_project_open { theme.accent_green } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Toggle docked Project Browser & Storey Levels strip inside top toolbar")
                        .clicked()
                        {
                            self.floating_project_open = !self.floating_project_open;
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::SlidersPanel,
                            "Properties",
                            10.5,
                            if self.floating_properties_open { Color32::WHITE } else { theme.text_secondary },
                            if self.floating_properties_open { theme.accent_green } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Toggle docked Properties, Driving Parameters & Canvas Sync inside top toolbar")
                        .clicked()
                        {
                            self.floating_properties_open = !self.floating_properties_open;
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::Cpu,
                            "CLI Log",
                            10.5,
                            if self.show_cli_float { Color32::WHITE } else { theme.text_secondary },
                            if self.show_cli_float { theme.accent_blue.gamma_multiply(0.85) } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Expand or collapse CAD Command Line history inside top toolbar")
                        .clicked()
                        {
                            self.show_cli_float = !self.show_cli_float;
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::Pencil,
                            "Ink Sketch",
                            10.5,
                            if self.sketch_draw_mode { Color32::WHITE } else { theme.text_secondary },
                            if self.sketch_draw_mode { theme.accent_orange } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Toggle Universal Canvas ink sketch & markup drawing mode")
                        .clicked()
                        {
                            self.sketch_draw_mode = !self.sketch_draw_mode;
                            if self.sketch_draw_mode {
                                self.sketch_overlay_synced = true;
                                if universal_canvas.active_tool == crate::views::universal_canvas::UniversalCanvasTool::SelectPan {
                                    universal_canvas.active_tool = crate::views::universal_canvas::UniversalCanvasTool::Freehand;
                                }
                            }
                        }

                        // Sync & Single-File Actions
                        if icons::icon_button(
                            ui,
                            AppIcon::SaveDisk,
                            "Save 1-File",
                            10.5,
                            Color32::WHITE,
                            theme.accent_green,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Save unified workspace (.eigen.json)")
                        .clicked()
                        {
                            universal_canvas.export_unified_file(
                                &self.bim_bus.project,
                                members,
                                &self.variables,
                                workspace_settings.clone(),
                            );
                            self.save_notification = universal_canvas.status_message.clone();
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::UploadImport,
                            "Import",
                            10.5,
                            theme.text_secondary,
                            theme.bg_card,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Import unified .eigen.json workspace archive")
                        .clicked()
                        {
                            universal_canvas.import_json_open = true;
                        }

                        let db_text = format!("DB ({})", db_components.len());
                        if icons::icon_button(
                            ui,
                            AppIcon::Database,
                            db_text,
                            10.5,
                            if self.show_db_drawer { Color32::WHITE } else { theme.text_secondary },
                            if self.show_db_drawer { theme.accent_blue } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Open Parametric Component Catalog")
                        .clicked()
                        {
                            self.show_db_drawer = !self.show_db_drawer;
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::Printer,
                            "PDF/IFC4",
                            10.5,
                            theme.accent_blue,
                            theme.bg_card,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Export Vector PDF, SVG & IFC4 Deliverables")
                        .clicked()
                        {
                            self.export_bim_deliverables();
                        }
                    });

                    ui.add_space(4.0);

                    // Row 2: Active Drafting Tools (Universal Ink Sketch Mode OR CAD + BIMCraft Authoring)
                    ui.horizontal_wrapped(|ui| {
                        if self.sketch_draw_mode {
                            icons::icon_label(
                                ui,
                                AppIcon::Pencil,
                                13.0,
                                theme.accent_orange,
                                "UNIVERSAL SKETCH:",
                                10.5,
                                theme.accent_orange,
                                true,
                            );
                            universal_canvas.render_sketch_tools_bar(ui, theme, 24.0, true);
                            ui.separator();
                            universal_canvas.render_color_swatches(ui, theme);
                            ui.separator();
                            if icons::icon_button(
                                ui,
                                AppIcon::Undo,
                                "Undo Ink",
                                10.0,
                                theme.text_secondary,
                                theme.bg_card,
                                Stroke::NONE,
                                CornerRadius::same(5),
                                Vec2::new(0.0, 24.0),
                            )
                            .clicked()
                            {
                                universal_canvas.undo();
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Layers,
                                if self.sketch_overlay_synced { "Overlay ON" } else { "Overlay OFF" },
                                10.0,
                                if self.sketch_overlay_synced { Color32::WHITE } else { theme.text_secondary },
                                if self.sketch_overlay_synced { theme.accent_blue } else { theme.bg_card },
                                Stroke::NONE,
                                CornerRadius::same(5),
                                Vec2::new(0.0, 24.0),
                            )
                            .clicked()
                            {
                                self.sketch_overlay_synced = !self.sketch_overlay_synced;
                            }
                        } else {
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
                                if icons::icon_button(
                                    ui,
                                    cad_tool_icon(tool),
                                    text,
                                    10.0,
                                    if is_selected { Color32::WHITE } else { theme.text_secondary },
                                    if is_selected { theme.accent_blue } else { theme.bg_card },
                                    Stroke::NONE,
                                    CornerRadius::same(5),
                                    Vec2::new(0.0, 23.0),
                                )
                                .clicked()
                                {
                                    self.active_tool = tool;
                                    self.bim_tool = BimAuthoringTool::CadStandard;
                                    self.pending_start_pt = None;
                                    self.command_history
                                        .push(format!("CAD Tool changed to: {:?}", tool));
                                }
                            }

                            ui.separator();

                            let bim_tools = [
                                (BimAuthoringTool::WallChain, AppIcon::Wall, "Wall [WA]"),
                                (BimAuthoringTool::StructuralBeam, AppIcon::Beam, "Beam [BM]"),
                                (BimAuthoringTool::StructuralColumn, AppIcon::Column, "Col [CL]"),
                                (BimAuthoringTool::PlaceDoor, AppIcon::Door, "Door [DR]"),
                                (BimAuthoringTool::PlaceWindow, AppIcon::Window, "Win [WN]"),
                                (BimAuthoringTool::ReinforceConcrete, AppIcon::Rebar, "Rebar [RF]"),
                            ];
                            for (btool, icon, label) in bim_tools {
                                let active = self.bim_tool == btool;
                                if icons::icon_button(
                                    ui,
                                    icon,
                                    label,
                                    10.0,
                                    if active { Color32::WHITE } else { theme.text_secondary },
                                    if active { theme.accent_orange } else { theme.bg_card },
                                    Stroke::NONE,
                                    CornerRadius::same(5),
                                    Vec2::new(0.0, 23.0),
                                )
                                .clicked()
                                {
                                    self.bim_tool = btool;
                                    self.pending_start_pt = None;
                                }
                            }

                            ui.separator();

                            // Bottom-Toolbar Modify & Rooms Actions (shared with keyboard shortcuts M / O / X / Del)
                            if icons::icon_button(
                                ui,
                                AppIcon::Undo,
                                "Undo",
                                10.0,
                                theme.text_secondary,
                                theme.bg_card,
                                Stroke::NONE,
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Undo last BIM command (Ctrl/Cmd+Z)")
                            .clicked()
                            {
                                if let Some(msg) = self.bim_bus.undo() {
                                    self.save_notification = Some(msg.clone());
                                    self.command_history.push(msg);
                                    self.refresh_live_ari();
                                }
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Redo,
                                "Redo",
                                10.0,
                                theme.text_secondary,
                                theme.bg_card,
                                Stroke::NONE,
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Redo undone BIM command (Ctrl/Cmd+Y)")
                            .clicked()
                            {
                                if let Some(msg) = self.bim_bus.redo() {
                                    self.save_notification = Some(msg.clone());
                                    self.command_history.push(msg);
                                    self.refresh_live_ari();
                                }
                            }
                            ui.label(RichText::new("Step:").size(10.0).color(theme.text_muted));
                            ui.add(
                                egui::DragValue::new(&mut universal_canvas.cad_step_mm)
                                    .range(10.0..=50_000.0)
                                    .speed(50.0)
                                    .suffix(" mm"),
                            )
                            .on_hover_text("Precision step distance for Move & Copy");
                            if icons::icon_button(
                                ui,
                                AppIcon::ArrowRight,
                                "Move",
                                10.0,
                                theme.text_primary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Translate latest element East by Step mm (M)")
                            .clicked()
                            {
                                self.apply_parametric_move_1m(universal_canvas);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Layers,
                                "Copy",
                                10.0,
                                theme.text_primary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Duplicate latest wall/column/sketch East by Step mm")
                            .clicked()
                            {
                                self.apply_parametric_duplicate(universal_canvas);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Compass,
                                "Rot 90°",
                                10.0,
                                theme.text_primary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Rotate latest element or sketch +90°")
                            .clicked()
                            {
                                self.apply_parametric_rotate_90(universal_canvas);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Layers,
                                "Offset Wall",
                                10.0,
                                theme.text_primary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Duplicate and offset latest wall by +500 mm (O)")
                            .clicked()
                            {
                                self.apply_parametric_offset(universal_canvas);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::SectionCut,
                                "Mirror",
                                10.0,
                                theme.text_primary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Mirror walls/columns/beams symmetrically across plan centerline (X)")
                            .clicked()
                            {
                                self.apply_parametric_mirror(universal_canvas);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Trash,
                                "Delete",
                                10.0,
                                theme.accent_red,
                                theme.bg_card,
                                Stroke::new(1.0, theme.accent_red.gamma_multiply(0.35)),
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Delete latest element or selected sketch (Del)")
                            .clicked()
                            {
                                self.apply_parametric_delete(universal_canvas);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Building,
                                "Auto Rooms",
                                10.0,
                                theme.accent_green,
                                theme.bg_card,
                                Stroke::NONE,
                                CornerRadius::same(5),
                                Vec2::new(0.0, 23.0),
                            )
                            .on_hover_text("Auto-detect enclosed planar rooms on active level")
                            .clicked()
                            {
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
                                    self.save_notification = Some(msg.clone());
                                    self.command_history.push(msg);
                                }
                            }
                        }
                    });

                    // Sheet Composer 2D Control Bar
                    if self.view_mode == DraftingViewMode::SheetLayoutA101 {
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            icons::icon_label(
                                ui,
                                AppIcon::Document,
                                13.0,
                                theme.accent_blue,
                                "Sheet Config:",
                                11.0,
                                theme.accent_blue,
                                true,
                            );

                            for p in [
                                SheetPaperSize::A4Landscape,
                                SheetPaperSize::A3Landscape,
                                SheetPaperSize::A2Landscape,
                                SheetPaperSize::A1Landscape,
                            ] {
                                let label = match p {
                                    SheetPaperSize::A4Landscape => "A4",
                                    SheetPaperSize::A3Landscape => "A3",
                                    SheetPaperSize::A2Landscape => "A2",
                                    SheetPaperSize::A1Landscape => "A1",
                                };
                                if ui.selectable_label(self.sheet_paper_size == p, label).clicked() {
                                    self.sheet_paper_size = p;
                                }
                            }

                            ui.separator();

                            for s in [SheetScale::Scale1_50, SheetScale::Scale1_100, SheetScale::Scale1_200] {
                                if ui.selectable_label(self.sheet_scale == s, s.label()).clicked() {
                                    self.sheet_scale = s;
                                }
                            }

                            ui.separator();

                            crate::ui_kit::wrap_before(ui, 95.0);
                            ui.label(RichText::new("No:").size(10.5).color(theme.text_secondary));
                            ui.add(egui::TextEdit::singleline(&mut self.sheet_number).desired_width(45.0));

                            let title_w = crate::ui_kit::clamped_row_width(ui, 72.0, 170.0, 62.0);
                            crate::ui_kit::wrap_before(ui, title_w + 56.0);
                            ui.label(RichText::new("Title:").size(10.5).color(theme.text_secondary));
                            ui.add(egui::TextEdit::singleline(&mut self.sheet_title).desired_width(title_w));

                            crate::ui_kit::wrap_before(ui, 165.0);
                            if icons::icon_button(
                                ui,
                                AppIcon::Printer,
                                "PDF",
                                10.5,
                                Color32::WHITE,
                                theme.accent_blue,
                                Stroke::NONE,
                                CornerRadius::same(5),
                                Vec2::new(0.0, 24.0),
                            )
                            .on_hover_text("Export scaled Sheet to vector PDF")
                            .clicked()
                            {
                                self.export_sheet_format(SheetExportFormat::Pdf);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Document,
                                "DXF",
                                10.5,
                                Color32::WHITE,
                                theme.accent_green,
                                Stroke::NONE,
                                CornerRadius::same(5),
                                Vec2::new(0.0, 24.0),
                            )
                            .on_hover_text("Export CAD Sheet layers directly to AutoCAD DXF")
                            .clicked()
                            {
                                self.export_sheet_format(SheetExportFormat::Dxf);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Layers,
                                "SVG",
                                10.5,
                                theme.text_primary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(5),
                                Vec2::new(0.0, 24.0),
                            )
                            .on_hover_text("Export crisp vector SVG sheet")
                            .clicked()
                            {
                                self.export_sheet_format(SheetExportFormat::Svg);
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::SaveDisk,
                                "All Pack",
                                10.5,
                                theme.text_secondary,
                                theme.bg_card,
                                Stroke::NONE,
                                CornerRadius::same(5),
                                Vec2::new(0.0, 24.0),
                            )
                            .on_hover_text("Export PDF + DXF + SVG + IFC4 delivery pack")
                            .clicked()
                            {
                                self.export_bim_deliverables();
                            }
                        });
                    }

                    // Row 2B: Organized Levels Quick Bar + Inline CAD Command Line inside Top Toolbar
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        let lvl_w = crate::ui_kit::clamped_row_width(ui, 80.0, 138.0, 118.0);
                        crate::ui_kit::wrap_before(ui, lvl_w + 115.0);
                        icons::icon_label(
                            ui,
                            AppIcon::Layers,
                            12.0,
                            theme.accent_blue,
                            "Level:",
                            10.5,
                            theme.text_secondary,
                            true,
                        );
                        let active_lvl_text = self
                            .bim_bus
                            .project
                            .levels
                            .get(self.active_level_idx)
                            .map(|l| format!("{} ({:+.1} m)", l.name, l.elevation_mm * 1e-3))
                            .unwrap_or_else(|| "Level 1 (+0.0 m)".to_string());
                        egui::ComboBox::from_id_salt("param_top_level_combo")
                            .width(lvl_w)
                            .selected_text(RichText::new(active_lvl_text).size(10.5).color(theme.text_primary))
                            .show_ui(ui, |ui| {
                                for (idx, lvl) in self.bim_bus.project.levels.iter().enumerate() {
                                    let label = format!("{} ({:+.1} m)", lvl.name, lvl.elevation_mm * 1e-3);
                                    if ui.selectable_label(self.active_level_idx == idx, label).clicked() {
                                        self.active_level_idx = idx;
                                    }
                                }
                            });
                        if ui
                            .add(
                                egui::Button::new(RichText::new("+ Lvl").size(10.0).color(theme.accent_green))
                                    .fill(theme.bg_card)
                                    .corner_radius(CornerRadius::same(5))
                                    .min_size(Vec2::new(0.0, 22.0)),
                            )
                            .on_hover_text("Add a new storey level (+3.5 m)")
                            .clicked()
                        {
                            let next_num = self.bim_bus.project.levels.len() + 1;
                            let next_elev = self
                                .bim_bus
                                .project
                                .levels
                                .last()
                                .map(|l| l.elevation_mm + 3500.0)
                                .unwrap_or(0.0);
                            let _ = self.bim_bus.execute(BimCommand::AddLevel {
                                name: format!("Level {}", next_num),
                                elevation_mm: next_elev,
                            });
                            self.active_level_idx = self.bim_bus.project.levels.len().saturating_sub(1);
                            self.refresh_live_ari();
                        }

                        let cli_w = crate::ui_kit::clamped_row_width(ui, 68.0, 175.0, 146.0);
                        crate::ui_kit::wrap_before(ui, cli_w + 146.0);
                        icons::icon_label(
                            ui,
                            AppIcon::Cpu,
                            12.0,
                            theme.accent_blue,
                            "CAD Cmd:",
                            10.5,
                            theme.accent_blue,
                            true,
                        );
                        let cli_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.command_line_input)
                                .desired_width(cli_w)
                                .hint_text("@dx,dy | @d<deg | ro 90 | wall 0,0->6000,0"),
                        );
                        if ((cli_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            || ui
                                .add(
                                    egui::Button::new(RichText::new("Run").size(10.5).color(Color32::WHITE))
                                        .fill(theme.accent_blue)
                                        .corner_radius(CornerRadius::same(5))
                                        .min_size(Vec2::new(0.0, 22.0)),
                                )
                                .clicked())
                            && !self.command_line_input.trim().is_empty()
                        {
                            let raw = self.command_line_input.trim().to_string();
                            self.command_history.push(format!("Command: {raw}"));
                            self.execute_cad_command(&raw);
                            self.command_line_input.clear();
                            cli_resp.request_focus();
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::Shield,
                            "? Shortcuts",
                            10.0,
                            if universal_canvas.shortcuts_modal_open {
                                Color32::WHITE
                            } else {
                                theme.text_secondary
                            },
                            if universal_canvas.shortcuts_modal_open {
                                theme.accent_blue
                            } else {
                                theme.bg_card
                            },
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(5),
                            Vec2::new(0.0, 22.0),
                        )
                        .on_hover_text("Open CAD Shortcuts, Relative/Polar CLI & Quick-Jump Palette (Ctrl/Cmd+K)")
                        .clicked()
                        {
                            universal_canvas.shortcuts_modal_open = !universal_canvas.shortcuts_modal_open;
                        }
                    });
                    if let Some(last_log) = self.command_history.last() {
                        ui.add_space(2.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(last_log)
                                    .size(10.0)
                                    .color(theme.text_muted),
                            )
                            .wrap(),
                        );
                    }

                    // Row 3: Expandable Top-Toolbar Docked Inspectors (Project/Levels, Properties/Variables/Canvas, CLI Log)
                    self.render_top_toolbar_inspector_dock(
                        ui,
                        theme,
                        db_components,
                        active_pillar,
                        features,
                        universal_canvas,
                        members,
                        workspace_settings,
                    );
                });
            });
    }

    pub(super) fn export_sheet_format(&mut self, fmt: SheetExportFormat) {
        let safe_title = self.sheet_title.replace([' ', '/', '\\'], "_");
        let pdf_filename = format!("Sheet_{}_{}.pdf", self.sheet_number, safe_title);
        let dxf_filename = format!("Sheet_{}_{}.dxf", self.sheet_number, safe_title);
        let svg_filename = format!("Sheet_{}_{}.svg", self.sheet_number, safe_title);
        let ifc_filename = format!(
            "{}.ifc",
            self.bim_bus.project.project_name.replace([' ', '/', '\\'], "_")
        );

        let pdf_bytes = if matches!(fmt, SheetExportFormat::Pdf | SheetExportFormat::All) {
            Some(
                self.bim_bus
                    .project
                    .export_sheet_pdf(&self.sheet_number, &self.sheet_title),
            )
        } else {
            None
        };
        let dxf_str = if matches!(fmt, SheetExportFormat::Dxf | SheetExportFormat::All) {
            Some(
                self.bim_bus
                    .project
                    .export_sheet_dxf(&self.sheet_number, &self.sheet_title),
            )
        } else {
            None
        };
        let svg_str = if matches!(fmt, SheetExportFormat::Svg | SheetExportFormat::All) {
            Some(
                self.bim_bus
                    .project
                    .export_sheet_svg(&self.sheet_number, &self.sheet_title),
            )
        } else {
            None
        };
        let ifc_str = if fmt == SheetExportFormat::All {
            Some(self.bim_bus.project.export_ifc4_spatial())
        } else {
            None
        };

        #[cfg(target_arch = "wasm32")]
        {
            let mut exported = Vec::new();
            if let Some(ref bytes) = pdf_bytes {
                let _ = crate::core::web_bridge::trigger_browser_download(
                    &pdf_filename,
                    bytes,
                    "application/pdf",
                );
                exported.push(pdf_filename);
            }
            if let Some(ref dxf) = dxf_str {
                let _ = crate::core::web_bridge::trigger_browser_download(
                    &dxf_filename,
                    dxf.as_bytes(),
                    "application/dxf;charset=utf-8",
                );
                exported.push(dxf_filename);
            }
            if let Some(ref svg) = svg_str {
                let _ = crate::core::web_bridge::trigger_browser_download(
                    &svg_filename,
                    svg.as_bytes(),
                    "image/svg+xml;charset=utf-8",
                );
                exported.push(svg_filename);
            }
            if let Some(ref ifc) = ifc_str {
                let _ = crate::core::web_bridge::trigger_browser_download(
                    &ifc_filename,
                    ifc.as_bytes(),
                    "application/x-step;charset=utf-8",
                );
                exported.push(ifc_filename);
            }
            self.save_notification = Some(format!("Exported {} via browser downloads", exported.join(", ")));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let export_dir = crate::core::InternalVault::vault_root_dir().join("Exports");
            let _ = std::fs::create_dir_all(&export_dir);
            let mut exported = Vec::new();
            if let Some(bytes) = pdf_bytes {
                let _ = std::fs::write(export_dir.join(&pdf_filename), bytes);
                exported.push(pdf_filename);
            }
            if let Some(dxf) = dxf_str {
                let _ = std::fs::write(export_dir.join(&dxf_filename), dxf);
                exported.push(dxf_filename);
            }
            if let Some(svg) = svg_str {
                let _ = std::fs::write(export_dir.join(&svg_filename), svg);
                exported.push(svg_filename);
            }
            if let Some(ifc) = ifc_str {
                let _ = std::fs::write(export_dir.join(&ifc_filename), ifc);
                exported.push(ifc_filename);
            }
            self.save_notification = Some(format!(
                "Exported {} to {}",
                exported.join(", "),
                export_dir.display()
            ));
        }
    }

    pub(super) fn export_bim_deliverables(&mut self) {
        self.export_sheet_format(SheetExportFormat::All);
    }

}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SheetExportFormat {
    Pdf,
    Dxf,
    Svg,
    All,
}
