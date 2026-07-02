use assert_fs::prelude::*;

use ai_human::agent::mock::MockAgentClient;
use ai_human::workflow::ask::AskWorkflow;
use ai_human::workflow::impact::ImpactWorkflow;
use ai_human::workflow::plan::PlanWorkflow;
use ai_human::workflow::review::ReviewWorkflow;

#[tokio::test]
async fn ask_workflow_returns_plain_answer() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".ai-human/knowledge/engineering-rules.md")
        .write_str("# Rules\n\nAlways verify.")
        .unwrap();
    let agent = MockAgentClient::new(vec!["Use the project verification rules.".into()]);

    let answer = AskWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run("what should I verify?")
        .await
        .unwrap();

    assert_eq!(answer, "Use the project verification rules.");
}

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
    }"#
    .into()]);

    let report = PlanWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run("add payment audit")
        .await
        .unwrap();

    assert!(report.markdown.contains("# Add payment audit"));
    assert!(report.path.ends_with("-plan.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
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
    }"#
    .into()]);

    let report = ImpactWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run("service/pay/audit.go")
        .await
        .unwrap();

    assert!(report
        .markdown
        .contains("Payment audit touches service and dao"));
    assert!(report.path.ends_with("-impact.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn review_workflow_writes_markdown_report_from_diff_file() {
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
    }"#
    .into()]);

    let report = ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_diff_file(temp.path().join("change.diff"))
        .await
        .unwrap();

    assert!(report.markdown.contains("P1"));
    assert!(report.markdown.contains("Audit state is not persisted"));
    assert!(report.path.ends_with("-review.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
}

#[tokio::test]
async fn review_workflow_writes_markdown_report_from_text() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"No findings",
        "findings":[],
        "test_gaps":[],
        "residual_risks":[]
    }"#
    .into()]);

    let report = ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_text("REVIEW FILE: service/pay/audit.go")
        .await
        .unwrap();

    assert!(report.markdown.contains("No findings"));
    assert!(report.path.ends_with("-review.md"));
    temp.child(&report.path).assert(report.markdown.as_str());
}
