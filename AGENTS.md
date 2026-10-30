# EigenGraph Agent Guidelines & Architectural Rules

## 🔒 UI SIDEBAR NAVIGATION LAYOUT LOCK (STRICT PRESERVATION)

The macOS Left Navigation Sidebar layout in `src/app.rs` is **LOCKED BY USER SPECIFICATION**.
Do NOT alter, rearrange, add tags, or reintroduce sub-headings unless explicitly instructed by the user.

### Mandatory Rules for the Left Sidebar:
1. **Header Branding**:
   - MUST display only the clean title: `⚛️ EigenGraph`.
   - Do NOT add subtitle strings underneath (e.g., no "10D BIM & Adjusted Redundancy Index").
2. **No Section Headers**:
   - Do NOT add dividing headers such as `LIBRARY`, `WORKSPACES`, or `BIM LIFECYCLE (1D-10D)`.
3. **No Badge Tags**:
   - Do NOT display pill badges like `LIB`, `1D`, `2D`, ..., `10D`.
   - Each button MUST be rendered cleanly with its emoji icon and title: `format!("{}   {}", p.icon(), p.clean_name())`.
4. **Exact Ordered List of 11 Navigation Pillars**:
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
5. **Active Project Card & User Row**:
   - Positioned cleanly underneath the 11 navigation buttons.
