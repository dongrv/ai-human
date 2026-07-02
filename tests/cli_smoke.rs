use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn root_help_lists_core_commands() {
    let mut cmd = Command::cargo_bin("ai-human").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("impact"))
        .stdout(predicate::str::contains("review"));
}

#[test]
fn init_help_mentions_project_root() {
    let mut cmd = Command::cargo_bin("ai-human").unwrap();
    cmd.args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project-root"));
}

#[test]
fn init_runs_workflow_and_prints_confirmation() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.args(["init", "--project-root", temp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("ai-human project initialized"));

    temp.child(".ai-human/config.toml")
        .assert(predicate::path::exists());
}

#[test]
fn ask_uses_mock_agent_response_from_env() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env("AI_HUMAN_MOCK_RESPONSE", "mock answer")
        .args([
            "ask",
            "--project-root",
            temp.path().to_str().unwrap(),
            "--input",
            "what should I verify?",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("mock answer"));
}

#[test]
fn plan_uses_mock_agent_and_prints_report_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"title":"Mock Plan","goal":"Verify CLI wiring","non_goals":["No model call"],"affected_areas":["cli"],"risks":["Mock only"],"verification_plan":["cargo test"],"open_questions":[]}"#,
    )
    .args([
        "plan",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "verify cli wiring",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("# Mock Plan"))
    .stdout(predicate::str::contains("Report written to "))
    .stdout(predicate::str::contains("-plan.md"));
}

#[test]
fn impact_accepts_optional_path_and_prints_report_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn demo() {}")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Mock impact","files":["src/lib.rs"],"call_chains":[],"protocol_risks":[],"state_risks":[],"persistence_risks":[],"test_entrypoints":["cargo test"]}"#,
    )
    .args([
        "impact",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "check library impact",
        "--path",
        "src/lib.rs",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Mock impact"))
    .stdout(predicate::str::contains("Report written to "))
    .stdout(predicate::str::contains("-impact.md"));
}

#[test]
fn review_accepts_diff_file_and_prints_report_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("change.diff")
        .write_str("diff --git a/src/lib.rs b/src/lib.rs")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Mock review","findings":[],"test_gaps":[],"residual_risks":[]}"#,
    )
    .args([
        "review",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--diff-file",
        temp.path().join("change.diff").to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Mock review"))
    .stdout(predicate::str::contains("Report written to "))
    .stdout(predicate::str::contains("-review.md"));
}

#[test]
fn review_accepts_path_and_prints_report_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn demo() {}")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Mock file review","findings":[],"test_gaps":[],"residual_risks":[]}"#,
    )
    .args([
        "review",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--path",
        "src/lib.rs",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Mock file review"))
    .stdout(predicate::str::contains("Report written to "))
    .stdout(predicate::str::contains("-review.md"));
}

#[test]
fn review_errors_when_both_diff_file_and_path_are_provided() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("change.diff")
        .write_str("diff --git a/src/lib.rs b/src/lib.rs")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"unused","findings":[],"test_gaps":[],"residual_risks":[]}"#,
    )
    .args([
        "review",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--diff-file",
        temp.path().join("change.diff").to_str().unwrap(),
        "--path",
        "src/lib.rs",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "use either --diff-file or --path, not both",
    ));
}

#[test]
fn review_errors_when_neither_diff_file_nor_path_is_provided() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"unused","findings":[],"test_gaps":[],"residual_risks":[]}"#,
    )
    .args(["review", "--project-root", temp.path().to_str().unwrap()])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "review requires --diff-file or --path",
    ));
}
