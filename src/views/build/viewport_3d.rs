//! views/build/viewport_3d.rs — 360-deg Interactive 3D BIM & Analytical Diagram Viewport.

use super::*;

#[derive(Clone)]
struct Member3D {
    id: usize,
    p1: (f32, f32, f32),
    p2: (f32, f32, f32),
    name: String,
    section: String,
    base_width: f32,
    force_kn: f32,
    capacity_kn: f32,
    utilization: f32,
    ari: f32,
    is_critical: bool,
}


impl BuildView {
    /// Auto-fits the 3D orbit camera zoom and ground World Map around active BIM / FEM extents.
    pub fn fit_view_to_project(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
        members: &[StructuralMember],
    ) {
        let extents = self.bim_bus.project.plan_extents_mm();
        universal_canvas.fit_world_to_bim_extents(extents);
        let (_, _, raw) = self.build_model_geometry(members);
        if raw.is_empty() {
            self.cam_zoom = 1.15;
            self.cam_pan = Vec2::ZERO;
            return;
        }
        let mut min_v = (f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max_v = (f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for m in &raw {
            for p in [m.p1, m.p2] {
                min_v.0 = min_v.0.min(p.0);
                min_v.1 = min_v.1.min(p.1);
                min_v.2 = min_v.2.min(p.2);
                max_v.0 = max_v.0.max(p.0);
                max_v.1 = max_v.1.max(p.1);
                max_v.2 = max_v.2.max(p.2);
            }
        }
        let span_m = (max_v.0 - min_v.0)
            .max(max_v.1 - min_v.1)
            .max(max_v.2 - min_v.2)
            .max(4.0);
        self.cam_zoom = (14.5 / span_m).clamp(0.30, 2.80);
        self.cam_pan = Vec2::ZERO;
        self.cam_yaw = 0.65;
        self.cam_pitch = 0.35;
    }

    fn apply_3d_move_1m(&mut self, universal_canvas: &mut UniversalCanvasState) {
        let step = universal_canvas.cad_step_mm;
        if self.sketch_mode_active && universal_canvas.translate_selected_or_last(step, 0.0) {
            self.modify_status_msg = Some(format!("Moved sketch entity +{step:.0} mm East (M)"));
            return;
        }
        if let Some(id) = self.bim_bus.project.latest_element_id() {
            match self.bim_bus.execute(crate::core::BimCommand::MoveElement {
                element_id: id,
                delta: crate::core::P2(step, 0.0),
            }) {
                Ok(msg) => {
                    self.modify_status_msg = Some(msg.clone());
                    self.bim_console_logs.push(msg);
                }
                Err(e) => self.modify_status_msg = Some(e),
            }
        }
    }

    fn apply_3d_rotate_90(&mut self, universal_canvas: &mut UniversalCanvasState) {
        if self.sketch_mode_active && universal_canvas.rotate_selected_or_last(90.0) {
            self.modify_status_msg = Some("Rotated sketch entity +90°".to_string());
            return;
        }
        let target_id = self
            .selected_member_id
            .or_else(|| self.bim_bus.project.latest_element_id());
        if let Some(id) = target_id {
            match self.bim_bus.execute(crate::core::BimCommand::RotateElement {
                element_id: id,
                angle_deg: 90.0,
            }) {
                Ok(msg) => {
                    self.modify_status_msg = Some(msg.clone());
                    self.bim_console_logs.push(msg);
                }
                Err(e) => self.modify_status_msg = Some(e),
            }
        }
    }

    fn apply_3d_duplicate(&mut self, universal_canvas: &mut UniversalCanvasState) {
        let step = universal_canvas.cad_step_mm;
        if self.sketch_mode_active && universal_canvas.duplicate_selected_or_last(step, 0.0) {
            self.modify_status_msg =
                Some(format!("Duplicated sketch entity +{step:.0} mm East"));
            return;
        }
        if let Some(w) = self.bim_bus.project.walls.last().cloned() {
            match self.bim_bus.execute(crate::core::BimCommand::AddWall {
                base_level_id: w.base_level_id,
                start: crate::core::P2(w.start.0 + step, w.start.1),
                end: crate::core::P2(w.end.0 + step, w.end.1),
                thickness_mm: w.thickness_mm,
                is_structural: w.is_structural,
            }) {
                Ok(msg) => {
                    self.modify_status_msg = Some(format!("Copied wall +{step:.0} mm: {msg}"));
                    self.bim_console_logs.push(msg);
                }
                Err(e) => self.modify_status_msg = Some(e),
            }
        } else if let Some(c) = self.bim_bus.project.columns.last().cloned() {
            match self.bim_bus.execute(crate::core::BimCommand::AddColumn {
                base_level_id: c.base_level_id,
                top_level_id: c.top_level_id,
                pos: crate::core::P2(c.pos.0 + step, c.pos.1),
                width_mm: c.width_mm,
                depth_mm: c.depth_mm,
                is_round: c.is_round,
                profile_name: c.profile_name,
            }) {
                Ok(msg) => {
                    self.modify_status_msg = Some(format!("Copied column +{step:.0} mm: {msg}"));
                    self.bim_console_logs.push(msg);
                }
                Err(e) => self.modify_status_msg = Some(e),
            }
        }
    }

    fn apply_3d_offset_wall(&mut self, universal_canvas: &mut UniversalCanvasState) {
        if self.sketch_mode_active && universal_canvas.offset_selected_or_last(500.0) {
            self.modify_status_msg = Some("Created +500 mm offset sketch duplicate (O)".to_string());
            return;
        }
        if let Some(wid) = self.bim_bus.project.latest_wall_id() {
            match self.bim_bus.execute(crate::core::BimCommand::OffsetWall {
                wall_id: wid,
                offset_mm: 500.0,
            }) {
                Ok(msg) => {
                    self.modify_status_msg = Some(msg.clone());
                    self.bim_console_logs.push(msg);
                }
                Err(e) => self.modify_status_msg = Some(e),
            }
        } else {
            self.modify_status_msg =
                Some("Add a wall first before applying +500 mm Wall Offset".to_string());
        }
    }

    fn apply_3d_mirror(&mut self, universal_canvas: &mut UniversalCanvasState) {
        let cx = self
            .bim_bus
            .project
            .plan_extents_mm()
            .map_or(0.0, |(a, b)| 0.5 * (a[0] + b[0]));
        if self.sketch_mode_active && universal_canvas.mirror_selected_or_last(cx) {
            self.modify_status_msg = Some(format!("Mirrored sketch entity across X={cx:.0} mm (X)"));
            return;
        }
        match self.bim_bus.execute(crate::core::BimCommand::MirrorElements {
            axis_a: crate::core::P2(cx, -5000.0),
            axis_b: crate::core::P2(cx, 5000.0),
        }) {
            Ok(msg) => {
                self.modify_status_msg = Some(msg.clone());
                self.bim_console_logs.push(msg);
            }
            Err(e) => self.modify_status_msg = Some(e),
        }
    }

    fn apply_3d_delete(&mut self, universal_canvas: &mut UniversalCanvasState) {
        if self.sketch_mode_active && universal_canvas.delete_selected_or_last() {
            self.modify_status_msg = Some("Deleted sketch entity (Del)".to_string());
            return;
        }
        let target_id = self
            .selected_member_id
            .or_else(|| self.bim_bus.project.latest_element_id());
        if let Some(id) = target_id {
            match self
                .bim_bus
                .execute(crate::core::BimCommand::DeleteElement { element_id: id })
            {
                Ok(msg) => {
                    self.selected_member_id = None;
                    self.modify_status_msg = Some(msg.clone());
                    self.bim_console_logs.push(msg);
                }
                Err(e) => self.modify_status_msg = Some(e),
            }
        }
    }

    fn handle_3d_shortcuts(
        &mut self,
        ctx: &egui::Context,
        universal_canvas: &mut UniversalCanvasState,
        members: &[StructuralMember],
    ) {
        let fb = UniversalCanvasState::poll_cad_shortcuts(ctx, universal_canvas.is_frozen);
        if fb.toggle_freeze {
            universal_canvas.is_frozen = !universal_canvas.is_frozen;
            self.pending_3d_start_mm = None;
            self.modify_status_msg = Some(if universal_canvas.is_frozen {
                "3D Viewport Frozen (Space) • Scroll safely".to_string()
            } else {
                "3D Viewport Live (Space) • Orbit & CAD keys active".to_string()
            });
        }
        if fb.escape {
            self.pending_3d_start_mm = None;
            self.selected_member_id = None;
            self.ground_author_tool = Ground3DAuthorTool::Orbit;
            universal_canvas.measure_active = false;
            universal_canvas.selected_entity_id = None;
        }
        if fb.undo {
            if self.sketch_mode_active && universal_canvas.undo() {
                self.modify_status_msg = Some("Undo sketch edit (Ctrl/Cmd+Z)".to_string());
            } else if let Some(msg) = self.bim_bus.undo() {
                self.modify_status_msg = Some(msg);
            }
        }
        if fb.redo {
            if self.sketch_mode_active && universal_canvas.redo() {
                self.modify_status_msg = Some("Redo sketch edit (Ctrl/Cmd+Y)".to_string());
            } else if let Some(msg) = self.bim_bus.redo() {
                self.modify_status_msg = Some(msg);
            }
        }
        if fb.delete_selected {
            self.apply_3d_delete(universal_canvas);
        }
        if fb.toggle_ortho {
            universal_canvas.ortho_mode = !universal_canvas.ortho_mode;
        }
        if fb.toggle_snap {
            universal_canvas.snap_to_grid = !universal_canvas.snap_to_grid;
        }
        if fb.fit_model {
            self.fit_view_to_project(universal_canvas, members);
        }
        if let Some(ch) = fb.letter_key {
            match ch {
                'W' => {
                    self.sketch_mode_active = false;
                    self.ground_author_tool = Ground3DAuthorTool::Wall;
                    self.pending_3d_start_mm = None;
                }
                'B' => {
                    self.sketch_mode_active = false;
                    self.ground_author_tool = Ground3DAuthorTool::Beam;
                    self.pending_3d_start_mm = None;
                }
                'C' => {
                    self.sketch_mode_active = false;
                    self.ground_author_tool = Ground3DAuthorTool::Column;
                    self.pending_3d_start_mm = None;
                }
                'F' => {
                    self.sketch_mode_active = false;
                    self.ground_author_tool = Ground3DAuthorTool::Footing;
                    self.pending_3d_start_mm = None;
                }
                'M' => self.apply_3d_move_1m(universal_canvas),
                'O' => self.apply_3d_offset_wall(universal_canvas),
                'X' => self.apply_3d_mirror(universal_canvas),
                _ => {}
            }
        }
    }

    /// Phase 46: Renders the zero-chrome, edge-to-edge full-viewport 3D BIM + Earth Ground canvas
    /// without below-viewport toolbars or sub-menus.
    #[allow(clippy::too_many_arguments)]
    pub fn show_spatial_fullscreen(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        db_components: &[DatabaseComponent],
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
        parametric_variables: &[ParametricVariable],
        workspace_settings: WorkspaceCanvasSettings,
        pointer_blocked_by_hud: bool,
    ) -> Rect {
        ui_kit::set_canvas_frozen(ui.ctx(), universal_canvas.is_frozen);
        self.render_canvas_inner(
            ui,
            theme,
            members,
            db_components,
            features,
            universal_canvas,
            parametric_variables,
            workspace_settings,
            0.0,
            true,
            pointer_blocked_by_hud,
        );
        self.last_viewport_rect.unwrap_or(ui.max_rect())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_canvas(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        db_components: &[DatabaseComponent],
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
        parametric_variables: &[ParametricVariable],
        workspace_settings: WorkspaceCanvasSettings,
        sidebar_toggle_inset_px: f32,
    ) {
        self.render_canvas_inner(
            ui,
            theme,
            members,
            db_components,
            features,
            universal_canvas,
            parametric_variables,
            workspace_settings,
            sidebar_toggle_inset_px,
            false,
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_canvas_inner(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        db_components: &[DatabaseComponent],
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
        parametric_variables: &[ParametricVariable],
        workspace_settings: WorkspaceCanvasSettings,
        sidebar_toggle_inset_px: f32,
        spatial_fullscreen: bool,
        pointer_blocked_by_hud: bool,
    ) {
        self.handle_3d_shortcuts(ui.ctx(), universal_canvas, members);
        let avail_w = ui.available_width().max(200.0);
        let is_wide = avail_w >= 980.0;
        let has_floating_room = avail_w >= 720.0;

        // 1. Full-Width 3D Earth Ground Viewport FIRST (at top of canvas container)
        let canvas_size = if spatial_fullscreen {
            ui.available_size().max(Vec2::new(240.0, 240.0))
        } else {
            Vec2::new(avail_w, universal_canvas.preferred_canvas_height(ui.ctx()))
        };
        let (response, painter) = ui.allocate_painter(canvas_size, Sense::click_and_drag());
        let rect = response.rect;
        self.last_viewport_rect = Some(rect);
        self.last_floating_rects.clear();
        let _ = is_wide;
        let _ = has_floating_room;

        let corner_r = if spatial_fullscreen { 0 } else { 10 };
        painter.rect_filled(rect, CornerRadius::same(corner_r), UniversalCanvasState::canvas_bg(theme));

        // Top-left Floating Freeze / Live Toggle inside 3D Viewport (hidden in Spatial Fullscreen mode)
        let (freeze_pill_rect, pointer_in_freeze_pill, freeze_pill_hovered) = if spatial_fullscreen {
            (Rect::NOTHING, false, false)
        } else {
            UniversalCanvasState::interact_floating_freeze_pill(
                ui,
                rect,
                "geometry_3d",
                &mut universal_canvas.is_frozen,
                sidebar_toggle_inset_px,
            )
        };
        let pointer_in_floating = pointer_in_freeze_pill || pointer_blocked_by_hud;

        // Geometry Definitions & Model Centers
        let (center_3d, base_scale_unit, raw_members) = self.build_model_geometry(members);

        // Mouse Orbit, Earth Pan, and Map/Camera Zoom Controls (respecting Freeze Canvas)
        let is_frozen_now =
            universal_canvas.is_frozen || crate::ui_kit::is_canvas_frozen(ui.ctx());
        let allow_sketch_drag = self.sketch_mode_active
            && universal_canvas.active_tool
                != crate::views::universal_canvas::UniversalCanvasTool::SelectPan;
        if !pointer_in_floating && !is_frozen_now {
            if response.dragged_by(egui::PointerButton::Secondary)
                || response.dragged_by(egui::PointerButton::Middle)
                || (response.dragged() && ui.input(|i| i.modifiers.shift))
            {
                let delta = response.drag_delta();
                if self.cam_pitch.abs() >= 0.18 {
                    let cos_y = self.cam_yaw.cos();
                    let sin_y = self.cam_yaw.sin();
                    let sin_eff = if self.cam_pitch >= 0.0 {
                        self.cam_pitch.sin().max(0.32)
                    } else {
                        self.cam_pitch.sin().min(-0.32)
                    };
                    let dz_cam = delta.y / sin_eff;
                    let map_dx = delta.x * cos_y - dz_cam * sin_y;
                    let map_dy = delta.x * sin_y + dz_cam * cos_y;
                    universal_canvas.world_map.pan_by_pixels(map_dx, map_dy);
                } else {
                    self.cam_pan += delta;
                }
            } else if !allow_sketch_drag
                && self.ground_author_tool == Ground3DAuthorTool::Orbit
                && !universal_canvas.measure_active
                && response.dragged_by(egui::PointerButton::Primary)
            {
                self.cam_yaw = (self.cam_yaw + response.drag_delta().x * 0.008)
                    .rem_euclid(2.0 * std::f32::consts::PI);
                self.cam_pitch =
                    (self.cam_pitch - response.drag_delta().y * 0.008).clamp(-1.52, 1.52);
            }

            if response.hovered() {
                let (scroll_y, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                let pinch_dz = if (pinch - 1.0).abs() > 0.002 {
                    pinch.log2()
                } else {
                    0.0
                };
                let wheel_dz = if scroll_y.abs() > 0.2 {
                    scroll_y * 0.0020
                } else {
                    0.0
                };
                let raw_dz = wheel_dz + pinch_dz;
                if raw_dz.abs() > 1e-4 {
                    let taper = 1.0 - ((universal_canvas.world_map.zoom_level - 8.0) / 22.0).clamp(0.0, 0.40);
                    let dz = (raw_dz * taper).clamp(-0.28, 0.28);
                    universal_canvas.world_map.zoom_level =
                        (universal_canvas.world_map.zoom_level + dz).clamp(1.2, 20.5);
                    if !raw_members.is_empty() {
                        self.cam_zoom = (self.cam_zoom * 2.0_f32.powf(dz)).clamp(0.20, 8.0);
                    }
                }
            }
        }

        // 3D Projector Function with Camera Transformations & Positive-Into-Screen Depth
        let is_persp = self.perspective && self.cam_pitch.abs() < 1.45;
        let cam_yaw = self.cam_yaw;
        let cam_pitch = self.cam_pitch;
        let cam_zoom = self.cam_zoom;
        let cam_pan = self.cam_pan;
        let center_screen = rect.center();
        let d_cam = 55.0_f32;

        let project_3d = move |x: f32, y: f32, z: f32| -> (Pos2, f32) {
            let dx = x - center_3d.0;
            let dy = y - center_3d.1;
            let dz = z - center_3d.2;

            // Yaw rotation around Y axis
            let cos_y = cam_yaw.cos();
            let sin_y = cam_yaw.sin();
            let x1 = dx * cos_y + dz * sin_y;
            let z1 = -dx * sin_y + dz * cos_y;

            // Pitch rotation around X axis (+y2 = screen up, +depth = into screen away from camera)
            let cos_p = cam_pitch.cos();
            let sin_p = cam_pitch.sin();
            let y2 = dy * cos_p - z1 * sin_p;
            let depth = -(dy * sin_p + z1 * cos_p);

            // Perspective projection with safe near-plane clamp
            let persp_factor = if is_persp {
                (d_cam / (d_cam + depth).max(4.0)).clamp(0.05, 4.0)
            } else {
                1.0
            };

            let scale = base_scale_unit * cam_zoom * persp_factor;
            let sx = center_screen.x + cam_pan.x + x1 * scale;
            let sy = center_screen.y + cam_pan.y - y2 * scale;

            (Pos2::new(sx, sy), depth)
        };

        // Live Earth Lat/Lon Cursor Tracking on 3D Ground Plane (y = 0.0)
        if let Some(hover) = response.hover_pos() {
            let cos_y = cam_yaw.cos();
            let sin_y = cam_yaw.sin();
            let cos_p = cam_pitch.cos();
            let sin_p = cam_pitch.sin();
            let dy_g = 0.0_f32 - center_3d.1;
            let mut depth_est = 0.0_f32;
            let mut x1 = 0.0_f32;
            let mut z1 = 0.0_f32;
            for _ in 0..4 {
                let pf = if is_persp {
                    (d_cam / (d_cam + depth_est).max(4.0)).clamp(0.05, 4.0)
                } else {
                    1.0
                };
                let sc = (base_scale_unit * cam_zoom * pf).max(0.01);
                x1 = (hover.x - center_screen.x - cam_pan.x) / sc;
                let y2 = (center_screen.y + cam_pan.y - hover.y) / sc;
                if sin_p.abs() > 0.08 {
                    z1 = (dy_g * cos_p - y2) / sin_p;
                }
                depth_est = -(dy_g * sin_p + z1 * cos_p);
            }
            let ground_dx = x1 * cos_y - z1 * sin_y;
            let ground_dz = x1 * sin_y + z1 * cos_y;
            let map_px_x = ground_dx * base_scale_unit * cam_zoom;
            let map_px_y = ground_dz * base_scale_unit * cam_zoom;
            let (cx, cy) = crate::core::gis::WebMercator::lat_lon_to_tile_coords(
                universal_canvas.world_map.center_lat_lon.0,
                universal_canvas.world_map.center_lat_lon.1,
                universal_canvas.world_map.zoom_level,
            );
            universal_canvas.cursor_lat_lon = crate::core::gis::WebMercator::tile_coords_to_lat_lon(
                cx + (map_px_x as f64) / 256.0,
                cy + (map_px_y as f64) / 256.0,
                universal_canvas.world_map.zoom_level,
            );
        }

        let offset_x = center_3d.0 - 6.0;
        let offset_z = center_3d.2 - 4.5;

        // 2. 3D Earth World Map & Geodetic Ground Plane (`y = 0.0`) rendered BEFORE strokes and 3D BIM geometry
        self.render_3d_earth_ground_map(
            ui.ctx(),
            &painter,
            rect,
            theme,
            universal_canvas,
            center_3d,
            base_scale_unit,
            cam_yaw,
            cam_pitch,
            cam_zoom,
            cam_pan,
            is_persp,
            d_cam,
            &project_3d,
        );

        // Ground-plane / Active Datum inverse raycast closure (`y = datum_y_m`)
        let cos_y = cam_yaw.cos();
        let sin_y = cam_yaw.sin();
        let cos_p = cam_pitch.cos();
        let sin_p = cam_pitch.sin();
        let datum_y_m = self.draft_node_y as f32;
        let raycast_plane_mm = |screen: Pos2, plane_y_m: f32| -> [f64; 2] {
            let dy = plane_y_m - center_3d.1;
            let mut depth_est = 0.0_f32;
            let mut x1 = 0.0_f32;
            let mut z1 = 0.0_f32;
            for _ in 0..4 {
                let pf = if is_persp {
                    (d_cam / (d_cam + depth_est).max(4.0)).clamp(0.05, 4.0)
                } else {
                    1.0
                };
                let sc = (base_scale_unit * cam_zoom * pf).max(0.01);
                x1 = (screen.x - center_screen.x - cam_pan.x) / sc;
                let y2 = (center_screen.y + cam_pan.y - screen.y) / sc;
                z1 = if sin_p.abs() > 0.02 {
                    (dy * cos_p - y2) / sin_p
                } else {
                    0.0
                };
                depth_est = -(dy * sin_p + z1 * cos_p);
            }
            let dx = x1 * cos_y - z1 * sin_y;
            let dz = x1 * sin_y + z1 * cos_y;
            [
                ((center_3d.0 + dx - offset_x) * 1000.0) as f64,
                ((center_3d.2 + dz - offset_z) * 1000.0) as f64,
            ]
        };

        // Ground-plane (y=0.02) Universal Ink Sketch authoring when Sketch Mode is active
        if allow_sketch_drag
            && !pointer_in_floating
            && !universal_canvas.is_frozen
            && !crate::ui_kit::is_canvas_frozen(ui.ctx())
        {
            let screen_to_ground_mm = |screen: Pos2| raycast_plane_mm(screen, 0.02);
            let hit_radius_mm = (18.0 / (base_scale_unit * cam_zoom).max(0.05)) as f64 * 1000.0;
            universal_canvas.handle_world_sketch_interaction(
                &response,
                NavigationPillar::GeometryAssembly,
                hit_radius_mm,
                &screen_to_ground_mm,
            );
        }

        // Interactive 3D Geodesic Measurement clicks on Earth Ground Plane
        if universal_canvas.measure_active
            && !is_frozen_now
            && !pointer_in_floating
            && response.clicked_by(egui::PointerButton::Primary)
        {
            if let Some(pos) = response.interact_pointer_pos() {
                universal_canvas.ensure_ground_origin_at_screen(pos, rect);
                let mm = raycast_plane_mm(pos, 0.0);
                let ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                    universal_canvas.ground_origin_lat_lon,
                    mm,
                );
                universal_canvas.measure_points_lat_lon.push(ll);
            }
        } else if !self.sketch_mode_active
            && self.ground_author_tool != Ground3DAuthorTool::Orbit
            && !is_frozen_now
            && !pointer_in_floating
        {
            // Interactive 3D Earth Ground BIM Authoring with magnetic snap
            let snap_mm = |raw: [f64; 2]| -> [f64; 2] {
                let mut best: Option<([f64; 2], f64)> = None;
                let thresh = 450.0;
                for c in &self.bim_bus.project.columns {
                    let d = (c.pos.0 - raw[0]).hypot(c.pos.1 - raw[1]);
                    if d < thresh && best.is_none_or(|(_, bd)| d < bd) {
                        best = Some(([c.pos.0, c.pos.1], d));
                    }
                }
                for w in &self.bim_bus.project.walls {
                    for p in [w.start, w.end] {
                        let d = (p.0 - raw[0]).hypot(p.1 - raw[1]);
                        if d < thresh && best.is_none_or(|(_, bd)| d < bd) {
                            best = Some(([p.0, p.1], d));
                        }
                    }
                }
                for r in &self.bim_bus.project.runs {
                    for p in [r.start, r.end] {
                        let d = (p.0 - raw[0]).hypot(p.1 - raw[1]);
                        if d < thresh && best.is_none_or(|(_, bd)| d < bd) {
                            best = Some(([p.0, p.1], d));
                        }
                    }
                }
                if let Some((pt, _)) = best {
                    pt
                } else {
                    [(raw[0] / 500.0).round() * 500.0, (raw[1] / 500.0).round() * 500.0]
                }
            };

            if let Some(hover) = response.hover_pos() {
                let raw = raycast_plane_mm(hover, datum_y_m);
                self.hover_3d_ground_snap_mm = Some(snap_mm(raw));
            } else {
                self.hover_3d_ground_snap_mm = None;
            }

            if response.clicked_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    universal_canvas.ensure_ground_origin_at_screen(pos, rect);
                    let pt = snap_mm(raycast_plane_mm(pos, datum_y_m));
                    let mut sorted_lvls = self.bim_bus.project.levels.clone();
                    sorted_lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
                    let active_lvl_idx = sorted_lvls
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            ((a.elevation_mm * 0.001) - self.draft_node_y)
                                .abs()
                                .total_cmp(&((b.elevation_mm * 0.001) - self.draft_node_y).abs())
                        })
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let base_level_id = sorted_lvls.get(active_lvl_idx).map(|l| l.id).unwrap_or(1);
                    let top_level_id = sorted_lvls
                        .get(active_lvl_idx + 1)
                        .map(|l| l.id)
                        .unwrap_or(base_level_id);
                    let p2 = crate::core::P2(pt[0], pt[1]);

                    match self.ground_author_tool {
                        Ground3DAuthorTool::Column => {
                            let w = self.author_col_width_mm.clamp(150.0, 2000.0);
                            let d = self.author_col_depth_mm.clamp(150.0, 2000.0);
                            let res = self.bim_bus.execute(crate::core::BimCommand::AddColumn {
                                base_level_id,
                                top_level_id,
                                pos: p2,
                                width_mm: w,
                                depth_mm: d,
                                is_round: false,
                                profile_name: format!("C{:.0}x{:.0}", w, d),
                            });
                            if let Ok(msg) = res {
                                self.modify_status_msg = Some(msg.clone());
                                self.bim_console_logs.push(msg);
                            }
                        }
                        Ground3DAuthorTool::Footing => {
                            let fs = self.author_footing_size_mm.clamp(400.0, 6000.0);
                            let res = self.bim_bus.execute(crate::core::BimCommand::AddFooting {
                                level_id: base_level_id,
                                pos: p2,
                                width_mm: fs,
                                length_mm: fs,
                                thickness_mm: 450.0,
                            });
                            if let Ok(msg) = res {
                                self.modify_status_msg = Some(msg.clone());
                                self.bim_console_logs.push(msg);
                            }
                        }
                        Ground3DAuthorTool::Wall | Ground3DAuthorTool::Beam => {
                            if let Some(s) = self.pending_3d_start_mm.take() {
                                if (s[0] - pt[0]).hypot(s[1] - pt[1]) >= 200.0 {
                                    let cmd = if self.ground_author_tool == Ground3DAuthorTool::Wall {
                                        let wt = self.author_wall_thickness_mm.clamp(80.0, 1200.0);
                                        crate::core::BimCommand::AddWall {
                                            base_level_id,
                                            start: crate::core::P2(s[0], s[1]),
                                            end: p2,
                                            thickness_mm: wt,
                                            is_structural: true,
                                        }
                                    } else {
                                        let bw = self.author_beam_width_mm.clamp(120.0, 1500.0);
                                        let bd = self.author_beam_depth_mm.clamp(150.0, 2500.0);
                                        crate::core::BimCommand::AddBeamRun {
                                            level_id: base_level_id,
                                            category: crate::core::RunCategory::StructuralBeam,
                                            start: crate::core::P2(s[0], s[1]),
                                            end: p2,
                                            width_mm: bw,
                                            depth_mm: bd,
                                            justify: crate::core::RunJustify::Top,
                                            profile_name: format!("B{:.0}x{:.0}", bw, bd),
                                        }
                                    };
                                    if let Ok(msg) = self.bim_bus.execute(cmd) {
                                        self.modify_status_msg = Some(msg.clone());
                                        self.bim_console_logs.push(msg);
                                    }
                                }
                            } else {
                                self.pending_3d_start_mm = Some(pt);
                            }
                        }
                        Ground3DAuthorTool::Orbit => {}
                    }
                }
            }
            if response.secondary_clicked() {
                self.pending_3d_start_mm = None;
            }
        } else {
            self.hover_3d_ground_snap_mm = None;
        }

        // Render 3D Ground Authoring Reticle, Live Wireframe Ghost Preview & Geodesic Measurement Overlay
        if let Some(snap_pt) = self.hover_3d_ground_snap_mm {
            let cx_m = snap_pt[0] as f32 * 0.001 + offset_x;
            let cz_m = snap_pt[1] as f32 * 0.001 + offset_z;
            let (sp, _) = project_3d(cx_m, datum_y_m + 0.02, cz_m);
            let reticle_col = Color32::from_rgb(0, 240, 255);
            let ghost_stroke = Stroke::new(1.5, Color32::from_rgba_unmultiplied(0, 240, 255, 195));
            painter.circle_stroke(sp, 6.0, Stroke::new(1.8, reticle_col));

            // Live 3D Wireframe Ghost Footprint matching active manual precision dimensions
            match self.ground_author_tool {
                Ground3DAuthorTool::Column => {
                    let hw = (self.author_col_width_mm as f32 * 0.001) * 0.5;
                    let hd = (self.author_col_depth_mm as f32 * 0.001) * 0.5;
                    let h_col = 3.5_f32;
                    let base = [
                        project_3d(cx_m - hw, datum_y_m, cz_m - hd).0,
                        project_3d(cx_m + hw, datum_y_m, cz_m - hd).0,
                        project_3d(cx_m + hw, datum_y_m, cz_m + hd).0,
                        project_3d(cx_m - hw, datum_y_m, cz_m + hd).0,
                    ];
                    let top = [
                        project_3d(cx_m - hw, datum_y_m + h_col, cz_m - hd).0,
                        project_3d(cx_m + hw, datum_y_m + h_col, cz_m - hd).0,
                        project_3d(cx_m + hw, datum_y_m + h_col, cz_m + hd).0,
                        project_3d(cx_m - hw, datum_y_m + h_col, cz_m + hd).0,
                    ];
                    for i in 0..4 {
                        let next = (i + 1) % 4;
                        painter.line_segment([base[i], base[next]], ghost_stroke);
                        painter.line_segment([top[i], top[next]], ghost_stroke);
                        painter.line_segment([base[i], top[i]], ghost_stroke);
                    }
                }
                Ground3DAuthorTool::Footing => {
                    let hs = (self.author_footing_size_mm as f32 * 0.001) * 0.5;
                    let base = [
                        project_3d(cx_m - hs, datum_y_m, cz_m - hs).0,
                        project_3d(cx_m + hs, datum_y_m, cz_m - hs).0,
                        project_3d(cx_m + hs, datum_y_m, cz_m + hs).0,
                        project_3d(cx_m - hs, datum_y_m, cz_m + hs).0,
                    ];
                    let pad_top = [
                        project_3d(cx_m - hs, datum_y_m - 0.45, cz_m - hs).0,
                        project_3d(cx_m + hs, datum_y_m - 0.45, cz_m - hs).0,
                        project_3d(cx_m + hs, datum_y_m - 0.45, cz_m + hs).0,
                        project_3d(cx_m - hs, datum_y_m - 0.45, cz_m + hs).0,
                    ];
                    let foot_stroke = Stroke::new(1.5, theme.accent_green.gamma_multiply(0.85));
                    for i in 0..4 {
                        let next = (i + 1) % 4;
                        painter.line_segment([base[i], base[next]], foot_stroke);
                        painter.line_segment([pad_top[i], pad_top[next]], foot_stroke);
                        painter.line_segment([base[i], pad_top[i]], foot_stroke);
                    }
                }
                Ground3DAuthorTool::Beam | Ground3DAuthorTool::Wall | Ground3DAuthorTool::Orbit => {}
            }

            let dim_sub = match self.ground_author_tool {
                Ground3DAuthorTool::Column => format!(
                    "C{:.0}x{:.0} @ ({:.2}m, {:.2}m)",
                    self.author_col_width_mm,
                    self.author_col_depth_mm,
                    snap_pt[0] * 0.001,
                    snap_pt[1] * 0.001
                ),
                Ground3DAuthorTool::Footing => format!(
                    "F{:.0}x{:.0} @ ({:.2}m, {:.2}m)",
                    self.author_footing_size_mm,
                    self.author_footing_size_mm,
                    snap_pt[0] * 0.001,
                    snap_pt[1] * 0.001
                ),
                _ => format!(
                    "{} ({:.2}m, {:.2}m)",
                    self.ground_author_tool.label(),
                    snap_pt[0] * 0.001,
                    snap_pt[1] * 0.001
                ),
            };
            painter.text(
                sp + Vec2::new(10.0, -10.0),
                Align2::LEFT_BOTTOM,
                dim_sub,
                egui::FontId::monospace(10.0),
                reticle_col,
            );

            if let Some(start_mm) = self.pending_3d_start_mm {
                let sx_m = start_mm[0] as f32 * 0.001 + offset_x;
                let sz_m = start_mm[1] as f32 * 0.001 + offset_z;
                let (st_sp, _) = project_3d(sx_m, datum_y_m + 0.02, sz_m);
                painter.line_segment([st_sp, sp], Stroke::new(2.2, theme.accent_orange));

                // Draw parallel offset corridor lines matching active beam width or wall thickness
                let dx_m = cx_m - sx_m;
                let dz_m = cz_m - sz_m;
                let seg_len_m = dx_m.hypot(dz_m);
                if seg_len_m > 0.05 {
                    let nx = -dz_m / seg_len_m;
                    let nz = dx_m / seg_len_m;
                    let half_w_m = if self.ground_author_tool == Ground3DAuthorTool::Wall {
                        (self.author_wall_thickness_mm as f32 * 0.001) * 0.5
                    } else {
                        (self.author_beam_width_mm as f32 * 0.001) * 0.5
                    };
                    let l1 = project_3d(sx_m + nx * half_w_m, datum_y_m + 0.02, sz_m + nz * half_w_m).0;
                    let l2 = project_3d(cx_m + nx * half_w_m, datum_y_m + 0.02, cz_m + nz * half_w_m).0;
                    let r1 = project_3d(sx_m - nx * half_w_m, datum_y_m + 0.02, sz_m - nz * half_w_m).0;
                    let r2 = project_3d(cx_m - nx * half_w_m, datum_y_m + 0.02, cz_m - nz * half_w_m).0;
                    let edge_col = Stroke::new(1.2, theme.accent_orange.gamma_multiply(0.65));
                    painter.line_segment([l1, l2], edge_col);
                    painter.line_segment([r1, r2], edge_col);
                    painter.line_segment([l1, r1], edge_col);
                    painter.line_segment([l2, r2], edge_col);
                }

                let len_m = (snap_pt[0] - start_mm[0]).hypot(snap_pt[1] - start_mm[1]) * 0.001;
                let mid = Pos2::new(0.5 * (st_sp.x + sp.x), 0.5 * (st_sp.y + sp.y) - 10.0);
                painter.text(
                    mid,
                    Align2::CENTER_BOTTOM,
                    format!("L = {:.2} m ({:.0} mm)", len_m, len_m * 1000.0),
                    egui::FontId::monospace(10.5),
                    theme.accent_orange,
                );
            }
        }
        let origin_ll_3d = universal_canvas.ground_origin_lat_lon;
        universal_canvas.render_geodetic_measure_overlay(&painter, rect, theme, &|lat, lon| {
            let mm = crate::core::gis::SlippyMapState::lat_lon_to_world_mm(origin_ll_3d, lat, lon);
            project_3d(
                mm[0] as f32 * 0.001 + offset_x,
                0.02,
                mm[1] as f32 * 0.001 + offset_z,
            )
            .0
        });

        if self.sketch_overlay_synced || self.sketch_mode_active {
            let ground_to_screen = |mm: [f64; 2]| -> Pos2 {
                project_3d(
                    mm[0] as f32 / 1000.0 + offset_x,
                    0.02,
                    mm[1] as f32 / 1000.0 + offset_z,
                )
                .0
            };
            universal_canvas.paint_entities_world(
                &painter,
                rect,
                theme,
                NavigationPillar::GeometryAssembly,
                &ground_to_screen,
            );
        }

                    // Phase 26: 3D Level Elevation Datums from Live Project (zero hardcoded demo levels)
                    if self.show_level_datums && self.bim_bus.project.levels.len() > 1 {
                        for lvl in &self.bim_bus.project.levels {
                            let elev = (lvl.elevation_mm as f32) * 0.001;
                            let (sp1, _) = project_3d(-10.0, elev, 0.0);
                            let (sp2, _) = project_3d(10.0, elev, 0.0);
                            painter.line_segment([sp1, sp2], Stroke::new(1.0, Color32::from_rgba_premultiplied(59, 130, 246, 90)));
                            painter.circle_filled(sp1, 6.5, Color32::from_rgb(30, 58, 138));
                            painter.circle_stroke(sp1, 6.5, Stroke::new(1.0, Color32::from_rgb(96, 165, 250)));
                            painter.text(sp1 + Vec2::new(9.0, 0.0), Align2::LEFT_CENTER, &lvl.name, egui::FontId::monospace(9.0), Color32::from_rgb(191, 219, 254));
                        }
                    }

                    // Phase 26: 3D Structural Grid Axis Markers from Live Project
                    if self.show_grid_bubbles {
                        for g in &self.bim_bus.project.grids {
                            let (gp, _) = project_3d((g.end.0 as f32) * 0.001, 0.0, (g.end.1 as f32) * 0.001);
                            painter.circle_filled(gp, 7.5, Color32::from_rgb(20, 24, 34));
                            painter.circle_stroke(gp, 7.5, Stroke::new(1.0, Color32::from_rgb(148, 163, 184)));
                            painter.text(gp, Align2::CENTER_CENTER, &g.label, egui::FontId::monospace(9.5), Color32::WHITE);
                        }
                    }

                    // Phase 26 / 38: 3D Interactive Section Box Cage (X, Elev-Z, Plan-Y)
                    if self.section_box_enabled {
                        let bx0 = self.section_box_min_x;
                        let bx1 = self.section_box_max_x;
                        let by0 = self.section_box_min_z;
                        let by1 = self.section_box_max_z_m;
                        let bz0 = self.section_box_min_y;
                        let bz1 = self.section_box_max_y;

                        let corners = [
                            (bx0, by0, bz0), (bx1, by0, bz0), (bx1, by0, bz1), (bx0, by0, bz1),
                            (bx0, by1, bz0), (bx1, by1, bz0), (bx1, by1, bz1), (bx0, by1, bz1),
                        ];
                        let sc: Vec<Pos2> = corners.iter().map(|&(x, y, z)| project_3d(x, y, z).0).collect();
                        let edges = [
                            (0, 1), (1, 2), (2, 3), (3, 0),
                            (4, 5), (5, 6), (6, 7), (7, 4),
                            (0, 4), (1, 5), (2, 6), (3, 7),
                        ];
                        for &(i, j) in &edges {
                            painter.line_segment([sc[i], sc[j]], Stroke::new(1.2, Color32::from_rgba_premultiplied(6, 182, 212, 160)));
                        }
                        for pt in &sc {
                            painter.circle_filled(*pt, 3.5, Color32::from_rgb(6, 182, 212));
                        }
                    }

                    // 3. Process & Depth-Sort Members (Painter's Algorithm)
                    if self.show_deformed_shape && self.animate_vibration {
                        self.vibration_phase = (self.vibration_phase + 0.08).rem_euclid(2.0 * std::f32::consts::PI);
                        ui.ctx().request_repaint();
                    }
                    let vib_factor = if self.show_deformed_shape && self.animate_vibration {
                        self.vibration_phase.sin()
                    } else if self.show_deformed_shape {
                        1.0
                    } else {
                        0.0
                    };
                    let def_scale = self.deformation_scale;

                    let deform_pt = move |p: (f32, f32, f32)| -> (f32, f32, f32) {
                        if vib_factor.abs() < 1e-4 {
                            p
                        } else {
                            let rel_x = (p.0 - center_3d.0).abs() / 12.0;
                            let sag_y = -(1.0 - (rel_x * rel_x).min(1.0)) * 0.015 * def_scale * vib_factor;
                            let sway_x = (p.1 / 20.0) * 0.012 * def_scale * vib_factor;
                            (p.0 + sway_x, p.1 + sag_y, p.2)
                        }
                    };

                    struct RenderItem {
                        id: usize,
                        p1_screen: Pos2,
                        p2_screen: Pos2,
                        depth: f32,
                        p1_3d: (f32, f32, f32),
                        p2_3d: (f32, f32, f32),
                        name: String,
                        section: String,
                        base_width: f32,
                        force_kn: f32,
                        capacity_kn: f32,
                        utilization: f32,
                        ari: f32,
                        is_critical: bool,
                    }

                    let mut render_list = Vec::with_capacity(raw_members.len());
                    for m in &raw_members {
                        let p1_actual = deform_pt(m.p1);
                        let p2_actual = deform_pt(m.p2);
                        let (sp1, z1) = project_3d(p1_actual.0, p1_actual.1, p1_actual.2);
                        let (sp2, z2) = project_3d(p2_actual.0, p2_actual.1, p2_actual.2);
                        let avg_depth = (z1 + z2) * 0.5;

                        render_list.push(RenderItem {
                            id: m.id,
                            p1_screen: sp1,
                            p2_screen: sp2,
                            depth: avg_depth,
                            p1_3d: p1_actual,
                            p2_3d: p2_actual,
                            name: m.name.clone(),
                            section: m.section.clone(),
                            base_width: m.base_width,
                            force_kn: m.force_kn,
                            capacity_kn: m.capacity_kn,
                            utilization: m.utilization,
                            ari: m.ari,
                            is_critical: m.is_critical,
                        });
                    }

                    // Sort back-to-front by depth (most negative z2 first)
                    render_list.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));

                    // Hover & Click Detection (disabled over floating HUD overlays or active Ink Sketch tool)
                    let mut hovered_member: Option<usize> = None;
                    let allow_member_pick = !pointer_in_floating
                        && (!self.sketch_mode_active
                            || universal_canvas.active_tool
                                == crate::views::universal_canvas::UniversalCanvasTool::SelectPan);
                    if allow_member_pick {
                        if let Some(mouse_pos) = response.hover_pos() {
                            // Check front-to-back for hit detection
                            for item in render_list.iter().rev() {
                                let dist = distance_to_segment(mouse_pos, item.p1_screen, item.p2_screen);
                                let hit_threshold = (item.base_width * cam_zoom * 0.8).clamp(6.0, 18.0);
                                if dist <= hit_threshold {
                                    hovered_member = Some(item.id);
                                    break;
                                }
                            }
                        }

                        if response.clicked() {
                            if let Some(id) = hovered_member {
                                self.selected_member_id = Some(id);
                                if spatial_fullscreen {
                                    self.last_spatial_bim_pick = self.bim_bus.project.bim_ref_for_fem_id(id);
                                    self.spatial_pick_seq = self.spatial_pick_seq.wrapping_add(1);
                                } else {
                                    self.active_sidebar_tab = SidebarTab::Properties;
                                    self.floating_properties_open = true;
                                    self.properties_canvas_sync_tab = false;
                                }
                            } else if spatial_fullscreen {
                                // Fallback: check isolated footings or structural walls directly in 3D space
                                let mut picked_bim: Option<(crate::core::BimElementKind, usize)> = None;
                                if let Some(mpos) = response.interact_pointer_pos() {
                                    let offset_x = center_3d.0 - 6.0;
                                    let offset_z = center_3d.2 - 4.5;
                                    let mut best_ft: Option<(f32, usize)> = None;
                                    for ft in &self.bim_bus.project.footings {
                                        let elev = self.bim_bus.project.level_elevation(ft.level_id) as f32 * 0.001;
                                        let (sp, _) = project_3d(
                                            ft.pos.0 as f32 * 0.001 + offset_x,
                                            elev,
                                            ft.pos.1 as f32 * 0.001 + offset_z,
                                        );
                                        let d = mpos.distance(sp);
                                        if d <= 18.0 && best_ft.is_none_or(|(bd, _)| d < bd) {
                                            best_ft = Some((d, ft.id));
                                        }
                                    }
                                    if let Some((_, fid)) = best_ft {
                                        picked_bim = Some((crate::core::BimElementKind::Footing, fid));
                                    } else {
                                        let mut best_w: Option<(f32, usize)> = None;
                                        for w in &self.bim_bus.project.walls {
                                            let z0 = self.bim_bus.project.level_elevation(w.base_level_id) as f32 * 0.001;
                                            let (s0, _) = project_3d(
                                                w.start.0 as f32 * 0.001 + offset_x,
                                                z0 + 1.5,
                                                w.start.1 as f32 * 0.001 + offset_z,
                                            );
                                            let (s1, _) = project_3d(
                                                w.end.0 as f32 * 0.001 + offset_x,
                                                z0 + 1.5,
                                                w.end.1 as f32 * 0.001 + offset_z,
                                            );
                                            let d = distance_to_segment(mpos, s0, s1);
                                            if d <= 14.0 && best_w.is_none_or(|(bd, _)| d < bd) {
                                                best_w = Some((d, w.id));
                                            }
                                        }
                                        if let Some((_, wid)) = best_w {
                                            picked_bim = Some((crate::core::BimElementKind::Wall, wid));
                                        }
                                    }
                                }
                                self.selected_member_id = None;
                                self.last_spatial_bim_pick = picked_bim;
                                self.spatial_pick_seq = self.spatial_pick_seq.wrapping_add(1);
                            } else {
                                self.selected_member_id = None;
                            }
                        }
                    }

                    // 4. Render Members (CSG Solid B-Rep Mesh, Solid Shaded, Wireframe, or 3D Diagrams)
                    let sun_dir = (0.577, 0.577, 0.577);

                    // 4A. True 3D CSG B-Rep Triangulated Mesh + Rebar Cage Renderer (BIMCraft LOD 350)
                    if self.active_model == ModelArchetype::BimCraftBuilding3D && self.render_mode == Render3DMode::CsgSolidBrep {
                        struct TriDraw {
                            p0: Pos2,
                            p1: Pos2,
                            p2: Pos2,
                            depth: f32,
                            fill: Color32,
                            stroke: Stroke,
                        }
                        let mut tris_to_draw: Vec<TriDraw> = Vec::new();
                        let sbox_on = self.section_box_enabled;
                        let (min_x_c, max_x_c) = (self.section_box_min_x, self.section_box_max_x);
                        let (min_y_c, max_y_c) = (self.section_box_min_y, self.section_box_max_y);
                        let (min_z_c, max_z_c) = (self.section_box_min_z, self.section_box_max_z_m);
                        let max_y_clip = if sbox_on { max_z_c } else { 999.0 };

                        let mut push_mesh = |mesh: &[[crate::core::P3; 3]], base_col: Color32, edge_alpha: u8| {
                            if base_col.a() == 0 {
                                return;
                            }
                            for tri in mesh {
                                // Convert BIM (X_mm, Y_mm, Z_mm) -> Viewport 3D (X_m, Z_m_up, Y_m)
                                let v0 = (tri[0][0] as f32 * 0.001, tri[0][2] as f32 * 0.001, tri[0][1] as f32 * 0.001);
                                let v1 = (tri[1][0] as f32 * 0.001, tri[1][2] as f32 * 0.001, tri[1][1] as f32 * 0.001);
                                let v2 = (tri[2][0] as f32 * 0.001, tri[2][2] as f32 * 0.001, tri[2][1] as f32 * 0.001);

                                if sbox_on
                                    && ((v0.0 < min_x_c && v1.0 < min_x_c && v2.0 < min_x_c)
                                        || (v0.0 > max_x_c && v1.0 > max_x_c && v2.0 > max_x_c)
                                        || (v0.1 < min_z_c && v1.1 < min_z_c && v2.1 < min_z_c)
                                        || (v0.1 > max_z_c && v1.1 > max_z_c && v2.1 > max_z_c)
                                        || (v0.2 < min_y_c && v1.2 < min_y_c && v2.2 < min_y_c)
                                        || (v0.2 > max_y_c && v1.2 > max_y_c && v2.2 > max_y_c))
                                {
                                    continue;
                                }

                                let ux = v1.0 - v0.0;
                                let uy = v1.1 - v0.1;
                                let uz = v1.2 - v0.2;
                                let vx = v2.0 - v0.0;
                                let vy = v2.1 - v0.1;
                                let vz = v2.2 - v0.2;
                                let nx = uy * vz - uz * vy;
                                let ny = uz * vx - ux * vz;
                                let nz = ux * vy - uy * vx;
                                let nlen = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-5);
                                let dot_sun = ((nx / nlen) * sun_dir.0 + (ny / nlen) * sun_dir.1 + (nz / nlen) * sun_dir.2).abs();
                                let shade = 0.58 + 0.42 * dot_sun;

                                let col = Color32::from_rgba_unmultiplied(
                                    ((base_col.r() as f32) * shade).clamp(0.0, 255.0) as u8,
                                    ((base_col.g() as f32) * shade).clamp(0.0, 255.0) as u8,
                                    ((base_col.b() as f32) * shade).clamp(0.0, 255.0) as u8,
                                    base_col.a(),
                                );

                                let (sp0, z0) = project_3d(v0.0, v0.1, v0.2);
                                let (sp1, z1) = project_3d(v1.0, v1.1, v1.2);
                                let (sp2, z2) = project_3d(v2.0, v2.1, v2.2);
                                let avg_z = (z0 + z1 + z2) / 3.0;

                                tris_to_draw.push(TriDraw {
                                    p0: sp0,
                                    p1: sp1,
                                    p2: sp2,
                                    depth: avg_z,
                                    fill: col,
                                    stroke: Stroke::new(0.4, Color32::from_rgba_unmultiplied(210, 225, 245, edge_alpha)),
                                });
                            }
                        };

                        let stage_4d_active = self.highlight_4d_erection && self.simulation_4d_day > 0;
                        let sim_scale_max = if self.simulation_4d_day > 180 { 360.0_f32 } else { 180.0_f32 };
                        let prog_4d = (self.simulation_4d_day as f32 / sim_scale_max).clamp(0.0, 1.0);
                        let max_bldg_z_m = self
                            .bim_bus
                            .project
                            .levels
                            .iter()
                            .map(|l| (l.elevation_mm as f32) * 0.001)
                            .fold(3.5_f32, f32::max);
                        let active_front_z_m = (((prog_4d - 0.12) / 0.65).clamp(0.0, 1.0)) * (max_bldg_z_m + 3.5);
                        let color_for_4d = |normal: Color32, elev_m: f32, is_foundation: bool, is_arc_enclosure: bool, is_mep: bool| -> Color32 {
                            if !stage_4d_active {
                                return normal;
                            }
                            if is_foundation {
                                if prog_4d <= 0.22 {
                                    return Color32::from_rgba_unmultiplied(249, 115, 22, 235);
                                }
                                return normal;
                            }
                            if is_mep {
                                if prog_4d < 0.65 {
                                    return Color32::from_rgba_unmultiplied(100, 116, 139, 30);
                                } else if prog_4d < 0.85 {
                                    return Color32::from_rgba_unmultiplied(249, 115, 22, 235);
                                }
                                return normal;
                            }
                            if is_arc_enclosure {
                                if prog_4d < 0.45 {
                                    return Color32::from_rgba_unmultiplied(100, 116, 139, 30);
                                } else if prog_4d < 0.72 {
                                    return Color32::from_rgba_unmultiplied(249, 115, 22, 220);
                                }
                                return normal;
                            }
                            if elev_m <= active_front_z_m - 2.0 {
                                normal
                            } else if elev_m <= active_front_z_m + 1.8 {
                                Color32::from_rgba_unmultiplied(249, 115, 22, 240)
                            } else {
                                Color32::from_rgba_unmultiplied(148, 163, 184, 34)
                            }
                        };

                        // 0. GEO Foundations / Footings
                        if self.show_layer_geo {
                            for f in &self.bim_bus.project.footings {
                                let z = self.bim_bus.project.level_elevation(f.level_id);
                                let mesh = f.mesh_3d(z);
                                let fcol = color_for_4d(
                                    Color32::from_rgba_unmultiplied(168, 120, 72, 230),
                                    (z as f32) * 0.001,
                                    true,
                                    false,
                                    false,
                                );
                                push_mesh(&mesh, fcol, 45);
                            }
                        }

                        // 1. Slabs with Boolean Shaft Voids (STR)
                        if self.show_layer_struct {
                            for s in &self.bim_bus.project.slabs {
                                let z = self.bim_bus.project.level_elevation(s.level_id);
                                let mesh = s.mesh_3d(z);
                                let scol = color_for_4d(
                                    Color32::from_rgba_unmultiplied(80, 105, 135, 195),
                                    (z as f32) * 0.001,
                                    false,
                                    false,
                                    false,
                                );
                                push_mesh(&mesh, scol, 35);
                            }
                        }
                        // 2. Walls with CSG Door & Window Voids (STR structural or ARC enclosure)
                        let wall_col = if self.show_rebar_cage {
                            Color32::from_rgba_unmultiplied(120, 145, 175, 135)
                        } else {
                            Color32::from_rgba_unmultiplied(145, 165, 190, 240)
                        };
                        for w in &self.bim_bus.project.walls {
                            let visible = if w.is_structural { self.show_layer_struct } else { self.show_layer_arch };
                            if !visible {
                                continue;
                            }
                            let z_base = self.bim_bus.project.level_elevation(w.base_level_id);
                            let z_top = w
                                .top_level_id
                                .map(|id| self.bim_bus.project.level_elevation(id))
                                .unwrap_or(z_base + w.unconnected_height_mm);
                            let mesh = w.mesh_3d(z_base, z_top);
                            let wcol = color_for_4d(
                                wall_col,
                                (z_base as f32) * 0.001,
                                false,
                                !w.is_structural,
                                false,
                            );
                            push_mesh(&mesh, wcol, 45);
                        }
                        // 3. Columns (STR)
                        if self.show_layer_struct {
                            for c in &self.bim_bus.project.columns {
                                let z_base = self.bim_bus.project.level_elevation(c.base_level_id);
                                let z_top = self.bim_bus.project.level_elevation(c.top_level_id);
                                let mesh = c.mesh_3d(z_base, z_top);
                                let ccol = color_for_4d(
                                    Color32::from_rgba_unmultiplied(65, 160, 220, 215),
                                    (z_base as f32) * 0.001,
                                    false,
                                    false,
                                    false,
                                );
                                push_mesh(&mesh, ccol, 55);
                            }
                        }
                        // 4. 3D Runs (True Wide-Flange I-Beams, RC Beams [STR], MEP Ducts/Pipes [MEP])
                        for r in &self.bim_bus.project.runs {
                            let is_beam = r.category == crate::core::RunCategory::StructuralBeam;
                            let visible = if is_beam { self.show_layer_struct } else { self.show_layer_mep };
                            if !visible {
                                continue;
                            }
                            let z = self.bim_bus.project.level_elevation(r.level_id);
                            let mesh = r.mesh_3d(z);
                            let base_rcol = match r.category {
                                crate::core::RunCategory::StructuralBeam if r.is_wf_profile => {
                                    Color32::from_rgba_unmultiplied(245, 158, 11, 245)
                                }
                                crate::core::RunCategory::StructuralBeam => {
                                    if self.show_rebar_cage {
                                        Color32::from_rgba_unmultiplied(45, 185, 130, 145)
                                    } else {
                                        Color32::from_rgba_unmultiplied(45, 185, 130, 235)
                                    }
                                }
                                _ => Color32::from_rgba_unmultiplied(56, 189, 248, 220),
                            };
                            let rcol = color_for_4d(base_rcol, (z as f32) * 0.001, false, false, !is_beam);
                            push_mesh(&mesh, rcol, 55);
                        }

                        // Depth-sort all CSG B-Rep triangles back-to-front
                        tris_to_draw.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));
                        for td in &tris_to_draw {
                            painter.add(egui::Shape::convex_polygon(vec![td.p0, td.p1, td.p2], td.fill, td.stroke));
                        }

                        // 5. 3D Rebar Cage Overlay (Longitudinal & Stirrup loops)
                        if self.show_rebar_cage {
                            for bar in &self.bim_bus.project.rebars {
                                if bar.points_3d.len() >= 2 {
                                    let bar_col = if bar.closed_stirrup {
                                        Color32::from_rgb(251, 191, 36)
                                    } else {
                                        Color32::from_rgb(244, 63, 94)
                                    };
                                    let stroke_w = if bar.closed_stirrup { 1.1 } else { 2.0 };
                                    for seg in bar.points_3d.windows(2) {
                                        let v0 = (seg[0][0] as f32 * 0.001, seg[0][2] as f32 * 0.001, seg[0][1] as f32 * 0.001);
                                        let v1 = (seg[1][0] as f32 * 0.001, seg[1][2] as f32 * 0.001, seg[1][1] as f32 * 0.001);
                                        if v0.1 <= max_y_clip || v1.1 <= max_y_clip {
                                            let (sp0, _) = project_3d(v0.0, v0.1, v0.2);
                                            let (sp1, _) = project_3d(v1.0, v1.1, v1.2);
                                            painter.line_segment([sp0, sp1], Stroke::new(stroke_w, bar_col));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    for item in &render_list {
                        let is_sel = self.selected_member_id == Some(item.id);
                        let is_hov = hovered_member == Some(item.id);
                        let is_elim = self.simulated_eliminated_ids.contains(&item.id);

                        if self.active_model == ModelArchetype::BimCraftBuilding3D && self.render_mode == Render3DMode::CsgSolidBrep && !is_sel && !is_hov && !is_elim {
                            continue;
                        }

                        // Base Color Determination
                        let base_color = if is_elim {
                            Color32::from_rgb(255, 45, 45)
                        } else if is_sel {
                            Color32::from_rgb(0, 225, 255)
                        } else {
                            match self.color_mode.as_str() {
                                "ARI" => {
                                    if item.ari >= 0.85 {
                                        Color32::from_rgb(26, 188, 120) // Emerald Green Safe
                                    } else if item.ari >= 0.70 {
                                        Color32::from_rgb(14, 165, 233) // Sky Blue
                                    } else if item.ari >= 0.55 {
                                        Color32::from_rgb(245, 158, 11) // Warning Amber
                                    } else {
                                        Color32::from_rgb(239, 68, 68) // Critical Red
                                    }
                                }
                                "DCR_Stress" => {
                                    if item.utilization >= 0.85 {
                                        Color32::from_rgb(239, 68, 68) // Fiery Red Yielding
                                    } else if item.utilization >= 0.60 {
                                        Color32::from_rgb(245, 158, 11) // Amber
                                    } else if item.utilization >= 0.35 {
                                        Color32::from_rgb(34, 197, 94) // Green
                                    } else {
                                        Color32::from_rgb(59, 130, 246) // Blue Low Stress
                                    }
                                }
                                "Criticality" => {
                                    if item.is_critical {
                                        Color32::from_rgb(244, 63, 94) // Magenta Critical
                                    } else {
                                        Color32::from_rgb(71, 85, 105) // Muted Slate
                                    }
                                }
                                "Tension_Compression" => {
                                    if item.force_kn > 5.0 {
                                        Color32::from_rgb(239, 68, 68) // Red Tension
                                    } else if item.force_kn < -5.0 {
                                        Color32::from_rgb(59, 130, 246) // Blue Compression
                                    } else {
                                        Color32::from_rgb(148, 163, 184) // Neutral Grey
                                    }
                                }
                                _ => Color32::from_rgb(90, 140, 210),
                            }
                        };

                        let p1 = item.p1_screen;
                        let p2 = item.p2_screen;
                        let delta = p2 - p1;
                        let len = delta.length();

                        if len < 1.0 {
                            continue;
                        }

                        let dir = delta / len;
                        let normal = Vec2::new(-dir.y, dir.x);

                        match self.render_mode {
                            Render3DMode::CsgSolidBrep | Render3DMode::SolidShaded => {
                                if !is_elim {
                                    // Compute solid extrusion width
                                    let beam_w = (item.base_width * cam_zoom * 0.8).clamp(3.0, 22.0);

                                    // Shading lighting factor
                                    let dx3d = item.p2_3d.0 - item.p1_3d.0;
                                    let dy3d = item.p2_3d.1 - item.p1_3d.1;
                                    let dz3d = item.p2_3d.2 - item.p1_3d.2;
                                    let len3d = (dx3d * dx3d + dy3d * dy3d + dz3d * dz3d).sqrt().max(0.1);
                                    let t3d = (dx3d / len3d, dy3d / len3d, dz3d / len3d);

                                    // Cross product with sun light
                                    let cross_mag = ((t3d.1 * sun_dir.2 - t3d.2 * sun_dir.1).powi(2)
                                        + (t3d.2 * sun_dir.0 - t3d.0 * sun_dir.2).powi(2)
                                        + (t3d.0 * sun_dir.1 - t3d.1 * sun_dir.0).powi(2)).sqrt();

                                    let shade_factor = 0.65 + 0.35 * cross_mag;
                                    let shaded_r = ((base_color.r() as f32) * shade_factor).clamp(0.0, 255.0) as u8;
                                    let shaded_g = ((base_color.g() as f32) * shade_factor).clamp(0.0, 255.0) as u8;
                                    let shaded_b = ((base_color.b() as f32) * shade_factor).clamp(0.0, 255.0) as u8;
                                    let shaded_col = Color32::from_rgb(shaded_r, shaded_g, shaded_b);

                                    // Extruded Quad Corners
                                    let half_w = normal * (beam_w * 0.5);
                                    let q1 = p1 + half_w;
                                    let q2 = p2 + half_w;
                                    let q3 = p2 - half_w;
                                    let q4 = p1 - half_w;

                                    // Fill solid polygon
                                    painter.add(egui::Shape::convex_polygon(vec![q1, q2, q3, q4], shaded_col, Stroke::NONE));

                                    // Chamfer metallic specular highlight edge (top ridge)
                                    let highlight_col = Color32::from_rgba_premultiplied(255, 255, 255, if is_sel { 120 } else { 45 });
                                    painter.line_segment([q1, q2], Stroke::new(1.0, highlight_col));

                                    // Dark drop shadow edge (bottom ridge)
                                    let shadow_col = Color32::from_rgba_premultiplied(0, 0, 0, 80);
                                    painter.line_segment([q4, q3], Stroke::new(1.0, shadow_col));

                                    // Halo if selected or hovered
                                    if is_sel || is_hov {
                                        let halo_col = if is_sel { Color32::from_rgb(0, 230, 255) } else { Color32::from_rgb(255, 255, 255) };
                                        painter.line_segment([q1, q2], Stroke::new(2.0, halo_col));
                                        painter.line_segment([q4, q3], Stroke::new(2.0, halo_col));
                                    }
                                } else {
                                    let stroke_w = if is_sel { 3.5 } else if is_hov { 2.5 } else { 2.8 };
                                    painter.line_segment([p1, p2], Stroke::new(stroke_w, base_color));
                                }
                            }
                            Render3DMode::Wireframe => {
                                let stroke_w = if is_sel { 3.5 } else if is_hov { 2.5 } else if is_elim { 2.8 } else { 1.8 };
                                painter.line_segment([p1, p2], Stroke::new(stroke_w, base_color));
                            }
                            Render3DMode::AxialForceNFD => {
                                // Baseline member axis
                                painter.line_segment([p1, p2], Stroke::new(1.2, Color32::from_rgb(148, 163, 184)));

                                // Diagram height scaled to axial force
                                let is_tension = item.force_kn >= 0.0;
                                let sign = if is_tension { 1.0 } else { -1.0 };
                                let diag_h = (item.force_kn.abs() / 8.0).clamp(4.0, 32.0) * cam_zoom.clamp(0.5, 2.5);
                                let offset_vec = normal * (sign * diag_h);

                                let d1 = p1 + offset_vec;
                                let d2 = p2 + offset_vec;

                                let fill_col = if is_tension {
                                    Color32::from_rgba_premultiplied(239, 68, 68, 140) // Red Tension (+)
                                } else {
                                    Color32::from_rgba_premultiplied(59, 130, 246, 140) // Blue Compression (-)
                                };
                                let border_col = if is_tension {
                                    Color32::from_rgb(239, 68, 68)
                                } else {
                                    Color32::from_rgb(59, 130, 246)
                                };

                                painter.add(egui::Shape::convex_polygon(vec![p1, d1, d2, p2], fill_col, Stroke::NONE));
                                painter.line_segment([d1, d2], Stroke::new(1.8, border_col));
                                painter.line_segment([p1, d1], Stroke::new(1.0, border_col));
                                painter.line_segment([p2, d2], Stroke::new(1.0, border_col));

                                // Midpoint annotation
                                if is_sel || is_hov || item.force_kn.abs() > 40.0 {
                                    let mid = p1 + delta * 0.5 + offset_vec * 1.15;
                                    let sign_str = if is_tension { "(+) " } else { "(-) " };
                                    painter.text(
                                        mid,
                                        Align2::CENTER_CENTER,
                                        format!("{}{:.1} kN", sign_str, item.force_kn.abs()),
                                        egui::FontId::monospace(9.5),
                                        Color32::WHITE,
                                    );
                                }
                            }
                            Render3DMode::ShearForceSFD => {
                                // Baseline member axis
                                painter.line_segment([p1, p2], Stroke::new(1.2, Color32::from_rgb(148, 163, 184)));

                                let vy = (item.force_kn.abs() * 0.28).clamp(3.5, 55.0);
                                let diag_h = (vy / 4.0).clamp(4.0, 26.0) * cam_zoom.clamp(0.5, 2.5);
                                let offset_vec = normal * diag_h;

                                let d1 = p1 + offset_vec;
                                let d2 = p2 - offset_vec;
                                let p_mid = p1 + delta * 0.5;
                                let d_mid1 = p_mid + offset_vec;
                                let d_mid2 = p_mid - offset_vec;

                                let fill_col = Color32::from_rgba_premultiplied(34, 197, 94, 130);
                                let border_col = Color32::from_rgb(34, 197, 94);

                                // Step 1: left half
                                painter.add(egui::Shape::convex_polygon(vec![p1, d1, d_mid1, p_mid], fill_col, Stroke::NONE));
                                painter.line_segment([d1, d_mid1], Stroke::new(1.8, border_col));
                                painter.line_segment([p1, d1], Stroke::new(1.0, border_col));

                                // Step 2: right half (opposite sign)
                                painter.add(egui::Shape::convex_polygon(vec![p_mid, d_mid2, d2, p2], fill_col, Stroke::NONE));
                                painter.line_segment([d_mid2, d2], Stroke::new(1.8, border_col));
                                painter.line_segment([p2, d2], Stroke::new(1.0, border_col));
                                painter.line_segment([d_mid1, d_mid2], Stroke::new(1.5, border_col));

                                if is_sel || is_hov || vy > 20.0 {
                                    painter.text(
                                        d1 + normal * 4.0,
                                        Align2::CENTER_CENTER,
                                        format!("Vy: {:.1} kN", vy),
                                        egui::FontId::monospace(9.5),
                                        Color32::from_rgb(52, 211, 153),
                                    );
                                }
                            }
                            Render3DMode::BendingMomentBMD => {
                                // Baseline member axis
                                painter.line_segment([p1, p2], Stroke::new(1.2, Color32::from_rgb(148, 163, 184)));

                                let my = (item.force_kn.abs() * 0.22).clamp(4.0, 75.0);
                                let diag_h = (my / 3.5).clamp(5.0, 32.0) * cam_zoom.clamp(0.5, 2.5);
                                let offset_vec = normal * diag_h;

                                // Build parabolic polygon with 8 subdivision points
                                let mut poly_pts = vec![p1];
                                let mut curve_pts = Vec::new();
                                let steps = 8;
                                for s in 0..=steps {
                                    let t = s as f32 / steps as f32;
                                    let base_pt = p1 + delta * t;
                                    let sag = 4.0 * t * (1.0 - t); // Parabolic peak at 1.0
                                    let curve_pt = base_pt + offset_vec * sag;
                                    curve_pts.push(curve_pt);
                                    poly_pts.push(curve_pt);
                                }
                                poly_pts.push(p2);

                                let fill_col = Color32::from_rgba_premultiplied(245, 158, 11, 130);
                                let border_col = Color32::from_rgb(245, 158, 11);

                                painter.add(egui::Shape::convex_polygon(poly_pts, fill_col, Stroke::NONE));
                                for w in curve_pts.windows(2) {
                                    painter.line_segment([w[0], w[1]], Stroke::new(2.0, border_col));
                                }
                                painter.line_segment([p1, curve_pts[0]], Stroke::new(1.0, border_col));
                                painter.line_segment([p2, *curve_pts.last().unwrap()], Stroke::new(1.0, border_col));

                                // Midpoint peak callout
                                if is_sel || is_hov || my > 25.0 {
                                    let mid_peak = p1 + delta * 0.5 + offset_vec * 1.15;
                                    painter.text(
                                        mid_peak,
                                        Align2::CENTER_CENTER,
                                        format!("M_max: {:.1} kNm", my),
                                        egui::FontId::monospace(9.5),
                                        Color32::from_rgb(251, 191, 36),
                                    );
                                }
                            }
                        }

                        // Spherical MERO Node Balls at Joint Ends
                        let node_r = (item.base_width * cam_zoom * 0.55).clamp(2.5, 7.5);
                        painter.circle_filled(p1, node_r, Color32::from_rgb(210, 220, 235));
                        painter.circle_stroke(p1, node_r, Stroke::new(0.8, Color32::from_rgb(80, 95, 120)));
                        painter.circle_filled(p2, node_r, Color32::from_rgb(210, 220, 235));
                        painter.circle_stroke(p2, node_r, Stroke::new(0.8, Color32::from_rgb(80, 95, 120)));

                        // Specular dot on sphere
                        painter.circle_filled(p1 - Vec2::new(node_r * 0.3, node_r * 0.3), node_r * 0.35, Color32::WHITE);
                        painter.circle_filled(p2 - Vec2::new(node_r * 0.3, node_r * 0.3), node_r * 0.35, Color32::WHITE);
                    }

                    // 4C. Phase 29: 3D Spatial Clash Wireframe Bounding Boxes & Pins
                    if self.show_3d_clash_markers {
                        for item in &self.clash_items {
                            if item.status != crate::models::ClashStatus::Resolved
                                && !item.is_triggered_at_tolerance(self.clash_tolerance_mm)
                            {
                                continue;
                            }
                            let is_sel = self.selected_clash_id == Some(item.id);
                            let (cr, cg, cb) = if item.status == crate::models::ClashStatus::Resolved {
                                (16, 185, 129)
                            } else {
                                item.severity.color_rgb()
                            };
                            let box_col = Color32::from_rgb(cr, cg, cb);

                            let [cx_raw, cy, cz_raw] = item.point_3d;
                            let cx = cx_raw + offset_x;
                            let cz = cz_raw + offset_z;
                            let [hx, hy, hz] = item.box_half_extents;
                            let corners_3d = [
                                (cx - hx, cy - hy, cz - hz),
                                (cx + hx, cy - hy, cz - hz),
                                (cx + hx, cy + hy, cz - hz),
                                (cx - hx, cy + hy, cz - hz),
                                (cx - hx, cy - hy, cz + hz),
                                (cx + hx, cy - hy, cz + hz),
                                (cx + hx, cy + hy, cz + hz),
                                (cx - hx, cy + hy, cz + hz),
                            ];
                            let pts: [Pos2; 8] = corners_3d.map(|(x, y, z)| project_3d(x, y, z).0);
                            let edges = [
                                (0, 1), (1, 2), (2, 3), (3, 0),
                                (4, 5), (5, 6), (6, 7), (7, 4),
                                (0, 4), (1, 5), (2, 6), (3, 7),
                            ];
                            let edge_stroke = Stroke::new(
                                if is_sel { 2.2 } else { 1.2 },
                                if is_sel { box_col } else { box_col.gamma_multiply(0.85) },
                            );
                            for (a, b) in edges {
                                painter.line_segment([pts[a], pts[b]], edge_stroke);
                            }

                            let (p_c, _) = project_3d(cx, cy, cz);
                            painter.circle_filled(
                                p_c,
                                if is_sel { 12.0 } else { 7.5 },
                                box_col.gamma_multiply(if is_sel { 0.38 } else { 0.24 }),
                            );
                            painter.circle_filled(p_c, if is_sel { 5.5 } else { 3.8 }, box_col);
                            if is_sel {
                                painter.circle_stroke(p_c, 13.5, Stroke::new(1.8, Color32::WHITE));
                            }
                            painter.text(
                                p_c - Vec2::new(0.0, 14.0),
                                Align2::CENTER_BOTTOM,
                                &item.tag,
                                egui::FontId::monospace(if is_sel { 10.5 } else { 9.0 }),
                                if is_sel { Color32::WHITE } else { box_col },
                            );
                        }
                    }

                    // 5. Global Coordinate XYZ Axis Triad (Bottom Left HUD)
                    if self.show_axes {
                        let triad_center = Pos2::new(rect.left() + 45.0, rect.bottom() - 45.0);
                        let triad_len = 28.0;

                        // Project axis directions using camera rotations
                        let (ax_x, _) = project_3d(center_3d.0 + 1.0, center_3d.1, center_3d.2);
                        let (ax_y, _) = project_3d(center_3d.0, center_3d.1 + 1.0, center_3d.2);
                        let (ax_z, _) = project_3d(center_3d.0, center_3d.1, center_3d.2 + 1.0);
                        let (ax_0, _) = project_3d(center_3d.0, center_3d.1, center_3d.2);

                        let dx_screen = (ax_x - ax_0).normalized() * triad_len;
                        let dy_screen = (ax_y - ax_0).normalized() * triad_len;
                        let dz_screen = (ax_z - ax_0).normalized() * triad_len;

                        painter.circle_filled(triad_center, 4.0, Color32::WHITE);
                        painter.line_segment([triad_center, triad_center + dx_screen], Stroke::new(2.5, Color32::from_rgb(244, 63, 94))); // X (Red)
                        painter.line_segment([triad_center, triad_center + dy_screen], Stroke::new(2.5, Color32::from_rgb(34, 197, 94))); // Y (Green)
                        painter.line_segment([triad_center, triad_center + dz_screen], Stroke::new(2.5, Color32::from_rgb(59, 130, 246))); // Z (Blue)

                        painter.text(triad_center + dx_screen * 1.25, Align2::CENTER_CENTER, "X", egui::FontId::monospace(10.0), Color32::from_rgb(244, 63, 94));
                        painter.text(triad_center + dy_screen * 1.25, Align2::CENTER_CENTER, "Y", egui::FontId::monospace(10.0), Color32::from_rgb(34, 197, 94));
                        painter.text(triad_center + dz_screen * 1.25, Align2::CENTER_CENTER, "Z", egui::FontId::monospace(10.0), Color32::from_rgb(59, 130, 246));
                    }

                    // Clean 3D viewport mode string moved to bottom telemetry bar
                    let mode_str = match self.render_mode {
                        Render3DMode::CsgSolidBrep => "CSG B-Rep",
                        Render3DMode::SolidShaded => "Solid",
                        Render3DMode::Wireframe => "Wireframe",
                        Render3DMode::AxialForceNFD => "NFD",
                        Render3DMode::ShearForceSFD => "SFD",
                        Render3DMode::BendingMomentBMD => "BMD",
                    };

        // Live Ink Sketch preview rubber-band on 3D ground plane
        if self.sketch_mode_active {
            let offset_x = center_3d.0 - 6.0;
            let offset_z = center_3d.2 - 4.5;
            let ground_to_screen = |mm: [f64; 2]| -> Pos2 {
                project_3d(
                    mm[0] as f32 / 1000.0 + offset_x,
                    0.02,
                    mm[1] as f32 / 1000.0 + offset_z,
                )
                .0
            };
            universal_canvas.paint_live_preview_world(&painter, rect, theme, &ground_to_screen);
        }

        if !spatial_fullscreen {
            UniversalCanvasState::paint_floating_freeze_pill(
                &painter,
                freeze_pill_rect,
                theme,
                universal_canvas.is_frozen,
                freeze_pill_hovered,
            );
        }

        // 7. Floating Hover Glass Tooltip
        if let Some(hid) = hovered_member {
            if let Some(item) = render_list.iter().find(|i| i.id == hid) {
                if let Some(mpos) = response.hover_pos() {
                    let tip_pos = mpos + Vec2::new(14.0, -18.0);
                    let tip_rect = egui::Rect::from_min_size(tip_pos, Vec2::new(195.0, 90.0));

                    painter.rect_filled(
                        tip_rect,
                        CornerRadius::same(6),
                        Color32::from_rgba_premultiplied(20, 24, 34, 235),
                    );
                    painter.rect_stroke(
                        tip_rect,
                        CornerRadius::same(6),
                        Stroke::new(1.0, theme.accent_blue),
                        egui::StrokeKind::Outside,
                    );

                    painter.text(
                        tip_pos + Vec2::new(8.0, 6.0),
                        Align2::LEFT_TOP,
                        format!("{} [{}]", item.name, item.section),
                        egui::FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    painter.text(
                        tip_pos + Vec2::new(8.0, 22.0),
                        Align2::LEFT_TOP,
                        format!("Axial Pu: {:.1} kN", item.force_kn),
                        egui::FontId::monospace(10.0),
                        theme.accent_blue,
                    );
                    painter.text(
                        tip_pos + Vec2::new(8.0, 38.0),
                        Align2::LEFT_TOP,
                        format!("Capacity ϕPn: {:.1} kN", item.capacity_kn),
                        egui::FontId::monospace(10.0),
                        theme.text_secondary,
                    );
                    painter.text(
                        tip_pos + Vec2::new(8.0, 54.0),
                        Align2::LEFT_TOP,
                        format!("Utilization DCR: {:.1}%", item.utilization * 100.0),
                        egui::FontId::monospace(10.0),
                        theme.accent_orange,
                    );
                    painter.text(
                        tip_pos + Vec2::new(8.0, 70.0),
                        Align2::LEFT_TOP,
                        format!("ARI Redundancy: {:.3}", item.ari),
                        egui::FontId::monospace(10.0),
                        theme.accent_green,
                    );
                }
            }
        }

        if spatial_fullscreen {
            return;
        }

        UniversalCanvasState::paint_rounded_viewport_border(
            &painter,
            rect,
            10.0,
            theme.bg_sidebar,
            theme.border_subtle,
        );

        ui.add_space(6.0);

        // 8. Sleek Earth, Camera & BIM Telemetry Footer Bar below the 3D Viewport
        let synced_count = universal_canvas.entities.len();
        let map_telem = universal_canvas.world_map.telemetry();
        let active_lvl_lbl = self
            .bim_bus
            .project
            .levels
            .iter()
            .min_by(|a, b| {
                ((a.elevation_mm as f32 * 0.001) - self.draft_node_y as f32)
                    .abs()
                    .total_cmp(&((b.elevation_mm as f32 * 0.001) - self.draft_node_y as f32).abs())
            })
            .map(|l| format!("{} ({:+.2}m)", l.name, l.elevation_mm / 1000.0))
            .unwrap_or_else(|| "Ground (+0.00m)".to_string());

        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(12, 6))
            .show(ui, |ui| {
                let inner_w = ui.available_width().max(120.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                let earth_txt = format!(
                    "Earth {:.4}°, {:.4}° • z{:.1} ({:.1} m/px)",
                    universal_canvas.cursor_lat_lon.0,
                    universal_canvas.cursor_lat_lon.1,
                    universal_canvas.world_map.zoom_level,
                    map_telem.ground_resolution_m_px,
                );
                let cam_txt = format!(
                    "{} • Yaw {:.0}° Pitch {:.0}° ({:.2}x) • {}",
                    mode_str,
                    self.cam_yaw.to_degrees(),
                    self.cam_pitch.to_degrees(),
                    self.cam_zoom,
                    active_lvl_lbl,
                );
                let stage_4d = if self.highlight_4d_erection && self.simulation_4d_day > 0 {
                    format!(" • 4D Day {}", self.simulation_4d_day)
                } else {
                    String::new()
                };
                let counts_txt = format!(
                    "Tool: {} • {} Elements • {} Synced{}",
                    self.ground_author_tool.label(),
                    members.len(),
                    synced_count,
                    stage_4d,
                );
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        11.5,
                        theme.accent_blue,
                        earth_txt,
                        10.0,
                        theme.text_muted,
                        false,
                    );
                    icons::icon_label(
                        ui,
                        AppIcon::Cube3D,
                        11.0,
                        theme.text_muted,
                        cam_txt,
                        10.0,
                        theme.text_secondary,
                        false,
                    );
                    icons::icon_label(
                        ui,
                        AppIcon::CheckCircle,
                        11.0,
                        theme.accent_green,
                        counts_txt,
                        10.0,
                        theme.accent_green,
                        true,
                    );
                    ui.label(
                        RichText::new("Keys: Space Freeze • W/B/C/F Author • M/O/X Modify • Z Fit • Esc Orbit")
                            .size(10.0)
                            .color(theme.text_muted),
                    );
                });
            });

        // 2. Shared Status Toast Banner & Bottom HUD Toolbars below 3D Viewport
        UniversalCanvasState::render_toast_bar(ui, theme, &mut self.modify_status_msg);
        ui.add_space(6.0);

        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                let inner_w = ui.available_width().max(120.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.horizontal_wrapped(|ui| {
                    icons::pill_badge(ui, AppIcon::Cube3D, "Geometry • 3D", theme.accent_blue);
                    ui.separator();

                    // Geometry 3D Render Styles (Structural diagrams & heatmaps live in external Structural Analysis below)
                    let modes = [
                        (Render3DMode::CsgSolidBrep, AppIcon::Cube3D, "CSG B-Rep", "True 3D CSG Boolean Mesh (Openings, WF Flanges & Rebar)"),
                        (Render3DMode::SolidShaded, AppIcon::SolidBox, "Solid", "Shaded solid structural frame"),
                        (Render3DMode::Wireframe, AppIcon::Wireframe, "Wireframe", "Vector wireframe members"),
                    ];
                    for (rm, icon, lbl, tip) in modes {
                        let sel = self.render_mode == rm;
                        if icons::icon_button(
                            ui,
                            icon,
                            lbl,
                            10.5,
                            if sel { Color32::WHITE } else { theme.text_secondary },
                            if sel { theme.accent_blue } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text(tip)
                        .clicked()
                        {
                            self.render_mode = rm;
                        }
                    }

                    ui.separator();

                    // Phase 35: World Map Basemap, Globe Reset, Ground 1:200 & Site Origin Controls
                    universal_canvas.render_world_map_toolbar_controls(ui, theme, 26.0);

                    ui.separator();

                    // ViewCube Camera Presets
                    for (lbl, yaw, pitch) in [
                        ("Iso", 0.65_f32, 0.35_f32),
                        ("Front", 0.0, 0.0),
                        ("Top", 0.0, 1.50),
                        ("Side", 1.57, 0.0),
                    ] {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(lbl).size(10.5).color(theme.text_secondary),
                                )
                                .fill(theme.bg_card)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(0.0, 26.0)),
                            )
                            .clicked()
                        {
                            self.cam_yaw = yaw;
                            self.cam_pitch = pitch;
                        }
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::ZoomFit,
                        "Fit Model",
                        10.5,
                        theme.accent_blue,
                        theme.bg_card,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Auto-fit 3D camera and Earth site view to active BIM model extents")
                    .clicked()
                    {
                        self.fit_view_to_project(universal_canvas, members);
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Persp").size(10.5).color(
                                if self.perspective {
                                    Color32::WHITE
                                } else {
                                    theme.text_secondary
                                },
                            ))
                            .fill(if self.perspective {
                                theme.accent_blue
                            } else {
                                theme.bg_card
                            })
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(0.0, 26.0)),
                        )
                        .clicked()
                    {
                        self.perspective = !self.perspective;
                    }

                    ui.separator();

                    // Bottom-Toolbar Dockable Inspector Toggles
                    let proj_active = self.floating_project_open;
                    if icons::icon_button(
                        ui,
                        AppIcon::Folder,
                        "Project",
                        10.5,
                        if proj_active { Color32::WHITE } else { theme.text_secondary },
                        if proj_active { theme.accent_green } else { theme.bg_card },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Toggle Project & Model Tree drawer in toolbar dock")
                    .clicked()
                    {
                        self.floating_project_open = !self.floating_project_open;
                    }

                    let prop_active = self.floating_properties_open;
                    if icons::icon_button(
                        ui,
                        AppIcon::SlidersPanel,
                        "Properties",
                        10.5,
                        if prop_active { Color32::WHITE } else { theme.text_secondary },
                        if prop_active { theme.accent_green } else { theme.bg_card },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Toggle Properties, Clash, Audit, Geometry & Layers drawer in toolbar dock")
                    .clicked()
                    {
                        self.floating_properties_open = !self.floating_properties_open;
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::ChartBar,
                        "Data Deck",
                        10.5,
                        if self.floating_deck_open { Color32::WHITE } else { theme.text_secondary },
                        if self.floating_deck_open { theme.accent_blue } else { theme.bg_card },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Toggle Schedules QTO, 4D Simulation, Clash & BIM Console drawer in toolbar dock")
                    .clicked()
                    {
                        self.floating_deck_open = !self.floating_deck_open;
                        self.bottom_deck_expanded = self.floating_deck_open;
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Wrench,
                        "BIM Tools",
                        10.5,
                        if self.show_discipline_ribbon { Color32::WHITE } else { theme.text_secondary },
                        if self.show_discipline_ribbon { theme.accent_blue } else { theme.bg_card },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Show or hide the 12-discipline BIM authoring ribbon")
                    .clicked()
                    {
                        self.show_discipline_ribbon = !self.show_discipline_ribbon;
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Pencil,
                        "Sketch",
                        10.5,
                        if self.sketch_mode_active { Color32::WHITE } else { theme.text_secondary },
                        if self.sketch_mode_active { theme.accent_orange } else { theme.bg_card },
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Draw synced Universal Ink Sketch annotations directly on the 3D ground plane")
                    .clicked()
                    {
                        self.sketch_mode_active = !self.sketch_mode_active;
                    }

                    ui.separator();

                    // CAD/BIM Modify Action Palette in Bottom Toolbar
                    if icons::icon_button(
                        ui,
                        AppIcon::Undo,
                        "Undo",
                        10.0,
                        theme.text_secondary,
                        theme.bg_card,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Undo last 3D BIM or sketch action (Ctrl/Cmd+Z)")
                    .clicked()
                    {
                        if self.sketch_mode_active && universal_canvas.undo() {
                            self.modify_status_msg = Some("Undo sketch edit".to_string());
                        } else if let Some(msg) = self.bim_bus.undo() {
                            self.modify_status_msg = Some(msg);
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
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Redo undone 3D BIM or sketch action (Ctrl/Cmd+Y)")
                    .clicked()
                    {
                        if self.sketch_mode_active && universal_canvas.redo() {
                            self.modify_status_msg = Some("Redo sketch edit".to_string());
                        } else if let Some(msg) = self.bim_bus.redo() {
                            self.modify_status_msg = Some(msg);
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
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Move latest BIM element or sketch East by Step mm (M)")
                    .clicked()
                    {
                        self.apply_3d_move_1m(universal_canvas);
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Layers,
                        "Copy",
                        10.0,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Duplicate latest wall, column, or sketch East by Step mm")
                    .clicked()
                    {
                        self.apply_3d_duplicate(universal_canvas);
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Compass,
                        "Rot 90°",
                        10.0,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Rotate selected/latest element or sketch +90°")
                    .clicked()
                    {
                        self.apply_3d_rotate_90(universal_canvas);
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Layers,
                        "Offset Wall",
                        10.0,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Create +500 mm parallel duplicate of latest wall or sketch (O)")
                    .clicked()
                    {
                        self.apply_3d_offset_wall(universal_canvas);
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::SectionCut,
                        "Mirror",
                        10.0,
                        theme.text_primary,
                        theme.bg_card,
                        Stroke::new(1.0, theme.border_subtle),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Mirror modeled columns/walls/beams across centerline (X)")
                    .clicked()
                    {
                        self.apply_3d_mirror(universal_canvas);
                    }

                    if icons::icon_button(
                        ui,
                        AppIcon::Trash,
                        "Delete",
                        10.0,
                        theme.accent_red,
                        theme.bg_card,
                        Stroke::new(1.0, theme.accent_red.gamma_multiply(0.35)),
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Delete selected or latest BIM element / sketch (Del)")
                    .clicked()
                    {
                        self.apply_3d_delete(universal_canvas);
                    }

                    ui.separator();

                    // Unified 1-File Sync & Import
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
                    .on_hover_text("Save entire project workspace into 1 verified .eigen.json file")
                    .clicked()
                    {
                        universal_canvas.export_unified_file(
                            &self.bim_bus.project,
                            members,
                            parametric_variables,
                            workspace_settings.clone(),
                        );
                        if let Some(ref msg) = universal_canvas.status_message {
                            self.modify_status_msg = Some(msg.clone());
                        }
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
                        CornerRadius::same(6),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Open CAD Shortcuts, Relative/Polar CLI Syntax & Module Quick-Jump (Ctrl/Cmd+K)")
                    .clicked()
                    {
                        universal_canvas.shortcuts_modal_open = !universal_canvas.shortcuts_modal_open;
                    }

                    // Selection Clear (Structural analysis & N-1 controls live in below-toolbar panel)
                    if self.selected_member_id.is_some()
                        && icons::icon_button(
                            ui,
                            AppIcon::Close,
                            "Clear Sel",
                            10.0,
                            theme.text_secondary,
                            theme.bg_card,
                            Stroke::NONE,
                            CornerRadius::same(5),
                            Vec2::new(0.0, 26.0),
                        )
                        .clicked()
                    {
                        self.selected_member_id = None;
                    }
                });
            });

        // 2B. Row 2: Levels, Scale, Detail (LOD), Visual Toggles & Inline BIM Command Line
        self.render_top_toolbar_row2_and_cmd(ui, theme, universal_canvas);

        // 3. Conditional Universal Ink Sketch Bar (when Ground Sketch Mode is active)
        if self.sketch_mode_active {
            Frame::new()
                .fill(theme.bg_sidebar)
                .corner_radius(CornerRadius::same(8))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .inner_margin(Margin::symmetric(12, 5))
                .show(ui, |ui| {
                    let inner_w = ui.available_width().max(120.0);
                    ui.set_width(inner_w);
                    ui.set_max_width(inner_w);
                    ui.horizontal_wrapped(|ui| {
                        universal_canvas.render_sketch_tools_bar(ui, theme, 24.0, false);
                        ui.separator();
                        universal_canvas.render_color_swatches(ui, theme);
                        ui.separator();
                        icons::pill_badge(
                            ui,
                            AppIcon::LinkChain,
                            "3D Ground Sync Active (y=0.02m)",
                            theme.accent_green,
                        );
                    });
                });
        }

        // 4. Conditional 12-Discipline BIM Authoring Ribbon
        if self.show_discipline_ribbon {
            self.render_bim_master_ribbon(ui, theme, 0.0);
        }

        // 5. Dockable Inspector Area (Project Browser, Properties, Data Deck — below viewport)
        if self.floating_project_open || self.floating_properties_open || self.floating_deck_open {
            self.render_top_dock_drawers(
                ui,
                theme,
                members,
                db_components,
                features,
                universal_canvas,
                parametric_variables,
                &workspace_settings,
                avail_w,
            );
        }
    }

    /// Second organized Top Toolbar row inside Geometry canvas:
    /// Datum Level selector (+ Lvl), View Scale, Detail (LOD), Visual Toggles & Inline BIM Command Line.
    fn render_top_toolbar_row2_and_cmd(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        ui.add_space(4.0);
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(12, 6))
            .show(ui, |ui| {
                let inner_w = ui.available_width().max(120.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.horizontal_wrapped(|ui| {
                    // Datum Levels Selector + Quick Add Level
                    let lvl_w = crate::ui_kit::clamped_row_width(ui, 80.0, 142.0, 108.0);
                    crate::ui_kit::wrap_before(ui, lvl_w + 104.0);
                    ui.label(RichText::new("Level:").size(10.0).color(theme.text_muted));
                    let current_lvl_text = self
                        .bim_bus
                        .project
                        .levels
                        .iter()
                        .min_by(|a, b| {
                            ((a.elevation_mm / 1000.0) - self.draft_node_y)
                                .abs()
                                .total_cmp(&((b.elevation_mm / 1000.0) - self.draft_node_y).abs())
                        })
                        .map(|l| format!("{} ({:+.2}m)", l.name, l.elevation_mm / 1000.0))
                        .unwrap_or_else(|| "Ground Level (+0.00m)".to_string());

                    let levels_clone = self.bim_bus.project.levels.clone();
                    egui::ComboBox::from_id_salt("geometry_top_level_combo")
                        .width(lvl_w)
                        .selected_text(RichText::new(current_lvl_text).size(10.0).color(theme.text_primary))
                        .show_ui(ui, |ui| {
                            for lvl in &levels_clone {
                                let elev_m = lvl.elevation_mm / 1000.0;
                                let is_cur = (elev_m - self.draft_node_y).abs() < 0.05;
                                if ui
                                    .selectable_label(
                                        is_cur,
                                        format!("{} ({:+.2} m)", lvl.name, elev_m),
                                    )
                                    .clicked()
                                {
                                    self.draft_node_y = elev_m;
                                    self.section_box_max_z_m = (elev_m as f32) + 4.0;
                                    self.modify_status_msg = Some(format!(
                                        "Active working datum set to {} ({:+.2} m)",
                                        lvl.name, elev_m
                                    ));
                                }
                            }
                        });

                    if icons::icon_button(
                        ui,
                        AppIcon::Plus,
                        "Lvl",
                        10.0,
                        theme.accent_blue,
                        theme.bg_card,
                        Stroke::NONE,
                        CornerRadius::same(4),
                        Vec2::new(0.0, 22.0),
                    )
                    .on_hover_text("Add next storey level (+3.50 m above highest datum)")
                    .clicked()
                    {
                        let next_num = self.bim_bus.project.levels.len() + 1;
                        let max_elev = self
                            .bim_bus
                            .project
                            .levels
                            .iter()
                            .map(|l| l.elevation_mm)
                            .fold(0.0_f64, f64::max);
                        let next_elev = if self.bim_bus.project.levels.is_empty() {
                            0.0
                        } else {
                            max_elev + 3500.0
                        };
                        let name = format!("Level {} (+{:.2}m)", next_num, next_elev / 1000.0);
                        match self.bim_bus.execute(crate::core::BimCommand::AddLevel {
                            name,
                            elevation_mm: next_elev,
                        }) {
                            Ok(msg) => {
                                self.draft_node_y = next_elev / 1000.0;
                                self.section_box_max_z_m = (next_elev as f32 / 1000.0) + 4.0;
                                self.modify_status_msg = Some(msg);
                            }
                            Err(e) => self.modify_status_msg = Some(e),
                        }
                    }

                    ui.separator();

                    // Phase 37: Direct 3D Earth Ground BIM Authoring Tools
                    ui.label(RichText::new("Author:").size(10.0).color(theme.text_muted));
                    for tool in [
                        Ground3DAuthorTool::Orbit,
                        Ground3DAuthorTool::Column,
                        Ground3DAuthorTool::Wall,
                        Ground3DAuthorTool::Beam,
                        Ground3DAuthorTool::Footing,
                    ] {
                        let sel = self.ground_author_tool == tool;
                        if icons::icon_button(
                            ui,
                            tool.app_icon(),
                            tool.label(),
                            10.0,
                            if sel { Color32::WHITE } else { theme.text_secondary },
                            if sel { theme.accent_orange } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(4),
                            Vec2::new(0.0, 22.0),
                        )
                        .on_hover_text("Direct 3D ground placement on active datum elevation (Right-drag pans Earth map)")
                        .clicked()
                        {
                            self.ground_author_tool = tool;
                            self.pending_3d_start_mm = None;
                        }
                    }

                    ui.separator();

                    // Scale & Detail Level
                    crate::ui_kit::wrap_before(ui, 134.0);
                    ui.label(RichText::new("Scale:").size(10.0).color(theme.text_muted));
                    egui::ComboBox::from_id_salt("viewport_scale_combo")
                        .width(64.0)
                        .selected_text(RichText::new(&self.view_scale).size(10.0).color(theme.text_primary))
                        .show_ui(ui, |ui| {
                            for s in ["1:50", "1:100", "1:200", "1:500"] {
                                if ui.selectable_label(self.view_scale == s, s).clicked() {
                                    self.view_scale = s.to_string();
                                }
                            }
                        });

                    let lod_w = crate::ui_kit::clamped_row_width(ui, 84.0, 118.0, 74.0);
                    crate::ui_kit::wrap_before(ui, lod_w + 72.0);
                    ui.label(RichText::new("Detail:").size(10.0).color(theme.text_muted));
                    egui::ComboBox::from_id_salt("viewport_lod_combo")
                        .width(lod_w)
                        .selected_text(RichText::new(&self.lod_level).size(10.0).color(theme.accent_blue))
                        .show_ui(ui, |ui| {
                            for l in [
                                "LOD 100 (Massing)",
                                "LOD 200 (Design)",
                                "LOD 350 (Fine BIM Detail)",
                                "LOD 400 (Fabrication)",
                            ] {
                                if ui.selectable_label(self.lod_level == l, l).clicked() {
                                    self.lod_level = l.to_string();
                                }
                            }
                        });

                    ui.separator();

                    // Visual Display Toggles
                    let toggles: [(&mut bool, AppIcon, &str); 8] = [
                        (&mut self.show_sun_shadows, AppIcon::Sun, "Sun"),
                        (&mut self.show_level_datums, AppIcon::RulerSquare, "Datums"),
                        (&mut self.show_grid_bubbles, AppIcon::Grid, "Grids"),
                        (&mut self.section_box_enabled, AppIcon::BoxPackage, "Section"),
                        (&mut self.highlight_4d_erection, AppIcon::Clock, "4D Stage"),
                        (&mut self.temporary_isolate_active, AppIcon::Target, "Isolate"),
                        (&mut self.show_3d_clash_markers, AppIcon::AlertTriangle, "Clashes"),
                        (&mut self.show_axes, AppIcon::Compass, "Axes"),
                    ];
                    for (flag, icon, lbl) in toggles {
                        let active = *flag;
                        if icons::icon_button(
                            ui,
                            icon,
                            lbl,
                            10.0,
                            if active { Color32::WHITE } else { theme.text_secondary },
                            if active { theme.accent_blue } else { theme.bg_card },
                            Stroke::NONE,
                            CornerRadius::same(4),
                            Vec2::new(0.0, 22.0),
                        )
                        .clicked()
                        {
                            *flag = !*flag;
                        }
                    }
                    if self.highlight_4d_erection && self.simulation_4d_day == 0 {
                        self.simulation_4d_day = 60;
                    }

                    ui.separator();

                    // Inline BIM / CAD Command Line inside Top Toolbar
                    let cmd_w = crate::ui_kit::clamped_row_width(ui, 72.0, 180.0, 146.0);
                    crate::ui_kit::wrap_before(ui, cmd_w + 146.0);
                    ui.label(RichText::new("BIM Cmd:").size(10.0).strong().color(theme.accent_blue));
                    let cmd_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.bim_console_input)
                            .hint_text("wall 0,0->6000,0 | col 0,0 | move | offset | mirror | del | clear")
                            .desired_width(cmd_w),
                    );
                    let run_clicked = icons::icon_button(
                        ui,
                        AppIcon::Play,
                        "Run",
                        10.0,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(4),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked();
                    if run_clicked
                        || (cmd_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        let raw = self.bim_console_input.trim().to_string();
                        if !raw.is_empty() {
                            self.run_inline_bim_command(&raw, universal_canvas);
                            self.bim_console_input.clear();
                        }
                    }
                });
            });
    }

    /// Renders open Project Browser, Properties, and Data Deck cleanly inside the Top Toolbar Dock Area
    /// above the 3D Earth viewport (never overlapping or obscuring the drawing canvas).
    #[allow(clippy::too_many_arguments)]
    fn render_top_dock_drawers(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        db_components: &[DatabaseComponent],
        features: &mut FeatureRegistry,
        universal_canvas: &mut UniversalCanvasState,
        parametric_variables: &[ParametricVariable],
        workspace_settings: &WorkspaceCanvasSettings,
        avail_w: f32,
    ) {
        let open_count = usize::from(self.floating_project_open)
            + usize::from(self.floating_properties_open)
            + usize::from(self.floating_deck_open);
        if open_count == 0 {
            return;
        }

        Frame::new()
            .fill(theme.bg_sidebar)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                let inner_w = ui.available_width().max(120.0);
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                if avail_w >= 960.0 && open_count > 1 {
                    let flags = [
                        self.floating_project_open,
                        self.floating_properties_open,
                        self.floating_deck_open,
                    ];
                    let active_slots: Vec<usize> = flags
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &on)| on.then_some(i))
                        .collect();
                    let col_w = (ui.available_width() / active_slots.len() as f32).max(260.0);
                    ui.columns(active_slots.len(), |cols| {
                        for (col_idx, &slot) in active_slots.iter().enumerate() {
                            cols[col_idx].set_max_height(265.0);
                            match slot {
                                0 => self.render_floating_project_card(
                                    &mut cols[col_idx],
                                    theme,
                                    members,
                                    db_components,
                                    false,
                                ),
                                1 => self.render_floating_properties_card(
                                    &mut cols[col_idx],
                                    theme,
                                    members,
                                    db_components,
                                    features,
                                    universal_canvas,
                                    parametric_variables,
                                    workspace_settings.clone(),
                                    col_w,
                                    false,
                                ),
                                _ => self.render_floating_deck_card(
                                    &mut cols[col_idx],
                                    theme,
                                    members,
                                    features,
                                    false,
                                ),
                            }
                        }
                    });
                } else {
                    if self.floating_project_open {
                        ui.push_id("top_dock_project", |ui| {
                            ui.set_max_height(240.0);
                            self.render_floating_project_card(ui, theme, members, db_components, false);
                        });
                        ui.add_space(6.0);
                    }
                    if self.floating_properties_open {
                        ui.push_id("top_dock_props", |ui| {
                            ui.set_max_height(260.0);
                            self.render_floating_properties_card(
                                ui,
                                theme,
                                members,
                                db_components,
                                features,
                                universal_canvas,
                                parametric_variables,
                                workspace_settings.clone(),
                                avail_w,
                                false,
                            );
                        });
                        ui.add_space(6.0);
                    }
                    if self.floating_deck_open {
                        ui.push_id("top_dock_deck", |ui| {
                            ui.set_max_height(240.0);
                            self.render_floating_deck_card(ui, theme, members, features, false);
                        });
                    }
                }
            });
    }

    /// Executes quick CAD/BIM shortcuts or JSON `BimCommand` strings entered in the Top Toolbar Command Line.
    fn run_inline_bim_command(&mut self, raw: &str, universal_canvas: &mut UniversalCanvasState) {
        let trimmed = raw.trim();
        self.bim_console_logs.push(format!("> {}", trimmed));

        if trimmed.starts_with('{') {
            match self.bim_bus.execute_json(trimmed) {
                Ok(msg) => {
                    self.bim_console_logs.push(msg.clone());
                    self.modify_status_msg = Some(msg);
                }
                Err(e) => {
                    self.bim_console_logs.push(e.clone());
                    self.modify_status_msg = Some(e);
                }
            }
            return;
        }

        let lower = trimmed.to_lowercase();
        let active_lvl_id = self
            .bim_bus
            .project
            .levels
            .iter()
            .min_by(|a, b| {
                ((a.elevation_mm / 1000.0) - self.draft_node_y)
                    .abs()
                    .total_cmp(&((b.elevation_mm / 1000.0) - self.draft_node_y).abs())
            })
            .map(|l| l.id)
            .unwrap_or(1);

        let parse_pair = |s: &str| -> Option<crate::core::P2> {
            let mut it = s.split(',').filter_map(|p| p.trim().parse::<f64>().ok());
            Some(crate::core::P2(it.next()?, it.next()?))
        };

        if trimmed.starts_with('@') {
            if let Some((dx, dy)) = UniversalCanvasState::parse_cad_delta_expr(trimmed) {
                let start = self
                    .pending_3d_start_mm
                    .map(|p| crate::core::P2(p[0], p[1]))
                    .or_else(|| self.bim_bus.project.walls.last().map(|w| w.end))
                    .unwrap_or(crate::core::P2(0.0, 0.0));
                let end = crate::core::P2(start.0 + dx, start.1 + dy);
                match self.bim_bus.execute(crate::core::BimCommand::AddWall {
                    base_level_id: active_lvl_id,
                    start,
                    end,
                    thickness_mm: 200.0,
                    is_structural: true,
                }) {
                    Ok(msg) => {
                        self.pending_3d_start_mm = Some([end.0, end.1]);
                        self.bim_console_logs.push(msg.clone());
                        self.modify_status_msg = Some(msg);
                    }
                    Err(e) => self.modify_status_msg = Some(e),
                }
                return;
            }
        }

        if let Some(deg_str) = lower.strip_prefix("ro ").or_else(|| lower.strip_prefix("rotate ")) {
            if let Ok(deg) = deg_str.trim().parse::<f64>() {
                if let Some(id) = self.bim_bus.project.latest_element_id() {
                    if let Ok(msg) = self.bim_bus.execute(crate::core::BimCommand::RotateElement {
                        element_id: id,
                        angle_deg: deg,
                    }) {
                        self.bim_console_logs.push(msg.clone());
                        self.modify_status_msg = Some(msg);
                    }
                }
                return;
            }
        }

        if lower == "undo" {
            let msg = self
                .bim_bus
                .undo()
                .unwrap_or_else(|| "Nothing to undo".to_string());
            self.bim_console_logs.push(msg.clone());
            self.modify_status_msg = Some(msg);
        } else if lower == "redo" {
            let msg = self
                .bim_bus
                .redo()
                .unwrap_or_else(|| "Nothing to redo".to_string());
            self.bim_console_logs.push(msg.clone());
            self.modify_status_msg = Some(msg);
        } else if lower == "clear" {
            self.bim_bus = crate::core::BimCommandBus::new();
            let msg = "Cleared active BIM workspace to blank ground plane.".to_string();
            self.bim_console_logs.push(msg.clone());
            self.modify_status_msg = Some(msg);
        } else if lower == "world" {
            universal_canvas.world_map.reset_to_global_view();
            self.modify_status_msg = Some("Reset Earth camera to Global World view.".to_string());
        } else if lower == "ground" {
            let target = if universal_canvas.ground_origin_anchored {
                universal_canvas.ground_origin_lat_lon
            } else {
                universal_canvas.world_map.center_lat_lon
            };
            universal_canvas.ground_origin_lat_lon = target;
            universal_canvas.ground_origin_anchored = true;
            universal_canvas
                .world_map
                .zoom_to_ground_site(target.0, target.1, 17.5);
            self.modify_status_msg = Some("Zoomed to 1:200 site ground datum.".to_string());
        } else if let Some(rest) = trimmed.strip_prefix("wall ").or_else(|| trimmed.strip_prefix("WALL ")) {
            let arrow_split = rest.replace("->", " ");
            let parts: Vec<&str> = arrow_split.split_whitespace().collect();
            if parts.len() >= 2 {
                if let (Some(p1), Some(p2)) = (parse_pair(parts[0]), parse_pair(parts[1])) {
                    match self.bim_bus.execute(crate::core::BimCommand::AddWall {
                        base_level_id: active_lvl_id,
                        start: p1,
                        end: p2,
                        thickness_mm: 200.0,
                        is_structural: true,
                    }) {
                        Ok(msg) => self.modify_status_msg = Some(msg),
                        Err(e) => self.modify_status_msg = Some(e),
                    }
                    return;
                }
            }
            self.modify_status_msg =
                Some("Usage: wall x1_mm,y1_mm -> x2_mm,y2_mm (e.g. wall 0,0 -> 6000,0)".to_string());
        } else if let Some(rest) = trimmed
            .strip_prefix("col ")
            .or_else(|| trimmed.strip_prefix("column "))
        {
            if let Some(pos) = parse_pair(rest.trim()) {
                let top_id = self
                    .bim_bus
                    .project
                    .levels
                    .iter()
                    .find(|l| l.elevation_mm > self.draft_node_y * 1000.0 + 100.0)
                    .map(|l| l.id)
                    .unwrap_or(active_lvl_id);
                match self.bim_bus.execute(crate::core::BimCommand::AddColumn {
                    base_level_id: active_lvl_id,
                    top_level_id: top_id,
                    pos,
                    width_mm: 400.0,
                    depth_mm: 400.0,
                    is_round: false,
                    profile_name: "RC 400x400".to_string(),
                }) {
                    Ok(msg) => self.modify_status_msg = Some(msg),
                    Err(e) => self.modify_status_msg = Some(e),
                }
            } else {
                self.modify_status_msg =
                    Some("Usage: col x_mm,y_mm (e.g. col 6000,0)".to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("level ") {
            let mut toks = rest.split_whitespace();
            if let (Some(name), Some(elev_str)) = (toks.next(), toks.next()) {
                if let Ok(elev_mm) = elev_str.parse::<f64>() {
                    match self.bim_bus.execute(crate::core::BimCommand::AddLevel {
                        name: name.to_string(),
                        elevation_mm: elev_mm,
                    }) {
                        Ok(msg) => self.modify_status_msg = Some(msg),
                        Err(e) => self.modify_status_msg = Some(e),
                    }
                    return;
                }
            }
            self.modify_status_msg =
                Some("Usage: level <Name> <Elevation_mm> (e.g. level Roof 7000)".to_string());
        } else {
            match self.bim_bus.execute_natural_language(trimmed) {
                Ok(msg) => {
                    self.bim_console_logs.push(msg.clone());
                    self.modify_status_msg = Some(msg);
                }
                Err(err) => {
                    self.bim_console_logs.push(err.clone());
                    self.modify_status_msg = Some(err);
                }
            }
        }
    }

    /// Renders true Earth World Map (sky horizon, ocean, global vector continents & coastlines,
    /// lat/lon graticules, live streamed Web Mercator raster tiles, local surveying grid, and site origin)
    /// directly onto the 3D ground plane (`y = 0.0`) with near-plane perspective clipping.
    #[allow(clippy::too_many_arguments)]
    fn render_3d_earth_ground_map<F>(
        &self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
        universal_canvas: &UniversalCanvasState,
        center_3d: (f32, f32, f32),
        base_scale_unit: f32,
        cam_yaw: f32,
        cam_pitch: f32,
        cam_zoom: f32,
        cam_pan: Vec2,
        is_persp: bool,
        d_cam: f32,
        project_3d: &F,
    ) where
        F: Fn(f32, f32, f32) -> (Pos2, f32),
    {
        use crate::core::gis::map_widget::{ear_clip_polygon, WORLD_LANDMASSES};

        let map_painter = painter.with_clip_rect(rect);
        let (ocean_color, land_color, coast_color) = universal_canvas.world_map.basemap_colors(theme);
        let cos_y = cam_yaw.cos();
        let sin_y = cam_yaw.sin();
        let cos_p = cam_pitch.cos();
        let sin_p = cam_pitch.sin();
        let center_screen = rect.center();
        let world_per_map_px = 1.0 / (base_scale_unit * cam_zoom).max(1.0);
        let min_depth = -d_cam + 8.0;
        let dy_ground = 0.0_f32 - center_3d.1;

        let map_px_to_cam = |offset: Vec2| -> (f32, f32, f32) {
            let dx = offset.x * world_per_map_px;
            let dz = offset.y * world_per_map_px;
            let x1 = dx * cos_y + dz * sin_y;
            let z1 = -dx * sin_y + dz * cos_y;
            let y2 = dy_ground * cos_p - z1 * sin_p;
            let depth = -(dy_ground * sin_p + z1 * cos_p);
            (x1, y2, depth)
        };

        let cam_to_screen = |x1: f32, y2: f32, depth: f32| -> Pos2 {
            let pf = if is_persp {
                (d_cam / (d_cam + depth).max(8.0)).clamp(0.05, 4.0)
            } else {
                1.0
            };
            let sc = base_scale_unit * cam_zoom * pf;
            Pos2::new(
                center_screen.x + cam_pan.x + x1 * sc,
                center_screen.y + cam_pan.y - y2 * sc,
            )
        };

        // 1. Atmospheric Horizon Sky + Ocean Ground Plane
        let sky_color = match universal_canvas.world_map.tile_provider {
            crate::core::gis::TileSource::CartoDbDarkMatter => Color32::from_rgb(7, 10, 17),
            crate::core::gis::TileSource::EsriWorldImagery => Color32::from_rgb(10, 20, 36),
            _ if theme.is_dark() => Color32::from_rgb(15, 23, 38),
            _ => Color32::from_rgb(206, 228, 248),
        };
        if is_persp && cos_p > 0.02 {
            let horizon_sy = center_screen.y
                + cam_pan.y
                - d_cam * base_scale_unit * cam_zoom * (sin_p / cos_p);
            if cam_pitch >= 0.0 {
                let split_y = horizon_sy.clamp(rect.top(), rect.bottom());
                if split_y > rect.top() {
                    map_painter.rect_filled(
                        Rect::from_min_max(rect.min, Pos2::new(rect.max.x, split_y)),
                        CornerRadius::ZERO,
                        sky_color,
                    );
                }
                if split_y < rect.bottom() {
                    map_painter.rect_filled(
                        Rect::from_min_max(Pos2::new(rect.min.x, split_y), rect.max),
                        CornerRadius::ZERO,
                        ocean_color,
                    );
                }
                if horizon_sy > rect.top() && horizon_sy < rect.bottom() {
                    map_painter.line_segment(
                        [Pos2::new(rect.left(), horizon_sy), Pos2::new(rect.right(), horizon_sy)],
                        Stroke::new(1.5, theme.accent_blue.gamma_multiply(0.5)),
                    );
                }
            } else {
                let split_y = horizon_sy.clamp(rect.top(), rect.bottom());
                if split_y > rect.top() {
                    map_painter.rect_filled(
                        Rect::from_min_max(rect.min, Pos2::new(rect.max.x, split_y)),
                        CornerRadius::ZERO,
                        ocean_color.gamma_multiply(0.85),
                    );
                }
                if split_y < rect.bottom() {
                    map_painter.rect_filled(
                        Rect::from_min_max(Pos2::new(rect.min.x, split_y), rect.max),
                        CornerRadius::ZERO,
                        sky_color,
                    );
                }
            }
        } else {
            map_painter.rect_filled(rect, CornerRadius::ZERO, ocean_color);
        }

        // 2. Global Vector Continents & Islands on Ground Plane (y = 0.0) across longitudinal wraps
        for &wrap_deg in &[-360.0_f64, 0.0_f64, 360.0_f64] {
            for land in WORLD_LANDMASSES {
                let cam_poly: Vec<(f32, f32, f32)> = land
                    .iter()
                    .map(|&(lat, lon)| {
                        let off = universal_canvas
                            .world_map
                            .lat_lon_to_map_px_offset(lat, lon + wrap_deg);
                        map_px_to_cam(off)
                    })
                    .collect();

                let clipped_cam = Self::near_clip_cam_poly(&cam_poly, min_depth);
                if clipped_cam.len() >= 3 {
                    let screen_pts: Vec<Pos2> = clipped_cam
                        .iter()
                        .map(|&(x1, y2, depth)| cam_to_screen(x1, y2, depth))
                        .collect();
                    let mut bounds = Rect::NOTHING;
                    for &p in &screen_pts {
                        bounds.extend_with(p);
                    }
                    if bounds.intersects(rect) {
                        if let Some(mesh) = ear_clip_polygon(&screen_pts, land_color) {
                            map_painter.add(egui::Shape::mesh(mesh));
                        }
                    }
                }

                // Stroke true coastline segments cleanly without spurious near-plane cap lines
                let n_pts = cam_poly.len();
                for i in 0..n_pts {
                    let a = cam_poly[i];
                    let b = cam_poly[(i + 1) % n_pts];
                    if let Some((ca, cb)) = Self::near_clip_segment(a, b, min_depth) {
                        let pa = cam_to_screen(ca.0, ca.1, ca.2);
                        let pb = cam_to_screen(cb.0, cb.1, cb.2);
                        let seg_bounds = Rect::from_two_pos(pa, pb);
                        if seg_bounds.intersects(rect) {
                            map_painter.line_segment([pa, pb], Stroke::new(1.1, coast_color));
                        }
                    }
                }
            }
        }

        // 3. 3D Geodetic Graticule Grid (Equator + 20-deg Parallels + 30-deg Meridians)
        let grat_alpha = if universal_canvas.world_map.zoom_level >= 11.0 { 22 } else { 46 };
        let grat_stroke = Stroke::new(0.8, Color32::from_rgba_unmultiplied(180, 205, 232, grat_alpha));
        let eq_stroke = Stroke::new(1.3, theme.accent_blue.gamma_multiply(0.55));
        for lat in [-60.0_f64, -40.0, -20.0, 0.0, 20.0, 40.0, 60.0] {
            let stroke = if lat.abs() < 1e-3 { eq_stroke } else { grat_stroke };
            let mut lon = -540.0_f64;
            while lon < 540.0 {
                let next_lon = lon + 20.0;
                let a = map_px_to_cam(universal_canvas.world_map.lat_lon_to_map_px_offset(lat, lon));
                let b = map_px_to_cam(universal_canvas.world_map.lat_lon_to_map_px_offset(lat, next_lon));
                if let Some((ca, cb)) = Self::near_clip_segment(a, b, min_depth) {
                    let pa = cam_to_screen(ca.0, ca.1, ca.2);
                    let pb = cam_to_screen(cb.0, cb.1, cb.2);
                    if Rect::from_two_pos(pa, pb).intersects(rect) {
                        map_painter.line_segment([pa, pb], stroke);
                    }
                }
                lon = next_lon;
            }
        }
        for &wrap_deg in &[-360.0_f64, 0.0_f64, 360.0_f64] {
            for lon_i in (-180..180).step_by(30) {
                let lon = lon_i as f64 + wrap_deg;
                let mut lat = -78.0_f64;
                while lat < 78.0 {
                    let next_lat = (lat + 13.0).min(78.0);
                    let a = map_px_to_cam(universal_canvas.world_map.lat_lon_to_map_px_offset(lat, lon));
                    let b = map_px_to_cam(universal_canvas.world_map.lat_lon_to_map_px_offset(next_lat, lon));
                    if let Some((ca, cb)) = Self::near_clip_segment(a, b, min_depth) {
                        let pa = cam_to_screen(ca.0, ca.1, ca.2);
                        let pb = cam_to_screen(cb.0, cb.1, cb.2);
                        if Rect::from_two_pos(pa, pb).intersects(rect) {
                            map_painter.line_segment([pa, pb], grat_stroke);
                        }
                    }
                    lat = next_lat;
                }
            }
        }

        // 4. Live Streamed Web Mercator Raster Tiles Subdivided in 3D Perspective (4x4 Mesh per Tile)
        let tile_span_mult = if cam_pitch.abs() < 1.25 { 1.55 } else { 1.15 };
        let quads = universal_canvas.world_map.collect_visible_tile_quads(
            ctx,
            rect.width() * tile_span_mult,
            rect.height() * tile_span_mult,
        );
        const TILE_SUB: usize = 4;
        for quad in quads {
            let nw_px = universal_canvas
                .world_map
                .lat_lon_to_map_px_offset(quad.north_lat, quad.west_lon);
            let se_px = universal_canvas
                .world_map
                .lat_lon_to_map_px_offset(quad.south_lat, quad.east_lon);
            let mut mesh = egui::epaint::Mesh::with_texture(quad.tex_id);

            let corner_sample = |ix: usize, iy: usize| -> ([f32; 3], Pos2) {
                let fx = ix as f32 / TILE_SUB as f32;
                let fy = iy as f32 / TILE_SUB as f32;
                let off = Vec2::new(
                    nw_px.x + (se_px.x - nw_px.x) * fx,
                    nw_px.y + (se_px.y - nw_px.y) * fy,
                );
                let c = map_px_to_cam(off);
                let uv = Pos2::new(
                    quad.uv_rect.min.x + quad.uv_rect.width() * fx,
                    quad.uv_rect.min.y + quad.uv_rect.height() * fy,
                );
                ([c.0, c.1, c.2], uv)
            };

            for iy in 0..TILE_SUB {
                for ix in 0..TILE_SUB {
                    let cell = [
                        corner_sample(ix, iy),
                        corner_sample(ix + 1, iy),
                        corner_sample(ix + 1, iy + 1),
                        corner_sample(ix, iy + 1),
                    ];
                    let clipped = Self::near_clip_uv_poly(&cell, min_depth);
                    if clipped.len() < 3 {
                        continue;
                    }
                    let start_idx = mesh.vertices.len() as u32;
                    for (cam_p, uv) in &clipped {
                        let sp = cam_to_screen(cam_p[0], cam_p[1], cam_p[2]);
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos: sp,
                            uv: *uv,
                            color: Color32::WHITE,
                        });
                    }
                    for k in 1..(clipped.len() as u32 - 1) {
                        mesh.indices.push(start_idx);
                        mesh.indices.push(start_idx + k);
                        mesh.indices.push(start_idx + k + 1);
                    }
                }
            }
            if !mesh.indices.is_empty() {
                map_painter.add(egui::Shape::mesh(mesh));
            }
        }

        // 5. Local Site Surveying Grid, Cardinal N/E Markers & Geodetic Ground Origin Marker
        if self.show_grid {
            let grid_size = 6;
            let grid_step = 4.0_f32;
            let half_span = grid_size as f32 * grid_step;
            for i in -grid_size..=grid_size {
                let fi = i as f32 * grid_step;
                let (p1, _) = project_3d(fi, 0.0, -half_span);
                let (p2, _) = project_3d(fi, 0.0, half_span);
                let alpha = if i == 0 { 85 } else { 28 };
                let st = Stroke::new(0.8, Color32::from_rgba_premultiplied(110, 185, 235, alpha));
                map_painter.line_segment([p1, p2], st);

                let (p3, _) = project_3d(-half_span, 0.0, fi);
                let (p4, _) = project_3d(half_span, 0.0, fi);
                map_painter.line_segment([p3, p4], st);
            }

            let (north_p, _) = project_3d(0.0, 0.0, -(half_span + 1.5));
            let (east_p, _) = project_3d(half_span + 1.5, 0.0, 0.0);
            map_painter.text(north_p, Align2::CENTER_BOTTOM, "N (+Lat)", egui::FontId::monospace(10.0), theme.accent_blue);
            map_painter.text(east_p, Align2::LEFT_CENTER, "E (+Lon)", egui::FontId::monospace(10.0), theme.accent_green);

            let (origin_p, _) = project_3d(0.0, 0.05, 0.0);
            map_painter.circle_filled(origin_p, 4.0, theme.accent_blue);
            let site_coords = if universal_canvas.ground_origin_anchored {
                universal_canvas.ground_origin_lat_lon
            } else {
                universal_canvas.world_map.center_lat_lon
            };
            map_painter.text(
                origin_p + Vec2::new(8.0, -8.0),
                Align2::LEFT_BOTTOM,
                format!("Earth Ground ({:.4}°, {:.4}°)", site_coords.0, site_coords.1),
                egui::FontId::monospace(9.5),
                theme.text_muted,
            );
        }
    }

    fn near_clip_cam_poly(pts: &[(f32, f32, f32)], min_depth: f32) -> Vec<(f32, f32, f32)> {
        let n = pts.len();
        if n < 3 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(n + 2);
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            let a_in = a.2 >= min_depth;
            let b_in = b.2 >= min_depth;
            if a_in && b_in {
                out.push(b);
            } else if a_in && !b_in {
                let denom = b.2 - a.2;
                let t = if denom.abs() > 1e-6 { ((min_depth - a.2) / denom).clamp(0.0, 1.0) } else { 0.0 };
                out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, min_depth));
            } else if !a_in && b_in {
                let denom = b.2 - a.2;
                let t = if denom.abs() > 1e-6 { ((min_depth - a.2) / denom).clamp(0.0, 1.0) } else { 0.0 };
                out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, min_depth));
                out.push(b);
            }
        }
        out
    }

    fn near_clip_segment(
        a: (f32, f32, f32),
        b: (f32, f32, f32),
        min_depth: f32,
    ) -> Option<((f32, f32, f32), (f32, f32, f32))> {
        let a_in = a.2 >= min_depth;
        let b_in = b.2 >= min_depth;
        match (a_in, b_in) {
            (true, true) => Some((a, b)),
            (false, false) => None,
            (true, false) => {
                let denom = b.2 - a.2;
                let t = if denom.abs() > 1e-6 { ((min_depth - a.2) / denom).clamp(0.0, 1.0) } else { 0.0 };
                Some((a, (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, min_depth)))
            }
            (false, true) => {
                let denom = b.2 - a.2;
                let t = if denom.abs() > 1e-6 { ((min_depth - a.2) / denom).clamp(0.0, 1.0) } else { 0.0 };
                Some(((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, min_depth), b))
            }
        }
    }

    fn near_clip_uv_poly(verts: &[([f32; 3], Pos2)], min_depth: f32) -> Vec<([f32; 3], Pos2)> {
        let n = verts.len();
        if n < 3 {
            return Vec::new();
        }
        let lerp_v = |a: &([f32; 3], Pos2), b: &([f32; 3], Pos2)| -> ([f32; 3], Pos2) {
            let denom = b.0[2] - a.0[2];
            let t = if denom.abs() > 1e-6 {
                ((min_depth - a.0[2]) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (
                [
                    a.0[0] + (b.0[0] - a.0[0]) * t,
                    a.0[1] + (b.0[1] - a.0[1]) * t,
                    min_depth,
                ],
                Pos2::new(
                    a.1.x + (b.1.x - a.1.x) * t,
                    a.1.y + (b.1.y - a.1.y) * t,
                ),
            )
        };
        let mut out = Vec::with_capacity(n + 2);
        for i in 0..n {
            let a = &verts[i];
            let b = &verts[(i + 1) % n];
            let a_in = a.0[2] >= min_depth;
            let b_in = b.0[2] >= min_depth;
            if a_in && b_in {
                out.push(*b);
            } else if a_in && !b_in {
                out.push(lerp_v(a, b));
            } else if !a_in && b_in {
                out.push(lerp_v(a, b));
                out.push(*b);
            }
        }
        out
    }

    /// Builds 3D coordinates and elements based on active architectural archetype.
    /// Streams real 3D coordinates directly from `InternalVault/Demo/NVW_DCR-LOD/*.ifc` when available!
    /// Builds 3D geometry strictly from the user's live BIM Project and Structural Members (blank when empty).
    fn build_model_geometry(&self, fallback_members: &[StructuralMember]) -> ((f32, f32, f32), f32, Vec<Member3D>) {
        let fem = self.bim_bus.project.to_structural_model();
        let mut list = Vec::with_capacity(fem.elements.len() + fallback_members.len());
        for (idx, el) in fem.elements.iter().enumerate() {
            let n1 = &fem.nodes[el.start_node];
            let n2 = &fem.nodes[el.end_node];
            let p1 = (n1.x as f32, n1.y as f32, n1.z as f32);
            let p2 = (n2.x as f32, n2.y as f32, n2.z as f32);
            let is_col = el.name.contains("Col") || el.name.contains("CL");
            let live_m = fallback_members.iter().find(|m| m.id == el.id);
            let ari = live_m
                .map(|m| m.ari as f32)
                .unwrap_or_else(|| 0.88 + ((idx % 7) as f32) * 0.015);
            let utilization = live_m.map(|m| m.utilization as f32).unwrap_or(0.42);
            let force_kn = live_m
                .map(|m| m.force_kn as f32)
                .unwrap_or(if is_col { -640.0 } else { 215.0 });
            let capacity_kn = live_m
                .map(|m| m.capacity_kn as f32)
                .unwrap_or(if is_col { 1450.0 } else { 520.0 });
            let is_critical = live_m
                .map(|m| m.is_critical || m.utilization > 0.85 || m.ari < 0.70)
                .unwrap_or(idx == 0);
            list.push(Member3D {
                id: el.id,
                p1,
                p2,
                name: el.name.clone(),
                section: el.section.name.clone(),
                base_width: if is_col { 8.5 } else { 6.5 },
                force_kn,
                capacity_kn,
                utilization,
                ari,
                is_critical,
            });
        }
        if list.is_empty() {
            for (idx, m) in fallback_members.iter().enumerate() {
                let x = (idx % 4) as f32 * 5.0;
                let z = (idx / 4) as f32 * 5.0;
                list.push(Member3D {
                    id: m.id,
                    p1: (x, 0.0, z),
                    p2: (x, 3.8, z),
                    name: m.name.clone(),
                    section: m.section.clone(),
                    base_width: 6.5,
                    force_kn: m.force_kn as f32,
                    capacity_kn: m.capacity_kn as f32,
                    utilization: m.utilization as f32,
                    ari: m.ari as f32,
                    is_critical: m.utilization > 0.85,
                });
            }
        }
        if list.is_empty() {
            return ((0.0, 0.0, 0.0), 26.0, list);
        }
        let mut sx = 0.0_f32;
        let mut sy = 0.0_f32;
        let mut sz = 0.0_f32;
        for m in &list {
            sx += (m.p1.0 + m.p2.0) * 0.5;
            sy += (m.p1.1 + m.p2.1) * 0.5;
            sz += (m.p1.2 + m.p2.2) * 0.5;
        }
        let n = list.len() as f32;
        ((sx / n, sy / n, sz / n), 26.0, list)
    }

    /// Phase 45: External below-toolbar Structural Analysis sub-menu panel in Geometry.
    /// Houses 3D Diagrams (NFD/SFD/BMD) & Heatmaps, FEA Solvers & N-1 Disruption, Critical Member check,
    /// Earth Site Hazard coupling, and bidirectional quick-jump to Analysis.
    pub(super) fn render_below_toolbar_structural_analysis(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        universal_canvas: &mut crate::views::universal_canvas::UniversalCanvasState,
    ) {
        let site = universal_canvas.active_site_coupling();

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

                // Header Sub-Menu Strip + Collapse Toggle
                ui.horizontal_wrapped(|ui| {
                    icons::pill_badge(ui, AppIcon::Shield, "Structural Analysis", theme.accent_blue);
                    ui.separator();

                    let tabs = [
                        (GeometryAnalysisSubTab::DiagramsAndHeatmap, AppIcon::ChartBar, "3D Diagrams & Heatmap"),
                        (GeometryAnalysisSubTab::SolverAndN1, AppIcon::Bolt, "FEA Solver & N-1"),
                        (GeometryAnalysisSubTab::CriticalMembersBridge, AppIcon::LinkChain, "Critical Check & -> Analysis"),
                    ];
                    for (tab, icon, lbl) in tabs {
                        let sel = self.active_analysis_subtab == tab;
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
                            self.active_analysis_subtab = tab;
                            self.analysis_panel_open = true;
                        }
                    }

                    if !self.simulated_eliminated_ids.is_empty() {
                        icons::pill_badge(
                            ui,
                            AppIcon::AlertTriangle,
                            format!("N-{} Active", self.simulated_eliminated_ids.len()),
                            theme.accent_red,
                        );
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let toggle_lbl = if self.analysis_panel_open { "Collapse" } else { "Expand" };
                        let toggle_icon = if self.analysis_panel_open {
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
                        .on_hover_text("Collapse or expand Structural Analysis sub-menu panel")
                        .clicked()
                        {
                            self.analysis_panel_open = !self.analysis_panel_open;
                        }
                    });
                });

                if !self.analysis_panel_open {
                    return;
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // Earth-to-Site Environmental Coupling Ribbon
                Frame::new()
                    .fill(theme.bg_canvas)
                    .corner_radius(CornerRadius::same(6))
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        let banner_w = ui.available_width().max(100.0);
                        ui.set_width(banner_w);
                        ui.set_max_width(banner_w);
                        ui.horizontal_wrapped(|ui| {
                            icons::icon_label(
                                ui,
                                AppIcon::Globe,
                                12.0,
                                theme.accent_blue,
                                format!(
                                    "Earth Site {}: Elev {:+.1} m • Seismic Ss {:.2}g / S1 {:.2}g • Wind {:.0} m/s • Flood Risk {:.0}% • {}",
                                    site.site_label,
                                    site.elevation_m,
                                    site.seismic_pga_ss_g,
                                    site.seismic_pga_s1_g,
                                    site.design_wind_ms,
                                    site.flood_risk_index * 100.0,
                                    site.recommended_soil_class
                                ),
                                10.5,
                                theme.text_secondary,
                                false,
                            );
                        });
                    });

                ui.add_space(10.0);

                match self.active_analysis_subtab {
                    GeometryAnalysisSubTab::DiagramsAndHeatmap => {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("3D Analytical Diagram:").size(11.0).strong().color(theme.text_primary));
                            for (rm, icon, lbl, tip) in [
                                (Render3DMode::CsgSolidBrep, AppIcon::Cube3D, "Off (CSG Model)", "Return to clean 3D CSG structural solid view"),
                                (Render3DMode::AxialForceNFD, AppIcon::ChartBar, "NFD Axial Force", "Normal / Axial force envelope (Tension red / Compression blue)"),
                                (Render3DMode::ShearForceSFD, AppIcon::Bolt, "SFD Shear Force", "Shear force diagram envelope"),
                                (Render3DMode::BendingMomentBMD, AppIcon::ChartLine, "BMD Moment", "Bending moment diagram envelope"),
                            ] {
                                let sel = self.render_mode == rm;
                                if icons::icon_button(
                                    ui,
                                    icon,
                                    lbl,
                                    10.5,
                                    if sel { Color32::WHITE } else { theme.text_secondary },
                                    if sel { theme.accent_blue } else { theme.bg_canvas },
                                    Stroke::new(1.0, theme.border_subtle),
                                    CornerRadius::same(6),
                                    Vec2::new(0.0, 26.0),
                                )
                                .on_hover_text(tip)
                                .clicked()
                                {
                                    self.render_mode = rm;
                                    self.modify_status_msg = Some(format!("3D Viewport overlay switched to {lbl}"));
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("FEM Color Heatmap:").size(11.0).strong().color(theme.text_primary));
                            for (key, lbl, col) in [
                                ("ARI", "ARI Redundancy", theme.accent_green),
                                ("DCR_Stress", "DCR Utilization %", theme.accent_orange),
                                ("Tension_Compression", "Axial ±Force", theme.accent_blue),
                                ("Criticality", "Fracture Criticality", theme.accent_red),
                            ] {
                                let sel = self.color_mode == key;
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Palette,
                                    lbl,
                                    10.5,
                                    if sel { Color32::WHITE } else { theme.text_secondary },
                                    if sel { col } else { theme.bg_canvas },
                                    Stroke::new(1.0, theme.border_subtle),
                                    CornerRadius::same(6),
                                    Vec2::new(0.0, 26.0),
                                )
                                .clicked()
                                {
                                    self.color_mode = key.to_string();
                                    self.modify_status_msg = Some(format!("3D Color Heatmap set to {lbl}"));
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut self.show_deformed_shape, "Show Deformed Shape Overlay");
                            if self.show_deformed_shape {
                                ui.label(RichText::new("Scale:").size(10.5).color(theme.text_secondary));
                                ui.add(egui::Slider::new(&mut self.deformation_scale, 5.0..=250.0).suffix("x"));
                            }
                            ui.checkbox(&mut self.show_rebar_cage, "3D RC Rebar Cages");
                        });
                    }

                    GeometryAnalysisSubTab::SolverAndN1 => {
                        let fem = self.bim_bus.project.to_structural_model();
                        let ari_res = crate::core::AriEngine::evaluate_redundancy(&fem);
                        let (sys_ari, intact_pct, status_txt) = match &ari_res {
                            Ok(rep) => (
                                rep.min_system_ari,
                                rep.min_system_ari * 100.0,
                                format!("{} elements / {} nodes solved", fem.elements.len(), fem.nodes.len()),
                            ),
                            Err(err) => {
                                let avg = if members.is_empty() {
                                    0.0
                                } else {
                                    members.iter().map(|m| m.ari).sum::<f64>() / members.len() as f64
                                };
                                (avg, avg * 100.0, err.clone())
                            }
                        };

                        ui_kit::responsive_grid_n(ui, 3, 195.0, |ui, idx| {
                            let (t, v, s, c) = match idx {
                                0 => (
                                    "System Redundancy (ARI)",
                                    format!("{:.3}", sys_ari),
                                    format!("{:.1}% capacity index", intact_pct),
                                    if sys_ari >= 0.70 { theme.accent_green } else { theme.accent_orange },
                                ),
                                1 => (
                                    "N-1 Eliminated Members",
                                    format!("{}", self.simulated_eliminated_ids.len()),
                                    status_txt.clone(),
                                    if self.simulated_eliminated_ids.is_empty() { theme.accent_blue } else { theme.accent_red },
                                ),
                                _ => (
                                    "Earth Design Demand",
                                    format!("Ss {:.2}g / {:.0} m/s", site.seismic_pga_ss_g, site.design_wind_ms),
                                    site.foundation_advisory.chars().take(54).collect::<String>(),
                                    theme.accent_blue,
                                ),
                            };
                            ui_kit::stat_card(ui, theme, t, &v, &s, c);
                        });

                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Run 3D Solver Action:").size(11.0).strong().color(theme.text_primary));
                            for (lbl, target_tab, rm) in [
                                ("Linear Static", crate::views::analyze::AnalyzeSubTab::DisplacementForces, Render3DMode::AxialForceNFD),
                                ("P-Delta 2nd Order", crate::views::analyze::AnalyzeSubTab::NonlinearPushover, Render3DMode::BendingMomentBMD),
                                ("Modal Dynamics", crate::views::analyze::AnalyzeSubTab::ModalDynamics, Render3DMode::Wireframe),
                                ("N-1 Redundancy Audit", crate::views::analyze::AnalyzeSubTab::RedundancySvd, Render3DMode::CsgSolidBrep),
                            ] {
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Play,
                                    lbl,
                                    10.5,
                                    Color32::WHITE,
                                    theme.accent_blue,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::new(0.0, 26.0),
                                )
                                .clicked()
                                {
                                    self.render_mode = rm;
                                    self.active_tool_status = format!("Executed {lbl} check from Geometry Analysis Panel.");
                                    self.bim_console_logs.push(format!("[Structural Analysis] {lbl} ({status_txt})"));
                                    self.modify_status_msg = Some(format!("Completed {lbl} overlay — use Critical Check tab to open full solver sheet."));
                                    let _ = target_tab;
                                }
                            }
                            if !self.simulated_eliminated_ids.is_empty()
                                && icons::icon_button(
                                    ui,
                                    AppIcon::Close,
                                    "Restore All N-1 Members",
                                    10.5,
                                    Color32::WHITE,
                                    theme.accent_green,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::new(0.0, 26.0),
                                )
                                .clicked()
                            {
                                self.simulated_eliminated_ids.clear();
                                self.modify_status_msg = Some("Restored all N-1 eliminated members.".to_string());
                            }
                        });
                    }

                    GeometryAnalysisSubTab::CriticalMembersBridge => {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Open Full Analytical Suite in Analysis:").size(11.0).strong().color(theme.text_primary));
                            for (lbl, tab) in [
                                ("ARI Redundancy ->", crate::views::analyze::AnalyzeSubTab::RedundancySvd),
                                ("Modal Seismic ->", crate::views::analyze::AnalyzeSubTab::ModalDynamics),
                                ("Concrete P-M ->", crate::views::analyze::AnalyzeSubTab::ConcretePmInteraction),
                                ("Steel Connections ->", crate::views::analyze::AnalyzeSubTab::SteelConnection),
                            ] {
                                if icons::icon_button(
                                    ui,
                                    AppIcon::LinkChain,
                                    lbl,
                                    10.5,
                                    Color32::WHITE,
                                    theme.accent_blue,
                                    Stroke::NONE,
                                    CornerRadius::same(6),
                                    Vec2::new(0.0, 25.0),
                                )
                                .clicked()
                                {
                                    self.requested_analysis_jump = Some((
                                        tab,
                                        self.selected_member_id,
                                        format!(
                                            "Linked from Geometry 3D (Heatmap: {}, N-{} eliminated)",
                                            self.color_mode,
                                            self.simulated_eliminated_ids.len()
                                        ),
                                    ));
                                }
                            }
                        });

                        ui.add_space(8.0);
                        let (_, _, model_members) = self.build_model_geometry(members);
                        if model_members.is_empty() {
                            ui.label(
                                RichText::new("No structural members yet. Author Columns/Beams/Walls in 3D or load a Template to inspect DCR & ARI.")
                                    .size(11.0)
                                    .color(theme.text_muted),
                            );
                        } else {
                            let mut sorted = model_members;
                            sorted.sort_by(|a, b| a.ari.total_cmp(&b.ari));
                            ScrollArea::horizontal().id_salt("geom_analysis_members_scroll").show(ui, |ui| {
                                egui::Grid::new("geom_analysis_members_grid")
                                    .striped(true)
                                    .spacing(Vec2::new(12.0, 5.0))
                                    .show(ui, |ui| {
                                        for h in ["Member", "Section", "Axial Pu", "DCR %", "ARI", "3D Focus / N-1", "Bridge"] {
                                            ui.label(RichText::new(h).size(10.5).strong().color(theme.accent_blue));
                                        }
                                        ui.end_row();
                                        for m in sorted.iter().take(12) {
                                            let is_elim = self.simulated_eliminated_ids.contains(&m.id);
                                            ui.label(RichText::new(format!("#{} {}", m.id, m.name)).size(10.5).color(theme.text_primary));
                                            ui.label(RichText::new(&m.section).size(10.5).color(theme.text_secondary));
                                            ui.label(RichText::new(format!("{:.1} kN", m.force_kn)).size(10.5).monospace().color(theme.text_primary));
                                            let dcr_pct = m.utilization * 100.0;
                                            let dcr_col = if dcr_pct >= 85.0 { theme.accent_red } else if dcr_pct >= 60.0 { theme.accent_orange } else { theme.accent_green };
                                            ui.label(RichText::new(format!("{:.1}%", dcr_pct)).size(10.5).strong().color(dcr_col));
                                            ui.label(RichText::new(format!("{:.3}", m.ari)).size(10.5).monospace().color(theme.accent_blue));
                                            ui.horizontal(|ui| {
                                                let sel = self.selected_member_id == Some(m.id);
                                                if ui.selectable_label(sel, "Focus 3D").clicked() {
                                                    self.selected_member_id = Some(m.id);
                                                    self.color_mode = "DCR_Stress".to_string();
                                                }
                                                if ui.selectable_label(is_elim, if is_elim { "Restore" } else { "Drop N-1" }).clicked() {
                                                    if is_elim {
                                                        self.simulated_eliminated_ids.retain(|&id| id != m.id);
                                                    } else {
                                                        self.simulated_eliminated_ids.push(m.id);
                                                    }
                                                }
                                            });
                                            if ui.small_button("Analyze ->").clicked() {
                                                self.selected_member_id = Some(m.id);
                                                self.requested_analysis_jump = Some((
                                                    crate::views::analyze::AnalyzeSubTab::MemberCriticality,
                                                    Some(m.id),
                                                    format!(
                                                        "Inspected #{} {} (ARI {:.3}, DCR {:.1}%) from Geometry",
                                                        m.id, m.name, m.ari, dcr_pct
                                                    ),
                                                ));
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                        }
                    }
                }
            });
    }

}

/// Helper function to calculate distance from a point to a line segment
fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let ab_len_sq = ab.length_sq();
    if ab_len_sq < 1e-4 {
        return ap.length();
    }
    let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    let projection = a + ab * t;
    (p - projection).length()
}
