use ai_human::cli_output::{
    doctor_result_summary, first_next_stage, render_result_summary, report_summary, ResultSummary,
};

#[test]
fn report_summary_prefers_summary_section() {
    let markdown = "# Plan\n\n## Summary\n\nDesign payment audit.\n\n## Next\n\n- Later\n";

    assert_eq!(report_summary(markdown), "Design payment audit.");
}

#[test]
fn report_summary_falls_back_to_title() {
    let markdown = "# Fix Dry Run\n\n## Result\n\n- Source code files modified: no\n";

    assert_eq!(report_summary(markdown), "Fix Dry Run");
}

#[test]
fn first_next_stage_reads_first_next_stage_item() {
    let markdown = "## Next\n\n- Next stage: run `ai-human review`\n- Other action\n";

    assert_eq!(
        first_next_stage(markdown),
        Some("run `ai-human review`".into())
    );
}

#[test]
fn doctor_summary_reports_not_ready_with_first_run_command() {
    let report = "# AI Human Doctor\n\n## Project\n\n- Project state: not initialized\n\n## Next\n\n- Run `ai-human init --project-root .`.\n";

    assert_eq!(
        doctor_result_summary(report),
        ResultSummary {
            summary: "Setup is not ready.".into(),
            report: "none".into(),
            next_stage: "run `ai-human init --project-root .`".into(),
        }
    );
}

#[test]
fn render_result_summary_uses_stable_shape() {
    let output = render_result_summary(&ResultSummary {
        summary: "Done.".into(),
        report: "none".into(),
        next_stage: "run `ai-human doctor`".into(),
    });

    assert_eq!(
        output,
        "## Result Summary\n- Summary: Done.\n- Report: none\n- Next stage: run `ai-human doctor`\n"
    );
}
