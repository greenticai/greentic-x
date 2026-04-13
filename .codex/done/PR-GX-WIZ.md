# PR-GX-WIZ Architecture Lock

This document is the locked implementation baseline for GX wizard work.

## Status

- PR-GX-WIZ-00: done (audit + architecture lock)
- PR-GX-WIZ-01: done in `gx` (`wizard run|validate|apply` command surface + flags + tests)
- PR-GX-WIZ-02: done in `gx` (answer envelope + schema/migration validation + tests)
- PR-GX-WIZ-03: done in `gx` (assistant-bundle normalization + deterministic plan/warnings + bundle handoff wiring + distributor remote resolution/pinning)
- PR-GX-WIZ-04: done in `gx` (`--mode` flows for assistant/domain template create/update + source-ref validation + template apply materialization)
- PR-GX-WIZ-05: done in `gx` (QA-driven interactive flow, embedded i18n, replay hardening, and cross-tool `.gtbundle` smoke replay test)
- Compatibility update: emitted `gx wizard` answers now use `greentic-bundle.wizard.run` / `greentic-bundle.wizard.answers` metadata and include bundle-required replay fields (`mode`, `bundle_name`, `bundle_id`, `output_dir`).
- Added gated replay smoke test in `gx`: emits answers, replays through `greentic-bundle wizard apply --answers`, and verifies `.gtbundle` output when `greentic-bundle` is available in `PATH`.
- Wizard runtime now executes through explicit staged lifecycle in code: `wizard_spec(...) -> wizard_apply(...) -> wizard_execute_plan(...)`.
- Started physical module extraction: staged wizard orchestrator moved to `crates/gx/src/wizard/mod.rs`; CLI routing now delegates there.
- Continued module extraction: wizard normalization, answer-document parsing/migration checks, plan summary/writes computation, and bundle handoff helpers moved into `crates/gx/src/wizard/mod.rs`.
- `crates/gx/src/lib.rs` now retains CLI wiring and non-wizard tooling paths while wizard behavior is centralized under `crates/gx/src/wizard/mod.rs`.
- Wizard module now has internal submodules for structure:
  - `crates/gx/src/wizard/plan.rs` (plan/action/summary/writes/warnings)
  - `crates/gx/src/wizard/handoff.rs` (answers-path resolution + bundle handoff execution/invocation)
- Added `crates/gx/src/wizard/answers.rs` for answer document parsing/schema handling/mode normalization and source-policy validation.
- `:latest` policy handling now supports interactive TTY prompt (`keep_latest` vs `pin`) when policy is missing in run/apply flows.
- Added `crates/gx/src/wizard/remote.rs` with `greentic-distributor-client v0.4` integration for execute-mode remote source resolution (`oci://`, `repo://`, `store://`), digest lock recording, and optional `:latest` pin rewrite when `latest_policy=pin`.
- Added `crates/gx/src/wizard/qa.rs` and wired interactive execute-mode answer collection through `greentic-qa-lib` (`WizardDriver`) for TTY flows without `--answers`.
- Added template mode execute side effects (`crates/gx/src/wizard/template.rs`): `run/apply` now materialize deterministic JSON template outputs at `template_output_path`.
- Added replay-hardening coverage for template modes: emitted answers (`--emit-answers`) are replayed through `gx wizard apply --answers` and verified to produce the template artifact.
- Added deterministic plan regression tests for bundle/template dry-run flows and tightened lock metadata assertions for remote latest-policy resolution.
- Added locale initialization fallback order for wizard flows: CLI `--locale` > replay document locale > environment variables > `en`.

## Fixed Decisions

1. Command shape:
- `gx wizard [run|validate|apply]`

2. Answer/replay compatibility baseline:
- use the `greentic-component` / `greentic-bundle` answer envelope
- do not use the older `greentic-flow` replay shape as canonical

3. Bundle output target:
- produce `.gtbundle`
- generated answers must be consumable by:
  - `greentic-bundle wizard --answers <file>`

4. i18n scope:
- all user-facing prompts/help/errors localized
- embedded locale catalogs with fallback

5. Remote refs:
- use `greentic-distributor-client` `v0.4`
- when a ref uses `:latest`, prompt user to keep latest or pin

## Tooling Rules For Codex

When making changes in adjacent repos:

- For packs/flows/components in `greentic-flow`:
  - use `greentic-pack wizard`
- For creating new components:
  - use `greentic-component wizard`
- For bundle generation/replay:
  - use `greentic-bundle wizard`

All of these support dry-run and answer replay workflows:

- `--dry-run`
- `--emit-answers /tmp/answers.json`
- `--answers <file>`

---

PR-GX-WIZ-00
Audit baseline and architecture lock
PR-GX-WIZ-00-audit-baseline-and-lock.md

# PR-GX-WIZ-00
Audit Baseline and Architecture Lock

## Goal

Finalize the wizard baseline from:

- `../greentic-flow`
- `../greentic-bundle`
- `../greentic-component`

and lock the architecture decisions for implementation.

## Baseline Mapping

- `greentic-component`:
  - answer envelope baseline
  - run/validate/apply command semantics
- `greentic-bundle`:
  - bundle handoff and `.gtbundle` lifecycle baseline
  - replay target (`wizard --answers`)
- `greentic-flow`:
  - provider-style `spec/apply/execute_plan` pattern

## Deliverables

- `docs/wizard-audit.md`
- this PR plan (`.codex/PR-GX-WIZ.md`) updated as architecture lock

## Acceptance Criteria

- baseline source-of-truth is explicit and conflict-free
- answer envelope baseline is locked to greentic-component/greentic-bundle style
- lifecycle baseline is locked to `run|validate|apply`

---

PR-GX-WIZ-01
GX wizard foundation and command surface
PR-GX-WIZ-01-wizard-foundation-and-cli-shape.md

# PR-GX-WIZ-01
GX Wizard Foundation and CLI Shape

## Goal

Implement the exact command surface:

- `gx wizard run`
- `gx wizard validate`
- `gx wizard apply`

## Required Flags

- `--answers <file>`
- `--emit-answers <file>`
- `--dry-run`
- `--locale <locale>`
- `--schema-version <ver>` (if schema versioning is enabled in this stage)
- `--migrate` (if schema migration is enabled in this stage)

## Lifecycle Contract

Adopt the combined deterministic model:

- `spec(...) -> QaSpec`
- `apply(...) -> WizardPlan`
- `execute_plan(...)`

with explicit run modes:

- `run`
- `validate`
- `apply`

## Acceptance Criteria

- command shape is exactly `gx wizard [run|validate|apply]`
- `validate` has no side effects
- `run/apply` replay semantics are deterministic with `--answers`
- `--emit-answers` output is available for replay

---

PR-GX-WIZ-02
Normalized request and answer document contract
PR-GX-WIZ-02-normalized-request-and-answer-contract.md

# PR-GX-WIZ-02
Normalized Request and Answer Document Contract

## Goal

Define and implement the canonical GX answer envelope for assistant-bundle workflows.

## Required Envelope Fields

- `wizard_id`
- `schema_id`
- `schema_version`
- `locale`
- `answers`
- `locks`

## Compatibility Requirement

- compatible with greentic-component / greentic-bundle style envelope
- replay-safe via `--answers`
- emit-safe via `--emit-answers`

## Versioning

- define schema-version behavior
- define migration behavior for future schema changes

## Acceptance Criteria

- emitted answer document includes all required envelope fields
- answer document round-trips deterministically
- version mismatch behavior is explicit and tested

---

PR-GX-WIZ-03
Assistant bundle wizard flow
PR-GX-WIZ-03-assistant-bundle-wizard-flow.md

# PR-GX-WIZ-03
Assistant Bundle Wizard Flow

## Goal

Implement assistant bundle workflow end-to-end with deterministic plan generation and `.gtbundle` output handoff.

## Required Behavior

- interactive `run`
- side-effect-free `validate`
- side-effecting `apply`
- deterministic plan generation
- remote refs resolved via `greentic-distributor-client v0.4`
- prompt for pinning when `:latest` is used

## Workflow Content

Include the refined menu/flow for:

- assistant template source
- domain template source
- deployment profile
- deployment target
- bundle identity
- provider categories
- plan review

## Bundle Handoff

Wizard output must support replay into bundle generation:

- `greentic-bundle wizard --answers <file>`

## Acceptance Criteria

- plan is deterministic across repeated replay
- apply path produces expected authored outputs for bundle handoff
- replay output can be consumed by greentic-bundle

---

PR-GX-WIZ-04
Assistant/domain template authoring and loading
PR-GX-WIZ-04-assistant-domain-template-modes.md

# PR-GX-WIZ-04
Assistant/Domain Template Modes

## Goal

Implement template-specific wizard capabilities as first-class modes.

## Required Modes

- create assistant template
- update assistant template
- create domain template
- update domain template

## Source Types

- local path
- `file://`
- `oci://`
- `repo://`
- `store://`

All remote fetching:

- via `greentic-distributor-client v0.4`
- prompt on `:latest` to keep or pin

## Acceptance Criteria

- template modes are first-class wizard flows
- template load/update works across supported source types
- replay works for template flows as well

---

PR-GX-WIZ-05
QA + i18n + compatibility hardening
PR-GX-WIZ-05-qa-i18n-compat-hardening.md

# PR-GX-WIZ-05
QA, i18n, and Compatibility Hardening

## Goal

Harden wizard quality gates and cross-tool replay compatibility.

## QA Requirements

- QA-driven prompt/forms via `greentic-qa-lib`
- validation errors in QA-compatible shape
- interactive and replay paths converge on same normalized request path

## i18n Requirements

- embedded locale catalogs
- locale init from CLI/env
- fallback behavior
- all user-facing wizard text through localization (`tr(...)`-style)

## Non-Negotiable Integration Test

Add integration smoke test that performs:

1. `gx wizard --emit-answers <file> ...`
2. `greentic-bundle wizard --answers <file>`
3. verify resulting `.gtbundle` exists

## Acceptance Criteria

- QA + replay + interactive flows are consistent
- user-facing wizard text is fully localized
- emitted GX answers successfully drive greentic-bundle replay to produce `.gtbundle`

---

## Recommended GX Internal Structure

Use this project layout for implementation:

- CLI adapter layer:
  - `crates/gx/src/cmd/wizard.rs`
  - parse run|validate|apply, locale, answer I/O

- Core wizard layer:
  - `crates/gx/src/wizard/mod.rs`
  - spec, normalize request, build plan, validate, execute

- Answer document layer:
  - `crates/gx/src/answers/document.rs`
  - envelope type, schema ids/versioning, locks, serde

- i18n layer:
  - `crates/gx/src/i18n/mod.rs`
  - embedded catalogs, fallback, translation helpers

## Final Implementation Guidance

Do not invent a parallel wizard framework.

Instead:

- mirror greentic-component answer envelope semantics
- mirror greentic-bundle run/validate/apply lifecycle semantics
- reuse greentic-flow provider-style `spec/apply/execute_plan` where it fits
- localize all user-facing wizard text
- validate plans through QA-style flows
- prove replay interoperability through greentic-bundle handoff test
