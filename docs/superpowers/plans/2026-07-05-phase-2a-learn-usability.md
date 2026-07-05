# Phase 2A Learn Usability Implementation Plan

Date: 2026-07-05

## Goal

Implement the first Phase 2 slice: a friendly `ai-human learn` command that turns user-provided engineering lessons into reviewable project knowledge and structured memory.

## User Experience Rules

- `ai-human learn --input "..."` must be enough for the happy path.
- The command must print the learning report and every written path.
- Errors for source report paths must be actionable and project-root safe.
- The workflow must append knowledge, not rewrite existing knowledge.
- The output should make it clear that source code files were not modified.

## Implementation Steps

1. Add failing tests for the `learn` workflow, CLI smoke path, memory append, and Markdown rendering.
2. Add `LearningOutput` and `LearningRecord` domain types.
3. Add `JsonlMemoryStore::append_learning`.
4. Add a small project filesystem helper for safe project-root reads and appends.
5. Add a learning Markdown renderer.
6. Add `LearnWorkflow` to orchestrate context loading, model JSON output, knowledge append, memory append, and report writing.
7. Add `learn` CLI arguments and main command routing.
8. Update README examples.
9. Run `cargo fmt -- --check`, `cargo test`, `cargo check`, and `cargo clippy --all-targets -- -D warnings`.

## Acceptance

- `learn` persists Markdown under `.ai-human/knowledge/`.
- `learn` appends one JSON line to `.ai-human/memory/learnings.jsonl`.
- `learn --source-report` rejects files outside the project root before model calls.
- CLI output contains the report path and knowledge path.
- Existing commands keep passing their current tests.
