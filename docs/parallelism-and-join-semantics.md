# Parallelism and Join Semantics

`greentic-x-flow` models split and join explicitly, but the current executor
keeps the implementation deterministic and local.

## Split

A `SplitStep` contains named branches. Each branch runs its own ordered list of
steps against the shared input and current flow context.

Today, branches are executed synchronously in-process. This is a deliberate
implementation choice:

- the flow model already records branch identity and branch status
- the runtime surface does not need to change when true parallel dispatch is
  added later
- tests remain deterministic

## Branch Status

Each branch produces a `BranchExecution` with one of:

- `succeeded`
- `failed`
- `timed_out`

The current code supports simulated timeout handling through
`simulated_duration_ms`. That gives the repo a concrete timeout path and join
behavior without requiring wall-clock orchestration.

## Join

A `JoinStep` merges prior split results using one of:

- `all`: every branch must succeed
- `any`: the first successful branch result is enough
- `all_or_timeout`: merge successful branches and tolerate timed-out branches

Join output is written back into flow context so later steps can use a stable
merged shape rather than branch-local data.

## Why This Shape

The key goal is to make the GX flow model explicit now:

- split/join is part of the domain model
- branch state is observable
- timeout tolerance is representable

The repo can later swap in actual concurrent execution without rewriting the
flow vocabulary.
