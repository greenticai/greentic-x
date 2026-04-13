# GX Composition-Only Migration

## Purpose

This note records the post-audit shift in how `gx` should be understood inside
`greentic-x`.

## Old Framing

Older GX wizard work often described `gx` as if it owned a packaging-oriented
wizard flow that generated final bundle outputs itself.

That framing is now retired.

## New Framing

`gx` is the solution composition engine in this repo.

It owns:

- solution composition inputs
- catalog/template/provider selection
- defaults merging
- solution intent and handoff artifact emission
- validation and deterministic replay of GX-controlled outputs

It does not own:

- top-level Greentic wizard launching
- pack scaffolding/build/update/sign execution
- bundle normalization/setup execution
- final `.gtbundle` generation semantics

## Current Toolchain Boundary

- `gtc wizard` routes into `greentic-dev`
- `greentic-dev` owns the launcher contract
- `greentic-pack` owns pack execution
- `greentic-bundle` owns bundle execution
- `gx` may emit compatibility/handoff artifacts for those tools

## Practical Guidance

- Keep GX outputs explicit and versioned.
- Treat bundle answers as downstream handoff artifacts.
- Do not add new pack or bundle execution logic to `greentic-x`.
- Prefer compatibility layers over duplicated workflow implementations.
