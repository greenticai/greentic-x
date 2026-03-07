# Flow Executor

`greentic-x-flow` is the first GX execution layer that sits above
`greentic-x-runtime`.

It introduces:

- `FlowDefinition` for ordered step graphs
- `FlowEngine` for execution
- `FlowRunRecord` for durable run output
- `FlowRuntime` as the adapter boundary into the underlying runtime
- `EvidenceStore` for evidence persistence
- `ViewRenderer` for neutral presentation output

Supported step kinds today:

- `resolve`: invoke a registered resolver through the normalized resolver envelope
- `call`: invoke a registered operation through the normalized operation envelope
- `map`: write derived values into flow context
- `branch`: choose one branch by evaluating simple path-equality cases
- `split`: execute named branches and collect branch results
- `join`: merge branch results using a declared join mode
- `return`: emit the final flow result

The current implementation is intentionally deterministic and in-process. It is
designed to prove the GX execution model without pulling in schedulers, queues,
or external persistence yet.

## Composition Model

`greentic-x-runtime` still owns:

- contracts
- resources
- typed links
- resolvers
- operations
- audit events

`greentic-x-flow` composes those capabilities into flow execution. The default
`RuntimeFlowAdapter` delegates resolver and operation calls to the runtime and
captures the normalized results for later evidence and view handling.

## Current Limits

- split execution is synchronous, not parallelized by worker infrastructure
- branch predicates are intentionally simple
- view rendering is renderer-driven, not a full template engine
- evidence storage is pluggable but currently in-memory in repo examples/tests

Those limits are acceptable for `PR-GX-03`; they preserve the target model
without overcommitting to infrastructure too early.
