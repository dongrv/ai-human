use serde_json::Value;

use ai_human::metrics::{CommandMetric, CommandStatus, MetricsStore};

#[tokio::test]
async fn appends_command_metrics_as_local_json_lines() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = MetricsStore::new(temp.path());

    store
        .append(&CommandMetric {
            command: "plan".into(),
            status: CommandStatus::Success,
            duration_ms: 42,
            task_id: Some("pay-audit-001".into()),
            report_path: Some(".ai-human/reports/pay-audit-001-plan.md".into()),
            error: None,
        })
        .await
        .unwrap();

    let content = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/metrics.jsonl"))
        .await
        .unwrap();
    let value = serde_json::from_str::<Value>(content.trim()).unwrap();

    assert_eq!(value["command"], "plan");
    assert_eq!(value["status"], "success");
    assert_eq!(value["duration_ms"], 42);
    assert_eq!(value["task_id"], "pay-audit-001");
    assert_eq!(
        value["report_path"],
        ".ai-human/reports/pay-audit-001-plan.md"
    );
    assert!(value["recorded_at"].is_string());
    assert!(value.get("error").unwrap().is_null());
}

#[tokio::test]
async fn appends_failed_command_metric_without_source_context() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = MetricsStore::new(temp.path());

    store
        .append(&CommandMetric {
            command: "review".into(),
            status: CommandStatus::Failed,
            duration_ms: 7,
            task_id: None,
            report_path: None,
            error: Some("path must reference an existing project file".into()),
        })
        .await
        .unwrap();

    let content = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/metrics.jsonl"))
        .await
        .unwrap();
    let value = serde_json::from_str::<Value>(content.trim()).unwrap();

    assert_eq!(value["command"], "review");
    assert_eq!(value["status"], "failed");
    assert_eq!(
        value["error"],
        "path must reference an existing project file"
    );
    assert!(value.get("input").is_none());
    assert!(value.get("source").is_none());
}

#[tokio::test]
async fn rejects_metrics_write_when_project_root_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    let missing_root = temp.path().join("missing-project");
    let store = MetricsStore::new(&missing_root);

    let err = store
        .append(&CommandMetric {
            command: "doctor".into(),
            status: CommandStatus::Failed,
            duration_ms: 1,
            task_id: None,
            report_path: None,
            error: Some("project root does not exist".into()),
        })
        .await
        .unwrap_err();

    assert!(err.to_string().contains("project root does not exist"));
    assert!(!missing_root.exists());
}
