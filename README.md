<div align="center">

<img src="assets/icon.png" width="140" height="140" alt="EigenGraph Logo" style="border-radius: 28px; box-shadow: 0 12px 36px rgba(0,0,0,0.4);" />

# EigenGraph
### Desktop BIM & Structural Graph Analytics Powered by Adjusted Redundancy Index (ARI)

[![Release](https://img.shields.io/badge/release-v1.0.0-0A84FF.svg)](https://github.com/haridsirawan/EigenGraph/releases/tag/v1.0.0)
[![Platform](https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon%20%26%20Intel)-black.svg)](#)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE)
[![Research](https://img.shields.io/badge/research-Doctoral%20Dissertation-30D158.svg)](#)

*Developed by **Harids Irawan** (Doctoral Research in Structural Redundancy & Progressive Collapse Prevention).*

</div>

---

## 🎯 The Paradigm Shift: Why EigenGraph?

Traditional structural engineering CAD and BIM software (**SAP2000, ETABS, Revit, STAAD.Pro, Tekla**) share a fundamental 40-year-old limitation: **they are purely capacity-centric ($DCR \le 1.0$) rather than redundancy-centric.**

| Feature | Conventional BIM / FEM (SAP2000, ETABS, Revit) | **EigenGraph (Adjusted Redundancy Index)** |
| :--- | :--- | :--- |
| **Safety Evaluation** | Static Demand-to-Capacity Ratio ($DCR = \frac{\text{Demand}}{\phi \cdot \text{Capacity}} \le 1.0$) | **Adjusted Redundancy Index (ARI)** + SVD Topology Nullspace Screening |
| **Collapse Risk** | **Blind to Progressive Collapse**: A structure with $DCR = 0.5$ can still suffer total unzipping collapse if a single key member fails. | **Simulates Alternate Load Paths**: Perturbs individual members to guarantee safe force redistribution without collapse. |
| **Kinematic Stability** | Determinant checks ($|K| \approx 0$) often throw opaque numerical divergence errors. | **SVD Kinematic Guard**: Jacobi decomposition identifies exact rigid-body mechanism vectors ($\sigma_i < \varepsilon_{tol}$). |
| **BIM 5D Cost Intelligence** | Separate offline spreadsheet / manual RAB calculation. | **Cost vs. ARI Sensitivity Trade-Off ($\Delta \text{Cost} \text{ vs } \Delta \text{ARI}$)**: Know the exact budget needed to eliminate collapse risks. |
| **BIM 4D Scheduling** | External Primavera / MS Project Gantt charts without structural checks. | **Staged Erection SVD Guard**: Verifies zero mechanism during cantilever erection phases (WBS 1.0–6.0). |
| **User Experience** | Cluttered 90s-era dialog boxes, modal windows, and thousands of nested menus. | **Apple HIG Full-Height Sidebar + Floating Dynamic Island AI Copilot (`⌘K`)**. |

---

## 🔬 Mathematical Backbone: Adjusted Redundancy Index (ARI)

EigenGraph treats structural frameworks as **high-dimensional mathematical graphs and stiffness networks**. The core engine (`src/core/`) formulates redundancy through rigorous matrix mechanics:

### 1. SVD Kinematic Stability Guard
Before any load is applied, EigenGraph performs **Two-Sided Jacobi Singular Value Decomposition (SVD)** on the partitioned free-DOF stiffness matrix $K_{ff}$:

$$K_{ff} = U \Sigma V^T$$

The rank and nullspace dimension verify structural determinacy:

$$\text{rank}(K_{ff}) = n_{\text{free}} \implies \text{Mechanically Certified Unconditionally Stable}$$

If any singular value drops below tolerance $\sigma_i < \varepsilon_{\text{tol}} \cdot \sigma_{\max}$ ($\varepsilon_{\text{tol}} = 10^{-7}$), the corresponding column of $V$ reveals the **exact physical kinematic collapse mechanism mode**.

### 2. Adjusted Redundancy Index (ARI) Formulation
For every member $i$ in the structural model, EigenGraph simulates single-member sudden elimination ($K_{(-i)}$). If the damaged network remains kinematic-stable, the member's Adjusted Redundancy Index is calculated as:

$$ARI_i = 1.0 - \frac{\max_j \left(U_j^{(-i)} - U_j^{(0)}\right)}{1.0 - U_j^{(0)}}$$

* **$ARI_i \ge 0.85$ (Grade A - Highly Redundant)**: Alternate load paths absorb redistributed forces with negligible stress increase.
* **$0.70 \le ARI_i < 0.85$ (Grade B - Moderate Redundancy)**: Safe under standard design loads; secondary members carry reserve stresses.
* **$ARI_i < 0.70$ (Grade C/D - Critical Element)**: **Catastrophic single point of failure.** Sudden loss causes progressive unzipping or mechanism formation.

---

## 🏛️ The 5 Pillars Architecture

EigenGraph organizes structural engineering into 5 cohesive workspaces accessible via an **Apple HIG Full-Height Vertical Navigation Sidebar**:

```
┌──────────────────┬──────────────────────────────────────────────────────────────┐
│  ⚛️ EigenGraph    │  [Top Toolbar] 📐 Parametric — AutoCAD Studio     [🤖 Ask AI]│
├──────────────────┼──────────────────────────────────────────────────────────────┤
│  📁 My Project   │  [AutoCAD Ribbon: Line, Polyline, Arc, Dim, Trim, Offset]     │
│  📐 Parametric   │  ───────────────────────────────────────────────────────────  │
│  🏗️ Build        │  Driving Variables Table  │  Free-hand Drawing Canvas         │
│  ⚡ Analyze      │  • L = 24000 mm           │  (Dynamic Cursor HUD, Snap,       │
│  ⚙️ Settings     │  • H = L / 7.5 (3200 mm)  │   AutoCAD Command Line Prompt)    │
│                  │  • n = 6 panels           │                                   │
│  ──────────────  │                           │                                   │
│  Active Project  │  ───────────────────────────────────────────────────────────  │
│  ARI: 0.94 Safe  │  [Status Bar] 🟢 Model: Jembatan Busur | 60 FPS Metal GPU     │
│  DB: 3 Parts     │                                                               │
│                  │       [ ⚛️ EigenAI  •  📐 Parametric  |  Ask or type ⌘K... ]   │
│  👤 Researcher   │                      (Floating Dynamic Island)                │
└──────────────────┴──────────────────────────────────────────────────────────────┘
```

1. **📁 MY PROJECT (Zero-Home App Store Portfolio)**
   - Hero Featured Project Banner & 3-column curated project card grid.
   - Instant visual metrics: System ARI Gauge, element count, and kinematic stability badges.
   - One-click export for **Structural Engineering Dossiers (PDF)** and client presentations.

2. **📐 PARAMETRIC (AutoCAD Free Drawing Studio & DB Persistence)**
   - 2D/3D Drafting Ribbon: `Line (L)`, `Polyline (PL)`, `Arc (A)`, `Circle (C)`, `Rectangle (REC)`, `Dimension (DIM)`, `Trim (TR)`, `Offset (O)`.
   - Dynamic Snapping (`🧲 SNAP: ON`), Ortho Mode (`📐 ORTHO: ON - F8`), and Dynamic Input HUD.
   - **Parametric DAG Variables Table**: Bind geometric formulas ($H = L / 7.5, n = 6$) that update node coordinates topologically.
   - **`💾 Save to Database`**: Automatically stores drawn parametric truss/frame components into the project component database.

3. **🏗️ BUILD (3D Assembly Workspace & Dedicated CAD Sidebar)**
   - Hardware-accelerated 3D viewport canvas.
   - **Dedicated 4-Tab Modeling Sidebar**:
     - ⚙️ *Properties Inspector*: Axial force $N_e$, capacity $\phi P_n$, utilization $U^*$, SVD margin $m_e$.
     - 📁 *Project Browser*: Hierarchy tree including **`📦 Imported from PARAMETRIC DB`** for one-click component insertion.
     - 🛠️ *Modify Tools*: Move, Rotate 3D, Scale, Subdivide Member, Mirror, Array.
     - 📑 *Display Layers*: Ground Grid, Axes, Wireframe, Node IDs, Axial Heatmap, Deformed Mesh.

4. **⚡ ANALYZE (Multi-Dimensional Engineering & Project Intelligence)**
   - **Sub-Tab 1: Stabilitas SVD Guard & Redundansi (ARI)**:
     - Real-time SVD rank verification ($\text{rank}(K_{ff}) = n_{free}$) to eliminate unconstrained mechanisms.
     - Member-by-member sensitivity screening table sorted by critical ARI.
   - **Sub-Tab 2: Estimasi Biaya & RAB (BIM 5D)**:
     - Automatic Quantity Takeoff (QTO): steel tonnage (48.5 Ton), fireproofing area ($1,420\text{ m}^2$), concrete footing volume ($18.5\text{ m}^3$).
     - AHSP Unit Price Table (Procurement, workshop fabrication, transportation, crane erection).
     - **Cost vs. ARI Sensitivity Curve**: Analyzes the monetary trade-off to elevate system redundancy from 0.78 to 0.92.
   - **Sub-Tab 3: Penjadwalan & Kurva S (BIM 4D)**:
     - Work Breakdown Structure (WBS 1.0–6.0) across 140 days erection timeline.
     - Interactive Gantt Chart and S-Curve projection.
     - Staged Construction SVD Stability check preventing cantilever unzipping collapse.

5. **⚙️ SETTINGS (System Preferences)**
   - Design Standards: SNI 1729:2020 / AISC 360-16 LRFD, Eurocode 3, AS 4100.
   - Units: SI Metric (kN, mm, MPa) vs. US Customary (kips, in, ksi).
   - SVD Tolerance ($\varepsilon_{\text{tol}} = 10^{-7}$).
   - Google Gemini 2.5 Flash API autonomous agent integration.

---

## 🤖 Global Floating Dynamic Island AI Copilot (`⌘K`)

EigenAI Copilot floats as an ambient capsule at the bottom-center across **all 5 features**, offering workspace-aware intelligence:

* **In `MY PROJECT`**: `[📑 Portfolio Summary]`, `[📊 Rank by Safety]`, `[📤 Export Dossier PDF]`
* **In `PARAMETRIC`**: `[📐 Warren Truss 24m]`, `[🔗 Bind H = L / 7.5]`, `[🧲 Verify Constraints]`, `[💾 Save to DB]`
* **In `BUILD`**: `[⚡ Check SVD Mechanism]`, `[🎯 Screen Critical ARI]`, `[🛡️ Collapse Risk]`, `[📐 Warren Truss 24m]`
* **In `ANALYZE`**: `[⚡ SVD Stability Nullspace]`, `[💰 Optimize RAB for ARI ≥ 0.9]`, `[📅 4D Staged Erection Check]`
* **In `SETTINGS`**: `[🔑 Test Gemini API]`, `[📐 SNI 1729 / AISC LRFD]`, `[⚙️ SVD Tol (1e-7)]`

---

## 📦 Downloads & Installation

### Option 1: macOS Disk Image (Recommended)
Download the official release:
* **[EigenGraph-v1.0.0.dmg](https://github.com/haridsirawan/EigenGraph/releases/tag/v1.0.0)** (Drag-and-drop to Applications).

### Option 2: Build from Source
Ensure Rust 1.80+ is installed:

```bash
git clone https://github.com/haridsirawan/EigenGraph.git
cd EigenGraph

# Run full core mechanics test suite
cargo test

# Launch desktop app in release profile
cargo run --release
```

---

## ⌨️ Essential Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| **`⌘K`** | Toggle Floating Dynamic Island AI Copilot (Open / Minimize) |
| **`Esc`** | Minimize AI Copilot / Cancel active CAD drafting command |
| **`L` / `PL`** | Start Line / Polyline drawing command |
| **`A` / `C`** | Start Arc / Circle drawing command |
| **`DIM`** | Measure and add linear parametric dimension constraint |
| **`TR` / `O`** | Trim cutting edge / Offset curve |
| **`F8`** | Toggle Ortho Mode (Horizontal / Vertical locking) |

---

## 📄 Academic Research & Citation

If you use EigenGraph or the Adjusted Redundancy Index formulation in your academic research, please cite:

```bibtex
@phdthesis{irawan2026eigengraph,
  author       = {Harids Irawan},
  title        = {Adjusted Redundancy Index and Singular Value Decomposition Kinematic Guards for Structural Robustness and Progressive Collapse Prevention},
  school       = {Doctoral Program in Civil & Structural Engineering},
  year         = {2026},
  url          = {https://github.com/haridsirawan/EigenGraph}
}
```

---

## 🛡️ License
Dual-licensed under **MIT** or **Apache-2.0** at your option.
Developed by **Harids Irawan**.
