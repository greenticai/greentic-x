# PR-02 Implement Core Shared Types and Events

## Depends on

- `PR-01-repo-bootstrap.md`

## Goal

Create the first reusable shared crates needed by the rest of the Greentic-X repo:

- `greentic-x-types`
- `greentic-x-events`

These should contain the canonical identifiers, envelopes, requests, revisions, and event models used by runtime, contracts, ops, and examples.

## Why first

This reduces duplication and forces an early alignment on the core language of the repo.

## `greentic-x-types` scope

Define canonical generic types such as:

- contract identifiers
- contract versions
- resource type identifiers
- resource IDs
- revisions
- patch/append/transition requests
- operation identifiers
- schema references / compatibility declarations
- actor / provenance / audit metadata envelopes where appropriate

Keep these generic and platform-neutral where possible.

## `greentic-x-events` scope

Define structured event models such as:

- `ResourceCreated`
- `ResourcePatched`
- `ResourceAppended`
- `ResourceTransitioned`
- `OperationInstalled`
- `OperationExecuted`
- `ContractInstalled`
- `ContractActivated`

Include:
- event envelope shape
- payload structs
- serialization support aligned with repo conventions

## Work items

### 1. Add crate skeletons
Create both crates with tests and docs.

### 2. Define stable type vocabulary
Prefer a small clean initial vocabulary over too many speculative types.

### 3. Add serialization/tests
Use your normal repo conventions.

### 4. Add examples/docs
Show sample serialized event/request payloads.

## Constraints

- keep types generic
- avoid embedding telecom/finance/health-specific concepts
- avoid premature runtime policy logic in this PR

## Success criteria

Other crates can depend on these types/events without re-defining their own model vocabulary.
