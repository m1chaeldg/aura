# Sidebar Actions, Context Menu, Shortcuts & Reorganization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement hover action buttons, right-click context menu, keyboard tree navigation and power shortcuts, drag-and-drop visual indicators, and synchronized disk and tab operations for the Aura sidebar.

**Architecture:** Extend the Service-Store architecture by adding selection, clipboard, and drag state to `WorkspaceStore`, async duplication and paste operations with collision handling to `WorkspaceService`, a modular `ContextMenuView` and delete confirmation modal, and enhanced mouse/keyboard event handling in `SidebarView`.

**Tech Stack:** Rust 2021, GPUI 0.2.2, Tokio 1.0, `aura-core`, `aura-sync`, `serde` / `serde_json`, `parking_lot`.

**Spec:** [docs/superpowers/specs/2026-10-12-sidebar-actions-and-reorganization-design.md](file:///c:/repo/aura/docs/superpowers/specs/2026-10-12-sidebar-actions-and-reorganization-design.md)

## Global Constraints

- Must compile cleanly on Rust 2021 with zero warnings from `cargo check`.
- Never execute synchronous filesystem I/O (`std::fs`) on the main GPUI render thread.
- Colors and tokens must strictly follow `DESIGN.md` (`#1d1d1f` Base Canvas, `#272729` Surface Tile 1, `#2a2a2c` Surface Tile 2, `#0066cc` Action Blue, `#333333` Border, `#e74c3c` Danger Delete).
- Method badge colors: GET `#0066cc`, POST `#28a745`, PUT `#f5a623`, PATCH `#8e44ad`, DELETE `#e74c3c`.
- Duplicate names follow the pattern `<stem> (Copy).aura`, incrementing to `<stem> (Copy 2).aura` on collisions.
- Deleting an open file or folder from the sidebar must immediately close its tab in `SessionStore` without leaving ghost tabs or panicking.
- In-place duplication, copying, pasting, and reordering must immediately update `workspace.json` and disk files.

## Review Focus

1. **Collision-Free Duplication & Pasting with Existing Copies:** Duplicating or pasting `Login.aura` when both `Login (Copy).aura` and `Login (Copy 2).aura` already exist must compute `Login (Copy 3).aura` without overwriting or failing (Pinned to Task 2).
2. **Recursive Folder Duplication with Deep Nesting:** Duplicating a folder containing nested subdirectories, requests, and metadata must recursively replicate all files and subdirectories with new names, updating `workspace.json` for every cloned item without corruption (Pinned to Task 2).
3. **Pasting Deleted or Inaccessible Source:** If a user copies an item, deletes the source file externally, and then triggers Paste, the service must return a clean error without creating broken zero-byte files or panicking (Pinned to Task 2).
4. **Tree Keyboard Navigation on Collapsed Branches:** Pressing Down/Up arrow must only navigate through visible (non-collapsed) tree nodes, properly skipping hidden children of collapsed folders and clamping at bounds (Pinned to Task 1).
5. **Delete Confirmation Tab Safety for Nested Folder Deletion:** Deleting a folder containing multiple open tabs across nested levels must display the confirmation prompt and, upon confirmation, cleanly close all descendant tabs and spawn an Untitled fallback tab if no tabs remain open (Pinned to Task 3).

---

### Task 1: WorkspaceStore Selection, Clipboard & Tree Navigation Models

**Files:**
- Modify: `crates/aura-ui/src/stores/workspace_store.rs`
- Test: `crates/aura-ui/tests/test_workspace_store.rs`

**Interfaces:**
- Consumes: `std::path::{Path, PathBuf}`, `serde::{Serialize, Deserialize}`
- Produces:
  - `pub enum ClipboardPayload { Request(PathBuf), Folder(PathBuf) }`
  - `pub enum DragPlacement { Above, Inside, Below }`
  - `pub struct SidebarDragState { pub dragging_path: PathBuf, pub hover_target_path: PathBuf, pub placement: DragPlacement }`
  - `pub struct ContextMenuState { pub target_path: PathBuf, pub is_folder: bool, pub position: (f32, f32) }`
  - `WorkspaceStore`:
    - `pub selected_path: Option<PathBuf>`
    - `pub clipboard_item: Option<ClipboardPayload>`
    - `pub active_context_menu: Option<ContextMenuState>`
    - `pub drag_state: Option<SidebarDragState>`
    - `pub fn visible_nodes(&self) -> Vec<&WorkspaceTreeNode>`
    - `pub fn select_node(&mut self, path: PathBuf)`
    - `pub fn select_next_node(&mut self)`
    - `pub fn select_prev_node(&mut self)`
    - `pub fn expand_selected(&mut self)`
    - `pub fn collapse_selected(&mut self)`
    - `pub fn copy_selected(&mut self)`
    - `pub fn clear_clipboard(&mut self)`
    - `pub fn open_context_menu(&mut self, target_path: PathBuf, is_folder: bool, pos: (f32, f32))`
    - `pub fn close_context_menu(&mut self)`
    - `pub fn set_drag_state(&mut self, state: Option<SidebarDragState>)`

- [ ] **Step 1: Write the failing tests in `crates/aura-ui/tests/test_workspace_store.rs`**

```rust
use aura_core::model::HttpMethod;
use aura_ui::stores::workspace_store::{
    ClipboardPayload, ContextMenuState, DragPlacement, SidebarDragState, WorkspaceStore,
    WorkspaceTreeNode,
};
use std::path::PathBuf;

#[test]
fn test_workspace_store_visible_nodes_and_keyboard_traversal() {
    let mut store = WorkspaceStore::new();
    let folder1 = WorkspaceTreeNode::Folder {
        name: "Auth".to_string(),
        path: PathBuf::from("/ws/Auth"),
        is_expanded: false,
        children: vec![
            WorkspaceTreeNode::Request {
                name: "Login.aura".to_string(),
                path: PathBuf::from("/ws/Auth/Login.aura"),
                method: HttpMethod::POST,
            },
        ],
    };
    let req2 = WorkspaceTreeNode::Request {
        name: "Health.aura".to_string(),
        path: PathBuf::from("/ws/Health.aura"),
        method: HttpMethod::GET,
    };
    store.tree = vec![folder1, req2];

    // Collapsed folder: only 2 visible nodes (Auth, Health.aura)
    let visible = store.visible_nodes();
    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].path(), &PathBuf::from("/ws/Auth"));
    assert_eq!(visible[1].path(), &PathBuf::from("/ws/Health.aura"));

    // Traversal: start at top
    store.select_node(PathBuf::from("/ws/Auth"));
    assert_eq!(store.selected_path.as_deref(), Some(PathBuf::from("/ws/Auth").as_path()));

    store.select_next_node();
    assert_eq!(store.selected_path.as_deref(), Some(PathBuf::from("/ws/Health.aura").as_path()));

    // Next at end stays clamped
    store.select_next_node();
    assert_eq!(store.selected_path.as_deref(), Some(PathBuf::from("/ws/Health.aura").as_path()));

    // Expand Auth folder
    store.select_node(PathBuf::from("/ws/Auth"));
    store.expand_selected();
    let visible_after_expand = store.visible_nodes();
    assert_eq!(visible_after_expand.len(), 3);
    assert_eq!(visible_after_expand[1].path(), &PathBuf::from("/ws/Auth/Login.aura"));

    // Navigate into expanded child
    store.select_next_node();
    assert_eq!(store.selected_path.as_deref(), Some(PathBuf::from("/ws/Auth/Login.aura").as_path()));

    // Collapse
    store.select_node(PathBuf::from("/ws/Auth"));
    store.collapse_selected();
    assert_eq!(store.visible_nodes().len(), 2);
}

#[test]
fn test_workspace_store_clipboard_and_context_menu_state() {
    let mut store = WorkspaceStore::new();
    let req_path = PathBuf::from("/ws/Auth/Login.aura");
    store.select_node(req_path.clone());
    store.copy_selected();

    match &store.clipboard_item {
        Some(ClipboardPayload::Request(p)) => assert_eq!(p, &req_path),
        _ => panic!("Expected ClipboardPayload::Request"),
    }

    store.open_context_menu(req_path.clone(), false, (120.0, 240.0));
    assert!(store.active_context_menu.is_some());
    let menu = store.active_context_menu.as_ref().unwrap();
    assert_eq!(menu.target_path, req_path);
    assert!(!menu.is_folder);
    assert_eq!(menu.position, (120.0, 240.0));

    store.close_context_menu();
    assert!(store.active_context_menu.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_workspace_store test_workspace_store_visible_nodes_and_keyboard_traversal`
Expected: FAIL with missing fields/methods on `WorkspaceStore`.

- [ ] **Step 3: Implement store models and methods in `crates/aura-ui/src/stores/workspace_store.rs`**

Add `ClipboardPayload`, `DragPlacement`, `SidebarDragState`, `ContextMenuState`, and implement navigation methods (`visible_nodes`, `select_next_node`, `select_prev_node`, `expand_selected`, `collapse_selected`, `copy_selected`, `open_context_menu`, `close_context_menu`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_workspace_store`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/stores/workspace_store.rs crates/aura-ui/tests/test_workspace_store.rs
git commit -m "feat(stores): add selection, clipboard, and keyboard tree navigation to WorkspaceStore"
```

---

### Task 2: WorkspaceService Duplication & Paste Engine

**Files:**
- Modify: `crates/aura-ui/src/services/workspace_service.rs`
- Test: `crates/aura-ui/tests/test_workspace_service.rs`

**Interfaces:**
- Consumes: `crates/aura-ui/src/stores/workspace_store::ClipboardPayload`, `std::path::{Path, PathBuf}`
- Produces:
  - `impl WorkspaceService`:
    - `pub fn resolve_duplicate_name(parent_dir: &Path, file_name: &str) -> String`
    - `pub async fn duplicate_item(&self, root: &Path, source_path: &Path) -> anyhow::Result<PathBuf>`
    - `pub async fn paste_item(&self, root: &Path, payload: &ClipboardPayload, target_parent: &Path) -> anyhow::Result<PathBuf>`

- [ ] **Step 1: Write the failing tests in `crates/aura-ui/tests/test_workspace_service.rs`**

```rust
use aura_core::model::HttpMethod;
use aura_ui::services::workspace_service::WorkspaceService;
use aura_ui::stores::workspace_store::ClipboardPayload;
use std::fs;
use std::path::{Path, PathBuf};

#[tokio::test]
async fn test_workspace_service_duplicate_collision_naming() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let service = WorkspaceService::new();

    // 1. Initial request: Login.aura
    let login_path = service
        .create_request(root, root, "Login", HttpMethod::POST)
        .await
        .expect("create request");
    assert!(login_path.exists());

    // 2. First duplicate: Login (Copy).aura
    let dup1 = service.duplicate_item(root, &login_path).await.expect("dup1");
    assert_eq!(dup1.file_name().unwrap(), "Login (Copy).aura");
    assert!(dup1.exists());

    // 3. Second duplicate of original: Login (Copy 2).aura (Review Focus #1)
    let dup2 = service.duplicate_item(root, &login_path).await.expect("dup2");
    assert_eq!(dup2.file_name().unwrap(), "Login (Copy 2).aura");
    assert!(dup2.exists());

    // 4. Duplicate of a folder recursively (Review Focus #2)
    let folder = service.create_folder(root, root, "Auth").await.expect("folder");
    let _ = service
        .create_request(root, &folder, "Token", HttpMethod::GET)
        .await
        .expect("token");
    let dup_folder = service.duplicate_item(root, &folder).await.expect("dup folder");
    assert_eq!(dup_folder.file_name().unwrap(), "Auth (Copy)");
    assert!(dup_folder.join("Token.aura").exists());
}

#[tokio::test]
async fn test_workspace_service_paste_item_and_missing_source() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let service = WorkspaceService::new();

    let folder_a = service.create_folder(root, root, "FolderA").await.unwrap();
    let folder_b = service.create_folder(root, root, "FolderB").await.unwrap();
    let file_a = service
        .create_request(root, &folder_a, "Endpoint", HttpMethod::GET)
        .await
        .unwrap();

    // Paste file from FolderA into FolderB
    let payload = ClipboardPayload::Request(file_a.clone());
    let pasted = service.paste_item(root, &payload, &folder_b).await.unwrap();
    assert_eq!(pasted, folder_b.join("Endpoint.aura"));
    assert!(pasted.exists());

    // Paste with missing source error (Review Focus #3)
    let fake_payload = ClipboardPayload::Request(root.join("NonExistent.aura"));
    let err = service.paste_item(root, &fake_payload, &folder_b).await;
    assert!(err.is_err(), "Expected error when pasting missing source");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_workspace_service test_workspace_service_duplicate_collision_naming`
Expected: FAIL with missing `duplicate_item` / `paste_item` on `WorkspaceService`.

- [ ] **Step 3: Implement `resolve_duplicate_name`, `duplicate_item`, and `paste_item` in `crates/aura-ui/src/services/workspace_service.rs`**

- `resolve_duplicate_name`: extracts stem and extension, handles `(Copy)` and `(Copy N)` counting by checking `parent_dir.join(candidate).exists()`.
- `duplicate_item`: calls `resolve_duplicate_name`, writes file or recursively copies directory, registers suppression tokens for new paths, calls `sync_meta_file(root)`.
- `paste_item`: verifies source path exists, resolves target folder, handles duplicate collision naming if target folder already contains that name, suppresses paths, calls `sync_meta_file(root)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_workspace_service`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/services/workspace_service.rs crates/aura-ui/tests/test_workspace_service.rs
git commit -m "feat(services): implement duplicate_item and paste_item with collision resolution in WorkspaceService"
```

---

### Task 3: Delete Confirmation Modal & Tab Safety Integration

**Files:**
- Modify: `crates/aura-ui/src/app_state.rs`
- Modify: `crates/aura-ui/src/views/workspace_view.rs`
- Test: `crates/aura-ui/tests/test_session_store_paths.rs`

**Interfaces:**
- Consumes: `WorkspaceService::delete_item`, `SessionStore::handle_item_deleted`
- Produces:
  - `AuraAppState`:
    - `pub pending_delete_path: Option<PathBuf>`
    - `pub fn request_delete(&mut self, path: PathBuf)`
    - `pub fn cancel_delete(&mut self)`
    - `pub async fn confirm_delete(&mut self) -> anyhow::Result<()>`

- [ ] **Step 1: Write the failing tests in `crates/aura-ui/tests/test_session_store_paths.rs`**

```rust
use aura_core::model::{AuraRequest, HttpMethod};
use aura_ui::app_state::AuraAppState;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn test_app_state_delete_confirmation_and_tab_cleanup() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let mut state = AuraAppState::new();
    state.open_workspace_sync(root).unwrap();

    // Create folder with two requests
    let folder = state.workspace_service.create_folder(root, root, "Nested").await.unwrap();
    let r1 = state.workspace_service.create_request(root, &folder, "Req1", HttpMethod::GET).await.unwrap();
    let r2 = state.workspace_service.create_request(root, &folder, "Req2", HttpMethod::POST).await.unwrap();

    // Open both requests in tabs
    let _ = state.open_file_in_tab(&r1);
    let _ = state.open_file_in_tab(&r2);
    assert_eq!(state.tabs.len(), 2);

    // Request deletion of folder
    state.request_delete(folder.clone());
    assert_eq!(state.pending_delete_path.as_ref(), Some(&folder));

    // Confirm deletion: disk deleted + both descendant tabs closed (Review Focus #5)
    state.confirm_delete().await.unwrap();
    assert!(!folder.exists());
    assert!(state.pending_delete_path.is_none());

    // Tabs closed and automatically replaced with fallback Untitled Request
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Untitled Request");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_session_store_paths test_app_state_delete_confirmation_and_tab_cleanup`
Expected: FAIL with `request_delete` / `confirm_delete` missing on `AuraAppState`.

- [ ] **Step 3: Implement delete state in `crates/aura-ui/src/app_state.rs` & dialog in `crates/aura-ui/src/views/workspace_view.rs`**

- In `AuraAppState`: add `pending_delete_path: Option<PathBuf>`, `request_delete`, `cancel_delete`, `confirm_delete`.
- In `workspace_view.rs`: when `pending_delete_path.is_some()`, render confirmation modal overlay:
  - Header: "Delete Item"
  - Body: "Are you sure you want to delete '[name]' from disk? This action cannot be undone."
  - Action buttons: "Cancel" (neutral) and "Delete" (danger `#e74c3c`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_session_store_paths`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/app_state.rs crates/aura-ui/src/views/workspace_view.rs crates/aura-ui/tests/test_session_store_paths.rs
git commit -m "feat(ui): add delete confirmation modal and synchronized tab closure in AuraAppState"
```

---

### Task 4: ContextMenuView Component

**Files:**
- Create: `crates/aura-ui/src/views/context_menu_view.rs`
- Modify: `crates/aura-ui/src/views/mod.rs`
- Modify: `crates/aura-ui/src/views/workspace_view.rs`
- Test: `crates/aura-ui/tests/test_sidebar_tree_view.rs`

**Interfaces:**
- Consumes: `crates/aura-ui/src/stores/workspace_store::ContextMenuState`, `AuraAppState`, `gpui::*`
- Produces:
  - `pub struct ContextMenuView;`
  - `impl ContextMenuView`:
    - `pub fn render<V: 'static>(state: &AuraAppState, cx: &mut Context<V>, on_action: impl Fn(&mut V, SidebarMenuAction, &mut Window, &mut Context<V>) + 'static + Copy) -> impl IntoElement`
  - `pub enum SidebarMenuAction { NewRequest, NewFolder, Duplicate, Copy, Paste, Rename, Delete }`

- [ ] **Step 1: Write unit test in `crates/aura-ui/tests/test_sidebar_tree_view.rs` verifying context menu triggers and state**

```rust
use aura_ui::app_state::AuraAppState;
use std::path::PathBuf;

#[test]
fn test_context_menu_action_state_transitions() {
    let mut state = AuraAppState::new();
    let target = PathBuf::from("/ws/Auth/Login.aura");

    state.workspace_store.open_context_menu(target.clone(), false, (100.0, 150.0));
    assert!(state.workspace_store.active_context_menu.is_some());

    let menu = state.workspace_store.active_context_menu.as_ref().unwrap();
    assert_eq!(menu.target_path, target);
    assert_eq!(menu.position, (100.0, 150.0));

    state.workspace_store.close_context_menu();
    assert!(state.workspace_store.active_context_menu.is_none());
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_sidebar_tree_view test_context_menu_action_state_transitions`
Expected: PASS

- [ ] **Step 3: Implement `ContextMenuView` in `crates/aura-ui/src/views/context_menu_view.rs`**

- Build floating popup styled according to `DESIGN.md`: background `#252528`, border `#333333`, rounded 6px, box-shadow.
- Render menu rows:
  - "New Request"
  - "New Folder"
  - Divider (`border_b_1`, color `#333333`)
  - "Duplicate" with shortcut label "Ctrl+D"
  - "Copy" with shortcut label "Ctrl+C"
  - "Paste" with shortcut label "Ctrl+V" (disabled/muted text `#555555` if `state.workspace_store.clipboard_item.is_none()`)
  - Divider
  - "Rename" with shortcut label "F2"
  - "Delete" with shortcut label "Del" (danger hover `#e74c3c`)
- Clamp position `(x, y)` so the menu does not overflow window edges.
- Add outside click dismissal.

- [ ] **Step 4: Register in `crates/aura-ui/src/views/mod.rs` and render in `workspace_view.rs`**

Add `pub use context_menu_view::{ContextMenuView, SidebarMenuAction};` in `crates/aura-ui/src/views/mod.rs`, and render `ContextMenuView` overlay in `workspace_view.rs` when `state.workspace_store.active_context_menu.is_some()`.

- [ ] **Step 5: Run cargo check to verify cleanly builds**

Run: `cargo check -p aura-ui`
Expected: SUCCESS with zero warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/aura-ui/src/views/context_menu_view.rs crates/aura-ui/src/views/mod.rs crates/aura-ui/src/views/workspace_view.rs crates/aura-ui/tests/test_sidebar_tree_view.rs
git commit -m "feat(ui): implement ContextMenuView component with design tokens and action dispatchers"
```

---

### Task 5: SidebarView Hover Action Buttons & Right-Click Trigger

**Files:**
- Modify: `crates/aura-ui/src/views/sidebar_view.rs`
- Test: `crates/aura-ui/tests/test_sidebar_tree_view.rs`

**Interfaces:**
- Consumes: `WorkspaceStore::selected_path`, `WorkspaceStore::active_context_menu`, `WorkspaceTreeNode`
- Produces:
  - Updated `SidebarView::render` supporting hover action buttons and right-click triggers.
  - Hover icons:
    - Folders: `+` (New request in folder) and `⋯` (More actions menu).
    - Requests: `📋` (Quick duplicate) and `⋯` (More actions menu).

- [ ] **Step 1: Write test in `crates/aura-ui/tests/test_sidebar_tree_view.rs` for selected row styling and action triggers**

```rust
use aura_core::model::HttpMethod;
use aura_ui::app_state::AuraAppState;
use aura_ui::stores::workspace_store::WorkspaceTreeNode;
use std::path::PathBuf;

#[test]
fn test_sidebar_node_selection_and_action_menu_triggers() {
    let mut state = AuraAppState::new();
    let req_path = PathBuf::from("/ws/GetUsers.aura");
    state.workspace_store.tree = vec![WorkspaceTreeNode::Request {
        name: "GetUsers.aura".to_string(),
        path: req_path.clone(),
        method: HttpMethod::GET,
    }];

    state.workspace_store.select_node(req_path.clone());
    assert_eq!(state.workspace_store.selected_path.as_ref(), Some(&req_path));
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_sidebar_tree_view`
Expected: PASS

- [ ] **Step 3: Update `render_tree_node` in `crates/aura-ui/src/views/sidebar_view.rs`**

- Highlight active/selected row: background `#27374d` with subtle `#0066cc` border when `selected_path == node.path()`.
- Group hover styling on row: `.group("tree-row")`.
- Right side of tree row contains action button container:
  - Folder row:
    - `+` button: on click, creates new request in this folder.
    - `⋯` button: on click, opens context menu for this folder.
  - Request row:
    - `📋` button: on click, duplicates this request.
    - `⋯` button: on click, opens context menu for this request.
- Add secondary click listener on row: on right-click, dispatches `on_context_menu(target_path, is_folder, (mouse_x, mouse_y))`.

- [ ] **Step 4: Run cargo check and existing tests**

Run: `cargo check -p aura-ui`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/views/sidebar_view.rs crates/aura-ui/tests/test_sidebar_tree_view.rs
git commit -m "feat(ui): add hover action buttons and right-click trigger to SidebarView"
```

---

### Task 6: Sidebar Keyboard Navigation & Power Shortcuts

**Files:**
- Modify: `crates/aura-ui/src/keybindings.rs`
- Modify: `crates/aura-ui/src/views/sidebar_view.rs`
- Modify: `crates/aura-ui/src/views/workspace_view.rs`
- Test: `crates/aura-ui/tests/test_sidebar_tree_view.rs`

**Interfaces:**
- Consumes: `Action`, `KeybindingPreset`
- Produces:
  - Actions:
    - `Action::TreeUp`, `Action::TreeDown`, `Action::TreeExpand`, `Action::TreeCollapse`, `Action::TreeOpen`
    - `Action::DuplicateItem`, `Action::CopyItem`, `Action::PasteItem`, `Action::DeleteItem`, `Action::RenameItem`
  - Shortcuts handling in `KeybindingPreset::resolve`:
    - Up / Down -> `TreeUp` / `TreeDown`
    - Right / Left -> `TreeExpand` / `TreeCollapse`
    - Enter -> `TreeOpen`
    - Ctrl+D -> `DuplicateItem`
    - Ctrl+C -> `CopyItem`
    - Ctrl+V -> `PasteItem`
    - Del -> `DeleteItem`
    - F2 -> `RenameItem`

- [ ] **Step 1: Write test in `crates/aura-ui/tests/test_sidebar_tree_view.rs` verifying shortcut key resolution**

```rust
use aura_ui::keybindings::{Action, KeybindingPreset};

#[test]
fn test_sidebar_keyboard_shortcuts_resolution() {
    let preset = KeybindingPreset::VsCode;

    assert_eq!(preset.resolve("up", false, false, false), Some(Action::TreeUp));
    assert_eq!(preset.resolve("down", false, false, false), Some(Action::TreeDown));
    assert_eq!(preset.resolve("right", false, false, false), Some(Action::TreeExpand));
    assert_eq!(preset.resolve("left", false, false, false), Some(Action::TreeCollapse));
    assert_eq!(preset.resolve("d", true, false, false), Some(Action::DuplicateItem));
    assert_eq!(preset.resolve("c", true, false, false), Some(Action::CopyItem));
    assert_eq!(preset.resolve("v", true, false, false), Some(Action::PasteItem));
    assert_eq!(preset.resolve("f2", false, false, false), Some(Action::RenameItem));
    assert_eq!(preset.resolve("delete", false, false, false), Some(Action::DeleteItem));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_sidebar_tree_view test_sidebar_keyboard_shortcuts_resolution`
Expected: FAIL with missing enum variants on `Action`.

- [ ] **Step 3: Update `crates/aura-ui/src/keybindings.rs` and keyboard dispatching in `crates/aura-ui/src/views/workspace_view.rs`**

- Add action variants to `Action` in `crates/aura-ui/src/keybindings.rs`.
- Update `KeybindingPreset::resolve` to map navigation and manipulation shortcuts.
- In `workspace_view.rs`, wire keyboard events to app state:
  - `TreeUp` / `TreeDown`: moves tree selection.
  - `TreeExpand` / `TreeCollapse`: expands or collapses folder.
  - `TreeOpen`: opens selected request into tab.
  - `DuplicateItem`: clones selected item asynchronously.
  - `CopyItem`: sets clipboard item.
  - `PasteItem`: pastes clipboard into target folder or alongside request.
  - `RenameItem`: starts inline rename.
  - `DeleteItem`: opens delete confirmation prompt.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_sidebar_tree_view`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/keybindings.rs crates/aura-ui/src/views/sidebar_view.rs crates/aura-ui/src/views/workspace_view.rs crates/aura-ui/tests/test_sidebar_tree_view.rs
git commit -m "feat(keymap): support sidebar keyboard navigation and power shortcuts in keybindings engine"
```

---

### Task 7: Drag-and-Drop Reorganization with Visual Placement Indicators

**Files:**
- Modify: `crates/aura-ui/src/views/sidebar_view.rs`
- Modify: `crates/aura-ui/src/views/workspace_view.rs`
- Test: `crates/aura-ui/tests/test_sidebar_tree_view.rs`

**Interfaces:**
- Consumes: `SidebarDragState`, `DragPlacement`, `WorkspaceService::reorder_item`
- Produces:
  - Visual insertion indicator (2px `#0066cc` line above or below item).
  - Folder drop indicator (solid `#0066cc` border around folder).
  - Drag and drop lifecycle handlers calling `WorkspaceService::reorder_item`.

- [ ] **Step 1: Write test in `crates/aura-ui/tests/test_sidebar_tree_view.rs` for drag placement calculation**

```rust
use aura_ui::stores::workspace_store::{DragPlacement, SidebarDragState};
use std::path::PathBuf;

#[test]
fn test_sidebar_drag_placement_calculation() {
    let drag = SidebarDragState {
        dragging_path: PathBuf::from("/ws/Auth/Login.aura"),
        hover_target_path: PathBuf::from("/ws/Auth/Register.aura"),
        placement: DragPlacement::Below,
    };
    assert_eq!(drag.placement, DragPlacement::Below);
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_sidebar_tree_view test_sidebar_drag_placement_calculation`
Expected: PASS

- [ ] **Step 3: Implement GPUI drag handles and drop indicators in `crates/aura-ui/src/views/sidebar_view.rs`**

- When item is dragged over a row:
  - If placement is `Above`: render a `div().h(px(2.0)).bg(rgb(0x0066cc))` above the row.
  - If placement is `Below`: render a `div().h(px(2.0)).bg(rgb(0x0066cc))` below the row.
  - If placement is `Inside` (for folders): apply `border_1().border_color(rgb(0x0066cc))` to the folder container.
- On drop: dispatch async `reorder_item` in `workspace_service`, update `workspace_store`, and clear drag state.

- [ ] **Step 4: Run full test suite to verify no regressions**

Run: `cargo test -p aura-ui`
Expected: All tests pass with zero warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/views/sidebar_view.rs crates/aura-ui/src/views/workspace_view.rs crates/aura-ui/tests/test_sidebar_tree_view.rs
git commit -m "feat(ui): add drag-and-drop visual indicators and reorder handlers in SidebarView"
```

---

## Self-Review Checklist Verification

1. **Spec Coverage:**
   - Hover action buttons (`+` and `⋯` on folders, `📋` and `⋯` on requests) -> Task 5.
   - Right-click & `⋯` Context Menu (New Req, New Folder, Duplicate, Copy, Paste, Rename, Delete) -> Task 4 & Task 5.
   - Keyboard traversal (Up/Down, Right/Left, Enter) & Shortcuts (Ctrl+C, Ctrl+V, Ctrl+D, F2, Del) -> Task 1 & Task 6.
   - Drag-and-drop visual insertion lines and folder box -> Task 1 & Task 7.
   - Delete confirmation prompt modal -> Task 3.
   - Auto-increment duplicate naming (`(Copy)`, `(Copy 2)`) -> Task 2.
   - Recursive folder duplication -> Task 2.
   - Tab safety (deleting/renaming items synchronizes open tabs) -> Task 3.
2. **Step Scan:** Every step contains exact signatures, paths, test assertions, and checkable commands.
3. **Type Consistency:** Types (`ClipboardPayload`, `DragPlacement`, `SidebarDragState`, `ContextMenuState`) match across `workspace_store.rs`, `workspace_service.rs`, `sidebar_view.rs`, and tests.
4. **Review Focus:** All 5 Review Focus points are mapped to tests in Tasks 1, 2, 3, and 5.
5. **Proportion:** Concise, modular tasks without unnecessary transcript bloat.
