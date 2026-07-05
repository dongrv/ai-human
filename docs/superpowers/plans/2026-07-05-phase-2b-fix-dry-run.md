# Phase 2B Fix Dry Run Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `ai-human fix` dry-run mode so users can preview a bounded local fix plan without modifying source files.

**Architecture:** `FixWorkflow` reuses `ProjectFs` for safe file reads, `ContextLoader` for local knowledge, `AgentClient` for structured JSON, and Markdown renderers for human-readable output. Phase 2B does not write source files or run commands; it only writes the generated report under `.ai-human/reports/`.

**Tech Stack:** Rust 2021, clap, tokio, anyhow, serde, existing `AgentClient`, existing JSONL/report test style.

---

## File Structure

- Modify `src/cli.rs`: add `FixArgs` and `Command::Fix`.
- Modify `src/core.rs`: add `FixPlanOutput` and `FixReplacementFile`.
- Modify `src/report.rs`: add `render_fix_plan`.
- Modify `src/workflow.rs`: add `fix` module with `FixRequest` and `FixWorkflow`.
- Modify `src/main.rs`: route `fix`, reject `--apply` until Phase 2C.
- Modify `README.md`: add fix dry-run examples and safety notes.
- Modify `tests/memory_jsonl.rs`: renderer test.
- Modify `tests/workflow_reports.rs`: workflow dry-run and prompt tests.
- Modify `tests/cli_smoke.rs`: CLI smoke and missing path tests.

## Task 1: Renderer And Domain Types

- [ ] **Step 1: Write failing renderer test**

Add to `tests/memory_jsonl.rs`:

```rust
#[test]
fn renders_fix_plan_output_as_dry_run_markdown() {
    let markdown = render_fix_plan(&FixPlanOutput {
        summary: "Add a nil guard before audit parsing.".into(),
        target_files: vec!["service/pay/audit.go".into()],
        change_intent: "Prevent panic on missing audit payload.".into(),
        risk_level: "low".into(),
        risks: vec!["Behavior changes for malformed payloads".into()],
        verification_commands: vec!["go test ./service/pay".into()],
        replacement_files: vec![FixReplacementFile {
            path: "service/pay/audit.go".into(),
            contents: "package pay\n".into(),
        }],
        open_questions: vec![],
    });

    assert!(markdown.starts_with("# Fix Dry Run\n\n"));
    assert!(markdown.contains("## Summary\n\nAdd a nil guard before audit parsing.\n\n"));
    assert!(markdown.contains("- Source code files modified: no\n"));
}
```

- [ ] **Step 2: Run test to verify RED**

Run: `cargo test renders_fix_plan_output_as_dry_run_markdown`

Expected: fail because `FixPlanOutput`, `FixReplacementFile`, and `render_fix_plan` do not exist.

- [ ] **Step 3: Add minimal types and renderer**

Add `FixPlanOutput` and `FixReplacementFile` in `src/core.rs`. Add `render_fix_plan` in `src/report.rs` with summary, target files, change intent, risks, verification, replacement files, open questions, and dry-run result.

- [ ] **Step 4: Run test to verify GREEN**

Run: `cargo test renders_fix_plan_output_as_dry_run_markdown`

Expected: pass.

## Task 2: FixWorkflow Dry Run

- [ ] **Step 1: Write failing workflow tests**

Add tests to `tests/workflow_reports.rs`:

```rust
#[tokio::test]
async fn fix_workflow_writes_dry_run_report_without_modifying_target_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("service/pay/audit.go")
        .write_str("package pay\n\nfunc Audit() {}\n")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"Add a nil guard before audit parsing.",
        "target_files":["service/pay/audit.go"],
        "change_intent":"Prevent panic on missing audit payload.",
        "risk_level":"low",
        "risks":["Behavior changes for malformed payloads"],
        "verification_commands":["go test ./service/pay"],
        "replacement_files":[{"path":"service/pay/audit.go","contents":"package pay\n\nfunc Audit() {}\n"}],
        "open_questions":[]
    }"#.into()]);

    let report = FixWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_dry_run(FixRequest {
            input: "Fix missing nil guard in payment audit parser".into(),
            path: "service/pay/audit.go".into(),
            verify_commands: vec![],
            format_command: None,
        })
        .await
        .unwrap();

    let target = tokio::fs::read_to_string(temp.path().join("service/pay/audit.go"))
        .await
        .unwrap();

    assert_eq!(target, "package pay\n\nfunc Audit() {}\n");
    assert!(report.markdown.contains("# Fix Dry Run"));
    assert!(report.markdown.contains("Source code files modified: no"));
    assert!(report.path.ends_with("-fix-dry-run.md"));
}
```

- [ ] **Step 2: Run workflow test to verify RED**

Run: `cargo test fix_workflow_writes_dry_run_report_without_modifying_target_file`

Expected: fail because `FixWorkflow` does not exist.

- [ ] **Step 3: Implement workflow**

Add `workflow::fix` with `FixRequest` and `FixWorkflow::run_dry_run`. It must read exactly one target file through `ProjectFs`, include the file text in the model prompt, parse `FixPlanOutput`, render dry-run Markdown, and write a `-fix-dry-run.md` report.

- [ ] **Step 4: Run workflow tests to verify GREEN**

Run: `cargo test fix_workflow`

Expected: pass.

## Task 3: CLI Wiring

- [ ] **Step 1: Write failing CLI smoke tests**

Add tests to `tests/cli_smoke.rs`:

```rust
#[test]
fn fix_uses_mock_agent_and_prints_dry_run_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("service/pay/audit.go")
        .write_str("package pay\n\nfunc Audit() {}\n")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env("AI_HUMAN_MOCK_RESPONSE", r#"{"summary":"Add a nil guard before audit parsing.","target_files":["service/pay/audit.go"],"change_intent":"Prevent panic on missing audit payload.","risk_level":"low","risks":["Behavior changes for malformed payloads"],"verification_commands":["go test ./service/pay"],"replacement_files":[{"path":"service/pay/audit.go","contents":"package pay\n\nfunc Audit() {}\n"}],"open_questions":[]}"#)
        .args(["fix", "--project-root", temp.path().to_str().unwrap(), "--input", "Fix missing nil guard", "--path", "service/pay/audit.go"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Fix Dry Run"))
        .stdout(predicate::str::contains("Source code files modified: no"))
        .stdout(predicate::str::contains("-fix-dry-run.md"));
}
```

- [ ] **Step 2: Run CLI test to verify RED**

Run: `cargo test fix_uses_mock_agent_and_prints_dry_run_report`

Expected: fail because `fix` CLI does not exist.

- [ ] **Step 3: Add CLI command**

Add `FixArgs` with `--project-root`, `--input`, required `--path`, optional `--dry-run`, `--apply`, `--verify`, and `--format`. Route `Command::Fix` in `src/main.rs`. If `--apply` is supplied, fail with `fix --apply is reserved for Phase 2C; run without --apply to review the dry-run plan first.`

- [ ] **Step 4: Run CLI tests to verify GREEN**

Run: `cargo test fix`

Expected: pass.

## Task 4: Docs And Verification

- [ ] **Step 1: Update README**

Add a `Fix Dry Run Example` section showing:

```powershell
cargo run -- fix --input "Fix missing nil guard in payment audit parser" --path service/pay/audit.go
```

State that dry-run is the default and source code files are not modified.

- [ ] **Step 2: Run full verification**

Run:

```powershell
cargo fmt -- --check
cargo test
cargo check
cargo clippy --all-targets -- -D warnings
```

Expected: all pass.

- [ ] **Step 3: Commit**

Run:

```powershell
git add README.md docs/superpowers/plans/2026-07-05-phase-2b-fix-dry-run.md src tests
git commit -m "feat: add fix dry-run workflow"
```

Expected: clean working tree except any user-created local files.
