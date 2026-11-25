use super::*;
use crate::core::bim::{BimElementKind, BimProject};
use crate::core::gis::city_builder::IfcxZoneKind;
use crate::core::gis::district_hazard::DistrictHazardConfig;
use crate::core::gis::neighborhood_physics::{NeighborhoodAuditReport, PoundingSeverity};
use crate::core::gis::world_index::{GeoPlacementSource, WorldBuildingProxy};
use crate::core::gis::DemTerrainPreset;
use crate::core::ifcx::IfcxDocument;

    #[test]
    fn test_phase65_rtc_mvp_matches_globe_screen_transform_subpixel() {
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(1280.0, 720.0));
        let cam = GlobeCamera {
            tilt_deg: 38.0,
            heading_deg: 27.0,
            ..GlobeCamera::default()
        };

        // 1. Orbital zoom planetary matrix parity
        let xf_orb = GlobeScreenTransform::from_view(rect, -6.2088, 106.8456, 2.4, &cam);
        let u_orb = RtcCameraUniforms::from_globe_xform(
            &xf_orb,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.0,
            false,
        );
        for &(lat, lon) in &[(-6.2088, 106.8456), (-7.7956, 110.3695), (-0.9730, 116.7090)] {
            let (cpu_sp, _, front) = xf_orb.project_lat_lon_elev(lat, lon, 0.0).expect("cpu proj");
            assert!(front);
            let p_w = lat_lon_to_unit_sphere(lat, lon);
            let pt = [p_w[0] as f32, p_w[1] as f32, p_w[2] as f32];
            let gpu_sp = u_orb
                .project_planet_to_screen(pt, rect)
                .expect("gpu planet proj");
            let ndc_z = u_orb
                .project_planet_ndc_z(pt, rect)
                .expect("gpu planet ndc z");
            assert!(
                (0.0..=1.0).contains(&ndc_z),
                "WebGPU Planetary NDC z must stay within [0.0, 1.0], got {ndc_z}"
            );
            let err_px = (cpu_sp - gpu_sp).length();
            assert!(
                err_px < 0.05,
                "Planetary MVP mismatch at ({lat},{lon}): cpu={cpu_sp:?} gpu={gpu_sp:?} err={err_px:.4}px"
            );
        }

        // 2. Street-scale Floating-Origin RTC matrix parity (zoom = 18.5, WebGPU z in [0, 1])
        let site = (-6.2088, 106.8456);
        let xf_site = GlobeScreenTransform::from_view(rect, -6.2085, 106.8459, 18.5, &cam);
        let u_site = RtcCameraUniforms::from_globe_xform(
            &xf_site,
            VectorEarthTheme::CityDaylight,
            &cam,
            site,
            1.5,
            true,
        );
        for &(east_m, north_m, up_m) in &[(0.0_f64, 0.0, 0.0), (24.0, -16.0, 18.0), (-35.0, 42.0, 48.0)] {
            let ll = SlippyMapState::world_mm_to_lat_lon(site, [east_m * 1000.0, north_m * 1000.0]);
            let (cpu_sp, _, front) = xf_site.project_lat_lon_elev(ll.0, ll.1, up_m).expect("cpu site proj");
            assert!(front);
            let pt = [east_m as f32, north_m as f32, up_m as f32];
            let gpu_sp = u_site
                .project_site_to_screen(pt, rect)
                .expect("gpu rtc site proj");
            let ndc_z = u_site
                .project_site_ndc_z(pt, rect)
                .expect("gpu rtc site ndc z");
            assert!(
                (0.0..=1.0).contains(&ndc_z),
                "WebGPU RTC Site NDC z must stay within [0.0, 1.0], got {ndc_z}"
            );
            let err_px = (cpu_sp - gpu_sp).length();
            assert!(
                err_px < 0.25,
                "RTC Site MVP mismatch at ({east_m},{north_m},{up_m}): cpu={cpu_sp:?} gpu={gpu_sp:?} err={err_px:.4}px"
            );
        }
    }

    #[test]
    fn test_phase65_planetary_and_rtc_site_gpu_mesh_generation() {
        for vt in VectorEarthTheme::ALL {
            let globe = build_planetary_vector_globe_mesh(vt);
            assert!(globe.vertices.len() > 2_500, "Expected rich globe vertices for {vt:?}");
            assert!(globe.indices.len() >= 12_000, "Expected 4,000+ triangles for {vt:?}");
            assert_eq!(globe.indices.len() % 3, 0);
            for v in &globe.vertices {
                assert!(v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite());
                assert!(v.normal[0].is_finite() && v.normal[1].is_finite() && v.normal[2].is_finite());
            }
        }

        let origin = (-6.9175, 107.6191);
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::VolcanicFoothillsRidge,
            300.0,
            300.0,
            16,
            16,
            1.0,
        );
        let mut world_idx = WorldSpatialIndex::default();
        world_idx.upsert_building(
            WorldBuildingProxy {
                id: 1,
                rel_path: "Projects/Bandung/Tower_B.ifcx".to_string(),
                name: "Tower B".to_string(),
                filename: "Tower_B.ifcx".to_string(),
                lat: origin.0,
                lon: origin.1,
                elev_m: 710.0,
                heading_deg: 15.0,
                footprint_local_m: vec![[-12.0, -9.0], [12.0, -9.0], [12.0, 9.0], [-12.0, 9.0]],
                total_height_m: 44.0,
                story_count: 12,
                member_count: 180,
                max_dcr: 0.78,
                min_eigengate: 0.84,
                source: GeoPlacementSource::UserPinnedOnEarth,
                address_hint: "Bandung".to_string(),
            },
            true,
        );
        let urban = UrbanVectorLayerCache::default();
        let site_mesh = build_rtc_site_scene_mesh(
            VectorEarthTheme::CoastalBlueprint,
            origin,
            17.5,
            true,
            Some(&tin),
            &world_idx,
            None,
            &urban,
            None,
        );
        assert!(site_mesh.indices.len() / 3 > 400, "Site TIN + extruded IFC tower mesh should be populated");
        assert!(
            site_mesh.vertices.iter().any(|v| (v.meta[0] - 2.0).abs() < 0.01 && v.meta[1] > 40.0),
            "Extruded IFC building vertices must encode height_m for procedural facade shading"
        );
        assert!(
            site_mesh.vertices.iter().any(|v| (v.meta[0] - 2.0).abs() < 0.01 && v.meta[2] < 0.65),
            "Building ground vertices must encode base Ambient Occlusion"
        );
    }

    #[test]
    fn test_phase67_unified_gpu_city_builder_and_active_bim_modal_n1_mesh() {
        let mut city = IfcxCityScene::default();
        city.add_road([-40.0, 0.0], [40.0, 0.0], crate::core::gis::IfcxRoadKind::Arterial, false);
        city.add_road([0.0, -30.0], [0.0, 30.0], crate::core::gis::IfcxRoadKind::Collector, true);
        city.add_zone([24.0, 20.0], 26.0, 22.0, 0.0, IfcxZoneKind::Commercial);
        city.add_zone([-24.0, 20.0], 24.0, 24.0, 0.0, IfcxZoneKind::Park);

        let urban = UrbanVectorLayerCache::default();
        let world_idx = WorldSpatialIndex::default();
        let site_with_city = build_rtc_site_scene_mesh(
            VectorEarthTheme::CityDaylight,
            (-6.9175, 107.6191),
            17.5,
            false,
            None,
            &world_idx,
            None,
            &urban,
            Some(&city),
        );
        assert!(
            site_with_city.indices.len() / 3 > 500,
            "City-builder roads, bridge piers, context towers, and stylized trees must generate GPU triangles"
        );
        // Bridge deck should sit above 6.0m
        assert!(site_with_city.vertices.iter().any(|v| v.pos[2] >= 6.4));

        // Active 3D BIM + subsurface pile caps + GPU modal vibration + crimson N-1 ghost
        let (proj, _) = IfcxDocument::sample_villa_ifcx()
            .to_bim_project("Villa")
            .expect("villa project");
        let first_col_id = proj.columns[0].id;
        let clean_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::Clean,
            erection_day_limit: 999,
            deform_scale: 40.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &[],
            replay_story_disp_xyz_m: &[],
            replay_ground_shake_xyz_m: [0.0, 0.0, 0.0],
            replay_hinge_states: &[],
            shm_story_retention: &[],
        });
        let modal_n1_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::ModalVibration,
            erection_day_limit: 999,
            deform_scale: 60.0,
            active_modal_mode: 0,
            modal_phase_sin: 1.0,
            n1_removed_bim: Some((BimElementKind::Column, first_col_id)),
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &[],
            replay_story_disp_xyz_m: &[],
            replay_ground_shake_xyz_m: [0.0, 0.0, 0.0],
            replay_hinge_states: &[],
            shm_story_retention: &[],
        });
        assert!(clean_mesh.indices.len() / 3 > 400);
        // Subsurface deep pile shafts extend below z = -3.0m
        assert!(clean_mesh.vertices.iter().any(|v| v.pos[2] < -3.0));
        // Modal vibration shifts elevated vertices along X
        let max_x_clean = clean_mesh.vertices.iter().map(|v| v.pos[0]).fold(f32::NEG_INFINITY, f32::max);
        let max_x_modal = modal_n1_mesh.vertices.iter().map(|v| v.pos[0]).fold(f32::NEG_INFINITY, f32::max);
        assert!(max_x_modal > max_x_clean + 0.25);
        // N-1 removed column renders with translucent crimson ghost alpha
        assert!(modal_n1_mesh.vertices.iter().any(|v| v.color[0] > 0.90 && v.color[3] < 0.55));
    }

    #[test]
    fn test_phase68_gpu_district_multi_hazard_mesh_generation() {
        use crate::core::gis::district_hazard::evaluate_district_multi_hazard;
        use crate::core::gis::neighborhood_physics::NeighborPoundingContact;

        let mut cfg = DistrictHazardConfig::default();
        let nb = NeighborhoodAuditReport {
            neighbors_evaluated: 1,
            pounding_risk_count: 1,
            venturi_corridor_count: 1,
            venturi_speedup_factor: 1.08,
            contacts: vec![NeighborPoundingContact {
                neighbor_rel_path: "nb.ifcx".to_string(),
                neighbor_name: "Tower NB".to_string(),
                neighbor_lat: -6.9176,
                neighbor_lon: 107.6193,
                neighbor_height_m: 24.0,
                neighbor_min_eigengate: 0.58,
                clear_gap_m: 0.22,
                required_gap_m: 0.52,
                severity: PoundingSeverity::PoundingRisk,
                closest_active_ll: (-6.9175, 107.6191),
                closest_neighbor_ll: (-6.9176, 107.6193),
                venturi_channel: true,
                inside_collapse_fall_zone: true,
            }],
            ..Default::default()
        };
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::CoastalAlluvialBasin,
            240.0,
            240.0,
            12,
            12,
            1.0,
        );
        let proj = BimProject::sample_bimcraft_building();
        let city = IfcxCityScene::default();
        let rep = evaluate_district_multi_hazard(
            &cfg,
            &nb,
            Some(&tin),
            &WorldSpatialIndex::default(),
            (-6.9175, 107.6191),
            &proj,
            &city,
        );

        let empty_mesh = build_rtc_district_hazard_mesh(&DistrictHazardGpuInput {
            config: &cfg,
            report: &rep,
            neighborhood: &nb,
            origin_lat_lon: (-6.9175, 107.6191),
            active_building_height_m: 12.0,
            active_site_tin: Some(&tin),
        });
        assert!(empty_mesh.vertices.is_empty());

        cfg.seismic_wave_active = true;
        cfg.pounding_overlay_active = true;
        cfg.venturi_wind_active = true;
        cfg.flood_plane_active = true;
        cfg.flood_water_level_m = 4.0;
        cfg.sim_time_sec = 0.75;
        let rep_on = evaluate_district_multi_hazard(
            &cfg,
            &nb,
            Some(&tin),
            &WorldSpatialIndex::default(),
            (-6.9175, 107.6191),
            &proj,
            &city,
        );
        let full_mesh = build_rtc_district_hazard_mesh(&DistrictHazardGpuInput {
            config: &cfg,
            report: &rep_on,
            neighborhood: &nb,
            origin_lat_lon: (-6.9175, 107.6191),
            active_building_height_m: 12.0,
            active_site_tin: Some(&tin),
        });
        assert!(full_mesh.indices.len() / 3 > 250);
        assert!(full_mesh
            .vertices
            .iter()
            .all(|v| v.pos.iter().all(|f| f.is_finite()) && v.normal.iter().all(|f| f.is_finite())));
    }

    #[test]
    fn test_phase70_gpu_progressive_collapse_cascade_domino_mesh() {
        let proj = BimProject::sample_bimcraft_building();
        let failed = vec![
            (BimElementKind::Column, proj.columns[0].id),
            (BimElementKind::Run, proj.runs[0].id),
        ];
        let mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::Clean,
            erection_day_limit: 999,
            deform_scale: 35.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &failed,
            cascade_step_progress: 0.8,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &[],
            replay_story_disp_xyz_m: &[],
            replay_ground_shake_xyz_m: [0.0, 0.0, 0.0],
            replay_hinge_states: &[],
            shm_story_retention: &[],
        });
        assert!(mesh.indices.len() / 3 > 200);
        assert!(
            mesh.vertices
                .iter()
                .any(|v| v.color[0] > 0.94 && v.color[1] < 0.28 && v.color[3] < 0.42),
            "Cascading domino-failed members must render as translucent crimson-orange ghosts"
        );
    }

    #[test]
    fn test_phase71_gpu_subsurface_stratigraphy_and_dynamic_pile_mesh() {
        use crate::core::geotech::{
            evaluate_coupled_subsurface_ssi_for_bim, BoreholePreset, SubsurfaceSsiConfig,
        };

        let proj = BimProject::sample_bimcraft_building();
        let site = crate::core::compute_site_environmental_coupling(
            -6.2088,
            106.8456,
            &crate::core::gis::earth_globe::PlanetaryWorldState::default(),
        );
        let mut cfg = SubsurfaceSsiConfig {
            ssi_coupling_enabled: true,
            xray_subsurface_visible: false,
            borehole_preset: BoreholePreset::CoastalLiquefiableSand,
            groundwater_depth_m: 1.5,
            liquefaction_trigger_active: true,
        };
        let hidden = build_rtc_subsurface_ssi_mesh(&SubsurfaceSsiGpuInput {
            project: &proj,
            config: &cfg,
            report: None,
            heading_deg: 0.0,
        });
        assert!(hidden.vertices.is_empty());

        cfg.xray_subsurface_visible = true;
        let rep = evaluate_coupled_subsurface_ssi_for_bim(&proj, &site, &cfg)
            .expect("Subsurface SSI evaluation succeeds");
        let ss_mesh = build_rtc_subsurface_ssi_mesh(&SubsurfaceSsiGpuInput {
            project: &proj,
            config: &cfg,
            report: Some(&rep),
            heading_deg: 0.0,
        });
        assert!(ss_mesh.indices.len() / 3 >= 40);
        assert!(ss_mesh.vertices.iter().any(|v| (v.meta[0] - 4.0).abs() < 0.1));
        assert!(ss_mesh.vertices.iter().any(|v| (v.meta[0] - 5.0).abs() < 0.1));
        assert!(ss_mesh
            .vertices
            .iter()
            .any(|v| (v.meta[0] - 4.0).abs() < 0.1 && v.color[0] > 0.90 && v.color[1] < 0.40));

        let deep_piles = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::Clean,
            erection_day_limit: 999,
            deform_scale: 35.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 22.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &[],
            replay_story_disp_xyz_m: &[],
            replay_ground_shake_xyz_m: [0.0, 0.0, 0.0],
            replay_hinge_states: &[],
            shm_story_retention: &[],
        });
        assert!(
            deep_piles.vertices.iter().any(|v| v.pos[2] < -20.0),
            "Dynamic subsurface pile embedment must reach bearing stratum depth"
        );
    }

    #[test]
    fn test_phase72_gpu_ifcx_visual_diff_highlight_and_deleted_ghosts() {
        let proj = BimProject::sample_bimcraft_building();
        let added = vec![(BimElementKind::Column, proj.columns[0].id)];
        let modified = vec![(BimElementKind::Run, proj.runs[0].id)];
        let ghosts = vec![([1.0, 1.0, 0.0], [3.0, 4.0, 3.6], "/deleted/wall_99".to_string())];

        let mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::Clean,
            erection_day_limit: 999,
            deform_scale: 35.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: true,
            diff_added_bim: &added,
            diff_modified_bim: &modified,
            diff_deleted_ghosts: &ghosts,
            replay_story_elevs_m: &[],
            replay_story_disp_xyz_m: &[],
            replay_ground_shake_xyz_m: [0.0, 0.0, 0.0],
            replay_hinge_states: &[],
            shm_story_retention: &[],
        });

        assert!(
            mesh.vertices.iter().any(|v| v.color[1] > 0.85 && v.color[0] < 0.25),
            "Added BIM elements must render emerald green during IFCX Visual Diff"
        );
        assert!(
            mesh.vertices.iter().any(|v| v.color[0] > 0.92 && v.color[1] > 0.65 && v.color[2] < 0.25),
            "Modified BIM elements must render amber gold during IFCX Visual Diff"
        );
        assert!(
            mesh.vertices.iter().any(|v| v.color[0] > 0.92 && v.color[3] < 0.40),
            "Deleted IFCX nodes must emit translucent crimson AABB ghost prisms"
        );
    }

    #[test]
    fn test_phase73_gpu_district_portfolio_retrofit_overlay_mesh() {
        use crate::core::portfolio_retrofit::{
            optimize_district_portfolio_retrofit, PortfolioRetrofitConfig,
        };

        let proj = BimProject::sample_bimcraft_building();
        let site = crate::core::compute_site_environmental_coupling(
            -6.2088,
            106.8456,
            &crate::core::gis::earth_globe::PlanetaryWorldState::default(),
        );
        let rep = optimize_district_portfolio_retrofit(
            &WorldSpatialIndex::default(),
            &proj,
            None,
            &[],
            (-6.2088, 106.8456),
            &site,
            &PortfolioRetrofitConfig {
                budget_cap_usd: 250_000.0,
                overlay_visible: true,
                ..Default::default()
            },
        );

        let pf_mesh = build_rtc_portfolio_retrofit_mesh(&PortfolioGpuOverlayInput {
            report: &rep,
            origin_lat_lon: (-6.2088, 106.8456),
        });
        assert!(pf_mesh.indices.len() / 3 >= 24);
        assert!(
            pf_mesh.vertices.iter().any(|v| v.color[1] > 0.82 && v.color[3] > 0.45),
            "Retrofitted district buildings must render glowing emerald/cyan/amber parity crowns"
        );
        assert!(pf_mesh
            .vertices
            .iter()
            .all(|v| v.pos.iter().all(|f| f.is_finite()) && v.normal.iter().all(|f| f.is_finite())));
    }

    #[test]
    fn test_phase74_gpu_time_history_replay_sway_and_plastic_hinge_beacons() {
        use crate::core::ground_motion_replay::{
            run_ground_motion_replay_for_bim, GroundMotionReplayConfig,
        };
        use crate::core::time_history::GroundMotionRecord;

        let proj = BimProject::sample_bimcraft_building();
        let site = crate::core::compute_site_environmental_coupling(
            -6.2088,
            106.8456,
            &crate::core::gis::earth_globe::PlanetaryWorldState::default(),
        );
        let rep = run_ground_motion_replay_for_bim(
            &proj,
            &site,
            None,
            &GroundMotionReplayConfig {
                record: GroundMotionRecord::NorthridgeSylmar1994,
                scale_factor: 1.85,
                ..Default::default()
            },
            "GpuUnitTest",
            None,
        )
        .expect("Replay report");

        let frame = rep.sample_interpolated(rep.peak_time_s);
        let disp_triples: Vec<[f64; 3]> = (0..frame.story_elevations_m.len())
            .map(|i| {
                [
                    frame.story_disp_x_m.get(i).copied().unwrap_or(0.0),
                    frame.story_disp_y_m.get(i).copied().unwrap_or(0.0),
                    frame.story_disp_z_m.get(i).copied().unwrap_or(0.0),
                ]
            })
            .collect();
        let hinge_states = rep.worst_hinge_states_at(rep.peak_time_s);

        let base_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::Clean,
            erection_day_limit: 999,
            deform_scale: 40.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &[],
            replay_story_disp_xyz_m: &[],
            replay_ground_shake_xyz_m: [0.0, 0.0, 0.0],
            replay_hinge_states: &[],
            shm_story_retention: &[],
        });

        let sway_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::TimeHistoryReplay,
            erection_day_limit: 999,
            deform_scale: 40.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &frame.story_elevations_m,
            replay_story_disp_xyz_m: &disp_triples,
            replay_ground_shake_xyz_m: [
                frame.ground_disp_m,
                frame.ground_disp_y_m,
                frame.ground_disp_z_m,
            ],
            replay_hinge_states: &hinge_states,
            shm_story_retention: &[],
        });

        assert!(sway_mesh.vertices.iter().all(|v| v.pos.iter().all(|f| f.is_finite())));
        let max_dx = base_mesh
            .vertices
            .iter()
            .zip(sway_mesh.vertices.iter())
            .filter(|(a, _)| a.pos[2] > 2.0)
            .map(|(a, b)| (b.pos[0] - a.pos[0]).hypot(b.pos[1] - a.pos[1]))
            .fold(0.0_f32, f32::max);
        assert!(
            max_dx > 0.05,
            "TimeHistoryReplay lens must displace upper-story BIM vertices (got max_dx={max_dx})"
        );

        let hinge_mesh = build_rtc_plastic_hinge_mesh(&PlasticHingeGpuInput {
            project: &proj,
            frame: &frame,
            show_hinges: true,
            deform_scale: 40.0,
            heading_deg: 0.0,
        });
        assert!(hinge_mesh.indices.len() / 3 >= 24);
        assert!(hinge_mesh
            .indices
            .iter()
            .all(|&ix| (ix as usize) < hinge_mesh.vertices.len()));
        assert!(hinge_mesh
            .vertices
            .iter()
            .all(|v| v.pos.iter().all(|f| f.is_finite()) && v.meta[0] >= 5.5));
    }

    #[test]
    fn test_phase75_webgpu_tridirectional_sway_and_fragility_loss_mesh() {
        use crate::core::ground_motion_replay::{
            run_ground_motion_replay_for_bim, GroundMotionReplayConfig, TriDirectionalMode,
        };
        use crate::core::time_history::{parse_custom_accelerogram, sample_peer_at2_text};

        let proj = BimProject::sample_bimcraft_building();
        let site = crate::core::compute_site_environmental_coupling(
            -6.2088,
            106.8456,
            &crate::core::gis::earth_globe::PlanetaryWorldState::default(),
        );
        let acc = parse_custom_accelerogram(sample_peer_at2_text(), "sample_peer.at2")
            .expect("Sample PEER .at2 should parse");

        let rep = run_ground_motion_replay_for_bim(
            &proj,
            &site,
            None,
            &GroundMotionReplayConfig {
                tridirectional_mode: TriDirectionalMode::TriDirectionalXYZ,
                custom_accelerogram: Some(acc),
                auto_match_sa_t1: true,
                show_fragility_loss_overlay: true,
                ..Default::default()
            },
            "WebGpuPhase75Test",
            None,
        )
        .expect("Tri-directional PEER replay report");

        let frame = rep.sample_interpolated(rep.peak_time_s);
        let disp_triples: Vec<[f64; 3]> = (0..frame.story_elevations_m.len())
            .map(|i| {
                [
                    frame.story_disp_x_m.get(i).copied().unwrap_or(0.0),
                    frame.story_disp_y_m.get(i).copied().unwrap_or(0.0),
                    frame.story_disp_z_m.get(i).copied().unwrap_or(0.0),
                ]
            })
            .collect();
        let hinge_states = rep.worst_hinge_states_at(rep.peak_time_s);

        let base_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::Clean,
            erection_day_limit: 999,
            deform_scale: 40.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &[],
            replay_story_disp_xyz_m: &[],
            replay_ground_shake_xyz_m: [0.0, 0.0, 0.0],
            replay_hinge_states: &[],
            shm_story_retention: &[],
        });

        let tridir_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::TimeHistoryReplay,
            erection_day_limit: 999,
            deform_scale: 40.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &frame.story_elevations_m,
            replay_story_disp_xyz_m: &disp_triples,
            replay_ground_shake_xyz_m: [
                frame.ground_disp_m,
                frame.ground_disp_y_m,
                frame.ground_disp_z_m,
            ],
            replay_hinge_states: &hinge_states,
            shm_story_retention: &[],
        });

        assert!(tridir_mesh
            .vertices
            .iter()
            .all(|v| v.pos.iter().all(|f| f.is_finite())));
        let max_dz = base_mesh
            .vertices
            .iter()
            .zip(tridir_mesh.vertices.iter())
            .filter(|(a, _)| a.pos[2] > 2.0)
            .map(|(a, b)| (b.pos[2] - a.pos[2]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            max_dz > 1e-4,
            "TriAxialXYZ replay must propagate vertical seismic heave to BIM vertices (max_dz={max_dz})"
        );

        let frag_mesh = build_rtc_fragility_loss_mesh(&FragilityLossGpuInput {
            project: &proj,
            synthesis: &rep.fragility_loss,
            visible: true,
            heading_deg: 0.0,
        });
        assert!(frag_mesh.indices.len() / 3 >= 8);
        assert!(frag_mesh
            .vertices
            .iter()
            .all(|v| v.pos.iter().all(|f| f.is_finite()) && (v.meta[0] - 7.0).abs() < 0.01));

        let hidden_frag = build_rtc_fragility_loss_mesh(&FragilityLossGpuInput {
            project: &proj,
            synthesis: &rep.fragility_loss,
            visible: false,
            heading_deg: 0.0,
        });
        assert!(hidden_frag.vertices.is_empty() && hidden_frag.indices.is_empty());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_phase75_real_wgpu_pipelines_wgsl_and_non_black_render() {
        use std::sync::{Arc, Mutex};

        fn block_on_local<F: std::future::Future>(fut: F) -> F::Output {
            use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
            fn no_op(_: *const ()) {}
            fn clone_w(p: *const ()) -> RawWaker {
                RawWaker::new(p, &VTABLE)
            }
            const VTABLE: RawWakerVTable = RawWakerVTable::new(clone_w, no_op, no_op, no_op);
            let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
            let mut cx = Context::from_waker(&waker);
            let mut fut = std::pin::pin!(fut);
            loop {
                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(val) => return val,
                    Poll::Pending => std::thread::yield_now(),
                }
            }
        }

        let instance = wgpu::Instance::default();
        let Ok(adapter) = block_on_local(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })) else {
            // Headless CI without a GPU adapter skips cleanly while compile/linking still verifies.
            return;
        };
        let Ok((device, queue)) =
            block_on_local(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };

        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        let planet_mesh = build_planetary_vector_globe_mesh(VectorEarthTheme::CoastalBlueprint);
        res.planet.upload(&device, &queue, "test_planet", 1, &planet_mesh);

        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(64.0, 64.0));
        let cam = GlobeCamera::default();
        let xform = GlobeScreenTransform::from_view(rect, -2.0, 112.0, 3.0, &cam);
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xform,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-2.0, 112.0),
            1.25,
            false,
        );
        let pu = uniforms.to_planet_pass_uniforms();
        queue.write_buffer(&res.planet_uniform_buf, 0, struct_as_u8_slice(&pu));
        res.ensure_targets(&device, 64, 64);

        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test_composite_out"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&Default::default());
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test_readback"),
            size: (64 * 64 * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, false);
        {
            let target = res.targets.as_ref().expect("Offscreen targets allocated");
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test_composite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_viewport(0.0, 0.0, 64.0, 64.0, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, 64, 64);
            pass.set_pipeline(&res.composite_pipeline);
            pass.set_bind_group(0, &target.composite_bg, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            out_tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(64 * 4),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);

        let map_res: Arc<Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> =
            Arc::new(Mutex::new(None));
        let map_res_cb = Arc::clone(&map_res);
        read_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                *map_res_cb.lock().unwrap() = Some(r);
            });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = block_on_local(val_scope.pop());
        assert!(
            val_err.is_none(),
            "WebGPU pipeline triggered GPU validation error: {val_err:?}"
        );
        assert!(
            matches!(*map_res.lock().unwrap(), Some(Ok(()))),
            "Readback buffer failed to map"
        );

        let bytes = read_buf
            .slice(..)
            .get_mapped_range()
            .expect("Mapped range");
        let non_black_pixels = bytes
            .chunks_exact(4)
            .filter(|px| (px[0] > 0 || px[1] > 0 || px[2] > 0) && px[3] > 0)
            .count();
        drop(bytes);
        read_buf.unmap();
        assert!(
            non_black_pixels > 500,
            "Expected curved Vector Earth globe to render non-black opaque pixels (got {non_black_pixels}/4096)"
        );
    }

    #[test]
    fn test_phase76_webgpu_shm_sensor_beacons_atc20_placard_and_real_gpu_render() {
        let proj = BimProject::sample_bimcraft_building();
        let site = crate::core::SiteEnvironmentalCoupling::default();
        let mut world = WorldSpatialIndex::default();
        world.upsert_building(
            crate::core::gis::WorldBuildingProxy::from_bim_project(
                "adjacent_bldg.ifcx",
                "adjacent_bldg.ifcx",
                -6.2080,
                106.8462,
                15.0,
                &proj,
                &[],
                crate::core::gis::GeoPlacementSource::UserPinnedOnEarth,
                "Jakarta".to_string(),
            ),
            true,
        );
        let cfg = crate::core::shm_twin::ShmDigitalTwinConfig {
            fault_scenario: crate::core::shm_twin::ShmFaultScenario::SoftStoryFirstFloor,
            severity_pct: 85.0,
            target_story: 1,
            ..Default::default()
        };
        let rep = crate::core::shm_twin::evaluate_shm_digital_twin_for_bim(
            &proj,
            &site,
            None,
            None,
            None,
            Some(&world),
            &cfg,
            (-6.2088, 106.8456),
        )
        .expect("SHM report");

        let retention = rep.story_retention_pairs();
        let shm_bim_mesh = build_rtc_active_bim_fem_mesh(&ActiveBimGpuInput {
            project: &proj,
            members: &[],
            lens: EarthBimLens::ShmDigitalTwin,
            erection_day_limit: 999,
            deform_scale: 35.0,
            active_modal_mode: 0,
            modal_phase_sin: 0.0,
            n1_removed_bim: None,
            cascade_failed_bim: &[],
            cascade_step_progress: 0.0,
            heading_deg: 0.0,
            subsurface_pile_depth_m: 4.0,
            diff_overlay_active: false,
            diff_added_bim: &[],
            diff_modified_bim: &[],
            diff_deleted_ghosts: &[],
            replay_story_elevs_m: &[],
            replay_story_disp_xyz_m: &[],
            replay_ground_shake_xyz_m: [0.0, 0.0, 0.0],
            replay_hinge_states: &[],
            shm_story_retention: &retention,
        });
        assert!(shm_bim_mesh.indices.len() / 3 > 100);
        // Soft-story floor degradation must emit crimson/amber stiffness-loss colors
        assert!(shm_bim_mesh
            .vertices
            .iter()
            .any(|v| v.color[0] > 0.90 && v.color[1] < 0.72));

        let shm_triage_mesh = build_rtc_shm_triage_mesh(&ShmTriageGpuInput {
            project: &proj,
            report: &rep,
            visible: true,
            heading_deg: 0.0,
            site_origin: (-6.2088, 106.8456),
        });
        assert!(shm_triage_mesh.indices.len() / 3 > 20);
        assert!(shm_triage_mesh
            .vertices
            .iter()
            .all(|v| (v.meta[0] - 8.0).abs() < 1e-3));

        // Real Headless WebGPU / Metal GPU Pipeline Compilation (Pass 1 + Pass 2 + SHM Triage)
        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };

        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        let planet_mesh = build_planetary_vector_globe_mesh(VectorEarthTheme::CoastalBlueprint);
        res.planet
            .upload(&device, &queue, "planet_p76", 1, &planet_mesh);
        res.bim.upload(&device, &queue, "bim_p76", 1, &shm_bim_mesh);
        res.shm_triage
            .upload(&device, &queue, "shm_triage_p76", 1, &shm_triage_mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 42.0;
        cam.heading_deg = 20.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            1.25,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("p76_composite_out"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&Default::default());
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("p76_readback"),
            size: (64 * 64 * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        {
            let target = res.targets.as_ref().expect("Targets allocated");
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("p76_composite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_viewport(0.0, 0.0, 64.0, 64.0, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, 64, 64);
            pass.set_pipeline(&res.composite_pipeline);
            pass.set_bind_group(0, &target.composite_bg, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            out_tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(64 * 4),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);

        let map_res: std::sync::Arc<std::sync::Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let map_res_cb = std::sync::Arc::clone(&map_res);
        read_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                *map_res_cb.lock().unwrap() = Some(r);
            });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 76 WGSL SHM pipeline triggered WebGPU validation error: {val_err:?}"
        );
        assert!(matches!(*map_res.lock().unwrap(), Some(Ok(()))));

        let bytes = read_buf
            .slice(..)
            .get_mapped_range()
            .expect("Mapped range");
        let non_black_pixels = bytes
            .chunks_exact(4)
            .filter(|px| (px[0] > 0 || px[1] > 0 || px[2] > 0) && px[3] > 0)
            .count();
        drop(bytes);
        read_buf.unmap();
        assert!(
            non_black_pixels > 400,
            "Expected Pass 1 + Pass 2 + SHM triage render to produce non-black pixels (got {non_black_pixels}/4096)"
        );
    }

    #[test]
    fn test_phase77_webgpu_evacuation_corridors_debris_cordons_and_real_gpu_render() {
        let proj = BimProject::sample_bimcraft_building();
        let site = crate::core::SiteEnvironmentalCoupling::default();
        let mut city = IfcxCityScene::default();
        city.add_road([-40.0, 0.0], [60.0, 0.0], crate::core::IfcxRoadKind::Collector, false);
        city.add_road([-40.0, 0.0], [-40.0, 50.0], crate::core::IfcxRoadKind::Collector, false);
        city.add_road([-40.0, 50.0], [60.0, 50.0], crate::core::IfcxRoadKind::Arterial, false);
        city.add_zone([60.0, 50.0], 24.0, 20.0, 0.0, crate::core::IfcxZoneKind::Park);

        let shm_rep = crate::core::shm_twin::evaluate_shm_digital_twin_for_bim(
            &proj,
            &site,
            None,
            None,
            None,
            None,
            &crate::core::shm_twin::ShmDigitalTwinConfig {
                fault_scenario: crate::core::shm_twin::ShmFaultScenario::SoftStoryFirstFloor,
                severity_pct: 90.0,
                target_story: 1,
                ..Default::default()
            },
            (-6.2088, 106.8456),
        )
        .expect("SHM report");

        let evac_cfg = crate::core::evacuation_routing::EvacuationRoutingConfig {
            overlay_visible: true,
            mode: crate::core::evacuation_routing::EvacuationMode::Both,
            ..Default::default()
        };
        let evac_rep = crate::core::evacuation_routing::evaluate_evacuation_and_lifelines(
            &proj,
            &city,
            Some(&shm_rep),
            None,
            None,
            None,
            None,
            None,
            &evac_cfg,
            (-6.2088, 106.8456),
        )
        .expect("Evacuation report");

        let evac_mesh = build_rtc_evacuation_lifelines_mesh(&EvacuationLifelineGpuInput {
            report: &evac_rep,
            visible: true,
        });
        assert!(evac_mesh.indices.len() / 3 > 40);
        assert!(evac_mesh
            .vertices
            .iter()
            .all(|v| (v.meta[0] - 9.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };

        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        let planet_mesh = build_planetary_vector_globe_mesh(VectorEarthTheme::CoastalBlueprint);
        res.planet
            .upload(&device, &queue, "planet_p77", 1, &planet_mesh);
        res.evacuation
            .upload(&device, &queue, "evacuation_p77", 1, &evac_mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 42.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.0,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            1.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("p77_composite_out"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&Default::default());
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("p77_readback"),
            size: (64 * 64 * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        {
            let target = res.targets.as_ref().expect("Targets allocated");
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("p77_composite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_viewport(0.0, 0.0, 64.0, 64.0, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, 64, 64);
            pass.set_pipeline(&res.composite_pipeline);
            pass.set_bind_group(0, &target.composite_bg, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            out_tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(64 * 4),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);

        let map_res: std::sync::Arc<std::sync::Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let map_res_cb = std::sync::Arc::clone(&map_res);
        read_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                *map_res_cb.lock().unwrap() = Some(r);
            });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 77 WGSL evacuation pipeline triggered WebGPU validation error: {val_err:?}"
        );
        assert!(matches!(*map_res.lock().unwrap(), Some(Ok(()))));

        let bytes = read_buf
            .slice(..)
            .get_mapped_range()
            .expect("Mapped range");
        let non_black_pixels = bytes
            .chunks_exact(4)
            .filter(|px| (px[0] > 0 || px[1] > 0 || px[2] > 0) && px[3] > 0)
            .count();
        drop(bytes);
        read_buf.unmap();
        assert!(
            non_black_pixels > 400,
            "Expected Pass 1 + Pass 2 + Evacuation Lifelines render to produce non-black pixels (got {non_black_pixels}/4096)"
        );
    }

    #[test]
    fn test_phase79_webgpu_compute_pipelines_wgsl_particles_and_mc_gpu() {
        let proj = BimProject::sample_bimcraft_building();
        let mc_cfg = crate::core::gpu_fragility_mc::MonteCarloFragilityConfig {
            sample_count: 1024,
            seed: 79_2026,
            gpu_particles_enabled: true,
            gpu_particle_count: 512,
            ..Default::default()
        };
        let cpu_mc = crate::core::gpu_fragility_mc::run_fema_p58_monte_carlo_cpu(
            &proj, None, None, None, &mc_cfg,
        );
        let hc_input = HazardComputeInput {
            enabled: true,
            params: HazardComputeParams {
                time_secs: 1.25,
                pga_g: 0.42,
                wave_speed_ms: 280.0,
                wave_azimuth_rad: 0.5,
                wind_speed_ms: 34.0,
                wind_azimuth_rad: 1.1,
                vs30_ms: 240.0,
                particle_count_f32: 512.0,
                mode_flags_f32: 3.0,
                domain_radius_m: 120.0,
                obstacle_count_f32: 1.0,
                intensity_scale: 1.0,
                active_cx: 0.0,
                active_cy: 0.0,
                active_half_w: 8.0,
                active_half_d: 8.0,
            },
            obstacles: vec![HazardObstacleGpu {
                xy_r_h: [25.0, -15.0, 12.0, 18.0],
            }],
        };
        let cpu_verts = step_hazard_particles_cpu(&hc_input);
        assert_eq!(cpu_verts.len(), 512 * 6);
        assert!(cpu_verts.iter().all(|v| (v.meta[0] - 6.0).abs() < 1e-3 && v.pos[0].is_finite()));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };

        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        let planet_mesh = build_planetary_vector_globe_mesh(VectorEarthTheme::CoastalBlueprint);
        res.planet
            .upload(&device, &queue, "planet_p79", 1, &planet_mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 42.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.0,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            1.25,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("p79_composite_out"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&Default::default());
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("p79_readback"),
            size: (64 * 64 * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        res.hazard_compute.dispatch(&queue, &mut encoder, &hc_input);
        assert_eq!(res.hazard_compute.active_vertex_count, 512 * 6);
        res.render_offscreen_passes(&mut encoder, true);
        {
            let target = res.targets.as_ref().expect("Targets allocated");
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("p79_composite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_viewport(0.0, 0.0, 64.0, 64.0, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, 64, 64);
            pass.set_pipeline(&res.composite_pipeline);
            pass.set_bind_group(0, &target.composite_bg, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            out_tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(64 * 4),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);

        let map_res: std::sync::Arc<std::sync::Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let map_res_cb = std::sync::Arc::clone(&map_res);
        read_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                *map_res_cb.lock().unwrap() = Some(r);
            });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        assert!(matches!(*map_res.lock().unwrap(), Some(Ok(()))));
        let bytes = read_buf.slice(..).get_mapped_range().expect("Mapped range");
        let non_black_pixels = bytes
            .chunks_exact(4)
            .filter(|px| (px[0] > 0 || px[1] > 0 || px[2] > 0) && px[3] > 0)
            .count();
        drop(bytes);
        read_buf.unmap();
        assert!(non_black_pixels > 400);

        // Real GPU @compute FEMA P-58 Monte Carlo execution & CPU parity check
        let gpu_mc = crate::core::gpu_fragility_mc::run_fema_p58_monte_carlo_wgpu(
            &device, &queue, &proj, None, None, None, &mc_cfg,
        )
        .expect("GPU Monte Carlo compute execution");
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 79 @compute WGSL triggered WebGPU validation error: {val_err:?}"
        );
        assert_eq!(
            gpu_mc.backend,
            crate::core::gpu_fragility_mc::ComputeBackendKind::WebGpuCompute
        );
        assert_eq!(gpu_mc.sample_count, 1024);
        assert!(gpu_mc.p10_loss_usd <= gpu_mc.p50_loss_usd);
        assert!(gpu_mc.p50_loss_usd <= gpu_mc.p90_loss_usd);
        let rel_mean_diff =
            (gpu_mc.mean_loss_usd - cpu_mc.mean_loss_usd).abs() / cpu_mc.mean_loss_usd.max(1.0);
        let rel_p90_diff =
            (gpu_mc.p90_loss_usd - cpu_mc.p90_loss_usd).abs() / cpu_mc.p90_loss_usd.max(1.0);
        assert!(
            rel_mean_diff < 0.02 && rel_p90_diff < 0.02,
            "GPU MC vs CPU MC parity diverged: gpu=({}, {}) vs cpu=({}, {})",
            gpu_mc.mean_loss_usd,
            gpu_mc.p90_loss_usd,
            cpu_mc.mean_loss_usd,
            cpu_mc.p90_loss_usd
        );
    }

    #[test]
    fn test_phase80_webgpu_district_risk_heatmap_mesh_and_real_gpu_render() {
        use crate::core::ifcx_collab::{
            evaluate_district_risk_field, DistrictRiskMetric, IfcxCollabConfig,
        };

        let proj = BimProject::sample_bimcraft_building();
        let tin = TinSurfaceMesh::generate_from_dem_preset(
            DemTerrainPreset::TerracedCutFillPad,
            260.0,
            260.0,
            12,
            12,
            1.0,
        );
        let cfg = IfcxCollabConfig {
            risk_heatmap_visible: true,
            risk_metric: DistrictRiskMetric::CompositeRisk,
            risk_grid_resolution: 16,
            risk_opacity: 0.72,
            ..Default::default()
        };
        let report = evaluate_district_risk_field(
            &cfg,
            (-6.2088, 106.8456),
            &proj,
            &[],
            &WorldSpatialIndex::default(),
            Some(&tin),
            None,
            None,
            None,
            None,
        );
        let mesh = build_rtc_district_risk_heatmap_mesh(&DistrictRiskHeatmapGpuInput {
            report: &report,
            opacity: cfg.risk_opacity,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 10.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };

        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.risk_heatmap
            .upload(&device, &queue, "risk_heatmap_p80", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 45.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            16.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("p80_composite_out"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&Default::default());
        let read_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("p80_readback"),
            size: (64 * 64 * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        {
            let target = res.targets.as_ref().expect("Targets allocated");
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("p80_composite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_viewport(0.0, 0.0, 64.0, 64.0, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, 64, 64);
            pass.set_pipeline(&res.composite_pipeline);
            pass.set_bind_group(0, &target.composite_bg, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            out_tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(64 * 4),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);

        let map_res: std::sync::Arc<std::sync::Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let map_res_cb = std::sync::Arc::clone(&map_res);
        read_buf
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                *map_res_cb.lock().unwrap() = Some(r);
            });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        assert!(matches!(*map_res.lock().unwrap(), Some(Ok(()))));
        let bytes = read_buf.slice(..).get_mapped_range().expect("Mapped range");
        let non_black_pixels = bytes
            .chunks_exact(4)
            .filter(|px| (px[0] > 0 || px[1] > 0 || px[2] > 0) && px[3] > 0)
            .count();
        drop(bytes);
        read_buf.unmap();
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 80 District Risk Heatmap WGSL triggered WebGPU validation error: {val_err:?}"
        );
        assert!(non_black_pixels > 200);
    }

    #[test]
    fn test_phase82_webgpu_field_qaqc_mesh_and_real_gpu_render() {
        use crate::core::field_fabrication::{evaluate_field_fabrication, FieldFabricationConfig};
        use crate::core::lifecycle_ops::FieldQualityControlLedger;

        let mut proj = BimProject::sample_bimcraft_building();
        proj.reinforce_all_concrete(&crate::core::bim::RebarPreset::default());
        let ledger = FieldQualityControlLedger::default();
        let report = evaluate_field_fabrication(&proj, &ledger, &FieldFabricationConfig::default());

        let mesh = build_rtc_field_qaqc_mesh(&FieldQaqcGpuInput {
            report: &report,
            heading_deg: 15.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 11.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.field_qaqc
            .upload(&device, &queue, "field_qaqc_p82", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 42.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 82 Field QA/QC WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase83_webgpu_eigenflow_mesh_and_real_gpu_render() {
        use crate::core::eigenflow::{evaluate_eigenflow_studio, EigenFlowConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_eigenflow_studio(&proj, None, &EigenFlowConfig::default());
        let mesh = build_rtc_eigenflow_mesh(&EigenFlowGpuInput {
            report: &report,
            active_step_idx: 6,
            heading_deg: 18.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 12.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.eigenflow
            .upload(&device, &queue, "eigenflow_p83", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 83 EigenFlow WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_webgpu_extreme_events_mesh_and_real_gpu_render() {
        use crate::core::extreme_events::{
            evaluate_extreme_events, ExtremeEventMode, ExtremeEventsConfig,
        };

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_extreme_events(
            &proj,
            None,
            &ExtremeEventsConfig {
                mode: ExtremeEventMode::Combined,
                ..ExtremeEventsConfig::default()
            },
        );
        let mesh = build_rtc_extreme_events_mesh(&ExtremeEventsGpuInput {
            report: &report,
            heading_deg: 22.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 13.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.extreme_events
            .upload(&device, &queue, "extreme_events_p84", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84 Extreme Events WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_1_webgpu_isolation_mesh_and_real_gpu_render() {
        use crate::core::base_isolation_studio::{evaluate_isolation_studio, IsolationStudioConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_isolation_studio(&proj, None, &IsolationStudioConfig::default());
        let mesh = build_rtc_isolation_mesh(&IsolationGpuInput {
            report: &report,
            heading_deg: 10.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 14.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.isolation
            .upload(&device, &queue, "isolation_p84_1", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84.1 Isolation WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_2_webgpu_wind_mesh_and_real_gpu_render() {
        use crate::core::wind_studio::{evaluate_wind_studio, WindStudioConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_wind_studio(&proj, None, &WindStudioConfig::default());
        let mesh = build_rtc_wind_mesh(&WindGpuInput {
            report: &report,
            heading_deg: 10.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 15.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.wind_aero
            .upload(&device, &queue, "wind_aero_p84_2", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84.2 Wind WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_3_webgpu_shear_wall_mesh_and_real_gpu_render() {
        use crate::core::shear_wall_studio::{evaluate_shear_wall_studio, ShearWallStudioConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_shear_wall_studio(&proj, None, &ShearWallStudioConfig::default());
        let mesh = build_rtc_shear_wall_mesh(&ShearWallGpuInput {
            report: &report,
            heading_deg: 10.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 16.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.shear_wall
            .upload(&device, &queue, "shear_wall_p84_3", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84.3 Shear Wall WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_4_webgpu_connection_mesh_and_real_gpu_render() {
        use crate::core::connection_studio::{evaluate_connection_studio, ConnectionStudioConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_connection_studio(&proj, None, &ConnectionStudioConfig::default());
        let mesh = build_rtc_connection_mesh(&ConnectionGpuInput {
            report: &report,
            heading_deg: 10.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 17.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.steel_connection
            .upload(&device, &queue, "steel_conn_p84_4", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84.4 Steel Connection WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_5_webgpu_prestress_mesh_and_real_gpu_render() {
        use crate::core::prestress_studio::{evaluate_prestress_studio, PrestressStudioConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_prestress_studio(&proj, None, &PrestressStudioConfig::default());
        let mesh = build_rtc_prestress_mesh(&PrestressGpuInput {
            report: &report,
            heading_deg: 10.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 18.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.prestress
            .upload(&device, &queue, "prestress_p84_5", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84.5 Prestress WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_6_webgpu_composite_mesh_and_real_gpu_render() {
        use crate::core::composite_studio::{evaluate_composite_studio, CompositeStudioConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_composite_studio(&proj, None, &CompositeStudioConfig::default());
        let mesh = build_rtc_composite_mesh(&CompositeGpuInput {
            report: &report,
            heading_deg: 10.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 19.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.composite_deck
            .upload(&device, &queue, "composite_p84_6", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84.6 Composite Deck WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_7_webgpu_reliability_mesh_and_real_gpu_render() {
        use crate::core::reliability_studio::{evaluate_reliability_studio, ReliabilityStudioConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_reliability_studio(&proj, None, &ReliabilityStudioConfig::default());
        let mesh = build_rtc_reliability_mesh(&ReliabilityGpuInput {
            report: &report,
            heading_deg: 10.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 20.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.reliability_form
            .upload(&device, &queue, "reliability_p84_7", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84.7 Reliability FORM/SORM WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }

    #[test]
    fn test_phase84_8_webgpu_foundation_mesh_and_real_gpu_render() {
        use crate::core::foundation_studio::{evaluate_foundation_studio, FoundationStudioConfig};

        let proj = BimProject::sample_bimcraft_building();
        let report = evaluate_foundation_studio(&proj, None, &FoundationStudioConfig::default());
        let mesh = build_rtc_foundation_mesh(&FoundationGpuInput {
            report: &report,
            heading_deg: 10.0,
            visible: true,
        });
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.iter().all(|v| (v.meta[0] - 21.0).abs() < 1e-3));

        fn poll_fut<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Ok(adapter) = poll_fut(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })) else {
            return;
        };
        let Ok((device, queue)) =
            poll_fut(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        let val_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let fmt = wgpu::TextureFormat::Bgra8Unorm;
        let mut res = WebGpuEarthResources::new(&device, fmt);
        res.deep_foundation
            .upload(&device, &queue, "foundation_p84_8", 1, &mesh);

        let mut cam = GlobeCamera::default();
        cam.tilt_deg = 44.0;
        let xf = GlobeScreenTransform::from_view(
            Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(64.0, 64.0)),
            -6.2088,
            106.8456,
            17.5,
            &cam,
        );
        let uniforms = RtcCameraUniforms::from_globe_xform(
            &xf,
            VectorEarthTheme::CoastalBlueprint,
            &cam,
            (-6.2088, 106.8456),
            0.5,
            true,
        );
        queue.write_buffer(
            &res.planet_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_planet_pass_uniforms()),
        );
        queue.write_buffer(
            &res.site_uniform_buf,
            0,
            struct_as_u8_slice(&uniforms.to_site_pass_uniforms()),
        );
        res.ensure_targets(&device, 64, 64);

        let mut encoder = device.create_command_encoder(&Default::default());
        res.render_offscreen_passes(&mut encoder, true);
        queue.submit([encoder.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let val_err = poll_fut(val_scope.pop());
        assert!(
            val_err.is_none(),
            "Phase 84.8 Deep Foundation WGSL pass triggered WebGPU validation error: {val_err:?}"
        );
    }
