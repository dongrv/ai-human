use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

fn result_summary_pattern(next_stage_prefix: &str) -> predicates::str::RegexPredicate {
    predicate::str::is_match(format!(
        "(?s)Report written to .*## Result Summary\\s+- Summary: .+\\s+- Report: .+\\s+- Next stage: {next_stage_prefix}"
    ))
    .unwrap()
}

#[test]
fn root_help_lists_core_commands() {
    let mut cmd = Command::cargo_bin("ai-human").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("impact"))
        .stdout(predicate::str::contains("review"))
        .stdout(predicate::str::contains("learn"))
        .stdout(predicate::str::contains("fix"))
        .stdout(predicate::str::contains("task"))
        .stdout(predicate::str::contains("Check project setup"))
        .stdout(predicate::str::contains("Preview or apply"));
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
        .stdout(predicate::str::contains("ai-human project initialized"))
        .stdout(predicate::str::contains("## Result Summary"))
        .stdout(predicate::str::contains("- Summary: Project initialized."))
        .stdout(predicate::str::contains(
            "- Next stage: run `ai-human doctor --project-root",
        ));

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
        .stdout(predicate::str::contains("mock answer"))
        .stdout(predicate::str::contains("## Result Summary"))
        .stdout(predicate::str::contains("- Summary: Answer generated."))
        .stdout(predicate::str::contains(
            "- Next stage: run `ai-human plan`, `ai-human impact`, or `ai-human learn`",
        ));
}

#[test]
fn ask_help_does_not_expose_unused_task_id() {
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.args(["ask", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--input"))
        .stdout(predicate::str::contains("--task-id").not());
}

#[test]
fn doctor_reports_uninitialized_project_with_next_command() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env_remove("AI_HUMAN_MOCK_RESPONSE")
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENAI_BASE_URL")
        .env_remove("AI_HUMAN_OPENAI_WIRE_API")
        .args(["doctor", "--project-root", temp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("# AI Human Doctor"))
        .stdout(predicate::str::contains("Project state"))
        .stdout(predicate::str::contains("not initialized"))
        .stdout(predicate::str::contains("## Readiness"))
        .stdout(predicate::str::contains("Not ready"))
        .stdout(predicate::str::contains("## Checks"))
        .stdout(predicate::str::contains("missing `.ai-human/config.toml`"))
        .stdout(predicate::str::contains(
            "missing `.ai-human/knowledge/README.md`",
        ))
        .stdout(predicate::str::contains("ai-human init --project-root"))
        .stdout(predicate::str::contains("## Result Summary"))
        .stdout(predicate::str::contains("- Summary: Setup is not ready."))
        .stdout(predicate::str::contains(
            "- Next stage: run `ai-human init --project-root",
        ));
}

#[test]
fn doctor_reports_initialized_project_and_model_env() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/config.toml").write_str("").unwrap();
    temp.child(".ai-human/knowledge/README.md")
        .write_str("# Knowledge")
        .unwrap();
    temp.child(".ai-human/memory/tasks.jsonl")
        .write_str("")
        .unwrap();
    temp.child(".ai-human/reports/.keep").write_str("").unwrap();
    temp.child(".env")
        .write_str("OPENAI_API_KEY=test-key\nAI_HUMAN_OPENAI_WIRE_API=responses\n")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env_remove("AI_HUMAN_MOCK_RESPONSE")
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENAI_BASE_URL")
        .env_remove("AI_HUMAN_OPENAI_WIRE_API")
        .args(["doctor", "--project-root", temp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Project state: initialized"))
        .stdout(predicate::str::contains("## Readiness"))
        .stdout(predicate::str::contains("Ready"))
        .stdout(predicate::str::contains("## Checks"))
        .stdout(predicate::str::contains("present `.ai-human/config.toml`"))
        .stdout(predicate::str::contains("OPENAI_API_KEY configured"))
        .stdout(predicate::str::contains("Wire API: responses"))
        .stdout(predicate::str::contains("## Suggested Workflow"))
        .stdout(predicate::str::contains("ai-human impact"))
        .stdout(predicate::str::contains("ai-human review"))
        .stdout(predicate::str::contains(
            "No blocking setup issues detected",
        ))
        .stdout(predicate::str::contains("## Result Summary"))
        .stdout(predicate::str::contains("- Summary: Setup is ready."))
        .stdout(predicate::str::contains(
            "- Next stage: run `ai-human ask`, `ai-human plan`, or `ai-human impact`",
        ));
}

#[test]
fn ask_loads_mock_agent_response_from_project_env_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".env")
        .write_str("AI_HUMAN_MOCK_RESPONSE=mock answer from env file\n")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env_remove("AI_HUMAN_MOCK_RESPONSE")
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENAI_BASE_URL")
        .args([
            "ask",
            "--project-root",
            temp.path().to_str().unwrap(),
            "--input",
            "what should I verify?",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("mock answer from env file"))
        .stdout(predicate::str::contains("## Result Summary"))
        .stdout(predicate::str::contains("- Summary: Answer generated."))
        .stdout(predicate::str::contains(
            "- Next stage: run `ai-human plan`, `ai-human impact`, or `ai-human learn`",
        ));
}

#[test]
fn ask_missing_openai_api_key_prints_recovery_hint() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env_remove("AI_HUMAN_MOCK_RESPONSE")
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENAI_BASE_URL")
        .env_remove("AI_HUMAN_OPENAI_WIRE_API")
        .args([
            "ask",
            "--project-root",
            temp.path().to_str().unwrap(),
            "--input",
            "what should I verify?",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("OPENAI_API_KEY"))
        .stderr(predicate::str::contains("Next: set OPENAI_API_KEY"))
        .stderr(predicate::str::contains("ai-human doctor"));
}

#[test]
fn ask_unsupported_provider_prints_recovery_hint() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env("AI_HUMAN_MODEL_PROVIDER", "ollama")
        .env_remove("AI_HUMAN_MOCK_RESPONSE")
        .env_remove("OPENAI_API_KEY")
        .args([
            "ask",
            "--project-root",
            temp.path().to_str().unwrap(),
            "--input",
            "what should I verify?",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported model provider"))
        .stderr(predicate::str::contains("AI_HUMAN_MODEL_PROVIDER=openai"))
        .stderr(predicate::str::contains("Next:"));
}

#[test]
fn ask_rejects_unknown_openai_wire_api() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env("AI_HUMAN_OPENAI_WIRE_API", "legacy")
        .args([
            "ask",
            "--project-root",
            temp.path().to_str().unwrap(),
            "--input",
            "what should I verify?",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AI_HUMAN_OPENAI_WIRE_API"))
        .stderr(predicate::str::contains("responses"))
        .stderr(predicate::str::contains("chat-completions"));
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
    .stdout(result_summary_pattern("run `ai-human impact`"))
    .stdout(predicate::str::contains("-plan.md"));
}

#[test]
fn plan_accepts_task_id_and_prints_it_in_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"title":"Mock Plan","goal":"Verify task id wiring","non_goals":[],"affected_areas":["cli"],"risks":[],"verification_plan":["cargo test"],"open_questions":[]}"#,
    )
    .args([
        "plan",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "verify task id wiring",
        "--task-id",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"))
    .stdout(predicate::str::contains("Report written to "))
    .stdout(predicate::str::contains("-plan.md"));

    temp.child(".ai-human/memory/tasks.jsonl")
        .assert(predicate::str::contains(r#""task_id":"pay-audit-001""#))
        .assert(predicate::str::contains(r#""task_type":"Plan""#))
        .assert(predicate::str::contains(
            r#""report_path":".ai-human/reports/"#,
        ))
        .assert(predicate::str::contains("-plan.md"));
    temp.child(".ai-human/memory/metrics.jsonl")
        .assert(predicate::str::contains(r#""command":"plan""#))
        .assert(predicate::str::contains(r#""status":"success""#))
        .assert(predicate::str::contains(r#""task_id":"pay-audit-001""#));
}

#[test]
fn task_command_shows_reports_for_task_id() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/memory/tasks.jsonl")
        .write_str(
            r#"{"task_id":"pay-audit-001","task_type":"Plan","repo":"sample","input":"plan payment audit","status":"Completed","started_at":"2026-07-09T00:00:00Z","completed_at":"2026-07-09T00:00:01Z","summary":"Design payment audit.","report_path":".ai-human/reports/pay-audit-001-plan.md"}
{"task_id":"pay-audit-001","task_type":"ImpactAnalysis","repo":"sample","input":"audit impact\nPATH: service/pay/audit.go","status":"Completed","started_at":"2026-07-09T00:00:02Z","completed_at":"2026-07-09T00:00:03Z","summary":"Audit touches payment state.","report_path":".ai-human/reports/pay-audit-001-impact.md"}
"#,
        )
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.args([
        "task",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--id",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("# Task pay-audit-001"))
    .stdout(predicate::str::contains(
        "plan completed: Design payment audit.",
    ))
    .stdout(predicate::str::contains(
        "impact completed: Audit touches payment state.",
    ))
    .stdout(predicate::str::contains(
        ".ai-human/reports/pay-audit-001-impact.md",
    ))
    .stdout(predicate::str::contains("## Result Summary"))
    .stdout(predicate::str::contains(
        "- Next stage: open the latest report or continue with the listed command",
    ))
    .stdout(predicate::str::contains(
        "ai-human review --task-id pay-audit-001 --path",
    ));
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
    .stdout(result_summary_pattern("run `ai-human review`"))
    .stdout(predicate::str::contains("-impact.md"));
}

#[test]
fn impact_accepts_task_id_and_prints_it_in_report() {
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
        "--task-id",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"));
}

#[test]
fn impact_from_task_uses_latest_plan_input_and_task_id() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/memory/tasks.jsonl")
        .write_str(
            r#"{"task_id":"pay-audit-001","task_type":"Plan","repo":"sample","input":"check library impact","status":"Completed","started_at":"2026-07-09T00:00:00Z","completed_at":"2026-07-09T00:00:01Z","summary":"Mock plan","report_path":".ai-human/reports/pay-audit-001-plan.md"}
"#,
        )
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Mock inherited impact","files":["src/lib.rs"],"call_chains":[],"protocol_risks":[],"state_risks":[],"persistence_risks":[],"test_entrypoints":["cargo test"]}"#,
    )
    .args([
        "impact",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--from-task",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Mock inherited impact"))
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"))
    .stdout(predicate::str::contains("-impact.md"));
}

#[test]
fn impact_from_task_rejects_explicit_task_options() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"unused","files":[],"call_chains":[],"protocol_risks":[],"state_risks":[],"persistence_risks":[],"test_entrypoints":[]}"#,
    )
    .args([
        "impact",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--from-task",
        "pay-audit-001",
        "--input",
        "explicit input",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "use either --from-task or explicit impact options",
    ))
    .stderr(predicate::str::contains("Next:"));
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
    .stdout(result_summary_pattern("run `ai-human learn`"))
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
    .stdout(result_summary_pattern("run `ai-human learn`"))
    .stdout(predicate::str::contains("-review.md"));
}

#[test]
fn review_accepts_task_id_and_prints_it_in_report() {
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
        "--task-id",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"));
}

#[test]
fn review_from_task_uses_latest_impact_path_and_task_id() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn demo() {}")
        .unwrap();
    temp.child(".ai-human/memory/tasks.jsonl")
        .write_str(
            r#"{"task_id":"pay-audit-001","task_type":"ImpactAnalysis","repo":"sample","input":"check library impact\nPATH: src/lib.rs","status":"Completed","started_at":"2026-07-09T00:00:00Z","completed_at":"2026-07-09T00:00:01Z","summary":"Mock impact","report_path":".ai-human/reports/pay-audit-001-impact.md"}
"#,
        )
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Mock inherited review","findings":[],"test_gaps":[],"residual_risks":[]}"#,
    )
    .args([
        "review",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--from-task",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Mock inherited review"))
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"))
    .stdout(predicate::str::contains("-review.md"));
}

#[test]
fn review_from_task_rejects_explicit_review_source() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn demo() {}")
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
        "--from-task",
        "pay-audit-001",
        "--path",
        "src/lib.rs",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "use either --from-task or an explicit review source",
    ))
    .stderr(predicate::str::contains("Next:"));
}

#[test]
fn review_errors_when_path_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"unused","findings":[],"test_gaps":[],"residual_risks":[]}"#,
    )
    .args([
        "review",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--path",
        "missing.rs",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "path must reference an existing project file",
    ))
    .stderr(predicate::str::contains("Next:"));
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
    ))
    .stderr(predicate::str::contains("Next:"));
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
    ))
    .stderr(predicate::str::contains("Next:"));
}

#[test]
fn learn_uses_mock_agent_and_prints_written_paths() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"title":"Payment audit ownership","category":"rule","summary":"Audit rules have a single owner.","rule":"Payment audit rules are owned by service/pay.","evidence":["Team review conclusion"],"applies_to":["service/pay"],"target_doc":"engineering-rules"}"#,
    )
    .args([
        "learn",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "Payment audit rules are owned by service/pay.",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("# Payment audit ownership"))
    .stdout(predicate::str::contains(
        "Knowledge written to .ai-human/knowledge/engineering-rules.md",
    ))
    .stdout(predicate::str::contains("Source code files modified: no"))
    .stdout(predicate::str::contains("Report written to "))
    .stdout(result_summary_pattern(
        "run `ai-human ask`, `ai-human plan`, or `ai-human review`",
    ))
    .stdout(predicate::str::contains("-learn.md"));

    temp.child(".ai-human/knowledge/engineering-rules.md")
        .assert(predicate::str::contains(
            "Payment audit rules are owned by service/pay.",
        ));
    temp.child(".ai-human/memory/learnings.jsonl")
        .assert(predicate::str::contains(r#""category":"rule""#));
}

#[test]
fn learn_accepts_task_id_and_prints_it_in_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"title":"Payment audit ownership","category":"rule","summary":"Audit rules have a single owner.","rule":"Payment audit rules are owned by service/pay.","evidence":["Team review conclusion"],"applies_to":["service/pay"],"target_doc":"engineering-rules"}"#,
    )
    .args([
        "learn",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "Payment audit rules are owned by service/pay.",
        "--task-id",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"));
}

#[test]
fn learn_from_task_uses_latest_fix_report_and_task_id() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/reports/pay-audit-001-fix-apply.md")
        .write_str("# Fix Apply Report\n\n## Summary\n\nApplied nil guard.\n")
        .unwrap();
    temp.child(".ai-human/memory/tasks.jsonl")
        .write_str(
            r#"{"task_id":"pay-audit-001","task_type":"SmallFix","repo":"sample","input":"Fix missing nil guard\nPATH: src/lib.rs","status":"Completed","started_at":"2026-07-09T00:00:00Z","completed_at":"2026-07-09T00:00:01Z","summary":"Applied nil guard.","report_path":".ai-human/reports/pay-audit-001-fix-apply.md"}
"#,
        )
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"title":"Nil guard rule","category":"rule","summary":"Nil guards prevent audit panic.","rule":"Validate audit payload before parsing.","evidence":["Fix report"],"applies_to":["src/lib.rs"],"target_doc":"engineering-rules"}"#,
    )
    .args([
        "learn",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--from-task",
        "pay-audit-001",
        "--input",
        "Capture the reusable lesson from this task.",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("# Nil guard rule"))
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"))
    .stdout(predicate::str::contains("Knowledge written to"));
}

#[test]
fn learn_from_task_rejects_explicit_task_id_or_source_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"title":"unused","category":"rule","summary":"unused","rule":"unused","evidence":[],"applies_to":[],"target_doc":"engineering-rules"}"#,
    )
    .args([
        "learn",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--from-task",
        "pay-audit-001",
        "--task-id",
        "other-task",
        "--input",
        "Capture lesson",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "use either --from-task or explicit task/source options",
    ))
    .stderr(predicate::str::contains("Next:"));
}

#[test]
fn fix_uses_mock_agent_and_prints_dry_run_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("service/pay/audit.go")
        .write_str("package pay\n\nfunc Audit() {}\n")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Add a nil guard before audit parsing.","target_files":["service/pay/audit.go"],"change_intent":"Prevent panic on missing audit payload.","risk_level":"low","risks":["Behavior changes for malformed payloads"],"verification_commands":["go test ./service/pay"],"replacement_files":[{"path":"service/pay/audit.go","contents":"package pay\n\nfunc Audit() {}\n"}],"open_questions":[]}"#,
    )
    .args([
        "fix",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "Fix missing nil guard",
        "--path",
        "service/pay/audit.go",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("# Fix Dry Run"))
    .stdout(predicate::str::contains("Source code files modified: no"))
    .stdout(predicate::str::contains("Report written to "))
    .stdout(result_summary_pattern("run `ai-human fix --apply`"))
    .stdout(predicate::str::contains("-fix-dry-run.md"));

    temp.child("service/pay/audit.go")
        .assert("package pay\n\nfunc Audit() {}\n");
}

#[test]
fn fix_accepts_task_id_and_prints_it_in_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("service/pay/audit.go")
        .write_str("package pay\n\nfunc Audit() {}\n")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Add a nil guard before audit parsing.","target_files":["service/pay/audit.go"],"change_intent":"Prevent panic on missing audit payload.","risk_level":"low","risks":["Behavior changes for malformed payloads"],"verification_commands":["go test ./service/pay"],"replacement_files":[{"path":"service/pay/audit.go","contents":"package pay\n\nfunc Audit() {}\n"}],"open_questions":[]}"#,
    )
    .args([
        "fix",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "Fix missing nil guard",
        "--path",
        "service/pay/audit.go",
        "--task-id",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"));
}

#[test]
fn fix_from_task_uses_latest_review_path_and_task_id() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn demo() {}\n")
        .unwrap();
    temp.child(".ai-human/memory/tasks.jsonl")
        .write_str(
            r#"{"task_id":"pay-audit-001","task_type":"CodeReview","repo":"sample","input":"PATH: src/lib.rs","status":"Completed","started_at":"2026-07-09T00:00:00Z","completed_at":"2026-07-09T00:00:01Z","summary":"Review found a nil guard issue.","report_path":".ai-human/reports/pay-audit-001-review.md"}
"#,
        )
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Add a nil guard before audit parsing.","target_files":["src/lib.rs"],"change_intent":"Prevent panic on missing payload.","risk_level":"low","risks":[],"verification_commands":["cargo test"],"replacement_files":[{"path":"src/lib.rs","contents":"pub fn demo() {}\n"}],"open_questions":[]}"#,
    )
    .args([
        "fix",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--from-task",
        "pay-audit-001",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("# Fix Dry Run"))
    .stdout(predicate::str::contains("- Task ID: pay-audit-001"))
    .stdout(predicate::str::contains("-fix-dry-run.md"));
}

#[test]
fn fix_from_task_rejects_explicit_fix_options() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn demo() {}\n")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"unused","target_files":[],"change_intent":"unused","risk_level":"low","risks":[],"verification_commands":[],"replacement_files":[],"open_questions":[]}"#,
    )
    .args([
        "fix",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--from-task",
        "pay-audit-001",
        "--input",
        "explicit fix",
        "--path",
        "src/lib.rs",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains(
        "use either --from-task or explicit fix options",
    ))
    .stderr(predicate::str::contains("Next:"));
}

#[test]
fn fix_uses_configured_default_commands_when_flags_are_omitted() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("service/pay/audit.go")
        .write_str("package pay\n\nfunc Audit() {}\n")
        .unwrap();
    temp.child(".ai-human/config.toml")
        .write_str(
            r#"project_name = "sample"
model_provider = "openai"
model_name = "gpt-4o-mini"
knowledge_dir = ".ai-human/knowledge"
memory_dir = ".ai-human/memory"
reports_dir = ".ai-human/reports"

[fix]
default_verify_commands = ["cargo --version"]
"#,
        )
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Add a nil guard before audit parsing.","target_files":["service/pay/audit.go"],"change_intent":"Prevent panic on missing audit payload.","risk_level":"low","risks":[],"verification_commands":["go test ./service/pay"],"replacement_files":[{"path":"service/pay/audit.go","contents":"package pay\n\nfunc Audit() {}\n"}],"open_questions":[]}"#,
    )
    .args([
        "fix",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "Fix missing nil guard",
        "--path",
        "service/pay/audit.go",
        "--apply",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("## Verification Results"))
    .stdout(predicate::str::contains("cargo --version"));
}

#[test]
fn fix_cli_commands_override_configured_default_commands() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("service/pay/audit.go")
        .write_str("package pay\n\nfunc Audit() {}\n")
        .unwrap();
    temp.child(".ai-human/config.toml")
        .write_str(
            r#"project_name = "sample"
model_provider = "openai"
model_name = "gpt-4o-mini"
knowledge_dir = ".ai-human/knowledge"
memory_dir = ".ai-human/memory"
reports_dir = ".ai-human/reports"

[fix]
default_verify_commands = ["go test ./service/pay"]
default_format_command = "gofmt -w service/pay/audit.go"
"#,
        )
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Add a nil guard before audit parsing.","target_files":["service/pay/audit.go"],"change_intent":"Prevent panic on missing audit payload.","risk_level":"low","risks":[],"verification_commands":["cargo test --test override"],"replacement_files":[{"path":"service/pay/audit.go","contents":"package pay\n\nfunc Audit() {}\n"}],"open_questions":[]}"#,
    )
    .args([
        "fix",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "Fix missing nil guard",
        "--path",
        "service/pay/audit.go",
        "--verify",
        "cargo --version",
        "--format",
        "cargo --version",
        "--apply",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("cargo --version"))
    .stdout(predicate::str::contains("go test ./service/pay").not());
}

#[test]
fn fix_apply_uses_mock_agent_and_modifies_target_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn value() -> i32 { 1 }\n")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Return updated value.","target_files":["src/lib.rs"],"change_intent":"Change the returned value.","risk_level":"low","risks":[],"verification_commands":[],"replacement_files":[{"path":"src/lib.rs","contents":"pub fn value() -> i32 { 2 }\n"}],"open_questions":[]}"#,
    )
    .args([
        "fix",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "Change value to two",
        "--path",
        "src/lib.rs",
        "--apply",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("# Fix Apply Report"))
    .stdout(predicate::str::contains("Source code files modified: yes"))
    .stdout(predicate::str::contains("Report written to "))
    .stdout(result_summary_pattern("run `ai-human learn`"))
    .stdout(predicate::str::contains("-fix-apply.md"));

    temp.child("src/lib.rs")
        .assert("pub fn value() -> i32 { 2 }\n");
}

#[test]
fn fix_apply_blocks_high_risk_mock_plan_and_keeps_target_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn value() -> i32 { 1 }\n")
        .unwrap();
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.env(
        "AI_HUMAN_MOCK_RESPONSE",
        r#"{"summary":"Change protocol and persistence behavior.","target_files":["src/lib.rs"],"change_intent":"Change behavior with production protocol risk.","risk_level":"high","risks":["Protocol compatibility and persistence ownership need human review"],"verification_commands":[],"replacement_files":[{"path":"src/lib.rs","contents":"pub fn value() -> i32 { 2 }\n"}],"open_questions":["Who owns the migration?"]}"#,
    )
    .args([
        "fix",
        "--project-root",
        temp.path().to_str().unwrap(),
        "--input",
        "Change value with high risk",
        "--path",
        "src/lib.rs",
        "--apply",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("high risk"))
    .stderr(predicate::str::contains("fix dry-run"));

    temp.child("src/lib.rs")
        .assert("pub fn value() -> i32 { 1 }\n");
}

#[test]
fn fix_help_describes_current_apply_behavior() {
    let mut cmd = Command::cargo_bin("ai-human").unwrap();

    cmd.args(["fix", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reserved for Phase 2C").not())
        .stdout(predicate::str::contains("Apply a validated replacement"))
        .stdout(predicate::str::contains(
            "Single target file to analyze or update",
        ));
}
