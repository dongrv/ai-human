use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ai-human")]
#[command(about = "CLI digital human for service-side engineering workflows")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init(InitArgs),
    Ask(TextInputArgs),
    Plan(TextInputArgs),
    Impact(ImpactArgs),
    Review(ReviewArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,
}

#[derive(Debug, Args)]
pub struct TextInputArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,

    #[arg(long)]
    pub input: String,
}

#[derive(Debug, Args)]
pub struct ImpactArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,

    #[arg(long)]
    pub input: String,

    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ReviewArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,

    #[arg(long)]
    pub diff_file: Option<PathBuf>,

    #[arg(long)]
    pub path: Option<PathBuf>,
}
