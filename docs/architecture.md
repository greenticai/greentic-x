# Architecture Overview

## Purpose

`greentic-x` is the broader Greentic layer that sits above minimal Greentic core primitives. It is the place for:

- shared object and resource contracts
- runtime lifecycle logic for those contracts
- reusable operation descriptors
- reference contract and op artifacts
- runnable example applications
- architecture and spec notes

Greentic core should stay smaller and more foundational. Greentic-X is where the richer object model and extension story can evolve.

## Main Building Blocks

### Shared types

`crates/greentic-x-types` defines the generic vocabulary used across the repo:

- identifiers
- revisions
- provenance
- schema references
- mutation requests

This crate should remain generic and domain-neutral.

### Events

`crates/greentic-x-events` defines structured event envelopes and payloads for:

- contract installation and activation
- resource creation, patching, append, and transition
- operation installation and execution

These are the canonical runtime event shapes for the repo.

### Contracts

`crates/greentic-x-contracts` defines the contract manifest format and structural validation helpers. The `contracts/` directory contains the local reference contract artifacts that use that format.

At the moment, contract artifacts are local JSON manifests plus schemas and examples. They are not yet packaged as `.gtpack` bundles.

### Runtime

`crates/greentic-x-runtime` provides the lifecycle core:

- contract install / activate
- resource create / get / list / patch / append / transition
- operation registration / invocation
- revision conflict checks
- event emission through an abstract sink

The runtime is intentionally storage-agnostic and uses in-memory adapters for tests and examples.

### Ops

`crates/greentic-x-ops` defines operation manifests, supported-contract declarations, permission requirements, and example payloads. The `ops/` directory contains the local reference operation artifacts.

At the moment, the repo models and validates op descriptors, but the reference ops are not yet separate executable components or packaged bundles.

### Examples

The `examples/` workspace members show how contracts, runtime, and ops fit together:

- `simple-case-app`
- `simple-playbook-app`
- `end-to-end-demo`

These examples are deterministic and local-only. They are intended to explain the model, not to represent production integrations.

## Repository Boundaries

What belongs here:

- shared Greentic-X crates and models
- reference contracts and ops
- runtime lifecycle logic
- example apps and supporting docs

What does not belong in minimal Greentic core:

- richer contract libraries
- runtime orchestration for those contracts
- reusable operational workflows and example apps
- most extended documentation for the Greentic-X model

## Current State

The repo now has real first-cut implementations for:

- shared types
- events
- contracts
- runtime
- ops
- examples

The main remaining gaps are:

- deeper schema validation
- policy and migration integration
- pack generation / wizard integration
- richer documentation and governance details
