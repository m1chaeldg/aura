# Design Document: Aura Request Execution Engine, Scripting Runtime & Dual-Pane UI Tabs

**Date:** 2026-10-15  
**Topic:** End-to-End Request Dispatch, Structured Request/Response Logging, Deno Scripting Prelude Integration, and Request/Response Tabs UI  
**Target:** `aura` workspace (`crates/aura-scripting`, `crates/aura-engine`, `crates/aura-core`, `crates/aura-ui`)

---

## 1. Intent & The Why

Aura is a high-performance REST API client engineered for speed, privacy, and ergonomics. While the basic shell, multi-tabs, and sidebar tree exist, the core product loop—crafting an API request with headers, body, and scripts, executing it, and inspecting the live response, logs, and test results—remains incomplete. Currently:
1. Clicking **Send** in `RequestHeaderView` only updates a status message string without invoking `ExecutionService` or `aura-engine`.
2. `RequestEditorView` and `ResponsePaneView` are static placeholder stubs without interactive tabs or inputs.
3. `AuraScriptRuntime` in `crates/aura-scripting` does not load `prelude.js`, leaving the zero-dependency `faker` library and Jasmine/Chai testing environment (`describe`, `it`, `expect`) disconnected.
4. Outgoing requests and incoming responses are not comprehensively logged to the rolling system log and live UI Log Viewer.

This project delivers the complete, interactive request lifecycle adhering strictly to the Aura specification (`.aura` format and `AuraRequest` domain model).

---

## 2. Goals, Non-Goals & Anti-Goals

### Goals
- **Runtime Prelude Integration (`crates/aura-scripting`)**:
  - Automatically evaluate `prelude.js` on `AuraScriptRuntime` bootstrap via `include_str!("prelude.js")`.
  - Provide full `faker` (person, internet, datatype, lorem, number, date) and Jasmine/Chai BDD environment (`describe`, `it`, `expect`, `to.deep.equal`, `to.have.property`, `toContain`, etc.).
  - Capture all `console.log`, `console.warn`, and `console.error` calls alongside test assertions (`TestAssertion`) from both synchronous and asynchronous tests (`__aura_drain_tests`).
  - Seamlessly inject response context (`status`, `body`, `headers`, `json()`) for post-response and test scripts.
- **Engine & Execution Service (`crates/aura-engine` & `crates/aura-ui`)**:
  - Full request dispatch pipeline: pre-request scripting -> variable templating -> auth injection -> HTTP execution via `reqwest` -> timing & size metrics -> post-response scripting -> test execution.
  - Asynchronous dispatch on GPUI's background executor with cooperative cancellation (`CancellationToken`).
  - Structured request/response logging using `tracing` emitting to the live log broadcaster and disk appender:
    - Outgoing: HTTP method, URL, headers summary, payload size.
    - Incoming: Status code, duration in ms, response size, and test assertion outcomes.
- **Interactive Request Editor (`RequestEditorView`)**:
  - Sub-tabs: `Params`, `Headers`, `Auth`, `Body`, `Scripts`, `Tests`, `Settings`.
  - `Headers`: Editable key-value table with active/inactive checkboxes, description, and row deletion/addition.
  - `Body`: Format dropdown/segmented control (`None`, `JSON`, `XML`, `Text`, `Raw`, `Form`, `FormUrlEncoded`, `Multipart`, `Binary`) with dedicated editing areas.
  - `Scripts`: Tabbed or dual editor for Pre-Request Script and Post-Response Script.
  - `Tests`: Editor for BDD JavaScript tests with code completion snippets (`describe`, `it`, `expect`).
  - Bi-directional synchronization with the active tab's `AuraRequest`.
- **Interactive Response Pane (`ResponsePaneView`)**:
  - Response header bar: Status pill (color-coded 2xx green, 3xx blue, 4xx amber, 5xx red), latency badge (`duration_ms`), and size indicator (`size_bytes`).
  - Sub-tabs:
    - `Body`: Formatted/Pretty view, Raw view, Copy button.
    - `Headers`: Complete key-value table of response headers.
    - `Test Results`: Pass/fail badge counter (e.g. "3/3 Passed"), list of test suites, test names, duration, and error messages for failures.
    - `Console Logs`: Script execution logs (`INFO`, `WARN`, `ERROR`) with timestamps.

### Non-Goals
- Full GraphQL schema introspection and visual query builder (scheduled for future release).
- Network Interceptor proxy server UI in this milestone.
- WebSocket / SSE streaming connection inspectors in this milestone.

### Anti-Goals
- **No UI Thread Blocking**: Never run `std::fs`, network calls, or Deno script evaluation on the GPUI main thread (`AGENTS.md` rule).
- **No Monolithic Views**: Keep views under ~350 lines by modularizing sub-components (e.g., `HeadersEditor`, `BodyEditor`, `TestResultsView`).
- **No Style Regressions**: Strict compliance with `DESIGN.md` Apple Near-Black tokens and exact badge colors.

---

## 3. Architecture & Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                    GPUI Main Thread                                     │
│  [RequestHeaderView] ──Send Action──► [AuraAppState]                                    │
│         ▲                                   │                                           │
│         │ renders                           │ spawns async task                         │
│         │                                   ▼                                           │
│  [RequestEditorView]                 [ExecutionService]                                 │
│  [ResponsePaneView]                         │                                           │
└─────────────────────────────────────────────┼───────────────────────────────────────────┘
                                              │ (Background Executor)
                                              ▼
                        ┌───────────────────────────────────────────┐
                        │                Aura Engine                │
                        │ 1. Log outgoing request (tracing)         │
                        │ 2. Pre-Request Script (AuraScriptRuntime) │
                        │ 3. Interpolate {{vars}} & apply Auth      │
                        │ 4. Reqwest HTTP Dispatch (timed)          │
                        │ 5. Injected Response into Deno Runtime    │
                        │ 6. Post-Request Script & Tests Run        │
                        │ 7. Drain test assertions & console logs   │
                        │ 8. Log incoming response (tracing)        │
                        └─────────────────────┬─────────────────────┘
                                              │ returns Result<AuraResponse>
                                              ▼
                        ┌───────────────────────────────────────────┐
                        │             GPUI Context                  │
                        │ Update active tab's response & notify UI  │
                        └───────────────────────────────────────────┘
```

---

## 4. Component Breakdown & Specifications

### 4.1. `crates/aura-scripting`
- Embed `prelude.js`:
  ```rust
  const PRELUDE_JS: &str = include_str!("prelude.js");
  ```
- Sandbox Initialization:
  1. Initialize `globalThis.__aura_internal = { logs: [], assertions: [], vars: ... }`.
  2. Execute `PRELUDE_JS` to attach `globalThis.faker`, `globalThis.describe`, `globalThis.it`, `globalThis.expect`, and test runner queue.
  3. Attach `globalThis.console = { log, warn, error }` routing into `__aura_internal.logs`.
  4. Attach `globalThis.aura = { variables, test, expect, response }`.
- Test Drain:
  - Add `__aura_drain_tests()` call at the end of script execution to await asynchronous `it(...)` blocks.
  - Map internal assertions into `Vec<TestAssertion>`.

### 4.2. `crates/aura-engine`
- Outgoing Log:
  ```rust
  tracing::info!(
      target: "aura_engine::request",
      method = %req.method,
      url = %rendered_url,
      headers_count = header_pairs.len(),
      body_type = %req.body.type_name(),
      "Dispatching HTTP request"
  );
  ```
- Incoming Log:
  ```rust
  tracing::info!(
      target: "aura_engine::response",
      status = status,
      duration_ms = duration_ms,
      size_bytes = size_bytes,
      assertions_count = assertions.len(),
      assertions_failed = assertions.iter().filter(|a| !a.passed).count(),
      "Received HTTP response"
  );
  ```
- Response Object Enrichment:
  - Enrich `AuraResponse` with captured `console_logs: Vec<ConsoleMessage>`.

### 4.3. `crates/aura-ui`
- **State Additions (`AuraAppState`)**:
  - `active_request_tab: RequestEditorTab` (`Params`, `Headers`, `Auth`, `Body`, `Scripts`, `Tests`, `Settings`).
  - `active_response_tab: ResponseEditorTab` (`Body`, `Headers`, `Tests`, `Console`).
  - `body_type_selection: BodyTypeKind` (`None`, `Json`, `Xml`, `Text`, `Form`, `Multipart`, `Binary`, `Raw`).
  - `active_body_mode: ResponseBodyMode` (`Pretty`, `Raw`).
- **Request Dispatch Integration (`workspace_view.rs`)**:
  - Connect `RequestHeaderView`'s `on_send` callback to `cx.spawn(|this, mut cx| async move { ... })`.
  - Check in-flight status to toggle Send/Cancel button.
  - Handle success and network errors gracefully with clear user feedback.
- **Modular Views**:
  - `RequestEditorView`: Sub-tab bar and active panel rendering.
  - `ResponsePaneView`: Status badge header, sub-tab bar, and panel rendering (formatted JSON, headers table, assertion cards, console list).

---

## 5. Builder's Decisions (Technical Calls)

1. **`AuraResponse` Enrichment with Console Logs**:
   - Add `#[serde(default)] pub console_logs: Vec<ConsoleMessage>` to `AuraResponse` in `crates/aura-core/src/model.rs`.
   - Both pre-request and post-response logs will be collected into this field so the user can inspect script console output in the Response Pane's "Console Logs" tab.
2. **In-Flight Button State**:
   - When a request is actively in-flight (`is_in_flight == true`), the "Send ↵" button in `RequestHeaderView` transforms into a "Cancel" button styled with `rgb(0xe74c3c)` (Red). Clicking it immediately triggers `execution_service.cancel()`.
3. **JSON Formatting**: Pretty-print JSON bodies in the Response Viewer using `serde_json::to_string_pretty`. If JSON parsing fails, fall back gracefully to raw text display.
4. **Tab Memory**: Store the active sub-tab index in tab-specific metadata or app state so switching between tabs preserves sub-tab selections.
5. **Execution Safety**: If the active document is deleted or closed while a request is in flight, the cancellation token immediately cancels the background HTTP task to prevent memory leaks or dangling state updates.
6. **Zero Main-Thread Network / I/O**: All reqwest networking, script execution, and file reads for binary/multipart payloads remain strictly in `crates/aura-engine` on Tokio worker threads.

---

## 6. Verification Plan

1. **Unit Tests**:
   - `aura-scripting`: Validate `faker` generates valid data, `describe`/`it`/`expect` assertions pass/fail accurately, and `console.log` captures entries.
   - `aura-engine`: Test complete execution of GET/POST requests with headers, body, pre-request variable injection, and post-response assertion parsing.
   - `aura-ui`: Test state transitions for sub-tabs, body type switches, and execution dispatch handling.
2. **Integration Verification**:
   - Dispatch requests to `https://postman-echo.com/post` and `https://postman-echo.com/get`.
   - Verify that outgoing and incoming logs appear in the live log viewer (`Cmd/Ctrl+Shift+L`).
   - Run tests using `cargo check` and `cargo test --workspace`.
