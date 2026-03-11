# PR-GX-01 — Runtime Foundations for Greentic-X

## Title

Extend the current Greentic-X workspace into a fuller open-source, industry-agnostic Contract + Ops runtime over Greentic

## Goal

Build the missing runtime foundations so `greentic-x` can host:

- generic resources
- resolvers
- operations
- flows
- evidence
- views

without encoding customer-, industry-, or vendor-specific semantics.

This PR should establish the missing runtime boundaries while **building on the
existing `greentic-x-*` crates** rather than replacing them.

## Alignment With Current Repo

The repo already has:

- `greentic-x-types`
- `greentic-x-events`
- `greentic-x-contracts`
- `greentic-x-ops`
- `greentic-x-runtime`
- reference `contracts/`, `ops/`, `examples/`, `docs/`, and `packs/`

This PR should treat those as the baseline. The objective is to add the missing
runtime capabilities, not to rename the repo into a different crate topology.

If later extraction becomes justified, modules can be split into dedicated
crates, but this PR should prefer additive evolution inside the current
workspace.

## Why

We want `greentic-x` to become the reusable framework that sits above Greentic
core and below customer/domain implementations such as `zain-x`.

Greentic-X should provide the reusable machinery, while customer layers provide:

- domain contracts
- adapters
- analysis ops
- playbooks
- UI/card renderers

This PR extends the current runtime shell toward that model.

## Non-goals

Do **not** in this PR:

- add Zain-specific terms
- add Arbor/APIC/EPNM/Intersight/Splunk-specific knowledge
- add concrete network contracts
- add telecom-specific use cases
- over-design governance or marketplace flows
- rewrite the current crate layout purely to match aspirational names

## Expected repo structure after this PR

Keep the current workspace and extend it logically:

```text
greentic-x/
├── crates/
│   ├── greentic-x-types/
│   ├── greentic-x-events/
│   ├── greentic-x-contracts/
│   ├── greentic-x-ops/
│   └── greentic-x-runtime/
├── specs/          # may start in PR-GX-02
├── catalog/        # may start in PR-GX-02
├── examples/
├── tools/          # optional; may begin in later GX PRs
├── packs/
└── docs/
```

Logical responsibilities that earlier drafts described as separate crates may be
implemented initially as modules inside the existing crates.

## Main deliverables

### 1. Extend the base runtime capability areas

#### `greentic-x-types`
Add shared models/utilities for:

- ids/refs not yet modeled
- labels/metadata helpers
- timestamps/version helpers where needed
- resolver envelopes
- operation call/result envelopes
- flow/evidence/view references if required by follow-on GX PRs

#### `greentic-x-contracts`
Continue to own:

- installable contract descriptors
- validation of descriptor metadata
- contract compatibility declarations

Keep registry logic in the runtime for now unless a clean extraction becomes
necessary.

#### `greentic-x-runtime`
Extend responsibilities to cover:

- contract install/register/activate/list/look-up
- resource lifecycle and query helpers
- typed links between resources
- resolver registration/invocation
- operation registration/invocation
- audit/event hooks

If this becomes too large, split by modules first and extract crates only after
interfaces stabilize.

#### `greentic-x-ops`
Continue to own:

- operation descriptors
- compatibility metadata
- permission/risk metadata
- operation input/output contract references when available

### 2. Define the runtime API surface (internal first)

At this PR stage, the transport can remain internal Rust API first, with
CLI/API/WASM integration following later.

The runtime façade should grow toward operations such as:

```rust
install_contract(...)
activate_contract(...)
register_resolver(...)
resolve(...)
register_op(...)
call_op(...)
create_resource(...)
get_resource(...)
query_resources(...)
upsert_link(...)
list_links(...)
```

Exact method names can follow existing repo conventions.

### 3. Standardize envelopes at runtime level

Create or extend shared envelope structs in `greentic-x-types`:

- operation input envelope
- operation output envelope
- resolver input envelope
- resolver output envelope

These should be coherent with the existing manifest and runtime model rather
than invented as a parallel abstraction.

### 4. Add runtime-level audit hooks

Even if the implementation is minimal, add extension points now:

- contract install/update audit
- resource create/update/link audit
- resolver invocation audit
- operation invocation audit

Do not overbuild. Event structs / trait hooks / telemetry interfaces are enough.

### 5. Add docs

Create or update:

- `docs/runtime-overview.md`
- `docs/runtime-boundary.md`
- `docs/why-six-primitives.md`

If the current docs already cover some of this material, expand them rather than
creating near-duplicates.

## Detailed design constraints

### Keep Greentic-X generic

The runtime must not hardcode:

- network
- telecom
- SRE
- finance
- healthcare

### Graph is data, not a core runtime primitive

A topology graph should be representable as:

- resources
- typed links
- traversal/query ops

Do not create a separate graph subsystem unless required by proven constraints.

### Resolver is a first-class concept at Greentic-X level

Not because Greentic core needs it, but because many user-facing questions
start with name/prefix/service resolution. Treat it as reusable and standardized.

### Operation is the main generic execution unit

Most domain work should land here, not in special-case engines.

## Tests

Add tests for:

- contract registration/activation
- resource query and link behavior
- resolver registration/invocation
- operation call envelopes
- audit hook/event emission integration

## Acceptance criteria

- The existing runtime crate cleanly hosts contract/resource/op behavior and the
  newly added resolver/link capabilities
- Shared call/result envelopes exist and are reusable
- Runtime audit hooks are explicit
- Docs explain the current runtime boundary in terms of the existing workspace
