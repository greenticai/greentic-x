# PR-GX-02 — Core Specs and Core Catalog

## Title

Define the core Greentic-X specifications and the smallest reusable catalog
around the six primitives

## Goal

Create the specification layer for Greentic-X and seed the repo with a minimal,
reusable catalog of generic contracts and operations.

This PR should build on the current manifests and runtime models so downstream
repos can rely on stable GX specs without forcing a redesign of existing crates.

## Alignment With Current Repo

The repo already has:

- resource/contract concepts in `greentic-x-contracts`
- operation descriptors in `greentic-x-ops`
- shared identifiers/provenance/revisions in `greentic-x-types`
- runtime lifecycle behavior in `greentic-x-runtime`
- reference contract/op artifacts under `contracts/` and `ops/`

This PR should add a formal `specs/` and `catalog/` layer **on top of those
working concepts**, not beside them.

## Why

Without stable specs, customer/domain repos will drift immediately.

Greentic-X should standardize:

- generic resource envelope
- resolver result envelope
- operation descriptor + call/result envelopes
- flow run/state envelope
- evidence envelope
- view envelope

It should also publish a tiny core catalog proving that these specs are usable
across domains.

## Non-goals

Do **not**:

- add Zain-specific contracts or adapters
- add vendor-specific query ops
- define a broad industry semantic vocabulary
- turn Greentic-X into a telecom toolkit

## Target structure

```text
greentic-x/
├── specs/
│   ├── contracts/
│   │   ├── gx.resource.v1/
│   │   ├── gx.resolver.result.v1/
│   │   ├── gx.operation.descriptor.v1/
│   │   ├── gx.flow.run.v1/
│   │   ├── gx.evidence.v1/
│   │   └── gx.view.v1/
│   └── profiles/
│       └── gx.observability.playbook.v1/
├── catalog/
│   └── core/
│       ├── contracts/
│       ├── ops/
│       ├── resolvers/
│       ├── views/
│       └── flow-templates/
```

These may be represented as checked-in manifests, schemas, examples, and pack
sources. They do not need to be separate Rust crates.

## Main deliverables

### 1. Create the six core spec packages

Each package should include:

- descriptor/manifest
- machine-readable schemas
- human-readable README
- examples

These specs should align with current code where possible.

#### `gx.resource.v1`

Derive from current resource/runtime concepts:

- resource envelope
- resource ref
- labels/metadata
- link structure

#### `gx.resolver.result.v1`

Introduce:

- statuses
- candidate format
- selected result
- ambiguity handling

#### `gx.operation.descriptor.v1`

Generalize the current `OperationManifest` model to include:

- operation descriptor
- compatibility metadata
- input/output contract refs
- permissions/risk declaration
- call/result envelopes

#### `gx.flow.run.v1`

Introduce:

- flow run metadata
- step states
- branch/split/join result structures
- outcome statuses

#### `gx.evidence.v1`

Introduce:

- evidence item
- evidence ref
- lineage/provenance metadata
- attachment/linking semantics

#### `gx.view.v1`

Introduce:

- neutral view model
- summary/table/chart/timeline/card-friendly variants
- hints for channel-specific renderers

### 2. Create a small optional profile

#### `gx.observability.playbook.v1`

This is not a new core primitive. It is a higher-level authoring convenience
profile that should later compile into ordinary GX flows.

### 3. Create the minimal core catalog

#### Core resolvers

- `resolve.by_name`
- `resolve.by_alias`
- `resolve.by_label`

#### Core ops

- `query.resource`
- `query.linked`
- `transform.filter`
- `transform.group_by`
- `analyse.threshold`
- `analyse.percentile`
- `correlate.join`
- `correlate.time_window`
- `present.summary`
- `present.table`
- `present.timeline`

#### Core views

- summary-card
- summary-table
- ranked-list
- timeline
- chart-timeseries

#### Core flow templates

- resolve-query-analyse-present
- resolve-query-correlate-rank-present
- resolve-expand-split-join-synthesise
- resolve-query-evaluate-present

These can begin as descriptors/example manifests before the full executor lands.

## Packaging guidance

Stay aligned with the current pack direction already used in this repo:

- manifests
- JSON schemas
- examples
- optional `packs/` source packs when useful

The priority is the semantic model, not file-format perfection.

## Docs to add

- `docs/specs-overview.md`
- `docs/core-catalog.md`
- `docs/authoring-profile-observability.md`

These should explain:

- the difference between core specs and customer/domain contracts
- why the observability playbook profile is optional
- how customer repos should depend on GX specs without GX depending on them

## Conformance helpers

Add initial validation helpers so later domain/customer repos can validate:

- descriptor shape
- schema compatibility
- catalog entry structure

## Acceptance criteria

- `specs/` exists with the six core spec families
- `catalog/core/` exists with initial generic entries
- the specs clearly map back to current repo concepts instead of contradicting
  them
- docs explain how the catalog/spec layer relates to the existing runtime and
  manifests
