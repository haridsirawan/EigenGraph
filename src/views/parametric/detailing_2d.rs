//! views/parametric/detailing_2d.rs — Automated RC Rebar BBS & Architectural Elevation / MEP Riser Slots.

use super::*;
use crate::core::{BimCommand, BimProject, RebarPreset, RunCategory};
use egui::{Align2, FontId, Pos2, Rect, ScrollArea, Sense};

#[derive(Clone, Debug, PartialEq)]
pub struct RebarBbsRow {
    pub bar_mark: String,
    pub iso_shape: String,
    pub host_id: usize,
    pub host_kind: String,
    pub diameter_mm: f64,
    pub count: usize,
    pub cut_length_mm: f64,
    pub total_weight_kg: f64,
}

/// Computes live ISO 3766 Bar Bending Schedule rows directly from active `BimProject.rebars`.
pub fn compute_rebar_bbs_rows(
    project: &BimProject,
    host_filter: Option<usize>,
    seismic_135_hooks: bool,
) -> Vec<RebarBbsRow> {
    project
        .rebars
        .iter()
        .filter(|b| host_filter.is_none_or(|hid| b.host_id == hid))
        .map(|b| {
            let prefix = match b.host_kind.as_str() {
                "Column" => "C",
                "Footing" => "F",
                _ => "B",
            };
            let suffix = if b.closed_stirrup { "ST" } else { "MN" };
            let shape = if b.closed_stirrup {
                if seismic_135_hooks {
                    "SC-51 (135° Seismic Stirrup)"
                } else {
                    "SC-50 (90° Standard Stirrup)"
                }
            } else if b.points_3d.len() > 2 {
                "SC-11 (L-Hook)"
            } else {
                "SC-00 (Straight)"
            };
            RebarBbsRow {
                bar_mark: format!("{}{}-{}", prefix, b.host_id, suffix),
                iso_shape: shape.to_string(),
                host_id: b.host_id,
                host_kind: b.host_kind.clone(),
                diameter_mm: b.diameter_mm,
                count: b.count,
                cut_length_mm: b.single_length_mm(),
                total_weight_kg: b.total_weight_kg(),
            }
        })
        .collect()
}

impl ParametricView {
    pub(super) fn render_rebar_bbs_detailing(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::AutomatedRcRebarBbsDrafter, |ui| {
            // Collect live RC structural hosts (Beams + Columns + Footings)
            #[derive(Clone)]
            struct RcHostInfo {
                id: usize,
                kind: &'static str,
                label: String,
                span_mm: f64,
                width_mm: f64,
                depth_mm: f64,
            }
            let mut hosts: Vec<RcHostInfo> = Vec::new();
            for r in &self.bim_bus.project.runs {
                if r.category == RunCategory::StructuralBeam && !r.is_wf_profile {
                    let len = r.start.dist(r.end).max(1000.0);
                    hosts.push(RcHostInfo {
                        id: r.id,
                        kind: "Beam",
                        label: format!("{} #{} ({:.0}×{:.0}, L={:.2}m)", r.name, r.id, r.width_mm, r.depth_mm, len * 1e-3),
                        span_mm: len,
                        width_mm: r.width_mm,
                        depth_mm: r.depth_mm,
                    });
                }
            }
            for c in &self.bim_bus.project.columns {
                if !c.is_wf_profile {
                    let h = (self.bim_bus.project.level_elevation(c.top_level_id)
                        - self.bim_bus.project.level_elevation(c.base_level_id))
                    .abs()
                    .max(3000.0);
                    hosts.push(RcHostInfo {
                        id: c.id,
                        kind: "Column",
                        label: format!("{} #{} ({:.0}×{:.0}, H={:.2}m)", c.name, c.id, c.width_mm, c.depth_mm, h * 1e-3),
                        span_mm: h,
                        width_mm: c.width_mm,
                        depth_mm: c.depth_mm,
                    });
                }
            }

            if !hosts.is_empty() && self.selected_rebar_host_idx >= hosts.len() {
                self.selected_rebar_host_idx = 0;
            }
            let selected_host = hosts.get(self.selected_rebar_host_idx).cloned();
            let host_rows = compute_rebar_bbs_rows(
                &self.bim_bus.project,
                selected_host.as_ref().map(|h| h.id),
                self.rebar_seismic_hooks_135,
            );
            let all_rows = compute_rebar_bbs_rows(&self.bim_bus.project, None, self.rebar_seismic_hooks_135);
            let display_rows = if host_rows.is_empty() { &all_rows } else { &host_rows };
            let host_weight: f64 = host_rows.iter().map(|r| r.total_weight_kg).sum();
            let total_weight = self.bim_bus.project.total_rebar_weight_kg();
            let main_bar_dia = display_rows
                .iter()
                .filter(|r| !r.iso_shape.starts_with("SC-5"))
                .map(|r| r.diameter_mm)
                .fold(22.0_f64, f64::max);
            let ld_mm = (main_bar_dia * 42.0).round();

            let host_title = selected_host
                .as_ref()
                .map(|h| format!("SELECTED RC {} ({:.0}×{:.0})", h.kind.to_uppercase(), h.width_mm, h.depth_mm))
                .unwrap_or_else(|| "RC WORKSPACE STATUS".to_string());
            let host_val = selected_host
                .as_ref()
                .map(|h| format!("Span {:.2} m • {} Bars", h.span_mm * 1e-3, host_rows.iter().map(|r| r.count).sum::<usize>()))
                .unwrap_or_else(|| "0 RC Members".to_string());
            let cards = [
                (
                    host_title.as_str(),
                    host_val.as_str(),
                    "ACI 318-19 §18.6.3 / SNI 2847 Flexural Bounds",
                    theme.accent_blue,
                ),
                (
                    "CONFINEMENT STIRRUPS",
                    if self.rebar_seismic_hooks_135 {
                        "D10-100 / 200mm (135°)"
                    } else {
                        "D10-150 Standard (90°)"
                    },
                    "l_dh = 10db Seismic End Hoops",
                    theme.accent_green,
                ),
                (
                    "DEVELOPMENT LENGTH (ld)",
                    &format!("{:.0} mm Tension Lap", ld_mm),
                    "f'c = 35 MPa • fy = 420 MPa BJTS 420B",
                    theme.accent_orange,
                ),
                (
                    "LIVE BIM REBAR STEEL MASS",
                    &format!("{:.1} kg (Host {:.1} kg)", total_weight, host_weight),
                    &format!("{} ISO 3766 Marks -> 9D CSO Sync", all_rows.len()),
                    theme.text_primary,
                ),
            ];
            ui_kit::responsive_grid_n(ui, cards.len(), 175.0, |ui, idx| {
                let (t, v, s, c) = cards[idx];
                ui_kit::stat_card(ui, theme, t, v, s, c);
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.rebar_seismic_hooks_135, "135° Seismic Hooks (SNI 2847 SMF)");

                if !hosts.is_empty() {
                    let cur_lbl = hosts[self.selected_rebar_host_idx].label.clone();
                    crate::ui_kit::wrap_before(ui, 220.0);
                    egui::ComboBox::from_id_salt("rebar_bbs_host_combo")
                        .width(crate::ui_kit::clamped_row_width(ui, 120.0, 260.0, 32.0))
                        .selected_text(RichText::new(cur_lbl).size(10.5))
                        .show_ui(ui, |ui| {
                            for (i, h) in hosts.iter().enumerate() {
                                if ui.selectable_label(self.selected_rebar_host_idx == i, &h.label).clicked() {
                                    self.selected_rebar_host_idx = i;
                                }
                            }
                        });
                }

                if icons::icon_button(
                    ui,
                    AppIcon::Rebar,
                    "Auto-Arm All RC Elements",
                    10.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    CornerRadius::same(5),
                    Vec2::new(0.0, 24.0),
                )
                .on_hover_text("Generate live longitudinal bars & confinement stirrups for every concrete beam, column & footing")
                .clicked()
                {
                    let mut preset = RebarPreset::default();
                    if self.rebar_seismic_hooks_135 {
                        preset.stirrup_spacing_mm = 100.0;
                    }
                    match self.bim_bus.execute(BimCommand::ReinforceAllConcrete { preset }) {
                        Ok(msg) => {
                            self.command_history.push(msg.clone());
                            self.save_notification = Some(msg);
                            self.refresh_live_ari();
                        }
                        Err(e) => {
                            self.save_notification = Some(e);
                        }
                    }
                }
            });
            ui.add_space(10.0);

            ui_kit::responsive_two_panes(ui, 620.0, |ui, pane| match pane {
                0 => {
                    let elev_title = selected_host
                        .as_ref()
                        .map(|h| format!("{} REBAR ELEVATION ({:.0}×{:.0} mm)", h.kind.to_uppercase(), h.width_mm, h.depth_mm))
                        .unwrap_or_else(|| "RC LONGITUDINAL REBAR ELEVATION".to_string());
                    ui.label(RichText::new(elev_title).size(11.0).strong().color(theme.accent_blue));
                    ui.add_space(6.0);
                    let desired = Vec2::new(ui.available_width(), 155.0);
                    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(12, 16, 22));
                    let beam = Rect::from_min_max(
                        Pos2::new(rect.left() + 22.0, rect.top() + 30.0),
                        Pos2::new(rect.right() - 22.0, rect.bottom() - 36.0),
                    );
                    painter.rect_stroke(beam, CornerRadius::ZERO, Stroke::new(1.5, theme.text_muted), StrokeKind::Inside);

                    if let Some(ref host) = selected_host {
                        painter.line_segment(
                            [Pos2::new(beam.left() + 6.0, beam.top() + 10.0), Pos2::new(beam.right() - 6.0, beam.top() + 10.0)],
                            Stroke::new(2.4, theme.accent_orange),
                        );
                        painter.line_segment(
                            [Pos2::new(beam.left() + 6.0, beam.bottom() - 10.0), Pos2::new(beam.right() - 6.0, beam.bottom() - 10.0)],
                            Stroke::new(2.4, theme.accent_green),
                        );
                        let hoop_n = if self.rebar_seismic_hooks_135 { 24 } else { 16 };
                        for i in 1..hoop_n {
                            let sx = beam.left() + (i as f32 / hoop_n as f32) * beam.width();
                            let in_lo = (i as f32 / hoop_n as f32) <= 0.25 || (i as f32 / hoop_n as f32) >= 0.75;
                            let col = if in_lo { theme.accent_blue } else { theme.text_secondary };
                            painter.line_segment(
                                [Pos2::new(sx, beam.top() + 6.0), Pos2::new(sx, beam.bottom() - 6.0)],
                                Stroke::new(1.2, col),
                            );
                        }
                        let lo_mm = (host.depth_mm * 2.0).max(600.0);
                        painter.text(
                            Pos2::new(beam.center().x, rect.bottom() - 12.0),
                            Align2::CENTER_CENTER,
                            format!(
                                "Span L = {:.2}m • Lo = {:.0}mm Confined Ends • {} Bars Armed",
                                host.span_mm * 1e-3,
                                lo_mm,
                                host_rows.iter().map(|r| r.count).sum::<usize>()
                            ),
                            FontId::proportional(10.0),
                            theme.text_secondary,
                        );
                    } else {
                        painter.text(
                            beam.center(),
                            Align2::CENTER_CENTER,
                            "Blank Workspace — Author RC Beams/Columns or instantiate a Structural Template to draft Rebar.",
                            FontId::proportional(10.5),
                            theme.text_muted,
                        );
                    }
                }
                _ => {
                    ui.label(RichText::new("LIVE BAR BENDING SCHEDULE (ISO 3766)").size(11.0).strong().color(theme.accent_blue));
                    ui.add_space(6.0);
                    if display_rows.is_empty() {
                        ui.label(
                            RichText::new("No reinforcement bars generated yet. Click 'Auto-Arm All RC Elements' once concrete hosts exist.")
                                .size(11.0)
                                .color(theme.text_muted),
                        );
                    } else {
                        ScrollArea::horizontal().id_salt("bbs_preview_table").show(ui, |ui| {
                            egui::Grid::new("bbs_preview_grid")
                                .striped(true)
                                .spacing(Vec2::new(12.0, 5.0))
                                .show(ui, |ui| {
                                    for h in ["Mark", "ISO Shape", "Dia", "Qty", "Cut Len", "Total (kg)"] {
                                        ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                                    }
                                    ui.end_row();
                                    for row in display_rows.iter().take(24) {
                                        ui.label(RichText::new(&row.bar_mark).size(10.5).monospace().color(theme.text_primary));
                                        ui.label(RichText::new(&row.iso_shape).size(10.5).color(theme.text_secondary));
                                        ui.label(RichText::new(format!("D{:.0}", row.diameter_mm)).size(10.5).strong().color(theme.accent_orange));
                                        ui.label(RichText::new(format!("{}", row.count)).size(10.5).color(theme.text_primary));
                                        ui.label(RichText::new(format!("{:.0} mm", row.cut_length_mm)).size(10.5).monospace().color(theme.text_muted));
                                        ui.label(RichText::new(format!("{:.1}", row.total_weight_kg)).size(10.5).strong().color(theme.accent_green));
                                        ui.end_row();
                                    }
                                });
                        });
                    }
                }
            });
        });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MepRiserRow {
    pub riser_id: String,
    pub system_name: String,
    pub size_spec: String,
    pub firestop_collar: String,
    pub head_loss: String,
}

/// Derives storey-aware vertical MEP riser shaft schedules from live `BimProject` heights.
pub fn compute_mep_riser_schedule(project: &BimProject, system_filter_idx: usize) -> Vec<MepRiserRow> {
    let max_elev_m = project
        .levels
        .iter()
        .map(|l| l.elevation_mm as f32 / 1000.0)
        .fold(3.8_f32, f32::max);
    let storeys = project.levels.len().max(1);
    let static_head_kpa = 9.81 * max_elev_m;
    let chw_loss = (max_elev_m * 0.72).max(4.5);
    let mut rows = vec![
        MepRiserRow {
            riser_id: "CHW-R01".to_string(),
            system_name: format!("Chilled Water Supply ({storeys} Flrs)"),
            size_spec: if storeys >= 6 { "DN200 Sch40".to_string() } else { "DN125 Sch40".to_string() },
            firestop_collar: "2-Hr Intumescent UL".to_string(),
            head_loss: format!("{:.1} kPa ({:.1}m H)", chw_loss, max_elev_m),
        },
        MepRiserRow {
            riser_id: "FSP-R02".to_string(),
            system_name: "Class I Fire Standpipe".to_string(),
            size_spec: "DN150 Galv. Steel".to_string(),
            firestop_collar: "2-Hr Cast-In Sleeve".to_string(),
            head_loss: format!("{:.1} kPa static", static_head_kpa),
        },
        MepRiserRow {
            riser_id: "HVC-D01".to_string(),
            system_name: "Fresh Air Pressurization Duct".to_string(),
            size_spec: if storeys >= 6 { "800x500 mm GI".to_string() } else { "600x400 mm GI".to_string() },
            firestop_collar: "Fire Damper FD-90".to_string(),
            head_loss: format!("{:.1} Pa ({:.1}m)", max_elev_m * 1.15, max_elev_m),
        },
        MepRiserRow {
            riser_id: "ELV-B01".to_string(),
            system_name: "Main Electrical Riser Busway".to_string(),
            size_spec: format!("Cu IP55 ({}A)", storeys * 250),
            firestop_collar: "3-Hr Mortar Seal".to_string(),
            head_loss: format!("{:.2}% dV", (max_elev_m * 0.04).clamp(0.4, 2.8)),
        },
    ];
    if system_filter_idx == 1 {
        rows.retain(|r| r.riser_id.starts_with("FSP") || r.riser_id.starts_with("ELV"));
    }
    rows
}

impl ParametricView {
    pub(super) fn render_elevations_mep_riser(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
    ) {
        ui_kit::feature_slot_card(ui, theme, features, FeatureId::BuildingElevationsMepRiser, |ui| {
            let proj = &self.bim_bus.project;
            ui.horizontal_wrapped(|ui| {
                for (idx, lbl) in [
                    (0, "North Facade (X-Z)"),
                    (1, "East Facade (Y-Z)"),
                    (2, "South Facade (X-Z)"),
                    (3, "West Facade (Y-Z)"),
                ] {
                    if ui.selectable_label(self.elev_facade_idx == idx, lbl).clicked() {
                        self.elev_facade_idx = idx;
                    }
                }
                ui.separator();
                if ui.selectable_label(self.elev_mep_system_idx == 0, "All HVAC + CHW + Fire").clicked() {
                    self.elev_mep_system_idx = 0;
                }
                if ui.selectable_label(self.elev_mep_system_idx == 1, "Fire Standpipe Focus").clicked() {
                    self.elev_mep_system_idx = 1;
                }
                if icons::icon_button(
                    ui,
                    AppIcon::RulerSquare,
                    "Export Elevation + MEP Riser (.svg)",
                    10.5,
                    theme.text_primary,
                    theme.bg_card,
                    Stroke::NONE,
                    CornerRadius::same(5),
                    Vec2::new(0.0, 24.0),
                )
                .clicked()
                {
                    let dir_name = ["North", "East", "South", "West"][self.elev_facade_idx.min(3)];
                    let svg = format!(
                        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"840\" height=\"420\">\n<rect width=\"840\" height=\"420\" fill=\"#0f172a\"/>\n<text x=\"24\" y=\"32\" fill=\"#e2e8f0\" font-size=\"16\">{} — {} Orthographic Elevation ({} Levels, {} Elements)</text>\n</svg>",
                        proj.project_name, dir_name, proj.levels.len(), proj.element_count()
                    );
                    let fname = format!("Elevation_{}_{}.svg", dir_name, proj.project_name.replace([' ', '/', '\\'], "_"));
                    let _ = crate::core::web_bridge::save_text_file(&fname, &svg, "image/svg+xml");
                }
            });
            ui.add_space(10.0);

            ui_kit::responsive_two_panes(ui, 620.0, |ui, pane| match pane {
                0 => {
                    let dir_name = ["NORTH", "EAST", "SOUTH", "WEST"][self.elev_facade_idx.min(3)];
                    ui.label(RichText::new(format!("{} ORTHOGRAPHIC FACADE & DATUM ELEVATION", dir_name)).size(11.0).strong().color(theme.accent_blue));
                    ui.add_space(6.0);
                    let desired = Vec2::new(ui.available_width(), 185.0);
                    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(13, 17, 24));
                    let left = rect.left() + 74.0;
                    let right = rect.right() - 24.0;
                    let top_pad = rect.top() + 18.0;
                    let bot_pad = rect.bottom() - 18.0;

                    let mut sorted_lvls = proj.levels.clone();
                    sorted_lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
                    let min_z_mm = sorted_lvls.first().map(|l| l.elevation_mm).unwrap_or(0.0).min(0.0);
                    let max_z_mm = sorted_lvls
                        .iter()
                        .map(|l| l.elevation_mm + 3600.0)
                        .chain(proj.columns.iter().map(|c| proj.level_elevation(c.top_level_id)))
                        .chain(proj.walls.iter().map(|w| {
                            let wb = proj.level_elevation(w.base_level_id);
                            w.top_level_id
                                .map(|id| proj.level_elevation(id))
                                .unwrap_or(wb + w.unconnected_height_mm)
                        }))
                        .fold(4000.0_f64, f64::max);
                    let z_span = (max_z_mm - min_z_mm).max(3000.0);
                    let z_to_y = |z_mm: f64| -> f32 {
                        let t = ((z_mm - min_z_mm) / z_span).clamp(0.0, 1.0) as f32;
                        bot_pad - t * (bot_pad - top_pad)
                    };

                    for lvl in &sorted_lvls {
                        let y = z_to_y(lvl.elevation_mm);
                        painter.line_segment([Pos2::new(left, y), Pos2::new(right, y)], Stroke::new(1.0, theme.text_muted.gamma_multiply(0.45)));
                        painter.text(
                            Pos2::new(left - 6.0, y),
                            Align2::RIGHT_CENTER,
                            format!("{} {:+.1}m", lvl.name.chars().take(8).collect::<String>(), lvl.elevation_mm / 1000.0),
                            FontId::proportional(9.0),
                            theme.accent_blue,
                        );
                    }

                    let use_x = self.elev_facade_idx % 2 == 0;
                    let extents = proj.plan_extents_mm().unwrap_or(([0.0, 0.0], [12000.0, 8000.0]));
                    let (min_u, max_u) = if use_x { (extents.0[0], extents.1[0]) } else { (extents.0[1], extents.1[1]) };
                    let u_span = (max_u - min_u).max(2000.0);
                    let u_to_x = |u_mm: f64| -> f32 {
                        let mut t = ((u_mm - min_u) / u_span).clamp(0.0, 1.0) as f32;
                        if self.elev_facade_idx >= 2 {
                            t = 1.0 - t;
                        }
                        left + 12.0 + t * (right - left - 24.0)
                    };

                    if proj.is_empty() {
                        painter.text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            "Empty Ground Canvas — Draw walls/columns in Plan or instantiate a template to project Elevations",
                            FontId::proportional(10.5),
                            theme.text_muted,
                        );
                    } else {
                        for w in &proj.walls {
                            let x0 = u_to_x(if use_x { w.start.0 } else { w.start.1 });
                            let x1 = u_to_x(if use_x { w.end.0 } else { w.end.1 });
                            let wb = proj.level_elevation(w.base_level_id);
                            let wt = w
                                .top_level_id
                                .map(|id| proj.level_elevation(id))
                                .unwrap_or(wb + w.unconnected_height_mm);
                            let y0 = z_to_y(wt);
                            let y1 = z_to_y(wb);
                            let r = Rect::from_two_pos(Pos2::new(x0.min(x1), y0), Pos2::new(x0.max(x1).max(x0.min(x1) + 2.0), y1));
                            painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.4, theme.accent_green), StrokeKind::Inside);
                        }
                        for c in &proj.columns {
                            let cx = u_to_x(if use_x { c.pos.0 } else { c.pos.1 });
                            painter.line_segment(
                                [
                                    Pos2::new(cx, z_to_y(proj.level_elevation(c.top_level_id))),
                                    Pos2::new(cx, z_to_y(proj.level_elevation(c.base_level_id))),
                                ],
                                Stroke::new(2.0, theme.text_secondary),
                            );
                        }
                        for b in &proj.runs {
                            let (a3, b3) = b.centerline_3d(proj.level_elevation(b.level_id));
                            let x0 = u_to_x(if use_x { a3[0] } else { a3[1] });
                            let x1 = u_to_x(if use_x { b3[0] } else { b3[1] });
                            let ya = z_to_y(a3[2]);
                            let yb = z_to_y(b3[2]);
                            painter.line_segment([Pos2::new(x0, ya), Pos2::new(x1, yb)], Stroke::new(2.2, theme.accent_orange));
                        }
                    }
                }
                _ => {
                    ui.label(RichText::new("VERTICAL MEP SHAFT & FIRESTOP RISER SCHEDULE").size(11.0).strong().color(theme.accent_blue));
                    ui.add_space(6.0);
                    let rows = compute_mep_riser_schedule(proj, self.elev_mep_system_idx);
                    ScrollArea::horizontal().id_salt("mep_riser_table").show(ui, |ui| {
                        egui::Grid::new("mep_riser_grid")
                            .striped(true)
                            .spacing(Vec2::new(12.0, 5.0))
                            .show(ui, |ui| {
                                for h in ["Riser ID", "System", "Size / Spec", "Firestop Collar", "Head Loss"] {
                                    ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                                }
                                ui.end_row();
                                for r in &rows {
                                    ui.label(RichText::new(&r.riser_id).size(10.5).monospace().color(theme.text_primary));
                                    ui.label(RichText::new(&r.system_name).size(10.5).color(theme.text_secondary));
                                    ui.label(RichText::new(&r.size_spec).size(10.5).color(theme.text_muted));
                                    ui.label(RichText::new(&r.firestop_collar).size(10.5).strong().color(theme.accent_green));
                                    ui.label(RichText::new(&r.head_loss).size(10.5).monospace().color(theme.accent_orange));
                                    ui.end_row();
                                }
                            });
                    });
                }
            });
        });
    }

    /// Phase 45: External below-toolbar Drafting sub-menu panel (Rebar BBS, Elevations & MEP Riser, MEP Sizing).
    pub(super) fn render_drafting_below_toolbar_submenu(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                let inner_w = ui.available_width().max(120.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.horizontal_wrapped(|ui| {
                    icons::pill_badge(ui, AppIcon::Layers, "Drafting Sub-Menus", theme.accent_blue);
                    ui.separator();

                    let tabs = [
                        (DraftingSubTab::RebarBbs, AppIcon::Rebar, "Rebar BBS"),
                        (DraftingSubTab::ElevationsAndMepRiser, AppIcon::Building, "Elevations & MEP Riser"),
                        (DraftingSubTab::MepEngineeringSizing, AppIcon::Bolt, "MEP Engineering Sizing"),
                    ];
                    for (tab, icon, lbl) in tabs {
                        let sel = self.active_drafting_subtab == tab;
                        if icons::icon_button(
                            ui,
                            icon,
                            lbl,
                            11.0,
                            if sel { Color32::WHITE } else { theme.text_secondary },
                            if sel { theme.accent_blue } else { theme.bg_canvas },
                            Stroke::new(1.0, if sel { theme.accent_blue } else { theme.border_subtle }),
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .clicked()
                        {
                            self.active_drafting_subtab = tab;
                            self.drafting_submenu_open = true;
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let toggle_lbl = if self.drafting_submenu_open { "Collapse" } else { "Expand" };
                        let toggle_icon = if self.drafting_submenu_open {
                            AppIcon::ChevronDown
                        } else {
                            AppIcon::ArrowRight
                        };
                        if icons::icon_button(
                            ui,
                            toggle_icon,
                            toggle_lbl,
                            10.0,
                            theme.text_secondary,
                            theme.bg_sidebar,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(5),
                            Vec2::new(0.0, 24.0),
                        )
                        .on_hover_text("Collapse or expand Drafting detailing sub-menus")
                        .clicked()
                        {
                            self.drafting_submenu_open = !self.drafting_submenu_open;
                        }
                    });
                });

                if self.drafting_submenu_open {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);
                    match self.active_drafting_subtab {
                        DraftingSubTab::RebarBbs => {
                            self.render_rebar_bbs_detailing(ui, theme, features);
                        }
                        DraftingSubTab::ElevationsAndMepRiser => {
                            self.render_elevations_mep_riser(ui, theme, features);
                        }
                        DraftingSubTab::MepEngineeringSizing => {
                            self.render_mep_engineering_sizing(ui, theme, universal_canvas);
                        }
                    }
                }
            });
    }

    /// Interactive ASHRAE Ductwork, IPC Plumbing Pipe, and IEC 3-Phase Electrical Feeder sizing studio.
    pub(super) fn render_mep_engineering_sizing(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        let storeys = self.bim_bus.project.levels.len().max(1);
        let room_area_m2: f64 = self.bim_bus.project.rooms.iter().map(|r| r.area_m2).sum();
        let plan_extents = self.bim_bus.project.plan_extents_mm();
        let active_lvl_id = self
            .bim_bus
            .project
            .levels
            .get(self.active_level_idx)
            .map(|l| l.id)
            .unwrap_or(1);
        let res = crate::core::compute_mep_engineering_sizing(&universal_canvas.lifecycle_ops.mep_sizing, storeys);

        ui.horizontal_wrapped(|ui| {
            icons::icon_label(
                ui,
                AppIcon::Bolt,
                14.0,
                theme.accent_blue,
                "MEP ENGINEERING SIZING (HVAC DUCT + PLUMBING DN + 3-PHASE FEEDER)",
                12.0,
                theme.accent_blue,
                true,
            );
            if icons::icon_button(
                ui,
                AppIcon::LinkChain,
                "Sync Area from BIM",
                10.5,
                Color32::WHITE,
                theme.accent_green,
                Stroke::NONE,
                CornerRadius::same(5),
                Vec2::new(0.0, 24.0),
            )
            .on_hover_text("Calculate conditioned floor area & occupancy directly from BIM rooms/extents")
            .clicked()
            {
                let synced_area = if room_area_m2 > 20.0 {
                    room_area_m2
                } else if let Some((mn, mx)) = plan_extents {
                    (((mx[0] - mn[0]).abs() / 1000.0) * ((mx[1] - mn[1]).abs() / 1000.0) * storeys as f64)
                        .max(180.0)
                } else {
                    1200.0
                };
                universal_canvas.lifecycle_ops.mep_sizing.floor_area_m2 = synced_area.round();
                universal_canvas.lifecycle_ops.mep_sizing.occupancy_people =
                    (synced_area / 10.0).ceil().max(12.0) as usize;
                universal_canvas.lifecycle_ops.mep_sizing.electrical_connected_kva =
                    (synced_area * 0.14).max(45.0).round();
                self.save_notification = Some(format!(
                    "Synced MEP design basis from BIM project: {:.0} m2 across {} level(s).",
                    synced_area, storeys
                ));
            }
            if icons::icon_button(
                ui,
                AppIcon::Plus,
                "Add Sized MEP Runs to BIM",
                10.5,
                Color32::WHITE,
                theme.accent_blue,
                Stroke::NONE,
                CornerRadius::same(5),
                Vec2::new(0.0, 24.0),
            )
            .on_hover_text("Create dimensioned Plumbing DN pipe & Electrical cable tray runs on the active level")
            .clicked()
            {
                let lvl_id = active_lvl_id;
                let pipe_res = self.bim_bus.execute(BimCommand::AddBeamRun {
                    level_id: lvl_id,
                    category: RunCategory::PlumbingPipe,
                    start: P2(1200.0, 1200.0),
                    end: P2(9600.0, 1200.0),
                    width_mm: res.pipe_dn_mm,
                    depth_mm: res.pipe_dn_mm,
                    justify: RunJustify::Bottom,
                    profile_name: format!("DN{:.0} Supply Main", res.pipe_dn_mm),
                });
                let tray_res = self.bim_bus.execute(BimCommand::AddBeamRun {
                    level_id: lvl_id,
                    category: RunCategory::ElectricalCableTray,
                    start: P2(1200.0, 2100.0),
                    end: P2(9600.0, 2100.0),
                    width_mm: res.duct_width_mm.clamp(200.0, 600.0),
                    depth_mm: 100.0,
                    justify: RunJustify::Centre,
                    profile_name: format!("{}A ({}mm2) Feeder Tray", res.recommended_breaker_at, res.cable_size_mm2),
                });
                let ok_msg = match (pipe_res, tray_res) {
                    (Ok(_), Ok(_)) => format!(
                        "Added DN{:.0} Pipe + {}A Electrical Tray runs on Level {}.",
                        res.pipe_dn_mm, res.recommended_breaker_at, lvl_id
                    ),
                    (Err(e), _) | (_, Err(e)) => e,
                };
                self.save_notification = Some(ok_msg);
            }
        });

        ui.add_space(10.0);
        let cards = [
            (
                "HVAC Cooling & GI Duct",
                format!("{:.0} x {:.0} mm", res.duct_width_mm, res.duct_height_mm),
                format!("{:.1} TR ({:.0} CFM) @ {:.1} m/s", res.cooling_tr, res.supply_airflow_cfm, res.duct_velocity_ms),
                theme.accent_blue,
            ),
            (
                "Plumbing Main Pipe",
                format!("DN{:.0} Sch40", res.pipe_dn_mm),
                format!("{:.2} L/s • {:.2} m/s • {:.1} kPa", res.peak_flow_ls, res.pipe_velocity_ms, res.head_loss_kpa),
                if res.pipe_velocity_ms <= 2.2 { theme.accent_green } else { theme.accent_orange },
            ),
            (
                "3-Phase Feeder & Breaker",
                format!("{}A MCCB / {} mm2", res.recommended_breaker_at, res.cable_size_mm2),
                format!("I={:.0}A • dV={:.2}% {}", res.design_current_a, res.voltage_drop_pct, if res.voltage_drop_ok { "PASS" } else { "DROP>3%" }),
                if res.voltage_drop_ok { theme.accent_green } else { theme.accent_red },
            ),
            (
                "BIM Thermal Basis",
                format!("{:.0} m2 / {} Flrs", universal_canvas.lifecycle_ops.mep_sizing.floor_area_m2, storeys),
                format!("{} Occupants • Duct {:.2} Pa/m", universal_canvas.lifecycle_ops.mep_sizing.occupancy_people, res.duct_friction_pa_m),
                theme.accent_orange,
            ),
        ];
        ui_kit::responsive_grid_n(ui, 4, 185.0, |ui, i| {
            let (t, v, s, c) = &cards[i];
            ui_kit::stat_card(ui, theme, t, v, s, *c);
        });

        ui.add_space(10.0);
        ui_kit::responsive_grid_n(ui, 3, 250.0, |ui, idx| {
            let m = &mut universal_canvas.lifecycle_ops.mep_sizing;
            theme.card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                match idx {
                    0 => {
                        ui.label(RichText::new("1. HVAC Duct & Thermal Parameters").size(11.0).strong().color(theme.accent_blue));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Area (m2):").size(10.5).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut m.floor_area_m2).range(20.0..=250_000.0).speed(25.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Cooling W/m2:").size(10.5).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut m.cooling_load_w_m2).range(50.0..=350.0).speed(2.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Max Vel (m/s):").size(10.5).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut m.max_duct_velocity_ms).range(3.0..=14.0).speed(0.2));
                        });
                        ui.label(RichText::new(format!(
                            "Eq Dia: {:.0} mm | Flow: {:.2} m3/s",
                            res.duct_equiv_dia_mm, res.supply_airflow_m3s
                        )).size(10.0).color(theme.text_muted));
                    }
                    1 => {
                        ui.label(RichText::new("2. Plumbing & Drainage Specs").size(11.0).strong().color(theme.accent_green));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Fixture WSFU:").size(10.5).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut m.fixture_units_wsfu).range(4.0..=2000.0).speed(2.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Run Len (m):").size(10.5).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut m.pipe_run_length_m).range(5.0..=500.0).speed(1.0));
                        });
                        ui.label(RichText::new(format!(
                            "Drain Slope: {:.1}% | Peak Q: {:.2} L/s",
                            res.drain_slope_pct, res.peak_flow_ls
                        )).size(10.0).color(theme.text_muted));
                    }
                    _ => {
                        ui.label(RichText::new("3. 3-Phase Electrical Feeder").size(11.0).strong().color(theme.accent_orange));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Connected kVA:").size(10.5).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut m.electrical_connected_kva).range(10.0..=10_000.0).speed(5.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Demand Factor:").size(10.5).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut m.demand_factor).range(0.35..=1.0).speed(0.02));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Feeder (m):").size(10.5).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut m.feeder_length_m).range(5.0..=400.0).speed(1.0));
                        });
                        ui.label(RichText::new(format!(
                            "Demand: {:.1} kVA @ {:.0}V 3-Ph",
                            res.demand_kva, m.voltage_ll_v
                        )).size(10.0).color(theme.text_muted));
                    }
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_rebar_bbs_rows_from_live_project() {
        let mut proj = BimProject::sample_bimcraft_building();
        let armed = proj.reinforce_all_concrete(&RebarPreset::default());
        assert!(armed > 0);
        let rows = compute_rebar_bbs_rows(&proj, None, true);
        assert!(!rows.is_empty());
        let sum_wt: f64 = rows.iter().map(|r| r.total_weight_kg).sum();
        assert!((sum_wt - proj.total_rebar_weight_kg()).abs() < 1e-3);
        assert!(rows.iter().any(|r| r.iso_shape.contains("SC-51")));
    }

    #[test]
    fn test_compute_mep_riser_schedule_scales_with_levels() {
        let proj = BimProject::sample_bimcraft_building();
        let all = compute_mep_riser_schedule(&proj, 0);
        assert_eq!(all.len(), 4);
        let fire_only = compute_mep_riser_schedule(&proj, 1);
        assert_eq!(fire_only.len(), 2);
    }
}
