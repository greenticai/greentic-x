# Repository Overview

## High-Level Purpose

`greentic-x` is the Greentic-X workspace that extends the narrower Greentic
core with:

- shared Rust crates for GX types, events, contracts, runtime, flow execution,
  and ops metadata
- the `gx` CLI for scaffolding, validation, simulation, doctor checks, catalog
  workflows, and wizard-driven solution composition
- checked-in reference contracts, ops, specs, catalog entries, packs, and
  runnable examples

The repository is both a code workspace and a reference implementation surface
for the current GX model.

## Workspace Snapshot

- Root manifest: `Cargo.toml`
- Workspace version: `0.4.11`
- Rust edition: `2024`
- Workspace crates:
  - `crates/gx`
  - `crates/greentic-x-types`
  - `crates/greentic-x-events`
  - `crates/greentic-x-contracts`
  - `crates/greentic-x-runtime`
  - `crates/greentic-x-flow`
  - `crates/greentic-x-ops`
- Workspace example apps:
  - `examples/simple-case-app`
  - `examples/simple-playbook-app`
  - `examples/end-to-end-demo`

## Main Repository Areas

- `crates/`: Rust workspace code, including the `gx` CLI and GX libraries
- `contracts/`: reference contracts for `case`, `evidence`, `outcome`, and
  `playbook`
- `ops/`: reference operations for `approval-basic`, `playbook-select`, and
  `rca-basic`
- `examples/`: runnable smoke apps plus checked-in generic flow/profile examples
- `packs/`: source packs and wizard answers for contracts, ops, and runtime
  capability references
- `specs/`: formal GX contract/profile spec packages
- `catalog/`: checked-in core catalog entries and indexes
- `docs/`: architecture, runtime, contracts, ops, tooling, examples, and
  governance notes
- `schemas/`: shared JSON schemas used by repo tooling and generated outputs
- `templates/`: starter assistant/domain templates
- `flows/`: example flow assets
- `setup_profiles/`: setup profile examples
- `dist/`: checked-in generated solution outputs
- `ci/`: local and CI helper scripts
- `.github/workflows/`: CI and publish automation
- `SECURITY.md`: security policy

## Crates

- `crates/gx`
  - CLI for scaffolding packages, validating contracts/ops/flows/resolvers/views,
    compiling profiles, simulating flows, running doctor checks, catalog work,
    and wizard composition.
- `crates/greentic-x-types`
  - Shared identifiers, provenance, schema references, links, mutation request
    types, and operation/resolver envelope models.
- `crates/greentic-x-events`
  - Event envelope and lifecycle payload models used for runtime audit events.
- `crates/greentic-x-contracts`
  - Contract manifest and validation models.
- `crates/greentic-x-runtime`
  - Storage-agnostic runtime core for contracts, resources, links, resolvers,
    operations, and schema-aware validation when schemas are registered.
- `crates/greentic-x-flow`
  - Flow execution, evidence capture, split/join handling, and neutral view
    rendering on top of the runtime.
- `crates/greentic-x-ops`
  - Operation manifest and validation models.

## Examples And Artifacts

- Runnable app crates:
  - `examples/simple-case-app`
  - `examples/simple-playbook-app`
  - `examples/end-to-end-demo`
- Generic checked-in examples:
  - `examples/change-correlation-generic`
  - `examples/entity-utilisation-generic`
  - `examples/root-cause-split-join-generic`
  - `examples/top-contributors-generic`
- Source packs:
  - `packs/greentic-x-contracts-reference`
  - `packs/greentic-x-ops-reference`
  - `packs/greentic-x-runtime-capability-reference`
- Checked-in generated solution artifacts:
  - `dist/gx-solution.README.generated.md`
  - `dist/gx-solution.bundle-plan.json`
  - `dist/gx-solution.bundle.answers.json`
  - `dist/gx-solution.setup.answers.json`
  - `dist/gx-solution.solution.json`

## Tooling And Verification

- Main local validation entrypoint: `bash ci/local_check.sh`
- Supporting repo checks:
  - `ci/check_packs.sh`
  - `ci/check_specs_catalog.py`
  - `ci/publishable_crates.py`
- CI workflows:
  - `.github/workflows/ci.yml`
  - `.github/workflows/publish.yml`

## Toolchain Boundary

- `gx` is the Greentic-X composition/tooling surface inside this repo.
- `gtc wizard` currently routes into `greentic-dev`, which is the present
  top-level launcher/orchestration host.
- `greentic-pack` owns pack workflows and side effects.
- `greentic-bundle` owns bundle workflows and `.gtbundle` generation.
- `greentic-qa-lib` is a reusable QA runtime/form layer used by sibling tools,
  not the current top-level orchestration owner.
- `greentic-cap` defines useful capability and bundle/setup artifact concepts,
  but current GX/toolchain integration is still incomplete.

## Current State

- The workspace structure, reference artifacts, specs, catalog, examples, and
  `gx` CLI are all present and wired together.
- `gx` currently overlaps with the broader Greentic wizard toolchain by acting
  as its own composition wizard instead of integrating through
  `greentic-dev`'s launcher contract.
- The runtime supports registered-schema validation for resource and operation
  payloads, but policy hooks, migrations, durable backends, and production
  provider integrations are still incomplete.
- Flow execution, evidence, and view support exist as an in-process first cut;
  they are not yet a distributed or durable execution system.
- Pack components under `packs/*/components/` are still scaffolds/placeholders
  rather than production implementations.
- Example apps and generic examples are deterministic local references, not
  production integrations.

## `.codex` Notes

- Historical planning and implementation notes live under `.codex/done/` and
  related `.codex/PR-*.md` files.
- This overview should be updated whenever workspace members, top-level repo
  areas, or checked-in generated outputs change.
