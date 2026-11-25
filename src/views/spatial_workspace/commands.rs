//! Planning proposals, Gemini reply polling, context payload builder, and `/` command executor.

use super::*;

use crate::core::{
    compute_site_environmental_coupling, BimCommand, BimElementKind, BimProject,
};
use crate::models::{DatabaseComponent, NavigationPillar, ProjectItem, StructuralMember};
use crate::views::build::{Ground3DAuthorTool, Render3DMode};
use crate::views::universal_canvas::UniversalCanvasTool;
use crate::views::{AiAssistantView, BuildView, UniversalCanvasState};

impl SpatialWorkspaceState {
    /// Queues an interactive proposal review card in `Planning` mode before applying model changes.
    pub fn queue_planning_proposal(
        &mut self,
        title: impl Into<String>,
        rationale: impl Into<String>,
        action_preview: impl Into<String>,
        actions: Vec<PlannedProposalAction>,
    ) -> u64 {
        let id = self.next_proposal_id;
        self.next_proposal_id += 1;
        self.pending_proposals.push(PlannedAgentProposal {
            id,
            title: title.into(),
            rationale: rationale.into(),
            action_preview: action_preview.into(),
            actions,
            status: ProposalStatus::Pending,
        });
        if self.pending_proposals.len() > 8 {
            let excess = self.pending_proposals.len() - 8;
            self.pending_proposals.drain(0..excess);
        }
        self.command_bar_expanded = true;
        id
    }

    /// Commits a list of planned structural/Earth actions via transactional `BimCommandBus` / canvas state.
    pub fn apply_proposal_actions(
        &mut self,
        planned: &[PlannedProposalAction],
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
    ) -> SpatialHudActions {
        let mut out = SpatialHudActions::default();
        for item in planned {
            match item {
                PlannedProposalAction::ExecuteBimNl(cmd) => {
                    if let Ok(msg) = build_view.bim_bus.execute_natural_language(cmd) {
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        universal_canvas.globe_mode = true;
                        universal_canvas.ground_origin_anchored = true;
                        out.sync_fem_and_earth = true;
                        out.fit_camera = true;
                        self.push_trace(msg.clone(), true);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[Applied BIM Proposal]: {msg}"),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::AutoRepair => {
                    out.run_auto_repair = true;
                    self.push_trace("Queued Structural Auto-Repair", true);
                }
                PlannedProposalAction::FlyEarth {
                    lat,
                    lon,
                    zoom,
                    label,
                } => {
                    self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                    universal_canvas.globe_mode = true;
                    universal_canvas
                        .earth_world
                        .start_fly_to(&universal_canvas.world_map, *lat, *lon, *zoom as f32, label);
                    universal_canvas.ground_origin_lat_lon = (*lat, *lon);
                    universal_canvas.ground_origin_anchored = true;
                    self.selected_target = Some(SmartSelectionTarget::EarthSite {
                        lat: *lat,
                        lon: *lon,
                    });
                    self.push_trace(format!("Earth Fly-To -> {label}"), true);
                }
                PlannedProposalAction::AnchorSite { lat, lon } => {
                    out.anchor_and_build_site = Some((*lat, *lon));
                    self.push_trace(format!("Anchored BIM Site @ {lat:.3}°, {lon:.3}°"), true);
                }
                PlannedProposalAction::ApplyEigenGateLens => {
                    self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                    universal_canvas.globe_mode = true;
                    self.apply_lens_to_views(
                        SpatialLens::EigenGateRedundancy,
                        build_view,
                        universal_canvas,
                    );
                    out.sync_fem_and_earth = true;
                    self.push_trace("Applied EigenGate Redundancy Heatmap Lens on Earth 3D", true);
                }
                PlannedProposalAction::ResizeColumn {
                    column_id,
                    width_mm,
                    depth_mm,
                } => {
                    let rot = build_view
                        .bim_bus
                        .project
                        .columns
                        .iter()
                        .find(|c| c.id == *column_id)
                        .map(|c| c.rotation_deg)
                        .unwrap_or(0.0);
                    if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeColumn {
                        column_id: *column_id,
                        width_mm: *width_mm,
                        depth_mm: *depth_mm,
                        rotation_deg: rot,
                        profile_name: format!("RC {:.0}x{:.0}", width_mm, depth_mm),
                    }) {
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        out.sync_fem_and_earth = true;
                        self.push_trace(msg.clone(), true);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[Applied Column Resize]: {msg}"),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::ResizeBeam {
                    run_id,
                    width_mm,
                    depth_mm,
                } => {
                    if let Ok(msg) = build_view.bim_bus.execute(BimCommand::ResizeRun {
                        run_id: *run_id,
                        width_mm: *width_mm,
                        depth_mm: *depth_mm,
                        profile_name: format!("RC {:.0}x{:.0}", width_mm, depth_mm),
                    }) {
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        out.sync_fem_and_earth = true;
                        self.push_trace(msg.clone(), true);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[Applied Beam Resize]: {msg}"),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::AddColumnAt {
                    x_mm,
                    y_mm,
                    width_mm,
                    depth_mm,
                } => {
                    let mut lvls = build_view.bim_bus.project.levels.clone();
                    lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
                    let base_level_id = lvls.first().map(|l| l.id).unwrap_or(1);
                    let top_level_id = lvls.get(1).map(|l| l.id).unwrap_or(base_level_id);
                    if let Ok(msg) = build_view.bim_bus.execute(BimCommand::AddColumn {
                        base_level_id,
                        top_level_id,
                        pos: crate::core::P2(*x_mm, *y_mm),
                        width_mm: *width_mm,
                        depth_mm: *depth_mm,
                        is_round: false,
                        profile_name: format!("C{:.0}x{:.0}", width_mm, depth_mm),
                    }) {
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        universal_canvas.globe_mode = true;
                        universal_canvas.ground_origin_anchored = true;
                        out.sync_fem_and_earth = true;
                        out.fit_camera = true;
                        self.push_trace(msg.clone(), true);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!("[Added Column]: {msg}"),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::ApplySiteSeismic => {
                    out.apply_site_seismic = true;
                    out.sync_fem_and_earth = true;
                    self.push_trace("Coupled Earth Site Seismic & Wind Loads to FEM", true);
                }
                PlannedProposalAction::OpenSheet(pillar) => {
                    self.open_module_overlay(*pillar);
                    self.push_trace(format!("Opened {} Sheet", pillar.clean_name()), true);
                }
                PlannedProposalAction::AddStory => {
                    if self.add_story_above_top(build_view).is_ok() {
                        out.sync_fem_and_earth = true;
                        out.fit_camera = true;
                    }
                }
                PlannedProposalAction::AddSlab => {
                    if self.add_slab_at_active_story(build_view).is_ok() {
                        out.sync_fem_and_earth = true;
                    }
                }
                PlannedProposalAction::BatchPinCity { query, city_name } => {
                    if let Some((canon, lat, lon)) =
                        crate::core::gis::SlippyMapState::resolve_city_hint_in_text(city_name)
                    {
                        let moved = universal_canvas
                            .world_index
                            .relocate_city_batch(query, lat, lon, &canon);
                        if moved > 0 {
                            crate::core::InternalVault::save_world_spatial_index(&universal_canvas.world_index);
                        }
                        self.push_trace(format!("Batch-pinned {moved} models matching '{query}' -> {canon}"), moved > 0);
                        ai_assistant.chat_history.push(crate::models::ChatMessage {
                            is_user: false,
                            message: format!(
                                "[World Index]: Relocated {moved} Staging model(s) matching '{query}' onto {canon} ({lat:.4}°, {lon:.4}°)."
                            ),
                            time: crate::core::web_bridge::unix_timestamp_label(),
                        });
                    }
                }
                PlannedProposalAction::PinActiveBuilding { lat, lon, heading_deg } => {
                    universal_canvas.ground_origin_lat_lon = (*lat, *lon);
                    universal_canvas.ground_origin_anchored = true;
                    if let Some(rel) = universal_canvas.active_world_project_rel_path.clone() {
                        universal_canvas.world_index.pin_building_at(&rel, *lat, *lon, *heading_deg);
                        universal_canvas.pending_world_pin_event =
                            Some((rel, *lat, *lon, *heading_deg));
                    }
                    universal_canvas.earth_world.start_fly_to(
                        &universal_canvas.world_map,
                        *lat,
                        *lon,
                        18.0,
                        "Pinned Active Site",
                    );
                    universal_canvas.refresh_neighborhood_coupling(&build_view.bim_bus.project);
                    out.sync_fem_and_earth = true;
                    self.push_trace(format!("Pinned active model @ ({lat:.4}°, {lon:.4}°)"), true);
                }
                PlannedProposalAction::AuditNeighborhood => {
                    universal_canvas.refresh_neighborhood_coupling(&build_view.bim_bus.project);
                    let rep = &universal_canvas.neighborhood_report;
                    let summary = format!(
                        "[Neighborhood Coupled Audit]: Evaluated {} neighbor(s) within 220m • Pounding Risks={} • Tight Buffers={} • Min Gap={} • Recommended Wind Exposure={:?} • Venturi Speedup={:.2}x • Cascade Overlap={}",
                        rep.neighbors_evaluated,
                        rep.pounding_risk_count,
                        rep.tight_buffer_count,
                        rep.min_clear_gap_m.map_or("N/A".to_string(), |g| format!("{g:.2}m")),
                        rep.recommended_exposure,
                        rep.venturi_speedup_factor,
                        rep.cascade_overlap_count,
                    );
                    ai_assistant.chat_history.push(crate::models::ChatMessage {
                        is_user: false,
                        message: summary,
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
                    self.push_trace(
                        format!(
                            "Neighborhood: {} checked, {} pounding risk(s)",
                            rep.neighbors_evaluated, rep.pounding_risk_count
                        ),
                        rep.pounding_risk_count == 0,
                    );
                }
                PlannedProposalAction::EnableTerrainContours => {
                    universal_canvas.show_terrain_contours = true;
                    universal_canvas.ensure_site_tin_generated();
                    self.push_trace("Enabled 3D TIN Terrain Contours & Slope Hazard", true);
                }
                PlannedProposalAction::SwitchPureVector => {
                    universal_canvas.world_map.tile_provider =
                        crate::core::gis::VectorEarthTheme::CoastalBlueprint;
                    universal_canvas.world_map.vector_site_overlay = true;
                    self.push_trace("Activated Coastal Blueprint Pure-Rust Vector Earth", true);
                }
                PlannedProposalAction::ApplyDeformedLens => {
                    self.apply_lens_to_views(SpatialLens::DeformedShape, build_view, universal_canvas);
                    self.push_trace("Activated 6-DOF Deformed Shape lens on Earth 3D", true);
                }
                PlannedProposalAction::ApplyModalLens(mode_idx) => {
                    universal_canvas.earth_bim_author.active_modal_mode = (*mode_idx).min(2);
                    universal_canvas.earth_bim_author.modal_anim_playing = true;
                    self.apply_lens_to_views(SpatialLens::ModalVibration, build_view, universal_canvas);
                    self.push_trace(
                        format!("Activated Animated Modal Mode {} on Earth 3D", mode_idx + 1),
                        true,
                    );
                }
                PlannedProposalAction::SimulateN1(fid) => {
                    out.toggle_n1_member = Some(*fid);
                    self.push_trace(format!("Triggered N-1 disruption on FEM #{fid}"), true);
                }
                PlannedProposalAction::RestoreN1 => {
                    out.restore_n1_simulation = true;
                    self.push_trace("Restored intact structure (N-1 cleared)", true);
                }
                PlannedProposalAction::RunEqReplay(rec_opt) => {
                    if let Some(rec) = rec_opt {
                        universal_canvas.ground_motion_replay_config.record = *rec;
                    }
                    self.apply_lens_to_views(
                        SpatialLens::TimeHistoryReplay,
                        build_view,
                        universal_canvas,
                    );
                    self.vault_drawer_open = false;
                    self.vault_path_popover_open = false;
                    self.context_picker_open = false;
                    self.history_popover_open = false;
                    self.api_key_popover_open = false;
                    self.ifcx_diff_drawer_open = false;
                    self.portfolio_drawer_open = false;
                    self.ground_motion_drawer_open = true;
                    out.run_async_ground_motion_replay = true;
                    self.push_trace("Dispatched Async StructGNN-CRC & EQ Replay", true);
                }
                PlannedProposalAction::TogglePlasticHinges => {
                    universal_canvas.ground_motion_replay_config.show_plastic_hinges =
                        !universal_canvas.ground_motion_replay_config.show_plastic_hinges;
                    self.apply_lens_to_views(
                        SpatialLens::TimeHistoryReplay,
                        build_view,
                        universal_canvas,
                    );
                    self.push_trace(
                        format!(
                            "3D ASCE 41-17 Plastic Hinges: {}",
                            if universal_canvas.ground_motion_replay_config.show_plastic_hinges {
                                "ON"
                            } else {
                                "OFF"
                            }
                        ),
                        true,
                    );
                }
                PlannedProposalAction::ToggleTriDirectional(mode_opt) => {
                    let next = mode_opt.unwrap_or_else(|| {
                        universal_canvas.ground_motion_replay_config.tridirectional_mode.next()
                    });
                    universal_canvas.ground_motion_replay_config.tridirectional_mode = next;
                    self.apply_lens_to_views(
                        SpatialLens::TimeHistoryReplay,
                        build_view,
                        universal_canvas,
                    );
                    out.run_async_ground_motion_replay = true;
                    self.push_trace(format!("Tri-Directional coupling: {}", next.short_label()), true);
                }
                PlannedProposalAction::ToggleAutoMatchSa => {
                    let next = !universal_canvas.ground_motion_replay_config.auto_match_sa_t1;
                    universal_canvas.ground_motion_replay_config.auto_match_sa_t1 = next;
                    self.ground_motion_drawer_tab = 1;
                    self.open_ground_motion_drawer();
                    out.run_async_ground_motion_replay = true;
                    self.push_trace(
                        format!("Auto Match Sa(T1) spectrum scaling: {}", if next { "ON" } else { "OFF" }),
                        true,
                    );
                }
                PlannedProposalAction::OpenShmDigitalTwin => {
                    self.shm_drawer_tab = 0;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    out.run_shm_digital_twin = true;
                    self.push_trace("Opened Live SHM Digital Twin & Modal Identification", true);
                }
                PlannedProposalAction::RunAtc20Triage => {
                    self.shm_drawer_tab = 1;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    out.run_shm_digital_twin = true;
                    self.push_trace("Executed Automated ATC-20 Post-Earthquake Placard Triage", true);
                }
                PlannedProposalAction::RunEvacuationRouting
                | PlannedProposalAction::OpenEvacuationDrawer => {
                    universal_canvas.evacuation_config.overlay_visible = true;
                    self.open_evacuation_drawer();
                    out.run_evacuation_routing = true;
                    self.push_trace("Computed Autonomous Multi-Hazard Evacuation & Emergency Lifelines", true);
                }
                PlannedProposalAction::StartShmWatchdog => {
                    self.shm_drawer_tab = 2;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    self.push_trace("Opened Live WebSocket SHM Watchdog", true);
                }
                PlannedProposalAction::ConnectShmWs(url) => {
                    self.shm_drawer_tab = 2;
                    self.open_shm_drawer();
                    out.connect_shm_ws = Some(url.clone());
                    self.push_trace(format!("Armed SHM WebSocket stream: {url}"), true);
                }
                PlannedProposalAction::TriggerWatchdogSim(mode_str) => {
                    let mode = crate::core::WatchdogSimMode::from_keyword(mode_str)
                        .unwrap_or(crate::core::WatchdogSimMode::AftershockPulse);
                    universal_canvas.shm_watchdog.start_simulator(mode);
                    self.shm_drawer_tab = 2;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    self.push_trace(format!("Started Live Watchdog Simulator: {}", mode.label()), true);
                }
                PlannedProposalAction::RunGpuMonteCarlo(n) => {
                    universal_canvas.monte_carlo_config.sample_count = (*n).clamp(256, 16_384);
                    self.ground_motion_drawer_tab = 2;
                    self.toggle_exclusive_drawer("ground_motion");
                    out.run_gpu_monte_carlo = Some(universal_canvas.monte_carlo_config.sample_count);
                    self.push_trace(
                        format!("Running GPU @compute FEMA P-58 Monte Carlo ({} samples)", n),
                        true,
                    );
                }
                PlannedProposalAction::ToggleGpuParticles => {
                    universal_canvas.monte_carlo_config.gpu_particles_enabled =
                        !universal_canvas.monte_carlo_config.gpu_particles_enabled;
                    let on = universal_canvas.monte_carlo_config.gpu_particles_enabled;
                    if on && !universal_canvas.district_hazard.any_active() {
                        universal_canvas.district_hazard.seismic_wave_active = true;
                        universal_canvas.district_hazard.venturi_wind_active = true;
                    }
                    self.push_trace(
                        if on {
                            format!(
                                "GPU @compute Hazard Particles ON ({})",
                                universal_canvas.monte_carlo_config.normalized_particle_count()
                            )
                        } else {
                            "GPU @compute Hazard Particles OFF".to_string()
                        },
                        true,
                    );
                }
                PlannedProposalAction::ExportMcIfcx => {
                    out.export_monte_carlo_ifcx = true;
                }
                PlannedProposalAction::CreateCheckpoint(lbl) => {
                    let clean_lbl = if lbl.trim().is_empty() {
                        universal_canvas.collab_state.checkpoint_label_draft.clone()
                    } else {
                        lbl.trim().to_string()
                    };
                    self.open_collab_drawer(0);
                    out.create_checkpoint_label = Some(clean_lbl);
                }
                PlannedProposalAction::ApproveCheckpoint => {
                    self.open_collab_drawer(0);
                    out.approve_checkpoint = true;
                }
                PlannedProposalAction::RunIfcxMerge => {
                    self.open_collab_drawer(0);
                    out.run_ifcx_three_way_merge = true;
                }
                PlannedProposalAction::ToggleRiskHeatmap(metric_opt) => {
                    if let Some(m) = metric_opt {
                        universal_canvas.collab_state.config.risk_metric = *m;
                        universal_canvas.collab_state.config.risk_heatmap_visible = true;
                    } else {
                        universal_canvas.collab_state.config.risk_heatmap_visible =
                            !universal_canvas.collab_state.config.risk_heatmap_visible;
                    }
                    self.open_collab_drawer(1);
                    if universal_canvas.collab_state.config.risk_heatmap_visible {
                        out.refresh_district_risk_field = true;
                    }
                }
                PlannedProposalAction::RunTradeStudy(obj_opt) => {
                    let obj = obj_opt.unwrap_or(universal_canvas.trade_study_config.objective);
                    universal_canvas.trade_study_config.objective = obj;
                    self.open_collab_drawer(2);
                    out.run_trade_study = Some(obj);
                    out.run_permit_compliance = true;
                }
                PlannedProposalAction::ExportPermitAnnex => {
                    self.open_collab_drawer(2);
                    out.export_permit_annex_svg = true;
                }
                PlannedProposalAction::SyncPeerRelay(url_opt) => {
                    self.open_collab_drawer(0);
                    if let Some(u) = url_opt {
                        out.connect_collab_relay = Some(u.clone());
                    }
                    out.sync_peer_relay_delta = true;
                }
                PlannedProposalAction::RunBbsOptimize(stock_m) => {
                    universal_canvas.field_fab_config.stock_length_m = *stock_m;
                    self.open_field_qaqc_drawer(0);
                    out.run_field_fabrication = Some(*stock_m);
                }
                PlannedProposalAction::ToggleQaqc3d => {
                    self.open_field_qaqc_drawer(1);
                    out.toggle_field_qaqc_3d = true;
                }
                PlannedProposalAction::ExportShopSvg => {
                    self.open_field_qaqc_drawer(0);
                    out.export_shop_drawing_svg = true;
                }
                PlannedProposalAction::RunEigenFlow => {
                    self.open_eigenflow_drawer(0);
                    out.run_eigenflow_studio = true;
                    self.push_trace("Queued Branch 2 EigenFlow Non-Linear Woodbury check", true);
                }
                PlannedProposalAction::OpenEigenFlowStudio => {
                    self.open_eigenflow_drawer(0);
                    out.run_eigenflow_studio = true;
                }
                PlannedProposalAction::RunPushover(pat) => {
                    self.open_eigenflow_drawer(1);
                    out.eigenflow_set_pattern = Some(*pat);
                }
                PlannedProposalAction::ToggleEigenFlow3d => {
                    self.open_eigenflow_drawer(0);
                    out.toggle_eigenflow_3d = true;
                }
                PlannedProposalAction::ExportEigenFlowAnnex => {
                    self.open_eigenflow_drawer(1);
                    out.export_eigenflow_svg = true;
                }
                PlannedProposalAction::RunBlast { mass_kg, standoff_m } => {
                    universal_canvas.extreme_events_config.mode =
                        crate::core::extreme_events::ExtremeEventMode::Blast;
                    universal_canvas.extreme_events_config.explosive_mass_kg = *mass_kg;
                    universal_canvas.extreme_events_config.standoff_distance_m = *standoff_m;
                    universal_canvas.extreme_events_overlay_visible = true;
                    self.open_extreme_events_drawer(0);
                    out.run_extreme_events = true;
                }
                PlannedProposalAction::RunFire { minutes, story } => {
                    universal_canvas.extreme_events_config.mode =
                        crate::core::extreme_events::ExtremeEventMode::StructuralFire;
                    universal_canvas.extreme_events_config.exposure_minutes = *minutes;
                    if let Some(st) = story {
                        universal_canvas.extreme_events_config.fire_story_idx = *st;
                    }
                    universal_canvas.extreme_events_overlay_visible = true;
                    self.open_extreme_events_drawer(1);
                    out.run_extreme_events = true;
                }
                PlannedProposalAction::ToggleExtreme3d => {
                    self.open_extreme_events_drawer(0);
                    out.toggle_extreme_events_3d = true;
                }
                PlannedProposalAction::ExportExtremeAnnex => {
                    self.open_extreme_events_drawer(0);
                    out.export_extreme_events_svg = true;
                }
                PlannedProposalAction::RunIsolation(iso) => {
                    universal_canvas.isolation_studio_config.isolator_type = *iso;
                    universal_canvas.isolation_overlay_visible = true;
                    self.open_isolation_drawer(0);
                    out.run_isolation_studio = true;
                }
                PlannedProposalAction::ToggleIsolation3d => {
                    self.open_isolation_drawer(0);
                    out.toggle_isolation_3d = true;
                }
                PlannedProposalAction::ExportIsolationAnnex => {
                    self.open_isolation_drawer(0);
                    out.export_isolation_svg = true;
                }
                PlannedProposalAction::RunWind {
                    speed_ms,
                    exposure,
                    azimuth_deg,
                } => {
                    if let Some(v) = speed_ms {
                        universal_canvas.wind_studio_config.basic_wind_speed_ms = *v;
                    }
                    if let Some(exp) = exposure {
                        universal_canvas.wind_studio_config.exposure = *exp;
                    }
                    if let Some(az) = azimuth_deg {
                        universal_canvas.wind_studio_config.wind_azimuth_deg = *az;
                    }
                    universal_canvas.wind_overlay_visible = true;
                    self.open_wind_drawer(0);
                    out.run_wind_studio = true;
                }
                PlannedProposalAction::ToggleWind3d => {
                    self.open_wind_drawer(0);
                    out.toggle_wind_3d = true;
                }
                PlannedProposalAction::ExportWindAnnex => {
                    self.open_wind_drawer(0);
                    out.export_wind_svg = true;
                }
                PlannedProposalAction::RunShearWall {
                    tw_mm,
                    fc_mpa,
                    drift_ratio,
                } => {
                    if let Some(tw) = tw_mm {
                        universal_canvas.shear_wall_studio_config.wall_thickness_mm = *tw;
                    }
                    if let Some(fc) = fc_mpa {
                        universal_canvas.shear_wall_studio_config.fc_mpa = *fc;
                    }
                    if let Some(dr) = drift_ratio {
                        universal_canvas.shear_wall_studio_config.design_drift_ratio = *dr;
                    }
                    universal_canvas.shear_wall_overlay_visible = true;
                    self.open_shear_wall_drawer(0);
                    out.run_shear_wall_studio = true;
                }
                PlannedProposalAction::ToggleShearWall3d => {
                    self.open_shear_wall_drawer(0);
                    out.toggle_shear_wall_3d = true;
                }
                PlannedProposalAction::ExportShearWallAnnex => {
                    self.open_shear_wall_drawer(0);
                    out.export_shear_wall_svg = true;
                }
                PlannedProposalAction::RunConnection {
                    conn_type,
                    bolt_dia_mm,
                    plate_thick_mm,
                } => {
                    if let Some(ct) = conn_type {
                        universal_canvas.connection_studio_config.connection_type = *ct;
                    }
                    if let Some(db) = bolt_dia_mm {
                        universal_canvas.connection_studio_config.bolt_diameter_mm = *db;
                    }
                    if let Some(tp) = plate_thick_mm {
                        universal_canvas.connection_studio_config.plate_thickness_mm = *tp;
                    }
                    universal_canvas.connection_overlay_visible = true;
                    self.open_connection_drawer(0);
                    out.run_connection_studio = true;
                }
                PlannedProposalAction::ToggleConnection3d => {
                    self.open_connection_drawer(0);
                    out.toggle_connection_3d = true;
                }
                PlannedProposalAction::ExportConnectionAnnex => {
                    self.open_connection_drawer(0);
                    out.export_connection_svg = true;
                }
                PlannedProposalAction::RunPrestress {
                    profile,
                    num_strands,
                    fc_mpa,
                } => {
                    if let Some(pr) = profile {
                        universal_canvas.prestress_studio_config.profile_kind = *pr;
                    }
                    if let Some(ns) = num_strands {
                        universal_canvas.prestress_studio_config.num_strands = *ns;
                    }
                    if let Some(fc) = fc_mpa {
                        universal_canvas.prestress_studio_config.fc_prime_mpa = *fc;
                    }
                    universal_canvas.prestress_overlay_visible = true;
                    self.open_prestress_drawer(0);
                    out.run_prestress_studio = true;
                }
                PlannedProposalAction::TogglePrestress3d => {
                    self.open_prestress_drawer(0);
                    out.toggle_prestress_3d = true;
                }
                PlannedProposalAction::ExportPrestressAnnex => {
                    self.open_prestress_drawer(0);
                    out.export_prestress_svg = true;
                }
                PlannedProposalAction::RunComposite {
                    studs,
                    orientation,
                    unshored,
                } => {
                    if let Some(st) = studs {
                        universal_canvas.composite_studio_config.studs_provided = *st;
                    }
                    if let Some(or) = orientation {
                        universal_canvas.composite_studio_config.deck_orientation = *or;
                    }
                    if let Some(un) = unshored {
                        universal_canvas.composite_studio_config.unshored = *un;
                    }
                    universal_canvas.composite_overlay_visible = true;
                    self.open_composite_drawer(0);
                    out.run_composite_studio = true;
                }
                PlannedProposalAction::ToggleComposite3d => {
                    self.open_composite_drawer(0);
                    out.toggle_composite_3d = true;
                }
                PlannedProposalAction::ExportCompositeAnnex => {
                    self.open_composite_drawer(0);
                    out.export_composite_svg = true;
                }
                PlannedProposalAction::RunReliability {
                    method,
                    target_beta,
                    limit_state,
                } => {
                    if let Some(m) = method {
                        universal_canvas.reliability_studio_config.method = *m;
                    }
                    if let Some(tb) = target_beta {
                        universal_canvas.reliability_studio_config.target_beta = tb.clamp(2.0, 5.5);
                    }
                    if let Some(ls) = limit_state {
                        universal_canvas.reliability_studio_config.limit_state = *ls;
                    }
                    universal_canvas.reliability_overlay_visible = true;
                    self.open_reliability_drawer(0);
                    out.run_reliability_studio = true;
                }
                PlannedProposalAction::ToggleReliability3d => {
                    self.open_reliability_drawer(0);
                    out.toggle_reliability_3d = true;
                }
                PlannedProposalAction::ExportReliabilityAnnex => {
                    self.open_reliability_drawer(0);
                    out.export_reliability_svg = true;
                }
                PlannedProposalAction::RunFoundation {
                    soil_type,
                    piles_x,
                    piles_y,
                    pile_length_m,
                } => {
                    if let Some(st) = soil_type {
                        universal_canvas.foundation_studio_config.soil_type = *st;
                    }
                    if let Some(nx) = piles_x {
                        universal_canvas.foundation_studio_config.piles_x = *nx;
                    }
                    if let Some(ny) = piles_y {
                        universal_canvas.foundation_studio_config.piles_y = *ny;
                    }
                    if let Some(lm) = pile_length_m {
                        universal_canvas.foundation_studio_config.pile_length_m = lm.clamp(6.0, 48.0);
                    }
                    universal_canvas.foundation_overlay_visible = true;
                    self.open_foundation_drawer(0);
                    out.run_foundation_studio = true;
                }
                PlannedProposalAction::ToggleFoundation3d => {
                    self.open_foundation_drawer(0);
                    out.toggle_foundation_3d = true;
                }
                PlannedProposalAction::ExportFoundationAnnex => {
                    self.open_foundation_drawer(0);
                    out.export_foundation_svg = true;
                }
                PlannedProposalAction::RunConstructionLca {
                    active_day,
                    steel_route,
                    concrete_mix,
                } => {
                    if let Some(d) = active_day {
                        universal_canvas.construction_lca_config.active_day = (*d).min(540);
                    }
                    if let Some(sr) = steel_route {
                        universal_canvas.construction_lca_config.steel_route = *sr;
                    }
                    if let Some(cm) = concrete_mix {
                        universal_canvas.construction_lca_config.concrete_mix = *cm;
                    }
                    universal_canvas.construction_lca_overlay_visible = true;
                    self.open_construction_lca_drawer(0);
                    out.run_construction_lca_studio = true;
                }
                PlannedProposalAction::ToggleConstruction3d => {
                    self.open_construction_lca_drawer(0);
                    out.toggle_construction_3d = true;
                }
                PlannedProposalAction::ExportConstructionAnnex => {
                    self.open_construction_lca_drawer(0);
                    out.export_construction_svg = true;
                }
                PlannedProposalAction::InjectShmFault {
                    scenario,
                    severity_pct,
                    story,
                } => {
                    let parsed = crate::core::ShmFaultScenario::from_keyword(scenario)
                        .unwrap_or(crate::core::ShmFaultScenario::SoftStoryFirstFloor);
                    universal_canvas.shm_twin_config.fault_scenario = parsed;
                    universal_canvas.shm_twin_config.severity_pct = severity_pct.clamp(0.0, 100.0);
                    universal_canvas.shm_twin_config.target_story = (*story).max(1);
                    self.shm_drawer_tab = 0;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    out.run_shm_digital_twin = true;
                    self.push_trace(
                        format!(
                            "Injected SHM Fault '{}' ({:.0}% @ L{})",
                            parsed.short_label(),
                            universal_canvas.shm_twin_config.severity_pct,
                            universal_canvas.shm_twin_config.target_story
                        ),
                        true,
                    );
                }
                PlannedProposalAction::OpenFragilityTab => {
                    universal_canvas.ground_motion_replay_config.show_fragility_loss_overlay = true;
                    self.ground_motion_drawer_tab = 1;
                    self.open_ground_motion_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::TimeHistoryReplay,
                        build_view,
                        universal_canvas,
                    );
                    if universal_canvas.last_ground_motion_replay_report.is_none() {
                        out.run_async_ground_motion_replay = true;
                    }
                    self.push_trace("Opened Real-Time FEMA P-58 Fragility & Spectrum tab", true);
                }
            }
        }
        self.sessions_dirty = true;
        out
    }

    /// Commits a pending proposal card by ID when the user clicks `Apply Changes`.
    pub fn apply_proposal_by_id(
        &mut self,
        id: u64,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
    ) -> SpatialHudActions {
        let mut to_run: Option<Vec<PlannedProposalAction>> = None;
        if let Some(p) = self.pending_proposals.iter_mut().find(|p| p.id == id) {
            if p.status == ProposalStatus::Pending {
                p.status = ProposalStatus::Applied;
                to_run = Some(p.actions.clone());
            }
        }
        if let Some(actions_list) = to_run {
            self.apply_proposal_actions(&actions_list, universal_canvas, build_view, ai_assistant)
        } else {
            SpatialHudActions::default()
        }
    }

    /// Dismisses a pending proposal card by ID when the user clicks `Reject`.
    pub fn reject_proposal_by_id(&mut self, id: u64, ai_assistant: &mut AiAssistantView) {
        let mut rejected_title: Option<String> = None;
        if let Some(p) = self.pending_proposals.iter_mut().find(|p| p.id == id) {
            if p.status == ProposalStatus::Pending {
                p.status = ProposalStatus::Rejected;
                rejected_title = Some(p.title.clone());
            }
        }
        if let Some(title) = rejected_title {
            self.push_trace(format!("Rejected proposal: {title}"), false);
            ai_assistant.chat_history.push(crate::models::ChatMessage {
                is_user: false,
                message: format!("[Planning Mode]: Proposal '{title}' rejected; model unchanged."),
                time: crate::core::web_bridge::unix_timestamp_label(),
            });
            self.sessions_dirty = true;
        }
    }

    /// Polls asynchronous Gemini REST API replies and routes any structured `[[...]]` directives.
    pub fn poll_gemini_replies(
        &mut self,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
    ) -> SpatialHudActions {
        let replies = crate::core::web_bridge::gemini_poll_responses();
        let mut out = SpatialHudActions::default();
        for reply in replies {
            if self.gemini_inflight_req_id == Some(reply.request_id) {
                self.gemini_inflight_req_id = None;
            }
            if reply.ok {
                let (clean_text, directives) = parse_gemini_action_directives(&reply.text);
                let msg_idx = ai_assistant.chat_history.len();
                let final_msg = if clean_text.is_empty() {
                    format!("[{}] Prepared {} engineering directive(s).", reply.model, directives.len())
                } else {
                    format!("[{}]:\n{}", reply.model, clean_text)
                };
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: false,
                    message: final_msg,
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                self.turn_metadata.push(AntigravityTurnMeta {
                    msg_index: msg_idx,
                    mode_label: self.agent_mode.label().to_string(),
                    model_label: self.agent_model.short_label().to_string(),
                    context_chips: Vec::new(),
                    trace_steps: vec![format!("Gemini ({}) replied", reply.model)],
                });
                if !directives.is_empty() {
                    let mapped: Vec<PlannedProposalAction> = directives
                        .iter()
                        .map(|d| match d {
                            GeminiDirective::Fly { lat, lon, zoom } => PlannedProposalAction::FlyEarth {
                                lat: *lat,
                                lon: *lon,
                                zoom: *zoom,
                                label: format!("Gemini Target ({lat:.2}°, {lon:.2}°)"),
                            },
                            GeminiDirective::Bim(cmd) => PlannedProposalAction::ExecuteBimNl(cmd.clone()),
                            GeminiDirective::Repair => PlannedProposalAction::AutoRepair,
                            GeminiDirective::EigenGateAudit => PlannedProposalAction::ApplyEigenGateLens,
                            GeminiDirective::ResizeColumn { id, w_mm, d_mm } => {
                                PlannedProposalAction::ResizeColumn {
                                    column_id: *id,
                                    width_mm: *w_mm,
                                    depth_mm: *d_mm,
                                }
                            }
                            GeminiDirective::ResizeBeam { id, b_mm, h_mm } => {
                                PlannedProposalAction::ResizeBeam {
                                    run_id: *id,
                                    width_mm: *b_mm,
                                    depth_mm: *h_mm,
                                }
                            }
                            GeminiDirective::AddColumn { x_mm, y_mm, w_mm, d_mm } => {
                                PlannedProposalAction::AddColumnAt {
                                    x_mm: *x_mm,
                                    y_mm: *y_mm,
                                    width_mm: *w_mm,
                                    depth_mm: *d_mm,
                                }
                            }
                            GeminiDirective::ApplySiteSeismic => PlannedProposalAction::ApplySiteSeismic,
                            GeminiDirective::OpenSheet(pillar) => {
                                PlannedProposalAction::OpenSheet(*pillar)
                            }
                            GeminiDirective::AddStory => PlannedProposalAction::AddStory,
                            GeminiDirective::AddSlab => PlannedProposalAction::AddSlab,
                            GeminiDirective::BatchPinCity { query, city_name } => {
                                PlannedProposalAction::BatchPinCity {
                                    query: query.clone(),
                                    city_name: city_name.clone(),
                                }
                            }
                            GeminiDirective::PinActiveBuilding { lat, lon, heading_deg } => {
                                PlannedProposalAction::PinActiveBuilding {
                                    lat: *lat,
                                    lon: *lon,
                                    heading_deg: *heading_deg,
                                }
                            }
                            GeminiDirective::AuditNeighborhood => PlannedProposalAction::AuditNeighborhood,
                            GeminiDirective::EnableTerrainContours => PlannedProposalAction::EnableTerrainContours,
                            GeminiDirective::SwitchPureVector => PlannedProposalAction::SwitchPureVector,
                            GeminiDirective::ApplyDeformedLens => PlannedProposalAction::ApplyDeformedLens,
                            GeminiDirective::ApplyModalLens(m) => PlannedProposalAction::ApplyModalLens(*m),
                            GeminiDirective::SimulateN1(fid) => PlannedProposalAction::SimulateN1(*fid),
                            GeminiDirective::RestoreN1 => PlannedProposalAction::RestoreN1,
                            GeminiDirective::RunEqReplay(r) => PlannedProposalAction::RunEqReplay(*r),
                            GeminiDirective::TogglePlasticHinges => {
                                PlannedProposalAction::TogglePlasticHinges
                            }
                            GeminiDirective::ToggleTriDirectional(m) => {
                                PlannedProposalAction::ToggleTriDirectional(*m)
                            }
                            GeminiDirective::ToggleAutoMatchSa => {
                                PlannedProposalAction::ToggleAutoMatchSa
                            }
                            GeminiDirective::OpenFragilityTab => {
                                PlannedProposalAction::OpenFragilityTab
                            }
                            GeminiDirective::OpenShmDigitalTwin => {
                                PlannedProposalAction::OpenShmDigitalTwin
                            }
                            GeminiDirective::RunAtc20Triage => {
                                PlannedProposalAction::RunAtc20Triage
                            }
                            GeminiDirective::RunEvacuationRouting => {
                                PlannedProposalAction::RunEvacuationRouting
                            }
                            GeminiDirective::OpenEvacuationDrawer => {
                                PlannedProposalAction::OpenEvacuationDrawer
                            }
                            GeminiDirective::StartShmWatchdog => {
                                PlannedProposalAction::StartShmWatchdog
                            }
                            GeminiDirective::ConnectShmWs(url) => {
                                PlannedProposalAction::ConnectShmWs(url.clone())
                            }
                            GeminiDirective::TriggerWatchdogSim(mode) => {
                                PlannedProposalAction::TriggerWatchdogSim(mode.clone())
                            }
                            GeminiDirective::RunGpuMonteCarlo(n) => {
                                PlannedProposalAction::RunGpuMonteCarlo(*n)
                            }
                            GeminiDirective::ToggleGpuParticles => {
                                PlannedProposalAction::ToggleGpuParticles
                            }
                            GeminiDirective::ExportMcIfcx => PlannedProposalAction::ExportMcIfcx,
                            GeminiDirective::CreateCheckpoint(lbl) => {
                                PlannedProposalAction::CreateCheckpoint(lbl.clone())
                            }
                            GeminiDirective::ApproveCheckpoint => {
                                PlannedProposalAction::ApproveCheckpoint
                            }
                            GeminiDirective::RunIfcxMerge => PlannedProposalAction::RunIfcxMerge,
                            GeminiDirective::ToggleRiskHeatmap(m) => {
                                PlannedProposalAction::ToggleRiskHeatmap(*m)
                            }
                            GeminiDirective::RunTradeStudy(obj) => {
                                PlannedProposalAction::RunTradeStudy(*obj)
                            }
                            GeminiDirective::ExportPermitAnnex => {
                                PlannedProposalAction::ExportPermitAnnex
                            }
                            GeminiDirective::SyncPeerRelay(url) => {
                                PlannedProposalAction::SyncPeerRelay(url.clone())
                            }
                            GeminiDirective::RunBbsOptimize(len_m) => {
                                PlannedProposalAction::RunBbsOptimize(*len_m)
                            }
                            GeminiDirective::ToggleQaqc3d => PlannedProposalAction::ToggleQaqc3d,
                            GeminiDirective::ExportShopSvg => PlannedProposalAction::ExportShopSvg,
                            GeminiDirective::RunEigenFlow => PlannedProposalAction::RunEigenFlow,
                            GeminiDirective::OpenEigenFlowStudio => {
                                PlannedProposalAction::OpenEigenFlowStudio
                            }
                            GeminiDirective::RunPushover(pat) => {
                                PlannedProposalAction::RunPushover(*pat)
                            }
                            GeminiDirective::ToggleEigenFlow3d => {
                                PlannedProposalAction::ToggleEigenFlow3d
                            }
                            GeminiDirective::ExportEigenFlowAnnex => {
                                PlannedProposalAction::ExportEigenFlowAnnex
                            }
                            GeminiDirective::RunBlast { mass_kg, standoff_m } => {
                                PlannedProposalAction::RunBlast {
                                    mass_kg: *mass_kg,
                                    standoff_m: *standoff_m,
                                }
                            }
                            GeminiDirective::RunFire { minutes, story } => {
                                PlannedProposalAction::RunFire {
                                    minutes: *minutes,
                                    story: *story,
                                }
                            }
                            GeminiDirective::ToggleExtreme3d => {
                                PlannedProposalAction::ToggleExtreme3d
                            }
                            GeminiDirective::ExportExtremeAnnex => {
                                PlannedProposalAction::ExportExtremeAnnex
                            }
                            GeminiDirective::RunIsolation(iso) => {
                                PlannedProposalAction::RunIsolation(*iso)
                            }
                            GeminiDirective::ToggleIsolation3d => {
                                PlannedProposalAction::ToggleIsolation3d
                            }
                            GeminiDirective::ExportIsolationAnnex => {
                                PlannedProposalAction::ExportIsolationAnnex
                            }
                            GeminiDirective::RunWind {
                                speed_ms,
                                exposure,
                                azimuth_deg,
                            } => PlannedProposalAction::RunWind {
                                speed_ms: *speed_ms,
                                exposure: *exposure,
                                azimuth_deg: *azimuth_deg,
                            },
                            GeminiDirective::ToggleWind3d => PlannedProposalAction::ToggleWind3d,
                            GeminiDirective::ExportWindAnnex => {
                                PlannedProposalAction::ExportWindAnnex
                            }
                            GeminiDirective::RunShearWall {
                                tw_mm,
                                fc_mpa,
                                drift_ratio,
                            } => PlannedProposalAction::RunShearWall {
                                tw_mm: *tw_mm,
                                fc_mpa: *fc_mpa,
                                drift_ratio: *drift_ratio,
                            },
                            GeminiDirective::ToggleShearWall3d => {
                                PlannedProposalAction::ToggleShearWall3d
                            }
                            GeminiDirective::ExportShearWallAnnex => {
                                PlannedProposalAction::ExportShearWallAnnex
                            }
                            GeminiDirective::RunConnection {
                                conn_type,
                                bolt_dia_mm,
                                plate_thick_mm,
                            } => PlannedProposalAction::RunConnection {
                                conn_type: *conn_type,
                                bolt_dia_mm: *bolt_dia_mm,
                                plate_thick_mm: *plate_thick_mm,
                            },
                            GeminiDirective::ToggleConnection3d => {
                                PlannedProposalAction::ToggleConnection3d
                            }
                            GeminiDirective::ExportConnectionAnnex => {
                                PlannedProposalAction::ExportConnectionAnnex
                            }
                            GeminiDirective::RunPrestress {
                                profile,
                                num_strands,
                                fc_mpa,
                            } => PlannedProposalAction::RunPrestress {
                                profile: *profile,
                                num_strands: *num_strands,
                                fc_mpa: *fc_mpa,
                            },
                            GeminiDirective::TogglePrestress3d => {
                                PlannedProposalAction::TogglePrestress3d
                            }
                            GeminiDirective::ExportPrestressAnnex => {
                                PlannedProposalAction::ExportPrestressAnnex
                            }
                            GeminiDirective::RunComposite {
                                studs,
                                orientation,
                                unshored,
                            } => PlannedProposalAction::RunComposite {
                                studs: *studs,
                                orientation: *orientation,
                                unshored: *unshored,
                            },
                            GeminiDirective::ToggleComposite3d => {
                                PlannedProposalAction::ToggleComposite3d
                            }
                            GeminiDirective::ExportCompositeAnnex => {
                                PlannedProposalAction::ExportCompositeAnnex
                            }
                            GeminiDirective::RunReliability {
                                method,
                                target_beta,
                                limit_state,
                            } => PlannedProposalAction::RunReliability {
                                method: *method,
                                target_beta: *target_beta,
                                limit_state: *limit_state,
                            },
                            GeminiDirective::ToggleReliability3d => {
                                PlannedProposalAction::ToggleReliability3d
                            }
                            GeminiDirective::ExportReliabilityAnnex => {
                                PlannedProposalAction::ExportReliabilityAnnex
                            }
                            GeminiDirective::RunFoundation {
                                soil_type,
                                piles_x,
                                piles_y,
                                pile_length_m,
                            } => PlannedProposalAction::RunFoundation {
                                soil_type: *soil_type,
                                piles_x: *piles_x,
                                piles_y: *piles_y,
                                pile_length_m: *pile_length_m,
                            },
                            GeminiDirective::ToggleFoundation3d => {
                                PlannedProposalAction::ToggleFoundation3d
                            }
                            GeminiDirective::ExportFoundationAnnex => {
                                PlannedProposalAction::ExportFoundationAnnex
                            }
                            GeminiDirective::RunConstructionLca {
                                active_day,
                                steel_route,
                                concrete_mix,
                            } => PlannedProposalAction::RunConstructionLca {
                                active_day: *active_day,
                                steel_route: *steel_route,
                                concrete_mix: *concrete_mix,
                            },
                            GeminiDirective::ToggleConstruction3d => {
                                PlannedProposalAction::ToggleConstruction3d
                            }
                            GeminiDirective::ExportConstructionAnnex => {
                                PlannedProposalAction::ExportConstructionAnnex
                            }
                            GeminiDirective::InjectShmFault {
                                scenario,
                                severity_pct,
                                story,
                            } => PlannedProposalAction::InjectShmFault {
                                scenario: scenario.clone(),
                                severity_pct: *severity_pct,
                                story: *story,
                            },
                        })
                        .collect();
                    let preview: String = directives
                        .iter()
                        .map(|d| match d {
                            GeminiDirective::Fly { lat, lon, .. } => format!("Fly({lat:.2},{lon:.2})"),
                            GeminiDirective::Bim(c) => format!("BIM:{c}"),
                            GeminiDirective::Repair => "AutoRepair".to_string(),
                            GeminiDirective::EigenGateAudit => "EigenGate_Lens".to_string(),
                            GeminiDirective::ResizeColumn { id, w_mm, d_mm } => {
                                format!("ResizeCol#{id}({w_mm:.0}x{d_mm:.0})")
                            }
                            GeminiDirective::ResizeBeam { id, b_mm, h_mm } => {
                                format!("ResizeBeam#{id}({b_mm:.0}x{h_mm:.0})")
                            }
                            GeminiDirective::AddColumn { x_mm, y_mm, w_mm, d_mm } => {
                                format!("AddCol({x_mm:.0},{y_mm:.0},{w_mm:.0}x{d_mm:.0})")
                            }
                            GeminiDirective::ApplySiteSeismic => "SyncSiteSeismic".to_string(),
                            GeminiDirective::OpenSheet(pillar) => {
                                format!("OpenSheet({})", pillar.clean_name())
                            }
                            GeminiDirective::AddStory => "AddStory(+3.5m)".to_string(),
                            GeminiDirective::AddSlab => "AddSlab".to_string(),
                            GeminiDirective::BatchPinCity { query, city_name } => {
                                format!("BatchPin('{query}'->{city_name})")
                            }
                            GeminiDirective::PinActiveBuilding { lat, lon, heading_deg } => {
                                format!("PinSite({lat:.3},{lon:.3}@{heading_deg:.0}°)")
                            }
                            GeminiDirective::AuditNeighborhood => "AuditNeighborhood".to_string(),
                            GeminiDirective::EnableTerrainContours => "EnableTopoTIN".to_string(),
                            GeminiDirective::SwitchPureVector => "PureVectorEarth".to_string(),
                            GeminiDirective::ApplyDeformedLens => "LensDeformed".to_string(),
                            GeminiDirective::ApplyModalLens(m) => format!("LensModal(M{})", m + 1),
                            GeminiDirective::SimulateN1(fid) => format!("SimulateN1(#{fid})"),
                            GeminiDirective::RestoreN1 => "RestoreN1".to_string(),
                            GeminiDirective::RunEqReplay(_) => "AsyncEqReplay".to_string(),
                            GeminiDirective::TogglePlasticHinges => "ToggleHinges".to_string(),
                            GeminiDirective::ToggleTriDirectional(m) => {
                                format!(
                                    "TriDir({})",
                                    m.map(|v| v.short_label()).unwrap_or("Cycle")
                                )
                            }
                            GeminiDirective::ToggleAutoMatchSa => "MatchSaT1".to_string(),
                            GeminiDirective::OpenFragilityTab => "OpenFemaP58".to_string(),
                            GeminiDirective::OpenShmDigitalTwin => "OpenShmTwin".to_string(),
                            GeminiDirective::RunAtc20Triage => "Atc20Triage".to_string(),
                            GeminiDirective::RunEvacuationRouting => "RunEvacuation".to_string(),
                            GeminiDirective::OpenEvacuationDrawer => "OpenEvacDrawer".to_string(),
                            GeminiDirective::StartShmWatchdog => "StartShmWatchdog".to_string(),
                            GeminiDirective::ConnectShmWs(u) => format!("ConnectWs({u})"),
                            GeminiDirective::TriggerWatchdogSim(m) => format!("WatchdogSim({m})"),
                            GeminiDirective::RunGpuMonteCarlo(n) => format!("GpuMonteCarlo({n})"),
                            GeminiDirective::ToggleGpuParticles => "ToggleGpuParticles".to_string(),
                            GeminiDirective::ExportMcIfcx => "ExportMcIfcx".to_string(),
                            GeminiDirective::CreateCheckpoint(lbl) => {
                                format!("Checkpoint({})", if lbl.is_empty() { "Auto" } else { lbl })
                            }
                            GeminiDirective::ApproveCheckpoint => "ApproveCheckpoint".to_string(),
                            GeminiDirective::RunIfcxMerge => "ThreeWayMergeIfcx".to_string(),
                            GeminiDirective::ToggleRiskHeatmap(m) => format!(
                                "RiskHeatmap({})",
                                m.map(|v| v.short_label()).unwrap_or("Toggle")
                            ),
                            GeminiDirective::RunTradeStudy(obj) => format!(
                                "TradeStudy({})",
                                obj.map(|v| v.short_label()).unwrap_or("Balanced")
                            ),
                            GeminiDirective::ExportPermitAnnex => "ExportPermitAnnex(S-102)".to_string(),
                            GeminiDirective::SyncPeerRelay(url) => format!(
                                "SyncPeerRelay({})",
                                url.as_deref().unwrap_or("Active")
                            ),
                            GeminiDirective::RunBbsOptimize(len_m) => {
                                format!("RunBbsOptimize({len_m:.0}m)")
                            }
                            GeminiDirective::ToggleQaqc3d => "ToggleQaqc3D".to_string(),
                            GeminiDirective::ExportShopSvg => "ExportShopSvg(S-103)".to_string(),
                            GeminiDirective::RunEigenFlow => "RunEigenFlow(Woodbury)".to_string(),
                            GeminiDirective::OpenEigenFlowStudio => "OpenEigenFlowStudio".to_string(),
                            GeminiDirective::RunPushover(pat) => {
                                format!("RunPushover({})", pat.label())
                            }
                            GeminiDirective::ToggleEigenFlow3d => "ToggleEigenFlow3d".to_string(),
                            GeminiDirective::ExportEigenFlowAnnex => {
                                "ExportEigenFlowAnnex(S-104)".to_string()
                            }
                            GeminiDirective::RunBlast { mass_kg, standoff_m } => {
                                format!("RunBlast({mass_kg:.0}kg@{standoff_m:.1}m)")
                            }
                            GeminiDirective::RunFire { minutes, .. } => {
                                format!("RunFire({minutes:.0}m)")
                            }
                            GeminiDirective::ToggleExtreme3d => "ToggleExtreme3d".to_string(),
                            GeminiDirective::ExportExtremeAnnex => {
                                "ExportExtremeAnnex(S-105)".to_string()
                            }
                            GeminiDirective::RunIsolation(iso) => {
                                format!("RunIsolation({iso:?})")
                            }
                            GeminiDirective::ToggleIsolation3d => "ToggleIsolation3d".to_string(),
                            GeminiDirective::ExportIsolationAnnex => {
                                "ExportIsolationAnnex(S-106)".to_string()
                            }
                            GeminiDirective::RunWind {
                                speed_ms,
                                exposure,
                                azimuth_deg,
                            } => format!("RunWind({speed_ms:?}/{exposure:?}/{azimuth_deg:?})"),
                            GeminiDirective::ToggleWind3d => "ToggleWind3d".to_string(),
                            GeminiDirective::ExportWindAnnex => {
                                "ExportWindAnnex(S-107)".to_string()
                            }
                            GeminiDirective::RunShearWall {
                                tw_mm,
                                fc_mpa,
                                drift_ratio,
                            } => format!("RunShearWall({tw_mm:?}/{fc_mpa:?}/{drift_ratio:?})"),
                            GeminiDirective::ToggleShearWall3d => "ToggleShearWall3d".to_string(),
                            GeminiDirective::ExportShearWallAnnex => {
                                "ExportShearWallAnnex(S-108)".to_string()
                            }
                            GeminiDirective::RunConnection {
                                conn_type,
                                bolt_dia_mm,
                                plate_thick_mm,
                            } => format!("RunConnection({conn_type:?}/{bolt_dia_mm:?}/{plate_thick_mm:?})"),
                            GeminiDirective::ToggleConnection3d => {
                                "ToggleConnection3d".to_string()
                            }
                            GeminiDirective::ExportConnectionAnnex => {
                                "ExportConnectionAnnex(S-109)".to_string()
                            }
                            GeminiDirective::RunPrestress {
                                profile,
                                num_strands,
                                fc_mpa,
                            } => format!("RunPrestress({profile:?}/{num_strands:?}/{fc_mpa:?})"),
                            GeminiDirective::TogglePrestress3d => "TogglePrestress3d".to_string(),
                            GeminiDirective::ExportPrestressAnnex => {
                                "ExportPrestressAnnex(S-110)".to_string()
                            }
                            GeminiDirective::RunComposite {
                                studs,
                                orientation,
                                unshored,
                            } => format!("RunComposite({studs:?}/{orientation:?}/{unshored:?})"),
                            GeminiDirective::ToggleComposite3d => "ToggleComposite3d".to_string(),
                            GeminiDirective::ExportCompositeAnnex => {
                                "ExportCompositeAnnex(S-111)".to_string()
                            }
                            GeminiDirective::RunReliability {
                                method,
                                target_beta,
                                limit_state,
                            } => format!("RunReliability({method:?}/{target_beta:?}/{limit_state:?})"),
                            GeminiDirective::ToggleReliability3d => {
                                "ToggleReliability3d".to_string()
                            }
                            GeminiDirective::ExportReliabilityAnnex => {
                                "ExportReliabilityAnnex(S-112)".to_string()
                            }
                            GeminiDirective::RunFoundation {
                                soil_type,
                                piles_x,
                                piles_y,
                                pile_length_m,
                            } => format!("RunFoundation({soil_type:?}/{piles_x:?}x{piles_y:?}/{pile_length_m:?}m)"),
                            GeminiDirective::ToggleFoundation3d => {
                                "ToggleFoundation3d".to_string()
                            }
                            GeminiDirective::ExportFoundationAnnex => {
                                "ExportFoundationAnnex(S-113)".to_string()
                            }
                            GeminiDirective::RunConstructionLca {
                                active_day,
                                steel_route,
                                concrete_mix,
                            } => format!("RunConstructionLca(D{active_day:?}/{steel_route:?}/{concrete_mix:?})"),
                            GeminiDirective::ToggleConstruction3d => {
                                "ToggleConstruction3d".to_string()
                            }
                            GeminiDirective::ExportConstructionAnnex => {
                                "ExportConstructionAnnex(S-114)".to_string()
                            }
                            GeminiDirective::InjectShmFault {
                                scenario,
                                severity_pct,
                                story,
                            } => format!("ShmFault({scenario}:{severity_pct:.0}%@L{story})"),
                        })
                        .collect::<Vec<_>>()
                        .join(" + ");
                    if self.agent_mode == AgentExecutionMode::Planning {
                        self.queue_planning_proposal(
                            "Gemini Structured Action Plan",
                            "Review Gemini-proposed BIM / Earth changes before mutating the workspace.",
                            preview,
                            mapped,
                        );
                    } else {
                        let sub = self.apply_proposal_actions(&mapped, universal_canvas, build_view, ai_assistant);
                        out.merge(sub);
                    }
                }
                self.push_trace(format!("Gemini {} ready", reply.model), true);
                self.sessions_dirty = true;
            } else {
                let err = reply.error.unwrap_or_else(|| "Unknown Gemini API error".to_string());
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: false,
                    message: format!(
                        "[Gemini API Notice]: {err}\nClick the Key icon in Dynamic Island to verify your Gemini API Key or switch model to EigenAI Pro (offline FEM engine)."
                    ),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                self.push_trace("Gemini API request failed (check Key)", false);
                self.sessions_dirty = true;
            }
        }
        out
    }

    /// Formats all attached context pills into prompt metadata, trace step labels, and UI chip strings.
    pub fn build_enriched_context_payload(
        &self,
        project: &BimProject,
        members: &[StructuralMember],
        universal_canvas: &UniversalCanvasState,
    ) -> (Vec<String>, Vec<String>, String) {
        let mut chip_labels = Vec::new();
        let mut step_traces = Vec::new();
        let mut sections = Vec::new();

        for att in &self.attached_contexts {
            chip_labels.push(att.chip_label(project, self.selected_target));
            match att {
                AiContextAttachment::SelectedElement => {
                    if let Some(summary) =
                        self.format_selected_context_summary(project, members, universal_canvas)
                    {
                        step_traces.push("Read @Selected Element".to_string());
                        sections.push(format!("[Selected Element]: {summary}"));
                    }
                }
                AiContextAttachment::EarthSite => {
                    let (lat, lon) = match self.selected_target {
                        Some(SmartSelectionTarget::EarthSite { lat, lon }) => (lat, lon),
                        _ => universal_canvas.world_map.center_lat_lon,
                    };
                    let site = compute_site_environmental_coupling(
                        lat,
                        lon,
                        &universal_canvas.earth_world,
                    );
                    step_traces.push("Sampled Earth Site & Hazards".to_string());
                    sections.push(format!(
                        "[Earth Site ({lat:.3}°, {lon:.3}°) @ zoom {:.1}]: Elev {:.0}m, PGA Ss={:.2}g, Wind {:.0}m/s, Temp {:+.1}°C, CO2 {:.0} ppm",
                        universal_canvas.world_map.zoom_level,
                        site.elevation_m,
                        site.seismic_pga_ss_g,
                        site.design_wind_ms,
                        universal_canvas.earth_world.vitals.temp_anomaly_c,
                        universal_canvas.earth_world.vitals.co2_ppm,
                    ));
                }
                AiContextAttachment::WorldPortfolio => {
                    let sum = universal_canvas.world_index.portfolio_summary();
                    let reg_str = sum
                        .top_regions
                        .iter()
                        .map(|(r, c)| format!("{r} ({c})"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let crit_str = sum
                        .top_critical
                        .iter()
                        .map(|b| {
                            format!(
                                "{} @ ({:.3}°,{:.3}°) DCR={:.0}% EigenGate={:.2}",
                                b.name,
                                b.lat,
                                b.lon,
                                b.max_dcr * 100.0,
                                b.min_eigengate
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ");
                    step_traces.push("Audited Global World Portfolio".to_string());
                    sections.push(format!(
                        "[Planetary World Portfolio]: Total={}, GPS-Verified={}, City-Hint={}, User-Pinned={}, Unlocated Staging={}, Critical Risk={}. Regions: [{}]. Top Critical: [{}]",
                        sum.total_buildings,
                        sum.gps_verified_count,
                        sum.city_hint_count,
                        sum.user_pinned_count,
                        sum.unlocated_count,
                        sum.critical_count,
                        reg_str,
                        crit_str
                    ));
                }
                AiContextAttachment::BimGeometry => {
                    step_traces.push("Scanned BIM Model Topology".to_string());
                    sections.push(format!(
                        "[BIM Topology '{}']: {} Columns, {} Beams/Runs, {} Walls, {} Slabs, {} Footings, {} Levels. Sample/Top Elements: {}",
                        project.project_name,
                        project.columns.len(),
                        project.runs.len(),
                        project.walls.len(),
                        project.slabs.len(),
                        project.footings.len(),
                        project.levels.len(),
                        Self::format_critical_bim_context(project, members),
                    ));
                }
                AiContextAttachment::FemRedundancy => {
                    let crit = members.iter().filter(|m| m.is_critical || m.eigengate < 0.70).count();
                    let min_eigengate = members
                        .iter()
                        .map(|m| m.eigengate)
                        .fold(1.0_f64, f64::min);
                    let max_dcr = members
                        .iter()
                        .map(|m| m.utilization * 100.0)
                        .fold(0.0_f64, f64::max);
                    let fc = &universal_canvas.earth_fem_cache;
                    let modal_tag = fc
                        .modal_modes
                        .first()
                        .map(|m| format!("T1={:.2}s ({:.2}Hz)", m.period_s, m.frequency_hz))
                        .unwrap_or_else(|| "T1=n/a".to_string());
                    let n1_tag = fc
                        .n1_simulation
                        .as_ref()
                        .map(|s| {
                            format!(
                                " | Active N-1 Removed '{}': {}, Post Max DCR={:.0}%, Post Δ={:.1}mm, Res EigenGate={:.2}",
                                s.removed_name,
                                if s.report.is_kinematic_mechanism { "MECHANISM" } else { "Stable" },
                                s.report.post_max_dcr * 100.0,
                                s.report.post_max_disp_mm,
                                s.report.residual_eigengate
                            )
                        })
                        .unwrap_or_default();
                    step_traces.push("Inspected FEM & EigenGate 6-DOF + Modal Redundancy".to_string());
                    sections.push(format!(
                        "[FEM & EigenGate 6-DOF Matrix]: {} members, Max DCR={:.1}%, Min EigenGate={:.3}, Critical (<0.70)={crit}, Δmax={:.1}mm, Drift={:.2}%, V_seismic={:.0}kN, {modal_tag}{n1_tag}. Bottlenecks: {}",
                        members.len(),
                        max_dcr,
                        min_eigengate,
                        fc.max_disp_mm,
                        fc.max_story_drift_ratio * 100.0,
                        fc.seismic_base_shear_kn,
                        Self::format_critical_bim_context(project, members),
                    ));
                }
                AiContextAttachment::ScheduleAndCost => {
                    let total_vol_m3: f64 = project
                        .columns
                        .iter()
                        .map(|c| (c.width_mm / 1000.0) * (c.depth_mm / 1000.0) * 3.5)
                        .sum::<f64>()
                        + project
                            .runs
                            .iter()
                            .map(|r| {
                                (r.width_mm / 1000.0)
                                    * (r.depth_mm / 1000.0)
                                    * (r.length_mm(0.0) / 1000.0)
                            })
                            .sum::<f64>();
                    let est_idr_m = total_vol_m3 * 3.85 + 240.0;
                    step_traces.push("Compiled 4D/5D QTO Metrics".to_string());
                    sections.push(format!(
                        "[4D/5D Summary]: Structural concrete volume ≈ {:.1} m³, Indicative RC budget ≈ Rp {:.1}M IDR",
                        total_vol_m3, est_idr_m
                    ));
                }
                AiContextAttachment::VaultWorkspace => {
                    let status = crate::core::InternalVault::connection_status();
                    let projs = crate::core::InternalVault::list_personal_vault_projects();
                    step_traces.push(format!("Read Vault '{}'", status.vault_name));
                    sections.push(format!(
                        "[Connected Vault '{}']: backend={:?}, projects={}, location={}",
                        status.vault_name, status.backend, projs.len(), status.location_label
                    ));
                }
                AiContextAttachment::VaultFile {
                    name,
                    rel_path,
                    size_bytes,
                    ..
                } => {
                    let snippet =
                        crate::core::InternalVault::read_vault_context_snippet(rel_path, 900)
                            .unwrap_or_default();
                    step_traces.push(format!("Read Vault file '{name}'"));
                    sections.push(format!(
                        "[Vault File '{rel_path}' ({size_bytes} B)]:\n{snippet}"
                    ));
                }
                AiContextAttachment::LocalFile {
                    filename,
                    size_bytes,
                    snippet,
                } => {
                    step_traces.push(format!("Read local file '{filename}'"));
                    sections.push(format!(
                        "[Attached File '{filename}' ({size_bytes} B)]:\n{snippet}"
                    ));
                }
            }
        }

        (chip_labels, step_traces, sections.join("\n"))
    }

    pub fn apply_lens_to_views(
        &mut self,
        lens: SpatialLens,
        build_view: &mut BuildView,
        universal_canvas: &mut UniversalCanvasState,
    ) {
        use crate::views::universal_canvas::EarthBimLens;
        self.active_lens = lens;
        match lens {
            SpatialLens::Clean => {
                build_view.render_mode = Render3DMode::CsgSolidBrep;
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = false;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::Clean;
            }
            SpatialLens::DcrHeatmap => {
                build_view.render_mode = Render3DMode::SolidShaded;
                build_view.color_mode = "DCR%".to_string();
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = true;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::DcrHeatmap;
            }
            SpatialLens::EigenGateRedundancy => {
                build_view.render_mode = Render3DMode::SolidShaded;
                build_view.color_mode = "EigenGate".to_string();
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = true;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::EigenGateRedundancy;
            }
            SpatialLens::AxialForce => {
                build_view.render_mode = Render3DMode::AxialForceNFD;
                build_view.highlight_4d_erection = false;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::MomentStress;
            }
            SpatialLens::ShearForce => {
                build_view.render_mode = Render3DMode::ShearForceSFD;
                build_view.highlight_4d_erection = false;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::MomentStress;
            }
            SpatialLens::BendingMoment => {
                build_view.render_mode = Render3DMode::BendingMomentBMD;
                build_view.highlight_4d_erection = false;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::MomentStress;
            }
            SpatialLens::DeformedShape => {
                build_view.render_mode = Render3DMode::SolidShaded;
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = true;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::DeformedShape;
            }
            SpatialLens::ModalVibration => {
                build_view.render_mode = Render3DMode::SolidShaded;
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = true;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::ModalVibration;
                universal_canvas.earth_bim_author.modal_anim_playing = true;
            }
            SpatialLens::TimeHistoryReplay => {
                build_view.render_mode = Render3DMode::SolidShaded;
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = true;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::TimeHistoryReplay;
            }
            SpatialLens::ShmDigitalTwin => {
                build_view.render_mode = Render3DMode::SolidShaded;
                build_view.highlight_4d_erection = false;
                universal_canvas.show_domain_overlay = true;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::ShmDigitalTwin;
            }
            SpatialLens::Erection4D => {
                build_view.render_mode = Render3DMode::CsgSolidBrep;
                build_view.highlight_4d_erection = true;
                build_view.simulation_4d_day = self.active_4d_day as usize;
                universal_canvas.show_domain_overlay = true;
                universal_canvas.earth_bim_author.active_lens = EarthBimLens::Erection4D;
            }
        }
    }

    /// Executes any `/` slash command, `@Element` directive, or natural-language AI query.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_spatial_input(
        &mut self,
        raw_input: &str,
        universal_canvas: &mut UniversalCanvasState,
        build_view: &mut BuildView,
        ai_assistant: &mut AiAssistantView,
        members: &[StructuralMember],
        projects: &[ProjectItem],
        db_components: &mut Vec<DatabaseComponent>,
    ) -> SpatialHudActions {
        let mut actions = SpatialHudActions::default();
        let trimmed = raw_input.trim();
        if trimmed.is_empty() {
            return actions;
        }
        let lower = trimmed.to_lowercase();

        // 1. Slash Command Dispatch
        if lower.starts_with('/') {
            let mut parts = trimmed.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap_or("").to_lowercase();
            let arg = parts.next().unwrap_or("").trim();

            match cmd.as_str() {
                "" | "/" => {
                    self.command_input = "/".to_string();
                    self.request_focus_command = true;
                }
                "/earth" | "/globe" => {
                    self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                    universal_canvas.globe_mode = true;
                    universal_canvas.active_tool = UniversalCanvasTool::SelectPan;
                    universal_canvas.earth_bim_author.active_tool =
                        crate::views::universal_canvas::EarthBimTool::None;
                    self.push_trace("Focused 3D Earth Globe", true);
                }
                "/iso" | "/3d" | "/bim" => {
                    self.apply_camera_preset(build_view, universal_canvas, "iso");
                    actions.fit_camera = true;
                }
                "/top" | "/2d" | "/plan" => {
                    self.apply_camera_preset(build_view, universal_canvas, "top");
                    actions.fit_camera = true;
                }
                "/fit" | "/zoom" => {
                    actions.fit_camera = true;
                    self.push_trace("Fitted Earth 3D camera to active model", true);
                }
                "/classic" | "/sidebar" => {
                    actions.switch_to_classic_ui = true;
                    self.push_trace("Switched to Classic 15-Pillar UI", true);
                }
                "/portal" | "/frame" | "/box" | "/grid" | "/col" | "/rebar" | "/clear" => {
                    let nl_cmd = trimmed.trim_start_matches('/');
                    match build_view.bim_bus.execute_natural_language(nl_cmd) {
                        Ok(msg) => {
                            if nl_cmd == "clear" {
                                self.selected_target = None;
                                universal_canvas.highlighted_bim_element = None;
                            } else {
                                self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                                universal_canvas.globe_mode = true;
                                universal_canvas.ground_origin_anchored = true;
                            }
                            actions.sync_fem_and_earth = true;
                            actions.fit_camera = true;
                            self.push_trace(msg.clone(), true);
                            ai_assistant.chat_history.push(crate::models::ChatMessage {
                                is_user: false,
                                message: format!("[Spatial Command]: {msg}"),
                                time: crate::core::web_bridge::unix_timestamp_label(),
                            });
                        }
                        Err(err) => {
                            self.push_trace(err.clone(), false);
                        }
                    }
                }
                "/column" => {
                    let _ = SpatialManualTool::Column.apply(self, universal_canvas, build_view);
                }
                "/beam" => {
                    let _ = SpatialManualTool::Beam.apply(self, universal_canvas, build_view);
                }
                "/wall" => {
                    let _ = SpatialManualTool::Wall.apply(self, universal_canvas, build_view);
                }
                "/footing" => {
                    let _ = SpatialManualTool::Footing.apply(self, universal_canvas, build_view);
                }
                "/line" => {
                    let _ = SpatialManualTool::Line2D.apply(self, universal_canvas, build_view);
                }
                "/measure" => {
                    universal_canvas.measure_active = !universal_canvas.measure_active;
                    if !universal_canvas.measure_active {
                        universal_canvas.measure_points_lat_lon.clear();
                    }
                    self.push_trace(
                        if universal_canvas.measure_active {
                            "Geodesic Earth Measure ON"
                        } else {
                            "Geodesic Earth Measure OFF"
                        },
                        true,
                    );
                }
                "/lens" => {
                    let arg_l = arg.to_lowercase();
                    let next_lens = if arg_l.contains("dcr") || arg_l.contains("util") {
                        SpatialLens::DcrHeatmap
                    } else if arg_l.contains("eigengate") || arg_l.contains("ari") || arg_l.contains("red") {
                        SpatialLens::EigenGateRedundancy
                    } else if arg_l.contains("mom") || arg_l.contains("bmd") {
                        SpatialLens::BendingMoment
                    } else if arg_l.contains("shear") || arg_l.contains("sfd") {
                        SpatialLens::ShearForce
                    } else if arg_l.contains("ax") || arg_l.contains("nfd") {
                        SpatialLens::AxialForce
                    } else if arg_l.contains("def") || arg_l.contains("drift") || arg_l.contains("sway") {
                        SpatialLens::DeformedShape
                    } else if arg_l.contains("mod") || arg_l.contains("eig") || arg_l.contains("vib") {
                        SpatialLens::ModalVibration
                    } else if arg_l.contains("eq") || arg_l.contains("rep") || arg_l.contains("tha") {
                        SpatialLens::TimeHistoryReplay
                    } else if arg_l.contains("shm")
                        || arg_l.contains("atc")
                        || arg_l.contains("placard")
                        || arg_l.contains("sensor")
                    {
                        SpatialLens::ShmDigitalTwin
                    } else if arg_l.contains("4d") || arg_l.contains("time") {
                        SpatialLens::Erection4D
                    } else {
                        SpatialLens::Clean
                    };
                    self.apply_lens_to_views(next_lens, build_view, universal_canvas);
                    self.push_trace(format!("Earth 3D Lens: {}", next_lens.label()), true);
                }
                "/deform" | "/deflection" | "/drift" => {
                    if let Ok(scale) = arg.trim_end_matches('x').trim().parse::<f64>() {
                        if scale.is_finite() {
                            universal_canvas.earth_bim_author.deform_scale = scale.clamp(1.0, 150.0);
                        }
                    }
                    self.apply_lens_to_views(SpatialLens::DeformedShape, build_view, universal_canvas);
                    self.push_trace(
                        format!(
                            "Earth 3D 6-DOF Deformed Shape ({:.0}x, Δmax={:.1}mm)",
                            universal_canvas.earth_bim_author.deform_scale,
                            universal_canvas.earth_fem_cache.max_disp_mm
                        ),
                        true,
                    );
                }
                "/modal" | "/eigen" | "/vibrate" => {
                    if let Ok(m_num) = arg.trim().parse::<usize>() {
                        universal_canvas.earth_bim_author.active_modal_mode =
                            m_num.saturating_sub(1).min(2);
                    }
                    self.apply_lens_to_views(SpatialLens::ModalVibration, build_view, universal_canvas);
                    let m_idx = universal_canvas.earth_bim_author.active_modal_mode;
                    let period_str = universal_canvas
                        .earth_fem_cache
                        .modal_modes
                        .get(m_idx)
                        .map(|m| format!("T={:.2}s f={:.2}Hz", m.period_s, m.frequency_hz))
                        .unwrap_or_else(|| "animating".to_string());
                    self.push_trace(
                        format!("Earth 3D Modal Mode {} ({period_str})", m_idx + 1),
                        true,
                    );
                }
                "/n1" | "/disrupt" => {
                    let target_fid = arg
                        .trim()
                        .parse::<usize>()
                        .ok()
                        .or_else(|| match self.selected_target {
                            Some(SmartSelectionTarget::Column(id)) => {
                                universal_canvas.earth_fem_cache.fem_id(BimElementKind::Column, id)
                            }
                            Some(SmartSelectionTarget::Beam(id)) => {
                                universal_canvas.earth_fem_cache.fem_id(BimElementKind::Run, id)
                            }
                            Some(SmartSelectionTarget::Wall(id)) => {
                                universal_canvas.earth_fem_cache.fem_id(BimElementKind::Wall, id)
                            }
                            _ => None,
                        })
                        .or_else(|| {
                            members
                                .iter()
                                .max_by(|a, b| {
                                    let sa = a.utilization + (1.0 - a.eigengate).max(0.0);
                                    let sb = b.utilization + (1.0 - b.eigengate).max(0.0);
                                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                                })
                                .map(|m| m.id)
                        });
                    if let Some(fid) = target_fid {
                        actions.toggle_n1_member = Some(fid);
                    } else {
                        self.push_trace("No FEM structural member available for N-1 disruption", false);
                    }
                }
                "/restore" | "/intact" => {
                    actions.restore_n1_simulation = true;
                }
                "/day" => {
                    let lon = universal_canvas.world_map.center_lat_lon.1;
                    universal_canvas.earth_world.camera.set_local_sun_hour(13.5, lon);
                    self.push_trace("Earth Vector lighting set to Daylight (13:30)", true);
                }
                "/dusk" | "/sunset" => {
                    let lon = universal_canvas.world_map.center_lat_lon.1;
                    universal_canvas.earth_world.camera.set_local_sun_hour(17.4, lon);
                    self.push_trace("Earth Vector lighting set to Golden Hour Dusk (17:24)", true);
                }
                "/night" => {
                    let lon = universal_canvas.world_map.center_lat_lon.1;
                    universal_canvas.earth_world.camera.set_local_sun_hour(22.0, lon);
                    self.push_trace("Earth Vector lighting set to Night (22:00, procedural windows)", true);
                }
                "/gpu" => {
                    universal_canvas.webgpu_earth.enabled = !universal_canvas.webgpu_earth.enabled;
                    let mode = if universal_canvas.webgpu_earth.is_active() {
                        "WebGPU (WGSL) 3D active"
                    } else if universal_canvas.webgpu_earth.enabled {
                        "WebGPU enabled (CPU fallback in headless context)"
                    } else {
                        "CPU Pure-Vector mode"
                    };
                    self.push_trace(format!("Earth Vector renderer: {mode}"), true);
                }
                "/ifcx" => {
                    actions.export_ifcx = true;
                }
                "/earth-ifcx" | "/earth_ifcx" => {
                    actions.export_earth_ifcx = true;
                }
                "/road" => {
                    build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                    universal_canvas.earth_bim_author.active_tool =
                        crate::views::universal_canvas::EarthBimTool::Road;
                    universal_canvas.earth_bim_author.pending_start_mm = None;
                    universal_canvas.is_frozen = false;
                    self.push_trace(
                        format!(
                            "City-Builder IfcRoad active ({}) — click start & end on Earth 3D",
                            universal_canvas.city_scene.active_road_kind.label()
                        ),
                        true,
                    );
                }
                "/bridge" => {
                    build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                    universal_canvas.earth_bim_author.active_tool =
                        crate::views::universal_canvas::EarthBimTool::Bridge;
                    universal_canvas.earth_bim_author.pending_start_mm = None;
                    universal_canvas.is_frozen = false;
                    self.push_trace(
                        "City-Builder IfcBridge active (6.5m pier/deck) — click start & end on Earth 3D",
                        true,
                    );
                }
                "/zone" => {
                    build_view.ground_author_tool = Ground3DAuthorTool::Orbit;
                    universal_canvas.earth_bim_author.active_tool =
                        crate::views::universal_canvas::EarthBimTool::Zone;
                    universal_canvas.earth_bim_author.pending_start_mm = None;
                    universal_canvas.is_frozen = false;
                    self.push_trace(
                        format!(
                            "City-Builder IfcSpatialZone active ({}) — click on Earth 3D to zone parcel",
                            universal_canvas.city_scene.active_zone_kind.label()
                        ),
                        true,
                    );
                }
                "/gradepad" | "/grade" => {
                    let tin = universal_canvas.active_site_tin.get_or_insert_with(|| {
                        crate::core::gis::TinSurfaceMesh::generate_from_dem_preset(
                            universal_canvas.site_tin_preset,
                            400.0,
                            400.0,
                            36,
                            36,
                            1.0,
                        )
                    });
                    let pad = universal_canvas
                        .city_scene
                        .auto_grade_building_pad(&build_view.bim_bus.project, tin);
                    universal_canvas.show_terrain_contours = true;
                    self.push_trace(
                        format!(
                            "Graded {} ({:.0}x{:.0}m @ {:.1}m): Cut {:.0} m3 | Fill {:.0} m3",
                            pad.name,
                            pad.width_m,
                            pad.depth_m,
                            pad.target_elev_m,
                            pad.cut_volume_m3,
                            pad.fill_volume_m3
                        ),
                        true,
                    );
                }
                "/seismic" | "/wave" => {
                    let next = !universal_canvas.district_hazard.seismic_wave_active;
                    universal_canvas.district_hazard.seismic_wave_active = next;
                    if next {
                        universal_canvas.district_hazard.pounding_overlay_active = true;
                        universal_canvas.show_neighborhood_hazards = true;
                        let c = universal_canvas.active_site_coupling();
                        universal_canvas.district_hazard.seismic_pga_g =
                            f64::from(c.seismic_pga_ss_g).clamp(0.15, 1.5);
                    }
                    actions.run_district_hazard_audit = true;
                    self.push_trace(
                        format!(
                            "District Seismic Wave + Pounding: {}",
                            if next { "ON" } else { "OFF" }
                        ),
                        true,
                    );
                }
                "/wind" | "/venturi" => {
                    let next = !universal_canvas.district_hazard.venturi_wind_active;
                    universal_canvas.district_hazard.venturi_wind_active = next;
                    if next {
                        let c = universal_canvas.active_site_coupling();
                        universal_canvas.district_hazard.base_wind_ms =
                            f64::from(c.design_wind_ms).clamp(18.0, 65.0);
                    }
                    actions.run_district_hazard_audit = true;
                    self.push_trace(
                        format!(
                            "Venturi Urban Wind Streamlines: {}",
                            if next { "ON" } else { "OFF" }
                        ),
                        true,
                    );
                }
                "/flood" => {
                    let wl_opt = arg
                        .trim_end_matches('m')
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite());
                    if let Some(wl) = wl_opt {
                        universal_canvas.district_hazard.flood_water_level_m =
                            wl.clamp(-5.0, 25.0);
                        universal_canvas.district_hazard.flood_plane_active = true;
                    } else {
                        universal_canvas.district_hazard.flood_plane_active =
                            !universal_canvas.district_hazard.flood_plane_active;
                    }
                    if universal_canvas.district_hazard.flood_plane_active {
                        universal_canvas.ensure_site_tin_generated();
                        universal_canvas.show_terrain_contours = true;
                    }
                    actions.run_district_hazard_audit = true;
                    self.push_trace(
                        format!(
                            "3D Flood Plane ({:+.1}m MSL): {}",
                            universal_canvas.district_hazard.flood_water_level_m,
                            if universal_canvas.district_hazard.flood_plane_active {
                                "ON"
                            } else {
                                "OFF"
                            }
                        ),
                        true,
                    );
                }
                "/hazard" | "/multihazard" => {
                    universal_canvas.ensure_site_tin_generated();
                    universal_canvas.show_terrain_contours = true;
                    universal_canvas.show_neighborhood_hazards = true;
                    universal_canvas.district_hazard.seismic_wave_active = true;
                    universal_canvas.district_hazard.pounding_overlay_active = true;
                    universal_canvas.district_hazard.venturi_wind_active = true;
                    universal_canvas.district_hazard.flood_plane_active = true;
                    actions.run_district_hazard_audit = true;
                }
                "/ai-capture" | "/aicapture" | "/scan-ifcx" => {
                    actions.ingest_ai_real_building_capture = true;
                }
                "/retrofit" => {
                    let strat = if arg.trim().is_empty() {
                        universal_canvas.retrofit_strategy
                    } else {
                        crate::core::retrofit::RetrofitStrategy::from_arg(arg)
                    };
                    universal_canvas.retrofit_strategy = strat;
                    actions.run_structural_retrofit = Some(strat);
                }
                "/brb" | "/brace" => {
                    let strat = crate::core::retrofit::RetrofitStrategy::BrbBracingOnly;
                    universal_canvas.retrofit_strategy = strat;
                    actions.run_structural_retrofit = Some(strat);
                }
                "/jacket" | "/jacketing" => {
                    let strat = crate::core::retrofit::RetrofitStrategy::RcJacketingOnly;
                    universal_canvas.retrofit_strategy = strat;
                    actions.run_structural_retrofit = Some(strat);
                }
                "/retrofit-ifcx" | "/delta-ifcx" => {
                    actions.export_retrofit_delta_ifcx = true;
                }
                "/before-after" | "/compare-retrofit" => {
                    actions.toggle_retrofit_before_after = true;
                }
                "/cascade" | "/collapse" => {
                    let daf = arg
                        .trim()
                        .trim_end_matches('x')
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .unwrap_or(universal_canvas.cascade_daf)
                        .clamp(1.0, 2.5);
                    universal_canvas.cascade_daf = daf;
                    actions.run_crc_cascade_screening = Some(daf);
                }
                "/crc" | "/structgnn" => {
                    actions.run_crc_cascade_screening = Some(universal_canvas.cascade_daf);
                }
                "/cascade-ifcx" | "/crc-ifcx" => {
                    actions.export_crc_cascade_ifcx = true;
                }
                "/ssi" | "/winkler" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    if sub == "off" || sub == "fixed" {
                        universal_canvas.subsurface_ssi.ssi_coupling_enabled = false;
                    } else if sub == "on" {
                        universal_canvas.subsurface_ssi.ssi_coupling_enabled = true;
                        universal_canvas.subsurface_ssi.xray_subsurface_visible = true;
                    } else if !sub.is_empty() {
                        universal_canvas.subsurface_ssi.borehole_preset =
                            crate::core::geotech::BoreholePreset::from_arg(arg);
                        universal_canvas.subsurface_ssi.ssi_coupling_enabled = true;
                        universal_canvas.subsurface_ssi.xray_subsurface_visible = true;
                    } else {
                        let next = !universal_canvas.subsurface_ssi.ssi_coupling_enabled;
                        universal_canvas.subsurface_ssi.ssi_coupling_enabled = next;
                        if next {
                            universal_canvas.subsurface_ssi.xray_subsurface_visible = true;
                        }
                    }
                    actions.run_subsurface_ssi_sync = true;
                }
                "/borehole" | "/soil" | "/strata" => {
                    universal_canvas.subsurface_ssi.borehole_preset = if arg.trim().is_empty() {
                        universal_canvas.subsurface_ssi.borehole_preset.cycle_next()
                    } else {
                        crate::core::geotech::BoreholePreset::from_arg(arg)
                    };
                    universal_canvas.subsurface_ssi.xray_subsurface_visible = true;
                    universal_canvas.subsurface_ssi.ssi_coupling_enabled = true;
                    actions.run_subsurface_ssi_sync = true;
                }
                "/liquefaction" | "/liquefy" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    let next = if sub == "off" || sub == "false" {
                        false
                    } else if sub == "on" || sub == "true" {
                        true
                    } else {
                        !universal_canvas.subsurface_ssi.liquefaction_trigger_active
                    };
                    universal_canvas.subsurface_ssi.liquefaction_trigger_active = next;
                    if next {
                        universal_canvas.subsurface_ssi.xray_subsurface_visible = true;
                        universal_canvas.subsurface_ssi.ssi_coupling_enabled = true;
                    }
                    actions.run_subsurface_ssi_sync = true;
                }
                "/subsurface" | "/xray" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    universal_canvas.subsurface_ssi.xray_subsurface_visible =
                        if sub == "off" || sub == "false" {
                            false
                        } else if sub == "on" || sub == "true" {
                            true
                        } else {
                            !universal_canvas.subsurface_ssi.xray_subsurface_visible
                        };
                    actions.run_subsurface_ssi_sync = true;
                }
                "/ssi-ifcx" | "/geotech-ifcx" => {
                    actions.export_subsurface_ssi_ifcx = true;
                }
                "/snapshot" | "/ifcx-baseline" => {
                    actions.capture_ifcx_baseline = true;
                }
                "/diff" | "/ifcx-diff" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    if sub == "off" || sub == "close" {
                        self.ifcx_diff_drawer_open = false;
                        universal_canvas.ifcx_diff_overlay_active = false;
                    } else {
                        if !sub.is_empty() {
                            universal_canvas.ifcx_diff_filter =
                                crate::core::ifcx::IfcxDiffLayerFilter::from_arg(arg);
                        }
                        self.vault_drawer_open = false;
                        self.vault_path_popover_open = false;
                        self.context_picker_open = false;
                        self.history_popover_open = false;
                        self.api_key_popover_open = false;
                        self.portfolio_drawer_open = false;
                        self.ground_motion_drawer_open = false;
                        self.ifcx_diff_drawer_open = true;
                        universal_canvas.ifcx_diff_overlay_active = true;
                    }
                    actions.run_ifcx_visual_diff = true;
                }
                "/diff-ifcx" => {
                    actions.export_ifcx_diff_manifest = true;
                }
                "/dossier" | "/dossier-svg" | "/vector-dossier" => {
                    actions.export_vector_dossier_svg = true;
                }
                "/portfolio" | "/district-retrofit" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    if sub == "off" || sub == "close" {
                        self.portfolio_drawer_open = false;
                        universal_canvas.portfolio_retrofit_config.overlay_visible = false;
                        actions.run_portfolio_retrofit = true;
                    } else if sub == "fly" || sub == "next" || sub == "jump" || sub == "world" {
                        self.attach_context(AiContextAttachment::WorldPortfolio);
                        let sum = universal_canvas.world_index.portfolio_summary();
                        if sum.critical_count > 0 {
                            actions.fly_to_next_world_building =
                                Some(crate::core::gis::WorldPortfolioFilter::CriticalRisk);
                        } else if sum.unlocated_count > 0 {
                            actions.fly_to_next_world_building =
                                Some(crate::core::gis::WorldPortfolioFilter::UnlocatedStaging);
                        }
                        self.push_trace(
                            format!(
                                "World Portfolio fly-to: {} total ({} unlocated, {} critical)",
                                sum.total_buildings, sum.unlocated_count, sum.critical_count
                            ),
                            true,
                        );
                    } else {
                        if !sub.is_empty() {
                            universal_canvas.portfolio_retrofit_config.objective =
                                crate::core::portfolio_retrofit::PortfolioOptimizationObjective::from_arg(arg);
                        }
                        self.attach_context(AiContextAttachment::WorldPortfolio);
                        self.vault_drawer_open = false;
                        self.vault_path_popover_open = false;
                        self.context_picker_open = false;
                        self.history_popover_open = false;
                        self.api_key_popover_open = false;
                        self.ifcx_diff_drawer_open = false;
                        self.ground_motion_drawer_open = false;
                        self.portfolio_drawer_open = true;
                        universal_canvas.portfolio_retrofit_config.overlay_visible = true;
                        actions.run_portfolio_retrofit = true;
                    }
                }
                "/pareto" => {
                    self.vault_drawer_open = false;
                    self.vault_path_popover_open = false;
                    self.context_picker_open = false;
                    self.history_popover_open = false;
                    self.api_key_popover_open = false;
                    self.ifcx_diff_drawer_open = false;
                    self.portfolio_drawer_open = true;
                    universal_canvas.portfolio_retrofit_config.overlay_visible = true;
                    actions.run_portfolio_retrofit = true;
                }
                "/budget" => {
                    let raw = arg
                        .trim()
                        .trim_start_matches('$')
                        .trim()
                        .to_ascii_lowercase();
                    let mult = if raw.ends_with('m') {
                        1_000_000.0
                    } else if raw.ends_with('k') {
                        1_000.0
                    } else {
                        1.0
                    };
                    let num_str = raw.trim_end_matches(&['k', 'm'][..]).trim();
                    if let Ok(v) = num_str.parse::<f64>() {
                        if v.is_finite() && v > 0.0 {
                            let scaled = if mult == 1.0 && v < 5_000.0 {
                                v * 1_000.0
                            } else {
                                v * mult
                            };
                            universal_canvas.portfolio_retrofit_config.budget_cap_usd =
                                scaled.clamp(25_000.0, 50_000_000.0);
                        }
                    }
                    self.vault_drawer_open = false;
                    self.vault_path_popover_open = false;
                    self.context_picker_open = false;
                    self.history_popover_open = false;
                    self.api_key_popover_open = false;
                    self.ifcx_diff_drawer_open = false;
                    self.portfolio_drawer_open = true;
                    universal_canvas.portfolio_retrofit_config.overlay_visible = true;
                    actions.run_portfolio_retrofit = true;
                }
                "/portfolio-ifcx" => {
                    actions.export_portfolio_retrofit_ifcx = true;
                }
                "/replay" | "/eq" | "/tha" | "/time-history" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    if sub == "off" || sub == "close" {
                        self.ground_motion_drawer_open = false;
                        self.apply_lens_to_views(SpatialLens::Clean, build_view, universal_canvas);
                    } else {
                        if let Some(rec) = crate::core::ground_motion_replay::GroundMotionReplayConfig::record_from_arg(&sub) {
                            universal_canvas.ground_motion_replay_config.record = rec;
                        }
                        self.vault_drawer_open = false;
                        self.vault_path_popover_open = false;
                        self.context_picker_open = false;
                        self.history_popover_open = false;
                        self.api_key_popover_open = false;
                        self.ifcx_diff_drawer_open = false;
                        self.portfolio_drawer_open = false;
                        self.ground_motion_drawer_open = true;
                        universal_canvas.replay_anim_playing = true;
                        self.apply_lens_to_views(
                            SpatialLens::TimeHistoryReplay,
                            build_view,
                            universal_canvas,
                        );
                        actions.run_async_ground_motion_replay = true;
                    }
                }
                "/record" => {
                    let next_rec = if let Some(rec) =
                        crate::core::ground_motion_replay::GroundMotionReplayConfig::record_from_arg(arg)
                    {
                        rec
                    } else {
                        crate::core::ground_motion_replay::GroundMotionReplayConfig::next_record(
                            universal_canvas.ground_motion_replay_config.record,
                        )
                    };
                    universal_canvas.ground_motion_replay_config.record = next_rec;
                    self.vault_drawer_open = false;
                    self.vault_path_popover_open = false;
                    self.context_picker_open = false;
                    self.history_popover_open = false;
                    self.api_key_popover_open = false;
                    self.ifcx_diff_drawer_open = false;
                    self.portfolio_drawer_open = false;
                    self.ground_motion_drawer_open = true;
                    self.apply_lens_to_views(
                        SpatialLens::TimeHistoryReplay,
                        build_view,
                        universal_canvas,
                    );
                    actions.run_async_ground_motion_replay = true;
                    self.push_trace(
                        format!(
                            "Ground motion set to {} ({:.2}g) — running async...",
                            crate::core::ground_motion_replay::GroundMotionReplayConfig::record_short_label(next_rec),
                            next_rec.pga_g() * universal_canvas.ground_motion_replay_config.scale_factor
                        ),
                        true,
                    );
                }
                "/hinges" | "/plastic-hinges" => {
                    universal_canvas.ground_motion_replay_config.show_plastic_hinges =
                        !universal_canvas.ground_motion_replay_config.show_plastic_hinges;
                    self.apply_lens_to_views(
                        SpatialLens::TimeHistoryReplay,
                        build_view,
                        universal_canvas,
                    );
                    if let Some(ref rep) = universal_canvas.last_ground_motion_replay_report {
                        universal_canvas.replay_cursor_time_s = rep.peak_time_s;
                        universal_canvas.replay_anim_playing = false;
                    } else {
                        actions.run_async_ground_motion_replay = true;
                    }
                    self.push_trace(
                        format!(
                            "ASCE 41-17 3D Plastic Hinges {}",
                            if universal_canvas.ground_motion_replay_config.show_plastic_hinges {
                                "ON (jumped to peak drift)"
                            } else {
                                "OFF"
                            }
                        ),
                        true,
                    );
                }
                "/replay-ifcx" => {
                    actions.export_ground_motion_replay_ifcx = true;
                }
                "/at2" | "/accelerogram" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    if sub == "clear" || sub == "off" || sub == "reset" {
                        universal_canvas.ground_motion_replay_config.custom_accelerogram = None;
                        self.open_ground_motion_drawer();
                        self.apply_lens_to_views(
                            SpatialLens::TimeHistoryReplay,
                            build_view,
                            universal_canvas,
                        );
                        actions.run_async_ground_motion_replay = true;
                        self.push_trace(
                            "Cleared custom accelerogram; reverted to built-in benchmark suite",
                            true,
                        );
                    } else if sub == "sample" || sub == "demo" || sub == "peer" {
                        match crate::core::time_history::parse_custom_accelerogram(
                            crate::core::time_history::sample_peer_at2_text(),
                            "PEER_Sample_Northridge_Sylmar.at2",
                        ) {
                            Ok(acc) => {
                                let label = format!(
                                    "{} (PGA={:.2}g, dt={:.3}s)",
                                    acc.name, acc.pga_x_g, acc.dt_s
                                );
                                universal_canvas
                                    .ground_motion_replay_config
                                    .custom_accelerogram = Some(acc);
                                self.open_ground_motion_drawer();
                                self.apply_lens_to_views(
                                    SpatialLens::TimeHistoryReplay,
                                    build_view,
                                    universal_canvas,
                                );
                                actions.run_async_ground_motion_replay = true;
                                self.push_trace(format!("Loaded sample PEER .at2: {label}"), true);
                            }
                            Err(err) => self.push_trace(err, false),
                        }
                    } else {
                        self.open_ground_motion_drawer();
                        actions.open_accelerogram_file_picker = true;
                    }
                }
                "/tridir" | "/3axis" => {
                    use crate::core::ground_motion_replay::TriDirectionalMode;
                    let sub = arg.trim().to_ascii_lowercase();
                    let next = match sub.as_str() {
                        "x" | "1d" | "uniaxial" => TriDirectionalMode::UniaxialX,
                        "xy" | "2d" | "biaxial" => TriDirectionalMode::BiaxialXY,
                        "xyz" | "3d" | "tridirectional" => TriDirectionalMode::TriDirectionalXYZ,
                        _ => universal_canvas
                            .ground_motion_replay_config
                            .tridirectional_mode
                            .next(),
                    };
                    universal_canvas.ground_motion_replay_config.tridirectional_mode = next;
                    self.open_ground_motion_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::TimeHistoryReplay,
                        build_view,
                        universal_canvas,
                    );
                    actions.run_async_ground_motion_replay = true;
                    self.push_trace(
                        format!("Tri-Directional seismic coupling: {}", next.short_label()),
                        true,
                    );
                }
                "/match-sa" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    let next = match sub.as_str() {
                        "on" | "1" | "true" => true,
                        "off" | "0" | "false" => false,
                        _ => !universal_canvas.ground_motion_replay_config.auto_match_sa_t1,
                    };
                    universal_canvas.ground_motion_replay_config.auto_match_sa_t1 = next;
                    self.ground_motion_drawer_tab = 1;
                    self.open_ground_motion_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::TimeHistoryReplay,
                        build_view,
                        universal_canvas,
                    );
                    actions.run_async_ground_motion_replay = true;
                    self.push_trace(
                        format!(
                            "Auto Match SNI 1726 Sa(T1) spectrum scaling: {}",
                            if next { "ON" } else { "OFF" }
                        ),
                        true,
                    );
                }
                "/fragility" | "/p58" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    if sub == "off" || sub == "hide" {
                        universal_canvas
                            .ground_motion_replay_config
                            .show_fragility_loss_overlay = false;
                        self.push_trace("FEMA P-58 3D story-loss halo overlay OFF", true);
                    } else {
                        universal_canvas
                            .ground_motion_replay_config
                            .show_fragility_loss_overlay = true;
                        self.ground_motion_drawer_tab = 1;
                        self.open_ground_motion_drawer();
                        self.apply_lens_to_views(
                            SpatialLens::TimeHistoryReplay,
                            build_view,
                            universal_canvas,
                        );
                        if universal_canvas.last_ground_motion_replay_report.is_none() {
                            actions.run_async_ground_motion_replay = true;
                        }
                        self.push_trace(
                            "Opened FEMA P-58 Fragility & Spectrum tab with 3D story-loss halo",
                            true,
                        );
                    }
                }
                "/fragility-ifcx" => {
                    actions.export_fragility_loss_ifcx = true;
                }
                "/shm" => {
                    self.shm_drawer_tab = 0;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    actions.run_shm_digital_twin = true;
                }
                "/sensors" => {
                    let sub = arg.trim().to_ascii_lowercase();
                    if sub.contains("sample") {
                        let csv = crate::core::sample_shm_telemetry_csv();
                        if let Ok(log) = crate::core::parse_shm_telemetry_log(&csv, "sample_shm.csv") {
                            universal_canvas.shm_twin_config.custom_telemetry = Some(log);
                            universal_canvas.shm_twin_config.fault_scenario =
                                crate::core::ShmFaultScenario::CustomTelemetryStream;
                            self.shm_drawer_tab = 0;
                            self.open_shm_drawer();
                            self.apply_lens_to_views(
                                SpatialLens::ShmDigitalTwin,
                                build_view,
                                universal_canvas,
                            );
                            actions.run_shm_digital_twin = true;
                            self.push_trace("Loaded sample 3-story SHM sensor stream (CSV)", true);
                        }
                    } else if sub == "clear" || sub == "reset" {
                        universal_canvas.shm_twin_config.custom_telemetry = None;
                        universal_canvas.shm_twin_config.fault_scenario =
                            crate::core::ShmFaultScenario::NominalIntact;
                        self.shm_drawer_tab = 0;
                        self.open_shm_drawer();
                        self.apply_lens_to_views(
                            SpatialLens::ShmDigitalTwin,
                            build_view,
                            universal_canvas,
                        );
                        actions.run_shm_digital_twin = true;
                        self.push_trace("Cleared custom SHM telemetry stream; reset to Nominal Intact", true);
                    } else if sub.contains("upload") || sub.contains("import") {
                        self.shm_drawer_tab = 0;
                        self.open_shm_drawer();
                        actions.open_shm_telemetry_picker = true;
                    } else {
                        self.shm_drawer_tab = 0;
                        self.open_shm_drawer();
                        self.apply_lens_to_views(
                            SpatialLens::ShmDigitalTwin,
                            build_view,
                            universal_canvas,
                        );
                        actions.run_shm_digital_twin = true;
                    }
                }
                "/placard" | "/atc20" => {
                    self.shm_drawer_tab = 1;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    actions.run_shm_digital_twin = true;
                }
                "/shm-fault" => {
                    let parts: Vec<&str> = arg.split_whitespace().collect();
                    let scen = parts
                        .first()
                        .and_then(|s| crate::core::ShmFaultScenario::from_keyword(s))
                        .unwrap_or(crate::core::ShmFaultScenario::SoftStoryFirstFloor);
                    let sev = parts
                        .get(1)
                        .and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
                        .unwrap_or(75.0)
                        .clamp(0.0, 100.0);
                    let story = parts
                        .get(2)
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(1)
                        .max(1);
                    universal_canvas.shm_twin_config.fault_scenario = scen;
                    universal_canvas.shm_twin_config.severity_pct = sev;
                    universal_canvas.shm_twin_config.target_story = story;
                    self.shm_drawer_tab = 0;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    actions.run_shm_digital_twin = true;
                    self.push_trace(
                        format!("SHM Fault: {} ({sev:.0}% @ L{story})", scen.short_label()),
                        true,
                    );
                }
                "/shm-ifcx" => {
                    actions.export_shm_atc20_ifcx = true;
                }
                "/evac" | "/lifelines" => {
                    universal_canvas.evacuation_config.overlay_visible = true;
                    self.open_evacuation_drawer();
                    actions.run_evacuation_routing = true;
                }
                "/safezone" => {
                    if !universal_canvas
                        .city_scene
                        .zones
                        .iter()
                        .any(|z| z.kind == crate::core::IfcxZoneKind::Park)
                    {
                        universal_canvas.city_scene.add_zone(
                            [65.0, 55.0],
                            32.0,
                            24.0,
                            0.0,
                            crate::core::IfcxZoneKind::Park,
                        );
                    }
                    universal_canvas.evacuation_config.overlay_visible = true;
                    self.open_evacuation_drawer();
                    actions.run_evacuation_routing = true;
                    self.push_trace(
                        "Added Safe Assembly Park Zone & recomputed evacuation lifelines",
                        true,
                    );
                }
                "/evac-ifcx" => {
                    actions.export_evacuation_ifcx = true;
                }
                "/watchdog" => {
                    self.shm_drawer_tab = 2;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    if universal_canvas.last_shm_twin_report.is_none() {
                        actions.run_shm_digital_twin = true;
                    }
                    self.push_trace("Opened Live WebSocket SHM Watchdog", true);
                }
                "/ws" => {
                    let url = if arg.trim().is_empty() {
                        universal_canvas.shm_watchdog.config.ws_url.clone()
                    } else {
                        arg.trim().to_string()
                    };
                    self.shm_drawer_tab = 2;
                    self.open_shm_drawer();
                    actions.connect_shm_ws = Some(url);
                }
                "/ws-stop" => {
                    actions.disconnect_shm_ws = true;
                }
                "/ws-sim" => {
                    let mode = crate::core::WatchdogSimMode::from_keyword(arg)
                        .unwrap_or(crate::core::WatchdogSimMode::AftershockPulse);
                    universal_canvas.shm_watchdog.start_simulator(mode);
                    self.shm_drawer_tab = 2;
                    self.open_shm_drawer();
                    self.apply_lens_to_views(
                        SpatialLens::ShmDigitalTwin,
                        build_view,
                        universal_canvas,
                    );
                    self.push_trace(format!("Watchdog Simulator: {}", mode.label()), true);
                }
                "/watchdog-ifcx" => {
                    actions.export_watchdog_ifcx = true;
                }
                "/gpu-compute" => {
                    self.ground_motion_drawer_tab = 2;
                    self.toggle_exclusive_drawer("ground_motion");
                    if universal_canvas.last_monte_carlo_report.is_none() {
                        actions.run_gpu_monte_carlo =
                            Some(universal_canvas.monte_carlo_config.normalized_sample_count());
                    }
                    self.push_trace("Opened WebGPU @compute Particles & Monte Carlo drawer", true);
                }
                "/mc" => {
                    let n = arg
                        .trim()
                        .parse::<u32>()
                        .ok()
                        .unwrap_or_else(|| universal_canvas.monte_carlo_config.normalized_sample_count())
                        .clamp(256, 16_384);
                    universal_canvas.monte_carlo_config.sample_count = n;
                    self.ground_motion_drawer_tab = 2;
                    if !self.ground_motion_drawer_open {
                        self.toggle_exclusive_drawer("ground_motion");
                    }
                    actions.run_gpu_monte_carlo = Some(n);
                    self.push_trace(
                        format!("Running FEMA P-58 Monte Carlo ({n} realizations)..."),
                        true,
                    );
                }
                "/particles" => {
                    if let Ok(cnt) = arg.trim().parse::<u32>() {
                        universal_canvas.monte_carlo_config.gpu_particle_count =
                            cnt.clamp(128, 4096);
                        universal_canvas.monte_carlo_config.gpu_particles_enabled = true;
                    } else {
                        universal_canvas.monte_carlo_config.gpu_particles_enabled =
                            !universal_canvas.monte_carlo_config.gpu_particles_enabled;
                    }
                    let on = universal_canvas.monte_carlo_config.gpu_particles_enabled;
                    if on && !universal_canvas.district_hazard.any_active() {
                        universal_canvas.district_hazard.seismic_wave_active = true;
                        universal_canvas.district_hazard.venturi_wind_active = true;
                    }
                    self.push_trace(
                        if on {
                            format!(
                                "GPU @compute Particles ON ({} @ 60 FPS)",
                                universal_canvas.monte_carlo_config.normalized_particle_count()
                            )
                        } else {
                            "GPU @compute Particles OFF".to_string()
                        },
                        true,
                    );
                }
                "/mc-ifcx" => {
                    actions.export_monte_carlo_ifcx = true;
                }
                "/checkpoint" => {
                    let lbl = if arg.trim().is_empty() {
                        universal_canvas.collab_state.checkpoint_label_draft.clone()
                    } else {
                        arg.trim().to_string()
                    };
                    self.open_collab_drawer(0);
                    actions.create_checkpoint_label = Some(lbl.clone());
                    self.push_trace(format!("Capturing signed IFCX checkpoint '{lbl}'..."), true);
                }
                "/approve" => {
                    self.open_collab_drawer(0);
                    actions.approve_checkpoint = true;
                    self.push_trace("Signing off active IFCX governance checkpoint...", true);
                }
                "/merge-ifcx" => {
                    self.open_collab_drawer(0);
                    actions.run_ifcx_three_way_merge = true;
                    self.push_trace("Running deterministic 3-way IFCX peer merge...", true);
                }
                "/risk-map" => {
                    let low = arg.trim().to_ascii_lowercase();
                    if low.contains("pml") || low.contains("mc") || low.contains("loss") {
                        universal_canvas.collab_state.config.risk_metric =
                            crate::core::ifcx_collab::DistrictRiskMetric::McPml90Loss;
                        universal_canvas.collab_state.config.risk_heatmap_visible = true;
                    } else if low.contains("eigengate") || low.contains("dcr") {
                        universal_canvas.collab_state.config.risk_metric =
                            crate::core::ifcx_collab::DistrictRiskMetric::StructuralEigenGate;
                        universal_canvas.collab_state.config.risk_heatmap_visible = true;
                    } else if low.contains("iso") || low.contains("evac") {
                        universal_canvas.collab_state.config.risk_metric =
                            crate::core::ifcx_collab::DistrictRiskMetric::EvacuationIsolation;
                        universal_canvas.collab_state.config.risk_heatmap_visible = true;
                    } else if low.contains("comp") {
                        universal_canvas.collab_state.config.risk_metric =
                            crate::core::ifcx_collab::DistrictRiskMetric::CompositeRisk;
                        universal_canvas.collab_state.config.risk_heatmap_visible = true;
                    } else if low == "off" {
                        universal_canvas.collab_state.config.risk_heatmap_visible = false;
                    } else {
                        universal_canvas.collab_state.config.risk_heatmap_visible = true;
                    }
                    self.open_collab_drawer(1);
                    actions.refresh_district_risk_field = true;
                    self.push_trace(
                        format!(
                            "District Risk Heatmap ({}) {}",
                            universal_canvas.collab_state.config.risk_metric.short_label(),
                            if universal_canvas.collab_state.config.risk_heatmap_visible {
                                "ON"
                            } else {
                                "OFF"
                            }
                        ),
                        true,
                    );
                }
                "/collab-ifcx" => {
                    self.open_collab_drawer(0);
                    actions.export_collab_bundle_ifcx = true;
                }
                "/trade-study" => {
                    let low = arg.trim().to_ascii_lowercase();
                    let obj = if low.contains("safe") || low.contains("eigengate") {
                        crate::core::trade_study::TradeStudyObjective::MaxSafetyEigenGate
                    } else if low.contains("cost") {
                        crate::core::trade_study::TradeStudyObjective::MinCost
                    } else if low.contains("carb") || low.contains("co2") {
                        crate::core::trade_study::TradeStudyObjective::MinCarbon
                    } else if low.contains("bal") {
                        crate::core::trade_study::TradeStudyObjective::Balanced
                    } else {
                        universal_canvas.trade_study_config.objective
                    };
                    universal_canvas.trade_study_config.objective = obj;
                    self.open_collab_drawer(2);
                    actions.run_trade_study = Some(obj);
                    actions.run_permit_compliance = true;
                    self.push_trace(
                        format!("Running 6-scenario Trade-Study ({})...", obj.short_label()),
                        true,
                    );
                }
                "/permit-check" => {
                    self.open_collab_drawer(2);
                    actions.run_permit_compliance = true;
                    self.push_trace("Auditing 10-clause SNI/ASCE/UFC Permit Matrix...", true);
                }
                "/permit-svg" => {
                    self.open_collab_drawer(2);
                    actions.export_permit_annex_svg = true;
                }
                "/relay" => {
                    let trimmed_arg = arg.trim();
                    self.open_collab_drawer(0);
                    if trimmed_arg.eq_ignore_ascii_case("off")
                        || trimmed_arg.eq_ignore_ascii_case("stop")
                    {
                        actions.disconnect_collab_relay = true;
                    } else {
                        let url = if trimmed_arg.is_empty() {
                            universal_canvas.collab_state.relay_url_draft.clone()
                        } else {
                            universal_canvas.collab_state.relay_url_draft = trimmed_arg.to_string();
                            trimmed_arg.to_string()
                        };
                        actions.connect_collab_relay = Some(url);
                    }
                }
                "/sync-peer" => {
                    self.open_collab_drawer(0);
                    actions.sync_peer_relay_delta = true;
                    self.push_trace("Syncing peer IFCX delta via Relay + 3-Way Merge...", true);
                }
                "/bbs" | "/cutlist" => {
                    let stock_m = if arg.trim().contains('6') { 6.0 } else { 12.0 };
                    universal_canvas.field_fab_config.stock_length_m = stock_m;
                    self.open_field_qaqc_drawer(0);
                    actions.run_field_fabrication = Some(stock_m);
                    self.push_trace(format!("Optimizing 1D Rebar BBS Cut-List ({stock_m:.0}m stock)..."), true);
                }
                "/qaqc-3d" => {
                    self.open_field_qaqc_drawer(1);
                    actions.toggle_field_qaqc_3d = true;
                }
                "/waypoint" => {
                    self.open_field_qaqc_drawer(1);
                    let total = universal_canvas
                        .last_field_fab_report
                        .as_ref()
                        .map(|r| r.waypoints.len())
                        .unwrap_or_else(|| {
                            let lp = &universal_canvas.field_qaqc_ledger;
                            lp.itps.len() + lp.ncrs.len()
                        })
                        .max(1);
                    let next_idx = if let Ok(one_based) = arg.trim().parse::<usize>() {
                        one_based.saturating_sub(1) % total
                    } else {
                        (universal_canvas.active_waypoint_idx + 1) % total
                    };
                    actions.fly_to_waypoint_idx = Some(next_idx);
                }
                "/shop-svg" => {
                    self.open_field_qaqc_drawer(0);
                    actions.export_shop_drawing_svg = true;
                }
                "/section" | "/cut" => {
                    let elev_opt = arg
                        .trim_end_matches('m')
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite());
                    if let Some(z_m) = elev_opt {
                        universal_canvas.earth_bim_author.section_cut_elev_m =
                            z_m.clamp(0.5, 120.0);
                        universal_canvas.earth_bim_author.section_cut_active = true;
                    } else {
                        universal_canvas.earth_bim_author.section_cut_active =
                            !universal_canvas.earth_bim_author.section_cut_active;
                    }
                    let trace_msg = if universal_canvas.earth_bim_author.section_cut_active {
                        format!(
                            "Section Cut ON @ Z = {:.1}m",
                            universal_canvas.earth_bim_author.section_cut_elev_m
                        )
                    } else {
                        "Section Cut OFF".to_string()
                    };
                    self.push_trace(trace_msg, true);
                }
                "/clonestory" | "/clone" => {
                    match self.clone_active_story_upward(build_view) {
                        Ok(_) => {
                            actions.sync_fem_and_earth = true;
                            actions.fit_camera = true;
                        }
                        Err(e) => self.push_trace(e, false),
                    }
                }
                "/footings" | "/foundations" => {
                    match self.auto_generate_missing_footings(build_view) {
                        Ok(_) => {
                            actions.sync_fem_and_earth = true;
                        }
                        Err(e) => self.push_trace(e, false),
                    }
                }
                "/fly" | "/go" | "/loc" => {
                    if let Some((label, lat, lon, zoom)) =
                        crate::core::gis::SlippyMapState::resolve_location_query(arg)
                    {
                        self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                        universal_canvas.globe_mode = true;
                        universal_canvas
                            .earth_world
                            .start_fly_to(&universal_canvas.world_map, lat, lon, zoom, &label);
                        universal_canvas.ground_origin_lat_lon = (lat, lon);
                        universal_canvas.ground_origin_anchored = true;
                        self.selected_target = Some(SmartSelectionTarget::EarthSite { lat, lon });
                        self.push_trace(format!("Flying Earth -> {label}"), true);
                    } else {
                        self.push_trace("Usage: /fly <city or lat, lon>", false);
                    }
                }
                "/repair" => {
                    actions.run_auto_repair = true;
                    self.command_bar_expanded = true;
                    self.push_trace("Executed Closed-Loop AI Repair", true);
                }
                "/svd" | "/eigengate" | "/ari" => {
                    self.command_bar_expanded = true;
                    ai_assistant.execute_prompt(
                        "check svd mechanism and screen eigengate",
                        NavigationPillar::AnalysisSimulation,
                        members,
                        projects,
                        db_components,
                    );
                    self.push_trace("Ran SVD Kinematic Guard & EigenGate Linear Screening", true);
                }
                "/eigenflow" | "/woodbury" => {
                    self.command_bar_expanded = true;
                    self.open_eigenflow_drawer(0);
                    let target_id = arg.trim().parse::<usize>().ok();
                    if target_id.is_some() {
                        actions.eigenflow_select_element = target_id;
                    }
                    actions.run_eigenflow_studio = true;
                    let rep = universal_canvas.run_eigenflow_studio(&build_view.bim_bus.project);
                    let wb = &rep.woodbury;
                    ai_assistant.chat_history.push(crate::models::ChatMessage {
                        is_user: false,
                        message: format!(
                            "[EigenFlow Woodbury Non-Linear (Element #{})]: m_s={:.3}, ΔN={:.1} kN -> P2={} members, RRR={:.2}, μ={:.2} | {}",
                            wb.disrupted_element_id,
                            wb.stability_margin_ms,
                            wb.total_shed_force_kn,
                            wb.redistributed_p2.len(),
                            wb.residual_reserve_ratio_rrr,
                            wb.max_ductility_mu,
                            wb.prescriptive_recommendation,
                        ),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
                    self.push_trace("Ran Branch 2 EigenFlow Woodbury Non-Linear Screening", true);
                }
                "/eigenflow-studio" => {
                    self.open_eigenflow_drawer(0);
                    actions.run_eigenflow_studio = true;
                }
                "/pushover" => {
                    self.open_eigenflow_drawer(1);
                    let lower = arg.to_ascii_lowercase();
                    let pat = if lower.contains("uni") {
                        crate::core::pushover::LateralLoadPattern::UniformMass
                    } else if lower.contains("tri") {
                        crate::core::pushover::LateralLoadPattern::InvertedTriangular
                    } else {
                        crate::core::pushover::LateralLoadPattern::FirstModeModal
                    };
                    actions.eigenflow_set_pattern = Some(pat);
                }
                "/adrs" => {
                    self.open_eigenflow_drawer(1);
                    actions.run_eigenflow_studio = true;
                }
                "/eigenflow-3d" => {
                    self.open_eigenflow_drawer(0);
                    actions.toggle_eigenflow_3d = true;
                }
                "/eigenflow-ifcx" => {
                    actions.export_eigenflow_ifcx = true;
                }
                "/eigenflow-svg" => {
                    actions.export_eigenflow_svg = true;
                }
                "/extreme" => {
                    self.open_extreme_events_drawer(0);
                    actions.run_extreme_events = true;
                }
                "/blast" => {
                    let nums: Vec<f64> = arg
                        .split_whitespace()
                        .filter_map(|s| s.trim_end_matches("kg").trim_end_matches('m').parse::<f64>().ok())
                        .collect();
                    if let Some(&mass) = nums.first() {
                        universal_canvas.extreme_events_config.explosive_mass_kg = mass.clamp(10.0, 5_000.0);
                    }
                    if let Some(&standoff) = nums.get(1) {
                        universal_canvas.extreme_events_config.standoff_distance_m = standoff.clamp(3.0, 120.0);
                    }
                    universal_canvas.extreme_events_config.mode =
                        crate::core::extreme_events::ExtremeEventMode::Blast;
                    self.open_extreme_events_drawer(0);
                    actions.run_extreme_events = true;
                }
                "/fire" => {
                    if let Ok(mins) = arg.trim().trim_end_matches("min").trim_end_matches('m').parse::<f64>() {
                        universal_canvas.extreme_events_config.exposure_minutes = mins.clamp(0.0, 180.0);
                    }
                    universal_canvas.extreme_events_config.mode =
                        crate::core::extreme_events::ExtremeEventMode::StructuralFire;
                    self.open_extreme_events_drawer(1);
                    actions.run_extreme_events = true;
                }
                "/fire-story" => {
                    if let Ok(story_1b) = arg.trim().trim_start_matches('L').parse::<usize>() {
                        universal_canvas.extreme_events_config.fire_story_idx = story_1b.saturating_sub(1);
                    }
                    universal_canvas.extreme_events_config.mode =
                        crate::core::extreme_events::ExtremeEventMode::StructuralFire;
                    self.open_extreme_events_drawer(1);
                    actions.run_extreme_events = true;
                }
                "/extreme-3d" => {
                    self.open_extreme_events_drawer(0);
                    actions.toggle_extreme_events_3d = true;
                }
                "/extreme-ifcx" => {
                    actions.export_extreme_events_ifcx = true;
                }
                "/extreme-svg" => {
                    actions.export_extreme_events_svg = true;
                }
                "/isolation" | "/lrb" | "/fps" | "/fvd" => {
                    let iso = if cmd == "/fps" || arg.to_ascii_lowercase().contains("fps") {
                        crate::core::isolation::IsolatorType::FrictionPendulum
                    } else if cmd == "/fvd" || arg.to_ascii_lowercase().contains("fvd") {
                        crate::core::isolation::IsolatorType::FluidViscousDamper
                    } else if arg.to_ascii_lowercase().contains("hdrb") {
                        crate::core::isolation::IsolatorType::HighDampingRubberBearing
                    } else {
                        crate::core::isolation::IsolatorType::LeadRubberBearing
                    };
                    universal_canvas.isolation_studio_config.isolator_type = iso;
                    self.open_isolation_drawer(0);
                    actions.run_isolation_studio = true;
                }
                "/isolation-3d" => {
                    self.open_isolation_drawer(0);
                    actions.toggle_isolation_3d = true;
                }
                "/isolation-ifcx" => {
                    actions.export_isolation_ifcx = true;
                }
                "/isolation-svg" => {
                    actions.export_isolation_svg = true;
                }
                "/wind-studio" | "/aero" | "/cladding" => {
                    self.open_wind_drawer(0);
                    actions.run_wind_studio = true;
                }
                "/wind-dir" => {
                    let nums: Vec<f64> = arg
                        .split_whitespace()
                        .filter_map(|t| t.trim_end_matches("deg").parse::<f64>().ok())
                        .collect();
                    if let Some(&v) = nums.first() {
                        if nums.len() >= 2 {
                            universal_canvas.wind_studio_config.basic_wind_speed_ms =
                                v.clamp(20.0, 90.0);
                            universal_canvas.wind_studio_config.wind_azimuth_deg =
                                nums[1].rem_euclid(360.0);
                        } else if v > 90.0 {
                            universal_canvas.wind_studio_config.wind_azimuth_deg =
                                v.rem_euclid(360.0);
                        } else {
                            universal_canvas.wind_studio_config.basic_wind_speed_ms =
                                v.clamp(20.0, 90.0);
                        }
                    }
                    self.open_wind_drawer(0);
                    actions.run_wind_studio = true;
                }
                "/wind-exposure" => {
                    let lo = arg.to_ascii_lowercase();
                    let exp = if lo.contains('b') {
                        crate::core::wind::ExposureCategory::ExposureB
                    } else if lo.contains('d') {
                        crate::core::wind::ExposureCategory::ExposureD
                    } else {
                        crate::core::wind::ExposureCategory::ExposureC
                    };
                    universal_canvas.wind_studio_config.exposure = exp;
                    self.open_wind_drawer(0);
                    actions.run_wind_studio = true;
                }
                "/wind-3d" => {
                    self.open_wind_drawer(0);
                    actions.toggle_wind_3d = true;
                }
                "/wind-ifcx" => {
                    actions.export_wind_ifcx = true;
                }
                "/wind-svg" => {
                    actions.export_wind_svg = true;
                }
                "/shear-wall" | "/sbe" => {
                    let nums: Vec<f64> = arg
                        .split_whitespace()
                        .filter_map(|t| t.parse::<f64>().ok())
                        .collect();
                    if let Some(&tw) = nums.first() {
                        universal_canvas.shear_wall_studio_config.wall_thickness_mm =
                            tw.clamp(200.0, 800.0);
                    }
                    if let Some(&fc) = nums.get(1) {
                        universal_canvas.shear_wall_studio_config.fc_mpa = fc.clamp(25.0, 70.0);
                    }
                    self.open_shear_wall_drawer(0);
                    actions.run_shear_wall_studio = true;
                }
                "/coupling-beam" => {
                    let nums: Vec<f64> = arg
                        .split_whitespace()
                        .filter_map(|t| t.parse::<f64>().ok())
                        .collect();
                    if let Some(&ln) = nums.first() {
                        universal_canvas.shear_wall_studio_config.cb_clear_span_m =
                            ln.clamp(0.8, 4.5);
                    }
                    if let Some(&h) = nums.get(1) {
                        universal_canvas.shear_wall_studio_config.cb_depth_m =
                            h.clamp(0.45, 1.8);
                    }
                    self.open_shear_wall_drawer(1);
                    actions.run_shear_wall_studio = true;
                }
                "/shear-wall-3d" => {
                    self.open_shear_wall_drawer(0);
                    actions.toggle_shear_wall_3d = true;
                }
                "/shear-wall-ifcx" => {
                    actions.export_shear_wall_ifcx = true;
                }
                "/shear-wall-svg" => {
                    actions.export_shear_wall_svg = true;
                }
                "/connection" | "/endplate" | "/shear-tab" => {
                    if cmd == "/shear-tab" {
                        universal_canvas.connection_studio_config.connection_type =
                            crate::core::connection::ConnectionType::ShearTabSinglePlate;
                    } else if cmd == "/endplate" {
                        universal_canvas.connection_studio_config.connection_type =
                            crate::core::connection::ConnectionType::ExtendedEndPlate4E;
                    }
                    let nums: Vec<f64> = arg
                        .split_whitespace()
                        .filter_map(|t| t.parse::<f64>().ok())
                        .collect();
                    if let Some(&db) = nums.first() {
                        universal_canvas.connection_studio_config.bolt_diameter_mm =
                            db.clamp(16.0, 36.0);
                    }
                    if let Some(&tp) = nums.get(1) {
                        universal_canvas.connection_studio_config.plate_thickness_mm =
                            tp.clamp(10.0, 50.0);
                    }
                    self.open_connection_drawer(0);
                    actions.run_connection_studio = true;
                }
                "/connection-3d" => {
                    self.open_connection_drawer(0);
                    actions.toggle_connection_3d = true;
                }
                "/connection-ifcx" => {
                    actions.export_connection_ifcx = true;
                }
                "/connection-svg" => {
                    actions.export_connection_svg = true;
                }
                "/prestress" | "/tendon" | "/pt-losses" => {
                    let lo = arg.to_ascii_lowercase();
                    if lo.contains("harp") {
                        universal_canvas.prestress_studio_config.profile_kind =
                            crate::core::prestressed_concrete::TendonProfileKind::HarpedTwoPoint;
                    } else if lo.contains("straight") {
                        universal_canvas.prestress_studio_config.profile_kind =
                            crate::core::prestressed_concrete::TendonProfileKind::StraightEccentric;
                    } else if lo.contains("para") {
                        universal_canvas.prestress_studio_config.profile_kind =
                            crate::core::prestressed_concrete::TendonProfileKind::Parabolic;
                    }
                    if let Some(ns) = arg
                        .split_whitespace()
                        .filter_map(|t| t.parse::<usize>().ok())
                        .next()
                    {
                        universal_canvas.prestress_studio_config.num_strands = ns.clamp(4, 64);
                    }
                    self.open_prestress_drawer(if cmd == "/pt-losses" { 1 } else { 0 });
                    actions.run_prestress_studio = true;
                }
                "/prestress-3d" => {
                    self.open_prestress_drawer(0);
                    actions.toggle_prestress_3d = true;
                }
                "/prestress-ifcx" => {
                    actions.export_prestress_ifcx = true;
                }
                "/prestress-svg" => {
                    actions.export_prestress_svg = true;
                }
                "/composite" | "/studs" | "/floor-vib" => {
                    let lo = arg.to_ascii_lowercase();
                    if lo.contains("para") {
                        universal_canvas.composite_studio_config.deck_orientation =
                            crate::core::composite::DeckOrientation::Parallel;
                    } else if lo.contains("perp") {
                        universal_canvas.composite_studio_config.deck_orientation =
                            crate::core::composite::DeckOrientation::Perpendicular;
                    }
                    if let Some(st) = arg
                        .split_whitespace()
                        .filter_map(|t| t.parse::<usize>().ok())
                        .next()
                    {
                        universal_canvas.composite_studio_config.studs_provided = st.clamp(12, 160);
                    }
                    self.open_composite_drawer(if cmd == "/floor-vib" { 1 } else { 0 });
                    actions.run_composite_studio = true;
                }
                "/composite-3d" => {
                    self.open_composite_drawer(0);
                    actions.toggle_composite_3d = true;
                }
                "/composite-ifcx" => {
                    actions.export_composite_ifcx = true;
                }
                "/composite-svg" => {
                    actions.export_composite_svg = true;
                }
                "/reliability" | "/form" | "/sorm" | "/fragility" => {
                    let lo = arg.to_ascii_lowercase();
                    if cmd == "/form" || lo.contains("form") {
                        universal_canvas.reliability_studio_config.method =
                            crate::core::reliability_studio::ReliabilityMethod::FormHasoferLind;
                    } else if cmd == "/sorm" || lo.contains("sorm") {
                        universal_canvas.reliability_studio_config.method =
                            crate::core::reliability_studio::ReliabilityMethod::SormBreitung;
                    } else if lo.contains("fosm") {
                        universal_canvas.reliability_studio_config.method =
                            crate::core::reliability_studio::ReliabilityMethod::FosmLogNormal;
                    }
                    if lo.contains("drift") {
                        universal_canvas.reliability_studio_config.limit_state =
                            crate::core::reliability_studio::LimitStateKind::InterstoryDrift;
                    } else if lo.contains("col") || lo.contains("axial") {
                        universal_canvas.reliability_studio_config.limit_state =
                            crate::core::reliability_studio::LimitStateKind::AxialColumn;
                    } else if lo.contains("shear") {
                        universal_canvas.reliability_studio_config.limit_state =
                            crate::core::reliability_studio::LimitStateKind::ShearBrittle;
                    } else if lo.contains("flex") {
                        universal_canvas.reliability_studio_config.limit_state =
                            crate::core::reliability_studio::LimitStateKind::FlexurePlastic;
                    }
                    if let Some(tb) = arg
                        .split_whitespace()
                        .filter_map(|t| t.parse::<f64>().ok())
                        .next()
                    {
                        universal_canvas.reliability_studio_config.target_beta =
                            tb.clamp(2.0, 5.5);
                    }
                    self.open_reliability_drawer(if cmd == "/fragility" { 1 } else { 0 });
                    actions.run_reliability_studio = true;
                }
                "/reliability-3d" => {
                    self.open_reliability_drawer(0);
                    actions.toggle_reliability_3d = true;
                }
                "/reliability-ifcx" => {
                    actions.export_reliability_ifcx = true;
                }
                "/reliability-svg" => {
                    actions.export_reliability_svg = true;
                }
                "/foundation" | "/pile-group" | "/punching" | "/winkler" => {
                    let lo = arg.to_ascii_lowercase();
                    if lo.contains("soft") {
                        universal_canvas.foundation_studio_config.soil_type =
                            crate::core::geotech::SoilType::SoftClay;
                    } else if lo.contains("stiff") {
                        universal_canvas.foundation_studio_config.soil_type =
                            crate::core::geotech::SoilType::StiffClay;
                    } else if lo.contains("loose") {
                        universal_canvas.foundation_studio_config.soil_type =
                            crate::core::geotech::SoilType::LooseSand;
                    } else if lo.contains("dense") {
                        universal_canvas.foundation_studio_config.soil_type =
                            crate::core::geotech::SoilType::DenseSand;
                    } else if lo.contains("rock") || lo.contains("bed") {
                        universal_canvas.foundation_studio_config.soil_type =
                            crate::core::geotech::SoilType::Bedrock;
                    }
                    for tok in lo.split_whitespace() {
                        if let Some((lx, ly)) = tok.split_once('x') {
                            if let (Ok(nx), Ok(ny)) = (lx.parse::<usize>(), ly.parse::<usize>()) {
                                universal_canvas.foundation_studio_config.piles_x = nx.clamp(1, 5);
                                universal_canvas.foundation_studio_config.piles_y = ny.clamp(1, 5);
                            }
                        } else if let Ok(len_m) = tok.trim_end_matches('m').parse::<f64>() {
                            universal_canvas.foundation_studio_config.pile_length_m =
                                len_m.clamp(6.0, 48.0);
                        }
                    }
                    self.open_foundation_drawer(if cmd == "/punching" || cmd == "/winkler" {
                        1
                    } else {
                        0
                    });
                    actions.run_foundation_studio = true;
                }
                "/foundation-3d" => {
                    self.open_foundation_drawer(0);
                    actions.toggle_foundation_3d = true;
                }
                "/foundation-ifcx" => {
                    actions.export_foundation_ifcx = true;
                }
                "/foundation-svg" => {
                    actions.export_foundation_svg = true;
                }
                "/4d-studio" | "/sequence" | "/scurve" | "/lca" => {
                    let lo = arg.to_ascii_lowercase();
                    if lo.contains("dri") || lo.contains("h2") {
                        universal_canvas.construction_lca_config.steel_route =
                            crate::core::construction_lca_studio::SteelSmeltingRoute::GreenHydrogenDri;
                    } else if lo.contains("eaf") {
                        universal_canvas.construction_lca_config.steel_route =
                            crate::core::construction_lca_studio::SteelSmeltingRoute::EafRecycled;
                    } else if lo.contains("bof") {
                        universal_canvas.construction_lca_config.steel_route =
                            crate::core::construction_lca_studio::SteelSmeltingRoute::BfBofStandard;
                    }
                    if lo.contains("geo") {
                        universal_canvas.construction_lca_config.concrete_mix =
                            crate::core::construction_lca_studio::ConcreteMixGrade::GeopolymerGreen;
                    } else if lo.contains("ggbs") {
                        universal_canvas.construction_lca_config.concrete_mix =
                            crate::core::construction_lca_studio::ConcreteMixGrade::Ggbs30LowCarbon;
                    } else if lo.contains("opc") {
                        universal_canvas.construction_lca_config.concrete_mix =
                            crate::core::construction_lca_studio::ConcreteMixGrade::OpcStandard;
                    }
                    if let Some(day) = lo
                        .split_whitespace()
                        .filter_map(|t| t.trim_start_matches("day").parse::<usize>().ok())
                        .next()
                    {
                        universal_canvas.construction_lca_config.active_day = day.min(540);
                    }
                    self.open_construction_lca_drawer(if cmd == "/lca" { 1 } else { 0 });
                    actions.run_construction_lca_studio = true;
                }
                "/4d-3d" => {
                    self.open_construction_lca_drawer(0);
                    actions.toggle_construction_3d = true;
                }
                "/4d-ifcx" => {
                    actions.export_construction_ifcx = true;
                }
                "/4d-svg" => {
                    actions.export_construction_svg = true;
                }
                "/undo" => {
                    if let Some(msg) = build_view.bim_bus.undo() {
                        actions.sync_fem_and_earth = true;
                        self.push_trace(msg, true);
                    } else if universal_canvas.undo() {
                        self.push_trace("Undid canvas stroke", true);
                    } else {
                        self.push_trace("Nothing to undo", false);
                    }
                }
                "/redo" => {
                    if let Some(msg) = build_view.bim_bus.redo() {
                        actions.sync_fem_and_earth = true;
                        self.push_trace(msg, true);
                    } else if universal_canvas.redo() {
                        self.push_trace("Redid canvas stroke", true);
                    } else {
                        self.push_trace("Nothing to redo", false);
                    }
                }
                "/save" | "/export" => {
                    actions.export_archive = true;
                    self.push_trace("Exported .eigen.json workspace", true);
                }
                "/theme" => {
                    actions.toggle_theme = true;
                    self.push_trace("Toggled Pro Dark / Engineering Daylight", true);
                }
                "/settings" => {
                    actions.open_settings = true;
                    self.push_trace("Opened Settings", true);
                }
                "/vault" => self.open_module_overlay(NavigationPillar::MyLibrary),
                "/add-folder" | "/vault-folder" => {
                    actions.add_vault_folder = true;
                    self.vault_drawer_open = true;
                    self.push_trace("Choose local folder as Engineering Vault", true);
                }
                "/context" | "/add-context" => {
                    self.context_picker_open = true;
                    self.push_trace("Opened @ Add Context picker", true);
                }
                "/world-portfolio" => {
                    self.attach_context(AiContextAttachment::WorldPortfolio);
                    let sum = universal_canvas.world_index.portfolio_summary();
                    if sum.critical_count > 0 {
                        actions.fly_to_next_world_building =
                            Some(crate::core::gis::WorldPortfolioFilter::CriticalRisk);
                    } else if sum.unlocated_count > 0 {
                        actions.fly_to_next_world_building =
                            Some(crate::core::gis::WorldPortfolioFilter::UnlocatedStaging);
                    }
                    self.push_trace(
                        format!(
                            "Portfolio: {} total ({} unlocated, {} critical)",
                            sum.total_buildings, sum.unlocated_count, sum.critical_count
                        ),
                        true,
                    );
                }
                "/agent" => self.open_module_overlay(NavigationPillar::AiAgent),
                "/planning" => self.open_module_overlay(NavigationPillar::ResearchConcept),
                "/drafting" => self.open_module_overlay(NavigationPillar::ParametricDED),
                "/geometry" => self.open_module_overlay(NavigationPillar::GeometryAssembly),
                "/analysis" => self.open_module_overlay(NavigationPillar::AnalysisSimulation),
                "/schedule" => self.open_module_overlay(NavigationPillar::TimeScheduling),
                "/cost" | "/boq" | "/rab" => self.open_module_overlay(NavigationPillar::CostBudgeting),
                "/sustainability" | "/carbon" => {
                    self.open_module_overlay(NavigationPillar::SustainabilityEnergy)
                }
                "/facility" => self.open_module_overlay(NavigationPillar::FacilityManagement),
                "/safety" => self.open_module_overlay(NavigationPillar::SafetyHealth),
                "/construction" | "/lean" | "/qaqc" => {
                    self.open_module_overlay(NavigationPillar::LeanConstruction)
                }
                "/automation" | "/ifc" | "/dfma" => {
                    self.open_module_overlay(NavigationPillar::IndustrializedAutomation)
                }
                "/prediction" => self.open_module_overlay(NavigationPillar::Prediction),
                "/report" | "/pbg" => self.open_module_overlay(NavigationPillar::ReportPublishing),
                _ => {
                    // Fallback: treat unknown `/...` as natural language query
                    self.command_bar_expanded = true;
                    ai_assistant.execute_prompt(
                        trimmed.trim_start_matches('/'),
                        NavigationPillar::GeometryAssembly,
                        members,
                        projects,
                        db_components,
                    );
                    self.push_trace(format!("Ran AI prompt '{cmd}'"), true);
                }
            }
            return actions;
        }

        // 1b. Smart Local Imperative Routing when offline or explicit workspace imperatives are requested
        if lower == "undo" {
            actions.undo = true;
            return actions;
        } else if lower == "redo" {
            actions.redo = true;
            return actions;
        } else if lower == "focus 3d" || lower == "open 3d" || lower == "bim 3d" || lower == "earth 3d" {
            self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
            universal_canvas.globe_mode = true;
            actions.fit_camera = true;
            self.push_trace("Focused Earth 3D Canvas", true);
            return actions;
        } else if lower == "iso view" || lower == "isometric view" {
            self.apply_camera_preset(build_view, universal_canvas, "iso");
            actions.fit_camera = true;
            return actions;
        } else if lower == "top view" || lower == "plan view" {
            self.apply_camera_preset(build_view, universal_canvas, "top");
            actions.fit_camera = true;
            return actions;
        } else if lower == "front view" || lower == "elevation view" {
            self.apply_camera_preset(build_view, universal_canvas, "front");
            actions.fit_camera = true;
            return actions;
        } else if lower == "add story" || lower == "add level" || lower == "new story" {
            if self.add_story_above_top(build_view).is_ok() {
                actions.sync_fem_and_earth = true;
                actions.fit_camera = true;
            }
            return actions;
        } else if lower == "add slab" || lower == "create slab" {
            if self.add_slab_at_active_story(build_view).is_ok() {
                actions.sync_fem_and_earth = true;
                actions.fit_camera = true;
            }
            return actions;
        } else if lower == "next critical" || lower == "fly critical" {
            actions.fly_to_next_world_building = Some(crate::core::gis::WorldPortfolioFilter::CriticalRisk);
            return actions;
        } else if lower == "next unlocated" || lower == "next staging" {
            actions.fly_to_next_world_building = Some(crate::core::gis::WorldPortfolioFilter::UnlocatedStaging);
            return actions;
        } else if lower.contains("pounding") || lower.contains("neighborhood") || lower.contains("tetangga") {
            let sub = self.apply_proposal_actions(
                &[PlannedProposalAction::AuditNeighborhood],
                universal_canvas,
                build_view,
                ai_assistant,
            );
            actions.merge(sub);
            self.command_bar_expanded = true;
            return actions;
        } else if lower.contains("progressive collapse")
            || lower.contains("structgnn")
            || lower.contains("crc screening")
            || lower.contains("cascade")
        {
            actions.run_crc_cascade_screening = Some(universal_canvas.cascade_daf);
            self.push_trace(
                format!(
                    "Queued Two-Stage EigenGate + StructGNN-CRC Cascade (DAF {:.2}x)",
                    universal_canvas.cascade_daf
                ),
                true,
            );
            return actions;
        } else if lower == "pure vector" || lower == "vector earth" {
            let sub = self.apply_proposal_actions(
                &[PlannedProposalAction::SwitchPureVector],
                universal_canvas,
                build_view,
                ai_assistant,
            );
            actions.merge(sub);
            return actions;
        } else if lower.contains("portfolio") && (lower.contains("audit") || lower.contains("summary") || lower.contains("ringkas")) {
            let sum = universal_canvas.world_index.portfolio_summary();
            let msg = format!(
                "Planetary Portfolio Audit: {} indexed Vault models (GPS Verified: {}, City Hint: {}, User Pinned: {}, Unlocated Staging: {}, Critical Risk DCR>=90%/EigenGate<0.70: {}). Top Regions: {}",
                sum.total_buildings,
                sum.gps_verified_count,
                sum.city_hint_count,
                sum.user_pinned_count,
                sum.unlocated_count,
                sum.critical_count,
                sum.top_regions.iter().map(|(r, c)| format!("{r}:{c}")).collect::<Vec<_>>().join(", "),
            );
            ai_assistant.chat_history.push(crate::models::ChatMessage {
                is_user: false,
                message: msg,
                time: crate::core::web_bridge::unix_timestamp_label(),
            });
            self.command_bar_expanded = true;
            self.push_trace("Completed World Portfolio Audit", true);
            return actions;
        } else if let Some(target_sheet) = lower.strip_prefix("open ") {
            let pillar_opt = if target_sheet.contains("analysis") || target_sheet.contains("fem") {
                Some(NavigationPillar::AnalysisSimulation)
            } else if target_sheet.contains("sched") || target_sheet.contains("4d") {
                Some(NavigationPillar::TimeScheduling)
            } else if target_sheet.contains("budget") || target_sheet.contains("cost") || target_sheet.contains("boq") {
                Some(NavigationPillar::CostBudgeting)
            } else if target_sheet.contains("sustain") || target_sheet.contains("carbon") || target_sheet.contains("6d") {
                Some(NavigationPillar::SustainabilityEnergy)
            } else if target_sheet.contains("safety") || target_sheet.contains("8d") {
                Some(NavigationPillar::SafetyHealth)
            } else if target_sheet.contains("report") || target_sheet.contains("pbg") {
                Some(NavigationPillar::ReportPublishing)
            } else if target_sheet.contains("vault") {
                Some(NavigationPillar::MyLibrary)
            } else {
                None
            };
            if let Some(p) = pillar_opt {
                if self.agent_mode == AgentExecutionMode::Planning && self.agent_model != AgentModelChoice::EigenAiProFem {
                    self.queue_planning_proposal(
                        format!("Open {} Sheet", p.clean_name()),
                        "Review sheet navigation request before opening floating panel.",
                        format!("OPEN_SHEET({})", p.clean_name()),
                        vec![PlannedProposalAction::OpenSheet(p)],
                    );
                } else {
                    self.open_module_overlay(p);
                }
                return actions;
            }
        }

        // 2. Direct Earth Place Navigation if query matches a city or lat,lon cleanly
        if !lower.contains("portal")
            && !lower.contains("frame")
            && !lower.contains("col")
            && !lower.contains("beam")
            && !lower.contains("repair")
            && !lower.contains("eigengate")
            && !lower.contains("svd")
            && !lower.contains("cost")
        {
            if let Some((label, lat, lon, zoom)) =
                crate::core::gis::SlippyMapState::resolve_location_query(trimmed)
            {
                self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                universal_canvas.globe_mode = true;
                universal_canvas
                    .earth_world
                    .start_fly_to(&universal_canvas.world_map, lat, lon, zoom, &label);
                universal_canvas.ground_origin_lat_lon = (lat, lon);
                universal_canvas.ground_origin_anchored = true;
                self.selected_target = Some(SmartSelectionTarget::EarthSite { lat, lon });
                self.push_trace(format!("Earth Fly-To -> {label}"), true);
                return actions;
            }
        }

        // 3. Natural Language BIM Synthesis or Selected-Element Context Prompt
        if lower.contains("build portal")
            || lower.contains("create portal")
            || lower.starts_with("portal ")
            || lower.starts_with("frame ")
            || lower.starts_with("box ")
        {
            let norm = trimmed
                .replace("build ", "")
                .replace("create ", "")
                .replace("stories", "floors")
                .replace("story", "floors");
            if self.agent_mode == AgentExecutionMode::Planning {
                let (lat, lon) = universal_canvas.ground_origin_lat_lon;
                let site = compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
                self.queue_planning_proposal(
                    "Synthesize Parametric Structural Frame",
                    format!(
                        "Planning Mode pre-check at Earth Site ({lat:.3}°, {lon:.3}°): Ss={:.2}g, Wind={:.0} m/s. Click Apply Changes to execute BIM synthesis.",
                        site.seismic_pga_ss_g, site.design_wind_ms
                    ),
                    format!("BIM Command: {norm}"),
                    vec![PlannedProposalAction::ExecuteBimNl(norm)],
                );
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: true,
                    message: trimmed.to_string(),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: false,
                    message: "[Planning Mode]: Prepared structural frame proposal below. Click Apply Changes to build or Reject to cancel.".to_string(),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                self.sessions_dirty = true;
                return actions;
            }
            match build_view.bim_bus.execute_natural_language(&norm) {
                Ok(msg) => {
                    self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                    universal_canvas.globe_mode = true;
                    universal_canvas.ground_origin_anchored = true;
                    actions.sync_fem_and_earth = true;
                    actions.fit_camera = true;
                    self.push_trace(msg.clone(), true);
                    ai_assistant.chat_history.push(crate::models::ChatMessage {
                        is_user: true,
                        message: trimmed.to_string(),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
                    ai_assistant.chat_history.push(crate::models::ChatMessage {
                        is_user: false,
                        message: format!("[Antigravity BIM Agent]: {msg}"),
                        time: crate::core::web_bridge::unix_timestamp_label(),
                    });
                    self.sessions_dirty = true;
                    return actions;
                }
                Err(_) => {}
            }
        }

        // Collect Antigravity attached context chips + optional selected element context
        let (mut chip_labels, mut step_traces, ctx_markdown) =
            self.build_enriched_context_payload(&build_view.bim_bus.project, members, universal_canvas);

        let selected_extra = self.format_selected_context_summary(
            &build_view.bim_bus.project,
            members,
            universal_canvas,
        );
        let include_selected = selected_extra.is_some()
            && !self
                .attached_contexts
                .contains(&AiContextAttachment::SelectedElement)
            && (trimmed.contains('@')
                || lower.contains("this column")
                || lower.contains("this element")
                || lower.contains("selected"));
        if include_selected {
            if let Some(m) = self.active_context_mention(&build_view.bim_bus.project) {
                chip_labels.push(m);
            }
            step_traces.push("Read @Selected Element".to_string());
        }

        let mut prompt_sections = Vec::new();
        if !ctx_markdown.is_empty() {
            prompt_sections.push(ctx_markdown);
        }
        if include_selected {
            if let Some(ref s) = selected_extra {
                prompt_sections.push(format!("[Active Spatial Selection]: {s}"));
            }
        }
        let enriched_prompt = if prompt_sections.is_empty() {
            trimmed.to_string()
        } else {
            format!("{trimmed}\n\n{}", prompt_sections.join("\n"))
        };

        if self.conversation_title == "EigenAI Spatial Agent"
            || self.conversation_title == "New Spatial Thread"
        {
            let short_title: String = trimmed.chars().take(32).collect();
            self.conversation_title = if trimmed.chars().count() > 32 {
                format!("{short_title}...")
            } else {
                short_title
            };
        }

        // 4. Live Google Gemini REST Client when Gemini model + API Key are configured
        if matches!(
            self.agent_model,
            AgentModelChoice::Gemini25Flash | AgentModelChoice::Gemini25Pro
        ) && !self.gemini_api_key_draft.trim().is_empty()
        {
            let (lat, lon) = universal_canvas.ground_origin_lat_lon;
            let site = compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
            let crit_ctx = Self::format_critical_bim_context(&build_view.bim_bus.project, members);
            let sys_prompt = format!(
                "You are EigenAI inside EigenGraph (Geospatial Earth + Structural BIM/FEM & EigenGate EigenGate platform). \
                 Respond concisely in professional civil & structural engineering English. \
                 Current Earth Site: ({lat:.3} deg, {lon:.3} deg), Elevation={:.1}m, Seismic Ss={:.2}g, Wind={:.1}m/s. \
                 Active BIM elements={}, Slabs={}, FEM members={}. Top members: {crit_ctx}. \
                 To propose workspace actions, you may append bracket directives: \
                 [[FLY:lat,lon,zoom]], [[BIM:portal 3x2 @6m 2floors]], [[RESIZE_COL:id,w_mm,d_mm]], [[RESIZE_BEAM:id,b_mm,h_mm]], [[ADD_COL:x_mm,y_mm,w_mm,d_mm]], [[ADD_STORY]], [[ADD_SLAB]], [[APPLY_SITE_SEISMIC]], [[REPAIR]], [[EIGENGATE_AUDIT]], [[BATCH_PIN_CITY:keyword,city]], [[PIN_ACTIVE:lat,lon,heading]], [[AUDIT_NEIGHBORHOOD]], [[TOGGLE_TOPO]], [[TOGGLE_VECTOR]], [[OPEN_SHEET:Analysis|Scheduling|Budgeting|Sustainability|Safety|Report]].",
                site.elevation_m,
                site.seismic_pga_ss_g,
                site.design_wind_ms,
                build_view.bim_bus.project.element_count(),
                build_view.bim_bus.project.slabs.len(),
                members.len()
            );
            let mut history_turns: Vec<(bool, String)> = ai_assistant
                .chat_history
                .iter()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|m| (m.is_user, m.message.clone()))
                .collect();
            history_turns.push((true, enriched_prompt.clone()));

            let model_id = self.agent_model.gemini_model_id();
            if let Ok(req_id) = crate::core::web_bridge::gemini_send_conversation(
                &self.gemini_api_key_draft,
                model_id,
                &sys_prompt,
                &history_turns,
            ) {
                let user_idx = ai_assistant.chat_history.len();
                ai_assistant.chat_history.push(crate::models::ChatMessage {
                    is_user: true,
                    message: trimmed.to_string(),
                    time: crate::core::web_bridge::unix_timestamp_label(),
                });
                if !chip_labels.is_empty() || !step_traces.is_empty() {
                    self.turn_metadata.push(AntigravityTurnMeta {
                        msg_index: user_idx,
                        mode_label: self.agent_mode.label().to_string(),
                        model_label: self.agent_model.short_label().to_string(),
                        context_chips: chip_labels,
                        trace_steps: step_traces,
                    });
                }
                self.gemini_inflight_req_id = Some(req_id);
                self.command_bar_expanded = true;
                self.context_picker_open = false;
                self.sessions_dirty = true;
                self.push_trace(format!("Querying {}...", self.agent_model.short_label()), true);
                return actions;
            }
        }

        let user_idx = ai_assistant.chat_history.len();
        self.command_bar_expanded = true;
        self.context_picker_open = false;
        ai_assistant.execute_prompt(
            &enriched_prompt,
            self.active_overlay_pillar
                .unwrap_or(NavigationPillar::GeometryAssembly),
            members,
            projects,
            db_components,
        );
        let asst_idx = ai_assistant.chat_history.len().saturating_sub(1);

        if !chip_labels.is_empty() {
            self.turn_metadata.push(AntigravityTurnMeta {
                msg_index: user_idx,
                mode_label: self.agent_mode.label().to_string(),
                model_label: self.agent_model.short_label().to_string(),
                context_chips: chip_labels,
                trace_steps: Vec::new(),
            });
        }

        if asst_idx > user_idx {
            if self.agent_mode == AgentExecutionMode::Planning {
                step_traces.insert(0, "Planning Check".to_string());
            } else {
                step_traces.insert(0, "Fast Execution".to_string());
            }
            if !prompt_sections.is_empty() {
                if let Some(last_msg) = ai_assistant.chat_history.get_mut(asst_idx) {
                    if !last_msg.message.contains("[Attached") && !prompt_sections.is_empty() {
                        last_msg.message = format!(
                            "{}\n\nContext Applied:\n{}",
                            last_msg.message,
                            prompt_sections.join("\n")
                        );
                    }
                }
            }
            self.turn_metadata.push(AntigravityTurnMeta {
                msg_index: asst_idx,
                mode_label: self.agent_mode.label().to_string(),
                model_label: self.agent_model.short_label().to_string(),
                context_chips: Vec::new(),
                trace_steps: step_traces,
            });
        }

        if ai_assistant.pending_bim_nl_command.as_deref() == Some("__repair_critical__") {
            ai_assistant.pending_bim_nl_command = None;
            if self.agent_mode == AgentExecutionMode::Planning {
                self.queue_planning_proposal(
                    "Auto-Repair Critical EigenGate Members",
                    "Planning Mode detected repair request. Click Apply Changes to upgrade low-redundancy / overstressed elements cleanly via undoable BimCommand.",
                    "BimCommand::AutoRepairCritical",
                    vec![PlannedProposalAction::AutoRepair],
                );
            } else {
                actions.run_auto_repair = true;
            }
        } else if let Some(cmd) = ai_assistant.pending_bim_nl_command.take() {
            if self.agent_mode == AgentExecutionMode::Planning {
                self.queue_planning_proposal(
                    "Apply Proposed BIM Synthesis",
                    "Review BIM natural-language synthesis before updating the active workspace.",
                    format!("BIM Command: {cmd}"),
                    vec![PlannedProposalAction::ExecuteBimNl(cmd)],
                );
            } else if let Ok(msg) = build_view.bim_bus.execute_natural_language(&cmd) {
                self.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
                universal_canvas.globe_mode = true;
                universal_canvas.ground_origin_anchored = true;
                actions.sync_fem_and_earth = true;
                actions.fit_camera = true;
                self.push_trace(msg, true);
            }
        } else {
            self.push_trace(
                format!("{} responded", self.agent_model.short_label()),
                true,
            );
        }
        self.sessions_dirty = true;

        actions
    }

    pub(super) fn open_module_overlay(&mut self, pillar: NavigationPillar) {
        self.active_overlay_pillar = Some(pillar);
        self.command_bar_expanded = false;
        self.push_trace(format!("Opened {} sheet", pillar.clean_name()), true);
    }

    pub(super) fn format_selected_context_summary(
        &self,
        project: &BimProject,
        members: &[StructuralMember],
        universal_canvas: &UniversalCanvasState,
    ) -> Option<String> {
        match self.selected_target? {
            SmartSelectionTarget::Column(id) => {
                let col = project.columns.iter().find(|c| c.id == id)?;
                let fem_m = universal_canvas
                    .earth_fem_cache
                    .member_for_bim(BimElementKind::Column, id, members)
                    .or_else(|| {
                        project
                            .fem_id_for_bim_element(BimElementKind::Column, id)
                            .and_then(|fid| members.iter().find(|m| m.id == fid))
                    });
                let dcr = fem_m.map(|m| m.utilization * 100.0).unwrap_or(42.0);
                let eigengate = fem_m.map(|m| m.eigengate).unwrap_or(0.89);
                Some(format!(
                    "Column {} ({:.0}x{:.0}mm, {}), DCR={:.1}%, EigenGate={:.3}",
                    col.name, col.width_mm, col.depth_mm, col.profile_name, dcr, eigengate
                ))
            }
            SmartSelectionTarget::Beam(id) => {
                let r = project.runs.iter().find(|r| r.id == id)?;
                let fem_m = universal_canvas
                    .earth_fem_cache
                    .member_for_bim(BimElementKind::Run, id, members)
                    .or_else(|| {
                        project
                            .fem_id_for_bim_element(BimElementKind::Run, id)
                            .and_then(|fid| members.iter().find(|m| m.id == fid))
                    });
                let dcr = fem_m.map(|m| m.utilization * 100.0).unwrap_or(45.0);
                let eigengate = fem_m.map(|m| m.eigengate).unwrap_or(0.87);
                Some(format!(
                    "Beam {} ({:.0}x{:.0}mm), DCR={:.1}%, EigenGate={:.3}",
                    r.name, r.width_mm, r.depth_mm, dcr, eigengate
                ))
            }
            SmartSelectionTarget::Wall(id) => {
                let w = project.walls.iter().find(|w| w.id == id)?;
                Some(format!(
                    "Wall {} (L={:.0}mm, t={:.0}mm, structural={})",
                    w.name,
                    w.length_mm(),
                    w.thickness_mm,
                    w.is_structural
                ))
            }
            SmartSelectionTarget::Footing(id) => {
                let f = project.footings.iter().find(|f| f.id == id)?;
                Some(format!(
                    "Footing {} ({:.0}x{:.0}x{:.0}mm)",
                    f.name, f.width_mm, f.length_mm, f.thickness_mm
                ))
            }
            SmartSelectionTarget::Slab(id) => {
                let s = project.slabs.iter().find(|s| s.id == id)?;
                Some(format!(
                    "Slab {} (t={:.0}mm, level=#{}, openings={})",
                    s.name, s.thickness_mm, s.level_id, s.openings.len()
                ))
            }
            SmartSelectionTarget::EarthSite { lat, lon } => {
                let site = compute_site_environmental_coupling(lat, lon, &universal_canvas.earth_world);
                Some(format!(
                    "Earth Site ({:.3}°, {:.3}°), Elev {:.0}m, PGA Ss={:.2}g, Wind {:.0}m/s",
                    lat, lon, site.elevation_m, site.seismic_pga_ss_g, site.design_wind_ms
                ))
            }
            SmartSelectionTarget::SketchEntity(id) => Some(format!("CAD Entity #{id}")),
        }
    }

}
