use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::core::task::{EvidenceRecord, RuleHitRecord, TaskRecord, TaskStatus, TaskType};
use crate::memory::jsonl::JsonlMemoryStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTaskReport {
    pub task_id: String,
    pub task_type: TaskType,
    pub repo: String,
    pub input: String,
    pub summary: String,
    pub report_path: String,
    pub evidence: Vec<EvidenceRecord>,
    pub rule_hits: Vec<RuleHitRecord>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTimeline {
    pub task_id: String,
    pub records: Vec<TaskRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Continuation {
    ImpactInput {
        task_id: String,
        input: String,
    },
    ReviewPath {
        task_id: String,
        path: String,
    },
    FixPath {
        task_id: String,
        input: String,
        path: String,
    },
    LearnSourceReport {
        task_id: String,
        source_report: String,
    },
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
                evidence: report.evidence,
                rule_hits: report.rule_hits,
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

    push_evidence_trail(&mut markdown, &timeline.records);
    push_rule_hits(&mut markdown, &timeline.records);

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
        if let Some(command) = suggested_command(timeline) {
            markdown.push_str(&format!("- Suggested command: `{command}`\n"));
        }
    }

    markdown
}

fn push_evidence_trail(markdown: &mut String, records: &[TaskRecord]) {
    markdown.push_str("## Evidence Trail\n\n");
    let mut found = false;

    for record in records {
        let label = task_type_label(&record.task_type);
        for evidence in &record.evidence {
            found = true;
            markdown.push_str(&format!(
                "- {label}: [{}] {}\n",
                evidence.kind, evidence.detail
            ));
        }
    }

    if !found {
        markdown.push_str("- None recorded.\n");
    }
    markdown.push('\n');
}

fn push_rule_hits(markdown: &mut String, records: &[TaskRecord]) {
    markdown.push_str("## Rule Hits\n\n");
    let mut found = false;

    for record in records {
        let label = task_type_label(&record.task_type);
        for hit in &record.rule_hits {
            found = true;
            markdown.push_str(&format!("- {label}: [{}] {}\n", hit.source, hit.detail));
        }
    }

    if !found {
        markdown.push_str("- None recorded.\n");
    }
    markdown.push('\n');
}

pub fn continuation_from_timeline(timeline: &TaskTimeline) -> Option<Continuation> {
    let latest = timeline.records.last()?;

    match latest.task_type {
        TaskType::Plan => Some(Continuation::ImpactInput {
            task_id: timeline.task_id.clone(),
            input: latest.input.clone(),
        }),
        TaskType::ImpactAnalysis => path_hint(&latest.input).map(|path| Continuation::ReviewPath {
            task_id: timeline.task_id.clone(),
            path,
        }),
        TaskType::CodeReview => path_hint(&latest.input).map(|path| Continuation::FixPath {
            task_id: timeline.task_id.clone(),
            input: latest.summary.clone(),
            path,
        }),
        TaskType::SmallFix => {
            latest
                .report_path
                .as_ref()
                .map(|source_report| Continuation::LearnSourceReport {
                    task_id: timeline.task_id.clone(),
                    source_report: source_report.clone(),
                })
        }
        _ => None,
    }
}

fn suggested_command(timeline: &TaskTimeline) -> Option<String> {
    let latest = timeline.records.last()?;
    let task_id = shell_arg(&timeline.task_id);

    match latest.task_type {
        TaskType::Plan => Some(format!(
            "ai-human impact --task-id {task_id} --input {}",
            quoted_arg(&latest.input)
        )),
        TaskType::ImpactAnalysis => match continuation_from_timeline(timeline) {
            Some(Continuation::ReviewPath { path, .. }) => Some(
                format!(
                    "ai-human review --task-id {task_id} --path {}",
                    shell_arg(&path)
                ),
            ),
            _ => None,
        },
        TaskType::CodeReview => path_hint(&latest.input).map(|path| {
            format!(
                "ai-human fix --task-id {task_id} --input {} --path {}",
                quoted_arg(&latest.summary),
                shell_arg(&path)
            )
        }),
        TaskType::SmallFix => match continuation_from_timeline(timeline) {
            Some(Continuation::LearnSourceReport { source_report, .. }) => Some(format!(
                "ai-human learn --task-id {task_id} --input \"Capture the reusable lesson from this task.\" --source-report {}",
                shell_arg(&source_report)
            )),
            _ => None,
        },
        TaskType::Ask | TaskType::Learn => None,
    }
}

fn path_hint(input: &str) -> Option<String> {
    input.lines().find_map(|line| {
        line.trim()
            .strip_prefix("PATH: ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn quoted_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn shell_arg(value: &str) -> String {
    if value.chars().any(char::is_whitespace) {
        quoted_arg(value)
    } else {
        value.into()
    }
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
