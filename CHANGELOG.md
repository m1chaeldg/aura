# Changelog

All notable changes to the Aura API Client will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
- **Testing**:
  - Comprehensive unit test suite and live integration test suite against live Postman Echo endpoints.
