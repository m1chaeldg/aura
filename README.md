# Aura API Client

A high-performance REST API client engineered in Rust, featuring local workspace management, lossless dual-mode editing, offline sandboxed scripting (`deno_core`), and an Apple-inspired UI design system (`DESIGN.md`).

## Key Features

- **Multi-Tab Document Workspace**:
  - Open multiple simultaneous request tabs with dedicated close buttons (`×`).
  - Dirty state tracking with visual dot indicator (`•`) on tabs with unsaved edits.
  - Persistent `+` button in the tab bar to quickly append new, untitled requests.
  - Globally accessible Command Palette modal (`Cmd/Ctrl + P`) for cross-collection request search without touching the mouse.
- **Responsive Pane Layout & Ergonomics**:
  - Flexible split view with percentage divider adjustment (10% to 90%).
  - Layout orientation toggling between Horizontal (Side-by-Side) and Vertical (Stacked).
  - Smooth drag-to-resize divider with double-click reset to instantly restore 50/50 split (`Cmd/Ctrl + R`).
- **Standardized HTTP Method Badges**:
  - Color-coded badges for visual distinction: GET (Blue: `#0066cc`), POST (Green: `#28a745`), PUT (Amber: `#f5a623`), PATCH (Purple: `#8e44ad`), DELETE (Rose: `#e74c3c`).
- **Power User Keyboard Shortcuts & IDE Presets**:
  - Native keybinding presets: **VS Code**, **IntelliJ IDEA**, and **Sublime Text**.
  - Shortcuts:
    | Action | VS Code | IntelliJ IDEA | Sublime Text |
    |---|---|---|---|
    | **Send Request** | `Ctrl+Enter` | `Ctrl+Enter` | `Ctrl+Enter` |
    | **Command Palette** | `Ctrl+P` | `Shift+Shift` / `Ctrl+Shift+A` | `Ctrl+Shift+P` |
    | **Log Viewer** | `Ctrl+Shift+L` | `Ctrl+Shift+L` | `Ctrl+Shift+L` |
    | **New Request Tab** | `Ctrl+N` / `Ctrl+T` | `Ctrl+Alt+Insert` / `Ctrl+N` | `Ctrl+N` |
    | **Close Tab** | `Ctrl+W` | `Ctrl+F4` / `Ctrl+W` | `Ctrl+W` |
    | **Toggle Layout** | `Ctrl+\` | `Ctrl+\` | `Alt+Shift+2` / `Ctrl+\` |
    | **Reset Divider** | `Ctrl+R` | `Ctrl+R` | `Ctrl+R` |
    | **Save Request** | `Ctrl+S` | `Ctrl+S` | `Ctrl+S` |
    | **Next Tab** | `Ctrl+Tab` | `Alt+Right` | `Ctrl+PageDown` |
    | **Previous Tab** | `Ctrl+Shift+Tab` | `Alt+Left` | `Ctrl+PageUp` |
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