pub mod init {
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use tokio::fs;

    use crate::config::{AiHumanConfig, PolicyConfig};

    #[derive(Debug, Clone)]
    pub struct InitWorkflow {
        project_root: PathBuf,
    }

    impl InitWorkflow {
        pub fn new(project_root: PathBuf) -> Self {
            Self { project_root }
        }

        pub async fn run(&self) -> Result<()> {
            let root = self.project_root.join(".ai-human");
            let knowledge = root.join("knowledge");
            let workflows = knowledge.join("workflows");
            let memory = root.join("memory");
            let reports = root.join("reports");
            let templates = root.join("templates");

            for dir in [&root, &knowledge, &workflows, &memory, &reports, &templates] {
                fs::create_dir_all(dir).await?;
            }

            write_if_missing(
                &root.join("config.toml"),
                &toml::to_string_pretty(&AiHumanConfig::default())?,
            )
            .await?;
            write_if_missing(
                &root.join("policy.toml"),
                &toml::to_string_pretty(&PolicyConfig::default())?,
            )
            .await?;
            write_if_missing(
                &knowledge.join("README.md"),
                "# AI Human Knowledge Base\n\nProject-owned engineering knowledge for ai-human workflows.\n",
            )
            .await?;
            write_if_missing(
                &knowledge.join("project-map.md"),
                "# Project Map\n\nRecord repositories, services, modules, and common paths here.\n",
            )
            .await?;
            write_if_missing(
                &knowledge.join("engineering-rules.md"),
                "# Engineering Rules\n\nRecord protocol, state, testing, and delivery rules here.\n",
            )
            .await?;
            write_if_missing(
                &workflows.join("requirement-plan.md"),
                "# Requirement Plan Workflow\n\nClassify the task, load rules, inspect context, produce plan, risks, and verification.\n",
            )
            .await?;
            write_if_missing(
                &workflows.join("impact-analysis.md"),
                "# Impact Analysis Workflow\n\nIdentify files, call chains, protocol risks, state risks, persistence risks, and tests.\n",
            )
            .await?;
            write_if_missing(
                &workflows.join("code-review.md"),
                "# Code Review Workflow\n\nReview business correctness, compatibility, state consistency, tests, and maintainability.\n",
            )
            .await?;
            write_if_missing(
                &workflows.join("small-fix.md"),
                "# Small Fix Workflow\n\nConfirm the plan, make the smallest local change, verify it, and write a delivery note.\n",
            )
            .await?;
            write_if_missing(
                &knowledge.join("faq.md"),
                "# FAQ\n\nRecord frequently asked engineering questions and path indexes here.\n",
            )
            .await?;

            for file_name in [
                "tasks.jsonl",
                "decisions.jsonl",
                "reviews.jsonl",
                "learnings.jsonl",
            ] {
                write_if_missing(&memory.join(file_name), "").await?;
            }

            Ok(())
        }
    }

    async fn write_if_missing(path: &Path, contents: &str) -> Result<()> {
        if !fs::try_exists(path).await? {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(path, contents).await?;
        }

        Ok(())
    }
}
