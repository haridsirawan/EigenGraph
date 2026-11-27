//! core/solar_envelope.rs — Update `v0.8.6.1` (`1/10` -> `v0.8.7`):
//! 3D Solar Irradiation, Cast Shadow & OTTV/RTTV Facade Envelope Engine (`SNI 6389 / ASHRAE 90.1`).
//!
//! Couples active `BimProject` extents (or active `MassingStudyReport` option) and
//! `SiteEnvironmentalCoupling` latitude into orientation-resolved Solar Factors (`N/E/S/W/Roof`),
//! real-time Overall Thermal Transfer Value (`OTTV` / `RTTV`), cooling energy intensity,
//! per-story 3D ENU irradiance heatmap panels, analytical cast ground shadows, and
//! non-destructive `eigengraph::design::solar_ottv` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::massing_agent::MassingStudyReport;

/// Canonical EigenGraph IFCX attribute namespace for 3D Solar Irradiation & OTTV Overlays.
pub const NS_EG_SOLAR_OTTV: &str = "eigengraph::design::solar_ottv";

/// Glazing optical & thermal performance presets for facade OTTV evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GlazingPerformancePreset {
    ClearSingle,
    TintedDouble,
    LowESpectrallySelective,
    TripleHighPerf,
}

impl GlazingPerformancePreset {
    pub const ALL: [Self; 4] = [
        Self::ClearSingle,
        Self::TintedDouble,
        Self::LowESpectrallySelective,
        Self::TripleHighPerf,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ClearSingle => "Clear Single Glass (6mm)",
            Self::TintedDouble => "Tinted Double IGU",
            Self::LowESpectrallySelective => "Low-E Spectrally Selective IGU",
            Self::TripleHighPerf => "Triple Glazed High-Performance IGU",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::ClearSingle => "Clear",
            Self::TintedDouble => "Tinted",
            Self::LowESpectrallySelective => "Low-E",
            Self::TripleHighPerf => "Triple",
        }
    }

    /// Returns `(u_glass_w_m2k, shgc)`.
    pub fn defaults(self) -> (f64, f64) {
        match self {
            Self::ClearSingle => (5.60, 0.81),
            Self::TintedDouble => (2.80, 0.52),
            Self::LowESpectrallySelective => (1.60, 0.34),
            Self::TripleHighPerf => (1.10, 0.25),
        }
    }
}

/// External solar shading device archetype modulating fenestration solar heat gain (`ESF`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolarShadingStrategy {
    None,
    HorizontalOverhang,
    VerticalFins,
    EggCrateLouvers,
}

impl SolarShadingStrategy {
    pub const ALL: [Self; 4] = [
        Self::None,
        Self::HorizontalOverhang,
        Self::VerticalFins,
        Self::EggCrateLouvers,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "Unshaded Curtain Wall",
            Self::HorizontalOverhang => "Horizontal Sunshade Overhang",
            Self::VerticalFins => "Vertical Architectural Fins",
            Self::EggCrateLouvers => "3D Egg-Crate Louvers",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::HorizontalOverhang => "Overhang",
            Self::VerticalFins => "Fins",
            Self::EggCrateLouvers => "Louvers",
        }
    }

    /// Effective shading multiplier (`0.42..=1.0`, lower blocks more direct solar gain).
    pub fn base_shading_factor(self, depth_m: f64) -> f64 {
        let d = depth_m.clamp(0.0, 2.5);
        let eff = (d / 1.2).clamp(0.0, 1.4);
        match self {
            Self::None => 1.0,
            Self::HorizontalOverhang => (1.0 - 0.34 * eff).clamp(0.54, 1.0),
            Self::VerticalFins => (1.0 - 0.29 * eff).clamp(0.58, 1.0),
            Self::EggCrateLouvers => (1.0 - 0.45 * eff).clamp(0.42, 1.0),
        }
    }
}

/// Persisted configuration for 3D Solar Irradiation, Shadow & SNI 6389 OTTV Engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarEnvelopeConfig {
    pub glazing_preset: GlazingPerformancePreset,
    pub shading_strategy: SolarShadingStrategy,
    pub wwr_pct: f64,
    pub shgc: f64,
    pub u_wall_w_m2k: f64,
    pub u_glass_w_m2k: f64,
    pub wall_absorptance: f64,
    pub shading_depth_m: f64,
    pub inspection_month: u8,
    pub inspection_hour_local: f64,
    pub building_azimuth_deg: f64,
    pub ottv_limit_w_m2: f64,
    pub show_shadow_polygon: bool,
}

impl Default for SolarEnvelopeConfig {
    fn default() -> Self {
        let preset = GlazingPerformancePreset::LowESpectrallySelective;
        let (u_glass, shgc) = preset.defaults();
        Self {
            glazing_preset: preset,
            shading_strategy: SolarShadingStrategy::HorizontalOverhang,
            wwr_pct: 42.0,
            shgc,
            u_wall_w_m2k: 1.25,
            u_glass_w_m2k: u_glass,
            wall_absorptance: 0.55,
            shading_depth_m: 0.90,
            inspection_month: 6,
            inspection_hour_local: 14.0,
            building_azimuth_deg: 0.0,
            ottv_limit_w_m2: 35.0,
            show_shadow_polygon: true,
        }
    }
}

impl SolarEnvelopeConfig {
    /// Applies a glazing performance preset (`U_glass`, `SHGC`) cleanly.
    pub fn apply_glazing(&mut self, preset: GlazingPerformancePreset) {
        let (u, shgc) = preset.defaults();
        self.glazing_preset = preset;
        self.u_glass_w_m2k = u;
        self.shgc = shgc;
    }
}

/// Thermal transfer & annual solar irradiation telemetry for a single building envelope orientation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacadeOrientationMetric {
    pub orientation: String,
    pub azimuth_deg: f64,
    pub gross_area_m2: f64,
    pub solar_factor_sf_w_m2: f64,
    pub annual_irradiation_kwh_m2: f64,
    pub ottv_w_m2: f64,
    pub peak_heat_gain_kw: f64,
    pub shading_reduction_pct: f64,
    pub compliant: bool,
}

/// 3D ENU facade or roof quad colored by instantaneous solar intensity & OTTV thermal severity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarHeatmapPanel3D {
    pub orientation: String,
    pub story_index: usize,
    pub corners_enu_m: [[f64; 3]; 4],
    pub normal_enu: [f32; 3],
    pub intensity_norm: f32,
    pub ottv_w_m2: f64,
    pub color_rgba: [f32; 4],
}

/// Analytical cast ground shadow footprint projected from the building box along the active sun ray.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarShadowQuad3D {
    pub corners_enu_m: [[f64; 3]; 4],
    pub shadow_length_m: f64,
}

/// Compact serializable IFCX overlay payload under `eigengraph::design::solar_ottv`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarOttvIfcxOverlay {
    pub schema_version: String,
    pub site_label: String,
    pub lat_deg: f64,
    pub overall_ottv_w_m2: f64,
    pub ottv_limit_w_m2: f64,
    pub rttv_roof_w_m2: f64,
    pub wwr_pct: f64,
    pub shgc: f64,
    pub shading_strategy: String,
    pub annual_cooling_kwh_m2: f64,
    pub peak_solar_load_kw: f64,
    pub worst_orientation: String,
    pub sni_6389_compliant: bool,
    pub summary_verdict: String,
}

/// Complete 3D Solar Irradiation, Shadow & SNI 6389 / ASHRAE 90.1 OTTV Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarEnvelopeReport {
    pub config: SolarEnvelopeConfig,
    pub site_label: String,
    pub lat_deg: f64,
    pub building_width_m: f64,
    pub building_depth_m: f64,
    pub building_height_m: f64,
    pub stories: usize,
    pub min_enu_xy_m: [f64; 2],
    pub max_enu_xy_m: [f64; 2],
    pub sun_altitude_deg: f64,
    pub sun_azimuth_deg: f64,
    pub sun_dir_enu: [f64; 3],
    pub overall_ottv_w_m2: f64,
    pub rttv_roof_w_m2: f64,
    pub annual_cooling_kwh_m2: f64,
    pub peak_solar_load_kw: f64,
    pub total_facade_area_m2: f64,
    pub worst_orientation: String,
    pub sni_6389_compliant: bool,
    pub orientations: Vec<FacadeOrientationMetric>,
    pub heatmap_panels_3d: Vec<SolarHeatmapPanel3D>,
    pub shadow_quads_3d: Vec<SolarShadowQuad3D>,
    pub summary_verdict: String,
}

impl SolarEnvelopeReport {
    pub fn to_ifcx_overlay(&self) -> SolarOttvIfcxOverlay {
        SolarOttvIfcxOverlay {
            schema_version: "solar-ottv-ifcx-v1".to_string(),
            site_label: self.site_label.clone(),
            lat_deg: self.lat_deg,
            overall_ottv_w_m2: self.overall_ottv_w_m2,
            ottv_limit_w_m2: self.config.ottv_limit_w_m2,
            rttv_roof_w_m2: self.rttv_roof_w_m2,
            wwr_pct: self.config.wwr_pct,
            shgc: self.config.shgc,
            shading_strategy: self.config.shading_strategy.label().to_string(),
            annual_cooling_kwh_m2: self.annual_cooling_kwh_m2,
            peak_solar_load_kw: self.peak_solar_load_kw,
            worst_orientation: self.worst_orientation.clone(),
            sni_6389_compliant: self.sni_6389_compliant,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

fn compute_sun_vector(lat_deg: f64, month: u8, hour_local: f64) -> (f64, f64, [f64; 3]) {
    let day_of_year = ((month.clamp(1, 12) as f64) - 1.0) * 30.4 + 15.0;
    let decl_rad = (23.44_f64.to_radians())
        * (2.0 * std::f64::consts::PI * (284.0 + day_of_year) / 365.0).sin();
    let lat_rad = lat_deg.clamp(-85.0, 85.0).to_radians();
    let hour_angle_rad = ((hour_local.clamp(6.0, 18.0) - 12.0) * 15.0).to_radians();

    let sin_alt = (lat_rad.sin() * decl_rad.sin()
        + lat_rad.cos() * decl_rad.cos() * hour_angle_rad.cos())
    .clamp(-1.0, 1.0);
    let alt_rad = sin_alt.asin();
    let alt_deg = alt_rad.to_degrees();

    let cos_alt = alt_rad.cos().max(1e-5);
    let cos_az = ((decl_rad.sin() - lat_rad.sin() * sin_alt) / (lat_rad.cos() * cos_alt))
        .clamp(-1.0, 1.0);
    let mut az_deg = cos_az.acos().to_degrees();
    if hour_local > 12.0 {
        az_deg = 360.0 - az_deg;
    }

    let az_rad = az_deg.to_radians();
    let horiz = alt_rad.max(0.0).cos();
    let dir_enu = [horiz * az_rad.sin(), horiz * az_rad.cos(), sin_alt.max(0.02)];
    (alt_deg, az_deg, dir_enu)
}

fn extract_envelope_box(
    bim: &BimProject,
    massing: Option<&MassingStudyReport>,
) -> ([f64; 2], [f64; 2], f64, usize) {
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
        let (h, st) = if elevs.len() >= 2 {
            let top = elevs.last().copied().unwrap_or(3.6);
            let base = elevs.first().copied().unwrap_or(0.0);
            ((top - base).max(3.2), elevs.len().saturating_sub(1).max(1))
        } else {
            (3.6, 1)
        };
        return ([min_x, min_y], [max_x, max_y], h, st);
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
                return ([mx0, my0], [mx1, my1], opt.total_height_m.max(3.6), opt.stories.max(1));
            }
        }
    }

    ([-14.0, -10.0], [14.0, 10.0], 21.6, 6)
}

fn thermal_heatmap_rgba(intensity: f32) -> [f32; 4] {
    let t = intensity.clamp(0.0, 1.0);
    if t < 0.45 {
        let u = t / 0.45;
        // Cool cyan-emerald -> warm amber-yellow
        [0.08 + u * 0.84, 0.78 - u * 0.04, 0.74 - u * 0.56, 0.72]
    } else {
        let u = (t - 0.45) / 0.55;
        // Warm amber -> intense solar coral-red
        [0.92 + u * 0.06, 0.74 - u * 0.52, 0.18 - u * 0.04, 0.80]
    }
}

/// Evaluates 3D solar irradiation, analytical ground shadows, and SNI 6389 / ASHRAE 90.1 OTTV/RTTV.
pub fn evaluate_solar_envelope(
    bim: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    massing: Option<&MassingStudyReport>,
    config: &SolarEnvelopeConfig,
) -> SolarEnvelopeReport {
    let mut cfg = config.clone();
    cfg.wwr_pct = cfg.wwr_pct.clamp(10.0, 85.0);
    cfg.shgc = cfg.shgc.clamp(0.18, 0.88);
    cfg.u_wall_w_m2k = cfg.u_wall_w_m2k.clamp(0.25, 3.5);
    cfg.u_glass_w_m2k = cfg.u_glass_w_m2k.clamp(0.80, 5.8);
    cfg.wall_absorptance = cfg.wall_absorptance.clamp(0.20, 0.90);
    cfg.shading_depth_m = cfg.shading_depth_m.clamp(0.0, 2.5);
    cfg.inspection_month = cfg.inspection_month.clamp(1, 12);
    cfg.inspection_hour_local = cfg.inspection_hour_local.clamp(6.0, 18.0);
    cfg.ottv_limit_w_m2 = cfg.ottv_limit_w_m2.clamp(20.0, 60.0);

    let default_site = SiteEnvironmentalCoupling::default();
    let s = site.unwrap_or(&default_site);
    let lat_deg = s.lat_deg;
    let site_label = s.site_label.clone();

    let (min_xy, max_xy, height_m, stories) = extract_envelope_box(bim, massing);
    let width_m = (max_xy[0] - min_xy[0]).max(4.0);
    let depth_m = (max_xy[1] - min_xy[1]).max(4.0);

    let (sun_alt_deg, sun_az_deg, sun_dir_enu) =
        compute_sun_vector(lat_deg, cfg.inspection_month, cfg.inspection_hour_local);

    let wwr = cfg.wwr_pct / 100.0;
    let sc = (cfg.shgc / 0.87).clamp(0.18, 1.0);
    let shade_factor = cfg.shading_strategy.base_shading_factor(cfg.shading_depth_m);
    let reduction_pct = ((1.0 - shade_factor) * 100.0).clamp(0.0, 75.0);

    // SNI 6389:2020 base constants: TD_eq = 15.0 K (opaque wall), dT = 5.0 K (fenestration)
    let td_eq = 15.0;
    let delta_t = 5.0;
    let lat_abs = lat_deg.abs();

    // Base orientation solar factors (N, E, S, W) adjusted by latitude & azimuth rotation
    let ori_specs = [
        ("North", 0.0_f64, width_m * height_m, [0.0_f32, 1.0, 0.0]),
        ("East", 90.0_f64, depth_m * height_m, [1.0_f32, 0.0, 0.0]),
        ("South", 180.0_f64, width_m * height_m, [0.0_f32, -1.0, 0.0]),
        ("West", 270.0_f64, depth_m * height_m, [-1.0_f32, 0.0, 0.0]),
    ];

    let mut orientations = Vec::with_capacity(5);
    let mut ottv_area_sum = 0.0;
    let mut total_facade_area_m2 = 0.0;
    let mut total_peak_kw = 0.0;

    for (name, base_az, area_m2, _nrm) in ori_specs {
        let eff_az = (base_az + cfg.building_azimuth_deg).rem_euclid(360.0);
        let az_rad = eff_az.to_radians();
        // East/West peaks (~205 W/m2 equatorial), North/South milder (~130-150 W/m2)
        let east_west_weight = az_rad.sin().abs();
        let south_hemisphere_bias = if lat_deg < 0.0 {
            (-az_rad.cos()).max(0.0) * 18.0
        } else {
            az_rad.cos().max(0.0) * 18.0
        };
        let sf = (132.0 + 78.0 * east_west_weight + south_hemisphere_bias - (lat_abs * 0.45))
            .clamp(95.0, 225.0);

        let opaque_term = cfg.wall_absorptance * cfg.u_wall_w_m2k * (1.0 - wwr) * td_eq;
        let cond_term = cfg.u_glass_w_m2k * wwr * delta_t;
        let solar_term = sf * wwr * sc * shade_factor;
        let ottv_w_m2 = opaque_term + cond_term + solar_term;

        let annual_irr = (sf * 6.25 * (1.0 - 0.10 * (1.0 - shade_factor))).clamp(450.0, 1650.0);
        let peak_kw = (ottv_w_m2 * area_m2) / 1000.0;

        ottv_area_sum += ottv_w_m2 * area_m2;
        total_facade_area_m2 += area_m2;
        total_peak_kw += peak_kw;

        orientations.push(FacadeOrientationMetric {
            orientation: name.to_string(),
            azimuth_deg: eff_az,
            gross_area_m2: area_m2,
            solar_factor_sf_w_m2: sf,
            annual_irradiation_kwh_m2: annual_irr,
            ottv_w_m2,
            peak_heat_gain_kw: peak_kw,
            shading_reduction_pct: reduction_pct,
            compliant: ottv_w_m2 <= cfg.ottv_limit_w_m2,
        });
    }

    let overall_ottv_w_m2 = (ottv_area_sum / total_facade_area_m2.max(1.0)).max(0.0);

    // Roof RTTV (SNI 6389 Roof Thermal Transfer Value)
    let roof_area_m2 = width_m * depth_m;
    let roof_sf = (265.0 - lat_abs * 1.1).clamp(160.0, 290.0);
    let skylight_ratio = (wwr * 0.08).clamp(0.01, 0.08);
    let rttv_roof_w_m2 = (cfg.wall_absorptance * 0.55 * (1.0 - skylight_ratio) * 18.0)
        + (cfg.u_glass_w_m2k * skylight_ratio * delta_t)
        + (roof_sf * skylight_ratio * sc * shade_factor);
    let roof_peak_kw = (rttv_roof_w_m2 * roof_area_m2) / 1000.0;
    total_peak_kw += roof_peak_kw;

    orientations.push(FacadeOrientationMetric {
        orientation: "Roof".to_string(),
        azimuth_deg: 0.0,
        gross_area_m2: roof_area_m2,
        solar_factor_sf_w_m2: roof_sf,
        annual_irradiation_kwh_m2: (roof_sf * 6.8).clamp(1100.0, 2100.0),
        ottv_w_m2: rttv_roof_w_m2,
        peak_heat_gain_kw: roof_peak_kw,
        shading_reduction_pct: 0.0,
        compliant: rttv_roof_w_m2 <= 35.0,
    });

    let worst_orientation = orientations[..4]
        .iter()
        .max_by(|a, b| a.ottv_w_m2.total_cmp(&b.ottv_w_m2))
        .map(|o| o.orientation.clone())
        .unwrap_or_else(|| "West".to_string());

    let floor_gfa_est = (width_m * depth_m * (stories as f64)).max(100.0);
    let annual_cooling_kwh_m2 = (28.0 + overall_ottv_w_m2 * 1.85 + rttv_roof_w_m2 * 0.35)
        * (1.0 + (1.0 - lat_abs / 90.0) * 0.15)
        * (total_facade_area_m2 / floor_gfa_est).clamp(0.35, 1.45);

    // 3D ENU per-story facade heatmap quads + roof quad
    let mut heatmap_panels_3d = Vec::with_capacity(stories * 4 + 1);
    let story_h = height_m / (stories.max(1) as f64);
    let offset = 0.22_f64;

    for st in 0..stories.max(1) {
        let z0 = (st as f64) * story_h + 0.08;
        let z1 = ((st + 1) as f64) * story_h - 0.08;
        let height_exposure = 0.82 + 0.18 * (((st + 1) as f64) / (stories.max(1) as f64));

        // 0 = North (+Y), 1 = East (+X), 2 = South (-Y), 3 = West (-X)
        let wall_quads = [
            (
                0usize,
                [
                    [min_xy[0], max_xy[1] + offset, z0],
                    [max_xy[0], max_xy[1] + offset, z0],
                    [max_xy[0], max_xy[1] + offset, z1],
                    [min_xy[0], max_xy[1] + offset, z1],
                ],
                [0.0_f32, 1.0, 0.0],
            ),
            (
                1usize,
                [
                    [max_xy[0] + offset, min_xy[1], z0],
                    [max_xy[0] + offset, max_xy[1], z0],
                    [max_xy[0] + offset, max_xy[1], z1],
                    [max_xy[0] + offset, min_xy[1], z1],
                ],
                [1.0_f32, 0.0, 0.0],
            ),
            (
                2usize,
                [
                    [max_xy[0], min_xy[1] - offset, z0],
                    [min_xy[0], min_xy[1] - offset, z0],
                    [min_xy[0], min_xy[1] - offset, z1],
                    [max_xy[0], min_xy[1] - offset, z1],
                ],
                [0.0_f32, -1.0, 0.0],
            ),
            (
                3usize,
                [
                    [min_xy[0] - offset, max_xy[1], z0],
                    [min_xy[0] - offset, min_xy[1], z0],
                    [min_xy[0] - offset, min_xy[1], z1],
                    [min_xy[0] - offset, max_xy[1], z1],
                ],
                [-1.0_f32, 0.0, 0.0],
            ),
        ];

        for (oidx, corners, nrm) in wall_quads {
            let base_ottv = orientations[oidx].ottv_w_m2;
            let inc = ((nrm[0] as f64) * sun_dir_enu[0] + (nrm[1] as f64) * sun_dir_enu[1])
                .max(0.0);
            let ottv_inst = base_ottv * height_exposure * (0.78 + 0.42 * inc);
            let norm = ((ottv_inst - 16.0) / 36.0).clamp(0.05, 1.0) as f32;
            heatmap_panels_3d.push(SolarHeatmapPanel3D {
                orientation: orientations[oidx].orientation.clone(),
                story_index: st,
                corners_enu_m: corners,
                normal_enu: nrm,
                intensity_norm: norm,
                ottv_w_m2: ottv_inst,
                color_rgba: thermal_heatmap_rgba(norm),
            });
        }
    }

    // Roof heatmap panel
    let roof_norm = ((rttv_roof_w_m2 - 12.0) / 34.0).clamp(0.08, 1.0) as f32;
    heatmap_panels_3d.push(SolarHeatmapPanel3D {
        orientation: "Roof".to_string(),
        story_index: stories,
        corners_enu_m: [
            [min_xy[0], min_xy[1], height_m + 0.16],
            [max_xy[0], min_xy[1], height_m + 0.16],
            [max_xy[0], max_xy[1], height_m + 0.16],
            [min_xy[0], max_xy[1], height_m + 0.16],
        ],
        normal_enu: [0.0, 0.0, 1.0],
        intensity_norm: roof_norm,
        ottv_w_m2: rttv_roof_w_m2,
        color_rgba: thermal_heatmap_rgba(roof_norm),
    });

    // Analytical cast ground shadow footprint opposite the horizontal sun vector
    let mut shadow_quads_3d = Vec::new();
    if cfg.show_shadow_polygon && sun_alt_deg > 2.5 {
        let tan_alt = sun_alt_deg.to_radians().tan().max(0.18);
        let raw_len = (height_m / tan_alt).clamp(2.0, 85.0);
        let h_mag = sun_dir_enu[0].hypot(sun_dir_enu[1]).max(1e-4);
        let sx = -(sun_dir_enu[0] / h_mag) * raw_len;
        let sy = -(sun_dir_enu[1] / h_mag) * raw_len;
        let sz = 0.08;

        // Cast ground convex quad projecting building bounds along (sx, sy)
        let c0 = if sx >= 0.0 { min_xy[0] } else { max_xy[0] };
        let c1 = if sy >= 0.0 { min_xy[1] } else { max_xy[1] };
        let p_base_a = [c0, if sy >= 0.0 { max_xy[1] } else { min_xy[1] }, sz];
        let p_base_b = [if sx >= 0.0 { max_xy[0] } else { min_xy[0] }, c1, sz];
        let p_tip_a = [p_base_a[0] + sx, p_base_a[1] + sy, sz];
        let p_tip_c = [
            (if sx >= 0.0 { max_xy[0] } else { min_xy[0] }) + sx,
            (if sy >= 0.0 { max_xy[1] } else { min_xy[1] }) + sy,
            sz,
        ];
        let p_tip_b = [p_base_b[0] + sx, p_base_b[1] + sy, sz];
        // Clean shadow hull quad (uses p_tip_c anchor when diagonal, else clean projected box)
        let _ = p_tip_c;
        shadow_quads_3d.push(SolarShadowQuad3D {
            corners_enu_m: [p_base_a, p_base_b, p_tip_b, p_tip_a],
            shadow_length_m: raw_len,
        });
    }

    let sni_6389_compliant = overall_ottv_w_m2 <= cfg.ottv_limit_w_m2;
    let status_lbl = if sni_6389_compliant { "PASS" } else { "EXCEED" };
    let summary_verdict = format!(
        "OTTV {:.1} / {:.0} W/m² [{status_lbl}] • RTTV {:.1} • Cool {:.0} kWh/m²/yr • Peak {:.0} kW (Worst: {worst_orientation})",
        overall_ottv_w_m2,
        cfg.ottv_limit_w_m2,
        rttv_roof_w_m2,
        annual_cooling_kwh_m2,
        total_peak_kw,
    );

    SolarEnvelopeReport {
        config: cfg,
        site_label,
        lat_deg,
        building_width_m: width_m,
        building_depth_m: depth_m,
        building_height_m: height_m,
        stories: stories.max(1),
        min_enu_xy_m: min_xy,
        max_enu_xy_m: max_xy,
        sun_altitude_deg: sun_alt_deg,
        sun_azimuth_deg: sun_az_deg,
        sun_dir_enu,
        overall_ottv_w_m2,
        rttv_roof_w_m2,
        annual_cooling_kwh_m2,
        peak_solar_load_kw: total_peak_kw,
        total_facade_area_m2,
        worst_orientation,
        sni_6389_compliant,
        orientations,
        heatmap_panels_3d,
        shadow_quads_3d,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn glazing_and_shading_reduce_ottv_monotonically() {
        let bim = BimProject::empty();
        let site = SiteEnvironmentalCoupling::default();

        let mut cfg_clear = SolarEnvelopeConfig::default();
        cfg_clear.apply_glazing(GlazingPerformancePreset::ClearSingle);
        cfg_clear.shading_strategy = SolarShadingStrategy::None;
        let rep_clear = evaluate_solar_envelope(&bim, Some(&site), None, &cfg_clear);

        let mut cfg_low_e = cfg_clear.clone();
        cfg_low_e.apply_glazing(GlazingPerformancePreset::LowESpectrallySelective);
        let rep_low_e = evaluate_solar_envelope(&bim, Some(&site), None, &cfg_low_e);

        let mut cfg_louver = cfg_low_e.clone();
        cfg_louver.shading_strategy = SolarShadingStrategy::EggCrateLouvers;
        let rep_louver = evaluate_solar_envelope(&bim, Some(&site), None, &cfg_louver);

        assert!(rep_clear.overall_ottv_w_m2 > rep_low_e.overall_ottv_w_m2);
        assert!(rep_low_e.overall_ottv_w_m2 > rep_louver.overall_ottv_w_m2);
        assert!(rep_louver.sni_6389_compliant);
    }

    #[test]
    fn generates_3d_heatmap_panels_cast_shadows_and_ifcx_overlay() {
        let bim = BimProject::empty();
        let rep = evaluate_solar_envelope(&bim, None, None, &SolarEnvelopeConfig::default());
        assert_eq!(rep.orientations.len(), 5);
        assert_eq!(rep.heatmap_panels_3d.len(), rep.stories * 4 + 1);
        assert!(!rep.shadow_quads_3d.is_empty());

        let base = IfcxDocument::from_bim_project(&bim, None, 0.0, None, None, None);
        let composed = IfcxStage::compose_with_solar_ottv_overlay(&base, &rep.to_ifcx_overlay());
        assert!(composed
            .data
            .iter()
            .any(|n| n.attributes.contains_key(NS_EG_SOLAR_OTTV)));
    }
}
