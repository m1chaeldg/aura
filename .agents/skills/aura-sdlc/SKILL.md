---
name: aura-sdlc
description: Use when managing the end-to-end software development lifecycle for Aura, orchestrating multi-role development workflows across architecture, implementation, code review, QA, DevOps, and documentation.
---

# Aura SDLC Orchestrator

Master SDLC Orchestrator for the Aura API Client. Manages an autonomous, closed-loop software development lifecycle by dynamically assuming specialized roles (Architect, Developer, Debugger, Code Reviewer, QA, DevOps, Technical Writer) and enforcing strict progression through validation Gates and iterative Loops.

## When to Use

- Orchestrating a full software development lifecycle for new Aura features, refactors, or bugfixes.
- Coordinating sequential or autonomous role handoffs between Architecture, Development, Review, QA, DevOps, and Documentation.
- Enforcing project-wide gates (design compliance, clean builds, zero telemetry, Postman Echo integration tests).

## When NOT to Use

- Micro-tasks (single typo fixes, trivial 1-line edits) that do not require multi-phase ceremony.
- Isolated tasks that only require a single phase—use the dedicated role skill directly instead:
  - `aura-architect`: Architecture & module scaffolding
  - `aura-developer`: TDD implementation
  - `aura-debugger`: Diagnostics, panic tracing & compiler errors
  - `aura-reviewer`: Gate 3 compliance checks
  - `aura-qa`: Postman Echo E2E testing & load stability
  - `aura-devops`: CI verification, clippy, fmt & worktree management
  - `aura-tech-writer`: Documentation, changelog & rustdoc

---

## 1. Global Operating Constraints

All phases and roles must strictly adhere to these invariant project constraints:

1. **Workspace & Worktree Lifecycle:**
   - All file mutations, builds, and test executions MUST occur strictly within an isolated Git worktree (`workgittree`).
   - Once implementation is complete and verified across all gates, merge changes into `main` and clean up the worktree.
2. **UI Design System:**
   - Always consult and strictly adhere to `DESIGN.md` for any UI-related work, including GPUI component design, visual styling, typography, colors, and layout.
3. **Execution & Integration Testing Mandate:**
   - Code is never assumed to work without empirical execution.
   - You must compile, run, test, and debug the application in reality before passing any gate.
   - Integration tests are mandatory for all HTTP request and client workflows against live Postman Echo endpoints:
     - `GET https://postman-echo.com/get?foo1=bar1&foo2=bar2`
     - `POST https://postman-echo.com/post`
     - `PUT https://postman-echo.com/put`
     - `PATCH https://postman-echo.com/patch`
     - `DELETE https://postman-echo.com/delete`
4. **Engine & Telemetry:**
   - Enforce `deno_core` (never `rquickjs`).
   - Runtime scripts must execute strictly offline without network leaking.
   - Eradicate any telemetry or tracking.
5. **Tooling & MCP Integration:**
   - Utilize filesystem MCP tools for reading and writing files.
   - Utilize terminal/shell MCP tools for executing `cargo run`, `cargo test`, `cargo clippy`, and `cargo fmt`.
   - Utilize rust-analyzer for borrow-checker validation.

---

## 2. Pipeline: Roles, Loops, and Gates

```
+-----------------------------------------------------------+
| Phase 1: Architecture & Scaffolding (Architect)            |
| -> Gate 1: Design Review                                  |
+-----------------------------------------------------------+
                             |
+----------------------------v------------------------------+
| Phase 2: Implementation & Debugging (Developer + Debugger)|
|   [Implementation Loop: Dev <-> Debugger]                 |
| -> Gate 2: Compilation & Test Pass (0 warnings)           |
+-----------------------------------------------------------+
                             |
+----------------------------v------------------------------+
| Phase 3: Code Review (Code Reviewer)                      |
| -> Gate 3: Compliance Check (6 Invariant Rules)            |
+-----------------------------------------------------------+
                             |
+----------------------------v------------------------------+
| Phase 4: Quality Assurance (QA)                           |
|   [Testing Loop: QA <-> Debugger <-> Dev]                 |
| -> Gate 4: Functional Sign-Off (Live Postman Echo)        |
+-----------------------------------------------------------+
                             |
+----------------------------v------------------------------+
| Phase 5: Build & Release (DevOps)                         |
| -> Gate 5: Release Ready (clippy, fmt, size audit)        |
+-----------------------------------------------------------+
                             |
+----------------------------v------------------------------+
| Phase 6: Documentation & Completion (Tech Writer + DevOps)|
| -> Gate 6: Final Audit & Main Merge                       |
+-----------------------------------------------------------+
```

### Phase 1: Architecture & Scaffolding (Role: Architect)
- **Action:** Ingests specifications and consults `DESIGN.md`. Designs state management (GPUI), runtime boundaries (`deno_core`), and data structures. Creates empty trait definitions and scaffolding.
- **Gate 1 (Design Review):**
  - Does architecture follow KISS?
  - Are module boundaries clear?
  - Does it avoid premature abstraction?
  - Does UI design strictly adhere to `DESIGN.md`?
  - *Outcome:* If NO -> Loop back to Architect. If YES -> Proceed to Phase 2.

### Phase 2: Implementation & Debugging (Roles: Developer + Debugger)
- **Action (Dev):** Executes strict TDD. Writes unit tests and integration tests first. Implements GPUI components (strictly adhering to `DESIGN.md`) and Rust logic to pass the tests. Adheres to hybrid DRY & KISS.
- **Action (Debugger):** Compiles code with `cargo build`. Runs tests using `cargo test` (unit and integration). Executes binary. Captures stack traces, Tokio deadlocks, or GPUI rendering panics.
- **Implementation Loop:** Dev writes code -> Debugger runs/tests -> Debugger feeds errors back to Dev -> Dev refactors.
- **Gate 2 (Compilation & Test Pass):**
  - Does code compile with zero warnings?
  - Do all unit and integration tests pass?
  - *Outcome:* If NO -> Continue Implementation Loop. If YES -> Proceed to Phase 3.

### Phase 3: Code Review (Role: Code Reviewer)
- **Action:** Analyzes passing code against project constraints.
- **Gate 3 (Compliance Check):**
  1. Is telemetry entirely absent?
  2. Are all paths using Relative Path Anchoring?
  3. Is `deno_core` used securely without network leaking?
  4. Is error handling lean (no over-defensive bloating)?
  5. Do all UI elements strictly comply with `DESIGN.md`?
  6. Are integration tests implemented for HTTP request workflows?
- *Outcome:* If NO -> Send back to Dev with mandatory revision notes. If YES -> Proceed to Phase 4.

### Phase 4: Quality Assurance (Role: QA)
- **Action:** Executes end-to-end integration tests across component boundaries (e.g., UI to filesystem, UI to `deno_core`). Verifies HTTP request dispatch and live integration against Postman Echo endpoints (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`). Triggers Collection Runner to verify high-concurrency micro-load stability and memory usage.
- **Testing Loop:** QA identifies request execution bugs, memory leaks, or concurrency race conditions -> Debugger isolates thread -> Dev fixes.
- **Gate 4 (Functional Sign-Off):**
  - Do all integration tests (including Postman Echo live endpoints) pass reliably?
  - Does app perform reliably under load without memory spikes?
  - Are bidirectional UI syncs lossless?
  - *Outcome:* If NO -> Trigger Testing Loop. If YES -> Proceed to Phase 5.

### Phase 5: Build & Release (Role: DevOps)
- **Action:** Orchestrates local CI pipeline. Runs `cargo clippy -- -D warnings` and `cargo fmt -- --check`. Audits dependencies for bloat. Packages optimized release binary.
- **Gate 5 (Release Ready):**
  - Is release binary built successfully without bloated dependencies?
  - *Outcome:* If NO -> Return to Dev to prune dependencies. If YES -> Proceed to Phase 6.

### Phase 6: Documentation & Lifecycle Completion (Roles: Technical Writer + DevOps)
- **Action (Tech Writer):** Generates and updates documentation based on finalized code. Writes comprehensive Rustdoc comments for new modules. Updates `CHANGELOG.md` under `[Unreleased]`. Updates `README.md` with new features or CLI commands.
- **Action (DevOps):** Verifies docs, commits changes, merges feature branch into `main`, and removes the worktree.
- **Gate 6 (Final Audit & Integration):**
  - Are docs perfectly aligned with compiled code?
  - Is all work verified?
  - *Outcome:* If YES -> Commit, merge to `main`, clean up worktree, conclude cycle.

---

## 3. Orchestration State Transitions

When orchestrating the pipeline, emit explicit state transition markers so progress is traceable:

```
[INITIATING SDLC: Feature Name]
[ROLE: Architect] -> Scaffolding modules (referencing DESIGN.md for UI)...
[GATE 1: PASSED]
[ROLE: Dev] -> Writing tests & implementation...
[LOOP: Debugger] -> cargo test failed. Refactoring...
[GATE 2: PASSED]
[ROLE: Code Reviewer] -> Auditing Gate 3 compliance...
[GATE 3: PASSED]
[ROLE: QA] -> Executing live Postman Echo integration & load tests...
[GATE 4: PASSED]
[ROLE: DevOps] -> Running clippy, fmt, and release build...
[GATE 5: PASSED]
[ROLE: Technical Writer] -> Updating Rustdocs, CHANGELOG, and README...
[GATE 6: PASSED]
[ROLE: DevOps] -> Merging into main and cleaning up worktree...
[SDLC COMPLETE] -> Documentation updated, merged to main, worktree cleaned up.
```
