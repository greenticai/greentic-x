# PR-GX-TOOLCHAIN-04
Add Pack Compatibility Mapping Without Pack Execution

## Goal

Prepare GX outputs for `greentic-pack` reuse without reimplementing pack
behavior in this repo.

## Problem

GX should be able to tell downstream tooling what pack-oriented inputs are
needed, but the actual pack wizard/build/update/sign workflow already belongs to
`greentic-pack`.

## Deliverables

- New mapping module:
  - `crates/gx/src/wizard/intent_to_pack.rs`
- Optional generated output:
  - `<solution-id>.pack.input.json`
- Mapping docs:
  - `docs/architecture/gx-to-pack-flow.md`

## Mapping Responsibilities

- Translate GX composition results into a pack-oriented compatibility document:
  - required provider references
  - required capability offers
  - flow suggestions
  - component/provider hints
  - defaults and template selections
- Record what GX knows and what remains unresolved for the pack toolchain.

## Explicit Constraint

This PR must not:

- scaffold packs
- run `greentic-pack wizard`
- run `greentic-pack build`
- run `greentic-pack resolve`
- write `pack.yaml`

Those remain external tool responsibilities.

## Relationship To `greentic-cap`

- Reuse `greentic-cap` concepts where they help describe required capabilities.
- Do not create a parallel capability model in GX.
- If GX needs a local compatibility type, document how it maps to
  `greentic-cap` concepts and where the mapping is partial.

## Acceptance Criteria

- GX can emit a pack-oriented compatibility document from solution intent.
- The document is explicit about known values vs unresolved downstream work.
- No pack execution logic is introduced into `greentic-x`.
