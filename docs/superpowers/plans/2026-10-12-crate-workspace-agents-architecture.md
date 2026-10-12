# Multi-Crate Workspace & Agents Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Partition the monolithic Aura codebase into a high-performance 5-crate Cargo Workspace (`aura-core`, `aura-engine`, `aura-scripting`, `aura-sync`, `aura-ui`), decompose the UI into granular Domain Stores and Modular Views under 400 lines, and maintain 100% backwards compatibility with all existing test suites.

**Architecture:** Split the codebase along domain boundaries into a clean DAG workspace where `aura-core` has zero heavy dependencies, `deno_core` is isolated to `aura-scripting`, `reqwest`/`tokio` live in `aura-engine`, and `aura-ui` follows the Service-Store pattern (`WorkspaceStore`, `SessionStore`, `LayoutStore`, `ExecutionService`, `WorkspaceService`). The root `aura` crate serves as the application executable and re-export facade.

**Tech Stack:** Rust 2021, Cargo Workspace, GPUI 0.2.2, Tokio 1.x, Reqwest 0.12, Deno Core 0.330.0, Serde.

**Spec:** [`docs/superpowers/specs/2026-10-12-crate-workspace-agents-architecture-design.md`](file:///c:/repo/aura/docs/superpowers/specs/2026-10-12-crate-workspace-agents-architecture-design.md)

---

## Global Constraints

- Must follow the Service-Store & Modular Component pattern defined in `AGENTS.md`.
- No monolithic views: each view component must not exceed ~300–400 lines of code.
- No god-object state struct: decompose `AuraAppState` into `WorkspaceStore`, `SessionStore`, and `LayoutStore`.
- Strict separation of async work from the UI loop: background I/O and HTTP execution must run via async services with cancellation tokens.
- All 30 existing unit and integration tests in `tests/` must compile and pass with zero regressions.
- All UI colors, spacing, and badge styles must adhere strictly to `DESIGN.md`.

## Review Focus

1. **Compilation memory thrashing:** Ensure crates compile independently so `rustc` does not exhaust the Windows paging file (`os error 1455`).
2. **Circular crate dependencies:** Ensure the crate dependency graph is strictly a Directed Acyclic Graph (DAG) with `aura-core` as the root.
3. **Tab close fallback behavior:** Closing the last active tab must cleanly spawn an untitled request tab without crashing or panicking.
4. **Offline sandbox integrity:** Ensure `aura-scripting` does not pull in network or filesystem capabilities into the V8 isolate.
5. **Lossless dirty-state sync:** Verify that typing in tabs marks `is_dirty = true` and saving persists TOML files accurately to disk.

---

### Task 1: Initialize Cargo Workspace and Extract `crates/aura-core`

**Files:**
- Create: `crates/aura-core/Cargo.toml`
- Create: `crates/aura-core/src/lib.rs`
- Move/Create: `crates/aura-core/src/model.rs`
- Move/Create: `crates/aura-core/src/templating.rs`
- Move/Create: `crates/aura-core/src/security.rs`
- Move/Create: `crates/aura-core/src/interceptor.rs`
- Modify: `Cargo.toml` (add workspace declaration)

**Interfaces:**
- Produces: `aura_core::model::*`, `aura_core::templating::*`, `aura_core::security::*`, `aura_core::interceptor::*`
- Consumes: `serde`, `serde_json`, `uuid`, `chrono`, `regex`, `base64`

- [x] **Step 1: Write the failing crate test**
  Add a smoke test in `crates/aura-core/tests/test_core_models.rs`:
  ```rust
  use aura_core::model::{AuraRequest, HttpMethod};
  use aura_core::templating::TemplateContext;

  #[test]
  fn test_core_smoke() {
      let req = AuraRequest::new("Smoke", HttpMethod::Get, "https://httpbin.org/get");
      assert_eq!(req.name, "Smoke");
      let mut ctx = TemplateContext::new();
      ctx.set("host", "httpbin.org");
      assert_eq!(ctx.resolve("https://{{host}}/get"), "https://httpbin.org/get");
  }
  ```

- [x] **Step 2: Run test to verify it fails**
  Run: `cargo test -p aura-core`
  Expected: FAIL (crate not found or unconfigured)

- [x] **Step 3: Implement `crates/aura-core` and configure workspace root**
  Create `crates/aura-core/Cargo.toml` with minimal dependencies, move `model`, `templating`, `security`, and `interceptor` modules into `crates/aura-core/src/`, and update root `Cargo.toml` to declare `[workspace]`.

- [x] **Step 4: Run test to verify it passes**
  Run: `cargo test -p aura-core`
  Expected: PASS

- [x] **Step 5: Commit**
  ```bash
  git add crates/aura-core Cargo.toml
  git commit -m "feat(core): scaffold aura-core crate with domain models and templating"
  ```

---

### Task 2: Extract `crates/aura-engine`

**Files:**
- Create: `crates/aura-engine/Cargo.toml`
- Create: `crates/aura-engine/src/lib.rs`
- Move/Create: `crates/aura-engine/src/engine.rs`
- Move/Create: `crates/aura-engine/src/runner.rs`
- Move/Create: `crates/aura-engine/src/mock.rs`
- Move/Create: `crates/aura-engine/src/logging.rs`

**Interfaces:**
- Produces: `aura_engine::execute_request`, `aura_engine::run_collection`, `aura_engine::MockServer`, `aura_engine::logging::*`
- Consumes: `aura-core`, `tokio`, `reqwest`, `tracing`, `tracing-subscriber`, `tracing-appender`, `parking_lot`, `anyhow`

- [x] **Step 1: Write the failing engine test**
  Add `crates/aura-engine/tests/test_engine_smoke.rs`:
  ```rust
  use aura_core::model::{AuraRequest, HttpMethod};
  use aura_engine::execute_request;

  #[tokio::test]
  async fn test_engine_smoke() {
      let req = AuraRequest::new("Smoke", HttpMethod::Get, "https://postman-echo.com/get");
      let result = execute_request(&req, None).await;
      assert!(result.is_ok());
  }
  ```

- [x] **Step 2: Run test to verify it fails**
  Run: `cargo test -p aura-engine`
  Expected: FAIL (crate not configured)

- [x] **Step 3: Implement `crates/aura-engine`**
  Move `engine.rs`, `runner.rs`, `mock.rs`, and `logging.rs` to `crates/aura-engine/src/`, referencing `aura_core`.

- [x] **Step 4: Run test to verify it passes**
  Run: `cargo test -p aura-engine`
  Expected: PASS

- [x] **Step 5: Commit**
  ```bash
  git add crates/aura-engine
  git commit -m "feat(engine): extract aura-engine crate for async HTTP execution and logging"
  ```

---

### Task 3: Extract `crates/aura-scripting`

**Files:**
- Create: `crates/aura-scripting/Cargo.toml`
- Create: `crates/aura-scripting/src/lib.rs`
- Move/Create: `crates/aura-scripting/src/scripting.rs`

**Interfaces:**
- Produces: `aura_scripting::AuraScriptRuntime`
- Consumes: `aura-core`, `deno_core`, `anyhow`

- [x] **Step 1: Write the failing scripting test**
  Add `crates/aura-scripting/tests/test_scripting_smoke.rs`:
  ```rust
  use aura_scripting::AuraScriptRuntime;

  #[test]
  fn test_scripting_smoke() {
      let mut runtime = AuraScriptRuntime::new().expect("runtime initialization");
      let result = runtime.execute("pm.environment.set('token', 'abc123xyz');");
      assert!(result.is_ok());
      assert_eq!(runtime.get_environment_var("token"), Some("abc123xyz".to_string()));
  }
  ```

- [x] **Step 2: Run test to verify it fails**
  Run: `cargo test -p aura-scripting`
  Expected: FAIL

- [x] **Step 3: Implement `crates/aura-scripting`**
  Move `scripting.rs` to `crates/aura-scripting/src/lib.rs`, configuring isolated offline `deno_core` bindings.

- [x] **Step 4: Run test to verify it passes**
  Run: `cargo test -p aura-scripting`
  Expected: PASS

- [x] **Step 5: Commit**
  ```bash
  git add crates/aura-scripting
  git commit -m "feat(scripting): isolate deno_core V8 sandbox in aura-scripting crate"
  ```

---

### Task 4: Extract `crates/aura-sync`

**Files:**
- Create: `crates/aura-sync/Cargo.toml`
- Create: `crates/aura-sync/src/lib.rs`
- Move/Create: `crates/aura-sync/src/sync.rs`
- Move/Create: `crates/aura-sync/src/importers/` (curl, insomnia, openapi, postman)

**Interfaces:**
- Produces: `aura_sync::request_to_toml`, `aura_sync::toml_to_request`, `aura_sync::importers::*`
- Consumes: `aura-core`, `toml`, `serde`, `serde_json`, `anyhow`

- [x] **Step 1: Write the failing sync test**
  Add `crates/aura-sync/tests/test_sync_smoke.rs`:
  ```rust
  use aura_core::model::{AuraRequest, HttpMethod};
  use aura_sync::{request_to_toml, toml_to_request};

  #[test]
  fn test_sync_smoke() {
      let req = AuraRequest::new("SyncTest", HttpMethod::Post, "https://api.example.com");
      let toml_str = request_to_toml(&req).expect("serialize to toml");
      let parsed = toml_to_request(&toml_str).expect("deserialize from toml");
      assert_eq!(parsed.name, "SyncTest");
  }
  ```

- [x] **Step 2: Run test to verify it fails**
  Run: `cargo test -p aura-sync`
  Expected: FAIL

- [x] **Step 3: Implement `crates/aura-sync`**
  Move `sync.rs` and `importers/` into `crates/aura-sync/src/`.

- [x] **Step 4: Run test to verify it passes**
  Run: `cargo test -p aura-sync`
  Expected: PASS

- [x] **Step 5: Commit**
  ```bash
  git add crates/aura-sync
  git commit -m "feat(sync): extract aura-sync crate for disk persistence and format importers"
  ```

---

### Task 5: Implement `crates/aura-ui` with Service-Store Pattern

**Files:**
- Create: `crates/aura-ui/Cargo.toml`
- Create: `crates/aura-ui/src/lib.rs`
- Create: `crates/aura-ui/src/design.rs`
- Create: `crates/aura-ui/src/keybindings.rs`
- Create: `crates/aura-ui/src/stores/mod.rs`
- Create: `crates/aura-ui/src/stores/workspace_store.rs`
- Create: `crates/aura-ui/src/stores/session_store.rs`
- Create: `crates/aura-ui/src/stores/layout_store.rs`
- Create: `crates/aura-ui/src/services/mod.rs`
- Create: `crates/aura-ui/src/services/execution_service.rs`
- Create: `crates/aura-ui/src/services/workspace_service.rs`
- Create: `crates/aura-ui/src/services/format_service.rs`

**Interfaces:**
- Produces:
  - `WorkspaceStore` (collections, folder indexing)
  - `SessionStore` (tabs, active tab index, dirty dot tracking)
  - `LayoutStore` (split ratio 0.1..0.9, horizontal/vertical orientation, modals)
  - `ExecutionService` (Tokio HTTP execution and cancellation handle)
  - `WorkspaceService` (asynchronous directory loader and autosave)
  - `FormatService` (syntax parsing and formatting)
- Consumes: `aura-core`, `aura-engine`, `aura-sync`, `aura-scripting`, `gpui`, `tokio`

- [x] **Step 1: Write store unit tests**
  Add unit tests in `crates/aura-ui/tests/test_stores.rs` validating that:
  - `SessionStore` adds tabs, switches active index, marks dirty, and closes tabs with safe fallback.
  - `LayoutStore` clamps split ratio between 0.1 and 0.9, and resets to 0.5 upon double click.
  - `WorkspaceStore` manages collections and searches requests.

- [x] **Step 2: Run test to verify it fails**
  Run: `cargo test -p aura-ui`
  Expected: FAIL

- [x] **Step 3: Implement domain stores and async services in `crates/aura-ui`**
  Implement `WorkspaceStore`, `SessionStore`, `LayoutStore`, `ExecutionService`, and `WorkspaceService` conforming to `AGENTS.md` Layer A and Layer B.

- [x] **Step 4: Run test to verify it passes**
  Run: `cargo test -p aura-ui`
  Expected: PASS

- [x] **Step 5: Commit**
  ```bash
  git add crates/aura-ui
  git commit -m "feat(ui): implement Domain Stores and Async Services in aura-ui"
  ```

---

### Task 6: Decompose `workspace_view` into Modular Views (< 300–400 Lines Each)

**Files:**
- Create: `crates/aura-ui/src/views/mod.rs`
- Create: `crates/aura-ui/src/views/sidebar_view.rs`
- Create: `crates/aura-ui/src/views/tab_bar_view.rs`
- Create: `crates/aura-ui/src/views/request_header_view.rs`
- Create: `crates/aura-ui/src/views/request_editor_view.rs`
- Create: `crates/aura-ui/src/views/response_pane_view.rs`
- Create: `crates/aura-ui/src/views/split_resizer_view.rs`
- Create: `crates/aura-ui/src/views/command_palette_view.rs`
- Create: `crates/aura-ui/src/views/workspace_view.rs`

**Interfaces:**
- Produces: `AuraWorkspaceView` (modular root GPUI view coordinating sub-views)
- Consumes: `SidebarView`, `TabBarView`, `RequestHeaderView`, `RequestEditorView`, `ResponsePaneView`, `SplitResizerView`, `CommandPaletteView`

- [x] **Step 1: Write view composition tests**
  Add tests verifying that `AuraWorkspaceView` initializes and renders sub-views without exceeding line count constraints.

- [x] **Step 2: Run test to verify it fails**
  Run: `cargo test -p aura-ui`
  Expected: FAIL

- [x] **Step 3: Implement modular view components**
  Decompose the 1,000+ line monolith into focused single-responsibility view components under 300–400 lines each adhering strictly to `DESIGN.md`.

- [x] **Step 4: Run test to verify it passes**
  Run: `cargo test -p aura-ui`
  Expected: PASS

- [x] **Step 5: Commit**
  ```bash
  git add crates/aura-ui/src/views
  git commit -m "refactor(ui): decompose monolithic workspace_view into modular views under 400 lines"
  ```

---

### Task 7: Rewire Root `aura` Facade & Run Global Test Verification

**Files:**
- Modify: `Cargo.toml` (declare workspace members and root facade dependencies)
- Modify: `src/lib.rs` (re-export `aura_core`, `aura_engine`, `aura_scripting`, `aura_sync`, `aura_ui`)
- Modify: `src/main.rs` (launch `aura_ui::AuraWorkspaceView`)
- Test: All 30 tests in `tests/` (`unit_ui.rs`, `unit_importers.rs`, `integration_postman_echo.rs`, `qa_load_stability.rs`, etc.)

**Interfaces:**
- Produces: `aura` binary executable and seamless facade library `aura::*`
- Consumes: `aura-core`, `aura-engine`, `aura-scripting`, `aura-sync`, `aura-ui`

- [x] **Step 1: Verify all 11 test suites compile against root facade**
  Run: `cargo test --tests -j 2`
  Expected: All 30 tests pass.

- [x] **Step 2: Run `cargo check --workspace`**
  Expected: Clean compilation with 0 warnings or errors.

- [x] **Step 3: Verify binary build `cargo build --bin aura`**
  Expected: Successfully produces `target/debug/aura.exe`.

- [x] **Step 4: Commit**
  ```bash
  git add Cargo.toml src/lib.rs src/main.rs
  git commit -m "chore: rewire aura facade and verify full workspace test suite passes"
  ```
