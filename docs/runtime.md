# Runtime Model

## Overview

The runtime crate provides the lifecycle engine for Greentic-X resources and operations.

Its current responsibilities are:

- install and activate contracts
- create, read, list, patch, append, and transition resources
- register and invoke operations
- enforce optimistic concurrency with revisions
- emit lifecycle events through an abstract event sink

## Resource Lifecycle

### Create

Resource creation requires:

- an active contract
- a matching resource definition in that contract
- a JSON object document
- a unique resource ID

Created resources start at revision `1`.

### Patch

Patch requests are checked against the active contract’s patch rules.

The runtime currently enforces:

- allowed vs denied paths
- revision match
- JSON pointer style patch application

### Append

Append requests are checked against declared append-only collections.

The runtime ensures:

- the collection exists in the contract metadata
- the target document is an object
- the append target is an array or can be initialized as one

### Transition

Transitions require:

- a string `state` field in the resource document
- a declared `from_state -> to_state` pair in the active contract
- a matching base revision

## Revisions

`Revision` is used for optimistic concurrency. Mutating operations compare the request’s base revision with the stored revision.

If they differ, the runtime returns a `RevisionConflict`.

## Events

The runtime emits typed events for:

- contract installed
- contract activated
- resource created
- resource patched
- resource appended
- resource transitioned
- operation installed
- operation executed

Event emission is abstracted behind an `EventSink` trait so the runtime does not depend on a specific transport.

## Storage Boundary

The runtime depends on a `ResourceStore` trait rather than a specific backend.

The repo currently includes an in-memory store for:

- unit tests
- examples
- proving the model works without external dependencies

This keeps the runtime core independent from future storage decisions.

## Operation Boundary

Operation execution is abstracted behind an `OperationHandler` trait.

The runtime:

- registers validated op manifests
- checks supported contracts
- invokes handlers with JSON input
- emits operation execution events

This is deliberately lightweight for now. The runtime does not yet include a richer execution harness or external provider integration.

## Current Limitation

The runtime uses contract and op metadata structurally, but it does not yet:

- execute JSON Schema validation
- enforce policy hooks
- execute migrations
- integrate with production storage or transport backends
