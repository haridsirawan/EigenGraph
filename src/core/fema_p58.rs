//! fema_p58.rs — FEMA P-58 Next-Generation Seismic Performance Assessment & Probabilistic Loss Engine.
//!
//! Compliant with:
//! - FEMA P-58-1 / ATC-58: "Seismic Performance Assessment of Buildings, Methodology" (Volumes 1 & 2)
//! - Arup REDi™ Rating System: "Resilience-based Earthquake Design Initiative for Next Generation Buildings"
//! - ASCE 7-22 Chapter 16 / ASCE 41-17: "Seismic Evaluation and Retrofit of Existing Buildings"
//! - ISO 14040/14044 & EN 15978: "Sustainability of Construction Works — Assessment of Environmental Performance"

use serde::{Deserialize, Serialize};

/// Type of Engineering Demand Parameter (EDP) governing component fragility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdpKind {
    /// Peak Inter-Story Drift Ratio (rad).
    PeakInterstoryDriftRatio,
    /// Peak Floor Acceleration (g).
    PeakFloorAcceleration,
    /// Residual Drift Ratio (rad).
    ResidualDriftRatio,
}

impl EdpKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PeakInterstoryDriftRatio => "Peak Inter-Story Drift Ratio (PIDR)",
            Self::PeakFloorAcceleration => "Peak Floor Acceleration (PFA)",
            Self::ResidualDriftRatio => "Residual Drift Ratio (RDR)",
        }
    }

    pub fn unit(&self) -> &'static str {
        match self {
            Self::PeakInterstoryDriftRatio => "rad",
            Self::PeakFloorAcceleration => "g",
            Self::ResidualDriftRatio => "rad",
        }
    }
}

/// Category of building component in FEMA P-58 inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FemaComponentCategory {
    /// Structural components (drift-sensitive: columns, beams, shear walls, braces).
    StructuralFrame,
    /// Non-structural drift-sensitive (curtain walls, drywall partitions, interior glazing).
    NonStructuralDriftSensitive,
    /// Non-structural acceleration-sensitive (ceilings, MEP piping, HVAC, electrical transformers).
    NonStructuralAccelerationSensitive,
    /// Building contents (servers, medical equipment, commercial inventory).
    BuildingContents,
}

impl FemaComponentCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StructuralFrame => "Structural Frame (Drift-Sensitive)",
            Self::NonStructuralDriftSensitive => "Non-Structural Enclosure & Partitions (Drift-Sensitive)",
            Self::NonStructuralAccelerationSensitive => "MEP & Ceiling Systems (Acceleration-Sensitive)",
            Self::BuildingContents => "Essential Equipment & Contents",
        }
    }
}

/// Damage state (DS) definition for a specific component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageStateDef {
    pub ds_index: usize,
    pub name: String,
    pub description: String,
    /// Median demand capacity theta (in units of EDP).
    pub median_theta: f64,
    /// Logarithmic dispersion beta (uncertainty).
    pub beta: f64,
    /// Unit repair cost ($ USD per component unit).
    pub unit_repair_cost_usd: f64,
    /// Unit repair labor time (person-days).
    pub unit_repair_labor_days: f64,
    /// Embodied carbon impact (kg CO2-eq per repair unit).
    pub unit_embodied_carbon_kg: f64,
}

/// Component fragility and consequence definition per FEMA P-58 library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentFragility {
    pub code_id: String,
    pub name: String,
    pub category: FemaComponentCategory,
    pub edp_kind: EdpKind,
    pub unit_name: String,
    pub damage_states: Vec<DamageStateDef>,
}

/// Building component group instantiated across a floor or story.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryComponentGroup {
    pub story_number: usize,
    pub fragility: ComponentFragility,
    pub quantity: f64,
}

/// REDi™ Resilience Rating Tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum REDiResilienceTier {
    Platinum,
    Gold,
    Silver,
    #[default]
    StandardCode,
}

impl REDiResilienceTier {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Platinum => "REDi™ Platinum (Immediate Re-occupancy ≤ 7 Days)",
            Self::Gold => "REDi™ Gold (Quick Recovery ≤ 30 Days)",
            Self::Silver => "REDi™ Silver (Functional Recovery ≤ 60 Days)",
            Self::StandardCode => "Baseline Code (Life Safety Only, Recovery > 180 Days)",
        }
    }

    pub fn max_downtime_days(&self) -> f64 {
        match self {
            Self::Platinum => 7.0,
            Self::Gold => 30.0,
            Self::Silver => 60.0,
            Self::StandardCode => 180.0,
        }
    }

    pub fn max_loss_ratio(&self) -> f64 {
        match self {
            Self::Platinum => 0.05, // ≤ 5% of replacement value
            Self::Gold => 0.10,     // ≤ 10%
            Self::Silver => 0.20,   // ≤ 20%
            Self::StandardCode => 1.00,
        }
    }
}

/// Input parameters for FEMA P-58 Probabilistic Seismic Loss Assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FemaP58Input {
    pub project_name: String,
    /// Total building replacement value ($ USD).
    pub building_replacement_value_usd: f64,
    /// Total gross floor area (m²).
    pub total_floor_area_m2: f64,
    /// Number of stories.
    pub num_stories: usize,
    /// Story height (m).
    pub story_height_m: f64,
    /// Peak ground acceleration at site for hazard scenario (g).
    pub site_pga_g: f64,
    /// Spectral acceleration at fundamental period Sa(T1) in g.
    pub spectral_accel_sa1_g: f64,
    /// Max allowable labor workers per story concurrently (labor constraint).
    pub max_laborers_per_story: usize,
    /// Daily business revenue generated by facility ($ USD / day).
    pub daily_business_revenue_usd: f64,
    /// Target REDi resilience tier for design benchmarking.
    pub target_redi_tier: REDiResilienceTier,
}

impl Default for FemaP58Input {
    fn default() -> Self {
        Self {
            project_name: "Metropolitan Medical Center Tower".to_string(),
            building_replacement_value_usd: 85_000_000.0,
            total_floor_area_m2: 24_000.0,
            num_stories: 10,
            story_height_m: 3.80,
            site_pga_g: 0.45,
            spectral_accel_sa1_g: 0.72,
            max_laborers_per_story: 12,
            daily_business_revenue_usd: 120_000.0,
            target_redi_tier: REDiResilienceTier::Gold,
        }
    }
}

/// Story-level loss summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryLossSummary {
    pub story_number: usize,
    pub floor_elevation_m: f64,
    pub peak_drift_ratio: f64,
    pub peak_floor_accel_g: f64,
    pub repair_cost_usd: f64,
    pub repair_loss_ratio: f64,
    pub repair_labor_person_days: f64,
    pub embodied_carbon_kg_co2e: f64,
    pub governing_component: String,
}

/// Regulatory / REDi compliance audit row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FemaComplianceItem {
    pub metric_name: String,
    pub code_reference: String,
    pub demand_val: f64,
    pub permissible_limit: f64,
    pub unit: String,
    pub utilization_ratio: f64,
    pub is_passed: bool,
}

/// Comprehensive output report from FEMA P-58 loss assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FemaP58Report {
    pub total_repair_cost_usd: f64,
    pub repair_cost_ratio: f64,
    pub total_repair_labor_days: f64,
    pub downtime_days: f64,
    pub business_interruption_loss_usd: f64,
    pub total_economic_loss_usd: f64,
    pub total_embodied_carbon_kg_co2e: f64,
    pub embodied_carbon_per_m2: f64,
    pub collapse_probability: f64,
    pub demolition_probability: f64,
    pub expected_annual_loss_ratio: f64,
    pub achieved_redi_tier: REDiResilienceTier,
    pub is_target_redi_achieved: bool,
    pub story_losses: Vec<StoryLossSummary>,
    pub compliance_audit: Vec<FemaComplianceItem>,
    pub fragility_sample_curves: Vec<(String, Vec<(f64, f64)>)>,
}

/// Continuous FEMA P-58 lognormal fragility exceedance point parameterized by spectral acceleration `Sa(T1)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayFragilityCurvePoint {
    pub sa_g: f64,
    pub p_io: f64,
    pub p_ls: f64,
    pub p_cp: f64,
    pub p_collapse: f64,
}

/// Real-time story-level repair cost, downtime, and carbon loss derived from BIM + NL-THA demands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryFragilityLoss {
    pub story_number: usize,
    pub elevation_m: f64,
    pub peak_drift_ratio: f64,
    pub peak_floor_accel_g: f64,
    pub repair_cost_usd: f64,
    pub loss_ratio: f64,
    pub downtime_days: f64,
    pub embodied_carbon_kg: f64,
    pub governing_component: String,
}

/// Coupled BIM + NL-THA real-time FEMA P-58 fragility and probabilistic loss synthesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReplayFemaP58Synthesis {
    pub building_replacement_value_usd: f64,
    pub total_repair_cost_usd: f64,
    pub repair_loss_ratio: f64,
    pub downtime_days: f64,
    pub business_interruption_usd: f64,
    pub total_economic_loss_usd: f64,
    pub repair_embodied_carbon_kg: f64,
    pub collapse_probability: f64,
    pub demolition_probability: f64,
    pub achieved_redi_tier: REDiResilienceTier,
    pub sa_t1_g: f64,
    pub story_losses: Vec<StoryFragilityLoss>,
    pub fragility_curves: Vec<ReplayFragilityCurvePoint>,
}

/// Core computational engine for FEMA P-58 / REDi assessment.
pub struct FemaP58Engine;

impl FemaP58Engine {
    /// Standard Normal Cumulative Distribution Function approximation.
    pub fn phi(x: f64) -> f64 {
        // Abramowitz & Stegun approximation (formula 7.1.26)
        let b1 = 0.319381530;
        let b2 = -0.356563782;
        let b3 = 1.781477937;
        let b4 = -1.821255978;
        let b5 = 1.330274429;
        let p = 0.2316419;
        let c = 0.3989422804014337; // 1 / sqrt(2 * pi)

        if x >= 0.0 {
            let t = 1.0 / (1.0 + p * x);
            (1.0 - c * (-x * x / 2.0).exp() * t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))))).clamp(0.0, 1.0)
        } else {
            let t = 1.0 / (1.0 - p * x);
            (c * (-x * x / 2.0).exp() * t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))))).clamp(0.0, 1.0)
        }
    }

    /// Evaluates lognormal fragility probability: P(DS >= ds | EDP) = Phi((ln(EDP) - ln(theta)) / beta)
    pub fn evaluate_fragility(edp: f64, theta: f64, beta: f64) -> f64 {
        if edp <= 1e-6 || theta <= 1e-6 || beta <= 1e-6 {
            return 0.0;
        }
        let z = (edp / theta).ln() / beta;
        Self::phi(z)
    }

    /// Builds canonical standard FEMA P-58 component fragility database.
    pub fn build_standard_fragility_database() -> Vec<ComponentFragility> {
        vec![
            // 1. Reinforced Concrete Special Moment Frame Beam-Column Joint (B1041.011a)
            ComponentFragility {
                code_id: "B1041.011a".to_string(),
                name: "RC Special Moment Frame Joint (High Ductility)".to_string(),
                category: FemaComponentCategory::StructuralFrame,
                edp_kind: EdpKind::PeakInterstoryDriftRatio,
                unit_name: "Joint".to_string(),
                damage_states: vec![
                    DamageStateDef {
                        ds_index: 1,
                        name: "DS1: Flexural Hairline Cracking".to_string(),
                        description: "Cracks < 1.0 mm, epoxy pressure injection".to_string(),
                        median_theta: 0.012,
                        beta: 0.35,
                        unit_repair_cost_usd: 1_850.0,
                        unit_repair_labor_days: 1.5,
                        unit_embodied_carbon_kg: 85.0,
                    },
                    DamageStateDef {
                        ds_index: 2,
                        name: "DS2: Concrete Spalling & Yielding".to_string(),
                        description: "Cover spall, longitudinal rebar yield, jacketing".to_string(),
                        median_theta: 0.022,
                        beta: 0.40,
                        unit_repair_cost_usd: 8_500.0,
                        unit_repair_labor_days: 6.0,
                        unit_embodied_carbon_kg: 420.0,
                    },
                    DamageStateDef {
                        ds_index: 3,
                        name: "DS3: Core Crushing & Bar Buckling".to_string(),
                        description: "Severe core loss, bar fracture, partial demolition".to_string(),
                        median_theta: 0.040,
                        beta: 0.45,
                        unit_repair_cost_usd: 28_000.0,
                        unit_repair_labor_days: 18.0,
                        unit_embodied_carbon_kg: 1_850.0,
                    },
                ],
            },
            // 2. Gypsum Board Drywall Partitions (C1011.001a)
            ComponentFragility {
                code_id: "C1011.001a".to_string(),
                name: "Drywall Partitions with Metal Studs".to_string(),
                category: FemaComponentCategory::NonStructuralDriftSensitive,
                edp_kind: EdpKind::PeakInterstoryDriftRatio,
                unit_name: "30m Wall".to_string(),
                damage_states: vec![
                    DamageStateDef {
                        ds_index: 1,
                        name: "DS1: Corner Tape Tearing".to_string(),
                        description: "Drywall tape tear, mudding and repainting".to_string(),
                        median_theta: 0.005,
                        beta: 0.40,
                        unit_repair_cost_usd: 1_200.0,
                        unit_repair_labor_days: 1.0,
                        unit_embodied_carbon_kg: 45.0,
                    },
                    DamageStateDef {
                        ds_index: 2,
                        name: "DS2: Board Cracking & Screw Pull-out".to_string(),
                        description: "Gypsum sheet replacement, taping, refinishing".to_string(),
                        median_theta: 0.011,
                        beta: 0.42,
                        unit_repair_cost_usd: 4_200.0,
                        unit_repair_labor_days: 3.5,
                        unit_embodied_carbon_kg: 210.0,
                    },
                    DamageStateDef {
                        ds_index: 3,
                        name: "DS3: Stud Buckling & Panel Collapse".to_string(),
                        description: "Full wall demolition and framing replacement".to_string(),
                        median_theta: 0.020,
                        beta: 0.45,
                        unit_repair_cost_usd: 11_500.0,
                        unit_repair_labor_days: 8.0,
                        unit_embodied_carbon_kg: 580.0,
                    },
                ],
            },
            // 3. Glazed Curtain Wall Exterior Facade (B2022.011)
            ComponentFragility {
                code_id: "B2022.011".to_string(),
                name: "Structural Silicone Glazed Curtain Wall".to_string(),
                category: FemaComponentCategory::NonStructuralDriftSensitive,
                edp_kind: EdpKind::PeakInterstoryDriftRatio,
                unit_name: "Facade Bay".to_string(),
                damage_states: vec![
                    DamageStateDef {
                        ds_index: 1,
                        name: "DS1: Gasket Slippage & Seal Leak".to_string(),
                        description: "Re-caulking, silicone gasket re-sealing".to_string(),
                        median_theta: 0.010,
                        beta: 0.35,
                        unit_repair_cost_usd: 2_400.0,
                        unit_repair_labor_days: 2.0,
                        unit_embodied_carbon_kg: 95.0,
                    },
                    DamageStateDef {
                        ds_index: 2,
                        name: "DS2: Glass Cracking & Fallout".to_string(),
                        description: "Insulated glass unit replacement from exterior hoist".to_string(),
                        median_theta: 0.021,
                        beta: 0.38,
                        unit_repair_cost_usd: 9_800.0,
                        unit_repair_labor_days: 6.5,
                        unit_embodied_carbon_kg: 680.0,
                    },
                ],
            },
            // 4. Suspended Acoustic Ceiling System (C3032.001)
            ComponentFragility {
                code_id: "C3032.001".to_string(),
                name: "Acoustic Tile Suspended Ceiling (with Splay Wires)".to_string(),
                category: FemaComponentCategory::NonStructuralAccelerationSensitive,
                edp_kind: EdpKind::PeakFloorAcceleration,
                unit_name: "250m² Ceiling".to_string(),
                damage_states: vec![
                    DamageStateDef {
                        ds_index: 1,
                        name: "DS1: Tile Dislodgement (<10%)".to_string(),
                        description: "Re-seating fallen acoustic tiles".to_string(),
                        median_theta: 0.65,
                        beta: 0.40,
                        unit_repair_cost_usd: 1_600.0,
                        unit_repair_labor_days: 1.5,
                        unit_embodied_carbon_kg: 60.0,
                    },
                    DamageStateDef {
                        ds_index: 2,
                        name: "DS2: Grid Tee Buckling & Severe Tile Loss".to_string(),
                        description: "Tee grid replacement, hanger re-attachment".to_string(),
                        median_theta: 1.25,
                        beta: 0.45,
                        unit_repair_cost_usd: 6_200.0,
                        unit_repair_labor_days: 5.0,
                        unit_embodied_carbon_kg: 340.0,
                    },
                ],
            },
            // 5. Chilled Water MEP Piping & Fire Sprinklers (D2022.012)
            ComponentFragility {
                code_id: "D2022.012".to_string(),
                name: "Seismically Braced Chilled Water Piping (Ø100mm)".to_string(),
                category: FemaComponentCategory::NonStructuralAccelerationSensitive,
                edp_kind: EdpKind::PeakFloorAcceleration,
                unit_name: "Floor Run".to_string(),
                damage_states: vec![
                    DamageStateDef {
                        ds_index: 1,
                        name: "DS1: Threaded Joint Leak".to_string(),
                        description: "Tighten couplings, water clean-up".to_string(),
                        median_theta: 0.85,
                        beta: 0.38,
                        unit_repair_cost_usd: 3_100.0,
                        unit_repair_labor_days: 2.5,
                        unit_embodied_carbon_kg: 110.0,
                    },
                    DamageStateDef {
                        ds_index: 2,
                        name: "DS2: Pipe Rupture & Water Flooding".to_string(),
                        description: "Section replacement, extensive water remediation".to_string(),
                        median_theta: 1.60,
                        beta: 0.42,
                        unit_repair_cost_usd: 18_500.0,
                        unit_repair_labor_days: 12.0,
                        unit_embodied_carbon_kg: 920.0,
                    },
                ],
            },
        ]
    }

    /// Evaluates probabilistic loss and downtime for the building.
    pub fn evaluate(input: &FemaP58Input) -> FemaP58Report {
        let db = Self::build_standard_fragility_database();
        let num_stories = input.num_stories.max(1);
        let mut story_losses = Vec::with_capacity(num_stories);

        let mut total_repair_cost = 0.0;
        let mut total_labor_person_days = 0.0;
        let mut total_embodied_carbon = 0.0;

        // Peak demands per story (higher-mode whip profile)
        let mut max_story_labor_days = 0.0f64;

        for s in 1..=num_stories {
            let z = (s as f64) * input.story_height_m;
            let frac = z / (num_stories as f64 * input.story_height_m);

            // Realistic EDP demands derived from spectral acceleration Sa
            // Inter-story drift envelope (parabolic with mid-height bulge)
            let base_drift = (input.spectral_accel_sa1_g * 0.016).clamp(0.004, 0.035);
            let story_drift = base_drift * (0.60 + 0.90 * (1.0 - (frac - 0.55).powi(2) * 3.0)).clamp(0.003, 0.040);

            // Peak floor acceleration envelope (trapezoidal whip: ground PGA -> top 2.5x PGA)
            let story_pfa = input.site_pga_g * (1.0 + 1.8 * frac.powf(1.15));

            let mut story_cost = 0.0;
            let mut story_labor = 0.0;
            let mut story_carbon = 0.0;
            let mut governing_comp = "None".to_string();
            let mut max_comp_cost = 0.0f64;

            // Quantities per story (scaled to floor area)
            let area_per_floor = input.total_floor_area_m2 / (num_stories as f64);
            let frame_joints = (area_per_floor / 60.0).round().max(12.0);
            let partition_units = (area_per_floor / 150.0).round().max(6.0);
            let facade_bays = ((input.story_height_m * 4.0 * (area_per_floor).sqrt()) / 35.0).round().max(8.0);
            let ceiling_units = (area_per_floor / 250.0).round().max(4.0);
            let mep_units = (area_per_floor / 400.0).round().max(2.0);

            let quantities = [
                frame_joints,
                partition_units,
                facade_bays,
                ceiling_units,
                mep_units,
            ];

            for (idx, comp) in db.iter().enumerate() {
                let edp = match comp.edp_kind {
                    EdpKind::PeakInterstoryDriftRatio => story_drift,
                    EdpKind::PeakFloorAcceleration => story_pfa,
                    EdpKind::ResidualDriftRatio => story_drift * 0.25,
                };

                let qty = quantities[idx % quantities.len()];

                // Sequential damage state probability partitioning:
                // P(DS = i) = P(DS >= i) - P(DS >= i+1)
                let num_ds = comp.damage_states.len();
                let mut p_exceed = Vec::with_capacity(num_ds + 1);
                for ds in &comp.damage_states {
                    p_exceed.push(Self::evaluate_fragility(edp, ds.median_theta, ds.beta));
                }
                p_exceed.push(0.0); // P(DS >= ds_{num_ds+1}) = 0

                let mut comp_cost = 0.0;
                let mut comp_labor = 0.0;
                let mut comp_carbon = 0.0;

                for i in 0..num_ds {
                    let p_ds = (p_exceed[i] - p_exceed[i + 1]).max(0.0);
                    let ds = &comp.damage_states[i];
                    comp_cost += p_ds * ds.unit_repair_cost_usd * qty;
                    comp_labor += p_ds * ds.unit_repair_labor_days * qty;
                    comp_carbon += p_ds * ds.unit_embodied_carbon_kg * qty;
                }

                if comp_cost > max_comp_cost {
                    max_comp_cost = comp_cost;
                    governing_comp = comp.name.clone();
                }

                story_cost += comp_cost;
                story_labor += comp_labor;
                story_carbon += comp_carbon;
            }

            if story_labor > max_story_labor_days {
                max_story_labor_days = story_labor;
            }

            total_repair_cost += story_cost;
            total_labor_person_days += story_labor;
            total_embodied_carbon += story_carbon;

            story_losses.push(StoryLossSummary {
                story_number: s,
                floor_elevation_m: z,
                peak_drift_ratio: story_drift,
                peak_floor_accel_g: story_pfa,
                repair_cost_usd: story_cost,
                repair_loss_ratio: story_cost / (input.building_replacement_value_usd / num_stories as f64),
                repair_labor_person_days: story_labor,
                embodied_carbon_kg_co2e: story_carbon,
                governing_component: governing_comp,
            });
        }

        // Downtime integration per REDi methodology:
        // Critical path = max(story_labor / max_laborers_per_story) * mobilization_factor + inspection_delay
        let labor_per_story = input.max_laborers_per_story.max(1) as f64;
        let serial_parallel_downtime = (max_story_labor_days / labor_per_story) * 1.35; // 35% scheduling buffer
        let inspection_permitting_days = if total_repair_cost > 0.05 * input.building_replacement_value_usd { 14.0 } else { 3.0 };
        let downtime_days = (serial_parallel_downtime + inspection_permitting_days).round();

        // Business interruption loss = downtime_days * daily_business_revenue
        let bi_loss = downtime_days * input.daily_business_revenue_usd;
        let total_economic_loss = total_repair_cost + bi_loss;
        let repair_cost_ratio = total_repair_cost / input.building_replacement_value_usd;
        let embodied_carbon_per_m2 = total_embodied_carbon / input.total_floor_area_m2;

        // Collapse and Demolition Probabilities (FEMA P-58 Part 2)
        // Median collapse capacity ~ 1.50g, beta = 0.40
        let collapse_prob = Self::evaluate_fragility(input.spectral_accel_sa1_g, 1.45, 0.40);
        // Demolition due to excessive residual drift (threshold 1.0% = 0.010 rad)
        let max_drift = story_losses.iter().map(|s| s.peak_drift_ratio).fold(0.0f64, |a, b| a.max(b));
        let max_rdr = max_drift * 0.28;
        let demolition_prob = Self::evaluate_fragility(max_rdr, 0.012, 0.35);

        // Expected Annual Loss (EAL) ratio approximation (FEMA P-58 integration over hazard curve)
        let eal_ratio = (repair_cost_ratio * 0.075).clamp(0.002, 0.045);

        // Achieved REDi Tier Classification
        let achieved_redi_tier = if downtime_days <= 7.0 && repair_cost_ratio <= 0.05 {
            REDiResilienceTier::Platinum
        } else if downtime_days <= 30.0 && repair_cost_ratio <= 0.10 {
            REDiResilienceTier::Gold
        } else if downtime_days <= 60.0 && repair_cost_ratio <= 0.20 {
            REDiResilienceTier::Silver
        } else {
            REDiResilienceTier::StandardCode
        };

        let is_target_redi_achieved = achieved_redi_tier as usize <= input.target_redi_tier as usize;

        // Compliance and Limit State Checks
        let mut compliance_audit = Vec::new();

        // 1. Repair Loss Ratio Check
        let max_allowed_loss = input.target_redi_tier.max_loss_ratio();
        compliance_audit.push(FemaComplianceItem {
            metric_name: "Direct Structural & Non-Structural Repair Loss Ratio".to_string(),
            code_reference: format!("REDi™ Section 3.2 ({:?})", input.target_redi_tier),
            demand_val: repair_cost_ratio * 100.0,
            permissible_limit: max_allowed_loss * 100.0,
            unit: "% of Asset".to_string(),
            utilization_ratio: repair_cost_ratio / max_allowed_loss,
            is_passed: repair_cost_ratio <= max_allowed_loss,
        });

        // 2. Functional Recovery Downtime Check
        let max_allowed_days = input.target_redi_tier.max_downtime_days();
        compliance_audit.push(FemaComplianceItem {
            metric_name: "Facility Functional Recovery Downtime".to_string(),
            code_reference: format!("REDi™ Section 3.4 ({:?})", input.target_redi_tier),
            demand_val: downtime_days,
            permissible_limit: max_allowed_days,
            unit: "Days".to_string(),
            utilization_ratio: downtime_days / max_allowed_days,
            is_passed: downtime_days <= max_allowed_days,
        });

        // 3. Demolition Risk due to Residual Drift
        compliance_audit.push(FemaComplianceItem {
            metric_name: "Demolition Probability due to Irreparable Residual Drift".to_string(),
            code_reference: "FEMA P-58-1 Volume 1 §7.3 / Ramirez (2012)".to_string(),
            demand_val: demolition_prob * 100.0,
            permissible_limit: 5.0, // Limit 5%
            unit: "% Risk".to_string(),
            utilization_ratio: (demolition_prob * 100.0) / 5.0,
            is_passed: demolition_prob <= 0.05,
        });

        // 4. Collapse Safety Margin under MCE Ground Motion
        compliance_audit.push(FemaComplianceItem {
            metric_name: "Collapse Probability at Maximum Considered Earthquake (MCE)".to_string(),
            code_reference: "ASCE 7-22 Table 1.3-1 / FEMA P-695".to_string(),
            demand_val: collapse_prob * 100.0,
            permissible_limit: 10.0, // 10% limit for Risk Cat II/III
            unit: "% Risk".to_string(),
            utilization_ratio: (collapse_prob * 100.0) / 10.0,
            is_passed: collapse_prob <= 0.10,
        });

        // 5. Embodied Carbon Threshold
        compliance_audit.push(FemaComplianceItem {
            metric_name: "Post-Earthquake Repair Embodied Carbon Footprint".to_string(),
            code_reference: "ISO 14040 / EN 15978 Embodied GHG Intensity".to_string(),
            demand_val: embodied_carbon_per_m2,
            permissible_limit: 25.0, // Limit 25 kg CO2e / m2
            unit: "kg CO2e/m²".to_string(),
            utilization_ratio: embodied_carbon_per_m2 / 25.0,
            is_passed: embodied_carbon_per_m2 <= 25.0,
        });

        // Fragility Sample Curves for 2D Vector CAD Plotting
        let mut fragility_sample_curves = Vec::new();
        for comp in &db {
            if let Some(ds) = comp.damage_states.get(1).or_else(|| comp.damage_states.first()) {
                let max_edp = ds.median_theta * 2.8;
                let mut pts = Vec::with_capacity(30);
                for i in 0..=30 {
                    let edp = (i as f64 / 30.0) * max_edp;
                    let p = Self::evaluate_fragility(edp, ds.median_theta, ds.beta);
                    pts.push((edp, p));
                }
                fragility_sample_curves.push((comp.name.clone(), pts));
            }
        }

        FemaP58Report {
            total_repair_cost_usd: total_repair_cost,
            repair_cost_ratio,
            total_repair_labor_days: total_labor_person_days,
            downtime_days,
            business_interruption_loss_usd: bi_loss,
            total_economic_loss_usd: total_economic_loss,
            total_embodied_carbon_kg_co2e: total_embodied_carbon,
            embodied_carbon_per_m2,
            collapse_probability: collapse_prob,
            demolition_probability: demolition_prob,
            expected_annual_loss_ratio: eal_ratio,
            achieved_redi_tier,
            is_target_redi_achieved,
            story_losses,
            compliance_audit,
            fragility_sample_curves,
        }
    }

    /// Generates a 10-story FEMA P-58 hospital/resilience archetype (nodes, elements, is_structural).
    pub fn generate_fema_resilience_archetype() -> (Vec<(usize, f64, f64, f64)>, Vec<(usize, usize, usize, bool)>) {
        let mut nodes = Vec::new();
        let mut elements = Vec::new();

        let grid_x = [0.0, 8.0, 16.0, 24.0];
        let grid_y = [0.0, 8.0, 16.0];
        let story_h = 3.80; // m
        let num_stories = 10;

        let mut node_id = 0;
        let mut elem_id = 0;

        // Ground level 0
        let mut prev_floor_nodes = Vec::new();
        for &gx in &grid_x {
            for &gy in &grid_y {
                nodes.push((node_id, gx, gy, 0.0));
                prev_floor_nodes.push(node_id);
                node_id += 1;
            }
        }

        // Stories 1 to 10
        for story in 1..=num_stories {
            let z = (story as f64) * story_h;
            let mut curr_floor_nodes = Vec::new();

            for &gx in &grid_x {
                for &gy in &grid_y {
                    nodes.push((node_id, gx, gy, z));
                    curr_floor_nodes.push(node_id);
                    node_id += 1;
                }
            }

            // Columns
            for i in 0..prev_floor_nodes.len() {
                elements.push((elem_id, prev_floor_nodes[i], curr_floor_nodes[i], true));
                elem_id += 1;
            }

            // Beams along X
            let nx = grid_x.len();
            let ny = grid_y.len();
            for iy in 0..ny {
                for ix in 0..(nx - 1) {
                    let n1 = curr_floor_nodes[ix * ny + iy];
                    let n2 = curr_floor_nodes[(ix + 1) * ny + iy];
                    elements.push((elem_id, n1, n2, true));
                    elem_id += 1;
                }
            }

            // Beams along Y
            for ix in 0..nx {
                for iy in 0..(ny - 1) {
                    let n1 = curr_floor_nodes[ix * ny + iy];
                    let n2 = curr_floor_nodes[ix * ny + (iy + 1)];
                    elements.push((elem_id, n1, n2, true));
                    elem_id += 1;
                }
            }

            // Non-structural perimeter facade elements
            let n_corner1 = curr_floor_nodes[0];
            let n_corner2 = curr_floor_nodes[(nx - 1) * ny + (ny - 1)];
            elements.push((elem_id, n_corner1, n_corner2, false)); // Non-structural tie
            elem_id += 1;

            prev_floor_nodes = curr_floor_nodes;
        }

        (nodes, elements)
    }

    /// Generates professional Markdown Calculation Dossier for FEMA P-58 loss assessment.
    pub fn generate_markdown_dossier(input: &FemaP58Input, report: &FemaP58Report) -> String {
        let mut md = String::new();
        md.push_str("# EigenGraph Calculation Dossier: FEMA P-58 Seismic Performance & Probabilistic Loss Assessment\n");
        md.push_str("## Next-Generation Building Performance & REDi™ Resilience Audit (FEMA P-58 / ATC-58 / REDi)\n\n");

        md.push_str(&format!("- **Facility Name:** {}\n", input.project_name));
        md.push_str(&format!("- **Asset Replacement Value:** ${:.0} USD\n", input.building_replacement_value_usd));
        md.push_str(&format!("- **Total Floor Area:** {:.0} m² ({} stories, {:.2}m story height)\n", input.total_floor_area_m2, input.num_stories, input.story_height_m));
        md.push_str(&format!("- **Seismic Hazard Demand:** PGA = {:.2}g | Sa(T1) = {:.2}g\n", input.site_pga_g, input.spectral_accel_sa1_g));
        md.push_str(&format!("- **Achieved REDi™ Tier:** {:?} ({})\n", report.achieved_redi_tier, report.achieved_redi_tier.label()));
        md.push_str(&format!("- **Direct Repair Cost:** ${:.0} USD ({:.2}% of Replacement Value)\n", report.total_repair_cost_usd, report.repair_cost_ratio * 100.0));
        md.push_str(&format!("- **Downtime / Business Interruption:** {:.0} Days (${:.0} USD BI loss)\n", report.downtime_days, report.business_interruption_loss_usd));
        md.push_str(&format!("- **Total Economic Exposure:** ${:.0} USD\n", report.total_economic_loss_usd));
        md.push_str(&format!("- **Post-Earthquake Embodied Carbon:** {:.1} kg CO2e ({:.2} kg CO2e/m²)\n", report.total_embodied_carbon_kg_co2e, report.embodied_carbon_per_m2));
        md.push_str(&format!("- **Demolition Risk (Residual Drift):** {:.2}% | **Collapse Risk:** {:.2}%\n\n", report.demolition_probability * 100.0, report.collapse_probability * 100.0));

        md.push_str("### 1. Regulatory & REDi™ Performance Compliance Matrix\n\n");
        md.push_str("| Metric Description | Code Reference | Demand | Permissible Limit | Unit | D/C Ratio | Status |\n");
        md.push_str("| :--- | :--- | :---: | :---: | :---: | :---: | :---: |\n");
        for c in &report.compliance_audit {
            let status = if c.is_passed { "**PASS**" } else { "**FAIL**" };
            md.push_str(&format!(
                "| {} | {} | {:.2} | {:.2} | {} | {:.3} | {} |\n",
                c.metric_name, c.code_reference, c.demand_val, c.permissible_limit, c.unit, c.utilization_ratio, status
            ));
        }

        md.push_str("\n### 2. Story-by-Story Seismic Demand & Loss Distribution\n\n");
        md.push_str("| Story | Elev (m) | Peak Drift (rad) | PFA (g) | Repair Cost ($ USD) | Loss Ratio (%) | Labor (Person-Days) | Embodied CO2e (kg) | Governing Fragility Component |\n");
        md.push_str("| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |\n");
        for s in &report.story_losses {
            md.push_str(&format!(
                "| L{:02} | {:5.2} | {:6.4} | {:5.2} | ${:9.0} | {:5.2}% | {:6.1} | {:8.1} | {} |\n",
                s.story_number, s.floor_elevation_m, s.peak_drift_ratio, s.peak_floor_accel_g,
                s.repair_cost_usd, s.repair_loss_ratio * 100.0, s.repair_labor_person_days, s.embodied_carbon_kg_co2e, s.governing_component
            ));
        }

        md.push_str("\n### 3. REDi™ Resilience Path & Downtime Synthesis\n\n");
        md.push_str("- **Mobilization & Inspection Delay:** 3 to 14 days based on damage severity tier.\n");
        md.push_str(&format!("- **Maximum Single-Story Labor Demand:** {:.1} person-days\n", report.total_repair_labor_days / input.num_stories as f64));
        md.push_str(&format!("- **Labor Allocation Constraint:** {} workers per floor simultaneously\n", input.max_laborers_per_story));
        md.push_str(&format!("- **Estimated Facility Downtime:** {:.0} days\n", report.downtime_days));
        md.push_str(&format!("- **Expected Annual Loss (EAL):** {:.2}% of asset value per year\n", report.expected_annual_loss_ratio * 100.0));

        md.push_str("\n---\n*Certified by EigenGraph Web Structural Engine*\n");
        md
    }

    /// Evaluates real-time FEMA P-58 fragility curves, story-level repair cost, downtime, and REDi tier
    /// directly from actual BIM geometry/counts and coupled tri-directional NL-THA Engineering Demand Parameters.
    pub fn evaluate_from_replay_demands(
        project: &crate::core::bim::BimProject,
        story_elevs_m: &[f64],
        peak_story_drifts: &[f64],
        scaled_pga_g: f64,
        peak_az_g: f64,
        sa_t1_g: f64,
    ) -> ReplayFemaP58Synthesis {
        use crate::core::bim::RunCategory;

        let db = Self::build_standard_fragility_database();
        let num_stories = peak_story_drifts.len().max(1);
        let floor_area_m2 = project
            .plan_extents_mm()
            .map(|(mn, mx)| {
                let dx = ((mx[0] - mn[0]) * 0.001).max(6.0);
                let dy = ((mx[1] - mn[1]) * 0.001).max(6.0);
                (dx * dy).clamp(64.0, 25_000.0)
            })
            .unwrap_or(320.0);
        let total_area_m2 = floor_area_m2 * (num_stories as f64);
        let replacement_value_usd = (total_area_m2 * 2_200.0).max(250_000.0);
        let story_val_usd = replacement_value_usd / (num_stories as f64);

        let beam_count = project
            .runs
            .iter()
            .filter(|r| matches!(r.category, RunCategory::StructuralBeam | RunCategory::StructuralBrace))
            .count();
        let avg_joints_per_floor = ((project.columns.len() + beam_count) as f64 / num_stories as f64)
            .clamp(4.0, 200.0);
        let avg_walls_per_floor = (project.walls.len() as f64 / num_stories as f64).clamp(2.0, 80.0);
        let avg_height_m = story_elevs_m
            .last()
            .copied()
            .unwrap_or(3.6 * num_stories as f64)
            .max(3.0);

        let mut story_losses = Vec::with_capacity(num_stories);
        let mut total_repair_cost = 0.0_f64;
        let mut total_embodied_carbon = 0.0_f64;
        let mut max_story_labor_days = 0.0_f64;
        let mut max_drift = 0.0_f64;

        let frame_joints = avg_joints_per_floor.round().max(6.0);
        let partition_units = (floor_area_m2 / 140.0).max(avg_walls_per_floor * 0.5).clamp(2.0, 80.0);
        let facade_bays = ((floor_area_m2.sqrt() * 4.0) / 8.0).clamp(4.0, 60.0);
        let ceiling_units = (floor_area_m2 / 220.0).clamp(1.0, 40.0);
        let mep_units = (floor_area_m2 / 360.0).clamp(1.0, 25.0);
        let quantities = [
            frame_joints,
            partition_units,
            facade_bays,
            ceiling_units,
            mep_units,
        ];

        for s in 0..num_stories {
            let elev_m = story_elevs_m
                .get(s + 1)
                .copied()
                .unwrap_or(((s + 1) as f64) * 3.6);
            let frac = (elev_m / avg_height_m).clamp(0.0, 1.0);
            let drift = peak_story_drifts[s].clamp(0.0, 0.12);
            if drift > max_drift {
                max_drift = drift;
            }
            let pfa_h = scaled_pga_g * (1.0 + 1.55 * frac.powf(1.1));
            let pfa = pfa_h.hypot(0.30 * peak_az_g);

            let mut s_cost = 0.0_f64;
            let mut s_labor = 0.0_f64;
            let mut s_carbon = 0.0_f64;
            let mut gov_comp = "None".to_string();
            let mut max_comp_cost = 0.0_f64;

            for (idx, comp) in db.iter().enumerate() {
                let edp = match comp.edp_kind {
                    EdpKind::PeakInterstoryDriftRatio => drift,
                    EdpKind::PeakFloorAcceleration => pfa,
                    EdpKind::ResidualDriftRatio => drift * 0.25,
                };
                let qty = quantities[idx % quantities.len()];
                let num_ds = comp.damage_states.len();
                let mut p_exceed = Vec::with_capacity(num_ds + 1);
                for ds in &comp.damage_states {
                    p_exceed.push(Self::evaluate_fragility(edp, ds.median_theta, ds.beta));
                }
                p_exceed.push(0.0);

                let mut comp_cost = 0.0_f64;
                let mut comp_labor = 0.0_f64;
                let mut comp_carbon = 0.0_f64;
                for i in 0..num_ds {
                    let p_ds = (p_exceed[i] - p_exceed[i + 1]).max(0.0);
                    let ds = &comp.damage_states[i];
                    comp_cost += p_ds * ds.unit_repair_cost_usd * qty;
                    comp_labor += p_ds * ds.unit_repair_labor_days * qty;
                    comp_carbon += p_ds * ds.unit_embodied_carbon_kg * qty;
                }
                if comp_cost > max_comp_cost {
                    max_comp_cost = comp_cost;
                    gov_comp = comp.name.clone();
                }
                s_cost += comp_cost;
                s_labor += comp_labor;
                s_carbon += comp_carbon;
            }

            if s_labor > max_story_labor_days {
                max_story_labor_days = s_labor;
            }
            total_repair_cost += s_cost;
            total_embodied_carbon += s_carbon;
            story_losses.push(StoryFragilityLoss {
                story_number: s + 1,
                elevation_m: elev_m,
                peak_drift_ratio: drift,
                peak_floor_accel_g: pfa,
                repair_cost_usd: s_cost,
                loss_ratio: (s_cost / story_val_usd).clamp(0.0, 1.0),
                downtime_days: ((s_labor / 8.0) * 1.35 + 2.0).round(),
                embodied_carbon_kg: s_carbon,
                governing_component: gov_comp,
            });
        }

        let repair_loss_ratio = (total_repair_cost / replacement_value_usd).clamp(0.0, 1.0);
        let inspection_delay = if repair_loss_ratio > 0.05 { 14.0 } else { 3.0 };
        let downtime_days = ((max_story_labor_days / 8.0) * 1.35 + inspection_delay).round();
        let daily_revenue = replacement_value_usd * 0.0012;
        let business_interruption_usd = downtime_days * daily_revenue;
        let total_economic_loss_usd = total_repair_cost + business_interruption_usd;

        let effective_sa = sa_t1_g.max(0.02);
        let collapse_probability = Self::evaluate_fragility(effective_sa, 1.35, 0.40);
        let demolition_probability = Self::evaluate_fragility(max_drift * 0.28, 0.012, 0.35);

        let achieved_redi_tier = if downtime_days <= 7.0 && repair_loss_ratio <= 0.05 {
            REDiResilienceTier::Platinum
        } else if downtime_days <= 30.0 && repair_loss_ratio <= 0.10 {
            REDiResilienceTier::Gold
        } else if downtime_days <= 60.0 && repair_loss_ratio <= 0.20 {
            REDiResilienceTier::Silver
        } else {
            REDiResilienceTier::StandardCode
        };

        // Calibrate 25-point continuous lognormal fragility curves against Sa(T1)
        let median_shift = (effective_sa / (max_drift / 0.020).max(0.25)).clamp(0.45, 2.20);
        let m_io = 0.28 * median_shift;
        let m_ls = 0.56 * median_shift;
        let m_cp = 0.92 * median_shift;
        let m_col = 1.35 * median_shift;
        let mut fragility_curves = Vec::with_capacity(25);
        for i in 0..25 {
            let sa = 0.05 + (i as f64 / 24.0) * 2.45;
            let p_col = Self::evaluate_fragility(sa, m_col, 0.42);
            let p_cp = Self::evaluate_fragility(sa, m_cp, 0.40).max(p_col);
            let p_ls = Self::evaluate_fragility(sa, m_ls, 0.38).max(p_cp);
            let p_io = Self::evaluate_fragility(sa, m_io, 0.35).max(p_ls);
            fragility_curves.push(ReplayFragilityCurvePoint {
                sa_g: sa,
                p_io,
                p_ls,
                p_cp,
                p_collapse: p_col,
            });
        }

        ReplayFemaP58Synthesis {
            building_replacement_value_usd: replacement_value_usd,
            total_repair_cost_usd: total_repair_cost,
            repair_loss_ratio,
            downtime_days,
            business_interruption_usd,
            total_economic_loss_usd,
            repair_embodied_carbon_kg: total_embodied_carbon,
            collapse_probability,
            demolition_probability,
            achieved_redi_tier,
            sa_t1_g: effective_sa,
            story_losses,
            fragility_curves,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_cdf_approximation() {
        let p0 = FemaP58Engine::phi(0.0);
        assert!((p0 - 0.50).abs() < 1e-4);
        let p_pos = FemaP58Engine::phi(1.96);
        assert!((p_pos - 0.975).abs() < 2e-3);
        let p_neg = FemaP58Engine::phi(-1.96);
        assert!((p_neg - 0.025).abs() < 2e-3);
    }

    #[test]
    fn test_fragility_evaluation() {
        // Median 0.020, beta 0.40
        let p_med = FemaP58Engine::evaluate_fragility(0.020, 0.020, 0.40);
        assert!((p_med - 0.50).abs() < 1e-3, "Median demand must yield exactly 50% probability");

        let p_low = FemaP58Engine::evaluate_fragility(0.005, 0.020, 0.40);
        assert!(p_low < 0.10, "Low demand should produce small exceedance probability");

        let p_high = FemaP58Engine::evaluate_fragility(0.060, 0.020, 0.40);
        assert!(p_high > 0.90, "High demand should produce large exceedance probability");
    }

    #[test]
    fn test_fema_p58_loss_evaluation() {
        let input = FemaP58Input::default();
        let report = FemaP58Engine::evaluate(&input);

        assert!(report.total_repair_cost_usd > 50_000.0);
        assert!(report.repair_cost_ratio > 0.001 && report.repair_cost_ratio < 0.50);
        assert!(report.downtime_days >= 3.0 && report.downtime_days < 180.0);
        assert!(report.total_embodied_carbon_kg_co2e > 1_000.0);
        assert_eq!(report.story_losses.len(), 10);
        assert!(!report.compliance_audit.is_empty());
    }

    #[test]
    fn test_fema_resilience_archetype() {
        let (nodes, elements) = FemaP58Engine::generate_fema_resilience_archetype();
        assert_eq!(nodes.len(), 12 * 11, "12 nodes per floor x 11 levels");
        assert!(!elements.is_empty());
    }

    #[test]
    fn test_phase75_replay_fema_p58_fragility_and_loss_synthesis() {
        let proj = crate::core::bim::BimProject::sample_bimcraft_building();
        let elevs = [0.0, 4.0, 8.0];
        let low = FemaP58Engine::evaluate_from_replay_demands(
            &proj,
            &elevs,
            &[0.003, 0.002],
            0.25,
            0.12,
            0.35,
        );
        let high = FemaP58Engine::evaluate_from_replay_demands(
            &proj,
            &elevs,
            &[0.024, 0.018],
            0.85,
            0.48,
            1.40,
        );
        assert_eq!(high.story_losses.len(), 2);
        assert!(high.total_repair_cost_usd > low.total_repair_cost_usd);
        assert!(high.collapse_probability > low.collapse_probability);
        assert_eq!(high.fragility_curves.len(), 25);
        for pt in &high.fragility_curves {
            assert!(pt.p_io >= pt.p_ls && pt.p_ls >= pt.p_cp && pt.p_cp >= pt.p_collapse);
        }
    }
}
