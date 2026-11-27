# Changelog

All notable changes to the Aura API Client will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Multi-Platform Release Workflow (`.github/workflows/release.yml`)**:
  - Automated GitHub Actions workflow building release binaries for **macOS** (Apple Silicon `aarch64-apple-darwin` and Intel `x86_64-apple-darwin`), **Linux** (`x86_64-unknown-linux-gnu`), and **Windows** (`x86_64-pc-windows-msvc`).
  - Bundles `README.md`, `CHANGELOG.md`, `LICENSE`, `docs/`, and SHA256 checksums into each release archive and auto-populates GitHub Release notes from `CHANGELOG.md`.
- **Multi-Protocol Support (GraphQL, WebSockets, SSE, gRPC, SOAP)**:
  - **GraphQL**: Dedicated query & variables editor and execution support.
  - **WebSockets (`ws://` / `wss://`)**: Live bidirectional connection management and message streaming.
  - **Server-Sent Events (SSE)**: Streaming `text/event-stream` viewer.
  - **gRPC & SOAP/WSDL**: `.proto` / gRPC request support and WSDL import into Aura collections.
- **JMeter-Style Load Testing & Headless CLI (`aura load`)**:
  - **Load Profiles**: Fixed iterations, time-boxed soak/stress durations (`--duration`), ramp-up/ramp-down (`--ramp-up`, `--ramp-down`), multi-stage spike schedules (`--stages`), constant throughput shaping (`--target-rps`), uniform/Gaussian think-time timers, synchronizing rendezvous barriers (`--sync-barrier`), and weighted traffic routing (`--weighted`).
  - **CSV / JSON Data Feeder**: Parameterize virtual users with `AllVUs` (shared) or `PerVU` row distribution and `Recycle` or `StopVuOnEof` EOF handling.
  - **Zero-JS Native Extractors**: High-concurrency response correlation via JSONPath, Regex Body, Regex Header, Header, and Cookie extractors without V8 overhead.
  - **SLA Gates & Reporting**: Per-sample duration/size assertions, global CI SLA thresholds (`--sla-p95`, `--sla-p99`, `--sla-error`), and exports to JTL-style CSV, JSON, and standalone HTML reports.
- **Declarative Assertions, Async Scripting & Request Chaining**:
  - **No-Code Assertions (`[assert]`)**: Point-and-click response validation rules (`res.status`, `res.duration_ms`, `res.body.*`, `res.headers.*`) persisted directly in `.aura` files.
  - **Async `fetch` & `aura.sendRequest`**: Pre-request, post-response, and test scripts support `await fetch(...)` and `await aura.sendRequest(...)` (plus `pm.sendRequest`).
  - **Request Chaining (`aura.setNextRequest`)**: Conditional flow control across Collection Runner steps with shared variables and cookies.
- **Network Tooling, Codegen & Git Integration**:
  - **Cookie Jar**: Automatic `Set-Cookie` persistence and modal inspection/management.
  - **Proxy & MITM Capture Proxy**: Global HTTP/HTTPS/SOCKS5 proxy configuration plus a local capture proxy & interceptor rules.
  - **Code Snippet Generator**: Export active requests as `cURL`, `fetch`, `axios`, Python, Rust, or Go.
  - **Built-in Git UI**: In-app `git status`, diff viewer, commit, branch, and push/pull controls.
- **Request Spec Tab & Lossless Bidirectional Sync**:
  - Added new "Spec" sub-tab in `RequestEditorView` (positioned after Settings) displaying the canonical `.aura` specification format (`docs/file_format.md`).
  - Implemented lossless bidirectional synchronization: visual tab modifications automatically regenerate the spec buffer on switch, and live text edits in the Spec editor update the underlying request model and visual tabs in real-time.
  - Built-in syntax error resilience: keeps visual tabs safely anchored to the last valid state during active typing while displaying an inline syntax status indicator (`⚠`).
  - Full developer toolbar with line and byte metrics, "⚡ Format Spec" canonical formatter, "Copy Spec" clipboard integration, "Revert" button, and live "✓ Synced" / "⚠ Error" status pills.
  - Integrated with Aura's unified save and debounced auto-save lifecycle to persist `.aura` files directly to disk.
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
