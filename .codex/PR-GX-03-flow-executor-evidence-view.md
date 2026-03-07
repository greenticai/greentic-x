# PR-GX-03 — Flow Executor, Evidence, and View Rendering

## Title

Implement Greentic-X execution semantics for sequential and parallel flows, plus
evidence capture and neutral view rendering

## Goal

Turn Greentic-X from a runtime/spec shell into an executable system that can run
real playbooks composed of resolvers and operations, capture evidence, and
render neutral views.

This PR should focus on:

- flow execution
- split/join semantics
- evidence production/linking
- view rendering/model transformation

## Alignment With Current Repo

The repo already has a working imperative runtime plus Rust examples that
manually orchestrate contracts/resources/ops. This PR should add a real flow
execution layer on top of that runtime rather than replacing the existing
resource/op model.

Acceptable implementation shapes:

- extend `greentic-x-runtime` with flow modules, or
- add focused crates such as `greentic-x-flow`, `greentic-x-evidence`, or
  `greentic-x-view` if separation becomes justified

What matters is the execution boundary, not the crate names.

## Why

The workshop-style use cases consistently follow orchestrated multi-step
patterns:

- resolve
- query
- analyse/correlate/evaluate
- present

The more advanced cases add:

- scope expansion
- parallel branches
- join
- synthesis

Greentic-X needs a reusable executor to support that generically.

## Non-goals

Do **not**:

- add network-specific playbooks here
- hardcode customer logic
- overbuild a full workflow language beyond what the six primitives need

## Main deliverables

### 1. Create/complete a GX flow executor

Responsibilities:

- execute flow steps in order
- pass context/envelopes between steps
- support:
  - `call`
  - `map`
  - `branch`
  - `split`
  - `join`
  - `return`
- support timeout and partial-join behavior in a minimal but explicit way

### 2. Create/complete evidence storage/linking

Responsibilities:

- persist evidence items and refs
- link evidence to:
  - flow runs
  - resources
  - operation calls
- retain provenance metadata
- expose listing/retrieval APIs

### 3. Create/complete neutral view rendering

Responsibilities:

- render `gx.view.v1`
- convert evidence/result payloads into:
  - summary
  - table
  - timeline
  - chart-ready series
  - generic card-like model
- keep rendering neutral; customer-specific adaptive cards stay outside GX

## Detailed flow semantics

### Step kinds

#### `call`

Invoke:

- resolver
- operation
- optionally a nested/template flow later

#### `map`

Transform context fields into the next step input.

#### `branch`

Choose next path based on structured outputs/status.

#### `split`

Start two or more child branches with isolated branch contexts.

#### `join`

Wait for:

- all
- any
- quorum

or a timeout, depending on declared mode.

#### `return`

Emit final outputs/evidence/view refs.

## State model

A flow run should capture:

- run id
- input envelope
- current state/context
- step statuses
- branch statuses
- evidence refs
- final result status
- final view ref(s)

A simple persisted run model is enough for this PR.

## Parallel branch guidance

Support at least:

- `join_all`
- `join_any`
- `join_all_or_timeout`

On merge:

- preserve branch-local outputs under distinct namespaces
- avoid implicit key collisions
- surface timeout/partial data in status/warnings

## Evidence behavior

Operations should be able to emit evidence refs directly in their output
envelopes.

The executor should:

- gather these refs
- add lineage back to the step/run
- make them available to later steps
- optionally create synthetic evidence items for run-level summaries

## View behavior

Rendering should be a separate capability from analysis.

Support rendering from:

- raw outputs
- evidence refs
- aggregated evidence collections

Neutral view families:

- summary
- table
- timeline
- chart-series
- card-model

## Docs to add

- `docs/flow-executor.md`
- `docs/parallelism-and-join-semantics.md`
- `docs/evidence-and-view-separation.md`

## Tests

Add tests for:

- sequential flow success
- branch selection
- split + join_all
- split + timeout
- evidence propagation across steps
- final view generation from evidence/results
- partial-join warning handling

Use fake ops, fake resolvers, and fake renderers where necessary.

## Acceptance criteria

- A flow with resolve → query → analyse → present can run end-to-end
- A split/join flow can run with deterministic merge behavior
- Evidence emitted by ops is stored and linked to the run
- A neutral view can be produced from final outputs/evidence
- The current imperative Rust examples can later be restated in terms of the new
  executor model
