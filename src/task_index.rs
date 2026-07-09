use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::core::task::{TaskRecord, TaskStatus, TaskType};
use crate::memory::jsonl::JsonlMemoryStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTaskReport {
    pub task_id: String,
    pub task_type: TaskType,
    pub repo: String,
    pub input: String,
    pub summary: String,
    pub report_path: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTimeline {
    pub task_id: String,
    pub records: Vec<TaskRecord>,
}

#[derive(Debug, Clone)]
pub struct TaskIndex {
    project_root: PathBuf,
}

impl TaskIndex {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    pub async fn append_completed_report(&self, report: CompletedTaskReport) -> Result<()> {
        let store = JsonlMemoryStore::new(self.project_root.join(".ai-human/memory"));
        store
            .append_task(&TaskRecord {
                task_id: report.task_id,
                task_type: report.task_type,
                repo: report.repo,
                input: report.input,
                status: TaskStatus::Completed,
                started_at: report.started_at,
                completed_at: Some(report.completed_at),
                summary: report.summary,
                report_path: Some(report.report_path),
            })
            .await
    }

    pub async fn load(&self, task_id: &str) -> Result<TaskTimeline> {
        let path = self.project_root.join(".ai-human/memory/tasks.jsonl");
        if !tokio::fs::try_exists(&path).await? {
            return Ok(TaskTimeline {
                task_id: task_id.into(),
                records: Vec::new(),
            });
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let mut records = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let record: TaskRecord = serde_json::from_str(line).with_context(|| {
                format!(
                    "invalid task record at {}:{}",
                    display_path(&self.project_root, &path),
                    idx + 1
                )
            })?;
            if record.task_id == task_id {
                records.push(record);
            }
        }

        records.sort_by_key(|record| record.started_at);

        Ok(TaskTimeline {
            task_id: task_id.into(),
            records,
        })
    }
}

pub fn render_task_timeline(timeline: &TaskTimeline) -> String {
    let mut markdown = String::new();

    markdown.push_str(&format!("# Task {}\n\n", timeline.task_id));
    markdown.push_str("## Timeline\n\n");
    if timeline.records.is_empty() {
        markdown.push_str("- No records found.\n\n");
    } else {
        for record in &timeline.records {
            markdown.push_str(&format!(
                "- {} {}: {}\n",
                task_type_label(&record.task_type),
                task_status_label(&record.status),
                record.summary
            ));
            if let Some(report_path) = &record.report_path {
                markdown.push_str(&format!("  Report: {report_path}\n"));
            }
            markdown.push_str(&format!("  Input: {}\n", single_line(&record.input)));
        }
        markdown.push('\n');
    }

    markdown.push_str("## Next\n\n");
    if timeline.records.is_empty() {
        markdown.push_str(&format!(
            "- Start this task with `ai-human plan --task-id {} --input \"describe the change\"`.\n",
            timeline.task_id
        ));
    } else {
        markdown.push_str("- Open the latest report listed above.\n");
        markdown.push_str(&format!(
            "- Continue the workflow with the same task id: `--task-id {}`.\n",
            timeline.task_id
        ));
    }

    markdown
}

fn task_type_label(task_type: &TaskType) -> &'static str {
    match task_type {
        TaskType::Ask => "ask",
        TaskType::Plan => "plan",
        TaskType::ImpactAnalysis => "impact",
        TaskType::CodeReview => "review",
        TaskType::SmallFix => "fix",
        TaskType::Learn => "learn",
    }
}

fn task_status_label(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Started => "started",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
    }
}

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn display_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
