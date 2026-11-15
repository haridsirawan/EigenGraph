//! core/lifecycle_ops.rs — Phase 45: Earth-to-Site Environmental Coupling,
//! MEP Engineering Sizing, Commercial Procurement/VO Claims, Site Daily Diary & Fleet,
//! Field QA/QC (Concrete Lab, ITP, NCR, Punch List, MEP T&C), and Geometry<->Analysis Bridge.

use crate::core::gis::{estimate_baseline_elevation_m, PlanetaryWorldState};
use serde::{Deserialize, Serialize};

/// Geotechnical, seismic, wind, flood, and logistics telemetry coupled directly
/// to the active EigenEarth / Google Earth site coordinate and terrain modifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteEnvironmentalCoupling {
    pub site_label: String,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub elevation_m: f32,
    pub seismic_pga_ss_g: f32,
    pub seismic_pga_s1_g: f32,
    pub design_wind_ms: f32,
    pub flood_risk_index: f32,
    pub climate_rain_delay_pct: f32,
    pub remote_logistics_multiplier: f32,
    pub recommended_soil_class: String,
    pub foundation_advisory: String,
}

impl Default for SiteEnvironmentalCoupling {
    fn default() -> Self {
        compute_site_environmental_coupling(-6.1754, 106.8272, &PlanetaryWorldState::default())
    }
}

/// Deterministically derives structural hazard & site construction telemetry from
/// the active Earth coordinate `(lat_deg, lon_deg)` and live planetary terraforming state.
pub fn compute_site_environmental_coupling(
    lat_deg: f64,
    lon_deg: f64,
    world: &PlanetaryWorldState,
) -> SiteEnvironmentalCoupling {
    let elev_m = world.effective_elevation_m(lat_deg, lon_deg);
    let hazard_shield: f32 = ((world.vitals.seawall_count as f32 * 0.06)
        + (world.vitals.forest_cells as f32 * 0.002))
        .clamp(0.0_f32, 0.75_f32);

    // Tectonic proximity proxy: Ring of Fire / Alpine-Himalayan / Cascadia-Andes belts
    let in_indonesian_arc = (-11.0..=6.5).contains(&lat_deg) && (95.0..=142.0).contains(&lon_deg);
    let in_japan_pacific_rim =
        (24.0..=48.0).contains(&lat_deg) && (122.0..=150.0).contains(&lon_deg);
    let in_us_west_andes = (-45.0..=50.0).contains(&lat_deg) && (-130.0..=-65.0).contains(&lon_deg);
    let in_mediterranean_himalaya =
        (25.0..=43.0).contains(&lat_deg) && (10.0..=102.0).contains(&lon_deg);

    let tectonic_base_ss = if in_indonesian_arc || in_japan_pacific_rim {
        1.05_f32
    } else if in_us_west_andes || in_mediterranean_himalaya {
        0.92_f32
    } else {
        0.38_f32
    };
    let local_basin_mod = (((lat_deg * 0.7 + lon_deg * 0.4).sin().abs()) as f32) * 0.32;
    let seismic_pga_ss_g = (tectonic_base_ss + local_basin_mod).clamp(0.25, 1.55);
    let seismic_pga_s1_g = (seismic_pga_ss_g * 0.42).clamp(0.10, 0.72);

    // Wind exposure: coastal low-elevation + mid-latitude storm tracks + high altitude ridge speedup
    let abs_lat = lat_deg.abs() as f32;
    let tropical_cyclone_belt = if (8.0..=30.0).contains(&abs_lat) { 6.5 } else { 0.0 };
    let altitude_windup = (elev_m.max(0.0) / 500.0).min(8.0);
    let coastal_exposure = if elev_m < 25.0 { 4.5 } else { 0.0 };
    let design_wind_ms = (32.0 + tropical_cyclone_belt + altitude_windup + coastal_exposure).clamp(28.0, 56.0);

    // Coastal / fluvial flood risk reduced dynamically by Earth seawalls & dikes
    let base_elev = estimate_baseline_elevation_m(lat_deg, lon_deg);
    let terrain_freeboard = (elev_m - world.vitals.sea_level_m).max(-10.0);
    let raw_flood = if terrain_freeboard <= 1.0 {
        0.90
    } else if terrain_freeboard <= 6.0 {
        0.65 - (terrain_freeboard - 1.0) * 0.06
    } else if terrain_freeboard <= 25.0 {
        0.32 - (terrain_freeboard - 6.0) * 0.012
    } else {
        0.08
    };
    let flood_risk_index = (raw_flood * (1.0 - hazard_shield * 0.80)).clamp(0.03, 0.96);

    // Equatorial / monsoon climate rain delay allowance for 4D scheduling & Daily Diary
    let equatorial_factor = ((15.0 - (abs_lat - 5.0).abs()).max(0.0) / 15.0) * 14.0;
    let climate_rain_delay_pct = (5.0 + equatorial_factor + flood_risk_index * 6.0).clamp(4.0, 26.0);

    // Logistics cost factor for mountainous/offshore/remote Earth coordinates
    let alt_logistics: f32 = if elev_m > 1200.0 {
        0.18
    } else if elev_m > 400.0 {
        0.08
    } else {
        0.0
    };
    let coastal_barge_factor: f32 = if base_elev <= 1.5 { 0.14 } else { 0.0 };
    let remote_logistics_multiplier = (1.0_f32 + alt_logistics + coastal_barge_factor).clamp(1.0_f32, 1.38_f32);

    let (recommended_soil_class, foundation_advisory) = if base_elev <= 2.0 || terrain_freeboard <= 4.0 {
        (
            "SE (Soft Delta / Liquefiable)".to_string(),
            "High groundwater/flood exposure: specify bored piles >= 24m + waterproof cutoff wall.".to_string(),
        )
    } else if elev_m >= 600.0 {
        (
            "SC (Very Dense Soil / Soft Rock)".to_string(),
            "Upland ridge bearing stratum: spread footings or socketed short piles viable; monitor slope stability.".to_string(),
        )
    } else {
        (
            "SD (Stiff Clay / Dense Sand)".to_string(),
            "Standard alluvial urban profile: pile cap grid or raft mat foundation with perimeter drainage.".to_string(),
        )
    };

    SiteEnvironmentalCoupling {
        site_label: format!("{:.3}°, {:.3}°", lat_deg, lon_deg),
        lat_deg,
        lon_deg,
        elevation_m: elev_m,
        seismic_pga_ss_g,
        seismic_pga_s1_g,
        design_wind_ms,
        flood_risk_index,
        climate_rain_delay_pct,
        remote_logistics_multiplier,
        recommended_soil_class,
        foundation_advisory,
    }
}

/// Input specification for integrated MEP Engineering Sizing inside Drafting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MepSizingInput {
    pub floor_area_m2: f64,
    pub occupancy_people: usize,
    pub cooling_load_w_m2: f64,
    pub max_duct_velocity_ms: f64,
    pub fixture_units_wsfu: f64,
    pub pipe_run_length_m: f64,
    pub electrical_connected_kva: f64,
    pub demand_factor: f64,
    pub voltage_ll_v: f64,
    pub feeder_length_m: f64,
}

impl Default for MepSizingInput {
    fn default() -> Self {
        Self {
            floor_area_m2: 1200.0,
            occupancy_people: 120,
            cooling_load_w_m2: 135.0,
            max_duct_velocity_ms: 7.5,
            fixture_units_wsfu: 85.0,
            pipe_run_length_m: 45.0,
            electrical_connected_kva: 250.0,
            demand_factor: 0.78,
            voltage_ll_v: 380.0,
            feeder_length_m: 42.0,
        }
    }
}

/// Computed HVAC Duct, Plumbing DN Pipe, and 3-Phase Electrical Feeder sizing results.
#[derive(Debug, Clone, PartialEq)]
pub struct MepSizingResult {
    // HVAC
    pub cooling_kw: f64,
    pub cooling_tr: f64,
    pub supply_airflow_cfm: f64,
    pub supply_airflow_m3s: f64,
    pub duct_width_mm: f64,
    pub duct_height_mm: f64,
    pub duct_equiv_dia_mm: f64,
    pub duct_velocity_ms: f64,
    pub duct_friction_pa_m: f64,
    // Plumbing
    pub peak_flow_ls: f64,
    pub pipe_dn_mm: f64,
    pub pipe_velocity_ms: f64,
    pub head_loss_kpa: f64,
    pub drain_slope_pct: f64,
    // Electrical
    pub demand_kva: f64,
    pub design_current_a: f64,
    pub recommended_breaker_at: usize,
    pub cable_size_mm2: usize,
    pub voltage_drop_pct: f64,
    pub voltage_drop_ok: bool,
}

/// Evaluates ASHRAE HVAC duct, IPC/Hunter plumbing pipe, and IEC 3-phase feeder sizing.
pub fn compute_mep_engineering_sizing(
    inp: &MepSizingInput,
    storey_count: usize,
) -> MepSizingResult {
    let area = inp.floor_area_m2.max(20.0);
    let storeys = storey_count.max(1) as f64;
    let occ = inp.occupancy_people.max(1) as f64;

    // 1. HVAC Cooling & Supply Duct (sensible + latent + fresh air load)
    let occ_heat_kw = occ * 0.13;
    let cooling_kw = (area * inp.cooling_load_w_m2.max(40.0)) / 1000.0 + occ_heat_kw;
    let cooling_tr = cooling_kw / 3.517;
    // Standard delta-T ~ 10 K -> ~400 CFM per TR + ventilation minimum 7.5 L/s/person
    let supply_cfm_thermal = cooling_tr * 400.0;
    let supply_cfm_vent = occ * 16.0;
    let supply_airflow_cfm = supply_cfm_thermal.max(supply_cfm_vent);
    let supply_airflow_m3s = supply_airflow_cfm * 0.000471947;

    let target_v = inp.max_duct_velocity_ms.clamp(2.5, 14.0);
    let req_area_m2 = supply_airflow_m3s / target_v;
    let equiv_dia_m = (4.0 * req_area_m2 / std::f64::consts::PI).sqrt();
    let duct_equiv_dia_mm = (equiv_dia_m * 1000.0).ceil();

    // Aspect ratio 1.6 : 1 rounded to nearest 50 mm commercial GI size
    let raw_w_mm = ((req_area_m2 * 1_000_000.0 * 1.6).sqrt() / 50.0).ceil() * 50.0;
    let duct_width_mm = raw_w_mm.clamp(200.0, 2400.0);
    let raw_h_mm = ((req_area_m2 * 1_000_000.0 / duct_width_mm) / 50.0).ceil() * 50.0;
    let duct_height_mm = raw_h_mm.clamp(150.0, 1600.0);
    let actual_duct_area_m2 = (duct_width_mm / 1000.0) * (duct_height_mm / 1000.0);
    let duct_velocity_ms = supply_airflow_m3s / actual_duct_area_m2.max(0.01);
    let dh_m = 2.0 * (duct_width_mm * duct_height_mm) / (1000.0 * (duct_width_mm + duct_height_mm));
    // Darcy approximation for galvanized sheet metal f ~ 0.020
    let duct_friction_pa_m = 0.020 * (1.2 / 2.0) * (duct_velocity_ms * duct_velocity_ms) / dh_m.max(0.05);

    // 2. Plumbing Hunter curve demand + standard DN pipe velocity check
    let wsfu = inp.fixture_units_wsfu.max(4.0);
    let peak_flow_ls = 0.25 * wsfu.powf(0.62); // smooth Hunter-equivalent L/s demand
    let flow_m3s = peak_flow_ls / 1000.0;
    let standard_dns = [25.0_f64, 32.0, 40.0, 50.0, 65.0, 80.0, 100.0, 125.0, 150.0, 200.0];
    let mut pipe_dn_mm = 200.0;
    for dn in standard_dns {
        let d_m = dn / 1000.0;
        let area_p = std::f64::consts::PI * d_m * d_m * 0.25;
        let v = flow_m3s / area_p;
        if v <= 2.2 {
            pipe_dn_mm = dn;
            break;
        }
    }
    let pipe_area = std::f64::consts::PI * (pipe_dn_mm / 1000.0).powi(2) * 0.25;
    let pipe_velocity_ms = flow_m3s / pipe_area.max(1e-5);
    let static_head_m = storeys * 3.6;
    let friction_head_m =
        0.022 * (inp.pipe_run_length_m.max(5.0) / (pipe_dn_mm / 1000.0))
            * (pipe_velocity_ms * pipe_velocity_ms / (2.0 * 9.81));
    let head_loss_kpa = (static_head_m + friction_head_m) * 9.81;
    let drain_slope_pct = if pipe_dn_mm <= 65.0 { 2.0 } else if pipe_dn_mm <= 100.0 { 1.5 } else { 1.0 };

    // 3. Electrical 3-Phase Feeder & Breaker Selection
    let df = inp.demand_factor.clamp(0.35, 1.0);
    let v_ll = inp.voltage_ll_v.clamp(208.0, 690.0);
    let demand_kva = inp.electrical_connected_kva.max(5.0) * df;
    let design_current_a = (demand_kva * 1000.0) / (3.0_f64.sqrt() * v_ll);
    let mccb_ratings = [32_usize, 50, 63, 80, 100, 125, 160, 200, 250, 320, 400, 500, 630, 800, 1000, 1250, 1600];
    let req_at = (design_current_a * 1.15).ceil() as usize;
    let recommended_breaker_at = mccb_ratings
        .iter()
        .copied()
        .find(|&at| at >= req_at)
        .unwrap_or(1600);

    // Copper XLPE/PVC ampacity table approximation (mm2 -> nominal A, R Ohm/km)
    let cables: [(usize, f64, f64); 11] = [
        (10, 55.0, 1.83),
        (16, 75.0, 1.15),
        (25, 100.0, 0.727),
        (35, 125.0, 0.524),
        (50, 150.0, 0.387),
        (70, 192.0, 0.268),
        (95, 232.0, 0.193),
        (120, 269.0, 0.153),
        (150, 309.0, 0.124),
        (185, 353.0, 0.0991),
        (240, 415.0, 0.0754),
    ];
    let mut cable_size_mm2 = 240_usize;
    let mut resistance_ohm_km = 0.0754_f64;
    for (sz, amp, r) in cables {
        if amp >= recommended_breaker_at as f64 * 0.90 {
            cable_size_mm2 = sz;
            resistance_ohm_km = r;
            break;
        }
    }
    let len_km = inp.feeder_length_m.max(1.0) / 1000.0;
    let dv_volts = 3.0_f64.sqrt() * design_current_a * len_km * (resistance_ohm_km * 0.85 + 0.08 * 0.52);
    let voltage_drop_pct = (dv_volts / v_ll) * 100.0;

    MepSizingResult {
        cooling_kw,
        cooling_tr,
        supply_airflow_cfm,
        supply_airflow_m3s,
        duct_width_mm,
        duct_height_mm,
        duct_equiv_dia_mm,
        duct_velocity_ms,
        duct_friction_pa_m,
        peak_flow_ls,
        pipe_dn_mm,
        pipe_velocity_ms,
        head_loss_kpa,
        drain_slope_pct,
        demand_kva,
        design_current_a,
        recommended_breaker_at,
        cable_size_mm2,
        voltage_drop_pct,
        voltage_drop_ok: voltage_drop_pct <= 3.0,
    }
}

/// Procurement Purchase Order / Subcontract status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcurementStatus {
    TenderEval,
    AwardedPo,
    InProgressIpc,
    CompletedFinal,
}

impl ProcurementStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::TenderEval => "Tender Eval",
            Self::AwardedPo => "PO Awarded",
            Self::InProgressIpc => "IPC Active",
            Self::CompletedFinal => "Final Account",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::TenderEval => Self::AwardedPo,
            Self::AwardedPo => Self::InProgressIpc,
            Self::InProgressIpc => Self::CompletedFinal,
            Self::CompletedFinal => Self::TenderEval,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PurchaseOrderRow {
    pub po_no: String,
    pub vendor: String,
    pub wbs_package: String,
    pub committed_idr: f64,
    pub certified_pct: f32,
    pub lead_time_days: u32,
    pub status: ProcurementStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariationOrderRow {
    pub vo_no: String,
    pub description: String,
    pub cost_impact_idr: f64,
    pub eot_days: i32,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcurementContractLedger {
    pub purchase_orders: Vec<PurchaseOrderRow>,
    pub variation_orders: Vec<VariationOrderRow>,
    pub retention_pct: f32,
    pub advance_payment_pct: f32,
}

impl Default for ProcurementContractLedger {
    fn default() -> Self {
        Self {
            purchase_orders: vec![
                PurchaseOrderRow {
                    po_no: "PO-2026-01".to_string(),
                    vendor: "PT Krakatau Baja Konstruksi".to_string(),
                    wbs_package: "05 Structural Steel WF + Bolts".to_string(),
                    committed_idr: 4_850_000_000.0,
                    certified_pct: 65.0,
                    lead_time_days: 28,
                    status: ProcurementStatus::InProgressIpc,
                },
                PurchaseOrderRow {
                    po_no: "PO-2026-02".to_string(),
                    vendor: "PT Pionir Beton Ready-Mix".to_string(),
                    wbs_package: "03 Structural Concrete K-400 + Pump".to_string(),
                    committed_idr: 3_620_000_000.0,
                    certified_pct: 48.0,
                    lead_time_days: 7,
                    status: ProcurementStatus::InProgressIpc,
                },
                PurchaseOrderRow {
                    po_no: "PO-2026-03".to_string(),
                    vendor: "PT Karya Pondasi Nusantara".to_string(),
                    wbs_package: "02 Bored Pile 800mm & Pile Cap".to_string(),
                    committed_idr: 2_980_000_000.0,
                    certified_pct: 100.0,
                    lead_time_days: 14,
                    status: ProcurementStatus::CompletedFinal,
                },
                PurchaseOrderRow {
                    po_no: "PO-2026-04".to_string(),
                    vendor: "PT Graha MEP Systems".to_string(),
                    wbs_package: "21-26 HVAC Chiller, Pump & Switchgear".to_string(),
                    committed_idr: 2_450_000_000.0,
                    certified_pct: 25.0,
                    lead_time_days: 42,
                    status: ProcurementStatus::AwardedPo,
                },
            ],
            variation_orders: vec![
                VariationOrderRow {
                    vo_no: "VO-001".to_string(),
                    description: "Deepened Bored Pile Toe (+4.0m) in SE Alluvial Lens".to_string(),
                    cost_impact_idr: 345_000_000.0,
                    eot_days: 6,
                    approved: true,
                },
                VariationOrderRow {
                    vo_no: "VO-002".to_string(),
                    description: "Upgraded 135-deg Seismic Confinement Ties at L02-L04".to_string(),
                    cost_impact_idr: 162_000_000.0,
                    eot_days: 3,
                    approved: false,
                },
            ],
            retention_pct: 5.0,
            advance_payment_pct: 15.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommercialPositionSummary {
    pub baseline_budget_idr: f64,
    pub total_committed_po_idr: f64,
    pub certified_ipc_idr: f64,
    pub net_payable_after_retention_idr: f64,
    pub approved_vo_idr: f64,
    pub pending_vo_idr: f64,
    pub revised_forecast_idr: f64,
    pub total_eot_days: i32,
}

impl ProcurementContractLedger {
    pub fn evaluate_position(&self, baseline_rab_idr: f64) -> CommercialPositionSummary {
        let total_committed_po_idr: f64 = self.purchase_orders.iter().map(|p| p.committed_idr).sum();
        let certified_ipc_idr: f64 = self
            .purchase_orders
            .iter()
            .map(|p| p.committed_idr * (p.certified_pct.clamp(0.0, 100.0) as f64 / 100.0))
            .sum();
        let retention_frac = (self.retention_pct.clamp(0.0, 20.0) as f64) / 100.0;
        let net_payable_after_retention_idr = certified_ipc_idr * (1.0 - retention_frac);
        let approved_vo_idr: f64 = self
            .variation_orders
            .iter()
            .filter(|v| v.approved)
            .map(|v| v.cost_impact_idr)
            .sum();
        let pending_vo_idr: f64 = self
            .variation_orders
            .iter()
            .filter(|v| !v.approved)
            .map(|v| v.cost_impact_idr)
            .sum();
        let total_eot_days: i32 = self
            .variation_orders
            .iter()
            .filter(|v| v.approved)
            .map(|v| v.eot_days)
            .sum();
        let revised_forecast_idr = baseline_rab_idr.max(total_committed_po_idr) + approved_vo_idr;
        CommercialPositionSummary {
            baseline_budget_idr: baseline_rab_idr,
            total_committed_po_idr,
            certified_ipc_idr,
            net_payable_after_retention_idr,
            approved_vo_idr,
            pending_vo_idr,
            revised_forecast_idr,
            total_eot_days,
        }
    }
}

/// Site Daily Diary entry tracking weather, crew manpower, heavy plant utilization, and pour volumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyDiaryEntry {
    pub day_label: String,
    pub weather: String,
    pub temp_c: f32,
    pub workable_hours: f32,
    pub carpentry_crew: u32,
    pub rebar_crew: u32,
    pub concrete_crew: u32,
    pub mep_crew: u32,
    pub crane_util_pct: f32,
    pub pump_m3_poured: f32,
    pub incident_free: bool,
    pub remarks: String,
}

impl DailyDiaryEntry {
    pub fn total_headcount(&self) -> u32 {
        self.carpentry_crew + self.rebar_crew + self.concrete_crew + self.mep_crew
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteDailyDiaryLedger {
    pub entries: Vec<DailyDiaryEntry>,
}

impl Default for SiteDailyDiaryLedger {
    fn default() -> Self {
        Self {
            entries: vec![
                DailyDiaryEntry {
                    day_label: "Day 42 (Mon)".to_string(),
                    weather: "Clear".to_string(),
                    temp_c: 31.2,
                    workable_hours: 8.5,
                    carpentry_crew: 18,
                    rebar_crew: 16,
                    concrete_crew: 14,
                    mep_crew: 10,
                    crane_util_pct: 82.0,
                    pump_m3_poured: 96.0,
                    incident_free: true,
                    remarks: "L03 Zone-A beam-slab rebar fix & concrete pour completed.".to_string(),
                },
                DailyDiaryEntry {
                    day_label: "Day 43 (Tue)".to_string(),
                    weather: "Afternoon Squall".to_string(),
                    temp_c: 28.4,
                    workable_hours: 6.0,
                    carpentry_crew: 18,
                    rebar_crew: 14,
                    concrete_crew: 8,
                    mep_crew: 12,
                    crane_util_pct: 58.0,
                    pump_m3_poured: 0.0,
                    incident_free: true,
                    remarks: "Tower crane stood down 2.5 hrs due to 19 m/s gust alarm.".to_string(),
                },
                DailyDiaryEntry {
                    day_label: "Day 44 (Wed)".to_string(),
                    weather: "Partly Cloudy".to_string(),
                    temp_c: 30.6,
                    workable_hours: 9.0,
                    carpentry_crew: 20,
                    rebar_crew: 18,
                    concrete_crew: 16,
                    mep_crew: 12,
                    crane_util_pct: 88.0,
                    pump_m3_poured: 112.0,
                    incident_free: true,
                    remarks: "Core shear wall jump-form lift & embedded MEP sleeves signed off.".to_string(),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DailyDiarySummary {
    pub avg_headcount: f32,
    pub avg_crane_util_pct: f32,
    pub total_concrete_m3: f32,
    pub total_weather_lost_hrs: f32,
    pub zero_incident_days: usize,
}

impl SiteDailyDiaryLedger {
    pub fn summarize(&self) -> DailyDiarySummary {
        let n = self.entries.len().max(1) as f32;
        let head_sum: u32 = self.entries.iter().map(|e| e.total_headcount()).sum();
        let crane_sum: f32 = self.entries.iter().map(|e| e.crane_util_pct).sum();
        let total_concrete_m3: f32 = self.entries.iter().map(|e| e.pump_m3_poured).sum();
        let total_weather_lost_hrs: f32 = self
            .entries
            .iter()
            .map(|e| (8.5 - e.workable_hours).max(0.0))
            .sum();
        let zero_incident_days = self.entries.iter().filter(|e| e.incident_free).count();
        DailyDiarySummary {
            avg_headcount: head_sum as f32 / n,
            avg_crane_util_pct: crane_sum / n,
            total_concrete_m3,
            total_weather_lost_hrs,
            zero_incident_days,
        }
    }
}

/// Concrete cylinder / cube lab test record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConcreteCylinderBatch {
    pub batch_id: String,
    pub pour_zone: String,
    pub target_fc_mpa: f32,
    pub slump_mm: f32,
    pub day7_mpa: f32,
    pub day28_mpa: f32,
}

impl ConcreteCylinderBatch {
    pub fn passes_fc(&self) -> bool {
        self.day28_mpa >= self.target_fc_mpa
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItpInspectionItem {
    pub code: String,
    pub activity: String,
    pub hold_point: bool,
    pub signed_off: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NcrPunchItem {
    pub ncr_id: String,
    pub discipline: String,
    pub location: String,
    pub description: String,
    pub critical: bool,
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MepTestingItem {
    pub system: String,
    pub protocol: String,
    pub target_spec: String,
    pub measured_val: String,
    pub witnessed_pass: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldQualityControlLedger {
    pub batches: Vec<ConcreteCylinderBatch>,
    pub itps: Vec<ItpInspectionItem>,
    pub ncrs: Vec<NcrPunchItem>,
    pub mep_tests: Vec<MepTestingItem>,
}

impl Default for FieldQualityControlLedger {
    fn default() -> Self {
        Self {
            batches: vec![
                ConcreteCylinderBatch {
                    batch_id: "CY-2601".to_string(),
                    pour_zone: "Pile Cap PC-04 (Footing)".to_string(),
                    target_fc_mpa: 35.0,
                    slump_mm: 120.0,
                    day7_mpa: 26.8,
                    day28_mpa: 38.4,
                },
                ConcreteCylinderBatch {
                    batch_id: "CY-2602".to_string(),
                    pour_zone: "L02 Columns Grid B1-B3".to_string(),
                    target_fc_mpa: 40.0,
                    slump_mm: 135.0,
                    day7_mpa: 30.1,
                    day28_mpa: 43.2,
                },
                ConcreteCylinderBatch {
                    batch_id: "CY-2603".to_string(),
                    pour_zone: "L03 Transfer Girder B-12".to_string(),
                    target_fc_mpa: 40.0,
                    slump_mm: 115.0,
                    day7_mpa: 27.9,
                    day28_mpa: 39.1,
                },
            ],
            itps: vec![
                ItpInspectionItem {
                    code: "ITP-STR-01".to_string(),
                    activity: "Bored Pile Integrity (PIT & Sonic Logging)".to_string(),
                    hold_point: true,
                    signed_off: true,
                },
                ItpInspectionItem {
                    code: "ITP-STR-04".to_string(),
                    activity: "Rebar Lap Splice, Cover Blocks & 135° Hooks Before Pour".to_string(),
                    hold_point: true,
                    signed_off: true,
                },
                ItpInspectionItem {
                    code: "ITP-STR-09".to_string(),
                    activity: "UT Weld Scan on Moment Connection Flange Butt Joint".to_string(),
                    hold_point: true,
                    signed_off: false,
                },
            ],
            ncrs: vec![
                NcrPunchItem {
                    ncr_id: "NCR-014".to_string(),
                    discipline: "STR".to_string(),
                    location: "Grid C-2 / L02 Column".to_string(),
                    description: "Minor honeycomb 45x80mm at cold joint; epoxy-grout repair required.".to_string(),
                    critical: false,
                    closed: false,
                },
                NcrPunchItem {
                    ncr_id: "PL-089".to_string(),
                    discipline: "MEP".to_string(),
                    location: "Core Riser CHW-R01".to_string(),
                    description: "2-Hr UL intumescent firestop collar missing around DN150 sleeve.".to_string(),
                    critical: true,
                    closed: false,
                },
                NcrPunchItem {
                    ncr_id: "NCR-011".to_string(),
                    discipline: "ARC".to_string(),
                    location: "Lobby Curtain Wall East".to_string(),
                    description: "Anchor mullion torque verified & structural silicone cured.".to_string(),
                    critical: false,
                    closed: true,
                },
            ],
            mep_tests: vec![
                MepTestingItem {
                    system: "CHW-R01 Chilled Water".to_string(),
                    protocol: "Hydrostatic Pressure Test (1.5x Working)".to_string(),
                    target_spec: "12.0 Bar / 24h (0% drop)".to_string(),
                    measured_val: "12.2 Bar / 0.0% drop".to_string(),
                    witnessed_pass: true,
                },
                MepTestingItem {
                    system: "ELV-B01 Main Riser".to_string(),
                    protocol: "Insulation Resistance (Megger 1000V)".to_string(),
                    target_spec: ">= 100 MOhm".to_string(),
                    measured_val: "420 MOhm".to_string(),
                    witnessed_pass: true,
                },
                MepTestingItem {
                    system: "HVC-D01 Stairwell Pressurization".to_string(),
                    protocol: "TAB Airflow & dP Test (NFPA 92)".to_string(),
                    target_spec: "50 Pa / 8200 CFM".to_string(),
                    measured_val: "Pending Fan VFD Tune".to_string(),
                    witnessed_pass: false,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualityScorecardSummary {
    pub fc_pass_rate_pct: f32,
    pub itp_cleared_count: usize,
    pub itp_total_count: usize,
    pub open_ncr_count: usize,
    pub mep_tc_pass_pct: f32,
}

impl FieldQualityControlLedger {
    pub fn evaluate_scorecard(&self) -> QualityScorecardSummary {
        let b_total = self.batches.len().max(1);
        let b_pass = self.batches.iter().filter(|b| b.passes_fc()).count();
        let itp_cleared_count = self.itps.iter().filter(|i| i.signed_off).count();
        let open_ncr_count = self.ncrs.iter().filter(|n| !n.closed).count();
        let m_total = self.mep_tests.len().max(1);
        let m_pass = self.mep_tests.iter().filter(|m| m.witnessed_pass).count();
        QualityScorecardSummary {
            fc_pass_rate_pct: (b_pass as f32 / b_total as f32) * 100.0,
            itp_cleared_count,
            itp_total_count: self.itps.len(),
            open_ncr_count,
            mep_tc_pass_pct: (m_pass as f32 / m_total as f32) * 100.0,
        }
    }
}

/// Consolidated Phase 45 lifecycle operations archive persisted cleanly in `.eigen.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LifecycleOpsState {
    #[serde(default)]
    pub mep_sizing: MepSizingInput,
    #[serde(default)]
    pub procurement: ProcurementContractLedger,
    #[serde(default)]
    pub daily_diary: SiteDailyDiaryLedger,
    #[serde(default)]
    pub qa_qc: FieldQualityControlLedger,
    #[serde(default)]
    pub site_coupling: SiteEnvironmentalCoupling,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::gis::EarthBrushAction;

    #[test]
    fn test_earth_to_site_coupling_responds_to_seawalls_and_coordinates() {
        let mut world = PlanetaryWorldState::default();
        // Low coastal delta in Jakarta (-6.12, 106.85) vs inland high elevation plateau (-6.9, 107.6)
        let coastal = compute_site_environmental_coupling(-6.12, 106.85, &world);
        assert!(coastal.seismic_pga_ss_g >= 0.90);
        assert!(coastal.climate_rain_delay_pct >= 12.0);

        // Apply seawalls to reduce flood risk
        world.apply_brush(-6.12, 106.85, EarthBrushAction::BuildSeawall, true);
        let mitigated = compute_site_environmental_coupling(-6.12, 106.85, &world);
        assert!(mitigated.flood_risk_index <= coastal.flood_risk_index);
    }

    #[test]
    fn test_mep_engineering_sizing_hvac_plumbing_electrical() {
        let inp = MepSizingInput::default();
        let res = compute_mep_engineering_sizing(&inp, 4);
        assert!(res.cooling_kw > 150.0);
        assert!(res.duct_width_mm >= 200.0 && res.duct_height_mm >= 150.0);
        assert!(res.pipe_dn_mm >= 25.0 && res.pipe_velocity_ms <= 2.25);
        assert!(res.recommended_breaker_at >= 250);
        assert!(res.voltage_drop_pct > 0.0 && res.voltage_drop_pct < 5.0);
    }

    #[test]
    fn test_procurement_diary_and_qa_qc_ledgers() {
        let proc = ProcurementContractLedger::default();
        let pos = proc.evaluate_position(16_000_000_000.0);
        assert!(pos.certified_ipc_idr > 0.0);
        assert!(pos.net_payable_after_retention_idr < pos.certified_ipc_idr);
        assert!((pos.approved_vo_idr - 345_000_000.0).abs() < 1.0);
        assert_eq!(pos.total_eot_days, 6);

        let diary = SiteDailyDiaryLedger::default();
        let dsum = diary.summarize();
        assert!(dsum.avg_headcount > 45.0);
        assert!((dsum.total_concrete_m3 - 208.0).abs() < 1e-3);

        let qa = FieldQualityControlLedger::default();
        let qsum = qa.evaluate_scorecard();
        assert_eq!(qsum.open_ncr_count, 2);
        assert!(qsum.fc_pass_rate_pct > 60.0 && qsum.fc_pass_rate_pct < 70.0);
    }
}
