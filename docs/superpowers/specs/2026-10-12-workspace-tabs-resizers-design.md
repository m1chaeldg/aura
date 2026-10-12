# Design Document: Aura Milestone 1 — Workspace, Multi-Tabs & Resizers

**Date:** 2026-10-12  
**Topic:** Interactive Workspace, Collections Loading, Multi-Tab Document Management & Resizable Dividers  
**Target:** `aura` desktop client (Rust + GPUI)

---

## 1. Intent & The Why

Aura is designed to be a high-performance, developer-ergonomic REST API client built with Rust and GPUI. While data models and background services (importers, runner, logging, deno runtime) exist, the desktop interface must be interactive, responsive, and tactile.

Milestone 1 establishes the foundational interactive user experience:
1. Launching into a desktop window where users can click UI elements.
2. Browsing and loading collections and request files directly from the local workspace folder.
3. Managing open requests via a multi-tab document bar with dirty-state tracking indicators and tab closing.
4. Smoothly adjusting pane layout using drag-to-resize dividers (with double-click 50/50 reset and horizontal/vertical layout toggle).

This milestone directly unblocks Milestone 2 (Interactive Request Editor) and Milestone 3 (Live Tokio Dispatch & Response Inspector).

---

## 2. Goals, Non-Goals & Anti-Goals

### Goals
- **Interactive UI Foundation:** Ensure all buttons, tabs, split dividers, and sidebar items respond visually and update state upon user clicks and keybindings.
- **Workspace & Collections Loader:** Detect and display collections, folders, and request files from the workspace directory in the sidebar tree.
- **Multi-Tab Document Workspace:**
  - Support multiple simultaneous open tabs.
  - Display HTTP method badge colors (GET: Blue, POST: Green, PUT: Amber, PATCH: Purple, DELETE: Rose).
  - Track unsaved dirty state via an amber dot indicator.
  - Dedicated tab close button (`✕`) with safe fallback when the active tab is closed.
  - Quick action `+` button to create an untitled request tab.
- **Responsive Resizable Divider:**
  - Drag-to-resize divider between Request and Response panes with visual hover feedback (`#0071e3` Focus Blue).
  - Double-click on the divider instantly resets the split to 50/50.
  - Toggle between Horizontal (Side-by-Side) and Vertical (Stacked) layouts.
  - Min/max clamp bounds (e.g. 15% to 85%) preventing pane collapse.

### Non-Goals
- Full live HTTP network execution (reserved for Milestone 3).
- Live execution of Deno scripts or BDD assertions in this milestone (handled in Milestone 2/3).
- Git version control operations dialogs (commits/push/pull UI) in Milestone 1.

### Anti-Goals
- Non-responsive UI or mockups that do not react to mouse input.
- Hardcoded layouts that break when resized or when changing window dimensions.
- Divergence from `DESIGN.md` Apple Near-Black aesthetic tokens.

---

## 3. Architectural Constraints & Tech Stack

- **Framework:** GPUI (`gpui` crate) adhering strictly to `DESIGN.md`.
- **Palette Tokens:**
  - Canvas / Background: `#1d1d1f`
  - Surface Tile 1: `#272729`
  - Surface Tile 2: `#2a2a2c`
  - Borders: `#333333`
  - Action Blue / Primary: `#0066cc`
  - Focus Blue: `#0071e3`
  - Badges: GET (`#0066cc`), POST (`#28a745`), PUT (`#f5a623`), PATCH (`#8e44ad`), DELETE (`#e74c3c`).
- **State Management:** `AuraAppState` in `src/ui/mod.rs` and GPUI `AuraWorkspaceView` component in `src/ui/workspace_view.rs`.

---

## 4. User-Decided Specifications

1. **Staged Milestones Approach:** Validating Milestone 1 (Workspace, Tabs, Resizers) thoroughly before moving to Editor (Milestone 2) and Engine (Milestone 3).
2. **Interactive Resizable Divider:**
   - Smooth drag resizing between the Request and Response panes.
   - Double-click to instantly snap back to 50/50.
   - Support for horizontal and vertical orientations.
3. **Workspace File Discovery:**
   - Sidebar displays requests loaded from workspace collections.
   - Clicking a sidebar request activates or opens it in a new tab.

---

## 5. Builder's Decisions (Technical Calls)

1. **Divider Drag Interaction in GPUI:**
   - Mouse down on the divider initiates a dragging state (`is_dragging_divider: bool`).
   - Mouse move updates `split_ratio` based on local coordinate deltas, clamped between `0.15` and `0.85`.
   - Double-click resets `split_ratio` to `0.50`.
2. **Active Tab Selection & Closing Behavior:**
   - If the active tab is closed, activate the tab to its left (or right if index 0). If all tabs are closed, automatically spawn an untitled request tab.
3. **Collection Tree Hierarchy:**
   - Render collections as collapsible groups with folder icons.
   - Requests display their method pill badge alongside the request title.
4. **Automated Verification Suite:**
   - Headless unit tests covering tab lifecycle, split ratio clamping, double-click reset, dirty state toggling, and collections loader integration.

---

## 6. Verification Plan

- `cargo test --test unit_ui` passes with 100% success covering all tab, divider, and layout behaviors.
- `cargo check` and `cargo test` pass across the entire workspace without warnings or regressions.
