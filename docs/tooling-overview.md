# Tooling Overview

Greentic-X now ships a small CLI, `gx`, for downstream authoring and validation.

The current commands are:

- `gx contract new`
- `gx contract validate`
- `gx op new`
- `gx op validate`
- `gx flow new`
- `gx flow validate`
- `gx resolver new`
- `gx resolver validate`
- `gx view new`
- `gx view validate`
- `gx profile validate`
- `gx profile compile`
- `gx simulate`
- `gx doctor`
- `gx catalog list`
- `gx wizard run`
- `gx wizard validate`
- `gx wizard apply`

## What The CLI Covers Today

- scaffold generic contract, op, flow, resolver, and view packages
- validate checked-in contract, op, flow, resolver, and view packages
- simulate a flow package with stubbed resolver/op responses
- run repo-level structural doctor checks
- inspect the checked-in core catalog
- run the initial wizard run/validate/apply command surface

Bare `gx wizard` defaults to the `run` action.
Wizard replay currently supports `--answers`, `--emit-answers`, `--schema-version`, and `--migrate`.
Wizard execution supports `--dry-run`, `--locale`, and `--bundle-handoff` for delegated `greentic-bundle` replay/apply handoff.
Wizard locale currently supports embedded `en` (default) and `nl` catalogs with fallback to `en`.
Locale resolution order is `--locale` CLI flag, then answer-document locale (when replaying `--answers`), then environment (`GX_LOCALE`, `GREENTIC_LOCALE`, `LC_ALL`, `LC_MESSAGES`, `LANG`).
Interactive execute flows now collect answers via `greentic-qa-lib` (`WizardDriver`) when no `--answers` file is provided and stdin/stdout are TTY.
Wizard mode selection supports `--mode` with:
- `assistant_bundle`
- `assistant_template_create`
- `assistant_template_update`
- `domain_template_create`
- `domain_template_update`
Wizard plan output includes normalized input summary and expected file writes.
Wizard dry-run planning is covered by deterministic regression tests (repeated identical input yields identical plan JSON for bundle and template workflows).
Template workflows now materialize a JSON template artifact at `template_output_path` during execute-mode `run/apply`.
Wizard emits answer-document metadata compatible with `greentic-bundle` replay:
- `wizard_id=greentic-bundle.wizard.run`
- `schema_id=greentic-bundle.wizard.answers`
- `schema_version=1.0.0` (default)
`gx` includes a gated replay smoke test that, when `greentic-bundle` is available in `PATH`, replays emitted answers via `greentic-bundle wizard apply --answers ...` and verifies a `.gtbundle` artifact is produced.
`gx` also includes template replay coverage where emitted template-mode answers are fed back into `gx wizard apply --answers ...` and the expected template artifact is materialized.
Wizard normalization currently targets the `assistant_bundle` workflow and defaults these answer keys when missing:
- `workflow=assistant_bundle`
- `mode=create`
- `bundle_name=GX Bundle`
- `bundle_id=gx-bundle`
- `output_dir=dist/bundle`
- `assistant_template_source=local://templates/assistant/default`
- `domain_template_source=local://templates/domain/default`
- `deployment_profile=default`
- `deployment_target=local`
- `provider_categories=[\"llm\"]`
- `bundle_output_path=dist/app.gtbundle`

When a remote template source contains `:latest`, answers must include `latest_policy` (`keep_latest` or `pin`).
In interactive TTY runs, if `:latest` refs are detected and `latest_policy` is missing, the wizard prompts to choose `keep_latest` or `pin`.
For execute-mode `run/apply` flows, `gx` resolves `oci://`, `repo://`, and `store://` refs through `greentic-distributor-client` `v0.4` and records lock metadata in `locks.resolved_source_refs`.
If `latest_policy=pin`, `:latest` refs are rewritten to digest-pinned refs in emitted/handoff answers.
Supported source ref schemes for template/bundle sources are:
- `local://`
- `file://`
- `oci://`
- `repo://`
- `store://`

The implementation is intentionally CLI-first. There is no separate visual
designer yet. The CLI is the current downstream entrypoint for GX authoring.

## Typical Usage

```bash
cargo run -p gx -- contract new contracts/example-contract --contract-id gx.example --resource-type example
cargo run -p gx -- op new ops/example-op --operation-id analyse.example --contract-id gx.example
cargo run -p gx -- flow new flows/example-flow --flow-id example.flow
cargo run -p gx -- profile validate examples/top-contributors-generic/profile.json
cargo run -p gx -- profile compile examples/top-contributors-generic/profile.json --out examples/top-contributors-generic/flow.json

cargo run -p gx -- contract validate contracts/example-contract
cargo run -p gx -- op validate ops/example-op
cargo run -p gx -- flow validate flows/example-flow

cargo run -p gx -- simulate flows/example-flow
cargo run -p gx -- doctor .
cargo run -p gx -- catalog list --kind ops
```

## Relationship To `greentic-pack`

The `gx` CLI does not replace `greentic-pack`.

Use `gx` to scaffold and validate GX package content. Use `greentic-pack` when
you want to package repo assets into `.gtpack` source packs or update the
checked-in pack scaffolds.
