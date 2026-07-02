pub mod markdown {
    use crate::core::report::{ImpactOutput, PlanOutput, ReviewOutput};

    pub fn render_plan(output: &PlanOutput) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", output.title));
        md.push_str(&format!("## Goal\n\n{}\n\n", output.goal));
        push_list(&mut md, "Non Goals", &output.non_goals);
        push_list(&mut md, "Affected Areas", &output.affected_areas);
        push_list(&mut md, "Risks", &output.risks);
        push_list(&mut md, "Verification Plan", &output.verification_plan);
        push_list(&mut md, "Open Questions", &output.open_questions);

        md
    }

    pub fn render_impact(output: &ImpactOutput) -> String {
        let mut md = String::new();

        md.push_str("# Impact Analysis\n\n");
        md.push_str(&format!("## Summary\n\n{}\n\n", output.summary));
        push_list(&mut md, "Files", &output.files);
        push_list(&mut md, "Call Chains", &output.call_chains);
        push_list(&mut md, "Protocol Risks", &output.protocol_risks);
        push_list(&mut md, "State Risks", &output.state_risks);
        push_list(&mut md, "Persistence Risks", &output.persistence_risks);
        push_list(&mut md, "Test Entrypoints", &output.test_entrypoints);

        md
    }

    pub fn render_review(output: &ReviewOutput) -> String {
        let mut md = String::new();

        md.push_str("# Code Review\n\n");
        md.push_str(&format!("## Summary\n\n{}\n\n", output.summary));
        md.push_str("## Findings\n\n");
        if output.findings.is_empty() {
            md.push_str("- No findings.\n\n");
        } else {
            for finding in &output.findings {
                let location = match (&finding.file, finding.line) {
                    (Some(file), Some(line)) => format!("{file}:{line}"),
                    (Some(file), None) => file.clone(),
                    (None, _) => "unspecified location".into(),
                };

                md.push_str(&format!(
                    "- **{}** `{}`: {} Suggestion: {}\n",
                    finding.severity, location, finding.issue, finding.suggestion
                ));
            }
            md.push('\n');
        }
        push_list(&mut md, "Test Gaps", &output.test_gaps);
        push_list(&mut md, "Residual Risks", &output.residual_risks);

        md
    }

    fn push_list(md: &mut String, title: &str, values: &[String]) {
        md.push_str(&format!("## {title}\n\n"));
        if values.is_empty() {
            md.push_str("- None.\n\n");
            return;
        }

        for value in values {
            md.push_str(&format!("- {value}\n"));
        }
        md.push('\n');
    }
}
