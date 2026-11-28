//! Earth 3D orbit/zoom interaction, sketching, and screen-space BIM/world picking.

use super::*;
use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Stroke, Ui, Vec2};

use crate::core::gis::SlippyMapState;
use crate::core::BimProject;
use crate::models::NavigationPillar;
use crate::theme::MacTheme;

impl UniversalCanvasState {
    pub(super) fn apply_globe_zoom_at_screen(&mut self, rect: Rect, raw_dz: f32, focus_pos: Option<Pos2>) {
        let z_curr = self.world_map.zoom_level;
        // Gently taper scroll/pinch increment when zoomed close to city/site scale
        let zoom_taper = 1.0 - ((z_curr - 8.0) / 22.0).clamp(0.0, 0.42);
        let dz = (raw_dz * zoom_taper).clamp(-0.28, 0.28);
        if dz.abs() <= 1e-5 {
            return;
        }

        self.earth_world.camera.fly_to = None;
        self.earth_world.camera.angular_vel_lat = 0.0;
        self.earth_world.camera.angular_vel_lon = 0.0;

        let pre_hit = focus_pos.map(|pos| {
            let pre_xf = self
                .last_globe_xform
                .as_ref()
                .filter(|x| x.rect == rect)
                .cloned()
                .unwrap_or_else(|| {
                    GlobeScreenTransform::from_view(
                        rect,
                        self.world_map.center_lat_lon.0,
                        self.world_map.center_lat_lon.1,
                        self.world_map.zoom_level,
                        &self.earth_world.camera,
                    )
                });
            (pos, pre_xf.screen_to_lat_lon(pos))
        });

        let old_center = self.world_map.center_lat_lon;
        self.world_map.zoom_level = (self.world_map.zoom_level + dz).clamp(1.0, 20.5);

        if let Some((pos, (lat_before, lon_before, hit_before))) = pre_hit {
            if hit_before {
                let post_xf = GlobeScreenTransform::from_view(
                    rect,
                    old_center.0,
                    old_center.1,
                    self.world_map.zoom_level,
                    &self.earth_world.camera,
                );
                let (lat_after, lon_after, hit_after) = post_xf.screen_to_lat_lon(pos);
                if hit_after {
                    let max_step_deg = (post_xf.d_surf * 65.0).clamp(1e-4, 18.0);
                    let dlat = (lat_before - lat_after).clamp(-max_step_deg, max_step_deg);
                    let mut dlon = lon_before - lon_after;
                    if dlon > 180.0 {
                        dlon -= 360.0;
                    } else if dlon < -180.0 {
                        dlon += 360.0;
                    }
                    let dlon = dlon.clamp(-max_step_deg * 1.4, max_step_deg * 1.4);
                    self.world_map
                        .set_center((old_center.0 + dlat).clamp(-85.0, 85.0), old_center.1 + dlon);
                }
            }
        }

        self.last_globe_xform = Some(GlobeScreenTransform::from_view(
            rect,
            self.world_map.center_lat_lon.0,
            self.world_map.center_lat_lon.1,
            self.world_map.zoom_level,
            &self.earth_world.camera,
        ));
    }

    pub(super) fn orbit_globe_by_pixels(&mut self, rect: Rect, delta: Vec2, alt_held: bool, dt: f32) {
        self.earth_world.camera.fly_to = None;
        if alt_held {
            self.earth_world.camera.tilt_deg =
                (self.earth_world.camera.tilt_deg - delta.y * 0.30).clamp(0.0, 65.0);
            self.earth_world.camera.heading_deg =
                (self.earth_world.camera.heading_deg + delta.x * 0.38).rem_euclid(360.0);
            self.earth_world.camera.angular_vel_lat = 0.0;
            self.earth_world.camera.angular_vel_lon = 0.0;
            return;
        }
        let xform = self
            .last_globe_xform
            .as_ref()
            .filter(|x| x.rect == rect)
            .cloned()
            .unwrap_or_else(|| {
                GlobeScreenTransform::from_view(
                    rect,
                    self.world_map.center_lat_lon.0,
                    self.world_map.center_lat_lon.1,
                    self.world_map.zoom_level,
                    &self.earth_world.camera,
                )
            });
        let dpp_x = xform.deg_per_pixel();
        let dpp_y = xform.deg_per_pixel_y();
        let cos_lat = self
            .world_map
            .center_lat_lon
            .0
            .to_radians()
            .cos()
            .abs()
            .max(0.25);
        let hr = self.earth_world.camera.heading_deg.to_radians() as f64;
        let dx_h = (delta.x as f64) * dpp_x;
        let dy_h = -(delta.y as f64) * dpp_y;
        let d_east = dx_h * hr.cos() + dy_h * hr.sin();
        let d_north = -dx_h * hr.sin() + dy_h * hr.cos();
        let dlon = -d_east / cos_lat;
        let dlat = -d_north;
        let nlat = (self.world_map.center_lat_lon.0 + dlat).clamp(-85.0, 85.0);
        let nlon = self.world_map.center_lat_lon.1 + dlon;
        self.world_map.set_center(nlat, nlon);
        self.last_globe_xform = Some(GlobeScreenTransform::from_view(
            rect,
            self.world_map.center_lat_lon.0,
            self.world_map.center_lat_lon.1,
            self.world_map.zoom_level,
            &self.earth_world.camera,
        ));
        if self.world_map.zoom_level < 11.0 {
            let inv_dt = 1.0 / dt.max(0.008);
            self.earth_world.camera.angular_vel_lat =
                (dlat as f32 * inv_dt * 0.28).clamp(-35.0, 35.0);
            self.earth_world.camera.angular_vel_lon =
                (dlon as f32 * inv_dt * 0.28).clamp(-45.0, 45.0);
        } else {
            self.earth_world.camera.angular_vel_lat = 0.0;
            self.earth_world.camera.angular_vel_lon = 0.0;
        }
    }

    pub(super) fn handle_canvas_interaction(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        rect: Rect,
        active_pillar: NavigationPillar,
        bim_project: &BimProject,
        pointer_in_inspector: bool,
    ) {
        if self.is_frozen || pointer_in_inspector {
            self.pending_start_mm = None;
            self.pending_earth_arc_start = None;
            self.marquee_anchor_mm = None;
            self.marquee_current_mm = None;
            self.active_freehand_mm.clear();
            return;
        }

        // Scroll Wheel & Pinch Zoom around cursor on 3D Globe or 2D World Map
        if response.hovered() {
            let (scroll_y, pinch_factor) =
                ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let pinch_dz = if (pinch_factor - 1.0).abs() > 0.002 {
                pinch_factor.log2()
            } else {
                0.0
            };
            let wheel_dz = if scroll_y.abs() > 0.2 {
                scroll_y * 0.0022
            } else {
                0.0
            };
            let raw_dz = wheel_dz + pinch_dz;
            if raw_dz.abs() > 1e-4 {
                if self.globe_mode {
                    self.apply_globe_zoom_at_screen(rect, raw_dz, response.hover_pos());
                } else {
                    let zoom_taper =
                        1.0 - ((self.world_map.zoom_level - 8.0) / 22.0).clamp(0.0, 0.42);
                    let dz = (raw_dz * zoom_taper).clamp(-0.28, 0.28);
                    self.world_map
                        .zoom_at_screen_pos(dz, response.hover_pos(), rect);
                }
            }
        }

        let scale = self.world_map.pixels_per_mm_at_origin(self.ground_origin_lat_lon);
        let hit_radius_mm = (18.0 / scale.max(1e-9)) as f64;
        self.last_osnap_radius_mm = hit_radius_mm.clamp(250.0, 2500.0);

        if let Some(hover) = response.hover_pos() {
            let (lat, lon, _) = self.screen_to_lat_lon_with_hit(hover, rect);
            self.cursor_lat_lon = (lat, lon);
            let raw_mm = self.screen_to_world(rect, hover);
            let (snapped, kind) = self.snap_point_with_osnap(
                raw_mm,
                self.pending_start_mm,
                Some(bim_project),
                self.last_osnap_radius_mm,
            );
            self.cursor_world_mm = snapped;
            self.last_osnap_kind = kind;
        }

        let (shift_held, alt_held, dt) =
            ui.input(|i| (i.modifiers.shift, i.modifiers.alt, i.unstable_dt));

        // Right-drag, Middle-drag, Shift-drag, or Alt-drag orbits/tilts 3D Globe or pans 2D map in any tool
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged() && (shift_held || alt_held))
        {
            let delta = response.drag_delta();
            if self.globe_mode {
                self.orbit_globe_by_pixels(rect, delta, alt_held, dt);
            } else {
                self.world_map.pan_by_pixels(delta.x, delta.y);
            }
            return;
        }

        if self.measure_active && response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                if hit {
                    self.measure_points_lat_lon.push((lat, lon));
                }
            }
            return;
        }

        // Update v0.8.6.6 (6/10 -> v0.8.7): Live Interactive 3D Terrain & Water Sculpting (`IfcSite` Terraform)
        if let Some(sculpt_brush) = self.terrain_sculpt.active_brush {
            if let Some(hover) = response.hover_pos().or_else(|| response.interact_pointer_pos()) {
                let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(hover, rect);
                if hit {
                    let mm = crate::core::gis::SlippyMapState::lat_lon_to_world_mm(
                        self.ground_origin_lat_lon,
                        lat,
                        lon,
                    );
                    self.terrain_sculpt_hover_enu =
                        Some([(mm[0] * 0.001) as f32, (mm[1] * 0.001) as f32]);
                } else {
                    self.terrain_sculpt_hover_enu = None;
                }
            } else {
                self.terrain_sculpt_hover_enu = None;
            }

            let stroke_start = response.drag_started_by(egui::PointerButton::Primary)
                || response.clicked_by(egui::PointerButton::Primary);
            let dragging = response.dragged_by(egui::PointerButton::Primary);
            if stroke_start || dragging {
                if let Some(pos) = response
                    .hover_pos()
                    .or_else(|| response.interact_pointer_pos())
                {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        self.show_terrain_contours = true;
                        self.realistic_landscape_enabled = true;
                        self.ensure_site_tin_generated();
                        let mm = crate::core::gis::SlippyMapState::lat_lon_to_world_mm(
                            self.ground_origin_lat_lon,
                            lat,
                            lon,
                        );
                        let cx_m = (mm[0] * 0.001) as f32;
                        let cy_m = (mm[1] * 0.001) as f32;
                        self.terrain_sculpt_hover_enu = Some([cx_m, cy_m]);

                        if let Some(ref mut tin) = self.active_site_tin {
                            if stroke_start {
                                let snap: Vec<f32> = tin.points.iter().map(|p| p.z).collect();
                                self.terrain_sculpt_undo_stack.push(snap);
                                if self.terrain_sculpt_undo_stack.len() > 24 {
                                    self.terrain_sculpt_undo_stack.remove(0);
                                }
                                if sculpt_brush == crate::core::gis::TerrainSculptBrush::Flatten
                                    && self.terrain_sculpt.auto_sample_flatten
                                {
                                    self.terrain_sculpt.target_flatten_z_m =
                                        tin.sample_elevation_and_slope(cx_m, cy_m).0;
                                }
                            }
                            let eff_strength = if dragging && !stroke_start {
                                self.terrain_sculpt.brush_strength_m * 0.35
                            } else {
                                self.terrain_sculpt.brush_strength_m
                            };
                            let res = tin.apply_sculpt_brush(
                                cx_m,
                                cy_m,
                                sculpt_brush,
                                self.terrain_sculpt.brush_radius_m,
                                eff_strength,
                                self.terrain_sculpt.target_flatten_z_m,
                                self.sea_level_m,
                            );
                            if sculpt_brush == crate::core::gis::TerrainSculptBrush::CarveWater {
                                self.show_water_surface = true;
                            }
                            self.terrain_sculpt.total_cut_m3 += res.delta_cut_m3;
                            self.terrain_sculpt.total_fill_m3 += res.delta_fill_m3;
                            self.terrain_sculpt.stroke_count =
                                self.terrain_sculpt.stroke_count.saturating_add(1);
                            self.status_message = Some(format!(
                                "Sculpt {} @ (E{:+.0}m, N{:+.0}m) • Z={:+.1}m • Cut {:.0} m³ / Fill {:.0} m³ • Max Slope {:.1}°",
                                sculpt_brush.short_label(),
                                cx_m,
                                cy_m,
                                res.center_elev_after_m,
                                self.terrain_sculpt.total_cut_m3,
                                self.terrain_sculpt.total_fill_m3,
                                res.max_slope_deg_after
                            ));
                        }
                    }
                }
            }
            return;
        }
        self.terrain_sculpt_hover_enu = None;

        // Phase 43: Earth Terraforming Brushes & Planetary Megaproject Tools
        if let Some(brush) = self.active_tool.earth_brush_action() {
            let stroke_start = response.drag_started_by(egui::PointerButton::Primary)
                || response.clicked_by(egui::PointerButton::Primary);
            let dragging = response.dragged_by(egui::PointerButton::Primary);
            if stroke_start || dragging {
                if let Some(pos) = response
                    .hover_pos()
                    .or_else(|| response.interact_pointer_pos())
                {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        let count = self.earth_world.apply_brush(lat, lon, brush, stroke_start);
                        if count > 0 || stroke_start {
                            let new_el = self.earth_world.effective_elevation_m(lat, lon);
                            self.status_message = Some(format!(
                                "{} @ {:.2}°, {:.2}° (Elev {:+.0}m • CO2 {:.0} ppm • Hab {:.0}/100)",
                                brush.label(),
                                lat,
                                lon,
                                new_el,
                                self.earth_world.vitals.co2_ppm,
                                self.earth_world.vitals.habitability_score,
                            ));
                        }
                    }
                }
            }
            return;
        }

        if let Some(kind) = self.active_tool.earth_megastructure_kind() {
            use crate::core::gis::MegastructureKind;
            if kind == MegastructureKind::ArcologyMegaTower {
                if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                        if hit {
                            let id = self.earth_world.add_megastructure(kind, (lat, lon), (lat, lon));
                            self.status_message = Some(format!(
                                "Constructed Mega-Tower #{id} at {:.2}°, {:.2}° (Habitability {:.0}/100)",
                                lat, lon, self.earth_world.vitals.habitability_score
                            ));
                        }
                    }
                }
                return;
            }
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pos) = response
                    .hover_pos()
                    .or_else(|| response.interact_pointer_pos())
                {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        self.pending_earth_arc_start = Some((lat, lon));
                    }
                }
            } else if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let Some(start_ll) = self.pending_earth_arc_start.take() {
                    if let Some(pos) = response
                        .hover_pos()
                        .or_else(|| response.interact_pointer_pos())
                    {
                        let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                        if hit {
                            let id = self.earth_world.add_megastructure(kind, start_ll, (lat, lon));
                            if let Some(m) = self.earth_world.megastructures.last() {
                                self.status_message = Some(format!(
                                    "Built {} (#{id}): {:.0} km (${:.1}B, {:.1} Mt/yr CO2)",
                                    m.name, m.length_km, m.cost_b_usd, m.carbon_delta_mt_yr
                                ));
                            }
                        }
                    }
                }
            } else if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        if let Some(start_ll) = self.pending_earth_arc_start.take() {
                            let id = self.earth_world.add_megastructure(kind, start_ll, (lat, lon));
                            if let Some(m) = self.earth_world.megastructures.last() {
                                self.status_message = Some(format!(
                                    "Built {} (#{id}): {:.0} km",
                                    m.name, m.length_km
                                ));
                            }
                        } else {
                            self.pending_earth_arc_start = Some((lat, lon));
                            self.status_message = Some(format!(
                                "{} start anchored ({:.2}°, {:.2}°) — click or drag destination",
                                kind.label(),
                                lat,
                                lon
                            ));
                        }
                    }
                }
            }
            return;
        }

        let world_map_snapshot = self.world_map.clone();
        let origin_lat_lon = self.ground_origin_lat_lon;
        let globe_xform_snapshot = if self.globe_mode {
            Some(
                self.last_globe_xform
                    .as_ref()
                    .filter(|x| x.rect == rect)
                    .cloned()
                    .unwrap_or_else(|| {
                        GlobeScreenTransform::from_view(
                            rect,
                            world_map_snapshot.center_lat_lon.0,
                            world_map_snapshot.center_lat_lon.1,
                            world_map_snapshot.zoom_level,
                            &self.earth_world.camera,
                        )
                    }),
            )
        } else {
            None
        };
        let to_world = |pos: Pos2| -> [f64; 2] {
            if let Some(ref xf) = globe_xform_snapshot {
                let (lat, lon, _) = xf.screen_to_lat_lon(pos);
                SlippyMapState::lat_lon_to_world_mm(origin_lat_lon, lat, lon)
            } else {
                world_map_snapshot.screen_to_world_mm(origin_lat_lon, pos, rect)
            }
        };

        // Phase 57 & 59: Interactive "Place on Earth" Ghost Footprint mode + Vector Edge/Parcel OSNAP
        if self.globe_mode && self.placing_unlocated_rel_path.is_some() {
            let resolve_place_coord = |raw_lat: f64, raw_lon: f64, skip_snap: bool| -> ((f64, f64), Option<f64>) {
                if !skip_snap {
                    if let Some((sla, slo, az)) = self
                        .urban_vectors
                        .snap_lat_lon_to_vector_edge((raw_lat, raw_lon), 18.0)
                    {
                        return ((sla, slo), Some(az));
                    }
                }
                ((raw_lat, raw_lon), None)
            };
            if let Some(hover) = response.hover_pos() {
                let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(hover, rect);
                if hit {
                    let ((pla, plo), az_opt) = resolve_place_coord(lat, lon, alt_held);
                    self.placing_hover_lat_lon = Some((pla, plo));
                    self.active_vector_snap_az_deg = az_opt;
                    self.cursor_lat_lon = (pla, plo);
                } else {
                    self.placing_hover_lat_lon = None;
                    self.active_vector_snap_az_deg = None;
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Q)) {
                self.placing_heading_deg = (self.placing_heading_deg - 15.0).rem_euclid(360.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::E)) {
                self.placing_heading_deg = (self.placing_heading_deg + 15.0).rem_euclid(360.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::A)) {
                if let Some(az) = self.active_vector_snap_az_deg {
                    self.placing_heading_deg = az.rem_euclid(360.0);
                }
            }
            if response.secondary_clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.placing_unlocated_rel_path = None;
                self.placing_hover_lat_lon = None;
                self.active_vector_snap_az_deg = None;
                return;
            }
            if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                    if hit {
                        let ((pla, plo), _) = resolve_place_coord(lat, lon, alt_held);
                        if let Some(rel) = self.placing_unlocated_rel_path.take() {
                            let hdg = self.placing_heading_deg;
                            self.world_index.pin_building_at(&rel, pla, plo, hdg);
                            if self.active_world_project_rel_path.as_deref() == Some(rel.as_str()) {
                                self.ground_origin_lat_lon = (pla, plo);
                                self.ground_origin_anchored = true;
                            }
                            self.pending_world_pin_event = Some((rel.clone(), pla, plo, hdg));
                            self.status_message = Some(format!(
                                "Pinned building '{}' onto Earth @ ({:.4}°, {:.4}°), heading {:.0}°",
                                rel.split('/').next_back().unwrap_or(&rel),
                                pla,
                                plo,
                                hdg
                            ));
                        }
                        self.placing_hover_lat_lon = None;
                        self.active_vector_snap_az_deg = None;
                    }
                }
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                self.orbit_globe_by_pixels(rect, delta, alt_held, dt);
            }
            return;
        }

        // Phase 55: Direct 3D BIM Placement (Column / Beam / Wall / Footing) straight onto Earth 3D surface
        if self.globe_mode && self.earth_bim_author.active_tool != EarthBimTool::None {
            let bim_snap = |uc: &Self, raw: [f64; 2], anchor_start: Option<[f64; 2]>| -> [f64; 2] {
                let (mut pt, _) = uc.snap_point_with_osnap(
                    raw,
                    anchor_start,
                    Some(bim_project),
                    uc.last_osnap_radius_mm.max(450.0),
                );
                if let Some(s) = anchor_start {
                    if uc.ortho_mode {
                        let dx = (pt[0] - s[0]).abs();
                        let dy = (pt[1] - s[1]).abs();
                        if dx >= dy {
                            pt[1] = s[1];
                        } else {
                            pt[0] = s[0];
                        }
                    }
                }
                pt
            };

            if let Some(hover) = response.hover_pos() {
                let snapped = bim_snap(self, to_world(hover), self.earth_bim_author.pending_start_mm);
                self.earth_bim_author.hover_snap_mm = Some(snapped);
                self.cursor_world_mm = snapped;
            } else {
                self.earth_bim_author.hover_snap_mm = None;
            }

            if response.secondary_clicked() {
                self.earth_bim_author.pending_start_mm = None;
            }

            if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let was_anchored = self.ground_origin_anchored;
                    self.ensure_ground_origin_at_screen(pos, rect);
                    let active_origin = self.ground_origin_lat_lon;
                    let raw = if !was_anchored {
                        if let Some(ref xf) = globe_xform_snapshot {
                            let (lat, lon, _) = xf.screen_to_lat_lon(pos);
                            SlippyMapState::lat_lon_to_world_mm(active_origin, lat, lon)
                        } else {
                            world_map_snapshot.screen_to_world_mm(active_origin, pos, rect)
                        }
                    } else {
                        to_world(pos)
                    };
                    let mut pt = bim_snap(self, raw, self.earth_bim_author.pending_start_mm);
                    // Phase 67: Road & Parcel Setback Snapping for BIM columns/walls/footings
                    if matches!(
                        self.earth_bim_author.active_tool,
                        EarthBimTool::Column | EarthBimTool::Wall | EarthBimTool::Footing
                    ) {
                        if let Some(hit) = self
                            .city_scene
                            .snap_point_to_road_or_parcel([pt[0] * 0.001, pt[1] * 0.001], 3.5)
                        {
                            pt = [hit.snapped_enu_m[0] * 1000.0, hit.snapped_enu_m[1] * 1000.0];
                            self.active_vector_snap_az_deg = Some(hit.bearing_deg);
                        }
                    }
                    let base_level_id = self.earth_bim_author.base_level_id.max(1);
                    let top_level_id = self.earth_bim_author.top_level_id.max(base_level_id);
                    let p2 = crate::core::P2(pt[0], pt[1]);

                    match self.earth_bim_author.active_tool {
                        EarthBimTool::Column => {
                            let w = self.earth_bim_author.col_width_mm.clamp(150.0, 2000.0);
                            let d = self.earth_bim_author.col_depth_mm.clamp(150.0, 2000.0);
                            self.pending_earth_bim_commands.push(crate::core::BimCommand::AddColumn {
                                base_level_id,
                                top_level_id,
                                pos: p2,
                                width_mm: w,
                                depth_mm: d,
                                is_round: false,
                                profile_name: format!("C{:.0}x{:.0}", w, d),
                            });
                            self.status_message = Some(format!(
                                "Placed Column ({w:.0}x{d:.0}mm) on Earth 3D @ ({:.2}m, {:.2}m)",
                                pt[0] * 0.001,
                                pt[1] * 0.001
                            ));
                        }
                        EarthBimTool::Footing => {
                            let fs = self.earth_bim_author.footing_size_mm.clamp(400.0, 6000.0);
                            self.pending_earth_bim_commands.push(crate::core::BimCommand::AddFooting {
                                level_id: base_level_id,
                                pos: p2,
                                width_mm: fs,
                                length_mm: fs,
                                thickness_mm: 450.0,
                            });
                            self.status_message = Some(format!(
                                "Placed Footing ({fs:.0}mm) on Earth 3D @ ({:.2}m, {:.2}m)",
                                pt[0] * 0.001,
                                pt[1] * 0.001
                            ));
                        }
                        EarthBimTool::Wall | EarthBimTool::Beam => {
                            if let Some(s) = self.earth_bim_author.pending_start_mm.take() {
                                if (s[0] - pt[0]).hypot(s[1] - pt[1]) >= 200.0 {
                                    if self.earth_bim_author.active_tool == EarthBimTool::Wall {
                                        let wt = self.earth_bim_author.wall_thickness_mm.clamp(80.0, 1200.0);
                                        self.pending_earth_bim_commands.push(crate::core::BimCommand::AddWall {
                                            base_level_id,
                                            start: crate::core::P2(s[0], s[1]),
                                            end: p2,
                                            thickness_mm: wt,
                                            is_structural: true,
                                        });
                                        self.status_message = Some(format!(
                                            "Placed Wall ({wt:.0}mm) on Earth 3D (L={:.2}m)",
                                            (s[0] - pt[0]).hypot(s[1] - pt[1]) * 0.001
                                        ));
                                    } else {
                                        let bw = self.earth_bim_author.beam_width_mm.clamp(120.0, 1500.0);
                                        let bd = self.earth_bim_author.beam_depth_mm.clamp(150.0, 2500.0);
                                        self.pending_earth_bim_commands.push(crate::core::BimCommand::AddBeamRun {
                                            level_id: top_level_id,
                                            category: crate::core::RunCategory::StructuralBeam,
                                            start: crate::core::P2(s[0], s[1]),
                                            end: p2,
                                            width_mm: bw,
                                            depth_mm: bd,
                                            justify: crate::core::RunJustify::Top,
                                            profile_name: format!("B{:.0}x{:.0}", bw, bd),
                                        });
                                        self.status_message = Some(format!(
                                            "Placed Beam ({bw:.0}x{bd:.0}mm) on Earth 3D (L={:.2}m)",
                                            (s[0] - pt[0]).hypot(s[1] - pt[1]) * 0.001
                                        ));
                                    }
                                }
                            } else {
                                self.earth_bim_author.pending_start_mm = Some(pt);
                            }
                        }
                        EarthBimTool::Slab => {
                            if let Some(s) = self.earth_bim_author.pending_start_mm.take() {
                                let min_x = s[0].min(pt[0]);
                                let max_x = s[0].max(pt[0]);
                                let min_y = s[1].min(pt[1]);
                                let max_y = s[1].max(pt[1]);
                                if (max_x - min_x) >= 300.0 && (max_y - min_y) >= 300.0 {
                                    self.pending_earth_bim_commands.push(crate::core::BimCommand::AddSlab {
                                        level_id: base_level_id,
                                        boundary: vec![
                                            crate::core::P2(min_x, min_y),
                                            crate::core::P2(max_x, min_y),
                                            crate::core::P2(max_x, max_y),
                                            crate::core::P2(min_x, max_y),
                                        ],
                                        openings: Vec::new(),
                                        thickness_mm: 150.0,
                                        is_roof: false,
                                        roof_slope_deg: 0.0,
                                    });
                                    self.status_message = Some(format!(
                                        "Placed Slab ({:.2} x {:.2} m) on Earth 3D Level #{}",
                                        (max_x - min_x) * 0.001,
                                        (max_y - min_y) * 0.001,
                                        base_level_id
                                    ));
                                }
                            } else {
                                self.earth_bim_author.pending_start_mm = Some(pt);
                            }
                        }
                        EarthBimTool::Door | EarthBimTool::Window => {
                            let is_door = self.earth_bim_author.active_tool == EarthBimTool::Door;
                            let target_wall_id = self
                                .pick_bim_element_at_screen(pos, rect, bim_project)
                                .and_then(|(k, id)| {
                                    if k == crate::core::BimElementKind::Wall {
                                        Some(id)
                                    } else {
                                        None
                                    }
                                })
                                .or_else(|| {
                                    // Proximity fallback to closest wall segment in XY world mm
                                    bim_project
                                        .walls
                                        .iter()
                                        .filter_map(|w| {
                                            let ab_x = w.end.0 - w.start.0;
                                            let ab_y = w.end.1 - w.start.1;
                                            let l2 = ab_x * ab_x + ab_y * ab_y;
                                            if l2 < 1.0 {
                                                return None;
                                            }
                                            let t = (((raw[0] - w.start.0) * ab_x + (raw[1] - w.start.1) * ab_y) / l2).clamp(0.0, 1.0);
                                            let d = ((raw[0] - (w.start.0 + t * ab_x)).powi(2)
                                                + (raw[1] - (w.start.1 + t * ab_y)).powi(2))
                                            .sqrt();
                                            if d <= 1200.0 { Some((w.id, d)) } else { None }
                                        })
                                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                                        .map(|(id, _)| id)
                                });
                            if let Some(wid) = target_wall_id {
                                if let Some(wall) = bim_project.walls.iter().find(|w| w.id == wid) {
                                    let wlen = wall.start.dist(wall.end).max(400.0);
                                    let ux = (wall.end.0 - wall.start.0) / wlen;
                                    let uy = (wall.end.1 - wall.start.1) / wlen;
                                    let proj_along = (raw[0] - wall.start.0) * ux + (raw[1] - wall.start.1) * uy;
                                    let (kind, width_mm, height_mm, sill_height_mm) = if is_door {
                                        (crate::core::bim::OpeningKind::SingleDoor, 900.0, 2100.0, 0.0)
                                    } else {
                                        (crate::core::bim::OpeningKind::CasementWindow, 1800.0, 1400.0, 900.0)
                                    };
                                    let offset_along_mm = (proj_along - width_mm * 0.5)
                                        .clamp(120.0, (wlen - width_mm - 120.0).max(120.0));
                                    self.pending_earth_bim_commands.push(crate::core::BimCommand::AddOpening {
                                        wall_id: wid,
                                        kind,
                                        offset_along_mm,
                                        width_mm,
                                        height_mm,
                                        sill_height_mm,
                                    });
                                    self.status_message = Some(format!(
                                        "Added {} to Wall #{} on Earth 3D",
                                        if is_door { "Door" } else { "Window" },
                                        wid
                                    ));
                                }
                            } else {
                                self.status_message = Some(format!(
                                    "Click near a 3D Wall on Earth 3D to insert a {}.",
                                    if is_door { "Door" } else { "Window" }
                                ));
                            }
                        }
                        EarthBimTool::Road | EarthBimTool::Bridge => {
                            let is_bridge = self.earth_bim_author.active_tool == EarthBimTool::Bridge;
                            let pt_m_raw = [pt[0] * 0.001, pt[1] * 0.001];
                            let snapped_m = self
                                .city_scene
                                .snap_road_endpoint(pt_m_raw, 12.0)
                                .unwrap_or(pt_m_raw);
                            let pt_snapped_mm = [snapped_m[0] * 1000.0, snapped_m[1] * 1000.0];
                            let kind = self.city_scene.active_road_kind;
                            let elev_off = self.earth_bim_author.road_elevation_offset_m;

                            if self.city_scene.road_draw_mode
                                == crate::core::gis::RoadDrawMode::CurvedSpline
                            {
                                if self.earth_bim_author.pending_start_mm.is_none() {
                                    self.earth_bim_author.pending_start_mm = Some(pt_snapped_mm);
                                    self.earth_bim_author.pending_road_control_mm = None;
                                    self.status_message = Some(
                                        "Spline Step 1/3 anchored — click 2nd point to bend curve"
                                            .to_string(),
                                    );
                                } else if self.earth_bim_author.pending_road_control_mm.is_none() {
                                    self.earth_bim_author.pending_road_control_mm =
                                        Some(pt_snapped_mm);
                                    self.status_message = Some(
                                        "Spline Step 2/3 bend set — click 3rd point to finish Road / Auto-Bridge"
                                            .to_string(),
                                    );
                                } else if let (Some(s_mm), Some(c_mm)) = (
                                    self.earth_bim_author.pending_start_mm.take(),
                                    self.earth_bim_author.pending_road_control_mm.take(),
                                ) {
                                    let start_m = [s_mm[0] * 0.001, s_mm[1] * 0.001];
                                    let ctrl_m = [c_mm[0] * 0.001, c_mm[1] * 0.001];
                                    if let Some(seg) = self.city_scene.add_curved_road_with_auto_bridge(
                                        start_m,
                                        Some(ctrl_m),
                                        snapped_m,
                                        kind,
                                        is_bridge,
                                        elev_off,
                                        self.active_site_tin.as_ref(),
                                        self.sea_level_m,
                                    ) {
                                        self.status_message = Some(format!(
                                            "Built Curved {} '{}' ({:.1} m, {:.1} m wide) as IFCX {}",
                                            if seg.is_bridge { "Auto-Bridge" } else { "Road" },
                                            seg.name,
                                            seg.length_m(),
                                            seg.width_m,
                                            if seg.is_bridge { "IfcBridge" } else { "IfcRoad" }
                                        ));
                                    }
                                }
                            } else if let Some(s_mm) = self.earth_bim_author.pending_start_mm.take() {
                                let start_m = [s_mm[0] * 0.001, s_mm[1] * 0.001];
                                self.earth_bim_author.pending_road_control_mm = None;
                                if let Some(seg) = self.city_scene.add_curved_road_with_auto_bridge(
                                    start_m,
                                    None,
                                    snapped_m,
                                    kind,
                                    is_bridge,
                                    elev_off,
                                    self.active_site_tin.as_ref(),
                                    self.sea_level_m,
                                ) {
                                    self.status_message = Some(format!(
                                        "Authored {} '{}' ({:.1} m, {:.1} m wide) as IFCX {}",
                                        if seg.is_bridge { "Bridge" } else { "Road" },
                                        seg.name,
                                        seg.length_m(),
                                        seg.width_m,
                                        if seg.is_bridge { "IfcBridge" } else { "IfcRoad" }
                                    ));
                                }
                            } else {
                                self.earth_bim_author.pending_start_mm = Some(pt_snapped_mm);
                                self.earth_bim_author.pending_road_control_mm = None;
                                self.status_message = Some(
                                    "Road Start anchored — click destination point".to_string(),
                                );
                            }
                        }
                        EarthBimTool::Zone => {
                            let raw_m = [pt[0] * 0.001, pt[1] * 0.001];
                            let snap_hit = self.city_scene.snap_point_to_road_or_parcel(raw_m, 22.0);
                            let center_m = snap_hit.map(|h| h.snapped_enu_m).unwrap_or(raw_m);
                            let bearing = snap_hit.map(|h| h.bearing_deg).unwrap_or(0.0);
                            let submerged = self.active_site_tin.as_ref().is_some_and(|t| {
                                t.sample_elevation_and_slope(center_m[0] as f32, center_m[1] as f32).0
                                    < self.sea_level_m + 0.3
                            });
                            if submerged {
                                self.status_message = Some(
                                    "Cannot zone submerged water — sculpt land or reclaim seabed first"
                                        .to_string(),
                                );
                            } else {
                                let kind = self.city_scene.active_zone_kind;
                                let (z_name, z_kind_lbl, floors_opt) = {
                                    let z = self.city_scene.add_zone(
                                        center_m,
                                        28.0,
                                        24.0,
                                        bearing,
                                        kind,
                                    );
                                    let fl = z.synthesize_procedural_building().map(|b| b.floors);
                                    (z.name.clone(), z.kind.label(), fl)
                                };
                                let metrics = self.city_scene.district_metrics();
                                let bldg_note = floors_opt
                                    .map(|fl| format!(" -> Spawned {fl}-story IfcBuilding"))
                                    .unwrap_or_else(|| " -> Greenery Park".to_string());
                                self.status_message = Some(format!(
                                    "Zoned '{}' ({}, 28x24 m @ {:.0}°){} | District: {} Bldgs • {:.0} m² GFA",
                                    z_name,
                                    z_kind_lbl,
                                    bearing,
                                    bldg_note,
                                    metrics.building_count,
                                    metrics.total_gfa_m2
                                ));
                            }
                        }
                        EarthBimTool::None => {}
                    }
                }
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                self.orbit_globe_by_pixels(rect, delta, alt_held, dt);
            }
            return;
        } else {
            self.earth_bim_author.hover_snap_mm = None;
        }

        if self.active_tool == UniversalCanvasTool::SelectPan {
            if self.globe_mode
                && self.world_map.zoom_level >= 8.5
                && !response.dragged()
            {
                if let Some(hover) = response.hover_pos() {
                    self.hovered_world_building_rel_path =
                        match self.pick_world_building_or_cluster_at_screen(hover, rect) {
                            Some(WorldPickOutcome::HydrateBuilding(rel)) => Some(rel),
                            _ => None,
                        };
                } else {
                    self.hovered_world_building_rel_path = None;
                }
            } else {
                self.hovered_world_building_rel_path = None;
            }
            if self.globe_mode
                && !ui.ctx().egui_wants_keyboard_input()
                && ui.input(|i| i.key_pressed(egui::Key::R))
            {
                if let Some(ref active_rel) = self.active_world_project_rel_path.clone() {
                    let step = if shift_held { -15.0 } else { 15.0 };
                    if let Some(b) = self.world_index.get(active_rel).cloned() {
                        let next_hdg = (b.heading_deg + step).rem_euclid(360.0);
                        self.world_index.update_heading(active_rel, next_hdg);
                        self.pending_world_pin_event = Some((active_rel.clone(), b.lat, b.lon, next_hdg));
                        self.status_message = Some(format!(
                            "Rotated '{}' True North heading -> {:.0}°",
                            b.name, next_hdg
                        ));
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                if self.globe_mode {
                    self.orbit_globe_by_pixels(rect, delta, alt_held, dt);
                } else {
                    self.world_map.pan_by_pixels(delta.x, delta.y);
                }
            } else if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some((kind, id)) = self.pick_bim_element_at_screen(pos, rect, bim_project) {
                        self.last_picked_bim_element = Some((kind, id));
                        self.highlighted_bim_element = Some((kind, id));
                        self.last_picked_earth_lat_lon = None;
                        self.set_single_selection(None);
                        self.spatial_pick_seq = self.spatial_pick_seq.wrapping_add(1);
                    } else {
                        let w = to_world(pos);
                        let picked = self
                            .find_nearest_entity_idx(w, hit_radius_mm)
                            .and_then(|i| self.entities.get(i).map(|e| e.id));
                        self.set_single_selection(picked);
                        self.last_picked_bim_element = None;
                        self.highlighted_bim_element = None;
                        if picked.is_none() {
                            if let Some(outcome) = self.pick_world_building_or_cluster_at_screen(pos, rect) {
                                match outcome {
                                    WorldPickOutcome::FlyToCluster { lat, lon, label } => {
                                        self.earth_world.start_fly_to(
                                            &self.world_map,
                                            lat,
                                            lon,
                                            13.5,
                                            &label,
                                        );
                                        self.last_picked_earth_lat_lon = Some((lat, lon));
                                    }
                                    WorldPickOutcome::HydrateBuilding(rel) => {
                                        self.clicked_world_building_rel_path = Some(rel);
                                        self.last_picked_earth_lat_lon = None;
                                    }
                                }
                            } else {
                                let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(pos, rect);
                                self.last_picked_earth_lat_lon = if hit { Some((lat, lon)) } else { None };
                            }
                        } else {
                            self.last_picked_earth_lat_lon = None;
                        }
                        self.spatial_pick_seq = self.spatial_pick_seq.wrapping_add(1);
                    }
                }
            }
            return;
        }

        // Auto-anchor ground origin on first drawing stroke anywhere on Earth
        if self.active_tool != UniversalCanvasTool::Eraser
            && (response.drag_started_by(egui::PointerButton::Primary)
                || response.clicked_by(egui::PointerButton::Primary))
        {
            if let Some(pos) = response.interact_pointer_pos() {
                self.ensure_ground_origin_at_screen(pos, rect);
            }
        }

        self.handle_world_sketch_interaction(response, active_pillar, hit_radius_mm, &to_world);
    }

    /// Renders interactive WGS84 geodesic polyline/parcel area measurement overlay across 2D & 3D World canvases.
    pub fn render_geodetic_measure_overlay(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        lat_lon_to_screen: &impl Fn(f64, f64) -> Pos2,
    ) {
        if self.measure_points_lat_lon.is_empty() && !self.measure_active {
            return;
        }
        let clip = painter.with_clip_rect(rect);
        let pts: Vec<Pos2> = self
            .measure_points_lat_lon
            .iter()
            .map(|&(lat, lon)| lat_lon_to_screen(lat, lon))
            .collect();

        if self.measure_area_mode && pts.len() >= 3 {
            clip.add(egui::Shape::convex_polygon(
                pts.clone(),
                theme.accent_orange.gamma_multiply(0.16),
                Stroke::new(2.0, theme.accent_orange),
            ));
        } else if pts.len() >= 2 {
            for win in pts.windows(2) {
                clip.line_segment([win[0], win[1]], Stroke::new(2.4, theme.accent_orange));
            }
        }

        for (idx, p) in pts.iter().enumerate() {
            clip.circle_filled(*p, 4.5, theme.accent_orange);
            clip.circle_stroke(*p, 4.5, Stroke::new(1.5, Color32::WHITE));
            if idx > 0 {
                let seg_m = crate::core::gis::SlippyMapState::geodetic_distance_m(
                    self.measure_points_lat_lon[idx - 1],
                    self.measure_points_lat_lon[idx],
                );
                let mid = Pos2::new(0.5 * (pts[idx - 1].x + p.x), 0.5 * (pts[idx - 1].y + p.y));
                let lbl = if seg_m >= 1000.0 {
                    format!("{:.2} km", seg_m / 1000.0)
                } else {
                    format!("{:.1} m", seg_m)
                };
                clip.text(
                    mid + Vec2::new(0.0, -8.0),
                    Align2::CENTER_BOTTOM,
                    lbl,
                    egui::FontId::monospace(10.0),
                    theme.accent_orange,
                );
            }
        }

        let total_m = crate::core::gis::SlippyMapState::geodetic_polyline_length_m(&self.measure_points_lat_lon);
        let summary = if self.measure_points_lat_lon.is_empty() {
            "Geodesic Measure Active: Click points on Earth ground".to_string()
        } else if self.measure_area_mode && self.measure_points_lat_lon.len() >= 3 {
            let area_m2 = crate::core::gis::SlippyMapState::geodetic_polygon_area_m2(
                &self.measure_points_lat_lon,
            );
            format!(
                "Parcel Area: {:.1} m² ({:.3} ha) • Perimeter {:.1} m ({} pts)",
                area_m2,
                area_m2 / 10_000.0,
                total_m,
                self.measure_points_lat_lon.len()
            )
        } else if total_m >= 1000.0 {
            format!(
                "Geodesic Distance: {:.3} km ({:.0} m) • {} pts",
                total_m / 1000.0,
                total_m,
                self.measure_points_lat_lon.len()
            )
        } else {
            format!(
                "Geodesic Distance: {:.2} m • {} pts",
                total_m,
                self.measure_points_lat_lon.len()
            )
        };

        let chip_pos = Pos2::new(rect.left() + 14.0, rect.bottom() - 26.0);
        clip.rect_filled(
            Rect::from_min_size(chip_pos, Vec2::new((summary.len() as f32 * 6.4 + 20.0).min(rect.width() - 28.0), 22.0)),
            CornerRadius::same(6),
            theme.bg_card.gamma_multiply(0.90),
        );
        clip.text(
            chip_pos + Vec2::new(8.0, 11.0),
            Align2::LEFT_CENTER,
            summary,
            egui::FontId::monospace(10.5),
            theme.accent_orange,
        );
    }

    /// Interactive drawing & erasing handler for synchronized Universal Canvas shapes using any 2D/3D ground projection.
    pub fn handle_world_sketch_interaction(
        &mut self,
        response: &egui::Response,
        active_pillar: NavigationPillar,
        hit_radius_mm: f64,
        screen_to_world: &impl Fn(Pos2) -> [f64; 2],
    ) {
        self.last_osnap_radius_mm = hit_radius_mm.clamp(250.0, 2500.0);
        if let Some(hover) = response.hover_pos() {
            let (snapped, kind) = self.snap_point_with_osnap(
                screen_to_world(hover),
                self.pending_start_mm,
                None,
                self.last_osnap_radius_mm,
            );
            self.cursor_world_mm = snapped;
            self.last_osnap_kind = kind;
        }

        let badge = active_pillar.badge().to_string();
        let pname = active_pillar.clean_name().to_string();
        let color_rgb = self.active_color_rgb();
        let stroke_width = self.stroke_width;

        match self.active_tool {
            UniversalCanvasTool::SelectPan => {
                if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = screen_to_world(pos);
                        let picked = self
                            .find_nearest_entity_idx(w, hit_radius_mm)
                            .and_then(|i| self.entities.get(i).map(|e| e.id));
                        self.set_single_selection(picked);
                    }
                }
            }
            UniversalCanvasTool::BoxSelect => {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = screen_to_world(pos);
                        self.marquee_anchor_mm = Some(w);
                        self.marquee_current_mm = Some(w);
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.marquee_current_mm = Some(screen_to_world(pos));
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    if let (Some(anchor), Some(pos)) =
                        (self.marquee_anchor_mm.take(), response.interact_pointer_pos())
                    {
                        let end_mm = screen_to_world(pos);
                        let count = self.select_marquee(anchor, end_mm);
                        let mode = if end_mm[0] >= anchor[0] { "Window" } else { "Crossing" };
                        self.status_message = Some(format!(
                            "{mode} marquee selected {count} entities"
                        ));
                    }
                    self.marquee_current_mm = None;
                } else if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = screen_to_world(pos);
                        let picked = self
                            .find_nearest_entity_idx(w, hit_radius_mm)
                            .and_then(|i| self.entities.get(i).map(|e| e.id));
                        self.set_single_selection(picked);
                    }
                }
            }
            UniversalCanvasTool::Freehand => {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.active_freehand_mm.clear();
                        self.active_freehand_mm.push(screen_to_world(pos));
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let pt = screen_to_world(pos);
                        let push_it = self
                            .active_freehand_mm
                            .last()
                            .is_none_or(|last| (last[0] - pt[0]).hypot(last[1] - pt[1]) >= 40.0);
                        if push_it {
                            self.active_freehand_mm.push(pt);
                        }
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary)
                    && self.active_freehand_mm.len() >= 2
                {
                    let pts = std::mem::take(&mut self.active_freehand_mm);
                    self.push_undo();
                    let id = self.next_entity_id;
                    self.next_entity_id += 1;
                    self.entities.push(CanvasEntity {
                        id,
                        pillar_badge: badge,
                        pillar_name: pname,
                        kind: CanvasShapeKind::Freehand { points_mm: pts },
                        color_rgb,
                        stroke_width,
                        layer_id: self.visibility_graphics.active_cad_layer_id.clone(),
                    });
                }
            }
            UniversalCanvasTool::Line
            | UniversalCanvasTool::Rectangle
            | UniversalCanvasTool::Circle
            | UniversalCanvasTool::Dimension => {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.pending_start_mm = Some(self.snap_point(screen_to_world(pos)));
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    if let (Some(start), Some(pos)) =
                        (self.pending_start_mm.take(), response.interact_pointer_pos())
                    {
                        let raw_end = self.snap_point(screen_to_world(pos));
                        let end = if matches!(
                            self.active_tool,
                            UniversalCanvasTool::Line | UniversalCanvasTool::Dimension
                        ) {
                            self.apply_ortho(start, raw_end)
                        } else {
                            raw_end
                        };
                        let dist = (end[0] - start[0]).hypot(end[1] - start[1]);
                        if dist >= 50.0 {
                            let kind = match self.active_tool {
                                UniversalCanvasTool::Line => CanvasShapeKind::Line {
                                    start_mm: start,
                                    end_mm: end,
                                },
                                UniversalCanvasTool::Dimension => CanvasShapeKind::Dimension {
                                    start_mm: start,
                                    end_mm: end,
                                },
                                UniversalCanvasTool::Rectangle => CanvasShapeKind::Rectangle {
                                    min_mm: [start[0].min(end[0]), start[1].min(end[1])],
                                    max_mm: [start[0].max(end[0]), start[1].max(end[1])],
                                },
                                UniversalCanvasTool::Circle => CanvasShapeKind::Circle {
                                    center_mm: start,
                                    radius_mm: dist,
                                },
                                _ => unreachable!(),
                            };
                            self.push_undo();
                            let id = self.next_entity_id;
                            self.next_entity_id += 1;
                            self.set_single_selection(Some(id));
                            self.entities.push(CanvasEntity {
                                id,
                                pillar_badge: badge,
                                pillar_name: pname,
                                kind,
                                color_rgb,
                                stroke_width,
                                layer_id: self.visibility_graphics.active_cad_layer_id.clone(),
                            });
                        }
                    } else {
                        self.pending_start_mm = None;
                    }
                }
            }
            UniversalCanvasTool::Callout => {
                if response.clicked_by(egui::PointerButton::Primary) {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = self.snap_point(screen_to_world(pos));
                        let note = if self.callout_draft.trim().is_empty() {
                            format!("{} Note", active_pillar.badge())
                        } else {
                            self.callout_draft.trim().to_string()
                        };
                        self.push_undo();
                        let id = self.next_entity_id;
                        self.next_entity_id += 1;
                        self.entities.push(CanvasEntity {
                            id,
                            pillar_badge: badge,
                            pillar_name: pname,
                            kind: CanvasShapeKind::Callout {
                                pos_mm: w,
                                text: note,
                            },
                            color_rgb,
                            stroke_width,
                            layer_id: self.visibility_graphics.active_cad_layer_id.clone(),
                        });
                    }
                }
            }
            UniversalCanvasTool::Eraser => {
                if response.clicked_by(egui::PointerButton::Primary)
                    || response.dragged_by(egui::PointerButton::Primary)
                {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let w = screen_to_world(pos);
                        if let Some(idx) = self.find_nearest_entity_idx(w, hit_radius_mm) {
                            self.push_undo();
                            self.entities.remove(idx);
                        }
                    }
                }
            }
            UniversalCanvasTool::EarthUplift
            | UniversalCanvasTool::EarthCarve
            | UniversalCanvasTool::EarthReclaim
            | UniversalCanvasTool::EarthForest
            | UniversalCanvasTool::EarthSolar
            | UniversalCanvasTool::EarthLake
            | UniversalCanvasTool::EarthSeawall
            | UniversalCanvasTool::EarthBridge
            | UniversalCanvasTool::EarthTower
            | UniversalCanvasTool::EarthCorridor => {}
        }
    }

    pub(super) fn find_nearest_entity_idx(&self, pt_mm: [f64; 2], max_dist_mm: f64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (idx, ent) in self.entities.iter().enumerate() {
            let d = match &ent.kind {
                CanvasShapeKind::Line { start_mm, end_mm }
                | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    point_to_segment_dist(pt_mm, *start_mm, *end_mm)
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    let c0 = *min_mm;
                    let c1 = [max_mm[0], min_mm[1]];
                    let c2 = *max_mm;
                    let c3 = [min_mm[0], max_mm[1]];
                    point_to_segment_dist(pt_mm, c0, c1)
                        .min(point_to_segment_dist(pt_mm, c1, c2))
                        .min(point_to_segment_dist(pt_mm, c2, c3))
                        .min(point_to_segment_dist(pt_mm, c3, c0))
                }
                CanvasShapeKind::Circle {
                    center_mm,
                    radius_mm,
                } => ((pt_mm[0] - center_mm[0]).hypot(pt_mm[1] - center_mm[1]) - radius_mm).abs(),
                CanvasShapeKind::Freehand { points_mm } => points_mm
                    .windows(2)
                    .map(|w| point_to_segment_dist(pt_mm, w[0], w[1]))
                    .fold(f64::MAX, f64::min),
                CanvasShapeKind::Callout { pos_mm, .. } => {
                    (pt_mm[0] - pos_mm[0]).hypot(pt_mm[1] - pos_mm[1])
                }
            };
            if d <= max_dist_mm && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((idx, d));
            }
        }
        best.map(|(i, _)| i)
    }
    /// Screen-space hit-testing against projected BIM Columns, Beams/Runs, Walls, Slabs, and Footings
    /// across both tilted 3D Earth projections and orthographic ground projections.
    pub fn pick_bim_element_at_screen(
        &self,
        pos: Pos2,
        rect: Rect,
        bim_project: &BimProject,
    ) -> Option<(crate::core::BimElementKind, usize)> {
        if bim_project.is_empty() || !rect.contains(pos) {
            return None;
        }
        let seg_dist = |p: Pos2, a: Pos2, b: Pos2| -> f32 {
            let ab = b - a;
            let len_sq = ab.length_sq();
            if len_sq <= 1e-4 {
                return p.distance(a);
            }
            let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
            p.distance(a + ab * t)
        };
        let point_in_poly = |pt: Pos2, poly: &[Pos2]| -> bool {
            if poly.len() < 3 {
                return false;
            }
            let mut inside = false;
            let mut j = poly.len() - 1;
            for i in 0..poly.len() {
                let pi = poly[i];
                let pj = poly[j];
                if ((pi.y > pt.y) != (pj.y > pt.y))
                    && (pt.x < (pj.x - pi.x) * (pt.y - pi.y) / ((pj.y - pi.y) + 1e-6) + pi.x)
                {
                    inside = !inside;
                }
                j = i;
            }
            inside
        };

        // Height-aware 3D Earth Globe picking when rendering in Earth 3D
        if self.globe_mode && self.ground_origin_anchored {
            if let Some(ref xf) = self.last_globe_xform {
                let lift_scale = if self.world_map.zoom_level < 14.5 { 8.0 } else { 1.0 };
                let proj_mm = |x_mm: f64, y_mm: f64, elev_m: f64| -> Option<Pos2> {
                    self.project_active_bim_mm(xf, x_mm, y_mm, elev_m, lift_scale)
                };

                let isolate = self.earth_bim_author.isolate_active_story;
                let active_lvl_id = self.earth_bim_author.base_level_id.max(1);
                let active_top_id = self.earth_bim_author.top_level_id.max(active_lvl_id);

                // 1. Columns (3D vertical shaft)
                let mut best_col: Option<(f32, usize)> = None;
                for c in &bim_project.columns {
                    if isolate && c.base_level_id != active_lvl_id && c.top_level_id != active_lvl_id {
                        continue;
                    }
                    let z0 = bim_project.level_elevation(c.base_level_id) * 0.001;
                    let z1 = bim_project.level_elevation(c.top_level_id) * 0.001;
                    if let (Some(p0), Some(p1)) = (proj_mm(c.pos.0, c.pos.1, z0), proj_mm(c.pos.0, c.pos.1, z1)) {
                        let d = seg_dist(pos, p0, p1);
                        if d <= 18.0 && best_col.is_none_or(|(bd, _)| d < bd) {
                            best_col = Some((d, c.id));
                        }
                    }
                }
                if let Some((_, id)) = best_col {
                    return Some((crate::core::BimElementKind::Column, id));
                }

                // 2. Beams / Runs (at elevated storey height)
                let mut best_run: Option<(f32, usize)> = None;
                for r in &bim_project.runs {
                    if isolate && r.level_id != active_lvl_id && r.level_id != active_top_id {
                        continue;
                    }
                    let z = bim_project.level_elevation(r.level_id) * 0.001;
                    if let (Some(p0), Some(p1)) = (proj_mm(r.start.0, r.start.1, z), proj_mm(r.end.0, r.end.1, z)) {
                        let d = seg_dist(pos, p0, p1);
                        if d <= 14.0 && best_run.is_none_or(|(bd, _)| d < bd) {
                            best_run = Some((d, r.id));
                        }
                    }
                }
                if let Some((_, id)) = best_run {
                    return Some((crate::core::BimElementKind::Run, id));
                }

                // 3. Walls (3D extruded face quad or chords)
                for w in bim_project.walls.iter().rev() {
                    if isolate && w.base_level_id != active_lvl_id {
                        continue;
                    }
                    let z0 = bim_project.level_elevation(w.base_level_id) * 0.001;
                    let z1 = w
                        .top_level_id
                        .map(|tid| bim_project.level_elevation(tid))
                        .unwrap_or_else(|| bim_project.level_elevation(w.base_level_id) + w.unconnected_height_mm)
                        * 0.001;
                    if let (Some(b0), Some(b1), Some(t1), Some(t0)) = (
                        proj_mm(w.start.0, w.start.1, z0),
                        proj_mm(w.end.0, w.end.1, z0),
                        proj_mm(w.end.0, w.end.1, z1),
                        proj_mm(w.start.0, w.start.1, z1),
                    ) {
                        let quad = [b0, b1, t1, t0];
                        if point_in_poly(pos, &quad)
                            || seg_dist(pos, b0, b1) <= 10.0
                            || seg_dist(pos, t0, t1) <= 10.0
                        {
                            return Some((crate::core::BimElementKind::Wall, w.id));
                        }
                    }
                }

                // 4. Footings
                for f in &bim_project.footings {
                    if isolate && active_lvl_id > 1 && f.level_id != active_lvl_id {
                        continue;
                    }
                    let z = bim_project.level_elevation(f.level_id) * 0.001;
                    if let Some(sp) = proj_mm(f.pos.0, f.pos.1, z) {
                        if pos.distance(sp) <= 18.0 {
                            return Some((crate::core::BimElementKind::Footing, f.id));
                        }
                    }
                }

                // 5. Slabs / Roofs (elevated 3D polygon)
                for sl in bim_project.slabs.iter().rev() {
                    if isolate && sl.level_id != active_lvl_id && sl.level_id != active_top_id {
                        continue;
                    }
                    let z = bim_project.level_elevation(sl.level_id) * 0.001;
                    let pts: Vec<Pos2> = sl
                        .boundary
                        .iter()
                        .filter_map(|p| proj_mm(p.0, p.1, z))
                        .collect();
                    if point_in_poly(pos, &pts) {
                        return Some((crate::core::BimElementKind::Slab, sl.id));
                    }
                }
            }
        }

        // Orthographic fallback picking
        // 1. Columns (highest priority at grid intersections)
        let mut best_col: Option<(f32, usize)> = None;
        for c in &bim_project.columns {
            let sp = self.world_to_screen(rect, [c.pos.0, c.pos.1]);
            if !rect.expand(24.0).contains(sp) {
                continue;
            }
            let d = pos.distance(sp);
            if d <= 16.0 && best_col.is_none_or(|(bd, _)| d < bd) {
                best_col = Some((d, c.id));
            }
        }
        if let Some((_, id)) = best_col {
            return Some((crate::core::BimElementKind::Column, id));
        }

        // 2. Beams / Structural Runs
        let mut best_run: Option<(f32, usize)> = None;
        for r in &bim_project.runs {
            let s0 = self.world_to_screen(rect, [r.start.0, r.start.1]);
            let s1 = self.world_to_screen(rect, [r.end.0, r.end.1]);
            if !rect.expand(30.0).contains(s0) && !rect.expand(30.0).contains(s1) {
                continue;
            }
            let d = seg_dist(pos, s0, s1);
            if d <= 11.0 && best_run.is_none_or(|(bd, _)| d < bd) {
                best_run = Some((d, r.id));
            }
        }
        if let Some((_, id)) = best_run {
            return Some((crate::core::BimElementKind::Run, id));
        }

        // 3. Walls
        let mut best_wall: Option<(f32, usize)> = None;
        for w in &bim_project.walls {
            let s0 = self.world_to_screen(rect, [w.start.0, w.start.1]);
            let s1 = self.world_to_screen(rect, [w.end.0, w.end.1]);
            if !rect.expand(30.0).contains(s0) && !rect.expand(30.0).contains(s1) {
                continue;
            }
            let d = seg_dist(pos, s0, s1);
            if d <= 12.0 && best_wall.is_none_or(|(bd, _)| d < bd) {
                best_wall = Some((d, w.id));
            }
        }
        if let Some((_, id)) = best_wall {
            return Some((crate::core::BimElementKind::Wall, id));
        }

        // 4. Isolated Footings
        let mut best_ft: Option<(f32, usize)> = None;
        for f in &bim_project.footings {
            let sp = self.world_to_screen(rect, [f.pos.0, f.pos.1]);
            if !rect.expand(24.0).contains(sp) {
                continue;
            }
            let d = pos.distance(sp);
            if d <= 16.0 && best_ft.is_none_or(|(bd, _)| d < bd) {
                best_ft = Some((d, f.id));
            }
        }
        if let Some((_, id)) = best_ft {
            return Some((crate::core::BimElementKind::Footing, id));
        }

        // 5. Floor Slabs / Roofs (reverse order prefers top-most/latest slab)
        for sl in bim_project.slabs.iter().rev() {
            if sl.boundary.len() < 3 {
                continue;
            }
            let pts: Vec<Pos2> = sl
                .boundary
                .iter()
                .map(|p| self.world_to_screen(rect, [p.0, p.1]))
                .collect();
            if point_in_poly(pos, &pts) {
                return Some((crate::core::BimElementKind::Slab, sl.id));
            }
        }
        None
    }

    /// Projects local active-building `(x_mm, y_mm, elev_m)` onto Earth 3D applying active heading rotation cleanly.
    pub fn project_active_bim_mm(
        &self,
        xf: &GlobeScreenTransform,
        x_mm: f64,
        y_mm: f64,
        elev_m: f64,
        lift_scale: f64,
    ) -> Option<Pos2> {
        let heading = self
            .active_world_project_rel_path
            .as_deref()
            .and_then(|r| self.world_index.get(r))
            .map(|b| b.heading_deg)
            .unwrap_or(0.0);
        let (rx_mm, ry_mm) = if heading.abs() > 1e-3 {
            let rad = heading.to_radians();
            let (s, c) = (rad.sin(), rad.cos());
            (x_mm * c - y_mm * s, x_mm * s + y_mm * c)
        } else {
            (x_mm, y_mm)
        };
        let ll = SlippyMapState::world_mm_to_lat_lon(self.ground_origin_lat_lon, [rx_mm, ry_mm]);
        let relief = self.local_site_relief_m(ll.0, ll.1);
        xf.project_lat_lon_elev(ll.0, ll.1, relief + elev_m * lift_scale)
            .and_then(|(p, _, vis)| if vis { Some(p) } else { None })
    }

    /// Computes conservative `(min_lat, max_lat, min_lon, max_lon)` visible across `xform.rect`.
    pub fn visible_geo_bbox(&self, xform: &GlobeScreenTransform) -> (f64, f64, f64, f64) {
        if self.world_map.zoom_level < 4.5 {
            return (-85.0, 85.0, -180.0, 180.0);
        }
        let mut min_lat = 85.0_f64;
        let mut max_lat = -85.0_f64;
        let mut min_lon = 180.0_f64;
        let mut max_lon = -180.0_f64;
        let mut hits = 0usize;
        for gy in 0..=4 {
            let fy = gy as f32 * 0.25;
            for gx in 0..=4 {
                let fx = gx as f32 * 0.25;
                let sp = Pos2::new(
                    xform.rect.min.x + xform.rect.width() * fx,
                    xform.rect.min.y + xform.rect.height() * fy,
                );
                let (la, lo, hit) = xform.screen_to_lat_lon(sp);
                if hit {
                    hits += 1;
                    min_lat = min_lat.min(la);
                    max_lat = max_lat.max(la);
                    min_lon = min_lon.min(lo);
                    max_lon = max_lon.max(lo);
                }
            }
        }
        if hits < 6 || (max_lon - min_lon) > 160.0 {
            return (-85.0, 85.0, -180.0, 180.0);
        }
        let pad = if self.world_map.zoom_level < 10.0 { 0.35 } else { 0.04 };
        (
            (min_lat - pad).clamp(-85.0, 85.0),
            (max_lat + pad).clamp(-85.0, 85.0),
            min_lon - pad,
            max_lon + pad,
        )
    }

    pub fn pick_world_building_or_cluster_at_screen(
        &self,
        pos: Pos2,
        rect: Rect,
    ) -> Option<WorldPickOutcome> {
        if self.world_index.buildings.is_empty() || !rect.contains(pos) {
            return None;
        }
        let xf = self.last_globe_xform.as_ref()?;
        let (min_lat, max_lat, min_lon, max_lon) = self.visible_geo_bbox(xf);

        if self.world_map.zoom_level < 8.5 {
            let clusters = self.world_index.query_clusters(
                min_lat,
                max_lat,
                min_lon,
                max_lon,
                self.world_map.zoom_level,
            );
            for cl in clusters {
                if let Some((sp, _, true)) = xf.project_lat_lon_elev(cl.lat, cl.lon, 0.0) {
                    if pos.distance(sp) <= 20.0 {
                        if cl.count == 1 {
                            return Some(WorldPickOutcome::HydrateBuilding(cl.primary_rel_path));
                        } else {
                            return Some(WorldPickOutcome::FlyToCluster {
                                lat: cl.lat,
                                lon: cl.lon,
                                label: cl.label,
                            });
                        }
                    }
                }
            }
            return None;
        }

        let lift_scale = if self.world_map.zoom_level < 14.5 { 6.0 } else { 1.0 };
        let point_in_poly = |pt: Pos2, poly: &[Pos2]| -> bool {
            if poly.len() < 3 {
                return false;
            }
            let mut inside = false;
            let mut j = poly.len() - 1;
            for i in 0..poly.len() {
                let pi = poly[i];
                let pj = poly[j];
                if ((pi.y > pt.y) != (pj.y > pt.y))
                    && (pt.x < (pj.x - pi.x) * (pt.y - pi.y) / ((pj.y - pi.y) + 1e-6) + pi.x)
                {
                    inside = !inside;
                }
                j = i;
            }
            inside
        };

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
            let hull_mm = b.rotated_footprint_world_mm();
            let mut base_pts = Vec::with_capacity(hull_mm.len());
            let mut roof_pts = Vec::with_capacity(hull_mm.len());
            let base_elev = self.local_site_relief_m(b.lat, b.lon);
            let top_elev = base_elev + b.total_height_m * lift_scale;
            let mut ok = true;
            for pt_mm in &hull_mm {
                let ll = SlippyMapState::world_mm_to_lat_lon((b.lat, b.lon), *pt_mm);
                if let (Some((bp, _, true)), Some((tp, _, true))) = (
                    xf.project_lat_lon_elev(ll.0, ll.1, base_elev),
                    xf.project_lat_lon_elev(ll.0, ll.1, top_elev),
                ) {
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
            if point_in_poly(pos, &roof_pts) {
                return Some(WorldPickOutcome::HydrateBuilding(b.rel_path.clone()));
            }
            for i in 0..base_pts.len() {
                let ni = (i + 1) % base_pts.len();
                if point_in_poly(pos, &[base_pts[i], base_pts[ni], roof_pts[ni], roof_pts[i]]) {
                    return Some(WorldPickOutcome::HydrateBuilding(b.rel_path.clone()));
                }
            }
        }
        None
    }

}
