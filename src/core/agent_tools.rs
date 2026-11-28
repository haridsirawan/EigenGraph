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

/// Visibility/Graphics (VG) & overlay action requested by the Gemini agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolVgCommand {
    ResetAll,
    ShowAllBim,
    ClearOverlays,
    IsolateCategory {
        category: String,
    },
    SetCategory {
        category: String,
        visible: Option<bool>,
        locked: Option<bool>,
    },
    ExportVgIfcx,
}

/// AutoCAD-compatible CAD drafting layer action requested by the Gemini agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolCadLayerCommand {
    AddLayer {
        name: String,
        color_rgb: [u8; 3],
    },
    SetActiveLayer {
        name: String,
    },
    ConfigureLayer {
        name: String,
        visible: Option<bool>,
        locked: Option<bool>,
    },
    SetByLayer {
        enabled: bool,
    },
    IsolateLayer {
        name: String,
    },
    ShowAllLayers,
    DeleteLayer {
        name: String,
    },
    ApplyPreset {
        preset: String,
    },
    ExportDxf,
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
    pub vg_command: Option<ToolVgCommand>,
    pub cad_layer_command: Option<ToolCadLayerCommand>,
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
            vg_command: None,
            cad_layer_command: None,
        }
    }
}

const TOOL_NAMES: [&str; 17] = [
    "inspect_model",
    "run_structural_audit",
    "run_solar_ottv",
    "run_daylight_analysis",
    "run_mep_systems",
    "run_clash_detection",
    "run_urban_utilities",
    "evaluate_road_intersections",
    "promote_city_building_to_bim",
    "resize_column",
    "resize_beam",
    "add_story",
    "auto_repair_model",
    "set_section_box",
    "fly_to_location",
    "set_visibility_graphics",
    "manage_cad_layers",
];

/// Returns all registered headless engineering tool names.
pub fn registered_tool_names() -> &'static [&'static str] {
    &TOOL_NAMES
}

/// Returns the Gemini REST `tools` array containing `functionDeclarations` for all engineering tools.
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
                    "name": "run_clash_detection",
                    "description": "Run unified 3D clash detection (Structure beams/columns/walls x MEP ducts/pipes/trays/fire x openings), returning hard clashes, clearance warnings, BCF summary, and optionally auto-resolving ceiling plenum drop.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "clearance_mm": {
                                "type": "NUMBER",
                                "description": "Required clearance tolerance in mm (10..250, default 50)."
                            },
                            "auto_resolve": {
                                "type": "BOOLEAN",
                                "description": "Automatically apply recommended plenum drop below beam soffits and re-verify."
                            }
                        }
                    }
                },
                {
                    "name": "run_urban_utilities",
                    "description": "Route subsurface potable water mains, sanitary/stormwater trunk sewers, and 20kV underground power feeders along authored IfcRoad corridors and evaluate city Demand vs. Supply balance.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "rainfall_mm_hr": {
                                "type": "NUMBER",
                                "description": "Design storm rainfall intensity in mm/hr (20..240, default 95)."
                            },
                            "water_capacity_lps": {
                                "type": "NUMBER",
                                "description": "Municipal water supply booster capacity in L/s (10..1500)."
                            },
                            "sewer_capacity_lps": {
                                "type": "NUMBER",
                                "description": "Drainage / wastewater plant capacity in L/s (20..4000)."
                            },
                            "substation_mva": {
                                "type": "NUMBER",
                                "description": "20kV district substation transformer rating in MVA (1..250)."
                            }
                        }
                    }
                },
                {
                    "name": "evaluate_road_intersections",
                    "description": "Detect road intersections (4-Way Cross, T-Junction, Roundabout) along authored IfcRoad corridors and evaluate HCM Level of Service delay.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "prefer_roundabouts": {
                                "type": "BOOLEAN",
                                "description": "Upgrade eligible >=3-branch junctions to Modern Roundabouts (default false)."
                            },
                            "snap_radius_m": {
                                "type": "NUMBER",
                                "description": "Cluster snap tolerance in meters (4..24, default 8.5)."
                            }
                        }
                    }
                },
                {
                    "name": "promote_city_building_to_bim",
                    "description": "Convert a procedural city parcel building (or largest zoned tower) into a full editable 6-DOF FEM-ready OpenBIM project.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "zone_id": {
                                "type": "INTEGER",
                                "description": "Optional target IfcxSpatialZone ID to promote."
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
                },
                {
                    "name": "set_visibility_graphics",
                    "description": "Control BIM category visibility/locks (Columns, Beams, Walls, Slabs, Footings, Grid), isolate categories, clear overlays, or export VG-filtered IFCX.",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "mode": {
                                "type": "STRING",
                                "description": "Action: 'set_category', 'isolate', 'show_all', 'reset_all', 'clear_overlays', or 'export_vg_ifcx'."
                            },
                            "category": {
                                "type": "STRING",
                                "description": "BIM category for 'set_category' or 'isolate': 'columns', 'beams', 'walls', 'slabs', 'footings', or 'grid'."
                            },
                            "visible": { "type": "BOOLEAN", "description": "Show or hide the target category in 'set_category' mode." },
                            "locked": { "type": "BOOLEAN", "description": "Lock or unlock the target category against accidental edits in 'set_category' mode (grid cannot be locked)." }
                        },
                        "required": ["mode"]
                    }
                },
                {
                    "name": "manage_cad_layers",
                    "description": "Manage AutoCAD-compatible 2D CAD drafting layers (create, activate, configure visibility/lock, isolate, delete with migration, toggle ByLayer colors, apply AIA/ISO 13567 presets, or export DXF).",
                    "parameters": {
                        "type": "OBJECT",
                        "properties": {
                            "mode": {
                                "type": "STRING",
                                "description": "Action: 'add', 'set_active', 'configure', 'isolate', 'show_all', 'delete', 'set_by_layer', 'apply_preset', or 'export_dxf'."
                            },
                            "layer_name": {
                                "type": "STRING",
                                "description": "Layer name or ID (e.g. '0-Sketch', 'S-Grid', 'A-Anno', 'C-Survey', or custom name)."
                            },
                            "visible": { "type": "BOOLEAN", "description": "Toggle layer visibility in 'configure' mode." },
                            "locked": { "type": "BOOLEAN", "description": "Lock or unlock layer entities against edits/deletion in 'configure' mode." },
                            "by_layer": { "type": "BOOLEAN", "description": "Enable or disable AutoCAD ByLayer color mode in 'set_by_layer' mode." },
                            "preset": { "type": "STRING", "description": "Layer standard bundle for 'apply_preset' mode: 'aia' or 'iso13567'." },
                            "color_hex": { "type": "STRING", "description": "Optional '#RRGGBB' stroke accent color when adding a custom layer." }
                        },
                        "required": ["mode"]
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

fn normalize_bim_vg_category(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "column" | "columns" | "col" | "cols" => Some("columns"),
        "beam" | "beams" | "run" | "runs" | "girder" | "girders" => Some("beams"),
        "wall" | "walls" => Some("walls"),
        "slab" | "slabs" | "floor" | "floors" | "roof" => Some("slabs"),
        "footing" | "footings" | "foundation" | "foundations" => Some("footings"),
        "grid" | "grids" | "axis" | "axes" => Some("grid"),
        _ => None,
    }
}

fn parse_hex_rgb_or_default(hex: Option<&str>, default: [u8; 3]) -> [u8; 3] {
    let Some(raw) = hex.map(|s| s.trim().trim_start_matches('#')) else {
        return default;
    };
    if raw.len() != 6 {
        return default;
    }
    let r = u8::from_str_radix(&raw[0..2], 16);
    let g = u8::from_str_radix(&raw[2..4], 16);
    let b = u8::from_str_radix(&raw[4..6], 16);
    match (r, g, b) {
        (Ok(r), Ok(g), Ok(b)) => [r, g, b],
        _ => default,
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
    execute_headless_tool_with_city(
        call,
        bim_bus,
        site,
        site_lat_lon,
        active_story_idx,
        section_box_enabled,
        None,
        None,
        0.0,
    )
}

/// Executes a structured engineering tool call with optional urban GIS scene & TIN terrain context.
#[allow(clippy::too_many_arguments)]
pub fn execute_headless_tool_with_city(
    call: &GeminiToolCall,
    bim_bus: &mut BimCommandBus,
    site: &SiteEnvironmentalCoupling,
    site_lat_lon: (f64, f64),
    active_story_idx: usize,
    section_box_enabled: bool,
    city_scene: Option<&crate::core::gis::IfcxCityScene>,
    tin: Option<&crate::core::gis::TinSurfaceMesh>,
    sea_level_m: f32,
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
                vg_command: None,
                cad_layer_command: None,
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
                        vg_command: None,
                        cad_layer_command: None,
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
                vg_command: None,
                cad_layer_command: None,
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
                vg_command: None,
                cad_layer_command: None,
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
                vg_command: None,
                cad_layer_command: None,
            }
        }
        "run_clash_detection" => {
            use crate::core::clash_engine::{evaluate_spatial_clashes, SpatialClashConfig};
            use crate::core::mep_engine::{evaluate_mep_systems, MepSystemsConfig};
            let solar_rep = evaluate_solar_envelope(
                &bim_bus.project,
                Some(site),
                None,
                &SolarEnvelopeConfig::default(),
            );
            let mut mep_cfg = MepSystemsConfig::default();
            let mut clash_cfg = SpatialClashConfig::default();
            if let Some(tol) = arg_f64(&call.args, "clearance_mm") {
                clash_cfg.clearance_tolerance_mm = tol.clamp(10.0, 250.0);
            }
            let auto_resolve = call
                .args
                .get("auto_resolve")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mep_rep =
                evaluate_mep_systems(&bim_bus.project, Some(site), Some(&solar_rep), &mep_cfg);
            let mut clash_rep = evaluate_spatial_clashes(&bim_bus.project, &mep_rep, &clash_cfg);
            if auto_resolve && clash_rep.recommended_plenum_drop_m > mep_cfg.plenum_drop_below_slab_m {
                mep_cfg.plenum_drop_below_slab_m = clash_rep.recommended_plenum_drop_m;
                let mep_fixed =
                    evaluate_mep_systems(&bim_bus.project, Some(site), Some(&solar_rep), &mep_cfg);
                clash_rep = evaluate_spatial_clashes(&bim_bus.project, &mep_fixed, &clash_cfg);
            }
            let top_clashes: Vec<Value> = clash_rep
                .clashes
                .iter()
                .take(8)
                .map(|c| {
                    json!({
                        "bcf_guid": c.bcf_guid,
                        "severity": c.severity.short_label(),
                        "pair": format!("{} vs {}", c.element_a_tag, c.element_b_tag),
                        "penetration_mm": c.penetration_mm,
                        "action": c.recommended_action,
                    })
                })
                .collect();
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: clash_rep.summary_verdict.clone(),
                result_json: json!({
                    "ok": true,
                    "hard_clashes": clash_rep.hard_clashes,
                    "clearance_warnings": clash_rep.clearance_warnings,
                    "sleeve_advisories": clash_rep.sleeve_advisories,
                    "max_penetration_mm": clash_rep.max_penetration_mm,
                    "recommended_plenum_drop_m": clash_rep.recommended_plenum_drop_m,
                    "coordination_pass": clash_rep.coordination_pass,
                    "auto_resolved": auto_resolve,
                    "top_clashes": top_clashes,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
                vg_command: None,
                cad_layer_command: None,
            }
        }
        "run_urban_utilities" => {
            use crate::core::gis::{
                evaluate_urban_utilities, IfcxCityScene, UrbanUtilitiesConfig,
            };
            let mut cfg = UrbanUtilitiesConfig::default();
            if let Some(r) = arg_f64(&call.args, "rainfall_mm_hr") {
                cfg.design_rainfall_mm_hr = r.clamp(20.0, 240.0);
            }
            if let Some(w) = arg_f64(&call.args, "water_capacity_lps") {
                cfg.water_supply_capacity_lps = w.clamp(10.0, 1500.0);
            }
            if let Some(s) = arg_f64(&call.args, "sewer_capacity_lps") {
                cfg.sewer_plant_capacity_lps = s.clamp(20.0, 4000.0);
            }
            if let Some(mva) = arg_f64(&call.args, "substation_mva") {
                cfg.substation_capacity_mva = mva.clamp(1.0, 250.0);
            }
            let fallback_city = IfcxCityScene::default();
            let city = city_scene.unwrap_or(&fallback_city);
            let rep = evaluate_urban_utilities(
                city,
                &bim_bus.project,
                tin,
                sea_level_m,
                Some(site),
                &cfg,
            );
            let b = &rep.balance;
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: rep.summary_verdict.clone(),
                result_json: json!({
                    "ok": true,
                    "road_corridor_count": rep.road_corridor_count,
                    "connected_parcels": rep.connected_parcels,
                    "water_peak_demand_lps": b.water_peak_demand_lps,
                    "water_utilization_pct": b.water_utilization_pct,
                    "water_trunk_dn_mm": b.water_trunk_dn_mm,
                    "sewer_total_demand_lps": b.total_drainage_demand_lps,
                    "sewer_utilization_pct": b.sewer_utilization_pct,
                    "sewer_trunk_dn_mm": b.sewer_trunk_dn_mm,
                    "power_demand_mva": b.power_demand_mva,
                    "power_utilization_pct": b.power_utilization_pct,
                    "mv_cable_mm2": b.mv_cable_mm2,
                    "estimated_infra_capex_usd": rep.estimated_infra_capex_usd,
                    "all_utilities_adequate": rep.all_utilities_adequate,
                    "segment_count": rep.segments_3d.len(),
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
                vg_command: None,
                cad_layer_command: None,
            }
        }
        "evaluate_road_intersections" => {
            use crate::core::gis::{
                detect_and_evaluate_road_intersections, CityIntersectionConfig, IfcxCityScene,
            };
            let mut cfg = CityIntersectionConfig::default();
            if let Some(rb) = call
                .args
                .get("prefer_roundabouts")
                .and_then(|v| v.as_bool())
            {
                cfg.prefer_roundabouts = rb;
            }
            if let Some(sr) = arg_f64(&call.args, "snap_radius_m") {
                cfg.snap_radius_m = sr.clamp(4.0, 24.0);
            }
            let fallback_city = IfcxCityScene::default();
            let city = city_scene.unwrap_or(&fallback_city);
            let rep = detect_and_evaluate_road_intersections(city, tin, sea_level_m, &cfg);
            let junctions: Vec<Value> = rep
                .intersections
                .iter()
                .take(8)
                .map(|j| {
                    json!({
                        "name": j.name,
                        "kind": j.kind.short_label(),
                        "branches": j.branch_count,
                        "peak_vph": j.peak_vph,
                        "avg_delay_sec": j.avg_delay_sec,
                        "los": j.los.short_label(),
                    })
                })
                .collect();
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: rep.summary_verdict.clone(),
                result_json: json!({
                    "ok": true,
                    "total_intersections": rep.total_intersections,
                    "roundabout_count": rep.roundabout_count,
                    "signalized_count": rep.signalized_count,
                    "worst_los": rep.worst_los.short_label(),
                    "avg_delay_sec": rep.avg_delay_sec,
                    "all_los_acceptable": rep.all_los_acceptable,
                    "junctions": junctions,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
                vg_command: None,
                cad_layer_command: None,
            }
        }
        "promote_city_building_to_bim" => {
            use crate::core::gis::{
                promote_city_building_to_bim, IfcxSpatialZone, IfcxZoneKind, ZoneDensityTier,
            };
            let requested_id = arg_usize(&call.args, "zone_id");
            let candidate_zone = city_scene
                .and_then(|c| {
                    if let Some(zid) = requested_id {
                        c.zones.iter().find(|z| z.id == zid).cloned()
                    } else {
                        c.zones
                            .iter()
                            .filter(|z| Some(z.id) != c.promoted_zone_id)
                            .filter_map(|z| z.synthesize_procedural_building().map(|b| (z, b.gfa_m2)))
                            .max_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|(z, _)| z.clone())
                    }
                })
                .unwrap_or_else(|| IfcxSpatialZone {
                    id: requested_id.unwrap_or(1),
                    name: "Zone_Com_Promo".to_string(),
                    kind: IfcxZoneKind::Commercial,
                    center_enu_m: [0.0, 0.0],
                    width_m: 28.0,
                    depth_m: 24.0,
                    rotation_deg: 0.0,
                    synthesize_context_towers: true,
                    tree_density: 2,
                    density: ZoneDensityTier::MediumDensity,
                });
            let Some((new_bim, summary)) =
                promote_city_building_to_bim(&candidate_zone, site_lat_lon)
            else {
                return ToolExecutionOutcome::err(
                    name,
                    "Selected parcel does not contain a promotable procedural building.",
                );
            };
            bim_bus.project = new_bim;
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: summary.summary_line.clone(),
                result_json: json!({
                    "ok": true,
                    "zone_id": summary.zone_id,
                    "building_name": summary.building_name,
                    "zone_kind": summary.zone_kind_label,
                    "floors": summary.floors,
                    "gfa_m2": summary.gfa_m2,
                    "columns": summary.columns,
                    "beams": summary.beams,
                    "walls": summary.walls,
                    "levels": summary.levels,
                }),
                mutated_bim: true,
                sync_fem_and_earth: true,
                fit_camera: true,
                section_box_update: None,
                fly_target: Some((summary.target_lat_lon.0, summary.target_lat_lon.1, 17.8)),
                vg_command: None,
                cad_layer_command: None,
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
                    vg_command: None,
                    cad_layer_command: None,
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
                    vg_command: None,
                    cad_layer_command: None,
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
                    vg_command: None,
                    cad_layer_command: None,
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
                vg_command: None,
                cad_layer_command: None,
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
                vg_command: None,
                cad_layer_command: None,
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
                vg_command: None,
                cad_layer_command: None,
            }
        }
        "set_visibility_graphics" => {
            let mode = call
                .args
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("set_category")
                .trim()
                .to_ascii_lowercase();
            let raw_cat = call.args.get("category").and_then(|v| v.as_str());
            let visible = call.args.get("visible").and_then(|v| v.as_bool());
            let locked = call.args.get("locked").and_then(|v| v.as_bool());
            let (cmd, summary) = match mode.as_str() {
                "reset_all" | "reset" => (
                    ToolVgCommand::ResetAll,
                    "Reset BIM Visibility/Graphics: all categories visible & unlocked".to_string(),
                ),
                "show_all" => (
                    ToolVgCommand::ShowAllBim,
                    "Revealed all BIM Visibility/Graphics categories".to_string(),
                ),
                "clear_overlays" | "clear" => (
                    ToolVgCommand::ClearOverlays,
                    "Cleared active 3D engineering overlays".to_string(),
                ),
                "export_vg_ifcx" | "export_ifcx" => (
                    ToolVgCommand::ExportVgIfcx,
                    "Exported VG-filtered IFCX (IFC 5) OpenBIM snapshot".to_string(),
                ),
                "isolate" => {
                    let Some(cat) = raw_cat.and_then(normalize_bim_vg_category) else {
                        return ToolExecutionOutcome::err(
                            name,
                            "isolate mode requires a valid 'category' ('columns', 'beams', 'walls', 'slabs', 'footings', or 'grid').",
                        );
                    };
                    (
                        ToolVgCommand::IsolateCategory {
                            category: cat.to_string(),
                        },
                        format!("Isolated BIM Visibility/Graphics category '{cat}'"),
                    )
                }
                "set_category" | "set" => {
                    let Some(cat) = raw_cat.and_then(normalize_bim_vg_category) else {
                        return ToolExecutionOutcome::err(
                            name,
                            "set_category mode requires a valid 'category' ('columns', 'beams', 'walls', 'slabs', 'footings', or 'grid').",
                        );
                    };
                    if visible.is_none() && locked.is_none() {
                        return ToolExecutionOutcome::err(
                            name,
                            "set_category mode requires 'visible' and/or 'locked' boolean.",
                        );
                    }
                    if cat == "grid" && locked.is_some() && visible.is_none() {
                        return ToolExecutionOutcome::err(
                            name,
                            "Structural grid axis visibility can be toggled, but grid axes cannot be locked.",
                        );
                    }
                    (
                        ToolVgCommand::SetCategory {
                            category: cat.to_string(),
                            visible,
                            locked,
                        },
                        format!(
                            "Updated BIM VG '{cat}' (visible={visible:?}, locked={locked:?})"
                        ),
                    )
                }
                _ => {
                    return ToolExecutionOutcome::err(
                        name,
                        "Unknown mode for set_visibility_graphics. Use 'set_category', 'isolate', 'show_all', 'reset_all', 'clear_overlays', or 'export_vg_ifcx'.",
                    );
                }
            };
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: summary.clone(),
                result_json: json!({
                    "ok": true,
                    "mode": mode,
                    "category": raw_cat,
                    "visible": visible,
                    "locked": locked,
                    "message": summary,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
                vg_command: Some(cmd),
                cad_layer_command: None,
            }
        }
        "manage_cad_layers" => {
            let mode = call
                .args
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("set_active")
                .trim()
                .to_ascii_lowercase();
            let layer_name = call
                .args
                .get("layer_name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let visible = call.args.get("visible").and_then(|v| v.as_bool());
            let locked = call.args.get("locked").and_then(|v| v.as_bool());
            let color_rgb = parse_hex_rgb_or_default(
                call.args.get("color_hex").and_then(|v| v.as_str()),
                [56, 189, 248],
            );
            let (cmd, summary) = match mode.as_str() {
                "add" | "create" => {
                    let Some(lname) = layer_name else {
                        return ToolExecutionOutcome::err(name, "add mode requires non-empty 'layer_name'.");
                    };
                    (
                        ToolCadLayerCommand::AddLayer {
                            name: lname.to_string(),
                            color_rgb,
                        },
                        format!("Created & activated CAD layer '{lname}' (RGB {:?})", color_rgb),
                    )
                }
                "set_active" | "activate" => {
                    let Some(lname) = layer_name else {
                        return ToolExecutionOutcome::err(name, "set_active mode requires 'layer_name'.");
                    };
                    (
                        ToolCadLayerCommand::SetActiveLayer {
                            name: lname.to_string(),
                        },
                        format!("Activated CAD drafting layer '{lname}'"),
                    )
                }
                "configure" | "set" => {
                    let Some(lname) = layer_name else {
                        return ToolExecutionOutcome::err(name, "configure mode requires 'layer_name'.");
                    };
                    if visible.is_none() && locked.is_none() {
                        return ToolExecutionOutcome::err(
                            name,
                            "configure mode requires 'visible' and/or 'locked' boolean.",
                        );
                    }
                    (
                        ToolCadLayerCommand::ConfigureLayer {
                            name: lname.to_string(),
                            visible,
                            locked,
                        },
                        format!("Configured CAD layer '{lname}' (visible={visible:?}, locked={locked:?})"),
                    )
                }
                "export_dxf" | "dxf" => (
                    ToolCadLayerCommand::ExportDxf,
                    "Exported layer-aware AutoCAD AC1015 .dxf sheet".to_string(),
                ),
                "set_by_layer" | "bylayer" => {
                    let Some(enabled) = call.args.get("by_layer").and_then(|v| v.as_bool()) else {
                        return ToolExecutionOutcome::err(
                            name,
                            "set_by_layer mode requires boolean 'by_layer'.",
                        );
                    };
                    (
                        ToolCadLayerCommand::SetByLayer { enabled },
                        format!("Set CAD ByLayer color override -> {enabled}"),
                    )
                }
                "isolate" | "solo" => {
                    let Some(lname) = layer_name else {
                        return ToolExecutionOutcome::err(name, "isolate mode requires 'layer_name'.");
                    };
                    (
                        ToolCadLayerCommand::IsolateLayer {
                            name: lname.to_string(),
                        },
                        format!("Isolated CAD layer '{lname}'"),
                    )
                }
                "show_all" => (
                    ToolCadLayerCommand::ShowAllLayers,
                    "Revealed all 2D CAD drafting layers".to_string(),
                ),
                "delete" | "remove" => {
                    let Some(lname) = layer_name else {
                        return ToolExecutionOutcome::err(name, "delete mode requires 'layer_name'.");
                    };
                    if lname.eq_ignore_ascii_case("0-sketch") {
                        return ToolExecutionOutcome::err(
                            name,
                            "Base CAD layer '0-Sketch' cannot be deleted.",
                        );
                    }
                    (
                        ToolCadLayerCommand::DeleteLayer {
                            name: lname.to_string(),
                        },
                        format!("Deleted custom CAD layer '{lname}' (entities migrate to 0-Sketch)"),
                    )
                }
                "apply_preset" | "preset" => {
                    let raw_preset = call
                        .args
                        .get("preset")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_ascii_lowercase();
                    let norm = match raw_preset.as_str() {
                        "aia" | "aia_structural" | "ncs" => "aia",
                        "iso" | "iso13567" | "iso_13567" => "iso13567",
                        _ => {
                            return ToolExecutionOutcome::err(
                                name,
                                "apply_preset mode requires 'preset' as 'aia' or 'iso13567'.",
                            );
                        }
                    };
                    (
                        ToolCadLayerCommand::ApplyPreset {
                            preset: norm.to_string(),
                        },
                        format!("Applied CAD standard layer preset '{norm}'"),
                    )
                }
                _ => {
                    return ToolExecutionOutcome::err(
                        name,
                        "Unknown mode for manage_cad_layers. Use 'add', 'set_active', 'configure', 'isolate', 'show_all', 'delete', 'set_by_layer', 'apply_preset', or 'export_dxf'.",
                    );
                }
            };
            ToolExecutionOutcome {
                name: name.to_string(),
                ok: true,
                summary: summary.clone(),
                result_json: json!({
                    "ok": true,
                    "mode": mode,
                    "layer_name": layer_name,
                    "visible": visible,
                    "locked": locked,
                    "color_rgb": color_rgb,
                    "message": summary,
                }),
                mutated_bim: false,
                sync_fem_and_earth: false,
                fit_camera: false,
                section_box_update: None,
                fly_target: None,
                vg_command: None,
                cad_layer_command: Some(cmd),
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
        assert_eq!(fn_decls.len(), 17);
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

        let clash = execute_headless_tool(
            &GeminiToolCall {
                name: "run_clash_detection".to_string(),
                args: json!({ "clearance_mm": 60.0, "auto_resolve": true }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(clash.ok);
        assert!(clash.result_json["recommended_plenum_drop_m"].as_f64().is_some());

        let util = execute_headless_tool(
            &GeminiToolCall {
                name: "run_urban_utilities".to_string(),
                args: json!({ "rainfall_mm_hr": 110.0, "water_capacity_lps": 80.0 }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(util.ok);
        assert!(util.result_json["water_trunk_dn_mm"].as_u64().unwrap_or(0) >= 150);
        assert!(util.result_json["segment_count"].as_u64().unwrap_or(0) >= 6);

        let inter = execute_headless_tool(
            &GeminiToolCall {
                name: "evaluate_road_intersections".to_string(),
                args: json!({ "prefer_roundabouts": true }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(inter.ok);
        assert!(inter.result_json["total_intersections"].as_u64().is_some());

        let promo = execute_headless_tool(
            &GeminiToolCall {
                name: "promote_city_building_to_bim".to_string(),
                args: json!({}),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(promo.ok && promo.mutated_bim && promo.sync_fem_and_earth);
        assert!(promo.result_json["columns"].as_u64().unwrap_or(0) >= 6);

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

        let vg = execute_headless_tool(
            &GeminiToolCall {
                name: "set_visibility_graphics".to_string(),
                args: json!({ "mode": "set_category", "category": "columns", "locked": true, "visible": true }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(vg.ok);
        assert_eq!(
            vg.vg_command,
            Some(ToolVgCommand::SetCategory {
                category: "columns".to_string(),
                visible: Some(true),
                locked: Some(true),
            })
        );

        let cad = execute_headless_tool(
            &GeminiToolCall {
                name: "manage_cad_layers".to_string(),
                args: json!({ "mode": "add", "layer_name": "S-FOUNDATION-GRID", "color_hex": "#22C55E" }),
            },
            &mut bus,
            &site,
            (-6.2088, 106.8456),
            0,
            false,
        );
        assert!(cad.ok);
        assert_eq!(
            cad.cad_layer_command,
            Some(ToolCadLayerCommand::AddLayer {
                name: "S-FOUNDATION-GRID".to_string(),
                color_rgb: [34, 197, 94],
            })
        );

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
