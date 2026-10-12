# Milestone 1: Workspace, Multi-Tabs & Resizers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an interactive GPUI desktop workspace in Aura supporting local collections discovery, a multi-tab document bar with dirty-state tracking, and responsive drag-to-resize pane dividers.

**Architecture:** Extend `AuraAppState` in `src/ui/mod.rs` with workspace collections state and proportional divider math. Update `AuraWorkspaceView` in `src/ui/workspace_view.rs` to render an interactive collections sidebar and bind click/drag/keyboard interactions across tabs, buttons, and dividers adhering strictly to `DESIGN.md`.

**Tech Stack:** Rust 2021, GPUI, `serde`, `uuid`, `chrono`, Tokio

**Spec:** [docs/superpowers/specs/2026-10-12-workspace-tabs-resizers-design.md](file:///c:/repo/aura/docs/superpowers/specs/2026-10-12-workspace-tabs-resizers-design.md)

## Global Constraints

- Must compile on Rust 2021 with zero compiler warnings on `cargo check`.
- Palette tokens must strictly follow `DESIGN.md` (`#1d1d1f` background, `#272729` surface, `#0066cc` action blue, `#0071e3` focus blue, `#333333` border).
- Resizable divider clamp bounds must strictly enforce `0.15` (15%) to `0.85` (85%) split ratio.
- Double-clicking the divider must always reset the split ratio to `0.50` (50/50).
- Tab close on the active tab must shift focus to an adjacent tab; closing the final tab must auto-spawn an "Untitled Request".

## Review Focus

1. **Empty Workspace Directory:** When scanning a path with no collections, `load_workspace_collections` must return `Ok(0)` without erroring or crashing.
2. **Divider Clamping Bounds:** Extreme delta inputs (e.g. -2.0 or +5.0) must clamp between 0.15 and 0.85, never resulting in NaN or negative dimensions.
3. **Closing Last Open Tab:** Closing the final tab must automatically create an "Untitled Request" so the editor is never in a null-pointer tab state.
4. **Duplicate Tab Activation:** Opening a request file that is already open in a tab must switch to that tab rather than creating a duplicate tab.
5. **Rapid Orientation Toggling:** Repeatedly switching between Horizontal and Vertical orientations must preserve the active `split_ratio` and active tab.

---

### Task 1: Workspace & Collections File Discovery State

**Files:**
- Modify: `src/ui/mod.rs:80-140`
- Test: `tests/unit_ui.rs`

**Interfaces:**
- Consumes: `crate::model::{Collection, Folder, AuraRequest, HttpMethod}`
- Produces:
  - `pub collections: Vec<Collection>` in `AuraAppState`
  - `pub fn load_workspace_collections(&mut self, dir: &std::path::Path) -> anyhow::Result<usize>`
  - `pub fn open_collection_request(&mut self, collection_id: &str, request_id: &str) -> bool`

- [ ] **Step 1: Write the failing tests in `tests/unit_ui.rs`**

```rust
#[test]
fn test_workspace_collections_loading_and_activation() {
    let mut state = AuraAppState::new();
    assert!(state.collections.is_empty());

    // Create a temporary directory with a dummy collection file
    let tmp = tempfile::tempdir().unwrap();
    let col_path = tmp.path().join("sample.collection.json");
    let col_json = serde_json::json!({
        "id": "col-1",
        "name": "Echo Test Suite",
        "auth": { "type": "Inherit" },
        "variables": {},
        "requests": [
            {
                "id": "req-echo-1",
                "name": "Live GET Echo",
                "method": "GET",
                "url": "https://postman-echo.com/get",
                "params": [],
                "headers": [],
                "variables": {},
                "body": { "type": "None" },
                "auth": { "type": "Inherit" },
                "scripts": { "pre_request": null, "post_response": null },
                "settings": { "inherit_auth": true, "verify_ssl": true, "follow_redirects": true, "max_redirects": 5, "timeout_ms": 30000 },
                "description": null
            }
        ],
        "folders": []
    });
    std::fs::write(&col_path, col_json.to_string()).unwrap();

    let loaded = state.load_workspace_collections(tmp.path()).unwrap();
    assert_eq!(loaded, 1);
    assert_eq!(state.collections.len(), 1);
    assert_eq!(state.collections[0].name, "Echo Test Suite");

    // Open request from collection
    let opened = state.open_collection_request("col-1", "req-echo-1");
    assert!(opened);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Live GET Echo");

    // Opening it again should focus the existing tab, not duplicate
    let reopened = state.open_collection_request("col-1", "req-echo-1");
    assert!(reopened);
    assert_eq!(state.tabs.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test unit_ui test_workspace_collections_loading_and_activation`  
Expected: FAIL with "no field `collections` on type `AuraAppState`" or "no method named `load_workspace_collections`"

- [ ] **Step 3: Implement collections state and loader in `src/ui/mod.rs`**

Add `pub collections: Vec<crate::model::Collection>` to `AuraAppState`, initialize in `Default`, and implement `load_workspace_collections` and `open_collection_request`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test unit_ui test_workspace_collections_loading_and_activation`  
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add src/ui/mod.rs tests/unit_ui.rs
git commit -m "feat(ui): add collections discovery and activation to AuraAppState"
```

---

### Task 2: Drag-to-Resize Divider Mechanics & Proportions

**Files:**
- Modify: `src/ui/mod.rs:225-245`
- Test: `tests/unit_ui.rs`

**Interfaces:**
- Consumes: `AuraAppState::split_ratio`
- Produces:
  - `pub is_dragging_divider: bool` in `AuraAppState`
  - `pub fn start_dragging_divider(&mut self)`
  - `pub fn stop_dragging_divider(&mut self)`
  - `pub fn drag_divider(&mut self, delta_ratio: f32)` (strictly clamped to `0.15`..=`0.85`)
  - `pub fn reset_divider(&mut self)` (snaps to `0.50`)

- [ ] **Step 1: Write the failing tests in `tests/unit_ui.rs`**

```rust
#[test]
fn test_divider_drag_clamping_and_reset() {
    let mut state = AuraAppState::new();
    assert_eq!(state.split_ratio, 0.50);
    assert!(!state.is_dragging_divider);

    state.start_dragging_divider();
    assert!(state.is_dragging_divider);

    // Drag forward within normal bounds
    state.drag_divider(0.15);
    assert!((state.split_ratio - 0.65).abs() < 1e-5);

    // Drag past upper limit (0.85)
    state.drag_divider(0.50);
    assert_eq!(state.split_ratio, 0.85);

    // Drag past lower limit (0.15)
    state.drag_divider(-0.90);
    assert_eq!(state.split_ratio, 0.15);

    // Double-click reset
    state.reset_divider();
    assert_eq!(state.split_ratio, 0.50);

    state.stop_dragging_divider();
    assert!(!state.is_dragging_divider);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test unit_ui test_divider_drag_clamping_and_reset`  
Expected: FAIL with "no field `is_dragging_divider`" or "no method named `drag_divider`"

- [ ] **Step 3: Implement divider drag methods in `src/ui/mod.rs`**

Update `set_split_ratio`, `drag_divider`, `start_dragging_divider`, and `stop_dragging_divider` in `src/ui/mod.rs` enforcing the `0.15..=0.85` bounds and `0.50` reset.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test unit_ui test_divider_drag_clamping_and_reset`  
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add src/ui/mod.rs tests/unit_ui.rs
git commit -m "feat(ui): implement divider drag clamping and double-click reset"
```

---

### Task 3: Interactive Sidebar & Proportional Pane Layout in GPUI

**Files:**
- Modify: `src/ui/workspace_view.rs:50-440`
- Test: `tests/unit_ui.rs`

**Interfaces:**
- Consumes: `AuraAppState::collections`, `AuraAppState::split_ratio`, `AuraAppState::orientation`
- Produces:
  - Sidebar tree component with collections and requests in `AuraWorkspaceView::render`
  - Proportional flex sizing (`flex_grow` or percentage weights) for Request Pane and Response Pane based on `split_ratio`
  - Interactive click handlers on tabs (`select_tab`), tab close (`close_tab`), new tab (`+`), and layout toggle

- [ ] **Step 1: Write the failing test for tab close fallback in `tests/unit_ui.rs`**

```rust
#[test]
fn test_close_tab_last_remaining_auto_spawns_untitled() {
    let mut state = AuraAppState::new();
    state.new_request();
    assert_eq!(state.tabs.len(), 1);

    // Close the only tab
    state.close_active_tab();
    // Must auto-spawn an untitled request to prevent null tab state
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Untitled Request");
    assert_eq!(state.active_tab_index, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test unit_ui test_close_tab_last_remaining_auto_spawns_untitled`  
Expected: FAIL if `state.tabs.len()` becomes 0

- [ ] **Step 3: Implement fallback in `src/ui/mod.rs` and sidebar/proportions in `src/ui/workspace_view.rs`**

- In `close_tab` (`src/ui/mod.rs`): if `self.tabs.is_empty()`, call `self.new_request()`.
- In `src/ui/workspace_view.rs`:
  - Render a collapsible sidebar on the left showing collections and requests with their method badges (`GET`, `POST`, etc.).
  - Update Request Pane and Response Pane layout to size proportionally according to `split_ratio` (e.g., using `split_pct` and `100 - split_pct` weights).
  - Add interactive event bindings (on_click for tabs, close buttons, plus button, orientation toggle, and divider reset).

- [ ] **Step 4: Run tests to verify all pass**

Run: `cargo test --test unit_ui`  
Expected: PASS with all tests passing.

- [ ] **Step 5: Commit changes**

```bash
git add src/ui/mod.rs src/ui/workspace_view.rs tests/unit_ui.rs
git commit -m "feat(ui): add sidebar collections tree and proportional pane layout in GPUI"
```

---

### Task 4: Full Workspace Integration & Regression Verification

**Files:**
- Modify: `src/main.rs:1-24`
- Test: `tests/integration_postman_echo.rs`, `tests/qa_load_stability.rs`

**Interfaces:**
- Consumes: All UI components and state
- Produces: Verified executable startup with collections discovery

- [ ] **Step 1: Verify main launcher loads current workspace collections on startup**

In `src/main.rs`, initialize the view and load any local workspace collections from `./.aura` or `./collections` if present.

- [ ] **Step 2: Run full automated test suite**

Run: `cargo test`  
Expected: 100% pass across all unit, integration, and QA tests with zero errors.

- [ ] **Step 3: Run clippy and format check**

Run: `cargo clippy -- -D warnings`  
Expected: Clean pass with zero warnings.

- [ ] **Step 4: Commit changes**

```bash
git add src/main.rs
git commit -m "chore(main): initialize workspace collections in desktop application startup"
```
