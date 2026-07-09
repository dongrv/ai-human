use anyhow::{bail, Result};
use clap::Parser;

use ai_human::agent::mock::MockAgentClient;
use ai_human::agent::rig_client::{OpenAiWireApi, RigAgentClient};
use ai_human::agent::AgentClient;
use ai_human::cli::{Cli, Command};
use ai_human::config::load_project_config;
use ai_human::core::task::TaskId;
use ai_human::env::load_project_env;
use ai_human::workflow::ask::AskWorkflow;
use ai_human::workflow::doctor::DoctorWorkflow;
use ai_human::workflow::fix::{FixRequest, FixWorkflow};
use ai_human::workflow::impact::ImpactWorkflow;
use ai_human::workflow::init::InitWorkflow;
use ai_human::workflow::learn::{LearnRequest, LearnWorkflow};
use ai_human::workflow::plan::PlanWorkflow;
use ai_human::workflow::review::ReviewWorkflow;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init(args) => {
            load_project_env(&args.project_root)?;
            InitWorkflow::new(args.project_root).run().await?;
            println!("ai-human project initialized");
        }
        Command::Ask(args) => {
            load_project_env(&args.project_root)?;
            let answer = AskWorkflow::new(args.project_root, agent_from_env()?)
                .run(&args.input)
                .await?;
            println!("{answer}");
        }
        Command::Doctor(args) => {
            load_project_env(&args.project_root)?;
            let report = DoctorWorkflow::new(args.project_root).run().await?;
            println!("{report}");
        }
        Command::Plan(args) => {
            load_project_env(&args.project_root)?;
            let report = PlanWorkflow::new(args.project_root, agent_from_env()?)
                .run_with_task_id(&args.input, TaskId::from_user_input(args.task_id))
                .await?;
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
        Command::Impact(args) => {
            load_project_env(&args.project_root)?;
            let mut input = args.input;
            if let Some(path) = args.path {
                input.push_str(&format!("\nPATH: {}", path.display()));
            }

            let report = ImpactWorkflow::new(args.project_root, agent_from_env()?)
                .run_with_task_id(&input, TaskId::from_user_input(args.task_id))
                .await?;
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
        Command::Review(args) => {
            load_project_env(&args.project_root)?;
            let workflow = ReviewWorkflow::new(args.project_root, agent_from_env()?);
            let task_id = TaskId::from_user_input(args.task_id);
            let report = match (args.diff_file, args.path) {
                (Some(diff_file), None) => {
                    workflow
                        .run_diff_file_with_task_id(diff_file, task_id)
                        .await?
                }
                (None, Some(path)) => workflow.run_path_with_task_id(path, task_id).await?,
                (Some(_), Some(_)) => bail!("use either --diff-file or --path, not both"),
                (None, None) => bail!("review requires --diff-file or --path"),
            };
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
        Command::Learn(args) => {
            load_project_env(&args.project_root)?;
            let report = LearnWorkflow::new(args.project_root, agent_from_env()?)
                .run_with_task_id(
                    LearnRequest {
                        input: args.input,
                        category: args.category,
                        target: args.target,
                        source_report: args.source_report,
                    },
                    TaskId::from_user_input(args.task_id),
                )
                .await?;
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
        Command::Fix(args) => {
            load_project_env(&args.project_root)?;
            let config = load_project_config(&args.project_root).await?;
            let apply = args.apply;
            let verify_commands = if args.verify.is_empty() {
                config.fix.default_verify_commands
            } else {
                args.verify
            };
            let format_command = args.format.or(config.fix.default_format_command);
            let request = FixRequest {
                input: args.input,
                path: args.path,
                verify_commands,
                format_command,
            };
            let workflow = FixWorkflow::new(args.project_root, agent_from_env()?);
            let task_id = TaskId::from_user_input(args.task_id);
            let report = if apply {
                workflow.run_apply_with_task_id(request, task_id).await?
            } else {
                workflow.run_dry_run_with_task_id(request, task_id).await?
            };
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
    }

    Ok(())
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
