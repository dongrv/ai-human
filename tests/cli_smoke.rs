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
