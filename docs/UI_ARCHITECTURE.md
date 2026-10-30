# 🏛️ EigenGraph UI Architecture & Component Hierarchy

```
+==================================================================================================+
|  TOP NAVIGATION BAR: ⚛️ EigenGraph  [MY PROJECT] [PARAMETRIC] [BUILD] [ANALYZE] [SETTINGS]  🔍 👤  |
+==================================================================================================+
|                                                                                                  |
| [1] MY PROJECT (Portfolio & App Store Homepage)                                                  |
|     ├── Hero Featured Banner (Active Project, High-Res 3D Thumbnail, ARI Score Gauge, Open CTA) |
|     ├── Category Filter Bar (All Projects, High Redundancy, Screening Required, Archived)       |
|     └── Curated 3-Column Grid (Cards with Status Pills, Element Counts, Dossier / Deck buttons)   |
|                                                                                                  |
| [2] PARAMETRIC (AutoCAD Free Drawing & Parametric Component Studio)                             |
|     ├── Top CAD Ribbon: Line (L), Polyline (PL), Arc (A), Dimension (DIM), Snap, Ortho           |
|     ├── Left Panel: Driving Variables (L, H, n, sections) & Geometric Constraint Status         |
|     ├── Center Viewport: AutoCAD Charcoal Canvas with Dynamic Crosshair & Coordinate HUD         |
|     ├── Bottom Command Bar: AutoCAD Command Line (_LINE, _TRIM, _OFFSET, math expressions)       |
|     └── Persistence Action: 💾 "Save to Database" -> Stores components for BUILD                 |
|                                                                                                  |
| [3] BUILD (3D Assembly Workspace & Floating Dynamic Island Copilot)                              |
|     ├── Left Activity Bar (Dedicated CAD/BIM Inspection):                                        |
|     │   ├── ⚙️ Properties Inspector (Section, Axial Force Ne, Capacity Re, Utilization U*)       |
|     │   ├── 📁 Project Browser + 📦 Database Component Inserter (From PARAMETRIC)                |
|     │   ├── 🛠️ Modify Tools (Move, Rotate, Subdivide, Mirror, Array)                             |
|     │   └── 📑 Display Layers (Grid, Axes, Centroid Wireframe, Node IDs, Heatmaps)               |
|     ├── 3D Canvas Viewport (wgpu Hardware-accelerated renderer, Orbit, Pan, Zoom)                |
|     └── Floating Dynamic Island AI Copilot (Option B - Antigravity style):                       |
|         ├── Resting State: Sleek Capsule Pill Bar (⌘K) at bottom center                          |
|         └── Expanded State: Rich Floating Console (SVD Rank check, ARI screening, Quick chips)    |
|                                                                                                  |
| [4] ANALYZE (Adjusted Redundancy Index & SVD Kinematic Guard)                                    |
|     ├── SVD Kinematic Guard Banner (Real-time rank(K) = n_dof - n_rigid Mechanism Alert)         |
|     ├── KPI Summary Cards (Mean ARI, Indeterminacy s, Critical Members Count, Reserve Capacity)  |
|     ├── Member Screening Table (Filterable by Redundancy Contribution Ri and Failure Risk)       |
|     └── Redundancy Contribution Distribution Chart                                               |
|                                                                                                  |
| [5] SETTINGS (macOS System Preferences)                                                          |
|     ├── Units & Coordinate System (SI / Metric vs Imperial)                                     |
|     ├── Structural Standards (SNI 1729 / AISC 360 LRFD)                                          |
|     ├── SVD Kinematic Tolerances (Singular value threshold eps_tol)                              |
|     └── Gemini API Key & Generative Agent Integration                                            |
|                                                                                                  |
+==================================================================================================+
|  BOTTOM STATUS BAR: 🟢 Model: Jembatan Busur Pelengkung | Units: kN, mm, MPa | 60 FPS Metal GPU   |
+==================================================================================================+
```
