# EigenGraph Spatial UI Architecture Contract (North Star Specification)

To maintain consistent product direction across all future phases (`v0.4.8+`), EigenGraph enforces four strict UI/UX boundaries in **Spatial Mode**:

1. **100% Full-Viewport Earth & BIM Canvas**:
   - No permanent docked sidebars, side panels (`Panel::right` / `Panel::left`), multi-row fixed toolbars, bottom-right Earth tile watermarks (`v0.5.3`), or top-left corner chrome (`antigravity_top_left_capsule` removed in `v0.5.0`) may clutter the interactive viewport in Spatial Mode.
   - Users always start directly on the edge-to-edge 3D Earth Globe / BIM spatial viewport with unobstructed corners.

2. **Single Hover-Expandable Floating Dynamic Island (`src/views/spatial_workspace.rs`)**:
   - **Single Unified Morphing Capsule (`v0.5.2–v0.5.3`)**: Resting controls, Hover Extension controls (`Undo / Redo`, `Earth 3D / BIM 3D / Plan 2D`, `API Key`, `Settings`, `Grid`, `Map BG`, `Ortho`, `Clean / DCR% / ARI / Moment / 4D`, `Iso / Top / Front` camera presets, `Story` / `+ Story` / `+ Slab`, `Fit`, `Export`, and `More Draw`), interactive **Earth Site Location** card (`v0.5.3`), and drawers (`Vault`, `@ Context`, `/ Slash`, `History`, `Chat`) all grow seamlessly upward inside **one continuous rounded glass frame** (never stacking into two separate boxes). Mobile/touch viewports expose a compact `Deck` toggle in Row 1 so touch users can expand/collapse the island cleanly without hover.
   - **Resting State**: Displays the sleek 3-row translucent bottom-center dock (`Vault / Switch Vault / + Add Context / Model / Planning-Fast / Chat / History`, `Draw: Orbit Column Beam Wall Footing Line Rect Measure Terraform`, and Prompt Composer) while keeping the rest of the canvas unobstructed.
   - **Manual Drawing, Precision Drafting & `Esc` Orbit Return (`SpatialManualTool`)**: Direct 1-click toggles for `Orbit`, `Column`, `Beam`, `Wall`, `Footing`, `Line`, `Rect`, `Measure`, and `Terraform`, with numeric `L / deg / W / H / R` drafting inputs in the Active Tool Pill and instant `Esc` fallback back to pointer `Orbit / Select`.
   - **Slash Command Palette (`/`)**: Full command access to views, structural lenses, and on-demand engineering sheets (also triggerable from Gemini responses via `[[OPEN_SHEET:...]]`).

3. **Click-to-Inspect Smart Element Pop-Up**:
   - Element properties (`ResizeColumn`, `ResizeRun`, `ResizeWall`, `ResizeFooting`), FEM stress/utilization (`DCR%`, `ARI`, `SVD`, `N-1`), 5D/6D metrics, and element-scoped `@Element` AI prompts appear exclusively in a compact translucent pop-up when an element or Earth coordinate is clicked.

4. **On-Demand Heavy Engineering Sheets & Engine Growth**:
   - Detailed multi-table engineering workspaces (`/vault`..`/report`) open strictly as transient translucent modal sheets (`Esc` to close), keeping `/classic` as an explicit opt-in fallback.
   - Future phases (such as live Gemini API execution, async FEM/ARI tool calling, Planning diff review cards, and Vault AI thread persistence) must execute **inside the Dynamic Island and Smart Pop-Up** without introducing new permanent panels or toolbars.
