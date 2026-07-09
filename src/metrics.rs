use std::path::PathBuf;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMetric {
    pub command: String,
    pub status: CommandStatus,
    pub duration_ms: u64,
    pub task_id: Option<String>,
    pub report_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone)]
pub struct MetricsStore {
    project_root: PathBuf,
}

impl MetricsStore {
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    pub async fn append(&self, metric: &CommandMetric) -> Result<()> {
        if !self.project_root.exists() {
            bail!(
                "project root does not exist: {}",
                self.project_root.display()
            );
        }

        let memory_dir = self.project_root.join(".ai-human/memory");
        fs::create_dir_all(&memory_dir).await?;

        let record = StoredCommandMetric {
            recorded_at: Utc::now(),
            command: metric.command.clone(),
            status: metric.status,
            duration_ms: metric.duration_ms,
            task_id: metric.task_id.clone(),
            report_path: metric.report_path.clone(),
            error: metric.error.clone(),
        };
        let line = serde_json::to_string(&record)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(memory_dir.join("metrics.jsonl"))
            .await?;

        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct StoredCommandMetric {
    recorded_at: DateTime<Utc>,
    command: String,
    status: CommandStatus,
    duration_ms: u64,
    task_id: Option<String>,
    report_path: Option<String>,
    error: Option<String>,
}
