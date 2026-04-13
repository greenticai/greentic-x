# Audit: `greentic-cap`

## Current Responsibilities

- Defines capability data models, resolution types, and compatibility helpers.
- Provides schema types for bundle/setup-oriented capability resolution
  artifacts.
- Encodes pack capability compatibility checking and emitted binding shapes.
- Documents a capability workspace intended for pack, bundle, setup, and
  runtime tooling.

## Current Inputs

- capability declarations
- pack capability sections
- component self-descriptions
- capability resolution reports and bundle metadata

## Current Outputs

- compatibility reports
- emitted binding structures
- bundle/setup-oriented resolution artifacts
- schema-validated capability payloads

## Extension Points

- capability profiles
- capability resolvers
- compatibility checking
- bundle/setup artifact generation from capability resolution reports

## Gaps Vs Desired GX / DW Model

- The types are useful, but current wizard/toolchain flows do not yet appear to
  depend on `greentic-cap` directly.
- There is no formal GX solution-intent to capability-resolution handoff yet.
- A risk for GX is inventing a parallel capability model instead of mapping onto
  `greentic-cap` concepts.

## Implication For `greentic-x`

- Reuse `greentic-cap` concepts where capability requirements need to be
  described.
- Avoid introducing a new capability taxonomy in GX unless it is explicitly a
  compatibility layer with a documented mapping.
