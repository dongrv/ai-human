# AI Human

AI Human is a Rust rig-powered CLI digital human for service-side engineering workflows. It focuses on initializing project knowledge, loading local context, producing structured engineering reports, and preserving reusable team knowledge.

## MVP Capabilities

- Initialize project-owned `.ai-human/` configuration, policy, knowledge, memory, report, and template folders.
- Load local project rules, Markdown knowledge, README-style context, and referenced source paths.
- Answer project questions with the `ask` workflow.
- Produce structured requirement plans with goals, non-goals, affected areas, risks, verification steps, and open questions.
- Produce impact analysis reports with files, call chains, protocol risks, state risks, persistence risks, and test entrypoints.
- Produce code review reports from diffs or file references.
- Capture reusable engineering learnings into Markdown knowledge and JSONL memory.
- Preview and apply one-file local fixes with explicit `--apply`.
- Render plan, impact, review, learning, and fix outputs as Markdown reports under `.ai-human/reports/`.

## Quick Start

Initialize AI Human metadata in the current project:

```powershell
cargo run -- init --project-root .
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

The fix workflow is dry-run by default. It reads the target file, prints a `# Fix Dry Run` report, writes a report ending in `-fix-dry-run.md`, and explicitly reports `Source code files modified: no`.

Apply a bounded replacement after reviewing the dry-run plan:

```powershell
cargo run -- fix --input "Fix missing nil guard in payment audit parser" --path service/pay/audit.go --apply --verify "go test ./service/pay"
```

`fix --apply` writes only the file named by `--path`, only when the model returns a matching replacement for that exact file. Verification and formatting commands are executed only when supplied by CLI flags, not because the model suggested them. Apply mode prints a `# Fix Apply Report`, writes a report ending in `-fix-apply.md`, lists written files, and includes command results.

## Safety Boundary

AI Human does not perform non-model operational side effects such as auto-commit, push, deploy, production config mutation, file deletion, or database changes. Real model calls are external provider actions and may transmit prompt context as described above. Local write workflows are limited to explicit workflow outputs such as `.ai-human/` initialization, generated reports, knowledge files, memory files, and `fix --apply` writes to one explicit project file.
