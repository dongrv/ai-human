# Phase 2C Fix Apply Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable `ai-human fix --apply` for one explicit target file, with safe local writes, optional CLI-provided verification commands, and a delivery report that states exactly what happened.

**Architecture:** `FixWorkflow` will keep dry-run behavior unchanged and add an apply path that validates the model replacement file, writes only the requested project file through `ProjectFs`, runs optional CLI-provided commands through a small `CommandRunner`, and renders an apply report. The model may propose verification commands, but execution uses only CLI arguments.

**Tech Stack:** Rust 2021, clap, tokio filesystem/process APIs, anyhow, serde, existing `AgentClient`, existing report/test style.

---

## File Structure

- Modify `src/tools.rs`: add `ProjectFs::write_text` and a `command` module with `CommandRunner` and `CommandResult`.
- Modify `src/core.rs`: add `VerificationResult` and `FixApplyOutput`.
- Modify `src/report.rs`: add `render_fix_apply`.
- Modify `src/workflow.rs`: add `FixWorkflow::run_apply`, replacement validation, and verification execution.
- Modify `src/main.rs`: route `fix --apply` to apply workflow instead of rejecting.
- Modify `README.md`: document apply safety and examples.
- Modify `tests/project_fs.rs`: safe write tests.
- Add or modify tests in `tests/workflow_reports.rs`, `tests/cli_smoke.rs`, and `tests/memory_jsonl.rs`.

## Task 1: Safe Project File Writes

- [ ] **Step 1: Write failing tests**

Add tests to `tests/project_fs.rs`:

```rust
#[tokio::test]
async fn write_text_replaces_existing_project_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs").write_str("old").unwrap();
    let fs = ProjectFs::new(temp.path());

    let display_path = fs.write_text(Path::new("src/lib.rs"), "new").await.unwrap();

    assert_eq!(display_path, "src/lib.rs");
    temp.child("src/lib.rs").assert("new");
}

#[tokio::test]
async fn write_text_rejects_parent_traversal() {
    let temp = assert_fs::TempDir::new().unwrap();
    let fs = ProjectFs::new(temp.path());

    let error = fs
        .write_text(Path::new("../outside.rs"), "new")
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must stay inside project root"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test write_text_`

Expected: fail because `ProjectFs::write_text` does not exist.

- [ ] **Step 3: Implement minimal write**

Add `ProjectFs::write_text(&self, relative_path: &Path, contents: &str) -> Result<String>`. It must accept only safe relative paths, create parents, reject symlink/non-file targets, ensure canonical parent stays inside project root, write the file, flush it, and return a slash-normalized display path.

- [ ] **Step 4: Run GREEN**

Run: `cargo test write_text_`

Expected: pass.

## Task 2: Command Runner

- [ ] **Step 1: Write failing tests**

Add tests to `tests/command_runner.rs`:

```rust
#[tokio::test]
async fn command_runner_captures_success() {
    let temp = assert_fs::TempDir::new().unwrap();
    let runner = CommandRunner::new(temp.path());

    let result = runner.run("cargo --version").await.unwrap();

    assert_eq!(result.exit_code, Some(0));
    assert!(result.succeeded);
    assert!(result.stdout.contains("cargo"));
}

#[tokio::test]
async fn command_runner_captures_failure() {
    let temp = assert_fs::TempDir::new().unwrap();
    let runner = CommandRunner::new(temp.path());

    let result = runner.run("cargo definitely-not-a-real-command").await.unwrap();

    assert!(!result.succeeded);
    assert_ne!(result.exit_code, Some(0));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test command_runner`

Expected: fail because `CommandRunner` does not exist.

- [ ] **Step 3: Implement minimal runner**

Add `tools::command::{CommandRunner, CommandResult}`. On Windows, run commands with `cmd /C`; on Unix, run commands with `sh -c`. Capture stdout, stderr, exit code, duration in milliseconds, and `succeeded`.

- [ ] **Step 4: Run GREEN**

Run: `cargo test command_runner`

Expected: pass.

## Task 3: Apply Report Renderer

- [ ] **Step 1: Write failing renderer test**

Add to `tests/memory_jsonl.rs`:

```rust
#[test]
fn renders_fix_apply_output_as_delivery_markdown() {
    let markdown = render_fix_apply(&FixApplyOutput {
        summary: "Applied nil guard.".into(),
        written_files: vec!["service/pay/audit.go".into()],
        verification_results: vec![VerificationResult {
            command: "cargo --version".into(),
            exit_code: Some(0),
            succeeded: true,
            stdout: "cargo 1.78".into(),
            stderr: "".into(),
            duration_ms: 10,
        }],
        residual_risks: vec!["Manual business review still needed".into()],
    });

    assert!(markdown.starts_with("# Fix Apply Report\n\n"));
    assert!(markdown.contains("- Source code files modified: yes\n"));
    assert!(markdown.contains("- service/pay/audit.go\n"));
    assert!(markdown.contains("cargo --version"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test renders_fix_apply_output_as_delivery_markdown`

Expected: fail because apply report types and renderer do not exist.

- [ ] **Step 3: Implement types and renderer**

Add `VerificationResult` and `FixApplyOutput` in `src/core.rs`. Add `render_fix_apply` in `src/report.rs`.

- [ ] **Step 4: Run GREEN**

Run: `cargo test renders_fix_apply_output_as_delivery_markdown`

Expected: pass.

## Task 4: Fix Apply Workflow

- [ ] **Step 1: Write failing workflow tests**

Add tests to `tests/workflow_reports.rs`:

```rust
#[tokio::test]
async fn fix_apply_writes_replacement_and_delivery_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs").write_str("pub fn value() -> i32 { 1 }\n").unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"Return updated value.",
        "target_files":["src/lib.rs"],
        "change_intent":"Change the returned value.",
        "risk_level":"low",
        "risks":["Manual review still needed"],
        "verification_commands":[],
        "replacement_files":[{"path":"src/lib.rs","contents":"pub fn value() -> i32 { 2 }\n"}],
        "open_questions":[]
    }"#.into()]);

    let report = FixWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_apply(FixRequest {
            input: "Change value to two".into(),
            path: "src/lib.rs".into(),
            verify_commands: vec!["cargo --version".into()],
            format_command: None,
        })
        .await
        .unwrap();

    let target = tokio::fs::read_to_string(temp.path().join("src/lib.rs")).await.unwrap();
    assert_eq!(target, "pub fn value() -> i32 { 2 }\n");
    assert!(report.markdown.contains("# Fix Apply Report"));
    assert!(report.markdown.contains("Source code files modified: yes"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test fix_apply_writes_replacement_and_delivery_report`

Expected: fail because `run_apply` does not exist.

- [ ] **Step 3: Implement apply workflow**

Add `FixWorkflow::run_apply`. It should reuse the same model prompt as dry-run, find exactly one replacement matching the requested target file display path, write it with `ProjectFs::write_text`, run optional format command first and then verify commands through `CommandRunner`, and render a `-fix-apply.md` report.

- [ ] **Step 4: Run GREEN**

Run: `cargo test fix_apply`

Expected: pass.

## Task 5: CLI Wiring And Docs

- [ ] **Step 1: Write failing CLI test**

Add test to `tests/cli_smoke.rs`:

```rust
#[test]
fn fix_apply_uses_mock_agent_and_modifies_target_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs").write_str("pub fn value() -> i32 { 1 }\n").unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env("AI_HUMAN_MOCK_RESPONSE", r#"{"summary":"Return updated value.","target_files":["src/lib.rs"],"change_intent":"Change the returned value.","risk_level":"low","risks":[],"verification_commands":[],"replacement_files":[{"path":"src/lib.rs","contents":"pub fn value() -> i32 { 2 }\n"}],"open_questions":[]}"#)
        .args(["fix", "--project-root", temp.path().to_str().unwrap(), "--input", "Change value to two", "--path", "src/lib.rs", "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Fix Apply Report"))
        .stdout(predicate::str::contains("Source code files modified: yes"));

    temp.child("src/lib.rs").assert("pub fn value() -> i32 { 2 }\n");
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test fix_apply_uses_mock_agent_and_modifies_target_file`

Expected: fail because CLI still rejects `--apply`.

- [ ] **Step 3: Wire CLI**

In `src/main.rs`, route `Command::Fix(args)` to `run_apply` when `args.apply` is true; otherwise keep `run_dry_run`.

- [ ] **Step 4: Update README and run full verification**

Run:

```powershell
cargo fmt -- --check
cargo test
cargo check
cargo clippy --all-targets -- -D warnings
```

Expected: all pass.

- [ ] **Step 5: Commit and push**

Run:

```powershell
git add README.md docs/superpowers/plans/2026-07-06-phase-2c-fix-apply.md src tests
git commit -m "feat: add controlled fix apply workflow"
git push origin feature/ai-human-mvp
```

Expected: remote branch updated.
