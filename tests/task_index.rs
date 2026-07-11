use assert_fs::prelude::*;
use chrono::Utc;

use ai_human::core::task::{EvidenceRecord, RuleHitRecord, TaskRecord, TaskStatus, TaskType};
use ai_human::task_index::{
    continuation_from_timeline, render_evidence_query, render_task_timeline, CompletedTaskReport,
    Continuation, EvidenceFilter, TaskIndex,
};

#[tokio::test]
async fn appends_and_loads_task_timeline_records() {
    let temp = assert_fs::TempDir::new().unwrap();
    let index = TaskIndex::new(temp.path());

    index
        .append_completed_report(CompletedTaskReport {
            task_id: "pay-audit-001".into(),
            task_type: TaskType::Plan,
            repo: "sample".into(),
            input: "plan payment audit".into(),
            summary: "Design payment audit.".into(),
            report_path: ".ai-human/reports/pay-audit-001-plan.md".into(),
            evidence: vec![EvidenceRecord {
                kind: "Project Context".into(),
                detail: "Repository knowledge loaded.".into(),
            }],
            rule_hits: vec![RuleHitRecord {
                source: ".ai-human/knowledge/engineering-rules.md".into(),
                detail: "Project engineering rules were included in model context.".into(),
            }],
            started_at: Utc::now(),
            completed_at: Utc::now(),
        })
        .await
        .unwrap();

    let timeline = index.load("pay-audit-001").await.unwrap();

    assert_eq!(timeline.task_id, "pay-audit-001");
    assert_eq!(timeline.records.len(), 1);
    assert_eq!(timeline.records[0].summary, "Design payment audit.");
    assert_eq!(
        timeline.records[0].report_path.as_deref(),
        Some(".ai-human/reports/pay-audit-001-plan.md")
    );
    assert_eq!(timeline.records[0].evidence[0].kind, "Project Context");
    assert_eq!(
        timeline.records[0].rule_hits[0].source,
        ".ai-human/knowledge/engineering-rules.md"
    );
}

#[tokio::test]
async fn loading_missing_task_returns_empty_timeline() {
    let temp = assert_fs::TempDir::new().unwrap();
    let index = TaskIndex::new(temp.path());

    let timeline = index.load("missing-task").await.unwrap();

    assert_eq!(timeline.task_id, "missing-task");
    assert!(timeline.records.is_empty());
}

#[tokio::test]
async fn loads_legacy_task_records_without_evidence_fields() {
    let temp = assert_fs::TempDir::new().unwrap();
    let memory = temp.child(".ai-human/memory/tasks.jsonl");
    memory
        .write_str(
            r#"{"task_id":"legacy-001","task_type":"Plan","repo":"sample","input":"legacy plan","status":"Completed","started_at":"2026-07-11T00:00:00Z","completed_at":"2026-07-11T00:00:01Z","summary":"Legacy report","report_path":".ai-human/reports/legacy-plan.md"}"#,
        )
        .unwrap();

    let timeline = TaskIndex::new(temp.path())
        .load("legacy-001")
        .await
        .unwrap();

    assert_eq!(timeline.records.len(), 1);
    assert!(timeline.records[0].evidence.is_empty());
    assert!(timeline.records[0].rule_hits.is_empty());
}

#[test]
fn renders_task_timeline_as_actionable_markdown() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::ImpactAnalysis,
            "service/pay audit flow\nPATH: service/pay/audit.go",
            "Audit flow touches payment state.",
            ".ai-human/reports/pay-audit-001-impact.md",
        )],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.starts_with("# Task pay-audit-001\n\n"));
    assert!(markdown.contains("## Timeline\n\n"));
    assert!(markdown.contains("- impact completed: Audit flow touches payment state."));
    assert!(markdown.contains("  Report: .ai-human/reports/pay-audit-001-impact.md"));
    assert!(markdown.contains("## Next\n\n- Open the latest report listed above"));
    assert!(markdown.contains(
        "- Suggested command: `ai-human review --task-id pay-audit-001 --path service/pay/audit.go`"
    ));
}

#[test]
fn renders_task_timeline_with_cross_report_evidence_and_rule_hits() {
    let mut plan = task_record(
        TaskType::Plan,
        "plan payment audit",
        "Design payment audit.",
        ".ai-human/reports/pay-audit-001-plan.md",
    );
    plan.evidence.push(EvidenceRecord {
        kind: "Project Context".into(),
        detail: "Repository knowledge loaded.".into(),
    });
    plan.rule_hits.push(RuleHitRecord {
        source: ".ai-human/knowledge/engineering-rules.md".into(),
        detail: "Project engineering rules were included in model context.".into(),
    });

    let mut review = task_record(
        TaskType::CodeReview,
        "PATH: service/pay/audit.go",
        "Review found a persistence risk.",
        ".ai-human/reports/pay-audit-001-review.md",
    );
    review.evidence.push(EvidenceRecord {
        kind: "File".into(),
        detail: "Reviewed file `service/pay/audit.go`.".into(),
    });

    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![plan, review],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.contains("## Evidence Trail"));
    assert!(markdown.contains("- plan: [Project Context] Repository knowledge loaded."));
    assert!(markdown.contains("- review: [File] Reviewed file `service/pay/audit.go`."));
    assert!(markdown.contains("## Rule Hits"));
    assert!(markdown.contains(
        "- plan: [.ai-human/knowledge/engineering-rules.md] Project engineering rules were included in model context."
    ));
}

#[test]
fn renders_evidence_query_with_kind_and_source_filters() {
    let mut plan = task_record(
        TaskType::Plan,
        "plan payment audit",
        "Design payment audit.",
        ".ai-human/reports/pay-audit-001-plan.md",
    );
    plan.evidence.push(EvidenceRecord {
        kind: "Project Context".into(),
        detail: "Repository knowledge loaded.".into(),
    });
    plan.rule_hits.push(RuleHitRecord {
        source: ".ai-human/knowledge/engineering-rules.md".into(),
        detail: "Project engineering rules were included in model context.".into(),
    });

    let mut review = task_record(
        TaskType::CodeReview,
        "PATH: service/pay/audit.go",
        "Review found a persistence risk.",
        ".ai-human/reports/pay-audit-001-review.md",
    );
    review.evidence.push(EvidenceRecord {
        kind: "File".into(),
        detail: "Reviewed file `service/pay/audit.go`.".into(),
    });

    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![plan, review],
    };

    let markdown = render_evidence_query(
        &timeline,
        &EvidenceFilter {
            kind: Some("file".into()),
            source: Some("engineering-rules".into()),
        },
    );

    assert!(markdown.starts_with("# Evidence pay-audit-001\n\n"));
    assert!(markdown.contains("## Filters"));
    assert!(markdown.contains("- Kind: file"));
    assert!(markdown.contains("- Source: engineering-rules"));
    assert!(markdown.contains("- Evidence matches: 1"));
    assert!(markdown.contains("- Rule hit matches: 1"));
    assert!(markdown.contains("- review: [File] Reviewed file `service/pay/audit.go`."));
    assert!(markdown.contains(
        "- plan: [.ai-human/knowledge/engineering-rules.md] Project engineering rules were included in model context."
    ));
    assert!(!markdown.contains("[Project Context] Repository knowledge loaded."));
}

#[test]
fn renders_plan_timeline_with_impact_continuation_command() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::Plan,
            "analyze payment audit requirement",
            "Design payment audit.",
            ".ai-human/reports/pay-audit-001-plan.md",
        )],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.contains(
        "- Suggested command: `ai-human impact --task-id pay-audit-001 --input \"analyze payment audit requirement\"`"
    ));
}

#[test]
fn renders_review_timeline_with_fix_continuation_command() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::CodeReview,
            "PATH: service/pay/audit.go",
            "Review found a nil guard issue.",
            ".ai-human/reports/pay-audit-001-review.md",
        )],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.contains(
        "- Suggested command: `ai-human fix --task-id pay-audit-001 --input \"Review found a nil guard issue.\" --path service/pay/audit.go`"
    ));
}

#[test]
fn renders_fix_timeline_with_learn_continuation_command() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::SmallFix,
            "Fix missing nil guard\nPATH: service/pay/audit.go",
            "Applied nil guard.",
            ".ai-human/reports/pay-audit-001-fix-apply.md",
        )],
    };

    let markdown = render_task_timeline(&timeline);

    assert!(markdown.contains(
        "- Suggested command: `ai-human learn --task-id pay-audit-001 --input \"Capture the reusable lesson from this task.\" --source-report .ai-human/reports/pay-audit-001-fix-apply.md`"
    ));
}

#[test]
fn continuation_from_impact_task_returns_review_path() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::ImpactAnalysis,
            "check library impact\nPATH: src/lib.rs",
            "Mock impact",
            ".ai-human/reports/pay-audit-001-impact.md",
        )],
    };

    assert_eq!(
        continuation_from_timeline(&timeline),
        Some(Continuation::ReviewPath {
            task_id: "pay-audit-001".into(),
            path: "src/lib.rs".into(),
        })
    );
}

#[test]
fn continuation_from_plan_task_returns_impact_input() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::Plan,
            "analyze payment audit requirement",
            "Design payment audit.",
            ".ai-human/reports/pay-audit-001-plan.md",
        )],
    };

    assert_eq!(
        continuation_from_timeline(&timeline),
        Some(Continuation::ImpactInput {
            task_id: "pay-audit-001".into(),
            input: "analyze payment audit requirement".into(),
        })
    );
}

#[test]
fn continuation_from_review_task_returns_fix_path_and_input() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::CodeReview,
            "PATH: src/lib.rs",
            "Review found a nil guard issue.",
            ".ai-human/reports/pay-audit-001-review.md",
        )],
    };

    assert_eq!(
        continuation_from_timeline(&timeline),
        Some(Continuation::FixPath {
            task_id: "pay-audit-001".into(),
            input: "Review found a nil guard issue.".into(),
            path: "src/lib.rs".into(),
        })
    );
}

#[test]
fn continuation_from_fix_task_returns_learn_source_report() {
    let timeline = ai_human::task_index::TaskTimeline {
        task_id: "pay-audit-001".into(),
        records: vec![task_record(
            TaskType::SmallFix,
            "Fix missing nil guard\nPATH: src/lib.rs",
            "Applied nil guard.",
            ".ai-human/reports/pay-audit-001-fix-apply.md",
        )],
    };

    assert_eq!(
        continuation_from_timeline(&timeline),
        Some(Continuation::LearnSourceReport {
            task_id: "pay-audit-001".into(),
            source_report: ".ai-human/reports/pay-audit-001-fix-apply.md".into(),
        })
    );
}

fn task_record(task_type: TaskType, input: &str, summary: &str, report_path: &str) -> TaskRecord {
    TaskRecord {
        task_id: "pay-audit-001".into(),
        task_type,
        repo: "sample".into(),
        input: input.into(),
        status: TaskStatus::Completed,
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        summary: summary.into(),
        report_path: Some(report_path.into()),
        evidence: Vec::new(),
        rule_hits: Vec::new(),
    }
}
