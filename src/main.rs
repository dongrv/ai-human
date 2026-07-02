use anyhow::Result;
use clap::Parser;

use ai_human::cli::{Cli, Command};
use ai_human::workflow::init::InitWorkflow;

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
        Command::Ask(_) => {
            println!("ask workflow is not wired yet");
        }
        Command::Plan(_) => {
            println!("plan workflow is not wired yet");
        }
        Command::Impact(_) => {
            println!("impact workflow is not wired yet");
        }
        Command::Review(_) => {
            println!("review workflow is not wired yet");
        }
    }

    Ok(())
}
