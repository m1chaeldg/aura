# Aura API Client

A high-performance, offline-first desktop API client and load-testing tool built with **Rust**, **GPUI**, **Tokio**, and **`deno_core`**.

Aura stores collections as human-readable, Git-friendly `.aura` TOML files, keeps your data local with zero telemetry, and combines Postman/Bruno-style API workflows with JMeter-grade load testing.

---

## Table of Contents

- [Installation](#installation)
- [Features Overview](#features-overview)
- [User Guide: How to Use Aura](#user-guide-how-to-use-aura)
- [Documentation & Reference](#documentation--reference)
- [Examples: Scripts, Tests, Fakes & Runner](#examples-scripts-tests-fakes--runner)
  - [1. Pre-Request & Post-Response Scripts](#1-pre-request--post-response-scripts)
  - [2. Tests (No-Code Assertions & BDD JavaScript)](#2-tests-no-code-assertions--bdd-javascript)
  - [3. Fakes & Dynamic Test Data (`faker` & `MockServer`)](#3-fakes--dynamic-test-data-faker--mockserver)
  - [4. Collection Runner, Chaining & Load Testing (`aura load`)](#4-collection-runner-chaining--load-testing-aura-load)
- [Keyboard Shortcuts & IDE Presets](#keyboard-shortcuts--ide-presets)
- [Developer Guide](#developer-guide)
  - [Workspace Architecture](#workspace-architecture)
  - [Prerequisites & Platform Setup](#prerequisites--platform-setup)
  - [Building & Running](#building--running)
  - [Hot Reload & Debugging](#hot-reload--debugging)
  - [Running Tests & Linting](#running-tests--linting)
  - [Building & Publishing Releases](#building--publishing-releases)
- [License](#license)

---

## Installation

### Pre-built Binaries (Recommended)

Download the latest archive for your platform from the [GitHub Releases](../../releases) page:

| Platform | Archive | Notes |
|---|---|---|
| **macOS (Apple Silicon)** | `aura-macos-aarch64.tar.gz` | Extract and run `./aura` |
| **macOS (Intel)** | `aura-macos-x86_64.tar.gz` | Extract and run `./aura` |
| **Linux (x86_64)** | `aura-linux-x86_64.tar.gz` | Requires X11/Wayland & Vulkan/Mesa drivers |
| **Windows (x86_64)** | `aura-windows-x86_64.zip` | Extract and run `aura.exe` |

Each release archive includes the `aura` binary, `README.md`, `CHANGELOG.md`, `LICENSE`, and the `docs/` folder (with example `.aura` collections).

---

## Features Overview

- **Multi-Protocol Support**:
  - **HTTP / REST**: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS` with color-coded method badges.
  - **GraphQL**: Dedicated query & variables editor.
  - **WebSockets (`ws://` / `wss://`)**: Live bidirectional connection & message stream.
  - **Server-Sent Events (SSE)**: Streaming `text/event-stream` viewer.
  - **gRPC & SOAP/WSDL**: gRPC request execution and WSDL import into Aura collections.
- **Dual-Mode Request Editor & `.aura` Spec Sync**:
  - Edit requests visually (Params, Headers, Auth, Body, Scripts, Tests, Extractors, Settings) or switch to the **Spec** tab to edit the canonical `.aura` TOML format with live bidirectional sync and syntax error resilience.
  - Body modes: JSON, XML, Text, Form URL-encoded, Multipart, and relative-path Binary uploads.
  - Disable query params, headers, or assertions without deleting them using `~` (e.g. `~key = "value"`).
- **Multi-Tab Workspace & Responsive Layout**:
  - Open multiple request tabs with unsaved dirty indicators (`•`), quick `+` tab creation, and Command Palette (`Ctrl+P` / `Ctrl+K`).
  - Switch between Side-by-Side and Stacked split layouts, drag the divider (10%–90%), or double-click / press `Ctrl+R` to reset to 50/50.
  - Live filesystem watcher automatically refreshes the sidebar when `.aura` files or folders are added, renamed, or deleted externally.
- **Environments, Templating & Security**:
  - Cascading `{{var}}` resolution: **Request → Environment → Collection → OS Environment (`{{$ENV_VAR}}`)**.
  - **Git-safe Secret Management**: Environment `[secrets]` backed by **OS Keychain** (Windows Credential Manager, macOS Keychain, Linux `secret-tool`), **HashiCorp Vault** (KV v1/v2), **AWS Secrets Manager**, and **KeePass (`.kdbx`)** with in-memory TTL caching and fail-fast resolution.
  - Auth schemes: Bearer, Basic, API Key, AWS SigV4 (including `~/.aws/credentials` profiles), OAuth2, mTLS client certificates.
  - Built-in offline **JWT Inspector** and token-boundary **Secret Scrubber** to prevent credentials from leaking into logs.
- **Sandboxed Offline Scripting (`deno_core` V8)**:
  - Run Pre-Request, Post-Response, and Test scripts in an isolated V8 sandbox with zero telemetry.
  - Built-in `faker` generator, `fetch()` / `aura.sendRequest()`, `aura.setNextRequest()` chaining, and `console.log`/`warn`/`error` capture.
- **Zero-JS Native Extractors**:
  - Correlate responses at load-test speed using pure Rust **JSONPath**, **Regex Body**, **Regex Header**, **Header**, and **Cookie** extractors.
- **JMeter-Style Collection Runner & Load Tester**:
  - Run load tests from the **Load Test** toolbar modal or headlessly via `aura load`.
  - Supports ramp-up/ramp-down, soak durations, multi-stage spike profiles, target RPS shaping, uniform/Gaussian think-time, rendezvous barriers, weighted routing, CSV/JSON data feeders, and CI SLA gates (`p95`, `p99`, error rate).
  - Exports JTL-style CSV, JSON, and standalone HTML reports with HDR latency percentiles (`min`, `mean`, `p50`, `p90`, `p95`, `p99`, `max`).
- **Network Tooling, Codegen & Git**:
  - **Cookie Jar**: Automatic `Set-Cookie` persistence and modal cookie manager.
  - **Proxy & MITM Capture Proxy**: Global HTTP/HTTPS/SOCKS5 proxy settings plus a local capture proxy with interceptor rules.
  - **Code Snippet Generator**: Export any request to `cURL`, `fetch`, `axios`, Python, Rust, or Go.
  - **Built-in Git UI**: Inspect `git status`, view diffs, commit, switch branches, and push/pull right from the toolbar.
  - **Importers**: Import Postman Collections (v2.1), Insomnia exports, OpenAPI v2/v3, WSDL, and raw cURL commands.

---

## User Guide: How to Use Aura

1. **Open a Workspace**:
   - Click **Workspace** in the top toolbar to select any folder on your computer.
   - Aura indexes all `.aura` files and environment `.toml` files in that folder and watches for external changes.
2. **Create or Select a Request**:
   - Click a request in the left sidebar, press `+` in the tab bar, or open the **Command Palette** (`Ctrl+P` / `Ctrl+K`) to jump to any request.
3. **Configure & Send**:
   - Choose the HTTP method (`GET`, `POST`, etc.) and enter a URL (e.g., `{{base_url}}/users`).
   - Add query params, headers, auth, body, scripts, or assertions in the request tabs.
   - Press **Send** (or `Ctrl+Enter`). Inspect the response in **JSON**, **Table**, **Tree**, **Headers**, **Raw**, **Tests**, or **Visualize** tabs.
4. **Switch Environments**:
   - Use the **Environment** dropdown on the right side of the top toolbar to switch between profiles (e.g. `dev`, `staging`, `prod`). Variables like `{{base_url}}` update automatically.
5. **Save & Sync**:
   - Press `Ctrl+S` (or rely on debounced auto-save) to write clean `.aura` TOML files to disk. Use the **Git** toolbar button to commit and push your collection.
6. **Tools & Network Utilities**:
   - Secondary tools (Cookie Jar, Global Proxy, MITM Capture Proxy, Security Vaults & mTLS, Load Tester, Settings) live in the **Tools** menu (and **Tools** toolbar button) or Command Palette (`Ctrl+K`), keeping the main toolbar clean while showing live pills whenever Proxy, Capture, or Load Test is running.

---

## Documentation & Reference

- **[Reference Guide](docs/reference.md)**: Complete reference for the UI, keyboard shortcuts, `.aura` TOML spec, JS scripting API, and `aura load` CLI flags/exit codes.
- **[`.aura` File Format](docs/file_format.md)**: Annotated walkthrough of the lossless `.aura` TOML format.
- **[Scripting & Chaining](docs/scripting_and_chaining.md)**: Declarative `[assert]` rules, async `fetch`/`aura.sendRequest`, and `aura.setNextRequest` chaining.
- **[Sample Collections](docs/examples/)**: Ready-to-use `.aura` examples for REST CRUD, GraphQL, and request chaining.

---

## Examples: Scripts, Tests, Fakes & Runner

### 1. Pre-Request & Post-Response Scripts

Aura executes JavaScript in an embedded V8 sandbox (`deno_core`). Both `aura.*` and Postman-compatible `pm.*` APIs are supported.

**Pre-Request: Fetch an auth token asynchronously before sending the main request**

```javascript
const authRes = await fetch("https://auth.example.com/oauth/token", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ grant_type: "client_credentials" })
});

const data = await authRes.json();
if (data.access_token) {
    aura.variables.set("access_token", data.access_token);
}
```

**Post-Response: Send an auxiliary webhook with `aura.sendRequest`**

```javascript
const audit = await aura.sendRequest({
    url: "https://postman-echo.com/post",
    method: "POST",
    headers: { "content-type": "application/json" },
    body: { status: response.status, duration_ms: response.duration_ms }
});

console.log("Audit webhook status:", audit.status);
```

See [docs/scripting_and_chaining.md](docs/scripting_and_chaining.md) for the full scripting API reference.

---

### 2. Tests (No-Code Assertions & BDD JavaScript)

#### Option A: No-Code Declarative Assertions (`[assert]`)

In the **Tests** tab, click **Add Rule** (or use quick preset chips like `Status 200`, `body.id isNumber`, `Time < 500ms`). Rules are saved directly in your `.aura` file:

```toml
[assert]
res.status = "== 200"
res.duration_ms = "< 1500"
res.headers.content-type = "contains application/json"
res.body.id = "isNumber"
res.body.tags = "isArray"
~res.body.debug_flag = "== true"   # Prefix with ~ to temporarily disable
```

#### Option B: BDD JavaScript Tests (`describe` / `it` / `aura.test` / `expect`)

```javascript
describe("User API", () => {
    it("returns 200 OK within 500ms", () => {
        expect(response.status).to.equal(200);
        expect(response.duration_ms).to.be.below(500);
    });

    it("includes a valid user object", () => {
        const body = response.json();
        expect(body).to.have.property("id");
        expect(body.email).to.include("@");
    });
});

// Postman-style syntax is also supported:
aura.test("Content-Type is JSON", () => {
    expect(response.headers["content-type"]).to.include("application/json");
});
```

---

### 3. Fakes & Dynamic Test Data (`faker` & `MockServer`)

#### Built-in `faker` in Scripts

Aura includes a zero-dependency `faker` object in the V8 scripting runtime—no `npm install` required:

```javascript
// Generate realistic fake payloads in a Pre-Request script
const fakeUser = {
    id: faker.datatype.uuid(),
    name: faker.person.fullName(),
    job: faker.person.jobTitle(),
    email: faker.internet.email(),
    username: faker.internet.userName(),
    city: faker.location.city(),
    company: faker.company.name(),
    score: faker.number.int({ min: 1, max: 100 }),
    createdAt: faker.date.recent().toISOString()
};

aura.variables.set("fake_user_json", JSON.stringify(fakeUser));
aura.variables.set("fake_email", fakeUser.email);
```

Supported `faker` namespaces include: `person` / `name`, `internet`, `number`, `datatype`, `random`, `string`, `location` / `address`, `company`, `commerce`, `lorem`, and `date`.

#### Environment & OS Variable Templating

In URLs, headers, query params, and request bodies, use `{{var_name}}` (resolved from Request → Environment → Collection → OS env `{{$VAR_NAME}}`).

#### Secrets, OS Keychain & External Vaults

Aura keeps secrets out of Git by storing local secret values in your **OS Keychain / Credential Manager** (Windows Credential Manager, macOS Keychain, Linux `secret-tool`) and supporting external secret providers:

1. **Environment Secret Variables (Recommended, Git-safe)**:
   - Open the **Environment Editor** from the Environment dropdown.
   - Click **Plain** on a variable row to toggle it to **Secret** (values are masked; use **Show**/**Hide** to inspect).
   - Enter the secret value and click **Save** (or **Keychain**). Aura stores the plaintext value in your OS Keychain (`Aura:env-<env_name>:<KEY>`) and writes only `[secrets]` (`KEY = "keyring"`) to `environment/<name>.toml`.
   - Reference the variable normally in requests as `{{KEY}}`.

2. **OS Keychain Manager (`keyring://service/key`)**:
   - Open **Tools → Security Vaults & mTLS** to store, load, or delete shared entries under **OS KEYCHAIN / CREDENTIAL MANAGER**.
   - Reference them in an environment variable or inline in any request field with `{{keyring://aura/my_key}}`.

3. **External Secret Providers (`vault://`, `aws-sm://`, `keepass://`)**:
   - Configure named profiles in **Tools → Security Vaults & mTLS** (Vault tokens and KeePass master passwords are stored in your OS Keychain, never plaintext preferences):
     - **HashiCorp Vault (KV v1/v2)**: `vault://[profile@]secret/data/app#api_key`
     - **AWS Secrets Manager**: `aws-sm://[profile@]prod/db#password`
     - **KeePass (`.kdbx`)**: `keepass://[profile@]Group/EntryTitle#Password` (defaults to `Password` if `#Field` is omitted)
   - Use any URI as an environment variable value or inline (e.g. `Authorization: Bearer {{vault://prod@secret/data/api#token}}`).
   - Resolved secrets are cached in memory (default 300s TTL; use **Clear Secret Cache** in the Security modal to flush immediately), fail fast on resolution errors, and are automatically redacted from logs by `SecretScrubber`.

#### Local `MockServer` (Fakes for Offline Testing)

`aura-engine` includes a lightweight local HTTP `MockServer` (`crates/aura-engine/src/mock.rs`) for spinning up deterministic fake endpoints in Rust tests or local integration harnesses:

```rust
use aura_engine::mock::{MockRoute, MockServer};
use std::collections::HashMap;

let server = MockServer::new(18080);
server.add_route(MockRoute {
    method: "GET".into(),
    path: "/api/users/1".into(),
    status: 200,
    headers: HashMap::new(),
    body: r#"{"id": 1, "name": "Ada Lovelace"}"#.into(),
});
server.run().await?;
```

---

### 4. Collection Runner, Chaining & Load Testing (`aura load`)

#### Request Chaining (`aura.setNextRequest`)

Control execution order dynamically in post-response scripts during a Collection Run:

```javascript
if (response.status === 200) {
    const data = response.json();
    aura.variables.set("auth_token", data.token);
    aura.setNextRequest("02 Fetch User Profile");
} else {
    aura.setNextRequest("03 Fallback Auth Error");
}
// Stop the iteration cleanly: aura.setNextRequest(null);
```

See the ready-to-import example collection in [docs/examples/chaining/](docs/examples/chaining/).

#### UI Load Tester

Click **Load Test** in the top toolbar to configure Virtual Users (VUs), iterations or duration, ramp-up/ramp-down, spike stages, target RPS, think-time, CSV/JSON data feeders, and SLA thresholds, and view live aggregate metrics.

#### Headless CLI Load Runner (`aura load`)

Run load tests in CI/CD pipelines without launching the GUI:

```bash
# Basic 10 VUs for 30 seconds against a collection
aura load ./my-collection --env ./env/staging.toml --vus 10 --duration 30

# Multi-stage spike profile with CSV data feeder, CI SLA gates, and HTML report
aura load ./my-collection \
  --stages 10:30,50:60,10:30 \
  --target-rps 200 \
  --think-uniform 100:300 \
  --data ./users.csv --sharing all --eof recycle \
  --sla-p95 250 \
  --sla-error 1.0 \
  --export-html ./report.html \
  --export-csv ./samples.csv \
  --export-json ./report.json
```

Run `aura load --help` to see all supported flags (`--ramp-up`, `--ramp-down`, `--think-gaussian`, `--sync-barrier`, `--weighted`, `--max-duration`, etc.).

---

## Keyboard Shortcuts & IDE Presets

Aura supports **VS Code**, **IntelliJ IDEA**, and **Sublime Text** keybinding presets:

| Action | VS Code | IntelliJ IDEA | Sublime Text |
|---|---|---|---|
| **Send Request** | `Ctrl+Enter` | `Ctrl+Enter` | `Ctrl+Enter` |
| **Command Palette** | `Ctrl+P` / `Ctrl+K` | `Shift+Shift` / `Ctrl+Shift+A` | `Ctrl+Shift+P` |
| **Log Viewer** | `Ctrl+Shift+L` | `Ctrl+Shift+L` | `Ctrl+Shift+L` |
| **New Request Tab** | `Ctrl+N` / `Ctrl+T` | `Ctrl+Alt+Insert` / `Ctrl+N` | `Ctrl+N` |
| **Close Tab** | `Ctrl+W` | `Ctrl+F4` / `Ctrl+W` | `Ctrl+W` |
| **Toggle Layout** | `Ctrl+\` | `Ctrl+\` | `Alt+Shift+2` / `Ctrl+\` |
| **Reset Divider** | `Ctrl+R` | `Ctrl+R` | `Ctrl+R` |
| **Save Request** | `Ctrl+S` | `Ctrl+S` | `Ctrl+S` |
| **Next Tab** | `Ctrl+Tab` | `Alt+Right` | `Ctrl+PageDown` |
| **Previous Tab** | `Ctrl+Shift+Tab` | `Alt+Left` | `Ctrl+PageUp` |

---

## Developer Guide

### Workspace Architecture

Aura follows the **Service-Store & Modular Component** pattern defined in [AGENTS.md](./AGENTS.md):

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

| Crate | Purpose | Key Dependencies |
|---|---|---|
| [`aura-core`](./crates/aura-core) | Domain models, templating, assertions, extractors, data feeder, security, design tokens | `serde`, `chrono`, `regex` (Zero GUI / async) |
| [`aura-parser`](./crates/aura-parser) | Lossless CST parser & canonical `.aura` TOML serializer | `nom`, `aura-core` |
| [`aura-scripting`](./crates/aura-scripting) | Sandboxed offline JavaScript / V8 runtime (`faker`, `expect`, `fetch`) | `deno_core`, `aura-core` |
| [`aura-engine`](./crates/aura-engine) | Async HTTP/GraphQL/WS/SSE/gRPC dispatcher, load runner, reporting, mock server | `reqwest`, `tokio`, `aura-core`, `aura-scripting` |
| [`aura-sync`](./crates/aura-sync) | Two-way TOML synchronization, Git integration, & format importers | `toml`, `aura-core`, `aura-parser` |
| [`aura-ui`](./crates/aura-ui) | GPUI views, domain stores, async services | `gpui`, `gpui-kit`, `aura-core`, `aura-engine`, `aura-sync` |
| `aura` (Root) | Facade library, desktop GUI entrypoint, and `aura load` CLI | Re-exports all domain crates |

---

### Prerequisites & Platform Setup

- **Rust**: 1.80+ (via [rustup](https://rustup.rs/))
- **Windows**: Visual Studio Build Tools (MSVC C++ workload) — `build.rs` automatically links `advapi32` for V8/ETW.
- **macOS**: Xcode Command Line Tools (`xcode-select --install`).
- **Linux (Ubuntu/Debian)**: Install GPUI & windowing system dependencies:
  ```bash
  sudo apt-get update && sudo apt-get install -y \
    pkg-config libxkbcommon-dev libxkbcommon-x11-dev \
    libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libwayland-dev libfontconfig1-dev libfreetype6-dev libvulkan-dev \
    libx11-dev libxcursor-dev libxi-dev libxrandr-dev libdbus-1-dev
  ```

---

### Building & Running

```bash
# Debug build
cargo build

# Optimized release binary
cargo build --release --bin aura

# Run the desktop GUI
cargo run --bin aura

# Run the headless load CLI
cargo run --bin aura -- load --help
```

> **Note for Windows Developers**: `.cargo/config.toml` sets `jobs = 2` and `pipelining = false` to avoid Windows virtual memory (`os error 1455`) contention when linking V8 (`deno_core`) and `gpui`.

---

### Hot Reload & Debugging

The repository includes a ready-to-use VS Code configuration:

- **Hot Reload Launcher (`Ctrl+Shift+B`)**:
  - Runs the `Aura: Hot Reload` task watching `src/` and `Cargo.toml`.
  - Uses `cargo-watch` if installed (`cargo install cargo-watch`), falling back to `scripts/watch.ps1` on Windows or `scripts/watch.sh` on macOS/Linux.
- **Run & Debug (`F5`)**:
  - `Debug Aura (CodeLLDB)`: Full LLDB breakpoint debugging.
  - `Debug Aura (Windows MSVC)`: Native MSVC debugger for Windows.
  - `Debug All Tests (CodeLLDB)`: Step through workspace tests.

---

### Running Tests & Linting

```bash
# Run all unit, integration, and load runner tests across the workspace
cargo test --workspace --tests -j 2

# Run tests for an individual crate
cargo test -p aura-core
cargo test -p aura-parser
cargo test -p aura-scripting
cargo test -p aura-engine
cargo test -p aura-sync
cargo test -p aura-ui

# Formatting & strict Clippy lints
cargo fmt -- --check
cargo clippy --workspace -- -D warnings

# Build Rustdoc documentation
cargo doc --workspace --no-deps
```

---

### Building & Publishing Releases

GitHub Actions (`.github/workflows/release.yml`) automatically builds, packages, and publishes releases for **macOS (Apple Silicon & Intel)**, **Linux (x86_64)**, and **Windows (x86_64)**:

1. Update `version` in `Cargo.toml` and move release items from `## [Unreleased]` to `## [X.Y.Z]` in `CHANGELOG.md`.
2. Commit and push a version tag:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. You can also trigger a build manually from the GitHub **Actions → Build & Release → Run workflow** button.

---

## License

MIT — see [LICENSE](./LICENSE).
