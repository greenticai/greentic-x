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
6. Run `gx doctor .` before opening a PR or handing off to downstream Greentic
   packaging tools.

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
- treat `<solution-id>.pack.input.json` as the GX-produced compatibility input
  for later `greentic-pack` integration
- keep the GX package content source-of-truth readable in the repo
- run both `gx doctor` and `greentic-pack doctor`

When the downstream solution also needs a `.gtbundle`:

- use `gx wizard` to compose the solution and emit bundle handoff inputs
- treat `<solution-id>.toolchain-handoff.json` as the stable machine-readable
  bridge from GX composition into downstream Greentic tool execution
- use `<solution-id>.launcher.answers.json` when you need compatibility with
  the current `greentic-dev wizard` launcher envelope
- use `greentic-bundle` to perform bundle-specific execution
- treat the emitted GX JSON as composition/handoff artifacts rather than as a
  replacement for bundle tooling
