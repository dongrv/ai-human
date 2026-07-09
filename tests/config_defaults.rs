use assert_fs::prelude::*;

use ai_human::config::load_project_config;

#[tokio::test]
async fn load_project_config_returns_defaults_when_missing() {
    let temp = assert_fs::TempDir::new().unwrap();

    let config = load_project_config(temp.path()).await.unwrap();

    assert_eq!(config.project_name, "ai-human");
    assert!(config.fix.default_verify_commands.is_empty());
    assert_eq!(config.fix.default_format_command, None);
}

#[tokio::test]
async fn load_project_config_reads_fix_default_commands() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/config.toml")
        .write_str(
            r#"project_name = "sample"
model_provider = "openai"
model_name = "gpt-4o-mini"
knowledge_dir = ".ai-human/knowledge"
memory_dir = ".ai-human/memory"
reports_dir = ".ai-human/reports"

[fix]
default_verify_commands = ["cargo test", "cargo check"]
default_format_command = "cargo fmt"
"#,
        )
        .unwrap();

    let config = load_project_config(temp.path()).await.unwrap();

    assert_eq!(config.project_name, "sample");
    assert_eq!(
        config.fix.default_verify_commands,
        vec!["cargo test", "cargo check"]
    );
    assert_eq!(config.fix.default_format_command, Some("cargo fmt".into()));
}

#[tokio::test]
async fn load_project_config_accepts_legacy_config_without_fix_section() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/config.toml")
        .write_str(
            r#"project_name = "legacy"
model_provider = "openai"
model_name = "gpt-4o-mini"
knowledge_dir = ".ai-human/knowledge"
memory_dir = ".ai-human/memory"
reports_dir = ".ai-human/reports"
"#,
        )
        .unwrap();

    let config = load_project_config(temp.path()).await.unwrap();

    assert_eq!(config.project_name, "legacy");
    assert!(config.fix.default_verify_commands.is_empty());
    assert_eq!(config.fix.default_format_command, None);
}
