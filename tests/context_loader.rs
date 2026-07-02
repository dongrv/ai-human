use assert_fs::prelude::*;

use ai_human::context::loader::ContextLoader;

#[tokio::test]
async fn loads_project_rules_and_knowledge_files() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("README.md")
        .write_str("# Project\n\nRoot readme.")
        .unwrap();
    temp.child(".agents/README.md")
        .write_str("# Agents\n\nAgent rules.")
        .unwrap();
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
    assert_eq!(
        context.sources,
        vec![
            "README.md",
            ".agents/README.md",
            ".ai-human/knowledge/engineering-rules.md"
        ]
    );
}

#[tokio::test]
async fn loads_existing_workflows_and_input_path_hint() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/knowledge/workflows/code-review.md")
        .write_str("# Code Review\n\nReview for persistence risk.")
        .unwrap();
    temp.child("src/payment.rs")
        .write_str("pub fn settle_payment() {}")
        .unwrap();

    let context = ContextLoader::new(temp.path().to_path_buf())
        .load_for_input("inspect src/payment.rs")
        .await
        .unwrap();

    assert!(context
        .combined_text
        .contains("Review for persistence risk."));
    assert!(context.combined_text.contains("settle_payment"));
    assert!(context
        .sources
        .contains(&".ai-human/knowledge/workflows/code-review.md".into()));
    assert!(context.sources.contains(&"src/payment.rs".into()));
}

#[tokio::test]
async fn skips_missing_files_and_files_over_the_size_limit() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("README.md")
        .write_str("# Project\n\nSmall enough.")
        .unwrap();
    temp.child(".ai-human/knowledge/faq.md")
        .write_str(&"x".repeat(65 * 1024))
        .unwrap();

    let context = ContextLoader::new(temp.path().to_path_buf())
        .load_for_input("ask about missing/path.rs")
        .await
        .unwrap();

    assert!(context.combined_text.contains("Small enough."));
    assert!(!context.combined_text.contains(&"x".repeat(1024)));
    assert_eq!(context.sources, vec!["README.md"]);
}

#[tokio::test]
async fn skips_symlinked_path_hints() {
    let project = assert_fs::TempDir::new().unwrap();
    let outside = assert_fs::TempDir::new().unwrap();
    outside
        .child("secret.md")
        .write_str("outside project secret")
        .unwrap();

    let link_path = project.path().join("linked-secret.md");
    if create_file_symlink(outside.path().join("secret.md"), &link_path).is_err() {
        return;
    }

    let context = ContextLoader::new(project.path().to_path_buf())
        .load_for_input("inspect linked-secret.md")
        .await
        .unwrap();

    assert!(!context.combined_text.contains("outside project secret"));
    assert!(!context.sources.contains(&"linked-secret.md".into()));
}

#[tokio::test]
async fn rejects_lexical_parent_traversal_path_hints() {
    let project = assert_fs::TempDir::new().unwrap();
    let outside = assert_fs::TempDir::new().unwrap();
    outside
        .child("outside.md")
        .write_str("parent traversal secret")
        .unwrap();

    let input = format!(
        "inspect ../{}/outside.md",
        outside.path().file_name().unwrap().to_string_lossy()
    );
    let context = ContextLoader::new(project.path().to_path_buf())
        .load_for_input(&input)
        .await
        .unwrap();

    assert!(!context.combined_text.contains("parent traversal secret"));
    assert!(context.sources.is_empty());
}

#[cfg(unix)]
fn create_file_symlink(
    target: impl AsRef<std::path::Path>,
    link: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_file_symlink(
    target: impl AsRef<std::path::Path>,
    link: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}
