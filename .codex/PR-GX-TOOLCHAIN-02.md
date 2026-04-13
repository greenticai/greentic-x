# PR-GX-TOOLCHAIN-02
Introduce Stable GX Handoff Contracts

## Goal

Define the exact contracts that GX emits for downstream tools so `greentic-dev`,
`greentic-pack`, and `greentic-bundle` can consume GX output without GX owning
their runtime logic.

## Problem

Right now GX outputs are useful, but they are too tied to current GX wizard
implementation details. We need an explicit, versioned handoff boundary.

## Deliverables

- New handoff schemas under `schemas/`:
  - `solution-intent.schema.json`
  - `toolchain-handoff.schema.json`
  - optional `pack-input.schema.json` if GX can define a stable internal
    compatibility shape without claiming ownership of pack execution
- New Rust types in `crates/gx/src/wizard/compose.rs` or a dedicated module:
  - `ResolvedSolutionIntent`
  - `ToolchainHandoff`
  - `BundleHandoff`
  - `PackHandoff`
- Versioned emitted JSON files under `dist/` in execute flows:
  - `<solution-id>.solution.json`
  - `<solution-id>.toolchain-handoff.json`
  - `<solution-id>.bundle.answers.json` when bundle compatibility output is
    requested
  - `<solution-id>.pack.input.json` when pack compatibility output is requested

## Required Fields

### `ResolvedSolutionIntent`

- `solution_kind`
- `solution_name`
- `solution_id`
- `description`
- `required_capabilities`
- `provider_presets`
- `required_contracts`
- `suggested_flows`
- `catalog_refs`
- `defaults`
- `notes`

### `ToolchainHandoff`

- `schema_version`
- `solution_intent_ref`
- `bundle_handoff`
- `pack_handoff`
- `provenance`
- `locks`

## Important Constraint

These are GX-owned compatibility artifacts, not replacements for
`greentic-pack` or `greentic-bundle` answer documents. They exist to isolate GX
composition logic from downstream execution details.

## Non-Goals

- No command execution against sibling tools in this PR.
- No `greentic-dev` launcher changes yet.

## Acceptance Criteria

- GX emits stable, versioned handoff JSON.
- The handoff structures are documented and schema-validated.
- Downstream integration work can target these files without depending on GX
  internals.
