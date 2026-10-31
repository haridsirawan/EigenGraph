//! optimizer.rs — AI-ARI Topology & Bracing Optimization Engine.
//! Discovers optimal bracing candidates to eliminate fracture-critical single points of failure (ARI < 0.70).
//! Uses Sherman-Morrison rank-1 perturbation evaluator to search Pareto-optimal bracing topologies.

use crate::core::ari::AriEngine;
use crate::core::model::{CrossSection, Element3D, ElementType, Material, StructuralModel};

#[derive(Debug, Clone, PartialEq)]
pub struct BracingCandidate {
    pub from_node: usize,
    pub to_node: usize,
    pub section_name: String,
    pub delta_min_ari: f64,
    pub new_min_ari: f64,
    pub added_weight_kg: f64,
    pub efficiency_score: f64,
    pub is_mechanism_eliminated: bool,
    pub explanation: String,
}

#[derive(Debug, Clone)]
pub struct BracingOptimizationReport {
    pub baseline_min_ari: f64,
    pub critical_members_count: usize,
    pub candidates_evaluated: usize,
    pub recommendations: Vec<BracingCandidate>,
    pub best_candidate: Option<BracingCandidate>,
}

pub struct AriTopologyOptimizer;

impl AriTopologyOptimizer {
    /// Discovers and ranks candidate bracing elements to maximize system redundancy
    pub fn optimize_bracing(
        model: &StructuralModel,
        search_radius_m: Option<f64>,
    ) -> Result<BracingOptimizationReport, String> {
        let baseline = AriEngine::evaluate_redundancy_fast(model)?;
        let baseline_min = baseline.min_system_ari;

        let max_dist = search_radius_m.unwrap_or(12.0);
        let n_nodes = model.nodes.len();

        let mut existing_pairs = std::collections::HashSet::new();
        for elem in &model.elements {
            let u = elem.start_node.min(elem.end_node);
            let v = elem.start_node.max(elem.end_node);
            existing_pairs.insert((u, v));
        }

        let mut candidates = Vec::new();
        let mut evaluated_count = 0;

        let brace_section = CrossSection::hss8x8();
        let steel = Material::structural_steel();

        for i in 0..n_nodes {
            for j in (i + 1)..n_nodes {
                let u = model.nodes[i].id;
                let v = model.nodes[j].id;

                let pair_key = (u.min(v), u.max(v));
                if existing_pairs.contains(&pair_key) {
                    continue;
                }

                let dist = model.nodes[i].distance_to(&model.nodes[j]);
                if dist < 0.5 || dist > max_dist {
                    continue;
                }

                evaluated_count += 1;

                let mut candidate_model = model.clone();
                let new_elem_id = candidate_model.elements.iter().map(|e| e.id).max().unwrap_or(0) + 1;
                let mut new_elem = Element3D::new(
                    new_elem_id,
                    format!("AI_Brace_{}_{}", u, v),
                    u,
                    v,
                );
                new_elem.element_type = ElementType::Truss3D;
                new_elem.section = brace_section.clone();
                new_elem.material = steel.clone();
                candidate_model.add_element(new_elem);

                if let Ok(eval) = AriEngine::evaluate_redundancy_fast(&candidate_model) {
                    let delta_ari = eval.min_system_ari - baseline_min;
                    let added_weight = dist * brace_section.area * steel.density;
                    let eff = if added_weight > 0.0 { (delta_ari * 1000.0) / added_weight } else { 0.0 };

                    if delta_ari > 0.001 || eval.critical_members_count < baseline.critical_members_count {
                        candidates.push(BracingCandidate {
                            from_node: u,
                            to_node: v,
                            section_name: brace_section.name.clone(),
                            delta_min_ari: delta_ari,
                            new_min_ari: eval.min_system_ari,
                            added_weight_kg: added_weight,
                            efficiency_score: eff,
                            is_mechanism_eliminated: eval.critical_members_count < baseline.critical_members_count,
                            explanation: format!(
                                "Adding cross-brace ({}, {}) increases system minimum ARI from {:.3} to {:.3} (+{:.1}%), reducing critical members from {} to {}.",
                                u, v, baseline_min, eval.min_system_ari, delta_ari * 100.0, baseline.critical_members_count, eval.critical_members_count
                            ),
                        });
                    }
                }
            }
        }

        candidates.sort_by(|a, b| {
            b.delta_min_ari.partial_cmp(&a.delta_min_ari).unwrap_or(std::cmp::Ordering::Equal)
        });

        let best = candidates.first().cloned();

        Ok(BracingOptimizationReport {
            baseline_min_ari: baseline_min,
            critical_members_count: baseline.critical_members_count,
            candidates_evaluated: evaluated_count,
            recommendations: candidates,
            best_candidate: best,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parametric::ParametricEngine;
    use crate::core::model::NodalLoad;

    #[test]
    fn test_bracing_optimizer_suggestion() {
        let mut engine = ParametricEngine::new();
        let mut model = engine.generate_warren_truss().expect("Generate Warren");
        model.add_load(NodalLoad::force_y(3, -50_000.0));

        let opt_report = AriTopologyOptimizer::optimize_bracing(&model, Some(10.0))
            .expect("Bracing optimization should run");

        assert!(opt_report.candidates_evaluated > 0);
        // If candidates found, verify delta_min_ari is non-negative
        if let Some(best) = &opt_report.best_candidate {
            assert!(best.new_min_ari >= opt_report.baseline_min_ari);
            assert!(best.added_weight_kg > 0.0);
        }
    }
}
