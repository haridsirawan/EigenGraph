# ⚛️ EigenGraph

> **Desktop BIM & Structural Graph Analytics** with Mac App Store-inspired Human Interface Design.
> Developed by **Harids Irawan** (Doctoral Research in Structural Redundancy & Progressive Collapse).

---

## 🌟 Overview & Design Philosophy

**EigenGraph** breaks away from the cluttered, intimidative, and 90s-era interfaces of traditional structural CAD/BIM tools (like SAP2000, ETABS, or Revit). Instead, it adopts the **macOS App Store & Human Interface Guidelines (HIG)**:

* **Clean Card-Based Experience**: Projects, parametric components, and analytics are presented as rich, visual cards with live 3D thumbnails and Apple-style pill badges.
* **Zero-Home Portfolio**: The application opens directly into your curated project portfolio (**MY PROJECT**) with a large Hero Featured Banner.
* **AutoCAD Free Drawing Studio (PARAMETRIC)**: A full free-hand 2D/3D drafting studio (Line, Polyline, Arc, Dimensions, Constraints). Draw components freely, bind driving parametric formulas ($L, H, n$), and save them to the database to be assembled in BUILD.
* **Organizing & Assembly (BUILD)**: An interactive 3D modeling canvas with a dedicated CAD sidebar and quick access to import your saved parametric database components.
* **Option B: Floating Dynamic Island AI Copilot**: Inspired by modern AI interfaces (Google Antigravity, macOS Dynamic Island, Raycast), EigenAI floats as a sleek capsule at the bottom center of the 3D viewport. It expands seamlessly (`⌘K`) into a full structural copilot console with quick diagnostic chips.
* **Deep Mechanics Integration**: Directly integrates the **Adjusted Redundancy Index (ARI)** and **SVD Kinematic Guard** for fast, mechanics-based screening of structural safety and collapse prevention.

---

## 🏛️ The 5 Pillars Architecture

EigenGraph organizes structural engineering into 5 clear, focused pillars accessible via the persistent macOS-style segmented pill top bar:

| Pillar | Focus | Interface Model |
| :--- | :--- | :--- |
| **1. 📑 MY PROJECT** | Client portfolio, dossiers, and project gallery | Mac App Store Featured Hero Banner + 3-Column Curated Card Grid |
| **2. 📐 PARAMETRIC** | AutoCAD Free Drawing & Parametric Component Studio | 2D/3D Drafting Canvas + Driving Variables Table + "Save to Database" |
| **3. 🏗️ BUILD** | 3D structural assembly & component instantiation | 3D Canvas + Dedicated BIM Sidebar + DB Component Inserter + Floating AI Copilot (`⌘K`) |
| **4. ⚡ ANALYZE** | Redundancy analytics & kinematic collapse guard | SVD Kinematic Guard Banner + KPI Cards + Member Screening Table |
| **5. ⚙️ SETTINGS** | Preferences, units, tolerances, and integrations | macOS System Preferences Grouped Cards (SNI/AISC, SVD $\varepsilon_{tol}$, Gemini API) |

---

## 🔄 Core Workflow: Parametric Drawing to Build Assembly

1. **Draw in PARAMETRIC**:
   * Open the **PARAMETRIC** pillar.
   * Draw structural geometry freely using AutoCAD tools (`Line`, `Polyline`, `Arc`, `Trim`, `Dimension`).
   * Bind driving parametric variables (e.g. $L = 24.0\text{ m}, H = L / 7.5$).
   * Click **`💾 Save to Database`**.
2. **Assemble in BUILD**:
   * Switch to the **BUILD** pillar.
   * In the Project Browser, open **`📦 Imported Parametric Components`**.
   * Click **`+ Insert`** to assemble your custom-drawn components into the full 3D building/bridge model.
3. **Verify in ANALYZE**:
   * Switch to **ANALYZE** to run singular value decomposition (SVD) and evaluate the Adjusted Redundancy Index (ARI).
4. **Publish in MY PROJECT**:
   * Generate calculation dossiers, client decks, and portfolio cards in **MY PROJECT**.

---

## 🚀 Quickstart & Installation

```bash
git clone https://github.com/haridsirawan/EigenGraph.git
cd EigenGraph

# Run desktop native application in release mode
cargo run --release
```

### Keyboard Shortcuts
* `⌘K`: Toggle Floating Dynamic Island AI Copilot (Open / Minimize)
* `Esc`: Close or minimize AI Copilot console / Cancel CAD tool
* `L`, `PL`, `A`, `C`, `REC`, `DIM`, `TR`, `O`: Standard AutoCAD drafting tool shortcuts

---

## 📄 License
Dual-licensed under MIT OR Apache-2.0. Developed by Harids Irawan.
