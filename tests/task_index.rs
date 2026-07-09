use chrono::Utc;

use ai_human::core::task::{TaskRecord, TaskStatus, TaskType};
use ai_human::task_index::{
    continuation_from_timeline, render_task_timeline, CompletedTaskReport, Continuation, TaskIndex,
};

#[tokio::test]
async fn appends_and_loads_task_timeline_records() {
    let temp = assert_fs::TempDir::new().unwrap();
    let index = TaskIndex::new(temp.path());

    index
        .append_completed_report(CompletedTaskReport {
            task_id: "pay-audit-001".into(),
            task_type: TaskType::Plan,
            repo: "sample".into(),
            input: "plan payment audit".into(),
            summary: "Design payment audit.".into(),
            report_path: ".ai-human/reports/pay-audit-001-plan.md".into(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
        })
        .await
        .unwrap();

    let timeline = index.load("pay-audit-001").await.unwrap();

    assert_eq!(timeline.task_id, "pay-audit-001");
    assert_eq!(timeline.records.len(), 1);
    assert_eq!(timeline.records[0].summary, "Design payment audit.");
    assert_eq!(
        timeline.records[0].report_path.as_deref(),
        Some(".ai-human/reports/pay-audit-001-plan.md")
    );
}

#[tokio::test]
async fn loading_missing_task_returns_empty_timeline() {
    let temp = assert_fs::TempDir::new().unwrap();
    let index = TaskIndex::new(temp.path());

    let timeline = index.load("missing-task").await.unwrap();

    assert_eq!(timeline.task_id, "missing-task");
    assert!(timeline.records.is_empty());
}

#[test]
fn renders_task_timeline_as_actionable_markdown() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::ImpactAnalysis,
            "service/pay audit flow\nPATH: service/pay/audit.go",
            "Audit flow touches payment state.",
            ".ai-human/reports/pay-audit-001-impact.md",
        )],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.starts_with("# Task pay-audit-001\n\n"));
    assert!(markdown.contains("## Timeline\n\n"));
    assert!(markdown.contains("- impact completed: Audit flow touches payment state."));
    assert!(markdown.contains("  Report: .ai-human/reports/pay-audit-001-impact.md"));
    assert!(markdown.contains("## Next\n\n- Open the latest report listed above"));
    assert!(markdown.contains(
        "- Suggested command: `ai-human review --task-id pay-audit-001 --path service/pay/audit.go`"
    ));
}

#[test]
fn renders_plan_timeline_with_impact_continuation_command() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::Plan,
            "analyze payment audit requirement",
            "Design payment audit.",
            ".ai-human/reports/pay-audit-001-plan.md",
        )],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.contains(
        "- Suggested command: `ai-human impact --task-id pay-audit-001 --input \"analyze payment audit requirement\"`"
    ));
}

#[test]
fn renders_review_timeline_with_fix_continuation_command() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::CodeReview,
            "PATH: service/pay/audit.go",
            "Review found a nil guard issue.",
            ".ai-human/reports/pay-audit-001-review.md",
        )],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.contains(
        "- Suggested command: `ai-human fix --task-id pay-audit-001 --input \"Review found a nil guard issue.\" --path service/pay/audit.go`"
    ));
}

#[test]
fn renders_fix_timeline_with_learn_continuation_command() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::SmallFix,
            "Fix missing nil guard\nPATH: service/pay/audit.go",
            "Applied nil guard.",
            ".ai-human/reports/pay-audit-001-fix-apply.md",
        )],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.contains(
        "- Suggested command: `ai-human learn --task-id pay-audit-001 --input \"Capture the reusable lesson from this task.\" --source-report .ai-human/reports/pay-audit-001-fix-apply.md`"
    ));
}

#[test]
fn continuation_from_impact_task_returns_review_path() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::ImpactAnalysis,
            "check library impact\nPATH: src/lib.rs",
            "Mock impact",
            ".ai-human/reports/pay-audit-001-impact.md",
        )],
    };

    assert_eq!(
        continuation_from_timeline(&timeline),
        Some(Continuation::ReviewPath {
            task_id: "pay-audit-001".into(),
            path: "src/lib.rs".into(),
        })
    );
}

#[test]
fn continuation_from_fix_task_returns_learn_source_report() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::SmallFix,
            "Fix missing nil guard\nPATH: src/lib.rs",
            "Applied nil guard.",
            ".ai-human/reports/pay-audit-001-fix-apply.md",
        )],
    };

    assert_eq!(
        continuation_from_timeline(&timeline),
        Some(Continuation::LearnSourceReport {
            task_id: "pay-audit-001".into(),
            source_report: ".ai-human/reports/pay-audit-001-fix-apply.md".into(),
        })
    );
}

fn task_record(task_type: TaskType, input: &str, summary: &str, report_path: &str) -> TaskRecord {
    TaskRecord {
        task_id: "pay-audit-001".into(),
        task_type,
        repo: "sample".into(),
        input: input.into(),
        status: TaskStatus::Completed,
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        summary: summary.into(),
        report_path: Some(report_path.into()),
    }
}
