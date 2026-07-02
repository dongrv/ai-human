# AI Digital Human MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the M0-M2 MVP for `ai-human`: a Rust rig powered CLI that initializes project knowledge, loads local context, records task memory, and produces structured `plan`, `impact`, and `review` reports.

**Architecture:** The implementation is workflow-driven rather than free-agent-driven. CLI commands call workflow handlers; workflows depend on small traits for agent calls, context loading, memory, policy, and report writing; concrete filesystem, command, and rig adapters live behind those traits.

**Tech Stack:** Rust 2021, clap, tokio, serde, serde_json, toml, thiserror, anyhow, rig-core 0.36, assert_cmd, assert_fs, insta.

---

## Scope

This plan implements M0, M1, and M2 from the approved design:

- M0: Rust CLI product skeleton and `init`.
- M1: `ask`, `plan`, and `impact` analysis flow.
- M2: `review` flow for diffs or specified files.

This plan creates interface seams for `fix` and `learn`, but does not implement code mutation. M3 and M4 should be planned after M0-M2 pass tests on a real repository.

## Repository Preparation

The implementation worker must start from a valid Git worktree for `https://github.com/dongrv/ai-human.git`.

If the current directory is not a Git worktree, run these commands before Task 1:

```powershell
git init
git remote add origin https://github.com/dongrv/ai-human.git
git status --short
```

Expected: `git status --short` succeeds and shows existing docs as untracked or staged files.

If `git remote add origin` fails because `origin` already exists, run:

```powershell
git remote -v
```

Expected: `origin` points to `https://github.com/dongrv/ai-human.git`.

## File Structure

Create this Rust project structure:

```text
Cargo.toml
src/
  main.rs
  lib.rs
  cli.rs
  config.rs
  core/
    mod.rs
    action.rs
    report.rs
    task.rs
  policy/
    mod.rs
  memory/
    mod.rs
    jsonl.rs
  report/
    mod.rs
    markdown.rs
  context/
    mod.rs
    loader.rs
  agent/
    mod.rs
    mock.rs
    rig_client.rs
  workflow/
    mod.rs
    init.rs
    ask.rs
    plan.rs
    impact.rs
    review.rs
tests/
  cli_smoke.rs
  init_workflow.rs
  memory_jsonl.rs
  context_loader.rs
  workflow_reports.rs
```

Responsibility boundaries:

- `cli.rs`: command parsing only.
- `core/`: serializable domain types shared across workflows.
- `policy/`: action permission rules.
- `memory/`: append-only structured JSONL records.
- `report/`: Markdown report rendering.
- `context/`: local project knowledge and file context loading.
- `agent/`: rig adapter and deterministic mock adapter.
- `workflow/`: workflow orchestration for each command.

## Task 1: Project Skeleton and CLI Smoke Tests

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/cli.rs`
- Create: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing CLI smoke tests**

Create `tests/cli_smoke.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn root_help_lists_core_commands() {
    let mut cmd = Command::cargo_bin("ai-human").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("impact"))
        .stdout(predicate::str::contains("review"));
}

#[test]
fn init_help_mentions_project_root() {
    let mut cmd = Command::cargo_bin("ai-human").unwrap();
    cmd.args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project-root"));
}
```

- [ ] **Step 2: Run smoke tests and verify failure**

Run:

```powershell
cargo test --test cli_smoke
```

Expected: FAIL because `Cargo.toml` and the `ai-human` binary do not exist.

- [ ] **Step 3: Create Cargo manifest**

Create `Cargo.toml`:

```toml
[package]
name = "ai-human"
version = "0.1.0"
edition = "2021"
rust-version = "1.78"
description = "CLI digital human for service-side engineering workflows"
license = "MIT"

[dependencies]
anyhow = "1"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4.5", features = ["derive", "env"] }
rig-core = "0.36"
schemars = { version = "1", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "process", "fs", "io-util"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
uuid = { version = "1", features = ["v4", "serde"] }
walkdir = "2"

[dev-dependencies]
assert_cmd = "2"
assert_fs = "1"
insta = { version = "1", features = ["yaml"] }
predicates = "3"
tempfile = "3"
```

- [ ] **Step 4: Create library module root**

Create `src/lib.rs`:

```rust
pub mod agent;
pub mod cli;
pub mod config;
pub mod context;
pub mod core;
pub mod memory;
pub mod policy;
pub mod report;
pub mod workflow;
```

- [ ] **Step 5: Create CLI definitions**

Create `src/cli.rs`:

```rust
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
```

- [ ] **Step 6: Create executable entrypoint**

Create `src/main.rs`:

```rust
use anyhow::Result;
use clap::Parser;

use ai_human::cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init(_) => {
            println!("init workflow is not wired yet");
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
```

- [ ] **Step 7: Run smoke tests and verify pass**

Run:

```powershell
cargo test --test cli_smoke
```

Expected: PASS.

- [ ] **Step 8: Format and commit**

Run:

```powershell
cargo fmt
cargo test --test cli_smoke
git add Cargo.toml src tests docs
git commit -m "chore: scaffold ai-human cli"
```

Expected: `cargo test --test cli_smoke` passes and Git creates the first implementation commit.

## Task 2: Core Domain Types and Policy Gate

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/action.rs`
- Create: `src/core/task.rs`
- Create: `src/core/report.rs`
- Create: `src/policy/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create core module index**

Create `src/core/mod.rs`:

```rust
pub mod action;
pub mod report;
pub mod task;
```

- [ ] **Step 2: Create action and permission types**

Create `src/core/action.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    L0Read,
    L1Analyze,
    L2LocalWrite,
    L3Verify,
    L4VcsMutate,
    L5ExternalSideEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    ReadFile,
    SearchFiles,
    Analyze,
    WriteLocalFile,
    AppendMemory,
    RunFormatter,
    RunTest,
    GitCommit,
    GitPush,
    DeletePath,
    ModifyProductionConfig,
    ExternalDeployment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRequest {
    pub kind: ActionKind,
    pub description: String,
    pub target: Option<String>,
}

impl ActionKind {
    pub fn permission_level(&self) -> PermissionLevel {
        match self {
            ActionKind::ReadFile | ActionKind::SearchFiles => PermissionLevel::L0Read,
            ActionKind::Analyze => PermissionLevel::L1Analyze,
            ActionKind::WriteLocalFile | ActionKind::AppendMemory => PermissionLevel::L2LocalWrite,
            ActionKind::RunFormatter | ActionKind::RunTest => PermissionLevel::L3Verify,
            ActionKind::GitCommit | ActionKind::GitPush | ActionKind::DeletePath => {
                PermissionLevel::L4VcsMutate
            }
            ActionKind::ModifyProductionConfig | ActionKind::ExternalDeployment => {
                PermissionLevel::L5ExternalSideEffect
            }
        }
    }
}
```

- [ ] **Step 3: Create task memory types**

Create `src/core/task.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Ask,
    Plan,
    ImpactAnalysis,
    CodeReview,
    SmallFix,
    Learn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Started,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: String,
    pub task_type: TaskType,
    pub repo: String,
    pub input: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub summary: String,
    pub report_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub task_id: String,
    pub decision: String,
    pub reason: String,
    pub risk: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRecord {
    pub task_id: String,
    pub severity: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub issue: String,
    pub suggestion: String,
}
```

- [ ] **Step 4: Create report output types**

Create `src/core/report.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanOutput {
    pub title: String,
    pub goal: String,
    pub non_goals: Vec<String>,
    pub affected_areas: Vec<String>,
    pub risks: Vec<String>,
    pub verification_plan: Vec<String>,
    pub open_questions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactOutput {
    pub summary: String,
    pub files: Vec<String>,
    pub call_chains: Vec<String>,
    pub protocol_risks: Vec<String>,
    pub state_risks: Vec<String>,
    pub persistence_risks: Vec<String>,
    pub test_entrypoints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub severity: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub issue: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewOutput {
    pub summary: String,
    pub findings: Vec<ReviewFinding>,
    pub test_gaps: Vec<String>,
    pub residual_risks: Vec<String>,
}
```

- [ ] **Step 5: Create Policy Gate**

Create `src/policy/mod.rs`:

```rust
use crate::core::action::{ActionKind, ActionRequest, PermissionLevel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    RequireConfirmation(String),
    Deny(String),
}

#[derive(Debug, Clone)]
pub struct PolicyGate {
    allow_local_write: bool,
}

impl Default for PolicyGate {
    fn default() -> Self {
        Self {
            allow_local_write: true,
        }
    }
}

impl PolicyGate {
    pub fn evaluate(&self, request: &ActionRequest) -> PolicyDecision {
        match request.kind.permission_level() {
            PermissionLevel::L0Read | PermissionLevel::L1Analyze => PolicyDecision::Allow,
            PermissionLevel::L2LocalWrite if self.allow_local_write => {
                PolicyDecision::RequireConfirmation("local file writes require a task plan confirmation".into())
            }
            PermissionLevel::L3Verify => PolicyDecision::Allow,
            PermissionLevel::L4VcsMutate => match request.kind {
                ActionKind::GitCommit => {
                    PolicyDecision::RequireConfirmation("git commit requires explicit confirmation".into())
                }
                _ => PolicyDecision::Deny("VCS mutation is blocked in V1".into()),
            },
            PermissionLevel::L5ExternalSideEffect => {
                PolicyDecision::Deny("external side effects are outside V1 scope".into())
            }
            PermissionLevel::L2LocalWrite => PolicyDecision::Deny("local writes are disabled".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(kind: ActionKind) -> ActionRequest {
        ActionRequest {
            kind,
            description: "test action".into(),
            target: None,
        }
    }

    #[test]
    fn read_actions_are_allowed() {
        let gate = PolicyGate::default();
        assert_eq!(gate.evaluate(&req(ActionKind::ReadFile)), PolicyDecision::Allow);
    }

    #[test]
    fn local_writes_require_confirmation() {
        let gate = PolicyGate::default();
        assert!(matches!(
            gate.evaluate(&req(ActionKind::WriteLocalFile)),
            PolicyDecision::RequireConfirmation(_)
        ));
    }

    #[test]
    fn external_deployment_is_denied() {
        let gate = PolicyGate::default();
        assert!(matches!(
            gate.evaluate(&req(ActionKind::ExternalDeployment)),
            PolicyDecision::Deny(_)
        ));
    }
}
```

- [ ] **Step 6: Run policy tests**

Run:

```powershell
cargo test policy
```

Expected: PASS.

- [ ] **Step 7: Format and commit**

Run:

```powershell
cargo fmt
cargo test policy
git add src
git commit -m "feat: add core domain and policy gate"
```

Expected: tests pass and Git records the commit.

## Task 3: Configuration and Init Workflow

**Files:**
- Create: `src/config.rs`
- Create: `src/workflow/mod.rs`
- Create: `src/workflow/init.rs`
- Create: `tests/init_workflow.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing init workflow test**

Create `tests/init_workflow.rs`:

```rust
use assert_fs::prelude::*;
use predicates::prelude::*;

use ai_human::workflow::init::InitWorkflow;

#[tokio::test]
async fn init_creates_ai_human_layout() {
    let temp = assert_fs::TempDir::new().unwrap();
    InitWorkflow::new(temp.path().to_path_buf()).run().await.unwrap();

    temp.child(".ai-human/config.toml").assert(predicate::path::exists());
    temp.child(".ai-human/policy.toml").assert(predicate::path::exists());
    temp.child(".ai-human/knowledge/README.md").assert(predicate::path::exists());
    temp.child(".ai-human/knowledge/project-map.md").assert(predicate::path::exists());
    temp.child(".ai-human/knowledge/engineering-rules.md").assert(predicate::path::exists());
    temp.child(".ai-human/memory/tasks.jsonl").assert(predicate::path::exists());
    temp.child(".ai-human/reports").assert(predicate::path::is_dir());
}
```

- [ ] **Step 2: Run init test and verify failure**

Run:

```powershell
cargo test --test init_workflow
```

Expected: FAIL because `workflow::init::InitWorkflow` does not exist.

- [ ] **Step 3: Create configuration types**

Create `src/config.rs`:

```rust
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiHumanConfig {
    pub project_name: String,
    pub model_provider: String,
    pub model_name: String,
    pub knowledge_dir: PathBuf,
    pub memory_dir: PathBuf,
    pub reports_dir: PathBuf,
}

impl Default for AiHumanConfig {
    fn default() -> Self {
        Self {
            project_name: "ai-human".into(),
            model_provider: "openai".into(),
            model_name: "gpt-4o-mini".into(),
            knowledge_dir: ".ai-human/knowledge".into(),
            memory_dir: ".ai-human/memory".into(),
            reports_dir: ".ai-human/reports".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub allow_local_write: bool,
    pub allow_vcs_commit: bool,
    pub allow_external_side_effects: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            allow_local_write: true,
            allow_vcs_commit: false,
            allow_external_side_effects: false,
        }
    }
}
```

- [ ] **Step 4: Create workflow module index**

Create `src/workflow/mod.rs`:

```rust
pub mod ask;
pub mod impact;
pub mod init;
pub mod plan;
pub mod review;
```

Create empty workflow files so the module compiles:

```rust
// src/workflow/ask.rs
```

```rust
// src/workflow/impact.rs
```

```rust
// src/workflow/plan.rs
```

```rust
// src/workflow/review.rs
```

- [ ] **Step 5: Implement init workflow**

Create `src/workflow/init.rs`:

```rust
use std::path::PathBuf;

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

        for dir in [&knowledge, &workflows, &cases, &memory, &reports, &templates] {
            fs::create_dir_all(dir).await?;
        }

        write_if_missing(root.join("config.toml"), &toml::to_string_pretty(&AiHumanConfig::default())?).await?;
        write_if_missing(root.join("policy.toml"), &toml::to_string_pretty(&PolicyConfig::default())?).await?;
        write_if_missing(knowledge.join("README.md"), "# AI Human Knowledge Base\n\nProject-owned engineering knowledge for ai-human workflows.\n").await?;
        write_if_missing(knowledge.join("project-map.md"), "# Project Map\n\nRecord repositories, services, modules, and common paths here.\n").await?;
        write_if_missing(knowledge.join("engineering-rules.md"), "# Engineering Rules\n\nRecord protocol, state, testing, and delivery rules here.\n").await?;
        write_if_missing(workflows.join("requirement-plan.md"), "# Requirement Plan Workflow\n\nClassify the task, load rules, inspect context, produce plan, risks, and verification.\n").await?;
        write_if_missing(workflows.join("impact-analysis.md"), "# Impact Analysis Workflow\n\nIdentify files, call chains, protocol risks, state risks, persistence risks, and tests.\n").await?;
        write_if_missing(workflows.join("code-review.md"), "# Code Review Workflow\n\nReview business correctness, compatibility, state consistency, tests, and maintainability.\n").await?;
        write_if_missing(workflows.join("small-fix.md"), "# Small Fix Workflow\n\nCreate a plan, request confirmation, edit locally, verify, and report.\n").await?;
        write_if_missing(knowledge.join("faq.md"), "# FAQ\n\nCapture frequent engineering questions and answers here.\n").await?;

        for file in ["tasks.jsonl", "decisions.jsonl", "reviews.jsonl", "learnings.jsonl"] {
            write_if_missing(memory.join(file), "").await?;
        }

        Ok(())
    }
}

async fn write_if_missing(path: PathBuf, content: &str) -> Result<()> {
    if fs::try_exists(&path).await? {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(path, content).await?;
    Ok(())
}
```

- [ ] **Step 6: Wire init command**

Replace `src/main.rs` with:

```rust
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
```

- [ ] **Step 7: Run init tests**

Run:

```powershell
cargo test --test init_workflow
cargo test --test cli_smoke
```

Expected: PASS.

- [ ] **Step 8: Format and commit**

Run:

```powershell
cargo fmt
cargo test --test init_workflow
git add src tests
git commit -m "feat: initialize ai-human project layout"
```

Expected: tests pass and Git records the commit.

## Task 4: JSONL Memory Store and Markdown Reports

**Files:**
- Create: `src/memory/mod.rs`
- Create: `src/memory/jsonl.rs`
- Create: `src/report/mod.rs`
- Create: `src/report/markdown.rs`
- Create: `tests/memory_jsonl.rs`

- [ ] **Step 1: Write failing memory test**

Create `tests/memory_jsonl.rs`:

```rust
use assert_fs::prelude::*;
use chrono::Utc;

use ai_human::core::task::{TaskRecord, TaskStatus, TaskType};
use ai_human::memory::jsonl::JsonlMemoryStore;

#[tokio::test]
async fn appends_task_records_as_json_lines() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = JsonlMemoryStore::new(temp.path().join(".ai-human/memory"));

    let record = TaskRecord {
        task_id: "task-1".into(),
        task_type: TaskType::Plan,
        repo: "sample".into(),
        input: "design a feature".into(),
        status: TaskStatus::Completed,
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        summary: "created a plan".into(),
        report_path: Some(".ai-human/reports/task-1-plan.md".into()),
    };

    store.append_task(&record).await.unwrap();

    let content = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/tasks.jsonl"))
        .await
        .unwrap();
    assert!(content.contains("\"task_id\":\"task-1\""));
}
```

- [ ] **Step 2: Run memory test and verify failure**

Run:

```powershell
cargo test --test memory_jsonl
```

Expected: FAIL because `JsonlMemoryStore` does not exist.

- [ ] **Step 3: Create memory module**

Create `src/memory/mod.rs`:

```rust
pub mod jsonl;
```

Create `src/memory/jsonl.rs`:

```rust
use std::path::PathBuf;

use anyhow::Result;
use serde::Serialize;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::core::task::{DecisionRecord, ReviewRecord, TaskRecord};

#[derive(Debug, Clone)]
pub struct JsonlMemoryStore {
    memory_dir: PathBuf,
}

impl JsonlMemoryStore {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self { memory_dir }
    }

    pub async fn append_task(&self, record: &TaskRecord) -> Result<()> {
        self.append_json_line("tasks.jsonl", record).await
    }

    pub async fn append_decision(&self, record: &DecisionRecord) -> Result<()> {
        self.append_json_line("decisions.jsonl", record).await
    }

    pub async fn append_review(&self, record: &ReviewRecord) -> Result<()> {
        self.append_json_line("reviews.jsonl", record).await
    }

    async fn append_json_line<T: Serialize>(&self, file_name: &str, value: &T) -> Result<()> {
        fs::create_dir_all(&self.memory_dir).await?;
        let path = self.memory_dir.join(file_name);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        let line = serde_json::to_string(value)?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }
}
```

- [ ] **Step 4: Create Markdown report renderer**

Create `src/report/mod.rs`:

```rust
pub mod markdown;
```

Create `src/report/markdown.rs`:

```rust
use crate::core::report::{ImpactOutput, PlanOutput, ReviewOutput};

pub fn render_plan(output: &PlanOutput) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", output.title));
    md.push_str(&format!("## Goal\n\n{}\n\n", output.goal));
    push_list(&mut md, "Non Goals", &output.non_goals);
    push_list(&mut md, "Affected Areas", &output.affected_areas);
    push_list(&mut md, "Risks", &output.risks);
    push_list(&mut md, "Verification Plan", &output.verification_plan);
    push_list(&mut md, "Open Questions", &output.open_questions);
    md
}

pub fn render_impact(output: &ImpactOutput) -> String {
    let mut md = String::new();
    md.push_str("# Impact Analysis\n\n");
    md.push_str(&format!("## Summary\n\n{}\n\n", output.summary));
    push_list(&mut md, "Files", &output.files);
    push_list(&mut md, "Call Chains", &output.call_chains);
    push_list(&mut md, "Protocol Risks", &output.protocol_risks);
    push_list(&mut md, "State Risks", &output.state_risks);
    push_list(&mut md, "Persistence Risks", &output.persistence_risks);
    push_list(&mut md, "Test Entrypoints", &output.test_entrypoints);
    md
}

pub fn render_review(output: &ReviewOutput) -> String {
    let mut md = String::new();
    md.push_str("# Code Review\n\n");
    md.push_str(&format!("## Summary\n\n{}\n\n", output.summary));
    md.push_str("## Findings\n\n");
    if output.findings.is_empty() {
        md.push_str("- No findings.\n\n");
    } else {
        for finding in &output.findings {
            let location = match (&finding.file, finding.line) {
                (Some(file), Some(line)) => format!("{file}:{line}"),
                (Some(file), None) => file.clone(),
                (None, _) => "unspecified location".into(),
            };
            md.push_str(&format!(
                "- **{}** `{}`: {} Suggestion: {}\n",
                finding.severity, location, finding.issue, finding.suggestion
            ));
        }
        md.push('\n');
    }
    push_list(&mut md, "Test Gaps", &output.test_gaps);
    push_list(&mut md, "Residual Risks", &output.residual_risks);
    md
}

fn push_list(md: &mut String, title: &str, values: &[String]) {
    md.push_str(&format!("## {title}\n\n"));
    if values.is_empty() {
        md.push_str("- None.\n\n");
        return;
    }

    for value in values {
        md.push_str(&format!("- {value}\n"));
    }
    md.push('\n');
}
```

- [ ] **Step 5: Run memory tests and full current tests**

Run:

```powershell
cargo test --test memory_jsonl
cargo test
```

Expected: PASS.

- [ ] **Step 6: Format and commit**

Run:

```powershell
cargo fmt
cargo test
git add src tests
git commit -m "feat: add jsonl memory and markdown reports"
```

Expected: tests pass and Git records the commit.

## Task 5: Context Loader

**Files:**
- Create: `src/context/mod.rs`
- Create: `src/context/loader.rs`
- Create: `tests/context_loader.rs`

- [ ] **Step 1: Write failing context loader test**

Create `tests/context_loader.rs`:

```rust
use assert_fs::prelude::*;

use ai_human::context::loader::ContextLoader;

#[tokio::test]
async fn loads_project_rules_and_knowledge_files() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("README.md").write_str("# Project\n\nRoot readme.").unwrap();
    temp.child(".agents/README.md").write_str("# Agents\n\nAgent rules.").unwrap();
    temp.child(".ai-human/knowledge/engineering-rules.md")
        .write_str("# Rules\n\nNever skip verification.")
        .unwrap();

    let context = ContextLoader::new(temp.path().to_path_buf())
        .load_for_input("review payment flow")
        .await
        .unwrap();

    assert!(context.combined_text.contains("Root readme."));
    assert!(context.combined_text.contains("Agent rules."));
    assert!(context.combined_text.contains("Never skip verification."));
}
```

- [ ] **Step 2: Run context test and verify failure**

Run:

```powershell
cargo test --test context_loader
```

Expected: FAIL because `ContextLoader` does not exist.

- [ ] **Step 3: Create context module**

Create `src/context/mod.rs`:

```rust
pub mod loader;
```

Create `src/context/loader.rs`:

```rust
use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedContext {
    pub sources: Vec<String>,
    pub combined_text: String,
}

#[derive(Debug, Clone)]
pub struct ContextLoader {
    project_root: PathBuf,
    max_file_bytes: u64,
}

impl ContextLoader {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_file_bytes: 64 * 1024,
        }
    }

    pub async fn load_for_input(&self, input: &str) -> Result<LoadedContext> {
        let mut candidates = vec![
            self.project_root.join("AGENTS.md"),
            self.project_root.join("README.md"),
            self.project_root.join(".agents/README.md"),
            self.project_root.join(".ai-human/knowledge/README.md"),
            self.project_root.join(".ai-human/knowledge/project-map.md"),
            self.project_root.join(".ai-human/knowledge/engineering-rules.md"),
            self.project_root.join(".ai-human/knowledge/faq.md"),
        ];

        candidates.extend(self.knowledge_workflow_files());

        if let Some(path_hint) = first_path_hint(input) {
            candidates.push(self.project_root.join(path_hint));
        }

        let mut sources = Vec::new();
        let mut combined_text = String::new();

        for path in candidates {
            if !fs::try_exists(&path).await? {
                continue;
            }
            let metadata = fs::metadata(&path).await?;
            if !metadata.is_file() || metadata.len() > self.max_file_bytes {
                continue;
            }
            let text = fs::read_to_string(&path).await?;
            let source = normalize_path(&self.project_root, &path);
            sources.push(source.clone());
            combined_text.push_str(&format!("\n\n--- SOURCE: {source} ---\n{text}\n"));
        }

        Ok(LoadedContext {
            sources,
            combined_text,
        })
    }

    fn knowledge_workflow_files(&self) -> Vec<PathBuf> {
        let dir = self.project_root.join(".ai-human/knowledge/workflows");
        if !dir.exists() {
            return Vec::new();
        }

        WalkDir::new(dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
            .collect()
    }
}

fn first_path_hint(input: &str) -> Option<&str> {
    input
        .split_whitespace()
        .find(|token| token.contains('/') || token.contains('\\'))
}

fn normalize_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
```

- [ ] **Step 4: Run context tests**

Run:

```powershell
cargo test --test context_loader
cargo test
```

Expected: PASS.

- [ ] **Step 5: Format and commit**

Run:

```powershell
cargo fmt
cargo test
git add src tests
git commit -m "feat: load project context for workflows"
```

Expected: tests pass and Git records the commit.

## Task 6: Agent Abstraction, Mock Agent, and Rig Adapter

**Files:**
- Create: `src/agent/mod.rs`
- Create: `src/agent/mock.rs`
- Create: `src/agent/rig_client.rs`

- [ ] **Step 1: Create agent trait**

Create `src/agent/mod.rs`:

```rust
pub mod mock;
pub mod rig_client;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub system_prompt: String,
    pub user_prompt: String,
}

#[async_trait]
pub trait AgentClient: Send + Sync {
    async fn complete(&self, request: AgentRequest) -> anyhow::Result<String>;

    async fn complete_json<T>(&self, request: AgentRequest) -> anyhow::Result<T>
    where
        T: DeserializeOwned + Send,
    {
        let raw = self.complete(request).await?;
        let cleaned = strip_json_fence(raw.trim());
        let value = serde_json::from_str(cleaned)?;
        Ok(value)
    }
}

fn strip_json_fence(input: &str) -> &str {
    input
        .strip_prefix("```json")
        .and_then(|rest| rest.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(input)
}
```

- [ ] **Step 2: Create deterministic mock agent**

Create `src/agent/mock.rs`:

```rust
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::agent::{AgentClient, AgentRequest};

#[derive(Debug, Clone)]
pub struct MockAgentClient {
    responses: Arc<Mutex<VecDeque<String>>>,
}

impl MockAgentClient {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
        }
    }
}

#[async_trait]
impl AgentClient for MockAgentClient {
    async fn complete(&self, _request: AgentRequest) -> anyhow::Result<String> {
        let mut responses = self.responses.lock().expect("mock responses mutex poisoned");
        responses
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("mock agent has no queued response"))
    }
}
```

- [ ] **Step 3: Create rig client adapter**

Create `src/agent/rig_client.rs`:

```rust
use async_trait::async_trait;
use rig::{client::{CompletionClient, ProviderClient}, completion::Prompt, providers::openai};

use crate::agent::{AgentClient, AgentRequest};

#[derive(Debug, Clone)]
pub struct RigAgentClient {
    provider: String,
    model: String,
}

impl RigAgentClient {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl AgentClient for RigAgentClient {
    async fn complete(&self, request: AgentRequest) -> anyhow::Result<String> {
        match self.provider.as_str() {
            "openai" => {
                let client = openai::Client::from_env()?;
                let agent = client.agent(&self.model).build();
                let prompt = format!(
                    "{}\n\nUSER REQUEST:\n{}",
                    request.system_prompt, request.user_prompt
                );
                Ok(agent.prompt(prompt).await?)
            }
            other => Err(anyhow::anyhow!("unsupported model provider: {other}")),
        }
    }
}
```

- [ ] **Step 4: Run agent compile check**

Run:

```powershell
cargo check
```

Expected: PASS. If the rig API has changed, inspect the current `rig-core` docs for the replacement imports and keep `AgentClient` unchanged so workflow code remains isolated from the SDK.

- [ ] **Step 5: Commit agent abstraction**

Run:

```powershell
cargo fmt
cargo check
git add src
git commit -m "feat: add agent abstraction and rig adapter"
```

Expected: `cargo check` passes and Git records the commit.

## Task 7: Plan, Impact, and Review Workflows

**Files:**
- Modify: `src/workflow/ask.rs`
- Modify: `src/workflow/plan.rs`
- Modify: `src/workflow/impact.rs`
- Modify: `src/workflow/review.rs`
- Create: `tests/workflow_reports.rs`

- [ ] **Step 1: Write failing workflow tests**

Create `tests/workflow_reports.rs`:

```rust
use assert_fs::prelude::*;

use ai_human::agent::mock::MockAgentClient;
use ai_human::workflow::impact::ImpactWorkflow;
use ai_human::workflow::plan::PlanWorkflow;
use ai_human::workflow::review::ReviewWorkflow;

#[tokio::test]
async fn plan_workflow_writes_markdown_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/knowledge/engineering-rules.md")
        .write_str("# Rules\n\nAlways verify.")
        .unwrap();

    let agent = MockAgentClient::new(vec![r#"{
        "title":"Add payment audit",
        "goal":"Design a safe payment audit change",
        "non_goals":["No deployment"],
        "affected_areas":["service/pay"],
        "risks":["Missing persistence check"],
        "verification_plan":["Run target package tests"],
        "open_questions":["Which table owns audit rows?"]
    }"#.into()]);

    let report = PlanWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run("add payment audit")
        .await
        .unwrap();

    assert!(report.markdown.contains("# Add payment audit"));
    assert!(report.path.ends_with("-plan.md"));
}

#[tokio::test]
async fn impact_workflow_writes_markdown_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"Payment audit touches service and dao",
        "files":["service/pay/audit.go"],
        "call_chains":["controller -> service/pay -> dao"],
        "protocol_risks":["No client protocol change"],
        "state_risks":["No player state change"],
        "persistence_risks":["New audit row ownership"],
        "test_entrypoints":["go test ./service/pay"]
    }"#.into()]);

    let report = ImpactWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run("service/pay/audit.go")
        .await
        .unwrap();

    assert!(report.markdown.contains("Payment audit touches service and dao"));
    assert!(report.path.ends_with("-impact.md"));
}

#[tokio::test]
async fn review_workflow_writes_markdown_report() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("change.diff")
        .write_str("diff --git a/service/pay/audit.go b/service/pay/audit.go")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"One persistence risk found",
        "findings":[{
            "severity":"P1",
            "file":"service/pay/audit.go",
            "line":42,
            "issue":"Audit state is not persisted after mutation",
            "suggestion":"Persist audit state before returning success"
        }],
        "test_gaps":["No regression test for failed persistence"],
        "residual_risks":["Database migration ownership must be confirmed"]
    }"#.into()]);

    let report = ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_diff_file(temp.path().join("change.diff"))
        .await
        .unwrap();

    assert!(report.markdown.contains("P1"));
    assert!(report.markdown.contains("Audit state is not persisted"));
    assert!(report.path.ends_with("-review.md"));
}
```

- [ ] **Step 2: Run workflow tests and verify failure**

Run:

```powershell
cargo test --test workflow_reports
```

Expected: FAIL because workflow structs do not exist.

- [ ] **Step 3: Add shared workflow report type**

Add this to the top of `src/workflow/mod.rs` above module declarations:

```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowReport {
    pub path: String,
    pub markdown: String,
}

fn report_path(project_root: &std::path::Path, task_suffix: &str) -> PathBuf {
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    project_root
        .join(".ai-human")
        .join("reports")
        .join(format!("{date}-{task_suffix}.md"))
}
```

Ensure `src/workflow/mod.rs` still declares:

```rust
pub mod ask;
pub mod impact;
pub mod init;
pub mod plan;
pub mod review;
```

- [ ] **Step 4: Implement plan workflow**

Replace `src/workflow/plan.rs` with:

```rust
use std::path::PathBuf;

use anyhow::Result;
use tokio::fs;

use crate::agent::{AgentClient, AgentRequest};
use crate::context::loader::ContextLoader;
use crate::core::report::PlanOutput;
use crate::report::markdown::render_plan;
use crate::workflow::{report_path, WorkflowReport};

pub struct PlanWorkflow {
    project_root: PathBuf,
    agent: Box<dyn AgentClient>,
}

impl PlanWorkflow {
    pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
        Self { project_root, agent }
    }

    pub async fn run(&self, input: &str) -> Result<WorkflowReport> {
        let context = ContextLoader::new(self.project_root.clone())
            .load_for_input(input)
            .await?;
        let request = AgentRequest {
            system_prompt: PLAN_SYSTEM_PROMPT.into(),
            user_prompt: format!("INPUT:\n{input}\n\nCONTEXT:\n{}", context.combined_text),
        };
        let output: PlanOutput = self.agent.complete_json(request).await?;
        let markdown = render_plan(&output);
        let path = report_path(&self.project_root, "plan");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, &markdown).await?;
        Ok(WorkflowReport {
            path: path.to_string_lossy().replace('\\', "/"),
            markdown,
        })
    }
}

const PLAN_SYSTEM_PROMPT: &str = r#"You are AI数字人 V1, a service-side engineering lead assistant.
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
```

- [ ] **Step 5: Implement impact workflow**

Replace `src/workflow/impact.rs` with:

```rust
use std::path::PathBuf;

use anyhow::Result;
use tokio::fs;

use crate::agent::{AgentClient, AgentRequest};
use crate::context::loader::ContextLoader;
use crate::core::report::ImpactOutput;
use crate::report::markdown::render_impact;
use crate::workflow::{report_path, WorkflowReport};

pub struct ImpactWorkflow {
    project_root: PathBuf,
    agent: Box<dyn AgentClient>,
}

impl ImpactWorkflow {
    pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
        Self { project_root, agent }
    }

    pub async fn run(&self, input: &str) -> Result<WorkflowReport> {
        let context = ContextLoader::new(self.project_root.clone())
            .load_for_input(input)
            .await?;
        let request = AgentRequest {
            system_prompt: IMPACT_SYSTEM_PROMPT.into(),
            user_prompt: format!("INPUT:\n{input}\n\nCONTEXT:\n{}", context.combined_text),
        };
        let output: ImpactOutput = self.agent.complete_json(request).await?;
        let markdown = render_impact(&output);
        let path = report_path(&self.project_root, "impact");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, &markdown).await?;
        Ok(WorkflowReport {
            path: path.to_string_lossy().replace('\\', "/"),
            markdown,
        })
    }
}

const IMPACT_SYSTEM_PROMPT: &str = r#"You are AI数字人 V1, a service-side impact analysis assistant.
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
```

- [ ] **Step 6: Implement review workflow**

Replace `src/workflow/review.rs` with:

```rust
use std::path::PathBuf;

use anyhow::Result;
use tokio::fs;

use crate::agent::{AgentClient, AgentRequest};
use crate::context::loader::ContextLoader;
use crate::core::report::ReviewOutput;
use crate::report::markdown::render_review;
use crate::workflow::{report_path, WorkflowReport};

pub struct ReviewWorkflow {
    project_root: PathBuf,
    agent: Box<dyn AgentClient>,
}

impl ReviewWorkflow {
    pub fn new(project_root: PathBuf, agent: Box<dyn AgentClient>) -> Self {
        Self { project_root, agent }
    }

    pub async fn run_diff_file(&self, diff_file: PathBuf) -> Result<WorkflowReport> {
        let diff_text = fs::read_to_string(&diff_file).await?;
        self.run_text(&format!("DIFF FILE: {}\n\n{diff_text}", diff_file.display()))
            .await
    }

    pub async fn run_text(&self, review_input: &str) -> Result<WorkflowReport> {
        let context = ContextLoader::new(self.project_root.clone())
            .load_for_input(review_input)
            .await?;
        let request = AgentRequest {
            system_prompt: REVIEW_SYSTEM_PROMPT.into(),
            user_prompt: format!("REVIEW INPUT:\n{review_input}\n\nCONTEXT:\n{}", context.combined_text),
        };
        let output: ReviewOutput = self.agent.complete_json(request).await?;
        let markdown = render_review(&output);
        let path = report_path(&self.project_root, "review");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, &markdown).await?;
        Ok(WorkflowReport {
            path: path.to_string_lossy().replace('\\', "/"),
            markdown,
        })
    }
}

const REVIEW_SYSTEM_PROMPT: &str = r#"You are AI数字人 V1, a strict service-side code reviewer.
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
```

- [ ] **Step 7: Implement ask workflow as analysis alias**

Replace `src/workflow/ask.rs` with:

```rust
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
        Self { project_root, agent }
    }

    pub async fn run(&self, input: &str) -> Result<String> {
        let context = ContextLoader::new(self.project_root.clone())
            .load_for_input(input)
            .await?;
        self.agent
            .complete(AgentRequest {
                system_prompt: "You are AI数字人 V1. Answer using the supplied project context. If context is insufficient, say what is missing.".into(),
                user_prompt: format!("QUESTION:\n{input}\n\nCONTEXT:\n{}", context.combined_text),
            })
            .await
    }
}
```

- [ ] **Step 8: Run workflow tests**

Run:

```powershell
cargo test --test workflow_reports
cargo test
```

Expected: PASS.

- [ ] **Step 9: Format and commit**

Run:

```powershell
cargo fmt
cargo test
git add src tests
git commit -m "feat: add analysis and review workflows"
```

Expected: tests pass and Git records the commit.

## Task 8: Wire CLI to Workflows

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update CLI entrypoint**

Replace `src/main.rs` with:

```rust
use anyhow::{bail, Result};
use clap::Parser;

use ai_human::agent::mock::MockAgentClient;
use ai_human::agent::rig_client::RigAgentClient;
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
            let answer = AskWorkflow::new(args.project_root, agent_from_env())
                .run(&args.input)
                .await?;
            println!("{answer}");
        }
        Command::Plan(args) => {
            let report = PlanWorkflow::new(args.project_root, agent_from_env())
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
            let report = ImpactWorkflow::new(args.project_root, agent_from_env())
                .run(&input)
                .await?;
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
        Command::Review(args) => {
            let workflow = ReviewWorkflow::new(args.project_root, agent_from_env());
            let report = match (args.diff_file, args.path) {
                (Some(diff_file), None) => workflow.run_diff_file(diff_file).await?,
                (None, Some(path)) => workflow
                    .run_text(&format!("REVIEW FILE: {}", path.display()))
                    .await?,
                (Some(_), Some(_)) => bail!("use either --diff-file or --path, not both"),
                (None, None) => bail!("review requires --diff-file or --path"),
            };
            println!("{}", report.markdown);
            println!("Report written to {}", report.path);
        }
    }

    Ok(())
}

fn agent_from_env() -> Box<dyn AgentClient> {
    if let Ok(response) = std::env::var("AI_HUMAN_MOCK_RESPONSE") {
        return Box::new(MockAgentClient::new(vec![response]));
    }

    let provider = std::env::var("AI_HUMAN_MODEL_PROVIDER").unwrap_or_else(|_| "openai".into());
    let model = std::env::var("AI_HUMAN_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
    Box::new(RigAgentClient::new(provider, model))
}
```

- [ ] **Step 2: Run CLI tests and full tests**

Run:

```powershell
cargo test --test cli_smoke
cargo test
```

Expected: PASS.

- [ ] **Step 3: Manually verify init command**

Run:

```powershell
cargo run -- init --project-root .
```

Expected: prints `ai-human project initialized` and creates `.ai-human/`.

- [ ] **Step 4: Manually verify mock plan command**

Run:

```powershell
$env:AI_HUMAN_MOCK_RESPONSE='{"title":"Mock Plan","goal":"Verify CLI wiring","non_goals":["No model call"],"affected_areas":["cli"],"risks":["Mock only"],"verification_plan":["cargo test"],"open_questions":[]}'
cargo run -- plan --input "verify cli wiring"
Remove-Item Env:\AI_HUMAN_MOCK_RESPONSE
```

Expected: prints a Markdown plan with `# Mock Plan` and reports a written path ending in `-plan.md`.

- [ ] **Step 5: Format and commit**

Run:

```powershell
cargo fmt
cargo test
git add src tests .ai-human
git commit -m "feat: wire cli workflows"
```

Expected: tests pass and Git records the commit. If `.ai-human` should not be committed, add `.ai-human/memory/` and `.ai-human/reports/` to `.gitignore` before staging generated runtime files.

## Task 9: Documentation, Git Ignore, and Final Verification

**Files:**
- Create: `.gitignore`
- Create: `README.md`
- Modify: `docs/superpowers/specs/2026-07-01-ai-digital-human-design.md` only if implementation discovers a necessary wording correction.

- [ ] **Step 1: Add Git ignore rules**

Create `.gitignore`:

```gitignore
/target/
.ai-human/memory/
.ai-human/reports/
.env
```

- [ ] **Step 2: Create user-facing README**

Create `README.md`:

````markdown
# AI Human

AI Human is a Rust rig powered CLI digital human for service-side engineering workflows.

## MVP Capabilities

- Initialize project-owned AI knowledge folders.
- Load local project rules and Markdown knowledge.
- Produce structured requirement plans.
- Produce impact analysis reports.
- Produce code review reports from diffs or file references.

## Quick Start

```powershell
cargo run -- init --project-root .
```

Set model credentials for real model calls:

```powershell
$env:OPENAI_API_KEY="your-key"
$env:AI_HUMAN_MODEL_PROVIDER="openai"
$env:AI_HUMAN_MODEL="gpt-4o-mini"
```

Generate a plan:

```powershell
cargo run -- plan --input "analyze payment audit requirement"
```

Generate an impact report:

```powershell
cargo run -- impact --input "service/pay audit flow"
```

Review a diff:

```powershell
cargo run -- review --diff-file change.diff
```

## Safety Boundary

The MVP does not auto-commit, push, deploy, mutate production config, or run external side effects. Local write workflows must pass through explicit workflow and policy gates.
````

- [ ] **Step 3: Run full verification**

Run:

```powershell
cargo fmt --check
cargo test
cargo check
```

Expected: all commands pass.

- [ ] **Step 4: Inspect git status**

Run:

```powershell
git status --short
```

Expected: only intended project files are modified or untracked.

- [ ] **Step 5: Commit documentation**

Run:

```powershell
git add .gitignore README.md docs
git commit -m "docs: document ai-human mvp usage"
```

Expected: Git records documentation commit.

## Self-Review Checklist

- Spec coverage: M0, M1, and M2 are implemented through Tasks 1-9.
- Architecture coverage: CLI, core, workflow, agent, context, policy, memory, report, and adapters are split into focused modules.
- SOLID coverage: workflows depend on traits and small modules rather than concrete model or filesystem behavior.
- Safety coverage: policy types and V1 action boundaries exist before code mutation work begins.
- Testing coverage: CLI, init, memory, context loading, and workflow report tests are present.
- Runtime coverage: real model calls use rig through one adapter; tests use deterministic mock responses.

## Execution Notes

- Rig documentation confirms the current crate is `rig-core 0.36.0` and the library is imported as `rig`.
- Rig supports high-level `Agent` abstractions, provider clients, and OpenAI provider integration.
- Keep the `AgentClient` trait stable if rig API details change; update only `src/agent/rig_client.rs`.
- Push to `https://github.com/dongrv/ai-human.git` only after all verification commands pass and the user approves pushing.
