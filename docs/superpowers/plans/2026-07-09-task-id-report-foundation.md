# Task ID Report Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a task id and unified report sections to the main AI Human engineering reports.

**Architecture:** Keep model JSON schemas stable and add the task metadata at the local workflow/report layer. Introduce a small task id value object so CLI parsing, workflow state, report rendering, and future memory records can share one boundary.

**Tech Stack:** Rust, clap, tokio, uuid, assert_cmd, assert_fs.

---

### Task 1: Task ID Value Object

**Files:**
- Modify: `src/core.rs`
- Test: `tests/workflow_reports.rs`

- [ ] **Step 1: Write failing tests**

Add tests that assert generated task ids start with `task-` and that explicit ids are normalized without changing the caller's value.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test workflow_report_includes_task_id`
Expected: FAIL because task id support is not implemented.

- [ ] **Step 3: Implement minimal task id type**

Add `TaskId` under `core::task` with `new()` and `from_user_input()`.

- [ ] **Step 4: Run tests**

Run: `cargo test workflow_report_includes_task_id`
Expected: PASS.

### Task 2: CLI Task ID Argument

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Test: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing CLI tests**

Assert `plan --task-id pay-audit-001` prints `Task ID: pay-audit-001` and still writes a report.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test plan_accepts_task_id`
Expected: FAIL because `--task-id` is unknown.

- [ ] **Step 3: Add `--task-id` to plan, impact, review, learn, and fix arguments**

Use `Option<String>` and pass it into workflow requests.

- [ ] **Step 4: Run tests**

Run: `cargo test plan_accepts_task_id`
Expected: PASS.

### Task 3: Unified Report Skeleton

**Files:**
- Modify: `src/report.rs`
- Modify: `src/workflow.rs`
- Test: `tests/workflow_reports.rs`
- Test: `tests/memory_jsonl.rs`

- [ ] **Step 1: Write failing report tests**

Assert plan, impact, review, learn, fix dry-run, and fix apply reports include `## Summary`, `## Task`, `## Evidence`, `## Risks`, and `## Next` where applicable.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test report_includes_unified_sections`
Expected: FAIL because current renderers do not include task metadata or unified sections.

- [ ] **Step 3: Add report metadata wrapper**

Render task id, evidence, risk, and next action sections locally without changing model response schemas.

- [ ] **Step 4: Run tests**

Run: `cargo test report_includes_unified_sections`
Expected: PASS.

### Task 4: Full Verification and Commit

**Files:**
- Verify all touched files.

- [ ] **Step 1: Format**

Run: `cargo fmt -- --check`
Expected: PASS.

- [ ] **Step 2: Test**

Run: `cargo test`
Expected: PASS.

- [ ] **Step 3: Check build**

Run: `cargo check`
Expected: PASS.

- [ ] **Step 4: Commit and push**

Run:

```bash
git add src/core.rs src/cli.rs src/main.rs src/report.rs src/workflow.rs tests/workflow_reports.rs tests/cli_smoke.rs tests/memory_jsonl.rs docs/superpowers/plans/2026-07-09-task-id-report-foundation.md
git commit -m "feat: add task id report foundation"
git push
```
