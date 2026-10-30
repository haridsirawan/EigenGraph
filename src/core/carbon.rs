//! carbon.rs — Embodied Carbon & Sustainability Life Cycle Assessment (LCA) Engine.
//! Implements ISO 14040/14044 and EN 15978 Cradle-to-Grave environmental scoring.
//! Tracks Embodied Carbon (kg CO2e / ton CO2e) across Stages A1-A3 (Manufacturing),
//! A4 (Transport to Site), and A5 (Installation & Erection).
//! Evaluates Carbon vs. Redundancy Trade-Off (Eco-Redundancy Efficiency) and GBCI/LEED ratings.

use crate::core::model::StructuralModel;

/// Steel Manufacturing & Smelting Route for LCA
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarbonSteelRoute {
    /// Blast Furnace - Basic Oxygen Furnace (Primary Virgin Ore, high emission)
    BfBofStandard,
    /// Electric Arc Furnace (90%+ Recycled Scrap, low carbon)
    EafRecycled,
    /// Green Hydrogen Direct Reduced Iron (DRI-H2, near-zero emission)
    GreenHydrogenDri,
}

impl CarbonSteelRoute {
    /// Returns A1-A3 emission factor in kg CO2e per kg of structural steel
    pub fn emission_factor_kg_per_kg(&self) -> f64 {
        match self {
            CarbonSteelRoute::BfBofStandard => 2.20,
            CarbonSteelRoute::EafRecycled => 0.65,
            CarbonSteelRoute::GreenHydrogenDri => 0.28,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CarbonSteelRoute::BfBofStandard => "Standard BF-BOF (Virgin Ore)",
            CarbonSteelRoute::EafRecycled => "EAF (High Recycled Scrap)",
            CarbonSteelRoute::GreenHydrogenDri => "Green Hydrogen DRI-H2",
        }
    }
}

/// Life Cycle Stage Breakdown (EN 15978)
#[derive(Debug, Clone, PartialEq)]
pub struct CarbonLcaStage {
    pub stage_code: String,
    pub stage_name: String,
    pub emission_ton_co2e: f64,
    pub percentage: f64,
}

/// Comprehensive Embodied Carbon & Sustainability Report
#[derive(Debug, Clone, PartialEq)]
pub struct CarbonLcaReport {
    pub steel_tonnage: f64,
    pub route: CarbonSteelRoute,
    pub transport_distance_km: f64,
    pub a1_a3_emissions_ton: f64,
    pub a4_transport_emissions_ton: f64,
    pub a5_erection_emissions_ton: f64,
    pub total_embodied_carbon_ton: f64,
    pub baseline_bf_bof_carbon_ton: f64,
    pub carbon_reduction_pct: f64,
    pub greenship_leed_tier: String,
    pub eco_redundancy_efficiency: f64,
    pub stages: Vec<CarbonLcaStage>,
}

pub struct CarbonEngine;

impl CarbonEngine {
    /// Evaluates full Cradle-to-Site (A1–A5) Embodied Carbon for a structural model
    pub fn calculate_lca(
        model: &StructuralModel,
        route: CarbonSteelRoute,
        transport_distance_km: Option<f64>,
        system_ari: Option<f64>,
    ) -> CarbonLcaReport {
        let steel_tonnage = model.total_steel_tonnage();
        let steel_kg = steel_tonnage * 1000.0;
        let dist_km = transport_distance_km.unwrap_or(65.0); // Default Jakarta-Bandung or regional fabrication distance
        let ari_val = system_ari.unwrap_or(0.942);

        // A1-A3 Manufacturing: kg CO2e = kg_steel * factor
        let a1_a3_kg = steel_kg * route.emission_factor_kg_per_kg();
        let a1_a3_ton = a1_a3_kg / 1000.0;

        // Baseline comparison using BF-BOF standard
        let baseline_a1_a3_kg = steel_kg * CarbonSteelRoute::BfBofStandard.emission_factor_kg_per_kg();

        // A4 Transport to Site: 0.082 kg CO2e / (ton * km) (Euro 5 Diesel Haulage Truck)
        let a4_kg = steel_tonnage * dist_km * 0.082;
        let a4_ton = a4_kg / 1000.0;

        // A5 Erection & Cranage: 26.5 kg CO2e / ton erected (Mobile telescopic crane + site genset)
        let a5_kg = steel_tonnage * 26.5;
        let a5_ton = a5_kg / 1000.0;

        let total_ton = a1_a3_ton + a4_ton + a5_ton;
        let baseline_total_ton = (baseline_a1_a3_kg + a4_kg + a5_kg) / 1000.0;

        let reduction_pct = if baseline_total_ton > 1e-6 {
            ((baseline_total_ton - total_ton) / baseline_total_ton * 100.0).max(0.0)
        } else {
            0.0
        };

        // GBCI (Green Building Council Indonesia) Greenship & LEED v4.1 BD+C Tier
        let tier = if reduction_pct >= 55.0 {
            "LEED Platinum / GBCI Platinum (Outstanding Carbon Mitigation)".to_string()
        } else if reduction_pct >= 40.0 {
            "LEED Gold / GBCI Gold (Significant Carbon Optimization)".to_string()
        } else if reduction_pct >= 25.0 {
            "LEED Silver / GBCI Silver (Moderate Carbon Reduction)".to_string()
        } else if reduction_pct >= 10.0 {
            "LEED Certified / GBCI Bronze (Basic Carbon Reduction)".to_string()
        } else {
            "Baseline Conventional (Standard Industrial Carbon Footprint)".to_string()
        };

        // Eco-Redundancy Efficiency: System ARI / Total tCO2e
        let eco_efficiency = if total_ton > 1e-6 {
            ari_val / total_ton
        } else {
            0.0
        };

        let stages = vec![
            CarbonLcaStage {
                stage_code: "A1-A3".to_string(),
                stage_name: "Raw Material Extraction & Fabrication".to_string(),
                emission_ton_co2e: a1_a3_ton,
                percentage: if total_ton > 0.0 { (a1_a3_ton / total_ton) * 100.0 } else { 0.0 },
            },
            CarbonLcaStage {
                stage_code: "A4".to_string(),
                stage_name: format!("Transport to Site ({} km)", dist_km as usize),
                emission_ton_co2e: a4_ton,
                percentage: if total_ton > 0.0 { (a4_ton / total_ton) * 100.0 } else { 0.0 },
            },
            CarbonLcaStage {
                stage_code: "A5".to_string(),
                stage_name: "Installation & Heavy Cranage Erection".to_string(),
                emission_ton_co2e: a5_ton,
                percentage: if total_ton > 0.0 { (a5_ton / total_ton) * 100.0 } else { 0.0 },
            },
        ];

        CarbonLcaReport {
            steel_tonnage,
            route,
            transport_distance_km: dist_km,
            a1_a3_emissions_ton: a1_a3_ton,
            a4_transport_emissions_ton: a4_ton,
            a5_erection_emissions_ton: a5_ton,
            total_embodied_carbon_ton: total_ton,
            baseline_bf_bof_carbon_ton: baseline_total_ton,
            carbon_reduction_pct: reduction_pct,
            greenship_leed_tier: tier,
            eco_redundancy_efficiency: eco_efficiency,
            stages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parametric::ParametricEngine;

    #[test]
    fn test_carbon_lca_warren_truss() {
        let mut engine = ParametricEngine::new();
        let model = engine.generate_warren_truss().expect("Generate Warren");

        let report_bof = CarbonEngine::calculate_lca(&model, CarbonSteelRoute::BfBofStandard, Some(50.0), Some(0.942));
        assert!(report_bof.total_embodied_carbon_ton > 0.0);
        assert_eq!(report_bof.carbon_reduction_pct, 0.0);

        let report_eaf = CarbonEngine::calculate_lca(&model, CarbonSteelRoute::EafRecycled, Some(50.0), Some(0.942));
        assert!(report_eaf.total_embodied_carbon_ton < report_bof.total_embodied_carbon_ton);
        assert!(report_eaf.carbon_reduction_pct > 50.0);
        assert!(report_eaf.eco_redundancy_efficiency > report_bof.eco_redundancy_efficiency);
    }
}
