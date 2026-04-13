# GX To Pack Flow

## Purpose

This note describes the current compatibility boundary between `gx` and
`greentic-pack`.

## Ownership

- `gx` composes solution intent and emits compatibility artifacts.
- `greentic-pack` owns pack scaffolding, pack state, doctor, resolve, build,
  sign, and any `pack.yaml`-driven execution.

## Current GX Output

`gx wizard` now emits `<solution-id>.pack.input.json`.

This file is a pack-oriented compatibility document. It is not a pack manifest,
not a `pack.yaml`, and not a replacement for `greentic-pack` wizard answers.

## What GX Maps Today

- provider references derived from selected provider presets
- required capability offers derived from `solution_intent.required_capabilities`
- required contracts derived from `solution_intent.required_contracts`
- suggested flows carried through from solution intent
- template selections and defaults
- provider hints from the selected preset entries

## What Stays Unresolved

The pack input explicitly leaves these concerns to `greentic-pack`:

- choosing or scaffolding the concrete pack root
- turning compatibility input into pack wizard answers or `pack.yaml` state
- capability offer editing and extension/component authoring
- doctor, resolve, build, sign, and manifest synchronization

## `greentic-cap` Mapping

GX does not introduce a second capability model here.

Instead, `required_capabilities` from solution intent are carried into the pack
input as `required_capability_offers`, with an explicit
`greentic_cap_mapping` section that marks this as a partial compatibility
mapping onto `greentic-cap` concepts.
