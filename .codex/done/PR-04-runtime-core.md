# PR-04 Implement Greentic-X Runtime Core

## Depends on

- `PR-01-repo-bootstrap.md`
- `PR-02-types-and-events.md`
- `PR-03-contracts-crate-and-reference-contracts.md`

## Goal

Implement the initial `greentic-x-runtime` crate that provides the generic contract + ops runtime core.

This is the most important implementation PR in the repo.

## Runtime responsibilities

The runtime should provide abstractions and core logic for:

- contract registry
- contract activation
- resource creation
- resource get/list
- resource patch
- resource append
- resource transition
- operation registration
- operation invocation
- audit/event emission hooks
- revision/optimistic concurrency handling
- compatibility checks against declared contracts/ops metadata

## Important architectural constraint

This crate should be a **runtime library/core**, not a hard-coded domain application.

It should not know what a case or playbook is; it should only know:
- contracts
- resources
- schemas
- revisions
- mutation rules
- transitions
- operations
- events/audit hooks

## Storage/integration boundary

The runtime should not assume a specific storage backend.

Design the runtime so storage/state can be provided by adapters or traits consistent with your Greentic style.

Similarly:
- event emission should be abstracted
- policy authorization should be abstracted
- operator/provider concerns should remain outside this crate

## Minimum API surface

Define a clean internal/external API for runtime actions equivalent in spirit to:

- contract install / activate / list / describe
- resource create / get / list / patch / append / transition
- op install / list / describe / call

Refine names based on crate style.

## Work items

### 1. Contract registry core
Install/activate/look up contracts.

### 2. Resource lifecycle engine
Implement generic create/get/list/patch/append/transition logic.

### 3. Revision handling
Support safe mutation with revision checks.

### 4. Event hooks
Emit structured events through an abstraction boundary.

### 5. Validation hooks
Validate requests against contract metadata and schemas.

### 6. Operation registry + invocation abstraction
Register declared operations and invoke through an abstraction/interface.

### 7. Tests
Add strong unit tests around:
- allowed vs denied patch
- append-only behavior
- transitions
- revision conflicts
- missing contract/op
- event emission behavior

## Non-goals

- Greentic operator integration wiring in this PR
- production storage backend selection
- full migration subsystem
- full policy engine implementation

## Success criteria

The runtime crate can execute core resource and op lifecycle rules in memory or through test adapters, proving the model works end-to-end.
