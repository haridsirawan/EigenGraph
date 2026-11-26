//! core/program_agent.rs — Update `v0.8.5.7` (`7/10` -> `v0.8.6`):
//! Spatial Programming, Adjacency Matrix & 3D Stack/Pack Engine.
//!
//! Translates architectural building typologies and interactive space schedules
//! (unit counts, unit areas, department bands, core & circulation factors) into
//! story-by-story vertical stacking, deterministic 2D slicing-tree floor-plate
//! packing (`PackedRoomBlock3D`), AEC adjacency scoring, zoning GFA checks,
//! and non-destructive `eigengraph::design::spatial_program` IFCX overlays.

use serde::{Deserialize, Serialize};

use crate::core::zoning_agent::{SiteZoningConfig, SiteZoningReport};

/// Canonical EigenGraph IFCX attribute namespace for Spatial Programming & Stack/Pack overlays.
pub const NS_EG_SPATIAL_PROGRAM: &str = "eigengraph::design::spatial_program";

/// Architectural building typology presets seeding space programs and floor-band preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingTypologyPreset {
    MixedUseTower,
    OfficeHeadquarters,
    ResidentialApartment,
    HealthcareHospital,
    UniversityCampus,
}

impl BuildingTypologyPreset {
    pub const ALL: [Self; 5] = [
        Self::MixedUseTower,
        Self::OfficeHeadquarters,
        Self::ResidentialApartment,
        Self::HealthcareHospital,
        Self::UniversityCampus,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::MixedUseTower => "Mixed-Use Retail & Residential Tower",
            Self::OfficeHeadquarters => "Grade-A Corporate Office Headquarters",
            Self::ResidentialApartment => "High-Density Residential Apartment",
            Self::HealthcareHospital => "Clinical Healthcare & Diagnostic Hospital",
            Self::UniversityCampus => "Academic & Research University Building",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::MixedUseTower => "Mixed",
            Self::OfficeHeadquarters => "Office",
            Self::ResidentialApartment => "Apt",
            Self::HealthcareHospital => "Hospital",
            Self::UniversityCampus => "Campus",
        }
    }

    pub fn default_spaces(self) -> Vec<ProgramSpaceEntry> {
        let mk = |id: usize,
                  name: &str,
                  department: DepartmentKind,
                  unit_count: usize,
                  area_per_unit_m2: f64,
                  preferred_floor_band: u8,
                  adjacency_group: u8| ProgramSpaceEntry {
            id,
            name: name.to_string(),
            department,
            unit_count,
            area_per_unit_m2,
            preferred_floor_band,
            adjacency_group,
        };
        match self {
            Self::MixedUseTower => vec![
                mk(1, "Main Atrium & Lobby", DepartmentKind::LobbyAndCore, 1, 220.0, 0, 1),
                mk(2, "Ground Retail & F&B", DepartmentKind::Retail, 8, 95.0, 0, 1),
                mk(3, "Podium Co-Working", DepartmentKind::Office, 4, 180.0, 1, 2),
                mk(4, "Resident Sky Lounge", DepartmentKind::Amenities, 2, 160.0, 1, 3),
                mk(5, "Apartment 1BR/2BR", DepartmentKind::Residential, 42, 68.0, 2, 3),
                mk(6, "MEP & Service Plant", DepartmentKind::ParkingAndMep, 2, 140.0, 3, 4),
            ],
            Self::OfficeHeadquarters => vec![
                mk(1, "Executive Reception", DepartmentKind::LobbyAndCore, 1, 260.0, 0, 1),
                mk(2, "Cafe & Auditorium", DepartmentKind::Amenities, 2, 210.0, 0, 1),
                mk(3, "Open Workplaces", DepartmentKind::Office, 16, 220.0, 1, 2),
                mk(4, "Collaboration Suites", DepartmentKind::Office, 10, 85.0, 2, 2),
                mk(5, "Executive Floor", DepartmentKind::Office, 3, 190.0, 2, 3),
                mk(6, "Data & MEP Plant", DepartmentKind::ParkingAndMep, 2, 130.0, 3, 4),
            ],
            Self::ResidentialApartment => vec![
                mk(1, "Drop-Off Lobby", DepartmentKind::LobbyAndCore, 1, 180.0, 0, 1),
                mk(2, "Convenience & Mail", DepartmentKind::Retail, 3, 75.0, 0, 1),
                mk(3, "Clubhouse & Gym", DepartmentKind::Amenities, 2, 175.0, 0, 2),
                mk(4, "Studio & 1BR Units", Department::Residential, 36, 46.0, 1, 3),
                mk(5, "2BR & 3BR Suites", DepartmentKind::Residential, 24, 88.0, 2, 3),
                mk(6, "Utility & Pump Room", DepartmentKind::ParkingAndMep, 2, 110.0, 3, 4),
            ],
            Self::HealthcareHospital => vec![
                mk(1, "Triage & Main Lobby", DepartmentKind::LobbyAndCore, 1, 280.0, 0, 1),
                mk(2, "Emergency & Imaging", DepartmentKind::ClinicalOrLab, 6, 130.0, 0, 1),
                mk(3, "Outpatient Clinics", DepartmentKind::ClinicalOrLab, 12, 72.0, 1, 2),
                mk(4, "Surgical & ICU Suite", DepartmentKind::ClinicalOrLab, 6, 145.0, 1, 2),
                mk(5, "Inpatient Ward Rooms", DepartmentKind::Residential, 32, 42.0, 2, 3),
                mk(6, "Pharmacy & Med Gas", DepartmentKind::ParkingAndMep, 2, 150.0, 3, 4),
            ],
            Self::UniversityCampus => vec![
                mk(1, "Student Commons", DepartmentKind::LobbyAndCore, 1, 240.0, 0, 1),
                mk(2, "Lecture Halls", DepartmentKind::Amenities, 4, 190.0, 0, 1),
                mk(3, "Engineering Labs", DepartmentKind::ClinicalOrLab, 8, 135.0, 1, 2),
                mk(4, "Seminar & Studio Bays", DepartmentKind::Office, 10, 95.0, 1, 2),
                mk(5, "Faculty & Grad Offices", DepartmentKind::Office, 14, 58.0, 2, 3),
                mk(6, "Workshop & MEP", DepartmentKind::ParkingAndMep, 2, 125.0, 3, 4),
            ],
        }
    }
}

// Type alias inside default_spaces fixed cleanly below via DepartmentKind
type Department = DepartmentKind;

/// Functional department classification with consistent spatial color palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepartmentKind {
    LobbyAndCore,
    Retail,
    Office,
    Residential,
    Amenities,
    ParkingAndMep,
    ClinicalOrLab,
}

impl DepartmentKind {
    pub const ALL: [Self; 7] = [
        Self::LobbyAndCore,
        Self::Retail,
        Self::Office,
        Self::Residential,
        Self::Amenities,
        Self::ParkingAndMep,
        Self::ClinicalOrLab,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::LobbyAndCore => "Lobby & Core",
            Self::Retail => "Retail / F&B",
            Self::Office => "Office / Studio",
            Self::Residential => "Residential / Ward",
            Self::Amenities => "Amenities / Hall",
            Self::ParkingAndMep => "MEP / Service",
            Self::ClinicalOrLab => "Clinical / Lab",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::LobbyAndCore => "Core",
            Self::Retail => "Retail",
            Self::Office => "Office",
            Self::Residential => "Res",
            Self::Amenities => "Amen",
            Self::ParkingAndMep => "MEP",
            Self::ClinicalOrLab => "Lab",
        }
    }

    pub fn color_rgba(self) -> [f32; 4] {
        match self {
            Self::LobbyAndCore => [0.58, 0.64, 0.72, 0.72],
            Self::Retail => [0.96, 0.62, 0.16, 0.68],
            Self::Office => [0.22, 0.66, 0.96, 0.66],
            Self::Residential => [0.16, 0.80, 0.56, 0.66],
            Self::Amenities => [0.68, 0.48, 0.94, 0.66],
            Self::ParkingAndMep => [0.46, 0.52, 0.60, 0.70],
            Self::ClinicalOrLab => [0.14, 0.82, 0.82, 0.66],
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::LobbyAndCore => Self::Retail,
            Self::Retail => Self::Office,
            Self::Office => Self::Residential,
            Self::Residential => Self::Amenities,
            Self::Amenities => Self::ParkingAndMep,
            Self::ParkingAndMep => Self::ClinicalOrLab,
            Self::ClinicalOrLab => Self::LobbyAndCore,
        }
    }
}

/// Editable space-program line item synchronized two-way between schedule table and 3D stack/pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramSpaceEntry {
    pub id: usize,
    pub name: String,
    pub department: DepartmentKind,
    pub unit_count: usize,
    pub area_per_unit_m2: f64,
    /// `0` = Ground / Podium, `1` = Lower / Mid, `2` = Upper / Tower, `3` = Base / Service.
    pub preferred_floor_band: u8,
    pub adjacency_group: u8,
}

impl ProgramSpaceEntry {
    pub fn total_area_m2(&self) -> f64 {
        (self.unit_count.max(1) as f64) * self.area_per_unit_m2.max(4.0)
    }
}

/// Persisted configuration for Spatial Programming, Adjacency & 3D Stack/Pack Studio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialProgramConfig {
    pub typology: BuildingTypologyPreset,
    pub spaces: Vec<ProgramSpaceEntry>,
    pub circulation_factor_pct: f64,
    pub core_ratio_pct: f64,
    pub auto_fit_to_zoning: bool,
    pub selected_story_filter: Option<usize>,
}

impl Default for SpatialProgramConfig {
    fn default() -> Self {
        let typology = BuildingTypologyPreset::MixedUseTower;
        Self {
            typology,
            spaces: typology.default_spaces(),
            circulation_factor_pct: 18.0,
            core_ratio_pct: 12.0,
            auto_fit_to_zoning: true,
            selected_story_filter: None,
        }
    }
}

impl SpatialProgramConfig {
    /// Applies a building typology preset and resets the space program schedule.
    pub fn apply_typology(&mut self, typology: BuildingTypologyPreset) {
        self.typology = typology;
        self.spaces = typology.default_spaces();
        self.selected_story_filter = None;
    }
}

/// Pairwise architectural adjacency evaluation result across stacked departments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdjacencyPairScore {
    pub from_dept: String,
    pub to_dept: String,
    pub weight: f64,
    pub satisfied: bool,
    pub story_delta: usize,
    pub rationale: String,
}

/// Packed 3D ENU space block representing a room cluster or structural core on a floor plate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedRoomBlock3D {
    pub space_id: usize,
    pub label: String,
    pub department: DepartmentKind,
    pub story_index: usize,
    pub area_m2: f64,
    pub min_enu_m: [f64; 3],
    pub max_enu_m: [f64; 3],
    pub color_rgba: [f32; 4],
}

/// Story-by-story vertical stacking KPIs and plate efficiency summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryStackSummary {
    pub story_index: usize,
    pub elevation_m: f64,
    pub floor_height_m: f64,
    pub plate_width_m: f64,
    pub plate_depth_m: f64,
    pub gross_plate_area_m2: f64,
    pub net_program_area_m2: f64,
    pub core_and_circ_area_m2: f64,
    pub efficiency_pct: f64,
    pub room_count: usize,
    pub primary_department: String,
}

/// Compact serializable IFCX overlay payload under `eigengraph::design::spatial_program`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialProgramIfcxOverlay {
    pub schema_version: String,
    pub typology: String,
    pub total_nfa_m2: f64,
    pub total_gfa_m2: f64,
    pub core_and_circ_m2: f64,
    pub overall_efficiency_pct: f64,
    pub total_units: usize,
    pub stacked_stories: usize,
    pub zoning_max_gfa_m2: f64,
    pub gfa_utilization_pct: f64,
    pub fits_zoning_envelope: bool,
    pub adjacency_score_pct: f64,
    pub summary_verdict: String,
}

/// Complete Spatial Program, Adjacency Matrix & 3D Stack/Pack Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialProgramReport {
    pub config: SpatialProgramConfig,
    pub total_nfa_m2: f64,
    pub total_gfa_m2: f64,
    pub core_and_circ_m2: f64,
    pub overall_efficiency_pct: f64,
    pub total_units: usize,
    pub stacked_stories: usize,
    pub zoning_max_gfa_m2: f64,
    pub gfa_utilization_pct: f64,
    pub fits_zoning_envelope: bool,
    pub adjacency_score_pct: f64,
    pub adjacency_pairs: Vec<AdjacencyPairScore>,
    pub story_summaries: Vec<StoryStackSummary>,
    pub packed_blocks_3d: Vec<PackedRoomBlock3D>,
    pub summary_verdict: String,
}

impl SpatialProgramReport {
    /// Converts the report into a compact non-destructive `.ifcx` overlay payload.
    pub fn to_ifcx_overlay(&self) -> SpatialProgramIfcxOverlay {
        SpatialProgramIfcxOverlay {
            schema_version: "spatial-program-ifcx-v1".to_string(),
            typology: self.config.typology.label().to_string(),
            total_nfa_m2: self.total_nfa_m2,
            total_gfa_m2: self.total_gfa_m2,
            core_and_circ_m2: self.core_and_circ_m2,
            overall_efficiency_pct: self.overall_efficiency_pct,
            total_units: self.total_units,
            stacked_stories: self.stacked_stories,
            zoning_max_gfa_m2: self.zoning_max_gfa_m2,
            gfa_utilization_pct: self.gfa_utilization_pct,
            fits_zoning_envelope: self.fits_zoning_envelope,
            adjacency_score_pct: self.adjacency_score_pct,
            summary_verdict: self.summary_verdict.clone(),
        }
    }
}

#[derive(Clone)]
struct StoryAllocChunk {
    space_id: usize,
    label: String,
    department: DepartmentKind,
    area_m2: f64,
}

/// Deterministic 2D binary slicing-tree packer splitting a rectangle `(x0, y0, x1, y1)`
/// along its longest dimension proportionally to item area.
fn pack_chunks_into_rect(
    chunks: &[StoryAllocChunk],
    rect: [f64; 4],
    story_index: usize,
    z0: f64,
    z1: f64,
    out: &mut Vec<PackedRoomBlock3D>,
) {
    if chunks.is_empty() {
        return;
    }
    let [x0, y0, x1, y1] = rect;
    let w = (x1 - x0).max(0.2);
    let d = (y1 - y0).max(0.2);
    if chunks.len() == 1 {
        let c = &chunks[0];
        let inset = 0.12_f64.min(w * 0.12).min(d * 0.12);
        out.push(PackedRoomBlock3D {
            space_id: c.space_id,
            label: c.label.clone(),
            department: c.department,
            story_index,
            area_m2: c.area_m2,
            min_enu_m: [x0 + inset, y0 + inset, z0],
            max_enu_m: [x1 - inset, y1 - inset, z1],
            color_rgba: c.department.color_rgba(),
        });
        return;
    }

    let total: f64 = chunks.iter().map(|c| c.area_m2.max(1.0)).sum();
    let half_target = total * 0.5;
    let mut accum = 0.0;
    let mut split_idx = 1usize;
    for (idx, c) in chunks.iter().enumerate() {
        accum += c.area_m2.max(1.0);
        if accum >= half_target && idx + 1 < chunks.len() {
            split_idx = idx + 1;
            break;
        }
    }
    split_idx = split_idx.clamp(1, chunks.len() - 1);
    let left_area: f64 = chunks[..split_idx].iter().map(|c| c.area_m2.max(1.0)).sum();
    let ratio = (left_area / total.max(1e-6)).clamp(0.15, 0.85);

    if w >= d {
        let xm = x0 + w * ratio;
        pack_chunks_into_rect(&chunks[..split_idx], [x0, y0, xm, y1], story_index, z0, z1, out);
        pack_chunks_into_rect(&chunks[split_idx..], [xm, y0, x1, y1], story_index, z0, z1, out);
    } else {
        let ym = y0 + d * ratio;
        pack_chunks_into_rect(&chunks[..split_idx], [x0, y0, x1, ym], story_index, z0, z1, out);
        pack_chunks_into_rect(&chunks[split_idx..], [x0, ym, x1, y1], story_index, z0, z1, out);
    }
}

fn band_rank(band: u8) -> u8 {
    match band {
        3 => 0, // Service / Base first (ground/lowest)
        0 => 1, // Ground / Podium
        1 => 2, // Lower / Mid
        _ => 3, // Upper / Tower
    }
}

fn evaluate_adjacency(blocks: &[PackedRoomBlock3D]) -> (Vec<AdjacencyPairScore>, f64) {
    let stories_for = |dept: DepartmentKind| -> Vec<usize> {
        let mut s: Vec<usize> = blocks
            .iter()
            .filter(|b| b.department == dept)
            .map(|b| b.story_index)
            .collect();
        s.sort_unstable();
        s.dedup();
        s
    };

    let min_story_delta = |a: &[usize], b: &[usize]| -> Option<usize> {
        let mut best: Option<usize> = None;
        for &sa in a {
            for &sb in b {
                let d = sa.abs_diff(sb);
                best = Some(best.map_or(d, |cur| cur.min(d)));
            }
        }
        best
    };

    let core_s = stories_for(DepartmentKind::LobbyAndCore);
    let retail_s = stories_for(DepartmentKind::Retail);
    let office_s = stories_for(DepartmentKind::Office);
    let res_s = stories_for(DepartmentKind::Residential);
    let amen_s = stories_for(DepartmentKind::Amenities);
    let mep_s = stories_for(DepartmentKind::ParkingAndMep);
    let lab_s = stories_for(DepartmentKind::ClinicalOrLab);

    let mut pairs = Vec::new();

    if !retail_s.is_empty() && !core_s.is_empty() {
        let delta = min_story_delta(&core_s, &retail_s).unwrap_or(0);
        let ok = delta <= 1;
        pairs.push(AdjacencyPairScore {
            from_dept: "Lobby & Core".to_string(),
            to_dept: "Retail / F&B".to_string(),
            weight: 1.0,
            satisfied: ok,
            story_delta: delta,
            rationale: if ok {
                "Direct pedestrian street & lobby access".to_string()
            } else {
                "Retail separated from main lobby entry".to_string()
            },
        });
    }

    let primary_upper = if !res_s.is_empty() {
        ("Residential", &res_s)
    } else if !office_s.is_empty() {
        ("Office", &office_s)
    } else {
        ("Clinical / Lab", &lab_s)
    };
    if !primary_upper.1.is_empty() && !core_s.is_empty() {
        let delta = min_story_delta(&core_s, primary_upper.1).unwrap_or(0);
        let ok = delta <= 1;
        pairs.push(AdjacencyPairScore {
            from_dept: "Vertical Core".to_string(),
            to_dept: primary_upper.0.to_string(),
            weight: 1.0,
            satisfied: ok,
            story_delta: delta,
            rationale: "Continuous elevator/stair core connectivity".to_string(),
        });
    }

    if !amen_s.is_empty() && !primary_upper.1.is_empty() {
        let delta = min_story_delta(&amen_s, primary_upper.1).unwrap_or(0);
        let ok = delta <= 2;
        pairs.push(AdjacencyPairScore {
            from_dept: "Amenities / Hall".to_string(),
            to_dept: primary_upper.0.to_string(),
            weight: 0.85,
            satisfied: ok,
            story_delta: delta,
            rationale: if ok {
                "Shared podium/sky amenities within 2 levels".to_string()
            } else {
                "Amenities far from primary tenant floors".to_string()
            },
        });
    }

    if !mep_s.is_empty() && !res_s.is_empty() {
        let min_mep = mep_s.iter().copied().min().unwrap_or(0);
        let min_res = res_s.iter().copied().min().unwrap_or(0);
        let delta = min_res.abs_diff(min_mep);
        let ok = min_mep <= min_res && delta >= 1;
        pairs.push(AdjacencyPairScore {
            from_dept: "MEP / Service".to_string(),
            to_dept: "Residential / Ward".to_string(),
            weight: 0.90,
            satisfied: ok,
            story_delta: delta,
            rationale: if ok {
                "Acoustic & vibration separation below quiet zones".to_string()
            } else {
                "MEP plant overlaps residential acoustic zone".to_string()
            },
        });
    }

    if !lab_s.is_empty() && !core_s.is_empty() {
        let delta = min_story_delta(&lab_s, &core_s).unwrap_or(0);
        let ok = delta <= 1;
        pairs.push(AdjacencyPairScore {
            from_dept: "Clinical / Lab".to_string(),
            to_dept: "Triage & Core".to_string(),
            weight: 1.0,
            satisfied: ok,
            story_delta: delta,
            rationale: "Rapid vertical/horizontal transfer from triage core".to_string(),
        });
    }

    if pairs.is_empty() {
        pairs.push(AdjacencyPairScore {
            from_dept: "Vertical Core".to_string(),
            to_dept: "Floor Plate".to_string(),
            weight: 1.0,
            satisfied: true,
            story_delta: 0,
            rationale: "Single-zone compact program".to_string(),
        });
    }

    let total_w: f64 = pairs.iter().map(|p| p.weight).sum();
    let pass_w: f64 = pairs.iter().filter(|p| p.satisfied).map(|p| p.weight).sum();
    let score = if total_w > 0.0 {
        (pass_w / total_w) * 100.0
    } else {
        100.0
    };
    (pairs, score)
}

/// Evaluates Spatial Programming, Vertical Stacking across zoning podium/tower floor plates,
/// deterministic 2D floor-plate packing (`PackedRoomBlock3D`), and AEC Adjacency Scoring.
pub fn evaluate_spatial_program(
    zoning: Option<&SiteZoningReport>,
    config: &SpatialProgramConfig,
) -> SpatialProgramReport {
    let mut cfg = config.clone();
    cfg.circulation_factor_pct = cfg.circulation_factor_pct.clamp(5.0, 40.0);
    cfg.core_ratio_pct = cfg.core_ratio_pct.clamp(6.0, 30.0);
    if cfg.spaces.is_empty() {
        cfg.spaces = cfg.typology.default_spaces();
    }

    let default_zoning_cfg = SiteZoningConfig::default();
    let (
        podium_w,
        podium_d,
        tower_w,
        tower_d,
        podium_stories,
        story_h,
        max_stories,
        zoning_max_gfa_m2,
        pad_y_shift,
    ) = if let Some(z) = zoning {
        let pod_prism = z.envelope_prisms_3d.iter().find(|p| p.tier_kind == 1);
        let twr_prism = z
            .envelope_prisms_3d
            .iter()
            .find(|p| p.tier_kind == 2 || p.tier_kind == 3);
        let (pw, pd) = pod_prism
            .map(|p| (p.max_enu_m[0] - p.min_enu_m[0], p.max_enu_m[1] - p.min_enu_m[1]))
            .unwrap_or((z.buildable_pad_width_m, z.buildable_pad_depth_m));
        let (tw, td) = twr_prism
            .map(|p| (p.max_enu_m[0] - p.min_enu_m[0], p.max_enu_m[1] - p.min_enu_m[1]))
            .unwrap_or((pw, pd));
        let y_shift = (z.config.setback_front_m - z.config.setback_rear_m) * 0.5;
        (
            pw.max(8.0),
            pd.max(8.0),
            tw.max(8.0),
            td.max(8.0),
            z.config.podium_stories.max(1),
            z.config.typical_story_height_m.max(3.0),
            z.max_allowable_stories.clamp(1, 60),
            z.max_gfa_klb_m2.max(200.0),
            y_shift,
        )
    } else {
        let pw = (default_zoning_cfg.lot_width_m - 2.0 * default_zoning_cfg.setback_side_m).max(12.0);
        let pd = (default_zoning_cfg.lot_depth_m
            - default_zoning_cfg.setback_front_m
            - default_zoning_cfg.setback_rear_m)
            .max(12.0);
        let tw = (pw - 2.0 * default_zoning_cfg.tower_stepback_m).max(10.0);
        let td = (pd - 2.0 * default_zoning_cfg.tower_stepback_m).max(10.0);
        let max_st = (default_zoning_cfg.max_height_m / default_zoning_cfg.typical_story_height_m)
            .floor()
            .max(1.0) as usize;
        let max_gfa = default_zoning_cfg.lot_width_m
            * default_zoning_cfg.lot_depth_m
            * default_zoning_cfg.klb_far;
        let y_shift =
            (default_zoning_cfg.setback_front_m - default_zoning_cfg.setback_rear_m) * 0.5;
        (
            pw,
            pd,
            tw,
            td,
            default_zoning_cfg.podium_stories.max(1),
            default_zoning_cfg.typical_story_height_m,
            max_st,
            max_gfa,
            y_shift,
        )
    };

    let plate_for_story = |s_idx: usize| -> (f64, f64) {
        if s_idx < podium_stories {
            (podium_w, podium_d)
        } else {
            (tower_w, tower_d)
        }
    };

    let overhead_ratio = ((cfg.circulation_factor_pct + cfg.core_ratio_pct) / 100.0).clamp(0.12, 0.55);
    let net_ratio = (1.0 - overhead_ratio).max(0.45);

    let mut ordered: Vec<&ProgramSpaceEntry> = cfg.spaces.iter().collect();
    ordered.sort_by_key(|s| (band_rank(s.preferred_floor_band), s.id));

    let total_units: usize = cfg.spaces.iter().map(|s| s.unit_count.max(1)).sum();
    let total_nfa_m2: f64 = cfg.spaces.iter().map(ProgramSpaceEntry::total_area_m2).sum();

    // Stack spaces vertically into story buckets without exceeding net capacity per floor
    let max_stack_cap = max_stories.max(24);
    let mut story_buckets: Vec<Vec<StoryAllocChunk>> = vec![Vec::new()];
    let mut story_used_nfa: Vec<f64> = vec![0.0];

    let story_net_cap = |s_idx: usize| -> f64 {
        let (w, d) = plate_for_story(s_idx);
        (w * d * net_ratio).max(60.0)
    };

    for sp in ordered {
        let mut rem_area = sp.total_area_m2();
        let mut cursor_story = match sp.preferred_floor_band {
            3 | 0 => 0usize,
            1 => 1usize.min(podium_stories),
            _ => podium_stories.min(max_stack_cap.saturating_sub(1)),
        };

        let mut part = 1usize;
        while rem_area > 1.0 && cursor_story < max_stack_cap {
            while story_buckets.len() <= cursor_story {
                story_buckets.push(Vec::new());
                story_used_nfa.push(0.0);
            }
            let cap = story_net_cap(cursor_story);
            let avail = (cap - story_used_nfa[cursor_story]).max(0.0);
            if avail < 18.0 && !story_buckets[cursor_story].is_empty() {
                cursor_story += 1;
                continue;
            }
            let take = rem_area.min(avail.max(cap * 0.85));
            let label = if rem_area > take + 1.0 || part > 1 {
                format!("{} #{part}", sp.name)
            } else {
                sp.name.clone()
            };
            story_buckets[cursor_story].push(StoryAllocChunk {
                space_id: sp.id,
                label,
                department: sp.department,
                area_m2: take,
            });
            story_used_nfa[cursor_story] += take;
            rem_area -= take;
            part += 1;
            if rem_area > 1.0 {
                cursor_story += 1;
            }
        }
    }

    // Trim trailing empty stories
    while story_buckets.len() > 1 && story_buckets.last().is_some_and(Vec::is_empty) {
        story_buckets.pop();
    }

    let mut story_summaries = Vec::with_capacity(story_buckets.len());
    let mut packed_blocks_3d = Vec::new();
    let mut total_gross_stacked_m2 = 0.0;

    for (s_idx, chunks) in story_buckets.iter().enumerate() {
        if chunks.is_empty() {
            continue;
        }
        let (pw, pd) = plate_for_story(s_idx);
        let elev = (s_idx as f64) * story_h;
        let z0 = elev + 0.10;
        let z1 = elev + story_h - 0.16;

        // Reserve central core strip in the middle of the floor plate
        let core_w = (pw * (cfg.core_ratio_pct / 100.0).sqrt()).clamp(3.2, pw * 0.42);
        let core_d = (pd * (cfg.core_ratio_pct / 100.0).sqrt()).clamp(3.2, pd * 0.42);
        packed_blocks_3d.push(PackedRoomBlock3D {
            space_id: 0,
            label: format!("Core L{}", s_idx + 1),
            department: DepartmentKind::LobbyAndCore,
            story_index: s_idx,
            area_m2: core_w * core_d,
            min_enu_m: [-core_w * 0.5, pad_y_shift - core_d * 0.5, z0],
            max_enu_m: [core_w * 0.5, pad_y_shift + core_d * 0.5, z1],
            color_rgba: DepartmentKind::LobbyAndCore.color_rgba(),
        });

        // Pack assignable program chunks into North/South and East/West perimeter wings around core
        let half_w = pw * 0.5;
        let half_d = pd * 0.5;
        let y_min = pad_y_shift - half_d;
        let y_max = pad_y_shift + half_d;
        let yc_min = pad_y_shift - core_d * 0.5;
        let yc_max = pad_y_shift + core_d * 0.5;

        // Partition outer perimeter into 2 primary wings (South-West & North-East) so blocks never overlap central core
        let split = (chunks.len() + 1) / 2;
        let south_wing = [-half_w, y_min, half_w, yc_min];
        let north_wing = [-half_w, yc_max, half_w, y_max];
        if split < chunks.len() {
            pack_chunks_into_rect(&chunks[..split], south_wing, s_idx, z0, z1, &mut packed_blocks_3d);
            pack_chunks_into_rect(&chunks[split..], north_wing, s_idx, z0, z1, &mut packed_blocks_3d);
        } else {
            pack_chunks_into_rect(chunks, south_wing, s_idx, z0, z1, &mut packed_blocks_3d);
        }

        let net_area: f64 = chunks.iter().map(|c| c.area_m2).sum();
        let gross_plate = (net_area / net_ratio).min(pw * pd).max(net_area);
        let core_circ = (gross_plate - net_area).max(0.0);
        let eff_pct = if gross_plate > 0.0 {
            (net_area / gross_plate) * 100.0
        } else {
            0.0
        };
        total_gross_stacked_m2 += gross_plate;

        let primary_dept = chunks
            .iter()
            .max_by(|a, b| {
                a.area_m2
                    .partial_cmp(&b.area_m2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| c.department.short_label().to_string())
            .unwrap_or_else(|| "Mixed".to_string());

        story_summaries.push(StoryStackSummary {
            story_index: s_idx,
            elevation_m: elev,
            floor_height_m: story_h,
            plate_width_m: pw,
            plate_depth_m: pd,
            gross_plate_area_m2: gross_plate,
            net_program_area_m2: net_area,
            core_and_circ_area_m2: core_circ,
            efficiency_pct: eff_pct,
            room_count: chunks.len(),
            primary_department: primary_dept,
        });
    }

    let stacked_stories = story_summaries.len().max(1);
    let total_gfa_m2 = (total_nfa_m2 / net_ratio).max(total_gross_stacked_m2 * 0.85);
    let core_and_circ_m2 = (total_gfa_m2 - total_nfa_m2).max(0.0);
    let overall_efficiency_pct = if total_gfa_m2 > 0.0 {
        (total_nfa_m2 / total_gfa_m2) * 100.0
    } else {
        0.0
    };

    let gfa_utilization_pct = if zoning_max_gfa_m2 > 0.0 {
        (total_gfa_m2 / zoning_max_gfa_m2) * 100.0
    } else {
        0.0
    };
    let fits_zoning_envelope =
        total_gfa_m2 <= zoning_max_gfa_m2 + 1.0 && stacked_stories <= max_stories;

    let (adjacency_pairs, adjacency_score_pct) = evaluate_adjacency(&packed_blocks_3d);

    let fit_tag = if fits_zoning_envelope {
        "FITS ZONING"
    } else {
        "EXCEEDS ZONING"
    };
    let summary_verdict = format!(
        "Program {fit_tag} ({}) • NFA {:.0} m2 / GFA {:.0} m2 ({:.0}% Eff) • {} Stories • Adj {:.0}%",
        cfg.typology.short_label(),
        total_nfa_m2,
        total_gfa_m2,
        overall_efficiency_pct,
        stacked_stories,
        adjacency_score_pct
    );

    SpatialProgramReport {
        config: cfg,
        total_nfa_m2,
        total_gfa_m2,
        core_and_circ_m2,
        overall_efficiency_pct,
        total_units,
        stacked_stories,
        zoning_max_gfa_m2,
        gfa_utilization_pct,
        fits_zoning_envelope,
        adjacency_score_pct,
        adjacency_pairs,
        story_summaries,
        packed_blocks_3d,
        summary_verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};
    use crate::core::zoning_agent::{evaluate_site_zoning, SiteZoningConfig, ZoningDistrictPreset};

    #[test]
    fn test_spatial_program_stack_pack_and_non_overlap() {
        let zoning = evaluate_site_zoning(&BimProject::default(), None, &SiteZoningConfig::default());
        for typology in BuildingTypologyPreset::ALL {
            let mut cfg = SpatialProgramConfig::default();
            cfg.apply_typology(typology);
            let rep = evaluate_spatial_program(Some(&zoning), &cfg);
            assert!(rep.total_nfa_m2 > 500.0);
            assert!(rep.total_gfa_m2 > rep.total_nfa_m2);
            assert!(rep.overall_efficiency_pct > 45.0 && rep.overall_efficiency_pct < 90.0);
            assert!(!rep.packed_blocks_3d.is_empty());
            assert!(rep.adjacency_score_pct >= 50.0);

            // Verify non-overlapping packed rectangles per story
            for s in 0..rep.stacked_stories {
                let blocks: Vec<&PackedRoomBlock3D> = rep
                    .packed_blocks_3d
                    .iter()
                    .filter(|b| b.story_index == s)
                    .collect();
                for i in 0..blocks.len() {
                    for j in (i + 1)..blocks.len() {
                        let a = blocks[i];
                        let b = blocks[j];
                        let overlap_x = a.min_enu_m[0] < b.max_enu_m[0] - 0.01
                            && a.max_enu_m[0] > b.min_enu_m[0] + 0.01;
                        let overlap_y = a.min_enu_m[1] < b.max_enu_m[1] - 0.01
                            && a.max_enu_m[1] > b.min_enu_m[1] + 0.01;
                        assert!(
                            !(overlap_x && overlap_y),
                            "Story {s} overlap between '{}' and '{}'",
                            a.label,
                            b.label
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_spatial_program_detects_zoning_overflow_and_ifcx_overlay() {
        let mut zcfg = SiteZoningConfig::default();
        zcfg.apply_preset(ZoningDistrictPreset::LowRiseSuburban);
        let zoning = evaluate_site_zoning(&BimProject::default(), None, &zcfg);
        let pcfg = SpatialProgramConfig::default(); // MixedUseTower > LowRiseSuburban KLB 1.2
        let rep = evaluate_spatial_program(Some(&zoning), &pcfg);
        assert!(!rep.fits_zoning_envelope);
        assert!(rep.summary_verdict.contains("EXCEEDS ZONING"));

        let base = IfcxDocument::new("program_test");
        let composed =
            IfcxStage::compose_with_spatial_program_overlay(&base, &rep.to_ifcx_overlay());
        let node = composed
            .data
            .iter()
            .find(|n| n.path == "/earth/design/spatial_program")
            .expect("spatial_program node");
        assert!(node.attributes.contains_key(NS_EG_SPATIAL_PROGRAM));
    }
}
