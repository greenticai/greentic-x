# greentic-x

`greentic-x` is the bootstrap repository for the Greentic-X layer: shared runtime crates, reference contracts, reference ops, examples, and architecture/spec documentation that sit above the minimal Greentic core. This repository is intentionally starting small, but its structure is now aligned with the planned multi-crate workspace so future PRs can add real implementation without restructuring the repo again.

Greentic core should stay narrow and foundational. This repository is where the broader object model, runtime behavior, reusable operations, example applications, and supporting reference material can grow.

## What Belongs Here

- Shared Rust crates for Greentic-X types, events, contracts, runtime, and ops.
- Reference contract artifacts under `contracts/`.
- Reference operation artifacts under `ops/`.
- Example applications and end-to-end demos under `examples/`.
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
- `crates/greentic-x-runtime`: in-memory-capable runtime core for contracts, resources, and op invocation.
- `crates/greentic-x-ops`: op manifest and validation models.
- `contracts/`: reference contract directories for `case`, `evidence`, `outcome`, and `playbook`.
- `ops/`: reference op directories for `approval-basic`, `playbook-select`, and `rca-basic`.
- `examples/`: runnable example applications for `simple-case-app`, `simple-playbook-app`, and `end-to-end-demo`.
- `docs/`: architecture, model, governance, and examples notes.

## Current Status

- Workspace structure: implemented.
- Shared types, events, contracts, runtime, and ops: implemented first-cut models.
- Example apps: runnable deterministic demos using the local reference artifacts.
- CI and releases: wired through `ci/local_check.sh` and GitHub Actions.
- Remaining work: deeper validation, richer docs, pack/wizard integration, and publish decisions.

## Non-Goals For This Bootstrap

- No production storage backend or operator integration yet.
- No pack generation or wizard integration for the new contract/op artifacts yet.
- No detailed governance machinery in code yet.

## CI and Releases

Run local checks with:

```bash
bash ci/local_check.sh
```

The script runs formatting, clippy, tests, build, docs, and crates.io dry-run packaging checks for publishable workspace crates.

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
- `docs/ops.md`
- `docs/runtime.md`
- `docs/governance.md`
- `docs/examples.md`

## License

MIT
