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
- `greentic-x wizard apply`

## What The CLI Covers Today

- scaffold generic contract, op, flow, resolver, and view packages
- validate checked-in contract, op, flow, resolver, and view packages
- simulate a flow package with stubbed resolver/op responses
- run repo-level structural doctor checks
- inspect the checked-in core catalog
- scaffold and validate downstream solution catalog repos
- build canonical `catalog.json` indexes for those repos
- run the catalog-driven wizard run/validate/apply command surface

Bare `greentic-x wizard` defaults to the `run` action.
Wizard replay currently supports `--answers`, `--emit-answers`, `--schema-version`, `--migrate`, and repeated `--catalog` flags.
Wizard execution supports `--dry-run`, `--locale`, and delegated bundling through `greentic-bundle`.
Wizard locale currently supports embedded `en` (default) and `nl` catalogs with fallback to `en`.
Locale resolution order is `--locale` CLI flag, then answer-document locale (when replaying `--answers`), then environment (`GX_LOCALE`, `GREENTIC_LOCALE`, `LC_ALL`, `LC_MESSAGES`, `LANG`).
Interactive execute flows present a persistent composition menu with create, update, and advanced catalog-source options when no `--answers` file is provided and stdin/stdout are TTY.
Wizard plan output includes normalized input summary and expected file writes.
Wizard dry-run planning is covered by deterministic regression tests.
Wizard emits answer-document metadata compatible with `greentic-bundle` replay:
- `wizard_id=greentic-bundle.wizard.run`
- `schema_id=greentic-bundle.wizard.answers`
- `schema_version=1.0.0` (default)
`greentic-x` writes composition artifacts under `dist/<solution-id>.*`, then delegates final bundle generation through:
- `greentic-bundle wizard apply --answers dist/<solution-id>.bundle.answers.json`
Wizard catalog loading merges the built-in GX base catalog with any explicit `--catalog` sources.
Supported explicit catalog source types are:
- local `catalog.json` paths
- `oci://...` catalog refs fetched through `greentic-distributor-client`
Remote catalog and provider refs default to `update_then_pin` resolution so generated artifacts can preserve pinned references.

The implementation is intentionally CLI-first. There is no separate visual
designer yet. The CLI is the current downstream entrypoint for GX authoring.

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

## Relationship To `greentic-pack`

The `gx` CLI does not replace `greentic-pack`.

Use `gx` to scaffold and validate GX package content. Use `greentic-pack` when
you want to package repo assets into `.gtpack` source packs or update the
checked-in pack scaffolds.
