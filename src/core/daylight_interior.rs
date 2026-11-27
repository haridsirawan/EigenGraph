//! core/daylight_interior.rs — Update `v0.8.6.4` (`4/10` -> `v0.8.7`):
//! Interior Daylight Autonomy (`sDA 300/50%`, `ASE 1000,250h`, `UDI`, `DF%`), Glare (`DGP`),
//! LEED v4.1 EQ Credit, SNI 6197 LPD Savings & Architectural Room Finish / FF&E Engine.
//!
//! Couples active `BimProject` extents, room definitions, story elevations, and
//! `SiteEnvironmentalCoupling` latitude into spatial work-plane lux grids (3D ENU cells),
//! architectural surface reflectance, glare indices, and `eigengraph::design::daylight_interior` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::massing_agent::MassingStudyReport;
use crate::core::solar_envelope::SolarEnvelopeConfig;

/// Canonical EigenGraph IFCX attribute namespace for Interior Daylight & Architectural Finishes.
pub const NS_EG_DAYLIGHT_INTERIOR: &str = "eigengraph::design::daylight_interior";

/// Interior architectural finish palette presets driving surface reflectance (`R_c, R_w, R_f`) & finish cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteriorFinishPalette {
    MinimalistDaylight,
    ExecutiveWarmTimber,
    IndustrialExposedConcrete,
    HealthcareCleanroom,
}

impl InteriorFinishPalette {
    pub const ALL: [Self; 4] = [
        Self::MinimalistDaylight,
        Self::ExecutiveWarmTimber,
        Self::IndustrialExposedConcrete,
        Self::HealthcareCleanroom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::MinimalistDaylight => "Minimalist High-Reflectance Acoustic",
            Self::ExecutiveWarmTimber => "Executive Warm Timber & Terrazzo",
            Self::IndustrialExposedConcrete => "Industrial Exposed Concrete Loft",
            Self::HealthcareCleanroom => "Healthcare Anti-Microbial Bright Vinyl",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::MinimalistDaylight => "Minimalist",
            Self::ExecutiveWarmTimber => "Timber",
            Self::IndustrialExposedConcrete => "Concrete",
            Self::HealthcareCleanroom => "Cleanroom",
        }
    }

    /// Returns `(ceiling_reflectance, wall_reflectance, floor_reflectance)`.
    pub fn surface_reflectances(self) -> (f64, f64, f64) {
        match self {
            Self::MinimalistDaylight => (0.85, 0.72, 0.36),
            Self::ExecutiveWarmTimber => (0.76, 0.48, 0.24),
            Self::IndustrialExposedConcrete => (0.52, 0.42, 0.22),
            Self::HealthcareCleanroom => (0.88, 0.78, 0.44),
        }
    }

    pub fn finish_cost_usd_m2(self) -> f64 {
        match self {
            Self::MinimalistDaylight => 145.0,
            Self::ExecutiveWarmTimber => 235.0,
            Self::IndustrialExposedConcrete => 120.0,
            Self::HealthcareCleanroom => 260.0,
        }
    }
}

/// Interior functional space typology setting SNI 6197 target lux, baseline LPD & FF&E unit costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteriorSpaceTypology {
    OpenOffice,
    Classroom,
    HealthcareWard,
    RetailShowroom,
    ResidentialSuite,
}

impl InteriorSpaceTypology {
    pub const ALL: [Self; 5] = [
        Self::OpenOffice,
        Self::Classroom,
        Self::HealthcareWard,
        Self::RetailShowroom,
        Self::ResidentialSuite,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OpenOffice => "Open-Plan Office (SNI 6197: 350 lx)",
            Self::Classroom => "Academic Classroom (SNI 6197: 300 lx)",
            Self::HealthcareWard => "Healthcare Ward / Clinic (SNI 6197: 400 lx)",
            Self::RetailShowroom => "Commercial Retail Showroom (SNI 6197: 500 lx)",
            Self::ResidentialSuite => "Residential Apartment Suite (SNI 6197: 200 lx)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::OpenOffice => "Office",
            Self::Classroom => "Classroom",
            Self::HealthcareWard => "Healthcare",
            Self::RetailShowroom => "Retail",
            Self::ResidentialSuite => "Residential",
        }
    }

    pub fn target_lux_sni_6197(self) -> f64 {
        match self {
            Self::OpenOffice => 350.0,
            Self::Classroom => 300.0,
            Self::HealthcareWard => 400.0,
            Self::RetailShowroom => 500.0,
            Self::ResidentialSuite => 200.0,
        }
    }

    pub fn baseline_lpd_w_m2(self) -> f64 {
        match self {
            Self::OpenOffice => 9.5,
            Self::Classroom => 10.5,
            Self::HealthcareWard => 11.5,
            Self::RetailShowroom => 13.5,
            Self::ResidentialSuite => 7.5,
        }
    }

    pub fn ffe_cost_usd_m2(self) -> f64 {
        match self {
            Self::OpenOffice => 185.0,
            Self::Classroom => 140.0,
            Self::HealthcareWard => 320.0,
            Self::RetailShowroom => 260.0,
            Self::ResidentialSuite => 210.0,
        }
    }
}

/// Persisted configuration for 3D Interior Daylight Autonomy, Glare & Room Finishes Engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaylightInteriorConfig {
    pub finish_palette: InteriorFinishPalette,
    pub space_typology: InteriorSpaceTypology,
    pub wwr_pct: f64,
    pub visible_transmittance_vlt: f64,
    pub shading_transmittance: f64,
    pub active_story_index: usize,
    pub work_plane_height_m: f64,
    pub grid_nx: usize,
    pub grid_ny: usize,
}

impl Default for DaylightInteriorConfig {
    fn default() -> Self {
        Self {
            finish_palette: InteriorFinishPalette::MinimalistDaylight,
            space_typology: InteriorSpaceTypology::OpenOffice,
            wwr_pct: 42.0,
            visible_transmittance_vlt: 0.62,
            shading_transmittance: 0.78,
            active_story_index: 0,
            work_plane_height_m: 0.85,
            grid_nx: 10,
            grid_ny: 8,
        }
    }
}

/// Work-plane illuminance & daylight autonomy grid cell in local 3D ENU coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaylightCell3D {
    pub ix: usize,
    pub iy: usize,
    pub corners_enu_m: [[f64; 3]; 4],
    pub daylight_factor_pct: f64,
    pub mean_lux: f64,
    pub sda_pass: bool,
    pub ase_glare: bool,
    pub color_rgba: [f32; 4],
}

/// Architectural finish surface & FF&E quantity breakdown per functional zone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomFinishScheduleItem {
    pub zone_name: String,
    pub floor_area_m2: f64,
    pub ceiling_area_m2: f64,
    pub wall_area_m2: f64,
    pub target_lux: f64,
    pub achieved_lux: f64,
    pub finish_cost_usd: f64,
    pub ffe_cost_usd: f64,
}

/// Compact serializable IFCX overlay payload under `eigengraph::design::daylight_interior`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaylightInteriorIfcxOverlay {
    pub schema_version: String,
    pub site_label: String,
    pub finish_palette: String,
    pub space_typology: String,
    pub sda_300_50_pct: f64,
    pub ase_1000_250_pct: f64,
    pub udi_300_3000_pct: f64,
    pub mean_lux: f64,
    pub peak_dgp_glare: f64,
    pub leed_v41_credits: u8,
    pub lighting_energy_savings_pct: f64,
    pub total_finish_ffe_usd: f64,
    pub en_17037_compliant: bool,
    pub summary_verdict: String,
}

/// Complete Interior Daylight Autonomy, Glare (DGP), SNI 6197 LPD & Finish/FF&E Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaylightInteriorReport {
    pub config: DaylightInteriorConfig,
    pub site_label: String,
    pub building_width_m: f64,
    pub building_depth_m: f64,
    pub story_elev_m: f64,
    pub stories: usize,
    pub min_enu_xy_m: [f64; 2],
    pub max_enu_xy_m: [f64; 2],
    pub avg_reflectance: f64,
    pub mean_daylight_factor_pct: f64,
    pub mean_lux: f64,
    pub sda_300_50_pct: f64,
    pub ase_1000_250_pct: f64,
    pub udi_300_3000_pct: f64,
    pub peak_dgp_glare: f64,
    pub leed_v41_credits: u8,
    pub lighting_energy_savings_pct: f64,
    pub annual_lighting_kwh_m2: f64,
    pub total_finish_usd: f64,
    pub total_ffe_usd: f64,
    pub en_17037_compliant: bool,
    pub rooms: Vec<RoomFinishScheduleItem>,
    pub cells_3d: Vec<DaylightCell3D>,
    pub summary_verdict: String,
}

impl DaylightInteriorReport {
    pub fn to_ifcx_overlay(&self) -> DaylightInteriorIfcxOverlay {
        DaylightInteriorIfcxOverlay {
            schema_version: "daylight-interior-ifcx-v1".to_string(),
            site_label: self.site_label.clone(),
            finish_palette: self.config.finish_palette.label().to_string(),
            space_typology: self.config.space_typology.label().to_string(),
            sda_300_50_pct: self.sda_300_50_pct,
            ase_1000_250_pct: self.ase_1000_250_pct,
            udi_300_3000_pct: self.udi_300_3000_pct,
            mean_lux: self.mean_lux,
            peak_dgp_glare: self.peak_dgp_glare,
            leed_v41_credits: self.leed_v41_credits,
            lighting_energy_savings_pct: self.lighting_energy_savings_pct,
            total_finish_ffe_usd: self.total_finish_usd + self.total_ffe_usd,
            en_17037_compliant: self.en_17037_compliant,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

fn extract_daylight_footprint(
    bim: &BimProject,
    massing: Option<&MassingStudyReport>,
    story_idx: usize,
) -> ([f64; 2], [f64; 2], f64, f64, usize) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for col in &bim.columns {
        let x = col.pos.0 * 0.001;
        let y = col.pos.1 * 0.001;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    for wall in &bim.walls {
        for p in [wall.start, wall.end] {
            let x = p.0 * 0.001;
            let y = p.1 * 0.001;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    for slab in &bim.slabs {
        for p in &slab.boundary {
            let x = p.0 * 0.001;
            let y = p.1 * 0.001;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }

    if min_x.is_finite() && (max_x - min_x) > 1.0 && (max_y - min_y) > 1.0 {
        let mut elevs: Vec<f64> = bim.levels.iter().map(|l| l.elevation_mm * 0.001).collect();
        elevs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        elevs.dedup_by(|a, b| (*a - *b).abs() < 0.25);
        let stories = elevs.len().saturating_sub(1).max(1);
        let clamped_idx = story_idx.min(stories.saturating_sub(1));
        let base_elev = elevs.get(clamped_idx).copied().unwrap_or(0.0);
        let next_elev = elevs.get(clamped_idx + 1).copied().unwrap_or(base_elev + 3.6);
        let floor_h = (next_elev - base_elev).clamp(2.8, 6.0);
        return ([min_x, min_y], [max_x, max_y], base_elev, floor_h, stories);
    }

    if let Some(mrep) = massing {
        if let Some(opt) = mrep.active_option() {
            let mut mx0 = f64::INFINITY;
            let mut mx1 = f64::NEG_INFINITY;
            let mut my0 = f64::INFINITY;
            let mut my1 = f64::NEG_INFINITY;
            for b in opt.blocks_3d.iter().filter(|b| !b.is_core) {
                mx0 = mx0.min(b.min_enu_m[0]);
                mx1 = mx1.max(b.max_enu_m[0]);
                my0 = my0.min(b.min_enu_m[1]);
                my1 = my1.max(b.max_enu_m[1]);
            }
            if mx0.is_finite() && mx1 > mx0 && my1 > my0 {
                let stories = opt.stories.max(1);
                let floor_h = (opt.total_height_m / (stories as f64)).clamp(2.8, 6.0);
                let clamped_idx = story_idx.min(stories.saturating_sub(1));
                return (
                    [mx0, my0],
                    [mx1, my1],
                    (clamped_idx as f64) * floor_h,
                    floor_h,
                    stories,
                );
            }
        }
    }

    let stories = 6usize;
    let floor_h = 3.6;
    let clamped_idx = story_idx.min(stories - 1);
    (
        [-14.0, -10.0],
        [14.0, 10.0],
        (clamped_idx as f64) * floor_h,
        floor_h,
        stories,
    )
}

fn lux_heatmap_rgba(lux: f64, sda_pass: bool, ase_glare: bool) -> [f32; 4] {
    if ase_glare {
        // High-intensity direct solar glare risk -> warm coral-amber
        let u = ((lux - 1000.0) / 1400.0).clamp(0.0, 1.0) as f32;
        [0.98, 0.56 - 0.30 * u, 0.14, 0.80]
    } else if sda_pass {
        // Optimal sDA >= 300 lux -> vibrant emerald to bright cyan-gold
        let u = ((lux - 300.0) / 700.0).clamp(0.0, 1.0) as f32;
        [0.12 + 0.45 * u, 0.82 + 0.08 * u, 0.52 - 0.22 * u, 0.74]
    } else {
        // Dim core < 300 lux -> deep slate blue to muted teal
        let u = (lux / 300.0).clamp(0.0, 1.0) as f32;
        [0.12 + 0.06 * u, 0.24 + 0.36 * u, 0.58 + 0.16 * u, 0.68]
    }
}

/// Evaluates 3D interior daylight autonomy (`sDA/ASE/UDI/DF%`), glare (`DGP`),
/// LEED v4.1 credits, SNI 6197 lighting energy savings, and room finish / FF&E schedules.
pub fn evaluate_daylight_interior(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    massing: Option<&MassingStudyReport>,
    solar_cfg: Option<&SolarEnvelopeConfig>,
    config: &DaylightInteriorConfig,
) -> DaylightInteriorReport {
    let mut cfg = config.clone();
    if let Some(scfg) = solar_cfg {
        cfg.wwr_pct = scfg.wwr_pct;
    }
    cfg.wwr_pct = cfg.wwr_pct.clamp(10.0, 85.0);
    cfg.visible_transmittance_vlt = cfg.visible_transmittance_vlt.clamp(0.20, 0.88);
    cfg.shading_transmittance = cfg.shading_transmittance.clamp(0.30, 1.00);
    cfg.work_plane_height_m = cfg.work_plane_height_m.clamp(0.60, 1.10);
    cfg.grid_nx = cfg.grid_nx.clamp(4, 16);
    cfg.grid_ny = cfg.grid_ny.clamp(4, 16);

    let default_site = SiteEnvironmentalCoupling::default();
    let s = site.unwrap_or(&default_site);
    let site_label = s.site_label.clone();
    let lat_abs = s.lat_deg.abs();

    let (min_xy, max_xy, story_elev_m, floor_h, stories) =
        extract_daylight_footprint(bim, massing, cfg.active_story_index);
    cfg.active_story_index = cfg.active_story_index.min(stories.saturating_sub(1));

    let width_m = (max_xy[0] - min_xy[0]).max(4.0);
    let depth_m = (max_xy[1] - min_xy[1]).max(4.0);
    let floor_area_m2 = width_m * depth_m;
    let perimeter_m = 2.0 * (width_m + depth_m);
    let total_wall_area = perimeter_m * floor_h;
    let wwr = cfg.wwr_pct / 100.0;
    let a_win = total_wall_area * wwr;
    let a_opaque = total_wall_area * (1.0 - wwr);
    let a_surf = 2.0 * floor_area_m2 + total_wall_area;

    let (r_c, r_w, r_f) = cfg.finish_palette.surface_reflectances();
    let avg_reflectance =
        ((r_c * floor_area_m2 + r_f * floor_area_m2 + r_w * a_opaque + 0.14 * a_win)
            / a_surf.max(1.0))
        .clamp(0.15, 0.85);

    // Latitude-dependent sky diffuse zenith illuminance (tropical sites have higher diffuse sky lux)
    let sky_diffuse_lux = (24_500.0 - 95.0 * lat_abs).clamp(15_000.0, 26_000.0);
    let direct_sun_lux = (68_000.0 - 180.0 * lat_abs).clamp(42_000.0, 72_000.0);
    let h_win = (floor_h * (0.45 + 0.45 * wwr)).clamp(1.2, 4.2);
    let eff_t = cfg.visible_transmittance_vlt * cfg.shading_transmittance;
    let bre_theta = 42.0_f64;

    let work_z = story_elev_m + cfg.work_plane_height_m;
    let dx = width_m / (cfg.grid_nx as f64);
    let dy = depth_m / (cfg.grid_ny as f64);

    let mut cells_3d = Vec::with_capacity(cfg.grid_nx * cfg.grid_ny);
    let mut sum_df = 0.0_f64;
    let mut sum_lux = 0.0_f64;
    let mut sda_count = 0usize;
    let mut ase_count = 0usize;
    let mut udi_count = 0usize;
    let mut peak_direct_v = 0.0_f64;
    let mut perim_lux_sum = 0.0_f64;
    let mut perim_cnt = 0usize;
    let mut mid_lux_sum = 0.0_f64;
    let mut mid_cnt = 0usize;
    let mut core_lux_sum = 0.0_f64;
    let mut core_cnt = 0usize;

    for iy in 0..cfg.grid_ny {
        let y0 = min_xy[1] + (iy as f64) * dy;
        let y1 = y0 + dy;
        let yc = 0.5 * (y0 + y1);
        for ix in 0..cfg.grid_nx {
            let x0 = min_xy[0] + (ix as f64) * dx;
            let x1 = x0 + dx;
            let xc = 0.5 * (x0 + x1);

            let d_left = (xc - min_xy[0]).max(0.15);
            let d_right = (max_xy[0] - xc).max(0.15);
            let d_bot = (yc - min_xy[1]).max(0.15);
            let d_top = (max_xy[1] - yc).max(0.15);
            let d_perim = d_left.min(d_right).min(d_bot).min(d_top);

            // BRE Split-Flux Daylight Factor (%) with perimeter-to-core exponential attenuation
            let df_irc_pct = (0.42 * eff_t * a_win * bre_theta)
                / (a_surf * (1.0 - avg_reflectance).max(0.15));
            let df_pct = (df_irc_pct * (0.45 + 1.95 * (-d_perim / (2.2 * h_win)).exp()))
                .clamp(0.15, 16.0);

            let diffuse_lux = (df_pct * 0.01) * sky_diffuse_lux;
            let direct_component = if d_perim < 3.8 {
                direct_sun_lux
                    * 0.085
                    * eff_t
                    * wwr
                    * ((3.8 - d_perim) / 3.8).powf(1.35)
            } else {
                0.0
            };
            let mean_lux = diffuse_lux + direct_component;

            let sda_pass = mean_lux >= 300.0;
            let ase_glare = d_perim < 2.6 && direct_component >= 950.0 && eff_t > 0.44;
            let udi_ok = (300.0..=3000.0).contains(&mean_lux);

            sum_df += df_pct;
            sum_lux += mean_lux;
            if sda_pass {
                sda_count += 1;
            }
            if ase_glare {
                ase_count += 1;
            }
            if udi_ok {
                udi_count += 1;
            }
            if mean_lux > peak_direct_v {
                peak_direct_v = mean_lux;
            }

            if d_perim <= 4.0 {
                perim_lux_sum += mean_lux;
                perim_cnt += 1;
            } else if d_perim <= 7.5 {
                mid_lux_sum += mean_lux;
                mid_cnt += 1;
            } else {
                core_lux_sum += mean_lux;
                core_cnt += 1;
            }

            cells_3d.push(DaylightCell3D {
                ix,
                iy,
                corners_enu_m: [
                    [x0 + 0.04, y0 + 0.04, work_z],
                    [x1 - 0.04, y0 + 0.04, work_z],
                    [x1 - 0.04, y1 - 0.04, work_z],
                    [x0 + 0.04, y1 - 0.04, work_z],
                ],
                daylight_factor_pct: df_pct,
                mean_lux,
                sda_pass,
                ase_glare,
                color_rgba: lux_heatmap_rgba(mean_lux, sda_pass, ase_glare),
            });
        }
    }

    let n_cells = (cells_3d.len()).max(1) as f64;
    let mean_daylight_factor_pct = sum_df / n_cells;
    let mean_lux = sum_lux / n_cells;
    let sda_300_50_pct = ((sda_count as f64) / n_cells) * 100.0;
    let ase_1000_250_pct = ((ase_count as f64) / n_cells) * 100.0;
    let udi_300_3000_pct = ((udi_count as f64) / n_cells) * 100.0;

    // Simplified Wienold Daylight Glare Probability (DGP) driven by vertical eye illuminance
    let vertical_eye_lux = peak_direct_v * 1.15;
    let peak_dgp_glare =
        (5.87e-5 * vertical_eye_lux + 0.16 + 0.002 * ase_1000_250_pct).clamp(0.16, 0.72);

    let leed_v41_credits = if sda_300_50_pct >= 75.0 && ase_1000_250_pct <= 10.0 {
        3
    } else if sda_300_50_pct >= 55.0 && ase_1000_250_pct <= 10.0 {
        2
    } else if sda_300_50_pct >= 40.0 && ase_1000_250_pct <= 15.0 {
        1
    } else {
        0
    };

    let target_lux = cfg.space_typology.target_lux_sni_6197();
    let dimming_frac = ((sda_300_50_pct / 100.0) * 0.65).clamp(0.08, 0.62);
    let lighting_energy_savings_pct = dimming_frac * 100.0;
    let base_lpd = cfg.space_typology.baseline_lpd_w_m2();
    let annual_lighting_kwh_m2 = base_lpd * (1.0 - dimming_frac) * 2.85;

    let fin_rate = cfg.finish_palette.finish_cost_usd_m2();
    let ffe_rate = cfg.space_typology.ffe_cost_usd_m2();

    let perim_area = floor_area_m2 * ((perim_cnt as f64) / n_cells).max(0.25);
    let mid_area = floor_area_m2 * ((mid_cnt as f64) / n_cells).max(0.20);
    let core_area = (floor_area_m2 - perim_area - mid_area).max(floor_area_m2 * 0.15);
    let scale_sum = (perim_area + mid_area + core_area).max(1.0);

    let zones = [
        (
            "Perimeter Daylight Zone (0-4m)",
            floor_area_m2 * (perim_area / scale_sum),
            if perim_cnt > 0 { perim_lux_sum / (perim_cnt as f64) } else { mean_lux * 1.35 },
        ),
        (
            "Mid-Depth Work Zone (4-7.5m)",
            floor_area_m2 * (mid_area / scale_sum),
            if mid_cnt > 0 { mid_lux_sum / (mid_cnt as f64) } else { mean_lux * 0.85 },
        ),
        (
            "Core & Circulation Zone (>7.5m)",
            floor_area_m2 * (core_area / scale_sum),
            if core_cnt > 0 { core_lux_sum / (core_cnt as f64) } else { mean_lux * 0.45 },
        ),
    ];

    let mut rooms = Vec::with_capacity(zones.len());
    let mut total_finish_usd = 0.0;
    let mut total_ffe_usd = 0.0;
    for (zname, area, ach_lux) in zones {
        let wall_a = (area / floor_area_m2) * a_opaque;
        let fin_cost = area * fin_rate * 0.85 + wall_a * fin_rate * 0.15;
        let ffe_cost = area * ffe_rate;
        total_finish_usd += fin_cost;
        total_ffe_usd += ffe_cost;
        rooms.push(RoomFinishScheduleItem {
            zone_name: zname.to_string(),
            floor_area_m2: area,
            ceiling_area_m2: area,
            wall_area_m2: wall_a,
            target_lux,
            achieved_lux: ach_lux,
            finish_cost_usd: fin_cost,
            ffe_cost_usd: ffe_cost,
        });
    }

    let en_17037_compliant = sda_300_50_pct >= 50.0 && ase_1000_250_pct <= 12.0 && peak_dgp_glare <= 0.45;
    let tag = if en_17037_compliant { "PASS" } else { "REVIEW" };
    let summary_verdict = format!(
        "sDA {:.0}% • ASE {:.0}% • Mean {:.0} lx (DF {:.1}%) • DGP {:.2} • LEED +{}pt [{tag}]",
        sda_300_50_pct, ase_1000_250_pct, mean_lux, mean_daylight_factor_pct, peak_dgp_glare, leed_v41_credits,
    );

    DaylightInteriorReport {
        config: cfg,
        site_label,
        building_width_m: width_m,
        building_depth_m: depth_m,
        story_elev_m,
        stories,
        min_enu_xy_m: min_xy,
        max_enu_xy_m: max_xy,
        avg_reflectance,
        mean_daylight_factor_pct,
        mean_lux,
        sda_300_50_pct,
        ase_1000_250_pct,
        udi_300_3000_pct,
        peak_dgp_glare,
        leed_v41_credits,
        lighting_energy_savings_pct,
        annual_lighting_kwh_m2,
        total_finish_usd,
        total_ffe_usd,
        en_17037_compliant,
        rooms,
        cells_3d,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn finish_reflectance_and_vlt_increase_sda_monotonically() {
        let bim = BimProject::empty();
        let site = SiteEnvironmentalCoupling::default();

        let mut cfg_dark = DaylightInteriorConfig {
            finish_palette: InteriorFinishPalette::IndustrialExposedConcrete,
            visible_transmittance_vlt: 0.30,
            ..Default::default()
        };
        let rep_dark = evaluate_daylight_interior(&bim, Some(&site), None, None, &cfg_dark);

        cfg_dark.finish_palette = InteriorFinishPalette::MinimalistDaylight;
        let rep_bright_wall = evaluate_daylight_interior(&bim, Some(&site), None, None, &cfg_dark);

        cfg_dark.visible_transmittance_vlt = 0.72;
        let rep_high_vlt = evaluate_daylight_interior(&bim, Some(&site), None, None, &cfg_dark);

        assert!(rep_bright_wall.mean_lux > rep_dark.mean_lux);
        assert!(rep_high_vlt.mean_lux > rep_bright_wall.mean_lux);
        assert!(rep_high_vlt.sda_300_50_pct >= rep_dark.sda_300_50_pct);
    }

    #[test]
    fn generates_3d_lux_grid_leed_credits_and_ifcx_overlay() {
        let bim = BimProject::empty();
        let rep = evaluate_daylight_interior(
            &bim,
            None,
            None,
            None,
            &DaylightInteriorConfig::default(),
        );
        assert_eq!(rep.cells_3d.len(), rep.config.grid_nx * rep.config.grid_ny);
        assert_eq!(rep.rooms.len(), 3);
        assert!(rep.mean_daylight_factor_pct > 0.2);

        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed =
            IfcxStage::compose_with_daylight_interior_overlay(&base, &rep.to_ifcx_overlay());
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_DAYLIGHT_INTERIOR)));
    }
}
