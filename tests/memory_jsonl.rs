use chrono::Utc;
use serde_json::Value;

use ai_human::core::report::{
    FixApplyOutput, FixPlanOutput, FixReplacementFile, ImpactOutput, LearningOutput, PlanOutput,
    ReviewFinding, ReviewOutput, VerificationResult,
};
use ai_human::core::task::{
    DecisionRecord, LearningRecord, ReviewRecord, TaskRecord, TaskStatus, TaskType,
};
use ai_human::memory::jsonl::JsonlMemoryStore;
use ai_human::report::markdown::{
    render_fix_apply, render_fix_plan, render_impact, render_learning, render_plan, render_review,
};

#[tokio::test]
async fn appends_task_records_as_compact_json_lines() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = JsonlMemoryStore::new(temp.path().join(".ai-human/memory"));

    let first = task_record("task-1", "created a plan");
    let second = task_record("task-2", "reviewed a diff");

    store.append_task(&first).await.unwrap();
    store.append_task(&second).await.unwrap();

    let content = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/tasks.jsonl"))
        .await
        .unwrap();
    let lines = content.lines().collect::<Vec<_>>();

    assert_eq!(lines.len(), 2);
    assert!(!lines[0].contains('\n'));
    assert_eq!(
        serde_json::from_str::<Value>(lines[0]).unwrap()["task_id"],
        "task-1"
    );
    assert_eq!(
        serde_json::from_str::<Value>(lines[1]).unwrap()["task_id"],
        "task-2"
    );
}

#[tokio::test]
async fn appends_decision_and_review_records_to_dedicated_files() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = JsonlMemoryStore::new(temp.path().join(".ai-human/memory"));

    store
        .append_decision(&DecisionRecord {
            task_id: "task-1".into(),
            decision: "Keep JSONL for MVP".into(),
            reason: "It is append-only and easy to inspect".into(),
            risk: "Needs migration if volume grows".into(),
        })
        .await
        .unwrap();
    store
        .append_review(&ReviewRecord {
            task_id: "task-1".into(),
            severity: "P1".into(),
            file: Some("src/lib.rs".into()),
            line: Some(42),
            issue: "Missing verification".into(),
            suggestion: "Add a targeted integration test".into(),
        })
        .await
        .unwrap();

    let decision = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/decisions.jsonl"))
        .await
        .unwrap();
    let review = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/reviews.jsonl"))
        .await
        .unwrap();

    assert_eq!(
        serde_json::from_str::<Value>(decision.trim()).unwrap()["decision"],
        "Keep JSONL for MVP"
    );
    assert_eq!(
        serde_json::from_str::<Value>(review.trim()).unwrap()["issue"],
        "Missing verification"
    );
}

#[tokio::test]
async fn appends_learning_records_to_dedicated_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = JsonlMemoryStore::new(temp.path().join(".ai-human/memory"));

    store
        .append_learning(&LearningRecord {
            task_id: "learn-1".into(),
            category: "rule".into(),
            title: "Payment audit ownership".into(),
            learning: "Payment audit rules are owned by service/pay.".into(),
            target_doc: ".ai-human/knowledge/engineering-rules.md".into(),
            evidence: vec!["Team review conclusion".into()],
        })
        .await
        .unwrap();

    let content = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/learnings.jsonl"))
        .await
        .unwrap();
    let value = serde_json::from_str::<Value>(content.trim()).unwrap();

    assert_eq!(value["task_id"], "learn-1");
    assert_eq!(value["category"], "rule");
    assert_eq!(
        value["target_doc"],
        ".ai-human/knowledge/engineering-rules.md"
    );
}

#[test]
fn renders_plan_output_as_markdown() {
    let markdown = render_plan(&PlanOutput {
        title: "Add memory store".into(),
        goal: "Persist task history.".into(),
        non_goals: vec!["SQLite migration".into()],
        affected_areas: vec!["memory".into()],
        risks: vec!["Concurrent appends".into()],
        verification_plan: vec!["cargo test --test memory_jsonl".into()],
        open_questions: vec![],
    });

    assert!(markdown.starts_with("# Add memory store\n\n"));
    assert!(markdown.contains("## Goal\n\nPersist task history.\n\n"));
    assert!(markdown.contains("## Non Goals\n\n- SQLite migration\n\n"));
    assert!(markdown.contains("## Open Questions\n\n- None.\n\n"));
}

#[test]
fn renders_impact_output_as_markdown() {
    let markdown = render_impact(&ImpactOutput {
        summary: "Memory writes touch local project state.".into(),
        files: vec!["src/memory.rs".into()],
        call_chains: vec!["workflow -> memory".into()],
        protocol_risks: vec![],
        state_risks: vec!["Append failure leaves no record".into()],
        persistence_risks: vec!["Malformed JSON breaks readers".into()],
        test_entrypoints: vec!["cargo test".into()],
    });

    assert!(markdown.starts_with("# Impact Analysis\n\n"));
    assert!(markdown.contains("## Files\n\n- src/memory.rs\n\n"));
    assert!(markdown.contains("## Protocol Risks\n\n- None.\n\n"));
    assert!(markdown.contains("## Test Entrypoints\n\n- cargo test\n\n"));
}

#[test]
fn renders_review_output_as_markdown() {
    let markdown = render_review(&ReviewOutput {
        summary: "One issue found.".into(),
        findings: vec![ReviewFinding {
            severity: "P1".into(),
            file: Some("src/report.rs".into()),
            line: Some(7),
            issue: "Report omits findings".into(),
            suggestion: "Include every finding in Markdown".into(),
        }],
        test_gaps: vec![],
        residual_risks: vec!["Manual report review still needed".into()],
    });

    assert!(markdown.starts_with("# Code Review\n\n"));
    assert!(markdown.contains(
        "- **P1** `src/report.rs:7`: Report omits findings Suggestion: Include every finding in Markdown\n"
    ));
    assert!(markdown.contains("## Test Gaps\n\n- None.\n\n"));
    assert!(markdown.contains("## Residual Risks\n\n- Manual report review still needed\n\n"));
}

#[test]
fn renders_learning_output_as_markdown() {
    let markdown = render_learning(&LearningOutput {
        title: "Payment audit ownership".into(),
        category: "rule".into(),
        summary: "Audit rules have a single owner.".into(),
        rule: "Payment audit rules are owned by service/pay.".into(),
        evidence: vec!["Review finding P1".into()],
        applies_to: vec!["service/pay".into()],
        target_doc: "engineering-rules".into(),
    });

    assert!(markdown.starts_with("# Payment audit ownership\n\n"));
    assert!(markdown.contains("## Category\n\nrule\n\n"));
    assert!(markdown.contains("## Rule\n\nPayment audit rules are owned by service/pay.\n\n"));
    assert!(markdown.contains("## Evidence\n\n- Review finding P1\n\n"));
}

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
    assert!(markdown.contains("## Target Files\n\n- service/pay/audit.go\n\n"));
    assert!(markdown.contains("- Source code files modified: no\n"));
    assert!(markdown.contains("## Verification Commands\n\n- go test ./service/pay\n\n"));
}

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
    assert!(markdown.contains("succeeded"));
}

fn task_record(task_id: &str, summary: &str) -> TaskRecord {
    TaskRecord {
        task_id: task_id.into(),
        task_type: TaskType::Plan,
        repo: "sample".into(),
        input: "design a feature".into(),
        status: TaskStatus::Completed,
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        summary: summary.into(),
        report_path: Some(format!(".ai-human/reports/{task_id}-plan.md")),
    }
}
