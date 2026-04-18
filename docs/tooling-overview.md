# Tooling Overview

Greentic-X now ships a small CLI, `greentic-x`, for downstream authoring and validation.

The current commands are:

- `greentic-x contract new`
- `greentic-x contract validate`
- `greentic-x op new`
- `greentic-x op validate`
- `greentic-x flow new`
- `greentic-x flow validate`
- `greentic-x resolver new`
- `greentic-x resolver validate`
- `greentic-x view new`
- `greentic-x view validate`
- `greentic-x profile validate`
- `greentic-x profile compile`
- `greentic-x simulate`
- `greentic-x doctor`
- `greentic-x catalog init`
- `greentic-x catalog build`
- `greentic-x catalog validate`
- `greentic-x catalog list`
- `greentic-x wizard run`
- `greentic-x wizard validate`
- `greentic-x wizard apply` (deprecated compatibility bridge)

## What The CLI Covers Today

- scaffold generic contract, op, flow, resolver, and view packages
- validate checked-in contract, op, flow, resolver, and view packages
- simulate a flow package with stubbed resolver/op responses
- run repo-level structural doctor checks
- inspect the checked-in core catalog
- scaffold and validate downstream solution catalog repos
- build canonical `catalog.json` indexes for those repos
- run the catalog-driven wizard run/validate/apply command surface, with
  `apply` retained only as a compatibility bridge

Bare `greentic-x wizard` defaults to the `run` action.
Wizard replay currently supports `--answers`, `--emit-answers`, `--schema-version`, `--migrate`, and repeated `--catalog` flags.
Wizard execution supports `--dry-run`, `--locale`, and a deprecated optional
downstream bundle handoff bridge through `greentic-bundle`.
Wizard locale currently supports embedded `en` (default) and `nl` catalogs with fallback to `en`.
Locale resolution order is `--locale` CLI flag, then answer-document locale (when replaying `--answers`), then environment (`GX_LOCALE`, `GREENTIC_LOCALE`, `LC_ALL`, `LC_MESSAGES`, `LANG`).
Interactive execute flows use a QA-spec-driven composition form when no
`--answers` file is provided and stdin/stdout are TTY.
Interactive GX composition now runs through embedded QA-spec forms under
`crates/gx/questions/`, with GX injecting runtime catalog and existing-solution
choices before handing the form to `greentic-qa-lib`.
Wizard plan output includes normalized input summary and expected file writes.
Wizard dry-run planning is covered by deterministic regression tests.
Wizard emits answer-document metadata compatible with `greentic-bundle` replay:
- `wizard_id=greentic-bundle.wizard.run`
- `schema_id=greentic-bundle.wizard.answers`
- `schema_version=1.0.0` (default)
`greentic-x` writes GX-authored composition outputs under `dist/<solution-id>.*`
and can also emit downstream handoff artifacts for the broader Greentic
toolchain.

GX-authored outputs include:

- `<solution-id>.solution.json`
- `<solution-id>.toolchain-handoff.json`
- `<solution-id>.launcher.answers.json`
- `<solution-id>.pack.input.json`
- `<solution-id>.bundle-plan.json`
- `<solution-id>.bundle.answers.json`
- `<solution-id>.setup.answers.json`
- `<solution-id>.gtc.setup.handoff.json`
- `<solution-id>.gtc.start.handoff.json`
- `<solution-id>.README.generated.md`

The handoff contract is the stable bridge for downstream Greentic tools. It
records the generated solution intent reference, bundle replay inputs, current
`greentic-dev` launcher compatibility details, and the composition locks that
GX resolved while composing the solution.

The pack compatibility input is intentionally partial. It tells
`greentic-pack` what GX knows about provider refs, capability offers,
contracts, flows, and template/default selections, while leaving pack-owned
workflow steps unresolved for the downstream tool.

The generic `gtc` handoff outputs keep orchestration contract-driven:

- `gtc setup --extension-setup-handoff dist/<solution-id>.gtc.setup.handoff.json`
- `gtc start --extension-start-handoff dist/<solution-id>.gtc.start.handoff.json`

This keeps `gtc` at the routing boundary without forcing it to understand GX
composition internals.

When `greentic-bundle` replay is used, the expected downstream bundle artifact
path remains `dist/dist/<solution-id>.gtbundle` because bundle build output is
still rooted under the bundle workspace `dist/` directory.

## Compatibility Test Gating

The repo includes fixture-driven compatibility tests under
`crates/gx/tests/`.

- Default test runs validate GX in isolation.
- Set `GX_TEST_EXTERNAL_TOOLCHAIN=1` to enable optional external-tool replay
  checks when `greentic-bundle` and/or `greentic-dev` are available in `PATH`.
- Those gated checks replay emitted bundle answers through
  `greentic-bundle wizard apply --answers ...` and validate emitted launcher
  answers against the current `greentic-dev wizard --schema` output.

When bundle handoff is enabled, downstream bundle execution is performed via a
deprecated compatibility bridge:

- `greentic-bundle wizard apply --answers dist/<solution-id>.bundle.answers.json`

That final `.gtbundle` is a downstream tool output, not a GX-owned artifact
format or execution path.
Wizard catalog loading merges the built-in GX base catalog with any explicit `--catalog` sources.
Supported explicit catalog source types are:
- local `catalog.json` paths
- `oci://...` catalog refs fetched through `greentic-distributor-client`
Remote catalog and provider refs default to `update_then_pin` resolution so generated artifacts can preserve pinned references.

The implementation is intentionally CLI-first. There is no separate visual
designer yet. The CLI is the current downstream entrypoint for GX composition
authoring.

## Typical Usage

```bash
cargo run -p greentic-x -- contract new contracts/example-contract --contract-id gx.example --resource-type example
cargo run -p greentic-x -- op new ops/example-op --operation-id analyse.example --contract-id gx.example
cargo run -p greentic-x -- flow new flows/example-flow --flow-id example.flow
cargo run -p greentic-x -- profile validate examples/top-contributors-generic/profile.json
cargo run -p greentic-x -- profile compile examples/top-contributors-generic/profile.json --out examples/top-contributors-generic/flow.json

cargo run -p greentic-x -- contract validate contracts/example-contract
cargo run -p greentic-x -- op validate ops/example-op
cargo run -p greentic-x -- flow validate flows/example-flow

cargo run -p greentic-x -- simulate flows/example-flow
cargo run -p greentic-x -- doctor .
cargo run -p greentic-x -- catalog init zain-x
cargo run -p greentic-x -- catalog build --repo zain-x
cargo run -p greentic-x -- catalog validate --repo zain-x
cargo run -p greentic-x -- catalog list --kind ops
cargo run -p greentic-x -- wizard --catalog oci://ghcr.io/greenticai/catalogs/zain-x/catalog.json:latest
```

## Relationship To `greentic-pack` And `greentic-bundle`

The `gx` CLI does not replace `greentic-pack` or `greentic-bundle`.

Use `gx` to scaffold and validate GX package content. Use `greentic-pack` when
you want to package repo assets into `.gtpack` source packs or update the
checked-in pack scaffolds.

Use `greentic-bundle` when you want to run bundle-specific normalization,
setup, and `.gtbundle` generation. GX may emit compatibility artifacts for that
toolchain, but it does not own bundle execution semantics.

See also: `docs/migration/gx-pack-bundle-deprecation.md`.
