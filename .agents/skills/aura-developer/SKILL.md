---
name: aura-developer
description: Use when implementing new features, components, or logic in Aura using strict test-driven development (TDD), GPUI components, and Rust code.
---

# Aura Developer

Guides Phase 2 Implementation for the Aura API Client. Enforces strict Test-Driven Development (TDD), constructs GPUI components in accordance with `DESIGN.md`, and implements high-performance Rust logic adhering to DRY and KISS principles.

## When to Use

- Writing implementation code for Aura features, models, views, or engine logic.
- Authoring unit and integration tests prior to feature implementation.
- Refactoring code during the Implementation Loop based on debugger feedback.

## When NOT to Use

- Designing overall module architecture or runtime boundaries—use `aura-architect`.
- Diagnosing compiler errors, panic stack traces, or deadlocks—use `aura-debugger`.
- Running compliance audits or code review—use `aura-reviewer`.

---

## Implementation Directives

1. **Strict Test-Driven Development (TDD):**
   - Write tests BEFORE implementation code. Never write production code without a failing test first.
   - Author both unit tests for local algorithms and integration tests for component workflows.
   - For HTTP client and request execution workflows, write tests targeting the required live Postman Echo endpoints:
     - `GET https://postman-echo.com/get?foo1=bar1&foo2=bar2`
     - `POST https://postman-echo.com/post`
     - `PUT https://postman-echo.com/put`
     - `PATCH https://postman-echo.com/patch`
     - `DELETE https://postman-echo.com/delete`
2. **GPUI Components & UI Styling:**
   - Always consult and adhere to `DESIGN.md` for styling, sizing, margins, typography, and palette.
   - Use `gpui-kit` components where applicable.
   - Enforce standard HTTP method badge color rules: GET (Blue), POST (Green), PUT (Amber), PATCH (Purple), DELETE (Rose).
3. **Core Engine & Sandboxing:**
   - Use `deno_core` for script execution. Never introduce `rquickjs`.
   - Ensure script execution runs strictly offline without network leaking.
   - Completely eliminate all telemetry, tracking, or background metrics beaconing.
4. **Code Quality & Paths:**
   - Enforce Relative Path Anchoring on all filesystem interactions.
   - Keep error handling lean and idiomatic using `Result` and `anyhow`/`thiserror`—avoid defensive bloating.
   - Balance DRY (don't repeat yourself) and KISS (keep it simple, stupid).

---

## Implementation Loop (Dev <-> Debugger)

```
[Write Tests First] -> [Implement Code] -> [Handoff to Debugger]
                             ^                      |
                             |-- [Fix & Refactor] <-+ (on error/warning)
```

1. Dev writes tests and implements logic.
2. Handoff to `aura-debugger` to compile and execute tests.
3. If errors or warnings arise, Dev receives diagnostics and refactors until zero warnings and 100% test pass.
4. Once verified, transition to Gate 2 sign-off.
