# AI Digital Human Phase 2 Controlled Fix And Learn Design

Date: 2026-07-05

## 1. Background

The current MVP can initialize project knowledge, load local context, call OpenAI through rig, produce plan/impact/review reports, and append review findings into JSONL memory. Binary testing has confirmed the model connection path is usable enough to move forward.

Phase 2 should not expand into Web, IM, RAG, or autonomous development. The next value step is to close the engineering loop inside the CLI:

```text
analyze -> plan local change -> human-controlled apply -> verify -> report -> learn
```

The product should become useful as a technical lead assistant that can safely make small local edits and preserve reusable engineering knowledge.

## 2. Product Positioning

Phase 2 adds two capabilities:

- `ai-human fix`: controlled, small-scope local code changes with an explicit dry-run-first execution model.
- `ai-human learn`: human-approved knowledge capture into Markdown knowledge files and structured JSONL memory.

This keeps the product distinct from a general coding agent. AI Human is not trying to replace Codex. It is packaging team engineering workflow, safety policy, reports, and knowledge reuse into a repeatable server-side development assistant.

## 3. Goals

- Add a controlled execution loop for small, low-risk local fixes.
- Keep all local writes inside `--project-root`.
- Require an explicit apply flag before code files are modified.
- Record verification commands and outcomes in delivery reports.
- Add a first-class learning workflow for rules, FAQ, cases, and lessons.
- Keep architecture SOLID: workflow orchestration does not directly become model, filesystem, command, policy, and report code all at once.

### 3.1 User Experience Requirements

Human-friendly operation is a core requirement, not a polish task. The CLI should reduce decision cost for service-side engineers and technical leads.

- Friendly defaults: `learn` should work with only `--input`; `fix` should default to dry-run and clearly say no files were modified.
- Actionable errors: every validation error should explain what failed and show the safe next action when possible.
- Progressive output: show the main result first, then written paths, then follow-up actions; do not bury the outcome in verbose logs.
- Explicit writes: every command that writes project state should print the exact files it wrote.
- Copy-paste friendly examples: command help and README examples should be runnable with minimal editing.
- Low jargon: user-facing text should say "report", "knowledge", "dry run", and "next command" instead of internal architecture terms.
- Safe learning: `learn` should append reviewable Markdown before teams rely on the new rule.
- Recovery awareness: reports should make it obvious whether source files were untouched, modified, or only knowledge was appended.

## 4. Non-Goals

- No automatic Git commit, push, merge, tag, or branch mutation.
- No production config mutation.
- No deployment or external side effects.
- No database migrations or live data changes.
- No free-form autonomous agent loop.
- No broad refactor command.
- No Web UI, IM integration, vector database, or multi-user permission system.

## 5. Recommended Delivery Order

### Phase 2A: Learn Closed Loop

Implement `ai-human learn` first. It is lower risk than file mutation and immediately improves knowledge reuse.

User value:

- Convert a task conclusion, review finding, or manually supplied text into reusable project knowledge.
- Append structured learning records to `.ai-human/memory/learnings.jsonl`.
- Create or append Markdown entries under `.ai-human/knowledge/`.

### Phase 2B: Fix Dry Run

Implement `ai-human fix` in dry-run mode. It produces a fix plan and delivery preview but does not change code.

User value:

- Understand proposed target files, change intent, risks, and verification before allowing writes.
- Reuse existing context loading and planning/reporting patterns.

### Phase 2C: Fix Apply

Add `ai-human fix --apply` for small local edits.

User value:

- Apply bounded local changes.
- Run configured formatter/test commands.
- Generate a delivery report with evidence.

This order intentionally builds trust before allowing mutation.

## 6. CLI Design

### 6.1 `ai-human learn`

Initial command shape:

```powershell
ai-human learn --input "Payment audit rules are owned by service/pay and must persist before success."
ai-human learn --input "..." --category rule
ai-human learn --input "..." --target engineering-rules
ai-human learn --input "..." --source-report .ai-human/reports/example-review.md
```

Arguments:

- `--project-root <path>`: default `.`
- `--input <text>`: human-provided lesson or source material
- `--category <rule|faq|case|decision|pitfall>`: default `rule`
- `--target <engineering-rules|faq|case>`: optional routing hint
- `--source-report <path>`: optional report file inside project root

Behavior:

- Load `.env` and project context.
- If `--source-report` is provided, read it through a project-root-safe path validator.
- Ask the model for structured learning output.
- Render a Markdown learning entry.
- Append to the selected knowledge file, or create a case file when `target=case`.
- Append a `LearningRecord` to `learnings.jsonl`.
- Print the Markdown and written paths.

### 6.2 `ai-human fix`

Initial command shape:

```powershell
ai-human fix --input "Fix missing nil guard in payment audit parser" --path service/pay/audit.go
ai-human fix --input "..." --path service/pay/audit.go --dry-run
ai-human fix --input "..." --path service/pay/audit.go --apply
ai-human fix --input "..." --path service/pay/audit.go --apply --verify "go test ./service/pay"
```

Arguments:

- `--project-root <path>`: default `.`
- `--input <text>`: required problem statement
- `--path <path>`: one or more target files in later versions; Phase 2 starts with one file
- `--dry-run`: explicit no-write mode; default behavior if neither flag is set
- `--apply`: allow local file write after plan generation
- `--verify <command>`: optional verification command
- `--format <command>`: optional formatting command

Behavior:

- Default is dry run. It never edits files unless `--apply` is present.
- Require at least one `--path` for Phase 2. This prevents broad repository mutation.
- Read target files through project-root-safe validation.
- Ask the model for a structured fix plan and optional replacement content.
- In dry run, render the plan and write a report only.
- In apply mode, validate policy, write the modified file, run formatter/verification commands if configured, and render delivery report.

## 7. Architecture

### 7.1 New Domain Types

Add focused domain types under `core`:

- `FixPlanOutput`
- `FixChange`
- `VerificationCommand`
- `VerificationResult`
- `DeliveryOutput`
- `LearningOutput`
- `LearningRecord`

These types represent structured data after model parsing or tool execution. They should not know about filesystem or CLI details.

### 7.2 Tool Layer

Introduce a small tool layer rather than letting workflows call everything directly:

```text
tools/
  fs.rs
  command.rs
```

`ProjectFs` responsibilities:

- Resolve paths under project root.
- Reject paths outside project root.
- Reject symlink writes in Phase 2.
- Read UTF-8 files.
- Write files atomically enough for local development.

`CommandRunner` responsibilities:

- Run formatter or verification commands in `project_root`.
- Capture command, exit code, stdout summary, stderr summary, and duration.
- Never run commands by model decision alone. Commands come from CLI args or config.

Phase 2 should avoid a large `AgentTool` abstraction. Keep small cohesive interfaces.

### 7.3 Policy Gate

Extend policy evaluation to cover Phase 2 actions:

- `WriteLocalFile`
- `RunFormatter`
- `RunTest`
- `AppendMemory`

`fix --dry-run` uses only read/analyze/report actions.

`fix --apply` requires:

- explicit `--apply`
- target path inside project root
- policy decision that local write is allowed
- report of planned change before write

The CLI flag is the human confirmation in Phase 2. Interactive prompts can come later.

### 7.4 Workflow Boundaries

`FixWorkflow` orchestrates:

```text
load target file -> load context -> request fix plan -> render dry-run report
if apply:
  policy check -> write file -> run format/verify -> render delivery report -> append memory
```

`LearnWorkflow` orchestrates:

```text
load optional source report -> load context -> request structured learning -> write markdown knowledge -> append learning memory -> render report
```

Workflows may call `AgentClient`, `ContextLoader`, `ProjectFs`, `CommandRunner`, `JsonlMemoryStore`, and report renderers. They should not contain raw path traversal logic, command process plumbing, or JSONL append mechanics.

## 8. Data Flow

### 8.1 Learn Flow

```text
CLI args
  -> load .env
  -> LearnWorkflow
  -> optional ProjectFs.read_text(source report)
  -> ContextLoader
  -> AgentClient.complete_json(LearningOutput)
  -> render learning markdown
  -> ProjectFs.append_text(knowledge target)
  -> JsonlMemoryStore.append_learning
  -> report output
```

### 8.2 Fix Dry Run Flow

```text
CLI args
  -> load .env
  -> ProjectFs.read_text(target file)
  -> ContextLoader
  -> AgentClient.complete_json(FixPlanOutput)
  -> render fix plan report
  -> no code writes
```

### 8.3 Fix Apply Flow

```text
Fix dry-run data
  -> require --apply
  -> PolicyGate.evaluate(WriteLocalFile)
  -> ProjectFs.write_text(target file)
  -> optional CommandRunner.run(format)
  -> optional CommandRunner.run(verify)
  -> render delivery report
  -> append task/decision/learning memory as appropriate
```

## 9. Model Output Contracts

### 9.1 Fix Plan Output

Required fields:

- `summary`
- `target_files`
- `change_intent`
- `risk_level`
- `risks`
- `verification_commands`
- `replacement_files`
- `open_questions`

`replacement_files` should contain full replacement content for Phase 2. Do not implement patch application first; full-file replacement is easier to validate, test, and reason about for small target files.

Phase 2 should cap target file size before full replacement. Large files should fail with a message asking for a narrower file or manual review.

### 9.2 Learning Output

Required fields:

- `title`
- `category`
- `summary`
- `rule`
- `evidence`
- `applies_to`
- `target_doc`

The model can suggest a target doc, but final routing is controlled by workflow rules and CLI hints.

## 10. Configuration

Extend `.ai-human/config.toml` later, not `.env`, for project behavior:

```toml
[fix]
max_file_bytes = 65536
default_format_commands = []
default_verify_commands = []

[learn]
default_target = "engineering-rules"
```

Phase 2 can start with hard-coded defaults and introduce config parsing when tests require it. Avoid adding config complexity before behavior is stable.

## 11. Reports

Add renderers for:

- fix plan report
- delivery report
- learning report

Report requirements:

- Include input summary.
- Include target paths.
- Include model-proposed risks.
- Include whether writes occurred.
- Include verification commands and exit status.
- Include residual risks and open questions.

Dry-run reports must clearly say no files were modified.

## 12. Safety Rules

- Default `fix` behavior must be no-write.
- `fix --apply` must require explicit target path.
- All read/write paths must stay inside project root after canonicalization.
- Phase 2 must reject symlink target writes.
- Commands must run in project root.
- Commands must come from CLI args or project config, not arbitrary model output.
- Verification failure must make the delivery report say failed or incomplete.
- The tool must never claim success just because files were written.

## 13. Testing Strategy

### Unit Tests

- `.env` loading stays non-overriding.
- `ProjectFs` rejects parent traversal, absolute outside paths, missing files, symlink writes, and oversized files.
- `CommandRunner` captures success and failure.
- `PolicyGate` returns expected decisions for write and verify actions.
- Markdown renderers are deterministic.

### Workflow Tests

- `learn` writes Markdown knowledge and appends `learnings.jsonl`.
- `learn --source-report` rejects paths outside project root.
- `fix` without `--apply` writes only a report and leaves target file unchanged.
- `fix --apply` writes a small target file when policy allows.
- `fix --apply --verify <failing command>` writes a delivery report showing verification failure.
- `fix` rejects missing `--path`.

### CLI Smoke Tests

- `ai-human learn --input ...` prints a learning report.
- `ai-human fix --input ... --path file` prints dry-run report.
- `ai-human fix --input ... --path file --apply` applies mock replacement content in tests.

## 14. Acceptance Criteria

Phase 2 is accepted when:

- `learn` can persist at least one useful engineering rule to Markdown and JSONL.
- `fix` dry-run produces a useful plan without modifying files.
- `fix --apply` can complete one controlled local edit on a small test file.
- Verification evidence is included in the delivery report.
- Path and symlink safety tests pass.
- CLI output is understandable without reading source code and includes written paths for every generated artifact.
- Friendly defaults let a first-time user complete `learn` with one command.
- Existing `ask`, `plan`, `impact`, and `review` behavior remains unchanged.
- `cargo fmt -- --check`, `cargo test`, `cargo check`, and `cargo clippy --all-targets -- -D warnings` pass.

## 15. Open Decisions

The recommended defaults are:

- Implement `learn` before `fix`.
- Start `fix` with one required `--path`.
- Use full-file replacement instead of patch hunks in Phase 2.
- Treat `--apply` as explicit human confirmation for local writes.
- Keep command execution limited to CLI/config-provided commands.

These defaults prioritize safety and testability over autonomy.

## 16. Design Self-Review

- Placeholder scan: no TBD/TODO placeholders are present.
- Scope check: the design covers only Phase 2 CLI learn/fix capabilities, not Web, IM, RAG, or autonomous VCS operations.
- Boundary check: model calls, filesystem, command execution, policy, workflow, memory, and report rendering remain separate responsibilities.
- Ambiguity check: default `fix` behavior is explicitly dry-run and write behavior requires `--apply`.
