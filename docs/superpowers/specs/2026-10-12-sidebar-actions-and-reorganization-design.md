# Design Document: Sidebar Actions, Context Menu, Shortcuts & Reorganization

**Date:** 2026-10-12  
**Status:** Draft / Pending User Approval  
**Topic:** Sidebar Action Buttons, Context Menu, Keyboard Shortcuts, Drag-and-Drop Reorganization & Disk Synchronization  

---

## 1. Intent & The Why

Aura is a high-performance desktop REST API client built with Rust and GPUI. When managing large workspaces with dozens or hundreds of API requests and nested folders, users need rapid, frictionless organization and manipulation.

Relying solely on external file managers or manual file creation hinders productivity. Users need:
- **Instant action triggers:** Hover buttons and right-click context menus for common actions (duplicate, copy, paste, delete, new request, new folder).
- **Power keyboard ergonomics:** Full keyboard tree traversal (<kbd>↑</kbd>/<kbd>↓</kbd>/<kbd>←</kbd>/<kbd>→</kbd>), standard shortcuts (<kbd>Ctrl+C</kbd>, <kbd>Ctrl+V</kbd>, <kbd>Ctrl+D</kbd>, <kbd>Del</kbd>, <kbd>F2</kbd>), and seamless mouse-keyboard parity.
- **Visual drag-and-drop:** Intuitive reordering within folders and moving across folders with clear visual indicators.
- **Robust disk & tab synchronization:** Every action immediately persists to disk (`.aura` files, directories, and `workspace.json` sequence), and deleting an open request immediately closes its editor tab.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- **Hover Action Buttons (Option A):** On hover over a sidebar tree row, reveal action buttons:
  - Folders: `+` (new request inside) and `⋯` (more actions menu).
  - Requests: `📋` (quick duplicate) and `⋯` (more actions menu).
- **Context Menu:** Accessible via right-click or clicking `⋯`:
  - **New Request**
  - **New Folder**
  - **Duplicate** (<kbd>Ctrl+D</kbd>)
  - **Copy** (<kbd>Ctrl+C</kbd>)
  - **Paste** (<kbd>Ctrl+V</kbd>)
  - **Rename** (<kbd>F2</kbd>)
  - **Delete** (<kbd>Del</kbd>)
- **Keyboard Shortcuts & Traversal:**
  - <kbd>↑</kbd> / <kbd>↓</kbd>: Navigate through visible tree nodes (folders & requests).
  - <kbd>→</kbd> / <kbd>←</kbd>: Expand / collapse selected folder.
  - <kbd>Enter</kbd>: Open/select the active request into an editor tab.
  - <kbd>Ctrl+C</kbd> / <kbd>Ctrl+V</kbd>: Copy and paste requests/folders with auto-incrementing duplicate names (`(Copy)`).
  - <kbd>Ctrl+D</kbd>: Instant duplicate in-place.
  - <kbd>F2</kbd>: Trigger inline rename.
  - <kbd>Del</kbd>: Delete selected item (with confirmation or undoable trash).
- **Drag-and-Drop Reorganization:**
  - Drag an item up/down to reorder with a horizontal insertion line indicator.
  - Drag an item over a folder to move it inside that folder with a highlight boundary.
- **Two-Way Synchronization & Tab Safety:**
  - Every UI action executes asynchronously via `WorkspaceService` off the main thread.
  - Updates `workspace.json` to maintain persistent order.
  - Deleting an open `.aura` file immediately closes its tab in `SessionStore`.
  - Renaming/moving an open `.aura` file updates the tab path and title.

### Non-Goals
- Multi-selection drag-and-drop (e.g., selecting 10 items with Shift/Ctrl and dragging together). Focus on single-item selection & operations first.
- Operating system clipboard sharing of binary `.aura` files (clipboard uses internal Aura workspace clipboard representation + plain-text path fallback).
- Remote cloud syncing.

### Anti-Goals
- **No UI freezes:** Synchronous filesystem calls (`std::fs`) on GPUI render thread are strictly forbidden.
- **No ghost tabs:** Keeping open tabs pointing to deleted files on disk.
- **No noisy UI:** Hover actions and context menus must adhere strictly to `DESIGN.md` (no heavy glow, no unmotivated drop shadows, clean borders `#333333`, Tile backgrounds `#272729` / `#222224`).

---

## 3. Architecture & Module Decomposition

Following `AGENTS.md` Service-Store architecture:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        GPUI Views (Presentation)                       │
│    SidebarView (Tree + Hover Icons)  │  ContextMenuView (Popup overlay)│
└───────────────────┬─────────────────────────────────┬──────────────────┘
                    │ observes & dispatches           │
                    ▼                                 ▼
┌───────────────────────────────────────┐   ┌────────────────────────────┐
│         Stores (GPUI Models)          │   │       Async Services       │
│  - WorkspaceStore (tree, selection,   │◄──┤ - WorkspaceService         │
│    clipboard, drag state)             │   │   (duplicate, paste, move, │
│  - SessionStore (tab close/rename)    │   │    delete, disk sync)      │
└───────────────────────────────────────┘   └────────────────────────────┘
```

### 3.1 Store Extensions (`crates/aura-ui/src/stores/workspace_store.rs`)
- `selected_path: Option<PathBuf>`: Currently keyboard-focused tree item.
- `clipboard_item: Option<ClipboardPayload>`: Holds path and item type for Copy/Paste operations:
  ```rust
  #[derive(Clone, Debug)]
  pub enum ClipboardPayload {
      Request(PathBuf),
      Folder(PathBuf),
  }
  ```
- `active_context_menu: Option<ContextMenuState>`: Tracks position and target node for the open context menu.
- `drag_state: Option<SidebarDragState>`: Tracks dragging item path, hover target path, and insertion placement (Above, Inside, Below).

### 3.2 Service Extensions (`crates/aura-ui/src/services/workspace_service.rs`)
- `pub async fn duplicate_item(&self, root: &Path, source_path: &Path) -> Result<PathBuf>`
  - Reads `.aura` content or folder recursively.
  - Determines next unique name (e.g. `Login (Copy).aura` or `Login (Copy 2).aura`).
  - Writes to disk, updates suppression token, updates `workspace.json`, returns new path.
- `pub async fn paste_item(&self, root: &Path, payload: &ClipboardPayload, target_dir: &Path) -> Result<PathBuf>`
  - Copies payload source into `target_dir` with collision resolution.
  - Updates `workspace.json`.
- `pub async fn reorder_item(...)` (already existing, extended for folder drops).

### 3.3 Modular UI Views
- `SidebarView` (`crates/aura-ui/src/views/sidebar_view.rs`):
  - Renders rows with hover action buttons.
  - Renders insertion indicator line during drag-over.
  - Intercepts keyboard navigation (<kbd>↑</kbd>, <kbd>↓</kbd>, <kbd>Enter</kbd>, <kbd>Ctrl+C</kbd>, etc.) via GPUI key binding / on_key_down.
- `ContextMenuView` (`crates/aura-ui/src/views/context_menu_view.rs`):
  - Lightweight overlay rendered above sidebar when right-click or `⋯` is triggered.
  - Auto-dismisses on outside click or Escape.

---

## 4. Interaction & UX Details

1. **Hover State:**
   - On cursor enter, row background transitions to `#2a2a2c`.
   - On row right side, mini-buttons fade in:
     - Folders: `+` (New request in folder) and `⋯` (Context menu).
     - Requests: `📋` (Duplicate) and `⋯` (Context menu).
2. **Context Menu:**
   - Styled with background `#252528`, border `#333333`, rounded 6px, subtle drop shadow.
   - Hovered menu item uses Action Blue `#0066cc`.
   - Danger item (Delete) uses `#e74c3c` on hover.
3. **Keyboard Navigation:**
   - Active tree row has a distinct focus indicator (`#27374d` background with subtle `#0066cc` border outline).
   - Standard navigation shortcuts operate instantaneously on the selected node.
4. **Drag & Drop:**
   - Drag handle on any tree node.
   - Reorder between items: displays a 2px `#0066cc` insertion bar between items.
   - Move into folder: highlights the target folder row with `#0066cc` border.
   - Drop: dispatches `reorder_item` async task.

---

## 5. Design Decisions & Builder Calls

- **Delete Confirmation Prompt:** Deleting an item (via Del key or context menu) displays a confirmation modal dialog ("Are you sure you want to delete '[Name]' from disk?") before deleting from disk and closing associated open tabs.
- **Duplicate Naming Strategy (Builder call):** Pattern will be `<original_name> (Copy).aura`. If already exists, `<original_name> (Copy 2).aura`.
- **Recursive Folder Duplication (Builder call):** Duplicating a folder performs a full recursive copy of all child directories, subfolders, and `.aura` files, registering all new items with `workspace.json`.
- **Paste Target Context (Builder call):** If a folder is focused, paste puts the item inside that folder. If a request file is focused, paste places the item in that request's parent folder alongside it.
- **Clipboard Scope (Builder call):** Internal workspace clipboard stored in `WorkspaceStore`.
- **GPUI Drag-and-Drop (Builder call):** Uses GPUI's native drag-and-drop primitives with visual placement lines.

---

## 6. Verification & Test Plan

1. **Unit Tests (`crates/aura-ui/src/services/workspace_service.rs`):**
   - Test `duplicate_item` generates conflict-free name and valid `.aura` copy.
   - Test `paste_item` handles cross-folder pasting and updates `workspace.json`.
   - Test `delete_item` removes file and cleans up `workspace.json`.
2. **Store Tests (`crates/aura-ui/src/stores/workspace_store.rs`):**
   - Test tree selection keyboard navigation indices.
   - Test clipboard state transitions.
3. **UI Integration:**
   - Verify hover icons appear only on hovered row.
   - Verify context menu renders at click coordinates and dismisses on Escape.
   - Verify drag-and-drop updates tree hierarchy and persists to disk.
