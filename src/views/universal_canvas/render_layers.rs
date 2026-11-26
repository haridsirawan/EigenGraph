//! Pure-vector 2D/3D canvas rendering layers for BIM, World buildings, hazards, and entities.

use super::*;
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::core::gis::SlippyMapState;
use crate::core::BimProject;
use crate::models::{NavigationPillar, StructuralMember};
use crate::theme::MacTheme;

impl UniversalCanvasState {
    pub(super) fn paint_grid(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        let step = self.grid_spacing_mm.max(250.0);
        let top_left_mm = self.screen_to_world(rect, rect.min);
        let bot_right_mm = self.screen_to_world(rect, rect.max);
        let mut min_x = (top_left_mm[0].min(bot_right_mm[0]) / step).floor() as i64 - 1;
        let mut max_x = (top_left_mm[0].max(bot_right_mm[0]) / step).ceil() as i64 + 1;
        let mut min_y = (top_left_mm[1].min(bot_right_mm[1]) / step).floor() as i64 - 1;
        let mut max_y = (top_left_mm[1].max(bot_right_mm[1]) / step).ceil() as i64 + 1;

        if self.globe_mode && ((max_x - min_x).abs() > 120 || (max_y - min_y).abs() > 120) {
            // When Earth 3D camera is pitched steeply toward horizon, draw a crisp local site grid around origin
            let half_cells = 30_i64;
            min_x = -half_cells;
            max_x = half_cells;
            min_y = -half_cells;
            max_y = half_cells;
        }

        let line_col = if theme.is_dark() {
            Color32::from_rgba_premultiplied(70, 90, 125, 28)
        } else {
            Color32::from_rgba_premultiplied(120, 140, 170, 42)
        };
        let major_col = if theme.is_dark() {
            Color32::from_rgba_premultiplied(90, 120, 165, 52)
        } else {
            Color32::from_rgba_premultiplied(100, 125, 165, 70)
        };

        // Limit line count on extreme zoom out
        if (max_x - min_x) <= 120 {
            for ix in min_x..=max_x {
                let x = ix as f64 * step;
                let p0 = to_screen([x, min_y as f64 * step]);
                let p1 = to_screen([x, max_y as f64 * step]);
                let col = if ix % 4 == 0 { major_col } else { line_col };
                painter.line_segment([p0, p1], Stroke::new(0.7, col));
            }
        }
        if (max_y - min_y) <= 120 {
            for iy in min_y..=max_y {
                let y = iy as f64 * step;
                let p0 = to_screen([min_x as f64 * step, y]);
                let p1 = to_screen([max_x as f64 * step, y]);
                let col = if iy % 4 == 0 { major_col } else { line_col };
                painter.line_segment([p0, p1], Stroke::new(0.7, col));
            }
        }

        // World Origin Axes
        let o = to_screen([0.0, 0.0]);
        if rect.expand(40.0).contains(o) {
            painter.line_segment(
                [o, o + Vec2::new(36.0, 0.0)],
                Stroke::new(1.8, theme.accent_red),
            );
            painter.line_segment(
                [o, o - Vec2::new(0.0, 36.0)],
                Stroke::new(1.8, theme.accent_green),
            );
            painter.circle_filled(o, 3.0, theme.accent_blue);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_bim_base_layer(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        bim_project: &BimProject,
        members: &[StructuralMember],
        active_pillar: NavigationPillar,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        let clip_painter = painter.with_clip_rect(rect);

        // 1. Structural Grid Axes & Bubbles
        for g in &bim_project.grids {
            let s0 = to_screen([g.start.0, g.start.1]);
            let s1 = to_screen([g.end.0, g.end.1]);
            clip_painter.line_segment(
                [s0, s1],
                Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.40)),
            );
            clip_painter.circle_filled(s1, 9.5, theme.bg_card);
            clip_painter.circle_stroke(s1, 9.5, Stroke::new(1.2, theme.accent_blue));
            clip_painter.text(
                s1,
                Align2::CENTER_CENTER,
                &g.label,
                egui::FontId::monospace(9.5),
                theme.accent_blue,
            );
        }

        // 2. Rooms Halftone Fill
        for rm in &bim_project.rooms {
            let pts: Vec<Pos2> = rm.outer_ring.iter().map(|p| to_screen([p.0, p.1])).collect();
            if pts.len() >= 3 {
                clip_painter.add(egui::Shape::convex_polygon(
                    pts,
                    theme.accent_green.gamma_multiply(0.08),
                    Stroke::NONE,
                ));
            }
            let c = to_screen([rm.seed_point.0, rm.seed_point.1]);
            clip_painter.text(
                c,
                Align2::CENTER_CENTER,
                format!("{} ({:.0} m²)", rm.number, rm.area_m2),
                egui::FontId::proportional(9.5),
                theme.text_muted,
            );
        }

        if !self.globe_mode {
            // 2B. Isolated Pad Footings (Flat 2D Projection)
            for ft in &bim_project.footings {
                let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Footing, ft.id));
                let ring: Vec<Pos2> = ft
                    .footprint_ring()
                    .into_iter()
                    .map(|p| to_screen([p.0, p.1]))
                    .collect();
                let fill = if is_sel {
                    theme.accent_blue.gamma_multiply(0.35)
                } else {
                    theme.accent_orange.gamma_multiply(0.14)
                };
                let stroke = if is_sel {
                    Stroke::new(2.2, Color32::from_rgb(56, 189, 248))
                } else {
                    Stroke::new(1.0, theme.accent_orange.gamma_multiply(0.45))
                };
                clip_painter.add(egui::Shape::convex_polygon(ring, fill, stroke));
            }

            // 3. Structural Beams / Runs (Flat 2D Projection)
            for run in &bim_project.runs {
                let s0 = to_screen([run.start.0, run.start.1]);
                let s1 = to_screen([run.end.0, run.end.1]);
                let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Run, run.id));
                let col = if is_sel {
                    Color32::from_rgb(56, 189, 248)
                } else if active_pillar == NavigationPillar::AnalysisSimulation {
                    let util = self
                        .earth_fem_cache
                        .member_for_bim(crate::core::BimElementKind::Run, run.id, members)
                        .map(|m| m.utilization)
                        .unwrap_or(0.45);
                    if util >= 0.85 {
                        theme.accent_red
                    } else if util >= 0.65 {
                        theme.accent_orange
                    } else {
                        theme.accent_green
                    }
                } else {
                    theme.accent_green.gamma_multiply(0.65)
                };
                clip_painter.line_segment([s0, s1], Stroke::new(if is_sel { 4.2 } else { 1.8 }, col));
            }

            // 4. Compound Walls (Flat 2D Projection)
            for w in &bim_project.walls {
                let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Wall, w.id));
                let ring: Vec<Pos2> = w
                    .footprint_ring()
                    .into_iter()
                    .map(|p| to_screen([p.0, p.1]))
                    .collect();
                let fill = if is_sel {
                    Color32::from_rgb(56, 189, 248).gamma_multiply(0.65)
                } else if theme.is_dark() {
                    Color32::from_rgb(65, 80, 102)
                } else {
                    Color32::from_rgb(148, 163, 184)
                };
                clip_painter.add(egui::Shape::convex_polygon(
                    ring,
                    fill.gamma_multiply(0.75),
                    if is_sel {
                        Stroke::new(2.2, Color32::from_rgb(56, 189, 248))
                    } else {
                        Stroke::new(1.2, theme.text_secondary.gamma_multiply(0.7))
                    },
                ));
            }

            // 5. Structural Columns (Flat 2D Projection)
            for col in &bim_project.columns {
                let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Column, col.id));
                let ring: Vec<Pos2> = col
                    .footprint_ring()
                    .into_iter()
                    .map(|p| to_screen([p.0, p.1]))
                    .collect();
                let fill = if is_sel {
                    Color32::from_rgb(56, 189, 248)
                } else if active_pillar == NavigationPillar::AnalysisSimulation {
                    let util = self
                        .earth_fem_cache
                        .member_for_bim(crate::core::BimElementKind::Column, col.id, members)
                        .map(|m| m.utilization)
                        .unwrap_or(0.45);
                    if util >= 0.85 {
                        theme.accent_red.gamma_multiply(0.85)
                    } else if util >= 0.65 {
                        theme.accent_orange.gamma_multiply(0.85)
                    } else {
                        theme.accent_green.gamma_multiply(0.85)
                    }
                } else {
                    theme.accent_blue.gamma_multiply(0.75)
                };
                clip_painter.add(egui::Shape::convex_polygon(
                    ring,
                    fill,
                    if is_sel {
                        Stroke::new(2.2, Color32::WHITE)
                    } else {
                        Stroke::new(1.2, theme.text_primary)
                    },
                ));
                if is_sel {
                    let cp = to_screen([col.pos.0, col.pos.1]);
                    if rect.expand(12.0).contains(cp) {
                        clip_painter.circle_stroke(cp, 14.0, Stroke::new(2.0, Color32::from_rgb(56, 189, 248)));
                        clip_painter.text(
                            cp + Vec2::new(0.0, -18.0),
                            Align2::CENTER_BOTTOM,
                            &col.name,
                            egui::FontId::monospace(10.0),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }

        // 6. Phase 55–63: Full 3D Extruded BIM Architecture, O(1) Structural Lenses, Deformed/Modal Animation & N-1 Disruption on Earth 3D
        if self.globe_mode
            && (self.ground_origin_anchored
                || !bim_project.is_empty()
                || self.earth_bim_author.active_tool != EarthBimTool::None)
        {
            if let Some(ref xf) = self.last_globe_xform {
                let lift_scale = if self.world_map.zoom_level < 14.5 { 8.0 } else { 1.0 };
                let active_lens = self.earth_bim_author.active_lens;
                let deform_scale = self.earth_bim_author.deform_scale;
                let modal_idx = self.earth_bim_author.active_modal_mode;
                let modal_playing = self.earth_bim_author.modal_anim_playing;
                let n1_active = self.earth_fem_cache.n1_simulation.is_some();
                let replay_frame_opt = if active_lens == EarthBimLens::TimeHistoryReplay {
                    self.current_replay_frame()
                } else {
                    None
                };
                let replay_hinges_map: std::collections::HashMap<
                    (crate::core::BimElementKind, usize),
                    crate::core::nonlinear::PlasticHingeState,
                > = if active_lens == EarthBimLens::TimeHistoryReplay {
                    self.last_ground_motion_replay_report
                        .as_ref()
                        .map(|rep| {
                            rep.worst_hinge_states_at(self.replay_cursor_time_s)
                                .into_iter()
                                .map(|(k, id, st)| ((k, id), st))
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    std::collections::HashMap::new()
                };
                let deform_visual_active = matches!(
                    active_lens,
                    EarthBimLens::DeformedShape
                        | EarthBimLens::ModalVibration
                        | EarthBimLens::TimeHistoryReplay
                ) || n1_active;

                let anim_time_s = if active_lens == EarthBimLens::ModalVibration {
                    if modal_playing {
                        clip_painter.ctx().request_repaint();
                    }
                    clip_painter.ctx().input(|i| i.time)
                } else {
                    if active_lens == EarthBimLens::TimeHistoryReplay && self.replay_anim_playing {
                        clip_painter.ctx().request_repaint();
                    }
                    0.0
                };

                let proj_mm = |x_mm: f64, y_mm: f64, elev_m: f64| -> Option<Pos2> {
                    self.project_active_bim_mm(xf, x_mm, y_mm, elev_m, lift_scale)
                };
                let proj_def_mm = |x_mm: f64, y_mm: f64, elev_m: f64| -> Option<Pos2> {
                    if !deform_visual_active {
                        return self.project_active_bim_mm(xf, x_mm, y_mm, elev_m, lift_scale);
                    }
                    if let Some(ref rf) = replay_frame_opt {
                        let vis = (deform_scale * 0.25).clamp(1.0, 28.0);
                        let shake_x_mm = rf.ground_disp_m * vis * 0.35 * 1000.0;
                        let shake_y_mm = rf.ground_disp_y_m * vis * 0.35 * 1000.0;
                        let shake_z_m = rf.ground_disp_z_m * vis * 0.22;
                        let n = rf
                            .story_elevations_m
                            .len()
                            .min(rf.story_disp_x_m.len())
                            .min(rf.story_disp_y_m.len());
                        let (mut dx_m, mut dy_m, mut dz_m) = (0.0, 0.0, 0.0);
                        if n > 0 {
                            dx_m = rf.story_disp_x_m[n - 1];
                            dy_m = rf.story_disp_y_m[n - 1];
                            dz_m = rf.story_disp_z_m.get(n - 1).copied().unwrap_or(0.0);
                            if elev_m <= rf.story_elevations_m[0] {
                                dx_m = rf.story_disp_x_m[0];
                                dy_m = rf.story_disp_y_m[0];
                                dz_m = rf.story_disp_z_m.first().copied().unwrap_or(0.0);
                            } else {
                                for k in 1..n {
                                    if elev_m <= rf.story_elevations_m[k] {
                                        let t = ((elev_m - rf.story_elevations_m[k - 1])
                                            / (rf.story_elevations_m[k] - rf.story_elevations_m[k - 1])
                                                .max(1e-4))
                                        .clamp(0.0, 1.0);
                                        dx_m = rf.story_disp_x_m[k - 1]
                                            + (rf.story_disp_x_m[k] - rf.story_disp_x_m[k - 1]) * t;
                                        dy_m = rf.story_disp_y_m[k - 1]
                                            + (rf.story_disp_y_m[k] - rf.story_disp_y_m[k - 1]) * t;
                                        let z0 = rf.story_disp_z_m.get(k - 1).copied().unwrap_or(0.0);
                                        let z1 = rf.story_disp_z_m.get(k).copied().unwrap_or(0.0);
                                        dz_m = z0 + (z1 - z0) * t;
                                        break;
                                    }
                                }
                            }
                        }
                        return self.project_active_bim_mm(
                            xf,
                            x_mm + (dx_m * vis * 1000.0 + shake_x_mm).clamp(-2800.0, 2800.0),
                            y_mm + (dy_m * vis * 1000.0 + shake_y_mm).clamp(-2800.0, 2800.0),
                            elev_m + (dz_m * vis * 0.45 + shake_z_m).clamp(-1.8, 1.8),
                            lift_scale,
                        );
                    }
                    let ([dx_mm, dy_mm, dz_m], _) = self.earth_fem_cache.sample_displacement_at_mm(
                        x_mm,
                        y_mm,
                        elev_m,
                        active_lens,
                        deform_scale,
                        modal_idx,
                        anim_time_s,
                        modal_playing,
                    );
                    self.project_active_bim_mm(
                        xf,
                        x_mm + dx_mm,
                        y_mm + dy_mm,
                        (elev_m + dz_m).max(-6.0),
                        lift_scale,
                    )
                };

                let disp_gradient_color = |norm: f64| -> Color32 {
                    let t = norm.clamp(0.0, 1.0);
                    if t >= 0.78 {
                        Color32::from_rgb(244, 63, 94) // Crimson peak sway
                    } else if t >= 0.48 {
                        Color32::from_rgb(245, 158, 11) // Amber mid-high sway
                    } else if t >= 0.22 {
                        Color32::from_rgb(91, 146, 165) // Coastal Teal moderate sway
                    } else {
                        Color32::from_rgb(208, 235, 225) // Seafoam Mint base/low sway
                    }
                };

                let lens_color = |kind: crate::core::BimElementKind,
                                  id: usize,
                                  level_idx: usize,
                                  default_col: Color32,
                                  is_sel: bool,
                                  disp_norm: f64| -> Color32 {
                    if is_sel {
                        return Color32::from_rgb(56, 189, 248);
                    }
                    // Interactive N-1 disruption override: highlight redistributed load paths & newly critical members
                    if let Some(ref n1) = self.earth_fem_cache.n1_simulation {
                        if let Some(fid) = self.earth_fem_cache.fem_id(kind, id) {
                            if n1.report.newly_critical_fem_ids.contains(&fid) {
                                return Color32::from_rgb(244, 63, 94);
                            }
                            if let Some(&post_dcr) = n1.report.post_member_dcr.get(&fid) {
                                return if post_dcr >= 0.85 {
                                    theme.accent_red
                                } else if post_dcr >= 0.65 {
                                    theme.accent_orange
                                } else {
                                    theme.accent_green
                                };
                            }
                        }
                    }
                    let mem_opt = self.earth_fem_cache.member_for_bim(kind, id, members);
                    match active_lens {
                        EarthBimLens::Clean => default_col,
                        EarthBimLens::DcrHeatmap => {
                            let u = mem_opt
                                .map(|m| m.dcr_interaction.max(m.utilization))
                                .unwrap_or(0.40);
                            if u >= 0.85 || mem_opt.is_some_and(|m| m.is_critical) {
                                theme.accent_red
                            } else if u >= 0.65 {
                                theme.accent_orange
                            } else {
                                theme.accent_green
                            }
                        }
                        EarthBimLens::EigenGateRedundancy => {
                            let eigengate = mem_opt.map(|m| m.eigengate).unwrap_or(0.95);
                            if eigengate < 0.70 {
                                theme.accent_red
                            } else if eigengate < 0.85 {
                                theme.accent_orange
                            } else {
                                Color32::from_rgb(16, 185, 129)
                            }
                        }
                        EarthBimLens::MomentStress => {
                            let m_max = mem_opt
                                .map(|m| m.moment_my_knm.abs().max(m.moment_mz_knm.abs()))
                                .unwrap_or(15.0);
                            if m_max > 120.0 {
                                Color32::from_rgb(244, 63, 94)
                            } else if m_max > 55.0 {
                                Color32::from_rgb(217, 70, 239)
                            } else {
                                Color32::from_rgb(129, 140, 248)
                            }
                        }
                        EarthBimLens::DeformedShape | EarthBimLens::ModalVibration => {
                            disp_gradient_color(disp_norm)
                        }
                        EarthBimLens::TimeHistoryReplay => {
                            use crate::core::nonlinear::PlasticHingeState;
                            match replay_hinges_map.get(&(kind, id)).copied() {
                                Some(PlasticHingeState::ImmediateOccupancy) => {
                                    Color32::from_rgb(10, 132, 255)
                                }
                                Some(PlasticHingeState::LifeSafety) => {
                                    Color32::from_rgb(255, 159, 10)
                                }
                                Some(PlasticHingeState::CollapsePrevention) => {
                                    Color32::from_rgb(255, 69, 58)
                                }
                                Some(PlasticHingeState::Mechanism) => {
                                    Color32::from_rgb(191, 90, 242)
                                }
                                _ => Color32::from_rgb(91, 146, 165),
                            }
                        }
                        EarthBimLens::ShmDigitalTwin => {
                            if let Some(ref shm) = self.last_shm_twin_report {
                                let sh_opt = shm.story_health.get(level_idx.min(shm.story_health.len().saturating_sub(1)));
                                match sh_opt.map(|s| s.story_placard) {
                                    Some(crate::core::Atc20Placard::UnsafeRed) => {
                                        Color32::from_rgb(239, 68, 68)
                                    }
                                    Some(crate::core::Atc20Placard::RestrictedYellow) => {
                                        Color32::from_rgb(245, 158, 11)
                                    }
                                    _ => Color32::from_rgb(34, 197, 94),
                                }
                            } else {
                                Color32::from_rgb(34, 197, 94)
                            }
                        }
                        EarthBimLens::Erection4D => {
                            if level_idx == 0 {
                                Color32::from_rgb(16, 185, 129)
                            } else if level_idx == 1 {
                                Color32::from_rgb(56, 189, 248)
                            } else {
                                Color32::from_rgb(251, 191, 36)
                            }
                        }
                    }
                };
                let level_rank = |lvl_id: usize| -> usize {
                    bim_project.levels.iter().position(|l| l.id == lvl_id).unwrap_or(0)
                };
                let isolate = self.earth_bim_author.isolate_active_story;
                let active_lvl_id = self.earth_bim_author.base_level_id.max(1);
                let active_top_id = self.earth_bim_author.top_level_id.max(active_lvl_id);
                let show_on_story = |lvl: usize| -> bool { !isolate || lvl == active_lvl_id };
                let sec_active = self.earth_bim_author.section_cut_active;
                let sec_elev = self.earth_bim_author.section_cut_elev_m;

                // 6.0 Undeformed Reference Ghost Wireframe when Deformed / Modal / N-1 Disruption is active
                if deform_visual_active {
                    let ghost_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(208, 235, 225, 62));
                    for col in &bim_project.columns {
                        if isolate && col.base_level_id != active_lvl_id && col.top_level_id != active_lvl_id {
                            continue;
                        }
                        let b_m = bim_project.level_elevation(col.base_level_id) * 0.001;
                        let t_m = bim_project.level_elevation(col.top_level_id) * 0.001;
                        if let (Some(g0), Some(g1)) = (
                            proj_mm(col.pos.0, col.pos.1, b_m),
                            proj_mm(col.pos.0, col.pos.1, t_m),
                        ) {
                            clip_painter.line_segment([g0, g1], ghost_stroke);
                        }
                    }
                    for run in &bim_project.runs {
                        if isolate && run.level_id != active_lvl_id && run.level_id != active_top_id {
                            continue;
                        }
                        let e_m = bim_project.level_elevation(run.level_id) * 0.001;
                        if let (Some(g0), Some(g1)) = (
                            proj_mm(run.start.0, run.start.1, e_m),
                            proj_mm(run.end.0, run.end.1, e_m),
                        ) {
                            clip_painter.line_segment([g0, g1], ghost_stroke);
                        }
                    }
                }

                // 6A. Subsurface 3D Foundation Pads / Pile Caps & Deep Foundation Pile Shafts
                for ft in &bim_project.footings {
                    if !show_on_story(ft.level_id) && !(isolate && active_lvl_id <= 1) {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Footing, ft.id));
                    let z_top_m = bim_project.level_elevation(ft.level_id) * 0.001;
                    if sec_active && z_top_m > sec_elev + 0.05 {
                        continue;
                    }
                    let cap_h_m = (ft.thickness_mm * 0.001).max(0.45);
                    let z_bot_m = z_top_m - cap_h_m;
                    let ring = ft.footprint_ring();

                    // Subsurface 4-pile drilled shafts below pile cap at ground/zoomed scale or when selected
                    if is_sel || self.world_map.zoom_level >= 15.8 {
                        let z_pile_tip_m = z_bot_m - 3.8;
                        let ox = ft.width_mm * 0.32;
                        let oy = ft.length_mm * 0.32;
                        let pile_col = if is_sel {
                            Color32::from_rgba_unmultiplied(91, 146, 165, 220)
                        } else {
                            Color32::from_rgba_unmultiplied(91, 146, 165, 130)
                        };
                        let pile_stroke_w = if self.world_map.zoom_level >= 17.5 { 3.2 } else { 2.0 };
                        for [sx, sy] in [[-ox, -oy], [ox, -oy], [ox, oy], [-ox, oy]] {
                            if let (Some(p_top), Some(p_tip)) = (
                                proj_mm(ft.pos.0 + sx, ft.pos.1 + sy, z_bot_m),
                                proj_mm(ft.pos.0 + sx, ft.pos.1 + sy, z_pile_tip_m),
                            ) {
                                if rect.expand(60.0).contains(p_top) || rect.expand(60.0).contains(p_tip) {
                                    clip_painter.line_segment(
                                        [p_top, p_tip],
                                        Stroke::new(pile_stroke_w, pile_col),
                                    );
                                    clip_painter.circle_filled(p_tip, 2.2, pile_col);
                                }
                            }
                        }
                    }

                    // 3D Extruded Pile Cap Block (Subsurface X-Ray side quads + top cap)
                    let top_pts: Vec<Pos2> = ring
                        .iter()
                        .filter_map(|p| proj_mm(p.0, p.1, z_top_m))
                        .collect();
                    let bot_pts: Vec<Pos2> = ring
                        .iter()
                        .filter_map(|p| proj_mm(p.0, p.1, z_bot_m))
                        .collect();
                    if top_pts.len() == 4 && top_pts.iter().any(|p| rect.expand(40.0).contains(*p)) {
                        if bot_pts.len() == 4 {
                            let side_base = if is_sel {
                                Color32::from_rgb(91, 146, 165)
                            } else {
                                Color32::from_rgb(206, 134, 42)
                            };
                            for k in 0..4 {
                                let nk = (k + 1) % 4;
                                let face = vec![bot_pts[k], bot_pts[nk], top_pts[nk], top_pts[k]];
                                let side_fill = Self::directional_face_shade(side_base, bot_pts[k], bot_pts[nk], false)
                                    .gamma_multiply(0.55);
                                clip_painter.add(egui::Shape::convex_polygon(
                                    face,
                                    side_fill,
                                    Stroke::new(0.8, side_base.gamma_multiply(0.65)),
                                ));
                            }
                        }
                        let fill = if is_sel {
                            Color32::from_rgba_unmultiplied(91, 146, 165, 135)
                        } else {
                            Color32::from_rgba_unmultiplied(238, 168, 72, 85)
                        };
                        let stroke = if is_sel {
                            Stroke::new(2.2, Color32::from_rgb(208, 235, 225))
                        } else {
                            Stroke::new(1.2, Color32::from_rgba_unmultiplied(238, 168, 72, 200))
                        };
                        clip_painter.add(egui::Shape::convex_polygon(top_pts, fill, stroke));
                    }
                }

                // 6B. Floor & Roof Slabs
                for sl in &bim_project.slabs {
                    if sl.boundary.len() < 3 {
                        continue;
                    }
                    if isolate && sl.level_id != active_lvl_id && sl.level_id != active_top_id {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Slab, sl.id));
                    let z_m = bim_project.level_elevation(sl.level_id) * 0.001;
                    if sec_active && z_m > sec_elev + 0.05 {
                        continue;
                    }
                    let pts: Vec<Pos2> = sl
                        .boundary
                        .iter()
                        .filter_map(|p| proj_def_mm(p.0, p.1, z_m))
                        .collect();
                    if pts.len() >= 3 && pts.iter().any(|p| rect.expand(60.0).contains(*p)) {
                        let fill = if is_sel {
                            Color32::from_rgba_unmultiplied(56, 189, 248, 95)
                        } else if sl.is_roof {
                            Color32::from_rgba_unmultiplied(234, 88, 12, 55)
                        } else {
                            Color32::from_rgba_unmultiplied(148, 163, 184, 52)
                        };
                        let stroke = if is_sel {
                            Stroke::new(2.2, Color32::from_rgb(56, 189, 248))
                        } else if sl.is_roof {
                            Stroke::new(1.3, Color32::from_rgba_unmultiplied(251, 146, 60, 210))
                        } else {
                            Stroke::new(1.1, Color32::from_rgba_unmultiplied(203, 213, 225, 175))
                        };
                        clip_painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
                    }
                }

                // 6C. Extruded 3D Walls & Door/Window Openings (with Directional Sun Shading & Section Cut)
                for w in &bim_project.walls {
                    if !show_on_story(w.base_level_id) {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Wall, w.id));
                    let z0 = bim_project.level_elevation(w.base_level_id) * 0.001;
                    let raw_z1 = w
                        .top_level_id
                        .map(|tid| bim_project.level_elevation(tid))
                        .unwrap_or_else(|| bim_project.level_elevation(w.base_level_id) + w.unconnected_height_mm)
                        * 0.001;
                    if sec_active && z0 >= sec_elev - 0.02 {
                        continue;
                    }
                    let wall_sliced = sec_active && raw_z1 > sec_elev + 0.01;
                    let z1 = if wall_sliced { sec_elev } else { raw_z1 };
                    let wall_n1_removed = self
                        .earth_fem_cache
                        .is_n1_removed(crate::core::BimElementKind::Wall, w.id);
                    if let (Some(b0), Some(b1), Some(t1), Some(t0)) = (
                        proj_def_mm(w.start.0, w.start.1, z0),
                        proj_def_mm(w.end.0, w.end.1, z0),
                        proj_def_mm(w.end.0, w.end.1, z1),
                        proj_def_mm(w.start.0, w.start.1, z1),
                    ) {
                        if [b0, b1, t1, t0].iter().any(|p| rect.expand(60.0).contains(*p)) {
                            if wall_n1_removed {
                                let rem_stroke = Stroke::new(2.0, Color32::from_rgb(244, 63, 94));
                                clip_painter.add(egui::Shape::convex_polygon(
                                    vec![b0, b1, t1, t0],
                                    Color32::from_rgba_unmultiplied(244, 63, 94, 34),
                                    rem_stroke,
                                ));
                                clip_painter.line_segment([b0, t1], rem_stroke);
                                clip_painter.line_segment([b1, t0], rem_stroke);
                                let mid = Pos2::new((t0.x + t1.x) * 0.5, (t0.y + t1.y) * 0.5 - 6.0);
                                let mech = self
                                    .earth_fem_cache
                                    .n1_simulation
                                    .as_ref()
                                    .is_some_and(|s| s.report.is_kinematic_mechanism);
                                clip_painter.text(
                                    mid,
                                    Align2::CENTER_BOTTOM,
                                    if mech { "✕ N-1 MECHANISM" } else { "✕ N-1 REMOVED" },
                                    egui::FontId::monospace(9.5),
                                    Color32::from_rgb(253, 164, 175),
                                );
                                continue;
                            }
                            let base_wall_col = if is_sel {
                                Color32::from_rgb(56, 189, 248)
                            } else if w.is_structural {
                                let (_, d_norm) = self.earth_fem_cache.sample_displacement_at_mm(
                                    0.5 * (w.start.0 + w.end.0),
                                    0.5 * (w.start.1 + w.end.1),
                                    z1,
                                    active_lens,
                                    deform_scale,
                                    modal_idx,
                                    anim_time_s,
                                    modal_playing,
                                );
                                lens_color(
                                    crate::core::BimElementKind::Wall,
                                    w.id,
                                    level_rank(w.base_level_id),
                                    Color32::from_rgb(112, 130, 156),
                                    false,
                                    d_norm,
                                )
                            } else {
                                Color32::from_rgb(162, 176, 196)
                            };
                            let wall_fill = Self::directional_face_shade(base_wall_col, b0, b1, false);
                            let wall_stroke = if is_sel {
                                Stroke::new(2.3, Color32::from_rgb(56, 189, 248))
                            } else {
                                Stroke::new(1.3, Color32::from_rgba_unmultiplied(226, 232, 240, 185))
                            };
                            clip_painter.add(egui::Shape::convex_polygon(
                                vec![b0, b1, t1, t0],
                                wall_fill,
                                wall_stroke,
                            ));
                            if wall_sliced {
                                clip_painter.line_segment(
                                    [t0, t1],
                                    Stroke::new(2.6, Color32::from_rgb(245, 158, 11)),
                                );
                            }
                            // Render CSG Door/Window Openings directly on the 3D wall face
                            let len_mm = w.length_mm().max(1.0);
                            for op in &w.openings {
                                let u0 = (op.offset_along_mm / len_mm).clamp(0.0, 1.0);
                                let u1 = ((op.offset_along_mm + op.width_mm) / len_mm).clamp(0.0, 1.0);
                                let oz0 = z0 + op.sill_height_mm * 0.001;
                                let oz1 = (oz0 + op.height_mm * 0.001).min(z1);
                                if oz1 <= oz0 + 0.05 {
                                    continue;
                                }
                                let x0 = w.start.0 + (w.end.0 - w.start.0) * u0;
                                let y0 = w.start.1 + (w.end.1 - w.start.1) * u0;
                                let x1 = w.start.0 + (w.end.0 - w.start.0) * u1;
                                let y1 = w.start.1 + (w.end.1 - w.start.1) * u1;
                                if let (Some(ob0), Some(ob1), Some(ot1), Some(ot0)) = (
                                    proj_def_mm(x0, y0, oz0),
                                    proj_def_mm(x1, y1, oz0),
                                    proj_def_mm(x1, y1, oz1),
                                    proj_def_mm(x0, y0, oz1),
                                ) {
                                    let is_door = matches!(
                                        op.kind,
                                        crate::core::OpeningKind::SingleDoor
                                            | crate::core::OpeningKind::DoubleDoor
                                            | crate::core::OpeningKind::SlidingDoor
                                    );
                                    let (ofill, ocol) = if is_door {
                                        (
                                            Color32::from_rgba_unmultiplied(245, 158, 11, 115),
                                            Color32::from_rgb(251, 191, 36),
                                        )
                                    } else {
                                        (
                                            Color32::from_rgba_unmultiplied(56, 189, 248, 125),
                                            Color32::from_rgb(125, 211, 252),
                                        )
                                    };
                                    clip_painter.add(egui::Shape::convex_polygon(
                                        vec![ob0, ob1, ot1, ot0],
                                        ofill,
                                        Stroke::new(1.2, ocol),
                                    ));
                                }
                            }
                        }
                    }
                }

                // 6D. Structural Columns (Directional Sun-Shaded Prisms + Deformed/Modal + N-1 Ghost + Force Diagrams)
                let mut apex_callout_pt: Option<(f64, Pos2)> = None;
                for col in &bim_project.columns {
                    if isolate && col.base_level_id != active_lvl_id && col.top_level_id != active_lvl_id {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Column, col.id));
                    let base_m = bim_project.level_elevation(col.base_level_id) * 0.001;
                    let raw_top_m = bim_project.level_elevation(col.top_level_id) * 0.001;
                    if sec_active && base_m >= sec_elev - 0.02 {
                        continue;
                    }
                    let col_sliced = sec_active && raw_top_m > sec_elev + 0.01;
                    let top_m = if col_sliced { sec_elev } else { raw_top_m };

                    let col_n1_removed = self
                        .earth_fem_cache
                        .is_n1_removed(crate::core::BimElementKind::Column, col.id);
                    if col_n1_removed {
                        if let (Some(p0), Some(p1)) = (
                            proj_mm(col.pos.0, col.pos.1, base_m),
                            proj_mm(col.pos.0, col.pos.1, top_m),
                        ) {
                            let rem_col = Color32::from_rgb(244, 63, 94);
                            let segs = 6usize;
                            for s in (0..segs).step_by(2) {
                                let t0 = s as f32 / segs as f32;
                                let t1 = ((s + 1) as f32 / segs as f32).min(1.0);
                                let a = Pos2::new(p0.x + (p1.x - p0.x) * t0, p0.y + (p1.y - p0.y) * t0);
                                let b = Pos2::new(p0.x + (p1.x - p0.x) * t1, p0.y + (p1.y - p0.y) * t1);
                                clip_painter.line_segment([a, b], Stroke::new(2.4, rem_col));
                            }
                            let mid = Pos2::new(0.5 * (p0.x + p1.x), 0.5 * (p0.y + p1.y));
                            clip_painter.line_segment(
                                [mid + Vec2::new(-7.0, -7.0), mid + Vec2::new(7.0, 7.0)],
                                Stroke::new(2.2, rem_col),
                            );
                            clip_painter.line_segment(
                                [mid + Vec2::new(7.0, -7.0), mid + Vec2::new(-7.0, 7.0)],
                                Stroke::new(2.2, rem_col),
                            );
                            let mech = self
                                .earth_fem_cache
                                .n1_simulation
                                .as_ref()
                                .is_some_and(|s| s.report.is_kinematic_mechanism);
                            clip_painter.text(
                                p1 + Vec2::new(0.0, -10.0),
                                Align2::CENTER_BOTTOM,
                                if mech {
                                    format!("✕ {} [MECHANISM]", col.name)
                                } else {
                                    format!("✕ {} [N-1 REMOVED]", col.name)
                                },
                                egui::FontId::monospace(9.5),
                                Color32::from_rgb(253, 164, 175),
                            );
                        }
                        continue;
                    }

                    let (_, col_disp_norm) = self.earth_fem_cache.sample_displacement_at_mm(
                        col.pos.0,
                        col.pos.1,
                        top_m,
                        active_lens,
                        deform_scale,
                        modal_idx,
                        anim_time_s,
                        modal_playing,
                    );
                    let col_color = lens_color(
                        crate::core::BimElementKind::Column,
                        col.id,
                        level_rank(col.base_level_id),
                        Color32::from_rgba_unmultiplied(56, 189, 248, 235),
                        is_sel,
                        col_disp_norm,
                    );
                    let is_n1_redist = self.earth_fem_cache.n1_simulation.as_ref().is_some_and(|s| {
                        self.earth_fem_cache
                            .fem_id(crate::core::BimElementKind::Column, col.id)
                            .is_some_and(|fid| s.report.newly_critical_fem_ids.contains(&fid))
                    });
                    if self.world_map.zoom_level >= 16.5 {
                        let ring = col.footprint_ring();
                        let b_pts: Vec<Pos2> = ring.iter().filter_map(|p| proj_def_mm(p.0, p.1, base_m)).collect();
                        let t_pts: Vec<Pos2> = ring.iter().filter_map(|p| proj_def_mm(p.0, p.1, top_m)).collect();
                        if b_pts.len() == 4 && t_pts.len() == 4 && t_pts.iter().any(|p| rect.expand(45.0).contains(*p)) {
                            for i in 0..4 {
                                let ni = (i + 1) % 4;
                                let face_fill = Self::directional_face_shade(col_color, b_pts[i], b_pts[ni], false);
                                clip_painter.add(egui::Shape::convex_polygon(
                                    vec![b_pts[i], b_pts[ni], t_pts[ni], t_pts[i]],
                                    face_fill,
                                    Stroke::new(if is_sel || is_n1_redist { 1.8 } else { 1.0 }, col_color),
                                ));
                            }
                            let (cap_fill, cap_stroke) = if col_sliced {
                                (
                                    Color32::from_rgba_unmultiplied(245, 158, 11, 215),
                                    Stroke::new(2.0, Color32::from_rgb(248, 113, 113)),
                                )
                            } else {
                                (
                                    Self::directional_face_shade(col_color, t_pts[0], t_pts[1], true),
                                    Stroke::new(if is_sel { 2.2 } else { 1.2 }, Color32::WHITE.gamma_multiply(0.7)),
                                )
                            };
                            clip_painter.add(egui::Shape::convex_polygon(t_pts, cap_fill, cap_stroke));
                        }
                    }
                    if let (Some(p0), Some(p1)) = (
                        proj_def_mm(col.pos.0, col.pos.1, base_m),
                        proj_def_mm(col.pos.0, col.pos.1, top_m),
                    ) {
                        if apex_callout_pt.map_or(true, |(h, _)| top_m > h) {
                            apex_callout_pt = Some((top_m, p1));
                        }
                        if rect.expand(45.0).contains(p0) || rect.expand(45.0).contains(p1) {
                            clip_painter.line_segment(
                                [p0, p1],
                                Stroke::new(if is_sel || is_n1_redist { 4.2 } else { 2.4 }, col_color),
                            );
                            if is_n1_redist {
                                clip_painter.circle_stroke(
                                    p1,
                                    7.5,
                                    Stroke::new(1.8, Color32::from_rgb(251, 191, 36)),
                                );
                            }
                            if active_lens == EarthBimLens::MomentStress {
                                let m_col = self
                                    .earth_fem_cache
                                    .member_for_bim(crate::core::BimElementKind::Column, col.id, members)
                                    .map(|m| m.moment_my_knm.abs().max(m.moment_mz_knm.abs()).max(m.shear_vy_kn.abs()))
                                    .unwrap_or(28.0)
                                    .max(12.0);
                                let dx_mm = (m_col * 8.5).clamp(220.0, 950.0);
                                let mid_m = 0.5 * (base_m + top_m);
                                if let (Some(pm), Some(ft), Some(fb)) = (
                                    proj_def_mm(col.pos.0, col.pos.1, mid_m),
                                    proj_def_mm(col.pos.0 + dx_mm, col.pos.1, top_m),
                                    proj_def_mm(col.pos.0 - dx_mm * 0.7, col.pos.1, base_m),
                                ) {
                                    let fin_fill = Color32::from_rgba_unmultiplied(168, 85, 247, 72);
                                    let fin_stroke = Stroke::new(1.5, Color32::from_rgb(192, 132, 252));
                                    clip_painter.add(egui::Shape::convex_polygon(
                                        vec![p1, ft, pm],
                                        fin_fill,
                                        fin_stroke,
                                    ));
                                    clip_painter.add(egui::Shape::convex_polygon(
                                        vec![p0, fb, pm],
                                        fin_fill,
                                        fin_stroke,
                                    ));
                                }
                            }
                            if is_sel {
                                clip_painter.circle_stroke(p1, 10.0, Stroke::new(1.8, Color32::WHITE));
                                clip_painter.text(
                                    p1 + Vec2::new(0.0, -14.0),
                                    Align2::CENTER_BOTTOM,
                                    &col.name,
                                    egui::FontId::monospace(10.0),
                                    Color32::WHITE,
                                );
                            }
                        }
                    }
                }

                // 6E. Structural Beams / Runs (Sun-Shaded 3D Extruded Prisms + Deformed/Modal + N-1 Ghost + 3D Moment Diagrams)
                for (beam_idx, run) in bim_project.runs.iter().enumerate() {
                    if isolate && run.level_id != active_lvl_id && run.level_id != active_top_id {
                        continue;
                    }
                    let is_sel = self.highlighted_bim_element == Some((crate::core::BimElementKind::Run, run.id));
                    let elev_m = bim_project.level_elevation(run.level_id) * 0.001;
                    if sec_active && elev_m > sec_elev + 0.05 {
                        continue;
                    }

                    let beam_n1_removed = self
                        .earth_fem_cache
                        .is_n1_removed(crate::core::BimElementKind::Run, run.id);
                    if beam_n1_removed {
                        if let (Some(p0), Some(p1)) = (
                            proj_mm(run.start.0, run.start.1, elev_m),
                            proj_mm(run.end.0, run.end.1, elev_m),
                        ) {
                            let rem_col = Color32::from_rgb(244, 63, 94);
                            let segs = 6usize;
                            for s in (0..segs).step_by(2) {
                                let t0 = s as f32 / segs as f32;
                                let t1 = ((s + 1) as f32 / segs as f32).min(1.0);
                                let a = Pos2::new(p0.x + (p1.x - p0.x) * t0, p0.y + (p1.y - p0.y) * t0);
                                let b = Pos2::new(p0.x + (p1.x - p0.x) * t1, p0.y + (p1.y - p0.y) * t1);
                                clip_painter.line_segment([a, b], Stroke::new(2.4, rem_col));
                            }
                            let mid = Pos2::new(0.5 * (p0.x + p1.x), 0.5 * (p0.y + p1.y));
                            clip_painter.text(
                                mid + Vec2::new(0.0, -8.0),
                                Align2::CENTER_BOTTOM,
                                format!("✕ {} [N-1]", run.name),
                                egui::FontId::monospace(9.5),
                                Color32::from_rgb(253, 164, 175),
                            );
                        }
                        continue;
                    }

                    let (_, beam_disp_norm) = self.earth_fem_cache.sample_displacement_at_mm(
                        0.5 * (run.start.0 + run.end.0),
                        0.5 * (run.start.1 + run.end.1),
                        elev_m,
                        active_lens,
                        deform_scale,
                        modal_idx,
                        anim_time_s,
                        modal_playing,
                    );
                    let beam_col = lens_color(
                        crate::core::BimElementKind::Run,
                        run.id,
                        level_rank(run.level_id),
                        theme.accent_green.gamma_multiply(0.92),
                        is_sel,
                        beam_disp_norm,
                    );
                    let is_n1_redist = self.earth_fem_cache.n1_simulation.as_ref().is_some_and(|s| {
                        self.earth_fem_cache
                            .fem_id(crate::core::BimElementKind::Run, run.id)
                            .is_some_and(|fid| s.report.newly_critical_fem_ids.contains(&fid))
                    });
                    let is_brace = run.category == crate::core::bim::RunCategory::StructuralBrace
                        || (run.end_offset_mm - run.start_offset_mm).abs() > 10.0;
                    if is_brace {
                        let z0_m = elev_m + run.start_offset_mm * 0.001;
                        let z1_m = elev_m + run.end_offset_mm * 0.001;
                        let brace_col = if active_lens == EarthBimLens::Clean {
                            Color32::from_rgb(245, 174, 41)
                        } else {
                            beam_col
                        };
                        if let (Some(p0), Some(p1)) = (
                            proj_def_mm(run.start.0, run.start.1, z0_m),
                            proj_def_mm(run.end.0, run.end.1, z1_m),
                        ) {
                            if rect.expand(45.0).contains(p0) || rect.expand(45.0).contains(p1) {
                                clip_painter.line_segment(
                                    [p0, p1],
                                    Stroke::new(if is_sel || is_n1_redist { 5.2 } else { 3.6 }, brace_col),
                                );
                            }
                        }
                        continue;
                    }
                    if self.world_map.zoom_level >= 16.5 {
                        let z_bot = (elev_m - run.depth_mm * 0.001).max(0.0);
                        let ring = run.footprint_ring();
                        let b_pts: Vec<Pos2> = ring.iter().filter_map(|p| proj_def_mm(p.0, p.1, z_bot)).collect();
                        let t_pts: Vec<Pos2> = ring.iter().filter_map(|p| proj_def_mm(p.0, p.1, elev_m)).collect();
                        if b_pts.len() == 4 && t_pts.len() == 4 && t_pts.iter().any(|p| rect.expand(45.0).contains(*p)) {
                            for i in 0..4 {
                                let ni = (i + 1) % 4;
                                let side_fill = Self::directional_face_shade(beam_col, b_pts[i], b_pts[ni], false);
                                clip_painter.add(egui::Shape::convex_polygon(
                                    vec![b_pts[i], b_pts[ni], t_pts[ni], t_pts[i]],
                                    side_fill,
                                    Stroke::new(if is_sel || is_n1_redist { 1.6 } else { 0.85 }, beam_col),
                                ));
                            }
                            let top_fill = Self::directional_face_shade(beam_col, t_pts[0], t_pts[1], true);
                            clip_painter.add(egui::Shape::convex_polygon(
                                t_pts,
                                top_fill,
                                Stroke::new(if is_sel { 1.9 } else { 1.0 }, Color32::WHITE.gamma_multiply(0.55)),
                            ));
                        }
                    }
                    if let (Some(p0), Some(p1)) = (
                        proj_def_mm(run.start.0, run.start.1, elev_m),
                        proj_def_mm(run.end.0, run.end.1, elev_m),
                    ) {
                        if rect.expand(45.0).contains(p0) || rect.expand(45.0).contains(p1) {
                            clip_painter.line_segment(
                                [p0, p1],
                                Stroke::new(if is_sel || is_n1_redist { 4.2 } else { 2.3 }, beam_col),
                            );
                            if active_lens == EarthBimLens::MomentStress {
                                let m_knm = self
                                    .earth_fem_cache
                                    .member_for_bim(crate::core::BimElementKind::Run, run.id, members)
                                    .map(|m| m.moment_my_knm.abs().max(m.moment_mz_knm.abs()))
                                    .unwrap_or(45.0)
                                    .max(15.0);
                                let mut base_pts = Vec::with_capacity(7);
                                let mut curve_pts = Vec::with_capacity(7);
                                for t in 0..=6 {
                                    let u = t as f64 / 6.0;
                                    let sx = run.start.0 + (run.end.0 - run.start.0) * u;
                                    let sy = run.start.1 + (run.end.1 - run.start.1) * u;
                                    let dz_m = 4.0 * u * (1.0 - u) * (m_knm / 120.0).clamp(0.25, 1.45);
                                    if let (Some(bp), Some(cp)) = (
                                        proj_def_mm(sx, sy, elev_m),
                                        proj_def_mm(sx, sy, elev_m + dz_m),
                                    ) {
                                        base_pts.push(bp);
                                        curve_pts.push(cp);
                                    }
                                }
                                if base_pts.len() == 7 && curve_pts.len() == 7 {
                                    let rib_fill = Color32::from_rgba_unmultiplied(244, 63, 94, 72);
                                    let rib_stroke = Stroke::new(1.8, Color32::from_rgb(244, 63, 94));
                                    for k in 0..6 {
                                        clip_painter.add(egui::Shape::convex_polygon(
                                            vec![base_pts[k], base_pts[k + 1], curve_pts[k + 1], curve_pts[k]],
                                            rib_fill,
                                            Stroke::NONE,
                                        ));
                                        clip_painter.line_segment([curve_pts[k], curve_pts[k + 1]], rib_stroke);
                                    }
                                    if is_sel || m_knm > 65.0 || beam_idx < 3 {
                                        clip_painter.text(
                                            curve_pts[3] + Vec2::new(0.0, -6.0),
                                            Align2::CENTER_BOTTOM,
                                            format!("M={m_knm:.0}kNm"),
                                            egui::FontId::monospace(9.5),
                                            Color32::from_rgb(253, 164, 175),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // 6E-ter. Peak 3D Deformed / Modal Eigenmode Apex Telemetry Badge
                if let Some((_, apex_pt)) = apex_callout_pt {
                    if rect.expand(40.0).contains(apex_pt) {
                        if active_lens == EarthBimLens::ModalVibration {
                            if let Some(m) = self
                                .earth_fem_cache
                                .modal_modes
                                .get(modal_idx.min(self.earth_fem_cache.modal_modes.len().saturating_sub(1)))
                            {
                                clip_painter.text(
                                    apex_pt + Vec2::new(0.0, -14.0),
                                    Align2::CENTER_BOTTOM,
                                    format!(
                                        "Mode {} • T={:.2}s • f={:.2}Hz • Mx={:.0}%",
                                        m.mode_number, m.period_s, m.frequency_hz, m.mass_participation_ratio_x
                                    ),
                                    egui::FontId::monospace(10.0),
                                    Color32::from_rgb(208, 235, 225),
                                );
                            }
                        } else if active_lens == EarthBimLens::ShmDigitalTwin {
                            if let Some(ref shm) = self.last_shm_twin_report {
                                let [pr, pg, pb] = shm.atc20_placard.badge_color_rgb();
                                clip_painter.text(
                                    apex_pt + Vec2::new(0.0, -14.0),
                                    Align2::CENTER_BOTTOM,
                                    format!(
                                        "ATC-20 {} • f1={:.2}Hz ({:+.1}%) • MAC={:.2} • K_ret={:.0}%",
                                        shm.atc20_placard.short_label(),
                                        shm.observed_freq_1_hz,
                                        shm.freq_shift_pct,
                                        shm.mac_mode_1,
                                        shm.global_stiffness_retention_pct
                                    ),
                                    egui::FontId::monospace(10.0),
                                    Color32::from_rgb(pr, pg, pb),
                                );
                            }
                        } else if active_lens == EarthBimLens::TimeHistoryReplay {
                            if let Some(ref rf) = replay_frame_opt {
                                let rec_lbl = self.ground_motion_replay_config.active_record_label();
                                let mode_lbl = self
                                    .ground_motion_replay_config
                                    .tridirectional_mode
                                    .short_label();
                                clip_painter.text(
                                    apex_pt + Vec2::new(0.0, -14.0),
                                    Align2::CENTER_BOTTOM,
                                    format!(
                                        "{rec_lbl} ({mode_lbl}) • t={:.1}s • Drift={:.2}% • Hinges={}",
                                        rf.time_s,
                                        rf.roof_drift_ratio * 100.0,
                                        rf.active_hinges.len()
                                    ),
                                    egui::FontId::monospace(10.0),
                                    Color32::from_rgb(208, 235, 225),
                                );
                            }
                        } else if active_lens == EarthBimLens::DeformedShape && self.earth_fem_cache.max_disp_mm > 0.01 {
                            let disp_mm = self
                                .earth_fem_cache
                                .n1_simulation
                                .as_ref()
                                .map(|s| s.report.post_max_disp_mm)
                                .unwrap_or(self.earth_fem_cache.max_disp_mm);
                            let drift_pct = self.earth_fem_cache.max_story_drift_ratio * 100.0;
                            clip_painter.text(
                                apex_pt + Vec2::new(0.0, -14.0),
                                Align2::CENTER_BOTTOM,
                                format!("Δmax={disp_mm:.1}mm ({deform_scale:.0}x) • Drift={drift_pct:.2}%"),
                                egui::FontId::monospace(10.0),
                                Color32::from_rgb(251, 191, 36),
                            );
                        }
                    }
                }

                // 6E-bis. Interactive 3D Section Cut Horizontal Plane Indicator
                if sec_active {
                    let (min_x, max_x, min_y, max_y) = bim_project
                        .plan_extents_mm()
                        .map(|(pmin, pmax)| (pmin[0] - 1500.0, pmax[0] + 1500.0, pmin[1] - 1500.0, pmax[1] + 1500.0))
                        .unwrap_or((-4000.0, 4000.0, -4000.0, 4000.0));
                    let corners = [
                        (min_x, min_y),
                        (max_x, min_y),
                        (max_x, max_y),
                        (min_x, max_y),
                    ];
                    let sec_pts: Vec<Pos2> = corners
                        .iter()
                        .filter_map(|(x, y)| proj_mm(*x, *y, sec_elev))
                        .collect();
                    if sec_pts.len() == 4 {
                        let label_pos = sec_pts[1];
                        clip_painter.add(egui::Shape::convex_polygon(
                            sec_pts,
                            Color32::from_rgba_unmultiplied(56, 189, 248, 28),
                            Stroke::new(1.6, Color32::from_rgba_unmultiplied(245, 158, 11, 215)),
                        ));
                        clip_painter.text(
                            label_pos + Vec2::new(4.0, -4.0),
                            Align2::LEFT_BOTTOM,
                            format!("SECTION CUT Z = {sec_elev:.1}m"),
                            egui::FontId::monospace(10.0),
                            Color32::from_rgb(251, 191, 36),
                        );
                    }
                }

                // 6F. Live 3D Wireframe Ghost Preview on Earth 3D Surface
                if self.earth_bim_author.active_tool != EarthBimTool::None {
                    if let Some(snap_pt) = self.earth_bim_author.hover_snap_mm {
                        let base_m = bim_project.level_elevation(self.earth_bim_author.base_level_id) * 0.001;
                        let top_m = bim_project.level_elevation(self.earth_bim_author.top_level_id) * 0.001;
                        let top_z = if (top_m - base_m).abs() < 0.2 { base_m + 3.5 } else { top_m };
                        let ghost_col = Color32::from_rgb(0, 240, 255);
                        let ghost_stroke = Stroke::new(1.8, ghost_col);

                        if let Some(sp) = proj_mm(snap_pt[0], snap_pt[1], base_m) {
                            clip_painter.circle_stroke(sp, 6.5, ghost_stroke);
                            clip_painter.circle_filled(sp, 2.5, ghost_col);
                        }

                        match self.earth_bim_author.active_tool {
                            EarthBimTool::Column => {
                                let hw = self.earth_bim_author.col_width_mm * 0.5;
                                let hd = self.earth_bim_author.col_depth_mm * 0.5;
                                let corners = [
                                    (snap_pt[0] - hw, snap_pt[1] - hd),
                                    (snap_pt[0] + hw, snap_pt[1] - hd),
                                    (snap_pt[0] + hw, snap_pt[1] + hd),
                                    (snap_pt[0] - hw, snap_pt[1] + hd),
                                ];
                                let b_pts: Vec<Pos2> = corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, base_m)).collect();
                                let t_pts: Vec<Pos2> = corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, top_z)).collect();
                                if b_pts.len() == 4 && t_pts.len() == 4 {
                                    for i in 0..4 {
                                        let ni = (i + 1) % 4;
                                        clip_painter.line_segment([b_pts[i], b_pts[ni]], ghost_stroke);
                                        clip_painter.line_segment([t_pts[i], t_pts[ni]], ghost_stroke);
                                        clip_painter.line_segment([b_pts[i], t_pts[i]], ghost_stroke);
                                    }
                                }
                            }
                            EarthBimTool::Footing => {
                                let hs = self.earth_bim_author.footing_size_mm * 0.5;
                                let corners = [
                                    (snap_pt[0] - hs, snap_pt[1] - hs),
                                    (snap_pt[0] + hs, snap_pt[1] - hs),
                                    (snap_pt[0] + hs, snap_pt[1] + hs),
                                    (snap_pt[0] - hs, snap_pt[1] + hs),
                                ];
                                let pts: Vec<Pos2> = corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, base_m)).collect();
                                if pts.len() == 4 {
                                    clip_painter.add(egui::Shape::convex_polygon(
                                        pts,
                                        Color32::from_rgba_unmultiplied(0, 240, 255, 40),
                                        ghost_stroke,
                                    ));
                                }
                            }
                            EarthBimTool::Wall | EarthBimTool::Beam => {
                                if let Some(s) = self.earth_bim_author.pending_start_mm {
                                    let z_draw = if self.earth_bim_author.active_tool == EarthBimTool::Beam {
                                        top_z
                                    } else {
                                        base_m
                                    };
                                    if let (Some(p0), Some(p1)) = (
                                        proj_mm(s[0], s[1], z_draw),
                                        proj_mm(snap_pt[0], snap_pt[1], z_draw),
                                    ) {
                                        clip_painter.line_segment([p0, p1], Stroke::new(2.5, ghost_col));
                                        if self.earth_bim_author.active_tool == EarthBimTool::Wall {
                                            if let (Some(t0), Some(t1)) = (
                                                proj_mm(s[0], s[1], top_z),
                                                proj_mm(snap_pt[0], snap_pt[1], top_z),
                                            ) {
                                                clip_painter.add(egui::Shape::convex_polygon(
                                                    vec![p0, p1, t1, t0],
                                                    Color32::from_rgba_unmultiplied(0, 240, 255, 42),
                                                    ghost_stroke,
                                                ));
                                            }
                                        }
                                        let dx_m = (snap_pt[0] - s[0]) * 0.001;
                                        let dy_m = (snap_pt[1] - s[1]) * 0.001;
                                        let len_m = dx_m.hypot(dy_m);
                                        let ang_deg = dy_m.atan2(dx_m).to_degrees();
                                        let mid = Pos2::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5 - 10.0);
                                        clip_painter.text(
                                            mid,
                                            Align2::CENTER_BOTTOM,
                                            format!("{len_m:.2} m @ {ang_deg:.0}°"),
                                            egui::FontId::monospace(10.5),
                                            ghost_col,
                                        );
                                    }
                                }
                            }
                            EarthBimTool::Slab => {
                                if let Some(s) = self.earth_bim_author.pending_start_mm {
                                    let min_x = s[0].min(snap_pt[0]);
                                    let max_x = s[0].max(snap_pt[0]);
                                    let min_y = s[1].min(snap_pt[1]);
                                    let max_y = s[1].max(snap_pt[1]);
                                    let corners = [
                                        (min_x, min_y),
                                        (max_x, min_y),
                                        (max_x, max_y),
                                        (min_x, max_y),
                                    ];
                                    let pts: Vec<Pos2> = corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, base_m)).collect();
                                    if pts.len() == 4 {
                                        clip_painter.add(egui::Shape::convex_polygon(
                                            pts,
                                            Color32::from_rgba_unmultiplied(0, 240, 255, 48),
                                            ghost_stroke,
                                        ));
                                    }
                                    if let Some(p_lbl) = proj_mm(0.5 * (min_x + max_x), 0.5 * (min_y + max_y), base_m) {
                                        clip_painter.text(
                                            p_lbl,
                                            Align2::CENTER_CENTER,
                                            format!("{:.2} x {:.2} m", (max_x - min_x) * 0.001, (max_y - min_y) * 0.001),
                                            egui::FontId::monospace(10.5),
                                            ghost_col,
                                        );
                                    }
                                }
                            }
                            EarthBimTool::Door | EarthBimTool::Window => {
                                let is_door = self.earth_bim_author.active_tool == EarthBimTool::Door;
                                if let Some(w) = bim_project
                                    .walls
                                    .iter()
                                    .filter(|w| show_on_story(w.base_level_id))
                                    .min_by(|a, b| {
                                        let dist_w = |wl: &crate::core::BimWall| -> f64 {
                                            let ab_x = wl.end.0 - wl.start.0;
                                            let ab_y = wl.end.1 - wl.start.1;
                                            let l2 = (ab_x * ab_x + ab_y * ab_y).max(1.0);
                                            let t = (((snap_pt[0] - wl.start.0) * ab_x + (snap_pt[1] - wl.start.1) * ab_y) / l2).clamp(0.0, 1.0);
                                            ((snap_pt[0] - (wl.start.0 + t * ab_x)).powi(2)
                                                + (snap_pt[1] - (wl.start.1 + t * ab_y)).powi(2))
                                            .sqrt()
                                        };
                                        dist_w(a).partial_cmp(&dist_w(b)).unwrap_or(std::cmp::Ordering::Equal)
                                    })
                                {
                                    let wlen = w.start.dist(w.end).max(400.0);
                                    let ux = (w.end.0 - w.start.0) / wlen;
                                    let uy = (w.end.1 - w.start.1) / wlen;
                                    let proj_along = (snap_pt[0] - w.start.0) * ux + (snap_pt[1] - w.start.1) * uy;
                                    let (ow, oh, osill) = if is_door { (900.0, 2100.0, 0.0) } else { (1800.0, 1400.0, 900.0) };
                                    let off = (proj_along - ow * 0.5).clamp(120.0, (wlen - ow - 120.0).max(120.0));
                                    let u0 = (off / wlen).clamp(0.0, 1.0);
                                    let u1 = ((off + ow) / wlen).clamp(0.0, 1.0);
                                    let wz0 = bim_project.level_elevation(w.base_level_id) * 0.001 + osill * 0.001;
                                    let wz1 = wz0 + oh * 0.001;
                                    if let (Some(p0), Some(p1), Some(p2), Some(p3)) = (
                                        proj_mm(w.start.0 + (w.end.0 - w.start.0) * u0, w.start.1 + (w.end.1 - w.start.1) * u0, wz0),
                                        proj_mm(w.start.0 + (w.end.0 - w.start.0) * u1, w.start.1 + (w.end.1 - w.start.1) * u1, wz0),
                                        proj_mm(w.start.0 + (w.end.0 - w.start.0) * u1, w.start.1 + (w.end.1 - w.start.1) * u1, wz1),
                                        proj_mm(w.start.0 + (w.end.0 - w.start.0) * u0, w.start.1 + (w.end.1 - w.start.1) * u0, wz1),
                                    ) {
                                        clip_painter.add(egui::Shape::convex_polygon(
                                            vec![p0, p1, p2, p3],
                                            Color32::from_rgba_unmultiplied(0, 240, 255, 90),
                                            ghost_stroke,
                                        ));
                                    }
                                }
                            }
                            EarthBimTool::Road | EarthBimTool::Bridge => {
                                let is_bridge = self.earth_bim_author.active_tool == EarthBimTool::Bridge;
                                let z_draw = if is_bridge { 6.5 } else { 0.16 };
                                if let Some(s) = self.earth_bim_author.pending_start_mm {
                                    if let (Some(p0), Some(p1)) = (
                                        proj_mm(s[0], s[1], z_draw),
                                        proj_mm(snap_pt[0], snap_pt[1], z_draw),
                                    ) {
                                        let len_m = ((snap_pt[0] - s[0]).hypot(snap_pt[1] - s[1])) * 0.001;
                                        clip_painter.line_segment([p0, p1], Stroke::new(3.2, ghost_col));
                                        let mid = Pos2::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5 - 10.0);
                                        clip_painter.text(
                                            mid,
                                            Align2::CENTER_BOTTOM,
                                            format!(
                                                "{} {:.1} m",
                                                if is_bridge { "IfcBridge" } else { "IfcRoad" },
                                                len_m
                                            ),
                                            egui::FontId::monospace(10.5),
                                            ghost_col,
                                        );
                                    }
                                }
                            }
                            EarthBimTool::Zone => {
                                let hw = 15_000.0;
                                let hd = 12_000.0;
                                let corners = [
                                    (snap_pt[0] - hw, snap_pt[1] - hd),
                                    (snap_pt[0] + hw, snap_pt[1] - hd),
                                    (snap_pt[0] + hw, snap_pt[1] + hd),
                                    (snap_pt[0] - hw, snap_pt[1] + hd),
                                ];
                                let pts: Vec<Pos2> =
                                    corners.iter().filter_map(|(x, y)| proj_mm(*x, *y, 0.18)).collect();
                                if pts.len() == 4 {
                                    clip_painter.add(egui::Shape::convex_polygon(
                                        pts,
                                        Color32::from_rgba_unmultiplied(64, 210, 150, 48),
                                        ghost_stroke,
                                    ));
                                }
                            }
                            EarthBimTool::None => {}
                        }
                    }
                }
            }
        }
    }

    /// Renders all visible Vault Geo-BIM projects simultaneously on Earth 3D across LOD 0/1/2
    /// plus interactive cursor-tracked ghost footprint when placing an unlocated model.
    pub fn paint_planetary_world_buildings(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
    ) {
        if self.world_index.buildings.is_empty() && self.placing_unlocated_rel_path.is_none() {
            return;
        }
        let Some(ref xf) = self.last_globe_xform else {
            return;
        };
        let clip_painter = painter.with_clip_rect(rect);
        let (min_lat, max_lat, min_lon, max_lon) = self.visible_geo_bbox(xf);

        // LOD 0: Continental / Regional Cluster Beacons (`zoom < 8.5`)
        if self.world_map.zoom_level < 8.5 {
            let clusters = self.world_index.query_clusters(
                min_lat,
                max_lat,
                min_lon,
                max_lon,
                self.world_map.zoom_level,
            );
            for cl in clusters.into_iter().take(120) {
                let Some((sp, _, vis)) = xf.project_lat_lon_elev(cl.lat, cl.lon, 0.0) else {
                    continue;
                };
                if !vis || !rect.expand(36.0).contains(sp) {
                    continue;
                }
                let is_staging = cl.unlocated_count > 0 && cl.unlocated_count == cl.count;
                let col = if is_staging {
                    theme.accent_orange
                } else if cl.min_eigengate < 0.72 || cl.max_dcr >= 0.90 {
                    theme.accent_red
                } else {
                    theme.accent_blue
                };
                let r = if cl.count > 100 {
                    15.0
                } else if cl.count > 10 {
                    12.5
                } else {
                    10.0
                };
                clip_painter.circle_filled(sp, r + 3.0, col.gamma_multiply(0.28));
                clip_painter.circle_filled(sp, r, col);
                clip_painter.circle_stroke(sp, r, Stroke::new(1.5, Color32::WHITE));
                let num_lbl = if cl.count >= 1000 {
                    format!("{:.1}k", cl.count as f64 / 1000.0)
                } else {
                    format!("{}", cl.count)
                };
                clip_painter.text(
                    sp,
                    Align2::CENTER_CENTER,
                    num_lbl,
                    egui::FontId::proportional(9.5),
                    Color32::WHITE,
                );
                if r >= 11.0 || self.world_map.zoom_level >= 5.0 {
                    clip_painter.text(
                        sp + Vec2::new(0.0, r + 4.0),
                        Align2::CENTER_TOP,
                        &cl.label,
                        egui::FontId::proportional(9.5),
                        Color32::WHITE,
                    );
                }
            }
        } else {
            // LOD 1 / 2: Simultaneous 2.5D / 3D City Extruded Massing across visible neighbor buildings
            let lift_scale = if self.world_map.zoom_level < 14.5 { 6.0 } else { 1.0 };
            let visible = self.world_index.query_visible_indices(
                min_lat,
                max_lat,
                min_lon,
                max_lon,
                xf.center_lat_deg,
                xf.center_lon_deg,
                450,
            );
            for idx in visible {
                let b = &self.world_index.buildings[idx];
                if self.active_world_project_rel_path.as_deref() == Some(b.rel_path.as_str()) {
                    continue;
                }
                let is_hovered =
                    self.hovered_world_building_rel_path.as_deref() == Some(b.rel_path.as_str());
                // When WebGPU (WGSL) GPU Z-buffered extrusion is active at street scale, only paint hover highlight & badge on CPU
                if self.webgpu_earth.is_active() && self.world_map.zoom_level >= 14.5 && !is_hovered {
                    continue;
                }
                let hull_mm = b.rotated_footprint_world_mm();
                let mut base_pts = Vec::with_capacity(hull_mm.len());
                let mut roof_pts = Vec::with_capacity(hull_mm.len());
                let base_elev = self.local_site_relief_m(b.lat, b.lon);
                let top_elev = base_elev + b.total_height_m * lift_scale;
                let mut ok = true;
                for pt_mm in &hull_mm {
                    let ll = SlippyMapState::world_mm_to_lat_lon((b.lat, b.lon), *pt_mm);
                    if let (Some((bp, _, v0)), Some((tp, _, v1))) = (
                        xf.project_lat_lon_elev(ll.0, ll.1, base_elev),
                        xf.project_lat_lon_elev(ll.0, ll.1, top_elev),
                    ) {
                        if !v0 || !v1 {
                            ok = false;
                            break;
                        }
                        base_pts.push(bp);
                        roof_pts.push(tp);
                    } else {
                        ok = false;
                        break;
                    }
                }
                if !ok || roof_pts.len() < 3 {
                    continue;
                }
                if !roof_pts.iter().any(|p| rect.expand(45.0).contains(*p))
                    && !base_pts.iter().any(|p| rect.expand(45.0).contains(*p))
                {
                    continue;
                }

                let base_col = if b.is_unlocated() {
                    Color32::from_rgb(245, 158, 11)
                } else {
                    match self.earth_bim_author.active_lens {
                        EarthBimLens::Clean => Color32::from_rgb(96, 165, 250),
                        EarthBimLens::DcrHeatmap => {
                            if b.max_dcr >= 0.85 {
                                theme.accent_red
                            } else if b.max_dcr >= 0.65 {
                                theme.accent_orange
                            } else {
                                theme.accent_green
                            }
                        }
                        EarthBimLens::EigenGateRedundancy => {
                            if b.min_eigengate < 0.70 {
                                theme.accent_red
                            } else if b.min_eigengate < 0.85 {
                                theme.accent_orange
                            } else {
                                Color32::from_rgb(16, 185, 129)
                            }
                        }
                        EarthBimLens::MomentStress => Color32::from_rgb(129, 140, 248),
                        EarthBimLens::DeformedShape => Color32::from_rgb(245, 158, 11),
                        EarthBimLens::ModalVibration => Color32::from_rgb(91, 146, 165),
                        EarthBimLens::TimeHistoryReplay => Color32::from_rgb(245, 158, 11),
                        EarthBimLens::ShmDigitalTwin => {
                            if let Some(ref shm) = self.last_shm_twin_report {
                                if let Some(dp) = shm
                                    .district_placards
                                    .iter()
                                    .find(|d| d.rel_path == b.rel_path)
                                {
                                    let [r, g, bl] = dp.placard.badge_color_rgb();
                                    Color32::from_rgb(r, g, bl)
                                } else {
                                    Color32::from_rgb(34, 197, 94)
                                }
                            } else {
                                Color32::from_rgb(34, 197, 94)
                            }
                        }
                        EarthBimLens::Erection4D => Color32::from_rgb(56, 189, 248),
                    }
                };
                let roof_fill = if is_hovered {
                    base_col.gamma_multiply(0.85)
                } else {
                    Self::directional_face_shade(base_col, roof_pts[0], roof_pts[1], true)
                };
                let edge_stroke = if is_hovered {
                    Stroke::new(2.0, Color32::from_rgb(0, 240, 255))
                } else {
                    Stroke::new(1.1, base_col.gamma_multiply(0.95))
                };
                for i in 0..base_pts.len() {
                    let ni = (i + 1) % base_pts.len();
                    let side_fill = if is_hovered {
                        base_col.gamma_multiply(0.62)
                    } else {
                        Self::directional_face_shade(base_col, base_pts[i], base_pts[ni], false)
                    };
                    clip_painter.add(egui::Shape::convex_polygon(
                        vec![base_pts[i], base_pts[ni], roof_pts[ni], roof_pts[i]],
                        side_fill,
                        edge_stroke,
                    ));
                }
                clip_painter.add(egui::Shape::convex_polygon(
                    roof_pts.clone(),
                    roof_fill,
                    Stroke::new(
                        if is_hovered { 2.2 } else { 1.4 },
                        if is_hovered {
                            Color32::from_rgb(0, 240, 255)
                        } else {
                            Color32::WHITE.gamma_multiply(0.75)
                        },
                    ),
                ));

                if is_hovered {
                    let rc = roof_pts
                        .iter()
                        .fold(Vec2::ZERO, |acc, p| acc + p.to_vec2())
                        / (roof_pts.len() as f32);
                    let city_tag = if b.address_hint.is_empty() {
                        b.source.short_label().to_string()
                    } else {
                        b.address_hint.clone()
                    };
                    let tip = format!(
                        "{} • {} • {}F • DCR {:.0}% • EigenGate {:.2} [Click to Edit]",
                        b.name,
                        city_tag,
                        b.story_count,
                        b.max_dcr * 100.0,
                        b.min_eigengate
                    );
                    let tip_pos = Pos2::new(rc.x, rc.y - 12.0);
                    let galley = clip_painter.layout_no_wrap(
                        tip,
                        egui::FontId::proportional(10.0),
                        Color32::WHITE,
                    );
                    let pill_rect = Rect::from_center_size(
                        tip_pos - Vec2::new(0.0, galley.size().y * 0.5),
                        galley.size() + Vec2::new(14.0, 8.0),
                    );
                    clip_painter.rect_filled(
                        pill_rect,
                        CornerRadius::same(7),
                        Color32::from_rgba_unmultiplied(15, 23, 42, 232),
                    );
                    clip_painter.rect_stroke(
                        pill_rect,
                        CornerRadius::same(7),
                        Stroke::new(1.1, Color32::from_rgb(0, 240, 255)),
                        egui::StrokeKind::Inside,
                    );
                    clip_painter.galley(
                        pill_rect.min + Vec2::new(7.0, 4.0),
                        galley,
                        Color32::WHITE,
                    );
                } else if self.world_map.zoom_level >= 14.8 {
                    let rc = roof_pts
                        .iter()
                        .fold(Vec2::ZERO, |acc, p| acc + p.to_vec2())
                        / (roof_pts.len() as f32);
                    let lbl = if b.is_unlocated() {
                        format!("{} [Staging]", b.name)
                    } else {
                        b.name.clone()
                    };
                    clip_painter.text(
                        Pos2::new(rc.x, rc.y - 6.0),
                        Align2::CENTER_BOTTOM,
                        lbl,
                        egui::FontId::proportional(9.5),
                        Color32::WHITE,
                    );
                }
            }
        }

        // Interactive "Place on Earth" translucent ghost footprint following cursor
        if let (Some(ref rel), Some((hlat, hlon))) =
            (&self.placing_unlocated_rel_path, self.placing_hover_lat_lon)
        {
            let (hull_m, height_m) = if let Some(b) = self.world_index.get(rel) {
                (b.footprint_local_m.clone(), b.total_height_m)
            } else {
                (crate::core::gis::WorldBuildingProxy::default_footprint_m(), 14.0)
            };
            let base_relief = self.local_site_relief_m(hlat, hlon);
            let msl_elev = self.earth_world.effective_elevation_m(hlat, hlon);
            let rad = self.placing_heading_deg.to_radians();
            let (sh, ch) = (rad.sin(), rad.cos());
            let mut bpts = Vec::with_capacity(hull_m.len());
            let mut rpts = Vec::with_capacity(hull_m.len());
            for [xm, ym] in &hull_m {
                let rx_mm = (xm * ch - ym * sh) * 1000.0;
                let ry_mm = (xm * sh + ym * ch) * 1000.0;
                let ll = SlippyMapState::world_mm_to_lat_lon((hlat, hlon), [rx_mm, ry_mm]);
                if let (Some((bp, _, true)), Some((rp, _, true))) = (
                    xf.project_lat_lon_elev(ll.0, ll.1, base_relief),
                    xf.project_lat_lon_elev(ll.0, ll.1, base_relief + height_m),
                ) {
                    bpts.push(bp);
                    rpts.push(rp);
                }
            }
            if bpts.len() >= 3 && rpts.len() == bpts.len() {
                let ghost_stroke = Stroke::new(1.8, Color32::from_rgb(0, 240, 255));
                for i in 0..bpts.len() {
                    let ni = (i + 1) % bpts.len();
                    clip_painter.add(egui::Shape::convex_polygon(
                        vec![bpts[i], bpts[ni], rpts[ni], rpts[i]],
                        Color32::from_rgba_unmultiplied(0, 240, 255, 50),
                        ghost_stroke,
                    ));
                }
                clip_painter.add(egui::Shape::convex_polygon(
                    rpts.clone(),
                    Color32::from_rgba_unmultiplied(0, 240, 255, 85),
                    ghost_stroke,
                ));
                if let Some((cp, _, true)) = xf.project_lat_lon_elev(hlat, hlon, base_relief + height_m + 1.5) {
                    let snap_note = self
                        .active_vector_snap_az_deg
                        .map(|az| format!(" • Snap {az:.0}° [A]"))
                        .unwrap_or_default();
                    clip_painter.text(
                        cp,
                        Align2::CENTER_BOTTOM,
                        format!(
                            "Place Here ({hlat:.4}°, {hlon:.4}° | Hdg {:.0}° | {:+.0}m MSL{snap_note})",
                            self.placing_heading_deg,
                            msl_elev
                        ),
                        egui::FontId::monospace(10.0),
                        Color32::from_rgb(0, 240, 255),
                    );
                }
            }
        }
    }

    /// Renders inter-building seismic pounding separation gaps, Venturi wind corridors, and
    /// collapse-risk proximity rings directly over neighboring structures on Earth 3D.
    pub fn paint_neighborhood_hazards(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
    ) {
        if self.world_map.zoom_level < 14.2 || self.neighborhood_report.contacts.is_empty() {
            return;
        }
        let Some(ref xf) = self.last_globe_xform else {
            return;
        };
        let clip_painter = painter.with_clip_rect(rect);
        for contact in &self.neighborhood_report.contacts {
            let elev = self.local_site_relief_m(contact.neighbor_lat, contact.neighbor_lon) + 2.0;
            let (Some((pa, _, true)), Some((pb, _, true))) = (
                xf.project_lat_lon_elev(contact.closest_active_ll.0, contact.closest_active_ll.1, elev),
                xf.project_lat_lon_elev(contact.closest_neighbor_ll.0, contact.closest_neighbor_ll.1, elev),
            ) else {
                continue;
            };
            if !rect.expand(30.0).contains(pa) && !rect.expand(30.0).contains(pb) {
                continue;
            }
            use crate::core::gis::PoundingSeverity;
            let col = match contact.severity {
                PoundingSeverity::PoundingRisk => theme.accent_red,
                PoundingSeverity::TightBuffer => theme.accent_orange,
                PoundingSeverity::SafeClearance if contact.venturi_channel => theme.accent_blue,
                PoundingSeverity::SafeClearance => continue,
            };
            clip_painter.line_segment([pa, pb], Stroke::new(2.0, col));
            clip_painter.circle_filled(pa, 3.0, col);
            clip_painter.circle_filled(pb, 3.0, col);
            let mid = Pos2::new(0.5 * (pa.x + pb.x), 0.5 * (pa.y + pb.y) - 6.0);
            let tag = match contact.severity {
                PoundingSeverity::PoundingRisk => format!(
                    "Gap {:.2}m < {:.2}m [POUNDING]",
                    contact.clear_gap_m, contact.required_gap_m
                ),
                PoundingSeverity::TightBuffer => {
                    format!("Gap {:.2}m [Tight]", contact.clear_gap_m)
                }
                PoundingSeverity::SafeClearance => {
                    format!("Wind Venturi ({:.1}m)", contact.clear_gap_m)
                }
            };
            clip_painter.text(
                mid,
                Align2::CENTER_BOTTOM,
                tag,
                egui::FontId::monospace(9.5),
                col,
            );
            if contact.inside_collapse_fall_zone {
                if let Some((nc, _, true)) =
                    xf.project_lat_lon_elev(contact.neighbor_lat, contact.neighbor_lon, elev + contact.neighbor_height_m * 0.5)
                {
                    clip_painter.text(
                        nc,
                        Align2::CENTER_CENTER,
                        "FALL ZONE",
                        egui::FontId::monospace(9.0),
                        theme.accent_red,
                    );
                }
            }
        }
    }

    /// Pure-vector CPU fallback rendering for district seismic waves, Venturi wind streamlines,
    /// and 3D flood plane when WebGPU (WGSL) hardware acceleration is inactive (e.g. headless CI).
    pub(super) fn paint_cpu_district_hazard_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if self.world_map.zoom_level < 12.5 {
            return;
        }
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };

        if self.district_hazard.seismic_wave_active {
            for r_m in [35.0_f64, 70.0, 110.0] {
                let mut ring_pts = Vec::with_capacity(25);
                for k in 0..=24 {
                    let a = (k as f64 / 24.0) * std::f64::consts::TAU;
                    let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                        origin,
                        [r_m * a.cos() * 1000.0, r_m * a.sin() * 1000.0],
                    );
                    if let Some((sp, _, true)) = xform.project_lat_lon_elev(ll.0, ll.1, 0.25) {
                        ring_pts.push(sp);
                    }
                }
                if ring_pts.len() >= 4 {
                    clip.add(egui::Shape::line(
                        ring_pts,
                        Stroke::new(1.4, theme.accent_orange.gamma_multiply(0.65)),
                    ));
                }
            }
        }

        if self.district_hazard.venturi_wind_active {
            for seg in &self.district_hazard_report.streamlines {
                let ll0 = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                    origin,
                    [seg.start_enu_m[0] * 1000.0, seg.start_enu_m[1] * 1000.0],
                );
                let ll1 = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                    origin,
                    [seg.end_enu_m[0] * 1000.0, seg.end_enu_m[1] * 1000.0],
                );
                if let (Some((p0, _, true)), Some((p1, _, true))) = (
                    xform.project_lat_lon_elev(ll0.0, ll0.1, seg.start_enu_m[2]),
                    xform.project_lat_lon_elev(ll1.0, ll1.1, seg.end_enu_m[2]),
                ) {
                    let col = if seg.is_corridor_accelerated {
                        Color32::from_rgba_unmultiplied(56, 224, 255, 200)
                    } else {
                        theme.accent_blue.gamma_multiply(0.55)
                    };
                    clip.line_segment([p0, p1], Stroke::new(1.6, col));
                }
            }
        }

        if self.district_hazard.flood_plane_active {
            let wl = self.district_hazard.flood_water_level_m;
            let corners = [
                [-140.0_f64, -140.0],
                [140.0, -140.0],
                [140.0, 140.0],
                [-140.0, 140.0],
            ];
            let mut poly = Vec::with_capacity(4);
            for [ex, ny] in corners {
                let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                    origin,
                    [ex * 1000.0, ny * 1000.0],
                );
                if let Some((sp, _, true)) = xform.project_lat_lon_elev(ll.0, ll.1, wl) {
                    poly.push(sp);
                }
            }
            if poly.len() == 4 {
                clip.add(egui::Shape::convex_polygon(
                    poly,
                    Color32::from_rgba_unmultiplied(30, 122, 188, 72),
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 210, 255, 140)),
                ));
            }
        }
    }

    /// Pure-CPU translucent 3D Subsurface Soil Horizons, Borehole Depth Column, and Groundwater
    /// Phreatic Surface fallback when WebGPU is inactive and X-Ray subsurface mode is enabled.
    pub(super) fn paint_cpu_subsurface_ssi_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        xform: &crate::core::gis::GlobeScreenTransform,
        bim_project: &BimProject,
    ) {
        if self.world_map.zoom_level < 12.5 || !self.subsurface_ssi.xray_subsurface_visible {
            return;
        }
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };

        let strata = if let Some(ref rep) = self.last_subsurface_ssi_report {
            rep.strata.clone()
        } else {
            crate::core::geotech::GeotechEngine::build_borehole_strata(
                self.subsurface_ssi.borehole_preset,
                self.subsurface_ssi.groundwater_depth_m,
                0.35,
                self.subsurface_ssi.liquefaction_trigger_active,
            )
            .0
        };

        let (min_x, max_x, min_y, max_y) = if let Some((bmin, bmax)) = bim_project.plan_extents_mm() {
            (
                bmin[0] * 0.001 - 6.0,
                bmax[0] * 0.001 + 6.0,
                bmin[1] * 0.001 - 6.0,
                bmax[1] * 0.001 + 6.0,
            )
        } else {
            (-12.0, 12.0, -12.0, 12.0)
        };
        let corners = [
            [min_x, min_y],
            [max_x, min_y],
            [max_x, max_y],
            [min_x, max_y],
        ];
        let project_corner = |ex: f64, ny: f64, elev_m: f64| -> Option<Pos2> {
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, elev_m)
                .and_then(|(sp, _, front)| front.then_some(sp))
        };

        for s in strata.iter().rev() {
            let mut poly = Vec::with_capacity(4);
            for [ex, ny] in corners {
                if let Some(sp) = project_corner(ex, ny, -s.depth_bottom_m) {
                    poly.push(sp);
                }
            }
            if poly.len() == 4 {
                let is_liq = self.subsurface_ssi.liquefaction_trigger_active
                    && s.liquefiable
                    && s.liquefaction_fs < 1.15;
                let fill = if is_liq {
                    Color32::from_rgba_unmultiplied(242, 86, 42, 72)
                } else {
                    let [r, g, b, _] = s.color_rgba;
                    Color32::from_rgba_unmultiplied(
                        (r * 255.0).round() as u8,
                        (g * 255.0).round() as u8,
                        (b * 255.0).round() as u8,
                        48,
                    )
                };
                let stroke_col = if is_liq {
                    theme.accent_red.gamma_multiply(0.80)
                } else {
                    theme.border_card.gamma_multiply(0.55)
                };
                clip.add(egui::Shape::convex_polygon(
                    poly,
                    fill,
                    Stroke::new(1.0, stroke_col),
                ));
            }
        }

        let gwt_z = -self.subsurface_ssi.groundwater_depth_m.clamp(0.0, 28.0);
        let mut gwt_poly = Vec::with_capacity(4);
        for [ex, ny] in corners {
            if let Some(sp) = project_corner(ex, ny, gwt_z) {
                gwt_poly.push(sp);
            }
        }
        if gwt_poly.len() == 4 {
            clip.add(egui::Shape::convex_polygon(
                gwt_poly,
                Color32::from_rgba_unmultiplied(40, 194, 245, 76),
                Stroke::new(1.4, Color32::from_rgba_unmultiplied(110, 228, 255, 195)),
            ));
        }

        if let (Some(p_top), Some(p_bot)) = (
            project_corner(max_x - 1.5, max_y - 1.5, 0.5),
            project_corner(max_x - 1.5, max_y - 1.5, -30.0),
        ) {
            clip.line_segment(
                [p_top, p_bot],
                Stroke::new(2.2, Color32::from_rgba_unmultiplied(82, 214, 224, 215)),
            );
        }
    }

    /// Pure-CPU translucent crimson wireframe ghost fallback for deleted `.ifcx` nodes when
    /// WebGPU is inactive and Multi-Layer IFCX Visual Diff overlay is active.
    pub(super) fn paint_cpu_ifcx_diff_ghosts_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.ifcx_diff_overlay_active || self.world_map.zoom_level < 12.5 {
            return;
        }
        let Some(ref diff) = self.last_ifcx_visual_diff else {
            return;
        };
        if diff.deleted_ghost_boxes_m.is_empty() {
            return;
        }
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let proj_pt = |ex: f64, ny: f64, elev_m: f64| -> Option<Pos2> {
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, elev_m)
                .and_then(|(sp, _, front)| front.then_some(sp))
        };
        for (bmin, bmax, _) in &diff.deleted_ghost_boxes_m {
            let corners = [
                [bmin[0], bmin[1]],
                [bmax[0], bmin[1]],
                [bmax[0], bmax[1]],
                [bmin[0], bmax[1]],
            ];
            let mut poly = Vec::with_capacity(4);
            for [ex, ny] in corners {
                if let Some(sp) = proj_pt(ex, ny, bmax[2]) {
                    poly.push(sp);
                }
            }
            if poly.len() == 4 {
                clip.add(egui::Shape::convex_polygon(
                    poly,
                    Color32::from_rgba_unmultiplied(245, 56, 72, 68),
                    Stroke::new(1.5, theme.accent_red),
                ));
            }
        }
    }

    /// Pure-CPU translucent district portfolio retrofit crown fallback when WebGPU is inactive.
    pub(super) fn paint_cpu_portfolio_retrofit_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.portfolio_retrofit_config.overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref rep) = self.last_portfolio_retrofit_report else {
            return;
        };
        let Some(pt) = rep.active_frontier() else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        for d in &pt.decisions {
            let off_mm =
                crate::core::gis::SlippyMapState::lat_lon_to_world_mm(origin, d.lat, d.lon);
            let cx_m = off_mm[0] * 0.001;
            let cy_m = off_mm[1] * 0.001;
            if cx_m.hypot(cy_m) > 15_000.0 || d.footprint_local_m.len() < 3 {
                continue;
            }
            let rad = d.heading_deg.to_radians();
            let (sr, cr) = (rad.sin(), rad.cos());
            let mut poly = Vec::with_capacity(d.footprint_local_m.len());
            for &[lx, ly] in &d.footprint_local_m {
                let wx = cx_m + lx * cr - ly * sr;
                let wy = cy_m + lx * sr + ly * cr;
                let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                    origin,
                    [wx * 1000.0, wy * 1000.0],
                );
                if let Some((sp, _, true)) = xform.project_lat_lon_elev(ll.0, ll.1, d.height_m) {
                    poly.push(sp);
                }
            }
            if poly.len() >= 3 {
                let (fill, stroke_col) = if d.selected_strategy.is_some() {
                    (
                        Color32::from_rgba_unmultiplied(30, 224, 158, 72),
                        theme.accent_green,
                    )
                } else if d.pre_min_eigengate < rep.config.target_min_eigengate || d.pre_max_dcr > 0.85 {
                    (
                        Color32::from_rgba_unmultiplied(240, 72, 62, 62),
                        theme.accent_red,
                    )
                } else {
                    continue;
                };
                clip.add(egui::Shape::convex_polygon(
                    poly,
                    fill,
                    Stroke::new(1.5, stroke_col),
                ));
            }
        }
    }

    /// Pure-vector CPU projection fallback for 3D ASCE 41-17 plastic hinge beacons and ground
    /// seismic pulse ring when WebGPU (WGSL) hardware acceleration is inactive.
    pub(super) fn paint_cpu_plastic_hinges_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        _theme: &MacTheme,
        xform: &crate::core::gis::GlobeScreenTransform,
        bim_project: &BimProject,
    ) {
        use crate::core::nonlinear::PlasticHingeState;

        if self.earth_bim_author.active_lens != EarthBimLens::TimeHistoryReplay
            || self.world_map.zoom_level < 14.0
        {
            return;
        }
        let Some(frame) = self.current_replay_frame() else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let lift_scale = if self.world_map.zoom_level < 14.5 { 8.0 } else { 1.0 };
        let vis = (self.earth_bim_author.deform_scale * 0.25).clamp(1.0, 28.0);
        let shake_x_mm = frame.ground_disp_m * vis * 0.35 * 1000.0;
        let shake_y_mm = frame.ground_disp_y_m * vis * 0.35 * 1000.0;
        let shake_z_m = frame.ground_disp_z_m * vis * 0.22;

        // Ground pulse ring around building centroid
        if let Some((bmin, bmax)) = bim_project.plan_extents_mm() {
            let cx = (bmin[0] + bmax[0]) * 0.5;
            let cy = (bmin[1] + bmax[1]) * 0.5;
            let r_m = ((bmax[0] - bmin[0]).hypot(bmax[1] - bmin[1]) * 0.00055).max(5.0)
                + (frame.ground_accel_g.abs() * 1.5);
            let mut ring_pts = Vec::with_capacity(24);
            for i in 0..24 {
                let th = (i as f64 / 24.0) * std::f64::consts::TAU;
                let px_mm = cx + r_m * 1000.0 * th.cos();
                let py_mm = cy + r_m * 1000.0 * th.sin();
                if let Some(sp) = self.project_active_bim_mm(xform, px_mm, py_mm, 0.14 + shake_z_m * 0.2, lift_scale) {
                    ring_pts.push(sp);
                }
            }
            if ring_pts.len() >= 12 {
                let col = if frame.ground_accel_g.abs() > 0.35 {
                    Color32::from_rgba_unmultiplied(255, 110, 64, 170)
                } else {
                    Color32::from_rgba_unmultiplied(91, 196, 225, 130)
                };
                clip.add(egui::Shape::closed_line(ring_pts, Stroke::new(2.0, col)));
            }
        }

        if !self.ground_motion_replay_config.show_plastic_hinges {
            return;
        }
        let n = frame
            .story_elevations_m
            .len()
            .min(frame.story_disp_x_m.len())
            .min(frame.story_disp_y_m.len());
        for h in &frame.active_hinges {
            let (col, rad) = match h.state {
                PlasticHingeState::Elastic => continue,
                PlasticHingeState::ImmediateOccupancy => (Color32::from_rgb(10, 132, 255), 5.0_f32),
                PlasticHingeState::LifeSafety => (Color32::from_rgb(255, 159, 10), 6.2_f32),
                PlasticHingeState::CollapsePrevention => (Color32::from_rgb(255, 69, 58), 7.5_f32),
                PlasticHingeState::Mechanism => (Color32::from_rgb(191, 90, 242), 8.8_f32),
            };
            let z_m = h.local_pos_m[2];
            let (mut dx_m, mut dy_m, mut dz_m) = (0.0, 0.0, 0.0);
            if n > 0 {
                dx_m = frame.story_disp_x_m[n - 1];
                dy_m = frame.story_disp_y_m[n - 1];
                dz_m = frame.story_disp_z_m.get(n - 1).copied().unwrap_or(0.0);
                if z_m <= frame.story_elevations_m[0] {
                    dx_m = frame.story_disp_x_m[0];
                    dy_m = frame.story_disp_y_m[0];
                    dz_m = frame.story_disp_z_m.first().copied().unwrap_or(0.0);
                } else {
                    for k in 1..n {
                        if z_m <= frame.story_elevations_m[k] {
                            let t = ((z_m - frame.story_elevations_m[k - 1])
                                / (frame.story_elevations_m[k] - frame.story_elevations_m[k - 1])
                                    .max(1e-4))
                            .clamp(0.0, 1.0);
                            dx_m = frame.story_disp_x_m[k - 1]
                                + (frame.story_disp_x_m[k] - frame.story_disp_x_m[k - 1]) * t;
                            dy_m = frame.story_disp_y_m[k - 1]
                                + (frame.story_disp_y_m[k] - frame.story_disp_y_m[k - 1]) * t;
                            let z0 = frame.story_disp_z_m.get(k - 1).copied().unwrap_or(0.0);
                            let z1 = frame.story_disp_z_m.get(k).copied().unwrap_or(0.0);
                            dz_m = z0 + (z1 - z0) * t;
                            break;
                        }
                    }
                }
            }
            let x_mm = h.local_pos_m[0] * 1000.0 + (dx_m * vis * 1000.0 + shake_x_mm).clamp(-2800.0, 2800.0);
            let y_mm = h.local_pos_m[1] * 1000.0 + (dy_m * vis * 1000.0 + shake_y_mm).clamp(-2800.0, 2800.0);
            let z_def_m = z_m + (dz_m * vis * 0.45 + shake_z_m).clamp(-1.8, 1.8);
            if let Some(sp) = self.project_active_bim_mm(xform, x_mm, y_mm, z_def_m, lift_scale) {
                clip.circle_filled(sp, rad, col.gamma_multiply(0.85));
                clip.circle_stroke(sp, rad + 1.5, Stroke::new(1.2, Color32::WHITE.gamma_multiply(0.7)));
            }
        }
    }

    /// Pure-vector CPU fallback rendering translucent FEMA P-58 per-story loss bands when WebGPU is inactive.
    pub(super) fn paint_cpu_fragility_loss_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        xform: &crate::core::gis::GlobeScreenTransform,
        bim_project: &BimProject,
    ) {
        if !self.ground_motion_replay_config.show_fragility_loss_overlay
            || self.world_map.zoom_level < 14.0
        {
            return;
        }
        let Some(ref rep) = self.last_ground_motion_replay_report else {
            return;
        };
        if rep.fragility_loss.story_losses.is_empty() || bim_project.is_empty() {
            return;
        }
        let (bmin, bmax) = bim_project
            .plan_extents_mm()
            .unwrap_or(([-6500.0, -5500.0], [6500.0, 5500.0]));
        let clip = painter.with_clip_rect(rect);
        let lift_scale = if self.world_map.zoom_level < 14.5 { 8.0 } else { 1.0 };
        let pad_mm = 750.0;
        let corners_mm = [
            [bmin[0] - pad_mm, bmin[1] - pad_mm],
            [bmax[0] + pad_mm, bmin[1] - pad_mm],
            [bmax[0] + pad_mm, bmax[1] + pad_mm],
            [bmin[0] - pad_mm, bmax[1] + pad_mm],
        ];
        for st in &rep.fragility_loss.story_losses {
            let stroke_col = if st.loss_ratio >= 0.15 {
                theme.accent_red
            } else if st.loss_ratio >= 0.05 {
                theme.accent_orange
            } else if st.loss_ratio >= 0.015 {
                Color32::from_rgb(245, 200, 62)
            } else {
                theme.accent_green
            };
            let fill_col = stroke_col.gamma_multiply(0.22);
            let mut poly = Vec::with_capacity(4);
            for [x_mm, y_mm] in corners_mm {
                if let Some(sp) =
                    self.project_active_bim_mm(xform, x_mm, y_mm, st.elevation_m, lift_scale)
                {
                    poly.push(sp);
                }
            }
            if poly.len() == 4 {
                clip.add(egui::Shape::convex_polygon(
                    poly,
                    fill_col,
                    Stroke::new(1.5, stroke_col.gamma_multiply(0.85)),
                ));
            }
        }
    }

    /// Pure-vector CPU fallback rendering SHM sensors & rooftop ATC-20 placard shield when WebGPU is inactive.
    pub(super) fn paint_cpu_shm_triage_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
        bim_project: &BimProject,
    ) {
        if self.earth_bim_author.active_lens != EarthBimLens::ShmDigitalTwin
            || self.world_map.zoom_level < 14.0
            || bim_project.is_empty()
        {
            return;
        }
        let Some(ref shm) = self.last_shm_twin_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let lift_scale = if self.world_map.zoom_level < 14.5 { 8.0 } else { 1.0 };
        for s in &shm.sensors {
            let [ex, ny, ez] = s.position_local_m;
            if let Some(sp) =
                self.project_active_bim_mm(xform, ex * 1000.0, ny * 1000.0, ez, lift_scale)
            {
                let col = if s.health_index < 0.72 {
                    Color32::from_rgb(239, 68, 68)
                } else if s.is_triggered || s.health_index < 0.88 {
                    Color32::from_rgb(245, 158, 11)
                } else {
                    Color32::from_rgb(34, 197, 94)
                };
                clip.circle_filled(sp, if s.is_triggered { 5.5 } else { 4.2 }, col);
                clip.circle_stroke(sp, 7.5, Stroke::new(1.2, col.gamma_multiply(0.75)));
            }
        }
        if let Some(sp) = self.project_active_bim_mm(
            xform,
            0.0,
            0.0,
            shm.building_height_m + 2.2,
            lift_scale,
        ) {
            let [r, g, b] = shm.atc20_placard.badge_color_rgb();
            let badge_col = Color32::from_rgb(r, g, b);
            let box_rect = Rect::from_center_size(sp, egui::Vec2::new(82.0, 20.0));
            clip.rect_filled(box_rect, 4.0, badge_col.gamma_multiply(0.92));
            clip.rect_stroke(box_rect, 4.0, Stroke::new(1.2, Color32::WHITE), egui::StrokeKind::Inside);
            clip.text(
                sp,
                Align2::CENTER_CENTER,
                format!("ATC-20: {}", shm.atc20_placard.short_label()),
                egui::FontId::monospace(10.0),
                Color32::WHITE,
            );
        }
    }

    pub(super) fn paint_cpu_evacuation_lifelines_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        use crate::core::evacuation_routing::{EvacuationMode, RoadEdgeStatus};
        use crate::core::shm_twin::Atc20Placard;

        if !self.evacuation_config.overlay_visible || self.world_map.zoom_level < 12.5 {
            return;
        }
        let Some(ref rep) = self.last_evacuation_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let proj_enu = |enu: [f64; 2], elev_m: f64| -> Option<Pos2> {
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [enu[0] * 1000.0, enu[1] * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, elev_m)
                .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
        };

        // Debris cordons
        for c in &rep.cordons {
            if c.placard == Atc20Placard::InspectedGreen {
                continue;
            }
            let col = if c.placard == Atc20Placard::UnsafeRed {
                Color32::from_rgb(239, 68, 68)
            } else {
                Color32::from_rgb(245, 158, 11)
            };
            let mut ring = Vec::with_capacity(25);
            for k in 0..=24 {
                let th = (k as f64 / 24.0) * std::f64::consts::TAU;
                let p = [
                    c.center_enu_m[0] + c.cordon_radius_m * th.cos(),
                    c.center_enu_m[1] + c.cordon_radius_m * th.sin(),
                ];
                if let Some(sp) = proj_enu(p, 0.25) {
                    ring.push(sp);
                }
            }
            if ring.len() >= 4 {
                clip.add(egui::Shape::line(ring, Stroke::new(2.0, col)));
            }
        }

        // Blocked / Degraded roads
        for e in &rep.edges {
            if e.road_name == "Site Access Spur" || e.status == RoadEdgeStatus::OpenSafe {
                continue;
            }
            let (Some(nu), Some(nv)) = (rep.nodes.get(e.u), rep.nodes.get(e.v)) else {
                continue;
            };
            if let (Some(p0), Some(p1)) = (proj_enu(nu.pos_enu_m, 0.30), proj_enu(nv.pos_enu_m, 0.30))
            {
                let col = if e.status == RoadEdgeStatus::BlockedClosed {
                    Color32::from_rgb(239, 68, 68)
                } else {
                    Color32::from_rgb(245, 158, 11)
                };
                clip.line_segment([p0, p1], Stroke::new(3.2, col));
            }
        }

        // Evacuation & Responder routes
        let show_ped = matches!(
            rep.config.mode,
            EvacuationMode::PedestrianEvacuation | EvacuationMode::Both
        );
        let show_resp = matches!(
            rep.config.mode,
            EvacuationMode::EmergencyResponder | EvacuationMode::Both
        );
        for route in &rep.routes {
            if !route.reachable || route.waypoints_enu_m.len() < 2 {
                continue;
            }
            if (route.is_responder_route && !show_resp) || (!route.is_responder_route && !show_ped)
            {
                continue;
            }
            let pts: Vec<Pos2> = route
                .waypoints_enu_m
                .iter()
                .filter_map(|&p| proj_enu(p, if route.is_responder_route { 0.8 } else { 0.6 }))
                .collect();
            if pts.len() >= 2 {
                let col = if route.is_responder_route {
                    Color32::from_rgb(56, 189, 248)
                } else {
                    Color32::from_rgb(34, 211, 153)
                };
                clip.add(egui::Shape::line(
                    pts,
                    Stroke::new(if route.is_active_building { 3.5 } else { 2.2 }, col),
                ));
            }
        }

        // Assembly safe points
        for sp_node in &rep.safe_points {
            if let Some(sp) = proj_enu(sp_node.center_enu_m, 2.0) {
                clip.circle_filled(sp, 6.0, Color32::from_rgb(34, 197, 94));
                clip.text(
                    sp + Vec2::new(0.0, -10.0),
                    Align2::CENTER_BOTTOM,
                    format!("SAFE ZONE: {}", sp_node.name),
                    egui::FontId::monospace(9.5),
                    Color32::from_rgb(167, 243, 208),
                );
            }
        }
    }

    /// Pure-vector CPU fallback rendering the Phase 80 District Risk Field IDW surface & hotspots
    /// when running headless UI tests or without a native WebGPU adapter.
    pub(super) fn paint_cpu_district_risk_heatmap_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.collab_state.config.risk_heatmap_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_risk_field_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let proj_enu = |east_m: f32, north_m: f32, elev_m: f32| -> Option<Pos2> {
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [east_m as f64 * 1000.0, north_m as f64 * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, elev_m as f64)
                .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
        };
        let op = self.collab_state.config.risk_opacity.clamp(0.15, 0.90);
        for cell in &report.cells {
            let hs = cell.half_step_m * 0.94;
            let z = cell.elev_m + 0.16 + cell.risk_01 * 0.45;
            let pts: Vec<Pos2> = [
                (cell.cx_m - hs, cell.cy_m - hs),
                (cell.cx_m + hs, cell.cy_m - hs),
                (cell.cx_m + hs, cell.cy_m + hs),
                (cell.cx_m - hs, cell.cy_m + hs),
            ]
            .iter()
            .filter_map(|&(ex, ny)| proj_enu(ex, ny, z))
            .collect();
            if pts.len() == 4 {
                let t = cell.risk_01.clamp(0.0, 1.0);
                let (r, g, b) = if t < 0.35 {
                    (16_u8, 185_u8, 155_u8)
                } else if t < 0.65 {
                    (245_u8, 158_u8, 11_u8)
                } else {
                    (239_u8, 68_u8, 68_u8)
                };
                let alpha = (op * (70.0 + 115.0 * t)).round().clamp(24.0, 185.0) as u8;
                clip.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(r, g, b, alpha),
                    Stroke::NONE,
                ));
            }
        }
        for h in report.hotspots.iter().take(6) {
            if let Some(sp) = proj_enu(h.east_m, h.north_m, 11.0 + h.risk_score * 6.0) {
                let col = if h.risk_score >= 0.70 {
                    Color32::from_rgb(239, 68, 68)
                } else if h.risk_score >= 0.45 {
                    Color32::from_rgb(245, 158, 11)
                } else {
                    Color32::from_rgb(16, 185, 129)
                };
                clip.circle_filled(sp, 5.0, col);
                clip.text(
                    sp + Vec2::new(0.0, -9.0),
                    Align2::CENTER_BOTTOM,
                    format!("{} ({:.0}%)", h.name, h.risk_score * 100.0),
                    egui::FontId::monospace(9.0),
                    Color32::WHITE,
                );
            }
        }
    }

    /// Pure-vector CPU fallback rendering Phase 82 Field QA/QC Pour-Strips & Spatial Waypoint Beacons.
    pub(super) fn paint_cpu_field_qaqc_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.field_qaqc_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_field_fab_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |lx_m: f64, ly_m: f64, lz_m: f64| -> Option<Pos2> {
            let ex = lx_m * cr - ly_m * sr;
            let ny = lx_m * sr + ly_m * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, lz_m)
                .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
        };

        for ps in &report.pour_strips {
            let hw = ps.width_m * 0.5;
            let hd = ps.depth_m * 0.5;
            let corners = [
                (ps.center_x_m - hw, ps.center_y_m - hd),
                (ps.center_x_m + hw, ps.center_y_m - hd),
                (ps.center_x_m + hw, ps.center_y_m + hd),
                (ps.center_x_m - hw, ps.center_y_m + hd),
            ];
            let pts: Vec<Pos2> = corners
                .iter()
                .filter_map(|&(x, y)| proj_enu(x, y, ps.z_m))
                .collect();
            if pts.len() == 4 {
                let [r, g, b] = ps.status.color_rgb();
                clip.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(r, g, b, 58),
                    Stroke::new(1.2, Color32::from_rgb(r, g, b)),
                ));
            }
        }

        for (idx, wp) in report.waypoints.iter().enumerate() {
            if let (Some(base_p), Some(top_p)) = (
                proj_enu(wp.local_x_m, wp.local_y_m, wp.local_z_m),
                proj_enu(wp.local_x_m, wp.local_y_m, wp.local_z_m + 2.2),
            ) {
                let [r, g, b] = wp.status.badge_color_rgb();
                let col = Color32::from_rgb(r, g, b);
                let is_sel = idx == self.active_waypoint_idx;
                clip.line_segment([base_p, top_p], Stroke::new(1.4, col));
                clip.circle_filled(top_p, if is_sel { 7.0 } else { 5.2 }, col);
                if is_sel {
                    clip.circle_stroke(top_p, 10.5, Stroke::new(1.5, Color32::WHITE));
                }
                clip.text(
                    top_p + Vec2::new(0.0, -9.0),
                    Align2::CENTER_BOTTOM,
                    format!("{} [{}]", wp.code, wp.status.short_label()),
                    egui::FontId::monospace(8.8),
                    Color32::WHITE,
                );
            }
        }
    }

    /// Pure-vector CPU fallback rendering Phase 83 EigenFlow Woodbury `P1 -> P2` Load-Path & Hinges.
    pub(super) fn paint_cpu_eigenflow_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.eigenflow_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_eigenflow_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        let wb = &report.woodbury;
        if let (Some(a), Some(b)) = (
            proj_enu(wb.disrupted_p0_enu_m),
            proj_enu(wb.disrupted_p1_enu_m),
        ) {
            let p1_col = Color32::from_rgb(255, 69, 58);
            clip.line_segment([a, b], Stroke::new(4.0, p1_col));
            let mid = Pos2::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y));
            clip.circle_filled(mid, 6.0, p1_col);
            clip.text(
                mid + Vec2::new(0.0, -8.0),
                Align2::CENTER_BOTTOM,
                format!("P1 #{}", wb.disrupted_element_id),
                egui::FontId::monospace(8.5),
                Color32::WHITE,
            );
        }

        for m in &wb.redistributed_p2 {
            if let (Some(a), Some(b)) = (proj_enu(m.p0_enu_m), proj_enu(m.p1_enu_m)) {
                let col = if m.is_critical_overload {
                    Color32::from_rgb(255, 69, 58)
                } else if m.post_yield_dcr > 0.72 {
                    Color32::from_rgb(255, 179, 64)
                } else {
                    Color32::from_rgb(48, 209, 240)
                };
                clip.line_segment([a, b], Stroke::new(2.6, col));
            }
        }

        let active_step = self.eigenflow_config.active_step_idx;
        for h in &report.hinges_3d {
            if h.formed_at_step <= active_step {
                if let Some(p) = proj_enu(h.pos_enu_m) {
                    let col = match h.state {
                        crate::core::nonlinear::PlasticHingeState::Elastic => Color32::from_rgb(48, 209, 88),
                        crate::core::nonlinear::PlasticHingeState::ImmediateOccupancy => Color32::from_rgb(10, 132, 255),
                        crate::core::nonlinear::PlasticHingeState::LifeSafety => Color32::from_rgb(255, 159, 10),
                        crate::core::nonlinear::PlasticHingeState::CollapsePrevention => Color32::from_rgb(255, 69, 58),
                        crate::core::nonlinear::PlasticHingeState::Mechanism => Color32::from_rgb(191, 90, 242),
                    };
                    clip.circle_filled(p, 4.8, col);
                    clip.circle_stroke(p, 7.0, Stroke::new(1.1, Color32::WHITE));
                }
            }
        }
    }

    /// Pure-vector CPU fallback rendering Phase 84 Extreme Events Blast shockwave & Fire compartment.
    pub(super) fn paint_cpu_extreme_events_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.extreme_events_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_extreme_events_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        if matches!(
            report.config.mode,
            crate::core::extreme_events::ExtremeEventMode::Blast
                | crate::core::extreme_events::ExtremeEventMode::Combined
        ) {
            if let (Some(cp), Some(fp)) = (
                proj_enu(report.charge_pos_enu_m),
                proj_enu(report.target_face_enu_m),
            ) {
                clip.line_segment([cp, fp], Stroke::new(2.0, Color32::from_rgb(255, 149, 0)));
                clip.circle_filled(cp, 6.5, Color32::from_rgb(255, 69, 58));
                clip.circle_stroke(cp, 18.0, Stroke::new(1.4, Color32::from_rgb(255, 120, 40)));
                clip.circle_stroke(cp, 32.0, Stroke::new(1.0, Color32::from_rgb(255, 179, 64)));
            }
            for m in &report.blast_member_impacts {
                if let (Some(a), Some(b)) = (proj_enu(m.p0_enu_m), proj_enu(m.p1_enu_m)) {
                    let (r, g, b_col) = m.damage_level.color_rgb();
                    clip.line_segment([a, b], Stroke::new(3.0, Color32::from_rgb(r, g, b_col)));
                }
            }
        }
        if matches!(
            report.config.mode,
            crate::core::extreme_events::ExtremeEventMode::StructuralFire
                | crate::core::extreme_events::ExtremeEventMode::Combined
        ) {
            let fc = &report.fire_compartment;
            let hw = fc.width_m * 0.5;
            let hd = fc.depth_m * 0.5;
            let z_mid = 0.5 * (fc.z_bottom_m + fc.z_top_m);
            let pts: Vec<Pos2> = [
                [fc.center_x_m - hw, fc.center_y_m - hd, z_mid],
                [fc.center_x_m + hw, fc.center_y_m - hd, z_mid],
                [fc.center_x_m + hw, fc.center_y_m + hd, z_mid],
                [fc.center_x_m - hw, fc.center_y_m + hd, z_mid],
            ]
            .iter()
            .filter_map(|&p| proj_enu(p))
            .collect();
            if pts.len() == 4 {
                clip.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(255, 110, 20, 68),
                    Stroke::new(1.6, Color32::from_rgb(255, 149, 0)),
                ));
            }
        }
    }

    /// Pure-vector CPU fallback rendering Update `v0.8.4.1` Base Isolator Pads, Moat Ring & FVD Dampers.
    pub(super) fn paint_cpu_isolation_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.isolation_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_isolation_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        let m = &report.moat_3d;
        let ow = m.inner_width_m * 0.5 + m.moat_gap_m;
        let od = m.inner_depth_m * 0.5 + m.moat_gap_m;
        let moat_pts: Vec<Pos2> = [
            [m.center_x_m - ow, m.center_y_m - od, m.z_m],
            [m.center_x_m + ow, m.center_y_m - od, m.z_m],
            [m.center_x_m + ow, m.center_y_m + od, m.z_m],
            [m.center_x_m - ow, m.center_y_m + od, m.z_m],
        ]
        .iter()
        .filter_map(|&p| proj_enu(p))
        .collect();
        if moat_pts.len() == 4 {
            for i in 0..4 {
                clip.line_segment(
                    [moat_pts[i], moat_pts[(i + 1) % 4]],
                    Stroke::new(1.8, Color32::from_rgb(255, 179, 64)),
                );
            }
        }
        for pad in &report.pads_3d {
            if let Some(sp) = proj_enu(pad.pos_enu_m) {
                let col = if pad.is_safe {
                    Color32::from_rgb(48, 209, 240)
                } else {
                    Color32::from_rgb(255, 69, 58)
                };
                clip.circle_filled(sp, 5.0, col);
            }
        }
        for d in &report.dampers_3d {
            if let (Some(a), Some(b)) = (proj_enu(d.p0_enu_m), proj_enu(d.p1_enu_m)) {
                clip.line_segment([a, b], Stroke::new(2.2, Color32::from_rgb(48, 209, 88)));
            }
        }
    }

    /// Pure-vector CPU fallback rendering Update `v0.8.4.2` Wind Facade Cladding Pressures & Flow Vectors.
    pub(super) fn paint_cpu_wind_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.wind_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_wind_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for pan in &report.panels_3d {
            if let Some(sp) = proj_enu(pan.center_enu_m) {
                let col = if pan.is_windward {
                    Color32::from_rgb(56, 189, 248)
                } else {
                    Color32::from_rgb(245, 158, 11)
                };
                clip.circle_filled(sp, 4.2, col);
            }
        }
        for arr in &report.arrows_3d {
            if let (Some(a), Some(b)) = (proj_enu(arr.p0_enu_m), proj_enu(arr.p1_enu_m)) {
                let col = if arr.is_suction {
                    Color32::from_rgb(255, 112, 48)
                } else {
                    Color32::from_rgb(48, 224, 255)
                };
                clip.line_segment([a, b], Stroke::new(2.2, col));
                clip.circle_filled(b, 3.6, col);
            }
        }
    }

    /// Pure-vector CPU fallback rendering Update `v0.8.4.3` Special RC Shear Wall SBE & Coupling Beam X-Cages.
    pub(super) fn paint_cpu_shear_wall_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.shear_wall_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_shear_wall_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for pier in &report.piers_3d {
            let sbe_col = if pier.is_compliant {
                Color32::from_rgb(48, 209, 88)
            } else {
                Color32::from_rgb(255, 69, 58)
            };
            if let (Some(a), Some(b)) = (
                proj_enu([pier.p0_enu_m[0], pier.p0_enu_m[1], pier.base_z_m]),
                proj_enu([pier.p1_enu_m[0], pier.p1_enu_m[1], pier.base_z_m]),
            ) {
                clip.line_segment([a, b], Stroke::new(3.2, Color32::from_rgb(96, 165, 250)));
                clip.circle_filled(a, 5.2, sbe_col);
                clip.circle_filled(b, 5.2, sbe_col);
            }
        }
        for cb in &report.coupling_beams_3d {
            let z_bot = cb.p0_enu_m[2] - cb.depth_m;
            if let (Some(a0), Some(a1), Some(b0), Some(b1)) = (
                proj_enu([cb.p0_enu_m[0], cb.p0_enu_m[1], z_bot]),
                proj_enu(cb.p0_enu_m),
                proj_enu([cb.p1_enu_m[0], cb.p1_enu_m[1], z_bot]),
                proj_enu(cb.p1_enu_m),
            ) {
                let col = Color32::from_rgb(56, 189, 248);
                clip.line_segment([a0, b1], Stroke::new(1.8, col));
                if cb.is_diagonal_x_cage {
                    clip.line_segment([a1, b0], Stroke::new(1.8, col));
                }
            }
        }
    }

    /// Pure-vector CPU fallback rendering Update `v0.8.4.4` Steel Connection Joint Plates & Bolt Clusters.
    pub(super) fn paint_cpu_connection_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.connection_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_connection_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for j in &report.joints_3d {
            if let Some(sp) = proj_enu(j.pos_enu_m) {
                let col = if !j.is_compliant {
                    Color32::from_rgb(255, 69, 58)
                } else if j.doubler_required {
                    Color32::from_rgb(255, 179, 64)
                } else {
                    Color32::from_rgb(48, 224, 255)
                };
                clip.rect_filled(Rect::from_center_size(sp, Vec2::splat(9.0)), 2.0, col);
            }
        }
        for b in &report.bolts_3d {
            if let Some(sp) = proj_enu(b.pos_enu_m) {
                clip.circle_filled(sp, 2.5, Color32::from_rgb(250, 204, 21));
            }
        }
    }

    /// Pure-vector CPU fallback rendering Update `v0.8.4.5` Post-Tensioned Tendon Drape & Anchorages.
    pub(super) fn paint_cpu_prestress_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.prestress_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_prestress_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for bay in &report.bays_3d {
            let col = if bay.is_compliant {
                Color32::from_rgb(250, 204, 21)
            } else {
                Color32::from_rgb(255, 69, 58)
            };
            let pts: Vec<Pos2> = bay
                .tendon_pts_enu_m
                .iter()
                .filter_map(|p| proj_enu(*p))
                .collect();
            for win in pts.windows(2) {
                clip.line_segment([win[0], win[1]], Stroke::new(2.6, col));
            }
        }
        for anc in &report.anchorages_3d {
            if let Some(sp) = proj_enu(anc.pos_enu_m) {
                let col = if anc.is_live_end {
                    Color32::from_rgb(56, 189, 248)
                } else {
                    Color32::from_rgb(245, 158, 11)
                };
                clip.circle_filled(sp, 4.5, col);
            }
        }
    }

    /// Pure-vector CPU fallback rendering Update `v0.8.4.6` Composite Deck Strips & Shear Stud Pins.
    pub(super) fn paint_cpu_composite_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.composite_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_composite_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for bay in &report.bays_3d {
            if let (Some(a), Some(b)) = (proj_enu(bay.p0_enu_m), proj_enu(bay.p1_enu_m)) {
                let col = if bay.is_compliant {
                    Color32::from_rgb(36, 214, 184)
                } else {
                    Color32::from_rgb(255, 96, 64)
                };
                clip.line_segment([a, b], Stroke::new(4.2, col));
            }
        }
        for st in &report.studs_3d {
            if let Some(sp) = proj_enu(st.pos_enu_m) {
                clip.circle_filled(sp, 2.8, Color32::from_rgb(250, 204, 21));
            }
        }
    }

    /// Update `v0.8.4.7` (`7/10` -> `v0.8.5`): Pure-vector CPU fallback rendering Structural Reliability member beta contours & risk hotspots.
    pub(super) fn paint_cpu_reliability_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.reliability_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_reliability_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        let target = report.config.target_beta;
        for m in &report.members_3d {
            if let (Some(a), Some(b)) = (proj_enu(m.p0_enu_m), proj_enu(m.p1_enu_m)) {
                let col = if m.is_below_target {
                    Color32::from_rgb(239, 68, 68)
                } else if m.beta_index < target + 0.5 {
                    Color32::from_rgb(245, 158, 11)
                } else {
                    Color32::from_rgb(16, 185, 129)
                };
                clip.line_segment([a, b], Stroke::new(3.6, col));
            }
        }
        for hs in &report.hotspots_3d {
            if let Some(sp) = proj_enu(hs.pos_enu_m) {
                let col = if hs.is_critical {
                    Color32::from_rgb(248, 113, 113)
                } else {
                    Color32::from_rgb(251, 191, 36)
                };
                let r = if hs.is_critical { 5.2 } else { 3.8 };
                let pts = vec![
                    sp + Vec2::new(0.0, -r),
                    sp + Vec2::new(r, 0.0),
                    sp + Vec2::new(0.0, r),
                    sp + Vec2::new(-r, 0.0),
                ];
                clip.add(egui::Shape::closed_line(pts, Stroke::new(1.8, col)));
            }
        }
    }

    /// Update `v0.8.4.8` (`8/10` -> `v0.8.5`): Pure-vector CPU fallback rendering Deep Foundation pile caps, pile shafts & Winkler spring nodes.
    pub(super) fn paint_cpu_foundation_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.foundation_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_foundation_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for pile in &report.piles_3d {
            if let (Some(a), Some(b)) = (proj_enu(pile.top_enu_m), proj_enu(pile.tip_enu_m)) {
                let col = if !pile.is_safe {
                    Color32::from_rgb(239, 68, 68)
                } else if pile.is_uplift {
                    Color32::from_rgb(56, 189, 248)
                } else {
                    Color32::from_rgb(16, 185, 129)
                };
                clip.line_segment([a, b], Stroke::new(3.2, col));
            }
        }
        for cap in &report.caps_3d {
            let hw = cap.width_m * 0.5;
            let hl = cap.length_m * 0.5;
            let z = cap.center_enu_m[2];
            let corners: Vec<Pos2> = [
                [cap.center_enu_m[0] - hw, cap.center_enu_m[1] - hl, z],
                [cap.center_enu_m[0] + hw, cap.center_enu_m[1] - hl, z],
                [cap.center_enu_m[0] + hw, cap.center_enu_m[1] + hl, z],
                [cap.center_enu_m[0] - hw, cap.center_enu_m[1] + hl, z],
            ]
            .into_iter()
            .filter_map(proj_enu)
            .collect();
            if corners.len() == 4 {
                let col = if cap.is_compliant {
                    Color32::from_rgb(52, 211, 153)
                } else {
                    Color32::from_rgb(248, 113, 113)
                };
                clip.add(egui::Shape::closed_line(corners, Stroke::new(2.0, col)));
            }
        }
        for sp in &report.springs_3d {
            if let Some(pt) = proj_enu(sp.pos_enu_m) {
                clip.circle_filled(pt, 2.8, Color32::from_rgb(250, 204, 21));
            }
        }
    }

    /// Update `v0.8.4.9` (`9/10` -> `v0.8.5`): Pure-vector CPU fallback rendering 4D/5D/6D story erection zones & tower crane.
    pub(super) fn paint_cpu_construction_lca_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.construction_lca_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_construction_lca_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for z in &report.story_zones_3d {
            let col = if z.completion_pct >= 99.5 {
                Color32::from_rgb(16, 185, 129)
            } else if z.completion_pct > 0.5 {
                Color32::from_rgb(245, 158, 11)
            } else {
                Color32::from_rgb(56, 189, 248)
            };
            let z_top = z.min_enu_m[2]
                + (z.max_enu_m[2] - z.min_enu_m[2]) * (z.completion_pct / 100.0).clamp(0.18, 1.0);
            let pts: Vec<Pos2> = [
                [z.min_enu_m[0], z.min_enu_m[1], z_top],
                [z.max_enu_m[0], z.min_enu_m[1], z_top],
                [z.max_enu_m[0], z.max_enu_m[1], z_top],
                [z.min_enu_m[0], z.max_enu_m[1], z_top],
            ]
            .into_iter()
            .filter_map(proj_enu)
            .collect();
            if pts.len() == 4 {
                clip.add(egui::Shape::closed_line(pts, Stroke::new(2.2, col)));
            }
        }
        let c = &report.crane_3d;
        let top = [c.base_enu_m[0], c.base_enu_m[1], c.base_enu_m[2] + c.mast_height_m];
        let jib_ang = c.slew_deg.to_radians();
        let tip = [
            c.base_enu_m[0] + c.jib_length_m * jib_ang.cos(),
            c.base_enu_m[1] + c.jib_length_m * jib_ang.sin(),
            top[2],
        ];
        if let (Some(a), Some(b), Some(d)) = (proj_enu(c.base_enu_m), proj_enu(top), proj_enu(tip)) {
            let crane_col = Color32::from_rgb(250, 204, 21);
            clip.line_segment([a, b], Stroke::new(3.0, crane_col));
            clip.line_segment([b, d], Stroke::new(2.6, crane_col));
            clip.circle_filled(d, 3.6, Color32::from_rgb(249, 115, 22));
        }
    }

    /// Screen-space click picking against LOD 0 regional clusters or LOD 1/2 3D neighbor buildings.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_domain_overlay(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        bim_project: &BimProject,
        members: &[StructuralMember],
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        if bim_project.is_empty() && members.is_empty() && self.entities.is_empty() {
            return;
        }
        let clip_painter = painter.with_clip_rect(rect);

        match active_pillar {
            NavigationPillar::MyLibrary => {
                // Graph topology links connecting structural column nodes
                let pts: Vec<Pos2> = bim_project
                    .columns
                    .iter()
                    .take(12)
                    .map(|c| to_screen([c.pos.0, c.pos.1]))
                    .collect();
                for pair in pts.windows(2) {
                    clip_painter.line_segment(
                        [pair[0], pair[1]],
                        Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.45)),
                    );
                }
            }
            NavigationPillar::AiAgent => {
                // AI focus nodes highlighting primary bays
                for &(x, y, tag) in &[
                    (3000.0, 2250.0, "AI-OPT Bay A1"),
                    (9000.0, 6750.0, "AI-OPT Bay B2"),
                ] {
                    let p = to_screen([x, y]);
                    clip_painter.circle_stroke(p, 22.0, Stroke::new(1.4, theme.accent_blue));
                    clip_painter.text(
                        p + Vec2::new(0.0, -28.0),
                        Align2::CENTER_BOTTOM,
                        tag,
                        egui::FontId::monospace(9.5),
                        theme.accent_blue,
                    );
                }
            }
            NavigationPillar::ResearchConcept => {
                // Site boundary setback & geodetic CRS datum box
                let p0 = to_screen([-1000.0, -1000.0]);
                let p1 = to_screen([13000.0, 10000.0]);
                clip_painter.rect_stroke(
                    Rect::from_two_pos(p0, p1),
                    CornerRadius::ZERO,
                    Stroke::new(1.5, theme.accent_orange),
                    StrokeKind::Inside,
                );
                clip_painter.text(
                    Pos2::new(p0.x.min(p1.x) + 8.0, p0.y.min(p1.y) + 8.0),
                    Align2::LEFT_TOP,
                    "WGS84 / UTM Site Boundary • Seismic SDC-D",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::ParametricDED => {
                let p0 = to_screen([0.0, -500.0]);
                let p1 = to_screen([6000.0, -500.0]);
                clip_painter.line_segment([p0, p1], Stroke::new(1.6, theme.accent_orange));
                clip_painter.text(
                    Pos2::new((p0.x + p1.x) * 0.5, p0.y + 12.0),
                    Align2::CENTER_TOP,
                    "Bay X1 = 6000 mm (Parametric)",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::GeometryAssembly => {
                let p = to_screen([12000.0, 9000.0]);
                clip_painter.text(
                    p + Vec2::new(8.0, -10.0),
                    Align2::LEFT_BOTTOM,
                    format!(
                        "LOD 350 • {} Walls • {} Cols • {} Beams",
                        bim_project.walls.len(),
                        bim_project.columns.len(),
                        bim_project.runs.len()
                    ),
                    egui::FontId::monospace(9.5),
                    theme.accent_green,
                );
            }
            NavigationPillar::AnalysisSimulation => {
                let avg_eigengate = if members.is_empty() {
                    0.942
                } else {
                    members.iter().map(|m| m.eigengate_score).sum::<f64>() / members.len() as f64
                };
                for col in &bim_project.columns {
                    let cp = to_screen([col.pos.0, col.pos.1]);
                    clip_painter.circle_stroke(
                        cp,
                        12.0,
                        Stroke::new(1.4, theme.accent_green.gamma_multiply(0.8)),
                    );
                }
                let p = to_screen([6000.0, 9500.0]);
                clip_painter.text(
                    p,
                    Align2::CENTER_BOTTOM,
                    format!("FEA Overlay • Mean EigenGate {:.3} • SVD Guard Active", avg_eigengate),
                    egui::FontId::monospace(10.0),
                    theme.accent_green,
                );
            }
            NavigationPillar::TimeScheduling => {
                for (idx, (x, y, label)) in [
                    (3000.0, 2250.0, "4D Phase 1 (W1-W4)"),
                    (9000.0, 2250.0, "4D Phase 2 (W5-W8)"),
                    (3000.0, 6750.0, "4D Phase 3 (W9-W12)"),
                    (9000.0, 6750.0, "4D Phase 4 (W13-W16)"),
                ]
                .into_iter()
                .enumerate()
                {
                    let c = to_screen([x, y]);
                    let col = if idx < 2 {
                        theme.accent_green
                    } else {
                        theme.accent_blue
                    };
                    clip_painter.circle_stroke(c, 16.0, Stroke::new(1.5, col));
                    clip_painter.text(
                        c + Vec2::new(0.0, 20.0),
                        Align2::CENTER_TOP,
                        label,
                        egui::FontId::monospace(9.0),
                        col,
                    );
                }
            }
            NavigationPillar::CostBudgeting => {
                let p0 = to_screen([0.0, 0.0]);
                let p1 = to_screen([6000.0, 4500.0]);
                clip_painter.rect_filled(
                    Rect::from_two_pos(p0, p1),
                    CornerRadius::ZERO,
                    theme.accent_orange.gamma_multiply(0.10),
                );
                clip_painter.text(
                    Rect::from_two_pos(p0, p1).center(),
                    Align2::CENTER_CENTER,
                    "5D High-Cost Core Zone (42% BoQ)",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::SustainabilityEnergy => {
                // 6D Solar Heliodon Arc
                let center = to_screen([6000.0, 4500.0]);
                let edge = to_screen([12500.0, 4500.0]);
                let rad = (edge.x - center.x).abs().max(40.0);
                clip_painter.circle_stroke(
                    center,
                    rad,
                    Stroke::new(1.2, theme.accent_orange.gamma_multiply(0.65)),
                );
                let sun = center + Vec2::new(rad * 0.72, -rad * 0.68);
                clip_painter.circle_filled(sun, 7.0, theme.accent_orange);
                clip_painter.line_segment(
                    [sun, center],
                    Stroke::new(1.2, theme.accent_orange.gamma_multiply(0.8)),
                );
                clip_painter.text(
                    sun + Vec2::new(10.0, -4.0),
                    Align2::LEFT_CENTER,
                    "Heliodon Azimuth 52° • OTTV 31 W/m²",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::FacilityManagement => {
                for (x, y, tag) in [
                    (1500.0, 4500.0, "SHM-01"),
                    (6000.0, 4500.0, "SHM-02"),
                    (10500.0, 4500.0, "MEP Riser"),
                ] {
                    let p = to_screen([x, y]);
                    clip_painter.circle_filled(p, 5.0, theme.accent_green);
                    clip_painter.text(
                        p + Vec2::new(8.0, -8.0),
                        Align2::LEFT_BOTTOM,
                        tag,
                        egui::FontId::monospace(9.5),
                        theme.accent_green,
                    );
                }
            }
            NavigationPillar::SafetyHealth => {
                // 8D Tower Crane Sweep Radius & Boom
                let mast = to_screen([6000.0, 10200.0]);
                let tip = to_screen([12800.0, 10200.0]);
                let rad = (tip.x - mast.x).abs().max(40.0);
                clip_painter.circle_stroke(
                    mast,
                    rad,
                    Stroke::new(1.6, theme.accent_orange.gamma_multiply(0.8)),
                );
                let boom_end = mast + Vec2::new(rad * 0.7, rad * 0.68);
                clip_painter.line_segment([mast, boom_end], Stroke::new(2.6, theme.accent_red));
                clip_painter.circle_filled(mast, 5.5, theme.accent_red);
                clip_painter.text(
                    mast + Vec2::new(10.0, -10.0),
                    Align2::LEFT_BOTTOM,
                    "TC-01 Jib Sweep Radius (45m)",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::LeanConstruction => {
                let l0 = to_screen([-1400.0, 0.0]);
                let l1 = to_screen([-200.0, 4500.0]);
                let laydown = Rect::from_two_pos(l0, l1);
                clip_painter.rect_stroke(
                    laydown,
                    CornerRadius::same(4),
                    Stroke::new(1.5, theme.accent_green),
                    StrokeKind::Inside,
                );
                clip_painter.text(
                    laydown.center(),
                    Align2::CENTER_CENTER,
                    "JIT Laydown",
                    egui::FontId::monospace(9.0),
                    theme.accent_green,
                );
            }
            NavigationPillar::IndustrializedAutomation => {
                let p0 = to_screen([0.0, 0.0]);
                let p1 = to_screen([6000.0, 4500.0]);
                clip_painter.rect_stroke(
                    Rect::from_two_pos(p0, p1),
                    CornerRadius::ZERO,
                    Stroke::new(1.6, theme.accent_blue),
                    StrokeKind::Inside,
                );
                clip_painter.text(
                    Rect::from_two_pos(p0, p1).center(),
                    Align2::CENTER_CENTER,
                    "10D DfMA Pod Module M-01",
                    egui::FontId::monospace(9.5),
                    theme.accent_blue,
                );
            }
            NavigationPillar::Prediction => {
                let p = to_screen([6000.0, 4500.0]);
                clip_painter.circle_stroke(p, 28.0, Stroke::new(1.6, theme.accent_red));
                clip_painter.circle_stroke(p, 42.0, Stroke::new(1.0, theme.accent_orange));
                clip_painter.text(
                    p + Vec2::new(0.0, -48.0),
                    Align2::CENTER_BOTTOM,
                    "Drift Hotspot Watch • Pf < 1e-4",
                    egui::FontId::monospace(9.5),
                    theme.accent_orange,
                );
            }
            NavigationPillar::ReportPublishing => {
                let margin_rect = rect.shrink(14.0);
                clip_painter.rect_stroke(
                    margin_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.4, theme.text_muted),
                    StrokeKind::Inside,
                );
                clip_painter.text(
                    Pos2::new(margin_rect.right() - 8.0, margin_rect.bottom() - 8.0),
                    Align2::RIGHT_BOTTOM,
                    "ISO A3 SHEET BORDER • EIGENGRAPH DED",
                    egui::FontId::monospace(9.0),
                    theme.text_muted,
                );
            }
        }
    }

    /// Paint synchronized sketch/annotation entities onto any compatible 2D world canvas
    /// (used to sync Universal Canvas markup inside ParametricView's single 2D canvas).
    pub fn paint_entities_world(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        self.paint_entities(painter, rect, theme, active_pillar, to_screen);
    }

    pub(super) fn paint_entities(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        active_pillar: NavigationPillar,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        let clip_painter = painter.with_clip_rect(rect);
        let active_badge = active_pillar.badge();
        let is_screen_valid =
            |p: Pos2| p.x.is_finite() && p.y.is_finite() && p.x > -50_000.0 && p.y > -50_000.0;

        let globe_xform = if self.globe_mode {
            self.last_globe_xform
                .as_ref()
                .filter(|x| x.rect == rect)
                .cloned()
        } else {
            None
        };

        for ent in &self.entities {
            if !self.sync_all_layers && ent.pillar_badge != active_badge {
                continue;
            }
            let base_col = Color32::from_rgb(ent.color_rgb[0], ent.color_rgb[1], ent.color_rgb[2]);
            let is_selected = self.is_entity_selected(ent.id);
            let col = if is_selected {
                theme.accent_orange
            } else if ent.pillar_badge == active_badge {
                base_col
            } else {
                base_col.gamma_multiply(0.82)
            };
            let stroke = Stroke::new(
                if is_selected {
                    ent.stroke_width + 1.4
                } else {
                    ent.stroke_width
                },
                col,
            );

            match &ent.kind {
                CanvasShapeKind::Freehand { points_mm } => {
                    let spts: Vec<Pos2> = points_mm.iter().map(|p| to_screen(*p)).collect();
                    for w in spts.windows(2) {
                        if is_screen_valid(w[0]) && is_screen_valid(w[1]) {
                            clip_painter.line_segment([w[0], w[1]], stroke);
                        }
                    }
                }
                CanvasShapeKind::Line { start_mm, end_mm } => {
                    let len_mm = (end_mm[0] - start_mm[0]).hypot(end_mm[1] - start_mm[1]);
                    if let Some(xf) = globe_xform.as_ref().filter(|_| len_mm > 5_000_000.0) {
                        let a = SlippyMapState::world_mm_to_lat_lon(
                            self.ground_origin_lat_lon,
                            *start_mm,
                        );
                        let b = SlippyMapState::world_mm_to_lat_lon(
                            self.ground_origin_lat_lon,
                            *end_mm,
                        );
                        let arc = crate::core::gis::great_circle_arc(a, b, 16);
                        let proj: Vec<Option<Pos2>> = arc
                            .iter()
                            .map(|&(la, lo)| {
                                xf.project_lat_lon_elev(la, lo, 0.0)
                                    .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
                            })
                            .collect();
                        for pair in proj.windows(2) {
                            if let (Some(p0), Some(p1)) = (pair[0], pair[1]) {
                                clip_painter.line_segment([p0, p1], stroke);
                            }
                        }
                    } else {
                        let p0 = to_screen(*start_mm);
                        let p1 = to_screen(*end_mm);
                        if is_screen_valid(p0) && is_screen_valid(p1) {
                            clip_painter.line_segment([p0, p1], stroke);
                        }
                    }
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    let p0 = to_screen(*min_mm);
                    let p1 = to_screen(*max_mm);
                    if is_screen_valid(p0) && is_screen_valid(p1) {
                        let r = Rect::from_two_pos(p0, p1);
                        clip_painter.rect_filled(r, CornerRadius::ZERO, col.gamma_multiply(0.08));
                        clip_painter.rect_stroke(r, CornerRadius::ZERO, stroke, StrokeKind::Inside);
                    }
                }
                CanvasShapeKind::Circle {
                    center_mm,
                    radius_mm,
                } => {
                    let c = to_screen(*center_mm);
                    if is_screen_valid(c) {
                        let edge = to_screen([center_mm[0] + radius_mm, center_mm[1]]);
                        let r_px = if is_screen_valid(edge) {
                            (edge - c).length().max(2.0)
                        } else {
                            (self.world_map.pixels_per_mm_at_origin(self.ground_origin_lat_lon)
                                * (*radius_mm as f32))
                                .max(2.0)
                        };
                        clip_painter.circle_stroke(c, r_px, stroke);
                    }
                }
                CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    let s0 = to_screen(*start_mm);
                    let s1 = to_screen(*end_mm);
                    let len_mm = (end_mm[0] - start_mm[0]).hypot(end_mm[1] - start_mm[1]);
                    if let Some(xf) = globe_xform.as_ref().filter(|_| len_mm > 5_000_000.0) {
                        let a = SlippyMapState::world_mm_to_lat_lon(
                            self.ground_origin_lat_lon,
                            *start_mm,
                        );
                        let b = SlippyMapState::world_mm_to_lat_lon(
                            self.ground_origin_lat_lon,
                            *end_mm,
                        );
                        let arc = crate::core::gis::great_circle_arc(a, b, 16);
                        let proj: Vec<Option<Pos2>> = arc
                            .iter()
                            .map(|&(la, lo)| {
                                xf.project_lat_lon_elev(la, lo, 0.0)
                                    .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
                            })
                            .collect();
                        for pair in proj.windows(2) {
                            if let (Some(p0), Some(p1)) = (pair[0], pair[1]) {
                                clip_painter.line_segment([p0, p1], stroke);
                            }
                        }
                        if is_screen_valid(s0) {
                            clip_painter.circle_filled(s0, 3.0, col);
                        }
                        if is_screen_valid(s1) {
                            clip_painter.circle_filled(s1, 3.0, col);
                        }
                        if let Some(Some(mid)) = proj.get(proj.len() / 2) {
                            let label = if len_mm >= 1_000_000.0 {
                                format!("[{}] {:.2} km", ent.pillar_badge, len_mm / 1_000_000.0)
                            } else {
                                format!("[{}] {:.0} mm", ent.pillar_badge, len_mm)
                            };
                            clip_painter.text(
                                *mid + Vec2::new(0.0, -10.0),
                                Align2::CENTER_BOTTOM,
                                label,
                                egui::FontId::monospace(10.0),
                                col,
                            );
                        }
                    } else if is_screen_valid(s0) && is_screen_valid(s1) {
                        clip_painter.line_segment([s0, s1], stroke);
                        clip_painter.circle_filled(s0, 3.0, col);
                        clip_painter.circle_filled(s1, 3.0, col);
                        let mid = Pos2::new((s0.x + s1.x) * 0.5, (s0.y + s1.y) * 0.5 - 10.0);
                        clip_painter.text(
                            mid,
                            Align2::CENTER_BOTTOM,
                            format!("[{}] {:.0} mm", ent.pillar_badge, len_mm),
                            egui::FontId::monospace(10.0),
                            col,
                        );
                    }
                }
                CanvasShapeKind::Callout { pos_mm, text } => {
                    let p = to_screen(*pos_mm);
                    if !is_screen_valid(p) {
                        continue;
                    }
                    clip_painter.circle_filled(p, 4.0, col);
                    let box_pos = p + Vec2::new(8.0, -18.0);
                    let bg = if theme.is_dark() {
                        Color32::from_rgba_premultiplied(20, 24, 34, 225)
                    } else {
                        Color32::from_rgba_premultiplied(255, 255, 255, 235)
                    };
                    let tag = format!("[{}] {}", ent.pillar_badge, text);
                    let approx_w = (tag.len() as f32 * 6.3 + 14.0).clamp(80.0, 280.0);
                    let tag_rect = Rect::from_min_size(box_pos, Vec2::new(approx_w, 20.0));
                    clip_painter.rect_filled(tag_rect, CornerRadius::same(5), bg);
                    clip_painter.rect_stroke(
                        tag_rect,
                        CornerRadius::same(5),
                        Stroke::new(1.0, col),
                        StrokeKind::Inside,
                    );
                    clip_painter.text(
                        tag_rect.center(),
                        Align2::CENTER_CENTER,
                        tag,
                        egui::FontId::monospace(9.5),
                        theme.text_primary,
                    );
                }
            }
        }
    }

    /// Paints live rubber-band sketch/dimension/rectangle/circle preview using any world-to-screen mapping.
    pub fn paint_live_preview_world(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        self.paint_live_preview(painter, rect, theme, to_screen);
    }

    pub(super) fn paint_live_preview(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        to_screen: &impl Fn([f64; 2]) -> Pos2,
    ) {
        let clip_painter = painter.with_clip_rect(rect);
        let preview_col = self.active_color();
        let stroke = Stroke::new(self.stroke_width, preview_col);

        if self.active_freehand_mm.len() >= 2 {
            let spts: Vec<Pos2> = self
                .active_freehand_mm
                .iter()
                .map(|p| to_screen(*p))
                .collect();
            for w in spts.windows(2) {
                clip_painter.line_segment([w[0], w[1]], stroke);
            }
        }

        if let Some(start) = self.pending_start_mm {
            let end = if matches!(
                self.active_tool,
                UniversalCanvasTool::Line | UniversalCanvasTool::Dimension
            ) {
                self.apply_ortho(start, self.cursor_world_mm)
            } else {
                self.cursor_world_mm
            };
            let s0 = to_screen(start);
            let s1 = to_screen(end);
            match self.active_tool {
                UniversalCanvasTool::Line | UniversalCanvasTool::Dimension => {
                    clip_painter.line_segment([s0, s1], stroke);
                }
                UniversalCanvasTool::Rectangle => {
                    clip_painter.rect_stroke(
                        Rect::from_two_pos(s0, s1),
                        CornerRadius::ZERO,
                        stroke,
                        StrokeKind::Inside,
                    );
                }
                UniversalCanvasTool::Circle => {
                    let r_px = (s1 - s0).length().max(2.0);
                    clip_painter.circle_stroke(s0, r_px, stroke);
                }
                _ => {}
            }
            let dist = (end[0] - start[0]).hypot(end[1] - start[1]);
            clip_painter.text(
                s1 + Vec2::new(10.0, -10.0),
                Align2::LEFT_BOTTOM,
                format!("{:.0} mm", dist),
                egui::FontId::monospace(10.0),
                theme.accent_orange,
            );
        }

        // Window (blue L->R) vs Crossing (green R->L) Marquee Box Preview
        if let (Some(anchor), Some(cur)) = (self.marquee_anchor_mm, self.marquee_current_mm) {
            let s0 = to_screen(anchor);
            let s1 = to_screen(cur);
            let mrect = Rect::from_two_pos(s0, s1);
            let is_window = cur[0] >= anchor[0];
            let (mcol, lbl) = if is_window {
                (theme.accent_blue, "Window (Enclosed)")
            } else {
                (theme.accent_green, "Crossing (Touch)")
            };
            clip_painter.rect_filled(mrect, CornerRadius::ZERO, mcol.gamma_multiply(0.14));
            clip_painter.rect_stroke(mrect, CornerRadius::ZERO, Stroke::new(1.5, mcol), StrokeKind::Inside);
            clip_painter.text(
                mrect.left_top() + Vec2::new(4.0, -4.0),
                Align2::LEFT_BOTTOM,
                lbl,
                egui::FontId::monospace(9.5),
                mcol,
            );
        }

        // OSNAP Active Reticle Glyph (outside idle Grid)
        if self.osnap_enabled
            && !matches!(self.last_osnap_kind, OsnapKind::None | OsnapKind::Grid)
        {
            let sp = to_screen(self.cursor_world_mm);
            if rect.expand(10.0).contains(sp) {
                let col = match self.last_osnap_kind {
                    OsnapKind::Endpoint => theme.accent_green,
                    OsnapKind::Midpoint => theme.accent_blue,
                    OsnapKind::Perpendicular => theme.accent_orange,
                    OsnapKind::Intersection => theme.accent_red,
                    OsnapKind::None | OsnapKind::Grid => theme.text_muted,
                };
                let st = Stroke::new(1.6, col);
                match self.last_osnap_kind {
                    OsnapKind::Endpoint => {
                        clip_painter.rect_stroke(
                            Rect::from_center_size(sp, Vec2::splat(9.0)),
                            CornerRadius::ZERO,
                            st,
                            StrokeKind::Middle,
                        );
                    }
                    OsnapKind::Midpoint => {
                        let tri = vec![
                            sp + Vec2::new(0.0, -5.5),
                            sp + Vec2::new(5.5, 4.5),
                            sp + Vec2::new(-5.5, 4.5),
                        ];
                        clip_painter.add(egui::Shape::closed_line(tri, st));
                    }
                    OsnapKind::Perpendicular => {
                        clip_painter.line_segment([sp + Vec2::new(-5.0, -5.0), sp + Vec2::new(-5.0, 5.0)], st);
                        clip_painter.line_segment([sp + Vec2::new(-5.0, 5.0), sp + Vec2::new(5.0, 5.0)], st);
                    }
                    OsnapKind::Intersection => {
                        clip_painter.line_segment([sp + Vec2::new(-5.0, -5.0), sp + Vec2::new(5.0, 5.0)], st);
                        clip_painter.line_segment([sp + Vec2::new(-5.0, 5.0), sp + Vec2::new(5.0, -5.0)], st);
                    }
                    OsnapKind::None | OsnapKind::Grid => {}
                }
                clip_painter.text(
                    sp + Vec2::new(9.0, -7.0),
                    Align2::LEFT_BOTTOM,
                    self.last_osnap_kind.short_tag(),
                    egui::FontId::monospace(9.0),
                    col,
                );
            }
        }
    }

    /// Pure-vector CPU fallback rendering GPU compute multi-hazard particles when WebGPU is inactive.
    pub(super) fn paint_cpu_hazard_particles_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
        hc_input: &crate::views::webgpu_earth::HazardComputeInput,
    ) {
        if !hc_input.enabled || self.world_map.zoom_level < 12.0 {
            return;
        }
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let mut preview_in = hc_input.clone();
        preview_in.params.particle_count_f32 = preview_in.params.particle_count_f32.min(256.0);
        let verts = crate::views::webgpu_earth::step_hazard_particles_cpu(&preview_in);
        let clip = painter.with_clip_rect(rect);
        for tri in verts.chunks_exact(6) {
            let p0 = tri[0].pos;
            let p1 = tri[1].pos;
            let ll0 = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [(p0[0] as f64) * 1000.0, (p0[1] as f64) * 1000.0],
            );
            let ll1 = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [(p1[0] as f64) * 1000.0, (p1[1] as f64) * 1000.0],
            );
            if let (Some((s0, _, v0)), Some((s1, _, v1))) = (
                xform.project_lat_lon_elev(ll0.0, ll0.1, p0[2] as f64),
                xform.project_lat_lon_elev(ll1.0, ll1.1, p1[2] as f64),
            ) {
                if v0 && v1 {
                    let c = tri[0].color;
                    let col = Color32::from_rgba_unmultiplied(
                        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[3].clamp(0.0, 1.0) * 200.0) as u8,
                    );
                    clip.line_segment([s0, s1], Stroke::new(1.8, col));
                }
            }
        }
    }

    /// Update `v0.8.5.6` (`6/10` -> `v0.8.6`): Pure-vector CPU fallback rendering 3D Zoning Buildable Envelope & Sun Path.
    pub(super) fn paint_cpu_zoning_envelope_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.zoning_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_zoning_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for p in &report.envelope_prisms_3d {
            let col = match p.tier_kind {
                0 => Color32::from_rgb(91, 180, 216),
                1 => Color32::from_rgb(56, 189, 248),
                2 => Color32::from_rgb(16, 185, 129),
                _ => Color32::from_rgb(239, 68, 68),
            };
            let bot: Vec<Pos2> = [
                [p.min_enu_m[0], p.min_enu_m[1], p.min_enu_m[2]],
                [p.max_enu_m[0], p.min_enu_m[1], p.min_enu_m[2]],
                [p.max_enu_m[0], p.max_enu_m[1], p.min_enu_m[2]],
                [p.min_enu_m[0], p.max_enu_m[1], p.min_enu_m[2]],
            ]
            .into_iter()
            .filter_map(proj_enu)
            .collect();
            let top: Vec<Pos2> = [
                [p.min_enu_m[0], p.min_enu_m[1], p.max_enu_m[2]],
                [p.max_enu_m[0], p.min_enu_m[1], p.max_enu_m[2]],
                [p.max_enu_m[0], p.max_enu_m[1], p.max_enu_m[2]],
                [p.min_enu_m[0], p.max_enu_m[1], p.max_enu_m[2]],
            ]
            .into_iter()
            .filter_map(proj_enu)
            .collect();
            if bot.len() == 4 {
                clip.add(egui::Shape::closed_line(bot.clone(), Stroke::new(1.5, col)));
            }
            if top.len() == 4 {
                clip.add(egui::Shape::closed_line(top.clone(), Stroke::new(2.0, col)));
            }
            if bot.len() == 4 && top.len() == 4 && p.tier_kind > 0 {
                for i in 0..4 {
                    clip.line_segment([bot[i], top[i]], Stroke::new(1.4, col));
                }
            }
        }

        if report.config.show_sun_path {
            let sun_pts: Vec<Pos2> = report
                .sun_path_arc_3d
                .iter()
                .filter_map(|sp| proj_enu(sp.enu_pos_m))
                .collect();
            for win in sun_pts.windows(2) {
                clip.line_segment([win[0], win[1]], Stroke::new(2.0, Color32::from_rgb(250, 204, 21)));
            }
            if let Some(sun_sp) = proj_enu(report.active_sun_pos_3d) {
                clip.circle_filled(sun_sp, 6.0, Color32::from_rgb(250, 204, 21));
                if let Some(org_sp) = proj_enu([0.0, 0.0, 0.5]) {
                    clip.line_segment(
                        [sun_sp, org_sp],
                        Stroke::new(1.4, Color32::from_rgba_unmultiplied(250, 204, 21, 130)),
                    );
                }
            }
        }
    }

    /// Update `v0.8.5.7` (`7/10` -> `v0.8.6`): Pure-vector CPU fallback rendering stacked & packed 3D space blocks.
    pub(super) fn paint_cpu_spatial_program_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.program_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_program_report else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        for blk in &report.packed_blocks_3d {
            let is_focused = report
                .config
                .selected_story_filter
                .is_none_or(|f| f == blk.story_index);
            let alpha = if is_focused { 180_u8 } else { 45_u8 };
            let c = blk.color_rgba;
            let stroke_col = Color32::from_rgba_unmultiplied(
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                alpha,
            );
            let fill_col = Color32::from_rgba_unmultiplied(
                stroke_col.r(),
                stroke_col.g(),
                stroke_col.b(),
                alpha / 3,
            );
            let top: Vec<Pos2> = [
                [blk.min_enu_m[0], blk.min_enu_m[1], blk.max_enu_m[2]],
                [blk.max_enu_m[0], blk.min_enu_m[1], blk.max_enu_m[2]],
                [blk.max_enu_m[0], blk.max_enu_m[1], blk.max_enu_m[2]],
                [blk.min_enu_m[0], blk.max_enu_m[1], blk.max_enu_m[2]],
            ]
            .into_iter()
            .filter_map(proj_enu)
            .collect();
            if top.len() == 4 {
                clip.add(egui::Shape::convex_polygon(
                    top,
                    fill_col,
                    Stroke::new(if is_focused { 1.6 } else { 0.8 }, stroke_col),
                ));
            }
        }
    }

    /// Update `v0.8.5.8` (`8/10` -> `v0.8.6`): Pure-vector CPU fallback rendering Multi-Option Architectural Massing.
    pub(super) fn paint_cpu_massing_study_fallback(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        xform: &crate::core::gis::GlobeScreenTransform,
    ) {
        if !self.massing_overlay_visible || self.world_map.zoom_level < 12.0 {
            return;
        }
        let Some(ref report) = self.last_massing_report else {
            return;
        };
        let Some(opt) = report.active_option() else {
            return;
        };
        let clip = painter.with_clip_rect(rect);
        let origin = if self.ground_origin_anchored {
            self.ground_origin_lat_lon
        } else {
            self.world_map.center_lat_lon
        };
        let heading_deg = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let rad = heading_deg.to_radians();
        let (sr, cr) = (rad.sin(), rad.cos());
        let proj_enu = |p: [f64; 3]| -> Option<Pos2> {
            let ex = p[0] * cr - p[1] * sr;
            let ny = p[0] * sr + p[1] * cr;
            let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                origin,
                [ex * 1000.0, ny * 1000.0],
            );
            xform
                .project_lat_lon_elev(ll.0, ll.1, p[2])
                .and_then(|(sp, _, vis)| if vis { Some(sp) } else { None })
        };

        let story_h = report.story_height_m.clamp(2.8, 6.0);
        for blk in &opt.blocks_3d {
            let c = blk.color_rgba;
            let stroke_col = Color32::from_rgba_unmultiplied(
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                if blk.is_core { 220 } else { 195 },
            );
            let fill_col = Color32::from_rgba_unmultiplied(
                stroke_col.r(),
                stroke_col.g(),
                stroke_col.b(),
                if blk.is_core { 95 } else { 52 },
            );
            let end_s = if blk.is_core { opt.stories } else { blk.end_story };
            let start_s = if blk.is_core { 1 } else { blk.start_story + 1 };
            for s in start_s..=end_s {
                let sz = (s as f64) * story_h;
                if sz > blk.max_enu_m[2] + 0.05 {
                    break;
                }
                let ring: Vec<Pos2> = [
                    [blk.min_enu_m[0], blk.min_enu_m[1], sz],
                    [blk.max_enu_m[0], blk.min_enu_m[1], sz],
                    [blk.max_enu_m[0], blk.max_enu_m[1], sz],
                    [blk.min_enu_m[0], blk.max_enu_m[1], sz],
                ]
                .into_iter()
                .filter_map(proj_enu)
                .collect();
                if ring.len() == 4 {
                    clip.add(egui::Shape::convex_polygon(
                        ring,
                        if s == end_s { fill_col } else { Color32::TRANSPARENT },
                        Stroke::new(if s == end_s { 1.8 } else { 0.8 }, stroke_col),
                    ));
                }
            }
        }
    }
}
