# Fix Default Commands Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `fix` reuse project-owned default verification and formatting commands from `.ai-human/config.toml`.

**Architecture:** Extend `AiHumanConfig` with a focused `fix` section and load it from the project root in the CLI layer. Keep `FixWorkflow` unchanged; `main.rs` merges CLI flags and config defaults into the existing `FixRequest` so workflow responsibilities stay narrow.

**Tech Stack:** Rust, serde/toml, clap, assert_cmd, assert_fs, tokio.

---

### Task 1: Config Shape

**Files:**
- Modify: `src/config.rs`
- Modify: `tests/init_workflow.rs`

- [ ] **Step 1: Write failing test**

Assert the default generated `.ai-human/config.toml` contains:

```toml
[fix]
default_verify_commands = []
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test init_config_contains_fix_defaults`
Expected: FAIL because `fix` config does not exist.

- [ ] **Step 3: Implement config structs**

Add `FixConfig` with `default_verify_commands: Vec<String>` and `default_format_command: Option<String>`, then add `fix: FixConfig` to `AiHumanConfig`.

- [ ] **Step 4: Run target test**

Run: `cargo test init_config_contains_fix_defaults`
Expected: PASS.

### Task 2: Config Loader

**Files:**
- Modify: `src/config.rs`
- Test: `tests/config_defaults.rs`

- [ ] **Step 1: Write failing tests**

Add tests for loading `.ai-human/config.toml` and returning defaults when it is missing.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --test config_defaults`
Expected: FAIL because loader function does not exist.

- [ ] **Step 3: Implement loader**

Add async `load_project_config(project_root: &Path) -> Result<AiHumanConfig>` that reads `.ai-human/config.toml` when present and returns `AiHumanConfig::default()` when missing.

- [ ] **Step 4: Run target test**

Run: `cargo test --test config_defaults`
Expected: PASS.

### Task 3: Fix CLI Merge

**Files:**
- Modify: `src/main.rs`
- Test: `tests/cli_smoke.rs`

- [ ] **Step 1: Write failing CLI tests**

Assert `fix` includes configured default verify/format commands when flags are omitted, and explicit CLI flags override config defaults.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test fix_uses_configured_default_commands`
Expected: FAIL because `fix` ignores config defaults.

- [ ] **Step 3: Implement merge**

Load project config in the `Command::Fix` branch and create `FixRequest` with:

```rust
verify_commands = if args.verify.is_empty() {
    config.fix.default_verify_commands
} else {
    args.verify
};
format_command = args.format.or(config.fix.default_format_command);
```

- [ ] **Step 4: Run target tests**

Run: `cargo test fix_uses_configured_default_commands`
Expected: PASS.

### Task 4: Documentation and Verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Document config defaults**

Add a short `.ai-human/config.toml` example for default fix commands.

- [ ] **Step 2: Format and verify**

Run:

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all PASS.

- [ ] **Step 3: Commit and push**

Run:

```bash
git add README.md src/config.rs src/main.rs tests/init_workflow.rs tests/config_defaults.rs tests/cli_smoke.rs docs/superpowers/plans/2026-07-09-fix-default-commands-config.md
git commit -m "feat: add fix default command config"
git push
```
