use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Utc;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowReport {
    pub path: String,
    pub markdown: String,
}

fn report_path(project_root: &Path, task_suffix: &str) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S%.9fZ");
    let id = Uuid::new_v4();
    project_root
        .join(".ai-human")
        .join("reports")
        .join(format!("{timestamp}-{id}-{task_suffix}.md"))
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
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await?;
    file.write_all(markdown.as_bytes()).await?;
    file.flush().await?;
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

pub mod doctor {
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use tokio::fs;

    #[derive(Debug, Clone)]
    pub struct DoctorWorkflow {
        project_root: PathBuf,
    }

    impl DoctorWorkflow {
        pub fn new(project_root: PathBuf) -> Self {
            Self { project_root }
        }

        pub async fn run(&self) -> Result<String> {
            let initialized = required_paths_exist(&self.project_root).await?;
            let mut markdown = String::new();

            markdown.push_str("# AI Human Doctor\n\n");
            markdown.push_str("## Project\n\n");
            markdown.push_str(&format!(
                "- Project root: {}\n",
                self.project_root.display()
            ));
            if initialized {
                markdown.push_str("- Project state: initialized\n\n");
            } else {
                markdown.push_str("- Project state: not initialized\n\n");
            }

            markdown.push_str("## Model\n\n");
            markdown.push_str(&model_status());
            markdown.push('\n');

            markdown.push_str("## Next\n\n");
            if initialized && model_ready() {
                markdown.push_str("- No blocking setup issues detected.\n");
                markdown.push_str(
                    "- Try `ai-human ask --input \"Which module owns this workflow?\"`.\n",
                );
            } else {
                if !initialized {
                    markdown.push_str(&format!(
                        "- Run `ai-human init --project-root {}`.\n",
                        self.project_root.display()
                    ));
                }
                if !model_ready() {
                    markdown.push_str("- Set `OPENAI_API_KEY` in your shell or project `.env`.\n");
                }
            }

            Ok(markdown)
        }
    }

    async fn required_paths_exist(project_root: &Path) -> Result<bool> {
        for path in [
            ".ai-human/config.toml",
            ".ai-human/knowledge/README.md",
            ".ai-human/memory/tasks.jsonl",
            ".ai-human/reports",
        ] {
            if !fs::try_exists(project_root.join(path)).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn model_status() -> String {
        let mut status = String::new();

        if std::env::var_os("AI_HUMAN_MOCK_RESPONSE").is_some() {
            status.push_str("- AI_HUMAN_MOCK_RESPONSE configured\n");
        }
        if std::env::var_os("OPENAI_API_KEY").is_some() {
            status.push_str("- OPENAI_API_KEY configured\n");
        } else {
            status.push_str("- OPENAI_API_KEY missing\n");
        }

        let provider = std::env::var("AI_HUMAN_MODEL_PROVIDER").unwrap_or_else(|_| "openai".into());
        let model = std::env::var("AI_HUMAN_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
        let wire_api =
            std::env::var("AI_HUMAN_OPENAI_WIRE_API").unwrap_or_else(|_| "responses".into());
        status.push_str(&format!("- Provider: {provider}\n"));
        status.push_str(&format!("- Model: {model}\n"));
        status.push_str(&format!("- Wire API: {wire_api}\n"));

        status
    }

    fn model_ready() -> bool {
        std::env::var_os("AI_HUMAN_MOCK_RESPONSE").is_some()
            || std::env::var_os("OPENAI_API_KEY").is_some()
    }
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

pub mod fix {
    use std::path::PathBuf;

    use anyhow::{bail, Result};

    use crate::agent::{AgentClient, AgentRequest};
    use crate::context::loader::ContextLoader;
    use crate::core::report::{FixApplyOutput, FixPlanOutput, VerificationResult};
    use crate::report::markdown::{render_fix_apply, render_fix_plan};
    use crate::tools::command::{CommandResult, CommandRunner};
    use crate::tools::fs::ProjectFs;
    use crate::workflow::{report_display_path, report_path, write_report, WorkflowReport};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FixRequest {
        pub input: String,
        pub path: PathBuf,
        pub verify_commands: Vec<String>,
        pub format_command: Option<String>,
    }

    pub struct FixWorkflow {
        project_root: PathBuf,
        agent: Box<dyn AgentClient>,
    }

    impl FixWorkflow {
        pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
            Self {
                project_root,
                agent,
            }
        }

        pub async fn run_dry_run(&self, request: FixRequest) -> Result<WorkflowReport> {
            let output = self.build_plan(&request).await?;
            let markdown = render_fix_plan(&output);
            let path = report_path(&self.project_root, "fix-dry-run");
            write_report(&path, &markdown).await?;

            Ok(WorkflowReport {
                path: report_display_path(&self.project_root, &path),
                markdown,
            })
        }

        pub async fn run_apply(&self, request: FixRequest) -> Result<WorkflowReport> {
            let output = self.build_plan(&request).await?;
            let replacement = replacement_for_requested_path(&output, &request)?;
            let fs = ProjectFs::new(self.project_root.clone());
            let written_path = fs.write_text(&request.path, &replacement.contents).await?;
            let verification_results = self.run_requested_commands(&request).await?;
            let apply_output = FixApplyOutput {
                summary: output.summary.clone(),
                written_files: vec![written_path],
                verification_results,
                residual_risks: output.risks.clone(),
            };
            let markdown = render_fix_apply(&apply_output);
            let path = report_path(&self.project_root, "fix-apply");
            write_report(&path, &markdown).await?;

            Ok(WorkflowReport {
                path: report_display_path(&self.project_root, &path),
                markdown,
            })
        }

        async fn build_plan(&self, request: &FixRequest) -> Result<FixPlanOutput> {
            let fs = ProjectFs::new(self.project_root.clone());
            let target_file = fs.read_text(&request.path).await?;
            let context = ContextLoader::new(self.project_root.clone())
                .load_for_input(&request.input)
                .await?;

            self.agent
                .complete_json(AgentRequest {
                    system_prompt: FIX_SYSTEM_PROMPT.into(),
                    user_prompt: format!(
                        "INPUT:\n{}\n\nTARGET FILE: {}\n\n{}\n\nUSER VERIFY COMMANDS:\n{}\n\nUSER FORMAT COMMAND:\n{}\n\nCONTEXT:\n{}",
                        request.input,
                        target_file.display_path,
                        target_file.text,
                        format_commands(&request.verify_commands),
                        request.format_command.as_deref().unwrap_or("none"),
                        context.combined_text
                    ),
                })
                .await
        }

        async fn run_requested_commands(
            &self,
            request: &FixRequest,
        ) -> Result<Vec<VerificationResult>> {
            let runner = CommandRunner::new(self.project_root.clone());
            let mut results = Vec::new();

            if let Some(command) = &request.format_command {
                results.push(command_result_to_verification(runner.run(command).await?));
            }
            for command in &request.verify_commands {
                results.push(command_result_to_verification(runner.run(command).await?));
            }

            Ok(results)
        }
    }

    fn replacement_for_requested_path<'a>(
        output: &'a FixPlanOutput,
        request: &FixRequest,
    ) -> Result<&'a crate::core::report::FixReplacementFile> {
        let requested_path = request.path.to_string_lossy().replace('\\', "/");
        let matches = output
            .replacement_files
            .iter()
            .filter(|file| file.path.replace('\\', "/") == requested_path)
            .collect::<Vec<_>>();

        if matches.len() != 1 {
            bail!("model replacement must target exactly {requested_path}");
        }

        Ok(matches[0])
    }

    fn command_result_to_verification(result: CommandResult) -> VerificationResult {
        VerificationResult {
            command: result.command,
            exit_code: result.exit_code,
            succeeded: result.succeeded,
            stdout: result.stdout,
            stderr: result.stderr,
            duration_ms: result.duration_ms,
        }
    }

    fn format_commands(commands: &[String]) -> String {
        if commands.is_empty() {
            return "none".into();
        }

        commands
            .iter()
            .map(|command| format!("- {command}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    const FIX_SYSTEM_PROMPT: &str = r#"You are AI Digital Human V1, a cautious service-side fix planner.
Create a dry-run plan for the smallest safe local code change. Do not claim files were modified.
Return only JSON matching this schema:
{
  "summary": "short fix summary",
  "target_files": ["target files"],
  "change_intent": "what should change and why",
  "risk_level": "low|medium|high",
  "risks": ["business, protocol, state, persistence, or compatibility risks"],
  "verification_commands": ["commands the human should run"],
  "replacement_files": [{"path": "target file path", "contents": "full replacement file content"}],
  "open_questions": ["questions requiring human confirmation"]
}
"#;
}

pub mod learn {
    use std::path::{Path, PathBuf};

    use anyhow::{bail, Result};
    use uuid::Uuid;

    use crate::agent::{AgentClient, AgentRequest};
    use crate::context::loader::ContextLoader;
    use crate::core::report::LearningOutput;
    use crate::core::task::LearningRecord;
    use crate::memory::jsonl::JsonlMemoryStore;
    use crate::report::markdown::render_learning;
    use crate::tools::fs::ProjectFs;
    use crate::workflow::{report_display_path, report_path, write_report, WorkflowReport};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LearnRequest {
        pub input: String,
        pub category: String,
        pub target: Option<String>,
        pub source_report: Option<PathBuf>,
    }

    pub struct LearnWorkflow {
        project_root: PathBuf,
        agent: Box<dyn AgentClient>,
    }

    impl LearnWorkflow {
        pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
            Self {
                project_root,
                agent,
            }
        }

        pub async fn run(&self, request: LearnRequest) -> Result<WorkflowReport> {
            let fs = ProjectFs::new(self.project_root.clone());
            let source_report = match &request.source_report {
                Some(path) => Some(fs.read_text(path).await?),
                None => None,
            };
            let context = ContextLoader::new(self.project_root.clone())
                .load_for_input(&request.input)
                .await?;
            let source_prompt = source_report
                .as_ref()
                .map(|file| format!("SOURCE REPORT: {}\n\n{}", file.display_path, file.text))
                .unwrap_or_else(|| "SOURCE REPORT: none".into());
            let target_hint = request.target.as_deref().unwrap_or("auto");

            let output: LearningOutput = self
                .agent
                .complete_json(AgentRequest {
                    system_prompt: LEARN_SYSTEM_PROMPT.into(),
                    user_prompt: format!(
                        "INPUT:\n{}\n\nCATEGORY:\n{}\n\nTARGET HINT:\n{}\n\n{}\n\nCONTEXT:\n{}",
                        request.input,
                        request.category,
                        target_hint,
                        source_prompt,
                        context.combined_text
                    ),
                })
                .await?;

            let target_doc =
                learning_target_doc(request.target.as_deref(), &request.category, &output)?;
            let entry = render_learning(&output);
            let knowledge_path = fs
                .append_text(Path::new(target_doc), &format!("\n\n{entry}"))
                .await?;
            let memory_path = ".ai-human/memory/learnings.jsonl";
            append_learning_record(&self.project_root, &output, &knowledge_path).await?;

            let mut markdown = entry;
            markdown.push_str("## Saved\n\n");
            markdown.push_str(&format!("- Knowledge written to {knowledge_path}\n"));
            markdown.push_str(&format!("- Memory written to {memory_path}\n"));
            markdown.push_str("- Source code files modified: no\n\n");
            markdown.push_str("## Next\n\n");
            markdown.push_str("- Review the Markdown entry before treating it as a team rule.\n");
            markdown.push_str("- Re-run related `ask`, `plan`, `impact`, or `review` commands to reuse this knowledge.\n");

            let path = report_path(&self.project_root, "learn");
            write_report(&path, &markdown).await?;

            Ok(WorkflowReport {
                path: report_display_path(&self.project_root, &path),
                markdown,
            })
        }
    }

    fn learning_target_doc(
        requested_target: Option<&str>,
        category: &str,
        output: &LearningOutput,
    ) -> Result<&'static str> {
        let raw = requested_target
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                let suggested = output.target_doc.trim();
                if suggested.is_empty() {
                    None
                } else {
                    Some(suggested)
                }
            })
            .unwrap_or(category);
        let normalized = raw.trim().to_ascii_lowercase().replace('_', "-");

        match normalized.as_str() {
            "engineering-rules" | "engineering-rule" | "rules" | "rule" | "decision"
            | "decisions" | "pitfall" | "pitfalls" => Ok(".ai-human/knowledge/engineering-rules.md"),
            "faq" | "question" | "questions" => Ok(".ai-human/knowledge/faq.md"),
            "case" | "cases" => Ok(".ai-human/knowledge/cases/learned-cases.md"),
            _ => bail!(
                "unknown learning target `{raw}`. Use --target engineering-rules, --target faq, or --target case."
            ),
        }
    }

    async fn append_learning_record(
        project_root: &Path,
        output: &LearningOutput,
        knowledge_path: &str,
    ) -> Result<()> {
        let store = JsonlMemoryStore::new(project_root.join(".ai-human/memory"));
        store
            .append_learning(&LearningRecord {
                task_id: format!("learn-{}", Uuid::new_v4()),
                category: output.category.clone(),
                title: output.title.clone(),
                learning: output.rule.clone(),
                target_doc: knowledge_path.into(),
                evidence: output.evidence.clone(),
            })
            .await
    }

    const LEARN_SYSTEM_PROMPT: &str = r#"You are AI Digital Human V1, a service-side engineering knowledge curator.
Turn the supplied input into concise, reusable, reviewable project knowledge.
Return only JSON matching this schema:
{
  "title": "short learning title",
  "category": "rule|faq|case|decision|pitfall",
  "summary": "one or two sentence summary",
  "rule": "the reusable lesson or rule",
  "evidence": ["source facts that support the learning"],
  "applies_to": ["modules, files, services, workflows, or situations"],
  "target_doc": "engineering-rules|faq|case"
}
"#;
}

pub mod review {
    use std::path::{Path, PathBuf};

    use anyhow::{bail, Context, Result};
    use tokio::fs;
    use uuid::Uuid;

    use crate::agent::{AgentClient, AgentRequest};
    use crate::context::loader::ContextLoader;
    use crate::core::report::ReviewOutput;
    use crate::core::task::ReviewRecord;
    use crate::memory::jsonl::JsonlMemoryStore;
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
            let (source, diff_text) = read_project_file(&self.project_root, &diff_file).await?;
            self.run_text(&format!("DIFF FILE: {source}\n\n{diff_text}",))
                .await
        }

        pub async fn run_path(&self, path: PathBuf) -> Result<WorkflowReport> {
            let (source, contents) = read_project_file(&self.project_root, &path).await?;
            self.run_text(&format!("FILE: {source}\n\n{contents}"))
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
            append_review_findings(&self.project_root, &output).await?;

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

    async fn read_project_file(
        project_root: &Path,
        requested_path: &Path,
    ) -> Result<(String, String)> {
        let canonical_root = fs::canonicalize(project_root)
            .await
            .with_context(|| format!("project root does not exist: {}", project_root.display()))?;
        let path = if requested_path.is_absolute() {
            requested_path.to_path_buf()
        } else {
            project_root.join(requested_path)
        };

        let metadata = fs::symlink_metadata(&path).await.with_context(|| {
            format!(
                "path must reference an existing project file: {}",
                requested_path.display()
            )
        })?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() || !file_type.is_file() {
            bail!(
                "path must reference a regular project file: {}",
                requested_path.display()
            );
        }

        let canonical_path = fs::canonicalize(&path).await.with_context(|| {
            format!(
                "path must reference an existing project file: {}",
                requested_path.display()
            )
        })?;
        if !canonical_path.starts_with(&canonical_root) {
            bail!(
                "path must stay inside project root: {}",
                requested_path.display()
            );
        }

        let display_path = canonical_path
            .strip_prefix(&canonical_root)
            .unwrap_or(&canonical_path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&canonical_path)
            .await
            .with_context(|| format!("path must be readable UTF-8 text: {display_path}"))?;

        Ok((display_path, text))
    }

    async fn append_review_findings(project_root: &Path, output: &ReviewOutput) -> Result<()> {
        let task_id = format!("review-{}", Uuid::new_v4());
        let store = JsonlMemoryStore::new(project_root.join(".ai-human/memory"));

        for finding in &output.findings {
            store
                .append_review(&ReviewRecord {
                    task_id: task_id.clone(),
                    severity: finding.severity.clone(),
                    file: finding.file.clone(),
                    line: finding.line,
                    issue: finding.issue.clone(),
                    suggestion: finding.suggestion.clone(),
                })
                .await?;
        }

        Ok(())
    }
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
