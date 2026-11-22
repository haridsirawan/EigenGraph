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
            let gpu_sp = u_orb
                .project_planet_to_screen([p_w[0] as f32, p_w[1] as f32, p_w[2] as f32], rect)
                .expect("gpu planet proj");
            let err_px = (cpu_sp - gpu_sp).length();
            assert!(
                err_px < 0.05,
                "Planetary MVP mismatch at ({lat},{lon}): cpu={cpu_sp:?} gpu={gpu_sp:?} err={err_px:.4}px"
            );
        }

        // 2. Street-scale Floating-Origin RTC matrix parity (zoom = 18.5, zero f32 jitter!)
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
            let gpu_sp = u_site
                .project_site_to_screen([east_m as f32, north_m as f32, up_m as f32], rect)
                .expect("gpu rtc site proj");
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
                min_ari: 0.84,
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
                neighbor_min_ari: 0.58,
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
