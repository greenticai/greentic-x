# PR-GX-TOOLCHAIN-01
Refactor GX Into Composition-Only Core

## Goal

Remove the remaining architectural assumption that `gx wizard` owns packaging
execution. GX should produce composition results and handoff artifacts only.

## Problem

Today `gx` already delegates bundle creation to `greentic-bundle`, but the code
and docs still frame GX as if it owns a packaging-oriented wizard flow. That
creates duplication against `greentic-dev`, `greentic-pack`, and
`greentic-bundle`.

## Required Changes

- Reframe the GX wizard around solution composition only.
- Rename internal concepts where necessary so they stop implying GX is the pack
  or bundle orchestrator.
- Make the main GX output a resolved solution intent plus handoff artifacts.
- Keep optional bundle handoff only as a compatibility bridge, not as the core
  product definition of GX.

## Deliverables

- New or updated internal types under `crates/gx/src/wizard/`:
  - `ResolvedSolutionIntent`
  - composition request/normalized answers types
  - handoff artifact structs separated from execution behavior
- Updated docs:
  - `docs/tooling-overview.md`
  - `docs/how-to-build-a-downstream-solution.md`
  - `docs/architecture.md`
- New migration note:
  - `docs/migration/gx-composition-only.md`

## Detailed Scope

- Keep:
  - catalog loading
  - template selection
  - provider preset selection
  - defaults merging
  - remote ref pinning
  - generated `solution.json`
  - generated handoff JSON for downstream tools
- Remove or de-emphasize:
  - GX as owner of bundle production
  - GX as owner of any future pack production
  - docs/tests that describe GX as a packaging tool rather than a composition
    tool

## Code Targets

- `crates/gx/src/wizard/mod.rs`
- `crates/gx/src/wizard/compose.rs`
- `crates/gx/src/wizard/plan.rs`
- `crates/gx/src/wizard/handoff.rs`
- any wizard-facing docs/tests that still assume GX packaging ownership

## Non-Goals

- No `greentic-dev` integration yet.
- No new QA form model yet.
- No new sibling-repo schemas yet.

## Acceptance Criteria

- GX can still produce deterministic solution outputs.
- GX no longer describes itself as the owner of bundle/pack generation.
- The composition result is represented independently from downstream
  tool-specific execution.
- Optional bundle handoff remains available only as a compatibility path.
