# Example Flows

This page explains how the runnable examples in `examples/` exercise the current Greentic-X crates and reference artifacts.

There are now two example layers:

- Rust smoke apps that drive the runtime directly
- flow/profile-driven reference packages that run through `gx simulate`

## `simple-case-app`

Exercises:
- `greentic-x-contracts`: loads `contracts/case/contract.json`
- `greentic-x-runtime`: installs/activates the contract, creates a case, patches fields, appends evidence, transitions state
- `greentic-x-types`: uses patch, append, and transition request models

Artifacts used:
- `contracts/case/contract.json`
- `contracts/case/examples/case.created.json`

Run:

```bash
cargo run -p simple-case-app
```

## `simple-playbook-app`

Exercises:
- `greentic-x-contracts`: loads `gx.playbook` and `gx.outcome`
- `greentic-x-ops`: loads the `playbook-select` op manifest
- `greentic-x-runtime`: creates playbook and outcome resources, installs and invokes the selector op, tracks a playbook-run

Artifacts used:
- `contracts/playbook/contract.json`
- `contracts/playbook/examples/playbook-run.created.json`
- `contracts/outcome/contract.json`
- `contracts/outcome/examples/outcome.created.json`
- `ops/playbook-select/op.json`

Run:

```bash
cargo run -p simple-playbook-app
```

## `end-to-end-demo`

Exercises:
- `greentic-x-contracts`: loads all current reference contracts
- `greentic-x-ops`: loads all current reference ops
- `greentic-x-runtime`: runs a deterministic end-to-end flow across case, evidence, playbook-run, and outcome resources

Artifacts used:
- `contracts/case/`
- `contracts/evidence/`
- `contracts/outcome/`
- `contracts/playbook/`
- `ops/approval-basic/`
- `ops/playbook-select/`
- `ops/rca-basic/`

Run:

```bash
cargo run -p end-to-end-demo
```

## Flow/Profile-Driven Reference Packages

These packages demonstrate the intended GX authoring shape:

- `examples/top-contributors-generic/`
- `examples/entity-utilisation-generic/`
- `examples/change-correlation-generic/`
- `examples/root-cause-split-join-generic/`

Run them with:

```bash
cargo run -p gx -- flow validate examples/top-contributors-generic
cargo run -p gx -- simulate examples/top-contributors-generic
```

The profile-driven examples include `profile.json` alongside the compiled
`flow.json`. The raw-flow example keeps only the flow package form.
