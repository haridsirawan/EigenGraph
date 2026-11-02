# EigenGraph Agent Guidelines & Architectural Rules

## 🔒 UI IMMUTABILITY & SIDEBAR NAVIGATION LAYOUT LOCK (STRICT PRESERVATION)

The entire Application Shell (macOS Titlebar & Left Navigation Sidebar in `src/app.rs`) is **PERMANENTLY LOCKED BY USER SPECIFICATION**.
Do NOT alter, rearrange, add badge tags, or reintroduce sub-headings under ANY circumstances.

### 🛡️ Strict Immutability Protocol:
1. **Scope of General Instructions**:
   - Broad prompts such as *"tingkatkan semua"*, *"tingkatkan"*, or *"perbaiki"* are strictly confined to:
     - Core numerical algorithms (`src/core/`)
     - Solver performance and GPU compute pipelines
     - Unit tests and verification benchmarks
     - Packaging and CI/CD pipelines
   - The agent is **FORBIDDEN** from modifying the Left Sidebar layout, headers, button styling, or titlebar during broad/general prompts.
2. **Explicit Override Keyword**:
   - Any modification to the Left Sidebar or App Chrome layout requires an explicit directive starting with `UBAH_UI:` or `UBAH_SIDEBAR:`. Without this prefix, the UI must remain 100% untouched.

### Mandatory Rules for the Left Sidebar (macOS App Store Style):
1. **Full-Height Geometry & Window Controls**:
   - The sidebar must run uninterrupted from the very top of the window to the bottom (`y = 0` to window height). It must never be truncated or cut off by top panels.
   - macOS Traffic Lights (`🔴  🟡  🟢`) sit inside the sidebar header.
2. **Header & Search Bar (No Logo)**:
   - Do NOT display any application logo or "EigenGraph" brand header at the top of the sidebar.
   - A native pill search bar (`🔍 Search`) sits directly beneath the traffic lights to filter navigation pillars.
3. **No Section Headers & No Badge Tags**:
   - Do NOT add dividing headers such as `LIBRARY`, `WORKSPACES`, `LIFECYCLE DIMENSIONS`, or `ANALYTICS & EXECUTIVE`.
   - Do NOT display pill badges like `LIB`, `1D`, `2D`, ..., `10D`.
   - Each button MUST be rendered cleanly with its emoji icon and title: `format!("{}   {}", p.icon(), p.clean_name())`, styled with subtle rounded pill selection.
4. **Exact Ordered List of 13 Navigation Pillars**:
   1. `📁   My Library`
   2. `💡   Research & Concept`
   3. `📐   Drafting & Drawings`
   4. `🏗️   Modeling & Geometry`
   5. `⏱️   Time & Scheduling`
   6. `💰   Cost & Budgeting`
   7. `🌱   Sustainability & Energy`
   8. `🏢   Facility Management`
   9. `🛡️   Safety/Health & Safety`
   10. `♻️   Lean Construction`
   11. `🤖   Industrialized & Automation`
   12. `🔮   Prediction`
   13. `📊   Results`
5. **Bottom Profile & Settings Row**:
   - Pinned at the bottom: User Profile (`👤 Harids Irawan`) with Settings gear (`⚙️`) to open System Preferences (⌘,).
   - Quick triggers for `✨ AI Copilot` (⌘K) and `🚀 Update`.
6. **Minimalist Workspace (No Strip Bars)**:
   - Do NOT add secondary strip bars, breadcrumbs, or horizontal toolbars across the workspace canvas.
   - Clean view heading with subtle dividing separator line.
7. **Window Icon**:
   - Window icon is hard-embedded from `assets/icon.png` via `eframe::icon_data::from_png_bytes`.
   - macOS application bundle icon is `assets/AppIcon.icns`.
