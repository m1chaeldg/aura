---
name: aura-reviewer
description: Use when performing code review or compliance auditing on Aura pull requests, branches, or code changes against architectural and project constraints.
---

# Aura Reviewer

Guides Phase 3 Code Review & Compliance Auditing for the Aura API Client. Evaluates passing code against 6 invariant project constraints: zero telemetry, relative path anchoring, `deno_core` sandbox security, lean error handling, `DESIGN.md` compliance, and HTTP integration testing coverage.

## When to Use

- Auditing code after Phase 2 (Gate 2) has passed.
- Conducting compliance checks on pull requests or feature branches.
- Verifying Phase 3 / Gate 3 compliance before QA testing.

## When NOT to Use

- Implementing fixes or writing code—use `aura-developer`.
- Running test executions or debugging panics—use `aura-debugger`.
- Executing end-to-end load tests or live Postman Echo tests—use `aura-qa`.

---

## Gate 3 Compliance Check (The 6 Invariant Rules)

Every review must rigorously audit against these 6 non-negotiable rules:

### 1. Telemetry Eradication
- Verify that NO telemetry, analytics, tracking, usage reporting, or metrics beaconing exists in the codebase.
- Audit dependencies for hidden analytics or beacon libraries.
- Ensure all logging stays local via `tracing-appender` rolling log files.

### 2. Relative Path Anchoring
- Verify all filesystem operations use relative path anchoring rooted in the active workspace directory.
- Forbid hardcoded absolute paths (`/Users/...`, `C:\...`, `/home/...`).
- Ensure paths operate cross-platform (Windows, macOS, Linux).

### 3. Deno Core Security & Offline Sandboxing
- Ensure the JavaScript scripting engine strictly uses `deno_core` (never `rquickjs`).
- Verify that runtime scripts run completely offline with NO network primitives exposed or leaking.
- Sandbox script execution from unauthorized filesystem modifications.

### 4. Lean Error Handling
- Prevent defensive bloat: avoid wrapping every simple operation in multi-layered custom error hierarchies if standard idiomatic error propagation suffices.
- Ensure errors provide clear, user-actionable context without leaking internal secrets or memory addresses.

### 5. UI Design System (`DESIGN.md`) Compliance
- Compare all GPUI elements, styling tokens, margins, colors, and typography against `DESIGN.md`.
- Verify method badge colors: GET (Blue), POST (Green), PUT (Amber), PATCH (Purple), DELETE (Rose).
- Check dirty-state indicator behavior on tabs and responsive split view ergonomics.

### 6. HTTP Integration Test Coverage
- Verify that integration tests are implemented for all HTTP request workflows.
- Confirm tests exercise client workflows against the mandatory live Postman Echo endpoints.

---

## Review Output Format

Provide a structured review report:

```markdown
## Gate 3 Compliance Audit: [Feature/PR Name]

1. [PASS/FAIL] Telemetry Eradication: [Notes]
2. [PASS/FAIL] Relative Path Anchoring: [Notes]
3. [PASS/FAIL] Deno Core Sandbox & Offline Security: [Notes]
4. [PASS/FAIL] Lean Error Handling: [Notes]
5. [PASS/FAIL] UI Design System (DESIGN.md): [Notes]
6. [PASS/FAIL] HTTP Integration Test Coverage: [Notes]

**Verdict:** [PASSED / CHANGES REQUESTED]
```

**Gate 3 Decision:**
- If ANY item is marked FAIL: Reject and return to `aura-developer` with mandatory revision notes.
- If ALL 6 items pass: Mark `[GATE 3: PASSED]` and hand off to `aura-qa`.
