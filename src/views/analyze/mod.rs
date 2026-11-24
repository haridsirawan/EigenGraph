//! views/analyze/mod.rs — Multi-Hazard Geo-Structural Analysis & Multi-Code Verification Suite.
//! Modularized into grouped engineering disciplines for Phase 30 responsive workflows.

pub mod core_fea;
pub mod extended_mechanics;
pub mod extreme_hazards;
pub mod header;
pub mod member_design;
pub mod seismic_dynamic;

use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Pos2, RichText, ScrollArea, Sense,
    Stroke, Ui, Vec2,
};
use crate::core::PnaLocation;
use crate::features::{FeatureId, FeatureRegistry};
use crate::icons::{self, AppIcon};
use crate::models::StructuralMember;
use crate::theme::MacTheme;
use crate::ui_kit;

/// Top-level engineering discipline groupings organizing the 22 Analysis capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyzeCategory {
    CoreFeaAndEigenGate,
    SeismicAndDynamics,
    CodeMemberDesign,
    ExtremeHazardsAndResilience,
}

impl AnalyzeCategory {
    pub fn all() -> &'static [AnalyzeCategory] {
        &[
            Self::CoreFeaAndEigenGate,
            Self::SeismicAndDynamics,
            Self::CodeMemberDesign,
            Self::ExtremeHazardsAndResilience,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::CoreFeaAndEigenGate => "⚡ Core FEA & EigenGate",
            Self::SeismicAndDynamics => "🌊 Seismic & Dynamics",
            Self::CodeMemberDesign => "🏗️ Code Member Design",
            Self::ExtremeHazardsAndResilience => "🛡️ Extreme Hazards & Resilience",
        }
    }

    pub fn default_subtab(&self) -> AnalyzeSubTab {
        match self {
            Self::CoreFeaAndEigenGate => AnalyzeSubTab::RedundancySvd,
            Self::SeismicAndDynamics => AnalyzeSubTab::ModalDynamics,
            Self::CodeMemberDesign => AnalyzeSubTab::ConcretePmInteraction,
            Self::ExtremeHazardsAndResilience => AnalyzeSubTab::FemaP58LossAssessment,
        }
    }

    pub fn subtabs(&self) -> &'static [(AnalyzeSubTab, &'static str)] {
        match self {
            Self::CoreFeaAndEigenGate => &[
                (AnalyzeSubTab::RedundancySvd, "⚡ SVD Guard & Redundancy"),
                (AnalyzeSubTab::MemberCriticality, "🎯 EigenGate Sensitivity Matrix"),
                (AnalyzeSubTab::DisplacementForces, "📊 Forces & Deflection"),
                (AnalyzeSubTab::Frame3dInteraction, "📐 3D Frame P-M-V"),
                (AnalyzeSubTab::CascadingCollapse, "💥 Cascading Collapse"),
                (AnalyzeSubTab::MovingLoadInfluence, "🌉 Moving Load & Influence (SIM-06)"),
            ],
            Self::SeismicAndDynamics => &[
                (AnalyzeSubTab::ModalDynamics, "🌊 SNI 1726 Modal"),
                (AnalyzeSubTab::NonlinearPushover, "📉 Pushover P-Δ"),
                (AnalyzeSubTab::GeotechSsi, "🌱 Geotech & SSI"),
                (AnalyzeSubTab::SlopeRetainingStability, "⛰️ Retaining & Slope (SIM-07)"),
                (AnalyzeSubTab::BaseIsolationDesign, "🛡️ Base Isolation & Dampers"),
                (AnalyzeSubTab::TimeHistoryAnalysis, "📈 NL-THA Time-History"),
            ],
            Self::CodeMemberDesign => &[
                (AnalyzeSubTab::ConcretePmInteraction, "🧱 Concrete P-M (SNI/ACI)"),
                (AnalyzeSubTab::SteelConnection, "🔩 Steel Connections"),
                (AnalyzeSubTab::WindEngineering, "🌪️ ASCE 7 Wind"),
                (AnalyzeSubTab::CompositeDesign, "🥞 AISC Composite"),
                (AnalyzeSubTab::PrestressedDesign, "🪢 PT Concrete (ACI 318)"),
                (AnalyzeSubTab::ShearWallDesign, "🏢 RC Shear Walls & Coupling"),
                (AnalyzeSubTab::FloorVibrationFatigue, "〰️ Floor Vib & Fatigue (SIM-08)"),
            ],
            Self::ExtremeHazardsAndResilience => &[
                (AnalyzeSubTab::FemaP58LossAssessment, "🏛️ FEMA P-58 Loss & REDi"),
                (AnalyzeSubTab::BlastResistantDesign, "💣 Blast & Explosive (UFC/ASCE)"),
                (AnalyzeSubTab::StructuralFireEngineering, "🔥 Structural Fire (AISC/EC3)"),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyzeSubTab {
    RedundancySvd,
    MemberCriticality,
    DisplacementForces,
    Frame3dInteraction,
    ConcretePmInteraction,
    CascadingCollapse,
    NonlinearPushover,
    ModalDynamics,
    GeotechSsi,
    SteelConnection,
    WindEngineering,
    CompositeDesign,
    PrestressedDesign,
    ShearWallDesign,
    BaseIsolationDesign,
    TimeHistoryAnalysis,
    FemaP58LossAssessment,
    BlastResistantDesign,
    StructuralFireEngineering,
    MovingLoadInfluence,
    SlopeRetainingStability,
    FloorVibrationFatigue,
}

impl AnalyzeSubTab {
    pub fn category(&self) -> AnalyzeCategory {
        match self {
            Self::RedundancySvd
            | Self::MemberCriticality
            | Self::DisplacementForces
            | Self::Frame3dInteraction
            | Self::CascadingCollapse
            | Self::MovingLoadInfluence => AnalyzeCategory::CoreFeaAndEigenGate,

            Self::ModalDynamics
            | Self::NonlinearPushover
            | Self::GeotechSsi
            | Self::SlopeRetainingStability
            | Self::BaseIsolationDesign
            | Self::TimeHistoryAnalysis => AnalyzeCategory::SeismicAndDynamics,

            Self::ConcretePmInteraction
            | Self::SteelConnection
            | Self::WindEngineering
            | Self::CompositeDesign
            | Self::PrestressedDesign
            | Self::ShearWallDesign
            | Self::FloorVibrationFatigue => AnalyzeCategory::CodeMemberDesign,

            Self::FemaP58LossAssessment
            | Self::BlastResistantDesign
            | Self::StructuralFireEngineering => AnalyzeCategory::ExtremeHazardsAndResilience,
        }
    }
}

pub struct AnalyzeView {
    pub active_subtab: AnalyzeSubTab,
    pub published_to_dossier: bool,
    pub export_status: Option<String>,
    pub initial_disrupt_input: String,
    pub daf_val: f32,
    pub last_cascade_report: Option<crate::core::CascadingCollapseReport>,
    pub site_class: crate::core::SiteClass,
    pub ss_val: f32,
    pub s1_val: f32,
    pub r_val: f32,
    pub modal_report: Option<crate::core::ModalAnalysisReport>,
    // Concrete Column P-M parameters (SNI 2847:2019 / ACI 318-19)
    pub concrete_col_width_mm: f32,
    pub concrete_col_height_mm: f32,
    pub concrete_fc_mpa: f32,
    pub concrete_fy_mpa: f32,
    pub concrete_cover_mm: f32,
    pub concrete_bars_per_face_x: usize,
    pub concrete_bars_intermediate_y: usize,
    pub concrete_bar_dia_mm: f32,
    pub concrete_demand_pu_kn: f32,
    pub concrete_demand_mu_knm: f32,
    pub concrete_demand_muy_knm: f32,
    pub concrete_is_spiral: bool,
    pub woodbury_disrupt_elem_id: usize,
    pub woodbury_lambda_val: f32,
    pub modal_damping_ratio: f32,
    pub pushover_load_pattern: crate::core::LateralLoadPattern,
    pub geotech_soil_type: crate::core::SoilType,
    pub geotech_footing_w: f32,
    pub geotech_footing_l: f32,
    pub geotech_piles_x: usize,
    pub geotech_piles_y: usize,
    // Steel Connection Parameters (AISC 360-16 / SNI 1729:2020)
    pub conn_type: crate::core::ConnectionType,
    pub conn_bolt_grade: crate::core::BoltGrade,
    pub conn_bolt_dia_mm: f32,
    pub conn_bolt_rows: usize,
    pub conn_plate_thick_mm: f32,
    pub conn_demand_vu_kn: f32,
    pub conn_demand_mu_knm: f32,
    // Wind Engineering Parameters (ASCE 7-22 / SNI 1727:2020)
    pub wind_v_ms: f32,
    pub wind_exposure: crate::core::ExposureCategory,
    pub wind_risk: crate::core::RiskCategory,
    pub wind_enclosure: crate::core::EnclosureClassification,
    pub wind_building_height_m: f32,
    pub wind_building_width_m: f32,
    pub wind_building_length_m: f32,
    pub wind_num_stories: usize,
    pub wind_t1_s: f32,
    pub wind_damping: f32,
    // Composite Beam Parameters (AISC 360-16 / SNI 1729:2020)
    pub comp_span_l_m: f32,
    pub comp_beam_spacing_s_m: f32,
    pub comp_steel_depth_mm: f32,
    pub comp_steel_bf_mm: f32,
    pub comp_steel_tf_mm: f32,
    pub comp_steel_tw_mm: f32,
    pub comp_steel_fy_mpa: f32,
    pub comp_slab_tc_mm: f32,
    pub comp_deck_hr_mm: f32,
    pub comp_concrete_fc_mpa: f32,
    pub comp_stud_dia_mm: f32,
    pub comp_studs_count: usize,
    pub comp_unshored: bool,
    pub comp_demand_mu_knm: f32,
    pub comp_live_load_kn_m: f32,
    // Prestressed & Post-Tensioned Concrete Parameters (ACI 318-19 / SNI 2847:2019)
    pub pt_span_length_m: f32,
    pub pt_girder_spacing_m: f32,
    pub pt_archetype: crate::core::GirderSectionArchetype,
    pub pt_method: crate::core::PrestressingMethod,
    pub pt_profile: crate::core::TendonProfileKind,
    pub pt_strand_dia_mm: f32,
    pub pt_num_strands: usize,
    pub pt_eccentricity_mid_mm: f32,
    pub pt_eccentricity_end_mm: f32,
    pub pt_fc_mpa: f32,
    pub pt_fci_mpa: f32,
    pub pt_sdl_kn_m: f32,
    pub pt_live_load_kn_m: f32,
    // RC Special Structural Shear Walls & Coupling Beams (ACI 318-19 / SNI 2847:2019)
    pub sw_length_lw_m: f32,
    pub sw_height_hw_m: f32,
    pub sw_thickness_tw_mm: f32,
    pub sw_story_height_hu_m: f32,
    pub sw_fc_mpa: f32,
    pub sw_fy_mpa: f32,
    pub sw_fyt_mpa: f32,
    pub sw_demand_pu_kn: f32,
    pub sw_demand_mu_knm: f32,
    pub sw_demand_vu_kn: f32,
    pub sw_drift_ratio: f32,
    pub sw_cb_clear_span_m: f32,
    pub sw_cb_depth_m: f32,
    pub sw_cb_width_mm: f32,
    pub sw_cb_demand_vu_kn: f32,
    pub sw_cb_bar_dia_mm: f32,
    pub sw_cb_bars_per_group: usize,
    // Seismic Base Isolation & Supplementary Energy Dissipation (ASCE 7-22 Ch 17-18 / SNI 1726)
    pub iso_type: crate::core::IsolatorType,
    pub iso_seismic_weight_w_kn: f32,
    pub iso_num_bearings: usize,
    pub iso_sm1_g: f32,
    pub iso_sms_g: f32,
    pub iso_fixed_period_s: f32,
    pub iso_superstructure_ri: f32,
    pub iso_plan_b_m: f32,
    pub iso_plan_d_m: f32,
    pub iso_eccentricity_e_m: f32,
    pub iso_lrb_diameter_mm: f32,
    pub iso_lrb_lead_dia_mm: f32,
    pub iso_lrb_rubber_layer_mm: f32,
    pub iso_lrb_num_layers: usize,
    pub iso_lrb_shear_mod_g: f32,
    pub iso_fps_radius_mm: f32,
    pub iso_fps_mu_fast: f32,
    pub iso_fps_housing_dia_mm: f32,
    pub iso_fvd_c_kn_s: f32,
    pub iso_fvd_alpha: f32,
    pub iso_fvd_stroke_limit_mm: f32,
    pub iso_fvd_count: usize,
    // Nonlinear Dynamic Time-History Analysis (NL-THA - ASCE 7-22 Ch 16 / SNI 1726 Pasal 11)
    pub tha_record: crate::core::GroundMotionRecord,
    pub tha_scale_factor: f32,
    pub tha_dt: f32,
    pub tha_duration: f32,
    pub tha_building_height_m: f32,
    pub tha_num_stories: usize,
    pub tha_seismic_weight_kn: f32,
    pub tha_t1_s: f32,
    pub tha_damping: f32,
    pub tha_cy: f32,
    pub tha_alpha_p: f32,
    // FEMA P-58 Seismic Performance Assessment & REDi Loss Engine
    pub fema_project_name: String,
    pub fema_asset_value_usd: f64,
    pub fema_floor_area_m2: f64,
    pub fema_num_stories: usize,
    pub fema_story_height_m: f64,
    pub fema_site_pga_g: f64,
    pub fema_spectral_sa1_g: f64,
    pub fema_workers_per_floor: usize,
    pub fema_daily_revenue_usd: f64,
    pub fema_target_redi_tier: crate::core::REDiResilienceTier,
    // Blast Resistant Design & Explosive Dynamics (UFC 3-340-02 / ASCE 59-11)
    pub blast_project_name: String,
    pub blast_charge_kg: f32,
    pub blast_explosive_type: crate::core::ExplosiveType,
    pub blast_burst_type: crate::core::BurstType,
    pub blast_standoff_m: f32,
    pub blast_component_kind: crate::core::BlastComponentKind,
    pub blast_support_condition: crate::core::SupportCondition,
    pub blast_span_length_m: f32,
    pub blast_tributary_width_m: f32,
    pub blast_thickness_m: f32,
    pub blast_fc_mpa: f32,
    pub blast_fy_mpa: f32,
    pub blast_rebar_percent: f32,
    pub blast_target_damage: crate::core::UfcDamageLevel,
    // Structural Fire Engineering & Coupled Thermo-Mechanical Response (AISC 360-16 / EN 1993-1-2)
    pub fire_project_name: String,
    pub fire_curve_kind: crate::core::FireCurveKind,
    pub fire_member_kind: crate::core::FireMemberKind,
    pub fire_protection_kind: crate::core::FireProtectionKind,
    pub fire_protection_thickness_mm: f32,
    pub fire_duration_minutes: f32,
    pub fire_target_frr_minutes: f32,
    pub fire_span_m: f32,
    pub fire_depth_mm: f32,
    pub fire_width_mm: f32,
    pub fire_web_thk_mm: f32,
    pub fire_flange_thk_mm: f32,
    pub fire_cover_mm: f32,
    pub fire_aggregate_kind: crate::core::ConcreteAggregateKind,
    pub fire_steel_fy_mpa: f32,
    pub fire_concrete_fc_mpa: f32,
    pub fire_moment_demand_knm: f32,
    pub fire_axial_demand_kn: f32,
    pub fire_thermal_restraint: crate::core::ThermalRestraintKind,
    // Phase 38: Interactive Bridge Moving Load & Influence Line (SIM-06)
    pub ml_span_m: f32,
    pub ml_truck_pos_frac: f32,
    pub ml_standard_idx: usize,
    pub ml_dynamic_im: f32,
    pub ml_two_span: bool,
    // Phase 39: Retaining Wall & Bishop Circular Slope Stability (SIM-07)
    pub rw_wall_height_m: f32,
    pub rw_base_width_m: f32,
    pub rw_backfill_phi_deg: f32,
    pub rw_cohesion_kpa: f32,
    pub rw_unit_weight_knm3: f32,
    pub rw_kh_seismic: f32,
    pub rw_shear_key: bool,
    pub rw_surcharge_kpa: f32,
    // Phase 41: Floor Vibration & Steel S-N Fatigue (SIM-08)
    pub vib_span_m: f32,
    pub vib_spacing_m: f32,
    pub vib_slab_mm: f32,
    pub vib_damping_beta: f32,
    pub vib_occupancy_idx: usize,
    pub fatigue_cat_idx: usize,
    pub fatigue_stress_range_mpa: f32,
    pub fatigue_cycles_millions: f32,
    pub request_bim_fem_sync: bool,
    pub last_bim_sync_label: String,
    pub synced_bim_element_count: usize,
    pub last_modal_bim_sig: (usize, usize, usize),
    // Phase 45: Earth Site Hazard Coupling + Bidirectional Geometry <-> Analysis & Drafting Bridge
    pub site_coupling: crate::core::SiteEnvironmentalCoupling,
    pub linked_from_geometry_note: Option<String>,
    pub requested_geometry_jump: Option<(
        Option<usize>,
        Option<crate::views::build::Render3DMode>,
        Option<String>,
    )>,
    pub requested_drafting_jump: Option<crate::views::parametric::DraftingSubTab>,
}

impl Default for AnalyzeView {
    fn default() -> Self {
        Self {
            active_subtab: AnalyzeSubTab::RedundancySvd,
            published_to_dossier: false,
            export_status: None,
            initial_disrupt_input: "1, 2".to_string(),
            daf_val: 1.20,
            last_cascade_report: None,
            site_class: crate::core::SiteClass::SD,
            ss_val: 1.20,
            s1_val: 0.50,
            r_val: 8.0,
            modal_report: None,
            concrete_col_width_mm: 400.0,
            concrete_col_height_mm: 400.0,
            concrete_fc_mpa: 30.0,
            concrete_fy_mpa: 420.0,
            concrete_cover_mm: 40.0,
            concrete_bars_per_face_x: 3,
            concrete_bars_intermediate_y: 1,
            concrete_bar_dia_mm: 22.0,
            concrete_demand_pu_kn: 1500.0,
            concrete_demand_mu_knm: 120.0,
            concrete_demand_muy_knm: 80.0,
            concrete_is_spiral: false,
            woodbury_disrupt_elem_id: 1,
            woodbury_lambda_val: 0.0,
            modal_damping_ratio: 0.05,
            pushover_load_pattern: crate::core::LateralLoadPattern::FirstModeModal,
            geotech_soil_type: crate::core::SoilType::MediumSand,
            geotech_footing_w: 3.5,
            geotech_footing_l: 3.5,
            geotech_piles_x: 3,
            geotech_piles_y: 3,
            conn_type: crate::core::ConnectionType::ExtendedEndPlate4E,
            conn_bolt_grade: crate::core::BoltGrade::A325X,
            conn_bolt_dia_mm: 22.0,
            conn_bolt_rows: 4,
            conn_plate_thick_mm: 20.0,
            conn_demand_vu_kn: 120.0,
            conn_demand_mu_knm: 240.0,
            wind_v_ms: 38.0,
            wind_exposure: crate::core::ExposureCategory::ExposureB,
            wind_risk: crate::core::RiskCategory::CategoryII,
            wind_enclosure: crate::core::EnclosureClassification::Enclosed,
            wind_building_height_m: 48.0,
            wind_building_width_m: 24.0,
            wind_building_length_m: 36.0,
            wind_num_stories: 12,
            wind_t1_s: 1.20,
            wind_damping: 0.02,
            comp_span_l_m: 9.0,
            comp_beam_spacing_s_m: 3.0,
            comp_steel_depth_mm: 450.0,
            comp_steel_bf_mm: 152.0,
            comp_steel_tf_mm: 10.8,
            comp_steel_tw_mm: 7.6,
            comp_steel_fy_mpa: 345.0,
            comp_slab_tc_mm: 85.0,
            comp_deck_hr_mm: 75.0,
            comp_concrete_fc_mpa: 30.0,
            comp_stud_dia_mm: 19.0,
            comp_studs_count: 36,
            comp_unshored: true,
            comp_demand_mu_knm: 280.0,
            comp_live_load_kn_m: 12.0,
            pt_span_length_m: 30.0,
            pt_girder_spacing_m: 2.4,
            pt_archetype: crate::core::GirderSectionArchetype::AashtoTypeIV,
            pt_method: crate::core::PrestressingMethod::PostTensioned,
            pt_profile: crate::core::TendonProfileKind::Parabolic,
            pt_strand_dia_mm: 15.24,
            pt_num_strands: 36,
            pt_eccentricity_mid_mm: 480.0,
            pt_eccentricity_end_mm: 100.0,
            pt_fc_mpa: 50.0,
            pt_fci_mpa: 35.0,
            pt_sdl_kn_m: 4.0,
            pt_live_load_kn_m: 12.0,
            sw_length_lw_m: 6.0,
            sw_height_hw_m: 36.0,
            sw_thickness_tw_mm: 350.0,
            sw_story_height_hu_m: 3.5,
            sw_fc_mpa: 35.0,
            sw_fy_mpa: 420.0,
            sw_fyt_mpa: 420.0,
            sw_demand_pu_kn: 3600.0,
            sw_demand_mu_knm: 14500.0,
            sw_demand_vu_kn: 1850.0,
            sw_drift_ratio: 0.012,
            sw_cb_clear_span_m: 1.8,
            sw_cb_depth_m: 0.9,
            sw_cb_width_mm: 350.0,
            sw_cb_demand_vu_kn: 620.0,
            sw_cb_bar_dia_mm: 25.0,
            sw_cb_bars_per_group: 4,
            iso_type: crate::core::IsolatorType::LeadRubberBearing,
            iso_seismic_weight_w_kn: 96000.0,
            iso_num_bearings: 24,
            iso_sm1_g: 0.65,
            iso_sms_g: 1.25,
            iso_fixed_period_s: 0.85,
            iso_superstructure_ri: 1.68,
            iso_plan_b_m: 36.0,
            iso_plan_d_m: 24.0,
            iso_eccentricity_e_m: 1.8,
            iso_lrb_diameter_mm: 850.0,
            iso_lrb_lead_dia_mm: 160.0,
            iso_lrb_rubber_layer_mm: 7.0,
            iso_lrb_num_layers: 28,
            iso_lrb_shear_mod_g: 0.45,
            iso_fps_radius_mm: 2200.0,
            iso_fps_mu_fast: 0.055,
            iso_fps_housing_dia_mm: 1000.0,
            iso_fvd_c_kn_s: 240.0,
            iso_fvd_alpha: 0.35,
            iso_fvd_stroke_limit_mm: 250.0,
            iso_fvd_count: 16,
            tha_record: crate::core::GroundMotionRecord::NorthridgeSylmar1994,
            tha_scale_factor: 1.15,
            tha_dt: 0.010,
            tha_duration: 25.0,
            tha_building_height_m: 60.0,
            tha_num_stories: 16,
            tha_seismic_weight_kn: 160_000.0,
            tha_t1_s: 1.65,
            tha_damping: 0.025,
            tha_cy: 0.12,
            tha_alpha_p: 0.035,
            fema_project_name: "Metropolitan Medical Center Tower".to_string(),
            fema_asset_value_usd: 85_000_000.0,
            fema_floor_area_m2: 24_000.0,
            fema_num_stories: 10,
            fema_story_height_m: 3.80,
            fema_site_pga_g: 0.45,
            fema_spectral_sa1_g: 0.72,
            fema_workers_per_floor: 12,
            fema_daily_revenue_usd: 120_000.0,
            fema_target_redi_tier: crate::core::REDiResilienceTier::Gold,
            blast_project_name: "Strategic Embassy Perimeter Blast Gate & Bunker Wall".to_string(),
            blast_charge_kg: 250.0,
            blast_explosive_type: crate::core::ExplosiveType::C4,
            blast_burst_type: crate::core::BurstType::SurfaceBurst,
            blast_standoff_m: 15.0,
            blast_component_kind: crate::core::BlastComponentKind::RcWall,
            blast_support_condition: crate::core::SupportCondition::FixedFixed,
            blast_span_length_m: 4.20,
            blast_tributary_width_m: 1.00,
            blast_thickness_m: 0.40,
            blast_fc_mpa: 40.0,
            blast_fy_mpa: 420.0,
            blast_rebar_percent: 1.20,
            blast_target_damage: crate::core::UfcDamageLevel::Low,
            fire_project_name: "Metropolitan Commercial Center Multi-Story Frame".to_string(),
            fire_curve_kind: crate::core::FireCurveKind::Iso834Standard,
            fire_member_kind: crate::core::FireMemberKind::SteelBeam,
            fire_protection_kind: crate::core::FireProtectionKind::SprayedMineralFiber,
            fire_protection_thickness_mm: 20.0,
            fire_duration_minutes: 120.0,
            fire_target_frr_minutes: 90.0,
            fire_span_m: 7.50,
            fire_depth_mm: 457.0,
            fire_width_mm: 190.0,
            fire_web_thk_mm: 9.0,
            fire_flange_thk_mm: 14.5,
            fire_cover_mm: 40.0,
            fire_aggregate_kind: crate::core::ConcreteAggregateKind::Siliceous,
            fire_steel_fy_mpa: 355.0,
            fire_concrete_fc_mpa: 35.0,
            fire_moment_demand_knm: 240.0,
            fire_axial_demand_kn: 600.0,
            fire_thermal_restraint: crate::core::ThermalRestraintKind::ModeratelyRestrained,
            ml_span_m: 36.0,
            ml_truck_pos_frac: 0.45,
            ml_standard_idx: 0,
            ml_dynamic_im: 0.33,
            ml_two_span: false,
            rw_wall_height_m: 5.5,
            rw_base_width_m: 3.8,
            rw_backfill_phi_deg: 32.0,
            rw_cohesion_kpa: 12.0,
            rw_unit_weight_knm3: 18.5,
            rw_kh_seismic: 0.18,
            rw_shear_key: true,
            rw_surcharge_kpa: 15.0,
            vib_span_m: 9.0,
            vib_spacing_m: 3.0,
            vib_slab_mm: 130.0,
            vib_damping_beta: 0.03,
            vib_occupancy_idx: 0,
            fatigue_cat_idx: 1,
            fatigue_stress_range_mpa: 68.0,
            fatigue_cycles_millions: 2.5,
            request_bim_fem_sync: false,
            last_bim_sync_label: "Auto-sync active".to_string(),
            synced_bim_element_count: 0,
            last_modal_bim_sig: (usize::MAX, 0, 0),
            site_coupling: crate::core::SiteEnvironmentalCoupling::default(),
            linked_from_geometry_note: None,
            requested_geometry_jump: None,
            requested_drafting_jump: None,
        }
    }
}

impl AnalyzeView {
    /// Resolves the active structural FEM model from the live `BimProject`, falling back to
    /// the built-in Warren Truss reference model when no BIM frame elements exist yet.
    pub fn resolve_active_model(
        bim_project: &crate::core::BimProject,
    ) -> (crate::core::StructuralModel, bool) {
        if !bim_project.is_empty() {
            let fem = bim_project.to_structural_model();
            if fem.elements.len() >= 2 && fem.nodes.len() >= 2 {
                return (fem, true);
            }
        }
        let mut engine = crate::core::ParametricEngine::new();
        let fallback = engine
            .generate_warren_truss()
            .unwrap_or_else(|_| crate::core::StructuralModel::new("Reference Model"));
        (fallback, false)
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &MacTheme,
        members: &[StructuralMember],
        bim_project: &crate::core::BimProject,
        features: &mut FeatureRegistry,
    ) {
        let cur_sig = (
            bim_project.element_count(),
            bim_project.columns.len(),
            bim_project.runs.len(),
        );
        if cur_sig != self.last_modal_bim_sig {
            self.last_modal_bim_sig = cur_sig;
            self.modal_report = None;
            self.last_cascade_report = None;
        }
        ui.vertical(|ui| {
            Frame::new().inner_margin(Margin::symmetric(0, 8)).show(ui, |ui| {
                let full_w = ui.available_width().max(160.0);
                ui.set_width(full_w);
                ui.set_max_width(full_w);
                // 1. TWO-TIER GROUPED ANALYSIS NAVIGATION & EXPORT HEADER
                self.render_subtabs_header(ui, theme);

                let mut dismiss_status = false;
                if let Some(ref status_msg) = self.export_status {
                    ui.add_space(10.0);
                    theme.card_frame().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            icons::icon_label(
                                ui,
                                AppIcon::CheckCircle,
                                13.0,
                                theme.accent_green,
                                "AEC EXPORT READY:",
                                11.5,
                                theme.accent_green,
                                true,
                            );
                            ui.label(
                                RichText::new(status_msg)
                                    .size(11.5)
                                    .color(theme.text_primary),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if icons::icon_only_button(
                                    ui,
                                    AppIcon::Close,
                                    theme.text_muted,
                                    theme.bg_card,
                                    Stroke::NONE,
                                    CornerRadius::same(4),
                                    Vec2::splat(22.0),
                                    "Dismiss",
                                )
                                .clicked()
                                {
                                    dismiss_status = true;
                                }
                            });
                        });
                    });
                }
                if dismiss_status {
                    self.export_status = None;
                }

                ui.add_space(12.0);

                // 2. PHASE 30 FROZEN LOCKED FEATURE SLOT: LIVE BIM -> FEM REACTIVE SYNC
                self.render_bim_sync_gate(ui, theme, features);

                ui.add_space(14.0);

                // 3. ACTIVE SUB-TAB CONTENT
                match self.active_subtab {
                    AnalyzeSubTab::RedundancySvd => self.render_redundancy_and_svd(ui, theme, members),
                    AnalyzeSubTab::MemberCriticality => self.render_member_criticality(ui, theme, members, bim_project),
                    AnalyzeSubTab::DisplacementForces => self.render_displacement_and_forces(ui, theme, members),
                    AnalyzeSubTab::Frame3dInteraction => self.render_frame_3d_interaction(ui, theme, members),
                    AnalyzeSubTab::ConcretePmInteraction => self.render_concrete_pm_interaction(ui, theme),
                    AnalyzeSubTab::CascadingCollapse => self.render_cascading_collapse(ui, theme, bim_project),
                    AnalyzeSubTab::NonlinearPushover => self.render_nonlinear_pushover(ui, theme, bim_project),
                    AnalyzeSubTab::ModalDynamics => self.render_modal_dynamics(ui, theme, bim_project),
                    AnalyzeSubTab::GeotechSsi => self.render_geotech_ssi(ui, theme, bim_project),
                    AnalyzeSubTab::SteelConnection => self.render_steel_connection(ui, theme),
                    AnalyzeSubTab::WindEngineering => self.render_wind_engineering(ui, theme),
                    AnalyzeSubTab::CompositeDesign => self.render_composite_design(ui, theme),
                    AnalyzeSubTab::PrestressedDesign => self.render_prestressed_design(ui, theme),
                    AnalyzeSubTab::ShearWallDesign => self.render_shear_wall_design(ui, theme),
                    AnalyzeSubTab::BaseIsolationDesign => self.render_base_isolation_design(ui, theme),
                    AnalyzeSubTab::TimeHistoryAnalysis => self.render_time_history_analysis(ui, theme),
                    AnalyzeSubTab::FemaP58LossAssessment => self.render_fema_p58_assessment(ui, theme),
                    AnalyzeSubTab::BlastResistantDesign => self.render_blast_resistant_design(ui, theme),
                    AnalyzeSubTab::StructuralFireEngineering => self.render_fire_engineering(ui, theme),
                    AnalyzeSubTab::MovingLoadInfluence => self.render_moving_load_influence(ui, theme, features),
                    AnalyzeSubTab::SlopeRetainingStability => self.render_slope_retaining_stability(ui, theme, features),
                    AnalyzeSubTab::FloorVibrationFatigue => self.render_floor_vibration_fatigue(ui, theme, features),
                }

                ui.add_space(64.0);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_view_live_bim_model_resolution() {
        let empty = crate::core::BimProject::empty();
        let (fallback_model, is_live_empty) = AnalyzeView::resolve_active_model(&empty);
        assert!(!is_live_empty);
        assert!(!fallback_model.elements.is_empty());

        let sample = crate::core::BimProject::sample_bimcraft_building();
        let (live_model, is_live) = AnalyzeView::resolve_active_model(&sample);
        assert!(is_live);
        assert!(live_model.elements.len() >= 4);
    }
}
