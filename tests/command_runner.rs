use ai_human::tools::command::CommandRunner;

#[tokio::test]
async fn command_runner_captures_success() {
    let temp = assert_fs::TempDir::new().unwrap();
    let runner = CommandRunner::new(temp.path());

    let result = runner.run("cargo --version").await.unwrap();

    assert_eq!(result.exit_code, Some(0));
    assert!(result.succeeded);
    assert!(result.stdout.contains("cargo"));
}

#[tokio::test]
async fn command_runner_captures_failure() {
    let temp = assert_fs::TempDir::new().unwrap();
    let runner = CommandRunner::new(temp.path());

    let result = runner
        .run("cargo definitely-not-a-real-command")
        .await
        .unwrap();

    assert!(!result.succeeded);
    assert_ne!(result.exit_code, Some(0));
}
