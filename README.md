# AI Human

AI Human is a Rust rig-powered CLI digital human for service-side engineering workflows. It focuses on initializing project knowledge, loading local context, producing structured engineering reports, and preserving reusable team knowledge.

## MVP Capabilities

- Initialize project-owned `.ai-human/` configuration, policy, knowledge, memory, report, and template folders.
- Check project setup and model configuration with a no-network doctor workflow.
- Load local project rules, Markdown knowledge, README-style context, and referenced source paths.
- Answer project questions with the `ask` workflow.
- Produce structured requirement plans with goals, non-goals, affected areas, risks, verification steps, and open questions.
- Produce impact analysis reports with files, call chains, protocol risks, state risks, persistence risks, and test entrypoints.
- Produce code review reports from diffs or file references.
- Capture reusable engineering learnings into Markdown knowledge and JSONL memory.
- Preview and apply one-file local fixes with explicit `--apply`.
- Render plan, impact, review, learning, and fix outputs as Markdown reports under `.ai-human/reports/`.
- Connect related plan, impact, review, learn, and fix reports with `--task-id`.
- Each completed report includes a `Next stage:` action so users can continue the engineering loop without memorizing the workflow.

## Quick Start

Initialize AI Human metadata in the current project:

```powershell
cargo run -- init --project-root .
```

Check setup before calling a model:

```powershell
cargo run -- doctor --project-root .
```

Ask a context-aware engineering question:

```powershell
cargo run -- ask --input "Which modules own payment audit rules?"
```

Save a reusable engineering lesson:

```powershell
cargo run -- learn --input "Payment audit rules are owned by service/pay and must persist before success."
```

Preview a small local fix without changing source files:

```powershell
cargo run -- fix --input "Fix missing nil guard in payment audit parser" --path service/pay/audit.go
```

Apply a reviewed one-file fix:

```powershell
cargo run -- fix --input "Fix missing nil guard in payment audit parser" --path service/pay/audit.go --apply --verify "go test ./service/pay"
```

## Model Environment Variables

Copy `.env.example` to `.env` and fill in local model credentials:

```dotenv
OPENAI_API_KEY=your-key
OPENAI_BASE_URL=http://your-openai-compatible-proxy/v1
AI_HUMAN_MODEL_PROVIDER=openai
AI_HUMAN_MODEL=gpt-4o-mini
AI_HUMAN_OPENAI_WIRE_API=responses
```

CLI commands load `.env` from `--project-root` before reading model configuration. Existing shell environment variables take precedence over `.env` values.

In non-mock mode, AI Human sends the user input and loaded local project context to the configured model provider. Review provider policies and avoid sending sensitive project context to providers that are not approved for that data.
The OpenAI provider uses rig's Responses API client by default. Set `AI_HUMAN_OPENAI_WIRE_API` to `responses` for `/responses` proxies, or `chat-completions` for `/chat/completions` proxies. `OPENAI_BASE_URL` is used as the exact API root before the endpoint path, so use `http://host:port` when the proxy serves `/responses`, or `http://host:port/v1` when it serves `/v1/responses`.

Provider configuration failures include actionable recovery hints. Missing `OPENAI_API_KEY`, unsupported `AI_HUMAN_MODEL_PROVIDER`, invalid `AI_HUMAN_OPENAI_WIRE_API`, and endpoint status errors print a `Next:` step before exiting.

Some OpenAI-compatible Responses proxies omit fields that rig currently requires on response output items, such as `output[].id` or `output[].status`. AI Human normalizes those missing fields before rig parses the response. To inspect the raw provider response during troubleshooting, set:

```powershell
$env:AI_HUMAN_OPENAI_DEBUG_RAW="1"
```

For deterministic local no-network CLI checks without a model call, provide one JSON response:

```powershell
$env:AI_HUMAN_MOCK_RESPONSE='{"title":"Mock Plan","goal":"Verify CLI wiring","non_goals":["No model call"],"affected_areas":["cli"],"risks":["Mock only"],"verification_plan":["cargo test"],"open_questions":[]}'
cargo run -- plan --input "verify cli wiring"
Remove-Item Env:\AI_HUMAN_MOCK_RESPONSE
```

## Doctor Example

Check whether `.ai-human/` exists and model configuration is visible:

```powershell
cargo run -- doctor --project-root .
```

The doctor workflow does not call the model. It prints readiness, project file checks, model environment status, selected provider/model/wire API, and the next command to run when setup is incomplete. When setup is ready, it also suggests a simple `impact -> review` workflow.

## Plan Example

```powershell
cargo run -- plan --input "analyze payment audit requirement"
```

The plan workflow returns Markdown and writes a report ending in `-plan.md`.

Use `--task-id` to connect a plan, impact analysis, review, learning, and fix to the same engineering task:

```powershell
cargo run -- plan --task-id pay-audit-001 --input "analyze payment audit requirement"
cargo run -- impact --task-id pay-audit-001 --input "service/pay audit flow" --path service/pay/audit.go
cargo run -- review --task-id pay-audit-001 --path service/pay/audit.go
```

If `--task-id` is omitted, AI Human generates one automatically and prints it in the report's `## Task` section.

Inspect the local task timeline later:

```powershell
cargo run -- task --id pay-audit-001
```

The task command reads `.ai-human/memory/tasks.jsonl`, prints the reports recorded for that task id, and suggests the next continuation step. When enough context is available, it also prints a `Suggested command:` line that can be copied directly, such as `ai-human review --task-id pay-audit-001 --path service/pay/audit.go`. It does not call the model.

Some commands can inherit task context directly:

```powershell
cargo run -- impact --from-task pay-audit-001
cargo run -- review --from-task pay-audit-001
cargo run -- fix --from-task pay-audit-001
cargo run -- learn --from-task pay-audit-001 --input "Capture the reusable lesson from this task."
```

`impact --from-task` uses the latest plan input. `review --from-task` uses the latest impact path. `fix --from-task` uses the latest review path and summary. `learn --from-task` uses the latest fix report as `--source-report`. Explicit source options such as `--input`, `--path`, `--diff-file`, `--task-id`, or `--source-report` are rejected when they conflict with `--from-task`.

Reports use `Next stage:` entries to connect the normal loop:

```text
plan -> impact -> review -> fix dry-run -> fix --apply -> learn
```

Successful commands print a compact result summary so the next task is visible without reading the full output. Report-producing commands print it after `Report written to ...`:

```text
## Result Summary
- Summary: ...
- Report: .ai-human/reports/...
- Next stage: ...
```

Recoverable command errors include a `Next:` hint. For example, path, review-source, task-inheritance, and provider configuration errors tell you what to change before retrying.

Main reports include typed evidence entries such as `[File]`, `[Project Context]`, `[History Report]`, `[Command]`, and `[Model Inference]`. Reports also include `## Rule Hits` when loaded context contains team rule sources such as `AGENTS.md`, `.agents/README.md`, `.ai-human/knowledge/engineering-rules.md`, or workflow knowledge files.

AI Human also appends local command metrics to `.ai-human/memory/metrics.jsonl`. Each record contains the command name, success or failure status, duration, task id when available, report path when produced, and a short error summary on failure. Metrics do not include source file contents or user prompt text.

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

## Learn Example

Save a rule with friendly defaults:

```powershell
cargo run -- learn --input "Payment audit rules are owned by service/pay and must persist before success."
```

Save a lesson from an existing report:

```powershell
cargo run -- learn --input "Turn this review into a reusable rule" --source-report .ai-human/reports/example-review.md
```

The learn workflow prints the learning report, writes a report ending in `-learn.md`, appends Markdown to `.ai-human/knowledge/engineering-rules.md` by default, and appends structured memory to `.ai-human/memory/learnings.jsonl`. Source code files are not modified by `learn`.

## Fix Example

Preview a bounded fix plan for one target file:

```powershell
cargo run -- fix --input "Fix missing nil guard in payment audit parser" --path service/pay/audit.go
```

Include verification and formatting commands for the model to consider:

```powershell
cargo run -- fix --input "Fix missing nil guard in payment audit parser" --path service/pay/audit.go --verify "go test ./service/pay" --format "gofmt -w service/pay/audit.go"
```

You can also save project defaults in `.ai-human/config.toml`:

```toml
[fix]
default_verify_commands = ["go test ./service/pay"]
default_format_command = "gofmt -w service/pay/audit.go"
```

When `--verify` or `--format` is supplied on the CLI, the CLI value takes precedence over the config default.

The fix workflow is dry-run by default. It reads the target file, prints a `# Fix Dry Run` report, writes a report ending in `-fix-dry-run.md`, and explicitly reports `Source code files modified: no`.

Apply a bounded replacement after reviewing the dry-run plan:

```powershell
cargo run -- fix --input "Fix missing nil guard in payment audit parser" --path service/pay/audit.go --apply --verify "go test ./service/pay"
```

`fix --apply` writes only the file named by `--path`, only when the model returns a matching replacement for that exact file. Verification and formatting commands are executed only when supplied by CLI flags, not because the model suggested them. Apply mode prints a `# Fix Apply Report`, writes a report ending in `-fix-apply.md`, lists written files, and includes command results.

High-risk fix plans are blocked in apply mode. When the model returns `risk_level: "high"`, AI Human keeps the target file unchanged and asks you to keep the output as a dry-run, split the change, or get human review before applying manually.

## Safety Boundary

AI Human does not perform non-model operational side effects such as auto-commit, push, deploy, production config mutation, file deletion, or database changes. Real model calls are external provider actions and may transmit prompt context as described above. Local write workflows are limited to explicit workflow outputs such as `.ai-human/` initialization, generated reports, knowledge files, memory files, and `fix --apply` writes to one explicit project file.
