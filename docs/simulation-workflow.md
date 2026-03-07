# Simulation Workflow

`gx simulate` is the current local execution loop for flow packages.

## Flow Package Shape

A scaffolded flow package contains:

- `manifest.json`
- `flow.json`
- `stubs.json`
- `README.md`

`flow.json` is deserialized into `greentic-x-flow::FlowDefinition`.
`stubs.json` provides deterministic operation and resolver responses for local
simulation.

## Run A Simulation

```bash
cargo run -p gx -- flow new flows/example-flow --flow-id example.flow
cargo run -p gx -- flow validate flows/example-flow
cargo run -p gx -- simulate flows/example-flow
```

The output is a serialized `FlowRunRecord` containing:

- run status
- final context
- per-step state
- evidence refs
- final result
- optional neutral view

## What To Stub

Use operation stubs when the flow depends on:

- query operations
- analysis steps
- presentation steps

Use resolver stubs when the flow depends on resource lookup before later
processing.

This keeps downstream teams productive before real adapters or components exist.

## Limits

The current simulator is intentionally local and deterministic:

- it uses `StaticFlowRuntime`
- split execution is still in-process
- there is no queue, scheduler, or remote worker model yet

That is enough for structural validation and authoring feedback, which is the
goal of `PR-GX-04`.
