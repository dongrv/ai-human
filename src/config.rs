use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiHumanConfig {
    pub project_name: String,
    pub model_provider: String,
    pub model_name: String,
    pub knowledge_dir: PathBuf,
    pub memory_dir: PathBuf,
    pub reports_dir: PathBuf,
    #[serde(default)]
    pub fix: FixConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixConfig {
    pub default_verify_commands: Vec<String>,
    pub default_format_command: Option<String>,
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
            fix: FixConfig::default(),
        }
    }
}

pub async fn load_project_config(project_root: &std::path::Path) -> Result<AiHumanConfig> {
    let path = project_root.join(".ai-human/config.toml");
    if !fs::try_exists(&path).await? {
        return Ok(AiHumanConfig::default());
    }

    let text = fs::read_to_string(path).await?;
    Ok(toml::from_str(&text)?)
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
