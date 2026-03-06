# Ops Model

## Overview

Operations describe reusable executable behaviors that sit alongside contracts. An op manifest says:

- what the operation is called
- which input and output schemas it expects
- which contracts and versions it supports
- which permissions/capabilities it expects
- which example payloads describe intended behavior

The op crate models this with `OperationManifest` and related structs.

## Operation Structure

An operation manifest contains:

- `operation_id`
- `version`
- `description`
- `input_schema`
- `output_schema`
- optional compatibility declarations
- optional supported contract declarations
- optional permission requirements
- optional example payloads

## Compatibility and Supported Contracts

There are two related but different ideas:

- compatibility declarations
  These describe schema or compatibility references for the op itself.

- supported contracts
  These declare which contract/version pairs the op expects to work with.

The runtime currently checks that supported contracts are installed before an op can be registered.

## Permission Metadata

Permission requirements are descriptive metadata today. They are not yet enforced by a policy engine.

They exist to make expected capabilities visible early, for example:

- `decision:write`
- `playbook:read`
- `evidence:read`

## Reference Ops

The repo currently includes:

- `approval-basic`
- `playbook-select`
- `rca-basic`

These are intentionally deterministic, generic reference ops. They show extension patterns without pretending to be production integrations.

## Extension Philosophy

Ops should stay:

- generic
- schema-described
- explicit about supported contracts
- small enough to reason about and test

The first cut focuses on descriptors and examples. Future work may add:

- separate execution components
- richer harnesses
- packaging and distribution flows
- tighter runtime integration
