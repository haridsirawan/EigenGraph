use super::*;
use crate::theme::ThemeMode;

    fn sample_settings() -> WorkspaceCanvasSettings {
        WorkspaceCanvasSettings {
            theme_mode: ThemeMode::ProDark,
            canvas_height_ratio: 0.74,
            freeze_by_default: true,
            show_grid: true,
            snap_to_grid: true,
            grid_spacing_mm: 500.0,
            show_bim_base_layer: true,
            show_domain_overlay: true,
            sync_all_layers: true,
            show_floating_inspector: false,
            show_ai_copilot: true,
            unit_system: "kN, mm, MPa (Structural Metric)".to_string(),
            design_standard: "SNI 1729 / AISC 360-16".to_string(),
            solver_threads: 8,
            enable_svd_guard: true,
            enable_anti_leakage: true,
        }
    }

    #[test]
    fn test_unified_single_file_archive_roundtrip_and_tamper_detection() {
        let canvas = UniversalCanvasState::default();
        assert!(canvas.is_frozen);
        assert!(canvas.entities.is_empty());
        assert!(canvas.world_map.zoom_level < 3.0);

        let bim = BimProject::sample_bimcraft_building();
        let members = vec![StructuralMember::default()];
        let vars = vec![ParametricVariable {
            name: "L".to_string(),
            expression: "24000".to_string(),
            evaluated_value: 24000.0,
            unit: "mm".to_string(),
        }];

        let archive = canvas.build_unified_archive(&bim, &members, &vars, sample_settings());
        let json = archive.to_pretty_json().expect("serialize archive");
        let verified = UnifiedWorkspaceArchive::from_verified_json(&json).expect("verify archive");
        assert_eq!(verified.canvas_entities.len(), 0);
        assert_eq!(verified.members.len(), 1);
        assert_eq!(verified.bim_project.walls.len(), bim.walls.len());

        // Tampering with JSON content must fail SHA-256 verification
        let tampered = json.replace("24000", "99999");
        assert!(UnifiedWorkspaceArchive::from_verified_json(&tampered).is_err());
    }

    #[test]
    fn test_canvas_sync_across_pillars_undo_and_svg_export() {
        let mut canvas = UniversalCanvasState::default();
        let initial_len = canvas.entities.len();
        canvas.push_undo();
        canvas.entities.push(CanvasEntity {
            id: 99,
            pillar_badge: NavigationPillar::SafetyHealth.badge().to_string(),
            pillar_name: NavigationPillar::SafetyHealth.clean_name().to_string(),
            kind: CanvasShapeKind::Circle {
                center_mm: [6000.0, 4500.0],
                radius_mm: 3000.0,
            },
            color_rgb: [245, 158, 11],
            stroke_width: 2.5,
        });
        assert_eq!(canvas.count_for_pillar(NavigationPillar::SafetyHealth), 1);
        assert_eq!(canvas.entities.len(), initial_len + 1);

        let bim = BimProject::sample_bimcraft_building();
        let svg = canvas.export_svg(&bim, NavigationPillar::SafetyHealth);
        assert!(svg.contains("<circle"));

        assert!(canvas.undo());
        assert_eq!(canvas.entities.len(), initial_len);
        assert!(canvas.redo());
        assert_eq!(canvas.entities.len(), initial_len + 1);
    }

    #[test]
    fn test_canvas_ortho_translate_offset_mirror_and_delete() {
        let mut canvas = UniversalCanvasState::default();
        canvas.ortho_mode = true;
        assert_eq!(
            canvas.apply_ortho([1000.0, 1000.0], [4500.0, 1200.0]),
            [4500.0, 1000.0]
        );
        assert_eq!(
            canvas.apply_ortho([1000.0, 1000.0], [1150.0, 3800.0]),
            [1000.0, 3800.0]
        );

        canvas.entities.push(CanvasEntity {
            id: 1,
            pillar_badge: "3D".to_string(),
            pillar_name: "Geometry".to_string(),
            kind: CanvasShapeKind::Line {
                start_mm: [0.0, 0.0],
                end_mm: [3000.0, 0.0],
            },
            color_rgb: [56, 189, 248],
            stroke_width: 2.0,
        });
        canvas.next_entity_id = 2;
        canvas.selected_entity_id = Some(1);

        assert!(canvas.translate_selected_or_last(1000.0, 500.0));
        match &canvas.entities[0].kind {
            CanvasShapeKind::Line { start_mm, end_mm } => {
                assert_eq!(*start_mm, [1000.0, 500.0]);
                assert_eq!(*end_mm, [4000.0, 500.0]);
            }
            _ => panic!("expected line"),
        }

        assert!(canvas.offset_selected_or_last(500.0));
        assert_eq!(canvas.entities.len(), 2);
        assert!(canvas.mirror_selected_or_last(6000.0));
        assert_eq!(canvas.entities.len(), 3);
        assert!(canvas.delete_selected_or_last());
        assert_eq!(canvas.entities.len(), 2);
        assert!(canvas.undo());
        assert_eq!(canvas.entities.len(), 3);
    }

    #[test]
    fn test_canvas_precision_polar_relative_duplicate_and_rotate() {
        let mut canvas = UniversalCanvasState::default();
        let mut bim = BimProject::sample_bimcraft_building();

        let (dx, dy) = UniversalCanvasState::parse_cad_delta_expr("@5000<0").expect("polar 0 deg");
        assert!((dx - 5000.0).abs() < 1e-6);
        assert!(dy.abs() < 1e-6);

        let (dx2, dy2) =
            UniversalCanvasState::parse_cad_delta_expr("@1200,-600").expect("relative coords");
        assert_eq!((dx2, dy2), (1200.0, -600.0));

        let msg = canvas.execute_precision_command(
            "@4000<0",
            NavigationPillar::ResearchConcept,
            &mut bim,
        );
        assert!(msg.contains("Drew relative segment"));
        assert_eq!(canvas.entities.len(), 1);

        assert!(canvas.rotate_selected_or_last(90.0));
        match &canvas.entities[0].kind {
            CanvasShapeKind::Line { start_mm, end_mm } => {
                assert!((end_mm[1] - start_mm[1] - 4000.0).abs() < 1e-4);
            }
            _ => panic!("expected rotated line"),
        }

        assert!(canvas.duplicate_selected_or_last(1500.0, 0.0));
        assert_eq!(canvas.entities.len(), 2);
    }

    #[test]
    fn test_canvas_osnap_marquee_grid_array_and_promote_to_bim() {
        let mut canvas = UniversalCanvasState::default();
        let mut bim = BimProject::empty();

        // 1. Structural Grid Generation & OSNAP Intersection + Endpoint
        let created = canvas.generate_structural_grid(
            3,
            2,
            6000.0,
            5000.0,
            NavigationPillar::ParametricDED,
            [0.0, 0.0],
        );
        assert_eq!(created, 5); // 3 vertical + 2 horizontal axes

        let (ix0_pt, ix0_kind) =
            canvas.snap_point_with_osnap([40.0, 35.0], None, None, 200.0);
        assert!((ix0_pt[0]).abs() < 1e-3 && (ix0_pt[1]).abs() < 1e-3);
        assert_eq!(ix0_kind, OsnapKind::Intersection);

        let (mid_pt, mid_kind) =
            canvas.snap_point_with_osnap([25.0, 2520.0], None, None, 200.0);
        assert!(mid_pt[0].abs() < 1e-3 && (mid_pt[1] - 2500.0).abs() < 1e-3);
        assert_eq!(mid_kind, OsnapKind::Midpoint);

        let (perp_pt, perp_kind) = canvas.snap_point_with_osnap(
            [35.0, 1210.0],
            Some([3000.0, 1200.0]),
            None,
            200.0,
        );
        assert!(perp_pt[0].abs() < 1e-3 && (perp_pt[1] - 1200.0).abs() < 1e-3);
        assert_eq!(perp_kind, OsnapKind::Perpendicular);

        let (ix_pt, ix_kind) =
            canvas.snap_point_with_osnap([6030.0, 4985.0], None, None, 200.0);
        assert!((ix_pt[0] - 6000.0).abs() < 1e-3 && (ix_pt[1] - 5000.0).abs() < 1e-3);
        assert!(matches!(ix_kind, OsnapKind::Endpoint | OsnapKind::Intersection));

        // Standalone segment endpoint verifies pure Endpoint OSNAP without crossing axes
        canvas.entities.push(CanvasEntity {
            id: 89,
            pillar_badge: "P4".to_string(),
            pillar_name: "Parametric".to_string(),
            kind: CanvasShapeKind::Line {
                start_mm: [20000.0, 0.0],
                end_mm: [24000.0, 0.0],
            },
            color_rgb: [56, 189, 248],
            stroke_width: 2.0,
        });
        let (pure_end_pt, pure_end_kind) =
            canvas.snap_point_with_osnap([23980.0, 15.0], None, None, 200.0);
        assert!((pure_end_pt[0] - 24000.0).abs() < 1e-3 && pure_end_pt[1].abs() < 1e-3);
        assert_eq!(pure_end_kind, OsnapKind::Endpoint);
        canvas.entities.pop();

        // 2. Marquee Window (L->R) vs Crossing (R->L) selection + Multi-entity Array & Promote
        canvas.entities.push(CanvasEntity {
            id: 90,
            pillar_badge: "P4".to_string(),
            pillar_name: "Parametric".to_string(),
            kind: CanvasShapeKind::Rectangle {
                min_mm: [0.0, 0.0],
                max_mm: [6000.0, 5000.0],
            },
            color_rgb: [56, 189, 248],
            stroke_width: 2.0,
        });
        canvas.next_entity_id = 91;

        // Small Window marquee excluding the 6000x5000 rectangle should find 0 full-enclosed boxes inside [100,100]->[1000,1000]
        let win_cnt = canvas.select_marquee([100.0, 100.0], [1000.0, 1000.0]);
        assert_eq!(win_cnt, 0);

        // Crossing (Right->Left) over the edge intersects rectangle & lines
        let cross_cnt = canvas.select_marquee([1000.0, 1000.0], [-50.0, -50.0]);
        assert!(cross_cnt >= 3);

        // Window selection enclosing [0,0]->[6000,5000] excludes 12m horizontal axes (3 enclosed)
        assert_eq!(canvas.select_marquee([-50.0, -50.0], [6050.0, 5050.0]), 3);
        // Full Window selection enclosing all 5 grid axes + rectangle (6 entities)
        let full_win = canvas.select_marquee([-50.0, -50.0], [12050.0, 5050.0]);
        assert_eq!(full_win, 6);

        // Promote selection into 3D BIM Walls, Beams, Columns & Slabs
        let msg = canvas.execute_precision_command(
            "promote",
            NavigationPillar::GeometryAssembly,
            &mut bim,
        );
        assert!(msg.contains("Walls"));
        assert!(!bim.walls.is_empty());
        assert!(!bim.slabs.is_empty());
        assert!(!bim.columns.is_empty());

        // Rectangular ARRAY command
        let arr_msg = canvas.execute_precision_command(
            "array 2x2 7000 6000",
            NavigationPillar::GeometryAssembly,
            &mut bim,
        );
        assert!(arr_msg.contains("Arrayed 2x2"));
    }

    #[test]
    fn test_phase43_default_3d_google_earth_mode_and_archive_roundtrip() {
        use crate::core::gis::{EarthBrushAction, MegastructureKind, VectorEarthTheme};

        let mut canvas = UniversalCanvasState::default();
        assert!(canvas.globe_mode);
        assert_eq!(canvas.world_map.tile_provider, VectorEarthTheme::CoastalBlueprint);
        assert!(canvas.world_map.zoom_level < 3.0);

        let before_co2 = canvas.earth_world.vitals.co2_ppm;
        let touched = canvas
            .earth_world
            .apply_brush(-1.0, 114.0, EarthBrushAction::PlantForest, true);
        assert!(touched > 0);
        assert!(canvas.earth_world.vitals.co2_ppm < before_co2);

        let id = canvas.earth_world.add_megastructure(
            MegastructureKind::TransStraitBridge,
            (-5.9, 105.9),
            (-6.1, 106.1),
        );
        assert!(id >= 1);
        assert_eq!(canvas.earth_world.megastructures.len(), 1);

        canvas.globe_mode = false;
        let bim = BimProject::sample_bimcraft_building();
        let members = vec![StructuralMember::default()];
        let vars = Vec::new();
        let archive = canvas.build_unified_archive(&bim, &members, &vars, sample_settings());
        let json = archive.to_pretty_json().expect("serialize archive");
        let _verified = UnifiedWorkspaceArchive::from_verified_json(&json).expect("verify archive");

        let mut restored = UniversalCanvasState::default();
        let mut out_bim = BimProject::default();
        let mut out_members = Vec::new();
        let mut out_vars = Vec::new();
        let out_settings = restored
            .apply_unified_archive(&json, &mut out_bim, &mut out_members, &mut out_vars)
            .expect("restore unified archive");

        assert!(!restored.globe_mode);
        assert_eq!(out_settings.solver_threads, sample_settings().solver_threads);
        assert_eq!(restored.earth_world.cells.len(), canvas.earth_world.cells.len());
        assert_eq!(restored.earth_world.megastructures.len(), 1);
        assert!((restored.earth_world.vitals.co2_ppm - canvas.earth_world.vitals.co2_ppm).abs() < 1e-6);
    }

    #[test]
    fn test_phase43_3d_globe_projection_and_responsive_viewport() {
        use crate::core::gis::{EarthBrushAction, EarthScenario, MegastructureKind};

        let mut canvas = UniversalCanvasState::default();
        let (slat, slon, szoom) = canvas
            .earth_world
            .load_scenario(EarthScenario::CoastalDeltaDefense);
        canvas.world_map.zoom_to_ground_site(slat, slon, szoom);
        canvas.ground_origin_lat_lon = (slat, slon);
        canvas.ground_origin_anchored = true;
        canvas
            .earth_world
            .apply_brush(slat, slon, EarthBrushAction::BuildSeawall, true);
        canvas.earth_world.add_megastructure(
            MegastructureKind::SolarCleanCorridor,
            (slat, slon),
            (slat + 0.8, slon + 1.2),
        );

        let mut bim = BimProject::sample_bimcraft_building();
        let mut members = vec![StructuralMember::default()];
        let mut vars = Vec::new();
        let theme = MacTheme::for_mode(ThemeMode::ProDark);
        let ws = sample_settings();

        for w in [1000.0_f32, 700.0, 390.0] {
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, 820.0))),
                ..Default::default()
            };
            let mut max_right = 0.0_f32;
            let mut panel_right = 0.0_f32;
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    panel_right = ui.max_rect().right();
                    let resp = ui.scope(|ui| {
                        canvas.show(
                            ui,
                            &theme,
                            NavigationPillar::GeometryAssembly,
                            &mut bim,
                            &mut members,
                            &mut vars,
                            ws.clone(),
                            0.0,
                        );
                    });
                    max_right = resp.response.rect.right();
                });
            });
            out.textures_delta.clear();
            assert!(canvas.last_globe_xform.is_some());
            assert!(
                max_right <= panel_right + 1.0,
                "Overflow at width {w}: content right {max_right} > panel right {panel_right}"
            );
        }

        let rect = canvas.last_canvas_rect.expect("canvas rect");
        let p = canvas.world_to_screen(rect, [0.0, 0.0]);
        assert!(rect.expand(40.0).contains(p));
        let back_mm = canvas.screen_to_world(rect, p);
        assert!(back_mm[0].abs() < 25_000_000.0 && back_mm[1].abs() < 25_000_000.0);
    }

    #[test]
    fn test_phase45_lifecycle_archive_roundtrip() {
        let mut canvas = UniversalCanvasState::default();
        canvas.ground_origin_lat_lon = (-6.12, 106.85);
        canvas.ground_origin_anchored = true;
        canvas.lifecycle_ops.mep_sizing.floor_area_m2 = 2450.0;
        canvas.lifecycle_ops.procurement.retention_pct = 7.5;
        canvas
            .lifecycle_ops
            .daily_diary
            .entries
            .push(crate::core::DailyDiaryEntry {
                day_label: "Day 45 (Thu)".to_string(),
                weather: "Clear".to_string(),
                temp_c: 32.0,
                workable_hours: 9.0,
                carpentry_crew: 22,
                rebar_crew: 18,
                concrete_crew: 16,
                mep_crew: 14,
                crane_util_pct: 91.0,
                pump_m3_poured: 140.0,
                incident_free: true,
                remarks: "Transfer deck pour complete".to_string(),
            });
        canvas.lifecycle_ops.qa_qc.ncrs[0].closed = true;

        let bim = BimProject::sample_bimcraft_building();
        let members = vec![StructuralMember::default()];
        let vars = Vec::new();
        let archive = canvas.build_unified_archive(&bim, &members, &vars, sample_settings());
        let json = archive.to_pretty_json().expect("serialize");

        let mut restored = UniversalCanvasState::default();
        let mut out_bim = BimProject::default();
        let mut out_members = Vec::new();
        let mut out_vars = Vec::new();
        let _ = restored
            .apply_unified_archive(&json, &mut out_bim, &mut out_members, &mut out_vars)
            .expect("restore");

        assert!((restored.lifecycle_ops.mep_sizing.floor_area_m2 - 2450.0).abs() < 1e-6);
        assert!((restored.lifecycle_ops.procurement.retention_pct - 7.5).abs() < 1e-6);
        assert_eq!(restored.lifecycle_ops.daily_diary.entries.len(), 4);
        assert!(restored.lifecycle_ops.qa_qc.ncrs[0].closed);
        assert!(restored.lifecycle_ops.site_coupling.seismic_pga_ss_g >= 0.85);
    }

    #[test]
    fn test_phase47_earth_zoom_drag_sensitivity_and_cursor_lock() {
        let mut canvas = UniversalCanvasState::default();
        canvas.globe_mode = true;
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
        canvas.world_map.set_center(-6.2088, 106.8456);
        canvas.world_map.zoom_level = 16.0;
        canvas.last_globe_xform = Some(GlobeScreenTransform::from_view(
            rect,
            canvas.world_map.center_lat_lon.0,
            canvas.world_map.center_lat_lon.1,
            canvas.world_map.zoom_level,
            &canvas.earth_world.camera,
        ));

        // 1. At zoom=16.0, dragging right by 20px keeps ground movement ~20px on screen (no oversensitivity)
        let focus = Pos2::new(620.0, 360.0);
        let (pin_lat, pin_lon, hit_pin) = canvas.screen_to_lat_lon_with_hit(focus, rect);
        assert!(hit_pin);
        canvas.orbit_globe_by_pixels(rect, Vec2::new(20.0, 0.0), false, 0.016);
        // Inertia disabled above zoom 11.0 so site editing never slides after pointer release
        assert_eq!(canvas.earth_world.camera.angular_vel_lat, 0.0);
        assert_eq!(canvas.earth_world.camera.angular_vel_lon, 0.0);

        let moved_xf = canvas.last_globe_xform.as_ref().unwrap();
        let (after_drag_px, _, _) = moved_xf
            .project_lat_lon_elev(pin_lat, pin_lon, 0.0)
            .unwrap();
        let shift_x = (after_drag_px.x - focus.x) as f64;
        assert!(
            (shift_x - 20.0).abs() < 2.0,
            "20px globe drag at z=16.0 shifted point by {shift_x:.2}px instead of ~20px"
        );

        // 2. Cursor-anchored zoom at z=16.0 keeps the hovered Earth point under `focus`
        let (hover_lat, hover_lon, _) = canvas.screen_to_lat_lon_with_hit(focus, rect);
        canvas.apply_globe_zoom_at_screen(rect, 0.25, Some(focus));
        let post_xf = canvas.last_globe_xform.as_ref().unwrap();
        let (reproj, _, _) = post_xf
            .project_lat_lon_elev(hover_lat, hover_lon, 0.0)
            .unwrap();
        let lock_err = (reproj - focus).length();
        assert!(
            lock_err < 2.0,
            "Cursor-anchored Earth zoom drifted hover point by {lock_err:.2}px"
        );
    }
