# Audit: `greentic-dev wizard`

## Current Responsibilities

- Owns the current top-level launcher experience for wizard flows.
- Defines the launcher `AnswerDocument` envelope.
- Offers a small launcher menu that selects either `pack` or `bundle`.
- Builds a simple execution plan of delegated commands.
- Embeds delegated answer schemas from `greentic-pack` and
  `greentic-bundle` into the launcher schema output.
- Executes delegated commands under an allowlisted command runner.

## Current Inputs

- launcher `AnswerDocument`
- interactive launcher selection (`pack` or `bundle`)
- delegated answer documents for pack or bundle flows
- locale, schema-version, migrate, dry-run, and execution flags

## Current Outputs

- launcher plan JSON
- delegated process execution of:
  - `greentic-pack wizard ...`
  - `greentic-bundle wizard ...`
- launcher-wrapped emitted answers containing:
  - `selected_action`
  - `delegate_answer_document`

## Extension Points

- The current extension model is hard-coded to two actions:
  - `pack`
  - `bundle`
- It has a plan/executor structure that could support broader delegation later,
  but no generic plugin or extension provider interface exists yet.
- The closest current integration seam is compatibility with the launcher
  envelope and delegated answer shape.

## Gaps Vs Desired GX / DW Model

- No generic wizard extension host yet.
- No direct concept of GX as a composition extension provider.
- No industry-layer chaining model.
- No dynamic discovery of extension modules.

## Implication For `greentic-x`

- GX should target launcher compatibility instead of bypassing the launcher
  forever.
- The correct near-term contract is:
  - GX produces composition outputs
  - GX can emit launcher-compatible handoff artifacts
  - `greentic-dev` remains the future integration host
