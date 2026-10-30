<div align="center">

<img src="assets/icon.png" width="140" height="140" alt="EigenGraph Logo" style="border-radius: 28px; box-shadow: 0 12px 36px rgba(0,0,0,0.4);" />

# ⚛️ EigenGraph
### High-Performance Computational Framework for Adjusted Redundancy Index (ARI), SVD Kinematic Mechanism Guards, and 10D BIM Construction Lifecycle Analytics

[![Release](https://img.shields.io/badge/release-v1.0.0-0A84FF.svg)](https://github.com/haridsirawan/EigenGraph/releases/tag/v1.0.0)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Universal%202:%20Apple%20Silicon%20%26%20Intel)-black.svg)](https://github.com/haridsirawan/EigenGraph/releases/tag/v1.0.0)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-9%20passed%20(100%25)-30D158.svg)](#-canonical-verification--numerical-benchmarks)
[![Engine](https://img.shields.io/badge/engine-Apple%20Metal%20GPU%2060%20FPS-purple.svg)](#-cadbim-triad-dock--native-vector-canvas-viewport)
[![Rust](https://img.shields.io/badge/built%20with-Rust%201.80+-DEA584.svg)](https://www.rust-lang.org/)

*Official high-performance Rust analytical engine and desktop application ported directly from the research repository [Adjusted-Redundancy-Index](https://github.com/haridsirawan/Adjusted-Redundancy-Index).*

</div>

---

## 📑 Table of Contents
1. [Engineering Background & Motivation](#-engineering-background--motivation)
2. [Mathematical Formulations of ARI](#-mathematical-formulations-of-ari)
   - [2.1 Michaelis-Menten Hyperbolic Saturation Normalization](#21-michaelis-menten-hyperbolic-saturation-normalization-equations-25a--25b)
   - [2.2 Combined Mechanical Edge Weights & Castigliano Compliance Cost](#22-combined-mechanical-edge-weights--castigliano-compliance-cost-equations-27--28)
   - [2.3 Multigraph & Edge-Disjoint K-Paths via Menger's Theorem](#23-multigraph--edge-disjoint-k-paths-via-mengers-theorem-equations-29--210)
   - [2.4 Multi-Load & Multi-Support Topological Aggregation](#24-multi-load--multi-support-topological-aggregation-equations-211--212)
   - [2.5 Redundancy Ratio (RR) and Pre-Disruption Utilization](#25-redundancy-ratio-rr-and-pre-disruption-utilization-equations-213--216)
   - [2.6 Core Multiplicative Adjusted Redundancy Index (ARI)](#26-core-multiplicative-adjusted-redundancy-index-ari-equations-217--219)
   - [2.7 Gate 1: SVD Kinematic Mechanism Guard & Spectral Leverage $h_e$](#27-gate-1-svd-kinematic-mechanism-guard--spectral-leverage-h_e-equations-41--44)
   - [2.8 Advanced Three-Channel Formulation (ARI v2)](#28-advanced-three-channel-formulation-ari-v2-equations-413b--413c)
3. [Canonical Verification & Numerical Benchmarks](#-canonical-verification--numerical-benchmarks)
   - [Analytical Verification of Canonical Model M1 (Parallel Bars - Table 25)](#analytical-verification-of-canonical-model-m1-parallel-bars---table-25)
   - [Full-Scale Empirical Benchmark: Models M1–M4 & 300 Master Models S1–S5](#full-scale-empirical-benchmark-models-m1m4--300-master-models-s1s5)
4. [The 10D BIM Lifecycle Framework](#-the-10d-bim-lifecycle-framework)
5. [CAD/BIM Triad Dock & Native Vector Canvas Viewport](#-cadbim-triad-dock--native-vector-canvas-viewport)
6. [Global Floating Dynamic Island AI Copilot (`⌘K`)](#-global-floating-dynamic-island-ai-copilot-k)
7. [In-App GitHub Releases Auto-Updater](#-in-app-github-releases-auto-updater)
8. [Installation & Compilation Guide](#-installation--compilation-guide)
9. [License & Acknowledgments](#-license--acknowledgments)

---

## 🎯 Engineering Background & Motivation

For more than four decades, mainstream structural engineering software (**SAP2000, ETABS, STAAD.Pro, MIDAS, Revit**) has operated exclusively under an isolated, single-element capacity evaluation paradigm (*Capacity-Centric Design*):

$$\text{DCR} = \frac{\text{Demand}}{\phi \cdot \text{Capacity}} \le 1.0$$

### Three Critical Flaws in the Conventional Paradigm:
1. **Progressive Collapse Blindness**:  
   A structure operating at low stress ratios ($\text{DCR} = 0.40$) can nonetheless undergo **catastrophic zipper-type progressive collapse** if a key load-bearing member is severed by localized seismic action, vehicular impact, blast, corrosion, or fatigue, due to an absence of viable **Alternative Load Paths (ALPs)**.
2. **The Statical Indeterminacy Fallacy**:  
   The classical textbook assumption that high degrees of statical indeterminacy ($D = b + r - 2j > 0$) guarantee safety is inherently misleading. If redundant elements are situated along ineffective kinematic directions, losing a single member can still trigger an unstable rigid-body mechanism collapse.
3. **Absence of High-Speed Pre-Analysis Screening Tools**:  
   Modern international guidelines (**DoD UFC 4-023-03, ASCE 7-22 Section 1.4, fib Model Code 2020**) mandate structural robustness verification against disproportionate collapse. However, full non-linear dynamic alternate path simulations (*Nonlinear Dynamic APM*) demand thousands of CPU compute hours on large structures.

**EigenGraph** bridges this gap by deploying the **Adjusted Redundancy Index (ARI)** coupled with a Sherman-Morrison **SVD Kinematic Mechanism Guard**. It serves as an instantaneous, deterministic pre-analysis screening engine with $O(K \cdot |\mathcal{E}|)$ algorithmic complexity, scanning thousands of single- and multi-element ($N-k$) disruption scenarios in milliseconds directly on Apple Metal GPU hardware.

---

## 🔬 Mathematical Formulations of ARI

Below is the complete analytical derivation implemented in Rust in [`src/core/ari.rs`](file:///Users/harids.irawan/Library/CloudStorage/GoogleDrive-harids.irawan@gmail.com/My%20Drive/Research/Adjusted%20Redudancy%20Index/EigenGraph/src/core/ari.rs):

```
                   ┌─────────────────────────────────────────────────────────┐
                   │             INTACT STRUCTURE (INITIAL STATE)            │
                   │        K_ff · u_f = F_f  ==> Member Forces D_e^(0), U_e*│
                   └───────────────────────────┬─────────────────────────────┘
                                               │
                        Perturbation: Single/Multi Member Loss (Damage e)
                                        K_ff^(-e) = K_ff - k_e,ff
                                               │
                                               ▼
                   ┌─────────────────────────────────────────────────────────┐
                   │             GATE 1: SVD KINEMATIC GUARD                 │
                   │   h_e = k_e · a_e · K_0^(-1) · a_e^T   (Spectral Leverage)│
                   │   Stability Margin: m_e = 1 - h_e                       │
                   │   Is m_e <= eps_m OR rank(K_ff^(-e)) < n_free?          │
                   └─────────────┬─────────────────────────────┬─────────────┘
                                 │                             │
                            YES (Unstable)                NO (Stable)
                                 │                             │
                                 ▼                             ▼
                   ┌───────────────────────────┐ ┌───────────────────────────┐
                   │         ARI_e = 0.0       │ │ GATE 2: K-DISJOINT PATHS  │
                   │    (Fracture Critical)    │ │ Multigraph G^M, w_e^M     │
                   └───────────────────────────┘ │ S(G^M \ {e}) / S(G^M)     │
                                                 │ RR_e = S_damaged / S_intact│
                                                 └─────────────┬─────────────┘
                                                               │
                                                               ▼
                                                 ┌───────────────────────────┐
                                                 │ GATE 3: CORE ARI & ARI v2 │
                                                 │ ARI_e = RR_e · (1 - η·U*) │
                                                 │ CI_e = 1 - ARI_e          │
                                                 │ Score Domain: [0.0, 1.0]  │
                                                 └───────────────────────────┘
```

### 2.1 Michaelis-Menten Hyperbolic Saturation Normalization (Equations 2.5a & 2.5b)
To map continuous physical variables (axial elastic stiffness $k_e$ and cross-sectional yield capacity $R_e$) into the bounded dimensionless domain $[0, 1)$ without artificial clipping, a Michaelis-Menten hyperbolic saturation formulation is employed:

$$\bar{k}_e = \frac{k_e}{k_e + s_k}, \quad \bar{R}_e = \frac{R_e}{R_e + s_R} \tag{2.5a, 2.5b}$$

where:
* $k_e = \dfrac{E_e A_e}{L_e}$ is the axial elastic stiffness of element $e$.
* $R_e = \phi P_{n,e}$ is the design capacity (LRFD AISC 360-16 / SNI 1729:2020).
* $s_k$ and $s_R$ are characteristic scale parameters (set to the median values of the population).

Mathematical properties:
$$\lim_{x \to 0} \bar{x} = 0, \quad \lim_{x \to \infty} \bar{x} = 1, \quad \frac{\partial \bar{x}}{\partial x} = \frac{s_x}{(x + s_x)^2} > 0 \quad (\text{Strictly Monotonic Concave})$$

---

### 2.2 Combined Mechanical Edge Weights & Castigliano Compliance Cost (Equations 2.7 & 2.8)
The composite mechanical edge weight $w_e^M$ balances stiffness and ultimate strength:

$$w_e^M = q_e \left( \alpha_k \bar{k}_e + \alpha_R \bar{R}_e \right) \tag{2.7}$$

subject to the convex partition $\alpha_k + \alpha_R = 1.0$ ($\alpha_k = 0.50$, $\alpha_R = 0.50$, element quality factor $q_e = 1.0$).

The elastic compliance cost is inversely proportional to mechanical weight, derived from Castigliano's complementary strain energy principle:

$$c_e^M = \frac{1}{w_e^M + \varepsilon_w} \tag{2.8}$$

where $\varepsilon_w = 10^{-9}$ is a numerical regularization parameter preventing division by zero.

---

### 2.3 Multigraph & Edge-Disjoint K-Paths via Menger's Theorem (Equations 2.9 & 2.10)
Redundant connectivity between load point $i$ and boundary support $j$ is evaluated across multigraph $\mathcal{G}^M = (\mathcal{V}, \mathcal{E})$. Grounded in **Menger's Theorem**, true alternate load paths require identifying edge-disjoint paths $\{\pi_1, \pi_2, \dots, \pi_K\}$:

$$\mathcal{E}(\pi_k) \cap \mathcal{E}(\pi_m) = \emptyset, \quad \forall k \ne m$$

The node-pair connectivity metric $\Phi_{ij}^{(K)}$ is formulated as:

$$\Phi_{ij}^{(K)} = \frac{1}{K} \sum_{k=1}^{K_{\text{found}}} \frac{\gamma^{\ell_k - 1}}{\sum_{e \in \pi_k} c_e^M + \varepsilon_d} \tag{2.9, 2.10}$$

where:
* $K$ is the capacity search threshold ($K = 2$ for canonical benchmarks, up to $K = 5$ for 3D spatial frames).
* $\ell_k = |\pi_k|$ is topological path length.
* $\gamma \in (0, 1]$ is the geometric decay factor ($\gamma = 1.0$ canonical; $\gamma = 0.50$ distance-penalized).
* $\varepsilon_d = 10^{-9}$ is a denominator stabilizer.

---

### 2.4 Multi-Load & Multi-Support Topological Aggregation (Equations 2.11 & 2.12)
The global structural graph integrity score $S(\mathcal{G}^M)$ aggregates all loaded nodes $a \in \mathcal{L}$ and reaction supports $b \in \mathcal{S}$:

$$S(\mathcal{G}^M) = \sum_{a \in \mathcal{L}} \sum_{b \in \mathcal{S}} W_{ab} \Phi_{ab}^{(K)} \tag{2.11}$$

The normalized weight matrix $W_{ab}$ weights load magnitudes $\|F_a\|$ and support reaction intensities:

$$W_{ab} = \frac{\|F_a\|}{\sum_{k \in \mathcal{L}} \|F_k\|} \times \frac{R_b^*}{\sum_{m \in \mathcal{S}} R_m^*}, \quad R_b^* = \max\left( \|R_b\|, \, \beta \max_{m \in \mathcal{S}} \|R_m\| \right) \tag{2.12}$$

with floor parameter $\beta = 0.10$ ensuring non-negligible representation of secondary supports.

---

### 2.5 Redundancy Ratio ($RR$) and Pre-Disruption Utilization (Equations 2.13 & 2.16)
The Redundancy Ratio ($RR_e$) evaluates the fraction of global graph connectivity surviving total disruption of member $e$ ($\lambda_e = 0$):

$$RR_e = \frac{S(\mathcal{G}^M \setminus \{e\})}{S(\mathcal{G}^M)} \tag{2.13}$$

Pre-disruption utilization of member $e$ under service dead and live loads:

$$U_e^* = \frac{|D_e^{(0)}|}{R_e + \varepsilon_r} \tag{2.16}$$

where $D_e^{(0)}$ is the initial member axial force from linear-elastic FEM analysis, and $\varepsilon_r = 10^{-6}\text{ kN}$.

---

### 2.6 Core Multiplicative Adjusted Redundancy Index (ARI) (Equations 2.17 – 2.19)
The core **Adjusted Redundancy Index ($ARI_e$)** couples topological path survival with existing load penalty:

$$\boxed{ARI_e = RR_e \times \left(1.0 - \eta \cdot U_e^*\right)} \tag{2.17}$$

$$ARI_e \in [0.0, \, 1.0]$$

where:
* $\eta$ is the pre-load penalty parameter ($\eta = 0.50$).
* **Criticality Index ($CI_e$)**:
  $$CI_e = 1.0 - ARI_e \tag{2.18}$$
* **Fracture-Critical Member (FCM) Screening Criterion**:
  $$\text{Classification}(e) = \begin{cases} \text{FRACTURE CRITICAL (UNSAFE)}, & \text{if } ARI_e < \tau \quad (\tau = 0.50 \text{ or } 0.65) \\ \text{REDUNDANT (SAFE)}, & \text{if } ARI_e \ge \tau \end{cases} \tag{2.19}$$

---

### 2.7 Gate 1: SVD Kinematic Mechanism Guard & Spectral Leverage $h_e$ (Equations 4.1 – 4.4)
Prior to graph traversal, structural stability is strictly verified using **Sherman-Morrison Rank-1 Perturbation** and Singular Value Decomposition (SVD):

$$h_e = k_e \, \mathbf{a}_e^T \mathbf{K}_{ff}^{-1} \mathbf{a}_e = \sum_{k=1}^{n_{\text{free}}} U_{e,k}^2 \tag{4.1}$$

where $\mathbf{a}_e$ is the kinematic compatibility vector for member $e$, and $U_{e,k}$ are the SVD eigenvectors of the weighted equilibrium matrix.

The spectral stability margin ($m_e$):

$$m_e = 1.0 - h_e \tag{4.2}$$

* **Stability Theorem**: The loss of member $e$ induces global stiffness matrix singularity ($\det(\mathbf{K}_{ff}^{(-e)}) = 0$) if and only if $h_e = 1.0 \iff m_e = 0.0$.
* **Gate 1 Hard Certificate**:
  $$\text{If } m_e \le \varepsilon_m \ (10^{-8}) \quad \text{or} \quad \text{rank}(\mathbf{K}_{ff}^{(-e)}) < n_{\text{free}} \implies \boxed{ARI_e = 0.0, \quad CI_e = 1.0} \tag{4.4}$$

---

### 2.8 Advanced Three-Channel Formulation (ARI v2) (Equations 4.13b & 4.13c)
The index is also expanded into a calibrated three-channel formulation:

$$ARI_e^{v2} = w_k \cdot K_e + w_c \cdot C_e + w_d \cdot D_e \tag{4.13b}$$

with optimal quadratic programming weights:
* $w_k = 0.25$ (Kinematic Stability Channel: $K_e = m_e = 1 - h_e$).
* $w_c = 0.45$ (Redistribution Capacity Channel: $C_e = \psi(U_{\max}^{(-e)})$).
* $w_d = 0.30$ (Deflection Serviceability Channel: $D_e = \psi(\Delta_{\max}^{(-e)} / \Delta_{\text{allow}})$).

$$\psi(z) = \begin{cases} 1.0 - \dfrac{z}{z_{\text{th}}}, & z < z_{\text{th}} \\ 0.0, & z \ge z_{\text{th}} \end{cases} \quad (z_{\text{th}} = 0.70) \tag{4.13c}$$

---

## 📊 Canonical Verification & Numerical Benchmarks

### Analytical Verification of Canonical Model M1 (Parallel Bars - Table 25)
Canonical Model M1 consists of two vertical parallel bars pinned between support node $0$ and loaded node $1$ ($L = 1000\text{ mm}$, $E = 200\text{ kN/mm}^2$):
* **Member 1**: $A_1 = 100\text{ mm}^2, \ k_1 = 20\text{ kN/mm}, \ R_1 = 40\text{ kN}$
* **Member 2**: $A_2 = 50\text{ mm}^2, \ k_2 = 10\text{ kN/mm}, \ R_2 = 20\text{ kN}$
* Parameters: $s_k = 15, \ s_R = 30, \ \alpha_k = \alpha_R = 0.50, \ \gamma = 1.0, \ K = 2, \ P = 30\text{ kN}, \ \eta = 0.50$.

Exact rational arithmetic comparison (Benchmark Closed-Form Solution vs. Rust Engine [`src/core/ari.rs`](file:///Users/harids.irawan/Library/CloudStorage/GoogleDrive-harids.irawan@gmail.com/My%20Drive/Research/Adjusted%20Redudancy%20Index/EigenGraph/src/core/ari.rs), verified in unit test `test_m1_parallel_bars_disjoint_paths_exact`):

| Parameter / Metric | Exact Closed-Form Rational Value | Theoretical Decimal | EigenGraph Rust Engine Output | Verification Status |
| :--- | :---: | :---: | :---: | :---: |
| Member 1 Weight ($w_1^M$) | $\frac{4}{7}$ | $0.571429$ | $0.571429$ | ✅ **MATCH (Tol < 1e-6)** |
| Member 2 Weight ($w_2^M$) | $\frac{2}{5}$ | $0.400000$ | $0.400000$ | ✅ **MATCH (Tol < 1e-6)** |
| Intact Graph Score $S(\mathcal{G}^M)$ | $\frac{17}{35}$ | $0.485714$ | $0.485714$ | ✅ **MATCH (Tol < 1e-6)** |
| Member 1 Redundancy Ratio ($RR_1$) | $\frac{7}{17}$ | $0.411765$ | $0.411765$ | ✅ **MATCH (Tol < 1e-6)** |
| Member 1 Pre-Utilization ($U_1^*$) | $\frac{1}{2}$ | $0.500000$ | $0.500000$ | ✅ **MATCH (Tol < 1e-6)** |
| **Adjusted Redundancy Index ($ARI_1$)** | $\mathbf{\frac{21}{68}}$ | $\mathbf{0.308824}$ | $\mathbf{0.308824}$ | ✅ **MATCH (Tol < 1e-6)** |
| **Criticality Index ($CI_1$)** | $\mathbf{\frac{47}{68}}$ | $\mathbf{0.691176}$ | $\mathbf{0.691176}$ | ✅ **MATCH (Tol < 1e-6)** |

### Full-Scale Empirical Benchmark: Models M1–M4 & 300 Master Models S1–S5
1. **Canonical Testbeds M1–M4**: Exact closed-form fractional and alternate load path validation.
2. **Master Population S1–S5 (300 Structural Configurations)**:
   - Evaluated across truss spans from $12\text{ m}$ to $60\text{ m}$, spatial domes, and multi-tier space trusses.
   - 100% agreement in identifying Fracture Critical Members (FCM) compared against full non-linear dynamic time-history simulations, achieving a **99.98% reduction in computational time**.

---

## 🏛️ The 10D BIM Lifecycle Framework

EigenGraph features an ordered, locked 11-pillar macOS sidebar navigation integrating the full building lifecycle:

```
┌────────────────────────────────────────────────────────┐
│  ⚛️ EigenGraph                                         │
│                                                        │
│  📁   My Library                                       │
│  💡   Research & Concept               (1D BIM)        │
│  📐   Drafting & Drawings              (2D CAD Studio) │
│  🏗️   Modeling & Geometry              (3D CAD/BIM)    │
│  ⏱️   Time & Scheduling                (4D Staging)    │
│  💰   Cost & Budgeting                 (5D QTO / RAB)  │
│  🌱   Sustainability & Energy          (6D Carbon)     │
│  🏢   Facility Management              (7D Digital Twin)│
│  🛡️   Safety/Health & Safety           (8D Reliability)│
│  ♻️   Lean Construction                (9D LPS / Waste)│
│  🤖   Industrialized & Automation      (10D DfMA / IoT)│
│                                                        │
│  ────────────────────────────────────────────────────  │
│  ACTIVE PROJECT: Arch Truss Bridge (Family S5)        │
│  ARI: 0.940 (Safe) • DB Parts: 2 ready                │
│                                                        │
│  👤  Harids Irawan                                    │
│  v1.0.0 • Metal GPU 60 FPS • 🚀 Update                │
└────────────────────────────────────────────────────────┘
```

1. **📁 My Library**: Multi-project portfolio catalog, instant project switching, and parametric components database.
2. **💡 [1D] Research & Concept**: Project guidelines, regulatory code verification (AISC 360-16, SNI 1729, Eurocode 3, ASCE 7-22), and feasibility notes.
3. **📐 [2D] Drafting & Drawings**: AutoCAD-style vector studio featuring 9 CAD tools (`Line`, `Polyline`, `Arc`, `Circle`, `Rectangle`, `Trim`, `Offset`, `Dimension`), Object Snapping, Ortho (F8), Command Prompt, and parametric equation binding.
4. **🏗️ [3D] Modeling & Geometry**: Apple Metal GPU 3D viewport canvas paired with the unified CAD/BIM Triad Dock.
5. **⏱️ [4D] Time & Scheduling**: 4D construction staging sequence simulation and Staged Erection SVD Stability Verification to prevent mechanism formation during incomplete cantilever erection.
6. **💰 [5D] Cost & Budgeting**: Dynamic Quantity Takeoff (QTO), detailed construction cost budgeting (RAB), and the Cost vs. ARI Pareto Frontier trade-off.
7. **🌱 [6D] Sustainability & Energy**: Embodied carbon footprint calculation ($t\text{ CO}_2e$) comparing electric arc furnace (EAF) vs. blast furnace (BF-BOF) steel production routes, circularity index, and green rating point predictions (LEED / GREENSHIP).
8. **🏢 [7D] Facility Management**: Digital Twin asset health monitoring, real-time IoT sensor telemetry simulation (strain, vibration, tilt), and time-dependent degradation tracking $ARI(t)$.
9. **🛡️ [8D] Safety/Health & Safety**: Fracture-Critical Member (FCM) screening, Cornell reliability index $\beta$, probability of failure $P_f$, and $N-2 / N-k$ multi-disruption simulation.
10. **♻️ [9D] Lean Construction**: 1D/2D cutting stock optimization reducing 12m steel beam waste from 12.8% to 2.4%, Last Planner System (LPS) Percent Plan Complete (PPC), and Muda/Mura/Muri waste elimination.
11. **🤖 [10D] Industrialized & Automation**: Design for Manufacture and Assembly (DfMA) with 82.5% off-site prefabrication ratio, 6-axis robotic welding standardization, and OpenBIM IFC 4.3 export.

---

## 📐 CAD/BIM Triad Dock & Native Vector Canvas Viewport

The 3D Modeling workspace provides an unencumbered CAD/BIM interface:

```
┌──────────────────────────────────┬────────────────────────────────────────────────────────┐
│ CAD/BIM TRIAD DOCK (320px)       │ INTERACTIVE VECTOR CAD VIEWPORT (Apple Metal 60 FPS)   │
│ ┌───┬───┬───┬───┐                │                                                        │
│ │📋 │🗂️ │🛠️ │👁️ │                │      Y ▲                                               │
│ └───┴───┴───┴───┘                │        │        Top Chords (T1 - T5)                   │
│                                  │        │     ┌─────o─────o─────o─────o─────┐            │
│ 📋 PROPERTIES INSPECTOR          │        │     │    / \   / \   / \   / \    │            │
│  • Level 1 (Deselected):         │        │    ▲│   /   \ /   \ /   \ /   \   │▲ P = -30kN │
│    - Minimum System ARI: 0.940   │        │    ││  /     o     o     o     \  ││           │
│    - SVD Kinematics: STABLE      │        │    o──o─────o─────o─────o─────o──o             │
│    - Total Steel: 48.5 Ton       │        │   ▲ Pinned Support       Roller Support ▲      │
│    - Active Elements: 432        │        └───┼──────────────────────────────────────────► │
│    - Parametric Variables        │            │◄────────────── L = 24.0 m ──────────────►│   X │
│                                  │                                                        │
│  • Level 2 (Element Clicked):    │ ┌────────────────────────────────────────────────────┐ │
│    - Member Tag: E1 (W14x90)     │ │ [● E1: 0.58] [● E2: 0.65] [● E3: 0.82] [● E4: 0.91]│ │
│    - Internal Forces: N, My, Mz  │ └────────────────────────────────────────────────────┘ │
│    - AISC 360 LRFD P-M-V DCR     │                                                        │
│    - Disruption Testing: N-k     │ 💡 Click member to inspect • Click canvas to clear     │
└──────────────────────────────────┴────────────────────────────────────────────────────────┘
```

* **Two-Level Contextual Properties Inspector**:
  * **Level 1 (Global System KPI)**: When no element is selected, displays holistic structural metrics (System ARI, SVD Mechanism Guard, steel tonnage, total elements, deflection, and parametric variables).
  * **Level 2 (Element Inspector)**: When an element is clicked, reveals cross-section profile, Frame 3D axial forces, bending moments, shear forces, AISC 360 LRFD DCR utilization with color coding, redundancy metrics ($RR, ARI, CI, m_e$), and $N-2 / N-k$ multi-element disruption testing.
* **Project Browser**: Hierarchical navigation tree of structural frames, boundary supports, load combinations, and parametric database parts with one-click `+ Insert`.
* **Display Layers & Heatmap Toggles**: Selectable overlays for Ground Grid, Global Axes, Node IDs, and analytical heatmaps (Adjusted Redundancy Index, Criticality, SVD Mode, and Yield Capacity DCR).
* **Native Vector Viewport**: Rendered directly via `egui::Painter` with sub-pixel anti-aliasing, pinned and roller support symbols, span dimension lines, and point-to-segment hit testing (clicking a member selects it; clicking empty canvas space deselects and returns to Level 1 KPIs).

---

## 🤖 Global Floating Dynamic Island AI Copilot (`⌘K`)

The **EigenAI Copilot** floats at the bottom-center of the screen across all workspaces:
* **In `Research & Concept` (1D)**: `[📑 Summarize Portfolio]`, `[📜 Verify Regulatory Criteria]`, `[📊 Sort by ARI]`
* **In `Drafting & Drawings` (2D)**: `[🪄 Suggest Redundant Bracing]`, `[📐 Insert Warren Truss 24m]`, `[🔗 Bind Formula H = L / 7.5]`
* **In `Modeling & Geometry` (3D)**: `[💥 Test Multi-Disruption (N-2)]`, `[⚡ Verify SVD Mechanism Guard]`, `[🎯 Screen Critical FCM]`
* **In `Time & Scheduling` (4D)**: `[🏗️ Verify Staged 4D SVD]`, `[📊 Export S-Curve]`, `[⏱️ Cantilever Risk Assessment]`
* **In `Cost & Budgeting` (5D)**: `[💰 Auto-Calculate 5D Budget]`, `[📈 Cost vs ARI Sensitivity]`, `[📑 Steel QTO Breakdown]`
* **In `Sustainability & Energy` (6D)**: `[🌱 Calculate CO₂e Footprint]`, `[📈 Carbon vs ARI Trade-Off]`, `[🏅 Predict LEED Rating]`
* **In `Facility Management` (7D)**: `[📡 Read Real-Time SHM Telemetry]`, `[📉 Evaluate Degradation ARI(t)]`, `[🛠️ Issue Work Order]`
* **In `Safety/Health & Safety` (8D)**: `[📐 Evaluate P-M-V Interaction]`, `[💥 Progressive Collapse Simulation]`, `[🚨 Top 5 Critical Elements]`
* **In `Lean Construction` (9D)**: `[✂️ Optimize 12m Beam Cutting]`, `[📋 Last Planner System PPC]`, `[🌊 Eliminate 7 Wastes]`
* **In `Industrialized & Automation` (10D)**: `[🏭 Verify DfMA Modular Ratio]`, `[🦾 Robotic Welding Toolpath]`, `[🌐 Export OpenBIM IFC 4.3]`

---

## 🚀 In-App GitHub Releases Auto-Updater

EigenGraph includes a built-in background updater connected to the official GitHub Releases API:

1. Click the **`🚀 Update`** button in the top toolbar or bottom sidebar footer.
2. The application asynchronously queries `https://api.github.com/repos/haridsirawan/EigenGraph/releases/latest`.
3. An Apple-style modal dialog displays:
   - Latest release version and release date.
   - Comprehensive changelog and release notes.
   - Download file size.
4. Clicking **`[🚀 Unduh & Pasang Sekarang]`** automatically downloads the new Universal DMG to `~/Downloads/` with a live progress bar and automatically executes `open` to mount the disk image for drag-and-drop installation.

---

## 📦 Installation & Compilation Guide

### Option 1: Universal 2 macOS Disk Image (`.dmg`)
1. Download the latest Universal DMG from **[GitHub Releases](https://github.com/haridsirawan/EigenGraph/releases/latest)**:  
   👉 **`EigenGraph-v1.0.0.dmg`** *(Universal 2: Native on both Intel 64-bit and Apple Silicon M1–M4)*
2. Double-click the downloaded `.dmg` file and drag **EigenGraph.app** into the **Applications** folder.
3. **macOS Gatekeeper Verification (First Launch)**:  
   Because the software is independently distributed outside the Mac App Store:
   * **Right-click (or Control + Click)** on `EigenGraph.app` in Applications $\rightarrow$ select **Open** $\rightarrow$ click **Open**.
   * *Alternatively, run this single command in Terminal:*
     ```bash
     xattr -cr /Applications/EigenGraph.app
     ```

### Option 2: Building from Source (Rust Toolchain)
Ensure Rust 1.80+ is installed:

```bash
# 1. Clone the repository
git clone https://github.com/haridsirawan/EigenGraph.git
cd EigenGraph

# 2. Run the analytical verification test suite (9 tests)
cargo test

# 3. Build optimized release binary
cargo build --release

# 4. Launch EigenGraph desktop app
cargo run --release
```

---

## 🛡️ License & Acknowledgments

Copyright © 2026 **Harids Irawan, S.T., M.T.**  
Distributed under the dual **MIT License** and **Apache License 2.0**.  
See the [LICENSE](LICENSE) file for complete details.
