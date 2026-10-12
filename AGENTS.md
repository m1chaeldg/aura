# Aura Agent & Architecture Guidelines (`AGENTS.md`)

This document defines the architectural standards, design patterns, and engineering principles for agents and contributors working on the **Aura** codebase.

---

## 1. Architectural Philosophy: Service-Store & Modular Components

Aura is a high-performance desktop REST API client built with **Rust**, **GPUI**, **Tokio**, and **`deno_core`**. 

To maintain high developer ergonomics, long-term maintainability, and silky 60+ FPS UI responsiveness, the codebase strictly follows the **Service-Store & Modular Component** pattern.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        GPUI Views (Presentation)                       │
│   SidebarView │ TabBarView │ RequestHeaderView │ EditorView │ Response │
└───────────────────┬─────────────────────────────────┬──────────────────┘
                    │ observes & renders              │ dispatches user intents
                    ▼                                 ▼
┌───────────────────────────────────────┐   ┌────────────────────────────┐
│         Stores (GPUI Models)          │   │       Async Services       │
│  - WorkspaceStore (collections, files)│◄──┤ - ExecutionService (HTTP,  │
│  - SessionStore (tabs, dirty tracking)│   │   scripts, cancellation)   │
│  - LayoutStore (splits, orientation)  │   │ - WorkspaceService (disk)  │
└───────────────────────────────────────┘   └────────────────────────────┘
```

### Core Tenets

1. **No Monolithic Views**:
   - Views must not exceed ~300–400 lines of code.
   - Avoid monolithic `render()` methods containing multiple complex sub-panels.
   - Decompose screens into single-responsibility GPUI view components (e.g., `SidebarView`, `TabBarView`, `RequestHeaderView`, `ResponsePaneView`).

2. **No God-Object State Structs**:
   - Never bundle UI layout toggles, tabs, disk I/O, network execution, and search indices into a single monolithic state struct.
   - State must be decomposed into granular, focused domain stores wrapped in `gpui::Model<T>`.

3. **Strict Separation of Async Work from the UI Loop**:
   - The main GPUI thread is reserved exclusively for layout, rendering, and handling immediate user input.
   - **Never run synchronous filesystem I/O (`std::fs`) or blocking network calls on the main UI thread.**
   - All I/O, script execution, formatting, and HTTP requests must run via background services.

---

## 2. Layer Specifications

### Layer A: Domain Stores (`gpui::Model<T>`)
Stores represent observable state containers with fine-grained reactivity.
- **`WorkspaceStore`**: Manages collections, folders, and request files loaded from disk. Emits notifications only when the workspace hierarchy changes.
- **`SessionStore`**: Tracks open document tabs, active tab selection, and dirty-state tracking (unsaved changes).
- **`LayoutStore`**: Tracks pane split ratios (0.1 to 0.9), horizontal vs. vertical layout orientation, and drawer visibility.

*Rules for Stores:*
- Store methods must perform pure, deterministic state updates.
- Stores do not initiate direct HTTP calls or blocking disk I/O; they mutate data and notify observers via GPUI's context.

### Layer B: Async Services (Background Engines)
Services encapsulate domain workflows and coordinate asynchronous jobs.
- **`ExecutionService`**:
  - Handles the full HTTP request lifecycle via [`crate::engine::execute_request`](file:///c:/repo/aura/src/engine.rs#L19).
  - Coordinates pre/post-request sandboxed scripting in [`crate::scripting::AuraScriptRuntime`](file:///c:/repo/aura/src/scripting.rs).
  - Manages cooperative cancellation via tokens (`tokio_util::sync::CancellationToken`) so users can abort long-running requests or tab closures immediately.
  - Streams progress and updates back to the UI stores.
- **`WorkspaceService`**:
  - Performs asynchronous directory traversal and collection indexing using `cx.background_executor().spawn(...)`.
  - Handles two-way file persistence and autosave off the main thread.
- **`FormatService`**:
  - Offloads JSON/XML pretty-printing, syntax tree building, and response body filtering to background threads.

### Layer C: Modular UI Views (`gpui::Render`)
Each view is an isolated component responsible only for rendering its slice of the interface:
- **`SidebarView`**: Renders collection folders and request items; emits selection events.
- **`TabBarView`**: Renders open request tabs with method badges (GET, POST, etc.), dirty dots, and tab close buttons.
- **`RequestHeaderView`**: Renders the HTTP method selector, URL input field, and Send action trigger.
- **`RequestEditorView`**: Renders sub-tabs for Query Parameters, Headers, Auth, Body, Scripts, and Tests.
- **`ResponsePaneView`**: Renders status pills, timing/size metrics, response tabs (JSON, Table, Headers, Raw), and formatted content.
- **`SplitResizerView`**: Renders the draggable divider with visual hover states, drag calculations, and double-click 50/50 reset.
- **`CommandPaletteView`**: Modal overlay for quick-search and keyboard-driven actions.

---

## 3. Asynchronous Offloading & Concurrency Rules

1. **Background Spawning**:
   - For GPUI-bound background tasks, use `cx.spawn(|this, mut cx| async move { ... })`.
   - For CPU-heavy or blocking computation (such as Deno V8 execution or large file reads), use GPUI's background executor:
     ```rust
     cx.background_executor().spawn(async move {
         // Heavy I/O or parsing
     })
     ```
2. **Channel-Based Communication**:
   - Use `tokio::sync::mpsc` channels or GPUI's async context to deliver results from background workers to stores.
   - Avoid holding locks (`std::sync::Mutex` or `parking_lot::Mutex`) across `.await` points.
3. **Cancellation**:
   - Every dispatched request must accept a cancellation handle. When the user closes a tab or presses "Cancel", cancel the in-flight task immediately.

---

## 4. Design System Compliance (`DESIGN.md`)

All UI components must adhere strictly to [`DESIGN.md`](file:///c:/repo/aura/DESIGN.md):
- **Base Canvas**: Near-Black `#1d1d1f`
- **Surface Tiles**: Tile 1 `#272729`, Tile 2 `#2a2a2c`
- **Borders**: `#333333`
- **Primary / Action**: Action Blue `#0066cc` (hover: `#0071e3`, dark: `#2997ff`)
- **Method Badges**:
  - GET: `#0066cc` (Blue)
  - POST: `#28a745` (Green)
  - PUT / Dirty Dot: `#f5a623` (Amber)
  - PATCH: `#8e44ad` (Purple)
  - DELETE: `#e74c3c` (Red)
- **Zero Decorative Noise**: No unmotivated gradients, heavy drop shadows on chrome, or decorative clutter. Interface chrome must recede so API requests and data are prominent.

---

## 5. Testing & Verification

1. **Pure Unit Tests**: Keep domain models ([`src/model/`](file:///c:/repo/aura/src/model/)), templating, and engine logic decoupled from GPUI so they can be tested via standard `cargo test`.
2. **Service Tests**: Mock network calls and test asynchronous flows using Tokio test harness.
3. **No Regressions**: Run `cargo check` and `cargo test` before submitting changes.
