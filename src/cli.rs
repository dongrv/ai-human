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
    #[command(about = "Create .ai-human project state")]
    Init(InitArgs),
    #[command(about = "Answer a project question using local context")]
    Ask(AskArgs),
    #[command(about = "Create a requirement or change plan")]
    Plan(PlanArgs),
    #[command(about = "Analyze files, call chains, risks, and tests")]
    Impact(ImpactArgs),
    #[command(about = "Review a diff or file for engineering risks")]
    Review(ReviewArgs),
    #[command(about = "Save reusable project knowledge")]
    Learn(LearnArgs),
    #[command(about = "Preview or apply one-file local fixes")]
    Fix(FixArgs),
    #[command(about = "Check project setup and model configuration")]
    Doctor(DoctorArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long, default_value = ".", help = "Project root to inspect")]
    pub project_root: PathBuf,
}

#[derive(Debug, Args)]
pub struct AskArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,

    #[arg(long)]
    pub input: String,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,

    #[arg(long)]
    pub input: String,

    #[arg(long, help = "Optional task id used to connect related reports")]
    pub task_id: Option<String>,
}

#[derive(Debug, Args)]
pub struct ImpactArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,

    #[arg(long)]
    pub input: String,

    #[arg(long, help = "Optional task id used to connect related reports")]
    pub task_id: Option<String>,

    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ReviewArgs {
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,

    #[arg(long, help = "Optional task id used to connect related reports")]
    pub task_id: Option<String>,

    #[arg(long)]
    pub diff_file: Option<PathBuf>,

    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct LearnArgs {
    #[arg(
        long,
        default_value = ".",
        help = "Project root that owns .ai-human state"
    )]
    pub project_root: PathBuf,

    #[arg(long, help = "Lesson, conclusion, review finding, or rule to save")]
    pub input: String,

    #[arg(long, help = "Optional task id used to connect related reports")]
    pub task_id: Option<String>,

    #[arg(
        long,
        default_value = "rule",
        help = "Learning category, for example rule, faq, case, decision, or pitfall"
    )]
    pub category: String,

    #[arg(
        long,
        help = "Knowledge target, for example engineering-rules, faq, or case"
    )]
    pub target: Option<String>,

    #[arg(
        long,
        help = "Optional .ai-human report file to include as source material"
    )]
    pub source_report: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct FixArgs {
    #[arg(
        long,
        default_value = ".",
        help = "Project root that owns the target file and .ai-human state"
    )]
    pub project_root: PathBuf,

    #[arg(long, help = "Problem statement or desired small local fix")]
    pub input: String,

    #[arg(long, help = "Optional task id used to connect related reports")]
    pub task_id: Option<String>,

    #[arg(long, help = "Single target file to analyze or update")]
    pub path: PathBuf,

    #[arg(long, help = "Preview the fix plan without modifying source files")]
    pub dry_run: bool,

    #[arg(long, help = "Apply a validated replacement to the target file")]
    pub apply: bool,

    #[arg(
        long,
        help = "Verification command to run after --apply or include in dry-run"
    )]
    pub verify: Vec<String>,

    #[arg(
        long,
        help = "Formatting command to run after --apply or include in dry-run"
    )]
    pub format: Option<String>,
}
