pub mod markdown {
    use crate::core::report::{
        FixApplyOutput, FixPlanOutput, ImpactOutput, LearningOutput, PlanOutput, ReviewOutput,
    };
    use crate::core::task::TaskId;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum EvidenceKind {
        ProjectContext,
        File,
        HistoryReport,
        Command,
        ModelInference,
        UserInput,
    }

    impl EvidenceKind {
        fn label(&self) -> &'static str {
            match self {
                Self::ProjectContext => "Project Context",
                Self::File => "File",
                Self::HistoryReport => "History Report",
                Self::Command => "Command",
                Self::ModelInference => "Model Inference",
                Self::UserInput => "User Input",
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct EvidenceEntry {
        pub kind: EvidenceKind,
        pub detail: String,
    }

    impl EvidenceEntry {
        pub fn project_context(detail: impl Into<String>) -> Self {
            Self {
                kind: EvidenceKind::ProjectContext,
                detail: detail.into(),
            }
        }

        pub fn file(detail: impl Into<String>) -> Self {
            Self {
                kind: EvidenceKind::File,
                detail: detail.into(),
            }
        }

        pub fn history_report(detail: impl Into<String>) -> Self {
            Self {
                kind: EvidenceKind::HistoryReport,
                detail: detail.into(),
            }
        }

        pub fn command(detail: impl Into<String>) -> Self {
            Self {
                kind: EvidenceKind::Command,
                detail: detail.into(),
            }
        }

        pub fn model_inference(detail: impl Into<String>) -> Self {
            Self {
                kind: EvidenceKind::ModelInference,
                detail: detail.into(),
            }
        }

        pub fn user_input(detail: impl Into<String>) -> Self {
            Self {
                kind: EvidenceKind::UserInput,
                detail: detail.into(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RuleHit {
        pub source: String,
        pub detail: String,
    }

    impl RuleHit {
        pub fn new(source: impl Into<String>, detail: impl Into<String>) -> Self {
            Self {
                source: source.into(),
                detail: detail.into(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ReportMeta {
        pub task_id: TaskId,
        pub evidence: Vec<EvidenceEntry>,
        pub rule_hits: Vec<RuleHit>,
        pub next_actions: Vec<String>,
    }

    pub fn rule_hits_from_sources(sources: &[String]) -> Vec<RuleHit> {
        sources
            .iter()
            .filter_map(|source| {
                let normalized = source.to_ascii_lowercase();
                if normalized == "agents.md" || normalized == ".agents/readme.md" {
                    Some(RuleHit::new(
                        source,
                        "Agent and team instructions were included in model context.",
                    ))
                } else if normalized == ".ai-human/knowledge/engineering-rules.md" {
                    Some(RuleHit::new(
                        source,
                        "Project engineering rules were included in model context.",
                    ))
                } else if normalized.starts_with(".ai-human/knowledge/workflows/")
                    && normalized.ends_with(".md")
                {
                    Some(RuleHit::new(
                        source,
                        "Project workflow guidance was included in model context.",
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

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

    pub fn render_plan_report(output: &PlanOutput, meta: &ReportMeta) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", output.title));
        md.push_str(&format!("## Summary\n\n{}\n\n", output.goal));
        push_task(&mut md, meta);
        push_evidence(&mut md, &meta.evidence);
        push_rule_hits(&mut md, &meta.rule_hits);
        push_list(&mut md, "Non Goals", &output.non_goals);
        push_list(&mut md, "Affected Areas", &output.affected_areas);
        push_list(&mut md, "Risks", &output.risks);
        push_list(&mut md, "Verification Plan", &output.verification_plan);
        push_list(&mut md, "Open Questions", &output.open_questions);
        push_list(&mut md, "Next", &meta.next_actions);

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

    pub fn render_impact_report(output: &ImpactOutput, meta: &ReportMeta) -> String {
        let mut md = String::new();

        md.push_str("# Impact Analysis\n\n");
        md.push_str(&format!("## Summary\n\n{}\n\n", output.summary));
        push_task(&mut md, meta);
        push_evidence(&mut md, &meta.evidence);
        push_rule_hits(&mut md, &meta.rule_hits);
        push_list(&mut md, "Files", &output.files);
        push_list(&mut md, "Call Chains", &output.call_chains);
        push_risk_level(&mut md, impact_risk_level(output));
        push_list(&mut md, "Risks", &impact_risks(output));
        push_list(&mut md, "Protocol Risks", &output.protocol_risks);
        push_list(&mut md, "State Risks", &output.state_risks);
        push_list(&mut md, "Persistence Risks", &output.persistence_risks);
        push_list(&mut md, "Test Entrypoints", &output.test_entrypoints);
        push_list(&mut md, "Next", &meta.next_actions);

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

    pub fn render_review_report(output: &ReviewOutput, meta: &ReportMeta) -> String {
        let mut md = String::new();

        md.push_str("# Code Review\n\n");
        md.push_str(&format!("## Summary\n\n{}\n\n", output.summary));
        push_task(&mut md, meta);
        push_evidence(&mut md, &meta.evidence);
        push_rule_hits(&mut md, &meta.rule_hits);
        push_findings(&mut md, output);
        push_risk_level(&mut md, review_risk_level(output));
        push_list(&mut md, "Test Gaps", &output.test_gaps);
        push_list(&mut md, "Risks", &output.residual_risks);
        push_list(&mut md, "Residual Risks", &output.residual_risks);
        push_list(&mut md, "Next", &meta.next_actions);

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

    pub fn render_learning_report(output: &LearningOutput, meta: &ReportMeta) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", output.title));
        md.push_str(&format!("## Summary\n\n{}\n\n", output.summary));
        push_task(&mut md, meta);
        md.push_str("## Category\n\n");
        md.push_str(&format!("{}\n\n", output.category));
        md.push_str("## Rule\n\n");
        md.push_str(&format!("{}\n\n", output.rule));
        push_evidence(&mut md, &meta.evidence);
        push_rule_hits(&mut md, &meta.rule_hits);
        push_list(&mut md, "Applies To", &output.applies_to);
        push_list(
            &mut md,
            "Risks",
            &["Knowledge must be reviewed before it becomes a team rule.".into()],
        );
        push_list(&mut md, "Next", &meta.next_actions);

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

    pub fn render_fix_plan_report(output: &FixPlanOutput, meta: &ReportMeta) -> String {
        let mut md = String::new();

        md.push_str("# Fix Dry Run\n\n");
        md.push_str("## Summary\n\n");
        md.push_str(&format!("{}\n\n", output.summary));
        push_task(&mut md, meta);
        push_evidence(&mut md, &meta.evidence);
        push_rule_hits(&mut md, &meta.rule_hits);
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
        push_list(&mut md, "Next", &meta.next_actions);

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

    pub fn render_fix_apply_report(output: &FixApplyOutput, meta: &ReportMeta) -> String {
        let mut md = String::new();

        md.push_str("# Fix Apply Report\n\n");
        md.push_str("## Summary\n\n");
        md.push_str(&format!("{}\n\n", output.summary));
        push_task(&mut md, meta);
        push_evidence(&mut md, &meta.evidence);
        push_rule_hits(&mut md, &meta.rule_hits);
        md.push_str("## Result\n\n");
        md.push_str("- Source code files modified: yes\n\n");
        push_list(&mut md, "Written Files", &output.written_files);
        push_verification_results(&mut md, output);
        push_list(&mut md, "Risks", &output.residual_risks);
        push_list(&mut md, "Residual Risks", &output.residual_risks);
        push_list(&mut md, "Next", &meta.next_actions);

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

    fn push_findings(md: &mut String, output: &ReviewOutput) {
        md.push_str("## Findings\n\n");
        if output.findings.is_empty() {
            md.push_str("- No findings.\n\n");
            return;
        }

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

    fn impact_risks(output: &ImpactOutput) -> Vec<String> {
        output
            .protocol_risks
            .iter()
            .chain(output.state_risks.iter())
            .chain(output.persistence_risks.iter())
            .cloned()
            .collect()
    }

    fn impact_risk_level(output: &ImpactOutput) -> &'static str {
        if !output.protocol_risks.is_empty() {
            "High"
        } else if !output.state_risks.is_empty() || !output.persistence_risks.is_empty() {
            "Medium"
        } else {
            "Low"
        }
    }

    fn review_risk_level(output: &ReviewOutput) -> &'static str {
        if output
            .findings
            .iter()
            .any(|finding| matches!(finding.severity.to_ascii_uppercase().as_str(), "P0" | "P1"))
        {
            "High"
        } else if output
            .findings
            .iter()
            .any(|finding| matches!(finding.severity.to_ascii_uppercase().as_str(), "P2"))
            || !output.test_gaps.is_empty()
            || !output.residual_risks.is_empty()
        {
            "Medium"
        } else {
            "Low"
        }
    }

    fn push_risk_level(md: &mut String, level: &str) {
        md.push_str("## Risk Level\n\n");
        md.push_str(&format!("{level}\n\n"));
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

    fn push_evidence(md: &mut String, evidence: &[EvidenceEntry]) {
        md.push_str("## Evidence\n\n");
        if evidence.is_empty() {
            md.push_str("- None.\n\n");
            return;
        }

        for entry in evidence {
            md.push_str(&format!("- [{}] {}\n", entry.kind.label(), entry.detail));
        }
        md.push('\n');
    }

    fn push_rule_hits(md: &mut String, rule_hits: &[RuleHit]) {
        md.push_str("## Rule Hits\n\n");
        if rule_hits.is_empty() {
            md.push_str("- None.\n\n");
            return;
        }

        for hit in rule_hits {
            md.push_str(&format!("- [{}] {}\n", hit.source, hit.detail));
        }
        md.push('\n');
    }

    fn push_task(md: &mut String, meta: &ReportMeta) {
        md.push_str("## Task\n\n");
        md.push_str(&format!("- Task ID: {}\n\n", meta.task_id.as_str()));
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
