# GPUI Kit Component Suite Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate the `gpui-kit` UI component suite (`TabBar`, `Sidebar`, `Tree`, `ContextMenu`, `Editor`, and `Dock`) across `crates/aura-ui` to replace hand-crafted elements while preserving Aura's method badges, dirty tracking, and dark aesthetic.

**Architecture:** We adopt `gpui-kit = "0.7.3"` as the unified UI component foundation for `crates/aura-ui`. The root workspace window uses `gpui_kit::component::dock` to arrange the panels, `gpui_kit::component::sidebar` and `gpui_kit::component::tree` for collection hierarchies, `gpui_kit::component::tab` for open document management, `gpui_kit::component::context_menu` for right-click workflows, and `gpui_kit::component::editor` for syntax-highlighted request body and script editing.

**Tech Stack:** Rust 2021, GPUI 0.2.2, `gpui-kit` 0.7.3, Tokio 1.x, `deno_core`.

**Spec:** [`docs/superpowers/specs/2026-10-16-gpui-kit-integration-design.md`](file:///c:/repo/aura/docs/superpowers/specs/2026-10-16-gpui-kit-integration-design.md)

## Global Constraints

- Keep view file lengths under ~300–400 lines per `AGENTS.md`.
- No synchronous filesystem or blocking network I/O on the GPUI UI thread.
- Preserve method badge colors: GET `#0066cc`, POST `#28a745`, PUT/Dirty `#f5a623`, PATCH `#8e44ad`, DELETE `#e74c3c`.
- Maintain dark surface palette: Base `#1d1d1f`, Tile 1 `#272729`, Tile 2 `#2a2a2c`, Borders `#333333`.
- All pure domain models remain decoupled from GPUI for unit testability.

## Review Focus

1. **Tab Close Confirmation on Dirty Tab**: When closing a tab with unsaved edits, dirty state must be checked before dismissal.
2. **Empty Collection Tree Handling**: When a workspace has 0 collections or an empty collection, the Tree component must render the empty state gracefully without panic.
3. **HTTP Method Badge Styling in Tab and Tree**: Ensure custom badges (GET, POST, etc.) inside `gpui_kit` slots retain contrasting text and correct border radius.
4. **Dock Split Resize Boundaries**: Ensure dock pane resizing respects minimum/maximum widths (e.g., sidebar cannot collapse to 0 or expand past 80% screen width).
5. **Editor Input Sync**: Ensure text changes in `gpui_kit::component::editor` update the active request model in `AuraAppState` without cursor jumps.

---

### Task 1: Add `gpui-kit` Dependency and Initialize Runtime Assets

**Files:**
- Modify: `Cargo.toml:24-35`
- Modify: `crates/aura-ui/Cargo.toml:6-26`
- Modify: `crates/aura-ui/src/lib.rs:1-30`

**Interfaces:**
- Consumes: `gpui-kit = "0.7.3"` crate.
- Produces: `gpui_kit::init(cx)` initialization in `aura-ui`.

- [x] **Step 1: Write verification test for gpui-kit availability**
- [x] **Step 2: Run test to verify it fails**
- [x] **Step 3: Add `gpui-kit` to Cargo dependencies and initialize in `crates/aura-ui/src/lib.rs`**
- [x] **Step 4: Run test to verify it passes**
- [x] **Step 5: Commit** (Commit `953ead0`)

---

### Task 2: Refactor `TabBarView` Using `gpui_kit::component::tab::{Tab, TabBar}`

**Files:**
- Modify: `crates/aura-ui/src/views/tab_bar_view.rs:1-124`
- Modify: `crates/aura-ui/tests/tab_bar_test.rs`

**Interfaces:**
- Consumes: `gpui_kit::component::tab::{Tab, TabBar}`, `AuraAppState`, `HttpMethod`.
- Produces: `TabBarView::render(state, cx, on_select, on_close, on_new_request) -> impl IntoElement`.

- [x] **Step 1: Write test for TabBarView rendering tabs and dirty dot**
- [x] **Step 2: Run test to verify baseline data mapping**
- [x] **Step 3: Refactor `TabBarView::render` to use `gpui_kit::component::tab::{Tab, TabBar}`**
- [x] **Step 4: Verify aura-ui compiles and unit tests pass**
- [x] **Step 5: Commit** (Commit `3605451`)

---

### Task 3: Refactor `SidebarView` Using `gpui_kit::component::sidebar` and `gpui_kit::component::tree`

**Files:**
- Modify: `crates/aura-ui/src/views/sidebar_view.rs:1-550`
- Create: `crates/aura-ui/tests/tree_mapping_test.rs`

**Interfaces:**
- Consumes: `gpui_kit::component::sidebar::{Sidebar, SidebarHeader, SidebarFooter, SidebarGroup}`, `gpui_kit::component::tree::{tree, TreeState, TreeItem, TreeEntry}`.
- Produces: `SidebarView::render(...)` with tree-driven collection rendering.

- [x] **Step 1: Write test for collection tree structure mapping**
- [x] **Step 2: Run test to verify collection hierarchy**
- [x] **Step 3: Refactor `SidebarView` to use `gpui_kit::component::sidebar` and `tree()`**
- [x] **Step 4: Verify aura-ui compiles and tests pass**
- [x] **Step 5: Commit** (Commit `6bf2c5e`)

---

### Task 4: Standardize Context Menus with `gpui_kit::component::context_menu`

**Files:**
- Modify: `crates/aura-ui/src/views/sidebar_view.rs`
- Modify: `crates/aura-ui/src/views/tab_bar_view.rs`

**Interfaces:**
- Consumes: `gpui_kit::component::context_menu::{ContextMenu, ContextMenuItem}`.
- Produces: Context menu bindings for sidebar tree entries and workspace tabs.

- [x] **Step 1: Write test for context menu actions enumeration**
- [x] **Step 2: Run test to verify context actions coverage**
- [x] **Step 3: Attach `gpui_kit::component::context_menu` to Tree entries and Tabs**
- [x] **Step 4: Verify aura-ui compiles cleanly**
- [x] **Step 5: Commit** (Commit `15b9183`)

---

### Task 5: Integrate `gpui_kit::component::editor` for Request Body and Scripting

**Files:**
- Modify: `crates/aura-ui/src/views/request_editor_view.rs:1-350`
- Modify: `crates/aura-ui/src/views/response_pane_view.rs:1-350`

**Interfaces:**
- Consumes: `gpui_kit::component::editor::{Editor, EditorState}`.
- Produces: Syntax-highlighted code editing in request body / script sub-tabs, and formatted JSON/XML viewing in response pane.

- [x] **Step 1: Write test for editor language mode resolution**
- [x] **Step 2: Run test to verify language detection**
- [x] **Step 3: Integrate `gpui_kit::component::editor` into `RequestEditorView` and `ResponsePaneView`**
- [x] **Step 4: Run check to verify compilation**
- [x] **Step 5: Commit** (Commit `9a9bb81`)

---

### Task 6: Workspace Root Dock Integration (`gpui_kit::component::dock`) & Full Verification

**Files:**
- Modify: `crates/aura-ui/src/views/workspace_view.rs` (or root layout)
- Modify: `src/main.rs:1-50`

**Interfaces:**
- Consumes: `gpui_kit::component::dock::{Dock, DockArea, DockItem}`.
- Produces: Fully integrated multi-pane workspace with docking, tabs, tree, and editors.

- [x] **Step 1: Write integration test for workspace component assembly**
- [x] **Step 2: Run test to verify ratio logic**
- [x] **Step 3: Wire `gpui_kit::component::dock` in root workspace view**
- [x] **Step 4: Run full workspace test suite and clippy**
- [x] **Step 5: Commit** (Commit `e1ad6fa`)

