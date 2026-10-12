# Design Document: Workspace Management and Two-Way Synchronization

**Date:** 2026-10-12  
**Status:** Draft / Pending Review  
**Author:** Aura Core Architecture Team  

---

## 1. Intent & The Why

Aura is a high-performance desktop REST API client. Developers and teams need to organize complex APIs into logical collections, folders, and request files stored directly on disk in standard version-controlled repositories. 

This feature allows users to open any directory as a workspace. Folders on disk represent standard filesystem directories, requests are saved as `.aura` files, and custom ordering is preserved in a `workspace.json` metadata file. The UI provides a fluid sidebar tree with drag-and-drop reorganization, inline renaming, deletion, and two-way synchronization: changes in the UI immediately persist to disk, and external filesystem changes (e.g., in Windows Explorer or Git checkouts) reflect live in the UI and update open tabs.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- **Open Workspace:** Open an arbitrary directory via **File -> Open Workspace** with a native folder dialog.
- **Session Persistence:** Automatically remember and reopen the last active workspace on application launch.
- **Hierarchical Sidebar Tree:** Render nested directories, subdirectories, and `.aura` request files according to the sequence preserved in `workspace.json`.
- **Drag-and-Drop & Reorganization:** Reorder requests and folders within the same container, or drag items into different folders.
- **Full Disk Synchronization (Two-Way):**
  - In-app actions (create, rename, move, delete) directly update disk directories/files and update `workspace.json`.
  - External filesystem changes (copy/paste in Explorer, Git branch switches) automatically update the sidebar tree and metadata in real time.
- **Open Tab Synchronization:** Renaming or moving an `.aura` file or parent folder updates open tab titles and underlying file paths.
- **Immediate Tab Closure on Deletion:** Deleting an open `.aura` file (via sidebar or disk) immediately closes its associated editor tab.

### Non-Goals
- **`.aura` File Schema Parsing:** The internal content/specification of `.aura` files will be specified in a separate task. For this feature, `.aura` files are treated as opaque request documents identified by name, path, and method badge.
- **Remote / Cloud Workspace Sync:** Sync is purely local filesystem and `workspace.json`. Cloud collaboration is out of scope.
- **Workspace Git Integration:** Git branch switching will naturally trigger the file watcher, but Aura is not building a bespoke Git GUI in this scope.

### Anti-Goals
- **No Monolithic State or UI Blocking:** Blocking the main GPUI render thread with synchronous `std::fs` operations or file watching.
- **No Proprietary Database Lock-in:** The workspace must be transparently inspectable, editable, and versionable using plain files and `workspace.json`.

---

## 3. System Architecture & Component Design

Following `AGENTS.md`, the implementation is partitioned into Domain Stores, Async Background Services, and GPUI Views.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        GPUI Views (Presentation)                       │
│     SidebarView       │      TabBarView       │    WorkspaceMenuView   │
└───────────┬───────────────────────┬───────────────────────┬────────────┘
            │ observes              │ observes              │ triggers
            ▼                       ▼                       ▼
┌───────────────────────┐ ┌───────────────────┐ ┌────────────────────────┐
│    WorkspaceStore     │ │   SessionStore    │ │    WorkspaceService    │
│ - Root path           │ │ - Open tabs       │ │ - Native folder picker │
│ - Virtual tree        │ │ - Active tab      │ │ - Async disk I/O       │
│ - workspace.json meta │ │ - Tab file paths  │ │ - notify file watcher  │
└───────────▲───────────┘ └─────────▲─────────┘ └───────────┬────────────┘
            │                       │                       │
            └───────────────────────┴───────────────────────┘
                     dispatches mutations & watcher events
```

### 3.1 Data Structures & `workspace.json` Schema

Located at `<workspace_root>/workspace.json`:

```json
{
  "version": 1,
  "name": "My Workspace",
  "order": [
    { "type": "folder", "name": "Auth", "order": [
      { "type": "request", "name": "Login.aura" },
      { "type": "request", "name": "Refresh.aura" }
    ]},
    { "type": "folder", "name": "Users", "order": [
      { "type": "request", "name": "Get Profile.aura" }
    ]},
    { "type": "request", "name": "Health Check.aura" }
  ]
}
```

- If an item exists on disk but is absent from `workspace.json` (e.g. newly pasted in Explorer), Aura automatically adds it to the appropriate folder's order array.
- If an item in `workspace.json` no longer exists on disk, Aura cleans up the orphan entry upon synchronization.

### 3.2 Layer Specifications

1. **`WorkspaceStore` (`crates/aura-ui/src/stores/workspace_store.rs`):**
   - Holds `workspace_root: Option<PathBuf>`, workspace name, and hierarchical tree nodes.
   - Provides pure deterministic state mutation methods: `apply_reorder()`, `apply_rename()`, `apply_delete()`, `apply_external_fs_update()`.
   - Never runs synchronous disk I/O directly.

2. **`WorkspaceService` (`crates/aura-ui/src/services/workspace_service.rs`):**
   - Coordinates file operations via `cx.background_executor()`.
   - Uses `rfd` (Rust File Dialog) to show native folder selection dialogs asynchronously.
   - Manages a `notify::RecommendedWatcher` with debouncing (e.g., 100ms) to detect disk events (Create, Modify, Remove, Rename).
   - Ignores echo events triggered by Aura's own writes via short-lived suppression tokens.
   - Persists the last active workspace path in application preferences (e.g. `~/.config/aura/preferences.json` or Windows `APPDATA/aura/preferences.json`).

3. **`SessionStore` (`crates/aura-ui/src/stores/session_store.rs`):**
   - Each tab maintains an optional `file_path: Option<PathBuf>`.
   - When notified of a file/folder rename or move, updates all affected tabs' paths and display titles.
   - When notified of an open file's deletion, immediately removes that tab from the open tab list and resets the active tab.

4. **`SidebarView` (`crates/aura-ui/src/views/sidebar_view.rs`):**
   - Renders collection/workspace root title, folder expansion toggles, and request items.
   - Supports drag-and-drop:
     - Dragging item over an item's top/bottom half displays an insertion line (reorder within same parent).
     - Dragging item over a folder highlights the folder (reorganize into child).
   - Context menus & keyboard triggers:
     - Inline rename (text input field replaces label on F2 or double-click / context menu).
     - Delete (confirmation dialog, then asynchronous disk deletion).
     - New Folder / New Request triggers.

---

## 4. Two-Way Synchronization Flow

1. **User Action in Sidebar (UI -> Disk):**
   - User renames `Auth` to `Authentication`.
   - `SidebarView` dispatches `WorkspaceService::rename_item(...)`.
   - Background worker renames directory on disk from `<root>/Auth` to `<root>/Authentication`.
   - Background worker updates `workspace.json` on disk.
   - `WorkspaceStore` updates in-memory model.
   - `SessionStore` updates paths for all tabs under `<root>/Auth/*`.
2. **External Explorer Action (Disk -> UI):**
   - User pastes `Billing.aura` into `Users/` via Windows Explorer.
   - `notify` watcher fires a debounced `Create` event for `<root>/Users/Billing.aura`.
   - Background service validates the event, reads file metadata, and appends it to `workspace.json`.
   - Background service dispatches tree update to `WorkspaceStore` on the UI thread.
   - `SidebarView` re-renders with `Billing.aura` visible under `Users`.
3. **File Deletion (UI or Disk -> Tab Closure):**
   - An open `.aura` file is removed.
   - `SessionStore` checks all active tabs; any tab whose `file_path` matches the deleted path is immediately closed.

---

## 5. Design System Compliance (`DESIGN.md`)

- **Canvas & Surface:**
  - Sidebar background: Surface Tile 1 `#272729`.
  - Divider borders: `#333333`.
  - Hover background on tree rows: Surface Tile 2 `#2a2a2c`.
  - Drag insertion indicator: Action Blue `#0066cc` (1px horizontal line).
- **Typography & Icons:**
  - Folders: chevron toggle + folder icon.
  - Requests: HTTP method badge (GET `#0066cc`, POST `#28a745`, etc.) with request name in `#f5f5f7`.
- **Clean Aesthetic:** Zero drop shadows or unmotivated gradients. Crisp, functional desktop IDE chrome.

---

## 6. What's Left to the Builder

- Choosing exact debounce duration for the `notify` file watcher (recommend 100ms–200ms).
- Handling cross-platform folder picker integration via `rfd::AsyncFileDialog`.
- Implementation of GPUI drag-and-drop mechanics (custom drag payload containing source item ID and path).
- Preference storage path resolution using `dirs::config_dir()`.
