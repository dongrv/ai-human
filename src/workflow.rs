use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Utc;
use tokio::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowReport {
    pub path: String,
    pub markdown: String,
}

fn report_path(project_root: &Path, task_suffix: &str) -> PathBuf {
    let date = Utc::now().format("%Y-%m-%d").to_string();
    project_root
        .join(".ai-human")
        .join("reports")
        .join(format!("{date}-{task_suffix}.md"))
}

fn report_display_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

async fn write_report(path: &Path, markdown: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, markdown).await?;
    Ok(())
}

pub mod ask {
    use std::path::PathBuf;

    use anyhow::Result;

    use crate::agent::{AgentClient, AgentRequest};
    use crate::context::loader::ContextLoader;

    pub struct AskWorkflow {
        project_root: PathBuf,
        agent: Box<dyn AgentClient>,
    }

    impl AskWorkflow {
        pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
            Self {
                project_root,
                agent,
            }
        }

        pub async fn run(&self, input: &str) -> Result<String> {
            let context = ContextLoader::new(self.project_root.clone())
                .load_for_input(input)
                .await?;

            self.agent
                .complete(AgentRequest {
                    system_prompt: ASK_SYSTEM_PROMPT.into(),
                    user_prompt: format!(
                        "QUESTION:\n{input}\n\nCONTEXT:\n{}",
                        context.combined_text
                    ),
                })
                .await
        }
    }

    const ASK_SYSTEM_PROMPT: &str = "You are AI Digital Human V1. Answer using the supplied project context. If context is insufficient, say what is missing.";
}

pub mod plan {
    use std::path::PathBuf;

    use anyhow::Result;

    use crate::agent::{AgentClient, AgentRequest};
    use crate::context::loader::ContextLoader;
    use crate::core::report::PlanOutput;
    use crate::report::markdown::render_plan;
    use crate::workflow::{report_display_path, report_path, write_report, WorkflowReport};

    pub struct PlanWorkflow {
        project_root: PathBuf,
        agent: Box<dyn AgentClient>,
    }

    impl PlanWorkflow {
        pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
            Self {
                project_root,
                agent,
            }
        }

        pub async fn run(&self, input: &str) -> Result<WorkflowReport> {
            let context = ContextLoader::new(self.project_root.clone())
                .load_for_input(input)
                .await?;
            let output: PlanOutput = self
                .agent
                .complete_json(AgentRequest {
                    system_prompt: PLAN_SYSTEM_PROMPT.into(),
                    user_prompt: format!("INPUT:\n{input}\n\nCONTEXT:\n{}", context.combined_text),
                })
                .await?;
            let markdown = render_plan(&output);
            let path = report_path(&self.project_root, "plan");
            write_report(&path, &markdown).await?;

            Ok(WorkflowReport {
                path: report_display_path(&self.project_root, &path),
                markdown,
            })
        }
    }

    const PLAN_SYSTEM_PROMPT: &str = r#"You are AI Digital Human V1, a service-side engineering lead assistant.
Return only JSON matching this schema:
{
  "title": "short plan title",
  "goal": "one clear goal",
  "non_goals": ["items excluded from scope"],
  "affected_areas": ["modules, files, services, protocols"],
  "risks": ["business, protocol, state, persistence, testing risks"],
  "verification_plan": ["concrete verification steps"],
  "open_questions": ["questions requiring human confirmation"]
}
"#;
}

pub mod impact {
    use std::path::PathBuf;

    use anyhow::Result;

    use crate::agent::{AgentClient, AgentRequest};
    use crate::context::loader::ContextLoader;
    use crate::core::report::ImpactOutput;
    use crate::report::markdown::render_impact;
    use crate::workflow::{report_display_path, report_path, write_report, WorkflowReport};

    pub struct ImpactWorkflow {
        project_root: PathBuf,
        agent: Box<dyn AgentClient>,
    }

    impl ImpactWorkflow {
        pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
            Self {
                project_root,
                agent,
            }
        }

        pub async fn run(&self, input: &str) -> Result<WorkflowReport> {
            let context = ContextLoader::new(self.project_root.clone())
                .load_for_input(input)
                .await?;
            let output: ImpactOutput = self
                .agent
                .complete_json(AgentRequest {
                    system_prompt: IMPACT_SYSTEM_PROMPT.into(),
                    user_prompt: format!("INPUT:\n{input}\n\nCONTEXT:\n{}", context.combined_text),
                })
                .await?;
            let markdown = render_impact(&output);
            let path = report_path(&self.project_root, "impact");
            write_report(&path, &markdown).await?;

            Ok(WorkflowReport {
                path: report_display_path(&self.project_root, &path),
                markdown,
            })
        }
    }

    const IMPACT_SYSTEM_PROMPT: &str = r#"You are AI Digital Human V1, a service-side impact analysis assistant.
Return only JSON matching this schema:
{
  "summary": "short impact summary",
  "files": ["likely files"],
  "call_chains": ["important call chains"],
  "protocol_risks": ["protocol compatibility risks"],
  "state_risks": ["state consistency risks"],
  "persistence_risks": ["database/cache/persistence risks"],
  "test_entrypoints": ["specific test or verification commands"]
}
"#;
}

pub mod review {
    use std::path::PathBuf;

    use anyhow::Result;
    use tokio::fs;

    use crate::agent::{AgentClient, AgentRequest};
    use crate::context::loader::ContextLoader;
    use crate::core::report::ReviewOutput;
    use crate::report::markdown::render_review;
    use crate::workflow::{report_display_path, report_path, write_report, WorkflowReport};

    pub struct ReviewWorkflow {
        project_root: PathBuf,
        agent: Box<dyn AgentClient>,
    }

    impl ReviewWorkflow {
        pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
            Self {
                project_root,
                agent,
            }
        }

        pub async fn run_diff_file(&self, diff_file: PathBuf) -> Result<WorkflowReport> {
            let diff_text = fs::read_to_string(&diff_file).await?;
            self.run_text(&format!(
                "DIFF FILE: {}\n\n{diff_text}",
                diff_file.display()
            ))
            .await
        }

        pub async fn run_text(&self, review_input: &str) -> Result<WorkflowReport> {
            let context = ContextLoader::new(self.project_root.clone())
                .load_for_input(review_input)
                .await?;
            let output: ReviewOutput = self
                .agent
                .complete_json(AgentRequest {
                    system_prompt: REVIEW_SYSTEM_PROMPT.into(),
                    user_prompt: format!(
                        "REVIEW INPUT:\n{review_input}\n\nCONTEXT:\n{}",
                        context.combined_text
                    ),
                })
                .await?;
            let markdown = render_review(&output);
            let path = report_path(&self.project_root, "review");
            write_report(&path, &markdown).await?;

            Ok(WorkflowReport {
                path: report_display_path(&self.project_root, &path),
                markdown,
            })
        }
    }

    const REVIEW_SYSTEM_PROMPT: &str = r#"You are AI Digital Human V1, a strict service-side code reviewer.
Prioritize bugs, business correctness, protocol compatibility, state consistency, persistence, missing tests, and maintainability.
Return only JSON matching this schema:
{
  "summary": "short review summary",
  "findings": [{
    "severity": "P0|P1|P2|P3",
    "file": "optional file path",
    "line": 123,
    "issue": "specific problem",
    "suggestion": "specific fix or mitigation"
  }],
  "test_gaps": ["missing tests or verification"],
  "residual_risks": ["risks that remain after review"]
}
"#;
}

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
            let cases = knowledge.join("cases");
            let memory = root.join("memory");
            let reports = root.join("reports");
            let templates = root.join("templates");

            for dir in [
                &root, &knowledge, &workflows, &cases, &memory, &reports, &templates,
            ] {
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
