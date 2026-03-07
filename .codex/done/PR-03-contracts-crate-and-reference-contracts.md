# PR-03 Implement Contracts Crate and Reference Contract Artifacts

## Depends on

- `PR-01-repo-bootstrap.md`
- `PR-02-types-and-events.md`

## Goal

Create:

1. `crates/greentic-x-contracts`
2. initial reference contract artifacts under `contracts/`

This PR establishes how contracts are represented and validated at a repo level, without yet building the full runtime.

## `greentic-x-contracts` responsibilities

The crate should define generic models and helpers for:

- contract manifest/descriptor
- resource schema references
- mutation rule declarations
- append-only collection declarations
- transition declarations
- optional policy hook references
- version compatibility / migration references
- validation helpers for contract definitions

Do not hard-code specific industry semantics.

## Reference contracts to add

Create initial minimal reference contracts for:

- case
- evidence
- outcome
- playbook

These should be intentionally minimal and generic.

### Contract content expectations

Each reference contract directory should include, as appropriate:

- manifest/descriptor
- resource schema(s)
- mutation rule definitions
- transitions
- event declarations
- examples
- README/documentation

Use exact file names/formats that fit the repo conventions you establish in this PR.

## Design guidance

### Minimal and generic
For example:
- `case` should model a shared operational case record, not telecom-specific incident semantics
- `evidence` should model appendable evidence references/results generically
- `outcome` should model proposed/approved/executed outcomes generically
- `playbook` should model reusable workflow definitions/runs generically

### Versioning
Include a clear versioning approach from the start, even if only `v1` is implemented now.

### Validation
Add tests that ensure the reference contracts are structurally valid according to the crate models/helpers.

## Non-goals

- full runtime enforcement
- migration execution engine
- policy engine execution

## Success criteria

The repo has a coherent contract model and a small standard library of reference contracts ready for runtime integration.
