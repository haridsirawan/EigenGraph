//! core/zoning_agent.rs — Update `v0.8.5.6` (`6/10` -> `v0.8.6`):
//! Site Zoning & 3D Buildable Envelope Engine (`KDB / BCR`, `KLB / FAR`, `GSB / Setbacks`,
//! `KDH / Green Open Space`, Height Limits & Analytical Solar Path).
//!
//! Couples live `BimProject` footprint/levels and `SiteEnvironmentalCoupling` latitude
//! into a 3D ENU buildable envelope (lot boundary, podium envelope, stepped-back tower envelope),
//! an analytical Sun Path trajectory, compliance checks, and non-destructive
//! `eigengraph::design::zoning_envelope` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::bim::BimProject;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;

/// Canonical EigenGraph IFCX attribute namespace for Site Zoning & Buildable Envelope overlays.
pub const NS_EG_ZONING_ENVELOPE: &str = "eigengraph::design::zoning_envelope";

/// Urban zoning district archetypes defining KDB (BCR), KLB (FAR), KDH (green), setbacks, and height limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoningDistrictPreset {
    CommercialCbd,
    MixedUseUrban,
    ResidentialHighDensity,
    MidRiseInstitutional,
    LowRiseSuburban,
}

impl ZoningDistrictPreset {
    pub const ALL: [Self; 5] = [
        Self::CommercialCbd,
        Self::MixedUseUrban,
        Self::ResidentialHighDensity,
        Self::MidRiseInstitutional,
        Self::LowRiseSuburban,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::CommercialCbd => "Commercial CBD (High FAR)",
            Self::MixedUseUrban => "Mixed-Use Urban Corridor",
            Self::ResidentialHighDensity => "Residential High-Density",
            Self::MidRiseInstitutional => "Mid-Rise Institutional",
            Self::LowRiseSuburban => "Low-Rise Suburban",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::CommercialCbd => "CBD",
            Self::MixedUseUrban => "Mixed",
            Self::ResidentialHighDensity => "Res-HD",
            Self::MidRiseInstitutional => "Instit",
            Self::LowRiseSuburban => "Suburb",
        }
    }

    /// Returns `(kdb_bcr_pct, klb_far, kdh_green_pct, front_m, side_m, rear_m, max_height_m, podium_stories)`.
    pub fn default_params(self) -> (f64, f64, f64, f64, f64, f64, f64, usize) {
        match self {
            Self::CommercialCbd => (65.0, 8.0, 15.0, 8.0, 4.0, 4.0, 120.0, 3),
            Self::MixedUseUrban => (60.0, 4.5, 20.0, 6.0, 4.0, 4.0, 54.0, 2),
            Self::ResidentialHighDensity => (50.0, 4.0, 25.0, 6.0, 4.5, 5.0, 48.0, 2),
            Self::MidRiseInstitutional => (45.0, 2.4, 30.0, 8.0, 5.0, 5.0, 28.0, 1),
            Self::LowRiseSuburban => (40.0, 1.2, 35.0, 5.0, 3.0, 3.0, 14.4, 1),
        }
    }
}

/// Persisted configuration for Site Zoning & 3D Buildable Envelope evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteZoningConfig {
    pub preset: ZoningDistrictPreset,
    pub lot_width_m: f64,
    pub lot_depth_m: f64,
    pub kdb_bcr_pct: f64,
    pub klb_far: f64,
    pub kdh_green_pct: f64,
    pub setback_front_m: f64,
    pub setback_side_m: f64,
    pub setback_rear_m: f64,
    pub max_height_m: f64,
    pub typical_story_height_m: f64,
    pub podium_stories: usize,
    pub tower_stepback_m: f64,
    pub solar_month: u8,
    pub solar_hour_local: f64,
    pub show_sun_path: bool,
}

impl Default for SiteZoningConfig {
    fn default() -> Self {
        let preset = ZoningDistrictPreset::MixedUseUrban;
        let (kdb, klb, kdh, front, side, rear, max_h, pod) = preset.default_params();
        Self {
            preset,
            lot_width_m: 48.0,
            lot_depth_m: 36.0,
            kdb_bcr_pct: kdb,
            klb_far: klb,
            kdh_green_pct: kdh,
            setback_front_m: front,
            setback_side_m: side,
            setback_rear_m: rear,
            max_height_m: max_h,
            typical_story_height_m: 3.6,
            podium_stories: pod,
            tower_stepback_m: 4.0,
            solar_month: 6,
            solar_hour_local: 10.5,
            show_sun_path: true,
        }
    }
}

impl SiteZoningConfig {
    /// Applies a zoning district preset while keeping lot dimensions and solar inspection time.
    pub fn apply_preset(&mut self, preset: ZoningDistrictPreset) {
        let (kdb, klb, kdh, front, side, rear, max_h, pod) = preset.default_params();
        self.preset = preset;
        self.kdb_bcr_pct = kdb;
        self.klb_far = klb;
        self.kdh_green_pct = kdh;
        self.setback_front_m = front;
        self.setback_side_m = side;
        self.setback_rear_m = rear;
        self.max_height_m = max_h;
        self.podium_stories = pod;
    }
}

/// Sampled 3D solar position along the site's diurnal Sun Path arc in local ENU meters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SunPositionPoint3D {
    pub hour_local: f64,
    pub altitude_deg: f64,
    pub azimuth_deg: f64,
    pub enu_pos_m: [f64; 3],
}

/// 3D ENU prism representing lot pad, podium buildable volume, tower envelope, or violation volume.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoningTierPrism3D {
    pub label: String,
    pub min_enu_m: [f64; 3],
    pub max_enu_m: [f64; 3],
    /// `0` = lot boundary pad, `1` = podium envelope, `2` = tower envelope, `3` = violation warning.
    pub tier_kind: u8,
}

/// Single deterministic regulatory clause audit comparing actual BIM metrics against zoning limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoningComplianceItem {
    pub code: String,
    pub actual_val: f64,
    pub limit_val: f64,
    pub unit: String,
    pub compliant: bool,
    pub detail: String,
}

/// Compact serializable IFCX overlay payload under `eigengraph::design::zoning_envelope`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoningIfcxOverlay {
    pub schema_version: String,
    pub preset: String,
    pub site_label: String,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub lot_area_m2: f64,
    pub kdb_bcr_pct: f64,
    pub actual_kdb_pct: f64,
    pub klb_far: f64,
    pub actual_klb_far: f64,
    pub kdh_green_pct: f64,
    pub actual_kdh_pct: f64,
    pub max_height_m: f64,
    pub existing_bim_height_m: f64,
    pub max_gfa_klb_m2: f64,
    pub existing_bim_gfa_m2: f64,
    pub remaining_gfa_capacity_m2: f64,
    pub max_allowable_stories: usize,
    pub active_sun_altitude_deg: f64,
    pub active_sun_azimuth_deg: f64,
    pub solar_exposure_score_pct: f64,
    pub overall_compliant: bool,
    pub summary_verdict: String,
}

/// Complete Site Zoning, Buildable Envelope & Solar Geometry Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteZoningReport {
    pub config: SiteZoningConfig,
    pub site_label: String,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub lot_area_m2: f64,
    pub buildable_pad_width_m: f64,
    pub buildable_pad_depth_m: f64,
    pub buildable_pad_area_m2: f64,
    pub max_footprint_kdb_m2: f64,
    pub effective_footprint_limit_m2: f64,
    pub max_gfa_klb_m2: f64,
    pub required_green_kdh_m2: f64,
    pub max_allowable_stories: usize,
    pub existing_bim_width_m: f64,
    pub existing_bim_depth_m: f64,
    pub existing_bim_footprint_m2: f64,
    pub existing_bim_height_m: f64,
    pub existing_bim_stories: usize,
    pub existing_bim_gfa_m2: f64,
    pub actual_kdb_pct: f64,
    pub actual_klb_far: f64,
    pub actual_kdh_pct: f64,
    pub remaining_gfa_capacity_m2: f64,
    pub active_sun_altitude_deg: f64,
    pub active_sun_azimuth_deg: f64,
    pub solar_exposure_score_pct: f64,
    pub shadow_length_factor: f64,
    pub envelope_prisms_3d: Vec<ZoningTierPrism3D>,
    pub sun_path_arc_3d: Vec<SunPositionPoint3D>,
    pub active_sun_pos_3d: [f64; 3],
    pub compliance_checks: Vec<ZoningComplianceItem>,
    pub overall_compliant: bool,
    pub summary_verdict: String,
}

impl SiteZoningReport {
    /// Converts the report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> ZoningIfcxOverlay {
        ZoningIfcxOverlay {
            schema_version: "zoning-envelope-ifcx-v1".to_string(),
            preset: self.config.preset.label().to_string(),
            site_label: self.site_label.clone(),
            lat_deg: self.lat_deg,
            lon_deg: self.lon_deg,
            lot_area_m2: self.lot_area_m2,
            kdb_bcr_pct: self.config.kdb_bcr_pct,
            actual_kdb_pct: self.actual_kdb_pct,
            klb_far: self.config.klb_far,
            actual_klb_far: self.actual_klb_far,
            kdh_green_pct: self.config.kdh_green_pct,
            actual_kdh_pct: self.actual_kdh_pct,
            max_height_m: self.config.max_height_m,
            existing_bim_height_m: self.existing_bim_height_m,
            max_gfa_klb_m2: self.max_gfa_klb_m2,
            existing_bim_gfa_m2: self.existing_bim_gfa_m2,
            remaining_gfa_capacity_m2: self.remaining_gfa_capacity_m2,
            max_allowable_stories: self.max_allowable_stories,
            active_sun_altitude_deg: self.active_sun_altitude_deg,
            active_sun_azimuth_deg: self.active_sun_azimuth_deg,
            solar_exposure_score_pct: self.solar_exposure_score_pct,
            overall_compliant: self.overall_compliant,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

fn extract_bim_envelope_metrics(project: &BimProject) -> (f64, f64, f64, usize) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for col in &project.columns {
        let x = col.pos.0 * 0.001;
        let y = col.pos.1 * 0.001;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    for wall in &project.walls {
        for p in [wall.start, wall.end] {
            let x = p.0 * 0.001;
            let y = p.1 * 0.001;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    for run in &project.runs {
        for p in [run.start, run.end] {
            let x = p.0 * 0.001;
            let y = p.1 * 0.001;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    for slab in &project.slabs {
        for p in &slab.outline {
            let x = p.0 * 0.001;
            let y = p.1 * 0.001;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }

    let (w, d) = if min_x.is_finite() && min_y.is_finite() {
        ((max_x - min_x).max(6.0), (max_y - min_y).max(6.0))
    } else {
        (0.0, 0.0)
    };

    let mut elevs: Vec<f64> = project
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .collect();
    elevs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    elevs.dedup_by(|a, b| (*a - *b).abs() < 0.25);

    let (height_m, stories) = if w <= 0.0 {
        (0.0, 0)
    } else if elevs.len() >= 2 {
        let top = elevs.last().copied().unwrap_or(3.6);
        let base = elevs.first().copied().unwrap_or(0.0);
        ((top - base).max(3.2), elevs.len().saturating_sub(1).max(1))
    } else {
        (3.6, 1)
    };

    (w, d, height_m, stories)
}

/// Computes analytical solar altitude (deg above horizon), azimuth (deg clockwise from North),
/// and 3D ENU position on a sky arc of `radius_m` centered at site origin.
fn compute_sun_point(
    lat_deg: f64,
    month: u8,
    hour_local: f64,
    radius_m: f64,
) -> SunPositionPoint3D {
    let day_of_year = ((month.clamp(1, 12) as f64) - 1.0) * 30.4 + 15.0;
    let decl_rad = (23.44_f64.to_radians())
        * (2.0 * std::f64::consts::PI * (284.0 + day_of_year) / 365.0).sin();
    let lat_rad = lat_deg.clamp(-85.0, 85.0).to_radians();
    let hour_angle_rad = ((hour_local - 12.0) * 15.0).to_radians();

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
    let horiz = radius_m * alt_rad.max(0.05).cos();
    let east_m = horiz * az_rad.sin();
    let north_m = horiz * az_rad.cos();
    let up_m = (radius_m * alt_rad.max(0.05).sin()).max(2.5);

    SunPositionPoint3D {
        hour_local,
        altitude_deg: alt_deg,
        azimuth_deg: az_deg,
        enu_pos_m: [east_m, north_m, up_m],
    }
}

/// Evaluates site zoning regulations (`KDB`, `KLB`, `KDH`, `GSB`, Height Cap), generates
/// 3D ENU buildable envelope prisms + analytical Sun Path, and compares existing `BimProject`.
pub fn evaluate_site_zoning(
    project: &BimProject,
    site: Option<&SiteEnvironmentalCoupling>,
    config: &SiteZoningConfig,
) -> SiteZoningReport {
    let mut cfg = config.clone();
    cfg.lot_width_m = cfg.lot_width_m.clamp(10.0, 400.0);
    cfg.lot_depth_m = cfg.lot_depth_m.clamp(10.0, 400.0);
    cfg.kdb_bcr_pct = cfg.kdb_bcr_pct.clamp(10.0, 90.0);
    cfg.klb_far = cfg.klb_far.clamp(0.4, 24.0);
    cfg.kdh_green_pct = cfg.kdh_green_pct.clamp(5.0, 70.0);
    cfg.setback_front_m = cfg.setback_front_m.clamp(0.0, 30.0);
    cfg.setback_side_m = cfg.setback_side_m.clamp(0.0, 25.0);
    cfg.setback_rear_m = cfg.setback_rear_m.clamp(0.0, 25.0);
    cfg.max_height_m = cfg.max_height_m.clamp(4.0, 350.0);
    cfg.typical_story_height_m = cfg.typical_story_height_m.clamp(2.8, 6.0);
    cfg.podium_stories = cfg.podium_stories.clamp(0, 12);
    cfg.tower_stepback_m = cfg.tower_stepback_m.clamp(0.0, 16.0);
    cfg.solar_month = cfg.solar_month.clamp(1, 12);
    cfg.solar_hour_local = cfg.solar_hour_local.clamp(6.0, 18.0);

    let (site_label, lat_deg, lon_deg) = site
        .map(|s| (s.site_label.clone(), s.lat_deg, s.lon_deg))
        .unwrap_or_else(|| ("-6.175°, 106.827°".to_string(), -6.1754, 106.8272));

    let lot_area_m2 = cfg.lot_width_m * cfg.lot_depth_m;
    let buildable_pad_width_m = (cfg.lot_width_m - 2.0 * cfg.setback_side_m).max(4.0);
    let buildable_pad_depth_m =
        (cfg.lot_depth_m - cfg.setback_front_m - cfg.setback_rear_m).max(4.0);
    let buildable_pad_area_m2 = buildable_pad_width_m * buildable_pad_depth_m;

    let max_footprint_kdb_m2 = lot_area_m2 * (cfg.kdb_bcr_pct / 100.0);
    let effective_footprint_limit_m2 = max_footprint_kdb_m2.min(buildable_pad_area_m2);
    let max_gfa_klb_m2 = lot_area_m2 * cfg.klb_far;
    let required_green_kdh_m2 = lot_area_m2 * (cfg.kdh_green_pct / 100.0);
    let max_allowable_stories =
        (cfg.max_height_m / cfg.typical_story_height_m).floor().max(1.0) as usize;

    let (existing_bim_width_m, existing_bim_depth_m, existing_bim_height_m, existing_bim_stories) =
        extract_bim_envelope_metrics(project);
    let existing_bim_footprint_m2 = existing_bim_width_m * existing_bim_depth_m;
    let existing_bim_gfa_m2 = existing_bim_footprint_m2 * (existing_bim_stories as f64);

    let actual_kdb_pct = if lot_area_m2 > 0.0 {
        (existing_bim_footprint_m2 / lot_area_m2) * 100.0
    } else {
        0.0
    };
    let actual_klb_far = if lot_area_m2 > 0.0 {
        existing_bim_gfa_m2 / lot_area_m2
    } else {
        0.0
    };
    let actual_kdh_pct = (100.0 - actual_kdb_pct).clamp(0.0, 100.0);
    let remaining_gfa_capacity_m2 = (max_gfa_klb_m2 - existing_bim_gfa_m2).max(0.0);

    // Compliance checks (when no BIM exists yet, actual = 0 -> compliant empty lot ready for design)
    let kdb_ok = actual_kdb_pct <= cfg.kdb_bcr_pct + 0.05;
    let klb_ok = actual_klb_far <= cfg.klb_far + 0.01;
    let height_ok = existing_bim_height_m <= cfg.max_height_m + 0.05;
    let setback_ok = existing_bim_width_m <= buildable_pad_width_m + 0.05
        && existing_bim_depth_m <= buildable_pad_depth_m + 0.05;
    let kdh_ok = actual_kdh_pct + 0.05 >= cfg.kdh_green_pct;

    let compliance_checks = vec![
        ZoningComplianceItem {
            code: "KDB / BCR".to_string(),
            actual_val: actual_kdb_pct,
            limit_val: cfg.kdb_bcr_pct,
            unit: "%".to_string(),
            compliant: kdb_ok,
            detail: format!(
                "Footprint {:.0} / {:.0} m2",
                existing_bim_footprint_m2, max_footprint_kdb_m2
            ),
        },
        ZoningComplianceItem {
            code: "KLB / FAR".to_string(),
            actual_val: actual_klb_far,
            limit_val: cfg.klb_far,
            unit: "x".to_string(),
            compliant: klb_ok,
            detail: format!("GFA {:.0} / {:.0} m2", existing_bim_gfa_m2, max_gfa_klb_m2),
        },
        ZoningComplianceItem {
            code: "Height Cap".to_string(),
            actual_val: existing_bim_height_m,
            limit_val: cfg.max_height_m,
            unit: "m".to_string(),
            compliant: height_ok,
            detail: format!(
                "{} / {} stories",
                existing_bim_stories, max_allowable_stories
            ),
        },
        ZoningComplianceItem {
            code: "GSB / Setback".to_string(),
            actual_val: existing_bim_footprint_m2,
            limit_val: buildable_pad_area_m2,
            unit: "m2".to_string(),
            compliant: setback_ok,
            detail: format!(
                "Pad {:.1}x{:.1}m (F{:.0}/S{:.0}/R{:.0})",
                buildable_pad_width_m,
                buildable_pad_depth_m,
                cfg.setback_front_m,
                cfg.setback_side_m,
                cfg.setback_rear_m
            ),
        },
        ZoningComplianceItem {
            code: "KDH / Green".to_string(),
            actual_val: actual_kdh_pct,
            limit_val: cfg.kdh_green_pct,
            unit: "%".to_string(),
            compliant: kdh_ok,
            detail: format!("Min Open Space {:.0} m2", required_green_kdh_m2),
        },
    ];
    let overall_compliant = compliance_checks.iter().all(|c| c.compliant);

    // Center the buildable pad around site ENU origin; front road faces -Y (South)
    let half_lot_w = cfg.lot_width_m * 0.5;
    let half_lot_d = cfg.lot_depth_m * 0.5;
    let pad_y_shift = (cfg.setback_front_m - cfg.setback_rear_m) * 0.5;

    // Fit podium footprint inside both setback pad and KDB area limit
    let kdb_scale = if buildable_pad_area_m2 > max_footprint_kdb_m2 {
        (max_footprint_kdb_m2 / buildable_pad_area_m2).sqrt()
    } else {
        1.0
    };
    let pod_w = (buildable_pad_width_m * kdb_scale).max(4.0);
    let pod_d = (buildable_pad_depth_m * kdb_scale).max(4.0);
    let podium_h = ((cfg.podium_stories as f64) * cfg.typical_story_height_m)
        .min(cfg.max_height_m * 0.65);

    let mut envelope_prisms_3d = vec![ZoningTierPrism3D {
        label: "Lot Boundary".to_string(),
        min_enu_m: [-half_lot_w, -half_lot_d, 0.0],
        max_enu_m: [half_lot_w, half_lot_d, 0.18],
        tier_kind: 0,
    }];

    if podium_h > 0.5 {
        envelope_prisms_3d.push(ZoningTierPrism3D {
            label: "Podium Envelope".to_string(),
            min_enu_m: [-pod_w * 0.5, pad_y_shift - pod_d * 0.5, 0.0],
            max_enu_m: [pod_w * 0.5, pad_y_shift + pod_d * 0.5, podium_h],
            tier_kind: 1,
        });
    }

    if cfg.max_height_m > podium_h + 0.5 {
        let twr_w = (pod_w - 2.0 * cfg.tower_stepback_m).max(6.0);
        let twr_d = (pod_d - 2.0 * cfg.tower_stepback_m).max(6.0);
        envelope_prisms_3d.push(ZoningTierPrism3D {
            label: "Tower Envelope".to_string(),
            min_enu_m: [-twr_w * 0.5, pad_y_shift - twr_d * 0.5, podium_h],
            max_enu_m: [twr_w * 0.5, pad_y_shift + twr_d * 0.5, cfg.max_height_m],
            tier_kind: if overall_compliant { 2 } else { 3 },
        });
    }

    // Analytical Sun Path arc (06:00 to 18:00)
    let sky_radius =
        (cfg.lot_width_m.max(cfg.lot_depth_m) * 0.85 + cfg.max_height_m * 0.55).clamp(28.0, 120.0);
    let mut sun_path_arc_3d = Vec::with_capacity(13);
    let mut daylight_alt_sum = 0.0;
    let mut daylight_cnt = 0usize;
    for h in 6..=18 {
        let pt = compute_sun_point(lat_deg, cfg.solar_month, h as f64, sky_radius);
        if pt.altitude_deg > 5.0 {
            daylight_alt_sum += pt.altitude_deg;
            daylight_cnt += 1;
        }
        sun_path_arc_3d.push(pt);
    }
    let active_sun = compute_sun_point(lat_deg, cfg.solar_month, cfg.solar_hour_local, sky_radius);
    let mean_alt = if daylight_cnt > 0 {
        daylight_alt_sum / (daylight_cnt as f64)
    } else {
        35.0
    };
    let solar_exposure_score_pct = ((mean_alt / 65.0) * 100.0).clamp(35.0, 98.0);
    let shadow_length_factor = if active_sun.altitude_deg > 3.0 {
        (1.0 / active_sun.altitude_deg.to_radians().tan()).clamp(0.1, 12.0)
    } else {
        12.0
    };

    let status_tag = if overall_compliant {
        "COMPLIANT"
    } else {
        "VIOLATION"
    };
    let summary_verdict = format!(
        "Zoning {status_tag} ({}) • Max GFA {:.0} m2 (KLB {:.1}) • Pad {:.0} m2 • {} Stories Max • Sun {:.0}°/{:.0}°",
        cfg.preset.short_label(),
        max_gfa_klb_m2,
        cfg.klb_far,
        buildable_pad_area_m2,
        max_allowable_stories,
        active_sun.altitude_deg,
        active_sun.azimuth_deg
    );

    SiteZoningReport {
        config: cfg,
        site_label,
        lat_deg,
        lon_deg,
        lot_area_m2,
        buildable_pad_width_m,
        buildable_pad_depth_m,
        buildable_pad_area_m2,
        max_footprint_kdb_m2,
        effective_footprint_limit_m2,
        max_gfa_klb_m2,
        required_green_kdh_m2,
        max_allowable_stories,
        existing_bim_width_m,
        existing_bim_depth_m,
        existing_bim_footprint_m2,
        existing_bim_height_m,
        existing_bim_stories,
        existing_bim_gfa_m2,
        actual_kdb_pct,
        actual_klb_far,
        actual_kdh_pct,
        remaining_gfa_capacity_m2,
        active_sun_altitude_deg: active_sun.altitude_deg,
        active_sun_azimuth_deg: active_sun.azimuth_deg,
        solar_exposure_score_pct,
        shadow_length_factor,
        envelope_prisms_3d,
        sun_path_arc_3d,
        active_sun_pos_3d: active_sun.enu_pos_m,
        compliance_checks,
        overall_compliant,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::{BimColumn, BimLevel};
    use crate::core::geom::P2;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_site_zoning_kdb_klb_setback_and_solar_math() {
        let project = BimProject::default();
        let cfg = SiteZoningConfig::default();
        let rep = evaluate_site_zoning(&project, None, &cfg);

        assert!((rep.lot_area_m2 - 1728.0).abs() < 1e-6);
        assert!((rep.buildable_pad_width_m - 40.0).abs() < 1e-6);
        assert!((rep.buildable_pad_depth_m - 26.0).abs() < 1e-6);
        assert!((rep.max_footprint_kdb_m2 - 1036.8).abs() < 1e-3);
        assert!((rep.max_gfa_klb_m2 - 7776.0).abs() < 1e-3);
        assert_eq!(rep.max_allowable_stories, 15);
        assert!(rep.overall_compliant);
        assert_eq!(rep.sun_path_arc_3d.len(), 13);
        assert!(rep.active_sun_altitude_deg > 10.0 && rep.active_sun_altitude_deg < 90.0);
        assert!(rep.envelope_prisms_3d.len() >= 3);
    }

    #[test]
    fn test_site_zoning_detects_height_violation_and_ifcx_overlay() {
        let mut project = BimProject::default();
        project.levels = vec![
            BimLevel {
                id: 1,
                name: "L0".to_string(),
                elevation_mm: 0.0,
            },
            BimLevel {
                id: 2,
                name: "Roof".to_string(),
                elevation_mm: 32_000.0,
            },
        ];
        project.columns = vec![
            BimColumn {
                id: 1,
                name: "C1".to_string(),
                base_level_id: 1,
                top_level_id: 2,
                pos: P2(0.0, 0.0),
                width_mm: 500.0,
                depth_mm: 500.0,
                is_round: false,
                profile_name: "C500".to_string(),
                rebars: vec![],
            },
            BimColumn {
                id: 2,
                name: "C2".to_string(),
                base_level_id: 1,
                top_level_id: 2,
                pos: P2(18_000.0, 14_000.0),
                width_mm: 500.0,
                depth_mm: 500.0,
                is_round: false,
                profile_name: "C500".to_string(),
                rebars: vec![],
            },
        ];

        let mut cfg = SiteZoningConfig::default();
        cfg.apply_preset(ZoningDistrictPreset::LowRiseSuburban);
        let rep = evaluate_site_zoning(&project, None, &cfg);

        assert!(!rep.overall_compliant);
        assert!(rep.summary_verdict.contains("VIOLATION"));
        assert!(rep.envelope_prisms_3d.iter().any(|p| p.tier_kind == 3));

        let base = IfcxDocument::new("zoning_test");
        let composed = IfcxStage::compose_with_zoning_envelope_overlay(&base, &rep.to_ifcx_overlay());
        let node = composed
            .data
            .iter()
            .find(|n| n.path == "/earth/design/zoning_envelope")
            .expect("zoning overlay node");
        assert!(node.attributes.contains_key(NS_EG_ZONING_ENVELOPE));
    }
}
