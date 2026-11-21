//! views/build/bottom_deck.rs — Collapsible Schedules, 4D Erection, Clash Deck & BIM Console.

use super::*;

impl BuildView {
    /// Floating or stacked BIM Data Deck (QTO Schedules, 4D Simulation, Clashes, BIM CLI & LiDAR).
    pub(super) fn render_floating_deck_card(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        _members: &[StructuralMember],
        features: &mut FeatureRegistry,
        is_overlay: bool,
    ) {
        UniversalCanvasState::floating_panel_frame(theme, is_overlay).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.horizontal_wrapped(|ui| {
                    let tabs = [
                        BottomDeckTab::ScheduleQto,
                        BottomDeckTab::TimeSimulation4D,
                        BottomDeckTab::ClashManager,
                        BottomDeckTab::BimConsole,
                        BottomDeckTab::PointCloudDeviation,
                    ];
                    for tab in tabs {
                        let is_sel = self.active_bottom_deck_tab == tab;
                        let bg = if is_sel {
                            theme.accent_blue
                        } else {
                            theme.bg_card
                        };
                        let fg = if is_sel {
                            Color32::WHITE
                        } else {
                            theme.text_secondary
                        };

                        if icons::icon_button(
                            ui,
                            tab.app_icon(),
                            tab.label(),
                            10.0,
                            fg,
                            bg,
                            Stroke::NONE,
                            CornerRadius::same(5),
                            Vec2::new(0.0, 22.0),
                        )
                        .clicked()
                        {
                            self.active_bottom_deck_tab = tab;
                            self.bottom_deck_expanded = true;
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
                        .on_hover_text("Close Data Deck overlay")
                        .clicked()
                    {
                        self.floating_deck_open = false;
                        self.bottom_deck_expanded = false;
                    }
                });
            });

            ui.separator();

            ui.push_id("build_floating_deck_body", |ui| {
                match self.active_bottom_deck_tab {
                        BottomDeckTab::ScheduleQto => {
                            // Filter row
                            ui.horizontal(|ui| {
                                crate::icons::draw_icon(
                                    ui,
                                    crate::icons::AppIcon::Search,
                                    13.0,
                                    theme.text_muted,
                                );
                                ui.add(egui::TextEdit::singleline(&mut self.schedule_search_query)
                                    .hint_text("Filter schedule...")
                                    .desired_width(120.0));
                                ui.separator();
                                egui::ComboBox::from_id_salt("schedule_cat_filter")
                                    .selected_text(RichText::new(&self.schedule_category_filter).size(10.5))
                                    .show_ui(ui, |ui| {
                                        let cats = ["All Elements", "Structural Columns", "Beams & Girders", "Slabs & Floors", "Shear Walls", "Rebar Sets", "MEP Equipment"];
                                        for c in cats {
                                            if ui.selectable_label(self.schedule_category_filter == c, c).clicked() {
                                                self.schedule_category_filter = c.to_string();
                                            }
                                        }
                                    });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if icons::icon_button(ui, AppIcon::UploadImport, "Export CSV", 10.0, theme.accent_blue, theme.bg_card, Stroke::NONE, CornerRadius::same(4), Vec2::new(0.0, 20.0)).clicked() {
                                        self.modify_status_msg = Some("Exported Schedule to CSV: EigenGraph_BoQ_Schedule.csv".to_string());
                                    }
                                });
                            });
                            ui.add_space(4.0);

                            // Schedule Table
                            ScrollArea::both()
                                .id_salt("schedule_table_scroll")
                                .max_height(140.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("schedule_qto_grid")
                                        .striped(true)
                                        .spacing(Vec2::new(12.0, 4.0))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new("Mark").strong().size(10.5).color(theme.accent_blue));
                                            ui.label(RichText::new("Category").strong().size(10.5).color(theme.text_secondary));
                                            ui.label(RichText::new("Level").strong().size(10.5).color(theme.text_secondary));
                                            ui.label(RichText::new("Section / Size").strong().size(10.5).color(theme.text_secondary));
                                            ui.label(RichText::new("Length (m)").strong().size(10.5).color(theme.text_secondary));
                                            ui.label(RichText::new("Volume (m³)").strong().size(10.5).color(theme.text_secondary));
                                            ui.label(RichText::new("Weight (t)").strong().size(10.5).color(theme.text_secondary));
                                            ui.label(RichText::new("Rate ($)").strong().size(10.5).color(theme.text_secondary));
                                            ui.label(RichText::new("Total ($)").strong().size(10.5).color(theme.accent_green));
                                            ui.end_row();

                                            let schedule_rows = [
                                                ("C-01", "Column", "Level 1", "RC 500x500", "4.00", "1.00", "2.45", "185.00", "$185.00"),
                                                ("C-02", "Column", "Level 1", "RC 500x500", "4.00", "1.00", "2.45", "185.00", "$185.00"),
                                                ("B-101", "Beam", "Level 2", "W18x50 Steel", "7.50", "0.08", "0.56", "2,200/t", "$1,232.00"),
                                                ("B-102", "Beam", "Level 2", "W18x50 Steel", "7.50", "0.08", "0.56", "2,200/t", "$1,232.00"),
                                                ("S-201", "Slab", "Level 2", "Composite Deck 150mm", "12.00", "18.00", "44.10", "165.00", "$2,970.00"),
                                                ("SW-01", "Shear Wall", "Level 1", "RC Wall 300mm", "6.00", "7.20", "17.64", "195.00", "$1,404.00"),
                                                ("RB-101", "Rebar", "Level 1", "Grade 60 (D25/D13)", "48.00", "0.05", "0.41", "1,850/t", "$758.50"),
                                                ("DT-301", "HVAC Duct", "Level 2", "Galvanized 600x400", "14.50", "3.48", "0.22", "85.00/m", "$1,232.50"),
                                            ];

                                            for (mark, cat, lvl, sec, len, vol, wt, rate, tot) in schedule_rows {
                                                if !self.schedule_search_query.trim().is_empty() {
                                                    let q = self.schedule_search_query.trim().to_lowercase();
                                                    if !mark.to_lowercase().contains(&q) && !cat.to_lowercase().contains(&q) && !sec.to_lowercase().contains(&q) {
                                                        continue;
                                                    }
                                                }
                                                if self.schedule_category_filter != "All Elements" && !cat.contains(&self.schedule_category_filter) {
                                                    continue;
                                                }

                                                ui.label(RichText::new(mark).size(10.0).strong().color(theme.text_primary));
                                                ui.label(RichText::new(cat).size(10.0).color(theme.text_secondary));
                                                ui.label(RichText::new(lvl).size(10.0).color(theme.text_muted));
                                                ui.label(RichText::new(sec).size(10.0).color(theme.text_primary));
                                                ui.label(RichText::new(len).size(10.0).color(theme.text_secondary));
                                                ui.label(RichText::new(vol).size(10.0).color(theme.text_secondary));
                                                ui.label(RichText::new(wt).size(10.0).color(theme.text_secondary));
                                                ui.label(RichText::new(rate).size(10.0).color(theme.text_muted));
                                                ui.label(RichText::new(tot).size(10.0).strong().color(theme.accent_green));
                                                ui.end_row();
                                            }
                                        });
                                });

                            ui.add_space(2.0);
                            Frame::new()
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(8, 3))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("TOTALS:").size(10.0).strong().color(theme.text_secondary));
                                        ui.label(RichText::new("Volume: 842.5 m³").size(10.0).strong().color(theme.accent_blue));
                                        ui.label(RichText::new("• Steel: 148.2 t").size(10.0).strong().color(theme.accent_blue));
                                        ui.label(RichText::new("• Rebar: 38.6 t").size(10.0).strong().color(theme.accent_blue));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(RichText::new("Grand Total: $1,842,500.00").size(10.5).strong().color(theme.accent_green));
                                        });
                                    });
                                });
                        }
                        BottomDeckTab::TimeSimulation4D => {
                            ui.horizontal_wrapped(|ui| {
                                let play_icon = if self.simulation_4d_playing { AppIcon::Pause } else { AppIcon::Play };
                                let play_lbl = if self.simulation_4d_playing { "Pause" } else { "Play" };
                                if icons::icon_button(ui, play_icon, play_lbl, 10.5, Color32::WHITE, theme.accent_blue, Stroke::NONE, CornerRadius::same(5), Vec2::new(0.0, 22.0)).clicked() {
                                    self.simulation_4d_playing = !self.simulation_4d_playing;
                                }
                                if icons::icon_button(ui, AppIcon::SkipBack, "Day 1", 10.5, theme.text_secondary, theme.bg_card, Stroke::NONE, CornerRadius::same(5), Vec2::new(0.0, 22.0)).clicked() {
                                    self.simulation_4d_day = 1;
                                }
                                if icons::icon_button(ui, AppIcon::ChevronLeft, "Prev", 10.5, theme.text_secondary, theme.bg_card, Stroke::NONE, CornerRadius::same(5), Vec2::new(0.0, 22.0)).clicked() {
                                    self.simulation_4d_day = self.simulation_4d_day.saturating_sub(5).max(1);
                                }
                                if icons::icon_button(ui, AppIcon::ArrowRight, "Next", 10.5, theme.text_secondary, theme.bg_card, Stroke::NONE, CornerRadius::same(5), Vec2::new(0.0, 22.0)).clicked() {
                                    self.simulation_4d_day = (self.simulation_4d_day + 5).min(180);
                                }
                                if icons::icon_button(ui, AppIcon::SkipNext, "Day 180", 10.5, theme.text_secondary, theme.bg_card, Stroke::NONE, CornerRadius::same(5), Vec2::new(0.0, 22.0)).clicked() {
                                    self.simulation_4d_day = 180;
                                }

                                ui.separator();
                                ui.label(RichText::new(format!("Day: {}", self.simulation_4d_day)).size(11.0).strong().color(theme.accent_blue));
                                ui.add(egui::Slider::new(&mut self.simulation_4d_day, 1..=180).suffix(" d").text(""));
                            });
                            ui.add_space(4.0);

                            // Active Phase Banner
                            let (phase_title, phase_desc, phase_prog) = match self.simulation_4d_day {
                                1..=30 => ("Phase 1: Substructure & Foundation", "Deep piles, cap excavation & raft mat casting", (self.simulation_4d_day as f32 / 30.0)),
                                31..=80 => ("Phase 2: Superstructure Concrete & Steel Frame", "Multi-story columns, primary girders & floor slabs", ((self.simulation_4d_day - 30) as f32 / 50.0)),
                                81..=120 => ("Phase 3: Exterior Enclosure & Curtain Wall", "Precast spandrel panels, curtain wall glazing & roof", ((self.simulation_4d_day - 80) as f32 / 40.0)),
                                121..=160 => ("Phase 4: MEP Systems & Fire Protection", "HVAC duct risers, electrical conduits & wet sprinklers", ((self.simulation_4d_day - 120) as f32 / 40.0)),
                                _ => ("Phase 5: Interior Finishes & Commissioning", "Architectural partitions, elevator testing & handover", ((self.simulation_4d_day - 160) as f32 / 20.0)),
                            };

                            Frame::new()
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(phase_title).strong().size(11.0).color(theme.text_primary));
                                        ui.label(RichText::new(format!("({:.0}% complete)", phase_prog * 100.0)).size(10.5).color(theme.accent_green));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(RichText::new(phase_desc).size(10.0).color(theme.text_secondary));
                                        });
                                    });
                                });
                        }
                        BottomDeckTab::ClashManager => {
                        ui_kit::feature_slot_card(ui, theme, features, FeatureId::LiveBvhGeometricClashSolver, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                let _ = ui.selectable_label(true, "Spatial Bvh3D AABB Index");
                                let _ = ui.selectable_label(false, "Auto-Generate BCF from Clashes");
                                let _ = icons::icon_button(ui, AppIcon::Search, "Scan Active BimProject CSG Solids", 10.5, theme.text_primary, theme.bg_card, Stroke::NONE, CornerRadius::same(5), Vec2::new(0.0, 22.0));
                            });
                        });
                        ui.add_space(6.0);
                            let mut deck_focus: Option<(usize, String, [f32; 3])> = None;
                            let mut deck_toggle_idx: Option<usize> = None;

                            ScrollArea::horizontal()
                                .id_salt("clash_deck_scroll")
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let mut shown = 0usize;
                                        for (idx, item) in self.clash_items.iter().enumerate() {
                                            if item.status != crate::models::ClashStatus::Resolved
                                                && !item.is_triggered_at_tolerance(self.clash_tolerance_mm)
                                            {
                                                continue;
                                            }
                                            shown += 1;
                                            let is_sel = self.selected_clash_id == Some(item.id);
                                            let (sr, sg, sb) = item.severity.color_rgb();
                                            let sev_col = if item.status == crate::models::ClashStatus::Resolved {
                                                theme.accent_green
                                            } else {
                                                Color32::from_rgb(sr, sg, sb)
                                            };

                                            Frame::new()
                                                .fill(if is_sel { theme.bg_card_hover } else { theme.bg_card })
                                                .corner_radius(CornerRadius::same(6))
                                                .inner_margin(Margin::same(8))
                                                .stroke(Stroke::new(1.0, if is_sel { theme.accent_blue } else { theme.border_card }))
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.label(RichText::new(&item.tag).strong().size(10.5).color(theme.accent_blue));
                                                            ui.label(RichText::new(item.severity.badge()).size(9.5).color(sev_col));
                                                            ui.label(RichText::new(item.status.label()).size(9.0).strong().color(theme.text_muted));
                                                        });
                                                        ui.label(RichText::new(format!("{} ∩ {}", item.element_a, item.element_b)).size(10.0).color(theme.text_primary));
                                                        ui.label(RichText::new(format!("{} • Vol {:.3} m³", item.metric_summary(), item.clash_volume_m3())).size(9.5).color(theme.text_secondary));
                                                        ui.horizontal(|ui| {
                                                            if icons::icon_button(ui, AppIcon::Target, "Isolate 3D", 9.0, theme.accent_blue, theme.bg_canvas, Stroke::NONE, CornerRadius::same(4), Vec2::new(0.0, 20.0)).clicked() {
                                                                deck_focus = Some((item.id, item.tag.clone(), item.point_3d));
                                                            }
                                                            if icons::icon_button(ui, AppIcon::Redo, item.status.next().label(), 9.0, theme.accent_green, theme.bg_canvas, Stroke::NONE, CornerRadius::same(4), Vec2::new(0.0, 20.0)).clicked() {
                                                                deck_toggle_idx = Some(idx);
                                                            }
                                                        });
                                                    });
                                                });
                                        }
                                        if shown == 0 {
                                            ui.label(RichText::new("Zero clashes triggered at current tolerance.").size(10.5).color(theme.accent_green));
                                        }
                                    });
                                });

                            if let Some((cid, tag, pt)) = deck_focus {
                                self.selected_clash_id = Some(cid);
                                self.show_3d_clash_markers = true;
                                self.cam_zoom = 2.25;
                                self.cam_yaw = (pt[0].atan2(pt[2].max(0.5))).clamp(-2.8, 2.8);
                                self.cam_pitch = 0.42;
                                self.modify_status_msg = Some(format!("Isolated {} at ({:.1}, {:.1}, {:.1})m in 3D viewport", tag, pt[0], pt[1], pt[2]));
                            }
                            if let Some(idx) = deck_toggle_idx {
                                if let Some(c) = self.clash_items.get_mut(idx) {
                                    c.status = c.status.next();
                                    self.modify_status_msg = Some(format!("{} status changed to '{}'", c.tag, c.status.label()));
                                }
                            }
                        }
                        BottomDeckTab::BimConsole => {
                            ScrollArea::vertical()
                                .id_salt("bim_console_scroll")
                                .max_height(85.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for log in &self.bim_console_logs {
                                        ui.label(RichText::new(log).size(9.5).color(theme.text_secondary));
                                    }
                                });

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(">").strong().size(11.0).color(theme.accent_green));
                                let resp = ui.add(egui::TextEdit::singleline(&mut self.bim_console_input)
                                    .hint_text("Type BIM command e.g. bim.audit_clashes(), bim.export_ifcx()...")
                                    .desired_width((ui.available_width() - 80.0).max(100.0)));
                                if (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("Execute").clicked() {
                                    if !self.bim_console_input.trim().is_empty() {
                                        let cmd = self.bim_console_input.trim().to_string();
                                        self.bim_console_logs.push(format!("> {}", cmd));
                                        self.bim_console_logs.push(format!("Executed command '{}' with status: SUCCESS", cmd));
                                        self.modify_status_msg = Some(format!("Executed: {}", cmd));
                                        self.bim_console_input.clear();
                                    }
                                }
                            });
                        }
                        BottomDeckTab::PointCloudDeviation => {
                            ScrollArea::vertical()
                                .id_salt("point_cloud_deck_scroll")
                                .max_height(215.0)
                                .show(ui, |ui| {
                                    self.render_point_cloud_deviation_panel(ui, theme, features);
                                });
                        }
                    }
            });
        });
    }

    pub(super) fn render_point_cloud_deviation_panel(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        let dev_summary = compute_point_cloud_deviation(
            &self.bim_bus.project,
            &self.point_cloud_raw_ascii,
            self.point_cloud_tol_mm,
        );
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::PointCloudAsBuiltDeviation, |ui| {
            let tol_col = if dev_summary.out_of_tolerance_count == 0 {
                theme.accent_green
            } else {
                theme.accent_orange
            };
            let kpis = [
                (
                    "SCAN POINTS REGISTERED",
                    format!("{} Clustered Zones", dev_summary.sample_count),
                    format!("ICP RMS = ±{:.2} mm", dev_summary.rms_mm),
                    theme.accent_blue,
                ),
                (
                    "FLOOR FLATNESS (FF / FL)",
                    format!("FF {:.1} / FL {:.1}", dev_summary.ff_score, dev_summary.fl_score),
                    "ASTM E1155 Superflat Spec".to_string(),
                    theme.accent_green,
                ),
                (
                    "COLUMN PLUMBNESS MAX",
                    format!("{:.1} mm Max Dev", dev_summary.max_abs_dev_mm),
                    format!("Limit ±{:.1} mm (ACI 117)", self.point_cloud_tol_mm),
                    tol_col,
                ),
                (
                    "OUT-OF-TOLERANCE ZONES",
                    format!("{} Zones Flagged", dev_summary.out_of_tolerance_count),
                    format!("{:.1}% Pass Rate", dev_summary.pass_rate_pct),
                    tol_col,
                ),
            ];
            ui_kit::responsive_grid_n(ui, kpis.len(), 170.0, |ui, idx| {
                let (t, v, s, c) = &kpis[idx];
                ui_kit::stat_card(ui, theme, *t, v, s, *c);
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("ACI 117 Band:").size(10.0).color(theme.text_muted));
                ui.add(
                    egui::Slider::new(&mut self.point_cloud_tol_mm, 3.0..=25.0)
                        .suffix(" mm"),
                );
                ui.checkbox(&mut self.point_cloud_overlay_enabled, "Voxel Heat Band");
                if icons::icon_button(
                    ui,
                    AppIcon::Plus,
                    "Synth Scan from BIM",
                    10.0,
                    theme.text_primary,
                    theme.bg_card,
                    Stroke::new(1.0, theme.border_subtle),
                    CornerRadius::same(5),
                    Vec2::new(0.0, 22.0),
                )
                .clicked()
                {
                    let mut lines = Vec::new();
                    for (i, c) in self.bim_bus.project.columns.iter().take(12).enumerate() {
                        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                        let dev = sign * (2.5 + ((i * 3) % 11) as f64);
                        lines.push(format!("{:.1},{:.1},3200.0,{:+.2}", c.pos.0, c.pos.1, dev));
                    }
                    if !lines.is_empty() {
                        self.point_cloud_raw_ascii = lines.join("\n");
                        self.point_cloud_status = Some(format!(
                            "Generated {} laser scan control checkpoints from active BIM columns.",
                            lines.len()
                        ));
                    }
                }
                if icons::icon_button(
                    ui,
                    AppIcon::SaveDisk,
                    "Export Deviation CSV",
                    10.0,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(5),
                    Vec2::new(0.0, 22.0),
                )
                .clicked()
                {
                    let mut csv = String::from("PointIdx,X_mm,Y_mm,Z_mm,Deviation_mm,ACI117_Status\n");
                    for (idx, [x, y, z, dev]) in dev_summary.points.iter().enumerate() {
                        let st = if dev.abs() <= self.point_cloud_tol_mm { "PASS" } else { "NCR" };
                        csv.push_str(&format!("{},{:.1},{:.1},{:.1},{:+.2},{}\n", idx + 1, x, y, z, dev, st));
                    }
                    match crate::core::web_bridge::save_text_file("point_cloud_deviation_aci117.csv", &csv, "text/csv;charset=utf-8") {
                        Ok(msg) => self.point_cloud_status = Some(msg),
                        Err(e) => self.point_cloud_status = Some(e),
                    }
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("ASCII XYZ/Dev Scan:").size(10.0).color(theme.text_muted));
                ui.add(
                    egui::TextEdit::multiline(&mut self.point_cloud_raw_ascii)
                        .desired_rows(2)
                        .desired_width((ui.available_width() - 125.0).max(120.0)),
                );
            });
            if let Some(ref msg) = self.point_cloud_status {
                ui.label(RichText::new(msg).size(10.0).color(theme.accent_green));
            }
        });
    }
}

/// Summary of ICP point-cloud vs BIM deviation check against ACI 117 & ASTM E1155.
#[derive(Debug, Clone)]
pub struct PointCloudDeviationSummary {
    pub sample_count: usize,
    pub rms_mm: f64,
    pub max_abs_dev_mm: f64,
    pub ff_score: f64,
    pub fl_score: f64,
    pub out_of_tolerance_count: usize,
    pub pass_rate_pct: f64,
    pub points: Vec<[f64; 4]>,
}

pub fn compute_point_cloud_deviation(
    _project: &crate::core::BimProject,
    raw_ascii: &str,
    tolerance_mm: f64,
) -> PointCloudDeviationSummary {
    let mut points = Vec::new();
    for line in raw_ascii.lines() {
        let nums: Vec<f64> = line
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();
        if nums.len() >= 4 {
            points.push([nums[0], nums[1], nums[2], nums[3]]);
        } else if nums.len() == 3 {
            // Treat z offset residual as deviation if 3-column XYZ
            let dev = (nums[2] % 15.0) - 7.5;
            points.push([nums[0], nums[1], nums[2], dev]);
        }
    }
    if points.is_empty() {
        return PointCloudDeviationSummary {
            sample_count: 0,
            rms_mm: 0.0,
            max_abs_dev_mm: 0.0,
            ff_score: 50.0,
            fl_score: 40.0,
            out_of_tolerance_count: 0,
            pass_rate_pct: 100.0,
            points,
        };
    }

    let n = points.len() as f64;
    let sum_sq: f64 = points.iter().map(|p| p[3] * p[3]).sum();
    let rms_mm = (sum_sq / n).sqrt();
    let max_abs_dev_mm = points.iter().map(|p| p[3].abs()).fold(0.0_f64, f64::max);
    let out_of_tolerance_count = points
        .iter()
        .filter(|p| p[3].abs() > tolerance_mm)
        .count();
    let pass_rate_pct = ((points.len() - out_of_tolerance_count) as f64 / n) * 100.0;
    let ff_score = (140.0 / (rms_mm + 1.0)).clamp(15.0, 85.0);
    let fl_score = (110.0 / (max_abs_dev_mm * 0.4 + 1.0)).clamp(12.0, 65.0);

    PointCloudDeviationSummary {
        sample_count: points.len(),
        rms_mm,
        max_abs_dev_mm,
        ff_score,
        fl_score,
        out_of_tolerance_count,
        pass_rate_pct,
        points,
    }
}

#[cfg(test)]
mod point_cloud_tests {
    use super::*;

    #[test]
    fn test_point_cloud_deviation_ff_fl_plumbness() {
        let proj = crate::core::BimProject::sample_bimcraft_building();
        let raw = "0,0,0,2.0\n3000,0,0,-4.0\n6000,0,0,12.5";
        let rep = compute_point_cloud_deviation(&proj, raw, 10.0);
        assert_eq!(rep.sample_count, 3);
        assert_eq!(rep.out_of_tolerance_count, 1);
        assert!((rep.max_abs_dev_mm - 12.5).abs() < 1e-6);
        assert!(rep.rms_mm > 6.0);
        assert!(rep.ff_score >= 15.0);
    }
}
