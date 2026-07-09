use assert_fs::prelude::*;

use ai_human::agent::mock::MockAgentClient;
use ai_human::agent::{AgentClient, AgentRequest};
use ai_human::core::task::TaskId;
use ai_human::workflow::ask::AskWorkflow;
use ai_human::workflow::fix::{FixRequest, FixWorkflow};
use ai_human::workflow::impact::ImpactWorkflow;
use ai_human::workflow::learn::{LearnRequest, LearnWorkflow};
use ai_human::workflow::plan::PlanWorkflow;
use ai_human::workflow::review::ReviewWorkflow;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn ask_workflow_returns_plain_answer() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/knowledge/engineering-rules.md")
        .write_str("# Rules\n\nAlways verify.")
        .unwrap();
    let agent = MockAgentClient::new(vec!["Use the project verification rules.".into()]);

    let answer = AskWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run("what should I verify?")
        .await
        .unwrap();

    assert_eq!(answer, "Use the project verification rules.");
}

#[tokio::test]
async fn plan_workflow_writes_markdown_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/knowledge/engineering-rules.md")
        .write_str("# Rules\n\nAlways verify.")
        .unwrap();

    let agent = MockAgentClient::new(vec![r#"{
        "title":"Add payment audit",
        "goal":"Design a safe payment audit change",
        "non_goals":["No deployment"],
        "affected_areas":["service/pay"],
        "risks":["Missing persistence check"],
        "verification_plan":["Run target package tests"],
        "open_questions":["Which table owns audit rows?"]
    }"#
    .into()]);

    let report = PlanWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run("add payment audit")
        .await
        .unwrap();

    assert!(report.markdown.contains("# Add payment audit"));
    assert!(report.path.ends_with("-plan.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[test]
fn task_id_preserves_explicit_user_value() {
    let task_id = TaskId::from_user_input(Some("pay-audit-001".into()));

    assert_eq!(task_id.as_str(), "pay-audit-001");
}

#[test]
fn task_id_generates_stable_prefix_when_missing() {
    let task_id = TaskId::from_user_input(None);

    assert!(task_id.as_str().starts_with("task-"));
}

#[tokio::test]
async fn plan_workflow_report_includes_task_id_and_unified_sections() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "title":"Add payment audit",
        "goal":"Design a safe payment audit change",
        "non_goals":["No deployment"],
        "affected_areas":["service/pay"],
        "risks":["Missing persistence check"],
        "verification_plan":["Run target package tests"],
        "open_questions":[]
    }"#
    .into()]);

    let report = PlanWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_with_task_id(
            "add payment audit",
            TaskId::from_user_input(Some("pay-audit-001".into())),
        )
        .await
        .unwrap();

    assert!(report.markdown.contains("## Summary"));
    assert!(report.markdown.contains("## Task"));
    assert!(report.markdown.contains("- Task ID: pay-audit-001"));
    assert!(report.markdown.contains("## Evidence"));
    assert!(report.markdown.contains("## Risks"));
    assert!(report.markdown.contains("## Next"));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn repeated_plan_runs_create_distinct_reports_without_overwriting() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![
        r#"{
        "title":"First plan",
        "goal":"Write the first report",
        "non_goals":[],
        "affected_areas":[],
        "risks":[],
        "verification_plan":["cargo test"],
        "open_questions":[]
    }"#
        .into(),
        r#"{
        "title":"Second plan",
        "goal":"Write the second report",
        "non_goals":[],
        "affected_areas":[],
        "risks":[],
        "verification_plan":["cargo test"],
        "open_questions":[]
    }"#
        .into(),
    ]);
    let workflow = PlanWorkflow::new(temp.path().to_path_buf(), Box::new(agent));

    let first = workflow.run("write first report").await.unwrap();
    let second = workflow.run("write second report").await.unwrap();

    assert_ne!(first.path, second.path);
    assert!(first.path.ends_with("-plan.md"));
    assert!(second.path.ends_with("-plan.md"));
    temp.child(&first.path).assert(first.markdown.as_str());
    temp.child(&second.path).assert(second.markdown.as_str());
    assert!(first.markdown.contains("# First plan"));
    assert!(second.markdown.contains("# Second plan"));
}

#[tokio::test]
async fn impact_workflow_writes_markdown_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"Payment audit touches service and dao",
        "files":["service/pay/audit.go"],
        "call_chains":["controller -> service/pay -> dao"],
        "protocol_risks":["No client protocol change"],
        "state_risks":["No player state change"],
        "persistence_risks":["New audit row ownership"],
        "test_entrypoints":["go test ./service/pay"]
    }"#
    .into()]);

    let report = ImpactWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run("service/pay/audit.go")
        .await
        .unwrap();

    assert!(report
        .markdown
        .contains("Payment audit touches service and dao"));
    assert!(report.path.ends_with("-impact.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn impact_report_includes_task_id_and_unified_sections() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"Payment audit touches service and dao",
        "files":["service/pay/audit.go"],
        "call_chains":["controller -> service/pay -> dao"],
        "protocol_risks":[],
        "state_risks":["Audit state ownership"],
        "persistence_risks":["New audit row ownership"],
        "test_entrypoints":["go test ./service/pay"]
    }"#
    .into()]);

    let report = ImpactWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_with_task_id(
            "service/pay/audit.go",
            TaskId::from_user_input(Some("pay-audit-001".into())),
        )
        .await
        .unwrap();

    assert!(report.markdown.contains("## Summary"));
    assert!(report.markdown.contains("## Task"));
    assert!(report.markdown.contains("- Task ID: pay-audit-001"));
    assert!(report.markdown.contains("## Evidence"));
    assert!(report.markdown.contains("## Risks"));
    assert!(report.markdown.contains("## Next"));
}

#[tokio::test]
async fn review_workflow_writes_markdown_report_from_diff_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("change.diff")
        .write_str("diff --git a/service/pay/audit.go b/service/pay/audit.go")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"One persistence risk found",
        "findings":[{
            "severity":"P1",
            "file":"service/pay/audit.go",
            "line":42,
            "issue":"Audit state is not persisted after mutation",
            "suggestion":"Persist audit state before returning success"
        }],
        "test_gaps":["No regression test for failed persistence"],
        "residual_risks":["Database migration ownership must be confirmed"]
    }"#
    .into()]);

    let report = ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_diff_file(temp.path().join("change.diff"))
        .await
        .unwrap();

    assert!(report.markdown.contains("P1"));
    assert!(report.markdown.contains("Audit state is not persisted"));
    assert!(report.path.ends_with("-review.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn review_workflow_appends_findings_to_reviews_memory() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("change.diff")
        .write_str("diff --git a/service/pay/audit.go b/service/pay/audit.go")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"One persistence risk found",
        "findings":[{
            "severity":"P1",
            "file":"service/pay/audit.go",
            "line":42,
            "issue":"Audit state is not persisted after mutation",
            "suggestion":"Persist audit state before returning success"
        }],
        "test_gaps":[],
        "residual_risks":[]
    }"#
    .into()]);

    ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_diff_file(temp.path().join("change.diff"))
        .await
        .unwrap();

    let reviews = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/reviews.jsonl"))
        .await
        .unwrap();

    assert!(reviews.contains(r#""severity":"P1""#));
    assert!(reviews.contains(r#""issue":"Audit state is not persisted after mutation""#));
}

#[tokio::test]
async fn review_workflow_rejects_diff_file_outside_project_root() {
    let project = assert_fs::TempDir::new().unwrap();
    let outside = assert_fs::TempDir::new().unwrap();
    outside
        .child("change.diff")
        .write_str("secret diff")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"unused",
        "findings":[],
        "test_gaps":[],
        "residual_risks":[]
    }"#
    .into()]);

    let error = ReviewWorkflow::new(project.path().to_path_buf(), Box::new(agent))
        .run_diff_file(outside.path().join("change.diff"))
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must stay inside project root"));
}

#[tokio::test]
async fn review_workflow_writes_markdown_report_from_text() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"No findings",
        "findings":[],
        "test_gaps":[],
        "residual_risks":[]
    }"#
    .into()]);

    let report = ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_text("REVIEW FILE: service/pay/audit.go")
        .await
        .unwrap();

    assert!(report.markdown.contains("No findings"));
    assert!(report.path.ends_with("-review.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn review_report_includes_task_id_and_unified_sections() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"No findings",
        "findings":[],
        "test_gaps":[],
        "residual_risks":["Manual business review still needed"]
    }"#
    .into()]);

    let report = ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_text_with_task_id(
            "REVIEW FILE: service/pay/audit.go",
            TaskId::from_user_input(Some("pay-audit-001".into())),
        )
        .await
        .unwrap();

    assert!(report.markdown.contains("## Summary"));
    assert!(report.markdown.contains("## Task"));
    assert!(report.markdown.contains("- Task ID: pay-audit-001"));
    assert!(report.markdown.contains("## Evidence"));
    assert!(report.markdown.contains("## Risks"));
    assert!(report.markdown.contains("## Next"));
}

#[tokio::test]
async fn learn_workflow_writes_knowledge_report_and_memory() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/knowledge/README.md")
        .write_str("# Knowledge\n")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "title":"Payment audit ownership",
        "category":"rule",
        "summary":"Audit rules have a single owner.",
        "rule":"Payment audit rules are owned by service/pay.",
        "evidence":["Team review conclusion"],
        "applies_to":["service/pay"],
        "target_doc":"engineering-rules"
    }"#
    .into()]);

    let report = LearnWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run(LearnRequest {
            input: "Payment audit rules are owned by service/pay.".into(),
            category: "rule".into(),
            target: None,
            source_report: None,
        })
        .await
        .unwrap();

    assert!(report.markdown.contains("# Payment audit ownership"));
    assert!(report
        .markdown
        .contains("Knowledge written to .ai-human/knowledge/engineering-rules.md"));
    assert!(report.markdown.contains("Source code files modified: no"));
    assert!(report.path.ends_with("-learn.md"));

    let knowledge =
        tokio::fs::read_to_string(temp.path().join(".ai-human/knowledge/engineering-rules.md"))
            .await
            .unwrap();
    let memory = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/learnings.jsonl"))
        .await
        .unwrap();

    assert!(knowledge.contains("Payment audit rules are owned by service/pay."));
    assert!(memory.contains(r#""category":"rule""#));
    assert!(memory.contains(r#""target_doc":".ai-human/knowledge/engineering-rules.md""#));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn learn_report_includes_task_id_and_unified_sections() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/knowledge/README.md")
        .write_str("# Knowledge\n")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "title":"Payment audit ownership",
        "category":"rule",
        "summary":"Audit rules have a single owner.",
        "rule":"Payment audit rules are owned by service/pay.",
        "evidence":["Team review conclusion"],
        "applies_to":["service/pay"],
        "target_doc":"engineering-rules"
    }"#
    .into()]);

    let report = LearnWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_with_task_id(
            LearnRequest {
                input: "Payment audit rules are owned by service/pay.".into(),
                category: "rule".into(),
                target: None,
                source_report: None,
            },
            TaskId::from_user_input(Some("pay-audit-001".into())),
        )
        .await
        .unwrap();

    assert!(report.markdown.contains("## Summary"));
    assert!(report.markdown.contains("## Task"));
    assert!(report.markdown.contains("- Task ID: pay-audit-001"));
    assert!(report.markdown.contains("## Evidence"));
    assert!(report.markdown.contains("## Risks"));
    assert!(report.markdown.contains("## Next"));
}

#[tokio::test]
async fn learn_workflow_reads_source_report_into_model_prompt() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/reports/review.md")
        .write_str("# Review\n\nPersist audit rows before success.")
        .unwrap();
    let captured = Arc::new(Mutex::new(None));
    let agent = RecordingAgentClient {
        captured: Arc::clone(&captured),
        response: r#"{
            "title":"Persist audit rows",
            "category":"rule",
            "summary":"Persistence must happen before success.",
            "rule":"Persist audit rows before returning success.",
            "evidence":["Review report"],
            "applies_to":["service/pay"],
            "target_doc":"engineering-rules"
        }"#
        .into(),
    };

    LearnWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run(LearnRequest {
            input: "Turn the review into a rule.".into(),
            category: "rule".into(),
            target: None,
            source_report: Some(".ai-human/reports/review.md".into()),
        })
        .await
        .unwrap();

    let request = captured.lock().unwrap().clone().unwrap();

    assert!(request
        .user_prompt
        .contains("SOURCE REPORT: .ai-human/reports/review.md"));
    assert!(request
        .user_prompt
        .contains("Persist audit rows before success."));
}

#[tokio::test]
async fn learn_workflow_rejects_source_report_outside_project_root() {
    let project = assert_fs::TempDir::new().unwrap();
    let outside = assert_fs::TempDir::new().unwrap();
    outside
        .child("review.md")
        .write_str("# Secret report")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "title":"unused",
        "category":"rule",
        "summary":"unused",
        "rule":"unused",
        "evidence":[],
        "applies_to":[],
        "target_doc":"engineering-rules"
    }"#
    .into()]);

    let error = LearnWorkflow::new(project.path().to_path_buf(), Box::new(agent))
        .run(LearnRequest {
            input: "learn from report".into(),
            category: "rule".into(),
            target: None,
            source_report: Some(outside.path().join("review.md")),
        })
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must stay inside project root"));
}

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
    }"#
    .into()]);

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
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn fix_dry_run_report_includes_task_id_and_unified_sections() {
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
    }"#
    .into()]);

    let report = FixWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_dry_run_with_task_id(
            FixRequest {
                input: "Fix missing nil guard in payment audit parser".into(),
                path: "service/pay/audit.go".into(),
                verify_commands: vec![],
                format_command: None,
            },
            TaskId::from_user_input(Some("pay-audit-001".into())),
        )
        .await
        .unwrap();

    assert!(report.markdown.contains("## Summary"));
    assert!(report.markdown.contains("## Task"));
    assert!(report.markdown.contains("- Task ID: pay-audit-001"));
    assert!(report.markdown.contains("## Evidence"));
    assert!(report.markdown.contains("## Risks"));
    assert!(report.markdown.contains("## Next"));
}

#[tokio::test]
async fn fix_workflow_includes_target_file_in_model_prompt() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("service/pay/audit.go")
        .write_str("package pay\n\nfunc Audit() {}\n")
        .unwrap();
    let captured = Arc::new(Mutex::new(None));
    let agent = RecordingAgentClient {
        captured: Arc::clone(&captured),
        response: r#"{
            "summary":"Add a nil guard before audit parsing.",
            "target_files":["service/pay/audit.go"],
            "change_intent":"Prevent panic on missing audit payload.",
            "risk_level":"low",
            "risks":[],
            "verification_commands":[],
            "replacement_files":[],
            "open_questions":[]
        }"#
        .into(),
    };

    FixWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_dry_run(FixRequest {
            input: "Fix missing nil guard".into(),
            path: "service/pay/audit.go".into(),
            verify_commands: vec!["go test ./service/pay".into()],
            format_command: Some("gofmt -w service/pay/audit.go".into()),
        })
        .await
        .unwrap();

    let request = captured.lock().unwrap().clone().unwrap();

    assert!(request
        .user_prompt
        .contains("TARGET FILE: service/pay/audit.go"));
    assert!(request.user_prompt.contains("func Audit() {}"));
    assert!(request
        .user_prompt
        .contains("USER VERIFY COMMANDS:\n- go test ./service/pay"));
    assert!(request
        .user_prompt
        .contains("USER FORMAT COMMAND:\ngofmt -w service/pay/audit.go"));
}

#[tokio::test]
async fn fix_workflow_rejects_missing_target_file_before_model_call() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"unused",
        "target_files":[],
        "change_intent":"unused",
        "risk_level":"low",
        "risks":[],
        "verification_commands":[],
        "replacement_files":[],
        "open_questions":[]
    }"#
    .into()]);

    let error = FixWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_dry_run(FixRequest {
            input: "Fix missing nil guard".into(),
            path: "missing.go".into(),
            verify_commands: vec![],
            format_command: None,
        })
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must reference an existing project file"));
}

#[tokio::test]
async fn fix_apply_writes_replacement_and_delivery_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn value() -> i32 { 1 }\n")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"Return updated value.",
        "target_files":["src/lib.rs"],
        "change_intent":"Change the returned value.",
        "risk_level":"low",
        "risks":["Manual review still needed"],
        "verification_commands":[],
        "replacement_files":[{"path":"src/lib.rs","contents":"pub fn value() -> i32 { 2 }\n"}],
        "open_questions":[]
    }"#
    .into()]);

    let report = FixWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_apply(FixRequest {
            input: "Change value to two".into(),
            path: "src/lib.rs".into(),
            verify_commands: vec!["cargo --version".into()],
            format_command: None,
        })
        .await
        .unwrap();

    let target = tokio::fs::read_to_string(temp.path().join("src/lib.rs"))
        .await
        .unwrap();

    assert_eq!(target, "pub fn value() -> i32 { 2 }\n");
    assert!(report.markdown.contains("# Fix Apply Report"));
    assert!(report.markdown.contains("Source code files modified: yes"));
    assert!(report.markdown.contains("cargo --version"));
    assert!(report.path.ends_with("-fix-apply.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn fix_apply_report_includes_task_id_and_unified_sections() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn value() -> i32 { 1 }\n")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"Return updated value.",
        "target_files":["src/lib.rs"],
        "change_intent":"Change the returned value.",
        "risk_level":"low",
        "risks":["Manual review still needed"],
        "verification_commands":[],
        "replacement_files":[{"path":"src/lib.rs","contents":"pub fn value() -> i32 { 2 }\n"}],
        "open_questions":[]
    }"#
    .into()]);

    let report = FixWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_apply_with_task_id(
            FixRequest {
                input: "Change value to two".into(),
                path: "src/lib.rs".into(),
                verify_commands: vec![],
                format_command: None,
            },
            TaskId::from_user_input(Some("pay-audit-001".into())),
        )
        .await
        .unwrap();

    assert!(report.markdown.contains("## Summary"));
    assert!(report.markdown.contains("## Task"));
    assert!(report.markdown.contains("- Task ID: pay-audit-001"));
    assert!(report.markdown.contains("## Evidence"));
    assert!(report.markdown.contains("## Risks"));
    assert!(report.markdown.contains("## Next"));
}

#[tokio::test]
async fn fix_apply_rejects_replacement_for_different_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn value() -> i32 { 1 }\n")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"Return updated value.",
        "target_files":["src/lib.rs"],
        "change_intent":"Change the returned value.",
        "risk_level":"low",
        "risks":[],
        "verification_commands":[],
        "replacement_files":[{"path":"src/other.rs","contents":"pub fn value() -> i32 { 2 }\n"}],
        "open_questions":[]
    }"#
    .into()]);

    let error = FixWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_apply(FixRequest {
            input: "Change value to two".into(),
            path: "src/lib.rs".into(),
            verify_commands: vec![],
            format_command: None,
        })
        .await
        .unwrap_err()
        .to_string();

    let target = tokio::fs::read_to_string(temp.path().join("src/lib.rs"))
        .await
        .unwrap();

    assert_eq!(target, "pub fn value() -> i32 { 1 }\n");
    assert!(error.contains("model replacement must target exactly src/lib.rs"));
}

#[tokio::test]
async fn review_workflow_reads_file_content_from_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn reviewed_symbol() {}")
        .unwrap();
    let captured = Arc::new(Mutex::new(None));
    let agent = RecordingAgentClient {
        captured: Arc::clone(&captured),
        response: r#"{
            "summary":"No findings",
            "findings":[],
            "test_gaps":[],
            "residual_risks":[]
        }"#
        .into(),
    };

    ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_path("src/lib.rs".into())
        .await
        .unwrap();

    let request = captured.lock().unwrap().clone().unwrap();

    assert!(request.user_prompt.contains("FILE: src/lib.rs"));
    assert!(request.user_prompt.contains("pub fn reviewed_symbol() {}"));
}

#[tokio::test]
async fn review_workflow_rejects_missing_path_before_model_call() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"unused",
        "findings":[],
        "test_gaps":[],
        "residual_risks":[]
    }"#
    .into()]);

    let error = ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_path("missing.rs".into())
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must reference an existing project file"));
}

#[derive(Debug)]
struct RecordingAgentClient {
    captured: Arc<Mutex<Option<AgentRequest>>>,
    response: String,
}

#[async_trait]
impl AgentClient for RecordingAgentClient {
    async fn complete(&self, request: AgentRequest) -> anyhow::Result<String> {
        *self.captured.lock().unwrap() = Some(request);
        Ok(self.response.clone())
    }
}
