# Repository Overview

## 1. High-Level Purpose

This repository is the Greentic-X workspace that sits above narrow Greentic
core primitives. It currently contains shared Rust crates for Greentic-X types,
events, contracts, runtime behavior, and ops metadata; local reference
artifacts for contracts and ops; runnable example applications; first-cut
architecture/governance docs; and wizard-backed `greentic-pack` source packs for
packaging those artifacts.

The repo’s main responsibilities are to define a reusable Greentic-X model,
provide a storage-agnostic runtime core, carry reference artifacts that explain
the model concretely, and expose both an initial `.gtpack` packaging path and a
first executable GX toolchain. The runtime foundation includes standardized
operation/resolver envelopes, typed resource links, resolver
registration/invocation, and explicit runtime-boundary docs aligned with
`PR-GX-01`. The repo also now contains a formal GX standards layer under
`specs/` and `catalog/`, `greentic-x-flow` for flow execution/evidence/views,
and a `gx` CLI for scaffolding, validation, simulation, doctor checks, catalog
inspection, and observability-profile compilation. The repo also now ships
checked-in flow/profile-driven reference example packages aligned with
`PR-GX-05`.

## 2. Main Components and Functionality

- **Path:** `Cargo.toml`
  - **Role:** Root Cargo workspace manifest.
  - **Key functionality:**
    - Defines the seven core crates and three example application crates.
    - Stores shared workspace package metadata, lints, and the coordinated version `0.4.0`.
  - **Key dependencies / integration points:**
    - Used by `cargo metadata`, `ci/local_check.sh`, and release-version checks.

- **Path:** `crates/greentic-x-types`
  - **Role:** Shared type vocabulary crate.
  - **Key functionality:**
    - Defines validated identifiers, revisions, provenance, schema references, compatibility references, resource refs, typed links, and mutation request types.
    - Defines standardized operation call/result envelopes plus resolver descriptor/query/result envelopes.
    - Supports `serde`/`serde_json` and includes unit tests.
  - **Key dependencies / integration points:**
    - Used by the events, contracts, runtime, and ops crates as the main shared vocabulary layer.

- **Path:** `crates/greentic-x-events`
  - **Role:** Structured event model crate.
  - **Key functionality:**
    - Defines typed event envelopes and payloads for contract, resource, operation, resolver, and resource-link lifecycle events.
    - Supports serialization and includes unit tests.
  - **Key dependencies / integration points:**
    - Used by `greentic-x-runtime` for emitted runtime audit events.

- **Path:** `crates/greentic-x-contracts`
  - **Role:** Contract manifest and validation crate.
  - **Key functionality:**
    - Defines `ContractManifest`, resource definitions, mutation rules, append collections, transitions, policy hook references, migration references, and validation issues.
    - Tests parse and validate the reference contracts under `contracts/`.
  - **Key dependencies / integration points:**
    - Used directly by `greentic-x-runtime`.
    - The same artifacts are mirrored into `packs/greentic-x-contracts-reference/assets/contracts/`.

- **Path:** `crates/greentic-x-runtime`
  - **Role:** Storage-agnostic runtime core.
  - **Key functionality:**
    - Implements contract installation/activation, resource create/get/list/patch/append/transition, typed link upsert/list, operation installation/invocation, resolver installation/invocation, revision conflict handling, and event emission.
    - Supports JSON Schema registration from local contract/op artifact directories and enforces registered input/output/resource schemas during runtime operations.
    - Provides in-memory store/event sink adapters for tests and examples.
  - **Key dependencies / integration points:**
    - Depends on `greentic-x-types`, `greentic-x-events`, `greentic-x-contracts`, and `greentic-x-ops`.
    - The runtime-capability reference pack mirrors the intended extension-pack shape for future runtime integrations.
    - Serves as the current composition façade for the aligned `PR-GX-01` runtime foundation.

- **Path:** `crates/greentic-x-flow`
  - **Role:** Flow execution, evidence capture, and neutral view rendering crate.
  - **Key functionality:**
    - Defines `FlowDefinition`, `FlowEngine`, step kinds for resolve/call/map/branch/split/join/return, and `FlowRunRecord`.
    - Provides `EvidenceStore` and `ViewRenderer` traits with in-memory/no-op helpers for tests and examples.
    - Includes a `RuntimeFlowAdapter` that invokes current runtime resolvers and operations through standardized envelopes.
    - Covers branch selection, split/join handling, timeout-tolerant joins, evidence propagation, and view generation in tests.
  - **Key dependencies / integration points:**
    - Builds directly on `greentic-x-runtime`, `greentic-x-types`, and `greentic-x-ops`.
    - Aligns the checked-in `gx.flow.run.v1`, `gx.evidence.v1`, and `gx.view.v1` specs with executable code.

- **Path:** `crates/greentic-x-ops`
  - **Role:** Operation manifest and validation crate.
  - **Key functionality:**
    - Defines `OperationManifest`, supported-contract declarations, permission requirements, example payloads, and validation issues.
    - Tests parse and validate the reference ops under `ops/`.
  - **Key dependencies / integration points:**
    - Used by `greentic-x-runtime` for op registration compatibility checks.
    - The same artifacts are mirrored into `packs/greentic-x-ops-reference/assets/ops/`.

- **Path:** `crates/gx`
  - **Role:** Downstream-facing GX CLI tooling crate.
  - **Key functionality:**
    - Provides `gx contract new|validate`, `gx op new|validate`, `gx flow new|validate`, `gx resolver new|validate`, `gx view new|validate`, `gx profile validate|compile`, `gx simulate`, `gx doctor`, and `gx catalog list`.
    - Scaffolds generic contract/op/flow/resolver/view package directories with documented starter files.
    - Validates contract, op, flow, resolver, view, and profile packages; compiles `gx.observability.playbook.v1` profiles into normal GX flows; simulates flow packages against stubbed resolver/op data; and runs repo-level doctor checks against contracts, resolvers, ops, views, flows, profiles, and catalog entries.
  - **Key dependencies / integration points:**
    - Reuses `greentic-x-contracts`, `greentic-x-ops`, `greentic-x-flow`, and the checked-in `catalog/` and `specs/profiles/` data.
    - Invoked directly by `ci/local_check.sh` via `cargo run -p gx -- doctor .`.

- **Path:** `contracts/`
  - **Role:** Reference contract artifact area.
  - **Key functionality:**
    - Contains concrete reference contracts for `case`, `evidence`, `outcome`, and `playbook`.
    - Each contract includes `contract.json`, JSON Schemas, examples, and a README.
  - **Key dependencies / integration points:**
    - Parsed by `greentic-x-contracts` tests.
    - Mirrored into the contracts reference source pack under `packs/`.

- **Path:** `ops/`
  - **Role:** Reference operation artifact area.
  - **Key functionality:**
    - Contains concrete reference ops for `approval-basic`, `playbook-select`, and `rca-basic`.
    - Each op includes `op.json`, input/output schemas, examples, `source.md`, and a README.
  - **Key dependencies / integration points:**
    - Parsed by `greentic-x-ops` tests.
    - Mirrored into the ops reference source pack under `packs/`.

- **Path:** `specs/`
  - **Role:** Formal Greentic-X standards layer.
  - **Key functionality:**
    - Defines checked-in spec packages for `gx.resource.v1`, `gx.resolver.result.v1`, `gx.operation.descriptor.v1`, `gx.flow.run.v1`, `gx.evidence.v1`, `gx.view.v1`, and `gx.observability.playbook.v1`.
    - Each spec package includes a manifest, JSON Schema, README, and example payloads.
  - **Key dependencies / integration points:**
    - The resource, resolver, and operation specs map directly to concepts already implemented in the current crates.
    - The flow/evidence/view specs now have executable counterparts in `greentic-x-flow`, and the observability profile spec now has a compile path in `gx`.

- **Path:** `catalog/`
  - **Role:** Minimal reusable GX catalog.
  - **Key functionality:**
    - Provides small checked-in catalog indices for core contract/spec references, resolvers, ops, views, and flow templates.
    - Defines a generic baseline vocabulary without introducing customer- or industry-specific terms.
  - **Key dependencies / integration points:**
    - References the `specs/` layer and documents the intended GX reusable baseline for downstream repos.

- **Path:** `packs/`
  - **Role:** Wizard-backed `greentic-pack` source packs for packaging reference assets.
  - **Key functionality:**
    - `greentic-x-contracts-reference` packages the current contract artifacts under `assets/contracts/`.
    - `greentic-x-ops-reference` packages the current op artifacts under `assets/ops/`.
    - `greentic-x-runtime-capability-reference` packages a runtime-capability scaffold with example schemas and an illustrative `greentic.ext.capabilities.v1` offer.
    - `packs/_wizard/*.answers.json` stores replayable wizard answer documents used to scaffold the pack directories.
  - **Key dependencies / integration points:**
    - Built and inspected by `ci/check_packs.sh` when `greentic-pack` is available.
    - The contract and ops packs mirror repo-level artifacts rather than replacing them.

- **Path:** `examples/`
  - **Role:** Runnable example application area.
  - **Key functionality:**
    - Contains Rust smoke apps `simple-case-app`, `simple-playbook-app`, and `end-to-end-demo`.
    - Also contains flow/profile-driven reference packages `top-contributors-generic`, `entity-utilisation-generic`, `change-correlation-generic`, and `root-cause-split-join-generic`.
    - The generic example packages include input, stub data, expected evidence/view outputs, and optional `profile.json` source material.
  - **Key dependencies / integration points:**
    - Demonstrates both direct runtime usage and the intended GX flow/profile authoring path.

- **Path:** `docs/`
  - **Role:** Architecture, model, governance, and examples documentation set.
  - **Key functionality:**
    - Documents the current contract, ops, runtime, governance, and example model.
    - Now also explains the first-cut pack scaffolding story.
    - Includes `runtime-overview.md`, `runtime-boundary.md`, and `why-six-primitives.md` to describe the current GX runtime foundation.
    - Includes `specs-overview.md`, `core-catalog.md`, and `authoring-profile-observability.md` to explain the new GX standards layer.
    - Includes `flow-executor.md`, `parallelism-and-join-semantics.md`, and `evidence-and-view-separation.md` to describe the current GX execution layer.
    - Includes `tooling-overview.md`, `how-to-build-a-downstream-solution.md`, and `simulation-workflow.md` for the new downstream CLI workflow.
    - Includes `reference-examples.md` and `observability-profile-vs-raw-flows.md` for the new example/profile guidance.
  - **Key dependencies / integration points:**
    - Complements the code and reduces reliance on the older planning notes.

- **Path:** `.codex/PR-GX-01-runtime-foundations.md` through `.codex/PR-GX-05-observability-profile-and-reference-examples.md`
  - **Role:** Aligned forward-plan documents for the next Greentic-X implementation phase.
  - **Key functionality:**
    - Define the intended GX runtime/spec/executor/tooling/profile roadmap.
    - Were updated to target the current `greentic-x-*` workspace and pack/tooling baseline rather than a parallel crate rewrite.
  - **Key dependencies / integration points:**
    - These docs now describe how future GX work should extend the existing repo instead of replacing it.

- **Path:** `ci/local_check.sh`
  - **Role:** Local developer CI wrapper.
  - **Key functionality:**
    - Runs `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build`, `cargo doc`, and crates.io dry-run packaging checks.
    - Runs `cargo run -p gx -- doctor .` so repo-level GX authoring diagnostics stay enforced.
    - Runs `bash ci/check_packs.sh` when `greentic-pack` is installed and `packs/` exists.
  - **Key dependencies / integration points:**
    - Uses `ci/publishable_crates.py` and optionally `ci/check_packs.sh`.

- **Path:** `ci/check_packs.sh`
  - **Role:** Local pack validation helper.
  - **Key functionality:**
    - Finds each source pack under `packs/`.
    - Runs `greentic-pack build` and `greentic-pack doctor` for each pack.
  - **Key dependencies / integration points:**
    - Depends on the local `greentic-pack` CLI.
    - Invoked from `ci/local_check.sh` when available.

- **Path:** `ci/check_specs_catalog.py`
  - **Role:** Spec/catalog conformance helper.
  - **Key functionality:**
    - Validates that required GX spec directories exist and that each one includes manifest, schema, and example files.
    - Validates that the core catalog indices exist and contain entries.
  - **Key dependencies / integration points:**
    - Invoked from `ci/local_check.sh` when `specs/` and `catalog/` are present.

- **Path:** `.github/workflows/ci.yml` and `.github/workflows/publish.yml`
  - **Role:** CI and release automation.
  - **Key functionality:**
    - `ci.yml` runs lint, test, build/doc, and local check steps on pushes and PRs.
    - `publish.yml` verifies the version tag, runs local checks, and publishes crates when any become publishable.
  - **Key dependencies / integration points:**
    - Both workflows now install `greentic-pack` via cached `cargo-binstall` bootstrap steps before running `ci/local_check.sh`.

## 3. Work In Progress, TODOs, and Stubs

- **Location:** `crates/greentic-x-contracts/src/lib.rs`
  - **Status:** partial
  - **Short description:** Contract manifest models and structural validation exist, and the runtime can now enforce referenced JSON Schemas once registered, but migration execution and policy execution are still not implemented.

- **Location:** `crates/greentic-x-runtime/src/lib.rs`
  - **Status:** partial
  - **Short description:** The runtime now implements lifecycle logic, in-memory adapters, typed links, resolver/operation invocation, event emission, and optional registered-schema validation, but it still lacks policy hooks, migrations, durable backend integrations, and automatic schema distribution.

- **Location:** `crates/greentic-x-flow/src/lib.rs`
  - **Status:** partial
  - **Short description:** The flow layer now implements resolve/call/map/branch/split/join/return execution with evidence and view hooks, but it remains in-process and deterministic, with simple branch predicates and no external scheduler or durable run store yet.

- **Location:** `crates/gx/src/lib.rs`
  - **Status:** partial
  - **Short description:** The `gx` CLI now covers core scaffolding, validation, profile compilation, simulation, and doctor flows for contracts, ops, flows, resolvers, and views, but it is still CLI-only and has no separate visual designer.

- **Location:** `crates/greentic-x-types/src/lib.rs`
  - **Status:** partial
  - **Short description:** Shared operation/resolver envelopes and typed link/resource-ref models now exist, but some flow/evidence/view concepts still live partly as crate-local execution models and partly in the checked-in GX specs rather than one consolidated shared vocabulary crate.

- **Location:** `specs/contracts/gx.flow.run.v1/`, `specs/contracts/gx.evidence.v1/`, `specs/contracts/gx.view.v1/`, `specs/profiles/gx.observability.playbook.v1/`
  - **Status:** partial
  - **Short description:** These specs now exist as formal manifests/schemas/examples, `greentic-x-flow` covers the first executor/evidence/view implementation, and `gx` now compiles the observability profile, but broader spec-driven tooling is still pending.

- **Location:** `catalog/core/`
  - **Status:** partial
  - **Short description:** The core catalog now exists and is internally validated, but its entries are standards-first descriptors rather than executable catalog-backed components or flows.

- **Location:** `crates/greentic-x-ops/src/lib.rs`
  - **Status:** partial
  - **Short description:** Operation metadata and compatibility checks exist, but there is still no dedicated execution harness or real component-backed provider implementation.

- **Location:** `packs/greentic-x-contracts-reference/components/contract-hook/`
  - **Status:** stub
  - **Short description:** The generated component bundle is only a wizard scaffold so the contract pack remains structurally valid; it is not a real hook provider.

- **Location:** `packs/greentic-x-ops-reference/components/ops-provider/`
  - **Status:** stub
  - **Short description:** The generated ops provider component is placeholder-only; the useful content is the mirrored op metadata under `assets/ops/`.

- **Location:** `packs/greentic-x-runtime-capability-reference/components/runtime-provider/`
  - **Status:** stub
  - **Short description:** The runtime provider component and capability offer are illustrative scaffolds, not a production runtime extension.

- **Location:** `examples/simple-case-app/`, `examples/simple-playbook-app/`, `examples/end-to-end-demo/`, `examples/*-generic/`
  - **Status:** partial
  - **Short description:** Runnable smoke apps and flow/profile-driven reference examples now exist and work, but they remain local deterministic examples rather than production integrations or full downstream repos.

- **Location:** `docs/`
  - **Status:** partial
  - **Short description:** Documentation now covers the main model and packaging direction, but it is still a first-cut reference set rather than a final specification.

- **Location:** `.codex/PR-GX-01-runtime-foundations.md` through `.codex/PR-GX-05-observability-profile-and-reference-examples.md`
  - **Status:** partial
  - **Short description:** The GX PR documents are now aligned to the current workspace and preserve the intended target capabilities, but later tooling/profile/example polish steps are still unimplemented.

## 4. Broken, Failing, or Conflicting Areas

- **Location:** `crates/greentic-x-runtime/src/lib.rs`
  - **Evidence:** The runtime uses contract and op metadata structurally but does not validate resource or op payloads against the referenced JSON Schemas.
  - **Likely cause / nature of issue:** The current implementation prioritizes lifecycle flow and compatibility checks over deep schema enforcement.

- **Location:** GX runtime scope vs. current implementation
  - **Evidence:** `greentic-x-flow` now provides a dedicated flow executor, evidence store abstraction, and neutral view renderer abstraction, but they are still in-process components and are not yet wired into examples, packs, or external tooling.
  - **Likely cause / nature of issue:** `PR-GX-03` was implemented as an additive first cut that preserves the final GX model without introducing distributed execution or simulator/tooling layers yet.

- **Location:** `specs/` and `catalog/` versus executable behavior
  - **Evidence:** The GX standards layer now has executable counterparts in `greentic-x-flow` and the `gx` profile compiler, but the checked-in `catalog/` entries still do not drive code generation or catalog-backed resolution beyond validation/doctor checks.
  - **Likely cause / nature of issue:** `PR-GX-05` implements the first profile compile path and reference examples, but catalog-aware authoring/generation is still limited.

- **Location:** `crates/gx/src/lib.rs`
  - **Evidence:** The tooling layer now provides a CLI, simulator, profile compiler, and doctor checks, but there is still no separate graphical designer and no dedicated resolver/view validation commands beyond scaffolding and doctor coverage.
  - **Likely cause / nature of issue:** `PR-GX-04` and `PR-GX-05` were implemented as a pragmatic CLI-first toolchain using the existing repo conventions instead of introducing a larger UI/designer surface immediately.

- **Location:** `packs/greentic-x-contracts-reference/components/contract-hook/`, `packs/greentic-x-ops-reference/components/ops-provider/`, `packs/greentic-x-runtime-capability-reference/components/runtime-provider/`
  - **Evidence:** These components are wizard-generated placeholder WASM/component manifests with README scaffolds, not real implementations.
  - **Likely cause / nature of issue:** The repo now supports pack scaffolding and packaging, but production provider components have not been implemented yet.

- **Location:** Workspace publish/release path
  - **Evidence:** `ci/local_check.sh` still reports `No publishable crates found in workspace.` after crate/package checks because all workspace crates remain `publish = false`.
  - **Likely cause / nature of issue:** Crate publishing is intentionally deferred until the crates are considered package-ready.

## 5. Notes for Future Work

- Replace the wizard-generated placeholder provider components with real contract hook, ops provider, and runtime capability implementations when the execution model is ready.
- Decide whether the repo-level `contracts/` and `ops/` trees remain the primary source of truth or whether the `packs/` assets should become canonical.
- Revisit the pinned `cargo-binstall` and `greentic-pack` versions periodically so CI stays deterministic without drifting too far behind the supported toolchain.
- Add deeper JSON Schema validation and policy-hook execution so the runtime enforces more than structural rules.
- Extend the examples so at least one checked-in demo runs through `greentic-x-flow` instead of direct imperative runtime calls.
- Decide whether flow/evidence/view shared models should stay in `greentic-x-flow` or move into a broader shared vocabulary crate as GX stabilizes.
- Decide whether resolver/view packages should gain first-class validation commands beyond the current scaffold-and-doctor approach.
- Extend the simulator so it can consume richer profile/catalog inputs instead of only local flow packages with stub files.
- Decide whether the checked-in compiled `flow.json` files for profile-driven examples should remain committed or be regenerated automatically in CI/tooling.
- Revisit crate publishing once one or more workspace crates are stable enough to remove `publish = false`.
