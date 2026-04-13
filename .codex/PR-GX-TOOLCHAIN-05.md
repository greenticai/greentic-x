# PR-GX-TOOLCHAIN-05
Replace Custom GX Prompting With QA-Spec Forms

## Goal

Stop growing a custom GX wizard UI model and move interactive composition onto
`greentic-qa-lib` / `qa-spec`, matching the rest of the toolchain more closely.

## Problem

`gx` currently has custom prompt/menu code in `crates/gx/src/wizard/qa.rs`.
That duplicates interactive logic already present in the Greentic QA stack and
makes later integration into `greentic-dev` harder.

## Deliverables

- QA specs for GX composition flows under a new directory, for example:
  - `crates/gx/questions/core.json`
  - `crates/gx/questions/composition.json`
  - `crates/gx/questions/providers.json`
- Replace or heavily shrink `crates/gx/src/wizard/qa.rs`
- New loader/adapter code that feeds QA-spec answers into existing GX
  normalization/composition logic

## Required QA Features

- `visible_if`
- defaults
- validation rules
- reusable includes where useful
- locale-aware prompts

## Important Boundary

This PR still does not make `greentic-qa-lib` the top-level workflow owner.
It only makes GX interactive collection reuse the same QA model that pack and
bundle already use.

## Acceptance Criteria

- GX interactive flows run through QA specs instead of bespoke prompts for the
  core composition path.
- Non-interactive replay continues to work with deterministic emitted answers.
- Existing normalized composition logic remains separate from the UI/form layer.
