PR-GX-CATALOG-01 — Add gx catalog commands and remote catalog consumption

Summary

- Add `gx catalog init <repo-name>`
- Add `gx catalog build`
- Add `gx catalog validate`
- Extend `gx wizard` with repeated `--catalog <ref>` support
- Keep catalog authoring separate from the normal compose flow
- Keep final bundle generation delegated to `greentic-bundle`

Implemented direction

- `schemas/catalog-index.schema.json` defines the canonical root `catalog.json`
  contract for solution catalog repos.
- `gx catalog init` scaffolds a standard solution catalog repo layout with an
  initial valid `catalog.json`.
- `gx catalog build` scans the standard repo layout and regenerates a canonical
  sorted root `catalog.json`.
- `gx catalog validate` validates `catalog.json`, checks refs, checks duplicate
  IDs, and validates supported typed assets against embedded GX schemas where
  available.
- `gx wizard --catalog <ref>` now merges explicit local and OCI-backed solution
  catalogs with the built-in GX base catalog before composition.

Wizard resolution model

- Base local GX catalog entries load first.
- Explicit `--catalog` sources load next.
- OCI catalogs are fetched through `greentic-distributor-client`.
- Resolved entries retain provenance metadata:
  - `source_type`
  - `source_ref`
  - `resolved_digest`
- Moving refs default to `update_then_pin` behavior in downstream artifacts.

Delegation boundary

- `gx` composes solutions and emits handoff artifacts.
- `greentic-bundle` remains responsible for:
  - bundle assembly
  - bundle manifest packing
  - archive creation
  - writing `.gtbundle`

Follow-up packaging requirement

- `gx catalog init` should scaffold a starter `Cargo.toml` for downstream
  solution repos that uses versioned registry dependencies rather than local
  path dependencies.
- The scaffold should reference Greentic-X support crates with `0.4` versions
  so downstream repos such as `zain-x` can consume published crates without a
  sibling checkout:
  - `greentic-x-contracts = "0.4"`
  - `greentic-x-flow = "0.4"`
  - `greentic-x-ops = "0.4"`
  - `greentic-x-runtime = "0.4"`
  - `greentic-x-types = "0.4"`
