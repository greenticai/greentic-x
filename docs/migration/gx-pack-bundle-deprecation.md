# GX Pack/Bundle Deprecation

## Purpose

This note records the explicit deprecation of legacy wording and workflow
assumptions that made `gx` look like a pack or bundle generator.

## Deprecated Framing

These ideas are now deprecated in `greentic-x`:

- treating `gx` as the owner of pack creation
- treating `gx` as the owner of bundle generation
- treating `gx wizard apply` as the primary packaging workflow
- treating direct `greentic-bundle` invocation from GX as the long-term design

## Current Boundary

`gx` owns:

- composition input collection
- solution intent generation
- compatibility and handoff artifact emission
- deterministic replay for GX-controlled artifacts

`gx` does not own:

- `greentic-pack` scaffolding, doctor, resolve, build, sign, or manifest state
- `greentic-bundle` normalization, setup, replay, or `.gtbundle` generation
- `greentic-dev` launcher orchestration

## Transitional Compatibility Paths

Two compatibility paths remain on purpose:

- `<solution-id>.launcher.answers.json` for current `greentic-dev` launcher
  compatibility
- optional direct bundle handoff through `greentic-bundle` for temporary replay
  scenarios

These are bridges, not ownership claims.

## Practical Guidance

- Prefer `gx wizard run` to generate composition outputs and handoff artifacts.
- Treat `gx wizard apply` as a deprecated compatibility bridge.
- Feed emitted artifacts into downstream tools rather than extending GX with new
  packaging behavior.
