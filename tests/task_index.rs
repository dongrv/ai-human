use chrono::Utc;

use ai_human::core::task::TaskType;
use ai_human::task_index::{render_task_timeline, CompletedTaskReport, TaskIndex};

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
        records: vec![ai_human::core::task::TaskRecord {
            task_id: "pay-audit-001".into(),
            task_type: TaskType::ImpactAnalysis,
            repo: "sample".into(),
            input: "service/pay audit flow".into(),
            status: ai_human::core::task::TaskStatus::Completed,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            summary: "Audit flow touches payment state.".into(),
            report_path: Some(".ai-human/reports/pay-audit-001-impact.md".into()),
        }],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.starts_with("# Task pay-audit-001\n\n"));
    assert!(markdown.contains("## Timeline\n\n"));
    assert!(markdown.contains("- impact completed: Audit flow touches payment state."));
    assert!(markdown.contains("  Report: .ai-human/reports/pay-audit-001-impact.md"));
    assert!(markdown.contains("## Next\n\n- Open the latest report listed above"));
}
