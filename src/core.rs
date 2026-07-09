pub mod action {
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
                ActionKind::WriteLocalFile | ActionKind::AppendMemory => {
                    PermissionLevel::L2LocalWrite
                }
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
}

pub mod report {
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

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct LearningOutput {
        pub title: String,
        pub category: String,
        pub summary: String,
        pub rule: String,
        pub evidence: Vec<String>,
        pub applies_to: Vec<String>,
        pub target_doc: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FixReplacementFile {
        pub path: String,
        pub contents: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FixPlanOutput {
        pub summary: String,
        pub target_files: Vec<String>,
        pub change_intent: String,
        pub risk_level: String,
        pub risks: Vec<String>,
        pub verification_commands: Vec<String>,
        pub replacement_files: Vec<FixReplacementFile>,
        pub open_questions: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct VerificationResult {
        pub command: String,
        pub exit_code: Option<i32>,
        pub succeeded: bool,
        pub stdout: String,
        pub stderr: String,
        pub duration_ms: u64,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct FixApplyOutput {
        pub summary: String,
        pub written_files: Vec<String>,
        pub verification_results: Vec<VerificationResult>,
        pub residual_risks: Vec<String>,
    }
}

pub mod task {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TaskId(String);

    impl TaskId {
        pub fn new() -> Self {
            Self(format!("task-{}", Uuid::new_v4()))
        }

        pub fn from_user_input(value: Option<String>) -> Self {
            match value {
                Some(value) if !value.trim().is_empty() => Self(value.trim().into()),
                _ => Self::new(),
            }
        }

        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    impl Default for TaskId {
        fn default() -> Self {
            Self::new()
        }
    }

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

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct LearningRecord {
        pub task_id: String,
        pub category: String,
        pub title: String,
        pub learning: String,
        pub target_doc: String,
        pub evidence: Vec<String>,
    }
}
