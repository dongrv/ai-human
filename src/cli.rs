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
    Learn(LearnArgs),
    Fix(FixArgs),
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

    #[arg(long, help = "Single target file to analyze in Phase 2B")]
    pub path: PathBuf,

    #[arg(long, help = "Preview the fix plan without modifying source files")]
    pub dry_run: bool,

    #[arg(
        long,
        help = "Reserved for Phase 2C; Phase 2B only produces dry-run reports"
    )]
    pub apply: bool,

    #[arg(long, help = "Verification command to include in the dry-run plan")]
    pub verify: Vec<String>,

    #[arg(long, help = "Formatting command to include in the dry-run plan")]
    pub format: Option<String>,
}
