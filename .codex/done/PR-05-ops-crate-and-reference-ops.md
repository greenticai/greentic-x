# PR-05 Implement Ops Crate and Reference Ops

## Depends on

- `PR-01-repo-bootstrap.md`
- `PR-02-types-and-events.md`
- `PR-03-contracts-crate-and-reference-contracts.md`
- `PR-04-runtime-core.md`

## Goal

Create:

1. `crates/greentic-x-ops`
2. initial reference ops under `ops/`

These provide the generic operation model and a starter set of reusable reference operations.

## `greentic-x-ops` responsibilities

Define generic models/helpers for:
- op descriptors/manifests
- input/output schema references
- compatibility declarations
- permission requirements metadata
- invocation metadata
- registration helpers
- testing harness/helpers if useful

Keep this crate generic and reusable.

## Reference ops to add

Start with a small set:

- `playbook-select`
- `rca-basic`
- `approval-basic`

Optionally add a ticket/reference op only if the repo structure supports it cleanly without dragging in external platform coupling yet.

## Guidance for the initial ops

### `playbook-select`
A minimal deterministic example that takes an input/header/context and returns a selected playbook identifier or route.

### `rca-basic`
A minimal reference op that derives a simple RCA-style result from evidence or status snapshots.

### `approval-basic`
A minimal reference op/pattern around proposal/approval transitions or result shaping.

These are examples and should stay generic rather than pretending to be production-complete.

## Work items

### 1. Add op crate
Create descriptors and validation helpers.

### 2. Add reference op directories
Each should include:
- source
- schemas
- metadata/descriptor
- tests/examples
- docs

### 3. Runtime compatibility checks
Integrate with runtime models as needed so ops can declare which contracts/versions they support.

### 4. Tests
Validate registration metadata and sample invocation paths.

## Non-goals

- production integrations like ServiceNow/Jira in first cut
- full WASM packaging pipeline in this PR unless the repo is ready for it

## Success criteria

The repo contains a coherent generic op model plus a few concrete reference ops that demonstrate how the runtime layer is meant to be extended.
