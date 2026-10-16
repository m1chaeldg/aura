# Design Specification: GPUI Kit Component Suite Integration

**Date**: 2026-10-16  
**Status**: Draft for Approval  
**Target Crates**: `crates/aura-ui`, `Cargo.toml`  

---

## 1. Intent & Context

Aura is a high-performance desktop REST API client built on Rust, GPUI, Tokio, and `deno_core`. Currently, many UI elements in `crates/aura-ui` (such as the sidebar collection tree, tab bar, split resizers, and raw input fields) are hand-crafted from basic `div()` primitives, requiring bespoke state handling, custom row indentation loops, and manual event bindings.

This project integrates the ready-made component library `gpui_kit` across Aura's presentation layer to standardize interface chrome, simplify maintainability, and provide a polished desktop developer experience.

---

## 2. Goals, Non-Goals & Anti-Goals

### Goals
- **Adopt TabBar Primitives**: Replace custom tab-strip code in [`tab_bar_view.rs`](file:///c:/repo/aura/crates/aura-ui/src/views/tab_bar_view.rs) with `gpui_kit::component::tab::{Tab, TabBar}`.
- **Adopt Sidebar & Tree**: Replace manual recursive tree rendering in [`sidebar_view.rs`](file:///c:/repo/aura/crates/aura-ui/src/views/sidebar_view.rs) with `gpui_kit::component::sidebar::{Sidebar, SidebarHeader, SidebarFooter, SidebarGroup, SidebarMenu, SidebarMenuItem, SidebarToggleButton}` and `gpui_kit::component::tree::{tree, TreeState, TreeItem, TreeEntry}`.
- **Dock Layout Integration**: Leverage `gpui_kit::component::dock` to handle the workspace layout (left navigation sidebar, center request editor, and collapsible response pane).
- **Code Editor Integration**: Replace basic text inputs in request body, headers, and scripting panels with `gpui_kit::component::editor` for syntax highlighting and formatted viewing.
- **Context Menus**: Standardize right-click context menus across collections, folders, request items, and open tabs using `gpui_kit::component::context_menu`.

### Non-Goals
- Modifying backend core engines (`aura-engine`, `aura-scripting`, `aura-sync`, `aura-parser`, `aura-core`).
- Altering disk serialization formats for collections, environments, or requests.
- Modifying Tokio asynchronous worker lifecycles or HTTP execution routines.

### Anti-Goals
- **No Visual Regression**: Must not lose Aura's established method badge colors (GET `#0066cc`, POST `#28a745`, PUT/Dirty `#f5a623`, PATCH `#8e44ad`, DELETE `#e74c3c`).
- **No Loss of Dirty Tracking**: Tabs must preserve the unsaved changes indicator dot and close confirmation safeguards.
- **No Performance Regressions**: Component rendering must maintain smooth 60+ FPS interaction without main-thread blocking.

---

## 3. Architecture & Component Mapping

```
┌────────────────────────────────────────────────────────────────────────┐
│             Root Window: gpui_kit::component::dock::Dock              │
├────────────────────┬───────────────────────────────────────────────────┤
│ Navigation Dock    │ Workspace Dock                                    │
│                    │                                                   │
│ Sidebar            │ TabBar (gpui_kit::component::tab::TabBar)        │
│ ├─ SidebarHeader   │ ├─ Tab: GET /profile (Active, Dirty Dot, Close)   │
│ ├─ SidebarGroup    │ └─ Tab: POST /login (Inactive, Close)             │
│ │  └─ Tree         ├───────────────────────────────────────────────────┤
│ │     └─ TreeEntry │ RequestHeaderView (Method selector + URL + Send)  │
│ └─ SidebarFooter   ├─────────────────────────┬─────────────────────────┤
│                    │ RequestEditorView       │ ResponsePaneView        │
│ [Context Menus]    │ (gpui_kit Editor)       │ (gpui_kit Editor)       │
│ - New Request      │ - Params / Headers      │ - JSON / XML Viewer     │
│ - New Folder       │ - JSON Body Editor      │ - Headers Table         │
│ - Delete / Rename  │ - Tests / Scripts       │ - Timing / Size Stats   │
└────────────────────┴─────────────────────────┴─────────────────────────┘
```

### Detailed Component Roles

1. **`TabBar` (`gpui_kit::component::tab`)**:
   - Manages horizontal tab strip scrolling, tab selection, and close actions.
   - Renders child `Tab` elements containing Aura's colored HTTP method badges, request name labels, unsaved dirty dots, and close buttons.
   - Emits callbacks directly to `AuraAppState` / `SessionStore`.

2. **`Sidebar` & `Tree` (`gpui_kit::component::sidebar`, `gpui_kit::component::tree`)**:
   - `Sidebar`: Wraps top header (collection search / filter, new collection trigger) and bottom footer (settings / sync status).
   - `Tree`: Uses `TreeState` to track expanded/collapsed folder nodes, selection, and drag-and-drop reordering.
   - `TreeItem` / `TreeEntry`: Renders collection folders and request leaves with method badges and item count tags.

3. **`Dock` (`gpui_kit::component::dock`)**:
   - Provides split containers with draggable handles between the sidebar, request editor, and response pane.
   - Replaces manual split percentage math in `app_state.rs` with `gpui_kit`'s dock panel state.

4. **`Editor` (`gpui_kit::component::editor`)**:
   - Used for JSON/XML/GraphQL request bodies and pre/post-request JavaScript scripting tabs.
   - Used in read-only mode for formatted response payload viewing with syntax highlighting.

5. **`ContextMenu` (`gpui_kit::component::context_menu`)**:
   - Attached to tree nodes for folder/request operations (Rename, Duplicate, New Request, New Folder, Delete).
   - Attached to tabs for tab operations (Close, Close Others, Close to the Right).

---

## 4. Technical Constraints & Design System Alignment

1. **GPUI Compatibility**: Must cleanly integrate with workspace GPUI version `0.2.2`.
2. **Color Palette Compliance (`DESIGN.md`)**:
   - Base Canvas: `#1d1d1f`
   - Surface Tile 1: `#272729`
   - Surface Tile 2: `#2a2a2c`
   - Divider / Borders: `#333333`
   - Accent / Active: `#0066cc`
   - Text Colors: Primary `#ffffff`, Secondary `#cccccc`, Muted `#7a7a7a`
3. **Modular Code Limit**: Individual view components must adhere to the `AGENTS.md` constraint of <= 300–400 lines of code per file.

---

## 5. Decisions Made & Left to Builder

### Decided by User
- Complete workspace overhaul (Option A) covering all major UI components: `Dock`, `Sidebar`, `Tree`, `TabBar`, `Editor`, and `ContextMenu`.
- Full replacement of hand-crafted row math and tab strips in favor of `gpui_kit`'s official component API.

### Left to Builder
- Exact version pin or dependency specification for `gpui_kit` in `Cargo.toml`.
- Fine-tuning the internal bridge between `AuraAppState` collections and `gpui_kit::component::tree::TreeState`.
- Styling configuration mapping to ensure `gpui_kit`'s default theme aligns with Aura's `#1d1d1f` dark palette.
