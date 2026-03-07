# PR-GX-04 — Tooling, Designer, Simulator, and Doctor

## Title

Create Greentic-X authoring and validation tooling for contracts, ops, flows,
and profiles

## Goal

Make Greentic-X usable by external teams by providing strong tooling for:

- scaffolding
- validation
- simulation
- inspection

This PR should create the first practical developer experience for building on
GX while reusing the repo’s existing pack/tooling approach wherever possible.

## Alignment With Current Repo

This repo already has:

- contract/op manifests and validation logic
- `greentic-pack`-backed pack scaffolds
- local examples
- CI hooks for pack validation

So this PR should avoid inventing redundant tooling where existing Greentic
tools already provide the right base. A small GX-specific CLI, `cargo xtask`, or
subcommands layered on current tooling are all acceptable.

## Deliverables

### 1. CLI

Create a `gx` CLI, or align with existing repo conventions, with at least:

```text
gx contract new
gx contract validate
gx op new
gx op validate
gx flow new
gx flow validate
gx simulate
gx doctor
gx catalog list
```

### 2. Scaffolding

Scaffold:

- core contract packages
- generic op packages
- resolver packages
- flow/playbook packages
- view templates

Generated scaffolds should be intentionally generic and heavily documented.

Where practical, reuse `greentic-pack` wizard and the repo’s checked-in pack
patterns instead of creating a second scaffolding universe.

### 3. Simulator

Create a local simulation runner that can:

- load contracts
- register stub ops/resolvers
- execute a flow/profile locally
- inspect outputs, evidence, and final views

### 4. Doctor/validation

Add checks for:

- missing required fields
- broken contract references
- invalid operation descriptors
- missing version metadata
- broken flow step references
- inconsistent view refs/evidence refs

## Docs to add

- `docs/tooling-overview.md`
- `docs/how-to-build-a-downstream-solution.md`
- `docs/simulation-workflow.md`

These docs should be written specifically with downstream solution repos in
mind.

## Tests

Add:

- scaffold snapshot tests
- validate/doctor tests
- simulation smoke tests
- broken-package diagnostic tests

## Acceptance criteria

- A downstream team can scaffold a contract, op, and flow
- A flow can be simulated locally
- Doctor catches common structural mistakes
- Tooling docs clearly show how GX is intended to be consumed by another repo

## Codex instruction

Optimize for downstream usability. The success criterion is that a repo like
`zain-x` can be built mostly by filling in adapters, contracts, playbooks, and
views using GX tooling rather than inventing its own structure.
