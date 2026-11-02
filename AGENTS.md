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

### Mandatory Rules for the Left Sidebar:
1. **Header Branding**:
   - Do NOT add subtitle strings underneath (no "10D BIM", no dimension labels).
2. **No Section Headers**:
   - Do NOT add dividing headers such as `LIBRARY`, `WORKSPACES`, `LIFECYCLE DIMENSIONS`, or `ANALYTICS & EXECUTIVE`.
3. **No Badge Tags**:
   - Do NOT display pill badges like `LIB`, `1D`, `2D`, ..., `10D`.
   - Each button MUST be rendered cleanly with its emoji icon and title: `format!("{}   {}", p.icon(), p.clean_name())`.
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
5. **Footer Branding**:
   - Displays clean version & architecture (e.g. `EigenGraph v1.1.0 (Universal 2)` or `(Intel x86_64)`).
   - Single clean update button (`🚀 Update`).
6. **Window Icon**:
   - Window icon is hard-embedded from `assets/icon.png` via `eframe::icon_data::from_png_bytes`.
   - macOS application bundle icon is `assets/AppIcon.icns`.
