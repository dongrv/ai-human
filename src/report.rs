pub mod markdown {
    use crate::core::report::{
        FixApplyOutput, FixPlanOutput, ImpactOutput, LearningOutput, PlanOutput, ReviewOutput,
    };

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

    pub fn render_learning(output: &LearningOutput) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", output.title));
        md.push_str("## Category\n\n");
        md.push_str(&format!("{}\n\n", output.category));
        md.push_str("## Summary\n\n");
        md.push_str(&format!("{}\n\n", output.summary));
        md.push_str("## Rule\n\n");
        md.push_str(&format!("{}\n\n", output.rule));
        push_list(&mut md, "Evidence", &output.evidence);
        push_list(&mut md, "Applies To", &output.applies_to);

        md
    }

    pub fn render_fix_plan(output: &FixPlanOutput) -> String {
        let mut md = String::new();

        md.push_str("# Fix Dry Run\n\n");
        md.push_str("## Summary\n\n");
        md.push_str(&format!("{}\n\n", output.summary));
        md.push_str("## Result\n\n");
        md.push_str("- Source code files modified: no\n");
        md.push_str(
            "- This is a dry-run report. Review it before using `--apply` in a later phase.\n\n",
        );
        push_list(&mut md, "Target Files", &output.target_files);
        md.push_str("## Change Intent\n\n");
        md.push_str(&format!("{}\n\n", output.change_intent));
        md.push_str("## Risk Level\n\n");
        md.push_str(&format!("{}\n\n", output.risk_level));
        push_list(&mut md, "Risks", &output.risks);
        push_list(
            &mut md,
            "Verification Commands",
            &output.verification_commands,
        );
        push_replacement_files(&mut md, output);
        push_list(&mut md, "Open Questions", &output.open_questions);

        md
    }

    pub fn render_fix_apply(output: &FixApplyOutput) -> String {
        let mut md = String::new();

        md.push_str("# Fix Apply Report\n\n");
        md.push_str("## Summary\n\n");
        md.push_str(&format!("{}\n\n", output.summary));
        md.push_str("## Result\n\n");
        md.push_str("- Source code files modified: yes\n\n");
        push_list(&mut md, "Written Files", &output.written_files);
        push_verification_results(&mut md, output);
        push_list(&mut md, "Residual Risks", &output.residual_risks);

        md
    }

    fn push_replacement_files(md: &mut String, output: &FixPlanOutput) {
        md.push_str("## Proposed Replacement Files\n\n");
        if output.replacement_files.is_empty() {
            md.push_str("- None.\n\n");
            return;
        }

        for file in &output.replacement_files {
            md.push_str(&format!("- `{}`\n", file.path));
        }
        md.push('\n');
    }

    fn push_verification_results(md: &mut String, output: &FixApplyOutput) {
        md.push_str("## Verification Results\n\n");
        if output.verification_results.is_empty() {
            md.push_str("- None.\n\n");
            return;
        }

        for result in &output.verification_results {
            let status = if result.succeeded {
                "succeeded"
            } else {
                "failed"
            };
            md.push_str(&format!(
                "- `{}`: {} (exit: {}, {} ms)\n",
                result.command,
                status,
                result
                    .exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "terminated".into()),
                result.duration_ms
            ));
            if !result.stdout.trim().is_empty() {
                md.push_str(&format!("  - stdout: {}\n", one_line(&result.stdout)));
            }
            if !result.stderr.trim().is_empty() {
                md.push_str(&format!("  - stderr: {}\n", one_line(&result.stderr)));
            }
        }
        md.push('\n');
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

    fn one_line(value: &str) -> String {
        let text = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.len() > 240 {
            format!("{}...", &text[..240])
        } else {
            text
        }
    }
}
