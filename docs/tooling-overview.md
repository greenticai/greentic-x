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

## What The CLI Covers Today

- scaffold generic contract, op, flow, resolver, and view packages
- validate checked-in contract, op, flow, resolver, and view packages
- simulate a flow package with stubbed resolver/op responses
- run repo-level structural doctor checks
- inspect the checked-in core catalog

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
