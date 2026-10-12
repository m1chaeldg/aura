---
name: aura-qa
description: Use when conducting quality assurance, end-to-end integration testing against Postman Echo endpoints, or load and memory stability testing for Aura.
---

# Aura QA

Guides Phase 4 Quality Assurance for the Aura API Client. Executes end-to-end integration tests across component boundaries (UI to filesystem, UI to `deno_core`, request engine to network), verifies live HTTP communication against Postman Echo endpoints, and conducts high-concurrency micro-load stability testing.

## When to Use

- Executing Phase 4 Quality Assurance after code review passes Gate 3.
- Running live Postman Echo integration tests across all HTTP verbs.
- Validating Collection Runner high-concurrency performance and memory usage.
- Verifying bidirectional disk synchronization and lossless UI state updates.
- Performing Gate 4 (Functional Sign-Off).

## When NOT to Use

- Writing initial unit tests or business logic implementation—use `aura-developer`.
- Running compiler diagnostics or isolated thread debugging—use `aura-debugger`.
- Packaging release binaries or checking `cargo clippy`—use `aura-devops`.

---

## Testing Mandates

### 1. Live Postman Echo Endpoint Validation
Verify HTTP request dispatch and client capabilities against all 5 mandatory live Postman Echo endpoints:

| Method | Endpoint | Verification Points |
|---|---|---|
| `GET` | `https://postman-echo.com/get?foo1=bar1&foo2=bar2` | Query parameter serialization, headers, status 200, response body parsing |
| `POST` | `https://postman-echo.com/post` | JSON / Form / Raw payload transmission, content-type header, payload echo |
| `PUT` | `https://postman-echo.com/put` | Full payload update, header preservation, response validation |
| `PATCH` | `https://postman-echo.com/patch` | Partial payload update, response body verification |
| `DELETE` | `https://postman-echo.com/delete` | Delete request dispatch, status code, response body confirmation |

Execute these tests via the integration test suite:
```bash
cargo test --test integration_tests -- --nocapture
```

### 2. End-to-End Component Boundary Testing
- **UI <-> Filesystem:** Test file creation, renaming, deletion, and external file modification reloading (hot reloading). Confirm bidirectional sync is lossless.
- **UI <-> Deno Core:** Test pre-request and test scripts executing in the sandboxed runtime. Verify variables set via `pm.environment.set` or `aura.variables.set` propagate to subsequent requests.
- **Authentication Cascading:** Test inheritance cascading from Workspace -> Collection -> Folder -> Request.

### 3. Collection Runner & Load Stability
- Trigger the JMeter-inspired Collection Runner for concurrent iterations.
- Verify micro-load stability under parallel request executions.
- Monitor resident set size (RSS) memory consumption to detect leaks or unbounded buffer growth.

---

## Testing Loop (QA <-> Debugger <-> Dev)

```
[QA Runs Tests & Load] -> [Bug/Leak Detected?]
                                |
             +--- YES ----------+---------- NO ---+
             |                                    |
             v                                    v
[Debugger Isolates Thread]               [Gate 4: PASSED]
             |                                    |
             v                                    v
[Dev Fixes Defect]                       [Proceed to Phase 5]
```

---

## Gate 4 Checklist (Functional Sign-Off)

| Checkpoint | Requirement | Pass Criteria |
|---|---|---|
| **Live Echo Endpoints** | Do all 5 Postman Echo endpoints pass? | GET, POST, PUT, PATCH, DELETE 100% green. |
| **Concurrency & Load** | Does app perform reliably under load? | Zero deadlocks, zero dropped requests. |
| **Memory Stability** | Are there memory leaks or runaway spikes? | Stable memory profile during runner execution. |
| **Bidirectional Sync** | Is UI-to-disk sync completely lossless? | No file corruption, correct dirty-state clearing. |

**Gate 4 Decision:**
- If ANY test fails or memory leaks: Trigger Testing Loop with reproduction steps.
- If ALL tests pass: Mark `[GATE 4: PASSED]` and hand off to `aura-devops`.
