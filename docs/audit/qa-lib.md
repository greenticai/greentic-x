# Audit: `greentic-qa-lib`

## Current Responsibilities

- Provides reusable QA runtime primitives for wizard-style flows.
- Defines a frontend-neutral driver based on QA specs.
- Supports text, JSON UI, and card/adaptive-card style presentation modes.
- Evaluates progress and validation through the QA spec stack.

## Current Inputs

- QA spec JSON
- optional initial answers
- selected frontend
- i18n settings
- answer patches supplied by a caller or interactive loop

## Current Outputs

- wizard progress payloads
- validation results
- final answer sets
- serialized answer-set forms for downstream callers

## Extension Points

- QA specs with:
  - `visible_if`
  - defaults
  - validations
  - includes
  - i18n-aware prompts
- pluggable frontends over the same payload
- host-defined answer providers

## Gaps Vs Desired GX / DW Model

- `greentic-qa-lib` is a form/runtime layer, not the owner of top-level wizard
  orchestration.
- Current pack and bundle wizards already reuse it.
- GX currently does not fully reuse it for its interactive composition flow and
  still has bespoke prompt/menu code.

## Implication For `greentic-x`

- Reuse `greentic-qa-lib` for GX interactive collection where possible.
- Do not confuse QA reuse with top-level launcher ownership.
- A future GX QA migration should replace bespoke prompting, not create a second
  orchestration layer.
