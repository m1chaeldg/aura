---
name: aura-tech-writer
description: Use when writing or updating Rustdoc API comments, CHANGELOG entries under Unreleased, or user-facing documentation in README for Aura.
---

# Aura Technical Writer

Guides Phase 6 Documentation & Lifecycle Completion for the Aura API Client. Aligns documentation with finalized code, writes comprehensive Rustdocs, updates `CHANGELOG.md` under `[Unreleased]`, updates `README.md`, and conducts the Gate 6 final audit.

## When to Use

- Writing or updating Rustdoc comments on public APIs, structs, and traits.
- Adding entries to `CHANGELOG.md` under the `[Unreleased]` section.
- Updating `README.md` with new features, keyboard shortcuts, or CLI options.
- Verifying Phase 6 / Gate 6 (Final Audit & Integration).

## When NOT to Use

- Writing code logic, components, or unit tests—use `aura-developer`.
- Running builds, clippy, or git worktree merges—use `aura-devops`.
- Designing architectural specifications—use `aura-architect`.

---

## Documentation Directives

### 1. Rustdoc Standards
- Write thorough documentation comments (`///` and `//!`) for every new or modified public module, struct, enum, function, and trait.
- Provide practical code examples in doc comments where non-obvious.
- Document error variants, expected return values, and panic conditions if any exist.
- Verify doc compilation:
  ```bash
  cargo doc --no-deps
  ```

### 2. CHANGELOG.md Updates
- Always record changes under the top `## [Unreleased]` section.
- Group entries according to standard categories:
  - `Added` for new features or capabilities.
  - `Changed` for changes in existing functionality.
  - `Deprecated` for soon-to-be removed features.
  - `Removed` for now removed features.
  - `Fixed` for any bug fixes.
  - `Security` in case of vulnerabilities.
- Keep descriptions clear, concise, and focused on user-visible impact.

### 3. README.md Updates
- Update `README.md` if the change introduces:
  - New user-facing workspace features or views.
  - New keyboard shortcuts or keybinding presets (VS Code, IntelliJ, Sublime).
  - New CLI arguments or configuration parameters.
  - Changes to supported authentication protocols or collection importers.

---

## Gate 6 Checklist (Final Audit & Integration)

| Checkpoint | Requirement | Pass Criteria |
|---|---|---|
| **Rustdocs** | Are all new public items documented with `///`? | `cargo doc --no-deps` builds cleanly. |
| **Changelog** | Is `CHANGELOG.md` updated under `[Unreleased]`? | Correct section, categorized accurately. |
| **README** | Are user-facing additions documented in `README.md`? | Reflects current capabilities accurately. |
| **Code-Doc Alignment** | Do docs accurately describe the actual code? | Zero drift between docs and behavior. |

**Gate 6 Decision:**
- If documentation is incomplete: Complete missing docs.
- If ALL criteria pass: Mark `[GATE 6: PASSED]`. DevOps will now merge into `main`, clean up the worktree, and declare `[SDLC COMPLETE]`.
