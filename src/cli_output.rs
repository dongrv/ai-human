use crate::workflow::WorkflowReport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSummary {
    pub summary: String,
    pub report: String,
    pub next_stage: String,
}

pub fn render_result_summary(summary: &ResultSummary) -> String {
    format!(
        "## Result Summary\n- Summary: {}\n- Report: {}\n- Next stage: {}\n",
        summary.summary, summary.report, summary.next_stage
    )
}

pub fn print_result_summary(summary: ResultSummary) {
    print!("{}", render_result_summary(&summary));
}

pub fn print_workflow_report(report: &WorkflowReport) {
    println!("{}", report.markdown);
    println!("Report written to {}", report.path);
    print_result_summary(ResultSummary {
        summary: report_summary(&report.markdown),
        report: report.path.clone(),
        next_stage: first_next_stage(&report.markdown)
            .unwrap_or_else(|| "Review the report.".into()),
    });
}

pub fn doctor_result_summary(report: &str) -> ResultSummary {
    if report.contains("Project state: initialized") && report.contains("model credentials present")
    {
        ResultSummary {
            summary: "Setup is ready.".into(),
            report: "none".into(),
            next_stage: "run `ai-human ask`, `ai-human plan`, or `ai-human impact`".into(),
        }
    } else {
        ResultSummary {
            summary: "Setup is not ready.".into(),
            report: "none".into(),
            next_stage: first_doctor_next_command(report)
                .unwrap_or_else(|| "fix the blocking setup issue listed above".into()),
        }
    }
}

fn first_doctor_next_command(report: &str) -> Option<String> {
    report.lines().find_map(|line| {
        line.trim()
            .strip_prefix("- Run `")
            .and_then(|value| value.strip_suffix("`."))
            .map(|command| format!("run `{command}`"))
    })
}

pub fn first_next_stage(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        line.trim()
            .strip_prefix("- Next stage: ")
            .map(|action| action.to_string())
    })
}

pub fn report_summary(markdown: &str) -> String {
    markdown_section_first_text(markdown, "## Summary")
        .or_else(|| markdown_title(markdown))
        .unwrap_or_else(|| "Report generated.".into())
}

fn markdown_section_first_text(markdown: &str, heading: &str) -> Option<String> {
    let mut in_section = false;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed == heading {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            return None;
        }
        if in_section && !trimmed.is_empty() {
            return Some(trimmed.trim_start_matches("- ").into());
        }
    }

    None
}

fn markdown_title(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::to_string))
}
