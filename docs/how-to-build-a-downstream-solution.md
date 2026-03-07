# How To Build A Downstream Solution

This repo is intended to be reused by downstream solution repos rather than
copied wholesale.

## Suggested Downstream Layout

Start with a repo that has:

- `contracts/`
- `ops/`
- `flows/`
- `resolvers/`
- `views/`
- optional `packs/` if you also package the solution with `greentic-pack`

## Recommended Workflow

1. Scaffold the baseline GX packages:

```bash
cargo run -p gx -- contract new contracts/customer-case --contract-id gx.customer.case --resource-type case
cargo run -p gx -- op new ops/analyse-customer-case --operation-id analyse.customer.case --contract-id gx.customer.case
cargo run -p gx -- flow new flows/customer-triage --flow-id customer.triage
cargo run -p gx -- resolver new resolvers/resolve-customer --resolver-id resolve.customer
cargo run -p gx -- view new views/customer-summary --view-id customer-summary
```

2. Replace generic scaffold text with your real domain details.
3. Point ops and resolvers at downstream adapters or components.
4. Validate each package as you fill it in.
5. Run `gx simulate` on flows while the downstream adapters are still stubbed.
6. Run `gx doctor .` before packaging or opening a PR.

## Reuse Guidance

Downstream repos should prefer:

- GX core specs from `catalog/core/`
- shared runtime and flow models from this repo
- existing Greentic shared crates when a type/interface already exists there

They should avoid:

- redefining GX envelopes or flow vocabulary locally
- inventing a second validation convention
- coupling views directly to one channel-specific rendering surface too early

## Packaging

When the downstream solution also needs `.gtpack` artifacts:

- use `greentic-pack` for pack creation and packaging
- keep the GX package content source-of-truth readable in the repo
- run both `gx doctor` and `greentic-pack doctor`
