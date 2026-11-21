//! views/parametric/inspector.rs — Floating In-Canvas HUD Panels for Parametric Variables,
//! Storey Levels, Rooms, Component Database, Bidirectional Sync & Universal Sync Controls.

use super::*;

impl ParametricView {
    /// Body content for Project Browser: Active Storey Level, View Range, Auto-Bounded Rooms,
    /// OpenBIM Component Database quick save/open, and Live BIM -> FEM/ARI telemetry.
    pub(super) fn render_project_levels_body(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        db_components: &mut Vec<DatabaseComponent>,
    ) {
        // Active Storey Level Selector
        theme.card_frame().show(ui, |ui| {
            icons::icon_label(
                ui,
                AppIcon::Building,
                12.5,
                theme.accent_blue,
                "ACTIVE STOREY LEVEL",
                10.5,
                theme.accent_blue,
                true,
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
                    "Cut: +{:.0}mm | Rooms: {} | Rebar: {:.1} kg",
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
                icons::icon_label(
                    ui,
                    AppIcon::Layers,
                    12.0,
                    theme.accent_orange,
                    "ROOMS (HALF-EDGE)",
                    10.0,
                    theme.accent_orange,
                    true,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::Redo,
                        "Recalc",
                        10.0,
                        theme.text_primary,
                        theme.bg_sidebar,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(5),
                        Vec2::new(0.0, 20.0),
                    )
                    .clicked()
                    {
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
                            RichText::new(format!("{:.1} m² ({:.1}m)", rm.area_m2, rm.perimeter_m))
                                .size(10.0)
                                .strong()
                                .color(theme.accent_green),
                        );
                    });
                });
            }
        });

        ui.add_space(6.0);

        // Component Database Quick Save & Catalog Trigger
        theme.card_frame().show(ui, |ui| {
            icons::icon_label(
                ui,
                AppIcon::Folder,
                12.0,
                theme.accent_blue,
                "COMPONENT DATABASE",
                10.0,
                theme.accent_blue,
                true,
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if icons::icon_button(
                    ui,
                    AppIcon::SaveDisk,
                    "Save Comp",
                    10.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(5),
                    Vec2::new(0.0, 22.0),
                )
                .clicked()
                {
                    self.save_current_to_db(db_components);
                }

                if icons::icon_button(
                    ui,
                    AppIcon::Layers,
                    &format!("DB ({})", db_components.len()),
                    10.5,
                    theme.text_primary,
                    theme.bg_sidebar,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(5),
                    Vec2::new(0.0, 22.0),
                )
                .clicked()
                {
                    self.show_db_drawer = !self.show_db_drawer;
                }
            });
        });

        ui.add_space(6.0);

        // Live BIM -> FEM/ARI Telemetry
        Frame::new()
            .fill(if theme.is_dark() {
                Color32::from_rgb(18, 22, 28)
            } else {
                theme.bg_sidebar
            })
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_card))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Bolt,
                    12.0,
                    theme.accent_blue,
                    "LIVE BIM -> FEM/ARI SOLVER",
                    10.0,
                    theme.accent_blue,
                    true,
                );
                ui.label(
                    RichText::new(format!(
                        "Global ARI: {:.3} (Safe/Redundant)",
                        self.live_ari_score
                    ))
                    .size(10.5)
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
    }

    /// Body content for Properties Inspector: Component Metadata, Driving Parametric Variables,
    /// 5D/6D Workflow Links, Bidirectional Sync, and OpenBIM Vector/IFCX Deliverables.
    pub(super) fn render_properties_body(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        active_pillar: &mut NavigationPillar,
        features: &mut FeatureRegistry,
    ) {
        ui.label(
            RichText::new("Component Name:")
                .size(10.5)
                .color(theme.text_muted),
        );
        ui.text_edit_singleline(&mut self.component_name);

        ui.add_space(4.0);
        ui.label(
            RichText::new("Driving Dimensions & Parameters")
                .size(11.0)
                .strong()
                .color(theme.text_primary),
        );
        ui.add_space(4.0);

        for var in &mut self.variables {
            theme.card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&var.name)
                            .size(11.0)
                            .strong()
                            .color(theme.accent_blue),
                    );
                    ui.label(RichText::new("=").size(11.0).color(theme.text_secondary));
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut var.expression)
                                .desired_width(64.0),
                        )
                        .changed()
                    {
                        if let Ok(val) = var.expression.parse::<f64>() {
                            var.evaluated_value = val;
                        }
                    }
                    ui.label(RichText::new(&var.unit).size(9.5).color(theme.text_muted));
                });
            });
            ui.add_space(2.0);
        }

        ui.add_space(4.0);
        if ui.button("+ Add Variable Parameter").clicked() {
            self.variables.push(ParametricVariable {
                name: format!("var_{}", self.variables.len() + 1),
                expression: "1000".to_string(),
                evaluated_value: 1000.0,
                unit: "mm".to_string(),
            });
        }

        ui.add_space(6.0);
        if icons::icon_button(
            ui,
            AppIcon::ChartBar,
            "Budgeting (5D BoQ)",
            10.5,
            Color32::WHITE,
            theme.accent_blue,
            Stroke::NONE,
            CornerRadius::same(6),
            Vec2::new(ui.available_width(), 26.0),
        )
        .clicked()
        {
            *active_pillar = NavigationPillar::CostBudgeting;
        }

        ui.add_space(4.0);
        if icons::icon_button(
            ui,
            AppIcon::Leaf,
            "Sustainability (6D Carbon)",
            10.5,
            theme.text_primary,
            theme.bg_card,
            Stroke::NONE,
            CornerRadius::same(6),
            Vec2::new(ui.available_width(), 26.0),
        )
        .clicked()
        {
            *active_pillar = NavigationPillar::SustainabilityEnergy;
        }

        ui.add_space(6.0);
        ui_kit::feature_slot_card(
            ui,
            theme,
            features,
            FeatureId::ParametricBidirectionalSync,
            |ui| {
                ui.vertical(|ui| {
                    let _ = ui.selectable_label(true, "Source: ParametricView -> BuildView");
                    let _ = ui.selectable_label(false, "Auto-Sync Dynamo Bakes & 2D Walls");
                    if icons::icon_button(
                        ui,
                        AppIcon::Redo,
                        "Push 2D Scene -> 3D BIM Bus",
                        10.0,
                        theme.text_primary,
                        theme.bg_sidebar,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(5),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked()
                    {
                        self.refresh_live_ari();
                        self.save_notification = Some(
                            "Pushed 2D Parametric floor plan & variables to 3D BIM Command Bus."
                                .to_string(),
                        );
                    }
                });
            },
        );

        ui.add_space(6.0);
        if icons::icon_button(
            ui,
            AppIcon::Printer,
            "Export PDF / SVG / IFCX",
            10.5,
            Color32::WHITE,
            theme.accent_green,
            Stroke::NONE,
            CornerRadius::same(6),
            Vec2::new(ui.available_width(), 26.0),
        )
        .clicked()
        {
            self.export_bim_deliverables();
        }
    }

    /// Organized Top-Toolbar Docked Inspector Strip (replaces floating panels over the canvas).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_top_toolbar_inspector_dock(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        db_components: &mut Vec<DatabaseComponent>,
        active_pillar: &mut NavigationPillar,
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
        members: &[crate::models::StructuralMember],
        workspace_settings: WorkspaceCanvasSettings,
    ) {
        if !self.floating_project_open && !self.floating_properties_open && !self.show_cli_float {
            return;
        }
        ui.add_space(6.0);
        Frame::new()
            .fill(theme.bg_card)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let show_two_cols = self.floating_project_open && self.floating_properties_open;
                let avail = ui.available_width();
                if show_two_cols && avail >= 820.0 {
                    ui.columns(2, |cols| {
                        cols[0].push_id("param_dock_project", |ui| {
                            self.render_floating_project_card(ui, theme, db_components, false);
                        });
                        cols[1].push_id("param_dock_props", |ui| {
                            self.render_floating_properties_card(
                                ui,
                                theme,
                                db_components,
                                active_pillar,
                                features,
                                universal_canvas,
                                members,
                                workspace_settings.clone(),
                                avail * 0.5,
                                false,
                            );
                        });
                    });
                } else {
                    if self.floating_project_open {
                        ui.push_id("param_dock_project_solo", |ui| {
                            self.render_floating_project_card(ui, theme, db_components, false);
                        });
                        if self.floating_properties_open {
                            ui.add_space(6.0);
                        }
                    }
                    if self.floating_properties_open {
                        ui.push_id("param_dock_props_solo", |ui| {
                            self.render_floating_properties_card(
                                ui,
                                theme,
                                db_components,
                                active_pillar,
                                features,
                                universal_canvas,
                                members,
                                workspace_settings.clone(),
                                avail,
                                false,
                            );
                        });
                    }
                }
                if self.show_cli_float {
                    ui.add_space(6.0);
                    ui.push_id("param_dock_cli", |ui| {
                        self.render_floating_cli_card(ui, theme, false);
                    });
                }
            });
    }

    /// Docked card for Project Browser & Storey Levels inside the top toolbar.
    pub(super) fn render_floating_project_card(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        db_components: &mut Vec<DatabaseComponent>,
        is_overlay: bool,
    ) {
        UniversalCanvasState::floating_panel_frame(theme, is_overlay).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Layers,
                    13.0,
                    theme.accent_blue,
                    "Project & Levels",
                    11.5,
                    theme.text_primary,
                    true,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("×").size(14.0).color(theme.text_muted),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .on_hover_text("Close Project & Levels overlay")
                        .clicked()
                    {
                        self.floating_project_open = false;
                    }
                });
            });
            ui.separator();
            ui.push_id("parametric_floating_project_scroll", |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_project_levels_body(ui, theme, db_components);
                });
            });
        });
    }

    /// Floating HUD card for Properties, Driving Parameters & Universal Canvas Sync inside canvas.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_floating_properties_card(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        db_components: &mut Vec<DatabaseComponent>,
        active_pillar: &mut NavigationPillar,
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
        members: &[crate::models::StructuralMember],
        workspace_settings: WorkspaceCanvasSettings,
        avail_w: f32,
        is_overlay: bool,
    ) {
        UniversalCanvasState::floating_panel_frame(theme, is_overlay).show(ui, |ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.horizontal_wrapped(|ui| {
                    let show_project_tab = avail_w < 960.0 || !self.floating_project_open;
                    let tabs = [
                        (ParametricFloatingTab::Properties, "Properties", true),
                        (ParametricFloatingTab::Project, "Project", show_project_tab),
                        (ParametricFloatingTab::CanvasSync, "Canvas", true),
                    ];
                    for (tab, label, visible) in tabs {
                        if !visible {
                            continue;
                        }
                        let active = self.floating_right_tab == tab;
                        if ui
                            .add(
                                egui::Button::new(RichText::new(label).size(10.5).color(
                                    if active {
                                        Color32::WHITE
                                    } else {
                                        theme.text_secondary
                                    },
                                ))
                                .fill(if active {
                                    theme.accent_blue
                                } else {
                                    theme.bg_card
                                })
                                .corner_radius(CornerRadius::same(5))
                                .min_size(Vec2::new(0.0, 22.0)),
                            )
                            .clicked()
                        {
                            self.floating_right_tab = tab;
                        }
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("×").size(14.0).color(theme.text_muted),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .on_hover_text("Close Properties Inspector overlay")
                        .clicked()
                    {
                        self.floating_properties_open = false;
                    }
                });
            });

            ui.separator();

            ui.push_id("parametric_floating_right_scroll", |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match self.floating_right_tab {
                    ParametricFloatingTab::Properties => {
                        self.render_properties_body(ui, theme, active_pillar, features);
                    }
                    ParametricFloatingTab::Project => {
                        self.render_project_levels_body(ui, theme, db_components);
                    }
                    ParametricFloatingTab::CanvasSync => {
                        universal_canvas.render_canvas_sync_controls(
                            ui,
                            theme,
                            NavigationPillar::ParametricDED,
                            &self.bim_bus.project,
                            members,
                            &self.variables,
                            workspace_settings,
                            false,
                        );
                    }
                });
            });
        });
    }

    /// Floating or stacked Command Line Interface HUD card inside the 2D drafting studio.
    pub(super) fn render_floating_cli_card(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        is_overlay: bool,
    ) {
        UniversalCanvasState::floating_panel_frame(theme, is_overlay).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Cpu,
                    12.0,
                    theme.accent_blue,
                    "CAD Command Line",
                    11.0,
                    theme.text_primary,
                    true,
                );
                ui.label(
                    RichText::new("(WA, BM, CL, DR, WN, RM, RF, UNDO, REDO, PDF)")
                        .size(9.5)
                        .color(theme.text_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("×").size(14.0).color(theme.text_muted),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .on_hover_text("Hide Command Line overlay")
                        .clicked()
                    {
                        self.show_cli_float = false;
                    }
                });
            });

            ui.push_id("parametric_floating_cli_scroll", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(46.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let start = self.command_history.len().saturating_sub(4);
                        for line in &self.command_history[start..] {
                            ui.label(RichText::new(line).size(10.0).color(theme.text_secondary));
                        }
                    });
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Command:")
                        .size(10.5)
                        .strong()
                        .color(theme.accent_blue),
                );
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.command_line_input)
                        .desired_width((ui.available_width() - 125.0).max(60.0))
                        .hint_text("Type WA, BM, CL, DR, WN, RM, RF, @dx,dy, @d<deg..."),
                );
                if ((resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    || ui.button("Run").clicked())
                    && !self.command_line_input.trim().is_empty()
                {
                    let raw = self.command_line_input.trim().to_string();
                    self.command_history.push(format!("Command: {raw}"));
                    self.execute_cad_command(&raw);
                    self.command_line_input.clear();
                    resp.request_focus();
                }
            });
        });
    }
}
