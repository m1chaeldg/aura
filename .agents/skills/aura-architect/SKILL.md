---
name: aura-architect
description: Use when designing software architecture, module boundaries, data structures, GPUI state management, or deno_core runtime scaffolding for Aura features before implementation begins.
---

# Aura Architect

Guides Phase 1 Architecture & Scaffolding for the Aura API Client. Ingests requirements, aligns with the UI design system, designs core data structures and runtime boundaries, and creates trait scaffolding while enforcing KISS.

## When to Use

- Designing new architectural components, subsystems, or modules for Aura.
- Defining GPUI state management, models, and UI tree hierarchies.
- Establishing `deno_core` V8 sandboxed runtime boundaries and IPC channels.
- Conducting Phase 1 / Gate 1 Design Reviews.

## When NOT to Use

- Writing concrete component implementations or business logic—use `aura-developer`.
- Debugging compilation errors or panics—use `aura-debugger`.
- Reviewing completed code against project compliance rules—use `aura-reviewer`.

---

## Architecture Directives

1. **Design System & UI Alignment:**
   - Always consult and strictly adhere to `DESIGN.md`.
   - Map all UI requirements to GPUI views, elements, and styles.
   - Design layout hierarchy conforming to Aura's multi-tab document workspace, adjustable split dividers, and standard HTTP method badge color specifications.
2. **Runtime Sandbox Isolation (`deno_core`):**
   - Strictly mandate `deno_core` (never `rquickjs`).
   - Design request pre/post-request scripts to run strictly offline with no network leaks.
   - Keep JavaScript runtime operations cleanly separated from Rust native client pipelines.
3. **Data Structures & State Management:**
   - Define minimal, expressive Rust data models for collections, folders, and requests.
   - Favor two-way filesystem synchronization and local-first persistence models.
   - Anchor all filesystem operations to Relative Path Anchoring.
4. **Scaffolding:**
   - Create clean, empty trait definitions and module boundaries.
   - Avoid premature abstraction or deep inheritance-like trait nests. Follow KISS.

---

## Gate 1 Checklist (Design Review)

Before approving any architectural design and moving to Phase 2 (Implementation), verify every checkpoint:

| Checkpoint | Requirement | Pass Criteria |
|---|---|---|
| **KISS Compliance** | Is the architecture straightforward and minimal? | No speculative generalization or over-engineering. |
| **Module Boundaries** | Are boundaries between UI, engine, and storage clean? | Clear separation between GPUI, `deno_core`, and HTTP dispatch. |
| **Premature Abstraction** | Does it avoid unnecessary traits or indirection? | Only essential traits defined. |
| **UI Design System** | Does any UI specification strictly follow `DESIGN.md`? | Typography, tokens, colors, and layout conform to `DESIGN.md`. |
| **Offline Sandbox** | Is `deno_core` isolated from external networks? | No network primitives exposed to the script sandbox. |

**Gate 1 Decision:**
- If ANY item fails: Loop back to Architect with concrete redesign notes.
- If ALL items pass: Mark `[GATE 1: PASSED]` and hand off to `aura-developer`.
