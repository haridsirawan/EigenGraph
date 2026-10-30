//! schedule.rs — BIM 4D Construction Scheduling & Staged Erection SVD Guard.
//! Implements Critical Path Method (CPM), Early/Late Start/Finish calculation,
//! Total Float determination, weekly S-Curve physical & cashflow projection,
//! and SVD Kinematic Guard evaluation across chronological erection stages.

use crate::core::model::StructuralModel;
use crate::core::solver::FemSolver;

/// A single construction activity in the 4D Work Breakdown Structure (WBS)
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructionActivity {
    pub id: usize,
    pub name: String,
    pub duration_days: usize,
    pub predecessors: Vec<usize>,
    pub early_start: usize,
    pub early_finish: usize,
    pub late_start: usize,
    pub late_finish: usize,
    pub total_float: usize,
    pub free_float: usize,
    pub is_critical: bool,
    pub weight_percentage: f64,
}

/// S-Curve weekly milestone data point
#[derive(Debug, Clone, PartialEq)]
pub struct SCurvePoint {
    pub week_number: usize,
    pub day_number: usize,
    pub weekly_planned_pct: f64,
    pub cumulative_planned_pct: f64,
    pub weekly_cost_idr: f64,
    pub cumulative_cost_idr: f64,
}

/// Evaluation of structural stability at a specific erection stage
#[derive(Debug, Clone, PartialEq)]
pub struct StageStabilityResult {
    pub stage_index: usize,
    pub stage_name: String,
    pub erected_elements_count: usize,
    pub active_dofs: usize,
    pub rank: usize,
    pub is_stable: bool,
    pub condition_number: f64,
    pub min_singular_value: f64,
    pub max_deflection_mm: f64,
    pub warning: Option<String>,
}

/// Complete BIM 4D Schedule and Staged Erection Report
#[derive(Debug, Clone, PartialEq)]
pub struct Schedule4DReport {
    pub total_duration_days: usize,
    pub critical_path_activity_ids: Vec<usize>,
    pub activities: Vec<ConstructionActivity>,
    pub scurve: Vec<SCurvePoint>,
    pub staged_stability: Vec<StageStabilityResult>,
    pub is_erection_sequence_safe: bool,
}

pub struct ScheduleEngine;

impl ScheduleEngine {
    /// Computes Critical Path Method (CPM) for a given set of activities
    pub fn compute_cpm(mut activities: Vec<ConstructionActivity>) -> Result<Vec<ConstructionActivity>, String> {
        if activities.is_empty() {
            return Err("Activity list is empty".to_string());
        }

        let n = activities.len();

        // 1. Forward Pass: Compute Early Start (ES) and Early Finish (EF)
        for i in 0..n {
            let pred_ef_max = activities[i]
                .predecessors
                .iter()
                .filter_map(|&pred_id| activities.iter().find(|a| a.id == pred_id))
                .map(|pred| pred.early_finish)
                .max()
                .unwrap_or(0);

            activities[i].early_start = pred_ef_max;
            activities[i].early_finish = activities[i].early_start + activities[i].duration_days;
        }

        // Project duration is max of all early finishes
        let total_project_duration = activities
            .iter()
            .map(|a| a.early_finish)
            .max()
            .unwrap_or(0);

        // 2. Backward Pass: Compute Late Finish (LF) and Late Start (LS)
        for i in (0..n).rev() {
            let act_id = activities[i].id;
            let act_ef = activities[i].early_finish;
            let act_dur = activities[i].duration_days;

            let mut succ_ls_min = total_project_duration;
            let mut succ_es_min = total_project_duration;
            let mut has_successors = false;

            for a in &activities {
                if a.predecessors.contains(&act_id) {
                    has_successors = true;
                    if a.late_start < succ_ls_min {
                        succ_ls_min = a.late_start;
                    }
                    if a.early_start < succ_es_min {
                        succ_es_min = a.early_start;
                    }
                }
            }

            if !has_successors {
                succ_ls_min = total_project_duration;
                succ_es_min = total_project_duration;
            }

            let late_finish = succ_ls_min;
            let late_start = late_finish.saturating_sub(act_dur);
            let total_float = late_finish.saturating_sub(act_ef);
            let is_critical = total_float == 0;
            let free_float = succ_es_min.saturating_sub(act_ef);

            activities[i].late_finish = late_finish;
            activities[i].late_start = late_start;
            activities[i].total_float = total_float;
            activities[i].is_critical = is_critical;
            activities[i].free_float = free_float;
        }

        Ok(activities)
    }

    /// Generates weekly S-Curve physical progress and financial cashflow points
    pub fn generate_scurve(
        activities: &[ConstructionActivity],
        total_budget_idr: f64,
    ) -> Vec<SCurvePoint> {
        let total_days = activities
            .iter()
            .map(|a| a.early_finish)
            .max()
            .unwrap_or(1)
            .max(1);

        let total_weeks = (total_days + 6) / 7;
        let mut curve = Vec::new();
        let mut cumulative_pct = 0.0;

        // Equal weight if weight_percentage is 0
        let uniform_weight = 100.0 / (activities.len() as f64);

        for week in 1..=total_weeks {
            let day_start = (week - 1) * 7;
            let day_end = (week * 7).min(total_days);

            let mut weekly_pct = 0.0;
            for act in activities {
                let weight = if act.weight_percentage > 0.0 {
                    act.weight_percentage
                } else {
                    uniform_weight
                };

                let act_dur = act.duration_days.max(1);
                // Overlap between week [day_start, day_end] and act [early_start, early_finish]
                let overlap_start = day_start.max(act.early_start);
                let overlap_end = day_end.min(act.early_finish);

                if overlap_end > overlap_start {
                    let active_days_in_week = overlap_end - overlap_start;
                    let portion = (active_days_in_week as f64) / (act_dur as f64);
                    weekly_pct += portion * weight;
                }
            }

            cumulative_pct = (cumulative_pct + weekly_pct).min(100.0);
            if week == total_weeks {
                cumulative_pct = 100.0;
            }

            let weekly_cost = (weekly_pct / 100.0) * total_budget_idr;
            let cumulative_cost = (cumulative_pct / 100.0) * total_budget_idr;

            curve.push(SCurvePoint {
                week_number: week,
                day_number: day_end,
                weekly_planned_pct: weekly_pct,
                cumulative_planned_pct: cumulative_pct,
                weekly_cost_idr: weekly_cost,
                cumulative_cost_idr: cumulative_cost,
            });
        }

        curve
    }

    /// Evaluates SVD kinematic stability across chronological erection stages of a structure
    pub fn verify_staged_erection(model: &StructuralModel) -> Vec<StageStabilityResult> {
        if model.elements.is_empty() || model.nodes.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        // Standard 4 Erection Stages:
        // Stage 1: Columns / Lower Chord (Foundation interface)
        // Stage 2: Main Chords & Web Diagonals
        // Stage 3: Full Primary Truss Frame
        // Stage 4: Secondary Bracing & Final Assembly
        let total_elements = model.elements.len();
        let quarter = (total_elements / 4).max(1);

        let stage_specs = [
            (quarter.min(total_elements), "Stage 1: Base Anchors & Foundation Pedestals"),
            ((quarter * 2).min(total_elements), "Stage 2: Lower Chords & Column Cantilever"),
            ((quarter * 3).min(total_elements), "Stage 3: Web Diagonals & Main Post Erection"),
            (total_elements, "Stage 4: Upper Chords & Secondary Bracing Closure"),
        ];

        for (idx, &(count, name)) in stage_specs.iter().enumerate() {
            let mut partial_model = model.clone();
            partial_model.elements = model.elements[0..count].to_vec();

            // Run FEM assemble & solve
            let solve_res = FemSolver::solve(&partial_model);
            match solve_res {
                Ok(analysis) => {
                    let (rank, null_dim, _) = analysis.k_free.rank_and_nullspace(1e-7);
                    let (_, s_vec, _) = analysis.k_free.svd();
                    let s_max = s_vec.first().copied().unwrap_or(0.0);
                    let s_min = s_vec.last().copied().unwrap_or(0.0);
                    let cond = if s_min > 1e-15 { s_max / s_min } else { f64::INFINITY };

                    let is_stable = null_dim == 0 && rank == analysis.k_free.rows;
                    let warning = if !is_stable {
                        Some(format!("Kinematic mechanism detected! {} rigid modes active before closure.", null_dim))
                    } else if cond > 1e12 {
                        Some("High condition number: temporary shoring recommended.".to_string())
                    } else {
                        None
                    };

                    results.push(StageStabilityResult {
                        stage_index: idx + 1,
                        stage_name: name.to_string(),
                        erected_elements_count: count,
                        active_dofs: analysis.k_free.rows,
                        rank,
                        is_stable,
                        condition_number: cond,
                        min_singular_value: s_min,
                        max_deflection_mm: analysis.max_displacement_mm,
                        warning,
                    });
                }
                Err(err) => {
                    results.push(StageStabilityResult {
                        stage_index: idx + 1,
                        stage_name: name.to_string(),
                        erected_elements_count: count,
                        active_dofs: 0,
                        rank: 0,
                        is_stable: false,
                        condition_number: f64::INFINITY,
                        min_singular_value: 0.0,
                        max_deflection_mm: f64::INFINITY,
                        warning: Some(format!("Stiffness singularity: {}. Shoring tower required.", err)),
                    });
                }
            }
        }

        results
    }

    /// Creates standard steel truss construction schedule with WBS, CPM, S-Curve, and Staged SVD Guard
    pub fn generate_standard_schedule(
        model: &StructuralModel,
        total_budget_idr: f64,
    ) -> Schedule4DReport {
        let raw_activities = vec![
            ConstructionActivity {
                id: 1,
                name: "Mobilisasi & Survey Titik Pengukuran (Topografi)".to_string(),
                duration_days: 7,
                predecessors: vec![],
                early_start: 0,
                early_finish: 0,
                late_start: 0,
                late_finish: 0,
                total_float: 0,
                free_float: 0,
                is_critical: false,
                weight_percentage: 5.0,
            },
            ConstructionActivity {
                id: 2,
                name: "Pondasi Bored Pile & Pemasangan Angkur Pedestal".to_string(),
                duration_days: 21,
                predecessors: vec![1],
                early_start: 0,
                early_finish: 0,
                late_start: 0,
                late_finish: 0,
                total_float: 0,
                free_float: 0,
                is_critical: false,
                weight_percentage: 20.0,
            },
            ConstructionActivity {
                id: 3,
                name: "Fabrikasi & Pengelasan Batang Rangka di Workshop".to_string(),
                duration_days: 28,
                predecessors: vec![1],
                early_start: 0,
                early_finish: 0,
                late_start: 0,
                late_finish: 0,
                total_float: 0,
                free_float: 0,
                is_critical: false,
                weight_percentage: 25.0,
            },
            ConstructionActivity {
                id: 4,
                name: "Transportasi Modul Baja & Erection Lower Chords".to_string(),
                duration_days: 14,
                predecessors: vec![2, 3],
                early_start: 0,
                early_finish: 0,
                late_start: 0,
                late_finish: 0,
                total_float: 0,
                free_float: 0,
                is_critical: false,
                weight_percentage: 15.0,
            },
            ConstructionActivity {
                id: 5,
                name: "Erection Batang Diagonal Web & Upper Chords".to_string(),
                duration_days: 21,
                predecessors: vec![4],
                early_start: 0,
                early_finish: 0,
                late_start: 0,
                late_finish: 0,
                total_float: 0,
                free_float: 0,
                is_critical: false,
                weight_percentage: 20.0,
            },
            ConstructionActivity {
                id: 6,
                name: "Pemasangan Bracing Sekunder & Pengencangan Baut Mutu Tinggi".to_string(),
                duration_days: 14,
                predecessors: vec![5],
                early_start: 0,
                early_finish: 0,
                late_start: 0,
                late_finish: 0,
                total_float: 0,
                free_float: 0,
                is_critical: false,
                weight_percentage: 10.0,
            },
            ConstructionActivity {
                id: 7,
                name: "Inspeksi NDT Ultrasonik & Uji Beban Jembatan (Proof Load)".to_string(),
                duration_days: 7,
                predecessors: vec![6],
                early_start: 0,
                early_finish: 0,
                late_start: 0,
                late_finish: 0,
                total_float: 0,
                free_float: 0,
                is_critical: false,
                weight_percentage: 5.0,
            },
        ];

        let activities = Self::compute_cpm(raw_activities).unwrap_or_default();
        let total_duration_days = activities.iter().map(|a| a.early_finish).max().unwrap_or(0);
        let critical_path_activity_ids = activities
            .iter()
            .filter(|a| a.is_critical)
            .map(|a| a.id)
            .collect();

        let scurve = Self::generate_scurve(&activities, total_budget_idr);
        let staged_stability = Self::verify_staged_erection(model);
        let is_erection_sequence_safe = staged_stability.last().map(|s| s.is_stable).unwrap_or(true);

        Schedule4DReport {
            total_duration_days,
            critical_path_activity_ids,
            activities,
            scurve,
            staged_stability,
            is_erection_sequence_safe,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parametric::ParametricEngine;

    #[test]
    fn test_cpm_and_scurve_generation() {
        let mut engine = ParametricEngine::new();
        let model = engine.generate_warren_truss().expect("Generate Warren");

        let report = ScheduleEngine::generate_standard_schedule(&model, 3_840_000_000.0);

        assert!(report.total_duration_days > 0);
        assert!(!report.critical_path_activity_ids.is_empty());
        assert!(!report.scurve.is_empty());
        assert_eq!(report.scurve.last().unwrap().cumulative_planned_pct, 100.0);
        assert!(!report.staged_stability.is_empty());
    }
}
