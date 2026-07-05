pub mod jsonl {
    use std::path::PathBuf;

    use anyhow::Result;
    use serde::Serialize;
    use tokio::fs::{self, OpenOptions};
    use tokio::io::AsyncWriteExt;

    use crate::core::task::{DecisionRecord, LearningRecord, ReviewRecord, TaskRecord};

    #[derive(Debug, Clone)]
    pub struct JsonlMemoryStore {
        memory_dir: PathBuf,
    }

    impl JsonlMemoryStore {
        pub fn new(memory_dir: impl Into<PathBuf>) -> Self {
            Self {
                memory_dir: memory_dir.into(),
            }
        }

        pub async fn append_task(&self, record: &TaskRecord) -> Result<()> {
            self.append_json_line("tasks.jsonl", record).await
        }

        pub async fn append_decision(&self, record: &DecisionRecord) -> Result<()> {
            self.append_json_line("decisions.jsonl", record).await
        }

        pub async fn append_review(&self, record: &ReviewRecord) -> Result<()> {
            self.append_json_line("reviews.jsonl", record).await
        }

        pub async fn append_learning(&self, record: &LearningRecord) -> Result<()> {
            self.append_json_line("learnings.jsonl", record).await
        }

        async fn append_json_line<T: Serialize>(&self, file_name: &str, value: &T) -> Result<()> {
            fs::create_dir_all(&self.memory_dir).await?;

            let path = self.memory_dir.join(file_name);
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .await?;
            let line = serde_json::to_string(value)?;

            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;

            Ok(())
        }
    }
}
