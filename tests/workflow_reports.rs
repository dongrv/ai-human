use assert_fs::prelude::*;

use ai_human::agent::mock::MockAgentClient;
use ai_human::agent::{AgentClient, AgentRequest};
use ai_human::workflow::ask::AskWorkflow;
use ai_human::workflow::impact::ImpactWorkflow;
use ai_human::workflow::plan::PlanWorkflow;
use ai_human::workflow::review::ReviewWorkflow;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

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
async fn repeated_plan_runs_create_distinct_reports_without_overwriting() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![
        r#"{
        "title":"First plan",
        "goal":"Write the first report",
        "non_goals":[],
        "affected_areas":[],
        "risks":[],
        "verification_plan":["cargo test"],
        "open_questions":[]
    }"#
        .into(),
        r#"{
        "title":"Second plan",
        "goal":"Write the second report",
        "non_goals":[],
        "affected_areas":[],
        "risks":[],
        "verification_plan":["cargo test"],
        "open_questions":[]
    }"#
        .into(),
    ]);
    let workflow = PlanWorkflow::new(temp.path().to_path_buf(), Box::new(agent));

    let first = workflow.run("write first report").await.unwrap();
    let second = workflow.run("write second report").await.unwrap();

    assert_ne!(first.path, second.path);
    assert!(first.path.ends_with("-plan.md"));
    assert!(second.path.ends_with("-plan.md"));
    temp.child(&first.path).assert(first.markdown.as_str());
    temp.child(&second.path).assert(second.markdown.as_str());
    assert!(first.markdown.contains("# First plan"));
    assert!(second.markdown.contains("# Second plan"));
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
async fn review_workflow_appends_findings_to_reviews_memory() {
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
        "test_gaps":[],
        "residual_risks":[]
    }"#
    .into()]);

    ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_diff_file(temp.path().join("change.diff"))
        .await
        .unwrap();

    let reviews = tokio::fs::read_to_string(temp.path().join(".ai-human/memory/reviews.jsonl"))
        .await
        .unwrap();

    assert!(reviews.contains(r#""severity":"P1""#));
    assert!(reviews.contains(r#""issue":"Audit state is not persisted after mutation""#));
}

#[tokio::test]
async fn review_workflow_rejects_diff_file_outside_project_root() {
    let project = assert_fs::TempDir::new().unwrap();
    let outside = assert_fs::TempDir::new().unwrap();
    outside
        .child("change.diff")
        .write_str("secret diff")
        .unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"unused",
        "findings":[],
        "test_gaps":[],
        "residual_risks":[]
    }"#
    .into()]);

    let error = ReviewWorkflow::new(project.path().to_path_buf(), Box::new(agent))
        .run_diff_file(outside.path().join("change.diff"))
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must stay inside project root"));
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

#[tokio::test]
async fn review_workflow_reads_file_content_from_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/lib.rs")
        .write_str("pub fn reviewed_symbol() {}")
        .unwrap();
    let captured = Arc::new(Mutex::new(None));
    let agent = RecordingAgentClient {
        captured: Arc::clone(&captured),
        response: r#"{
            "summary":"No findings",
            "findings":[],
            "test_gaps":[],
            "residual_risks":[]
        }"#
        .into(),
    };

    ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_path("src/lib.rs".into())
        .await
        .unwrap();

    let request = captured.lock().unwrap().clone().unwrap();

    assert!(request.user_prompt.contains("FILE: src/lib.rs"));
    assert!(request.user_prompt.contains("pub fn reviewed_symbol() {}"));
}

#[tokio::test]
async fn review_workflow_rejects_missing_path_before_model_call() {
    let temp = assert_fs::TempDir::new().unwrap();
    let agent = MockAgentClient::new(vec![r#"{
        "summary":"unused",
        "findings":[],
        "test_gaps":[],
        "residual_risks":[]
    }"#
    .into()]);

    let error = ReviewWorkflow::new(temp.path().to_path_buf(), Box::new(agent))
        .run_path("missing.rs".into())
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("path must reference an existing project file"));
}

#[derive(Debug)]
struct RecordingAgentClient {
    captured: Arc<Mutex<Option<AgentRequest>>>,
    response: String,
}

#[async_trait]
impl AgentClient for RecordingAgentClient {
    async fn complete(&self, request: AgentRequest) -> anyhow::Result<String> {
        *self.captured.lock().unwrap() = Some(request);
        Ok(self.response.clone())
    }
}
