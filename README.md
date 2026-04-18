# greentic-x

`greentic-x` is the bootstrap repository for the Greentic-X layer: shared runtime crates, reference contracts, reference ops, examples, and architecture/spec documentation that sit above the minimal Greentic core. This repository is intentionally starting small, but its structure is now aligned with the planned multi-crate workspace so future PRs can add real implementation without restructuring the repo again.

Greentic core should stay narrow and foundational. This repository is where the broader object model, runtime behavior, reusable operations, example applications, and supporting reference material can grow.

## What Belongs Here

- Shared Rust crates for Greentic-X types, events, contracts, runtime, flow execution, and ops.
- Reference contract artifacts under `contracts/`.
- Reference operation artifacts under `ops/`.
- Wizard-backed source packs under `packs/`.
- Formal GX specs under `specs/`.
- Minimal reusable GX catalog entries under `catalog/`.
- Example applications and end-to-end demos under `examples/`.
- Flow/profile-driven reference packages under `examples/`.
- Architecture and specification notes under `docs/`.

## What Does Not Belong In Greentic Core

- Domain-rich contract libraries and reference resources.
- Runtime orchestration and contract enforcement logic.
- Reusable operational workflows and reference ops.
- Example apps, demos, and extended documentation for the Greentic-X model.

## Repository Layout

- `crates/greentic-x-types`: shared type vocabulary for identifiers, revisions, provenance, schemas, and mutation requests.
- `crates/greentic-x-events`: structured event envelopes and lifecycle payloads.
- `crates/greentic-x-contracts`: contract manifest and validation models.
- `crates/greentic-x-runtime`: in-memory-capable runtime core for contracts, resources, typed links, resolvers, and op invocation.
- `crates/greentic-x-flow`: flow execution, evidence capture, and neutral view rendering built on top of the runtime.
- `crates/greentic-x-ops`: op manifest and validation models.
- `crates/gx`: CLI tooling for GX scaffolding, validation, simulation, doctor checks, and catalog inspection.
- `contracts/`: reference contract directories for `case`, `evidence`, `outcome`, and `playbook`.
- `ops/`: reference op directories for `approval-basic`, `playbook-select`, and `rca-basic`.
- `examples/`: runnable example applications for `simple-case-app`, `simple-playbook-app`, and `end-to-end-demo`.
- `examples/`: runnable Rust smoke apps plus flow/profile-driven reference packages such as `top-contributors-generic` and `root-cause-split-join-generic`.
- `packs/`: wizard-backed source packs for the current contract, ops, and runtime-capability references.
- `specs/`: formal GX spec packages for resources, resolver results, operation descriptors, flow runs, evidence, views, and the observability profile.
- `catalog/`: minimal reusable GX contract/spec, resolver, op, view, and flow-template catalog entries.
- `docs/`: architecture, model, governance, and examples notes.

## Current Status

- Workspace structure: implemented.
- Shared types, events, contracts, runtime, flow execution, and ops: implemented first-cut models.
- Example apps: runnable deterministic demos using the local reference artifacts.
- CI and releases: wired through `ci/local_check.sh` and GitHub Actions.
- Pack scaffolding: wizard-backed source packs exist for contracts, ops, and runtime capability references.
- Specs and catalog: initial GX standards layer now exists under `specs/` and `catalog/`.
- Tooling: `gx` now scaffolds contract/op/flow/resolver/view packages, validates contracts/ops/flows/resolvers/views, simulates flows, runs doctor checks, and lists catalog entries.
- Catalog tooling: `gx` now also scaffolds solution catalog repos, builds canonical `catalog.json` indexes, validates them, publishes both catalog indexes and tar bundles, and lets `gx wizard` merge local or OCI-backed solution catalogs through repeated `--catalog` flags.
- `gtc` integration: `gx wizard` now emits generic `gtc` setup/start handoff artifacts alongside the existing GX solution, pack, bundle, and launcher outputs so orchestration can stay contract-driven.
- Profile/examples: `gx.observability.playbook.v1` now compiles into normal GX flows, and four generic reference example packages are checked in under `examples/`.
- Remaining work: policy/migration execution, richer pack contents, designer UX, and publish decisions.

## Non-Goals For This Bootstrap

- No production storage backend or operator integration yet.
- No production-ready contract/op/runtime provider components yet; the current
  `packs/` entries are wizard-backed source packs with placeholder generated
  components.
- No detailed governance machinery in code yet.

## CI and Releases

Run local checks with:

```bash
bash ci/local_check.sh
```

The script runs formatting, clippy, tests, build, docs, and crates.io dry-run packaging checks for publishable workspace crates.
It also validates the checked-in GX specs and catalog when `specs/` and `catalog/` are present.
It also runs `cargo run -p greentic-x -- doctor .` so the repo-level GX tooling checks stay green.

When `greentic-pack` is available locally, it also builds and inspects the
source packs under `packs/`. GitHub Actions now installs `greentic-pack` via
`cargo-binstall`, using a cached `cargo-binstall` binary and a cached
`greentic-pack` binary where available.

## Packs

- `packs/greentic-x-contracts-reference`: source pack that mirrors the repo
  reference contracts into `assets/contracts/`.
- `packs/greentic-x-ops-reference`: source pack that mirrors the repo
  reference ops into `assets/ops/`.
- `packs/greentic-x-runtime-capability-reference`: wizard-backed runtime
  capability reference scaffold with an example capability offer.

Regenerate or update a scaffold with:

```bash
greentic-pack wizard apply --answers packs/_wizard/greentic-x-contracts-reference.answers.json
greentic-pack wizard apply --answers packs/_wizard/greentic-x-ops-reference.answers.json
greentic-pack wizard apply --answers packs/_wizard/greentic-x-runtime-capability.answers.json
```

Build a source pack into a `.gtpack` with:

```bash
greentic-pack build --in packs/greentic-x-contracts-reference
greentic-pack build --in packs/greentic-x-ops-reference
greentic-pack build --in packs/greentic-x-runtime-capability-reference
```

GitHub Actions:

- `.github/workflows/ci.yml` runs lint, test, and package validation on pull requests and pushes.
- `.github/workflows/publish.yml` verifies the version tag and publishes any publishable crates before creating a GitHub release.

Release flow:

1. Bump the shared workspace version in `Cargo.toml`.
2. Run `bash ci/local_check.sh`.
3. Create and push a matching tag such as `v0.4.0`.
4. Push the tag to trigger `.github/workflows/publish.yml`.

Required GitHub secret:

- `CARGO_REGISTRY_TOKEN`

## Docs

- `docs/architecture.md`
- `docs/contracts.md`
- `docs/specs-overview.md`
- `docs/core-catalog.md`
- `docs/authoring-profile-observability.md`
- `docs/ops.md`
- `docs/runtime.md`
- `docs/runtime-overview.md`
- `docs/runtime-boundary.md`
- `docs/flow-executor.md`
- `docs/parallelism-and-join-semantics.md`
- `docs/evidence-and-view-separation.md`
- `docs/tooling-overview.md`
- `docs/catalog-repos.md`
- `docs/how-to-build-a-downstream-solution.md`
- `docs/gtc-integration.md`
- `docs/telco-gtc-e2e-smoke.md`
- `docs/simulation-workflow.md`
- `docs/reference-examples.md`
- `docs/observability-profile-vs-raw-flows.md`
- `docs/why-six-primitives.md`
- `docs/governance.md`
- `docs/examples.md`

## License

MIT
