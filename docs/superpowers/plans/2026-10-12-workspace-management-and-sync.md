# Workspace Management and Two-Way Synchronization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a robust, non-blocking workspace management and two-way filesystem synchronization system with hierarchical sidebar navigation, native folder picker, drag-and-drop ordering, and automatic tab synchronization for Aura.

**Architecture:** Implement pure domain data models and disk reconciliation in `crates/aura-sync` (`workspace_meta.rs`), reactive GPUI models in `crates/aura-ui/src/stores` (`WorkspaceStore`, `SessionStore`), asynchronous background services with `notify` file watching and `rfd` dialogs in `crates/aura-ui/src/services` (`WorkspaceService`), and modular UI views with drag-and-drop and inline renaming in `crates/aura-ui/src/views` (`SidebarView`).

**Tech Stack:** Rust 2021, GPUI 0.2.2, Tokio 1.0, `notify` 6.1, `rfd` 0.15, `dirs` 5.0, `serde` / `serde_json` 1.0, `parking_lot` 0.12.

**Spec:** [docs/superpowers/specs/2026-10-12-workspace-management-and-sync-design.md](file:///c:/repo/aura/docs/superpowers/specs/2026-10-12-workspace-management-and-sync-design.md)

## Global Constraints

- Must compile on Rust 2021 with zero compiler warnings on `cargo check`.
- Never run synchronous disk I/O (`std::fs`) on the main GPUI render thread.
- Palette tokens must strictly follow `DESIGN.md` (`#1d1d1f` background, `#272729` Surface Tile 1, `#2a2a2c` Surface Tile 2, `#0066cc` Action Blue, `#333333` Border).
- Method badges must use exact colors: GET `#0066cc`, POST `#28a745`, PUT `#f5a623`, PATCH `#8e44ad`, DELETE `#e74c3c`.
- `workspace.json` schema must strictly match `version: 1`, `name: String`, and recursive `order` array with `type: "folder"` and `type: "request"`.
- Requests on disk are identified by `.aura` file extension.
- External filesystem changes must update the sidebar tree in real-time without blocking UI frames.
- Deleting an open file (in UI or externally) must immediately close its tab and reset the active tab without panic.

## Review Focus

1. **Empty or Non-Existent Directory:** Scanning or opening a non-existent or empty folder must initialize a valid `workspace.json` without failing or panicking (Pinned to Task 1).
2. **Duplicate Tab Activation:** Opening a request file already open in an existing tab must switch to that tab instead of creating a duplicate tab (Pinned to Task 3).
3. **Sole Open Tab Deletion:** Deleting the file backing the only open tab must close the tab and immediately auto-spawn an "Untitled Request" so the session is never empty (Pinned to Task 3).
4. **Folder Rename with Open Descendants:** Renaming a folder containing multiple open tabs must update the file paths of all descendant tabs while keeping dirty status and tab selection intact (Pinned to Task 3).
5. **Watcher Echo Suppression:** In-app file operations (create, rename, move, delete) must register short-lived suppression tokens so the file watcher ignores its own writes and avoids redundant re-sync cycles (Pinned to Task 5).

---

### Task 1: Workspace Metadata Model & Disk Reconciliation (`aura-sync`)

**Files:**
- Create: `crates/aura-sync/src/workspace_meta.rs`
- Modify: `crates/aura-sync/src/lib.rs:1-15`
- Test: `crates/aura-sync/tests/test_workspace_meta.rs`

**Interfaces:**
- Consumes: `serde::{Serialize, Deserialize}`, `std::path::{Path, PathBuf}`
- Produces:
  - `pub struct WorkspaceMetadata { pub version: u32, pub name: String, pub order: Vec<WorkspaceOrderItem> }`
  - `pub enum WorkspaceOrderItem { Folder { name: String, order: Vec<WorkspaceOrderItem> }, Request { name: String } }`
  - `impl WorkspaceMetadata`:
    - `pub fn new(name: impl Into<String>) -> Self`
    - `pub fn reconcile_with_disk(root: &Path, existing: Option<WorkspaceMetadata>) -> anyhow::Result<Self>`
    - `pub fn load_from_disk(root: &Path) -> anyhow::Result<Self>`
    - `pub fn save_to_disk(&self, root: &Path) -> anyhow::Result<PathBuf>`
    - `pub fn reorder_item(&mut self, parent_rel_path: &Path, from_index: usize, to_index: usize) -> anyhow::Result<()>`
    - `pub fn move_item(&mut self, source_rel_path: &Path, target_parent_rel_path: &Path, insert_index: usize) -> anyhow::Result<()>`
    - `pub fn rename_item(&mut self, rel_path: &Path, new_name: &str) -> anyhow::Result<()>`
    - `pub fn remove_item(&mut self, rel_path: &Path) -> anyhow::Result<()>`

- [ ] **Step 1: Write the failing tests in `crates/aura-sync/tests/test_workspace_meta.rs`**

```rust
use aura_sync::workspace_meta::{WorkspaceMetadata, WorkspaceOrderItem};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_workspace_metadata_serde_schema() {
    let json_sample = r#"{
      "version": 1,
      "name": "My Workspace",
      "order": [
        {
          "type": "folder",
          "name": "Auth",
          "order": [
            { "type": "request", "name": "Login.aura" },
            { "type": "request", "name": "Refresh.aura" }
          ]
        },
        {
          "type": "folder",
          "name": "Users",
          "order": [
            { "type": "request", "name": "Get Profile.aura" }
          ]
        },
        { "type": "request", "name": "Health Check.aura" }
      ]
    }"#;

    let meta: WorkspaceMetadata = serde_json::from_str(json_sample).expect("valid parse");
    assert_eq!(meta.version, 1);
    assert_eq!(meta.name, "My Workspace");
    assert_eq!(meta.order.len(), 3);

    let serialized = serde_json::to_string_pretty(&meta).expect("serialize");
    let back: WorkspaceMetadata = serde_json::from_str(&serialized).expect("deserialize back");
    assert_eq!(meta, back);
}

#[test]
fn test_reconcile_with_disk_discovers_new_files_and_cleans_orphans() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    // Create folders and files
    fs::create_dir_all(root.join("Auth")).unwrap();
    fs::write(root.join("Auth").join("Login.aura"), "").unwrap();
    fs::write(root.join("Health.aura"), "").unwrap();

    // Initial reconcile with no existing metadata
    let meta = WorkspaceMetadata::reconcile_with_disk(root, None).expect("reconcile");
    assert_eq!(meta.version, 1);
    assert_eq!(meta.order.len(), 2);

    // Save to disk
    meta.save_to_disk(root).expect("save");
    assert!(root.join("workspace.json").exists());

    // External change: add Users/Profile.aura and delete Health.aura
    fs::create_dir_all(root.join("Users")).unwrap();
    fs::write(root.join("Users").join("Profile.aura"), "").unwrap();
    fs::remove_file(root.join("Health.aura")).unwrap();

    // Reconcile again with existing metadata
    let updated = WorkspaceMetadata::reconcile_with_disk(root, Some(meta)).expect("reconcile update");
    // Health.aura should be gone; Users should be added; Auth should remain
    let has_health = updated.order.iter().any(|item| matches!(item, WorkspaceOrderItem::Request { name } if name == "Health.aura"));
    let has_users = updated.order.iter().any(|item| matches!(item, WorkspaceOrderItem::Folder { name, .. } if name == "Users"));
    assert!(!has_health);
    assert!(has_users);
}

#[test]
fn test_reconcile_empty_or_nonexistent_directory_does_not_crash() {
    // Review Focus #1
    let tmp = tempdir().expect("tempdir");
    let missing_path = tmp.path().join("does_not_exist");
    let meta = WorkspaceMetadata::reconcile_with_disk(&missing_path, None).expect("reconcile missing");
    assert_eq!(meta.version, 1);
    assert!(meta.order.is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-sync --test test_workspace_meta`
Expected: FAIL with "unresolved import `aura_sync::workspace_meta`"

- [ ] **Step 3: Implement `workspace_meta.rs` in `crates/aura-sync`**

1. Define `WorkspaceMetadata` and `WorkspaceOrderItem` with `#[serde(tag = "type", rename_all = "lowercase")]`.
2. Implement recursive disk scanner scanning for directories (excluding `.git`, `node_modules`, `.aura`) and files ending with `.aura`.
3. Implement `reconcile_with_disk` preserving prior items from `order`, appending newly discovered items, and pruning missing items.
4. Implement `load_from_disk` and `save_to_disk` for `workspace.json`.
5. Implement in-memory reordering, moving, renaming, and removal methods.
6. Re-export in `crates/aura-sync/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-sync --test test_workspace_meta`
Expected: PASS with 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/aura-sync/src/workspace_meta.rs crates/aura-sync/src/lib.rs crates/aura-sync/tests/test_workspace_meta.rs
git commit -m "feat(sync): add workspace metadata schema and disk reconciliation"
```

---

### Task 2: Application Preferences Persistence (`AuraPreferences`)

**Files:**
- Create: `crates/aura-ui/src/preferences.rs`
- Modify: `crates/aura-ui/Cargo.toml:10-25` (add `dirs = "5"`)
- Modify: `crates/aura-ui/src/lib.rs:1-20`
- Test: `crates/aura-ui/tests/test_preferences.rs`

**Interfaces:**
- Consumes: `dirs::config_dir()`, `std::path::{Path, PathBuf}`
- Produces:
  - `pub struct AuraPreferences { pub last_workspace_path: Option<PathBuf> }`
  - `impl AuraPreferences`:
    - `pub fn default() -> Self`
    - `pub fn load() -> Self`
    - `pub fn save(&self) -> anyhow::Result<PathBuf>`
    - `pub fn load_from_file(path: &Path) -> Self`
    - `pub fn save_to_file(&self, path: &Path) -> anyhow::Result<()>`

- [ ] **Step 1: Write the failing tests in `crates/aura-ui/tests/test_preferences.rs`**

```rust
use aura_ui::preferences::AuraPreferences;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_preferences_save_and_load_cycle() {
    let tmp = tempdir().expect("tempdir");
    let pref_file = tmp.path().join("preferences.json");

    // Missing file returns default
    let default_prefs = AuraPreferences::load_from_file(&pref_file);
    assert_eq!(default_prefs.last_workspace_path, None);

    // Save custom path
    let mut prefs = AuraPreferences::default();
    prefs.last_workspace_path = Some(PathBuf::from("/custom/workspace/path"));
    prefs.save_to_file(&pref_file).expect("save");

    // Load back
    let loaded = AuraPreferences::load_from_file(&pref_file);
    assert_eq!(loaded.last_workspace_path, Some(PathBuf::from("/custom/workspace/path")));
}

#[test]
fn test_corrupted_preferences_recovers_to_default() {
    let tmp = tempdir().expect("tempdir");
    let pref_file = tmp.path().join("preferences.json");
    fs::write(&pref_file, "{ invalid json content").unwrap();

    let loaded = AuraPreferences::load_from_file(&pref_file);
    assert_eq!(loaded.last_workspace_path, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_preferences`
Expected: FAIL with "unresolved import `aura_ui::preferences`"

- [ ] **Step 3: Implement `AuraPreferences` in `crates/aura-ui/src/preferences.rs`**

1. Add `dirs = "5"` to `crates/aura-ui/Cargo.toml`.
2. Implement `AuraPreferences` with `serde::{Serialize, Deserialize}`.
3. In `load()`, resolve `dirs::config_dir().map(|p| p.join("aura").join("preferences.json"))` and call `load_from_file()`.
4. In `save()`, ensure parent directory exists via `std::fs::create_dir_all` before writing formatted JSON.
5. In `load_from_file()`, return `Self::default()` gracefully on missing file or parse errors.
6. Export `pub mod preferences;` in `crates/aura-ui/src/lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_preferences`
Expected: PASS with 2 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/Cargo.toml crates/aura-ui/src/preferences.rs crates/aura-ui/src/lib.rs crates/aura-ui/tests/test_preferences.rs
git commit -m "feat(ui): add application preferences persistence for last active workspace"
```

---

### Task 3: SessionStore Tab Path & File Synchronization

**Files:**
- Modify: `crates/aura-ui/src/stores/session_store.rs:8-155`
- Test: `crates/aura-ui/tests/test_session_store_paths.rs`

**Interfaces:**
- Consumes: `AuraRequest`, `Path`, `PathBuf`
- Produces:
  - `pub file_path: Option<PathBuf>` on `WorkspaceTab`
  - `impl SessionStore`:
    - `pub fn open_request_file(&mut self, request: AuraRequest, path: PathBuf)`
    - `pub fn handle_item_renamed(&mut self, old_path: &Path, new_path: &Path, new_name: &str)`
    - `pub fn handle_item_deleted(&mut self, deleted_path: &Path)`

- [ ] **Step 1: Write the failing tests in `crates/aura-ui/tests/test_session_store_paths.rs`**

```rust
use aura_core::model::{AuraRequest, HttpMethod};
use aura_ui::stores::SessionStore;
use std::path::PathBuf;

#[test]
fn test_open_request_file_prevents_duplicates() {
    // Review Focus #2
    let mut store = SessionStore::new();
    let req1 = AuraRequest::new("Login", HttpMethod::POST, "https://api.example.com/login");
    let path = PathBuf::from("/workspace/Auth/Login.aura");

    store.open_request_file(req1.clone(), path.clone());
    assert_eq!(store.tabs.len(), 1);
    assert_eq!(store.active_tab_index, 0);

    // Opening same path again must activate existing tab, not duplicate
    store.new_request(); // opens second tab
    assert_eq!(store.tabs.len(), 2);
    assert_eq!(store.active_tab_index, 1);

    store.open_request_file(req1, path);
    assert_eq!(store.tabs.len(), 2); // No new tab added
    assert_eq!(store.active_tab_index, 0); // Jumped back to tab 0
}

#[test]
fn test_handle_item_renamed_updates_tab_title_and_path() {
    let mut store = SessionStore::new();
    let req = AuraRequest::new("Login", HttpMethod::POST, "https://api.example.com/login");
    let old_path = PathBuf::from("/workspace/Auth/Login.aura");
    let new_path = PathBuf::from("/workspace/Auth/SignIn.aura");

    store.open_request_file(req, old_path.clone());
    store.handle_item_renamed(&old_path, &new_path, "SignIn.aura");

    assert_eq!(store.tabs[0].title, "SignIn.aura");
    assert_eq!(store.tabs[0].file_path, Some(new_path));
}

#[test]
fn test_handle_folder_renamed_updates_descendant_tab_paths() {
    // Review Focus #4
    let mut store = SessionStore::new();
    let req = AuraRequest::new("Login", HttpMethod::POST, "https://api.example.com/login");
    let old_folder = PathBuf::from("/workspace/Auth");
    let new_folder = PathBuf::from("/workspace/Authentication");
    let file_path = old_folder.join("Login.aura");

    store.open_request_file(req, file_path);
    store.handle_item_renamed(&old_folder, &new_folder, "Authentication");

    assert_eq!(store.tabs[0].file_path, Some(new_folder.join("Login.aura")));
}

#[test]
fn test_handle_item_deleted_closes_tab_and_spawns_fallback_if_empty() {
    // Review Focus #3
    let mut store = SessionStore::new();
    let req = AuraRequest::new("Login", HttpMethod::POST, "https://api.example.com/login");
    let path = PathBuf::from("/workspace/Auth/Login.aura");

    store.open_request_file(req, path.clone());
    assert_eq!(store.tabs.len(), 1);

    store.handle_item_deleted(&path);
    // Sole tab closed -> auto-spawns Untitled Request
    assert_eq!(store.tabs.len(), 1);
    assert_eq!(store.tabs[0].title, "Untitled Request");
    assert_eq!(store.tabs[0].file_path, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_session_store_paths`
Expected: FAIL with "no method named `open_request_file`"

- [ ] **Step 3: Implement tab synchronization methods in `session_store.rs`**

1. Add `pub file_path: Option<PathBuf>` to `WorkspaceTab`.
2. Update `WorkspaceTab::new` to initialize `file_path: None`.
3. Implement `SessionStore::open_request_file(&mut self, request: AuraRequest, path: PathBuf)`:
   - Check if any tab has `tab.file_path == Some(path)`. If so, activate it and return.
   - Otherwise, create `WorkspaceTab` with `file_path: Some(path)` and push to `self.tabs`.
4. Implement `SessionStore::handle_item_renamed`:
   - For file match: update `title` and `file_path`.
   - For directory match: check `path.starts_with(old_path)`, replace prefix with `new_path`, and update `file_path`.
5. Implement `SessionStore::handle_item_deleted`:
   - Collect IDs of tabs where `tab.file_path == Some(deleted_path)` or `tab.file_path.starts_with(deleted_path)`.
   - For each matching ID, call `self.close_tab(&id)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_session_store_paths`
Expected: PASS with 4 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/stores/session_store.rs crates/aura-ui/tests/test_session_store_paths.rs
git commit -m "feat(ui): add tab file path tracking, duplicate detection, and deletion handling"
```

---

### Task 4: WorkspaceStore Virtual Tree & Pure State Mutations

**Files:**
- Modify: `crates/aura-ui/src/stores/workspace_store.rs:1-92`
- Test: `crates/aura-ui/tests/test_workspace_store.rs`

**Interfaces:**
- Consumes: `WorkspaceMetadata`, `WorkspaceOrderItem`, `HttpMethod`
- Produces:
  - `pub enum WorkspaceTreeNode`:
    - `Folder { name: String, path: PathBuf, is_expanded: bool, children: Vec<WorkspaceTreeNode> }`
    - `Request { name: String, path: PathBuf, method: HttpMethod }`
  - `impl WorkspaceStore`:
    - `pub fn set_workspace(&mut self, root: PathBuf, metadata: WorkspaceMetadata, tree: Vec<WorkspaceTreeNode>)`
    - `pub fn apply_external_sync(&mut self, metadata: WorkspaceMetadata, tree: Vec<WorkspaceTreeNode>)`
    - `pub fn toggle_folder_expanded(&mut self, path: &Path)`
    - `pub fn apply_rename(&mut self, old_path: &Path, new_path: &Path, new_name: &str)`
    - `pub fn apply_delete(&mut self, target_path: &Path)`
    - `pub fn apply_reorder(&mut self, source_path: &Path, target_parent: &Path, insert_index: usize)`

- [ ] **Step 1: Write the failing tests in `crates/aura-ui/tests/test_workspace_store.rs`**

```rust
use aura_core::model::HttpMethod;
use aura_sync::workspace_meta::WorkspaceMetadata;
use aura_ui::stores::workspace_store::{WorkspaceStore, WorkspaceTreeNode};
use std::path::PathBuf;

#[test]
fn test_workspace_store_tree_expansion_and_mutations() {
    let mut store = WorkspaceStore::new();
    let root = PathBuf::from("/workspace");
    let folder_path = root.join("Auth");
    let req_path = folder_path.join("Login.aura");

    let tree = vec![WorkspaceTreeNode::Folder {
        name: "Auth".to_string(),
        path: folder_path.clone(),
        is_expanded: false,
        children: vec![WorkspaceTreeNode::Request {
            name: "Login.aura".to_string(),
            path: req_path.clone(),
            method: HttpMethod::POST,
        }],
    }];

    store.set_workspace(root.clone(), WorkspaceMetadata::new("Test Workspace"), tree);
    assert_eq!(store.tree.len(), 1);

    // Toggle expand
    store.toggle_folder_expanded(&folder_path);
    if let WorkspaceTreeNode::Folder { is_expanded, .. } = &store.tree[0] {
        assert!(*is_expanded);
    } else {
        panic!("expected folder");
    }

    // Rename
    let new_req_path = folder_path.join("SignIn.aura");
    store.apply_rename(&req_path, &new_req_path, "SignIn.aura");
    if let WorkspaceTreeNode::Folder { children, .. } = &store.tree[0] {
        assert_eq!(children[0].name(), "SignIn.aura");
        assert_eq!(children[0].path(), new_req_path);
    }

    // Delete
    store.apply_delete(&new_req_path);
    if let WorkspaceTreeNode::Folder { children, .. } = &store.tree[0] {
        assert!(children.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_workspace_store`
Expected: FAIL with "unresolved import `WorkspaceTreeNode`"

- [ ] **Step 3: Implement `WorkspaceTreeNode` and pure tree mutations in `workspace_store.rs`**

1. Define `WorkspaceTreeNode` enum with helper methods `path(&self) -> &Path` and `name(&self) -> &str`.
2. Add `pub tree: Vec<WorkspaceTreeNode>` and `pub metadata: Option<WorkspaceMetadata>` to `WorkspaceStore`.
3. Implement `toggle_folder_expanded` recursively finding folder by path.
4. Implement `apply_rename` updating node name and recursively updating descendant paths for renamed folders.
5. Implement `apply_delete` removing matching node from root or child list.
6. Implement `apply_reorder` moving child node to target container index.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_workspace_store`
Expected: PASS with 1 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/stores/workspace_store.rs crates/aura-ui/tests/test_workspace_store.rs
git commit -m "feat(ui): add hierarchical workspace tree and deterministic mutation methods"
```

---

### Task 5: WorkspaceService Async Operations & Watcher Engine

**Files:**
- Modify: `crates/aura-ui/Cargo.toml:10-25` (add `notify = "6.1"`, `rfd = "0.15"`)
- Modify: `crates/aura-ui/src/services/workspace_service.rs:1-31`
- Test: `crates/aura-ui/tests/test_workspace_service.rs`

**Interfaces:**
- Consumes: `WorkspaceMetadata`, `WorkspaceTreeNode`, `notify::Watcher`, `rfd::AsyncFileDialog`
- Produces:
  - `pub enum WorkspaceDiskEvent { Created(PathBuf), Modified(PathBuf), Deleted(PathBuf), Renamed { from: PathBuf, to: PathBuf } }`
  - `impl WorkspaceService`:
    - `pub fn new() -> Self`
    - `pub async fn pick_folder() -> Option<PathBuf>`
    - `pub async fn create_request(&self, root: &Path, parent_dir: &Path, name: &str, method: HttpMethod) -> anyhow::Result<PathBuf>`
    - `pub async fn create_folder(&self, root: &Path, parent_dir: &Path, name: &str) -> anyhow::Result<PathBuf>`
    - `pub async fn rename_item(&self, root: &Path, old_path: &Path, new_name: &str) -> anyhow::Result<PathBuf>`
    - `pub async fn delete_item(&self, root: &Path, target_path: &Path) -> anyhow::Result<()>`
    - `pub async fn reorder_item(&self, root: &Path, source_path: &Path, target_parent: &Path, insert_index: usize) -> anyhow::Result<()>`
    - `pub fn start_watcher(&self, root: &Path) -> anyhow::Result<tokio::sync::mpsc::UnboundedReceiver<WorkspaceDiskEvent>>`
    - `pub fn suppress_path(&self, path: &Path)`

- [ ] **Step 1: Write the failing tests in `crates/aura-ui/tests/test_workspace_service.rs`**

```rust
use aura_core::model::HttpMethod;
use aura_ui::services::workspace_service::WorkspaceService;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_workspace_service_async_crud_and_metadata_sync() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let service = WorkspaceService::new();

    // Create folder
    let folder_path = service.create_folder(root, root, "Auth").await.expect("create folder");
    assert!(folder_path.is_dir());

    // Create request
    let req_path = service.create_request(root, &folder_path, "Login", HttpMethod::POST).await.expect("create request");
    assert!(req_path.is_file());
    assert!(req_path.to_string_lossy().ends_with("Login.aura"));

    // Verify workspace.json was updated
    let meta_content = fs::read_to_string(root.join("workspace.json")).expect("read meta");
    assert!(meta_content.contains("Login.aura"));

    // Rename request
    let renamed_path = service.rename_item(root, &req_path, "SignIn.aura").await.expect("rename");
    assert!(renamed_path.exists());
    assert!(!req_path.exists());

    // Delete request
    service.delete_item(root, &renamed_path).await.expect("delete");
    assert!(!renamed_path.exists());
}

#[tokio::test]
async fn test_watcher_echo_suppression() {
    // Review Focus #5
    let service = WorkspaceService::new();
    let path = std::path::PathBuf::from("/workspace/Auth/Login.aura");

    service.suppress_path(&path);
    assert!(service.is_suppressed(&path));
    // Check suppression clears after query or timeout
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_workspace_service`
Expected: FAIL with "no method named `create_folder`"

- [ ] **Step 3: Implement `WorkspaceService` async operations and watcher**

1. Add `notify = "6.1"` and `rfd = "0.15"` to `crates/aura-ui/Cargo.toml`.
2. Add `suppression_tokens: Arc<parking_lot::Mutex<std::collections::HashSet<PathBuf>>>` to `WorkspaceService`.
3. Implement `create_request`: write default `.aura` TOML file, register path in suppression set, update `workspace.json`.
4. Implement `create_folder`: create directory via `tokio::fs::create_dir_all`, register suppression token, update `workspace.json`.
5. Implement `rename_item`: rename on disk via `tokio::fs::rename`, register old and new paths in suppression set, update `workspace.json`.
6. Implement `delete_item`: remove file/directory, register suppression token, remove from `workspace.json`.
7. Implement `start_watcher`: spawn Tokio background task with `notify::RecommendedWatcher`, debounce disk events across 150ms, filter out suppressed paths, and send `WorkspaceDiskEvent` across mpsc channel.
8. Implement `pick_folder`: wrap `rfd::AsyncFileDialog::new().pick_folder().await`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_workspace_service`
Expected: PASS with 2 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/Cargo.toml crates/aura-ui/src/services/workspace_service.rs crates/aura-ui/tests/test_workspace_service.rs
git commit -m "feat(ui): add async workspace file operations, file watcher, and echo suppression"
```

---

### Task 6: SidebarView Tree Rendering, Drag-and-Drop & Context Actions

**Files:**
- Modify: `crates/aura-ui/src/views/sidebar_view.rs:1-167`
- Modify: `crates/aura-ui/src/app_state.rs:20-60`
- Test: `crates/aura-ui/tests/test_sidebar_tree_view.rs`

**Interfaces:**
- Consumes: `WorkspaceStore`, `WorkspaceTreeNode`, `HttpMethod`, GPUI `div`, `rgb`
- Produces:
  - `pub struct DragItemPayload { pub path: PathBuf, pub is_folder: bool }`
  - `pub struct InlineRenameState { pub path: PathBuf, pub current_value: String }`
  - `SidebarView::render(...)` with:
    - Recursive folder hierarchy with chevron expansion toggle
    - HTTP method badges in DESIGN.md colors
    - Drag-and-drop 1px Action Blue insertion line and folder drop hover
    - Inline rename input activation on F2 / double-click
    - Add Folder, Add Request, and Delete action buttons

- [ ] **Step 1: Write the failing tests in `crates/aura-ui/tests/test_sidebar_tree_view.rs`**

```rust
use aura_core::model::HttpMethod;
use aura_ui::app_state::AuraAppState;
use aura_ui::stores::workspace_store::WorkspaceTreeNode;
use std::path::PathBuf;

#[test]
fn test_sidebar_state_supports_tree_and_inline_rename() {
    let mut state = AuraAppState::new();
    assert!(state.inline_rename.is_none());

    let target_path = PathBuf::from("/workspace/Auth/Login.aura");
    state.start_inline_rename(target_path.clone(), "Login.aura");
    assert!(state.inline_rename.is_some());
    assert_eq!(state.inline_rename.as_ref().unwrap().path, target_path);

    state.cancel_inline_rename();
    assert!(state.inline_rename.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p aura-ui --test test_sidebar_tree_view`
Expected: FAIL with "no field named `inline_rename`"

- [ ] **Step 3: Implement sidebar tree rendering and interaction states**

1. Add `inline_rename: Option<InlineRenameState>` and `drag_target: Option<PathBuf>` to `AuraAppState`.
2. Update `SidebarView::render` to recursively render `WorkspaceTreeNode` elements:
   - For folders: render chevron (`▾`/`▸`), folder icon `📁`, folder name, hover background `#2a2a2c`.
   - For requests: render method badge (`GET` in `#0066cc`, `POST` in `#28a745`, etc.), request name, hover `#2a2a2c`.
   - If item matches `inline_rename`, render inline text input with autofocus.
   - When dragged over: render 1px `#0066cc` border line for reordering.
3. Add action buttons: "+ Req", "+ Folder", and header title showing workspace name.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p aura-ui --test test_sidebar_tree_view`
Expected: PASS with 1 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/aura-ui/src/views/sidebar_view.rs crates/aura-ui/src/app_state.rs crates/aura-ui/tests/test_sidebar_tree_view.rs
git commit -m "feat(ui): implement sidebar hierarchical tree, inline rename, and drag-and-drop visuals"
```

---

### Task 7: End-to-End Workspace Wiring & Two-Way Sync Integration

**Files:**
- Modify: `crates/aura-ui/src/views/workspace_view.rs:1-120`
- Modify: `crates/aura-ui/src/app_state.rs:60-150`
- Modify: `src/main.rs:1-30`
- Test: `tests/integration_workspace_sync.rs`

**Interfaces:**
- Consumes: `WorkspaceService`, `WorkspaceStore`, `SessionStore`, `AuraPreferences`
- Produces:
  - Menu trigger / Button: "File -> Open Workspace"
  - Auto-open workspace on startup from `AuraPreferences`
  - Real-time event loop updating `WorkspaceStore` and `SessionStore` when disk watcher fires

- [ ] **Step 1: Write the failing integration test in `tests/integration_workspace_sync.rs`**

```rust
use aura_core::model::HttpMethod;
use aura_sync::workspace_meta::WorkspaceMetadata;
use aura_ui::app_state::AuraAppState;
use aura_ui::services::workspace_service::WorkspaceService;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_e2e_workspace_loading_and_external_sync_cycle() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    // Set up disk files
    fs::create_dir_all(root.join("Auth")).unwrap();
    fs::write(root.join("Auth").join("Login.aura"), "[request]\nname = \"Login\"\nmethod = \"POST\"\nurl = \"https://echo.com\"").unwrap();

    let mut state = AuraAppState::new();
    state.open_workspace_directory(root).await.expect("open workspace");

    // Verify workspace loaded into store
    assert_eq!(state.workspace_store.tree.len(), 1);

    // Open the request into a tab
    let req_path = root.join("Auth").join("Login.aura");
    state.open_file_in_tab(&req_path).expect("open tab");
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].file_path, Some(req_path.clone()));

    // Simulate external deletion of Login.aura
    fs::remove_file(&req_path).unwrap();

    // Trigger sync cycle
    state.reconcile_workspace_with_disk().await.expect("reconcile");

    // Request tab should have been closed automatically, falling back to Untitled Request
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].title, "Untitled Request");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration_workspace_sync`
Expected: FAIL with "no method named `open_workspace_directory`"

- [ ] **Step 3: Wire workspace lifecycle and watcher into `AuraAppState` and `AuraWorkspaceView`**

1. In `AuraAppState`:
   - Add `open_workspace_directory(&mut self, root: &Path) -> Result<()>`:
     - Saves path to `AuraPreferences`.
     - Reconciles disk with `WorkspaceMetadata`.
     - Builds tree and populates `WorkspaceStore`.
     - Starts `WorkspaceService` watcher.
   - Add `reconcile_workspace_with_disk(&mut self) -> Result<()>`:
     - Reads disk, updates `WorkspaceMetadata`, prunes deleted tabs in `SessionStore`.
2. In `AuraWorkspaceView`:
   - Bind "Open Workspace" button in title bar to trigger `WorkspaceService::pick_folder()`.
   - On launch, check `AuraPreferences::load().last_workspace_path` and automatically load it if directory exists.
   - Spawn background task listening to watcher mpsc channel, updating stores on UI context.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test integration_workspace_sync`
Expected: PASS with 1 passed.

- [ ] **Step 5: Run full workspace test suite and clippy**

Run: `cargo check --workspace`
Expected: Finished with 0 errors.

- [ ] **Step 6: Commit**

```bash
git add crates/aura-ui/src/views/workspace_view.rs crates/aura-ui/src/app_state.rs src/main.rs tests/integration_workspace_sync.rs
git commit -m "feat: wire two-way workspace sync, preference auto-load, and tab synchronization"
```
