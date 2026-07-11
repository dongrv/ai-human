use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use clap::Parser;

use ai_human::agent::mock::MockAgentClient;
use ai_human::agent::rig_client::{OpenAiWireApi, RigAgentClient};
use ai_human::agent::AgentClient;
use ai_human::cli::{Cli, Command};
use ai_human::cli_output::{
    doctor_result_summary, print_result_summary, print_workflow_report, report_summary,
    ResultSummary,
};
use ai_human::config::load_project_config;
use ai_human::core::task::{EvidenceRecord, RuleHitRecord, TaskId, TaskType};
use ai_human::env::load_project_env;
use ai_human::metrics::{CommandMetric, CommandStatus, MetricsStore};
use ai_human::provider_error::provider_error_hint;
use ai_human::task_index::{
    continuation_from_timeline, evidence_match_counts, render_evidence_query, render_task_timeline,
    CompletedTaskReport, Continuation, EvidenceFilter, TaskIndex, TaskTimeline,
};
use ai_human::workflow::ask::AskWorkflow;
use ai_human::workflow::doctor::DoctorWorkflow;
use ai_human::workflow::fix::{FixRequest, FixWorkflow};
use ai_human::workflow::impact::ImpactWorkflow;
use ai_human::workflow::init::InitWorkflow;
use ai_human::workflow::learn::{LearnRequest, LearnWorkflow};
use ai_human::workflow::plan::PlanWorkflow;
use ai_human::workflow::review::ReviewWorkflow;
use ai_human::workflow::WorkflowReport;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let metric_context = MetricContext::from_command(&cli.command);
    let started = Instant::now();
    let result = run(cli).await;

    record_metric(&metric_context, &result, duration_ms(started)).await;

    result.map(|_| ()).map_err(with_provider_recovery_hint)
}

async fn run(cli: Cli) -> Result<CommandOutcome> {
    match cli.command {
        Command::Init(args) => {
            load_project_env(&args.project_root)?;
            let project_root = args.project_root;
            InitWorkflow::new(project_root.clone()).run().await?;
            println!("ai-human project initialized");
            print_result_summary(ResultSummary {
                summary: "Project initialized.".into(),
                report: "none".into(),
                next_stage: format!(
                    "run `ai-human doctor --project-root {}`",
                    project_root.display()
                ),
            });
            Ok(CommandOutcome::default())
        }
        Command::Ask(args) => {
            load_project_env(&args.project_root)?;
            let answer = AskWorkflow::new(args.project_root, agent_from_env()?)
                .run(&args.input)
                .await?;
            println!("{answer}");
            print_result_summary(ResultSummary {
                summary: "Answer generated.".into(),
                report: "none".into(),
                next_stage: "run `ai-human plan`, `ai-human impact`, or `ai-human learn`".into(),
            });
            Ok(CommandOutcome::default())
        }
        Command::Doctor(args) => {
            load_project_env(&args.project_root)?;
            let report = DoctorWorkflow::new(args.project_root).run().await?;
            println!("{report}");
            print_result_summary(doctor_result_summary(&report));
            Ok(CommandOutcome::default())
        }
        Command::Plan(args) => {
            let project_root = args.project_root;
            load_project_env(&project_root)?;
            let input = args.input;
            let task_id = TaskId::from_user_input(args.task_id);
            let started_at = Utc::now();
            let report = PlanWorkflow::new(project_root.clone(), agent_from_env()?)
                .run_with_task_id(&input, task_id.clone())
                .await?;
            record_completed_task(
                &project_root,
                TaskType::Plan,
                &task_id,
                &input,
                &report,
                started_at,
                Utc::now(),
            )
            .await?;
            print_workflow_report(&report);
            Ok(CommandOutcome::from_report(task_id, &report))
        }
        Command::Impact(args) => {
            let project_root = args.project_root;
            load_project_env(&project_root)?;
            let inherited =
                impact_inheritance(&project_root, &args.from_task, &args.task_id, &args.input)
                    .await?;
            let mut input = args.input.or(inherited.input).ok_or_else(|| {
                anyhow::anyhow!(
                    "impact requires --input or --from-task. Next: pass --input \"describe the change\" or --from-task task-id."
                )
            })?;
            if let Some(path) = args.path {
                input.push_str(&format!("\nPATH: {}", path.display()));
            }

            let task_id = TaskId::from_user_input(args.task_id.or(inherited.task_id));
            let started_at = Utc::now();
            let report = ImpactWorkflow::new(project_root.clone(), agent_from_env()?)
                .run_with_task_id(&input, task_id.clone())
                .await?;
            record_completed_task(
                &project_root,
                TaskType::ImpactAnalysis,
                &task_id,
                &input,
                &report,
                started_at,
                Utc::now(),
            )
            .await?;
            print_workflow_report(&report);
            Ok(CommandOutcome::from_report(task_id, &report))
        }
        Command::Review(args) => {
            let project_root = args.project_root;
            load_project_env(&project_root)?;
            let workflow = ReviewWorkflow::new(project_root.clone(), agent_from_env()?);
            let inherited =
                review_inheritance(&project_root, &args.from_task, &args.path, &args.diff_file)
                    .await?;
            let task_id =
                TaskId::from_user_input(args.task_id.or_else(|| inherited.task_id.clone()));
            let started_at = Utc::now();
            let review_path = args.path.or(inherited.path);
            let (input, report) = match (args.diff_file, review_path) {
                (Some(diff_file), None) => {
                    let input = format!("DIFF FILE: {}", diff_file.display());
                    let report = workflow
                        .run_diff_file_with_task_id(diff_file, task_id.clone())
                        .await?;
                    (input, report)
                }
                (None, Some(path)) => {
                    let input = format!("PATH: {}", path.display());
                    let report = workflow.run_path_with_task_id(path, task_id.clone()).await?;
                    (input, report)
                }
                (Some(_), Some(_)) => bail!(
                    "use either --diff-file or --path, not both. Next: choose one review source."
                ),
                (None, None) => bail!(
                    "review requires --diff-file or --path. Next: pass --path path/to/file or --diff-file path/to/change.diff."
                ),
            };
            record_completed_task(
                &project_root,
                TaskType::CodeReview,
                &task_id,
                &input,
                &report,
                started_at,
                Utc::now(),
            )
            .await?;
            print_workflow_report(&report);
            Ok(CommandOutcome::from_report(task_id, &report))
        }
        Command::Learn(args) => {
            let project_root = args.project_root;
            load_project_env(&project_root)?;
            let inherited = learn_inheritance(
                &project_root,
                &args.from_task,
                &args.task_id,
                &args.source_report,
            )
            .await?;
            let request = LearnRequest {
                input: args.input,
                category: args.category,
                target: args.target,
                source_report: args.source_report.or(inherited.source_report),
            };
            let input = request.input.clone();
            let task_id = TaskId::from_user_input(args.task_id.or(inherited.task_id));
            let started_at = Utc::now();
            let report = LearnWorkflow::new(project_root.clone(), agent_from_env()?)
                .run_with_task_id(request, task_id.clone())
                .await?;
            record_completed_task(
                &project_root,
                TaskType::Learn,
                &task_id,
                &input,
                &report,
                started_at,
                Utc::now(),
            )
            .await?;
            print_workflow_report(&report);
            Ok(CommandOutcome::from_report(task_id, &report))
        }
        Command::Fix(args) => {
            let project_root = args.project_root;
            load_project_env(&project_root)?;
            let config = load_project_config(&project_root).await?;
            let inherited = fix_inheritance(
                &project_root,
                &args.from_task,
                &args.task_id,
                &args.input,
                &args.path,
            )
            .await?;
            let apply = args.apply;
            let verify_commands = if args.verify.is_empty() {
                config.fix.default_verify_commands
            } else {
                args.verify
            };
            let format_command = args.format.or(config.fix.default_format_command);
            let request = FixRequest {
                input: args.input.or(inherited.input).ok_or_else(|| {
                    anyhow::anyhow!(
                        "fix requires --input or --from-task. Next: pass --input \"describe the fix\" and --path file, or --from-task task-id."
                    )
                })?,
                path: args.path.or(inherited.path).ok_or_else(|| {
                    anyhow::anyhow!(
                        "fix requires --path or --from-task. Next: pass --path file or --from-task task-id."
                    )
                })?,
                verify_commands,
                format_command,
            };
            let input = format!("{}\nPATH: {}", request.input, request.path.display());
            let workflow = FixWorkflow::new(project_root.clone(), agent_from_env()?);
            let task_id = TaskId::from_user_input(args.task_id.or(inherited.task_id));
            let started_at = Utc::now();
            let report = if apply {
                workflow
                    .run_apply_with_task_id(request, task_id.clone())
                    .await?
            } else {
                workflow
                    .run_dry_run_with_task_id(request, task_id.clone())
                    .await?
            };
            record_completed_task(
                &project_root,
                TaskType::SmallFix,
                &task_id,
                &input,
                &report,
                started_at,
                Utc::now(),
            )
            .await?;
            print_workflow_report(&report);
            Ok(CommandOutcome::from_report(task_id, &report))
        }
        Command::Task(args) => {
            load_project_env(&args.project_root)?;
            let timeline = TaskIndex::new(&args.project_root).load(&args.id).await?;
            println!("{}", render_task_timeline(&timeline));
            print_result_summary(task_result_summary(&timeline));
            Ok(CommandOutcome {
                task_id: Some(args.id),
                report_path: None,
            })
        }
        Command::Evidence(args) => {
            load_project_env(&args.project_root)?;
            let timeline = TaskIndex::new(&args.project_root)
                .load(&args.task_id)
                .await?;
            let filter = EvidenceFilter {
                kind: args.kind,
                source: args.source,
            };
            println!("{}", render_evidence_query(&timeline, &filter));
            print_result_summary(evidence_result_summary(&timeline, &filter));
            Ok(CommandOutcome {
                task_id: Some(args.task_id),
                report_path: None,
            })
        }
    }
}

fn agent_from_env() -> Result<Box<dyn AgentClient>> {
    if let Ok(response) = std::env::var("AI_HUMAN_MOCK_RESPONSE") {
        return Ok(Box::new(MockAgentClient::new(vec![response])));
    }

    let provider = std::env::var("AI_HUMAN_MODEL_PROVIDER").unwrap_or_else(|_| "openai".into());
    let model = std::env::var("AI_HUMAN_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
    let openai_wire_api = std::env::var("AI_HUMAN_OPENAI_WIRE_API")
        .map(|value| OpenAiWireApi::parse(&value))
        .unwrap_or_else(|_| Ok(OpenAiWireApi::default()))?;

    Ok(Box::new(RigAgentClient::with_openai_wire_api(
        provider,
        model,
        openai_wire_api,
    )))
}

fn with_provider_recovery_hint(error: anyhow::Error) -> anyhow::Error {
    let message = error.to_string();
    if let Some(hint) = provider_error_hint(&message) {
        error.context(hint)
    } else {
        error
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CommandOutcome {
    task_id: Option<String>,
    report_path: Option<String>,
}

impl CommandOutcome {
    fn from_report(task_id: TaskId, report: &WorkflowReport) -> Self {
        Self {
            task_id: Some(task_id.as_str().into()),
            report_path: Some(report.path.clone()),
        }
    }
}

#[derive(Debug, Clone)]
struct MetricContext {
    command: String,
    project_root: PathBuf,
    task_id: Option<String>,
}

impl MetricContext {
    fn from_command(command: &Command) -> Self {
        match command {
            Command::Init(args) => Self::new("init", args.project_root.clone(), None),
            Command::Ask(args) => Self::new("ask", args.project_root.clone(), None),
            Command::Plan(args) => {
                Self::new("plan", args.project_root.clone(), args.task_id.clone())
            }
            Command::Impact(args) => Self::new(
                "impact",
                args.project_root.clone(),
                args.task_id.clone().or_else(|| args.from_task.clone()),
            ),
            Command::Review(args) => Self::new(
                "review",
                args.project_root.clone(),
                args.task_id.clone().or_else(|| args.from_task.clone()),
            ),
            Command::Learn(args) => Self::new(
                "learn",
                args.project_root.clone(),
                args.task_id.clone().or_else(|| args.from_task.clone()),
            ),
            Command::Fix(args) => Self::new(
                "fix",
                args.project_root.clone(),
                args.task_id.clone().or_else(|| args.from_task.clone()),
            ),
            Command::Task(args) => {
                Self::new("task", args.project_root.clone(), Some(args.id.clone()))
            }
            Command::Evidence(args) => Self::new(
                "evidence",
                args.project_root.clone(),
                Some(args.task_id.clone()),
            ),
            Command::Doctor(args) => Self::new("doctor", args.project_root.clone(), None),
        }
    }

    fn new(command: &str, project_root: PathBuf, task_id: Option<String>) -> Self {
        Self {
            command: command.into(),
            project_root,
            task_id,
        }
    }
}

async fn record_completed_task(
    project_root: &Path,
    task_type: TaskType,
    task_id: &TaskId,
    input: &str,
    report: &WorkflowReport,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
) -> Result<()> {
    TaskIndex::new(project_root)
        .append_completed_report(CompletedTaskReport {
            task_id: task_id.as_str().into(),
            task_type,
            repo: repo_label(project_root),
            input: input.into(),
            summary: report_summary(&report.markdown),
            report_path: report.path.clone(),
            evidence: report
                .evidence
                .iter()
                .map(|entry| EvidenceRecord {
                    kind: entry.kind_label().into(),
                    detail: entry.detail.clone(),
                })
                .collect(),
            rule_hits: report
                .rule_hits
                .iter()
                .map(|hit| RuleHitRecord {
                    source: hit.source.clone(),
                    detail: hit.detail.clone(),
                })
                .collect(),
            started_at,
            completed_at,
        })
        .await
}

fn task_result_summary(timeline: &TaskTimeline) -> ResultSummary {
    let summary = if timeline.records.is_empty() {
        "No task records found.".into()
    } else {
        format!("{} task record(s) found.", timeline.records.len())
    };
    let next_stage = if timeline.records.is_empty() {
        format!(
            "run `ai-human plan --task-id {} --input \"describe the change\"`",
            timeline.task_id
        )
    } else {
        "open the latest report or continue with the listed command".into()
    };

    ResultSummary {
        summary,
        report: "none".into(),
        next_stage,
    }
}

fn evidence_result_summary(timeline: &TaskTimeline, filter: &EvidenceFilter) -> ResultSummary {
    let (evidence_matches, rule_hit_matches) = evidence_match_counts(timeline, filter);
    let summary = format!(
        "{} evidence match(es), {} rule hit match(es).",
        evidence_matches, rule_hit_matches
    );
    let next_stage = if timeline.records.is_empty() {
        format!(
            "run `ai-human plan --task-id {} --input \"describe the change\"`",
            timeline.task_id
        )
    } else {
        format!(
            "run `ai-human task --id {}` for the full task timeline",
            timeline.task_id
        )
    };

    ResultSummary {
        summary,
        report: "none".into(),
        next_stage,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ImpactInheritance {
    task_id: Option<String>,
    input: Option<String>,
}

async fn impact_inheritance(
    project_root: &Path,
    from_task: &Option<String>,
    task_id: &Option<String>,
    input: &Option<String>,
) -> Result<ImpactInheritance> {
    let Some(from_task_id) = from_task else {
        return Ok(ImpactInheritance::default());
    };
    if task_id.is_some() || input.is_some() {
        bail!(
            "use either --from-task or explicit impact options, not both. Next: remove --task-id/--input or remove --from-task."
        );
    }

    let timeline = TaskIndex::new(project_root).load(from_task_id).await?;
    match continuation_from_timeline(&timeline) {
        Some(Continuation::ImpactInput { task_id, input }) => Ok(ImpactInheritance {
            task_id: Some(task_id),
            input: Some(input),
        }),
        _ => bail!(
            "task `{}` does not have an inheritable impact input. Next: run `ai-human task --id {}` and follow its Suggested command, or pass --input explicitly.",
            from_task_id,
            from_task_id
        ),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ReviewInheritance {
    task_id: Option<String>,
    path: Option<PathBuf>,
}

async fn review_inheritance(
    project_root: &Path,
    from_task: &Option<String>,
    path: &Option<PathBuf>,
    diff_file: &Option<PathBuf>,
) -> Result<ReviewInheritance> {
    let Some(task_id) = from_task else {
        return Ok(ReviewInheritance::default());
    };
    if path.is_some() || diff_file.is_some() {
        bail!(
            "use either --from-task or an explicit review source, not both. Next: remove --path/--diff-file or remove --from-task."
        );
    }

    let timeline = TaskIndex::new(project_root).load(task_id).await?;
    match continuation_from_timeline(&timeline) {
        Some(Continuation::ReviewPath { task_id, path }) => Ok(ReviewInheritance {
            task_id: Some(task_id),
            path: Some(PathBuf::from(path)),
        }),
        _ => bail!(
            "task `{}` does not have an inheritable review source. Next: run `ai-human task --id {}` and follow its Suggested command, or pass --path explicitly.",
            task_id,
            task_id
        ),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FixInheritance {
    task_id: Option<String>,
    input: Option<String>,
    path: Option<PathBuf>,
}

async fn fix_inheritance(
    project_root: &Path,
    from_task: &Option<String>,
    task_id: &Option<String>,
    input: &Option<String>,
    path: &Option<PathBuf>,
) -> Result<FixInheritance> {
    let Some(from_task_id) = from_task else {
        return Ok(FixInheritance::default());
    };
    if task_id.is_some() || input.is_some() || path.is_some() {
        bail!(
            "use either --from-task or explicit fix options, not both. Next: remove --task-id/--input/--path or remove --from-task."
        );
    }

    let timeline = TaskIndex::new(project_root).load(from_task_id).await?;
    match continuation_from_timeline(&timeline) {
        Some(Continuation::FixPath {
            task_id,
            input,
            path,
        }) => Ok(FixInheritance {
            task_id: Some(task_id),
            input: Some(input),
            path: Some(PathBuf::from(path)),
        }),
        _ => bail!(
            "task `{}` does not have an inheritable fix path. Next: run `ai-human task --id {}` and follow its Suggested command, or pass --input and --path explicitly.",
            from_task_id,
            from_task_id
        ),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LearnInheritance {
    task_id: Option<String>,
    source_report: Option<PathBuf>,
}

async fn learn_inheritance(
    project_root: &Path,
    from_task: &Option<String>,
    task_id: &Option<String>,
    source_report: &Option<PathBuf>,
) -> Result<LearnInheritance> {
    let Some(from_task_id) = from_task else {
        return Ok(LearnInheritance::default());
    };
    if task_id.is_some() || source_report.is_some() {
        bail!(
            "use either --from-task or explicit task/source options, not both. Next: remove --task-id/--source-report or remove --from-task."
        );
    }

    let timeline = TaskIndex::new(project_root).load(from_task_id).await?;
    match continuation_from_timeline(&timeline) {
        Some(Continuation::LearnSourceReport {
            task_id,
            source_report,
        }) => Ok(LearnInheritance {
            task_id: Some(task_id),
            source_report: Some(PathBuf::from(source_report)),
        }),
        _ => bail!(
            "task `{}` does not have an inheritable learning source report. Next: run `ai-human task --id {}` and follow its Suggested command, or pass --source-report explicitly.",
            from_task_id,
            from_task_id
        ),
    }
}

async fn record_metric(context: &MetricContext, result: &Result<CommandOutcome>, duration_ms: u64) {
    let status = if result.is_ok() {
        CommandStatus::Success
    } else {
        CommandStatus::Failed
    };
    let task_id = result
        .as_ref()
        .ok()
        .and_then(|outcome| outcome.task_id.clone())
        .or_else(|| context.task_id.clone());
    let report_path = result
        .as_ref()
        .ok()
        .and_then(|outcome| outcome.report_path.clone());
    let error = result.as_ref().err().map(error_summary);
    let metric = CommandMetric {
        command: context.command.clone(),
        status,
        duration_ms,
        task_id,
        report_path,
        error,
    };

    if let Err(err) = MetricsStore::new(&context.project_root)
        .append(&metric)
        .await
    {
        tracing::warn!(error = ?err, command = %context.command, "failed to write command metric");
    }
}

fn error_summary(error: &anyhow::Error) -> String {
    error
        .to_string()
        .lines()
        .next()
        .unwrap_or("command failed")
        .into()
}

fn duration_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn repo_label(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("project")
        .into()
}
