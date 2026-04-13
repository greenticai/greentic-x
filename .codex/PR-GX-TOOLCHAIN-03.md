# PR-GX-TOOLCHAIN-03
Add Greentic-Dev Launcher Compatibility

## Goal

Make GX output compatible with the current `greentic-dev wizard` launcher model
so future integration into `gtc wizard` can happen without redesigning GX.

## Problem

`greentic-dev` expects a launcher envelope with `selected_action` and an
optional delegated answer document. GX currently bypasses that launcher and
calls `greentic-bundle` directly.

## Deliverables

- A launcher compatibility module in `crates/gx/src/wizard/`:
  - `launcher.rs`
  - helper functions to emit a `greentic-dev`-compatible launcher envelope
- Generated compatibility artifact:
  - `<solution-id>.launcher.answers.json`
- Tests that validate the emitted launcher envelope against the current
  `greentic-dev` expectations captured in the audit

## Required Behavior

- GX must be able to emit:
  - a launcher document whose `wizard_id` / `schema_id` match the
    `greentic-dev` launcher
  - `answers.selected_action = "bundle"` when the GX path is targeting bundle
    generation
  - `answers.delegate_answer_document` carrying the bundle answer document
- GX must not execute `greentic-dev` in this PR.
- GX must not assume `greentic-dev` will stay unchanged forever; the
  compatibility module should be isolated and documented as current-toolchain
  compatibility code.

## Code Targets

- `crates/gx/src/wizard/mod.rs`
- `crates/gx/src/wizard/handoff.rs`
- `crates/gx/src/wizard/answers.rs`
- new launcher compatibility tests

## Non-Goals

- No actual `gtc` integration.
- No new launcher commands in sibling repos.

## Acceptance Criteria

- GX can emit a launcher-compatible answer file alongside bundle answers.
- The compatibility output is deterministic and replay-safe.
- Existing direct bundle handoff can be preserved temporarily, but the launcher
  compatibility path exists and is tested.
