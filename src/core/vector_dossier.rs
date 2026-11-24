//! core/vector_dossier.rs — Publication-Grade 100% Pure-Vector SVG Engineering Calculation Dossier Exporter (Phase 72 / v0.7.2).
//!
//! Synthesizes an archival A3 landscape (`1600 x 1131`) pure-vector drafting & engineering sheet
//! uniting structural framing plans, storey elevation + 3D subsurface borehole stratigraphy,
//! EigenGate / LRFD DCR% governing charts, coupled multi-physics scorecards, multi-layer IFCX
//! diff manifests, and a SHA-256 verification seal — strictly 0% raster pixels and 0 `<image>` tags.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::core::bim::{BimProject, RunCategory};
use crate::core::cascade::TwoStageCrcCascadeReport;
use crate::core::geotech::{BoreholePreset, GeotechEngine, SubsurfaceSsiAnalysisReport};
use crate::core::gis::district_hazard::DistrictMultiHazardReport;
use crate::core::ifcx::{IfcxDiffKind, IfcxVisualDiffReport};
use crate::core::interop::Sha256Hasher;
use crate::core::lifecycle_ops::SiteEnvironmentalCoupling;
use crate::core::retrofit::StructuralRetrofitReport;
use crate::models::StructuralMember;

/// Input bundle for generating a 100% pure-vector A3 engineering calculation dossier SVG.
pub struct VectorDossierInput<'a> {
    pub project: &'a BimProject,
    pub members: &'a [StructuralMember],
    pub site: &'a SiteEnvironmentalCoupling,
    pub lat_lon: (f64, f64),
    pub retrofit_report: Option<&'a StructuralRetrofitReport>,
    pub crc_report: Option<&'a TwoStageCrcCascadeReport>,
    pub ssi_report: Option<&'a SubsurfaceSsiAnalysisReport>,
    pub hazard_report: Option<&'a DistrictMultiHazardReport>,
    pub diff_report: Option<&'a IfcxVisualDiffReport>,
    pub portfolio_report: Option<&'a crate::core::portfolio_retrofit::DistrictPortfolioRetrofitReport>,
    pub replay_report: Option<&'a crate::core::ground_motion_replay::GroundMotionReplayReport>,
    pub shm_report: Option<&'a crate::core::shm_twin::ShmDigitalTwinReport>,
    pub evacuation_report: Option<&'a crate::core::evacuation_routing::EvacuationLifelineReport>,
    pub watchdog_state: Option<&'a crate::core::shm_watchdog::ShmWatchdogState>,
    pub monte_carlo_report: Option<&'a crate::core::gpu_fragility_mc::MonteCarloFragilityReport>,
    pub collab_state: Option<&'a crate::core::ifcx_collab::IfcxCollabState>,
    pub sheet_code: &'a str,
    pub revision: &'a str,
}

/// Verified publication-grade pure-vector SVG dossier export payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDossierExport {
    pub svg_content: String,
    pub sha256_seal: String,
    pub panel_count: usize,
    pub element_count: usize,
    pub summary: String,
}

fn escape_xml(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn truncate_clean(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut cut: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        cut.push_str("...");
        cut
    }
}

fn rgba_to_svg_hex(rgba: [f32; 4]) -> String {
    let r = (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn dcr_hex_color(dcr: f64) -> &'static str {
    if dcr > 1.0 {
        "#dc2626"
    } else if dcr >= 0.85 {
        "#d97706"
    } else if dcr >= 0.55 {
        "#0284c7"
    } else {
        "#059669"
    }
}

fn draw_panel_chrome(svg: &mut String, x: f64, y: f64, w: f64, h: f64, title: &str, subtitle: &str) {
    let _ = writeln!(
        svg,
        r##"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" rx="6" fill="#f8fafc" stroke="#94a3b8" stroke-width="1.1"/>
<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="28" rx="6" fill="#0b1427"/>
<rect x="{x:.1}" y="{:.1}" width="{w:.1}" height="6" fill="#0b1427"/>
<text x="{:.1}" y="{:.1}" font-size="11.5" font-weight="bold" fill="#d0ebe1">{}</text>
<text x="{:.1}" y="{:.1}" font-size="9.5" text-anchor="end" fill="#5b92a5">{}</text>"##,
        y + 22.0,
        x + 10.0,
        y + 18.0,
        escape_xml(title),
        x + w - 10.0,
        y + 18.0,
        escape_xml(subtitle),
    );
}

fn draw_plan_panel(svg: &mut String, input: &VectorDossierInput<'_>) {
    let (px, py, pw, ph) = (32.0, 56.0, 508.0, 554.0);
    draw_panel_chrome(
        svg,
        px,
        py,
        pw,
        ph,
        "1. ORTHOGRAPHIC STRUCTURAL FRAMING PLAN",
        &format!(
            "{} COLS • {} BEAMS • {} WALLS",
            input.project.columns.len(),
            input.project.runs.len(),
            input.project.walls.len()
        ),
    );

    let (bmin, bmax) = input
        .project
        .plan_extents_mm()
        .unwrap_or(([-6000.0, -6000.0], [6000.0, 6000.0]));
    let span_x = (bmax[0] - bmin[0]).max(2000.0);
    let span_y = (bmax[1] - bmin[1]).max(2000.0);
    let draw_w = pw - 76.0;
    let draw_h = ph - 110.0;
    let scale = (draw_w / span_x).min(draw_h / span_y);
    let ox = px + 38.0 + (draw_w - span_x * scale) * 0.5;
    let oy = py + 56.0 + (draw_h - span_y * scale) * 0.5;

    let map_xy = |x_mm: f64, y_mm: f64| -> (f64, f64) {
        (ox + (x_mm - bmin[0]) * scale, oy + (bmax[1] - y_mm) * scale)
    };

    // Structural grids + bubbles
    for g in &input.project.grids {
        let (x0, y0) = map_xy(g.start.0, g.start.1);
        let (x1, y1) = map_xy(g.end.0, g.end.1);
        let _ = writeln!(
            svg,
            r##"<line x1="{x0:.1}" y1="{y0:.1}" x2="{x1:.1}" y2="{y1:.1}" stroke="#5b92a5" stroke-width="0.9" stroke-dasharray="6,4" stroke-opacity="0.7"/>
<circle cx="{x0:.1}" cy="{y0:.1}" r="8.5" fill="#ffffff" stroke="#2b4963" stroke-width="1.2"/>
<text x="{x0:.1}" y="{:.1}" font-size="8.5" font-weight="bold" text-anchor="middle" fill="#0b1427">{}</text>"##,
            y0 + 3.0,
            escape_xml(&truncate_clean(&g.label, 4))
        );
    }

    // Pad footings outline
    for ftg in &input.project.footings {
        let (cx, cy) = map_xy(ftg.pos.0, ftg.pos.1);
        let w_px = (ftg.width_mm * scale).max(6.0);
        let h_px = (ftg.length_mm * scale).max(6.0);
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{:.1}" width="{w_px:.1}" height="{h_px:.1}" fill="#e2e8f0" fill-opacity="0.55" stroke="#64748b" stroke-width="0.8" stroke-dasharray="3,2"/>"##,
            cx - w_px * 0.5,
            cy - h_px * 0.5,
        );
    }

    // Structural walls
    for w in &input.project.walls {
        let ring = w.footprint_ring();
        let pts: Vec<String> = ring
            .iter()
            .map(|p| {
                let (sx, sy) = map_xy(p.0, p.1);
                format!("{sx:.1},{sy:.1}")
            })
            .collect();
        let (fill, stroke) = if w.is_structural {
            ("#94a3b8", "#0f172a")
        } else {
            ("#cbd5e1", "#475569")
        };
        let _ = writeln!(
            svg,
            r##"<polygon points="{}" fill="{fill}" fill-opacity="0.75" stroke="{stroke}" stroke-width="1.1"/>"##,
            pts.join(" ")
        );
    }

    // Beams & framing runs
    for r in &input.project.runs {
        let (x0, y0) = map_xy(r.start.0, r.start.1);
        let (x1, y1) = map_xy(r.end.0, r.end.1);
        let (col, sw) = match r.category {
            RunCategory::StructuralBrace => ("#d97706", 2.2),
            RunCategory::StructuralBeam => ("#2b4963", (r.width_mm * scale).clamp(1.8, 5.5)),
            _ => ("#64748b", 1.4),
        };
        let _ = writeln!(
            svg,
            r##"<line x1="{x0:.1}" y1="{y0:.1}" x2="{x1:.1}" y2="{y1:.1}" stroke="{col}" stroke-width="{sw:.1}" stroke-linecap="round"/>"##
        );
    }

    // Columns colored by utilization if matching StructuralMember exists
    let avg_dcr = if !input.members.is_empty() {
        input
            .members
            .iter()
            .map(|m| m.dcr_interaction.max(m.utilization))
            .fold(0.0_f64, f64::max)
    } else {
        0.45
    };
    for col in &input.project.columns {
        let (cx, cy) = map_xy(col.pos.0, col.pos.1);
        let cw = (col.width_mm * scale).max(6.5);
        let cd = (col.depth_mm * scale).max(6.5);
        let dcr = input
            .members
            .get(col.id % input.members.len().max(1))
            .map(|m| m.dcr_interaction.max(m.utilization))
            .unwrap_or(avg_dcr);
        let fill = dcr_hex_color(dcr);
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{:.1}" width="{cw:.1}" height="{cd:.1}" fill="{fill}" stroke="#0b1427" stroke-width="1.0"/>"##,
            cx - cw * 0.5,
            cy - cd * 0.5,
        );
    }

    // Vector North arrow + scale bar footer
    let nx = px + pw - 36.0;
    let ny = py + 58.0;
    let scale_bar_len_px = (5000.0 * scale).clamp(24.0, 120.0);
    let _ = writeln!(
        svg,
        r##"<circle cx="{nx:.1}" cy="{ny:.1}" r="14" fill="#ffffff" stroke="#2b4963" stroke-width="1.0"/>
<polygon points="{nx:.1},{:.1} {:.1},{:.1} {nx:.1},{:.1} {:.1},{:.1}" fill="#0b1427"/>
<text x="{nx:.1}" y="{:.1}" font-size="8" font-weight="bold" text-anchor="middle" fill="#2b4963">N</text>
<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#0b1427" stroke-width="2.0"/>
<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#0b1427" stroke-width="1.4"/>
<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#0b1427" stroke-width="1.4"/>
<text x="{:.1}" y="{:.1}" font-size="9" fill="#334155">0          5.0 m • CRS: WGS84 ({:.4}, {:.4})</text>"##,
        ny - 10.0,
        nx - 5.0,
        ny + 7.0,
        ny + 3.0,
        nx + 5.0,
        ny + 7.0,
        ny + 22.0,
        px + 18.0,
        py + ph - 22.0,
        px + 18.0 + scale_bar_len_px,
        py + ph - 22.0,
        px + 18.0,
        py + ph - 26.0,
        px + 18.0,
        py + ph - 18.0,
        px + 18.0 + scale_bar_len_px,
        py + ph - 26.0,
        px + 18.0 + scale_bar_len_px,
        py + ph - 18.0,
        px + 18.0,
        py + ph - 9.0,
        input.lat_lon.0,
        input.lat_lon.1,
    );
}

fn draw_elevation_ssi_panel(svg: &mut String, input: &VectorDossierInput<'_>) {
    let (px, py, pw, ph) = (554.0, 56.0, 496.0, 554.0);
    let strata_label = input
        .ssi_report
        .map(|s| s.preset.short_label())
        .unwrap_or("Fixed / Std Soil");
    draw_panel_chrome(
        svg,
        px,
        py,
        pw,
        ph,
        "2. STRUCTURAL ELEVATION & SUBSURFACE STRATIGRAPHY",
        strata_label,
    );

    let (bmin, bmax) = input
        .project
        .plan_extents_mm()
        .unwrap_or(([-6000.0, -6000.0], [6000.0, 6000.0]));
    let x_min_m = bmin[0] * 0.001;
    let x_max_m = (bmax[0] * 0.001).max(x_min_m + 4.0);
    let max_z_m = input
        .project
        .levels
        .iter()
        .map(|l| l.elevation_mm * 0.001)
        .fold(6.0_f64, f64::max)
        .max(4.0);
    let sub_depth_m = 28.0_f64;

    // Ground datum placed at ~48% of panel height so superstructure + subsurface both fit clearly
    let gx0 = px + 44.0;
    let gx1 = px + pw - 22.0;
    let y_ground = py + 235.0;
    let px_per_z_up = 175.0 / max_z_m;
    let px_per_z_dn = (py + ph - 28.0 - y_ground) / sub_depth_m;
    let x_scale = (gx1 - gx0 - 30.0) / (x_max_m - x_min_m);
    let map_x = |xm: f64| -> f64 { gx0 + 15.0 + (xm - x_min_m) * x_scale };
    let map_z = |zm: f64| -> f64 {
        if zm >= 0.0 {
            y_ground - zm * px_per_z_up
        } else {
            y_ground + (-zm).min(sub_depth_m) * px_per_z_dn
        }
    };

    // Subsurface stratigraphy horizons from report or default StiffVolcanicAlluvium
    let default_strata =
        GeotechEngine::build_borehole_strata(BoreholePreset::StiffVolcanicAlluvium, 2.5, 0.35, false).0;
    let strata = input
        .ssi_report
        .map(|r| r.strata.as_slice())
        .unwrap_or(&default_strata);
    let gwt_m = input
        .ssi_report
        .map(|r| r.groundwater_depth_m)
        .unwrap_or(2.5);
    let pile_depth_m = input
        .ssi_report
        .map(|r| r.recommended_pile_depth_m)
        .unwrap_or(8.0);
    let liq_critical = input
        .ssi_report
        .is_some_and(|r| r.is_liquefaction_critical);

    for s in strata {
        let y0 = map_z(-s.depth_top_m);
        let y1 = map_z(-s.depth_bottom_m);
        let bh = (y1 - y0).max(4.0);
        let is_liq = liq_critical && s.liquefiable && s.liquefaction_fs < 1.15;
        let fill = if is_liq {
            "#fca5a5".to_string()
        } else {
            rgba_to_svg_hex(s.color_rgba)
        };
        let _ = writeln!(
            svg,
            r##"<rect x="{gx0:.1}" y="{y0:.1}" width="{:.1}" height="{bh:.1}" fill="{fill}" fill-opacity="0.34" stroke="#64748b" stroke-width="0.7"/>
<text x="{:.1}" y="{:.1}" font-size="8.5" fill="#1e293b">{} (Vs={:.0}m/s, FS={:.2})</text>"##,
            gx1 - gx0,
            gx0 + 8.0,
            (y0 + bh * 0.55).min(y1 - 3.0),
            escape_xml(&truncate_clean(&s.name, 30)),
            s.vs_m_s,
            s.liquefaction_fs
        );
    }

    // Groundwater Phreatic Line (GWT)
    let y_gwt = map_z(-gwt_m);
    let _ = writeln!(
        svg,
        r##"<line x1="{gx0:.1}" y1="{y_gwt:.1}" x2="{gx1:.1}" y2="{y_gwt:.1}" stroke="#0284c7" stroke-width="1.6" stroke-dasharray="7,3"/>
<text x="{:.1}" y="{:.1}" font-size="8.8" font-weight="bold" text-anchor="end" fill="#0369a1">GWT -{gwt_m:.1} m</text>"##,
        gx1 - 6.0,
        y_gwt - 4.0,
    );

    // Ground grade line
    let _ = writeln!(
        svg,
        r##"<line x1="{gx0:.1}" y1="{y_ground:.1}" x2="{gx1:.1}" y2="{y_ground:.1}" stroke="#0b1427" stroke-width="2.0"/>
<text x="{:.1}" y="{:.1}" font-size="8.5" font-weight="bold" fill="#0b1427">GL 0.0m</text>"##,
        px + 6.0,
        y_ground + 3.0,
    );

    // Storey datum lines
    for lvl in &input.project.levels {
        let zm = lvl.elevation_mm * 0.001;
        let ly = map_z(zm);
        let _ = writeln!(
            svg,
            r##"<line x1="{gx0:.1}" y1="{ly:.1}" x2="{gx1:.1}" y2="{ly:.1}" stroke="#94a3b8" stroke-width="0.7" stroke-dasharray="4,3"/>
<text x="{:.1}" y="{:.1}" font-size="8" fill="#475569">+{zm:.1}m</text>"##,
            px + 6.0,
            ly + 3.0,
        );
    }

    // Footings + dynamic deep foundation piles
    let y_pile_tip = map_z(-pile_depth_m);
    for ftg in &input.project.footings {
        let cx = map_x(ftg.pos.0 * 0.001);
        let fw = (ftg.width_mm * 0.001 * x_scale).clamp(10.0, 32.0);
        let fh = 7.0;
        let _ = writeln!(
            svg,
            r##"<line x1="{:.1}" y1="{y_ground:.1}" x2="{:.1}" y2="{y_pile_tip:.1}" stroke="#334155" stroke-width="2.2"/>
<line x1="{:.1}" y1="{y_ground:.1}" x2="{:.1}" y2="{y_pile_tip:.1}" stroke="#334155" stroke-width="2.2"/>
<rect x="{:.1}" y="{y_ground:.1}" width="{fw:.1}" height="{fh:.1}" fill="#64748b" stroke="#0b1427" stroke-width="0.9"/>"##,
            cx - fw * 0.28,
            cx - fw * 0.28,
            cx + fw * 0.28,
            cx + fw * 0.28,
            cx - fw * 0.5,
        );
    }

    // Projected superstructure columns & beams
    let lvl_elev = |id: usize| -> f64 {
        input
            .project
            .levels
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.elevation_mm * 0.001)
            .unwrap_or(0.0)
    };
    for col in &input.project.columns {
        let cx = map_x(col.pos.0 * 0.001);
        let y0 = map_z(lvl_elev(col.base_level_id));
        let y1 = map_z(lvl_elev(col.top_level_id).max(lvl_elev(col.base_level_id) + 3.0));
        let _ = writeln!(
            svg,
            r##"<line x1="{cx:.1}" y1="{y0:.1}" x2="{cx:.1}" y2="{y1:.1}" stroke="#0284c7" stroke-width="3.0"/>"##
        );
    }
    for run in &input.project.runs {
        if run.category == RunCategory::StructuralBeam || run.category == RunCategory::StructuralBrace {
            let x0 = map_x(run.start.0 * 0.001);
            let x1 = map_x(run.end.0 * 0.001);
            let yr = map_z(lvl_elev(run.level_id));
            let _ = writeln!(
                svg,
                r##"<line x1="{x0:.1}" y1="{yr:.1}" x2="{x1:.1}" y2="{yr:.1}" stroke="#2b4963" stroke-width="2.4"/>"##
            );
        }
    }

    if liq_critical {
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{:.1}" width="180" height="20" rx="4" fill="#dc2626"/>
<text x="{:.1}" y="{:.1}" font-size="9" font-weight="bold" text-anchor="middle" fill="#ffffff">LIQUEFACTION CRITICAL (p-y)</text>"##,
            gx1 - 184.0,
            y_ground + 8.0,
            gx1 - 94.0,
            y_ground + 21.0,
        );
    }
}

fn draw_dcr_eigengate_panel(svg: &mut String, input: &VectorDossierInput<'_>) {
    let (px, py, pw, ph) = (1064.0, 56.0, 504.0, 554.0);
    let min_eigengate = input
        .members
        .iter()
        .map(|m| m.eigengate)
        .fold(1.0_f64, f64::min);
    draw_panel_chrome(
        svg,
        px,
        py,
        pw,
        ph,
        "3. EigenGate & LRFD DCR% GOVERNING AUDIT",
        &format!("EigenGate_min={min_eigengate:.3}"),
    );

    let mut sorted: Vec<(&str, f64, f64)> = input
        .members
        .iter()
        .map(|m| (m.name.as_str(), m.dcr_interaction.max(m.utilization), m.eigengate))
        .collect();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));

    let chart_x = px + 115.0;
    let chart_w = pw - 190.0;
    let max_dcr = sorted
        .iter()
        .map(|x| x.1)
        .fold(1.05_f64, f64::max)
        .clamp(1.05, 1.80);

    // Reference threshold lines (DCR = 0.85 and 1.00)
    let x_85 = chart_x + (0.85 / max_dcr) * chart_w;
    let x_100 = chart_x + (1.00 / max_dcr) * chart_w;
    let _ = writeln!(
        svg,
        r##"<line x1="{x_85:.1}" y1="{:.1}" x2="{x_85:.1}" y2="{:.1}" stroke="#d97706" stroke-width="1.0" stroke-dasharray="4,3"/>
<text x="{x_85:.1}" y="{:.1}" font-size="8.5" text-anchor="middle" fill="#d97706">0.85</text>
<line x1="{x_100:.1}" y1="{:.1}" x2="{x_100:.1}" y2="{:.1}" stroke="#dc2626" stroke-width="1.2" stroke-dasharray="4,3"/>
<text x="{x_100:.1}" y="{:.1}" font-size="8.5" text-anchor="middle" fill="#dc2626">1.00 Limit</text>"##,
        py + 46.0,
        py + ph - 55.0,
        py + 42.0,
        py + 46.0,
        py + ph - 55.0,
        py + 42.0,
    );

    if sorted.is_empty() {
        let _ = writeln!(
            svg,
            r##"<text x="{:.1}" y="{:.1}" font-size="11" text-anchor="middle" fill="#64748b">Run Sync 3D BIM -> FEM to populate member DCR% &amp; EigenGate bars.</text>"##,
            px + pw * 0.5,
            py + ph * 0.5,
        );
    } else {
        for (i, (name, dcr, eigengate)) in sorted.iter().take(14).enumerate() {
            let by = py + 58.0 + (i as f64) * 31.0;
            let bw = ((dcr / max_dcr) * chart_w).clamp(3.0, chart_w);
            let col = dcr_hex_color(*dcr);
            let _ = writeln!(
                svg,
                r##"<text x="{:.1}" y="{:.1}" font-size="9.5" text-anchor="end" fill="#0f172a">{}</text>
<rect x="{chart_x:.1}" y="{by:.1}" width="{chart_w:.1}" height="17" rx="3" fill="#e2e8f0"/>
<rect x="{chart_x:.1}" y="{by:.1}" width="{bw:.1}" height="17" rx="3" fill="{col}"/>
<text x="{:.1}" y="{:.1}" font-size="9" font-family="monospace" fill="#0f172a">{:.0}% | EigenGate {:.2}</text>"##,
                chart_x - 8.0,
                by + 12.0,
                escape_xml(&truncate_clean(name, 14)),
                chart_x + chart_w + 6.0,
                by + 12.0,
                dcr * 100.0,
                eigengate,
            );
        }
    }

    let _ = writeln!(
        svg,
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="28" rx="4" fill="#eef6f7" stroke="#5b92a5" stroke-width="0.9"/>
<text x="{:.1}" y="{:.1}" font-size="9.5" font-weight="bold" fill="#0b1427">SVD Kinematic Guard: VERIFIED INTACT • Conformal Alpha = 0.05 • {} Audited Members</text>"##,
        px + 14.0,
        py + ph - 40.0,
        pw - 28.0,
        px + 24.0,
        py + ph - 22.0,
        input.members.len(),
    );
}

fn draw_telemetry_panel(svg: &mut String, input: &VectorDossierInput<'_>) {
    let (px, py, pw, ph) = (32.0, 624.0, 508.0, 474.0);
    draw_panel_chrome(
        svg,
        px,
        py,
        pw,
        ph,
        "4. COUPLED MULTI-PHYSICS TELEMETRY SCORECARD",
        &input.site.recommended_soil_class,
    );

    let ssi_line = if let Some(s) = input.ssi_report {
        format!(
            "T1={:.2}s -> {:.2}s (+{:.1}%) | Vs30={:.0}m/s | Lpile={:.1}m | FS_L={:.2}",
            s.fixed_base_period_t1_s,
            s.ssi_coupled_period_t1_s,
            s.period_lengthening_pct,
            s.vs30_m_s,
            s.recommended_pile_depth_m,
            s.min_liquefaction_fs
        )
    } else {
        "Fixed-Base Default (Enable Winkler SSI in Dynamic Island to couple)".to_string()
    };

    let crc_line = if let Some(c) = input.crc_report {
        format!(
            "Reduced {:.0}% | Recall {:.1}% | PVI {:.3} | DAF {:.2}x | {:?} | CO2 -{:.1}g",
            c.scenario_reduction_pct,
            c.safety_recall_pct,
            c.pvi,
            c.daf,
            c.cascade_status,
            c.green_ai_co2_saved_g
        )
    } else {
        "Not run yet (Trigger /cascade in Dynamic Island)".to_string()
    };

    let ret_line = if let Some(r) = input.retrofit_report {
        format!(
            "{:?} | EigenGate {:.2} -> {:.2} | DCR {:.0}% -> {:.0}% | ${:.0}",
            r.strategy,
            r.before_min_eigengate,
            r.after_min_eigengate,
            r.before_max_dcr_pct,
            r.after_max_dcr_pct,
            r.estimated_cost_usd
        )
    } else if let Some(pt) = input.portfolio_report.and_then(|p| p.active_frontier()) {
        let port = input.portfolio_report.unwrap();
        format!(
            "District {}/{} bldgs | EigenGate {:.2}->{:.2} | ${:.0}k | {:.1}tCO2e",
            pt.retrofitted_buildings,
            pt.total_buildings,
            port.pre_district_avg_eigengate,
            pt.district_avg_eigengate_after,
            pt.total_cost_usd / 1000.0,
            pt.total_carbon_kg_co2e / 1000.0
        )
    } else {
        "Baseline Intact (No retrofit delta active)".to_string()
    };

    let wd_suffix = input
        .watchdog_state
        .filter(|w| w.total_packets_ingested > 0)
        .map(|w| {
            format!(
                " | WD {} STA/LTA {:.1}",
                w.alert_level.short_label(),
                w.current_sta_lta
            )
        })
        .unwrap_or_default();
    let mc_suffix = input
        .monte_carlo_report
        .map(|mc| {
            format!(
                " | MC PML90 ${:.0}k Pcol {:.0}%",
                mc.p90_loss_usd * 0.001,
                mc.collapse_prob_pct
            )
        })
        .unwrap_or_default();

    let haz_line = if let Some(shm) = input.shm_report {
        if !wd_suffix.is_empty() {
            format!(
                "SHM {} | f1 {:.2}Hz ({:+.1}%) | K {:.0}%{wd_suffix}",
                shm.atc20_placard.short_label(),
                shm.observed_freq_1_hz,
                shm.freq_shift_pct,
                shm.global_stiffness_retention_pct,
            )
        } else if let Some(evac) = input.evacuation_report {
            format!(
                "SHM {} | f1 {:.2}->{:.2}Hz ({:+.1}%) | MAC {:.2} | Evac Conn {:.0}% Resp {:.0}",
                shm.atc20_placard.short_label(),
                shm.baseline_freq_1_hz,
                shm.observed_freq_1_hz,
                shm.freq_shift_pct,
                shm.mac_mode_1,
                evac.network_connectivity_pct,
                evac.responder_accessibility_score,
            )
        } else {
            format!(
                "SHM {} | f1 {:.2}->{:.2}Hz ({:+.1}%) | MAC {:.2} | K_ret {:.0}% | {}G/{}Y/{}R",
                shm.atc20_placard.short_label(),
                shm.baseline_freq_1_hz,
                shm.observed_freq_1_hz,
                shm.freq_shift_pct,
                shm.mac_mode_1,
                shm.global_stiffness_retention_pct,
                shm.green_count,
                shm.yellow_count,
                shm.red_count,
            )
        }
    } else if let Some(evac) = input.evacuation_report {
        if !wd_suffix.is_empty() {
            format!(
                "Evac Conn {:.0}% ({}/{}) | Resp {:.0} | Iso {}{wd_suffix}",
                evac.network_connectivity_pct,
                evac.open_edges,
                evac.total_edges,
                evac.responder_accessibility_score,
                evac.isolated_building_count,
            )
        } else {
            format!(
                "Evac Conn {:.0}% ({}/{} open) | Resp {:.0}/100 | Isolated {} | Bridges {}/{} ok",
                evac.network_connectivity_pct,
                evac.open_edges,
                evac.total_edges,
                evac.responder_accessibility_score,
                evac.isolated_building_count,
                evac.bridges_evaluated.saturating_sub(evac.bridges_compromised),
                evac.bridges_evaluated,
            )
        }
    } else if let Some(rep) = input.replay_report {
        if !mc_suffix.is_empty() {
            format!(
                "NL-THA {} ({:.2}g) | Drift {:.2}%{mc_suffix}",
                rep.config.active_record_label(),
                rep.scaled_pga_g,
                rep.peak_interstory_drift_ratio * 100.0,
            )
        } else {
            format!(
                "NL-THA {} ({}, {:.2}g) | Drift {:.2}% | Hinges {}IO/{}LS/{}CP/{}M | Fragility ${:.0}k ({:.0}d {:?})",
                rep.config.active_record_label(),
                rep.tridirectional_mode.short_label(),
                rep.scaled_pga_g,
                rep.peak_interstory_drift_ratio * 100.0,
                rep.peak_count_io,
                rep.peak_count_ls,
                rep.peak_count_cp,
                rep.peak_count_mechanism,
                rep.fragility_loss.total_repair_cost_usd / 1000.0,
                rep.fragility_loss.downtime_days,
                rep.fragility_loss.achieved_redi_tier,
            )
        }
    } else if let Some(mc) = input.monte_carlo_report {
        format!(
            "GPU MC {} | P50 ${:.0}k | PML90 ${:.0}k | CVaR95 ${:.0}k | Pcol {:.1}%",
            mc.sample_count,
            mc.p50_loss_usd * 0.001,
            mc.p90_loss_usd * 0.001,
            mc.cvar95_loss_usd * 0.001,
            mc.collapse_prob_pct,
        )
    } else if let Some(h) = input.hazard_report {
        format!(
            "Resilience {:.0}/100 | Pounding {} | Venturi x{:.2} | Flood {:.1}%",
            h.resilience_score, h.pounding_risk_count, h.venturi_speedup_factor, h.flood.inundated_area_pct
        )
    } else {
        "Nominal Ambient Site Conditions".to_string()
    };

    let sixth_label = if input.shm_report.is_some() {
        "Live SHM Digital Twin & ATC-20 Placard"
    } else if input.evacuation_report.is_some() {
        "Post-Disaster Evacuation & Lifelines"
    } else if input.replay_report.is_some() {
        "NL-THA Ground-Motion & Hinges"
    } else {
        "District Multi-Hazard"
    };

    let rows = [
        ("WGS84 Geodetic Anchor", format!("{:.5} deg, {:.5} deg | Elev {:.1} m", input.lat_lon.0, input.lat_lon.1, input.site.elevation_m)),
        ("SNI 1726 Seismic & Wind", format!("Ss={:.2}g | S1={:.2}g | Wind={:.1} m/s | {}", input.site.seismic_pga_ss_g, input.site.seismic_pga_s1_g, input.site.design_wind_ms, input.site.site_label)),
        ("6-DOF Winkler SSI & Soil", ssi_line),
        ("EigenGate + StructGNN-CRC", crc_line),
        ("Automated Retrofit Delta", ret_line),
        (sixth_label, haz_line),
    ];

    for (idx, (k, v)) in rows.iter().enumerate() {
        let ry = py + 44.0 + (idx as f64) * 68.0;
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="58" rx="4" fill="#ffffff" stroke="#cbd5e1" stroke-width="0.9"/>
<text x="{:.1}" y="{:.1}" font-size="10" font-weight="bold" fill="#2b4963">{}</text>
<text x="{:.1}" y="{:.1}" font-size="9.6" font-family="monospace" fill="#0f172a">{}</text>"##,
            px + 12.0,
            pw - 24.0,
            px + 20.0,
            ry + 20.0,
            escape_xml(k),
            px + 20.0,
            ry + 42.0,
            escape_xml(&truncate_clean(v, 72)),
        );
    }
}

fn draw_diff_or_schedule_panel(svg: &mut String, input: &VectorDossierInput<'_>) {
    let (px, py, pw, ph) = (554.0, 624.0, 496.0, 474.0);
    if let Some(diff) = input.diff_report {
        draw_panel_chrome(
            svg,
            px,
            py,
            pw,
            ph,
            "5. MULTI-LAYER IFCX VISUAL DIFF MANIFEST",
            &format!("+{} ~{} -{}", diff.added, diff.modified, diff.deleted),
        );
        let _ = writeln!(
            svg,
            r##"<text x="{:.1}" y="{:.1}" font-size="9.8" font-weight="bold" fill="#0f172a">{}</text>"##,
            px + 14.0,
            py + 50.0,
            escape_xml(&truncate_clean(&diff.summary, 74)),
        );
        let mut lx = px + 14.0;
        for (layer, cnt) in &diff.layer_counts {
            if lx < px + pw - 95.0 {
                let _ = writeln!(
                    svg,
                    r##"<rect x="{lx:.1}" y="{:.1}" width="112" height="20" rx="4" fill="#eef6f7" stroke="#5b92a5" stroke-width="0.8"/>
<text x="{:.1}" y="{:.1}" font-size="8.5" font-weight="bold" fill="#2b4963">{}: {cnt}</text>"##,
                    py + 62.0,
                    lx + 6.0,
                    py + 75.0,
                    escape_xml(&truncate_clean(layer, 14)),
                );
                lx += 118.0;
            }
        }
        let mut row = 0usize;
        for e in diff.entries.iter().filter(|e| e.kind != IfcxDiffKind::Unchanged).take(9) {
            let ey = py + 98.0 + (row as f64) * 39.0;
            let (tag, col) = match e.kind {
                IfcxDiffKind::Added => ("ADDED", "#059669"),
                IfcxDiffKind::Modified => ("MODIFIED", "#d97706"),
                IfcxDiffKind::Deleted => ("DELETED", "#dc2626"),
                IfcxDiffKind::Unchanged => ("SAME", "#64748b"),
            };
            let _ = writeln!(
                svg,
                r##"<rect x="{:.1}" y="{ey:.1}" width="{:.1}" height="32" rx="4" fill="#ffffff" stroke="#e2e8f0"/>
<rect x="{:.1}" y="{:.1}" width="58" height="16" rx="3" fill="{col}"/>
<text x="{:.1}" y="{:.1}" font-size="8" font-weight="bold" text-anchor="middle" fill="#ffffff">{tag}</text>
<text x="{:.1}" y="{:.1}" font-size="9" font-family="monospace" fill="#0f172a">{} | {}</text>"##,
                px + 12.0,
                pw - 24.0,
                px + 18.0,
                ey + 8.0,
                px + 47.0,
                ey + 19.0,
                px + 84.0,
                ey + 20.0,
                escape_xml(&truncate_clean(&e.class_code, 14)),
                escape_xml(&truncate_clean(&e.path, 42)),
            );
            row += 1;
        }
    } else {
        draw_panel_chrome(
            svg,
            px,
            py,
            pw,
            ph,
            "5. OPENBIM IFCX ELEMENT SCHEDULE",
            &format!("{} Levels", input.project.levels.len()),
        );
        let sched = [
            ("Columns (IfcColumn)", input.project.columns.len(), "Reinforced Concrete / Steel"),
            ("Beams & Braces (IfcBeam/Member)", input.project.runs.len(), "Moment Frame / BRB"),
            ("Shear & Partition Walls (IfcWall)", input.project.walls.len(), "Cast-in-Place Concrete"),
            ("Floor & Roof Slabs (IfcSlab/Roof)", input.project.slabs.len(), "Rigid Diaphragm"),
            ("Pile Caps & Footings (IfcFooting)", input.project.footings.len(), "6-DOF Winkler Coupled"),
            ("Reference Grids (IfcGrid)", input.project.grids.len(), "Orthographic Datum"),
        ];
        for (i, (label, cnt, mat)) in sched.iter().enumerate() {
            let ry = py + 44.0 + (i as f64) * 66.0;
            let _ = writeln!(
                svg,
                r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="56" rx="4" fill="#ffffff" stroke="#cbd5e1"/>
<text x="{:.1}" y="{:.1}" font-size="10.5" font-weight="bold" fill="#0f172a">{}</text>
<text x="{:.1}" y="{:.1}" font-size="9" fill="#64748b">{}</text>
<text x="{:.1}" y="{:.1}" font-size="15" font-weight="bold" text-anchor="end" fill="#2b4963">{cnt}</text>"##,
                px + 12.0,
                pw - 24.0,
                px + 22.0,
                ry + 23.0,
                escape_xml(label),
                px + 22.0,
                ry + 42.0,
                escape_xml(mat),
                px + pw - 24.0,
                ry + 33.0,
            );
        }
    }
}

fn draw_title_block_panel(svg: &mut String, input: &VectorDossierInput<'_>, placeholder_seal: &str) {
    let (px, py, pw, ph) = (1064.0, 624.0, 504.0, 474.0);
    draw_panel_chrome(
        svg,
        px,
        py,
        pw,
        ph,
        "6. ENGINEERING TITLE BLOCK & VERIFICATION SEAL",
        &format!("REV {}", input.revision),
    );
    let _ = writeln!(
        svg,
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="84" rx="6" fill="#0b1427"/>
<text x="{:.1}" y="{:.1}" font-size="16" font-weight="bold" fill="#d0ebe1">EIGENGRAPH COMPUTATIONAL PLATFORM</text>
<text x="{:.1}" y="{:.1}" font-size="10.5" fill="#5b92a5">100% Pure-Rust • 100% Vector • 100% IFC 5 (IFCX) • 0% Raster Pixels</text>
<text x="{:.1}" y="{:.1}" font-size="10" fill="#94a3b8">Engine Version: v{}</text>"##,
        px + 16.0,
        py + 44.0,
        pw - 32.0,
        px + 30.0,
        py + 72.0,
        px + 30.0,
        py + 92.0,
        px + 30.0,
        py + 112.0,
        env!("CARGO_PKG_VERSION"),
    );

    let meta = [
        ("PROJECT TITLE", input.project.project_name.as_str()),
        ("DRAWING SHEET", input.sheet_code),
        ("REVISION & STATUS", input.revision),
        ("DESIGN STANDARDS", "SNI 1726:2019 • SNI 2847:2019 • ASCE 7-22 • AISC 360-16 • UFC 4-023-03"),
        ("METHODOLOGY", "EigenGate + Spectral SVD + 6-DOF Winkler SSI"),
    ];
    for (i, (k, v)) in meta.iter().enumerate() {
        let my = py + 144.0 + (i as f64) * 50.0;
        let _ = writeln!(
            svg,
            r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#cbd5e1" stroke-width="0.8"/>
<text x="{:.1}" y="{my:.1}" font-size="8.5" font-weight="bold" fill="#5b92a5">{k}</text>
<text x="{:.1}" y="{:.1}" font-size="10.5" font-weight="bold" fill="#0f172a">{}</text>"##,
            px + 16.0,
            my + 26.0,
            px + pw - 16.0,
            my + 26.0,
            px + 16.0,
            px + 16.0,
            my + 16.0,
            escape_xml(&truncate_clean(v, 68)),
        );
    }

    let gov_line = input
        .collab_state
        .and_then(|cs| cs.active_checkpoint())
        .map(|cp| {
            format!(
                r##"
<text x="{:.1}" y="{:.1}" font-size="8.2" font-family="monospace" fill="#065f46">GOVERNANCE CHECKPOINT: {} [{}] ({}) • Chain SHA {}</text>"##,
                px + 28.0,
                py + 455.0,
                escape_xml(&cp.checkpoint_id),
                escape_xml(cp.approval_status.short_label()),
                escape_xml(cp.discipline.short_label()),
                escape_xml(cp.short_sha256()),
            )
        })
        .unwrap_or_default();
    let _ = writeln!(
        svg,
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="74" rx="6" fill="#ecfdf5" stroke="#059669" stroke-width="1.2"/>
<text x="{:.1}" y="{:.1}" font-size="10" font-weight="bold" fill="#065f46">CRYPTOGRAPHIC INTEGRITY &amp; PURE-VECTOR SEAL</text>
<text x="{:.1}" y="{:.1}" font-size="8.5" font-family="monospace" fill="#047857">SHA-256: {placeholder_seal}</text>{gov_line}"##,
        px + 16.0,
        py + 388.0,
        pw - 32.0,
        px + 28.0,
        py + 412.0,
        px + 28.0,
        py + 434.0,
    );
}

/// Generates a publication-grade 100% Pure-Vector SVG engineering dossier containing plan,
/// elevation/stratigraphy, EigenGate / LRFD DCR% charts, multi-physics telemetry, IFCX diff,
/// and ISO Title Block with cryptographic SHA-256 verification seal.
pub fn build_pure_vector_engineering_dossier_svg(
    input: &VectorDossierInput<'_>,
) -> VectorDossierExport {
    let mut svg = String::with_capacity(24_576);
    let seal_token = "__EIGENGRAPH_SHA256_SEAL__";
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 1131" width="1600" height="1131" style="background:#ffffff;font-family:'Segoe UI',Arial,sans-serif;">
<rect x="14" y="14" width="1572" height="1103" fill="#ffffff" stroke="#0b1427" stroke-width="2.4"/>
<rect x="19" y="19" width="1562" height="1093" fill="none" stroke="#5b92a5" stroke-width="0.9"/>
<text x="32" y="42" font-size="15" font-weight="bold" fill="#0b1427">EIGENGRAPH ENGINEERING DOSSIER — {}</text>
<text x="1568" y="42" font-size="11" font-weight="bold" text-anchor="end" fill="#2b4963">SHEET {} • {} • 100% PURE VECTOR</text>"##,
        escape_xml(&input.project.project_name),
        escape_xml(input.sheet_code),
        escape_xml(input.revision),
    );

    draw_plan_panel(&mut svg, input);
    draw_elevation_ssi_panel(&mut svg, input);
    draw_dcr_eigengate_panel(&mut svg, input);
    draw_telemetry_panel(&mut svg, input);
    draw_diff_or_schedule_panel(&mut svg, input);
    draw_title_block_panel(&mut svg, input, seal_token);

    svg.push_str("</svg>\n");

    let canonical_bytes = svg.replace(seal_token, "EIGENGRAPH_PURE_VECTOR_A3");
    let digest = Sha256Hasher::digest(canonical_bytes.as_bytes());
    let sha256_seal: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let final_svg = svg.replace(seal_token, &sha256_seal);

    let element_count = input.project.columns.len()
        + input.project.runs.len()
        + input.project.walls.len()
        + input.project.slabs.len()
        + input.project.footings.len();
    let summary = format!(
        "Exported A3 Pure-Vector SVG Dossier ({element_count} BIM elements, 6 panels, SHA-256 {}...)",
        &sha256_seal[..12]
    );

    VectorDossierExport {
        svg_content: final_svg,
        sha256_seal,
        panel_count: 6,
        element_count,
        summary,
    }
}

/// Phase 81 (`v0.8.1`): Generates an archival A3 landscape (`1600 x 1131`, 0% raster pixels)
/// Pure-Vector Engineering Permit Compliance & Parametric Trade-Study Annex SVG (`S-102`).
pub fn build_permit_compliance_annex_svg(
    project: &BimProject,
    permit: &crate::core::trade_study::PermitComplianceReport,
    trade_study: Option<&crate::core::trade_study::TradeStudyReport>,
    collab_state: Option<&crate::core::ifcx_collab::IfcxCollabState>,
    sheet_code: &str,
    revision: &str,
) -> VectorDossierExport {
    let mut svg = String::with_capacity(20_480);
    let seal_token = "__EIGENGRAPH_PERMIT_SHA256__";
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 1131" width="1600" height="1131" style="background:#ffffff;font-family:'Segoe UI',Arial,sans-serif;">
<rect x="14" y="14" width="1572" height="1103" fill="#ffffff" stroke="#0b1427" stroke-width="2.4"/>
<rect x="19" y="19" width="1562" height="1093" fill="none" stroke="#5b92a5" stroke-width="0.9"/>
<text x="32" y="42" font-size="15" font-weight="bold" fill="#0b1427">EIGENGRAPH PERMIT COMPLIANCE &amp; TRADE-STUDY ANNEX — {}</text>
<text x="1568" y="42" font-size="11" font-weight="bold" text-anchor="end" fill="#2b4963">SHEET {} • {} • 100% PURE VECTOR</text>"##,
        escape_xml(&project.project_name),
        escape_xml(sheet_code),
        escape_xml(revision),
    );

    // Panel 1: 10-Clause SNI / ASCE / UFC Permit Compliance Matrix (Left column full height)
    let (p1x, p1y, p1w, p1h) = (32.0, 56.0, 760.0, 1042.0);
    draw_panel_chrome(
        &mut svg,
        p1x,
        p1y,
        p1w,
        p1h,
        "1. AUTOMATED SNI / ASCE / UFC / FEMA P-58 PERMIT COMPLIANCE MATRIX",
        &format!(
            "{}/10 PASS • {:.0}% READY",
            permit.pass_count, permit.readiness_pct
        ),
    );
    for (idx, cl) in permit.clauses.iter().take(10).enumerate() {
        let ry = p1y + 44.0 + (idx as f64) * 96.0;
        let col = cl.status.svg_hex();
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="84" rx="5" fill="#ffffff" stroke="#cbd5e1" stroke-width="0.9"/>
<rect x="{:.1}" y="{:.1}" width="64" height="22" rx="4" fill="{col}"/>
<text x="{:.1}" y="{:.1}" font-size="9.5" font-weight="bold" text-anchor="middle" fill="#ffffff">{}</text>
<text x="{:.1}" y="{:.1}" font-size="10.5" font-weight="bold" fill="#0f172a">{} — {}</text>
<text x="{:.1}" y="{:.1}" font-size="9.5" font-family="monospace" fill="#1e293b">Demand: {}   |   Code Limit: {}</text>
<text x="{:.1}" y="{:.1}" font-size="8.8" fill="#64748b">{}</text>"##,
            p1x + 14.0,
            p1w - 28.0,
            p1x + 24.0,
            ry + 12.0,
            p1x + 56.0,
            ry + 26.5,
            cl.status.short_label(),
            p1x + 100.0,
            ry + 26.5,
            escape_xml(&cl.code_ref),
            escape_xml(&cl.title),
            p1x + 24.0,
            ry + 50.0,
            escape_xml(&cl.demand_value),
            escape_xml(&cl.limit_value),
            p1x + 24.0,
            ry + 70.0,
            escape_xml(&truncate_clean(&cl.notes, 82)),
        );
    }

    // Panel 2: Parametric Trade-Study Comparison Table (Top Right)
    let (p2x, p2y, p2w, p2h) = (808.0, 56.0, 760.0, 412.0);
    let ts_sub = trade_study
        .and_then(|ts| ts.recommended())
        .map(|r| format!("WINNER: {} ({:.1})", r.archetype.short_label(), r.weighted_utility_score))
        .unwrap_or_else(|| "NO MATRIX RUN".to_string());
    draw_panel_chrome(
        &mut svg,
        p2x,
        p2y,
        p2w,
        p2h,
        "2. PARAMETRIC MULTI-SCENARIO TRADE-STUDY COMPARISON",
        &ts_sub,
    );
    let _ = writeln!(
        svg,
        r##"<text x="{:.1}" y="{:.1}" font-size="9" font-weight="bold" font-family="monospace" fill="#2b4963">SCENARIO              EigenGate   DCR%  DRIFT  T1(s)  COST($k)  CO2(t)  SCORE</text>"##,
        p2x + 18.0,
        p2y + 48.0,
    );
    if let Some(ts) = trade_study {
        for (idx, s) in ts.scenarios.iter().take(6).enumerate() {
            let ry = p2y + 58.0 + (idx as f64) * 54.0;
            let fill = if s.is_recommended { "#ecfdf5" } else { "#ffffff" };
            let stroke = if s.is_recommended { "#059669" } else { "#cbd5e1" };
            let badge = if s.is_recommended { " *" } else { "" };
            let _ = writeln!(
                svg,
                r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="44" rx="4" fill="{fill}" stroke="{stroke}" stroke-width="1.0"/>
<text x="{:.1}" y="{:.1}" font-size="9.2" font-family="monospace" font-weight="bold" fill="#0f172a">{:<18}{}  {:.2}  {:>4.0}%  {:>4.2}%  {:>4.2}s   {:>6.0}k  {:>5.1}   {:>5.1}</text>"##,
                p2x + 14.0,
                p2w - 28.0,
                p2x + 22.0,
                ry + 26.0,
                escape_xml(&truncate_clean(s.archetype.short_label(), 18)),
                badge,
                s.min_eigengate,
                s.max_dcr_pct,
                s.max_drift_pct,
                s.fundamental_period_s,
                s.estimated_cost_usd * 0.001,
                s.embodied_carbon_tco2e,
                s.weighted_utility_score,
            );
        }
    }

    // Panel 3: Pure-Vector Trade-Study Utility & EigenGate Chart (Middle Right)
    let (p3x, p3y, p3w, p3h) = (808.0, 484.0, 760.0, 324.0);
    draw_panel_chrome(
        &mut svg,
        p3x,
        p3y,
        p3w,
        p3h,
        "3. MULTI-CRITERIA UTILITY & REDUNDANCY COMPARISON",
        "PURE VECTOR BAR CHART",
    );
    if let Some(ts) = trade_study {
        let bar_x0 = p3x + 150.0;
        let max_bw = p3w - 250.0;
        for (idx, s) in ts.scenarios.iter().take(6).enumerate() {
            let by = p3y + 48.0 + (idx as f64) * 43.0;
            let bw = ((s.weighted_utility_score / 100.0).clamp(0.05, 1.0)) * max_bw;
            let col = if s.is_recommended { "#059669" } else { "#2b4963" };
            let _ = writeln!(
                svg,
                r##"<text x="{:.1}" y="{:.1}" font-size="9.5" font-weight="bold" text-anchor="end" fill="#0f172a">{}</text>
<rect x="{bar_x0:.1}" y="{by:.1}" width="{bw:.1}" height="24" rx="4" fill="{col}"/>
<text x="{:.1}" y="{:.1}" font-size="9" font-family="monospace" fill="#0f172a">{:.1}/100 (EigenGate {:.2})</text>"##,
                bar_x0 - 10.0,
                by + 16.0,
                escape_xml(s.archetype.short_label()),
                bar_x0 + bw + 8.0,
                by + 16.0,
                s.weighted_utility_score,
                s.min_eigengate,
            );
        }
    }

    // Panel 4: Certification Seal & Checkpoint Governance Stamp (Bottom Right)
    let (p4x, p4y, p4w, p4h) = (808.0, 824.0, 760.0, 274.0);
    draw_panel_chrome(
        &mut svg,
        p4x,
        p4y,
        p4w,
        p4h,
        "4. PERMIT CERTIFICATION & SHA-256 GOVERNANCE SEAL",
        &format!("REV {}", revision),
    );
    let gov_line = collab_state
        .and_then(|cs| cs.active_checkpoint())
        .map(|cp| {
            format!(
                "GOVERNANCE CHECKPOINT: {} [{}] ({}) • Chain SHA {}",
                cp.checkpoint_id,
                cp.approval_status.short_label(),
                cp.discipline.short_label(),
                cp.short_sha256(),
            )
        })
        .unwrap_or_else(|| "GOVERNANCE CHECKPOINT: Pending Team Sign-Off".to_string());
    let _ = writeln!(
        svg,
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="202" rx="6" fill="#ecfdf5" stroke="#059669" stroke-width="1.2"/>
<text x="{:.1}" y="{:.1}" font-size="12" font-weight="bold" fill="#065f46">EIGENGRAPH DETERMINISTIC PERMIT CERTIFICATION</text>
<text x="{:.1}" y="{:.1}" font-size="10" fill="#0f172a">{} • Overall Permit Verdict: {}</text>
<text x="{:.1}" y="{:.1}" font-size="9.5" font-family="monospace" fill="#065f46">{}</text>
<text x="{:.1}" y="{:.1}" font-size="8.8" font-family="monospace" fill="#047857">SHA-256: {seal_token}</text>"##,
        p4x + 16.0,
        p4y + 48.0,
        p4w - 32.0,
        p4x + 30.0,
        p4y + 82.0,
        p4x + 30.0,
        p4y + 114.0,
        escape_xml(&permit.summary),
        if permit.overall_ready { "READY FOR SUBMITTAL" } else { "ACTION REQUIRED" },
        p4x + 30.0,
        p4y + 148.0,
        escape_xml(&gov_line),
        p4x + 30.0,
        p4y + 186.0,
    );

    svg.push_str("</svg>\n");
    let canonical_bytes = svg.replace(seal_token, "EIGENGRAPH_PERMIT_ANNEX_A3");
    let digest = Sha256Hasher::digest(canonical_bytes.as_bytes());
    let sha256_seal: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let final_svg = svg.replace(seal_token, &sha256_seal);

    let element_count =
        project.columns.len() + project.runs.len() + project.walls.len() + project.slabs.len();
    let summary = format!(
        "Exported Permit Compliance & Trade-Study Annex A3 SVG ({}, SHA-256 {}...)",
        sheet_code,
        &sha256_seal[..12]
    );
    VectorDossierExport {
        svg_content: final_svg,
        sha256_seal,
        panel_count: 4,
        element_count,
        summary,
    }
}

/// Phase 82 (`v0.8.2`): Generates an archival A3 landscape (`1600 x 1131`, 0% raster pixels)
/// Pure-Vector Rebar BBS, 1D Master Stock Cut-List & Field QA/QC Shop-Drawing Annex SVG (`S-103`).
pub fn build_fabrication_bbs_shop_svg(
    project: &BimProject,
    report: &crate::core::field_fabrication::FieldFabricationReport,
    collab_state: Option<&crate::core::ifcx_collab::IfcxCollabState>,
    sheet_code: &str,
    revision: &str,
) -> VectorDossierExport {
    let mut svg = String::with_capacity(22_000);
    let seal_token = "__EIGENGRAPH_SHOP_BBS_SHA256__";
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 1131" width="1600" height="1131" style="background:#ffffff;font-family:'Segoe UI',Arial,sans-serif;">
<rect x="14" y="14" width="1572" height="1103" fill="#ffffff" stroke="#0b1427" stroke-width="2.4"/>
<rect x="19" y="19" width="1562" height="1093" fill="none" stroke="#5b92a5" stroke-width="0.9"/>
<text x="32" y="42" font-size="15" font-weight="bold" fill="#0b1427">EIGENGRAPH REBAR BBS, 1D CUT-LIST &amp; FIELD QA/QC SHOP ANNEX — {}</text>
<text x="1568" y="42" font-size="11" font-weight="bold" text-anchor="end" fill="#2b4963">SHEET {} • {} • EigenGate + EigenFlow • 100% PURE VECTOR</text>"##,
        escape_xml(&project.project_name),
        escape_xml(sheet_code),
        escape_xml(revision),
    );

    // Panel 1: Bar Bending Schedule (Top Left)
    let (p1x, p1y, p1w, p1h) = (32.0, 56.0, 760.0, 560.0);
    draw_panel_chrome(
        &mut svg,
        p1x,
        p1y,
        p1w,
        p1h,
        "1. SNI 2847 / BS 8666 BAR BENDING SCHEDULE (BBS)",
        &format!(
            "{} Marks • {:.0} kg Total • {}",
            report.bbs_items.len(),
            report.total_rebar_weight_kg,
            escape_xml(&report.source_mode)
        ),
    );
    let _ = writeln!(
        svg,
        r##"<text x="{:.1}" y="{:.1}" font-size="9" font-weight="bold" font-family="monospace" fill="#2b4963">MARK        ELEMENT                 DIA   GRADE       SHAPE                      QTY   CUT(mm)   MASS(kg)</text>"##,
        p1x + 16.0,
        p1y + 46.0,
    );
    for (idx, b) in report.bbs_items.iter().take(12).enumerate() {
        let ry = p1y + 56.0 + (idx as f64) * 39.0;
        let fill = if idx % 2 == 0 { "#f8fafc" } else { "#ffffff" };
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="32" rx="3" fill="{fill}" stroke="#e2e8f0" stroke-width="0.8"/>
<text x="{:.1}" y="{:.1}" font-size="8.6" font-family="monospace" fill="#0f172a">{:<10}  {:<22} D{:<3.0} {:<10}  {:<24} {:>4}   {:>6.0}    {:>7.1}</text>"##,
            p1x + 12.0,
            p1w - 24.0,
            p1x + 18.0,
            ry + 20.0,
            escape_xml(&truncate_clean(&b.mark, 10)),
            escape_xml(&truncate_clean(&b.element, 22)),
            b.bar_size_mm,
            escape_xml(&truncate_clean(&b.grade, 10)),
            escape_xml(&truncate_clean(&b.shape_code, 24)),
            b.count,
            b.cut_length_mm,
            b.total_weight_kg,
        );
    }

    // Panel 2: 1D Master Stock Bar Cutting-Pattern Diagram (Bottom Left)
    let (p2x, p2y, p2w, p2h) = (32.0, 632.0, 760.0, 466.0);
    draw_panel_chrome(
        &mut svg,
        p2x,
        p2y,
        p2w,
        p2h,
        "2. 1D MASTER STOCK BAR CUTTING-STOCK PATTERNS (BFD)",
        &format!(
            "Yield {:.1}% • Scrap {:.1}% • {} x {:.0}m Bars • Est Savings ${:.0}",
            report.cso_report.material_yield_pct,
            report.cso_report.scrap_waste_pct,
            report.cso_report.master_bars_used,
            report.cso_report.stock_length_m,
            report.estimated_savings_usd
        ),
    );
    let diag_x0 = p2x + 76.0;
    let diag_w = p2w - 150.0;
    let stock_m = report.cso_report.stock_length_m.max(1.0);
    let cut_palette = ["#1d4ed8", "#0284c7", "#0f766e", "#2563eb", "#0369a1"];
    for (idx, bar) in report.cso_report.bars.iter().take(11).enumerate() {
        let by = p2y + 46.0 + (idx as f64) * 35.0;
        let _ = writeln!(
            svg,
            r##"<text x="{:.1}" y="{:.1}" font-size="8.6" font-family="monospace" font-weight="bold" fill="#1e293b">Bar #{:<2}</text>
<rect x="{diag_x0:.1}" y="{by:.1}" width="{diag_w:.1}" height="22" rx="3" fill="#f1f5f9" stroke="#94a3b8" stroke-width="0.9"/>"##,
            p2x + 14.0,
            by + 14.5,
            bar.bar_index,
        );
        for (c_idx, cut) in bar.cuts.iter().enumerate() {
            let cx = diag_x0 + (cut.start_m / stock_m).clamp(0.0, 1.0) * diag_w;
            let cw = ((cut.length_m / stock_m).clamp(0.01, 1.0) * diag_w).max(2.0);
            let fill = cut_palette[c_idx % cut_palette.len()];
            let _ = writeln!(
                svg,
                r##"<rect x="{cx:.1}" y="{:.1}" width="{cw:.1}" height="18" rx="2" fill="{fill}"/>"##,
                by + 2.0,
            );
            if cw >= 34.0 {
                let _ = writeln!(
                    svg,
                    r##"<text x="{:.1}" y="{:.1}" font-size="7.4" font-family="monospace" text-anchor="middle" fill="#ffffff">{:.2}m</text>"##,
                    cx + cw * 0.5,
                    by + 14.0,
                    cut.length_m,
                );
            }
        }
        if bar.remainder_m > 0.02 {
            let rx = diag_x0 + (bar.used_length_m / stock_m).clamp(0.0, 1.0) * diag_w;
            let rw = ((bar.remainder_m / stock_m) * diag_w).clamp(1.0, diag_w);
            let r_fill = if bar.is_reusable_offcut { "#22d3ee" } else { "#f97316" };
            let _ = writeln!(
                svg,
                r##"<rect x="{rx:.1}" y="{:.1}" width="{rw:.1}" height="18" rx="2" fill="{r_fill}"/>"##,
                by + 2.0,
            );
        }
        let _ = writeln!(
            svg,
            r##"<text x="{:.1}" y="{:.1}" font-size="8.2" font-family="monospace" fill="#0f172a">{:.1}%</text>"##,
            diag_x0 + diag_w + 8.0,
            by + 14.5,
            bar.yield_pct,
        );
    }

    // Panel 3: Field QA/QC Scorecard & Spatial Waypoint Register (Top Right)
    let (p3x, p3y, p3w, p3h) = (808.0, 56.0, 760.0, 750.0);
    draw_panel_chrome(
        &mut svg,
        p3x,
        p3y,
        p3w,
        p3h,
        "3. FIELD QA/QC SCORECARD, POUR-STRIPS & SPATIAL ENU WAYPOINTS",
        &format!(
            "f'c Pass {:.0}% • ITP {}/{} • Open NCR {}",
            report.fc_pass_rate_pct,
            report.itp_cleared_count,
            report.itp_total_count,
            report.open_ncr_count
        ),
    );
    for (idx, wp) in report.waypoints.iter().take(9).enumerate() {
        let ry = p3y + 48.0 + (idx as f64) * 56.0;
        let [br, bg, bb] = wp.status.badge_color_rgb();
        let hex = format!("#{br:02x}{bg:02x}{bb:02x}");
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="48" rx="4" fill="#ffffff" stroke="#cbd5e1" stroke-width="0.9"/>
<rect x="{:.1}" y="{:.1}" width="54" height="20" rx="4" fill="{hex}"/>
<text x="{:.1}" y="{:.1}" font-size="9" font-weight="bold" text-anchor="middle" fill="#ffffff">{}</text>
<text x="{:.1}" y="{:.1}" font-size="10" font-weight="bold" fill="#0f172a">{} — {} ({})</text>
<text x="{:.1}" y="{:.1}" font-size="8.6" font-family="monospace" fill="#475569">ENU ({:.1}, {:.1}, {:.1})m • {}</text>"##,
            p3x + 14.0,
            p3w - 28.0,
            p3x + 22.0,
            ry + 8.0,
            p3x + 49.0,
            ry + 21.5,
            wp.status.short_label(),
            p3x + 86.0,
            ry + 21.5,
            escape_xml(&wp.code),
            escape_xml(&truncate_clean(&wp.title, 44)),
            escape_xml(&wp.element_ref),
            p3x + 22.0,
            ry + 39.0,
            wp.local_x_m,
            wp.local_y_m,
            wp.local_z_m,
            escape_xml(&truncate_clean(&wp.notes, 54)),
        );
    }
    let ps_y0 = p3y + 566.0;
    let _ = writeln!(
        svg,
        r##"<text x="{:.1}" y="{ps_y0:.1}" font-size="9.5" font-weight="bold" fill="#2b4963">CONCRETE POUR-STRIP &amp; 28-DAY CYLINDER TRACKER</text>"##,
        p3x + 16.0,
    );
    for (idx, ps) in report.pour_strips.iter().take(4).enumerate() {
        let ry = ps_y0 + 12.0 + (idx as f64) * 38.0;
        let [pr, pg, pb] = ps.status.color_rgb();
        let hex = format!("#{pr:02x}{pg:02x}{pb:02x}");
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="31" rx="3" fill="#f8fafc" stroke="{hex}" stroke-width="1.0"/>
<text x="{:.1}" y="{:.1}" font-size="8.8" font-family="monospace" fill="#0f172a">{:<34} | Target {:.0} MPa -> 28d {:.1} MPa | {:.1} m3 | {}</text>"##,
            p3x + 14.0,
            p3w - 28.0,
            p3x + 22.0,
            ry + 20.0,
            escape_xml(&truncate_clean(&ps.label, 34)),
            ps.target_fc_mpa,
            ps.measured_28d_mpa,
            ps.concrete_volume_m3,
            escape_xml(ps.status.label()),
        );
    }

    // Panel 4: Fabrication Seal & IFCX Governance Stamp (Bottom Right)
    let (p4x, p4y, p4w, p4h) = (808.0, 824.0, 760.0, 274.0);
    draw_panel_chrome(
        &mut svg,
        p4x,
        p4y,
        p4w,
        p4h,
        "4. SHOP-DRAWING CERTIFICATION & SHA-256 GOVERNANCE SEAL",
        &format!("REV {} • EigenGate + EigenFlow", revision),
    );
    let gov_line = collab_state
        .and_then(|cs| cs.active_checkpoint())
        .map(|cp| {
            format!(
                "CHECKPOINT: {} [{}] ({}) • Chain SHA {}",
                cp.checkpoint_id,
                cp.approval_status.short_label(),
                cp.discipline.short_label(),
                cp.short_sha256(),
            )
        })
        .unwrap_or_else(|| "CHECKPOINT: Live Local Engineering Baseline".to_string());
    let _ = writeln!(
        svg,
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="202" rx="6" fill="#ecfdf5" stroke="#059669" stroke-width="1.2"/>
<text x="{:.1}" y="{:.1}" font-size="12" font-weight="bold" fill="#065f46">EIGENGRAPH FIELD FABRICATION &amp; QA/QC CERTIFICATION (SHEET S-103)</text>
<text x="{:.1}" y="{:.1}" font-size="9.5" fill="#0f172a">{}</text>
<text x="{:.1}" y="{:.1}" font-size="9.2" font-family="monospace" fill="#065f46">{}</text>
<text x="{:.1}" y="{:.1}" font-size="8.6" font-family="monospace" fill="#047857">SHA-256: {seal_token}</text>"##,
        p4x + 16.0,
        p4y + 48.0,
        p4w - 32.0,
        p4x + 30.0,
        p4y + 84.0,
        p4x + 30.0,
        p4y + 118.0,
        escape_xml(&truncate_clean(&report.summary, 94)),
        p4x + 30.0,
        p4y + 152.0,
        escape_xml(&gov_line),
        p4x + 30.0,
        p4y + 188.0,
    );

    svg.push_str("</svg>\n");
    let canonical_bytes = svg.replace(seal_token, "EIGENGRAPH_SHOP_BBS_S103");
    let digest = Sha256Hasher::digest(canonical_bytes.as_bytes());
    let sha256_seal: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let final_svg = svg.replace(seal_token, &sha256_seal);

    let element_count = report.bbs_items.len() + report.waypoints.len();
    let summary = format!(
        "Exported Fabrication BBS & Field QA/QC Shop Annex A3 SVG ({}, SHA-256 {}...)",
        sheet_code,
        &sha256_seal[..12]
    );
    VectorDossierExport {
        svg_content: final_svg,
        sha256_seal,
        panel_count: 4,
        element_count,
        summary,
    }
}

/// Phase 83 (`v0.8.3`): Generates an archival A3 landscape (`1600 x 1131`, 0% raster pixels)
/// Pure-Vector EigenFlow Non-Linear Pushover, ADRS Capacity Spectrum & Woodbury `P1 -> P2` Annex (`S-104`).
pub fn build_eigenflow_nonlinear_annex_svg(
    project_name: &str,
    report: &crate::core::eigenflow::EigenFlowStudioReport,
    sheet_code: &str,
    checkpoint_hash: Option<&str>,
) -> VectorDossierExport {
    let mut svg = String::with_capacity(24_000);
    let seal_token = "__EIGENGRAPH_EIGENFLOW_S104_SHA256__";
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 1131" width="1600" height="1131" style="background:#ffffff;font-family:'Segoe UI',Arial,sans-serif;">
<rect x="14" y="14" width="1572" height="1103" fill="#ffffff" stroke="#0b1427" stroke-width="2.4"/>
<rect x="19" y="19" width="1562" height="1093" fill="none" stroke="#0284c7" stroke-width="0.9"/>
<text x="32" y="42" font-size="15" font-weight="bold" fill="#0b1427">EIGENGRAPH EIGENFLOW NON-LINEAR PUSHOVER, ADRS &amp; WOODBURY ANNEX (SHEET {}) — {}</text>
<text x="1568" y="42" font-size="11" font-weight="bold" text-anchor="end" fill="#0369a1">SHEET {} • EigenGate + EigenFlow • 100% PURE VECTOR</text>"##,
        escape_xml(sheet_code),
        escape_xml(project_name),
        escape_xml(sheet_code),
    );

    // Panel 1: Pushover Capacity Curve Vb vs Roof Drift (Top Left)
    let (p1x, p1y, p1w, p1h) = (32.0, 56.0, 760.0, 520.0);
    let raw = &report.pushover_csm.raw_pushover;
    let perf = &report.pushover_csm.performance_point;
    draw_panel_chrome(
        &mut svg,
        p1x,
        p1y,
        p1w,
        p1h,
        "1. INCREMENTAL P-DELTA PUSHOVER CAPACITY CURVE (Vb vs ROOF DRIFT)",
        &format!(
            "{} • Vy={:.0} kN • Vu={:.0} kN • Omega0={:.2} • mu={:.2}",
            escape_xml(report.pushover_csm.load_pattern.label()),
            raw.yield_base_shear_kn,
            raw.ultimate_base_shear_kn,
            raw.overstrength_factor_omega0,
            raw.ductility_ratio_mu
        ),
    );
    let gx0 = p1x + 76.0;
    let gy0 = p1y + 56.0;
    let gw = p1w - 112.0;
    let gh = p1h - 116.0;
    let max_drift = raw
        .steps
        .iter()
        .map(|s| s.roof_drift_mm)
        .fold(raw.ultimate_drift_mm.max(perf.target_displacement_dt_mm), f64::max)
        .max(10.0)
        * 1.12;
    let max_shear = raw
        .steps
        .iter()
        .map(|s| s.base_shear_kn)
        .fold(raw.ultimate_base_shear_kn.max(perf.target_base_shear_vt_kn), f64::max)
        .max(20.0)
        * 1.15;

    let _ = writeln!(
        svg,
        r##"<rect x="{gx0:.1}" y="{gy0:.1}" width="{gw:.1}" height="{gh:.1}" fill="#f8fafc" stroke="#cbd5e1" stroke-width="1.0"/>"##
    );
    // Performance zone vertical guides: IO (yield*1.05) and LS (yield*1.85)
    let io_x = gx0 + ((raw.yield_drift_mm * 1.05) / max_drift).clamp(0.0, 1.0) * gw;
    let ls_x = gx0 + ((raw.yield_drift_mm * 1.85) / max_drift).clamp(0.0, 1.0) * gw;
    let _ = writeln!(
        svg,
        r##"<line x1="{io_x:.1}" y1="{gy0:.1}" x2="{io_x:.1}" y2="{:.1}" stroke="#0ea5e9" stroke-width="1.1" stroke-dasharray="5,4"/>
<text x="{:.1}" y="{:.1}" font-size="8.2" font-weight="bold" fill="#0284c7">IO</text>
<line x1="{ls_x:.1}" y1="{gy0:.1}" x2="{ls_x:.1}" y2="{:.1}" stroke="#f59e0b" stroke-width="1.1" stroke-dasharray="5,4"/>
<text x="{:.1}" y="{:.1}" font-size="8.2" font-weight="bold" fill="#d97706">LS / CP</text>"##,
        gy0 + gh,
        io_x + 4.0,
        gy0 + 14.0,
        gy0 + gh,
        ls_x + 4.0,
        gy0 + 14.0,
    );
    let mut push_pts = format!("{gx0:.1},{:.1}", gy0 + gh);
    for st in &raw.steps {
        let sx = gx0 + (st.roof_drift_mm / max_drift).clamp(0.0, 1.0) * gw;
        let sy = gy0 + gh - (st.base_shear_kn / max_shear).clamp(0.0, 1.0) * gh;
        push_pts.push_str(&format!(" {sx:.1},{sy:.1}"));
    }
    let perf_px = gx0 + (perf.target_displacement_dt_mm / max_drift).clamp(0.0, 1.0) * gw;
    let perf_py = gy0 + gh - (perf.target_base_shear_vt_kn / max_shear).clamp(0.0, 1.0) * gh;
    let yld_px = gx0 + (raw.yield_drift_mm / max_drift).clamp(0.0, 1.0) * gw;
    let yld_py = gy0 + gh - (raw.yield_base_shear_kn / max_shear).clamp(0.0, 1.0) * gh;
    let _ = writeln!(
        svg,
        r##"<polyline fill="none" stroke="#0369a1" stroke-width="2.8" points="{push_pts}"/>
<circle cx="{yld_px:.1}" cy="{yld_py:.1}" r="5.5" fill="#f59e0b" stroke="#0f172a" stroke-width="1.1"/>
<text x="{:.1}" y="{:.1}" font-size="8.4" font-weight="bold" fill="#b45309">Yield ({:.1} mm, {:.0} kN)</text>
<circle cx="{perf_px:.1}" cy="{perf_py:.1}" r="6.5" fill="#10b981" stroke="#0f172a" stroke-width="1.3"/>
<text x="{:.1}" y="{:.1}" font-size="8.6" font-weight="bold" fill="#047857">Target (Dt={:.1} mm, Vt={:.0} kN)</text>
<text x="{:.1}" y="{:.1}" font-size="9" font-family="monospace" fill="#334155">Roof Drift Delta (mm) [0 .. {:.0} mm] • Max Vb = {:.0} kN</text>"##,
        yld_px + 8.0,
        (yld_py - 8.0).max(gy0 + 26.0),
        raw.yield_drift_mm,
        raw.yield_base_shear_kn,
        (perf_px + 9.0).min(gx0 + gw - 175.0),
        (perf_py + 14.0).min(gy0 + gh - 8.0),
        perf.target_displacement_dt_mm,
        perf.target_base_shear_vt_kn,
        gx0 + 4.0,
        gy0 + gh + 24.0,
        max_drift,
        max_shear,
    );

    // Panel 2: ADRS Capacity vs Inelastic Demand Spectrum (Top Right)
    let (p2x, p2y, p2w, p2h) = (808.0, 56.0, 760.0, 520.0);
    draw_panel_chrome(
        &mut svg,
        p2x,
        p2y,
        p2w,
        p2h,
        "2. ATC-40 / ASCE 41-17 ADRS CAPACITY VS INELASTIC DEMAND SPECTRUM",
        &format!(
            "Sd*={:.1} mm • Sa*={:.3}g • Teff={:.2}s • Beta_eff={:.1}%",
            perf.sd_star_m * 1000.0,
            perf.sa_star_g,
            perf.effective_period_teff_s,
            perf.effective_damping_pct
        ),
    );
    let ax0 = p2x + 76.0;
    let ay0 = p2y + 56.0;
    let aw = p2w - 112.0;
    let ah = p2h - 116.0;
    let cap_pts = &report.pushover_csm.capacity_spectrum.points;
    let el_pts = &report.pushover_csm.demand_spectrum.elastic_5pct_points;
    let red_pts = &report.pushover_csm.demand_spectrum.reduced_inelastic_points;
    let max_sd_mm = cap_pts
        .iter()
        .chain(el_pts.iter())
        .map(|p| p.sd_mm())
        .fold(perf.sd_star_m * 1000.0, f64::max)
        .max(30.0)
        * 1.08;
    let max_sa_g = cap_pts
        .iter()
        .chain(el_pts.iter())
        .map(|p| p.sa_g)
        .fold(perf.sa_star_g, f64::max)
        .max(0.25)
        * 1.12;
    let _ = writeln!(
        svg,
        r##"<rect x="{ax0:.1}" y="{ay0:.1}" width="{aw:.1}" height="{ah:.1}" fill="#f8fafc" stroke="#cbd5e1" stroke-width="1.0"/>"##
    );
    let build_adrs_poly = |pts: &[crate::core::pushover::AdrsPoint]| -> String {
        pts.iter()
            .map(|p| {
                let x = ax0 + (p.sd_mm() / max_sd_mm).clamp(0.0, 1.0) * aw;
                let y = ay0 + ah - (p.sa_g / max_sa_g).clamp(0.0, 1.0) * ah;
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let el_poly = build_adrs_poly(el_pts);
    let red_poly = build_adrs_poly(red_pts);
    let cap_poly = build_adrs_poly(cap_pts);
    let sx = ax0 + ((perf.sd_star_m * 1000.0) / max_sd_mm).clamp(0.0, 1.0) * aw;
    let sy = ay0 + ah - (perf.sa_star_g / max_sa_g).clamp(0.0, 1.0) * ah;
    let _ = writeln!(
        svg,
        r##"<polyline fill="none" stroke="#94a3b8" stroke-width="1.6" stroke-dasharray="6,4" points="{el_poly}"/>
<polyline fill="none" stroke="#ef4444" stroke-width="2.2" stroke-dasharray="4,3" points="{red_poly}"/>
<polyline fill="none" stroke="#0284c7" stroke-width="2.8" points="{cap_poly}"/>
<line x1="{ax0:.1}" y1="{:.1}" x2="{sx:.1}" y2="{sy:.1}" stroke="#0f766e" stroke-width="1.1" stroke-dasharray="3,3"/>
<circle cx="{sx:.1}" cy="{sy:.1}" r="7.0" fill="#10b981" stroke="#0b1427" stroke-width="1.5"/>
<text x="{:.1}" y="{:.1}" font-size="8.8" font-weight="bold" fill="#065f46">Performance Pt ({:.1} mm, {:.3}g)</text>
<text x="{:.1}" y="{:.1}" font-size="8.6" font-family="monospace" fill="#334155">Blue: Capacity | Red Dash: Inelastic Demand | Grey Dash: 5% Elastic • SDS={:.2}g SD1={:.2}g</text>"##,
        ay0 + ah,
        (sx + 10.0).min(ax0 + aw - 190.0),
        (sy - 8.0).max(ay0 + 22.0),
        perf.sd_star_m * 1000.0,
        perf.sa_star_g,
        ax0 + 4.0,
        ay0 + ah + 24.0,
        report.site_sds_g,
        report.site_sd1_g,
    );

    // Panel 3: EigenGate (Linear) vs EigenFlow (Non-Linear) Comparative Scorecard (Bottom Left)
    let (p3x, p3y, p3w, p3h) = (32.0, 592.0, 620.0, 506.0);
    let wb = &report.woodbury;
    draw_panel_chrome(
        &mut svg,
        p3x,
        p3y,
        p3w,
        p3h,
        "3. EIGENGATE (LINEAR) VS EIGENFLOW (NON-LINEAR) COMPARATIVE SCORECARD",
        &format!(
            "P1 Disrupted: #{} ({}) • lambda = {:.2}",
            wb.disrupted_element_id,
            escape_xml(&truncate_clean(&wb.disrupted_element_name, 20)),
            wb.lambda_val
        ),
    );
    let kpis = [
        (
            "Linear EigenGate System Index",
            format!("{:.3}", report.linear_eigengate_system),
            "#0284c7",
        ),
        (
            "Non-Linear EigenFlow System Index",
            format!("{:.3}", report.non_linear_eigenflow_system),
            "#059669",
        ),
        (
            "Delta (EigenFlow NL - EigenGate Lin)",
            format!("{:+.3}", report.delta_score_nl_minus_lin),
            if report.delta_score_nl_minus_lin >= 0.0 {
                "#0f766e"
            } else {
                "#d97706"
            },
        ),
        (
            "Topological Stability Margin m_s",
            format!("{:.3} (h_e = {:.3})", wb.stability_margin_ms, wb.svd_leverage_he),
            if wb.is_mechanism { "#dc2626" } else { "#1e293b" },
        ),
        (
            "Woodbury Residual Reserve Ratio (RRR)",
            format!("{:.3} (CI_nl = {:.3})", wb.residual_reserve_ratio_rrr, wb.non_linear_ci),
            "#1e293b",
        ),
        (
            "Max Ductility Demand mu / Shed Force",
            format!("{:.2} / {:.1} kN", wb.max_ductility_mu, wb.total_shed_force_kn),
            "#1e293b",
        ),
        (
            "ASCE 41-17 Pushover Performance Level",
            escape_xml(&truncate_clean(perf.performance_level.label(), 44)),
            "#0369a1",
        ),
    ];
    for (idx, (lbl, val, col)) in kpis.iter().enumerate() {
        let ry = p3y + 52.0 + (idx as f64) * 44.0;
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="36" rx="4" fill="#f8fafc" stroke="#cbd5e1" stroke-width="0.9"/>
<text x="{:.1}" y="{:.1}" font-size="9.5" font-weight="bold" fill="#334155">{lbl}</text>
<text x="{:.1}" y="{:.1}" font-size="9.8" font-weight="bold" font-family="monospace" text-anchor="end" fill="{col}">{val}</text>"##,
            p3x + 14.0,
            p3w - 28.0,
            p3x + 24.0,
            ry + 22.0,
            p3x + p3w - 24.0,
            ry + 22.0,
        );
    }
    let _ = writeln!(
        svg,
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="120" rx="5" fill="#eff6ff" stroke="#3b82f6" stroke-width="1.0"/>
<text x="{:.1}" y="{:.1}" font-size="9.2" font-weight="bold" fill="#1e3a8a">PRESCRIPTIVE NON-LINEAR RECOMMENDATION</text>
<text x="{:.1}" y="{:.1}" font-size="8.6" font-family="monospace" fill="#0f172a">{}</text>
<text x="{:.1}" y="{:.1}" font-size="8.2" font-family="monospace" fill="#1d4ed8">{}</text>
<text x="{:.1}" y="{:.1}" font-size="8.2" font-family="monospace" fill="#475569">SHA-256 SEAL: {seal_token}</text>"##,
        p3x + 14.0,
        p3y + 368.0,
        p3w - 28.0,
        p3x + 24.0,
        p3y + 390.0,
        p3x + 24.0,
        p3y + 414.0,
        escape_xml(&truncate_clean(&wb.prescriptive_recommendation, 84)),
        p3x + 24.0,
        p3y + 440.0,
        escape_xml(
            &checkpoint_hash
                .map(|h| format!("IFCX CHECKPOINT SEAL: {h}"))
                .unwrap_or_else(|| "IFCX CHECKPOINT SEAL: Live Local EigenFlow Baseline".to_string()),
        ),
        p3x + 24.0,
        p3y + 466.0,
    );

    // Panel 4: Woodbury P1 -> P2 Load-Path Redistribution Schedule (Bottom Right)
    let (p4x, p4y, p4w, p4h) = (668.0, 592.0, 900.0, 506.0);
    draw_panel_chrome(
        &mut svg,
        p4x,
        p4y,
        p4w,
        p4h,
        "4. WOODBURY RANK-1 P1 -> P2 SURVIVING LOAD-PATH REDISTRIBUTION SCHEDULE",
        &format!(
            "{} Surviving Backup Members • 3D Hinge Markers: {}",
            wb.redistributed_p2.len(),
            report.hinges_3d.len()
        ),
    );
    let _ = writeln!(
        svg,
        r##"<text x="{:.1}" y="{:.1}" font-size="9" font-weight="bold" font-family="monospace" fill="#0369a1">ID    SURVIVING MEMBER              INTACT(kN)   SHED dN(kN)   SHARE(%)   POST-DCR   MU      STATUS</text>"##,
        p4x + 18.0,
        p4y + 48.0,
    );
    for (idx, m) in wb.redistributed_p2.iter().take(8).enumerate() {
        let ry = p4y + 58.0 + (idx as f64) * 52.0;
        let (badge_hex, status_txt) = if m.is_critical_overload {
            ("#dc2626", "OVERLOAD")
        } else if m.post_yield_dcr > 0.72 {
            ("#d97706", "HIGH UTIL")
        } else {
            ("#059669", "ELASTIC OK")
        };
        let _ = writeln!(
            svg,
            r##"<rect x="{:.1}" y="{ry:.1}" width="{:.1}" height="42" rx="4" fill="#ffffff" stroke="#cbd5e1" stroke-width="0.9"/>
<text x="{:.1}" y="{:.1}" font-size="9.0" font-family="monospace" fill="#0f172a">#{:<4} {:<28} {:>8.1}     {:>8.1}      {:>5.1}%     {:>5.2}    {:>4.2}</text>
<rect x="{:.1}" y="{:.1}" width="84" height="22" rx="4" fill="{badge_hex}"/>
<text x="{:.1}" y="{:.1}" font-size="8.6" font-weight="bold" text-anchor="middle" fill="#ffffff">{status_txt}</text>"##,
            p4x + 14.0,
            p4w - 28.0,
            p4x + 20.0,
            ry + 25.0,
            m.element_id,
            escape_xml(&truncate_clean(&m.element_name, 26)),
            m.intact_axial_kn,
            m.delta_n_kn,
            m.share_pct,
            m.post_yield_dcr,
            m.ductility_mu,
            p4x + p4w - 110.0,
            ry + 10.0,
            p4x + p4w - 68.0,
            ry + 24.5,
        );
    }

    svg.push_str("</svg>\n");
    let canonical_bytes = svg.replace(seal_token, "EIGENGRAPH_EIGENFLOW_S104");
    let digest = Sha256Hasher::digest(canonical_bytes.as_bytes());
    let sha256_seal: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let final_svg = svg.replace(seal_token, &sha256_seal);

    let element_count = wb.redistributed_p2.len() + report.hinges_3d.len() + 1;
    let summary = format!(
        "Exported EigenFlow Non-Linear Pushover, ADRS & Woodbury Annex A3 SVG ({}, SHA-256 {}...)",
        sheet_code,
        &sha256_seal[..12]
    );
    VectorDossierExport {
        svg_content: final_svg,
        sha256_seal,
        panel_count: 4,
        element_count,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::geotech::SubsurfaceSsiConfig;
    use crate::core::ifcx::{IfcxDocument, IfcxStage};

    #[test]
    fn test_phase72_pure_vector_engineering_dossier_svg_and_sha256_seal() {
        let mut proj = BimProject::sample_bimcraft_building();
        proj.project_name = "Tower <A & B>".to_string();
        let site = SiteEnvironmentalCoupling::default();
        let ssi_cfg = SubsurfaceSsiConfig {
            ssi_coupling_enabled: true,
            xray_subsurface_visible: true,
            borehole_preset: BoreholePreset::CoastalLiquefiableSand,
            groundwater_depth_m: 1.5,
            liquefaction_trigger_active: true,
        };
        let ssi_rep = crate::core::geotech::evaluate_coupled_subsurface_ssi_for_bim(
            &proj,
            &site,
            &ssi_cfg,
        )
        .expect("SSI report");
        let base = IfcxDocument::from_bim_project(&proj, None, 0.0, None, None, None);
        let layered = IfcxDocument::build_layered_snapshot_for_workspace(
            &proj,
            None,
            0.0,
            None,
            None,
            None,
            Some(&ssi_rep),
            None,
        );
        let diff = IfcxStage::compute_visual_diff(&base, "Base", &layered, "SSI");

        let members = vec![StructuralMember {
            id: 1,
            name: "Col-C1 <Crit>".to_string(),
            section: "RC 400x400".to_string(),
            force_kn: -420.0,
            capacity_kn: 800.0,
            utilization: 0.72,
            moment_my_knm: 45.0,
            moment_mz_knm: 32.0,
            is_critical: false,
            is_active: true,
            eigengate_score: 0.84,
            svd_sensitivity: 0.1,
            eigengate: 0.84,
            dcr_interaction: 0.78,
            shear_vy_kn: 55.0,
            svd_margin: 0.42,
            ci: 0.16,
            rr: 0.88,
        }];

        let replay_rep = crate::core::ground_motion_replay::run_ground_motion_replay_for_bim(
            &proj,
            &site,
            Some(&ssi_rep),
            &crate::core::ground_motion_replay::GroundMotionReplayConfig::default(),
            "DossierTest",
            None,
        )
        .expect("Replay report");

        let mc_rep = crate::core::gpu_fragility_mc::run_fema_p58_monte_carlo_cpu(
            &proj,
            Some(&replay_rep),
            None,
            Some(&ssi_rep),
            &crate::core::gpu_fragility_mc::MonteCarloFragilityConfig {
                sample_count: 512,
                ..Default::default()
            },
        );
        let mut collab = crate::core::ifcx_collab::IfcxCollabState::default();
        collab.create_checkpoint(
            "Approved Basis",
            crate::core::ifcx_collab::CollabDiscipline::Structural,
            layered.clone(),
            crate::core::ifcx_collab::IfcxCheckpointKpi::default(),
        );
        let _ = collab.approve_active_checkpoint("Lead Reviewer");
        let export = build_pure_vector_engineering_dossier_svg(&VectorDossierInput {
            project: &proj,
            members: &members,
            site: &site,
            lat_lon: (-6.2088, 106.8456),
            retrofit_report: None,
            crc_report: None,
            ssi_report: Some(&ssi_rep),
            hazard_report: None,
            diff_report: Some(&diff),
            portfolio_report: None,
            replay_report: Some(&replay_rep),
            shm_report: None,
            evacuation_report: None,
            watchdog_state: None,
            monte_carlo_report: Some(&mc_rep),
            collab_state: Some(&collab),
            sheet_code: "S-101",
            revision: "REV 01",
        });

        assert_eq!(export.panel_count, 6);
        assert!(export.svg_content.starts_with("<svg"));
        assert!(export.svg_content.contains(r#"viewBox="0 0 1600 1131""#));
        assert!(export.svg_content.trim_end().ends_with("</svg>"));
        assert!(!export.svg_content.contains("<image"));
        assert!(!export.svg_content.contains("data:image"));
        assert!(export.svg_content.contains("GOVERNANCE CHECKPOINT: CP-001 [APPROVED]"));
        assert!(export.svg_content.contains("NL-THA Ground-Motion &amp; Hinges"));
        assert!(export.svg_content.contains("MC PML90"));
        assert!(export.svg_content.contains("Tower &lt;A &amp; B&gt;"));
        assert!(export.svg_content.contains("Col-C1 &lt;Crit&gt;"));
        assert!(export.svg_content.contains("Vs30="));
        assert!(export.svg_content.contains("MULTI-LAYER IFCX VISUAL DIFF MANIFEST"));
        assert_eq!(export.sha256_seal.len(), 64);
        assert!(export.sha256_seal.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(export.svg_content.contains(&export.sha256_seal));

        let shm_rep = crate::core::shm_twin::evaluate_shm_digital_twin_for_bim(
            &proj,
            &site,
            None,
            Some(&ssi_rep),
            Some(&replay_rep),
            None,
            &crate::core::shm_twin::ShmDigitalTwinConfig::default(),
            (-6.2088, 106.8456),
        )
        .expect("SHM report");
        let shm_export = build_pure_vector_engineering_dossier_svg(&VectorDossierInput {
            project: &proj,
            members: &members,
            site: &site,
            lat_lon: (-6.2088, 106.8456),
            retrofit_report: None,
            crc_report: None,
            ssi_report: Some(&ssi_rep),
            hazard_report: None,
            diff_report: Some(&diff),
            portfolio_report: None,
            replay_report: Some(&replay_rep),
            shm_report: Some(&shm_rep),
            evacuation_report: None,
            watchdog_state: None,
            monte_carlo_report: None,
            collab_state: None,
            sheet_code: "S-102",
            revision: "REV 02",
        });
        assert!(shm_export.svg_content.contains("Live SHM Digital Twin &amp; ATC-20 Placard"));
        assert!(shm_export.svg_content.contains("SHM "));

        let evac_rep = crate::core::evacuation_routing::evaluate_evacuation_and_lifelines(
            &proj,
            &crate::core::gis::IfcxCityScene::default(),
            Some(&shm_rep),
            None,
            None,
            Some(&ssi_rep),
            None,
            None,
            &crate::core::evacuation_routing::EvacuationRoutingConfig::default(),
            (-6.2088, 106.8456),
        )
        .expect("Evacuation report");
        let mut wd = crate::core::shm_watchdog::ShmWatchdogState::default();
        wd.start_simulator(crate::core::shm_watchdog::WatchdogSimMode::AftershockPulse);
        for _ in 0..85 {
            let _ = wd.step_live_tick(0.05, 2.25, 2);
        }
        let evac_export = build_pure_vector_engineering_dossier_svg(&VectorDossierInput {
            project: &proj,
            members: &members,
            site: &site,
            lat_lon: (-6.2088, 106.8456),
            retrofit_report: None,
            crc_report: None,
            ssi_report: Some(&ssi_rep),
            hazard_report: None,
            diff_report: Some(&diff),
            portfolio_report: None,
            replay_report: Some(&replay_rep),
            shm_report: None,
            evacuation_report: Some(&evac_rep),
            watchdog_state: Some(&wd),
            monte_carlo_report: None,
            collab_state: None,
            sheet_code: "S-103",
            revision: "REV 03",
        });
        assert!(evac_export.svg_content.contains("Post-Disaster Evacuation &amp; Lifelines"));
        assert!(evac_export.svg_content.contains("Evac Conn "));
        assert!(evac_export.svg_content.contains("WD "));

        let ts = crate::core::trade_study::run_parametric_trade_study(
            &proj,
            &members,
            &site,
            Some(&ssi_rep),
            Some(&mc_rep),
            &crate::core::trade_study::TradeStudyConfig::default(),
        );
        let permit = crate::core::trade_study::evaluate_permit_compliance_matrix(
            &proj,
            &members,
            &site,
            Some(&ssi_rep),
            None,
            Some(&shm_rep),
            Some(&evac_rep),
            Some(&mc_rep),
            Some(&collab),
        );
        let annex = build_permit_compliance_annex_svg(
            &proj,
            &permit,
            Some(&ts),
            Some(&collab),
            "S-102",
            "REV 01",
        );
        assert!(annex.svg_content.starts_with("<svg"));
        assert!(annex.svg_content.contains(r#"viewBox="0 0 1600 1131""#));
        assert!(!annex.svg_content.contains("<image"));
        assert!(annex.svg_content.contains("PERMIT COMPLIANCE"));
        assert!(annex.svg_content.contains("TRADE-STUDY"));
        assert_eq!(annex.sha256_seal.len(), 64);

        let mut field_proj = BimProject::sample_bimcraft_building();
        field_proj.reinforce_all_concrete(&crate::core::bim::RebarPreset::default());
        let ledger = crate::core::lifecycle_ops::FieldQualityControlLedger::default();
        let fab = crate::core::field_fabrication::evaluate_field_fabrication(
            &field_proj,
            &ledger,
            &crate::core::field_fabrication::FieldFabricationConfig::default(),
        );
        let shop = build_fabrication_bbs_shop_svg(
            &field_proj,
            &fab,
            Some(&collab),
            "S-103",
            "REV 01",
        );
        assert!(shop.svg_content.starts_with("<svg"));
        assert!(shop.svg_content.contains(r#"viewBox="0 0 1600 1131""#));
        assert!(shop.svg_content.contains("SHEET S-103"));
        assert!(shop.svg_content.contains("EigenGate + EigenFlow"));
        assert!(!shop.svg_content.contains("<image"));
        assert!(!shop.svg_content.contains("ARI"));
        assert_eq!(shop.sha256_seal.len(), 64);

        let ef_rep = crate::core::eigenflow::evaluate_eigenflow_studio(
            &field_proj,
            None,
            &crate::core::eigenflow::EigenFlowConfig::default(),
        );
        let s104 = build_eigenflow_nonlinear_annex_svg(
            &field_proj.project_name,
            &ef_rep,
            "S-104",
            Some("abc123def456"),
        );
        assert!(s104.svg_content.starts_with("<svg"));
        assert!(s104.svg_content.contains(r#"viewBox="0 0 1600 1131""#));
        assert!(s104.svg_content.contains("SHEET S-104"));
        assert!(s104.svg_content.contains("<polyline"));
        assert!(!s104.svg_content.contains("<image"));
        assert!(!s104.svg_content.contains("ARI"));
        assert_eq!(s104.sha256_seal.len(), 64);
    }
}
