//! agent_tools.rs — Headless Deterministic Engineering JSON Tool Registry (`wasm32`-safe).
//!
//! Exposes EigenGraph's BIM inspection, 6-DOF FEM + EigenGate redundancy audit,
//! SNI 6389 Solar OTTV, BRE Daylight Autonomy, parametric resizing, and 3D WebGPU Section Box
//! tools as structured `functionDeclarations` schemas for native Gemini Function Calling
//! and future autonomous reasoning models.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::bim::{BimCommand, BimCommandBus};
use crate::core::daylight_interior::{
    evaluate_daylight_interior, DaylightInteriorConfig, InteriorFinishPalette,
    InteriorSpaceTypology,
};
use crate::core::eigengate::EigenGateEngine;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::solar_envelope::{
    evaluate_solar_envelope, GlazingPerformancePreset, SolarEnvelopeConfig, SolarShadingStrategy,
};

/// Structured tool invocation emitted by Gemini `functionCall` parts or headless agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeminiToolCall {
    pub name: String,
    #[serde(default)]
    pub args: Value,
}

/// Section box mutation emitted by `set_section_box` for the spatial viewport to apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolSectionBoxCommand {
    Fit,
    Disable,
    LevelCut {
        z_m: Option<f32>,
        story_idx: Option<usize>,
    },
    SetBounds {
        min_enu_m: [f32; 3],
        max_enu_m: [f32; 3],
        clip_terrain: bool,
    },
}

/// Deterministic outcome returned by [`execute_headless_tool`] and fed back to Gemini via `functionResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionOutcome {
    pub name: String,
    pub ok: bool,
    pub summary: String,
    pub result_json: Value,
    pub mutated_bim: bool,
    pub sync_fem_and_earth: bool,
    pub fit_camera: bool,
    pub section_box_update: Option<ToolSectionBoxCommand>,
    pub fly_target: Option<(f64, f64, f32)>,
}

impl ToolExecutionOutcome {
    fn err(name: &str, msg: impl Into<String>) -> Self {
        let message = msg.into();
        Self {
            name: name.to_string(),
            ok: false,
            summary: message.clone(),
            result_json: json!({ "ok": false, "error": message }),
            mutated_bim: false,
            sync_fem_and_earth: false,
            fit_camera: false,
            section_box_update: None,
            fly_target: None,
        }
    }
}

const TOOL_NAMES: [&str; 11] = [
    "inspect_model",
    "run_structural_audit",
    "run_solar_ottv",
    "run_daylight_analysis",
    "run_mep_systems",
    "resize_column",
    "resize_beam",
    "add_story",
    "auto_repair_model",
    "set_section_box",
    "fly_to_location",
];

/// Returns all registered headless engineering tool names.
pub fn registered_tool_names() -> &'static [&'static str] {
    &TOOL_NAMES
}

/// Returns the Gemini REST `tools` array containing `functionDeclarations` for all 10 engineering tools.
pub fn gemini_function_declarations() -> Value {
    json!([
        {
            "functionDeclarations": [
                {
                    "name": "inspect_model",
                    "description": "Inspect the active OpenBIM project geometry, levels, columns, beams, walls, footings, slabs, site seismic/wind parameters, and section box status.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "include_elements": {
                                "type": "BOOLEAN",
                                "description": "Include per-column and per-beam IDs and dimensions (default true)."
                            }
                        }
                    }
                },
                {
                    "name": "run_structural_audit",
                    "description": "Run deterministic 6-DOF FEM solver and EigenGate redundancy evaluation on the active BIM project, returning max DCR, min ARI redundancy, and critical member count.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {}
                    }
                },
                {
                    "name": "run_solar_ottv",
                    "description": "Evaluate SNI 6389 / ASHRAE 90.1 3D solar irradiation, OTTV/RTTV facade thermal load, and cooling energy intensity.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "glazing": {
                                "type": "STRING",
                                "description": "Glazing preset: clear, tinted, low_e, or triple."
                            },
                            "shading": {
                                "type": "STRING",
                                "description": "Shading strategy: none, overhang, fins, or louvers."
                            },
                            "wwr_pct": {
                                "type": "NUMBER",
                                "description": "Window-to-wall ratio percentage (10..85)."
                            },
                            "hour_local": {
                                "type": "NUMBER",
                                "description": "Local solar inspection hour (6..18)."
                            }
                        }
                    }
                },
                {
                    "name": "run_daylight_analysis",
                    "description": "Evaluate BRE Split-Flux Daylight Autonomy (sDA 300/50%, ASE, UDI, DGP glare) and LEED v4.1 EQ credits.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "palette": {
                                "type": "STRING",
                                "description": "Interior finish palette: minimalist, timber, concrete, or cleanroom."
                            },
                            "typology": {
                                "type": "STRING",
                                "description": "Space typology: office, classroom, healthcare, retail, or residential."
                            },
                            "vlt": {
                                "type": "NUMBER",
                                "description": "Glazing visible light transmittance (0.20..0.88)."
                            }
                        }
                    }
                },
                {
                    "name": "run_mep_systems",
                    "description": "Evaluate Building MEP Systems (ASHRAE HVAC ducts/cooling TR, IPC plumbing DN pipe, 3-phase electrical feeder/breaker, and NFPA 13 fire sprinkler hydraulics).",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "hvac_preset": {
                                "type": "STRING",
                                "description": "HVAC system archetype: vrf, chilled_ahu, or packaged_dx."
                            },
                            "fire_hazard": {
                                "type": "STRING",
                                "description": "NFPA 13 occupancy hazard class: light, ord_1, or ord_2."
                            },
                            "occupant_density_m2": {
                                "type": "NUMBER",
                                "description": "Floor area per person in m^2/person (4..35)."
                            }
                        }
                    }
                },
                {
                    "name": "resize_column",
                    "description": "Resize a structural column in the active BIM project by column ID in millimetres.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "column_id": { "type": "INTEGER", "description": "Target BimColumn ID." },
                            "width_mm": { "type": "NUMBER", "description": "Column width in mm (150..2500)." },
                            "depth_mm": { "type": "NUMBER", "description": "Column depth in mm (150..2500)." }
                        },
                        "required": ["column_id", "width_mm", "depth_mm"]
                    }
                },
                {
                    "name": "resize_beam",
                    "description": "Resize a structural beam or run in the active BIM project by run ID in millimetres.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "run_id": { "type": "INTEGER", "description": "Target BimRun ID." },
                            "width_mm": { "type": "NUMBER", "description": "Beam width in mm (120..2000)." },
                            "depth_mm": { "type": "NUMBER", "description": "Beam depth in mm (150..2500)." }
                        },
                        "required": ["run_id", "width_mm", "depth_mm"]
                    }
                },
                {
                    "name": "add_story",
                    "description": "Add a new upper story level to the active BIM project.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "height_mm": {
                                "type": "NUMBER",
                                "description": "Story-to-story height in mm (default 3500)."
                            }
                        }
                    }
                },
                {
                    "name": "auto_repair_model",
                    "description": "Run EigenGate closed-loop structural hardening to upsize weak columns/beams and improve redundancy/DCR.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {}
                    }
                },
                {
                    "name": "set_section_box",
                    "description": "Control the real-time WebGPU 3D Section Box or horizontal Active Level Cut Plane.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "mode": {
                                "type": "STRING",
                                "description": "One of: fit, off, level_cut, bounds."
                            },
                            "z_cut_m": {
                                "type": "NUMBER",
                                "description": "Horizontal cut elevation in meters for level_cut mode."
                            },
                            "story_idx": {
                                "type": "INTEGER",
                                "description": "Zero-based story index for level_cut mode."
                            },
                            "clip_terrain": {
                                "type": "BOOLEAN",
                                "description": "Clip site terrain and city buildings in bounds mode."
                            },
                            "min_xyz": {
                                "type": "ARRAY",
                                "items": { "type": "NUMBER" },
                                "description": "Minimum ENU bounds [x, y, z] in meters for bounds mode."
                            },
                            "max_xyz": {
                                "type": "ARRAY",
                                "items": { "type": "NUMBER" },
                                "description": "Maximum ENU bounds [x, y, z] in meters for bounds mode."
                            }
                        },
                        "required": ["mode"]
                    }
                },
                {
                    "name": "fly_to_location",
                    "description": "Fly the 3D Vector Earth viewport camera to geographic latitude/longitude coordinates.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "lat": { "type": "NUMBER", "description": "Latitude in decimal degrees (-85..85)." },
                            "lon": { "type": "NUMBER", "description": "Longitude in decimal degrees (-180..180)." },
                            "zoom": { "type": "NUMBER", "description": "Map zoom level (2..19)." }
                        },
                        "required": ["lat", "lon"]
                    }
                }
            ]
        }
    ])
}

fn arg_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_i64().map(|i| i as f64))
            .or_else(|| v.as_u64().map(|u| u as f64))
            .or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
    })
}

fn arg_usize(args: &Value, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| {
        v.as_u64()
            .map(|u| u as usize)
            .or_else(|| v.as_i64().filter(|&i| i >= 0).map(|i| i as usize))
            .or_else(|| v.as_f64().filter(|f| *f >= 0.0).map(|f| f.round() as usize))
            .or_else(|| v.as_str().and_then(|s| s.trim().parse::<usize>().ok()))
    })
}

fn arg_vec3(args: &Value, key: &str) -> Option<[f32; 3]> {
    let arr = args.get(key)?.as_array()?;
    if arr.len() != 3 {
        return None;
    }
    let x = arr[0].as_f64()? as f32;
    let y = arr[1].as_f64()? as f32;
    let z = arr[2].as_f64()? as f32;
    if x.is_finite() && y.is_finite() && z.is_finite() {
        Some([x, y, z])
    } else {
        None
    }
}

/// Executes a structured engineering tool call deterministically against the headless BIM & physics kernel.
pub fn execute_headless_tool(
    call: &GeminiToolCall,
    bim_bus: &mut BimCommandBus,
    site: &SiteEnvironmentalCoupling,
    site_lat_lon: (f64, f64),
    active_story_idx: usize,
    section_box_enabled: bool,
) -> ToolExecutionOutcome {
    let name = call.name.trim();
    match name {
        "inspect_model" => {
            let include_elements = call
                .args
                .get("include_elements")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let proj = &bim_bus.project;
            let mut lvls = proj.levels.clone();
            lvls.sort_by(|a, b| a.elevation_mm.total_cmp(&b.elevation_mm));
            let levels_json: Vec<Value> = lvls
                .iter()
                .map(|l| {
                    json!({
                        "id": l.id,
                        "name": l.name,
                        "elevation_m": (l.elevation_mm * 0.001 * 100.0).round() / 100.0
                    })
                })
                .collect();
            let cols_json: Vec<Value> = if include_elements {
                proj.columns
                    .iter()
                    .take(24)
                    .map(|c| {
                        json!({
                            "id": c.id,
                            "name": c.name,
                            "width_mm": c.width_mm,
                            "depth_mm": c.depth_mm,
                            "profile": c.profile_name
                        })
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let beams_json: Vec<Value> = if include_elements {
                proj.runs
                    .iter()
                    .take(24)
                    .map(|r| {
                        json!({
                            "id": r.id,
                            "name": r.name,
                            "width_mm": r.width_mm,
                            "depth_mm": r.depth_mm,
                            "profile": r.profile_name
                        })
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let summary = format!(
                "{}: {} elements ({} cols, {} beams, {} walls, {} slabs, {} footings), {} levels",
                proj.project_name,
                proj.element_count(),
                proj.columns.len(),
                proj.runs.len(),
                proj.walls.len(),
                proj.slabs.len(),
                proj.footings.len(),
                proj.levels.len()
            );
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary,
                result_json: json!({
                    "ok": true,
                    "project_name": proj.project_name,
                    "element_count": proj.element_count(),
                    "counts": {
                        "levels": proj.levels.len(),
                        "columns": proj.columns.len(),
                        "beams": proj.runs.len(),
                        "walls": proj.walls.len(),
                        "slabs": proj.slabs.len(),
                        "footings": proj.footings.len(),
                    },
                    "site": {
                        "label": site.site_label,
                        "lat_deg": site_lat_lon.0,
                        "lon_deg": site_lat_lon.1,
                        "elevation_m": site.elevation_m,
                        "seismic_pga_ss_g": site.seismic_pga_ss_g,
                        "design_wind_ms": site.design_wind_ms,
                    },
                    "active_story_idx": active_story_idx,
                    "section_box_enabled": section_box_enabled,
                    "levels": levels_json,
                    "columns_sample": cols_json,
                    "beams_sample": beams_json,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
            }
        }
        "run_structural_audit" => {
            if bim_bus.project.is_empty() {
                return ToolExecutionOutcome::err(
                    name,
                    "Active BIM project is empty; add columns/beams or adopt a massing first.",
                );
            }
            let model = bim_bus.project.to_structural_model();
            if model.elements.is_empty() {
                return ToolExecutionOutcome::err(
                    name,
                    "No analytical 3D frame elements found in active BIM project.",
                );
            }
            match EigenGateEngine::evaluate_redundancy(&model) {
                Ok(eval) => {
                    let max_dcr = eval
                        .member_results
                        .iter()
                        .map(|m| m.utilization_initial)
                        .fold(0.0_f64, f64::max);
                    let top_critical: Vec<Value> = eval
                        .member_results
                        .iter()
                        .take(8)
                        .map(|m| {
                            json!({
                                "member_id": m.member_id,
                                "name": m.member_name,
                                "dcr": (m.utilization_initial * 1000.0).round() / 1000.0,
                                "eigengate_ari": (m.eigengate * 1000.0).round() / 1000.0,
                            })
                        })
                        .collect();
                    let summary = format!(
                        "FEM+EigenGate Audit: {} members, Max DCR={:.1}%, Min ARI={:.3}, Critical={}",
                        eval.member_results.len(),
                        max_dcr * 100.0,
                        eval.min_system_eigengate,
                        eval.critical_members_count
                    );
                    ToolExecutionOutcome {
                        name: name.to_string(),
                        ok: true,
                        summary,
                        result_json: json!({
                            "ok": true,
                            "kinematic_stable": eval.kinematic_guard.is_stable,
                            "member_count": eval.member_results.len(),
                            "max_dcr": max_dcr,
                            "max_dcr_pct": max_dcr * 100.0,
                            "min_system_eigengate": eval.min_system_eigengate,
                            "average_eigengate": eval.average_eigengate,
                            "critical_members_count": eval.critical_members_count,
                            "members_sample": top_critical,
                        }),
                        mutated_bim: false,
                        sync_fem_and_earth: false,
                        fit_camera: false,
                        section_box_update: None,
                        fly_target: None,
                    }
                }
                Err(err) => ToolExecutionOutcome::err(name, err),
            }
        }
        "run_solar_ottv" => {
            let mut cfg = SolarEnvelopeConfig::default();
            if let Some(glazing) = call.args.get("glazing").and_then(|v| v.as_str()) {
                let preset = match glazing.trim().to_ascii_lowercase().as_str() {
                    "clear" | "clear_single" => GlazingPerformancePreset::ClearSingle,
                    "tinted" | "tinted_double" => GlazingPerformancePreset::TintedDouble,
                    "triple" | "triple_high_perf" => GlazingPerformancePreset::TripleHighPerf,
                    _ => GlazingPerformancePreset::LowESpectrallySelective,
                };
                cfg.apply_glazing(preset);
            }
            if let Some(shading) = call.args.get("shading").and_then(|v| v.as_str()) {
                cfg.shading_strategy = match shading.trim().to_ascii_lowercase().as_str() {
                    "none" => SolarShadingStrategy::None,
                    "fins" | "vertical_fins" => SolarShadingStrategy::VerticalFins,
                    "louvers" | "eggcrate" => SolarShadingStrategy::EggCrateLouvers,
                    _ => SolarShadingStrategy::HorizontalOverhang,
                };
            }
            if let Some(wwr) = arg_f64(&call.args, "wwr_pct") {
                cfg.wwr_pct = wwr.clamp(10.0, 85.0);
            }
            if let Some(hr) = arg_f64(&call.args, "hour_local") {
                cfg.inspection_hour_local = hr.clamp(6.0, 18.0);
            }
            let rep = evaluate_solar_envelope(&bim_bus.project, Some(site), None, &cfg);
            let summary = format!(
                "Solar OTTV={:.1} W/m² (SNI 6389 <= {:.0}: {}), RTTV={:.1} W/m², Cooling={:.1} kWh/m²/yr",
                rep.overall_ottv_w_m2,
                rep.config.ottv_limit_w_m2,
                if rep.sni_6389_compliant { "PASS" } else { "FAIL" },
                rep.rttv_roof_w_m2,
                rep.annual_cooling_kwh_m2
            );
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary,
                result_json: json!({
                    "ok": true,
                    "overall_ottv_w_m2": rep.overall_ottv_w_m2,
                    "rttv_roof_w_m2": rep.rttv_roof_w_m2,
                    "ottv_limit_w_m2": rep.config.ottv_limit_w_m2,
                    "sni_6389_compliant": rep.sni_6389_compliant,
                    "annual_cooling_kwh_m2": rep.annual_cooling_kwh_m2,
                    "peak_solar_load_kw": rep.peak_solar_load_kw,
                    "worst_orientation": rep.worst_orientation,
                    "verdict": rep.summary_verdict,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
            }
        }
        "run_daylight_analysis" => {
            let mut cfg = DaylightInteriorConfig::default();
            if let Some(pal) = call.args.get("palette").and_then(|v| v.as_str()) {
                cfg.finish_palette = match pal.trim().to_ascii_lowercase().as_str() {
                    "timber" => InteriorFinishPalette::ExecutiveWarmTimber,
                    "concrete" => InteriorFinishPalette::IndustrialExposedConcrete,
                    "cleanroom" | "healthcare" => InteriorFinishPalette::HealthcareCleanroom,
                    _ => InteriorFinishPalette::MinimalistDaylight,
                };
            }
            if let Some(typ) = call.args.get("typology").and_then(|v| v.as_str()) {
                cfg.space_typology = match typ.trim().to_ascii_lowercase().as_str() {
                    "classroom" => InteriorSpaceTypology::Classroom,
                    "healthcare" | "clinic" => InteriorSpaceTypology::HealthcareWard,
                    "retail" => InteriorSpaceTypology::RetailShowroom,
                    "residential" => InteriorSpaceTypology::ResidentialSuite,
                    _ => InteriorSpaceTypology::OpenOffice,
                };
            }
            if let Some(vlt) = arg_f64(&call.args, "vlt") {
                cfg.visible_transmittance_vlt = vlt.clamp(0.20, 0.88);
            }
            let rep = evaluate_daylight_interior(&bim_bus.project, Some(site), None, None, &cfg);
            let summary = format!(
                "Daylight: sDA={:.1}%, ASE={:.1}%, DGP={:.2}, LEED v4.1={} pts",
                rep.sda_300_50_pct, rep.ase_1000_250_pct, rep.peak_dgp_glare, rep.leed_v41_credits
            );
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary,
                result_json: json!({
                    "ok": true,
                    "mean_lux": rep.mean_lux,
                    "mean_daylight_factor_pct": rep.mean_daylight_factor_pct,
                    "sda_300_50_pct": rep.sda_300_50_pct,
                    "ase_1000_250_pct": rep.ase_1000_250_pct,
                    "udi_300_3000_pct": rep.udi_300_3000_pct,
                    "peak_dgp_glare": rep.peak_dgp_glare,
                    "leed_v41_credits": rep.leed_v41_credits,
                    "en_17037_compliant": rep.en_17037_compliant,
                    "verdict": rep.summary_verdict,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
            }
        }
        "run_mep_systems" => {
            use crate::core::mep_engine::{
                evaluate_mep_systems, FireHazardClass, HvacSystemPreset, MepSystemsConfig,
            };
            let solar_rep =
                evaluate_solar_envelope(&bim_bus.project, Some(site), None, &SolarEnvelopeConfig::default());
            let mut cfg = MepSystemsConfig::default();
            if let Some(hp) = call.args.get("hvac_preset").and_then(|v| v.as_str()) {
                cfg.hvac_preset = match hp.trim().to_ascii_lowercase().as_str() {
                    "chilled_ahu" | "ahu" | "chiller" => HvacSystemPreset::ChilledWaterAhu,
                    "packaged_dx" | "dx" => HvacSystemPreset::PackagedDxDucted,
                    _ => HvacSystemPreset::VrfInverter,
                };
            }
            if let Some(fh) = call.args.get("fire_hazard").and_then(|v| v.as_str()) {
                cfg.fire_hazard = match fh.trim().to_ascii_lowercase().as_str() {
                    "ord_1" | "ordinary_1" | "ord1" => FireHazardClass::OrdinaryHazardGroup1,
                    "ord_2" | "ordinary_2" | "ord2" => FireHazardClass::OrdinaryHazardGroup2,
                    _ => FireHazardClass::LightHazard,
                };
            }
            if let Some(occ) = arg_f64(&call.args, "occupant_density_m2") {
                cfg.occupant_density_m2_per_person = occ.clamp(4.0, 35.0);
            }
            let rep = evaluate_mep_systems(&bim_bus.project, Some(site), Some(&solar_rep), &cfg);
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: rep.summary_verdict.clone(),
                result_json: json!({
                    "ok": true,
                    "stories": rep.stories,
                    "total_gfa_m2": rep.total_gfa_m2,
                    "hvac_cooling_tr": rep.cooling_tr,
                    "hvac_duct_w_mm": rep.duct_width_mm,
                    "hvac_duct_h_mm": rep.duct_height_mm,
                    "plumbing_pipe_dn_mm": rep.plumbing_pipe_dn_mm,
                    "elec_demand_kva": rep.elec_demand_kva,
                    "elec_breaker_at": rep.elec_breaker_at,
                    "elec_voltage_drop_pct": rep.elec_voltage_drop_pct,
                    "fire_pump_flow_lpm": rep.fire.fire_pump_flow_lpm,
                    "sprinkler_heads": rep.fire.total_sprinkler_heads,
                    "segment_count": rep.segments_3d.len(),
                    "estimated_mep_cost_usd": rep.estimated_mep_cost_usd,
                    "all_codes_compliant": rep.all_codes_compliant,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
            }
        }
        "resize_column" => {
            let Some(column_id) = arg_usize(&call.args, "column_id") else {
                return ToolExecutionOutcome::err(name, "Missing integer argument 'column_id'.");
            };
            let Some(width_mm) = arg_f64(&call.args, "width_mm") else {
                return ToolExecutionOutcome::err(name, "Missing numeric argument 'width_mm'.");
            };
            let Some(depth_mm) = arg_f64(&call.args, "depth_mm") else {
                return ToolExecutionOutcome::err(name, "Missing numeric argument 'depth_mm'.");
            };
            let rot = bim_bus
                .project
                .columns
                .iter()
                .find(|c| c.id == column_id)
                .map(|c| c.rotation_deg)
                .unwrap_or(0.0);
            let w = width_mm.clamp(150.0, 2500.0);
            let d = depth_mm.clamp(150.0, 2500.0);
            let profile_name = format!("RC {:.0}x{:.0}", w, d);
            match bim_bus.execute(BimCommand::ResizeColumn {
                column_id,
                width_mm: w,
                depth_mm: d,
                rotation_deg: rot,
                profile_name: profile_name.clone(),
            }) {
                Ok(msg) => ToolExecutionOutcome {
                    name: name.to_string(),
                    ok: true,
                    summary: msg.clone(),
                    result_json: json!({
                        "ok": true,
                        "column_id": column_id,
                        "width_mm": w,
                        "depth_mm": d,
                        "profile": profile_name,
                        "message": msg,
                    }),
                    mutated_bim: true,
                    sync_fem_and_earth: true,
                    fit_camera: false,
                    section_box_update: None,
                    fly_target: None,
                },
                Err(err) => ToolExecutionOutcome::err(name, err),
            }
        }
        "resize_beam" => {
            let Some(run_id) = arg_usize(&call.args, "run_id") else {
                return ToolExecutionOutcome::err(name, "Missing integer argument 'run_id'.");
            };
            let Some(width_mm) = arg_f64(&call.args, "width_mm") else {
                return ToolExecutionOutcome::err(name, "Missing numeric argument 'width_mm'.");
            };
            let Some(depth_mm) = arg_f64(&call.args, "depth_mm") else {
                return ToolExecutionOutcome::err(name, "Missing numeric argument 'depth_mm'.");
            };
            let w = width_mm.clamp(120.0, 2000.0);
            let d = depth_mm.clamp(150.0, 2500.0);
            let profile_name = format!("RC {:.0}x{:.0}", w, d);
            match bim_bus.execute(BimCommand::ResizeRun {
                run_id,
                width_mm: w,
                depth_mm: d,
                profile_name: profile_name.clone(),
            }) {
                Ok(msg) => ToolExecutionOutcome {
                    name: name.to_string(),
                    ok: true,
                    summary: msg.clone(),
                    result_json: json!({
                        "ok": true,
                        "run_id": run_id,
                        "width_mm": w,
                        "depth_mm": d,
                        "profile": profile_name,
                        "message": msg,
                    }),
                    mutated_bim: true,
                    sync_fem_and_earth: true,
                    fit_camera: false,
                    section_box_update: None,
                    fly_target: None,
                },
                Err(err) => ToolExecutionOutcome::err(name, err),
            }
        }
        "add_story" => {
            let height_mm = arg_f64(&call.args, "height_mm")
                .unwrap_or(3500.0)
                .clamp(2200.0, 8000.0);
            let max_elev = bim_bus
                .project
                .levels
                .iter()
                .map(|l| l.elevation_mm)
                .fold(0.0_f64, f64::max);
            let next_num = bim_bus.project.levels.len() + 1;
            let new_elev = max_elev + height_mm;
            match bim_bus.execute(BimCommand::AddLevel {
                name: format!("Level {next_num}"),
                elevation_mm: new_elev,
            }) {
                Ok(msg) => ToolExecutionOutcome {
                    name: name.to_string(),
                    ok: true,
                    summary: msg.clone(),
                    result_json: json!({
                        "ok": true,
                        "level_count": bim_bus.project.levels.len(),
                        "elevation_m": new_elev * 0.001,
                        "message": msg,
                    }),
                    mutated_bim: true,
                    sync_fem_and_earth: true,
                    fit_camera: true,
                    section_box_update: None,
                    fly_target: None,
                },
                Err(err) => ToolExecutionOutcome::err(name, err),
            }
        }
        "auto_repair_model" => match bim_bus.repair_critical_members(&[]) {
            Ok(rep) => ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: rep.summary.clone(),
                result_json: json!({
                    "ok": true,
                    "before_min_eigengate": rep.before_min_eigengate,
                    "after_min_eigengate": rep.after_min_eigengate,
                    "before_max_dcr": rep.before_max_dcr,
                    "after_max_dcr": rep.after_max_dcr,
                    "upgraded_columns": rep.upgraded_columns,
                    "upgraded_beams": rep.upgraded_beams,
                    "added_braces": rep.added_braces,
                    "summary": rep.summary,
                }),
                mutated_bim: true,
                sync_fem_and_earth: true,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
            },
            Err(err) => ToolExecutionOutcome::err(name, err),
        },
        "set_section_box" => {
            let mode = call
                .args
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("fit")
                .trim()
                .to_ascii_lowercase();
            let (cmd, summary) = match mode.as_str() {
                "off" | "disable" => (
                    ToolSectionBoxCommand::Disable,
                    "Disabled WebGPU 3D Section Box".to_string(),
                ),
                "level_cut" | "level" => {
                    let z_m = arg_f64(&call.args, "z_cut_m").map(|z| z as f32);
                    let story_idx = arg_usize(&call.args, "story_idx");
                    (
                        ToolSectionBoxCommand::LevelCut { z_m, story_idx },
                        format!("Enabled Active Level Cut Plane (z={z_m:?}, story={story_idx:?})"),
                    )
                }
                "bounds" => {
                    let Some(min_enu_m) = arg_vec3(&call.args, "min_xyz") else {
                        return ToolExecutionOutcome::err(
                            name,
                            "bounds mode requires 'min_xyz': [x, y, z].",
                        );
                    };
                    let Some(max_enu_m) = arg_vec3(&call.args, "max_xyz") else {
                        return ToolExecutionOutcome::err(
                            name,
                            "bounds mode requires 'max_xyz': [x, y, z].",
                        );
                    };
                    let clip_terrain = call
                        .args
                        .get("clip_terrain")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    (
                        ToolSectionBoxCommand::SetBounds {
                            min_enu_m,
                            max_enu_m,
                            clip_terrain,
                        },
                        format!(
                            "Set 3D Section Box bounds {:?}..{:?} (clip_terrain={clip_terrain})",
                            min_enu_m, max_enu_m
                        ),
                    )
                }
                _ => (
                    ToolSectionBoxCommand::Fit,
                    "Fitted 3D Section Box to active BIM project".to_string(),
                ),
            };
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: summary.clone(),
                result_json: json!({ "ok": true, "mode": mode, "message": summary }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: Some(cmd),
                fly_target: None,
            }
        }
        "fly_to_location" => {
            let Some(lat) = arg_f64(&call.args, "lat") else {
                return ToolExecutionOutcome::err(name, "Missing numeric argument 'lat'.");
            };
            let Some(lon) = arg_f64(&call.args, "lon") else {
                return ToolExecutionOutcome::err(name, "Missing numeric argument 'lon'.");
            };
            let zoom = arg_f64(&call.args, "zoom").unwrap_or(16.5).clamp(2.0, 19.0) as f32;
            let lat_c = lat.clamp(-85.0, 85.0);
            let lon_c = lon.clamp(-180.0, 180.0);
            let summary = format!("Flying Earth 3D camera -> ({lat_c:.4}°, {lon_c:.4}°) @ z{zoom:.1}");
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: summary.clone(),
                result_json: json!({
                    "ok": true,
                    "lat": lat_c,
                    "lon": lon_c,
                    "zoom": zoom,
                    "message": summary,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: true,
                section_box_update: None,
                fly_target: Some((lat_c, lon_c, zoom)),
            }
        }
        _ => ToolExecutionOutcome::err(name, format!("Unknown engineering tool '{name}'.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bim::BimProject;

    #[test]
    fn test_v0874_headless_tool_registry_and_execution() {
        let decls = gemini_function_declarations();
        let fn_decls = decls[0]["functionDeclarations"]
            .as_array()
            .expect("functionDeclarations array");
        assert_eq!(fn_decls.len(), 11);
        for expected in registered_tool_names() {
            assert!(
                fn_decls.iter().any(|d| d["name"].as_str() == Some(*expected)),
                "Missing tool schema for {expected}"
            );
        }

        let mut bus = BimCommandBus::from_project(BimProject::sample_bimcraft_building());
        let site = SiteEnvironmentalCoupling::default();

        let inspect = execute_headless_tool(
            &GeminiToolCall {
                name: "inspect_model".to_string(),
                args: json!({}),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(inspect.ok);
        assert!(inspect.result_json["counts"]["columns"].as_u64().unwrap_or(0) > 0);

        let audit = execute_headless_tool(
            &GeminiToolCall {
                name: "run_structural_audit".to_string(),
                args: json!({}),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(audit.ok, "Audit failed: {}", audit.summary);
        assert!(audit.result_json["min_system_eigengate"].as_f64().is_some());

        let solar = execute_headless_tool(
            &GeminiToolCall {
                name: "run_solar_ottv".to_string(),
                args: json!({ "glazing": "low_e", "shading": "louvers", "wwr_pct": 38.0 }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(solar.ok);
        assert!(solar.result_json["overall_ottv_w_m2"].as_f64().unwrap_or(0.0) > 0.0);

        let daylight = execute_headless_tool(
            &GeminiToolCall {
                name: "run_daylight_analysis".to_string(),
                args: json!({ "palette": "minimalist", "typology": "office", "vlt": 0.68 }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(daylight.ok);
        assert!(daylight.result_json["sda_300_50_pct"].as_f64().is_some());

        let mep = execute_headless_tool(
            &GeminiToolCall {
                name: "run_mep_systems".to_string(),
                args: json!({ "hvac_preset": "chilled_ahu", "fire_hazard": "ord_1" }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(mep.ok);
        assert!(mep.result_json["hvac_cooling_tr"].as_f64().unwrap_or(0.0) > 0.0);
        assert!(mep.result_json["segment_count"].as_u64().unwrap_or(0) >= 8);

        let col_id = bus.project.columns[0].id;
        let resize = execute_headless_tool(
            &GeminiToolCall {
                name: "resize_column".to_string(),
                args: json!({ "column_id": col_id, "width_mm": 550, "depth_mm": 550 }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(resize.ok && resize.mutated_bim && resize.sync_fem_and_earth);
        assert!((bus.project.columns[0].width_mm - 550.0).abs() < 1e-6);

        let prev_levels = bus.project.levels.len();
        let story = execute_headless_tool(
            &GeminiToolCall {
                name: "add_story".to_string(),
                args: json!({ "height_mm": 3600 }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(story.ok && bus.project.levels.len() == prev_levels + 1);

        let sec = execute_headless_tool(
            &GeminiToolCall {
                name: "set_section_box".to_string(),
                args: json!({ "mode": "level_cut", "z_cut_m": 4.2 }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(sec.ok);
        assert!(matches!(
            sec.section_box_update,
            Some(ToolSectionBoxCommand::LevelCut { z_m: Some(z), .. }) if (z - 4.2).abs() < 1e-4
        ));

        let bad = execute_headless_tool(
            &GeminiToolCall {
                name: "nonexistent_tool".to_string(),
                args: json!({}),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(!bad.ok);
        assert!(bad.result_json["error"].as_str().is_some());
    }
}
