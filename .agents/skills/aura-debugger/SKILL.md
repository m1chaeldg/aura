---
name: aura-debugger
description: Use when compiling code, running tests, diagnosing Rust borrow-checker errors, investigating Tokio deadlocks, tracing GPUI rendering panics, or resolving test failures in Aura.
---

# Aura Debugger

Guides compilation verification, test execution, runtime diagnostics, and deadlock/panic tracing for the Aura API Client across Phase 2 (Implementation Loop) and Phase 4 (Testing Loop).

## When to Use

- Executing `cargo build` and `cargo test` to validate code changes.
- Diagnosing Rust borrow-checker, lifetime, or compiler error diagnostics.
- Investigating Tokio async task deadlocks, thread stalls, or channel starvation.
- Debugging GPUI rendering panics, window event crashes, or dirty-state tracking bugs.
- Verifying Phase 2 / Gate 2 (Compilation & Test Pass).

## When NOT to Use

- Writing initial feature code or new tests—use `aura-developer`.
- Conducting compliance or security audits—use `aura-reviewer`.
- Running release packaging, clippy, or formatting checks—use `aura-devops`.

---

## Diagnostic Directives

1. **Compilation Verification:**
   - Execute compilation using terminal/shell MCP tools:
     ```bash
     cargo build
     ```
   - Enforce zero compiler warnings. Treat any warning as a blocking defect.
   - Utilize rust-analyzer diagnostics to resolve borrow-checker and ownership conflicts cleanly.
2. **Test Execution:**
   - Run both unit and integration suites:
     ```bash
     cargo test
     ```
   - For specific module tests:
     ```bash
     cargo test <module_name> -- --nocapture
     ```
   - Inspect failures and capture exact line numbers, assertion values, and test names.
3. **Async & Tokio Diagnostics:**
   - Check for async lock deadlocks (e.g., holding a `std::sync::Mutex` across an `await` point instead of a `tokio::sync::Mutex`).
   - Identify unbounded Tokio channels or stalled background worker loops.
   - Trace structured logs via `tracing` subscriber output for sequence breakdowns.
4. **GPUI Rendering & State Diagnostics:**
   - Capture stack traces on GPUI view layout panics or model mutation conflicts.
   - Ensure views only mutate models inside proper entity transactions (`cx.update(...)`).
   - Check tab dirty-state tracking, multi-split divider resizers, and event routing.

---

## Gate 2 Checklist (Compilation & Test Pass)

Before advancing from Phase 2 to Phase 3 (Code Review), verify:

| Checkpoint | Requirement | Pass Criteria |
|---|---|---|
| **Compilation** | Does `cargo build` succeed? | Exit code 0, zero errors. |
| **Warnings** | Are there zero compiler warnings? | No unused variables, dead code, or deprecations. |
| **Unit Tests** | Do all unit tests pass? | 100% pass rate. |
| **Integration Tests** | Do integration tests pass? | All HTTP and workflow integration tests green. |
| **Runtime Panics** | Does binary execute without panic? | Clean initialization and shutdown. |

**Gate 2 Decision:**
- If ANY failure or warning exists: Send failure details and stack trace to `aura-developer` in the Implementation Loop.
- If ALL criteria pass: Mark `[GATE 2: PASSED]` and hand off to `aura-reviewer`.
