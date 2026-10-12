# Changelog

All notable changes to the Aura API Client will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Modular Cargo Workspace Architecture (`AGENTS.md`)**:
  - Partitioned monolithic repository into 5 domain-isolated crates:
    - `aura-core`: Zero-dependency domain models (`AuraRequest`, `AuraResponse`, `HttpMethod`), cascading templating, token-boundary secret scrubber, JWT inspector, and design tokens.
    - `aura-scripting`: Sandboxed offline `deno_core` V8 runtime with BDD assertions (`pm.test`, `pm.expect`) and console logging.
    - `aura-engine`: Asynchronous Tokio HTTP dispatcher, collection runner, mock server, and diagnostic log broadcaster.
    - `aura-sync`: Lossless two-way TOML synchronization and format importers (Postman, Insomnia, OpenAPI, cURL).
    - `aura-ui`: GPUI presentation layer implementing the Service-Store pattern.
  - Configured root `aura` crate as a seamless facade library and desktop binary entrypoint (`aura.exe`) ensuring 100% backward compatibility.
- **Service-Store & Modular Component Pattern**:
  - **Layer A (Domain Stores)**: `WorkspaceStore` (collections and folder indexing), `SessionStore` (tabs, active tab index, and dirty modification tracking), and `LayoutStore` (split ratio 0.10–0.90, 50/50 reset, and orientation).
  - **Layer B (Async Services)**: `ExecutionService` (with cooperative cancellation via `tokio_util::sync::CancellationToken`), `WorkspaceService` (background directory traversal and file persistence off the main UI loop), and `FormatService` (background syntax parsing).
  - **Layer C (Modular Views)**: Decomposed 909-line view monolith into isolated, single-responsibility components under 300 lines each: `SidebarView`, `TabBarView`, `RequestHeaderView`, `RequestEditorView`, `ResponsePaneView`, `SplitResizerView`, `CommandPaletteView`, and `AuraWorkspaceView`.
- **Core Architecture & Engine**:
  - High-performance asynchronous HTTP engine dispatching GET, POST, PUT, PATCH, DELETE, HEAD, and OPTIONS via `reqwest`.
  - Built-in sandboxed V8 scripting runtime powered by `deno_core` for offline pre-request, post-response, and BDD test suites (`aura.test(...)`, `expect(...)`).
  - Eradicated all telemetry and network tracking.
- **UI Design System (`DESIGN.md`) & Workspace Ergonomics**:
  - Full adherence to Apple design language: Action Blue (`#0066cc`), pure white canvas (`#ffffff`), parchment canvas (`#f5f5f7`), and dark tile surfaces (`#272729`).
  - Standardized HTTP Method badges: GET (Blue: `#0066cc`), POST (Green: `#28a745`), PUT (Amber: `#f5a623`), PATCH (Purple: `#8e44ad`), DELETE (Rose: `#e74c3c`).
  - Color-coded HTTP status pills: 2xx Green, 3xx Blue, 4xx Amber, 5xx Red.
  - Multi-tab document workspace with simultaneous tab management, dedicated close buttons (`×`), dirty state tracking with visual dot indicators (`•`), and persistent `+` action.
  - Responsive split pane layout with adjustable percentage dividers (0.10 to 0.90), smooth resizer handle, 50/50 double-click reset, and Horizontal (Side-by-Side) vs Vertical (Stacked) orientation toggling.
  - Globally accessible Command Palette modal overlay with instant request fuzzy search and keyboard execution (`↑`/`↓`/`↵`/`Esc`).
  - Native Keybinding Presets supporting popular IDE profiles: Visual Studio Code, IntelliJ IDEA, and Sublime Text.
  - Comprehensive keyboard shortcuts for primary workflows: Send Request (`Cmd/Ctrl + Enter`), Command Palette (`Cmd/Ctrl + P`), Log Viewer (`Cmd/Ctrl + Shift + L`), New Tab (`Cmd/Ctrl + N`/`T`), Close Tab (`Cmd/Ctrl + W`), Toggle Layout (`Cmd/Ctrl + \`), Reset Divider (`Cmd/Ctrl + R`), and Save (`Cmd/Ctrl + S`).
- **Dual-Mode Request Editor**:
  - Lossless bidirectional synchronization between Visual Editor and Code Editor (TOML).
  - Query parameters and headers serialization with trailing tilde (`key~ = "value"`) for disabled items.
  - Body support for JSON, XML, Text, Form URL-encoded, Multipart, and relative path-anchored Binary uploads.
- **Dynamic Templating & Environments**:
  - Cascading `{{var}}` resolution across Request -> Environment -> Collection -> OS Environment (`{{$ENV}}`).
  - Token-boundary Secret Scrubber to guard sensitive credentials in logs and outputs.
  - Integrated offline JWT Inspector parsing base64 headers and claims.
- **Collection Runner & Statistical HDR Latency Profiler**:
  - Concurrent worker pool micro-load execution with delay/pacing configuration.
  - High Dynamic Range (HDR) percentile profiler calculating Min, Mean, p50, p90, p95, p99, and Max latencies.
  - Real-time RPS throughput, success/failure counts, and error rate tracking.
- **Interoperability & Importers**:
  - Native importers for Postman Collections (v2.1), Insomnia exports, OpenAPI v2/v3 specifications, and raw cURL commands.
- **Diagnostic Logging & Console**:
  - Structured, rolling file logging to `aura.log`.
  - In-memory real-time event broadcaster for the UI log viewer dock and script developer console.
- **Testing & Verification**:
  - Comprehensive workspace test suites (37 tests) with zero test regressions across unit, integration, and load suites.

### Changed
- Re-architected monolithic code structure into a modular Cargo workspace, reducing compilation unit memory pressure and eliminating linker contention.
- Refactored `AuraWorkspaceView` from a 909-line monolith into composable sub-views strictly adhering to view size caps in `AGENTS.md`.

### Fixed
- Fixed two-way external workspace synchronization: wired live filesystem watcher in `AuraWorkspaceView` with debounced reconciliation, enabling instantaneous sidebar treeview reload when external files or folders are pasted, created, moved, renamed, or deleted in Windows Explorer.
- Fixed Windows mmap memory exhaustion (`os error 1455`) during parallel test execution by partitioning crate translation units.
- Fixed missing MSVC ETW and registry symbol linkage (`advapi32`) when compiling V8 and `deno_core`.
