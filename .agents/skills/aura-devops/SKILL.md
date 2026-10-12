---
name: aura-devops
description: Use when running local CI pipelines, checking clippy and code formatting, auditing dependency bloat, packaging release binaries, or managing Git worktrees for Aura.
---

# Aura DevOps

Guides Phase 5 Build & Release and workspace/worktree lifecycle management for the Aura API Client. Orchestrates local CI verification, dependency bloat audits, release binary optimization, and Git worktree isolation and merges.

## When to Use

- Initializing or cleaning up isolated Git worktrees (`workgittree`).
- Running local CI validation (`cargo clippy` and `cargo fmt`).
- Auditing dependencies in `Cargo.toml` / `Cargo.lock` for bloat.
- Building the optimized production release binary.
- Performing Gate 5 (Release Ready) and final Phase 6 Git merge into `main`.

## When NOT to Use

- Writing code or tests—use `aura-developer`.
- Investigating runtime crashes or Tokio deadlocks—use `aura-debugger`.
- Writing documentation or updating CHANGELOG—use `aura-tech-writer`.

---

## DevOps Directives

### 1. Workspace & Worktree Lifecycle
All mutations, builds, and executions MUST take place within an isolated Git worktree:

```bash
# 1. Ensure latest main
git fetch origin

# 2. Create isolated worktree for the task
git worktree add ../worktrees/aura-<feature-name> -b feat/<feature-name> origin/main

# 3. Work exclusively inside the worktree directory
cd ../worktrees/aura-<feature-name>
```

Upon Gate 6 completion, DevOps finalizes the lifecycle:
```bash
# Merge into main
git checkout main
git merge feat/<feature-name> --no-ff

# Remove worktree and feature branch
git worktree remove ../worktrees/aura-<feature-name>
git branch -d feat/<feature-name>
```

### 2. Local CI Pipeline Verification
Run strict linting and formatting gates:

```bash
# Code formatting check
cargo fmt -- --check

# Strict clippy lints with warnings as errors
cargo clippy -- -D warnings
```

If clippy or fmt flags issues, return to `aura-developer` to clean up code styles.

### 3. Dependency Bloat Audit
- Inspect `Cargo.toml` for unused or duplicate dependencies.
- Ensure no heavy or prohibited dependencies are introduced (e.g. `rquickjs`, telemetry SDKs).
- Verify binary dependencies are properly pruned with `cargo tree --duplicates`.

### 4. Optimized Release Binary Build
Compile the release artifact:

```bash
cargo build --release
```

Ensure optimization flags in `Cargo.toml` (LTO, codegen-units, opt-level) produce an efficient binary.

---

## Gate 5 Checklist (Release Ready)

| Checkpoint | Requirement | Pass Criteria |
|---|---|---|
| **Formatting** | Does `cargo fmt -- --check` pass cleanly? | Exit code 0, zero formatting diffs. |
| **Clippy Lints** | Does `cargo clippy -- -D warnings` pass? | Zero warnings, zero errors. |
| **Dependency Bloat** | Are all dependencies audited and minimal? | No unnecessary crates, no banned libraries. |
| **Release Build** | Does `cargo build --release` succeed? | Clean release binary generated. |

**Gate 5 Decision:**
- If ANY check fails: Return to `aura-developer` with lint/dependency errors.
- If ALL checks pass: Mark `[GATE 5: PASSED]` and hand off to `aura-tech-writer`.
