use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

use ai_human::workflow::init::InitWorkflow;

const EXPECTED_FILES: &[&str] = &[
    ".ai-human/config.toml",
    ".ai-human/policy.toml",
    ".ai-human/knowledge/README.md",
    ".ai-human/knowledge/project-map.md",
    ".ai-human/knowledge/engineering-rules.md",
    ".ai-human/knowledge/workflows/requirement-plan.md",
    ".ai-human/knowledge/workflows/impact-analysis.md",
    ".ai-human/knowledge/workflows/code-review.md",
    ".ai-human/knowledge/workflows/small-fix.md",
    ".ai-human/knowledge/faq.md",
    ".ai-human/memory/tasks.jsonl",
    ".ai-human/memory/decisions.jsonl",
    ".ai-human/memory/reviews.jsonl",
    ".ai-human/memory/learnings.jsonl",
    ".ai-human/memory/metrics.jsonl",
];

#[tokio::test]
async fn init_creates_ai_human_layout() {
    let temp = assert_fs::TempDir::new().unwrap();

    InitWorkflow::new(temp.path().to_path_buf())
        .run()
        .await
        .unwrap();

    for path in EXPECTED_FILES {
        temp.child(path).assert(predicate::path::exists());
    }
    temp.child(".ai-human/reports")
        .assert(predicate::path::is_dir());
    temp.child(".ai-human/templates")
        .assert(predicate::path::is_dir());
    temp.child(".ai-human/knowledge/cases")
        .assert(predicate::path::is_dir());
}

#[tokio::test]
async fn init_config_contains_fix_defaults() {
    let temp = assert_fs::TempDir::new().unwrap();

    InitWorkflow::new(temp.path().to_path_buf())
        .run()
        .await
        .unwrap();

    temp.child(".ai-human/config.toml")
        .assert(predicate::str::contains("[fix]"))
        .assert(predicate::str::contains("default_verify_commands = []"));
}

#[tokio::test]
async fn init_does_not_overwrite_existing_files() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/config.toml")
        .write_str("project_name = \"kept\"\n")
        .unwrap();

    InitWorkflow::new(temp.path().to_path_buf())
        .run()
        .await
        .unwrap();

    temp.child(".ai-human/config.toml")
        .assert("project_name = \"kept\"\n");
    temp.child(".ai-human/policy.toml")
        .assert(predicate::path::exists());
}

#[test]
fn init_cli_runs_workflow_and_prints_success() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("ai-human").unwrap();
    cmd.args(["init", "--project-root"])
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("ai-human project initialized"));

    temp.child(".ai-human/config.toml")
        .assert(predicate::path::exists());
}
