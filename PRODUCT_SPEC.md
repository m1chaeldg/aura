# Aura Product Specification

**Description:** A high-performance REST API client featuring built-in workspace management and advanced request scripting capabilities, designed to streamline API development, testing, and debugging.

## Core Architecture

* **Primary Language:** Rust
* **UI Framework:** GPUI
* **Component Library:** `gpui-kit` (utilized for multi-tab views, code editors, and enforcing a consistent design language across the application)
* **Data Persistence:** Local file system-first approach with two-way synchronization.
* **Scripting Runtime:** Sandboxed backend `deno_core` V8 runtime for fast, lightweight script execution without bloat (strictly offline execution, zero telemetry).
* **Logging & Diagnostic Subsystem:** Asynchronous, non-blocking structured logging architecture (`tracing` / `tracing-appender`). Writes rolling logs directly to the local filesystem with configurable rotation, zero telemetry, and real-time event broadcasting to the UI layer.

## Workspace & User Interface

**Multi-Tab Document Workspace**

* **Tab Management:** Supports opening multiple requests in simultaneous tabs, each with a dedicated close button.
* **Dirty State Tracking:** Displays a visual dot indicator on tabs containing unsaved edits.
* **New Request Quick Action:** A persistent `+` button that instantly appends a new, untitled request to the active collection.
* **Command Palette:** A globally accessible modal for quick searching and executing API requests across all collections without utilizing the mouse.

**Responsive Pane Layout**

* **Adjustable Dividers:** Flexible split view allowing users to adjust the exact percentage between the Request Pane and the Response Pane.
* **Layout Orientation:** Toggle between Vertical (Stacked) or Horizontal (Side-by-Side) layout views.
* **Divider Resizer:** Smooth drag-to-resize divider separating the request and response panes, featuring a double-click reset to instantly return to a 50/50 split.

**HTTP Method Badges**
Requests are visually distinct using standardized color coding: GET (Blue), POST (Green), PUT (Amber), PATCH (Purple), DELETE (Rose).

**Power User Ergonomics**

* **Keyboard Shortcuts:** Comprehensive keybindings for all major actions (e.g., `Cmd/Ctrl + Enter` to send, `Cmd/Ctrl + P` for Command Palette, `Cmd/Ctrl + Shift + L` for App/System Log Viewer).
* **Keybinding Presets:** Native support for popular IDE and code editor keymap profiles—including VS Code, IntelliJ IDEA, and Sublime Text—allowing developers to navigate the editors using their existing muscle memory.

## Version Control, File Management & Interoperability

**Workspace Synchronization & Operations**

* **Disk-Synced & Hot Reloading:** The sidebar mirrors the actual folder structure on the local disk. UI changes automatically save, and the app detects/reloads changes made externally.
* **Standard CRUD:** Full capabilities via UI or context menu (Create, Rename, Delete, Move, Duplicate/Clone).
* **Drag and Drop:** Reorganize the workspace intuitively across collections, folders, subfolders, and requests.
* **Git Integration:** Built-in Git sync for standard commands (commit, push, pull, branching, conflict resolution).

**Interoperability (Import & Export)**

* **Collection Importers:** Native support for importing Postman Collections (v2.1), Insomnia exports, and standard OpenAPI (Swagger) v2/v3 specifications.
* **cURL Import/Paste:** Ability to paste a raw cURL command directly into the UI to automatically parse it into a new request (mapping headers, body, method, etc.).

## Security & Advanced Authentication

* **Protocols Supported:** OAuth (1.0/2.0), Bearer Token, Basic Auth, AWS Signature V4, API Key, and Digest Auth.
* **AWS Profile Integration:** Native support for utilizing local AWS Profiles (e.g., `~/.aws/credentials`) directly for AWS Signature V4 authentication, eliminating manual key entry.
* **Cascading Inheritance:** Auth configurations cascade from Workspace -> Collection -> Folder -> Request.
* **External Secrets Management:** Native runtime integration with OS-level managers (macOS Keychain, Windows Credential Manager, Linux Secret Service) and cloud vaults (AWS Secrets Manager, HashiCorp Vault).
* **Automatic Token Refresh:** Native support for OAuth 2.0 PKCE and automatic background token refreshing when a token expires.
* **JWT Inspector:** A built-in JWT decoder that automatically detects JWTs in headers or responses, parses the base64 payload, and displays the decoded claims and expiration time on hover.

## Request Engine & Editor Features

**Dual-Mode Request Editor**

* **Code Editor:** A raw text editor utilizing TOML configuration combined with fenced/code blocks.
* **Lossless Bidirectional Sync:** Edits made in the Visual Editor seamlessly and instantly update the Code Editor (and vice versa) without state divergence.

**Visual Request Editor Details**

* **Params Tab:** Key-value query parameter table with checkboxes. Disabled parameters serialize with a trailing tilde (`key~ = "value"`). Bi-directional URL synchronization.
* **Headers Tab:** Key-value table with active/inactive toggles. Common Header Presets dropdown for fast insertion.
* **Variables Tab:** Request-scoped variable table overriding collection, environment, and workspace variables at execution time.
* **Body Tab:**
  * *Formats:* `none`, `json` / `xml` (CodeMirror with formatting utilities), `text`, `form-urlencoded`, `multipart`, and `binary` (drag-and-drop, relative path anchoring).
  * *Toolbar:* Format/Prettify, Compress/Minify, Copy, with transient feedback badges.
* **Scripts Tab (Pre-Request & Post-Response):** Executes Sandboxed JavaScript targeting `deno_core`. Full Web APIs supported (`fetch`, `crypto.subtle`, etc.).
* **Tests Tab:** BDD-style JavaScript test suite editor utilizing `aura.test(...)` syntax.
* **Settings Tab:** Granular network toggles (`inherit_auth`, `verify_ssl`, `follow_redirects`, `max_redirects`, `timeout_ms`).

**Advanced Protocols**

* **GraphQL Schema Introspection:** Fetches the GraphQL schema from the endpoint to provide rich autocomplete and linting for queries.
* **Streaming & Persistent Connections:** Future-proofed architectural support for WebSockets (WS/WSS), Server-Sent Events (SSE), and gRPC.

## Environment, Configuration & Templating

**Variables & Dynamic Templating**

* **Handlebars Syntax:** Universal support for `{{variable_name}}` template replacement across URLs, query parameters, headers, and body payloads.
* **OS Environment Variables:** Native resolution of underlying operating system environment variables (e.g., `{{$OS_USER}}`) directly within the templating engine and `deno_core` scripts.

**Environment Profiles Manager**

* **Profile Management:** Create, rename, duplicate, and delete environment profiles (e.g., Development, Staging, Production, Local). Activating a profile triggers immediate workspace-wide reactivity.
* **Key-Value Variable Grid:** Manage variables via a dedicated grid featuring key name, variable value, and an enabled checkbox.
* **Cross-Environment Variable Transfer:** Single-click action to copy or promote a variable to another environment profile easily.
* **Secret Masking & Storage:** Mark variables as secrets to automatically mask their values. Integrates securely with OS Enclave storage.
* **Keyring Masking Indicator:** Provides visual confirmation that sensitive secrets are actively guarded by the token-boundary secret scrubber.

## API Interceptor & Proxy

* **Network Interceptor:** A built-in proxy server capability to capture HTTP/HTTPS traffic from web browsers or mobile emulators, funneling requests directly into Aura for inspection and saving.
* **Request Rewriting:** Ability to modify intercepted requests on the fly (e.g., swapping headers, redirecting production domains to `localhost`).

## Mock Servers & Prototyping

* **Local Mock Servers:** Spin up a local mock server directly from a collection. Allows frontend developers to build against the API before the backend is fully implemented.
* **Dynamic Mock Responses:** Utilize `deno_core` scripts to generate dynamic mock responses (e.g., returning randomized UUIDs, sequential timestamps, or conditional payloads based on incoming request parameters).

## Script Debugging, Logging & System Console

* **Developer Console:** A dedicated log panel at the bottom of the screen to capture `console.log()`, `console.warn()`, and `console.error()` outputs from the Pre-Request and Post-Response `deno_core` scripts.
* **Variable Inspector:** A runtime view showing the exact evaluated state of all variables (OS + Environment + Collection + Local) at the exact moment of request execution.
* **Application & System Log Viewer:**
  * **Dedicated Diagnostic Console:** Built-in bottom dock / tabbed viewer to inspect application runtime logs, background worker activities, and server subsystem events.
  * **Real-Time Server Log Tailing:** Live log tailing for internal server processes—including the local mock server, network interceptor proxy, and file synchronization engine—with an auto-scroll lock toggle.
  * **Colored Syntax Highlighting:** Visual differentiation and syntax highlighting adhering to `DESIGN.md` color tokens for log levels (`TRACE` in Muted Ink, `DEBUG` in Secondary Ink, `INFO` in Action Blue, `WARN` in Amber, `ERROR` in Rose/Red), ISO 8601 timestamps, thread/component targets, and stack traces.
  * **Log Filtering & Search:** Search bar supporting text and regex matches, quick level filter chips (`ALL`, `INFO`, `WARN`, `ERROR`, `DEBUG`, `TRACE`), and subsystem source selectors (`All`, `Engine`, `Deno Runtime`, `Mock Server`, `Interceptor`, `File Sync`).
  * **File Logging & Storage Controls:** Direct persistence to rolling log files on the local filesystem (`aura.log` with daily and size-based rotation). Integrated toolbar action to "Reveal in File Manager", "Open in External Editor", "Clear Display", and "Export Log Slice".

## Collection Runner & Statistical HDR Profiler

Designed for both functional test execution and JMeter-inspired high-concurrency micro-load testing directly within the app.

**Execution Parameters & Data Input**

* **Target Collection:** Run the entire workspace or filter down to a specific collection/folder.
* **Iterations & Concurrency:** Configurable total execution count and simulated concurrent worker pool for load testing.
* **Delay (Pacing):** Configurable delay between requests in milliseconds.
* **Parameterized Test Data:** Integrated JSON and CSV editor to map data rows to request iterations.

**Statistical HDR Metric Cards & Logging**

* **Throughput & Status:** Real-time requests per second (RPS) tracking, success/failure counts, and calculated Error Rate percentage.
* **Latency Percentile Profiler:** Granular metric tracking displaying Min, Mean, p50 (Median), p90, p95, p99, and Max latencies.
* **Real-Time Execution Log & Filter:** Live tabular feed displaying the iteration number, request name, HTTP method, status code, exact latency, and assertion pass/fail badge. Filterable by All, Passed, or Failed.

## Observability & API Monitoring

* **Scheduled Runs:** Ability to run specific collections on a localized cron schedule to monitor endpoint health and uptime automatically.
* **Webhook Alerts:** Send notifications to external services (Slack, Discord, Teams, etc.) if a scheduled collection run fails its assertions or exceeds latency thresholds.
* **Application Logging & Diagnostics:** Comprehensive local file-backed logging with real-time tailing of server and engine subsystems. Enforces the strict eradication of telemetry and remote tracking.

## Response Viewer & Inspector

**Metrics & Status Badge**

* **Status Pill:** Color-coded HTTP status indicators: 2xx Green, 3xx Blue, 4xx Amber, 5xx Red.
* **Telemetry & Metrics:** Exact latency measurement in milliseconds (`durationMs`), dynamic response payload size formatting (B, KB, MB), and execution timestamp.

**Body Inspection Modes**

* **JSON View:** Syntax-highlighted viewer with an integrated interactive collapsible tree view. Supports copying JSON path expressions and node values. Includes visual type badges.
* **Table View:** Automatically detects tabular JSON arrays. Features column sorting, search filtering, pagination, and data export to CSV or JSON.
* **Visualizer:** Sandboxed `<iframe>` rendering for visual HTML output, SVGs, or custom visualizer templates.
* **HTML, XML & Raw Views.**

**Response Headers & Tools**

* Key-value list of returned HTTP response headers with a search/filter bar and single-click copy.
* **Tools Menu:** Dedicated menu to download the response body (JSON/TXT format), copy the response body directly, or clear the current response view.

**Test & Assertion Results**

* Summary badge displaying total passed vs. failed assertions.
* Individual assertion status cards detailing error traces, expected vs. actual values, and execution duration.

**Response Timeline**

* Historical execution timeline across the active session displaying sequential timestamps, status codes, latencies, and payload sizes.

## Global Search & History

* **Global History:** A dedicated "History" tab in the sidebar that logs all executed requests globally across the application, allowing users to rapidly restore previous sessions or closed requests.
* **Global Find and Replace:** Ability to search for specific headers, URLs, text snippets, or variable names across the entire workspace, making bulk refactoring easy.
