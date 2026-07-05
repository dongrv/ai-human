use std::path::Path;

use assert_fs::prelude::*;

use ai_human::tools::fs::ProjectFs;

#[tokio::test]
async fn append_text_creates_project_relative_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    let fs = ProjectFs::new(temp.path());

    let display_path = fs
        .append_text(
            Path::new(".ai-human/knowledge/engineering-rules.md"),
            "# Rule\n",
        )
        .await
        .unwrap();

    assert_eq!(display_path, ".ai-human/knowledge/engineering-rules.md");
    temp.child(".ai-human/knowledge/engineering-rules.md")
        .assert("# Rule\n");
}

#[tokio::test]
async fn append_text_rejects_parent_traversal() {
    let temp = assert_fs::TempDir::new().unwrap();
    let fs = ProjectFs::new(temp.path());

    let error = fs
        .append_text(Path::new("../outside.md"), "secret")
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must stay inside project root"));
}

#[tokio::test]
async fn read_text_rejects_absolute_path_outside_project_root() {
    let project = assert_fs::TempDir::new().unwrap();
    let outside = assert_fs::TempDir::new().unwrap();
    outside.child("secret.md").write_str("secret").unwrap();
    let fs = ProjectFs::new(project.path());

    let error = fs
        .read_text(&outside.path().join("secret.md"))
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must stay inside project root"));
}
