# Repository Overview

## 1. High-Level Purpose

This repository is the bootstrap workspace for `greentic-x`, a Rust-based Greentic repo intended to host shared runtime crates, reference contracts, reusable ops, example applications, and architecture/spec documentation. The repository now has implemented shared types, events, contract descriptors, local reference contract artifacts, an initial storage-agnostic runtime core, an ops descriptor layer with local reference ops, runnable example applications, and a first-cut documentation set covering architecture, contracts, ops, runtime, governance, and examples.

The repo contains planning material under `.codex/` for the follow-on implementation sequence, plus CI/release automation that already understands the workspace layout. This makes the repo structurally ready for incremental implementation without another workspace reorganization.

## 2. Main Components and Functionality

- **Path:** `Cargo.toml`
  - **Role:** Root Cargo workspace manifest.
  - **Key functionality:**
    - Defines a workspace with five core crates plus three example application crates.
    - Stores shared workspace package metadata, workspace lints, and the shared version `0.4.0`.
  - **Key dependencies / integration points:**
    - Used by `cargo metadata`, `ci/local_check.sh`, and the publish workflow’s tag/version verification logic.

- **Path:** `crates/greentic-x-types`
  - **Role:** Shared type vocabulary crate for Greentic-X resource and operation models.
  - **Key functionality:**
    - Defines validated string identifiers for actors, contracts, contract versions, operations, resource IDs, and resource types.
    - Defines generic audit/provenance models, revision handling, schema/compatibility references, and mutation request types for patch, append, and transition operations.
    - Supports `serde`/`serde_json` serialization and includes doc examples plus unit tests for JSON payloads and identifier validation.
  - **Key dependencies / integration points:**
    - Used by `greentic-x-events`.
    - Intended to be the canonical shared model layer for contracts, runtime, ops, and examples.

- **Path:** `crates/greentic-x-events`
  - **Role:** Structured event model crate for Greentic-X lifecycle events.
  - **Key functionality:**
    - Defines generic `EventEnvelope<T>` and `EventMetadata` models for audit, causation, and partitioning.
    - Defines payload structs and typed constructors for resource creation/patch/append/transition events, contract installation/activation events, and operation installation/execution events.
    - Supports `serde`/`serde_json` serialization and includes doc examples plus unit tests for JSON event payloads.
  - **Key dependencies / integration points:**
    - Depends on `greentic-x-types` for identifiers, provenance, revisions, and compatibility references.
    - Intended to be emitted by the future runtime and consumed by contracts, ops, and examples.

- **Path:** `crates/greentic-x-contracts`
  - **Role:** Contract descriptor and validation crate for Greentic-X reference contracts.
  - **Key functionality:**
    - Defines `ContractManifest`, `ResourceDefinition`, mutation rules, append collection declarations, transitions, event declarations, optional policy hooks, migration references, and validation issues.
    - Provides structural validation for contract manifests and tests that load the reference contracts under `contracts/` and verify they parse and validate successfully.
    - Includes a doc example showing how to build and validate a manifest programmatically.
  - **Key dependencies / integration points:**
    - Depends on `greentic-x-types` and `greentic-x-events` for identifiers, schema references, compatibility references, and event types.
    - Used to define and validate the local JSON-based contract artifacts under `contracts/`.

- **Path:** `crates/greentic-x-runtime`
  - **Role:** Storage-agnostic runtime core for contract/resource/op lifecycle handling.
  - **Key functionality:**
    - Defines a generic `Runtime<S, E>` over a resource store and event sink, plus in-memory adapters for tests and examples.
    - Implements contract installation/activation, resource creation/get/list/patch/append/transition, operation registration/invocation, optimistic concurrency checks, and runtime event emission.
    - Includes tests for allowed vs. denied patches, append-only collections, transitions, revision conflicts, missing operations, and emitted event flows.
  - **Key dependencies / integration points:**
    - Depends on `greentic-x-types`, `greentic-x-events`, `greentic-x-contracts`, and `greentic-x-ops`.
    - Consumes active contract manifests to validate patch, append, and transition behavior.
    - Registers validated `OperationManifest` values from `greentic-x-ops` and checks declared supported contracts before allowing operation installation.

- **Path:** `crates/greentic-x-ops`
  - **Role:** Operation descriptor and validation crate for Greentic-X reference ops.
  - **Key functionality:**
    - Defines `OperationManifest`, supported contract declarations, permission requirements, example payloads, and validation issues.
    - Provides structural validation for operation manifests and tests that load the reference ops under `ops/` and verify they parse and validate successfully.
    - Includes a doc example showing how to build and validate an op manifest programmatically.
  - **Key dependencies / integration points:**
    - Depends on `greentic-x-types` for identifiers, schema references, and compatibility references.
    - Used by `greentic-x-runtime` for operation registration and compatibility checks.
    - Used to define and validate the local JSON-based op artifacts under `ops/`.

- **Path:** `contracts/`
  - **Role:** Reference contract artifact area.
  - **Key functionality:**
    - Contains concrete reference contracts for `case`, `evidence`, `outcome`, and `playbook`.
    - Each contract directory now includes a `contract.json` manifest, JSON Schemas under `schemas/`, sample payloads under `examples/`, and a short README.
  - **Key dependencies / integration points:**
    - Parsed and validated by tests in `greentic-x-contracts`.
    - Intentionally kept as local repo artifacts only; no `.gtpack` generation is implemented yet.

- **Path:** `ops/`
  - **Role:** Reference operation artifact area.
  - **Key functionality:**
    - Contains concrete reference operations for `approval-basic`, `playbook-select`, and `rca-basic`.
    - Each op directory now includes an `op.json` manifest, JSON Schemas under `schemas/`, sample invocation payloads under `examples/`, a simple behavior note in `source.md`, and a short README.
  - **Key dependencies / integration points:**
    - Parsed and validated by tests in `greentic-x-ops`.
    - Serve as local repo artifacts only; no `.gtpack` generation is implemented yet.

- **Path:** `examples/`
  - **Role:** Example application area.
  - **Key functionality:**
    - Contains runnable binary crates for `simple-case-app`, `simple-playbook-app`, and `end-to-end-demo`.
    - Each example loads the real local contract/op manifests from `contracts/` and `ops/`, uses the in-memory runtime, and prints a deterministic final state snapshot.
  - **Key dependencies / integration points:**
    - `simple-case-app` exercises case creation, patching, evidence append, and transition flow.
    - `simple-playbook-app` exercises playbook selection, playbook-run tracking, and outcome updates.
    - `end-to-end-demo` exercises a larger flow spanning case, evidence, playbook, outcome, and all current reference ops.

- **Path:** `docs/`
  - **Role:** First-cut architecture, model, governance, and examples documentation set.
  - **Key functionality:**
    - `docs/architecture.md` explains repo boundaries and the relationship between core crates, reference artifacts, and examples.
    - `docs/contracts.md`, `docs/ops.md`, and `docs/runtime.md` describe the current manifest and runtime models.
    - `docs/governance.md` provides lightweight compatibility, versioning, and proposal notes.
    - `docs/examples.md` explains which crates and artifacts each runnable example exercises and how to run them.
  - **Key dependencies / integration points:**
    - These docs now serve as the in-repo reference point for the implemented model and should reduce reliance on the planning notes under `.codex/`.

- **Path:** `ci/local_check.sh`
  - **Role:** Local developer CI wrapper.
  - **Key functionality:**
    - Runs `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build`, and `cargo doc`.
    - Discovers publishable crates and runs `cargo package`, packaged-file validation, and `cargo publish --dry-run` when any exist.
  - **Key dependencies / integration points:**
    - Uses `ci/publishable_crates.py` for publishable crate discovery.
    - Correctly exits cleanly when the workspace has no publishable crates yet.

- **Path:** `ci/publishable_crates.py`
  - **Role:** Publishable workspace crate discovery helper.
  - **Key functionality:**
    - Reads `cargo metadata`.
    - Filters out crates with `publish = false`.
    - Emits publishable crate names or crate/manifest details in dependency order, and can also print the shared workspace version.
  - **Key dependencies / integration points:**
    - Used by `ci/local_check.sh` and `.github/workflows/publish.yml`.

- **Path:** `.github/workflows/ci.yml`
  - **Role:** Pull request and branch CI workflow.
  - **Key functionality:**
    - Runs lint, test, and package-dry-run jobs on `pull_request` and pushes to `main`/`master`.
    - Cancels redundant runs per ref using workflow concurrency.
  - **Key dependencies / integration points:**
    - Uses standard Rust toolchain setup and executes `bash ci/local_check.sh` for package validation.

- **Path:** `.github/workflows/publish.yml`
  - **Role:** Tag/manual release workflow.
  - **Key functionality:**
    - Verifies that the Git tag matches the shared workspace version from the root Cargo manifest.
    - Runs the full local check sequence before publishing.
    - Publishes crates to crates.io only when publishable crates exist, then creates a GitHub release with generated notes.
  - **Key dependencies / integration points:**
    - Requires the `CARGO_REGISTRY_TOKEN` GitHub secret.
    - Uses `ci/publishable_crates.py` to determine publish order.

- **Path:** `.codex/PR-01-repo-bootstrap.md` through `.codex/PR-07-docs-architecture-and-governance.md`
  - **Role:** Planning documents for future repository expansion.
  - **Key functionality:**
    - Describe intended crates, directories, and implementation phases for types, events, contracts, runtime, ops, examples, and docs.
    - Provide roadmap-style guidance rather than executable code.
  - **Key dependencies / integration points:**
    - Inform the repo overview and clarify the gap between current implementation and intended architecture.

## 3. Work In Progress, TODOs, and Stubs

- **Location:** `crates/greentic-x-types/src/lib.rs`
  - **Status:** partial
  - **Short description:** Core identifier, provenance, revision, schema, compatibility, and mutation request models are implemented, but the crate does not yet include richer envelopes, schema registries, or cross-crate integration with contracts/runtime.

- **Location:** `crates/greentic-x-events/src/lib.rs`
  - **Status:** partial
  - **Short description:** Canonical event envelopes and core lifecycle payload structs are implemented and are now emitted by the runtime, but the broader event catalog and external integration story are still limited.

- **Location:** `crates/greentic-x-contracts/src/lib.rs`
  - **Status:** partial
  - **Short description:** Contract manifest models and structural validation helpers are implemented, but there is no runtime enforcement, schema validation engine, migration execution, or policy execution yet.

- **Location:** `crates/greentic-x-runtime/src/lib.rs`
  - **Status:** partial
  - **Short description:** The runtime crate now implements core registries, lifecycle logic, event hooks, in-memory adapters, and operation registration against `greentic-x-ops`, but it does not yet perform full schema validation, policy execution, or migration execution.

- **Location:** `crates/greentic-x-ops/src/lib.rs`
  - **Status:** partial
  - **Short description:** Operation manifest models, supported-contract declarations, permission metadata, examples, and structural validation helpers are implemented, but there is no separate execution harness, packaging workflow, or richer op runtime integration beyond manifest registration yet.

- **Location:** `contracts/case/`, `contracts/evidence/`, `contracts/outcome/`, `contracts/playbook/`
  - **Status:** partial
  - **Short description:** Reference contract directories now include manifests, schemas, transitions, event declarations, and examples, and the runtime can consume equivalent manifests in memory, but the on-disk artifacts are still local JSON only and are not yet packaged via `greentic-pack` wizard.

- **Location:** `ops/approval-basic/`, `ops/playbook-select/`, `ops/rca-basic/`
  - **Status:** partial
  - **Short description:** Reference op directories now include manifests, schemas, source notes, and examples, but they remain local JSON/documentation artifacts and are not yet packaged or backed by dedicated executable components.

- **Location:** `examples/simple-case-app/`, `examples/simple-playbook-app/`, `examples/end-to-end-demo/`
  - **Status:** partial
  - **Short description:** Runnable example binaries now exist and are exercised by the workspace build, but they are still deterministic local demos rather than richer user-facing apps or integration test harnesses.

- **Location:** `docs/architecture.md`, `docs/contracts.md`, `docs/ops.md`, `docs/runtime.md`, `docs/governance.md`, `docs/examples.md`
  - **Status:** partial
  - **Short description:** A first-cut documentation set now exists, but it is still intentionally lightweight and will need refinement as runtime validation, packaging, and publication policy mature.

- **Location:** `.codex/PR-01-repo-bootstrap.md`
  - **Status:** partial
  - **Short description:** The repo structure described here now exists, but the crates and artifact directories are still placeholders awaiting follow-on implementation.

- **Location:** `.codex/PR-02-types-and-events.md`
  - **Status:** partial
  - **Short description:** The requested crates now exist with initial reusable models and serialization tests, but follow-on work is still needed to expand the vocabulary as contracts/runtime/ops are implemented.

- **Location:** `.codex/PR-03-contracts-crate-and-reference-contracts.md`
  - **Status:** partial
  - **Short description:** The contracts crate and initial reference contracts now exist, but follow-on runtime integration, stronger validation, and packaging/generation workflows are still pending.

- **Location:** `.codex/PR-04-runtime-core.md`
  - **Status:** partial
  - **Short description:** The runtime core and in-memory adapters now exist with contract/resource/op lifecycle tests, but the later integration work called out in the PR remains to be implemented.

- **Location:** `.codex/PR-05-ops-crate-and-reference-ops.md`
  - **Status:** partial
  - **Short description:** The ops crate and initial reference ops now exist, but execution components, richer harnesses, and packaging/generation workflows are still pending.

- **Location:** `.codex/PR-06-examples-apps.md`
  - **Status:** partial
  - **Short description:** Runnable examples and a top-level examples walkthrough now exist, but broader polish and richer end-user docs are still pending.

- **Location:** `.codex/PR-07-docs-architecture-and-governance.md`
  - **Status:** partial
  - **Short description:** The requested architecture, model, runtime, governance, and examples docs now exist, but they are an initial documentation baseline rather than a final specification set.

## 4. Broken, Failing, or Conflicting Areas

- **Location:** Repository structure vs. `.codex/PR-01-repo-bootstrap.md`
  - **Evidence:** The planned workspace layout now exists, with real implementations in the core crates, `contracts/`, `ops/`, and `examples/`; most remaining gaps are in broader documentation and future integrations.
  - **Likely cause / nature of issue:** The repo is being implemented in staged PR order; the remaining planned areas have not been built yet.

- **Location:** `.codex/PR-02-types-and-events.md` through `.codex/PR-07-docs-architecture-and-governance.md`
  - **Evidence:** `PR-02` through `PR-07` are now at least partially implemented in code or docs, but later polish/integration work is still absent from the codebase.
  - **Likely cause / nature of issue:** The roadmap is now mainly ahead on depth, polish, and tooling integration rather than missing major categories of work.

- **Location:** `crates/greentic-x-runtime/src/lib.rs`
  - **Evidence:** The runtime validates lifecycle behavior against contract metadata and supported-contract declarations from ops manifests, but it does not execute JSON Schema validation against the referenced schemas.
  - **Likely cause / nature of issue:** The current runtime focuses on structural lifecycle enforcement first; deeper schema validation is deferred.

- **Location:** Contract artifact packaging workflow
  - **Evidence:** Reference contracts and reference ops are stored as local JSON manifests and schemas only; there is no wizard or `greentic-pack` integration to generate `.gtpack` bundles.
  - **Likely cause / nature of issue:** Pack generation was intentionally deferred until the wizard/tooling is updated to support these new contract and ops pack types.

- **Location:** Workspace publish/release path
  - **Evidence:** `ci/local_check.sh` reports `No publishable crates found in workspace.` because all five member crates are marked `publish = false`.
  - **Likely cause / nature of issue:** Publishing is intentionally disabled until the bootstrap placeholders are replaced with real, package-ready crates.

## 5. Notes for Future Work

- Refine the new docs as the implementation deepens so architecture and governance notes stay aligned with real runtime behavior.
- Consider turning the example binaries into richer smoke or integration tests once the runtime and op execution model stabilizes further.
- Update `greentic-pack` wizard later if these local contract and ops artifacts need to become generated or packaged `.gtpack` bundles.
- Decide which crates should become publishable first, then remove `publish = false` selectively and extend package include rules for any runtime assets.
- Add stronger schema-validation and policy-hook integration once the runtime’s extension boundaries stabilize, and consider moving reference op execution logic into dedicated components when examples arrive.
