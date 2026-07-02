use anyhow::{bail, Result};
use clap::Parser;

use ai_human::agent::mock::MockAgentClient;
use ai_human::agent::rig_client::{OpenAiWireApi, RigAgentClient};
use ai_human::agent::AgentClient;
use ai_human::cli::{Cli, Command};
use ai_human::workflow::ask::AskWorkflow;
use ai_human::workflow::impact::ImpactWorkflow;
use ai_human::workflow::init::InitWorkflow;
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
            InitWorkflow::new(args.project_root).run().await?;
            println!("ai-human project initialized");
        }
        Command::Ask(args) => {
            let answer = AskWorkflow::new(args.project_root, agent_from_env()?)
                .run(&args.input)
                .await?;
            println!("{answer}");
        }
        Command::Plan(args) => {
            let report = PlanWorkflow::new(args.project_root, agent_from_env()?)
                .run(&args.input)
                .await?;
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
        Command::Impact(args) => {
            let mut input = args.input;
            if let Some(path) = args.path {
                input.push_str(&format!("\nPATH: {}", path.display()));
            }

            let report = ImpactWorkflow::new(args.project_root, agent_from_env()?)
                .run(&input)
                .await?;
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
        Command::Review(args) => {
            let workflow = ReviewWorkflow::new(args.project_root, agent_from_env()?);
            let report = match (args.diff_file, args.path) {
                (Some(diff_file), None) => workflow.run_diff_file(diff_file).await?,
                (None, Some(path)) => workflow.run_path(path).await?,
                (Some(_), Some(_)) => bail!("use either --diff-file or --path, not both"),
                (None, None) => bail!("review requires --diff-file or --path"),
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
