//! cost.rs — BIM 5D Quantity Takeoff (QTO) & Dynamic AHSP Cost Engine.
//! Integrates structural steel metrics with Indonesian AHSP (Permen PUPR) unit rates,
//! calculates itemized Bill of Quantities (BOQ), and models Cost vs. ARI Trade-off curves.

use std::collections::HashMap;
use crate::core::model::StructuralModel;

/// Unit price rates based on Indonesian AHSP (Kementerian PUPR) & Market Averages
#[derive(Debug, Clone, PartialEq)]
pub struct AhspUnitRates {
    pub steel_material_idr_per_kg: f64,
    pub workshop_fabrication_idr_per_kg: f64,
    pub transport_idr_per_kg: f64,
    pub erection_crane_idr_per_kg: f64,
    pub coating_paint_idr_per_m2: f64,
    pub gusset_hardware_factor: f64,
    pub overhead_profit_ratio: f64,
    pub vat_ratio: f64,
}

impl Default for AhspUnitRates {
    fn default() -> Self {
        Self {
            steel_material_idr_per_kg: 24_500.0,
            workshop_fabrication_idr_per_kg: 8_500.0,
            transport_idr_per_kg: 2_500.0,
            erection_crane_idr_per_kg: 6_500.0,
            coating_paint_idr_per_m2: 45_000.0,
            gusset_hardware_factor: 0.08,
            overhead_profit_ratio: 0.10,
            vat_ratio: 0.11,
        }
    }
}

/// Profile-level Quantity Takeoff breakdown
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileQuantity {
    pub section_name: String,
    pub count: usize,
    pub total_length_m: f64,
    pub total_mass_kg: f64,
    pub surface_area_m2: f64,
}

/// Itemized line item in the Bill of Quantities (BOQ)
#[derive(Debug, Clone, PartialEq)]
pub struct BoqLineItem {
    pub item_code: String,
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub unit_price_idr: f64,
    pub total_price_idr: f64,
}

/// Comprehensive BIM 5D Cost Report
#[derive(Debug, Clone, PartialEq)]
pub struct CostEstimationReport {
    pub total_profile_steel_mass_kg: f64,
    pub total_connection_plates_kg: f64,
    pub grand_total_steel_tonnage: f64,
    pub total_surface_area_m2: f64,
    pub profile_takeoff: Vec<ProfileQuantity>,
    pub boq_items: Vec<BoqLineItem>,
    pub items: Vec<BoqLineItem>,
    pub direct_construction_cost_idr: f64,
    pub overhead_and_profit_idr: f64,
    pub vat_idr: f64,
    pub final_project_cost_idr: f64,
    pub total_cost_idr: f64,
}

pub struct CostEngine;

impl CostEngine {
    pub fn format_idr(val: f64) -> String {
        let rounded = val.round() as i64;
        let s = rounded.to_string();
        let mut result = String::new();
        let mut count = 0;
        for c in s.chars().rev() {
            if count > 0 && count % 3 == 0 {
                result.push('.');
            }
            result.push(c);
            count += 1;
        }
        result.chars().rev().collect()
    }
    /// Computes full BIM 5D Quantity Takeoff (QTO) & Rencana Anggaran Biaya (RAB)
    pub fn calculate_boq(model: &StructuralModel, rates: Option<&AhspUnitRates>) -> CostEstimationReport {
        let r = rates.cloned().unwrap_or_default();

        let mut profile_map: HashMap<String, (usize, f64, f64, f64)> = HashMap::new();
        let mut total_profile_mass = 0.0;
        let mut total_surface = 0.0;

        for elem in &model.elements {
            if let Some(len) = model.member_length(elem) {
                let mass = elem.section.area * len * elem.material.density;
                let perimeter = 2.0 * (elem.section.depth + elem.section.width);
                let surface = perimeter * len;

                total_profile_mass += mass;
                total_surface += surface;

                let entry = profile_map.entry(elem.section.name.clone()).or_insert((0, 0.0, 0.0, 0.0));
                entry.0 += 1;
                entry.1 += len;
                entry.2 += mass;
                entry.3 += surface;
            }
        }

        let mut profile_takeoff = Vec::new();
        for (sec_name, (cnt, len, mass, surf)) in profile_map {
            profile_takeoff.push(ProfileQuantity {
                section_name: sec_name,
                count: cnt,
                total_length_m: len,
                total_mass_kg: mass,
                surface_area_m2: surf,
            });
        }
        profile_takeoff.sort_by(|a, b| b.total_mass_kg.partial_cmp(&a.total_mass_kg).unwrap());

        let connection_plates_mass = total_profile_mass * r.gusset_hardware_factor;
        let all_steel_mass = total_profile_mass + connection_plates_mass;
        let all_steel_tonnage = all_steel_mass / 1000.0;

        let mut boq_items = Vec::new();

        // 1. Material Pengadaan Profil Baja
        let cost_mat_profiles = total_profile_mass * r.steel_material_idr_per_kg;
        boq_items.push(BoqLineItem {
            item_code: "DIV-05-101".to_string(),
            description: "Pengadaan Material Profil Baja Struktural (AISC/SNI)".to_string(),
            quantity: total_profile_mass,
            unit: "kg".to_string(),
            unit_price_idr: r.steel_material_idr_per_kg,
            total_price_idr: cost_mat_profiles,
        });

        // 2. Material Pelat Sambung & Buhul Gusset
        let cost_mat_gusset = connection_plates_mass * (r.steel_material_idr_per_kg * 1.05);
        boq_items.push(BoqLineItem {
            item_code: "DIV-05-102".to_string(),
            description: "Pengadaan Pelat Sambung, Gusset Plate & Hardware (+8%)".to_string(),
            quantity: connection_plates_mass,
            unit: "kg".to_string(),
            unit_price_idr: r.steel_material_idr_per_kg * 1.05,
            total_price_idr: cost_mat_gusset,
        });

        // 3. Jasa Fabrikasi Workshop
        let cost_fab = all_steel_mass * r.workshop_fabrication_idr_per_kg;
        boq_items.push(BoqLineItem {
            item_code: "DIV-05-201".to_string(),
            description: "Jasa Fabrikasi Workshop (Pemotongan, Pengeboran, Las CO2)".to_string(),
            quantity: all_steel_mass,
            unit: "kg".to_string(),
            unit_price_idr: r.workshop_fabrication_idr_per_kg,
            total_price_idr: cost_fab,
        });

        // 4. Transportasi Ekspedisi Jobsite
        let cost_trans = all_steel_mass * r.transport_idr_per_kg;
        boq_items.push(BoqLineItem {
            item_code: "DIV-05-301".to_string(),
            description: "Mobilisasi & Transportasi Trailer ke Lokasi Proyek".to_string(),
            quantity: all_steel_mass,
            unit: "kg".to_string(),
            unit_price_idr: r.transport_idr_per_kg,
            total_price_idr: cost_trans,
        });

        // 5. Erection & Instalasi Mobile Crane
        let cost_erection = all_steel_mass * r.erection_crane_idr_per_kg;
        boq_items.push(BoqLineItem {
            item_code: "DIV-05-401".to_string(),
            description: "Jasa Ereksi Struktur Lapangan & Sewa Mobile Crane".to_string(),
            quantity: all_steel_mass,
            unit: "kg".to_string(),
            unit_price_idr: r.erection_crane_idr_per_kg,
            total_price_idr: cost_erection,
        });

        // 6. Pengecatan Primer Anti-Karat & Finishing
        let cost_paint = total_surface * r.coating_paint_idr_per_m2;
        boq_items.push(BoqLineItem {
            item_code: "DIV-09-901".to_string(),
            description: "Proteksi Permukaan: Primer Zinc Chromate & Finishing".to_string(),
            quantity: total_surface,
            unit: "m2".to_string(),
            unit_price_idr: r.coating_paint_idr_per_m2,
            total_price_idr: cost_paint,
        });

        let direct_cost: f64 = boq_items.iter().map(|item| item.total_price_idr).sum();
        let overhead_profit = direct_cost * r.overhead_profit_ratio;
        let vat = (direct_cost + overhead_profit) * r.vat_ratio;
        let grand_total = direct_cost + overhead_profit + vat;

        CostEstimationReport {
            total_profile_steel_mass_kg: total_profile_mass,
            total_connection_plates_kg: connection_plates_mass,
            grand_total_steel_tonnage: all_steel_tonnage,
            total_surface_area_m2: total_surface,
            profile_takeoff,
            boq_items: boq_items.clone(),
            items: boq_items,
            direct_construction_cost_idr: direct_cost,
            overhead_and_profit_idr: overhead_profit,
            vat_idr: vat,
            final_project_cost_idr: grand_total,
            total_cost_idr: grand_total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parametric::ParametricEngine;

    #[test]
    fn test_cost_engine_warren_boq() {
        let mut engine = ParametricEngine::new();
        let model = engine.generate_warren_truss().expect("Generate Warren");
        let rep = CostEngine::calculate_boq(&model, None);

        assert!(rep.grand_total_steel_tonnage > 0.0);
        assert!(rep.final_project_cost_idr > 0.0);
        assert_eq!(rep.boq_items.len(), 6);
        assert!(rep.profile_takeoff.len() >= 2);
    }
}
