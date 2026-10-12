# Aura API Client

A high-performance REST API client engineered in Rust, featuring local workspace management, lossless dual-mode editing, offline sandboxed scripting (`deno_core`), and an Apple-inspired UI design system (`DESIGN.md`).

## Key Features

- **Apple Design Language (`DESIGN.md`)**:
  - Action Blue (`#0066cc`) interactive accents.
  - Multi-tab document workspace with dirty state tracking and quick `+` request creation.
  - Standardized HTTP Method badges (GET: Blue, POST: Green, PUT: Amber, PATCH: Purple, DELETE: Rose).
  - Responsive split pane with double-click 50/50 reset and horizontal/vertical orientation toggle.
  - Command Palette for quick search and navigation.
- **Dual-Mode Request Editor**:
  - Visual form editor seamlessly and losslessly synchronized with Code Editor (TOML format).
  - Disabled parameters and headers serialize as `key~ = "value"`.
  - Body support: JSON, XML, Text, Form-urlencoded, Multipart, and relative path-anchored Binary uploads.
- **Sandboxed Offline Scripting (`deno_core`)**:
  - Embedded V8 runtime for Pre-Request, Post-Response, and BDD-style test suites (`aura.test(...)`, `expect(...)`).
  - Zero telemetry and strictly offline execution (no external network leaking).
  - Integrated Developer Console capturing `console.log`, `console.warn`, and `console.error`.
- **Dynamic Templating & Security**:
  - Cascading `{{var}}` resolution: Request -> Environment -> Collection -> OS Environment (`{{$ENV}}`).
  - Token-boundary secret scrubber to prevent sensitive tokens from leaking into logs.
  - Built-in offline JWT Inspector with claims and expiration validation.
- **Collection Runner & Statistical HDR Latency Profiler**:
  - Micro-load testing with configurable concurrency worker pools and iteration pacing.
  - Granular latency percentiles: Min, Mean, p50, p90, p95, p99, and Max.
  - Real-time RPS and error rate calculations.
- **Interoperability**:
  - Import Postman Collections (v2.1), Insomnia exports, OpenAPI v2/v3, and raw cURL commands.
- **Structured Diagnostics**:
  - Rolling log files (`aura.log`) and in-memory broadcaster for the UI log viewer.

## Getting Started

### Prerequisites

- Rust 1.80+ (tested on Rust 1.97+)
- Cargo

### Building the Project

```bash
# Debug build
cargo build

# Optimized release binary
cargo build --release
```

### Running the Application

```bash
cargo run --bin aura
```

### Hot Reload & VS Code Launcher

The repository includes a ready-to-use VS Code environment:

- **Hot Reload Launcher (`Ctrl+Shift+B`)**:
  - Run the default build task (`Aura: Hot Reload`) to automatically watch `src/` and `Cargo.toml`.
  - Automatically utilizes `cargo-watch` if installed (`cargo install cargo-watch`).
  - Seamlessly falls back to a native built-in PowerShell watcher on Windows (`scripts/watch.ps1`) if `cargo-watch` is not installed.
  - POSIX runner (`scripts/watch.sh`) provided for macOS / Linux.
- **Run & Debug (`F5`)**:
  - `Debug Aura (CodeLLDB)`: Full LLDB breakpoint debugging.
  - `Debug Aura (Windows MSVC)`: Native MSVC debugger for Windows.
  - `Debug All Tests (CodeLLDB)`: Run and step through tests.


### Running the Test Suite

```bash
# Run all unit tests, load runner tests, and live Postman Echo integration tests
cargo test
```

### Live Postman Echo Endpoints Tested

Integration tests validate live network roundtrips and assertion logic against:
- `GET https://postman-echo.com/get?foo1=bar1&foo2=bar2`
- `POST https://postman-echo.com/post`
- `PUT https://postman-echo.com/put`
- `PATCH https://postman-echo.com/patch`
- `DELETE https://postman-echo.com/delete`

## License

MIT