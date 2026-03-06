# PR-06 Add Example Apps and End-to-End Demo

## Depends on

- `PR-01-repo-bootstrap.md`
- `PR-02-types-and-events.md`
- `PR-03-contracts-crate-and-reference-contracts.md`
- `PR-04-runtime-core.md`
- `PR-05-ops-crate-and-reference-ops.md`

## Goal

Add example applications under `examples/` to show how Greentic-X is actually used in practice.

This is critical so the repo is not just abstractions.

## Examples to add

### 1. `simple-case-app`
Demonstrates:
- create case resource
- patch case metadata
- append evidence refs
- transition case state

### 2. `simple-playbook-app`
Demonstrates:
- choose/select playbook
- create playbook run or equivalent tracked state
- append step results
- emit/update outcome of execution

### 3. `end-to-end-demo`
Demonstrates a realistic path:

- ingress signal arrives
- case created
- playbook selected
- one or more specialist-like checks run
- evidence appended
- RCA op called
- approval/outcome flow executed

## Important constraints

- examples should be small and deterministic
- examples should use the real crates and reference artifacts from this repo
- do not require external SaaS services for the first cut
- keep domain flavor generic, not telecom-specific

## Deliverables

- runnable example code
- README per example
- one top-level docs page explaining the example flow and which crate/artifact each step exercises

## Success criteria

A developer can clone the repo, run the examples, and understand how runtime + contracts + ops + apps fit together.
