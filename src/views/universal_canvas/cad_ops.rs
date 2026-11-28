//! Undo/redo, CAD precision transforms, snapping, and camera fit operations.

use super::*;
use egui::{Pos2, Rect, Vec2};

use crate::core::gis::SlippyMapState;
use crate::core::BimProject;
use crate::models::NavigationPillar;

impl UniversalCanvasState {
    /// Pushes current entity list to undo history before mutating.
    pub(super) fn push_undo(&mut self) {
        self.undo_stack.push(self.entities.clone());
        if self.undo_stack.len() > 30 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.entities.clone());
            if self.redo_stack.len() > 30 {
                self.redo_stack.remove(0);
            }
            self.entities = prev;
            if let Some(sel) = self.selected_entity_id {
                if !self.entities.iter().any(|e| e.id == sel) {
                    self.selected_entity_id = None;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.entities.clone());
            if self.undo_stack.len() > 30 {
                self.undo_stack.remove(0);
            }
            self.entities = next;
            if let Some(sel) = self.selected_entity_id {
                if !self.entities.iter().any(|e| e.id == sel) {
                    self.selected_entity_id = None;
                }
            }
            true
        } else {
            false
        }
    }

    /// Polls CAD keyboard shortcuts only when no text field holds focus.
    pub fn poll_cad_shortcuts(ctx: &egui::Context, is_frozen: bool) -> CadShortcutFeedback {
        if ctx.egui_wants_keyboard_input() {
            return CadShortcutFeedback::default();
        }
        ctx.input(|i| {
            let mut fb = CadShortcutFeedback::default();
            if i.key_pressed(egui::Key::Space) && !i.modifiers.any() {
                fb.toggle_freeze = true;
            }
            if is_frozen {
                return fb;
            }
            fb.escape = i.key_pressed(egui::Key::Escape);
            fb.delete_selected =
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace);
            fb.undo = i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z);
            fb.redo = (i.modifiers.command && i.key_pressed(egui::Key::Y))
                || (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z));
            fb.toggle_ortho = i.key_pressed(egui::Key::F8);
            fb.toggle_snap = i.key_pressed(egui::Key::F3);
            fb.toggle_osnap = i.key_pressed(egui::Key::F4);

            if !i.modifiers.command && !i.modifiers.ctrl && !i.modifiers.alt {
                if i.key_pressed(egui::Key::Z) {
                    fb.fit_model = true;
                } else {
                    const KEYS: [(egui::Key, char); 14] = [
                        (egui::Key::W, 'W'),
                        (egui::Key::B, 'B'),
                        (egui::Key::C, 'C'),
                        (egui::Key::F, 'F'),
                        (egui::Key::L, 'L'),
                        (egui::Key::R, 'R'),
                        (egui::Key::D, 'D'),
                        (egui::Key::M, 'M'),
                        (egui::Key::O, 'O'),
                        (egui::Key::X, 'X'),
                        (egui::Key::E, 'E'),
                        (egui::Key::T, 'T'),
                        (egui::Key::A, 'A'),
                        (egui::Key::V, 'V'),
                    ];
                    for (key, ch) in KEYS {
                        if i.key_pressed(key) {
                            fb.letter_key = Some(ch);
                            break;
                        }
                    }
                }
            }
            fb
        })
    }

    /// Constraints `end_mm` orthogonally relative to `start_mm` when `ortho_mode` is enabled.
    pub fn apply_ortho(&self, start_mm: [f64; 2], end_mm: [f64; 2]) -> [f64; 2] {
        if !self.ortho_mode {
            return end_mm;
        }
        let dx = (end_mm[0] - start_mm[0]).abs();
        let dy = (end_mm[1] - start_mm[1]).abs();
        if dx >= dy {
            [end_mm[0], start_mm[1]]
        } else {
            [start_mm[0], end_mm[1]]
        }
    }

    pub(super) fn target_entity_idx(&self) -> Option<usize> {
        if let Some(sel) = self.selected_entity_id {
            if let Some(pos) = self.entities.iter().position(|e| e.id == sel) {
                return Some(pos);
            }
        }
        self.entities.len().checked_sub(1)
    }

    pub(super) fn target_entity_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = if !self.selected_entity_ids.is_empty() {
            self.entities
                .iter()
                .enumerate()
                .filter(|(_, e)| self.selected_entity_ids.contains(&e.id))
                .map(|(i, _)| i)
                .collect()
        } else {
            Vec::new()
        };
        if indices.is_empty() {
            if let Some(idx) = self.target_entity_idx() {
                indices.push(idx);
            }
        }
        indices
    }

    pub(super) fn is_entity_selected(&self, id: usize) -> bool {
        self.selected_entity_id == Some(id) || self.selected_entity_ids.contains(&id)
    }

    pub(super) fn set_single_selection(&mut self, id: Option<usize>) {
        self.selected_entity_id = id;
        self.selected_entity_ids.clear();
        if let Some(eid) = id {
            self.selected_entity_ids.push(eid);
        }
    }

    pub(super) fn translate_kind(kind: &mut CanvasShapeKind, dx_mm: f64, dy_mm: f64) {
        match kind {
            CanvasShapeKind::Line { start_mm, end_mm }
            | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                start_mm[0] += dx_mm;
                start_mm[1] += dy_mm;
                end_mm[0] += dx_mm;
                end_mm[1] += dy_mm;
            }
            CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                min_mm[0] += dx_mm;
                min_mm[1] += dy_mm;
                max_mm[0] += dx_mm;
                max_mm[1] += dy_mm;
            }
            CanvasShapeKind::Circle { center_mm, .. } => {
                center_mm[0] += dx_mm;
                center_mm[1] += dy_mm;
            }
            CanvasShapeKind::Freehand { points_mm } => {
                for p in points_mm {
                    p[0] += dx_mm;
                    p[1] += dy_mm;
                }
            }
            CanvasShapeKind::Callout { pos_mm, .. } => {
                pos_mm[0] += dx_mm;
                pos_mm[1] += dy_mm;
            }
        }
    }

    pub(super) fn entity_aabb(kind: &CanvasShapeKind) -> ([f64; 2], [f64; 2]) {
        match kind {
            CanvasShapeKind::Line { start_mm, end_mm }
            | CanvasShapeKind::Dimension { start_mm, end_mm } => (
                [start_mm[0].min(end_mm[0]), start_mm[1].min(end_mm[1])],
                [start_mm[0].max(end_mm[0]), start_mm[1].max(end_mm[1])],
            ),
            CanvasShapeKind::Rectangle { min_mm, max_mm } => (*min_mm, *max_mm),
            CanvasShapeKind::Circle { center_mm, radius_mm } => (
                [center_mm[0] - radius_mm, center_mm[1] - radius_mm],
                [center_mm[0] + radius_mm, center_mm[1] + radius_mm],
            ),
            CanvasShapeKind::Freehand { points_mm } => {
                if points_mm.is_empty() {
                    ([0.0, 0.0], [0.0, 0.0])
                } else {
                    let mut mn = points_mm[0];
                    let mut mx = points_mm[0];
                    for p in points_mm {
                        mn[0] = mn[0].min(p[0]);
                        mn[1] = mn[1].min(p[1]);
                        mx[0] = mx[0].max(p[0]);
                        mx[1] = mx[1].max(p[1]);
                    }
                    (mn, mx)
                }
            }
            CanvasShapeKind::Callout { pos_mm, .. } => (*pos_mm, *pos_mm),
        }
    }

    /// Selects entities inside marquee `[a_mm, b_mm]`. Left->Right (`b_mm[0] >= a_mm[0]`) uses Window (fully enclosed); Right->Left (`b_mm[0] < a_mm[0]`) uses Crossing (intersecting/overlapping).
    pub fn select_marquee(&mut self, a_mm: [f64; 2], b_mm: [f64; 2]) -> usize {
        let min_x = a_mm[0].min(b_mm[0]);
        let max_x = a_mm[0].max(b_mm[0]);
        let min_y = a_mm[1].min(b_mm[1]);
        let max_y = a_mm[1].max(b_mm[1]);
        let is_window = b_mm[0] >= a_mm[0];

        self.selected_entity_ids.clear();
        for ent in &self.entities {
            if !self.visibility_graphics.is_cad_layer_visible(&ent.layer_id) {
                continue;
            }
            let (e_min, e_max) = Self::entity_aabb(&ent.kind);
            let matched = if is_window {
                e_min[0] >= min_x && e_max[0] <= max_x && e_min[1] >= min_y && e_max[1] <= max_y
            } else {
                e_max[0] >= min_x && e_min[0] <= max_x && e_max[1] >= min_y && e_min[1] <= max_y
            };
            if matched {
                self.selected_entity_ids.push(ent.id);
            }
        }
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        self.selected_entity_ids.len()
    }

    fn unlocked_target_entity_indices(&mut self) -> Vec<usize> {
        let raw = self.target_entity_indices();
        if raw.is_empty() {
            return raw;
        }
        let unlocked: Vec<usize> = raw
            .into_iter()
            .filter(|&idx| {
                self.entities
                    .get(idx)
                    .is_some_and(|e| !self.visibility_graphics.is_cad_layer_locked(&e.layer_id))
            })
            .collect();
        if unlocked.is_empty() {
            self.status_message = Some(
                "Action blocked: selected 2D CAD layer is locked (unlock in Layers / VG)."
                    .to_string(),
            );
        }
        unlocked
    }

    /// Translates the selected entities (or latest entity) by `(dx_mm, dy_mm)` with undo support.
    pub fn translate_selected_or_last(&mut self, dx_mm: f64, dy_mm: f64) -> bool {
        let indices = self.unlocked_target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let mut moved_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            if let Some(ent) = self.entities.get_mut(idx) {
                Self::translate_kind(&mut ent.kind, dx_mm, dy_mm);
                moved_ids.push(ent.id);
            }
        }
        self.selected_entity_ids = moved_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Creates a parallel/outward offset duplicate of the selected entities (or latest entity).
    pub fn offset_selected_or_last(&mut self, offset_mm: f64) -> bool {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let mut new_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            let mut cloned = self.entities[idx].clone();
            match &mut cloned.kind {
                CanvasShapeKind::Line { start_mm, end_mm }
                | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    let vx = end_mm[0] - start_mm[0];
                    let vy = end_mm[1] - start_mm[1];
                    let len = vx.hypot(vy).max(1.0);
                    let nx = -vy / len * offset_mm;
                    let ny = vx / len * offset_mm;
                    start_mm[0] += nx;
                    start_mm[1] += ny;
                    end_mm[0] += nx;
                    end_mm[1] += ny;
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    min_mm[0] -= offset_mm;
                    min_mm[1] -= offset_mm;
                    max_mm[0] += offset_mm;
                    max_mm[1] += offset_mm;
                }
                CanvasShapeKind::Circle { radius_mm, .. } => {
                    *radius_mm = (*radius_mm + offset_mm).max(100.0);
                }
                CanvasShapeKind::Freehand { points_mm } => {
                    for p in points_mm {
                        p[0] += offset_mm;
                        p[1] += offset_mm;
                    }
                }
                CanvasShapeKind::Callout { pos_mm, .. } => {
                    pos_mm[0] += offset_mm;
                    pos_mm[1] += offset_mm;
                }
            }
            let new_id = self.next_entity_id;
            self.next_entity_id += 1;
            cloned.id = new_id;
            self.entities.push(cloned);
            new_ids.push(new_id);
        }
        self.selected_entity_ids = new_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Creates a mirrored duplicate of the selected entities (or latest entity) across vertical axis `x = mirror_x_mm`.
    pub fn mirror_selected_or_last(&mut self, mirror_x_mm: f64) -> bool {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let mx = |x: f64| 2.0 * mirror_x_mm - x;
        let mut new_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            let mut cloned = self.entities[idx].clone();
            match &mut cloned.kind {
                CanvasShapeKind::Line { start_mm, end_mm }
                | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                    start_mm[0] = mx(start_mm[0]);
                    end_mm[0] = mx(end_mm[0]);
                }
                CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                    let nx0 = mx(max_mm[0]);
                    let nx1 = mx(min_mm[0]);
                    min_mm[0] = nx0.min(nx1);
                    max_mm[0] = nx0.max(nx1);
                }
                CanvasShapeKind::Circle { center_mm, .. } => {
                    center_mm[0] = mx(center_mm[0]);
                }
                CanvasShapeKind::Freehand { points_mm } => {
                    for p in points_mm {
                        p[0] = mx(p[0]);
                    }
                }
                CanvasShapeKind::Callout { pos_mm, .. } => {
                    pos_mm[0] = mx(pos_mm[0]);
                }
            }
            let new_id = self.next_entity_id;
            self.next_entity_id += 1;
            cloned.id = new_id;
            self.entities.push(cloned);
            new_ids.push(new_id);
        }
        self.selected_entity_ids = new_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Parses CAD Cartesian (`@dx,dy` or `dx,dy`) or polar (`@dist<deg` or `dist<deg`) coordinate vectors in mm.
    pub fn parse_cad_delta_expr(raw: &str) -> Option<(f64, f64)> {
        let s = raw.trim().trim_start_matches('@');
        if let Some((d_str, a_str)) = s.split_once('<') {
            let dist = d_str.trim().parse::<f64>().ok()?;
            let rad = a_str.trim().parse::<f64>().ok()?.to_radians();
            return Some((dist * rad.cos(), dist * rad.sin()));
        }
        if let Some((x_str, y_str)) = s.split_once(',') {
            let dx = x_str.trim().parse::<f64>().ok()?;
            let dy = y_str.trim().parse::<f64>().ok()?;
            return Some((dx, dy));
        }
        None
    }

    /// Creates a displaced duplicate of the selected entities (or latest entity) by `(dx_mm, dy_mm)` with undo support.
    pub fn duplicate_selected_or_last(&mut self, dx_mm: f64, dy_mm: f64) -> bool {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let mut new_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            let mut cloned = self.entities[idx].clone();
            Self::translate_kind(&mut cloned.kind, dx_mm, dy_mm);
            let new_id = self.next_entity_id;
            self.next_entity_id += 1;
            cloned.id = new_id;
            self.entities.push(cloned);
            new_ids.push(new_id);
        }
        self.selected_entity_ids = new_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Computes engineering summary metrics for a single 2D CAD sketch entity by ID.
    pub fn entity_summary(&self, id: usize) -> Option<(&'static str, f64, f64, f64, String, [u8; 3], f32)> {
        let ent = self.entities.iter().find(|e| e.id == id)?;
        let (kind_label, length_m, area_m2, radius_m, detail) = match &ent.kind {
            CanvasShapeKind::Line { start_mm, end_mm } => {
                let dx = (end_mm[0] - start_mm[0]) * 0.001;
                let dy = (end_mm[1] - start_mm[1]) * 0.001;
                let len = dx.hypot(dy);
                (
                    "Line Segment",
                    len,
                    0.0,
                    0.0,
                    format!(
                        "({:.2}, {:.2}) -> ({:.2}, {:.2}) m",
                        start_mm[0] * 0.001,
                        start_mm[1] * 0.001,
                        end_mm[0] * 0.001,
                        end_mm[1] * 0.001
                    ),
                )
            }
            CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                let w = (max_mm[0] - min_mm[0]).abs() * 0.001;
                let h = (max_mm[1] - min_mm[1]).abs() * 0.001;
                ("Rectangle", 2.0 * (w + h), w * h, 0.0, format!("{w:.2}m x {h:.2}m"))
            }
            CanvasShapeKind::Circle { center_mm, radius_mm } => {
                let r = (*radius_mm).abs() * 0.001;
                (
                    "Circle",
                    2.0 * std::f64::consts::PI * r,
                    std::f64::consts::PI * r * r,
                    r,
                    format!(
                        "Center ({:.2}, {:.2}) m",
                        center_mm[0] * 0.001,
                        center_mm[1] * 0.001
                    ),
                )
            }
            CanvasShapeKind::Dimension { start_mm, end_mm } => {
                let len =
                    ((end_mm[0] - start_mm[0]) * 0.001).hypot((end_mm[1] - start_mm[1]) * 0.001);
                ("Dimension", len, 0.0, 0.0, format!("Span {len:.3} m"))
            }
            CanvasShapeKind::Freehand { points_mm } => {
                let mut len = 0.0;
                for w in points_mm.windows(2) {
                    len += ((w[1][0] - w[0][0]) * 0.001).hypot((w[1][1] - w[0][1]) * 0.001);
                }
                ("Polyline", len, 0.0, 0.0, format!("{} vertices", points_mm.len()))
            }
            CanvasShapeKind::Callout { pos_mm, text } => (
                "Annotation Callout",
                0.0,
                0.0,
                0.0,
                format!(
                    "{} @ ({:.2}, {:.2}) m",
                    text,
                    pos_mm[0] * 0.001,
                    pos_mm[1] * 0.001
                ),
            ),
        };
        Some((
            kind_label,
            length_m,
            area_m2,
            radius_m,
            detail,
            ent.color_rgb,
            ent.stroke_width,
        ))
    }

    /// Updates stroke width and RGB color for a single CAD sketch entity with undo support.
    pub fn update_entity_style(&mut self, id: usize, color_rgb: [u8; 3], stroke_width: f32) -> bool {
        if !self.entities.iter().any(|e| e.id == id) {
            return false;
        }
        self.push_undo();
        if let Some(ent) = self.entities.iter_mut().find(|e| e.id == id) {
            ent.color_rgb = color_rgb;
            ent.stroke_width = stroke_width.clamp(0.8, 12.0);
        }
        true
    }

    /// Duplicates a specific CAD sketch entity with `(dx_mm, dy_mm)` offset and undo support.
    pub fn duplicate_offset_entity(&mut self, id: usize, dx_mm: f64, dy_mm: f64) -> Option<usize> {
        let idx = self.entities.iter().position(|e| e.id == id)?;
        self.push_undo();
        let mut cloned = self.entities[idx].clone();
        Self::translate_kind(&mut cloned.kind, dx_mm, dy_mm);
        let new_id = self.next_entity_id;
        self.next_entity_id += 1;
        cloned.id = new_id;
        self.entities.push(cloned);
        self.set_single_selection(Some(new_id));
        Some(new_id)
    }

    /// Deletes a specific CAD sketch entity by ID with undo support.
    pub fn delete_entity_by_id(&mut self, id: usize) -> bool {
        let Some(pos) = self.entities.iter().position(|e| e.id == id) else {
            return false;
        };
        self.push_undo();
        self.entities.remove(pos);
        self.selected_entity_ids.retain(|&eid| eid != id);
        if self.selected_entity_id == Some(id) {
            self.selected_entity_id = self.selected_entity_ids.first().copied();
        }
        true
    }

    /// Clears recorded geodesic measurement vertices.
    pub fn clear_measure(&mut self) {
        self.measure_points_lat_lon.clear();
    }

    /// Computes live geodesic measurement summary: `(polyline_m, optional_polygon_area_m2, vertex_count)`.
    pub fn geodesic_measure_summary(&self) -> (f64, Option<f64>, usize) {
        let total_m = crate::core::gis::SlippyMapState::geodetic_polyline_length_m(
            &self.measure_points_lat_lon,
        );
        let area_m2 = if self.measure_area_mode && self.measure_points_lat_lon.len() >= 3 {
            Some(crate::core::gis::SlippyMapState::geodetic_polygon_area_m2(
                &self.measure_points_lat_lon,
            ))
        } else {
            None
        };
        (total_m, area_m2, self.measure_points_lat_lon.len())
    }

    /// Adds a precision parametric 2D Line anchored at `(0, 0)` with exact length `length_m` and azimuth `angle_deg`.
    pub fn commit_parametric_2d_line(
        &mut self,
        length_m: f64,
        angle_deg: f64,
        pillar_badge: &str,
        pillar_name: &str,
    ) -> usize {
        self.push_undo();
        let len_mm = length_m.abs().max(0.1) * 1000.0;
        let rad = angle_deg.to_radians();
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.push(CanvasEntity {
            id,
            pillar_badge: pillar_badge.to_string(),
            pillar_name: pillar_name.to_string(),
            kind: CanvasShapeKind::Line {
                start_mm: [0.0, 0.0],
                end_mm: [len_mm * rad.cos(), len_mm * rad.sin()],
            },
            color_rgb: self.visibility_graphics.active_cad_layer_color_rgb(),
            stroke_width: 2.2,
            layer_id: self.visibility_graphics.active_cad_layer_id.clone(),
        });
        self.set_single_selection(Some(id));
        id
    }

    /// Adds a precision parametric 2D Rectangle centered at the origin with `width_m` x `height_m`.
    pub fn commit_parametric_2d_rect(
        &mut self,
        width_m: f64,
        height_m: f64,
        pillar_badge: &str,
        pillar_name: &str,
    ) -> usize {
        self.push_undo();
        let hw = width_m.abs().max(0.1) * 500.0;
        let hh = height_m.abs().max(0.1) * 500.0;
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.push(CanvasEntity {
            id,
            pillar_badge: pillar_badge.to_string(),
            pillar_name: pillar_name.to_string(),
            kind: CanvasShapeKind::Rectangle {
                min_mm: [-hw, -hh],
                max_mm: [hw, hh],
            },
            color_rgb: self.visibility_graphics.active_cad_layer_color_rgb(),
            stroke_width: 2.2,
            layer_id: self.visibility_graphics.active_cad_layer_id.clone(),
        });
        self.set_single_selection(Some(id));
        id
    }

    /// Adds a precision parametric 2D Circle centered at the origin with `radius_m`.
    pub fn commit_parametric_2d_circle(
        &mut self,
        radius_m: f64,
        pillar_badge: &str,
        pillar_name: &str,
    ) -> usize {
        self.push_undo();
        let r_mm = radius_m.abs().max(0.1) * 1000.0;
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.push(CanvasEntity {
            id,
            pillar_badge: pillar_badge.to_string(),
            pillar_name: pillar_name.to_string(),
            kind: CanvasShapeKind::Circle {
                center_mm: [0.0, 0.0],
                radius_mm: r_mm,
            },
            color_rgb: self.visibility_graphics.active_cad_layer_color_rgb(),
            stroke_width: 2.2,
            layer_id: self.visibility_graphics.active_cad_layer_id.clone(),
        });
        self.set_single_selection(Some(id));
        id
    }

    /// Rotates the selected entities (or latest entity) around each entity centroid by `angle_deg` degrees with undo support.
    pub fn rotate_selected_or_last(&mut self, angle_deg: f64) -> bool {
        let indices = self.unlocked_target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        let rad = angle_deg.to_radians();
        let (c, s) = (rad.cos(), rad.sin());
        let rot_pt = |pt: [f64; 2], pivot: [f64; 2]| -> [f64; 2] {
            let rx = pt[0] - pivot[0];
            let ry = pt[1] - pivot[1];
            [pivot[0] + rx * c - ry * s, pivot[1] + rx * s + ry * c]
        };
        let mut rot_ids = Vec::with_capacity(indices.len());
        for idx in indices {
            if let Some(ent) = self.entities.get_mut(idx) {
                match &mut ent.kind {
                    CanvasShapeKind::Line { start_mm, end_mm }
                    | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                        let pivot = [
                            0.5 * (start_mm[0] + end_mm[0]),
                            0.5 * (start_mm[1] + end_mm[1]),
                        ];
                        *start_mm = rot_pt(*start_mm, pivot);
                        *end_mm = rot_pt(*end_mm, pivot);
                    }
                    CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                        let pivot = [0.5 * (min_mm[0] + max_mm[0]), 0.5 * (min_mm[1] + max_mm[1])];
                        let corners = [
                            rot_pt([min_mm[0], min_mm[1]], pivot),
                            rot_pt([max_mm[0], min_mm[1]], pivot),
                            rot_pt([max_mm[0], max_mm[1]], pivot),
                            rot_pt([min_mm[0], max_mm[1]], pivot),
                        ];
                        let x0 = corners.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                        let x1 = corners.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
                        let y0 = corners.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
                        let y1 = corners.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
                        *min_mm = [x0, y0];
                        *max_mm = [x1, y1];
                    }
                    CanvasShapeKind::Freehand { points_mm } if !points_mm.is_empty() => {
                        let n = points_mm.len() as f64;
                        let pivot = [
                            points_mm.iter().map(|p| p[0]).sum::<f64>() / n,
                            points_mm.iter().map(|p| p[1]).sum::<f64>() / n,
                        ];
                        for p in points_mm {
                            *p = rot_pt(*p, pivot);
                        }
                    }
                    CanvasShapeKind::Circle { .. }
                    | CanvasShapeKind::Callout { .. }
                    | CanvasShapeKind::Freehand { .. } => {}
                }
                rot_ids.push(ent.id);
            }
        }
        self.selected_entity_ids = rot_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        true
    }

    /// Generates an orthogonal CAD structural grid (`nx` columns by `ny` rows at spacing `dx_mm`, `dy_mm`).
    pub fn generate_structural_grid(
        &mut self,
        nx: usize,
        ny: usize,
        dx_mm: f64,
        dy_mm: f64,
        active_pillar: NavigationPillar,
        origin_mm: [f64; 2],
    ) -> usize {
        let cols = nx.clamp(2, 16);
        let rows = ny.clamp(2, 16);
        let sx = dx_mm.clamp(500.0, 30_000.0);
        let sy = dy_mm.clamp(500.0, 30_000.0);
        self.push_undo();

        let total_w = (cols - 1) as f64 * sx;
        let total_h = (rows - 1) as f64 * sy;
        let badge = active_pillar.badge().to_string();
        let pname = active_pillar.clean_name().to_string();
        let col = self.active_color_rgb();
        let mut created_ids = Vec::with_capacity(cols + rows);

        for ix in 0..cols {
            let x = origin_mm[0] + ix as f64 * sx;
            let id = self.next_entity_id;
            self.next_entity_id += 1;
            self.entities.push(CanvasEntity {
                id,
                pillar_badge: badge.clone(),
                pillar_name: pname.clone(),
                kind: CanvasShapeKind::Line {
                    start_mm: [x, origin_mm[1]],
                    end_mm: [x, origin_mm[1] + total_h],
                },
                color_rgb: col,
                stroke_width: self.stroke_width,
                layer_id: "S-Grid".to_string(),
            });
            created_ids.push(id);
        }
        for iy in 0..rows {
            let y = origin_mm[1] + iy as f64 * sy;
            let id = self.next_entity_id;
            self.next_entity_id += 1;
            self.entities.push(CanvasEntity {
                id,
                pillar_badge: badge.clone(),
                pillar_name: pname.clone(),
                kind: CanvasShapeKind::Line {
                    start_mm: [origin_mm[0], y],
                    end_mm: [origin_mm[0] + total_w, y],
                },
                color_rgb: col,
                stroke_width: self.stroke_width,
                layer_id: "S-Grid".to_string(),
            });
            created_ids.push(id);
        }
        let n = created_ids.len();
        self.selected_entity_ids = created_ids;
        self.selected_entity_id = self.selected_entity_ids.first().copied();
        n
    }

    /// Creates a rectangular CAD `ARRAY` of `cols x rows` clones spaced by `(dx_mm, dy_mm)`.
    pub fn array_selected_or_last(&mut self, cols: usize, rows: usize, dx_mm: f64, dy_mm: f64) -> usize {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return 0;
        }
        let c_cnt = cols.clamp(1, 12);
        let r_cnt = rows.clamp(1, 12);
        self.push_undo();
        let mut new_ids = Vec::new();
        for idx in indices {
            let template = self.entities[idx].clone();
            for r in 0..r_cnt {
                for c in 0..c_cnt {
                    if c == 0 && r == 0 {
                        continue;
                    }
                    let mut cloned = template.clone();
                    Self::translate_kind(&mut cloned.kind, c as f64 * dx_mm, r as f64 * dy_mm);
                    let nid = self.next_entity_id;
                    self.next_entity_id += 1;
                    cloned.id = nid;
                    self.entities.push(cloned);
                    new_ids.push(nid);
                }
            }
        }
        let added = new_ids.len();
        if added > 0 {
            self.selected_entity_ids = new_ids;
            self.selected_entity_id = self.selected_entity_ids.first().copied();
        }
        added
    }

    /// Promotes selected (or latest) 2D sketch lines, rectangles, and dimensions into native 3D BIM elements (`BimWall`, `BimRun`, `BimColumn`, `BimSlab`).
    pub fn promote_selection_to_bim(&mut self, bim_project: &mut BimProject) -> (usize, usize, usize) {
        let indices = self.target_entity_indices();
        if indices.is_empty() {
            return (0, 0, 0);
        }
        let base_lvl_id = bim_project.levels.first().map(|l| l.id).unwrap_or(1);
        let top_lvl_id = bim_project.levels.get(1).map(|l| l.id).unwrap_or(base_lvl_id);
        let mut walls_added = 0usize;
        let mut runs_added = 0usize;
        let mut slabs_added = 0usize;

        for idx in indices {
            if let Some(ent) = self.entities.get(idx) {
                match &ent.kind {
                    CanvasShapeKind::Line { start_mm, end_mm } => {
                        let p0 = crate::core::P2(start_mm[0], start_mm[1]);
                        let p1 = crate::core::P2(end_mm[0], end_mm[1]);
                        if p0.dist(p1) >= 150.0 {
                            let wid = bim_project.alloc_id();
                            bim_project.walls.push(crate::core::BimWall {
                                id: wid,
                                name: format!("Sketch Wall W-{wid}"),
                                base_level_id: base_lvl_id,
                                top_level_id: Some(top_lvl_id).filter(|&t| t != base_lvl_id),
                                unconnected_height_mm: 3600.0,
                                start: p0,
                                end: p1,
                                thickness_mm: 200.0,
                                is_structural: true,
                                layers: crate::core::BimWall::default_compound_layers(200.0),
                                openings: Vec::new(),
                            });
                            walls_added += 1;

                            let rid = bim_project.alloc_id();
                            bim_project.runs.push(crate::core::BimRun {
                                id: rid,
                                name: format!("Sketch Beam B-{rid}"),
                                category: crate::core::RunCategory::StructuralBeam,
                                level_id: top_lvl_id,
                                start: p0,
                                end: p1,
                                start_offset_mm: 0.0,
                                end_offset_mm: 0.0,
                                justify: crate::core::RunJustify::Top,
                                profile_name: "WF 250x125".to_string(),
                                width_mm: 250.0,
                                depth_mm: 400.0,
                                is_round: false,
                                is_wf_profile: true,
                                system_name: "CAD Promote".to_string(),
                            });
                            runs_added += 1;

                            // Ensure end columns exist near endpoints for full FEA continuity
                            for pt in [p0, p1] {
                                let exists = bim_project.columns.iter().any(|c| c.pos.dist(pt) < 120.0);
                                if !exists {
                                    let cid = bim_project.alloc_id();
                                    bim_project.columns.push(crate::core::BimColumn {
                                        id: cid,
                                        name: format!("Col C-{cid}"),
                                        base_level_id: base_lvl_id,
                                        top_level_id: top_lvl_id,
                                        pos: pt,
                                        width_mm: 350.0,
                                        depth_mm: 350.0,
                                        rotation_deg: 0.0,
                                        is_round: false,
                                        is_wf_profile: false,
                                        profile_name: "RC 350x350".to_string(),
                                    });
                                }
                            }
                        }
                    }
                    CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                        let c0 = crate::core::P2(min_mm[0], min_mm[1]);
                        let c1 = crate::core::P2(max_mm[0], min_mm[1]);
                        let c2 = crate::core::P2(max_mm[0], max_mm[1]);
                        let c3 = crate::core::P2(min_mm[0], max_mm[1]);
                        if c0.dist(c1) >= 200.0 && c1.dist(c2) >= 200.0 {
                            let sid = bim_project.alloc_id();
                            bim_project.slabs.push(crate::core::BimSlab {
                                id: sid,
                                name: format!("Sketch Slab S-{sid}"),
                                level_id: base_lvl_id,
                                boundary: vec![c0, c1, c2, c3],
                                openings: Vec::new(),
                                thickness_mm: 160.0,
                                is_roof: false,
                                roof_slope_deg: 0.0,
                            });
                            slabs_added += 1;
                            for (p0, p1) in [(c0, c1), (c1, c2), (c2, c3), (c3, c0)] {
                                let wid = bim_project.alloc_id();
                                bim_project.walls.push(crate::core::BimWall {
                                    id: wid,
                                    name: format!("Perimeter Wall W-{wid}"),
                                    base_level_id: base_lvl_id,
                                    top_level_id: Some(top_lvl_id).filter(|&t| t != base_lvl_id),
                                    unconnected_height_mm: 3600.0,
                                    start: p0,
                                    end: p1,
                                    thickness_mm: 200.0,
                                    is_structural: true,
                                    layers: crate::core::BimWall::default_compound_layers(200.0),
                                    openings: Vec::new(),
                                });
                                walls_added += 1;
                            }
                            for pt in [c0, c1, c2, c3] {
                                let exists = bim_project.columns.iter().any(|c| c.pos.dist(pt) < 120.0);
                                if !exists {
                                    let cid = bim_project.alloc_id();
                                    bim_project.columns.push(crate::core::BimColumn {
                                        id: cid,
                                        name: format!("Col C-{cid}"),
                                        base_level_id: base_lvl_id,
                                        top_level_id: top_lvl_id,
                                        pos: pt,
                                        width_mm: 400.0,
                                        depth_mm: 400.0,
                                        rotation_deg: 0.0,
                                        is_round: false,
                                        is_wf_profile: false,
                                        profile_name: "RC 400x400".to_string(),
                                    });
                                }
                            }
                        }
                    }
                    CanvasShapeKind::Dimension { start_mm, end_mm } => {
                        let did = bim_project.alloc_id();
                        bim_project.dimensions.push(crate::core::BimDimension {
                            id: did,
                            start: crate::core::P2(start_mm[0], start_mm[1]),
                            end: crate::core::P2(end_mm[0], end_mm[1]),
                            offset_mm: 350.0,
                        });
                    }
                    _ => {}
                }
            }
        }
        if walls_added > 0 {
            bim_project.detect_rooms_on_level(base_lvl_id);
        }
        (walls_added, runs_added, slabs_added)
    }

    pub(super) fn parse_nx_ny_spacing(args: &str, default_n: (usize, usize), default_s: (f64, f64)) -> ((usize, usize), (f64, f64)) {
        let mut toks = args.split_whitespace();
        let (nx, ny) = if let Some(first) = toks.next() {
            if let Some((a, b)) = first.split_once('x') {
                (
                    a.trim().parse::<usize>().unwrap_or(default_n.0),
                    b.trim().parse::<usize>().unwrap_or(default_n.1),
                )
            } else {
                default_n
            }
        } else {
            default_n
        };
        let dx = toks
            .next()
            .and_then(|s| s.trim_end_matches("mm").parse::<f64>().ok())
            .unwrap_or(default_s.0);
        let dy = toks
            .next()
            .and_then(|s| s.trim_end_matches("mm").parse::<f64>().ok())
            .unwrap_or(dx);
        ((nx, ny), (dx, dy))
    }

    /// Executes a typed precision CAD command (`@dx,dy`, `@dist<deg`, `move`, `ro`, `cp`, `o`, `grid`, `array`, `promote`, tools).
    pub fn execute_precision_command(
        &mut self,
        raw: &str,
        active_pillar: NavigationPillar,
        bim_project: &mut BimProject,
    ) -> String {
        let trimmed = raw.trim();
        let lower = trimmed.to_lowercase();
        let anchor = self
            .entities
            .last()
            .and_then(|e| match &e.kind {
                CanvasShapeKind::Line { end_mm, .. } => Some(*end_mm),
                CanvasShapeKind::Rectangle { max_mm, .. } => Some(*max_mm),
                CanvasShapeKind::Circle { center_mm, .. } => Some(*center_mm),
                _ => None,
            })
            .unwrap_or(self.cursor_world_mm);

        if trimmed.starts_with('@') {
            if let Some((dx, dy)) = Self::parse_cad_delta_expr(trimmed) {
                self.push_undo();
                let id = self.next_entity_id;
                self.next_entity_id += 1;
                let end = [anchor[0] + dx, anchor[1] + dy];
                self.entities.push(CanvasEntity {
                    id,
                    pillar_badge: active_pillar.badge().to_string(),
                    pillar_name: active_pillar.clean_name().to_string(),
                    kind: CanvasShapeKind::Line {
                        start_mm: anchor,
                        end_mm: end,
                    },
                    color_rgb: self.active_color_rgb(),
                    stroke_width: self.stroke_width,
                    layer_id: self.visibility_graphics.active_cad_layer_id.clone(),
                });
                self.set_single_selection(Some(id));
                return format!(
                    "Drew relative segment ({:.0}, {:.0}) -> ({:.0}, {:.0}) mm",
                    anchor[0], anchor[1], end[0], end[1]
                );
            }
        }

        if let Some(rest) = lower.strip_prefix("move ") {
            if let Some((dx, dy)) = Self::parse_cad_delta_expr(rest) {
                return if self.translate_selected_or_last(dx, dy) {
                    format!("Moved selection by ({dx:.0}, {dy:.0}) mm")
                } else {
                    "No entity selected or present to move".to_string()
                };
            }
        } else if let Some(rest) = lower.strip_prefix("cp ").or_else(|| lower.strip_prefix("copy ")) {
            if let Some((dx, dy)) = Self::parse_cad_delta_expr(rest) {
                return if self.duplicate_selected_or_last(dx, dy) {
                    format!("Duplicated selection by ({dx:.0}, {dy:.0}) mm")
                } else {
                    "No entity selected or present to duplicate".to_string()
                };
            }
        } else if let Some(rest) = lower.strip_prefix("ro ").or_else(|| lower.strip_prefix("rotate ")) {
            if let Ok(deg) = rest.trim().parse::<f64>() {
                return if self.rotate_selected_or_last(deg) {
                    format!("Rotated selection {deg:.1}°")
                } else {
                    "No entity selected or present to rotate".to_string()
                };
            }
        } else if let Some(rest) = lower.strip_prefix("o ").or_else(|| lower.strip_prefix("offset ")) {
            if let Ok(d) = rest.trim().parse::<f64>() {
                return if self.offset_selected_or_last(d) {
                    format!("Offset duplicate by {d:.0} mm")
                } else {
                    "No entity selected or present to offset".to_string()
                };
            }
        } else if lower == "grid" || lower.starts_with("grid ") {
            let args = lower.strip_prefix("grid").unwrap_or("").trim();
            let ((nx, ny), (dx, dy)) = Self::parse_nx_ny_spacing(args, (4, 3), (6000.0, 5000.0));
            let lines = self.generate_structural_grid(nx, ny, dx, dy, active_pillar, [0.0, 0.0]);
            return format!("Generated {nx}x{ny} structural grid ({lines} axes @ {dx:.0}x{dy:.0} mm)");
        } else if lower == "array" || lower.starts_with("array ") {
            let args = lower.strip_prefix("array").unwrap_or("").trim();
            let ((c, r), (dx, dy)) = Self::parse_nx_ny_spacing(args, (3, 2), (self.cad_step_mm.max(1000.0), self.cad_step_mm.max(1000.0)));
            let added = self.array_selected_or_last(c, r, dx, dy);
            return if added > 0 {
                format!("Arrayed {c}x{r} (+{added} clones @ {dx:.0}x{dy:.0} mm)")
            } else {
                "Select or draw an entity before running ARRAY".to_string()
            };
        }

        match lower.as_str() {
            "v" | "box" | "marquee" => {
                self.active_tool = UniversalCanvasTool::BoxSelect;
                "Active tool: Box Marquee Select (V) — Window L->R | Crossing R->L".to_string()
            }
            "osnap" | "f4" => {
                self.osnap_enabled = !self.osnap_enabled;
                format!("Object Snap (OSNAP F4) = {}", if self.osnap_enabled { "ON" } else { "OFF" })
            }
            "promote" | "tobim" | "bim" => {
                let (w, r, s) = self.promote_selection_to_bim(bim_project);
                if w + r + s > 0 {
                    format!("Promoted CAD selection -> +{w} Walls, +{r} Beams, +{s} Slabs in 3D BIM")
                } else {
                    "Select Lines or Rectangles first to promote into 3D BIM".to_string()
                }
            }
            "l" | "line" => {
                self.active_tool = UniversalCanvasTool::Line;
                "Active tool: Line (L)".to_string()
            }
            "r" | "rect" | "rectangle" => {
                self.active_tool = UniversalCanvasTool::Rectangle;
                "Active tool: Rectangle (R)".to_string()
            }
            "c" | "circle" => {
                self.active_tool = UniversalCanvasTool::Circle;
                "Active tool: Circle (C)".to_string()
            }
            "d" | "dim" | "dimension" => {
                self.active_tool = UniversalCanvasTool::Dimension;
                "Active tool: Dimension (D)".to_string()
            }
            "t" | "text" | "callout" => {
                self.active_tool = UniversalCanvasTool::Callout;
                "Active tool: Callout (T)".to_string()
            }
            "e" | "erase" => {
                self.active_tool = UniversalCanvasTool::Eraser;
                "Active tool: Eraser (E)".to_string()
            }
            "ro" | "rotate" => {
                if self.rotate_selected_or_last(self.cad_rotate_deg) {
                    format!("Rotated selection {:.1}°", self.cad_rotate_deg)
                } else {
                    "No entity to rotate".to_string()
                }
            }
            "cp" | "copy" | "dup" => {
                if self.duplicate_selected_or_last(self.cad_step_mm, 0.0) {
                    format!("Copied selection +{:.0} mm East", self.cad_step_mm)
                } else {
                    "No entity to duplicate".to_string()
                }
            }
            "m" | "move" => {
                if self.translate_selected_or_last(self.cad_step_mm, 0.0) {
                    format!("Moved selection +{:.0} mm East", self.cad_step_mm)
                } else {
                    "No entity to move".to_string()
                }
            }
            "x" | "mirror" => {
                let mx = bim_project
                    .plan_extents_mm()
                    .map_or(0.0, |(a, b)| 0.5 * (a[0] + b[0]));
                if self.mirror_selected_or_last(mx) {
                    format!("Mirrored across X={mx:.0} mm")
                } else {
                    "No entity to mirror".to_string()
                }
            }
            "undo" | "u" => {
                if self.undo() {
                    "Undo applied".to_string()
                } else {
                    "Nothing to undo".to_string()
                }
            }
            "redo" => {
                if self.redo() {
                    "Redo applied".to_string()
                } else {
                    "Nothing to redo".to_string()
                }
            }
            "fit" | "z" => {
                self.fit_world_to_bim_extents(bim_project.plan_extents_mm());
                "Fitted camera to BIM footprint (Z)".to_string()
            }
            "freeze" => {
                self.is_frozen = !self.is_frozen;
                format!("Canvas Frozen = {}", self.is_frozen)
            }
            _ => format!("Unknown CAD command '{}' — try @5000,0 | grid 4x3 6000 | array 3x2 | promote", trimmed),
        }
    }

    /// Deletes the selected entities (or latest entity) with undo support.
    pub fn delete_selected_or_last(&mut self) -> bool {
        let mut indices = self.unlocked_target_entity_indices();
        if indices.is_empty() {
            return false;
        }
        self.push_undo();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        indices.dedup();
        for idx in indices {
            if idx < self.entities.len() {
                self.entities.remove(idx);
            }
        }
        self.set_single_selection(None);
        true
    }

    pub fn clear_entities(&mut self, active_pillar: Option<NavigationPillar>) {
        self.push_undo();
        match active_pillar {
            Some(p) => {
                let badge = p.badge();
                self.entities.retain(|e| e.pillar_badge != badge);
            }
            None => self.entities.clear(),
        }
        self.selected_entity_id = None;
    }

    pub fn count_for_pillar(&self, pillar: NavigationPillar) -> usize {
        let badge = pillar.badge();
        self.entities
            .iter()
            .filter(|e| e.pillar_badge == badge)
            .count()
    }

    /// Builds a single synchronized workspace archive across all 15 modules.
    /// Fits either the active site drawing footprint (if non-empty) or resets to the Global World Map overview.
    pub fn fit_world_or_site(&mut self, has_bim_elements: bool) {
        if self.ground_origin_anchored || !self.entities.is_empty() || has_bim_elements {
            self.world_map.zoom_to_ground_site(
                self.ground_origin_lat_lon.0,
                self.ground_origin_lat_lon.1,
                17.5,
            );
        } else {
            self.world_map.reset_to_global_view();
        }
        self.pan = Vec2::ZERO;
        self.zoom = 1.0;
    }

    /// Auto-fits the World Map camera smoothly around planar BIM extents `[min_mm, max_mm]`.
    pub fn fit_world_to_bim_extents(&mut self, extents_mm: Option<([f64; 2], [f64; 2])>) {
        if let Some((min_mm, max_mm)) = extents_mm {
            let cx = 0.5 * (min_mm[0] + max_mm[0]);
            let cy = 0.5 * (min_mm[1] + max_mm[1]);
            let (lat, lon) = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
                self.ground_origin_lat_lon,
                [cx, cy],
            );
            let span_m = ((max_mm[0] - min_mm[0]).abs().max((max_mm[1] - min_mm[1]).abs())
                / 1000.0)
                .max(6.0);
            let cos_lat = lat.to_radians().cos().abs().max(0.2);
            let desired_res_m_px = (span_m / 420.0).max(0.01);
            let target_zoom = ((156_543.03392 * cos_lat / desired_res_m_px).log2() as f32)
                .clamp(14.0, 19.0);
            self.ground_origin_anchored = true;
            self.world_map.zoom_to_ground_site(lat, lon, target_zoom);
            self.pan = Vec2::ZERO;
            self.zoom = 1.0;
        } else {
            self.fit_world_or_site(false);
        }
    }

    /// Converts a viewport screen position into `(lat_deg, lon_deg, hit_globe)` using exact ray-sphere picking when in 3D Globe mode.
    pub fn screen_to_lat_lon_with_hit(&self, screen: Pos2, rect: Rect) -> (f64, f64, bool) {
        if self.globe_mode {
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
            xform.screen_to_lat_lon(screen)
        } else {
            let (lat, lon) = self.world_map.screen_to_lat_lon(screen, rect);
            (lat, lon, true)
        }
    }

    /// Anchors ground `(0, 0) mm` origin to `screen_pos` on Earth if the workspace is still blank.
    pub fn ensure_ground_origin_at_screen(&mut self, screen_pos: Pos2, rect: Rect) {
        if !self.ground_origin_anchored && self.entities.is_empty() {
            let (lat, lon, hit) = self.screen_to_lat_lon_with_hit(screen_pos, rect);
            if hit {
                self.ground_origin_lat_lon = (lat, lon);
                self.ground_origin_anchored = true;
            }
        }
    }

    pub fn world_origin_and_scale(&self, rect: Rect) -> (Pos2, f32) {
        let origin = self.world_to_screen(rect, [0.0, 0.0]);
        let scale = self.world_map.pixels_per_mm_at_origin(self.ground_origin_lat_lon);
        (origin, scale)
    }

    pub fn world_to_screen(&self, rect: Rect, world_mm: [f64; 2]) -> Pos2 {
        if self.globe_mode {
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
            let (lat, lon) =
                SlippyMapState::world_mm_to_lat_lon(self.ground_origin_lat_lon, world_mm);
            if let Some((pos, _cam_z, visible)) = xform.project_lat_lon_elev(lat, lon, 0.0) {
                if visible {
                    return pos;
                }
            }
            return Pos2::new(-1e6, -1e6);
        }
        self.world_map
            .world_mm_to_screen(self.ground_origin_lat_lon, world_mm, rect)
    }

    pub fn screen_to_world(&self, rect: Rect, screen: Pos2) -> [f64; 2] {
        if self.globe_mode {
            let (lat, lon, _) = self.screen_to_lat_lon_with_hit(screen, rect);
            return SlippyMapState::lat_lon_to_world_mm(self.ground_origin_lat_lon, lat, lon);
        }
        self.world_map
            .screen_to_world_mm(self.ground_origin_lat_lon, screen, rect)
    }

    /// Snaps `raw_mm` using Object Snap (OSNAP: Endpoint, Midpoint, Perpendicular, Intersection) and grid snap fallback.
    pub fn snap_point(&self, raw_mm: [f64; 2]) -> [f64; 2] {
        self.snap_point_with_osnap(
            raw_mm,
            self.pending_start_mm,
            None,
            self.last_osnap_radius_mm.max(350.0),
        )
        .0
    }

    /// Resolves exact CAD OSNAP (`Endpoint`, `Midpoint`, `Perpendicular`, `Intersection`) against entities and optional `BimProject`, falling back to grid snap.
    pub fn snap_point_with_osnap(
        &self,
        raw_mm: [f64; 2],
        anchor_mm: Option<[f64; 2]>,
        bim_project: Option<&BimProject>,
        tol_mm: f64,
    ) -> ([f64; 2], OsnapKind) {
        if self.osnap_enabled && tol_mm > 1.0 {
            let mut endpoints: Vec<[f64; 2]> = Vec::new();
            let mut segments: Vec<([f64; 2], [f64; 2])> = Vec::new();

            for ent in &self.entities {
                match &ent.kind {
                    CanvasShapeKind::Line { start_mm, end_mm }
                    | CanvasShapeKind::Dimension { start_mm, end_mm } => {
                        endpoints.push(*start_mm);
                        endpoints.push(*end_mm);
                        segments.push((*start_mm, *end_mm));
                    }
                    CanvasShapeKind::Rectangle { min_mm, max_mm } => {
                        let c0 = *min_mm;
                        let c1 = [max_mm[0], min_mm[1]];
                        let c2 = *max_mm;
                        let c3 = [min_mm[0], max_mm[1]];
                        endpoints.extend([c0, c1, c2, c3]);
                        segments.extend([(c0, c1), (c1, c2), (c2, c3), (c3, c0)]);
                    }
                    CanvasShapeKind::Circle {
                        center_mm,
                        radius_mm,
                    } => {
                        endpoints.push(*center_mm);
                        endpoints.push([center_mm[0] + radius_mm, center_mm[1]]);
                        endpoints.push([center_mm[0] - radius_mm, center_mm[1]]);
                        endpoints.push([center_mm[0], center_mm[1] + radius_mm]);
                        endpoints.push([center_mm[0], center_mm[1] - radius_mm]);
                    }
                    CanvasShapeKind::Freehand { points_mm } => {
                        if let (Some(&first), Some(&last)) = (points_mm.first(), points_mm.last()) {
                            endpoints.push(first);
                            endpoints.push(last);
                        }
                    }
                    CanvasShapeKind::Callout { pos_mm, .. } => {
                        endpoints.push(*pos_mm);
                    }
                }
            }

            if let Some(bim) = bim_project {
                for w in &bim.walls {
                    let s = [w.start.0, w.start.1];
                    let e = [w.end.0, w.end.1];
                    endpoints.push(s);
                    endpoints.push(e);
                    segments.push((s, e));
                }
                for r in &bim.runs {
                    let s = [r.start.0, r.start.1];
                    let e = [r.end.0, r.end.1];
                    endpoints.push(s);
                    endpoints.push(e);
                    segments.push((s, e));
                }
                for g in &bim.grids {
                    segments.push(([g.start.0, g.start.1], [g.end.0, g.end.1]));
                }
                for c in &bim.columns {
                    endpoints.push([c.pos.0, c.pos.1]);
                }
            }

            let mut best: Option<([f64; 2], OsnapKind, f64)> = None;
            let consider = |best: &mut Option<([f64; 2], OsnapKind, f64)>,
                            pt: [f64; 2],
                            kind: OsnapKind,
                            score_bias: f64| {
                let d = (pt[0] - raw_mm[0]).hypot(pt[1] - raw_mm[1]);
                if d <= tol_mm {
                    let ranked = d * score_bias;
                    if best.is_none_or(|(_, _, bd)| ranked < bd) {
                        *best = Some((pt, kind, ranked));
                    }
                }
            };

            // 1. Endpoint snaps (highest priority right on vertex)
            for &ep in &endpoints {
                consider(&mut best, ep, OsnapKind::Endpoint, 0.90);
            }

            // 2. Segment intersection snaps (cap pairwise checks for responsiveness)
            let seg_cap = segments.len().min(64);
            for i in 0..seg_cap {
                for j in (i + 1)..seg_cap {
                    if let Some(ix) = segment_intersection_2d(segments[i].0, segments[i].1, segments[j].0, segments[j].1) {
                        consider(&mut best, ix, OsnapKind::Intersection, 0.88);
                    }
                }
            }

            // 3. Midpoint & true Perpendicular (from active anchor) snaps
            for &(a, b) in &segments {
                let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
                consider(&mut best, mid, OsnapKind::Midpoint, 0.98);

                if let Some(perp_ref) = anchor_mm {
                    let vx = b[0] - a[0];
                    let vy = b[1] - a[1];
                    let len_sq = vx * vx + vy * vy;
                    if len_sq > 1e-6 {
                        let t = ((perp_ref[0] - a[0]) * vx + (perp_ref[1] - a[1]) * vy) / len_sq;
                        if (0.02..=0.98).contains(&t) {
                            let foot = [a[0] + t * vx, a[1] + t * vy];
                            consider(&mut best, foot, OsnapKind::Perpendicular, 1.04);
                        }
                    }
                }
            }

            if let Some((pt, kind, _)) = best {
                return (pt, kind);
            }
        }

        if !self.snap_to_grid || self.grid_spacing_mm <= 1.0 {
            return (raw_mm, OsnapKind::None);
        }
        let step = self.grid_spacing_mm;
        (
            [
                (raw_mm[0] / step).round() * step,
                (raw_mm[1] / step).round() * step,
            ],
            OsnapKind::Grid,
        )
    }
}
