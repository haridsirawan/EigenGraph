//! views/analyze/seismic_dynamic.rs — Seismic, Pushover, Geotech SSI, Base Isolation & Nonlinear Time-History.

use super::*;

impl AnalyzeView {
    pub(super) fn render_nonlinear_pushover(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        bim_project: &crate::core::BimProject,
    ) {
        let (model, is_live_bim) = Self::resolve_active_model(bim_project);
        let csm = crate::core::PushoverEngine::evaluate_csm(
            &model,
            self.site_class,
            self.ss_val as f64,
            self.s1_val as f64,
            self.pushover_load_pattern,
            15,
        );
        let report = &csm.raw_pushover;

        ui.horizontal_wrapped(|ui| {
            icons::pill_badge(
                ui,
                if is_live_bim { AppIcon::CheckCircle } else { AppIcon::Info },
                if is_live_bim { "Pushover Model: Live BIM Frame" } else { "Pushover Model: Reference Warren Truss" },
                if is_live_bim { theme.accent_green } else { theme.accent_blue },
            );
        });
        ui.add_space(8.0);

        // Top Summary Cards
        ui_kit::responsive_grid_n(ui, 4, 190.0, |ui, col_idx| match col_idx {
            0 => {
                self.metric_card(ui, theme, "PERFORMANCE POINT (Dt)", &format!("{:.2} mm", csm.performance_point.target_displacement_dt_mm), &format!("Base Shear: {:.1} kN", csm.performance_point.target_base_shear_vt_kn), theme.accent_blue);
            }
            1 => {
                let perf_color = match csm.performance_point.performance_level {
                        crate::core::SeismicPerformanceLevel::ImmediateOccupancy => theme.accent_blue,
                        crate::core::SeismicPerformanceLevel::LifeSafety => theme.accent_green,
                        crate::core::SeismicPerformanceLevel::CollapsePrevention => theme.accent_orange,
                        crate::core::SeismicPerformanceLevel::CollapseExceeded => Color32::from_rgb(239, 68, 68),
                    };
                    self.metric_card(ui, theme, "SEISMIC AUDIT LEVEL", match csm.performance_point.performance_level {
                        crate::core::SeismicPerformanceLevel::ImmediateOccupancy => "Immediate Occupancy",
                        crate::core::SeismicPerformanceLevel::LifeSafety => "Life Safety (LS)",
                        crate::core::SeismicPerformanceLevel::CollapsePrevention => "Collapse Prevention",
                        crate::core::SeismicPerformanceLevel::CollapseExceeded => "Collapse Exceeded",
                    }, &format!("Margin: {:.1}%", csm.seismic_safety_margin_pct), perf_color);
            }
            2 => {
                self.metric_card(ui, theme, "EFFECTIVE DAMPING (βeff)", &format!("{:.1}%", csm.performance_point.effective_damping_pct), &format!("Secant Period: {:.2}s", csm.performance_point.effective_period_teff_s), theme.accent_orange);
            }
            _ => {
                self.metric_card(ui, theme, "DUCTILITY DEMAND (µ)", &format!("{:.2}", csm.performance_point.ductility_demand_mu), &format!("Max Drift: {:.1} mm", report.ultimate_drift_mm), theme.accent_blue);
            }
        });

        ui.add_space(12.0);

        // Control Toolbar: Lateral Load Pattern & Seismic Site Parameters
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Lateral Load Pattern:").size(11.0).strong().color(theme.text_secondary));
                    let patterns = [
                        (crate::core::LateralLoadPattern::FirstModeModal, "Mode 1 (Modal)"),
                        (crate::core::LateralLoadPattern::UniformMass, "Uniform Mass"),
                        (crate::core::LateralLoadPattern::InvertedTriangular, "Inverted Triangular"),
                    ];
                    for (pat, lbl) in patterns {
                        let is_sel = self.pushover_load_pattern == pat;
                        let btn = egui::Button::new(RichText::new(lbl).size(11.0).color(if is_sel { Color32::WHITE } else { theme.text_secondary }))
                            .fill(if is_sel { Color32::from_rgb(37, 99, 235) } else { theme.bg_canvas });
                        if ui.add(btn).clicked() {
                            self.pushover_load_pattern = pat;
                        }
                    }

                    ui.add_space(14.0);
                    ui.label(RichText::new("Site Class:").size(11.0).strong().color(theme.text_secondary));
                    for (sc, lbl) in [(crate::core::SiteClass::SB, "SB"), (crate::core::SiteClass::SC, "SC"), (crate::core::SiteClass::SD, "SD"), (crate::core::SiteClass::SE, "SE")] {
                        let is_sel = self.site_class == sc;
                        let btn = egui::Button::new(RichText::new(lbl).size(11.0).color(if is_sel { Color32::WHITE } else { theme.text_secondary }))
                            .fill(if is_sel { theme.accent_blue } else { theme.bg_canvas });
                        if ui.add(btn).clicked() {
                            self.site_class = sc;
                        }
                    }

                    ui.add_space(14.0);
                    ui.label(RichText::new("Ss (g):").size(11.0).color(theme.text_muted));
                    ui.add(egui::DragValue::new(&mut self.ss_val).speed(0.02).range(0.1..=2.5));
                    ui.label(RichText::new("S1 (g):").size(11.0).color(theme.text_muted));
                    ui.add(egui::DragValue::new(&mut self.s1_val).speed(0.02).range(0.05..=1.5));
                });
            });

        ui.add_space(12.0);

        // Capacity Curve Table & Dual Plot Area
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::icon_label(ui, AppIcon::ChartLine, 14.0, theme.accent_blue, "Capacity Spectrum Method (ATC-40 / ASCE 41-17) & Performance Point", 13.5, theme.text_primary, true);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (status_icon, status_tag, status_col) = if csm.is_code_compliant {
                            (AppIcon::CheckCircle, "Life Safety (LS) Compliant", theme.accent_green)
                        } else {
                            (AppIcon::AlertTriangle, "Collapse Exceeded Limit", Color32::from_rgb(239, 68, 68))
                        };
                        icons::pill_badge(ui, status_icon, status_tag, status_col);
                    });
                });

                ui.add_space(10.0);

                // DUAL VECTOR VISUALIZER (Left: V_base vs Drift, Right: ADRS Sa vs Sd)
                let pad_l = 42.0;
                let pad_r = 16.0;
                let pad_t = 16.0;
                let pad_b = 22.0;
                ui_kit::responsive_two_panes(ui, 720.0, |ui, pane_idx| match pane_idx {
                    0 => {
                        // LEFT: V_base vs Drift Capacity Curve
                        ui.label(RichText::new("1. Base Shear vs Roof Drift (Pushover Curve)").size(11.0).strong().color(theme.text_secondary));
                        let desired_left = Vec2::new(ui.available_width(), 160.0);
                        let (rect_l, _) = ui.allocate_exact_size(desired_left, Sense::hover());
                        let painter_l = ui.painter_at(rect_l);
                        painter_l.rect_filled(rect_l, CornerRadius::same(6), Color32::from_rgb(12, 16, 23));

                                    let plot_wl = (rect_l.width() - pad_l - pad_r).max(40.0);
                                    let plot_hl = (rect_l.height() - pad_t - pad_b).max(30.0);
                                    let x0_l = rect_l.left() + pad_l;
                                    let y0_l = rect_l.bottom() - pad_b;

                                    for i in 0..=3 {
                                        let y = y0_l - (i as f32 / 3.0) * plot_hl;
                                        painter_l.line_segment([Pos2::new(x0_l, y), Pos2::new(x0_l + plot_wl, y)], Stroke::new(0.8, Color32::from_white_alpha(18)));
                                    }

                                    let max_drift = report.ultimate_drift_mm.max(1.0) as f32 * 1.15;
                                    let max_shear = report.ultimate_base_shear_kn.max(10.0) as f32 * 1.15;
                                    let mut pts_l = Vec::with_capacity(report.steps.len() + 1);
                                    pts_l.push(Pos2::new(x0_l, y0_l));
                                    for st in &report.steps {
                                        let px = x0_l + ((st.roof_drift_mm as f32) / max_drift).clamp(0.0, 1.0) * plot_wl;
                                        let py = y0_l - ((st.base_shear_kn as f32) / max_shear).clamp(0.0, 1.0) * plot_hl;
                                        pts_l.push(Pos2::new(px, py));
                                    }
                                    for w in pts_l.windows(2) {
                                        painter_l.line_segment([w[0], w[1]], Stroke::new(2.2, theme.accent_green));
                                    }
                                    for &pt in pts_l.iter().skip(1) {
                                        painter_l.circle_filled(pt, 2.5, theme.accent_blue);
                                    }

                                    // Target Dt Reticle
                                    let dt_px = x0_l + ((csm.performance_point.target_displacement_dt_mm as f32) / max_drift).clamp(0.0, 1.0) * plot_wl;
                                    let vt_py = y0_l - ((csm.performance_point.target_base_shear_vt_kn as f32) / max_shear).clamp(0.0, 1.0) * plot_hl;
                                    let perf_pt_l = Pos2::new(dt_px, vt_py);
                                    painter_l.line_segment([Pos2::new(dt_px, y0_l), Pos2::new(dt_px, y0_l - plot_hl)], Stroke::new(1.0, Color32::from_rgba_premultiplied(14, 165, 233, 140)));
                                    painter_l.circle_filled(perf_pt_l, 4.5, Color32::from_rgb(14, 165, 233));
                                    painter_l.circle_stroke(perf_pt_l, 6.0, Stroke::new(1.2, Color32::WHITE));
                                    painter_l.text(perf_pt_l + Vec2::new(6.0, -8.0), egui::Align2::LEFT_BOTTOM, format!("Dt={:.1}mm", csm.performance_point.target_displacement_dt_mm), egui::FontId::proportional(9.5), Color32::WHITE);

                                    // Labels
                                    painter_l.text(Pos2::new(x0_l, rect_l.top() + 6.0), egui::Align2::LEFT_TOP, "V_base (kN)", egui::FontId::proportional(9.5), theme.text_muted);
                                    painter_l.text(Pos2::new(rect_l.right() - pad_r, y0_l + 6.0), egui::Align2::RIGHT_TOP, "Roof Drift (mm)", egui::FontId::proportional(9.5), theme.text_muted);
                    }
                    _ => {
                        // RIGHT: ADRS Sa vs Sd Capacity Spectrum
                        ui.label(RichText::new("2. Acceleration-Displacement Response Spectrum (ADRS)").size(11.0).strong().color(theme.text_secondary));
                                    let desired_right = Vec2::new(ui.available_width(), 160.0);
                                    let (rect_r, _) = ui.allocate_exact_size(desired_right, Sense::hover());
                                    let painter_r = ui.painter_at(rect_r);
                                    painter_r.rect_filled(rect_r, CornerRadius::same(6), Color32::from_rgb(12, 16, 23));

                                    let plot_wr = (rect_r.width() - pad_l - pad_r).max(40.0);
                                    let plot_hr = (rect_r.height() - pad_t - pad_b).max(30.0);
                                    let x0_r = rect_r.left() + pad_l;
                                    let y0_r = rect_r.bottom() - pad_b;

                                    for i in 0..=3 {
                                        let y = y0_r - (i as f32 / 3.0) * plot_hr;
                                        painter_r.line_segment([Pos2::new(x0_r, y), Pos2::new(x0_r + plot_wr, y)], Stroke::new(0.8, Color32::from_white_alpha(18)));
                                    }

                                    let max_sd = (csm.capacity_spectrum.peak_point_adrs.sd_m * 1.35).max(0.15) as f32;
                                    let max_sa = (csm.demand_spectrum.elastic_5pct_points.iter().map(|p| p.sa_g).fold(0.5, f64::max) * 1.25) as f32;

                                    // Draw Elastic Demand (Red Dashed)
                                    let mut pts_el = Vec::new();
                                    for p in &csm.demand_spectrum.elastic_5pct_points {
                                        let px = x0_r + ((p.sd_m as f32) / max_sd).clamp(0.0, 1.2) * plot_wr;
                                        let py = y0_r - ((p.sa_g as f32) / max_sa).clamp(0.0, 1.2) * plot_hr;
                                        pts_el.push(Pos2::new(px, py));
                                    }
                                    for w in pts_el.windows(2) {
                                        painter_r.line_segment([w[0], w[1]], Stroke::new(1.2, Color32::from_rgb(248, 113, 113)));
                                    }

                                    // Draw Reduced Inelastic Demand (Amber)
                                    let mut pts_red = Vec::new();
                                    for p in &csm.demand_spectrum.reduced_inelastic_points {
                                        let px = x0_r + ((p.sd_m as f32) / max_sd).clamp(0.0, 1.2) * plot_wr;
                                        let py = y0_r - ((p.sa_g as f32) / max_sa).clamp(0.0, 1.2) * plot_hr;
                                        pts_red.push(Pos2::new(px, py));
                                    }
                                    for w in pts_red.windows(2) {
                                        painter_r.line_segment([w[0], w[1]], Stroke::new(1.6, Color32::from_rgb(245, 158, 11)));
                                    }

                                    // Draw Capacity Spectrum (Solid Emerald)
                                    let mut pts_cap = Vec::new();
                                    pts_cap.push(Pos2::new(x0_r, y0_r));
                                    for p in &csm.capacity_spectrum.points {
                                        let px = x0_r + ((p.sd_m as f32) / max_sd).clamp(0.0, 1.2) * plot_wr;
                                        let py = y0_r - ((p.sa_g as f32) / max_sa).clamp(0.0, 1.2) * plot_hr;
                                        pts_cap.push(Pos2::new(px, py));
                                    }
                                    for w in pts_cap.windows(2) {
                                        painter_r.line_segment([w[0], w[1]], Stroke::new(2.4, theme.accent_green));
                                    }

                                    // Performance Point (Sd*, Sa*)
                                    let pt_x = x0_r + ((csm.performance_point.sd_star_m as f32) / max_sd).clamp(0.0, 1.0) * plot_wr;
                                    let pt_y = y0_r - ((csm.performance_point.sa_star_g as f32) / max_sa).clamp(0.0, 1.0) * plot_hr;
                                    let p_star = Pos2::new(pt_x, pt_y);
                                    painter_r.circle_filled(p_star, 5.0, Color32::from_rgb(56, 189, 248));
                                    painter_r.circle_stroke(p_star, 8.0, Stroke::new(1.4, Color32::WHITE));
                                    painter_r.text(p_star + Vec2::new(8.0, -6.0), egui::Align2::LEFT_BOTTOM, format!("P* ({:.3}m, {:.2}g)", csm.performance_point.sd_star_m, csm.performance_point.sa_star_g), egui::FontId::proportional(9.5), Color32::from_rgb(56, 189, 248));

                                    // Legend
                                    painter_r.text(Pos2::new(x0_r, rect_r.top() + 6.0), egui::Align2::LEFT_TOP, "Sa (g)", egui::FontId::proportional(9.5), theme.text_muted);
                                    painter_r.text(Pos2::new(rect_r.right() - pad_r, y0_r + 6.0), egui::Align2::RIGHT_TOP, "Sd (m)", egui::FontId::proportional(9.5), theme.text_muted);
                    }
                });

                ui.add_space(14.0);

                // Plastic Hinge Distribution Table (ASCE 41-17 Performance Envelope)
                Frame::new()
                    .fill(Color32::from_rgb(15, 23, 42))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Inelastic Plastic Hinge Matrix at Performance Point:").size(11.0).strong().color(Color32::WHITE));
                            ui.add_space(10.0);
                            for (lbl, col) in [
                                (format!("Elastic: {} Elements", csm.hinges_elastic), theme.accent_green),
                                (format!("IO: {} Elements", csm.hinges_io), theme.accent_blue),
                                (format!("LS: {} Elements", csm.hinges_ls), theme.accent_orange),
                                (format!("CP: {} Elements", csm.hinges_cp), Color32::from_rgb(255, 69, 58)),
                                (format!("Mech: {} Elements", csm.hinges_mechanism), Color32::from_rgb(191, 90, 242)),
                            ] {
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 4.0, col);
                                ui.label(RichText::new(lbl).size(10.5).color(col));
                                ui.add_space(4.0);
                            }
                        });
                        ui.add_space(4.0);
                        ui.label(RichText::new(&csm.performance_point.compliance_audit).size(11.0).strong().color(Color32::from_rgb(226, 232, 240)));
                    });

                ui.add_space(12.0);

                // Step Progression
                ui.label(RichText::new("Lateral Force Increment Stages & Plastic Hinge Formation History (FEMA 356 / ASCE 41):").size(11.5).color(theme.text_secondary));
                ui.add_space(6.0);

                for step in &report.steps {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Step {:02} (λ = {:.2}x)", step.step, step.load_factor)).size(11.5).strong().color(theme.text_primary));
                        ui.add_space(12.0);
                        ui.label(RichText::new(format!("Base Shear: {:6.1} kN", step.base_shear_kn)).size(11.5).color(theme.text_secondary));
                        ui.add_space(12.0);
                        ui.label(RichText::new(format!("Roof Drift: {:5.2} mm", step.roof_drift_mm)).size(11.5).color(theme.text_secondary));
                        ui.add_space(12.0);
                        ui.label(RichText::new(format!("Plastic Hinges: {} Locations", step.active_hinges_count)).size(11.5).color(theme.accent_blue));

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let badge_col = match step.state_summary {
                                crate::core::PlasticHingeState::Elastic => theme.accent_green,
                                crate::core::PlasticHingeState::ImmediateOccupancy => theme.accent_blue,
                                crate::core::PlasticHingeState::LifeSafety => theme.accent_orange,
                                crate::core::PlasticHingeState::CollapsePrevention => Color32::from_rgb(255, 69, 58),
                                crate::core::PlasticHingeState::Mechanism => Color32::from_rgb(191, 90, 242),
                            };
                            ui.label(RichText::new(step.state_summary.label()).size(11.0).strong().color(badge_col));
                        });
                    });
                    ui.separator();
                }

                ui.add_space(14.0);

                // Sherman-Morrison / Woodbury Non-Linear Post-Yield Redistribution & AI Advisor
                let woodbury_rep = crate::core::NonlinearEngine::evaluate_woodbury_disruption(
                    &model,
                    self.woodbury_disrupt_elem_id,
                    self.woodbury_lambda_val as f64,
                );

                Frame::new()
                    .fill(Color32::from_rgb(17, 24, 39))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(12))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(37, 99, 235)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            icons::icon_label(ui, AppIcon::NodesGraph, 13.0, Color32::WHITE, "Sherman-Morrison / Woodbury Non-Linear Redistribution & AI Advisor (Eqs. 4.13d–4.13g):", 12.5, Color32::WHITE, true);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let (badge_icon, badge_txt, badge_c) = if woodbury_rep.is_mechanism {
                                    (AppIcon::AlertTriangle, "KINEMATIC MECHANISM", Color32::from_rgb(239, 68, 68))
                                } else if woodbury_rep.non_linear_ci >= 0.35 {
                                    (AppIcon::AlertTriangle, "CRITICAL POST-YIELD", Color32::from_rgb(245, 158, 11))
                                } else {
                                    (AppIcon::CheckCircle, "REDUNDANT PATH CLEAR", theme.accent_green)
                                };
                                icons::pill_badge(ui, badge_icon, badge_txt, badge_c);
                            });
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Target Element ID:").size(11.0).color(theme.text_secondary));
                            ui.add(egui::Slider::new(&mut self.woodbury_disrupt_elem_id, 1..=model.elements.len().max(4)).step_by(1.0));
                            ui.add_space(12.0);
                            ui.label(RichText::new("Residual Capacity λ:").size(11.0).color(theme.text_secondary));
                            ui.add(egui::Slider::new(&mut self.woodbury_lambda_val, 0.0..=1.0).step_by(0.1));
                        });
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(format!("Stability Margin ms: {:.3}", woodbury_rep.stability_margin_ms)).size(11.0).strong().color(theme.accent_blue));
                            ui.label(RichText::new("•").size(11.0).color(theme.text_muted));
                            ui.label(RichText::new(format!("Redistributed Force ΔN: {:.2} kN", woodbury_rep.redistributed_force_delta_n_kn)).size(11.0).strong().color(theme.accent_orange));
                            ui.label(RichText::new("•").size(11.0).color(theme.text_muted));
                            ui.label(RichText::new(format!("Ductility Demand μ: {:.2}", woodbury_rep.ductility_demand_mu)).size(11.0).strong().color(Color32::from_rgb(168, 85, 247)));
                            ui.label(RichText::new("•").size(11.0).color(theme.text_muted));
                            ui.label(RichText::new(format!("EigenFlow: {:.4} (CI: {:.4})", woodbury_rep.non_linear_eigenflow, woodbury_rep.non_linear_ci)).size(11.0).strong().color(theme.accent_green));
                        });
                        ui.add_space(6.0);
                        ui.label(RichText::new(&woodbury_rep.prescriptive_recommendation).size(11.5).strong().color(Color32::from_rgb(253, 224, 71)));
                    });
            });
    }

    pub(super) fn render_modal_dynamics(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        bim_project: &crate::core::BimProject,
    ) {
        let (model, is_live_bim) = Self::resolve_active_model(bim_project);
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    icons::icon_label(ui, AppIcon::Pulse, 15.0, theme.accent_blue, "Dynamic Modal Analysis & Seismic Spectrum (SNI 1726 / ASCE 7)", 13.5, theme.text_primary, true);
                    icons::pill_badge(
                        ui,
                        if is_live_bim { AppIcon::CheckCircle } else { AppIcon::Info },
                        if is_live_bim { "Live BIM Frame" } else { "Reference Warren Truss" },
                        if is_live_bim { theme.accent_green } else { theme.accent_blue },
                    );
                    ui.label(RichText::new("Eigenvalue: K*phi = omega^2*M*phi").size(11.0).color(theme.accent_blue));
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Parameter Input Row (Site Class, Ss, S1, R, Ie)
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Site Class:").size(11.5).strong().color(theme.text_secondary));
                    let classes = [
                        (crate::core::SiteClass::SB, "SB (Rock)"),
                        (crate::core::SiteClass::SC, "SC (Dense/Soft Rock)"),
                        (crate::core::SiteClass::SD, "SD (Stiff/Standard)"),
                        (crate::core::SiteClass::SE, "SE (Soft Clay)"),
                    ];
                    for (sc, sc_lbl) in classes {
                        let is_sel = self.site_class == sc;
                        if ui.selectable_label(is_sel, sc_lbl).clicked() {
                            self.site_class = sc;
                        }
                    }

                    ui.add_space(12.0);
                    ui.label(RichText::new("Ss:").size(11.5).color(theme.text_secondary));
                    ui.add(egui::DragValue::new(&mut self.ss_val).speed(0.01).range(0.1..=2.5).suffix(" g"));

                    ui.add_space(8.0);
                    ui.label(RichText::new("S1:").size(11.5).color(theme.text_secondary));
                    ui.add(egui::DragValue::new(&mut self.s1_val).speed(0.01).range(0.05..=1.5).suffix(" g"));

                    ui.add_space(8.0);
                    ui.label(RichText::new("R:").size(11.5).color(theme.text_secondary));
                    ui.add(egui::DragValue::new(&mut self.r_val).speed(0.5).range(1.0..=10.0));

                    ui.add_space(12.0);
                    if icons::icon_button(
                        ui,
                        AppIcon::Bolt,
                        "Compute Modes & Seismic",
                        11.5,
                        Color32::WHITE,
                        theme.accent_blue,
                        Stroke::NONE,
                        CornerRadius::same(6),
                        Vec2::ZERO,
                    ).clicked() {
                        let spec = crate::core::modal::Sni1726Spectrum::new(self.site_class, self.ss_val as f64, self.s1_val as f64, self.r_val as f64, 1.0);
                        let res = crate::core::modal::ModalEngine::analyze(&model, 4, spec.clone()).or_else(|_| {
                            let mut eng = crate::core::ParametricEngine::new();
                            let fb = eng.generate_warren_truss()?;
                            crate::core::modal::ModalEngine::analyze(&fb, 4, spec)
                        });
                        self.modal_report = res.ok();
                    }
                });

                ui.add_space(12.0);

                // Auto-run if not yet computed
                if self.modal_report.is_none() {
                    let spec = crate::core::modal::Sni1726Spectrum::new(self.site_class, self.ss_val as f64, self.s1_val as f64, self.r_val as f64, 1.0);
                    let res = crate::core::modal::ModalEngine::analyze(&model, 4, spec.clone()).or_else(|_| {
                        let mut eng = crate::core::ParametricEngine::new();
                        let fb = eng.generate_warren_truss()?;
                        crate::core::modal::ModalEngine::analyze(&fb, 4, spec)
                    });
                    self.modal_report = res.ok();
                }

                if let Some(ref report) = self.modal_report {
                    // Summary Stat Cards Row (4 Columns)
                    ui_kit::responsive_grid_n(ui, 4, 190.0, |ui, col_idx| match col_idx {
                        0 => {
                            self.render_stat_card(ui, theme, "FUNDAMENTAL PERIOD (T1)", &format!("{:.4} s", report.fundamental_period_s), &format!("Frequency: {:.2} Hz", report.fundamental_frequency_hz), theme.accent_blue);
                        }
                        1 => {
                            self.render_stat_card(ui, theme, "SEISMIC BASE SHEAR (V_b)", &format!("{:.1} kN", report.seismic_base_shear_kn), &format!("Cs = {:.4} • W = {:.1} kN", report.spectrum.cs, report.total_weight_kn), theme.accent_green);
                        }
                        2 => {
                            self.render_stat_card(ui, theme, "DESIGN SPECTRAL ACCEL (SDS)", &format!("{:.3} g", report.spectrum.sds), &format!("SD1: {:.3} g • Ts: {:.2} s", report.spectrum.sd1, report.spectrum.ts), theme.text_primary);
                        }
                        _ => {
                            let badge_col = if report.is_90_percent_satisfied { theme.accent_green } else { theme.accent_orange };
                                            self.render_stat_card(ui, theme, "MODAL MASS PARTICIPATION", &format!("{:.1}%", report.cumulative_mass_ratio_x.max(report.cumulative_mass_ratio_y)), "Code Standard: Min. 90%", badge_col);
                        }
                    });

                    ui.add_space(14.0);

                    // LIVE VECTOR SNI 1726:2019 DESIGN RESPONSE SPECTRUM CURVE (Sa vs T)
                    let desired = Vec2::new(ui.available_width(), 140.0);
                    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(12, 16, 23));

                    let pad_l = 38.0;
                    let pad_r = 16.0;
                    let pad_t = 14.0;
                    let pad_b = 20.0;
                    let plot_w = (rect.width() - pad_l - pad_r).max(50.0);
                    let plot_h = (rect.height() - pad_t - pad_b).max(40.0);
                    let x0 = rect.left() + pad_l;
                    let y0 = rect.bottom() - pad_b;

                    let max_t = 3.0f32;
                    let max_sa = (report.spectrum.sds as f32 * 1.25).max(0.5);
                    let mut pts = Vec::with_capacity(61);
                    for i in 0..=60 {
                        let t = (i as f32 / 60.0) * max_t;
                        let sa = report.spectrum.spectral_acceleration(t as f64) as f32;
                        let px = x0 + (t / max_t) * plot_w;
                        let py = y0 - (sa / max_sa).clamp(0.0, 1.0) * plot_h;
                        pts.push(Pos2::new(px, py));
                    }
                    for w in pts.windows(2) {
                        painter.line_segment([w[0], w[1]], Stroke::new(2.2, theme.accent_blue));
                    }
                    // Fundamental period T1 vertical marker
                    let t1_x = x0 + ((report.fundamental_period_s as f32) / max_t).clamp(0.0, 1.0) * plot_w;
                    painter.line_segment(
                        [Pos2::new(t1_x, y0 - plot_h), Pos2::new(t1_x, y0)],
                        Stroke::new(1.4, theme.accent_orange),
                    );

                    ui.add_space(16.0);

                    // Mode Shapes Table
                    icons::icon_label(ui, AppIcon::Clipboard, 13.5, theme.accent_blue, "Natural Vibration Modes Characteristics Table:", 12.5, theme.text_primary, true);
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Mode").size(11.0).strong().color(theme.text_secondary));
                        ui.add_space(30.0);
                        ui.label(RichText::new("Angular Frequency (ω)").size(11.0).strong().color(theme.text_secondary));
                        ui.add_space(40.0);
                        ui.label(RichText::new("Frequency (f)").size(11.0).strong().color(theme.text_secondary));
                        ui.add_space(40.0);
                        ui.label(RichText::new("Period (T)").size(11.0).strong().color(theme.text_secondary));
                        ui.add_space(40.0);
                        ui.label(RichText::new("Participation X (%)").size(11.0).strong().color(theme.text_secondary));
                        ui.add_space(40.0);
                        ui.label(RichText::new("Participation Y (%)").size(11.0).strong().color(theme.text_secondary));
                    });
                    ui.separator();

                    for mode in &report.modes {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Mode {}", mode.mode_number)).size(11.5).strong().color(theme.accent_blue));
                            ui.add_space(45.0);
                            ui.label(RichText::new(format!("{:.2} rad/s", mode.omega_rad_s)).size(11.5).color(theme.text_primary));
                            ui.add_space(60.0);
                            ui.label(RichText::new(format!("{:.3} Hz", mode.frequency_hz)).size(11.5).strong().color(theme.text_primary));
                            ui.add_space(65.0);
                            ui.label(RichText::new(format!("{:.4} s", mode.period_s)).size(11.5).strong().color(theme.accent_green));
                            ui.add_space(65.0);
                            ui.label(RichText::new(format!("{:.2}%", mode.mass_participation_ratio_x)).size(11.5).color(theme.text_secondary));
                            ui.add_space(65.0);
                            ui.label(RichText::new(format!("{:.2}%", mode.mass_participation_ratio_y)).size(11.5).color(theme.text_secondary));
                        });
                        ui.separator();
                    }

                    // CQC vs. SRSS Modal Combination Comparison Card (SNI 1726:2019 Pasal 7.9.3 & ASCE 7-16 Section 12.9.1)
                    let modal_comb = crate::core::modal::ModalEngine::evaluate_modal_combinations(
                        report,
                        self.modal_damping_ratio as f64,
                    );

                    ui.add_space(14.0);
                    Frame::new()
                        .fill(Color32::from_rgb(17, 24, 39))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(12))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(14, 165, 233)))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                icons::icon_label(ui, AppIcon::ChartBar, 13.0, Color32::WHITE, "Dynamic Modal Combinations & 85% Static Scaling (SNI 1726:2019 Pasal 7.9.3 & ASCE 7-16):", 12.5, Color32::WHITE, true);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let (scale_icon, scale_badge, badge_color) = if modal_comb.scale_factor_85pct > 1.0 {
                                        (AppIcon::AlertTriangle, "SCALED TO 85% V_STATIC", Color32::from_rgb(245, 158, 11))
                                    } else {
                                        (AppIcon::CheckCircle, "DYNAMIC Vt ≥ 0.85 Vs", theme.accent_green)
                                    };
                                    icons::pill_badge(ui, scale_icon, scale_badge, badge_color);
                                });
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Damping Ratio ξ:").size(11.0).color(theme.text_secondary));
                                ui.add(egui::Slider::new(&mut self.modal_damping_ratio, 0.01..=0.10).step_by(0.01).suffix(" (5% RC)"));
                                ui.add_space(16.0);
                                ui.label(RichText::new(format!("SRSS Base Shear: {:.1} kN", modal_comb.srss_base_shear_kn)).size(11.0).color(theme.text_secondary));
                                ui.add_space(10.0);
                                ui.label(RichText::new(format!("CQC Base Shear: {:.1} kN", modal_comb.cqc_base_shear_kn)).size(11.0).strong().color(theme.accent_blue));
                                ui.add_space(10.0);
                                ui.label(RichText::new(format!("Static Base Shear Vs: {:.1} kN", modal_comb.static_base_shear_kn)).size(11.0).color(theme.text_secondary));
                                ui.add_space(10.0);
                                ui.label(RichText::new(format!("Governing Design V: {:.1} kN (Scale: {:.3}x)", modal_comb.governing_design_base_shear_kn, modal_comb.scale_factor_85pct)).size(11.0).strong().color(theme.accent_green));
                            });
                            if !modal_comb.correlation_matrix_rho.is_empty() {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Der Kiureghian Correlation ρ₁₂:").size(10.5).color(theme.text_muted));
                                    let rho12 = modal_comb.correlation_matrix_rho.get(0).and_then(|r| r.get(1)).copied().unwrap_or(0.0);
                                    let rho13 = modal_comb.correlation_matrix_rho.get(0).and_then(|r| r.get(2)).copied().unwrap_or(0.0);
                                    ui.label(RichText::new(format!("{:.4}", rho12)).size(10.5).strong().color(theme.accent_blue));
                                    ui.add_space(10.0);
                                    ui.label(RichText::new("ρ₁₃:").size(10.5).color(theme.text_muted));
                                    ui.label(RichText::new(format!("{:.4}", rho13)).size(10.5).strong().color(theme.accent_blue));
                                    ui.add_space(10.0);
                                    ui.label(RichText::new(format!("Code Min. 0.85*Vs: {:.1} kN", modal_comb.min_85pct_threshold_kn)).size(10.5).color(theme.text_muted));
                                });
                            }
                        });
                }
            });
    }

    // -------------------------------------------------------------
    // SUB-TAB 5: Concrete Column P-M Interaction (SNI 2847:2019 / ACI 318-19)
    // -------------------------------------------------------------

    // -------------------------------------------------------------
    // SUB-TAB 9: Geotechnical Engineering, Deep Foundations & SSI
    // -------------------------------------------------------------
    pub(super) fn render_geotech_ssi(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        bim_project: &crate::core::BimProject,
    ) {
        let (model, is_live_bim) = Self::resolve_active_model(bim_project);

        ui.horizontal_wrapped(|ui| {
            icons::pill_badge(
                ui,
                if is_live_bim { AppIcon::CheckCircle } else { AppIcon::Info },
                if is_live_bim { "Geotech SSI Model: Live BIM Frame" } else { "Geotech SSI Model: Reference Warren Truss" },
                if is_live_bim { theme.accent_green } else { theme.accent_blue },
            );
        });
        ui.add_space(8.0);

        let soil_layer = crate::core::SoilLayer::new_standard(self.geotech_soil_type, 0.0, 20.0);
        let audit = crate::core::GeotechEngine::run_geotech_ssi_audit(
            &model,
            soil_layer.clone(),
            self.geotech_footing_w as f64,
            self.geotech_footing_l as f64,
            self.geotech_piles_x,
            self.geotech_piles_y,
        );

        let piles = &audit.pile_group;
        let punch = &audit.punching_shear;
        let ssi = &audit.ssi_modal_shift;
        let winkler = &audit.winkler_springs;

        // 1. Top Summary Metric Cards (4 Columns)
        ui_kit::responsive_grid_n(ui, 4, 190.0, |ui, col_idx| match col_idx {
            0 => {
                self.metric_card(
                        ui,
                        theme,
                        "SUBGRADE MODULUS (ks)",
                        &format!("{:.0} kN/m³", soil_layer.subgrade_reaction_ks_kn_m3),
                        &format!("Vertical Kz: {:.1} MN/m", winkler.kz_trans_kn_m / 1000.0),
                        theme.accent_blue,
                    );
            }
            1 => {
                self.metric_card(
                        ui,
                        theme,
                        "CONVERSE-LABARRE (Eg)",
                        &format!("{:.3} ({:.1}%)", piles.converse_labarre_efficiency, piles.converse_labarre_efficiency * 100.0),
                        &format!("{} Piles ({}x{}) • DCR: {:.3}", piles.total_piles, piles.piles_count_x, piles.piles_count_y, piles.governing_dcr),
                        if piles.is_group_safe { theme.accent_green } else { Color32::from_rgb(239, 68, 68) },
                    );
            }
            2 => {
                self.metric_card(
                        ui,
                        theme,
                        "PUNCHING SHEAR (SNI 22.6)",
                        &format!("DCR: {:.3}", punch.punching_dcr),
                        &format!("vu: {:.2} MPa ≤ φVc: {:.2} MPa", punch.punching_stress_vu_mpa, punch.concrete_capacity_phi_vc_mpa),
                        if punch.is_punching_safe { theme.accent_green } else { Color32::from_rgb(239, 68, 68) },
                    );
            }
            _ => {
                self.metric_card(
                        ui,
                        theme,
                        "SSI DYNAMIC SHIFT (T̃/T)",
                        &format!("{:.3}x", ssi.period_lengthening_ratio),
                        &format!("Base Shear: -{:.1}% (βssi: {:.1}%)", (1.0 - ssi.base_shear_reduction_ratio) * 100.0, ssi.foundation_damping_factor * 100.0),
                        theme.accent_orange,
                    );
            }
        });

        ui.add_space(12.0);

        // 2. Control Toolbar: Soil Stratum & Footing/Pile Geometry Configuration
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Stratum Stratigraphy:").size(11.0).strong().color(theme.text_secondary));
                    let soil_types = [
                        (crate::core::SoilType::DenseSand, "Dense Sand"),
                        (crate::core::SoilType::MediumSand, "Medium Sand"),
                        (crate::core::SoilType::LooseSand, "Loose Sand"),
                        (crate::core::SoilType::StiffClay, "Stiff Clay"),
                        (crate::core::SoilType::MediumClay, "Medium Clay"),
                        (crate::core::SoilType::SoftClay, "Soft Clay"),
                        (crate::core::SoilType::Bedrock, "Bedrock"),
                    ];
                    for (st, lbl) in soil_types {
                        let is_sel = self.geotech_soil_type == st;
                        let btn = egui::Button::new(RichText::new(lbl).size(11.0).color(if is_sel { Color32::WHITE } else { theme.text_secondary }))
                            .fill(if is_sel { theme.accent_blue } else { theme.bg_canvas });
                        if ui.add(btn).clicked() {
                            self.geotech_soil_type = st;
                        }
                    }

                    ui.add_space(14.0);
                    ui.label(RichText::new("Footing Width B:").size(11.0).color(theme.text_secondary));
                    ui.add(egui::DragValue::new(&mut self.geotech_footing_w).speed(0.1).range(1.0..=15.0).suffix(" m"));

                    ui.add_space(8.0);
                    ui.label(RichText::new("Footing Length L:").size(11.0).color(theme.text_secondary));
                    ui.add(egui::DragValue::new(&mut self.geotech_footing_l).speed(0.1).range(1.0..=15.0).suffix(" m"));

                    ui.add_space(14.0);
                    ui.label(RichText::new("Pile Grid Nx:").size(11.0).color(theme.text_secondary));
                    ui.add(egui::DragValue::new(&mut self.geotech_piles_x).speed(1.0).range(1..=6));

                    ui.add_space(8.0);
                    ui.label(RichText::new("Ny:").size(11.0).color(theme.text_secondary));
                    ui.add(egui::DragValue::new(&mut self.geotech_piles_y).speed(1.0).range(1..=6));
                });
            });

        ui.add_space(12.0);

        // 3. DUAL VECTOR VISUALIZER: Left = 2D Pile Layout & Biaxial Load Iso-contours; Right = Punching Shear Critical Perimeter
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::icon_label(ui, AppIcon::RulerSquare, 14.0, theme.accent_blue, "Foundation & SSI Vector Visualizer (Deep Foundation Grid + Two-Way Punching Perimeter)", 13.5, theme.text_primary, true);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (status_icon, status_tag, status_col) = if audit.is_fully_compliant {
                            (AppIcon::CheckCircle, "Geotech & Punching Compliant", theme.accent_green)
                        } else {
                            (AppIcon::AlertTriangle, "Reinforcement / Piles Required", Color32::from_rgb(239, 68, 68))
                        };
                        icons::pill_badge(ui, status_icon, status_tag, status_col);
                    });
                });

                ui.add_space(10.0);

                ui_kit::responsive_two_panes(ui, 720.0, |ui, pane_idx| match pane_idx {
                    0 => {
                        // LEFT: Pile Group 2D Plan Layout with Biaxial Load Distribution
                        ui.label(RichText::new(format!("1. Pile Group Layout ({}x{} = {} Piles, Cap {:.1}x{:.1}m, Eg = {:.1}%)", piles.piles_count_x, piles.piles_count_y, piles.total_piles, piles.cap_width_m, piles.cap_length_m, piles.converse_labarre_efficiency * 100.0)).size(11.0).strong().color(theme.text_secondary));
                        let desired_left = Vec2::new(ui.available_width(), 200.0);
                        let (rect_l, _) = ui.allocate_exact_size(desired_left, Sense::hover());
                        let painter_l = ui.painter_at(rect_l);
                        painter_l.rect_filled(rect_l, CornerRadius::same(6), Color32::from_rgb(12, 16, 23));

                                    let center_l = rect_l.center();
                                    let max_cap_dim = piles.cap_width_m.max(piles.cap_length_m).max(2.0) as f32;
                                    let scale_l = (rect_l.height() * 0.72) / max_cap_dim;

                                    // Draw Pile Cap outline
                                    let cap_w_px = (piles.cap_width_m as f32) * scale_l;
                                    let cap_l_px = (piles.cap_length_m as f32) * scale_l;
                                    let cap_rect = egui::Rect::from_center_size(center_l, Vec2::new(cap_w_px, cap_l_px));
                                    painter_l.rect_filled(cap_rect, CornerRadius::same(4), Color32::from_rgba_premultiplied(30, 41, 59, 180));
                                    painter_l.rect_stroke(cap_rect, CornerRadius::same(4), Stroke::new(1.5, Color32::from_rgb(100, 116, 139)), egui::StrokeKind::Inside);

                                    // Draw Column base at center of cap
                                    let col_w_px = 0.60 * scale_l;
                                    let col_rect = egui::Rect::from_center_size(center_l, Vec2::new(col_w_px, col_w_px));
                                    painter_l.rect_filled(col_rect, CornerRadius::same(2), Color32::from_rgb(51, 65, 85));
                                    painter_l.rect_stroke(col_rect, CornerRadius::same(2), Stroke::new(1.0, Color32::from_rgb(148, 163, 184)), egui::StrokeKind::Inside);

                                    // Draw Individual Piles
                                    let r_pile_px = ((piles.pile_diameter_m as f32 * 0.5) * scale_l).clamp(5.0, 24.0);
                                    for p in &piles.pile_details {
                                        let px = center_l.x + (p.x_coord_m as f32) * scale_l;
                                        let py = center_l.y - (p.y_coord_m as f32) * scale_l;
                                        let pile_pos = Pos2::new(px, py);

                                        let p_color = if p.axial_dcr > 1.0 {
                                            Color32::from_rgb(239, 68, 68)
                                        } else if p.axial_dcr > 0.80 {
                                            Color32::from_rgb(245, 158, 11)
                                        } else {
                                            Color32::from_rgb(16, 185, 129)
                                        };

                                        painter_l.circle_filled(pile_pos, r_pile_px, p_color.gamma_multiply(0.25));
                                        painter_l.circle_filled(pile_pos, r_pile_px * 0.75, p_color);
                                        painter_l.circle_stroke(pile_pos, r_pile_px, Stroke::new(1.2, Color32::WHITE));

                                        // Label pile ID and axial load
                                        painter_l.text(
                                            pile_pos + Vec2::new(0.0, r_pile_px + 4.0),
                                            egui::Align2::CENTER_TOP,
                                            format!("P{}: {:.0}kN", p.pile_id, p.factored_axial_demand_pu_kn),
                                            egui::FontId::proportional(8.5),
                                            Color32::from_rgb(226, 232, 240),
                                        );
                                    }

                                    // Annotations
                                    painter_l.text(
                                        Pos2::new(rect_l.left() + 8.0, rect_l.top() + 8.0),
                                        egui::Align2::LEFT_TOP,
                                        format!("Applied Pu = {:.0} kN • Mux = {:.0} kNm • Muy = {:.0} kNm", piles.applied_pu_kn, piles.applied_mux_knm, piles.applied_muy_knm),
                                        egui::FontId::proportional(9.5),
                                        theme.accent_blue,
                                    );
                    }
                    _ => {
                        // RIGHT: Punching Shear Critical Perimeter (SNI 2847:2019 / ACI 318-19)
                        ui.label(RichText::new(format!("2. Critical Punching Perimeter (bo = {:.0} mm at d/2 = {:.0} mm)", punch.critical_perimeter_bo_mm, punch.slab_effective_depth_d_mm * 0.5)).size(11.0).strong().color(theme.text_secondary));
                                    let desired_right = Vec2::new(ui.available_width(), 200.0);
                                    let (rect_r, _) = ui.allocate_exact_size(desired_right, Sense::hover());
                                    let painter_r = ui.painter_at(rect_r);
                                    painter_r.rect_filled(rect_r, CornerRadius::same(6), Color32::from_rgb(12, 16, 23));

                                    let center_r = rect_r.center();
                                    let slab_view_dim = (punch.column_dim_c1_mm + punch.slab_effective_depth_d_mm * 4.0).max(1200.0) as f32;
                                    let scale_r = (rect_r.height() * 0.75) / slab_view_dim;

                                    // Footing / Slab background slab patch
                                    let slab_patch_w = slab_view_dim * scale_r * 0.95;
                                    let slab_patch_rect = egui::Rect::from_center_size(center_r, Vec2::new(slab_patch_w, slab_patch_w));
                                    painter_r.rect_filled(slab_patch_rect, CornerRadius::same(4), Color32::from_rgba_premultiplied(30, 41, 59, 140));
                                    painter_r.rect_stroke(slab_patch_rect, CornerRadius::same(4), Stroke::new(1.0, Color32::from_rgb(71, 85, 105)), egui::StrokeKind::Inside);

                                    // Column cross-section (c1 x c2)
                                    let c1_px = (punch.column_dim_c1_mm as f32) * scale_r;
                                    let c2_px = (punch.column_dim_c2_mm as f32) * scale_r;
                                    let col_sec_rect = egui::Rect::from_center_size(center_r, Vec2::new(c1_px, c2_px));
                                    painter_r.rect_filled(col_sec_rect, CornerRadius::same(2), Color32::from_rgb(71, 85, 105));
                                    painter_r.rect_stroke(col_sec_rect, CornerRadius::same(2), Stroke::new(1.5, Color32::from_rgb(203, 213, 225)), egui::StrokeKind::Inside);

                                    // Critical Perimeter bo at d/2
                                    let bo_w_px = ((punch.column_dim_c1_mm + punch.slab_effective_depth_d_mm) as f32) * scale_r;
                                    let bo_h_px = ((punch.column_dim_c2_mm + punch.slab_effective_depth_d_mm) as f32) * scale_r;
                                    let bo_rect = egui::Rect::from_center_size(center_r, Vec2::new(bo_w_px, bo_h_px));

                                    let bo_color = if punch.is_punching_safe { theme.accent_green } else { Color32::from_rgb(239, 68, 68) };
                                    painter_r.rect_stroke(bo_rect, CornerRadius::same(3), Stroke::new(2.2, bo_color), egui::StrokeKind::Inside);

                                    // Dimension lines for bo
                                    painter_r.text(
                                        Pos2::new(bo_rect.center().x, bo_rect.top() - 6.0),
                                        egui::Align2::CENTER_BOTTOM,
                                        format!("c1 + d = {:.0} mm", punch.column_dim_c1_mm + punch.slab_effective_depth_d_mm),
                                        egui::FontId::proportional(9.0),
                                        theme.accent_blue,
                                    );
                                    painter_r.text(
                                        Pos2::new(bo_rect.right() + 6.0, bo_rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        format!("c2 + d = {:.0} mm", punch.column_dim_c2_mm + punch.slab_effective_depth_d_mm),
                                        egui::FontId::proportional(9.0),
                                        theme.accent_blue,
                                    );

                                    // Corner reticles at d/2
                                    for corner in [bo_rect.left_top(), bo_rect.right_top(), bo_rect.left_bottom(), bo_rect.right_bottom()] {
                                        painter_r.circle_filled(corner, 3.5, bo_color);
                                    }

                                    // Annotations
                                    painter_r.text(
                                        Pos2::new(rect_r.left() + 8.0, rect_r.top() + 8.0),
                                        egui::Align2::LEFT_TOP,
                                        format!("Vu = {:.1} kN • vu = {:.3} MPa • φVc = {:.3} MPa", punch.factored_shear_vu_kn, punch.punching_stress_vu_mpa, punch.concrete_capacity_phi_vc_mpa),
                                        egui::FontId::proportional(9.5),
                                        bo_color,
                                    );
                                    painter_r.text(
                                        Pos2::new(rect_r.left() + 8.0, rect_r.bottom() - 8.0),
                                        egui::Align2::LEFT_BOTTOM,
                                        if punch.is_punching_safe { "PASS: Two-way punching shear compliant without stud rails" } else { "FAIL: Add shear studs or increase slab thickness" },
                                        egui::FontId::proportional(9.0),
                                        bo_color,
                                    );
                    }
                });
            });

        ui.add_space(14.0);

        // 4. Winkler Foundation 6-DOF Spring Matrix & Soil Stratum Parameters Card
        Frame::new()
            .fill(Color32::from_rgb(17, 24, 39))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .stroke(Stroke::new(1.0, Color32::from_rgb(14, 165, 233)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::icon_label(ui, AppIcon::NodesGraph, 13.0, Color32::WHITE, "Equivalent 6-DOF Winkler Subgrade Reaction Matrix (Vesic / Bowles Formulation):", 12.5, Color32::WHITE, true);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("ks = {:.0} kN/m³ • E = {:.1} MPa • ν = {:.2} • φ = {:.1}°", soil_layer.subgrade_reaction_ks_kn_m3, soil_layer.elastic_modulus_es_mpa, soil_layer.poisson_ratio_nu, soil_layer.friction_angle_deg)).size(11.0).color(theme.accent_blue));
                    });
                });
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("Kz (Vert): {:.1} MN/m", winkler.kz_trans_kn_m / 1000.0)).size(11.0).strong().color(theme.accent_blue));
                    ui.add_space(14.0);
                    ui.label(RichText::new(format!("Kx, Ky (Shear): {:.1} MN/m", winkler.kx_trans_kn_m / 1000.0)).size(11.0).color(theme.text_secondary));
                    ui.add_space(14.0);
                    ui.label(RichText::new(format!("Kθx (Rocking L): {:.1} MNm/rad", winkler.k_theta_x_knm_rad / 1000.0)).size(11.0).strong().color(theme.accent_green));
                    ui.add_space(14.0);
                    ui.label(RichText::new(format!("Kθy (Rocking B): {:.1} MNm/rad", winkler.k_theta_y_knm_rad / 1000.0)).size(11.0).strong().color(theme.accent_green));
                    ui.add_space(14.0);
                    ui.label(RichText::new(format!("Kθz (Torsion): {:.1} MNm/rad", winkler.k_theta_z_knm_rad / 1000.0)).size(11.0).color(theme.accent_orange));
                });
            });

        ui.add_space(14.0);

        // 5. Individual Pile Biaxial Distribution & Deflection Table
        Frame::new()
            .fill(theme.bg_card)
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::icon_label(ui, AppIcon::Clipboard, 13.0, theme.accent_blue, "Individual Pile Biaxial Load Distribution & Lateral Deflection Schedule:", 12.5, theme.text_primary, true);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("Allowable Compression: {:.0} kN • Tension: {:.0} kN", piles.pile_details.first().map(|p| p.allowable_compression_all_kn).unwrap_or(0.0), piles.pile_details.first().map(|p| p.allowable_tension_uplift_kn).unwrap_or(0.0))).size(11.0).color(theme.text_muted));
                    });
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Pile ID").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(25.0);
                    ui.label(RichText::new("Location (x, y)").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(35.0);
                    ui.label(RichText::new("Axial Demand Pu").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(35.0);
                    ui.label(RichText::new("Allowable Pall").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(35.0);
                    ui.label(RichText::new("Axial DCR").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(35.0);
                    ui.label(RichText::new("Lateral Hu").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(35.0);
                    ui.label(RichText::new("Deflection δ").size(11.0).strong().color(theme.text_secondary));
                    ui.add_space(35.0);
                    ui.label(RichText::new("Verdict").size(11.0).strong().color(theme.text_secondary));
                });
                ui.separator();

                for p in &piles.pile_details {
                    let dcr_col = if p.axial_dcr <= 0.80 {
                        theme.accent_green
                    } else if p.axial_dcr <= 1.00 {
                        theme.accent_orange
                    } else {
                        Color32::from_rgb(239, 68, 68)
                    };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Pile #{:02}", p.pile_id)).size(11.0).strong().color(theme.accent_blue));
                        ui.add_space(20.0);
                        ui.label(RichText::new(format!("({:+.2}m, {:+.2}m)", p.x_coord_m, p.y_coord_m)).size(11.0).color(theme.text_primary));
                        ui.add_space(30.0);
                        ui.label(RichText::new(format!("{:7.1} kN", p.factored_axial_demand_pu_kn)).size(11.0).color(theme.text_primary));
                        ui.add_space(30.0);
                        ui.label(RichText::new(format!("{:7.1} kN", p.allowable_compression_all_kn)).size(11.0).color(theme.text_muted));
                        ui.add_space(30.0);
                        ui.label(RichText::new(format!("{:.3}", p.axial_dcr)).size(11.0).strong().color(dcr_col));
                        ui.add_space(45.0);
                        ui.label(RichText::new(format!("{:.1} kN", p.lateral_shear_hu_kn)).size(11.0).color(theme.text_secondary));
                        ui.add_space(40.0);
                        ui.label(RichText::new(format!("{:.2} mm", p.lateral_deflection_mm)).size(11.0).color(theme.text_secondary));
                        ui.add_space(40.0);
                        let (p_icon, p_txt) = if p.is_safe { (AppIcon::CheckCircle, "SAFE") } else { (AppIcon::AlertTriangle, "OVERSTRESSED") };
                        icons::icon_label(ui, p_icon, 11.5, dcr_col, p_txt, 11.0, dcr_col, true);
                    });
                    ui.separator();
                }
            });

        ui.add_space(14.0);

        // 6. Dynamic SSI Modal Period Shift & Base Shear Reduction Card
        Frame::new()
            .fill(Color32::from_rgb(15, 23, 42))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .stroke(Stroke::new(1.0, Color32::from_rgb(16, 185, 129)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::icon_label(ui, AppIcon::Pulse, 13.0, Color32::WHITE, "Veletsos & Meek Dynamic Soil-Structure Interaction (SSI) Modal Shift (ASCE 7-22 Ch. 19):", 12.0, Color32::WHITE, true);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(&ssi.ssi_classification).size(11.0).strong().color(theme.accent_green));
                    });
                });
                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("Fixed-Base Period T: {:.3}s", ssi.fixed_base_period_t_s)).size(11.0).color(theme.text_secondary));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("Flexible SSI Period T̃: {:.3}s", ssi.flexible_ssi_period_t_tilde_s)).size(11.0).strong().color(theme.accent_blue));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("Period Lengthening T̃/T: {:.3}x (+{:.1}%)", ssi.period_lengthening_ratio, (ssi.period_lengthening_ratio - 1.0) * 100.0)).size(11.0).strong().color(theme.accent_green));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("Base Shear Reduction: -{:.1}%", (1.0 - ssi.base_shear_reduction_ratio) * 100.0)).size(11.0).strong().color(theme.accent_orange));
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("Foundation Damping βssi: {:.1}%", ssi.foundation_damping_factor * 100.0)).size(11.0).color(theme.text_muted));
                });
            });
    }

    // -------------------------------------------------------------
    // SUB-TAB 10: Steel Connection Design & AISC 360-16 / SNI 1729
    // -------------------------------------------------------------

    pub(super) fn render_base_isolation_design(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let input = crate::core::SeismicIsolationInput {
            isolator_type: self.iso_type,
            seismic_weight_w_kn: self.iso_seismic_weight_w_kn as f64,
            num_bearings: self.iso_num_bearings,
            axial_load_per_bearing_kn: (self.iso_seismic_weight_w_kn as f64) / (self.iso_num_bearings as f64).max(1.0),
            spectral_accel_sm1_g: self.iso_sm1_g as f64,
            spectral_accel_sms_g: self.iso_sms_g as f64,
            fixed_base_period_s: self.iso_fixed_period_s as f64,
            superstructure_r_i: self.iso_superstructure_ri as f64,
            building_plan_b_m: self.iso_plan_b_m as f64,
            building_plan_d_m: self.iso_plan_d_m as f64,
            plan_eccentricity_e_m: self.iso_eccentricity_e_m as f64,
            lrb_params: crate::core::LeadRubberBearingParams {
                outer_diameter_mm: self.iso_lrb_diameter_mm as f64,
                lead_core_diameter_mm: self.iso_lrb_lead_dia_mm as f64,
                rubber_layer_thick_mm: self.iso_lrb_rubber_layer_mm as f64,
                num_rubber_layers: self.iso_lrb_num_layers,
                steel_shim_thick_mm: 3.5,
                shear_modulus_g_mpa: self.iso_lrb_shear_mod_g as f64,
                lead_yield_stress_mpa: 10.2,
                bulk_modulus_mpa: 2000.0,
            },
            fps_params: crate::core::FrictionPendulumParams {
                radius_of_curvature_mm: self.iso_fps_radius_mm as f64,
                friction_coeff_fast: self.iso_fps_mu_fast as f64,
                friction_coeff_slow: (self.iso_fps_mu_fast as f64) * 0.55,
                friction_rate_param: 100.0,
                slider_diameter_mm: (self.iso_fps_housing_dia_mm as f64) * 0.28,
                housing_diameter_mm: self.iso_fps_housing_dia_mm as f64,
            },
            fvd_params: crate::core::FluidViscousDamperParams {
                damping_coeff_c: self.iso_fvd_c_kn_s as f64,
                velocity_exponent_alpha: self.iso_fvd_alpha as f64,
                stroke_limit_mm: self.iso_fvd_stroke_limit_mm as f64,
                cylinder_diameter_mm: 220.0,
                inclination_angle_deg: 26.57,
                configuration: crate::core::DamperConfiguration::ChevronBrace,
            },
            num_dampers: self.iso_fvd_count,
        };

        let rep = crate::core::SeismicIsolationEngine::evaluate(&input);

        // Header Title
        ui.horizontal(|ui| {
            icons::icon_label(ui, AppIcon::Shield, 16.0, theme.accent_blue, "Seismic Base Isolation & Supplementary Energy Dissipation Studio", 15.0, theme.text_primary, true);
            ui.add_space(8.0);
            ui.label(RichText::new("ASCE 7-22 Ch 17 & 18 / SNI 1726:2019 Pasal 12 & 13").size(11.0).color(theme.accent_blue));
        });
        ui.add_space(8.0);

        // 4 Executive KPI Cards
        ui_kit::responsive_grid_n(ui, 4, 190.0, |ui, col_idx| match col_idx {
            0 => {
                // Card 1: Effective Isolation Period
                    ui.group(|ui| {
                        ui.label(RichText::new("EFFECTIVE PERIOD (TM)").size(10.0).color(theme.text_secondary));
                        ui.label(RichText::new(format!("{:.2} s", rep.effective_period_tm_s)).size(20.0).strong().color(theme.accent_blue));
                        let period_mult = rep.effective_period_tm_s / (self.iso_fixed_period_s as f64).max(0.1);
                        ui.label(RichText::new(format!("Lengthening: {:.1}x (T1={:.2}s)", period_mult, self.iso_fixed_period_s)).size(10.0).color(theme.text_muted));
                    });
            }
            1 => {
                // Card 2: Maximum Design Displacement
                    ui.group(|ui| {
                        ui.label(RichText::new("TOTAL MAX DISP (DTM)").size(10.0).color(theme.text_secondary));
                        ui.label(RichText::new(format!("{:.1} mm", rep.total_max_displacement_dtm_mm)).size(20.0).strong().color(Color32::from_rgb(245, 158, 11)));
                        ui.label(RichText::new(format!("CM Disp DM = {:.1} mm (+Torsion)", rep.max_displacement_dm_mm)).size(10.0).color(theme.text_muted));
                    });
            }
            2 => {
                // Card 3: Total System Damping
                    ui.group(|ui| {
                        ui.label(RichText::new("EFFECTIVE DAMPING (βeff)").size(10.0).color(theme.text_secondary));
                        ui.label(RichText::new(format!("{:.1}%", rep.combined_total_damping * 100.0)).size(20.0).strong().color(Color32::from_rgb(180, 100, 255)));
                        ui.label(RichText::new(format!("Damping Factor BM = {:.2}", rep.damping_reduction_bm)).size(10.0).color(theme.text_muted));
                    });
            }
            _ => {
                // Card 4: Base Shear Reduction
                    let red_col = if rep.base_shear_reduction_ratio <= 0.50 { theme.accent_green } else { Color32::from_rgb(239, 68, 68) };
                    ui.group(|ui| {
                        ui.label(RichText::new("BASE SHEAR REDUCTION").size(10.0).color(theme.text_secondary));
                        ui.label(RichText::new(format!("{:.1}%", rep.base_shear_reduction_ratio * 100.0)).size(20.0).strong().color(red_col));
                        ui.label(RichText::new(format!("Vs = {:.0} kN (Req Moat {:.0}mm)", rep.superstructure_design_base_shear_vs_kn, rep.required_moat_gap_mm)).size(10.0).color(theme.text_muted));
                    });
            }
        });

        ui.add_space(10.0);

        // Parameter Studio Card
        theme.card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(ui, AppIcon::SettingsGear, 13.5, theme.accent_blue, "Isolation & Damping Parameter Studio", 12.5, theme.accent_blue, true);
                ui.add_space(20.0);
                ui.selectable_value(&mut self.iso_type, crate::core::IsolatorType::LeadRubberBearing, "Lead-Rubber Bearing (LRB)");
                ui.selectable_value(&mut self.iso_type, crate::core::IsolatorType::FrictionPendulum, "Friction Pendulum (FPS)");
                ui.selectable_value(&mut self.iso_type, crate::core::IsolatorType::FluidViscousDamper, "Fluid Viscous Damper (FVD)");
            });

            ui.separator();

            ui_kit::responsive_grid_n(ui, 3, 240.0, |ui, col_idx| match col_idx {
                0 => {
                    // Column 1: Seismic Demand Parameters
                            ui.vertical(|ui| {
                                ui.label(RichText::new("1. Superstructure & Seismic Demands").size(11.0).strong().color(theme.text_primary));
                                ui.horizontal(|ui| {
                                    ui.label("Total Weight W:");
                                    ui.add(egui::Slider::new(&mut self.iso_seismic_weight_w_kn, 20000.0..=250000.0).suffix(" kN"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Num Bearings:");
                                    ui.add(egui::Slider::new(&mut self.iso_num_bearings, 8..=64));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("MCE_R S_M1 (1.0s):");
                                    ui.add(egui::Slider::new(&mut self.iso_sm1_g, 0.20..=1.50).suffix(" g"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("MCE_R S_MS (0.2s):");
                                    ui.add(egui::Slider::new(&mut self.iso_sms_g, 0.50..=2.50).suffix(" g"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Fixed Period T1:");
                                    ui.add(egui::Slider::new(&mut self.iso_fixed_period_s, 0.30..=2.50).suffix(" s"));
                                });
                            });
                }
                1 => {
                    // Column 2: Plan Geometry & Torsion
                            ui.vertical(|ui| {
                                ui.label(RichText::new("2. Plan Geometry & Torsion").size(11.0).strong().color(theme.text_primary));
                                ui.horizontal(|ui| {
                                    ui.label("Plan Width (b):");
                                    ui.add(egui::Slider::new(&mut self.iso_plan_b_m, 12.0..=80.0).suffix(" m"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Plan Length (d):");
                                    ui.add(egui::Slider::new(&mut self.iso_plan_d_m, 12.0..=80.0).suffix(" m"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Accidental Ecc. (e):");
                                    ui.add(egui::Slider::new(&mut self.iso_eccentricity_e_m, 0.0..=6.0).suffix(" m"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Reduction Factor R_I:");
                                    ui.add(egui::Slider::new(&mut self.iso_superstructure_ri, 1.0..=2.5));
                                });
                            });
                }
                _ => {
                    // Column 3: Device Specific Properties
                            ui.vertical(|ui| {
                                match self.iso_type {
                                    crate::core::IsolatorType::LeadRubberBearing | crate::core::IsolatorType::HighDampingRubberBearing => {
                                        ui.label(RichText::new("3. Lead-Rubber Bearing Specs").size(11.0).strong().color(theme.text_primary));
                                        ui.horizontal(|ui| {
                                            ui.label("Outer Diameter (Do):");
                                            ui.add(egui::Slider::new(&mut self.iso_lrb_diameter_mm, 500.0..=1300.0).suffix(" mm"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Lead Plug Dia (dpb):");
                                            ui.add(egui::Slider::new(&mut self.iso_lrb_lead_dia_mm, 80.0..=250.0).suffix(" mm"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Rubber Layer (tr):");
                                            ui.add(egui::Slider::new(&mut self.iso_lrb_rubber_layer_mm, 4.0..=12.0).suffix(" mm"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Rubber Layers (N):");
                                            ui.add(egui::Slider::new(&mut self.iso_lrb_num_layers, 12..=48));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Rubber Modulus (G):");
                                            ui.add(egui::Slider::new(&mut self.iso_lrb_shear_mod_g, 0.35..=0.85).suffix(" MPa"));
                                        });
                                    }
                                    crate::core::IsolatorType::FrictionPendulum => {
                                        ui.label(RichText::new("3. Friction Pendulum Specs").size(11.0).strong().color(theme.text_primary));
                                        ui.horizontal(|ui| {
                                            ui.label("Spherical Radius (R):");
                                            ui.add(egui::Slider::new(&mut self.iso_fps_radius_mm, 1500.0..=4500.0).suffix(" mm"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Dynamic Friction (μ):");
                                            ui.add(egui::Slider::new(&mut self.iso_fps_mu_fast, 0.02..=0.10));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Housing Diameter:");
                                            ui.add(egui::Slider::new(&mut self.iso_fps_housing_dia_mm, 600.0..=1600.0).suffix(" mm"));
                                        });
                                    }
                                    crate::core::IsolatorType::FluidViscousDamper => {
                                        ui.label(RichText::new("3. Fluid Viscous Damper Specs").size(11.0).strong().color(theme.text_primary));
                                        ui.horizontal(|ui| {
                                            ui.label("Damping Coeff C:");
                                            ui.add(egui::Slider::new(&mut self.iso_fvd_c_kn_s, 50.0..=600.0).suffix(" kN·(s/m)^α"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Velocity Exponent α:");
                                            ui.add(egui::Slider::new(&mut self.iso_fvd_alpha, 0.20..=1.00));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Stroke Limit:");
                                            ui.add(egui::Slider::new(&mut self.iso_fvd_stroke_limit_mm, 100.0..=500.0).suffix(" mm"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Damper Count:");
                                            ui.add(egui::Slider::new(&mut self.iso_fvd_count, 4..=40));
                                        });
                                    }
                                }
                            });
                }
            });
        });

        ui.add_space(10.0);

        // Vector CAD Blueprint & Hysteresis Canvas
        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                icons::icon_label(ui, AppIcon::RulerSquare, 13.5, theme.accent_blue, "2D Vector CAD Blueprint & Dynamic Hysteresis Loop Canvas", 12.5, theme.accent_blue, true);
                ui.add_space(20.0);
                ui.label(RichText::new(format!("Energy Dissipated per Cycle: {:.1} kJ", rep.energy_dissipated_per_cycle_kj)).size(11.0).color(theme.accent_green));
            });

            ui.add_space(6.0);

            let (canvas_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 260.0), Sense::hover());
            let painter = ui.painter_at(canvas_rect);

            painter.rect_filled(canvas_rect, 6.0, Color32::from_rgb(18, 22, 30));
            painter.rect_stroke(canvas_rect, 6.0, Stroke::new(1.0, Color32::from_rgb(45, 55, 75)), egui::StrokeKind::Inside);

            let mid_x = canvas_rect.min.x + canvas_rect.width() * 0.48;

            // Divider line between Device CAD and Hysteresis Plot
            painter.line_segment(
                [Pos2::new(mid_x, canvas_rect.min.y + 10.0), Pos2::new(mid_x, canvas_rect.max.y - 10.0)],
                Stroke::new(1.0, Color32::from_rgb(45, 55, 75)),
            );

            // Left Side: Device Vector CAD Drawing
            let dev_center = Pos2::new(canvas_rect.min.x + (mid_x - canvas_rect.min.x) * 0.5, canvas_rect.min.y + 130.0);

            match self.iso_type {
                crate::core::IsolatorType::LeadRubberBearing | crate::core::IsolatorType::HighDampingRubberBearing => {
                    // Draw LRB Cross-Section
                    let lrb_w = 160.0;
                    let lrb_h = 100.0;
                    let lead_w = (lrb_w * (self.iso_lrb_lead_dia_mm / self.iso_lrb_diameter_mm)).clamp(20.0, 50.0);

                    // Top & Bottom Flange Plates (Steel)
                    let plate_h = 14.0;
                    painter.rect_filled(
                        egui::Rect::from_center_size(Pos2::new(dev_center.x, dev_center.y - lrb_h * 0.5 - plate_h * 0.5), Vec2::new(lrb_w + 30.0, plate_h)),
                        2.0,
                        Color32::from_rgb(140, 150, 165),
                    );
                    painter.rect_filled(
                        egui::Rect::from_center_size(Pos2::new(dev_center.x, dev_center.y + lrb_h * 0.5 + plate_h * 0.5), Vec2::new(lrb_w + 30.0, plate_h)),
                        2.0,
                        Color32::from_rgb(140, 150, 165),
                    );

                    // Alternating Rubber Layers & Steel Shims
                    let num_draw_layers = 12;
                    let layer_h = lrb_h / (num_draw_layers as f32);
                    for i in 0..num_draw_layers {
                        let y_top = dev_center.y - lrb_h * 0.5 + (i as f32) * layer_h;
                        let is_steel_shim = i % 2 == 1;
                        let fill_col = if is_steel_shim {
                            Color32::from_rgb(160, 170, 185)
                        } else {
                            Color32::from_rgb(35, 40, 50)
                        };
                        painter.rect_filled(
                            egui::Rect::from_min_max(Pos2::new(dev_center.x - lrb_w * 0.5, y_top), Pos2::new(dev_center.x + lrb_w * 0.5, y_top + layer_h)),
                            0.0,
                            fill_col,
                        );
                    }

                    // Central Lead Plug (Gold / Silver cylinder)
                    painter.rect_filled(
                        egui::Rect::from_center_size(dev_center, Vec2::new(lead_w, lrb_h)),
                        2.0,
                        Color32::from_rgb(234, 179, 8),
                    );
                    painter.rect_stroke(
                        egui::Rect::from_center_size(dev_center, Vec2::new(lead_w, lrb_h)),
                        2.0,
                        Stroke::new(1.0, Color32::from_rgb(250, 204, 21)),
                        egui::StrokeKind::Inside,
                    );

                    // Bearing boundary outline
                    painter.rect_stroke(
                        egui::Rect::from_center_size(dev_center, Vec2::new(lrb_w, lrb_h)),
                        0.0,
                        Stroke::new(1.5, Color32::WHITE),
                        egui::StrokeKind::Inside,
                    );

                    painter.text(
                        Pos2::new(dev_center.x, dev_center.y + lrb_h * 0.5 + 28.0),
                        Align2::CENTER_CENTER,
                        format!("LRB Ø{:.0}mm (Lead Core Ø{:.0}mm, Tr={:.0}mm)", self.iso_lrb_diameter_mm, self.iso_lrb_lead_dia_mm, rep.total_rubber_thick_tr_mm),
                        egui::FontId::proportional(11.0),
                        theme.accent_blue,
                    );
                }
                crate::core::IsolatorType::FrictionPendulum => {
                    // Draw Friction Pendulum Bearing (Concave dish + slider)
                    let dish_w = 180.0;
                    let dish_h = 30.0;
                    let slider_w = 50.0;
                    let slider_h = 40.0;

                    // Lower Concave Dish (Spherical Curvature)
                    let dish_rect = egui::Rect::from_center_size(Pos2::new(dev_center.x, dev_center.y + 35.0), Vec2::new(dish_w, dish_h));
                    painter.rect_filled(dish_rect, 6.0, Color32::from_rgb(100, 115, 135));
                    painter.rect_stroke(dish_rect, 6.0, Stroke::new(1.5, Color32::from_rgb(180, 195, 215)), egui::StrokeKind::Inside);

                    // Articulated Slider Puck (Composite PTFE / Bronze)
                    let slider_rect = egui::Rect::from_center_size(Pos2::new(dev_center.x, dev_center.y + 10.0), Vec2::new(slider_w, slider_h));
                    painter.rect_filled(slider_rect, 4.0, Color32::from_rgb(245, 158, 11));
                    painter.rect_stroke(slider_rect, 4.0, Stroke::new(1.5, Color32::WHITE), egui::StrokeKind::Inside);

                    // Upper Mounting Plate
                    let top_rect = egui::Rect::from_center_size(Pos2::new(dev_center.x, dev_center.y - 25.0), Vec2::new(dish_w, 20.0));
                    painter.rect_filled(top_rect, 3.0, Color32::from_rgb(140, 150, 165));
                    painter.rect_stroke(top_rect, 3.0, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Inside);

                    painter.text(
                        Pos2::new(dev_center.x, dev_center.y + 65.0),
                        Align2::CENTER_CENTER,
                        format!("FPS Radius R={:.0}mm (Housing Ø{:.0}mm, μ={:.3})", self.iso_fps_radius_mm, self.iso_fps_housing_dia_mm, self.iso_fps_mu_fast),
                        egui::FontId::proportional(11.0),
                        theme.accent_blue,
                    );
                }
                crate::core::IsolatorType::FluidViscousDamper => {
                    // Draw Fluid Viscous Damper (Cylinder + Piston + Clevis)
                    let cyl_w = 110.0;
                    let cyl_h = 36.0;
                    let rod_w = 70.0;
                    let rod_h = 14.0;

                    // Hydraulic Cylinder Barrel
                    let cyl_rect = egui::Rect::from_min_max(
                        Pos2::new(dev_center.x - cyl_w * 0.5, dev_center.y - cyl_h * 0.5),
                        Pos2::new(dev_center.x + cyl_w * 0.5, dev_center.y + cyl_h * 0.5),
                    );
                    painter.rect_filled(cyl_rect, 4.0, Color32::from_rgb(30, 41, 59));
                    painter.rect_stroke(cyl_rect, 4.0, Stroke::new(2.0, Color32::from_rgb(56, 189, 248)), egui::StrokeKind::Inside);

                    // Piston Rod (Chrome Steel)
                    let rod_rect = egui::Rect::from_min_max(
                        Pos2::new(dev_center.x + cyl_w * 0.5, dev_center.y - rod_h * 0.5),
                        Pos2::new(dev_center.x + cyl_w * 0.5 + rod_w, dev_center.y + rod_h * 0.5),
                    );
                    painter.rect_filled(rod_rect, 2.0, Color32::from_rgb(203, 213, 225));
                    painter.rect_stroke(rod_rect, 2.0, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Inside);

                    // End Clevis Spherical Bearings
                    painter.circle_filled(Pos2::new(dev_center.x - cyl_w * 0.5 - 12.0, dev_center.y), 10.0, Color32::from_rgb(148, 163, 184));
                    painter.circle_filled(Pos2::new(dev_center.x + cyl_w * 0.5 + rod_w + 12.0, dev_center.y), 10.0, Color32::from_rgb(148, 163, 184));

                    painter.text(
                        Pos2::new(dev_center.x, dev_center.y + 45.0),
                        Align2::CENTER_CENTER,
                        format!("FVD C={:.0} kN·(s/m)^α (α={:.2}, Fmax={:.0}kN)", self.iso_fvd_c_kn_s, self.iso_fvd_alpha, rep.damper_peak_force_kn),
                        egui::FontId::proportional(11.0),
                        theme.accent_blue,
                    );
                }
            }

            // Right Side: Dynamic Hysteresis Loop Canvas
            let hyst_center = Pos2::new(mid_x + (canvas_rect.max.x - mid_x) * 0.5, canvas_rect.min.y + 130.0);
            let hyst_w = (canvas_rect.max.x - mid_x) * 0.78;
            let hyst_h = 180.0;

            // Draw Coordinate Axes
            painter.line_segment(
                [Pos2::new(hyst_center.x - hyst_w * 0.5, hyst_center.y), Pos2::new(hyst_center.x + hyst_w * 0.5, hyst_center.y)],
                Stroke::new(1.0, Color32::from_rgb(70, 80, 105)),
            );
            painter.line_segment(
                [Pos2::new(hyst_center.x, hyst_center.y - hyst_h * 0.5), Pos2::new(hyst_center.x, hyst_center.y + hyst_h * 0.5)],
                Stroke::new(1.0, Color32::from_rgb(70, 80, 105)),
            );

            painter.text(
                Pos2::new(hyst_center.x + hyst_w * 0.5, hyst_center.y + 12.0),
                Align2::RIGHT_TOP,
                format!("+DTM ({:.0}mm)", rep.total_max_displacement_dtm_mm),
                egui::FontId::proportional(9.5),
                theme.text_secondary,
            );
            painter.text(
                Pos2::new(hyst_center.x + 6.0, hyst_center.y - hyst_h * 0.5),
                Align2::LEFT_TOP,
                "+Force (kN)",
                egui::FontId::proportional(9.5),
                theme.text_secondary,
            );

            // Draw Hysteresis Loop Points
            if !rep.hysteresis_loop.is_empty() {
                let max_disp = rep.total_max_displacement_dtm_mm.max(50.0);
                let max_force = rep.hysteresis_loop.iter().map(|p| p.lateral_force_kn.abs()).fold(1.0f64, |a, b| a.max(b));

                let scale_x = (hyst_w * 0.45) / (max_disp as f32);
                let scale_y = (hyst_h * 0.42) / (max_force as f32);

                let poly_points: Vec<Pos2> = rep
                    .hysteresis_loop
                    .iter()
                    .map(|p| {
                        Pos2::new(
                            hyst_center.x + (p.displacement_mm as f32) * scale_x,
                            hyst_center.y - (p.lateral_force_kn as f32) * scale_y,
                        )
                    })
                    .collect();

                // Shaded Hysteresis Energy Area
                painter.add(egui::Shape::convex_polygon(
                    poly_points.clone(),
                    Color32::from_rgba_unmultiplied(59, 130, 246, 35),
                    Stroke::new(2.0, Color32::from_rgb(96, 165, 250)),
                ));
            }

            painter.text(
                Pos2::new(hyst_center.x, canvas_rect.min.y + 16.0),
                Align2::CENTER_TOP,
                "Dynamic Hysteresis Energy Dissipation Loop (F vs D)",
                egui::FontId::proportional(11.0),
                Color32::from_rgb(245, 158, 11),
            );
        });

        ui.add_space(10.0);

        // ASCE 7-22 / SNI 1726 Regulatory Compliance Audit Table
        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                icons::icon_label(ui, AppIcon::Clipboard, 13.5, theme.accent_blue, "ASCE 7-22 & SNI 1726:2019 Regulatory Compliance Audit", 12.5, theme.accent_blue, true);
                ui.add_space(20.0);
                let (st_icon, overall_status, status_col) = if rep.is_design_acceptable {
                    (AppIcon::CheckCircle, "ALL CODE CHECKS SATISFIED", theme.accent_green)
                } else {
                    (AppIcon::AlertTriangle, "DESIGN REVISIONS REQUIRED", Color32::from_rgb(239, 68, 68))
                };
                icons::pill_badge(ui, st_icon, overall_status, status_col);
            });

            ui.add_space(6.0);

            // Table Header
            ui.horizontal(|ui| {
                ui.label(RichText::new("Code Clause").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(180.0);
                ui.label(RichText::new("Engineering Requirement").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(170.0);
                ui.label(RichText::new("Demand").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(50.0);
                ui.label(RichText::new("Capacity").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(50.0);
                ui.label(RichText::new("D/C Ratio").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(30.0);
                ui.label(RichText::new("Status").size(10.5).strong().color(theme.text_secondary));
            });
            ui.separator();

            for item in &rep.regulatory_checks {
                let row_col = if item.is_passed { theme.accent_green } else { Color32::from_rgb(239, 68, 68) };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.code_clause).size(10.5).color(theme.accent_blue));
                    ui.add_space(10.0);
                    ui.label(RichText::new(&item.description).size(10.5).color(theme.text_primary));
                    ui.add_space(10.0);
                    ui.label(RichText::new(format!("{:.2} {}", item.demand_val, item.unit)).size(10.5).color(theme.text_primary));
                    ui.add_space(15.0);
                    ui.label(RichText::new(format!("{:.2} {}", item.capacity_val, item.unit)).size(10.5).color(theme.accent_green));
                    ui.add_space(15.0);
                    ui.label(RichText::new(format!("{:.3}", item.demand_capacity_ratio)).size(10.5).strong().color(row_col));
                    ui.add_space(20.0);
                    let (r_icon, r_txt) = if item.is_passed { (AppIcon::CheckCircle, "PASS") } else { (AppIcon::AlertTriangle, "FAIL") };
                    icons::icon_label(ui, r_icon, 11.0, row_col, r_txt, 10.5, row_col, true);
                });
                ui.separator();
            }
        });
    }

    pub(super) fn render_time_history_analysis(&mut self, ui: &mut Ui, theme: &MacTheme) {
        let input = crate::core::TimeHistoryInput {
            ground_motion: self.tha_record,
            scale_factor: self.tha_scale_factor as f64,
            time_step_dt_s: self.tha_dt as f64,
            simulation_duration_s: self.tha_duration as f64,
            building_height_m: self.tha_building_height_m as f64,
            num_stories: self.tha_num_stories,
            total_seismic_weight_kn: self.tha_seismic_weight_kn as f64,
            fundamental_period_t1_s: self.tha_t1_s as f64,
            damping_ratio: self.tha_damping as f64,
            yield_shear_coeff_cy: self.tha_cy as f64,
            post_yield_stiffness_ratio: self.tha_alpha_p as f64,
            hysteresis_rule: crate::core::HysteresisModel::BoucWenDifferential,
            integration_scheme: crate::core::TimeIntegrationScheme::HilberHughesTaylorAlpha,
        };

        let rep = crate::core::TimeHistoryEngine::evaluate(&input);

        // Header & Dossier Export
        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    icons::icon_label(ui, AppIcon::Pulse, 14.0, theme.accent_blue, "Nonlinear Dynamic Direct Time-History Analysis (NL-THA) & Ground Motion Scaling", 13.0, theme.accent_blue, true);
                    ui.label(RichText::new("ASCE 7-22 Chapter 16 · SNI 1726:2019 Pasal 11 · NIST GCR 17-917-46 / ATC-114 · PEER/TBI (2017)").size(10.5).color(theme.text_secondary));
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icons::icon_button(
                        ui,
                        AppIcon::SaveDisk,
                        "Export NL-THA Dossier (.md)",
                        11.0,
                        Color32::WHITE,
                        theme.bg_canvas,
                        Stroke::new(1.0, theme.accent_blue),
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    ).clicked() {
                        let dossier = crate::core::TimeHistoryEngine::generate_markdown_dossier(&input, &rep);
                        if let Ok(path) = crate::core::DiskExporter::write_export_file("EigenGraph_NL_THA_Time_History_Dossier.md", &dossier) {
                            self.export_status = Some(format!("NL-THA Calculation Dossier saved to: {}", path));
                        }
                    }
                });
            });
        });

        ui.add_space(10.0);

        // 4 KPI Summary Cards
        ui_kit::responsive_grid_n(ui, 4, 210.0, |ui, idx| match idx {
    0 => {
        // Card 1: Roof Displacement & Drift
            let drift_col = if rep.peak_roof_drift_ratio <= 0.020 { theme.accent_green } else { Color32::from_rgb(239, 68, 68) };
            theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
ui.label(RichText::new("PEAK ROOF DISP").size(10.0).strong().color(theme.text_secondary));
                ui.label(RichText::new(format!("{:.1} mm", rep.peak_roof_disp_mm)).size(17.0).strong().color(drift_col));
                ui.label(RichText::new(format!("Roof Drift: {:.2}% (≤ 2.0%)", rep.peak_roof_drift_ratio * 100.0)).size(10.0).color(theme.text_muted));
            });
    }
    1 => {
        // Card 2: Peak Base Shear
            theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
ui.label(RichText::new("PEAK BASE SHEAR Vb").size(10.0).strong().color(theme.text_secondary));
                ui.label(RichText::new(format!("{:.0} kN", rep.peak_base_shear_kn)).size(17.0).strong().color(theme.accent_blue));
                ui.label(RichText::new(format!("{:.1}% W (Cy: {:.2})", rep.base_shear_ratio * 100.0, self.tha_cy)).size(10.0).color(theme.text_muted));
            });
    }
    2 => {
        // Card 3: Inelastic Energy Dissipation
            theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
ui.label(RichText::new("ENERGY DISSIPATION").size(10.0).strong().color(theme.text_secondary));
                ui.label(RichText::new(format!("{:.1}% Hysteretic", rep.hysteretic_energy_dissipation_ratio * 100.0)).size(17.0).strong().color(Color32::from_rgb(245, 158, 11)));
                ui.label(RichText::new(format!("Inelastic: {:.0} kJ", rep.total_hysteretic_energy_kj)).size(10.0).color(theme.text_muted));
            });
    }
    _ => {
        // Card 4: Energy Balance Error
            let err_col = if rep.max_energy_balance_error_pct <= 2.50 { theme.accent_green } else { Color32::from_rgb(239, 68, 68) };
            theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
ui.label(RichText::new("ENERGY CONSERVATION").size(10.0).strong().color(theme.text_secondary));
                ui.label(RichText::new(format!("{:.2}% Error", rep.max_energy_balance_error_pct)).size(17.0).strong().color(err_col));
                ui.label(RichText::new("ATC-114 Residual (≤ 2.5%)").size(10.0).color(theme.text_muted));
            });
    }
});

        ui.add_space(10.0);

        // Parameters Control Card (3 columns)
        theme.card_frame().show(ui, |ui| {
            ui_kit::responsive_grid_n(ui, 3, 240.0, |ui, col_idx| match col_idx {
                0 => {
                    // Column 1: Ground Motion & Time Stepping
                            ui.vertical(|ui| {
                                ui.label(RichText::new("1. Ground Motion Record & Scaling").size(11.0).strong().color(theme.text_primary));
                                ui.horizontal(|ui| {
                                    ui.label("Record:");
                                    egui::ComboBox::from_id_salt("tha_rec_cb")
                                        .selected_text(self.tha_record.label())
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut self.tha_record, crate::core::GroundMotionRecord::NorthridgeSylmar1994, "1994 Northridge (Sylmar)");
                                            ui.selectable_value(&mut self.tha_record, crate::core::GroundMotionRecord::KobeTakatori1995, "1995 Kobe (Takatori)");
                                            ui.selectable_value(&mut self.tha_record, crate::core::GroundMotionRecord::ImperialValleyElCentro1940, "1940 El Centro (Array #9)");
                                            ui.selectable_value(&mut self.tha_record, crate::core::GroundMotionRecord::ChiChiTaiwan1999, "1999 Chi-Chi Taiwan");
                                            ui.selectable_value(&mut self.tha_record, crate::core::GroundMotionRecord::SumatraSubduction2004, "2004 Sumatra Subduction");
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Scale Factor SF:");
                                    ui.add(egui::Slider::new(&mut self.tha_scale_factor, 0.50..=2.50));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Time Step Δt:");
                                    ui.add(egui::Slider::new(&mut self.tha_dt, 0.005..=0.020).suffix(" s"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Duration:");
                                    ui.add(egui::Slider::new(&mut self.tha_duration, 10.0..=40.0).suffix(" s"));
                                });
                            });
                }
                1 => {
                    // Column 2: Building Superstructure Archetype
                            ui.vertical(|ui| {
                                ui.label(RichText::new("2. Superstructure Geometry & Mass").size(11.0).strong().color(theme.text_primary));
                                ui.horizontal(|ui| {
                                    ui.label("Total Height (H):");
                                    ui.add(egui::Slider::new(&mut self.tha_building_height_m, 20.0..=120.0).suffix(" m"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Number of Stories (N):");
                                    ui.add(egui::Slider::new(&mut self.tha_num_stories, 4..=32));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Seismic Weight (W):");
                                    ui.add(egui::Slider::new(&mut self.tha_seismic_weight_kn, 20000.0..=400000.0).suffix(" kN"));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Period (T1):");
                                    ui.add(egui::Slider::new(&mut self.tha_t1_s, 0.50..=3.50).suffix(" s"));
                                });
                            });
                }
                _ => {
                    // Column 3: Inelastic Yield & Numerical Dissipation
                            ui.vertical(|ui| {
                                ui.label(RichText::new("3. Inelastic Yield & Energy Damping").size(11.0).strong().color(theme.text_primary));
                                ui.horizontal(|ui| {
                                    ui.label("Rayleigh Damping (ξ):");
                                    ui.add(egui::Slider::new(&mut self.tha_damping, 0.010..=0.060));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Yield Shear Coeff (Cy):");
                                    ui.add(egui::Slider::new(&mut self.tha_cy, 0.05..=0.30));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Hardening Ratio (αp):");
                                    ui.add(egui::Slider::new(&mut self.tha_alpha_p, 0.010..=0.100));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Solver Scheme:");
                                    ui.label(RichText::new("HHT-α (α=-0.10)").color(theme.accent_blue).strong());
                                });
                            });
                }
            });
        });

        ui.add_space(10.0);

        // 2D Vector CAD Canvas: Response Time Traces + Story Height Drift Profile
        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                icons::icon_label(ui, AppIcon::RulerSquare, 13.5, theme.accent_blue, "2D Vector CAD Dynamic Response Waves & Multi-Story Drift Envelope", 12.5, theme.accent_blue, true);
                ui.add_space(20.0);
                ui.label(RichText::new(format!("Scaled PGA: {:.3} g | Peak Roof Disp: {:.1} mm", rep.scaled_pga_g, rep.peak_roof_disp_mm)).size(11.0).color(theme.accent_green));
            });

            ui.add_space(6.0);

            let (canvas_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 270.0), Sense::hover());
            let painter = ui.painter_at(canvas_rect);

            painter.rect_filled(canvas_rect, 6.0, Color32::from_rgb(18, 22, 30));
            painter.rect_stroke(canvas_rect, 6.0, Stroke::new(1.0, Color32::from_rgb(45, 55, 75)), egui::StrokeKind::Inside);

            let mid_x = canvas_rect.min.x + canvas_rect.width() * 0.58;

            // Divider line between Time Wave and Drift Profile
            painter.line_segment(
                [Pos2::new(mid_x, canvas_rect.min.y + 10.0), Pos2::new(mid_x, canvas_rect.max.y - 10.0)],
                Stroke::new(1.0, Color32::from_rgb(45, 55, 75)),
            );

            // Left Side: Time-History Response Waves
            let wave_left = canvas_rect.min.x + 35.0;
            let wave_right = mid_x - 20.0;
            let wave_w = (wave_right - wave_left).max(10.0);
            let wave_center_y = canvas_rect.min.y + 130.0;
            let wave_h = 100.0;

            // Zero line
            painter.line_segment(
                [Pos2::new(wave_left, wave_center_y), Pos2::new(wave_right, wave_center_y)],
                Stroke::new(1.0, Color32::from_rgb(70, 80, 100)),
            );

            // Grid lines
            for t_step in 1..=4 {
                let gx = wave_left + wave_w * (t_step as f32 / 4.0);
                painter.line_segment(
                    [Pos2::new(gx, canvas_rect.min.y + 25.0), Pos2::new(gx, canvas_rect.max.y - 25.0)],
                    Stroke::new(0.5, Color32::from_rgb(35, 45, 60)),
                );
                let time_val = self.tha_duration * (t_step as f32 / 4.0);
                painter.text(
                    Pos2::new(gx, canvas_rect.max.y - 20.0),
                    Align2::CENTER_TOP,
                    format!("{:.0}s", time_val),
                    egui::FontId::proportional(9.5),
                    Color32::from_rgb(130, 140, 160),
                );
            }

            if !rep.time_history_trace.is_empty() {
                let n_pts = rep.time_history_trace.len();
                let max_u = rep.peak_roof_disp_mm.max(1.0) as f32;
                let max_ag = (rep.scaled_pga_g.max(0.1) * 9.80665) as f32;

                // 1. Ground acceleration wave (amber)
                let mut ag_points = Vec::with_capacity(n_pts.min(300));
                let step = (n_pts / 300).max(1);
                for i in (0..n_pts).step_by(step) {
                    let s = &rep.time_history_trace[i];
                    let px = wave_left + (s.time_s as f32 / self.tha_duration) * wave_w;
                    let py = wave_center_y - ((s.ground_accel_g as f32 * 9.80665) / max_ag) * (wave_h * 0.40);
                    ag_points.push(Pos2::new(px, py));
                }
                for w in ag_points.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(1.0, Color32::from_rgba_unmultiplied(245, 158, 11, 160)));
                }

                // 2. Roof displacement wave (cyan)
                let mut u_points = Vec::with_capacity(n_pts.min(300));
                for i in (0..n_pts).step_by(step) {
                    let s = &rep.time_history_trace[i];
                    let px = wave_left + (s.time_s as f32 / self.tha_duration) * wave_w;
                    let py = wave_center_y - (s.roof_disp_mm as f32 / max_u) * (wave_h * 0.42);
                    u_points.push(Pos2::new(px, py));
                }
                for w in u_points.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(1.8, Color32::from_rgb(56, 189, 248)));
                }
            }

            // Legend on Left
            painter.text(
                Pos2::new(wave_left, canvas_rect.min.y + 12.0),
                Align2::LEFT_TOP,
                "Dynamic Response Trace: — ag(t) (Ground)  — u_roof(t) (Displacement)",
                egui::FontId::proportional(11.0),
                Color32::from_rgb(56, 189, 248),
            );

            // Right Side: Multi-Story Drift Ratio Envelope Profile
            let profile_left = mid_x + 35.0;
            let profile_right = canvas_rect.max.x - 25.0;
            let profile_w = (profile_right - profile_left).max(10.0);
            let profile_bot = canvas_rect.max.y - 30.0;
            let profile_top = canvas_rect.min.y + 35.0;
            let profile_h = (profile_bot - profile_top).max(10.0);

            // Vertical axis (Height)
            painter.line_segment(
                [Pos2::new(profile_left, profile_bot), Pos2::new(profile_left, profile_top)],
                Stroke::new(1.2, Color32::from_rgb(70, 80, 100)),
            );

            // Horizontal axis (Drift 0 to 2.5%)
            painter.line_segment(
                [Pos2::new(profile_left, profile_bot), Pos2::new(profile_right, profile_bot)],
                Stroke::new(1.2, Color32::from_rgb(70, 80, 100)),
            );

            // Drift limit line (2.0% ASCE 7-22)
            let drift_limit_x = profile_left + profile_w * (0.020 / 0.025);
            painter.line_segment(
                [Pos2::new(drift_limit_x, profile_bot), Pos2::new(drift_limit_x, profile_top)],
                Stroke::new(1.2, Color32::from_rgb(239, 68, 68)),
            );
            painter.text(
                Pos2::new(drift_limit_x, profile_top - 12.0),
                Align2::CENTER_TOP,
                "Limit 2.0%",
                egui::FontId::proportional(9.0),
                Color32::from_rgb(239, 68, 68),
            );

            // Plot drift envelope line
            if !rep.story_profiles.is_empty() {
                let n_stories = rep.story_profiles.len();
                let mut drift_pts = Vec::with_capacity(n_stories + 1);
                drift_pts.push(Pos2::new(profile_left, profile_bot)); // Ground is 0 drift

                for p in &rep.story_profiles {
                    let y_frac = p.story_elevation_m as f32 / self.tha_building_height_m;
                    let py = profile_bot - y_frac * profile_h;
                    let x_frac = ((p.peak_drift_ratio as f32) / 0.025).clamp(0.0, 1.0);
                    let px = profile_left + x_frac * profile_w;
                    drift_pts.push(Pos2::new(px, py));
                }

                for w in drift_pts.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(2.2, Color32::from_rgb(16, 185, 129)));
                }
                for pt in &drift_pts {
                    painter.circle_filled(*pt, 2.5, Color32::from_rgb(52, 211, 153));
                }
            }

            painter.text(
                Pos2::new(profile_left, canvas_rect.min.y + 12.0),
                Align2::LEFT_TOP,
                "Inter-Story Drift Profile (θ vs Height)",
                egui::FontId::proportional(11.0),
                Color32::from_rgb(16, 185, 129),
            );
        });

        ui.add_space(10.0);

        // ASCE 7-22 / SNI 1726 Regulatory Compliance Audit Table
        theme.card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                icons::icon_label(ui, AppIcon::Clipboard, 13.5, theme.accent_blue, "ASCE 7-22 & SNI 1726:2019 NL-THA Compliance Audit", 12.5, theme.accent_blue, true);
                ui.add_space(20.0);
                let (st_icon, overall_status, status_col) = if rep.is_design_acceptable {
                    (AppIcon::CheckCircle, "ALL REGULATORY CHECKS SATISFIED", theme.accent_green)
                } else {
                    (AppIcon::AlertTriangle, "DESIGN REVISIONS REQUIRED", Color32::from_rgb(239, 68, 68))
                };
                icons::pill_badge(ui, st_icon, overall_status, status_col);
            });

            ui.add_space(6.0);

            // Table Header
            ui.horizontal(|ui| {
                ui.label(RichText::new("Code Clause").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(190.0);
                ui.label(RichText::new("Engineering Requirement").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(160.0);
                ui.label(RichText::new("Demand").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(50.0);
                ui.label(RichText::new("Capacity").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(50.0);
                ui.label(RichText::new("D/C Ratio").size(10.5).strong().color(theme.text_secondary));
                ui.add_space(30.0);
                ui.label(RichText::new("Status").size(10.5).strong().color(theme.text_secondary));
            });
            ui.separator();

            for item in &rep.regulatory_checks {
                let row_col = if item.is_passed { theme.accent_green } else { Color32::from_rgb(239, 68, 68) };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.code_clause).size(10.5).color(theme.accent_blue));
                    ui.add_space(10.0);
                    ui.label(RichText::new(&item.description).size(10.5).color(theme.text_primary));
                    ui.add_space(10.0);
                    ui.label(RichText::new(format!("{:.3} {}", item.demand_val, item.unit)).size(10.5).color(theme.text_primary));
                    ui.add_space(15.0);
                    ui.label(RichText::new(format!("{:.3} {}", item.capacity_val, item.unit)).size(10.5).color(theme.accent_green));
                    ui.add_space(15.0);
                    ui.label(RichText::new(format!("{:.3}", item.demand_capacity_ratio)).size(10.5).strong().color(row_col));
                    ui.add_space(20.0);
                    let (r_icon, r_txt) = if item.is_passed { (AppIcon::CheckCircle, "PASS") } else { (AppIcon::AlertTriangle, "FAIL") };
                    icons::icon_label(ui, r_icon, 11.0, row_col, r_txt, 10.5, row_col, true);
                });
                ui.separator();
            }
        });
    }


}
