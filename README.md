# AI Human

AI Human is a Rust rig-powered CLI digital human for service-side engineering workflows. The MVP focuses on initializing project knowledge, loading local context, and producing structured engineering reports.

## MVP Capabilities

- Initialize project-owned `.ai-human/` configuration, policy, knowledge, memory, report, and template folders.
- Load local project rules, Markdown knowledge, README-style context, and referenced source paths.
- Answer project questions with the `ask` workflow.
- Produce structured requirement plans with goals, non-goals, affected areas, risks, verification steps, and open questions.
- Produce impact analysis reports with files, call chains, protocol risks, state risks, persistence risks, and test entrypoints.
- Produce code review reports from diffs or file references.
- Render plan, impact, and review outputs as Markdown reports under `.ai-human/reports/`.

## Quick Start

Initialize AI Human metadata in the current project:

```powershell
cargo run -- init --project-root .
```

Ask a context-aware engineering question:

```powershell
cargo run -- ask --input "Which modules own payment audit rules?"
```

## Model Environment Variables

Set model credentials for real model calls:

```powershell
$env:OPENAI_API_KEY="your-key"
$env:AI_HUMAN_MODEL_PROVIDER="openai"
$env:AI_HUMAN_MODEL="gpt-4o-mini"
```

In non-mock mode, AI Human sends the user input and loaded local project context to the configured model provider. Review provider policies and avoid sending sensitive project context to providers that are not approved for that data.

For deterministic local no-network CLI checks without a model call, provide one JSON response:

```powershell
$env:AI_HUMAN_MOCK_RESPONSE='{"title":"Mock Plan","goal":"Verify CLI wiring","non_goals":["No model call"],"affected_areas":["cli"],"risks":["Mock only"],"verification_plan":["cargo test"],"open_questions":[]}'
cargo run -- plan --input "verify cli wiring"
Remove-Item Env:\AI_HUMAN_MOCK_RESPONSE
```

## Plan Example

```powershell
cargo run -- plan --input "analyze payment audit requirement"
```

The plan workflow returns Markdown and writes a report ending in `-plan.md`.

## Impact Example

```powershell
cargo run -- impact --input "service/pay audit flow" --path service/pay/audit.go
```

The impact workflow returns Markdown and writes a report ending in `-impact.md`.

## Review Example

Review a saved diff:

```powershell
cargo run -- review --diff-file change.diff
```

Review a file reference:

```powershell
cargo run -- review --path service/pay/audit.go
```

The review workflow returns Markdown and writes a report ending in `-review.md`.
Both `--diff-file` and `--path` must point to regular files inside `--project-root`.
Review findings are also appended to `.ai-human/memory/reviews.jsonl`.

## Safety Boundary

The MVP does not perform non-model operational side effects such as auto-commit, push, deploy, production config mutation, file deletion, or database changes. Real model calls are external provider actions and may transmit prompt context as described above. Local write workflows are limited to explicit workflow outputs such as `.ai-human/` initialization and generated reports, with policy types in place for future controlled execution.
