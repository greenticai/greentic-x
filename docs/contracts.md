# Contract Model

## Overview

Contracts describe the resource model that the runtime enforces. A contract says:

- which resources exist
- which schema reference each resource uses
- which fields are patchable
- which collections are append-only
- which state transitions are allowed
- which lifecycle events are relevant

The contract crate models this with `ContractManifest` and related structs.

## Contract Structure

A contract manifest contains:

- `contract_id`
- `version`
- `description`
- `resources`
- optional compatibility declarations
- optional event declarations
- optional policy hook reference
- optional migration references

Each resource definition contains:

- `resource_type`
- `schema`
- `patch_rules`
- `append_collections`
- `transitions`

## Mutation Rules

Patch behavior is described through `MutationRule` entries.

- `allow` means the path can be patched
- `deny` means the path is explicitly blocked

The current runtime uses these rules structurally. It does not yet perform deeper JSON Schema-driven field validation.

## Append-Only Collections

Append-only collections are declared explicitly with `AppendCollectionDefinition`.

This is used for cases like:

- evidence references on a case
- observations on an evidence record
- step results on a playbook run
- actions on an outcome

The runtime checks that only declared collections can be appended to.

## Transitions

State transitions are declared as `from_state -> to_state` pairs.

The runtime uses these to enforce lifecycle progression. For example:

- case: `new -> triaged -> investigating -> resolved -> closed`
- playbook-run: `pending -> running -> completed|failed`
- outcome: `proposed -> approved -> executed`

## Versioning and Migration Intent

Contracts carry a `version` and optional `migration_from` references.

The current repo only models migration intent. It does not yet execute migrations. For now, versioning is primarily declarative and used to make compatibility expectations explicit.

## Reference Contracts

The repo currently includes:

- `gx.case`
- `gx.evidence`
- `gx.outcome`
- `gx.playbook`

These are intentionally generic reference contracts, not domain-specific production models.

The same reference material is also mirrored into the source pack at
`packs/greentic-x-contracts-reference/assets/contracts/` so it can be bundled
with `greentic-pack`.

## Current Limitation

Contract validation is structural in the contracts crate, but the runtime now
supports JSON Schema enforcement when the referenced schemas are registered.

What is still future work:

- policy hook execution
- migration execution
- broader schema-registration automation for non-local backends

The current contract pack scaffold is useful for packaging and distribution, but
its generated `contract-hook` component is still only a placeholder.
