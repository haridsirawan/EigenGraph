//! settings.rs — Workspace Settings, Design Standards & Vault Connection Preferences.

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::features::FeatureRegistry;
use crate::models::NavigationPillar;
use crate::theme::MacTheme;
use crate::ui_kit;

use crate::core::BimProject;
use crate::models::{ParametricVariable, StructuralMember};
use crate::theme::ThemeMode;
use crate::views::universal_canvas::{UniversalCanvasState, WorkspaceCanvasSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSubTab {
    WorkspaceSettings,
    About,
}

pub struct SettingsView {
    pub active_subtab: SettingsSubTab,
    pub unit_system: String,
    pub design_standard: String,
    pub solver_threads: usize,
    pub enable_svd_guard: bool,
    pub enable_anti_leakage: bool,
    pub freeze_by_default: bool,
    pub gemini_api_key: String,
    pub remember_gemini_key: bool,
    pub export_feedback: Option<String>,
    pub vault_path_input: String,
    pub vault_new_name_input: String,
    pub request_open_vault_popup: bool,
    pub vault_state_changed: bool,
    pub about_category_filter: usize,
    pub about_search_query: String,
}

impl Default for SettingsView {
    fn default() -> Self {
        let (remember_gemini_key, gemini_api_key) =
            crate::core::web_bridge::gemini_key_load_device_opt_in();
        Self {
            active_subtab: SettingsSubTab::WorkspaceSettings,
            freeze_by_default: true,
            unit_system: "kN, mm, MPa (Structural Metric)".to_string(),
            design_standard: "SNI 1729 / AISC 360-16".to_string(),
            solver_threads: 8,
            enable_svd_guard: true,
            enable_anti_leakage: true,
            gemini_api_key,
            remember_gemini_key,
            export_feedback: None,
            vault_path_input: crate::core::InternalVault::default_vault_dir()
                .to_string_lossy()
                .to_string(),
            vault_new_name_input: "EigenGraph_Vault".to_string(),
            request_open_vault_popup: false,
            vault_state_changed: false,
            about_category_filter: 0,
            about_search_query: String::new(),
        }
    }
}

/// Complete Engineering Feature & Mathematical Logic Reference Specification (`Settings -> About EigenGraph`).
pub struct AboutFeatureMathSpec {
    pub category_idx: usize,
    pub category_badge: &'static str,
    pub title: &'static str,
    pub modules_and_codes: &'static str,
    pub capabilities: &'static str,
    pub math_logic: &'static str,
}

pub const ABOUT_MATH_CATEGORIES: [&str; 8] = [
    "All",
    "FEM & Mechanics",
    "EigenGate & Flow",
    "Seismic & Extreme",
    "Concrete/Steel/Geo",
    "Design & Solar",
    "4D-6D & Digital Twin",
    "Geometry & IFCX",
];

pub const ENGINEERING_MATH_CATALOG: &[AboutFeatureMathSpec] = &[
    AboutFeatureMathSpec {
        category_idx: 1,
        category_badge: "FEM & Mechanics",
        title: "6-DOF 3D Frame, Shell & Second-Order Stability Solver",
        modules_and_codes: "core/solver.rs, core/linalg.rs • AISC 360-16 App. 8, SNI 1729",
        capabilities: "Assembles sparse global 6-DOF stiffness matrices for 3D beam-columns and 4-node Mindlin/Kirchhoff shells under multi-load combinations, with Euler elastic buckling and P-Delta moment amplification.",
        math_logic: "K * u = F  (6 DOF/node: ux, uy, uz, rx, ry, rz)\nK_g = sum( T_e^T * K_local,e * T_e )\nEuler Buckling: P_e = (pi^2 * E * I) / (K * L)^2\nP-Delta Amp: B1 = max( C_m / (1 - alpha * P_u / P_e1), 1.0 )",
    },
    AboutFeatureMathSpec {
        category_idx: 2,
        category_badge: "EigenGate & Flow",
        title: "Branch 1: Linear EigenGate Redundancy & SVD Kinematic Guard",
        modules_and_codes: "core/eigengate.rs • UFC 4-023-03 Alternate Path, GSA",
        capabilities: "Evaluates O(N^2) single-element (N-1) and Woodbury simultaneous pair (N-2) outage redundancy, computes spectral leverage & mechanism ratios, and detects geometric instability via SVD.",
        math_logic: "Element Leverage: h_e = k_e * b_e^T * K^-1 * b_e\nKinematic Mechanism Guard: m_e = 1 - h_e  (singularity as m_e -> 0)\nSherman-Morrison (N-1): (K - k_e*b_e*b_e^T)^-1 = K^-1 + (K^-1*b_e*b_e^T*K^-1*k_e) / (1 - h_e)\nWoodbury (N-2): S = I_2 - U^T * K^-1 * U,  det(S) -> Redundancy Pair Index\nSingularity: sigma_min(K) via SVD; Aggregate Redundancy Index (ARI in [0, 1])",
    },
    AboutFeatureMathSpec {
        category_idx: 2,
        category_badge: "EigenGate & Flow",
        title: "Branch 2: EigenFlow Non-Linear Woodbury P1->P2 & Pushover ADRS",
        modules_and_codes: "core/eigenflow.rs, core/pushover.rs, core/nonlinear.rs • ATC-40, ASCE 41-17",
        capabilities: "Models staggered stiffness degradation (lambda), 1-hop & 2-hop topological load-path redistribution, P-Delta pushover capacity curves, ADRS capacity-demand intersection, and plastic hinge states.",
        math_logic: "Degraded Stiffness: K_deg = K - (1 - lambda) * k_e * b_e * b_e^T\nRedistribution Reserve: RRR = V_u / V_y ;  Ductility mu = Delta_u / Delta_y\nADRS Transform: S_a = V_b / (W * alpha_1),  S_d = Delta_roof / (PF_1 * phi_1)\nEffective Damping: beta_eff = 0.05 + kappa * (2/pi) * (mu*alpha - mu + 1) / ...",
    },
    AboutFeatureMathSpec {
        category_idx: 1,
        category_badge: "FEM & Mechanics",
        title: "Modal Eigenanalysis & SNI 1726 / ASCE 7 Response Spectrum",
        modules_and_codes: "core/modal.rs • SNI 1726:2019, ASCE 7-22 Ch. 11-12",
        capabilities: "Solves generalized eigenmodes for periods, mode shapes, and >= 90% participating mass ratio; constructs site-specific design spectra and combines modal demands via CQC/SRSS.",
        math_logic: "Eigenproblem: K * phi_i = omega_i^2 * M * phi_i,  T_i = 2*pi / omega_i\nParticipation: Gamma_i = (phi_i^T * M * r) / (phi_i^T * M * phi_i)\nDesign Spectrum: S_DS = (2/3) * F_a * S_s,  S_D1 = (2/3) * F_v * S_1\nModal Combination: R_CQC = sqrt( sum_i sum_j rho_ij * R_i * R_j )",
    },
    AboutFeatureMathSpec {
        category_idx: 3,
        category_badge: "Seismic & Extreme",
        title: "Implicit HHT-alpha NL-THA, PEER .at2 Scaling & Plastic Hinges",
        modules_and_codes: "core/time_history.rs, core/ground_motion_replay.rs, core/dynamics.rs • ASCE 41-17",
        capabilities: "Executes non-blocking 60 FPS tri-directional (X/Y/Z) nonlinear time-history integration with Bouc-Wen/Takeda hysteresis, Sa(T1) spectrum matching, sudden column-loss transient dynamics, and IO/LS/CP hinge beacons.",
        math_logic: "HHT-alpha: M*a_{n+1} + (1+alpha)*C*v_{n+1} - alpha*C*v_n + (1+alpha)*K*d_{n+1} - alpha*K*d_n = F\nNewmark-HHT Params: gamma = (1 - 2*alpha)/2,  beta = (1 - alpha)^2 / 4,  alpha in [-1/3, 0]\nBouc-Wen Hysteresis: dz/dt = A*du/dt - beta*|du/dt|*|z|^{n-1}*z - gamma*du/dt*|z|^n\nSa(T1) Scale Factor = S_{a,target}(T_1) / S_{a,record}(T_1)",
    },
    AboutFeatureMathSpec {
        category_idx: 2,
        category_badge: "EigenGate & Flow",
        title: "StructGNN-CRC Conformal Screening & Collapse Cascade",
        modules_and_codes: "core/cascade.rs • UFC 4-023-03 Alternate Path",
        capabilities: "Combines spectral EigenGate screening with split conformal risk control (alpha = 0.05) and multi-step progressive collapse cascades under configurable Dynamic Amplification Factors (DAF 1.20x–2.00x).",
        math_logic: "Split Conformal Quantile: q_hat = Quantile_{(1-alpha)(1 + 1/n)}( |y_i - y_hat_i| ), alpha = 0.05\nAlternate-Path Step k: Load_next = Load_initial * DAF  (DAF in [1.20, 2.00])\nPropagation halts when max(DCR_i) <= 1.0 and min(sigma_min) > threshold",
    },
    AboutFeatureMathSpec {
        category_idx: 3,
        category_badge: "Seismic & Extreme",
        title: "Seismic Base Isolation (LRB/FPS) & Supplemental Dampers",
        modules_and_codes: "core/isolation.rs, core/base_isolation_studio.rs • ASCE 7-22 Ch. 17-18, Sheet S-106",
        capabilities: "Sizes Lead-Rubber Bearings, Friction Pendulum isolators, Fluid Viscous Dampers, Buckling-Restrained Braces, and seismic moat clearances with closed hysteresis loops.",
        math_logic: "Isolated Period: T_M = 2*pi * sqrt( W / (g * K_M) )\nMoat Displacement: D_M = (g * S_M1 * T_M) / (4 * pi^2 * B_M)\nBilinear Effective Stiffness: K_eff = K_d + Q_d / D ;  beta_eff = 2*Q_d*(D - D_y) / (pi * K_eff * D^2)\nViscous Damper Force: F_d = C_d * |v|^alpha * sgn(v)",
    },
    AboutFeatureMathSpec {
        category_idx: 3,
        category_badge: "Seismic & Extreme",
        title: "Wind Aeroelasticity, Cladding Pressures & ISO 10137 Comfort",
        modules_and_codes: "core/wind.rs, core/wind_studio.rs • ASCE 7-22 Ch. 26-31, SNI 1727, ISO 10137, Sheet S-107",
        capabilities: "Computes story velocity pressures, gust-effect factors, across-wind vortex shedding frequencies, cladding pressure zones, and 1-yr / 10-yr peak floor acceleration habitability checks.",
        math_logic: "Velocity Pressure: q_z = 0.613 * K_z * K_zt * K_d * K_e * V^2  (N/m^2)\nPeak Base Shear: F_w = q_z * G_f * C_f * A_f\nVortex Shedding: f_s = (St * V_H) / B  (Strouhal St ~ 0.12 - 0.16)\nISO 10137 Comfort: check a_peak <= a_limit(f_1) for Office / Residential occupancy",
    },
    AboutFeatureMathSpec {
        category_idx: 3,
        category_badge: "Seismic & Extreme",
        title: "Extreme Events: Blast (UFC 3-340-02) & Structural Fire (ISO 834/EC)",
        modules_and_codes: "core/blast.rs, core/fire.rs, core/extreme_events.rs • UFC 3-340-02, ISO 834, EN 1993-1-2, S-105",
        capabilities: "Evaluates Kingery-Bulmash blast overpressures (TNT/ANFO/C-4, Surface/Free-Air) & SDOF ductility damage tiers B1–B5, plus 0–180 min thermal curves with Bare/SFRM/Intumescent protection and EigenFlow P1->P2 handoff.",
        math_logic: "Hopkinson-Cranz Scaled Range: Z = R / W^(1/3)  (m / kg^(1/3))\nBlast SDOF Ductility: mu = x_max / x_y ;  Support Rotation theta = atan(2*x_max / L)\nISO 834 Fire Curve: T_g(t) = T_0 + 345 * log10(8*t + 1)  (deg C)\nSteel Thermal Retention: k_E(T), k_y(T) per Eurocode -> degraded member stiffness lambda(T)",
    },
    AboutFeatureMathSpec {
        category_idx: 4,
        category_badge: "Concrete/Steel/Geo",
        title: "Special RC Shear Walls, SBE Confinement & Biaxial P-M Fiber Check",
        modules_and_codes: "core/concrete.rs, core/shear_wall.rs, core/shear_wall_studio.rs • ACI 318-19, SNI 2847, S-108",
        capabilities: "Generates P-Mx-My concrete interaction surfaces with reinforcement layers, checks Special Boundary Element (SBE) triggers, in-plane wall shear capacity, and diagonal coupling beams.",
        math_logic: "Whitney Block: a = beta_1 * c,  C_c = 0.85 * f'_c * a * b ;  epsilon_cu = 0.003\nBresler Biaxial: 1/P_n = 1/P_nx + 1/P_ny - 1/P_0\nSBE Displacement Check: c >= l_w / (600 * max(1.5 * delta_u / h_w, 0.005))\nWall Shear: phi*V_n = phi * A_cv * (alpha_c * lambda * sqrt(f'_c) + rho_t * f_y)\nDiagonal Coupling Beam: V_n = 2 * A_vd * f_y * sin(alpha)",
    },
    AboutFeatureMathSpec {
        category_idx: 4,
        category_badge: "Concrete/Steel/Geo",
        title: "Steel Seismic Connections, Composite Decks & PT Tendon Studios",
        modules_and_codes: "core/connection.rs, core/composite.rs, core/prestressed_concrete.rs • AISC 358/360, DG11, ACI 318, S-109..S-111",
        capabilities: "Verifies RBS/BUEEP moment connections & panel-zone shear, composite slab-beam PNA & headed studs with AISC DG11 footfall vibration, and post-tensioned parabolic tendon drape, friction/wobble losses, and fiber stresses.",
        math_logic: "Panel Zone Shear: R_v = 0.60 * F_y * d_c * t_w * (1 + 3*b_cf*t_cf^2 / (d_b*d_c*t_w))\nStud Capacity: Q_n = 0.5 * A_sa * sqrt(f'_c * E_c) <= R_g * R_p * A_sa * F_u\nDG11 Footfall Accel: a_p / g = (P_0 * exp(-0.35 * f_n)) / (beta * W)\nPT Losses: P(x) = P_0 * exp(-(mu * alpha(x) + K * x)) ;  e(x) = 4*e_max*(x/L)*(1 - x/L)",
    },
    AboutFeatureMathSpec {
        category_idx: 4,
        category_badge: "Concrete/Steel/Geo",
        title: "Deep Foundations, 6-DOF Winkler SSI & Soil Liquefaction Screening",
        modules_and_codes: "core/geotech.rs, core/foundation_studio.rs • SNI 8460:2017, Seed-Idriss, S-113",
        capabilities: "Computes layered borehole shaft + end-bearing pile capacities, Converse-Labarre group efficiency, two-way punching shear around columns, Veletsos-Meek SSI period lengthening, and Seed-Idriss liquefaction FS_L.",
        math_logic: "Single Pile Capacity: Q_u = sum( f_s,i * A_s,i ) + q_p * A_p\nConverse-Labarre Efficiency: eta_g = 1 - (theta / 90) * ((n-1)*m + (m-1)*n) / (m*n)\nPunching Shear: v_u <= phi * 0.33 * lambda * sqrt(f'_c)  on perimeter b_0 = 2*(c_1 + c_2 + 2*d)\nVeletsos-Meek SSI: T_tilde / T = sqrt( 1 + k/K_x * (1 + K_x*h^2/K_theta) )\nLiquefaction Safety Factor: FS_L = (CRR_7.5 * MSF) / CSR",
    },
    AboutFeatureMathSpec {
        category_idx: 2,
        category_badge: "EigenGate & Flow",
        title: "Reliability FORM/SORM, FEMA P-58 & WebGPU @compute Monte Carlo",
        modules_and_codes: "core/reliability.rs, core/fema_p58.rs, core/gpu_fragility_mc.rs • Hasofer-Lind, FEMA P-58, S-112",
        capabilities: "Solves Rackwitz-Fiessler FORM beta-indices and Breitung SORM curvature corrections across limit states, paired with 256–16,384 trial WebGPU @compute Monte Carlo lognormal fragility loss curves (P10/P50/PML-90/CVaR-95).",
        math_logic: "FORM Reliability Index: beta_HL = min_{g(U)=0} ||U||_2  (Rackwitz-Fiessler U-space)\nFailure Probability: P_f,FORM = Phi(-beta_HL) ;  SORM: P_f = Phi(-beta) * prod(1 + beta*kappa_i)^(-1/2)\nLognormal Fragility: P(DS >= ds | IM) = Phi( ln(IM / theta_ds) / beta_ds )\nLoss Metrics: PML-90 = Q_0.90(Loss),  CVaR-95 = E[ Loss | Loss >= Q_0.95 ]",
    },
    AboutFeatureMathSpec {
        category_idx: 5,
        category_badge: "Design & Solar",
        title: "AI Building Design Agent: Zoning, Program, Massing & Chained Concept-to-BIM",
        modules_and_codes: "core/zoning_agent.rs, program_agent.rs, massing_agent.rs, design_agent.rs • S-115",
        capabilities: "Synthesizes GSB/KDB/KLB 3D buildable envelopes, 2D Slicing-Tree floor-plate packing with adjacency scoring, 4-archetype multi-objective massing comparisons, and 1-click Mass-to-Engineered-BIM with live EigenGate audits.",
        math_logic: "Zoning Cap: Footprint_max = (KDB/100)*A_lot,  GFA_max = KLB*A_lot,  GSB setback polygon\nSpatial Packing: Slicing-Tree recursive bipartition minimizing aspect-ratio distortion\nAdjacency Score: S_adj = sum( w_ij / (1 + d_centroid(i, j)) )\nMassing Multi-Objective: J = w_1*ARI - w_2*max(DCR) - w_3*Cost_norm - w_4*CO2_norm",
    },
    AboutFeatureMathSpec {
        category_idx: 5,
        category_badge: "Design & Solar",
        title: "3D Solar OTTV, Cast Shadows, Interior Daylight Autonomy & Finishes",
        modules_and_codes: "core/solar_envelope.rs, core/daylight_interior.rs • SNI 6389, ASHRAE 90.1, EN 17037, LEED v4.1, SNI 6197",
        capabilities: "Computes astronomical sun vectors, N/E/S/W/Roof Solar Factors & SNI 6389 OTTV/RTTV (<= 35 W/m^2), alongside BRE Split-Flux work-plane lux grids, sDA 300/50%, ASE 1000,250h, DGP glare, LEED v4.1 credits, SNI 6197 dimming savings, and Room Finish/FF&E schedules.",
        math_logic: "Sun Altitude & OTTV: sin(alpha_s) = sin(phi)*sin(delta) + cos(phi)*cos(delta)*cos(omega_h) ;  L_s = H / tan(alpha_s)\nOTTV_i = alpha*U_w*(1 - WWR)*TD_eq + U_f*WWR*dT + SC*SF_i*ESF*WWR  (Weighted OTTV <= 35.0 W/m^2)\nBRE Split-Flux DF% = (T_vlt*T_shade*A_win*theta / (A_surf*(1 - R_avg))) * f(d_perim) * 100\nsDA 300/50% & Glare DGP: E >= 300 lx fraction;  DGP = 5.87e-5*E_v + 0.16 + c_ase  (LEED v4.1 0-3 pt + Finish/FFE BOQ)",
    },
    AboutFeatureMathSpec {
        category_idx: 6,
        category_badge: "4D-6D & Digital Twin",
        title: "4D CPM Sequence, 5D AHSP S-Curve, 6D Carbon LCA & 1D Rebar CSO",
        modules_and_codes: "core/schedule.rs, cost.rs, carbon.rs, cso.rs, field_fabrication.rs • EN 15978, SNI AHSP, S-103, S-114",
        capabilities: "Chains CPM forward/backward passes for critical path floats, logistic 5D Capex S-Curves, EN 15978 A1–A5 embodied carbon footprinting, and Best-Fit Decreasing (BFD) 1D master rebar cutting-stock optimization.",
        math_logic: "CPM Float: TF = LS - ES = LF - EF  (Critical Path when TF == 0)\nLogistic S-Curve: C(t) = C_total / (1 + exp(-k * (t - t_mid)))\nEmbodied Carbon: CO2e = sum_i ( Mass_i * EF_i(SteelRoute, ConcreteMix) )\n1D Cutting Stock: minimize sum_k y_k  s.t.  sum_i L_i*x_{ik} <= L_stock*y_k (BFD heuristic)",
    },
    AboutFeatureMathSpec {
        category_idx: 6,
        category_badge: "4D-6D & Digital Twin",
        title: "Live SHM Digital Twin, STA/LTA Watchdog & A* Evacuation Lifelines",
        modules_and_codes: "core/shm_twin.rs, shm_watchdog.rs, evacuation_routing.rs • ATC-20, Welch PSD",
        capabilities: "Processes triaxial sensor telemetry via Welch PSD & Modal Assurance Criterion (MAC), triggers rolling STA/LTA seismic alarms for Green/Yellow/Red ATC-20 placards, and routes A* safe egress over IfcRoad/IfcBridge graphs.",
        math_logic: "Modal Assurance Criterion: MAC(phi_a, phi_m) = |phi_a^T * phi_m|^2 / (||phi_a||^2 * ||phi_m||^2)\nSTA/LTA Trigger: R(t) = avg(|x|, W_short) / avg(|x|, W_long) >= theta_trig\nA* Lifeline Routing: f(n) = g(n) * penalty_debris(ATC-20, flood, liq) + ||p_n - p_goal||_2",
    },
    AboutFeatureMathSpec {
        category_idx: 7,
        category_badge: "Geometry & IFCX",
        title: "Half-Edge Geometry, BSP Solid CSG, Living Earth WebGPU & IFCX SHA-256",
        modules_and_codes: "core/geom.rs, core/csg.rs, core/gis.rs, core/ifcx.rs, vector_dossier.rs • IFC 5 ECS, FIPS 180-4, S-000",
        capabilities: "Provides planar half-edge arrangement, straight-skeleton roofs, non-recursive BSP CSG wall openings, WGS-84/ECEF/RTC-ENU Pure WebGPU WGSL shaders (material_kind 0..33: biome mountains, Beer-Lambert ocean waves, traffic headlights, curved auto-bridges, road-snapped IfcBuildings, hardware 3D Section Box & Underground X-Ray), 3-way IFCX merging, and S-000 / S-100..S-118 SHA-256 vector sheets.",
        math_logic: "Geodesic Transform: Geodetic(lat, lon, h) -> ECEF(X,Y,Z) -> RTC ENU(e,n,u) - cam_origin\nOcean & Biomes: C = mix(C_deep, C_shallow, exp(-depth * k_rgb)), Gerstner waves, FBM rock/forest strata\nSection Box Clipping: if p_enu notin [clip_min, clip_max] { discard; } + Underground X-Ray CompareFunction::Always\nIFCX & SHA-256 Seal: Non-destructive ECS composition (IfcSite/IfcRoad/IfcBridge/IfcBuilding) + FIPS 180-4",
    },
    AboutFeatureMathSpec {
        category_idx: 6,
        category_badge: "4D-6D & Digital Twin",
        title: "Building MEP Systems (HVAC, Plumbing, Elec, Fire) & Unified 3D Clash / BCF 2.1",
        modules_and_codes: "core/mep_engine.rs, core/clash_engine.rs • ASHRAE, IPC, IEC, NFPA 13, BCF 2.1, S-116..S-117",
        capabilities: "Synthesizes BIM-coupled HVAC ducting, Hunter plumbing risers, 3-phase electrical feeders & NFPA 13 sprinkler networks in 3D ENU space (material_kind = 31.0), and runs 3D Structure x MEP x Openings interference audits with BCF 2.1 pins (material_kind = 32.0) and 1-click ceiling plenum auto-resolution.",
        math_logic: "HVAC Cooling & Duct: Q_tr = (A_gfa*q_base + Q_OTTV) / 3.517 ;  D_eq = 1.30*(a*b)^0.625 / (a+b)^0.25\nIPC & IEC Sizing: Q_plumb = C_H * WSFU^0.48 ;  %Vd = sqrt(3)*I*L*(R*cos(phi) + X*sin(phi)) / V_LL * 100\n3D Clash & Plenum Guardrail: overlap_enu = max(0, min(b_max) - max(a_min)) -> delta_z = pen_z + clearance_tol",
    },
    AboutFeatureMathSpec {
        category_idx: 7,
        category_badge: "Geometry & IFCX",
        title: "Subsurface Urban Utilities, Smart Road Intersections (HCM LOS) & Native 15-Tool Agent",
        modules_and_codes: "core/gis/urban_utilities.rs, city_promotion.rs, core/agent_tools.rs • Hazen-Williams, Manning, HCM 6th Ed., S-118",
        capabilities: "Routes buried water, sewer/storm & 20 kV feeders under authored IfcRoad/IfcBridge corridors with X-Ray depth bypass (material_kind = 33.0), detects T-Junction / 4-Way / Roundabout hubs with HCM LOS grading & live City-to-BIM promotion, and exposes 15 deterministic headless tools via native Gemini function calling.",
        math_logic: "Hazen-Williams Headloss: S_f = 10.67 * Q^1.852 / (C^1.852 * D^4.87)\nManning Sewer & Storm: Q = (1/n) * A * R_h^(2/3) * S^(1/2) + Rational Q = C_r * i * A_catchment\nHCM Intersection LOS: d = d_1(C, v/c) + d_2(c, T) -> LOS A..F ;  Loop: Gemini functionCall -> execute_headless_tool -> functionResponse",
    },
];

impl SettingsView {
    /// Captures the active workspace settings snapshot for single-file `.eigen.json` synchronization.
    pub fn workspace_settings_snapshot(
        &self,
        theme: &MacTheme,
        canvas: &UniversalCanvasState,
        ai_copilot_visible: bool,
    ) -> WorkspaceCanvasSettings {
        WorkspaceCanvasSettings {
            theme_mode: theme.mode,
            canvas_height_ratio: canvas.canvas_height_ratio,
            freeze_by_default: self.freeze_by_default,
            show_grid: canvas.show_grid,
            snap_to_grid: canvas.snap_to_grid,
            grid_spacing_mm: canvas.grid_spacing_mm,
            show_bim_base_layer: canvas.show_bim_base_layer,
            show_domain_overlay: canvas.show_domain_overlay,
            sync_all_layers: canvas.sync_all_layers,
            show_floating_inspector: canvas.inspector_open,
            show_ai_copilot: ai_copilot_visible,
            unit_system: self.unit_system.clone(),
            design_standard: self.design_standard.clone(),
            solver_threads: self.solver_threads,
            enable_svd_guard: self.enable_svd_guard,
            enable_anti_leakage: self.enable_anti_leakage,
        }
    }

    /// Applies imported single-file `.eigen.json` workspace settings across theme, AI copilot, and preferences.
    pub fn apply_workspace_settings(
        &mut self,
        settings: &WorkspaceCanvasSettings,
        theme: &mut MacTheme,
        ai_copilot_visible: &mut bool,
    ) {
        if theme.mode != settings.theme_mode {
            *theme = MacTheme::for_mode(settings.theme_mode);
        }
        self.freeze_by_default = settings.freeze_by_default;
        self.unit_system = settings.unit_system.clone();
        self.design_standard = settings.design_standard.clone();
        self.solver_threads = settings.solver_threads;
        self.enable_svd_guard = settings.enable_svd_guard;
        self.enable_anti_leakage = settings.enable_anti_leakage;
        *ai_copilot_visible = settings.show_ai_copilot;
    }

    /// Renders Consolidated Workspace Settings & About.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        theme: &mut MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        ai_copilot_visible: &mut bool,
        bim_project: &BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
        _registry: &mut FeatureRegistry,
        _active_pillar: &mut NavigationPillar,
    ) -> bool {
        ui_kit::subtab_bar(
            ui,
            theme,
            "settings_subtabs",
            &mut self.active_subtab,
            &[
                (SettingsSubTab::WorkspaceSettings, "⚙️ Workspace Settings"),
                (SettingsSubTab::About, "ℹ️ About EigenGraph"),
            ],
        );
        ui.add_space(10.0);

        ScrollArea::vertical()
            .id_salt("settings_scroll_body")
            .auto_shrink([false, false])
            .show(ui, |ui| match self.active_subtab {
                SettingsSubTab::WorkspaceSettings => {
                    self.render_preferences(
                        ui,
                        theme,
                        universal_canvas,
                        ai_copilot_visible,
                        bim_project,
                        members,
                        variables,
                    );
                }
                SettingsSubTab::About => {
                    self.render_about(ui, theme);
                }
            });

        false
    }

    #[allow(clippy::too_many_arguments)]
    fn render_preferences(
        &mut self,
        ui: &mut Ui,
        theme: &mut MacTheme,
        universal_canvas: &mut UniversalCanvasState,
        ai_copilot_visible: &mut bool,
        bim_project: &BimProject,
        members: &[StructuralMember],
        variables: &[ParametricVariable],
    ) {
        crate::icons::icon_label(
            ui,
            crate::icons::AppIcon::SettingsGear,
            20.0,
            theme.accent_blue,
            "Workspace Settings",
            20.0,
            theme.text_primary,
            true,
        );
        ui.label(
            RichText::new(
                "Unified theme, universal 15-module canvas defaults, single-file sync, engineering standards, solver safeguards, and AI copilot configuration.",
            )
            .size(12.5)
            .color(theme.text_secondary),
        );
        ui.add_space(12.0);

        let mut pending_theme_mode: Option<ThemeMode> = None;
        ui_kit::responsive_two_columns(
            ui,
            640.0,
            |ui| {
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("APPEARANCE & AI COPILOT")
                            .strong()
                            .size(11.5)
                            .color(theme.accent_blue),
                    );
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(
                        RichText::new("Workspace Theme Mode:")
                            .size(12.5)
                            .color(theme.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        for mode in [ThemeMode::ProDark, ThemeMode::EngineeringLight] {
                            let is_active = theme.mode == mode;
                            if crate::icons::icon_button(
                                ui,
                                mode.app_icon(),
                                mode.label(),
                                11.5,
                                if is_active {
                                    Color32::WHITE
                                } else {
                                    theme.text_primary
                                },
                                if is_active {
                                    theme.accent_blue
                                } else {
                                    theme.bg_sidebar
                                },
                                Stroke::NONE,
                                egui::CornerRadius::same(7),
                                Vec2::ZERO,
                            )
                            .clicked()
                            {
                                pending_theme_mode = Some(mode);
                            }
                        }
                    });

                    ui.add_space(12.0);
                    ui.checkbox(
                        ai_copilot_visible,
                        RichText::new("Show Floating AI Copilot Pill (Cmd+K)")
                            .size(12.0)
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new(
                            "Hide or reveal the bottom-right AI Copilot floating pill across all 15 modules.",
                        )
                        .size(11.0)
                        .color(theme.text_muted),
                    );
                });
            },
            |ui| {
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("UNIFIED 15-MODULE CANVAS DEFAULTS")
                            .strong()
                            .size(11.5)
                            .color(theme.accent_blue),
                    );
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Canvas Viewport Height Ratio:")
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut universal_canvas.canvas_height_ratio,
                                0.45..=1.15,
                            )
                            .fixed_decimals(2),
                        );
                    });
                    ui.add_space(6.0);

                    if ui
                        .checkbox(
                            &mut self.freeze_by_default,
                            RichText::new("Freeze Canvas by Default (Wheel Scrolls Page Only)")
                                .size(12.0)
                                .color(theme.text_primary),
                        )
                        .changed()
                    {
                        universal_canvas.is_frozen = self.freeze_by_default;
                    }
                    ui.checkbox(
                        &mut universal_canvas.sync_all_layers,
                        RichText::new("Synchronize Drawings Across All 15 Modules")
                            .size(12.0)
                            .color(theme.text_primary),
                    );
                    ui.checkbox(
                        &mut universal_canvas.show_bim_base_layer,
                        RichText::new("Show BIM Base Plan Underlay")
                            .size(12.0)
                            .color(theme.text_primary),
                    );
                    ui.checkbox(
                        &mut universal_canvas.show_domain_overlay,
                        RichText::new("Show Module Telemetry HUD Overlay")
                            .size(12.0)
                            .color(theme.text_primary),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(
                            &mut universal_canvas.show_grid,
                            RichText::new("Grid").size(12.0).color(theme.text_primary),
                        );
                        ui.checkbox(
                            &mut universal_canvas.snap_to_grid,
                            RichText::new("Snap").size(12.0).color(theme.text_primary),
                        );
                        ui.checkbox(
                            &mut universal_canvas.inspector_open,
                            RichText::new("Floating Inspector")
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                    });

                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Grid Spacing (mm):")
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        egui::ComboBox::from_id_salt("settings_canvas_grid_spacing")
                            .selected_text(format!("{:.0} mm", universal_canvas.grid_spacing_mm))
                            .show_ui(ui, |ui| {
                                for step in [250.0_f64, 500.0, 1000.0, 1500.0] {
                                    ui.selectable_value(
                                        &mut universal_canvas.grid_spacing_mm,
                                        step,
                                        format!("{step:.0} mm"),
                                    );
                                }
                            });
                    });
                });
            },
        );
        if let Some(mode) = pending_theme_mode {
            *theme = MacTheme::for_mode(mode);
            theme.apply_to_ctx(ui.ctx());
        }

        ui.add_space(12.0);

        self.render_vault_settings_card(ui, theme);

        ui.add_space(12.0);

        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("1-FILE SYNCHRONIZED WORKSPACE ARCHIVE (.eigen.json)")
                    .strong()
                    .size(11.5)
                    .color(theme.accent_blue),
            );
            ui.separator();
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "Synchronized state: {} canvas objects • {} structural members • {} parametric variables • {} BIM elements",
                    universal_canvas.entities.len(),
                    members.len(),
                    variables.len(),
                    bim_project.walls.len() + bim_project.columns.len() + bim_project.runs.len(),
                ))
                .size(11.5)
                .color(theme.text_secondary),
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::SaveDisk,
                    "Export 1-File Unified Workspace (.eigen.json)",
                    11.5,
                    Color32::WHITE,
                    theme.accent_blue,
                    Stroke::NONE,
                    egui::CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    let snap = self.workspace_settings_snapshot(
                        theme,
                        universal_canvas,
                        *ai_copilot_visible,
                    );
                    universal_canvas.export_unified_file(bim_project, members, variables, snap);
                }

                if crate::icons::icon_button(
                    ui,
                    crate::icons::AppIcon::UploadImport,
                    "Import 1-File Archive (.eigen.json)",
                    11.5,
                    theme.text_primary,
                    theme.bg_sidebar,
                    Stroke::NONE,
                    egui::CornerRadius::same(7),
                    Vec2::ZERO,
                )
                .clicked()
                {
                    universal_canvas.import_json_open = true;
                }
            });
            if let Some(status) = &universal_canvas.status_message {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(status)
                        .size(11.0)
                        .color(theme.accent_green),
                );
            }
        });

        ui.add_space(12.0);

        ui_kit::responsive_two_columns(
            ui,
            640.0,
            |ui| {
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("ENGINEERING UNITS & DESIGN STANDARDS")
                            .strong()
                            .size(11.5)
                            .color(theme.accent_blue),
                    );
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label(RichText::new("Active Unit System:").size(12.5).color(theme.text_primary));
                    egui::ComboBox::from_id_salt("settings_unit_system")
                        .selected_text(&self.unit_system)
                        .width(ui.available_width().min(280.0))
                        .show_ui(ui, |ui| {
                            for opt in [
                                "kN, mm, MPa (Structural Metric)",
                                "kN, m, kPa (Civil / Geotech Metric)",
                                "kips, in, ksi (US Imperial)",
                            ] {
                                ui.selectable_value(&mut self.unit_system, opt.to_string(), opt);
                            }
                        });

                    ui.add_space(10.0);
                    ui.label(RichText::new("Design Standard:").size(12.5).color(theme.text_primary));
                    egui::ComboBox::from_id_salt("settings_design_standard")
                        .selected_text(&self.design_standard)
                        .width(ui.available_width().min(280.0))
                        .show_ui(ui, |ui| {
                            for opt in [
                                "SNI 1729 / AISC 360-16",
                                "SNI 2847:2019 / ACI 318-19",
                                "Eurocode 2 / Eurocode 3 / EN 1998",
                            ] {
                                ui.selectable_value(&mut self.design_standard, opt.to_string(), opt);
                            }
                        });
                });
            },
            |ui| {
                theme.card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("SOLVER & NUMERICAL SAFEGUARDS")
                            .strong()
                            .size(11.5)
                            .color(theme.accent_blue),
                    );
                    ui.separator();
                    ui.add_space(8.0);

                    ui.checkbox(
                        &mut self.enable_svd_guard,
                        RichText::new("Enable SVD Kinematic Guard (Sherman-Morrison margin check)")
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new("Instantly detects mechanism formation upon N-1 member damage without pseudoinverse.")
                            .size(11.0)
                            .color(theme.text_muted),
                    );

                    ui.add_space(8.0);
                    ui.checkbox(
                        &mut self.enable_anti_leakage,
                        RichText::new("Enforce Strict Pre-Analysis Protocol (Leakage-Free)")
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new("Guarantees screening upper bounds are computed 100% from intact structure facts.")
                            .size(11.0)
                            .color(theme.text_muted),
                    );

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Parallel Solver Threads:").color(theme.text_primary));
                        ui.add(egui::Slider::new(&mut self.solver_threads, 1..=32));
                    });
                });
            },
        );

        ui.add_space(12.0);
        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("AI COPILOT INTEGRATION")
                    .strong()
                    .size(11.5)
                    .color(theme.accent_blue),
            );
            ui.separator();
            ui.add_space(8.0);

            ui.label(RichText::new("Google Gemini API Key:").size(12.5).color(theme.text_primary));
            let key_resp = ui.add(
                egui::TextEdit::singleline(&mut self.gemini_api_key)
                    .password(true)
                    .desired_width(ui.available_width().min(380.0)),
            );
            let rem_resp = ui.checkbox(
                &mut self.remember_gemini_key,
                "Remember API key on this device (local storage only; never exported into .eigen archives)",
            );
            if rem_resp.changed() || (self.remember_gemini_key && key_resp.changed()) {
                crate::core::web_bridge::gemini_key_save_device_opt_in(
                    self.remember_gemini_key,
                    &self.gemini_api_key,
                );
            }
            ui.label(
                RichText::new(
                    "Powers live Gemini 2.5 Flash & Live Stream orchestration inside the Dynamic Island.",
                )
                .size(11.0)
                .color(theme.text_muted),
            );
        });
    }

    fn render_vault_settings_card(&mut self, ui: &mut Ui, theme: &MacTheme) {
        use crate::core::vault::VaultBackendKind;
        use crate::icons::{self, AppIcon};

        let status = crate::core::InternalVault::connection_status();
        let categories_count = if status.connected {
            crate::core::InternalVault::list_all_vault_categories().len()
        } else {
            0
        };
        let projects_count = if status.connected {
            crate::core::InternalVault::list_personal_vault_projects().len()
        } else {
            0
        };

        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                icons::icon_label(
                    ui,
                    AppIcon::Database,
                    14.0,
                    theme.accent_blue,
                    "PERSONAL ENGINEERING VAULT CONNECTION",
                    12.0,
                    theme.accent_blue,
                    true,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (badge_txt, badge_col) = if status.connected && status.needs_permission {
                        ("PERMISSION NEEDED", theme.accent_orange)
                    } else if status.connected {
                        ("CONNECTED", theme.accent_green)
                    } else if status.guest_mode {
                        ("GUEST SESSION", theme.accent_orange)
                    } else {
                        ("DISCONNECTED", theme.text_muted)
                    };
                    Frame::new()
                        .fill(badge_col.gamma_multiply(0.18))
                        .stroke(Stroke::new(1.0, badge_col.gamma_multiply(0.6)))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(RichText::new(badge_txt).size(10.0).strong().color(badge_col));
                        });
                });
            });
            ui.separator();
            ui.add_space(6.0);

            let backend_label = match status.backend {
                VaultBackendKind::LocalFilesystem => "Local Disk Directory",
                VaultBackendKind::WebDirectoryPicker => "Browser File System Access API (OS Folder)",
                VaultBackendKind::BrowserIndexedDb => "Browser IndexedDB Persistent Store",
                VaultBackendKind::Unbound => "No Vault Connected",
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Backend:").size(11.5).strong().color(theme.text_secondary));
                ui.label(RichText::new(backend_label).size(11.5).color(theme.text_primary));
                if status.connected {
                    ui.label(
                        RichText::new(format!(
                            "• {} Projects • {} Folders",
                            projects_count, categories_count
                        ))
                        .size(11.5)
                        .color(theme.accent_blue),
                    );
                }
            });
            ui.horizontal_wrapped(|ui| {
                icons::draw_icon(ui, AppIcon::Folder, 12.5, theme.accent_blue);
                ui.label(
                    RichText::new(&status.location_label)
                        .size(11.0)
                        .monospace()
                        .color(theme.text_secondary),
                );
            });

            if !status.status_msg.is_empty() {
                ui.label(RichText::new(&status.status_msg).size(10.5).color(theme.text_muted));
            }
            if !status.error_msg.is_empty() {
                ui.label(RichText::new(&status.error_msg).size(10.5).color(theme.accent_red));
            }
            if let Some(ref fb) = self.export_feedback {
                ui.label(RichText::new(fb).size(11.0).color(theme.accent_green));
            }

            ui.add_space(8.0);

            let outcome = crate::ui_kit::render_obsidian_vault_panel(
                ui,
                theme,
                &mut self.vault_path_input,
                &mut self.vault_new_name_input,
                false,
                false,
                true,
            );
            if outcome.state_changed {
                self.vault_state_changed = true;
            }
            if let Some(fb) = outcome.feedback {
                self.export_feedback = Some(fb);
            }
            if outcome.open_startup_modal {
                self.request_open_vault_popup = true;
            }
        });
    }

    fn render_about(&mut self, ui: &mut Ui, theme: &MacTheme) {
        theme.card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("ABOUT EIGENGRAPH — FEATURE CATALOG & MATHEMATICAL LOGIC")
                        .strong()
                        .size(12.5)
                        .color(theme.accent_blue),
                );
                ui.label(
                    RichText::new(format!("v{}  •  {} Core Engines", env!("CARGO_PKG_VERSION"), ENGINEERING_MATH_CATALOG.len()))
                        .strong()
                        .size(11.5)
                        .color(theme.accent_green),
                );
            });
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Runtime:").strong().size(11.5).color(theme.text_primary));
                let runtime = if cfg!(target_arch = "wasm32") {
                    "Browser (Pure WebGPU / WGSL) • WebAssembly (wasm32)"
                } else {
                    "EigenGraph Native & Web Engine (wgpu / WGSL)"
                };
                ui.label(RichText::new(runtime).size(11.5).color(theme.text_secondary));
                ui.label(RichText::new("•  Standards:").strong().size(11.5).color(theme.text_primary));
                ui.label(
                    RichText::new("AISC 360/358 • ACI 318-19 • ASCE 7-22 / 41-17 • SNI 1726/1729/2847/6389/8460 • ASHRAE / IPC / NFPA 13 • HCM 6th Ed. • BCF 2.1 • ISO 834 / 10137")
                        .size(11.0)
                        .color(theme.text_secondary),
                );
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Integrity & License:").strong().size(11.5).color(theme.text_primary));
                ui.label(
                    RichText::new("NIST FIPS 180-4 SHA-256 Validated • Dual MIT / Apache 2.0  •  ")
                        .size(11.0)
                        .color(theme.text_secondary),
                );
                ui.hyperlink_to(
                    "github.com/haridsirawan/EigenGraph",
                    "https://github.com/haridsirawan/EigenGraph",
                );
            });

            ui.add_space(10.0);
            ui.label(
                RichText::new("DOMAIN FILTER & FORMULA SEARCH")
                    .strong()
                    .size(11.5)
                    .color(theme.accent_blue),
            );
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                for (idx, cat_lbl) in ABOUT_MATH_CATEGORIES.iter().enumerate() {
                    let active = self.about_category_filter == idx;
                    if crate::icons::icon_button(
                        ui,
                        crate::icons::AppIcon::Layers,
                        *cat_lbl,
                        10.5,
                        if active { Color32::WHITE } else { theme.text_primary },
                        if active { theme.accent_blue } else { theme.bg_sidebar },
                        Stroke::NONE,
                        CornerRadius::same(7),
                        Vec2::ZERO,
                    )
                    .clicked()
                    {
                        self.about_category_filter = idx;
                    }
                }
            });

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Search:").strong().size(11.5).color(theme.text_primary));
                let search_w = (ui.available_width() - 120.0).clamp(180.0, 420.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.about_search_query)
                        .desired_width(search_w)
                        .hint_text("Filter features, formulas, codes (e.g., Woodbury, OTTV, HHT, ACI)..."),
                );
                if !self.about_search_query.is_empty() && ui.button("Clear").clicked() {
                    self.about_search_query.clear();
                }
            });

            ui.add_space(10.0);
            let q = self.about_search_query.trim().to_ascii_lowercase();
            let mut matched_count = 0usize;
            for entry in ENGINEERING_MATH_CATALOG {
                if self.about_category_filter != 0 && entry.category_idx != self.about_category_filter {
                    continue;
                }
                if !q.is_empty()
                    && !entry.title.to_ascii_lowercase().contains(&q)
                    && !entry.modules_and_codes.to_ascii_lowercase().contains(&q)
                    && !entry.capabilities.to_ascii_lowercase().contains(&q)
                    && !entry.math_logic.to_ascii_lowercase().contains(&q)
                    && !entry.category_badge.to_ascii_lowercase().contains(&q)
                {
                    continue;
                }
                matched_count += 1;
                Frame::new()
                    .fill(theme.bg_sidebar.gamma_multiply(0.55))
                    .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.32)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(12, 9))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_wrapped(|ui| {
                            Frame::new()
                                .fill(theme.accent_blue.gamma_multiply(0.18))
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(entry.category_badge)
                                            .size(9.8)
                                            .strong()
                                            .color(theme.accent_blue),
                                    );
                                });
                            ui.label(
                                RichText::new(entry.title)
                                    .size(12.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                        });
                        ui.label(
                            RichText::new(entry.modules_and_codes)
                                .size(10.2)
                                .monospace()
                                .color(theme.accent_green),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(entry.capabilities)
                                .size(11.0)
                                .color(theme.text_secondary),
                        );
                        ui.add_space(5.0);
                        Frame::new()
                            .fill(theme.bg_canvas.gamma_multiply(0.85))
                            .stroke(Stroke::new(1.0, theme.accent_blue.gamma_multiply(0.28)))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(
                                    RichText::new("Governing Mathematical Logic:")
                                        .size(9.8)
                                        .strong()
                                        .color(theme.accent_blue),
                                );
                                ui.label(
                                    RichText::new(entry.math_logic)
                                        .size(10.4)
                                        .monospace()
                                        .color(theme.text_primary),
                                );
                            });
                    });
                ui.add_space(6.0);
            }

            if matched_count == 0 {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("No matching feature or mathematical formulation found.")
                            .size(11.2)
                            .color(theme.text_muted),
                    );
                    if ui.button("Reset Filter").clicked() {
                        self.about_category_filter = 0;
                        self.about_search_query.clear();
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase77_about_eigengraph_release_highlights_render_cleanly() {
        let mut settings = SettingsView {
            active_subtab: SettingsSubTab::About,
            ..Default::default()
        };
        let mut theme = MacTheme::for_mode(ThemeMode::ProDark);
        let mut canvas = UniversalCanvasState::default();
        let mut ai_vis = false;
        let bim = BimProject::default();
        let members: Vec<StructuralMember> = Vec::new();
        let vars: Vec<ParametricVariable> = Vec::new();
        let mut registry = FeatureRegistry::default();
        let mut pillar = NavigationPillar::MyLibrary;

        let ctx = egui::Context::default();
        for win_w in [1280.0_f32, 390.0] {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(win_w, 800.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| {
                settings.show(
                    ui,
                    &mut theme,
                    &mut canvas,
                    &mut ai_vis,
                    &bim,
                    &members,
                    &vars,
                    &mut registry,
                    &mut pillar,
                );
            });
            out.textures_delta.clear();
        }
    }

    #[test]
    fn test_v0863_about_feature_catalog_and_math_logic_filters() {
        assert_eq!(ENGINEERING_MATH_CATALOG.len(), 20);
        for spec in ENGINEERING_MATH_CATALOG {
            assert!(!spec.title.is_empty());
            assert!(!spec.modules_and_codes.is_empty());
            assert!(!spec.capabilities.is_empty());
            assert!(!spec.math_logic.is_empty());
            assert!(
                spec.category_idx > 0 && spec.category_idx < ABOUT_MATH_CATEGORIES.len(),
                "Invalid category index for {}",
                spec.title
            );
        }

        let mut settings = SettingsView {
            active_subtab: SettingsSubTab::About,
            about_category_filter: 0,
            about_search_query: String::new(),
            ..Default::default()
        };
        let mut theme = MacTheme::for_mode(ThemeMode::EngineeringLight);
        let mut canvas = UniversalCanvasState::default();
        let mut ai_vis = false;
        let bim = BimProject::default();
        let members: Vec<StructuralMember> = Vec::new();
        let vars: Vec<ParametricVariable> = Vec::new();
        let mut registry = FeatureRegistry::default();
        let mut pillar = NavigationPillar::MyLibrary;
        let ctx = egui::Context::default();

        for (cat, q) in [
            (0usize, "Woodbury"),
            (5usize, "OTTV"),
            (3usize, "HHT-alpha"),
        ] {
            settings.about_category_filter = cat;
            settings.about_search_query = q.to_string();
            for win_w in [1280.0_f32, 390.0] {
                let raw = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::new(win_w, 800.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(raw, |ui| {
                    settings.show(
                        ui,
                        &mut theme,
                        &mut canvas,
                        &mut ai_vis,
                        &bim,
                        &members,
                        &vars,
                        &mut registry,
                        &mut pillar,
                    );
                });
                out.textures_delta.clear();
            }
        }
    }

    #[test]
    fn test_v088_milestone_release_version_and_about_catalog() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.8.8");
        assert!(ENGINEERING_MATH_CATALOG.len() >= 20);
        assert!(ENGINEERING_MATH_CATALOG
            .iter()
            .any(|s| s.title.contains("Building MEP Systems") && s.modules_and_codes.contains("S-116..S-117")));
        assert!(ENGINEERING_MATH_CATALOG
            .iter()
            .any(|s| s.title.contains("Subsurface Urban Utilities") && s.modules_and_codes.contains("S-118")));
    }
}
