//! cascade.rs — Dynamic Cascading Collapse & Progressive Collapse Engine.
//! Implements DoD UFC 4-023-03 and ASCE 7-22 Alternate Path Method.
//! Simulates instantaneous member loss, dynamic load amplification (DAF),
//! sequential domino effect of yielding/overloaded members, and computes
//! Propagation Vulnerability Index (PVI) and System Robustness Index (R_sys).

use crate::core::ari::AriEngine;
use crate::core::model::StructuralModel;
use crate::core::solver::FemSolver;

/// Status of the cascading progressive collapse simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollapseStatus {
    /// Structure redistributed loads successfully and collapse was arrested (No further members failed)
    ArrestedSafe,
    /// Partial sequential failure occurred but stabilized before total collapse
    ArrestedPartialCollapse,
    /// Kinematic mechanism formed (rank drop) or domino failure destroyed structural stability
    CatastrophicCollapse,
}

/// A single step in the cascading domino failure chronology
#[derive(Debug, Clone, PartialEq)]
pub struct CollapseStep {
    pub step_index: usize,
    pub failed_member_ids: Vec<usize>,
    pub max_dcr: f64,
    pub max_displacement_mm: f64,
    pub description: String,
}

/// Comprehensive Progressive Collapse Simulation Report
#[derive(Debug, Clone, PartialEq)]
pub struct CascadingCollapseReport {
    pub initial_disrupted_ids: Vec<usize>,
    pub total_collapsed_ids: Vec<usize>,
    pub status: CollapseStatus,
    pub daf: f64,
    pub steps_count: usize,
    pub pvi: f64,
    pub robustness_index: f64,
    pub residual_ari: f64,
    pub max_redistributed_dcr: f64,
    pub chronology: Vec<CollapseStep>,
}

pub struct CascadingSimulator;

impl CascadingSimulator {
    /// Simulates instantaneous disruption of specified members and tracks progressive failure
    pub fn simulate_disruption(
        model: &StructuralModel,
        initial_failed_ids: &[usize],
        daf: Option<f64>,
    ) -> Result<CascadingCollapseReport, String> {
        let daf_factor = daf.unwrap_or(1.20); // Standard UFC 4-023-03 dynamic amplification

        if model.nodes.is_empty() || model.elements.is_empty() {
            return Err("Model has no elements to simulate".to_string());
        }

        let initial_elem_count = model.elements.len();
        let mut active_model = model.clone();
        let mut all_failed = initial_failed_ids.to_vec();
        let mut chronology = Vec::new();

        // 1. Initial Step: Remove initial failed elements
        active_model.elements.retain(|e| !initial_failed_ids.contains(&e.id));

        // Scale loads on nodes connected to initial failed elements by DAF
        let mut affected_nodes = std::collections::HashSet::new();
        for elem in &model.elements {
            if initial_failed_ids.contains(&elem.id) {
                affected_nodes.insert(elem.start_node);
                affected_nodes.insert(elem.end_node);
            }
        }

        for load in &mut active_model.loads {
            if affected_nodes.contains(&load.node_id) {
                load.fx *= daf_factor;
                load.fy *= daf_factor;
                load.fz *= daf_factor;
            }
        }

        chronology.push(CollapseStep {
            step_index: 1,
            failed_member_ids: initial_failed_ids.to_vec(),
            max_dcr: 0.0,
            max_displacement_mm: 0.0,
            description: format!(
                "Initial trigger disruption: Members {:?} severed instantaneously. DAF = {:.2}x applied to adjacent joints.",
                initial_failed_ids, daf_factor
            ),
        });

        let mut step = 1;
        let mut max_dcr_recorded = 0.0;
        let final_status;

        let max_iterations = 20;

        loop {
            step += 1;
            if step > max_iterations {
                final_status = CollapseStatus::CatastrophicCollapse;
                break;
            }

            if active_model.elements.is_empty() {
                final_status = CollapseStatus::CatastrophicCollapse;
                break;
            }

            // Attempt FEM equilibrium solve
            let solve_result = FemSolver::solve(&active_model);
            let analysis = match solve_result {
                Ok(res) => res,
                Err(err) => {
                    chronology.push(CollapseStep {
                        step_index: step,
                        failed_member_ids: Vec::new(),
                        max_dcr: f64::INFINITY,
                        max_displacement_mm: f64::INFINITY,
                        description: format!("Kinematic instability detected: {}. Collapse occurred.", err),
                    });
                    final_status = CollapseStatus::CatastrophicCollapse;
                    break;
                }
            };

            // Gate 1 SVD Rank Stability Check
            let (_, s_vec, _) = analysis.k_free.svd();
            let mut zero_singular = 0;
            let rank_tol = 1e-7;
            for &s in &s_vec {
                if s < rank_tol {
                    zero_singular += 1;
                }
            }

            if zero_singular > 0 {
                chronology.push(CollapseStep {
                    step_index: step,
                    failed_member_ids: Vec::new(),
                    max_dcr: f64::INFINITY,
                    max_displacement_mm: analysis.max_displacement_mm,
                    description: format!(
                        "Gate 1 SVD Stability Failure: {} rigid-body mechanism mode(s) detected (dim(Null(K)) > 0).",
                        zero_singular
                    ),
                });
                final_status = CollapseStatus::CatastrophicCollapse;
                break;
            }

            // Evaluate DCR on all active members
            let mut newly_yielded = Vec::new();
            let mut step_max_dcr = 0.0;

            for (idx, elem) in active_model.elements.iter().enumerate() {
                let dcr = analysis.member_dcr_interactions.get(idx).copied().unwrap_or(0.0);
                if dcr > step_max_dcr {
                    step_max_dcr = dcr;
                }
                if dcr > 1.0 {
                    newly_yielded.push((elem.id, dcr));
                }
            }

            if step_max_dcr > max_dcr_recorded {
                max_dcr_recorded = step_max_dcr;
            }

            // If no new members yielded/ruptured, equilibrium is arrested!
            if newly_yielded.is_empty() {
                if all_failed.len() == initial_failed_ids.len() {
                    final_status = CollapseStatus::ArrestedSafe;
                } else {
                    final_status = CollapseStatus::ArrestedPartialCollapse;
                }
                chronology.push(CollapseStep {
                    step_index: step,
                    failed_member_ids: Vec::new(),
                    max_dcr: step_max_dcr,
                    max_displacement_mm: analysis.max_displacement_mm,
                    description: format!(
                        "Equilibrium arrested: Max redistributed DCR = {:.2}%. Displaced shape stable ({:.2} mm).",
                        step_max_dcr * 100.0, analysis.max_displacement_mm
                    ),
                });
                break;
            }

            // Sort yielded members by DCR descending
            newly_yielded.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let yielded_ids: Vec<usize> = newly_yielded.iter().map(|item| item.0).collect();

            chronology.push(CollapseStep {
                step_index: step,
                failed_member_ids: yielded_ids.clone(),
                max_dcr: step_max_dcr,
                max_displacement_mm: analysis.max_displacement_mm,
                description: format!(
                    "Domino Failure: {} member(s) {:?} exceeded plastic capacity (Max DCR = {:.1}%).",
                    yielded_ids.len(), yielded_ids, step_max_dcr * 100.0
                ),
            });

            for &id in &yielded_ids {
                all_failed.push(id);
            }

            active_model.elements.retain(|e| !yielded_ids.contains(&e.id));
        }

        let total_failed_count = all_failed.len();
        let pvi = (total_failed_count as f64 / initial_elem_count as f64).clamp(0.0, 1.0);
        let robustness = (1.0 - pvi).clamp(0.0, 1.0);

        let residual_ari = if final_status == CollapseStatus::CatastrophicCollapse {
            0.0
        } else {
            AriEngine::evaluate_redundancy_fast(&active_model)
                .map(|r| r.average_ari)
                .unwrap_or(0.0)
        };

        Ok(CascadingCollapseReport {
            initial_disrupted_ids: initial_failed_ids.to_vec(),
            total_collapsed_ids: all_failed,
            status: final_status,
            daf: daf_factor,
            steps_count: chronology.len(),
            pvi,
            robustness_index: robustness,
            residual_ari,
            max_redistributed_dcr: max_dcr_recorded,
            chronology,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::NodalLoad;
    use crate::core::parametric::ParametricEngine;

    #[test]
    fn test_cascading_simulation_warren_single_disruption() {
        let mut engine = ParametricEngine::new();
        let mut model = engine.generate_warren_truss().expect("Generate Warren");
        model.add_load(NodalLoad::force_y(3, -50_000.0));

        // Sever non-critical element 18 (first vertical post)
        let report = CascadingSimulator::simulate_disruption(&model, &[18], Some(1.20))
            .expect("Simulation should complete");

        assert!(report.daf >= 1.0);
        assert!(report.pvi >= 0.0 && report.pvi <= 1.0);
        assert!(report.robustness_index >= 0.0 && report.robustness_index <= 1.0);
        assert!(!report.chronology.is_empty());
    }
}
