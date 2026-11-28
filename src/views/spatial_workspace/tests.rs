use super::island_ui::BorrowedIslandHost;
use super::*;
use egui::{Color32, Frame, Margin, Vec2};
use crate::icons::AppIcon;
use crate::theme::MacTheme;

    #[test]
    fn test_phase46_slash_commands_cover_all_15_pillars_and_spatial_lenses() {
        let required = [
            "/earth",
            "/iso",
            "/top",
            "/lens dcr",
            "/lens eigengate",
            "/lens moment",
            "/lens 4d",
            "/vault",
            "/agent",
            "/planning",
            "/drafting",
            "/geometry",
            "/analysis",
            "/schedule",
            "/cost",
            "/sustainability",
            "/facility",
            "/safety",
            "/construction",
            "/automation",
            "/prediction",
            "/report",
            "/settings",
            "/classic",
        ];
        for cmd in required {
            assert!(
                SLASH_COMMANDS.iter().any(|s| s.cmd == cmd),
                "Missing required slash command {cmd}"
            );
        }
    }

    #[test]
    fn test_phase46_column_click_popup_and_transactional_resize() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            BimProject::sample_bimcraft_building(),
        );
        let first_col_id = build.bim_bus.project.columns[0].id;
        let fem_id = build
            .bim_bus
            .project
            .fem_id_for_bim_element(BimElementKind::Column, first_col_id)
            .expect("column maps deterministically to FEM element");
        assert!(fem_id >= 1);

        // Simulate 3D pick event
        build.last_spatial_bim_pick = Some((BimElementKind::Column, first_col_id));
        build.spatial_pick_seq = 1;
        spatial.sync_pick_events(&mut canvas, &mut build);
        assert_eq!(
            spatial.selected_target,
            Some(SmartSelectionTarget::Column(first_col_id))
        );
        assert!(spatial
            .active_context_mention(&build.bim_bus.project)
            .unwrap()
            .starts_with('@'));

        // Verify transactional ResizeColumn with Undo
        let orig_w = build.bim_bus.project.columns[0].width_mm;
        build
            .bim_bus
            .execute(BimCommand::ResizeColumn {
                column_id: first_col_id,
                width_mm: 550.0,
                depth_mm: 550.0,
                rotation_deg: 15.0,
                profile_name: "RC 550x550".to_string(),
            })
            .expect("resize column");
        assert!((build.bim_bus.project.columns[0].width_mm - 550.0).abs() < 1e-6);
        build.bim_bus.undo().expect("undo resize");
        assert!((build.bim_bus.project.columns[0].width_mm - orig_w).abs() < 1e-6);
    }

    #[test]
    fn test_phase46_spatial_hud_responsive_zero_overflow() {
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for win_w in [1000.0_f32, 700.0, 390.0] {
            let mut spatial = SpatialWorkspaceState::default();
            let mut canvas = UniversalCanvasState::default();
            canvas.is_frozen = false;
            let mut build = BuildView::default();
            build.bim_bus = crate::core::BimCommandBus::from_project(
                BimProject::sample_bimcraft_building(),
            );
            let col_id = build.bim_bus.project.columns[0].id;
            spatial.selected_target = Some(SmartSelectionTarget::Column(col_id));
            spatial.command_input = "/".to_string();

            let mut ai = AiAssistantView::default();
            let members = vec![StructuralMember::default()];
            let projects = Vec::new();
            let mut db_components = Vec::new();
            let ctx = egui::Context::default();

            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 840.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(theme.bg_canvas))
                        .show(ui, |ui| {
                            canvas.show_spatial_fullscreen(
                                ui,
                                &theme,
                                NavigationPillar::GeometryAssembly,
                                &mut build.bim_bus.project,
                                &members,
                                false,
                            );
                        });
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }

            for r in &spatial.hud_rects {
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "HUD rect {r:?} overflowed viewport width {win_w}"
                );
            }
        }
    }

    #[test]
    fn test_phase46_command_dispatch_fly_lens_overlay_and_classic_toggle() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember::default()];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // 1. /fly jakarta switches to EarthGlobe and anchors site
        let _ = spatial.execute_spatial_input(
            "/fly jakarta",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
        assert!(canvas.ground_origin_anchored);
        assert!(matches!(
            spatial.selected_target,
            Some(SmartSelectionTarget::EarthSite { .. })
        ));

        // 2. /lens dcr keeps Earth 3D active with DCR% heatmap mode
        let _ = spatial.execute_spatial_input(
            "/lens dcr",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(spatial.active_lens, SpatialLens::DcrHeatmap);
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
        assert!(canvas.globe_mode);
        assert_eq!(build.color_mode, "DCR%");
        assert_eq!(
            canvas.earth_bim_author.active_lens,
            crate::views::universal_canvas::EarthBimLens::DcrHeatmap
        );

        // 3. /cost summons CostBudgeting as an on-demand overlay sheet
        let _ = spatial.execute_spatial_input(
            "/cost",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            spatial.active_overlay_pillar,
            Some(NavigationPillar::CostBudgeting)
        );

        // 4. /classic flags switch back to classic 15-pillar sidebar mode
        let classic_act = spatial.execute_spatial_input(
            "/classic",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(classic_act.switch_to_classic_ui);
    }

    #[test]
    fn test_phase47_antigravity_add_context_and_vault_attach() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            BimProject::sample_bimcraft_building(),
        );
        let col_id = build.bim_bus.project.columns[0].id;
        spatial.selected_target = Some(SmartSelectionTarget::Column(col_id));

        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember::default()];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        spatial.attach_context(AiContextAttachment::SelectedElement);
        spatial.attach_context(AiContextAttachment::SelectedElement);
        spatial.attach_context(AiContextAttachment::FemRedundancy);
        spatial.attach_local_file_context(
            "spec_notes.md".to_string(),
            "# ACI 318 Seismic Column Note\nUse confinement hoops @ 100mm.".into(),
        );
        assert_eq!(spatial.attached_contexts.len(), 3);

        let (chips, traces, markdown) = spatial.build_enriched_context_payload(
            &build.bim_bus.project,
            &members,
            &canvas,
        );
        assert_eq!(chips.len(), 3);
        assert!(traces.iter().any(|t| t.contains("Selected Element")));
        assert!(markdown.contains("spec_notes.md"));
        assert!(markdown.contains("FEM & EigenGate"));

        let _ = spatial.execute_spatial_input(
            "audit attached column and spec",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.turn_metadata.len() >= 2);
        assert!(ai.chat_history.last().unwrap().message.contains("Context Applied"));

        spatial.detach_context("domain:fem");
        assert_eq!(spatial.attached_contexts.len(), 2);
    }

    #[test]
    fn test_phase48_dynamic_island_hover_expand_and_responsive_zero_overflow() {
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for win_w in [1280.0_f32, 840.0, 700.0, 390.0] {
            let mut spatial = SpatialWorkspaceState::default();
            let mut canvas = UniversalCanvasState::default();
            canvas.is_frozen = false;
            let mut build = BuildView::default();
            build.bim_bus = crate::core::BimCommandBus::from_project(
                BimProject::sample_bimcraft_building(),
            );
            let mut ai = AiAssistantView::default();
            let members = vec![StructuralMember::default()];
            let projects = Vec::new();
            let mut db_components = Vec::new();
            let ctx = egui::Context::default();

            let mut run_frame = |pointer: Option<egui::Pos2>,
                             spatial: &mut SpatialWorkspaceState,
                             canvas: &mut UniversalCanvasState,
                             build: &mut BuildView,
                             ai: &mut AiAssistantView| {
                let mut input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 840.0))),
                    ..Default::default()
                };
                if let Some(pos) = pointer {
                    input.events.push(egui::Event::PointerMoved(pos));
                }
                let mut out = ctx.run_ui(input, |ui| {
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(theme.bg_canvas))
                        .show(ui, |ui| {
                            canvas.show_spatial_fullscreen(
                                ui,
                                &theme,
                                NavigationPillar::GeometryAssembly,
                                &mut build.bim_bus.project,
                                &members,
                                false,
                            );
                        });
                    let _ = spatial.render_hud_overlays_with_vault(
                        ui.ctx(),
                        &theme,
                        canvas,
                        build,
                        ai,
                        &members,
                        &projects,
                        &mut db_components,
                        &[],
                    );
                });
                out.textures_delta.clear();
            };

            // 1. Pointer away on canvas -> compact idle Dynamic Island pill
            run_frame(
                Some(egui::Pos2::new(100.0, 100.0)),
                &mut spatial,
                &mut canvas,
                &mut build,
                &mut ai,
            );
            assert!(!spatial.island_expanded_prev);
            let idle_rect = spatial.dynamic_island_rect;
            assert!(idle_rect.is_positive());

            // 2. Hover onto Dynamic Island -> automatically expands to show Vault, Context, AI & Draw controls
            run_frame(
                Some(idle_rect.center()),
                &mut spatial,
                &mut canvas,
                &mut build,
                &mut ai,
            );
            assert!(
                spatial.island_expanded_prev,
                "Dynamic Island should expand immediately on cursor hover at width {win_w}"
            );
            let hovered_rect = spatial.dynamic_island_rect;
            assert!(
                hovered_rect.height() >= idle_rect.height(),
                "Expanded Dynamic Island must grow upward cleanly"
            );

            // 3. Pin chat + context + Vault drawers open across multiple frames until full animation width and confirm zero overflow
            spatial.command_bar_expanded = true;
            spatial.context_picker_open = true;
            spatial.vault_drawer_open = true;
            spatial.attach_context(AiContextAttachment::BimGeometry);
            for _ in 0..8 {
                run_frame(
                    Some(hovered_rect.center()),
                    &mut spatial,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                );
            }
            for r in &spatial.hud_rects {
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Dynamic Island HUD rect {r:?} overflowed viewport width {win_w}"
                );
            }

            // 4. Unpin and move cursor back to canvas -> Dynamic Island folds back to compact pill
            spatial.collapse_dynamic_island();
            run_frame(
                Some(egui::Pos2::new(60.0, 60.0)),
                &mut spatial,
                &mut canvas,
                &mut build,
                &mut ai,
            );
            assert!(!spatial.island_expanded_prev);
        }
    }

    #[test]
    fn test_phase48_manual_draw_commands_from_dynamic_island() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();

        let _ = SpatialManualTool::Column.apply(&mut spatial, &mut canvas, &mut build);
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
        assert!(canvas.globe_mode);
        assert_eq!(build.ground_author_tool, Ground3DAuthorTool::Column);
        assert_eq!(
            canvas.earth_bim_author.active_tool,
            crate::views::universal_canvas::EarthBimTool::Column
        );
        assert!(SpatialManualTool::Column.is_active(
            spatial.active_canvas_mode,
            &canvas,
            &build
        ));

        let _ = SpatialManualTool::Line2D.apply(&mut spatial, &mut canvas, &mut build);
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
        assert_eq!(canvas.active_tool, UniversalCanvasTool::Line);
        assert!(canvas.globe_mode);

        let _ = SpatialManualTool::Measure.apply(&mut spatial, &mut canvas, &mut build);
        assert!(canvas.measure_active);

        let _ = SpatialManualTool::Terraform.apply(&mut spatial, &mut canvas, &mut build);
        assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
        assert!(canvas.globe_mode);
        assert!(canvas.active_tool.is_earth_tool());

        let _ = SpatialManualTool::OrbitPan.apply(&mut spatial, &mut canvas, &mut build);
        assert!(SpatialManualTool::OrbitPan.is_active(
            spatial.active_canvas_mode,
            &canvas,
            &build
        ));
    }

    #[test]
    fn test_phase47_antigravity_mode_model_and_new_chat_history() {
        let mut spatial = SpatialWorkspaceState::default();
        assert_eq!(spatial.agent_mode, AgentExecutionMode::Planning);
        assert_eq!(spatial.agent_mode.next(), AgentExecutionMode::Fast);
        assert_eq!(spatial.agent_model.short_label(), "EigenAI Pro");
        assert_eq!(spatial.agent_model.next(), AgentModelChoice::Gemini25Flash);

        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember::default()];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let add_folder_act = spatial.execute_spatial_input(
            "/add-folder",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(add_folder_act.add_vault_folder);

        let ctx_act = spatial.execute_spatial_input(
            "/context",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.context_picker_open);
        assert!(!ctx_act.add_vault_folder);

        let _ = spatial.execute_spatial_input(
            "check redundancy",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(ai.chat_history.len() > 1);
        spatial.start_new_conversation(&mut ai);
        assert_eq!(spatial.conversation_archives.len(), 1);
        assert_eq!(ai.chat_history.len(), 1);
    }

    #[test]
    fn test_phase49_planning_proposal_apply_and_reject() {
        let mut spatial = SpatialWorkspaceState::default();
        assert_eq!(spatial.agent_mode, AgentExecutionMode::Planning);
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // In Planning mode, "build portal" queues a proposal card instead of mutating BIM immediately
        let _ = spatial.execute_spatial_input(
            "build portal 3x2 @6m 2floors",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(spatial.pending_proposals.len(), 1);
        assert_eq!(spatial.pending_proposals[0].status, ProposalStatus::Pending);
        assert!(build.bim_bus.project.is_empty());

        let prop_id = spatial.pending_proposals[0].id;
        let apply_act = spatial.apply_proposal_by_id(prop_id, &mut canvas, &mut build, &mut ai);
        assert_eq!(spatial.pending_proposals[0].status, ProposalStatus::Applied);
        assert!(!build.bim_bus.project.is_empty());
        assert!(apply_act.sync_fem_and_earth);
        assert!(apply_act.fit_camera);

        // Second proposal rejected leaves BIM count unchanged
        let before_count = build.bim_bus.project.element_count();
        let id2 = spatial.queue_planning_proposal(
            "Add Repair",
            "Test rationale",
            "BimCommand::Repair",
            vec![PlannedProposalAction::AutoRepair],
        );
        spatial.reject_proposal_by_id(id2, &mut ai);
        assert_eq!(spatial.pending_proposals[1].status, ProposalStatus::Rejected);
        assert_eq!(build.bim_bus.project.element_count(), before_count);
    }

    #[test]
    fn test_phase49_gemini_response_and_directive_parsing() {
        let raw = "Recommended 2-bay ductile moment frame aligned to seismic zone. [[FLY:-6.2088,106.8456,15.5]] [[BIM:portal 2x2 @6m 2floors]] [[EIGENGATE_AUDIT]]";
        let (clean, dirs) = parse_gemini_action_directives(raw);
        assert_eq!(
            clean,
            "Recommended 2-bay ductile moment frame aligned to seismic zone."
        );
        assert_eq!(dirs.len(), 3);
        assert_eq!(
            dirs[0],
            GeminiDirective::Fly {
                lat: -6.2088,
                lon: 106.8456,
                zoom: 15.5
            }
        );
        assert_eq!(
            dirs[1],
            GeminiDirective::Bim("portal 2x2 @6m 2floors".to_string())
        );
        assert_eq!(dirs[2], GeminiDirective::EigenGateAudit);

        let api_json = r#"{
            "candidates": [{
                "content": {
                    "parts": [{"text": "Column C1 utilization is 48%."}]
                }
            }]
        }"#;
        let parsed = crate::core::web_bridge::parse_gemini_generate_content_response(api_json)
            .expect("Gemini candidate should parse");
        assert!(parsed.contains("48%"));
    }

    #[test]
    fn test_phase49_obsidian_vault_sessions_roundtrip_and_earth_build_here() {
        let temp_vault = std::env::temp_dir().join(format!(
            "eigengraph_phase49_vault_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_vault);
        let mut spatial = SpatialWorkspaceState::default();
        spatial.conversation_title = "Jakarta Site Audit".to_string();
        let mut ai = AiAssistantView::default();
        ai.chat_history.push(crate::models::ChatMessage {
            is_user: true,
            message: "Audit Jakarta seismic columns".to_string(),
            time: "Now".to_string(),
        });
        let json = spatial.export_sessions_json(&ai);
        let saved_path = crate::core::InternalVault::save_ai_sessions_json_at(&temp_vault, &json)
            .expect("AI sessions JSON should save to AI_Context/");
        assert!(saved_path.ends_with("AI_Context/dynamic_island_sessions.json"));

        let loaded_json =
            crate::core::InternalVault::load_ai_sessions_json_at(&temp_vault).expect("Should reload");
        let mut spatial_restored = SpatialWorkspaceState::default();
        let mut ai_restored = AiAssistantView::default();
        assert!(spatial_restored.restore_sessions_json(&loaded_json, &mut ai_restored));
        assert_eq!(spatial_restored.conversation_title, "Jakarta Site Audit");
        assert_eq!(ai_restored.chat_history.len(), ai.chat_history.len());
        let _ = std::fs::remove_dir_all(&temp_vault);
    }

    #[test]
    fn test_phase50_dynamic_island_no_top_left_capsule_and_zero_overflow() {
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for win_w in [1100.0_f32, 390.0] {
            let mut spatial = SpatialWorkspaceState::default();
            spatial.command_bar_expanded = true;
            spatial.api_key_popover_open = true;
            let mut canvas = UniversalCanvasState::default();
            let mut build = BuildView::default();
            let mut ai = AiAssistantView::default();
            let members = Vec::new();
            let projects = Vec::new();
            let mut db_components = Vec::new();
            let ctx = egui::Context::default();

            for _ in 0..6 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 800.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }

            // Top-left corner has NO capsule in Clean + Orbit mode; only Dynamic Island rect is emitted
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0 && r.bottom() <= 801.0,
                "Dynamic Island {r:?} overflowed viewport width {win_w}"
            );
        }
    }

    #[test]
    fn test_phase50_gemini_multiturn_and_extended_directives() {
        let payload = crate::core::web_bridge::build_gemini_generate_content_payload(
            "System context",
            &[
                (true, "Audit column 1".to_string()),
                (false, "Column 1 DCR is 88%".to_string()),
                (true, "Resize column 1 to 500x500".to_string()),
            ],
        );
        let contents = payload["contents"].as_array().expect("contents array");
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[2]["role"], "user");

        let raw = "Upsizing column C1 to 500x500 mm and adding seismic sync. [[RESIZE_COL:1,500,500]] [[ADD_COL:6000,3000,450,450]] [[APPLY_SITE_SEISMIC]]";
        let (clean, dirs) = parse_gemini_action_directives(raw);
        assert_eq!(
            clean,
            "Upsizing column C1 to 500x500 mm and adding seismic sync."
        );
        assert_eq!(dirs.len(), 3);
        assert_eq!(
            dirs[0],
            GeminiDirective::ResizeColumn {
                id: 1,
                w_mm: 500.0,
                d_mm: 500.0
            }
        );
        assert_eq!(
            dirs[1],
            GeminiDirective::AddColumn {
                x_mm: 6000.0,
                y_mm: 3000.0,
                w_mm: 450.0,
                d_mm: 450.0
            }
        );
        assert_eq!(dirs[2], GeminiDirective::ApplySiteSeismic);
    }

    #[test]
    fn test_phase50_vault_markdown_note_and_optin_key_safety() {
        let temp_vault = std::env::temp_dir().join(format!(
            "eigengraph_phase50_vault_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_vault);
        let rel = crate::core::InternalVault::save_markdown_note_at(
            &temp_vault,
            "Seismic Criteria",
            "# Jakarta Zone\n- Ss = 0.85 g\n",
        )
        .expect("Markdown note should save inside KnowledgeBase/");
        assert_eq!(rel, "KnowledgeBase/Seismic_Criteria.md");
        let saved_md = std::fs::read_to_string(temp_vault.join(&rel)).expect("Note readable");
        assert!(saved_md.contains("Jakarta Zone"));
        let _ = std::fs::remove_dir_all(&temp_vault);

        let key_path = std::env::temp_dir().join(format!(
            "eigengraph_phase50_optin_{}.json",
            std::process::id()
        ));
        crate::core::web_bridge::gemini_key_save_device_opt_in_at(
            &key_path,
            true,
            "AIzaSecretLocalOnly",
        );
        let (rem, loaded) = crate::core::web_bridge::gemini_key_load_device_opt_in_at(&key_path);
        assert!(rem);
        assert_eq!(loaded, "AIzaSecretLocalOnly");

        // Opt-out deletes file cleanly
        crate::core::web_bridge::gemini_key_save_device_opt_in_at(&key_path, false, "");
        let (rem2, loaded2) = crate::core::web_bridge::gemini_key_load_device_opt_in_at(&key_path);
        assert!(!rem2);
        assert!(loaded2.is_empty());

        // Verify UnifiedWorkspaceArchive / WorkspaceCanvasSettings never serialize any API key
        let mut settings = crate::views::settings::SettingsView::default();
        settings.gemini_api_key = "AIzaSecretLocalOnly".to_string();
        let canvas = UniversalCanvasState::default();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ws = settings.workspace_settings_snapshot(&theme, &canvas, false);
        let ws_json = serde_json::to_string(&ws).unwrap_or_default();
        assert!(!ws_json.contains("gemini_api_key"));
        assert!(!ws_json.contains("AIzaSecretLocalOnly"));
    }

    #[test]
    fn test_phase51_gemini_model_ids_undo_redo_and_hover_deck_expansion() {
        assert_eq!(
            AgentModelChoice::EigenAiProFem.gemini_model_id(),
            ""
        );
        assert_eq!(
            AgentModelChoice::Gemini25Flash.gemini_model_id(),
            "gemini-2.5-flash"
        );
        assert_eq!(
            AgentModelChoice::Gemini25Pro.gemini_model_id(),
            "gemini-2.5-pro"
        );
        assert_eq!(AgentModelChoice::Gemini25Flash.next(), AgentModelChoice::Gemini25Pro);
        assert_eq!(AgentModelChoice::Gemini25Pro.next(), AgentModelChoice::EigenAiProFem);

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();

        // Resting screenshot state renders 3-row dock at bottom-center without hover extension
        for _ in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1100.0, 820.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let _ = spatial.render_hud_overlays(
                    ui.ctx(),
                    &theme,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                    &members,
                    &projects,
                    &mut db_components,
                );
            });
            out.textures_delta.clear();
        }
        assert!(!spatial.island_expanded_prev);
        let rest_rect = spatial.dynamic_island_rect;

        // Hover over resting Dynamic Island -> expands upward to expose Hover Extension Deck + Extended Draw
        let mut hover_rect = Rect::NOTHING;
        for _ in 0..6 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1100.0, 820.0))),
                events: vec![egui::Event::PointerMoved(rest_rect.center())],
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let _ = spatial.render_hud_overlays(
                    ui.ctx(),
                    &theme,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                    &members,
                    &projects,
                    &mut db_components,
                );
            });
            out.textures_delta.clear();
            hover_rect = spatial.dynamic_island_rect;
        }
        assert!(spatial.island_expanded_prev);
        assert!(hover_rect.height() > rest_rect.height());
    }

    #[test]
    fn test_phase52_single_dynamic_island_frame_and_mobile_toggle() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();

        // Mobile viewport (< 640px): starts compact, expands smoothly in ONE single outer capsule when `mobile_deck_pinned` is toggled
        let mut rest_mobile = Rect::NOTHING;
        for _ in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(390.0, 844.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let _ = spatial.render_hud_overlays(
                    ui.ctx(),
                    &theme,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                    &members,
                    &projects,
                    &mut db_components,
                );
            });
            out.textures_delta.clear();
            rest_mobile = spatial.dynamic_island_rect;
        }
        assert!(!spatial.island_expanded_prev);
        assert!(rest_mobile.width() <= 386.0);

        spatial.mobile_deck_pinned = true;
        assert!(spatial.is_dynamic_island_expanded(&ctx));
        let mut exp_mobile = Rect::NOTHING;
        for _ in 0..5 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(390.0, 844.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let _ = spatial.render_hud_overlays(
                    ui.ctx(),
                    &theme,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                    &members,
                    &projects,
                    &mut db_components,
                );
            });
            out.textures_delta.clear();
            exp_mobile = spatial.dynamic_island_rect;
        }
        assert!(spatial.island_expanded_prev);
        assert!(exp_mobile.height() > rest_mobile.height());
        assert!(exp_mobile.width() <= 386.0);

        spatial.collapse_dynamic_island();
        assert!(!spatial.mobile_deck_pinned);
    }

    #[test]
    fn test_phase52_sketch_entity_edit_delete_and_open_sheet_directive() {
        let mut canvas = UniversalCanvasState::default();
        let initial_cnt = canvas.entities.len();
        let line_id = canvas.commit_parametric_2d_line(6.0, 0.0, "2D", "Drafting");
        assert_eq!(canvas.entities.len(), initial_cnt + 1);
        let (_, len_m, _, _, _, _, _) = canvas.entity_summary(line_id).expect("line summary");
        assert!((len_m - 6.0).abs() < 1e-6);

        let rect_id = canvas.commit_parametric_2d_rect(8.0, 5.0, "2D", "Drafting");
        let (_, _, area_m2, _, _, _, _) = canvas.entity_summary(rect_id).expect("rect summary");
        assert!((area_m2 - 40.0).abs() < 1e-6);

        let circ_id = canvas.commit_parametric_2d_circle(3.0, "2D", "Drafting");
        let (_, _, _, rad_m, _, _, _) = canvas.entity_summary(circ_id).expect("circle summary");
        assert!((rad_m - 3.0).abs() < 1e-6);

        assert!(canvas.update_entity_style(line_id, [16, 185, 129], 4.5));
        let (_, _, _, _, _, col, sw) = canvas.entity_summary(line_id).unwrap();
        assert_eq!(col, [16, 185, 129]);
        assert!((sw - 4.5).abs() < 1e-4);

        let dup_id = canvas.duplicate_offset_entity(line_id, 500.0, 500.0).expect("duplicate");
        assert_ne!(dup_id, line_id);
        assert!(canvas.delete_entity_by_id(dup_id));
        assert!(canvas.entity_summary(dup_id).is_none());
        canvas.undo();
        assert!(canvas.entity_summary(dup_id).is_some());

        let (clean, directives) = parse_gemini_action_directives(
            "Open the phased schedule and FEM checks. [[OPEN_SHEET:Scheduling]] [[OPEN_SHEET:Analysis]]",
        );
        assert_eq!(clean, "Open the phased schedule and FEM checks.");
        assert_eq!(
            directives,
            vec![
                GeminiDirective::OpenSheet(NavigationPillar::TimeScheduling),
                GeminiDirective::OpenSheet(NavigationPillar::AnalysisSimulation),
            ]
        );

        let mut spatial = SpatialWorkspaceState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let _ = spatial.apply_proposal_actions(
            &[PlannedProposalAction::OpenSheet(NavigationPillar::TimeScheduling)],
            &mut canvas,
            &mut build,
            &mut ai,
        );
        assert_eq!(spatial.active_overlay_pillar, Some(NavigationPillar::TimeScheduling));
    }

    #[test]
    fn test_phase52_gemini_pro_payload_and_escape_reset() {
        let history = vec![(true, "Inspect buckling safety factor.".to_string())];
        let pro_payload = crate::core::web_bridge::build_gemini_generate_content_payload_for_model(
            "Sys",
            &history,
            "gemini-2.5-pro",
        );
        assert_eq!(pro_payload["generationConfig"]["maxOutputTokens"], 4096);

        let flash_payload =
            crate::core::web_bridge::build_gemini_generate_content_payload_for_model(
                "Sys",
                &history,
                "gemini-2.5-flash",
            );
        assert_eq!(flash_payload["generationConfig"]["maxOutputTokens"], 2048);

        let mut spatial = SpatialWorkspaceState::default();
        let mut build = BuildView::default();
        let mut canvas = UniversalCanvasState::default();
        assert!(build.show_3d_map_backdrop);

        build.ground_author_tool = Ground3DAuthorTool::Beam;
        build.pending_3d_start_mm = Some([1000.0, 2000.0]);
        canvas.active_tool = UniversalCanvasTool::Line;
        canvas.measure_active = true;
        canvas.measure_points_lat_lon.push((-6.2, 106.8));

        assert!(spatial.has_active_manual_tool(&build, &canvas));
        spatial.reset_manual_tools_to_orbit(&mut build, &mut canvas);
        assert!(!spatial.has_active_manual_tool(&build, &canvas));
        assert_eq!(build.ground_author_tool, Ground3DAuthorTool::Orbit);
        assert!(build.pending_3d_start_mm.is_none());
        assert_eq!(canvas.active_tool, UniversalCanvasTool::SelectPan);
        assert!(!canvas.measure_active);
        canvas.clear_measure();
        assert!(canvas.measure_points_lat_lon.is_empty());
    }

    #[test]
    fn test_phase53_earth_site_in_dynamic_island_no_top_right_popup_and_zero_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        spatial.selected_target = Some(SmartSelectionTarget::EarthSite {
            lat: -6.2088,
            lon: 106.8456,
        });
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);

        for win_w in [1100.0_f32, 390.0] {
            let ctx = egui::Context::default();
            for _ in 0..5 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            // EarthSite expands inside the ONE Dynamic Island (NO separate top-right Smart Pop-Up rect)
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(spatial.is_dynamic_island_expanded(&ctx));
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Dynamic Island with Earth Site Location {r:?} overflowed viewport {win_w}"
            );
        }

        // Collapsing dismisses EarthSite smoothly back to idle pill
        spatial.collapse_dynamic_island();
        assert!(spatial.selected_target.is_none());
    }

    #[test]
    fn test_phase53_lifo_undo_story_slab_camera_and_measure_summary() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut build = BuildView::default();
        let mut canvas = UniversalCanvasState::default();

        // 1. Chronological LIFO Undo/Redo across BIM 3D & 2D Canvas
        spatial.observe_undo_stacks(build.bim_bus.undo_stack.len(), canvas.undo_stack.len());
        let _ = build
            .bim_bus
            .execute_natural_language("portal 2x1 @6m 1floors")
            .unwrap();
        spatial.observe_undo_stacks(build.bim_bus.undo_stack.len(), canvas.undo_stack.len());
        let eid = canvas.commit_parametric_2d_line(6.0, 0.0, "2D", "Drafting");
        spatial.observe_undo_stacks(build.bim_bus.undo_stack.len(), canvas.undo_stack.len());

        assert_eq!(
            spatial.pop_undo_domain(build.bim_bus.undo_stack.len(), canvas.undo_stack.len()),
            Some(SpatialUndoDomain::Canvas2D)
        );
        assert!(canvas.undo());
        assert!(canvas.entity_summary(eid).is_none());
        assert_eq!(
            spatial.pop_undo_domain(build.bim_bus.undo_stack.len(), canvas.undo_stack.len()),
            Some(SpatialUndoDomain::Bim3D)
        );

        // 2. Story increment, Slab authoring, Slab selection mention, & Camera presets
        assert!(spatial.add_story_above_top(&mut build).is_ok());
        assert!(spatial.add_slab_at_active_story(&mut build).is_ok());
        let slab_id = build.bim_bus.project.slabs.last().unwrap().id;
        spatial.selected_target = Some(SmartSelectionTarget::Slab(slab_id));
        assert!(spatial
            .active_context_mention(&build.bim_bus.project)
            .unwrap()
            .contains("Slab"));

        spatial.apply_camera_preset(&mut build, &mut canvas, "top");
        assert!(!build.perspective);
        assert!((canvas.earth_world.camera.tilt_deg - 0.0).abs() < 1e-3);
        spatial.apply_camera_preset(&mut build, &mut canvas, "iso");
        assert!(build.perspective);
        assert!((canvas.earth_world.camera.tilt_deg - 42.0).abs() < 1e-3);

        // 3. Geodesic measure summary & local action routing
        canvas.measure_active = true;
        canvas.measure_points_lat_lon.push((-6.20, 106.84));
        canvas.measure_points_lat_lon.push((-6.21, 106.85));
        let (dist_m, _, pts_n) = canvas.geodesic_measure_summary();
        assert_eq!(pts_n, 2);
        assert!(dist_m > 1000.0);

        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let _ = spatial.execute_spatial_input(
            "open scheduling",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(
            spatial.active_overlay_pillar == Some(NavigationPillar::TimeScheduling)
                || !spatial.pending_proposals.is_empty()
        );
    }

    #[test]
    fn test_phase53_vault_subfolder_rename_and_delete() {
        let _guard = crate::core::vault::TEST_VAULT_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let temp_vault = std::env::temp_dir().join(format!(
            "eigengraph_phase53_vault_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_vault);
        crate::core::InternalVault::set_runtime_vault_dir(temp_vault.clone());

        let sub_rel = crate::core::InternalVault::create_vault_subfolder_at(&temp_vault, "Site_Data")
            .expect("Subfolder should create starter note");
        assert_eq!(sub_rel, "Site_Data/_index.md");
        assert!(temp_vault.join("Site_Data/_index.md").exists());

        let renamed = crate::core::InternalVault::rename_vault_file_at(
            &temp_vault,
            "Site_Data/_index.md",
            "Site_Data/Geotech_Overview.md",
        )
        .expect("Should rename file cleanly");
        assert_eq!(renamed, "Site_Data/Geotech_Overview.md");
        assert!(temp_vault.join("Site_Data/Geotech_Overview.md").exists());

        crate::core::InternalVault::delete_vault_context_file_at(
            &temp_vault,
            "Site_Data/Geotech_Overview.md",
        )
        .expect("Should delete file cleanly");
        assert!(!temp_vault.join("Site_Data/Geotech_Overview.md").exists());

        crate::core::InternalVault::reset_runtime_vault_dir();
        let _ = std::fs::remove_dir_all(&temp_vault);
    }

    #[test]
    fn test_phase54_restructured_dynamic_island_health_bar_and_mode_filters() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let _ = build
            .bim_bus
            .execute_natural_language("portal 2x1 @6m 1floors")
            .expect("Portal creation should succeed");
        let target_col_id = build.bim_bus.project.columns[0].id;
        let target_fem_id = build
            .bim_bus
            .project
            .fem_id_for_bim_element(crate::core::BimElementKind::Column, target_col_id)
            .expect("Column FEM mapping should exist");
        let mut critical = StructuralMember::default();
        critical.id = target_fem_id;
        critical.name = format!("Col #{target_col_id}");
        critical.utilization = 1.18;
        critical.eigengate = 0.42;
        critical.is_critical = true;
        let members = vec![critical];

        let (worst, max_dcr, min_eigengate, crit_cnt) =
            spatial.structural_health_summary(&build.bim_bus.project, &members);
        assert_eq!(worst, Some(SmartSelectionTarget::Column(target_col_id)));
        assert!((max_dcr - 118.0).abs() < 0.2);
        assert!((min_eigengate - 0.42).abs() < 1e-3);
        assert_eq!(crit_cnt, 1);
        assert!(!spatial.hover_more_tools_open);

        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        spatial.mobile_deck_pinned = true;

        spatial.active_canvas_mode = SpatialCanvasMode::EarthGlobe;
        for win_w in [1100.0_f32, 390.0] {
            let ctx = egui::Context::default();
            for _ in 0..4 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(win_w, 820.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Restructured Dynamic Island {r:?} overflowed viewport {win_w}"
            );
        }

        spatial.hover_more_tools_open = true;
        spatial.collapse_dynamic_island();
        assert!(!spatial.hover_more_tools_open);
        assert!(!spatial.mobile_deck_pinned);
    }

    #[test]
    fn test_phase54_enriched_gemini_ids_openings_and_2d_slab_picking() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let _ = build
            .bim_bus
            .execute_natural_language("portal 2x1 @6m 1floors")
            .expect("Portal creation should succeed");

        let col_id = build.bim_bus.project.columns[0].id;
        let fem_id = build
            .bim_bus
            .project
            .fem_id_for_bim_element(crate::core::BimElementKind::Column, col_id)
            .expect("Mapped FEM ID should exist");
        let mut m = StructuralMember::default();
        m.id = fem_id;
        m.utilization = 1.05;
        m.eigengate = 0.65;
        let members = vec![m];
        let ctx_str = SpatialWorkspaceState::format_critical_bim_context(&build.bim_bus.project, &members);
        assert!(ctx_str.contains(&format!("Column #{col_id}")));
        assert!(ctx_str.contains("DCR=105.0%"));
        assert!(ctx_str.contains("EigenGate=0.650"));

        let (cleaned, dirs) =
            parse_gemini_action_directives("Expand podium [[ADD_STORY]] [[ADD_SLAB]]");
        assert_eq!(cleaned.trim(), "Expand podium");
        assert!(matches!(dirs.as_slice(), [GeminiDirective::AddStory, GeminiDirective::AddSlab]));

        let prev_levels = build.bim_bus.project.levels.len();
        let prev_slabs = build.bim_bus.project.slabs.len();
        let _ = spatial.apply_proposal_actions(
            &[PlannedProposalAction::AddStory, PlannedProposalAction::AddSlab],
            &mut canvas,
            &mut build,
            &mut ai,
        );
        assert!(build.bim_bus.project.levels.len() > prev_levels);
        assert!(build.bim_bus.project.slabs.len() > prev_slabs);

        let base_level_id = build.bim_bus.project.levels[0].id;
        let _ = build
            .bim_bus
            .execute(BimCommand::AddWall {
                base_level_id,
                start: crate::core::P2(0.0, 0.0),
                end: crate::core::P2(5000.0, 0.0),
                thickness_mm: 200.0,
                is_structural: true,
            })
            .expect("Wall creation should succeed");
        let wall_id = build.bim_bus.project.walls.last().unwrap().id;
        let _ = build
            .bim_bus
            .execute(BimCommand::AddOpening {
                wall_id,
                kind: crate::core::OpeningKind::SingleDoor,
                offset_along_mm: 1500.0,
                width_mm: 900.0,
                height_mm: 2100.0,
                sill_height_mm: 0.0,
            })
            .expect("CSG door opening should insert cleanly");
        assert_eq!(
            build
                .bim_bus
                .project
                .walls
                .iter()
                .find(|w| w.id == wall_id)
                .unwrap()
                .openings
                .len(),
            1
        );

        canvas.globe_mode = false;
        canvas.world_map.center_lat_lon = canvas.ground_origin_lat_lon;
        canvas.world_map.zoom_level = 22.0;
        let slab = build.bim_bus.project.slabs.last().unwrap();
        let min_x = slab.boundary.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let min_y = slab.boundary.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let max_x = slab.boundary.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let max_y = slab.boundary.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let interior = [
            min_x + (max_x - min_x) * 0.25,
            min_y + (max_y - min_y) * 0.50,
        ];
        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1000.0, 700.0));
        let screen_pt = canvas.world_to_screen(rect, interior);
        let picked = canvas.pick_bim_element_at_screen(screen_pt, rect, &build.bim_bus.project);
        assert_eq!(picked, Some((crate::core::BimElementKind::Slab, slab.id)));
    }

    #[test]
    fn test_phase54_vault_subfolder_notes_audit_markdown_and_ifc4() {
        let _guard = crate::core::vault::TEST_VAULT_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let temp_vault = std::env::temp_dir().join(format!(
            "eigengraph_phase54_vault_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_vault);
        crate::core::InternalVault::set_runtime_vault_dir(temp_vault.clone());

        let rel = crate::core::InternalVault::save_markdown_note_in_folder_at(
            &temp_vault,
            "Calculations",
            "Seismic Check",
            "Max DCR 82% verified.",
        )
        .expect("Custom folder Markdown note save should succeed");
        assert_eq!(rel, "Calculations/Seismic_Check.md");
        let saved = std::fs::read_to_string(temp_vault.join(&rel))
            .expect("Saved Markdown note should be readable");
        assert!(saved.contains("Max DCR 82% verified."));
        assert!(crate::core::InternalVault::list_vault_subfolders()
            .contains(&"Calculations".to_string()));

        let mut build = BuildView::default();
        let _ = build
            .bim_bus
            .execute_natural_language("portal 2x1 @6m 1floors")
            .expect("Portal creation should succeed");
        let _ = build
            .bim_bus
            .execute_natural_language("slab")
            .ok();
        let mut m = StructuralMember::default();
        m.id = 1;
        m.name = "Column C1".to_string();
        m.utilization = 0.82;
        m.eigengate = 0.91;
        let audit = crate::core::InternalVault::build_engineering_audit_markdown(
            &build.bim_bus.project,
            &[m],
            -6.2088,
            106.8456,
            12.0,
            0.85,
            32.0,
        );
        assert!(audit.contains("# Engineering Structural & Earth Audit"));
        assert!(audit.contains("(-6.2088 deg, 106.8456 deg)"));
        assert!(audit.contains("DCR 82.0%"));
        assert!(audit.contains("EigenGate 0.910"));

        let ifcx = build.bim_bus.project.export_ifcx_spatial();
        assert!(ifcx.contains("\"ifcxVersion\""));
        assert!(ifcx.contains("IfcBuilding"));

        crate::core::InternalVault::reset_runtime_vault_dir();
        let _ = std::fs::remove_dir_all(&temp_vault);
    }

    #[test]
    fn test_phase66_vault_recursive_ifcx_discovery_and_zero_demo_files() {
        let _guard = crate::core::vault::TEST_VAULT_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let temp_vault = std::env::temp_dir().join(format!(
            "eigengraph_phase66_ifcx_vault_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_vault);
        crate::core::InternalVault::set_custom_vault_dir(temp_vault.clone())
            .expect("Setting custom Vault dir should succeed");
        let _ = crate::core::InternalVault::ensure_subdirectories();

        // 1. Verify zero built-in demo/notice/Welcome files were auto-seeded
        assert!(!temp_vault.join("KnowledgeBase/Welcome.md").exists());
        assert!(!temp_vault.join("_VAULT_NOTICE.txt").exists());
        assert!(!temp_vault.join("README_DO_NOT_DELETE.txt").exists());

        // 2. Place a nested uppercase .IFCX file with UTF-8 BOM & native eigengraph::geo::wgs84
        let nested_dir = temp_vault.join("Imports").join("Client_A");
        std::fs::create_dir_all(&nested_dir).unwrap();
        let ifcx_path = nested_dir.join("Jakarta_Tower.IFCX");
        let ai_doc = crate::core::ifcx::IfcxDocument::sample_ai_real_building_capture_ifcx();
        let ifcx_content = format!("\u{FEFF}{}", ai_doc.to_json_pretty());
        std::fs::write(&ifcx_path, &ifcx_content).unwrap();

        let projs = crate::core::InternalVault::list_personal_vault_projects();
        let ifcx_entry = projs
            .iter()
            .find(|p| p.filename == "Jakarta_Tower.IFCX")
            .expect("Nested .IFCX file must be discovered in Vault projects");
        assert_eq!(ifcx_entry.rel_path, "Imports/Client_A/Jakarta_Tower.IFCX");
        assert!(ifcx_entry.name.contains("[IFCX]"));
        assert!(ifcx_entry.member_count >= 4);

        let ctx_files = crate::core::InternalVault::list_vault_context_files(64);
        assert!(ctx_files.iter().any(|f| f.rel_path == "Imports/Client_A/Jakarta_Tower.IFCX"));

        let raw = crate::core::InternalVault::load_personal_vault_raw(ifcx_entry)
            .expect("Loading Vault .IFCX content should succeed");
        let (bim, site_geo, _src, _addr, summary) =
            crate::core::ifcx::IfcxDocument::parse_to_bim_project_with_geo(
                &raw,
                &ifcx_entry.filename,
                &ifcx_entry.rel_path,
            )
            .expect("IFCX file should convert cleanly into editable BimProject");
        assert_eq!(bim.columns.len(), 4);
        assert_eq!(bim.runs.len(), 4);
        assert_eq!(bim.slabs.len(), 1);
        assert!(summary.contains("IFCX"));
        let (lat, lon) = site_geo.expect("eigengraph::geo::wgs84 lat/lon should parse cleanly");
        assert!((lat - (-6.8915)).abs() < 1e-4);
        assert!((lon - 107.6107).abs() < 1e-4);

        crate::core::InternalVault::reset_runtime_vault_dir();
        let _ = std::fs::remove_dir_all(&temp_vault);
    }

    #[test]
    fn test_phase55_earth3d_direct_bim_authoring_picking_and_camera_presets() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        canvas.is_frozen = false;
        canvas.globe_mode = true;

        // Selecting all BIM & Vector draw tools operates directly inside Earth 3D (no Bim3D/Plan2D workspace)
        for tool in [
            SpatialManualTool::Column,
            SpatialManualTool::Beam,
            SpatialManualTool::Wall,
            SpatialManualTool::Footing,
            SpatialManualTool::Line2D,
            SpatialManualTool::Rect2D,
        ] {
            let _ = tool.apply(&mut spatial, &mut canvas, &mut build);
            assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
            assert!(canvas.globe_mode);
        }

        // Camera presets adjust tilted/nadir Earth 3D camera directly
        spatial.apply_camera_preset(&mut build, &mut canvas, "top");
        assert!((canvas.earth_world.camera.tilt_deg - 0.0).abs() < 1e-3);
        spatial.apply_camera_preset(&mut build, &mut canvas, "front");
        assert!((canvas.earth_world.camera.tilt_deg - 64.0).abs() < 1e-3);
        spatial.apply_camera_preset(&mut build, &mut canvas, "iso");
        assert!((canvas.earth_world.camera.tilt_deg - 42.0).abs() < 1e-3);

        // Place a column & test 3D height-aware picking directly on tilted Earth 3D
        let _ = build.bim_bus.execute_natural_language("portal 2x1 @6m 1floors").unwrap();
        let col = &build.bim_bus.project.columns[0];
        canvas.ground_origin_lat_lon = (-6.2088, 106.8456);
        canvas.ground_origin_anchored = true;
        canvas.world_map.center_lat_lon = (-6.2088, 106.8456);
        canvas.world_map.zoom_level = 19.0;
        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1000.0, 700.0));
        let xf = crate::core::gis::GlobeScreenTransform::from_view(
            rect,
            canvas.world_map.center_lat_lon.0,
            canvas.world_map.center_lat_lon.1,
            canvas.world_map.zoom_level,
            &canvas.earth_world.camera,
        );
        let top_ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
            canvas.ground_origin_lat_lon,
            [col.pos.0, col.pos.1],
        );
        let top_m = build.bim_bus.project.level_elevation(col.top_level_id) * 0.001;
        let (top_px, _, _) = xf.project_lat_lon_elev(top_ll.0, top_ll.1, top_m).unwrap();
        canvas.last_globe_xform = Some(xf);

        let picked = canvas.pick_bim_element_at_screen(top_px, rect, &build.bim_bus.project);
        assert_eq!(picked, Some((crate::core::BimElementKind::Column, col.id)));
    }

    #[test]
    fn test_phase55_no_sample_slash_commands_and_zero_hud_overflow() {
        assert!(!SLASH_COMMANDS.iter().any(|s| s.cmd == "/sample" || s.cmd == "/3d" || s.cmd == "/2d"));
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/iso"));
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/top"));

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        spatial.mobile_deck_pinned = true;

        for win_w in [1100.0_f32, 390.0] {
            let ctx = egui::Context::default();
            for _ in 0..4 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(r.right() <= win_w + 1.0 && r.left() >= -1.0);
        }
    }

    #[test]
    fn test_phase56_earth3d_slab_door_window_isolation_and_unlimited_vault_ifc() {
        // 1. Browser WASM Vault bridge has no artificial 64 MB cap and streams via lazyFiles + startFileStream
        let html = include_str!("../../../index.html");
        assert!(!html.contains("64 * 1024 * 1024"), "Artificial 64 MB limit must be removed from index.html");
        assert!(html.contains("startFileStream") && html.contains("lazyFiles"));

        // 2. Slab, Door, and Window authoring tools activate directly on Earth 3D
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        for tool in [
            SpatialManualTool::Slab,
            SpatialManualTool::Door,
            SpatialManualTool::Window,
        ] {
            let _ = tool.apply(&mut spatial, &mut canvas, &mut build);
            assert_eq!(spatial.active_canvas_mode, SpatialCanvasMode::EarthGlobe);
            assert!(tool.is_active(spatial.active_canvas_mode, &canvas, &build));
        }

        // 3. Story Isolation filters picking to only the active level
        let _ = build.bim_bus.execute_natural_language("portal 2x1 @6m 2floors").unwrap();
        let lvl2_id = build.bim_bus.project.levels[1].id;
        let col_l2 = build
            .bim_bus
            .project
            .columns
            .iter()
            .find(|c| c.base_level_id == lvl2_id)
            .expect("Level 2 column")
            .clone();
        let col_l1 = build
            .bim_bus
            .project
            .columns
            .iter()
            .find(|c| c.base_level_id == build.bim_bus.project.levels[0].id && c.top_level_id != lvl2_id + 1)
            .unwrap_or(&build.bim_bus.project.columns[0])
            .clone();

        canvas.ground_origin_lat_lon = (-6.2088, 106.8456);
        canvas.ground_origin_anchored = true;
        canvas.world_map.center_lat_lon = (-6.2088, 106.8456);
        canvas.world_map.zoom_level = 19.0;
        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1000.0, 700.0));
        let xf = crate::core::gis::GlobeScreenTransform::from_view(
            rect,
            canvas.world_map.center_lat_lon.0,
            canvas.world_map.center_lat_lon.1,
            canvas.world_map.zoom_level,
            &canvas.earth_world.camera,
        );
        let l2_ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
            canvas.ground_origin_lat_lon,
            [col_l2.pos.0, col_l2.pos.1],
        );
        let l2_top_m = build.bim_bus.project.level_elevation(col_l2.top_level_id) * 0.001;
        let (l2_px, _, _) = xf.project_lat_lon_elev(l2_ll.0, l2_ll.1, l2_top_m).unwrap();
        canvas.last_globe_xform = Some(xf);

        // Isolate Level 1 -> Level 2-only top column shaft at L2 top cannot be picked when Level 2 isn't active
        canvas.earth_bim_author.base_level_id = build.bim_bus.project.levels[0].id;
        canvas.earth_bim_author.top_level_id = lvl2_id;
        canvas.earth_bim_author.isolate_active_story = true;
        let _ = col_l1.id;
        let picked_isolated_l2 = canvas.pick_bim_element_at_screen(l2_px, rect, &build.bim_bus.project);
        assert_ne!(picked_isolated_l2, Some((crate::core::BimElementKind::Column, col_l2.id)));
    }

    #[test]
    fn test_phase57_multi_building_earth3d_lod_picking_and_place_on_earth_pinning() {
        use crate::core::gis::{GeoPlacementSource, WorldBuildingProxy};
        use crate::views::universal_canvas::WorldPickOutcome;

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);

        // Populate 200 buildings in WorldSpatialIndex across Jakarta + Staging Campus
        for i in 0..200_usize {
            let rel = format!("Projects/Complex/Tower_{i:03}.ifc");
            let is_staging = i < 20;
            canvas.world_index.upsert_building(
                WorldBuildingProxy {
                    id: WorldBuildingProxy::stable_id_for_path(&rel),
                    rel_path: rel,
                    name: format!("Tower_{i:03}"),
                    filename: format!("Tower_{i:03}.ifc"),
                    lat: if is_staging { 0.0 } else { -6.2088 + (i % 10) as f64 * 0.0008 },
                    lon: if is_staging { 0.0 } else { 106.8456 + (i / 10) as f64 * 0.0008 },
                    elev_m: 0.0,
                    heading_deg: 0.0,
                    footprint_local_m: vec![[-12.0, -10.0], [12.0, -10.0], [12.0, 10.0], [-12.0, 10.0]],
                    total_height_m: 28.0,
                    story_count: 8,
                    member_count: 64,
                    max_dcr: 0.62,
                    min_eigengate: 0.88,
                    source: if is_staging {
                        GeoPlacementSource::StagingCampusAuto
                    } else {
                        GeoPlacementSource::FileNativeGeo
                    },
                    address_hint: if is_staging { String::new() } else { "Jakarta".to_string() },
                },
                true,
            );
        }
        assert_eq!(canvas.world_index.buildings.len(), 200);
        assert_eq!(canvas.world_index.unlocated_count(), 20);

        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1100.0, 760.0));

        // 1. LOD 0 orbital cluster picking returns FlyToCluster
        canvas.world_map.center_lat_lon = (-6.2088, 106.8456);
        canvas.world_map.zoom_level = 3.0;
        let xf0 = crate::core::gis::GlobeScreenTransform::from_view(
            rect,
            canvas.world_map.center_lat_lon.0,
            canvas.world_map.center_lat_lon.1,
            canvas.world_map.zoom_level,
            &canvas.earth_world.camera,
        );
        let clusters = canvas.world_index.query_clusters(-85.0, 85.0, -180.0, 180.0, 3.0);
        assert!(!clusters.is_empty());
        let (cl_px, _, _) = xf0.project_lat_lon_elev(clusters[0].lat, clusters[0].lon, 0.0).unwrap();
        canvas.last_globe_xform = Some(xf0);
        assert!(matches!(
            canvas.pick_world_building_or_cluster_at_screen(cl_px, rect),
            Some(WorldPickOutcome::FlyToCluster { .. })
        ));

        // 2. LOD 1/2 city scale: clicking any 3D extruded neighbor massing triggers HydrateBuilding
        let target_b = canvas.world_index.buildings[50].clone();
        canvas.world_map.center_lat_lon = (target_b.lat, target_b.lon);
        canvas.world_map.zoom_level = 16.5;
        let xf1 = crate::core::gis::GlobeScreenTransform::from_view(
            rect,
            canvas.world_map.center_lat_lon.0,
            canvas.world_map.center_lat_lon.1,
            canvas.world_map.zoom_level,
            &canvas.earth_world.camera,
        );
        let (roof_px, _, _) = xf1.project_lat_lon_elev(target_b.lat, target_b.lon, target_b.total_height_m).unwrap();
        canvas.last_globe_xform = Some(xf1);
        assert_eq!(
            canvas.pick_world_building_or_cluster_at_screen(roof_px, rect),
            Some(WorldPickOutcome::HydrateBuilding(target_b.rel_path.clone()))
        );

        // 3. Interactive "Place on Earth" pinning + HUD zero-overflow across Desktop & Mobile
        let staging_rel = canvas.world_index.buildings[0].rel_path.clone();
        canvas.placing_unlocated_rel_path = Some(staging_rel.clone());
        canvas.placing_heading_deg = 45.0;
        spatial.mobile_deck_pinned = true;
        for win_w in [1200.0_f32, 390.0] {
            let ctx = egui::Context::default();
            for _ in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            // Placement bar is integrated inside the single Unified Dynamic Island (Phase 62)
            assert_eq!(spatial.hud_rects.len(), 1);
            for r in &spatial.hud_rects {
                assert!(r.right() <= win_w + 1.0 && r.left() >= -1.0);
            }
        }

        assert!(canvas.world_index.pin_building_at(&staging_rel, -0.9730, 116.7090, 45.0));
        assert_eq!(canvas.world_index.unlocated_count(), 19);
    }

    #[test]
    fn test_phase58_vector_layers_tin_relief_portfolio_triage_and_hud_bounds() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        // 1. Vector theme cycling covers CoastalBlueprint -> CityDaylight -> NightTwin -> Topographic
        assert_eq!(canvas.world_map.visual_layer_label(), "Blueprint");
        canvas.world_map.cycle_basemap_visual_mode();
        assert_eq!(canvas.world_map.visual_layer_label(), "Daylight");
        assert_eq!(
            canvas.world_map.tile_provider,
            crate::core::gis::VectorEarthTheme::CityDaylight
        );

        // 2. Site TIN generation & local terrain relief sampling on Earth 3D
        canvas.ground_origin_lat_lon = (-6.9175, 107.6191);
        canvas.ground_origin_anchored = true;
        canvas.site_tin_preset = crate::core::gis::DemTerrainPreset::VolcanicFoothillsRidge;
        canvas.show_terrain_contours = true;
        canvas.ensure_site_tin_generated();
        assert!(canvas.active_site_tin.is_some());
        assert!(canvas.local_site_relief_m(-6.9175, 107.6191).abs() < 0.05);
        let offset_ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(
            canvas.ground_origin_lat_lon,
            [120_000.0, 90_000.0],
        );
        assert!(canvas.local_site_relief_m(offset_ll.0, offset_ll.1).abs() > 0.1);

        // 3. Portfolio AI Context (@World Portfolio) injects global distribution into Gemini payload
        canvas.world_index.upsert_building(
            crate::core::gis::WorldBuildingProxy {
                id: 1,
                rel_path: "Projects/Bandung/Tower_A.ifc".to_string(),
                name: "Bandung Critical Frame".to_string(),
                filename: "Tower_A.ifc".to_string(),
                lat: -6.9175,
                lon: 107.6191,
                elev_m: 710.0,
                heading_deg: 20.0,
                footprint_local_m: crate::core::gis::WorldBuildingProxy::default_footprint_m(),
                total_height_m: 32.0,
                story_count: 9,
                member_count: 140,
                max_dcr: 1.08,
                min_eigengate: 0.64,
                source: crate::core::gis::GeoPlacementSource::UserPinnedOnEarth,
                address_hint: "Bandung".to_string(),
            },
            true,
        );
        canvas.active_world_project_rel_path = Some("Projects/Bandung/Tower_A.ifc".to_string());
        spatial.attach_context(AiContextAttachment::WorldPortfolio);
        let (chips, _, md) =
            spatial.build_enriched_context_payload(&build.bim_bus.project, &members, &canvas);
        assert!(chips.iter().any(|c| c == "@World Portfolio"));
        assert!(md.contains("Critical Risk=1"));
        assert!(md.contains("Bandung Critical Frame"));

        // 4. Expanded Hover Deck + Vault Filter Drawer fit cleanly inside 1200px Desktop and 390px Mobile
        spatial.mobile_deck_pinned = true;
        spatial.vault_drawer_open = true;
        for win_w in [1200.0_f32, 390.0] {
            let ctx = egui::Context::default();
            for _ in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(r.right() <= win_w + 1.0 && r.left() >= -1.0);
        }
    }

    #[test]
    fn test_phase59_osm_geojson_dem_neighborhood_batch_pin_and_gemini_tools() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        // 1. GeoJSON site parcel & road ingestion + vector snapping
        let geo = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"name":"Access Rd","highway":"secondary"},"geometry":{"type":"LineString","coordinates":[[106.844,-6.2088],[106.847,-6.2088]]}}]}"#;
        let (ingested, _) = canvas.urban_vectors.ingest_geojson(geo, "Jakarta").expect("GeoJSON ok");
        assert_eq!(ingested, 1);

        // 2. Seed unlocated Staging models + nearby tight neighbor for Pounding + Wind + DEM test
        canvas.world_index.upsert_building(
            crate::core::gis::WorldBuildingProxy {
                id: 11,
                rel_path: "Projects/Surabaya/Warehouse_01.ifc".to_string(),
                name: "Warehouse_01".to_string(),
                filename: "Warehouse_01.ifc".to_string(),
                lat: 0.0,
                lon: 0.0,
                elev_m: 0.0,
                heading_deg: 0.0,
                footprint_local_m: vec![[-10.0, -8.0], [10.0, -8.0], [10.0, 8.0], [-10.0, 8.0]],
                total_height_m: 14.0,
                story_count: 4,
                member_count: 40,
                max_dcr: 0.72,
                min_eigengate: 0.85,
                source: crate::core::gis::GeoPlacementSource::StagingCampusAuto,
                address_hint: String::new(),
            },
            true,
        );
        let origin = (-6.2088, 106.8456);
        let nb_ll = crate::core::gis::SlippyMapState::world_mm_to_lat_lon(origin, [16_500.0, 0.0]);
        canvas.world_index.upsert_building(
            crate::core::gis::WorldBuildingProxy {
                id: 12,
                rel_path: "Projects/Jakarta/Neighbor_Tower.ifc".to_string(),
                name: "Neighbor_Tower".to_string(),
                filename: "Neighbor_Tower.ifc".to_string(),
                lat: nb_ll.0,
                lon: nb_ll.1,
                elev_m: 10.0,
                heading_deg: 0.0,
                footprint_local_m: vec![[-10.0, -8.0], [10.0, -8.0], [10.0, 8.0], [-10.0, 8.0]],
                total_height_m: 38.0,
                story_count: 10,
                member_count: 150,
                max_dcr: 0.94,
                min_eigengate: 0.65,
                source: crate::core::gis::GeoPlacementSource::UserPinnedOnEarth,
                address_hint: "Jakarta".to_string(),
            },
            true,
        );
        canvas.ground_origin_lat_lon = origin;
        canvas.ground_origin_anchored = true;
        build.bim_bus.execute_natural_language("portal 2x2 @6m 10floors").expect("build tower");

        // 3. Parse & execute multi-action Gemini directives (Batch Pin + Audit Neighborhood + Topo + Vector)
        let raw_ai = "Plan ready. [[BATCH_PIN_CITY:Surabaya, Surabaya]] [[AUDIT_NEIGHBORHOOD]] [[TOGGLE_TOPO]] [[TOGGLE_VECTOR]]";
        let (_, dirs) = parse_gemini_action_directives(raw_ai);
        assert_eq!(dirs.len(), 4);

        let _ = spatial.apply_proposal_actions(
            &[
                PlannedProposalAction::BatchPinCity {
                    query: "Surabaya".to_string(),
                    city_name: "Surabaya".to_string(),
                },
                PlannedProposalAction::AuditNeighborhood,
                PlannedProposalAction::EnableTerrainContours,
                PlannedProposalAction::SwitchPureVector,
            ],
            &mut canvas,
            &mut build,
            &mut ai,
        );
        assert_eq!(canvas.world_index.unlocated_count(), 0);
        assert!(canvas.neighborhood_report.neighbors_evaluated >= 1);
        assert!(canvas.neighborhood_report.pounding_risk_count >= 1);
        assert!(canvas.show_terrain_contours);
        assert_eq!(
            canvas.world_map.tile_provider,
            crate::core::gis::VectorEarthTheme::CoastalBlueprint
        );

        // 4. Expanded Dynamic Island + Neighborhood Pill + Batch Pin Vault Drawer stay inside bounds on Desktop & Mobile
        spatial.mobile_deck_pinned = true;
        spatial.vault_drawer_open = true;
        for win_w in [1200.0_f32, 390.0] {
            let ctx = egui::Context::default();
            for _ in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert!(!spatial.hud_rects.is_empty());
            for r in &spatial.hud_rects {
                assert!(r.right() <= win_w + 1.0 && r.left() >= -1.0);
            }
        }
    }

    #[test]
    fn test_phase60_revision_hash_sparse_eigengate_section_cut_colorbar_and_rebar_popup() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = crate::views::AiAssistantView::default();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        // 1. Reactive content-revision hash & monotonic BimCommandBus revision across in-place resizes
        build.bim_bus.execute_natural_language("portal 2x1 @6m 2floors").expect("portal");
        let h0 = build.bim_bus.project.content_revision_hash();
        let r0 = build.bim_bus.revision;

        let col_id = build.bim_bus.project.columns[0].id;
        build
            .bim_bus
            .execute(BimCommand::ResizeColumn {
                column_id: col_id,
                width_mm: 550.0,
                depth_mm: 550.0,
                rotation_deg: 15.0,
                profile_name: "RC 550x550".to_string(),
            })
            .expect("resize column");
        let h1 = build.bim_bus.project.content_revision_hash();
        let r1 = build.bim_bus.revision;
        assert_ne!(h1, h0, "Column resize must change content_revision_hash without changing element counts");
        assert_eq!(r1, r0 + 1);

        let run_id = build.bim_bus.project.runs[0].id;
        build
            .bim_bus
            .execute(BimCommand::ResizeRun {
                run_id,
                width_mm: 350.0,
                depth_mm: 650.0,
                profile_name: "RC 350x650".to_string(),
            })
            .expect("resize run");
        let h2 = build.bim_bus.project.content_revision_hash();
        let r2 = build.bim_bus.revision;
        assert_ne!(h2, h1);
        assert_eq!(r2, r1 + 1);

        let base_level_id = build.bim_bus.project.levels[0].id;
        build
            .bim_bus
            .execute(BimCommand::AddWall {
                base_level_id,
                start: crate::core::P2(0.0, 0.0),
                end: crate::core::P2(4000.0, 0.0),
                thickness_mm: 200.0,
                is_structural: true,
            })
            .expect("add wall");
        let wall_id = build.bim_bus.project.walls.last().unwrap().id;
        let h_wall_before = build.bim_bus.project.content_revision_hash();
        build
            .bim_bus
            .execute(BimCommand::ResizeWall {
                wall_id,
                thickness_mm: 300.0,
                is_structural: true,
            })
            .expect("resize wall");
        assert_ne!(build.bim_bus.project.content_revision_hash(), h_wall_before);

        let footing_id = build.bim_bus.project.footings[0].id;
        let h_ft_before = build.bim_bus.project.content_revision_hash();
        build
            .bim_bus
            .execute(BimCommand::ResizeFooting {
                footing_id,
                width_mm: 2000.0,
                length_mm: 2000.0,
                thickness_mm: 550.0,
            })
            .expect("resize footing");
        let h_ft_after = build.bim_bus.project.content_revision_hash();
        assert_ne!(h_ft_after, h_ft_before);

        assert!(build.bim_bus.undo().is_some());
        assert_eq!(build.bim_bus.project.content_revision_hash(), h_ft_before);
        assert!(build.bim_bus.redo().is_some());
        assert_eq!(build.bim_bus.project.content_revision_hash(), h_ft_after);

        // 2. Single-SVD verify_kinematics + sparse 6-DOF Sherman-Morrison EigenGate evaluation
        let fem = build.bim_bus.project.to_structural_model();
        let assessment = crate::core::EigenGateEngine::evaluate_redundancy(&fem).expect("EigenGate evaluation");
        assert!(assessment.kinematic_guard.is_stable);
        assert!(!assessment.member_results.is_empty());
        assert!(assessment.average_eigengate > 0.50);

        let members: Vec<StructuralMember> = assessment
            .member_results
            .iter()
            .map(|r| StructuralMember {
                id: r.member_id,
                name: r.member_name.clone(),
                force_kn: r.force_kn,
                capacity_kn: r.capacity_kn,
                utilization: r.utilization_initial,
                moment_my_knm: (r.force_kn.abs() * 0.18 + r.utilization_initial * 65.0).max(8.0),
                shear_vy_kn: (r.force_kn.abs() * 0.35 + r.utilization_initial * 42.0).max(12.0),
                eigengate: r.eigengate,
                eigengate_score: r.eigengate,
                svd_margin: r.svd_margin,
                is_critical: r.is_critical,
                ..Default::default()
            })
            .collect();

        // 3. `/section 4.5` slash command activates 3D horizontal section cut plane
        let _ = spatial.execute_spatial_input(
            "/section 4.5",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas.earth_bim_author.section_cut_active);
        assert!((canvas.earth_bim_author.section_cut_elev_m - 4.5).abs() < 1e-6);

        // 4. Smart Element Pop-Up (2D Rebar Cross-Section) + Dynamic Island (DCR/Moment Colorbar + Sec Cut) fit viewport bounds
        spatial.mobile_deck_pinned = true;
        for (target, lens) in [
            (SmartSelectionTarget::Column(col_id), SpatialLens::DcrHeatmap),
            (SmartSelectionTarget::Beam(run_id), SpatialLens::BendingMoment),
        ] {
            spatial.selected_target = Some(target);
            spatial.apply_lens_to_views(lens, &mut build, &mut canvas);
            for win_w in [1200.0_f32, 390.0] {
                let ctx = egui::Context::default();
                for _ in 0..3 {
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                        ..Default::default()
                    };
                    let mut out = ctx.run_ui(input, |ui| {
                        let _ = spatial.render_hud_overlays(
                            ui.ctx(),
                            &theme,
                            &mut canvas,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    });
                    out.textures_delta.clear();
                }
                assert!(!spatial.hud_rects.is_empty());
                for r in &spatial.hud_rects {
                    assert!(
                        r.right() <= win_w + 1.0 && r.left() >= -1.0,
                        "Phase 60 HUD rect {r:?} overflowed viewport width {win_w}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_phase61_liquid_glass_ui_segmented_dynamic_island_and_exclusive_drawers() {
        // 1. Frosted Liquid Glass UI translucency across ProDark & EngineeringLight
        for mode in [crate::theme::ThemeMode::ProDark, crate::theme::ThemeMode::EngineeringLight] {
            let t = crate::theme::MacTheme::for_mode(mode);
            let outer = t.glass_capsule_frame(20, Margin::same(12));
            let sub = t.glass_subpill_frame(10, Margin::same(6));
            assert!(
                (210..235).contains(&outer.fill.a()),
                "Outer frosted glass frame should use high-opacity translucent frost (alpha={}) in {mode:?}",
                outer.fill.a()
            );
            assert!(sub.fill.a() < 220, "Glass sub-pill should remain translucent in {mode:?}");
        }

        // 2. Exclusive Drawer Policy (only one popover drawer open at any time)
        let mut spatial = SpatialWorkspaceState::default();
        spatial.toggle_exclusive_drawer("vault");
        assert!(spatial.vault_drawer_open);
        assert!(!spatial.context_picker_open && !spatial.history_popover_open && !spatial.api_key_popover_open);

        spatial.toggle_exclusive_drawer("context");
        assert!(spatial.context_picker_open);
        assert!(!spatial.vault_drawer_open && !spatial.history_popover_open && !spatial.api_key_popover_open);

        spatial.toggle_exclusive_drawer("history");
        assert!(spatial.history_popover_open);
        assert!(!spatial.vault_drawer_open && !spatial.context_picker_open && !spatial.api_key_popover_open);

        spatial.toggle_exclusive_drawer("api_key");
        assert!(spatial.api_key_popover_open);
        assert!(!spatial.vault_drawer_open && !spatial.context_picker_open && !spatial.history_popover_open);

        // Close api_key drawer before cycling Studio Tabs
        spatial.toggle_exclusive_drawer("api_key");
        assert!(!spatial.api_key_popover_open);

        // 3. All 4 Segmented Studio Tabs render within bounds on Desktop (1200px) & Mobile (390px)
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let _ = build.bim_bus.execute_natural_language("portal 2x1 @6m 1floors");
        spatial.mobile_deck_pinned = true;

        for tab in DynamicIslandStudioTab::ALL {
            spatial.studio_tab = tab;
            spatial.hover_more_tools_open = tab == DynamicIslandStudioTab::CadAndExport;
            for win_w in [1200.0_f32, 390.0] {
                let ctx = egui::Context::default();
                for _ in 0..3 {
                    let input = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                        ..Default::default()
                    };
                    let mut out = ctx.run_ui(input, |ui| {
                        let _ = spatial.render_hud_overlays(
                            ui.ctx(),
                            &theme,
                            &mut canvas,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    });
                    out.textures_delta.clear();
                }
                assert!(!spatial.hud_rects.is_empty());
                for r in &spatial.hud_rects {
                    assert!(
                        r.right() <= win_w + 1.0 && r.left() >= -1.0,
                        "Phase 61 HUD rect {r:?} for tab {tab:?} overflowed viewport width {win_w}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_phase62_coastal_glass_palette_integrated_lens_tool_strip_story_clone_and_subsurface_footings() {
        // 1. Coastal Marine Teal Palette (#0B1427, #2B4963, #5B92A5, #D0EBE1) + Higher-Opacity Frosted Glass
        let dark = crate::theme::MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        assert_eq!(dark.bg_canvas, Color32::from_rgb(11, 20, 39));
        assert_eq!(dark.bg_card_hover, Color32::from_rgb(43, 73, 99));
        assert_eq!(dark.accent_blue, Color32::from_rgb(91, 146, 165));
        assert_eq!(dark.text_primary, Color32::from_rgb(208, 235, 225));
        assert_eq!(dark.glass_capsule_frame(20, Margin::same(12)).fill.a(), 222);

        let light = crate::theme::MacTheme::for_mode(crate::theme::ThemeMode::EngineeringLight);
        assert_eq!(light.bg_canvas, Color32::from_rgb(208, 235, 225));
        assert_eq!(light.accent_blue, Color32::from_rgb(43, 73, 99));
        assert_eq!(light.text_primary, Color32::from_rgb(11, 20, 39));
        assert_eq!(light.glass_capsule_frame(20, Margin::same(12)).fill.a(), 228);

        // 2. Active Lens + Manual Tool Strip unified inside Dynamic Island (1 HUD rect, zero overflow on 1200px & 390px)
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        build
            .bim_bus
            .execute_natural_language("portal 2x1 @6m 1floors")
            .expect("portal");
        spatial.active_lens = SpatialLens::Erection4D;
        build.ground_author_tool = Ground3DAuthorTool::Column;
        spatial.mobile_deck_pinned = true;

        for win_w in [1200.0_f32, 390.0] {
            let ctx = egui::Context::default();
            for _ in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &dark,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(
                spatial.hud_rects.len(),
                1,
                "Active lens & manual tool controls must render inside the single Dynamic Island"
            );
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 62 integrated Dynamic Island {r:?} overflowed viewport width {win_w}"
            );
        }

        // 3. 1-Click Story Cloning upward + Auto-Foundation generation + Element Auto-Size
        let cols_before = build.bim_bus.project.columns.len();
        let runs_before = build.bim_bus.project.runs.len();
        let clone_msg = spatial
            .clone_active_story_upward(&mut build)
            .expect("story clone should succeed");
        assert!(clone_msg.contains("Cloned"));
        assert!(build.bim_bus.project.columns.len() >= cols_before * 2);
        assert!(build.bim_bus.project.runs.len() >= runs_before * 2);

        let ft_before = build.bim_bus.project.footings.len();
        let (base_id, top_id, _, _) = spatial.active_base_and_top_levels(&build.bim_bus.project);
        let mut sorted_lvls = build.bim_bus.project.levels.clone();
        sorted_lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
        let ground_id = sorted_lvls.first().map(|l| l.id).unwrap_or(base_id);
        let top_ground_id = sorted_lvls.get(1).map(|l| l.id).unwrap_or(top_id);
        build
            .bim_bus
            .execute(BimCommand::AddColumn {
                base_level_id: ground_id,
                top_level_id: top_ground_id,
                pos: crate::core::P2(24_000.0, 0.0),
                width_mm: 400.0,
                depth_mm: 400.0,
                is_round: false,
                profile_name: "C400x400".to_string(),
            })
            .expect("add un-founded column");

        let ft_msg = spatial
            .auto_generate_missing_footings(&mut build)
            .expect("footing generator should succeed");
        assert!(ft_msg.contains("pile caps"), "Unexpected msg: {ft_msg}");
        assert_eq!(build.bim_bus.project.footings.len(), ft_before + 1);

        let col0_id = build.bim_bus.project.columns[0].id;
        let orig_w = build.bim_bus.project.columns[0].width_mm;
        spatial.selected_target = Some(SmartSelectionTarget::Column(col0_id));
        let _ = spatial
            .auto_size_selected_element(&mut build)
            .expect("auto-size should succeed");
        assert!(build.bim_bus.project.columns[0].width_mm > orig_w);
    }

    #[test]
    fn test_phase63_site_coupled_6dof_fem_modal_and_n1_disruption() {
        let mut build = BuildView::default();
        let _ = build.bim_bus.execute_natural_language("portal 3x2");
        let mut high_site = crate::core::SiteEnvironmentalCoupling::default();
        high_site.seismic_pga_ss_g = 1.25;
        high_site.seismic_pga_s1_g = 0.55;
        high_site.recommended_soil_class = "SD (Stiff Soil)".to_string();

        let mut low_site = high_site.clone();
        low_site.seismic_pga_ss_g = 0.25;
        low_site.seismic_pga_s1_g = 0.10;
        low_site.recommended_soil_class = "SB (Rock)".to_string();

        let (fem_hi, map_hi, load_hi) =
            build.bim_bus.project.to_structural_model_with_site_loads(&high_site);
        let (_, _, load_lo) =
            build.bim_bus.project.to_structural_model_with_site_loads(&low_site);

        assert!(load_hi.total_gravity_kn > 50.0, "Gravity weight should be positive");
        assert!(
            load_hi.seismic_base_shear_kn > load_lo.seismic_base_shear_kn,
            "High-seismic site ({:.1} kN) must produce greater base shear than rock site ({:.1} kN)",
            load_hi.seismic_base_shear_kn,
            load_lo.seismic_base_shear_kn
        );
        assert!(!map_hi.is_empty());

        let fea = crate::core::FemSolver::solve(&fem_hi)
            .expect("Site-coupled 6-DOF 3D frame should solve");
        assert!(fea.max_displacement_mm > 0.01 && fea.max_displacement_mm.is_finite());
        let any_nonzero_moment = fea
            .member_bending_moments
            .iter()
            .any(|&(my, mz)| my.abs().max(mz.abs()) > 10.0);
        assert!(any_nonzero_moment, "6-DOF solver must compute genuine non-zero bending moments");

        // Dynamic Modal Eigenmodes (Modes 1..3)
        let modal = crate::core::ModalEngine::analyze_for_earth_3d(&fem_hi, 3, load_hi.spectrum)
            .expect("Earth 3D modal engine should succeed");
        assert_eq!(modal.modes.len(), 3);
        assert!(modal.modes[0].period_s > 0.01 && modal.modes[0].frequency_hz > 0.05);
        assert_eq!(modal.modes[0].nodal_deformations.len(), fem_hi.nodes.len());

        // Interactive N-1 Single-Member Removal on redundant portal column
        let first_col_fid = map_hi
            .iter()
            .find(|&&(_, k, _)| k == BimElementKind::Column)
            .map(|&(fid, _, _)| fid)
            .expect("column FEM id");
        let n1_rep =
            crate::core::EigenGateEngine::simulate_single_member_removal(&fem_hi, &fea, first_col_fid);
        assert_eq!(n1_rep.removed_fem_id, first_col_fid);
        assert!(n1_rep.post_max_disp_mm >= n1_rep.baseline_max_disp_mm * 0.99);
        assert_eq!(n1_rep.post_nodal_displacements.len(), fem_hi.nodes.len());

        // Determinate single cantilever column removal MUST trigger kinematic mechanism guard
        let mut cant = crate::core::StructuralModel::new("Cantilever");
        cant.add_node(
            crate::core::Node3D::new(0, 0.0, 0.0, 0.0)
                .with_fixity(crate::core::BoundaryFixity::fixed()),
        );
        cant.add_node(crate::core::Node3D::new(1, 0.0, 3.5, 0.0));
        cant.add_element(
            crate::core::Element3D::new(1, "C1", 0, 1)
                .with_type(crate::core::ElementType::Frame3D)
                .with_section(crate::core::CrossSection::rectangular_rc("RC 400x400", 0.4, 0.4, 0.7))
                .with_material(crate::core::Material::reinforced_concrete_fc30()),
        );
        cant.add_load(crate::core::NodalLoad::force_y(1, -100_000.0));
        let cant_base = crate::core::FemSolver::solve(&cant).expect("cantilever solves intact");
        let cant_n1 = crate::core::EigenGateEngine::simulate_single_member_removal(&cant, &cant_base, 1);
        assert!(cant_n1.is_kinematic_mechanism);
        assert_eq!(cant_n1.residual_eigengate, 0.0);
    }

    #[test]
    fn test_phase63_earth_fem_cache_o1_lookup_and_slash_directives() {
        let mut uc = UniversalCanvasState::default();
        uc.earth_fem_cache.bim_to_fem.insert((BimElementKind::Column, 10), 1);
        uc.earth_fem_cache.fem_to_bim.insert(1, (BimElementKind::Column, 10));
        uc.earth_fem_cache.fem_to_member_idx.insert(1, 0);
        uc.earth_fem_cache.node_coords_m = vec![[0.0, 0.0, 0.0], [5.0, 3.5, 2.0]];
        uc.earth_fem_cache.static_nodal_disp_m = vec![[0.0, 0.0, 0.0], [0.010, -0.002, 0.004]];
        uc.earth_fem_cache.max_disp_mm = 11.0;

        let members = vec![StructuralMember {
            id: 1,
            name: "C10".to_string(),
            utilization: 0.72,
            eigengate: 0.84,
            ..Default::default()
        }];
        let hit = uc
            .earth_fem_cache
            .member_for_bim(BimElementKind::Column, 10, &members)
            .expect("O(1) lookup must resolve");
        assert_eq!(hit.id, 1);

        // Verify FEM (x, y_up, z) -> BIM (x_mm, y_mm, elev_m) coordinate mapping
        let ([dx_mm, dy_mm, dz_m], norm) = uc.earth_fem_cache.sample_displacement_at_mm(
            5000.0,
            2000.0,
            3.5,
            crate::views::universal_canvas::EarthBimLens::DeformedShape,
            20.0,
            0,
            0.0,
            false,
        );
        assert!((dx_mm - 200.0).abs() < 1e-6, "Expected dx_mm=200, got {dx_mm}");
        assert!((dy_mm - 80.0).abs() < 1e-6, "Expected dy_mm=80 (FEM dz), got {dy_mm}");
        assert!((dz_m - (-0.04)).abs() < 1e-6, "Expected dz_m=-0.04 (FEM dy_up), got {dz_m}");
        assert!(norm > 0.5);

        // Verify slash commands & Gemini directives
        let mut spatial = SpatialWorkspaceState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let mut db = Vec::new();
        let _ = spatial.execute_spatial_input(
            "/deform 45x",
            &mut uc,
            &mut build,
            &mut ai,
            &members,
            &[],
            &mut db,
        );
        assert_eq!(spatial.active_lens, SpatialLens::DeformedShape);
        assert_eq!(
            uc.earth_bim_author.active_lens,
            crate::views::universal_canvas::EarthBimLens::DeformedShape
        );
        assert!((uc.earth_bim_author.deform_scale - 45.0).abs() < 1e-6);

        let _ = spatial.execute_spatial_input(
            "/modal 2",
            &mut uc,
            &mut build,
            &mut ai,
            &members,
            &[],
            &mut db,
        );
        assert_eq!(spatial.active_lens, SpatialLens::ModalVibration);
        assert_eq!(uc.earth_bim_author.active_modal_mode, 1);

        let n1_act = spatial.execute_spatial_input(
            "/n1",
            &mut uc,
            &mut build,
            &mut ai,
            &members,
            &[],
            &mut db,
        );
        assert_eq!(n1_act.toggle_n1_member, Some(1));

        let restore_act = spatial.execute_spatial_input(
            "/restore",
            &mut uc,
            &mut build,
            &mut ai,
            &members,
            &[],
            &mut db,
        );
        assert!(restore_act.restore_n1_simulation);

        let (clean, dirs) = parse_gemini_action_directives(
            "Inspect sway [[LENS_DEFORM]] [[LENS_MODAL:3]] [[SIMULATE_N1:2]] [[RESTORE_N1]]",
        );
        assert_eq!(clean, "Inspect sway");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::ApplyDeformedLens,
                GeminiDirective::ApplyModalLens(2),
                GeminiDirective::SimulateN1(2),
                GeminiDirective::RestoreN1,
            ]
        );
    }

    #[test]
    fn test_phase64_pure_vector_earth_themes_and_zero_hud_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        // Default startup uses 100% Pure-Rust Vector Earth CoastalBlueprint theme
        assert_eq!(
            canvas.world_map.tile_provider,
            crate::core::gis::VectorEarthTheme::CoastalBlueprint
        );
        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::GisAndSite;

        for vt in crate::core::gis::VectorEarthTheme::ALL {
            canvas.world_map.tile_provider = vt;
            for win_w in [1200.0_f32, 768.0, 390.0] {
                let ctx = egui::Context::default();
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                        ..Default::default()
                    },
                    |ui| {
                        let full = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0));
                        let painter = ui.painter_at(full);
                        let _xf = canvas.earth_world.render_3d_globe(
                            ui,
                            &painter,
                            full,
                            &theme,
                            &canvas.world_map,
                            canvas.ground_origin_lat_lon,
                            canvas.ground_origin_anchored,
                            true,
                            None,
                            canvas.cursor_lat_lon,
                            false,
                        );
                        let _ = spatial.render_hud_overlays(
                            ui.ctx(),
                            &theme,
                            &mut canvas,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    },
                );
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 64 Dynamic Island overflowed viewport {win_w} on theme {vt:?}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn test_phase65_day_dusk_night_sun_presets_and_zero_hud_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        // 1. Slash commands control Day / Dusk / Night solar presets and Pure WebGPU toggle
        for cmd in ["/day", "/dusk", "/night", "/gpu"] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }
        let _ = spatial.execute_spatial_input(
            "/dusk",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(canvas.earth_world.camera.lighting_preset_label(), "Dusk");

        let _ = spatial.execute_spatial_input(
            "/night",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(canvas.earth_world.camera.lighting_preset_label(), "Night");

        let _ = spatial.execute_spatial_input(
            "/day",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(canvas.earth_world.camera.lighting_preset_label(), "Day");

        // 2. Expanded GIS & Site Dynamic Island with Sun + GPU 3D pills fits cleanly on Desktop, Tablet & Mobile
        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::GisAndSite;
        for win_w in [1200.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 820.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 65 GIS & Site Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase67_unified_dynamic_island_all_popups_wide_expansion_and_city_slash_commands() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            crate::core::BimProject::sample_bimcraft_building(),
        );
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        // 1. Slash commands activate IFCX City-Builder tools & auto pad grading
        for cmd in ["/road", "/bridge", "/zone", "/gradepad"] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }
        let _ = spatial.execute_spatial_input(
            "/road",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            canvas.earth_bim_author.active_tool,
            crate::views::universal_canvas::EarthBimTool::Road
        );

        let _ = spatial.execute_spatial_input(
            "/bridge",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            canvas.earth_bim_author.active_tool,
            crate::views::universal_canvas::EarthBimTool::Bridge
        );

        let _ = spatial.execute_spatial_input(
            "/zone",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            canvas.earth_bim_author.active_tool,
            crate::views::universal_canvas::EarthBimTool::Zone
        );

        let _ = spatial.execute_spatial_input(
            "/gradepad",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas.active_site_tin.is_some());
        assert!(!canvas.city_scene.graded_pads.is_empty());

        // 2. Smart Element Inspector renders INSIDE the single Dynamic Island (ZERO detached pop-up windows!)
        let col_id = build.bim_bus.project.columns[0].id;
        spatial.selected_target = Some(SmartSelectionTarget::Column(col_id));
        for win_w in [1200.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(
                spatial.hud_rects.len(),
                1,
                "Smart Inspector must live strictly inside the single Dynamic Island (0 detached popups)"
            );
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 67 Smart Inspector Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }

        // 3. Embedded Settings / Module Sheet panel inside Dynamic Island expands wider on desktop & fits mobile cleanly
        spatial.selected_target = None;
        let compact_w_desktop = {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 840.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            spatial.hud_rects[0].width()
        };

        for win_w in [1280.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut close_flag = false;
            let mut rendered_panel = false;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let mut host = BorrowedIslandHost {
                        theme: &theme,
                        universal_canvas: &mut canvas,
                        build_view: &mut build,
                        ai_assistant: &mut ai,
                        members: &members,
                        projects: &projects,
                        db_components: &mut db_components,
                        vault_projects: &[],
                    };
                    let _ = spatial.render_hud_overlays_with_host(
                        ui.ctx(),
                        &mut host,
                        Some((
                            "Settings — Workspace & About",
                            AppIcon::SettingsGear,
                            &mut close_flag,
                        )),
                        |ui, _| {
                            rendered_panel = true;
                            ui.label("Embedded Settings panel inside Dynamic Island");
                        },
                    );
                },
            );
            out.textures_delta.clear();
            assert!(rendered_panel);
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 67 Embedded Settings Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
            if win_w >= 1200.0 {
                assert!(
                    r.width() > compact_w_desktop + 100.0,
                    "Desktop Dynamic Island should smoothly expand wider when hosting Settings ({} vs compact {})",
                    r.width(),
                    compact_w_desktop
                );
            }
        }
    }

    #[test]
    fn test_phase68_multi_hazard_and_ai_capture_slash_commands_and_zero_hud_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            crate::core::BimProject::sample_bimcraft_building(),
        );
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        for cmd in ["/seismic", "/wind", "/flood", "/hazard", "/ai-capture"] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }

        let act_f = spatial.execute_spatial_input(
            "/flood 3.5",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_f.run_district_hazard_audit);
        assert!(canvas.district_hazard.flood_plane_active);
        assert!((canvas.district_hazard.flood_water_level_m - 3.5).abs() < 1e-6);

        let act_h = spatial.execute_spatial_input(
            "/hazard",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_h.run_district_hazard_audit);
        assert!(canvas.district_hazard.seismic_wave_active);
        assert!(canvas.district_hazard.venturi_wind_active);
        assert!(canvas.district_hazard.pounding_overlay_active);
        canvas.refresh_district_hazard(&build.bim_bus.project);

        let act_ai = spatial.execute_spatial_input(
            "/ai-capture",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ai.ingest_ai_real_building_capture);

        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::GisAndSite;
        for win_w in [1280.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 68 Multi-Hazard Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase69_retrofit_slash_commands_and_zero_hud_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            crate::core::BimProject::sample_bimcraft_building(),
        );
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        for cmd in ["/retrofit", "/brb", "/jacket", "/retrofit-ifcx"] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }

        let act_brb = spatial.execute_spatial_input(
            "/brb",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            act_brb.run_structural_retrofit,
            Some(crate::core::retrofit::RetrofitStrategy::BrbBracingOnly)
        );

        let act_jkt = spatial.execute_spatial_input(
            "/jacket",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            act_jkt.run_structural_retrofit,
            Some(crate::core::retrofit::RetrofitStrategy::RcJacketingOnly)
        );

        let act_ifcx = spatial.execute_spatial_input(
            "/retrofit-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ifcx.export_retrofit_delta_ifcx);

        let site = canvas.active_site_coupling();
        let rep = crate::core::retrofit::synthesize_structural_retrofit(
            &build.bim_bus.project,
            crate::core::retrofit::RetrofitStrategy::AutoBalanced,
            &site,
        )
        .expect("Phase 69 retrofit report");
        canvas.pre_retrofit_baseline = Some(build.bim_bus.project.clone());
        build.bim_bus.project = rep.retrofitted_project.clone();
        canvas.last_retrofit_report = Some(rep);

        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::BimAndStory;
        for win_w in [1280.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 69 Retrofit Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase70_crc_cascade_slash_commands_and_zero_hud_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            crate::core::BimProject::sample_bimcraft_building(),
        );
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        for cmd in ["/cascade", "/crc", "/cascade-ifcx"] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }

        let act_cas = spatial.execute_spatial_input(
            "/cascade 1.75",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(act_cas.run_crc_cascade_screening, Some(1.75));

        let act_crc = spatial.execute_spatial_input(
            "/crc",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_crc.run_crc_cascade_screening.is_some());

        let act_exp = spatial.execute_spatial_input(
            "/cascade-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_exp.export_crc_cascade_ifcx);

        let site = canvas.active_site_coupling();
        let crc_rep = crate::core::cascade::run_two_stage_crc_and_cascade_for_bim(
            &build.bim_bus.project,
            &site,
            None,
            1.50,
        )
        .expect("Phase 70 CRC cascade report");
        canvas.last_crc_cascade_report = Some(crc_rep);
        canvas.active_cascade_step = 1;

        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::ViewAndLens;
        for win_w in [1280.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 70 CRC Cascade Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase71_subsurface_ssi_slash_commands_and_zero_hud_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            crate::core::BimProject::sample_bimcraft_building(),
        );
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();

        for cmd in [
            "/ssi",
            "/borehole",
            "/liquefaction",
            "/subsurface",
            "/ssi-ifcx",
        ] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }

        let act_ssi = spatial.execute_spatial_input(
            "/ssi on",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ssi.run_subsurface_ssi_sync);
        assert!(canvas.subsurface_ssi.ssi_coupling_enabled);

        let act_bh = spatial.execute_spatial_input(
            "/borehole liquefy",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_bh.run_subsurface_ssi_sync);
        assert_eq!(
            canvas.subsurface_ssi.borehole_preset,
            crate::core::geotech::BoreholePreset::CoastalLiquefiableSand
        );

        let act_liq = spatial.execute_spatial_input(
            "/liquefaction on",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_liq.run_subsurface_ssi_sync);
        assert!(canvas.subsurface_ssi.liquefaction_trigger_active);

        let act_sub = spatial.execute_spatial_input(
            "/subsurface on",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_sub.run_subsurface_ssi_sync);
        assert!(canvas.subsurface_ssi.xray_subsurface_visible);

        let act_exp = spatial.execute_spatial_input(
            "/ssi-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_exp.export_subsurface_ssi_ifcx);

        let site = canvas.active_site_coupling();
        let rep = crate::core::geotech::evaluate_coupled_subsurface_ssi_for_bim(
            &build.bim_bus.project,
            &site,
            &canvas.subsurface_ssi,
        )
        .expect("Phase 71 Subsurface SSI report");
        canvas.last_subsurface_ssi_report = Some(rep);

        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::GisAndSite;
        for win_w in [1280.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 71 Subsurface SSI Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase72_ifcx_diff_and_dossier_slash_commands_and_zero_hud_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            crate::core::BimProject::sample_bimcraft_building(),
        );
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::default();

        for cmd in ["/snapshot", "/diff", "/diff-ifcx", "/dossier"] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }

        let act_snap = spatial.execute_spatial_input(
            "/snapshot",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_snap.capture_ifcx_baseline);

        let act_diff = spatial.execute_spatial_input(
            "/diff added",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_diff.run_ifcx_visual_diff);
        assert!(spatial.ifcx_diff_drawer_open);
        assert!(canvas.ifcx_diff_overlay_active);
        assert_eq!(
            canvas.ifcx_diff_filter,
            crate::core::ifcx::IfcxDiffLayerFilter::AddedOnly
        );

        let act_dexp = spatial.execute_spatial_input(
            "/diff-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_dexp.export_ifcx_diff_manifest);

        let act_dos = spatial.execute_spatial_input(
            "/dossier",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_dos.export_vector_dossier_svg);

        // Populate layered diff report with an added SSI overlay to exercise full widened UI
        canvas.capture_ifcx_baseline(&build.bim_bus.project, &members, "Initial");
        canvas.subsurface_ssi.ssi_coupling_enabled = true;
        canvas.refresh_subsurface_ssi(&build.bim_bus.project);
        canvas.refresh_ifcx_visual_diff(&build.bim_bus.project, &members);
        assert!(canvas.last_ifcx_visual_diff.is_some());

        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::CadAndExport;
        for win_w in [1280.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 72 IFCX Diff & Dossier Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase73_portfolio_retrofit_pareto_slash_commands_and_zero_hud_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            crate::core::BimProject::sample_bimcraft_building(),
        );
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::default();

        for cmd in ["/portfolio", "/pareto", "/budget", "/portfolio-ifcx"] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }

        let act_bgt = spatial.execute_spatial_input(
            "/budget 350k",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_bgt.run_portfolio_retrofit);
        assert!((canvas.portfolio_retrofit_config.budget_cap_usd - 350_000.0).abs() < 1.0);
        assert!(spatial.portfolio_drawer_open);
        assert!(canvas.portfolio_retrofit_config.overlay_visible);

        let act_pf = spatial.execute_spatial_input(
            "/portfolio carbon",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_pf.run_portfolio_retrofit);
        assert_eq!(
            canvas.portfolio_retrofit_config.objective,
            crate::core::portfolio_retrofit::PortfolioOptimizationObjective::MinEmbodiedCarbon
        );

        let act_pexp = spatial.execute_spatial_input(
            "/portfolio-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_pexp.export_portfolio_retrofit_ifcx);

        canvas.refresh_portfolio_retrofit(&build.bim_bus.project, &members);
        assert!(canvas.last_portfolio_retrofit_report.is_some());

        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::BimAndStory;
        for win_w in [1280.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 73 Portfolio Pareto Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase74_ground_motion_replay_slash_commands_drawer_and_zero_hud_overflow() {
        use crate::core::ground_motion_replay::{
            run_ground_motion_replay_for_bim, GroundMotionReplayConfig,
        };
        use crate::core::time_history::GroundMotionRecord;

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus = crate::core::BimCommandBus::from_project(
            crate::core::BimProject::sample_bimcraft_building(),
        );
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = MacTheme::default();

        for cmd in ["/replay", "/record", "/hinges", "/replay-ifcx"] {
            assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == cmd));
        }

        let act_rec = spatial.execute_spatial_input(
            "/record sumatra",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_rec.run_async_ground_motion_replay);
        assert_eq!(
            canvas.ground_motion_replay_config.record,
            GroundMotionRecord::SumatraSubduction2004
        );
        assert_eq!(spatial.active_lens, SpatialLens::TimeHistoryReplay);
        assert!(spatial.ground_motion_drawer_open);

        // Exclusive drawer switching must close ground_motion_drawer_open when opening portfolio
        spatial.toggle_exclusive_drawer("portfolio");
        assert!(spatial.portfolio_drawer_open);
        assert!(!spatial.ground_motion_drawer_open);
        spatial.toggle_exclusive_drawer("ground_motion");
        assert!(spatial.ground_motion_drawer_open);
        assert!(!spatial.portfolio_drawer_open);

        let act_rep = spatial.execute_spatial_input(
            "/replay",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_rep.run_async_ground_motion_replay);

        let act_exp = spatial.execute_spatial_input(
            "/replay-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_exp.export_ground_motion_replay_ifcx);

        let site = canvas.active_site_coupling();
        let rep = run_ground_motion_replay_for_bim(
            &build.bim_bus.project,
            &site,
            None,
            &GroundMotionReplayConfig {
                record: GroundMotionRecord::NorthridgeSylmar1994,
                scale_factor: 1.50,
                ..Default::default()
            },
            "UiResponsiveTest",
            None,
        )
        .expect("Ground-motion replay report");
        canvas.last_ground_motion_replay_report = Some(rep);

        let initial_hinges = canvas.ground_motion_replay_config.show_plastic_hinges;
        let _ = spatial.execute_spatial_input(
            "/hinges",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_ne!(
            canvas.ground_motion_replay_config.show_plastic_hinges,
            initial_hinges
        );

        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::ViewAndLens;
        for win_w in [1280.0_f32, 768.0, 390.0] {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                    ..Default::default()
                },
                |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                },
            );
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 74 Ground-Motion Replay Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase75_peer_at2_tridir_fragility_slash_commands_and_zero_hud_overflow() {
        use crate::core::ground_motion_replay::{
            run_ground_motion_replay_for_bim, GroundMotionReplayConfig, TriDirectionalMode,
        };

        for cmd in ["/at2", "/tridir", "/match-sa", "/fragility", "/fragility-ifcx"] {
            assert!(
                SLASH_COMMANDS.iter().any(|s| s.cmd == cmd),
                "Missing Phase 75 slash command {cmd}"
            );
        }

        let (text_no_tags, directives) = parse_gemini_action_directives(
            "Applying tri-directional matched PEER analysis [[TOGGLE_TRIDIR:XYZ]] [[MATCH_SA_SPECTRUM]] [[OPEN_FRAGILITY]]",
        );
        assert_eq!(text_no_tags, "Applying tri-directional matched PEER analysis");
        assert_eq!(directives.len(), 3);
        assert_eq!(
            directives[0],
            GeminiDirective::ToggleTriDirectional(Some(TriDirectionalMode::TriDirectionalXYZ))
        );
        assert_eq!(directives[1], GeminiDirective::ToggleAutoMatchSa);
        assert_eq!(directives[2], GeminiDirective::OpenFragilityTab);

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = crate::views::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();
        let theme = crate::theme::MacTheme::default();
        let _ = build
            .bim_bus
            .execute_natural_language("portal 3x2 @6m 2floors");

        let act_at2 = spatial.execute_spatial_input(
            "/at2 sample",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_at2.run_async_ground_motion_replay);
        assert!(canvas.ground_motion_replay_config.custom_accelerogram.is_some());

        let act_tri = spatial.execute_spatial_input(
            "/tridir xyz",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_tri.run_async_ground_motion_replay);
        assert_eq!(
            canvas.ground_motion_replay_config.tridirectional_mode,
            TriDirectionalMode::TriDirectionalXYZ
        );

        let act_sa = spatial.execute_spatial_input(
            "/match-sa",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_sa.run_async_ground_motion_replay);
        assert!(canvas.ground_motion_replay_config.auto_match_sa_t1);
        assert_eq!(spatial.ground_motion_drawer_tab, 1);

        let _ = spatial.execute_spatial_input(
            "/fragility",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas.ground_motion_replay_config.show_fragility_loss_overlay);
        assert!(spatial.ground_motion_drawer_open);

        let act_ifcx = spatial.execute_spatial_input(
            "/fragility-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ifcx.export_fragility_loss_ifcx);

        let site = canvas.active_site_coupling();
        let rep = run_ground_motion_replay_for_bim(
            &build.bim_bus.project,
            &site,
            None,
            &GroundMotionReplayConfig {
                tridirectional_mode: TriDirectionalMode::TriDirectionalXYZ,
                custom_accelerogram: canvas.ground_motion_replay_config.custom_accelerogram.clone(),
                auto_match_sa_t1: true,
                show_fragility_loss_overlay: true,
                ..Default::default()
            },
            "Phase75HudTest",
            None,
        )
        .expect("Phase 75 replay report");
        canvas.last_ground_motion_replay_report = Some(rep);

        spatial.mobile_deck_pinned = true;
        spatial.studio_tab = DynamicIslandStudioTab::ViewAndLens;
        for tab_idx in [0_usize, 1_usize] {
            spatial.ground_motion_drawer_tab = tab_idx;
            for win_w in [1280.0_f32, 768.0, 390.0] {
                let ctx = egui::Context::default();
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, 844.0))),
                        ..Default::default()
                    },
                    |ui| {
                        let _ = spatial.render_hud_overlays(
                            ui.ctx(),
                            &theme,
                            &mut canvas,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    },
                );
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 75 tab={tab_idx} Dynamic Island overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn test_phase76_shm_digital_twin_atc20_slash_commands_and_zero_hud_overflow() {
        for cmd in ["/shm", "/sensors", "/placard", "/shm-fault", "/shm-ifcx"] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing Phase 76 slash command {cmd} in SLASH_COMMANDS"
            );
        }

        let raw_ai = "Soft-story detected [[OPEN_SHM_TWIN]] [[INJECT_SHM_FAULT:soft_story:85:1]] [[RUN_ATC20_TRIAGE]]";
        let (clean, directives) = parse_gemini_action_directives(raw_ai);
        assert_eq!(clean, "Soft-story detected");
        assert_eq!(directives.len(), 3);
        assert!(matches!(directives[0], GeminiDirective::OpenShmDigitalTwin));
        assert!(matches!(
            directives[1],
            GeminiDirective::InjectShmFault {
                ref scenario,
                severity_pct,
                story
            } if scenario == "soft_story" && (severity_pct - 85.0).abs() < 1e-6 && story == 1
        ));
        assert!(matches!(directives[2], GeminiDirective::RunAtc20Triage));

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members: Vec<StructuralMember> = Vec::new();
        let projects: Vec<crate::models::ProjectItem> = Vec::new();
        let mut db_components: Vec<crate::models::DatabaseComponent> = Vec::new();

        build.bim_bus.project = BimProject::sample_bimcraft_building();

        let act_shm = spatial.execute_spatial_input(
            "/shm",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_shm.run_shm_digital_twin);
        assert!(spatial.shm_drawer_open);
        assert_eq!(spatial.shm_drawer_tab, 0);
        assert_eq!(spatial.active_lens, SpatialLens::ShmDigitalTwin);

        // Idempotent open_shm_drawer on repeated /placard command must NOT collapse drawer
        let act_placard = spatial.execute_spatial_input(
            "/placard",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_placard.run_shm_digital_twin);
        assert!(spatial.shm_drawer_open);
        assert_eq!(spatial.shm_drawer_tab, 1);

        // Exclusive drawer switching: opening ground_motion closes shm_drawer_open
        spatial.toggle_exclusive_drawer("ground_motion");
        assert!(!spatial.shm_drawer_open);
        spatial.open_shm_drawer();
        assert!(spatial.shm_drawer_open);
        assert!(!spatial.ground_motion_drawer_open);

        let act_fault = spatial.execute_spatial_input(
            "/shm-fault soft_story 88 1",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_fault.run_shm_digital_twin);
        assert_eq!(
            canvas.shm_twin_config.fault_scenario,
            crate::core::ShmFaultScenario::SoftStoryFirstFloor
        );
        assert!((canvas.shm_twin_config.severity_pct - 88.0).abs() < 1e-6);

        let act_sample = spatial.execute_spatial_input(
            "/sensors sample",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_sample.run_shm_digital_twin);
        assert!(canvas.shm_twin_config.custom_telemetry.is_some());

        let act_ifcx = spatial.execute_spatial_input(
            "/shm-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ifcx.export_shm_atc20_ifcx);

        canvas
            .run_shm_digital_twin(&build.bim_bus.project)
            .expect("SHM evaluation succeeds");

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0usize, 1usize] {
            spatial.shm_drawer_tab = tab_idx;
            spatial.open_shm_drawer();
            for win_w in [1280.0_f32, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 800.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 76 tab={tab_idx} Dynamic Island overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn test_phase77_evacuation_lifelines_slash_commands_and_zero_hud_overflow() {
        for cmd in ["/evac", "/lifelines", "/safezone", "/evac-ifcx"] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing Phase 77 slash command {cmd} in SLASH_COMMANDS"
            );
        }

        let raw_ai = "Red cordon active [[RUN_EVACUATION_ROUTING]] [[OPEN_EVAC_DRAWER]]";
        let (clean, directives) = parse_gemini_action_directives(raw_ai);
        assert_eq!(clean, "Red cordon active");
        assert_eq!(directives.len(), 2);
        assert!(matches!(directives[0], GeminiDirective::RunEvacuationRouting));
        assert!(matches!(directives[1], GeminiDirective::OpenEvacuationDrawer));

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members: Vec<StructuralMember> = Vec::new();
        let projects: Vec<crate::models::ProjectItem> = Vec::new();
        let mut db_components: Vec<crate::models::DatabaseComponent> = Vec::new();

        build.bim_bus.project = BimProject::sample_bimcraft_building();

        let act_evac = spatial.execute_spatial_input(
            "/evac",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_evac.run_evacuation_routing);
        assert!(spatial.evacuation_drawer_open);
        assert!(canvas.evacuation_config.overlay_visible);

        let act_life = spatial.execute_spatial_input(
            "/lifelines",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_life.run_evacuation_routing);
        assert!(spatial.evacuation_drawer_open);

        spatial.toggle_exclusive_drawer("shm");
        assert!(!spatial.evacuation_drawer_open);
        spatial.open_evacuation_drawer();
        assert!(spatial.evacuation_drawer_open);
        assert!(!spatial.shm_drawer_open);

        let act_safe = spatial.execute_spatial_input(
            "/safezone",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_safe.run_evacuation_routing);
        assert!(canvas
            .city_scene
            .zones
            .iter()
            .any(|z| z.kind == crate::core::IfcxZoneKind::Park));

        let act_ifcx = spatial.execute_spatial_input(
            "/evac-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ifcx.export_evacuation_ifcx);

        canvas
            .run_evacuation_lifeline_routing(&build.bim_bus.project)
            .expect("Evacuation routing succeeds");

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        spatial.open_evacuation_drawer();
        for win_w in [1280.0_f32, 768.0, 390.0] {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(win_w, 800.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                let _ = spatial.render_hud_overlays(
                    ui.ctx(),
                    &theme,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                    &members,
                    &projects,
                    &mut db_components,
                );
            });
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 77 Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase78_watchdog_slash_commands_and_width_first_dynamic_island() {
        for cmd in ["/watchdog", "/ws", "/ws-stop", "/ws-sim", "/watchdog-ifcx"] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing Phase 78 slash command {cmd} in SLASH_COMMANDS"
            );
        }

        let raw_ai = "Watchdog armed [[START_SHM_WATCHDOG]] [[CONNECT_SHM_WS:wss://shm.eigengraph.net/ws]] [[TRIGGER_WATCHDOG_SIM:aftershock]]";
        let (clean, directives) = parse_gemini_action_directives(raw_ai);
        assert_eq!(clean, "Watchdog armed");
        assert_eq!(directives.len(), 3);
        assert!(matches!(directives[0], GeminiDirective::StartShmWatchdog));
        assert!(matches!(directives[1], GeminiDirective::ConnectShmWs(ref u) if u == "wss://shm.eigengraph.net/ws"));
        assert!(matches!(directives[2], GeminiDirective::TriggerWatchdogSim(ref m) if m == "aftershock"));

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members: Vec<StructuralMember> = Vec::new();
        let projects: Vec<crate::models::ProjectItem> = Vec::new();
        let mut db_components: Vec<crate::models::DatabaseComponent> = Vec::new();

        build.bim_bus.project = BimProject::sample_bimcraft_building();

        let _ = spatial.execute_spatial_input(
            "/ws-sim aftershock",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.shm_drawer_open);
        assert_eq!(spatial.shm_drawer_tab, 2);
        assert_eq!(
            canvas.shm_watchdog.config.sim_mode,
            crate::core::WatchdogSimMode::AftershockPulse
        );
        for _ in 0..85 {
            let _ = canvas.tick_live_shm_watchdog(0.05, &build.bim_bus.project);
        }

        let act_ifcx = spatial.execute_spatial_input(
            "/watchdog-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ifcx.export_watchdog_ifcx);

        // Width-first Dynamic Island verification:
        // On a 1600px wide screen with Studio Deck expanded, Dynamic Island expands wide (> 1200px)
        // before stacking vertically; on narrow viewports (768px / 390px) it fits with zero overflow.
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        spatial.shm_drawer_open = false;
        spatial.mobile_deck_pinned = true;
        let raw_wide = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(1600.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out_wide = ctx.run_ui(raw_wide, |ui| {
            let _ = spatial.render_hud_overlays(
                ui.ctx(),
                &theme,
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            );
        });
        out_wide.textures_delta.clear();
        assert_eq!(spatial.hud_rects.len(), 1);
        assert!(
            spatial.hud_rects[0].width() > 1200.0
                && spatial.hud_rects[0].right() <= 1601.0
                && spatial.hud_rects[0].left() >= -1.0,
            "Wide 1600px screen Studio Deck should expand horizontally first without overflow (got rect {:?})",
            spatial.hud_rects[0]
        );

        spatial.mobile_deck_pinned = false;
        spatial.shm_drawer_tab = 2;
        spatial.open_shm_drawer();
        for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(win_w, 800.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                let _ = spatial.render_hud_overlays(
                    ui.ctx(),
                    &theme,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                    &members,
                    &projects,
                    &mut db_components,
                );
            });
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 78 Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase79_gpu_compute_slash_commands_and_width_first_zero_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus.project = BimProject::sample_bimcraft_building();
        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember {
            id: 1,
            name: "C1".to_string(),
            section: "RC 400x400".to_string(),
            force_kn: -310.0,
            capacity_kn: 800.0,
            utilization: 0.62,
            moment_my_knm: 28.0,
            moment_mz_knm: 19.0,
            is_critical: false,
            is_active: true,
            eigengate_score: 0.86,
            svd_sensitivity: 0.08,
            eigengate: 0.86,
            dcr_interaction: 0.68,
            shear_vy_kn: 42.0,
            svd_margin: 0.45,
            ci: 0.14,
            rr: 0.88,
        }];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        for cmd in ["/gpu-compute", "/mc", "/particles", "/mc-ifcx"] {
            assert!(
                SLASH_COMMANDS.iter().any(|s| s.cmd == cmd),
                "Missing Phase 79 slash command {cmd}"
            );
        }

        let act_mc = spatial.execute_spatial_input(
            "/mc 1024",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(act_mc.run_gpu_monte_carlo, Some(1024));
        assert!(spatial.ground_motion_drawer_open);
        assert_eq!(spatial.ground_motion_drawer_tab, 2);

        let _ = canvas.run_monte_carlo_fragility_assessment(&build.bim_bus.project);
        assert!(canvas.last_monte_carlo_report.is_some());

        let _ = spatial.execute_spatial_input(
            "/particles 2048",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas.monte_carlo_config.gpu_particles_enabled);
        assert_eq!(canvas.monte_carlo_config.gpu_particle_count, 2048);

        let act_ifcx = spatial.execute_spatial_input(
            "/mc-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ifcx.export_monte_carlo_ifcx);

        let (clean_txt, dirs) = parse_gemini_action_directives(
            "Running GPU compute [[RUN_GPU_MONTE_CARLO:2048]] [[TOGGLE_GPU_PARTICLES]] [[EXPORT_MC_IFCX]]",
        );
        assert_eq!(clean_txt, "Running GPU compute");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunGpuMonteCarlo(2048),
                GeminiDirective::ToggleGpuParticles,
                GeminiDirective::ExportMcIfcx,
            ]
        );

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(win_w, 820.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                let _ = spatial.render_hud_overlays(
                    ui.ctx(),
                    &theme,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                    &members,
                    &projects,
                    &mut db_components,
                );
            });
            out.textures_delta.clear();
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.right() <= win_w + 1.0 && r.left() >= -1.0,
                "Phase 79 Dynamic Island overflowed viewport {win_w}: {r:?}"
            );
        }
    }

    #[test]
    fn test_phase80_collab_and_risk_map_slash_commands_zero_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember {
            id: 1,
            name: "Col-A1".to_string(),
            section: "RC 400x400".to_string(),
            force_kn: -380.0,
            capacity_kn: 720.0,
            utilization: 0.74,
            moment_my_knm: 38.0,
            moment_mz_knm: 26.0,
            is_critical: false,
            is_active: true,
            eigengate_score: 0.81,
            svd_sensitivity: 0.14,
            eigengate: 0.81,
            dcr_interaction: 0.76,
            shear_vy_kn: 50.0,
            svd_margin: 0.44,
            ci: 0.16,
            rr: 0.87,
        }];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        for cmd in ["/checkpoint", "/approve", "/merge-ifcx", "/risk-map", "/collab-ifcx"] {
            assert!(
                SLASH_COMMANDS.iter().any(|s| s.cmd == cmd),
                "Missing Phase 80 slash command {cmd}"
            );
        }

        let act_cp = spatial.execute_spatial_input(
            "/checkpoint Baseline Audit",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            act_cp.create_checkpoint_label.as_deref(),
            Some("Baseline Audit")
        );
        assert!(spatial.collab_drawer_open);
        assert_eq!(spatial.collab_drawer_tab, 0);
        let _ = canvas.create_signed_ifcx_checkpoint(&build.bim_bus.project, &members, "Baseline Audit");

        let act_app = spatial.execute_spatial_input(
            "/approve",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_app.approve_checkpoint);
        let _ = canvas
            .collab_state
            .approve_active_checkpoint("Lead Reviewer");

        let act_merge = spatial.execute_spatial_input(
            "/merge-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_merge.run_ifcx_three_way_merge);
        let _ = canvas.run_peer_three_way_ifcx_merge(&build.bim_bus.project, &members);

        let act_risk = spatial.execute_spatial_input(
            "/risk-map pml90",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_risk.refresh_district_risk_field);
        assert_eq!(spatial.collab_drawer_tab, 1);
        assert!(canvas.collab_state.config.risk_heatmap_visible);
        assert_eq!(
            canvas.collab_state.config.risk_metric,
            crate::core::ifcx_collab::DistrictRiskMetric::McPml90Loss
        );
        let _ = canvas.refresh_district_risk_field(&build.bim_bus.project, &members);

        let act_bundle = spatial.execute_spatial_input(
            "/collab-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_bundle.export_collab_bundle_ifcx);

        let (clean_txt, dirs) = parse_gemini_action_directives(
            "Governance update [[CREATE_CHECKPOINT:Peer Handover]] [[APPROVE_CHECKPOINT]] [[TOGGLE_RISK_HEATMAP:EigenGate]]",
        );
        assert_eq!(clean_txt, "Governance update");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::CreateCheckpoint("Peer Handover".to_string()),
                GeminiDirective::ApproveCheckpoint,
                GeminiDirective::ToggleRiskHeatmap(Some(
                    crate::core::ifcx_collab::DistrictRiskMetric::StructuralEigenGate
                )),
            ]
        );

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_collab_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 840.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 80 Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn test_phase81_trade_study_permit_and_relay_slash_commands_zero_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();
        let members = vec![StructuralMember {
            id: 1,
            name: "Col-A1".to_string(),
            section: "RC 400x400".to_string(),
            force_kn: -380.0,
            capacity_kn: 720.0,
            utilization: 0.74,
            moment_my_knm: 38.0,
            moment_mz_knm: 26.0,
            is_critical: false,
            is_active: true,
            eigengate_score: 0.81,
            svd_sensitivity: 0.14,
            eigengate: 0.81,
            dcr_interaction: 0.76,
            shear_vy_kn: 50.0,
            svd_margin: 0.44,
            ci: 0.16,
            rr: 0.87,
        }];
        let projects = Vec::new();
        let mut db_components = Vec::new();

        for cmd in ["/trade-study", "/permit-check", "/permit-svg", "/relay", "/sync-peer"] {
            assert!(
                SLASH_COMMANDS.iter().any(|s| s.cmd == cmd),
                "Missing Phase 81 slash command {cmd}"
            );
        }

        let act_ts = spatial.execute_spatial_input(
            "/trade-study carbon",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            act_ts.run_trade_study,
            Some(crate::core::trade_study::TradeStudyObjective::MinCarbon)
        );
        assert!(spatial.collab_drawer_open);
        assert_eq!(spatial.collab_drawer_tab, 2);
        let _ = canvas.run_trade_study_analysis(&build.bim_bus.project, &members);

        let act_perm = spatial.execute_spatial_input(
            "/permit-check",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_perm.run_permit_compliance);
        let _ = canvas.refresh_permit_compliance(&build.bim_bus.project, &members);

        let act_svg = spatial.execute_spatial_input(
            "/permit-svg",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_svg.export_permit_annex_svg);

        let act_relay = spatial.execute_spatial_input(
            "/relay wss://relay.eigengraph.net/test",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            act_relay.connect_collab_relay.as_deref(),
            Some("wss://relay.eigengraph.net/test")
        );
        assert_eq!(spatial.collab_drawer_tab, 0);

        let act_sync = spatial.execute_spatial_input(
            "/sync-peer",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_sync.sync_peer_relay_delta);
        let _ = canvas.simulate_peer_relay_sync(&build.bim_bus.project, &members);

        let (clean_txt, dirs) = parse_gemini_action_directives(
            "Trade check [[RUN_TRADE_STUDY:SAFETY]] [[EXPORT_PERMIT_ANNEX]] [[SYNC_PEER_RELAY:wss://relay.eigengraph.net/room1]]",
        );
        assert_eq!(clean_txt, "Trade check");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunTradeStudy(Some(
                    crate::core::trade_study::TradeStudyObjective::MaxSafetyEigenGate
                )),
                GeminiDirective::ExportPermitAnnex,
                GeminiDirective::SyncPeerRelay(Some(
                    "wss://relay.eigengraph.net/room1".to_string()
                )),
            ]
        );

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1, 2] {
            spatial.open_collab_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 81 Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase82_field_qaqc_bbs_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus.project = BimProject::sample_bimcraft_building();
        build
            .bim_bus
            .project
            .reinforce_all_concrete(&crate::core::bim::RebarPreset::default());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        for cmd in [
            "/bbs",
            "/cutlist",
            "/qaqc-3d",
            "/waypoint",
            "/shop-svg",
            "/eigengate",
            "/eigenflow",
        ] {
            assert!(
                SLASH_COMMANDS.iter().any(|s| s.cmd == cmd),
                "Missing Phase 82 slash command {cmd}"
            );
        }

        let act_bbs = spatial.execute_spatial_input(
            "/bbs 6",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(act_bbs.run_field_fabrication, Some(6.0));
        assert!(spatial.field_qaqc_drawer_open);
        assert_eq!(spatial.field_qaqc_drawer_tab, 0);
        let _ = canvas.run_field_fabrication_audit(&build.bim_bus.project);

        let act_3d = spatial.execute_spatial_input(
            "/qaqc-3d",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_3d.toggle_field_qaqc_3d);
        assert_eq!(spatial.field_qaqc_drawer_tab, 1);

        let act_wp = spatial.execute_spatial_input(
            "/waypoint 2",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(act_wp.fly_to_waypoint_idx, Some(1));

        let act_svg = spatial.execute_spatial_input(
            "/shop-svg",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_svg.export_shop_drawing_svg);

        let _ = spatial.execute_spatial_input(
            "/ari",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        let _ = spatial.execute_spatial_input(
            "/eigenflow",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );

        let (clean, dirs) = parse_gemini_action_directives(
            "Shop ready [[RUN_BBS_OPTIMIZE:6m]] [[TOGGLE_QAQC_3D]] [[EXPORT_SHOP_SVG]] [[RUN_EIGENFLOW]]",
        );
        assert_eq!(clean, "Shop ready");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunBbsOptimize(6.0),
                GeminiDirective::ToggleQaqc3d,
                GeminiDirective::ExportShopSvg,
                GeminiDirective::RunEigenFlow,
            ]
        );

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_field_qaqc_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 82 Field QA/QC Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase83_eigenflow_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_studio = spatial.execute_spatial_input(
            "/eigenflow-studio",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_studio.run_eigenflow_studio);
        assert!(spatial.eigenflow_drawer_open);
        assert_eq!(spatial.eigenflow_drawer_tab, 0);

        let act_push = spatial.execute_spatial_input(
            "/pushover triangular",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(
            act_push.eigenflow_set_pattern,
            Some(crate::core::pushover::LateralLoadPattern::InvertedTriangular)
        );
        assert_eq!(spatial.eigenflow_drawer_tab, 1);

        let act_svg = spatial.execute_spatial_input(
            "/eigenflow-svg",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_svg.export_eigenflow_svg);

        let act_ifcx = spatial.execute_spatial_input(
            "/eigenflow-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ifcx.export_eigenflow_ifcx);

        let act_3d = spatial.execute_spatial_input(
            "/eigenflow-3d",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_3d.toggle_eigenflow_3d);

        let (clean, dirs) = parse_gemini_action_directives(
            "Non-linear ready [[OPEN_EIGENFLOW_STUDIO]] [[RUN_PUSHOVER:TRIANGULAR]] [[TOGGLE_EIGENFLOW_3D]] [[EXPORT_EIGENFLOW_ANNEX]]",
        );
        assert_eq!(clean, "Non-linear ready");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::OpenEigenFlowStudio,
                GeminiDirective::RunPushover(
                    crate::core::pushover::LateralLoadPattern::InvertedTriangular
                ),
                GeminiDirective::ToggleEigenFlow3d,
                GeminiDirective::ExportEigenFlowAnnex,
            ]
        );

        let _ = canvas.run_eigenflow_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_eigenflow_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 83 EigenFlow Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_extreme_events_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_blast = spatial.execute_spatial_input(
            "/blast 400 9",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_blast.run_extreme_events);
        assert_eq!(canvas.extreme_events_config.explosive_mass_kg, 400.0);
        assert_eq!(canvas.extreme_events_config.standoff_distance_m, 9.0);
        assert!(spatial.extreme_events_drawer_open);
        assert_eq!(spatial.extreme_events_drawer_tab, 0);

        let act_fire = spatial.execute_spatial_input(
            "/fire 120",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_fire.run_extreme_events);
        assert_eq!(canvas.extreme_events_config.exposure_minutes, 120.0);
        assert_eq!(spatial.extreme_events_drawer_tab, 1);

        let act_story = spatial.execute_spatial_input(
            "/fire-story 2",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_story.run_extreme_events);
        assert_eq!(canvas.extreme_events_config.fire_story_idx, 1);

        assert!(spatial
            .execute_spatial_input(
                "/extreme-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_extreme_events_3d);
        assert!(spatial
            .execute_spatial_input(
                "/extreme-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_extreme_events_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/extreme-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_extreme_events_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Hazard audit [[RUN_BLAST:500kg,8m]] [[RUN_FIRE:120min,L2]] [[TOGGLE_EXTREME_3D]] [[EXPORT_EXTREME_ANNEX]]",
        );
        assert_eq!(clean, "Hazard audit");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunBlast {
                    mass_kg: 500.0,
                    standoff_m: 8.0,
                },
                GeminiDirective::RunFire {
                    minutes: 120.0,
                    story: Some(1),
                },
                GeminiDirective::ToggleExtreme3d,
                GeminiDirective::ExportExtremeAnnex,
            ]
        );

        let _ = canvas.run_extreme_events_audit(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_extreme_events_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84 Extreme Events Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_1_isolation_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_fps = spatial.execute_spatial_input(
            "/fps",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_fps.run_isolation_studio);
        assert_eq!(
            canvas.isolation_studio_config.isolator_type,
            crate::core::isolation::IsolatorType::FrictionPendulum
        );
        assert!(spatial.isolation_drawer_open);

        assert!(spatial
            .execute_spatial_input(
                "/isolation-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_isolation_3d);
        assert!(spatial
            .execute_spatial_input(
                "/isolation-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_isolation_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/isolation-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_isolation_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Isolate base [[RUN_ISOLATION:LRB]] [[TOGGLE_ISOLATION_3D]] [[EXPORT_ISOLATION_ANNEX]]",
        );
        assert_eq!(clean, "Isolate base");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunIsolation(
                    crate::core::isolation::IsolatorType::LeadRubberBearing
                ),
                GeminiDirective::ToggleIsolation3d,
                GeminiDirective::ExportIsolationAnnex,
            ]
        );

        let _ = canvas.run_isolation_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_isolation_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.1 Isolation Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_2_wind_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_dir = spatial.execute_spatial_input(
            "/wind-dir 52 135",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_dir.run_wind_studio);
        assert!((canvas.wind_studio_config.basic_wind_speed_ms - 52.0).abs() < 1e-6);
        assert!((canvas.wind_studio_config.wind_azimuth_deg - 135.0).abs() < 1e-6);
        assert!(spatial.wind_drawer_open);

        let act_exp = spatial.execute_spatial_input(
            "/wind-exposure d",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_exp.run_wind_studio);
        assert_eq!(
            canvas.wind_studio_config.exposure,
            crate::core::wind::ExposureCategory::ExposureD
        );

        assert!(spatial
            .execute_spatial_input(
                "/wind-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_wind_3d);
        assert!(spatial
            .execute_spatial_input(
                "/wind-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_wind_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/wind-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_wind_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Wind check [[RUN_WIND:48:D:270]] [[TOGGLE_WIND_3D]] [[EXPORT_WIND_ANNEX]]",
        );
        assert_eq!(clean, "Wind check");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunWind {
                    speed_ms: Some(48.0),
                    exposure: Some(crate::core::wind::ExposureCategory::ExposureD),
                    azimuth_deg: Some(270.0),
                },
                GeminiDirective::ToggleWind3d,
                GeminiDirective::ExportWindAnnex,
            ]
        );

        let _ = canvas.run_wind_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_wind_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.2 Wind Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_3_shear_wall_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_sbe = spatial.execute_spatial_input(
            "/sbe 400 45",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_sbe.run_shear_wall_studio);
        assert!((canvas.shear_wall_studio_config.wall_thickness_mm - 400.0).abs() < 1e-6);
        assert!((canvas.shear_wall_studio_config.fc_mpa - 45.0).abs() < 1e-6);
        assert!(spatial.shear_wall_drawer_open);

        let act_cb = spatial.execute_spatial_input(
            "/coupling-beam 1.4 0.90",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_cb.run_shear_wall_studio);
        assert!((canvas.shear_wall_studio_config.cb_clear_span_m - 1.4).abs() < 1e-6);
        assert_eq!(spatial.shear_wall_drawer_tab, 1);

        assert!(spatial
            .execute_spatial_input(
                "/shear-wall-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_shear_wall_3d);
        assert!(spatial
            .execute_spatial_input(
                "/shear-wall-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_shear_wall_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/shear-wall-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_shear_wall_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Wall audit [[RUN_SHEAR_WALL:350:40:0.015]] [[TOGGLE_SHEAR_WALL_3D]] [[EXPORT_SHEAR_WALL_ANNEX]]",
        );
        assert_eq!(clean, "Wall audit");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunShearWall {
                    tw_mm: Some(350.0),
                    fc_mpa: Some(40.0),
                    drift_ratio: Some(0.015),
                },
                GeminiDirective::ToggleShearWall3d,
                GeminiDirective::ExportShearWallAnnex,
            ]
        );

        let _ = canvas.run_shear_wall_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_shear_wall_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.3 Shear Wall Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_4_connection_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_ep = spatial.execute_spatial_input(
            "/endplate 27 28",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_ep.run_connection_studio);
        assert_eq!(
            canvas.connection_studio_config.connection_type,
            crate::core::connection::ConnectionType::ExtendedEndPlate4E
        );
        assert!((canvas.connection_studio_config.bolt_diameter_mm - 27.0).abs() < 1e-6);
        assert!((canvas.connection_studio_config.plate_thickness_mm - 28.0).abs() < 1e-6);
        assert!(spatial.connection_drawer_open);

        assert!(spatial
            .execute_spatial_input(
                "/connection-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_connection_3d);
        assert!(spatial
            .execute_spatial_input(
                "/connection-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_connection_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/connection-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_connection_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Check joints [[RUN_CONNECTION:ENDPLATE:24:26]] [[TOGGLE_CONNECTION_3D]] [[EXPORT_CONNECTION_ANNEX]]",
        );
        assert_eq!(clean, "Check joints");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunConnection {
                    conn_type: Some(crate::core::connection::ConnectionType::ExtendedEndPlate4E),
                    bolt_dia_mm: Some(24.0),
                    plate_thick_mm: Some(26.0),
                },
                GeminiDirective::ToggleConnection3d,
                GeminiDirective::ExportConnectionAnnex,
            ]
        );

        let _ = canvas.run_connection_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_connection_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.4 Connection Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_5_prestress_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_pt = spatial.execute_spatial_input(
            "/tendon harped 22",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_pt.run_prestress_studio);
        assert_eq!(
            canvas.prestress_studio_config.profile_kind,
            crate::core::prestressed_concrete::TendonProfileKind::HarpedTwoPoint
        );
        assert_eq!(canvas.prestress_studio_config.num_strands, 22);
        assert!(spatial.prestress_drawer_open);

        assert!(spatial
            .execute_spatial_input(
                "/prestress-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_prestress_3d);
        assert!(spatial
            .execute_spatial_input(
                "/prestress-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_prestress_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/prestress-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_prestress_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Check PT [[RUN_PRESTRESS:PARABOLIC:20:50]] [[TOGGLE_PRESTRESS_3D]] [[EXPORT_PRESTRESS_ANNEX]]",
        );
        assert_eq!(clean, "Check PT");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunPrestress {
                    profile: Some(crate::core::prestressed_concrete::TendonProfileKind::Parabolic),
                    num_strands: Some(20),
                    fc_mpa: Some(50.0),
                },
                GeminiDirective::TogglePrestress3d,
                GeminiDirective::ExportPrestressAnnex,
            ]
        );

        let _ = canvas.run_prestress_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_prestress_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.5 Prestress Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_6_composite_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_st = spatial.execute_spatial_input(
            "/studs 56 parallel",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_st.run_composite_studio);
        assert_eq!(canvas.composite_studio_config.studs_provided, 56);
        assert_eq!(
            canvas.composite_studio_config.deck_orientation,
            crate::core::composite::DeckOrientation::Parallel
        );
        assert!(spatial.composite_drawer_open);

        assert!(spatial
            .execute_spatial_input(
                "/composite-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_composite_3d);
        assert!(spatial
            .execute_spatial_input(
                "/composite-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_composite_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/composite-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_composite_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Deck check [[RUN_COMPOSITE:48:PERPENDICULAR:UNSHORED]] [[TOGGLE_COMPOSITE_3D]] [[EXPORT_COMPOSITE_ANNEX]]",
        );
        assert_eq!(clean, "Deck check");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunComposite {
                    studs: Some(48),
                    orientation: Some(crate::core::composite::DeckOrientation::Perpendicular),
                    unshored: Some(true),
                },
                GeminiDirective::ToggleComposite3d,
                GeminiDirective::ExportCompositeAnnex,
            ]
        );

        let _ = canvas.run_composite_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_composite_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.6 Composite Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_7_reliability_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_rel = spatial.execute_spatial_input(
            "/sorm 3.8 drift",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_rel.run_reliability_studio);
        assert!((canvas.reliability_studio_config.target_beta - 3.8).abs() < 1e-6);
        assert_eq!(
            canvas.reliability_studio_config.method,
            crate::core::reliability_studio::ReliabilityMethod::SormBreitung
        );
        assert_eq!(
            canvas.reliability_studio_config.limit_state,
            crate::core::reliability_studio::LimitStateKind::InterstoryDrift
        );
        assert!(spatial.reliability_drawer_open);

        assert!(spatial
            .execute_spatial_input(
                "/reliability-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_reliability_3d);
        assert!(spatial
            .execute_spatial_input(
                "/reliability-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_reliability_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/reliability-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_reliability_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Probabilistic audit [[RUN_RELIABILITY:FORM:3.8:DRIFT]] [[TOGGLE_RELIABILITY_3D]] [[EXPORT_RELIABILITY_ANNEX]]",
        );
        assert_eq!(clean, "Probabilistic audit");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunReliability {
                    method: Some(crate::core::reliability_studio::ReliabilityMethod::FormHasoferLind),
                    target_beta: Some(3.8),
                    limit_state: Some(crate::core::reliability_studio::LimitStateKind::InterstoryDrift),
                },
                GeminiDirective::ToggleReliability3d,
                GeminiDirective::ExportReliabilityAnnex,
            ]
        );

        let _ = canvas.run_reliability_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_reliability_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.7 Reliability Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_8_foundation_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_fnd = spatial.execute_spatial_input(
            "/pile-group 3x3 24m soft",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_fnd.run_foundation_studio);
        assert_eq!(canvas.foundation_studio_config.piles_x, 3);
        assert_eq!(canvas.foundation_studio_config.piles_y, 3);
        assert!((canvas.foundation_studio_config.pile_length_m - 24.0).abs() < 1e-6);
        assert_eq!(
            canvas.foundation_studio_config.soil_type,
            crate::core::geotech::SoilType::SoftClay
        );
        assert!(spatial.foundation_drawer_open);

        assert!(spatial
            .execute_spatial_input(
                "/foundation-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_foundation_3d);
        assert!(spatial
            .execute_spatial_input(
                "/foundation-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_foundation_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/foundation-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_foundation_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Deep piles [[RUN_FOUNDATION:SOFT_CLAY:3X3:24]] [[TOGGLE_FOUNDATION_3D]] [[EXPORT_FOUNDATION_ANNEX]]",
        );
        assert_eq!(clean, "Deep piles");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunFoundation {
                    soil_type: Some(crate::core::geotech::SoilType::SoftClay),
                    piles_x: Some(3),
                    piles_y: Some(3),
                    pile_length_m: Some(24.0),
                },
                GeminiDirective::ToggleFoundation3d,
                GeminiDirective::ExportFoundationAnnex,
            ]
        );

        let _ = canvas.run_foundation_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_foundation_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.8 Deep Foundation Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase84_9_construction_lca_studio_drawer_responsive_and_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_lca = spatial.execute_spatial_input(
            "/sequence 120 dri geopolymer",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_lca.run_construction_lca_studio);
        assert_eq!(canvas.construction_lca_config.active_day, 120);
        assert_eq!(
            canvas.construction_lca_config.steel_route,
            crate::core::construction_lca_studio::SteelSmeltingRoute::GreenHydrogenDri
        );
        assert_eq!(
            canvas.construction_lca_config.concrete_mix,
            crate::core::construction_lca_studio::ConcreteMixGrade::GeopolymerGreen
        );
        assert!(spatial.construction_lca_drawer_open);

        assert!(spatial
            .execute_spatial_input(
                "/4d-3d",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .toggle_construction_3d);
        assert!(spatial
            .execute_spatial_input(
                "/4d-ifcx",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_construction_ifcx);
        assert!(spatial
            .execute_spatial_input(
                "/4d-svg",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_construction_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "4D/5D/6D update [[RUN_CONSTRUCTION_LCA:DAY120:DRI:GEOPOLYMER]] [[TOGGLE_CONSTRUCTION_3D]] [[EXPORT_CONSTRUCTION_ANNEX]]",
        );
        assert_eq!(clean, "4D/5D/6D update");
        assert_eq!(
            dirs,
            vec![
                GeminiDirective::RunConstructionLca {
                    active_day: Some(120),
                    steel_route: Some(
                        crate::core::construction_lca_studio::SteelSmeltingRoute::GreenHydrogenDri
                    ),
                    concrete_mix: Some(
                        crate::core::construction_lca_studio::ConcreteMixGrade::GeopolymerGreen
                    ),
                },
                GeminiDirective::ToggleConstruction3d,
                GeminiDirective::ExportConstructionAnnex,
            ]
        );

        let _ = canvas.run_construction_lca_studio(&build.bim_bus.project);
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx = egui::Context::default();
        for tab_idx in [0_usize, 1] {
            spatial.open_construction_lca_drawer(tab_idx);
            for win_w in [1600.0_f32, 1280.0, 768.0, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 860.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Phase 84.9 4D/5D/6D Construction LCA Dynamic Island (tab={tab_idx}) overflowed viewport {win_w}: {r:?}"
                );
            }
        }
    }

    #[test]
    fn phase85_0_master_dossier_index_s000_to_s114_commands_and_directives() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = crate::views::build::BuildView::default();
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        assert!(spatial
            .execute_spatial_input(
                "/master-dossier",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_master_dossier_svg);
        assert!(spatial
            .execute_spatial_input(
                "/dossier-index",
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            )
            .export_master_dossier_svg);

        let (clean, dirs) = parse_gemini_action_directives(
            "Full index [[EXPORT_MASTER_DOSSIER_INDEX]]",
        );
        assert_eq!(clean, "Full index");
        assert_eq!(dirs, vec![GeminiDirective::ExportMasterDossierIndex]);
    }

    #[test]
    fn test_v0862_smart_layered_ribbon_dynamic_island() {
        assert_eq!(RibbonCategoryTab::ALL.len(), 10);
        let expected_labels = [
            "Planning",
            "Drafting",
            "Modeling",
            "Scheduling",
            "Budgeting",
            "Sustainability",
            "Facility",
            "Safety",
            "Construction",
            "Industrialization",
        ];
        for (tab, exp) in RibbonCategoryTab::ALL.iter().zip(expected_labels.iter()) {
            assert_eq!(tab.label(), *exp);
        }

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::EngineeringLight);
        let ctx = egui::Context::default();
        let mut build = crate::views::build::BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // Layer 1: Categorized Ribbon across all 10 AEC categories on Desktop, Tablet & Mobile
        for tab in RibbonCategoryTab::ALL {
            for win_w in [1000.0_f32, 700.0, 390.0] {
                let mut spatial = SpatialWorkspaceState::default();
                let mut canvas = UniversalCanvasState::default();
                spatial.active_ribbon_tab = tab;
                spatial.mobile_deck_pinned = true;

                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 840.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial.hud_rects.len(), 1);
                let r = spatial.hud_rects[0];
                assert!(
                    r.right() <= win_w + 1.0 && r.left() >= -1.0,
                    "Ribbon tab {} overflowed viewport {win_w}: {r:?}",
                    tab.label()
                );
                assert!(
                    r.height() <= 270.0,
                    "Ribbon tab {} expanded excessively in height ({}): {r:?}",
                    tab.label(),
                    r.height()
                );
            }
        }

        // Layer 2: Exclusive Project Browser overlay replaces Ribbon cleanly and returns via breadcrumb
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        spatial.mobile_deck_pinned = true;
        spatial.vault_drawer_open = true;
        assert_eq!(
            spatial.active_focus_layer_info(false),
            Some(("Project Browser & Vault", AppIcon::Folder))
        );
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(1000.0, 840.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| {
            let _ = spatial.render_hud_overlays(
                ui.ctx(),
                &theme,
                &mut canvas,
                &mut build,
                &mut ai,
                &members,
                &projects,
                &mut db_components,
            );
        });
        out.textures_delta.clear();
        assert_eq!(spatial.hud_rects.len(), 1);
        assert!(spatial.hud_rects[0].right() <= 1001.0);

        spatial.close_focus_layer_return_to_ribbon();
        assert_eq!(spatial.active_focus_layer_info(false), None);
        assert!(spatial.mobile_deck_pinned);
    }

    #[test]
    fn test_v0871_dynamic_island_strict_height_and_width_stability() {
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let mut build = crate::views::build::BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = crate::views::ai_assistant::AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        for (win_w, win_h, is_desktop) in [
            (1440.0_f32, 900.0_f32, true),
            (768.0_f32, 900.0_f32, false),
            (390.0_f32, 844.0_f32, false),
        ] {
            // 1. Layer 0 (Collapsed) over 6 frames
            let ctx0 = egui::Context::default();
            let mut spatial0 = SpatialWorkspaceState::default();
            let mut canvas0 = UniversalCanvasState::default();
            let mut rect0_f4 = Rect::NOTHING;
            let mut rect0_f6 = Rect::NOTHING;
            for frame_idx in 0..6 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx0.run_ui(raw, |ui| {
                    let _ = spatial0.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas0,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial0.hud_rects.len(), 1);
                let r = spatial0.hud_rects[0];
                assert!(
                    r.left() >= -1.0 && r.right() <= win_w + 1.0,
                    "Collapsed island overflowed viewport {win_w}: {r:?}"
                );
                let max_collapsed_h = if win_w < MacTheme::MOBILE_BREAKPOINT {
                    165.0
                } else {
                    130.0
                };
                assert!(
                    r.height() <= max_collapsed_h,
                    "Collapsed island too tall ({}) at {win_w}x{win_h}: {r:?}",
                    r.height()
                );
                if is_desktop {
                    assert!(
                        r.width() <= 430.0,
                        "Desktop collapsed island too wide ({}): {r:?}",
                        r.width()
                    );
                } else {
                    assert!(
                        r.width() <= win_w + 1.0,
                        "Mobile/tablet collapsed island exceeded viewport {win_w}: {r:?}"
                    );
                }
                if frame_idx >= 3 {
                    assert!(
                        r.top() >= -1.0 && r.bottom() <= win_h + 1.0,
                        "Collapsed island escaped vertical bounds at {win_w}x{win_h}: {r:?}"
                    );
                }
                if frame_idx == 3 {
                    rect0_f4 = r;
                } else if frame_idx == 5 {
                    rect0_f6 = r;
                }
            }
            assert!(
                (rect0_f6.width() - rect0_f4.width()).abs() <= 1.5
                    && (rect0_f6.height() - rect0_f4.height()).abs() <= 1.5,
                "Collapsed island drifted across frames: f4={rect0_f4:?}, f6={rect0_f6:?}"
            );

            // 2. Layer 1 (Categorized Ribbon) across ALL 10 tabs over 6 frames each
            for tab in RibbonCategoryTab::ALL {
                let ctx1 = egui::Context::default();
                let mut spatial1 = SpatialWorkspaceState::default();
                let mut canvas1 = UniversalCanvasState::default();
                spatial1.active_ribbon_tab = tab;
                spatial1.mobile_deck_pinned = true;
                let mut rect1_f4 = Rect::NOTHING;
                let mut rect1_f6 = Rect::NOTHING;
                for frame_idx in 0..6 {
                    let raw = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(win_w, win_h),
                        )),
                        ..Default::default()
                    };
                    let mut out = ctx1.run_ui(raw, |ui| {
                        let _ = spatial1.render_hud_overlays(
                            ui.ctx(),
                            &theme,
                            &mut canvas1,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    });
                    out.textures_delta.clear();
                    assert_eq!(spatial1.hud_rects.len(), 1);
                    let r = spatial1.hud_rects[0];
                    assert!(
                        r.left() >= -1.0 && r.right() <= win_w + 1.0,
                        "Layer 1 Ribbon {} overflowed {win_w}: {r:?}",
                        tab.label()
                    );
                    if is_desktop {
                        assert!(
                            r.width() <= 1140.0,
                            "Desktop Ribbon {} too wide ({}): {r:?}",
                            tab.label(),
                            r.width()
                        );
                    }
                    assert!(
                        r.height() <= 270.0,
                        "Ribbon {} blew up vertically ({}) at {win_w}x{win_h}: {r:?}",
                        tab.label(),
                        r.height()
                    );
                    if frame_idx >= 3 {
                        assert!(
                            r.top() >= -1.0 && r.bottom() <= win_h + 1.0,
                            "Ribbon {} escaped vertical bounds at {win_w}x{win_h}: {r:?}",
                            tab.label()
                        );
                    }
                    if frame_idx == 3 {
                        rect1_f4 = r;
                    } else if frame_idx == 5 {
                        rect1_f6 = r;
                    }
                }
                assert!(
                    (rect1_f6.width() - rect1_f4.width()).abs() <= 1.5
                        && (rect1_f6.height() - rect1_f4.height()).abs() <= 1.5,
                    "Ribbon {} drifted f4->f6 at {win_w}x{win_h}: f4={rect1_f4:?}, f6={rect1_f6:?}",
                    tab.label()
                );
            }

            // 3. Layer 2 (Vault / Focus Overlay) over 6 frames
            let ctx2 = egui::Context::default();
            let mut spatial2 = SpatialWorkspaceState::default();
            let mut canvas2 = UniversalCanvasState::default();
            spatial2.mobile_deck_pinned = true;
            spatial2.vault_drawer_open = true;
            for frame_idx in 0..6 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx2.run_ui(raw, |ui| {
                    let _ = spatial2.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas2,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
                assert_eq!(spatial2.hud_rects.len(), 1);
                let r = spatial2.hud_rects[0];
                assert!(
                    r.left() >= -1.0 && r.right() <= win_w + 1.0,
                    "Layer 2 overlay overflowed {win_w}: {r:?}"
                );
                if frame_idx >= 3 {
                    assert!(
                        r.top() >= -1.0 && r.bottom() <= win_h + 1.0,
                        "Layer 2 overlay escaped vertical bounds at {win_w}x{win_h}: {r:?}"
                    );
                }
                if is_desktop {
                    assert!(
                        r.width() <= 1170.0,
                        "Desktop Layer 2 overlay too wide ({}): {r:?}",
                        r.width()
                    );
                }
                assert!(
                    r.height() <= (win_h - 20.0).min(875.0),
                    "Layer 2 overlay exceeded height ceiling ({}) at {win_w}x{win_h}: {r:?}",
                    r.height()
                );
            }
        }
    }

    #[test]
    fn test_v0873_ribbon_no_redundant_file_pill_and_section_box_responsive() {
        for cmd in ["/section", "/level-cut", "/section-reset"] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing v0.8.7.3 section slash command {cmd} in SLASH_COMMANDS"
            );
        }

        // Verify redundant blue "File" pill was removed from Row B ribbon tabs
        let ribbon_src = include_str!("ribbon_ui.rs");
        assert!(
            !ribbon_src.contains("RichText::new(\"File\")"),
            "Redundant Row B File pill must be removed next to Planning"
        );

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let _ = spatial.execute_spatial_input(
            "/section",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas.section_box.enabled && !canvas.section_box.level_cut_mode);

        let _ = spatial.execute_spatial_input(
            "/section site",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas.section_box.clip_terrain_and_city);

        let _ = spatial.execute_spatial_input(
            "/level-cut 4.8",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas.section_box.enabled && canvas.section_box.level_cut_mode);
        assert!((canvas.section_box.max_enu_m[2] - 4.8).abs() < 1e-4);

        let _ = spatial.execute_spatial_input(
            "/section-reset",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(!canvas.section_box.enabled);
        assert!(!canvas.section_box.clip_terrain_and_city);

        // Render Modeling Ribbon with live Section Box bar active across Desktop / Tablet / Mobile
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        canvas.toggle_section_box(&build.bim_bus.project);
        assert!(canvas.section_box.enabled);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            spatial.mobile_deck_pinned = true;
            spatial.active_ribbon_tab = RibbonCategoryTab::Modeling;
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            let r = spatial.hud_rects[0];
            assert!(
                r.left() >= -1.0 && r.right() <= win_w + 1.0,
                "Modeling Ribbon with Section Box overflowed {win_w}: {r:?}"
            );
            assert!(
                r.height() <= 320.0,
                "Modeling Ribbon with Section Box too tall ({}) at {win_w}x{win_h}: {r:?}",
                r.height()
            );
        }
    }

    #[test]
    fn test_v0874_gemini_function_calling_loop_custom_model_and_ribbon_scrollbar_clearance() {
        for cmd in ["/model", "/tools"] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing v0.8.7.4 slash command {cmd} in SLASH_COMMANDS"
            );
        }

        // 1. Native Gemini functionCall + thoughtSignature parsing and payload schema
        let raw_fc_json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        { "text": "Inspecting model and applying level cut." },
                        {
                            "thoughtSignature": "sig_v0874_abc",
                            "functionCall": {
                                "name": "set_section_box",
                                "args": { "mode": "level_cut", "z_cut_m": 4.2 }
                            }
                        }
                    ]
                }
            }]
        }"#;
        let parsed = crate::core::web_bridge::parse_gemini_turn_response(raw_fc_json)
            .expect("Parsed functionCall turn");
        assert_eq!(parsed.function_calls.len(), 1);
        assert_eq!(parsed.function_calls[0].name, "set_section_box");
        assert!(parsed.raw_model_content.is_some());

        let tools_schema = crate::core::agent_tools::gemini_function_declarations();
        let payload = crate::core::web_bridge::build_gemini_tool_contents_payload(
            "System",
            &[serde_json::json!({ "role": "user", "parts": [{ "text": "Audit" }] })],
            Some(&tools_schema),
            "gemini-4-pro",
        );
        assert!(payload["tools"][0]["functionDeclarations"].as_array().is_some());
        assert_eq!(payload["generationConfig"]["maxOutputTokens"], 4096);

        // 2. Custom model ID + /tools slash commands & preserved AgentModelChoice::next()
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        assert_eq!(AgentModelChoice::Gemini25Pro.next(), AgentModelChoice::EigenAiProFem);
        let _ = spatial.execute_spatial_input(
            "/model gemini-3.5-pro",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert_eq!(spatial.active_gemini_model_id(), "gemini-3.5-pro");
        let _ = spatial.execute_spatial_input(
            "/model reset",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.custom_gemini_model_id.is_empty());

        let _ = spatial.execute_spatial_input(
            "/tools off",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(!spatial.gemini_native_tools_enabled);
        let _ = spatial.execute_spatial_input(
            "/tools on",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.gemini_native_tools_enabled);

        // 3. Simulated Gemini tool call execution via handle_gemini_reply
        let col_id = build.bim_bus.project.columns[0].id;
        let sim_reply = crate::core::web_bridge::GeminiApiReply {
            request_id: 99,
            model: "gemini-3.5-pro".to_string(),
            ok: true,
            text: parsed.text.clone(),
            error: None,
            function_calls: vec![
                crate::core::agent_tools::GeminiToolCall {
                    name: "inspect_model".to_string(),
                    args: serde_json::json!({}),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "set_section_box".to_string(),
                    args: serde_json::json!({ "mode": "level_cut", "z_cut_m": 4.2 }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "resize_column".to_string(),
                    args: serde_json::json!({ "column_id": col_id, "width_mm": 525, "depth_mm": 525 }),
                },
            ],
            raw_model_content: parsed.raw_model_content,
        };
        let hud_act =
            spatial.handle_gemini_reply(sim_reply, &mut canvas, &mut build, &mut ai);
        assert!(hud_act.sync_fem_and_earth);
        assert!(canvas.section_box.enabled && canvas.section_box.level_cut_mode);
        assert!((canvas.section_box.max_enu_m[2] - 4.2).abs() < 1e-4);
        assert!((build.bim_bus.project.columns[0].width_mm - 525.0).abs() < 1e-6);
        assert_eq!(spatial.gemini_tool_loop_contents.len(), 2);
        assert_eq!(
            spatial.gemini_tool_loop_contents[1]["parts"][1]["functionResponse"]["name"],
            "set_section_box"
        );

        // 4. Ribbon dedicated solid scrollbar clearance + API Key / Custom Model popover responsiveness
        let ribbon_src = include_str!("ribbon_ui.rs");
        assert!(ribbon_src.contains("ScrollStyle::solid()"));
        assert!(ribbon_src.contains("bar_inner_margin = 5.0"));

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            for tab in RibbonCategoryTab::ALL {
                let ctx = egui::Context::default();
                let mut sp = SpatialWorkspaceState::default();
                sp.mobile_deck_pinned = true;
                sp.active_ribbon_tab = tab;
                for _ in 0..4 {
                    let raw = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(win_w, win_h),
                        )),
                        ..Default::default()
                    };
                    let mut out = ctx.run_ui(raw, |ui| {
                        let _ = sp.render_hud_overlays(
                            ui.ctx(),
                            &theme,
                            &mut canvas,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    });
                    out.textures_delta.clear();
                }
                assert_eq!(sp.hud_rects.len(), 1);
                let r = sp.hud_rects[0];
                assert!(
                    r.height() <= 270.0,
                    "Ribbon tab {} exceeded 270px with scrollbar clearance ({}) at {win_w}x{win_h}",
                    tab.label(),
                    r.height()
                );
            }

            let ctx_key = egui::Context::default();
            let mut sp_key = SpatialWorkspaceState::default();
            sp_key.mobile_deck_pinned = true;
            sp_key.api_key_popover_open = true;
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(win_w, win_h),
                    )),
                    ..Default::default()
                };
                let mut out = ctx_key.run_ui(raw, |ui| {
                    let _ = sp_key.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(sp_key.hud_rects.len(), 1);
            assert!(sp_key.hud_rects[0].right() <= win_w + 1.0);
        }
    }

    #[test]
    fn test_v0875_mep_studio_ribbon_slash_commands_and_headless_tool() {
        for cmd in ["/mep", "/hvac", "/plumbing", "/sprinkler", "/mep-3d", "/mep-ifcx"] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing v0.8.7.5 MEP slash command {cmd} in SLASH_COMMANDS"
            );
        }

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_mep = spatial.execute_spatial_input(
            "/mep",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_mep.run_mep_systems);
        assert!(spatial.mep_drawer_open && spatial.mep_drawer_tab == 0);

        let act_sprinkler = spatial.execute_spatial_input(
            "/sprinkler",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_sprinkler.run_mep_systems);
        assert!(spatial.mep_drawer_open && spatial.mep_drawer_tab == 1);

        let act_3d = spatial.execute_spatial_input(
            "/mep-3d",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_3d.toggle_mep_3d);

        let _ = canvas.run_mep_systems(&build.bim_bus.project);
        assert!(canvas.last_mep_report.is_some());
        assert!(canvas.mep_overlay_visible);

        // Gemini native tool loop executing `run_mep_systems`
        let sim_reply = crate::core::web_bridge::GeminiApiReply {
            request_id: 105,
            model: "gemini-3.5-pro".to_string(),
            ok: true,
            text: String::new(),
            error: None,
            function_calls: vec![crate::core::agent_tools::GeminiToolCall {
                name: "run_mep_systems".to_string(),
                args: serde_json::json!({ "hvac_preset": "vrf", "fire_hazard": "light" }),
            }],
            raw_model_content: None,
        };
        let _ = spatial.handle_gemini_reply(sim_reply, &mut canvas, &mut build, &mut ai);
        assert!(spatial.gemini_tool_loop_contents.len() >= 2);

        // Render Modeling Ribbon + MEP Studio Drawer cleanly across Desktop / Tablet / Mobile
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            spatial.mobile_deck_pinned = true;
            spatial.open_mep_drawer(0);
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "MEP Studio Drawer overflowed {win_w}: {:?}",
                spatial.hud_rects[0]
            );
        }
    }

    #[test]
    fn test_v0876_clash_guardrail_drawer_slash_commands_and_gemini_tool() {
        for cmd in ["/clash", "/clash-3d", "/clash-fix", "/clash-ifcx", "/bcf"] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing v0.8.7.6 Clash slash command {cmd} in SLASH_COMMANDS"
            );
        }

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act_clash = spatial.execute_spatial_input(
            "/clash",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_clash.run_clash_guardrail);
        assert!(spatial.clash_drawer_open && spatial.clash_drawer_tab == 0);

        let act_bcf = spatial.execute_spatial_input(
            "/bcf",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_bcf.run_clash_guardrail);
        assert!(spatial.clash_drawer_open && spatial.clash_drawer_tab == 1);

        let act_fix = spatial.execute_spatial_input(
            "/clash-fix",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_fix.auto_resolve_clashes);

        canvas.mep_config.plenum_drop_below_slab_m = 0.25;
        let _ = canvas.run_mep_systems(&build.bim_bus.project);
        let initial = canvas.run_clash_guardrail(&build.bim_bus.project);
        assert!(!initial.clashes.is_empty());

        let resolved = canvas.auto_resolve_mep_clashes(&build.bim_bus.project);
        assert!(resolved.hard_clashes <= initial.hard_clashes);

        // Gemini tool #12 `run_clash_detection`
        let sim_reply = crate::core::web_bridge::GeminiApiReply {
            request_id: 106,
            model: "gemini-3.5-pro".to_string(),
            ok: true,
            text: String::new(),
            error: None,
            function_calls: vec![crate::core::agent_tools::GeminiToolCall {
                name: "run_clash_detection".to_string(),
                args: serde_json::json!({ "clearance_mm": 50.0, "auto_resolve": true }),
            }],
            raw_model_content: None,
        };
        let _ = spatial.handle_gemini_reply(sim_reply, &mut canvas, &mut build, &mut ai);
        assert!(spatial.gemini_tool_loop_contents.len() >= 2);

        // Render Clash Guardrail Drawer cleanly across Desktop / Tablet / Mobile
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            spatial.mobile_deck_pinned = true;
            spatial.open_clash_drawer(0);
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "Clash Guardrail Drawer overflowed {win_w}: {:?}",
                spatial.hud_rects[0]
            );
        }
    }

    #[test]
    fn test_v0877_urban_utilities_drawer_slash_commands_and_gemini_tool() {
        for cmd in [
            "/utilities",
            "/water-net",
            "/sewer-net",
            "/power-grid",
            "/xray-util",
            "/utilities-ifcx",
        ] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing v0.8.7.7 Urban Utilities slash command {cmd} in SLASH_COMMANDS"
            );
        }

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        canvas.city_scene.add_road(
            [-45.0, -14.0],
            [45.0, 14.0],
            crate::core::gis::IfcxRoadKind::Collector,
            false,
        );
        canvas.city_scene.add_zone(
            [0.0, 20.0],
            24.0,
            20.0,
            0.0,
            crate::core::gis::IfcxZoneKind::Commercial,
        );

        let act_util = spatial.execute_spatial_input(
            "/utilities",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_util.run_urban_utilities);
        assert!(spatial.utilities_drawer_open && spatial.utilities_drawer_tab == 0);

        let act_pwr = spatial.execute_spatial_input(
            "/power-grid",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_pwr.run_urban_utilities);
        assert!(spatial.utilities_drawer_open && spatial.utilities_drawer_tab == 1);

        let act_xray = spatial.execute_spatial_input(
            "/xray-util",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_xray.toggle_urban_utilities_xray);

        let rep = canvas.run_urban_utilities(&build.bim_bus.project);
        assert_eq!(rep.road_corridor_count, 1);
        assert_eq!(rep.connected_parcels, 1);
        assert!(canvas.urban_utilities_overlay_visible);

        // Gemini tool #13 `run_urban_utilities`
        let sim_reply = crate::core::web_bridge::GeminiApiReply {
            request_id: 107,
            model: "gemini-3.5-pro".to_string(),
            ok: true,
            text: String::new(),
            error: None,
            function_calls: vec![crate::core::agent_tools::GeminiToolCall {
                name: "run_urban_utilities".to_string(),
                args: serde_json::json!({ "rainfall_mm_hr": 120.0, "substation_mva": 18.0 }),
            }],
            raw_model_content: None,
        };
        let _ = spatial.handle_gemini_reply(sim_reply, &mut canvas, &mut build, &mut ai);
        assert!(spatial.gemini_tool_loop_contents.len() >= 2);

        // Render Urban Utilities Drawer cleanly across Desktop / Tablet / Mobile
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            spatial.mobile_deck_pinned = true;
            spatial.open_utilities_drawer(0);
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "Urban Utilities Drawer overflowed {win_w}: {:?}",
                spatial.hud_rects[0]
            );
        }
    }

    #[test]
    fn test_v0878_city_to_bim_drawer_slash_commands_and_gemini_tools() {
        for cmd in [
            "/promote-bim",
            "/intersections",
            "/roundabout",
            "/intersections-ifcx",
        ] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing v0.8.7.8 City Hub slash command {cmd} in SLASH_COMMANDS"
            );
        }

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        canvas.city_scene.add_road(
            [-55.0, 0.0],
            [55.0, 0.0],
            crate::core::gis::IfcxRoadKind::Arterial,
            false,
        );
        canvas.city_scene.add_road(
            [0.0, -50.0],
            [0.0, 50.0],
            crate::core::gis::IfcxRoadKind::Collector,
            false,
        );
        canvas.city_scene.add_zone(
            [24.0, 22.0],
            26.0,
            22.0,
            0.0,
            crate::core::gis::IfcxZoneKind::Commercial,
        );

        let act_int = spatial.execute_spatial_input(
            "/intersections",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_int.run_road_intersections);
        assert!(spatial.city_hub_drawer_open && spatial.city_hub_drawer_tab == 1);

        let act_rb = spatial.execute_spatial_input(
            "/roundabout",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_rb.toggle_roundabout_mode);

        let act_promo = spatial.execute_spatial_input(
            "/promote-bim",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act_promo.promote_city_zone_to_bim.is_some());
        assert!(spatial.city_hub_drawer_open && spatial.city_hub_drawer_tab == 0);

        let (promoted_bim, promo_sum) = canvas
            .promote_city_zone_to_bim(None)
            .expect("promoted zone to BIM");
        assert!(promoted_bim.columns.len() >= 6);
        assert_eq!(canvas.city_scene.promoted_zone_id, Some(promo_sum.zone_id));

        // Gemini tools #14 & #15
        let sim_reply = crate::core::web_bridge::GeminiApiReply {
            request_id: 108,
            model: "gemini-3.5-pro".to_string(),
            ok: true,
            text: String::new(),
            error: None,
            function_calls: vec![
                crate::core::agent_tools::GeminiToolCall {
                    name: "evaluate_road_intersections".to_string(),
                    args: serde_json::json!({ "prefer_roundabouts": true }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "promote_city_building_to_bim".to_string(),
                    args: serde_json::json!({}),
                },
            ],
            raw_model_content: None,
        };
        let _ = spatial.handle_gemini_reply(sim_reply, &mut canvas, &mut build, &mut ai);
        assert!(spatial.gemini_tool_loop_contents.len() >= 2);

        // Render City Hub Drawer cleanly across Desktop / Tablet / Mobile
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            spatial.mobile_deck_pinned = true;
            spatial.open_city_hub_drawer(0);
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "City Hub Drawer overflowed {win_w}: {:?}",
                spatial.hud_rects[0]
            );
        }
    }

    #[test]
    fn test_v0879_sheets_s116_s118_slash_commands_and_copilot_prompt_polish() {
        for cmd in ["/sheet-s116", "/sheet-s117", "/sheet-s118"] {
            assert!(
                SLASH_COMMANDS.iter().any(|c| c.cmd == cmd),
                "Missing v0.8.7.9 Deliverable slash command {cmd} in SLASH_COMMANDS"
            );
        }

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let act116 = spatial.execute_spatial_input(
            "/sheet-s116",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act116.export_mep_s116);

        let act117 = spatial.execute_spatial_input(
            "/sheet-s117",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act117.export_clash_s117);

        let act118 = spatial.execute_spatial_input(
            "/sheet-s118",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act118.export_city_infra_s118);

        // Populate live MEP, Clash, Urban Utilities & Road Intersections telemetry
        canvas.city_scene.add_road(
            [-50.0, 0.0],
            [50.0, 0.0],
            crate::core::gis::IfcxRoadKind::Arterial,
            false,
        );
        canvas.city_scene.add_road(
            [0.0, -45.0],
            [0.0, 45.0],
            crate::core::gis::IfcxRoadKind::Collector,
            false,
        );
        let _ = canvas.run_mep_systems(&build.bim_bus.project);
        let _ = canvas.run_clash_guardrail(&build.bim_bus.project);
        let _ = canvas.run_urban_utilities(&build.bim_bus.project);
        let _ = canvas.run_road_intersections();

        let sys_prompt = spatial.build_gemini_copilot_system_prompt(
            &canvas,
            &build.bim_bus.project,
            &members,
        );
        for tool_name in crate::core::agent_tools::registered_tool_names() {
            assert!(
                sys_prompt.contains(tool_name),
                "Gemini Copilot system prompt must expose registered tool '{tool_name}'"
            );
        }
        assert!(sys_prompt.contains("MEP:"));
        assert!(sys_prompt.contains("Clash BCF:"));
        assert!(sys_prompt.contains("Urban Utilities:"));
        assert!(sys_prompt.contains("Road Junctions:"));
        assert!(sys_prompt.contains("S-100..S-118"));

        // Verify Deliverables ribbon tab with S-116..S-118 tiles has zero overflow
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            spatial.mobile_deck_pinned = true;
            spatial.close_focus_layer_return_to_ribbon();
            spatial.active_ribbon_tab = RibbonCategoryTab::Drafting;
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "Deliverables ribbon overflowed {win_w}: {:?}",
                spatial.hud_rects[0]
            );
        }
    }

    #[test]
    fn test_v0881_row_a_project_browser_properties_and_layers_vg_responsive() {
        // 1. Slash commands registration & execution
        for cmd in ["/browser", "/properties", "/layers", "/vg"] {
            assert!(
                SLASH_COMMANDS.iter().any(|s| s.cmd == cmd),
                "Missing v0.8.8.1 slash command {cmd}"
            );
        }

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        let _ = spatial.execute_spatial_input(
            "/vg",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.project_browser_drawer_open);
        assert_eq!(spatial.project_browser_tab, 1);

        let _ = spatial.execute_spatial_input(
            "/properties",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.properties_drawer_open);
        assert!(!spatial.project_browser_drawer_open);

        // 2. Visibility/Graphics BIM mesh pruning + custom 2D CAD layers + archive persistence
        let all_tris = crate::views::webgpu_earth::build_rtc_active_bim_fem_mesh_with_visibility(
            &crate::views::webgpu_earth::ActiveBimGpuInput {
                project: &build.bim_bus.project,
                members: &members,
                lens: crate::views::universal_canvas::EarthBimLens::Clean,
                erection_day_limit: 90,
                deform_scale: 25.0,
                active_modal_mode: 0,
                modal_phase_sin: 0.0,
                n1_removed_bim: None,
                cascade_failed_bim: &[],
                cascade_step_progress: 0.0,
                subsurface_pile_depth_m: 6.0,
                heading_deg: 0.0,
                diff_overlay_active: false,
                diff_added_bim: &[],
                diff_modified_bim: &[],
                diff_deleted_ghosts: &[],
                replay_story_elevs_m: &[],
                replay_story_disp_xyz_m: &[],
                replay_ground_shake_xyz_m: [0.0; 3],
                replay_hinge_states: &[],
                shm_story_retention: &[],
            },
            &canvas.visibility_graphics.bim,
        )
        .indices
        .len();
        canvas.visibility_graphics.bim.show_columns = false;
        let no_col_tris = crate::views::webgpu_earth::build_rtc_active_bim_fem_mesh_with_visibility(
            &crate::views::webgpu_earth::ActiveBimGpuInput {
                project: &build.bim_bus.project,
                members: &members,
                lens: crate::views::universal_canvas::EarthBimLens::Clean,
                erection_day_limit: 90,
                deform_scale: 25.0,
                active_modal_mode: 0,
                modal_phase_sin: 0.0,
                n1_removed_bim: None,
                cascade_failed_bim: &[],
                cascade_step_progress: 0.0,
                subsurface_pile_depth_m: 6.0,
                heading_deg: 0.0,
                diff_overlay_active: false,
                diff_added_bim: &[],
                diff_modified_bim: &[],
                diff_deleted_ghosts: &[],
                replay_story_elevs_m: &[],
                replay_story_disp_xyz_m: &[],
                replay_ground_shake_xyz_m: [0.0; 3],
                replay_hinge_states: &[],
                shm_story_retention: &[],
            },
            &canvas.visibility_graphics.bim,
        )
        .indices
        .len();
        assert!(no_col_tris < all_tris, "Hiding Columns must reduce BIM triangle count");

        assert!(canvas.visibility_graphics.add_custom_layer("A-Furn", [200, 120, 255]));
        assert_eq!(canvas.visibility_graphics.active_cad_layer_id, "A-Furn");
        canvas.mep_overlay_visible = true;
        canvas.clash_overlay_visible = true;
        canvas.hide_all_engineering_overlays();
        assert!(!canvas.mep_overlay_visible && !canvas.clash_overlay_visible);

        // Roundtrip persistence inside .eigen.json
        let ws = crate::views::universal_canvas::WorkspaceCanvasSettings {
            theme_mode: crate::theme::ThemeMode::ProDark,
            canvas_height_ratio: 0.82,
            freeze_by_default: true,
            show_grid: true,
            snap_to_grid: true,
            grid_spacing_mm: 500.0,
            show_bim_base_layer: true,
            show_domain_overlay: true,
            sync_all_layers: true,
            show_floating_inspector: false,
            show_ai_copilot: false,
            unit_system: "SI".to_string(),
            design_standard: "SNI".to_string(),
            solver_threads: 4,
            enable_svd_guard: true,
            enable_anti_leakage: true,
        };
        let archive = canvas.build_unified_archive(&build.bim_bus.project, &members, &[], ws);
        let json = archive.to_pretty_json().expect("archive json");
        let mut canvas2 = UniversalCanvasState::default();
        let mut bim2 = BimProject::empty();
        let mut m2 = Vec::new();
        let mut v2 = Vec::new();
        canvas2
            .apply_unified_archive(&json, &mut bim2, &mut m2, &mut v2)
            .expect("apply roundtrip");
        assert!(!canvas2.visibility_graphics.bim.show_columns);
        assert!(canvas2
            .visibility_graphics
            .cad_layers
            .iter()
            .any(|l| l.id == "A-Furn"));

        // 3. Responsive single-capsule layout verification across Desktop, Tablet, and Mobile
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for (tab, open_props) in [(0usize, false), (1, false), (0, true)] {
            for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
                let ctx = egui::Context::default();
                spatial.mobile_deck_pinned = true;
                if open_props {
                    spatial.open_properties_drawer();
                    spatial.selected_target = build.bim_bus.project.columns.first().map(|c| SmartSelectionTarget::Column(c.id));
                } else {
                    spatial.selected_target = None;
                    spatial.open_project_browser_drawer(tab);
                }
                for _ in 0..4 {
                    let raw = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                        ..Default::default()
                    };
                    let mut out = ctx.run_ui(raw, |ui| {
                        let _ = spatial.render_hud_overlays(
                            ui.ctx(),
                            &theme,
                            &mut canvas,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    });
                    out.textures_delta.clear();
                }
                assert_eq!(spatial.hud_rects.len(), 1);
                assert!(
                    spatial.hud_rects[0].right() <= win_w + 1.0,
                    "Project Browser / Properties overflowed {win_w}: {:?}",
                    spatial.hud_rects[0]
                );
            }
        }
    }

    #[test]
    fn test_v0882_layer_lock_enforcement_dxf_export_and_drafting_ribbon() {
        // 1. Slash command & BIM lock helpers
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/export-dxf"));
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // Draw on default layer "0-Sketch", lock it, and verify translate/rotate/delete are blocked
        let eid = canvas.commit_parametric_2d_line(6.0, 0.0, "G", "Drafting");
        canvas.selected_entity_id = Some(eid);
        canvas
            .visibility_graphics
            .set_cad_layer_locked("0-Sketch", true);
        assert!(canvas.visibility_graphics.is_active_cad_layer_locked());
        assert!(!canvas.translate_selected_or_last(1000.0, 500.0));
        assert!(!canvas.rotate_selected_or_last(45.0));
        assert!(!canvas.delete_selected_or_last());
        assert_eq!(canvas.entities.len(), 1);

        // Unlock restores mutation cleanly
        canvas
            .visibility_graphics
            .set_cad_layer_locked("0-Sketch", false);
        assert!(canvas.translate_selected_or_last(1000.0, 500.0));

        // Add custom layer, draw entity, and test layer-aware DXF & SVG exports
        assert!(canvas
            .visibility_graphics
            .add_custom_layer("A-Furn", [200, 120, 255]));
        let _furn_id = canvas.commit_parametric_2d_rect(3.0, 2.0, "G", "Drafting");
        assert_eq!(canvas.entities.len(), 2);

        let full_dxf = canvas.export_cad_dxf(&build.bim_bus.project, "S-100", "Layer Plan");
        assert!(full_dxf.contains("A-Furn"));
        assert!(full_dxf.contains("S-COLS"));

        // Hide Columns BIM category and hide A-Furn CAD layer -> both omitted from DXF & SVG
        canvas.visibility_graphics.bim.show_columns = false;
        if let Some(l) = canvas
            .visibility_graphics
            .cad_layers
            .iter_mut()
            .find(|l| l.id == "A-Furn")
        {
            l.visible = false;
        }
        let filtered_dxf = canvas.export_cad_dxf(&build.bim_bus.project, "S-100", "Layer Plan");
        // Verify no ENTITIES polyline/circle/line on A-Furn (still declared in TABLES)
        let entities_sec = filtered_dxf
            .split("ENTITIES")
            .nth(1)
            .expect("ENTITIES section");
        assert!(!entities_sec.contains("\nA-Furn\n"));
        assert!(!entities_sec.contains("\nS-COLS\n"));

        let filtered_svg = canvas.export_svg(&build.bim_bus.project, NavigationPillar::ParametricDED);
        assert!(filtered_svg.contains(r#"data-layer="0-Sketch""#));
        assert!(!filtered_svg.contains(r#"data-layer="A-Furn""#));

        // 2. BIM category lock helper & Smart Inspector lock banner
        canvas
            .visibility_graphics
            .bim
            .set_bim_kind_locked(crate::core::BimElementKind::Column, true);
        assert!(canvas
            .visibility_graphics
            .bim
            .is_bim_kind_locked(crate::core::BimElementKind::Column));
        let col_id = build.bim_bus.project.columns[0].id;
        spatial.selected_target = Some(SmartSelectionTarget::Column(col_id));

        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        let ctx_lock = egui::Context::default();
        let mut out_lock = ctx_lock.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1280.0, 900.0),
                )),
                ..Default::default()
            },
            |ui| {
                let _ = spatial.render_hud_overlays(
                    ui.ctx(),
                    &theme,
                    &mut canvas,
                    &mut build,
                    &mut ai,
                    &members,
                    &projects,
                    &mut db_components,
                );
            },
        );
        out_lock.textures_delta.clear();

        // 3. Drafting ribbon with new "Layers" group fits cleanly without overflow across Desktop, Tablet, Mobile
        let act = spatial.execute_spatial_input(
            "/export-dxf",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act.export_cad_dxf);

        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            spatial.mobile_deck_pinned = true;
            spatial.close_focus_layer_return_to_ribbon();
            spatial.active_ribbon_tab = RibbonCategoryTab::Drafting;
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "Drafting Layers ribbon overflowed {win_w}: {:?}",
                spatial.hud_rects[0]
            );
        }
    }

    #[test]
    fn test_v0883_viewport_vg_picking_ifcx_filter_and_context_bar() {
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/export-vg-ifcx"));

        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // 1. VG BIM project filtering & reset_all
        let orig_cols = build.bim_bus.project.columns.len();
        assert!(orig_cols > 0);
        canvas.visibility_graphics.bim.show_columns = false;
        canvas.visibility_graphics.bim.lock_walls = true;
        let filtered = canvas
            .visibility_graphics
            .bim
            .filter_bim_project(&build.bim_bus.project);
        assert!(filtered.columns.is_empty());
        assert_eq!(filtered.walls.len(), build.bim_bus.project.walls.len());

        // 2. VG-filtered IFCX view export
        let vg_ifcx = canvas.export_vg_filtered_ifcx(&build.bim_bus.project);
        let parsed = crate::core::ifcx::IfcxDocument::parse(&vg_ifcx).expect("valid VG IFCX");
        assert!(vg_ifcx.contains("eigengraph::view::visibility_graphics"));
        assert!(
            parsed
                .data
                .iter()
                .all(|n| !n.path.contains("/columns/")),
            "Hidden Columns category must be omitted from VG-filtered IFCX export"
        );
        assert!(parsed.data.iter().any(|n| n.path.contains("/walls/")));

        let act = spatial.execute_spatial_input(
            "/export-vg-ifcx",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(act.export_vg_filtered_ifcx);

        // 3. Contextual VG Filter strip renders responsively with zero overflow across Desktop, Tablet, Mobile
        let theme = MacTheme::for_mode(crate::theme::ThemeMode::ProDark);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            spatial.mobile_deck_pinned = true;
            spatial.close_focus_layer_return_to_ribbon();
            spatial.active_ribbon_tab = RibbonCategoryTab::Modeling;
            for _ in 0..4 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme,
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "VG Filter contextual strip overflowed {win_w}: {:?}",
                spatial.hud_rects[0]
            );
        }

        canvas.visibility_graphics.bim.reset_all();
        assert!(canvas.visibility_graphics.bim.is_all_visible());
        assert!(!canvas.visibility_graphics.bim.any_locked());
    }

    #[test]
    fn test_v0884_gemini_tools_vg_cad_layers_and_prompt_telemetry() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        let mut ai = AiAssistantView::default();

        // 1. Registered tools & Gemini system prompt include #16 & #17 + initial telemetry.
        let prompt_before =
            spatial.build_gemini_copilot_system_prompt(&canvas, &build.bim_bus.project, &[]);
        assert!(prompt_before.contains("set_visibility_graphics"));
        assert!(prompt_before.contains("manage_cad_layers"));
        assert!(prompt_before.contains("VG: hidden=[none], locked=[none]"));
        assert!(prompt_before.contains("ActiveCADLayer=0-Sketch (vis=true, locked=false)"));

        // 2. Dispatch Tool #16 (hide beams & lock columns) + Tool #17 (add + lock custom layer)
        // via handle_gemini_reply without network key so it commits cleanly in-turn.
        spatial.gemini_api_key_draft.clear();
        let reply_mutate = crate::core::web_bridge::GeminiApiReply {
            request_id: 1,
            model: "gemini-2.5-flash".to_string(),
            ok: true,
            text: String::new(),
            function_calls: vec![
                crate::core::agent_tools::GeminiToolCall {
                    name: "set_visibility_graphics".to_string(),
                    args: serde_json::json!({
                        "mode": "set_category",
                        "category": "columns",
                        "locked": true,
                        "visible": true
                    }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "set_visibility_graphics".to_string(),
                    args: serde_json::json!({
                        "mode": "set_category",
                        "category": "beams",
                        "visible": false
                    }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "manage_cad_layers".to_string(),
                    args: serde_json::json!({
                        "mode": "add",
                        "layer_name": "S-FOUNDATION",
                        "color_hex": "#38BDF8"
                    }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "manage_cad_layers".to_string(),
                    args: serde_json::json!({
                        "mode": "configure",
                        "layer_name": "S-FOUNDATION",
                        "locked": true
                    }),
                },
            ],
            raw_model_content: None,
            error: None,
        };
        let _ = spatial.handle_gemini_reply(reply_mutate, &mut canvas, &mut build, &mut ai);
        assert!(canvas.visibility_graphics.bim.lock_columns);
        assert!(!canvas.visibility_graphics.bim.show_beams);
        assert_eq!(canvas.visibility_graphics.active_cad_layer_id, "S-FOUNDATION");
        assert!(canvas.visibility_graphics.is_cad_layer_locked("S-FOUNDATION"));

        let prompt_after =
            spatial.build_gemini_copilot_system_prompt(&canvas, &build.bim_bus.project, &[]);
        assert!(prompt_after.contains("VG: hidden=[Beams], locked=[Columns]"));
        assert!(prompt_after.contains("ActiveCADLayer=S-FOUNDATION (vis=true, locked=true)"));

        // 3. Export triggers & VG reset through native Gemini tools.
        let reply_export = crate::core::web_bridge::GeminiApiReply {
            request_id: 2,
            model: "gemini-2.5-flash".to_string(),
            ok: true,
            text: String::new(),
            function_calls: vec![
                crate::core::agent_tools::GeminiToolCall {
                    name: "set_visibility_graphics".to_string(),
                    args: serde_json::json!({ "mode": "export_vg_ifcx" }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "manage_cad_layers".to_string(),
                    args: serde_json::json!({ "mode": "export_dxf" }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "set_visibility_graphics".to_string(),
                    args: serde_json::json!({ "mode": "reset_all" }),
                },
            ],
            raw_model_content: None,
            error: None,
        };
        let out_export =
            spatial.handle_gemini_reply(reply_export, &mut canvas, &mut build, &mut ai);
        assert!(out_export.export_vg_filtered_ifcx);
        assert!(out_export.export_cad_dxf);
        assert!(canvas.visibility_graphics.bim.is_all_visible());
        assert!(!canvas.visibility_graphics.bim.any_locked());

        // 4. 3D Selected/Locked element badge smoke check (offscreen painter).
        if let Some(col) = build.bim_bus.project.columns.first() {
            canvas.globe_mode = true;
            canvas.world_map.set_center(
                canvas.ground_origin_lat_lon.0,
                canvas.ground_origin_lat_lon.1,
            );
            canvas.world_map.zoom_level = 16.5;
            canvas.highlighted_bim_element = Some((crate::core::BimElementKind::Column, col.id));
            canvas.visibility_graphics.bim.lock_columns = true;
            let ctx = egui::Context::default();
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 800.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 800.0));
                let xf = crate::core::gis::GlobeScreenTransform::from_view(
                    rect,
                    canvas.world_map.center_lat_lon.0,
                    canvas.world_map.center_lat_lon.1,
                    canvas.world_map.zoom_level,
                    &canvas.earth_world.camera,
                );
                canvas.paint_selected_element_badge_overlay(ui.painter(), &xf, &build.bim_bus.project);
            });
            out.textures_delta.clear();
        }
    }

    #[test]
    fn test_v0885_cad_bylayer_reassignment_delete_migration_and_model_tree_sync() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // 1. VisibilityGraphicsState ByLayer flag + JSON roundtrip + layer color cycle & isolate
        canvas.visibility_graphics.color_by_layer = true;
        let roundtrip_json = serde_json::to_string(&canvas.visibility_graphics).expect("serialize vg");
        let parsed_vg: crate::views::universal_canvas::VisibilityGraphicsState =
            serde_json::from_str(&roundtrip_json).expect("deserialize vg");
        assert!(parsed_vg.color_by_layer);

        let prev_col = canvas
            .visibility_graphics
            .layer_color_rgb("0-Sketch")
            .expect("base layer color");
        let next_col = canvas
            .visibility_graphics
            .cycle_layer_color("0-Sketch")
            .expect("cycled color");
        assert_ne!(prev_col, next_col);

        assert!(canvas
            .visibility_graphics
            .add_custom_layer("S-SHEAR", [168, 85, 247]));
        canvas.visibility_graphics.isolate_cad_layer("S-SHEAR");
        assert_eq!(canvas.visibility_graphics.active_cad_layer_id, "S-SHEAR");
        assert!(canvas.visibility_graphics.is_cad_layer_visible("S-SHEAR"));
        assert!(!canvas.visibility_graphics.is_cad_layer_visible("0-Sketch"));
        canvas.visibility_graphics.show_all_cad_layers();
        assert!(canvas.visibility_graphics.is_cad_layer_visible("0-Sketch"));

        // 2. Lock-guarded entity layer reassignment + ByLayer color sync
        let line_id = canvas.commit_parametric_2d_line(5.0, 0.0, "G", "Drafting");
        let rect_id = canvas.commit_parametric_2d_rect(4.0, 2.0, "G", "Drafting");
        assert_eq!(canvas.entities.len(), 2);

        canvas
            .visibility_graphics
            .set_cad_layer_locked("A-Anno", true);
        assert!(!canvas.reassign_entity_layer(line_id, "A-Anno"));
        canvas
            .visibility_graphics
            .set_cad_layer_locked("A-Anno", false);
        canvas
            .visibility_graphics
            .set_cad_layer_locked("S-SHEAR", true);
        assert!(!canvas.reassign_entity_layer(line_id, "S-SHEAR"));
        canvas
            .visibility_graphics
            .set_cad_layer_locked("S-SHEAR", false);
        assert!(canvas.reassign_entity_layer(line_id, "A-Anno"));
        let moved_ent = canvas
            .entities
            .iter()
            .find(|e| e.id == line_id)
            .expect("line present");
        assert_eq!(moved_ent.layer_id, "A-Anno");
        assert_eq!(
            Some(moved_ent.color_rgb),
            canvas.visibility_graphics.layer_color_rgb("A-Anno")
        );

        // 3. Delete custom layer protection & automatic entity migration to "0-Sketch"
        assert_eq!(canvas.delete_cad_layer_and_reassign_entities("0-Sketch"), None);
        canvas
            .visibility_graphics
            .set_cad_layer_locked("S-SHEAR", true);
        assert_eq!(canvas.delete_cad_layer_and_reassign_entities("S-SHEAR"), None);
        canvas
            .visibility_graphics
            .set_cad_layer_locked("S-SHEAR", false);
        let migrated = canvas
            .delete_cad_layer_and_reassign_entities("S-SHEAR")
            .expect("custom unlocked layer deletes cleanly");
        assert_eq!(migrated, 1);
        assert_eq!(
            canvas
                .entities
                .iter()
                .find(|e| e.id == rect_id)
                .map(|e| e.layer_id.as_str()),
            Some("0-Sketch")
        );
        assert_eq!(canvas.visibility_graphics.active_cad_layer_id, "0-Sketch");

        // 4. Dynamic Island responsive zero-overflow check for Model Tree + Layers/VG tabs
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            for tab_idx in [0usize, 1usize] {
                spatial.open_project_browser_drawer(tab_idx);
                let ctx = egui::Context::default();
                for _ in 0..2 {
                    let raw = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(win_w, win_h),
                        )),
                        ..Default::default()
                    };
                    let mut out = ctx.run_ui(raw, |ui| {
                        let _ = spatial.render_hud_overlays(
                            ui.ctx(),
                            &theme_helper_for_test(),
                            &mut canvas,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    });
                    out.textures_delta.clear();
                }
                assert_eq!(spatial.hud_rects.len(), 1);
                assert!(
                    spatial.hud_rects[0].right() <= win_w + 1.0,
                    "Dynamic Island overflow on tab {tab_idx} @ {win_w}px: {:?}",
                    spatial.hud_rects[0]
                );
            }
        }
    }

    fn theme_helper_for_test() -> crate::theme::MacTheme {
        crate::theme::MacTheme::for_mode(crate::theme::ThemeMode::ProDark)
    }

    #[test]
    fn test_v0886_cad_presets_svg_bylayer_and_extended_tool17() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // 1. AIA & ISO 13567 presets are non-destructive & idempotent.
        let aia_added_1 = canvas.visibility_graphics.apply_standard_preset(
            crate::views::universal_canvas::CadLayerStandardPreset::AiaStructural,
        );
        assert_eq!(aia_added_1, 9);
        assert_eq!(
            canvas.visibility_graphics.apply_standard_preset(
                crate::views::universal_canvas::CadLayerStandardPreset::AiaStructural
            ),
            0
        );
        let iso_added_1 = canvas.visibility_graphics.apply_standard_preset(
            crate::views::universal_canvas::CadLayerStandardPreset::Iso13567,
        );
        assert_eq!(iso_added_1, 8);
        assert_eq!(
            canvas.visibility_graphics.apply_standard_preset(
                crate::views::universal_canvas::CadLayerStandardPreset::Iso13567
            ),
            0
        );

        // 2. SVG export color parity under ByLayer = false vs true.
        let line_id = canvas.commit_parametric_2d_line(4.0, 0.0, "G", "Drafting");
        let layer_before = canvas
            .visibility_graphics
            .layer_color_rgb("0-Sketch")
            .expect("0-Sketch color");
        let cycled = canvas
            .visibility_graphics
            .cycle_layer_color("0-Sketch")
            .expect("cycle color");
        assert_ne!(layer_before, cycled);
        let ent_rgb = canvas
            .entities
            .iter()
            .find(|e| e.id == line_id)
            .expect("line entity")
            .color_rgb;
        assert_ne!(ent_rgb, cycled);

        canvas.visibility_graphics.color_by_layer = false;
        let svg_entity_col = canvas.export_svg(
            &build.bim_bus.project,
            crate::models::NavigationPillar::ParametricDED,
        );
        assert!(svg_entity_col.contains(&format!(
            "rgb({},{},{})",
            ent_rgb[0], ent_rgb[1], ent_rgb[2]
        )));

        canvas.visibility_graphics.color_by_layer = true;
        let svg_bylayer_col = canvas.export_svg(
            &build.bim_bus.project,
            crate::models::NavigationPillar::ParametricDED,
        );
        assert!(svg_bylayer_col.contains(&format!(
            "rgb({},{},{})",
            cycled[0], cycled[1], cycled[2]
        )));

        // 3. Extended Tool #17 modes via Gemini Copilot dispatch & telemetry.
        let mut tool_canvas = UniversalCanvasState::default();
        spatial.gemini_api_key_draft.clear();
        let reply = crate::core::web_bridge::GeminiApiReply {
            request_id: 6,
            model: "gemini-2.5-flash".to_string(),
            ok: true,
            text: String::new(),
            function_calls: vec![
                crate::core::agent_tools::GeminiToolCall {
                    name: "manage_cad_layers".to_string(),
                    args: serde_json::json!({ "mode": "apply_preset", "preset": "aia" }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "manage_cad_layers".to_string(),
                    args: serde_json::json!({ "mode": "set_by_layer", "by_layer": true }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "manage_cad_layers".to_string(),
                    args: serde_json::json!({ "mode": "isolate", "layer_name": "S-COLS" }),
                },
            ],
            raw_model_content: None,
            error: None,
        };
        let _ = spatial.handle_gemini_reply(reply, &mut tool_canvas, &mut build, &mut ai);
        assert!(tool_canvas.visibility_graphics.color_by_layer);
        assert_eq!(tool_canvas.visibility_graphics.active_cad_layer_id, "S-COLS");
        assert!(tool_canvas.visibility_graphics.is_cad_layer_visible("S-COLS"));
        assert!(!tool_canvas.visibility_graphics.is_cad_layer_visible("0-Sketch"));

        let prompt =
            spatial.build_gemini_copilot_system_prompt(&tool_canvas, &build.bim_bus.project, &[]);
        assert!(prompt.contains("ByLayer=true"));

        let reply_cleanup = crate::core::web_bridge::GeminiApiReply {
            request_id: 7,
            model: "gemini-2.5-flash".to_string(),
            ok: true,
            text: String::new(),
            function_calls: vec![
                crate::core::agent_tools::GeminiToolCall {
                    name: "manage_cad_layers".to_string(),
                    args: serde_json::json!({ "mode": "show_all" }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "manage_cad_layers".to_string(),
                    args: serde_json::json!({ "mode": "delete", "layer_name": "S-REBR" }),
                },
            ],
            raw_model_content: None,
            error: None,
        };
        let _ = spatial.handle_gemini_reply(reply_cleanup, &mut tool_canvas, &mut build, &mut ai);
        assert!(tool_canvas.visibility_graphics.is_cad_layer_visible("0-Sketch"));
        assert!(!tool_canvas
            .visibility_graphics
            .cad_layers
            .iter()
            .any(|l| l.id == "S-REBR"));

        // 4. Responsive Drafting Ribbon + Layers/VG zero-overflow check.
        spatial.mobile_deck_pinned = true;
        spatial.close_focus_layer_return_to_ribbon();
        spatial.active_ribbon_tab = RibbonCategoryTab::Drafting;
        spatial.open_project_browser_drawer(1);
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            for _ in 0..2 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme_helper_for_test(),
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "Drafting + Layers/VG overflow @ {win_w}px: {:?}",
                spatial.hud_rects[0]
            );
        }
    }

    #[test]
    fn test_v0887_hide_floating_checkmark_dxf_bylayer_aci_and_cad_slash_commands() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // 1. Resting Dynamic Island height is identical with vs without trace chips (no floating checkmark badge outside).
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            spatial.recent_trace_chips.clear();
            let ctx_empty = egui::Context::default();
            for _ in 0..3 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx_empty.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme_helper_for_test(),
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            let h_without_trace = spatial.hud_rects[0].height();

            spatial.push_trace("Model verified cleanly", true);
            let ctx_trace = egui::Context::default();
            for _ in 0..3 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(win_w, win_h))),
                    ..Default::default()
                };
                let mut out = ctx_trace.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme_helper_for_test(),
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(spatial.hud_rects[0].right() <= win_w + 1.0);
            let h_with_trace = spatial.hud_rects[0].height();
            assert!(
                (h_with_trace - h_without_trace).abs() < 0.5,
                "Floating trace row changed Dynamic Island height @ {win_w}px: without={h_without_trace:.1}, with={h_with_trace:.1}",
            );
        }

        // 2. Slash commands (/bylayer, /preset-aia, /preset-iso) registered & functional.
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/bylayer"));
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/preset-aia"));
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/preset-iso"));

        let _ = spatial.execute_spatial_input(
            "/preset-aia",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas
            .visibility_graphics
            .cad_layers
            .iter()
            .any(|l| l.id == "S-REBR"));
        assert_eq!(spatial.project_browser_tab, 1);

        let _ = spatial.execute_spatial_input(
            "/bylayer",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas.visibility_graphics.color_by_layer);

        let _ = spatial.execute_spatial_input(
            "/preset-iso",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(canvas
            .visibility_graphics
            .cad_layers
            .iter()
            .any(|l| l.id == "S-28-COLS"));

        // 3. AutoCAD AC1015 DXF export emits BYLAYER ACI 256 when color_by_layer is true,
        //    and explicit entity ACI when color_by_layer is false.
        let _line_id = canvas.commit_parametric_2d_line(6.0, 0.0, "G", "Drafting");
        canvas.visibility_graphics.color_by_layer = true;
        let dxf_bylayer = canvas.export_cad_dxf(&build.bim_bus.project, "S-100", "ByLayer DXF");
        let entities_bylayer = dxf_bylayer.split("ENTITIES").nth(1).unwrap_or("");
        assert!(entities_bylayer.contains("0\nLINE\n8\n0-Sketch\n62\n256\n"));

        canvas.visibility_graphics.color_by_layer = false;
        let dxf_explicit = canvas.export_cad_dxf(&build.bim_bus.project, "S-100", "Explicit DXF");
        let entities_explicit = dxf_explicit.split("ENTITIES").nth(1).unwrap_or("");
        assert!(!entities_explicit.contains("62\n256\n"));
        assert!(entities_explicit.contains("0\nLINE\n8\n0-Sketch\n62\n"));
    }

    #[test]
    fn test_v0888_dynamic_island_uniform_margins_and_zero_overflow() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            // 1. Resting Dynamic Island
            spatial.close_focus_layer_return_to_ribbon();
            spatial.project_browser_drawer_open = false;
            spatial.properties_drawer_open = false;
            spatial.mobile_deck_pinned = false;
            for tab in [0usize, 1usize] {
                if tab == 1 {
                    spatial.open_project_browser_drawer(1);
                }
                let ctx = egui::Context::default();
                for _ in 0..3 {
                    let raw = egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(win_w, win_h),
                        )),
                        ..Default::default()
                    };
                    let mut out = ctx.run_ui(raw, |ui| {
                        let _ = spatial.render_hud_overlays(
                            ui.ctx(),
                            &theme_helper_for_test(),
                            &mut canvas,
                            &mut build,
                            &mut ai,
                            &members,
                            &projects,
                            &mut db_components,
                        );
                    });
                    out.textures_delta.clear();
                }
                assert_eq!(spatial.hud_rects.len(), 1);
                assert!(
                    spatial.hud_rects[0].right() <= win_w + 1.0,
                    "Dynamic Island overflow with uniform padding @ {win_w}px (tab={tab}): {:?}",
                    spatial.hud_rects[0]
                );
            }
        }
    }

    #[test]
    fn test_v0889_eigenflow_cascade_tab_retrofit_advisor_and_slash_commands() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();
        let members = Vec::new();
        let projects = Vec::new();
        let mut db_components = Vec::new();

        // 1. Slash commands /eigenflow-cascade & /retrofit-advisor registered and populate live cascade + sensitivity state
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/eigenflow-cascade"));
        assert!(SLASH_COMMANDS.iter().any(|s| s.cmd == "/retrofit-advisor"));

        let _ = spatial.execute_spatial_input(
            "/eigenflow-cascade",
            &mut canvas,
            &mut build,
            &mut ai,
            &members,
            &projects,
            &mut db_components,
        );
        assert!(spatial.eigenflow_drawer_open);
        assert_eq!(spatial.eigenflow_drawer_tab, 2);
        assert!(canvas.eigenflow_overlay_visible);
        let ef_rep = canvas
            .last_eigenflow_report
            .as_ref()
            .expect("EigenFlow report populated");
        assert!(!ef_rep.cascade.waves.is_empty());
        let sens_rep = canvas
            .last_eigengate_sensitivity
            .as_ref()
            .expect("EigenGate sensitivity populated");
        assert!(!sens_rep.candidates.is_empty());

        let mesh = crate::views::webgpu_earth::build_rtc_eigenflow_mesh(
            &crate::views::webgpu_earth::EigenFlowGpuInput {
                report: ef_rep,
                active_step_idx: 0,
                active_cascade_wave: ef_rep.cascade.waves.len().saturating_sub(1),
                heading_deg: 0.0,
                visible: true,
            },
        );
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 2. Responsive Cascade & Retrofit Studio Tab 2 zero-overflow check across Desktop, Tablet & Mobile
        for (win_w, win_h) in [(1440.0_f32, 900.0_f32), (768.0, 900.0), (390.0, 844.0)] {
            let ctx = egui::Context::default();
            for _ in 0..2 {
                let raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(win_w, win_h),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    let _ = spatial.render_hud_overlays(
                        ui.ctx(),
                        &theme_helper_for_test(),
                        &mut canvas,
                        &mut build,
                        &mut ai,
                        &members,
                        &projects,
                        &mut db_components,
                    );
                });
                out.textures_delta.clear();
            }
            assert_eq!(spatial.hud_rects.len(), 1);
            assert!(
                spatial.hud_rects[0].right() <= win_w + 1.0,
                "EigenFlow Cascade & Retrofit Tab 2 overflow @ {win_w}px: {:?}",
                spatial.hud_rects[0]
            );
        }
    }

    #[test]
    fn test_v089_release_tools18_19_cascade_ifcx_s104_and_copilot_telemetry() {
        let mut spatial = SpatialWorkspaceState::default();
        let mut canvas = UniversalCanvasState::default();
        let mut build = BuildView::default();
        build.bim_bus =
            crate::core::BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let mut ai = AiAssistantView::default();

        // 1. Dispatch Tools #18 & #19 through handle_gemini_reply cleanly
        spatial.gemini_api_key_draft.clear();
        let reply = crate::core::web_bridge::GeminiApiReply {
            request_id: 10,
            model: "gemini-2.5-flash".to_string(),
            ok: true,
            text: String::new(),
            function_calls: vec![
                crate::core::agent_tools::GeminiToolCall {
                    name: "run_eigengate_n2_sensitivity".to_string(),
                    args: serde_json::json!({ "open_studio": true }),
                },
                crate::core::agent_tools::GeminiToolCall {
                    name: "run_eigenflow_cascade".to_string(),
                    args: serde_json::json!({ "lambda": 0.0, "show_3d": true }),
                },
            ],
            raw_model_content: None,
            error: None,
        };
        let _ = spatial.handle_gemini_reply(reply, &mut canvas, &mut build, &mut ai);
        assert!(spatial.eigenflow_drawer_open);
        assert_eq!(spatial.eigenflow_drawer_tab, 2);
        assert!(canvas.eigenflow_overlay_visible);

        let ef_rep = canvas
            .last_eigenflow_report
            .as_ref()
            .expect("EigenFlow studio report after Tool #19");
        assert!(!ef_rep.cascade.waves.is_empty());

        // 2. Copilot system prompt enumerates Tools #18-#19 and includes Cascade + Sensitivity telemetry
        let sys_prompt =
            spatial.build_gemini_copilot_system_prompt(&canvas, &build.bim_bus.project, &[]);
        assert!(sys_prompt.contains("run_eigengate_n2_sensitivity"));
        assert!(sys_prompt.contains("run_eigenflow_cascade"));
        assert!(sys_prompt.contains("EigenFlow Cascade:"));
        assert!(sys_prompt.contains("EigenGate Sensitivity:"));

        // 3. EigenFlow IFCX overlay carries cascade telemetry without exposing forbidden "ARI" label
        let overlay = ef_rep.to_ifcx_overlay();
        assert_eq!(overlay.cascade_waves_count, ef_rep.cascade.waves.len());
        assert_eq!(overlay.cascade_failed_elements, ef_rep.cascade.total_failed_elements);
        let overlay_json = serde_json::to_string(&overlay).expect("serialize overlay");
        assert!(!overlay_json.contains("ARI"));

        // 4. Archival Sheet S-104 SVG includes cascade summary verdict
        let s104 = crate::core::vector_dossier::build_eigenflow_nonlinear_annex_svg(
            &build.bim_bus.project.project_name,
            ef_rep,
            "S-104",
            None,
        );
        assert!(s104.svg_content.contains("CASCADE VERDICT"));
        assert!(s104.svg_content.contains("UFC DIF="));
    }
