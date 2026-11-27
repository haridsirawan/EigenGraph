//! mep_engine.rs — Update `v0.8.7.5` (`5/10` -> `v0.8.8`):
//! Building MEP Systems Engine (`HVAC`, `Plumbing`, `Electrical`, `Fire Sprinkler`),
//! 3D ENU Conduit & Equipment Synthesis, Solar OTTV Coupling & IFCX Overlay (`eigengraph::mep::systems`).

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::lifecycle_ops::{
    compute_mep_engineering_sizing, MepSizingInput, SiteEnvironmentalCoupling,
};
use crate::core::solar_envelope::SolarEnvelopeReport;

/// Canonical EigenGraph IFCX attribute namespace for Building MEP Systems (`IfcBuilding`).
pub const NS_EG_MEP_SYSTEMS: &str = "eigengraph::mep::systems";

/// Four primary Building MEP engineering disciplines rendered in 3D WebGPU and checked for clashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MepDiscipline {
    Hvac,
    Plumbing,
    Electrical,
    FireSprinkler,
}

impl MepDiscipline {
    pub const ALL: [Self; 4] = [
        Self::Hvac,
        Self::Plumbing,
        Self::Electrical,
        Self::FireSprinkler,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Hvac => "HVAC Air & Chilled Water",
            Self::Plumbing => "Domestic Water & Sanitary Plumbing",
            Self::Electrical => "3-Phase Power & Cable Trays",
            Self::FireSprinkler => "NFPA 13 Fire Sprinkler & Standpipe",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Hvac => "HVAC",
            Self::Plumbing => "Plumbing",
            Self::Electrical => "Electrical",
            Self::FireSprinkler => "Fire",
        }
    }

    /// Standard AEC discipline color coding (Cyan-Blue HVAC, Emerald Plumbing, Amber Electrical, Crimson Fire).
    pub fn rgba(self) -> [f32; 4] {
        match self {
            Self::Hvac => [0.22, 0.74, 0.97, 0.92],
            Self::Plumbing => [0.13, 0.77, 0.37, 0.92],
            Self::Electrical => [0.96, 0.62, 0.04, 0.92],
            Self::FireSprinkler => [0.94, 0.27, 0.27, 0.94],
        }
    }

    pub fn discipline_code(self) -> f32 {
        match self {
            Self::Hvac => 0.0,
            Self::Plumbing => 1.0,
            Self::Electrical => 2.0,
            Self::FireSprinkler => 3.0,
        }
    }
}

/// HVAC cooling & ventilation plant archetype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HvacSystemPreset {
    VrfInverter,
    ChilledWaterAhu,
    PackagedDxDucted,
}

impl HvacSystemPreset {
    pub const ALL: [Self; 3] = [
        Self::VrfInverter,
        Self::ChilledWaterAhu,
        Self::PackagedDxDucted,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::VrfInverter => "VRF Inverter Heat-Recovery (COP 4.1)",
            Self::ChilledWaterAhu => "Central Chilled Water + AHU (COP 5.2)",
            Self::PackagedDxDucted => "Packaged Rooftop DX Ducted (COP 3.4)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::VrfInverter => "VRF",
            Self::ChilledWaterAhu => "Chilled AHU",
            Self::PackagedDxDucted => "Packaged DX",
        }
    }

    pub fn cop(self) -> f64 {
        match self {
            Self::VrfInverter => 4.1,
            Self::ChilledWaterAhu => 5.2,
            Self::PackagedDxDucted => 3.4,
        }
    }

    pub fn base_w_m2(self) -> f64 {
        match self {
            Self::VrfInverter => 122.0,
            Self::ChilledWaterAhu => 138.0,
            Self::PackagedDxDucted => 145.0,
        }
    }

    pub fn target_velocity_ms(self) -> f64 {
        match self {
            Self::VrfInverter => 6.8,
            Self::ChilledWaterAhu => 7.8,
            Self::PackagedDxDucted => 7.2,
        }
    }
}

/// NFPA 13 occupancy hazard classification governing sprinkler density and hydraulic main sizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FireHazardClass {
    LightHazard,
    OrdinaryHazardGroup1,
    OrdinaryHazardGroup2,
}

impl FireHazardClass {
    pub const ALL: [Self; 3] = [
        Self::LightHazard,
        Self::OrdinaryHazardGroup1,
        Self::OrdinaryHazardGroup2,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::LightHazard => "NFPA 13 Light Hazard (Office/Resi)",
            Self::OrdinaryHazardGroup1 => "NFPA 13 Ordinary Hazard Group 1",
            Self::OrdinaryHazardGroup2 => "NFPA 13 Ordinary Hazard Group 2",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::LightHazard => "Light",
            Self::OrdinaryHazardGroup1 => "Ord-1",
            Self::OrdinaryHazardGroup2 => "Ord-2",
        }
    }

    /// Design spray density in mm/min (L/min/m^2).
    pub fn density_mm_min(self) -> f64 {
        match self {
            Self::LightHazard => 4.1,
            Self::OrdinaryHazardGroup1 => 6.1,
            Self::OrdinaryHazardGroup2 => 8.1,
        }
    }

    /// Maximum protection coverage area per sprinkler head (m^2).
    pub fn head_coverage_m2(self) -> f64 {
        match self {
            Self::LightHazard => 18.6,
            Self::OrdinaryHazardGroup1 => 12.1,
            Self::OrdinaryHazardGroup2 => 12.1,
        }
    }

    /// Combined inside/outside hose stream allowance per NFPA 13 (L/min).
    pub fn hose_allowance_lpm(self) -> f64 {
        match self {
            Self::LightHazard => 379.0,
            Self::OrdinaryHazardGroup1 => 946.0,
            Self::OrdinaryHazardGroup2 => 946.0,
        }
    }
}

/// Persisted configuration for Building MEP Systems (`HVAC`, `Plumbing`, `Electrical`, `Fire`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MepSystemsConfig {
    pub hvac_preset: HvacSystemPreset,
    pub fire_hazard: FireHazardClass,
    pub occupant_density_m2_per_person: f64,
    pub plenum_drop_below_slab_m: f64,
    pub couple_solar_ottv: bool,
    pub show_hvac: bool,
    pub show_plumbing: bool,
    pub show_electrical: bool,
    pub show_fire: bool,
}

impl Default for MepSystemsConfig {
    fn default() -> Self {
        Self {
            hvac_preset: HvacSystemPreset::VrfInverter,
            fire_hazard: FireHazardClass::LightHazard,
            occupant_density_m2_per_person: 10.0,
            plenum_drop_below_slab_m: 0.45,
            couple_solar_ottv: true,
            show_hvac: true,
            show_plumbing: true,
            show_electrical: true,
            show_fire: true,
        }
    }
}

/// Coordinated 3D ENU duct, pipe, cable tray, or riser segment for WebGPU rendering & clash checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MepConduitSegment3D {
    pub id: usize,
    pub tag: String,
    pub discipline: MepDiscipline,
    pub story_index: usize,
    pub is_riser: bool,
    pub start_enu_m: [f64; 3],
    pub end_enu_m: [f64; 3],
    pub width_m: f64,
    pub height_m: f64,
    pub flow_label: String,
}

/// 3D MEP plant equipment node (AHU/Chiller, Main Distribution Panel, Booster Pump, Fire Pump + Tank).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MepEquipmentNode3D {
    pub tag: String,
    pub discipline: MepDiscipline,
    pub center_enu_m: [f64; 3],
    pub size_enu_m: [f64; 3],
    pub rating_label: String,
}

/// NFPA 13 automatic sprinkler & standpipe hydraulic summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FireSprinklerSummary {
    pub hazard_class: FireHazardClass,
    pub design_density_mm_min: f64,
    pub total_sprinkler_heads: usize,
    pub sprinkler_main_dn_mm: f64,
    pub fire_pump_flow_lpm: f64,
    pub fire_pump_head_kpa: f64,
    pub water_tank_m3: f64,
    pub nfpa_13_compliant: bool,
}

/// Non-destructive IFCX export payload (`eigengraph::mep::systems`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MepSystemsIfcxOverlay {
    pub site_label: String,
    pub stories: usize,
    pub total_gfa_m2: f64,
    pub hvac_cooling_tr: f64,
    pub hvac_duct_w_mm: f64,
    pub hvac_duct_h_mm: f64,
    pub plumbing_riser_dn_mm: f64,
    pub elec_demand_kva: f64,
    pub elec_main_breaker_at: usize,
    pub elec_vdrop_pct: f64,
    pub fire_pump_flow_lpm: f64,
    pub sprinkler_heads: usize,
    pub segment_count: usize,
    pub estimated_mep_cost_usd: f64,
    pub all_codes_compliant: bool,
}

/// Comprehensive Building MEP report coupling BIM geometry, site climate, and optional Solar OTTV load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MepSystemsReport {
    pub config: MepSystemsConfig,
    pub site_label: String,
    pub stories: usize,
    pub building_width_m: f64,
    pub building_depth_m: f64,
    pub total_gfa_m2: f64,
    pub occupants: usize,
    pub solar_ottv_coupled_kw: f64,
    pub cooling_kw: f64,
    pub cooling_tr: f64,
    pub hvac_elec_input_kw: f64,
    pub supply_airflow_cfm: f64,
    pub duct_width_mm: f64,
    pub duct_height_mm: f64,
    pub duct_velocity_ms: f64,
    pub duct_friction_pa_m: f64,
    pub plumbing_peak_flow_ls: f64,
    pub plumbing_pipe_dn_mm: f64,
    pub plumbing_velocity_ms: f64,
    pub plumbing_head_loss_kpa: f64,
    pub drain_slope_pct: f64,
    pub elec_demand_kva: f64,
    pub elec_design_current_a: f64,
    pub elec_breaker_at: usize,
    pub elec_cable_mm2: usize,
    pub elec_voltage_drop_pct: f64,
    pub elec_voltage_drop_ok: bool,
    pub fire: FireSprinklerSummary,
    pub segments_3d: Vec<MepConduitSegment3D>,
    pub equipment_3d: Vec<MepEquipmentNode3D>,
    pub estimated_mep_cost_usd: f64,
    pub all_codes_compliant: bool,
    pub summary_verdict: String,
}

impl MepSystemsReport {
    /// Builds a compact IFCX attribute overlay (`eigengraph::mep::systems`).
    pub fn to_ifcx_overlay(&self) -> MepSystemsIfcxOverlay {
        MepSystemsIfcxOverlay {
            site_label: self.site_label.clone(),
            stories: self.stories,
            total_gfa_m2: self.total_gfa_m2,
            hvac_cooling_tr: self.cooling_tr,
            hvac_duct_w_mm: self.duct_width_mm,
            hvac_duct_h_mm: self.duct_height_mm,
            plumbing_riser_dn_mm: self.plumbing_pipe_dn_mm,
            elec_demand_kva: self.elec_demand_kva,
            elec_main_breaker_at: self.elec_breaker_at,
            elec_vdrop_pct: self.elec_voltage_drop_pct,
            fire_pump_flow_lpm: self.fire.fire_pump_flow_lpm,
            sprinkler_heads: self.fire.total_sprinkler_heads,
            segment_count: self.segments_3d.len(),
            estimated_mep_cost_usd: self.estimated_mep_cost_usd,
            all_codes_compliant: self.all_codes_compliant,
        }
    }
}

fn infer_bim_extents_and_levels(bim: &BimProject) -> ([f64; 2], [f64; 2], Vec<f64>) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut push_pt = |x_mm: f64, y_mm: f64| {
        let xm = x_mm * 0.001;
        let ym = y_mm * 0.001;
        min_x = min_x.min(xm);
        max_x = max_x.max(xm);
        min_y = min_y.min(ym);
        max_y = max_y.max(ym);
    };
    for c in &bim.columns {
        push_pt(c.pos.0, c.pos.1);
    }
    for w in &bim.walls {
        push_pt(w.start.0, w.start.1);
        push_pt(w.end.0, w.end.1);
    }
    for r in &bim.runs {
        push_pt(r.start.0, r.start.1);
        push_pt(r.end.0, r.end.1);
    }
    for s in &bim.slabs {
        for p in &s.boundary {
            push_pt(p.0, p.1);
        }
    }
    if !min_x.is_finite() || (max_x - min_x) < 4.0 || (max_y - min_y) < 4.0 {
        min_x = -9.0;
        max_x = 9.0;
        min_y = -6.0;
        max_y = 6.0;
    }
    let mut elevs: Vec<f64> = bim.levels.iter().map(|l| l.elevation_mm * 0.001).collect();
    elevs.sort_by(|a, b| a.total_cmp(b));
    elevs.dedup_by(|a, b| (*a - *b).abs() < 0.2);
    if elevs.is_empty() {
        elevs = vec![0.0, 3.5, 7.0, 10.5];
    } else if elevs.len() == 1 {
        elevs.push(elevs[0] + 3.6);
    }
    ([min_x, min_y], [max_x, max_y], elevs)
}

/// Evaluates coordinated Building MEP (`HVAC`, `Plumbing`, `Electrical`, `Fire Sprinkler`) sizing
/// and synthesizes 3D ENU ceiling-plenum trunks, branches, core risers & plant equipment.
pub fn evaluate_mep_systems(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    solar: Option<&SolarEnvelopeReport>,
    config: &MepSystemsConfig,
) -> MepSystemsReport {
    let mut cfg = config.clone();
    cfg.occupant_density_m2_per_person = cfg.occupant_density_m2_per_person.clamp(4.0, 35.0);
    cfg.plenum_drop_below_slab_m = cfg.plenum_drop_below_slab_m.clamp(0.25, 1.10);

    let default_site = SiteEnvironmentalCoupling::default();
    let s = site.unwrap_or(&default_site);
    let (min_xy, max_xy, elevs) = infer_bim_extents_and_levels(bim);
    let width_m = (max_xy[0] - min_xy[0]).max(6.0);
    let depth_m = (max_xy[1] - min_xy[1]).max(6.0);
    let story_count = elevs.len().saturating_sub(1).max(1);
    let floor_area_m2 = width_m * depth_m;
    let total_gfa_m2 = floor_area_m2 * story_count as f64;
    let occupants = ((total_gfa_m2 / cfg.occupant_density_m2_per_person).ceil() as usize).max(8);

    let solar_ottv_coupled_kw = if cfg.couple_solar_ottv {
        solar.map(|sr| sr.peak_solar_load_kw).unwrap_or(0.0)
    } else {
        0.0
    };
    let solar_w_m2 = (solar_ottv_coupled_kw * 1000.0) / total_gfa_m2.max(20.0);
    let temp_bonus_w_m2 = ((1.0 - (s.lat_deg.abs() / 45.0).clamp(0.0, 1.0)) * 18.0
        - (s.elevation_m.max(0.0) as f64 * 0.006).min(12.0))
    .clamp(0.0, 22.0);
    let cooling_load_w_m2 = (cfg.hvac_preset.base_w_m2() + solar_w_m2 * 0.45 + temp_bonus_w_m2)
        .clamp(75.0, 260.0);

    let avg_floor_occ = (occupants as f64 / story_count as f64).ceil() as usize;
    let wsfu_total = (occupants as f64 * 0.78).max(16.0);
    let total_building_h_m = (elevs.last().copied().unwrap_or(10.5) - elevs[0]).max(3.5);
    let pipe_run_length_m = total_building_h_m + width_m * 0.85 + depth_m * 0.5;
    let lighting_power_kva = total_gfa_m2 * 0.011;
    let receptacle_kva = total_gfa_m2 * 0.018;
    let hvac_elec_kw_est = (total_gfa_m2 * cooling_load_w_m2 * 0.001) / cfg.hvac_preset.cop();
    let electrical_connected_kva =
        (lighting_power_kva + receptacle_kva + hvac_elec_kw_est * 1.18 + 24.0).max(35.0);

    let sizing_inp = MepSizingInput {
        floor_area_m2: total_gfa_m2,
        occupancy_people: occupants.max(avg_floor_occ),
        cooling_load_w_m2,
        max_duct_velocity_ms: cfg.hvac_preset.target_velocity_ms(),
        fixture_units_wsfu: wsfu_total,
        pipe_run_length_m,
        electrical_connected_kva,
        demand_factor: 0.78,
        voltage_ll_v: 380.0,
        feeder_length_m: (total_building_h_m + width_m * 0.65).max(18.0),
    };
    let sizing = compute_mep_engineering_sizing(&sizing_inp, story_count);
    let hvac_elec_input_kw = sizing.cooling_kw / cfg.hvac_preset.cop();

    // NFPA 13 Sprinkler & Fire Pump Hydraulic Summary
    let remote_area_m2 = 139.0_f64.min(floor_area_m2.max(40.0));
    let sprinkler_flow_lpm = remote_area_m2 * cfg.fire_hazard.density_mm_min() * 1.15;
    let total_fire_flow_lpm =
        (sprinkler_flow_lpm + cfg.fire_hazard.hose_allowance_lpm()).ceil();
    let sprinkler_main_dn_mm: f64 = if total_fire_flow_lpm <= 750.0 {
        80.0
    } else if total_fire_flow_lpm <= 1500.0 {
        100.0
    } else if total_fire_flow_lpm <= 2600.0 {
        125.0
    } else {
        150.0
    };
    let fire_pump_head_kpa = (total_building_h_m * 9.81 + 310.0).ceil();
    let total_sprinkler_heads =
        ((total_gfa_m2 / cfg.fire_hazard.head_coverage_m2()).ceil() as usize).max(6);
    let water_tank_m3 = ((total_fire_flow_lpm * 60.0) / 1000.0).ceil();
    let nfpa_13_compliant = sprinkler_main_dn_mm >= 65.0 && water_tank_m3 >= 15.0;

    // Synthesize 3D ENU Vertical Core Risers + Per-Story Ceiling Plenum Trunks & Branches
    let cx = 0.5 * (min_xy[0] + max_xy[0]);
    let cy = 0.5 * (min_xy[1] + max_xy[1]);
    let z_base = elevs[0];
    let z_roof = elevs.last().copied().unwrap_or(10.5);

    let shaft_offsets: [(MepDiscipline, [f64; 2], f64, f64, String); 4] = [
        (
            MepDiscipline::Hvac,
            [cx - 1.35, cy - 0.65],
            (sizing.duct_width_mm * 0.001).clamp(0.28, 0.72),
            (sizing.duct_height_mm * 0.001).clamp(0.24, 0.55),
            format!("{:.0}×{:.0} mm • {:.0} CFM", sizing.duct_width_mm, sizing.duct_height_mm, sizing.supply_airflow_cfm),
        ),
        (
            MepDiscipline::Plumbing,
            [cx - 0.35, cy - 0.65],
            (sizing.pipe_dn_mm * 0.0014).clamp(0.12, 0.28),
            (sizing.pipe_dn_mm * 0.0014).clamp(0.12, 0.28),
            format!("DN{:.0} • {:.1} L/s", sizing.pipe_dn_mm, sizing.peak_flow_ls),
        ),
        (
            MepDiscipline::Electrical,
            [cx + 0.55, cy - 0.65],
            0.34,
            0.12,
            format!("{}mm² • {}AT", sizing.cable_size_mm2, sizing.recommended_breaker_at),
        ),
        (
            MepDiscipline::FireSprinkler,
            [cx + 1.45, cy - 0.65],
            (sprinkler_main_dn_mm * 0.0015).clamp(0.14, 0.26),
            (sprinkler_main_dn_mm * 0.0015).clamp(0.14, 0.26),
            format!("DN{sprinkler_main_dn_mm:.0} • {total_fire_flow_lpm:.0} L/m"),
        ),
    ];

    let mut segments_3d = Vec::new();
    let mut seg_id = 1_usize;

    for (disc, xy, w_m, h_m, lbl) in &shaft_offsets {
        segments_3d.push(MepConduitSegment3D {
            id: seg_id,
            tag: format!("{}-RISER-1", disc.short_label()),
            discipline: *disc,
            story_index: 0,
            is_riser: true,
            start_enu_m: [xy[0], xy[1], z_base + 0.15],
            end_enu_m: [xy[0], xy[1], z_roof + 0.6],
            width_m: *w_m,
            height_m: *h_m,
            flow_label: lbl.clone(),
        });
        seg_id += 1;
    }

    let x_left = min_xy[0] + 1.2;
    let x_right = max_xy[0] - 1.2;
    let y_south = min_xy[1] + 1.3;
    let y_north = max_xy[1] - 1.3;

    for story_idx in 0..story_count {
        let z_ceil = elevs[story_idx + 1];
        let story_h = (z_ceil - elevs[story_idx]).max(2.6);
        let drop = cfg.plenum_drop_below_slab_m.min(story_h * 0.35);

        // HVAC main trunk + twin north/south zone branches
        let z_hvac = z_ceil - drop;
        let trunk_w = (sizing.duct_width_mm * 0.00075).clamp(0.24, 0.56);
        let trunk_h = (sizing.duct_height_mm * 0.00075).clamp(0.18, 0.38);
        segments_3d.push(MepConduitSegment3D {
            id: seg_id,
            tag: format!("HVAC-L{}-TRUNK", story_idx + 1),
            discipline: MepDiscipline::Hvac,
            story_index: story_idx,
            is_riser: false,
            start_enu_m: [x_left, cy - 0.65, z_hvac],
            end_enu_m: [x_right, cy - 0.65, z_hvac],
            width_m: trunk_w,
            height_m: trunk_h,
            flow_label: format!("L{} Supply Trunk", story_idx + 1),
        });
        seg_id += 1;
        for bx in [min_xy[0] + width_m * 0.28, min_xy[0] + width_m * 0.72] {
            segments_3d.push(MepConduitSegment3D {
                id: seg_id,
                tag: format!("HVAC-L{}-BR", story_idx + 1),
                discipline: MepDiscipline::Hvac,
                story_index: story_idx,
                is_riser: false,
                start_enu_m: [bx, y_south, z_hvac],
                end_enu_m: [bx, y_north, z_hvac],
                width_m: (trunk_w * 0.62).max(0.16),
                height_m: (trunk_h * 0.72).max(0.14),
                flow_label: format!("L{} Diffuser Run", story_idx + 1),
            });
            seg_id += 1;
        }

        // Plumbing domestic water + sanitary header (staggered 0.16m lower)
        let z_plumb = z_hvac - 0.16;
        let p_dia = (sizing.pipe_dn_mm * 0.0012).clamp(0.10, 0.22);
        segments_3d.push(MepConduitSegment3D {
            id: seg_id,
            tag: format!("PLUMB-L{}-HDR", story_idx + 1),
            discipline: MepDiscipline::Plumbing,
            story_index: story_idx,
            is_riser: false,
            start_enu_m: [x_left + 0.6, cy + 0.45, z_plumb],
            end_enu_m: [x_right - 0.6, cy + 0.45, z_plumb],
            width_m: p_dia,
            height_m: p_dia,
            flow_label: format!("L{} DN{:.0} Header", story_idx + 1, sizing.pipe_dn_mm),
        });
        seg_id += 1;

        // Electrical perforated cable tray (tucked slightly higher near soffit)
        let z_elec = z_ceil - (drop * 0.62).max(0.18);
        segments_3d.push(MepConduitSegment3D {
            id: seg_id,
            tag: format!("ELEC-L{}-TRAY", story_idx + 1),
            discipline: MepDiscipline::Electrical,
            story_index: story_idx,
            is_riser: false,
            start_enu_m: [x_left + 0.4, cy + 1.25, z_elec],
            end_enu_m: [x_right - 0.4, cy + 1.25, z_elec],
            width_m: 0.30,
            height_m: 0.09,
            flow_label: format!("L{} 380V Tray", story_idx + 1),
        });
        seg_id += 1;

        // Fire Sprinkler cross-main + branch loop
        let z_fire = z_ceil - (drop * 0.78).max(0.22);
        let f_dia = (sprinkler_main_dn_mm * 0.0012).clamp(0.11, 0.22);
        segments_3d.push(MepConduitSegment3D {
            id: seg_id,
            tag: format!("FIRE-L{}-MAIN", story_idx + 1),
            discipline: MepDiscipline::FireSprinkler,
            story_index: story_idx,
            is_riser: false,
            start_enu_m: [x_left, cy - 1.45, z_fire],
            end_enu_m: [x_right, cy - 1.45, z_fire],
            width_m: f_dia,
            height_m: f_dia,
            flow_label: format!("L{} NFPA13 Main", story_idx + 1),
        });
        seg_id += 1;
    }

    let equipment_3d = vec![
        MepEquipmentNode3D {
            tag: format!("{} Plant", cfg.hvac_preset.short_label()),
            discipline: MepDiscipline::Hvac,
            center_enu_m: [cx - 2.2, cy, z_roof + 0.85],
            size_enu_m: [2.8, 1.9, 1.5],
            rating_label: format!("{:.1} TR • {:.0} kW", sizing.cooling_tr, sizing.cooling_kw),
        },
        MepEquipmentNode3D {
            tag: "MDP-380V".to_string(),
            discipline: MepDiscipline::Electrical,
            center_enu_m: [cx + 2.2, cy - 1.2, z_base + 1.05],
            size_enu_m: [1.8, 0.9, 1.9],
            rating_label: format!("{} AT • {:.1} kVA", sizing.recommended_breaker_at, sizing.demand_kva),
        },
        MepEquipmentNode3D {
            tag: "Booster Pump".to_string(),
            discipline: MepDiscipline::Plumbing,
            center_enu_m: [cx - 1.8, cy + 1.8, z_base + 0.65],
            size_enu_m: [1.5, 1.1, 1.1],
            rating_label: format!("DN{:.0} • {:.1} kPa", sizing.pipe_dn_mm, sizing.head_loss_kpa),
        },
        MepEquipmentNode3D {
            tag: "Fire Pump + Tank".to_string(),
            discipline: MepDiscipline::FireSprinkler,
            center_enu_m: [cx + 2.1, cy + 1.8, z_base + 0.95],
            size_enu_m: [2.4, 1.6, 1.8],
            rating_label: format!("{total_fire_flow_lpm:.0} L/m • {water_tank_m3:.0} m³"),
        },
    ];

    let estimated_mep_cost_usd =
        total_gfa_m2 * 165.0 + sizing.cooling_tr * 1350.0 + sizing.demand_kva * 420.0;
    let all_codes_compliant = sizing.voltage_drop_ok && nfpa_13_compliant && sizing.duct_velocity_ms <= 11.5;

    let summary_verdict = format!(
        "MEP ({}): HVAC {:.1} TR ({:.0}×{:.0}mm) • Plumb DN{:.0} • Elec {}AT (ΔV {:.2}%) • Fire {:.0} L/m ({} heads) [{}]",
        cfg.hvac_preset.short_label(),
        sizing.cooling_tr,
        sizing.duct_width_mm,
        sizing.duct_height_mm,
        sizing.pipe_dn_mm,
        sizing.recommended_breaker_at,
        sizing.voltage_drop_pct,
        total_fire_flow_lpm,
        total_sprinkler_heads,
        if all_codes_compliant { "COMPLIANT" } else { "REVIEW" }
    );

    MepSystemsReport {
        config: cfg,
        site_label: s.site_label.clone(),
        stories: story_count,
        building_width_m: width_m,
        building_depth_m: depth_m,
        total_gfa_m2,
        occupants,
        solar_ottv_coupled_kw,
        cooling_kw: sizing.cooling_kw,
        cooling_tr: sizing.cooling_tr,
        hvac_elec_input_kw,
        supply_airflow_cfm: sizing.supply_airflow_cfm,
        duct_width_mm: sizing.duct_width_mm,
        duct_height_mm: sizing.duct_height_mm,
        duct_velocity_ms: sizing.duct_velocity_ms,
        duct_friction_pa_m: sizing.duct_friction_pa_m,
        plumbing_peak_flow_ls: sizing.peak_flow_ls,
        plumbing_pipe_dn_mm: sizing.pipe_dn_mm,
        plumbing_velocity_ms: sizing.pipe_velocity_ms,
        plumbing_head_loss_kpa: sizing.head_loss_kpa,
        drain_slope_pct: sizing.drain_slope_pct,
        elec_demand_kva: sizing.demand_kva,
        elec_design_current_a: sizing.design_current_a,
        elec_breaker_at: sizing.recommended_breaker_at,
        elec_cable_mm2: sizing.cable_size_mm2,
        elec_voltage_drop_pct: sizing.voltage_drop_pct,
        elec_voltage_drop_ok: sizing.voltage_drop_ok,
        fire: FireSprinklerSummary {
            hazard_class: config.fire_hazard,
            design_density_mm_min: config.fire_hazard.density_mm_min(),
            total_sprinkler_heads,
            sprinkler_main_dn_mm,
            fire_pump_flow_lpm: total_fire_flow_lpm,
            fire_pump_head_kpa,
            water_tank_m3,
            nfpa_13_compliant,
        },
        segments_3d,
        equipment_3d,
        estimated_mep_cost_usd,
        all_codes_compliant,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};
    use crate::core::solar_envelope::{evaluate_solar_envelope, SolarEnvelopeConfig};

    #[test]
    fn test_v0875_mep_systems_hvac_plumbing_electrical_fire_and_ifcx_overlay() {
        let bim = BimProject::sample_bimcraft_building();
        let site = SiteEnvironmentalCoupling::default();
        let solar_rep =
            evaluate_solar_envelope(&bim, Some(&site), None, &SolarEnvelopeConfig::default());

        let mut cfg = MepSystemsConfig::default();
        cfg.couple_solar_ottv = false;
        let rep_base = evaluate_mep_systems(&bim, Some(&site), Some(&solar_rep), &cfg);

        cfg.couple_solar_ottv = true;
        let rep_coupled = evaluate_mep_systems(&bim, Some(&site), Some(&solar_rep), &cfg);

        assert!(rep_coupled.cooling_kw >= rep_base.cooling_kw);
        assert!(rep_coupled.cooling_tr > 5.0);
        assert!(rep_coupled.duct_width_mm >= 200.0 && rep_coupled.duct_height_mm >= 150.0);
        assert!(rep_coupled.plumbing_pipe_dn_mm >= 25.0);
        assert!(rep_coupled.elec_breaker_at >= 32 && rep_coupled.elec_cable_mm2 >= 10);
        assert!(rep_coupled.fire.total_sprinkler_heads >= 6);
        assert!(rep_coupled.fire.fire_pump_flow_lpm >= 379.0);
        assert_eq!(rep_coupled.equipment_3d.len(), 4);

        for disc in MepDiscipline::ALL {
            assert!(
                rep_coupled.segments_3d.iter().any(|s| s.discipline == disc && s.is_riser),
                "Missing vertical core riser for {disc:?}"
            );
            assert!(
                rep_coupled.segments_3d.iter().any(|s| s.discipline == disc && !s.is_riser),
                "Missing horizontal floor distribution for {disc:?}"
            );
        }

        let base_doc = IfcxDocument::new("mep_test");
        let ov_doc =
            IfcxStage::compose_with_mep_systems_overlay(&base_doc, &rep_coupled.to_ifcx_overlay());
        let json = ov_doc.to_json_pretty();
        assert!(json.contains(NS_EG_MEP_SYSTEMS));
        assert!(json.contains("hvacCoolingTr"));
    }
}
