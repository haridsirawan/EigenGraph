//! views/parametric/dynamo_studio.rs — Visual Node Graph Studio (Dynamo) & 3D Wireframe Preview.

use super::*;

impl ParametricView {
    /// Phase 29: Algorithmic Dynamo / Grasshopper Studio with Live 3D Wireframe Preview & BIM Bake
    pub(super) fn render_dynamo_algorithmic_studio(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        available_width: f32,
        available_height: f32,
    ) {
        // 1. Algorithmic Controls Bar (Preset Selector + 4 Live Geometric Sliders)
        Frame::new()
            .fill(theme.bg_sidebar)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        icons::icon_label(
                            ui,
                            AppIcon::NodesGraph,
                            13.0,
                            theme.accent_blue,
                            "NODE GRAPH BUILDER:",
                            11.0,
                            theme.accent_blue,
                            true,
                        );
                        if icons::icon_button(
                            ui,
                            AppIcon::Plus,
                            "Number Node",
                            10.5,
                            theme.text_primary,
                            theme.bg_card,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(6),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        {
                            let next_id = self.node_graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                            self.node_graph.nodes.push(crate::models::VisualGraphNode {
                                id: next_id,
                                title: format!("Parameter #{}", next_id),
                                category: crate::models::VisualNodeCategory::Input,
                                pos: [60.0 + (next_id as f32 * 40.0) % 240.0, 90.0 + (next_id as f32 * 25.0) % 140.0],
                                inputs: Vec::new(),
                                outputs: vec!["Value".to_string()],
                                value_display: "12.0".to_string(),
                                is_active: true,
                            });
                            self.selected_node_id = Some(next_id);
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::NodesGraph,
                            "Math Node",
                            10.5,
                            theme.text_primary,
                            theme.bg_card,
                            Stroke::new(1.0, theme.border_subtle),
                            CornerRadius::same(6),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        {
                            let next_id = self.node_graph.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                            self.node_graph.nodes.push(crate::models::VisualGraphNode {
                                id: next_id,
                                title: format!("Expression #{}", next_id),
                                category: crate::models::VisualNodeCategory::MathLogic,
                                pos: [140.0 + (next_id as f32 * 40.0) % 240.0, 120.0 + (next_id as f32 * 25.0) % 140.0],
                                inputs: vec!["x".to_string()],
                                outputs: vec!["out".to_string()],
                                value_display: "x * 1.5".to_string(),
                                is_active: true,
                            });
                            self.selected_node_id = Some(next_id);
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Clear Graph",
                            10.5,
                            theme.accent_red,
                            theme.bg_card,
                            Stroke::new(1.0, theme.accent_red.gamma_multiply(0.35)),
                            CornerRadius::same(6),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        {
                            self.node_graph = crate::models::VisualNodeGraph::empty();
                            self.selected_node_id = None;
                            self.graph_bake_status = Some("Node graph cleared to blank canvas.".to_string());
                        }

                        if icons::icon_button(
                            ui,
                            AppIcon::Bolt,
                            "Bake to BIM Scene",
                            10.5,
                            Color32::WHITE,
                            theme.accent_green,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        {
                            self.bake_parametric_to_bim();
                        }
                    });

                    if !self.node_graph.nodes.is_empty() {
                        ui.add_space(4.0);
                        let labels = ["Height / Scale", "Radius / Span", "Twist / Step", "Divisions"];
                        let ranges = [(2.0_f32, 120.0), (2.0, 60.0), (0.0, 90.0), (3.0, 24.0)];
                        ui.horizontal_wrapped(|ui| {
                            for idx in 0..4 {
                                let (lo, hi) = ranges[idx];
                                ui.label(
                                    RichText::new(format!("{}:", labels[idx]))
                                        .size(10.0)
                                        .color(theme.text_secondary),
                                );
                                let slider = if idx == 3 {
                                    egui::Slider::new(&mut self.preset_params[idx], lo..=hi)
                                        .step_by(1.0)
                                        .max_decimals(0)
                                } else {
                                    egui::Slider::new(&mut self.preset_params[idx], lo..=hi)
                                        .max_decimals(1)
                                };
                                ui.add(slider);
                                ui.add_space(6.0);
                            }
                        });
                    }
                });
            });

        ui.add_space(6.0);

        // 2. Responsive Split View: Left/Top Node Graph + Right/Bottom Live 3D Wireframe Preview
        let is_wide = available_width >= 680.0;
        let pane_h = (available_height - 96.0).max(260.0);

        if is_wide {
            let left_w = ((available_width - 8.0) * 0.55).max(260.0);
            let right_w = (available_width - 8.0 - left_w).max(220.0);
            ui.horizontal(|ui| {
                Frame::new()
                    .fill(Color32::from_rgb(18, 20, 26))
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(0))
                    .show(ui, |ui| {
                        let (response, painter) = ui.allocate_painter(
                            Vec2::new(left_w, pane_h),
                            Sense::click_and_drag(),
                        );
                        if response.hovered() && !crate::ui_kit::is_canvas_frozen(ui.ctx()) {
                            let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                            if scroll_y.abs() > 0.0 {
                                self.canvas_zoom =
                                    (self.canvas_zoom * (1.0 + scroll_y * 0.002)).clamp(0.45, 2.5);
                            }
                        }
                        let rect = response.rect;
                        self.last_canvas_rect = Some(rect);
                        self.draw_visual_node_graph(&response, &painter, rect, theme);
                    });

                ui.add_space(6.0);
                self.draw_parametric_3d_preview(ui, theme, right_w, pane_h);
            });
        } else {
            let stacked_h = pane_h.clamp(230.0, 300.0);
            Frame::new()
                .fill(Color32::from_rgb(18, 20, 26))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    let (response, painter) = ui.allocate_painter(
                        Vec2::new(available_width, stacked_h),
                        Sense::click_and_drag(),
                    );
                    if response.hovered() && !crate::ui_kit::is_canvas_frozen(ui.ctx()) {
                        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                        if scroll_y.abs() > 0.0 {
                            self.canvas_zoom =
                                (self.canvas_zoom * (1.0 + scroll_y * 0.002)).clamp(0.45, 2.5);
                        }
                    }
                    let rect = response.rect;
                    self.last_canvas_rect = Some(rect);
                    self.draw_visual_node_graph(&response, &painter, rect, theme);
                });

            ui.add_space(6.0);
            self.draw_parametric_3d_preview(ui, theme, available_width, stacked_h);
        }
    }

    /// Generates procedural 3D wireframe vertices & edges `(a, b, is_diagonal)` and estimated tonnage.
    pub(super) fn generate_parametric_wireframe(&self) -> (Vec<[f32; 3]>, Vec<(usize, usize, bool)>, f32) {
        if self.node_graph.nodes.is_empty() {
            return (Vec::new(), Vec::new(), 0.0);
        }
        let mut verts: Vec<[f32; 3]> = Vec::new();
        let mut edges: Vec<(usize, usize, bool)> = Vec::new();
        let [p0, p1, p2, p3] = self.preset_params;

        let steel_ton = match self.active_preset {
            ParametricGeneratorPreset::DiagridTower => {
                let height = p0.max(10.0);
                let base_r = p1.max(3.0);
                let twist_rad = p2.to_radians();
                let sides = (p3.round() as usize).clamp(4, 24);
                let tiers = 8usize;

                for t in 0..=tiers {
                    let frac = t as f32 / tiers as f32;
                    let y = frac * height - height * 0.5;
                    let taper = 1.0 - 0.16 * frac + 0.05 * (frac * std::f32::consts::PI).sin();
                    let r = base_r * taper;
                    let theta_off = twist_rad * frac;
                    for s in 0..sides {
                        let ang = (s as f32 / sides as f32) * std::f32::consts::TAU + theta_off;
                        verts.push([r * ang.cos(), y, r * ang.sin()]);
                    }
                }

                for t in 0..=tiers {
                    let row = t * sides;
                    for s in 0..sides {
                        let next_s = (s + 1) % sides;
                        edges.push((row + s, row + next_s, false));
                        if t < tiers {
                            let up_row = (t + 1) * sides;
                            edges.push((row + s, up_row + s, false));
                            edges.push((row + s, up_row + next_s, true));
                            edges.push((row + next_s, up_row + s, true));
                        }
                    }
                }
                (p0 * p1 * 0.42) + (p2 * 0.15)
            }
            ParametricGeneratorPreset::GeodesicDome => {
                let radius = (p0 * 0.5).max(4.0);
                let rise = p1.max(2.0);
                let rings = (p2.round() as usize).clamp(2, 10);
                let sectors = (p3.round() as usize).clamp(6, 28);

                for r_idx in 0..rings {
                    let frac = r_idx as f32 / rings as f32;
                    let phi = frac * (std::f32::consts::FRAC_PI_2 * 0.92);
                    let r_ring = radius * phi.cos();
                    let y = rise * phi.sin() - rise * 0.35;
                    let stagger = if r_idx % 2 == 1 {
                        std::f32::consts::PI / sectors as f32
                    } else {
                        0.0
                    };
                    for s in 0..sectors {
                        let ang = (s as f32 / sectors as f32) * std::f32::consts::TAU + stagger;
                        verts.push([r_ring * ang.cos(), y, r_ring * ang.sin()]);
                    }
                }
                let apex_idx = verts.len();
                verts.push([0.0, rise - rise * 0.35, 0.0]);

                for r_idx in 0..rings {
                    let row = r_idx * sectors;
                    for s in 0..sectors {
                        let next_s = (s + 1) % sectors;
                        edges.push((row + s, row + next_s, false));
                        if r_idx + 1 < rings {
                            let up = (r_idx + 1) * sectors;
                            edges.push((row + s, up + s, false));
                            edges.push((row + s, up + next_s, true));
                        } else {
                            edges.push((row + s, apex_idx, true));
                        }
                    }
                }
                (p0 * p1 * 0.18) + (sectors as f32 * 0.9)
            }
            ParametricGeneratorPreset::VoronoiCanopy => {
                let span_x = p0.max(8.0);
                let span_z = p1.max(6.0);
                let camber = p2.max(0.8);
                let div = (p3.round() as usize).clamp(4, 14);
                let stride = div + 1;

                for ix in 0..=div {
                    let u = (ix as f32 / div as f32) * 2.0 - 1.0;
                    for iz in 0..=div {
                        let v = (iz as f32 / div as f32) * 2.0 - 1.0;
                        let r2 = (u * u + v * v).min(2.0);
                        let y = camber * (0.65 - 0.55 * r2 + 0.35 * (u * 2.2).cos() * (v * 2.2).cos());
                        verts.push([u * span_x * 0.5, y, v * span_z * 0.5]);
                    }
                }

                for ix in 0..=div {
                    for iz in 0..=div {
                        let idx = ix * stride + iz;
                        if ix < div {
                            edges.push((idx, (ix + 1) * stride + iz, false));
                        }
                        if iz < div {
                            edges.push((idx, ix * stride + (iz + 1), false));
                        }
                        if ix < div && iz < div {
                            if (ix + iz) % 2 == 0 {
                                edges.push((idx, (ix + 1) * stride + (iz + 1), true));
                            } else {
                                edges.push(((ix + 1) * stride + iz, ix * stride + (iz + 1), true));
                            }
                        }
                    }
                }

                // Four tapered tree support columns down to slab datum
                let foot_y = -camber * 1.15 - 2.0;
                let anchors = [
                    (div / 4, div / 4),
                    ((3 * div) / 4, div / 4),
                    (div / 4, (3 * div) / 4),
                    ((3 * div) / 4, (3 * div) / 4),
                ];
                for (ax, az) in anchors {
                    let top_idx = ax * stride + az;
                    let top_p = verts[top_idx];
                    let base_idx = verts.len();
                    verts.push([top_p[0] * 0.72, foot_y, top_p[2] * 0.72]);
                    edges.push((base_idx, top_idx, false));
                    for (dx, dz) in [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)] {
                        let nx = (ax as isize + dx).clamp(0, div as isize) as usize;
                        let nz = (az as isize + dz).clamp(0, div as isize) as usize;
                        edges.push((base_idx, nx * stride + nz, true));
                    }
                }

                (p0 * p1 * 0.09) + (p2 * 2.4)
            }
        };

        (verts, edges, steel_ton)
    }

    pub(super) fn draw_parametric_3d_preview(&mut self, ui: &mut Ui, theme: &MacTheme, width: f32, height: f32) {
        let (verts, edges, steel_ton) = self.generate_parametric_wireframe();
        let vert_count = verts.len();
        let edge_count = edges.len();

        Frame::new()
            .fill(Color32::from_rgb(13, 16, 23))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(0))
            .show(ui, |ui| {
                ui.set_width(width);
                ui.vertical(|ui| {
                    // Header Strip
                    Frame::new()
                        .fill(theme.bg_sidebar)
                        .inner_margin(Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                icons::icon_label(
                                    ui,
                                    AppIcon::Cube3D,
                                    13.0,
                                    theme.accent_blue,
                                    "LIVE 3D PARAMETRIC MESH",
                                    10.5,
                                    theme.accent_blue,
                                    true,
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} Nodes • {} Struts • {:.1} t",
                                        vert_count, edge_count, steel_ton
                                    ))
                                    .size(9.5)
                                    .monospace()
                                    .color(theme.accent_green),
                                );
                            });
                        });

                    let canvas_h = (height - 34.0).max(190.0);
                    let (response, painter_raw) = ui.allocate_painter(
                        Vec2::new((width - 2.0).max(180.0), canvas_h),
                        Sense::click_and_drag(),
                    );
                    let rect = response.rect;
                    let painter = painter_raw.with_clip_rect(rect);

                    if response.dragged() {
                        self.preview_yaw = (self.preview_yaw + response.drag_delta().x * 0.01)
                            .rem_euclid(std::f32::consts::TAU);
                        self.preview_pitch = (self.preview_pitch - response.drag_delta().y * 0.01)
                            .clamp(-1.35, 1.45);
                    }

                    painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(12, 15, 22));

                    let max_coord = verts
                        .iter()
                        .flat_map(|v| v.iter().map(|c| c.abs()))
                        .fold(1.0f32, f32::max);
                    let scale = (rect.width().min(rect.height()) * 0.42) / max_coord;
                    let center = rect.center() + Vec2::new(0.0, 10.0);
                    let cos_y = self.preview_yaw.cos();
                    let sin_y = self.preview_yaw.sin();
                    let cos_p = self.preview_pitch.cos();
                    let sin_p = self.preview_pitch.sin();

                    let project_v = |v: [f32; 3]| -> (Pos2, f32) {
                        let x1 = v[0] * cos_y + v[2] * sin_y;
                        let z1 = -v[0] * sin_y + v[2] * cos_y;
                        let y2 = v[1] * cos_p - z1 * sin_p;
                        let z2 = v[1] * sin_p + z1 * cos_p;
                        let persp = (95.0 / (95.0 + z2)).clamp(0.45, 2.2);
                        let px = center.x + x1 * scale * persp;
                        let py = center.y - y2 * scale * persp;
                        (Pos2::new(px, py), z2)
                    };

                    let projected: Vec<(Pos2, f32)> = verts.iter().copied().map(project_v).collect();

                    // Base datum grid crosshairs
                    let g_ext = max_coord * 0.85;
                    let g_y = verts.iter().map(|v| v[1]).fold(f32::MAX, f32::min);
                    for i in -2..=2 {
                        let off = (i as f32 / 2.0) * g_ext;
                        let (a, _) = project_v([off, g_y, -g_ext]);
                        let (b, _) = project_v([off, g_y, g_ext]);
                        let (c, _) = project_v([-g_ext, g_y, off]);
                        let (d, _) = project_v([g_ext, g_y, off]);
                        let g_col = Color32::from_rgba_premultiplied(70, 90, 130, 45);
                        painter.line_segment([a, b], Stroke::new(0.8, g_col));
                        painter.line_segment([c, d], Stroke::new(0.8, g_col));
                    }

                    // Render wireframe struts
                    for &(ia, ib, is_diag) in &edges {
                        if let (Some(&(pa, _)), Some(&(pb, _))) =
                            (projected.get(ia), projected.get(ib))
                        {
                            let col = if is_diag {
                                Color32::from_rgba_premultiplied(52, 211, 153, 185)
                            } else {
                                Color32::from_rgba_premultiplied(56, 189, 248, 225)
                            };
                            painter.line_segment(
                                [pa, pb],
                                Stroke::new(if is_diag { 1.1 } else { 1.5 }, col),
                            );
                        }
                    }

                    for &(p, _) in &projected {
                        painter.circle_filled(p, 1.8, Color32::from_rgb(226, 232, 240));
                    }

                    painter.text(
                        rect.left_bottom() + Vec2::new(10.0, -8.0),
                        Align2::LEFT_BOTTOM,
                        format!(
                            "Drag to Orbit • Yaw {:.0}° Pitch {:.0}°",
                            self.preview_yaw.to_degrees(),
                            self.preview_pitch.to_degrees()
                        ),
                        egui::FontId::monospace(9.0),
                        Color32::from_rgb(130, 150, 185),
                    );
                });
            });
    }

    pub(super) fn bake_parametric_to_bim(&mut self) {
        let base_lvl_id = self
            .bim_bus
            .project
            .levels
            .first()
            .map(|l| l.id)
            .unwrap_or(1);
        let top_lvl_id = self
            .bim_bus
            .project
            .levels
            .get(1)
            .map(|l| l.id)
            .unwrap_or(base_lvl_id);

        let center = P2(6000.0, 4500.0);
        let mut baked_cols = 0usize;
        let mut baked_beams = 0usize;

        match self.active_preset {
            ParametricGeneratorPreset::DiagridTower => {
                let r_mm = (self.preset_params[1] * 260.0).clamp(1600.0, 3800.0) as f64;
                let sides = (self.preset_params[3].round() as usize).clamp(4, 10);
                let mut ring_pts = Vec::with_capacity(sides);
                for s in 0..sides {
                    let ang = (s as f64 / sides as f64) * std::f64::consts::TAU;
                    let pt = P2(center.0 + r_mm * ang.cos(), center.1 + r_mm * ang.sin());
                    ring_pts.push(pt);
                    if self
                        .bim_bus
                        .execute(BimCommand::AddColumn {
                            base_level_id: base_lvl_id,
                            top_level_id: top_lvl_id,
                            pos: pt,
                            width_mm: 350.0,
                            depth_mm: 350.0,
                            is_round: true,
                            profile_name: "CHS 355x16 Diagrid".to_string(),
                        })
                        .is_ok()
                    {
                        baked_cols += 1;
                    }
                }
                for s in 0..sides {
                    let a = ring_pts[s];
                    let b = ring_pts[(s + 1) % sides];
                    if self
                        .bim_bus
                        .execute(BimCommand::AddBeamRun {
                            level_id: top_lvl_id,
                            category: RunCategory::StructuralBeam,
                            start: a,
                            end: b,
                            width_mm: 200.0,
                            depth_mm: 380.0,
                            justify: RunJustify::Top,
                            profile_name: "WF 350x175 Diagrid Ring".to_string(),
                        })
                        .is_ok()
                    {
                        baked_beams += 1;
                    }
                }
            }
            ParametricGeneratorPreset::GeodesicDome => {
                let r_mm = (self.preset_params[0] * 140.0).clamp(1800.0, 4200.0) as f64;
                let sectors = (self.preset_params[3].round() as usize).clamp(6, 12);
                let mut outer_pts = Vec::with_capacity(sectors);
                for s in 0..sectors {
                    let ang = (s as f64 / sectors as f64) * std::f64::consts::TAU;
                    let pt = P2(center.0 + r_mm * ang.cos(), center.1 + r_mm * ang.sin());
                    outer_pts.push(pt);
                    if self
                        .bim_bus
                        .execute(BimCommand::AddColumn {
                            base_level_id: base_lvl_id,
                            top_level_id: top_lvl_id,
                            pos: pt,
                            width_mm: 300.0,
                            depth_mm: 300.0,
                            is_round: true,
                            profile_name: "CHS 300 Dome Pier".to_string(),
                        })
                        .is_ok()
                    {
                        baked_cols += 1;
                    }
                }
                for s in 0..sectors {
                    if self
                        .bim_bus
                        .execute(BimCommand::AddBeamRun {
                            level_id: top_lvl_id,
                            category: RunCategory::StructuralBeam,
                            start: outer_pts[s],
                            end: center,
                            width_mm: 180.0,
                            depth_mm: 300.0,
                            justify: RunJustify::Top,
                            profile_name: "HSS 200x200 Meridian Rib".to_string(),
                        })
                        .is_ok()
                    {
                        baked_beams += 1;
                    }
                }
            }
            ParametricGeneratorPreset::VoronoiCanopy => {
                let hx = (self.preset_params[0] * 160.0).clamp(1800.0, 4500.0) as f64;
                let hz = (self.preset_params[1] * 160.0).clamp(1500.0, 3600.0) as f64;
                let posts = [
                    P2(center.0 - hx, center.1 - hz),
                    P2(center.0 + hx, center.1 - hz),
                    P2(center.0 + hx, center.1 + hz),
                    P2(center.0 - hx, center.1 + hz),
                ];
                for p in posts {
                    if self
                        .bim_bus
                        .execute(BimCommand::AddColumn {
                            base_level_id: base_lvl_id,
                            top_level_id: top_lvl_id,
                            pos: p,
                            width_mm: 320.0,
                            depth_mm: 320.0,
                            is_round: true,
                            profile_name: "Tree Column CHS".to_string(),
                        })
                        .is_ok()
                    {
                        baked_cols += 1;
                    }
                }
                let ribs = [
                    (posts[0], posts[2]),
                    (posts[1], posts[3]),
                    (posts[0], posts[1]),
                    (posts[1], posts[2]),
                    (posts[2], posts[3]),
                    (posts[3], posts[0]),
                ];
                for (a, b) in ribs {
                    if self
                        .bim_bus
                        .execute(BimCommand::AddBeamRun {
                            level_id: top_lvl_id,
                            category: RunCategory::StructuralBeam,
                            start: a,
                            end: b,
                            width_mm: 200.0,
                            depth_mm: 350.0,
                            justify: RunJustify::Top,
                            profile_name: "Voronoi Diagrid Rib".to_string(),
                        })
                        .is_ok()
                    {
                        baked_beams += 1;
                    }
                }
            }
        }

        let (_, edges, ton) = self.generate_parametric_wireframe();
        self.refresh_live_eigengate();
        let msg = format!(
            "Baked {} ({} mesh struts / {:.1} t): added {} Columns & {} Beams to BIM Project (UNDO ready).",
            self.active_preset.label(),
            edges.len(),
            ton,
            baked_cols,
            baked_beams
        );
        self.command_history.push(msg.clone());
        self.graph_bake_status = Some(msg.clone());
        self.save_notification = Some(msg);
    }

    /// Phase 27 / 29: Renders the interactive Dynamo / Grasshopper style Visual Computational Node Graph Canvas
    pub(super) fn draw_visual_node_graph(
        &mut self,
        response: &egui::Response,
        painter_raw: &egui::Painter,
        rect: Rect,
        theme: &MacTheme,
    ) {
        let painter = painter_raw.with_clip_rect(rect);
        let node_w = 198.0 * self.canvas_zoom;
        let node_h = 92.0 * self.canvas_zoom;

        let hud_w = (rect.width() - 24.0).clamp(200.0, 310.0);
        let hud_rect = Rect::from_min_size(rect.min + Vec2::new(12.0, 12.0), Vec2::new(hud_w, 68.0));
        let run_btn_rect = Rect::from_min_size(
            hud_rect.min + Vec2::new(10.0, 40.0),
            Vec2::new(((hud_w - 28.0) * 0.54).max(85.0), 22.0),
        );
        let add_btn_rect = Rect::from_min_size(
            Pos2::new(run_btn_rect.max.x + 8.0, hud_rect.min.y + 40.0),
            Vec2::new(((hud_w - 28.0) * 0.44).max(75.0), 22.0),
        );

        // Node Dragging vs Canvas Panning
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                if !hud_rect.contains(pos) {
                    let hit = self.node_graph.nodes.iter().rev().find_map(|node| {
                        let n_pos = rect.min
                            + Vec2::new(node.pos[0], node.pos[1]) * self.canvas_zoom
                            + self.canvas_pan;
                        let n_rect = Rect::from_min_size(n_pos, Vec2::new(node_w, node_h));
                        if n_rect.contains(pos) {
                            Some(node.id)
                        } else {
                            None
                        }
                    });
                    if let Some(nid) = hit {
                        self.dragging_node_id = Some(nid);
                        self.selected_node_id = Some(nid);
                    }
                }
            }
        }

        if response.dragged() {
            let pan_mode = response.dragged_by(egui::PointerButton::Secondary);
            if !pan_mode {
                if let Some(drag_id) = self.dragging_node_id {
                    let delta = response.drag_delta() / self.canvas_zoom.max(0.2);
                    if let Some(node) = self.node_graph.nodes.iter_mut().find(|n| n.id == drag_id) {
                        node.pos[0] += delta.x;
                        node.pos[1] += delta.y;
                    }
                } else {
                    self.canvas_pan += response.drag_delta();
                }
            } else {
                self.canvas_pan += response.drag_delta();
            }
        }
        if response.drag_stopped() {
            self.dragging_node_id = None;
        }

        // Dark Slate Infinite Node Graph Background
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(18, 20, 26));

        // Subtle Grid Dot Lattice
        let grid_step = (28.0 * self.canvas_zoom).clamp(14.0, 56.0);
        let start_x = rect.min.x + (self.canvas_pan.x % grid_step);
        let start_y = rect.min.y + (self.canvas_pan.y % grid_step);
        let mut x = start_x;
        while x < rect.max.x {
            let mut y = start_y;
            while y < rect.max.y {
                painter.circle_filled(
                    Pos2::new(x, y),
                    1.0,
                    Color32::from_rgba_premultiplied(255, 255, 255, 22),
                );
                y += grid_step;
            }
            x += grid_step;
        }

        // Render Connecting Wires (Cubic Bezier Curves)
        for wire in &self.node_graph.wires {
            let n1_opt = self.node_graph.nodes.iter().find(|n| n.id == wire.from_node);
            let n2_opt = self.node_graph.nodes.iter().find(|n| n.id == wire.to_node);
            if let (Some(n1), Some(n2)) = (n1_opt, n2_opt) {
                let p1_base = rect.min
                    + Vec2::new(n1.pos[0], n1.pos[1]) * self.canvas_zoom
                    + self.canvas_pan;
                let p2_base = rect.min
                    + Vec2::new(n2.pos[0], n2.pos[1]) * self.canvas_zoom
                    + self.canvas_pan;

                let p1 = Pos2::new(
                    p1_base.x + node_w,
                    p1_base.y + (46.0 + wire.from_socket as f32 * 20.0) * self.canvas_zoom,
                );
                let p2 = Pos2::new(
                    p2_base.x,
                    p2_base.y + (46.0 + wire.to_socket as f32 * 20.0) * self.canvas_zoom,
                );

                let dx = (p2.x - p1.x).abs().max(40.0) * 0.55;
                let cp1 = p1 + Vec2::new(dx, 0.0);
                let cp2 = p2 - Vec2::new(dx, 0.0);

                let steps = 16;
                let mut prev = p1;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let u = 1.0 - t;
                    let pt = Pos2::new(
                        u * u * u * p1.x
                            + 3.0 * u * u * t * cp1.x
                            + 3.0 * u * t * t * cp2.x
                            + t * t * t * p2.x,
                        u * u * u * p1.y
                            + 3.0 * u * u * t * cp1.y
                            + 3.0 * u * t * t * cp2.y
                            + t * t * t * p2.y,
                    );
                    painter.line_segment(
                        [prev, pt],
                        Stroke::new(2.1 * self.canvas_zoom, Color32::from_rgb(0, 150, 255)),
                    );
                    prev = pt;
                }

                let mid_t = 0.5;
                let u = 1.0 - mid_t;
                let mid_pt = Pos2::new(
                    u * u * u * p1.x
                        + 3.0 * u * u * mid_t * cp1.x
                        + 3.0 * u * mid_t * mid_t * cp2.x
                        + mid_t * mid_t * mid_t * p2.x,
                    u * u * u * p1.y
                        + 3.0 * u * u * mid_t * cp1.y
                        + 3.0 * u * mid_t * mid_t * cp2.y
                        + mid_t * mid_t * mid_t * p2.y,
                );
                painter.circle_filled(
                    mid_pt,
                    4.0 * self.canvas_zoom,
                    Color32::from_rgb(52, 199, 89),
                );
                painter.circle_stroke(
                    mid_pt,
                    4.0 * self.canvas_zoom,
                    Stroke::new(1.0, Color32::WHITE),
                );
            }
        }

        // Render Graph Nodes
        for node in &self.node_graph.nodes {
            let n_pos = rect.min
                + Vec2::new(node.pos[0], node.pos[1]) * self.canvas_zoom
                + self.canvas_pan;
            let node_rect = Rect::from_min_size(n_pos, Vec2::new(node_w, node_h));

            let (_cat_label, (cr, cg, cb)) = node.category.badge_color_and_name();
            let cat_color = Color32::from_rgb(cr, cg, cb);

            painter.rect_filled(
                node_rect.translate(Vec2::new(3.0, 4.0)),
                CornerRadius::same(8),
                Color32::from_rgba_premultiplied(0, 0, 0, 80),
            );
            painter.rect_filled(node_rect, CornerRadius::same(8), Color32::from_rgb(26, 30, 38));
            let is_sel = self.selected_node_id == Some(node.id);
            let border_stroke = if is_sel {
                Stroke::new(2.0, theme.accent_blue)
            } else {
                Stroke::new(1.0, Color32::from_rgb(50, 56, 70))
            };
            painter.rect_stroke(node_rect, CornerRadius::same(8), border_stroke, StrokeKind::Inside);

            let header_rect = Rect::from_min_size(n_pos, Vec2::new(node_w, 23.0 * self.canvas_zoom));
            painter.rect_filled(header_rect, CornerRadius::same(6), Color32::from_rgb(34, 38, 48));
            painter.line_segment(
                [
                    Pos2::new(header_rect.min.x, header_rect.max.y),
                    Pos2::new(header_rect.max.x, header_rect.max.y),
                ],
                Stroke::new(1.0, Color32::from_rgb(45, 52, 65)),
            );

            painter.circle_filled(
                header_rect.min + Vec2::new(10.0 * self.canvas_zoom, 11.5 * self.canvas_zoom),
                3.5 * self.canvas_zoom,
                cat_color,
            );
            painter.text(
                header_rect.min + Vec2::new(18.0 * self.canvas_zoom, 5.5 * self.canvas_zoom),
                Align2::LEFT_TOP,
                &node.title,
                egui::FontId::proportional(9.5 * self.canvas_zoom),
                Color32::WHITE,
            );

            for (idx, in_name) in node.inputs.iter().enumerate() {
                let sock_pt = Pos2::new(
                    node_rect.min.x,
                    node_rect.min.y + (46.0 + idx as f32 * 20.0) * self.canvas_zoom,
                );
                painter.circle_filled(
                    sock_pt,
                    4.2 * self.canvas_zoom,
                    Color32::from_rgb(180, 190, 210),
                );
                painter.circle_stroke(sock_pt, 4.2 * self.canvas_zoom, Stroke::new(1.0, Color32::BLACK));
                painter.text(
                    sock_pt + Vec2::new(8.0 * self.canvas_zoom, 0.0),
                    Align2::LEFT_CENTER,
                    in_name,
                    egui::FontId::proportional(8.2 * self.canvas_zoom),
                    Color32::from_rgb(180, 190, 205),
                );
            }

            for (idx, out_name) in node.outputs.iter().enumerate() {
                let sock_pt = Pos2::new(
                    node_rect.max.x,
                    node_rect.min.y + (46.0 + idx as f32 * 20.0) * self.canvas_zoom,
                );
                painter.circle_filled(sock_pt, 4.2 * self.canvas_zoom, Color32::from_rgb(0, 160, 255));
                painter.circle_stroke(sock_pt, 4.2 * self.canvas_zoom, Stroke::new(1.0, Color32::BLACK));
                painter.text(
                    sock_pt - Vec2::new(8.0 * self.canvas_zoom, 0.0),
                    Align2::RIGHT_CENTER,
                    out_name,
                    egui::FontId::proportional(8.2 * self.canvas_zoom),
                    Color32::from_rgb(140, 200, 255),
                );
            }

            let pill_center =
                Pos2::new(node_rect.center().x, node_rect.min.y + 70.0 * self.canvas_zoom);
            let pill_rect = Rect::from_center_size(
                pill_center,
                Vec2::new(118.0 * self.canvas_zoom, 17.0 * self.canvas_zoom),
            );
            painter.rect_filled(pill_rect, CornerRadius::same(8), Color32::from_rgb(16, 18, 22));
            painter.rect_stroke(
                pill_rect,
                CornerRadius::same(8),
                Stroke::new(0.8, Color32::from_rgb(45, 50, 62)),
                StrokeKind::Inside,
            );
            painter.text(
                pill_center,
                Align2::CENTER_CENTER,
                &node.value_display,
                egui::FontId::monospace(8.2 * self.canvas_zoom),
                Color32::from_rgb(52, 199, 89),
            );
        }

        // Floating Top-Left HUD Toolbar
        painter.rect_filled(
            hud_rect,
            CornerRadius::same(8),
            Color32::from_rgba_premultiplied(20, 24, 30, 235),
        );
        painter.rect_stroke(
            hud_rect,
            CornerRadius::same(8),
            Stroke::new(1.0, Color32::from_rgb(50, 56, 70)),
            StrokeKind::Inside,
        );
        painter.text(
            hud_rect.min + Vec2::new(10.0, 8.0),
            Align2::LEFT_TOP,
            format!(
                "DYNAMO STUDIO • {} NODES (DRAG NODES OR PAN)",
                self.node_graph.nodes.len()
            ),
            egui::FontId::proportional(9.5),
            Color32::WHITE,
        );
        painter.text(
            hud_rect.min + Vec2::new(10.0, 23.0),
            Align2::LEFT_TOP,
            format!("Active Graph: {}", self.active_preset.label()),
            egui::FontId::proportional(8.5),
            Color32::from_rgb(160, 170, 185),
        );

        painter.rect_filled(run_btn_rect, CornerRadius::same(4), Color32::from_rgb(0, 122, 255));
        painter.text(
            run_btn_rect.center(),
            Align2::CENTER_CENTER,
            "Run & Bake",
            egui::FontId::proportional(9.0),
            Color32::WHITE,
        );

        painter.rect_filled(add_btn_rect, CornerRadius::same(4), Color32::from_rgb(45, 52, 65));
        painter.text(
            add_btn_rect.center(),
            Align2::CENTER_CENTER,
            "+ Add Node",
            egui::FontId::proportional(9.0),
            Color32::WHITE,
        );

        // Click Handler for HUD Buttons & Node Selection
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if run_btn_rect.contains(pos) {
                    self.bake_parametric_to_bim();
                } else if add_btn_rect.contains(pos) {
                    let next_id = self.node_graph.nodes.len() + 1;
                    self.node_graph.nodes.push(crate::models::VisualGraphNode {
                        id: next_id,
                        title: format!("Math Node #{}", next_id),
                        category: crate::models::VisualNodeCategory::MathLogic,
                        pos: [280.0 + ((next_id % 3) as f32) * 35.0, 210.0],
                        inputs: vec!["Param A".to_string(), "Param B".to_string()],
                        outputs: vec!["Result".to_string()],
                        value_display: "Evaluated".to_string(),
                        is_active: true,
                    });
                    self.selected_node_id = Some(next_id);
                    self.graph_bake_status =
                        Some(format!("Added Math Node #{} to visual programming graph.", next_id));
                } else {
                    let hit = self.node_graph.nodes.iter().rev().find_map(|node| {
                        let n_pos = rect.min
                            + Vec2::new(node.pos[0], node.pos[1]) * self.canvas_zoom
                            + self.canvas_pan;
                        let n_rect = Rect::from_min_size(n_pos, Vec2::new(node_w, node_h));
                        if n_rect.contains(pos) {
                            Some(node.id)
                        } else {
                            None
                        }
                    });
                    self.selected_node_id = hit;
                }
            }
        }

        // Notification Toast
        if let Some(ref status_msg) = self.graph_bake_status {
            let toast_rect = Rect::from_min_size(
                rect.min + Vec2::new(12.0, rect.height() - 36.0),
                Vec2::new((rect.width() - 24.0).max(160.0), 26.0),
            );
            painter.rect_filled(
                toast_rect,
                CornerRadius::same(6),
                Color32::from_rgba_premultiplied(16, 40, 25, 235),
            );
            painter.rect_stroke(
                toast_rect,
                CornerRadius::same(6),
                Stroke::new(1.0, Color32::from_rgb(40, 160, 80)),
                StrokeKind::Inside,
            );
            painter.text(
                toast_rect.center(),
                Align2::CENTER_CENTER,
                status_msg,
                egui::FontId::proportional(9.0),
                Color32::from_rgb(120, 240, 160),
            );
        }
    }


}
