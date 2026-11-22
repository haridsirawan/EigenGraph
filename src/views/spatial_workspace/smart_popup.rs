//! Embedded Smart Element Inspector, KPI strips, 2D RC/SBE/Footing/Slab previews, and colorbar legend.

use super::*;
use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, Rect, RichText, Stroke, Ui, Vec2};

use crate::core::{
    compute_site_environmental_coupling, BimCommand, BimElementKind, BimProject,
};
use crate::icons::{self, AppIcon};
use crate::models::{DatabaseComponent, NavigationPillar, ProjectItem, StructuralMember};
use crate::theme::MacTheme;
use crate::views::{AiAssistantView, BuildView, UniversalCanvasState};

impl SpatialWorkspaceState {
    /// Renders the Smart Element Inspector directly INSIDE the unified Dynamic Island (zero detached pop-up windows).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_embedded_smart_inspector_in_island(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        target: SmartSelectionTarget,
        cap_w: f32,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();

        // Sync editable draft inputs lazily when selection changes
        if self.last_loaded_target != Some(target) {
            self.sync_edit_draft_from_target(target, &build_view.bim_bus.project);
            if let SmartSelectionTarget::SketchEntity(eid) = target {
                if let Some((_, _, _, _, _, col, sw)) = universal_canvas.entity_summary(eid) {
                    self.sketch_edit_color = col;
                    self.sketch_edit_width = sw;
                }
            }
            self.last_loaded_target = Some(target);
        }

        let mut close_popup = false;
        let iw = (cap_w - 32.0).max(160.0);
        Frame::new()
            .fill(theme.bg_card.gamma_multiply(0.45))
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.35)))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(iw);
                ui.set_max_width(iw);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                // Header bar with element identity, Pin toggle, and Close button
                let (title, subtitle, elem_icon) =
                    Self::describe_target(target, &build_view.bim_bus.project);
                ui.horizontal(|ui| {
                    icons::icon_label(
                        ui,
                        elem_icon,
                        15.0,
                        theme.accent_blue,
                        &title,
                        12.5,
                        theme.text_primary,
                        true,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_only_button(
                            ui,
                            AppIcon::Close,
                            theme.text_secondary,
                            Color32::TRANSPARENT,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::splat(22.0),
                            "Close inspector (Esc)",
                        )
                        .clicked()
                        {
                            close_popup = true;
                        }
                        let pin_bg = if self.popup_pinned {
                            theme.accent_blue.gamma_multiply(0.22)
                        } else {
                            Color32::TRANSPARENT
                        };
                        if icons::icon_button(
                            ui,
                            AppIcon::MapPin,
                            if self.popup_pinned { "Pinned" } else { "Pin" },
                            10.0,
                            if self.popup_pinned {
                                theme.accent_blue
                            } else {
                                theme.text_muted
                            },
                            pin_bg,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::new(0.0, 22.0),
                        )
                        .on_hover_text("Pin inspector open inside Dynamic Island across canvas clicks")
                        .clicked()
                        {
                            self.popup_pinned = !self.popup_pinned;
                        }
                    });
                });
                ui.label(RichText::new(subtitle).size(10.5).color(theme.text_secondary));
                ui.add_space(5.0);

                // Context Tab Switcher (Properties | Calc | 5D/6D | Ask AI)
                if !matches!(
                    target,
                    SmartSelectionTarget::EarthSite { .. } | SmartSelectionTarget::SketchEntity(_)
                ) {
                    ui.horizontal_wrapped(|ui| {
                        for tab in [
                            SmartPopupTab::Properties,
                            SmartPopupTab::Calculations,
                            SmartPopupTab::Lifecycle5D6D,
                            SmartPopupTab::AskAi,
                        ] {
                            let active = self.popup_tab == tab;
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(tab.label()).size(10.5).color(if active {
                                            Color32::WHITE
                                        } else {
                                            theme.text_secondary
                                        }),
                                    )
                                    .fill(if active {
                                        theme.accent_blue
                                    } else {
                                        theme.bg_card
                                    })
                                    .corner_radius(CornerRadius::same(8))
                                    .min_size(Vec2::new(56.0, 22.0)),
                                )
                                .clicked()
                            {
                                self.popup_tab = tab;
                            }
                        }
                    });
                    ui.add_space(5.0);
                }

                ui.separator();
                ui.add_space(4.0);

                let body_actions = self.render_smart_popup_body(
                    ui,
                    theme,
                    target,
                    universal_canvas,
                    build_view,
                    ai_assistant,
                    members,
                    projects,
                    db_components,
                );
                actions.merge(body_actions);
            });

        if close_popup {
            self.selected_target = None;
            self.popup_pinned = false;
            universal_canvas.highlighted_bim_element = None;
            universal_canvas.last_picked_earth_lat_lon = None;
        }

        actions
    }

    pub(super) fn sync_edit_draft_from_target(&mut self, target: SmartSelectionTarget, project: &BimProject) {
        match target {
            SmartSelectionTarget::Column(id) => {
                if let Some(c) = project.columns.iter().find(|c| c.id == id) {
                    self.edit_width_mm = c.width_mm;
                    self.edit_depth_mm = c.depth_mm;
                    self.edit_rot_deg = c.rotation_deg;
                    self.edit_profile_name = c.profile_name.clone();
                }
            }
            SmartSelectionTarget::Beam(id) => {
                if let Some(r) = project.runs.iter().find(|r| r.id == id) {
                    self.edit_width_mm = r.width_mm;
                    self.edit_depth_mm = r.depth_mm;
                    self.edit_profile_name = r.profile_name.clone();
                }
            }
            SmartSelectionTarget::Footing(id) => {
                if let Some(f) = project.footings.iter().find(|f| f.id == id) {
                    self.edit_width_mm = f.width_mm;
                    self.edit_length_mm = f.length_mm;
                    self.edit_thickness_mm = f.thickness_mm;
                }
            }
            SmartSelectionTarget::Wall(id) => {
                if let Some(w) = project.walls.iter().find(|w| w.id == id) {
                    self.edit_thickness_mm = w.thickness_mm;
                    self.edit_is_structural = w.is_structural;
                }
            }
            _ => {}
        }
    }

    pub(super) fn describe_target(
        target: SmartSelectionTarget,
        project: &BimProject,
    ) -> (String, String, AppIcon) {
        match target {
            SmartSelectionTarget::Column(id) => {
                if let Some(c) = project.columns.iter().find(|c| c.id == id) {
                    let z0 = project.level_elevation(c.base_level_id) / 1000.0;
                    let z1 = project.level_elevation(c.top_level_id) / 1000.0;
                    (
                        c.name.clone(),
                        format!(
                            "Column • {:.0}x{:.0} mm • Elev {:.1}m -> {:.1}m",
                            c.width_mm, c.depth_mm, z0, z1
                        ),
                        AppIcon::Column,
                    )
                } else {
                    (format!("Column #{id}"), "Removed or missing".to_string(), AppIcon::Column)
                }
            }
            SmartSelectionTarget::Beam(id) => {
                if let Some(r) = project.runs.iter().find(|r| r.id == id) {
                    (
                        r.name.clone(),
                        format!(
                            "Beam/Run • L={:.2}m • {:.0}x{:.0} mm",
                            r.start.dist(r.end) * 0.001,
                            r.width_mm,
                            r.depth_mm
                        ),
                        AppIcon::Beam,
                    )
                } else {
                    (format!("Beam #{id}"), "Removed or missing".to_string(), AppIcon::Beam)
                }
            }
            SmartSelectionTarget::Wall(id) => {
                if let Some(w) = project.walls.iter().find(|w| w.id == id) {
                    (
                        w.name.clone(),
                        format!(
                            "Wall • L={:.2}m • t={:.0} mm • {} opening(s)",
                            w.length_mm() * 0.001,
                            w.thickness_mm,
                            w.openings.len()
                        ),
                        AppIcon::Wall,
                    )
                } else {
                    (format!("Wall #{id}"), "Removed or missing".to_string(), AppIcon::Wall)
                }
            }
            SmartSelectionTarget::Footing(id) => {
                if let Some(f) = project.footings.iter().find(|f| f.id == id) {
                    (
                        f.name.clone(),
                        format!(
                            "Isolated Footing • {:.0}x{:.0}x{:.0} mm",
                            f.width_mm, f.length_mm, f.thickness_mm
                        ),
                        AppIcon::Slab,
                    )
                } else {
                    (format!("Footing #{id}"), "Removed or missing".to_string(), AppIcon::Slab)
                }
            }
            SmartSelectionTarget::Slab(id) => {
                if let Some(s) = project.slabs.iter().find(|s| s.id == id) {
                    (
                        s.name.clone(),
                        format!(
                            "Structural {} • t={:.0} mm • {} opening(s)",
                            if s.is_roof { "Roof" } else { "Slab" },
                            s.thickness_mm,
                            s.openings.len()
                        ),
                        AppIcon::Slab,
                    )
                } else {
                    (format!("Slab #{id}"), "Removed or missing".to_string(), AppIcon::Slab)
                }
            }
            SmartSelectionTarget::SketchEntity(id) => (
                format!("CAD Sketch #{id}"),
                "2D Vector Annotation Entity".to_string(),
                AppIcon::Pencil,
            ),
            SmartSelectionTarget::EarthSite { lat, lon } => (
                "Earth Site Location".to_string(),
                format!("WGS84 Lat {:.4}°, Lon {:.4}°", lat, lon),
                AppIcon::Globe,
            ),
        }
    }

    /// Renders the interactive Earth Site Location inspector & build actions cleanly inside Dynamic Island.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_earth_site_inspector_strip(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        lat: f64,
        lon: f64,
        panel_w: f32,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let picked_site =
            compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);

        Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(Margin::symmetric(2, 4))
            .show(ui, |ui| {
                let sw = (panel_w - 10.0).max(160.0);
                ui.set_width(sw);
                ui.set_max_width(sw);
                ui.horizontal(|ui| {
                    icons::icon_label(
                        ui,
                        AppIcon::Globe,
                        13.0,
                        theme.accent_blue,
                        format!("Earth Site Location ({lat:.4}°, {lon:.4}°)"),
                        11.0,
                        theme.text_primary,
                        true,
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icons::icon_only_button(
                            ui,
                            AppIcon::Close,
                            theme.text_secondary,
                            Color32::TRANSPARENT,
                            Stroke::NONE,
                            CornerRadius::same(6),
                            Vec2::splat(18.0),
                            "Dismiss Earth Site Location",
                        )
                        .clicked()
                        {
                            self.selected_target = None;
                            universal_canvas.last_picked_earth_lat_lon = None;
                        }
                    });
                });
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    icons::pill_badge(
                        ui,
                        AppIcon::MapPin,
                        format!("Elev {:+.1} m", picked_site.elevation_m),
                        theme.accent_blue,
                    );
                    icons::pill_badge(
                        ui,
                        AppIcon::Bolt,
                        format!(
                            "Ss {:.2}g / S1 {:.2}g",
                            picked_site.seismic_pga_ss_g, picked_site.seismic_pga_s1_g
                        ),
                        theme.accent_orange,
                    );
                    icons::pill_badge(
                        ui,
                        AppIcon::Globe,
                        format!("Wind {:.1} m/s", picked_site.design_wind_ms),
                        theme.accent_blue,
                    );
                    icons::pill_badge(
                        ui,
                        AppIcon::AlertTriangle,
                        format!("Flood {:.0}%", picked_site.flood_risk_index * 100.0),
                        if picked_site.flood_risk_index > 0.55 {
                            theme.accent_red
                        } else {
                            theme.accent_green
                        },
                    );
                    ui.label(
                        RichText::new(&picked_site.recommended_soil_class)
                            .size(9.8)
                            .color(theme.text_secondary),
                    );
                });
                ui.label(
                    RichText::new(picked_site.foundation_advisory)
                        .size(9.6)
                        .color(theme.text_muted),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::MapPin,
                        "Anchor Site Here",
                        9.8,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        universal_canvas
                            .earth_world
                            .start_fly_to(&universal_canvas.world_map, lat, lon, 16.5, "Anchored Site");
                        self.push_trace(format!("Site anchored @ {lat:.3}°, {lon:.3}°"), true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Column,
                        "Build Here (Earth 3D)",
                        9.8,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        let _ = SpatialManualTool::Column.apply(self, universal_canvas, build_view);
                        actions.anchor_and_build_site = Some((lat, lon));
                        self.push_trace(format!("Anchored & Ready on Earth 3D @ {lat:.3}°, {lon:.3}°"), true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Grid,
                        "Build 3x2 Portal",
                        9.8,
                        Color32::WHITE,
                        theme.accent_orange,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        if let Ok(msg) = build_view
                            .bim_bus
                            .execute_natural_language("portal 3x2 @6m 2floors")
                        {
                            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                            universal_canvas.globe_mode = true;
                            universal_canvas.earth_world.camera.tilt_deg = 42.0;
                            universal_canvas.earth_world.start_fly_to(
                                &universal_canvas.world_map,
                                lat,
                                lon,
                                18.2,
                                "Earth 3D Portal",
                            );
                            actions.sync_fem_and_earth = true;
                            actions.fit_camera = true;
                            self.push_trace(msg, true);
                        }
                    }
                    if !build_view.bim_bus.project.is_empty()
                        && icons::icon_button(
                            ui,
                            AppIcon::ZoomFit,
                            "Focus Building",
                            9.8,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 22.0),
                        )
                        .clicked()
                    {
                        universal_canvas.earth_world.camera.tilt_deg = 42.0;
                        universal_canvas.earth_world.start_fly_to(
                            &universal_canvas.world_map,
                            universal_canvas.ground_origin_lat_lon.0,
                            universal_canvas.ground_origin_lat_lon.1,
                            18.2,
                            "Site Focus",
                        );
                        actions.fit_camera = true;
                        self.push_trace("Focused Earth 3D Building Site", true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Bolt,
                        "Sync Site Seismic",
                        9.8,
                        theme.accent_orange,
                        theme.bg_card,
                        Stroke::new(0.9, theme.accent_orange.gamma_multiply(0.45)),
                        CornerRadius::same(7),
                        Vec2::new(0.0, 22.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        actions.apply_site_seismic = true;
                        self.push_trace(
                            format!("Synced Earth Site Ss={:.2}g, Wind={:.0} m/s", picked_site.seismic_pga_ss_g, picked_site.design_wind_ms),
                            true,
                        );
                    }
                });
            });
        actions
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_smart_popup_body(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        target: SmartSelectionTarget,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let site = universal_canvas.active_site_coupling();

        match target {
            SmartSelectionTarget::EarthSite { lat, lon } => {
                let picked_site =
                    compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
                Self::render_kv_row(
                    ui,
                    theme,
                    "Ground Elevation",
                    &format!("{:+.1} m ASL", picked_site.elevation_m),
                    theme.text_primary,
                );
                Self::render_kv_row(
                    ui,
                    theme,
                    "Seismic Hazard (Ss / S1)",
                    &format!(
                        "{:.2}g / {:.2}g",
                        picked_site.seismic_pga_ss_g, picked_site.seismic_pga_s1_g
                    ),
                    theme.accent_orange,
                );
                Self::render_kv_row(
                    ui,
                    theme,
                    "Design Wind Speed",
                    &format!("{:.1} m/s", picked_site.design_wind_ms),
                    theme.accent_blue,
                );
                Self::render_kv_row(
                    ui,
                    theme,
                    "Flood Risk Index",
                    &format!("{:.0}%", picked_site.flood_risk_index * 100.0),
                    if picked_site.flood_risk_index > 0.55 {
                        theme.accent_red
                    } else {
                        theme.accent_green
                    },
                );
                Self::render_kv_row(
                    ui,
                    theme,
                    "Soil / Foundation",
                    &picked_site.recommended_soil_class,
                    theme.text_secondary,
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(picked_site.foundation_advisory)
                        .size(10.0)
                        .color(theme.text_muted),
                );
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::MapPin,
                        "Anchor Site Here",
                        10.5,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        universal_canvas
                            .earth_world
                            .start_fly_to(&universal_canvas.world_map, lat, lon, 16.5, "Anchored Site");
                        self.push_trace(format!("Site anchored @ {lat:.3}°, {lon:.3}°"), true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Column,
                        "Build Here (Earth 3D)",
                        10.5,
                        Color32::WHITE,
                        theme.accent_green,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Anchor BIM Origin at this Earth site and build directly on Earth 3D")
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        let _ = SpatialManualTool::Column.apply(self, universal_canvas, build_view);
                        actions.anchor_and_build_site = Some((lat, lon));
                        self.push_trace(
                            format!("Anchored & Ready on Earth 3D @ {lat:.3}°, {lon:.3}°"),
                            true,
                        );
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Grid,
                        "Build 3x2 Portal",
                        10.5,
                        Color32::WHITE,
                        theme.accent_orange,
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        if let Ok(msg) = build_view
                            .bim_bus
                            .execute_natural_language("portal 3x2 @6m 2floors")
                        {
                            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                            universal_canvas.globe_mode = true;
                            universal_canvas.earth_world.camera.tilt_deg = 42.0;
                            universal_canvas.earth_world.start_fly_to(
                                &universal_canvas.world_map,
                                lat,
                                lon,
                                18.2,
                                "Earth 3D Portal",
                            );
                            actions.sync_fem_and_earth = true;
                            actions.fit_camera = true;
                            self.push_trace(msg, true);
                        }
                    }
                    if !build_view.bim_bus.project.is_empty()
                        && icons::icon_button(
                            ui,
                            AppIcon::ZoomFit,
                            "Focus Building",
                            10.5,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 26.0),
                        )
                        .on_hover_text("Focus Earth 3D camera onto active structural model")
                        .clicked()
                    {
                        universal_canvas.earth_world.camera.tilt_deg = 42.0;
                        universal_canvas.earth_world.start_fly_to(
                            &universal_canvas.world_map,
                            universal_canvas.ground_origin_lat_lon.0,
                            universal_canvas.ground_origin_lat_lon.1,
                            18.2,
                            "Site Focus",
                        );
                        actions.fit_camera = true;
                        self.push_trace("Focused Earth 3D Building Site", true);
                    }
                    if icons::icon_button(
                        ui,
                        AppIcon::Bolt,
                        "Sync Site Seismic",
                        10.5,
                        theme.accent_orange,
                        theme.bg_card,
                        Stroke::new(1.0, theme.accent_orange.gamma_multiply(0.45)),
                        CornerRadius::same(7),
                        Vec2::new(0.0, 26.0),
                    )
                    .on_hover_text("Apply local seismic Ss/S1 & design wind speed from this coordinate to FEM design")
                    .clicked()
                    {
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        actions.apply_site_seismic = true;
                        self.push_trace(
                            format!("Synced Earth Site Ss={:.2}g, Wind={:.0} m/s", picked_site.seismic_pga_ss_g, picked_site.design_wind_ms),
                            true,
                        );
                    }
                });
            }
            SmartSelectionTarget::SketchEntity(id) => {
                if let Some((kind_lbl, len_m, area_m2, rad_m, detail, _, _)) =
                    universal_canvas.entity_summary(id)
                {
                    Self::render_kv_row(ui, theme, "Entity Type", kind_lbl, theme.accent_blue);
                    Self::render_kv_row(ui, theme, "Geometry", &detail, theme.text_primary);
                    if len_m > 0.001 {
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Length / Perimeter",
                            &format!("{len_m:.3} m"),
                            theme.accent_green,
                        );
                    }
                    if area_m2 > 0.0001 {
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Enclosed Area",
                            &format!("{area_m2:.2} m²"),
                            theme.accent_orange,
                        );
                    }
                    if rad_m > 0.0001 {
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Radius",
                            &format!("{rad_m:.3} m"),
                            theme.text_secondary,
                        );
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Stroke (px):").size(10.5).color(theme.text_secondary));
                        ui.add(
                            egui::DragValue::new(&mut self.sketch_edit_width)
                                .range(0.8..=12.0)
                                .speed(0.1),
                        );
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Color:").size(10.5).color(theme.text_secondary));
                        for (lbl, rgb) in [
                            ("Blue", [59, 130, 246]),
                            ("Green", [16, 185, 129]),
                            ("Orange", [249, 115, 22]),
                            ("Violet", [168, 85, 247]),
                        ] {
                            let active_c = self.sketch_edit_color == rgb;
                            let sw_col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(lbl).size(9.5).color(Color32::WHITE),
                                    )
                                    .min_size(Vec2::new(0.0, 21.0))
                                    .fill(sw_col)
                                    .stroke(if active_c {
                                        Stroke::new(1.5, Color32::WHITE)
                                    } else {
                                        Stroke::NONE
                                    })
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                self.sketch_edit_color = rgb;
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Style",
                            10.0,
                            Color32::WHITE,
                            theme.accent_green,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        && universal_canvas.update_entity_style(
                            id,
                            self.sketch_edit_color,
                            self.sketch_edit_width,
                        )
                        {
                            self.push_trace(format!("Updated 2D CAD #{id} style"), true);
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::Layers,
                            "Duplicate (+500mm)",
                            10.0,
                            theme.text_primary,
                            theme.bg_card,
                            Stroke::new(0.9, theme.border_subtle),
                            CornerRadius::same(7),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        {
                            if let Some(new_id) =
                                universal_canvas.duplicate_offset_entity(id, 500.0, 500.0)
                            {
                                self.selected_target = Some(SmartSelectionTarget::SketchEntity(new_id));
                                self.push_trace(format!("Duplicated CAD #{id} -> #{new_id}"), true);
                            }
                        }
                        if icons::icon_button(
                            ui,
                            AppIcon::Building,
                            "Promote to 3D BIM",
                            10.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(7),
                            Vec2::new(0.0, 24.0),
                        )
                        .clicked()
                        {
                            let (w, r, s) =
                                universal_canvas.promote_selection_to_bim(&mut build_view.bim_bus.project);
                            if w + r + s > 0 {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(format!("Promoted +{w}W +{r}B +{s}S to BIM"), true);
                            }
                        }
                    });
                    ui.add_space(4.0);
                    if icons::icon_button(
                        ui,
                        AppIcon::Trash,
                        "Delete Entity (Undoable)",
                        10.0,
                        theme.accent_red,
                        theme.accent_red.gamma_multiply(0.12),
                        Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                        CornerRadius::same(7),
                        Vec2::new(ui.available_width(), 24.0),
                    )
                    .clicked()
                    && universal_canvas.delete_entity_by_id(id)
                    {
                        self.selected_target = None;
                        self.push_trace(format!("Deleted 2D CAD entity #{id}"), true);
                    }
                } else {
                    Self::render_kv_row(
                        ui,
                        theme,
                        "Entity ID",
                        &format!("#{id} (Removed)"),
                        theme.text_muted,
                    );
                }
            }
            SmartSelectionTarget::Column(col_id) => {
                let col_opt = build_view
                    .bim_bus
                    .project
                    .columns
                    .iter()
                    .find(|c| c.id == col_id)
                    .cloned();
                let Some(col) = col_opt else {
                    ui.label("Column not found in active model.");
                    return actions;
                };
                let fem_id = universal_canvas
                    .earth_fem_cache
                    .fem_id(BimElementKind::Column, col_id)
                    .or_else(|| {
                        build_view
                            .bim_bus
                            .project
                            .fem_id_for_bim_element(BimElementKind::Column, col_id)
                    });
                let fem_member = fem_id.and_then(|fid| members.iter().find(|m| m.id == fid));
                let height_m = ((build_view.bim_bus.project.level_elevation(col.top_level_id)
                    - build_view.bim_bus.project.level_elevation(col.base_level_id))
                .abs()
                .max(3000.0))
                    * 0.001;

                self.render_structural_kpi_strip(ui, theme, fem_member);
                ui.add_space(4.0);
                Self::render_hosted_rc_section_preview(
                    ui,
                    theme,
                    &build_view.bim_bus.project,
                    col_id,
                    col.width_mm,
                    col.depth_mm,
                    true,
                    fem_member,
                );
                ui.add_space(6.0);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Width (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_width_mm)
                                    .range(150.0..=2000.0)
                                    .speed(10.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Depth (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_depth_mm)
                                    .range(150.0..=2000.0)
                                    .speed(10.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Rotation (°):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_rot_deg)
                                    .range(-180.0..=180.0)
                                    .speed(5.0),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            for (lbl, w, d) in [
                                ("350x350", 350.0, 350.0),
                                ("400x400", 400.0, 400.0),
                                ("500x500", 500.0, 500.0),
                                ("600x600", 600.0, 600.0),
                            ] {
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new(lbl).size(10.0))
                                            .corner_radius(CornerRadius::same(6)),
                                    )
                                    .clicked()
                                {
                                    self.edit_width_mm = w;
                                    self.edit_depth_mm = d;
                                    self.edit_profile_name = format!("RC {w:.0}x{d:.0}");
                                }
                            }
                        });
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Column Resize",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 28.0),
                        )
                        .clicked()
                        {
                            let new_prof = format!("RC {:.0}x{:.0}", self.edit_width_mm, self.edit_depth_mm);
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeColumn {
                                column_id: col_id,
                                width_mm: self.edit_width_mm,
                                depth_mm: self.edit_depth_mm,
                                rotation_deg: self.edit_rot_deg,
                                profile_name: new_prof,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Wrench,
                            "Auto-Size Element (DCR ≤ 85%)",
                            10.5,
                            theme.accent_green,
                            theme.accent_green.gamma_multiply(0.14),
                            Stroke::new(0.9, theme.accent_green.gamma_multiply(0.55)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                            && self.auto_size_selected_element(build_view).is_ok()
                        {
                            actions.sync_fem_and_earth = true;
                        }
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Column (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: col_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        let force_kn = fem_member.map(|m| m.force_kn).unwrap_or(-420.0);
                        let cap_kn = fem_member.map(|m| m.capacity_kn).unwrap_or(1450.0);
                        let my_knm = fem_member.map(|m| m.moment_my_knm).unwrap_or(0.0);
                        let mz_knm = fem_member.map(|m| m.moment_mz_knm).unwrap_or(0.0);
                        let vy_kn = fem_member.map(|m| m.shear_vy_kn).unwrap_or(0.0);
                        let util = fem_member.map(|m| m.utilization).unwrap_or(0.29);
                        let dcr_pmv = fem_member.map(|m| m.dcr_interaction).unwrap_or(util);
                        let ari = fem_member.map(|m| m.ari).unwrap_or(0.90);
                        let ci = fem_member.map(|m| m.ci).unwrap_or(0.10);
                        let svd_m = fem_member.map(|m| m.svd_margin).unwrap_or(0.95);
                        let r_min_mm = (col.width_mm.min(col.depth_mm) * 0.289).max(1.0);
                        let slenderness = (height_m * 1000.0) / r_min_mm;

                        Self::render_kv_row(
                            ui,
                            theme,
                            "Axial Load (Pu)",
                            &format!("{:.1} kN", force_kn),
                            theme.accent_blue,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "6-DOF Moment (My / Mz)",
                            &format!("{:.1} / {:.1} kNm", my_knm, mz_knm),
                            theme.text_primary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "6-DOF Shear (Vu) / ϕPn",
                            &format!("{:.1} kN / {:.0} kN", vy_kn, cap_kn),
                            theme.text_secondary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "LRFD Utilization (DCR)",
                            &format!("{:.1}%", util * 100.0),
                            if util > 0.85 { theme.accent_red } else { theme.accent_green },
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "P-M-V Second-Order DCR",
                            &format!("{:.3}", dcr_pmv),
                            theme.text_secondary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Redundancy Index (ARI / CI)",
                            &format!("{:.3} / {:.3}", ari, ci),
                            if ari >= 0.70 { theme.accent_green } else { theme.accent_orange },
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "SVD Kinematic Margin",
                            &format!("{:.3}", svd_m),
                            theme.text_secondary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Slenderness (kLu/r)",
                            &format!("{:.1} (Site Ss={:.2}g)", slenderness, site.seismic_pga_ss_g),
                            theme.text_muted,
                        );
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            if let Some(fid) = fem_id {
                                let elim = build_view.simulated_eliminated_ids.contains(&fid);
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Bolt,
                                    if elim { "Restore N-1" } else { "Simulate N-1" },
                                    10.0,
                                    Color32::WHITE,
                                    if elim { theme.accent_green } else { theme.accent_red },
                                    Stroke::NONE,
                                    CornerRadius::same(7),
                                    Vec2::new(0.0, 24.0),
                                )
                                .clicked()
                                {
                                    actions.toggle_n1_member = Some(fid);
                                    self.push_trace(
                                        format!("Toggled N-1 on {}", col.name),
                                        true,
                                    );
                                }
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Plus,
                                "Upsize +50mm",
                                10.0,
                                Color32::WHITE,
                                theme.accent_blue,
                                Stroke::NONE,
                                CornerRadius::same(7),
                                Vec2::new(0.0, 24.0),
                            )
                            .clicked()
                            {
                                let nw = col.width_mm + 50.0;
                                let nd = col.depth_mm + 50.0;
                                self.edit_width_mm = nw;
                                self.edit_depth_mm = nd;
                                if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeColumn {
                                    column_id: col_id,
                                    width_mm: nw,
                                    depth_mm: nd,
                                    rotation_deg: col.rotation_deg,
                                    profile_name: format!("RC {nw:.0}x{nd:.0}"),
                                }) {
                                    actions.sync_fem_and_earth = true;
                                    self.push_trace(msg, true);
                                }
                            }
                            if icons::icon_button(
                                ui,
                                AppIcon::Rebar,
                                "Auto-Rebar",
                                10.0,
                                theme.text_primary,
                                theme.bg_card,
                                Stroke::new(1.0, theme.border_subtle),
                                CornerRadius::same(7),
                                Vec2::new(0.0, 24.0),
                            )
                            .clicked()
                            {
                                if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ReinforceHost {
                                    host_id: col_id,
                                    preset: Default::default(),
                                }) {
                                    self.push_trace(msg, true);
                                }
                            }
                        });
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let vol_m3 = (col.width_mm * 0.001) * (col.depth_mm * 0.001) * height_m;
                        let rebar_kg = vol_m3 * 145.0;
                        let co2_kg = vol_m3 * 340.0 + rebar_kg * 1.15;
                        let cost_idr = (vol_m3 * 1_650_000.0 + rebar_kg * 18_500.0)
                            * site.remote_logistics_multiplier as f64;
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Concrete Volume",
                            &format!("{vol_m3:.3} m³"),
                            theme.text_primary,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Est. Reinforcement",
                            &format!("{rebar_kg:.1} kg (ρ≈1.8%)"),
                            theme.accent_blue,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Embodied Carbon (A1–A3)",
                            &format!("{co2_kg:.1} kgCO₂e"),
                            theme.accent_green,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Est. Element Cost",
                            &format!("Rp {}", crate::core::cost::CostEngine::format_idr(cost_idr)),
                            theme.accent_orange,
                        );
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &col.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
            SmartSelectionTarget::Beam(run_id) => {
                let run_opt = build_view
                    .bim_bus
                    .project
                    .runs
                    .iter()
                    .find(|r| r.id == run_id)
                    .cloned();
                let Some(run) = run_opt else {
                    ui.label("Beam/Run not found in active model.");
                    return actions;
                };
                let fem_id = universal_canvas
                    .earth_fem_cache
                    .fem_id(BimElementKind::Run, run_id)
                    .or_else(|| {
                        build_view
                            .bim_bus
                            .project
                            .fem_id_for_bim_element(BimElementKind::Run, run_id)
                    });
                let fem_member = fem_id.and_then(|fid| members.iter().find(|m| m.id == fid));
                let len_m = run.start.dist(run.end) * 0.001;

                self.render_structural_kpi_strip(ui, theme, fem_member);
                ui.add_space(4.0);
                Self::render_hosted_rc_section_preview(
                    ui,
                    theme,
                    &build_view.bim_bus.project,
                    run_id,
                    run.width_mm,
                    run.depth_mm,
                    false,
                    fem_member,
                );
                ui.add_space(6.0);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Width (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_width_mm)
                                    .range(100.0..=1500.0)
                                    .speed(10.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Depth (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_depth_mm)
                                    .range(150.0..=2500.0)
                                    .speed(10.0),
                            );
                        });
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Beam Resize",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 28.0),
                        )
                        .clicked()
                        {
                            let new_prof = format!("RC {:.0}x{:.0}", self.edit_width_mm, self.edit_depth_mm);
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeRun {
                                run_id,
                                width_mm: self.edit_width_mm,
                                depth_mm: self.edit_depth_mm,
                                profile_name: new_prof,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Wrench,
                            "Auto-Size Element (DCR ≤ 85%)",
                            10.5,
                            theme.accent_green,
                            theme.accent_green.gamma_multiply(0.14),
                            Stroke::new(0.9, theme.accent_green.gamma_multiply(0.55)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                            && self.auto_size_selected_element(build_view).is_ok()
                        {
                            actions.sync_fem_and_earth = true;
                        }
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Beam (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: run_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        let force_kn = fem_member.map(|m| m.force_kn).unwrap_or(0.0);
                        let cap_kn = fem_member.map(|m| m.capacity_kn).unwrap_or(520.0);
                        let my_knm = fem_member.map(|m| m.moment_my_knm).unwrap_or(0.0);
                        let mz_knm = fem_member.map(|m| m.moment_mz_knm).unwrap_or(0.0);
                        let vy_kn = fem_member.map(|m| m.shear_vy_kn).unwrap_or(0.0);
                        let util = fem_member.map(|m| m.utilization).unwrap_or(0.36);
                        let ari = fem_member.map(|m| m.ari).unwrap_or(0.88);
                        let span_depth = (len_m * 1000.0) / run.depth_mm.max(100.0);

                        Self::render_kv_row(ui, theme, "Clear Span (L) / L/h", &format!("{len_m:.2} m / {span_depth:.1}"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "6-DOF Moment (My / Mz)", &format!("{my_knm:.1} / {mz_knm:.1} kNm"), theme.accent_blue);
                        Self::render_kv_row(ui, theme, "6-DOF Shear (Vu) / Axial", &format!("{vy_kn:.1} kN / {force_kn:.1} kN"), theme.text_secondary);
                        Self::render_kv_row(
                            ui,
                            theme,
                            "P-M-V DCR / ARI / ϕPn",
                            &format!("{:.1}% / {:.3} / {:.0}kN", util * 100.0, ari, cap_kn),
                            if util > 0.85 { theme.accent_red } else { theme.accent_green },
                        );
                        if let Some(fid) = fem_id {
                            ui.add_space(6.0);
                            let elim = universal_canvas
                                .earth_fem_cache
                                .is_n1_removed(BimElementKind::Run, run_id)
                                || build_view.simulated_eliminated_ids.contains(&fid);
                            if icons::icon_button(
                                ui,
                                AppIcon::Bolt,
                                if elim { "Restore N-1 Beam" } else { "Simulate N-1 Removal" },
                                10.0,
                                Color32::WHITE,
                                if elim { theme.accent_green } else { theme.accent_red },
                                Stroke::NONE,
                                CornerRadius::same(7),
                                Vec2::new(ui.available_width(), 24.0),
                            )
                            .clicked()
                            {
                                actions.toggle_n1_member = Some(fid);
                            }
                        }
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let vol_m3 = len_m * (run.width_mm * 0.001) * (run.depth_mm * 0.001);
                        let rebar_kg = vol_m3 * 125.0;
                        let co2_kg = vol_m3 * 335.0 + rebar_kg * 1.15;
                        let cost_idr = (vol_m3 * 1_620_000.0 + rebar_kg * 18_500.0)
                            * site.remote_logistics_multiplier as f64;
                        Self::render_kv_row(ui, theme, "Concrete Volume", &format!("{vol_m3:.3} m³"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Est. Reinforcement", &format!("{rebar_kg:.1} kg"), theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Embodied Carbon", &format!("{co2_kg:.1} kgCO₂e"), theme.accent_green);
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Est. Element Cost",
                            &format!("Rp {}", crate::core::cost::CostEngine::format_idr(cost_idr)),
                            theme.accent_orange,
                        );
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &run.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
            SmartSelectionTarget::Wall(wall_id) => {
                let wall_opt = build_view
                    .bim_bus
                    .project
                    .walls
                    .iter()
                    .find(|w| w.id == wall_id)
                    .cloned();
                let Some(wall) = wall_opt else {
                    ui.label("Wall not found.");
                    return actions;
                };
                let len_m = wall.length_mm() * 0.001;
                let h_m = wall.unconnected_height_mm * 0.001;
                let vol_m3 = len_m * h_m * (wall.thickness_mm * 0.001);

                Self::render_wall_sbe_section_preview(ui, theme, &wall);
                ui.add_space(6.0);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Thickness (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(
                                egui::DragValue::new(&mut self.edit_thickness_mm)
                                    .range(80.0..=1000.0)
                                    .speed(10.0),
                            );
                        });
                        ui.checkbox(&mut self.edit_is_structural, "Structural Shear Wall");
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .small_button("+ Door (900x2100)")
                                .on_hover_text("Insert CSG Single Door opening centered along this wall")
                                .clicked()
                            {
                                let off = (wall.length_mm() * 0.5 - 450.0).max(200.0);
                                if let Ok(msg) = build_view.bim_bus.execute(BimCommand::AddOpening {
                                    wall_id,
                                    kind: crate::core::OpeningKind::SingleDoor,
                                    offset_along_mm: off,
                                    width_mm: 900.0,
                                    height_mm: 2100.0,
                                    sill_height_mm: 0.0,
                                }) {
                                    actions.sync_fem_and_earth = true;
                                    self.push_trace(msg, true);
                                }
                            }
                            if ui
                                .small_button("+ Window (1800x1400)")
                                .on_hover_text("Insert CSG Casement Window opening centered along this wall")
                                .clicked()
                            {
                                let off = (wall.length_mm() * 0.5 - 900.0).max(200.0);
                                if let Ok(msg) = build_view.bim_bus.execute(BimCommand::AddOpening {
                                    wall_id,
                                    kind: crate::core::OpeningKind::CasementWindow,
                                    offset_along_mm: off,
                                    width_mm: 1800.0,
                                    height_mm: 1400.0,
                                    sill_height_mm: 900.0,
                                }) {
                                    actions.sync_fem_and_earth = true;
                                    self.push_trace(msg, true);
                                }
                            }
                        });
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Wall Update",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 28.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeWall {
                                wall_id,
                                thickness_mm: self.edit_thickness_mm,
                                is_structural: self.edit_is_structural,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Wrench,
                            "Auto-Size Shear Wall",
                            10.5,
                            theme.accent_green,
                            theme.accent_green.gamma_multiply(0.14),
                            Stroke::new(0.9, theme.accent_green.gamma_multiply(0.55)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                            && self.auto_size_selected_element(build_view).is_ok()
                        {
                            actions.sync_fem_and_earth = true;
                        }
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Wall (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: wall_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        let wall_fem_id = universal_canvas
                            .earth_fem_cache
                            .fem_id(BimElementKind::Wall, wall_id);
                        let wall_member =
                            wall_fem_id.and_then(|fid| members.iter().find(|m| m.id == fid));
                        Self::render_kv_row(ui, theme, "Length x Height", &format!("{len_m:.2}m x {h_m:.2}m"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Structural Shear Role", if wall.is_structural { "Active Shear Wall" } else { "Architectural Partition" }, theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Compound Layers", &format!("{} plies", wall.layers.len()), theme.text_secondary);
                        if let Some(m) = wall_member {
                            Self::render_kv_row(
                                ui,
                                theme,
                                "Shear Strut DCR / ARI",
                                &format!("{:.1}% / {:.3}", m.utilization * 100.0, m.ari),
                                if m.utilization > 0.85 { theme.accent_red } else { theme.accent_green },
                            );
                        }
                        if let Some(fid) = wall_fem_id {
                            if wall.is_structural {
                                ui.add_space(6.0);
                                let elim = universal_canvas
                                    .earth_fem_cache
                                    .is_n1_removed(BimElementKind::Wall, wall_id)
                                    || build_view.simulated_eliminated_ids.contains(&fid);
                                if icons::icon_button(
                                    ui,
                                    AppIcon::Bolt,
                                    if elim { "Restore N-1 Wall" } else { "Simulate N-1 Removal" },
                                    10.0,
                                    Color32::WHITE,
                                    if elim { theme.accent_green } else { theme.accent_red },
                                    Stroke::NONE,
                                    CornerRadius::same(7),
                                    Vec2::new(ui.available_width(), 24.0),
                                )
                                .clicked()
                                {
                                    actions.toggle_n1_member = Some(fid);
                                }
                            }
                        }
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let co2_kg = vol_m3 * 290.0;
                        Self::render_kv_row(ui, theme, "Wall Volume", &format!("{vol_m3:.2} m³"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Embodied Carbon", &format!("{co2_kg:.1} kgCO₂e"), theme.accent_green);
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &wall.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
            SmartSelectionTarget::Footing(foot_id) => {
                let ft_opt = build_view
                    .bim_bus
                    .project
                    .footings
                    .iter()
                    .find(|f| f.id == foot_id)
                    .cloned();
                let Some(ft) = ft_opt else {
                    ui.label("Footing not found.");
                    return actions;
                };
                let area_m2 = (ft.width_mm * 0.001) * (ft.length_mm * 0.001);
                let vol_m3 = area_m2 * (ft.thickness_mm * 0.001);
                let qa_kpa = if site.recommended_soil_class.contains("SC") {
                    240.0
                } else if site.recommended_soil_class.contains("SD") {
                    175.0
                } else {
                    125.0
                };
                let est_bearing_kpa = 640.0 / area_m2.max(0.25);

                Self::render_footing_punching_and_pile_preview(ui, theme, &ft, est_bearing_kpa, qa_kpa);
                ui.add_space(6.0);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Width B (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut self.edit_width_mm).range(600.0..=6000.0).speed(50.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Length L (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut self.edit_length_mm).range(600.0..=6000.0).speed(50.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Thick h (mm):").size(11.0).color(theme.text_secondary));
                            ui.add(egui::DragValue::new(&mut self.edit_thickness_mm).range(250.0..=2000.0).speed(25.0));
                        });
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::CheckCircle,
                            "Apply Footing Resize",
                            11.0,
                            Color32::WHITE,
                            theme.accent_blue,
                            Stroke::NONE,
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 28.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeFooting {
                                footing_id: foot_id,
                                width_mm: self.edit_width_mm,
                                length_mm: self.edit_length_mm,
                                thickness_mm: self.edit_thickness_mm,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.push_trace(msg, true);
                            }
                        }
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Wrench,
                            "Auto-Size Footing (qa Check)",
                            10.5,
                            theme.accent_green,
                            theme.accent_green.gamma_multiply(0.14),
                            Stroke::new(0.9, theme.accent_green.gamma_multiply(0.55)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                            && self.auto_size_selected_element(build_view).is_ok()
                        {
                            actions.sync_fem_and_earth = true;
                        }
                        ui.add_space(4.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Footing (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: foot_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        Self::render_kv_row(ui, theme, "Bearing Area", &format!("{area_m2:.2} m²"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Site Allowable qa", &format!("{qa_kpa:.0} kPa ({})", site.recommended_soil_class), theme.accent_blue);
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Site Hazard (Ss / Wind)",
                            &format!("{:.2}g / {:.0} m/s", site.seismic_pga_ss_g, site.design_wind_ms),
                            theme.accent_orange,
                        );
                        Self::render_kv_row(
                            ui,
                            theme,
                            "Est. Soil Pressure",
                            &format!("{est_bearing_kpa:.0} kPa ({:.0}%)", (est_bearing_kpa / qa_kpa) * 100.0),
                            if est_bearing_kpa > qa_kpa { theme.accent_red } else { theme.accent_green },
                        );
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let rebar_kg = vol_m3 * 105.0;
                        let co2_kg = vol_m3 * 315.0 + rebar_kg * 1.15;
                        Self::render_kv_row(ui, theme, "Concrete Volume", &format!("{vol_m3:.3} m³"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Est. Rebar", &format!("{rebar_kg:.1} kg"), theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Embodied Carbon", &format!("{co2_kg:.1} kgCO₂e"), theme.accent_green);
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &ft.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
            SmartSelectionTarget::Slab(slab_id) => {
                let slab_opt = build_view
                    .bim_bus
                    .project
                    .slabs
                    .iter()
                    .find(|s| s.id == slab_id)
                    .cloned();
                let Some(slab) = slab_opt else {
                    ui.label("Slab/Roof not found in active model.");
                    return actions;
                };
                let area_mm2 = {
                    let n = slab.boundary.len();
                    let mut sum = 0.0_f64;
                    for i in 0..n {
                        let a = slab.boundary[i];
                        let b = slab.boundary[(i + 1) % n];
                        sum += a.0 * b.1 - b.0 * a.1;
                    }
                    0.5 * sum.abs()
                };
                let area_m2 = area_mm2 * 1e-6;
                let vol_m3 = area_m2 * (slab.thickness_mm * 0.001);
                let elev_m = build_view.bim_bus.project.level_elevation(slab.level_id) * 0.001;

                Self::render_slab_twoway_preview(ui, theme, &slab, area_m2);
                ui.add_space(6.0);

                match self.popup_tab {
                    SmartPopupTab::Properties => {
                        Self::render_kv_row(ui, theme, "Level Elevation", &format!("{elev_m:.2} m"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Thickness", &format!("{:.0} mm", slab.thickness_mm), theme.accent_blue);
                        Self::render_kv_row(ui, theme, "Footprint Area", &format!("{area_m2:.2} m²"), theme.accent_green);
                        Self::render_kv_row(ui, theme, "Shaft Openings", &format!("{}", slab.openings.len()), theme.text_secondary);
                        ui.add_space(6.0);
                        if icons::icon_button(
                            ui,
                            AppIcon::Trash,
                            "Delete Slab (Undoable)",
                            10.5,
                            theme.accent_red,
                            theme.accent_red.gamma_multiply(0.12),
                            Stroke::new(0.9, theme.accent_red.gamma_multiply(0.45)),
                            CornerRadius::same(8),
                            Vec2::new(ui.available_width(), 26.0),
                        )
                        .clicked()
                        {
                            if let Ok(msg) = build_view.bim_bus.execute(BimCommand::DeleteElement {
                                element_id: slab_id,
                            }) {
                                actions.sync_fem_and_earth = true;
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
                                self.push_trace(msg, true);
                            }
                        }
                    }
                    SmartPopupTab::Calculations => {
                        let sw_kpa = 24.0 * (slab.thickness_mm * 0.001);
                        let est_factored_kpa = 1.2 * (sw_kpa + 1.5) + 1.6 * 2.5;
                        Self::render_kv_row(ui, theme, "Self-Weight (DL)", &format!("{sw_kpa:.2} kPa"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Factored Qu (1.2D+1.6L)", &format!("{est_factored_kpa:.2} kPa"), theme.accent_orange);
                    }
                    SmartPopupTab::Lifecycle5D6D => {
                        let co2_kg = vol_m3 * 305.0;
                        Self::render_kv_row(ui, theme, "Concrete Volume", &format!("{vol_m3:.2} m³"), theme.text_primary);
                        Self::render_kv_row(ui, theme, "Embodied Carbon", &format!("{co2_kg:.1} kgCO₂e"), theme.accent_green);
                    }
                    SmartPopupTab::AskAi => {
                        self.render_popup_ai_actions(
                            ui,
                            theme,
                            &slab.name,
                            ai_assistant,
                            members,
                            projects,
                            db_components,
                        );
                    }
                }
            }
        }

        actions
    }

    pub(super) fn render_structural_kpi_strip(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        fem_member: Option<&StructuralMember>,
    ) {
        let dcr = fem_member.map(|m| m.utilization * 100.0).unwrap_or(42.0);
        let ari = fem_member.map(|m| m.ari).unwrap_or(0.89);
        let force = fem_member.map(|m| m.force_kn).unwrap_or(-420.0);
        let dcr_col = if dcr > 85.0 {
            theme.accent_red
        } else if dcr > 65.0 {
            theme.accent_orange
        } else {
            theme.accent_green
        };
        let ari_col = if ari >= 0.70 {
            theme.accent_green
        } else {
            theme.accent_orange
        };

        ui.columns(3, |cols| {
            Self::kpi_chip(&mut cols[0], theme, "DCR", &format!("{dcr:.1}%"), dcr_col);
            Self::kpi_chip(&mut cols[1], theme, "ARI", &format!("{ari:.3}"), ari_col);
            Self::kpi_chip(&mut cols[2], theme, "FORCE", &format!("{force:.0}kN"), theme.accent_blue);
        });
    }

    pub(super) fn kpi_chip(ui: &mut Ui, theme: &MacTheme, label: &str, val: &str, col: Color32) {
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::symmetric(8, 5))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(label).size(9.0).color(theme.text_muted));
                    ui.label(RichText::new(val).size(12.0).strong().color(col));
                });
            });
    }

    pub(super) fn render_kv_row(ui: &mut Ui, theme: &MacTheme, k: &str, v: &str, col: Color32) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(k).size(10.8).color(theme.text_secondary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(v).size(10.8).strong().color(col));
            });
        });
        ui.add_space(2.0);
    }

    /// Resolves hosted longitudinal rebars for a column or beam and draws the 2D RC section preview.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_hosted_rc_section_preview(
        ui: &mut Ui,
        theme: &MacTheme,
        project: &BimProject,
        host_id: usize,
        width_mm: f64,
        depth_mm: f64,
        is_column: bool,
        fem_member: Option<&StructuralMember>,
    ) {
        let hosted_rebars: Vec<_> = project
            .rebars
            .iter()
            .filter(|rb| rb.host_id == host_id && !rb.closed_stirrup)
            .collect();
        let rebar_bars = if hosted_rebars.is_empty() {
            if is_column { 8 } else { 6 }
        } else {
            hosted_rebars.iter().map(|rb| rb.count.max(1)).sum()
        };
        let bar_dia_mm = hosted_rebars
            .first()
            .map(|rb| rb.diameter_mm)
            .unwrap_or(if is_column { 19.0 } else { 16.0 });
        let default_util = if is_column { 0.42 } else { 0.36 };
        let default_ari = if is_column { 0.89 } else { 0.88 };
        Self::render_rc_cross_section_preview(
            ui,
            theme,
            width_mm,
            depth_mm,
            is_column,
            rebar_bars,
            bar_dia_mm,
            fem_member.map(|m| m.utilization).unwrap_or(default_util),
            fem_member.map(|m| m.ari).unwrap_or(default_ari),
        );
    }

    /// Renders a live 2D reinforced-concrete cross-section & rebar cage mini-canvas alongside
    /// LRFD DCR% and ARI redundancy gauges inside the Smart Element Pop-Up.
    #[allow(clippy::too_many_arguments)]
    pub fn render_rc_cross_section_preview(
        ui: &mut Ui,
        theme: &MacTheme,
        width_mm: f64,
        depth_mm: f64,
        is_column: bool,
        rebar_bars: usize,
        bar_dia_mm: f64,
        util_ratio: f64,
        ari_score: f64,
    ) {
        let avail_w = ui.available_width().max(150.0);
        Frame::new()
            .fill(theme.bg_sidebar)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(0.8, theme.border_subtle))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                ui.set_max_width((avail_w - 4.0).max(140.0));
                ui.horizontal(|ui| {
                    let (canvas_rect, _) =
                        ui.allocate_exact_size(Vec2::new(68.0, 68.0), egui::Sense::hover());
                    let painter = ui.painter_at(canvas_rect);
                    let max_dim = (width_mm.max(depth_mm) as f32).max(100.0);
                    let scale = 52.0 / max_dim;
                    let bw = (width_mm as f32 * scale).clamp(18.0, 52.0);
                    let bh = (depth_mm as f32 * scale).clamp(18.0, 52.0);
                    let sec_rect = Rect::from_center_size(canvas_rect.center(), Vec2::new(bw, bh));

                    painter.rect_filled(
                        sec_rect,
                        CornerRadius::same(2),
                        Color32::from_rgba_unmultiplied(100, 116, 139, 92),
                    );
                    painter.rect_stroke(
                        sec_rect,
                        CornerRadius::same(2),
                        Stroke::new(1.2, theme.text_secondary),
                        egui::StrokeKind::Inside,
                    );

                    // 40 mm concrete cover stirrup hoop inset
                    let hoop_rect = sec_rect.shrink(4.5);
                    if hoop_rect.width() > 6.0 && hoop_rect.height() > 6.0 {
                        painter.rect_stroke(
                            hoop_rect,
                            CornerRadius::same(2),
                            Stroke::new(1.1, theme.accent_orange.gamma_multiply(0.9)),
                            egui::StrokeKind::Inside,
                        );
                        let n_bars = rebar_bars.clamp(4, 16);
                        let mut bar_pts = vec![
                            hoop_rect.left_top(),
                            hoop_rect.right_top(),
                            hoop_rect.right_bottom(),
                            hoop_rect.left_bottom(),
                        ];
                        let extra = n_bars.saturating_sub(4);
                        if extra > 0 {
                            if is_column {
                                for k in 0..extra {
                                    let side = k % 4;
                                    let slot = (k / 4) + 1;
                                    let per_side = (extra / 4) + usize::from(side < extra % 4);
                                    let t = (slot as f32) / ((per_side + 1) as f32);
                                    let pt = match side {
                                        0 => egui::pos2(
                                            egui::lerp(hoop_rect.left()..=hoop_rect.right(), t),
                                            hoop_rect.top(),
                                        ),
                                        1 => egui::pos2(
                                            hoop_rect.right(),
                                            egui::lerp(hoop_rect.top()..=hoop_rect.bottom(), t),
                                        ),
                                        2 => egui::pos2(
                                            egui::lerp(hoop_rect.left()..=hoop_rect.right(), t),
                                            hoop_rect.bottom(),
                                        ),
                                        _ => egui::pos2(
                                            hoop_rect.left(),
                                            egui::lerp(hoop_rect.top()..=hoop_rect.bottom(), t),
                                        ),
                                    };
                                    bar_pts.push(pt);
                                }
                            } else {
                                let top_n = extra / 2;
                                let bot_n = extra - top_n;
                                for i in 0..top_n {
                                    let t = ((i + 1) as f32) / ((top_n + 1) as f32);
                                    bar_pts.push(egui::pos2(
                                        egui::lerp(hoop_rect.left()..=hoop_rect.right(), t),
                                        hoop_rect.top(),
                                    ));
                                }
                                for i in 0..bot_n {
                                    let t = ((i + 1) as f32) / ((bot_n + 1) as f32);
                                    bar_pts.push(egui::pos2(
                                        egui::lerp(hoop_rect.left()..=hoop_rect.right(), t),
                                        hoop_rect.bottom(),
                                    ));
                                }
                            }
                        }
                        let bar_col = Color32::from_rgb(56, 189, 248);
                        for pt in bar_pts {
                            painter.circle_filled(pt, 2.2, bar_col);
                        }
                    }

                    ui.vertical(|ui| {
                        let gauge_w = ui.available_width().clamp(72.0, 210.0);
                        ui.label(
                            RichText::new(format!(
                                "{:.0}×{:.0} mm • {}-D{:.0}",
                                width_mm, depth_mm, rebar_bars, bar_dia_mm
                            ))
                            .size(9.8)
                            .monospace()
                            .strong()
                            .color(theme.text_primary),
                        );
                        ui.add_space(3.0);

                        let dcr_frac = (util_ratio as f32).clamp(0.0, 1.0);
                        let dcr_col = if util_ratio > 0.85 {
                            theme.accent_red
                        } else if util_ratio > 0.65 {
                            theme.accent_orange
                        } else {
                            theme.accent_green
                        };
                        ui.label(
                            RichText::new(format!("LRFD DCR: {:.1}%", util_ratio * 100.0))
                                .size(9.2)
                                .monospace()
                                .color(dcr_col),
                        );
                        let (dcr_bar, _) =
                            ui.allocate_exact_size(Vec2::new(gauge_w, 7.0), egui::Sense::hover());
                        ui.painter().rect_filled(dcr_bar, CornerRadius::same(3), theme.bg_card);
                        let dcr_fill = Rect::from_min_size(
                            dcr_bar.min,
                            Vec2::new((dcr_bar.width() * dcr_frac).max(2.0), dcr_bar.height()),
                        );
                        ui.painter().rect_filled(dcr_fill, CornerRadius::same(3), dcr_col);

                        ui.add_space(3.0);
                        let ari_frac = (ari_score as f32).clamp(0.0, 1.0);
                        let ari_col = if ari_score >= 0.70 {
                            theme.accent_green
                        } else if ari_score >= 0.60 {
                            theme.accent_orange
                        } else {
                            theme.accent_red
                        };
                        ui.label(
                            RichText::new(format!("ARI: {:.3} (Target ≥ 0.70)", ari_score))
                                .size(9.2)
                                .monospace()
                                .color(ari_col),
                        );
                        let (ari_bar, _) =
                            ui.allocate_exact_size(Vec2::new(gauge_w, 7.0), egui::Sense::hover());
                        ui.painter().rect_filled(ari_bar, CornerRadius::same(3), theme.bg_card);
                        let ari_fill = Rect::from_min_size(
                            ari_bar.min,
                            Vec2::new((ari_bar.width() * ari_frac).max(2.0), ari_bar.height()),
                        );
                        ui.painter().rect_filled(ari_fill, CornerRadius::same(3), ari_col);
                        let target_x = ari_bar.left() + ari_bar.width() * 0.70;
                        ui.painter().line_segment(
                            [
                                egui::pos2(target_x, ari_bar.top() - 1.0),
                                egui::pos2(target_x, ari_bar.bottom() + 1.0),
                            ],
                            Stroke::new(1.3, Color32::WHITE),
                        );
                    });
                });
            });
    }

    /// Renders a live 2D Shear-Wall Special Boundary Element (SBE) confinement & web rebar
    /// horizontal section preview inside the Smart Element Pop-Up.
    pub fn render_wall_sbe_section_preview(
        ui: &mut Ui,
        theme: &MacTheme,
        wall: &crate::core::bim::BimWall,
    ) {
        let avail_w = ui.available_width().max(150.0);
        Frame::new()
            .fill(theme.bg_sidebar)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(0.8, theme.border_subtle))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                ui.set_max_width((avail_w - 4.0).max(140.0));
                ui.horizontal(|ui| {
                    let (canvas_rect, _) =
                        ui.allocate_exact_size(Vec2::new(82.0, 48.0), egui::Sense::hover());
                    let painter = ui.painter_at(canvas_rect);
                    let w_px = 72.0_f32;
                    let h_px = ((wall.thickness_mm as f32 / wall.length_mm().max(600.0) as f32) * w_px)
                        .clamp(12.0, 26.0);
                    let body = Rect::from_center_size(canvas_rect.center(), Vec2::new(w_px, h_px));
                    painter.rect_filled(
                        body,
                        CornerRadius::same(2),
                        theme.accent_blue.gamma_multiply(0.28),
                    );
                    painter.rect_stroke(
                        body,
                        CornerRadius::same(2),
                        Stroke::new(1.1, theme.text_secondary),
                        egui::StrokeKind::Inside,
                    );
                    let sbe_w = (w_px * 0.16).max(8.0);
                    for sbe_x in [body.left(), body.right() - sbe_w] {
                        let sbe_rect = Rect::from_min_size(
                            egui::pos2(sbe_x, body.top()),
                            Vec2::new(sbe_w, h_px),
                        )
                        .shrink(1.8);
                        painter.rect_stroke(
                            sbe_rect,
                            CornerRadius::same(1),
                            Stroke::new(1.1, theme.accent_orange),
                            egui::StrokeKind::Inside,
                        );
                        for pt in [
                            sbe_rect.left_top(),
                            sbe_rect.right_top(),
                            sbe_rect.right_bottom(),
                            sbe_rect.left_bottom(),
                        ] {
                            painter.circle_filled(pt, 1.8, theme.accent_blue);
                        }
                    }
                    // Double-curtain web rebar dots along wall centerline
                    for i in 1..=3 {
                        let t = i as f32 / 4.0;
                        let wx = egui::lerp((body.left() + sbe_w)..=(body.right() - sbe_w), t);
                        painter.circle_filled(
                            egui::pos2(wx, body.top() + 3.0),
                            1.4,
                            theme.accent_green,
                        );
                        painter.circle_filled(
                            egui::pos2(wx, body.bottom() - 3.0),
                            1.4,
                            theme.accent_green,
                        );
                    }
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Wall L={:.2}m × t={:.0}mm",
                                wall.length_mm() * 0.001,
                                wall.thickness_mm
                            ))
                            .size(9.8)
                            .monospace()
                            .strong()
                            .color(theme.text_primary),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} • {} openings • SBE {}",
                                if wall.is_structural { "Shear Wall" } else { "Partition" },
                                wall.openings.len(),
                                if wall.is_structural { "Confined" } else { "Min" }
                            ))
                            .size(9.0)
                            .color(theme.accent_blue),
                        );
                    });
                });
            });
    }

    /// Renders a live 2D Foundation Footing plan with critical `d/2` punching-shear perimeter `b0`,
    /// 4 corner subsurface pile seats, and soil bearing utilization gauge inside the Smart Pop-Up.
    pub fn render_footing_punching_and_pile_preview(
        ui: &mut Ui,
        theme: &MacTheme,
        ft: &crate::core::bim::BimFooting,
        est_bearing_kpa: f64,
        qa_kpa: f64,
    ) {
        let avail_w = ui.available_width().max(150.0);
        Frame::new()
            .fill(theme.bg_sidebar)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(0.8, theme.border_subtle))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                ui.set_max_width((avail_w - 4.0).max(140.0));
                ui.horizontal(|ui| {
                    let (canvas_rect, _) =
                        ui.allocate_exact_size(Vec2::new(66.0, 66.0), egui::Sense::hover());
                    let painter = ui.painter_at(canvas_rect);
                    let max_d = ft.width_mm.max(ft.length_mm).max(400.0) as f32;
                    let sc = 50.0 / max_d;
                    let fw = (ft.width_mm as f32 * sc).clamp(24.0, 52.0);
                    let fl = (ft.length_mm as f32 * sc).clamp(24.0, 52.0);
                    let f_rect = Rect::from_center_size(canvas_rect.center(), Vec2::new(fw, fl));
                    painter.rect_filled(
                        f_rect,
                        CornerRadius::same(3),
                        theme.accent_orange.gamma_multiply(0.24),
                    );
                    painter.rect_stroke(
                        f_rect,
                        CornerRadius::same(3),
                        Stroke::new(1.2, theme.accent_orange),
                        egui::StrokeKind::Inside,
                    );
                    // 4 deep pile locations
                    let p_in = f_rect.shrink(6.0);
                    for corner in [
                        p_in.left_top(),
                        p_in.right_top(),
                        p_in.right_bottom(),
                        p_in.left_bottom(),
                    ] {
                        painter.circle_stroke(corner, 3.2, Stroke::new(1.2, theme.accent_blue));
                    }
                    // d/2 critical punching-shear perimeter & central column stub
                    let punch_rect = Rect::from_center_size(
                        canvas_rect.center(),
                        Vec2::new(fw * 0.48, fl * 0.48),
                    );
                    painter.rect_stroke(
                        punch_rect,
                        CornerRadius::same(2),
                        Stroke::new(1.1, theme.accent_green),
                        egui::StrokeKind::Inside,
                    );
                    let col_stub = Rect::from_center_size(
                        canvas_rect.center(),
                        Vec2::new((fw * 0.24).max(8.0), (fl * 0.24).max(8.0)),
                    );
                    painter.rect_filled(col_stub, CornerRadius::same(1), theme.text_secondary);

                    ui.vertical(|ui| {
                        let gauge_w = ui.available_width().clamp(72.0, 200.0);
                        ui.label(
                            RichText::new(format!(
                                "{:.0}×{:.0}×{:.0} mm",
                                ft.width_mm, ft.length_mm, ft.thickness_mm
                            ))
                            .size(9.8)
                            .monospace()
                            .strong()
                            .color(theme.text_primary),
                        );
                        let ratio = (est_bearing_kpa / qa_kpa.max(1.0)) as f32;
                        let col = if ratio > 0.95 {
                            theme.accent_red
                        } else if ratio > 0.75 {
                            theme.accent_orange
                        } else {
                            theme.accent_green
                        };
                        ui.label(
                            RichText::new(format!(
                                "qu/qa: {:.0}/{:.0} kPa ({:.0}%)",
                                est_bearing_kpa,
                                qa_kpa,
                                ratio * 100.0
                            ))
                            .size(9.0)
                            .monospace()
                            .color(col),
                        );
                        let (bar, _) =
                            ui.allocate_exact_size(Vec2::new(gauge_w, 7.0), egui::Sense::hover());
                        ui.painter().rect_filled(bar, CornerRadius::same(3), theme.bg_card);
                        ui.painter().rect_filled(
                            Rect::from_min_size(
                                bar.min,
                                Vec2::new((bar.width() * ratio.clamp(0.0, 1.0)).max(2.0), bar.height()),
                            ),
                            CornerRadius::same(3),
                            col,
                        );
                    });
                });
            });
    }

    /// Renders a live 2D Two-Way Slab reinforcement mesh & thickness summary diagram inside the Smart Pop-Up.
    pub fn render_slab_twoway_preview(
        ui: &mut Ui,
        theme: &MacTheme,
        slab: &crate::core::bim::BimSlab,
        area_m2: f64,
    ) {
        let avail_w = ui.available_width().max(150.0);
        Frame::new()
            .fill(theme.bg_sidebar)
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(0.8, theme.border_subtle))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                ui.set_max_width((avail_w - 4.0).max(140.0));
                ui.horizontal(|ui| {
                    let (canvas_rect, _) =
                        ui.allocate_exact_size(Vec2::new(66.0, 54.0), egui::Sense::hover());
                    let painter = ui.painter_at(canvas_rect);
                    let panel = Rect::from_center_size(canvas_rect.center(), Vec2::new(54.0, 42.0));
                    painter.rect_filled(
                        panel,
                        CornerRadius::same(3),
                        theme.accent_blue.gamma_multiply(0.22),
                    );
                    painter.rect_stroke(
                        panel,
                        CornerRadius::same(3),
                        Stroke::new(1.1, theme.text_secondary),
                        egui::StrokeKind::Inside,
                    );
                    for k in 1..=3 {
                        let f = k as f32 / 4.0;
                        let gx = egui::lerp(panel.left()..=panel.right(), f);
                        let gy = egui::lerp(panel.top()..=panel.bottom(), f);
                        painter.line_segment(
                            [egui::pos2(gx, panel.top() + 3.0), egui::pos2(gx, panel.bottom() - 3.0)],
                            Stroke::new(0.9, theme.accent_green.gamma_multiply(0.8)),
                        );
                        painter.line_segment(
                            [egui::pos2(panel.left() + 3.0, gy), egui::pos2(panel.right() - 3.0, gy)],
                            Stroke::new(0.9, theme.accent_blue.gamma_multiply(0.85)),
                        );
                    }
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} • h={:.0}mm",
                                if slab.is_roof { "Roof Slab" } else { "2-Way Slab" },
                                slab.thickness_mm
                            ))
                            .size(9.8)
                            .monospace()
                            .strong()
                            .color(theme.text_primary),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Area {:.1} m² • {} shafts",
                                area_m2,
                                slab.openings.len()
                            ))
                            .size(9.0)
                            .color(theme.accent_blue),
                        );
                    });
                });
            });
    }

    /// Renders a compact lens colorbar legend inside Dynamic Island whenever a structural
    /// visualization lens (`DCR%`, `ARI`, `Moment/Force`, `Deformed`, `Modal`, `4D`) is active.
    pub(super) fn render_active_lens_colorbar_strip(
        &self,
        ui: &mut Ui,
        theme: &MacTheme,
        max_dcr: f64,
        min_ari: f64,
        fem_cache: &crate::views::universal_canvas::EarthStructuralRenderCache,
        active_modal_mode: usize,
        panel_w: f32,
    ) {
        if self.active_lens == SpatialLens::Clean {
            return;
        }
        let strip_w = (panel_w - 8.0).max(160.0);
        Frame::new()
            .fill(theme.bg_sidebar.gamma_multiply(0.85))
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(0.8, theme.border_subtle))
            .inner_margin(Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.set_max_width(strip_w);
                ui.horizontal_wrapped(|ui| {
                    match self.active_lens {
                        SpatialLens::DcrHeatmap => {
                            ui.label(
                                RichText::new("DCR% Legend:")
                                    .size(9.3)
                                    .strong()
                                    .color(theme.text_secondary),
                            );
                            for (lbl, col) in [
                                ("<50% Optimal", theme.accent_green),
                                ("50–70% Mod", Color32::from_rgb(56, 189, 248)),
                                ("70–85% High", theme.accent_orange),
                                (">85% Crit", theme.accent_red),
                            ] {
                                ui.colored_label(col, RichText::new(format!("■ {lbl}")).size(9.2));
                            }
                            ui.label(
                                RichText::new(format!("Peak {max_dcr:.0}%"))
                                    .size(9.3)
                                    .monospace()
                                    .color(theme.text_primary),
                            );
                        }
                        SpatialLens::AriRedundancy => {
                            ui.label(
                                RichText::new("ARI Legend:")
                                    .size(9.3)
                                    .strong()
                                    .color(theme.text_secondary),
                            );
                            for (lbl, col) in [
                                ("<0.60 Fragile", theme.accent_red),
                                ("0.60–0.70 Warn", theme.accent_orange),
                                ("0.70–0.85 Good", Color32::from_rgb(56, 189, 248)),
                                (">0.85 Redundant", theme.accent_green),
                            ] {
                                ui.colored_label(col, RichText::new(format!("■ {lbl}")).size(9.2));
                            }
                            ui.label(
                                RichText::new(format!("Min {min_ari:.2}"))
                                    .size(9.3)
                                    .monospace()
                                    .color(theme.text_primary),
                            );
                        }
                        SpatialLens::AxialForce
                        | SpatialLens::ShearForce
                        | SpatialLens::BendingMoment => {
                            ui.label(
                                RichText::new("Force Diagrams:")
                                    .size(9.3)
                                    .strong()
                                    .color(theme.text_secondary),
                            );
                            ui.colored_label(
                                Color32::from_rgb(244, 63, 94),
                                RichText::new("■ Beam 3D BMD (kNm)").size(9.2),
                            );
                            ui.colored_label(
                                Color32::from_rgb(192, 132, 252),
                                RichText::new("■ Col Moment/Shear Fin").size(9.2),
                            );
                            ui.label(
                                RichText::new("3D Force Ribbon Active (kNm / kN)")
                                    .size(9.2)
                                    .monospace()
                                    .color(theme.text_primary),
                            );
                        }
                        SpatialLens::DeformedShape => {
                            ui.label(
                                RichText::new("6-DOF Deformed:")
                                    .size(9.3)
                                    .strong()
                                    .color(theme.text_secondary),
                            );
                            for (lbl, col) in [
                                ("Low", Color32::from_rgb(208, 235, 225)),
                                ("Mid", Color32::from_rgb(91, 146, 165)),
                                ("High", Color32::from_rgb(245, 158, 11)),
                                ("Peak Sway", Color32::from_rgb(244, 63, 94)),
                            ] {
                                ui.colored_label(col, RichText::new(format!("■ {lbl}")).size(9.2));
                            }
                            ui.label(
                                RichText::new(format!(
                                    "Δmax {:.1}mm • Drift {:.2}%",
                                    fem_cache.max_disp_mm,
                                    fem_cache.max_story_drift_ratio * 100.0
                                ))
                                .size(9.2)
                                .monospace()
                                .color(theme.text_primary),
                            );
                        }
                        SpatialLens::ModalVibration => {
                            ui.label(
                                RichText::new("Modal Eigenmode:")
                                    .size(9.3)
                                    .strong()
                                    .color(theme.text_secondary),
                            );
                            for (lbl, col) in [
                                ("Node Base", Color32::from_rgb(208, 235, 225)),
                                ("Mid Amp", Color32::from_rgb(91, 146, 165)),
                                ("Peak Mode", Color32::from_rgb(244, 63, 94)),
                            ] {
                                ui.colored_label(col, RichText::new(format!("■ {lbl}")).size(9.2));
                            }
                            let slot = active_modal_mode.min(fem_cache.modal_modes.len().saturating_sub(1));
                            if let Some(m) = fem_cache.modal_modes.get(slot) {
                                ui.label(
                                    RichText::new(format!(
                                        "M{} • T={:.2}s • f={:.2}Hz",
                                        m.mode_number, m.period_s, m.frequency_hz
                                    ))
                                    .size(9.2)
                                    .monospace()
                                    .color(theme.text_primary),
                                );
                            }
                        }
                        SpatialLens::Erection4D => {
                            ui.label(
                                RichText::new("4D Erection:")
                                    .size(9.3)
                                    .strong()
                                    .color(theme.text_secondary),
                            );
                            for (lbl, col) in [
                                ("Ground", Color32::from_rgb(16, 185, 129)),
                                ("Mid Levels", Color32::from_rgb(56, 189, 248)),
                                ("Upper / Roof", Color32::from_rgb(251, 191, 36)),
                            ] {
                                ui.colored_label(col, RichText::new(format!("■ {lbl}")).size(9.2));
                            }
                            ui.label(
                                RichText::new(format!("Day {} Timeline", self.active_4d_day))
                                    .size(9.3)
                                    .monospace()
                                    .color(theme.text_primary),
                            );
                        }
                        SpatialLens::Clean => {}
                    }
                });
            });
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_popup_ai_actions(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        elem_name: &str,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) {
        let prompts = [
            (
                format!("Audit @{elem_name} DCR & ARI"),
                format!("screen ari and p-m-v interaction for @{elem_name}"),
            ),
            (
                format!("Check N-1 Collapse Risk"),
                format!("n-2 cascading disruption check around @{elem_name}"),
            ),
            (
                format!("Recommend Bracing / Repair"),
                "recommend ari bracing".to_string(),
            ),
        ];
        for (btn_lbl, query) in prompts {
            if icons::icon_button(
                ui,
                AppIcon::Sparkles,
                btn_lbl,
                10.5,
                theme.text_primary,
                theme.bg_card,
                Stroke::new(1.0, theme.border_subtle),
                CornerRadius::same(8),
                Vec2::new(ui.available_width(), 26.0),
            )
            .clicked()
            {
                self.attach_context(AiContextAttachment::SelectedElement);
                self.command_bar_expanded = true;
                ai_assistant.execute_prompt(
                    &query,
                    NavigationPillar::AnalysisSimulation,
                    members,
                    projects,
                    db_components,
                );
                self.push_trace(format!("EigenAI audited @{elem_name}"), true);
            }
            ui.add_space(4.0);
        }
        if icons::icon_button(
            ui,
            AppIcon::Bot,
            format!("Add @{elem_name} to Antigravity AI Context"),
            10.5,
            Color32::WHITE,
            theme.accent_blue,
            Stroke::NONE,
            CornerRadius::same(8),
            Vec2::new(ui.available_width(), 26.0),
        )
        .clicked()
        {
            self.attach_context(AiContextAttachment::SelectedElement);
            self.command_bar_expanded = true;
            self.request_focus_command = true;
        }
    }

}
