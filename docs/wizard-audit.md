# Wizard Audit

## Scope

Audit target repositories:

- `../greentic-flow`
- `../greentic-bundle`
- `../greentic-component`

Goal:

- establish an implementation baseline for `gx wizard`
- align replay/answers behavior with `greentic-component`
- support `.gtbundle` generation through `greentic-bundle wizard --answers ...`

## Wizard Architecture

### greentic-flow

CLI entrypoint:

- `greentic-flow` routes `wizard` through `Commands::Wizard(WizardArgs)` and `handle_wizard`.
- Source: [greentic-flow/src/bin/greentic-flow.rs](../greentic-flow/src/bin/greentic-flow.rs)

Command structure:

- `wizard <pack> [--answers-file] [--emit-answers] [--emit-schema] [--dry-run]`
- Source: [greentic-flow/src/bin/greentic-flow.rs:580](../greentic-flow/src/bin/greentic-flow.rs:580)

Module layout:

- interactive wizard/menu logic in `src/bin/greentic-flow.rs`
- deterministic scaffold provider in `src/wizard/mod.rs`
- provider contract documented in `docs/wizard/README.md`
- Source: [greentic-flow/src/wizard/mod.rs](../greentic-flow/src/wizard/mod.rs), [greentic-flow/docs/wizard/README.md](../greentic-flow/docs/wizard/README.md)

Notes:

- `greentic-flow` currently has two wizard shapes: menu/staging flow and provider `spec/apply/execute_plan` scaffold flow.
- `docs/cli.md` still documents `wizard new`, while current CLI args are `wizard <pack> ...`.
- Source: [greentic-flow/docs/cli.md:39](../greentic-flow/docs/cli.md:39), [greentic-flow/src/bin/greentic-flow.rs:580](../greentic-flow/src/bin/greentic-flow.rs:580)

### greentic-bundle

CLI entrypoint:

- `greentic-bundle` routes `wizard` from root CLI to `src/cli/wizard.rs`.
- Source: [greentic-bundle/src/cli/mod.rs:47](../greentic-bundle/src/cli/mod.rs:47)

Command structure:

- `wizard [run|validate|apply]`
- bare `wizard` defaults to interactive execution path.
- Source: [greentic-bundle/src/cli/wizard.rs](../greentic-bundle/src/cli/wizard.rs)

Module layout:

- CLI adapter in `src/cli/wizard.rs`
- core request normalization, plan generation, replay, apply in `src/wizard/mod.rs`
- answer document model in `src/answers/document.rs`
- Source: [greentic-bundle/src/wizard/mod.rs](../greentic-bundle/src/wizard/mod.rs), [greentic-bundle/src/answers/document.rs](../greentic-bundle/src/answers/document.rs)

### greentic-component

CLI entrypoint:

- root CLI routes `wizard` to `cmd::wizard::run_cli`.
- Source: [greentic-component/crates/greentic-component/src/cli.rs:35](../greentic-component/crates/greentic-component/src/cli.rs:35)

Command structure:

- `wizard [run|validate|apply] --mode ... [--answers] [--emit-answers] [--schema-version] [--migrate]`
- supports legacy compatibility flags (`--qa-answers`, `--qa-answers-out`).
- Source: [greentic-component/docs/cli.md:39](../greentic-component/docs/cli.md:39), [greentic-component/crates/greentic-component/src/cmd/wizard.rs:81](../greentic-component/crates/greentic-component/src/cmd/wizard.rs:81)

Module layout:

- CLI adapter and replay envelope in `src/cmd/wizard.rs`
- deterministic scaffold core in `src/wizard/mod.rs`
- Source: [greentic-component/crates/greentic-component/src/cmd/wizard.rs](../greentic-component/crates/greentic-component/src/cmd/wizard.rs), [greentic-component/crates/greentic-component/src/wizard/mod.rs](../greentic-component/crates/greentic-component/src/wizard/mod.rs)

## QA Integration

### greentic-flow

- Uses `greentic-qa-lib` `WizardDriver` for menu questions and interactive forms.
- Source: [greentic-flow/src/bin/greentic-flow.rs:1702](../greentic-flow/src/bin/greentic-flow.rs:1702)

### greentic-bundle

- Uses `greentic-qa-lib` for root wizard request forms and setup form prompting.
- Source: [greentic-bundle/src/wizard/mod.rs:520](../greentic-bundle/src/wizard/mod.rs:520), [greentic-bundle/src/wizard/mod.rs:3154](../greentic-bundle/src/wizard/mod.rs:3154)

### greentic-component

- Uses `greentic-qa-lib` error types and QA-style interactive handling in wizard CLI.
- Source: [greentic-component/crates/greentic-component/src/cmd/wizard.rs:10](../greentic-component/crates/greentic-component/src/cmd/wizard.rs:10)

## i18n Integration

### greentic-flow

- Locale resolution and fallback in `src/i18n.rs`.
- wizard-specific keys loaded from embedded wizard catalog and used by wizard prompts.
- Source: [greentic-flow/src/i18n.rs](../greentic-flow/src/i18n.rs), [greentic-flow/src/bin/greentic-flow.rs:20](../greentic-flow/src/bin/greentic-flow.rs:20)

### greentic-bundle

- Embedded locale catalogs generated at build time from `i18n/locales.json`.
- Root CLI and wizard strings localized through `crate::i18n::tr(...)`.
- Source: [greentic-bundle/build.rs](../greentic-bundle/build.rs), [greentic-bundle/src/i18n/mod.rs](../greentic-bundle/src/i18n/mod.rs), [greentic-bundle/src/wizard/mod.rs:292](../greentic-bundle/src/wizard/mod.rs:292)

### greentic-component

- Root CLI localizes help/subcommand text and initializes locale from CLI/env.
- Uses large embedded locale set with language fallback.
- Source: [greentic-component/crates/greentic-component/src/cli.rs:63](../greentic-component/crates/greentic-component/src/cli.rs:63), [greentic-component/crates/greentic-component/src/cmd/i18n.rs](../greentic-component/crates/greentic-component/src/cmd/i18n.rs)

## Answer Document Format

### greentic-flow

Current replay document (menu wizard):

- `schema_id`
- `schema_version`
- `answers`
- `events`

Source: [greentic-flow/src/bin/greentic-flow.rs:1571](../greentic-flow/src/bin/greentic-flow.rs:1571)

### greentic-bundle

Typed `AnswerDocument` with metadata and lock fields:

- `wizard_id`
- `schema_id`
- `schema_version` (semver)
- `locale`
- `answers`
- `locks`

Source: [greentic-bundle/src/answers/document.rs](../greentic-bundle/src/answers/document.rs)

### greentic-component

`AnswerDocument` envelope in wizard CLI:

- `wizard_id`
- `schema_id`
- `schema_version`
- `locale`
- `answers`
- `locks`

Constants:

- `ANSWER_DOC_WIZARD_ID = "greentic-component.wizard.run"`
- `ANSWER_DOC_SCHEMA_ID = "greentic-component.wizard.run"`

Source: [greentic-component/crates/greentic-component/src/cmd/wizard.rs:26](../greentic-component/crates/greentic-component/src/cmd/wizard.rs:26), [greentic-component/crates/greentic-component/src/cmd/wizard.rs:152](../greentic-component/crates/greentic-component/src/cmd/wizard.rs:152)

## Wizard Lifecycle

### greentic-flow provider lifecycle

- `spec(mode, ctx) -> QaSpec`
- `apply(mode, ctx, answers, options) -> WizardPlan`
- `execute_plan(plan)`

Source: [greentic-flow/docs/wizard/README.md:6](../greentic-flow/docs/wizard/README.md:6), [greentic-flow/src/wizard/mod.rs:86](../greentic-flow/src/wizard/mod.rs:86), [greentic-flow/src/wizard/mod.rs:168](../greentic-flow/src/wizard/mod.rs:168), [greentic-flow/src/wizard/mod.rs:247](../greentic-flow/src/wizard/mod.rs:247)

### greentic-bundle lifecycle

- parse or collect request
- normalize request
- resolve catalogs/setup specs
- build plan envelope
- optional apply
- optional emit answer document

Source: [greentic-bundle/src/wizard/mod.rs:428](../greentic-bundle/src/wizard/mod.rs:428), [greentic-bundle/src/wizard/mod.rs:2600](../greentic-bundle/src/wizard/mod.rs:2600), [greentic-bundle/src/wizard/mod.rs:2796](../greentic-bundle/src/wizard/mod.rs:2796)

### greentic-component lifecycle

- run/validate/apply command
- load and normalize answers
- build deterministic plan envelope
- optional execute side-effecting steps
- optional emit answer document

Source: [greentic-component/crates/greentic-component/src/cmd/wizard.rs:179](../greentic-component/crates/greentic-component/src/cmd/wizard.rs:179), [greentic-component/crates/greentic-component/src/cmd/wizard.rs:1249](../greentic-component/crates/greentic-component/src/cmd/wizard.rs:1249), [greentic-component/crates/greentic-component/src/cmd/wizard.rs:1162](../greentic-component/crates/greentic-component/src/cmd/wizard.rs:1162)

## Flag Behavior Audit

### `--answers`

- `greentic-flow`: uses `--answers-file` for menu wizard replay.
- `greentic-bundle`: uses `--answers` for run/validate/apply.
- `greentic-component`: uses `--answers` with legacy `--qa-answers` compatibility.

### `--emit-answers`

- present in all three, but envelope format differs.

### `--dry-run`

- present in all three.
- `greentic-bundle` and `greentic-component` express dry-run through explicit execution mode flows.

## Remote Source Resolution Audit

### greentic-bundle

- remote catalog resolution goes through `greentic-distributor-client` path and offline/cache options in wizard execution.
- Source: [greentic-bundle/src/wizard/mod.rs:437](../greentic-bundle/src/wizard/mod.rs:437)

### greentic-flow

- supports component refs (`oci://`, `repo://`, `store://`) and pinning-related behavior in add-step workflows.
- Source: [greentic-flow/docs/cli.md:115](../greentic-flow/docs/cli.md:115)

## Baseline For `gx wizard`

Fixed baseline for this repo:

- Command: `gx wizard`
- Answer/replay compatibility: follow `greentic-component` style envelope
- Bundle output: `.gtbundle`
- Replay path: generated answers must drive `greentic-bundle wizard --answers <file>`
- i18n scope: all user-facing text localized
- remote resolution: use `greentic-distributor-client` `v0.4`
- `:latest` policy: prompt user to keep latest or pin to resolved digest/reference

## Implementation Notes For Follow-up PRs

- use `crates/gx/src/wizard/` as the implementation root
- keep plan/apply deterministic and replayable
- include compatibility tests for emitted answer docs
- include an integration smoke test that runs:
  - `gx wizard --emit-answers ...`
  - `greentic-bundle wizard --answers ...`
  - verifies `.gtbundle` exists
